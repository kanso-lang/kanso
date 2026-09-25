#!/bin/sh
# The instruction that could not be followed.
#
# measured_on.sh refuses a host the golden does not name, and its refusal says:
# "let CI measure it and copy the rows out of the job log." Every compile gate
# called it under `set -e`, so a mismatch stopped the gate before it measured
# anything and the job log held no rows. On 2026-09-03 the runner's rustc moved
# 1.98.0 -> 1.98.1, all three compile veins refused at once, and each pointed a
# reader at numbers that did not exist. A branch in that state cannot be brought
# to green by anybody.
#
# host_gate.sh answers two questions where there was one: may this host be
# compared, and may it be measured. The mutation collapses them back, which is
# the shape that produced the deadlock.
set -e
gate=scripts/gates/host_gate.sh
if ! grep -q 'GITHUB_ACTIONS' "$gate"; then
  echo "no CI arm in $gate; the shape moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i 's/^if \[ -n "\$GITHUB_ACTIONS" \]; then$/if false; then/' "$gate"
if grep -q 'if \[ -n "\$GITHUB_ACTIONS" \]; then' "$gate"; then
  echo "the CI arm survived the cut" >&2
  exit 1
fi
