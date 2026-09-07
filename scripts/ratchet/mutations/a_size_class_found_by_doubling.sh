#!/bin/sh
# A buffer's size class is which free list its capacity belongs to, and
# k_buf_class answered it by doubling from four until it reached the
# capacity: 1,255,865 rounds of a six-instruction loop over runbench's
# 1,431,562 asks, eleven rounds for a capacity of 8,192. Since 2026-09-07
# the trailing zeros of a power of two say the class in one instruction. This
# mutation puts the doubling loop back. The answer is the same either way --
# buf_reuse, allocs and alloc_bytes print the same -- so the work vein is the
# witness: runbench 2,634,857,220 -> 2,621,939,707 on the container when the
# count landed.
set -e
target='    return __builtin_ctzll((unsigned long long)cap) - 2;'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the size class changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^    return __builtin_ctzll((unsigned long long)cap) - 2;$|    { int c = 0; long long size = 4; while (size < cap \&\& c < K_BUF_CLASSES - 1) { size <<= 1; c++; } return size == cap ? c : -1; }|' src/runtime.c
grep -qF 'while (size < cap && c < K_BUF_CLASSES - 1) { size <<= 1; c++; } return size == cap ? c : -1; }' src/runtime.c
