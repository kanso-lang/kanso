#!/bin/sh
# The wasm engine copies an accumulator on every write the linearity analysis
# proved it could extend.
#
# `builtin_door` sends `push`, `put` and `append` at a proven site to
# `rt_builtin_mut`, which moves the container out of its handle before the
# builtin sees it. Sending every call to `rt_builtin` instead leaves the
# registry holding the old container, so each write copies it and keeps the
# copy: an answer that is still right on small inputs, and quadratic memory on
# large ones.
set -e
f=src/wasm_backend.rs
grep -q 'fn builtin_door' src/wasm_backend.rs
grep -qF '            true => RT_BUILTIN_MUT,' "$f" || {
  echo "the in-place door moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^            true => RT_BUILTIN_MUT,$|            true => RT_BUILTIN,|' "$f"
grep -qF '            true => RT_BUILTIN,' "$f"
