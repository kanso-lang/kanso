#!/bin/sh
# Native code calls a lambda in tail position with a plain call.
#
# A tail call to a function value held in a local jumps through `k_tcall{n}`,
# which jumps on to the lambda's body. Without it, a loop written in
# continuation-passing style takes a native frame every step and
# a_lambda_called_in_tail_position_costs_no_stack ends in a segmentation fault.
set -e
f=src/codegen/emit.rs
grep -q "fn emit_value_tail_call" src/codegen/emit.rs
line='        if self.emit_value_tail_call(f, expr)? {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the value tail call moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if self\.emit_value_tail_call(f, expr)? {$/        if false \&\& self.emit_value_tail_call(f, expr)? {/' "$f"
grep -qxF '        if false && self.emit_value_tail_call(f, expr)? {' "$f"
