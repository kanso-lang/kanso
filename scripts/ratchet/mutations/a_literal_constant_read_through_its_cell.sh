#!/bin/sh
# A mention of a constant whose one arm is an integer literal that fits a word
# is emitted as that literal. This mutation takes that away, so every mention
# calls the constant's accessor, which tests the cell's flag and reads the
# cell. The output is the same; the witness is the work vein, where sha256's
# rounds read `whole` about a dozen times each.
set -e
line='                    if let Some(n) = self.literal_word_constant(name) {'
grep -qxF "$line" src/codegen/emit.rs || {
  echo "the literal constant's arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    if let Some(n) = self.literal_word_constant(name) {$/                    if let Some(n) = self.literal_word_constant(name).filter(|_| false) {/' src/codegen/emit.rs
if grep -qxF "$line" src/codegen/emit.rs; then exit 1; fi
