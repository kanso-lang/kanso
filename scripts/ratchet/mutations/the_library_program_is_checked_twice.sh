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
# `let merged_diags = check::check_merged_after_aliases(&program, false,
# &rewritten);` appears TWICE in src/lib.rs -- compile_one carries a block
# byte-identical to this one -- and a sed on that literal would patch whichever
# came first. So the function is found by its signature, which appears exactly
# once, and the duplicate goes in at the first such call after it. Both counts
# are asserted before anything is written.
#
# THE ANCHOR HAS GONE STALE ONCE, and the ratchet is what said so. The line was
# `check::check_merged(&program, false)` when this mutation was written, and the
# alias-pass reorder rewrote exactly that line at both sites in the same commit
# that made this row worth having. The ratchet's first pass -- every mutation
# still matches the source it patches -- caught it on the runner before the row
# it guards was ever read. A mutation anchored on a line a change is about to
# rewrite is a mutation that goes stale in that change, which is the ordinary
# case rather than a surprise.
#
# It is not a subtle defect and it is not meant to be. A mutation proves a gate
# CAN go red; the argument that this one is worth having is that the two rows
# beside it stay green through it.
set -e
file=src/lib.rs

# THE PATH IS SPELLED ON A GUARD LINE, and that is load-bearing rather than
# decorative. The ratchet's `touched` pass selects the rows a branch could have
# made blind by intersecting the files a branch changed with the paths each
# mutation names on a `grep -q` line. Every assertion below reaches src/lib.rs
# through "$file", so the path appeared on no guard line and this row was
# invisible to that pass -- it could never be selected, whatever a branch
# touched. Proved on kanso#1338, whose diff rewrites this very call: the pass
# selected three rows and neither this one nor its entry twin was among them.
grep -q 'pub fn compile_library' src/lib.rs
sig='pub fn compile_library(file: &str, source: &str) -> Result<ast::Program, String> {'
call='    let merged_diags = check::check_merged_after_aliases(&program, false, &rewritten);'
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
    print "    let _ = check::check_merged_after_aliases(&program, false, &rewritten);"
    print
    done = 1
    next
  }
  { print }
' "$file" > "$file.mutated"
mv "$file.mutated" "$file"
after=$(grep -c 'check::check_merged_after_aliases(&program, false, &rewritten)' "$file" || true)
if [ "$after" -ne 3 ]; then
  echo "wanted the library check asked twice, and the file holds $after calls" >&2
  exit 1
fi
