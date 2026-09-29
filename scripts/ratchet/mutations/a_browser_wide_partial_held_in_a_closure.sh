#!/bin/sh
# Hold a partial over a group of more than four parameters in a closure in the
# browser, as it did until 2026-09-29. `k = &f5 0 0; k 0 0 bad` then answers
# the err before f5 runs, the page's trace drops f5, and the wasm corpus goes
# red.
set -e
old='    matches!(lambda, Expr::Lambda { params, .. } if params.len() + held <= 8)'
[ "$(grep -cxF "$old" src/wasm_backend.rs)" -eq 1 ]
sed -i.bak 's#^    matches!(lambda, Expr::Lambda { params, .. } if params.len() + held <= 8)$#    matches!(lambda, Expr::Lambda { params, .. } if params.len() + held <= 4)#' src/wasm_backend.rs
rm -f src/wasm_backend.rs.bak
! grep -qxF "$old" src/wasm_backend.rs
