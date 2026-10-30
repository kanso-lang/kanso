#!/bin/sh
# A panel quoting a library file shows a declaration the file does not hold
# as written.
#
# the json chapter's excerpts once carried an underscore on every private name and a
# type on every field, and every gate passed them, because the check read
# only which names a panel used. The mutation puts an underscore back on
# `finish` in the api panel, a name the library also declares, so only the
# verbatim rule can see it.
set -e
page=docs/book/json.html
if ! grep -q '<span class="f">finish</span> _ <span class="k">none</span>' "$page"; then
  echo "expected json.kso's \`fn finish _ none\` in $page; the shape moved and" >&2
  echo "this mutation needs rewriting" >&2
  exit 1
fi
sed -i 's|<span class="f">finish</span> _ <span class="k">none</span>|<span class="f">_finish</span> _ <span class="k">none</span>|' "$page"
