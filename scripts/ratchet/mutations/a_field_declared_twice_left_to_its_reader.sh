#!/bin/sh
# Stop the checker asking whether a record names a field twice, as it did not
# until 2026-09-29. The only report is then the overlap of the field's
# internal reader, which the overlap check no longer makes, and the error
# corpus fixture a_name_declared_twice_in_a_type loses its first diagnostic.
set -e
old='            if ty.fields[..at].iter().any(|(earlier, _, _)| earlier == field) {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^            if ty.fields\[..at\].iter().any(|(earlier, _, _)| earlier == field) {$#            if false \&\& ty.fields[..at].iter().any(|(earlier, _, _)| earlier == field) {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
