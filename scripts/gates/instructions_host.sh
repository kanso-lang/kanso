#!/bin/sh
# The instructions vein's rows belong to the host that measured them, and until
# this existed nothing said so except prose. The golden's header has always
# warned that a row must never be read against a number measured somewhere
# else; the warning did not stop this branch pasting a container's numbers over
# the runner's, because what was in front of it was a diff, and a diff invites
# a paste.
#
# Two ubuntu 24.04 boxes one glibc revision apart — 2.39-0ubuntu8.7 against
# 2.39-0ubuntu8.8 — disagree by about four hundred retired instructions before
# main, and by a few thousand where memcpy carries the work. That is larger
# than most of what this vein exists to catch.
#
# So the golden names its host and this compares. scripts/gates/instructions.sh
# runs it before anything else, so off the runner the refusal costs
# milliseconds and prints no numbers at all — there is nothing to copy.
#
# glibc alone, not the valgrind version: callgrind's version moves the whole
# vein at once the way any toolchain bump does, and pinning it here would make
# this unprovable on a host that has no valgrind to ask.
set -e
glibc=$(ldd --version 2>/dev/null | head -1 | sed -n 's/.*GLIBC \([^)]*\)).*/\1/p')
if [ -z "$glibc" ]; then
  glibc=$(ldd --version 2>/dev/null | head -1 | awk '{print $NF}')
fi
have="glibc=${glibc:-unknown}"
want=$(sed -n 's/^# measured-on //p' bench/instructions_golden.txt)
echo "instructions vein: measured-on $want; here $have"
if [ "$want" != "$have" ]; then
  echo "::error::this vein's rows were measured on $want and this host is"
  echo "::error::$have, so the two cannot be compared. Do not regenerate"
  echo "::error::bench/instructions_golden.txt from here — let CI measure it"
  echo "::error::and copy the rows out of the job log. If the runner image"
  echo "::error::itself moved, every row moves with it and none has"
  echo "::error::regressed: regenerate all eight in one go, update the"
  echo "::error::measured-on line, and say so in design/compiler-log.md."
  exit 1
fi
