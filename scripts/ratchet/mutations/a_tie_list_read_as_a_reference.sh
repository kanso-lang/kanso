#!/bin/sh
# A list literal written as a constructor's field stops passing the field flag
# to its elements, so the references in it are refused as reads again.
#
# Ruled 2026-10-10: a field may hold a list of `tie` references. The micro
# corpus's `a_graph_holds_its_roads_in_a_list` keeps each town's roads in one,
# and stops compiling.
set -e
grep -qxF '            Expr::List(items, _) if in_field => {' src/check.rs || {
  echo "the tie check's list arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            Expr::List(items, _) if in_field => {$/            Expr::List(items, _) if in_field \&\& false => {/' src/check.rs
! grep -qxF '            Expr::List(items, _) if in_field => {' src/check.rs
