#!/bin/sh
# A built binary does not watch its stack.
#
# Without the handler a recursion deeper than the stack ends a binary run
# directly in a bare segmentation fault with nothing on stderr.
# a_built_binary_names_its_stack_overflow reads the binary's stderr.
set -e
f=src/runtime.c
grep -q "static void k_watch_the_stack" src/runtime.c
line='    k_watch_the_stack();'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the stack watch moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    k_watch_the_stack();$/d' "$f"
[ "$(grep -cxF "$line" "$f")" -eq 0 ]
