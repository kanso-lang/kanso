#!/bin/sh
# The lexer answers the text-block and span openers for a line with no quote
# and no `#` from one scan a word at a time. This mutation calls no line
# plain, so every line goes through both byte-at-a-time scans. Same openers,
# and the front end reads heavier.
set -e
l="        let plain = crate::compare::either_from(content.as_bytes(), 0, b'\"', b'#') == content.len();"
grep -qxF "$l" src/lexer.rs || {
  echo "the lexer's plain-line test moved; this mutation needs rewriting" >&2
  exit 1
}
awk -v l="$l" '$0 == l { print "        let plain = false;"; next } { print }' src/lexer.rs > src/lexer.rs.mut
mv src/lexer.rs.mut src/lexer.rs
grep -qxF "        let plain = false;" src/lexer.rs
