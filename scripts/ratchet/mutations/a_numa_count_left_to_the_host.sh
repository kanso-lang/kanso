#!/bin/sh
# Leave mimalloc to count the host's NUMA nodes.
#
# The compiler tells mimalloc there is one node. Without that, mimalloc counts
# them itself the first time a thread allocates: it formats a path under
# /sys/devices/system/node and asks `access` whether it exists, once per node
# and once more. The interpreter's thread is the first to ask, so the probe
# lands inside the frame `interp_instructions` counts, and what it costs is a
# property of the machine rather than of the interpreter.
set -e
sed -i.bak '/mi_option_set(USE_NUMA_NODES, 1)/d' src/main.rs
rm -f src/main.rs.bak
! grep -q 'mi_option_set(USE_NUMA_NODES' src/main.rs
