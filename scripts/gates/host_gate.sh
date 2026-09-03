#!/bin/sh
# Whether this host may be COMPARED against a golden, and — when it may not —
# whether it should still be MEASURED.
#
# scripts/gates/measured_on.sh refuses a host the golden does not name, and its
# refusal says what to do next: "let CI measure it and copy the rows out of the
# job log." That instruction could not be followed. Every compile gate called
# measured_on under `set -e`, so a mismatch stopped the gate before it measured
# anything, and the job log carried no rows to copy. On 2026-09-03 the runner's
# rustc moved 1.98.0 -> 1.98.1 and all three compile veins refused at once,
# each printing a sentence telling a reader to read numbers that were never
# produced.
#
# The refusal itself is right and stays. What was missing is the distinction
# between the two hosts that can hit it:
#
#   a CONTAINER may not measure, because a container's numbers going into a
#   golden over the runner's is the exact accident measured_on was written
#   after. It prints nothing. There is nothing to paste.
#
#   CI may measure, because CI's numbers are the only ones that may ever be
#   recorded. It measures, prints, and still FAILS — nothing is compared.
#
# `dispatch.sh` already draws this line the same way and for the same reason,
# printing its feature block only under GITHUB_ACTIONS.
#
#   sh scripts/gates/host_gate.sh <golden>
#
#   0  the host matches: compare as usual
#   1  the host does not match and may not measure: stop here
#   3  the host does not match and this is CI: measure, print, then fail
set -e
golden=$1
if [ -z "$golden" ]; then
  echo "::error::this wants the golden to check as its argument"
  exit 2
fi

said=0
sh scripts/gates/measured_on.sh "$golden" || said=$?

if [ "$said" -eq 0 ]; then
  exit 0
fi

# A usage error stays a usage error; only a host mismatch is 1.
if [ "$said" -ne 1 ]; then
  exit "$said"
fi

if [ -n "$GITHUB_ACTIONS" ]; then
  echo "::error::Measuring anyway, because this is CI and CI's sitting is the"
  echo "::error::only one that may ever be recorded. The rows below are this"
  echo "::error::runner's, on a host $golden does not name. NOTHING IS"
  echo "::error::COMPARED and this gate still fails."
  exit 3
fi

exit 1
