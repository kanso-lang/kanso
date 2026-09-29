#!/bin/sh
# Hold a partial that supplies arguments in a closure in the browser, as it
# did until 2026-09-29. `k = &f1 0; k bad` then answers the err before f1
# runs, the page's trace drops f1, and the wasm corpus goes red.
set -e
old='                if held_over_value(&partial_lambda(self.program, name, args, *span)?) {'
[ "$(grep -cxF "$old" src/wasm_backend.rs)" -eq 1 ]
sed -i.bak 's#^                if held_over_value(\&partial_lambda(self.program, name, args, \*span)?) {$#                if false \&\& held_over_value(\&partial_lambda(self.program, name, args, *span)?) {#' src/wasm_backend.rs
rm -f src/wasm_backend.rs.bak
! grep -qxF "$old" src/wasm_backend.rs
