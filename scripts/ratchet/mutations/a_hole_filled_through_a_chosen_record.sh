#!/bin/sh
# A hole is filled by the name of the record built with it, never through a
# record an `if` or a list chose. This drops the chosen-record refusal so
# the write falls through to the placeholder sentence, and
# a_hole_filled_through_a_chosen_record's errors golden no longer matches.
set -e
grep -qF 'None if chosen => format!(' src/check.rs
sed -i 's/None if chosen => format!(/None if false => format!(/' src/check.rs
! grep -qF 'None if chosen => format!(' src/check.rs
