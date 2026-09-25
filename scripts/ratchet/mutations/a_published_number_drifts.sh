#!/bin/sh
# A number the compiler page states as a fact about the present stops matching
# the golden it quotes.
#
# It moves the COMPILE family, which this gate could not reach until now: it
# read the decode and encode cost goldens and nothing else, and six numbers in
# the whole site carried the attribute, all six of them decode counters. So
# compiler.html said the front end DOES 17,786 expression visits while the
# golden went 17,886, then 16,818, then 16,806 underneath it, and this gate
# reported no drift through all three. Pointing the row at the family that was
# unreachable is what keeps it from going unreachable again.
#
# Anchored on the attribute rather than the value: the counter moves whenever
# the front end does, so a mutation naming today's figure would stop applying
# the next time it moves.
set -e
target='data-golden="compile.front_end_visits">'
grep -qF "$target" docs/compiler.html || {
  echo "the compile-vein tag moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s|\(${target}\)[0-9,]*|\1999,999|" docs/compiler.html
grep -qF "${target}999,999" docs/compiler.html
