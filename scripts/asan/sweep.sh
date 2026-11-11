#!/bin/sh
# Every kanso program in the corpus, run under AddressSanitizer.
#
# WHY THIS EXISTS. `src/runtime.c` reclaims memory in bulk — a beat rewinds an
# arena, `k_ten_release` frees a depth's tenure blocks — and the walks that
# decide what survives follow pointers into storage those two can take back.
# The file's own comments carry two accounts of that going wrong: a `KStr` read
# in a munmap'd page, found because the trend gate happened to move; and a node
# below a mark holding a tenured pointer, which `k_repaired_settle` still does
# not move and which is harmless today only because a beat with a heap result
# hands its blocks up instead of freeing them. Both were found by a program
# crashing, which is the expensive way.
#
# A sanitizer finds that class on a run that does not crash. Nothing in this
# tree ran one until 2026-09-22, and the sweep below came back clean — which is
# a fact worth being able to reproduce rather than a fact worth remembering.
#
# WHAT IT REPORTS, one line per program:
#
#   CLEAN   built, ran to its end, no report
#   DIED    no report, and the program did not finish: its output is not
#           its golden's, or it has no golden and exited non-zero. This
#           fails the sweep, because a program that stops at its first
#           call has shown nothing about the memory it never reached
#   KNOWN   the one report this tree expects, named below
#   FOUND   any other AddressSanitizer report — a finding, and the run's
#           stderr is kept
#   SKIP    could not be built or linked here. A fixture that holds
#           definitions beside bare statements is one `kanso play` runs and
#           `kanso build` refuses, and this sweep links what `build` emits,
#           so those are skipped (the work_ahead_* fixtures, today).
#
# THE ONE KNOWN REPORT. `k_b_find2_raw` loads sixteen bytes at a time and takes
# the last load whole even when fewer than sixteen bytes remain, masking the
# surplus afterwards. `k_tail_window` permits that load only when the pointer
# sits at least sixteen bytes below a page boundary, so it cannot reach an
# unmapped page. AddressSanitizer has no way to know that and reports a
# heap-buffer-overflow read; the address it names is the LAST byte of the load,
# not the first, so the report reads as though the access began past the end
# when it began inside the buffer. It is reported as KNOWN rather than filtered
# out, because a filter that hides a frame hides every future finding in it.
#
# No program reported it on 2026-11-11, and the branch stays: what a
# benchmark reads its input into decides whether the load crosses a block's
# end, and that has changed before.
#
# A JOB IN CI (`sanitizer` in .github/workflows/ci.yml) since 2026-11-11.
# Before that it was run by hand, and on that day a hand found it could not
# have reported anything: on clang 19 it built its runtime on the wrong
# calling convention (see below).
#
# WATCHED FAILING by the ratchet. Its row `freed_read` puts a read of one
# byte after the free in `k_ten_release` and wants this script red. To see
# it by hand, run
# scripts/ratchet/mutations/a_tenure_block_read_after_its_free.sh in a
# scratch worktree and sweep that.
#
# WHAT IT CANNOT SEE. The sanitizer watches what malloc hands out. A beat's
# arena is made of such blocks, each holding many values, so a read that
# strays from one value into its neighbour stays inside a live block and is
# not reported.
set -u

root=$(cd "$(dirname "$0")/../.." && pwd)
kanso="$root/target/release/kanso"
work=${KANSO_ASAN_WORK:-/tmp/kanso-asan}
cc=${CC:-clang}

die() { echo "asan sweep: $*" >&2; exit 2; }

[ -x "$kanso" ] || die "no compiler at $kanso — run cargo build --release first"

# A sanitized binary reserves terabytes of address space for its shadow
# before main runs, as private writable mappings, which is what the 6 GiB
# data limit of scripts/bounded.sh counts. Under it every program here dies
# in the sanitizer's own start-up. CI's steps and the ratchet's gates start
# under that limit, so this script lifts it for itself: the soft limit, which
# bounded.sh leaves liftable for this.
ulimit -S -d unlimited 2>/dev/null

# The sanitizer runtime is a separate package from clang, and its absence shows
# up as a LINK error rather than a compile one. Finding that out per program,
# 67 times, is what this check replaces.
probe="$work/probe"
mkdir -p "$probe" || die "cannot make $work"
printf 'int main(void){return 0;}\n' > "$probe/p.c"
if ! "$cc" -fsanitize=address -o "$probe/p" "$probe/p.c" >"$probe/log" 2>&1; then
  echo "asan sweep: REFUSED — $cc cannot link -fsanitize=address here."
  sed -n '1,4p' "$probe/log"
  echo "asan sweep: install the sanitizer runtime (libclang-rt-<version>-dev)."
  exit 3
fi
# And it has to start. A program that cannot is reported per program as a
# sanitizer error, which reads as a finding in every one of them.
if ! "$probe/p" >"$probe/log" 2>&1; then
  echo "asan sweep: REFUSED — a sanitized program does not start here."
  sed -n '1,4p' "$probe/log"
  exit 3
fi

