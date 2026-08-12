#!/bin/sh
# The uniqueness fixpoint decides nothing is linear, so every accumulator is
# copied instead of written in place. Measured on the decode benchmark: 6.27M
# allocations become 12.54M.
#
# Editing the golden's number instead would exercise the same diff and prove
# less. A counter wired to nothing — reading a constant, or a length that has
# no relation to the work — passes an edited golden and stays green under this.
set -e
python3 - <<'PY'
path = "src/linear.rs"
head = "    fn param_is_linear(&self, name: &str, arity: usize, i: usize) -> bool {\n"
source = open(path).read()
assert head in source, "param_is_linear moved; this mutation needs rewriting"
open(path, "w").write(source.replace(head, head + "        return false;\n", 1))
PY
grep -A1 'fn param_is_linear' src/linear.rs | grep -q 'return false'
