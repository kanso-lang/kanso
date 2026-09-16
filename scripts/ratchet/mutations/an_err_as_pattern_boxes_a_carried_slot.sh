#!/bin/sh
# An arm that names the whole value at a carried slot, `r@(parsed p v)`, wants
# the record itself, so the slot is passed boxed for every arm of the group.
# `e@(err _)` is exempt since 2026-09-16: on the failure path the two words
# are the failure, the dispatcher already reads them back as one value, and
# the name binds to that. This mutation drops the exemption, so the five
# hand-back arms the 2026-09-15 ruling asked of the json decoder turn the
# two-word convention off again and every scanner answer builds a record.
#
# The gate is the mem vein: an_err_as_pattern_keeps_a_carried_slot_unboxed
# reads sh_rec=0 with the exemption and 64000 without it.
set -e
grep -qF 'Some(Pattern::Ctor { ty, whole: Some(_), .. }) if ty != "err")' src/escape.rs
sed -i 's/Some(Pattern::Ctor { ty, whole: Some(_), .. }) if ty != "err")/Some(Pattern::Ctor { whole: Some(_), .. }))/' src/escape.rs
! grep -qF 'whole: Some(_), .. }) if ty != "err"' src/escape.rs
