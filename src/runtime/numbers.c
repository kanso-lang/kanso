/* The refusal both slow paths give. The interpreter reads bytes as text
   before it parses them, and bytes that are not utf-8 are refused as bytes
   rather than quoted; ascii is always utf-8, so only a range holding a high
   byte is asked. */
static KValue k_not_a_number(const char* data, long long len, const char* tail,
                             const char* as_bytes, const char* origin) {
    long long chars;
    if (!k_all_ascii(data, len) && k_utf8_bad(data, len, origin, &chars).tag == K_ERR)
        return k_err(k_str(as_bytes), origin);
    KValue str = k_str_n(data, len);
    return k_err(k_concat(k_concat(k_str("\""), str), k_str(tail)), origin);
}

/* Everything past the bare digit loop: libc's strtoll and the two refusals.
   It is a separate function so the fast path above carries no frame -- with
   the calls in the same body the compiler pinned the string's data, length
   and origin in callee-saved registers and paid five pushes and five pops on
   every call, for a path no call in either benchmark takes. preserve_most
   takes the last push with it, for the reason k_alloc_refill gives. */
static __attribute__((noinline, cold, preserve_most)) KValue k_b_to_int_slow(const char* data, long long len, const char* origin) {
    char* end = NULL;
    errno = 0;
    /* strtoll reads until a byte stops it, and a range read out of the
       middle of bytes has more digits after its last one. So the slow path
       parses a terminated copy, and `end` is measured against that. */
    char small[64];
    char* copy = len < 64 ? small : malloc((size_t)len + 1);
    memcpy(copy, data, (size_t)len);
    copy[len] = 0;
    long long n = strtoll(copy, &end, 10);
    /* strtoll also skips leading space, which the interpreter's parse does
       not; a number that starts with one is not an integer on either. */
    int whole = len != 0 && end == copy + len && !k_num_space(copy[0]);
    int range = errno == ERANGE;
    if (copy != small) free(copy);
    if (!whole) return k_not_a_number(data, len, "\" is not an integer", "bytes are not an integer", origin);
    /* strtoll saturates while consuming every digit, and the interpreter
       answers the whole integer, so a value past the word is read again from
       its digits into a bignum. An err here was a value a program could catch
       and carry on from, and std/json caught it and answered "invalid number"
       for a well-formed document. */
    if (range) return k_int_of_digits(data, len);
    return k_int(n);
}

static KValue k_to_int_text(const char* data, long long len, long long room,
                            const char* origin);
static KValue k_to_float_text(const char* data, long long len, long long room,
                              const char* origin);
KValue k_b_to_float(KValue v, const char* origin);

KValue k_b_to_int(KValue sv, const char* origin) {
    if (!k_not_failure(sv)) return sv;
    if (K_IS_INT(sv)) return sv;
    if (sv.tag != K_STR && sv.tag != K_BYTES) k_die_value("to_int takes a string, bytes, or int", sv);
    const char* data;
    long long len;
    if (sv.tag == K_STR) { KStr* s = k_as_str(sv); data = s->data; len = s->len; }
    else { KBytes* b = k_as_bytes(sv); data = (const char*)b->data; len = b->len; }
    return k_to_int_text(data, len, len, origin);
}

/* A word's bytes that are not ascii digits, as 0x80 in each such byte. Each
   byte is xored with '0', so a digit becomes 0 to 9, and a byte is flagged
   when it is above 9: its low seven bits plus 0x76 reach 0x80 from 0x0A up,
   and its own top bit covers the rest. The add stays under 0x100 in every
   byte, so no byte's answer carries into its neighbour's. */
static inline uint64_t k_nondigits8(uint64_t x) {
    uint64_t t = x ^ 0x3030303030303030ULL;
    return (((t & 0x7F7F7F7F7F7F7F7FULL) + 0x7676767676767676ULL) | t) & 0x8080808080808080ULL;
}

/* The top `n` bytes of `w` read as decimal digits, 1 <= n <= 8, most
   significant first in memory, with the bytes below them read as leading
   zeros. The caller has checked they are digits. The combine is three
   multiply-shifts: digits into pairs, pairs into quads, quads into eight. */
static inline long long k_digits8_value(uint64_t w, long long n) {
    uint64_t keep = ~0ULL << (8 * (8 - n));
    uint64_t v = ((w & keep & 0x0F0F0F0F0F0F0F0FULL) * 2561) >> 8;
    v = ((v & 0x00FF00FF00FF00FFULL) * 6553601) >> 16;
    return (long long)(uint32_t)(((v & 0x0000FFFF0000FFFFULL) * 42949672960001ULL) >> 32);
}

/* The last `n` of the eight bytes that end at `end` as a number, when all n
   are digits, 1 <= n <= 8. The caller guarantees the eight bytes are inside
   the buffer. */
static inline int k_digits_back8(const char* end, long long n, long long* out) {
    uint64_t w;
    memcpy(&w, end - 8, 8);
    if (k_nondigits8(w) & (~0ULL << (8 * (8 - n)))) return 0;
    *out = k_digits8_value(w, n);
    return 1;
}

/* `room` is how many bytes of the buffer end at data + len, so a run of up to
   eight digits is read as one word when there are eight to read. A json
   number's slice ends inside the document, so it nearly always has room; a
   short string on its own does not, and takes the loop. */
static KValue k_to_int_text(const char* data, long long len, long long room,
                            const char* origin) {
    /* Strict [-]?digits{1,18} parses in a bare loop (18 digits cannot
       overflow i64); every other shape — longer runs, leading space or '+',
       junk — falls through to strtoll so behavior stays exactly libc's. */
    long long start = (len > 0 && data[0] == '-') ? 1 : 0;
    if (start < len && len - start <= 8 && room >= 8) {
        long long v;
        if (k_digits_back8(data + len, len - start, &v)) return k_int(start ? -v : v);
    }
    if (start < len && len - start <= 18) {
        long long acc = 0, j = start;
        for (; j < len; j++) {
            unsigned char c = (unsigned char)data[j] - '0';
            if (c > 9) break;
            acc = acc * 10 + c;
        }
        if (j == len) return k_int(start ? -acc : acc);
    }
    return k_b_to_int_slow(data, len, origin);
}

/* `to_int (slice cs a b)` and `to_float (slice cs a b)`, where the slice is
   built for the one purpose of being read as a number and dropped. The JSON
   decoder does this for every number it meets, 417,483 of them on runbench,
   and each built a view header in the arena to hand a pointer and a length
   to the parse. These read the range straight out of `cs`. Anything but
   bytes and two positions inside them takes the long way, slice then parse,
   so the two spellings cannot disagree on a value or on the err a bad range
   gives. */
KValue k_b_to_int_slice(KValue cs, KValue fromv, KValue tov, const char* origin) {
    if (cs.tag == K_BYTES && fromv.tag == K_INT && tov.tag == K_INT) {
        KBytes* b = k_as_bytes(cs);
        long long from = fromv.payload, to = tov.payload;
        if (k_span_in(from, to, b->len)) {
            return k_to_int_text((const char*)b->data + (from - 1), to - from + 1, to,
                                 origin);
        }
    }
    return k_b_to_int(k_b_slice(cs, fromv, tov), origin);
}

K_DOORCC KValue k_b_to_float_slice(KValue cs, KValue fromv, KValue tov, const char* origin) {
    if (cs.tag == K_BYTES && fromv.tag == K_INT && tov.tag == K_INT) {
        KBytes* b = k_as_bytes(cs);
        long long from = fromv.payload, to = tov.payload;
        if (k_span_in(from, to, b->len)) {
            return k_to_float_text((const char*)b->data + (from - 1), to - from + 1, to,
                                   origin);
        }
    }
    return k_b_to_float(k_b_slice(cs, fromv, tov), origin);
}

/* Ruled 2026-10-04: `list/tie ids maker` calls the maker once per id with
   the id and `ref`, and answers the map from id to node. Each id's cell is a
   blackhole made before the first node, so a node can store the cell of an id
   made later; a constructor keeps a blackhole rather than demanding it, which
   is the rule a knotted constant already relies on. When every node is made,
   each cell is filled with its node.

   The cells do not stay in the graph. A thunk lives outside the arenas, and
   the deep copy at a cohort pop keeps a thunk and redirects its result, so a
   graph left holding cells would leave one behind per node every time it was
   tied and dropped. So once the cells are filled, every field the maker built
   that holds one is rewritten to hold the node itself, and the cells go back
   to the free list. The check keeps a reference in constructor fields, which
   is where this walk looks: records and lists below the nodes, never through
   a cell, and never twice through one value.

   One `tie` runs inside another's maker as easily as anywhere, so the calls
   still making their nodes are a stack, grown as deep as the program nests
   them. `ref` names its own call by serial rather than by position: a lambda
   the maker stored can call `ref` after its tie has returned, and by then the
   position may hold a later tie.

   Every runtime entry is in every binary whether the program calls it or not,
   so these are cold and left unoptimized: with the inliner free to fold the
   map and closure calls in, every benchmark's text grew by 13 KB for a
   feature none of them uses, and `minsize` alone still left 11 KB. */
typedef struct {
    KValue cells;   /* map from id to its cell, made before the first node */
    long long serial;
    int broken;     /* a link to an id no call made was asked for */
    KValue from, to;
} KTie;
static KTie* k_ties = 0;
static long long k_tie_cap = 0;
static long long k_tie_depth = 0;
static long long k_tie_serial = 0;
static KPtrMap k_tie_seen;
static size_t k_tie_seen_live = 0;

/* A blackhole cell from the free list the lazy tier recycles through. */
static __attribute__((cold, optnone, noinline)) KValue k_tie_cell(void) {
    KThunk* t = k_thunk_free;
    if (t) {
        k_thunk_free = t->next_free;
    } else {
        t = (KThunk*)malloc(sizeof(KThunk));
        if (!t) { fputs("out of memory\n", stderr); exit(1); }
    }
    t->rc = 1;
    t->site = K_SITE_BLACKHOLE;
    t->forced = K_MEMO_NONE;
    t->argc = 0;
    t->next_free = 0;
    t->pooled = 0;
    KValue v;
    v.tag = K_THUNK;
    v.payload = (long long)t;
    return v;
}

static __attribute__((cold, optnone, noinline)) K_CLOSCC KValue k_tie_ref(void* env, KValue id) {
    long long serial = k_env_get(env, 0).payload;
    KTie* t = 0;
    for (long long i = k_tie_depth - 1; i >= 0 && !t; i--)
        if (k_ties[i].serial == serial) t = &k_ties[i];
    if (!t) k_die("a reference from list/tie was asked for after its tie returned");
    if (!k_not_failure(id)) return id;
    KValue cell = k_b_at(t->cells, id);
    if (cell.tag == K_THUNK) return cell;
    /* A link to an id the list does not hold. The node under construction
       still builds around a cell nothing fills, and the tie answers the first
       such link once its maker is done. Both ids are frozen, because the one
       asked for may live in a region a helper's return is about to free. */
    if (!t->broken) {
        t->broken = 1;
        t->from = k_caf_freeze(k_env_get(env, 1));
        t->to = k_caf_freeze(id);
    }
    return k_tie_cell();
}

static int k_tie_filled(KValue v) {
    if (v.tag != K_THUNK) return 0;
    KThunk* t = (KThunk*)(intptr_t)v.payload;
    return t->site == K_SITE_BLACKHOLE && t->forced == K_MEMO_KEPT;
}

static void k_tie_resolve(KValue v);

/* Every `step`-th slot from `from`: a record's fields and a list's items are
   every slot, a map's values every second one. */
