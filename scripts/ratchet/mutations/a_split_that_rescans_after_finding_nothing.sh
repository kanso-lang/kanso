#!/bin/sh
# A split whose scan finds nothing ends there, since the scan has looked at
# every start after it. This mutation scans again from the next position, the
# way split did before 2026-09-25, which is quadratic in what follows the last
# separator.
#
# a_split_stops_where_the_separator_does reads beat_iters=45,453 against 303.
set -e
line='  return push acc (rest_of s from) if m.from == 0'
grep -qF "$line" lib/regexp/regexp.kso || {
  echo "the split's stop changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^  return push acc (rest_of s from) if m.from == 0$|  return cut prog s from (at + 1) acc if m.from == 0|' lib/regexp/regexp.kso
grep -qF '  return cut prog s from (at + 1) acc if m.from == 0' lib/regexp/regexp.kso
