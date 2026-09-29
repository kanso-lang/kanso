#!/bin/sh
# The wasm engine copies a string built onto itself on every lap.
#
# `emit_join` sends a template at a `linear::string_builders` join to
# `rt_template_mut`, which appends to the builder the group owns. Sending every
# join to the ordinary template leaves each lap's string in the registry, so
# small answers stay right and a long loop fills the page.
set -e
f=src/wasm_backend.rs
grep -q 'fn emit_join' src/wasm_backend.rs
grep -qF '        let Some(local) = builder.filter(|_| self.builder_joins.contains(&site)) else {' "$f" || {
  echo "the join's door moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let Some(local) = builder.filter(|_| self.builder_joins.contains(&site)) else {$/        let Some(local) = builder.filter(|_| false \&\& self.builder_joins.contains(\&site)) else {/' "$f"
grep -qF 'builder.filter(|_| false && self.builder_joins' "$f"
