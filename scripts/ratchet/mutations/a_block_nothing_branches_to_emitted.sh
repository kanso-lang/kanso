#!/bin/sh
# A function body drops the blocks its entry cannot reach before it is
# written. This mutation keeps them all, so a dispatcher whose checks were
# proved away writes its failure path again. The output is the same; the
# witness is a_block_nothing_branches_to_is_not_emitted, which finds them.
set -e
line='        let reached = without_unreached_blocks(&self.out);'
grep -qF "$line" src/codegen.rs || {
  echo "the block pass moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let reached = without_unreached_blocks(&self.out);$/        let reached: Option<String> = None;/' src/codegen.rs
[ "$(grep -cF '        let reached: Option<String> = None;' src/codegen.rs)" -eq 1 ]
