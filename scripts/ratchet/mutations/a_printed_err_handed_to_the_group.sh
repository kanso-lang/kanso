#!/bin/sh
# Tell the `print` site its argument cannot be an err, as a compiled build
# assumed until 2026-09-28. The err goes to the ambient `render/to_string`
# group, which hands it on with a hop the program never wrote, and the runtime
# corpus fixture a_printed_err_is_not_a_hop reads a trace line the interpreter
# does not print.
set -e
old='                    let may_be_err = f.set_of(&emitted[0]) & ERR != 0;'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^                    let may_be_err = f.set_of(&emitted\[0\]) & ERR != 0;$#                    let may_be_err = false;#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
