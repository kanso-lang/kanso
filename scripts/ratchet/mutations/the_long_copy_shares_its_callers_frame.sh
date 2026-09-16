#!/bin/sh
# A copy long enough to be libc's goes through k_copy_cold since 2026-09-15,
# a preserve_most call, so a function whose short copies are inline words
# keeps no frame for the long case: k_map_lit, k_mklist and the string
# builders behind k_b_utf8_slice_raw open with no pushes. This inlines the
# wrapper back, which puts memcpy's call in every one of them and the pushes
# with it. Every byte copied is the same, so nothing but the work vein can
# see this, which is exactly what the row asserts.
set -e
grep -q "static __attribute__((noinline, cold, preserve_most)) void k_copy_cold(" src/runtime.c || {
  echo "the long copy's wrapper changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/static __attribute__((noinline, cold, preserve_most)) void k_copy_cold(/static inline __attribute__((always_inline)) void k_copy_cold(/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "static inline __attribute__((always_inline)) void k_copy_cold(" src/runtime.c
