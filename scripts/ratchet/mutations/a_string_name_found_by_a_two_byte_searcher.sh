#!/bin/sh
# named_strings finds `@sN` by memchr on `@` and a check of the `s`
# (2026-10-23). It used to call find("@s"), whose two-way searcher skips
# almost nothing with a needle two bytes long. This mutation searches for "@s"
# again, which with the check after it is the old code, and the browser
# compile row is the witness: 261,513,550 on this tree, 263,889,085 under it.
set -e
target="        while let Some(at_sign) = text[at..].find('@') {"
n=$(grep -cxF "$target" src/codegen/ir_scan.rs)
[ "$n" -eq 1 ] || { echo "named_strings changed shape ($n); rewrite this" >&2; exit 1; }
sed -i "s/^        while let Some(at_sign) = text\[at..\].find('@') {\$/        while let Some(at_sign) = text[at..].find(\"@s\") {/" src/codegen/ir_scan.rs
grep -qxF '        while let Some(at_sign) = text[at..].find("@s") {' src/codegen/ir_scan.rs
