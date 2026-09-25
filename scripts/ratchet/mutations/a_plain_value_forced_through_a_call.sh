#!/bin/sh
# Asking whether a value is an unforced cell inlines at every caller, and
# only a cell reaches the out-of-line loop. This mutation sends every value
# through the loop, which hands a non-cell back as it came. The answer is the
# same; interp_instructions is the witness.
set -e
line='            other => Ok(other),'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the inline thunk test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            other => Ok(other),$/            other => self.force_cell(other),/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
