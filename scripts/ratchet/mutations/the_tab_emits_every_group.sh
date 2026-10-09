#!/bin/sh
# The tab emits only the groups its entry reaches; the prune drops the rest
# from the module either way. Since 2026-10-09 native builds take the same
# walk on the same line, so this mutation and
# a_native_build_emits_every_group make one edit and each has its own
# witness: here the browser compile row. It keeps the call because a walk
# nothing calls is dead code, which the build refuses.
set -e
kept='        let reached = reached_from_entry(owned);'
[ "$(grep -cxF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the walk moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let reached = reached_from_entry(owned);$/        let reached = reached_from_entry(owned).filter(|_| false);/' src/codegen.rs
if grep -qxF "$kept" src/codegen.rs; then exit 1; fi
