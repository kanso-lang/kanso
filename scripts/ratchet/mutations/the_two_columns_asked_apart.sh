#!/bin/sh
# Ask for a map's two columns one at a time again.
#
# Where a call's arguments are `keys m` and then `values m`, the emitter makes
# one runtime call that builds both lists from one allocation. This takes the
# match away, so each column is its own call and its own allocation, and every
# map the encoder writes allocates twice.
set -e
sed -i.bak 's/column(a, "keys").is_some())?;/column(a, "keys").is_some() \&\& false)?;/' src/codegen.rs
rm -f src/codegen.rs.bak
grep -q 'column(a, "keys").is_some() && false)?;' src/codegen.rs
