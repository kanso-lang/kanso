#!/bin/sh
# The prune compacts each piece's bytes around whole blocks and no longer
# validates them again in a release build (2026-10-23). This mutation
# validates them, and the browser compile row is the witness.
set -e
target='            unsafe { String::from_utf8_unchecked(bytes) }'
n=$(grep -cxF "$target" src/codegen/prune.rs)
[ "$n" -eq 1 ] || { echo "the prune changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^            unsafe { String::from_utf8_unchecked(bytes) }$/            String::from_utf8(bytes).expect("every block ends at a line break")/' src/codegen/prune.rs
grep -qxF '            String::from_utf8(bytes).expect("every block ends at a line break")' src/codegen/prune.rs
