#!/bin/sh
# queries_named finds each `@` with kanso::compare::find, eight bytes a pass
# (2026-10-23), where str::find paid a searcher's setup and a call into
# memchr for every one. This mutation puts str::find back, and the browser
# compile row is the witness.
set -e
f=src/codegen/prune.rs
target="    while let Some(next) = crate::compare::find(&bytes[at..], b'@') {"
n=$(grep -cxF "$target" $f)
[ "$n" -eq 1 ] || { echo "queries_named changed shape ($n); rewrite this" >&2; exit 1; }
sed -i "s/^    while let Some(next) = crate::compare::find(&bytes\[at..\], b'@') {\$/    while let Some(next) = text[at..].find('@') {/" $f
grep -qxF "    while let Some(next) = text[at..].find('@') {" $f
