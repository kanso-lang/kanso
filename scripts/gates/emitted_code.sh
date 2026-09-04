#!/bin/sh
# The cost counters say what a program ALLOCATES. This says what the compiler
# WROTE for it, counted from the IR before the linker runs — deterministic,
# and blind to code layout in a way no timing can be. Nine programs: the
# decoder in one golden, the eight built beside it in another.
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

# The other eight programs the same build step produces. Each has a cost
# golden of its own and none was watched on this dimension, so the leak this
# gate was built to catch could have happened in any of them unseen. They sit
# in a second file so the decoder's history — which its comments are — stays
# one series.
for b in encodebench oneshot basket widebench deepbench escapebench pendbench scanbench \
         indexbench digestbench readbench; do
  printf '%s defines=%s calls=%s branches=%s lines=%s\n' "$b" \
    "$(grep -c '^define' $b.ll)" "$(grep -c 'call ' $b.ll)" \
    "$(grep -c '^  br \|^  switch' $b.ll)" "$(wc -l < $b.ll | tr -d ' ')"
done > emitted_others.txt
grep -v '^#' bench/emitted_golden_others.txt > emitted_others_want.txt

# Both diffs run before either exits. They are independent programs, and
# stopping at the first hides the rest — the same reason the CI job runs each
# vein with continue-on-error.
moved=0
diff emitted_want.txt emitted.txt || {
  echo "::error::the decoder's emitted code moved. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank —"
  echo "::error::say which in the PR and regenerate"
  echo "::error::bench/emitted_golden.txt. Allocation counters cannot"
  echo "::error::see this: 7.6% of decode speed leaked away with all"
  echo "::error::of them flat."
  moved=1
}
diff emitted_others_want.txt emitted_others.txt || {
  echo "::error::a benchmark's emitted code moved. The line above the"
  echo "::error::diff names the program. A rise is a regression to"
  echo "::error::explain and a fall is a win to bank — say which in the"
  echo "::error::PR and regenerate bench/emitted_golden_others.txt."
  moved=1
}
[ "$moved" = 0 ] || exit 1
