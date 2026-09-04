# A counter's DEFINITION changes and the gate reads the change as a win, then
# lets that win pay for a real regression somewhere else.
#
# kanso#1241 is the fixture this row generalises. It redefined the compile row
# to count the compiler's own frame instead of the whole process, dropping
# 465,864 instructions of loader and stack guard from every reading, and
# subtracted the same constant from welfare's baseline so the ratio stayed
# comparable. Nothing in the compiler got faster. The gate printed
# `improved: compile_instructions 41,845,704 -> 41,379,840` and that sentence
# went into the permanent record of the merge.
#
# THE TWO EDITS ARE ONE MOVE AND BOTH HALVES ARE LOAD-BEARING. The golden alone
# is an ordinary improvement; the baseline alone is welfare's business and no
# counter here moves. Together they say the old reading and the new one are not
# of the same quantity, which is what makes the ratio meaningless in either
# direction — `welfare --set` moves the floor and never the baseline, so a
# baseline that moves is somebody deciding exactly that.
#
# THE LOG PARAGRAPH IS LOAD-BEARING TOO, and tests/a_re_basing_row_stays_a_
# pure_regression.rs holds it to the values below. Without it the gate goes red
# on UNPRICED — silence about a move — and the row would pass while proving
# nothing about how the move was CLASSIFIED. What must refuse this branch is
# the pure-regression rule: jsonbench got worse, the re-basing is not a win,
# and no history entry attributes the fall.
set -e
grep -q '^compile_instructions=[0-9]' bench/compile_instructions_golden.txt || {
  echo "the compile row changed shape; this mutation needs rewriting" >&2
  exit 1
}
grep -q '^jsonbench [0-9]' bench/instructions_golden.txt || {
  echo "the runtime instruction rows changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's/^compile_instructions=.*/compile_instructions=41000000/' \
  bench/compile_instructions_golden.txt
sed -i 's/"compile_instructions":[0-9]*/"compile_instructions":56000000/' \
  bench/welfare_floor.json
sed -i 's/^jsonbench [0-9].*/jsonbench 9999999999/' bench/instructions_golden.txt
cat >> design/compiler-log.md <<'ENTRY'

## ratchet mutation — a re-basing beside a regression

compile_instructions is re-based to 41,000,000: the reading counts something
else now, and welfare's baseline moves with it so the ratio stays comparable.
work_jsonbench rises to 9,999,999,999 and nothing is traded for it.
ENTRY
grep -q '^compile_instructions=41000000' bench/compile_instructions_golden.txt
grep -q '"compile_instructions":56000000' bench/welfare_floor.json
grep -q '^jsonbench 9999999999' bench/instructions_golden.txt
