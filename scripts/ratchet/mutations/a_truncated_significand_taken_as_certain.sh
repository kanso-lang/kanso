#!/bin/sh
# The fast decimal-to-double path claims certainty about a significand it
# truncated. `4409065699.4409065699e-2` reads one ULP low on native and
# correctly on the oracle, so the two engines disagree.
#
# Nineteen significant digits is all the scan keeps. Past that an integer
# digit is traded for a power of ten and a fraction digit is dropped, and the
# number handed to eisel-lemire is no longer the one that was written — it can
# sit on the far side of a rounding boundary from the true value. The `cut`
# flag is what sends those to strtod instead; dropping it from the condition
# puts the wrong answer back.
#
# The gate this turns red is the specs job, through
# `micro_corpus_agrees_across_engines` reading
# tests/golden/micro/a_long_significand_rounds_like_the_oracle.kso, which runs
# every micro program on native AND `--interp` and compares them.
set -e
grep -qF 'if (ok && any && !cut && p == stop) {' src/runtime.c || {
  echo "the truncation guard moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|if (ok \&\& any \&\& !cut \&\& p == stop) {|if (ok \&\& any \&\& p == stop) {|' src/runtime.c
grep -qF 'if (ok && any && p == stop) {' src/runtime.c
