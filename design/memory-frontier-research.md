# The memory-model frontier — is beats as far as we go?

A research memo (2026-07-18) from a 27-agent adversarial workflow: survey the
literature → dream grounded-but-wild ideas → refute each with a skeptic →
synthesize. 16 ideas generated; 0 survived unqualified, 9 partial, 7 refuted.
Every claim is falsifiable against the tree or a named paper.

## 0. The receipt that reframes everything (VERIFIED against the tree)

Our own honesty tiers are optimistic by one notch. In-place reuse for
uniquely-owned list builders is treated as **built**. It is not wired:

- `src/linear.rs` computes the linearity fixpoint correctly, but **nothing
  consumes its output** — the only external reference is `pub mod linear;` in
  lib.rs. Dead analysis.
- `k_b_push_mut` (runtime.c:1319, the in-place push) is **called by nothing**;
  it falls back to the copying `k_b_push`. codegen emits neither.
- The 16-byte `KHeader{rc,pad}` is allocated on **every** heap object; `k_dup`/
  `k_drop` "count only, no freeing yet"; codegen emits no calls.

So today we pay Perceus's per-object header tax with **none** of the reuse
benefit, and the marquee "functional-but-in-place" win doesn't fire. The honest
baseline is **beats + copying construction**, not beats + reuse. (What IS real:
the beat arena + rewind; JSON decoder flat at ~7.1MB, serde parity.)

## 1. Is beats the frontier?

**For what beats answers — "who frees this, and when" — yes, at or near optimal.
The remaining performance is not in the free schedule; it's in representation
and construction. Beats is not the bottleneck; the tag is.**

