#!/bin/sh
# A record goes back in two words only when every arm of its group answers
# that record or a failure. This mutation skips the check, so `list/next`
# answers `done` and `list/iter` answers a `cursor` through a convention
# that reads both back as the record, and two micro fixtures say so.
set -e
kept='        if answers_other {'
[ "$(grep -cxF "$kept" src/escape.rs)" -eq 1 ] || {
  echo "the arm check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if answers_other {$/        if answers_other \&\& false {/' src/escape.rs
if grep -qxF "$kept" src/escape.rs; then exit 1; fi
