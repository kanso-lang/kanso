#!/bin/sh
# The effect lines of a body make one group, and a side of it that begins
# with a sleep has continuations the pass lifts from. This mutation stops
# looking inside the group: nothing is lifted, the pool runs empty, and the
# mem golden's `ahead_forced=1` goes to 0.
set -e
line='        Expr::Join { lhs, rhs, .. } => side_parks(lhs) || side_parks(rhs),'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "side_parks's group arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        Expr::Join { lhs, rhs, .. } => side_parks(lhs) || side_parks(rhs),$/        Expr::Join { .. } => false,/' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
