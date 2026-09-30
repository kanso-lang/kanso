#!/bin/sh
# Lose the positions a dropped arm read and no kept arm reads. The prune then
# names nothing unread, a cell handed to such a position goes on unforced, and
# the runtime corpus fixture names `found_in` and `next` where every engine
# should name `find`.
set -e
old='        let lost = reads(d) & !kept;'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        let lost = reads(d) \& !kept;$#        let lost = reads(d) \& !kept \& 0;#' src/codegen.rs
rm -f src/codegen.rs.bak
grep -qF 'let lost = reads(d) & !kept & 0;' src/codegen.rs
