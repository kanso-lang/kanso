# A counter goes the wrong way and nothing goes the right way, with no entry
# added to bench/welfare_floor.json's history to attribute the fall. That is
# the pure regression the trend gate refuses outright.
#
# It worsens compile_instructions specifically, because that vein was INVISIBLE
# to this gate until 2026-08-26: the golden list held compile_golden and
# compile_memory and neither measured vein, so an instruction count could move
# by a million with the listing silent. Pointing the row at the counter that
# was blind is what keeps it from going blind again.
#
# The value is pinned rather than the anchor: the row is exact per host and
# moves whenever the front end does, so an anchor naming today's figure would
# stop applying the next time it moves.
set -e
sed -i 's/^compile_instructions=.*/compile_instructions=999999999/' \
  bench/compile_instructions_golden.txt
grep -q '^compile_instructions=999999999' bench/compile_instructions_golden.txt
