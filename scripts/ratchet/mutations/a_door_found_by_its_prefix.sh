#!/bin/sh
# A door is a runtime function called on the preserve_none convention, and
# the emitter writes the convention in front of every declare of and call to
# one. It matches the whole name up to its `(`. This mutation matches a prefix,
# so `k_b_utf8` also claims the `k_b_utf8_...` functions, which are defined on
# the ordinary convention: their arguments arrive in the wrong registers. The
# run program prints a wrong tally without failing, and the micro corpus
# disagrees with the interpreter.
set -e
grep -q "^            .any(|d| name.strip_prefix(d).is_some_and(|after| after.starts_with('(')));$" src/codegen.rs || {
  echo "the door match changed shape; rewrite this" >&2
  exit 1
}
sed -i "s/^            .any(|d| name.strip_prefix(d).is_some_and(|after| after.starts_with('(')));$/            .any(|d| name.starts_with(d));/" src/codegen.rs
grep -q '^            .any(|d| name.starts_with(d));$' src/codegen.rs
