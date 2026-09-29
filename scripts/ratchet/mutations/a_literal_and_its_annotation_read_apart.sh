#!/bin/sh
# Read `none` and `_:none` as two shapes, as the overlap check did until
# 2026-09-29. A group holding both then passes `kanso check`, and the error
# corpus fixture an_arm_other_arms_answer_first loses its first diagnostic.
set -e
old='        | (Pattern::Annotated { ty: y, .. }, Pattern::Nullary(x, _)) => x == y,'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        | (Pattern::Annotated { ty: y, .. }, Pattern::Nullary(x, _)) => x == y,$#        | (Pattern::Annotated { ty: y, .. }, Pattern::Nullary(x, _)) => false \&\& x == y,#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
