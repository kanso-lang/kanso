#!/bin/sh
# The decoder's key helper finds a short key in the token cache itself. This
# mutation sends every key to the runtime's slice instead, which finds the
# same string in the same cache after a call. The output is the same; the
# witness is the work vein.
set -e
line='  br i1 %short, label %token, label %general'
grep -qxF "$line" src/codegen/declares.rs || {
  echo "the short-key branch moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^  br i1 %short, label %token, label %general$/  br label %general/' src/codegen/declares.rs
if grep -qxF "$line" src/codegen/declares.rs; then exit 1; fi
