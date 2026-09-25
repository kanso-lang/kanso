#!/bin/sh
# An empty list opens with room for four again.
#
# The decoder's arrays run one to six elements, and at four every array of
# five or six grew on its fifth push. Put back, the empty literal's
# allocation shrinks by two slots and the mem vein reads the old bytes.
set -e
f=src/runtime.c
grep -q 'define K_LIST_SEED 6' src/runtime.c
grep -qxF '#define K_LIST_SEED 6' "$f" || {
  echo "the list seed moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^#define K_LIST_SEED 6$|#define K_LIST_SEED 4|' "$f"
grep -qxF '#define K_LIST_SEED 4' "$f"
