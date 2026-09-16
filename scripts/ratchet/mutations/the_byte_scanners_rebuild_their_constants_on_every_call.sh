#!/bin/sh
# k_b_find2_raw and k_b_find2_below_raw are always_inline since 2026-09-15:
# the link is LTO, every emitted caller hands them literal bytes, and inlined
# the two broadcasts fold to constant vectors. Before, each of 3,109,759
# calls a run rebuilt the vectors from registers, half the call's cost when
# the hit is in the first sixteen bytes. This puts the out-of-line doors
# back. Every scan answers the same position either way, so nothing but the
# work vein can see it, which is what the row asserts.
set -e
grep -q "^__attribute__((always_inline)) long long k_b_find2_raw(" src/runtime.c || {
  echo "the find2 door changed shape; this mutation needs rewriting" >&2
  exit 1
}
grep -q "^__attribute__((always_inline)) long long k_b_find2_below_raw(" src/runtime.c || {
  echo "the find2_below door changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/^__attribute__((always_inline)) long long k_b_find2_raw(/long long k_b_find2_raw(/" \
    -e "s/^__attribute__((always_inline)) long long k_b_find2_below_raw(/long long k_b_find2_below_raw(/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "^long long k_b_find2_raw(" src/runtime.c
grep -q "^long long k_b_find2_below_raw(" src/runtime.c
