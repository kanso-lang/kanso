#!/bin/sh
# Hold a partial over a declared group in a closure, as native did until
# 2026-09-29. `k = &f1 0; k bad` then answers the err before f1 runs, the
# trace drops f1, and the runtime corpus goes red.
set -e
old='            if params.len() + supplied.len() <= 8 {'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^            if params.len() + supplied.len() <= 8 {$#            if false {#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
