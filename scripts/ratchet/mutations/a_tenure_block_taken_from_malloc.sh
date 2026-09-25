#!/bin/sh
# A tenure block is mapped, not malloc'd. glibc's threshold for mapping a
# large malloc rises each time a mapped chunk is freed, so a malloc'd block
# landed in the heap once the run program had freed a few, above the
# encoder's byte builder, and the builder's realloc could no longer grow in
# place. This mutation takes the block from malloc again.
#
# No output and no allocation counter moves. runbench's instructions rise by
# the builder's copies, about thirteen million.
set -e
line='    void* m = mmap(NULL, cap, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);'
grep -qF "$line" src/runtime.c || {
  echo "the tenure block's mapping changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    void\* m = mmap(NULL, cap, PROT_READ \| PROT_WRITE, MAP_PRIVATE \| MAP_ANONYMOUS, -1, 0);$|    void* m = malloc(cap);|' src/runtime.c
sed -i 's|^        munmap(b->data, b->cap);$|        free(b->data);|' src/runtime.c
grep -q '^    void\* m = malloc(cap);$' src/runtime.c
grep -q '^        free(b->data);$' src/runtime.c
