#!/bin/sh
# The gates neither `all_counters.sh` nor `all_compile.sh` reads.
#
# Between them those two sweeps look like the whole board, and on 2026-09-18
# they were not. `all_counters.sh` reported "the twelve cost veins and the lazy
# tier agree with their goldens" on a tree whose interpreted row had moved
# 18,540,440 instructions, because `interp_instructions` is in neither list.
# The change was deliberate and CI measured it, so nothing was lost that day --
# but a sweep that names all but four looks like coverage, which is the reason
# `all_compile.sh` exists and the reason this does.
#
# The four are what is left once the runtime cost veins and the compile veins
# are taken out:
#
#   instructions           bench/instructions_golden.txt
#                          retired instructions per benchmark -- the dimension
#                          every allocation counter is blind to
#   interp_instructions    bench/interp_instructions_golden.txt
#   interp_memory          bench/interp_memory_golden.txt
#   startup_instructions   bench/startup_instructions_golden.txt
#
# ALL FOUR REFUSE ON A CONTAINER whose glibc or rustc does not match their
# measured-on line, and on 2026-09-18 all four refused here. That is not a
# reason to leave them out; it is the reason to put them in. A refusal printed
# as REFUSED tells a session these rows are unchecked and CI will measure them.
# Silence tells it the same rows agreed.
#
# It is not a gate. CI runs each of these directly, as its own step with its own
# row in the cost-goldens summary, and that is what fails a pull request. This
# exists so a container can tell a moved vein from an unmeasurable one before
# pushing. The verdicts are `all_compile.sh`'s, for the same reasons it gives.
set -e
# BUILD FIRST, for the reason `all_compile.sh` gives at length: these gates read
# a built compiler and staged benchmarks out of the working directory and none
# of them produces those files. `build_benchmarks.sh` opens with the release
# build, which is also what makes a `lib/*.kso` edit reach a program at all.
sh scripts/gates/build_benchmarks.sh > /dev/null

# Derived rather than remembered, and pinned by
# `tests/every_golden_is_swept_by_something.rs`: a gate script that names a
# golden under bench/ belongs to exactly one of the three sweeps, and a gate
# added later is a red spec rather than a vein nobody sweeps.
gates="instructions interp_instructions interp_memory startup_instructions"
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
