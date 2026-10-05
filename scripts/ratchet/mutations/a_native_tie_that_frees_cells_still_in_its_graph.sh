#!/bin/sh
# Native `tie` returns its cells to the free list without first rewriting the
# fields that hold them.
#
# The walk is what makes releasing the cells safe: every field a maker filled
# with a reference holds the node itself by the time the cell is freed. Skip it
# and the ring's first step reads a released blackhole, so the native half of
# the micro corpus stops where the interpreter prints the walk.
set -e
grep -qF 'for (long long i = 0; i < n; i++) k_tie_resolve(k_b_at(nodes, l->items[i]));' src/runtime.c || {
  echo "the tie's resolve loop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    for (long long i = 0; i < n; i++) k_tie_resolve(k_b_at(nodes, l->items\[i\]));$/    (void)k_tie_resolve;/' src/runtime.c
grep -qF '    (void)k_tie_resolve;' src/runtime.c
