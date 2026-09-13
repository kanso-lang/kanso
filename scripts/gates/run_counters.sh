#!/bin/sh
# The run program's memory counters. `arena_peak_bytes` plus `held_peak_bytes`
# plus `perm_peak_bytes` is the objective's whole run-memory term since the
# 2026-09-06 gavel, so this is the one cost golden welfare reads rather than
# one of twelve. The rest of the rows are diagnostics: they say WHERE a peak
# moved, which the single term cannot.
set -e
KANSO_COUNTERS=1 ./runbench-counters 2>run_counters.txt >/dev/null
diff bench/cost_golden_run.txt run_counters.txt || {
  echo "::error::the run program's counters diverged from"
  echo "::error::bench/cost_golden_run.txt. Its three peak rows are the"
  echo "::error::objective's run-memory term, so a move here moves welfare."
  echo "::error::If intentional, regenerate the golden in this PR and say"
  echo "::error::which way it went in design/compiler-log.md."
  exit 1
}
