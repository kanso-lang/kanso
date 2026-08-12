#!/bin/sh
# The dimension none of the veins beside it can see. Allocation counters say
# how often the allocator ran, the emitted golden how many lines were written,
# the text golden how big the code is — a decode that allocates identically
# and executes eight per cent more instructions moves none of them, which is
# exactly what happened over eleven days in August. Callgrind counts rather
# than samples, so three runs give the same digits and no other process on the
# box can reach it. Forty-four seconds for all four.
#
# The environment is emptied because the kernel copies it onto the new
# process's stack and libc walks it before main, so a run id that gained a
# digit reads as fourteen instructions of work that nobody wrote.
set -e
for b in jsonbench encodebench oneshot basket; do
  env -i PATH=/usr/bin:/bin \
    valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.$b ./$b \
    >/dev/null 2>/tmp/ir.$b
  printf '%s %s\n' "$b" "$(grep -o 'I   refs:.*' /tmp/ir.$b | tr -dc 0-9)"
done > work.txt
grep -v '^#' bench/instructions_golden.txt > work_want.txt
diff work_want.txt work.txt || {
  echo "::error::the work the benchmarks do changed. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank —"
  echo "::error::say which in the PR and regenerate"
  echo "::error::bench/instructions_golden.txt. This is the counter"
  echo "::error::that would have caught the 8.5% decode regression"
  echo "::error::that every allocation counter slept through."
  exit 1
}
