#!/bin/sh
# One job goes back to installing whatever rustc is stable that day.
#
# That is how every job installed rust until 2026-10-04, and it is how the
# runner's rustc moved from 1.98.1 to 1.99.0 underneath every golden that
# names it. The first toolchain step in ci.yml loses its pin; the spec names the
# line that floats.
set -e
grep -q 'dtolnay/rust-toolchain@master' .github/workflows/ci.yml || {
  echo "ci.yml installs rust some other way; this mutation needs rewriting" >&2
  exit 1
}
sed -i '0,/dtolnay\/rust-toolchain@master/s//dtolnay\/rust-toolchain@stable/' \
  .github/workflows/ci.yml
grep -q 'dtolnay/rust-toolchain@stable' .github/workflows/ci.yml
