#!/bin/sh
# The scored counters land in the row under names the objective does not use.
#
# perf_record takes the group from `welfare --counters`, so the two cannot hold
# different SETS. What they can still hold is different SPELLINGS, and a row
# whose counters sit under a prefix looks complete to every other check while
# the replay finds none of them — the state the newest row was in on
# 2026-09-03, when it held 12 of 24 and nothing said so.
#
# A prefix rather than a deletion because deleting the group leaves `wc` and
# `scored` unused, the compiler refuses that, and a gate that goes red on a
# build error has not read the row at all.
set -e
target='  list/to_list (list/map lines (l -> kept "" l))'
grep -qF "$target" scripts/perf_record/perf_record.kso || {
  echo "the scored group's naming moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|(l -> kept "" l)|(l -> kept "run_" l)|' \
  scripts/perf_record/perf_record.kso
grep -qF '(l -> kept "run_" l)' scripts/perf_record/perf_record.kso
