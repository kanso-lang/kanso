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
# BUILD FIRST. The gates below read `*.ll` and the linked binaries out of the
# working directory, and nothing in them produces those files -- so a sweep run
# without this line reads whatever the last build happened to leave behind. On
# 2026-09-06 that was artifacts from a patched tree three minutes stale, and the
# sweep reported `machine_code` and `emitted_code` MOVED on a tree identical to
# HEAD. The false positive is the harmless direction. The same staleness the
# other way round -- artifacts older than the source -- is a sweep that says
# nothing moved after an edit that moved a vein, which is the failure this whole
# script exists to prevent.
#
# `all_counters.sh` opens with `cargo build --release` for the same reason and
# CLAUDE.md says why: `lib/*.kso` is `include_str!`'d into the compiler, so a
# library edit does not reach a program until the compiler is rebuilt.
# `build_benchmarks.sh` starts with that build and then compiles all thirteen.
sh scripts/gates/build_benchmarks.sh > /dev/null

# Derived from the goldens rather than remembered: these are the gates whose
# script names a compile-side golden (bench/compile_*_golden,
# bench/entry_*_golden, text_golden, emitted_golden).
# `tests/the_compile_sweep_names_every_compile_gate.rs`
# replays that derivation, so a gate added later is a red spec rather than a
# vein nobody sweeps.
#
# ONE IS DELIBERATELY OUT. `build_benchmarks` is not a gate and says so in its
# own first line. `compile_ir_row` used to be the second exclusion; it went with
# the per-chip table on 2026-09-05, and the compile row now compares against its
# single golden inside `compile_instructions` itself.
gates="machine_code emitted_code compile_memory compile_allocs compile_instructions entry_instructions compile_libraries"
moved=""
refused=""
regen=""
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

# NOT DERIVED FROM THE LINE ABOVE, and it has to be named by hand because the
# derivation cannot see it: `bench/compile_golden.txt` and
# `bench/compile_golden_modules.txt` are read by a cargo test rather than by a
# gate script, so no file under scripts/gates mentions them and both the
# `gates=` line and the spec that replays it walk straight past. That is the
# hole b3024fb9 fell through — a library change moved the modules vein, this
# sweep said nothing moved, and CI found it a round later. The widened spec in
# `tests/the_compile_sweep_names_every_compile_gate.rs` now pins the property
# that matters: every compile-side golden on disk is read by something this
# sweep runs.
#
# It compares on any host. The rows are counter values the compiler computes,
# not measurements of the machine it runs on, so there is no REFUSED case here.
printf '%-22s ' compile_cost
if out=$(cargo test --release --test compile_cost 2>&1); then
  echo AGREED
else
  echo MOVED
  echo "$out" | sed 's/^/    /'
  moved="$moved compile_cost"
  regen="KANSO_REGEN_COMPILE_GOLDEN=1 cargo test --test compile_cost"
fi

echo
if [ -n "$refused" ]; then
  echo "not compared here (CI measures these):$refused"
fi
if [ -n "$moved" ]; then
  echo "compile veins moved:$moved"
  echo "regenerate them and say which way each went in design/compiler-log.md"
  if [ -n "$regen" ]; then
    echo "compile_cost regenerates with: $regen"
  fi
  exit 1
fi
echo "compile veins: nothing moved that this host can see"
