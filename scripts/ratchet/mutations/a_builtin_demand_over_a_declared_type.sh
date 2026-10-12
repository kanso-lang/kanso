#!/bin/sh
# The checker's literal guard lets a declared type shadow a builtin.
#
# Without it `run 1 2 3` is refused because the builtin `run` takes a
# string first. a_type_named_like_a_builtin_is_the_files_own sees the
# refusal.
set -e
f=src/check.rs
line='    let shadowed = !forwards && (declared || types.contains_key(name.as_str()));'
[ "$(grep -cxF "$line" src/check.rs)" -eq 1 ] || {
  echo "the literal guard moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let shadowed = !forwards \&\& (declared || types.contains_key(name.as_str()));$/    let shadowed = !forwards \&\& declared;/' "$f"
grep -qxF '    let shadowed = !forwards && declared;' "$f"
