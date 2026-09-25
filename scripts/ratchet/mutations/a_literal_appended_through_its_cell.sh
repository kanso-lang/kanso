#!/bin/sh
# The emitter writes a short literal appended in place as one word. This
# mutation never takes that road, so every `true`, `false` and `null` the json
# encoder writes goes back through the literal's cell and the string arm: the
# run program's work rises.
set -e
grep -q '^                if let Some(text) = text.filter(|t| mutate && (1..=8).contains(&t.len())) {$' src/codegen.rs || {
  echo "the literal word's emitter case changed shape; rewrite this" >&2
  exit 1
}
sed -i 's/^                if let Some(text) = text.filter(|t| mutate && (1..=8).contains(&t.len())) {$/                if let Some(text) = text.filter(|t| mutate \&\& t.len() > 8 \&\& t.len() < 8) {/' src/codegen.rs
grep -q 't.len() > 8 && t.len() < 8' src/codegen.rs
