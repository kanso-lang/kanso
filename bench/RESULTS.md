# benchmark results

All numbers from an Apple M-series laptop, macOS, clang -O3 -flto for the
native backend. The workload is `bench/large.json` (188 KB of deeply nested
generated JSON) decoded by kanso-json; the rival is Go's `encoding/json`
(`bench/main.go`), the standard library of the language famous for being fast
at exactly this.

## the headline

| engine | per decode |
| --- | --- |
| **kanso (LLVM backend)** | **~1.6 ms** |
| go `encoding/json` | ~1.88 ms |
| kanso (reference interpreter) | ~350 ms |

The interpreter is the semantics oracle, not the product; CI holds both
engines to byte-identical output on every golden test.

## how it got there (~350 ms → ~1.6 ms, in commit order)

1. LLVM backend replacing tree-walking
2. arena allocation, string interning, LTO
3. `tailcc` + `musttail`: real tail-call elimination everywhere
4. switch jump tables for literal dispatch
5. zero-copy byte views + an escape-free string fast path
6. whole-program inference feeding typed emission — tag checks and
   failure guards deleted on proven paths

The final profile shows only `memmove` and map insertion — no dispatch
machinery, no tag checks, no failure guards on proven-pure paths. Ten million
frames of mutual recursion run in constant stack (`musttail` is load-bearing,
not decorative).

## kq vs jq (1.7.1)

Byte-identical output to `jq -S` across the whole 188 KB document. Wall clock
per invocation including process startup, 50 runs:

| task | kq | jq |
| --- | --- | --- |
| path extraction (`.[0].k0_30…`) | **~3.9 ms** | ~4.4 ms |
| full-document pretty-print | ~12 ms | ~12 ms |

jq is mature C. kq is a few hundred lines of kanso written in one evening.

## still on the shelf

Perceus reference counting, a real map structure (inserts are O(n) today),
SIMD scanning, salsa-style incremental compilation, parallel `&`. The current
numbers were reached without any of these.

## reproduce

```sh
bench/make_jsonbench.sh          # regenerates bench/jsonbench from lib/json
cargo build --release
./target/release/kanso build bench/jsonbench
time ./jsonbench                 # 150 decodes
go run bench/main.go             # the rival
```
