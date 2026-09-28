#!/bin/sh
# Stop the chain rewrite at a guard, as it stopped before 2026-09-28.
#
# The count of a lazy binding's readers still descends into a `return ... if`,
# so the binding is removed and its reader past the return names nothing.
set -e
sed -i.bak 's|^    walk_children_mut(e, \&mut \|child\| substitute_ident(child, name, replacement));$|    if let ast::Expr::Guard { .. } = e { return; }\n&|' src/lib.rs
rm -f src/lib.rs.bak
grep -q 'if let ast::Expr::Guard { .. } = e { return; }' src/lib.rs
