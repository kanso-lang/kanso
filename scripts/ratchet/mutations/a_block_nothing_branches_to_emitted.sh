#!/bin/sh
# A dispatcher stops writing arms once one leaves no branch to its `fail`
# block, since nothing can reach the arms after it or the failure path. This
# mutation always writes them. The output is the same; the witness is
# a_block_nothing_branches_to_is_not_emitted, which finds the blocks.
set -e
line='            if !branches_to(&f.out[arm_at..], &fail) {'
grep -qF "$line" src/codegen.rs || {
  echo "the dispatcher's early close moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if !branches_to(&f.out\[arm_at..\], &fail) {$/            if arm_at == usize::MAX \&\& !branches_to(\&f.out[arm_at..], \&fail) {/' src/codegen.rs
[ "$(grep -cF '            if arm_at == usize::MAX && !branches_to(&f.out[arm_at..], &fail) {' src/codegen.rs)" -eq 1 ]
