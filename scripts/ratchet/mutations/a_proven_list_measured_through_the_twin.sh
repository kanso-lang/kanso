#!/bin/sh
# `length` of a value the sets prove is a list or bytes reads the header's
# first word. This mutation narrows that back to bytes alone, so a list's
# length goes through the twin's tag test again. The answers are the same;
# the work vein is the witness.
set -e
line='            if name == "length" && held != 0 && held & !(BYTES | LIST) == 0 {'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the proven length moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if name == "length" && held != 0 && held & !(BYTES | LIST) == 0 {$/            if name == "length" \&\& held == BYTES {/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
