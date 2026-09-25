#!/bin/sh
# A program that dies holding a cell leaves it held.
#
# A trap unwinds nothing, so a program that runs out of stack inside
# wasm_rt's `push` dies with REG borrowed. `Held::renew` replaces a cell that
# still reads as borrowed at the next program's `load`; asking the RefCell
# for the borrow instead is what the engine did before, and the next program,
# fine by itself, answered a compiler trap.
set -e
f=src/wasm_rt.rs
grep -q 'fn renew' src/wasm_rt.rs
grep -qxF '        if let Ok(mut held) = self.try_borrow_mut() {' "$f" || {
  echo "Held::renew moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^        if let Ok(mut held) = self.try_borrow_mut() {$|        if let Ok(mut held) = Ok::<_, ()>(self.borrow_mut()) {|' "$f"
grep -qF 'Ok::<_, ()>(self.borrow_mut())' "$f"
