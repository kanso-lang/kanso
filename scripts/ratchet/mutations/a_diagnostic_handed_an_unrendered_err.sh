#!/bin/sh
# The destructure diagnostic renders its value the way a program does.
#
# A plain render hands an err back unrendered, so the diagnostic read the err
# as a string and crashed, and the crash was reported as running out of
# stack. an_err_cannot_be_destructured sees native's stderr.
set -e
f=src/runtime.c
grep -q "static KValue k_shown" src/runtime.c
line='    KValue shown = k_shown(v, 1);'
[ "$(grep -cxF "$line" "$f")" -eq 2 ] || {
  echo "the diagnostics' renders moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    KValue shown = k_shown(v, 1);$/    KValue shown = k_render(v, 1);/' "$f"
[ "$(grep -cxF '    KValue shown = k_render(v, 1);' "$f")" -eq 2 ]
