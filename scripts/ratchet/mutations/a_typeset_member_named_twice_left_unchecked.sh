#!/bin/sh
# Stop the checker asking whether a typeset names a member twice, as it did
# not until 2026-09-29. `type both int int` then passes `kanso check`, and the
# error corpus fixture a_name_declared_twice_in_a_type loses its second
# diagnostic.
set -e
old='            if ty.members[..at].contains(member) {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^            if ty.members\[..at\].contains(member) {$#            if false \&\& ty.members[..at].contains(member) {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
