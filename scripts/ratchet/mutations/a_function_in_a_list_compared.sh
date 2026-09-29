#!/bin/sh
# Ask only the operands whether they are functions, as the interpreter did
# until 2026-09-29. `[k] == [k]` then answers false where native refuses, and
# the runtime corpus goes red.
set -e
old='    if opaque_to_equality(a) || opaque_to_equality(b) || host_fn(a) || host_fn(b) {'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^    if opaque_to_equality(a) || opaque_to_equality(b) || host_fn(a) || host_fn(b) {$#    if false \&\& host_fn(a) {#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
