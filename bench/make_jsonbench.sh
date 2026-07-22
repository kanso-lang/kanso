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
blocks = [b for b in s.strip().split('\n\n') if not b.startswith('fn failure_position')]
open('bench/jsonbench/json.kso', 'w').write('\n\n'.join(blocks) + '\n')
lib = (
    'fn go cs\n'
    '  print "decoded 150 times, checksum {loop cs 150 0}"\n\n'
    'fn loop _ 0 acc\n'
    '  acc\n\n'
    'fn loop cs n acc\n'
    '  loop cs (n - 1) (acc + (length (decode cs)))\n'
)
open('bench/jsonbench/bench.kso', 'w').write(lib)
open('bench/jsonbench/main.kso', 'w').write('import "std/io"\n\nio/read_file "bench/large.json" . go\n')
PY
echo "bench/jsonbench ready (150x runtime-read; run ./jsonbench from repo root)"
