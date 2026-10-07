#!/bin/sh
# Type round of a float as an int alone, as inference did before 2026-09-29.
# A rounded float handed to a group with no `none` arm is then accepted, and
# the error corpus goes red.
set -e
old='            true => ANY_INT | NONE | fails,'
[ "$(grep -cxF "$old" src/infer.rs)" -eq 1 ]
sed -i.bak 's#^            true => ANY_INT | NONE | fails,$#            true => ANY_INT | fails,#' src/infer.rs
rm -f src/infer.rs.bak
! grep -qxF "$old" src/infer.rs
