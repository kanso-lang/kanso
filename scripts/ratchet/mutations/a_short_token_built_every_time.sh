#!/bin/sh
# A short token is built again each time the decoder reads it.
#
# k_token_of hands a token of four to seven bytes back from a cache of
# permanent strings, for both utf8 doors. With its two hit tests gone every
# read misses, and a miss allocates, validates and copies, so the mem vein's
# short-token fixture reads 600 strings in the arena where it pinned five
# permanent ones.
set -e
f=src/runtime.c
grep -q 'k_token_miss' "$f"
for slot in 'slot' 'slot ^ 1'; do
  line="    if (k_token_key[$slot] == key) return k_token_cache[$slot];"
  [ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
    echo "the short-token cache moved; this mutation needs rewriting" >&2
    exit 1
  }
done
sed -i 's/^    if (k_token_key\[slot\(.*\)\] == key) return/    if (0 \&\& k_token_key[slot\1] == key) return/' "$f"
[ "$(grep -c '^    if (0 && k_token_key\[slot' "$f")" -eq 2 ]
