#!/bin/sh
# An empty list literal's buffer opens with room for four since 2026-09-15.
# Before, it held one slot, so the second push of every array the decoder
# builds went through k_b_push_grow: 225,621 of the run program's 252,499
# grows. This puts the one-slot buffer back. Every list holds the same
# values either way, so nothing but the work vein can see it, which is what
# the row asserts.
set -e
grep -q "    KValue\* buf = k_buf(n ? n : 4);" src/runtime.c || {
  echo "the empty literal's buffer changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/    KValue\* buf = k_buf(n ? n : 4);/    KValue* buf = k_buf(n ? n : 1);/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "    KValue\* buf = k_buf(n ? n : 1);" src/runtime.c
