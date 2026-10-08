#!/bin/sh
# Index every qualified declaration by its site again, where the pass now keeps
# only the sites a synthetic declaration stands at. The extra entries are never
# asked about, so the aliases and the output are unchanged, and the witness is
# the front end's peak: the index was the largest thing alive when it peaked.
set -e
target='        if let Some(entry) = at_site.get_mut(&site(twin)) {'
n=$(grep -cF "$target" src/lib.rs)
[ "$n" -eq 1 ] || { echo "the twin lookup moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk -v t="$target" '
  $0 == t { print "        if let Some(entry) = Some(at_site.entry(site(twin)).or_insert((None, Vec::new()))) {"; next }
  { print }
' src/lib.rs > src/lib.rs.mut && mv src/lib.rs.mut src/lib.rs
