#!/bin/sh
# Stop the browser's condition reading through a subtype of bool, as it did
# not until 2026-09-28. The wasm engine then refuses the wrapper, and the
# engine differential sees the micro fixture a_subtype_of_bool_is_a_condition
# die where the others print.
set -e
old='        other => match eval::sub_base(other.clone()) {'
[ "$(grep -cxF "$old" src/wasm_rt.rs)" -eq 1 ]
sed -i.bak 's#^        other => match eval::sub_base(other.clone()) {$#        other => match other.clone() {#' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
! grep -qxF "$old" src/wasm_rt.rs
