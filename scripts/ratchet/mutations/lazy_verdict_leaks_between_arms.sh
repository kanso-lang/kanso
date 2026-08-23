#!/bin/sh
# The per-arm fold becomes a per-key disjunction, so one arm qualifying makes
# the whole group lazy again — the shape that had regexp's character-class arm
# thunking `at <= length s` once per character examined. Measured on the scan
# benchmark: thunk_allocs 0 becomes 501,502 and peak RSS rises about 92 MB.
#
# It edits the fold rather than the golden because the golden is what an edit
# would prove nothing about. A gate reading a counter no program moves, or one
# pinned to a benchmark that allocates no thunks at all, survives an edited
# golden and dies here.
set -e
A='            *seen = *seen && qualifies;' \
awk '
  !hit && $0 == ENVIRON["A"] { print "            *seen = *seen || qualifies;"; hit = 1; next }
  { print }
  END { if (!hit) exit 3 }
' src/demand.rs > src/demand.rs.mut || {
  rm -f src/demand.rs.mut
  echo "the lazy fold moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/demand.rs.mut src/demand.rs
grep -q '\*seen = \*seen && qualifies;' src/demand.rs && exit 1
exit 0
