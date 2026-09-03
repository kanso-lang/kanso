#!/bin/sh
# A band assembled one reading at a time.
#
# bench/compile_instructions_by_cpu.txt may pin two values for a chip, because
# two CI runs printed the same binary sha256 de5bfab22fbd and the same cpu
# family 0x6 model 0xcf and counted 41,831,767 and 41,832,275. Neither named
# suspect explained it. Two is the cap, and the cap is the whole difference
# between the pair and the tolerance this vein refused: a range wide enough to
# hold 508 also holds kanso#1226's -5,621, which was a real change to the
# compiler.
#
# The failure mode the cap forbids is quiet and reasonable-looking. A run
# counts a third value, somebody appends it beside the other two, and the row
# now admits three numbers with no reading explained. Repeat and the row admits
# everything.
#
# The mutation removes the arity refusal, which is the state that would let the
# third value in.
set -e
row=scripts/gates/compile_ir_row.sh
before=$(grep -c '^  exit 1$' "$row")
if [ "$before" -ne 7 ]; then
  echo "expected seven refusals in $row, found $before;" >&2
  echo "the shape moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i '/^malformed=/,/^fi$/d' "$row"
after=$(grep -c '^  exit 1$' "$row")
if [ "$after" -ne 6 ]; then
  echo "wanted exactly the arity refusal removed, $before became $after" >&2
  exit 1
fi
if grep -q 'NF > 3' "$row"; then
  echo "the arity check survived the cut" >&2
  exit 1
fi
