#!/bin/sh
# A `-` with a space before it is read as a binary minus again.
#
# Spacing decides a minus sign (ruled 2026-09-30): a `-` touching a digit with
# a space before it is part of the number. Without that, `twice -3` and
# `[1 -3 4]` in the micro corpus's a_minus_touching_a_digit_is_the_number are
# refused as spacing errors, which is what they were before the ruling.
set -e
f=src/lexer.rs
grep -q 'let prefix = spaced' src/lexer.rs
line='            let prefix = spaced'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the spaced-minus test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            let prefix = spaced$/            let prefix = false/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
