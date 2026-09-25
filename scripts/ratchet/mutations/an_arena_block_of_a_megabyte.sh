#!/bin/sh
# An arena block is half a megabyte. The run program's base -- the text it
# reads and the document decoded from it -- takes three such blocks, where at
# a megabyte it took two and left most of the second unused under every phase
# above it. This mutation puts the block back at a megabyte.
#
# runbench's arena_peak_bytes reads 3,670,032 against 3,145,744.
set -e
grep -q '^#define K_BLOCK_BYTES ((size_t)1 << 19)$' src/runtime.c || {
  echo "the block size changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^#define K_BLOCK_BYTES ((size_t)1 << 19)$|#define K_BLOCK_BYTES ((size_t)1 << 20)|' src/runtime.c
grep -q '^#define K_BLOCK_BYTES ((size_t)1 << 20)$' src/runtime.c
