#!/bin/sh
# The module writer reads the calls and the line-ending symbols of every
# piece in one pass over its ats. This mutation scans each piece a second
# time for the endings, as the two separate scans did. Same sets, and the
# browser compile row reads heavier.
set -e
t='            called_and_ending(piece, &mut body_calls, &mut body_lines);'
[ "$(grep -cxF "$t" src/codegen/module.rs)" -eq 1 ] || {
  echo "the module's symbol scan moved; this mutation needs rewriting" >&2
  exit 1
}
awk -v t="$t" '
  $0 == t {
    print
    print "            called_and_ending(piece, &mut crate::hash::Set::default(), &mut body_lines);"
    next
  }
  { print }' src/codegen/module.rs > src/codegen/module.rs.new
mv src/codegen/module.rs.new src/codegen/module.rs
grep -qxF '            called_and_ending(piece, &mut crate::hash::Set::default(), &mut body_lines);' src/codegen/module.rs
