#!/bin/sh
# The interpreter points an err an insisting form answers at the line that
# called it. This mutation leaves the err as the form made it, so the oracle
# names the library's line and the engines disagree.
set -e
line='        Value::ErrV(info) if info.hako.as_deref() == Some(&**hako) => {'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "restamped moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        Value::ErrV(info) if info.hako.as_deref() == Some(&\*\*hako) => {$/        Value::ErrV(info) if false \&\& info.hako.as_deref() == Some(\&**hako) => {/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
