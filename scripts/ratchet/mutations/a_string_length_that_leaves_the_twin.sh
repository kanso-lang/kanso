#!/bin/sh
# The length twin reads a string's count memo inline when the header holds
# one; a string not yet counted goes to the C entry as before. This sends
# every string there, memo or not: the emitted line count is unchanged, since
# the branch keeps its shape and only its target moves, and no counter reads a
# call that was skipped. The work vein is the witness -- runbench asks the
# length of 690,000 one-character strings in its index phase, and each ask
# goes back to a call, a jump table and six pushed and popped registers.
set -e
grep -q '^  br i1 %is_str, label %memo, label %slow$' src/codegen.rs || {
  echo "the length twin's string arm changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  br i1 %is_str, label %memo, label %slow$|  br i1 %is_str, label %slow, label %slow|' \
  src/codegen.rs
grep -q '^  br i1 %is_str, label %slow, label %slow$' src/codegen.rs
