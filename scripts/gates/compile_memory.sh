#!/bin/sh
set -e
KANSO_COUNTERS=1 ./target/release/kanso check lib/json 2>counters_compile.txt >/dev/null
for k in rounds visits; do
  got=$(grep "^compile_${k}=" counters_compile.txt | cut -d= -f2)
  want=$(grep "^front_end_${k}=" bench/compile_memory_golden.txt | cut -d= -f2)
  if [ "$got" != "$want" ]; then
    echo "::error::the front end's ${k} on lib/json moved: ${want} -> ${got}."
    echo "::error::that is a welfare term. if intentional, regenerate"
    echo "::error::bench/compile_memory_golden.txt and say which way it"
    echo "::error::went in design/compiler-log.md."
    exit 1
  fi
done
got=$(grep '^compile_peak_bytes=' counters_compile.txt | cut -d= -f2)
want=$(grep '^compile_peak_bytes=' bench/compile_memory_golden.txt | cut -d= -f2)
margin=$((want / 50))
low=$((want - margin))
high=$((want + margin))
echo "front end holds ${got} bytes; golden ${want}, band ${low}..${high}"
if [ "$got" -lt "$low" ] || [ "$got" -gt "$high" ]; then
  echo "::error::what the front end holds while checking lib/json moved"
  echo "::error::more than two per cent: ${want} -> ${got}. that is a"
  echo "::error::welfare term. if intentional, regenerate"
  echo "::error::bench/compile_memory_golden.txt in this PR and say which"
  echo "::error::way it went in design/compiler-log.md."
  exit 1
fi
