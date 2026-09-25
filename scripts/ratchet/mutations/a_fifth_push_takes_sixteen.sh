#!/bin/sh
# A list growing out of four slots takes eight, then sixteen, and doubles
# again only past that. This mutation doubles every time, as the grow did,
# so a list of five holds sixteen slots; the witness is the mem fixture
# a_fifth_push_takes_eight_slots.
set -e
line='    if (cap > 16) cap <<= 1;'
[ "$(grep -cxF "$line" src/runtime.c)" -eq 1 ] || {
  echo "the grow's doubling moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (cap > 16) cap <<= 1;$/    cap <<= 1;/' src/runtime.c
grep -qxF '    cap <<= 1;' src/runtime.c
