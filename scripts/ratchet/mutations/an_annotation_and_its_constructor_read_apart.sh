#!/bin/sh
# Read `_:pt` and a constructor pattern that binds every field of a `pt` as two
# shapes, as the overlap check did until 2026-09-28. A group holding both then
# passes `kanso check`, and the error corpus fixture
# an_annotation_and_its_constructor_overlap reports nothing.
set -e
old='            x == y && fields.iter().all(|f| matches!(f, Pattern::Var(..) | Pattern::Wildcard(..)))'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^            x == y && fields.iter().all(#            false \&\& x == y \&\& fields.iter().all(#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
