#!/bin/sh
# An accumulator whose buffer has already left the arena grows by realloc,
# and the permanent peak holds one buffer. This mutation turns the regrow off,
# so every grow mallocs, copies and frees and the peak holds two; the witness
# is the mem fixture an_accumulator_regrows_where_it_is.
set -e
line='    if (perm && k_buf_malloced(k_buf_of(l->items))) {'
[ "$(grep -cxF "$line" src/runtime.c)" -eq 1 ] || {
  echo "the list's regrow moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (perm \&\& k_buf_malloced(k_buf_of(l->items))) {$/    if (0 \&\& perm \&\& k_buf_malloced(k_buf_of(l->items))) {/' src/runtime.c
if grep -qxF "$line" src/runtime.c; then exit 1; fi
