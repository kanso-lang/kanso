# The memory-model frontier — is beats as far as we go?

A research memo (2026-07-18) from a 27-agent adversarial workflow: survey the
literature → dream grounded-but-wild ideas → refute each with a skeptic →
synthesize. 16 ideas generated; 0 survived unqualified, 9 partial, 7 refuted.
Every claim is falsifiable against the tree or a named paper.

## Where every idea stands

Sixteen ideas, and the state of each. The prose below holds the rows still
open (3.3, 3.5 and 5.x) and the analysis behind them. What shipped is in the log, and what was
declined is on the compiler page with its measurement.

| # | idea | state |
|---|---|---|
| 3.1 | wire `linear.rs` into codegen | **shipped** — consumed at four sites in `beat.rs`; codegen selects `k_b_push_mut`; 334,950 buffer reuses per gauntlet run |
| 3.2 | free-the-top mini-rewind | **declined** 2026-07-27; the measurement and the reopening condition are on the compiler page's ledger |
| 3.3 | generalize the non-heap-scalar rewind | **not landed**, rechecked 2026-08-24 — the runtime rule is still `k_beat_pop` alone (`src/runtime/evacuate.c`, the comment above `k_memo_outlives`: "a non-heap result rewinds as always"), firing only when a beat closes. `SCALAR` in `beat.rs` is used for slot and threading decisions, never to rewind at a call boundary. The open idea is intact and so is its warning: a callee that returns a scalar may still have written heap into something the caller holds, and rewinding there frees it |
| 3.4 | three-way escape split | **declined** 2026-07-27, measured; on the compiler page's ledger |
| 3.5 | `--explain-copies` + AARA footprint ratchet | **half shipped** — the counter stack exists and is CI-gated; the diagnostic naming each copy's source site does not |
| 3.6 | TRMC | **shipped** (#394, #395), and **widened** 2026-08-23 — the operand proof comes off the shape, so `n * fact (n - 1)` qualifies without Inference threaded from check |
| 3.7 | cohort-counting soundness ratchet test | **shipped** |
| 4.1 | static reuse-in-place inside the build-block | **declined**, measured; **moot** since `build` retired on 2026-10-10 (kanso#1837) |
| 4.2 | tag-hoist under monomorphism speculation | **half harvested**, corrected 2026-09-02 — harvested for a value's tag in a numeric loop, and NOT for a callable's tag at a dispatch, which was worth 1.096% of oneshot and shipped in #1210 |
| 4.3 | auto-SoA via whole-program field-touch | **declined** for want of a workload; on the compiler page's ledger |
| 4.4 | build-blocks hosting in-place graph algorithms | **moot** since `build` retired on 2026-10-10 (kanso#1837), and `list/tie` is the one way to write a cycle. Before that: **the rule is widened** (2026-09-09): born flows through aliases, `if`, indexes and fields; a node reached through a call is still not proved, so the traversal algorithms wait on that flow |
| 4.5 | e-graph fusion over pure IR | **declined** for want of a customer; on the compiler page's ledger |
| 5.1 | copy-or-pin for survivors | **its premise is gone** (rechecked 2026-08-24) — one-shot's evacuation is 3 allocations, not 63,967; #868 deleted the copy-out this was going to delete. Reposed below against where evacuation actually lives now |
| 5.2 | per-beat policy selection by survivor ratio | **new 2026-08-07**, not among the original sixteen |
| 5.3 | the reuse delta — does dynamic reuse beat static? | **the well-posed form of "Perceus vs beats"** |
| 5.4 | make the survivorship decision cheaper, not the copy | **new 2026-09-01, and it reframes 5.1 and 5.2** — measured below |

## 5.4 — the deciding, not the copying (2026-09-01)

Every 5.x idea in this memo is aimed at the same thing: evacuation COPIES too
much, so pin what you can, or choose a policy by survivor ratio. On the one
workload built to stress a streaming carry, the copies were never the cost.

An 8,192-byte sha256 digest with the carry tier admitted:

| what | count |
|---|---:|
| calls to the survivorship walk | 33,024 |
| slots examined per call | 8,256 |
| slot examinations, total | 272,646,144 |
| heap slots found in any of them | 0 |
| allocations that actually evacuated | 33,827 |

The evacuation's copying was linear in the message throughout. What was
quadratic was `k_slots_survive` asking, once per element, whether a list that
grows with the message holds anything the rewind could take — and answering no
every time, by walking the whole thing.

So the family splits in two. 5.1 and 5.2 are about what to copy, and on this
shape they would have saved nothing, because almost nothing was being copied.
5.4 is about what it costs to decide, and it is where the time was.

Two memos were built against it. `k_isv_list` caches whether a list deep-
survives the OUTERMOST mark and is invalidated when that mark moves; it misses
entirely on the streaming shape, because a list built inside the outer beat can
never be below its mark. `k_isv_flat` caches the mark-INDEPENDENT half — this
items array holds no heap slot and no thunk, so no rewind can take anything out
from under it — keyed on the items pointer, with an epoch that moves on a push
and a pop and not on a rewind. That is what made the carry tier buildable at
all.

It was still declined, and not for this: with the deciding memoized, the carry
tier costs 5.4x the digest's retired instructions because the evacuation
discards exactly the lazy memos the at-risk state was built to keep, and
recomputing them is the bill. Welfare reads 74.31 against 73.75. The
2026-09-01 log entries carry the sweep.

**What this changes for the memo.** A survivorship check is O(arity) at every
node it visits, and nothing in the tree bounds how often it is asked. The
counters that would show it — `evac_allocs` and `evac_bytes` — count the
copying, so a workload whose cost is entirely in the deciding reads as free in
every vein. That is the gap this row names, and it is unmeasured outside the
digest.

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

Two things a first attempt at a reduced fixture ruled out, both worth not
repeating. A cohort is licensed narrowly — codegen wraps one only around a
qualified call that crosses DOWN into a nested module, from a caller that is
not itself a rewinding loop, with every argument a non-heapish shape — so a
same-module call never produces one however much garbage it makes, and the
`.mem` corpus has no cohort fixture for that reason rather than by oversight.
And a large list merely THREADED through a beat loop is not copied: a
16,000-element list built before the loop and passed through 8,010 iterations
evacuates 54 nodes of 32 bytes, because the list is below the mark and shared.
So wide's four copies are not "a big carried list" and the postcard is not
written yet. The next probe was which carry site produces them, and it
has been taken: `k_deep_copy` has exactly three entry points — `k_beat_pop`'s
result copy, `k_caf_freeze`, and `k_beat_iter_carry` — and with all three
tagged, **all 264 of wide's evacuations and all 1,032,336 bytes come from
`k_beat_iter_carry`**, the four large copies included. Nothing comes from the
other two.

A third probe says what the copied slot is, and it is not what the shape
suggested. There is exactly ONE carry slot (`n=1`), it is not marked kept, and
its tag is 8 — `K_DESC`. The carried value is the description
`io/write (...) . (_ -> stream_elems xs (i + 1))`, and what costs 256 KB is
the `xs` it captures: a 16,000-element list built by `decode` after the outer
beat's mark was taken, so it sits above the mark and the carry copies it
forward on every rewind. The list is loop-INVARIANT — the same value every
iteration, never rebuilt — and it is deep-copied whole each time anyway.

That is the finding, and it points somewhere the page-pinning framing did not.
`k_carry_kept` already exists for exactly this purpose, and its meaning today
is "this cycle's own builder". A loop-invariant capture is the other thing a
carried slot can be, it is statically recognisable, and promoting it once
would delete the whole megabyte without any change to how the arena rewinds.

**The postcard exists now**, as a pair in `tests/golden/mem`. Two programs
identical but for one line — a 500-element list built inside the chain, and the
same list built as an argument — over the same loop and the same number of
rewinds:

    a_loop_invariant_capture_is_copied_every_rewind   24 allocs   32,672 bytes
    the_same_capture_built_below_the_mark_is_shared    6 allocs      192 bytes

The first is wide's shape reduced to twenty lines. Nothing else in the corpus
watched the beat carry, which is where every remaining evacuation on the
benchmark shelves is spent, and a change that makes those two converge is the
one worth having.

One more fact about the pair, which narrows the next probe: **the two programs
have identical beat analysis.** `KANSO_BEAT_REPORT=1` prints the same two lines
for each — `walk/2` and `step/3` bracketed with their cluster, also unbracketed
entries — so the compiler sees no difference at all. The difference is where the
list's storage lands at run time relative to a mark the analysis places
identically. Whatever explains it is not in `beat.rs`'s verdicts; it is in when
the bracket is pushed against when the argument is evaluated, and the next probe
is `k_beat_depth` and the current mark at the moment the 500-element buffer is
allocated, in both programs.

**CORRECTED 2026-08-24, and the correction is the finding.** The claim above
that the beat carry has no guard is wrong. The guard is in the bind-chain
driver that pushes the beat those copies happen under: `k_exec` sizes the
staged continuation and takes one of three paths — under 4 KB in an undrifted
region, leave it; over 256 KB, floor the region instead of evacuating; between,
stage and copy. It landed with streaming stdout on 2026-07-27 and took kq's
full print from 47.5 MB to 30.0.

widebench's continuation captures 256,016 bytes, 2,128 short of that line, and
the cliff is sharp: 16,000 elements evacuate 1,028,512 bytes, 16,380 evacuate
3,648. Lowering the threshold to 64 KB measures beautifully — widebench's
`evac_bytes` 1,032,336 to 7,488, seven shelves byte-identical, `arena_peak_bytes`
unmoved anywhere. It should still not land: `bench/cost_golden_wide.txt` says in
its own header that sixteen thousand was chosen so the shelf sits BELOW the
threshold and exercises the staging path, and moving the line would hide the
cost this shelf exists to show.

The question that survives is whether the 4 KB-to-256 KB band is right for
programs rather than for a shelf placed under it. On the reduced fixture,
flooring is free at 128 KB and 192 KB survivors. That argument has to be made
on real programs, and it has to move widebench's size in the same change or the
shelf loses its point either way.

**And the pair said where to look, which was not the carry — but not here
either.** The two programs
differ only in whether the list is built before the mark or after it, and that
is a question about where the beat's mark is placed rather than about what the
carry does with what it finds. The prologue work here — decode once, then
stream — is loop-invariant, and the loop's bracket encloses it. Marking the
slot kept without moving its storage would be a use-after-free, and promoting
the storage is the copy again; hoisting the mark past invariant prologue work
is neither. That is a `beat.rs` question, and it is the one to answer before
any runtime change is written.

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
7.6 million and one-shot pays 3, as the 2026-08-24 recheck above found. See
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

3. **Generalize the non-heap-scalar rewind rule** (`k_memo_outlives` in
   `src/runtime/evacuate.c`) to every
   scalar-returning call site / arm — the one legitimate sub-beat case, using
   the SCALAR set `beat.rs` already computes. Abandon any placement that rewinds
   a frame producing surviving heap (use-after-free).
5. **`--explain-copies` diagnostic + AARA CI footprint ratchet — PARTLY
   SUPERSEDED (2026-07-27).** The reuse→copy flip is now observable and
   CI-gated by the counter stack built this stretch: bytes_peak,
   cohort_kept, carry_dedup, buf_reuse pinned per vein, the trend gate's
   direction table, and the welfare peak terms. What remains unserved is
   the *where* — a diagnostic naming the source site of each evacuation
   copy — which needs span plumbing through the carry machinery and a
   CLI surface worth a ruling before building.
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
