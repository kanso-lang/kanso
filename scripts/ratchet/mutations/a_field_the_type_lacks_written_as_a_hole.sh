#!/bin/sh
# A write to a field the type never declared is refused with the sentence a
# read of it gets, before the hole question is asked. This skips that test,
# so a_field_write_names_a_field_the_type_lacks falls through to the
# placeholder sentence and its errors golden no longer matches.
set -e
grep -qF '            if !declared {' src/check.rs
sed -i 's/            if !declared {/            if false {/' src/check.rs
! grep -qF '            if !declared {' src/check.rs
