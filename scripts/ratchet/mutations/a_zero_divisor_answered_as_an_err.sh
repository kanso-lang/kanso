#!/bin/sh
# Type a zero divisor's answer as an err, as inference did from the 2026-08-10
# ruling until 2026-09-28. A compiled build then treats a function's parameter
# as never holding it, computes on the string, and the runtime corpus fixture
# arithmetic_on_a_division_by_zero_is_refused reads a number where the
# refusal belongs.
set -e
grep -q '^                "/" | "%" => fails | STR | REC | numeric_result(op, a, b),$' src/infer.rs
sed -i.bak 's#^                "/" | "%" => fails | STR | REC | numeric_result(op, a, b),$#                "/" | "%" => fails | ERR | numeric_result(op, a, b),#' src/infer.rs
rm -f src/infer.rs.bak
grep -q '^                "/" | "%" => fails | ERR | numeric_result(op, a, b),$' src/infer.rs
