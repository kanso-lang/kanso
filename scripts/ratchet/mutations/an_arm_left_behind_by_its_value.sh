#!/bin/sh
# A value carries its owner's arms into code that never imported the owner
# (2026-10-12). This mutation lets no call travel, so a library calling
# `area` on a circle it was handed is refused with an unknown name.
set -e
line="        !name.contains('/')"
[ "$(grep -cxF "$line" src/check/shadow.rs)" -eq 1 ] || {
  echo "the traveling test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s|^        !name.contains('/')\$|        false \&\& !name.contains('/')|" src/check/shadow.rs
grep -qxF "        false && !name.contains('/')" src/check/shadow.rs
