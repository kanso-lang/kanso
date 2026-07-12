#!/bin/sh
# Rebuild bench/jsonbench: the json module plus a main embedding bench/large.json.
set -e
mkdir -p bench/jsonbench
cp lib/json/json.kso lib/json/number.kso lib/json/scan.kso lib/json/text.kso lib/json/value.kso bench/jsonbench/
python3 - <<'PY'
s = open('bench/jsonbench/json.kso').read()
blocks = [b for b in s.strip().split('\n\n') if not b.startswith('fn _failure_position')]
open('bench/jsonbench/json.kso', 'w').write('\n\n'.join(blocks) + '\n')
text = open("bench/large.json").read()
esc = text.replace("\\", "\\\\").replace('"', '\\"').replace("{", "\\{").replace("\n", "\\n").replace("\t", "\\t")
src = 'fixture = "' + esc + '"\n\nmain = print "decoded {length (decode fixture)} top-level values"\n'
open("bench/jsonbench/main.kso", "w").write(src)
PY
echo "bench/jsonbench ready"
