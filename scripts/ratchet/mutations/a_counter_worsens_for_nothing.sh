# A counter goes the wrong way and nothing goes the right way, with no entry
# added to bench/welfare_floor.json's history to attribute the fall. That is
# the pure regression the trend gate refuses outright — the one shape it exists
# to stop, and the shape it stopped refusing would leave every silent drift to
# the welfare scalar alone.
#
# The value is pinned rather than the anchor: compile_peak_bytes is exact per
# host and moves whenever the front end does, so an anchor naming today's
# figure would stop applying the next time it moves.
set -e
sed -i 's/^compile_peak_bytes=.*/compile_peak_bytes=999999999/' \
  bench/compile_memory_golden.txt
grep -q '^compile_peak_bytes=999999999' bench/compile_memory_golden.txt
