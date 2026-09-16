#!/bin/sh
# A deferred binding handed back through a pass-through arm can be the next
# call's deferred binding, so what a cell evaluates to may be another cell.
# Until 2026-09-16 k_force_slow stored that cell as the answer and the
# dispatcher on the far side tested its tag against `true` and `false`:
# native said `no overload of keep matches these arguments` where the
# interpreter answered. The force walks the chain now. This mutation takes
# the walk out, so a cell's answer is stored as it came.
#
# The gate is the micro corpus:
# a_deferred_answer_that_defers_again_forces_to_a_value prints its two lists
# on native with the walk and dies at `keep` without it.
set -e
grep -qF '    answer = k_force(answer);' src/runtime.c
sed -i 's/^    answer = k_force(answer);$/    (void)0;/' src/runtime.c
! grep -qF '    answer = k_force(answer);' src/runtime.c
