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

## the honest scoreboard (mean per decode, best of 5; 2026-07-14)

| engine | per decode | max RSS | notes |
| --- | --- | --- | --- |
| **serde_json** (Rust) | **~0.94 ms** | 7.1 MB | hand-tuned: SIMD scan, zero-copy |
| **kanso + heartbeat** | **~1.00 ms** | **12.4 MB, flat** | experimental — see below |
| **naive Rust** (`bench/naive_json`) | ~1.11 ms | 7.1 MB | the fair stick: same algorithm as kanso |
| **kanso** (shipped default) | ~1.26 ms | grows unboundedly | LLVM backend, arena never freed |
| go `encoding/json` | ~1.90 ms | — | |
| kanso (reference interpreter) | ~350 ms | — | oracle, not the product |

Two sticks, on purpose. `serde_json` is expert-hand-rolled (SIMD byte scanning,
zero-copy borrows, buffer reuse), so racing it measures hand-tuning, not the
language. `bench/naive_json` is the fair stick — recursive descent into an
enum, std `String`/`Vec`/`HashMap`, the same algorithm kanso's decoder uses,
written the way a competent Rust dev writes one in an afternoon.

**kanso's shipped default beats Go by ~1.5× and sits ~13% behind naive Rust.
The heartbeat experiment beats naive Rust by ~10%** — details below.

### the heartbeat experiment (2026-07-14)

The runtime arena is now block-chained and beat-resettable (`k_beat_boundary`
in `runtime.c`): the first call snapshots the arena frontier, every later call
rewinds to it, recycling retired blocks through a spare pool. For this
experiment the boundary was **hand-inserted into the compiled IR** between
decode iterations — sound for this loop because only an integer survives each
beat, verified by checksum. Nothing in the shipped compiler emits boundaries
yet, which is why the row above says experimental.

Receipts (150 decodes / 3000 decodes):

- max RSS **800 MB → 12.4 MB** (65×), and *flat*: still 12.4 MB at 3000 decodes
- wall time **0.19 s → 0.15 s** (~20% faster); sys time 0.03 s → 0.00 s — the
  kernel stopped faulting in fresh pages; the loop recycles warm ones

The speedup is the thesis in miniature: Rust pays malloc/free per object
unless its programmer hand-builds an arena; kanso's beat structure makes the
arena automatic and sound. The compiler-emitted version (gated on escape
analysis proving nothing survives the beat) is the next rung; the gentle
walkthrough with diagrams is on the site's compiler page, §11.

The interpreter is the semantics oracle, not the product; CI holds every engine
byte-identical on the golden corpus (floats included — every engine renders the
shortest round-trip that survives re-parsing).

## how the shipped default got from 1.80 ms to 1.26 ms

Unboxed dispatch (byte discriminators cross musttail edges raw), Perceus-style
in-place push for uniquely-owned list builders, register-returned `%parsed`
records, and a sweep of provably-redundant copies (utf8 built every string
twice; concat/entries/chars each allocated twice). The old diagnosis in this
file — "building an n-key object copies the whole map on every insert, O(n²)"
— is dead: maps append to a frontier-shared buffer in O(1) and sort once on
first read. What remains between kanso and serde is allocation traffic and
dispatch overhead, which the heartbeat model (compiler page §10–§11,
`design/memory-model-committee.md`) attacks structurally.

## how kanso got to 1.80 ms in the first place (in commit order)

1. LLVM backend replacing tree-walking (~350 ms → tens of ms)
2. arena allocation, string interning, LTO
3. `tailcc` + `musttail`: real tail-call elimination everywhere
4. switch jump tables for literal dispatch
5. zero-copy byte views + an escape-free string fast path
6. whole-program inference feeding typed emission — tag checks and failure
   guards deleted on proven paths

The next chapter is the heartbeat: compiler-emitted beat boundaries, then
copy-or-pin for survivors — no reference counting, no per-object header,
freeing always a pointer reset.
