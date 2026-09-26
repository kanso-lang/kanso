#!/bin/sh
# k_float_shape8 takes the run of digits at the end of the number as its
# fraction and needs a dot in front of it. Without that test any byte stands
# in for the dot, and "1e1" is read as 1.1.
set -e
t="    if (f < 1 || f > 7 || stop[-f - 1] != '.') return 0;"
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the float word's dot test changed shape ($n); rewrite this" >&2; exit 1; }
sed -i "s/    if (f < 1 || f > 7 || stop\[-f - 1\] != '.') return 0;/    if (f < 1 || f > 7 || stop[-f - 1] == 0) return 0;/" src/runtime.c
grep -qF '    if (f < 1 || f > 7 || stop[-f - 1] == 0) return 0;' src/runtime.c
