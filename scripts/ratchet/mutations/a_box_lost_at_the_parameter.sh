#!/bin/sh
# A box handed to a parameter is forgotten inside the body.
#
# The box check lets a box into a group position that binds anything, and a
# second reading of the body counts the parameter as the box it was handed.
# Without that reading, the error corpus's a_box_carried_through_a_parameter
# checks clean and dies at run time with "no overload of `seen` matches".
set -e
f=src/check.rs
grep -q 'let mut handed_here' src/check.rs
line='    if handed.is_empty() {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the held-parameter pass's entry moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if handed.is_empty() {$/    if true {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
