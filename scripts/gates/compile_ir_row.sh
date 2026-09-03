#!/bin/sh
# Which value this run should have counted, and whether it did.
#
# Split out of scripts/gates/compile_instructions.sh so it can be watched
# failing. That gate's own answer costs a callgrind run over the whole front
# end, which is not a thing a spec can drive five times to see five different
# refusals — and a refusal nobody has watched is the shape this repo keeps
# finding green and blind. Everything here is two files and four strings.
#
#   sh scripts/gates/compile_ir_row.sh <table> <golden> <key> <counted>
#
# <table> is bench/compile_instructions_by_cpu.txt, one row per chip, and its
# header carries why the row is keyed at all. <golden> is
# bench/compile_instructions_golden.txt, which holds the same quantity as a
# bare `compile_instructions=` for welfare, the trend gate and golden_prose.
# <key> is scripts/gates/dispatch.sh `key`. <counted> is what this run read.
#
# A ROW MAY PIN ONE VALUE OR TWO, and both are exact:
#
#     family0x6-model0xcf 41831767 41832275
#
# One chip and ONE BINARY read those two numbers, 508 apart. The evidence is
# two CI runs whose gate printed the same binary sha256 de5bfab22fbd and the
# same cpu family 0x6 model 0xcf: fc993f83 counted 41,831,767 and e47e412d
# counted 41,832,275. Neither named suspect explains it — the loader already
# sorts what a compile reads, and the second of those runs had `setarch -R`
# applied while the first did not.
#
# Two is the cap, and the cap is the whole difference between this and the
# tolerance the vein refused. A band wide enough to hold 508 also holds
# kanso#1226's -5,621, which was a real change to the compiler. A pair admits
# two numbers that were each measured, and refuses a third.
#
# Eight ways to be wrong and one way to be right. None of the eight is a
# warning: an unknown number never passes.
set -e
table=$1
golden=$2
key=$3
got=$4
if [ -z "$table" ] || [ -z "$golden" ] || [ -z "$key" ] || [ -z "$got" ]; then
  echo "::error::this wants <table> <golden> <key> <counted>"
  exit 2
fi

rows=$(grep -v '^#' "$table" | grep -v '^[[:space:]]*$' || true)

if [ -z "$rows" ]; then
  echo "::error::no chip has a row in $table, so there is nothing to check"
  echo "::error::this run's $got against. Add it as the first row, which"
  echo "::error::makes it the reference series:"
  echo "::error::"
  echo "::error::    $key $got"
  echo "::error::"
  echo "::error::and set compile_instructions=$got in $golden to match."
  exit 1
fi

# A key may appear once. The lookup below takes the first row matching a key
# and stops, so a second row for the same chip is read by nobody and looks
# authoritative to everybody — which is how a golden goes quiet. It is not a
# hypothetical: the Emerald Rapids row was once corrected in place AND appended
# in one edit, and every gate stayed green on the file that resulted. Checked
# before the lookup so the reader is told about the duplicate rather than sent
# to re-sit a front end that did not move.
dupes=$(echo "$rows" | awk '{print $1}' | sort | uniq -d)

if [ -n "$dupes" ]; then
  echo "::error::$table has more than one row for a chip:"
  for d in $dupes; do
    echo "::error::"
    echo "::error::    $d"
    echo "$rows" | awk -v k="$d" '$1 == k { print "::error::        " $0 }'
  done
  echo "::error::"
  echo "::error::Only the first is ever read, so the rest say nothing while"
  echo "::error::looking like a sitting. A re-sitting EDITS the chip's row"
  echo "::error::where it stands; it does not append a second one. Delete the"
  echo "::error::duplicates, keeping the value the re-sitting meant."
  exit 1
fi

# How many values a row pins, checked over the WHOLE table for the reason the
# duplicate check is: a malformed row that only this run's chip is spared makes
# the file wrong for three runs in four and read by nobody on the fourth.
#
# A row with no value would fall through the lookup below and be reported as a
# chip the table has never seen, sending a reader to add a row for a key that
# already has one. A row with three is a band being assembled a reading at a
# time, and every value added to it is a mode nobody explained.
malformed=$(echo "$rows" | awk 'NF < 2 || NF > 3 { print }')

