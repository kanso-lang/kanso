#!/bin/sh
# The inlined list push asks its two room questions through one `and`.
#
# Split, the frontier test and the capacity test are two compares and two
# branches. Joined again, LLVM computes both as flags and ors them.
set -e
f=src/codegen.rs
grep -q 'define internal %KValue @k_b_push_mut_fast' src/codegen.rs
line='  br i1 %lfront, label %lroom, label %lslow'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the push's room test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i -e '/^  br i1 %lfront, label %lroom, label %lslow$/d' -e '/^lroom:$/d' \
  -e 's/^  br i1 %lfits, label %lwrite, label %lslow$/  %lok = and i1 %lfront, %lfits\n  br i1 %lok, label %lwrite, label %lslow/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
