#!/bin/sh
# k_alloc inlines into every hot caller, and its counter gate was written
# as `!= 0` around a second `if`: a compare and two branches at every
# allocation, one of them for the -1 the switch has not held since the
# constructor began setting it before main. Since 2026-09-07 it makes the
# same `> 0` test every other counting site makes. Since 2026-09-13 it asks
# `K_COUNTING` in front of that, so a shipped binary carries no gate at all
# and this mutation patches the counting half of the condition. This
# mutation puts the two-branch gate back. The bytes out are the same and no counter moves --
# both gates count exactly when the counters are on -- so the work vein is
# the witness: runbench 2,466,456,226 -> 2,453,160,233 on the container.
set -e
anchor='       allocations a run on the run program, one instruction each. */'
n=$(grep -cF "$anchor" src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_alloc changed shape ($n); rewrite this" >&2; exit 1; }
sed -i '/^       allocations a run on the run program, one instruction each\. \*\/$/{n;s|^    if (__builtin_expect(K_COUNTING \&\& k_stats_on > 0, 0)) {$|    if (__builtin_expect(K_COUNTING \&\& k_stats_on != 0, 0)) if (k_stats_on) {|}' src/runtime.c
grep -qF '    if (__builtin_expect(K_COUNTING && k_stats_on != 0, 0)) if (k_stats_on) {' src/runtime.c
