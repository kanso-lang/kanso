#!/bin/sh
# Indexing a string by position hands back one character, and since
# 2026-09-07 a character of two, three or four bytes comes back from a
# permanent string the first index built, the way an ascii one comes from
# k_str_n's cache: runbench's index phase meets the same six characters
# 690,000 times, and building each in the arena was an allocation and a
# hundred instructions a step; the cursor answers the next character
# directly since the same day. This mutation sends every wide character
# down the arena path again and takes the cursor's shortcut away. The bytes out are the same; the work vein and
# the allocation counters both see it, and the work vein is the one asked.
set -e
target='        if (k_wide_key[slot] == key && k_wide_ready[slot]) return k_wide_cache[slot];'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_b_at changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        if (k_wide_key\[slot\] == key \&\& k_wide_ready\[slot\]) return k_wide_cache\[slot\];$|        if (0) return k_wide_cache[slot];|' src/runtime.c
sed -i 's|^        if (!k_wide_ready\[slot\]) {$|        if (0) {|' src/runtime.c
# and the cursor's next-character path, which landed with the cache and is
# most of what the index step stopped paying
sed -i 's|^        if (from == k_seek_char + 1) {$|        if (0) {|' src/runtime.c
grep -qF '        if (0) return k_wide_cache[slot];' src/runtime.c
grep -qF '        if (0) {' src/runtime.c
! grep -qF '        if (from == k_seek_char + 1) {' src/runtime.c
