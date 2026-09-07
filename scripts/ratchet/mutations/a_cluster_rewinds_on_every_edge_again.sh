#!/bin/sh
# A beat cluster of several members rewinds on the back edges of a walk over
# its internal tail graph, once a trip round any cycle. Before 2026-09-07 it
# rewound on every internal edge: a four-member cycle paid four rewinds a trip
# where one frees everything the trip allocated, and runbench's escape cycle
# spent 1.06% of the program on the other three. This puts every edge back.
# The back edges stay in the set, so nothing is dead and the build is a build;
# what moves is beat_iters, which every .mem golden in the corpus pins.
set -e
grep -qF "        for e in back_edges(edges) {" src/beat.rs || {
  echo "the rewind set changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|        for e in back_edges(edges) {|        for e in back_edges(edges).into_iter().chain(edges.iter().cloned()) {|' \
  src/beat.rs
grep -qF "chain(edges.iter().cloned())" src/beat.rs
