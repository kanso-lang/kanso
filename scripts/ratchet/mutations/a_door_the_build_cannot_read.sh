#!/bin/sh
# build.rs reads the runtime's doors, the functions defined with K_DOORCC,
# into the list the emitter calls with preserve_nonecc, and a door it passed
# over would be called on the C convention. This mutation puts k_b_utf8's
# opening brace on a line of its own, a shape the reader does not take, and
# the build must stop.
set -e
f=src/runtime/text.c
def='K_DOORCC KValue k_b_utf8(KValue lv, const char* origin) {'
[ "$(grep -cxF "$def" $f)" -eq 1 ] || {
  echo "k_b_utf8's definition moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^\(K_DOORCC KValue k_b_utf8(KValue lv, const char\* origin)\) {$/\1\n{/' $f
grep -qxF 'K_DOORCC KValue k_b_utf8(KValue lv, const char* origin)' $f
