#!/bin/sh
# The counting allocator keeps compile_alloc_bytes and compile_allocs for the
# command line alone, and the wasm build skips both (2026-10-23). This
# mutation keeps them in the tab again, and the browser compile row is the
# witness.
set -e
n=$(grep -c '^        ALLOC_\(BYTES\|CALLS\).fetch_add' src/main.rs)
[ "$n" -eq 2 ] || { echo "the tally changed shape ($n); rewrite this" >&2; exit 1; }
sed -i '/^        #\[cfg(not(target_arch = "wasm32"))\]$/{N;/\n        ALLOC_\(BYTES\|CALLS\).fetch_add/s/^[^\n]*\n//}' src/main.rs
[ "$(grep -B1 '^        ALLOC_\(BYTES\|CALLS\).fetch_add' src/main.rs | grep -c 'cfg(not(target_arch')" -eq 0 ]
