#!/bin/sh
# The interpreter calls a lambda in tail position as a nested call.
#
# A tail call to a function value hands the closure back to the loop that
# called the body, as a call to a named group does. Without that, a loop
# written in continuation-passing style takes a Rust frame every step and
# a_lambda_called_in_tail_position_costs_no_stack runs out of stack.
set -e
f=src/eval.rs
grep -q "fn closure_flow" src/eval.rs
arm='(None, Value::Closure(c)) => Ok(Flow::Closure(c, values, *span)),'
[ "$(grep -cF "$arm" "$f")" -eq 2 ] || {
  echo "the tail call's closure arms moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/(None, Value::Closure(c)) => Ok(Flow::Closure(c, values, \*span)),/d' "$f"
[ "$(grep -cF "$arm" "$f")" -eq 0 ]
