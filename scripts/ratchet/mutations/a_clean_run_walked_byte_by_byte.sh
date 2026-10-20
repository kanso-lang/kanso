#!/bin/sh
# A clean run between two escapes is copied a byte at a time again, as the
# kanso escaper's fold did before 2026-09-07 -- bytes the word test had
# already found clean. Since 2026-10-20 the run is copied by
# `k_json_escape_into` in the runtime, eight bytes a step; this mutation turns
# the word loop off, so the byte loop after it does all the copying. The
# output is the same bytes and no allocation counter moves, so the work vein
# is the witness: livebench and the run program escape a string in every
# sixth value they write.
set -e
target='        while (i + 8 <= len) {'
n=$(grep -cF "$target" src/runtime/text.c)
[ "$n" -eq 1 ] || { echo "k_json_escape_into changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        while (i + 8 <= len) {$|        while (0 \&\& i + 8 <= len) {|' src/runtime/text.c
grep -qF '        while (0 && i + 8 <= len) {' src/runtime/text.c
