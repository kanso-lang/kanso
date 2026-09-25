#!/bin/sh
# A list that leaves the arena grows through every step again.
#
# An accumulator that outlives its beat keeps its buffer outside the arena,
# and every grow there is a realloc. It used to leave at eight slots and pass
# through sixteen, sixty-four and two hundred and fifty-six on the way to a
# thousand; it leaves at two hundred and fifty-six now. Without the floor the
# mem vein's bytes_malloc counters read the old number of grows.
set -e
f=src/runtime.c
grep -q 'if (perm && cap < 256) cap = 256;' src/runtime.c
grep -qxF '    if (perm && cap < 256) cap = 256;' "$f" || {
  echo "the permanent floor moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    if (perm \&\& cap < 256) cap = 256;$/d' "$f"
! grep -qF 'if (perm && cap < 256) cap = 256;' "$f"
