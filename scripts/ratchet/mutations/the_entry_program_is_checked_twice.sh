#!/bin/sh
# A defect the entry vein sees and the compile vein cannot.
#
# The whole-program check on the ENTRY path is src/lib.rs's call in
# compile_parsed_entry. Asking it twice is the shape of every regression this
# vein was opened for -- work done once per compile that nobody notices,
# because the row watching the front end checks a DIRECTORY and a directory
# takes compile_module_inner instead. bench/compile_corpus never reaches this
# call, so compile_instructions cannot move for it however much work it does.
#
# It is not a subtle defect and it is not meant to be. A mutation proves a gate
# CAN go red; the argument that this one is worth having is that the same
# mutation leaves the compile row's own gate green.
set -e
file=src/lib.rs
# kanso#1335 moved the alias pass in front of this check and renamed the call,
# so the anchor is the new one. The shape the mutation makes is unchanged: the
# entry's whole-program check asked twice.
anchor='let merged_diags = check::check_merged_after_aliases(&merged, true, &rewritten);'
before=$(grep -cF "$anchor" "$file" || true)
if [ "$before" -ne 1 ]; then
  echo "expected one entry-path check_merged call in $file, found $before;" >&2
  echo "the call moved and this mutation needs rewriting" >&2
  exit 1
fi
awk -v anchor="$anchor" '
  index($0, anchor) {
    print "    let _ = check::check_merged_after_aliases(&merged, true, &rewritten);"
    print
    next
  }
  { print }
' "$file" > "$file.mutated"
mv "$file.mutated" "$file"
after=$(grep -cF 'check::check_merged_after_aliases(&merged, true, &rewritten)' "$file" || true)
if [ "$after" -ne 2 ]; then
  echo "wanted the entry check asked twice, and the file holds $after calls" >&2
  exit 1
fi
