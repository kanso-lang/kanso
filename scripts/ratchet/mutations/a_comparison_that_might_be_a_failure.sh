#!/bin/sh
# `inline_not_failure`'s fold stops firing: the mask it tests against goes from
# BOOL to nothing, so the condition asks for a set with no shapes in it at all
# and the `set != 0` beside it then rules every value out. Proved booleans get
# their tag test written again and the fall shows in all thirteen programs'
# `calls` and `lines`.
#
# The programs answer the same bytes either way, because testing a value that
# is not a failure answers true. What moves is the emitted code, which is the
# dimension #1064 built this gate for, and the reason this row is gated there
# rather than on a counter: removing a tag test allocates nothing, and all
# twelve cost veins were byte-identical across the change that added it.
#
# runbench's `calls`, measured 2026-09-07 over the two halves of the change:
#
#     phi keeps ERR, no fold   6026   (main)
#     phi drops ERR, no fold   6026   (this mutation)
#     phi keeps ERR, fold on   6001
#     phi drops ERR, fold on   5999   (shipped)
#
# The fold is 25 of the 27 and the phi's tighter set is the other 2 -- it buys
# nothing on its own, because nothing else in the emitter reads that bit. An
# earlier draft of this table read 20 for the second row; it was measuring a
# mutation that left the empty-set folds standing, which is the unsound case
# the `set != 0` guard exists to rule out. The thunk row beside this one sends
# the whole set to TOP, which takes the fold out AND puts `k_force_fast` back,
# so the two rows still say different things.
set -e
n=$(grep -cF 'if set != 0 && set & !infer::BOOL == 0 {' src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the cannot-fail fold changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|if set != 0 && set & !infer::BOOL == 0 {|if set != 0 \&\& set \& !0 == 0 {|' src/codegen.rs
grep -qF 'if set != 0 && set & !0 == 0 {' src/codegen.rs
