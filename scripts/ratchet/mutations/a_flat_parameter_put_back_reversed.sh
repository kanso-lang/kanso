#!/bin/sh
# A parameter of a flattened arm that is more than a word is rebuilt from its
# words at entry. This mutation puts them back in reversed order, so a
# `%KValue`'s payload is read as its tag; the witness is
# a_tail_cycle_crosses_arities_in_a_release_build.
set -e
line='                                "  {into} = insertvalue {ty} {built}, i64 {value}.w{w}, {w}\n"'
[ "$(grep -cxF "$line" src/main/tail_calls.rs)" -eq 1 ] || {
  echo "the rebuilt parameter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^\(                                "  {into} = insertvalue {ty} {built}, i64 {value}\.w{w}, \){w}\(\\n"\)$/\1{}\2, n - 1 - w/' src/main/tail_calls.rs
! grep -qxF "$line" src/main/tail_calls.rs
