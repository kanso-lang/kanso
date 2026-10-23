#!/bin/sh
# The tab's translator asks of every IR line whether it opens a switch. A
# switch names no result, so the line starts with the word, and that is the
# whole test (2026-10-23). It used to call `contains("= switch")` as well,
# which builds a substring searcher for every line; a byte scan for the `=`
# replaced that, and then the test was dropped. This mutation puts the
# searcher back, and the browser compile row is the witness:
# browser_compile_instructions 274,134,296 -> 268,290,909 when the byte scan
# landed.
set -e
target='        if body.starts_with("switch") {'
n=$(grep -cxF "$target" src/ir_wasm.rs)
[ "$n" -eq 1 ] || { echo "the switch check changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        if body.starts_with("switch") {$/        if body.starts_with("switch") || body.contains("= switch") {/' src/ir_wasm.rs
grep -qxF '        if body.starts_with("switch") || body.contains("= switch") {' src/ir_wasm.rs
