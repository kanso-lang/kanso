#!/bin/sh
# Rebuild bench/jsonbench: the json module plus a main that reads
# bench/large.json AT RUNTIME and decodes it 150 times, accumulating a
# checksum so the loop provably runs (no hoisting / dead-code elimination).
# Runtime read keeps the input opaque to the optimizer — an embedded
# compile-time fixture would let LLVM fold the decode and flatter the number.
# Run the built ./jsonbench from the repo root; divide its wall time by 150.
set -e
mkdir -p bench/jsonbench
cp lib/json/json.kso lib/json/number.kso lib/json/scan.kso lib/json/text.kso lib/json/value.kso bench/jsonbench/
python3 - <<'PY'
s = open('bench/jsonbench/json.kso').read()
blocks = [b for b in s.strip().split('\n\n') if not b.startswith('fn _failure_position')]
open('bench/jsonbench/json.kso', 'w').write('\n\n'.join(blocks) + '\n')
main = (
    'fn _bench _ 0 acc\n'
    '  acc\n\n'
    'fn _bench cs n acc\n'
    '  _bench cs (n - 1) (acc + (length (decode cs)))\n\n'
    'fn _run cs:string\n'
    '  print "decoded 150 times, checksum {_bench cs 150 0}"\n\n'
    'main = read_file "bench/large.json" . _run\n'
)
open('bench/jsonbench/main.kso', 'w').write(main)
PY
echo "bench/jsonbench ready (150x runtime-read; run ./jsonbench from repo root)"
