#!/bin/sh
# The shape this vein exists to see, and the one no other gate can: a pass that
# owns the program's names instead of borrowing them, allocating a String per
# identifier OCCURRENCE. Rounds, visits and peak are identical either way,
# because the strings are transient — which is how a quarter off this pass's
# time landed with every gate in the tree reporting nothing.
#
# THE ROW WENT BLIND ON 2026-08-30 AND TWO SEPARATE MERGES DID IT.
# kanso#1188 made an identifier a `Name` rather than a `String`, so the old
# `out.insert(name.clone())` no longer type-checked and the mutation became a
# compile error — and a mutation that cannot build proves nothing, which this
# script's own comment had already warned about once. Repairing the type is not
# enough: kanso#1157 gave the walk an early return for any name that cannot be
# a getter's, and eleven thousand of lib/json's twelve thousand occurrences take
# it, so an owned insert behind that guard is reached almost never and moves the
# counter by zero. Measured: 25,394 either way with the type repaired alone.
#
# So the mutation restores the shape the vein was built to catch rather than
# one line of it — the guard goes and the names are owned. Measured on this
# corpus: compile_allocs 25,394 -> 31,138 and compile_alloc_bytes 3,950,766 ->
# 4,062,065.
#
# The `name.as_str()` substitution is still scoped to `mentions_in_expr` by
# address. `bound_in_pattern` grew a line spelled identically when it too
# stopped owning its names, and an unscoped sed rewrote both.
set -e
guard='if !short.unwrap_or(name).starts_with("Get_") {'
grep -qF "$guard" src/lib.rs || {
  echo "the getter-name guard moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak \
  -e 's/let mut mentioned: crate::hash::Set<&str> = crate::hash::Set::default();/let mut mentioned: crate::hash::Set<String> = crate::hash::Set::default();/' \
  -e "s/^fn mentions_in_stmt<'a>(stmt: &'a ast::Stmt, out: \&mut crate::hash::Set<\&'a str>) {/fn mentions_in_stmt(stmt: \&ast::Stmt, out: \&mut crate::hash::Set<String>) {/" \
  -e "s/^fn mentions_in_expr<'a>(e: &'a ast::Expr, out: \&mut crate::hash::Set<\&'a str>) {/fn mentions_in_expr(e: \&ast::Expr, out: \&mut crate::hash::Set<String>) {/" \
  -e '/if !short.unwrap_or(name).starts_with("Get_") {/,+2d' \
  -e '/^fn mentions_in_expr/,/^}/ s/out.insert(name.as_str());/out.insert(name.as_str().to_string());/' \
  -e 's/out.insert(short);/out.insert(short.to_string());/' \
  src/lib.rs
rm -f src/lib.rs.bak
grep -q 'out.insert(name.as_str().to_string());' src/lib.rs
grep -q 'out.insert(short.to_string());' src/lib.rs
grep -qF "$guard" src/lib.rs && {
  echo "the getter-name guard survived the mutation; it would be inert" >&2
  exit 1
}
exit 0
