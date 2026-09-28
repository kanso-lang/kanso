#!/bin/sh
# Leave `&name` out of the mentions that count as a value, as the linear
# analysis did until 2026-09-28. A curried application is then no caller at
# all, a parameter read only as an interpolation's head qualifies as a string
# builder, and the micro fixture a_curried_int_heads_an_interpolation dies
# natively converting an int.
set -e
old='            Expr::Ident(n, _, _) | Expr::Partial(n, _) => {'
[ "$(grep -cxF "$old" src/linear.rs)" -eq 1 ]
sed -i.bak 's#^            Expr::Ident(n, _, _) | Expr::Partial(n, _) => {$#            Expr::Ident(n, _, _) => {#' src/linear.rs
rm -f src/linear.rs.bak
! grep -qxF "$old" src/linear.rs
