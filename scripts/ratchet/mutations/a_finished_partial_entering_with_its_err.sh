#!/bin/sh
# Enter the group a finished partial names even when a held argument failed,
# as native did until 2026-09-29. `(&g p v) 1` then answers what g answers,
# and the runtime corpus goes red.
set -e
old='        f.line(&format!("br i1 {bad}, label %{skip}, label %{go}"));'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        f.line(&format!("br i1 {bad}, label %{skip}, label %{go}"));$#        f.line(\&format!("br i1 false, label %{skip}, label %{go}"));#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
