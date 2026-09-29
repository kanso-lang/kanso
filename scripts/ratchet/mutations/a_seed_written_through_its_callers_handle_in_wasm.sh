#!/bin/sh
# The wasm engine writes a seed in place through the handle its caller reads.
#
# At a call `linear::handed_over_pushes` names, the backend passes the
# argument through `rt_rehandle`, so the parameter writes through a handle
# nobody else holds. Without it the write empties the caller's own handle, and
# the caller reads `[]` where it put `[0]`.
set -e
f=src/wasm_backend.rs
grep -q "RT_REHANDLE" src/wasm_backend.rs
grep -qF '                if self.rehandles.contains(&edge) {' "$f" || {
  echo "the rehandle at a call moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                if self.rehandles.contains(&edge) {$/                if false \&\& self.rehandles.contains(\&edge) {/' "$f"
grep -qF 'if false && self.rehandles.contains(&edge)' "$f"
