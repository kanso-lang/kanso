/* What a succeeded effect yields: `none` is absence and nothing else. */
KValue k_done(void) { KValue v; v.tag = K_DONE; v.payload = 0; return v; }

KValue k_ascii_cache[128];
char k_ascii_ready[128];

/* A string's header and its bytes have one lifetime and are made together,
   so they are one allocation: the payload sits immediately after the header
   and `data` points at it. Callers that adopt storage they did not make —
   the zero-copy finish, a deep copy that shares — assign `data` themselves
   and are unaffected. */
static inline __attribute__((always_inline)) KStr* k_str_alloc(long long len) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_str += (long long)((sizeof(KStr) + (size_t)len + 1 + 15) & ~(size_t)15);
    if (len > 2147483647LL) k_die("string too long");
    KStr* s = k_alloc(sizeof(KStr) + (size_t)len + 1);
    s->len = (int)len;
    s->data = (char*)(s + 1);
    s->cap = 0;
    return s;
}

/* An int interpolated on its own, `"{i}"`, is the common case: the run
   program makes 200,000 of them. Through k_render it went into a stack buffer
   by way of the switch every value takes, and k_str_n then copied the digits
   into a fresh string, about 109 instructions an int. The length is known
   before the first digit, so the string is allocated at that length and the
   digits are written into it where they will stay. */
KValue k_str_n(const char* data, long long len);
static KValue k_big_render(KValue x);
/* Everything but an int, kept out of line: with it inline the int path paid
   six register saves on every call for paths it never takes. */
static __attribute__((noinline)) KValue k_render_value_rest(KValue v) {
    if (v.tag == K_BIG) return k_big_render(v);
    return k_render(v, 0);
}

KValue k_b_render_value(KValue v) {
    if (v.tag == K_INT) {
        long long x = v.payload;
        /* One digit is a one-byte string, which k_str_n answers from the
           ascii cache without allocating; so does this. */
        if ((unsigned long long)x < 10) {
            char c = (char)('0' + x);
            return k_str_n(&c, 1);
        }
        uint64_t u = x < 0 ? (uint64_t)(-(x + 1)) + 1 : (uint64_t)x;
        long long n = (long long)ryu_declen(u) + (x < 0);
        KStr* s = k_str_alloc(n);
        if (x < 0) s->data[0] = '-';
        k_digits_into(s->data + n, u);
        s->data[n] = 0;
        KValue out; out.tag = K_STR; out.payload = k_ptr(s); return out;
    }
    return k_render_value_rest(v);
}

/* A slice of a long string shares the string's bytes: a header whose `data`
   points into the parent. Copying costs the bytes twice while both are live,
   and the parent is usually dying as its slice is made -- the index phase of
   the run program slices 1,380,000 bytes out of a 1,572,864-byte string that
   nothing reads again, and the copy was the top of the program's peak. A
   parent with room (`cap > 0`) is a builder, whose storage moves and is
   freed as it grows, so it is copied as before; any other string's bytes
   live exactly as long as the string does, and a holder of the view holds
   them for that long too. Evacuation copies a view's own bytes and nothing
   around them.

   The view does not end in a terminator, since the byte after it is its
   parent's. Nothing in the runtime reads one except what hands a string to
   the C library, which goes through `k_cstr`, and the copies, which write
   their own. Short slices keep the copy: below this length the header and
   the bytes fit the same sixteen-byte rounding the view's header alone
   takes, and a copy has no parent to hold. */
#define K_STR_VIEW_MIN 64
static KValue k_str_view(KStr* parent, long long at, long long len, long long chars) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_str += (long long)sizeof(KStr);
    KStr* s = k_alloc(sizeof(KStr));
    s->len = (int)len;
    s->data = parent->data + at;
    s->cap = chars < 2147483647LL ? (int)(-chars - 1) : 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

/* A string as the C library wants it, ending in a zero byte. Every string
   the runtime makes carries one except a view, whose next byte belongs to
   its parent; that one is copied. */
static char* k_cstr(KStr* s) {
    if (s->data[s->len] == 0) return s->data;
    char* c = k_alloc((size_t)s->len + 1);
    memcpy(c, s->data, (size_t)s->len);
    c[s->len] = 0;
    return c;
}

