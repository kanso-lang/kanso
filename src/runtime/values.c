/* ---- integers past a machine word ---------------------------------------
   An int that outgrows a word is a bignum: tag K_BIG, payload a KBytes whose
   bytes are a sign word (0 or 1, eight bytes) and then the magnitude in 64-bit
   limbs, least significant first, the top limb nonzero. The form is
   canonical, as the interpreter's `Int` keeps it: a K_BIG never holds a number
   a K_INT can, so two equal ints always share a tag. The bytes sit in the
   arena behind their header, which is the layout the copy and rewind paths
   already carry for bytes, and a bignum takes those paths beside K_BYTES.
   Those paths copy bytes wherever their allocator puts them, so a limb is read
   and written through a type that promises no alignment. Every operation here
   accepts either kind and answers the canonical one.

   The limbs are as wide as num-bigint's, which the interpreter uses: a limb
   product is one 64 x 64 -> 128 multiply. wasm32 has no such instruction and
   compiler-rt's `__multi3` stands in, which wasm/shim.c supplies; there is no
   128-bit division anywhere here, since that helper is one wasm32's
   compiler-rt lacks, and a quotient digit comes from `k_div_2by1`. */

typedef uint64_t KLimb;
typedef uint64_t KLimbU __attribute__((aligned(1)));
typedef unsigned __int128 KWide;

typedef struct { int neg; long long n; const KLimbU* d; KLimb tmp[1]; } KBigView;

/* The sign and magnitude of an int of either kind. A small int's limb is
   written into the view itself, so a view is used where it was filled. */
static void k_big_view(KValue v, KBigView* o) {
    if (v.tag == K_BIG) {
        KBytes* b = (KBytes*)(intptr_t)v.payload;
        uint64_t sign;
        memcpy(&sign, b->data, 8);
        o->neg = (int)sign;
        o->n = (b->len - 8) / 8;
        o->d = (const KLimbU*)(b->data + 8);
        return;
    }
    long long x = v.payload;
    uint64_t m = x < 0 ? (uint64_t)0 - (uint64_t)x : (uint64_t)x;
    o->neg = x < 0;
    o->tmp[0] = m;
    o->n = m != 0;
    o->d = o->tmp;
}

/* Room for a result of up to `n` limbs, in the arena where it will stay. */
static KBytes* k_big_room(long long n) {
    KBytes* b = k_alloc(sizeof(KBytes) + 8 + 8 * (size_t)(n > 0 ? n : 1));
    b->data = (const unsigned char*)(b + 1);
    b->cap = 0;
    return b;
}

static inline KLimbU* k_big_limbs(KBytes* b) { return (KLimbU*)((unsigned char*)(b + 1) + 8); }

/* Finish a result written into `b`'s limbs: strip leading zero limbs, and
   answer a small int when the number fits one, the arena bytes then going
   unused. */
static KValue k_big_finish(KBytes* b, int neg, long long n) {
    KLimbU* d = k_big_limbs(b);
    while (n > 0 && d[n - 1] == 0) n--;
    if (n <= 1) {
        uint64_t m = n == 0 ? 0 : d[0];
        if (!neg && m <= (uint64_t)INT64_MAX) return k_int((long long)m);
        if (neg && m <= (uint64_t)1 << 63) return k_int((long long)((uint64_t)0 - m));
    }
    uint64_t sign = neg ? 1 : 0;
    memcpy((unsigned char*)(b + 1), &sign, 8);
    b->len = 8 + 8 * n;
    KValue v; v.tag = K_BIG; v.payload = k_ptr(b); return v;
}

static int k_mag_cmp(const KLimbU* a, long long an, const KLimbU* b, long long bn) {
    if (an != bn) return an < bn ? -1 : 1;
    for (long long i = an - 1; i >= 0; i--)
        if (a[i] != b[i]) return a[i] < b[i] ? -1 : 1;
    return 0;
}

/* r = a + b; r has room for max(an, bn) + 1 limbs. */
static long long k_mag_add(KLimbU* r, const KLimbU* a, long long an, const KLimbU* b, long long bn) {
    if (an < bn) { const KLimbU* t = a; a = b; b = t; long long tn = an; an = bn; bn = tn; }
    KLimb carry = 0;
    long long i = 0;
    for (; i < bn; i++) { KWide s = (KWide)a[i] + b[i] + carry; r[i] = (KLimb)s; carry = (KLimb)(s >> 64); }
    for (; i < an; i++) { KLimb s = a[i] + carry; carry = s < carry; r[i] = s; }
    r[i] = carry;
    return an + 1;
}

/* r = a - b where a >= b; r has room for an limbs. */
static long long k_mag_sub(KLimbU* r, const KLimbU* a, long long an, const KLimbU* b, long long bn) {
    KLimb borrow = 0;
    long long i = 0;
    for (; i < bn; i++) {
        KLimb x = a[i], y = b[i];
        KLimb t = x - y;
        r[i] = t - borrow;
        borrow = (x < y) | (t < borrow);
    }
    for (; i < an; i++) {
        KLimb x = a[i];
        r[i] = x - borrow;
        borrow = x < borrow;
    }
    return an;
}

/* The signed sum of two ints given as views, `bneg` standing in for b's sign
   so subtraction is the sum with it flipped. */
static KValue k_big_addsub(const KBigView* a, const KBigView* b, int bneg) {
    long long n = (a->n > b->n ? a->n : b->n) + 1;
    KBytes* r = k_big_room(n);
    KLimbU* d = k_big_limbs(r);
    if (a->neg == bneg) {
        long long rn = k_mag_add(d, a->d, a->n, b->d, b->n);
        return k_big_finish(r, a->neg, rn);
    }
    int c = k_mag_cmp(a->d, a->n, b->d, b->n);
    if (c == 0) return k_int(0);
    if (c > 0) return k_big_finish(r, a->neg, k_mag_sub(d, a->d, a->n, b->d, b->n));
    return k_big_finish(r, bneg, k_mag_sub(d, b->d, b->n, a->d, a->n));
}

KValue k_int_add(KValue x, KValue y) {
    KBigView a, b;
    k_big_view(x, &a);
    k_big_view(y, &b);
    return k_big_addsub(&a, &b, b.neg);
}

KValue k_int_sub(KValue x, KValue y) {
    KBigView a, b;
    k_big_view(x, &a);
    k_big_view(y, &b);
    return k_big_addsub(&a, &b, b.n == 0 ? 0 : !b.neg);
}

/* One schoolbook row: d[0..ln] = l * m, or d[0..ln] += l * m when `add`. A
   limb's product, the limb already in the row and the carry fit in 128 bits:
   (2^64 - 1)^2 + 2(2^64 - 1) is 2^128 - 1. */
static inline void k_mag_row(KLimbU* d, const KLimbU* l, long long ln, KLimb m, int add) {
    KLimb c = 0;
    for (long long j = 0; j < ln; j++) {
        KWide t = (KWide)m * l[j] + (add ? d[j] : 0) + c;
        d[j] = (KLimb)t;
        c = (KLimb)(t >> 64);
    }
    d[ln] = c;
}

/* Schoolbook, one row per limb of the SHORTER operand. A number that grows by
   doubling meets `acc * 2` once a step, and with the long operand outside each
   of its limbs paid a whole row's setup for a row one limb long. The first row
   writes its products where the others add theirs, so the result is not
   cleared first; a row reads only limbs the row before it wrote. The views are
   swapped by pointer, since a small int's limb lives inside its own view. */
KValue k_int_mul(KValue x, KValue y) {
    KBigView a, b;
    k_big_view(x, &a);
    k_big_view(y, &b);
    if (a.n == 0 || b.n == 0) return k_int(0);
    const KBigView* l = &a;
    const KBigView* s = &b;
    if (a.n < b.n) { l = &b; s = &a; }
    long long ln = l->n, sn = s->n;
    long long n = ln + sn;
    KBytes* r = k_big_room(n);
    KLimbU* d = k_big_limbs(r);
    k_mag_row(d, l->d, ln, s->d[0], 0);
    for (long long i = 1; i < sn; i++) k_mag_row(d + i, l->d, ln, s->d[i], 1);
    return k_big_finish(r, a.neg != b.neg, n);
}

/* The quotient of the two-limb number hi:lo by d, and its remainder, where
   hi < d so the quotient fits a limb. x86-64 divides 128 bits by 64 in one
   instruction under exactly that condition. Elsewhere it is Hacker's
   Delight's divlu, two half-width digits each estimated from the divisor's
   top half and corrected at most twice, in 64-bit arithmetic only. */
static inline KLimb k_div_2by1(KLimb hi, KLimb lo, KLimb d, KLimb* rem) {
#if defined(__x86_64__)
    KLimb q, r;
    __asm__("divq %4" : "=a"(q), "=d"(r) : "a"(lo), "d"(hi), "rm"(d));
    *rem = r;
    return q;
#else
    const uint64_t b = (uint64_t)1 << 32;
    int s = __builtin_clzll(d);
    d <<= s;
    uint64_t vn1 = d >> 32, vn0 = d & 0xFFFFFFFFu;
    uint64_t un32 = s ? (hi << s) | (lo >> (64 - s)) : hi;
    uint64_t un10 = lo << s;
    uint64_t un1 = un10 >> 32, un0 = un10 & 0xFFFFFFFFu;
    uint64_t q1 = un32 / vn1, rhat = un32 - q1 * vn1;
    while (q1 >= b || q1 * vn0 > b * rhat + un1) {
        q1--;
        rhat += vn1;
        if (rhat >= b) break;
    }
    uint64_t un21 = un32 * b + un1 - q1 * d;
    uint64_t q0 = un21 / vn1;
    rhat = un21 - q0 * vn1;
    while (q0 >= b || q0 * vn0 > b * rhat + un0) {
        q0--;
        rhat += vn1;
        if (rhat >= b) break;
    }
    *rem = (un21 * b + un0 - q0 * d) >> s;
    return q1 * b + q0;
#endif
}

/* Magnitude division, u / v, for u at least as long as v and v nonzero:
   the quotient into q (m - n + 1 limbs) and the remainder into r (n limbs).
   Knuth's algorithm D with 64-bit digits, as Hacker's Delight writes it, the
   multiply-and-subtract done in unsigned arithmetic with the borrow carried
   by hand. */
static void k_mag_divmod(KLimbU* q, KLimbU* r, const KLimbU* u, long long m, const KLimbU* v, long long n) {
    if (n == 1) {
        KLimb rem = 0;
        for (long long i = m - 1; i >= 0; i--) q[i] = k_div_2by1(rem, u[i], v[0], &rem);
        r[0] = rem;
        return;
    }
    int s = __builtin_clzll(v[n - 1]);
    KLimb* vn = malloc(8 * (size_t)n);
    KLimb* un = malloc(8 * (size_t)(m + 1));
    for (long long i = n - 1; i > 0; i--)
        vn[i] = (v[i] << s) | (s ? v[i - 1] >> (64 - s) : 0);
    vn[0] = v[0] << s;
    un[m] = s ? u[m - 1] >> (64 - s) : 0;
    for (long long i = m - 1; i > 0; i--)
        un[i] = (u[i] << s) | (s ? u[i - 1] >> (64 - s) : 0);
    un[0] = u[0] << s;
    KLimb vtop = vn[n - 1], vnext = vn[n - 2];
    for (long long j = m - n; j >= 0; j--) {
        KLimb qhat, rhat;
        int rbig = 0;
        if (un[j + n] >= vtop) {
            /* the top digits are equal, and the estimate is the largest digit */
            qhat = ~(KLimb)0;
            rhat = un[j + n - 1] + vtop;
            rbig = rhat < vtop;
        } else {
            qhat = k_div_2by1(un[j + n], un[j + n - 1], vtop, &rhat);
        }
        while (!rbig && (KWide)qhat * vnext > (((KWide)rhat << 64) | un[j + n - 2])) {
            qhat--;
            KLimb was = rhat;
            rhat += vtop;
            rbig = rhat < was;
        }
        KLimb carry = 0, borrow = 0;
        for (long long i = 0; i < n; i++) {
            KWide p = (KWide)qhat * vn[i] + carry;
            carry = (KLimb)(p >> 64);
            KLimb lo = (KLimb)p, x = un[i + j];
            KLimb t = x - lo;
            un[i + j] = t - borrow;
            borrow = (x < lo) | (t < borrow);
        }
        KLimb x = un[j + n];
        KLimb t = x - carry;
        un[j + n] = t - borrow;
        int under = (x < carry) | (t < borrow);
        q[j] = qhat;
        if (under) {
            q[j]--;
            KLimb c = 0;
            for (long long i = 0; i < n; i++) {
                KWide w = (KWide)un[i + j] + vn[i] + c;
                un[i + j] = (KLimb)w;
                c = (KLimb)(w >> 64);
            }
            un[j + n] += c;
        }
    }
    for (long long i = 0; i < n - 1; i++)
        r[i] = (un[i] >> s) | (s ? un[i + 1] << (64 - s) : 0);
    r[n - 1] = un[n - 1] >> s;
    free(vn);
    free(un);
}

/* Truncating division and its remainder, the remainder taking the dividend's
   sign: what `BigInt`'s `/` and `%` answer, so what the interpreter does. The
   divisor is not zero. */
static KValue k_big_divrem(KValue x, KValue y, int want_rem) {
    KBigView a, b;
    k_big_view(x, &a);
    k_big_view(y, &b);
    if (k_mag_cmp(a.d, a.n, b.d, b.n) < 0) return want_rem ? x : k_int(0);
    long long qn = a.n - b.n + 1;
    KBytes* qb = k_big_room(qn);
    KBytes* rb = k_big_room(b.n);
    k_mag_divmod(k_big_limbs(qb), k_big_limbs(rb), a.d, a.n, b.d, b.n);
    if (want_rem) return k_big_finish(rb, a.neg, b.n);
    return k_big_finish(qb, a.neg != b.neg, qn);
}

KValue k_int_div(KValue x, KValue y) { return k_big_divrem(x, y, 0); }
KValue k_int_rem(KValue x, KValue y) { return k_big_divrem(x, y, 1); }

KValue k_int_neg(KValue x) { return k_int_sub(k_int(0), x); }

/* Ordering of two ints of either kind. */
int k_int_cmp(KValue x, KValue y) {
    if (x.tag == K_INT && y.tag == K_INT) return (x.payload > y.payload) - (x.payload < y.payload);
    KBigView a, b;
    k_big_view(x, &a);
    k_big_view(y, &b);
    if (a.neg != b.neg) return a.neg ? -1 : 1;
    int c = k_mag_cmp(a.d, a.n, b.d, b.n);
    return a.neg ? -c : c;
}

/* The nearest double, ties to even, as `BigInt::to_f64` answers: the top 64
   bits with every bit below them folded into the lowest, converted by the
   hardware, then scaled. A number past the largest double is an infinity. */
double k_int_to_f(KValue x) {
    if (x.tag == K_INT) return (double)x.payload;
    KBigView a;
    k_big_view(x, &a);
    long long bits = a.n * 64 - __builtin_clzll(a.d[a.n - 1]);
    long long shift = bits - 64;
    uint64_t top = 0;
    for (int k = 0; k < 64; k++) {
        long long bit = shift + k;
        if (a.d[bit / 64] >> (bit % 64) & 1) top |= (uint64_t)1 << k;
    }
    int sticky = 0;
    for (long long bit = 0; bit < shift && !sticky; bit++)
        if (a.d[bit / 64] >> (bit % 64) & 1) sticky = 1;
    double d = ldexp((double)(top | (uint64_t)sticky), (int)(shift > 2000 ? 2000 : shift));
    return a.neg ? -d : d;
}

/* The int a whole finite double holds, exactly. */
KValue k_int_of_whole_f(double d) {
    if (d >= -9223372036854775808.0 && d < 9223372036854775808.0) return k_int((long long)d);
    uint64_t bits;
    memcpy(&bits, &d, 8);
    int neg = (int)(bits >> 63);
    int exp = (int)((bits >> 52) & 0x7FF);
    uint64_t mant = (bits & (((uint64_t)1 << 52) - 1)) | ((uint64_t)1 << 52);
    long long e = exp - 1075; /* d = mant * 2^e, and e > 0 past 2^63 */
    long long n = (53 + e + 63) / 64 + 1;
    KBytes* r = k_big_room(n);
    KLimbU* l = k_big_limbs(r);
    for (long long i = 0; i < n; i++) l[i] = 0;
    for (int k = 0; k < 53; k++) {
        if (!(mant >> k & 1)) continue;
        long long bit = k + e;
        l[bit / 64] |= (KLimb)1 << (bit % 64);
    }
    return k_big_finish(r, neg, n);
}