static __attribute__((cold, optnone, noinline)) void k_tie_resolve_slots(KValue* items, long long n, long long from, long long step) {
    for (long long i = from; i < n; i += step) {
        if (k_tie_filled(items[i])) {
            items[i] = ((KThunk*)(intptr_t)items[i].payload)->result;
            continue;
        }
        k_tie_resolve(items[i]);
    }
}

static __attribute__((cold, optnone, noinline)) void k_tie_resolve(KValue v) {
    if (v.tag != K_REC && v.tag != K_LIST && v.tag != K_MAP && v.tag != K_SUB) return;
    void* p = (void*)(intptr_t)v.payload;
    KPtrSlot* slot = k_ptrmap_at(&k_tie_seen, p, &k_tie_seen_live);
    if (slot->gen == k_tie_seen.gen && slot->key == p) return;
    if (slot->gen != k_tie_seen.gen) k_tie_seen_live++;
    slot->gen = k_tie_seen.gen;
    slot->key = p;
    if (v.tag == K_REC) {
        k_tie_resolve_slots(k_as_rec(v)->fields, k_as_rec(v)->nfields, 0, 1);
    } else if (v.tag == K_LIST) {
        k_tie_resolve_slots(k_as_list(v)->items, k_as_list(v)->len, 0, 1);
    } else if (v.tag == K_MAP) {
        KMap* m = (KMap*)p;
        k_tie_resolve_slots(m->pairs, 2 * m->len, 1, 2);
        if (m->sorted) {
            long long n;
            KValue* s = k_map_sorted(m, &n);
            k_tie_resolve_slots(s, 2 * n, 1, 2);
        }
    } else {
        KSub* sub = (KSub*)p;
        k_tie_resolve_slots(&sub->inner, 1, 0, 1);
    }
}

__attribute__((cold, optnone, noinline)) KValue k_b_tie(KValue ids, KValue maker, KValue broken) {
    if (!k_not_failure(ids)) return ids;
    if (ids.tag != K_LIST) k_die("tie takes a list of ids and a maker");
    KList* l = k_as_list(ids);
    long long n = l->len;
    if (k_tie_depth == k_tie_cap) {
        k_tie_cap = k_tie_cap ? 2 * k_tie_cap : 8;
        k_ties = (KTie*)realloc(k_ties, (size_t)k_tie_cap * sizeof(KTie));
        if (!k_ties) { fputs("out of memory\n", stderr); exit(1); }
    }
    /* The stack can move while the maker runs, so its entry is read by index
       after each call rather than through a pointer held across one. */
    long long at = k_tie_depth++;
    long long serial = ++k_tie_serial;
    k_ties[at].serial = serial;
    k_ties[at].broken = 0;
    KValue cells = k_map_empty();
    for (long long i = 0; i < n; i++) cells = k_b_put(cells, l->items[i], k_tie_cell());
    k_ties[at].cells = cells;
    KValue nodes = k_map_empty();
    for (long long i = 0; i < n; i++) {
        KValue held[2];
        held[0].tag = K_INT;
        held[0].payload = serial;
        held[1] = l->items[i];
        KValue ref = k_closure(k_tie_ref, 1, 2, held, 0);
        KValue node = k_force(k_call2(maker, l->items[i], ref));
        if (!k_not_failure(node)) {
            k_tie_depth--;
            return node;
        }
        nodes = k_b_put(nodes, l->items[i], node);
    }
    KTie done = k_ties[at];
    k_tie_depth--;
    if (done.broken) return k_call2(broken, done.from, done.to);
    for (long long i = 0; i < n; i++) {
        KThunk* c = (KThunk*)(intptr_t)k_b_at(cells, l->items[i]).payload;
        c->result = k_b_at(nodes, l->items[i]);
        c->forced = K_MEMO_KEPT;
    }
    k_ptrmap_begin(&k_tie_seen);
    k_tie_seen_live = 0;
    for (long long i = 0; i < n; i++) k_tie_resolve(k_b_at(nodes, l->items[i]));
    /* Every reference now holds its node, so the cells go back to the free
       list. An id the list names twice reaches its cell twice, and the first
       release clears the mark the second one asks for. */
    for (long long i = 0; i < n; i++) {
        KThunk* th = (KThunk*)(intptr_t)k_b_at(cells, l->items[i]).payload;
        if (th->forced != K_MEMO_KEPT) continue;
        th->forced = K_MEMO_NONE;
        th->next_free = k_thunk_free;
        k_thunk_free = th;
    }
    return nodes;
}

KValue k_b_sqrt(KValue v) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_INT) return k_float(sqrt((double)v.payload));
    if (v.tag == K_BIG) return k_float(sqrt(k_int_to_f(v)));
    if (v.tag == K_FLOAT) return k_float(sqrt(k_as_f(v)));
    k_die_got("sqrt takes a number", v);
    return k_none();
}

/* Bitwise, over signed 64-bit ints. A shift count outside 0..63 is undefined
   in C and answers differently by host, so it is refused rather than left to
   the machine — the same call the overflow checks make. Right shift is
   arithmetic, which is what go, rust and java all spell `>>`. A subtype of
   int is read as the int it wraps, as `+` reads it; the operators reach here
   with the wrapper still on, where the builtins arrive already unwrapped. */
static long long k_bits_of(KValue v, const char* what) {
    if (v.tag == K_SUB) v = k_sub_base(v);
    if (v.tag == K_INT) return v.payload;
    if (v.tag == K_BIG) {
        char fits[96];
        snprintf(fits, sizeof fits, "%s takes whole numbers that fit 64 bits", what);
        k_die(fits);
    }
    char said[64];
    snprintf(said, sizeof said, "%s takes whole numbers", what);
    k_die_got(said, v);
    return 0;
}

static long long k_shift_of(KValue v, const char* what) {
    long long by = k_bits_of(v, what);
    if (by < 0 || by > 63) {
        char said[96];
        snprintf(said, sizeof said, "%s shifts by 0 to 63 places", what);
        k_die_got(said, v);
    }
    return by;
}

KValue k_b_bit_and(KValue a, KValue b) {
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    return k_int(k_bits_of(a, "and") & k_bits_of(b, "and"));
}

KValue k_b_bit_or(KValue a, KValue b) {
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    return k_int(k_bits_of(a, "or") | k_bits_of(b, "or"));
}

KValue k_b_bit_xor(KValue a, KValue b) {
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    return k_int(k_bits_of(a, "xor") ^ k_bits_of(b, "xor"));
}

KValue k_b_bit_not(KValue a) {
    if (!k_not_failure(a)) return a;
    return k_int(~k_bits_of(a, "not"));
}

KValue k_b_bit_shl(KValue a, KValue b) {
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    /* shifting a signed value left is undefined once it reaches the sign bit,
       so the shift is done on the unsigned twin and cast back */
    unsigned long long bits = (unsigned long long)k_bits_of(a, "shl");
    return k_int((long long)(bits << k_shift_of(b, "shl")));
}

KValue k_b_bit_shr(KValue a, KValue b) {
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    long long bits = k_bits_of(a, "shr");
    long long by = k_shift_of(b, "shr");
    /* arithmetic, spelled without relying on the implementation-defined sign
       behaviour of >> on a negative operand */
    if (bits >= 0) return k_int((long long)((unsigned long long)bits >> by));
    return k_int(~(long long)((unsigned long long)(~bits) >> by));
}

KValue k_b_round(KValue v) {
    if (!k_not_failure(v)) return v;
    if (K_IS_INT(v)) return v;
    if (v.tag == K_FLOAT) {
        double x = k_as_f(v);
        /* NaN and the infinities have no integer to round to and answer
           none, as the interpreter does (ruled 2026-09-29). Every finite
           float rounds to a whole number, and one past int64 is a bignum:
           1e30 rounds to a thirty-one digit int. */
        if (x != x || isinf(x)) return k_none();
        return k_int_of_whole_f(round(x));
    }
    k_die_got("round takes a number", v);
    return k_none();
}

/* Generated: 10^q = (hi.lo / 2^128) * 2^e2 for q in [-342, 308], the
   128-bit significand MSB-normalized; negative powers rounded up per
   the eisel-lemire convention. Regeneration: compiler log 2026-07-23. */
