#!/bin/sh
# A type twin's qualified origin is prefixed with the importing module's name.
#
# An import's pub type also exists under its short name, as a clone that
# remembers the declaration it came from, `list/step`. When the module holding
# that clone is imported in turn, the loader renames it `steps/step` and leaves
# its already-qualified origin alone. Prefixed again, the origin names a type
# nothing declares: an ascription `_:steps/step` misses a step in the
# interpreter, and native refuses the build with `unknown type`.
set -e
f=src/lib.rs
grep -q "if let Some(o) = ty.origin.as_mut().filter(|o| !ast::has_slash(o)) {" src/lib.rs
line='        if let Some(o) = ty.origin.as_mut().filter(|o| !ast::has_slash(o)) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the origin filter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if let Some(o) = ty\.origin\.as_mut()\.filter(|o| !ast::has_slash(o)) {$/        if let Some(o) = ty.origin.as_mut() {/' "$f"
grep -qxF '        if let Some(o) = ty.origin.as_mut() {' "$f"
