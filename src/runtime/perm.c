/* A permanent object: malloc'd, so it lives outside the beat arena and
   survives every rewind. Interned single-char strings and zero-field marker
   records are cached and reused across beats — an arena rewind moves the
   bump pointer, so caching arena storage and reusing it after a pop hands
   back reclaimed, since-reused memory. Permanent storage is the only cache
   that is sound across beats. */
static __attribute__((noinline, preserve_most)) void* k_alloc_perm(size_t n) {
    if (K_COUNTING) k_stat_perm_allocs++;
    void* p = malloc(n);
    if (!p) { fputs("out of memory\n", stderr); exit(1); }
    return p;
}

/* Permanent storage the copy walk knows is permanent. A literal, an interned
   character and a marker record are built once and handed out for the rest of
   the run, but malloc'd storage lies in no arena block, and the walk takes an
   unregistered pointer for one it has to rescue. Copying a constant into a
   carry is only waste; the harm is the repair, which leaves a surviving node
   where it is and copies what it points at -- so a map built before a loop,
   whose keys were literals, came back from the loop's first stage with its
   keys moved into the loop's region, and the next rewind of that region freed
   them under a map that was still live. Constants come from chunks registered
   with the frozen ranges instead, which the walk already treats as immortal.
   Bounded by the program text and the caches: a bignum moved to permanent
   storage is the run's own data, so it stays on k_alloc_perm and out of the
   ranges every outside pointer is checked against. */
/* Chunks start at 4 KiB and double up to 64 KiB. A program with few
   constants then holds one small chunk, which in the browser is the
   difference between staying inside the pages it already had and taking one
   more. */
#define K_CONST_FIRST (4 * 1024)
static char* k_const_at = NULL;
static size_t k_const_left = 0;

