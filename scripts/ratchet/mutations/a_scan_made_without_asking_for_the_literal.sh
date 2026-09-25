#!/bin/sh
# The first scan of a subject asks whether the subject holds the literal every
# match must contain, and answers no match without scanning when it does not.
# This mutation never asks, so `[a-z]+zzq` over the run program's split subject
# tries every start again, quadratic in the subject, to find the same nothing.
#
# No output changes. runbench's instructions rise by about sixty million.
set -e
line='  return no_hit if lacks? s prog.must'
grep -qF "$line" lib/regexp/regexp.kso || {
  echo "the literal question changed shape; rewrite this" >&2
  exit 1
}
sed -i '/^  return no_hit if lacks? s prog.must$/d' lib/regexp/regexp.kso
! grep -qF "$line" lib/regexp/regexp.kso
