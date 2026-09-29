#!/bin/sh
# Hold a partial over a group of more than four parameters in a closure, as
# native did until 2026-09-29. `k = &f5 0 0; k 0 0 bad` then answers the err
# before f5 runs, the trace drops f5, and the runtime corpus goes red.
set -e
old='            if params.len() + supplied.len() <= 8 {'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^            if params.len() + supplied.len() <= 8 {$#            if params.len() + supplied.len() <= 4 {#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
