#!/bin/sh
# A malloc'd buffer's doubled capacity is written one short.
#
# KBuf.capw is the capacity doubled plus the regime bit, and the setter is the
# one place that spells it. Subtract the bit instead of adding it and every
# malloc'd buffer reads back a slot smaller than it holds, and an empty one
# reads back as a negative capacity.
set -e
f=src/runtime.c
grep -q 'static inline void k_buf_set_cap(KBuf\* b' src/runtime.c
line='    b->capw = 2 * cap + (malloced ? 1 : 0);'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the capacity setter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    b->capw = 2 \* cap + (malloced ? 1 : 0);$/    b->capw = 2 * cap - (malloced ? 1 : 0);/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
