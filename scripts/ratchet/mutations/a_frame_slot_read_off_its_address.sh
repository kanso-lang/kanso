#!/bin/sh
# The interpreter's frame table takes its slot from where a declaration sits
# inside its allocator slice, which does not move when the heap does. This
# mutation takes the slot from the whole address again, so which declarations
# share a slot depends on where valgrind placed the heap, and the interpreted
# run's row reads one instruction apart with the mappings started at 48 MiB.
set -e
line='        let slot = recent_slot(in_slice(key), RECENT_FRAMES);'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the frame slot moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let slot = recent_slot(in_slice(key), RECENT_FRAMES);$/        let slot = recent_slot(key as u64, RECENT_FRAMES);/' src/eval.rs
grep -qxF '        let slot = recent_slot(key as u64, RECENT_FRAMES);' src/eval.rs
