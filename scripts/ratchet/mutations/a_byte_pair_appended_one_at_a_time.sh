#!/bin/sh
# Two byte appends in a row are merged into one call after the body is
# emitted. This mutation skips the merge, so each byte asks for room on its
# own again. The answers are the same; the work vein is the witness.
set -e
line='        let body = paired_appends(&body);'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the byte pair's merge moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let body = paired_appends(&body);$/        let body = body.to_string();/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