/* A string literal evaluates to the same value every time, so it is built
   once, in permanent storage, and the slot hands it back thereafter. The
   emitter passes one slot per interned literal; the ascii cache below is
   this same idea for the single-character strings every program shares. */
KValue k_str_lit(const char* data, long long len, KValue* slot) {
    if (slot->tag != K_STR) {
        KStr* s = k_alloc_const(sizeof(KStr) + (size_t)len + 1);
        s->len = (int)len;
        s->data = (char*)(s + 1);
        s->cap = 0;
        memcpy(s->data, data, (size_t)len);
        s->data[len] = 0;
        slot->tag = K_STR;
        slot->payload = k_ptr(s);
    }
    return *slot;
}

/* Inlined at every call site, which is a trade the objective priced rather
   than a free win. It is 1,305,302 calls on jsonbench alone, 46 instructions
   apiece, and most of a short string's cost is the call rather than the arena
   bump and the memcpy inside it. Inlining it makes ten work rows fall and one
   rise — readbench by 1.886% — and grows every program's `.text` by 400 to
   4,368 bytes. Opening the door at only the hot caller instead keeps readbench
   still and every `.text` row still, and scores 73.79 against this shape's
   73.82. The sum is the objective and the terms are diagnostics, so this is
   the shape that ships; the narrow one is recorded in the log beside it. */
__attribute__((always_inline))
/* The ascii cache's first fill of a slot, out of line and preserve_most so
   the hit path in k_str_n carries no frame into its callers. k_str_n is
   inlined everywhere the runtime uses it, and with the fill in its body two
   of those callers stopped inlining it. */
static __attribute__((noinline, cold, preserve_most)) KValue k_ascii_fill(unsigned char b) {
    KStr* s = k_alloc_const(sizeof(KStr) + 2);
    s->len = 1;
    s->cap = 0;
    s->data = (char*)(s + 1);
    s->data[0] = (char)b;
    s->data[1] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s);
    k_ascii_cache[b] = v;
    k_ascii_ready[b] = 1;
    return v;
}

/* always_inline: with the fill outlined the inliner declined this at two
   sites (k_b_slice and k_b_utf8_slice_raw) and 205,098 calls a run paid
   twenty-three instructions each. Nothing emitted calls it; the symbol is
   declared to the IR and never used. */
