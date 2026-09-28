#!/bin/sh
# Count only the annotating file's own types, as the checker did until
# 2026-09-28. `p:pt` with `pt` declared in the next file of the module is then
# refused, and tests/sibling_types.rs goes red on both engines.
set -e
old='        self.own.contains(name) || self.siblings.contains(name)'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        self.own.contains(name) || self.siblings.contains(name)$#        self.own.contains(name)#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
