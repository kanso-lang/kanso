#!/bin/sh
# is_a_stack_slot finds the end of a line's first word with a byte loop
# (2026-10-23), where text.find(' ') paid a searcher's setup, a call into
# memchr and a one-byte compare on every line the emitter writes. This
# mutation puts find back, and the browser compile row is the witness:
# 258,016,805 on this tree, 259,885,584 under it.
set -e
target="    match text.bytes().position(|b| b == b' ') {"
n=$(grep -cxF "$target" src/codegen.rs)
[ "$n" -eq 1 ] || { echo "is_a_stack_slot changed shape ($n); rewrite this" >&2; exit 1; }
sed -i "s/^    match text.bytes().position(|b| b == b' ') {\$/    match text.find(' ') {/" src/codegen.rs
grep -qxF "    match text.find(' ') {" src/codegen.rs
