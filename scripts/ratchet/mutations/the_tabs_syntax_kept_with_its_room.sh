#!/bin/sh
# The tab's lean emit gives back the room each kept function's statement and
# parameter lists doubled into (2026-10-23). This mutation skips that, and the
# browser compile peak is the witness.
set -e
target='    if let (true, Given::Owned(owned)) = (lean, &mut program) {'
n=$(grep -cxF "$target" src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the shrink changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^    if let (true, Given::Owned(owned)) = (lean, \&mut program) {$/    if let (true, Given::Owned(owned)) = (false, \&mut program) {/' src/codegen.rs
grep -qF '(false, &mut program) {' src/codegen.rs
