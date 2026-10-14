#!/bin/sh
# k_b_at's two arms that allocate or refuse -- a wide character the cache
# does not hold, and a map or an unindexable value -- are separate cold
# functions since 2026-09-15, so the arm every runbench index takes carries no
# frame. This inlines both arms back into the index and the six pushes and the
# forty-byte reservation return to every call. The value out is identical
# either way, so nothing but the work vein can see it.
set -e
grep -q "static __attribute__((noinline, cold, preserve_most)) KValue k_b_at_wide_miss(" src/runtime/text.c || {
  echo "the index's cold arms changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/static __attribute__((noinline, cold, preserve_most)) KValue k_b_at_wide_miss(/static inline __attribute__((always_inline)) KValue k_b_at_wide_miss(/" \
    -e "s/static __attribute__((noinline, cold, preserve_most)) KValue k_b_at_rest(/static inline __attribute__((always_inline)) KValue k_b_at_rest(/" \
    src/runtime/text.c > src/runtime/text.c.mut
mv src/runtime/text.c.mut src/runtime/text.c
grep -q "always_inline)) KValue k_b_at_wide_miss(" src/runtime/text.c
grep -q "always_inline)) KValue k_b_at_rest(" src/runtime/text.c
