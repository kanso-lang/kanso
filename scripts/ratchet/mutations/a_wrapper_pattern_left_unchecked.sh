#!/bin/sh
# Stop the checker asking whether a constructor pattern names a wrapper, as it
# did not until 2026-09-28. `(id n)` for `type id int` then passes `kanso
# check`, and the error corpus fixture a_wrapper_pattern_never_matches reports
# nothing.
set -e
old='                if wrapping.contains(ty.as_str()) && !fields.is_empty() {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^                if wrapping.contains(ty.as_str()) \&\& !fields.is_empty() {$#                if wrapping.is_empty() \&\& wrapping.contains(ty.as_str()) {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
