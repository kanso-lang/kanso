#!/bin/sh
# Regenerates bc/mathlib.kso from lib/math.bc. kanso has no multi-line
# string, so the library is embedded one line per `push` (FRICTION F2).
# Comments and blank lines are left out of the embedded copy. check.sh
# fails when the two disagree.
here=$(cd "$(dirname "$0")" && pwd)
{
  echo '# Generated from lib/math.bc by embed_mathlib.sh; edit that file instead.'
  echo 'import "std/text"'
  echo
  printf '%s\n' 'pub library_source = text/join library_lines "\n"'
  echo
  echo 'library_lines = []'
  awk '
    open { if (index($0, "*/")) open = 0; next }
    /^\/\*/ { if (!index($0, "*/")) open = 1; next }
    $0 != "" { print }
  ' "$here/lib/math.bc" |
  sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/{/\\{/g' \
      -e 's/^\(.*\)$/  . push "\1"/'
} > "$here/bc/mathlib.kso"
