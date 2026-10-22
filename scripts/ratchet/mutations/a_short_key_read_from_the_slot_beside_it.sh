#!/bin/sh
# The decoder's key helper finds a short key in the token cache itself. This
# mutation reads the cached string from the slot beside the one whose key
# matched, so a key that shares its slot with another answers the other one.
# The micro golden decodes such a pair.
set -e
line='  %tcp = getelementptr %KValue, ptr @k_token_cache, i64 %tslot'
grep -qxF "$line" src/codegen/declares.rs || {
  echo "the cached read moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^  %tcp = getelementptr %KValue, ptr @k_token_cache, i64 %tslot$/  %tsl1 = xor i64 %tslot, 1\n  %tcp = getelementptr %KValue, ptr @k_token_cache, i64 %tsl1/' src/codegen/declares.rs
if grep -qxF "$line" src/codegen/declares.rs; then exit 1; fi
