#!/bin/sh
# Refuse to call a callable the embedded interpreter made, as the page did
# before 2026-10-05. The first `ref` a `list/tie` maker calls on the page then
# dies as not callable, and the wasm corpus goes red on all four tie fixtures.
set -e
old='            if matches!(value, Value::FnRef(_) | Value::Partial(..) | Value::Closure(_)) {'
[ "$(grep -cxF "$old" src/wasm_rt.rs)" -eq 1 ]
sed -i.bak 's#^            if matches!(value, Value::FnRef(_) | Value::Partial(..) | Value::Closure(_)) {$#            if false \&\& matches!(value, Value::FnRef(_) | Value::Partial(..) | Value::Closure(_)) {#' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
! grep -qxF "$old" src/wasm_rt.rs
