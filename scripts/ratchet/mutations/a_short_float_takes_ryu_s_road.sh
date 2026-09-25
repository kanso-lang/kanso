#!/bin/sh
# ryu_d2d finds a short decimal by scaling f by each power of ten in turn and
# dividing the rounded product back, and hands ryu only the floats that search
# cannot settle. Since 2026-09-24 every float the benchmarks render takes the
# short road, which is why the general loop's own mutation was retired: no
# benchmark reaches it.
#
# This closes the short road. The text out is the same either way -- ryu
# computes the same digits -- so no output or allocation moves, and the
# `ryu_short` presence counter going to nought is what the row asserts.
set -e
t='    if ((uint32_t)(ieee_e - 1003) >= 70) return 0;'
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the short float path changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's#^    if ((uint32_t)(ieee_e - 1003) >= 70) return 0;$#    return 0;#' src/runtime.c
! grep -qF "$t" src/runtime.c
