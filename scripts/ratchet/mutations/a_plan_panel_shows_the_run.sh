#!/bin/sh
# A plan panel showing what the program prints instead of its plan.
#
# The mutation puts brew.kso's run output under its --plan title, which is
# what the harness wrote there while it matched plan panels against brew.out.
set -e
page=docs/book/effects.html
if ! grep -q 'kanso run brew.kso --plan</div>' "$page"; then
  echo "expected a brew.kso --plan panel in $page; the shape moved and this" >&2
  echo "mutation needs rewriting" >&2
  exit 1
fi
perl -0pi -e 's{(kanso run brew\.kso --plan</div>\s*<pre><code>).*?(</code></pre>)}{$1brewing\nsteeping\npouring$2}s' "$page"
