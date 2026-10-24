#!/bin/sh
# kanso::compare::either_from tests a word for two bytes at once, and each
# test needs its `& !` to keep a byte at 0x80 and above from passing for a
# match. This mutation drops the second, and the fuzzer in
# tests/the_word_compare_agrees_with_bytes.rs finds a search that stops early.
set -e
target='        let z = (x.wrapping_sub(LO) & !x | y.wrapping_sub(LO) & !y) & HI;'
n=$(grep -cxF "$target" src/compare.rs)
[ "$n" -eq 1 ] || { echo "either_from changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        let z = (x.wrapping_sub(LO) \& !x | y.wrapping_sub(LO) \& !y) \& HI;$/        let z = (x.wrapping_sub(LO) \& !x | y.wrapping_sub(LO)) \& HI;/' src/compare.rs
grep -qF '| y.wrapping_sub(LO)) & HI;' src/compare.rs
