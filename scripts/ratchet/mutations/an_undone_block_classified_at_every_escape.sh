#!/bin/sh
# `k_json_escaped` answers the escapes a sixteen-byte block holds whole from
# the block's one mask, storing the rest of the block again behind each
# undone byte. This mutation goes back to the block loop after every escape,
# so the next one loads and classifies a block of its own. The output and
# the kept string are the same; the witness is the work vein.
set -e
line='                if (!hit || o + 17 > lim) break;'
grep -qxF "$line" src/runtime/text.c || {
  echo "the undo walk's mask loop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                if (!hit || o + 17 > lim) break;$/                break;/' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
