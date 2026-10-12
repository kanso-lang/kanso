#!/bin/sh
# The interpreter calls a declared group before an imported type of the
# same bare name.
#
# Without it a play file's `counting a b` builds std/list's one-field
# record and stops on the count.
# a_function_named_like_an_imported_type_is_the_files_own sees the stop.
set -e
f=src/eval.rs
line='            if ty.origin.is_none() || !self.fns.contains_key(name) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the group-first guard moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if ty.origin.is_none() || !self.fns.contains_key(name) {$/            if true {/' "$f"
grep -qxF '            if true {' "$f"
