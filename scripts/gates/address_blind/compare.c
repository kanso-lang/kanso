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
   inside [0, n). */
#include <stddef.h>
#include <stdint.h>

static inline uint64_t load8(const unsigned char *p) {
    uint64_t v;
    __builtin_memcpy(&v, p, 8);
    return v;
}

__attribute__((noinline)) static int compare(const unsigned char *a, const unsigned char *b, size_t n) {
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        uint64_t x = load8(a + i), y = load8(b + i);
        if (x != y) {
            x = __builtin_bswap64(x);
            y = __builtin_bswap64(y);
            return x < y ? -1 : 1;
        }
    }
    for (; i < n; i++) {
        int d = (int)a[i] - (int)b[i];
        if (d) return d;
    }
    return 0;
}

int memcmp(const void *a, const void *b, size_t n) { return compare(a, b, n); }
int bcmp(const void *a, const void *b, size_t n) { return compare(a, b, n); }
