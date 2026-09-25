#!/bin/sh
# A map of eight entries that is given a ninth becomes a B-tree. This mutation
# builds the tree from the eight and drops the ninth, so the interpreter's map
# loses the key that made it grow while the native engine's keeps it.
#
# a_map_crosses_eight_entries prints nine as eight entries on the interpreter.
set -e
line='                    many.insert(key, value);'
test "$(grep -cxF "$line" src/eval.rs)" = 1 || {
  echo "the map's growth into a tree changed shape; rewrite this" >&2
  exit 1
}
grep -vxF "$line" src/eval.rs > src/eval.rs.new
mv src/eval.rs.new src/eval.rs
test "$(grep -cxF "$line" src/eval.rs)" = 0
