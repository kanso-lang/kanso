# The library's escaper drops its fast path: every string is folded byte by
# byte through a call, whether or not it holds a byte that needs escaping.
#
# This is the defect livebench was added for. bench/encodebench runs the same
# program four hundred times over and would not move by one instruction,
# because it vendors a snapshot of lib/json frozen at 20ab931d on purpose.
# bench/oneshot imports the real library and encodes exactly once, so it sees a
# sliver. The live vein sees all of it.
#
# The arm is repointed rather than deleted, so the program still answers the
# same bytes and the checksum gate stays green — a mutation that changed the
# OUTPUT would turn this gate red for a reason that has nothing to do with the
# counters it exists to watch. `s` is re-encoded rather than dropped for a
# duller reason: the language refuses a body that leaves a parameter unused, so
# a mutation that simply deleted the fast path did not compile, and a build that
# never runs is UNBUILT rather than red.
set -e
grep -q '^  if (length bs < n) (text/append acc s) (escape_able acc bs)$' \
  lib/json/text.kso || {
  echo "lib/json's escape fast path changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  if (length bs < n) (text/append acc s) (escape_able acc bs)$|  if (length bs < n) (escape_able acc (text/bytes s)) (escape_able acc bs)|' \
  lib/json/text.kso
grep -q 'escape_able acc (text/bytes s)' lib/json/text.kso
