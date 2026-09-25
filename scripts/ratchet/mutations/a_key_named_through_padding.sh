#!/bin/sh
# A key's name costs what its value makes it cost.
#
# `hex16` writes sixteen digits from a fixed loop. `{:016x}` pads a value
# with a leading zero nibble one `write_char` at a time, and the keys it
# names hash tools' modification times, so the start-up row read 52 apart on
# two runner images for one binary.
set -e
f=src/main.rs
grep -q 'fn hex16' src/main.rs
grep -qxF '    String::from_utf8(digits).expect("hex digits are ascii")' "$f" || {
  echo "hex16 moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    String::from_utf8(digits).expect("hex digits are ascii")$|    let _ = digits;\n    format!("{n:016x}")|' "$f"
grep -qF '    format!("{n:016x}")' "$f"
