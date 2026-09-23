#!/bin/sh
# Every stashed function table, packed, at the end of the job.
#
# WHY THIS IS NOT JUST A SECOND PRINT. The tables are the instrument for the
# standing row that moves by three instructions on byte-identical source: the
# only way to name which frames carry a delta is to diff two jobs' tables. A
# log API returns the TAIL of a job and caps it -- 5,000 lines, whatever tail
# length is asked for. On 2026-09-22 the `cost goldens` job ran 20,020 lines
# and the 5,000 it handed back held two tables of the twenty, so eighteen were
# written down and unreadable. Packing gzip+base64 turns 1,726 rows into 190
# lines, which puts every table inside the tail together.
#
# TO READ ONE BACK, from a saved copy of the job log:
#
#   sed -n 's/^[^ ]*Z //p' job.log \
#     | sed -n '/^#table interp rows=/,/^#end interp$/p' \
#     | sed '1d;$d' | base64 -d | gunzip > interp.txt
#
# The sha256 on the #table line is of the DECODED table, so a reader can tell
# a truncated tail from a whole one.
stash=${KANSO_FUNCTION_TABLES:-/tmp/function-tables}

# The whole point is to fit, so the budget is checked rather than assumed. It
# is well under the 5,000 the API returns, because the steps after this one
# spend lines too and the count here grows with every gate that stashes.
budget=${KANSO_FUNCTION_TABLES_BUDGET:-3500}

if [ ! -d "$stash" ]; then
  echo "no function tables were stashed: $stash does not exist"
  exit 0
fi

packed=$(mktemp)
manifest=$(mktemp)
names=''
any=0

for t in "$stash"/*.txt; do
  [ -e "$t" ] || continue
  any=1
  name=$(basename "$t" .txt)
  names="$names $name"
  rows=$(wc -l < "$t" | tr -d ' ')
  sum=$(sha256sum "$t" | cut -d' ' -f1)
  body=$(mktemp)
  # -n keeps the source name and mtime out of the gzip header, so the same
  # table packs to the same bytes in every job and a reader diffing two packed
  # blocks sees only what the profile did.
  # The trailing `echo` is load-bearing: `fold` ends its LAST chunk without a
  # newline, so without it `#end $name` lands on the end of the final base64
  # line and the marker a reader selects on is not there. Caught by running the
  # thing against a real profile on 2026-09-23, after a spec whose fixture
  # happened to fold on a 200-byte boundary said it was fine.
  { gzip -9 -n -c "$t" | base64 | tr -d '\n' | fold -w 200; echo; } > "$body"
  lines=$(wc -l < "$body" | tr -d ' ')
  {
    echo "::group::function table, packed: $name ($rows rows, $lines lines)"
    echo "#table $name rows=$rows lines=$lines sha256=$sum"
    cat "$body"
    echo "#end $name"
    echo "::endgroup::"
  } >> "$packed"
  echo "  $name  rows=$rows  packed=$lines" >> "$manifest"
  rm -f "$body"
done

if [ "$any" = 0 ]; then
  echo "no function tables were stashed in $stash"
  rm -f "$packed" "$manifest"
  exit 0
fi

total=$(wc -l < "$packed" | tr -d ' ')

echo "packed function tables: $total lines, budget $budget"
cat "$manifest"
if [ "$total" -gt "$budget" ]; then
  # A warning and not a failure: the gate's own verdict is already in, and a
  # reader who gets a short tail can see from the index below which tables the
  # cut took. It still wants fixing, because the tables are the instrument.
  echo "::warning::the packed tables run $total lines against a budget of" \
       "$budget, so a 5,000-line log tail may not hold them all. Pack fewer" \
       "tables, or move this step later, or raise the budget deliberately."
fi

cat "$packed"

# The index goes LAST and unpacked, so a reader who got only the final handful
# of lines still learns which tables this job carried and can say which of
# them the tail cut off.
echo "#tables$names"
rm -f "$packed" "$manifest"
