#!/bin/sh
# A beat past the deepest mark rewinds to the mark that stands in for none.
#
# Past the beat stack the cached top names k_beat_none, whose regime bit sends
# the rewind to its slow path, and the slow path returns at once for it. Take
# that return out and the rewind hands back every block the arena holds, and
# the strings the levels above are still reading are built over.
set -e
f=src/runtime.c
grep -q 'void k_beat_rewind_slow(KMark\* m)' src/runtime.c
line='    if (m == &k_beat_none) return;'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the sentinel's return moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    if (m == &k_beat_none) return;$/d' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
