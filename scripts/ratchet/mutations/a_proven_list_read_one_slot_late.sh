#!/bin/sh
# A plain index of a proven list loads `items[i - 1]` for the language's
# `xs[i]`, since positions count from one. This mutation loads `items[i]`, so
# every direct read lands one slot late; the micro fixture
# a_proven_list_is_read_in_place then answers differently from the interpreter.
set -e
line='            f.line(&format!("{slot} = getelementptr %KValue, ptr {items}, i64 {off}"));'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the proven list's read moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            f.line(&format!("{slot} = getelementptr %KValue, ptr {items}, i64 {off}"));$/            f.line(\&format!("{slot} = getelementptr %KValue, ptr {items}, i64 {idx}"));/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
