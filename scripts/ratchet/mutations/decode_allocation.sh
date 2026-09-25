#!/bin/sh
# The uniqueness fixpoint decides nothing is linear, so every accumulator is
# copied instead of written in place. Measured on the decode benchmark: 6.27M
# allocations become 12.54M.
#
# Editing the golden's number instead would exercise the same diff and prove
# less. A counter wired to nothing — reading a constant, or a length that has
# no relation to the work — passes an edited golden and stays green under this.
set -e
A='    fn param_is_linear(&self, name: &str, arity: usize, i: usize) -> bool {' \
awk '
  !hit && $0 == ENVIRON["A"] { print; print "        return false;"; hit = 1; next }
  { print }
  END { if (!hit) exit 3 }
' src/linear.rs > src/linear.rs.mut || {
  rm -f src/linear.rs.mut
  echo "param_is_linear moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/linear.rs.mut src/linear.rs
grep -A1 'fn param_is_linear' src/linear.rs | grep -q 'return false'
