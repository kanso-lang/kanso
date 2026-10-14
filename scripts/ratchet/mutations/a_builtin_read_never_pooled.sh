#!/bin/sh
# A pooled call may apply a builtin that reads a value, such as `length`, to
# its arguments. This mutation refuses every builtin there, so the call in
# the mem golden runs after the wait and its `ahead_forced=1` goes to 0.
set -e
line='            (is_user_call(head, args.len(), user) || reads_a_value(head, args.len()))'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "the pool's builtin admission moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/ || reads_a_value(head, args\.len()))$/ || false \&\& reads_a_value(head, args.len()))/' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
