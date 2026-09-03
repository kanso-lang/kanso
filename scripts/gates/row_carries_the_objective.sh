#!/bin/sh
# Every counter the objective weighs appears in the history row.
#
# The chart replays the current formula over the stored rows and starts at the
# first row carrying the whole counter set, so a row that loses a counter is a
# row the replay must skip — and on 2026-09-03 the newest row held 12 of the
# 24, which is why the replay named no commit to start from.
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
row=${1:?usage: row_carries_the_objective.sh <row.json>}

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
  echo "the replayed welfare line cannot start while that is true" >&2
  exit 1
}

echo "the row carries all $count counters the objective weighs"
