#!/bin/sh
# The interpreter's table of callees is as long as the names a run has
# called. This mutation starts it at 1,024 entries, as the fixed table it
# replaced once had: no call gets faster, and the run holds 16,384 more
# bytes for its whole length, so the witness is the interpreted run's peak.
set -e
target='            callees_by_number: RefCell::new(Vec::new()),'
n=$(grep -cxF "$target" src/eval.rs)
[ "$n" -eq 1 ] || { echo "the callee table's start moved or multiplied ($n); rewrite this" >&2; exit 1; }
sed -i 's/^            callees_by_number: RefCell::new(Vec::new()),$/            callees_by_number: RefCell::new(vec![None; 1024]),/' src/eval.rs
grep -qxF '            callees_by_number: RefCell::new(vec![None; 1024]),' src/eval.rs
