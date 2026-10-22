#!/bin/sh
# A plain number far from the end of its document is read a word at a time.
# This mutation sends every number to the sixteen-byte scan instead, which
# answers the same. The output is the same; the witness is the work vein.
set -e
line='    if (from + 40 <= len) {'
grep -qxF "$line" src/runtime/text.c || {
  echo "the fast path's guard moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (from + 40 <= len) {$/    if (0) {/' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
