#!/bin/sh
# Rounds and visits count the compiler's own algorithm, so they are the same on
# every host and are checked everywhere. Peak bytes belongs to the host that
# measured it, so it is checked only where its measured-on line says it may be,
# and exactly — Clay ruled tolerance bands away on 2026-08-24.
set -e
golden=bench/compile_memory_golden.txt
KANSO_COUNTERS=1 ./target/release/kanso check lib/json 2>counters_compile.txt >/dev/null
for k in rounds visits; do
  got=$(grep "^compile_${k}=" counters_compile.txt | cut -d= -f2)
  want=$(grep "^front_end_${k}=" "$golden" | cut -d= -f2)
  if [ "$got" != "$want" ]; then
    echo "::error::the front end's ${k} on lib/json moved: ${want} -> ${got}."
    echo "::error::that is a welfare term. if intentional, regenerate"
    echo "::error::$golden and say which way it went in"
    echo "::error::design/compiler-log.md."
    exit 1
  fi
done
sh scripts/gates/measured_on.sh "$golden"
got=$(grep '^compile_peak_bytes=' counters_compile.txt | cut -d= -f2)
want=$(grep '^compile_peak_bytes=' "$golden" | cut -d= -f2)
echo "front end holds ${got} bytes; golden ${want}"
if [ "$got" != "$want" ]; then
  echo "::error::what the front end holds while checking lib/json moved:"
  echo "::error::${want} -> ${got}. That is a welfare term, and this row is"
  echo "::error::exact for the host its measured-on line names, so any"
  echo "::error::difference is a real one. A rise is a regression to explain"
  echo "::error::and a fall is a win to bank — say which in"
  echo "::error::design/compiler-log.md, put ${got} in $golden, and move the"
  echo "::error::welfare floor with --set in the same PR."
  exit 1
fi
