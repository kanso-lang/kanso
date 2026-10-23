#!/bin/sh
# lines_holding_at compares the needle from where its anchor byte sits in it.
# This mutation compares from the anchor itself, and the spec beside it finds
# the lines the word search finds no longer match.
set -e
f=src/codegen/narrow.rs
target='        let s = hit - at;'
n=$(grep -cxF "$target" $f)
[ "$n" -eq 1 ] || { echo "lines_holding_at changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        let s = hit - at;$/        let s = hit;/' $f
grep -qxF '        let s = hit;' $f
