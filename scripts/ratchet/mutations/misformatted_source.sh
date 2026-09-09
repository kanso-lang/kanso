#!/bin/sh
set -e

# THE PATH IS SPELLED ON A GUARD LINE, for the reason clippy_bait.sh gives at
# length: an append-only mutation anchors on nothing, so it carried no grep,
# so the `touched` pass could not see it.
#
# Its blinding edit is a crate-level formatting escape in the file it appends
# to, which would leave rustfmt content with the misformatted line.
if grep -q 'rustfmt::skip' src/lib.rs; then
  echo "src/lib.rs escapes rustfmt; this mutation is inert" >&2
  exit 1
fi

printf 'pub fn  spaced ( ) { }\n' >> src/lib.rs
