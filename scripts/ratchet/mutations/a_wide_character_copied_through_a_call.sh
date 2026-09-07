#!/bin/sh
# `s[i]` on a string hands back one character, and for a wide one -- two,
# three or four bytes -- k_b_at built the string through k_str_n's memcpy,
# seventeen instructions for glibc to choose how to move three bytes, 345,000
# times a run on runbench. Since 2026-09-07 the character goes over as one
# four-byte word: the read past it stops at the string's own terminator at
# worst, and the write lands inside storage rounded up to sixteen. This
# mutation sends every wide character back through the call. The bytes out
# are the same either way and no counter moves, so the work vein is the
# witness: runbench 2,621,939,707 -> 2,612,624,701 on the container when the
# word landed.
set -e
target='        memcpy(os->data, &q, 4);'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the character's word copy changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        memcpy(os->data, \&q, 4);$|        memcpy(os->data, s->data + at, (size_t)w);|' src/runtime.c
grep -qF '        memcpy(os->data, s->data + at, (size_t)w);' src/runtime.c
