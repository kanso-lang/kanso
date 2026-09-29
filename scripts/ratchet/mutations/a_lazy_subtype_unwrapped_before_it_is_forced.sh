#!/bin/sh
# The native engine unwraps a builtin's subtype arguments before it forces
# them.
#
# A lazy binding holding a subtype's value is a thunk, and a thunk is not a
# subtype, so an unwrap that runs first passes it through and the builtin
# meets the subtype after its own force.
set -e
f=src/codegen.rs
grep -q "fn maybe_force" src/codegen.rs
grep -qF '                let forced = self.maybe_force(f, e.clone());' "$f" || {
  echo "the force ahead of the subtype unwrap moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                let forced = self.maybe_force(f, e.clone());$/                let forced = e.clone();/' "$f"
grep -qF '                let forced = e.clone();' "$f"
