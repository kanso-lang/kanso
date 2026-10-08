#!/bin/sh
# Size the interpreter's direct-mapped callee table at 1,024 slots again. The
# run gets slightly faster and holds 18,432 more bytes for its whole length,
# so the witness is the interpreted run's peak.
set -e
target='const RECENT_CALLEES: usize = 256;'
n=$(grep -cxF "$target" src/eval.rs)
[ "$n" -eq 1 ] || { echo "the callee table's size moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '$0 == t { print "const RECENT_CALLEES: usize = 1024;"; next } { print }' src/eval.rs > src/eval.rs.mut && mv src/eval.rs.mut src/eval.rs
