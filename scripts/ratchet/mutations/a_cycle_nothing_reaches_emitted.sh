#!/bin/sh
# The prune keeps a definition only when a chain of names reaches it from the
# entry, so a cycle of helpers nothing reaches goes whole. This mutation starts
# every definition marked, which keeps them all. The output is the same; the
# witness is a_cycle_nothing_reaches_is_not_emitted, whose summing program then
# defines std/list's merge.
set -e
grep -qF '    let mut alive = vec![false; blocks.len()];' src/codegen.rs || {
  echo "the prune's mark moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    let mut alive = vec!\[false; blocks.len()\];$|    let mut alive = vec![true; blocks.len()];|' src/codegen.rs
[ "$(grep -cF '    let mut alive = vec![true; blocks.len()];' src/codegen.rs)" -eq 1 ]
