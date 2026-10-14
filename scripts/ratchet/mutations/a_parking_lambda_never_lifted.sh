#!/bin/sh
# A lambda whose body is a chain that parks first is lifted into a function
# the pool can work in. This mutation skips the lift, so the call in the mem
# golden runs after the wait and its `ahead_forced=1` goes to 0.
set -e
line='    lift_lambdas(program, &user, &mut made);'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "the lambda lift moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    lift_lambdas(program, &user, &mut made);$/    let _ = lift_lambdas;/' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
