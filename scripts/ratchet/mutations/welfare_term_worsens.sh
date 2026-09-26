#!/bin/sh
# The run program claims to have done far more work than the baseline, which
# is a fall in the run-speed term and so a fall in the index.
#
# It worsened jsonbench until 2026-09-26. The run-speed term has read the one
# consolidated run program since the 2026-09-06 gavel, so jsonbench stopped
# moving the index that day and this row could not go red. The sharded nightly
# was the first run to reach it and reported it BLIND.
set -e
sed -i.bak 's/^runbench [0-9]*$/runbench 9999999999/' bench/instructions_golden.txt
rm -f bench/instructions_golden.txt.bak
grep -q '^runbench 9999999999$' bench/instructions_golden.txt
