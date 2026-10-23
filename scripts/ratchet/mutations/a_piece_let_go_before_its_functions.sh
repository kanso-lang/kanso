#!/bin/sh
# The translator lets a piece of the module go once the functions in it are
# lowered. This mutation lets it go before, and the interp corpus cut into
# pieces in tests/ir_to_wasm.rs fails to translate.
set -e
target='        while held < h.piece {'
n=$(grep -cxF "$target" src/ir_wasm.rs)
[ "$n" -eq 1 ] || { echo "the piece walk changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        while held < h.piece {$/        while held <= h.piece {/' src/ir_wasm.rs
grep -qxF '        while held <= h.piece {' src/ir_wasm.rs
