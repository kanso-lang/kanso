#!/bin/sh
# A map whose pairs are already ascending, none repeated, is its own sorted
# view: k_map_sort_build points the view at the pairs instead of copying them
# into a malloc'd buffer. This mutation takes the alias away, so every read
# copies again. The answers are the same; the witness is the fixture
# a_map_whose_keys_arrived_in_order_is_its_own_view, which reads view_allocs=0
# and held_peak_bytes=0 and turns to one view of 32,016 bytes.
set -e
target='        if (a >= n) {'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the ordered-pairs alias changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        if (a >= n) {$|        if (0 \&\& a >= n) {|' src/runtime.c
[ "$(grep -cF '        if (0 && a >= n) {' src/runtime.c)" -eq 1 ]
