# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

## 2026-07-18 — Seed: state of the memory/perf frontier

Full analysis: `design/memory-frontier-research.md` (27-agent adversarial memo).

### OPEN THREADS — do not drop

- **[OPEN — top priority] In-place reuse is DEAD CODE.** `src/linear.rs`
  (linearity fixpoint) is computed + tested but consumed by NOTHING (only
  `pub mod linear;` in lib.rs). `k_b_push_mut` (runtime.c:1319) is called by
  nothing; codegen emits no `push_mut`/`dup`/`drop`. Yet the 16-byte
  `KHeader{rc,pad}` sits on every heap object. → we pay the Perceus header tax
  with ZERO reuse benefit. FIX: thread the linear analysis into codegen to select
  `k_b_push_mut` on proven-unique lists; add an observable-allocation-count test
  (not a unit test on the analysis); measure the spine case first.
  **MEMORY-CORRUPTION-SENSITIVE — x86 gate, Clay watching.** (memo §0, §3.1)

- **[OPEN] Delete the 16-byte `KHeader` from arena objects.** Runtime RC on a bump
  arena is strictly negative (settled doctrine). We pay it for nothing today.
  MEMORY-SENSITIVE. (memo §2)

- **[OPEN] The TAG is the serde gap — not memory.** The ~13% to serde is
  tagged-value representation + per-call dispatch; allocation is already at serde
  parity. The lever is **tag-hoist under monomorphism speculation** (hoist element
  tag out of the hot loop, bail to the scalar path on failure). This — NOT the
  reuse-wiring — is what could close the serde SPEED gap. SPECULATIVE; measure
  before claiming. Do NOT conflate with the reuse fix. (memo §1, §4)

- **[OPEN — before regions ship] Cohort-birthday ratchet TEST.** "Cycles cannot
  cross birthdays" soundness rests on one "X can never happen" claim (the shape
  that shipped the kramdown crash). Write an adversarial property test — a
  build-block capturing + mutating an already-frozen outer value — BEFORE
  cohort/region codegen is load-bearing. Adversarial proof, not assertion.
  (memo §3.7)

- **[OPEN] Survivor double-write.** The unbuilt survivor path writes survivor
  bytes twice (bench + shelf). Three-way escape split (dies-this-beat / MUST-
  survive → born-in-shelf / MAY-survive → copy) fixes it; gating on MAY instead of
  MUST is a cross-beat leak. Measure survivor volume on VSE first. (memo §3.4)

### SPECULATIVE / NEW GROUND (survived adversarial attack; each needs an experiment)

- Static reuse-in-place inside the build-block (shape-preserving rebuild, 2x→1x
  peak); tag-hoist (above); auto-SoA via whole-program field-touch (gated on a
  NUMERIC workload, not JSON; co-selected against fusion); build-blocks as the
  host for in-place graph algorithms (union-find, compile-time e-graph, NbE).

### DECISIONS / HISTORY

- **[DONE] Beats / heartbeat arena** — bump-alloc + O(1) rewind; JSON decoder flat
  ~7.1MB, serde memory parity. Genuinely the frontier for MANAGEMENT (beats
  generational GC's O(live) trace; MLKit PLDI'02 confirms "regions alone, no GC"
  fastest where the discipline fits).
