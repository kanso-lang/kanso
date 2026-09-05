#!/bin/sh
set -e
KANSO_COUNTERS=1 ./livebench 2>counters_live.txt >/dev/null
diff bench/cost_golden_live.txt counters_live.txt || {
  echo "::error::the live encode counters diverged. This vein"
  echo "::error::watches lib/json's encoder, which bench/encodebench"
  echo "::error::cannot: that directory holds a frozen copy on"
  echo "::error::purpose. If intentional, update the golden in this PR."
  exit 1
}
