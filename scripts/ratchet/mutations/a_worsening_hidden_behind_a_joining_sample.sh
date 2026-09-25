# A counter worsens in a sample that was already there, in the same commit
# that adds a new one. The trend gate must still refuse it.
#
# This row exists because the first fix for the joining-sample false positive
# passed this case silently. `emitted_other_*` are sums across the benchmarks
# in bench/emitted_golden_others.txt, so a tenth benchmark raises every one of
# them and no program already there emitted a line more than before; the gate
# read that as a pure regression and refused a change that was fine. The first
# draft answered by skipping the whole golden for that run, which also skipped
# `scanbench calls 3,743 -> 9,999` sitting beside it. The gate drops only the
# joining sample now and sums both sides over what they share.
#
# So the mutation does both halves at once: it adds a sample and worsens one
# that was already there. A gate that skips the file goes green; a gate that
# intersects stays red.
#
# The added row's name is what matters, not its figures — it only has to be a
# sample the base does not have. scanbench's rise is a bare number for the
# same reason the compile-instructions mutation pins one: the golden's real
# row moves whenever the emitter does, and an anchor naming today's figure
# would stop applying the next time it moves.
set -e
printf 'a_sample_the_base_never_had defines=1 calls=1 branches=1 lines=1\n' \
  >> bench/emitted_golden_others.txt
sed -i 's/^scanbench defines=\([0-9]*\) calls=[0-9]*/scanbench defines=\1 calls=999999/' \
  bench/emitted_golden_others.txt
grep -q '^a_sample_the_base_never_had ' bench/emitted_golden_others.txt
grep -q '^scanbench .*calls=999999' bench/emitted_golden_others.txt
