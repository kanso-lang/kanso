#!/bin/sh
# Pack every int into a record's first word, as the native backend did until
# 2026-09-29. An int past 56 signed bits then loses its top byte on the way
# through a call, and the micro corpus goes red.
set -e
old='    f.line(&format!("{fits} = icmp eq i64 {back}, {n}"));'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^    f.line(&format!("{fits} = icmp eq i64 {back}, {n}"));$#    f.line(\&format!("{fits} = icmp eq i64 {back}, {back}"));#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
