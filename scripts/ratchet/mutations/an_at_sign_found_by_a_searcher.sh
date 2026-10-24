#!/bin/sh
# queries_named finds each `@` with kanso::compare::find, eight bytes a pass
# (2026-10-23), where str::find paid a searcher's setup and a call into
# memchr for every one. This mutation puts str::find back, and the browser
# compile row is the witness.
set -e
target="    while let Some(next) = crate::compare::find(&bytes[at..], b'@') {"
n=$(grep -cxF "$target" src/codegen/prune.rs)
[ "$n" -eq 1 ] || { echo "queries_named changed shape ($n); rewrite this" >&2; exit 1; }
sed -i "s/^    while let Some(next) = crate::compare::find(&bytes\[at..\], b'@') {\$/    while let Some(next) = text[at..].find('@') {/" src/codegen/prune.rs
grep -qxF "    while let Some(next) = text[at..].find('@') {" src/codegen/prune.rs
