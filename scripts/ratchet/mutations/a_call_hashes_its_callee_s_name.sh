#!/bin/sh
# A call through a function reference finds its callee at the reference's
# number. This mutation finds it by hashing the name again, as call_named
# does. The answer is the same, since both memories hold the same callee; the
# interpreted run's instruction row sees the hashing.
set -e
line='        let callee = self.callee_of_ref(name, at);'
[ "$(grep -cxF "$line" src/eval/calls.rs)" -eq 1 ] || {
  echo "the call by reference moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let callee = self.callee_of_ref(name, at);$/        let callee = { let _ = at; self.callee_named(name) };/' src/eval/calls.rs
grep -qxF '        let callee = { let _ = at; self.callee_named(name) };' src/eval/calls.rs
