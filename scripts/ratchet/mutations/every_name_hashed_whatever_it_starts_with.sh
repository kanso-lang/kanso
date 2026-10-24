#!/bin/sh
# queries_named passes over an at's name when no query starts with its first
# byte, before measuring and hashing it. This mutation admits every byte, so
# each name is measured and looked up as before. Same answers, and the
# browser compile row reads heavier.
set -e
t='    let mut leads = [false; 256];'
[ "$(grep -cxF "$t" src/codegen/prune.rs)" -eq 1 ] || {
  echo "the lead table moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let mut leads = \[false; 256\];$/    let mut leads = [true; 256];/' src/codegen/prune.rs
grep -qxF '    let mut leads = [true; 256];' src/codegen/prune.rs
