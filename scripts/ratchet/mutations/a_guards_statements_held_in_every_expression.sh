#!/bin/sh
# Hold a guard's statements inline again instead of behind a box. `Guard` is
# then the largest variant of `Expr`, so every expression in the program grows
# by eight bytes. Nothing a program prints changes, so the witness is the front
# end's peak.
set -e
decl='pub struct Rest(Box<Vec<Stmt>>);'
make='        Rest(Box::new(stmts))'
for t in "$decl" "$make"; do
  n=$(grep -cxF "$t" src/ast.rs)
  [ "$n" -eq 1 ] || { echo "the guard's box moved or multiplied ($n): $t; rewrite this" >&2; exit 1; }
done
awk -v d="$decl" -v m="$make" '
  $0 == d { print "pub struct Rest(Vec<Stmt>);"; next }
  $0 == m { print "        Rest(stmts)"; next }
  { print }
' src/ast.rs > src/ast.rs.mut && mv src/ast.rs.mut src/ast.rs
