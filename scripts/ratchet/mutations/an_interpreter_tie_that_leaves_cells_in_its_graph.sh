#!/bin/sh
# Leave the interpreter's filled cells in the fields a tie's maker built, as
# before 2026-10-05. A field still reads its node through the cell, but a
# nested pattern compares the field as it stands, so `second` in
# a_tied_node_matches_a_nested_pattern answers 0 where native answers 2 and
# the micro corpus goes red.
set -e
old='                    resolve_tied(node, &mut seen);'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^                    resolve_tied(node, &mut seen);$#                    let _ = (node, \&mut seen);#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
