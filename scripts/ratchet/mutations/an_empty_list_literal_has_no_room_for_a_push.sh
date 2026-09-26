#!/bin/sh
# An empty list literal's buffer opens with room for four since 2026-09-15.
# Before, it held one slot, so the second push of every array the decoder
# builds went through k_b_push_grow: 225,621 of the run program's 252,499
# grows. The literal is built by k_list_empty since 2026-09-25, with room for
# six since the list seed, and this gives the buffer it carves a capacity of
# one again. Every list holds the
# same values either way, so nothing but the work vein can see it, which is
# what the row asserts.
set -e
line='    k_buf_set_cap(b, K_LIST_SEED, 0);'
[ "$(grep -cxF "$line" src/runtime.c)" -eq 1 ] || {
  echo "the empty literal's buffer changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    k_buf_set_cap(b, K_LIST_SEED, 0);$/    k_buf_set_cap(b, 1, 0);/' src/runtime.c
if grep -qxF "$line" src/runtime.c; then exit 1; fi
