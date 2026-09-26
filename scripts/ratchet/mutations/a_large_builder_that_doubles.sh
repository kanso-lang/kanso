#!/bin/sh
# A malloc'd builder past 16 kb doubles again instead of growing by half.
#
# The doubled buffer is what `held_peak_bytes` read before: every encode of
# large.json ended in a 277,522-byte buffer. The mem vein pins the smaller
# peaks the half-step leaves, so doubling turns it red.
set -e
f=src/runtime.c
line='    if (!dies && a->len + n > 16384) cap = ((a->len + n) * 3 / 2) & ~1LL;'
grep -q 'a->len + n > 16384' src/runtime.c
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the half-step grow moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (!dies \&\& a->len + n > 16384) cap = ((a->len + n) \* 3 \/ 2) \& ~1LL;$/    if (!dies \&\& a->len + n > 16384) cap = 2 * (a->len + n);/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
