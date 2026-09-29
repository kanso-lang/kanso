#!/bin/sh
# Stop the checker reading which calls to `math/round` cannot answer `none`, so
# every call takes the union of every caller's arguments. `plus (math/round 7)`
# then inherits a float caller's `none` and is refused, and the error corpus
# goes red.
set -e
old='                Expr::Ident(_, span, _) if tables.none_free.contains(span) => false,'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^                Expr::Ident(_, span, _) if tables.none_free.contains(span) => false,$#                Expr::Ident(_, span, _) if false \&\& tables.none_free.contains(span) => false,#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
