#!/bin/sh
# A counter worsens, and the only thing on the other side of the trade is a
# counter no direction table names.
#
# `worsened?` answers false for a name neither `lower` nor `higher` knows, and
# the listing printed everything not worsened as `improved`, so an unclassified
# counter moving in ANY direction landed in the `better` list and satisfied the
# pure-regression rule on its own. Found on 2026-09-01: lifting the carry-tier
# prefix read `evac_allocs 27 -> 33,827`, `evac_bytes 1,520 -> 2,705,520` and
# `thunk_evals 1 -> 64` as three wins.
#
# So the mutation does both halves. `scanbench calls` is in `lower_d` and rises,
# which is a real worsening. `evac_allocs` in the digest golden is in neither
# table and rises beside it. A gate that reads the second as an improvement goes
# green; a gate that counts it toward neither side stays red.
#
# The figures are bare numbers rather than anchored to today's rows, for the
# reason the joining-sample mutation gives: the real rows move whenever the
# emitter or the runtime does.
set -e
sed -i 's/^scanbench defines=\([0-9]*\) calls=[0-9]*/scanbench defines=\1 calls=999999/' \
  bench/emitted_golden_others.txt
sed -i 's/^evac_allocs=[0-9]*/evac_allocs=999999/' bench/cost_golden_digest.txt
grep -q '^scanbench .*calls=999999' bench/emitted_golden_others.txt
grep -q '^evac_allocs=999999' bench/cost_golden_digest.txt
