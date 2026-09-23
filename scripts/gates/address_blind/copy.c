/* memcpy and memmove for the counted runs, preloaded over libc's.

   glibc's __memmove_avx_unaligned_erms, which also serves memcpy, chooses its
   path for a large copy partly by the distance between the destination and
   the source. The same 1,500 bytes copied cost 177 instructions at one distance
   and 179 at another on 2026-09-23. With memcmp already preloaded, that term
   alone moved kanso#1571's compile, entry and library rows by 44, 122 and 122
   when kanso#1561's runtime was merged under it.

   This version chooses every branch by the length alone. Up to 128 bytes it
   loads a head and a tail that may overlap, then stores both, the way libc
   handles short copies. Above that it loads the far end first, moves 128 bytes
   a step with unaligned AVX2 loads and stores, and stores the far end last.
   Everything a step writes was loaded before the step writes it, which keeps
   an overlapping move correct in either direction, and memmove walks backward
   when the destination starts inside the source. A version that finished with
   single bytes cost 1.9 times libc's memcpy on the start-up row, from the short
   copies; a byte loop cost 2.7 times on a 1,500-byte copy. Every runner that
   counts these rows already takes libc's AVX2 paths. */
#include <immintrin.h>
#include <stddef.h>
#include <stdint.h>

typedef __m256i v32;
typedef __m128i v16;

static inline v32 l32(const unsigned char *p) { return _mm256_loadu_si256((const v32 *)p); }
static inline void s32(unsigned char *p, v32 v) { _mm256_storeu_si256((v32 *)p, v); }
static inline v16 l16(const unsigned char *p) { return _mm_loadu_si128((const v16 *)p); }
static inline void s16(unsigned char *p, v16 v) { _mm_storeu_si128((v16 *)p, v); }
static inline uint64_t l8(const unsigned char *p) { uint64_t v; __builtin_memcpy(&v, p, 8); return v; }
static inline void s8(unsigned char *p, uint64_t v) { __builtin_memcpy(p, &v, 8); }
static inline uint32_t l4(const unsigned char *p) { uint32_t v; __builtin_memcpy(&v, p, 4); return v; }
static inline void s4(unsigned char *p, uint32_t v) { __builtin_memcpy(p, &v, 4); }
static inline uint16_t l2(const unsigned char *p) { uint16_t v; __builtin_memcpy(&v, p, 2); return v; }
static inline void s2(unsigned char *p, uint16_t v) { __builtin_memcpy(p, &v, 2); }

/* n <= 128. Every load happens before the first store. */
static inline void small(unsigned char *d, const unsigned char *s, size_t n) {
    if (n >= 64) {
        v32 a = l32(s), b = l32(s + 32), c = l32(s + n - 64), e = l32(s + n - 32);
        s32(d, a); s32(d + 32, b); s32(d + n - 64, c); s32(d + n - 32, e);
    } else if (n >= 32) {
        v32 a = l32(s), b = l32(s + n - 32);
        s32(d, a); s32(d + n - 32, b);
    } else if (n >= 16) {
        v16 a = l16(s), b = l16(s + n - 16);
        s16(d, a); s16(d + n - 16, b);
    } else if (n >= 8) {
        uint64_t a = l8(s), b = l8(s + n - 8);
        s8(d, a); s8(d + n - 8, b);
    } else if (n >= 4) {
        uint32_t a = l4(s), b = l4(s + n - 4);
        s4(d, a); s4(d + n - 4, b);
    } else if (n >= 2) {
        uint16_t a = l2(s), b = l2(s + n - 2);
        s2(d, a); s2(d + n - 2, b);
    } else if (n == 1) {
        d[0] = s[0];
    }
}

/* n > 128, front to back. The last 128 bytes are loaded before anything is
   stored and written last. */
__attribute__((noinline)) static void forward(unsigned char *d, const unsigned char *s, size_t n) {
    v32 t0 = l32(s + n - 128), t1 = l32(s + n - 96), t2 = l32(s + n - 64), t3 = l32(s + n - 32);
    for (size_t i = 0; i + 128 < n; i += 128) {
        v32 a = l32(s + i), b = l32(s + i + 32), c = l32(s + i + 64), e = l32(s + i + 96);
        s32(d + i, a); s32(d + i + 32, b); s32(d + i + 64, c); s32(d + i + 96, e);
    }
    s32(d + n - 128, t0); s32(d + n - 96, t1); s32(d + n - 64, t2); s32(d + n - 32, t3);
}

/* n > 128, back to front. The first 128 bytes are loaded before anything is
   stored and written last. */
__attribute__((noinline)) static void backward(unsigned char *d, const unsigned char *s, size_t n) {
    v32 h0 = l32(s), h1 = l32(s + 32), h2 = l32(s + 64), h3 = l32(s + 96);
    for (size_t i = n; i > 128; i -= 128) {
        v32 a = l32(s + i - 32), b = l32(s + i - 64), c = l32(s + i - 96), e = l32(s + i - 128);
        s32(d + i - 32, a); s32(d + i - 64, b); s32(d + i - 96, c); s32(d + i - 128, e);
    }
    s32(d, h0); s32(d + 32, h1); s32(d + 64, h2); s32(d + 96, h3);
}

void *memcpy(void *d, const void *s, size_t n) {
    if (n <= 128) small(d, s, n);
    else forward(d, s, n);
    return d;
}

void *memmove(void *d, const void *s, size_t n) {
    const unsigned char *src = s;
    unsigned char *dst = d;
    if (n <= 128) small(dst, src, n);
    else if (dst > src && dst < src + n) backward(dst, src, n);
    else forward(dst, src, n);
    return d;
}
