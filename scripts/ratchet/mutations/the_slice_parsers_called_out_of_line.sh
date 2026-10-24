#!/bin/sh
# The two slice parsers the decoder's number twins call have inline copies in
# the release build's hot unit, and the LTO link inlines them into the twins.
# This mutation puts noinline where their always_inline was, and the link
# leaves them as calls to the runtime's own copies. Same answers, and the work
# vein reads runbench heavier.
set -e
sig='KValue k_b_to_%s_slice_raw(const unsigned char* d, long long len, long long from, long long to, const char* origin) {'
for k in int float; do
  t="__attribute__((always_inline)) $(printf "$sig" "$k")"
  [ "$(grep -cxF "$t" src/runtime/numbers.c)" -eq 1 ] || {
    echo "the $k slice parser moved; this mutation needs rewriting" >&2
    exit 1
  }
  sed -i "s/^__attribute__((always_inline)) KValue k_b_to_${k}_slice_raw(/__attribute__((noinline)) KValue k_b_to_${k}_slice_raw(/" src/runtime/numbers.c
  if grep -qxF "$t" src/runtime/numbers.c; then exit 1; fi
  grep -qxF "__attribute__((noinline)) $(printf "$sig" "$k")" src/runtime/numbers.c
done
