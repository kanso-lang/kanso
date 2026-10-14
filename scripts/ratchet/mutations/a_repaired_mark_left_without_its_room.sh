#!/bin/sh
# A repair copies what a surviving node points at below the mark and raises
# the mark over it. Raising the pointer without the count left the mark
# describing a position that did not exist, and the arena went on to hand out
# memory past the end of a block (kanso#823). This mutation takes the count
# back out of `k_repaired_settle`.
#
# The gate is the mem vein: a_node_repaired_by_an_inner_pop_outlives_the_outer_stage
# exits non-zero without the count, which the corpus reports as a stdout
# mismatch. tests/carry_repair.rs was the guard until 2026-10-11, and its
# program had stopped reaching the repair, so it passed with this line gone.
set -e
grep -qF 'static void k_repaired_settle(KMark* m) {' src/runtime/core.c
sed -i '/^static void k_repaired_settle(KMark\* m) {$/,/^}$/ s/^    m->left = k_arena_left;$/    (void)0;/' src/runtime/core.c
! awk '/^static void k_repaired_settle\(KMark\* m\) \{$/,/^}$/' src/runtime/core.c | grep -qF 'm->left = k_arena_left;'
