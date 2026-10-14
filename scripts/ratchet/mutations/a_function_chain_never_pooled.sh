#!/bin/sh
# A function whose chain begins with a sleep binds each certain call on its
# locals as a cell the sleep hands to the pool. This mutation turns the pass
# off, so the call runs after the wait, and the mem golden's `ahead_forced=1`
# goes to 0.
set -e
line='    program.fns.iter().any(|f| poolable(program, f))'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "the pool's gate moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    program\.fns\.iter()\.any(|f| poolable(program, f))$/    false \&\& program.fns.iter().any(|f| poolable(program, f))/' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
