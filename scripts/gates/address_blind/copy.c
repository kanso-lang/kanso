/* memcpy and memmove for the counted runs, preloaded over libc's.

   glibc's __memmove_avx_unaligned_erms, which also serves memcpy, chooses its
   path for a large copy partly by the distance between the destination and
   the source. The same 1,500 bytes copied cost 177 instructions at one distance
   and 179 at another on 2026-09-23. With memcmp already preloaded, that term
   alone moved kanso#1571's compile, entry and library rows by 44, 122 and 122
   when kanso#1561's runtime was merged under it.

   This version moves 128 bytes a step with unaligned AVX2 loads and stores,
   then 8, then single bytes, so its cost depends on the length alone. A byte
   loop showed the same address-blindness at 2.7 times libc's cost, which would
   have weighed copying far above everything else a row counts; 128 bytes a
   step keeps it near libc's. Each step loads everything it will write before
   it writes, which keeps an overlapping move correct in either direction, and
   memmove walks backward when the destination starts inside the source. Every
   runner that counts these rows already takes libc's AVX2 paths. */
#include <immintrin.h>
#include <stddef.h>
#include <stdint.h>

static inline uint64_t copy_load8(const unsigned char *p) {
    uint64_t v;
    __builtin_memcpy(&v, p, 8);
    return v;
}

static inline void copy_store8(unsigned char *p, uint64_t v) { __builtin_memcpy(p, &v, 8); }

static inline __m256i copy_load32(const unsigned char *p) { return _mm256_loadu_si256((const __m256i *)p); }

static inline void copy_store32(unsigned char *p, __m256i v) { _mm256_storeu_si256((__m256i *)p, v); }

__attribute__((noinline)) static void forward(unsigned char *d, const unsigned char *s, size_t n) {
    size_t i = 0;
    for (; i + 128 <= n; i += 128) {
        __m256i a = copy_load32(s + i), b = copy_load32(s + i + 32), c = copy_load32(s + i + 64), e = copy_load32(s + i + 96);
        copy_store32(d + i, a);
        copy_store32(d + i + 32, b);
        copy_store32(d + i + 64, c);
        copy_store32(d + i + 96, e);
    }
    for (; i + 8 <= n; i += 8) copy_store8(d + i, copy_load8(s + i));
    for (; i < n; i++) d[i] = s[i];
}

__attribute__((noinline)) static void backward(unsigned char *d, const unsigned char *s, size_t n) {
    size_t i = n;
    for (; i >= 128; i -= 128) {
        __m256i a = copy_load32(s + i - 32), b = copy_load32(s + i - 64), c = copy_load32(s + i - 96), e = copy_load32(s + i - 128);
        copy_store32(d + i - 32, a);
        copy_store32(d + i - 64, b);
        copy_store32(d + i - 96, c);
        copy_store32(d + i - 128, e);
    }
    for (; i >= 8; i -= 8) copy_store8(d + i - 8, copy_load8(s + i - 8));
    for (; i > 0; i--) d[i - 1] = s[i - 1];
}

void *memcpy(void *d, const void *s, size_t n) {
    forward(d, s, n);
    return d;
}

void *memmove(void *d, const void *s, size_t n) {
    const unsigned char *src = s;
    unsigned char *dst = d;
    if (dst > src && dst < src + n) backward(dst, src, n);
    else forward(dst, src, n);
    return d;
}
