#!/bin/sh
# `k_str_n` builds every string the runtime does not already have cached, and
# its copy went through glibc's memcpy. On runbench that call is reached
# 177,706 times from `split` alone at 12.9 instructions apiece -- the copies
# are two or three bytes and the call is most of what they cost. Since
# 2026-09-08 anything shorter than sixteen bytes goes over as two overlapping
# loads and two overlapping stores. This mutation sends every one of them back
# through the call. The bytes out are the same either way and no counter
# moves, so the work vein is the witness.
set -e
target='    k_copy_short(s->data, data, len);'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_str_n's copy changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^    k_copy_short(s->data, data, len);$|    memcpy(s->data, data, len);|' src/runtime.c
grep -qF '    memcpy(s->data, data, len);' src/runtime.c
