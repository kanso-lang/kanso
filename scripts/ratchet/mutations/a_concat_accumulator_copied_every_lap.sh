#!/bin/sh
# A concat onto a list nobody else holds extends it where it stands. This
# mutation stops marking those sites, so every lap of a concat fold builds a
# fresh list and keeps the old one, and the mem golden for it goes from two
# arena blocks to over a thousand.
set -e
line='        if matches!(head.as_ref(), Expr::Ident(n, _, _) if matches!(n.as_str(), "concat" | "list/concat" | "builtin_concat"))'
[ "$(grep -cxF "$line" src/linear.rs)" -eq 1 ] || {
  echo "the concat site test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if matches!(head.as_ref(), Expr::Ident(n, _, _) if matches!(n.as_str(), "concat" | "list\/concat" | "builtin_concat"))$/        if matches!(head.as_ref(), Expr::Ident(n, _, _) if n.as_str() == "concat never")/' src/linear.rs
if grep -qxF "$line" src/linear.rs; then exit 1; fi
