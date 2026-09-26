#!/bin/sh
# A literal is read into a word through `BigInt::to_i64`'s general digit walk.
#
# `word_of` reads the single digit a word-sized number has directly, and the
# literal's evaluation inlines it. Sent back through `to_i64` first, every
# literal the interpreter evaluates pays the walk again, and the interpreted
# run's instruction row moves.
set -e
f=src/int.rs
grep -q 'fn word_of(n: &BigInt)' src/int.rs
line='    let mut digits = n.iter_u64_digits();'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the word read moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let mut digits = n.iter_u64_digits();$/    if let Some(word) = n.to_i64() {\n        return Some(word);\n    }\n    let mut digits = n.iter_u64_digits();/' "$f"
grep -q 'if let Some(word) = n.to_i64()' "$f"
