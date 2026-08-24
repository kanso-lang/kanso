# The memory-model frontier — is beats as far as we go?

A research memo (2026-07-18) from a 27-agent adversarial workflow: survey the
literature → dream grounded-but-wild ideas → refute each with a skeptic →
synthesize. 16 ideas generated; 0 survived unqualified, 9 partial, 7 refuted.
Every claim is falsifiable against the tree or a named paper.

## Where every idea stands

Fifteen ideas, and the state of each. The prose below carries the reasoning;
this says only which of them are in the tree.

| # | idea | state |
|---|---|---|
| 3.1 | wire `linear.rs` into codegen | **shipped** — consumed at four sites in `beat.rs`; codegen selects `k_b_push_mut`; 334,950 buffer reuses per gauntlet run |
| 3.2 | free-the-top mini-rewind | **declined** 2026-07-27, with the reopening condition named |
| 3.3 | generalize the non-heap-scalar rewind | **not landed**, rechecked 2026-08-24 — the runtime rule is still `k_beat_pop` alone (`src/runtime.c`, "a non-heap result rewinds as always"), firing only when a beat closes. `SCALAR` in `beat.rs` is used for slot and threading decisions, never to rewind at a call boundary. The open idea is intact and so is its warning: a callee that returns a scalar may still have written heap into something the caller holds, and rewinding there frees it |
| 3.4 | three-way escape split | **declined** 2026-07-27, measured |
| 3.5 | `--explain-copies` + AARA footprint ratchet | **half shipped** — the counter stack exists and is CI-gated; the diagnostic naming each copy's source site does not |
| 3.6 | TRMC | **shipped** (#394, #395), and **widened** 2026-08-23 — the operand proof comes off the shape, so `n * fact (n - 1)` qualifies without Inference threaded from check |
| 3.7 | cohort-counting soundness ratchet test | **shipped** |
| 4.1 | static reuse-in-place inside the build-block | **declined**, measured |
| 4.2 | tag-hoist under monomorphism speculation | **already harvested** |
| 4.3 | auto-SoA via whole-program field-touch | **declined** for want of a numeric workload |
| 4.4 | build-blocks hosting in-place graph algorithms | **not expressible today** — the blocker is the block-born rule, not the theorem |
| 4.5 | e-graph fusion over pure IR | **declined** for want of a customer |
| 5.1 | copy-or-pin for survivors | **its premise is gone** (rechecked 2026-08-24) — one-shot's evacuation is 3 allocations, not 63,967; #868 deleted the copy-out this was going to delete. Reposed below against where evacuation actually lives now |
| 5.2 | per-beat policy selection by survivor ratio | **new 2026-08-07**, not among the original sixteen |
| 5.3 | the reuse delta — does dynamic reuse beat static? | **the well-posed form of "Perceus vs beats"** |

## What the evacuation counter changed (2026-08-07)

This memo used to end its table by naming two gaps, both dated 2026-08-06:
evacuation copies had no counter, and nothing had been compared against a
Perceus runtime. **The first is closed.** `evac_allocs` and `evac_bytes` count
at `k_copy_alloc`, the one point every evacuated byte passes through, and they
are CI-gated across four cost goldens and 41 `.mem` fixtures.

The cost was concentrated far more sharply than anyone expected:

| shelf | evacuation allocations | of total | evacuated bytes |
|---|---|---|---|
| decode | 11 | 7,577,414 | 464 |
| encode | 19 | — | 624 |
| basket | 0 | — | 0 |
| **one-shot** | **63,967** | **128,528** | **1,991,456** |

Half of every allocation the one-shot program made was the copy-out, and that
is what put copy-or-pin at the top of the board.

**Rechecked 2026-08-24, and the table above is history.** One-shot reads
`evac_allocs=3`, `evac_bytes=96` today. #868 — the keyword compare rewrite —
took it from 63,967 to 5, and #977 to 3. The measured half copy-or-pin was
going to delete had already been deleted by something else, which is exactly
the thing a top-ranked idea priced against a stale number cannot tell you.

Where evacuation actually lives now, across the eight shelves:

| shelf | evacuation allocations | evacuated bytes | bytes per survivor |
|---|---|---|---|
| **wide** | 264 | **1,032,336** | 3,910 |
| **pending** | 2,658 | 498,976 | 188 |
| scan | 36 | 8,800 | 244 |
| encode | 17 | 576 | 34 |
| decode | 3 | 112 | 37 |
| one-shot | 3 | 96 | 32 |
| basket, escape | 0 | 0 | — |

That reposes 5.1 rather than retiring it, and it reposes it better. The shelf
with teeth is wide, and its survivors are large — four kilobytes each, against
the thirty-odd bytes the one-shot survivors averaged. Page pinning wins where
survivors are page-localized and loses where they are scattered through the
garbage, so the old shelf was close to the worst case for the idea and the new
one is close to the best. The measurement that decides it is where the bytes
sit, and it has now been taken — the evacuation path instrumented to record
each survivor's source address and copied size, on both shelves.

**Wide is four copies.** Four nodes of 256,016 bytes each — a 16,000-element
list buffer, `16 + 16 x 16000` — carry 99.2% of the megabyte, and
`bench/wide.json` is a 16,000-element list: that is its top-level buffer
evacuated as the streaming loop's carried accumulator, once per rewind. The
other 260 survivors total 8,272 bytes between them, median 32. This is the best
case the
idea could ask for: a quarter-megabyte survivor occupies whole pages by itself,
so retiring its storage instead of copying it retains almost no garbage, and it
does not need general page pinning — a size threshold and a block that does not
rewind would take the whole million bytes.

**Pending is the opposite, and the same instrument says so.** 666 of its 2,658
survivors are needed to reach 90% of half a megabyte, nothing is above four
kilobytes, and the largest is 3,216 bytes. Survivors that size are threaded
through the garbage; pinning their pages keeps the garbage with them.

So copy-or-pin is not one idea with one answer. On a large-survivor shelf it is
nearly free and deletes nearly everything; on a diffuse one it trades a copy
for retention. The size distribution is the decision variable, it is available
statically for the wide case (the list's length is a loop bound), and 5.2's
survivor-ratio selection is the same question asked one level up.

**And the project has already built that selection once — on the other path.**
`k_cohort_pop` sizes its survivor before the dance and refuses twice: when the
copy exceeds half the reclaim, and when the survivor is larger than four times
the block threshold, because the dance transiently holds the copy twice on top
of the garbage it frees. That is 5.2, shipped, for cohorts. The beat carry has
no such guard: `k_beat_iter_carry` copies every unkept slot, every rewind, at
any size. And the beat carry is where the remaining evacuation lives — wide
reports `cohort_frees=0` and `cohort_kept=0` beside its 264 evacuations, so
none of its megabyte passes the guard that would have refused it.

There is a precedent for the pin, too, in the same file and at the same
granularity the wide case wants. `k_carry_stage_kept` moves a builder's KStr
header off the arena — malloc'd once, "a promoted header survives the mark
from then on" — precisely so the rewind cannot reclaim it. It promotes a
header and shares the data. What wide needs is the same move made for the
storage rather than the header, on the one slot big enough to be worth it.

**5.1 Copy-or-pin.** The one-shot cost is the evacuation rather than the free
schedule. Instead of copying a survivor out before the rewind, pin its page and
rewind around it. This deletes the measured half without importing a single
refcount operation, which is what puts it ahead of reaching for Perceus. It was
named as planned-not-built on `compiler.html` §03 long before there was a
number to justify it; the counter is that number.

**5.2 Per-beat policy selection, survivor ratio as the decision variable.**
Beats and refcounting fail in complementary ways — beats lose when survivors
are large and frequent, refcounting loses on short-lived garbage — and those
are statically distinguishable by machinery already in the tree (`src/linear.rs`
plus whole-program inference). A beat with a provably small survivor set stays
an O(1) rewind; one with a large survivor set becomes a refcounted region. This
is absent from the original sixteen because the ratio it selects on could not be
measured until the counter existed.

**5.3 The reuse delta.** Perceus's real contribution over what kanso already
ships is *dynamic* reuse: catching last-reference cases static analysis cannot
prove. Kanso ships the static half (3.1, 334,950 buffer reuses per gauntlet
run) and declined the build-block extension on measurement (4.1). So the
well-posed experiment is not "Perceus vs beats" but: does dynamic reuse catch
enough that static misses to pay for a count on every object?

**The structural point to defend, and not to trade away.** Perceus requires a
count field on every heap object. Kanso has no per-object header at all —
`KHeader` does not exist. That is a representational advantage over Koka rather
than a tuning one, and any hybrid that reintroduces a count everywhere has
given it back.

## 0. Where the tree actually stands

Beat arenas plus rewind are the mechanism, and the reuse path is wired: the
linearity fixpoint in `src/linear.rs` is consumed by `beat.rs` through
`fold_spellings` and `in_place_pushes`, codegen selects `k_b_push_mut` at
proven sites, and the gauntlet reports 334,950 buffer reuses per run. No
per-object header is allocated — `KHeader` does not exist. The only reference
count left is on `KThunk`, which is malloc-backed on purpose so a pending thunk
cannot pin a rewindable region.

So the baseline is **beats + static reuse**. Both sides of the ledger are
weighed now: `evac_allocs` and `evac_bytes` count what a value outliving its
beat costs to move, and the answer is that decode pays 11 allocations out of
7.6 million while the one-shot shelf pays half of everything it allocates. See
"What the evacuation counter changed" above.

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

1. **Wire `linear.rs` into codegen — SHIPPED.** The reuse the docs claimed
   now fires. Analysis + runtime fn both exist and are tested; only the codegen
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
6. **TRMC — SHIPPED, and WIDENED (2026-08-23).** Accumulating integer
   recursion runs as a loop on all three engines. The widening landed
   without threading Inference from check, which is what the earlier
   scope assumed it needed: the proof comes off the shape instead. The
   wrapper the rewrite already generates ascribes every counter position
   `int`, so requiring each recursive call to hand those positions
   arithmetic over counters carries integer-ness down every level, and
   an operand built from counters and literals is an integer at every
   depth. `n * fact (n - 1)` and `n * n + r (n - 1)` now qualify where
   only `1 + count (n - 1)` did. Floats stay refused — reassociating
   their addition changes the answer, and three fixtures are there to be
   refused for exactly that reason. Single-consumer bit + surgical DPS
   remain unexplored.
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
- **Build-blocks as the sanctioned host for in-place graph algorithms —
  NOT EXPRESSIBLE TODAY (measured 2026-07-28); the blocker is the
  block-born rule, not the theorem.** Every algorithm named here reaches
  its nodes by traversal or by index, and block-born-ness survives
  neither. Measured, each rejected with `error[build]`: a node taken from
  a list built in the same block (`cells[1]!`), a node reached by
  following a field (`a.up`), a plain alias (`c = a`), and a node chosen
  by a conditional (`c = if … a b`). Only a name bound **directly** to a
  construction expression may be written.
  So union-find cannot compress a path, an e-graph cannot rewire a class,
  and unification cannot bind a variable it found — the mutation each one
  needs is on a node it reached, and reaching loses the licence. What
  build blocks support today is the case the book teaches: wiring a cycle
  among a fixed set of directly-named constructions.
  The rule is conservative rather than necessary. The theorem needs the
  cohort to be closed — everything born in the block, nothing escaping —
  and a node reached by indexing a block-born list *is* in the cohort. So
  the enabling work is to make block-born a dataflow property (flowing
  through aliases, conditionals, indexes of block-born collections and
  fields of block-born nodes) instead of a syntactic one on the binding.
  That is scoped compiler work, not a research question, and it is what
  4.4 is waiting on. Clay's call, since it widens what the checker
  admits.
  (Interaction nets stay REFUTED: ~10x slower on numeric code — Asperti's own
  retrospective; no polynomial bookkeeping bound; flat-freeze kills the sharing.)
- **e-graph fusion over pure IR — DECLINED for want of a customer
  (measured 2026-07-28); if the shape ever appears, inlining is the
  cheaper answer.** The cliff is real and steep. The same chain —
  select, then map, then sum, over 300,000 elements — costs **16
  allocations** written inside one function and **1,200,022** with a
  single link moved behind a function boundary, 11.2 MB against 92.8 MB
  of allocation. Fusion stops dead at the call, exactly the
  function-boundary limit the entry names.
  No workload crosses it. Thirty-seven functions across vse, kq, std and
  the benches have a single-expression body using an adapter, and every
  one is a self-contained chain: `sum (list/map voters f)`,
  `argmax (to_list (list/map (range ncand) f))`. The idiom the book
  teaches — write the chain where it is consumed, or pipe it — is
  exactly the idiom fusion already handles.
  And if a program does grow the shape, an e-graph is the expensive way
  to answer it. Inlining a small single-use function before fusion
  reaches the same result, has two precedents in the tree already
  (`inline_builtin_wrappers` undoes a rename; `inline_single_use_chains`
  folds a single-use adapter binding), and needs no equality saturation.
  Reopen with the cheap intervention first. The entry's warning stands
  either way: do NOT extend it to co-schedule layout/lifetime/reuse —
  those memory edges are context-sensitive and mutually recursive
  (Wansbrough-SPJ smuggled back in), and peak-RSS is non-additive.

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
LIFO win is cheaper via free-the-top. **Grindable new ground?** The reuse is wired
(3.1); free-the-top + generalized scalar rewind; the born-in-shelf
survivor split; tag-hoist (the real 13%); auto-SoA (gated on a numeric workload);
build-blocks for in-place graph algorithms. And before regions ship, write the
cohort-birthday ratchet test — the highest-value verification target because it's
exactly the "X can never happen" shape that's burned us before.
