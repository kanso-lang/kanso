#!/bin/sh
# render_ryu writes a short decimal straight from its scaled integer m and its
# place count p: m / 10^p, a point, and m's last p digits. Since 2026-09-24
# every float the benchmarks render takes that road, so the plain-form branches
# below it, which place the point by walking ryu's digit buffer, are reached by
# none of them -- which is why the row that guarded their copies was retired.
#
# This closes the direct road, sending every short float back through the
# digit buffer. The text is identical, so nothing but the instruction vein can
# see it.
set -e
t='        if (m < 1000000000000000ULL && m * 10000 >= pw) {'
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "render_ryu's direct writer changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's#^        if (m < 1000000000000000ULL \&\& m \* 10000 >= pw) {$#        if (0) {#' src/runtime.c
grep -qF '        if (0) {' src/runtime.c
