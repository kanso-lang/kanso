#!/bin/sh
# line_end finds a newline in eight bytes with the has-zero test, whose
# `& !w` keeps a byte at 0x80 and above from passing for a zero. This mutation
# drops it, and the fuzzer in src/ir_wasm.rs finds a line ending early.
set -e
target='        let z = w.wrapping_sub(LO) & !w & HI;'
n=$(grep -cxF "$target" src/ir_wasm.rs)
[ "$n" -eq 1 ] || { echo "line_end changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        let z = w.wrapping_sub(LO) \& !w \& HI;$/        let z = w.wrapping_sub(LO) \& HI;/' src/ir_wasm.rs
grep -qxF '        let z = w.wrapping_sub(LO) & HI;' src/ir_wasm.rs
