#!/bin/sh
# The block index answers the survival questions once the arena's chain is
# longer than K_BIDX_WALK blocks. Raising the threshold past any chain a
# program builds sends every question back to the walk, which gives the same
# answers more slowly, so no output and no allocation moves. `chain_finds` is
# the counter that goes to nought, and
# a_long_chain_answers_survival_from_its_index reads 0 against 477,919.
set -e
grep -q '^#define K_BIDX_WALK 8$' src/runtime.c || {
  echo "the block index threshold changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^#define K_BIDX_WALK 8$|#define K_BIDX_WALK 1000000|' src/runtime.c
grep -q '^#define K_BIDX_WALK 1000000$' src/runtime.c