static const struct { unsigned long long hi, lo; int e2; } k_el_pow10[] = {
    {0xeef453d6923bd65aULL, 0x113faa2906a13b40ULL, -1136},
    {0x9558b4661b6565f8ULL, 0x4ac7ca59a424c508ULL, -1132},
    {0xbaaee17fa23ebf76ULL, 0x5d79bcf00d2df64aULL, -1129},
    {0xe95a99df8ace6f53ULL, 0xf4d82c2c107973ddULL, -1126},
    {0x91d8a02bb6c10594ULL, 0x79071b9b8a4be86aULL, -1122},
    {0xb64ec836a47146f9ULL, 0x9748e2826cdee285ULL, -1119},
    {0xe3e27a444d8d98b7ULL, 0xfd1b1b2308169b26ULL, -1116},
    {0x8e6d8c6ab0787f72ULL, 0xfe30f0f5e50e20f8ULL, -1112},
    {0xb208ef855c969f4fULL, 0xbdbd2d335e51a936ULL, -1109},
    {0xde8b2b66b3bc4723ULL, 0xad2c788035e61383ULL, -1106},
    {0x8b16fb203055ac76ULL, 0x4c3bcb5021afcc32ULL, -1102},
    {0xaddcb9e83c6b1793ULL, 0xdf4abe242a1bbf3eULL, -1099},
    {0xd953e8624b85dd78ULL, 0xd71d6dad34a2af0eULL, -1096},
    {0x87d4713d6f33aa6bULL, 0x8672648c40e5ad69ULL, -1092},
    {0xa9c98d8ccb009506ULL, 0x680efdaf511f18c3ULL, -1089},
    {0xd43bf0effdc0ba48ULL, 0x0212bd1b2566def3ULL, -1086},
    {0x84a57695fe98746dULL, 0x014bb630f7604b58ULL, -1082},
    {0xa5ced43b7e3e9188ULL, 0x419ea3bd35385e2eULL, -1079},
    {0xcf42894a5dce35eaULL, 0x52064cac828675baULL, -1076},
    {0x818995ce7aa0e1b2ULL, 0x7343efebd1940994ULL, -1072},
    {0xa1ebfb4219491a1fULL, 0x1014ebe6c5f90bf9ULL, -1069},
    {0xca66fa129f9b60a6ULL, 0xd41a26e077774ef7ULL, -1066},
    {0xfd00b897478238d0ULL, 0x8920b098955522b5ULL, -1063},
    {0x9e20735e8cb16382ULL, 0x55b46e5f5d5535b1ULL, -1059},
    {0xc5a890362fddbc62ULL, 0xeb2189f734aa831eULL, -1056},
    {0xf712b443bbd52b7bULL, 0xa5e9ec7501d523e5ULL, -1053},
    {0x9a6bb0aa55653b2dULL, 0x47b233c92125366fULL, -1049},
    {0xc1069cd4eabe89f8ULL, 0x999ec0bb696e840bULL, -1046},
    {0xf148440a256e2c76ULL, 0xc00670ea43ca250eULL, -1043},
    {0x96cd2a865764dbcaULL, 0x380406926a5e5729ULL, -1039},
    {0xbc807527ed3e12bcULL, 0xc605083704f5ecf3ULL, -1036},
    {0xeba09271e88d976bULL, 0xf7864a44c633682fULL, -1033},
    {0x93445b8731587ea3ULL, 0x7ab3ee6afbe0211eULL, -1029},
    {0xb8157268fdae9e4cULL, 0x5960ea05bad82965ULL, -1026},
    {0xe61acf033d1a45dfULL, 0x6fb92487298e33beULL, -1023},
    {0x8fd0c16206306babULL, 0xa5d3b6d479f8e057ULL, -1019},
    {0xb3c4f1ba87bc8696ULL, 0x8f48a4899877186dULL, -1016},
    {0xe0b62e2929aba83cULL, 0x331acdabfe94de88ULL, -1013},
    {0x8c71dcd9ba0b4925ULL, 0x9ff0c08b7f1d0b15ULL, -1009},
    {0xaf8e5410288e1b6fULL, 0x07ecf0ae5ee44ddaULL, -1006},
    {0xdb71e91432b1a24aULL, 0xc9e82cd9f69d6151ULL, -1003},
    {0x892731ac9faf056eULL, 0xbe311c083a225cd3ULL, -999},
    {0xab70fe17c79ac6caULL, 0x6dbd630a48aaf407ULL, -996},
    {0xd64d3d9db981787dULL, 0x092cbbccdad5b109ULL, -993},
    {0x85f0468293f0eb4eULL, 0x25bbf56008c58ea6ULL, -989},
    {0xa76c582338ed2621ULL, 0xaf2af2b80af6f24fULL, -986},
    {0xd1476e2c07286faaULL, 0x1af5af660db4aee2ULL, -983},
    {0x82cca4db847945caULL, 0x50d98d9fc890ed4eULL, -979},
    {0xa37fce126597973cULL, 0xe50ff107bab528a1ULL, -976},
    {0xcc5fc196fefd7d0cULL, 0x1e53ed49a96272c9ULL, -973},
    {0xff77b1fcbebcdc4fULL, 0x25e8e89c13bb0f7bULL, -970},
    {0x9faacf3df73609b1ULL, 0x77b191618c54e9adULL, -966},
    {0xc795830d75038c1dULL, 0xd59df5b9ef6a2418ULL, -963},
    {0xf97ae3d0d2446f25ULL, 0x4b0573286b44ad1eULL, -960},
    {0x9becce62836ac577ULL, 0x4ee367f9430aec33ULL, -956},
    {0xc2e801fb244576d5ULL, 0x229c41f793cda740ULL, -953},
    {0xf3a20279ed56d48aULL, 0x6b43527578c11110ULL, -950},
    {0x9845418c345644d6ULL, 0x830a13896b78aaaaULL, -946},
    {0xbe5691ef416bd60cULL, 0x23cc986bc656d554ULL, -943},
    {0xedec366b11c6cb8fULL, 0x2cbfbe86b7ec8aa9ULL, -940},
    {0x94b3a202eb1c3f39ULL, 0x7bf7d71432f3d6aaULL, -936},
    {0xb9e08a83a5e34f07ULL, 0xdaf5ccd93fb0cc54ULL, -933},
    {0xe858ad248f5c22c9ULL, 0xd1b3400f8f9cff69ULL, -930},
    {0x91376c36d99995beULL, 0x23100809b9c21fa2ULL, -926},
    {0xb58547448ffffb2dULL, 0xabd40a0c2832a78bULL, -923},
    {0xe2e69915b3fff9f9ULL, 0x16c90c8f323f516dULL, -920},
    {0x8dd01fad907ffc3bULL, 0xae3da7d97f6792e4ULL, -916},
    {0xb1442798f49ffb4aULL, 0x99cd11cfdf41779dULL, -913},
    {0xdd95317f31c7fa1dULL, 0x40405643d711d584ULL, -910},
    {0x8a7d3eef7f1cfc52ULL, 0x482835ea666b2573ULL, -906},
    {0xad1c8eab5ee43b66ULL, 0xda3243650005eed0ULL, -903},
    {0xd863b256369d4a40ULL, 0x90bed43e40076a83ULL, -900},
    {0x873e4f75e2224e68ULL, 0x5a7744a6e804a292ULL, -896},
    {0xa90de3535aaae202ULL, 0x711515d0a205cb37ULL, -893},
    {0xd3515c2831559a83ULL, 0x0d5a5b44ca873e04ULL, -890},
    {0x8412d9991ed58091ULL, 0xe858790afe9486c3ULL, -886},
    {0xa5178fff668ae0b6ULL, 0x626e974dbe39a873ULL, -883},
    {0xce5d73ff402d98e3ULL, 0xfb0a3d212dc81290ULL, -880},
    {0x80fa687f881c7f8eULL, 0x7ce66634bc9d0b9aULL, -876},
    {0xa139029f6a239f72ULL, 0x1c1fffc1ebc44e81ULL, -873},
    {0xc987434744ac874eULL, 0xa327ffb266b56221ULL, -870},
    {0xfbe9141915d7a922ULL, 0x4bf1ff9f0062baa9ULL, -867},
    {0x9d71ac8fada6c9b5ULL, 0x6f773fc3603db4aaULL, -863},
    {0xc4ce17b399107c22ULL, 0xcb550fb4384d21d4ULL, -860},
    {0xf6019da07f549b2bULL, 0x7e2a53a146606a49ULL, -857},
    {0x99c102844f94e0fbULL, 0x2eda7444cbfc426eULL, -853},
    {0xc0314325637a1939ULL, 0xfa911155fefb5309ULL, -850},
    {0xf03d93eebc589f88ULL, 0x793555ab7eba27cbULL, -847},
    {0x96267c7535b763b5ULL, 0x4bc1558b2f3458dfULL, -843},
    {0xbbb01b9283253ca2ULL, 0x9eb1aaedfb016f17ULL, -840},
    {0xea9c227723ee8bcbULL, 0x465e15a979c1caddULL, -837},
    {0x92a1958a7675175fULL, 0x0bfacd89ec191ecaULL, -833},
    {0xb749faed14125d36ULL, 0xcef980ec671f667cULL, -830},
    {0xe51c79a85916f484ULL, 0x82b7e12780e7401bULL, -827},
    {0x8f31cc0937ae58d2ULL, 0xd1b2ecb8b0908811ULL, -823},
    {0xb2fe3f0b8599ef07ULL, 0x861fa7e6dcb4aa16ULL, -820},
    {0xdfbdcece67006ac9ULL, 0x67a791e093e1d49bULL, -817},
    {0x8bd6a141006042bdULL, 0xe0c8bb2c5c6d24e1ULL, -813},
    {0xaecc49914078536dULL, 0x58fae9f773886e19ULL, -810},
    {0xda7f5bf590966848ULL, 0xaf39a475506a899fULL, -807},
    {0x888f99797a5e012dULL, 0x6d8406c952429604ULL, -803},
    {0xaab37fd7d8f58178ULL, 0xc8e5087ba6d33b84ULL, -800},
    {0xd5605fcdcf32e1d6ULL, 0xfb1e4a9a90880a65ULL, -797},
    {0x855c3be0a17fcd26ULL, 0x5cf2eea09a550680ULL, -793},
    {0xa6b34ad8c9dfc06fULL, 0xf42faa48c0ea481fULL, -790},
    {0xd0601d8efc57b08bULL, 0xf13b94daf124da27ULL, -787},
    {0x823c12795db6ce57ULL, 0x76c53d08d6b70859ULL, -783},
    {0xa2cb1717b52481edULL, 0x54768c4b0c64ca6fULL, -780},
    {0xcb7ddcdda26da268ULL, 0xa9942f5dcf7dfd0aULL, -777},
    {0xfe5d54150b090b02ULL, 0xd3f93b35435d7c4dULL, -774},
    {0x9efa548d26e5a6e1ULL, 0xc47bc5014a1a6db0ULL, -770},
    {0xc6b8e9b0709f109aULL, 0x359ab6419ca1091cULL, -767},
    {0xf867241c8cc6d4c0ULL, 0xc30163d203c94b63ULL, -764},
    {0x9b407691d7fc44f8ULL, 0x79e0de63425dcf1eULL, -760},
    {0xc21094364dfb5636ULL, 0x985915fc12f542e5ULL, -757},
    {0xf294b943e17a2bc4ULL, 0x3e6f5b7b17b2939eULL, -754},
    {0x979cf3ca6cec5b5aULL, 0xa705992ceecf9c43ULL, -750},
    {0xbd8430bd08277231ULL, 0x50c6ff782a838354ULL, -747},
    {0xece53cec4a314ebdULL, 0xa4f8bf5635246429ULL, -744},
    {0x940f4613ae5ed136ULL, 0x871b7795e136be9aULL, -740},
    {0xb913179899f68584ULL, 0x28e2557b59846e40ULL, -737},
    {0xe757dd7ec07426e5ULL, 0x331aeada2fe589d0ULL, -734},
    {0x9096ea6f3848984fULL, 0x3ff0d2c85def7622ULL, -730},
    {0xb4bca50b065abe63ULL, 0x0fed077a756b53aaULL, -727},
    {0xe1ebce4dc7f16dfbULL, 0xd3e8495912c62895ULL, -724},
    {0x8d3360f09cf6e4bdULL, 0x64712dd7abbbd95dULL, -720},
    {0xb080392cc4349decULL, 0xbd8d794d96aacfb4ULL, -717},
    {0xdca04777f541c567ULL, 0xecf0d7a0fc5583a1ULL, -714},
    {0x89e42caaf9491b60ULL, 0xf41686c49db57245ULL, -710},
    {0xac5d37d5b79b6239ULL, 0x311c2875c522ced6ULL, -707},
    {0xd77485cb25823ac7ULL, 0x7d633293366b828cULL, -704},
    {0x86a8d39ef77164bcULL, 0xae5dff9c02033198ULL, -700},
    {0xa8530886b54dbdebULL, 0xd9f57f830283fdfdULL, -697},
    {0xd267caa862a12d66ULL, 0xd072df63c324fd7cULL, -694},
    {0x8380dea93da4bc60ULL, 0x4247cb9e59f71e6eULL, -690},
    {0xa46116538d0deb78ULL, 0x52d9be85f074e609ULL, -687},
    {0xcd795be870516656ULL, 0x67902e276c921f8cULL, -684},
    {0x806bd9714632dff6ULL, 0x00ba1cd8a3db53b7ULL, -680},
    {0xa086cfcd97bf97f3ULL, 0x80e8a40eccd228a5ULL, -677},
    {0xc8a883c0fdaf7df0ULL, 0x6122cd128006b2ceULL, -674},
    {0xfad2a4b13d1b5d6cULL, 0x796b805720085f82ULL, -671},
    {0x9cc3a6eec6311a63ULL, 0xcbe3303674053bb1ULL, -667},
    {0xc3f490aa77bd60fcULL, 0xbedbfc4411068a9dULL, -664},
    {0xf4f1b4d515acb93bULL, 0xee92fb5515482d45ULL, -661},
    {0x991711052d8bf3c5ULL, 0x751bdd152d4d1c4bULL, -657},
    {0xbf5cd54678eef0b6ULL, 0xd262d45a78a0635eULL, -654},
    {0xef340a98172aace4ULL, 0x86fb897116c87c35ULL, -651},
    {0x9580869f0e7aac0eULL, 0xd45d35e6ae3d4da1ULL, -647},
    {0xbae0a846d2195712ULL, 0x8974836059cca10aULL, -644},
    {0xe998d258869facd7ULL, 0x2bd1a438703fc94cULL, -641},
    {0x91ff83775423cc06ULL, 0x7b6306a34627ddd0ULL, -637},
    {0xb67f6455292cbf08ULL, 0x1a3bc84c17b1d543ULL, -634},
    {0xe41f3d6a7377eecaULL, 0x20caba5f1d9e4a94ULL, -631},
    {0x8e938662882af53eULL, 0x547eb47b7282ee9dULL, -627},
    {0xb23867fb2a35b28dULL, 0xe99e619a4f23aa44ULL, -624},
    {0xdec681f9f4c31f31ULL, 0x6405fa00e2ec94d5ULL, -621},
    {0x8b3c113c38f9f37eULL, 0xde83bc408dd3dd05ULL, -617},
    {0xae0b158b4738705eULL, 0x9624ab50b148d446ULL, -614},
    {0xd98ddaee19068c76ULL, 0x3badd624dd9b0958ULL, -611},
    {0x87f8a8d4cfa417c9ULL, 0xe54ca5d70a80e5d7ULL, -607},
    {0xa9f6d30a038d1dbcULL, 0x5e9fcf4ccd211f4dULL, -604},
    {0xd47487cc8470652bULL, 0x7647c32000696720ULL, -601},
    {0x84c8d4dfd2c63f3bULL, 0x29ecd9f40041e074ULL, -597},
    {0xa5fb0a17c777cf09ULL, 0xf468107100525891ULL, -594},
    {0xcf79cc9db955c2ccULL, 0x7182148d4066eeb5ULL, -591},
    {0x81ac1fe293d599bfULL, 0xc6f14cd848405531ULL, -587},
    {0xa21727db38cb002fULL, 0xb8ada00e5a506a7dULL, -584},
    {0xca9cf1d206fdc03bULL, 0xa6d90811f0e4851dULL, -581},
    {0xfd442e4688bd304aULL, 0x908f4a166d1da664ULL, -578},
    {0x9e4a9cec15763e2eULL, 0x9a598e4e043287ffULL, -574},
    {0xc5dd44271ad3cdbaULL, 0x40eff1e1853f29feULL, -571},
    {0xf7549530e188c128ULL, 0xd12bee59e68ef47dULL, -568},
    {0x9a94dd3e8cf578b9ULL, 0x82bb74f8301958cfULL, -564},
    {0xc13a148e3032d6e7ULL, 0xe36a52363c1faf02ULL, -561},
    {0xf18899b1bc3f8ca1ULL, 0xdc44e6c3cb279ac2ULL, -558},
    {0x96f5600f15a7b7e5ULL, 0x29ab103a5ef8c0baULL, -554},
    {0xbcb2b812db11a5deULL, 0x7415d448f6b6f0e8ULL, -551},
    {0xebdf661791d60f56ULL, 0x111b495b3464ad22ULL, -548},
    {0x936b9fcebb25c995ULL, 0xcab10dd900beec35ULL, -544},
    {0xb84687c269ef3bfbULL, 0x3d5d514f40eea743ULL, -541},
    {0xe65829b3046b0afaULL, 0x0cb4a5a3112a5113ULL, -538},
    {0x8ff71a0fe2c2e6dcULL, 0x47f0e785eaba72acULL, -534},
    {0xb3f4e093db73a093ULL, 0x59ed216765690f57ULL, -531},
    {0xe0f218b8d25088b8ULL, 0x306869c13ec3532dULL, -528},
    {0x8c974f7383725573ULL, 0x1e414218c73a13fcULL, -524},
    {0xafbd2350644eeacfULL, 0xe5d1929ef90898fbULL, -521},
    {0xdbac6c247d62a583ULL, 0xdf45f746b74abf3aULL, -518},
    {0x894bc396ce5da772ULL, 0x6b8bba8c328eb784ULL, -514},
    {0xab9eb47c81f5114fULL, 0x066ea92f3f326565ULL, -511},
    {0xd686619ba27255a2ULL, 0xc80a537b0efefebeULL, -508},
    {0x8613fd0145877585ULL, 0xbd06742ce95f5f37ULL, -504},
    {0xa798fc4196e952e7ULL, 0x2c48113823b73705ULL, -501},
    {0xd17f3b51fca3a7a0ULL, 0xf75a15862ca504c6ULL, -498},
    {0x82ef85133de648c4ULL, 0x9a984d73dbe722fcULL, -494},
    {0xa3ab66580d5fdaf5ULL, 0xc13e60d0d2e0ebbbULL, -491},
    {0xcc963fee10b7d1b3ULL, 0x318df905079926a9ULL, -488},
    {0xffbbcfe994e5c61fULL, 0xfdf17746497f7053ULL, -485},
    {0x9fd561f1fd0f9bd3ULL, 0xfeb6ea8bedefa634ULL, -481},
    {0xc7caba6e7c5382c8ULL, 0xfe64a52ee96b8fc1ULL, -478},
    {0xf9bd690a1b68637bULL, 0x3dfdce7aa3c673b1ULL, -475},
    {0x9c1661a651213e2dULL, 0x06bea10ca65c084fULL, -471},
    {0xc31bfa0fe5698db8ULL, 0x486e494fcff30a63ULL, -468},
    {0xf3e2f893dec3f126ULL, 0x5a89dba3c3efccfbULL, -465},
    {0x986ddb5c6b3a76b7ULL, 0xf89629465a75e01dULL, -461},
    {0xbe89523386091465ULL, 0xf6bbb397f1135824ULL, -458},
    {0xee2ba6c0678b597fULL, 0x746aa07ded582e2dULL, -455},
    {0x94db483840b717efULL, 0xa8c2a44eb4571cddULL, -451},
    {0xba121a4650e4ddebULL, 0x92f34d62616ce414ULL, -448},
    {0xe896a0d7e51e1566ULL, 0x77b020baf9c81d18ULL, -445},
    {0x915e2486ef32cd60ULL, 0x0ace1474dc1d122fULL, -441},
    {0xb5b5ada8aaff80b8ULL, 0x0d819992132456bbULL, -438},
    {0xe3231912d5bf60e6ULL, 0x10e1fff697ed6c6aULL, -435},
    {0x8df5efabc5979c8fULL, 0xca8d3ffa1ef463c2ULL, -431},
    {0xb1736b96b6fd83b3ULL, 0xbd308ff8a6b17cb3ULL, -428},
    {0xddd0467c64bce4a0ULL, 0xac7cb3f6d05ddbdfULL, -425},
    {0x8aa22c0dbef60ee4ULL, 0x6bcdf07a423aa96cULL, -421},
    {0xad4ab7112eb3929dULL, 0x86c16c98d2c953c7ULL, -418},
    {0xd89d64d57a607744ULL, 0xe871c7bf077ba8b8ULL, -415},
    {0x87625f056c7c4a8bULL, 0x11471cd764ad4973ULL, -411},
    {0xa93af6c6c79b5d2dULL, 0xd598e40d3dd89bd0ULL, -408},
    {0xd389b47879823479ULL, 0x4aff1d108d4ec2c4ULL, -405},
    {0x843610cb4bf160cbULL, 0xcedf722a585139bbULL, -401},
    {0xa54394fe1eedb8feULL, 0xc2974eb4ee658829ULL, -398},
    {0xce947a3da6a9273eULL, 0x733d226229feea33ULL, -395},
    {0x811ccc668829b887ULL, 0x0806357d5a3f5260ULL, -391},
    {0xa163ff802a3426a8ULL, 0xca07c2dcb0cf26f8ULL, -388},
    {0xc9bcff6034c13052ULL, 0xfc89b393dd02f0b6ULL, -385},
    {0xfc2c3f3841f17c67ULL, 0xbbac2078d443ace3ULL, -382},
    {0x9d9ba7832936edc0ULL, 0xd54b944b84aa4c0eULL, -378},
    {0xc5029163f384a931ULL, 0x0a9e795e65d4df12ULL, -375},
    {0xf64335bcf065d37dULL, 0x4d4617b5ff4a16d6ULL, -372},
    {0x99ea0196163fa42eULL, 0x504bced1bf8e4e46ULL, -368},
    {0xc06481fb9bcf8d39ULL, 0xe45ec2862f71e1d7ULL, -365},
    {0xf07da27a82c37088ULL, 0x5d767327bb4e5a4dULL, -362},
    {0x964e858c91ba2655ULL, 0x3a6a07f8d510f870ULL, -358},
    {0xbbe226efb628afeaULL, 0x890489f70a55368cULL, -355},
    {0xeadab0aba3b2dbe5ULL, 0x2b45ac74ccea842fULL, -352},
    {0x92c8ae6b464fc96fULL, 0x3b0b8bc90012929eULL, -348},
    {0xb77ada0617e3bbcbULL, 0x09ce6ebb40173745ULL, -345},
    {0xe55990879ddcaabdULL, 0xcc420a6a101d0516ULL, -342},
    {0x8f57fa54c2a9eab6ULL, 0x9fa946824a12232eULL, -338},
    {0xb32df8e9f3546564ULL, 0x47939822dc96abfaULL, -335},
    {0xdff9772470297ebdULL, 0x59787e2b93bc56f8ULL, -332},
    {0x8bfbea76c619ef36ULL, 0x57eb4edb3c55b65bULL, -328},
    {0xaefae51477a06b03ULL, 0xede622920b6b23f2ULL, -325},
    {0xdab99e59958885c4ULL, 0xe95fab368e45eceeULL, -322},
    {0x88b402f7fd75539bULL, 0x11dbcb0218ebb415ULL, -318},
    {0xaae103b5fcd2a881ULL, 0xd652bdc29f26a11aULL, -315},
    {0xd59944a37c0752a2ULL, 0x4be76d3346f04960ULL, -312},
    {0x857fcae62d8493a5ULL, 0x6f70a4400c562ddcULL, -308},
    {0xa6dfbd9fb8e5b88eULL, 0xcb4ccd500f6bb953ULL, -305},
    {0xd097ad07a71f26b2ULL, 0x7e2000a41346a7a8ULL, -302},
    {0x825ecc24c873782fULL, 0x8ed400668c0c28c9ULL, -298},
    {0xa2f67f2dfa90563bULL, 0x728900802f0f32fbULL, -295},
    {0xcbb41ef979346bcaULL, 0x4f2b40a03ad2ffbaULL, -292},
    {0xfea126b7d78186bcULL, 0xe2f610c84987bfa9ULL, -289},
    {0x9f24b832e6b0f436ULL, 0x0dd9ca7d2df4d7caULL, -285},
    {0xc6ede63fa05d3143ULL, 0x91503d1c79720dbcULL, -282},
    {0xf8a95fcf88747d94ULL, 0x75a44c6397ce912bULL, -279},
    {0x9b69dbe1b548ce7cULL, 0xc986afbe3ee11abbULL, -275},
    {0xc24452da229b021bULL, 0xfbe85badce996169ULL, -272},
    {0xf2d56790ab41c2a2ULL, 0xfae27299423fb9c4ULL, -269},
    {0x97c560ba6b0919a5ULL, 0xdccd879fc967d41bULL, -265},
    {0xbdb6b8e905cb600fULL, 0x5400e987bbc1c921ULL, -262},
    {0xed246723473e3813ULL, 0x290123e9aab23b69ULL, -259},
    {0x9436c0760c86e30bULL, 0xf9a0b6720aaf6522ULL, -255},
    {0xb94470938fa89bceULL, 0xf808e40e8d5b3e6aULL, -252},
    {0xe7958cb87392c2c2ULL, 0xb60b1d1230b20e05ULL, -249},
    {0x90bd77f3483bb9b9ULL, 0xb1c6f22b5e6f48c3ULL, -245},
    {0xb4ecd5f01a4aa828ULL, 0x1e38aeb6360b1af4ULL, -242},
    {0xe2280b6c20dd5232ULL, 0x25c6da63c38de1b1ULL, -239},
    {0x8d590723948a535fULL, 0x579c487e5a38ad0fULL, -235},
    {0xb0af48ec79ace837ULL, 0x2d835a9df0c6d852ULL, -232},
    {0xdcdb1b2798182244ULL, 0xf8e431456cf88e66ULL, -229},
    {0x8a08f0f8bf0f156bULL, 0x1b8e9ecb641b5900ULL, -225},
    {0xac8b2d36eed2dac5ULL, 0xe272467e3d222f40ULL, -222},
    {0xd7adf884aa879177ULL, 0x5b0ed81dcc6abb10ULL, -219},
    {0x86ccbb52ea94baeaULL, 0x98e947129fc2b4eaULL, -215},
    {0xa87fea27a539e9a5ULL, 0x3f2398d747b36225ULL, -212},
    {0xd29fe4b18e88640eULL, 0x8eec7f0d19a03aaeULL, -209},
    {0x83a3eeeef9153e89ULL, 0x1953cf68300424adULL, -205},
    {0xa48ceaaab75a8e2bULL, 0x5fa8c3423c052dd8ULL, -202},
    {0xcdb02555653131b6ULL, 0x3792f412cb06794eULL, -199},
    {0x808e17555f3ebf11ULL, 0xe2bbd88bbee40bd1ULL, -195},
    {0xa0b19d2ab70e6ed6ULL, 0x5b6aceaeae9d0ec5ULL, -192},
    {0xc8de047564d20a8bULL, 0xf245825a5a445276ULL, -189},
    {0xfb158592be068d2eULL, 0xeed6e2f0f0d56713ULL, -186},
    {0x9ced737bb6c4183dULL, 0x55464dd69685606cULL, -182},
    {0xc428d05aa4751e4cULL, 0xaa97e14c3c26b887ULL, -179},
    {0xf53304714d9265dfULL, 0xd53dd99f4b3066a9ULL, -176},
    {0x993fe2c6d07b7fabULL, 0xe546a8038efe402aULL, -172},
    {0xbf8fdb78849a5f96ULL, 0xde98520472bdd034ULL, -169},
    {0xef73d256a5c0f77cULL, 0x963e66858f6d4441ULL, -166},
    {0x95a8637627989aadULL, 0xdde7001379a44aa9ULL, -162},
    {0xbb127c53b17ec159ULL, 0x5560c018580d5d53ULL, -159},
    {0xe9d71b689dde71afULL, 0xaab8f01e6e10b4a7ULL, -156},
    {0x9226712162ab070dULL, 0xcab3961304ca70e9ULL, -152},
    {0xb6b00d69bb55c8d1ULL, 0x3d607b97c5fd0d23ULL, -149},
    {0xe45c10c42a2b3b05ULL, 0x8cb89a7db77c506bULL, -146},
    {0x8eb98a7a9a5b04e3ULL, 0x77f3608e92adb243ULL, -142},
    {0xb267ed1940f1c61cULL, 0x55f038b237591ed4ULL, -139},
    {0xdf01e85f912e37a3ULL, 0x6b6c46dec52f6689ULL, -136},
    {0x8b61313bbabce2c6ULL, 0x2323ac4b3b3da016ULL, -132},
    {0xae397d8aa96c1b77ULL, 0xabec975e0a0d081bULL, -129},
    {0xd9c7dced53c72255ULL, 0x96e7bd358c904a22ULL, -126},
    {0x881cea14545c7575ULL, 0x7e50d64177da2e55ULL, -122},
    {0xaa242499697392d2ULL, 0xdde50bd1d5d0b9eaULL, -119},
    {0xd4ad2dbfc3d07787ULL, 0x955e4ec64b44e865ULL, -116},
    {0x84ec3c97da624ab4ULL, 0xbd5af13bef0b113fULL, -112},
    {0xa6274bbdd0fadd61ULL, 0xecb1ad8aeacdd58fULL, -109},
    {0xcfb11ead453994baULL, 0x67de18eda5814af3ULL, -106},
    {0x81ceb32c4b43fcf4ULL, 0x80eacf948770ced8ULL, -102},
    {0xa2425ff75e14fc31ULL, 0xa1258379a94d028eULL, -99},
    {0xcad2f7f5359a3b3eULL, 0x096ee45813a04331ULL, -96},
    {0xfd87b5f28300ca0dULL, 0x8bca9d6e188853fdULL, -93},
    {0x9e74d1b791e07e48ULL, 0x775ea264cf55347eULL, -89},
    {0xc612062576589ddaULL, 0x95364afe032a819eULL, -86},
    {0xf79687aed3eec551ULL, 0x3a83ddbd83f52205ULL, -83},
    {0x9abe14cd44753b52ULL, 0xc4926a9672793543ULL, -79},
    {0xc16d9a0095928a27ULL, 0x75b7053c0f178294ULL, -76},
    {0xf1c90080baf72cb1ULL, 0x5324c68b12dd6339ULL, -73},
    {0x971da05074da7beeULL, 0xd3f6fc16ebca5e04ULL, -69},
    {0xbce5086492111aeaULL, 0x88f4bb1ca6bcf585ULL, -66},
    {0xec1e4a7db69561a5ULL, 0x2b31e9e3d06c32e6ULL, -63},
    {0x9392ee8e921d5d07ULL, 0x3aff322e62439fd0ULL, -59},
    {0xb877aa3236a4b449ULL, 0x09befeb9fad487c3ULL, -56},
    {0xe69594bec44de15bULL, 0x4c2ebe687989a9b4ULL, -53},
    {0x901d7cf73ab0acd9ULL, 0x0f9d37014bf60a11ULL, -49},
    {0xb424dc35095cd80fULL, 0x538484c19ef38c95ULL, -46},
    {0xe12e13424bb40e13ULL, 0x2865a5f206b06fbaULL, -43},
    {0x8cbccc096f5088cbULL, 0xf93f87b7442e45d4ULL, -39},
    {0xafebff0bcb24aafeULL, 0xf78f69a51539d749ULL, -36},
    {0xdbe6fecebdedd5beULL, 0xb573440e5a884d1cULL, -33},
    {0x89705f4136b4a597ULL, 0x31680a88f8953031ULL, -29},
    {0xabcc77118461cefcULL, 0xfdc20d2b36ba7c3eULL, -26},
    {0xd6bf94d5e57a42bcULL, 0x3d32907604691b4dULL, -23},
    {0x8637bd05af6c69b5ULL, 0xa63f9a49c2c1b110ULL, -19},
    {0xa7c5ac471b478423ULL, 0x0fcf80dc33721d54ULL, -16},
    {0xd1b71758e219652bULL, 0xd3c36113404ea4a9ULL, -13},
    {0x83126e978d4fdf3bULL, 0x645a1cac083126eaULL, -9},
    {0xa3d70a3d70a3d70aULL, 0x3d70a3d70a3d70a4ULL, -6},
    {0xccccccccccccccccULL, 0xcccccccccccccccdULL, -3},
    {0x8000000000000000ULL, 0x0000000000000000ULL, 1},
    {0xa000000000000000ULL, 0x0000000000000000ULL, 4},
    {0xc800000000000000ULL, 0x0000000000000000ULL, 7},
    {0xfa00000000000000ULL, 0x0000000000000000ULL, 10},
    {0x9c40000000000000ULL, 0x0000000000000000ULL, 14},
    {0xc350000000000000ULL, 0x0000000000000000ULL, 17},
    {0xf424000000000000ULL, 0x0000000000000000ULL, 20},
    {0x9896800000000000ULL, 0x0000000000000000ULL, 24},
    {0xbebc200000000000ULL, 0x0000000000000000ULL, 27},
    {0xee6b280000000000ULL, 0x0000000000000000ULL, 30},
    {0x9502f90000000000ULL, 0x0000000000000000ULL, 34},
    {0xba43b74000000000ULL, 0x0000000000000000ULL, 37},
    {0xe8d4a51000000000ULL, 0x0000000000000000ULL, 40},
    {0x9184e72a00000000ULL, 0x0000000000000000ULL, 44},
    {0xb5e620f480000000ULL, 0x0000000000000000ULL, 47},
    {0xe35fa931a0000000ULL, 0x0000000000000000ULL, 50},
    {0x8e1bc9bf04000000ULL, 0x0000000000000000ULL, 54},
    {0xb1a2bc2ec5000000ULL, 0x0000000000000000ULL, 57},
    {0xde0b6b3a76400000ULL, 0x0000000000000000ULL, 60},
    {0x8ac7230489e80000ULL, 0x0000000000000000ULL, 64},
    {0xad78ebc5ac620000ULL, 0x0000000000000000ULL, 67},
    {0xd8d726b7177a8000ULL, 0x0000000000000000ULL, 70},
    {0x878678326eac9000ULL, 0x0000000000000000ULL, 74},
    {0xa968163f0a57b400ULL, 0x0000000000000000ULL, 77},
    {0xd3c21bcecceda100ULL, 0x0000000000000000ULL, 80},
    {0x84595161401484a0ULL, 0x0000000000000000ULL, 84},
    {0xa56fa5b99019a5c8ULL, 0x0000000000000000ULL, 87},
    {0xcecb8f27f4200f3aULL, 0x0000000000000000ULL, 90},
    {0x813f3978f8940984ULL, 0x4000000000000000ULL, 94},
    {0xa18f07d736b90be5ULL, 0x5000000000000000ULL, 97},
    {0xc9f2c9cd04674edeULL, 0xa400000000000000ULL, 100},
    {0xfc6f7c4045812296ULL, 0x4d00000000000000ULL, 103},
    {0x9dc5ada82b70b59dULL, 0xf020000000000000ULL, 107},
    {0xc5371912364ce305ULL, 0x6c28000000000000ULL, 110},
    {0xf684df56c3e01bc6ULL, 0xc732000000000000ULL, 113},
    {0x9a130b963a6c115cULL, 0x3c7f400000000000ULL, 117},
    {0xc097ce7bc90715b3ULL, 0x4b9f100000000000ULL, 120},
    {0xf0bdc21abb48db20ULL, 0x1e86d40000000000ULL, 123},
    {0x96769950b50d88f4ULL, 0x1314448000000000ULL, 127},
    {0xbc143fa4e250eb31ULL, 0x17d955a000000000ULL, 130},
    {0xeb194f8e1ae525fdULL, 0x5dcfab0800000000ULL, 133},
    {0x92efd1b8d0cf37beULL, 0x5aa1cae500000000ULL, 137},
    {0xb7abc627050305adULL, 0xf14a3d9e40000000ULL, 140},
    {0xe596b7b0c643c719ULL, 0x6d9ccd05d0000000ULL, 143},
    {0x8f7e32ce7bea5c6fULL, 0xe4820023a2000000ULL, 147},
    {0xb35dbf821ae4f38bULL, 0xdda2802c8a800000ULL, 150},
    {0xe0352f62a19e306eULL, 0xd50b2037ad200000ULL, 153},
    {0x8c213d9da502de45ULL, 0x4526f422cc340000ULL, 157},
    {0xaf298d050e4395d6ULL, 0x9670b12b7f410000ULL, 160},
    {0xdaf3f04651d47b4cULL, 0x3c0cdd765f114000ULL, 163},
    {0x88d8762bf324cd0fULL, 0xa5880a69fb6ac800ULL, 167},
    {0xab0e93b6efee0053ULL, 0x8eea0d047a457a00ULL, 170},
    {0xd5d238a4abe98068ULL, 0x72a4904598d6d880ULL, 173},
    {0x85a36366eb71f041ULL, 0x47a6da2b7f864750ULL, 177},
    {0xa70c3c40a64e6c51ULL, 0x999090b65f67d924ULL, 180},
    {0xd0cf4b50cfe20765ULL, 0xfff4b4e3f741cf6dULL, 183},
    {0x82818f1281ed449fULL, 0xbff8f10e7a8921a4ULL, 187},
    {0xa321f2d7226895c7ULL, 0xaff72d52192b6a0dULL, 190},
    {0xcbea6f8ceb02bb39ULL, 0x9bf4f8a69f764490ULL, 193},
    {0xfee50b7025c36a08ULL, 0x02f236d04753d5b4ULL, 196},
    {0x9f4f2726179a2245ULL, 0x01d762422c946590ULL, 200},
    {0xc722f0ef9d80aad6ULL, 0x424d3ad2b7b97ef5ULL, 203},
    {0xf8ebad2b84e0d58bULL, 0xd2e0898765a7deb2ULL, 206},
    {0x9b934c3b330c8577ULL, 0x63cc55f49f88eb2fULL, 210},
    {0xc2781f49ffcfa6d5ULL, 0x3cbf6b71c76b25fbULL, 213},
    {0xf316271c7fc3908aULL, 0x8bef464e3945ef7aULL, 216},
    {0x97edd871cfda3a56ULL, 0x97758bf0e3cbb5acULL, 220},
    {0xbde94e8e43d0c8ecULL, 0x3d52eeed1cbea317ULL, 223},
    {0xed63a231d4c4fb27ULL, 0x4ca7aaa863ee4bddULL, 226},
    {0x945e455f24fb1cf8ULL, 0x8fe8caa93e74ef6aULL, 230},
    {0xb975d6b6ee39e436ULL, 0xb3e2fd538e122b44ULL, 233},
    {0xe7d34c64a9c85d44ULL, 0x60dbbca87196b616ULL, 236},
    {0x90e40fbeea1d3a4aULL, 0xbc8955e946fe31cdULL, 240},
    {0xb51d13aea4a488ddULL, 0x6babab6398bdbe41ULL, 243},
    {0xe264589a4dcdab14ULL, 0xc696963c7eed2dd1ULL, 246},
    {0x8d7eb76070a08aecULL, 0xfc1e1de5cf543ca2ULL, 250},
    {0xb0de65388cc8ada8ULL, 0x3b25a55f43294bcbULL, 253},
    {0xdd15fe86affad912ULL, 0x49ef0eb713f39ebeULL, 256},
    {0x8a2dbf142dfcc7abULL, 0x6e3569326c784337ULL, 260},
    {0xacb92ed9397bf996ULL, 0x49c2c37f07965404ULL, 263},
    {0xd7e77a8f87daf7fbULL, 0xdc33745ec97be906ULL, 266},
    {0x86f0ac99b4e8dafdULL, 0x69a028bb3ded71a3ULL, 270},
    {0xa8acd7c0222311bcULL, 0xc40832ea0d68ce0cULL, 273},
    {0xd2d80db02aabd62bULL, 0xf50a3fa490c30190ULL, 276},
    {0x83c7088e1aab65dbULL, 0x792667c6da79e0faULL, 280},
    {0xa4b8cab1a1563f52ULL, 0x577001b891185938ULL, 283},
    {0xcde6fd5e09abcf26ULL, 0xed4c0226b55e6f86ULL, 286},
    {0x80b05e5ac60b6178ULL, 0x544f8158315b05b4ULL, 290},
    {0xa0dc75f1778e39d6ULL, 0x696361ae3db1c721ULL, 293},
    {0xc913936dd571c84cULL, 0x03bc3a19cd1e38e9ULL, 296},
    {0xfb5878494ace3a5fULL, 0x04ab48a04065c723ULL, 299},
    {0x9d174b2dcec0e47bULL, 0x62eb0d64283f9c76ULL, 303},
    {0xc45d1df942711d9aULL, 0x3ba5d0bd324f8394ULL, 306},
    {0xf5746577930d6500ULL, 0xca8f44ec7ee36479ULL, 309},
    {0x9968bf6abbe85f20ULL, 0x7e998b13cf4e1ecbULL, 313},
    {0xbfc2ef456ae276e8ULL, 0x9e3fedd8c321a67eULL, 316},
    {0xefb3ab16c59b14a2ULL, 0xc5cfe94ef3ea101eULL, 319},
    {0x95d04aee3b80ece5ULL, 0xbba1f1d158724a12ULL, 323},
    {0xbb445da9ca61281fULL, 0x2a8a6e45ae8edc97ULL, 326},
    {0xea1575143cf97226ULL, 0xf52d09d71a3293bdULL, 329},
    {0x924d692ca61be758ULL, 0x593c2626705f9c56ULL, 333},
    {0xb6e0c377cfa2e12eULL, 0x6f8b2fb00c77836cULL, 336},
    {0xe498f455c38b997aULL, 0x0b6dfb9c0f956447ULL, 339},
    {0x8edf98b59a373fecULL, 0x4724bd4189bd5eacULL, 343},
    {0xb2977ee300c50fe7ULL, 0x58edec91ec2cb657ULL, 346},
    {0xdf3d5e9bc0f653e1ULL, 0x2f2967b66737e3edULL, 349},
    {0x8b865b215899f46cULL, 0xbd79e0d20082ee74ULL, 353},
    {0xae67f1e9aec07187ULL, 0xecd8590680a3aa11ULL, 356},
    {0xda01ee641a708de9ULL, 0xe80e6f4820cc9495ULL, 359},
    {0x884134fe908658b2ULL, 0x3109058d147fdcddULL, 363},
    {0xaa51823e34a7eedeULL, 0xbd4b46f0599fd415ULL, 366},
    {0xd4e5e2cdc1d1ea96ULL, 0x6c9e18ac7007c91aULL, 369},
    {0x850fadc09923329eULL, 0x03e2cf6bc604ddb0ULL, 373},
    {0xa6539930bf6bff45ULL, 0x84db8346b786151cULL, 376},
    {0xcfe87f7cef46ff16ULL, 0xe612641865679a63ULL, 379},
    {0x81f14fae158c5f6eULL, 0x4fcb7e8f3f60c07eULL, 383},
    {0xa26da3999aef7749ULL, 0xe3be5e330f38f09dULL, 386},
    {0xcb090c8001ab551cULL, 0x5cadf5bfd3072cc5ULL, 389},
    {0xfdcb4fa002162a63ULL, 0x73d9732fc7c8f7f6ULL, 392},
    {0x9e9f11c4014dda7eULL, 0x2867e7fddcdd9afaULL, 396},
    {0xc646d63501a1511dULL, 0xb281e1fd541501b8ULL, 399},
    {0xf7d88bc24209a565ULL, 0x1f225a7ca91a4226ULL, 402},
    {0x9ae757596946075fULL, 0x3375788de9b06958ULL, 406},
    {0xc1a12d2fc3978937ULL, 0x0052d6b1641c83aeULL, 409},
    {0xf209787bb47d6b84ULL, 0xc0678c5dbd23a49aULL, 412},
    {0x9745eb4d50ce6332ULL, 0xf840b7ba963646e0ULL, 416},
    {0xbd176620a501fbffULL, 0xb650e5a93bc3d898ULL, 419},
    {0xec5d3fa8ce427affULL, 0xa3e51f138ab4cebeULL, 422},
    {0x93ba47c980e98cdfULL, 0xc66f336c36b10137ULL, 426},
    {0xb8a8d9bbe123f017ULL, 0xb80b0047445d4184ULL, 429},
    {0xe6d3102ad96cec1dULL, 0xa60dc059157491e5ULL, 432},
    {0x9043ea1ac7e41392ULL, 0x87c89837ad68db2fULL, 436},
    {0xb454e4a179dd1877ULL, 0x29babe4598c311fbULL, 439},
    {0xe16a1dc9d8545e94ULL, 0xf4296dd6fef3d67aULL, 442},
    {0x8ce2529e2734bb1dULL, 0x1899e4a65f58660cULL, 446},
    {0xb01ae745b101e9e4ULL, 0x5ec05dcff72e7f8fULL, 449},
    {0xdc21a1171d42645dULL, 0x76707543f4fa1f73ULL, 452},
    {0x899504ae72497ebaULL, 0x6a06494a791c53a8ULL, 456},
    {0xabfa45da0edbde69ULL, 0x0487db9d17636892ULL, 459},
    {0xd6f8d7509292d603ULL, 0x45a9d2845d3c42b6ULL, 462},
    {0x865b86925b9bc5c2ULL, 0x0b8a2392ba45a9b2ULL, 466},
    {0xa7f26836f282b732ULL, 0x8e6cac7768d7141eULL, 469},
    {0xd1ef0244af2364ffULL, 0x3207d795430cd926ULL, 472},
    {0x8335616aed761f1fULL, 0x7f44e6bd49e807b8ULL, 476},
    {0xa402b9c5a8d3a6e7ULL, 0x5f16206c9c6209a6ULL, 479},
    {0xcd036837130890a1ULL, 0x36dba887c37a8c0fULL, 482},
    {0x802221226be55a64ULL, 0xc2494954da2c9789ULL, 486},
    {0xa02aa96b06deb0fdULL, 0xf2db9baa10b7bd6cULL, 489},
    {0xc83553c5c8965d3dULL, 0x6f92829494e5acc7ULL, 492},
    {0xfa42a8b73abbf48cULL, 0xcb772339ba1f17f9ULL, 495},
    {0x9c69a97284b578d7ULL, 0xff2a760414536efbULL, 499},
    {0xc38413cf25e2d70dULL, 0xfef5138519684abaULL, 502},
    {0xf46518c2ef5b8cd1ULL, 0x7eb258665fc25d69ULL, 505},
    {0x98bf2f79d5993802ULL, 0xef2f773ffbd97a61ULL, 509},
    {0xbeeefb584aff8603ULL, 0xaafb550ffacfd8faULL, 512},
    {0xeeaaba2e5dbf6784ULL, 0x95ba2a53f983cf38ULL, 515},
    {0x952ab45cfa97a0b2ULL, 0xdd945a747bf26183ULL, 519},
    {0xba756174393d88dfULL, 0x94f971119aeef9e4ULL, 522},
    {0xe912b9d1478ceb17ULL, 0x7a37cd5601aab85dULL, 525},
    {0x91abb422ccb812eeULL, 0xac62e055c10ab33aULL, 529},
    {0xb616a12b7fe617aaULL, 0x577b986b314d6009ULL, 532},
    {0xe39c49765fdf9d94ULL, 0xed5a7e85fda0b80bULL, 535},
    {0x8e41ade9fbebc27dULL, 0x14588f13be847307ULL, 539},
    {0xb1d219647ae6b31cULL, 0x596eb2d8ae258fc8ULL, 542},
    {0xde469fbd99a05fe3ULL, 0x6fca5f8ed9aef3bbULL, 545},
    {0x8aec23d680043beeULL, 0x25de7bb9480d5854ULL, 549},
    {0xada72ccc20054ae9ULL, 0xaf561aa79a10ae6aULL, 552},
    {0xd910f7ff28069da4ULL, 0x1b2ba1518094da04ULL, 555},
    {0x87aa9aff79042286ULL, 0x90fb44d2f05d0842ULL, 559},
    {0xa99541bf57452b28ULL, 0x353a1607ac744a53ULL, 562},
    {0xd3fa922f2d1675f2ULL, 0x42889b8997915ce8ULL, 565},
    {0x847c9b5d7c2e09b7ULL, 0x69956135febada11ULL, 569},
    {0xa59bc234db398c25ULL, 0x43fab9837e699095ULL, 572},
    {0xcf02b2c21207ef2eULL, 0x94f967e45e03f4bbULL, 575},
    {0x8161afb94b44f57dULL, 0x1d1be0eebac278f5ULL, 579},
    {0xa1ba1ba79e1632dcULL, 0x6462d92a69731732ULL, 582},
    {0xca28a291859bbf93ULL, 0x7d7b8f7503cfdcfeULL, 585},
    {0xfcb2cb35e702af78ULL, 0x5cda735244c3d43eULL, 588},
    {0x9defbf01b061adabULL, 0x3a0888136afa64a7ULL, 592},
    {0xc56baec21c7a1916ULL, 0x088aaa1845b8fdd0ULL, 595},
    {0xf6c69a72a3989f5bULL, 0x8aad549e57273d45ULL, 598},
    {0x9a3c2087a63f6399ULL, 0x36ac54e2f678864bULL, 602},
    {0xc0cb28a98fcf3c7fULL, 0x84576a1bb416a7ddULL, 605},
    {0xf0fdf2d3f3c30b9fULL, 0x656d44a2a11c51d5ULL, 608},
    {0x969eb7c47859e743ULL, 0x9f644ae5a4b1b325ULL, 612},
    {0xbc4665b596706114ULL, 0x873d5d9f0dde1feeULL, 615},
    {0xeb57ff22fc0c7959ULL, 0xa90cb506d155a7eaULL, 618},
    {0x9316ff75dd87cbd8ULL, 0x09a7f12442d588f2ULL, 622},
    {0xb7dcbf5354e9beceULL, 0x0c11ed6d538aeb2fULL, 625},
    {0xe5d3ef282a242e81ULL, 0x8f1668c8a86da5faULL, 628},
    {0x8fa475791a569d10ULL, 0xf96e017d694487bcULL, 632},
    {0xb38d92d760ec4455ULL, 0x37c981dcc395a9acULL, 635},
    {0xe070f78d3927556aULL, 0x85bbe253f47b1417ULL, 638},
    {0x8c469ab843b89562ULL, 0x93956d7478ccec8eULL, 642},
    {0xaf58416654a6babbULL, 0x387ac8d1970027b2ULL, 645},
    {0xdb2e51bfe9d0696aULL, 0x06997b05fcc0319eULL, 648},
    {0x88fcf317f22241e2ULL, 0x441fece3bdf81f03ULL, 652},
    {0xab3c2fddeeaad25aULL, 0xd527e81cad7626c3ULL, 655},
    {0xd60b3bd56a5586f1ULL, 0x8a71e223d8d3b074ULL, 658},
    {0x85c7056562757456ULL, 0xf6872d5667844e49ULL, 662},
    {0xa738c6bebb12d16cULL, 0xb428f8ac016561dbULL, 665},
    {0xd106f86e69d785c7ULL, 0xe13336d701beba52ULL, 668},
    {0x82a45b450226b39cULL, 0xecc0024661173473ULL, 672},
    {0xa34d721642b06084ULL, 0x27f002d7f95d0190ULL, 675},
    {0xcc20ce9bd35c78a5ULL, 0x31ec038df7b441f4ULL, 678},
    {0xff290242c83396ceULL, 0x7e67047175a15271ULL, 681},
    {0x9f79a169bd203e41ULL, 0x0f0062c6e984d386ULL, 685},
    {0xc75809c42c684dd1ULL, 0x52c07b78a3e60868ULL, 688},
    {0xf92e0c3537826145ULL, 0xa7709a56ccdf8a82ULL, 691},
    {0x9bbcc7a142b17ccbULL, 0x88a66076400bb691ULL, 695},
    {0xc2abf989935ddbfeULL, 0x6acff893d00ea435ULL, 698},
    {0xf356f7ebf83552feULL, 0x0583f6b8c4124d43ULL, 701},
    {0x98165af37b2153deULL, 0xc3727a337a8b704aULL, 705},
    {0xbe1bf1b059e9a8d6ULL, 0x744f18c0592e4c5cULL, 708},
    {0xeda2ee1c7064130cULL, 0x1162def06f79df73ULL, 711},
    {0x9485d4d1c63e8be7ULL, 0x8addcb5645ac2ba8ULL, 715},
    {0xb9a74a0637ce2ee1ULL, 0x6d953e2bd7173692ULL, 718},
    {0xe8111c87c5c1ba99ULL, 0xc8fa8db6ccdd0437ULL, 721},
    {0x910ab1d4db9914a0ULL, 0x1d9c9892400a22a2ULL, 725},
    {0xb54d5e4a127f59c8ULL, 0x2503beb6d00cab4bULL, 728},
    {0xe2a0b5dc971f303aULL, 0x2e44ae64840fd61dULL, 731},
    {0x8da471a9de737e24ULL, 0x5ceaecfed289e5d2ULL, 735},
    {0xb10d8e1456105dadULL, 0x7425a83e872c5f47ULL, 738},
    {0xdd50f1996b947518ULL, 0xd12f124e28f77719ULL, 741},
    {0x8a5296ffe33cc92fULL, 0x82bd6b70d99aaa6fULL, 745},
    {0xace73cbfdc0bfb7bULL, 0x636cc64d1001550bULL, 748},
    {0xd8210befd30efa5aULL, 0x3c47f7e05401aa4eULL, 751},
    {0x8714a775e3e95c78ULL, 0x65acfaec34810a71ULL, 755},
    {0xa8d9d1535ce3b396ULL, 0x7f1839a741a14d0dULL, 758},
    {0xd31045a8341ca07cULL, 0x1ede48111209a050ULL, 761},
    {0x83ea2b892091e44dULL, 0x934aed0aab460432ULL, 765},
    {0xa4e4b66b68b65d60ULL, 0xf81da84d5617853fULL, 768},
    {0xce1de40642e3f4b9ULL, 0x36251260ab9d668eULL, 771},
    {0x80d2ae83e9ce78f3ULL, 0xc1d72b7c6b426019ULL, 775},
    {0xa1075a24e4421730ULL, 0xb24cf65b8612f81fULL, 778},
    {0xc94930ae1d529cfcULL, 0xdee033f26797b627ULL, 781},
    {0xfb9b7cd9a4a7443cULL, 0x169840ef017da3b1ULL, 784},
    {0x9d412e0806e88aa5ULL, 0x8e1f289560ee864eULL, 788},
    {0xc491798a08a2ad4eULL, 0xf1a6f2bab92a27e2ULL, 791},
    {0xf5b5d7ec8acb58a2ULL, 0xae10af696774b1dbULL, 794},
    {0x9991a6f3d6bf1765ULL, 0xacca6da1e0a8ef29ULL, 798},
    {0xbff610b0cc6edd3fULL, 0x17fd090a58d32af3ULL, 801},
    {0xeff394dcff8a948eULL, 0xddfc4b4cef07f5b0ULL, 804},
    {0x95f83d0a1fb69cd9ULL, 0x4abdaf101564f98eULL, 808},
    {0xbb764c4ca7a4440fULL, 0x9d6d1ad41abe37f1ULL, 811},
    {0xea53df5fd18d5513ULL, 0x84c86189216dc5edULL, 814},
    {0x92746b9be2f8552cULL, 0x32fd3cf5b4e49bb4ULL, 818},
    {0xb7118682dbb66a77ULL, 0x3fbc8c33221dc2a1ULL, 821},
    {0xe4d5e82392a40515ULL, 0x0fabaf3feaa5334aULL, 824},
    {0x8f05b1163ba6832dULL, 0x29cb4d87f2a7400eULL, 828},
    {0xb2c71d5bca9023f8ULL, 0x743e20e9ef511012ULL, 831},
    {0xdf78e4b2bd342cf6ULL, 0x914da9246b255416ULL, 834},
    {0x8bab8eefb6409c1aULL, 0x1ad089b6c2f7548eULL, 838},
    {0xae9672aba3d0c320ULL, 0xa184ac2473b529b1ULL, 841},
    {0xda3c0f568cc4f3e8ULL, 0xc9e5d72d90a2741eULL, 844},
    {0x8865899617fb1871ULL, 0x7e2fa67c7a658892ULL, 848},
    {0xaa7eebfb9df9de8dULL, 0xddbb901b98feeab7ULL, 851},
    {0xd51ea6fa85785631ULL, 0x552a74227f3ea565ULL, 854},
    {0x8533285c936b35deULL, 0xd53a88958f87275fULL, 858},
    {0xa67ff273b8460356ULL, 0x8a892abaf368f137ULL, 861},
    {0xd01fef10a657842cULL, 0x2d2b7569b0432d85ULL, 864},
    {0x8213f56a67f6b29bULL, 0x9c3b29620e29fc73ULL, 868},
    {0xa298f2c501f45f42ULL, 0x8349f3ba91b47b8fULL, 871},
    {0xcb3f2f7642717713ULL, 0x241c70a936219a73ULL, 874},
    {0xfe0efb53d30dd4d7ULL, 0xed238cd383aa0110ULL, 877},
    {0x9ec95d1463e8a506ULL, 0xf4363804324a40aaULL, 881},
    {0xc67bb4597ce2ce48ULL, 0xb143c6053edcd0d5ULL, 884},
    {0xf81aa16fdc1b81daULL, 0xdd94b7868e94050aULL, 887},
    {0x9b10a4e5e9913128ULL, 0xca7cf2b4191c8326ULL, 891},
    {0xc1d4ce1f63f57d72ULL, 0xfd1c2f611f63a3f0ULL, 894},
    {0xf24a01a73cf2dccfULL, 0xbc633b39673c8cecULL, 897},
    {0x976e41088617ca01ULL, 0xd5be0503e085d813ULL, 901},
    {0xbd49d14aa79dbc82ULL, 0x4b2d8644d8a74e18ULL, 904},
    {0xec9c459d51852ba2ULL, 0xddf8e7d60ed1219eULL, 907},
    {0x93e1ab8252f33b45ULL, 0xcabb90e5c942b503ULL, 911},
    {0xb8da1662e7b00a17ULL, 0x3d6a751f3b936243ULL, 914},
    {0xe7109bfba19c0c9dULL, 0x0cc512670a783ad4ULL, 917},
    {0x906a617d450187e2ULL, 0x27fb2b80668b24c5ULL, 921},
    {0xb484f9dc9641e9daULL, 0xb1f9f660802dedf6ULL, 924},
    {0xe1a63853bbd26451ULL, 0x5e7873f8a0396973ULL, 927},
    {0x8d07e33455637eb2ULL, 0xdb0b487b6423e1e8ULL, 931},
    {0xb049dc016abc5e5fULL, 0x91ce1a9a3d2cda62ULL, 934},
    {0xdc5c5301c56b75f7ULL, 0x7641a140cc7810fbULL, 937},
    {0x89b9b3e11b6329baULL, 0xa9e904c87fcb0a9dULL, 941},
    {0xac2820d9623bf429ULL, 0x546345fa9fbdcd44ULL, 944},
    {0xd732290fbacaf133ULL, 0xa97c177947ad4095ULL, 947},
    {0x867f59a9d4bed6c0ULL, 0x49ed8eabcccc485dULL, 951},
    {0xa81f301449ee8c70ULL, 0x5c68f256bfff5a74ULL, 954},
    {0xd226fc195c6a2f8cULL, 0x73832eec6fff3111ULL, 957},
    {0x83585d8fd9c25db7ULL, 0xc831fd53c5ff7eabULL, 961},
    {0xa42e74f3d032f525ULL, 0xba3e7ca8b77f5e55ULL, 964},
    {0xcd3a1230c43fb26fULL, 0x28ce1bd2e55f35ebULL, 967},
    {0x80444b5e7aa7cf85ULL, 0x7980d163cf5b81b3ULL, 971},
    {0xa0555e361951c366ULL, 0xd7e105bcc332621fULL, 974},
    {0xc86ab5c39fa63440ULL, 0x8dd9472bf3fefaa7ULL, 977},
    {0xfa856334878fc150ULL, 0xb14f98f6f0feb951ULL, 980},
    {0x9c935e00d4b9d8d2ULL, 0x6ed1bf9a569f33d3ULL, 984},
    {0xc3b8358109e84f07ULL, 0x0a862f80ec4700c8ULL, 987},
    {0xf4a642e14c6262c8ULL, 0xcd27bb612758c0faULL, 990},
    {0x98e7e9cccfbd7dbdULL, 0x8038d51cb897789cULL, 994},
    {0xbf21e44003acdd2cULL, 0xe0470a63e6bd56c3ULL, 997},
    {0xeeea5d5004981478ULL, 0x1858ccfce06cac74ULL, 1000},
    {0x95527a5202df0ccbULL, 0x0f37801e0c43ebc8ULL, 1004},
    {0xbaa718e68396cffdULL, 0xd30560258f54e6baULL, 1007},
    {0xe950df20247c83fdULL, 0x47c6b82ef32a2069ULL, 1010},
    {0x91d28b7416cdd27eULL, 0x4cdc331d57fa5441ULL, 1014},
    {0xb6472e511c81471dULL, 0xe0133fe4adf8e952ULL, 1017},
    {0xe3d8f9e563a198e5ULL, 0x58180fddd97723a6ULL, 1020},
    {0x8e679c2f5e44ff8fULL, 0x570f09eaa7ea7648ULL, 1024},
};
#define K_EL_POW10_MIN (-342)
#define K_EL_POW10_MAX (308)


