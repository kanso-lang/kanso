# The memory-model frontier — is beats as far as we go?

A research memo (2026-07-18) from a 27-agent adversarial workflow: survey the
literature → dream grounded-but-wild ideas → refute each with a skeptic →
synthesize. 16 ideas generated; 0 survived unqualified, 9 partial, 7 refuted.
Every claim is falsifiable against the tree or a named paper.

## 0. The receipt that reframes everything (STALE — see note)

**This section describes a tree from before the uniqueness campaign.**
`linear.rs` is consumed by codegen (which selects `push_mut` at proven
sites), `k_b_push_mut` is called, and the gauntlet reports 334,950 buffer
reuses per run. The honest baseline is no longer "beats + copying
construction". Left in place because the reasoning that follows it is
still worth reading; the premise is not.

## 0. The receipt as originally written (VERIFIED against the tree, then)

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
2. **Free-the-top mini-rewind — DECLINED FOR NOW (2026-07-27, evidence).**
   No current workload exhibits unreclaimed LIFO scratch beyond what beats
   and the cohort already rewind: the decode gauntlet runs at 3 blocks,
   encode at 4, oneshot's boundary fires, vse at 2 blocks. Revisit when a
   workload shows arena growth with no beat or cohort fire — that signal,
   not the idea, is the reopening condition.
3. **Generalize the non-heap-scalar rewind rule** (runtime.c:134) to every
   scalar-returning call site / arm — the one legitimate sub-beat case, using
   the SCALAR set `beat.rs` already computes. Abandon any placement that rewinds
   a frame producing surviving heap (use-after-free).
4. **Three-way escape split — DECLINED FOR NOW (2026-07-27, measured).**
   The gating measurement ran: vse holds arena_blocks=2 with its loops
   beating — constant space already, so the split has nothing to buy on
   the workload that was named as its test. Reopen if a beat-heavy
   workload shows significant carry-copy volume (carry_dedup and the
   copy-size pass now make that visible).
5. **`--explain-copies` diagnostic + AARA CI footprint ratchet — PARTLY
   SUPERSEDED (2026-07-27).** The reuse→copy flip is now observable and
   CI-gated by the counter stack built this stretch: bytes_peak,
   cohort_kept, carry_dedup, buf_reuse pinned per vein, the trend gate's
   direction table, and the welfare peak terms. What remains unserved is
   the *where* — a diagnostic naming the source site of each evacuation
   copy — which needs span plumbing through the carry machinery and a
   CLI surface worth a ruling before building.
6. **TRMC — SHIPPED at the exact slice (#394, #395).** Accumulating
   integer recursion with literal operands runs as a loop on all three
   engines, through additive int-ascribed wrappers so every non-integer
   argument keeps its original behavior. Widening to inferred-int
   operands is scoped (needs Inference threaded from check, not
   recomputed). Single-consumer bit + surgical DPS remain unexplored.
7. **Cohort-counting soundness ratchet TEST — DONE (adversarial corpus,
   2026-07-27).** Five attacks live in tests/golden/errors, all rejected
   with pinned diagnostics: writing an argument's older value
   (build_write_older_cohort), the same value laundered through a local
   alias and a call (build_write_alias), an enclosing block's
   construction from a nested block (build_write_enclosing_block), a
   prior loop iteration's frozen cohort (build_write_prior_iteration),
   and a deep-path write through a young wrapper into an old cell
   (build_write_deep_path — a parse error: single-hop writes make the
   channel unrepresentable, so a field write can only land on the named
   root, which must be a construction of the innermost block). The legal
   pastward case — outer block referencing an inner block's frozen
   cohort — is pinned three-engine byte-identical in
   tests/golden/micro/build_nested_cohort.kso. Loop granularity is
   settled by the prior-iteration rejection: each incarnation of a
   syntactic block is its own cohort.
8. Two-level scratch arena — DECLINED FOR NOW (2026-07-27): same evidence
   as item 2; the gating scrap-volume signal does not exist in any
   current workload.

## 4. Breaking new ground (survived attack, SPECULATIVE — each gets an experiment)

- **Static reuse-in-place inside the build-block — DECLINED (measured
  2026-07-28): the build block is not where the cost is.** Three fixtures,
  seven runs each, maximum resident: a 20,000-node tree built once costs
  4.4 MB; the same tree carried across a beat costs 17.6 MB; the same tree
  carried across a beat *with the build block removed* costs 17.6 MB. The
  block contributes nothing measurable, so a rebuild scoped to it has
  nothing to reclaim. The 4x belongs to the carry, which copies any large
  survivor whatever built it — already characterised, with its own floor:
  evacuation holds the survivor twice while the garbage is still live
  (see the 2026-07-28 entry on the evacuation law). In-place push was the
  other suspect and was cleared: identical counters inside and outside a
  block.
- **Tag-hoist under monomorphism speculation — ALREADY HARVESTED
  (measured 2026-07-28).** The unboxing proof got there first. A tight
  numeric loop compiles with its arguments unboxed (`i64`, not `%KValue`),
  every tag a compile-time constant, and the recursion a `musttail` self
  call: there is no tag left in the loop to hoist. What separates it from
  Rust is three `llvm.*.with.overflow` intrinsics per iteration, which is
  semantics rather than representation — and against Rust compiled with
  `-C overflow-checks=on` the same loop runs at **1.03x**. The 1.37x
  against wrapping Rust is the price of kanso's int, not a gap in its
  compilation. What remains, if the price is ever judged too high: a range
  analysis that discharges the check where bounds are provable, which
  needs specialization to see a caller's constant.
- **Auto-SoA via whole-program field-touch — DECLINED for want of a
  customer (measured 2026-07-28), with the prize recorded so it can be
  reopened.** The prize is real: 200,000 three-field records traversed
  twenty times touching one field run 50.3 ms and 23.8 MB as an
  array-of-records, against 27.7 ms and 19.9 MB as three parallel arrays
  holding the same data — **1.82x on time, 1.20x on peak**. (The first
  version of this measurement was unfair, storing one field in the SoA
  case against three in the AoS case; the numbers above are the corrected
  run.)
  What is missing is a workload with that shape. The gate the entry sets
  — multi-pass or random access over a materialized collection — is met
  only by vse, which traverses its electorate six times, and vse holds no
  records at all: a voter is a list of scores read as `v[c]`, so a
  field-touch analysis has nothing to see. Every workload that does hold
  records — the json decoder, the encoder, kq — is single-pass, and
  fusion already owns that case. Reopen when a record-shaped, multi-pass
  workload exists; the transform's value is not in doubt, only its
  customer.
  Adjacent and distinct: vse's electorate is a matrix stored row-wise and
  read column-wise, which is a transpose opportunity rather than a
  field-touch one, and the entry's own note that the transpose fights
  deforestation applies to it.
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
