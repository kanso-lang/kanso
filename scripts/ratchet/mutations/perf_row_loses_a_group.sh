#!/bin/sh
# The row the numbers page charts is assembled from five groups of counters,
# and a miscompiled interpolation once collapsed each group's key names into a
# single key — so the page lost two trend series and three panel sections for
# months with this job green. perf_record checked its inputs for completeness
# and never its own output.
#
# This collapses the key the same way, which is the original failure rather
# than a stand-in for it: every pair in a group gets one name, `list/to_h`
# keeps the last, and the row loses every counter but one per group.
set -e
sed -i.bak 's|^  \["{prefix}{parts\[1\]!}" tally\]$|  ["{prefix}x" tally]|' \
  scripts/perf_record/perf_record.kso
rm -f scripts/perf_record/perf_record.kso.bak
grep -q '^  \["{prefix}x" tally\]$' scripts/perf_record/perf_record.kso
