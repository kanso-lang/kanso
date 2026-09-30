#!/bin/sh
# An arm whose answer can be any value is counted as raising, because its
# description bit is inference not knowing. This counts every such arm as a
# box again, and a raise handed on through a field of an entry reaches a group
# with no arm for it without a word.
set -e
t='        let boxed = r & crate::infer::DESC != 0 && r & any != any;'
n=$(grep -cF "$t" src/check.rs)
[ "$n" -eq 1 ] || { echo "the box test changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        let boxed = r & crate::infer::DESC != 0 \&\& r & any != any;/        let boxed = r \& crate::infer::DESC != 0 || any == 0;/' src/check.rs
grep -qF '        let boxed = r & crate::infer::DESC != 0 || any == 0;' src/check.rs
