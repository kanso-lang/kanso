#!/bin/sh
# A dispatch ranks its candidates into a `Score` held on the stack. This
# mutation gives the score a heap buffer at the start of every dispatch again,
# as the `Vec<u8>` it replaced had. The arms chosen are the same; the
# interpreted row is the witness.
set -e
kept='        let mut score = Score::default();'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the score moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let mut score = Score::default();$/        let mut score = Score { spill: Vec::with_capacity(args.len()), ..Score::default() };/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
