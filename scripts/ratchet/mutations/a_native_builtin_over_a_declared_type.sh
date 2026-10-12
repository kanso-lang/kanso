#!/bin/sh
# Native code lets a declared type shadow a builtin.
#
# Without it a play file's `run 1 2 3` stops the build with the builtin's
# count. a_type_named_like_a_builtin_is_the_files_own sees the failed build.
set -e
f=src/codegen.rs
line='            !was_builtin && (self.declares(name) || self.type_ids.contains_key(name));'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the native shadow moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            !was_builtin \&\& (self.declares(name) || self.type_ids.contains_key(name));$/            !was_builtin \&\& self.declares(name);/' "$f"
grep -qxF '            !was_builtin && self.declares(name);' "$f"
