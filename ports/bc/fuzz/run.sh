#!/bin/sh
# Differential testing against GNU bc. Each generator writes a random bc
# program for a seed; the program runs through GNU bc and through this
# port's release build, and any batch whose stdout or stderr differs is
# reported and kept in the scratch directory.
#
#   sh fuzz/run.sh arith 1 100      seeds 1..100 of fuzz/arith.py
#   sh fuzz/run.sh mathlib 1 30     the -l library
#   sh fuzz/run.sh bases 1 40       numbers read in random ibases
#
# Known differences, which README.md describes: GNU's negative zero (its
# "-0" lines are read as "0" here, but a comparison or sqrt of one still
# differs), and last-digit errors in GNU's library at low scales.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")/.." && pwd)
cd "$here" || exit 1
kind=$1; first=$2; last=$3
tmp=$(mktemp -d)
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/port"
rm -f main.ll
flags=""
[ "$kind" = mathlib ] && flags="-l"
size=200
[ "$kind" = mathlib ] && size=60
[ "$kind" = bases ] && size=100
bad=0
for seed in $(seq "$first" "$last"); do
  python3 "fuzz/$kind.py" "$seed" "$size" "$tmp/case.bc"
  bc -q $flags "$tmp/case.bc" < /dev/null > "$tmp/gnu" 2> "$tmp/gnu.err"
  timeout 120 "$tmp/port" $flags "$tmp/case.bc" < /dev/null \
    > "$tmp/port.out" 2> "$tmp/port.err"
  sed -i -e 's/adr=[0-9]*/adr=0/' "$tmp/gnu.err"
  sed -i -e 's/^-0$/0/' "$tmp/gnu"
  if ! cmp -s "$tmp/gnu" "$tmp/port.out" || ! cmp -s "$tmp/gnu.err" "$tmp/port.err"
  then
    bad=$((bad + 1))
    echo "seed $seed differs; kept as $tmp/$kind-$seed.bc"
    cp "$tmp/case.bc" "$tmp/$kind-$seed.bc"
    diff "$tmp/gnu" "$tmp/port.out" | head -6
  fi
done
echo "$bad of $((last - first + 1)) batches differ"
