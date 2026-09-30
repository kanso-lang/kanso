#!/bin/sh
# Rank a NaN level with every float, as C's comparisons did before the
# 2026-09-30 float order: the native build then sorts a NaN wherever it lands
# and answers it from list/min, and the micro corpus's
# every_nan_is_one_value_and_zero_is_one_value parts from the interpreter.
set -e
old='    if (xn || yn) return xn - yn;'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    if (xn || yn) return xn - yn;$#    if (xn || yn) return 0;#' src/runtime.c
rm -f src/runtime.c.bak
grep -qxF '    if (xn || yn) return 0;' src/runtime.c
