#!/bin/sh
# The tab's translator asks of every IR line whether it opens a switch. It
# finds the line's `=` and compares the words there (2026-10-23); it used to
# call `contains("= switch")`, which builds a substring searcher for every
# line. This mutation puts the searcher back, and the browser compile row is
# the witness: browser_compile_instructions 274,134,296 -> 268,290,909 when
# the check landed.
set -e
target='        if body.starts_with("switch") || switch(&body) {'
n=$(grep -cxF "$target" src/ir_wasm.rs)
[ "$n" -eq 1 ] || { echo "the switch check changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        if body.starts_with("switch") || switch(&body) {$/        if body.starts_with("switch") || body.contains("= switch") {/' src/ir_wasm.rs
grep -qxF '        if body.starts_with("switch") || body.contains("= switch") {' src/ir_wasm.rs
