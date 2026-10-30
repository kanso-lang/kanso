#!/bin/sh
# An index read assumes nothing about its container's length.
#
# A length is never negative and is far below the largest word, and saying so
# lets LLVM fold `1 <= i <= len` into one unsigned compare. Take the promise
# back and the read is two signed compares and an `and` again.
set -e
f=src/codegen/words.rs
grep -q 'fn assume_length(' src/codegen/words.rs
line='    f.line_fmt(format_args!("{ok} = icmp ult i64 {len}, {}", 1i64 << 62));'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the length assumption moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    f.line_fmt(format_args!("{ok} = icmp ult i64 {len}, {}", 1i64 << 62));$/    f.line_fmt(format_args!("{ok} = icmp ult i64 0, {}", 1i64 << 62));/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
