#!/bin/sh
# `s[i]` hands back one character and writes its count memo, so `length s[i]`
# reads a header field instead of walking the character's bytes. This puts
# the scan back: the memo is written as zero, which is "not yet counted", and
# every multi-byte character handed out is scanned once more by `length`.
# str_scans in the fixture rises by one per non-ascii position, 2,003 on
# 4,000, and the mem corpus goes red on that row.
set -e
grep -q '^        if (os->cap == 0) os->cap = -2;$' src/runtime.c || {
  echo "k_b_at's one-character memo changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^        if (os->cap == 0) os->cap = -2;$|        if (os->cap == 0) os->cap = 0;|' \
  src/runtime.c
grep -q '^        if (os->cap == 0) os->cap = 0;$' src/runtime.c
