#!/bin/sh
# Name the NUMA option by the wrong number.
#
# `mi_option_use_numa_nodes` is read out of mimalloc's vendored header by
# position, like the two options beside it, so a crate bump can move it in
# silence. This is what that looks like: the call is still there and sets
# whatever sits one further down the enum.
set -e
sed -i.bak 's/^const USE_NUMA_NODES: i32 = 16;$/const USE_NUMA_NODES: i32 = 17;/' src/main.rs
rm -f src/main.rs.bak
grep -q '^const USE_NUMA_NODES: i32 = 17;$' src/main.rs
