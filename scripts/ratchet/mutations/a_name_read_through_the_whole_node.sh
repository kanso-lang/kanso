#!/bin/sh
# A local whose place is kept is answered by `eval` before `eval_node`'s frame
# is built. This mutation makes the test refuse every name, so each one goes
# the long way through the function that holds every expression form. The
# program's output is the same; the interpreted row is the witness.
set -e
kept='            if (3..1 << 16).contains(&v) {'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the front door moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if (3\.\.1 << 16)\.contains(&v) {$/            if (3..1 << 16).contains(\&v) \&\& v == 0 {/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
