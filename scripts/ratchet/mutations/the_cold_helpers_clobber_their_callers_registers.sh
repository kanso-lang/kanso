#!/bin/sh
# Six cold helpers in the runtime carry `preserve_most` since 2026-09-15: the
# arena refill, the permanent allocator, the int parser's strtoll tail, the
# ascii cache's fill, and the two arms of an index that allocate or refuse.
# A caller of a preserve_most function keeps its live values in caller-saved
# registers across the call, so a hot function whose only calls are cold ones
# opens with no pushes at all. Without the attribute the compiler pins those
# values in callee-saved registers and every call to the hot function pays
# the pushes and pops for a path a run takes a few hundred times.
#
# This strips the attribute from all six. Every value out is identical, so
# nothing but the work vein can see it, which is exactly what the row asserts.
set -e
grep -q "static __attribute__((noinline, preserve_most)) void\* k_alloc_refill(" src/runtime.c || {
  echo "the arena refill's attributes changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/, preserve_most))/))/" src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
# anchored to the attribute's closing parens: the comments name the
# attribute too, and those stay.
if grep -q "preserve_most))" src/runtime.c; then
  echo "a preserve_most survived the mutation" >&2
  exit 1
fi
