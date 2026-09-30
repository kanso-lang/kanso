#!/bin/sh
# The interpreter hands an operator's arms a zero divisor's wrapped answer
# where the program never declared the wrapper.
#
# A zero divisor answers the text, wrapped in `divide_by_zero` only where the
# program names that type or `math_failure`. The interpreter builds the wrapper
# either way and unwraps it after `/` and `%` where the program named neither,
# as native never builds it. Told the program always named it, the interpreter
# sends the wrapped answer to a user `+` arm, and the runtime corpus fixture is
# refused in the arms' words instead of the builtin's.
set -e
f=src/eval.rs
grep -q "pub fn as_declared_math(answer: Value, declared: bool) -> Value" src/eval.rs
line='                        as_declared_math(answer, self.type_decl(crate::DIVIDE_BY_ZERO).is_some())'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the unwrap call moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                        as_declared_math(answer, self\.type_decl(crate::DIVIDE_BY_ZERO)\.is_some())$/                        as_declared_math(answer, true)/' "$f"
grep -qxF '                        as_declared_math(answer, true)' "$f"
