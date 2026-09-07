#!/bin/sh
# A push that is not on its buffer's frontier grows: new storage, the elements
# copied, the item after them. Whether the new storage is arena or malloc is
# decided by whether the list's header predates the beat, and k_outlives_beat
# answered that with two walks of the block chain -- k_survives against the
# whole chain, then against the innermost mark. On runbench the decoder's
# arrays outgrow their literal's one slot on the second push, 214,000 times a
# run, and every one of those headers sits in the head block a few bytes below
# the bump pointer, where the answer is a compare against the mark. Since
# 2026-09-07 the head block answers without a walk, as k_born_this_beat's
# does; the walks remain for a header anywhere else. This mutation sends every
# header back through the walks. No counter moves either way -- the answer is
# the same, only how it was reached -- so the work vein is the witness:
# runbench 2,692,921,601 -> 2,674,319,744 on the container when the change
# landed, together with the small-copy loops that landed beside it.
set -e
grep -q '^    if (k_blocks) {$' src/runtime.c || {
  echo "k_outlives_beat's head-block test changed shape; rewrite this" >&2
  exit 1
}
n=$(grep -c '^    if (k_blocks) {$' src/runtime.c)
[ "$n" -eq 1 ] || { echo "expected one head-block test, found $n" >&2; exit 1; }
sed -i 's|^    if (k_blocks) {$|    if (0 \&\& k_blocks) {|' src/runtime.c
grep -q '^    if (0 && k_blocks) {$' src/runtime.c
