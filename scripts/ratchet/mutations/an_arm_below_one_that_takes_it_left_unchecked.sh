#!/bin/sh
# Stop the checker asking whether an earlier arm takes every value a later one
# does, as it did not until 2026-09-29. `(pt n _)` above `(pt 1 _)` then passes
# `kanso check`, and the error corpus fixture
# an_arm_written_below_one_that_takes_it reports nothing.
set -e
old='                .any(|e| e.params.iter().zip(&later.params).all(|(p, q)| covers(p, q, true)));'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^                .any(|e| e.params.iter().zip(&later.params).all(|(p, q)| covers(p, q, true)));$#                .any(|e| false \&\& e.params.iter().zip(\&later.params).all(|(p, q)| covers(p, q, true)));#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
