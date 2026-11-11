#!/bin/sh
# linked_as copies the cached runtime object to a plain name and then hands
# the link the cached path anyway, so a measured link reads a file name that
# holds a digest of src/main.rs again. Every program still builds and runs.
set -e
grep -c 'Ok(_) => named,' src/main.rs | grep -qx 1 || {
  echo "linked_as's answer moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/Ok(_) => named,/Ok(_) => object,/' src/main.rs
grep -q 'Ok(_) => object,' src/main.rs
