#!/bin/sh
# A book panel that quotes a file the repository owns may not name something
# that file cannot.
#
# scripts/book_panels regenerates the panels whose title resolves to a sample
# it owns and leaves the rest alone. Eight ch08 panels titled `lib/json/*.kso`
# fell in the gap, and four of them had drifted: one showed `at cs p` for an
# index the language no longer spells that way, one named `_is_ws` for a
# function since renamed `ws?`, and two named byte-array constants the library
# had replaced. Every gate in the tree was green the whole time.
#
# The mutation renames one declaration in the library and leaves the panel
# quoting the old name, which is exactly the shape that drifted.
set -e
lib=lib/json/scan.kso
if ! grep -q '^fn ws? none$' "$lib"; then
  echo "expected `fn ws? none` in $lib; the shape moved and this" >&2
  echo "mutation needs rewriting" >&2
  exit 1
fi
sed -i 's/\bws?/blank?/g' "$lib"
