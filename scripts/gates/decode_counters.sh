#!/bin/sh
set -e
KANSO_COUNTERS=1 ./jsonbench 2>counters.txt >/dev/null
diff bench/cost_golden.txt counters.txt || {
  echo "::error::cost counters diverged from bench/cost_golden.txt — a"
  echo "::error::performance-relevant change (allocs, arena blocks, beat"
  echo "::error::iterations). If intentional, update the golden in this PR."
  exit 1
}