/* Correctly-rounded decimal-to-double on the fast path (eisel-lemire,
   "number parsing at a gigabyte per second", 2021), with the exact
   small-case (clinger, PLDI 1990) in front and strtod behind: returns 1
   and writes *out when certain, 0 to defer. w is the digit significand,
   q the decimal exponent — value = w * 10^q. */
static int k_el_parse(unsigned long long w, long long q, double* out) {
    if (K_COUNTING) k_stat_el_parses++;
    if (w == 0) { *out = 0.0; return 1; }
    static const double exact10[23] = {
        1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11,
        1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22
    };
    if (w < (1ULL << 53) && q >= -22 && q <= 22) {
        double d = (double)w;
        *out = q >= 0 ? d * exact10[q] : d / exact10[-q];
        return 1;
    }
    if (q < K_EL_POW10_MIN || q > K_EL_POW10_MAX) {
        /* certain under/overflow */
        if (q < K_EL_POW10_MIN) { *out = 0.0; return 1; }
        *out = 1.0 / 0.0;
        return 1;
    }
    int lz = __builtin_clzll(w);
    unsigned long long wn = w << lz;
    const unsigned long long mhi = k_el_pow10[q - K_EL_POW10_MIN].hi;
    const unsigned long long mlo = k_el_pow10[q - K_EL_POW10_MIN].lo;
    __uint128_t p = (__uint128_t)wn * mhi;
    unsigned long long phi = (unsigned long long)(p >> 64);
    unsigned long long plo = (unsigned long long)p;
    /* refine with the low table word when the truncated product is too
       close to a rounding boundary to be trusted */
    if ((phi & 0x1FF) == 0x1FF) {
        __uint128_t p2 = (__uint128_t)wn * mlo;
        unsigned long long carry = plo + (unsigned long long)(p2 >> 64) < plo;
        plo += (unsigned long long)(p2 >> 64);
        phi += carry;
        if ((phi & 0x1FF) == 0x1FF && plo + 1 == 0) return 0;
    }
    /* value = (phi + plo/2^64) * 2^(e2t - lz), phi's leading bit at
       62+msb: binary exponent E = 62 + msb + e2t - lz. */
    int msb = (int)(phi >> 63);
    int e_val = 62 + msb + k_el_pow10[q - K_EL_POW10_MIN].e2 - lz;
    int biased = e_val + 1023;
    if (biased <= 0 || biased >= 2047) return 0; /* subnormal/inf: defer */
    int shift = 10 + msb;
    unsigned long long m = phi >> shift;
    unsigned long long round_bit = (phi >> (shift - 1)) & 1;
    unsigned long long sticky = (phi & ((1ULL << (shift - 1)) - 1)) | plo;
    if (round_bit && sticky == 0) return 0; /* halfway under truncation: defer */
    m += round_bit & (sticky != 0 || (m & 1));
    if (m >= (1ULL << 53)) {
        m >>= 1;
        biased += 1;
        if (biased >= 2047) return 0;
    }
    unsigned long long bits =
        ((unsigned long long)biased << 52) | (m & ((1ULL << 52) - 1));
    double d;
    __builtin_memcpy(&d, &bits, 8);
    *out = d;
    return 1;
}


