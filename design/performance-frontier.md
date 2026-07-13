# The performance frontier — standing queue, draft 0.1

Clay's standing directive (2026-07-12): core language changes are on the
table **iff** they don't meaningfully tax user experience **and** the payoff
is game-changing. The goal stated plainly: faster than Rust, without making
the user think about lifetimes — the language's constraints should force the
right representation instead of asking for it. Every item below is scored
against that bar.

The unifying observation: kanso's purity rules aren't a tax the optimizer
tolerates — they are the license the optimizer runs on. Each entry names the
semantic property that makes the technique sound here when it's hard
elsewhere.

## 1. Perceus + reuse analysis (Koka lineage) — QUEUED NEXT

Precise, compiler-inserted reference counting with reuse: refcount-1 values
are updated in place, so persistent-looking code runs at imperative speed
("functional but in place"). License: no mutation, tree-shaped values → no
cycles, no cycle collector, none of Koka's caveats. Replaces the
never-freeing arena. Targets: map inserts (currently O(n) copies), JSON
number building, every builder loop. This is the main unplayed card against
the 1.6ms decode number — and the core of the faster-than-Rust thesis:
idiomatic Rust clones where the borrow checker frustrates; Perceus reuses.

## 2. AOT speculation with restart — OURS; generalize past ints

The int tiering gavel (fast i64 version bails and restarts into the bignum
version; sound because functions are pure, so restart is unobservable) is a
special case of a general mechanism: speculate on any probable-but-unproven
inference fact — monomorphic dispatch target, list-not-map, ASCII-not-UTF8 —
with bail-and-restart. JIT-engine speculation, ahead of time, with zero
deoptimization metadata. Possibly publishable. Design the int implementation
as the general mechanism from day one.

## 3. Automatic amortized resource analysis (RaML / Hoffmann) — FLAGSHIP CANDIDATE

Type-system-driven inference of polynomial cost bounds, solved by LP. Pure +
structural recursion is exactly the tractable fragment. Product shape:
`kanso check` reports asymptotic bounds and flags regressions ("decode was
O(n), your change made it O(n²)"); the same bounds feed the parallelism cost
model, turning the work-stealing gate from heuristic to inferred fact. No
mainstream-feeling language ships this. Research-grade tooling exists; we'd
be productizing.

## 4. Equality saturation / e-graphs (egg, egglog) — WHEN THE IR SETTLES

Rewrite via saturation instead of ordered passes; phase-ordering dissolves.
Apply to kanso's own IR before LLVM: pipeline fusion (`map f . map g` → one
pass) is unconditionally sound under purity — deforestation without
Haskell's fragility. The `egg` crate makes this adoptable rather than
research.

## 5. Interaction nets / HVM2 / Bend — WATCH LIST (revisit ~2027)

Optimal-reduction runtimes with automatic parallelism including GPU. The most
"new math" thing in compilers now. Honest read: spectacular on some
workloads, unproven for strict everyday code; adopting it is a runtime
replacement, not a pass. Keep the Cilk-style tier-1 parallelism plan;
re-evaluate when their strict-mode story matures.

## 6. SIMD structural scanning (simdjson-class) — STDLIB-LEVEL, PROVEN

Vectorized byte classification feeding the jump tables the backend already
emits. Lives behind `bytes` primitives; no language surface. The remaining
order-of-magnitude on parsing workloads.

## The faster-than-Rust thesis, stated honestly

Not "beat Rust at microbenchmark loops" — LLVM emits the same instructions
for both. The winnable claim: beat *idiomatic* Rust on allocation-heavy real
workloads, because (1) Perceus reuse eliminates the defensive clones the
borrow checker pushes people into, (2) fusion is unconditional under purity,
(3) speculation specializes what Rust's monomorphization can't see, and
(4) the cost model schedules parallelism the user never wrote. Rust's
performance requires the user to be excellent; kanso's should require the
compiler to be.

## Ownership pipeline + positioning guardrails (Clay handoff, 2026-07-12)

Goal reframed: not "no borrow checker" but **"no annotations and no rejected
programs"** — ownership is whole-program compiler inference, never
user-supplied proof.

1. Owned/borrowed parameter-mode inference per function (kills most RC traffic).
2. Perceus reuse + FIP discipline: hot paths = zero-allocation, reusing freed cells.
3. Static uniqueness inference: delete the refcount==1 branch where provable;
   recover `noalias`-grade aliasing facts.
4. Non-atomic RC by default; escape-based promotion to atomic only where a
   value provably crosses threads.
5. Tail-recursion-modulo-cons for builder loops.
6. Structured scoped parallelism: tasks borrow shared data, so cross-thread
   ownership (and atomics) mostly never arises.
7. Residual gap stays: unprovable-uniqueness reuse sites pay one predictable
   branch. Closeable only as opt-in per-function FIP guarantee (annotated fn
   fails to compile if unmet). Never close globally — that rebuilds the
   borrow checker.

Items 1–6 are kanso-side analyses LLVM cannot do — backend choice is
orthogonal; compile-to-C/cranelift-before-LLVM for phase 3 remains sound.

### Positioning guardrails (in force for ALL public docs)
- No unfalsifiable superlatives ("absolute obsolescence of go and rust" — rejected).
- No primacy claims: Koka, Lean 4, Roc have prior art on Perceus+reuse — credit
  explicitly; kanso's edge is its extra static rules (no-shadowing,
  nothing-wasted) making the analyses more complete, not being first.
- "Renders the borrow checker obsolete" — rejected; say "no annotations and no
  rejected programs" + state both residual gaps honestly.
- "Byte-identical execution" requires floating-point caveats.
- House style: aggressive claims grounded in falsifiable specifics +
  preemptive transparency about open problems.

AUDIT QUEUED: sweep index.html doctrine #6, compiler.html, about.html,
book ch07, bench/RESULTS.md for guardrail compliance (esp. byte-identical
phrasing + FP caveat, borrow-checker phrasing, prior-art credits).
