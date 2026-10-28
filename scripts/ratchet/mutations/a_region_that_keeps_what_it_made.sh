#!/bin/sh
# A region hands everything it allocated up to the enclosing frame.
#
# A descent into a map whose call builds the map's keys as an argument takes
# a mark before they are built, and the pop gives the region back when the
# result is the builder that was there before the mark. With the pop keeping
# the region instead, every map's keys stay in the arena until the next
# rewind, and the mem fixture a_descent_gives_back_the_keys_it_built reads a
# higher arena peak. std/json's encoder no longer builds anything there, so
# a_nested_map_gives_back_its_entries stopped seeing this on 2026-10-22.
set -e
f=src/runtime/regions.c
grep -q 'KValue k_region_pop(KValue r)' src/runtime/regions.c
line='KValue k_region_pop(KValue r) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the region pop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^KValue k_region_pop(KValue r) {$/KValue k_region_pop(KValue r) { if (k_beat_depth > 0 \&\& k_beat_depth <= K_BEAT_MAX) { k_beat_set_depth(k_beat_depth - 1); k_chunkreg_migrate(k_beat_depth); k_viewreg_migrate(k_beat_depth); k_permreg_migrate(k_beat_depth); } return r;/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
