#!/bin/sh
# A run of byte compares reads `data[p + k - 1]` for the language's `cs[p + k]`,
# since positions count from one. This mutation drops the `- 1`, so every
# fused read lands one byte late; the micro fixture
# a_byte_run_reads_its_window_once then answers differently from the
# interpreter.
set -e
line='            f.line(&format!("{off} = add i64 {idx}, {}", k - 1));'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the byte run's read moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            f.line(&format!("{off} = add i64 {idx}, {}", k - 1));$/            f.line(\&format!("{off} = add i64 {idx}, {}", k));/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
