#!/bin/sh
# The decoder's short tokens are interned: a slice of four to seven bytes the
# table already holds comes back as the interned string. This mutation never
# looks, so every key is a fresh string in the arena again.
#
# Nothing a program prints changes. a_repeated_key_is_one_string reads
# token_hits=0 against 398 and 400 more allocations.
set -e
grep -q '^        if (hit && k_token_key\[slot\] == key) {$' src/runtime.c || {
  echo "the token lookup changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^        if (hit \&\& k_token_key\[slot\] == key) {$|        if (0) {|' src/runtime.c
sed -i 's|^        if (!hit) return k_token_miss(key, len, slot);$|        if (0) return k_token_miss(key, len, slot);|' src/runtime.c
grep -q '^        if (0) return k_token_miss(key, len, slot);$' src/runtime.c
