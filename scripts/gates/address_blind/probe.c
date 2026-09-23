/* The self-test's program. It compares the same sixteen bytes placed at a
   page offset given on the command line, through a pointer the compiler
   cannot see through, inside `probe` so callgrind can count that frame alone.
   At offset 64 libc's avx2 memcmp takes its fast path; at 4080 the operands
   lie within 32 bytes of a page end and it takes the page-cross branch. */
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

int (*volatile cmp)(const void *, const void *, size_t) = memcmp;

__attribute__((noinline)) int probe(const char *a, const char *b) {
    return cmp(a, b, 16);
}

int main(int argc, char **argv) {
    long off = argc > 1 ? atol(argv[1]) : 64;
    char *pa = mmap(0, 8192, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    char *pb = mmap(0, 8192, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    memcpy(pa + off, "sixteen bytes ab", 16);
    memcpy(pb + off, "sixteen bytes ab", 16);
    return probe(pa + off, pb + off);
}
