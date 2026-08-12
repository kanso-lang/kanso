#!/bin/sh
set -e
KANSO_COUNTERS=1 ./basket 2>counters_basket.txt >/dev/null
diff bench/cost_golden_basket.txt counters_basket.txt || {
  echo "::error::basket counters diverged — the welfare index reads"
  echo "::error::this vein, and it samples a spread of what the"
  echo "::error::language does rather than one shelf of it. If"
  echo "::error::intentional, update the golden in this PR."
  exit 1
}
