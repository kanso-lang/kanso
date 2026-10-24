#!/bin/sh
# The string arm of an index is defined in the release build's LTO unit and
# inlined into the program's walks over text. This mutation takes its
# always_inline away, and the link leaves it as a call again. Same answers,
# and the work vein reads runbench heavier.
set -e
t='__attribute__((always_inline)) KValue k_b_at_str(KStr* s, long long want) {'
[ "$(grep -cxF "$t" src/runtime/text.c)" -eq 1 ] || {
  echo "the string index moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^__attribute__((always_inline)) KValue k_b_at_str(KStr\* s, long long want) {$/KValue k_b_at_str(KStr* s, long long want) {/' src/runtime/text.c
if grep -qxF "$t" src/runtime/text.c; then exit 1; fi
grep -qxF 'KValue k_b_at_str(KStr* s, long long want) {' src/runtime/text.c
