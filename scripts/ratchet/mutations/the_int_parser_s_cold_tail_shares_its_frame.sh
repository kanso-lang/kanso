#!/bin/sh
# k_b_to_int's strtoll fallthrough and its two refusals live in a separate
# cold function since 2026-09-15, so the digit loop that every call takes
# carries no frame. With the libc call in the same body the compiler pinned
# the string's data, length and origin in callee-saved registers and paid
# five pushes and five pops on every call for a path one call in a hundred
# thousand takes.
#
# This inlines the cold tail back into the fast path in runtime.c. It keeps
# preserve_most and external linkage, because the release build's hot unit
# declares and calls it on that convention. The value out is the same either
# way; only the frame comes back, so nothing but the work vein can see this,
# which is exactly what the row asserts.
set -e
grep -q "^__attribute__((noinline, cold, preserve_most)) KValue k_b_to_int_slow(" src/runtime/numbers.c || {
  echo "the int parser's cold tail changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/^__attribute__((noinline, cold, preserve_most)) KValue k_b_to_int_slow(/__attribute__((always_inline, preserve_most)) KValue k_b_to_int_slow(/" \
    src/runtime/numbers.c > src/runtime/numbers.c.mut
mv src/runtime/numbers.c.mut src/runtime/numbers.c
grep -q "^__attribute__((always_inline, preserve_most)) KValue k_b_to_int_slow(" src/runtime/numbers.c
