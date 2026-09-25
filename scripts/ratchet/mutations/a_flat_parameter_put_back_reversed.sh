#!/bin/sh
# A two-word parameter of a flattened arm is rebuilt from its two words at
# entry. This mutation puts them back in the wrong order, so the payload is
# read as the tag; the witness is a_tail_cycle_crosses_arities_in_a_release_build.
set -e
[ "$(grep -cF 'insertvalue {ty} poison, i64 {value}.w0, 0' src/main.rs)" -eq 1 ] || {
  echo "the rebuilt parameter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/insertvalue {ty} poison, i64 {value}\.w0, 0/insertvalue {ty} poison, i64 {value}.w1, 0/; s/insertvalue {ty} {value}\.half, i64 {value}\.w1, 1/insertvalue {ty} {value}.half, i64 {value}.w0, 1/' src/main.rs
[ "$(grep -cF 'insertvalue {ty} poison, i64 {value}.w1, 0' src/main.rs)" -eq 1 ]
