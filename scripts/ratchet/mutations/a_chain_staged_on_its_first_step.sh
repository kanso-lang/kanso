#!/bin/sh
# A bind chain measures its drift from where it began, so a first step that
# allocated little leaves its value where it is. This mutation forgets where
# the chain began, so every chain's first step sizes and stages its value
# again. The output is the same; the witness is the run program's counters.
set -e
line='            k_carries[k_beat_depth - 1].at_arena = k_arena;'
grep -qxF "$line" src/runtime/effects.c || {
  echo "the chain's starting point moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            k_carries\[k_beat_depth - 1\].at_arena = k_arena;$/            k_carries[k_beat_depth - 1].at_arena = NULL;/' src/runtime/effects.c
if grep -qxF "$line" src/runtime/effects.c; then exit 1; fi
