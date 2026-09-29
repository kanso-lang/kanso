#!/bin/sh
# Hold every partial over a declared group in a closure in the browser, as it
# did past four parameters until 2026-09-29. `k = &f5 0 0; k 0 0 bad` then
# answers the err before f5 runs, the page's trace drops f5, and the wasm
# corpus goes red.
set -e
old='    matches!(lambda, Expr::Lambda { .. })'
[ "$(grep -cxF "$old" src/wasm_backend.rs)" -eq 1 ]
sed -i.bak 's#^    matches!(lambda, Expr::Lambda { .. })$#    false \&\& matches!(lambda, Expr::Lambda { .. })#' src/wasm_backend.rs
rm -f src/wasm_backend.rs.bak
! grep -qxF "$old" src/wasm_backend.rs
