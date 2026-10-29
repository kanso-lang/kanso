#!/bin/sh
# A float that passes at the guessed place count walks down to the fewest
# places by taking the zeros off its scaled decimal, which needs no division
# of doubles. This mutation walks down by trying each place below, as the
# search did before: the same digits, and on runbench about 1.2 million
# instructions more.
set -e
t='        while (p > 0 && k_tenfold((uint64_t)m)) {'
n=$(grep -cxF "$t" src/runtime/ryu.c)
[ "$n" -eq 1 ] || { echo "the walk down from a passing guess changed shape ($n); rewrite this" >&2; exit 1; }
awk -v t="$t" '
  $0 == t {
    print "        int64_t below;"
    print "        while (p > 0 && ryu_short_at(f, p - 1, &below)) {"
    getline; print
    getline; print "            m = below;"
    next
  }
  { print }' src/runtime/ryu.c > src/runtime/ryu.c.new
mv src/runtime/ryu.c.new src/runtime/ryu.c
test "$(grep -cxF "$t" src/runtime/ryu.c)" = 0
grep -qxF '            m = below;' src/runtime/ryu.c
