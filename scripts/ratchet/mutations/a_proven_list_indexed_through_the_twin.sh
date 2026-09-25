#!/bin/sh
# A plain index of a list the sets prove is a list skips the twin's tag tests.
# This mutation sends it through the twin again. The answers are the same;
# the work vein is the witness.
set -e
line='        if !strict && f.set_of(container) == LIST && f.set_of(key) == INT {'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the proven list's index moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if !strict && f.set_of(container) == LIST && f.set_of(key) == INT {$/        if false \&\& !strict \&\& f.set_of(container) == LIST \&\& f.set_of(key) == INT {/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
