#!/bin/sh
# An allocation larger than a block gets a block of its own, and the block
# the arena was bumping in stays the bump region for what comes after. This
# mutation raises the smallest tail worth keeping past any block, so every
# oversize allocation leaves the next small one to open a fresh block. The
# witness is the fixture an_oversize_string_leaves_its_neighbour_open, whose
# arena_peak_bytes counts a second 1 MiB block.
set -e
grep -qF '#define K_TAIL_MIN 4096' src/runtime.c || {
  echo "the tail threshold moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^#define K_TAIL_MIN 4096$|#define K_TAIL_MIN (1LL << 40)|' src/runtime.c
[ "$(grep -cF '#define K_TAIL_MIN (1LL << 40)' src/runtime.c)" -eq 1 ]
