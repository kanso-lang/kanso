#!/bin/sh
# The scored counter group is dropped where the row is assembled.
#
# perf_record builds that group from `welfare --counters`, so its names cannot
# drift from the objective's. What can still happen is the group never reaching
# the row — and a row without it is a row the replay skips, silently, which is
# the state the newest row was in on 2026-09-03 when it held 12 of 24.
#
# Anchored on the concatenation rather than on a counter name, because the
# names move whenever the model does and this is about the group's presence.
set -e
target='  measured = text/concat own scored'
grep -qF "$target" scripts/perf_record/perf_record.kso || {
  echo "the row's assembly moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^  measured = text/concat own scored$|  measured = own|' \
  scripts/perf_record/perf_record.kso
grep -qF '  measured = own' scripts/perf_record/perf_record.kso
