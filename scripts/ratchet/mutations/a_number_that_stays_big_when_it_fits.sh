#!/bin/sh
# An arithmetic result that fits a machine word goes back into one, so that a
# number has one form and equality, ordering and map keys can compare forms.
# This mutation keeps every such result in the BigInt form, and the
# interpreter then says `max + 1 - 1` is not `max`.
#
# a_number_back_in_range_is_the_same_number reads false where it wants true.
set -e
anchor='        // An arithmetic result that fits a word goes back into one.'
test "$(grep -cxF "$anchor" src/int.rs)" = 1 || {
  echo "the int's return into a word changed shape; rewrite this" >&2
  exit 1
}
awk -v anchor="$anchor" '
  skip { print "        match None::<i64> {"; skip = 0; next }
  $0 == anchor { skip = 1 }
  { print }
' src/int.rs > src/int.rs.new
mv src/int.rs.new src/int.rs
grep -qxF '        match None::<i64> {' src/int.rs
