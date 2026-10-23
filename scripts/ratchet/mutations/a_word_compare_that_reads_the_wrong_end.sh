#!/bin/sh
# kanso::compare::bytes finds the first byte that differs from the lowest set
# bit of two words' difference, the first byte in memory on a little-endian
# machine. This mutation takes the highest, and the fuzzer in
# tests/the_word_compare_agrees_with_bytes.rs orders a pair the wrong way.
set -e
target='    let at = at + (diff.trailing_zeros() / 8) as usize;'
n=$(grep -cxF "$target" src/compare.rs)
[ "$n" -eq 1 ] || { echo "compare changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^    let at = at + (diff.trailing_zeros() \/ 8) as usize;$/    let at = at + (diff.leading_zeros() \/ 8) as usize;/' src/compare.rs
grep -qF 'diff.leading_zeros()' src/compare.rs
