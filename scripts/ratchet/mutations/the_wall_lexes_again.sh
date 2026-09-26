#!/bin/sh
# `>>` reaches the parser again.
#
# The lexer refuses the two characters where they are written and names the
# spelling that replaced them. Without the refusal they lex as two `>` and the
# program fails somewhere else, with a message about comparison, so the error
# corpus's the_wall_is_gone reads a different diagnostic.
set -e
f=src/lexer.rs
grep -q 'kanso has no `>>`' src/lexer.rs
line="        if c == '>' && s.peek(1) == Some('>') {"
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the wall's refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s/^        if c == '>' \&\& s.peek(1) == Some('>') {\$/        if false \&\& c == '>' \&\& s.peek(1) == Some('>') {/" "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
