#!/bin/sh
# A map handed back to its loop unchanged threads, so no lap asks whether its
# slots survive. This carries it again: every lap walks every key and value,
# and the mem fixture's survive_slots reads the walk.
set -e
t='const THREADED: Set = CROSSES | NONE | STR | BYTES | FN | REC | DESC | LIST | MAP;'
n=$(grep -cF "$t" src/beat.rs)
[ "$n" -eq 1 ] || { echo "the threaded set changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^const THREADED: Set = CROSSES | NONE | STR | BYTES | FN | REC | DESC | LIST | MAP;/const THREADED: Set = CROSSES | NONE | STR | BYTES | FN | REC | DESC | LIST;/' src/beat.rs
grep -qF 'const THREADED: Set = CROSSES | NONE | STR | BYTES | FN | REC | DESC | LIST;' src/beat.rs
