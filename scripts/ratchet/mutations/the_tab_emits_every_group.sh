#!/bin/sh
# The tab emits only the groups its entry reaches; the prune drops the rest
# from the module either way. This mutation walks and throws the answer away,
# so every group is emitted again, the module is the same, and the browser
# compile row is the witness. It keeps the call because a walk nothing calls
# is dead code, which the build refuses.
set -e
kept='            true => reached_from_entry(owned),'
[ "$(grep -cxF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the walk moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            true => reached_from_entry(owned),$/            true => reached_from_entry(owned).filter(|_| false),/' src/codegen.rs
if grep -qxF "$kept" src/codegen.rs; then exit 1; fi
