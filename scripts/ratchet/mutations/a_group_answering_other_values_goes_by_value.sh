#!/bin/sh
# A record goes back in two words only when every arm of its group answers
# that record or a failure. This mutation skips the check, so a group that
# answers something else is read back as the record: `list/next` answering
# `done`, `list/iter` answering a `cursor`, and regexp's own. The micro
# corpus reaches a regexp fixture first, and its binary allocates until the
# limit scripts/bounded.sh sets refuses it.
set -e
kept='        if answers_other {'
[ "$(grep -cxF "$kept" src/escape.rs)" -eq 1 ] || {
  echo "the arm check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if answers_other {$/        if answers_other \&\& false {/' src/escape.rs
if grep -qxF "$kept" src/escape.rs; then exit 1; fi
