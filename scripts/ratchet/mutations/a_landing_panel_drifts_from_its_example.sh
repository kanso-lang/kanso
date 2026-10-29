#!/bin/sh
# A landing-page panel quoting an example the example no longer matches.
#
# index.html says its samples are verbatim from examples/, and for months five
# of seven were not: they still showed `main =` and typed record fields after
# the language retired both. The mutation puts `main =` back on dispatch.kso's
# panel.
set -e
page=docs/index.html
if ! grep -q '^answer <span class="o">=</span> <span class="f">fact</span> <span class="n">20</span>$' "$page"; then
  echo "expected dispatch.kso's \`answer = fact 20\` line in $page; the shape" >&2
  echo "moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i 's|^answer <span class="o">=</span> <span class="f">fact</span> <span class="n">20</span>$|main <span class="o">=</span>\n  answer <span class="o">=</span> <span class="f">fact</span> <span class="n">20</span>|' "$page"
