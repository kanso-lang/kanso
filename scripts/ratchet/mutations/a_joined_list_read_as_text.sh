#!/bin/sh
# Type what `text/join` answers as text alone in the emitter, as a compiled
# build did until 2026-09-28. A list literal can keep an err that the join
# then answers, a guard reads `length` of it as a number, and the runtime
# corpus fixture a_joined_err_fails_a_guard returns 7 where the interpreter
# hands the err on.
set -e
old='                "join" if arg_sets[0] & LIST != 0 => self.inference.stored_fails,'
[ "$(grep -cxF "$old" src/codegen/emit.rs)" -eq 1 ]
sed -i.bak 's#^                "join" if arg_sets\[0\] & LIST != 0 => self.inference.stored_fails,$#                "join" if false => self.inference.stored_fails,#' src/codegen/emit.rs
rm -f src/codegen/emit.rs.bak
! grep -qxF "$old" src/codegen/emit.rs
