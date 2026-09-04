#!/bin/sh
# Every counter the objective weighs appears in the history row.
#
# The welfare column is written under one formula over the counters each row
# carries, so a row that loses a counter is a row scored on less of the
# objective than the rest of the line. It is not fatal — the terms renormalize
# and the row records its own `scored_weight` — and that is exactly why it
# wants a gate: a row quietly scored on half the formula reads like every
# other point on the chart. On 2026-09-03 the newest row held 12 of the 24.
#
# perf_record builds the group FROM `welfare --counters`, so the names cannot
# drift apart. What can still go wrong is the group being dropped where the row
# is concatenated, and that is what this reads: the row as written, against the
# objective as it stands.
#
# It runs HERE rather than as a cargo spec because perf_record shells out to
# ./target/release/kanso, which the specs job never builds. A spec that needs a
# release build in a debug job is a spec that fails for its environment.
set -e
cd "$(dirname "$0")/../.."
# CI hands in the row it just built, so the gate reads the artifact rather than
# a second one. Run without an argument it builds its own, which is what lets
# the ratchet read it standalone before mutating anything.
row=$1
if [ -z "$row" ]; then
  row=$(mktemp)
  trap 'rm -f "$row"' EXIT
  ./target/release/kanso run scripts/perf_record > "$row"
fi

missing=''
count=0
for name in $(./target/release/kanso run scripts/welfare -- --counters |
                sed -n 's/=.*//p'); do
  count=$((count + 1))
  grep -q "\"$name\"" "$row" || missing="$missing $name"
done

[ "$count" -gt 0 ] || {
  echo "the objective named no counters; this gate read nothing" >&2
  exit 1
}

[ -z "$missing" ] || {
  echo "the row is missing counters the objective weighs:$missing" >&2
  echo "the row would be scored on less of the objective than the" >&2
  echo "rest of the line while looking like every point on it" >&2
  exit 1
}

echo "the row carries all $count counters the objective weighs"
