#!/bin/sh
# The interpreter places a failure as it leaves the body that raised it, since
# every expression in a body shares the body's frame. This mutation stops
# placing it there, so the oracle's runtime goldens lose their `failed in`
# line.
set -e
line='        self.eval_body_flow_unplaced(decl, env).map_err(|e| placed(e, &self.frame_for(decl)))'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the body's placing moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        self.eval_body_flow_unplaced(decl, env).map_err(|e| placed(e, \&self.frame_for(decl)))$/        self.eval_body_flow_unplaced(decl, env)/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
