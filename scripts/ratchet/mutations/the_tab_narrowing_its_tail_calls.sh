#!/bin/sh
# The tab's module keeps its tail calls as written, since the translator reads
# no calling convention and the narrowing only works around an arm64
# miscompile. This mutation narrows the tab's pieces as `finish` narrows a
# native module, and the browser compile row reads heavier.
set -e
t='        self.assembled().0'
[ "$(grep -cxF "$t" src/codegen/module.rs)" -eq 1 ] || {
  echo "the tab's pieces moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        self\.assembled()\.0$/        let (all, declared) = self.assembled();\n        narrowed_pieces(all, declared)/' src/codegen/module.rs
if grep -qxF "$t" src/codegen/module.rs; then exit 1; fi
grep -qxF '        narrowed_pieces(all, declared)' src/codegen/module.rs
