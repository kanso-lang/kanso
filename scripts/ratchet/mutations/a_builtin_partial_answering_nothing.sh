#!/bin/sh
# Answer no argument count for a builtin handed to `&`, as the interpreter did
# until 2026-09-29. `(&length) [1 2]` then grows into a partial that prints
# `<fn>` where native runs length, and the micro corpus goes red.
set -e
old='                Callee::Builtin => crate::check::builtin_arity(name).into_iter().collect(),'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^                Callee::Builtin => crate::check::builtin_arity(name).into_iter().collect(),$#                Callee::Builtin => Vec::new(),#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
