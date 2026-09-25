#!/bin/sh
# The dispatcher finds a tail call's group by hashing its name again.
#
# `eval_tail` resolves the callee through `callee_of_ref`, which already holds
# the group, and the tail flow carries it to the dispatcher. Looked up by name
# instead, every tail call pays a hash of the name and a compare of its bytes,
# and the interpreted run's instruction row moves.
set -e
f=src/eval.rs
grep -q 'Flow::Tail(next, group' src/eval.rs
line='                            overloads = group;'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the tail's group moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                            overloads = group;$/                            overloads = { drop(group); self.fns.get(\&*next).expect("tails name real groups").clone() };/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
