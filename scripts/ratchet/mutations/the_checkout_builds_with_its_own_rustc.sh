#!/bin/sh
# The checkout stops naming its rustc.
#
# Without rust-toolchain.toml a container's default rustc answers every cargo
# run. On 2026-10-06 that was 1.94.1, and two log entries published playground
# sizes measured under it. The file goes; the spec says the checkout no longer
# pins one.
set -e
test -f rust-toolchain.toml || {
  echo "rust-toolchain.toml is gone already; this mutation needs rewriting" >&2
  exit 1
}
rm rust-toolchain.toml
test ! -f rust-toolchain.toml