# THE RUNTIME IS BUILT ON THE CONVENTION THE PROGRAM WAS EMITTED FOR. On a
# clang that takes `preserve_none` the compiler emits closures on that
# convention and builds its own runtime with -DKANSO_PRESERVE_NONE
# (`runtime_object` in src/main.rs). A runtime built here without the flag
# links, and then reads every closure's arguments from the wrong registers:
# the program fails with a run-time error of its own long before it reaches
# the memory this sweep is about. So the flag is read off each program's IR,
# and there is a runtime for each.
#
# Clang will not compile a `preserve_none` function with the sanitizer's fake
# stack ("stack realignment in presence of dynamic allocas is not supported
# with this calling convention"), so that runtime is built with
# use-after-return detection off. The heap checks, which are what the
# runtime's arenas and tenure blocks need, are untouched.
convention_of() {
  if grep -q 'preserve_nonecc' "$1"; then
    echo "-DKANSO_PRESERVE_NONE -Werror=unknown-attributes -fsanitize-address-use-after-return=never"
  fi
}
build_runtime() {
  src=$1; out=$2; shift 2
  # -O1 keeps the frames the report names readable; -g gives them line numbers.
  "$cc" -O1 -g -fsanitize=address -mssse3 "$@" -c "$src" -o "$out"
}
# The sanitized runtime for a program's IR, built the first time it is asked
# for. Prints the object's path.
runtime_for() {
  flags=$(convention_of "$1")
  case "$flags" in
    "") obj="$work/runtime_asan.o" ;;
    *) obj="$work/runtime_asan_preserve.o" ;;
  esac
  if [ ! -f "$obj" ]; then
    # shellcheck disable=SC2086
    build_runtime "$root/src/runtime.c" "$obj" $flags || return 1
  fi
  echo "$obj"
}

rm -f "$work/runtime_asan.o" "$work/runtime_asan_preserve.o"

# The dice the goldens were written under, read from the suite that pins
# them: a fixture that rolls prints its golden's bytes only on this seed.
seed=$(sed -n 's/.*env("KANSO_SEED", "\([0-9]*\)").*/\1/p' "$root/tests/golden.rs" | head -1)
[ -n "$seed" ] || die "tests/golden.rs no longer names the seed its goldens use"

found=0
sweep_one() {
  label=$1; dir=$2; entry=$3
  ( cd "$dir" && "$kanso" build "$entry" ) >"$dir/build.log" 2>&1 || { echo "SKIP  $label (build)"; return; }
  ll=$(ls "$dir"/*.ll 2>/dev/null | head -1)
  [ -n "$ll" ] || { echo "SKIP  $label (no ir)"; return; }
  runtime_obj=$(runtime_for "$ll") || die "the runtime does not compile under the sanitizer"
  "$cc" -O0 -Wno-override-module -fsanitize=address -o "$dir/bin" "$ll" "$runtime_obj" -lm >>"$dir/build.log" 2>&1 \
    || { echo "SKIP  $label (link)"; return; }
  # FROM THE REPOSITORY ROOT, because the benchmarks open their input by a path
  # relative to it. Run from the staging directory they die on a missing file,
  # and a sweep that reports every benchmark as SKIP looks much like a sweep
  # that found nothing.
  ( cd "$root" && KANSO_SEED=$seed \
    ASAN_OPTIONS=detect_leaks=0:fast_unwind_on_fatal=1 "$dir/bin" ) >"$dir/out" 2>"$dir/err"
  status=$?
  if ! grep -q 'ERROR: AddressSanitizer' "$dir/err"; then
    # The golden beside a fixture says what a whole run prints. A program
    # that stopped early touched less memory than the sweep is claiming for
    # it, so its output is compared before it is called clean.
    want="$root/tests/golden/mem/${label#mem/}.stdout"
    if [ "$status" -ne 0 ] && [ ! -f "$want" ]; then
      echo "DIED  $label (exit $status)"
      sed -n '1,3p' "$dir/err"
      found=1
    elif [ -f "$want" ] && ! cmp -s "$want" "$dir/out"; then
      echo "DIED  $label (its output is not its golden's)"
      sed -n '1,3p' "$dir/err"
      found=1
    else
      echo "CLEAN $label"
    fi
  elif grep -q 'in k_b_find2_raw' "$dir/err"; then
    echo "KNOWN $label (the guarded sixteen-byte tail read)"
  else
    echo "FOUND $label"
    sed -n 's/^ *//; /#0 /{p;q}' "$dir/err"
    echo "      stderr kept at $dir/err"
    found=1
  fi
}

for prog in "$root"/tests/golden/mem/*.kso; do
  name=$(basename "$prog" .kso)
  d="$work/mem/$name"; rm -rf "$d"; mkdir -p "$d"
  cp "$root"/tests/golden/mem/*.kso "$d"/ 2>/dev/null
  if grep -q '^pub play' "$prog"; then
    printf 'import "./%s"\n\n%s/play\n' "$name" "$name" > "$d/run.kso"
    sweep_one "mem/$name" "$d" run.kso
  else
    sweep_one "mem/$name" "$d" "$name.kso"
  fi
done

for b in "$root"/bench/*/; do
  [ -f "$b/main.kso" ] || continue
  name=$(basename "$b")
  # The corpora are inputs to the compile-side gates, not programs with a main
  # worth running here.
  case "$name" in *_corpus) continue ;; esac
  d="$work/bench/$name"; rm -rf "$d"; mkdir -p "$(dirname "$d")"
  # -L: runbench carries the other benchmarks' shapes as links to their
  # files, and a link copied as a link points nowhere from here.
  cp -rL "$b" "$d" || continue
  sweep_one "bench/$name" "$d" main.kso
done

d="$work/ratchet"; rm -rf "$d"; mkdir -p "$work"
cp -r "$root/scripts/ratchet" "$d" && sweep_one "scripts/ratchet" "$d" main.kso

if [ "$found" -ne 0 ]; then
  echo "asan sweep: a report the tree does not expect — read it."
  exit 1
fi
echo "asan sweep: nothing beyond the known tail read."