/* The decimal digits of a bignum, sign first, into a fresh arena string. Each
   pass divides by 10^19, the largest power of ten a limb holds, and writes
   the remainder's nineteen digits. */
KValue k_str_n(const char* data, long long len);
static KValue k_big_render(KValue x) {
    KBigView a;
    k_big_view(x, &a);
    const KLimb ten19 = 10000000000000000000ull;
    KLimb* w = malloc(8 * (size_t)a.n);
    for (long long i = 0; i < a.n; i++) w[i] = a.d[i];
    long long wn = a.n;
    /* twenty decimal digits per 64 bits is generous, plus a sign */
    long long cap = a.n * 20 + 2;
    char* buf = malloc((size_t)cap);
    long long at = cap;
    while (wn > 0) {
        KLimb rem = 0;
        for (long long i = wn - 1; i >= 0; i--) w[i] = k_div_2by1(rem, w[i], ten19, &rem);
        while (wn > 0 && w[wn - 1] == 0) wn--;
        for (int k = 0; k < 19; k++) {
            buf[--at] = (char)('0' + rem % 10);
            rem /= 10;
            if (wn == 0 && rem == 0) break;
        }
    }
    if (a.neg) buf[--at] = '-';
    KValue out = k_str_n(buf + at, cap - at);
    free(w);
    free(buf);
    return out;
}

/* The int a run of decimal digits spells, with an optional leading '-'. The
   digits go nineteen at a time, each chunk scaling what came before. */
KValue k_int_of_digits(const char* s, long long len) {
    int neg = 0;
    if (len > 0 && (s[0] == '-' || s[0] == '+')) { neg = s[0] == '-'; s++; len--; }
    long long n = len / 19 + 2;
    KBytes* r = k_big_room(n);
    KLimbU* d = k_big_limbs(r);
    long long dn = 0;
    long long i = 0;
    while (i < len) {
        long long take = len - i < 19 ? len - i : 19;
        KLimb chunk = 0, scale = 1;
        for (long long k = 0; k < take; k++) { chunk = chunk * 10 + (KLimb)(s[i + k] - '0'); scale *= 10; }
        KLimb carry = chunk;
        for (long long k = 0; k < dn; k++) {
            KWide t = (KWide)d[k] * scale + carry;
            d[k] = (KLimb)t;
            carry = (KLimb)(t >> 64);
        }
        if (carry) d[dn++] = carry;
        i += take;
    }
    return k_big_finish(r, neg, dn);
}
/* What a succeeded effect yields: `none` is absence and nothing else. */
KValue k_done(void) { KValue v; v.tag = K_DONE; v.payload = 0; return v; }

static KValue k_ascii_cache[128];
static char k_ascii_ready[128];

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
KValue k_b_render_value(KValue v) {
    if (v.tag == K_BIG) return k_big_render(v);
    if (v.tag == K_INT) {
        long long x = v.payload;
        /* One digit is a one-byte string, which k_str_n answers from the
           ascii cache without allocating; so does this. */
        if ((unsigned long long)x < 10) {
            char c = (char)('0' + x);
            return k_str_n(&c, 1);
        }
        uint64_t u = x < 0 ? (uint64_t)(-(x + 1)) + 1 : (uint64_t)x;
        KStr* s = k_str_alloc((long long)ryu_declen(u) + (x < 0));
        k_itoa(s->data, x);
        KValue out; out.tag = K_STR; out.payload = k_ptr(s); return out;
    }
    return k_render(v, 0);
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
    if (original.tag != K_ERR) k_die("wrap_err takes a reason and the err it wraps");
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

/* The by-value record convention packs a two-field record into two words:
   field 0's int payload shifted above field 1's tag in the first word,
   field 1's payload in the second. A failure crosses in the same two words
   untouched: its first word is exactly K_ERR, and no packed value's first
   word is, since a packed word carries an int above bit eight over a field
   the analysis proved is never a failure. These are the only conversions
   between the two shapes, and all of them pass a failure through.
   Until 2026-09-16 the emitter inlined them and neither did: a failure
   staged through a beat carry was boxed as a record whose second field was
   the failure, k_rec merged that into a failure, and the unpack on the far
   side read two fields off it and handed the consumer a value whose first
   word was not K_ERR. The consumer's record arm matched a failure.

   The convention was built for the scanner's byte position, but the
   analysis admits any record whose first field is an int. Eight bits of a
   64-bit int do not survive the shift, so an int outside 56 signed bits
   does not pack, and neither does a bignum: the record is built on the heap and crosses as
   K_PARSED_WIDE in the low byte, which no value's tag is, with the record
   in the second word. The shift back is arithmetic, so a negative int
   keeps its sign. Until 2026-09-29 it was logical and neither case was
   handled: `pt (x - 8) "a"` at x = 0 read back as 2^56 - 8. */
#define K_PARSED_WIDE 255
static int k_packs(long long n) {
    return (long long)((unsigned long long)n << 8) >> 8 == n;
}
static KValue k_parsed_spilled(long long w1) {
    KValue v; v.tag = K_REC; v.payload = w1; return v;
}
KValue k_parsed_box(long long type_id, long long w0, long long w1) {
    KValue words; words.tag = w0; words.payload = w1;
    if (!k_not_failure(words)) return words;
    if ((w0 & 255) == K_PARSED_WIDE) return k_parsed_spilled(w1);
    KValue fields[2];
    fields[0].tag = K_INT; fields[0].payload = w0 >> 8;
    fields[1].tag = w0 & 255; fields[1].payload = w1;
    return k_rec(type_id, 2, fields);
}
KValue k_parsed_words(KValue v) {
    if (!k_not_failure(v)) return v;
    KValue f0 = k_field(v, 0), f1 = k_field(v, 1);
    KValue out;
    if (f0.tag != K_INT || !k_packs(f0.payload)) {
        out.tag = K_PARSED_WIDE;
        out.payload = k_ptr(k_as_rec(v.tag == K_SUB ? k_sub_base(v) : v));
        return out;
    }
    out.tag = (long long)((unsigned long long)f0.payload << 8) | f1.tag;
    out.payload = f1.payload;
    return out;
}
/* The emitter's slow side of a pack: the int did not fit, so the record is
   built here and handed back in its spilled words. */
KValue k_parsed_spill(long long type_id, long long ntag, long long n, long long vtag, long long vpay) {
    KValue fields[2];
    fields[0].tag = ntag; fields[0].payload = n;
    fields[1].tag = vtag; fields[1].payload = vpay;
    KValue out; out.tag = K_PARSED_WIDE; out.payload = k_rec(type_id, 2, fields).payload;
    return out;
}
KValue k_parsed_wide_first(long long w1) { return k_as_rec(k_parsed_spilled(w1))->fields[0]; }
KValue k_parsed_wide_value(long long w1) { return k_as_rec(k_parsed_spilled(w1))->fields[1]; }
KValue k_err_inner(KValue v) { return k_err_box(v)->reason; }

/* An err's three readers, `.reason`, `.cause` and `.origin` — the second
   deliberate hole in infectiousness beside `wrap_err`'s second argument. The
   dispatcher of a reader's getter calls this at its entry, before any arm,
   and only for an err; the interpreter's `err_read` is the oracle. */
long long k_is_err(KValue v) { return v.tag == K_ERR; }
KValue k_err_read(KValue v, const char* field) {
    KErrBox* box = k_err_box(v);
    if (field[0] == 'r') return box->reason;
    if (field[0] == 'c') {
        if (!box->cause) return k_none();
        KValue out; out.tag = K_ERR; out.payload = k_ptr(box->cause); return out;
    }
    if (!box->origin) return k_none();
    return k_str(box->origin);
}

/* pattern checks: nonzero on match */
long long k_check_tag(KValue v, long long tag) { return v.tag == tag; }
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
        exit(1);
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
    exit(1);
}

/* `.` field access: failures ride through; a non-record dies loudly. */
// A getter group that matched nothing. The reader wrote `x.name`, so the
// failure is a field error in their words, not a dispatch error in ours.
void k_no_field(KValue v, const char* name) {
    if (v.tag != K_REC) {
        KValue shown = k_shown(v, 1);
        fprintf(stderr, "%serror[runtime]:%s `.` reads a field of a record, not %s\n",
                k_c_err(), k_c_off(), k_cstr(k_as_str(shown)));
        exit(1);
    }
    KRec* r = k_as_rec(v);
    fprintf(stderr, "%serror[runtime]:%s `%s` has no field `%s`\n", k_c_err(), k_c_off(),
            k_type_name(r->type_id), name);
    exit(1);
}

__attribute__((noreturn, noinline)) static void k_die_got(const char* msg, KValue v) {
    KValue shown = k_shown(v, 0);
    fprintf(stderr, "%serror[runtime]:%s %s, got %s\n", k_c_err(), k_c_off(), msg,
            k_cstr(k_as_str(shown)));
    exit(1);
}

__attribute__((noreturn, noinline)) static void k_die_value(const char* msg, KValue v) {
    KValue shown = k_shown(v, 1);
    fprintf(stderr, "%serror[runtime]:%s %s, not %s\n", k_c_err(), k_c_off(), msg,
            k_cstr(k_as_str(shown)));
    exit(1);
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
        exit(1);
    }
    KRec* r = k_as_rec(v);
    for (long long i = 0; i < r->nfields; i++) {
        if (!strcmp(k_type_field_name(r->type_id, i), name)) return r->fields[i];
    }
    fprintf(stderr, "%serror[runtime]:%s `%s` has no field `%s`\n", k_c_err(), k_c_off(),
            k_type_name(r->type_id), name);
    exit(1);
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
    exit(1);
}

/* ryu d2s tables (adams, PLDI 2018): 125-bit multipliers, generated exactly */

/* Powers of a hundred, for taking `vr` down in one step once the trip count
   is known. Nine entries: the loop below removes two digits a trip and `vr`
   starts with at most twenty. A u64 is below 1.9e19, so nine pairs is the
   most that can come off and index nine is the last the step reads. The
   table was written with nine entries first and the differential harness
   found the read past the end before anything was measured. */
