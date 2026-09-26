#!/bin/sh
# The slice walk counts a word's character starts with the builtin popcount
# again, which the ssse3 build writes out in shifts and masks. The counts are
# the same and every word costs a dozen more instructions.
set -e
f=src/runtime.c
grep -q 'long long leads = 8 - (long long)(((cont >> 7) \* 0x0101010101010101ULL) >> 56);' src/runtime.c
sed -i 's/long long leads = 8 - (long long)(((cont >> 7) \* 0x0101010101010101ULL) >> 56);/long long leads = 8 - __builtin_popcountll(cont);/' "$f"
if grep -q 'long long leads = 8 - (long long)(((cont >> 7)' "$f"; then exit 1; fi
