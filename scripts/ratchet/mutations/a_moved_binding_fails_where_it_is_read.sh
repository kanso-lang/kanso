#!/bin/sh
# A moved binding that dies in the pool is forced again outside the try, so
# its failure is raised at the park. This mutation drops that second force:
# the pool stops, the cell is left unbuilt, and the failure comes where the
# value is read, after "first" has printed.
set -e
line='                let died = if *moved { format!("again{i}") } else { "done".to_string() };'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the pool's moved-binding branch moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                let died = if \*moved { format!("again{i}") } else { "done".to_string() };$/                let died = if false { format!("again{i}") } else { "done".to_string() };/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
