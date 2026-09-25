#!/bin/sh
# Native rounds a half to even where the oracle rounds it away from zero, so
# `math/round 0.5` answers 0 on one engine and 1 on the other. Both answers are
# a defensible reading of "round"; the language may only have one.
#
# This is the shape the behaviour sweep exists for. Nothing complains, so
# `diagnostic_differential` sees nothing. No counter moves and no golden prints
# a half. What differs is the ANSWER at an edge — the one thing that sweep
# asks about — and `math/round 0.5`, `2.5` and `-0.5` are three of its probes.
set -e
grep -q 'llround(k_as_f(v))' src/runtime.c || {
  echo "round's implementation moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's/llround(k_as_f(v))/llrint(k_as_f(v))/' src/runtime.c
rm -f src/runtime.c.bak
grep -q 'llrint(k_as_f(v))' src/runtime.c
