#!/bin/sh
# A book panel that needs a filesystem offered a run button in the tab.
#
# book-play.js keeps a list of the calls a tab cannot answer. It named them
# io/read_file and io/args for months after they moved to os/, so every panel
# reading a file grew a button that could only fail. The mutation puts the
# old module back.
set -e
js=docs/book-play.js
if ! grep -q 'os\\/(?:read_file' "$js"; then
  echo "expected the os/ guard in $js; the shape moved and this mutation" >&2
  echo "needs rewriting" >&2
  exit 1
fi
sed -i 's|os\\/(?:read_file|io\\/(?:read_file|' "$js"
