# The read vein's peak rises and nothing goes the right way, with no entry
# added to bench/welfare_floor.json's history to attribute the fall.
#
# The same pure regression its siblings introduce, in the vein that was
# INVISIBLE. readbench joined the objective on 2026-09-04 with
# `read_arena_blocks` and `read_peak_bytes` as welfare terms, and
# `bench/cost_golden_read.txt` was never added to the trend gate's list — so
# the gate that exists to watch the objective's inputs printed nothing whatever
# for two of them. This row is what keeps the file listed.
#
# `arena_peak_bytes` rather than `allocs`, because welfare's `read_peak_bytes`
# is the sum of the arena, held and perm peaks and this is the term the
# benchmark was added for: what a streaming read holds.
#
# The value is pinned rather than the anchor: the row is exact and moves
# whenever the runtime does, so an anchor naming today's figure would stop
# applying the next time it moves.
set -e
grep -q '^arena_peak_bytes=[0-9]' bench/cost_golden_read.txt || {
  echo "the read golden changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^arena_peak_bytes=.*/arena_peak_bytes=9999999999/' bench/cost_golden_read.txt
grep -q '^arena_peak_bytes=9999999999' bench/cost_golden_read.txt
