#!/bin/sh
# The tab's bcmp answers equality alone, from a body of its own. This
# mutation gives it memcmp's ordering body again, and with one body under two
# names every compare went through memcmp into bcmp. Same answers, and the
# browser compile row reads heavier.
set -e
t='    unsafe { kanso::compare::differ(a, b, n) }'
[ "$(grep -cxF "$t" src/main.rs)" -eq 1 ] || {
  echo "the tab's bcmp moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    unsafe { kanso::compare::differ(a, b, n) }$/    unsafe { kanso::compare::bytes(a, b, n) }/' src/main.rs
if grep -qxF "$t" src/main.rs; then exit 1; fi
