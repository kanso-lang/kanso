#!/bin/sh
# Leave the placement of `Expr`'s tag to the compiler again. At 48 bytes it
# hides the tag in a spare range of one of the fields, every `match` over an
# expression decodes it, and checking the entry corpus costs 2.7 million more
# instructions. What the compiler emits is unchanged, so the witness is the
# instruction row.
set -e
target='#[cfg_attr(target_pointer_width = "64", repr(u8))]'
n=$(grep -cxF "$target" src/ast.rs)
[ "$n" -eq 1 ] || { echo "the explicit tag moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '$0 != t { print }' src/ast.rs > src/ast.rs.mut && mv src/ast.rs.mut src/ast.rs
