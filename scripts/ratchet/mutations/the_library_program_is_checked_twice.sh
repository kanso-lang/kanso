#!/bin/sh
# A defect the library vein sees and neither of the other two compile rows can.
#
# `kanso check` routes a single file by content: a directory is a module and
# takes compile_module_inner, a file with bare statements is an entry and takes
# compile_parsed_entry, a file of definitions alone is a library and takes
# compile_library. bench/compile_corpus is the first and bench/entry_corpus the
# second, so neither of their gates ever enters this function, however much
# work it does. That is the whole argument for the row, and this mutation is
# how it is proved rather than asserted: the same edit leaves
# compile_instructions and entry_instructions green.
#
# THE ANCHOR IS TWO STEPS, because the obvious one-liner is not unique.
# `let merged_diags = check::check_merged(&program, false);` appears TWICE in
# src/lib.rs -- compile_one carries a block byte-identical to this one -- and a
# sed on that literal would patch whichever came first. So the function is
# found by its signature, which appears exactly once, and the duplicate goes in
# at the first such call after it. Both counts are asserted before anything is
# written.
#
# It is not a subtle defect and it is not meant to be. A mutation proves a gate
# CAN go red; the argument that this one is worth having is that the two rows
# beside it stay green through it.
set -e
file=src/lib.rs
sig='pub fn compile_library(file: &str, source: &str) -> Result<ast::Program, String> {'
call='    let merged_diags = check::check_merged(&program, false);'
sigs=$(grep -cF "$sig" "$file" || true)
if [ "$sigs" -ne 1 ]; then
  echo "expected one compile_library signature in $file, found $sigs;" >&2
  echo "the function was renamed or split and this mutation needs rewriting" >&2
  exit 1
fi
before=$(grep -cF "$call" "$file" || true)
if [ "$before" -ne 2 ]; then
  echo "expected two whole-program check calls in $file, found $before;" >&2
  echo "compile_one and compile_library carried one each, and the shape this" >&2
  echo "mutation makes depends on which one it lands in" >&2
  exit 1
fi
awk -v sig="$sig" -v call="$call" '
  index($0, sig) { inside = 1 }
  inside && $0 == call && !done {
    print "    let _ = check::check_merged(&program, false);"
    print
    done = 1
    next
  }
  { print }
' "$file" > "$file.mutated"
mv "$file.mutated" "$file"
after=$(grep -c 'check::check_merged(&program, false)' "$file" || true)
if [ "$after" -ne 3 ]; then
  echo "wanted the library check asked twice, and the file holds $after calls" >&2
  exit 1
fi
