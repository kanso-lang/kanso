#!/bin/sh
# `s[i]` hands back one character and writes its count memo, so `length s[i]`
# reads a header field instead of walking the character's bytes. This puts
# the scan back on both arms: the ascii character's memo is written as zero,
# which is "not yet counted", and the wide character's header is left at
# zero the same way, so every multi-byte character handed out is scanned once
# more by `length`. str_scans in the fixture rises by one per non-ascii
# position, 2,003 on 4,000, and the mem corpus goes red on that row.
#
# The wide-character memo lives in k_b_at_wide_miss. The ascii one is written
# in two places: k_at_char, where the cache answers, and k_at_ascii_miss,
# where it does not; the anchors follow both.
set -e
hit='            if (os->cap == 0) os->cap = -2;'
miss='    if (os->cap == 0) os->cap = -2;'
wide='    os->cap = -2;'
for l in "$hit" "$miss" "$wide"; do
  [ "$(grep -cxF "$l" src/runtime/text.c)" -eq 1 ] || {
    echo "k_b_at_str's count memos changed shape; this needs rewriting" >&2
    exit 1
  }
done
sed -i 's|^            if (os->cap == 0) os->cap = -2;$|            if (os->cap == 0) os->cap = 0;|' src/runtime/text.c
sed -i 's|^    if (os->cap == 0) os->cap = -2;$|    if (os->cap == 0) os->cap = 0;|' src/runtime/text.c
sed -i 's|^    os->cap = -2;$|    os->cap = 0;|' src/runtime/text.c
for l in "$hit" "$miss" "$wide"; do
  if grep -qxF "$l" src/runtime/text.c; then exit 1; fi
done
