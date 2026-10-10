#!/bin/sh
# A snapshot of the port in which three small parser loops -- skip_newlines,
# skip_separators and recover in bc/parser.kso -- call themselves with the
# whole parser state, a record that holds the token list and three maps.
# The compiler makes them beat loops (`k_beat_iter_carry` in the .ll).
#
# Expected (the interpreter, and GNU bc -l): sixty lines of math-library
# results for input.bc, exit 0.
# Actual, native dev or --release: a segmentation fault inside d_bc/eval_2.
# Under valgrind the last read before the fault is k_b_find2_raw reading
# address 0x51 from bc/in_base -- the `digits` string of a parsed constant
# has become a small integer, and in_base only runs at all because ibase,
# which the library sets to ten around every call, reads as something else.
#
# The port rewrote the three loops to scan positions in the token list and
# move the cursor once (bc/parser.kso, past_kinds and line_end); with no
# other change the crash is gone, and so is a hang seen earlier on another
# input. Deleting any one line of input.bc also makes the crash go away, and
# a small standalone program carrying a map through a beat loop did not
# reproduce it (FRICTION F15).
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")" || exit 1
"$KANSO" build main.kso > /dev/null || exit 1
./main -l input.bc < /dev/null > /dev/null
echo "native exit $?"
"$KANSO" run main.kso --interp -- -l input.bc < /dev/null | tail -1
echo "interpreter exit $?"
rm -f main main.ll
