#!/bin/sh
# An exact instruction golden must be a property of the binary, not of where
# the tree happens to sit. It was not, and nothing could see it: the row moved
# with the length of the path the benchmark is exec'd from, so the goldens
# quietly meant "measured from the repo root" and any reader elsewhere -- the
# ratchet's scratch worktree, a local A/B in /tmp -- got a different number and
# no warning.
#
# This runs the same binary from two source trees whose names differ in length,
# through the same staging scripts/gates/instructions.sh uses, and refuses if
# the two disagree. It costs two callgrind passes over the cheapest pair of
# benchmarks, a few seconds.
#
# digestbench is here because it is the case that defeated the obvious fix:
# pinning only the exec path leaves it moving with the working directory, since
# it opens bench/digest_input.txt. indexbench is here because it opens nothing,
# so a failure on it is the exec path alone. Between them the two halves are
# covered, and either one going red names which half broke.
set -e

# FOUR source trees whose names differ by one character each, not two of
# arbitrary length. The first draft used two that happened to differ by
# sixteen; the swing has period four, so both sat in the same phase, and the
# check stayed green with the staging taken out -- it could not fail. Four
# consecutive lengths cover every phase, so the refusal does not depend on
# guessing the period right.
base=/tmp/kanso-pi
stage=/tmp/kanso-pi-run
srcs=""
pad=""
for i in 1 2 3 4; do
  src="$base$pad"
  srcs="$srcs $src"
  rm -rf "$src"
  mkdir -p "$src/bench"
  cp ./indexbench ./digestbench "$src/"
  cp bench/digest_input.txt "$src/bench/"
  pad="${pad}x"
done

# The count has to be READ before it can be compared, and the read is where
# this went wrong the first time it ran on CI. valgrind's stderr is where the
# total is, so it goes to a file -- and that file is also where a "valgrind:
# not found" lands. The step was placed ahead of the one that installs
# valgrind, so the gate exited 127 having printed NOTHING, and the job log said
# only "Process completed with exit code 127". A tool that is missing must say
# so out loud; a gate whose whole output is a number owes its reader the file
# when the number does not arrive.
measure() {  # measure <source tree> <benchmark>; echoes the instruction count
  rm -rf "$stage"
  mkdir -p "$stage/bench"
  cp "$1/$2" "$stage/$2"
  cp "$1/bench/digest_input.txt" "$stage/bench/"
  ( cd "$stage" && env -i PATH=/usr/bin:/bin \
      valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.pi ./"$2" \
      >/dev/null 2>/tmp/ir.pi ) || true
  ir=$(grep -o 'I   refs:.*' /tmp/ir.pi | tr -dc 0-9)
  if [ -z "$ir" ]; then
    echo "::error::no instruction count came back for $2 run from $1." >&2
    echo "::error::What the run said, in full:" >&2
    sed 's/^/::error::    /' /tmp/ir.pi >&2
    exit 1
  fi
  printf '%s' "$ir"
}

# `moved` is per benchmark and `rc` is the verdict for the job. They were one
# variable to begin with, which made the second benchmark report the first
# one's failure: indexbench going red printed the error block for digestbench
# too, over four readings that agreed. The comment above promises that either
# one going red names which half broke, and one variable could not keep it.
rc=0
for b in indexbench digestbench; do
  first=""
  seen=""
  moved=0
  for src in $srcs; do
    got=$(measure "$src" "$b")
    [ -z "$first" ] && first="$got"
    seen="$seen $got"
    [ "$got" = "$first" ] || { moved=1; rc=1; }
  done
  if [ "$moved" -eq 0 ]; then
    echo "    $b $first from all four tree lengths"
  else
    echo "::error::$b read$seen from four trees whose paths differ only in"
    echo "::error::length. The instruction goldens are supposed to be a property of"
    echo "::error::the binary; this one moves with the checkout path, so every reader"
    echo "::error::outside the repo root -- the ratchet's scratch worktree among them"
    echo "::error::-- compares against a number it cannot reproduce."
    echo "::error::scripts/gates/instructions.sh stages both the exec path and the"
    echo "::error::inputs to keep this fixed; something undid that."
  fi
done
[ "$rc" -eq 0 ] && echo "path independence: the rows do not move with the tree"
exit "$rc"
