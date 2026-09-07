#!/bin/sh
# The copy-out at a carried pop retires the depth's carry pair, and until
# 2026-09-07 it pruned at any survivor whose immediate interior survived.
# One level down is all that test can see, so a node two levels down
# holding a pointer into the pair was left for the caller's next stage to
# repair -- which was the next chain step until a step could leave. This
# mutation puts the prune back. The bytes out are the same on the corpus
# and the evacuation counters fall, because the walk stops copying what it
# should copy; on scripts/trend_gate the same mutation segfaults.
set -e
n=$(grep -cF 'KCopy cp = { NULL, NULL, 1, 0, 1 };' src/runtime.c)
[ "$n" -eq 1 ] || { echo "the pop's copy-out changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|KCopy cp = { NULL, NULL, 1, 0, 1 };|KCopy cp = { NULL, NULL, 1, 0, 0 };|' src/runtime.c
grep -qF 'KCopy cp = { NULL, NULL, 1, 0, 0 };' src/runtime.c
