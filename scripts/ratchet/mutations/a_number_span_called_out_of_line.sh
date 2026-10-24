#!/bin/sh
# The word-at-a-time number span has an inline copy in the release build's
# hot unit, and the LTO link inlines it into the decoder's walk. This mutation
# puts noinline where its always_inline was, and the link leaves it as a call
# to the runtime's own copy. Same answers, and the work vein reads runbench
# heavier.
set -e
t='__attribute__((always_inline)) long long k_b_number_span_raw(const unsigned char* d, long long len, long long from) {'
[ "$(grep -cxF "$t" src/runtime/text.c)" -eq 1 ] || {
  echo "the number span moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^__attribute__((always_inline)) long long k_b_number_span_raw(/__attribute__((noinline)) long long k_b_number_span_raw(/' src/runtime/text.c
if grep -qxF "$t" src/runtime/text.c; then exit 1; fi
grep -qxF '__attribute__((noinline)) long long k_b_number_span_raw(const unsigned char* d, long long len, long long from) {' src/runtime/text.c
