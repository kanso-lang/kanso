#!/bin/sh
# The pool's cell is how the pool holds its work, and the thunk counters both
# engines agree on leave it out. This mutation builds it as an ordinary cell,
# and the mem golden's `thunk_allocs=0` goes to 1.
set -e
line='    KValue v = k_thunk_vnew(site, argc, 1, ap);'
[ "$(grep -cxF "$line" src/runtime/core.c)" -eq 1 ] || {
  echo "the pooled cell's maker moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    KValue v = k_thunk_vnew(site, argc, 1, ap);$/    KValue v = k_thunk_vnew(site, argc, 0, ap);/' src/runtime/core.c
if grep -qxF "$line" src/runtime/core.c; then exit 1; fi
