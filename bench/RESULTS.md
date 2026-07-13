# benchmark results

All numbers: Apple M-series laptop, macOS, `clang -O3 -flto` for kanso's native
backend, `rustc -O` with LTO for serde, Go's default compiler. The workload is
`bench/large.json` — 188 KB of deeply nested generated JSON (160 top-level
values) — decoded into an untyped value.

## methodology (read this before quoting a number)

Every engine **reads the file at runtime** and decodes it **150 times in a
loop**, reporting the mean. This matters:

- Reading at runtime keeps the input opaque to the optimizer. An earlier
  version of this harness embedded the JSON as a compile-time string constant,
  which let LLVM partially fold the decode and produced an optimistic ~1.6 ms
  that does not represent real I/O-sourced JSON. The honest runtime-read
  number is higher. We corrected it rather than quote the flattering artifact.
- kanso's harness accumulates a checksum across all 150 decodes
  (`160 × 150 = 24000`), so the loop provably runs — no decode is hoisted or
  dead-code-eliminated.

Reproduce:

```sh
cargo build --release                          # kanso toolchain
bash bench/make_jsonbench.sh                    # generates the 150x harness
./target/release/kanso build bench/jsonbench --release
time ./jsonbench                                # divide wall time by 150
(cd bench/serde_bench && cargo build --release) # the rival
./bench/serde_bench/target/release/serde_bench bench/large.json
go run bench/main.go                            # the other rival
```

## the honest scoreboard (mean per decode, best of 5)

| engine | per decode | vs kanso |
| --- | --- | --- |
| **serde_json** (Rust) | **~0.86 ms** | **2.1× faster** |
| **kanso** (LLVM backend) | ~1.80 ms | — |
| go `encoding/json` | ~1.90 ms | 1.05× slower |
| kanso (reference interpreter) | ~350 ms | oracle, not the product |

What this says plainly: **kanso beats Go's standard library, and serde — the
JSON parser a Rust team actually ships — is about twice as fast as kanso
today.** That gap is the whole point of the optimization campaign; serde is the
number to beat, and it is now in `bench/` so every future step is measured
against the real target instead of the opponent we already pass.

The interpreter is the semantics oracle, not the product; CI holds every engine
byte-identical on the golden corpus (floats included — every engine renders the
shortest round-trip that survives re-parsing).

## why serde is ahead, and how the gap closes

serde wins today because kanso still allocates where serde doesn't. The
profiler's hot spots are `memmove` and `k_b_put` (map insertion): building an
n-key JSON object costs O(n²) allocation and copying, because each `put`
copies the whole map. Nothing about that is fundamental — it is the reference
counting and reuse analysis that have not shipped yet:

- **Perceus + reuse** (Koka's technique, shipped by Lean 4 and Roc): a map with
  one owner is updated in place, so object construction stops copying. This is
  the single biggest unplayed card and it directly kills the profiler's #1 hot
  spot.
- **owned/borrowed parameter modes** and **static uniqueness**: delete most
  reference-count traffic outright.

See `design/performance-frontier.md` for the full pipeline and the staged,
falsifiable claim: match serde on this gauntlet after Perceus + layout, then
win on multi-document and irregular workloads via free parallelism. Kanso's
edge is not inventing these techniques but being a stricter host for them —
no shadowing, nothing wasted, no observable identity make the analyses land
more often.

## how kanso got to 1.80 ms (in commit order)

1. LLVM backend replacing tree-walking (~350 ms → tens of ms)
2. arena allocation, string interning, LTO
3. `tailcc` + `musttail`: real tail-call elimination everywhere
4. switch jump tables for literal dispatch
5. zero-copy byte views + an escape-free string fast path
6. whole-program inference feeding typed emission — tag checks and failure
   guards deleted on proven paths

The reference-counting tier (Perceus, uniqueness, modes) is the next chapter,
and it is the one that targets serde.
