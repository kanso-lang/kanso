#!/bin/sh
# A beat whose result is a heap value hands its tenure block to the depth
# outside at every pop. Under an outer loop that is one 256 KiB block a lap,
# each holding a few kilobytes: runbench's inner loop handed forty-nine up in
# one phase, for 78 KB of tenured bytes, and every ask of k_ten_holds walked
# all forty-nine. Since 2026-09-07 an inner beat that has no block yet opens
# its tenure in the outer depth's block instead, while that block has room,
# so the outer depth ends the phase holding one block instead of forty-nine.
# This mutation makes every inner beat open its own block again. ten_blocks
# in bench/cost_golden_run.txt reads 6 with the sharing and 55 without it,
# and the run program counters go red on that row; the mem fixture
# an_inner_beat_opens_its_tenure_in_the_block_outside pins the same shape at
# five laps.
set -e
grep -q '^    if (!b && d > 0) {$' src/runtime.c || {
  echo "k_ten_alloc's parent-block opening changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    if (!b && d > 0) {$|    if (0 \&\& !b \&\& d > 0) {|' src/runtime.c
grep -q '^    if (0 && !b && d > 0) {$' src/runtime.c
