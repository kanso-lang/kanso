#!/bin/sh
# `&`, `|` and `^` share one precedence, so a chain that mixes them is refused
# until parentheses say which goes first (2026-10-12, #1893). This mutation
# lets the chain through, and `6 & 3 | 8` reads left to right again.
set -e
line='                Some(before) if before != op => {'
[ "$(grep -cxF "$line" src/parser.rs)" -eq 1 ] || {
  echo "the mixed-chain refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                Some(before) if before != op => {$/                Some(before) if false \&\& before != op => {/' src/parser.rs
if grep -qxF "$line" src/parser.rs; then exit 1; fi
