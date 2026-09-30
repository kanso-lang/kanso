#!/bin/sh
# A number heading an application of a negative number is let through.
#
# `10 -4` reads as 10 followed by -4 (ruled 2026-09-30), and the parser refuses
# it with the subtraction it meant. Without that, the error corpus's
# a_number_applied_to_a_negative_number reports a different error.
set -e
f=src/parser.rs
grep -q 'if matches!(head, Expr::Int(..) | Expr::Float(..)) {' src/parser.rs
line='        if matches!(head, Expr::Int(..) | Expr::Float(..)) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the number-heads-a-negative test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if matches!(head, Expr::Int(..) | Expr::Float(..)) {$/        if false {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
