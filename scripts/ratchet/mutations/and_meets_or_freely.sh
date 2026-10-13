#!/bin/sh
# `and` and `or` meeting without parentheses are refused (2026-10-13, #1887).
# This mutation stops the `and` chain reporting where its first `and` stands,
# so `true and false or true` parses by precedence again.
set -e
line='            first.get_or_insert(span);'
[ "$(grep -cxF "$line" src/parser.rs)" -eq 1 ] || {
  echo "the and chain's report moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            first.get_or_insert(span);$/            let _ = \&mut first;/' src/parser.rs
if grep -qxF "$line" src/parser.rs; then exit 1; fi
