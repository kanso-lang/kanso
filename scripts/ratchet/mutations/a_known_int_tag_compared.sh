#!/bin/sh
# Arithmetic asks whether both tags are the int tag, and a tag already known
# to be 0 is not compared. This mutation compares every tag, so `x + 1`
# writes `icmp eq i64 0, 0` again. The output is the same; the witness is
# a_tag_known_to_be_int_is_not_compared.
set -e
line='        if tag != "0" {'
[ "$(grep -cF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the known-tag test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if tag != "0" {$/        if !tag.is_empty() {/' src/codegen.rs
[ "$(grep -cF '        if !tag.is_empty() {' src/codegen.rs)" -eq 1 ]
