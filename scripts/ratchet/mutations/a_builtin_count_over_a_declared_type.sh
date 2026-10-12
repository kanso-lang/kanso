#!/bin/sh
# The checker's arity guard lets a declared type shadow a builtin.
#
# Without it `type run` built with `run 1 2 3` is refused with the count of
# the builtin std/os wraps. a_type_named_like_a_builtin_is_the_files_own sees
# the refusal.
set -e
f=src/check.rs
line='    if known.is_none() && !named.fields.contains_key(name) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the arity guard moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if known.is_none() \&\& !named.fields.contains_key(name) {$/    if known.is_none() {/' "$f"
grep -qxF '    if known.is_none() {' "$f"
