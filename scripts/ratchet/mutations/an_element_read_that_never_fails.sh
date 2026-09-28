#!/bin/sh
# Type an element read as never failing, as inference did before 2026-09-28.
# A list literal keeps a failing item, so a compiled build then calls the
# reader's callee with no test of the err, and the runtime corpus fixture
# an_err_read_out_of_a_list_is_handed_on reads a trace missing `items`.
set -e
grep -q '^                out |= (TOP & !FAIL & !THUNK) | deferred | ctx.stored_fails;$' src/infer.rs
sed -i.bak 's#^                out |= (TOP & !FAIL & !THUNK) | deferred | ctx.stored_fails;$#                out |= (TOP \& !FAIL \& !THUNK) | deferred;#' src/infer.rs
rm -f src/infer.rs.bak
grep -q '^                out |= (TOP & !FAIL & !THUNK) | deferred;$' src/infer.rs
