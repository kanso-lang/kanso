#!/bin/sh
set -e
KANSO_COUNTERS=1 ./oneshot 2>counters_oneshot.txt >/dev/null
diff bench/cost_golden_oneshot.txt counters_oneshot.txt || {
  echo "::error::one-shot counters diverged — the peak-footprint"
  echo "::error::term's input moved. If intentional, update the"
  echo "::error::golden in this PR."
  exit 1
}
