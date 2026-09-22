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
#   CLEAN   built, ran, no report
#   KNOWN   the one report this tree expects, named below
#   FOUND   any other AddressSanitizer report — a finding, and the run's
#           stderr is kept
#   SKIP    could not be built or linked here
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
# NOT A CI GATE, and the reason is the sanitizer runtime rather than the time:
# the runners would each need `libclang-rt-*-dev`, and the KNOWN report above
# would have to become a suppression maintained in two places. Run it by hand
# when a memory question comes up, which is what the runtime's comments keep
# asking for.
#
# PROVE IT FIRST. `sh scripts/asan/sweep.sh --prove` patches a deliberate
# read-after-free into a COPY of src/runtime.c, builds one program against it,
# and asserts the sanitizer says so. A clean sweep whose instrument was never
# watched fail is a green suite that proves nothing, and this repository has
# caught more than one of those.
set -u

root=$(cd "$(dirname "$0")/../.." && pwd)
kanso="$root/target/release/kanso"
work=${KANSO_ASAN_WORK:-/tmp/kanso-asan}
cc=${CC:-clang}

die() { echo "asan sweep: $*" >&2; exit 2; }

[ -x "$kanso" ] || die "no compiler at $kanso — run cargo build --release first"

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

runtime_obj="$work/runtime_asan.o"
build_runtime() {
  src=$1; out=$2
  # -O1 keeps the frames the report names readable; -g gives them line numbers.
  "$cc" -O1 -g -fsanitize=address -mssse3 -c "$src" -o "$out"
}

# --prove: the instrument, watched failing, before any clean result is believed.
if [ "${1:-}" = "--prove" ]; then
  broken_c="$work/runtime_broken.c"
  mkdir -p "$work"
  # A read of one byte from a block this function has just freed. It sits in
  # `k_ten_release`, so any program that frees a tenure block trips it.
  awk '
    { print }
    /^        free\(b->data\);$/ && !done {
      print "        { volatile char probe = ((const char*)b->data)[0]; (void)probe; }"
      done = 1
    }
  ' "$root/src/runtime.c" > "$broken_c"
  if ! grep -q 'volatile char probe' "$broken_c"; then
    die "--prove could not find the free in k_ten_release to patch"
  fi
  build_runtime "$broken_c" "$work/runtime_broken.o" || die "--prove: the patched runtime does not compile"
  # A fixture that frees a tenure block. Its .mem golden pins ten_frees=1.
  prog=$root/tests/golden/mem/a_repaired_node_below_the_mark_holds_tenure.kso
  d="$work/prove"; rm -rf "$d"; mkdir -p "$d"
  cp "$root"/tests/golden/mem/*.kso "$d"/ 2>/dev/null
  name=$(basename "$prog" .kso)
  printf 'import "./%s"\n\n%s/play\n' "$name" "$name" > "$d/run.kso"
  ( cd "$d" && "$kanso" build run.kso ) >"$d/build.log" 2>&1 || die "--prove: the fixture does not build"
  "$cc" -O0 -Wno-override-module -fsanitize=address -o "$d/bin" "$d/run.ll" "$work/runtime_broken.o" -lm >>"$d/build.log" 2>&1 \
    || die "--prove: the fixture does not link"
  ( cd "$root" && ASAN_OPTIONS=detect_leaks=0:fast_unwind_on_fatal=1 "$d/bin" ) >/dev/null 2>"$d/err"
  if grep -q 'ERROR: AddressSanitizer' "$d/err" && grep -q 'k_ten_release' "$d/err"; then
    echo "asan sweep: PROVEN — a read after free in k_ten_release is reported:"
    sed -n 's/^ *//; /#0 /{p;q}' "$d/err"
    exit 0
  fi
  echo "asan sweep: NOT PROVEN — the deliberate read after free was not reported." >&2
  echo "Until it is, a clean sweep says nothing. The run's stderr:" >&2
  sed -n '1,10p' "$d/err" >&2
  exit 1
fi

build_runtime "$root/src/runtime.c" "$runtime_obj" || die "the runtime does not compile under the sanitizer"

found=0
sweep_one() {
  label=$1; dir=$2; entry=$3
  ( cd "$dir" && "$kanso" build "$entry" ) >"$dir/build.log" 2>&1 || { echo "SKIP  $label (build)"; return; }
  ll=$(ls "$dir"/*.ll 2>/dev/null | head -1)
  [ -n "$ll" ] || { echo "SKIP  $label (no ir)"; return; }
  "$cc" -O0 -Wno-override-module -fsanitize=address -o "$dir/bin" "$ll" "$runtime_obj" -lm >>"$dir/build.log" 2>&1 \
    || { echo "SKIP  $label (link)"; return; }
  # FROM THE REPOSITORY ROOT, because the benchmarks open their input by a path
  # relative to it. Run from the staging directory they die on a missing file,
  # and a sweep that reports every benchmark as SKIP looks much like a sweep
  # that found nothing.
  ( cd "$root" && ASAN_OPTIONS=detect_leaks=0:fast_unwind_on_fatal=1 "$dir/bin" ) >"$dir/out" 2>"$dir/err"
  if ! grep -q 'ERROR: AddressSanitizer' "$dir/err"; then
    echo "CLEAN $label"
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
  cp -r "$b" "$d" || continue
  sweep_one "bench/$name" "$d" main.kso
done

d="$work/ratchet"; rm -rf "$d"; mkdir -p "$work"
cp -r "$root/scripts/ratchet" "$d" && sweep_one "scripts/ratchet" "$d" main.kso

if [ "$found" -ne 0 ]; then
  echo "asan sweep: a report the tree does not expect — read it."
  exit 1
fi
echo "asan sweep: nothing beyond the known tail read."
