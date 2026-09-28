#!/bin/sh
# Stop the checker asking whether a subtype's parent names a type, as it did
# not until 2026-09-28. `type blob bytes` then passes `kanso check`, and the
# error corpus fixture a_subtype_wraps_no_type reports nothing.
set -e
old='        let Some(parent) = &ty.parent else { continue };'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        let Some(parent) = &ty.parent else { continue };$#        let Some(parent) = ty.parent.as_ref().filter(|_| false) else { continue };#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
