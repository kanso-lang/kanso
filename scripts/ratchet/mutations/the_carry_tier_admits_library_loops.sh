#!/bin/sh
# scan_counters had no row of its own until this one. bench/scanbench walks a
# subject once, matching at every position and keeping nothing, and its peak
# grows with the subject because regexp/walked's scratch survives to the end:
# beat_loops drops every group whose file begins "std/" or "lib/" from the
# carry tier, and regexp/walked is one of them. This mutation clears that
# filter, so library loops are admitted.
#
# WHAT IT MOVES, measured 2026-09-16 on this benchmark: beat_iters 15 -> 16 and
# survive_slots 0 -> 2. arena_blocks does NOT move and the comment in
# bench/scanbench/scanbench reads as though it would -- that 2026-09-08 sitting
# cleared the BYTES condition and the unbracketed-entry check as well, and the
# filter is one of its three parts. The gate diffs the whole golden, so two
# moved rows turn it red exactly as arena_blocks would.
#
# Clearing the filter is a regression in the wide: the 2026-09-01 sitting
# priced the removal at -0.56 welfare, and runbench was still running after ten
# minutes against a 0.4-second baseline because every library loop begins
# evacuating. Nothing here runs runbench -- the setup builds the benchmarks and
# the gate runs scanbench-counters alone -- so the row costs a build, not a
# hang.
set -e
A='        .filter(|d| d.file.starts_with("std/") || d.file.starts_with("lib/"))' \
awk '
  !hit && $0 == ENVIRON["A"] { print "        .filter(|_d| false)"; hit = 1; next }
  { print }
  END { if (!hit) exit 3 }
' src/beat.rs > src/beat.rs.mut || {
  rm -f src/beat.rs.mut
  echo "the carry-tier filter moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/beat.rs.mut src/beat.rs
grep -q '.filter(|_d| false)' src/beat.rs
