#!/bin/sh
# Type a zero divisor's answer as an err, as inference did from the 2026-08-10
# ruling until 2026-09-28. A compiled build then treats a function's parameter
# as never holding it, computes on the string, and the runtime corpus fixture
# arithmetic_on_a_modulo_by_zero_is_refused reads a number where the refusal
# belongs. A remainder by a word is typed as a word, which is what leaves the
# parameter an int and nothing else; a quotient may be a bignum, so the `/`
# fixture's arithmetic tests the tag and refuses either way.
set -e
grep -q '^                "/" | "%" => fails | STR | REC | numeric_result(op, a, b),$' src/infer.rs
sed -i.bak 's#^                "/" | "%" => fails | STR | REC | numeric_result(op, a, b),$#                "/" | "%" => fails | ERR | numeric_result(op, a, b),#' src/infer.rs
rm -f src/infer.rs.bak
grep -q '^                "/" | "%" => fails | ERR | numeric_result(op, a, b),$' src/infer.rs
