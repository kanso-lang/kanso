#!/bin/sh
# A group whose sides all wait runs the work-ahead pool, as a bare sleep
# does. This mutation drops the call at the group's idle point, so the lifted
# call runs where it is read, and the mem golden's `ahead_forced=1` goes to 0.
set -e
[ "$(grep -c "group's first idle point runs the" src/runtime/effects.c)" -eq 1 ] || {
  echo "the group's pool call moved; this mutation needs rewriting" >&2
  exit 1
}
[ "$(grep -c 'k_ahead_pool();' src/runtime/effects.c)" -eq 2 ] || exit 1
sed -i "/group's first idle point runs the/,/^#endif/ s/k_ahead_pool();/(void)0;/" src/runtime/effects.c
[ "$(grep -c 'k_ahead_pool();' src/runtime/effects.c)" -eq 1 ] || exit 1
