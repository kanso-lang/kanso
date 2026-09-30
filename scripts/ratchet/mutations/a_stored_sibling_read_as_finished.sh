#!/bin/sh
# A sibling argument that stores the container is read as finished with it.
#
# A builtin forces its arguments before it writes, so a read of the container
# in a sibling argument -- `put m k (m[k] + 1)` -- is over by the write, and
# the write may still go in place. A sibling that puts the container in a
# list, a record or a closure keeps holding it. With every sibling mention
# discounted again, `push a [a 7]` writes into the list it has just stored,
# and the micro fixture prints the inner copy empty on the interpreter and a
# cycle on native.
set -e
f=src/linear.rs
grep -q "fn holds(var: &str, e: &Expr) -> bool" src/linear.rs
line='        Expr::Ident(n, _, _) => n == var,'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the holds test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        Expr::Ident(n, _, _) => n == var,$/        Expr::Ident(..) => false,/' "$f"
grep -qxF '        Expr::Ident(..) => false,' "$f"
