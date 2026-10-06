#!/bin/sh
# The check that keeps a tie's references in constructor fields stops running.
#
# A maker that reads through `ref`, or hands it to a helper that does, then
# compiles, and the interpreter stops at run time on a node still being made;
# a literal link to an id the list does not hold compiles and answers a broken
# link. The error corpus pins all three refusals.
set -e
grep -qxF '    check_tie_references(program, &mut diags);' src/check.rs || {
  echo "the tie check's call moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    check_tie_references(program, &mut diags);$/d' src/check.rs
! grep -qxF '    check_tie_references(program, &mut diags);' src/check.rs
