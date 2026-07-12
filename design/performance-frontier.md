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
