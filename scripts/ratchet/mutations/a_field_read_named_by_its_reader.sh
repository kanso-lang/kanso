#!/bin/sh
# Refuse a literal handed to a field read in the name of the field's reader,
# as the check did until 2026-09-29: `[1 2].a` then says no arm of `Get_a`
# takes a list, placed in the entry file, and the error corpus fixture
# a_field_read_of_a_list reads otherwise.
set -e
old='                    (Some(field), Some(file)) => diags.push('
[ "$(grep -cxF "$old" src/check/literals.rs)" -eq 1 ]
sed -i.bak 's#^                    (Some(field), Some(file)) => diags.push($#                    (Some(field), Some(file)) if false => diags.push(#' src/check/literals.rs
rm -f src/check/literals.rs.bak
! grep -qxF "$old" src/check/literals.rs
