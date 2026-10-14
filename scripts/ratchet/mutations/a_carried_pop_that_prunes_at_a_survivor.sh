#!/bin/sh
# The copy-out at a carried pop retires the depth's carry pair, and until
# 2026-09-07 it pruned at any survivor whose immediate interior survived.
# One level down is all that test can see, so a node two levels down holding
# a pointer into the pair was left for the caller's next stage to repair --
# which was the next chain step until a step could leave. It walks deep now
# once an in-place write has put such a pointer there, and `k_carry_written`
# is what says so. This mutation clears that condition, which puts the prune
# back on every pop that did not follow a skipped lap.
#
# The corpus prints the same bytes either way, so the gate is the mem vein.
# a_carried_value_written_into_an_older_node.mem caught it until the lap
# carry, which skips the pop that fixture relied on. Now
# an_inner_beat_opens_its_tenure_in_the_block_outside.mem reads
# survive_slots=2004 with the walk and 4004 without it.
#
# kanso#1300 recorded a segfault on scripts/trend_gate under this mutation.
# It does not reproduce on this tree: the gate's workload is the golden diff
# against a base, and five bases were tried on 2026-09-07 with the mutation
# applied and none faulted. The hazard is a mechanism, not a program that
# faults on demand, which is why this row is gated on counters.
set -e
n=$(grep -cF 'KCopy cp = { NULL, NULL, 1, 0, k_carry_written || c->skipped };' src/runtime/core.c)
[ "$n" -eq 1 ] || { echo "the pop's copy-out changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/KCopy cp = { NULL, NULL, 1, 0, k_carry_written || c->skipped };/KCopy cp = { NULL, NULL, 1, 0, c->skipped };/' src/runtime/core.c
grep -qF 'KCopy cp = { NULL, NULL, 1, 0, c->skipped };' src/runtime/core.c
