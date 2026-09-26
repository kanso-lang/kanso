#!/bin/sh
# An index read assumes nothing about its container's length.
#
# A length is never negative, and saying so lets LLVM fold `1 <= i <= len`
# into one unsigned compare. Take the promise back and the read is two signed
# compares and an `and` again.
set -e
f=src/codegen.rs
grep -q 'fn assume_length(' src/codegen.rs
line='    f.line(&format!("{ok} = icmp sge i64 {len}, 0"));'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the length assumption moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    f.line(&format!("{ok} = icmp sge i64 {len}, 0"));$/    f.line(\&format!("{ok} = icmp sge i64 0, 0"));/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
