#!/bin/sh
# The decode counters say what the program ALLOCATES. This says what the
# compiler WROTE for it, counted from the IR before the linker runs —
# deterministic, and blind to code layout in a way no timing can be.
#
# It exists because 7.6% of decode speed leaked away between 2026-07-27 and
# 2026-08-07 with every allocation counter byte-identical: the decoder gained
# 20% more calls and 23% more branches for the same work, one or two per cent
# at a time, and nothing was watching the dimension that moved. A diffuse
# regression can only be caught where it happens.
set -e
{
  grep -c '^define' jsonbench.ll | sed 's/^/defines=/'
  grep -c 'call ' jsonbench.ll | sed 's/^/calls=/'
  grep -c '^  br \|^  switch' jsonbench.ll | sed 's/^/branches=/'
  wc -l < jsonbench.ll | tr -d ' ' | sed 's/^/lines=/'
} > emitted.txt
grep -v '^#' bench/emitted_golden.txt > emitted_want.txt
diff emitted_want.txt emitted.txt || {
  echo "::error::the decoder's emitted code moved. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank —"
  echo "::error::say which in the PR and regenerate"
  echo "::error::bench/emitted_golden.txt. Allocation counters cannot"
  echo "::error::see this: 7.6% of decode speed leaked away with all"
  echo "::error::of them flat."
  exit 1
}
