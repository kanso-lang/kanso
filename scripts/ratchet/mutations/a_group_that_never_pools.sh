#!/bin/sh
# A group whose sides all wait runs the work-ahead pool, as a bare sleep
# does. This mutation drops the call at the group's idle point, so the lifted
# call runs where it is read, and the mem golden's `ahead_forced=1` goes to 0.
set -e
[ "$(grep -c "an idle point runs the work-ahead pool" src/runtime/effects.c)" -eq 1 ] || {
  echo "the group's pool call moved; this mutation needs rewriting" >&2
  exit 1
}
[ "$(grep -c 'k_ahead_pool(first);' src/runtime/effects.c)" -eq 2 ] || exit 1
sed -i "/an idle point runs the work-ahead pool/,/^#endif/ s/k_ahead_pool(first);/(void)first;/" src/runtime/effects.c
[ "$(grep -c 'k_ahead_pool(first);' src/runtime/effects.c)" -eq 1 ] || exit 1
