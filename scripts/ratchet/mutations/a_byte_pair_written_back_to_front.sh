#!/bin/sh
# Two byte appends in a row are one call that asks for room once and stores
# both bytes. This mutation stores the second byte first, so every json escape
# comes out letter-first; the micro fixture two_bytes_appended_at_once then
# answers differently from the interpreter.
set -e
first='  store i8 %xb, ptr %dst'
second='  store i8 %yb, ptr %dst1'
[ "$(grep -cxF "$first" src/codegen.rs)" -eq 1 ] && [ "$(grep -cxF "$second" src/codegen.rs)" -eq 1 ] || {
  echo "the byte pair's stores moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i -e 's/^  store i8 %xb, ptr %dst$/  store i8 %xb, ptr %dst1/' \
       -e 's/^  store i8 %yb, ptr %dst1$/  store i8 %yb, ptr %dst/' src/codegen.rs
if grep -qxF "$first" src/codegen.rs; then exit 1; fi
