#!/bin/sh
# The short-decimal search starts at the place count the last float took and
# walks down or up from there. This mutation starts it at zero every time,
# which is the search as it was: the same digits, and on runbench about five
# tries a float where the guess makes two.
#
# The work vein reads runbench about thirteen million instructions heavier.
set -e
line='    int p = k_ryu_places;'
test "$(grep -cxF "$line" src/runtime.c)" = 1 || {
  echo "the float search's starting guess changed shape; rewrite this" >&2
  exit 1
}
awk -v line="$line" '$0 == line { print "    int p = 0;"; next } { print }' \
  src/runtime.c > src/runtime.c.new
mv src/runtime.c.new src/runtime.c
test "$(grep -cxF "$line" src/runtime.c)" = 0
