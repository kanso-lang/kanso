#!/bin/sh
# The token table stores a token's first and last four bytes and not its
# length, which is sound because the length also chooses the slot. This
# mutation chooses the slot from the two words alone, so "abab" and "ababab"
# share a slot and a key, and the second reads back as the first.
#
# a_shared_key_reads_back_as_itself prints "abab 4" where it said "ababab 6"
# and finds one key in a map of two.
set -e
line='        unsigned slot = (unsigned)(((key ^ (uint64_t)len) * K_TOKEN_MUL) >> (64 - K_TOKEN_BITS));'
grep -qF "$line" src/runtime.c || {
  echo "the token slot changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|((key ^ (uint64_t)len) \* K_TOKEN_MUL)|(key * K_TOKEN_MUL)|' src/runtime.c
grep -qF '(unsigned)((key * K_TOKEN_MUL) >> (64 - K_TOKEN_BITS));' src/runtime.c
