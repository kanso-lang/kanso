#!/bin/sh
# k_b_slice's multibyte walk is its own function in tail position since
# 2026-09-15: the walk keeps a dozen values live, and inline it cost the
# slice's door seven pushes on every slice of any shape, 183,682 a run on the
# run program. This inlines the walk back. Every slice is the same bytes, so
# nothing but the work vein can see it, which is exactly what the row asserts.
set -e
grep -q "^static __attribute__((noinline)) KValue k_b_slice_walk(" src/runtime.c || {
  echo "the slice walk changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/^static __attribute__((noinline)) KValue k_b_slice_walk(/static inline __attribute__((always_inline)) KValue k_b_slice_walk(/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "^static inline __attribute__((always_inline)) KValue k_b_slice_walk(" src/runtime.c
