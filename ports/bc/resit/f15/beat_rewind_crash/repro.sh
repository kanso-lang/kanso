#!/bin/sh
# Native builds of this snapshot of the port (dev and --release alike)
# segfault on input.bc; the interpreter runs it correctly. gdb:
#
#   #0 __GI___libc_free (mem=0xfffffffffffffffa)
#   #1 k_beat_rewind_slow ()
#   #2 k_beat_pop_slow ()
#   #3 k_exec ()
#
# Expected (what --interp prints, and GNU bc agrees):
#   stderr: Runtime error (func=(main), adr=0): Function gcd not defined.
#           input.bc 5: syntax error
#   exit 0
# Actual: both lines, then SIGSEGV (exit 139).
#
# The crash is sensitive to the input's shape: renaming `ack` to `a`, or
# dropping the last line, makes it go away. Rewriting the driver to do no
# I/O until the end did not help. What did was building the parameter list
# in bc/parser.kso `formals` without a push-accumulating loop; see
# bugs/loop_list_clobbered, which is the same corruption caught earlier
# (FRICTION F15).
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")" || exit 1
"$KANSO" build main.kso > /dev/null || exit 1
./main input.bc < /dev/null
echo "native exit $?"
"$KANSO" run main.kso --interp -- input.bc < /dev/null
echo "interpreter exit $?"
rm -f main main.ll
