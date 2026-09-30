#!/bin/sh
# Answer a group value's failing arguments before entering the group in the
# browser, as it did before 2026-09-30. The page's trace then drops f1 where
# native and the interpreter name it, two errs merge where a direct call
# reports one, an `(err _)` arm is never reached, and the wasm corpus goes red.
set -e
old='    if arity <= MASKED && matches!(slot(env), Slot::E(held) if held.len() == 1) {'
[ "$(grep -cxF "$old" src/wasm_rt.rs)" -eq 1 ]
sed -i.bak 's#^    if arity <= MASKED \&\& matches!(slot(env), Slot::E(held) if held.len() == 1) {$#    if arity <= MASKED \&\& matches!(slot(env), Slot::E(held) if held.len() == 99) {#' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
! grep -qxF "$old" src/wasm_rt.rs
