#!/bin/sh
# A panel quoting a directory module's run that the module no longer produces.
#
# book_panels resolved only titles naming a .kso file, so `kanso run sealed`
# went unread and the page kept a diagnostic the compiler had since reworded.
# The mutation puts that old wording back.
set -e
page=docs/book/modules.html
if ! grep -q 'kanso run sealed</div>' "$page"; then
  echo "expected a \`kanso run sealed\` panel in $page; the shape moved and" >&2
  echo "this mutation needs rewriting" >&2
  exit 1
fi
perl -0pi -e 's{(kanso run sealed</div>\s*<pre><code>.*?)resort/ride_km}{$1ride_km}s' "$page"
