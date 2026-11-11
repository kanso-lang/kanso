#!/bin/sh
# A failure crosses the two-word record convention in its own two words, and
# until 2026-09-16 the unpack on the far side of a beat carry read two fields
# off it as if it were the record. The consumer's record arm then matched the
# failure and its body ran on garbage words: native printed `error[runtime]:
# `+` is not defined for these values` where the interpreter printed
# `stopped: end of input`. k_parsed_words asks first now. This mutation takes
# the ask out, so the unpack reads fields off the failure again.
#
# The gate is the mem vein: a_failure_handed_back_by_a_boxed_group_stays_a_failure
# prints `stopped: end of input` on native with the ask and stops on a runtime
# error without it, which the corpus reports as a stdout mismatch before it
# reads a counter. a_failure_crosses_a_beat_carry_in_two_words was the fixture
# until the carry began staging a record's two fields (kanso#2431): it unpacks
# no built record there any more, so it stopped reaching the ask.
set -e
grep -qF 'KValue k_parsed_words(KValue v) {' src/runtime/values.c
sed -i '/^KValue k_parsed_words(KValue v) {$/,/^}$/ s/^    if (!k_not_failure(v)) return v;$/    (void)0;/' src/runtime/values.c
! awk '/^KValue k_parsed_words\(KValue v\) \{$/,/^}$/' src/runtime/values.c | grep -qF 'k_not_failure(v)'
