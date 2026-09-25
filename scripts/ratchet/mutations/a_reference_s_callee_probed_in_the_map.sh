#!/bin/sh
# A call through a function reference first asks a direct-mapped table of the
# last callee per address slot. This mutation never takes its answer, so
# every call probes the map again. The answer is the same;
# interp_instructions is the witness.
set -e
line='            if *k == key {'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the recent-callee test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if \*k == key {$/            if *k == key \&\& false {/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
