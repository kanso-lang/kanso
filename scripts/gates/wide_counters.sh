#!/bin/sh
# The golden this reads carries a header, which the other counter gates' plain
# diff would report as five lines of divergence.
set -e
KANSO_COUNTERS=1 ./widebench 2>counters_wide.txt >/dev/null
grep -v '^#' bench/cost_golden_wide.txt > counters_wide_want.txt
diff counters_wide_want.txt counters_wide.txt || {
  echo "::error::wide counters diverged from cost_golden_wide.txt."
  echo "::error::A kernel's presence counter moved — a fast path was"
  echo "::error::dropped, rerouted, or intentionally changed. If"
  echo "::error::intentional, update the golden in this PR."
  exit 1
}
