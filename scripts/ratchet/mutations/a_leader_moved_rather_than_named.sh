#!/bin/sh
# Move a new leader's score and bindings into the other slot instead of
# flipping the index that names it. The same candidate wins, so the output is
# unchanged, and the witness is the interpreted run's instruction count.
set -e
target='                    cur ^= 1;'
n=$(grep -cxF "$target" src/eval.rs)
[ "$n" -eq 1 ] || { echo "the leader's flip moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '$0 == t { print "                    pair.swap(0, 1);"; print "                    scores.swap(0, 1);"; next } { print }' src/eval.rs > src/eval.rs.mut && mv src/eval.rs.mut src/eval.rs
