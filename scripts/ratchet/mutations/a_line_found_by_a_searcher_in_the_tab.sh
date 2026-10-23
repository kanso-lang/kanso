#!/bin/sh
# The tab's translator ends each IR line with line_end, eight bytes a pass
# (2026-10-23), where find('\n') and str::lines paid a searcher's setup, a
# memchr call and a one-byte compare per line. This mutation puts both back,
# and the browser compile row is the witness: 264,519,748 -> 261,513,550 when
# line_end landed.
set -e
a='        let end = line_end(ir.as_bytes(), at);'
b='    let mut lines = Lines { text, at: 0 };'
for t in "$a" "$b"; do
  n=$(grep -cxF "$t" src/ir_wasm.rs)
  [ "$n" -eq 1 ] || { echo "the line search changed shape ($n): $t" >&2; exit 1; }
done
sed -i "s/^        let end = line_end(ir.as_bytes(), at);\$/        let end = ir[at..].find('\\\\n').map_or(ir.len(), |n| at + n);/" src/ir_wasm.rs
sed -i 's/^    let mut lines = Lines { text, at: 0 };$/    let mut lines = text.lines();/' src/ir_wasm.rs
grep -qxF "        let end = ir[at..].find('\\n').map_or(ir.len(), |n| at + n);" src/ir_wasm.rs
grep -qxF '    let mut lines = text.lines();' src/ir_wasm.rs