static const uint64_t RYU_POW100[10] = {
    1ULL, 100ULL, 10000ULL, 1000000ULL, 100000000ULL, 10000000000ULL,
    1000000000000ULL, 100000000000000ULL, 10000000000000000ULL,
    1000000000000000000ULL,
};
#define RYU_POW5_BITCOUNT 125
#define RYU_POW5_INV_BITCOUNT 125
static const uint64_t RYU_POW5[326][2] = {
    {0x0000000000000000ULL, 0x1000000000000000ULL},
    {0x0000000000000000ULL, 0x1400000000000000ULL},
    {0x0000000000000000ULL, 0x1900000000000000ULL},
    {0x0000000000000000ULL, 0x1f40000000000000ULL},
    {0x0000000000000000ULL, 0x1388000000000000ULL},
    {0x0000000000000000ULL, 0x186a000000000000ULL},
    {0x0000000000000000ULL, 0x1e84800000000000ULL},
    {0x0000000000000000ULL, 0x1312d00000000000ULL},
    {0x0000000000000000ULL, 0x17d7840000000000ULL},
    {0x0000000000000000ULL, 0x1dcd650000000000ULL},
    {0x0000000000000000ULL, 0x12a05f2000000000ULL},
    {0x0000000000000000ULL, 0x174876e800000000ULL},
    {0x0000000000000000ULL, 0x1d1a94a200000000ULL},
    {0x0000000000000000ULL, 0x12309ce540000000ULL},
    {0x0000000000000000ULL, 0x16bcc41e90000000ULL},
    {0x0000000000000000ULL, 0x1c6bf52634000000ULL},
    {0x0000000000000000ULL, 0x11c37937e0800000ULL},
    {0x0000000000000000ULL, 0x16345785d8a00000ULL},
    {0x0000000000000000ULL, 0x1bc16d674ec80000ULL},
    {0x0000000000000000ULL, 0x1158e460913d0000ULL},
    {0x0000000000000000ULL, 0x15af1d78b58c4000ULL},
    {0x0000000000000000ULL, 0x1b1ae4d6e2ef5000ULL},
    {0x0000000000000000ULL, 0x10f0cf064dd59200ULL},
    {0x0000000000000000ULL, 0x152d02c7e14af680ULL},
    {0x0000000000000000ULL, 0x1a784379d99db420ULL},
    {0x0000000000000000ULL, 0x108b2a2c28029094ULL},
    {0x0000000000000000ULL, 0x14adf4b7320334b9ULL},
    {0x4000000000000000ULL, 0x19d971e4fe8401e7ULL},
    {0x8800000000000000ULL, 0x1027e72f1f128130ULL},
    {0xaa00000000000000ULL, 0x1431e0fae6d7217cULL},
    {0xd480000000000000ULL, 0x193e5939a08ce9dbULL},
    {0xc9a0000000000000ULL, 0x1f8def8808b02452ULL},
    {0xbe04000000000000ULL, 0x13b8b5b5056e16b3ULL},
    {0xad85000000000000ULL, 0x18a6e32246c99c60ULL},
    {0xd8e6400000000000ULL, 0x1ed09bead87c0378ULL},
    {0x878fe80000000000ULL, 0x13426172c74d822bULL},
    {0x6973e20000000000ULL, 0x1812f9cf7920e2b6ULL},
    {0x03d0da8000000000ULL, 0x1e17b84357691b64ULL},
    {0x8262889000000000ULL, 0x12ced32a16a1b11eULL},
    {0x22fb2ab400000000ULL, 0x178287f49c4a1d66ULL},
    {0xabb9f56100000000ULL, 0x1d6329f1c35ca4bfULL},
    {0xcb54395ca0000000ULL, 0x125dfa371a19e6f7ULL},
    {0xbe2947b3c8000000ULL, 0x16f578c4e0a060b5ULL},
    {0x2db399a0ba000000ULL, 0x1cb2d6f618c878e3ULL},
    {0xfc90400474400000ULL, 0x11efc659cf7d4b8dULL},
    {0x7bb4500591500000ULL, 0x166bb7f0435c9e71ULL},
    {0xdaa16406f5a40000ULL, 0x1c06a5ec5433c60dULL},
    {0xa8a4de8459868000ULL, 0x118427b3b4a05bc8ULL},
    {0xd2ce16256fe82000ULL, 0x15e531a0a1c872baULL},
    {0x87819baecbe22800ULL, 0x1b5e7e08ca3a8f69ULL},
    {0xf4b1014d3f6d5900ULL, 0x111b0ec57e6499a1ULL},
    {0x71dd41a08f48af40ULL, 0x1561d276ddfdc00aULL},
    {0x0e549208b31adb10ULL, 0x1aba4714957d300dULL},
    {0x28f4db456ff0c8eaULL, 0x10b46c6cdd6e3e08ULL},
    {0x33321216cbecfb24ULL, 0x14e1878814c9cd8aULL},
    {0xbffe969c7ee839edULL, 0x1a19e96a19fc40ecULL},
    {0xf7ff1e21cf512434ULL, 0x105031e2503da893ULL},
    {0xf5fee5aa43256d41ULL, 0x14643e5ae44d12b8ULL},
    {0x337e9f14d3eec892ULL, 0x197d4df19d605767ULL},
    {0x005e46da08ea7ab6ULL, 0x1fdca16e04b86d41ULL},
    {0xa03aec4845928cb2ULL, 0x13e9e4e4c2f34448ULL},
    {0xc849a75a56f72fdeULL, 0x18e45e1df3b0155aULL},
    {0x7a5c1130ecb4fbd6ULL, 0x1f1d75a5709c1ab1ULL},
    {0xec798abe93f11d65ULL, 0x13726987666190aeULL},
    {0xa797ed6e38ed64bfULL, 0x184f03e93ff9f4daULL},
    {0x517de8c9c728bdefULL, 0x1e62c4e38ff87211ULL},
    {0xd2eeb17e1c7976b5ULL, 0x12fdbb0e39fb474aULL},
    {0x87aa5ddda397d462ULL, 0x17bd29d1c87a191dULL},
    {0xe994f5550c7dc97bULL, 0x1dac74463a989f64ULL},
    {0x11fd195527ce9dedULL, 0x128bc8abe49f639fULL},
    {0xd67c5faa71c24568ULL, 0x172ebad6ddc73c86ULL},
    {0x8c1b77950e32d6c2ULL, 0x1cfa698c95390ba8ULL},
    {0x57912abd28dfc639ULL, 0x121c81f7dd43a749ULL},
    {0xad75756c7317b7c8ULL, 0x16a3a275d494911bULL},
    {0x98d2d2c78fdda5baULL, 0x1c4c8b1349b9b562ULL},
    {0x9f83c3bcb9ea8794ULL, 0x11afd6ec0e14115dULL},
    {0x0764b4abe8652979ULL, 0x161bcca7119915b5ULL},
    {0x493de1d6e27e73d7ULL, 0x1ba2bfd0d5ff5b22ULL},
    {0x6dc6ad264d8f0866ULL, 0x1145b7e285bf98f5ULL},
    {0xc938586fe0f2ca80ULL, 0x159725db272f7f32ULL},
    {0x7b866e8bd92f7d20ULL, 0x1afcef51f0fb5effULL},
    {0xad34051767bdae34ULL, 0x10de1593369d1b5fULL},
    {0x9881065d41ad19c1ULL, 0x15159af804446237ULL},
    {0x7ea147f492186032ULL, 0x1a5b01b605557ac5ULL},
    {0x6f24ccf8db4f3c1fULL, 0x1078e111c3556cbbULL},
    {0x4aee003712230b27ULL, 0x14971956342ac7eaULL},
    {0xdda98044d6abcdf0ULL, 0x19bcdfabc13579e4ULL},
    {0x0a89f02b062b60b6ULL, 0x10160bcb58c16c2fULL},
    {0xcd2c6c35c7b638e4ULL, 0x141b8ebe2ef1c73aULL},
    {0x8077874339a3c71dULL, 0x1922726dbaae3909ULL},
    {0xe0956914080cb8e4ULL, 0x1f6b0f092959c74bULL},
    {0x6c5d61ac8507f38eULL, 0x13a2e965b9d81c8fULL},
    {0x4774ba17a649f072ULL, 0x188ba3bf284e23b3ULL},
    {0x1951e89d8fdc6c8fULL, 0x1eae8caef261aca0ULL},
    {0x0fd3316279e9c3d9ULL, 0x132d17ed577d0be4ULL},
    {0x13c7fdbb186434cfULL, 0x17f85de8ad5c4eddULL},
    {0x58b9fd29de7d4203ULL, 0x1df67562d8b36294ULL},
    {0xb7743e3a2b0e4942ULL, 0x12ba095dc7701d9cULL},
    {0xe5514dc8b5d1db92ULL, 0x17688bb5394c2503ULL},
    {0xdea5a13ae3465277ULL, 0x1d42aea2879f2e44ULL},
    {0x0b2784c4ce0bf38aULL, 0x1249ad2594c37cebULL},
    {0xcdf165f6018ef06dULL, 0x16dc186ef9f45c25ULL},
    {0x416dbf7381f2ac88ULL, 0x1c931e8ab871732fULL},
    {0x88e497a83137abd5ULL, 0x11dbf316b346e7fdULL},
    {0xeb1dbd923d8596caULL, 0x1652efdc6018a1fcULL},
    {0x25e52cf6cce6fc7dULL, 0x1be7abd3781eca7cULL},
    {0x97af3c1a40105dceULL, 0x1170cb642b133e8dULL},
    {0xfd9b0b20d0147542ULL, 0x15ccfe3d35d80e30ULL},
    {0x3d01cde904199292ULL, 0x1b403dcc834e11bdULL},
    {0x462120b1a28ffb9bULL, 0x1108269fd210cb16ULL},
    {0xd7a968de0b33fa82ULL, 0x154a3047c694fddbULL},
    {0xcd93c3158e00f923ULL, 0x1a9cbc59b83a3d52ULL},
    {0xc07c59ed78c09bb6ULL, 0x10a1f5b813246653ULL},
    {0xb09b7068d6f0c2a3ULL, 0x14ca732617ed7fe8ULL},
    {0xdcc24c830cacf34cULL, 0x19fd0fef9de8dfe2ULL},
    {0xc9f96fd1e7ec180fULL, 0x103e29f5c2b18bedULL},
    {0x3c77cbc661e71e13ULL, 0x144db473335deee9ULL},
    {0x8b95beb7fa60e598ULL, 0x1961219000356aa3ULL},
    {0x6e7b2e65f8f91efeULL, 0x1fb969f40042c54cULL},
    {0xc50cfcffbb9bb35fULL, 0x13d3e2388029bb4fULL},
    {0xb6503c3faa82a037ULL, 0x18c8dac6a0342a23ULL},
    {0xa3e44b4f95234844ULL, 0x1efb1178484134acULL},
    {0xe66eaf11bd360d2bULL, 0x135ceaeb2d28c0ebULL},
    {0xe00a5ad62c839075ULL, 0x183425a5f872f126ULL},
    {0x980cf18bb7a47493ULL, 0x1e412f0f768fad70ULL},
    {0x5f0816f752c6c8dcULL, 0x12e8bd69aa19cc66ULL},
    {0xf6ca1cb527787b13ULL, 0x17a2ecc414a03f7fULL},
    {0xf47ca3e2715699d7ULL, 0x1d8ba7f519c84f5fULL},
    {0xf8cde66d86d62026ULL, 0x127748f9301d319bULL},
    {0xf7016008e88ba830ULL, 0x17151b377c247e02ULL},
    {0xb4c1b80b22ae923cULL, 0x1cda62055b2d9d83ULL},
    {0x50f91306f5ad1b65ULL, 0x12087d4358fc8272ULL},
    {0xe53757c8b318623fULL, 0x168a9c942f3ba30eULL},
    {0x9e852dbadfde7acfULL, 0x1c2d43b93b0a8bd2ULL},
    {0xa3133c94cbeb0cc1ULL, 0x119c4a53c4e69763ULL},
    {0x8bd80bb9fee5cff1ULL, 0x16035ce8b6203d3cULL},
    {0xaece0ea87e9f43eeULL, 0x1b843422e3a84c8bULL},
    {0x4d40c9294f238a75ULL, 0x1132a095ce492fd7ULL},
    {0x2090fb73a2ec6d12ULL, 0x157f48bb41db7bcdULL},
    {0x68b53a508ba78856ULL, 0x1adf1aea12525ac0ULL},
    {0x417144725748b536ULL, 0x10cb70d24b7378b8ULL},
    {0x51cd958eed1ae283ULL, 0x14fe4d06de5056e6ULL},
    {0xe640faf2a8619b24ULL, 0x1a3de04895e46c9fULL},
    {0xefe89cd7a93d00f7ULL, 0x1066ac2d5daec3e3ULL},
    {0xebe2c40d938c4134ULL, 0x14805738b51a74dcULL},
    {0x26db7510f86f5181ULL, 0x19a06d06e2611214ULL},
    {0x9849292a9b4592f1ULL, 0x100444244d7cab4cULL},
    {0xbe5b73754216f7adULL, 0x1405552d60dbd61fULL},
    {0xadf25052929cb598ULL, 0x1906aa78b912cba7ULL},
    {0x996ee4673743e2ffULL, 0x1f485516e7577e91ULL},
    {0xffe54ec0828a6ddfULL, 0x138d352e5096af1aULL},
    {0xbfdea270a32d0957ULL, 0x18708279e4bc5ae1ULL},
    {0x2fd64b0ccbf84badULL, 0x1e8ca3185deb719aULL},
    {0x5de5eee7ff7b2f4cULL, 0x1317e5ef3ab32700ULL},
    {0x755f6aa1ff59fb1fULL, 0x17dddf6b095ff0c0ULL},
    {0x92b7454a7f3079e7ULL, 0x1dd55745cbb7ecf0ULL},
    {0x5bb28b4e8f7e4c30ULL, 0x12a5568b9f52f416ULL},
    {0xf29f2e22335ddf3cULL, 0x174eac2e8727b11bULL},
    {0xef46f9aac035570bULL, 0x1d22573a28f19d62ULL},
    {0xd58c5c0ab8215667ULL, 0x123576845997025dULL},
    {0x4aef730d6629ac01ULL, 0x16c2d4256ffcc2f5ULL},
    {0x9dab4fd0bfb41701ULL, 0x1c73892ecbfbf3b2ULL},
    {0xa28b11e277d08e60ULL, 0x11c835bd3f7d784fULL},
    {0x8b2dd65b15c4b1f9ULL, 0x163a432c8f5cd663ULL},
    {0x6df94bf1db35de77ULL, 0x1bc8d3f7b3340bfcULL},
    {0xc4bbcf772901ab0aULL, 0x115d847ad000877dULL},
    {0x35eac354f34215cdULL, 0x15b4e5998400a95dULL},
    {0x8365742a30129b40ULL, 0x1b221effe500d3b4ULL},
    {0xd21f689a5e0ba108ULL, 0x10f5535fef208450ULL},
    {0x06a742c0f58e894aULL, 0x1532a837eae8a565ULL},
    {0x4851137132f22b9dULL, 0x1a7f5245e5a2cebeULL},
    {0xed32ac26bfd75b42ULL, 0x108f936baf85c136ULL},
    {0xa87f57306fcd3212ULL, 0x14b378469b673184ULL},
    {0xd29f2cfc8bc07e97ULL, 0x19e056584240fde5ULL},
    {0xa3a37c1dd7584f1eULL, 0x102c35f729689eafULL},
    {0x8c8c5b254d2e62e6ULL, 0x14374374f3c2c65bULL},
    {0x6faf71eea079fb9fULL, 0x1945145230b377f2ULL},
    {0x0b9b4e6a48987a87ULL, 0x1f965966bce055efULL},
    {0x674111026d5f4c94ULL, 0x13bdf7e0360c35b5ULL},
    {0xc111554308b71fbaULL, 0x18ad75d8438f4322ULL},
    {0x7155aa93cae4e7a8ULL, 0x1ed8d34e547313ebULL},
    {0x26d58a9c5ecf10c9ULL, 0x13478410f4c7ec73ULL},
    {0xf08aed437682d4fbULL, 0x1819651531f9e78fULL},
    {0xecada89454238a3aULL, 0x1e1fbe5a7e786173ULL},
    {0x73ec895cb4963664ULL, 0x12d3d6f88f0b3ce8ULL},
    {0x90e7abb3e1bbc3fdULL, 0x1788ccb6b2ce0c22ULL},
    {0x352196a0da2ab4fdULL, 0x1d6affe45f818f2bULL},
    {0x0134fe24885ab11eULL, 0x1262dfeebbb0f97bULL},
    {0xc1823dadaa715d65ULL, 0x16fb97ea6a9d37d9ULL},
    {0x31e2cd19150db4bfULL, 0x1cba7de5054485d0ULL},
    {0x1f2dc02fad2890f7ULL, 0x11f48eaf234ad3a2ULL},
    {0xa6f9303b9872b535ULL, 0x1671b25aec1d888aULL},
    {0x50b77c4a7e8f6282ULL, 0x1c0e1ef1a724eaadULL},
    {0x5272adae8f199d91ULL, 0x1188d357087712acULL},
    {0x670f591a32e004f6ULL, 0x15eb082cca94d757ULL},
    {0x40d32f60bf980633ULL, 0x1b65ca37fd3a0d2dULL},
    {0x4883fd9c77bf03e0ULL, 0x111f9e62fe44483cULL},
    {0x5aa4fd0395aec4d8ULL, 0x156785fbbdd55a4bULL},
    {0x314e3c447b1a760eULL, 0x1ac1677aad4ab0deULL},
    {0xded0e5aaccf089c9ULL, 0x10b8e0acac4eae8aULL},
    {0x96851f15802cac3bULL, 0x14e718d7d7625a2dULL},
    {0xfc2666dae037d74aULL, 0x1a20df0dcd3af0b8ULL},
    {0x9d980048cc22e68eULL, 0x10548b68a044d673ULL},
    {0x84fe005aff2ba032ULL, 0x1469ae42c8560c10ULL},
    {0xa63d8071bef6883eULL, 0x198419d37a6b8f14ULL},
    {0xcfcce08e2eb42a4eULL, 0x1fe52048590672d9ULL},
    {0x21e00c58dd309a70ULL, 0x13ef342d37a407c8ULL},
    {0x2a580f6f147cc10dULL, 0x18eb0138858d09baULL},
    {0xb4ee134ad99bf150ULL, 0x1f25c186a6f04c28ULL},
    {0x7114cc0ec80176d2ULL, 0x137798f428562f99ULL},
    {0xcd59ff127a01d486ULL, 0x18557f31326bbb7fULL},
    {0xc0b07ed7188249a8ULL, 0x1e6adefd7f06aa5fULL},
    {0xd86e4f466f516e09ULL, 0x1302cb5e6f642a7bULL},
    {0xce89e3180b25c98bULL, 0x17c37e360b3d351aULL},
    {0x822c5bde0def3beeULL, 0x1db45dc38e0c8261ULL},
    {0xf15bb96ac8b58575ULL, 0x1290ba9a38c7d17cULL},
    {0x2db2a7c57ae2e6d2ULL, 0x1734e940c6f9c5dcULL},
    {0x391f51b6d99ba086ULL, 0x1d022390f8b83753ULL},
    {0x03b3931248014454ULL, 0x1221563a9b732294ULL},
    {0x04a077d6da019569ULL, 0x16a9abc9424feb39ULL},
    {0x45c895cc9081fac3ULL, 0x1c5416bb92e3e607ULL},
    {0x8b9d5d9fda513cbaULL, 0x11b48e353bce6fc4ULL},
    {0xae84b507d0e58be8ULL, 0x1621b1c28ac20bb5ULL},
    {0x1a25e249c51eeee3ULL, 0x1baa1e332d728ea3ULL},
    {0xf057ad6e1b33554dULL, 0x114a52dffc679925ULL},
    {0x6c6d98c9a2002aa1ULL, 0x159ce797fb817f6fULL},
    {0x4788fefc0a803549ULL, 0x1b04217dfa61df4bULL},
    {0x0cb59f5d8690214eULL, 0x10e294eebc7d2b8fULL},
    {0xcfe30734e83429a1ULL, 0x151b3a2a6b9c7672ULL},
    {0x83dbc9022241340aULL, 0x1a6208b50683940fULL},
    {0xb2695da15568c086ULL, 0x107d457124123c89ULL},
    {0x1f03b509aac2f0a7ULL, 0x149c96cd6d16cbacULL},
    {0x26c4a24c1573acd1ULL, 0x19c3bc80c85c7e97ULL},
    {0x783ae56f8d684c03ULL, 0x101a55d07d39cf1eULL},
    {0x16499ecb70c25f03ULL, 0x1420eb449c8842e6ULL},
    {0x9bdc067e4cf2f6c4ULL, 0x19292615c3aa539fULL},
    {0x82d3081de02fb476ULL, 0x1f736f9b3494e887ULL},
    {0xb1c3e512ac1dd0c9ULL, 0x13a825c100dd1154ULL},
    {0xde34de57572544fcULL, 0x18922f31411455a9ULL},
    {0x55c215ed2cee963bULL, 0x1eb6bafd91596b14ULL},
    {0xb5994db43c151de5ULL, 0x133234de7ad7e2ecULL},
    {0xe2ffa1214b1a655eULL, 0x17fec216198ddba7ULL},
    {0xdbbf89699de0feb6ULL, 0x1dfe729b9ff15291ULL},
    {0x2957b5e202ac9f31ULL, 0x12bf07a143f6d39bULL},
    {0xf3ada35a8357c6feULL, 0x176ec98994f48881ULL},
    {0x70990c31242db8bdULL, 0x1d4a7bebfa31aaa2ULL},
    {0x865fa79eb69c9376ULL, 0x124e8d737c5f0aa5ULL},
    {0xe7f791866443b854ULL, 0x16e230d05b76cd4eULL},
    {0xa1f575e7fd54a669ULL, 0x1c9abd04725480a2ULL},
    {0xa53969b0fe54e801ULL, 0x11e0b622c774d065ULL},
    {0x0e87c41d3dea2202ULL, 0x1658e3ab7952047fULL},
    {0xd229b5248d64aa82ULL, 0x1bef1c9657a6859eULL},
    {0x435a1136d85eea91ULL, 0x117571ddf6c81383ULL},
    {0x143095848e76a536ULL, 0x15d2ce55747a1864ULL},
    {0x193cbae5b2144e83ULL, 0x1b4781ead1989e7dULL},
    {0x2fc5f4cf8f4cb112ULL, 0x110cb132c2ff630eULL},
    {0xbbb77203731fdd56ULL, 0x154fdd7f73bf3bd1ULL},
    {0x2aa54e844fe7d4acULL, 0x1aa3d4df50af0ac6ULL},
    {0xdaa75112b1f0e4ebULL, 0x10a6650b926d66bbULL},
    {0xd15125575e6d1e26ULL, 0x14cffe4e7708c06aULL},
    {0x85a56ead360865b0ULL, 0x1a03fde214caf085ULL},
    {0x7387652c41c53f8eULL, 0x10427ead4cfed653ULL},
    {0x50693e7752368f71ULL, 0x14531e58a03e8be8ULL},
    {0x64838e1526c4334eULL, 0x1967e5eec84e2ee2ULL},
    {0xfda4719a70754022ULL, 0x1fc1df6a7a61ba9aULL},
    {0xde86c70086494815ULL, 0x13d92ba28c7d14a0ULL},
    {0x162878c0a7db9a1aULL, 0x18cf768b2f9c59c9ULL},
    {0x5bb296f0d1d280a1ULL, 0x1f03542dfb83703bULL},
    {0x194f9e5683239064ULL, 0x1362149cbd322625ULL},
    {0x5fa385ec23ec747eULL, 0x183a99c3ec7eafaeULL},
    {0xf78c67672ce7919dULL, 0x1e494034e79e5b99ULL},
    {0x3ab7c0a07c10bb02ULL, 0x12edc82110c2f940ULL},
    {0x4965b0c89b14e9c3ULL, 0x17a93a2954f3b790ULL},
    {0x5bbf1cfac1da2433ULL, 0x1d9388b3aa30a574ULL},
    {0xb957721cb92856a0ULL, 0x127c35704a5e6768ULL},
    {0xe7ad4ea3e7726c48ULL, 0x171b42cc5cf60142ULL},
    {0xa198a24ce14f075aULL, 0x1ce2137f74338193ULL},
    {0x44ff65700cd16498ULL, 0x120d4c2fa8a030fcULL},
    {0x563f3ecc1005bdbeULL, 0x16909f3b92c83d3bULL},
    {0x2bcf0e7f14072d2eULL, 0x1c34c70a777a4c8aULL},
    {0x5b61690f6c847c3dULL, 0x11a0fc668aac6fd6ULL},
    {0xf239c35347a59b4cULL, 0x16093b802d578bcbULL},
    {0xeec83428198f021fULL, 0x1b8b8a6038ad6ebeULL},
    {0x553d20990ff96153ULL, 0x1137367c236c6537ULL},
    {0x2a8c68bf53f7b9a8ULL, 0x1585041b2c477e85ULL},
    {0x752f82ef28f5a812ULL, 0x1ae64521f7595e26ULL},
    {0x093db1d57999890bULL, 0x10cfeb353a97dad8ULL},
    {0x0b8d1e4ad7ffeb4eULL, 0x1503e602893dd18eULL},
    {0x8e7065dd8dffe622ULL, 0x1a44df832b8d45f1ULL},
    {0xf9063faa78bfefd5ULL, 0x106b0bb1fb384bb6ULL},
    {0xb747cf9516efebcaULL, 0x1485ce9e7a065ea4ULL},
    {0xe519c37a5cabe6bdULL, 0x19a742461887f64dULL},
    {0xaf301a2c79eb7036ULL, 0x1008896bcf54f9f0ULL},
    {0xdafc20b798664c43ULL, 0x140aabc6c32a386cULL},
    {0x11bb28e57e7fdf54ULL, 0x190d56b873f4c688ULL},
    {0x1629f31ede1fd72aULL, 0x1f50ac6690f1f82aULL},
    {0x4dda37f34ad3e67aULL, 0x13926bc01a973b1aULL},
    {0xe150c5f01d88e019ULL, 0x187706b0213d09e0ULL},
    {0x19a4f76c24eb181fULL, 0x1e94c85c298c4c59ULL},
    {0xb0071aa39712ef13ULL, 0x131cfd3999f7afb7ULL},
    {0x9c08e14c7cd7aad8ULL, 0x17e43c8800759ba5ULL},
    {0x030b199f9c0d958eULL, 0x1ddd4baa0093028fULL},
    {0x61e6f003c1887d79ULL, 0x12aa4f4a405be199ULL},
    {0xba60ac04b1ea9cd7ULL, 0x1754e31cd072d9ffULL},
    {0xa8f8d705de65440dULL, 0x1d2a1be4048f907fULL},
    {0xc99b8663aaff4a88ULL, 0x123a516e82d9ba4fULL},
    {0xbc0267fc95bf1d2aULL, 0x16c8e5ca239028e3ULL},
    {0xab0301fbbb2ee474ULL, 0x1c7b1f3cac74331cULL},
    {0xeae1e13d54fd4ec9ULL, 0x11ccf385ebc89ff1ULL},
    {0x659a598caa3ca27bULL, 0x1640306766bac7eeULL},
    {0xff00efefd4cbcb1aULL, 0x1bd03c81406979e9ULL},
    {0x3f6095f5e4ff5ef0ULL, 0x116225d0c841ec32ULL},
    {0xcf38bb735e3f36acULL, 0x15baaf44fa52673eULL},
    {0x8306ea5035cf0457ULL, 0x1b295b1638e7010eULL},
    {0x11e4527221a162b6ULL, 0x10f9d8ede39060a9ULL},
    {0x565d670eaa09bb64ULL, 0x15384f295c7478d3ULL},
    {0x2bf4c0d2548c2a3dULL, 0x1a8662f3b3919708ULL},
    {0x1b78f88374d79a66ULL, 0x1093fdd8503afe65ULL},
    {0x625736a4520d8100ULL, 0x14b8fd4e6449bdfeULL},
    {0xfaed044d6690e140ULL, 0x19e73ca1fd5c2d7dULL},
    {0xbcd422b0601a8cc8ULL, 0x103085e53e599c6eULL},
    {0x6c092b5c78212ffaULL, 0x143ca75e8df0038aULL},
    {0x070b763396297bf8ULL, 0x194bd136316c046dULL},
    {0x48ce53c07bb3daf6ULL, 0x1f9ec583bdc70588ULL},
    {0x2d80f4584d5068daULL, 0x13c33b72569c6375ULL},
    {0x78e1316e60a48310ULL, 0x18b40a4eec437c52ULL},
};
static const uint64_t RYU_POW5_INV[292][2] = {
    {0x0000000000000001ULL, 0x2000000000000000ULL},
    {0x999999999999999aULL, 0x1999999999999999ULL},
    {0x47ae147ae147ae15ULL, 0x147ae147ae147ae1ULL},
    {0x6c8b4395810624deULL, 0x10624dd2f1a9fbe7ULL},
    {0x7a786c226809d496ULL, 0x1a36e2eb1c432ca5ULL},
    {0x61f9f01b866e43abULL, 0x14f8b588e368f084ULL},
    {0xb4c7f34938583622ULL, 0x10c6f7a0b5ed8d36ULL},
    {0x87a6520ec08d236aULL, 0x1ad7f29abcaf4857ULL},
    {0x9fb841a566d74f88ULL, 0x15798ee2308c39dfULL},
    {0xe62d01511f12a607ULL, 0x112e0be826d694b2ULL},
    {0xd6ae6881cb5109a4ULL, 0x1b7cdfd9d7bdbab7ULL},
    {0xdef1ed34a2a73aeaULL, 0x15fd7fe17964955fULL},
    {0x7f27f0f6e885c8bbULL, 0x119799812dea1119ULL},
    {0x650cb4be40d60df8ULL, 0x1c25c268497681c2ULL},
    {0xea70909833de7193ULL, 0x16849b86a12b9b01ULL},
    {0x21f3a6e0297ec143ULL, 0x1203af9ee756159bULL},
    {0x6985d7cd0f313537ULL, 0x1cd2b297d889bc2bULL},
    {0x2137dfd73f5a90f9ULL, 0x170ef54646d49689ULL},
    {0xe75fe645cc4873faULL, 0x12725dd1d243aba0ULL},
    {0xa5663d3c7a0d865dULL, 0x1d83c94fb6d2ac34ULL},
    {0x511e976394d79eb1ULL, 0x179ca10c9242235dULL},
    {0xda7edf82dd794bc1ULL, 0x12e3b40a0e9b4f7dULL},
    {0x2a6498d1625bac68ULL, 0x1e392010175ee596ULL},
    {0xeeb6e0a781e2f053ULL, 0x182db34012b25144ULL},
    {0x58924d52ce4f26a9ULL, 0x1357c299a88ea76aULL},
    {0x27507bb7b07ea441ULL, 0x1ef2d0f5da7dd8aaULL},
    {0x52a6c95fc0655034ULL, 0x18c240c4aecb13bbULL},
    {0x0eebd44c99eaa690ULL, 0x13ce9a36f23c0fc9ULL},
    {0xb17953adc3110a80ULL, 0x1fb0f6be50601941ULL},
    {0xc12ddc8b02740867ULL, 0x195a5efea6b34767ULL},
    {0x3424b06f3529a052ULL, 0x14484bfeebc29f86ULL},
    {0x901d59f290ee19dbULL, 0x1039d66589687f9eULL},
    {0x4cfbc31db4b0295fULL, 0x19f623d5a8a73297ULL},
    {0x3d9635b15d59bab2ULL, 0x14c4e977ba1f5bacULL},
    {0x97ab5e277de16228ULL, 0x109d8792fb4c4956ULL},
    {0xf2abc9d8c9689d0dULL, 0x1a95a5b7f87a0ef0ULL},
    {0x5bbca17a3aba173eULL, 0x154484932d2e725aULL},
    {0xafca1ac82efb45cbULL, 0x11039d428a8b8eaeULL},
    {0xb2dcf7a6b1920945ULL, 0x1b38fb9daa78e44aULL},
    {0xf57d92ebc141a104ULL, 0x15c72fb1552d836eULL},
    {0xc46475896767b403ULL, 0x116c262777579c58ULL},
    {0x6d6d88dbd8a5ecd2ULL, 0x1be03d0bf225c6f4ULL},
    {0x8abe071646eb23dbULL, 0x164cfda3281e38c3ULL},
    {0x6efe6c11d255b649ULL, 0x11d7314f534b609cULL},
    {0xb197134fb6ef8a0eULL, 0x1c8b821885456760ULL},
    {0x27ac0f72f8bfa1a5ULL, 0x16d601ad376ab91aULL},
    {0xb95672c260994e1eULL, 0x1244ce242c5560e1ULL},
    {0xf5571e03cdc21695ULL, 0x1d3ae36d13bbce35ULL},
    {0x2aac18030b01ababULL, 0x17624f8a762fd82bULL},
    {0xbbbce0026f348956ULL, 0x12b50c6ec4f31355ULL},
    {0x92c7ccd0b1eda889ULL, 0x1dee7a4ad4b81eefULL},
    {0xdbd30a408e57ba07ULL, 0x17f1fb6f10934bf2ULL},
    {0x7ca8d50071dfc806ULL, 0x1327fc58da0f6ff5ULL},
    {0xfaa7bb33e9660cd6ULL, 0x1ea6608e29b24cbbULL},
    {0x9552fc298784d711ULL, 0x18851a0b548ea3c9ULL},
    {0xaaa8c9bad2d0ac0eULL, 0x139dae6f76d88307ULL},
    {0xdddadc5e1e1aace3ULL, 0x1f62b0b257c0d1a5ULL},
    {0x7e48b04b4b488a4fULL, 0x191bc08eac9a4151ULL},
    {0xcb6d59d5d5d3a1d9ULL, 0x141633a556e1cddaULL},
    {0x3c577b1177dc817bULL, 0x1011c2eaabe7d7e2ULL},
    {0xc6f25e825960cf2aULL, 0x19b604aaaca62636ULL},
    {0x6bf518684780a5bbULL, 0x14919d5556eb51c5ULL},
    {0x232a79ed06008496ULL, 0x10747ddddf22a7d1ULL},
    {0xd1dd8fe1a3340756ULL, 0x1a53fc9631d10c81ULL},
    {0xa7e4731ae8f66c45ULL, 0x150ffd44f4a73d34ULL},
    {0x531d28e253f8569eULL, 0x10d9976a5d52975dULL},
    {0xeb61db03b98d5762ULL, 0x1af5bf109550f22eULL},
    {0xbc4e48cfc7a445e8ULL, 0x159165a6ddda5b58ULL},
    {0x6371d3d96c836b20ULL, 0x11411e1f17e1e2adULL},
    {0x9f1c8628ad9f11cdULL, 0x1b9b6364f3030448ULL},
    {0xe5b06b53be18db0bULL, 0x1615e91d8f359d06ULL},
    {0xeaf3890fcb4715a2ULL, 0x11ab20e472914a6bULL},
    {0x44b8db4c7871bc37ULL, 0x1c45016d841baa46ULL},
    {0x03c715d6c6c1635fULL, 0x169d9abe03495505ULL},
    {0x3638de456bcde919ULL, 0x1217aefe69077737ULL},
    {0x56c163a2461641c1ULL, 0x1cf2b1970e725858ULL},
    {0xdf011c81d1ab67ceULL, 0x17288e1271f51379ULL},
    {0x7f3416ce4155eca5ULL, 0x1286d80ec190dc61ULL},
    {0x6520247d3556476eULL, 0x1da48ce468e7c702ULL},
    {0xea801d30f7783925ULL, 0x17b6d71d20b96c01ULL},
    {0xbb99b0f3f92cfa84ULL, 0x12f8ac174d612334ULL},
    {0x5f5c4e532847f739ULL, 0x1e5aacf215683854ULL},
    {0x7f7d0b75b9d32c2eULL, 0x18488a5b44536043ULL},
    {0x9930d5f7c7dc2358ULL, 0x136d3b7c36a919cfULL},
    {0x8eb4898c72f9d226ULL, 0x1f152bf9f10e8fb2ULL},
    {0x722a07a38f2e41b8ULL, 0x18ddbcc7f40ba628ULL},
    {0xc1bb394fa5be9afaULL, 0x13e497065cd61e86ULL},
    {0x9c5ec2190930f7f6ULL, 0x1fd424d6faf030d7ULL},
    {0x49e56814075a5ff8ULL, 0x197683df2f268d79ULL},
    {0x6e51201005e1e660ULL, 0x145ecfe5bf520ac7ULL},
    {0xf1da800cd181851aULL, 0x104bd984990e6f05ULL},
    {0x4fc400148268d4f5ULL, 0x1a12f5a0f4e3e4d6ULL},
    {0xd96999aa01ed772bULL, 0x14dbf7b3f71cb711ULL},
    {0xadee1488018ac5bcULL, 0x10aff95cc5b09274ULL},
    {0x497ceda668de092cULL, 0x1ab328946f80ea54ULL},
    {0x3aca57b853e4d424ULL, 0x155c2076bf9a5510ULL},
    {0x623b7960431d7683ULL, 0x1116805effaeaa73ULL},
    {0x9d2bf566d1c8bd9eULL, 0x1b5733cb32b110b8ULL},
    {0x7dbcc452416d647fULL, 0x15df5ca28ef40d60ULL},
    {0xcafd69db678ab6ccULL, 0x117f7d4ed8c33de6ULL},
    {0xab2f0fc572778adfULL, 0x1bff2ee48e052fd7ULL},
    {0x88f273045b92d580ULL, 0x1665bf1d3e6a8cacULL},
    {0xd3f528d049424466ULL, 0x11eaff4a98553d56ULL},
    {0xb988414d4203a0a3ULL, 0x1cab3210f3bb9557ULL},
    {0x6139cdd76802e6e9ULL, 0x16ef5b40c2fc7779ULL},
    {0xe761717920025254ULL, 0x125915cd68c9f92dULL},
    {0xa568b58e999d5086ULL, 0x1d5b561574765b7cULL},
    {0x5120913ee14aa6d2ULL, 0x177c44ddf6c515fdULL},
    {0xa74d40ff1aa21f0eULL, 0x12c9d0b1923744caULL},
    {0x0baece64f769cb4aULL, 0x1e0fb44f50586e11ULL},
    {0x3c8bd850c5ee3c3bULL, 0x180c903f7379f1a7ULL},
    {0xca0979da37f1c9c9ULL, 0x133d4032c2c7f485ULL},
    {0xa9a8c2f6bfe942dbULL, 0x1ec866b79e0cba6fULL},
    {0x2153cf2bccba9be3ULL, 0x18a0522c7e709526ULL},
    {0x1aa9728970954982ULL, 0x13b374f06526ddb8ULL},
    {0xf775840f1a88759dULL, 0x1f8587e7083e2f8cULL},
    {0x5f9136727ba05e17ULL, 0x19379fec0698260aULL},
    {0x1940f85b9619e4dfULL, 0x142c7ff0054684d5ULL},
    {0xe100c6afab47ea4cULL, 0x1023998cd1053710ULL},
    {0xce67a44c453fdd47ULL, 0x19d28f47b4d524e7ULL},
    {0xd852e9d69dccb106ULL, 0x14a8729fc3ddb71fULL},
    {0x79dbee454b0a2738ULL, 0x1086c219697e2c19ULL},
    {0x295fe3a211a9d859ULL, 0x1a71368f0f30468fULL},
    {0xbab31c81a7bb137aULL, 0x15275ed8d8f36ba5ULL},
    {0x6228e39aec95a92fULL, 0x10ec4be0ad8f8951ULL},
    {0x9d0e38f7e0ef7517ULL, 0x1b13ac9aaf4c0ee8ULL},
    {0xb0d82d931a592a79ULL, 0x15a956e225d67253ULL},
    {0x8d79be0f4847552eULL, 0x11544581b7dec1dcULL},
    {0x158f967eda0bbb7cULL, 0x1bba08cf8c979c94ULL},
    {0x77a611ff14d62f97ULL, 0x162e6d72d6dfb076ULL},
    {0xf951a7ff43de8c79ULL, 0x11bebdf578b2f391ULL},
    {0xc21c3ffed2fdad8eULL, 0x1c6463225ab7ec1cULL},
    {0x01b0333242648ad8ULL, 0x16b6b5b5155ff017ULL},
    {0x0159c28e9b83a246ULL, 0x122bc490dde659acULL},
    {0xcef604175f3903a3ULL, 0x1d12d41afca3c2acULL},
    {0x725e69ac4c2d9c83ULL, 0x17424348ca1c9bbdULL},
    {0xf5185489d68ae39cULL, 0x129b69070816e2fdULL},
    {0xee8d540fbdab05c6ULL, 0x1dc574d80cf16b2fULL},
    {0xbed77672fe226b05ULL, 0x17d12a4670c1228cULL},
    {0xff12c528cb4ebc04ULL, 0x130dbb6b8d674ed6ULL},
    {0xcb513b74787df9a0ULL, 0x1e7c5f127bd87e24ULL},
    {0x090dc929f9fe614dULL, 0x18637f41fcad31b7ULL},
    {0xa0d7d42194cb810aULL, 0x1382cc34ca2427c5ULL},
    {0x67bfb9cf5478ce77ULL, 0x1f37ad21436d0c6fULL},
    {0x1fcc94a5dd2d71f9ULL, 0x18f9574dcf8a7059ULL},
    {0x7fd6dd517dbdf4c7ULL, 0x13faac3e3fa1f37aULL},
    {0xffbe2ee8c92fee0bULL, 0x1ff779fd329cb8c3ULL},
    {0x6631bf20a0f324d6ULL, 0x1992c7fdc216fa36ULL},
    {0xb827cc1a1a5c1d78ULL, 0x14756ccb01abfb5eULL},
    {0x935309ae7b7ce460ULL, 0x105df0a267bcc918ULL},
    {0x1eeb42b0c594a099ULL, 0x1a2fe76a3f9474f4ULL},
    {0xe58902270476e6e1ULL, 0x14f31f8832dd2a5cULL},
    {0xb7a0ce859d2bebe7ULL, 0x10c27fa028b0eeb0ULL},
    {0x59014a6f61dfdfd8ULL, 0x1ad0cc33744e4ab4ULL},
    {0xe0cdd525e7e64cadULL, 0x1573d68f903ea229ULL},
    {0x4d7177518651d6f1ULL, 0x11297872d9cbb4eeULL},
    {0x7be8bee8d6e957e8ULL, 0x1b758d848fac54b0ULL},
    {0xfcba3253df211320ULL, 0x15f7a46a0c89dd59ULL},
    {0x63c8284318e74280ULL, 0x1192e9ee706e4aaeULL},
    {0x060d0d3827d86a66ULL, 0x1c1e43171a4a1117ULL},
    {0x6b3da42cecad21ebULL, 0x167e9c127b6e7412ULL},
    {0x88fe1cf0bd574e56ULL, 0x11fee341fc585cdbULL},
    {0x419694b462254a23ULL, 0x1ccb0536608d615fULL},
    {0x67abaa29e81dd4e9ULL, 0x1708d0f84d3de77fULL},
    {0xb95621bb2017dd87ULL, 0x126d73f9d764b932ULL},
    {0xc223692b668c95a5ULL, 0x1d7becc2f23ac1eaULL},
    {0xce82ba891ed6de1dULL, 0x179657025b6234bbULL},
    {0xa53562074bdf1818ULL, 0x12deac01e2b4f6fcULL},
    {0x3b889cd87964f359ULL, 0x1e3113363787f194ULL},
    {0xfc6d4a46c783f5e1ULL, 0x18274291c6065adcULL},
    {0x30576e9f06032b1aULL, 0x13529ba7d19eaf17ULL},
    {0x1a257dcb3cd1de90ULL, 0x1eea92a61c311825ULL},
    {0x481dfe3c30a7e540ULL, 0x18bba884e35a79b7ULL},
    {0xd34b31c9c0865100ULL, 0x13c9539d82aec7c5ULL},
    {0x5211e942cda3b4cdULL, 0x1fa885c8d117a609ULL},
    {0x74db21023e1c90a4ULL, 0x19539e3a40dfb807ULL},
    {0xf715b401cb4a0d50ULL, 0x1442e4fb67196005ULL},
    {0xf8de299b09080aa7ULL, 0x103583fc527ab337ULL},
    {0x8e304291a80cddd7ULL, 0x19ef3993b72ab859ULL},
    {0x3e8d020e200a4b13ULL, 0x14bf6142f8eef9e1ULL},
    {0x653d9b3e80083c0fULL, 0x10991a9bfa58c7e7ULL},
    {0x6ec8f864000d2ce4ULL, 0x1a8e90f9908e0ca5ULL},
    {0x8bd3f9e999a423eaULL, 0x153eda614071a3b7ULL},
    {0x3ca994bae1501cbbULL, 0x10ff151a99f482f9ULL},
    {0xc775bac49bb3612bULL, 0x1b31bb5dc320d18eULL},
    {0xd2c4956a16291a89ULL, 0x15c162b168e70e0bULL},
    {0xdbd0778811ba7ba1ULL, 0x11678227871f3e6fULL},
    {0x2c80bf401c5d929bULL, 0x1bd8d03f3e9863e6ULL},
    {0xbd33cc3349e47549ULL, 0x16470cff6546b651ULL},
    {0xca8fd68f6e505dd4ULL, 0x11d270cc51055ea7ULL},
    {0x4419574be3b3c953ULL, 0x1c83e7ad4e6efdd9ULL},
    {0x0347790982f63aa9ULL, 0x16cfec8aa52597e1ULL},
    {0xcf6c60d468c4fbbaULL, 0x123ff06eea847980ULL},
    {0xe57a34870e07f92aULL, 0x1d331a4b10d3f59aULL},
    {0x512e906c0b399422ULL, 0x175c1508da432ae2ULL},
    {0xda8ba6bcd5c7a9b5ULL, 0x12b010d3e1cf5581ULL},
    {0x90df712e22d90f87ULL, 0x1de6815302e5559cULL},
    {0xda4c5a8b4f140c6cULL, 0x17eb9aa8cf1dde16ULL},
    {0xaea37ba2a5a9a38aULL, 0x1322e220a5b17e78ULL},
    {0x7dd25f6aa2a905a9ULL, 0x1e9e369aa2b59727ULL},
    {0x97db7f888220d154ULL, 0x187e92154ef7ac1fULL},
    {0x797c6606ce80a777ULL, 0x139874ddd8c6234cULL},
    {0x8f2d700ae4010bf1ULL, 0x1f5a549627a36badULL},
    {0x0c2459a25000d65aULL, 0x191510781fb5efbeULL},
    {0x701d1481d99a4515ULL, 0x1410d9f9b2f7f2feULL},
    {0xc017439b147b6a77ULL, 0x100d7b2e28c65bfeULL},
    {0xccf205c4ed9243f2ULL, 0x19af2b7d0e0a2ccaULL},
    {0x0a5b37d0be0e9cc2ULL, 0x148c22ca71a1bd6fULL},
    {0x0848f973cb3ee3ceULL, 0x10701bd527b4978cULL},
    {0xda0e5bec78649fb0ULL, 0x1a4cf9550c5425acULL},
    {0x7b3eaff060507fc0ULL, 0x150a6110d6a9b7bdULL},
    {0x95cbbff380406633ULL, 0x10d51a73deee2c97ULL},
    {0xefac665266cd7052ULL, 0x1aee90b964b04758ULL},
    {0x2623850eb8a459dbULL, 0x158ba6fab6f36c47ULL},
    {0x1e82d0d893b6ae49ULL, 0x113c85955f29236cULL},
    {0xfd9e1af41f8ab075ULL, 0x1b9408eefea838acULL},
    {0x97b1af29b2d559f7ULL, 0x16100725988693bdULL},
    {0xac8e25baf5777b2cULL, 0x11a66c1e139edc97ULL},
    {0x7a7d092b2258c513ULL, 0x1c3d79c9b8fe2dbfULL},
    {0x61fda0ef4ead6a76ULL, 0x169794a160cb57ccULL},
    {0xe7fe1a590bbdeec5ULL, 0x1212dd4de7091309ULL},
    {0xa6635d5b45fcb13aULL, 0x1ceafbafd80e84dcULL},
    {0x851c4aaf6b308dc8ULL, 0x172262f3133ed0b0ULL},
    {0xd0e36ef2bc26d7d4ULL, 0x1281e8c275cbda26ULL},
    {0xb49f17eac6a48c86ULL, 0x1d9ca79d894629d7ULL},
    {0x2a18dfef0550706bULL, 0x17b08617a104ee46ULL},
    {0x54e0b3259dd9f389ULL, 0x12f39e794d9d8b6bULL},
    {0x87cdeb6f62f65274ULL, 0x1e5297287c2f4578ULL},
    {0xd30b22bf825ea85dULL, 0x18421286c9bf6ac6ULL},
    {0x0f3c1bcc684bb9e4ULL, 0x13680ed23aff889fULL},
    {0x18602c7a4079296dULL, 0x1f0ce4839198da98ULL},
    {0x46b356c833942124ULL, 0x18d71d360e13e213ULL},
    {0x388f78a029434db6ULL, 0x13df4a91a4dcb4dcULL},
    {0x5a7f2766a86baf8aULL, 0x1fcbaa82a1612160ULL},
    {0x153285ebb9efbfa2ULL, 0x196fbb9bb44db44dULL},
    {0xaa8ed189618c994eULL, 0x145962e2f6a4903dULL},
    {0xeed8a7a11ad6e10cULL, 0x1047824f2bb6d9caULL},
    {0x7e27729b5e249b45ULL, 0x1a0c03b1df8af611ULL},
    {0xfe85f549181d4904ULL, 0x14d6695b193bf80dULL},
    {0xcb9e5dd4134aa0d0ULL, 0x10ab877c142ff9a4ULL},
    {0xdf63c9535211014dULL, 0x1aac0bf9b9e65c3aULL},
    {0x191ca10f74da6771ULL, 0x15566ffafb1eb02fULL},
    {0xadb080d92a4852c1ULL, 0x1111f32f2f4bc025ULL},
    {0x15e7348eaa0d5134ULL, 0x1b4feb7eb212cd09ULL},
    {0xab1f5d3eee710dc4ULL, 0x15d98932280f0a6dULL},
    {0xbc1917658b8da49dULL, 0x117ad428200c0857ULL},
    {0x2cf4f23c127c3a94ULL, 0x1bf7b9d9cce00d59ULL},
    {0xf0c3f4fcdb969543ULL, 0x165fc7e170b33de0ULL},
    {0x5a365d9716121103ULL, 0x11e6398126f5cb1aULL},
    {0x9056fc24f01ce804ULL, 0x1ca38f350b22de90ULL},
    {0xd9df301d8ce3ecd0ULL, 0x16e93f5da2824ba6ULL},
    {0xe17f59b13d8323daULL, 0x125432b14ecea2ebULL},
    {0x68cbc2b52f38395cULL, 0x1d53844ee47dd179ULL},
    {0x53d6355dbf602de3ULL, 0x177603725064a794ULL},
    {0xa9782ab165e68b1cULL, 0x12c4cf8ea6b6ec76ULL},
    {0x0f26aab56fd744faULL, 0x1e07b27dd78b13f1ULL},
    {0x3f52222abfdf6a62ULL, 0x18062864ac6f4327ULL},
    {0x65db4e88997f884eULL, 0x1338205089f29c1fULL},
    {0x6fc54a7428cc0d4aULL, 0x1ec033b40fea9365ULL},
    {0x596aa1f68709a43bULL, 0x1899c2f673220f84ULL},
    {0xadeee7f86c07b696ULL, 0x13ae3591f5b4d936ULL},
    {0x497e3ff3e00c5756ULL, 0x1f7d228322baf524ULL},
    {0xd464fff64cd6ac45ULL, 0x1930e868e89590e9ULL},
    {0x4383fff83d7889d1ULL, 0x14272053ed4473eeULL},
    {0xcf9cccc69793a174ULL, 0x101f4d0ff1038ff1ULL},
    {0x7f6147a425b90252ULL, 0x19cbae7fe805b31cULL},
    {0xcc4dd2e9b7c7350fULL, 0x14a2f1ffecd15c16ULL},
    {0x3d0b0f215fd290d9ULL, 0x10825b3323dab012ULL},
    {0x61ab4b689950e7c1ULL, 0x1a6a2b85062ab350ULL},
    {0x4e22a2ba1440b967ULL, 0x1521bc6a6b555c40ULL},
    {0x0b4ee894dd009453ULL, 0x10e7c9eebc4449cdULL},
    {0x1217da87c800ed51ULL, 0x1b0c764ac6d3a948ULL},
    {0xdb46486ca000bddaULL, 0x15a391d56bdc876cULL},
    {0x490506bd4ccd64afULL, 0x114fa7ddefe39f8aULL},
    {0xa8080ac87ae23ab1ULL, 0x1bb2a62fe638ff43ULL},
    {0x5339a239fbe82ef4ULL, 0x162884f31e93ff69ULL},
    {0x75c7b4fb2fecf25dULL, 0x11ba03f5b20fff87ULL},
    {0x22d92191e647ea2eULL, 0x1c5cd322b67fff3fULL},
    {0xb57a8141850654f2ULL, 0x16b0a8e891ffff65ULL},
    {0xc4620101373843f5ULL, 0x1226ed86db3332b7ULL},
    {0x3a366801f1f39feeULL, 0x1d0b15a491eb8459ULL},
    {0xfb5eb99b27f6198bULL, 0x173c115074bc69e0ULL},
    {0x2f7efae2865e7ad6ULL, 0x129674405d6387e7ULL},
    {0xe597f7d0d6fd9156ULL, 0x1dbd86cd6238d971ULL},
    {0x8479930d78cadaabULL, 0x17cad23de82d7ac1ULL},
    {0xd06142712d6f1556ULL, 0x1308a831868ac89aULL},
    {0x4d686a4eaf182222ULL, 0x1e74404f3daada91ULL},
    {0xa453883ef279b4e8ULL, 0x185d003f6488aedaULL},
    {0xe9dc6cff28615d87ULL, 0x137d99cc506d58aeULL},
    {0xa960ae650d6895a4ULL, 0x1f2f5c7a1a488de4ULL},
    {0xbab3beb73ded4483ULL, 0x18f2b061aea07183ULL},
    {0x2ef6322c318a9d36ULL, 0x13f559e7bee6c136ULL},
};

