#!/bin/sh
# The repair that was reached for twice and is now forbidden.
#
# The front end's instruction count disagreed with itself twice. The first time
# the answer was a per-chip table; the second time a row that pins two values,
# because one binary on one chip had counted 41,831,767 and 41,832,275 and
# nothing explained the 508. Both were retired on 2026-09-05: the 508 was two
# binaries read as one, and eight within-binary sittings across two vendors and
# four CPU generations agree to the instruction.
#
# So a disagreement is a reproduction failure now. It halts the vein and is
# hunted to its source, the way the earlier drift was hunted to Rust's stack
# guard parsing /proc/self/maps. Writing the second number down beside the
# first is the cheap move that ends the hunt, and it ends the row's ability to
# fail with it.
#
# The mutation is that move: a second value on the golden's line.
set -e
golden=bench/compile_instructions_golden.txt
before=$(grep -c '^compile_instructions=[0-9][0-9]*$' "$golden")
if [ "$before" -ne 1 ]; then
  echo "expected one bare value line in $golden, found $before;" >&2
  echo "the shape moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i 's/^\(compile_instructions=[0-9]*\)$/\1 41378765/' "$golden"
after=$(grep -c '^compile_instructions=[0-9][0-9]*$' "$golden")
if [ "$after" -ne 0 ]; then
  echo "wanted the value line paired, and it is still bare" >&2
  exit 1
fi
if ! grep -q '^compile_instructions=[0-9]* [0-9]*$' "$golden"; then
  echo "the second value did not land on the line" >&2
  exit 1
fi
