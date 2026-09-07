#!/bin/sh
# `utf8` over a slice of bytes is the decoder's way of making a token into a
# string, and 840,807 of runbench's 861,498 slices are four to seven bytes:
# a key, a short value, a number's digits. Each went through k_str_n's memcpy,
# fifteen instructions for glibc to choose how to move five bytes. Since
# 2026-09-07 a slice of four to seven bytes is copied as two overlapping
# words with no call. This mutation sends every slice back through the call.
# The bytes out are the same either way and no counter moves, so the work
# vein is the witness: runbench 2,665,704,368 -> 2,649,866,050 on the
# container when the words landed.
set -e
target='    if (len >= 4 && len < 8) {'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the slice's word copy changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^    if (len >= 4 \&\& len < 8) {$|    if (0 \&\& len >= 4 \&\& len < 8) {|' src/runtime.c
grep -qF '    if (0 && len >= 4 && len < 8) {' src/runtime.c
