#!/bin/sh
# The 2026-09-15 ruling's third part: a raised err arriving where a value is
# wanted — an operator's operand, an index's base or key, a call at a position
# no arm names an err at — does not compile. This deletes the one call that
# asks the question, so every such program compiles again and the three error
# fixtures an_err_reaches_an_operator, an_err_reaches_an_index and
# an_err_reaches_a_group_with_no_arm_for_it answer no diagnostic where their
# goldens hold one.
set -e
grep -qF 'raised_err_at(cur, &raisers, &consts, &err_arms, &decl.file, none_diags);' src/check.rs
sed -i '/raised_err_at(cur, &raisers, &consts, &err_arms, &decl.file, none_diags);/d' src/check.rs
! grep -qF 'raised_err_at(cur, &raisers' src/check.rs