- Near-optimal: an O(1) pointer reset reclaims the whole dead set; vs
  generational GC's O(live-set) *trace* to discover survivors, beats pays O(1) +
  O(static-survivors), no trace, no write barrier (purity forbids the old→young
  edge). Sound because closed-world kills unknown-caller conservatism and
  no-aliasing kills control-flow-dependent survivor identity. Existence proof:
  MLKit (Hallenberg/Elsman/Tofte, PLDI'02) — "regions alone, without GC" is
  fastest where the discipline fits.
- Where it leaves performance on the table: (1) **representation** — the ~13%
  gap to serde is tagged values + per-call dispatch, not allocation traffic; no
  memory technique touches it; (2) peak sum-allocated vs max-live — real in
  theory, near-zero on our workloads (borrow-input + fixnum + deforestation +
  per-iter rewind already collapse it); (3) the unbuilt survivor double-write.

## 2. Perceus-on-beats?

**The runtime count: never — delete the header (settled doctrine). The static
reuse: a narrow measurable sliver whose right home is the build-block.**

- The count is strictly negative (per-object store traffic to reclaim into an
  allocator that bulk-reclaims for free). **Action: delete the 16-byte KHeader
  from arena objects.**
- Static reuse-in-place (FIP/ICFP'23 specialized to a bump arena) is sound and
  degrades gracefully (a missed pairing is a slower line, never a miscompile —
  so the Wansbrough–SPJ inferred-usage fragility does NOT apply; that was
  *dynamic* usage, this is *structural* last-use). But it only helps peak RSS,
  only for non-LIFO interior churn, only when the gap crosses a cache level — and
  the LIFO case it was pitched at is dominated by **free-the-top** (decrement the
  bump pointer at a static last-use). Persistent RB/HAMT rebuilds must be
  *excluded* (sharing-defined → not uniquely owned → degrade to copy). So its
  right home is the build-block, where uniqueness is *syntactic*.

## 3. Making beats more optimal — survivors ranked (payoff × feasibility)

1. **Wire `linear.rs` into codegen** — the reuse the docs already claim starts
   firing. Analysis + runtime fn both exist and are tested; only the codegen
   selection between `k_b_push`/`k_b_push_mut` is missing. Add an
   observable-allocation-count test, measure the spine case first. *(Memory-
   behavior-sensitive — mutation in place; supervise the x86 gate.)*
2. **Free-the-top mini-rewind** — captures the whole LIFO scratch win cheaply
   (any-size subsequent reuse, no pairing, no fragmentation); a finer-grained
   rewind that composes with beats.
3. **Generalize the non-heap-scalar rewind rule** (runtime.c:134) to every
   scalar-returning call site / arm — the one legitimate sub-beat case, using
   the SCALAR set `beat.rs` already computes. Abandon any placement that rewinds
   a frame producing surviving heap (use-after-free).
4. **Three-way escape split** — dies-this-beat → scratch; MUST-survive (proven on
   every path) → born-in-shelf, no copy; MAY-survive → build-on-bench + copy.
   Gating on *may* instead of *must* is a cross-beat leak (the MLKit
   region-pinning failure). Measure survivor volume on VSE first.
5. **`--explain-copies` diagnostic + AARA CI footprint ratchet** — make the
   design's one fragility (a distant edit silently flips reuse→copy) observable;
   symbolic peak-RSS bound as a CI contract. Observability, not speedup.
6. **TRMC (stack-safety + ergonomics, not a memory win) + single-consumer bit +
   surgical DPS** where a copy genuinely occurs.
7. **Cohort-counting soundness ratchet TEST — before regions ship.** "Cycles
   cannot cross birthdays" is the most novel + least-tested piece; its soundness
   rests on one "X can never happen" claim of exactly the shape that's burned us
   before. Write an adversarial property test (a build-block capturing + mutating
   an already-frozen outer value) in the `ratchet_*` style. Get an adversarial
   proof, not a confident assertion. Also nail the birthday granularity for two
   build-blocks sharing a beat.
8. Two-level scratch arena — lowest priority, gated on measured scrap volume.

## 4. Breaking new ground (survived attack, SPECULATIVE — each gets an experiment)

- **Static reuse-in-place inside the build-block** — shape-preserving rebuild
  brings a tree's 2x-until-beat-boundary footprint to 1x, scoped to where
  uniqueness is syntactic. Gated on item 3.1. (Halves the peak that bounds the
  per-cycle resident set for control loops — beats' own headline metric.)
- **Tag-hoist under monomorphism speculation** — the highest-value new ground
  because it hits the *actual* 13% (representation). Speculate a collection is
  monomorphic in element tag, hoist the tag out of the hot loop, bail to the
  scalar path on failure. The transpose (full SoA) fights deforestation; tag-
  hoist is the separable half.
- **Auto-SoA via whole-program field-touch** — "sell layout to the compiler"
  made literal (values have no address-identity contract; the compiler sees
  every access site — Rust can't, `&T` bakes in identity + separate compilation
  hides sites). Only profitable where deforestation is already defeated
  (multi-pass/random-access over a materialized collection). Gate as a cost-model
  term co-selected against fusion; validate on a *numeric* workload, not JSON.
- **Build-blocks as the sanctioned host for in-place graph algorithms** —
  union-find, a compile-time e-graph, NbE, unification — mutable aliased
  pointer-identity mutation with no lifetimes/rank-2, via the syntactic
  "nothing escapes" check. Construct 3 doing its advertised job on a new payload.
  (Interaction nets stay REFUTED: ~10x slower on numeric code — Asperti's own
  retrospective; no polynomial bookkeeping bound; flat-freeze kills the sharing.)
- **e-graph fusion over pure IR** — promote the already-planned fusion; purity
  makes every value-equality rewrite unconditionally valid, closed-world makes it
  program-wide (past GHC's function-boundary limit). Do NOT extend to co-schedule
  layout/lifetime/reuse (those memory edges are context-sensitive + mutually
  recursive = Wansbrough–SPJ smuggled back in; peak-RSS is non-additive).

## 5. The theoretical ceiling

"Zero runtime MM, statically-computed optimal schedule" is a **theorem only for
the constant-footprint fragment — where beats already wins more cheaply.** For
the ergonomic fragment (parsers, folds, control loops) the reachable ceiling
**is the four-construct model**: O(1) bulk rewind + a bounded static copy or one
cohort refcount per escape.

Undecidability does NOT bite on uniqueness (Rice is about Turing-expressible
semantic properties; a language that can't express aliasing reads uniqueness off
the grammar, like totality in Agda). It bites in exactly three named places:
1. **value-determined sizes** (AST, runtime arrays) → schedule parameterized by
   runtime data, not a static instruction sequence;
2. **the lazy `if`** → the one data-dependent last-use (`beat.rs` special-cases
   it; honest fallback is a conservative copy);
3. **the unbounded fold-state fixpoint** → AARA ⊤.

Three ceiling attempts, all refuted as *performance* wins, each with a salvage:
interval-graph optimal coloring (→ codegen peephole for disjoint-lifetime
fixed-size arm temporaries); totality-exactness certificate (false: termination
≠ bounded peak liveness; → a per-function footprint-class diagnostic); AARA
symbolic bound (→ the CI ratchet).

## 6. Refuted — dead, do not resurrect without new evidence

Runtime RC on the arena; sub-beat mini-rewinds keyed to arm last-use (can't fire
on recursive descent); heap register-allocation as a memory model (empty domain);
deep per-frame nested regions (fragments); one-e-graph co-scheduling of
layout+lifetime+reuse; cohort-freeze interaction net; beat-offset TRMC as a
memory win; optimal beat-boundary min-cost-cut with survivors (the survivor term
IS a copying-GC minor collection — regresses the defining property).

## The one-paragraph answer

**Is beats the frontier?** Yes for management (O(1)-dead + O(static-survivors)
beats generational O(live) trace; MLKit confirms). Nothing meaningful left in the
free schedule; the real gap is the tag. **Perceus-on-beats?** The count never
(delete the header); static reuse only in a sliver, home is the build-block; the
LIFO win is cheaper via free-the-top. **Grindable new ground?** Wire the dead
reuse (3.1, first); free-the-top + generalized scalar rewind; the born-in-shelf
survivor split; tag-hoist (the real 13%); auto-SoA (gated on a numeric workload);
build-blocks for in-place graph algorithms. And before regions ship, write the
cohort-birthday ratchet test — the highest-value verification target because it's
exactly the "X can never happen" shape that's burned us before.
