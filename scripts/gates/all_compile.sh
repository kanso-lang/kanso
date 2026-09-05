#!/bin/sh
# The gates `all_counters.sh` does not read.
#
# That sweep walks the runtime cost goldens. The compile side is its own set of
# gates, two of their counters are welfare terms, and a change under lib/ moves
# every one of them — `lib/*.kso` is `include_str!`'d into the compiler, so a
# line added to the library is a line the compiler carries and compiles. On
# 2026-09-05 a twelve-line library change read as a welfare RISE with these
# veins stale and a FALL once they were regenerated, and nothing in the sweep
# beside this one would have said so.
#
# Running them by hand is what this replaces, and the reason is not typing:
# three of them REFUSE on a container whose glibc or rustc does not match the
# golden's measured-on line, and a refusal exits non-zero exactly like a
# regression. A session that runs them raw and sees three failures learns
# nothing it can act on, which is the same as not running them. So this
# separates the two:
#
#   AGREED    the gate compared and the numbers match
#   MOVED     the gate compared and a number changed — read it, then say
#             which way it went in design/compiler-log.md
#   REFUSED   this host may not compare; CI measures and prints, and the
#             rows are copied out of its job log
#
# It is not a gate. CI runs them directly, each as its own step with its own
# row in the summary, and that is what fails a pull request. This exists so
# a container can tell a moved vein from an unmeasurable one before pushing.
set -e
# Derived from the goldens rather than remembered: these are the gates whose
# script names a compile-side golden (bench/compile_*_golden, text_golden,
# emitted_golden). `tests/the_compile_sweep_names_every_compile_gate.rs`
# replays that derivation, so a gate added later is a red spec rather than a
# vein nobody sweeps.
#
# TWO ARE DELIBERATELY OUT. `compile_ir_row` reads the same table but is not a
# gate — it takes four arguments and `compile_instructions` calls it, split out
# so its refusals could be watched. `build_benchmarks` is not a gate either and
# says so in its own first line.
gates="machine_code emitted_code compile_memory compile_allocs compile_instructions compile_libraries"
moved=""
refused=""
for g in $gates; do
  out=$(sh "scripts/gates/$g.sh" 2>&1) && verdict=AGREED || verdict=FAILED
  if [ "$verdict" = FAILED ]; then
    # The host gate's own refusal names itself in one sentence, and it is the
    # only thing in this tree that prints it. A gate that moved AND cannot
    # compare exits at the move first, before the host gate runs, so a real
    # diff never carries this line.
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
  echo "compile veins moved:$moved"
  echo "regenerate them and say which way each went in design/compiler-log.md"
  exit 1
fi
echo "compile veins: nothing moved that this host can see"
