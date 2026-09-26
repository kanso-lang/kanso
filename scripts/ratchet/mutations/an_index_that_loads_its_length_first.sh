#!/bin/sh
# An index loads its container's length before it branches on the lower bound.
#
# With the load between the two branches the pair stays two compares. Loaded
# first, both compares sit side by side and LLVM folds them back into flags
# and an `and`.
set -e
f=src/codegen.rs
grep -q 'fn index_in_range(' src/codegen.rs
line='    f.line(&format!("{len} = load i64, ptr {len_ptr}"));'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the index's length load moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i -e '/^    let len = f.tmp();$/d' -e '/^    f.line(&format!("{len} = load i64, ptr {len_ptr}"));$/d' \
  -e 's/^    let ge1 = f.tmp();$/    let len = f.tmp();\n    f.line(\&format!("{len} = load i64, ptr {len_ptr}"));\n    let ge1 = f.tmp();/' "$f"
[ "$(grep -cxF "$line" "$f")" -eq 1 ]
