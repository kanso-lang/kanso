#!/bin/sh
# A template whose first piece is a field of a record the arm is finished with
# joins into that field's string in place. This mutation takes the field away
# from the join sites, so every pass copies the whole string into a new one.
# The output is the same; the mem vein's `a_record_field_joined_in_place`
# allocates a copy a pass, and the run program's restate phase does the same
# work as before.
set -e
line='                    if a.field_moves(p, f, decl) {'
grep -qxF "$line" src/linear.rs || {
  echo "the field join's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    if a.field_moves(p, f, decl) {$/                    if false \&\& a.field_moves(p, f, decl) {/' src/linear.rs
if grep -qxF "$line" src/linear.rs; then exit 1; fi
