#!/bin/sh
# A division by a nonzero literal is read as one that might fail.
#
# Inference gives `/` and `%` an err only when the divisor is not a literal
# other than zero. With the arm gone, every remainder by a literal leaves a
# possible err in whatever parameter it reaches, that parameter's entry test
# comes back, and the benchmarks' emitted code grows by the tests.
set -e
f=src/infer.rs
grep -q 'fn nonzero_literal(e: &Expr)' src/infer.rs
line='                "/" | "%" if nonzero_literal(rhs) => fails | numeric_result(a, b),'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the literal divisor's arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                "\/" | "%" if nonzero_literal(rhs) => fails | numeric_result(a, b),$/                "\/" | "%" if nonzero_literal(rhs) => fails | ERR | numeric_result(a, b),/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
