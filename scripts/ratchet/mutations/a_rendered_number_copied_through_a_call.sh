#!/bin/sh
# A number renders its digits into a stack buffer and then builds the string
# from them, and the copy went through glibc's memcpy: fifteen instructions
# to choose how to move a handful of digits, 579,291 times a run on runbench.
# Since 2026-09-07 the digits go over as sixteen bytes and eight more past
# fifteen -- the buffer is sixty-four bytes and the string's storage is
# rounded up to sixteen, so both sides have the room -- with the call kept
# only for a rendering past twenty-three bytes. This mutation sends every
# rendering back through the call, by making the first copy exact. The bytes
# out are the same either way and no counter moves, so the work vein is the
# witness: runbench 2,649,866,050 -> 2,637,227,442 on the container when
# the words landed.
set -e
target='    memcpy(s->data, buf, 16);'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the render's word copy changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^    memcpy(s->data, buf, 16);$|    memcpy(s->data, buf, (size_t)nlen);|' src/runtime.c
grep -qF '    memcpy(s->data, buf, (size_t)nlen);' src/runtime.c
