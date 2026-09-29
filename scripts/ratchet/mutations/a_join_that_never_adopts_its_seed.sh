#!/bin/sh
# Native's first join never adopts a seed that was not a string.
#
# `k_b_str_builder` hands such a seed on as it came, so the first join renders
# it and `k_b_adopt` makes that rendering the builder. Without the adoption
# the join is handed a plain string and dies with `a string builder was
# expected here`.
set -e
f=src/codegen.rs
grep -q 'k_b_adopt' src/codegen.rs
grep -qF '                            match joins_builder && i == 0 && f.set_of(&value) & !STR != 0 {' "$f" || {
  echo "the adoption's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                            match joins_builder && i == 0 && f.set_of(&value) & !STR != 0 {$/                            match false \&\& joins_builder \&\& i == 0 \&\& f.set_of(\&value) \& !STR != 0 {/' "$f"
grep -qF 'match false && joins_builder && i == 0' "$f"
