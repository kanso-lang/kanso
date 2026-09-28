#!/bin/sh
# Stop the interpreter's condition reading through a subtype of bool, as it
# did not until 2026-09-28. It then says a condition is true or false and that
# it got true, and the micro fixture a_subtype_of_bool_is_a_condition stops
# where a compiled build prints.
set -e
old='        match sub_base(other.clone()) {'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^        match sub_base(other.clone()) {$#        match other.clone() {#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