static inline uint64_t ryu_mulshift64(uint64_t m, const uint64_t* mul, int j) {
    __uint128_t b0 = (__uint128_t)m * mul[0];
    __uint128_t b2 = (__uint128_t)m * mul[1];
    return (uint64_t)(((b0 >> 64) + b2) >> (j - 64));
}

static inline int ryu_pow5bits(int e) { return (int)(((uint32_t)e * 1217359) >> 19) + 1; }
static inline uint32_t ryu_log10pow2(int e) { return ((uint32_t)e * 78913) >> 18; }
static inline uint32_t ryu_log10pow5(int e) { return ((uint32_t)e * 732923) >> 20; }

static inline int ryu_pow5factor(uint64_t v) {
    int count = 0;
    for (;;) {
        uint64_t q = v / 5;
        if (q * 5 != v) return count;
        v = q;
        count++;
    }
}
static inline int ryu_multiple_of_pow5(uint64_t v, int p) { return ryu_pow5factor(v) >= p; }
static inline int ryu_multiple_of_pow2(uint64_t v, int p) {
    return (v & ((1ULL << p) - 1)) == 0;
}

/* the digits of `output`, most significant first, two at a time; returns
   the count */
static inline int ryu_write(uint64_t output, char* dig) {
    int n = ryu_declen(output);
    int a = n;
    while (output >= 100) {
        uint32_t c = (uint32_t)(output % 100);
        output /= 100;
        a -= 2;
        __builtin_memcpy(dig + a, RYU_DIGITS + c * 2, 2);
    }
    if (output >= 10) __builtin_memcpy(dig, RYU_DIGITS + (uint32_t)output * 2, 2);
    else dig[0] = (char)('0' + (uint32_t)output);
    dig[n] = 0;
    return n;
}

