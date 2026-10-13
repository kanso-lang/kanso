#!/bin/sh
# A field may hold `list/to_list (list/map xs ref)` (2026-10-11), and the
# check admits it in the form the program wrote and in the form fusion leaves
# in a module's code. This takes both admissions away, so
# a_graph_tied_from_a_map_of_edges is refused and the micro corpus goes red.
set -e
f=src/check/ties.rs
grep -qF 'if let Some((xs, f)) = mapped_over(inner) {' src/check/ties.rs
grep -qF 'if let Some((xs, item)) = pushed_onto(e) {' src/check/ties.rs
sed -i 's/if let Some((xs, f)) = mapped_over(inner) {/if let Some((xs, f)) = mapped_over(inner).filter(|_| false) {/' "$f"
sed -i 's/if let Some((xs, item)) = pushed_onto(e) {/if let Some((xs, item)) = pushed_onto(e).filter(|_| false) {/' "$f"
grep -qF 'pushed_onto(e).filter(|_| false)' src/check/ties.rs
