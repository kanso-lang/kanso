#!/bin/sh
# The interpreter's tail evaluator keeps a guard's tail position since
# 2026-09-16: the lines under `return x if c` hand their last call back to the
# dispatcher's loop. This puts the old shape back, evaluating the rest as a
# nested block, so every turn of a guarded loop nests a frame and the fixture
# runs the oracle out of stack past ten thousand elements.
set -e
grep -q '                Value::False => self.eval_stmts_flow(rest, env, frame),' src/eval.rs
sed -i 's|                Value::False => self.eval_stmts_flow(rest, env, frame),|                Value::False => Ok(Flow::Done(self.eval_stmts(rest, env, frame)?)),|' src/eval.rs
grep -q 'Value::False => Ok(Flow::Done(self.eval_stmts(rest, env, frame)?)),' src/eval.rs
