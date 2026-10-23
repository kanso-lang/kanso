#!/bin/sh
# `k_json_escape_into` answers every escape in a sixteen-byte block from the
# block's one mask, storing the rest of the block again behind each escape.
# This mutation stops after the first escape and moves on from there, so the
# next escape loads and classifies a block of its own. The output is the
# same; the witness is the work vein.
set -e
line='            k_json_rest(o, x, at);'
grep -qxF "$line" src/runtime/text.c || {
  echo "the escape writer's rest store moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            k_json_rest(o, x, at);$/            span = at; break;/' src/runtime/text.c
grep -qx '            span = at; break;' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