/* Exact powers of ten, as doubles: every one through 1e22 is representable,
   and the short path below reads the first sixteen. */
static const double RYU_POW10D[16] = {
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7,
    1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
};
static const uint64_t RYU_POW10U[16] = {
    1ULL, 10ULL, 100ULL, 1000ULL, 10000ULL, 100000ULL, 1000000ULL, 10000000ULL,
    100000000ULL, 1000000000ULL, 10000000000ULL, 100000000000ULL,
    1000000000000ULL, 10000000000000ULL, 100000000000000ULL, 1000000000000000ULL,
};

/* A SHORT DECIMAL IS FOUND BY SCALING, and proved by dividing back.
   Most floats a program writes down have few digits -- every one in
   the encode corpus has seven or fewer -- and for those the search
   below replaces the 125-bit multiplies and the digit removal with a
   multiply and a division per decimal place.

   Let u be the gap to the next double up, so the interval of decimals
   that read back as f is at most u wide. While u * 10^p <= 1/4:
     - at most one decimal with p places lies in it, since the scaled
       interval is narrower than the gap between integers;
     - that decimal, times 10^p, is within 1/8 of f * 10^p exactly,
       and the rounded product is within 1/4 of that, so rounding the
       product finds it;
     - m / 10^p is one correctly rounded division of two exact
       doubles, so it equals f exactly when m * 10^-p reads back as f
       under the same round-half-even strtod uses.
   So the first p that passes gives the one decimal with the fewest
   places. It is also the one ryu picks: a shorter candidate would sit
   at a larger power of ten inside the same interval, and that power
   of ten is itself a one-digit candidate at a place already tried.
   When the bound fails before a place passes, `ryu_long` decides.

   The range keeps every scaled product below 2^51 and every
   candidate a normal double: f in [2^-20, 2^50). `limit` is
   1/(4u), a power of two built from the exponent directly. */
