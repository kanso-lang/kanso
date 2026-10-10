#!/bin/sh
# Times the two loops separately. KANSO points at the compiler.
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")"
for which in bare_n boxed_n; do
  dir=$(mktemp -d)
  printf 'import "%s/probe"\n\nprint "{probe/%s 20000}"\n' "$(pwd)" "$which" > "$dir/main.kso"
  (cd "$dir" && "$KANSO" build main.kso --release > /dev/null)
  start=$(date +%s.%N)
  "$dir/main" > /dev/null
  stop=$(date +%s.%N)
  echo "$which 20000: $(echo "$stop - $start" | bc)s"
  rm -rf "$dir"
done
