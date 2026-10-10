#!/bin/sh
# A snapshot of the port whose native build corrupts a list. input.bc
# defines a function with one parameter and four autos and calls it.
#
# Expected (the interpreter, and GNU bc): prints 0.
# Actual, native dev or --release build, input on standard input:
#   error[runtime]: no overload of `bc/current_of` matches these arguments
# and in neighbouring versions of the code a SIGSEGV instead. Reading the
# definition back right after parsing it shows the autos list intact; after
# the function body has been parsed (another tail-recursive loop that
# pushes into a list), rendering the autos list crashes.
#
# The list is built in bc/parser.kso `formals`, a tail-recursive loop that
# pushes each formal into an accumulator and returns it inside a `parsed`
# record. The port now builds that list on the way back out of a non-tail
# recursion instead (`more_formals`), and the corruption is gone. With the
# same input given as a file argument instead of on standard input this
# snapshot prints 0, so the trigger is sensitive to allocation layout
# (FRICTION F15).
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")" || exit 1
"$KANSO" build main.kso > /dev/null || exit 1
./main < input.bc
echo "native exit $?"
"$KANSO" run main.kso --interp < input.bc
echo "interpreter exit $?"
rm -f main main.ll
