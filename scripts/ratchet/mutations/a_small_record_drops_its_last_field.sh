#!/bin/sh
# A record with four fields or fewer copies them one KValue at a time rather
# than through a call into memcpy, and the loop drops the last one.
#
# The mutation is the off-by-one a hand-written copy loop invites, and it is
# the reason the loop needs a row at all: the memcpy it replaced could not be
# wrong about the count, because the count was an argument. A loop can be, and
# a record that quietly loses its final field is a miscompilation rather than a
# slowdown.
#
# `k_rec_reuse` a few lines above already copies fields with the same shape,
# which is why this one is written over a local `dst` -- so the guard and the
# sed name one site and not two.
set -e
grep -qF 'for (long long i = 0; i < n; i++) dst[i] = args[i];' src/runtime.c || {
  echo "the small-record copy loop moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|for (long long i = 0; i < n; i++) dst\[i\] = args\[i\];|for (long long i = 0; i < n - 1; i++) dst[i] = args[i];|' \
  src/runtime.c
grep -qF 'for (long long i = 0; i < n - 1; i++) dst[i] = args[i];' src/runtime.c
