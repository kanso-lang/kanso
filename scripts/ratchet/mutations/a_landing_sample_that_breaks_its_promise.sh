#!/bin/sh
# The landing page's sample no longer produces what the page prints under it.
#
# docs/index.html shows an editable program and, beside it, the output the page
# promises it produces. Those two are written by hand and nothing but this job
# checks that they still agree: site_smoke loads the page in a browser, clicks
# run, and requires `hello, kanso` in the output element.
#
# Changing the greeting in the sample leaves the promise standing and makes it
# false, which is the shape of the mistake — an edit to one half of a pair that
# only a browser can compare.
set -e
f=docs/index.html
grep -qF '  "hello, {name}"' "$f" || {
  echo "the landing sample moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^  "hello, {name}"$|  "goodbye, {name}"|' "$f"
grep -qF '  "goodbye, {name}"' "$f"
