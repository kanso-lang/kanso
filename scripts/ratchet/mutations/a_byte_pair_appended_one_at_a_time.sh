#!/bin/sh
# Two byte appends in a row are merged into one call after the body is
# emitted. This mutation skips the merge, so each byte asks for room on its
# own again. The answers are the same; the work vein is the witness.
set -e
line='        let rewritten = match paired_appends(&pruned) {'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the byte pair's merge moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let rewritten = match paired_appends(&pruned) {$/        let rewritten = match std::borrow::Cow::<str>::Borrowed(pruned.as_str()) {/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
