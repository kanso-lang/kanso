#!/bin/sh
# Where one call's arguments read a map's key and then its value at the same
# position, the emitter tests the map's view once and loads both. This
# mutation never sees the pair, so the key and the value each test the view.
# The output is the same; the witness is the emitted gate.
set -e
line='        (f.lookup(m).is_some() && f.lookup(i).is_some()).then_some(at)'
grep -qxF "$line" src/codegen/emit.rs || {
  echo "the entry pair's check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        (f.lookup(m).is_some() \&\& f.lookup(i).is_some()).then_some(at)$/        (false \&\& f.lookup(m).is_some() \&\& f.lookup(i).is_some()).then_some(at)/' src/codegen/emit.rs
if grep -qxF "$line" src/codegen/emit.rs; then exit 1; fi
