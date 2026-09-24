#!/bin/sh
# A group of clauses that switches on its argument's type compiles to one
# function, and LLVM gives it one frame. `kept_out` marks the loop-bearing
# callees of its heavy arms `noinline`, so the arms that only append a scalar
# stop paying the six callee-saved pushes of an inlined escaper. This mutation
# takes the mark off both headers that write it. The bytes out are the same
# and no allocation moves, so the work vein is the witness: encodebench,
# livebench and runbench all rise.
set -e
target='        let apart = if self.kept_out.contains(name) { " noinline" } else { "" };'
n=$(grep -cF "$target" src/codegen.rs)
[ "$n" -eq 2 ] || { echo "the noinline mark changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        let apart = if self.kept_out.contains(name) { " noinline" } else { "" };$|        let apart = if self.kept_out.contains(name) { "" } else { "" };|' src/codegen.rs
[ "$(grep -cF 'let apart = if self.kept_out.contains(name) { "" } else { "" };' src/codegen.rs)" -eq 2 ]
