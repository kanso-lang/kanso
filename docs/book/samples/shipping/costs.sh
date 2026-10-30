#!/bin/sh
# wordfreq's cost golden: the allocations it makes on three fixed texts.
# CI runs this and fails when the result differs from costs.txt.
set -e
for f in tea.txt tea10.txt tea100.txt; do
  printf '%s ' "$f"
  KANSO_COUNTERS=1 kanso run wordfreq2 -- "$f" 2>&1 >/dev/null | grep '^allocs='
done
