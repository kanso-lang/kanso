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

# Whose numbers these are, before spending a minute measuring against them.
# A host the golden does not name still MEASURES on CI, so the job log carries
# the sitting the refusal tells a reader to copy. scripts/gates/host_gate.sh
# carries the reasons; 3 means measure, print, and refuse without comparing.
host=0
sh scripts/gates/host_gate.sh bench/instructions_golden.txt || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

# And which silicon is about to count them. This never refuses — the runner
# pool holds at least four CPUs, so a check that demanded one would be red on
# most runs for a reason no pull request causes. It prints, and it is asked a
# question further down, only if a row has actually moved.
sh scripts/gates/dispatch.sh name

# digestbench joined on 2026-09-01. Its peak is the row welfare weighs, and a
# term counted on one axis and not the other is a trade the index cannot see:
# every change that buys the digest's memory spends something here, and this is
# where that something has to show.
#
# scanbench joined on 2026-09-02, and it was the last of the eleven this gate
# could not see. Its two memory counters are in the objective and its work was
# in nothing, which is the exact asymmetry the digest paragraph above warns
# about: a change that gives the arena back per position and spends ten times
# the instructions doing it scored zero everywhere. Twelve seconds under
# callgrind.
for b in jsonbench encodebench oneshot basket widebench deepbench escapebench pendbench \
         indexbench scanbench digestbench; do
  env -i PATH=/usr/bin:/bin \
    valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.$b ./$b \
    >/dev/null 2>/tmp/ir.$b
  printf '%s %s\n' "$b" "$(grep -o 'I   refs:.*' /tmp/ir.$b | tr -dc 0-9)"
done > work.txt
# The profile is already on disk — the loop above threw away everything but the
# total. Where the work sits is the question every one of these regressions has
# turned on, and it cost a bespoke run to answer each time. It costs nothing to
# print it here.
# Printed, not only summarised. A step summary cannot be read back from the
# job log or the API, so a diagnostic that only goes there is one nobody can
# fetch afterwards.
if command -v callgrind_annotate >/dev/null; then
  for b in jsonbench oneshot; do
    echo "=== where the work is: $b"
    callgrind_annotate --threshold=90 /tmp/cg.$b 2>&1 | head -40
  done
else
  echo "=== no callgrind_annotate on this host, so no breakdown"
fi
# Measured, and on a host the golden does not name that is as far as this goes.
if [ "$host" -eq 3 ]; then
  echo "::error::this runner's sitting, to copy into bench/instructions_golden.txt"
  echo "::error::together with its measured-on line. NOTHING IS COMPARED:"
  sed 's/^/::error::    /' work.txt
  exit 1
fi

grep -v '^#' bench/instructions_golden.txt > work_want.txt
diff work_want.txt work.txt || {
  # The rows again, after the diff and after the profile above it, because
  # this file is pinned to a host and the sessions that have to regenerate it
  # are on a different one — they read the numbers out of this log and copy
  # them in. A diff eighty lines of callgrind output above the end of a step
  # is a diagnostic the log API will not hand back, which is the same
  # complaint the note above makes about step summaries.
  echo "=== every row as measured here, to copy into the golden"
  cat work.txt
  # A row moved. glibc dispatches memcpy and its neighbours by CPU feature and
  # this pool holds at least four, so silicon is a live explanation — but it is
  # an explanation printed BESIDE the failure, never instead of it. An earlier
  # shape of this exited 0 here, and that was wrong twice over: three runs in
  # four land on a cpu that is not the recorded one, so most real regressions
  # would have been waved through; and the ratchet's own mutations redden this
  # gate, so on those runs the rows would have gone BLIND — the one thing the
  # ratchet exists to prevent. The diff below says whether silicon could
  # account for it. Deciding that it does is a person's job, in the pull
  # request, with the re-run that lands on the recorded cpu as the evidence.
  sh scripts/gates/dispatch.sh differs bench/dispatch.txt && silicon=0 \
    || silicon=$?
  if [ "$silicon" -eq 1 ]; then
    echo "::error::and this is NOT the silicon these rows were counted on, so"
    echo "::error::the dispatch above may account for some of the move. It does"
    echo "::error::not excuse it: re-run until the job lands on the recorded"
    echo "::error::cpu, and say in the pull request which way it went."
  fi
  echo "::error::the work the benchmarks do changed. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank —"
  echo "::error::say which in the PR and regenerate"
  echo "::error::bench/instructions_golden.txt. This is the counter"
  echo "::error::that would have caught the 8.5% decode regression"
  echo "::error::that every allocation counter slept through."
  exit 1
}
