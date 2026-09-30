#!/bin/sh
# An arm that writes `int` at its counter ties with the loop's entry and keeps
# a frame per call.
#
# The accumulator rewrite enters its loop through a wrapper arm that ascribes
# every counter `int`. A group that writes `fn count n:int` already carries an
# arm of exactly that shape, and a tie goes to the arm the program wrote, so
# the rewrite now turns that arm into the entry itself. Told to leave it, the
# arm answers every call, a million-deep `1 + count (n - 1)` runs out of stack
# on both engines, and the micro corpus fixture fails.
set -e
f=src/trmc.rs
grep -q "in_place.push((index, params, entry.clone()));" src/trmc.rs
line='                in_place.push((index, params, entry.clone()));'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the in-place entry moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                in_place.push((index, params, entry.clone()));$/                let _ = (index, params);/' "$f"
grep -qxF '                let _ = (index, params);' "$f"
