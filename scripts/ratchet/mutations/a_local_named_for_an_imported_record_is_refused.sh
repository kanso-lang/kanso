#!/bin/sh
# A refusal of a short name is shadowable, because the name can be a local
# binding. This keeps the refusal whatever the body binds, so a lambda bound
# as `grown` is refused as a construction of std/list's `grown`.
set -e
t='                if diags.len() > refused {'
n=$(grep -cF "$t" src/check.rs)
[ "$n" -eq 1 ] || { echo "the shadowable push changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^                if diags.len() > refused {$/                if false {/' src/check.rs
grep -qF '                if false {' src/check.rs
