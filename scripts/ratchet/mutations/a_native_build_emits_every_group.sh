#!/bin/sh
# Every build emits only the groups its entry reaches: the tab since
# 2026-10-08, native builds since 2026-10-09. This mutation empties the walk's
# answer, so every group the program imports is emitted and pruned afterwards;
# the module is the same and the emit row is the witness.
set -e
kept='        let reached = reached_from_entry(owned);'
[ "$(grep -cxF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the walk moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let reached = reached_from_entry(owned);$/        let reached = reached_from_entry(owned).filter(|_| false);/' src/codegen.rs
if grep -qxF "$kept" src/codegen.rs; then exit 1; fi