KValue k_b_to_float(KValue v, const char* origin) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_FLOAT) return v;
    if (v.tag == K_INT) return k_float((double)v.payload);
    if (v.tag == K_BIG) return k_float(k_int_to_f(v));
    if (v.tag != K_STR && v.tag != K_BYTES) k_die_value("to_float takes a string, bytes, or number", v);
    const char* data;
    long long len;
    if (v.tag == K_STR) { KStr* s = k_as_str(v); data = s->data; len = s->len; }
    else { KBytes* b = k_as_bytes(v); data = (const char*)b->data; len = b->len; }
    return k_to_float_text(data, len, len, origin);
}

static const unsigned long long k_pow10_small[8] = {
    1, 10, 100, 1000, 10000, 100000, 1000000, 10000000,
};

/* `[-]I.F` with one to eight digits of I and one to seven of F, read as two
   words that end at the dot and at the last byte. The fraction's length is
   the run of digits at the top of the last word, so the dot is found without
   a scan, and the integer part must fill exactly the bytes between the sign
   and the dot. What comes out is the (w, q) the scan below would build from
   the same digits, and it goes to the same eisel-lemire. `room` is how many
   bytes of the buffer end at data + len; both words must lie inside it. */
static inline int k_float_shape8(const char* data, long long len, long long room,
                                 double* out) {
    if (room < 16 || len < 3 || len > 17) return 0;
    const char* stop = data + len;
    uint64_t fw;
    memcpy(&fw, stop - 8, 8);
    uint64_t fbad = k_nondigits8(fw);
    long long f = fbad ? (long long)(__builtin_clzll(fbad) >> 3) : 8;
    if (f < 1 || f > 7 || stop[-f - 1] != '.') return 0;
    const char* dot = stop - f - 1;
    long long s = data[0] == '-' ? 1 : 0;
    long long i = (long long)(dot - data) - s;
    if (i < 1 || i > 8) return 0;
    uint64_t iw;
    memcpy(&iw, dot - 8, 8);
    if (k_nondigits8(iw) & (~0ULL << (8 * (8 - i)))) return 0;
    unsigned long long w = (unsigned long long)k_digits8_value(iw, i) * k_pow10_small[f]
                         + (unsigned long long)k_digits8_value(fw, f);
    double d;
    if (!k_el_parse(w, -f, &d)) return 0;
    *out = s ? -d : d;
    return 1;
}

