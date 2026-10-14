#!/bin/sh
# A closed call bound ahead of the first park moves into it. This mutation
# finds no park to move into, so the binding runs before the sleep, the pool
# has nothing to force, and the mem golden's `ahead_forced=1` goes to 0.
set -e
line='    let Some(mut park) = first_park(entry) else {'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "the move's park lookup moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let Some(mut park) = first_park(entry) else {$/    let Some(mut park) = None::<usize> else {/' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
