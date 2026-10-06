#!/bin/sh
# The translator writes a subtraction where the emitter asked for an addition.
#
# `ir_wasm` lowers the native emitter's module to the wasm the page runs, and
# picks an integer opcode by its offset from the first one in the family.
# Handing `add` the offset of `sub` leaves every module valid and makes nearly
# every program answer something else, which the page's walk of the golden
# corpus compares against native byte for byte.
set -e
f=src/ir_wasm.rs
grep -q 'Bin::Add => 0,' src/ir_wasm.rs
[ "$(grep -c '^            Bin::Add => 0,$' "$f")" = 1 ] || {
  echo "the translator's opcode table moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^            Bin::Add => 0,$|            Bin::Add => 1,|' "$f"
grep -q '^            Bin::Add => 1,$' "$f"
