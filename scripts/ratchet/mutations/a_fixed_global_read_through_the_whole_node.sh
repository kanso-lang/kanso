#!/bin/sh
# A global name whose slot holds a function reference or a literal word is
# answered by `eval` from a table of fixed values, before `eval_node` builds its
# frame. This mutation leaves the table empty, so every global goes the long
# way again. The program's output is the same; the interpreted row is the
# witness.
set -e
kept='                        Named::FnRef(n) => Some(Value::FnRef(n.clone())),'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the fixed table moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                        Named::FnRef(n) => Some(Value::FnRef(n\.clone())),$/                        Named::FnRef(n) => Some(Value::FnRef(n.clone())).filter(|_| false),/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
