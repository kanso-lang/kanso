#!/bin/sh
# A loop that only grows the list it was handed keeps its bracket and takes no
# rewind. This puts the rewind back on every such loop: the run program's escape
# phase tests the arena 1,548,800 times again, for nothing, and the work vein
# reads it.
set -e
t='            || !grows_only_itself(program, mut_sites, &regions, &allocating, from)'
n=$(grep -cF "$t" src/beat.rs)
[ "$n" -eq 1 ] || { echo "the rewind's exemption changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/            || !grows_only_itself(program, mut_sites, &regions, &allocating, from)/            || true/' src/beat.rs
grep -qF '            || true' src/beat.rs
