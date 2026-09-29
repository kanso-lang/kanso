#!/bin/sh
# The playground reads the engine's output pointer as a signed number.
#
# A wasm export answers an i32, and past two gibibytes of memory its top bit is
# set. docs/kanso-engine.js turns every pointer and length it is handed into an
# unsigned offset before it builds a view on the memory; without that, an
# answer the program produced sits at a negative offset and the page throws a
# RangeError instead of printing it. site_smoke grows the memory past the line
# on its paint page and reads a sixteen-megabyte answer back.
set -e
f=docs/kanso-engine.js
anchor='wasm.kanso_out_ptr() >>> 0, wasm.kanso_out_len() >>> 0'
grep -qF "$anchor" "$f" || {
  echo "readOut moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|wasm.kanso_out_ptr() >>> 0, wasm.kanso_out_len() >>> 0|wasm.kanso_out_ptr(), wasm.kanso_out_len()|' "$f"
grep -qF 'wasm.kanso_out_ptr(), wasm.kanso_out_len()' "$f"
