#!/bin/sh
# Ask for a map's two columns one at a time again.
#
# Where a call's arguments are `keys m` and then `values m`, or two bindings
# in a row name them, the emitter makes one runtime call that builds both
# lists from one allocation. This makes the builtins look declared, so each
# column is its own call and its own allocation, and every such call
# allocates twice.
set -e
sed -i.bak 's/f.lookup(map).is_some() \&\& !declared("keys") \&\& !declared("values")/false \&\& f.lookup(map).is_some()/' src/codegen/calls.rs
rm -f src/codegen/calls.rs.bak
grep -q 'false && f.lookup(map).is_some()' src/codegen/calls.rs
