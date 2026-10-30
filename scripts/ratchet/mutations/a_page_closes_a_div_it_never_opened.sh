#!/bin/sh
# A book page closes a <div> it never opened.
#
# The modules chapter shipped with a stray </div> after a code panel, and appendix a carried
# two more. The browser forgives it, so the page looked right, and every gate
# passed over it. The mutation puts one back where the modules chapter's was: after the first
# code panel closes.
set -e
page=docs/book/modules.html
grep -q '^</div>$' "$page" || {
  echo "no panel closes on a line of its own in $page; this mutation needs" >&2
  echo "rewriting" >&2
  exit 1
}
sed -i '0,/^<\/div>$/s//<\/div>\n<\/div>/' "$page"
