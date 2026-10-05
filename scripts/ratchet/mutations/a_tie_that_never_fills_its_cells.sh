#!/bin/sh
# `list/tie` makes every node and never resolves the references.
#
# Each key's cell is made before the first node, so a node stores the cell for
# a key made later, and the tie fills every cell with its node once all are
# made. Without the fill a reference stays a blackhole, and the first walk
# through one stops with "a lazy binding demands its own value".
set -e
grep -qF '*cell.borrow_mut() = ThunkState::Forced(node.clone());' src/eval.rs
f=src/eval.rs
line='                    *cell.borrow_mut() = ThunkState::Forced(node.clone());'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the tie's fill moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    \*cell\.borrow_mut() = ThunkState::Forced(node\.clone());$/                    let _ = (cell, node);/' "$f"
grep -qxF '                    let _ = (cell, node);' "$f"
