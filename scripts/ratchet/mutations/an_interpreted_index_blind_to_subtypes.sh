#!/bin/sh
# Stop the interpreter's index reading through a subtype, as it did not until
# 2026-09-28. It then refuses a subtype of int as a position and as a map
# key, and the micro fixture a_subtype_indexes_as_its_base stops where a
# compiled build prints.
set -e
old='        (Value::Sub { .. }, _) | (_, Value::Sub { .. }) => {'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^        (Value::Sub { .. }, _) | (_, Value::Sub { .. }) => {$#        (Value::Sub { .. }, _) | (_, Value::Sub { .. }) if false => {#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
