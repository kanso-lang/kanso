#!/bin/sh
# The two slice parsers the decoder's number twins call are defined in the
# release build's LTO unit and inlined into the twins. This mutation takes
# their always_inline away, and the link leaves them as calls again. Same
# answers, and the work vein reads runbench heavier.
set -e
sig='KValue k_b_to_%s_slice_raw(const unsigned char* d, long long len, long long from, long long to, const char* origin) {'
for k in int float; do
  t="__attribute__((always_inline)) $(printf "$sig" "$k")"
  [ "$(grep -cxF "$t" src/runtime/numbers.c)" -eq 1 ] || {
    echo "the $k slice parser moved; this mutation needs rewriting" >&2
    exit 1
  }
  sed -i "s/^__attribute__((always_inline)) KValue k_b_to_${k}_slice_raw(/KValue k_b_to_${k}_slice_raw(/" src/runtime/numbers.c
  if grep -qxF "$t" src/runtime/numbers.c; then exit 1; fi
  grep -qxF "$(printf "$sig" "$k")" src/runtime/numbers.c
done
