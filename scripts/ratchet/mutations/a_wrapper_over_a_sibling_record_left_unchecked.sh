#!/bin/sh
# Hand the check that a constructor pattern can match only the file's own
# types, as until 2026-09-29. A wrapper arm over a record the next file
# declares then passes `kanso check` and never runs, and the module-error
# corpus goes red.
set -e
old='        sibling_decls.iter().copied().chain(&program.types).map(|t| (t.name.as_str(), t)).collect();'
[ "$(grep -cxF "$old" src/check/annotations.rs)" -eq 1 ]
sed -i.bak 's#^        sibling_decls\.iter()\.copied()#        sibling_decls[..0].iter().copied()#' src/check/annotations.rs
rm -f src/check/annotations.rs.bak
! grep -qxF "$old" src/check/annotations.rs
