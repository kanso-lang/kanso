#!/bin/sh
# An operator's arm is given a raw int slot.
#
# Inference widens an operator arm's parameters with the operands it meets,
# so a parameter that only ever meets ints is proven an int. The operator's
# dispatch still hands the arm both values boxed, so codegen keeps operator
# groups off the unboxed convention. Without that, the arm for `money * n`
# is defined with an i64 slot and called with a boxed value: the wasm
# translator refuses the module for an_operator_arm_on_a_subtype_of_int.
set -e
f=src/codegen.rs
grep -q "fn unboxed_param" src/codegen.rs
line='        self.group_param_set(name, arity, param) == INT && !crate::is_operator(name)'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the operator exclusion in unboxed_param moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        self\.group_param_set(name, arity, param) == INT \&\& !crate::is_operator(name)$/        self.group_param_set(name, arity, param) == INT/' "$f"
grep -qxF '        self.group_param_set(name, arity, param) == INT' "$f"