/* One place count tried: whether f * 10^p, rounded, divides back to f. */
static inline int ryu_short_at(double f, int p, int64_t* mo) {
    /* signed, because every product is below 2^51 and a signed
       conversion is one instruction each way on x86-64, where an
       unsigned one was a dozen */
    double y = f * RYU_POW10D[p];
    int64_t m = (int64_t)y;
    m += y - (double)m >= 0.5;
    *mo = m;
    return (double)m / RYU_POW10D[p] == f;
}

/* The place count the last short decimal took. */
static int k_ryu_places = 0;

static inline int ryu_short(double f, int64_t* mo, int* po) {
    uint64_t bits;
    __builtin_memcpy(&bits, &f, 8);
    uint32_t ieee_e = (uint32_t)(bits >> 52) & 0x7FF;
    if ((uint32_t)(ieee_e - 1003) >= 70) return 0;
    uint64_t lbits = (uint64_t)(2096 - ieee_e) << 52;
    double limit;
    __builtin_memcpy(&limit, &lbits, 8);
    /* THE SEARCH STARTS WHERE THE LAST ONE ENDED. Passing is monotone in p
       while the bound holds: a decimal with p places is also one with p + 1,
       it lies in the same interval, and rounding finds it there too. So a
       pass at the guess means the answer is at or below it, and a failure
       means it is above; either way the walk stops at the first place where
       the answer changes, which is the fewest places, as before. Floats in
       one document tend to share a precision: on runbench 170,820 of
       191,070 take four places, and starting from zero tried five places
       for each of them where starting from the last answer tries two. */
    int p = k_ryu_places;
    int64_t m;
    if (RYU_POW10D[p] > limit) p = 0;
    if (ryu_short_at(f, p, &m)) {
        int64_t below;
        while (p > 0 && ryu_short_at(f, p - 1, &below)) {
            p--;
            m = below;
        }
    } else {
        for (p++; ; p++) {
            if (p >= 16 || RYU_POW10D[p] > limit) return 0;
            if (ryu_short_at(f, p, &m)) break;
        }
    }
    if (K_COUNTING) k_stat_ryu_short++;
    k_ryu_places = p;
    *mo = m;
    *po = p;
    return 1;
}

/* The short decimal m * 10^-p as ryu's digits: an integer's trailing zeros
   come off into the exponent, and a fraction has none, since it would then
   have had fewer places. */
static inline int ryu_short_digits(int64_t m, int p, char* dig, int* e10) {
    int z = 0;
    if (p == 0) {
        while (m % 10 == 0) { m /= 10; z++; }
    }
    *e10 = z - p;
    return ryu_write((uint64_t)m, dig);
}

/* ryu itself, for the floats the short path leaves: shortest digits and
   decimal exponent of a positive finite double; returns the digit count,
   digits in dig[], value = dig * 10^*e10 */
static __attribute__((noinline, cold)) int ryu_long(double f, char* dig, int* e10) {
    uint64_t bits;
    __builtin_memcpy(&bits, &f, 8);
    uint64_t ieee_m = bits & ((1ULL << 52) - 1);
    uint32_t ieee_e = (uint32_t)(bits >> 52) & 0x7FF;
    int e2;
    uint64_t m2;
    /* Ruled 2026-09-08: `inf`, `-inf`, `nan`. Both are the all-ones exponent
       the line above isolated, and the mantissa says which; answering zero
       digits is the signal and dig[0] carries the word.
       The test rides the arm that was already here. `ieee_e - 1` wraps for
       zero, so one unsigned compare catches BOTH ends of the field — the
       subnormals this branch existed for, and the two encodings above — and
       the ordinary render pays exactly the compare it paid before. Asked its
       own way it is not free: in front of ryu, as a second load of the same
       eight bytes, it measured 1,049,580 instructions of runbench, and as its
       own compare here 572,400. */
    if (__builtin_expect((uint32_t)(ieee_e - 1) >= 0x7FE, 0)) {
        if (ieee_e == 0x7FF) {
            dig[0] = ieee_m ? 'n' : 'i';
            return 0;
        }
        e2 = 1 - 1023 - 52 - 2;
        m2 = ieee_m;
    } else {
        e2 = (int)ieee_e - 1023 - 52 - 2;
        m2 = (1ULL << 52) | ieee_m;
    }
    int even = (m2 & 1) == 0;
    int accept = even;

    uint64_t mv = 4 * m2;
    uint32_t mm_shift = ieee_m != 0 || ieee_e <= 1;

    uint64_t vr, vp, vm;
    int e10v;
    int vm_trailing = 0, vr_trailing = 0;
    uint8_t last_removed = 0;
    if (e2 >= 0) {
        uint32_t q = ryu_log10pow2(e2) - (e2 > 3);
        e10v = (int)q;
        int k = RYU_POW5_INV_BITCOUNT + ryu_pow5bits((int)q) - 1;
        int i = -e2 + (int)q + k;
        vr = ryu_mulshift64(mv, RYU_POW5_INV[q], i);
        vp = ryu_mulshift64(mv + 2, RYU_POW5_INV[q], i);
        vm = ryu_mulshift64(mv - 1 - mm_shift, RYU_POW5_INV[q], i);
        if (q <= 21) {
            if (mv % 5 == 0) vr_trailing = ryu_multiple_of_pow5(mv, (int)q);
            else if (accept) vm_trailing = ryu_multiple_of_pow5(mv - 1 - mm_shift, (int)q);
            else vp -= ryu_multiple_of_pow5(mv + 2, (int)q);
        }
    } else {
        uint32_t q = ryu_log10pow5(-e2) - (-e2 > 1);
        e10v = (int)q + e2;
        int i = -e2 - (int)q;
        int k = ryu_pow5bits(i) - RYU_POW5_BITCOUNT;
        int j = (int)q - k;
        vr = ryu_mulshift64(mv, RYU_POW5[i], j);
        vp = ryu_mulshift64(mv + 2, RYU_POW5[i], j);
        vm = ryu_mulshift64(mv - 1 - mm_shift, RYU_POW5[i], j);
        if (q <= 1) {
            vr_trailing = 1;
            if (accept) vm_trailing = mm_shift == 1;
            else vp -= 1;
        } else if (q < 63) {
            vr_trailing = ryu_multiple_of_pow2(mv, (int)q);
        }
    }

    int removed = 0;
    uint64_t output;
    if (vm_trailing || vr_trailing) {
        for (;;) {
            uint64_t vpd = vp / 10, vmd = vm / 10;
            if (vpd <= vmd) break;
            uint32_t vmm = (uint32_t)(vm % 10);
            uint64_t vrd = vr / 10;
            uint32_t vrm = (uint32_t)(vr % 10);
            vm_trailing &= vmm == 0;
            vr_trailing &= last_removed == 0;
            last_removed = (uint8_t)vrm;
            vr = vrd; vp = vpd; vm = vmd;
            removed++;
        }
        if (vm_trailing) {
            for (;;) {
                uint64_t vmd = vm / 10;
                uint32_t vmm = (uint32_t)(vm % 10);
                if (vmm != 0) break;
                uint64_t vpd = vp / 10;
                uint64_t vrd = vr / 10;
                uint32_t vrm = (uint32_t)(vr % 10);
                vr_trailing &= last_removed == 0;
                last_removed = (uint8_t)vrm;
                vr = vrd; vp = vpd; vm = vmd;
                removed++;
            }
        }
        if (vr_trailing && last_removed == 5 && vr % 2 == 0) {
            last_removed = 4;
        }
        output = vr + ((vr == vm && (!accept || !vm_trailing)) || last_removed >= 5);
    } else {
        int round_up = 0;
        /* Two digits a trip while two are there to take. This loop used to
           run once and hand the rest to the ten-loop below, and the ten-loop
           was averaging 9.41 trips a float on the encode corpus -- because a
           float a program writes down has few significant digits and `vr`
           starts with seventeen, so most of them come off. Both loops cost
           the same sixteen instructions a trip (three multiply-highs, three
           shifts, a compare and the branch), so a trip that takes two digits
           is worth two that take one. */
        /* TWO DIVISIONS A TRIP, NOT THREE. The loop only needs `vp` and `vm`
           to decide whether another pair comes off; `vr` is carried through
           it and read once at the end. Three divide-by-hundreds a trip cost
           twenty-one instructions on this box, of which seven are moves
           shuttling the three values around the back edge -- the 2026-09-14
           entry named those seven and left them. Dropping `vr` out of the
           loop takes one division and its pair of moves with it, and the one
           variable division at the bottom is paid once a float rather than
           5.35 times.

           `round_up` is the top digit of the LAST pair removed, which is
           `(vr / 100^(pairs-1)) % 100 >= 50` -- the same value the last trip
           of the old loop wrote. */
        int pairs = 0;
        for (;;) {
            uint64_t vpd100 = vp / 100, vmd100 = vm / 100;
            if (vpd100 <= vmd100) break;
            vp = vpd100; vm = vmd100;
            pairs++;
        }
        if (pairs) {
            /* ONE variable division, not two. `RYU_POW100[pairs]` is
               `RYU_POW100[pairs - 1] * 100`, so dividing by the smaller one
               first leaves both remaining steps with a CONSTANT divisor, and
               a constant divisor is a multiply-high rather than a `div`. The
               first shape of this step divided by both table entries and gave
               back most of what the loop saved. */
            uint64_t q = vr / RYU_POW100[pairs - 1];
            round_up = (uint32_t)(q % 100) >= 50;
            vr = q / 100;
            removed += 2 * pairs;
        }
        for (;;) {
            uint64_t vpd = vp / 10, vmd = vm / 10;
            if (vpd <= vmd) break;
            uint64_t vrd = vr / 10;
            uint32_t vrm = (uint32_t)(vr % 10);
            round_up = vrm >= 5;
            vr = vrd; vp = vpd; vm = vmd;
            removed++;
        }
        output = vr + (vr == vm || round_up);
    }
    *e10 = e10v + removed;
    return ryu_write(output, dig);
}

/* shortest digits + decimal exponent for a positive finite double; returns
   digit count, digits in dig[], value = dig * 10^*e10 */
static int ryu_d2d(double f, char* dig, int* e10) {
    int64_t m;
    int p;
    if (ryu_short(f, &m, &p)) return ryu_short_digits(m, p, dig, e10);
    return ryu_long(f, dig, e10);
}

/* format (digits, k, e10) exactly as the probe would have: %g with
   precision max(15, k) — fixed vs exponent at X < -4 or X >= P */
/* Returns the length written, for the reason k_itoa's does. */
static long long render_ryu(double d, char* buf) {
    if (d == 0.0) {
        char* z = buf;
        if (signbit(d)) *z++ = '-';
        z[0] = '0'; z[1] = '.'; z[2] = '0'; z[3] = 0;
        return (long long)(z - buf) + 3;
    }
    char dig[20];
    int e10;
    int k;
    int64_t sm;
    int sp;
    if (ryu_short(d, &sm, &sp)) {
        /* THE SHORT DECIMAL IS WRITTEN FROM m AND p, without the digit
           buffer. Below 10^15 and from 10^-4 up, the text is the plain form
           below with P = 15: m / 10^p, then when p > 0 a point and m's last
           p digits, zeros kept in front. That is what the plain branches
           write from ryu's digits, and they walked those digits a byte at a
           time to place the point. */
        uint64_t m = (uint64_t)sm;
        uint64_t pw = RYU_POW10U[sp];
        if (m < 1000000000000000ULL && m * 10000 >= pw) {
            uint64_t whole = m / pw, part = m - whole * pw;
            char* o = buf + ryu_write(whole, buf);
            if (sp) {
                *o++ = '.';
                char* w = o + sp;
                o = w;
                int left = sp;
                while (left >= 2) {
                    uint32_t c = (uint32_t)(part % 100);
                    part /= 100;
                    w -= 2;
                    __builtin_memcpy(w, RYU_DIGITS + c * 2, 2);
                    left -= 2;
                }
                if (left) *--w = (char)('0' + (uint32_t)part);
            }
            *o = 0;
            return (long long)(o - buf);
        }
        k = ryu_short_digits(sm, sp, dig, &e10);
    } else {
        k = ryu_long(d, dig, &e10);
    }
    if (k == 0) {
        /* The digit core read an all-ones exponent. Any sign is already
           written: the caller sends -inf here as +inf behind a minus, and
           nan never takes that branch. */
        if (dig[0] == 'n') { buf[0] = 'n'; buf[1] = 'a'; buf[2] = 'n'; }
        else { buf[0] = 'i'; buf[1] = 'n'; buf[2] = 'f'; }
        buf[3] = 0;
        return 3;
    }
    int x = e10 + k - 1; /* decimal exponent of the leading digit */
    int p = k > 15 ? k : 15;
    char* o = buf;
    if (x < -4 || x >= p) {
        *o++ = dig[0];
        *o++ = '.';
        if (k > 1) {
            k_copy_short(o, dig + 1, k - 1); o += k - 1;
        } else {
            *o++ = '0';
        }
        *o++ = 'e';
        int ex = x;
        *o++ = ex < 0 ? '-' : '+';
        if (ex < 0) ex = -ex;
        if (ex >= 100) { *o++ = (char)('0' + ex / 100); ex %= 100; *o++ = (char)('0' + ex / 10); }
        else { *o++ = (char)('0' + ex / 10); }
        *o++ = (char)('0' + ex % 10);
        *o = 0;
    } else if (x >= 0) {
        int ip = x + 1; /* digits before the point */
        for (int i = 0; i < ip; i++) *o++ = i < k ? dig[i] : '0';
        if (k > ip) {
            *o++ = '.';
            k_copy_short(o, dig + ip, k - ip); o += k - ip;
        }
        *o = 0;
    } else {
        *o++ = '0'; *o++ = '.';
        for (int i = 0; i < -x - 1; i++) *o++ = '0';
        k_copy_short(o, dig, k); o += k;
        *o = 0;
    }
    return (long long)(o - buf);
}


