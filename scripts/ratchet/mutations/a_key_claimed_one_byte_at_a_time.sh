#!/bin/sh
# An object key that the decoder shared as a clean token is written by
# k_b_append_key_fast as one word between a quote and the closing pair. This
# mutation sends every in-place key to k_b_append_key_mut, which writes the
# same bytes a piece at a time. The output is the same; the witness is the
# emitted gate, where every program that encodes JSON loses the inline path.
set -e
line='                "append_key_fast"'
grep -qxF "$line" src/codegen/emit.rs || {
  echo "the key builtin's fast route moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                "append_key_fast"$/                "append_key_mut"/' src/codegen/emit.rs
[ "$(grep -cxF '                "append_key_mut"' src/codegen/emit.rs)" -eq 1 ]
