#!/bin/sh
# A region hands everything it allocated up to the enclosing frame.
#
# The descent into a map takes a mark before its `entries` are built, and the
# pop gives the region back when the result is the builder that was there
# before the mark. With the pop keeping the region instead, every map's
# entries stay in the arena until the next rewind outside the encode, and the
# mem fixture a_nested_map_gives_back_its_entries reads a higher arena peak.
set -e
f=src/runtime.c
grep -q 'KValue k_region_pop(KValue r)' src/runtime.c
line='KValue k_region_pop(KValue r) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the region pop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^KValue k_region_pop(KValue r) {$/KValue k_region_pop(KValue r) { if (k_beat_depth > 0 \&\& k_beat_depth <= K_BEAT_MAX) { k_beat_set_depth(k_beat_depth - 1); k_chunkreg_migrate(k_beat_depth); k_viewreg_migrate(k_beat_depth); k_permreg_migrate(k_beat_depth); } return r;/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
