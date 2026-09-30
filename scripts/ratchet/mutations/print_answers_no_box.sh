#!/bin/sh
# `print` is forgotten as a box.
#
# The box check counts a call of `print` as a box beside a call of `effect`,
# ahead of the short circuit that skips programs no group answers a box in.
# Without it, the error corpus's a_printed_box_where_a_value_is_expected checks
# clean and dies at run time with "`+` is not defined for these values".
set -e
f=src/check.rs
grep -q 'if name == "effect" || name == "print") =>' src/check.rs
line='                        if name == "effect" || name == "print") =>'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the hand-built box's arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                        if name == "effect" || name == "print") =>$/                        if name == "effect") =>/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
