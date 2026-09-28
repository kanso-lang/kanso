#!/bin/sh
# Stop a native `_:done` arm reading through a subtype, as it did not until
# 2026-09-28. A subtype of done then falls past the arm, and the micro
# fixture a_subtype_of_bool_is_a_condition prints `other` where the
# interpreter prints `fin`.
set -e
old='            "done" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 {K_DONE})"),'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak '/^            "done" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 {K_DONE})"),$/d' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
