#!/bin/sh
# The runtime leaves stdout in stdio's buffer until the program exits.
#
# Then text written to stdout before a write to stderr arrives after it on
# a shared pipe, and a program killed while it sleeps loses what it printed.
# stdout_reaches_its_reader sees all three cases.
set -e
f=src/runtime.c
grep -q "static void k_out_settle" src/runtime.c
line='static void k_out_settle(void) { fflush(stdout); }'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "k_out_settle moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^static void k_out_settle(void) { fflush(stdout); }$/static void k_out_settle(void) {}/' "$f"
grep -qxF 'static void k_out_settle(void) {}' "$f"
