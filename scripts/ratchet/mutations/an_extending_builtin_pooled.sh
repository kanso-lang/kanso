#!/bin/sh
# `push` may write into the list it was given, so a pooled call may not apply
# it. This mutation admits it, and the mem golden that pushes inside a
# report's argument reads `ahead_forced=1` where it pins 0.
set -e
line='    "length", "keys", "values", "entries", "chars", "bytes", "char_code", "from_code", "to_int",'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "the value builtins moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    "length", "keys", /    "push", "length", "keys", /' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
