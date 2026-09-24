#!/bin/sh
# A greedy repetition of one character is counted in one pass and then backed
# off in order. The count stops at the repetition's ceiling. This mutation
# drops that stop, so `x{2,3}x` on "xxxxx" takes all five characters where it
# takes four, and the micro fixture
# a_greedy_run_of_one_character_backs_off_in_order reads the wrong match on
# both engines, since both run the library.
set -e
grep -qF '  return n if most >= 0 and n >= most' lib/regexp/regexp.kso || {
  echo "the counted run's ceiling moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^  return n if most >= 0 and n >= most$|  return n if false|' lib/regexp/regexp.kso
[ "$(grep -cF '  return n if false' lib/regexp/regexp.kso)" -eq 1 ]
