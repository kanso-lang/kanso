#!/bin/sh
# Answer a group value's failing argument without naming the group in the
# browser, as it did until 2026-09-29. The page's trace then drops f1 where
# native and the interpreter name it, and the wasm corpus goes red.
set -e
old='        Slot::E(held) if held.len() == 1 => rt_err_hop(failure, held[0]),'
[ "$(grep -cxF "$old" src/wasm_rt.rs)" -eq 1 ]
sed -i.bak 's#^        Slot::E(held) if held.len() == 1 => rt_err_hop(failure, held\[0\]),$#        Slot::E(held) if held.len() == 1 => failure,#' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
! grep -qxF "$old" src/wasm_rt.rs
