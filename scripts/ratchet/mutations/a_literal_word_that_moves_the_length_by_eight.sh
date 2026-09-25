#!/bin/sh
# A literal of one to eight bytes is stored as one word, and the builder's
# length moves by the literal's own length. This mutation moves it by the
# word's eight bytes, so the bytes past the literal become text:
# a_short_literal_is_appended_whole prints them.
set -e
grep -q '^  %lenn = add i64 %len, %n$' src/codegen.rs || {
  echo "the literal word's length changed shape; rewrite this" >&2
  exit 1
}
sed -i 's/^  %lenn = add i64 %len, %n$/  %lenn = add i64 %len, 8/' src/codegen.rs
grep -q '^  %lenn = add i64 %len, 8$' src/codegen.rs
