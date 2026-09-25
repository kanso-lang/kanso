#!/bin/sh
# Entering a declaration asks a direct-mapped table of the last frame per
# slot before the map. This mutation never takes the table's answer, so every
# entry probes the map again. The answer is the same; interp_instructions is
# the witness.
set -e
line='            if *at == key {'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the recent-frame test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if \*at == key {$/            if *at == key \&\& false {/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
