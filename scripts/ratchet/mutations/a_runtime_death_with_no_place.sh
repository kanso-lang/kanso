#!/bin/sh
# A run-time failure names the line it happened on (#1975): compiled code
# stamps a site before a call that can stop the program, and `k_died` prints
# it. This mutation stops printing it, so every placed runtime golden loses its
# `failed in` line on native and the engines disagree.
set -e
line='    if (site) {'
[ "$(grep -cxF "$line" src/runtime/core.c)" -eq 1 ] || {
  echo "k_died's print moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (site) {$/    if (0 \&\& site) {/' src/runtime/core.c
if grep -qxF "$line" src/runtime/core.c; then exit 1; fi
