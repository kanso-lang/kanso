#!/bin/sh
# A cluster of groups in one tail cycle goes back to being disqualified by any
# tail edge from outside it. The mem fixture that pins the demotion then reads
# nine arena blocks where its golden says one, and beat_iters zero where the
# golden says 400,001.
#
# It restores the old refusal rather than editing the golden, because an edit
# would prove nothing about the counters. A fixture whose numbers came from
# somewhere other than the bracket survives an edited golden and dies here.
set -e
A='            if entries.iter().any(|(from, _)| inside.contains(groups[*from].0.as_str())) {' \
awk '
  !hit && $0 == ENVIRON["A"] { print "            if true {"; hit = 1; next }
  { print }
  END { if (!hit) exit 3 }
' src/beat.rs > src/beat.rs.mut || {
  rm -f src/beat.rs.mut
  echo "the cluster entry test moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/beat.rs.mut src/beat.rs
grep -q 'inside.contains(groups\[\*from\]' src/beat.rs && exit 1
exit 0
