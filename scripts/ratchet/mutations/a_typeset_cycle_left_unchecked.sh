#!/bin/sh
# Stop the checker asking whether a typeset reaches itself through its
# members, as it did not until 2026-09-28. The error corpus fixture
# a_typeset_holds_itself then reports nothing.
set -e
old='            if member == ty.name {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^            if member == ty.name {$#            if member == ty.name \&\& ty.members.is_empty() {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
