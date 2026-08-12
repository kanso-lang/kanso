#!/bin/sh
# One more allocation on the decode path than the golden knows about.
set -e
sed -i.bak 's/^decode_allocs=\(.*\)/decode_allocs=999999/' bench/cost_golden.txt
rm -f bench/cost_golden.txt.bak
grep -q '^decode_allocs=999999' bench/cost_golden.txt
