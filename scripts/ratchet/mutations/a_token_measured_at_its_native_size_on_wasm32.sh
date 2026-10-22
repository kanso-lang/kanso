#!/bin/sh
# A shared token is 24 bytes natively and 20 on wasm32, and two WASM32_LINES
# rows give the key builtin's inline path the tab's numbers: the store's size
# and the division that finds a token's clean flag. This mutation gives the
# tab the native numbers back. The witness is
# a_dirty_key_beside_a_clean_one_is_escaped, whose dirty key then reads a
# clean neighbour's flag and goes out unescaped.
set -e
bound='    ("%intok = icmp ult i64 %off, 98304", "%intok = icmp ult i64 %off, 81920"),'
slot='    ("%slot = udiv i64 %off, 24", "%slot = udiv i64 %off, 20"),'
grep -qxF "$bound" src/codegen/wasm32.rs && grep -qxF "$slot" src/codegen/wasm32.rs || {
  echo "the token store's wasm32 rows moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/"%intok = icmp ult i64 %off, 81920")/"%intok = icmp ult i64 %off, 98304")/; s/"%slot = udiv i64 %off, 20")/"%slot = udiv i64 %off, 24")/' src/codegen/wasm32.rs
[ "$(grep -cF '%off, 98304", "%intok = icmp ult i64 %off, 98304")' src/codegen/wasm32.rs)" -eq 1 ]
[ "$(grep -cF '%off, 24", "%slot = udiv i64 %off, 24")' src/codegen/wasm32.rs)" -eq 1 ]
