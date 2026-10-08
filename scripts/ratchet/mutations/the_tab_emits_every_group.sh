#!/bin/sh
# The tab emits only the groups its entry reaches; the prune drops the rest
# from the module either way. This mutation never walks, so every group is
# emitted again, the module is the same, and the browser compile row is the
# witness.
set -e
kept='        true => reached_from_entry(program),'
[ "$(grep -cxF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the walk moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        true => reached_from_entry(program),$/        true => None,/' src/codegen.rs
if grep -qxF "$kept" src/codegen.rs; then exit 1; fi
