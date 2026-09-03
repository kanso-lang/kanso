#!/bin/sh
# A line in a golden that nobody reads.
#
# bench/compile_instructions_by_cpu.txt is looked up with
# `awk '$1 == k { print $2; exit }'`: the FIRST row matching a key answers and
# every later one is dead. So a chip with two rows has one authority and one
# decoration, and the decoration is the one a reader is most likely to trust,
# because it sits at the bottom of the file where the newest sitting goes.
#
# This is not a hypothesis. The Emerald Rapids row was corrected in place AND
# appended in the same edit on 2026-09-03, and every gate in the tree stayed
# green over the file that resulted.
#
# The mutation removes the refusal, which is the state the file was in when
# that happened.
set -e
row=scripts/gates/compile_ir_row.sh
before=$(grep -c '^  exit 1$' "$row")
if [ "$before" -ne 7 ]; then
  echo "expected seven refusals in $row, found $before;" >&2
  echo "the shape moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i '/^dupes=/,/^fi$/d' "$row"
after=$(grep -c '^  exit 1$' "$row")
if [ "$after" -ne 6 ]; then
  echo "wanted exactly the duplicate refusal removed, $before became $after" >&2
  exit 1
fi
if grep -q 'uniq -d' "$row"; then
  echo "the duplicate check survived the cut" >&2
  exit 1
fi
