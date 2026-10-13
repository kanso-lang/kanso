#!/bin/sh
# A file of a several-file module sees only the names its own imports bring,
# with the module's declarations (2026-10-12). This mutation hides nothing,
# so a file calls `fold` through the std/list import its sibling wrote and
# the module checks clean.
set -e
line='            _ => hidden_from(program, &parsed, &surfaced, &declared_here),'
[ "$(grep -cxF "$line" src/lib/modules.rs)" -eq 1 ] || {
  echo "the per-file visibility call moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            _ => hidden_from(program, \&parsed, \&surfaced, \&declared_here),$/            _ => crate::hash::Map::<\&str, String>::default(),/' src/lib/modules.rs
if grep -qxF "$line" src/lib/modules.rs; then exit 1; fi
