#!/bin/sh
# Entering a declaration reads its frame from a table, at the declaration's
# place in the program. This mutation reads past the table's end, so every
# entry makes the frame again: a trace line formatted and a package looked
# up for a string only an err reads. The answer is the same;
# interp_instructions is the witness.
set -e
line='        if let Some(frame @ Some(_)) = self.frames.borrow().get(at) {'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the frame table's read moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if let Some(frame @ Some(_)) = self.frames.borrow().get(at) {$/        if let Some(frame @ Some(_)) = self.frames.borrow().get(at + usize::MAX \/ 2) {/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
