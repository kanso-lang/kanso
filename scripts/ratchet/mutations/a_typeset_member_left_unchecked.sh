#!/bin/sh
# Stop the checker asking whether a typeset's members name types, as it did
# not until 2026-09-28. `type either id word` with neither declared then passes
# `kanso check`, and the error corpus fixture a_typeset_holds_no_type reports
# nothing.
set -e
old='        for member in &ty.members {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        for member in &ty.members {$#        for member in ty.members.iter().filter(|_| false) {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
