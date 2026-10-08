#!/bin/sh
# Hand each line's token buffer itself to the parse again, instead of moving
# its tokens into a vector of their exact length. The buffer then starts empty
# on every line and doubles to its length, an allocation per step, and keeps
# the spare half for the whole parse. The output is identical either way, so
# the witness is the allocation vein.
set -e
target='    let exact = tokens.drain(..).collect();'
n=$(grep -cF "$target" src/lexer.rs)
[ "$n" -eq 1 ] || { echo "the exact move moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '
  $0 == t { print "    let exact = std::mem::take(&mut tokens);"; next }
  { print }
' src/lexer.rs > src/lexer.rs.mut && mv src/lexer.rs.mut src/lexer.rs
