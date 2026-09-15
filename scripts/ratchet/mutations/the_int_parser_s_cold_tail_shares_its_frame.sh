#!/bin/sh
# k_b_to_int's strtoll fallthrough and its two refusals live in a separate
# cold function since 2026-09-15, so the digit loop that every call takes
# carries no frame. With the libc call in the same body the compiler pinned
# the string's data, length and origin in callee-saved registers and paid
# five pushes and five pops on every call for a path one call in a hundred
# thousand takes.
#
# This inlines the cold tail back into the fast path. The value out is the
# same either way; only the frame comes back, so nothing but the work vein
# can see this, which is exactly what the row asserts.
set -e
grep -q "static __attribute__((noinline, cold)) KValue k_b_to_int_slow(" src/runtime.c || {
  echo "the int parser's cold tail changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/static __attribute__((noinline, cold)) KValue k_b_to_int_slow(/static inline __attribute__((always_inline)) KValue k_b_to_int_slow(/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "static inline __attribute__((always_inline)) KValue k_b_to_int_slow(" src/runtime.c
