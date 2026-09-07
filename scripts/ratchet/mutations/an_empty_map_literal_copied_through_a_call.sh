#!/bin/sh
# The decoder opens an object with the empty map literal, 273,339 times a run
# on runbench, and the literal copied its pairs -- none -- through glibc's
# memcpy, thirteen instructions to learn it had nothing to move. Since
# 2026-09-07 a literal of two pairs or fewer copies by a loop, the split
# k_rec and k_mklist make. This mutation sends every literal back through
# the call. The bytes out are the same either way and no counter moves, so
# the work vein is the witness: runbench 2,638,957,305 -> 2,634,857,220 on
# the container when the split landed.
set -e
target='    if (n <= 2) {'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the map literal's small copy changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^    if (n <= 2) {$|    if (0 \&\& n <= 2) {|' src/runtime.c
grep -qF '    if (0 && n <= 2) {' src/runtime.c
