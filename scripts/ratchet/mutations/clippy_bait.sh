#!/bin/sh
set -e

# THE PATH IS SPELLED ON A GUARD LINE, and the guard is a real one rather than
# a formality. This mutation anchors on nothing -- it appends -- so it cannot
# go stale the way an anchored mutation does, and it carried no grep at all.
# That left it invisible to the ratchet's `touched` pass, which selects the
# rows a branch could have blinded by intersecting the changed files with the
# paths each mutation names on a guard line.
#
# It CAN still be blinded, and by an edit to this very file: an
# `allow(clippy::ptr_arg)` anywhere in src/lib.rs makes the bait inert and the
# gate green with the defect present.
if grep -q 'allow(clippy::ptr_arg)' src/lib.rs; then
  echo "src/lib.rs allows ptr_arg; this bait is inert" >&2
  exit 1
fi

printf 'pub fn lint_bait(v: &Vec<u8>) -> usize { v.len() }\n' >> src/lib.rs
