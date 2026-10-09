#!/bin/sh
# Choose the slot a new leader's score moves to at run time instead of
# naming slot 0. The same candidate wins, so the output is unchanged; every
# access through the computed slot pays for its address and its bounds, and
# the witness is the interpreted run's instruction count.
set -e
target='                    scores.swap(0, 1);'
n=$(grep -cxF "$target" src/eval.rs)
[ "$n" -eq 1 ] || { echo "the leader's swap moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '$0 == t { print "                    scores.swap(std::hint::black_box(0), 1);"; next } { print }' src/eval.rs > src/eval.rs.mut && mv src/eval.rs.mut src/eval.rs
