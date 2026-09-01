#!/bin/sh
# The arena hands out 1 MiB blocks, and that figure was chosen by measurement:
# kanso#1179 timed the depth loop against block size and kept the number it
# kept for a reason. Doubling it coarsens the high water mark without changing
# a single allocation total: every benchmark allocates exactly the same bytes,
# so allocs and alloc_bytes stay byte-identical and only what the program
# HOLDS moves.
#
# Proved on 2026-09-01 against bench/cost_golden_digest.txt: arena_blocks
# 79 -> 40 and arena_peak_bytes 82,837,504 -> 83,886,080, with every other
# row unchanged. That is the dimension digestbench exists to watch — its peak
# is the message it is hashing rather than the size of its own text, so it is
# the only vein whose headline row a change like this can reach.
set -e
grep -q 'k_arena_push(n > (1 << 20) ? n : (size_t)(1 << 20));' src/runtime.c || {
  echo "the refill block size moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/k_arena_push(n > (1 << 20) ? n : (size_t)(1 << 20));/k_arena_push(n > (1 << 21) ? n : (size_t)(1 << 21));/' src/runtime.c
grep -q '(size_t)(1 << 21)' src/runtime.c
