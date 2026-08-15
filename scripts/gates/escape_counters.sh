#!/bin/sh
# The one gate that can see storage the arena never held. perm_live_bytes is
# what a program still holds when it exits, and this is the only benchmark
# whose figure is bounded by its INPUT rather than by its text: 3,000 rounds,
# 3,000 accumulators, and before the registry landed it held all 24,624,000
# bytes of them to the end. Every other benchmark reads zero or a handful, so
# reintroducing the leak turns this row red and nothing else.
set -e
KANSO_COUNTERS=1 ./escapebench 2>counters_escape.txt >/dev/null
diff bench/cost_golden_escape.txt counters_escape.txt || {
  echo "::error::escape counters diverged. perm_live_bytes is the row to"
  echo "::error::read first: it is what the program still holds at exit,"
  echo "::error::and a rise there is storage nobody frees. If intentional,"
  echo "::error::update the golden in this PR."
  exit 1
}
