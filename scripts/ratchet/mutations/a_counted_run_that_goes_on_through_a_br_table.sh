#!/bin/sh
# The browser rows count the wasm instructions a module executes, with a copy
# of the module that adds each run's length to a global as the run begins
# (2026-10-23). A run has to end at every instruction that can branch, or the
# instructions after a `br_table` are counted on passes that never reach them.
# This mutation lets a run go on through a `br_table`; the hand-counted loop in
# tests/browser_cost.rs reads 95 instead of 91.
set -e
target='                                | Operator::BrTable { .. }'
n=$(grep -cxF "$target" tests/support/wasm32.rs)
[ "$n" -eq 1 ] || { echo "counted changed shape ($n); rewrite this" >&2; exit 1; }
sed -i '/^                                | Operator::BrTable { .. }$/d' tests/support/wasm32.rs
! grep -qxF "$target" tests/support/wasm32.rs
