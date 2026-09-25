#!/bin/sh
# A call through a function reference finds its callee by the reference's
# address. This mutation finds it by hashing the name again, as call_named
# does. The answer is the same, since both memories hold the same callee; the
# interpreted run's instruction row sees the hashing.
set -e
line='        let callee = self.callee_of_ref(name);'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the call by reference moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let callee = self.callee_of_ref(name);$/        let callee = self.callee_named(name);/' src/eval.rs
grep -qxF '        let callee = self.callee_named(name);' src/eval.rs