static KValue k_sub_base(KValue v) {
    while (v.tag == K_SUB) v = ((KSub*)(intptr_t)v.payload)->inner;
    return v;
}

/* A builtin sees a subtype's value as its parent's, as the oracle's
   call_builtin does: `length` of a `type name string` counts the string. The
   emitter routes a builtin's arguments through this only in a program that
   declares a subtype, so no other program pays for it. */
KValue k_unsub(KValue v) {
    return v.tag == K_SUB ? k_sub_base(v) : v;
}

/* Whether an operand sends an operator to its user arms. A record does, and
   so does a subtype of one, which is the same value wearing a narrower name. */
long long k_routes_to_arms(KValue v) {
    /* a subtype is user-owned whatever it wraps, so an arm written for
       `type money int` is reachable where the int's builtin would otherwise
       answer first and the arm would never run */
    if (v.tag == K_SUB) return 1;
    return v.tag == K_REC;
}

KValue k_sub_wrap(long long type_id, KValue inner) {
    if (!k_not_failure(inner)) return inner;
    KSub* s = k_alloc(sizeof(KSub));
    s->type_id = type_id;
    s->inner = inner;
    KValue v; v.tag = K_SUB; v.payload = k_ptr(s); return v;
}

/* Two parents that are not one tag: `bool` is either of two, and `some` is
   any value that is neither none nor a failure. They sit below every
   -(tag + 1), which is what the other primitives are. */
#define K_WANT_BOOL (-100)
#define K_WANT_SOME (-101)

/* Whether a value that wraps nothing is the primitive a negative want
   names. */
static int k_want_prim(KValue v, long long want_id) {
    if (want_id == K_WANT_BOOL) return v.tag == K_TRUE || v.tag == K_FALSE;
    if (want_id == K_WANT_SOME) return v.tag != K_NONE && k_not_failure(v);
    if (want_id == -(K_INT + 1)) return K_IS_INT(v);
    return v.tag == -(want_id + 1);
}

/* Chain match for dispatch: 0 = exact, +1 per parent hop; -1 = no match.
   want_id >= 0 names a declared type; want_id < 0 encodes a primitive tag
   as -(tag + 1), so annotated primitive params accept wrapped values. */
long long k_sub_depth(KValue v, long long want_id) {
    long long depth = 0;
    while (v.tag == K_SUB) {
        KSub* s = (KSub*)(intptr_t)v.payload;
        if (want_id >= 0 && s->type_id == want_id) return depth;
        v = s->inner;
        depth++;
    }
    if (want_id < 0 && k_want_prim(v, want_id)) return depth;
    if (want_id >= 0 && v.tag == K_REC
        && ((KRec*)(intptr_t)v.payload)->type_id == want_id) return depth;
    return -1;
}

/* Dispatch checks for programs that declare subtypes: the annotated
   param accepts a wrapped value whose chain reaches the annotation. */
long long k_check_sub_tag(KValue v, long long tag) {
    return k_sub_depth(v, -(tag + 1)) >= 0;
}

long long k_check_sub_bool(KValue v) {
    KValue b = k_sub_base(v);
    return b.tag == K_TRUE || b.tag == K_FALSE;
}

long long k_check_sub_id(KValue v, long long type_id) {
    return k_sub_depth(v, type_id) >= 0;
}

long long k_check_sub_rec(KValue v, long long type_id, long long nfields) {
    KValue b = k_sub_base(v);
    if (b.tag != K_REC) return 0;
    KRec* r = (KRec*)(intptr_t)b.payload;
    return r->type_id == type_id && r->nfields == nfields;
}

/* "an int", "a string" — a diagnostic that fumbles its own grammar reads as
   carelessness about everything else in it. The rule is spelling, not
   pronunciation: a type name is a program's word rather than an English one,
   and the letter is what a reader has in front of them. `article` in diag.rs
   is the same rule for the other two engines. */
static const char* k_article(const char* word) {
    switch (word[0]) {
        case 'a': case 'e': case 'i': case 'o': case 'u': return "an";
        default: return "a";
    }
}

/* Construction: `post_body s` — the inner value must be (or reach) the
   parent; want encodes a type id, or -(tag+1) for a primitive. */
KValue k_sub_ctor(long long type_id, long long want, KValue inner, const char* tyname, const char* parent) {
    if (!k_not_failure(inner)) return inner;
    if (k_sub_depth(inner, want) < 0) {
        fprintf(stderr, "%serror[runtime]:%s `%s` wraps %s %s\n", k_c_err(), k_c_off(), tyname,
                k_article(parent), parent);
        exit(1);
    }
    return k_sub_wrap(type_id, inner);
}

/* The upcast: strip to the named ancestor; widening only. */
KValue k_upcast(KValue v, long long want, const char* tyname) {
    KValue cur = v;
    if (!k_not_failure(cur)) return cur;
    for (;;) {
        if (cur.tag == K_SUB) {
            KSub* sb = (KSub*)(intptr_t)cur.payload;
            if (want >= 0 && sb->type_id == want) return cur;
            cur = sb->inner;
            continue;
        }
        if (want < 0 && k_want_prim(cur, want)) return cur;
        if (want >= 0 && cur.tag == K_REC
            && ((KRec*)(intptr_t)cur.payload)->type_id == want) return cur;
        fprintf(stderr, "%serror[runtime]:%s `:%s` widens; this value is not %s %s\n",
                k_c_err(), k_c_off(), tyname, k_article(tyname), tyname);
        exit(1);
    }
}

#define K_RENDER_PATH_MAX 4096
static const void* k_render_path[K_RENDER_PATH_MAX];
static int k_render_depth = 0;

/* Which values can reach a user's `render/to_string` arm. A compiled call
   site knows its argument's set and decides this at compile time; `print`
   handed over as a value does not, so the decision moves here. Coherence
   licenses the rest: no arm can exist for a primitive. */
long long k_render_dispatchable(KValue v) {
    return v.tag == K_REC || v.tag == K_NONE || v.tag == K_DESC || v.tag == K_SUB;
}

/* Rendering a value that IS a failure hands the failure back, because the
   interpolation asked for a failing value and infectiousness carries it. That
   is a property of the top of a render, not of every err it reaches: an err
   held INSIDE a list or a record is a thing the container holds, and the
   switch below renders it `err <reason>` the way the oracle does.

   The two used to be one function, and the early return shadowed the switch
   case, so printing a list of results handed the merged failure to the
   endpoint and took the successes with it. Native was alone in that: the
   interpreter and wasm both answer through `eval::render`, which has only the
   nested case. */
/* The digits of an int or a float, written into `buf` (64 bytes is enough
   for either) with the length returned and no string built. k_render_at is
   one reader; `append acc "{n}"` is the other, since 2026-09-09, and it
   copies the digits into the accumulator straight from here rather than
   through a KStr it would drop on the next line. */
static inline __attribute__((always_inline)) long long k_render_number(KValue v, char* buf) {
    long long nlen;
    switch (v.tag) {
        case K_INT:
            nlen = k_itoa(buf, v.payload);
            break;
        case K_FLOAT: {
            double d = k_as_f(v);
            if (d == floor(d) && fabs(d) < 1e15 && isfinite(d)) {
                /* An integral double under 1e15 casts exactly, so its digits
                   are an integer's digits and the fraction is exactly `.0`.
                   glibc's `%.1f` reaches __printf_fp for them anyway, which
                   is a multiprecision routine: it was 11.0% of the wide
                   benchmark. Negative zero is the one value the cast loses,
                   and `%.1f` writes `-0.0` for it. */
                char* o = buf;
                long long whole = (long long)d;
                if (whole == 0 && signbit(d)) *o++ = '-';
                o += k_itoa(o, whole);
                *o++ = '.';
                *o++ = '0';
                *o = 0;
                nlen = (long long)(o - buf);
                break;
            }
            /* shortest round-trip: %g trims trailing zeros, so probing
               15..17 yields byte-identical strings to probing 1..17 — a
               double never needs more, and rarely fewer, than 15 digits */
            /* the ryu digit core computes the true shortest round-trip
               representation directly — no probing, no dtoa */
            if (K_COUNTING) k_stat_ryu_renders++;
            if (d < 0) {
                /* Straight into the caller's buffer behind the sign. The
                   scratch this replaced was 63 bytes and a strcpy of the whole
                   rendering, paid on 430,400 of encodebench's 849,200 renders.
                   buf is 64 and render_ryu writes at most the 63 the scratch
                   held, so the byte the sign takes is the byte that was
                   spare. */
                buf[0] = '-';
                nlen = 1 + render_ryu(-d, buf + 1);
            } else {
                nlen = render_ryu(d, buf);
            }
            break;
        }
        default:
            k_die("k_render_number: not a number");
    }
    return nlen;
}

KValue k_render(KValue v, long long quote) { return k_render_at(v, quote, 0); }

/* A rendering is written into one buffer and made a string once. The
   containers used to join each element's text onto everything before it, and
   a join copies both halves into a fresh string, so a list's text was copied
   once per element and left behind in the arena each time: 4,000 small ints
   allocated 32,294,448 bytes to print 7,999, and a list of 20,000 maps
   outgrew the machine. The interpreter builds one String. The buffer is
   malloc'd and freed here, so a force inside the walk cannot rewind it. */
typedef struct { char* p; long long len; long long cap; } KRenderBuf;

static void k_rb_put(KRenderBuf* b, const char* s, long long n) {
    if (b->len + n > b->cap) {
        long long cap = b->cap ? b->cap : 64;
        while (cap < b->len + n) cap *= 2;
        char* p = realloc(b->p, (size_t)cap);
        if (!p) { fputs("out of memory\n", stderr); exit(1); }
        b->p = p;
        b->cap = cap;
    }
    memcpy(b->p + b->len, s, (size_t)n);
    b->len += n;
}

static void k_rb_lit(KRenderBuf* b, const char* s) { k_rb_put(b, s, (long long)strlen(s)); }

/* Everything k_render_at answers for a value inside a container, in the same
   order and with the same cycle path, appended rather than returned. */
static void k_render_into(KRenderBuf* b, KValue v, long long quote) {
    if (v.tag == K_SUB) { k_render_into(b, k_sub_base(v), quote); return; }
    if (v.tag == K_THUNK) {
        KThunk* cell = (KThunk*)(intptr_t)v.payload;
        for (int d = 0; d < k_render_depth; d++)
            if (k_render_path[d] == cell) { k_rb_lit(b, "<cycle>"); return; }
        if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = cell;
        k_render_into(b, k_force(v), quote);
        if (k_render_depth > 0) k_render_depth--;
        return;
    }
    char buf[64];
    switch (v.tag) {
        case K_INT:
        case K_FLOAT:
            k_rb_put(b, buf, k_render_number(v, buf));
            return;
        case K_BIG: {
            KStr* st = k_as_str(k_big_render(v));
            k_rb_put(b, st->data, st->len);
            return;
        }
        case K_TRUE: k_rb_lit(b, "true"); return;
        case K_FALSE: k_rb_lit(b, "false"); return;
        case K_NONE: k_rb_lit(b, "<none>"); return;
        case K_DONE: k_rb_lit(b, "<done>"); return;
        case K_ERR:
            k_rb_lit(b, "err ");
            k_render_into(b, k_err_inner(v), 1);
            return;
        case K_STR: {
            KStr* s = k_as_str(v);
            if (quote) k_rb_lit(b, "\"");
            k_rb_put(b, s->data, s->len);
            if (quote) k_rb_lit(b, "\"");
            return;
        }
        case K_REC: {
            KRec* r = k_as_rec(v);
            if (r->nfields > 0) {
                for (int d = 0; d < k_render_depth; d++)
                    if (k_render_path[d] == r) { k_rb_lit(b, "<cycle>"); return; }
                if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = r;
                k_rb_lit(b, k_type_shown(r->type_id));
                for (long long i = 0; i < r->nfields; i++) {
                    k_rb_lit(b, " ");
                    k_render_into(b, r->fields[i], 1);
                }
                k_render_depth--;
                return;
            }
            k_rb_lit(b, k_type_shown(r->type_id));
            return;
        }
        case K_DESC: k_rb_lit(b, "<io>"); return;
        case K_LIST: {
            KList* l = (KList*)(intptr_t)v.payload;
            for (int d = 0; d < k_render_depth; d++)
                if (k_render_path[d] == l) { k_rb_lit(b, "<cycle>"); return; }
            if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = l;
            k_rb_lit(b, "[");
            for (long long i = 0; i < l->len; i++) {
                if (i) k_rb_lit(b, " ");
                k_render_into(b, l->items[i], 1);
            }
            if (k_render_depth > 0) k_render_depth--;
            k_rb_lit(b, "]");
            return;
        }
        case K_MAP: {
            KMap* m = (KMap*)(intptr_t)v.payload;
            long long n;
            KValue* s = k_map_sorted(m, &n);
            if (n == 0) { k_rb_lit(b, "{}"); return; }
            for (int d = 0; d < k_render_depth; d++)
                if (k_render_path[d] == m) { k_rb_lit(b, "<cycle>"); return; }
            if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = m;
            k_rb_lit(b, "{ ");
            for (long long i = 0; i < n; i++) {
                if (i) k_rb_lit(b, " ");
                k_render_into(b, s[i * 2], 1);
                k_rb_lit(b, ":");
                k_render_into(b, s[i * 2 + 1], 1);
            }
            if (k_render_depth > 0) k_render_depth--;
            k_rb_lit(b, " }");
            return;
        }
        case K_BYTES: {
            KBytes* by = (KBytes*)(intptr_t)v.payload;
            k_rb_lit(b, "[");
            for (long long i = 0; i < by->len; i++) {
                if (i) k_rb_lit(b, " ");
                k_rb_put(b, buf, k_itoa(buf, (long long)by->data[i]));
            }
            k_rb_lit(b, "]");
            return;
        }
        case K_CLOSURE: case K_FNREF: k_rb_lit(b, "<fn>"); return;
        default: k_rb_lit(b, "<value>"); return;
    }
}

/* A container's whole rendering, made a string once. */
static __attribute__((noinline)) KValue k_render_whole(KValue v) {
    KRenderBuf b = {0, 0, 0};
    k_render_into(&b, v, 1);
    KValue out = k_str_n(b.p, b.len);
    free(b.p);
    return out;
}


static KValue k_render_at(KValue v, long long quote, int held) {
    if (v.tag == K_ERR && !held) return v;
    if (v.tag == K_SUB) return k_render_at(k_sub_base(v), quote, held);
    /* Output is the demand, so rendering forces what it prints. The oracle
       answers the same way. */
    if (v.tag == K_THUNK) {
        KThunk* cell = (KThunk*)(intptr_t)v.payload;
        for (int d = 0; d < k_render_depth; d++)
            if (k_render_path[d] == cell) return k_str("<cycle>");
        if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = cell;
        /* the wire is the demand: output forces what it prints, and the path
           above is what stops a knot walking round */
        KValue out = k_render_at(k_force(v), quote, held);
        if (k_render_depth > 0) k_render_depth--;
        return out;
    }
    char buf[64];
    /* The two number arms write their digits into `buf` and set `nlen`, then
       leave the switch by the one exit below. They shared a `k_str(buf)` per
       return until the length came back from the writers; five k_str_n bodies
       inlined there is 1,264 bytes of .text where one is 253. */
    long long nlen;
    switch (v.tag) {
        case K_INT:
        case K_FLOAT:
            nlen = k_render_number(v, buf);
            break;
        case K_BIG: return k_big_render(v);
        case K_TRUE: return k_str("true");
        case K_FALSE: return k_str("false");
        case K_NONE: return k_str("<none>");
        case K_DONE: return k_str("<done>");
        case K_ERR: return k_render_whole(v);
        case K_STR:
            if (!quote) return v;
            return k_concat(k_concat(k_str("\""), v), k_str("\""));
        case K_REC:
            if (k_as_rec(v)->nfields == 0) return k_str(k_type_shown(k_as_rec(v)->type_id));
            return k_render_whole(v);
        case K_DESC: return k_str("<io>");
        case K_LIST:
        case K_MAP:
        case K_BYTES:
            return k_render_whole(v);
        case K_CLOSURE: case K_FNREF: return k_str("<fn>");
        default: return k_str("<value>");
    }
    /* The one exit the number arms take. Every other tag returned above.
       The digits go over as sixteen bytes, eight more past fifteen, and the
       call for anything past twenty-three, which a number's rendering does
       not reach: buf is sixty-four bytes, so the reads past nlen are inside
       it, and k_alloc rounds a string's storage up to sixteen, so sixteen
       bytes fit a string of fifteen or fewer and twenty-four fit one of
       sixteen or more. glibc's memcpy took fifteen instructions to move a
       number's digits, 579,291 times a run on runbench, and a word loop
       bounded by nlen took twelve. */
    if (nlen == 1) return k_str_n(buf, 1);   /* a digit is in the cache */
    KStr* s = k_str_alloc(nlen);
    memcpy(s->data, buf, 16);
    if (nlen > 15) memcpy(s->data + 16, buf + 16, 8);
    if (nlen > 23) memcpy(s->data + 24, buf + 24, (size_t)(nlen - 24));
    s->data[nlen] = 0;
    KValue out; out.tag = K_STR; out.payload = k_ptr(s); return out;
}

