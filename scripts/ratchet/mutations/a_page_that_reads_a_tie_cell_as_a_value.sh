#!/bin/sh
# Read a field the way the page did before 2026-10-05, without asking the
# embedded interpreter for a cell's value. A field a `list/tie` maker filled
# with a `ref` then hands the page the cell itself, the graph fixture's walk
# reaches no arm of `trail`, and the wasm corpus goes red.
set -e
old='    if let Value::Thunk(_) = v {'
[ "$(grep -cxF "$old" src/wasm_rt.rs)" -eq 1 ]
sed -i.bak 's#^    if let Value::Thunk(_) = v {$#    if let (Value::Thunk(_), false) = (\&v, true) {#' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
! grep -qxF "$old" src/wasm_rt.rs
