#!/bin/sh
# A dispatch ranks its candidates into two `Score`s held on the stack, one for
# the leader and one for the candidate being scored. This mutation gives each a
# heap buffer at the start of every dispatch again, as the `Vec<u8>` they
# replaced had. The arms chosen are the same; the interpreted row is the
# witness.
set -e
kept='        let mut scores: [Score; 2] = Default::default();'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the scores moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let mut scores: \[Score; 2\] = Default::default();$/        let mut scores: [Score; 2] = [(); 2].map(|_| Score { spill: Vec::with_capacity(args.len()), ..Score::default() });/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