__attribute__((always_inline)) KValue k_str_n(const char* data, long long len) {
    if (len == 1) {
        unsigned char b = (unsigned char)data[0];
        if (b < 128 && b != 0) {
            if (!k_ascii_ready[b]) return k_ascii_fill(b);
            return k_ascii_cache[b];
        }
    }
    KStr* s = k_str_alloc(len);
    k_copy_short(s->data, data, len);
    s->data[len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

static KValue k_str(const char* data) { return k_str_n(data, (long long)strlen(data)); }

static KValue k_accumulate_failures(KValue l, KValue r);

/* Two failures in one operation merge, the way two in a parallel group do
   (Clay, 2026-08-05): neither side caused the other, and the one that lost a
   race to be first is not less true. One failure propagates as itself. */
static KValue k_both_or_either(KValue a, KValue b) {
    return k_accumulate_failures(a, b);
}

long long k_not_failure(KValue v) { return v.tag != K_ERR; }

/* Every failing argument of a builtin answers together, merged the way two
   failing operands of `+` are, before any argument is asked what it is: the
   interpreter's call_builtin tests them all and merges what fails, where these
   used to answer the first failing argument and, in push, to refuse a
   non-list before looking at a failing item at all. Out of line and cold,
   because the scan in front of it is the hot path and this runs only when an
   argument has already failed. */
static KValue k_failed_of(int n, const KValue* xs) {
    KValue out = xs[0];
    int have = 0;
    for (int i = 0; i < n; i++) {
        if (k_not_failure(xs[i])) continue;
        out = have ? k_accumulate_failures(out, xs[i]) : xs[i];
        have = 1;
    }
    return out;
}

__attribute__((noinline, cold)) static KValue k_failed2(KValue a, KValue b) {
    KValue xs[2] = {a, b};
    return k_failed_of(2, xs);
}

__attribute__((noinline, cold)) static KValue k_failed3(KValue a, KValue b, KValue c) {
    KValue xs[3] = {a, b, c};
    return k_failed_of(3, xs);
}


/* `any` is every value a slot may hold; the absence channel is disjoint */

static KErrBox* k_err_box(KValue v) { return (KErrBox*)(intptr_t)v.payload; }

/* Every field, every time. The arena bumps and does not zero, so a
   construction that leaves a field out reads whatever the last user of those
   bytes wrote. That is not theoretical: k_hop and k_b_wrap_err both skipped
   `merged`, so a merged err that passed through a function came back claiming
   its reason list was one reason rather than several, and three failures
   printed as [["e1" "e2"] "e3"] on native against ["e1" "e2" "e3"] on the
   interpreter. One constructor now, and a field added later cannot be
   forgotten by three call sites at once. */
static KErrBox* k_err_box_new(KValue reason, const char* origin, const char* hako,
                              KHop* hops, KErrBox* cause, long long merged) {
    KErrBox* box = k_alloc(sizeof(KErrBox));
    box->reason = reason;
    box->origin = origin;
    box->hako = hako;
    box->hops = hops;
    box->cause = cause;
    box->merged = merged;
    return box;
}

/* The trace half of a raise site's literal: past the package name's NUL. */
static const char* k_origin_text(const char* both) {
    return both ? both + strlen(both) + 1 : NULL;
}

KValue k_err(KValue reason, const char* origin) {
    if (!k_not_failure(reason)) return reason;
    KErrBox* box = k_err_box_new(reason, k_origin_text(origin), origin, NULL, NULL, 0);
    KValue v; v.tag = K_ERR; v.payload = k_ptr(box); return v;
}

/* The one hole in err's infectiousness: the wrapped err arrives as a
   value instead of propagating, and rides along as the new err's cause
   so a re-raise never discards what it was told. */
KValue k_b_wrap_err(KValue reason, KValue original, const char* origin) {
    if (!k_not_failure(reason)) return reason;
    /* a program hands it an err out of its box; annotate hands the box */
    if (original.tag != K_ERRV && original.tag != K_ERR)
        k_die("wrap_err takes a reason and the err it wraps");
    KErrBox* box =
        k_err_box_new(reason, k_origin_text(origin), origin, NULL, k_err_box(original), 0);
    KValue v; v.tag = K_ERR; v.payload = k_ptr(box); return v;
}

/* A dispatcher passing a failure through appends its name; none stays bare. */
KValue k_err_hop(KValue v, const char* fn) {
    if (v.tag != K_ERR) return v;
    // a field read is not a call the reader made, and its internal name is
    // not theirs to see — the getter prefix no source can spell
    if (strncmp(fn, "Get_", 4) == 0) return v;
    KErrBox* old = k_err_box(v);
    KHop* hop = k_alloc(sizeof(KHop));
    hop->fn = fn;
    hop->prev = old->hops;
    KErrBox* box = k_err_box_new(old->reason, old->origin, old->hako, hop, old->cause,
                                 old->merged);
    KValue out; out.tag = K_ERR; out.payload = k_ptr(box); return out;
}

/* Zero-field marker types (null, enum tags) have exactly one inhabitant, so
   every instance is interchangeable — intern one per type id instead of
   allocating. json's 2000+ nulls in a document collapse to a single record. */
#define K_MARKER_CACHE 256
static KValue k_marker_cache[K_MARKER_CACHE];
static char k_marker_ready[K_MARKER_CACHE];

/* Building a record from one that is finished writes into its storage rather
   than asking for more. `point (p.x + 1) p.y` reads every field it needs
   before the constructor runs, so by then `p` is done with, and the analysis
   is what says nobody else holds it. The width test is this side's guard: a
   record of a different shape, or the shared zero-field marker, falls through
   to a fresh allocation. */
KValue k_rec(long long type_id, long long n, KValue* args);

/* A record's fields are one operation, so two failing fields merge the way
   two failing operands of `+` do — neither caused the other and neither is
   less true. Native and wasm both used to hand back the FIRST failing field
   here, where the interpreter merged; the corpus never built a record from
   two failures and the three engines disagreed unwatched.

   Out of line and never inlined, because the caller's scan is the hot path:
   every record construction walks its fields, and folding the merge into that
   walk cost pendbench 3.2% and oneshot 0.9% for work that runs only when a
   field has already failed. The scan keeps its early exit; this takes over
   from the field that failed. */
__attribute__((noinline, cold)) static KValue k_merge_rest(long long n, KValue* args, long long at) {
    KValue out = args[at];
    for (long long i = at + 1; i < n; i++) {
        if (!k_not_failure(args[i])) out = k_accumulate_failures(out, args[i]);
    }
    return out;
}

/* The same merge for the register-returnable construction the codegen builds
   in tail position, where the two fields are in registers rather than an
   array. At least one of them failed; the caller tested that. */
KValue k_pair_failure(KValue a, KValue b) {
    KValue args[2];
    args[0] = a;
    args[1] = b;
    return k_merge_rest(2, args, k_not_failure(a) ? 1 : 0);
}

/* Inside a beat loop a record is reused only where the arena has put it since
   the loop's mark. A record below the mark survives every rewind, so writing
   a fresh field into it makes the next stage repair it: the field's storage
   is copied under the mark and the mark is raised over it. The next pass
   writes a newer field, and the copy is dead under the mark until the loop
   ends -- a loop interpolating a 1 MiB string into its reused state held
   420 MB at 400 passes. Outside the mark the record is built fresh, which is
   one allocation a stage rather than a megabyte a pass. */
static inline int k_reuse_above_mark(const KRec* r) {
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return 1;
    const KMark* m = &k_beat_stack[k_beat_depth - 1];
    if (m->block != k_blocks) return 0;
    const char* p = (const char*)r;
    return p >= m->ptr && p < k_arena;
}

KValue k_rec_reuse(long long type_id, long long n, KValue* args, KValue victim) {
    for (long long i = 0; i < n; i++)
        if (__builtin_expect(!k_not_failure(args[i]), 0)) return k_merge_rest(n, args, i);
    if (n > 0 && victim.tag == K_REC) {
        KRec* r = k_as_rec(victim);
        if (r->nfields == n && k_reuse_above_mark(r)) {
            for (long long i = 0; i < n; i++) r->fields[i] = args[i];
            r->type_id = type_id;
            return victim;
        }
    }
    return k_rec(type_id, n, args);
}

KValue k_rec(long long type_id, long long n, KValue* args) {
    for (long long i = 0; i < n; i++)
        if (__builtin_expect(!k_not_failure(args[i]), 0)) return k_merge_rest(n, args, i);
    if (n == 0 && type_id >= 0 && type_id < K_MARKER_CACHE) {
        if (!k_marker_ready[type_id]) {
            KRec* r = k_alloc_const(sizeof(KRec));
            r->type_id = type_id;
            r->nfields = 0;
            r->fields = NULL;
            KValue v; v.tag = K_REC; v.payload = k_ptr(r);
            k_marker_cache[type_id] = v;
            k_marker_ready[type_id] = 1;
        }
        return k_marker_cache[type_id];
    }
    /* A record's field count is fixed at construction and `set` writes
       through the existing slots, so the fields live immediately after the
       header rather than in a second allocation. The deep copier builds its
       own storage and assigns `fields` itself, which stays correct. */
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_rec += (long long)((sizeof(KRec) + sizeof(KValue) * (size_t)n + 15) & ~(size_t)15);
    KRec* r = k_alloc(sizeof(KRec) + sizeof(KValue) * (size_t)n);
    r->type_id = type_id;
    r->nfields = n;
    r->fields = (KValue*)(r + 1);
    /* A record has two or three fields far more often than it has many, and
       a call into glibc's memcpy costs twelve instructions before it moves a
       byte. Copying the common counts as whole KValues is four instructions a
       field and no call; anything longer keeps the call, where the vector path
       earns its entry. Measured: 3,344,400 of these a run in encodebench. */
    if (n <= 4) {
        KValue* dst = r->fields;
        for (long long i = 0; i < n; i++) dst[i] = args[i];
    } else {
        memcpy(r->fields, args, sizeof(KValue) * n);
    }
    KValue v; v.tag = K_REC; v.payload = k_ptr(r); return v;
}

static KValue k_sub_base(KValue v);

/* A pattern naming a type takes a subtype of it, so the check and the field
   read that follows both see through the wrapper — the same rule the oracle
   follows, and the reason a getter reads a subtype at all. */
KValue k_field(KValue v, long long i) {
    if (v.tag == K_SUB) v = k_sub_base(v);
    return k_as_rec(v)->fields[i];
}

/* The by-value record convention packs a two-field record into three
   words: field 1's tag in the low byte of the first with field 0's tag
   above it, field 1's payload in the second, field 0's payload in the
   third. Field 0 is an int, a word or a bignum's pointer, and either fits.
   A failure crosses in the first two words untouched: its first word is
   exactly K_ERR, and no packed value's first word is, since field 1 is a
   value the analysis proved is never a failure. These are the only
   conversions between the two shapes, and all of them pass a failure
   through. Until 2026-09-16 the emitter inlined them and neither did, and a
   record arm on the far side of a beat carry matched a failure. */
KValue k_parsed_box(long long type_id, long long w0, long long w1, long long w2) {
    KValue words; words.tag = w0; words.payload = w1;
    if (!k_not_failure(words)) return words;
    KValue fields[2];
    fields[0].tag = (unsigned long long)w0 >> 8; fields[0].payload = w2;
    fields[1].tag = w0 & 255; fields[1].payload = w1;
    return k_rec(type_id, 2, fields);
}
/* The first two words of a boxed record's packing; k_parsed_int is the
   third. */
KValue k_parsed_words(KValue v) {
    if (!k_not_failure(v)) return v;
    KValue f0 = k_field(v, 0), f1 = k_field(v, 1);
    KValue out;
    out.tag = (long long)((unsigned long long)f0.tag << 8) | f1.tag;
    out.payload = f1.payload;
    return out;
}
long long k_parsed_int(KValue v) {
    if (!k_not_failure(v)) return 0;
    return k_field(v, 0).payload;
}
KValue k_err_inner(KValue v) { return k_err_box(v)->reason; }

/* An err's three readers, `.reason`, `.cause` and `.origin` — the second
   deliberate hole in infectiousness beside `wrap_err`'s second argument. The
   dispatcher of a reader's getter calls this at its entry, before any arm,
   and only for an err; the interpreter's `err_read` is the oracle. */
long long k_is_err(KValue v) { return v.tag == K_ERRV; }
KValue k_err_read(KValue v, const char* field) {
    KErrBox* box = k_err_box(v);
    if (field[0] == 'r') return box->reason;
    if (field[0] == 'c') {
        if (!box->cause) return k_none();
        KValue out; out.tag = K_ERRV; out.payload = k_ptr(box->cause); return out;
    }
    if (!box->origin) return k_none();
    return k_str(box->origin);
}

/* pattern checks: nonzero on match */
long long k_check_tag(KValue v, long long tag) { return v.tag == tag; }
/* a box: an effect, or a failure, which `err` makes boxed (#2087) */
long long k_check_box(KValue v) { return v.tag == K_DESC || v.tag == K_ERR; }
/* `int` is two tags, a word and a bignum, and an annotation accepts either. */
long long k_check_int_type(KValue v) { return K_IS_INT(v); }
/* A pattern's literal past a word: only a bignum can equal it, and the digits
   are read and compared whole. */
long long k_check_big_lit(KValue v, const char* s, long long len) {
    return v.tag == K_BIG && k_int_cmp(v, k_int_of_digits(s, len)) == 0;
}
/* `some` is a value that is not none. A failure is neither, and answering
   otherwise made an arm that names no err at all take a foreign failure. */
long long k_check_some(KValue v) { return v.tag != K_NONE && v.tag != K_ERR; }
long long k_check_int(KValue v, long long n) { return v.tag == K_INT && v.payload == n; }
long long k_check_rec(KValue v, long long type_id, long long nfields) {
    /* A pattern naming a type takes a subtype of it, so the walk looks for
       the named type at each level before reaching the base: one naming the
       wrapper matches the wrapper, one naming what it wraps matches through
       it. The fields bound are the base record's either way, so the arity
       is checked there. */
    while (v.tag == K_SUB) {
        KSub* w = (KSub*)(intptr_t)v.payload;
        if (w->type_id == type_id) {
            KValue base = k_sub_base(v);
            return base.tag == K_REC && k_as_rec(base)->nfields == nfields;
        }
        v = w->inner;
    }
    return v.tag == K_REC && k_as_rec(v)->type_id == type_id
        && k_as_rec(v)->nfields == nfields;
}
long long k_check_bool(KValue v) { return v.tag == K_TRUE || v.tag == K_FALSE; }

/* One allocation for a whole template: sums the piece lengths, copies
   once. A failure piece propagates; the profile showed chained k_concat
   quadratic-copying hot on the encode path. An array, not varargs —
   16-byte structs through va_arg differ between the arm64 and x86_64
   ABIs when the caller is emitted IR. */
KValue k_concat_arr(long long n, const KValue* parts);

/* The seed of a string builder, made where `text/bytes ""` is made: its own
   header, outside the loop that will grow it, with malloc'd storage the
   arena's rewinds cannot reach. Every later join writes into this same header,
   which is what lets the fold-state shelf carry it across a beat by identity
   instead of copying it. */
KValue k_sub_base(KValue v);

/* A seed that is not a string stays what it is. `"{p}!"` renders whatever `p`
   holds, so the group is a builder whatever its callers hand in, and a seed of
   `id 13` is legal; answered unjoined by a base arm, it has to come back as
   `id 13`. The first join finds no room in what it renders and makes the
   builder there, in `k_concat_arr_mut`. A subtype of string builds from the
   string it wraps. */
KValue k_b_str_builder(KValue sv) {
    if (!k_not_failure(sv)) return sv;
    KValue wrapped = sv.tag == K_SUB ? k_sub_base(sv) : sv;
    if (wrapped.tag != K_STR) return sv;
    KStr* src = k_as_str(wrapped);
    long long cap = (long long)src->len * 2 + 32;
    char* base = malloc((size_t)(K_STR_HEAD + cap + 1));
    if (!base) { fputs("out of memory\n", stderr); exit(1); }
    char* room = base + K_STR_HEAD;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += cap + 1;
        if (K_COUNTING) k_stat_bytes_malloc++;
        k_stat_sh_str += (long long)sizeof(KStr);
    }
    memcpy(room, src->data, (size_t)src->len);
    room[src->len] = 0;
    KStr* out = k_alloc(sizeof(KStr));
    out->len = src->len;
    out->data = room;
    out->cap = (int)cap;
    k_str_count(out) = k_str_chars(src);
    KValue v; v.tag = K_STR; v.payload = k_ptr(out);
    return v;
}


/* Joining into a builder. Once the builder exists every join writes into the
   header it was given, because the shelf carries that header across a rewind.
   The first join may find a seed with no room: a fold's literal, a value that
   was not a string, or a string bound lazily. It makes the builder there. */
KValue k_concat_arr_mut(long long n, const KValue* parts) {
    if (n == 0) return k_concat_arr(n, parts);
    for (long long i = 0; i < n; i++) {
        if (!k_not_failure(parts[i])) return parts[i];
    }
    KStr* acc = k_as_str(parts[0]);
    /* A fold's folder joins onto the accumulator the fold hands it, and the
       seed arrives as whatever the caller wrote, usually a literal. The first
       join makes the builder; every later one finds it. */
    if (acc->cap <= 0) {
        KValue built = k_b_str_builder(parts[0]);
        KValue* writable = (KValue*)parts;
        writable[0] = built;
        acc = k_as_str(built);
    }
    long long extra = 0;
    for (long long i = 1; i < n; i++) extra += k_as_str(parts[i])->len;
    long long want = (long long)acc->len + extra;
    if (want > 2147483647LL) k_die("string too long");
    if (want > acc->cap) {
        long long cap = want * 2 + 32;
        if (cap > 2147483647LL) cap = 2147483647LL;
        char* base = malloc((size_t)(K_STR_HEAD + cap + 1));
        if (!base) { fputs("out of memory\n", stderr); exit(1); }
        char* room = base + K_STR_HEAD;
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
            if (K_COUNTING) k_stat_allocs++;
            k_stat_alloc_bytes += cap + 1;
            if (K_COUNTING) k_stat_bytes_malloc++;
        }
        memcpy(room, acc->data, (size_t)acc->len);
        long long carried = k_str_count(acc);
        free(acc->data - K_STR_HEAD);
        acc->data = room;
        acc->cap = (int)cap;
        k_str_count(acc) = carried;
    }
    long long at = acc->len;
    for (long long i = 1; i < n; i++) {
        KStr* ps = k_as_str(parts[i]);
        memcpy(acc->data + at, ps->data, (size_t)ps->len);
        /* Each part counts its own characters once and remembers; the sum is
           the whole, because a part is a string and a string is whole
           characters. Nothing walks what is already in the builder. */
        k_str_count(acc) += k_str_chars(ps);
        at += ps->len;
    }
    acc->data[want] = 0;
    acc->len = (int)want;
    return parts[0];
}

