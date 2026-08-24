#!/bin/sh
# The shape this vein exists to see, and the one no other gate can: a pass that
# owns the program's names instead of borrowing them, allocating a String per
# identifier OCCURRENCE. Rounds, visits and peak are identical either way,
# because the strings are transient — which is how a quarter off this pass's
# time landed with every gate in the tree reporting nothing.
set -e
sed -i.bak \
  -e 's/let mut mentioned: std::collections::HashSet<&str> = std::collections::HashSet::new();/let mut mentioned: std::collections::HashSet<String> = std::collections::HashSet::new();/' \
  -e 's/^fn mentions_in_stmt<.a>(stmt: &.a ast::Stmt, out: \&mut std::collections::HashSet<\&.a str>) {/fn mentions_in_stmt(stmt: \&ast::Stmt, out: \&mut std::collections::HashSet<String>) {/' \
  -e 's/^fn mentions_in_expr<.a>(e: &.a ast::Expr, out: \&mut std::collections::HashSet<\&.a str>) {/fn mentions_in_expr(e: \&ast::Expr, out: \&mut std::collections::HashSet<String>) {/' \
  -e 's/out.insert(name.as_str());/out.insert(name.clone());/' \
  -e 's/out.insert(short);/out.insert(short.to_string());/' \
  src/lib.rs
rm -f src/lib.rs.bak
grep -q 'out.insert(name.clone());' src/lib.rs
