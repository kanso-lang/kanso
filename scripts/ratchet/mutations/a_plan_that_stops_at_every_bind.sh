#!/bin/sh
# The plan shows every bind as a continuation, including `_ -> step`.
#
# A callback that ignores its argument builds the same description whatever it
# is handed, so `--plan` calls it and renders the step. Without that the plan
# of examples/effects.kso stops after its first print.
set -e
f=src/eval.rs
grep -q 'pub fn ignoring_step' src/eval.rs
line='        if !ignored {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the plan's ignoring step moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if !ignored {$/        if true {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
