#!/bin/sh
# `xs[i]!` answers a box (ruled 2026-09-16): the element, or the missing-index
# err bubbling through it, and `.>` is the word that opens it. The site walk
# learns that from one arm of `yields_box`, and this mutation turns that arm
# off, so the read reaches an operator, a field read and a builtin as if it
# were the element itself. The error corpus reads
# a_strict_index_where_a_value_is_expected refusing all four sites; under the
# mutation the program compiles and the golden holds the refusals.
set -e
grep -q '^            Expr::Index { strict: true, .. } => true,$' src/check.rs || {
  echo "the strict-index arm of yields_box changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^            Expr::Index { strict: true, .. } => true,$@            Expr::Index { strict: true, .. } => false,@' src/check.rs
grep -q '^            Expr::Index { strict: true, .. } => false,$' src/check.rs
