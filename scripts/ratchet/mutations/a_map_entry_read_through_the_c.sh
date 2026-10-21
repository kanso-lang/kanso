#!/bin/sh
# Where a map's sorted view is built, `key_at` is a bounds test and one load,
# written inline by k_b_key_at_fast. This mutation sends every `key_at` to the
# C, which answers the same key from a call. The witness is the emitted gate,
# where every program that encodes an object loses the inline path.
set -e
line='                "key_at_fast"'
grep -qxF "$line" src/codegen/emit.rs || {
  echo "the key_at route moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                "key_at_fast"$/                "key_at"/' src/codegen/emit.rs
[ "$(grep -cxF '                "key_at"' src/codegen/emit.rs)" -eq 1 ]
