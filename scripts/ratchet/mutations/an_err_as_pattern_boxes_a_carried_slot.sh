#!/bin/sh
# An arm that names the whole value at a carried slot, `r@(parsed p v)`, wants
# the record itself, so the slot is passed boxed for every arm of the group.
# `e@(err _)` is exempt since 2026-09-16: it matches only an err a rescue
# took out of its box (#2087), so at a slot the two words carry it never
# fires. This mutation drops the exemption, so a hand-back arm left from the
# 2026-09-15 ruling turns the two-word convention off again and every scanner
# answer builds a record.
#
# The gate is the mem vein: an_err_as_pattern_keeps_a_carried_slot_unboxed
# reads sh_rec=0 with the exemption and 64000 without it.
set -e
grep -qF 'Some(Pattern::Ctor { ty, whole: Some(_), .. }) if ty != "err")' src/escape.rs
sed -i 's/Some(Pattern::Ctor { ty, whole: Some(_), .. }) if ty != "err")/Some(Pattern::Ctor { whole: Some(_), .. }))/' src/escape.rs
! grep -qF 'whole: Some(_), .. }) if ty != "err"' src/escape.rs
