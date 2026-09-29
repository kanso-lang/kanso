#!/bin/sh
# Answer no arity for a partial, as the interpreter did until 2026-09-29.
# `(&h 3) 1` over `h = &g p` then wraps the three in a third partial and
# prints `<fn>` where native runs g, and the micro corpus goes red.
set -e
old='                .filter(|a| *a >= held.len())'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^                .filter(|a| \*a >= held.len())$#                .filter(|_| false)#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
