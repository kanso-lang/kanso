#!/bin/sh
# The interpreter's `push` stores a `list/tie` cell still being made, as a
# list literal does, so a field built with `list/to_list (list/map xs ref)`
# holds the cells. Forcing the item instead demands a node that does not exist
# yet, and a_graph_tied_from_a_map_of_edges dies on the interpreter.
set -e
f=src/eval.rs
line='                if at == 1 && name == "push" && matches!(&*cell.borrow(), ThunkState::Blackhole) {'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the interpreter's push keep moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                if at == 1 \&\& name == "push" \&\& matches!(/                if false \&\& at == 1 \&\& name == "push" \&\& matches!(/' "$f"
grep -qF 'if false && at == 1 && name == "push"' src/eval.rs