- **[REFUTED] do not resurrect without new evidence:** runtime RC on the arena;
  interaction nets (~10x slower on numeric — Asperti's own retrospective); heap
  register-allocation as a memory model; one-e-graph co-scheduling of
  layout+lifetime+reuse; sub-beat mini-rewinds keyed to arm last-use;
  beat-offset TRMC as a memory win; optimal beat-boundary min-cost-cut with
  survivors (its survivor term IS a copying-GC minor collection). (memo §6)
- **[REVERTED — measured neutral, don't re-try blind]** in-place put/maps;
  find_byte (2-memchr double scan); bytes-view fusion (~2% ceiling); inline record
  fields; utf8 ASCII fast path.
- **[DONE — compiler wins]** field-set inference; caught-failure propagation;
  unboxed-scalar ABI; register-return + escape analysis; unboxed dispatch;
  copy-elim bundle. Cumulative JSON decode ~1.93 → ~1.27ms.
- **[STANDING] hand-opts to back out as the compiler improves:** find2 +
  number-from-bytes (json-stdlib hand-compilation, ~10%). Back out once fusion +
  loop-generation-from-tail-recursion exist, and confirm the number holds.

---

## 2026-07-18 (later) — Pursuing tag-hoist; RECONCILE the gap first

Clay wants to chase tag-hoist (the speculative representation lever). Before
building it, **step 0: re-profile the current JSON decode** — two of our own
sources disagree on what the serde gap even is:

- **[CONFLICT]** memo / `compiler.html §11` says the ~13% gap is REPRESENTATION
  (tagged values + per-call dispatch). But the perf campaign's overnight
  diagnosis (above) measured the gap as ALLOCATION, not dispatch ("LLVM already
  folds the boxing across calls"), and unboxed-dispatch shipped for only ~3-4%.
  These can't both be current truth. **RE-PROFILE before committing** — point the
  work at the confirmed lever (tag-hoist if representation, reuse-wiring if
  allocation). Do not build tag-hoist on the §11 claim alone.

- **[OPEN, plan] tag-hoist mechanism** (if representation confirmed): speculate a
  collection is monomorphic in element tag, hoist the tag check out of the hot
  loop, run the body on raw payloads, bail-restart to the tagged path on
  violation (reuse the int-tiering AOT-restart mechanism). First cut = a CEILING
  EXPERIMENT: hand-hack the tag-hoisted fast path on the hottest loop (unsound,
  throwaway), measure the recovery vs the 13%. Real ceiling → build sound; noise →
  drop it (cf. the bytes-view fusion ceiling that measured ~2% and was parked).

---

## 2026-07-18 (step 0 RESULT) — Gap is REPRESENTATION; reuse correction

Re-profiled the current JSON decode (main, 3000×, macOS `sample`). VERIFIED:

- **The gap is DISPATCH/REPRESENTATION, not allocation.** Self-time split:
  dispatch/repr **53.5%** (d__value_for 469 = single hottest fn in the program;
  k_truthy 169 = guard/failure-bit overhead; the d__* dispatchers), alloc/
  construct 23.3%, copy 6.7%, str/num parse 16.5%. Confirms `compiler.html §11`.
  The earlier campaign "gap is allocation" note was STALE (pre-copy-elim; those
  wins moved the bottleneck to representation). → **tag/representation IS the
  lever; reuse-wiring is NOT the serde-speed lever.**

- **CORRECTION to the seed entry / memo §0:** the reuse is PARTIALLY wired, NOT
  fully dead. `k_b_push_mut` IS emitted (11× in jsonprof.ll, 202 samples) —
  in-place list-append fires via the runtime frontier-buffer trick, NOT via
  linear.rs. The memo's "push_mut called by nothing" was WRONG and I repeated it
  (macOS BSD-grep `\|`-is-literal bug hid it — use `-E` or single-term greps).
  STILL DEAD, confirmed vs the IR: `linear.rs` (general reuse analysis, 0
  `linear::` callers) and `k_dup`/`k_drop` (0 emissions) while the 16-byte KHeader
  is still on every object. So "delete the header" STANDS; "wire or delete the
  dead general reuse analysis" STANDS; "the in-place win doesn't fire at all" is
  FALSE (append fires).

- **Tag-hoist nuance (aim precisely):** d__value_for dispatches on the INPUT BYTE
  (data-dependent recursive descent), NOT a monomorphic collection element — so
  classic collection-tag-hoist doesn't map onto it. Real levers on d__value_for +
  k_truthy: (a) elide failure-bit/k_truthy plumbing where inference proves
  no-failure (169 samples pure guard overhead); (b) deeper KValue unboxing across
  the dispatch boundary. NEXT CEILING EXPERIMENT: strip the failure-bit/truthy
  checks in the hot dispatch path, measure recovery vs the ~13%.

---

## 2026-07-18 (experiment RESULT) — serde gap is SIMD, NOT representation; kanso beats naive Rust

Fresh baseline (this machine, best-of-10 ms/decode; kanso timed as a 3000× binary
so startup is negligible; naive/serde self-report decode-only mean):

| decoder | ms/decode |
|---|---|
| kanso | 0.932 |
| naive Rust (recursive descent, std String/Vec/HashMap) | 0.988 |
| serde_json | 0.846 |

- **kanso BEATS naive Rust by 5.6%.** The reframed campaign goal (beat reasonable
  native Rust, not serde) is ACHIEVED.
- **The ~10% serde gap is SIMD/zero-copy, NOT representation.** naive Rust — native
  types, zero tags, zero dispatch-boxing — is **16.7% behind serde**, MORE behind
  than kanso. A tag-free decoder does NOT close the serde gap ⇒ representation is
  not the serde gap.
- **This REFUTES the memo §11 premise AND the step-0 "tag is the serde gap"
  conclusion.** The profile's 53% dispatch/representation is kanso's INTERNAL
  self-time; cutting it widens kanso's lead over naive, but serde's SIMD lead is
  untouched. The two earlier claims were reasoning from an internal profile to an
  external gap — invalid; the cross-decoder comparison is the correct instrument.
- **CONSEQUENCE — tag-hoist DOWNGRADED:** do NOT build it expecting to crush serde;
  measured, the win isn't there. It would extend a naive-Rust lead we already hold.
  Beating serde specifically needs simdjson-class SIMD byte-classification — a
  separate, harder frontier — and per Clay's 2026-07-14 reframe serde was never the
  right north star. The tag-hoist OPEN thread above is superseded by this entry.

---

## 2026-07-18 (night, SIMD-frontier campaign) — KANSO BEATS SERDE, 25/25

Clay opened the SIMD frontier ("squeeze the lemon dry"). Ladder, each change
same-window A/B'd, lattice-gated (checksum 480000, goldens, json 16/16), merged
as PR #36 (x86 CI green on the final SHA):

1. **[DONE — the big one, −10.7%] IR-inlined predicates + constructors.**
   Discovery: `release_clang` passes `-flto` but LTO NEVER inlined the runtime's
   one-liner tag tests across the .ll/.o boundary — 27 `bl _k_truthy` calls
   survived in the release binary (169 profile samples), despite a runtime.c
   comment claiming LTO would inline it. Fix: `define internal ... alwaysinline`
   IR twins in the codegen DECLARES prelude for k_truthy / k_not_failure /
   k_check_tag / k_check_int / k_check_bool + constructors k_int / k_float /
   k_bool / k_none; cold path via newly-exported `k_truthy_bad`. Internal
   linkage avoids duplicate symbols vs runtime.c's own copies. Fully general —
   every program, every arch. **LESSON: never trust -flto to inline across the
   IR/C boundary; verify with `otool -tv | grep bl.*_fn`.**
2. **[DONE, −1.4%] SIMD find2** — NEON (shrn-by-4 mask, ctz>>2) on aarch64,
   SSE2 movemask on x86_64, scalar tail. serde's own memchr2 mechanism.
3. **[DONE, −2.2%] to_int integer fast path** — bare accumulate loop for strict
   [-]?digits{1,18} (can't overflow i64); everything else falls to strtoll
   unchanged. Floats NOT hand-rolled (shortest-roundtrip parity is sacred).
4. **[REVERTED, +3.0% regression] utf8 8-byte ASCII word-skip** — second time
   this exact idea failed on this fixture (strings < 8 bytes; the guard costs,
   the skip never fires). Do not try a third time without a long-string workload.

**RESULT (interleaved, 25 rounds, this M-series machine, 188KB gauntlet):**
kanso min 0.818 / median 0.846; serde min 0.853 / median 0.867 —
**kanso −4.2% min, −2.4% median, 25/25 pairwise wins. naive Rust −16%.**
Session start → now: 0.932 → 0.825 ms/decode.

**[OPEN] Scoreboard docs are stale the OTHER way now** — index.html §04 /
compiler.html §11 / book ch07 still say "~13% behind serde" / "~0.99ms". Update
with a careful fresh reproduced run (numbers above are same-window A/B deltas;
docs deserve a clean best-of-N pass + the reproduce recipe).

**[OPEN] Remaining profile after the ladder** (self-samples): d__value_for 393,
k_b_push_mut 161, k_utf8_bad 104, memmove 102 + memcpy 37, str_char 86,
obj_key_start 76, find2 76 (post-SIMD), slice 76 + bytes_view 53 (the parked
fusion, proportionally bigger now), mklist 66, utf8 61, put 57 + k_eq 46 (map-key
compares), strtoll residue 39→less. Next candidates: bytes-view/slice fusion
re-test (musttail-adjacent codegen — x86-risk zone, Clay watching), k_cmp int
fast path inline, dispatch-chain depth reduction (architectural).

---

## 2026-07-18 (later) — Clock-free performance ratchet (Clay's directive, PR #37)

Clay: make the perf wins a RATCHET via specs that read a representation, not
wall time. Built two halves, both gating in CI:

1. **Cost goldens** — runtime counters (allocs / alloc_bytes / arena_blocks /
   perm_allocs / beat_iters), dumped to stderr under `KANSO_COUNTERS=1` only
   (stdout goldens untouched), atexit-registered in main. Deterministic program
   ⇒ exact constants; CI diffs the gauntlet dump vs `bench/cost_golden.txt`.
   Baseline: **allocs=14799465, alloc_bytes=790444432, arena_blocks=6,
   perm_allocs=1, beat_iters=150** — arena_blocks=6 IS the flat-memory claim as
   a constant; beat_iters=150 is one heartbeat per decode. Updating the golden
   is a deliberate, diff-visible act.
2. **Structural IR specs** (`tests/perf_ratchet.rs`) — reads emit_ir output:
   (a) each alwaysinline twin exists AND carries the attribute on its define
   line; (b) recursion emits `musttail` (constant-stack as a testable fact);
   (c) a linear accumulator lowers to `k_b_push_mut` (in-place reuse wired).
   **Test-the-test done:** first version had a hole (prelude COMMENT contains
   the word "alwaysinline", inflating a count; per-name check didn't inspect
   the attribute) — a doctored prelude PASSED. Rewrote to per-define-line
   asserts; verified fail-for-the-right-reason, then green on restore.
   LESSON: always run the see-it-fail step; the first draft of a spec often
   specs nothing.

**[OPEN — answered by PR #37 CI] cross-arch counter determinism:** all sizes are
explicit i64 structs, so x86 counters should be bit-identical to arm64. If
ubuntu CI diffs, split per-arch goldens and investigate the divergence (that
would be a differential-lattice-class finding in its own right).

---

## 2026-07-19 — kq vs jq raced (gated); pretty-printer is the next target

`bench/kq_race.sh` (byte-identity gate per query, then interleaved timing):
path queries **kq 1.52x @188KB (25/25), 1.61x @1.9MB (15/15)** — the gap grows
with size (kq prints only the subtree). Full pretty-print: parity @188KB
(1.07x, 20/25), jq ahead 1.03x @1.9MB (0/15) — printer-bound, not
decoder-bound. **[OPEN] kq/_pretty is the target**: join-of-maps string
building; an encode-into-buffer printer should flip the identity rows.
**GATE STORY (log-worthy):** the harness's byte-identity gate caught that an
earlier ungated 1.9MB path number timed kq ERRORING (missing key — jq yields
null on missing paths, kanso errs; a real semantic difference now documented
in apps/kq/README.md). Never publish an ungated race.

---

## 2026-07-19 — kq broken out (kanso-lang/kq); fixtures caught a REAL bignum bug

kq now lives at github.com/kanso-lang/kq: fixture-gated specs (unicode/CJK/
emoji + escapes, precision numbers, deep nesting, the 188KB doc), each case
checked against a committed golden AND live `jq -S`; CI builds kanso from
source and gates. First run green.

**[OPEN — CORRECTNESS, HIGH] native bignum decode truncation.** The new
numbers fixture caught it: decoding `2^100` from json, the NATIVE engine
returns 9223372036854775807 (i64 max) — `k_b_to_int` parses via strtoll,
which SATURATES on overflow while still consuming every digit, so the
saturated value is silently accepted. The interpreter would produce the true
bignum → **engine divergence the differential lattice never caught** (no
huge-number golden existed). Fix: overflow detection in the native to_int
path with a bignum fallback (the int-tiering restart mechanism is the
designed home). Add a huge-number case to the golden corpus WITH the fix.
Also noted: float exponent rendering diverges from jq on exponent-form
values (kanso 1.5e-08 vs jq 1.5E-8) — parity edge, fixtures scoped around
it, revisit with the printer work.

apps/kq removed from this repo — kanso-lang/kq is the sole home (the
err-migration plan applies to it there).

---

## 2026-07-19 — GOAL: the memory-model deep frontier, ratified build order

Clay ratified (build in sequence): **memory frontier → module system →
lazy enumerable → build blocks → hako.** Strategic pivot alongside: stop
chasing narrow microbenchmark wins (qj is simdjson-class; beating it means
compiler-generated SIMD scanning — deferred, not urgent). Instead find
complex, holistic real-world workloads — robot tooling, production
utilities — where kanso earns its way into real use.

The frontier campaign, from design/memory-frontier-research.md, ground-
truthed against the tree today:

- **[OPEN] delete the 16-byte KHeader** (runtime.c:177) — VERIFIED still
  paid on every k_alloc_obj; codegen emits zero k_dup/k_drop calls, so the
  rc is written and never consulted. Settled doctrine says delete; pure
  win, cost-golden gated (alloc_bytes should drop materially).
- **[OPEN] cohort-birthday ratchet test** — the memo's highest-value
  verification target; does not exist yet. Write BEFORE regions/build-block
  work: adversarial property test of "cycles cannot cross birthdays."
- **[OPEN] free-the-top mini-rewind**; **[OPEN] generalized scalar rewind**
  (runtime.c:134 rule, using beat.rs SCALAR set); **[OPEN] three-way escape
  split** (dies-this-beat / MUST-survive born-in-shelf / MAY-survive
  bench+copy) — measure survivor volume on VSE first.
- **[OPEN] survivor machinery** (copy-or-pin split) + static sweep points
  for long beats — the two loose ends §03 of the compiler page names as
  planned-not-built.
- **[NOTE] ownership-analysis branch** (borrow/consume signatures +
  memory-model ratchet tests) is 3 commits but badly diverged from main
  (~11k lines); cherry-pick ownership.rs + tests forward rather than merge.
- **[STALE-CORRECTED] the memo's "k_b_push_mut is dead" claim**: it IS now
  wired (one codegen site) — the memo predates the wiring.
- Tag-hoist / auto-SoA / e-graph fusion stay queued behind the above; they
  attack the representation gap, not the free schedule.

(This goal was apparently stated last night and lost to a session usage
limit before it landed anywhere durable — hence this entry. If it's ever
unclear whether a directive got recorded: it goes here.)

---

## 2026-07-19 — KHeader deleted (merged); beat report built; THE VSE FINDING

**Merged (#48):** per-object header deleted. allocs unchanged 14,799,465;
alloc_bytes −19.0% (790,444,432 → 640,242,688); arena_blocks 6 → 5; peak RSS
6.8 → 5.8&nbsp;mb — kanso now under serde_json (6.7) on the memory column too.
Speed unchanged. Docs + book samples track the new numbers.

**Built:** KANSO_BEAT_REPORT=1 — beat.rs refactored around one classifier
(Verdict: Beat / PureLoop / ArgCrosses / OutsideTailCall / UsedAsValue);
`report` prints every self-loop's fate. jsonbench sanity: _bench/3 beats,
scanners are pure loops. The analysis is unchanged; all suites green.

**THE FINDING (measure-first paid off):** on VSE — the real workload —
**beat_iters=0. The heartbeat never fires.** 155 arena blocks, 158&nbsp;MB
peak RSS, pure grow-only: all 1000 trials' scratch retained to exit. Both
VSE loops reject as OutsideTailCall: they are mutual-recursion plumbing
(fold → step → fold). The memo's assumption that survivors were next is
WRONG for real code; the blocker is **loop-cluster coverage**:

- **[OPEN — next rung, slice 3] cluster beats**: bracket a tail-call SCC
  with a single plain-call entry, not just self-loops. Soundness: at every
  in-cluster tail edge, each arg must be scalar or transitively
  entry-threaded (a threaded-slot fixpoint over the SCC's edges — a bare
  param allocated mid-cycle is NOT entry-threaded; rewind would free it
  under a live register). Extend the report first, then codegen.
- **[OPEN — slice 4] the fold-state shelf**: expect VSE's fold accumulator
  to then reject as ArgCrosses — the acc IS the four-construct model's
  "state is a fold" case. Give the one threaded accumulator slot survivor
  treatment (shelf/copy-across) and the cluster rewinds around it. This is
  the memo's three-way split, scoped to the slot that matters on real code.
- Prediction to verify when both land: VSE peak RSS collapses ~158 MB →
  single-digit MB (one trial's scratch), the same flat-line the json
  gauntlet shows. That is the memory-frontier demo on a real program.

---

## 2026-07-19 — cluster beats built; VSE still grow-only; the REAL two blockers

Cluster analysis landed (tail-call SCCs via Tarjan; threaded-slot fixpoint so
a param allocated mid-cycle can never thread; entries must be plain calls;
codegen keys iter on cluster identity). All suites green; jsonbench golden
unchanged (self-loop path untouched). **But VSE: still beat_iters=0** — the
report shows why, and it is not mutual recursion:

- **Blocker 1 — tail ENTRY, not cycle:** `fold` (acyclic) tail-calls into
  `_fold_at`'s self-loop. An SCC never forms; the loop rejects as
  OutsideTailCall. **[OPEN — next] entry demotion:** when every tail-entry
  edge A→B comes from a group A in no tail cycle, demote those calls to
  plain calls (one bounded frame each; stack safety intact because A is
  acyclic) and B brackets normally. This unlocks the whole enumerable
  plumbing shape.
- **Blocker 2 — THREADED excludes list/closure:** `_fold_at coll f acc i`
  threads a list and a closure hand-to-hand; both are excluded today (the
  map memoization hazard generalized). A bare, never-rebound entry-threaded
  CLOSURE or REC is sound (no lazy internal mutation; captured pointers are
  below the mark by construction). LIST needs litigation (shared-buffer
  growth is about push, not threading — but prove it). **[OPEN] extend the
  entry-threaded rule to closure/rec after an adversarial review; list only
  with a written soundness argument.**
- Prediction stands: with both, VSE collapses from 155 blocks / 158 MB
  grow-only to single-digit MB. Then the fold accumulator (acc) becomes the
  live ArgCrosses case and the shelf work begins on real data.

---

## 2026-07-19 — GAVELED: build tail-entry demotion + THREADED extension

Clay greenlit both rungs. The implementation spec, so execution is mechanical:

**Rung A — tail-entry demotion (beat.rs + codegen emit_tail):**
1. beat.rs: find groups whose ONLY rejection is OutsideTailCall and whose
   every outside tail-caller is acyclic (in no tail-call SCC). Emit a demote
   set: (caller decl index, callee group). Callee joins the beat map.
2. codegen emit_tail: a tail call matching a demote edge is emitted as a
   PLAIN call — the existing beat_entry push/pop bracket then applies —
   followed by ret of the result (mind ret_ty conversion and the %parsed
   exclusion; record the fails set as plain calls do).
3. Stack safety argument: demoted callers are acyclic, so each adds one
   bounded frame; musttail everywhere else is untouched.
4. Gates: unit fixture (acyclic entry + self-loop → beat); jsonbench
   beat_iters=150 unchanged; goldens byte-identical; PR + x86 REQUIRED.

**Rung B — entry-threaded closures/recs (NOT lists yet):**
1. New ENTRY_THREADED = THREADED | CLOSURE | REC | DESC, used ONLY in the
   bare-own-param rule (a); scalar rule (b) unchanged.
2. Pre-req adversarial check: verify in runtime.c that closure/rec/desc have
   ZERO lazy internal mutation (maps memoize a sorted view — that is the
   hazard class; grep every write into an existing object). Captured/field
   pointers are below the mark by construction when the value itself is.
3. LIST stays excluded until a written soundness argument covers shared-
   buffer growth (push into below-mark spare capacity) — litigate separately.
4. Gates: closure-threaded loop fixture beats; map-threaded fixture still
   rejects; full suite; PR + x86 REQUIRED.

**Order: A, measure VSE, then B, measure again.** Prediction on record: both
landed → VSE beat_iters > 0 and peak RSS collapses 158 MB → single digits.
Then acc becomes the live ArgCrosses case and the shelf work starts.

---

## 2026-07-19 — rung A (tail-entry demotion) built; VSE's true wall is the acc

Demotion works: an acyclic tail entry into a self-loop is emitted as a plain
call (one bounded frame) and the loop brackets — fixture proves it (native ==
interp, beat fires), cyclic callers can never demote (test pins it),
jsonbench golden holds. **VSE still 0 beats, and now we know the whole
chain:** `_fold_at list acc f i` self-tails with a LIST param, a CLOSURE
param, and `(f acc x)` — a freshly-computed heap accumulator. So:
- the OutsideTailCall verdict MASKS ArgCrosses (classify priority) — report
  should surface both; minor, note for the next pass;
- rung B (closure/list threading) is necessary but NOT sufficient;
- **the wall is the accumulator: the fold-state shelf.** The acc is born
  above the mark each iteration and must survive the rewind. Design: at
  k_beat_iter, copy the one surviving slot down to the mark (the survivor
  double-write, scoped to the accumulator the analysis names) — the memo's
  three-way split arriving exactly where the four-construct model said state
  lives. THIS is the deep frontier's real build; spec it with adversarial
  care (copy must be transitive over the acc's reachable graph — a list acc
  reaches spine+elements; measure cost on VSE before committing).

---

## 2026-07-20 — rung B MERGED (#51); the LIST-threading draft and its landmine

Entry-threaded closures/records/descriptions are on main, full x86 green.
The mutation audit behind it: k_closure/k_rec/k_mkdesc write only at
construction; the runtime's only post-construction writes are the map's
cached sorted view and the list buffer's used count. Fixtures pin the
closure-threaded beat firing and the map-threaded rejection.

**LIST threading — draft soundness argument for the next hand:**
- k_b_push on a below-mark list writes an integer (buffer used count) and an
  element into below-mark spare capacity; the threaded KList header is never
  mutated. An above-mark element in a below-mark slot is unreachable after
  rewind (only above-mark headers had len covering it, and they died); a
  stale used count merely degrades later pushes to the copy path. Safe —
  unlike maps, which store an above-mark POINTER (the sorted view) into a
  below-mark header: instant dangle.
- **THE LANDMINE: k_b_push_mut.** The linear in-place push mutates the
  existing header and, on capacity growth, reallocates items to a fresh
  above-mark buffer — a below-mark threaded list that is also an in-place
  target ends pointing above the mark. Rewind, dangle, corruption. LIST
  threading may land only with an analysis-level guarantee that
  in_place_pushes and beat-threaded params never intersect, plus an
  adversarial test of exactly that overlap. Full care, x86 gate.
- VSE after rung B: still 0 beats, as predicted — _fold_at waits on LIST
  threading AND the computed accumulator. **The fold-state shelf is the
  frontier's next build.**

---

## 2026-07-20 — THE FOLD CARRY SHIPS; the last wall is pipe-loop recursion

The fold-state shelf is built and firing. Design: per-beat ping-pong malloc
buffers; staged args are deep-copied (measure pass first, so the buffer
never grows mid-copy) strictly BEFORE the rewind — source and destination
cannot overlap, no pointer rebasing. The survives-rewind test doubles as
the sharing preserver: below-mark data inside a carried value is shared,
not copied, so a threaded list inside a carried record costs nothing. At
the pop, a heap result is copied out to the caller's arena. KClosure gained
its capture count (deep copy needed it) — the cost golden moved +16 bytes,
one closure allocation in the gauntlet, defended here.

Analysis: ArgCrosses is now CarryBeat (≤8 positions); demotion composes
(crossing args ride as carried); a call through a closure VALUE counts as
may-allocate (the profitability gate was hiding _fold_at).

**VSE, measured:** beat_iters 0 → 5,303,200 (every fold iteration in the
simulation); arena blocks 155 → 104; peak RSS 158 → 112 MB; wall time
2.25s → 1.63s — the carry made VSE 27% FASTER (warm-cache rewinds).

**The remaining wall, named:** VSE's outer loops recurse through
pipe-bound lambdas — `cloud ... . (cp -> trials (k - 1) ...)` — the
idiomatic bind style. tail_exprs never sees a lambda body, so the trial
loop is invisible to the analysis (and pipes don't musttail either: pipe
recursion is O(depth) stack today). **[OPEN — next rung] pipe-loop beats:**
recognize `x . (p -> ... self ...)` tail-recursion-through-bind, bracket
it, and let captured accumulators ride the carry. That is where the
158→single-digit prediction gets its verdict.

---

## 2026-07-20 — the carry MERGED (#53) with the growing-accumulator gate; VSE 15x

The book gate caught what no unit test saw: carrying a growing accumulator
(push acc x feeding its own slot) copies quadratic bytes — the ch10
teaching program went 33KB → 16MB of traffic. The gate keeps growth on the
grow-only path (bounded fixed-shape rebuilds still carry; closure-hidden
growth is the cost-bound frontier's case). And gating _range_to's growing
carry deleted most of VSE's runtime: **2.25s pre-campaign → 1.63s with the
carry → 0.15s with the gate — 15x — output exact to the last digit.**
beat_iters 4.2M; RSS ~112MB pending the pipe-loop rung (VSE's outer loops
recurse through pipe-bound lambdas, invisible to tail_exprs, O(depth)
stack — the open rung that decides the single-digit prediction).

Session verdict: every gate fired correctly today — x86 CI caught clippy
drift, branch protection refused a premature merge, the book rule caught
the quadratic carry, the cost golden held throughout. Measure first, let
real code pick the rung, write the soundness argument before the code.

---

## 2026-07-20 — pipe inlining SHIPS (desc-gated); the final rung is cluster-carry

The inline broke effects on first cut — concurrency.kso went silent, because
on a DESCRIPTION the pipe is the executor's bind, not an application. The
fix: inline only when inference proves the piped value cannot be a desc
(set & DESC == 0); otherwise the k_maybe_bind path stands. All suites,
goldens, book samples green; VSE output exact.

What it bought: tail pipes into literal lambdas are now real musttails
(constant stack where pipe recursion was O(depth)) and visible to the beat
analysis. What it revealed: VSE's outer loop is a TWO-GROUP CYCLE
(trials ↔ _with_voters via pipe-lambdas) whose accumulator crosses on
cluster edges — and the carry only exists for self-loops. **[OPEN — the
rung that decides the RSS prediction] cluster-carry composition:** per-edge
carried positions on in-cluster tail edges, same staging machinery,
growing-accumulator gate per edge. cloud's push-acc loop is correctly
gated as growing; the trials tally is bounded and should carry.

---

## 2026-07-20 — cluster-carry COMPLETE; and the true final rung: EXECUTOR BEATS

Cluster-carry works end to end: the minimal cycle carries both directions
(engines exact); the from-stub collection bug and the empty-set-is-scalar
hazard are fixed (an EMPTY inferred slot set means "entered only through
lambdas" — unknown, now carried, never assumed threadable — that hazard
could have rewound over live values). VSE emits complete carries+musttails
in both directions of the trials cluster, output exact.

And the instrumented build revealed why the emitted path never runs:
`point` uses random — an EFFECT — so VSE's whole driver is a lazy
DESCRIPTION chain. The recursion executes inside the EXECUTOR's bind loop
(runtime.c:832, next = k_call1(d->y, yielded)), which has no brackets. The
pure folds beat; the effectful spine grow-onlys. **[OPEN — the real final
rung] EXECUTOR BEATS:** bracket the executor's bind step directly in the
runtime — push at chain start, per-step: carry the yielded value (deep
copy machinery already shipped), rewind, continue. Runtime-only, no
analysis, and it gives EVERY effectful kanso program flat memory
universally — request loops, control loops, robot loops: the exact
production shapes the strategy targets. Design care needed: what else
survives a bind step (the continuation closure's captures; nested joins);
adversarial tests first. This rung decides the RSS verdict for real.

---

## 2026-07-20 — BUG [OPEN, HIGH]: register-return ABI mismatch on the canonical destructure

A five-line program — `type user` + `fn foo (user age name)` + `main = foo
(user 44 "clay")` — fails the native build: the callee's parameter carries
%parsed (escape analysis: construct-then-destructure, register-returnable)
but the call site emits the construction as %KValue. The interpreter runs it
fine; the REPL (interp) fine. This is the same register-return machinery the
err-migration's union-return blocker lives in — fix them together. Repro
saved in the session ledger entry; the shape is escape.rs's own
construct_then_destructure_is_returnable test, which passes at the ANALYSIS
level while codegen's call_arg/abi_params disagree about the construction
site's type. Found live while answering a syntax question — the canonical
teaching form crashes the compiler.

---

## 2026-07-20 — DONE: register-return ABI mismatch on a nullary record constant

The construct-then-destructure crash (logged HIGH above) was still live for a
nullary case: a constant like `clay = user 44 "clay"` compiles to a
register-returnable `d_clay_0` returning `%parsed`, but the identifier-reference
call site in codegen.rs hardcoded `call tailcc %KValue @d_clay_0()`. The
`{i64,i64}` register struct was then read as a `%KValue`, and the consumer
(`age_of clay`, destructuring the record) segfaulted — native exit 139 while the
interpreter printed 44, an engine divergence the lattice never caught (no
golden exercised a nullary record constant). Fix: the nullary reference path now
uses `ret_ty(name, 0)` for the call's return type and `record_parsed` on a
`%parsed` result, mirroring the n-ary call paths. Regression: `examples/
register_return.kso` (native + differential). The n-ary call sites already
consulted `ret_ty`; this was the one gap.

---

## 2026-07-21 — FINDING: laziness memory model — RC beats regions; thunk-graph experiment confirms

Committee deep-dive (three research passes + two adversarial verifications)
plus a working prototype settled the lazy memory-model question.

**Regions lose under laziness.** A thunk's forcing point is a runtime fact,
so a thunk can force a region to outlive static inference. Every prior
system hit this seam: jhc (region-only Haskell) leaks by its own docs; GHC
Compact Regions cannot hold a thunk; ML Kit needed a GC backstop even
strict. Regions demote to a back-end optimization (bump-allocate clusters
proven to share a lifetime); they are not the model.

**RC wins because kanso's data graph is acyclic.** Immutability + no cyclic
references means refcounting is COMPLETE — every cell freed exactly when
its last reference drops, no tracing GC. The one cycle-maker is knot-tying
corecursion (`ones = 1:ones`, a physically self-referential cell). Ruling
(pending gavel): corecursion is generators/unfolds — fresh cell per step,
no self-reference — so cycles never enter the heap. Verified prior art:
Perceus (PLDI'21), Frame-Limited Reuse (ICFP'22), FP2 (ICFP'23) are all
strict; "First-Order Laziness" (ICFP'25, Distinguished) grafts RC+reuse
onto a first-order lazy fragment and names general lazy closures as open.
Pervasive-arbitrary laziness + precise RC+reuse is unoccupied ground.

**Experiment** (scratchpad/thunk-rc, instrumented Rust): refcounted
self-updating thunks, no tracing GC. 21.1M thunks allocated; after all
workloads exactly 1 cell live — the deliberately-leaked knot negative
control. Numbers: conditional demand (100k items, 5% used) lazy beats
eager-as-written 17.8x, only ~15% behind hand-restructured eager; thunk
tax 23 ns/alloc+force (what strictness analysis erases for provably-
demanded values); 1M-deep foldl chain builds visibly (peak 1M cells) and
is fully reclaimed by RC alone after force; 10M-element infinite fib
stream as generator runs at peak 2 live cells, memory flat; forcing drops
the captured env (8MB buffer freed at force — retention bounded by
demand). Perceus-style upgrade (defunctionalized first-order thunk states
+ free-list reuse of count-0 cells): same 10M stream in 10.9 ns/elem, 2.4x
faster than Rc+closures, allocator traffic 2 mallocs + 9,999,999 reuses —
steady-state zero malloc/free for an infinite stream.

**Open threads:** (1) pervasive-arbitrary vs structured/first-order lazy
forms — the risk gavel; (2) trampoline deep forces (1M chain needed a fat
stack); (3) speculative forcing during executor IO stalls (purity makes
mis-speculation free: no effects to undo; store err in the thunk, never
raise early); (4) incrementality is NOT free from the thunk graph
(Adapton-style dependency tracking is extra machinery, ~30ns/node — a
future opt-in layer, not a default); (5) atomic vs biased counts for
cross-thread sharing under the deterministic scheduler.

---

## 2026-07-21 — RULED: ship the proven lazy fragment; pervasive-arbitrary is a staged campaign

Clay's ruling on the open thread above: v1 implements the experimentally
verified fragment — compiler-defunctionalized RC thunks, generators-first
corecursion (knot idiom banned), free-list reuse. The pervasive-arbitrary
bet (arbitrary-closure thunks, the unoccupied research ground) is NOT
abandoned: it's a later campaign, entered the same way — prototype
experiments with instrumented counters first, engine work only after the
numbers hold. As laziness lands, add MEMORY GOLDENS alongside the stdout
goldens: golden files asserting structural/memory facts (exit_live=0,
per-site evaluation counts, steady-state allocator traffic) so
leak-freedom and lazy semantics are differentially PROVEN per program,
not just believed. Sharing (evaluated once), skipping (evaluated zero
times), and reclamation (exit_live=0) are semantics, so both engines must
agree byte-identically — the differential lattice extends to memory.

Note for the campaign: kanso compiles whole-program (no separate
compilation, no dynamic loading), so EVERY thunk shape is statically
enumerable — Reynolds-style total defunctionalization. ICFP'25's named
obstacle (open/library-extensible lazy constructors) may not exist here
at all: the "structured fragment" could grow to look pervasive without
ever admitting arbitrary runtime closures. The gap Clay staged around may
partially collapse in kanso's favor.

---

## 2026-07-21 — REFINEMENT: proven-demand thunks are risk-free out-of-order work

Clay's point sharpens the speculation thread: for a PROVABLY-demanded
value, computing during an IO stall isn't speculation — demand is proven,
so the work is guaranteed useful; only its timing moves. Out-of-order
execution at the language level, thunk pool as instruction window. So the
per-site representation decision is demand x cost x slack, not demand
alone: proven+cheap+no-slack compiles inline strict (cell costs more than
the work); proven+expensive+slack materializes a thunk into the work pool
(scheduler drains it during stalls — zero risk); unproven+expensive is
speculation-eligible (spends free stall cycles); unproven+cheap stays a
thunk for semantics (may err/diverge). Constraints: deterministic
schedule (heartbeat logical time, both engines byte-identical) and a
bounded pool depth (deferred envs hold memory until run). Fits the staged
ruling: v1 representation unchanged; scheduler-drains-pool lands on top.

---

## 2026-07-21 — FUTURE THREAD: in-process plugins as shared-nothing units (weeks out, not now)

Clay's sketch, noted for the future conversation: plugins as a performant
in-process analog of RPC. Each plugin compiles as its OWN whole program
(monomorphization, coherence, defunctionalization, RC-completeness all
hold per unit); only FORCED, acyclic values cross the boundary
(deep-copied — semantically invisible since values are immutable);
separate memory graphs per plugin (unload = drop the graph, nothing can
dangle); dispatch closed at the boundary (no arm injection — extension
points are explicit interface functions); errs surface at the crossing.
Streams cross as protocol (pull interface, forced chunk per call), not as
cells. Prior art: Erlang per-process heaps, WASM component model. Same
boundary contract can tier across in-process / WASM-sandboxed /
subprocess. Tradeoffs accepted: monomorphic boundary, copy cost, no
cross-boundary slack scheduling. NOT scheduled — revisit when plugins
become real.

---

## 2026-07-21 — FUTURE THREAD: strict mode as a worst-case benchmark tool

Clay's suggestion: a thunk-free diagnostic mode so performance-sensitive
code can be timed at its worst case (every deferred computation forced).
Nearly free to build — the demand pass is the single thunk gate, so
`--strict` = demand returns empty and everything compiles today's strict
paths (the KANSO_NO_LAZY debug hack during force-wiring was exactly
this). It is a MEASUREMENT mode, not a semantics switch: forcing runs
what laziness would skip, so skip-reliant programs may differ in output
(skipped_err pins the case). Complement: thunk_allocs - thunk_evals in
the .mem counters already reports the skip rate without a rebuild.
Dev-tooling tier, with the LSP.

---

## 2026-07-21 — FUTURE THREAD: sync blocks — scoped strictness as a guarantee

Companion to the strict-mode thread above (Clay): thunk mode can hold
more PEAK memory (cells + captures live until forced), so beyond the
whole-program measurement flag, a `sync`-style construct would mark a
SCOPE as no-deferral — compute now, hold no cells, peak memory equals
strict memory. Same single gate implements both (the demand pass skips
marked scopes); the mode measures, the construct guarantees. Candidate
gavel when the surface syntax conversation happens.

## 2026-07-22 overnight: the lazy tax, and types that alias

The serde regression root-caused by ratio bisect (ratio is
machine-noise-proof; absolutes are not). Lazy v1 (#83) moved
kanso/serde from 0.85 to 1.33 in one merge while creating zero thunks:
conservative TOP widenings carry the THUNK bit, and 133 static k_force
call sites landed in the strict decoder's hot loops as external no-op
calls. Two-part fix (#105): a program whose demand analysis deferred
nothing skips every force site — no thunk can exist anywhere — and
live-thunk programs pay one alwaysinline tag compare (k_force_fast)
instead of a call. Post-fix same-night interleave: kanso/serde 0.949.
Lesson for every future pass: a bit added to TOP is a cost added to
every conservatively-typed hot path; gate emission on whether the
feature is live in THIS program, not on the lattice alone.

Type enrollment identity (#113): clones forked type identity — a
bare-constructed `cursor` never matched std's `list/cursor` arms.
Ruling: types alias, never fork. TypeDecl.origin marks clones, records
tag with the canonical name, one post-check pass canonicalizes
patterns and typeset members (type positions cannot be shadowed, so
the rewrite needs no scope analysis), and both backends give aliases
their origin's type id, skipping them in the name/field switch tables.
Beat-demotion consistency (#99 fallout, fixed same night): a demoted
entry pair lives or dies with its target loop, never with the caller's
name — a clone sharing the caller's name had dropped the bracket while
the loop kept its rewinds, corrupting live memory.
