#!/bin/sh
# Hold a partial over a declared group in a closure in the browser, as it did
# until 2026-09-29. `k = &f1 0; k bad` then answers the err before f1 runs,
# the page's trace drops f1, and the wasm corpus goes red.
set -e
old='    matches!(lambda, Expr::Lambda { params, .. } if params.len() + held <= 8)'
[ "$(grep -cxF "$old" src/wasm_backend.rs)" -eq 1 ]
sed -i.bak 's#^    matches!(lambda, Expr::Lambda { params, .. } if params.len() + held <= 8)$#    matches!(lambda, Expr::Lambda { params, .. } if params.len() + held <= 8) \&\& false#' src/wasm_backend.rs
rm -f src/wasm_backend.rs.bak
! grep -qxF "$old" src/wasm_backend.rs
