#!/bin/sh
# A plain number far from the end of its document is read a word at a time.
# This mutation answers a number with a point there as an integer, so the
# decoder reads `3.25` as an int and refuses it. The micro golden decodes
# such numbers ahead of a long tail.
set -e
line='                    if (f >= 0 && !k_number_bytes[d[f]]) return -(f + 1);'
grep -qxF "$line" src/runtime/text.c || {
  echo "the fast path's float answer moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    if (f >= 0 \&\& !k_number_bytes\[d\[f\]\]) return -(f + 1);$/                    if (f >= 0 \&\& !k_number_bytes[d[f]]) return f + 1;/' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
