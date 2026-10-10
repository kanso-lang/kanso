#!/bin/sh
# Times both loops on a release build. KANSO points at the compiler.
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")"
for which in inline_n named_n; do
  for n in 1000 2000 4000; do
    dir=$(mktemp -d)
    printf 'import "%s/probe"\n\nprint "{probe/%s %s}"\n' "$(pwd)" "$which" "$n" > "$dir/main.kso"
    (cd "$dir" && "$KANSO" build main.kso --release > /dev/null)
    start=$(date +%s.%N)
    timeout 30 "$dir/main" > /dev/null
    stop=$(date +%s.%N)
    echo "$which $n: $(echo "$stop - $start" | bc)s"
    rm -rf "$dir"
  done
done
