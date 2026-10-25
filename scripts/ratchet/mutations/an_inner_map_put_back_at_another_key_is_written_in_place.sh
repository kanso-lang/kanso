#!/bin/sh
# Accept a take that goes back into the map at a different key.
#
# The inner map read at one key is written in place and stored at another,
# so both keys hold the one map.
set -e
sed -i.bak 's/if is(head, "put") \&\& args.len() == 3 \&\& is(\&args\[0\], p) \&\& is(\&args\[1\], k) {/if is(head, "put") \&\& args.len() == 3 \&\& is(\&args[0], p) \&\& !k.is_empty() {/' src/linear.rs
rm -f src/linear.rs.bak
test "$(grep -c '&& is(&args\[0\], p) && !k.is_empty() {' src/linear.rs)" = 1
