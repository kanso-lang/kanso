#!/bin/sh
# The gates neither `all_counters.sh` nor `all_compile.sh` reads.
#
# Between them those two sweeps look like the whole board, and on 2026-09-18
# they were not. `all_counters.sh` reported that every cost vein and the lazy
# tier agreed with its golden, on a tree whose interpreted row had moved
# 18,540,440 instructions, because `interp_instructions` is in neither list.
# The change was deliberate and CI measured it, so nothing was lost that day --
# but a sweep that names all but four looks like coverage, which is the reason
# `all_compile.sh` exists and the reason this does.
#
# This sweep runs what is left once the runtime cost veins and the compile
# veins are taken out, and `--list` prints it. On the day above that was four
# gates:
#
#   instructions           bench/instructions_golden.txt
#                          retired instructions per benchmark -- the dimension
#                          every allocation counter is blind to
#   interp_instructions    bench/interp_instructions_golden.txt
#   interp_memory          bench/interp_memory_golden.txt
#   startup_instructions   bench/startup_instructions_golden.txt
#
# `browser_cost` joined them on 2026-11-11. It had been in no sweep since it
# was written, and the spec that should have said so did not count it as
# reading a golden (kanso#2447).
#
# THE FIRST FOUR REFUSE ON A CONTAINER whose glibc or rustc does not match
# their measured-on line, and on 2026-09-18 all four refused here. That is not a
# reason to leave them out; it is the reason to put them in. A refusal printed
# as REFUSED tells a session these rows are unchecked and CI will measure them.
# Silence tells it the same rows agreed.
#
# It is not a gate. CI runs each of these directly, as its own step with its own
# row in the cost-goldens summary, and that is what fails a pull request. This
# exists so a container can tell a moved vein from an unmeasurable one before
# pushing. The verdicts are `all_compile.sh`'s, for the same reasons it gives.
set -e
# The gates are read from scripts/gates rather than listed, as all_compile.sh
# reads its own: a script that names a golden under bench/ and that neither
# of the other two sweeps runs is this sweep's, so a gate added later is
# swept without anyone remembering to add it. The counter sweep runs every
# `*_counters` gate and the compile sweep prints its own list.
# `tests/every_golden_is_swept_by_something.rs` holds the three together
# against what CI runs.
names_a_golden='bench/[a-z_]*golden[a-z_]*\.txt'
compile_side=" $(sh scripts/gates/all_compile.sh --list | tr '\n' ' ') "
gates=""
for script in scripts/gates/*.sh; do
  g=$(basename "$script" .sh)
  case "$g" in all_*|build_benchmarks|*_counters) continue ;; esac
  case "$compile_side" in *" $g "*) continue ;; esac
  if grep -qE "$names_a_golden" "$script"; then
    gates="$gates $g"
  fi
done

# `--list` prints the gates and stops, without building anything.
if [ "$1" = "--list" ]; then
  for g in $gates; do
    echo "$g"
  done
  exit 0
fi

# BUILD FIRST, for the reason `all_compile.sh` gives at length: these gates read
# a built compiler and staged benchmarks out of the working directory and none
# of them produces those files. `build_benchmarks.sh` opens with the release
# build, which is also what makes a `lib/` edit reach a program at all.
sh scripts/gates/build_benchmarks.sh > /dev/null

moved=""
refused=""
for g in $gates; do
  out=$(sh "scripts/gates/$g.sh" 2>&1) && verdict=AGREED || verdict=FAILED
  if [ "$verdict" = FAILED ]; then
    if echo "$out" | grep -q 'so the two cannot be compared'; then
      verdict=REFUSED
      refused="$refused $g"
    else
      verdict=MOVED
      moved="$moved $g"
    fi
  fi
  printf '%-22s %s\n' "$g" "$verdict"
  if [ "$verdict" = MOVED ]; then
    echo "$out" | sed 's/^/    /'
  fi
done

echo
if [ -n "$refused" ]; then
  echo "not compared here (CI measures these):$refused"
fi
if [ -n "$moved" ]; then
  echo "interpreted veins moved:$moved"
  echo "say which way each went in design/compiler-log.md, and copy CI's rows"
  echo "into the goldens rather than regenerating them on a host that refused"
  exit 1
fi
echo "interpreted veins: nothing moved that this host can see"
