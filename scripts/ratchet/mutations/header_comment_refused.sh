#!/bin/sh
# A file may open with a header comment and one blank line (2026-10-13,
# #1887). This mutation refuses the blank under the header as a leading blank,
# and the micro corpus's fixture that opens with one stops building.
set -e
line='        let under_header = lexed.header > 0 && *blank == lexed.header + 1;'
[ "$(grep -cxF "$line" src/parser.rs)" -eq 1 ] || {
  echo "the header's blank moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let under_header = lexed.header > 0 \&\& \*blank == lexed.header + 1;$/        let under_header = false;/' src/parser.rs
if grep -qxF "$line" src/parser.rs; then exit 1; fi
