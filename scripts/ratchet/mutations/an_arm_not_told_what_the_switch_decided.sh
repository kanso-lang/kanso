#!/bin/sh
# A group that dispatches on one parameter's tag compiles to a switch, and
# the arm the switch lands in knows what the value is. Until 2026-09-09 the
# arm's body did not: the discriminator kept the whole group's set inside
# every arm, so `n:int`'s body forced `n` again, asked whether a user
# to_string arm could claim it, and took the generic door on every builtin
# it handed `n` to. Now the set inside an arm is the case's tags. This
# mutation hands every arm the whole set again. The bytes out are the same
# and no counter moves on its own, so the work vein is the witness.
set -e
target='            let narrowed = self.arm_tags_set(&decl.params[disc], by_tag);'
n=$(grep -cF "$target" src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the arm narrowing changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^            let narrowed = self.arm_tags_set(&decl.params\[disc\], by_tag);$|            let narrowed = self.arm_tags_set(\&decl.params[disc], by_tag).filter(\|_\| false);|' src/codegen.rs
grep -qF 'let narrowed = self.arm_tags_set(&decl.params[disc], by_tag).filter(|_| false);' src/codegen.rs
