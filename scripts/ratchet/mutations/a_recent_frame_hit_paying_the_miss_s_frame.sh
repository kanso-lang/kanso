#!/bin/sh
# A hit in the recent-frame table is answered inline at the caller, and only
# a miss calls the out-of-line path that builds a frame. This mutation keeps
# the lookup out of line, so every hit pays for a call again. The answer is
# the same; interp_instructions is the witness.
set -e
[ "$(grep -c '^    fn frame_for(&self, decl: &.a FnDecl) -> Frame {$' src/eval.rs)" -eq 1 ] || {
  echo "frame_for moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    #\[inline\]$/{N;s/^    #\[inline\]\n    fn frame_for/    #[inline(never)]\n    fn frame_for/}' src/eval.rs
grep -B1 '^    fn frame_for' src/eval.rs | grep -qx '    #\[inline(never)\]'
