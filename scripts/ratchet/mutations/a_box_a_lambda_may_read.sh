#!/bin/sh
# A lambda applied to a box reads its parameter as a value unrefused.
#
# `box . (n -> n - 1)` hands the lambda the box itself, since the plain dot
# opens nothing. The checker reads the lambda's body for the places that
# parameter meets an operator, an index, a field or a reading builtin, and
# without that the error corpus's a_box_a_lambda_reads checks clean.
set -e
f=src/check.rs
grep -q 'fn read_as_value' src/check.rs
line='                    if param != "_" && is_box(arg) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the lambda parameter's box check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    if param != "_" \&\& is_box(arg) {$/                    if false \&\& is_box(arg) {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
