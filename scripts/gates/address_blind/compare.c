/* memcmp and bcmp for the counted runs, preloaded over libc's.

   glibc's __memcmp_avx2_movbe picks its path by where the operands sit: a
   comparison shorter than 32 bytes first asks whether (s1 | s2) & 4095 lies
   within 32 bytes of a page end, and takes a longer branch when it does. So
   the same comparison of the same bytes costs a different number of
   instructions depending on where the linker put one string and where the
   allocator put the other. A change that grows src/runtime.c by forty lines moves
   every string after it in .rodata, and on 2026-09-23 that moved 478 of the
   compile row's comparisons from one branch to the other with the call counts
   unchanged.

   This version costs a number of instructions that depends on the length and
   on where the first difference falls, and on nothing else. Every load stays
   inside [0, n). It takes the shape copy.c takes: a head and a tail that may
   overlap up to 32 bytes, 32-byte AVX2 steps above that. The first version
   compared eight bytes a step and finished byte by byte, and on the
   interpreted run it cost three times libc's bcmp per call, 44 instructions
   against about 15, over some 780,000 calls; that would have weighed name
   comparison far above what it costs. */
#include <immintrin.h>
#include <stddef.h>
#include <stdint.h>

static inline uint64_t be64(const unsigned char *p) { uint64_t v; __builtin_memcpy(&v, p, 8); return __builtin_bswap64(v); }
static inline uint32_t be32(const unsigned char *p) { uint32_t v; __builtin_memcpy(&v, p, 4); return __builtin_bswap32(v); }
static inline uint16_t be16(const unsigned char *p) { uint16_t v; __builtin_memcpy(&v, p, 2); return __builtin_bswap16(v); }

static inline int order64(uint64_t x, uint64_t y) { return (x > y) - (x < y); }

/* The index of the first differing byte in a 32- or 16-byte block, or -1. */
static inline int diff32(const unsigned char *a, const unsigned char *b) {
    __m256i x = _mm256_loadu_si256((const __m256i *)a), y = _mm256_loadu_si256((const __m256i *)b);
    unsigned m = ~(unsigned)_mm256_movemask_epi8(_mm256_cmpeq_epi8(x, y));
    return m ? __builtin_ctz(m) : -1;
}

static inline int diff16(const unsigned char *a, const unsigned char *b) {
    __m128i x = _mm_loadu_si128((const __m128i *)a), y = _mm_loadu_si128((const __m128i *)b);
    unsigned m = ~(unsigned)_mm_movemask_epi8(_mm_cmpeq_epi8(x, y)) & 0xffffu;
    return m ? __builtin_ctz(m) : -1;
}

__attribute__((noinline)) static int compare(const unsigned char *a, const unsigned char *b, size_t n) {
    int k;
    if (n >= 32) {
        size_t i = 0;
        for (; i + 32 < n; i += 32)
            if ((k = diff32(a + i, b + i)) >= 0) return (int)a[i + k] - (int)b[i + k];
        /* The last 32 bytes, overlapping what the loop already found equal. */
        i = n - 32;
        if ((k = diff32(a + i, b + i)) >= 0) return (int)a[i + k] - (int)b[i + k];
        return 0;
    }
    if (n >= 16) {
        if ((k = diff16(a, b)) >= 0) return (int)a[k] - (int)b[k];
        size_t i = n - 16;
        if ((k = diff16(a + i, b + i)) >= 0) return (int)a[i + k] - (int)b[i + k];
        return 0;
    }
    if (n >= 8) {
        uint64_t x = be64(a), y = be64(b);
        if (x != y) return order64(x, y);
        return order64(be64(a + n - 8), be64(b + n - 8));
    }
    if (n >= 4) {
        uint64_t x = ((uint64_t)be32(a) << 32) | be32(a + n - 4);
        uint64_t y = ((uint64_t)be32(b) << 32) | be32(b + n - 4);
        return order64(x, y);
    }
    if (n >= 2) {
        uint32_t x = ((uint32_t)be16(a) << 16) | be16(a + n - 2);
        uint32_t y = ((uint32_t)be16(b) << 16) | be16(b + n - 2);
        return (x > y) - (x < y);
    }
    if (n == 1) return (int)a[0] - (int)b[0];
    return 0;
}

int memcmp(const void *a, const void *b, size_t n) { return compare(a, b, n); }
int bcmp(const void *a, const void *b, size_t n) { return compare(a, b, n); }
