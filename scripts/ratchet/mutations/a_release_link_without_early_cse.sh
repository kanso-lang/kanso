#!/bin/sh
# The release link runs early-cse<memssa> before its first jump-threading.
# This mutation takes it out of the link's pipeline. The answers are the same;
# the work vein is the witness.
set -e
pass='early-cse<memssa>,jump-threading'
[ "$(grep -cF "$pass" src/main.rs)" -eq 1 ] || {
  echo "the link's early-cse moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/early-cse<memssa>,jump-threading/jump-threading/' src/main.rs
if grep -qF "$pass" src/main.rs; then exit 1; fi
