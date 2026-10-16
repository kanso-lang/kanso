#!/bin/sh
# The interpreter places a failure in the frame of the expression it leaves.
# This mutation stops placing it there, so the oracle's runtime goldens lose
# their `failed in` line.
set -e
line='        self.eval_node_inner(expr, env, frame).map_err(|e| placed(e, frame))'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "eval_node's placing moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        self.eval_node_inner(expr, env, frame).map_err(|e| placed(e, frame))$/        self.eval_node_inner(expr, env, frame)/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
