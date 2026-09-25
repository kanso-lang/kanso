#!/bin/sh
# The literal a match must hold is a run of plain characters side by side in
# the pattern's sequence, and any other part ends the run. This mutation lets
# the run carry across the part, so `ab.cd` asks for "abcd", a subject holding
# "abXcd" is judged to hold no match, and the scan is never made.
#
# a_match_holds_the_literal_its_pattern_spells prints "across:" with nothing
# after it where it printed "abXcd".
set -e
line='  literal_run parts (at + 1) "" (longer run best)'
grep -qF "$line" lib/regexp/regexp.kso || {
  echo "the literal run's reset changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^  literal_run parts (at + 1) "" (longer run best)$|  literal_run parts (at + 1) run best|' lib/regexp/regexp.kso
grep -qF '  literal_run parts (at + 1) run best' lib/regexp/regexp.kso
