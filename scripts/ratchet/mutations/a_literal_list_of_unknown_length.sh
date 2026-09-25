#!/bin/sh
# A literal index into a literal list is in range by arithmetic: the list
# spells its length out (ruled 2026-09-16, the second cost lever). This
# mutation inverts the upper-bound test, so `[4 5 6][1]` reads as a possible
# none and the group it feeds refuses it. The micro corpus reads
# a_guard_proves_the_read printing n4 for that read.
set -e
A='                    int_of(index).is_some_and(|i| i >= 1 && i <= items.len() as i64)'
grep -qF "$A" src/infer.rs || {
  echo "the literal-list arm of the prover changed shape; this needs rewriting" >&2
  exit 1
}
A="$A" awk '
  $0 == ENVIRON["A"] { print "                    int_of(index).is_some_and(|i| i >= 1 && i > items.len() as i64)"; next }
  { print }
' src/infer.rs > src/infer.rs.mut
mv src/infer.rs.mut src/infer.rs
grep -qF 'i >= 1 && i > items.len() as i64' src/infer.rs
