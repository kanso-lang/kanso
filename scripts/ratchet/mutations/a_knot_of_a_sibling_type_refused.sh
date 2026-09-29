#!/bin/sh
# Count only the file's own types in the constant-cycle check, as it did
# until 2026-09-29. A knot through a record the next file declares is then
# refused as defined in terms of itself, and tests/sibling_types.rs goes red
# on both engines.
set -e
old='    let types = TypeNames { own: &own, siblings: sibling_types };'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^    let types = TypeNames { own: &own, siblings: sibling_types };$#    let types = TypeNames { own: \&own, siblings: \&HashSet::default() };#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
