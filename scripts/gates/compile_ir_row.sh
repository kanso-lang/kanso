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
# Four ways to be wrong and one way to be right. None of the four is a warning:
# an unknown number never passes.
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

want=$(echo "$rows" | awk -v k="$key" '$1 == k { print $2; exit }')

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

if [ "$want" != "$got" ]; then
  echo "::error::the work the FRONT END does changed on $key: $table says"
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

echo "compile_instructions $got, matching $table on $key"