static long long k_bytes_eq_list(KBytes* b, KList* l) {
    if (b->len != l->len) return 0;
    for (long long i = 0; i < b->len; i++) {
        if (l->items[i].tag != K_INT || l->items[i].payload != (long long)b->data[i]) return 0;
    }
    return 1;
}

/* reused across comparisons so k_eq never mallocs on the acyclic path */
typedef struct { const void* a; const void* b; unsigned long long gen; } KEqSlot;
static KEqSlot* k_eq_slots = NULL;
static size_t k_eq_cap = 0;
static size_t k_eq_live = 0;
static unsigned long long k_eq_gen = 0;

static void k_eq_begin(void) {
    if (!k_eq_slots) {
        k_eq_cap = 1024;
        k_eq_slots = calloc(k_eq_cap, sizeof(KEqSlot));
        if (!k_eq_slots) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_eq_gen++;
    k_eq_live = 0;
}

static size_t k_eq_probe(const void* a, const void* b) {
    size_t h = (((uintptr_t)a >> 4) * 0x9E3779B97F4A7C15ULL)
             ^ (((uintptr_t)b >> 4) * 0xC2B2AE3D27D4EB4FULL);
    size_t i = h & (k_eq_cap - 1);
    while (k_eq_slots[i].gen == k_eq_gen
           && (k_eq_slots[i].a != a || k_eq_slots[i].b != b))
        i = (i + 1) & (k_eq_cap - 1);
    return i;
}

static void k_eq_grow(void) {
    KEqSlot* old = k_eq_slots;
    size_t old_cap = k_eq_cap;
    k_eq_cap *= 2;
    k_eq_slots = calloc(k_eq_cap, sizeof(KEqSlot));
    if (!k_eq_slots) { fputs("out of memory\n", stderr); exit(1); }
    for (size_t i = 0; i < old_cap; i++) {
        if (old[i].gen != k_eq_gen) continue;
        size_t j = k_eq_probe(old[i].a, old[i].b);
        k_eq_slots[j] = old[i];
    }
    free(old);
}

/* returns 1 if the pair was already assumed equal this comparison */
static int k_eq_assume(const void* a, const void* b) {
    if (k_eq_live * 10 >= k_eq_cap * 7) k_eq_grow();
    size_t i = k_eq_probe(a, b);
    if (k_eq_slots[i].gen == k_eq_gen) return 1;
    k_eq_slots[i].gen = k_eq_gen;
    k_eq_slots[i].a = a;
    k_eq_slots[i].b = b;
    k_eq_live++;
    return 0;
}

static long long k_eq_rec(KValue a, KValue b);

static long long k_eq(KValue a, KValue b) {
    k_eq_begin();
    return k_eq_rec(a, b);
}

/* Asking whether two functions are the same function is asking which one you
   were handed, and dispatch answers that. The same goes for an effect: if the
   answer would change what happens next, an arm names it. So the question is
   refused rather than answered falsely, the way `<` already refuses it. */
static int k_opaque_to_equality(KValue v) {
    if (v.tag == K_SUB) v = k_sub_base(v);
    return v.tag == K_CLOSURE || v.tag == K_FNREF || v.tag == K_DESC;
}

/* A whole number against a fractional one, decided exactly. Widening the
   integer first rounds it onto the nearest value a double can hold and
   reports two different numbers as equal — 9007199254740993 is the smallest
   integer where that happens. Comparing against the double's floor keeps the
   integer whole, and the fraction breaks the tie. Answers -1, 0 or 1 with the
   integer on the left. */
static int k_order_int_float(long long i, double d) {
    if (d != d) return -1;                      /* every NaN ranks last */
    if (d >= 9223372036854775808.0) return -1;  /* past every int64 */
    if (d < -9223372036854775808.0) return 1;
    double fl = floor(d);
    long long whole = (long long)fl;
    if (i != whole) return (i > whole) - (i < whole);
    return d > fl ? -1 : 0;
}

/* The builtin float order, ruled 2026-09-30. Every NaN equals every NaN and
   ranks above inf, whatever its sign bit and however it was made, and -0.0
   equals and ranks with 0.0, which C's comparisons already do. Until then
   equality was C's ==, so nan == nan was false, while the ordering answered
   "equal" for any pair holding a NaN. */
static int k_order_float(double x, double y) {
    int xn = x != x, yn = y != y;
    if (xn || yn) return xn - yn;
    return (x > y) - (x < y);
}

/* An int of either kind against a float, exactly, as the interpreter's
   `cmp_int_float` answers: the int against the float's floor, and a float
   with a fraction above its floor ranks above an int equal to that floor. */
static int k_order_intv_float(KValue i, double d) {
    if (i.tag == K_INT) return k_order_int_float(i.payload, d);
    if (d != d) return -1;
    if (d == INFINITY) return -1;
    if (d == -INFINITY) return 1;
    double fl = floor(d);
    int c = k_int_cmp(i, k_int_of_whole_f(fl));
    if (c == 0 && d > fl) return -1;
    return c;
}

static long long k_eq_rec(KValue a, KValue b) {
    if (k_opaque_to_equality(a) || k_opaque_to_equality(b))
        k_die("equality is not defined on a function or an effect — write an arm "
              "for the case you mean");
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    /* A cell is forced the way any demand forces it, so an ordinary lazy
       operand compares as its value. A cell's identity survives forcing, so
       arriving twice at the same pair is the cycle closing through one, and
       the pair is assumed equal — the assumption that makes bisimulation
       terminate, the same one the record case below makes. A cell demanded
       mid-construction still dies in the force: that one is not a value yet. */
    if (a.tag == K_THUNK && b.tag == K_THUNK) {
        if (k_eq_assume((void*)a.payload, (void*)b.payload)) return 1;
        return k_eq_rec(k_force(a), k_force(b));
    }
    if (a.tag == K_THUNK) return k_eq_rec(k_force(a), b);
    if (b.tag == K_THUNK) return k_eq_rec(a, k_force(b));
    if (a.tag == K_BYTES && b.tag == K_LIST) return k_bytes_eq_list(k_as_bytes(a), k_as_list(b));
    if (a.tag == K_LIST && b.tag == K_BYTES) return k_bytes_eq_list(k_as_bytes(b), k_as_list(a));
    if (a.tag == K_BYTES && b.tag == K_BYTES) {
        KBytes* x = k_as_bytes(a); KBytes* y = k_as_bytes(b);
        return x->len == y->len && memcmp(x->data, y->data, x->len) == 0;
    }
    /* One numeric domain, which `<=` and `>=` already assert: both answer
       true for 1 and 1.0, and `<` answers false either way round because
       neither is strictly less. Equality was the one operator dissenting. A
       float cannot be a map key, so nothing needs them told apart. */
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_order_intv_float(a, k_as_f(b)) == 0;
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_order_intv_float(b, k_as_f(a)) == 0;
    if (a.tag == K_BIG && b.tag == K_BIG) {
        KBytes* x = k_as_bytes(a); KBytes* y = k_as_bytes(b);
        return x->len == y->len && memcmp(x->data, y->data, (size_t)x->len) == 0;
    }
    if (a.tag != b.tag) return 0;
    switch (a.tag) {
        case K_INT: return a.payload == b.payload;
        case K_FLOAT: return k_order_float(k_as_f(a), k_as_f(b)) == 0;
        case K_TRUE: case K_FALSE: case K_NONE: case K_DONE: return 1;
        case K_STR: {
            KStr* sa = k_as_str(a);
            KStr* sb = k_as_str(b);
            return sa->len == sb->len && memcmp(sa->data, sb->data, sa->len) == 0;
        }
        case K_REC: {
            KRec* ra = k_as_rec(a);
            KRec* rb = k_as_rec(b);
            if (ra == rb) return 1;
            if (ra->type_id != rb->type_id) return 0;
            /* A cycle is one object however the program closed it, and it
               keeps the structural comparison bisimulation gives it.
               Assuming this pair equal is what makes the walk terminate;
               cycles through cells make the same assumption above. */
            if (k_eq_assume(ra, rb)) return 1;
            for (long long i = 0; i < ra->nfields; i++) {
                if (!k_eq_rec(ra->fields[i], rb->fields[i])) return 0;
            }
            return 1;
        }
        case K_LIST: {
            KList* la = k_as_list(a);
            KList* lb = k_as_list(b);
            if (la->len != lb->len) return 0;
            for (long long i = 0; i < la->len; i++) {
                if (!k_eq_rec(la->items[i], lb->items[i])) return 0;
            }
            return 1;
        }
        case K_MAP: {
            long long na, nb;
            KValue* sa = k_map_sorted(k_as_map(a), &na);
            KValue* sb = k_map_sorted(k_as_map(b), &nb);
            if (na != nb) return 0;
            for (long long i = 0; i < na * 2; i++) {
                if (!k_eq_rec(sa[i], sb[i])) return 0;
            }
            return 1;
        }
        default: return 0;
    }
}

long long k_check_str(KValue v, const char* data, long long len) {
    if (v.tag != K_STR) return 0;
    KStr* s = k_as_str(v);
    return s->len == len && memcmp(s->data, data, len) == 0;
}

KValue k_add(KValue a, KValue b) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_add_overflow(a.payload, b.payload, &r)) return k_int_add(a, b);
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) + k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_add(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_float(k_int_to_f(a) + k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_float(k_as_f(a) + k_int_to_f(b));
    k_die("`+` is not defined for these values");
    return k_none();
}

KValue k_sub(KValue a, KValue b) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_sub_overflow(a.payload, b.payload, &r)) return k_int_sub(a, b);
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) - k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_sub(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_float(k_int_to_f(a) - k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_float(k_as_f(a) - k_int_to_f(b));
    k_die("`-` is not defined for these values");
    return k_none();
}

KValue k_mul(KValue a, KValue b) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_mul_overflow(a.payload, b.payload, &r)) return k_int_mul(a, b);
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) * k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_mul(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_float(k_int_to_f(a) * k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_float(k_as_f(a) * k_int_to_f(b));
    k_die("`*` is not defined for these values");
    return k_none();
}

/* Division answers a value, not a failure, and the value's type is declared
   like any other — so its id is the compiler's to assign and this has to be
   told. The emitter calls this once before main. */
static long long k_math_dz = -1;
static long long k_math_mf = -1;

void k_math_ids(long long divide_by_zero, long long math_failure) {
    k_math_dz = divide_by_zero;
    k_math_mf = math_failure;
}

KValue k_sub_wrap(long long type_id, KValue inner);

static KValue k_math_failure(const char* reason) {
    KValue text = k_str(reason);
    /* A program that never names either type was not given their ids, because
       it cannot tell the wrapped value from the text it reads as. */
    if (k_math_dz < 0) return text;
    KValue root = k_sub_wrap(k_math_mf, text);
    return k_sub_wrap(k_math_dz, root);
}

KValue k_div(KValue a, KValue b, const char* origin) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_math_failure("division by zero");
        /* the one signed division that overflows: the least integer over -1
           is one past the greatest, which C leaves undefined and this machine
           answers by wrapping. The answer is a bignum. */
        if (a.payload == INT64_MIN && b.payload == -1) return k_int_div(a, b);
        return k_int(a.payload / b.payload);
    }
    if (K_IS_INT(a) && K_IS_INT(b)) {
        if (b.tag == K_INT && b.payload == 0) return k_math_failure("division by zero");
        return k_int_div(a, b);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        if (k_as_f(b) == 0.0) return k_math_failure("division by zero");
        return k_float(k_as_f(a) / k_as_f(b));
    }
    if ((K_IS_INT(a) || a.tag == K_FLOAT) && (K_IS_INT(b) || b.tag == K_FLOAT)) {
        double x = a.tag == K_FLOAT ? k_as_f(a) : k_int_to_f(a);
        double y = b.tag == K_FLOAT ? k_as_f(b) : k_int_to_f(b);
        if (y == 0.0) return k_math_failure("division by zero");
        return k_float(x / y);
    }
    k_die("`/` is not defined for these values");
    return k_none();
}

KValue k_mod(KValue a, KValue b, const char* origin) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_math_failure("modulo by zero");
        /* the least integer modulo -1 is zero, and zero fits — but the
           quotient it is computed from does not, and x86's division traps
           on the pair. ARM answers without trapping, which is why this
           crashed only on the machine nobody was testing on. */
        if (a.payload == INT64_MIN && b.payload == -1) return k_int(0);
        return k_int(a.payload % b.payload);
    }
    if (K_IS_INT(a) && K_IS_INT(b)) {
        if (b.tag == K_INT && b.payload == 0) return k_math_failure("modulo by zero");
        return k_int_rem(a, b);
    }
    if ((K_IS_INT(a) || a.tag == K_FLOAT) && (K_IS_INT(b) || b.tag == K_FLOAT)) {
        double x = a.tag == K_FLOAT ? k_as_f(a) : k_int_to_f(a);
        double y = b.tag == K_FLOAT ? k_as_f(b) : k_int_to_f(b);
        if (y == 0.0) return k_math_failure("modulo by zero");
        return k_float(fmod(x, y));
    }
    k_die("`%` is not defined for these values");
    return k_none();
}

static int k_order(KValue a, KValue b) {
    if (a.tag == K_INT && b.tag == K_INT) return (a.payload > b.payload) - (a.payload < b.payload);
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_order_float(k_as_f(a), k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_cmp(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_order_intv_float(a, k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return -k_order_intv_float(b, k_as_f(a));
    if (a.tag == K_STR && b.tag == K_STR) {
        KStr* sa = k_as_str(a);
        KStr* sb = k_as_str(b);
        long n = sa->len < sb->len ? sa->len : sb->len;
        int c = memcmp(sa->data, sb->data, n);
        if (c) return c > 0 ? 1 : -1;
        return (sa->len > sb->len) - (sa->len < sb->len);
    }
    k_die("comparison requires two values of one comparable type");
    return 0;
}

KValue k_cmp(KValue a, KValue b, long long op) {
    /* Two strings asked whether they are equal. k_eq opens a cycle-tracking
       generation and walks a dozen tag tests to reach the same length and
       byte compare it ends with here; std/regexp asks this of one character
       against a pattern literal at every step, 91,378 times on runbench's
       scan, where it cost ninety instructions a question. A one-character
       string comes from k_str_n's cache and a literal is permanent, so the
       same text is usually the same pointer. */
    if (op <= 1 && a.tag == K_STR && b.tag == K_STR) {
        KStr* sa = k_as_str(a);
        KStr* sb = k_as_str(b);
        int same = sa == sb
            || (sa->len == sb->len && memcmp(sa->data, sb->data, (size_t)sa->len) == 0);
        return k_bool(op == 0 ? same : !same);
    }
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (op == 0) return k_bool(k_eq(a, b));
    if (op == 1) return k_bool(!k_eq(a, b));
    int c = k_order(a, b);
    switch (op) {
        case 2: return k_bool(c < 0);
        case 3: return k_bool(c <= 0);
        case 4: return k_bool(c > 0);
        default: return k_bool(c >= 0);
    }
}

