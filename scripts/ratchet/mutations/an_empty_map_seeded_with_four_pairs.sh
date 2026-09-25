#!/bin/sh
# An empty map opens with room for four pairs again.
#
# `{}` seeds the decoder's objects, which run one to five keys, and at four
# pairs every fifth key grew the map. Put back, the empty literal's
# allocation shrinks by two slots and the mem vein reads the old bytes.
set -e
f=src/runtime.c
grep -q 'define K_MAP_SEED 10' src/runtime.c
grep -qxF '#define K_MAP_SEED 10' "$f" || {
  echo "the map seed moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^#define K_MAP_SEED 10$|#define K_MAP_SEED 8|' "$f"
grep -qxF '#define K_MAP_SEED 8' "$f"
