#!/bin/sh
# An append onto a bytes value with no buffer goes straight to the grow,
# tested inline at the caller. This mutation drops that test, so every such
# append enters k_b_append_fit and pays its frame on the way to the grow.
# The output is the same; the work vein is what sees it.
set -e
line='    if (!(a->cap & ~1LL)) return k_b_append_grow(acc, a, src, n, mutate);'
[ "$(grep -cxF "$line" src/runtime.c)" -eq 1 ] || {
  echo "the grow test moved; this mutation needs rewriting" >&2
  exit 1
}
grep -vxF "$line" src/runtime.c > src/runtime.c.new
mv src/runtime.c.new src/runtime.c
if grep -qxF "$line" src/runtime.c; then exit 1; fi
