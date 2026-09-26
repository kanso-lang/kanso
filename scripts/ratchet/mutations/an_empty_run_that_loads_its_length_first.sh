#!/bin/sh
# The append of a slice loads its container's length before the span's first
# bounds test.
#
# Loaded between the two tests, the pair stays two branches. Loaded first,
# both compares sit side by side and LLVM folds them back into flags.
set -e
f=src/codegen.rs
grep -q '%qspanhi' src/codegen.rs
line='  %qclen = load i64, ptr %qc'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the span's length load moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i -e '/^  %qclen = load i64, ptr %qc$/d' \
  -e 's/^  %qc = inttoptr i64 %qcp to ptr$/  %qc = inttoptr i64 %qcp to ptr\n  %qclen = load i64, ptr %qc/' "$f"
[ "$(grep -cxF "$line" "$f")" -eq 1 ]
