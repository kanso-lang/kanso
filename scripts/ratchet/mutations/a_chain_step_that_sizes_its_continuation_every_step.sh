#!/bin/sh
# The bind chain sizes its continuation to choose between leaving it,
# staging it and flooring the region under it, and until 2026-09-07 the
# walk ran on every step, with the leave taken only for a value it had
# already measured at 4 KB or under: deepbench's steps each walked a
# closure over a short list to learn what the previous step had learnt.
# Since then a step whose region has not drifted a quarter megabyte past
# the last staged top leaves without walking. This mutation makes the
# drift test always fail, so every step sizes and stages again. The bytes
# out are the same and the work vein moves.
set -e
anchor='                   once a drift instead of once a step. */'
n=$(grep -cF "$anchor" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the chain step changed shape ($n); rewrite this" >&2; exit 1; }
sed -i '/^                   once a drift instead of once a step\. \*\/$/{n;n;s|^                if (kc->at_blocks == (const void\*)k_blocks$|                if (0 \&\& kc->at_blocks == (const void*)k_blocks|}' src/runtime.c
grep -qF '                if (0 && kc->at_blocks == (const void*)k_blocks' src/runtime.c
