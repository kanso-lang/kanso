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
