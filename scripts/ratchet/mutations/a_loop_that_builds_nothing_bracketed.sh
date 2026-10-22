#!/bin/sh
# A cohort under a cycle is licensed only for a call that reaches a cycle that
# allocates. This mutation seeds the reach from every cycle, so a loop that
# builds nothing -- widebench's whitespace skip, run once per value -- takes a
# bracket on every call. The output is the same; the witness is the work vein.
set -e
line='            queue.extend(scc.into_iter().filter(|&at| allocating.contains(names[at])));'
grep -qxF "$line" src/codegen/prune.rs || {
  echo "the allocating seed of reaches_cycle moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            queue.extend(scc.into_iter().filter(|&at| allocating.contains(names\[at\])));$/            queue.extend(scc.into_iter().filter(|\&at| allocating.len() < usize::MAX || allocating.contains(names[at])));/' src/codegen/prune.rs
if grep -qxF "$line" src/codegen/prune.rs; then exit 1; fi
