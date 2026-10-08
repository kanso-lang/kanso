#!/bin/sh
# A string value of up to fifteen bytes is held inside the value. This
# mutation sets the inline width to nothing, so every string but the empty one
# goes to the heap as a `String` did before. The program's output is the same;
# the interpreted row is the witness.
set -e
kept='const TEXT_INLINE: usize = 15;'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the inline width moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^const TEXT_INLINE: usize = 15;$/const TEXT_INLINE: usize = 0;/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
