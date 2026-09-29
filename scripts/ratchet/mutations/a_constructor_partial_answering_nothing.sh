#!/bin/sh
# Answer no argument count for a record's constructor handed to `&`, as the
# interpreter did until 2026-09-29. `(&pt 2) "y"` then grows into a partial
# that prints `<fn>` where it should build the record, and the partial spec
# goes red.
set -e
old='                Callee::Constructor(ty) if ty.members.is_empty() => vec![ty.fields.len()],'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^                Callee::Constructor(ty) if ty.members.is_empty() => vec!\[ty.fields.len()\],$#                Callee::Constructor(ty) if ty.members.is_empty() => Vec::new(),#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
