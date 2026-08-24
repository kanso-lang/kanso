#!/bin/sh
# The shape this vein exists to see, and the one no other gate can: a pass that
# owns the program's names instead of borrowing them, allocating a String per
# identifier OCCURRENCE. Rounds, visits and peak are identical either way,
# because the strings are transient — which is how a quarter off this pass's
# time landed with every gate in the tree reporting nothing.
#
# The `name.as_str()` substitution is scoped to `mentions_in_expr` by address.
# `bound_in_pattern` grew a line spelled identically when it too stopped owning
# its names, and an unscoped sed rewrote both — which turned this mutation into
# a compile error rather than a moved counter, and a mutation that cannot build
# proves nothing about the gate.
set -e
sed -i.bak \
  -e 's/let mut mentioned: crate::hash::Set<&str> = crate::hash::Set::default();/let mut mentioned: crate::hash::Set<String> = crate::hash::Set::default();/' \
  -e "s/^fn mentions_in_stmt<'a>(stmt: &'a ast::Stmt, out: \&mut crate::hash::Set<\&'a str>) {/fn mentions_in_stmt(stmt: \&ast::Stmt, out: \&mut crate::hash::Set<String>) {/" \
  -e "s/^fn mentions_in_expr<'a>(e: &'a ast::Expr, out: \&mut crate::hash::Set<\&'a str>) {/fn mentions_in_expr(e: \&ast::Expr, out: \&mut crate::hash::Set<String>) {/" \
  -e '/^fn mentions_in_expr/,/^}/ s/out.insert(name.as_str());/out.insert(name.clone());/' \
  -e 's/out.insert(short);/out.insert(short.to_string());/' \
  src/lib.rs
rm -f src/lib.rs.bak
grep -q 'out.insert(name.clone());' src/lib.rs
