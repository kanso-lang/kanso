#!/bin/sh
# Check the compiler's own field readers the way a written arm is checked, as
# the checker did until 2026-09-29. A record and a subtype sharing a name then
# report the record's reader as an arm that can never match, and the error
# corpus fixture a_record_and_a_subtype_share_a_name gains a line.
set -e
old='    for decl in program.fns.iter().filter(|d| getter_field(&d.name).is_none()) {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^    for decl in program.fns.iter().filter(|d| getter_field(&d.name).is_none()) {$#    for decl in program.fns.iter() {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
