#!/bin/sh
# The measurement behind FRICTION.md F21: ten million vec3 additions in a
# kanso release build against the same loop in C, and ten million rounds of
# three scalar float additions for scale.
set -eu
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")"
"$KANSO" build main.kso --release > /dev/null
clang -O2 vecs.c -o vecs_c
for what in floats vecs; do
  start=$(date +%s.%N)
  ./main "$what" > /dev/null
  echo "kanso $what: $(echo "$(date +%s.%N) - $start" | bc) s"
done
start=$(date +%s.%N)
./vecs_c > /dev/null
echo "c vecs: $(echo "$(date +%s.%N) - $start" | bc) s"
rm -f main main.ll vecs_c
