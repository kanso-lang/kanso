#!/bin/sh
# The wasm engine copies a list seed its caller reads again on every lap.
#
# `linear::handed_over_pushes` grants a parameter whose callers do not all hand
# their value over, and names the calls that need a handle of their own.
# Asking `in_place_pushes` instead refuses the parameter, so every lap copies
# the list and the registry keeps each copy until the page is full.
set -e
f=src/wasm_backend.rs
grep -q "handed_over_pushes" src/wasm_backend.rs
grep -qF '    let (in_place, rehandles) = crate::linear::handed_over_pushes(program);' "$f" || {
  echo "the wasm backend's in-place question moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let (in_place, rehandles) = crate::linear::handed_over_pushes(program);$/    let (in_place, rehandles) = (crate::linear::in_place_pushes(program), crate::linear::Edges::default());/' "$f"
grep -qF 'crate::linear::Edges::default());' "$f"
