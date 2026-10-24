#!/bin/sh
# The tab's translator trims each line of IR as ASCII, since emitted IR is
# ASCII. This mutation puts `str::trim` back on the first pass, which decodes
# every character at both ends to ask the Unicode table. Same lines, and the
# tab's compile reads heavier.
set -e
f=src/ir_wasm.rs
[ "$(grep -c '        let line = raw.trim_ascii();' src/ir_wasm.rs)" -eq 2 ] || {
  echo "the translator's trims moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/        let line = raw.trim_ascii();/        let line = raw.trim();/' "$f"
[ "$(grep -c '        let line = raw.trim();' "$f")" -eq 2 ]
