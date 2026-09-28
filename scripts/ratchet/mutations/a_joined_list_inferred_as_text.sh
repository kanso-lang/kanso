#!/bin/sh
# Infer what `text/join` answers as text alone, as the compiler did until
# 2026-09-28. A release build then calls a `string` arm with the err a list
# literal kept, and the micro fixture a_joined_err_reaches_an_arm_as_an_err
# prints [5 5] where the interpreter prints the err.
set -e
old='        "join" if arg_sets.first().is_some_and(|s| s & LIST != 0) => ctx.stored_fails,'
[ "$(grep -cxF "$old" src/infer.rs)" -eq 1 ]
sed -i.bak 's#^        "join" if arg_sets.first().is_some_and(|s| s & LIST != 0) => ctx.stored_fails,$#        "join" if false => ctx.stored_fails,#' src/infer.rs
rm -f src/infer.rs.bak
! grep -qxF "$old" src/infer.rs
