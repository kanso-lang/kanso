#!/bin/sh
# Native `push` stores a `list/tie` cell still being made, as a list literal
# does, so a field built with `list/to_list (list/map xs ref)` holds the
# cells. Forcing the item instead demands a node that does not exist yet, and
# a_graph_tied_from_a_map_of_edges dies in the built binary.
set -e
f=src/codegen.rs
line='                .map(|(at, e)| match name == "push" && at == 1 && self.defers_self_reference {'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "native push's keep moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                .map(|(at, e)| match name == "push" \&\& at == 1 \&\& self.defers_self_reference {/                .map(|(at, e)| match false \&\& name == "push" \&\& at == 1 \&\& self.defers_self_reference {/' "$f"
grep -qF 'match false && name == "push"' src/codegen.rs
