#!/bin/sh
# A body built for the dev tier or the tab writes no `llvm.assume`, since
# neither is optimized: -O0 drops the call, and the tab's translator skips it
# but still runs the compare that feeds it. This mutation writes them for
# every body, and the browser compile row reads heavier.
set -e
t='        !self.words'
[ "$(grep -cxF "$t" src/codegen.rs)" -eq 1 ] || {
  echo "the assume switch moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s/^        !self\\.words\$/        true/" src/codegen.rs
if grep -qxF "$t" src/codegen.rs; then exit 1; fi
