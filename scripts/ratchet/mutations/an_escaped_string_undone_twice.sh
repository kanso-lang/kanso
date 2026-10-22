#!/bin/sh
# `k_json_escaped` undoes a JSON string's escapes in the walk that finds its
# closing quote, and keeps the string for the `unescape` call that follows.
# This mutation keeps nothing, so `unescape` reads the string a second time.
# The output is the same; the witness is the work vein.
set -e
line='    k_unescaped_d = d;'
n=$(grep -cxF "$line" src/runtime/text.c)
[ "$n" -eq 1 ] || {
  echo "the kept string's key moved ($n); this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    k_unescaped_d = d;$/    k_unescaped_d = NULL;/' src/runtime/text.c
[ "$(grep -cxF '    k_unescaped_d = NULL;' src/runtime/text.c)" -eq 2 ]
