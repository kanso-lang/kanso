#!/bin/sh
# What an INTERPRETED run holds and how much traffic it makes.
#
# `kanso run --interp` is a deployment rather than a stage of one: the program
# is parsed, inferred and then executed in the same process, and nothing is
# emitted. So these two rows cover the front end and the interpreter together,
# which is what an interpreted run actually costs, and the compile veins beside
# them cannot see either number -- they run `kanso check`, which stops before
# anything is executed at all.
#
# WHY IT EXISTS. Clay ruled on 2026-09-16 that the objective becomes a
# development welfare, a production welfare and a meta over them, and he named
# the interpreted engine's own order: "start-time is vastly more important than
# speed which is more important than memory usage." Start-up is counted by
# startup_instructions.sh, speed by interp_instructions.sh, and this is the
# third. It is an exact vein of its own and not yet an objective term, the way
# `.text` is under the 2026-09-05 ruling; the objective takes it when the model
# splits.
#
# BOTH ROWS BELONG TO THE RUSTC THAT BUILT THE COMPILER, the way
# compile_peak_bytes does: a HashMap's growth schedule and a Vec's doubling are
# the standard library's, and the interpreter is Rust. So the file is exact for
# the host its measured-on line names and any other host refuses to compare
# rather than fuzzing. Clay ruled tolerance bands away on 2026-08-24.
set -e
golden=bench/interp_memory_golden.txt
# A host the golden does not name still MEASURES on CI, so the job log carries
# the sitting the refusal tells a reader to copy. scripts/gates/host_gate.sh
# carries the reasons; 3 means measure, print, and fail at the end.
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi
sh scripts/gates/library_box.sh
(cd /tmp/kanso-compile-ir && KANSO_COUNTERS=1 ./kanso run interp_corpus --interp 2>&1 >/dev/null) \
  > counters_interp.txt
grep -v '^#' "$golden" > interp_mem_want.txt
for k in interp_allocs interp_peak_bytes; do
  grep "^${k}=" counters_interp.txt
done > interp_mem_got.txt

# Measured, and on a host the golden does not name that is as far as this goes:
# the rows are printed for CI's job log and nothing is compared against them.
if [ "$host" -eq 3 ]; then
  echo "::error::this runner's sitting, to copy into $golden together with its"
  echo "::error::measured-on line. NOTHING IS COMPARED:"
  sed 's/^/::error::    /' interp_mem_got.txt
  exit 1
fi

diff interp_mem_want.txt interp_mem_got.txt || {
  echo "::error::the interpreted run's memory moved. A rise is a regression to"
  echo "::error::explain and a fall is a win to bank -- say which in"
  echo "::error::design/compiler-log.md and regenerate $golden."
  echo "::error::"
  echo "::error::The two rows are different dimensions and move independently:"
  echo "::error::interp_allocs is TRAFFIC, what the run asked the allocator for"
  echo "::error::and gave back, and interp_peak_bytes is RESIDENCY, the most it"
  echo "::error::held at once. A thunk allocated and forced inside one beat"
  echo "::error::moves the first and not the second."
  exit 1
}
