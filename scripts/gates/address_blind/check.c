/* The replacements against libc, before any counted run uses them. A wrong
   memmove would not show up as a wrong count: it would corrupt the compiler
   under measurement, and the row would count whatever the corrupted compiler
   did. So every comparison sign, copy and overlapping move is checked here
   against libc's own answer. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define memcmp blind_memcmp
#define bcmp blind_bcmp
#include "compare.c"
#undef memcmp
#undef bcmp
#define memcpy blind_memcpy
#define memmove blind_memmove
#include "copy.c"
#undef memcpy
#undef memmove

static int sign(int x) { return (x > 0) - (x < 0); }

int main(void) {
    static unsigned char a[4096], b[4096], x[4096], y[4096];
    unsigned seed = 1;
    long cases = 0;
    for (int round = 0; round < 20000; round++) {
        for (int i = 0; i < 4096; i++) {
            seed = seed * 1103515245u + 12345u;
            a[i] = b[i] = (unsigned char)(seed >> 16);
        }
        seed = seed * 1103515245u + 12345u;
        size_t n = (seed >> 8) % (round % 4 == 0 ? 1500 : 80);
        size_t oa = (seed >> 3) % 512, ob = (seed >> 13) % 512;
        memcpy(b + ob, a + oa, n);
        if (n && round % 3) b[ob + (seed >> 20) % n] ^= (unsigned char)(1 + round % 255);
        if (sign(blind_memcmp(a + oa, b + ob, n)) != sign(memcmp(a + oa, b + ob, n)) ||
            (blind_bcmp(a + oa, b + ob, n) == 0) != (memcmp(a + oa, b + ob, n) == 0)) {
            fprintf(stderr, "compare disagrees with libc at n=%zu\n", n);
            return 1;
        }
        memcpy(x, a, 4096);
        memcpy(y, a, 4096);
        size_t src = (seed >> 5) % 2048, dst = (seed >> 17) % 2048;
        memmove(x + dst, x + src, n);
        blind_memmove(y + dst, y + src, n);
        if (memcmp(x, y, 4096)) {
            fprintf(stderr, "memmove disagrees with libc: n=%zu src=%zu dst=%zu\n", n, src, dst);
            return 1;
        }
        memcpy(x, a, 4096);
        memcpy(y, a, 4096);
        memcpy(x + 2048 + ob, a + oa, n);
        blind_memcpy(y + 2048 + ob, a + oa, n);
        if (memcmp(x, y, 4096)) {
            fprintf(stderr, "memcpy disagrees with libc at n=%zu\n", n);
            return 1;
        }
        cases += 4;
    }
    /* Every length up to 100, equal and then with one byte raised and one
       lowered at every position, which crosses each size class of the
       comparison and puts the first difference in every part of it. */
    for (size_t n = 0; n <= 100; n++) {
        for (size_t i = 0; i < n; i++) x[i] = y[i] = (unsigned char)(0x41 + i * 7);
        if (blind_memcmp(x, y, n) != 0 || blind_bcmp(x, y, n) != 0) {
            fprintf(stderr, "compare calls equal bytes different at n=%zu\n", n);
            return 1;
        }
        cases++;
        for (size_t pos = 0; pos < n; pos++) {
            for (int d = -1; d <= 1; d += 2) {
                for (size_t i = 0; i < n; i++) y[i] = x[i];
                y[pos] = (unsigned char)(y[pos] + d);
                if (sign(blind_memcmp(x, y, n)) != sign(memcmp(x, y, n)) ||
                    (blind_bcmp(x, y, n) == 0) != (memcmp(x, y, n) == 0)) {
                    fprintf(stderr, "compare disagrees with libc: n=%zu pos=%zu d=%d\n", n, pos, d);
                    return 1;
                }
                cases++;
            }
        }
    }
    /* Every length up to 300 at every distance from -140 to 140 between
       source and destination, which crosses each size class's boundary and
       every overlap in both directions. */
    for (size_t n = 0; n <= 300; n++) {
        for (long dist = -140; dist <= 140; dist++) {
            size_t src = 1024, dst = (size_t)(1024 + dist);
            memcpy(x, a, 4096);
            memcpy(y, a, 4096);
            memmove(x + dst, x + src, n);
            blind_memmove(y + dst, y + src, n);
            if (memcmp(x, y, 4096)) {
                fprintf(stderr, "memmove disagrees with libc: n=%zu distance=%ld\n", n, dist);
                return 1;
            }
            cases++;
        }
        memcpy(x, a, 4096);
        memcpy(y, a, 4096);
        memcpy(x + 3000, a + 7, n);
        blind_memcpy(y + 3000, a + 7, n);
        if (memcmp(x, y, 4096)) {
            fprintf(stderr, "memcpy disagrees with libc at n=%zu\n", n);
            return 1;
        }
        cases++;
    }
    printf("%ld cases agree with libc\n", cases);
    return 0;
}
