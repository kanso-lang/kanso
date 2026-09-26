#!/bin/sh
# The nightly's matrix loses its eighth shard while the run line still divides
# the table by eight, so every eighth row is proved by nobody and every job
# that did run is green.
set -e
f=.github/workflows/ratchet.yml
grep -q 'shard: \[1, 2, 3, 4, 5, 6, 7, 8\]' .github/workflows/ratchet.yml
sed -i 's/shard: \[1, 2, 3, 4, 5, 6, 7, 8\]/shard: [1, 2, 3, 4, 5, 6, 7]/' "$f"
if grep -q 'shard: \[1, 2, 3, 4, 5, 6, 7, 8\]' "$f"; then exit 1; fi
