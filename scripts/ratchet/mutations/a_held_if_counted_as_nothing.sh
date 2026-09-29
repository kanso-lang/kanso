#!/bin/sh
# A held `if` answers no argument count, so calling it only grows a partial.
#
# The interpreter reads a builtin's count from the checker's table, which
# leaves `if` out on purpose. With the count restored to nothing, `g c 1 2`
# over `g = &if` prints `<fn>` where it should print the branch, and
# a_held_if_picks_its_branch goes red.
set -e
f=src/eval.rs
grep -q 'Callee::Builtin if &\*\*name == "if" => vec!\[3\],' src/eval.rs
line='                Callee::Builtin if &**name == "if" => vec![3],'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the held if's count moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                Callee::Builtin if &\*\*name == "if" => vec!\[3\],$/                Callee::Builtin if false \&\& \&**name == "if" => vec![3],/' "$f"
grep -qxF '                Callee::Builtin if false && &**name == "if" => vec![3],' "$f"
