#!/bin/sh
# A caller a cycle reaches keeps the cohort bracket on a call that runs a loop
# and answers a scalar. This mutation takes that license back, so the run
# program's index phase keeps its 1.5 MB string to the end. The output is the
# same; the witness is the run program's arena peak.
set -e
line='                    || (self.reaches_cycle.contains(name)'
grep -qxF "$line" src/codegen/emit.rs || {
  echo "the cohort license for calls into a loop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    || (self.reaches_cycle.contains(name)$/                    || (false \&\& self.reaches_cycle.contains(name)/' src/codegen/emit.rs
if grep -qxF "$line" src/codegen/emit.rs; then exit 1; fi
