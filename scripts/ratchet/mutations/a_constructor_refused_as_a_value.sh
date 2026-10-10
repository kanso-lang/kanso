#!/bin/sh
# A record constructor used as a value stops lowering to the lambda that calls
# it, so native refuses to build a program the interpreter runs (lox F20).
# The micro corpus's `a_constructor_handed_over_as_a_function` stops building.
set -e
grep -qxF '                            Some(n @ 1..=4) => {' src/codegen.rs || {
  echo "the constructor-as-value arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                            Some(n @ 1..=4) => {$/                            Some(n @ 1..=4) if false => {/' src/codegen.rs
! grep -qxF '                            Some(n @ 1..=4) => {' src/codegen.rs
