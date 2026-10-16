#!/bin/sh
# An insisting form names its caller's line (#1975): compiled code hands the
# form's answer through `k_b_insisted`, which points an err born in the form's
# package at the call. This mutation hands the answer back untouched, so the
# err names the library's line on native and the engines disagree.
set -e
line='    if (answer.tag != K_DESC) return k_insisted_err(answer, site);'
[ "$(grep -cxF "$line" src/runtime/effects.c)" -eq 1 ] || {
  echo "k_b_insisted moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (answer.tag != K_DESC) return k_insisted_err(answer, site);$/    return answer;/' src/runtime/effects.c
if grep -qxF "$line" src/runtime/effects.c; then exit 1; fi
