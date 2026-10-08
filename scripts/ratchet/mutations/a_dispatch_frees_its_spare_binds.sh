#!/bin/sh
# A dispatch hands the bindings buffer its losing candidates matched into back
# to the interpreter, and the next dispatch that needs a second buffer takes it
# from there. This mutation stops the hand-back, so every such buffer goes to
# the allocator at the end of its dispatch as it did before. The arms chosen
# are the same; the interpreted row is the witness.
set -e
kept='                if spare.len() < 64 {'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the hand-back moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                if spare.len() < 64 {$/                if spare.len() == usize::MAX {/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
