#!/bin/sh
# The page's replay stops agreeing with the tool it replays.
#
# The chart computes the score in javascript because the page is static, so the
# rule is stated twice — once in scripts/welfare, once in docs/numbers.html —
# and a second statement drifts silently. The chart would go on drawing a line,
# and the line would be wrong in a way that looks exactly like a real move.
#
# Anchored on the saturation term rather than a number: it is the half of the
# rule the 2026-08-29 gavel settled, and dropping it is what a reader
# reimplementing from memory does.
set -e
target='sum += r / (r + term.satiation);'
grep -qF "$target" docs/numbers.html || {
  echo "the replay's saturation moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s|sum += r / (r + term.satiation);|sum += r / (r + 1);|" docs/numbers.html
grep -qF 'sum += r / (r + 1);' docs/numbers.html