/* Inlined into both doors by name. With the shape above in it, clang stopped
   inlining it into k_b_to_float, and widebench's 72,000 parses paid the
   call: 272,000 instructions. */
static inline __attribute__((always_inline)) KValue
k_to_float_text(const char* data, long long len, long long room, const char* origin) {
    {
        double shaped;
        if (k_float_shape8(data, len, room, &shaped)) return k_float(shaped);
    }
    /* the fast path: a plain decimal scanned into (w, q) and parsed by
       eisel-lemire; anything it can't be certain about — overlong digits,
       exotic forms, halfway cases — falls through to strtod, which stays
       the semantic authority.

       `cut` is why the overlong case is one of those. The scan keeps
       nineteen significant digits; past that an integer digit is traded for
       a `q++` and a fraction digit is dropped outright, and either way the
       significand handed on is no longer the number that was written. A
       truncated significand can sit on the far side of a rounding boundary
       from the true value, so eisel-lemire's answer is not certain and the
       algorithm does not claim it is — Lemire's own implementation carries
       the same flag. Without it this returned a double one ULP below the
       correctly-rounded one on `4409065699.4409065699e-2` and 220 other
       cases the parse harness found, and the interpreter — the oracle —
       disagreed with native on every one of them.

       A dropped ZERO changes nothing, so only a nonzero one sets the flag:
       "10000000000000000000" is exactly w * 10 and stays on the fast path.
       That is what keeps the run corpus at 210,177 fast-path parses, which
       is every call it makes. */
    if (len > 0) {
        const char* p = data;
        const char* stop = data + len;
        int neg = 0;
        if (*p == '-' || *p == '+') { neg = *p == '-'; p++; }
        unsigned long long w = 0;
        long long q = 0;
        int digits = 0, any = 0, ok = 1, cut = 0;
        /* leading zeros are not significant digits, and once the first
           nonzero digit has landed every later one is. skipping them here
           lets both loops count unconditionally instead of re-asking
           `if (w)` on every digit -- a predicate that is monotone in the
           digit position and was costing three instructions a digit. */
        while (p < stop && *p == '0') { any = 1; p++; }
        while (p < stop && *p >= '0' && *p <= '9') {
            any = 1;
            if (digits < 19) { w = w * 10 + (unsigned long long)(*p - '0'); digits++; }
            else { q++; if (*p != '0') cut = 1; }
            p++;
        }
        if (p < stop && *p == '.') {
            p++;
            if (w == 0) {
                while (p < stop && *p == '0') { any = 1; q--; p++; }
            }
            while (p < stop && *p >= '0' && *p <= '9') {
                any = 1;
                if (digits < 19) {
                    w = w * 10 + (unsigned long long)(*p - '0');
                    digits++;
                    q--;
                }
                else if (*p != '0') cut = 1;
                p++;
            }
        }
        if (p < stop && (*p == 'e' || *p == 'E')) {
            p++;
            int esign = 1;
            if (p < stop && (*p == '-' || *p == '+')) { esign = *p == '-' ? -1 : 1; p++; }
            long long e = 0;
            int edigits = 0;
            while (p < stop && *p >= '0' && *p <= '9') {
                if (e < 100000) e = e * 10 + (*p - '0');
                edigits++;
                p++;
            }
            if (!edigits) ok = 0;
            q += esign * e;
        }
        if (ok && any && !cut && p == stop) {
            double out;
            if (k_el_parse(w, q, &out)) {
                return k_float(neg ? -out : out);
            }
        }
    }
    /* The fallback parses a terminated copy, for the reason k_b_to_int_slow
       gives: a range read out of the middle of bytes has more digits after
       its last one, and strtod would read them. */
    char* end = NULL;
    char small[64];
    char* copy = len < 64 ? small : malloc((size_t)len + 1);
    memcpy(copy, data, (size_t)len);
    copy[len] = 0;
    double d = strtod(copy, &end);
    /* strtod reads more than the interpreter's parse does: leading space, a
       hex float (`0x1p3`) and a nan with a payload (`nan(1)`). None of them
       is a number on the oracle, so none of them is one here. */
    int whole = len != 0 && end == copy + len && !k_num_space(copy[0])
        && !memchr(copy, 'x', (size_t)len) && !memchr(copy, 'X', (size_t)len)
        && !memchr(copy, '(', (size_t)len);
    if (copy != small) free(copy);
    if (!whole) return k_not_a_number(data, len, "\" is not a number", "bytes are not a number", origin);
    return k_float(d);
}

