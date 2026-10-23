#!/bin/sh
# The tab's compiler exports its own memcmp and bcmp, eight bytes a pass
# (src/compare.rs, 2026-10-23), in place of compiler-builtins' loop of one
# byte a pass. This mutation renames both exports, so the wasm build links the
# byte loop again and the browser compile row is the witness:
# browser_compile_instructions 293,067,500 -> 274,134,296 when the word
# compare landed.
set -e
for f in memcmp bcmp; do
  target="pub unsafe extern \"C\" fn $f("
  n=$(grep -cF "$target" src/main.rs)
  [ "$n" -eq 1 ] || { echo "the $f export changed shape ($n); rewrite this" >&2; exit 1; }
  sed -i "s/^pub unsafe extern \"C\" fn $f(/pub unsafe extern \"C\" fn unused_$f(/" src/main.rs
  grep -qF "pub unsafe extern \"C\" fn unused_$f(" src/main.rs
done
