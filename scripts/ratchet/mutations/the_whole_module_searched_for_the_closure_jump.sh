#!/bin/sh
# The narrowing learns whether the module defines the closure-jump twin
# `k_tcall{n}` from the define lines it walks anyway (2026-10-23). It used to
# search the whole module for the twin's define first. This mutation puts that
# search back, and the browser compile row is the witness: 266,884,355 with
# the walk, 268,297,303 under the mutation.
set -e
target='    let mut jumps_to_lambdas = false;'
n=$(grep -cxF "$target" src/codegen/narrow.rs)
[ "$n" -eq 1 ] || { echo "the twin check changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^    let mut jumps_to_lambdas = false;$/    let mut jumps_to_lambdas = ir.contains("define internal tailcc %KValue @k_tcall");/' src/codegen/narrow.rs
grep -qF 'let mut jumps_to_lambdas = ir.contains(' src/codegen/narrow.rs
