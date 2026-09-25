#!/bin/sh
# The accumulator rewrite deleted, by making its pass return before it looks
# at anything.
#
# Nothing else in the board can see this. Every allocation counter stays flat,
# both compile goldens stay byte-identical, the welfare score holds, and every
# program in the corpus still answers what it always answered — the rewrite
# only changes the SHAPE a recursion runs in. What goes red is the differential
# that runs a sum twenty thousand deep on the interpreter, where a frame per
# call meets the ten-thousand-frame guard.
set -e
head="pub fn rewrite(program: &mut Program) {"
grep -qF "$head" src/trmc.rs || {
  echo "rewrite's signature moved; this mutation needs rewriting" >&2
  exit 1
}
awk -v h="$head" '{ print } $0 == h { print "    if true { return; }" }' \
  src/trmc.rs > src/trmc.rs.mutated
mv src/trmc.rs.mutated src/trmc.rs
grep -A1 '^pub fn rewrite' src/trmc.rs | grep -qF 'if true { return; }'
