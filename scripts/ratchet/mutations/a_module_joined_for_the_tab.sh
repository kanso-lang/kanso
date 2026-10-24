#!/bin/sh
# The tab translates the module in the pieces the emitter wrote, never
# joined (2026-10-23). This mutation joins them first, which holds the module
# twice at the tab's compile peak, and the browser compile peak is the
# witness.
set -e
target='    let side = ir.and_then(crate::ir_wasm::translate_pieces);'
n=$(grep -cxF "$target" src/wasm.rs)
[ "$n" -eq 1 ] || { echo "the tab's translation changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^    let side = ir.and_then(crate::ir_wasm::translate_pieces);$/    let side = ir.and_then(|p| crate::ir_wasm::translate_owned(p.concat()));/' src/wasm.rs
grep -qF 'translate_owned(p.concat())' src/wasm.rs
