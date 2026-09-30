#!/bin/sh
# An operator's arms are passed a record's words instead of the record.
#
# `a + b` reaches a user arm through the operator's own dispatch, which hands
# the arm the two values it holds, boxed. Escape analysis keeps an operator's
# groups on the boxed convention for that reason. With operators let through,
# a record of an int and one other value is passed as two words, the arm reads
# the boxed record's tag and pointer as a packed int, and the micro fixture
# prints the two records side by side instead of their sum.
set -e
f=src/escape.rs
grep -q "crate::is_operator(&d.name)" src/escape.rs
line='        if d.synthetic || crate::is_operator(&d.name) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the operator exclusion moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if d\.synthetic || crate::is_operator(&d\.name) {$/        if d.synthetic {/' "$f"
grep -qxF '        if d.synthetic {' "$f"
