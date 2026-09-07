#!/bin/sh
# A zero-argument definition is a constant and the interpreter computes it
# once, behind a knot cell. Until 2026-09-07 native froze only a constant
# whose body was a literal, and every other constant was recomputed at every
# mention: sha256's sixty-four round constants were concatenated from eleven
# literal lists 16,000 times a run on runbench. This mutation puts the old
# rule back -- only a knotted constant freezes -- so every constant built by
# a call is rebuilt per use again. The bytes out are the same and the work
# vein is the witness: runbench 2,602,519,000 -> 2,487,359,798 on the
# container when every constant froze, and 2,615,934,276 with this applied.
set -e
grep -q '^    fn is_constant_body(&self, _decl: &FnDecl) -> bool {$' src/codegen.rs || {
  echo "is_constant_body changed shape; rewrite this" >&2
  exit 1
}
sed -i '/fn is_constant_body(&self, _decl: &FnDecl) -> bool {/,/^    }$/ s/^        true$/        self.knotted.contains(\&_decl.name)/' src/codegen.rs
grep -q '^        self.knotted.contains(&_decl.name)$' src/codegen.rs
