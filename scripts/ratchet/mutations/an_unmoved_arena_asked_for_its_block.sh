#!/bin/sh
# A beat's rewind asks first whether the arena pointer still stands at the
# mark, and returns if it does: blocks never overlap, so the pointer answers
# for the block as well. This mutation skips that exit, so a loop that
# allocated nothing goes on to load the chain's head and compare it before
# returning. The answers are the same; the work vein is the witness.
set -e
line='        if (__builtin_expect(k_arena == m->ptr, 1)) return;'
[ "$(grep -cxF "$line" src/runtime.c)" -eq 1 ] || {
  echo "the unmoved arena's exit moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if (__builtin_expect(k_arena == m->ptr, 1)) return;$/        if (0 \&\& k_arena == m->ptr) return;/' src/runtime.c
if grep -qxF "$line" src/runtime.c; then exit 1; fi
