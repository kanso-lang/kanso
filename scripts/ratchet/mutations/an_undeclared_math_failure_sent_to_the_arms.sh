#!/bin/sh
# The interpreter hands an operator's arms a zero divisor's wrapped answer
# where the program never declared the wrapper.
#
# A zero divisor answers the text, wrapped in `divide_by_zero` only where the
# program names that type or `math_failure`. The interpreter builds the wrapper
# either way and takes it off where a value is about to reach an operator's
# arms, as native never builds it. Told to keep it, the interpreter sends the
# wrapped answer to a user `+` arm, and the runtime corpus fixture is refused
# in the arms' words instead of the builtin's.
set -e
f=src/eval.rs
grep -q "fn as_declared_math(&self, value: Value) -> Value" src/eval.rs
line='            None => bare_math(value),'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the unwrap moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            None => bare_math(value),$/            None => value,/' "$f"
grep -qxF '            None => value,' "$f"
