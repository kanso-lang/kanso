#!/bin/sh
# Native's integer `%` floors where the oracle truncates, so `-7 % 3` answers
# 2 on one engine and -1 on the other. Both are a language's defensible choice
# — C and Rust truncate, Python and Haskell floor — and kanso may only have
# one of them.
#
# Nothing else in the board can see this. Every allocation counter is flat, no
# diagnostic is raised so `diagnostic_differential` has nothing to compare, and
# no golden in the corpus prints a negative modulo. What differs is an ANSWER,
# on one operator, at the operands the numeric sweep exists to straddle.
set -e
target='return k_int(a.payload % b.payload);'
grep -qF "$target" src/runtime.c || {
  echo "integer modulo moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|return k_int(a.payload % b.payload);|return k_int(((a.payload % b.payload) + b.payload) % b.payload);|' \
  src/runtime.c
grep -qF 'return k_int(((a.payload % b.payload) + b.payload) % b.payload);' src/runtime.c
