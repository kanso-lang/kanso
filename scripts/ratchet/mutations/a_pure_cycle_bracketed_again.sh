#!/bin/sh
# The allocation classifier tells an append the linearity analysis proved in
# place — storage grown outside the arena, nothing for a rewind to free —
# from one that copies, since 2026-09-07. Before that every append read as
# an allocation, so a cycle that only appends into a builder was bracketed
# and rewound for nothing: lib/json's escaper paid 0.83% of runbench for it.
# This makes the in-place test never hold, so every such cycle is bracketed
# again. beat_iters is pinned by every .mem golden in the corpus, and the
# fixture that allocates nothing pins zero.
set -e
grep -q "^            && args.len() == 2$" src/beat.rs || {
  echo "the in-place append test changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            && args.len() == 2$/            \&\& args.len() == 3/' src/beat.rs
grep -q "^            && args.len() == 3$" src/beat.rs
