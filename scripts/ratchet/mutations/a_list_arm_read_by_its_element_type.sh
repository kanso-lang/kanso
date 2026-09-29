#!/bin/sh
# Compare two list or map annotations by what they say they hold, as the
# overlap check did until 2026-09-29. `_:[]int` beside `_:[]string` then
# passes `kanso check`, and the error corpus fixture
# a_list_arm_reads_no_element_type reports nothing.
set -e
old='            x == y || tested_as(x) == tested_as(y)'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^            x == y || tested_as(x) == tested_as(y)$#            x == y || (crate::ast::is_effect_type(x) \&\& crate::ast::is_effect_type(y))#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
