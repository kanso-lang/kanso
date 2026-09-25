#!/bin/sh
# A release module marks a lambda's wrapper alwaysinline, so a fold over a
# known lambda carries the lambda's body in its loop. This mutation drops the
# attribute, the inliner leaves the wrapper out of line again, and
# encodebench's escape loop pays a call per byte. The answers are the same;
# the work vein is the witness.
set -e
line='        let inline = if self.inline_helpers { " alwaysinline" } else { "" };'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the lambda wrapper's attribute moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let inline = if self.inline_helpers { " alwaysinline" } else { "" };$/        let inline = "";/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
