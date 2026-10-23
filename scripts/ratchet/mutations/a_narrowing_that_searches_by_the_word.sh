#!/bin/sh
# The narrowing finds `tailcc ` lines by their `c`, eight bytes a pass, and
# compares the word where one lands (2026-10-23). This mutation goes back to
# the two-way search for the word, and the browser compile row is the witness.
set -e
target='lines_holding_at(ir, "tailcc ", 4)'
n=$(grep -cF "$target" src/codegen/narrow.rs)
[ "$n" -eq 1 ] || { echo "the narrowing changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/lines_holding_at(ir, "tailcc ", 4)/lines_holding(ir, "tailcc ")/' src/codegen/narrow.rs
grep -qF 'lines_holding(ir, "tailcc ")' src/codegen/narrow.rs