static void k_const_chunk(size_t size) {
    char* chunk = malloc(size);
    if (!chunk) { fputs("out of memory\n", stderr); exit(1); }
    k_frozen_note(chunk, size);
    if (k_const_n == k_const_cap) {
        k_const_cap = k_const_cap ? 2 * k_const_cap : 8;
        k_const_lo = realloc(k_const_lo, sizeof(const char*) * (size_t)k_const_cap);
        k_const_hi = realloc(k_const_hi, sizeof(const char*) * (size_t)k_const_cap);
        if (!k_const_lo || !k_const_hi) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_const_lo[k_const_n] = chunk;
    k_const_hi[k_const_n] = chunk + size;
    k_const_n++;
    k_const_at = chunk;
    k_const_left = size;
}
static __attribute__((noinline, preserve_most)) void* k_alloc_const(size_t n) {
    if (K_COUNTING) k_stat_perm_allocs++;
    n = (n + 15) & ~(size_t)15;
    if (n > k_const_left) {
        size_t step = K_CONST_FIRST << (k_const_n < 4 ? k_const_n : 4);
        k_const_chunk(n > step ? n : step);
    }
    void* p = k_const_at;
    k_const_at += n;
    k_const_left -= n;
    return p;
}
/* Freeze a constant so it is built once instead of per call. A zero mark
   makes k_survives answer no for everything, which is what forces a full
   copy out of the arena rather than the sharing the carry path wants;
   permanent storage is then the only cache a rewind cannot invalidate. The
   copy lands at exact capacity, which is also what keeps k_b_push_mut from
   mutating a shared constant in place — it writes through only when a list
   has room to spare. */
KValue k_caf_freeze(KValue v) {
    if (!k_is_heap(v.tag)) return v;
    KMark none = { NULL, NULL, 0 };
    k_ptrmap_begin(&k_copy_seen);
    k_copy_seen_live = 0;
    size_t need = k_copy_size(v, &none);
    KCarryBuf* buf = k_alloc_perm(sizeof(KCarryBuf));
    buf->data = malloc(need ? need : 16);
    if (!buf->data) { fputs("out of memory\n", stderr); exit(1); }
    buf->cap = need ? need : 16;
    buf->used = 0;
    k_frozen_note(buf->data, buf->cap);
    KCopy cp = { buf, &none, 0, 0 };
    k_ptrmap_begin(&k_copy_map);
    k_copy_map_live = 0;
    return k_deep_copy(v, &cp);
}

/* A constant whose builder answered the very cell it was seeded with has
   computed nothing: freezing it would tie the knot to itself and every later
   demand would chase the loop rather than find a value. The blackhole is the
   answer, and a name sitting in a constructor's slot never reaches here —
   that one answers the record it helped build. */
KValue k_caf_complete(KValue built, KValue seeded) {
    if (built.tag == K_THUNK && seeded.tag == K_THUNK && built.payload == seeded.payload) {
        { k_site = 0; k_site_id = 0; k_die("a lazy binding demands its own value"); }
    }
    KValue frozen = k_caf_freeze(built);
    /* A field that stored this cell mid-flight holds the SEED, and the frozen
       value is a copy of what the builder made — so storing the copy in the
       global is not enough, and the field would chase a blackhole nobody ever
       answers. Answering the seed itself is what ties the knot: every holder
       of the cell, copied or not, resolves through the same thunk. */
    if (seeded.tag == K_THUNK) {
        KThunk* t = (KThunk*)seeded.payload;
        t->result = frozen;
        t->forced = K_MEMO_KEPT;
    }
    return frozen;
}


/* Every tag whose payload is a pointer. `K_SUB` was absent until 2026-08-30,
   and `k_cohort_pop` reads this to decide whether a beat's result has to be
   carried out of the arena before the rewind: a returned subtype was taken for
   a scalar, the arena went back to the mark under it, and the caller got a
   dangling `KSub*`. The value read back as whatever the arena was reused for,
   which on the fixtures below is a pointer rendered as a double. `K_THUNK` is
   spelled out at that call site rather than here because a thunk is a promise
   and not a value; every other pointer tag belongs in this list. */
/* Every tag whose payload is a pointer — K_ERR K_STR K_REC K_DESC K_LIST
   K_MAP K_CLOSURE K_BYTES K_SUB — as bit positions in the enum above. `tag`
   is masked to the enum's width so the shift is defined for any input; every
   tag a value can carry is already in range.

   `K_SUB` was absent until 2026-08-30 and its payload is a `KSub*`, so a
   returned subtype was taken for a scalar by `k_cohort_pop` and by
   `k_slots_survive` alike: the arena went back to the mark under it and the
   caller kept a dangling pointer.

   A mask rather than the switch this used to be, because the switch is inlined
   into `k_slots_survive` and through it into `k_copy_size`, which is 36% of
   deepbench. A tenth `case` cost that benchmark 49,528,233 instructions —
   6.14%, all of it inside `k_copy_size`, on both hosts to the instruction —
   on a program that never makes a subtype: `k_survives_x` and `k_ptrmap_at`
   are byte-identical across the change, so it is the same walk compiled
   differently. The mask gives 6,149,160 of that back. Two other shapes were
   measured and were worse: a mask carrying a bounds branch reads 878,869,219,
   and giving `k_slots_survive` its own copy of the switch reads exactly what
   the shared switch does. */
static int k_is_heap(long long tag) {
    /* bit 17 is K_BIG, whose limbs sit in the arena like bytes do, and bit 18
       K_ERRV, an err out of its box */
    return (int)((0x6AFE0U >> (tag & 31)) & 1U);
}

/* stdout is written through stdio's buffer and stderr is not. Text a program
   printed reached a shared pipe after a later error, and a program killed
   while it waited lost all of it, where the interpreter had already written
   both. So the buffer is emptied before anything goes to stderr, before the
   program waits on input, a connection or the clock, and before it starts a
   child that shares the stream. */
static void k_out_settle(void) { fflush(stdout); }

/* Diagnostics color from the site palette, only when stderr is a tty and
   NO_COLOR is unset: vermillion (24-bit 0xf03a00; 256-color 202) for the
   error kind, dim for trace lines. Piped output stays plain. */
static int k_color_mode(void) {
    static int mode = -1;
    if (mode >= 0) return mode;
    if (!isatty(2) || getenv("NO_COLOR")) return mode = 0;
    const char* ct = getenv("COLORTERM");
    if (ct && (strstr(ct, "truecolor") || strstr(ct, "24bit"))) return mode = 2;
    return mode = 1;
}

static const char* k_c_err(void) {
    /* every runtime diagnostic opens with this color, so it is where the
       program's own output is put ahead of the error */
    k_out_settle();
    switch (k_color_mode()) {
        case 2: return "\x1b[38;2;240;58;0m";
        case 1: return "\x1b[38;5;202m";
        default: return "";
    }
}

static const char* k_c_dim(void) { return k_color_mode() ? "\x1b[2m" : ""; }
static const char* k_c_off(void) { return k_color_mode() ? "\x1b[0m" : ""; }

/* The interpreter is the oracle and it names what it was handed. A reader
   told only what was wanted has to guess, and the two engines have to agree
   anyway, so a type complaint carries the value that caused it. */
__attribute__((noreturn, noinline)) static void k_die_value(const char* msg, KValue v);
__attribute__((noreturn, noinline)) static void k_die_got(const char* msg, KValue v);

static long long k_itoa(char* buf, long long v);

/* The site compiled code stamped before its last call that can stop the
   program: the "<hako>\0<fn> at <file>:<line>" literal an err's origin uses,
   so the place prints the way `born in` does (ruled on #1975). A death prints
   the second half under its message; NULL is a death nothing stamped. */
const char* k_site;
int k_site_id;
const char* k_deep_site;
/* The program line that last called into the library and has not returned:
   "<file>:<line>", set and put back around each such call (ruled on #2099). */
const char* k_from;

/* Whether "<fn> at <file>:<line>" names a library's file: the standard
   library's `std/` or a hako fetched under `.hako/`, as
   provenance::is_library_file says. */
static int k_library_place(const char* spelled) {
    const char* at = NULL;
    for (const char* p = strstr(spelled, " at "); p; p = strstr(p + 1, " at ")) at = p;
    if (!at) return 0;
    const char* file = at + 4;
    return strncmp(file, "std/", 4) == 0 || strstr(file, ".hako/") != NULL;
}

__attribute__((noreturn, noinline)) void k_died(void) {
    const char* site = k_site_now();
    if (site) {
        const char* spelled = site + strlen(site) + 1;
        fprintf(stderr, "%s  failed in %s%s\n", k_c_dim(), spelled, k_c_off());
        if (k_from && k_library_place(spelled))
            fprintf(stderr, "%s  called from %s%s\n", k_c_dim(), k_from, k_c_off());
    }
    exit(1);
}

__attribute__((noreturn, noinline)) void k_die_arity(long long want, long long got) {
    char w[24], g[24];
    k_itoa(w, want);
    k_itoa(g, got);
    fprintf(stderr, "%serror[runtime]:%s this function takes %s argument(s), got %s\n",
            k_c_err(), k_c_off(), w, g);
    k_died();
}

__attribute__((noreturn, noinline)) void k_die_overload(const char* name) {
    fprintf(stderr, "%serror[runtime]:%s no overload of `%s` matches these arguments\n",
            k_c_err(), k_c_off(), name);
    k_died();
}

__attribute__((noreturn, noinline)) void k_die(const char* msg) {
    fprintf(stderr, "%serror[runtime]:%s %s\n", k_c_err(), k_c_off(), msg);
    k_died();
}

/* A function value called with a count it does not take. A group names the
   arms that did not match; a builtin takes exactly one count and says both,
   which is the sentence the interpreter prints. */
__attribute__((noreturn, noinline)) static void k_die_ref_arity(KFnref* r, long long got) {
    if (!r->builtin) k_die_overload(r->name);
    char w[24], g[24], said[128];
    k_itoa(w, r->arity);
    k_itoa(g, got);
    snprintf(said, sizeof said, "`%s` takes %s argument(s), got %s", r->name, w, g);
    k_die(said);
}

/* Hand-rolled lld formatting: the vfprintf machinery showed up hot in
   the encode profile, and a digit loop beats it several times over. */
/* Two digits a time, and the length first — shared by `k_itoa` below and by
   `render_ryu`'s digit extraction further down. Each used to do one 64-bit
   division per digit into a scratch buffer and then reverse it; a pair table
   halves the divisions and writes straight into place, which needs the length
   before the first digit. The ladder climbs from one digit rather than down
   from twenty because the values that reach it are small: a json document's
   floats are short, and so are the integers beside them.

   Nineteen digits is a long long's most, and twenty a uint64_t's, so every
   rung is reachable from `k_itoa` even though `ryu_d2d` never passes more than
   seventeen. The extraction walks DOWN from the length it is given, and a
   length one short would write at a negative index rather than truncate. */
static const char RYU_DIGITS[201] =
    "0001020304050607080910111213141516171819"
    "2021222324252627282930313233343536373839"
    "4041424344454647484950515253545556575859"
    "6061626364656667686970717273747576777879"
    "8081828384858687888990919293949596979899";

static inline int ryu_declen(uint64_t v) {
    if (v < 10ULL) return 1;
    if (v < 100ULL) return 2;
    if (v < 1000ULL) return 3;
    if (v < 10000ULL) return 4;
    if (v < 100000ULL) return 5;
    if (v < 1000000ULL) return 6;
    if (v < 10000000ULL) return 7;
    if (v < 100000000ULL) return 8;
    if (v < 1000000000ULL) return 9;
    if (v < 10000000000ULL) return 10;
    if (v < 100000000000ULL) return 11;
    if (v < 1000000000000ULL) return 12;
    if (v < 10000000000000ULL) return 13;
    if (v < 100000000000000ULL) return 14;
    if (v < 1000000000000000ULL) return 15;
    if (v < 10000000000000000ULL) return 16;
    if (v < 100000000000000000ULL) return 17;
    if (v < 1000000000000000000ULL) return 18;
    if (v < 10000000000000000000ULL) return 19;
    return 20;
}

/* The magnitude's digits, the last of them just before `end`: a caller that
   has already counted them, to size what it writes into, does not count them
   again. */
static inline __attribute__((always_inline)) void k_digits_into(char* end, uint64_t u) {
    char* o = end;
    while (u >= 100) {
        uint32_t c = (uint32_t)(u % 100);
        u /= 100;
        o -= 2;
        o[0] = RYU_DIGITS[c * 2];
        o[1] = RYU_DIGITS[c * 2 + 1];
    }
    if (u >= 10) {
        o -= 2;
        o[0] = RYU_DIGITS[u * 2];
        o[1] = RYU_DIGITS[u * 2 + 1];
    } else {
        *--o = (char)('0' + (uint32_t)u);
    }
}

/* Returns the number of digits written, not counting the terminator, so a
   caller that turns the buffer into a string does not walk it again to find
   out how long it is. `k_render_at` did exactly that on every number it
   rendered: k_str is k_str_n behind a strlen, and the length was already here.

   Digits come out in pairs, straight into place. This used to fill a scratch
   buffer least-significant digit first, one division apiece, and then copy it
   back reversed — two passes over the digits and twice the divisions. */
static long long k_itoa(char* buf, long long v) {
    char* w = buf;
    if (v < 0) *w++ = '-';
    uint64_t u = v < 0 ? (uint64_t)(-(v + 1)) + 1 : (uint64_t)v;
    char* end = w + ryu_declen(u);
    k_digits_into(end, u);
    *end = 0;
    return (long long)(end - buf);
}

static long long k_ptr(void* p) { return (long long)(intptr_t)p; }
static KStr* k_as_str(KValue v) { return (KStr*)(intptr_t)v.payload; }
static KRec* k_as_rec(KValue v) { return (KRec*)(intptr_t)v.payload; }
static KList* k_as_list(KValue v) { return (KList*)(intptr_t)v.payload; }
static KBytes* k_as_bytes(KValue v) { return (KBytes*)(intptr_t)v.payload; }
static KMap* k_as_map(KValue v) { return (KMap*)(intptr_t)v.payload; }
static KDesc* k_as_desc(KValue v) { return (KDesc*)(intptr_t)v.payload; }

static double k_as_f(KValue v) { double d; memcpy(&d, &v.payload, 8); return d; }

KValue k_float(double d) {
    KValue v; v.tag = K_FLOAT; memcpy(&v.payload, &d, 8); return v;
}

/* glibc's memcpy spends a dozen instructions deciding how to move bytes
   before it moves any, which is most of the cost when the run is shorter
   than a couple of words. Two overlapping loads and two overlapping stores
   touch only bytes inside [s, s+n) and [d, d+n), so no caller needs slack
   at either end for this to be safe. */
/* A copy long enough to be libc's, out of line and preserve_most so a caller
   whose short copies are inline words keeps no frame for the long case. */
static __attribute__((noinline, cold, preserve_most)) void k_copy_cold(void* d, const void* s, size_t n) {
    memcpy(d, s, n);
}

static inline __attribute__((always_inline)) void k_copy_short(char* d, const char* s, long long n) {
    if (n >= 16) { k_copy_cold(d, s, (size_t)n); return; }
    if (n >= 8) {
        uint64_t a, b;
        __builtin_memcpy(&a, s, 8);
        __builtin_memcpy(&b, s + n - 8, 8);
        __builtin_memcpy(d, &a, 8);
        __builtin_memcpy(d + n - 8, &b, 8);
    } else if (n >= 4) {
        uint32_t a, b;
        __builtin_memcpy(&a, s, 4);
        __builtin_memcpy(&b, s + n - 4, 4);
        __builtin_memcpy(d, &a, 4);
        __builtin_memcpy(d + n - 4, &b, 4);
    } else if (n >= 2) {
        uint16_t a, b;
        __builtin_memcpy(&a, s, 2);
        __builtin_memcpy(&b, s + n - 2, 2);
        __builtin_memcpy(d, &a, 2);
        __builtin_memcpy(d + n - 2, &b, 2);
    } else if (n == 1) {
        d[0] = s[0];
    }
}

KValue k_int(long long i) { KValue v; v.tag = K_INT; v.payload = i; return v; }
KValue k_bool(long long b) { KValue v; v.tag = b ? K_TRUE : K_FALSE; v.payload = 0; return v; }
KValue k_none(void) { KValue v; v.tag = K_NONE; v.payload = 0; return v; }

