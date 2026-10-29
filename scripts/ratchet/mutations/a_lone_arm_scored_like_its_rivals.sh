#!/bin/sh
# Send a group of one bare-name arm through the candidate loop again, scored
# and compared like any other group. It still wins, and the witness is the
# interpreted run's instruction count.
set -e
target='                    Some(*decl)'
n=$(grep -cxF "$target" src/eval/calls.rs)
[ "$n" -eq 1 ] || { echo "the lone arm's answer moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '$0 == t { print "                    { let _ = decl; None::<&FnDecl> }"; next } { print }' src/eval/calls.rs > src/eval/calls.rs.mut && mv src/eval/calls.rs.mut src/eval/calls.rs