if [ -n "$malformed" ]; then
  echo "::error::$table has a row that does not pin one value or two:"
  echo "::error::"
  echo "$malformed" | while IFS= read -r bad; do
    echo "::error::    $bad"
  done
  echo "::error::"
  echo "::error::A row is a key and then its counted value, or a key and the"
  echo "::error::TWO values one chip has been seen to read on one binary. Two"
  echo "::error::is the cap: the pair is the ruled fallback for a residual"
  echo "::error::nobody could explain away, and a third value would make it a"
  echo "::error::band by enumeration — which admits the size of change this"
  echo "::error::vein exists to catch."
  exit 1
fi

# And a pair whose halves are the same number, which admits exactly what a
# single admits while claiming a second mode was measured.
doubled=$(echo "$rows" | awk 'NF == 3 && $2 == $3 { print }')

if [ -n "$doubled" ]; then
  echo "::error::$table pins a value twice on one row:"
  echo "::error::"
  echo "$doubled" | while IFS= read -r bad; do
    echo "::error::    $bad"
  done
  echo "::error::"
  echo "::error::That is a reading pasted twice, not a pair. It says the chip"
  echo "::error::was seen to read two values when it was seen to read one."
  echo "::error::Drop the repeat, or write the value the second sitting"
  echo "::error::actually counted."
  exit 1
fi

# The golden's bare line is the first row, checked on every run whatever chip
# this is, so a per-chip re-sitting cannot leave the number welfare reads
# behind. Two files and no measurement, so it costs nothing to always ask.
ref_key=$(echo "$rows" | head -1 | awk '{print $1}')
ref_val=$(echo "$rows" | head -1 | awk '{print $2}')
gold_val=$(sed -n 's/^compile_instructions=//p' "$golden")

if [ "$ref_val" != "$gold_val" ]; then
  echo "::error::$golden says compile_instructions=$gold_val and the first"
  echo "::error::row of $table says $ref_key $ref_val. welfare, the trend"
  echo "::error::gate and golden_prose read the golden; this gate reads the"
  echo "::error::table. The two disagreeing means the objective is tracking a"
  echo "::error::number no chip counted. Set the golden's bare line to"
  echo "::error::$ref_val, or move the row you re-sat to the top."
  exit 1
fi

want=$(echo "$rows" | awk -v k="$key" \
  '$1 == k { $1 = ""; sub(/^[ \t]+/, ""); print; exit }')

if [ -z "$want" ]; then
  echo "::error::nothing in $table was counted on $key, so this run's $got"
  echo "::error::cannot be compared to anything. That is a refusal and not a"
  echo "::error::warning: an unrecorded chip is an unsat row, and letting one"
  echo "::error::through is how three runs in four would wave a regression"
  echo "::error::past — which is the objection that killed recording a single"
  echo "::error::feature block and skipping elsewhere."
  echo "::error::"
  echo "::error::If the front end is unchanged on this branch, this chip is"
  echo "::error::new to the pool and wants a sitting. Add"
  echo "::error::"
  echo "::error::    $key $got"
  echo "::error::"
  echo "::error::to $table. If the front end DID change, every chip's row went"
  echo "::error::stale at once and each wants re-sitting, one per CI run."
  exit 1
fi

landed=no
for one in $want; do
  if [ "$one" = "$got" ]; then
    landed=yes
  fi
done

if [ "$landed" = no ]; then
  echo "::error::the work the FRONT END does changed on $key: $table pins"
  echo "::error::$want and this run counted $got. A rise is a regression to"
  echo "::error::explain and a fall is a win to bank — say which in"
  echo "::error::design/compiler-log.md and update the row."
  echo "::error::The row is keyed by silicon, so the runner is not the answer"
  echo "::error::here: the same family and model counted both numbers."
  echo "::error::This is the dimension that stayed silent while a quarter of"
  echo "::error::the compiler's work went away — allocations, rounds, visits"
  echo "::error::and peak were all identical across that change."
  echo "::error::If the front end moved, the other chips' rows moved with it"
  echo "::error::and are stale until CI re-sits each of them."
  exit 1
fi

echo "compile_instructions $got, matching $table on $key (pinned: $want)"
