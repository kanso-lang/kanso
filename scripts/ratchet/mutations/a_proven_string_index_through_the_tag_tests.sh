#!/bin/sh
# Where the sets prove a string and an int, the emitter calls `k_b_at_str`
# with the pointer and the position. This mutation takes that arm away, so
# the index goes through the bytes test and `k_b_at`'s tag tests as before.
# The output is the same; the witness is the work vein.
set -e
line='        if !strict && f.set_of(container) == STR && f.set_of(key) == INT {'
grep -qxF "$line" src/codegen/emit.rs || {
  echo "the proven string index moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if !strict \&\& f.set_of(container) == STR \&\& f.set_of(key) == INT {$/        if false \&\& !strict \&\& f.set_of(container) == STR \&\& f.set_of(key) == INT {/' src/codegen/emit.rs
if grep -qxF "$line" src/codegen/emit.rs; then exit 1; fi
