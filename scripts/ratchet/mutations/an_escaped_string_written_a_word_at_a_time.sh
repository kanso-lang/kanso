#!/bin/sh
# `k_json_escape_into` copies a string's bytes to the next one JSON has to
# escape sixteen at a time. This mutation takes no block, so every string
# goes through the word and byte walk. The output is the same; the witness
# is the work vein.
set -e
line='        if (left >= 16) {'
tail='        } else if (k_tail_window(d + i)) {'
for l in "$line" "$tail"; do
  grep -qxF "$l" src/runtime/text.c || {
    echo "the escape writer's block test moved; this mutation needs rewriting" >&2
    exit 1
  }
done
sed -i 's/^        if (left >= 16) {$/        if (0) {/; s/^        } else if (k_tail_window(d + i)) {$/        } else if (0) {/' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
if grep -qxF "$tail" src/runtime/text.c; then exit 1; fi
