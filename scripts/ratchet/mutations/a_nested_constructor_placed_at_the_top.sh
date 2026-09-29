#!/bin/sh
# Give a constructor pattern no place of its own, as until 2026-09-29: a
# refusal pointing at a constructor nested first in another is then printed
# at 0:0, and the module-error corpus goes red.
set -e
old='            fields.iter().map(other_span).find(|s| s.line != 0).unwrap_or(Span::at(0, 0))'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^            fields.iter().map(other_span).find(|s| s.line != 0).unwrap_or(Span::at(0, 0))$#            fields.first().map_or(Span::at(0, 0), |_| Span::at(0, 0))#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
