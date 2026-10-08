#!/bin/sh
# On wasm32 a block of 512 bytes or fewer goes back to a free list kept for
# its size and is handed out again before dlmalloc is asked. This mutation
# sets the ceiling to nothing, so every block goes to dlmalloc and back. The
# answers are the same; the browser compile row is the witness.
set -e
kept='pub const MAX: usize = 512;'
[ "$(grep -cF "$kept" src/main.rs)" -eq 1 ] || {
  echo "the small-block ceiling moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/pub const MAX: usize = 512;/pub const MAX: usize = 0;/' src/main.rs
if grep -qF "$kept" src/main.rs; then exit 1; fi
