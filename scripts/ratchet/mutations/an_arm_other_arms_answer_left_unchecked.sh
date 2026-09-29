#!/bin/sh
# Stop the checker asking whether other arms take every value a `bool` or
# typeset arm admits, as it did not until 2026-09-29. `_:bool` below arms for
# `true` and `false` then passes `kanso check`, and the error corpus fixture
# an_arm_other_arms_answer_first loses two of its diagnostics.
set -e
old='        if answered {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        if answered {$#        if false \&\& answered {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
