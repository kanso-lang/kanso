#!/bin/sh
# The prune that drops unreached definitions finds each line of the emitted
# IR a word at a time. This mutation puts the per-line `str::find` back, which
# calls memchr once a line and on wasm32 reads four bytes a step. Same
# segments, and the tab's compile reads heavier.
set -e
f=src/codegen/ir_scan.rs
t="        let end = crate::compare::byte_from(bytes, at, b'\\\\n');"
grep -qF "let end = crate::compare::byte_from(bytes, at, b'\\n');" "$f" || {
  echo "the prune's line step moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s/        let end = crate::compare::byte_from(bytes, at, b'\\\\n');/        let end = body[at..].find('\\\\n').map_or(bytes.len(), |n| at + n);/" "$f"
grep -qF "let end = body[at..].find('\\n').map_or(bytes.len(), |n| at + n);" "$f"
