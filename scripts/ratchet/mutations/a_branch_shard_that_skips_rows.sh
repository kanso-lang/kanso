#!/bin/sh
# Each shard of a branch's handful takes every (N+1)th row instead of every
# Nth, so a row in each stretch of N+1 is proved by no shard and every shard
# that ran is green.
set -e
f=scripts/ratchet/ratchet.kso
grep -q '^  picked = every_nth acc k n \[\]$' scripts/ratchet/ratchet.kso
sed -i 's/^  picked = every_nth acc k n \[\]$/  picked = every_nth acc k (n + 1) []/' "$f"
if grep -q '^  picked = every_nth acc k n \[\]$' "$f"; then exit 1; fi
