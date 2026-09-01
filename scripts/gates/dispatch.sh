#!/bin/sh
# Which silicon counted a row, for the veins that count instructions.
#
# glibc resolves memcpy, memcmp, strlen and their neighbours by ifunc at load
# time, reading CPU features, so one libc runs different code on different
# CPUs. kq#85 found the shape: four instruction rows moved 0.06% to 0.10%
# between two runs whose every printed version — rustc, LLVM, runner image,
# glibc, valgrind, gdb — agreed to the commit hash, and the only difference
# was the Azure region. Measured on an Intel host, feeding it one switch that
# differs on an AMD runner (rep_movsb_threshold 0x2000 against 0x840) moves a
# kq row 1.02%, ten times what that vein saw.
#
# THE FIRST DESIGN WAS WRONG AND CI KILLED IT IN TWO RUNS. It recorded one
# host's feature block and refused anywhere it did not match, the way
# measured_on.sh refuses a moved glibc. But the runner pool is not one CPU:
# three runs on 2026-09-01 landed on an AMD EPYC Zen 3 (family 0x19, model
# 0x1), an Intel Ice Lake-SP (0x6/0x6a), and the container this was written in
# is a Cascade Lake (0x6/0x55). A check refusing every run but one is red for
# a reason no pull request causes, which is a gate nobody can act on.
#
# So this never refuses. It answers a question, and the gates that count
# instructions ask it only when a row has already moved:
#
#   name      print this host's CPU, every run, so a later divergence is one
#             line of a job log rather than an afternoon of version
#             archaeology
#   differs   0 when this host matches the recorded block, 1 when it does not,
#             2 when there is nothing to compare — no block recorded, or a
#             loader that reports no features
#
# A row that landed on its recorded value is right whatever counted it, so the
# question is worth asking only about a row that moved. When the answer is
# "other silicon", the honest report is that the run cannot say whether
# anything regressed, which is neither green nor a regression.
#
# cpuid[0x1] is excluded by measurement: its top byte is the initial APIC id,
# and it took three values in six runs on one host while every other line held.
set -e
loader=/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2
verb=${1:-name}
block=${2:-bench/dispatch.txt}
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

if [ -x "$loader" ]; then
  "$loader" --list-diagnostics 2>/dev/null \
    | grep '^x86\.cpu_features' \
    | grep -v 'features\[0x0\]\.cpuid\[0x1\]=' \
    | sort > "$scratch/now.txt" || true
fi
[ -f "$scratch/now.txt" ] || : > "$scratch/now.txt"

case "$verb" in
  name)
    if [ ! -s "$scratch/now.txt" ]; then
      echo "silicon: this loader reports no x86 features, so the cpu is unnamed"
      exit 0
    fi
    fam=$(sed -n 's/^x86.cpu_features.basic.family=//p' "$scratch/now.txt")
    mod=$(sed -n 's/^x86.cpu_features.basic.model=//p' "$scratch/now.txt")
    echo "silicon: cpu family $fam model $mod"
    # While no block is recorded, CI prints the whole thing, because that is
    # the only way one ever gets recorded: a block may only be taken from a run
    # that BOTH names its cpu and matches every row, and a run that matches
    # never reaches the `differs` path. Printing 123 lines on every run
    # afterwards would be noise, so this stops as soon as the file exists.
    if [ ! -f "$block" ] && [ -n "$GITHUB_ACTIONS" ]; then
      echo "--- no block recorded; if every row below matches, this is the"
      echo "--- one to copy into $block"
      cat "$scratch/now.txt"
    fi
    ;;
  differs)
    if [ ! -s "$scratch/now.txt" ] || [ ! -f "$block" ]; then
      exit 2
    fi
    # `|| true` because grep answers 1 when a file is all comments, and under
    # `set -e` that would leave the script without saying which answer it meant.
    grep -v '^#' "$block" > "$scratch/want.txt" || true
    if [ ! -s "$scratch/want.txt" ]; then
      exit 2
    fi
    if diff -q "$scratch/want.txt" "$scratch/now.txt" >/dev/null; then
      exit 0
    fi
    echo "the cpu features that differ (recorded < , here > ):"
    diff "$scratch/want.txt" "$scratch/now.txt" || true
    # The block prints for pasting only in CI. These rows belong to the runner,
    # and measured_on.sh's own header records why that matters: a container
    # printed a diff once, somebody pasted, and the container's numbers went
    # into a golden over the runner's. Handing this box its own block back
    # holds that door open.
    if [ -n "$GITHUB_ACTIONS" ]; then
      echo "--- this host's block, should a fresh sitting record it ---"
      cat "$scratch/now.txt"
    else
      echo "The block itself prints only in CI, because these rows belong to"
      echo "the runner and its block is the only one that may be recorded."
    fi
    exit 1
    ;;
  *)
    echo "::error::dispatch.sh takes 'name' or 'differs', not '$verb'" >&2
    exit 2
    ;;
esac
