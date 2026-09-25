#!/bin/sh
# A join of pieces that each know their character count writes the sum into
# the joined string. This mutation drops that write, so the first `length` of
# the joined string scans it. The run program's index phase doubles its
# subject by joining it to itself and asks its length each time;
# `str_scan_bytes` rises from 945,324 to 4,092,666.
set -e
grep -q '^    k_join_seed_count(os, l, ss);$' src/runtime.c || {
  echo "the join's count seed changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    k_join_seed_count(os, l, ss);$||' src/runtime.c
! grep -q '^    k_join_seed_count(os, l, ss);$' src/runtime.c
