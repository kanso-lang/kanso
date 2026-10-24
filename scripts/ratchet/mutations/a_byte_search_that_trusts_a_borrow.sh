#!/bin/sh
# kanso::compare::byte_from finds a byte in eight with the has-zero test,
# whose `& !w` keeps a byte at 0x80 and above from passing for a match. This
# mutation drops it, and the fuzzer in tests/the_word_compare_agrees_with_bytes.rs
# finds a search that stops early.
set -e
target='        let z = w.wrapping_sub(LO) & !w & HI;'
n=$(grep -cxF "$target" src/compare.rs)
[ "$n" -eq 1 ] || { echo "byte_from changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        let z = w.wrapping_sub(LO) \& !w \& HI;$/        let z = w.wrapping_sub(LO) \& HI;/' src/compare.rs
grep -qxF '        let z = w.wrapping_sub(LO) & HI;' src/compare.rs
