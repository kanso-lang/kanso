#!/bin/sh
# Two byte appends in a row are merged into one call after the body is
# emitted. This mutation skips the merge, so each byte asks for room on its
# own again. The answers are the same; the work vein is the witness.
set -e
line='        let pieces: Vec<String> = pruned.into_iter().map(paired_appends).collect();'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the byte pair's merge moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let pieces: Vec<String> = pruned.into_iter().map(paired_appends).collect();$/        let pieces: Vec<String> = pruned;/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
