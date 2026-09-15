#!/bin/sh
# Since 2026-09-15 the two byte scanners finish a string shorter than a
# vector with one sixteen-byte load masked down to the string's bytes, taken
# when k_tail_window says the load stays inside the page. Before, the tail
# was walked one byte at a time: 8,042,860 bytes a run at eight instructions
# each on the run program, because a json key or value is usually shorter
# than sixteen bytes and the vector loop never opened. This makes the window
# test answer no, so the byte walk is the only tail again. Every scan answers
# the same position either way, so nothing but the work vein can see it,
# which is what the row asserts.
set -e
grep -q "    return ((uintptr_t)p & 4095) <= 4096 - 16;" src/runtime.c || {
  echo "the tail window test changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/    return ((uintptr_t)p & 4095) <= 4096 - 16;/    return 0;/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "^static inline int k_tail_window(const unsigned char\* p) {" src/runtime.c
