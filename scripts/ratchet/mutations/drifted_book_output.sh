#!/bin/sh
# A recorded sample output that the sample no longer produces.
set -e
out=docs/book/samples/ch01/greeting.out
test -f "$out" || out=$(ls docs/book/samples/*/*.out | head -1)
printf 'drifted\n' >> "$out"
