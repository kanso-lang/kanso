#!/bin/sh
# Let inference narrow a name an arm binds to a value that cannot fail, even
# when the name holds a cell nobody has forced. The group the cell reaches
# next keeps no guard for an err, so the err answers a frame later and the
# runtime corpus fixture names `next` where every engine should name `iter`.
set -e
old='                _ => joined,'
[ "$(grep -cxF "$old" src/infer.rs)" -eq 1 ]
sed -i.bak 's#^                _ => joined,$#                _ => joined \& !FAIL,#' src/infer.rs
rm -f src/infer.rs.bak
grep -qxF '                _ => joined & !FAIL,' src/infer.rs
