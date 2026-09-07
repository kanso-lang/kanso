#!/bin/sh
# The sizing walk asks of every node whether the arena keeps it, and when the
# arena does not, whether a tenure block does. k_survives walked the block
# chain to answer the first, and k_ten_holds walked it again through
# k_above_mark to rule out a pointer above the mark before it read the tenure
# blocks: two walks of the same chain for one node, 330,000 times a run on
# runbench. Since 2026-09-07 k_where walks it once and answers below, above
# or outside in one pass. This mutation puts the two walks back in front of
# it. The bytes out are the same either way and no counter moves, so the
# work vein is the witness: runbench 2,612,624,701 -> 2,606,982,659 on the
# container when the one walk landed.
set -e
target='    int w = k_where(p, m);'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_survives_x changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^    int w = k_where(p, m);$|    if (k_survives(p, m) \|\| (k_ten_any \&\& k_ten_holds(p, m))) return 1;\n    int w = k_where(p, m);|' src/runtime.c
grep -qF '    if (k_survives(p, m) || (k_ten_any && k_ten_holds(p, m))) return 1;' src/runtime.c
