#!/bin/sh
# A clean run between two escapes is copied a byte at a time again, as the
# kanso escaper's fold did before 2026-09-07 -- bytes the word test had
# already found clean. Since 2026-10-22 the run is copied by
# `k_json_escape_into` in the runtime sixteen bytes a block, and by the word
# walk where no block can be read; this mutation turns off both, so the byte
# loop does all the copying. The output is the same bytes and no allocation
# counter moves, so the work vein is the witness: livebench and the run
# program escape a string in every sixth value they write.
set -e
block='        if (left >= 16) {'
tail='        } else if (k_tail_window(d + i)) {'
word='        if (i + 8 <= len) {'
for l in "$block" "$tail" "$word"; do
  n=$(grep -cxF "$l" src/runtime/json.c)
  [ "$n" -eq 1 ] || { echo "k_json_escape_into changed shape ($n); rewrite this" >&2; exit 1; }
done
sed -i 's|^        if (left >= 16) {$|        if (0) {|; s|^        } else if (k_tail_window(d + i)) {$|        } else if (0) {|; s|^        if (i + 8 <= len) {$|        if (0 \&\& i + 8 <= len) {|' src/runtime/json.c
grep -qxF '        if (0 && i + 8 <= len) {' src/runtime/json.c
