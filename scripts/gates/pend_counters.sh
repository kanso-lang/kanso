#!/bin/sh
# The golden this reads carries a header, which the other counter gates' plain
# diff would report as six lines of divergence.
set -e
KANSO_COUNTERS=1 ./pendbench 2>counters_pend.txt >/dev/null
grep -v '^#' bench/cost_golden_pend.txt > counters_pend_want.txt
diff counters_pend_want.txt counters_pend.txt || {
  echo "::error::pending-cell counters diverged from cost_golden_pend.txt."
  echo "::error::The thunk-aware evacuation's presence pins moved — the"
  echo "::error::walk was dropped, rerouted, or intentionally changed. If"
  echo "::error::intentional, update the golden in this PR."
  exit 1
}
