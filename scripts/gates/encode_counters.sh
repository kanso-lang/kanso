#!/bin/sh
set -e
KANSO_COUNTERS=1 ./encodebench 2>counters_encode.txt >/dev/null
diff bench/cost_golden_encode.txt counters_encode.txt || {
  echo "::error::encode counters diverged from cost_golden_encode.txt."
  echo "::error::A kernel's presence counter moved — a fast path was"
  echo "::error::dropped, rerouted, or intentionally changed. If"
  echo "::error::intentional, update the golden in this PR."
  exit 1
}
