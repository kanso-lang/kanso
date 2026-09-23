#!/bin/sh
# One callgrind profile's whole function table, printed and stashed.
#
# Printed: collapsed in the job log, where a reader on the web can open it.
# Stashed: so `function_tables_tail.sh` can re-emit every table packed, at the
# end of the job, INSIDE THE LAST 5,000 LINES. That limit is the reason this
# script exists. A log API hands back the tail of a job and caps it; asking
# for 60,000 lines of a 20,020-line job returned exactly the last 5,000, which
# held two of the seven tables. The rest were written down and unreadable.
#
# Usage: sh scripts/gates/function_table.sh <profile> <name>
#
# The table is `<self cost> <function>`, one frame per line, count FIRST so a
# reader splits ONCE on the first space. Callgrind names contain spaces, and
# splitting on every whitespace run produced a phantom ten-million-instruction
# mover on 2026-09-22.
profile=$1
name=$2
stash=${KANSO_FUNCTION_TABLES:-/tmp/function-tables}
mkdir -p "$stash"

callgrind_annotate --threshold=100 "$profile" 2>/dev/null \
  | sed -n 's/^ *\([0-9,][0-9,]*\) ([^)]*)  *\(.*\)$/\1 \2/p' > "$stash/$name.txt"

echo "::group::the whole function table: $name, for diffing this job against another"
if [ -s "$stash/$name.txt" ]; then
  cat "$stash/$name.txt"
else
  # An empty table is a fact about the instrument, not a reason to fail the
  # gate: the row it guards is compared elsewhere and says its own piece.
  echo "(no rows: callgrind_annotate produced nothing for $profile)"
fi
echo "::endgroup::"
