#!/bin/sh
# Hand an err to the ambient `render/to_string` group as a compiled build did
# until 2026-09-28, instead of passing it on unrendered. The group hands it on
# with a hop the program never wrote, and the runtime corpus fixture
# an_interpolated_err_is_not_a_hop reads a trace line the interpreter does not
# print.
set -e
old='        if !may_be_err {'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        if !may_be_err {$#        if !may_be_err || true {#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
