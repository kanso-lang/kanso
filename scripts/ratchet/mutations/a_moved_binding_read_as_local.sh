#!/bin/sh
# A binding moved into the park is closed, so a call after the sleep that
# reads it is closed too and is lifted. This mutation reads the moved binding
# as a local: the call stays where it is, and the mem golden's
# `ahead_forced=2` goes to 1.
set -e
line='        Expr::Ident(n, _, _) => n.as_str().starts_with(MOVED),'
[ "$(grep -cxF "$line" src/ahead.rs)" -eq 1 ] || {
  echo "the moved binding's arm in closed moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        Expr::Ident(n, _, _) => n.as_str().starts_with(MOVED),$/        Expr::Ident(n, _, _) => n.as_str().starts_with("never~"),/' src/ahead.rs
if grep -qxF "$line" src/ahead.rs; then exit 1; fi
