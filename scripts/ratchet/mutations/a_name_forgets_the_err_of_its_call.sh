#!/bin/sh
# A name bound to a call that can raise carries the call's err into the check
# (#1884), a local through `raising_names` and a constant through the arity-0
# raisers table. This drops both: no local is recorded as raising, and a
# constant read by name stops being asked about. The three error fixtures
# an_err_bound_to_a_name_reaches_an_operator,
# an_err_bound_to_a_name_reaches_a_group_with_no_arm_for_it and
# an_err_bound_to_a_constant_reaches_an_operator then compile clean.
set -e
f=src/check/after_infer.rs
grep -qF 'named.raising.push((name, *at));' src/check/after_infer.rs
grep -qxF '        Expr::Ident(name, _, _) => named.constants && raisers.contains_key(&(name.as_str(), 0)),' "$f"
sed -i 's/^            named.raising.push((name, \*at));$/            let _ = at;/' "$f"
sed -i 's/^        Expr::Ident(name, _, _) => named.constants \&\& raisers.contains_key(&(name.as_str(), 0)),$/        Expr::Ident(_, _, _) => named.constants \&\& false,/' "$f"
! grep -qF 'named.raising.push' "$f"
