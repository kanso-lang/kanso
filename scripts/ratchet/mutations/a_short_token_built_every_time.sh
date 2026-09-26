#!/bin/sh
# A short token is built again each time the decoder reads it.
#
# k_b_utf8_slice_raw hands a token of four to seven bytes back from a cache of
# permanent strings. Without the cache every read allocates, validates and
# copies, and the mem vein's short-token fixture reads 600 strings in the
# arena where it pinned five permanent ones.
set -e
f=src/runtime.c
grep -q 'k_token_miss' src/runtime.c
line='    if (len >= 4 && len <= 7) {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the short-token cache moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (len >= 4 \&\& len <= 7) {$/    if (0 \&\& len >= 4 \&\& len <= 7) {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
