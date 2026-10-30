#!/bin/sh
# A panel title naming a command the reader's toolchain refuses.
#
# currying.kso declares a function beside its statements, so a reader runs it
# with `kanso play`; `kanso run` answers that it is a library. The mutation
# puts the run title back, which is how dozens of panels read until the
# harness held titles to the verb it used.
set -e
page=docs/book/effects.html
if ! grep -q 'code-panel-title">kanso play currying.kso' "$page"; then
  echo "expected a \`kanso play currying.kso\` panel in $page; the shape moved" >&2
  echo "and this mutation needs rewriting" >&2
  exit 1
fi
sed -i 's|code-panel-title">kanso play currying.kso|code-panel-title">kanso run currying.kso|' "$page"
