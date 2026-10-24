#!/bin/sh
# The word-at-a-time number span is defined in the release build's LTO unit
# and inlined into the decoder's walk. This mutation takes its always_inline
# away, and the link leaves it as a call again. Same answers, and the work
# vein reads runbench heavier.
set -e
t='__attribute__((always_inline)) long long k_b_number_span_raw(const unsigned char* d, long long len, long long from) {'
[ "$(grep -cxF "$t" src/runtime/text.c)" -eq 1 ] || {
  echo "the number span moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^__attribute__((always_inline)) long long k_b_number_span_raw(/long long k_b_number_span_raw(/' src/runtime/text.c
if grep -qxF "$t" src/runtime/text.c; then exit 1; fi
grep -qxF 'long long k_b_number_span_raw(const unsigned char* d, long long len, long long from) {' src/runtime/text.c
