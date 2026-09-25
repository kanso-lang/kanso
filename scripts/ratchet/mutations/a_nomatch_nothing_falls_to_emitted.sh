#!/bin/sh
# A switch dispatcher writes its `nomatch` failure path only when a case falls
# to it. This mutation always writes it, so a switch whose cases cover every
# value carries blocks nothing reaches. The output is the same; the witness is
# a_block_nothing_branches_to_is_not_emitted, whose JSON decode reaches such
# switches.
set -e
line='        if branches_to(&f.out, "nomatch") {'
grep -qF "$line" src/codegen.rs || {
  echo "the nomatch check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if branches_to(&f.out, "nomatch") {$/        if !f.out.is_empty() || branches_to(\&f.out, "nomatch") {/' src/codegen.rs
[ "$(grep -cF '        if !f.out.is_empty() || branches_to(&f.out, "nomatch") {' src/codegen.rs)" -eq 1 ]
