#!/bin/sh
# `append acc "{n}"` is how the JSON encoder writes every number: the template
# rendered `n` into a string and the append copied that string into the
# accumulator, so 379,530 strings a runbench were built to be copied once
# and dropped. Since 2026-09-09 the emitter fuses the pair into
# k_b_append_rendered, which writes the digits into a stack buffer and copies
# them from there. This mutation switches the fusion off, so every site takes
# the render and the append separately again. The bytes out are the same and
# append_rendered goes to zero, so the work vein is the witness alongside it.
set -e
target='                    let fuse_render = f.set_of(&value) & (INT | FLOAT) != 0 && fusable;'
n=$(grep -cF "$target" src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the render fusion changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^                    let fuse_render = f.set_of(&value) \& (INT \| FLOAT) != 0 \&\& fusable;$|                    let fuse_render = false \&\& fusable;|' src/codegen.rs
grep -qF '                    let fuse_render = false && fusable;' src/codegen.rs
