/* The self-test's copy program. It moves 1,500 bytes to a destination a given
   distance above the source, inside `probe` so callgrind counts that frame
   alone. libc's memcpy costs 177 instructions at a distance of 4096 and 179 at
   5000; the preloaded one must cost the same at both. */
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

void *(*volatile cpy)(void *, const void *, size_t) = memcpy;

__attribute__((noinline)) void *probe(char *d, const char *s, size_t n) {
    return cpy(d, s, n);
}

int main(int argc, char **argv) {
    long dist = argc > 1 ? atol(argv[1]) : 4096;
    char *a = mmap(0, 1 << 20, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    probe(a + 65536 + dist, a + 65536, 1500);
    return 0;
}
