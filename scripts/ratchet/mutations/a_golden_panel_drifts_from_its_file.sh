#!/bin/sh
# A panel titled with a golden's path shows something the golden does not say.
#
# about.html quotes three error goldens by their paths, and each had drifted
# to a stale message or line number before anything compared them. The
# mutation rewords one of them on the page.
set -e
page=docs/about.html
said='imports appear in alphabetical order'
grep -qF "$said" "$page" || {
  echo "the import-order panel moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|imports appear in alphabetical order|imports appear in sorted order|' "$page"