KValue k_concat_arr(long long n, const KValue* parts) {
    long long total = 0;
    for (long long i = 0; i < n; i++) {
        KValue p = parts[i];
        if (!k_not_failure(p)) return p;
        total += k_as_str(p)->len;
    }
    KStr* s = k_str_alloc(total);
    long long at = 0;
    for (long long i = 0; i < n; i++) {
        KStr* ps = k_as_str(parts[i]);
        memcpy(s->data + at, ps->data, ps->len);
        at += ps->len;
    }
    s->data[total] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s);
    return v;
}

KValue k_concat(KValue a, KValue b) {
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    KStr* sa = k_as_str(a);
    KStr* sb = k_as_str(b);
    KStr* s = k_str_alloc(sa->len + sb->len);
    memmove(s->data, sa->data, sa->len);
    memmove(s->data + sa->len, sb->data, sb->len);
    s->data[s->len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

extern const char* k_type_name(long long type_id);
/* the spelling a rendered record prints; k_type_name is the identity */
extern const char* k_type_shown(long long type_id);
static const char* k_lazy_hint(KValue v);
extern long long k_type_field_count(long long type_id);
extern const char* k_type_field_name(long long type_id, long long i);
KValue k_render(KValue v, long long quote);
static KValue k_render_at(KValue v, long long quote, int held);

static KValue k_render_whole(KValue v);

/* What a runtime diagnostic shows of the value it names. A plain render
   hands an err back unrendered so that it rides through the program, and a
   diagnostic that printed that as text read the err as a string and crashed;
   the parent then reported the crash as running out of stack. The err is
   unwrapped and rendered here rather than through k_render_at, which has one
   caller outside itself and is inlined into k_render on the print path; six
   more callers stopped that, and every print paid for a call. */
static __attribute__((cold, noinline)) KValue k_shown(KValue v, long long quote) {
    if (v.tag == K_THUNK) v = k_force(v);
    if (v.tag == K_SUB) v = k_sub_base(v);
    if (v.tag == K_ERR) return k_render_whole(v);
    return k_render(v, quote);
}

/* keyed reads: `{ author: writer title } = post` — fields resolve by name
   against the record's declared type */
KValue k_keyed_check(KValue v, long long entries) {
    if (v.tag != K_REC) {
        KValue r = k_shown(v, 1);
        fprintf(stderr, "%serror[runtime]:%s cannot read fields of %s; keyed reads take a record\n",
                k_c_err(), k_c_off(), k_cstr(k_as_str(r)));
        k_died();
    }
    if (entries >= k_as_rec(v)->nfields)
        k_die("a keyed read omits at least one field; reading every field is the positional form");
    return v;
}

/* The POSITIONAL destructuring bind — `point a b = v` — where `v` is not a
   `point`. Its keyed sibling above has always rendered the value; this form
   said `cannot destructure value as ...` and named neither the value nor what
   to do instead, so the two engines printed different sentences for the same
   program until 2026-08-27.

   The interpreter's wording is the one kept, because both halves native was
   missing are worth having: `7` is a value the reader's own program produced,
   unlike the socket handles in kanso#1094 that kanso hands out, and the clause
   after the semicolon is the only place the language says why a bind cannot
   simply fail over to another arm. */
__attribute__((noreturn, noinline)) void k_die_destructure(KValue v, const char* ty) {
    /* QUOTED, matching `render(self, &value, true)` at eval.rs:1283. Unquoted
       agrees for an int, a float and a list and diverges on a string — which a
       fixture holding one int would never have shown. */
    KValue shown = k_shown(v, 1);
    fprintf(stderr,
            "%serror[runtime]:%s cannot destructure %s as `%s`; bindings are irrefutable, "
            "so handle other types by dispatch first\n",
            k_c_err(), k_c_off(), k_cstr(k_as_str(shown)), ty);
    k_died();
}

/* `.` field access: failures ride through; a non-record dies loudly. */
// A getter group that matched nothing. The reader wrote `x.name`, so the
// failure is a field error in their words, not a dispatch error in ours.
void k_no_field(KValue v, const char* name) {
    if (v.tag != K_REC) {
        KValue shown = k_shown(v, 1);
        fprintf(stderr, "%serror[runtime]:%s `.` reads a field of a record, not %s\n",
                k_c_err(), k_c_off(), k_cstr(k_as_str(shown)));
        k_died();
    }
    KRec* r = k_as_rec(v);
    fprintf(stderr, "%serror[runtime]:%s `%s` has no field `%s`\n", k_c_err(), k_c_off(),
            k_type_name(r->type_id), name);
    k_died();
}

__attribute__((noreturn, noinline)) static void k_die_got(const char* msg, KValue v) {
    KValue shown = k_shown(v, 0);
    fprintf(stderr, "%serror[runtime]:%s %s, got %s\n", k_c_err(), k_c_off(), msg,
            k_cstr(k_as_str(shown)));
    k_died();
}

__attribute__((noreturn, noinline)) static void k_die_value(const char* msg, KValue v) {
    KValue shown = k_shown(v, 1);
    fprintf(stderr, "%serror[runtime]:%s %s, not %s\n", k_c_err(), k_c_off(), msg,
            k_cstr(k_as_str(shown)));
    k_died();
}

KValue k_b_field(KValue v, const char* name) {
    if (!k_not_failure(v)) return v;
    /* a getter is a constructor pattern, so it reads through a subtype for
       the same reason one matches */
    if (v.tag == K_SUB) v = k_sub_base(v);
    if (v.tag != K_REC) {
        KValue shown = k_shown(v, 1);
        fprintf(stderr, "%serror[runtime]:%s `.` reads a field of a record, not %s\n",
                k_c_err(), k_c_off(), k_cstr(k_as_str(shown)));
        k_died();
    }
    KRec* r = k_as_rec(v);
    for (long long i = 0; i < r->nfields; i++) {
        if (!strcmp(k_type_field_name(r->type_id, i), name)) return r->fields[i];
    }
    fprintf(stderr, "%serror[runtime]:%s `%s` has no field `%s`\n", k_c_err(), k_c_off(),
            k_type_name(r->type_id), name);
    k_died();
    KValue none; none.tag = K_NONE; none.payload = 0; return none;
}


/* A getter group matched nothing and the value is a cell. Reading a field
   DEMANDS the record, so this forces and reads rather than refusing — and
   unlike rendering it cannot recur, because the force answers a record and the
   read hands back whatever the field holds, forced or not. Native defers where
   the oracle does not, and without this the two disagree on the ordinary
   conditional accumulator.

   It can only turn an error into a value or into a different error: the path
   is reached when nothing matched, so today it exits. A forced value that a
   user-defined arm would have matched is read structurally instead, which is
   still better than stopping. */
KValue k_field_forced(KValue v, const char* name) {
    return k_b_field(k_force(v), name);
}

KValue k_keyed_field(KValue v, const char* name) {
    KRec* r = k_as_rec(v);
    long long n = k_type_field_count(r->type_id);
    for (long long i = 0; i < n; i++)
        if (!strcmp(k_type_field_name(r->type_id, i), name)) return r->fields[i];
    fprintf(stderr, "%serror[runtime]:%s `%s` has no field `%s`\n", k_c_err(), k_c_off(),
            k_type_name(r->type_id), name);
    k_died();
}

