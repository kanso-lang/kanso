#!/bin/sh
# Take a map literal's values as alone whatever they are.
#
# One inner map at two keys of the seed is then written in place through
# one key, and the other key and every other holder see the write.
set -e
sed -i.bak 's/Expr::MapLit(entries, _) => entries.iter().all(|(_, v)| self.unique_in(v, ctx, None)),/Expr::MapLit(..) => true,/' src/linear.rs
rm -f src/linear.rs.bak
test "$(grep -c 'Expr::MapLit(..) => true,' src/linear.rs)" = 1
