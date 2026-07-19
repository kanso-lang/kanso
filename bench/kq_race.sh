#!/bin/sh
# kq vs jq, interleaved, whole-process wall time. Verifies byte-identity
# before racing — a fast wrong answer is not a result.
set -e
./target/release/kanso build apps/kq --release >/dev/null
python3 - <<'PY'
import json, subprocess, time, statistics as st
d = json.load(open('bench/large.json'))
json.dump(d*10, open('/tmp/kq_big.json','w'), separators=(',',':'))
def gate(q, f):
    a = subprocess.run(['./kq', q, f], capture_output=True).stdout
    b = subprocess.run(['jq', '-S', q, f], capture_output=True).stdout
    assert a == b, f"kq and jq disagree on {q} {f}"
def t(cmd):
    x = time.perf_counter(); subprocess.run(cmd, capture_output=True)
    return (time.perf_counter() - x) * 1000
for q, f, n in [('.[0].k0_30', 'bench/large.json', 25), ('.[0].k0_30', '/tmp/kq_big.json', 15),
                ('.', 'bench/large.json', 25), ('.', '/tmp/kq_big.json', 15)]:
    gate(q, f)
    kq, jq = [], []
    for _ in range(n):
        kq.append(t(['./kq', q, f])); jq.append(t(['jq', '-S', q, f]))
    wins = sum(1 for a, b in zip(kq, jq) if a < b)
    print(f"{q:12} {f:22} kq {min(kq):6.1f}ms  jq {min(jq):6.1f}ms  "
          f"({min(jq)/min(kq):.2f}x, kq wins {wins}/{n})")
PY
