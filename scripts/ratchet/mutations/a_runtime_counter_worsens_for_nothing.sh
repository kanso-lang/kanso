# The decoder retires more instructions and nothing goes the right way, with
# no entry added to bench/welfare_floor.json's history to attribute the fall.
# The same pure regression the row beside this one introduces, in the other
# vein.
#
# Two rows rather than one repointed, because the two files are read by
# different code. `bench/compile_instructions_golden.txt` is `name=value`,
# which the trend gate has always parsed; `bench/instructions_golden.txt` is
# `name value`, which it could not parse at all, so it saw an empty file and
# an empty file is indistinguishable from an unchanged one. A 139,521,205
# instruction rise on jsonbench produced no output from the gate whatsoever.
#
# The value is pinned rather than the anchor: these rows are exact per host
# and move whenever the runtime does, so an anchor naming today's figure would
# stop applying the next time one moves.
set -e
grep -q '^jsonbench [0-9]' bench/instructions_golden.txt || {
  echo "the runtime instruction rows changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's/^jsonbench [0-9].*/jsonbench 9999999999/' bench/instructions_golden.txt
grep -q '^jsonbench 9999999999' bench/instructions_golden.txt
