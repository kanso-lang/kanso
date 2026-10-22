#!/bin/sh
# `k_json_escaped` undoes a JSON string's escapes and copies the run after
# each one sixteen bytes at a time. This mutation takes no block, so every
# run goes a word at a time. The output is the same; the witness is the work
# vein.
set -e
line='        while (p + 16 <= len) {'
grep -qxF "$line" src/runtime/text.c || {
  echo "the undo walk's block loop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        while (p + 16 <= len) {$/        while (0) {/' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
