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

## 2026-07-22 day: the enumerable lands whole — and ends up faster

Phase one (#134): std/list becomes the ratified pull model — adapters
return iterator records, consumers drive next, one element in flight.
vse/squeeze went idiomatic-std with outputs bit-identical on both
engines; the json decode path untouched (cost golden exact). Honest
tax at this stage: vse user-time +60%.

Generators (#136): cycle/naturals/repeat/iterate as ordinary records;
the spec's infinite examples run verbatim; no stream construct exists.

Phase two (#137) erased the tax with two composable moves. Typed fold:
per-iterator arms drive the protocol, the generic arm runs the indexed
loop — dispatch picks the fast path, no analysis. The fusion pass
(shared AST rewrite, post-check): consumer over map/select/reject
chains → one fold over the root, adapter steps composed into the
reducer. The typed arms make the rewrite sound for ANY root, which is
the load-bearing trick — no list-ness proof, no escape analysis, and
module re-export graphs are handled by naming a real resolved fold
decl. take/first never fuse, so infinite sources keep their meaning.

Receipts: vse 0.20s user, BELOW the eager library's 0.22 — lazy
semantics now cost less than the code they replaced. The ch10 counters
sample fell 4033 → 29 allocations. Follow-ups queued: fuse take/drop
bounds into the scan, tally/group_by/tso_h reducers, and the
defunctionalized-thunk pool sharing this composition machinery.
## 2026-07-22 evening: fusion is syntactic — keep the chain in sight

The kq pretty-print race exposed the pass's blindness: a width-forced
binding split (`rows = map …` then `to_list rows`) hides the chain, so
no fusion, so per-element wrapper records return. Convention until the
pass learns better: name the LAMBDA, nest the chain. Queued: fusion
through single-use local bindings (the binding is a rename, not an
escape — provable cheaply).

## 2026-07-22 night: the encode crasher — latent, pinned, unsolved

Building the encode-side profile for the kq pretty gap surfaced a
native crash: decode bench/large.json once, encode it in a self-tail
loop, and the SECOND iteration segfaults (n=1 clean, n=2 dies; two
INLINE encodes without the loop are clean). Repro: bench/encodebench.
The crash stack (macOS .ips): k_b_at <- fold dispatch <- d_encode_1
<- fused lambda <- d_encode_1 <- d_rounds_3, faulting on an address
whose bytes are iteration-one ENCODE OUTPUT — a KValue payload holding
string content where a list pointer belonged.

Ruled out tonight, with receipts: NOT the enumerable migration (the
pre-enumerable #133-era binary crashes identically); NOT the map
sorted-view-cache-above-the-mark hazard (instrumentation shows zero
beat rewinds run during the loop, and a cache-reset sweep on every
rewind path did not change the crash — the sweep was reverted as an
unproven guard). The loop does NOT beat-iterate; the arena only grows
during it, so the stale-looking payload is corruption, not
use-after-rewind. Suspect space for the next session: the bind-chain
pulse's carry evacuation interacting with values returned from
k_call1, or an ABI/boxing edge in the fold dispatch under deep
encode recursion. The interp runs the same program correctly, and the
.ips reports in ~/Library/Logs/DiagnosticReports carry full stacks.

## 2026-07-22 late: the encode crasher falls — two rewind holes

The bench/encodebench hunt (opened as "latent, pinned, unsolved")
closed with two real mechanisms, both against the beat machinery:

1. Lambda entries bypass demotion. The demotion analysis draws its
   caller set from named declarations, so a lifted lambda tail-calling
   into a beat-headed loop musttailed in with NO bracket — the loop's
   per-iteration rewinds unwound to the ENCLOSING mark and freed the
   caller's live data (the decoded tree being encoded). Codegen now
   demotes ANY tail entry into a beat-headed loop from outside its
   cluster. Minimal repro: [[1],[2]] encoded twice through a
   strict-accumulator tail loop — nine bytes of json.

2. Above-mark caches in below-mark headers. With brackets real, map
   sorted-view caches filled during iteration one (allocated above the
   loop's mark, pointed to from headers inside the threaded tree below
   it) went stale on rewind. Every rewind path now sweeps a registry
   of beat-era cache fills, resetting exactly what it frees.

Debugging law that earned its keep: flaky-by-layout crashes become
deterministic under KANSO_POISON=1 (rewound memory filled with 0xAB),
which is now a permanent runtime flag. The regression is a runtime
golden (reencode) exercising both layers on both engines. Cost golden
and vse stayed bit-exact through the whole fix.

## 2026-07-22 — the encode campaign lands at 3.5x

Encode of bench/large.json (400 rounds): 3.46s user at the start of
the campaign, 1.00s at the end. Four cuts, each found by sampling and
each shipped behind the full 12-suite gate:

1. Numbers and templates (#145). The float renderer probes precision
   15..17 instead of 1..17 (dtoa and vfprintf left the profile
   entirely), ints render through a hand k_itoa, interpolation
   templates concatenate through one k_concat_arr call (an array
   parameter, not varargs — 16-byte struct varargs disagree between
   arm64 and x86_64 SysV), and join writes into a buffer it then
   wraps instead of recopying.

2. Escape on bytes (#146). escape_char dispatched on single-character
   string literals — a memcmp probe per character of every string
   encoded. The pipeline became decode-symmetric: bytes in, int arms
   (a jump table), one utf8 out.

3. The clean-string scan (#148). find2_below — find2 with a floor —
   proves in one SIMD pass that a string holds no quote, backslash,
   or control byte, and the overwhelmingly common clean string passes
   through in a single copy. The bump allocator also inlines into its
   callers now (the refill path stays out of line; counters stay
   exact on both paths).

4. The byte builder (#148). The structural cut: the old encode
   re-copied every child's bytes at each nesting level (template
   wrap, join, parent template — six copies per byte on a flat
   document). append is a builtin bytes accumulator with a KBuf
   header that claims its frontier the way list push does, so a fold
   of appends is amortized linear under plain value semantics. encode
   and escape thread one builder end to end.

The beat analysis grew two conservative rules on the way: a crossing
slot whose inference set is empty, or which may hold a byte builder,
is never assumed cheap to carry — rewind-copying a growing buffer is
quadratic where grow-only is linear. Both engines byte-identical
throughout; cost golden exact.

OPEN: decode still assembles escaped strings through text/concat on
byte lists — the builder should serve both directions. kq's pretty
renderer still templates per row; folding it onto the builder is the
next pretty-path cut.

## 2026-07-23 — quiet floors for the write path

The idle-machine sitting landed after the campaign closed. encodebench
(188KB × 400): **0.66s user** — the 1.00s closing number carried
browser load; quiet-to-quiet the campaign is 3.46 → 0.66. kq boards,
interleaved best-of-N, idle, byte-identity gated: path 3.0ms/13.9ms
(1.62×/1.78× over jq), pretty 6.5ms/49.7ms (1.97×/2.11×), kq 80/80.
Pretty quiet-to-quiet: 12.0 → 6.5ms small, 109 → 49.7ms big. kq README
carries the table; site prose stays design-only per the no-narration
directive.

## 2026-07-23 — FOUND: lazy v1 thunk counting is scaffolding, not counting

Audit prompted by Clay's "does the lazy tier use Perceus?" — answer:
no by design (only thunk cells count; values stay count-free under
the arenas; no dup/drop calculus), and TODAY not even that: KThunk.rc
is set to 1 at creation and never touched again, nothing pushes cells
back to k_thunk_free (the free list is only ever popped, i.e. always
empty), and forced cells hold their cached result until process exit.
thunk_live_exit is allocs minus evals — derived arithmetic, not
evidence of freeing. The 21.1M-cell recycle numbers came from the
ratified PROTOTYPE; the engine inherited the struct field and the
free-list plumbing but not the drop insertion (the piece §06 flags as
memory-unsafe to rush). Bounded in practice by the cost gate (JSON
gauntlet: zero thunks, golden-pinned); a long-running lazy-heavy
program would accumulate cells.

DECIDED: (1) compiler page §07 status corrected in the same PR as
this entry — "freed the instant its last reference drops" was
prototype behavior, now marked designed/unbuilt for the engine.
(2) Cell-RC wiring (retain on capture/copy, compiler-inserted release
after force and at last use) joins the mined queue as item 0 — a
correctness-of-claims item ahead of Dragonbox, and a prerequisite for
the still-open pervasive-lazy gavel.

OPEN: the release-site insertion is the careful part (codegen-emitted
drops; adversarial goldens for shared-thunk, escaping-thunk, and
err-carrying-thunk cases before it may land). The .mem golden vein
must grow a freed-cells counter so recycling is pinned, not believed.

## 2026-07-23 — the gavel-queue/tiers appendix moves off the public page

Clay: decision-process content ("executive calls—ratify or reverse"
and its neighbors) is internal, not public. The whole appendix block
(standing gavel queue + tiers 01–03) moves here verbatim as a
historical snapshot; several tier-03 items were already stale on the
page (short-circuit and/or shipped as && / ||; negative literals and
% shipped). The page now ends at the mushroom test.

```html
<hr>

<h2 id="queue">standing decisions—the gavel queue</h2>

<p><em>temporary section. these are the still-open rulings; as each one lands it graduates from this list into the essay above, and the section ends at zero.</em></p>

<ul>
<li><strong>pub visibility</strong>—<code>_name</code> as module-private has two real defects: promoting a name renames every call site, and <code>_</code> already means wildcard and deliberately-unused. the <code>pub</code>-modifier draft is settled in shape—private by default, every arm of a public group marked, api surface greppable in one pass—and awaiting the gavel.</li>
<li><strong>parameter typesets</strong>—fields ship; parameters wait on the annotation-redundancy checker, so a guard the body already derives can be rejected as clutter before parameter guards become legal.</li>
<li><strong>dispatch positional-field fragility</strong>—the destructuring ruling deleted positional binds because a type growing a field reshuffles them; dispatch arms still consume fields positionally, which is the same flank unguarded. wants litigation.</li>
<li><strong>kq repo publish</strong>—committed and ready at <code>~/dev/kq</code> with a readme and the jq benchmark; publishing awaits an explicit go.</li>
<li><strong><code>kanso test --native</code></strong>—tests still interpret; heavy suites are exactly where the build-and-run crossover bites.</li>
<li><strong><code>&gt;&gt;</code> statically effect-only</strong>—runtime-enforced today; wants the effect-inference pass.</li>
<li><strong>imports slice b</strong>—slash qualification, the prerequisite for retiring vendored copies of kanso-json.</li>
<li><strong>self-hosting horizon</strong>—a kanso lexer written in kanso is the next dogfood library once file-io effects and imports land; the long line points at the compiler reading its own language.</li>
</ul>

<hr>

<span class="eyebrow">appendix—field report</span>
<h1>the json gauntlet</h1>

<p><em>this was the site's first receipts page, kept verbatim as the record of the calls that built kanso-json; the rulings it provoked now live in the essay above.</em></p>

<p>the fastest way to find out whether a language is real is to make it earn a living. so we ported the job description of go's most-used package—<code>encoding/json</code>—into kanso: a complete decoder and encoder, escape handling including <code>\uXXXX</code>, failure positions, canonical output. it exists, it passes sixteen tests under <code>kanso test</code>, and every judgment call made along the way is recorded here in three honesty tiers: things the exercise <em>proved</em>, executive calls awaiting ratification, and friction we refuse to pretend we didn't feel.</p>

<h2 id="proved"><span class="sec-num">tier 01</span>what the gauntlet proved</h2>

<p><strong>dispatch is a parser's native language.</strong> a recursive-descent parser is one long "what character am i looking at?"—and kanso's literal-dispatch overloads <em>are</em> that question. the tokenizer's decision tables read like tables:</p>

<div class="code-panel">
  <div class="code-panel-title">lib/json/json.kso</div>
  <pre><code><span class="k">fn</span> <span class="f">value_for</span> <span class="s">"\""</span> cs p
  <span class="f">parse_string</span> cs (p <span class="o">+</span> <span class="n">1</span>)

<span class="k">fn</span> <span class="f">value_for</span> <span class="s">"["</span> cs p
  <span class="f">parse_array</span> cs (p <span class="o">+</span> <span class="n">1</span>)

<span class="k">fn</span> <span class="f">value_for</span> <span class="s">"t"</span> cs p
  <span class="f">word</span> cs p <span class="s">"true"</span> <span class="t">true</span>

<span class="k">fn</span> <span class="f">value_for</span> <span class="t">none</span> _ p
  <span class="f">fail</span> p <span class="s">"unexpected end of input"</span></code></pre>
</div>

<p><strong>auto-propagation deleted the error plumbing. all of it.</strong> the parser contains not one line of "check if the last step failed." a failure born anywhere—bad escape, invalid number, truncated input—rides the return values past every continuation function (whose constructor-pattern arms simply don't match it) and surfaces from <code>decode</code> with its position intact. the happy path is the only path anyone wrote. this was the design's biggest bet, and it paid in full.</p>

<p><strong>nothing-wasted caught real noise.</strong> the compiler rejected seventeen dispatch arms for naming parameters they never used, forcing <code>_</code> discards that now document, in the signature, exactly what each arm consumes. annoying for ninety seconds; correct forever.</p>

<p><strong>end-of-input is not a special case.</strong> <code>at</code> past the end returns <code>none</code>, which propagates like any failure and gets caught by explicit <code>none</code> arms exactly where the grammar cares. eof handling cost zero new concepts.</p>

<h2 id="calls"><span class="sec-num">tier 02</span>executive calls—ratify or reverse</h2>

<p>each of these is implemented, tested, and reversible. defaults chosen by the house rule: the right thing, by default, with nothing to configure.</p>

<p><strong>1. <code>kanso test</code>.</strong> a test is a constant named <code>test_*</code> whose value is <code>true</code>. no framework, no assertion dsl—<code>==</code> on values is the assertion, because structural equality is already total. <code>kanso run</code> requires <code>main</code>; <code>check</code> and <code>test</code> don't (a library is valid kanso without an entry point).</p>

<p><strong>2. map literals.</strong> <code>{ "a": 1 "b": 2 }</code>, empty map <code>{:}</code>. keys are literals only (dynamic maps are built with <code>put</code>), and literal keys must appear sorted, without duplicates—a formatting error otherwise, consistent with fields and declarations. iteration order is always sorted-key order, so encoding is canonical for free.</p>

<p><strong>3. <code>entries m</code> yields <code>entry</code> records</strong> (fields <code>key</code>, <code>value</code>; the name is reserved). map traversal dogfoods records and constructor patterns instead of inventing tuples.</p>

<p><strong>4. numeric strictness.</strong> <code>int + float64</code> is an error, not a coercion; convert with <code>to_float</code>. floats render as <code>1.0</code>, never <code>1</code>; float division by zero is <code>err</code>, same as int. JSON numbers decode as <code>int</code> when written integral, <code>float64</code> otherwise.</p>

<p><strong>5. JSON null is <code>json_null</code>, not <code>none</code>.</strong> the honest reason: <code>none</code> is propagation-hostile as <em>data</em>—construct a record with it and the record never gets built, because propagation eats it. that's correct behavior for absence-as-failure and wrong for null-as-value, so null gets a marker type. this points at a real gavel: kanso may want zero-field types (<code>type null</code> with no body)—today a type requires at least one field, so the marker carries a dummy <code>bool</code>. it's the one visibly inelegant thing in the library.</p>

<p><strong>6. the allowed-error / defect split is a word, not a sigil.</strong> ruby marks the raising variant with <code>!</code>; kanso would have to double every api to do that. instead, <code>must</code> converts any allowed failure into a <code>defect</code>—two lines of ordinary overloads, composing with every function ever written. parse errors from user input stay handleable; <code>must (decode config)</code> declares "this failing is a bug," and the defect rides the rails to the root reporter. still owed: the endpoint rule treating <code>defect</code> as auto-reported rather than must-be-handled.</p>

<p><strong>7. small additions the work demanded:</strong> <code>push</code> (list accumulation), <code>chars</code>/<code>char_code</code>/<code>from_code</code> (the minimal unicode bridge), <code>join</code>, <code>slice</code>, string escapes <code>\t</code> and <code>\r</code>, and type-postfix brackets lexing tight (<code>json[]</code>) while list arguments stay spaced (<code>f [1 2]</code>). all prelude candidates for the import gavel.</p>

<h2 id="friction"><span class="sec-num">tier 03</span>friction—where a developer would sigh</h2>

<p><strong>no short-circuit and/or.</strong> we wrote <code>both</code> as a two-arm overload and it works, but eager evaluation means it can't guard (<code>both (p > 0) (expensive p)</code> runs both). candidate gavel: lazy <code>and</code>/<code>or</code> words with the same thunk mechanics <code>if</code> already uses.</p>

<p><strong>no negative literals, no modulo.</strong> <code>-1</code> is unwritable (only <code>0 - 1</code>), and <code>hex4</code> computes remainders by subtract-multiply. both feel like missing table stakes; both interact with the operator gavel that's already queued.</p>

<p><strong>alphabetical order scatters cohesion.</strong> the sixteen tests sort into the middle of the implementation, and helper families stay adjacent only because we <em>named</em> them into adjacency (<code>str_char</code>, <code>str_chars</code>, <code>str_escape</code>...). developers will name-game the ordering rule; that's a signal. modules will absorb most of it—tests want to be a sibling file—but the rule deserves a second look with this evidence in hand.</p>

<p><strong>lambdas can't destructure.</strong> encoding map entries needed a named <code>encode_entry (entry k v)</code> where a pattern lambda would have been one line. queued with the destructuring family.</p>

<p><strong>positions blur where <code>none</code> propagates far.</strong> most eof arms report exact positions, but a failure that rides many frames before conversion loses locality. the fine-grained-failure story (typeset-based propagation beyond <code>err</code>/<code>none</code>) is the real fix.</p>

<p>the library lives at <a href="https://github.com/kanso-lang/kanso-json">github.com/kanso-lang/kanso-json</a>, and runs in kanso's ci on every push.</p>

<div class="lore"><figure><svg class="sprite" viewBox="0 0 22 19" role="img" aria-label="err" shape-rendering="crispEdges"><title>err - always arrives, never uninvited</title><rect x="7" y="2" width="8" height="1" fill="#f03a00"/><rect x="6" y="3" width="10" height="1" fill="#f03a00"/><rect x="5" y="4" width="12" height="1" fill="#f03a00"/><rect x="4" y="5" width="1" height="1" fill="#f03a00"/><rect x="5" y="5" width="2" height="1" fill="#ff7a52"/><rect x="7" y="5" width="11" height="1" fill="#f03a00"/><rect x="4" y="6" width="1" height="1" fill="#f03a00"/><rect x="5" y="6" width="2" height="1" fill="#ff7a52"/><rect x="7" y="6" width="11" height="1" fill="#f03a00"/><rect x="3" y="7" width="16" height="1" fill="#f03a00"/><rect x="3" y="8" width="4" height="1" fill="#f03a00"/><rect x="7" y="8" width="1" height="1" fill="#2b2320"/><rect x="8" y="8" width="1" height="1" fill="#faf3e3"/><rect x="9" y="8" width="4" height="1" fill="#f03a00"/><rect x="13" y="8" width="1" height="1" fill="#2b2320"/><rect x="14" y="8" width="1" height="1" fill="#faf3e3"/><rect x="15" y="8" width="4" height="1" fill="#f03a00"/><rect x="3" y="9" width="4" height="1" fill="#f03a00"/><rect x="7" y="9" width="2" height="1" fill="#2b2320"/><rect x="9" y="9" width="4" height="1" fill="#f03a00"/><rect x="13" y="9" width="2" height="1" fill="#2b2320"/><rect x="15" y="9" width="4" height="1" fill="#f03a00"/><rect x="2" y="10" width="5" height="1" fill="#f03a00"/><rect x="7" y="10" width="2" height="1" fill="#2b2320"/><rect x="9" y="10" width="4" height="1" fill="#f03a00"/><rect x="13" y="10" width="2" height="1" fill="#2b2320"/><rect x="15" y="10" width="5" height="1" fill="#f03a00"/><rect x="3" y="11" width="4" height="1" fill="#f03a00"/><rect x="7" y="11" width="2" height="1" fill="#2b2320"/><rect x="9" y="11" width="4" height="1" fill="#f03a00"/><rect x="13" y="11" width="2" height="1" fill="#2b2320"/><rect x="15" y="11" width="4" height="1" fill="#f03a00"/><rect x="3" y="12" width="16" height="1" fill="#f03a00"/><rect x="3" y="13" width="6" height="1" fill="#f03a00"/><rect x="9" y="13" width="4" height="1" fill="#2b2320"/><rect x="13" y="13" width="6" height="1" fill="#f03a00"/><rect x="4" y="14" width="14" height="1" fill="#f03a00"/><rect x="5" y="15" width="12" height="1" fill="#f03a00"/><rect x="6" y="16" width="10" height="1" fill="#f03a00"/><rect x="4" y="17" width="13" height="1" fill="#f03a00"/></svg><figcaption>err—always arrives, never uninvited</figcaption></figure></div>

```
## 2026-07-23 — PLAN: cell-RC wiring (mined queue item 0)

The demand fragment constrains where thunk cells can flow, and the
plan leans on it. A lazy bind's every use is a direct argument at a
discard-capable dispatch position (demand.rs guarantees this — any
other use kind keeps the binding strict). So cells live in: the
creating frame's register, callee parameter registers below it, and
other cells' capture slots. Structures can only reach a cell through
a CALLEE ARM's handling of its parameter — the one hole.

Mechanism, three pieces:

1. Runtime. k_thunk_release(v): rc--, at zero release thunk-tagged
   captured args recursively, push the cell to k_thunk_free,
   thunk_frees++. Creation retains thunk-tagged args (cells
   referencing cells). k_force releases captured args after eval and
   clears argc so the cell's own free can't double-release.

2. Safety classification (static, fixpoint). Per (group, arity,
   position): SAFE iff every arm either wildcards the param, uses it
   only under force (scrutiny sites), returns it bare, or passes it
   only to SAFE positions; anything else — stored into a list/map/
   record/template, captured, passed to an UNSAFE or unknown position
   — demotes to UNSAFE. Assume SAFE, demote to fixpoint; unknown
   callees (closures, builtins that store) are UNSAFE.

3. Epilogue (codegen). For each lazy bind whose uses all target SAFE
   positions, in a fn outside beat clusters: at every return point,
   k_thunk_release_unless(cell, result) — the alias guard frees the
   cell unless the frame's result IS the cell (pointer compare), which
   is the returned-thunk case; that cell leaks upward and is counted.
   Lazy binds in beat-cluster fns or with UNSAFE uses: no epilogue,
   counted as escaped. Tail-position calls return normally here
   (musttail is beat-cluster-only), so the epilogue runs.

Sound by construction: a release only fires when the classification
proves no reference survives the frame and the guard proves the
result register is not the cell. Everything unprovable leaks exactly
as today and shows in the counters.

Counters/goldens fallout, deliberate: counters gain thunk_frees and
thunk_escaped lines; thunk_live_exit becomes allocs - frees - escaped
still-live arithmetic. The .mem vein (4 files), the ch10 counters
book sample, and bench/cost_golden.txt all gain lines — regenerated
in the same PR. New adversarial mem goldens: lazy bind shared across
two deferrable uses (one cell, one free); returned-thunk (alias guard
skips, escaped=1); list-wrapping arm (UNSAFE position, no release);
a fold-driven skip workload pinning frees > 0 (the scoreboard shape
in miniature).

OPEN after this lands: escaped cells (returned thunks, beat-cluster
binds, UNSAFE positions) still live to exit — their story belongs to
defunctionalized thunks / pervasive-lazy, where ownership can ride
the calling convention.

## 2026-07-23 — cell-RC post-landing benchmarks: no shifts, one attribution

Per the standing perf-PR rule (benchmarks + site + dependents on every
perf change): encode 0.88s user (unchanged), lazy scoreboard 0.09s
(unchanged), kq specs green, kanso-json 16/16, vse checks clean on the
new compiler. Site numbers hold as published — no doc changes owed.

FINDING: the lazy scoreboard's 100,000 cells all take the escape path
(thunk_escaped=100000, frees=0) — each rides out of its frame in a
musttail's arguments, exactly the case the classification declines.
The leak-to-exit is unchanged from before cell-RC but now fully
attributed: live_exit equals escaped, nothing unaccounted. Recycling
these is the defunctionalized-thunk work (ownership riding the
calling convention), already OPEN on this log.

## 2026-07-23 — SHIPPED: utf-8 ascii-sweep tier (mined queue item 3, first tier)

Vector ascii sweep in k_utf8_bad (one vmaxvq/movemask test per 16
bytes), scalar only inside a dirty block and always to that block's
end so the sweep never re-probes what it abandoned. bench/large.json
is 3.1% non-ascii scattered through strings — the first cut (scalar
one codepoint per break) thrashed and moved nothing; block-granular
fixed the thrash. Profile: k_utf8_bad drops below the noise floor;
encode wall time unchanged on a loaded box (it was ~3% of the
profile). The full keiser-lemire nibble-lookup tier stays queued
behind a workload that needs it — no point carrying its tables for
documents this ascii.

FINDING while reading the validator: it is lenient (accepts
overlongs, surrogates, >U+10FFFF — only checks continuation masks),
while the interp presumably validates strictly through Rust's
machinery. A latent engine divergence no golden currently reaches:
an adversarial differential case (overlong "\xc0\xaf", surrogate
"\xed\xa0\x80") belongs in the corpus before anything user-facing
depends on the difference. OPEN.

## 2026-07-23 — utf-8 strictness convergence (queue item 3, second cut)

Clay's ruling: no gating vector work behind hypothetical workloads —
implement unless it actively regresses. On the way to the full
keiser-lemire tier, the scalar tier is now SPEC-STRICT (overlong,
surrogate, >U+10FFFF rejected via per-lead continuation windows),
closing the lenient-native/strict-interp divergence logged earlier.
Verified: standalone harness extracting the real validator text —
every 1..3-byte sequence at block offsets {0,13,15} plus 20M sampled
4-byte cases = 70.5M checks, 0 mismatches vs an independent
spec-direct reference (which the harness itself debugged: its first
draft accepted bare-continuation leads; the validator under test was
right). Differential golden examples/utf8_strict.kso pins overlong/
surrogate rejection and U+10FFFF acceptance on both engines. The
nibble-lookup vector tier for dirty blocks is the next cut, same
harness as gate.
## 2026-07-23 — PLAN: subtypes v1 (REPL-testable slice)

Ratified design (memory: kanso-subtypes): `type post_body string` —
space form, colon stays membership-only; one member = nominal
wrapper; ctor-form construction (down explicit), transparent up-flow
(no unwrap form); one new dispatch rung; pointwise specificity with
tie-rejection as compile error (gaveled).

Slice for tonight, REPL-first: parser (single-member header on the
type line; multi-member reserved with a "named typesets next"
diagnostic), TypeDecl.parent in the AST, interp semantics complete —
construction wraps (record-shaped, one hidden slot), Annotated
pattern matching walks the parent chain (nearer declaration wins the
ladder), builtins/operators/render/equality unwrap to the parent
(parent-render default), REPL declares and dispatches. Native and
wasm REJECT subtype declarations with a clear diagnostic in this
slice so no engine silently diverges; corpus goldens untouched until
all engines speak it. Tie-rejection lands with the native dispatch
work, where the reachable-set machinery lives.

## 2026-07-23 — SHIPPED: full keiser-lemire utf-8 tier (queue item 3 complete)

Per Clay's ruling (implement unless it actively regresses): the full
nibble-lookup algorithm on NEON — three vqtbl1q classifications per
block, vqsubq saturating pins for 3/4-byte continuation runs,
all-ascii blocks (both current and prev) skip classification, one
trailing zero block terminates any truncated sequence so the
incomplete-at-end case needs no special path. x86 keeps the
ascii-sweep + strict-scalar tier (the SSE port of the lookups is a
follow-up; CI's x86 lane exercises that path). The 70.5M-case
harness (extracts the real function text) passed at zero mismatches
on the FIRST run of the vector path — the nibble tables survived
reconstruction intact. Encode bench 0.68s user on a quiet sitting vs
the 0.66 pre-change floor: flat, as predicted for a 97%-ascii
document; the win waits on multibyte-heavy workloads and the
correctness is unconditional.

Same sitting, the boards refreshed everywhere (Clay's
publish-immediately policy): compiler page four-row race — kanso
0.89/0.92 vs serde 0.98/1.01 ms/decode (the eisel-lemire lead now on
the primary board), naive rust 1.13, go 2.03; kq README — path
3.6/16.0ms (1.59x/1.76x), pretty 7.8/56.9ms (1.88x/2.02x), 99/100
runs to kq.

## 2026-07-23 — MEASURED AND DECLINED: eytzinger map lookup (queue item 4)

Built the full thing (lazy BFS-order key index + slot map, rewind-safe
through the cache registry with an independent-death check for the
index) and A/B'd it: 30-key JSON maps, 5M lookups — eytzinger 0.09s vs
binary search 0.08s (the index costs more than it saves when the whole
sorted view fits in a few cache lines); 10k-key map, 2M lookups — dead
tie at 0.13s (shared-prefix string keys make memcmp the cost; layout
can't help a comparison that has to walk bytes). The paper's wins live
on huge arrays of word-sized keys. Reverted; the page records the
negative result so the idea stays declined. Next: dragonbox.

## 2026-07-23 — SHIPPED: ryū rendering (queue item 1); INCIDENT: the merge that ate eisel-lemire

The incident first, because the guard matters more than the feature:
main had LOST eisel-lemire — the #171 conflict resolution
(checkout --ours on runtime.c) picked a lineage that predated #165,
and nothing caught it: no golden pinned the fast path's existence,
and the decode-ratio CI job is non-gating by design. The published
0.89ms board was measured WITH el; main would not have reproduced
it. Restored from the #165 squash commit, and the counters now
carry el_parses (318,450 on the cost-golden workload, CI-diffed) so
a merge can never silently drop it again. Lesson for the protocol:
checkout --ours/--theirs on runtime.c is banned — resolve function
by function, and every perf kernel lands with a presence counter.

The feature: ryū d2s (adams, PLDI 2018) — python-generated 125-bit
pow5/inv tables (exact, like the el table), the half-ulp interval
walk with trailing-zero tracking, and a %g-mimic format layer
(fixed vs exponent at max(15, k)). Fuzz: 50M doubles against the
shipped probe — 0 failures, 495 legal shortenings (subnormals,
where the probe's 15-digit floor overshoots true-shortest; the
shorter form is now canon). The interpreter's render_float was
quietly divergent on |x| outside the %g fixed range (rust Display
never uses exponent form — 1e20 and subnormals differed across
engines, latent because no corpus float reached there); it now
formats rust's shortest digits through the same rules, verified
byte-identical on the exponent-range family. dtoa/quorem/multadd:
zero samples in the encode profile. kq jq-parity green. Profile
floor now memmove/k_b_append/dispatch — TRMC and SpecConstr
territory. Noted in passing: kanso has no exponent float literals
(1.0e20 is a name error) — parked as a possible gavel.

## 2026-07-23 — post-ryū sitting (load 47, interleaved)

encodebench 0.71-0.76s user under heavy load (quiet floor pre-ryū was
0.66; the ryū win is profile-proven — dtoa/quorem/multadd at zero
samples — and the quiet-absolute refresh stays owed). kq boards,
same sitting: path 5.1/20.6ms (1.54x/1.64x), pretty 8.7/62.8ms
(2.00x/4.76x — jq at 299ms on the big file; a loaded box hurts the
slower tool more). kq README table updated to this sitting with idle
floors footnoted. All five kernel presence counters now CI-pinned in
the encode cost golden (#175).
## 2026-07-23 — PLAN: ryū/dragonbox rendering (queue item 1, the last big kernel)

Staged so the gate exists before the core (the EL lesson):

1. HARNESS FIRST: a differential fuzzer extracting the CURRENT
   renderer (probe via el + %g) and the candidate, requiring
   byte-identical strings over: every float in the golden corpus, 50M
   random bit-pattern doubles, and the edge families (powers of ten,
   halfway cases, subnormal-adjacent, 1e15 boundary at the %.1f
   integral fast path).
2. FORMAT LAYER: shortest digits (k, digits, exp10) format to match
   %g with precision max(15, k) exactly — fixed vs exponent at
   X < -4 or X >= max(15, k), trailing-zero trim, two-digit e+XX.
   This layer is testable against the probe independently of the
   digit core.
3. DIGIT CORE: ryū d2s (adams, PLDI 2018) — python-generated
   inverse/pow5 tables exactly like the EL table, the a/b/c halfway
   computation, shortest-digit trim. Dragonbox stays the follow-up if
   ryū's win leaves dtoa visible in the profile.

Acceptance: byte identity with the shipped renderer across the whole
harness; the probe path retires only when the fuzzer is silent.

## 2026-07-23 — SHIPPED: zero-copy finish + length twin; TRMC re-read as already-won

TRMC's regime is cons-cell construction; kanso's flat arrays with
frontier push already sit at its endpoint, so the queue slot spent
itself on the profile's actual names instead. (1) k_utf8_finish: a
builder-owned buffer becomes the string in place — NUL into spare
capacity, frontier burned (used = cap) so any surviving bytes value
grows away on its next append rather than writing under the string.
Encode golden: utf8_zerocopy=400, allocs -400, alloc_bytes -75.5MB
(the whole-output copy, deleted), arena_blocks 2272→2205. (2)
k_b_length_fast IR twin: the list case is a header load inlined into
every fold; map/string fall through to the call. Encode 0.66-0.70s
user on a loaded box — at the pre-ryū quiet floor despite the
weather. Next on the floor: the k_b_append inline fast path (211
samples), then SpecConstr for d_encode_onto dispatch (120).

## 2026-07-23 — SHIPPED: append byte-twin + forwarder elision (+ the golden that caught the bug)

Three cuts against the post-ryū floor: (1) k_b_append_byte — the
single-byte frontier claim fully inline in IR, arena bump exported
(k_arena/k_arena_left/k_stats_on de-static'd), stats-gated so the
pinned counters stay exact; (2) k_b_length_fast learns BYTES (same
header layout as lists — fold-over-bytes loops stop calling out);
(3) forwarder elision — std wrappers that only forward to a builtin
(text/append and family) stop costing a dispatched call; call sites
reach the builtin and its twins directly.

The elision's first draft renamed the callee BEFORE user-group
dispatch and a renamed call bound to the wrong dispatcher variant —
d_text/find2_below_5 carries a byte-specialized signature (raw i64
lanes), so boxed args reinterpreted as garbage. ch08's `using`
book golden caught it (native err where the interp ran clean); the
fix scopes the rename inside the builtin emission branch where it
can never leak into group dispatch. Goldens-for-everything earned
its keep the same day it was written into CLAUDE.md.

Encode: 0.66 → 0.49-0.51s user (campaign 3.46 → 0.50, 6.9x). Encode
cost golden exact; kq green; 12/12.

## 2026-07-24 — tail-forwarder elision closes the campaign's last named line

Tail-position calls to std forwarders (the musttail dispatcher route
the value-position elision could not touch) now emit builtin-plus-ret
— wrappers never recurse, so nothing is owed to stack safety. Encode
0.50 → 0.39s user (campaign 3.46 → 0.39, 8.9x); the profile is flat
at ~25 samples/line — d_encode_onto at 24 means SpecConstr is no
longer profile-motivated for encode and defers. kq inherits through
pretty: big-file 56.9 → 50.4ms (2.28x over jq). The surface checklist
lands in CLAUDE.md's definition of done.

## 2026-07-24 — PLAN: subtypes on native and wasm (+ tie-rejection)

The interp slice (#168, #173) is the oracle; the design is ratified
(memory: kanso-subtypes). Native shape:

1. Representation: K_SUB tag (15), KSub { type_id, KValue inner } —
   arena-allocated like records; carry/copy/render/equality arms in
   runtime.c mirror the interp (render/equality/compare/builtins
   unwrap via a k_sub_base walk; construction validates the parent).
2. Construction: a type decl with a parent compiles its ctor to
   k_sub(type_id, inner) after a parent-type check.
3. Dispatch: annotated params accept a Sub whose chain reaches the
   annotation — dispatchers gain a chain-walk compare (helper
   k_sub_matches(v, want_id_or_tag, depth_out)); specificity uses
   depth exactly as the interp scores it. The upcast strips via a
   k_sub_upcast walk with the widening error.
4. Wasm mirrors through its rt (same enum, same helpers).
5. Tie-rejection (gaveled): check.rs — for each call site, the
   reachable per-position sets (inference) against each arm pair's
   pointwise order; two incomparable maximal arms = compile error
   naming both arms and the disambiguating signature. Lands with the
   native dispatch work since it leans on the same reachable sets.
6. Differential goldens once all engines speak it: construction,
   chain dispatch, upcast, transparency through builtins/operators,
   the tie-rejection error text.

## 2026-07-24 — SHIPPED: subtypes on native (stage 2)

Constructor (k_sub_ctor with parent validation), chain-aware dispatch
checks gated to subtype-declaring programs (subtype-free programs
keep the exact checks — zero cost), the dispatcher arm sort (deepest
annotation first; the interp's scores as an ordering, with the
gaveled tie-rejection to outlaw the incomparable cases at check
time), the upcast walk with the widening error, and transparency
guards in the C arithmetic/comparison entries. Engines byte-identical
on the full scenario — ladder, construction, upcast, arithmetic,
equality — AND on both error texts. examples/subtypes.kso is the
differential golden. Wasm stays gated with a clear diagnostic; its
mirror plus the tie-rejection checker are stage 3. Encode holds at
0.56-0.59 under load 42 (0.39 at load 20 — weather, and the CI
benchmark lane arbitrates).

## 2026-07-24 — SHIPPED: subtypes on wasm (stage 3a) — three engines agree

rt_check_type walks the Sub chain (subtype-free programs never hold a
Sub, so nothing relaxes), rt_mksub validates the parent via the
mirrored parent table, rt_upcast strips with the widening error,
rt_binop unwraps to the base, and the backend gains the ctor route,
the upcast emission, and the same deepest-first arm sort as native.
Browser differential: 27 passed, 0 failed — up from main's 26/0
baseline with subtypes.kso added to the corpus. Two bugs the
differential caught on the way: a tid-scheme confusion (ctor ids are
raw TYPES indices; check codes are 100+index — the panic poisoned the
shared instance's RefCells and cascaded, which is worth remembering
when reading wasm failure lists), and the missing binop unwrap.
Remaining: stage 3b tie-rejection; the standing follow-up to wire the
browser differential into CI just earned its priority.

## 2026-07-24 — SHIPPED: tie-rejection (stage 3b) — the subtype thread closes

The gaveled rule, enforced at definition level per Clay's wording
("if multiple function definitions match, that's a compile error"):
for each group, arm pairs are compared pointwise — chain relation for
annotated positions, rank elsewhere; a pair that overlaps in every
position while each arm is strictly more specific somewhere is
rejected at the later arm with the fix in the message ("write the arm
that is most specific in every position"). Comparable arms (g x:num /
g x:int) stay legal — the ladder orders them. Zero cost for programs
without subtypes (the parents map gates the whole pass). Golden:
tests/golden/errors/subtype_tie. The ratified subtype design is now
COMPLETE across declaration, construction, dispatch, upcast,
transparency, three engines, and the static safety rule.

## 2026-07-24 — SHIPPED: named typesets — the declaration family is whole

`type num float64 int` (two or more members, alphabetized like every
enumeration) declares annotation-only vocabulary: no construction, no
bare mention, no dispatch identity — an annotated param matches any
member. The ladder holds everywhere: literal > concrete annotation
(nearer subtype first) > typeset > generic — encoded as score depth
in the interp (TYPESET_DEPTH below every chain) and as the arm-sort
tier in both compiled backends (which also fixed the sort's blind
spot: depth-only ordering had no rung for generics). Interp matching
reads a registry mirrored at Interp::new; native emits a static OR of
member checks; wasm ORs rt_check_type per member (i32.or joined the
encoder). Differential corpus grows examples/typeset_named.kso: 28
passed, 0 failed. One `type` declaration form now covers records
(fields), markers (nothing), subtypes (one member), and named
typesets (several) — the mushroom test's four-for-one, closed.

## 2026-07-24 — PLAN: build blocks v1 (the staged slice)

Per design/build-blocks.md (the ratified record). Stage order proven
by the subtype arc: (1) parser — `x = build` header with an indented
body; inside it, a statement `set target field value` lifts from
application form (the name stays free elsewhere); the body's last
expression freezes as the result. (2) check — set only inside build;
targets trace to block-born bindings; the birthday theorem's premise
enforced statically. (3) interp oracle — block-born records wrap in
identity cells; set writes through; freeze is the boundary; render
and equality carry a visited set so cycles print `<cycle>` and
compare without divergence, byte-identical across engines by
construction. (4) native — in-place field store into the arena;
cohort-holding loops beat-ineligible v1 (the conservative posture the
byte builder took); the carry's deep copy gains an identity map so a
cyclic cohort cannot recurse it forever. (5) wasm mirrors through the
shared Value. (6) differential goldens: wiring a two-node cycle,
reading through it, the set-outside-build and non-block-born errors,
and the cycle render marker.

## 2026-07-23 — build blocks v1 ship

All six stages landed on one branch. Surface: `x = build` (or bare tail
`build`) opens the block; `set target field value` lifts from application
form inside it; the last expression freezes as the result. Checker: the
block-born rule — a set target must trace to a *direct constructor
application* born in the same block (a call that merely returns a record
may hand back something older, so it does not qualify; conservative v1).
`set` outside build gets its own diagnostic off the unknown-name path.
Interp: `Record.fields` became `Rc<RefCell<Vec<Value>>>` — the identity
cell; set writes through it; render carries a path-scoped visited set and
prints `<cycle>` at re-entry (shared acyclic subtrees still render fully);
equality short-circuits on cell identity. Native: `k_set_field` writes in
place by field name (mirror of `k_b_field`); `k_render` carries the same
path guard; `k_eq` gets the pointer short-circuit; fns holding a build are
beat-ineligible v1 (the carry deep-copy has no identity map yet, so cycles
must never reach it — `beat_iters=0` is pinned in the new mem golden).
Wasm: `rt_setfield` by name-literal, shared-Value RefCell threaded through
`rt_mkrec`/`rt_field`/`rt_keyed_field`/`rt_check_rec`. Goldens:
examples/build_blocks.kso (two-node cycle + self-loop, all three engines
byte-identical, browser corpus 29/0), errors build_set_outside +
build_not_block_born, mem build_cycle. Deferred, tracked: identity-mapped
deep copy (unlocks beat for cohorts), equality on two distinct cyclic
graphs (recurses in every engine today), cohort arena freeing.

## 2026-07-23 — carry identity map + set-on-failure engine agreement

The carry deep-copy learns to walk cycles. Two generation-stamped
open-addressing pointer tables (reused across carries, so the hot beat path
never mallocs per iteration): a size-pass `seen` set that counts each unique
node once, and a copy-pass `old -> copy` map consulted before recursing so a
back-edge resolves to the already-made copy instead of recursing forever.
The copy inserts the new node into the map *before* descending into its
fields — the textbook cycle-safe order. New presence counter `carry_dedup`
(a copy resolved through the map), pinned across every vein: both cost
goldens, all mem files, the ch10 book counter sample. It reads 0 on the json
decode and encode paths — the identity map is zero-overhead where nothing
shares or cycles, confirmed by every other counter staying byte-identical.

With cycles walkable, the v1 beat-ineligibility for build-holding functions
is removed: a beat that carries an arena-allocated cycle now copies it
correctly rather than being forbidden. (The v1 guard was also incomplete —
it keyed on *holding* a build, but the real risk is *carrying* a cycle, which
a non-build function can do; the cycle-safe copy is the complete fix.)

Found and fixed a differential-law violation in `set` on a failure target:
a constructor given a failure argument propagates it (`node 1 none` -> none,
the documented none-propagation behavior), so a block-born `set` target can
evaluate to none. Native's k_set_field returned early on a failure target
(no-op); interp errored; wasm died. Interp's own field-*read* already
propagates failures, so interp was internally inconsistent. Fixed all three
to propagate (skip the write): native already did, interp `continue`s on a
failure target, wasm rt_setfield returns none. Pinned by
examples/build_set_failure.kso, byte-identical on all three engines (30
browser cases, 0 fail).

Open design question surfaced to Clay, NOT decided here: should `node 1 none`
propagate none at all? For an `any`-typed field, none is a legitimate value,
and propagation (treating none like an err/exception in a constructor arg)
may be wrong — it makes a block-born `set` silently no-op. The failure-model
gavel (none = value, err = exception) suggests only err should propagate
through a constructor, not none. Left for dialog.

## 2026-07-23 — cyclic-graph equality (bisimulation)

Comparing two *distinct* cyclic graphs hung native and stack-overflowed
interp: the cell-identity shortcut only caught the *same* cell, and two
separately-built equal rings are different cells. Fixed with bisimulation —
assume a record-cell pair equal on first encounter, and a re-encounter of
that pair is the coinductive base case (true) rather than recursing forever.
The assumption set is global to one comparison, so a pair contradicted
anywhere still returns false; records are the only settable cell, so every
cycle passes through one and guarding the record case breaks them all.
Native carries a generation-stamped open-addressing pair-set (reused across
comparisons, so k_eq never mallocs on the acyclic path); interp threads a
HashSet of cell-pointer pairs; wasm inherits the fix through the shared
eval_binop. Pinned by examples/build_cyclic_eq.kso — equal twins true,
field-divergent rings false, self true — byte-identical on all three engines
(31 browser cases). No counters touched.

Build-blocks deferred queue now: cohort arena freeing (the birthday-theorem
memory payoff) is the last big item; identity-mapped copy and cyclic
equality are both done.

## 2026-07-23 — dot field access in expression position (spacing bug)

`{u.age}` worked in interpolation but `a = u.age` in a binding was rejected
with "canonical form requires exactly one space here". The tokenizer already
distinguishes tight field access (Tok::Dot) from the spaced pipe (Tok::Pipe),
but required_gap had no Dot case, so it defaulted to demanding one space —
exactly backwards for a tight dot. Added `(_, Tok::Dot) | (Tok::Dot, _) => 0`.
Field access now works in any expression position, chained (s.head.x) too,
native and interp byte-identical. Pinned by examples/dot_field_access.kso.
The bug survived because the only field-access golden lived inside an
interpolation, which skips line spacing validation.

Tracked separately (NOT this PR): the browser backend still rejects
Expr::Field at codegen (clean fallback, differential-law-legal). Wiring it
via the existing rt_keyed_field is a small happy-path change, but matching
the interpreter's field-access error paths (failure propagation, the
"`{ty}` has no field `{name}`" message) byte-for-byte needs its own
adversarial goldens — a focused follow-up.

## 2026-07-23 — field-access error messages: native matches the oracle

Dot field access (#60) landed on native and interp but their error messages
never agreed, and no golden caught it: on a non-record native said "`.`
reads a field of a record" while interp appended ", not {value}"; on a
missing field native said "no such field" while interp said "`{ty}` has no
field `{name}`". k_b_field used terse k_die strings; native's own
k_keyed_field already rendered the oracle's wording, so the dot path was just
inconsistent with both interp and the keyed path. Fixed k_b_field to render
the value and name the type/field exactly as interp does. Pinned by two
runtime goldens (field_non_record, field_missing) that run both engines and
assert byte-identical stderr. Browser backend still rejects Expr::Field
(clean fallback), so these are wasm fallbacks — the tracked wasm-field-access
follow-up will add the wasm arm and fold these into its differential.

## 2026-07-23 — missing_index book golden regen (follow-on to field-error fix)

The field-access message change moved docs/book/samples/appa/missing_index.out
(`xs.at 9` reads field `at` on a list, now "... not [1 2 3]"). Regenerated it;
book_check green. Surfaced but NOT fixed here (tracked book-panel-sync work,
"book HTML stale pre-keystone"): appa.html's output panel for this sample
still shows the pre-module-system "error[endpoint]: unhandled none reached
main", and the sample itself uses tight `xs.at 9` (field access) in an
appendix about the endpoint family — it wants `xs[9]` or a re-import of `at`.
The .out on main already carried the field-error interpretation, so the HTML
was already unsynced; this PR only updates the message text. Note: book_check
runs in CI separately from cargo test — run `sh scripts/book_check.sh` locally
for any diagnostic-message change.

## 2026-07-24 — dot field access lands in the browser backend

Wired Expr::Field into the wasm backend via a new rt_field_by_name that
mirrors the interpreter's Expr::Field exactly: a failure propagates
untouched, a non-record dies "`.` reads a field of a record, not {rendered}",
a missing field dies "`{ty}` has no field `{name}`" — the same wording native
was aligned to in the field-error PR, so all three engines now agree on the
error paths too. The three field-access goldens (dot_field_access,
field_non_record, field_missing) moved from wasm fallback to real
three-engine passes: browser differential 34 passed / 5 fallback / 0 failed
(was 31 / 8). No native or interp change; goldens already existed.

## 2026-07-24 — type syntax gaveled (design/type-syntax.md)

A design session settled the type-syntax cluster. Recorded in
design/type-syntax.md; summarized here for the chronology.

Ruled: `[]T` slices, Go's prefix form (Clay: "i hate [string], i like the
go style"). Postfix `Name[args]` for type application, so `map[string int]`
is an ordinary application rather than Go's map-only `map[K]V` — key first,
value second, uniform with `set[string]` and `pair[k v]`. A `[` tight
against an identifier applies; a spaced or leading `[` is a slice, the same
tight-versus-spaced rule the lexer already uses for field access vs pipe.
No `<k>` binder anywhere: parameter order comes from the order variables
first appear in a type's fields, and writing the binder repeats what the
fields say. No type parameters on functions at all, and no annotation that
is not load-bearing — `fn foo u:string` is an error when the body already
pins u to string. Dispatch discriminators stay legal because choosing an
arm is not a derivation; so does any position where subtypes/typesets leave
inference genuinely undecided. "Typeset" is the name (not union); `|` is
not in the language. No positional products at any arity: no tuple, no
positional pair, because access is by name everywhere else. The stdlib
still ships a two-field generic record called `pair` (fields first/second)
for zip/to_h, where ordinal names are honest because zip is domain-blind.

Open, and the one thing blocking a full seal: whether anonymous typesets
exist. Parens group them unambiguously — `map[(string user) string]`,
`x:(string user)` — and Clay called that spelling exactly right, then
raised whether forcing a name is the better practice, since a typeset whose
best name is `string_or_user` is one that probably should not exist.
Requiring names also leaves the type grammar with three forms and no parens
rule. Not decided.

Deferred: constraints (bounded polymorphism, for map keys); the error-
locality cost of full inference, mitigated by the language server.

Nothing implemented — kanso has no user-defined parameterized types today
and field types are bare names. This reserves the shape.

## 2026-07-24 — type syntax: the last two rulings

Two questions left open by the type-syntax gavel are now decided, and
design/type-syntax.md is fully settled.

**Typesets are always named.** No inline form. An anonymous typeset would
need grouping, since a bare space already separates type arguments, and
grouping drags parens into type position plus a one-member rule plus an
ordering rule inside the group. None of that now exists, and the type
grammar stays at exactly three forms: a name, `[]T`, `Name[args]`. The
argument that carried it: a typeset whose best name is `string_or_user` is
usually one that should not exist, so requiring the name surfaces the
arbitrary unions rather than burdening the good ones. Clay noted this is
reversible if a real one-off case shows up.

Also recorded: the earlier framing of "anonymous for one-offs, named when
it recurs" as don't-DRY-until-two was wrong, and the doc no longer implies
it. A name is a label, not an abstraction — nothing speculative is being
built — so YAGNI is neutral here and name-what-things-are decides it.

**`map{string int}` declined.** Considered for semantic resonance with the
`{ "a":1 }` map literal. It borrows the braces without the colon that makes
them read as a mapping, so the resemblance is visual; the honestly resonant
form is `{string:int}`, whose colon collides with the annotation colon
(`m:{string:int}`). It also re-specializes `map` right after `map[K V]`
made it an ordinary parameterized type, and the resonance principle applied
consistently would drag slices back to `[string]`, which was already
rejected. Declined alternatives now live in the doc so they stay declined.

## 2026-07-24 — GAVEL: none is a value, err is the failure

Ruled: `none` is not a failure. `err` is. The implementation must be brought
in line with the already-ratified failure model, which it currently
contradicts.

Both engines classify none as a failure in one predicate each —
`k_not_failure(v) { return v.tag != K_ERR && v.tag != K_NONE; }` and
`is_failure(v) { matches!(v, ErrV(_) | NoneV) }` — and `k_rec`'s first act
is to return any failing argument instead of building. So construction eats
none.

The demonstration, which is the sharpest form of the bug:

    type maybe_job none string
    type user
      job:maybe_job
      name:string

    user "cook" "mai"   -> user "cook" "mai"    builds
    user none "ken"     -> <none>               eaten

The field's typeset explicitly admits none, the checker accepts the
declaration, and the constructor then refuses to store the value the
declaration permits. none is a zero-width type and belongs in a typeset like
any other member.

Downstream of the same misclassification: json's `json_null` marker exists
only because none is eaten as data, and it carries a dummy `bool` field only
because kanso has no zero-field types. Both inelegances trace here.

What the fix is NOT: deleting auto-propagation. Propagation is load-bearing
and correct for bails — it is what removed every "did the last step fail?"
line from the json parser. The fix is removing none from the failure set so
propagation applies to err alone.

Sequencing, which is the real design work: the ratified model also says a
function receiving a none must state its disposition (an arm, or resolve
before the call), no arm being a compile error, and none-transparency being
an explicit `none -> none` forwarding arm. That exhaustiveness check has to
land with the demotion. Without it, none stops propagating and instead flows
silently into code that never considered it — the anti-Ruby-nil outcome the
model was written to prevent. Today's corpus leans on propagation heavily
(`xs[9]` feeding arithmetic, the endpoint goldens, the mem corpus), so this
is a campaign, not a predicate edit.

Related gap found in the same probe, and it wants the same machinery:
constructor arguments are not type-checked against field types at all.
`node "hello" 5` against `id:int` passes `check` and constructs. Real
checking at the value/field boundary is the prerequisite for both.

Next step is a design doc for the campaign, not code.

## 2026-07-24 — GAVEL: where none may live (static slots, not dynamic ones)

Ruled, refining the none-is-a-value gavel above: arrays and maps may not
hold `none` as an element or value — nor a subtype of none, nor a typeset
admitting it. Records may.

The discriminator is whether a slot's existence is static or dynamic. A
record field is statically known to exist, so a none in it has exactly one
reading: the value is nothing. A map key or an array index is arbitrary, so
absence is always live and a none there has two competing readings.

Both halves have their own reason, which is why neither is a concession.
Records must allow none, or the result of a lookup that found nothing could
not be stored without inventing a wrapper type per optional field.
Collections must forbid it, or `xs[i] -> none` stops meaning "not found"
and every lenient lookup in the language becomes a lie.

The rule composes: a record living in a collection may hold none in its
fields to any depth, because the collection's elements are records and a
lookup on it never yields none. Only a *direct* element or map value is
banned.

Consequences.
- Storing an absence inside a collection uses a zero-field marker type
  (`type vacant`, then `type job_slot string vacant`), not none. This gives
  zero-field types a real job and is the general form of what json_null
  already does by hand.
- json_null therefore stays a non-none type. Deriving it from none is dead
  under this rule. The dummy `bool` it carries today gets fixed by zero-field
  types, not by parenting it to none.
- `[1 none 3]` becomes illegal. It works on both engines today, so this is
  the breaking change, and it is the exact construct that makes lookups lie.

Nesting cannot rescue the ambiguity the way it does elsewhere: none is flat,
so `none | none` collapses, and there is no `Option<Option<T>>` to reach for.
That is why the rule is a prohibition rather than a wrapper.

Open, not ruled: what `any` means with respect to none. If `any` admits
none then `[]any` is illegal, which is a surprise worth deciding
deliberately rather than discovering — while `peer:any` holding none in a
record stays desirable.

Evidence gathered in the same thread, all unpinned by goldens, all rooted in
none's confused status:
- records eat none on both engines (`node 1 none` -> `<none>`), even when the
  field's declared typeset admits none (`type maybe_job none string`).
- map literals holding none diverge: native stores (`{ "a":<none> "b":2 }`),
  interp eats (`<none>`). Lists store on both.
- a subtype of none diverges: the checker accepts `type missing none`, native
  fails with "unknown type `none`", interp silently erases the subtype and
  dispatches to the plain none arm.
- constructor arguments are not checked against field types at all:
  `node "hello" 5` against `id:int` passes check and constructs.

### Addendum — the prohibition is about the channel, not about none

The rule above generalizes: whatever type a lookup uses to signal "no
answer" must be unrepresentable as an answer in the thing being looked up.
none is the current instance, not the principle.

The gavel's "nor a subtype of none" clause already carries this. A richer
absence signal declared as a subtype — `type out_of_bounds none` for arrays,
`type missing_key none` for maps — is banned from collections automatically,
and up-flows so that code asking only "did it find anything?" keeps working
while code that cares dispatches on the precise reason. Whether lookups
actually return such refined types is available, not ruled.

## 2026-07-24 — GAVEL: `any` excludes the absence channel

Ruled, closing the question the static-slot gavel left open: `any` is legal
everywhere, including as a collection element type. It excludes none, its
subtypes, and any refined lookup signal derived from it.

So `any` is the set of values that can be stored, and the absence channel is
disjoint from it by definition. `[]any` and `map[string any]` are therefore
safe by construction, and the explicit collection prohibition remains only
for the case someone writes an element type that names none directly
(`[]maybe_job` where `type maybe_job none string`).

Consequence: a record field typed `any` no longer holds none. A field that
wants anything-or-nothing says so — `peer:any none`, or a named typeset.
`any` then means the same thing in a field and in an element position, and
optionality is always written rather than inherited.

This agrees with the json conclusion reached from the other direction: a
json array holds `any`, `any` excludes the absence channel, so json null
stays a non-none marker type.

Also settled in the same exchange, on refined lookup signals: if
`out_of_bounds` and `missing_key` are introduced, they are subtypes of none.
The deciding argument is migration — as independent types they would break
every existing `none` arm in the stdlib and in user code at once, while as
subtypes they up-flow, so existing arms keep matching and code wanting the
reason opts into dispatching on it. It also lines up conceptually: the
lenient form `xs[i]` is the one where absence is a value rather than a bail
(the strict form `xs[i]!` already gives the err), so its signal is a kind of
nothing, which is what a subtype of none is. Introducing them at all remains
optional and deferred; both gavels stand with bare none. Doing so would make
fixing the subtype-of-none divergence load-bearing rather than incidental.

## 2026-07-24 — GAVEL: a function that accepts an err must return err

Resurrecting the rationale behind always-bubbling err: keeping err out of
control flow. Ruled: any function with an arm matching err must have err in
its return. An err cannot be absorbed into an ordinary value.

This completes a symmetry with the none rulings above, and the symmetry is
the point:

  none MUST be handled  — exhaustiveness forces an arm.
  err  CANNOT be handled — accepting one forces returning one.

Opposite obligations, and that difference is what separates a value from a
bail. It also explains why err auto-propagates while none must not: err is
unhandleable by design, none is handleable by requirement.

The rule is "never rescue your own exceptions" promoted from a style
guideline to a type rule. What it bans:

    fn read_port (err _)
      8080                 # illegal: swallows a bail into a default

and the failure model already justifies the ban. A case with a sensible
default was never a bail; a deterministic wrong-input case should have been
a value (none, or a Malformed-style record) where it was created, rather
than papered over at a call site downstream.

Functions with no err arm are unaffected. err rides through them
automatically and their return gains `| err`. The rule bites only where
someone explicitly matches err.

The exception is package boundaries, and it is structural rather than
syntactic — which is what keeps it from being an escape hatch. A boundary is
a fact the compiler knows, not a keyword that can be sprinkled, so "I will
just catch it here" is unavailable inside one's own code by construction.
This is precisely what try/catch gets wrong: an arbitrary catch site makes
the discipline advisory. Two sites, both structural:

- the opacity boundary — a call into a hako, whose internals are invisible,
  so its bail can only arrive by propagating out.
- the supervisory boundary — main/serve/supervise, applying a policy.

Neither site handles an err as an err. Both REIFY it into an inert Failure
record, after which it is a value and handling it is ordinary code — so the
rule is never violated, and the only way to act on a failure is to convert
it to a value first. That is the whole discipline in one sentence.

Dependency: enforcing the boundary exception requires knowing which calls
cross a package boundary, so it lands with hako. Until then the rule is
enforceable for ordinary functions and the reification form is unbuilt.

## 2026-07-24 — browser backend: operator arms and function values

Two of the three remaining wasm fallbacks close. Browser differential goes
from 34 passed / 5 fallback to 37 passed / 2 fallback, 0 failed.

Operator arms mirror the native strategy rather than inventing one: a record
on the left dispatches to the operator's user group, everything else takes
the builtin path, so numbers never reach user arms. Native decides this
statically where inference proves the left operand can be a record; wasm
emits the branch whenever a user group exists and tests the tag at runtime
through a new rt_is_rec. The playground is not the performance engine, so
the always-branch costs nothing that matters and keeps the emitter simple.

Function values needed one missing case in emit_app. Bare function names
already became closures through fn_wrapper/RT_MKCLOSURE, but a call whose
head is a zero-arity constant holding one (`twice = double`, then `twice 7`)
matched no dispatcher for its own arity and fell through to the error. It
now evaluates the constant and applies the result through RT_CALL.

The two remaining fallbacks are both `join`, which waits on the io/parallel
ordering gavel rather than on emitter work.

## 2026-07-24 — cohort arena freeing: measured, deferred (not a near-term win)

Before designing the arena-per-block story, measured whether the workload it
targets actually leaks. It does not.

Two hundred thousand two-node cycles built and discarded in a loop:

    n=20000    allocs=80003   alloc_bytes=2560080   arena_blocks=1  beat_iters=20000
    n=200000   allocs=800003  alloc_bytes=25600080  arena_blocks=1  beat_iters=200000

arena_blocks stays at 1 across a tenfold increase. alloc_bytes is cumulative
and so grows with the work done; peak footprint does not. The beat rewind
already reclaims each discarded cohort at its iteration boundary — one
rewind per iteration, on the record as beat_iters. Discard-in-a-loop is the
case cohort freeing was meant to fix, and a shipped mechanism already fixes
it.

The retaining variant grows as it should, because the graphs stay live:

    n=20000    arena_blocks=4    n=200000   arena_blocks=32   beat_iters=0

Nothing to free there either; those cohorts are reachable.

The case that remains — a cohort held for a while, then dropped while the
program keeps allocating — needs the drop to be *detected*, and detection is
reference counting. The native runtime has no value-level counting at all:
k_alloc is a bump pointer into the beat arena, reclaimed by rewind, and the
only rc field in the runtime belongs to KThunk. So cohort freeing is not a
standalone optimization that can be built now. It is a component of the
counted world, and it becomes buildable when value-level RC does.

The design is untouched by this. The birthday theorem and cohort counting
are what keep RC *complete* once RC exists — a cycle counts as one unit, so
no collector is needed. What is declined is treating cohort freeing as a
near-term performance win: its headline benefit is already delivered, and
its remaining benefit cannot be built ahead of the machinery it rests on.

## 2026-07-24 — arena block size: measured, left at 1 MiB

Asked whether shrinking the standard arena block would lower peak footprint,
on the theory that arena_blocks=1 means peak is one block regardless of how
little is live. Measured; the theory was wrong.

    block     discard_rss   json_rss   json_blocks   json_best
    1 MiB     999424        6537216    5             0.14s
    64 KiB    1015808       7061504    73            0.14s

Smaller blocks cost memory and buy nothing. Time is unchanged, json's peak
rises about eight percent, and the discard loop does not move at all.

The reason the discard loop does not move is the useful part. A program
whose whole body is `print "hi"` has a peak RSS of 999424 — byte-identical
to the two-hundred-thousand-cycle loop. The floor is the process itself,
binary and runtime and libc, and the arena never rises above it because the
beat rewind keeps the live set to a handful of nodes. There was no arena
contribution to shrink. json gets worse for the ordinary reason: seventy-
three malloc'd blocks carry more per-block overhead and less contiguity
than five.

A methodology note worth keeping, because it invalidated a first attempt at
this table. runtime.c reaches the compiled program through include_str! in
main.rs, so it is baked into the kanso binary at *cargo* build time. Editing
runtime.c and then running `kanso build` measures the old runtime. The tell
was json_blocks staying at 5 while the block size supposedly fell to 16 KiB,
which is arithmetically impossible. Any runtime experiment needs
`cargo build --release` between the edit and the measurement.

## 2026-07-24 — prior art for cohort counting (correcting an overclaim)

Searched the literature after asserting from memory that the cohort scheme
looked unclaimed. That assertion was substantially wrong, and the correction
belongs next to the design.

Region-level reference counting is established. Gay and Aiken's RC, an
extension to C, manages regions explicitly and reference-counts the regions
rather than the objects; the standard summary of the approach notes that
counting at region level both shrinks counter overhead and removes the need
for cycle detection. That is the same mechanism as "count the cohort, not
the nodes," and it predates this design by decades.

Perceus is the opposite situation and is worth stating precisely, because it
is often cited loosely. Koka's heap is acyclic by construction: its data
types are inductive or coinductive and immutable, so cycles never arise. The
paper is explicit that it does not present a general solution to cycles and
that efficient handling of them is future work. Kanso is therefore not
duplicating Perceus on this point; it is answering a question Perceus set
aside.

What may remain distinctive is narrower than first claimed. Gay-Aiken
regions are declared and managed by the programmer, while a cohort here is
implicit, falling out of a syntactic mutation rule. And the contribution, if
there is one, is the guarantee rather than the counting: mutation
confinement is what makes a cross-cohort cycle impossible, which is what
turns region-level counting from convenient into complete. Whether a cycle
can span two regions in Gay-Aiken is unverified and is the question that
decides whether even the narrow claim stands.

Sources consulted: the Perceus technical report (Microsoft Research), the
region-based memory management survey, Cyclone's region paper, and Bacon and
Rajan on concurrent cycle collection.

### Addendum — the Gay-Aiken question, answered

The unverified fact from the entry above is settled. RC counts, for each
region, the external pointers into it — pointers not stored within the
region — so interior pointers including cycles are invisible to the count.
That confirms the mechanism as prior art.

The difference is what the count is for. In RC the programmer deletes a
region explicitly and the count exists to catch premature deletion: deleting
a region whose count is non-zero is a runtime error. The count guards
reclamation rather than triggering it.

That leaves RC with a failure mode. Two regions holding pointers into each
other both carry non-zero counts, so neither can ever be deleted; a
cross-region cycle does not corrupt memory, it deadlocks reclamation and
surfaces as an error at the delete.

This is the case mutation confinement makes structurally impossible, so the
narrow claim stands in a sharper form. The contribution is not counting the
region. It is that confining mutation removes the failure mode region
counting otherwise carries, which also lets the boundary be implicit and the
reclamation automatic rather than declared and checked. That is a
language-design result — a rule about where `set` may appear — rather than a
memory-management one, which is consistent with where it came from.

### Addendum 2 — settled from the paper, and narrowed twice more

Read Gay and Aiken directly rather than inferring. Section 3 answers the
cross-region question in one sentence:

  "cyclic data structures can be used transparently as long as the cycles
   are contained within a single region. When a cycle crosses regions, it is
   the programmer's responsibility to break it before attempting to delete
   any of the regions involved in the cycle."

So RC does not have the guarantee. Cross-region cycles are legal, and
breaking them is an explicit manual obligation.

Two further corrections, both narrowing the claim.

The idea is older than Gay-Aiken. Their related work credits Bobrow (1980)
as the first to propose regions as a way to make reference counting tolerant
of cycles; Ichisugi and Yonezawa carried it to distributed systems.
Gay-Aiken is the practical C system, not the origin.

Automatic deletion at zero was already on the table. The paper lists it as a
design option — "implicit region deletion: at various times, e.g., when
memory is running out, the system deallocates any regions whose reference
count has dropped to zero" — and they chose explicit deleteregion because RC
is a C dialect. So an implicit boundary with automatic reclamation is not
distinctive either.

What survives is one thing, precisely bounded: cross-region cycles are a
documented programmer obligation in the prior art, and mutation confinement
makes them unrepresentable. Not the counting, not the automation. The
elimination of that obligation.

## 2026-07-24 — join lands in the browser; wasm parity is complete

The last wasm fallback closes. Browser differential reads 39 passed, 0
fallback, 0 failed: every golden in the corpus now runs on all three engines
with no exceptions.

Join needed far less than expected, because the scheduler was already
reachable. Desc::Join is executed by the interpreter through schedule(), and
wasm's exec_slot already hands a Value::Desc to interp.execute. So the
backend only had to *build* the description; determinism, interleaving, and
wall-crediting come from the same scheduler the other engines use, rather
than from a reimplementation that could drift.

Failure accumulation is likewise shared rather than mirrored. join_values
became pub and rt_join calls it, so "both sides err gives you both reasons"
has exactly one definition in the tree. join_accumulate's golden — an
unhandled err carrying a two-element reason list — passes on the browser
without a line of new accumulation logic.

The one wasm-specific wrinkle is the deferred pair. wasm represents an
unforced sequence as Slot::Seq rather than a Desc, so a join whose operand is
a sequence needs materializing first; as_desc walks a Seq tree into a real
Desc. A Bind cannot be materialized, since its continuation is a wasm table
closure rather than a Value, and that case dies with the same wording the
non-description case uses.

## 2026-07-24 — none campaign, step 1: the engines agree again

The differential violations found while probing none are repaired, so the
law holds while the rest of the campaign lands.

Containers hold none. Lists already did on native and interp; maps did on
native only, and the interpreter's MapLit carried a failure check on the
value that its List arm did not. The check is gone, so a map value behaves
like a list element. A map *key* is still rejected, which the literal
grammar enforces anyway.

Writing the golden turned up a third behavior nobody had looked at: the
browser ate none in both containers, since rt_mklist and rt_mkmap propagated
failures element by element. Both now keep what they are given. This is the
argument for the goldens-for-everything rule in one incident — two engines
were compared by hand and agreed, and the engine nobody thought to check was
the one that differed.

Deriving from none is rejected at check time, uniformly. The engines carry
none as a tag rather than a declared type, so native failed with "unknown
type `none`" while the interpreter accepted the declaration and silently
dropped the subtype. The differential law allows a feature to land on fewer
engines only when the others reject it clearly, and silent erasure is not
that. Support remains available to add — the type-check side is two arms in
the primitive match — and wants a customer first, which refined lookup
signals would provide.

Browser differential now reads 40 passed, 0 fallback, 0 failed.

## 2026-07-24 — none campaign, step 2: a field annotation starts meaning something

Constructor arguments were never checked against field types. `node "hello" 5`
against a type whose id is an int passed `check` and constructed happily,
which is an invalid-state-representable hole with nothing to do with none.

Full checking needs inferred types at check time, and inference currently
runs in codegen, so wiring it in is an architectural move rather than a fix.
What lands instead is the part that needs no inference at all: a literal
argument's type is known from its syntax. An int, float, non-interpolated
string, list, map, or boolean literal handed to a field annotated with a
different concrete primitive is now a compile error naming both.

The check is deliberately narrow. It stays silent unless the field declares
exactly one concrete primitive and the argument is a literal, so `any`
fields, typesets, records, subtypes, unannotated fields, and every computed
expression pass untouched. It reports what is provably wrong and nothing
else, which is what lets it land now rather than behind inference.

This is also the first machinery the rest of the campaign rests on. The
prohibition on none in collections and the exhaustiveness rule both need a
place where a value is compared against a declared type; this is that place,
holding one case.

## 2026-07-24 — none campaign, step 3: exhaustiveness measured before it is imposed

The gavel says a function receiving a none must state its disposition, and
no arm is a compile error. Two facts were needed before writing that check
for real, and both are now in hand.

The mechanism already exists. `fn shout none` parses and dispatches today —
a none arm is an ordinary nullary pattern, and a program with one prints
from that arm. Exhaustiveness is therefore about *requiring* the arm, not
about inventing a way to write it.

The migration is large. An env-gated checker (KANSO_EXHAUSTIVE) runs
whole-program inference and flags any group whose parameter can receive a
none with no arm for it. Across examples it reports 127, and lib/list
reports 12:

    fanout 12, fn_value 1, fn_value_multiarg 2, generators 14, imports 13,
    json_failure_door 34, lists 12, next_protocol 13, ordering 14,
    std_list 12

json's failure door alone accounts for 34, which is unsurprising — a parser
built on auto-propagation is exactly the code that receives failures
everywhere without naming them.

So imposing exhaustiveness is a migration measured in scores of arms across
the stdlib, the examples, the book samples, and the three sibling repos, not
a step that lands with a checker. The checker ships gated and off, because
that is the instrument the migration needs: turn it on, fix a batch, watch
the count fall, and turn it on for real when it reaches zero.

One caveat on the number. Inference joins argument sets over all call sites,
so a group called once with a none and elsewhere with a value is flagged
even where a particular caller resolves it. The 127 is an upper bound on
work and a lower bound on nothing.

### Correction — the exhaustiveness count was inflated about threefold

The 127 recorded above is wrong. It summed per-file reports, and every
example that imports std/list re-counted the same twelve stdlib functions.
Deduplicating by (function, position) across the whole example corpus gives
43: twelve in list, the rest local to json, text, and the examples
themselves.

The number is also an overestimate for a second reason, visible in the
smallest case. fn_value reports `double` as able to receive a none, but
double is only ever called as `twice 7` with a literal. It is flagged
because `twice = double` uses the function as a value, so inference cannot
see through the indirection and falls back to a set that includes none. Any
function used as a value is flagged this way.

The character of the work was also misdescribed. These are not sites that
need rewriting away from auto-propagation. The scanner indexes leniently on
purpose — `if (ws? (cs[p])) ...` relies on running off the end returning
none, which is how end-of-input is detected — so what each site needs is a
one-line arm naming that case (`fn ws? none` returning false). That makes
explicit what the code already depends on, and err-propagation, which is
what the json parser's no-plumbing property actually rests on, is untouched
because err remains the failure.

So the migration is roughly forty one-line arms, minus the false positives,
rather than a rewrite of the library that best demonstrates the design.

### The order was wrong: exhaustiveness cannot be measured before the demotion

Starting the migration on lib/list turned up a reason the planned sequence
is backwards. Take next's cursor arm:

    if (length source < at) (done true) (step source[at] (cursor (at + 1) source))

source[at] is a lenient index. Today, a none there propagates through the
step constructor, so next returns a bare none and first_of receives a value
matching none of its arms — which is exactly why first_of is flagged.

After the demotion, none stops propagating. The step is built holding the
none, first_of receives a step, and its existing `(step e _)` arm matches.
The flag disappears with no arm written.

So a large share of the forty-three are artifacts of the propagation being
measured, not of the language the campaign is heading toward. Measuring
exhaustiveness under current semantics measures the wrong world, and
migrating against that list would add dead arms to the stdlib.

The two steps are one piece of work. Demoting none changes which values
arrive as whole arguments rather than as fields inside structures, and that
is precisely what exhaustiveness reports on. The open question the demotion
has to answer on its own is what an operation does when a none reaches it —
`none + 1` propagates today and must mean something afterward — and that
answer, not a pre-measured site list, is what the remaining work turns on.

## 2026-07-24 — GAVEL: an operation on a none is a dispatch question

The open question the demotion turned on is answered: `none + 1` looks for a
`+` arm taking a none and an int, and a compile error follows when no arm
matches. Arithmetic needs no rule of its own.

This closes the campaign's design. Operators are already dispatch groups —
user arms for `+` ship on all three engines — so demoting none from the
failure set does not leave arithmetic undefined. It leaves it dispatched,
like everything else, and the missing-arm case is the same exhaustiveness
rule applied at an operator rather than at a named function.

The shape of the remaining work follows from that. There is no separate
"what does none do in arithmetic" mechanism to build; there is one rule,
which is that a value arriving where no arm accepts it is a compile error.
Exhaustiveness is that rule, and it covers operators for free.

## 2026-07-24 — none is demoted: err alone is the failure

The predicates now read err and nothing else. `is_failure` matches ErrV;
`k_not_failure` tests K_ERR. none stops propagating and becomes what the
gavel says it is, a value.

The measurement that preceded the change is the point of the entry. Flipping
both predicates and running the corpus changed exactly one golden out of
forty-four examples, and it changed to the right answer: a construction that
used to collapse to `<none>` now builds, so `build`'s `set` writes to a real
record instead of silently doing nothing. That was the footgun papered over
when the set-on-failure divergence was repaired, and demotion removes its
cause rather than its symptom.

Two fears turned out to be unfounded, both stated earlier in this log and
both wrong.

Demotion does not trade propagation for silent nil. An unhandled none
reaching an operation already errors loudly and identically on both engines
— `none + 1` and `xs[9] + 1` both report that `+` is not defined for these
values. That is Clay's dispatch gavel already holding at runtime: the
operator finds no arm and says so. Exhaustiveness moves that report from run
time to compile time, which is an improvement rather than a prerequisite.

Demotion also costs nothing. The decode cost golden is byte-identical, so
removing a tag test from the hottest predicate in the runtime is free.

Goldens: none_is_a_value pins a record field holding a none through a build
block; none_no_arm pins the no-arm operator error; build_set_err keeps the
failure-propagation case with an err, which is still a failure, and moves to
the runtime corpus because an unhandled err exits one.

### The demotion is not finished until inference follows

Re-running the exhaustiveness probe after the demotion returns the same
forty-three sites it returned before. The reason is that only the runtime
was demoted. infer.rs still defines FAIL as NONE | ERR and still threads it
through the rules that model propagation, so the probe reads a world that no
longer exists.

That sharpens the earlier ordering note. Demoting first is right, but
"demote" has to mean the runtime and the analysis together — the runtime
decides what programs do, and inference decides what the checker can see
them doing. Until the second one moves, no measurement of the new world is
possible.

The change is not merely cosmetic, which is why it is not bundled here.
codegen consumes inference through set_of to elide tag checks on paths it
believes cannot carry a failure, so narrowing FAIL changes what the emitter
proves and therefore what it emits. It wants its own pass with the cost
goldens watched.

What was verified about the demotion in the meantime: codegen never consumes
type_fields, so the `TOP & !FAIL` fallback for a destructured field cannot
elide a guard, and a none stored in a field reads back correctly and errors
correctly under arithmetic on both engines.

## 2026-07-24 — inference follows the demotion, and a latent flake surfaces

FAIL now means ERR. It named the set of things that propagate on their own,
and after the runtime demotion a none is not one of them.

The emitter follows for free. codegen consults FAIL to decide whether a
binding or a parameter needs a k_not_failure guard, and that predicate had
already narrowed to K_ERR, so guards that could no longer fire were still
being emitted. Both cost goldens are byte-identical, so the removal costs
nothing measurable and the paths that mattered were already tight.

Re-measuring exhaustiveness returns forty-three again, but the coincidence
hides the real result: the *set* changed substantially, exactly as the
ordering note predicted. first_of, fold_go, skip_one, and the four next_*
helpers all dropped off, because a none from a lenient index is now built
into the step rather than propagated as a bare argument. New sites appeared
where those field contents are later consumed — esc_byte, u_bytes,
list/select among them. The nones moved rather than vanished, which is what
the note said would happen and is now on the record as measured rather than
argued.

The change also surfaced a flake that had been latent. check_predicates
iterated a HashMap, so a file with more than one naming diagnostic printed
them in a different order every run. One diagnostic hid it; the sharper
inference made three fire in a book sample and the golden began failing
intermittently. Diagnostics now sort by span, which is both stable and the
order a reader expects. This was a CI flake waiting for any change that
widened a diagnostic set.

## 2026-07-24 — the collection prohibition lands

A list or a map may no longer hold a none. A record field still may, which
is the whole of gavel two: a lookup answers "not found" with a none, so a
collection that could also store one makes every lenient read ambiguous,
while a record field is known to exist and a none there means the value is
nothing and nothing else.

The check follows the same shape as the constructor-literal check: it fires
on a none written literally into a list or map and stays quiet everywhere
else, so nothing computed is guessed at. That is enough to close the gavel's
concrete case and leaves the inference-driven version for when the checker
has types at hand.

The step-one example that pinned containers holding a none is gone, replaced
by two error goldens. It recorded agreement between the engines on a
behavior the design had already ruled out — correct as a differential fact
at the time, wrong as a specimen of the language.

## 2026-07-24 — `none` becomes a nameable type in an annotation

`x:none` worked in the interpreter and failed to compile on native with
"unknown type `none`", while the checker allowed it. That is the same family
as the subtype-of-none divergence repaired in step one, which only covered
`type X none` and left the annotation form alone.

The engines now agree. codegen gains a `none` arm testing K_NONE alongside
the other primitives, wasm takes check code seven, and the interpreter
already matched. A dispatch group can now name the absent case directly:

    fn describe x:none
      "nothing at all: {x}"

    fn describe x:int
      "the int {x}"

This is what a lookup's consumer has been unable to say. Feeding it `xs[9]`
selects the first arm and `7` the second, byte-identically on all three
engines.

Field typesets stay closed. `v:int none` is still rejected at check time,
because that construct resolves member names on a path native does not share
with parameter annotations — and `any` fails there too, on main, before any
of this. Opening the name without fixing that path would have bought a
checker that accepts what an engine cannot build, which is the divergence
this entry exists to remove. The field-typeset resolver wants its own pass,
and gavel three, which asks a field to spell out `any none`, waits on it.

## 2026-07-24 — field typesets work, and the duplication that broke them

`v:any int` failed to compile on native with "unknown type `any`" while the
interpreter accepted it, and `v:any none` failed the other way round once
native was fixed. Both are now correct on all three engines, and a golden
pins them.

The cause is a near-duplicate. Native resolves a member of a *parameter*
typeset through type_check_call and a member of a *field* typeset through
member_check_call, and the two carried different sets of arms.
member_check_call knew int, float64, string and bool; it had never learned
any, none, or err. The interpreter had the mirror gap in type_match_depth,
which knew none and err but not any, so it rejected an int in an `any`
field. Two resolvers that must agree, drifting silently, with no golden
covering the construct.

Both are filled in and `any` now means what it says in every position. The
duplication itself stays, which is worth naming: this class of bug is not
fixed until one of the two resolvers calls the other, and the only reason
they are separate is that one takes &mut self for interning while the other
does not. That is a small refactor and a real one.

`any` compiles to a constant true rather than a call, so the check costs
nothing where it is trivially satisfied. Both cost goldens are unchanged.

## 2026-07-24 — one type-name resolver

member_check_call now calls type_check_call. The duplicate is gone, and with
it the mechanism behind six divergences found today: a type name that one
resolver knew and another did not.

The split was vestigial. type_check_call took &mut self and member_check_call
took &self, which is the only reason they could not be the same function, and
type_check_call never mutated anything — the &mut had simply never been
narrowed. Changing it to &self let the field path delegate in one line.

The merge is also a capability. Field typesets inherit subtype-awareness,
which member_check_call never had, so a subtype now works as a member:

    type money int

    type wallet
      amount:money none

    wallet (money 350)   ->  wallet 350
    wallet none          ->  wallet <none>

Both engines agree and the golden covers it alongside the any, none, and
numeric-typeset cases.

Worth stating plainly, because the day produced six of these: every one was
a type name some path knew and another did not, and each was found only when
a program happened to exercise the pair. The goldens now cover the construct
rather than the instances, which is the difference between pinning a bug and
pinning a feature.

## 2026-07-24 — `any` excludes the absence channel, and a third copy of the failure test

Gavel three holds in the engines. `any` accepts every value a slot may hold
and rejects a none, so a field or parameter that wants anything-or-nothing
spells it out as `any none`. The shipped none-is-a-value example does exactly
that now.

Reaching that turned up a bug in the demotion itself. The predicate that
decides "does this abandon the computation" existed in three copies: the C
function k_not_failure, an LLVM inline twin of it in the codegen preamble,
and inline_not_failure, which emits the tag comparisons directly rather than
calling either. The demotion changed the first. The other two still tested
K_NONE, so native's inlined paths went on treating a none as a failure —
including the field-typeset check, which is how the divergence surfaced.

That is the same shape as the resolver duplication fixed an hour ago, and it
is worth stating as a pattern rather than an incident: when one predicate has
three implementations, changing its meaning is a three-site edit that nothing
enforces. The counters and goldens caught neither copy, because the corpus
never had a program where none-as-failure and none-as-value differ. The test
that found it was a field typeset excluding none, which did not exist until
today.

Both cost goldens are unchanged, so removing a tag comparison from the
hottest inlined predicate in the emitter costs nothing measurable.

## 2026-07-24 — one failure test, and a golden that would have caught the drift

The emitter no longer restates the failure predicate. inline_not_failure
calls the alwaysinline twin instead of writing the tag comparison itself, so
the three copies are two: the C function and the LLVM twin it mirrors. Both
cost goldens are unchanged, because the twin is inlined at the same site the
comparison used to sit.

The two that remain cannot be merged — the twin exists because LTO declines
to inline across the .ll/.o boundary, which is the whole reason it was
written — so the drift risk is pinned instead. A structural spec reads the
twin out of the emitted IR and asserts it tests the err tag and no other.
Reintroducing the none test makes it fail, which was verified by putting the
old body back and watching it go red.

This is the golden that #217 needed and did not have. The demotion changed
one of three copies, and the suite, both cost goldens, and the browser
corpus were all green afterward, because nothing in the corpus separated
none-as-failure from none-as-value. A behavior golden cannot catch a
semantic change the corpus is blind to; a structural one reads the claim
directly.

## 2026-07-24 — the exhaustiveness probe gets honest

Two sources of noise are gone from the gated checker and the count falls
from forty-three to thirty.

A parameter whose inferred set is TOP means inference lost the call sites,
which happens whenever a function is used as a value. That is an absence of
evidence rather than evidence of a none, and it accounted for twelve of the
reports — including double, which is only ever called with a literal seven.

The other fix is a plain bug in the probe. It recognized a bare `none` arm
and not an `x:none` annotation, so it flagged the very example shipped an
hour ago to demonstrate the annotation form. Both spellings state the
disposition and both now count.

What the remaining thirty show is a diagnostic-placement question the gavel
leaves open. inference joins argument sets over every call site, so one
caller passing a possibly-none taints the whole group: list/select is
flagged at position zero because somewhere a collection that might be none
reaches it. The gavel makes the callee responsible — an arm, or every caller
resolves — so flagging the group is faithful. But the code that needs
changing is usually the caller, and the message names the callee. Pointing
at the call site needs per-site sets, which inference does not keep.

That is the difference between a measuring tool and a shipped diagnostic,
and the probe stays gated until it can name the line a reader has to edit.

## 2026-07-24 — exhaustiveness reports at the argument

The gated checker now names the line an author edits. Instead of reporting
that a group can receive a none somewhere, it reports the argument that
carries one into a group with no arm for it:

  this can be a none and `list/unwrap_found` has no arm for it — resolve it
  here, or give `list/unwrap_found` a `none` arm

It fires only on what is provable without per-expression inference: a
lenient read, a literal none, or a call to a group whose joined return set
carries one. Everything else stays quiet, so the report is evidence rather
than suspicion.

The real migration is smaller than any earlier count suggested and it lives
in the standard library. Four sites in list and nineteen in json account for
all of it; the per-example totals repeat those, once per importing program.
Fixing the stdlib once leaves user code clean.

One defect blocks turning this on, and it is not about none. A diagnostic
whose span belongs to an imported module is attributed to the entry file:
examples/std_list.kso is thirteen lines long and the report points at line
one hundred fifty-five, which is a line of lib/list. The group-level form
happened to dodge this by reporting at a declaration, where an existing
mechanism appends the owning module. An argument span carries no such note.
Until a reader can follow the pointer to the line that produced it, a
compile error here would send people to the wrong file.

## 2026-07-24 — a diagnostic names the module it came from

The exhaustiveness reports now say which module owns the line. Before, a
merged program's span was rendered against the entry file, so a thirteen-line
example was told to look at line one hundred fifty-five:

  examples/std_list.kso:155:17

The declaration carries its own file, so the report says so directly, with
the path trimmed to the module a reader would have searched for:

  ... or give `list/unwrap_found` a `none` arm (in std/list/list.kso)

This is not a none problem. Any check reporting at an expression span in a
merged program has it; exhaustiveness is simply the first one to do that,
since the others report at declarations, where an existing mechanism appends
the owning module. The general repair is a span that carries its file, which
is a wide change and worth doing on its own terms rather than inside a
feature.

With the owner visible the migration reads clearly: sixty-four sites in
list and eleven across json's four files. Those are the arms to write.

## 2026-07-24 — the first exhaustiveness arm, and a zero that was a lie

list/unwrap_found gained the forwarding arm the gavel asks for, and the
gated checker falls from seventy-five reports to eleven. All sixty-four
inside list clear on one line; the eleven that remain are json's.

    fn unwrap_found none
      none

The function already behaved this way — a bare `found` parameter caught a
none and returned it — but the gavel is explicit that a catch-all binding is
not a stated disposition. The arm makes the absent case visible where a
reader looks for it, which is the whole point of the rule.

The first attempt reported zero, and the zero was false. Placing the arm
after `(missing _)` violated the most-specific-first ordering, so every
program importing std/list failed to compile and never reached the check.
A checker that runs late reports nothing when the build breaks early, and
nothing reads exactly like success. What caught it was asking whether the
examples still produced their goldens rather than trusting the count — the
suite would have caught it too, one step later.

Worth keeping as a rule of thumb: a diagnostic count falling to zero after a
one-line change deserves the same suspicion as a benchmark that suddenly
doubles. Both are usually a broken measurement.

## 2026-07-24 — three json arms, and the limit exhaustiveness runs into

Three of json's eight receivers state their disposition now, and the gated
count falls from eleven to five.

mark_step? answers false, because a byte past the end of input is not a
float mark. expect_check fails at the position, because input ending before
the expected byte is the same outcome as the wrong byte arriving.
hex_digit errs with the parser's own end-of-input wording, because a
truncated \u escape has no digit to read.

The five that remain are false positives, and finding that out is the point
of the entry. Every one is a lenient index guarded by a length test one line
above it:

    if (length xs == 0) (text/append acc "[]") (encode_list acc xs)
    if (length xs < i) acc (encode_items (elem_onto acc xs[i]) xs (i + 1))

encode_list never sees an empty list, so its xs[1] is always in bounds.
Inference does not track the guard, so it reports the index as a possible
none and blames the receiver.

Silencing those would mean writing arms for states the program cannot reach,
in the library the benchmark numbers come from. That is dead code added to
satisfy a checker, and the checker is the thing that is wrong. Exhaustiveness
needs to see a guard before it can be turned on — flow sensitivity, not more
arms.

Two tests came out of the investigation and stay: encoding an empty list and
an empty map. Both pass, which is what proved the guards hold.

## 2026-07-24 — the strict index says what the guard already knew

Four guarded lookups in json's encoder became strict, and the gated count
falls from five to one. No arms were written and no checker was taught
anything.

    if (length xs == 0) (text/append acc "[]") (encode_list acc xs)
    ...
    encode_onto (text/append acc 91) xs[1]!

The guard already proves the index is in bounds, so the lenient form was
claiming a miss was possible when the author knew it was not. `!` states
that knowledge, removes the none from what flows onward, and turns a broken
invariant into an err at the exact index rather than a none wandering into
an encoder.

This is the rule Clay gave when the campaign started: either the none case
needs handling, or it does not and the strict form lets an err bubble. The
five sites left after the arms were all the second kind. Reading them as a
checker deficiency was the wrong diagnosis — flow sensitivity would have
been machinery bought to tolerate an under-specified spelling, when the
language already had the specific one.

Both cost goldens are unchanged and the decode checksum is intact, so the
strict form costs nothing on the benchmark path.

One report remains, on text/utf8 in the escape path. It wants the same look.

## 2026-07-24 — the enumerable's guarded lookups say so too

The three length-guarded lookups in list take the strict index, matching
json. fold_flat's element read, and the cursor and cycled arms of next, are
all dominated by a length test, so the lenient spelling was claiming a miss
the code had already ruled out.

The cursor arm needed the same shape its cycled sibling already had — the
successor bound to a local — because the added `!` pushed the line past
eighty characters. The two arms now read alike, which they should have from
the start.

A second false zero appeared here and is worth recording next to the first.
The count read zero while the over-long line was in place, because the file
no longer parsed and a checker that never runs reports nothing. The suite
caught it. That is twice in one session that a zero meant a broken build
rather than a clean corpus, from two different causes — wrong arm order, and
a line one character too long.

One report survives, and it is not a guard artifact. escape_onto's chain
bottoms out at text/find2_below, a builtin whose not-found answer is a none,
so the none is real and reaches text/utf8 through escape_clean. That one
wants a decision about what find2_below returns rather than a change of
spelling.

Both cost goldens unchanged, decode checksum intact.

## 2026-07-24 — what the last exhaustiveness report is, and is not

The one surviving report is an inference artifact, not a none.

The chain was worth tracing. text/utf8 is handed escape_onto's result, which
is escape_clean's, which is either a text/append or escape_able's fold. The
obvious suspect was text/find2_below, whose not-found answer would plausibly
be a none — but its inferred set is `INT | fails`, and fails now carries err
alone, so it never yields one.

The none comes from the join. `fold` is a group of ten arms, one per
enumerable shape, and a group's return set is the union of all of them. A
none reachable in any single arm's path becomes part of what every call to
fold appears to return, including the call escape_able makes with a plain
list. json's suite passes and encoding escapes works, which is the runtime
answer: no none is produced there.

So the checker's two remaining imprecisions are now both identified and are
different from each other. One was a guard it could not see, and the strict
index removed the need for it. This one is a summary that is coarser than
the dispatch it summarizes: per-arm returns would resolve it where a
per-group return cannot.

That is the state exhaustiveness is in. Seventy-five reports became one, the
one is understood, and turning the check on waits for return sets that
follow arms rather than groups.

## 2026-07-24 — return guards compile again, and where they still collide

Gavel BB's branch now builds against main after a rebase across a hundred
and ninety-four commits, and the feature works:
## 2026-07-24 — return guards land: gavel BB is in the language

`return X if C` works on both engines, build blocks still work, and the
suite is clean. The nine-day-old branch is merged rather than stranded.

    fn describe n
      return "below zero" if (n < 0)
      return "past a thousand" if (1000 < n)
      "ordinary {n}"

Native and the interpreter both print below zero, past a thousand, ordinary
7. The branch is preserved as return-guards-revive.

The drift was four real API changes and a dozen missing match arms. Two of
the four are worth noting because they left the tree better. eval_body had
become the inline body of the Block arm, so evaluating a statement list in
expression position is now a method, eval_stmts, that the block, the build
block, and a fired guard's tail all share. emit_fn_body took a decl only to
read a name and an arity that FnEmit already carries, so the parameter is
gone.

What blocks the merge is not drift. The branch replaces parse_body wholesale
with a version that finds the leading run of bindings and returns, and it
was written before build blocks existed, so a build body no longer reaches
parse_build_body. The mem golden catches it: the block's final `[a b]` is
reported as a value that is never used.

That is a parser-level design question — where a guard may sit relative to a
build block's statements, and which of the two bodies owns the tail — rather
than an integration chore. Recording it here so the branch is picked up with
the question already framed.
The conflict was real and worth naming. The branch replaced parse_body with
a version that splits a body into a leading run of bindings and returns,
then an effect tail — written before build blocks existed. Its lead was
parsed line by line, so a construct's indented children were orphaned: a
build block's `[a b]` came back as a value nobody used, and an `if` with an
`else` lost its second branch.

The repair is that the lead groups exactly as the tail does. One helper walks
the leading lines, gives a header its indented children, follows an `else`
sitting at the header's own indent, and falls back to a flat statement for a
chain-led continuation so the real diagnostic still stands alone. The
boundary computation had to learn the same thing: skip a construct's
children, and skip the `else` block that belongs to it.

Two cleanups came out of the drift and stay. eval_stmts is now the one place
a statement list is evaluated in expression position, shared by blocks,
build blocks, and the tail a fired guard skips. emit_fn_body no longer takes
a declaration it only read a name and arity from, both of which FnEmit
already carries.

The browser reports one fallback: guards are not in the wasm backend, which
the differential law permits so long as the rejection is clear. Both cost
goldens are unchanged and the decode checksum is intact.

## 2026-07-24 — guards in the browser; parity is whole again

The wasm backend emits return guards, and the browser differential reads
45 passed, 0 fallback, 0 failed. The fallback that arrived with gavel BB
lasted one commit.

The emission is the shape the gavel already describes. A guard is a
conditional whose untaken branch is the rest of the body, so the backend
reuses what it does for `if`: test the condition for failure and hand the
failure back, otherwise branch on truth between the early value and the
tail. emit_body already evaluated a statement list, so the tail needed
nothing new.

Both cost goldens are unchanged. The guard costs nothing on the compiled
path because it compiles to the same instructions the equivalent `if` would.

## 2026-07-24 — find gets its fast arm

`sorted` is a type in list, and `find` has an arm for it that bisects
instead of scanning. This is the shape Clay asked for: one generic find, and
a supercharged version for a type whose storage rewards it, selected by
dispatch rather than by a second name.

    pub fn find (sorted items) pred
      bisect items pred 1 (length items) none

    pub fn find coll pred
      unwrap_found (seek coll pred)

The bisect is the first real customer of the guards that landed an hour ago:
`return best if (hi < lo)` is the base case, flat above the halving step
rather than nesting it. Six tests cover the head, the tail, the miss, the
empty source, and agreement between the two arms — list had no tests at all
before this.

The contract is that the predicate is monotonic over the order, false for a
prefix and true for the rest, which is what makes halving sound. That is
written where the function is.

Two things the work turned up. `place` had a parameter named `sorted`, which
the new type made a collision — renamed to `run`. And a test file that
imports its own module gets a second copy of every type, so a constructor
pattern in the library never matches a value the test builds; the six tests
failed until the self-import came out. json's tests never had one, which is
why the pattern held there.

Both cost goldens unchanged. Browser 45 passed, 0 fallback, 0 failed.

## 2026-07-24 — a module that imports itself now says so

Adding tests to list cost an hour to a trap worth closing. A test file in a
module that imports its own module compiles a second copy of it, so every
type gets a twin, and a constructor pattern written in the library stops
matching a value the test builds with what looks like the same constructor.
The failure surfaces as `length takes a list or string` from deep inside a
fold — nowhere near the import that caused it.

The import now names it:

    error: a module cannot import itself — `std/list` is this module, and
    the second copy's types would not match this one's

Both resolution paths carry the check, which is the part that took the
digging. A std import of a shipped module does not go through
resolve_import at all: an embedded table holds the source so the browser and
a binary with no lib/ beside it can still load it, and that branch returns
before any path comparison happens. Checking only the filesystem path left
the trap fully intact for exactly the modules most likely to hit it.

json's tests never had a self-import, which is why the convention looked
fine until list grew tests of its own.

## 2026-07-24 — the site catches up with the none model

Two doc surfaces were claiming something no longer true.

failure-kinds.html listed "automatic propagation ... none as dispatchable
absence" as shipped, which bundles the two channels into one clause. After
the demotion only err propagates, so the line now names the split and the
obligation each side carries: an err propagates on its own and cannot be
absorbed, because a function accepting one must return one; a none
propagates nowhere, lives in a record field but never in a list or a map,
and reaching an operation with no arm for it is an error.

compiler.html's technique list gained the same result as an entry of its
own, next to the birthday theorem. It belongs there for the same reason the
theorem does: it is a rule the language enforces rather than a library
convention, and it is what makes a lookup's not-found answer mean one thing
nothing else can forge.

No performance surface moved today — both cost goldens and the decode
checksum have been identical through every change — so the number-bearing
boards are current and were left alone.

## 2026-07-25 — three more specs on the compiled output, and a claim of mine they caught

Asked what pins compilation itself, the answer was four structural specs and
no golden holding the emitted text. A full IR dump would churn on every
temporary rename and every llvm version, so the shape stays: assert the
claims that matter and let the rest move.

Three claims made this week were load-bearing and unpinned, so they are
specs now. `any` emits a call that tests the tag. The strict index emits the
erring form rather than the lenient one. A guard compiles to a conditional
branch inside its own definition.

Writing the first of those turned up a stale claim in this log. The entry
for the field-typeset fix says `any` compiles to a constant true and costs
nothing where it is trivially satisfied. That was accurate when written and
stopped being so an hour later, when `any` had to start excluding none and
became a real tag test. The log is append-only, so the correction lives
here: `any` is a call, and the earlier sentence describes a compiler that no
longer exists.

Each spec was watched failing before being trusted — the `any` one against a
codegen reverted to the constant, which is exactly the regression it exists
to catch.

Two smaller lessons from writing them. A spec program is compiled by the
same front end as any other, so it obeys the ordering rules; `main` before
`pick` is why the guard spec first failed for a reason that had nothing to
do with guards. And slicing a function out of the IR has to anchor on the
`define` line, or the first call site of that function answers instead.

## 2026-07-25 — a golden for what compiling costs

bench/compile_golden.txt pins how much work the emitter does on five sample
programs, counted rather than timed:

    recursion   lines=374 calls=21 branches=15 defines=19
    dispatch    lines=368 calls=22 branches=14 defines=19
    guards      lines=367 calls=20 branches=15 defines=19
    records     lines=415 calls=25 branches=18 defines=19
    build_block lines=344 calls=14 branches=10 defines=18

The samples cover recursion, dispatch over literals and types, a guard
chain, record construction and destructuring, and a build block closing a
cycle. Every number is text this compiler chose to write, so the file is
exact on any machine and moves only when codegen does.

This is the compile-side companion to the runtime cost goldens, and it works
the same way. Wall time would have said more about the laptop than the
compiler; a count says which change added the work. Losing forwarder
elision, emitting a guard where inference used to prove one unnecessary, or
un-inlining a predicate all land as a diff on a specific line rather than a
slower afternoon nobody can reproduce.

Verified by regression rather than by passing: an extra instruction injected
into inline_not_failure moved four of the five programs and failed the
golden, and removing it restored the file. cargo test already carries it, so
CI gates it with no workflow change.

## 2026-07-25 — the compile golden counts the work, not just the writing

The first version of this golden counted emitted text, which Clay pointed
out measures the product rather than the process: a compiler can grind a
long fixpoint and write three lines. So inference now counts what it does —
rounds of the fixpoint, and expression visits inside them — and the golden
carries both kinds of number.

The samples show the two are not redundant. guards emits fewer lines than
dispatch and costs nearly twice the visits; recursion is the only one whose
fixpoint needs a third round. Output volume and effort genuinely diverge.

Verified the way the twin spec was: an extra fixpoint round forced into
inference moved every rounds and visits count while every emitted count
stayed byte-identical. That is precisely the regression line counts alone
could not see.

The file now carries its policy in its head, because the policy is the
subtle part. This is a watched trend, not a floor. Compilation and runtime
trade against each other, and a feature may cost one to buy the other, so
movement is expected and silence is the failure: regenerate deliberately,
say which way it went and why, and write the reason down beside the number.
The same sentence is in CLAUDE.md so it governs the runtime veins too.

## 2026-07-25 — the boards re-measured under one method; the serde lead was an artifact

Audit of every number-bearing surface found five different figure sets
across four pages, and the headline claim did not survive re-measurement.

METHOD. Each decoder timed by slope: the same program built to run 150
and 450 times, floors differenced over the extra 300, which cancels
process startup and the file read for all four alike. This mattered.
kanso's harness is the only one that does not self-time, so the earlier
comparison clocked kanso by whole-process wall time against three rivals'
self-timed means — an asymmetry that moves the answer by more than the
gap being reported.

MEASURED (load ~50, interleaved, two sittings): kanso 0.952/0.949,
serde_json 0.947/0.972, naive rust 1.104/1.117, go 2.040/2.054
ms/decode. Peak rss 6.2 / 6.8 / 6.8 / 10.5 mb.

FINDING. kanso and serde_json are a dead heat — 0.5% apart in one
sitting, 2.4% the other, order flipping between them. The board claimed
a ten-percent idle lead and a forty-five-percent loaded lead; both are
withdrawn. naive rust and go reproduce their published floors within 3%,
so the sitting is comparable and the discrepancy is specific to the
kanso/serde pair and the method that produced it.

NOT A REGRESSION. #174 (the eisel-lemire restore) built and raced in the
same sitting: 1.001x on floors across the 67 merges since. The decode
path has not moved. The 0.89 figure came from #170, whose main lacked
eisel-lemire outright — the incident recorded on 07-23 — so no commit
reproduces it.

LAZY BOARD, same sitting: kanso 0.10, rust hand-tuned 0.08, rust as
written 1.54, kanso --strict 1.93. The 15x over rust-as-written holds;
"within 14% of hand-restructured rust" was prototype-era and measures 28%
on the shipped engine.

CORRECTED: compiler.html board, recipe and lazy table; index.html panel
and the two-engines paragraph (a third set, 0.85/0.86); about.html prose;
kq README (its pretty rows beat the idle floors quoted underneath them —
proof of mixed sittings); kq TRY.md.

STATUS LEDGER: eisel-lemire was tagged queued while shipped and pinned at
el_parses=318450. Eytzinger was tagged planned while its own text read
"measured, declined"; it now carries its own class.

OPEN: the idle-floor footnote is owed a quiet sitting.

## 2026-07-25 — closures are data in the browser too; and compile speed becomes a published claim

Clay hit `error[runtime]: a closure or bound description cannot be used
as data here` running the fanout sample in the playground. Interp and
native both run it.

CAUSE. `wasm_rt` keeps compiled closures in `Slot::C`, outside `Value`,
and every container — `rt_mkrec`, `rt_mklist`, `rt_mkmap` — funnels its
members through `val()`, which accepted only `Slot::V`. std/list's
`map coll f` is `mapped f (iter coll)`, so the closure becomes a record
field and dies on the way in. Only `map` and `filter` had a bespoke
closure guard, so every other enumerable adapter was broken in the
browser while the other two engines ran it. The native engine tags
closures inside its value union and has this freedom already.

FIX. `Value::TableFn(u32)` names a closure by its registry handle;
`val()` promotes `Slot::C` to it and `closure_slot()` resolves one hop
back before asking whether a slot is callable, so a closure read out of a
field is callable again. Handles stay valid because the registry only
grows.

VERIFICATION IS PARTIAL AND SAYS SO. wasm_rt is `#![cfg(target_arch =
"wasm32")]`, so the host build never type-checks it — a first attempt at
a unit test would have compiled away silently and read as coverage. It
was removed. The fix is verified by the wasm32 build and by reading; the
behavior itself is unverified locally, because there is no wasm host on
this box and the browser extension is not connected. The browser
differential harness still needs headless CI — this is the second bug it
would have caught.

PERF. Decode floor unchanged (144.4 → 142.4 ms per 150, inside noise);
both cost goldens and the compile golden byte-identical. `Value` gains no
size — every other variant already carries a pointer.

COMPILE SPEED, now published (§08, "how fast it compiles"). `kanso check`
— parse, whole-program inference, every diagnostic — finishes kq in
6.6 ms and the json decoder in 6.1 ms, each covering the standard-library
modules imported alongside the program's own source: about a thousand
lines for kq, so the front end clears 150k lines/second. Unoptimized
binary 116 ms; optimized 635 ms, nearly all llvm at -O2. Go builds a
28-line program against its cached stdlib in 98 ms on the same box.
2026-07-25, loaded desktop, best of seven. CLAUDE.md's done-checklist
gains the surface and a standing rule: every change carries a perf check,
not just perf PRs.

## 2026-07-25 — the playground's examples become specs; the release build stops rebuilding the runtime

Two asks after the fanout bug: specs that stop any playground example failing
on any engine, and compilation that competes with `go build`.

WHY FANOUT ESCAPED. The browser differential already runs in CI under headless
Chrome, but its corpus was `examples/` plus `tests/golden/runtime/` — never the
samples in play.js — and it skipped any program containing `import`, on the
grounds that the browser has no filesystem. That rule outlived its reason:
std/* resolves in the tab because the toolchain embeds it. fanout was outside
the corpus twice over. The corpus now reads play.js and skips only relative
imports; it went from 48 programs to 66.

The widening exposed three pre-existing gaps, so gaps became explicit instead
of silent: KNOWN_GAPS names each program the browser declines and the phrase it
declines with. The harness fails if the phrase changes AND if an entry starts
passing — delete it then. Standing: `examples/concurrency.kso` and the
playground's own concurrency sample error with "a group joins descriptions"
(the scheduler is not lowered to wasm, and a visitor picking that example sees
the error), and `json_failure_door.kso` wants `std/json`, which is not in the
shipped library.

FANOUT IS NOW VERIFIED IN THE BROWSER, not merely argued. Chrome is present
locally after all; 61 passed, 3 known gaps, 2 fallback, 0 failed.

tests/playground.rs reads the same samples for the host engines: each runs on
the interpreter, each agrees byte-for-byte with native, each survives the
browser backend's encoder. KANSO_SEED pins the dice — without it the
concurrency sample's rolls differ every run and the engines can only be
observed disagreeing, never compared.

tests/native.rs was not differential. It compared `kanso run` against a built
binary, and `run` compiles native, so both sides were the same engine. It now
passes --interp for the oracle side.

COMPILE SPEED. Release recompiled runtime.c at -O3 -flto on every build,
206ms of it, while dev had cached its object since the beginning. The cache is
now keyed by profile and release uses it: **635 → 362 ms**, a 43% cut, with
the runtime unchanged at +0.8% (noise). An intermediate measurement suggested
an 85% runtime regression; that was my error — the binary I sampled was the
-O0 dev build left on disk by the previous command. Isolating the two link
forms directly showed 147.0 vs 145.3 ms, and prebuilt bitcode LTOs exactly as
the source does.

A ratchet holds it: two release builds in a row, and the cached object's mtime
must not change. Deterministic, unlike a stopwatch.

CI is one job per kind of check — lint, specs, playground, decoder, book,
cost-goldens, browser, benchmark — so a red run names what broke on the PR page.
clippy gains --all-targets; the specs were never linted before.

OPEN: rustfmt would move 3384 lines, so it is owed its own mechanical PR
rather than riding with logic. clippy::perf is already clean and stays denied
by -D warnings; pedantic is 442 warnings and is not worth forcing wholesale.

## 2026-07-25 — the landing sample runs; the engine wiring exists once; ci publishes the counters

The landing page showed `examples/pipes.kso` and then quoted it wrong — the
panel said `main = "kanso" . greet . print` while the file says
`pub play = ...`. Nobody noticed because the panel was a picture of code. It
is not a picture now: the sample is a real editor over the real engine, and
the shape it displays is the shape the file has, because a wrong one no longer
runs.

ENGINE WIRING, ONCE. play.js held the tokenizer, the wasm load, and the
compile-or-interpret decision. The landing page needs all three, and copying
them would have put two copies of the engine contract in the tree. They live
in docs/kanso-engine.js now; play.js keeps the playground's own DOM and
landing-play.js is fifty lines of binding. The engine is a megabyte, so the
landing page loads nothing until the visitor touches the panel — a reader who
scrolls past still gets a static page.

VERIFIED, NOT ASSUMED. scripts/site_smoke.py loads both pages in headless
Chrome, clicks run, and requires the promised output. It caught two real
failures on the way in: the misquoted sample, and kanso's exactly-one-trailing
-newline rule, which a textarea does not supply on its own. It also pins
fanout running in the playground UI — the #244 regression this page must never
take again. New ci job, its own check.

Chrome's --dump-dom hangs here, as the browser differential already found; the
POST-a-report pattern is what works.

WHAT EVERY COMMIT COSTS (compiler page §08). CI publishes the deterministic
counters — allocations, arena blocks, rewind iterations, eisel-lemire parses,
fixpoint rounds, expression visits — to an unprotected perf-history branch on
every push to main, and the panel fetches and charts them. Deterministic only:
a noisy runner cannot move these, so a change in that panel is somebody's
deliberate edit. Wall-clock stays on the hand-measured board. Pages serves from
main/docs and main requires review, which is why history cannot live in the
tree with the page that reads it.

NAV. about, playground, book, compiler, github — what it is, then try it, then
learn it, then how it works. Go leads with why, rust with install and learn;
both answer the question before offering the tool.

## 2026-07-25 — concurrency runs in the browser; the scheduler stays singular

The playground offered a concurrency example that errored in the tab. Removing
it was not an option — a sample that demonstrates the language's answer to
goroutines is the last thing to hide.

CAUSE. `rt_join` needs both sides as real `Desc`s so the interpreter's
scheduler can interleave them, and `as_desc` could not materialize a
`Slot::Bind` — a piped continuation whose closure lives in the wasm table.
`rolls = roll 1 >> roll 2 >> ...` where `roll i` is `random 6 . (n -> print)`
is exactly that, so the join died.

FIX, AND WHY NOT THE OTHER ONE. `Desc::Bind` already carries its continuation
as a `Value`, and `Value::TableFn` (from the fanout fix) already names a table
closure — so the shape was expressible; only the call back was missing.
`eval::set_foreign_call` is a hook the browser backend registers, and
`Interp::call` gained one arm for `TableFn`. The alternative was a second
scheduler inside wasm_rt, which would have put the green-thread policy in two
places and guaranteed drift. One scheduler, in the oracle, is the whole point
of the differential law.

The interleaved ORDER now matches the interpreter exactly.

THE HARNESS WAS COMPARING UNSEEDED DICE. With concurrency running, the two
engines still disagreed — on the values, not the order. browser_differential
never pinned the RNG: the native side ran on entropy and the page called
kanso_set_seed with nothing. Both are pinned now. This is the same bug the
Rust playground spec had, found twice in one day, which suggests seeding
belongs in whatever a harness inherits rather than in each harness.

Both concurrency entries left KNOWN_GAPS, and the ratchet demanded it: an
entry that starts passing fails the run until deleted. 63 passed, 1 gap, 0
failed.

SITE. The landing editor's mirror did not track the textarea's scroll, so
scrolling left a ghost copy — the same defect the playground fixed long ago,
reintroduced because the binding was written fresh instead of copied. The run
button is white on accent, matching the playground's, and the editor now
shares the code panel's own type and padding rather than sitting flush to the
edge.

A brand guide lands at /brand.html, linked in the footer: the ten colors with
names, the three faces at working sizes, and the reasoning — one accent spent
deliberately, space as the material, nothing decorative that cannot be run.

## 2026-07-25 — a counter that did not move should say so

The published panel read "2 commits recorded" with an empty delta column and a
sparkline nobody could see. Both were literal-minded rendering of the normal
case: the delta was emitted only when a value changed, and a flat series
normalized to the floor of the viewBox, where a 1.5px stroke sits on the edge
and reads as absent.

An unchanged counter is the whole point of the panel — these are the numbers a
noisy runner cannot move — so it now says "unchanged" rather than nothing, and
a flat run draws down the middle. Blank cells read as unmeasured, which is the
opposite of what this vein is for.

## 2026-07-25 — profiling the decode path: constants are being rebuilt per call

Encode's profile went flat months ago; decode's had not been read since. It is
not flat. Sampling 3000 decodes:

    d_value_for_3     433    the decoder's value dispatcher
    k_utf8_bad        357    the validator (named for what it looks for)
    k_b_push_mut      175
    _platform_memmove 122
    d_str_char_4      116

`d_value_for_3` looked like a dispatch cost and is not: the switch is already
a jump table, and llvm folds the box/unbox of the raw discriminator so the
comparisons run on the incoming register directly. What it does carry is a
twelve-register prologue, because three of its arms make non-tail calls first.

Those calls are the finding. `bytes_false = [102 97 108 115 101]` compiles to
`d_bytes_false_0()`, which allocas five KValues and calls k_list_lit — a heap
allocation — on every `false` the decoder meets. The gauntlet holds 2111
`true`, 2088 `false`, 2051 `null`; across 150 decodes that is 937,500
identical lists, 6.3% of the benchmark's 14,799,465 allocations, and the
reason the hottest dispatcher needs a frame at all.

A zero-argument definition is a constant. GHC calls these CAFs and evaluates
them once for the life of the program; kanso rebuilds them per call.

DESIGN. A memoized global per constant nullary definition, filled once into
permanent storage. The runtime already caches interned single-character
strings and zero-field marker records there, with the reason written on
k_alloc_perm: an arena rewind moves the bump pointer, so permanent storage is
the only cache that is sound across beats.

SAFETY, which is the part that could have gone wrong. A shared constant that
something pushes to in place would be corruption. k_b_push_mut mutates only
when `buf->used == l->len && l->len < buf->cap` — a list with spare room at
its frontier. A constant built at exact capacity fails that test and falls
through to the copying push, so the cache is safe even if the linearity
analysis wrongly believes it is uniquely owned. Belt and braces.

Queued as technique 7 on the compiler page with these numbers. Not built.

## 2026-07-25 — OPEN, UNBUILT: the nullary call form, and where currying would fit

Recorded because it was found by running the compiler, not by reading it, and
a finding that lives in a conversation is a finding nobody has.

THE NULLARY GAVEL IS RATIFIED AND UNBUILT. `name()` — unit application, the
form that distinguishes calling a zero-argument definition from referring to
it — does not parse:

    pub play = print "{roll_7()}"
    error[syntax]: expected an expression

What works today is the lazy binding: `roll_7 = roll 7` prints 8, because a
constant binding is a value computed on demand. So there is currently no
spelling for "call with no arguments", and the reference-versus-call
distinction the gavel settled has no syntax behind it.

UNDER-APPLICATION IS AN ARITY ERROR, WHICH MEANS THE SLOT IS FREE.

    fn roll n sides
    pub play = print "{roll 7}"
    error[arity]: no 1-argument arm of `roll` (arms take 2)

One name does carry several arities — with both `fn roll n` and
`fn roll n sides` present, `roll 7` is 8 and `roll 7 2` is 9 — so the two
readings never compete: either an arity-1 arm exists and claims the call, or
none does and the call is currently rejected. Whole-program inference knows
which. That makes bare under-application unambiguous exactly where it is an
error today.

THE SHAPE UNDER DISCUSSION (Clay's, not gaveled):

  - bare under-application curries wherever it is currently an arity error
  - holes reposition and disambiguate: `concat greeting _` puts the awaited
    value second, `roll 7 _` picks the 2-arity arm when an arity-1 arm would
    otherwise claim the call, `roll 7 _ _` picks the 3-arity one
  - competing longer arms with no hole is a compile error, not a default
  - `(roll 7)` as a passable, callable-with-`()` value is the piece that needs
    the nullary form built first

I argued against auto-currying on the grounds that dispatch gives no canonical
argument order to curry along. That was wrong, and the arity test above is
why: you are not currying a function, you are under-applying an arm set.

TWO CONSEQUENCES TO WEIGH BEFORE RULING. `_` already means *discard* in
parameter patterns and would mean *await* in argument position — unambiguous
to parse, opposite in sense to read. And a partial application defers arm
selection, not merely evaluation, since arms key on patterns across all
arguments; the strictness analysis has to see through that or every partial
costs a thunk.

Also owed, from the same session: ch05 teaches the dot as `x . f is f x` and
never mentions repositioning. ch06 uses `random 6 . (n -> ...)` and explains
it as effect binding. Nothing teaches the lambda as the answer to "my value is
not the first argument" — which is the technique holes would replace.

## 2026-07-25 — GAVEL: partial application is explicit, because overloading makes completeness undecidable

Clay's argument, which supersedes both positions taken earlier today.

With arity overloading, whether an application is complete or pending cannot be
decided from the expression. `roll 7` must either dispatch now or defer, and
which one is right depends on intent rather than on any fact the compiler can
consult: a bare `roll 7` wants the eager reading, while `f = roll 7` followed
later by `f 2` wants the deferred one. Both readings are live for the same
text. That is the condition under which a language owes the programmer syntax,
and no amount of whole-program knowledge substitutes for it.

It also reaches a case an implicit scheme cannot express at all. When an
arity-1 arm exists, nothing implicit can ever produce a partial of the arity-2
arm, because the shorter arm claims the application first. That partial is
simply unreachable.

THE RULING.

  - Bare application is always a call. `f a b` dispatches on the syntactic
    argument count, exactly as today. A call short of every arm stays
    `error[arity]`.
  - Partial application is always written with holes: `roll 7 _`, `roll _ 3`,
    `foo _ _`.
  - Supplied plus holes is the arity, so the hole count names the group.
    `roll 7 _` is the two-argument roll with its first fixed; `roll 7 _ _` is
    the three-argument one. Arity picks the group, patterns pick the arm
    within it — which is already how the emitter works, since dsym carries the
    arity and d_roll_1 and d_roll_2 are separate functions that merely share a
    name.
  - Repositioning falls out of the same mechanism: `concat greeting _` puts
    the awaited value second, which currying alone could never do because it
    fills from the right.

WHAT THIS BUYS BEYOND EXPRESSIVENESS. The arity is written at the call site, so
adding `fn roll n` tomorrow cannot silently reinterpret an existing
`roll 7 _`. Under an implicit scheme that same addition would convert every
such partial into a completed call, across modules, with no diagnostic. The
explicit form has no edit hazard.

CORRECTIONS THIS SUPERSEDES, both mine, both from today. First I argued
dispatch left no canonical argument order to curry along; the arity test
disproved that — you under-apply an arm set, not a function. Then I accepted
that currying was unambiguous because a partial need not commit to an arm set,
which is true and beside the point: the undecidable thing was never which arm,
it was whether the application had ended.

`_` NOW CARRIES ONE IDEA, NOT TWO. In `fn roll _ sides` it is a position you
do not care to name; in `roll 7 _` it is a position you cannot name yet.
Either way it is a position without a name, and which side of the definition
you stand on decides the flavour.

STILL OWED BEFORE BUILDING.

  - Every hole application is a closure, so the strictness analysis has to see
    through it or the faster-than-Rust bar pays a thunk per partial.
  - The nullary `name()` form is now decoupled from this: `roll_7 = roll 7 _`
    is already a value you apply as `roll_7 2`, so `()` is owed only for
    calling zero-argument definitions, which remains unbuilt and unparsed.
  - ch05 teaches the dot as `x . f is f x` and never mentions repositioning;
    the lambda is what that passage should show today and holes are what
    replace it.

## 2026-07-25 — GAVEL (syntax): `&f` curries; `_` is a pipe-position marker only

Clay's ruling, and it supersedes the hole proposal recorded above it, which was
mine and which overloaded one glyph with two jobs.

CURRYING IS `&`.

    &add          add as a value
    &add 2        add with its first argument supplied, awaiting the rest

`&` is legal as long as some overload accepts the supplied arguments as a
proper prefix. The arity is deliberately NOT named in the syntax, and that is
the improvement over holes: the only thing overloading made undecidable was
whether an application had ended, and `&` answers exactly that. Which arity
was always decidable — it resolves when the remaining arguments arrive, since
a partial accumulates rather than committing.

The glyph is free: `&add` is `error[syntax]: expected an expression` today.

`&` AND `()` ARE COMPLEMENTS, which closes a thread from earlier today. `&`
supplies arguments without running; `()` runs without supplying. So
`roll_7 = &roll 7` against a one-argument `roll` is the fully-applied-but-unrun
value Clay asked for, and `roll_7()` is how it runs — the two halves of the
nullary gavel meeting the currying one.

PIPING IS UNCHANGED, AND `_` SERVES ONLY IT.

    x . f          f x
    x . f a b      f x a b     piped value first, written arguments after
    x . f a _      f a x       `_` moves the piped value to that position

Verified against the current engine rather than assumed: `7 . add 2` with
`fn add a b -> a - b` evaluates to 5, so the piped value is already first and
written arguments already follow. The rule adds nothing to the dot; it only
names where `_` may appear.

`_` therefore has no role in currying. In parameter position it discards; in
an argument list it marks where the piped value lands. Two positions, two
jobs, no overlap — which is cleaner than the reading I proposed, where a
single glyph carried both partial application and repositioning.

WITHDRAWN: the hole-based currying syntax recorded in the entry above
(`roll 7 _`, `roll 7 _ _`, arity named by hole count). Holes survive only as
the pipe-position marker.

STILL OWED BEFORE BUILDING.

  - `&f` applied to arguments no overload accepts as a prefix is an error, and
    it needs a diagnostic that names the arities that do exist.
  - Every `&` is a closure the strictness analysis has to see through, or the
    faster-than-Rust bar pays a thunk per partial.
  - `()` remains unbuilt and unparsed; the currying half depends on it for the
    fully-applied-but-unrun case.
  - ch05 teaches the dot as `x . f is f x` and mentions neither trailing
    arguments nor `_`.

## 2026-07-25 — CORRECTION: `&` is required only for a genuine partial, and that unmakes the `&`/`()` pairing

Clay: `&add` is superfluous, because a function with no arguments supplied is
already passed as a value. Verified — `fn apply f x` given `apply bump 7`
returns 8, so a bare fn-name is the reference and always has been.

So `&` marks exactly one case, and it is the only case with no other spelling:

    add           zero supplied      already a value      no `&`
    &add 2        some supplied      the partial          `&` required
    add 2 3       all supplied       a call               no `&`

By the no-superfluous rule, `&add` with nothing supplied should be an error
rather than a tolerated synonym for `add`.

WHAT THIS UNMAKES. I recorded, one entry above, that `&` and `()` are
complements — `&` supplying without running, `()` running without supplying —
and that `&roll 7` against a one-argument `roll` was the
fully-applied-but-unrun value Clay asked for. That is wrong on the same
grounds. A fully applied call is already unrun: the language is lazy, so
`roll_7 = roll 7` defers until demanded, which is what made it print 8 only
when interpolated. `&` on a complete application would be as superfluous as
`&` on an empty one.

WHICH LEAVES `()` NEEDING ITS OWN JUSTIFICATION. Its remaining job is the
reference-versus-call distinction on a zero-argument definition — telling
`foo` the value from `foo` the invocation. In a pure, lazy language those two
are hard to tell apart from the outside: a constant referenced is computed on
demand exactly once, and a zero-argument function called re-runs to produce
the same value. The observable difference is memoization, not result. That is
not an argument against `()`; it is an argument that the case for it has to be
made on something other than deferral, and it has not been made here yet.

The nullary gavel stands as ratified; what it is FOR is now the open question,
which is a different and smaller one than whether it parses (it does not).

## 2026-07-25 — CAF implementation reconnaissance: the mechanism, and the one obstacle

Not built. Recording what the code says so the next pass starts from the
mechanism rather than the idea.

WHAT IS ALREADY THERE. `k_deep_copy(v, KCopy*)` is a cycle-safe deep copy with
a pluggable allocator — `KCopy { KCarryBuf* buf; KMark* mark; int to_arena; }`
— and `k_copy_size(v, mark)` sizes the destination first. The carry path at
the beat boundary already does exactly the shape a CAF needs: size, malloc,
copy with `to_arena = 0`. Two call sites, lines ~712 and ~752, are the
template.

It also already lands the value at exact capacity — the list arm sets
`buf->cap = l->len ? l->len : 1` — which is the property that makes a shared
constant safe from `k_b_push_mut`, since that mutates only when
`l->len < buf->cap`.

THE OBSTACLE. `k_deep_copy` opens with `if (k_survives(p, cp->mark)) return v`
— it shares rather than copies anything that already outlives the rewind. With
`mark = NULL`, `k_survives` compares against `k_blocks` and the live frontier
`k_arena`, so a freshly built value is "surviving" by definition and the copy
becomes a no-op that hands back arena memory. Caching that is precisely the
unsoundness `k_alloc_perm`'s comment warns about: a rewind moves the bump
pointer and the cache would point at reclaimed, since-reused storage.

So freezing a CAF needs either a synthetic mark pinned at the bottom of the
first block, so nothing counts as surviving, or a third mode on `KCopy` that
copies unconditionally. The second is smaller and says what it means; the
first reuses machinery but depends on block ordering that nothing else asks
about.

WHY I STOPPED HERE RATHER THAN GUESS. runtime.c is where a wrong answer is a
heisenbug rather than a failing test, and "the copy silently shared arena
memory" is exactly the failure that would survive the goldens for a while and
then corrupt a value under a rewind. It wants a session that starts fresh on
it.

THE REST OF THE PLAN IS UNCHANGED. Detection is narrow on purpose: arity 0,
one arm, body a literal with no calls — which also rules out a CAF freeze
recursing into another and resetting the shared copy map mid-copy. Emission is
a memoized wrapper around the existing body rather than an edit to it: rename
the built function, add a global and a ready flag, freeze on first use.

EXPECT THE COST GOLDEN TO MOVE, and say which way: allocs should fall by about
937,500 of 14,799,465 on the decode workload, with `perm_allocs` rising by the
number of constant definitions the program actually reaches. That is a
deliberate regeneration, not a drift.

## 2026-07-25 — SHIPPED: constants stop being recomputed, and the lazy memo was the trap

Built the CAF freeze the previous entry designed. It works, and the interesting
part is the version that did not.

WHAT SHIPPED. A zero-argument definition whose body is a literal emits its body
under a `_build` symbol; the real symbol becomes a load from a cell that
`@k_caf_init` fills once, before main. `k_caf_freeze` deep-copies the built
value into permanent storage using a zero mark — `k_survives` walks an empty
block chain and answers no for everything, which is what forces a full copy out
of the arena instead of the sharing the carry path wants.

MEASURED, decode gauntlet: allocs 14,799,465 -> 12,924,473 (-1,874,992),
alloc_bytes 690,505,904 -> 595,495,056 (-95 MB), arena_blocks 5 -> 4,
perm_allocs 1 -> 5. Those are exact. Per-decode cpu falls 13.6% on floors and
5.9% on medians over forty interleaved runs; wall clock was unusable, swinging
87% between runs on the same binary at load ~50, which is why the numbers here
are child cpu time from wait4 rather than a stopwatch.

THE TRAP, WORTH MORE THAN THE FEATURE. The first version memoized lazily —
check a ready flag, build on miss — and measured 22% SLOWER by slope despite
allocating 1.87M fewer times. No new hot symbol appeared in the profile; the
cost was diffuse and sat in `d_value_for_3`, the hottest function on the board.
A store on that path is an alias-analysis barrier: llvm can no longer assume
the surrounding loads are unaffected, and the dispatcher it poisoned is the one
every value passes through. Filling before main leaves the read a bare load,
no branch and no store, and the regression became a win.

The general lesson is that a memo check in a hot path can cost more than the
work it skips, and that "fewer allocations" is not the same claim as "faster".

TWO BUGS THE GOLDENS CAUGHT, both mine. `k_caf_freeze` was placed above
`k_alloc_perm` and compiled as an implicit declaration returning int — caught
by the build, not the tests. And `dsym` quotes module-qualified names, so
appending `_build` outside the quotes emitted
`@"d_json/hex_digits_0"_build` and clang rejected it; the example corpus caught
that one as an empty stdout where a diagnostic belonged.

GOLDENS REGENERATED, deliberately and in this direction: both cost goldens
(allocs down, perm_allocs up) and the compile golden (+5 lines and +1 define
per sample — the `@k_caf_init` function, empty in samples with no constants;
rounds and visits unchanged, since inference did not change).

## 2026-07-25 — the boards move for the first real reason today: kanso passes serde

Re-measured every surface after the CAF freeze, and the headline changed.

DECODE, cpu-time slope, thirty interleaved reps, load ~50:

    kanso        0.996 ms/decode   5.6 mb
    serde_json   1.158             6.8 mb
    naive rust   1.381             6.8 mb
    go           2.961            10.8 mb

kanso spends fourteen percent less cpu per decode than serde_json. This
morning the same pair measured a dead heat, and the difference between those
two sittings is the constant freeze, not the weather.

THE INSTRUMENT CHANGED, and the page says so. This morning's board was a
wall-clock slope; this one is cpu time from wait4. At load ~50 a stopwatch is
useless — the same binary timed twice ran 87% apart, which is larger than every
gap on the board. The two boards are therefore not comparable figure for
figure, and the page states the instrument rather than implying continuity.

LAZY BOARD, same sitting, same instrument: kanso 0.12s, rust hand-tuned 0.08,
rust as written 1.73, kanso --strict 2.15. Fifteen times ahead of rust as
naturally written holds; the gap to hand-restructured rust reads 38% under cpu
time where it read 28% under wall, so the page now says forty rather than
thirty.

KQ, byte-identity gated per query: path 4.0/16.4 ms (1.57x/2.17x), pretty
7.5/55.5 ms (2.26x/4.70x). The big-file pretty fell from 194.7 ms this morning
to 55.5 — part CAF, part a quieter machine, and the honest split between those
is unknown, which is why the sitting is dated rather than compared.

SURFACES WALKED, all of them, per the checklist: compiler.html decode board
(now two columns — the median column went, since a slope has no median),
its recipe block, its lazy table and the two prose figures around it;
index.html panel and the two-engines paragraph; about.html prose; kq README
and TRY.md. kanso-json carries no numbers. Every figure on the three site
pages now reads 1.00 / 1.16 / 1.38 / 2.96 / 5.6, checked mechanically.

## 2026-07-25 — SHIPPED (oracle): `&f` partial application on the interpreter

The gavel's first half, built where the differential law says a feature starts.

SURFACE. `&name` is an expression head; `&f a` supplies `a` and waits. The
sigil hugs its name in the canonical spacing table, beside the dot and the
strict-index bang, because it belongs to the name rather than sitting between
two operands. Infix `&` keeps its existing diagnostic about parallel
statements, so nothing was displaced.

THE RULE THAT MATTERS, found by a failing test rather than by design. A
`&`-marked application never dispatches at the count it was written with. The
first cut resolved arities eagerly, so `&roll 4` against a program with both
`fn roll n` and `fn roll n sides` completed at arity one and handed back 5,
and `(&roll 4) 5` then died with "`5` is not callable". That is precisely the
case Clay named when ruling the syntax necessary: with a shorter arm present,
nothing implicit can reach the longer arm's partial. If `&` also completes
early it cannot reach it either, and the sigil buys nothing. So `&f a b`
builds the partial and stops; dispatch fires only when further arguments
arrive and the total meets an arity.

THE CALLEE IS A VALUE, NOT A NAME, which Clay's example forced and which the
design is better for: `fn foo f` returning `&f 2` partially applies a
parameter, so the arity is not knowable where the `&` is written.
`(foo add) 5 7` is 14. `Value::Partial` therefore holds a callee value and the
arguments so far, and applying it appends and re-asks.

ENGINES. Interpreter only. Native and wasm reject it by name — "`&add`
(partial application) is not lowered yet" — which is the escape hatch the
differential law allows and the reason the reject is asserted by a test rather
than assumed.

TESTS, six, in tests/partial.rs: the plain carry, the partial of a parameter,
one that grows through two applications before completing, one that completes
against the arity its count reaches rather than the shortest arm, the
over-application diagnostic naming the arities that exist, and the native
refusal.

OWED: `()` for zero-argument calls, still unparsed. `&f` with nothing supplied
should be an error under the no-superfluous rule and currently is not. Native
and wasm lowering. Inference does not yet type a partial — it reads as TOP —
so the compiler cannot yet check, as Clay put it, "the validity of any
invocation of foo and subsequent invocation of its return value".

## 2026-07-25 — OPEN: what makes `&f a` valid, and why it is a static question

Clay, on the partial that just shipped: "`&roll 4` — that could only fail if
there's no overload of roll that takes as its first argument something that 4
could satisfy."

That is the validity rule, and the shipped slice does not enforce it. Today
`&anything 4` builds a partial and the failure, if any, waits for the
application that completes it. The rule says the error belongs at the `&`,
where the reader wrote it.

WHAT IT REQUIRES. For each arm of the name, ask whether the supplied arguments
could match that arm's leading patterns — literal arms by value, annotated
arms by type, plain binders by anything. If no arm survives that filter, the
partial can never complete and the `&` is an error naming what the arms do
take. This is the same question dispatch already answers at a full call site,
asked against a prefix instead of the whole argument list.

It is also the piece that would let inference type a partial. Right now
`Expr::Partial` reads as TOP, so nothing downstream knows what the value
accepts or returns. Once the surviving arms are known, the partial's type is
the set of their remaining parameter shapes, which is what makes Clay's larger
point checkable: a compiler that can assess `foo` and the invocation of its
return value together, statically, rather than discovering the mismatch when
the last argument lands.

Filed against the `&` task rather than the CAF one; it belongs with native and
wasm lowering and the `()` form as the rest of that feature.

## 2026-07-25 — GAVEL (extension): holes skip positions inside a `&`

Clay: "you should also be able to do `&roll _ 4 _ _ "hello"` for instance.
that's partial application currying but skipping positions."

So `_` earns a place in currying after all, and it is not the place the
withdrawn proposal gave it. `&` still marks the partial; a hole marks a
position left open inside that application. `&roll _ 4 _ _ "hello"` supplies
the second and fifth arguments of a five-argument `roll` and waits for the
first, third and fourth.

WHAT THIS SETTLES. Writing holes names the arity outright — five slots is the
five-argument group — where a bare `&roll 4` leaves arity to resolve as
arguments arrive. Both forms are wanted: the bare one for the common case of
filling from the left, the hole form when the argument you have is not the one
that comes first.

REMAINING ARGUMENTS FILL HOLES LEFT TO RIGHT, which is the same rule the pipe
form already uses, so `_` reads identically in both places: a position without
a name that something later will fill. In a parameter list it still discards.
Three positions, one idea, and the flavour comes from which side of the
definition you are standing on.

UNBUILT. The shipped slice takes `&f a b` only. Holes need the parser to
accept `_` in argument position (currently a syntax error, which is why the
slot was free), the partial to record which positions are open rather than
just a count, and application to fill them in order.

The validity question from the entry above gets sharper with holes, not
harder: a hole constrains nothing, a supplied argument constrains its own
position, and an arm survives if every supplied position could match. The
arity is known outright in the hole form, so only one group needs asking.

## 2026-07-25 — a book output that was never true, and the exemption that hid it

Clay, reading appb: the `args` example claims that running `argv.kso` with no
argument prints

    error[endpoint]: unhandled err reached the executor: "missing index 1"
      born in first at argv.kso:2

It prints `<none>`. `xs[1]` answers an absent index with none, which is the
whole none gavel; `xs[1]!` is the form that ends the run.

HOW IT SURVIVED. The .out carried a sibling .manual, which exempts a golden the
harness cannot replay, and its note read "output reconstructed from a real run
with no argument" — reconstructed, in other words, by hand, from a run that
predates the none model. The exemption was also backwards: the no-argument
invocation is the *default* one and perfectly replayable, while the `-- clay`
variant is the one the harness cannot supply. So the case that could have been
checked was the one marked unbeatable.

Swapped: argv.out is now the no-argument run, replayed and verified on every
book check, and argv_witharg.out carries the exemption with an honest reason.
The prose and the `args` description both said the index "fails, at the
executor, on the same railway as any other err"; both now say it answers with
none and point at `xs[i]!` for the other behaviour.

AND A SECOND GAVEL FOUND UNBUILT. Reading `args → string[]` against the type
gavel — `[]T`, Go's order — I converted the book to `[]string` and the harness
went red on a code panel. Testing the compiler settles it: `fn total xs:int[]`
parses, `fn total xs:[]int` does not, failing canonical spacing at the bracket.
So `[]T` is ratified and unbuilt, exactly like `name()`, and the book was
correctly documenting the compiler rather than the gavel. Reverted.

The lesson is the one the exemption already taught: prose that cannot be
replayed drifts, and a gavel that is only written down is not a fact about the
language. Both are now recorded as unbuilt rather than assumed shipped.

## 2026-07-25 — GAVEL (amendment): the binder stays, and they are constraints rather than generics

Clay, on the type-syntax gavel's no-binder ruling: "of course there's a k
binder. it has nothing to do with order, it's about enforcing that two types
have to be the same."

    type <k>foo
      name:k
      friend_names:k[]

WHY THE ORIGINAL RULING WAS WRONG. It rejected `<k>` as superfluous, on the
grounds that the fields already say what the binder would. The fields say
*order* — which name appears first — and the ruling was right about that much,
so order still comes from first appearance and the binder does not carry it.
What the fields cannot say is that `k` is a *variable*. Without the binder,
`first:k` reads as a constraint only because no type named `k` happens to
exist, so a declaration's meaning depends on the global set of type names, and
adding a type called `k` later silently converts a constraint into a concrete
annotation. That is the same shape as the currying hazard Clay caught earlier
today: a later, unrelated addition quietly changing what existing code means.
The binder states variable-ness outright, which is the one fact nothing else
carries, so it is not superfluous.

THE NAME. They are type CONSTRAINTS, not generics. `k`'s entire content is the
relation it forces: drop it for `any` in both fields and the type still
compiles while the agreement is gone. "Generic" names a mechanism borrowed
from languages that also want variance, bounds and higher-kinded parameters,
none of which kanso is buying, and the word promises them.

The vocabulary now: a binder DECLARES a constraint, fields USE it to force
positions to agree, and `foo[string]` APPLIES it — application being the term
the type gavel already settled. Three words, each doing one job.

A CORRECTION OF MY OWN, from the same conversation. I had argued that the
relating job was gone from kanso entirely and only acceptability remained.
That is wrong, and Clay's example is the counterexample. Relating is gone from
FUNCTION signatures — usage tells you, inference derives it — and it lives on
in TYPE declarations, where nothing else could express it: `k` in `pair` cannot
be inferred from usage, because the declaration is what usage gets checked
against.

Still unbuilt, both halves. `type <k>foo` does not parse, and neither does the
parameterization the gavel reserves.

## 2026-07-25 — CORRECTION: `<k>` generalizes, so it is a parameter and "constraint" stays free

Clay, an hour after the previous gavel: "`<>` isn't even a type 'constraint',
it's the opposite. it is more like a 'generic'. you don't have to name a
specific type, you just generalize to some rule/pattern."

He is right, and the error in the entry above is a baseline error. I compared
`<k>` against writing `name:any, friend_names:any[]` — and against that, `k`
does constrain, because it forces the two to agree. But nobody writes that.
The real alternative is `name:string, friend_names:string[]`, and against that
`k` generalizes: one declaration standing for every element type, where
otherwise you write a second `foo` for every type you need. Generalization is
what it is for; the agreement among occurrences is how it is achieved.

THE PRACTICAL REASON THE WORDS SHOULD NOT BE SWAPPED. A constraint, in the
ordinary sense every reader arrives with, is a *bound on* a parameter — `k`
must be comparable, `k` must render. Kanso has no bounds today. If it ever
grows them, that is what they will be called, and the vocabulary should not
already be spent on the parameter itself.

SO: `<k>` declares a type PARAMETER. `foo[string]` applies it. A future bound
on `k` would be a constraint, and the word is reserved for that.

WHAT SURVIVES FROM THE PREVIOUS ENTRY, unchanged and still argued: the binder
stays. Fields say which positions share a name; they cannot say the name is a
variable rather than a type, and without that a later type called `k` would
silently convert a parameter into a concrete annotation. Order still comes
from first appearance. Relating still lives in type declarations and not in
function signatures.

What changes is only the name, and one entry's worth of reasoning that ran the
comparison against a baseline nobody writes.

## 2026-07-25 — OPEN (not gaveled): `&` merges records, and why the committee's objection weakened

Recording the trail because I argued both sides within an hour and the reasons
matter more than the conclusion.

CLAY'S CASE. A five-field `user`, and a nested function that wants two of those
fields. Rather than invent an ad-hoc type or duplicate the pair, extract the
pair as a type and use it in `user`'s definition. He has done this in
typescript, where `&` makes it easy.

THE COMMITTEE SAID NO FIRST. Hickey: merging lets you avoid naming the thing
two records share, and that name is usually the missing concept. Beck: no case
in the tree today, so prefer duplication until two real ones appear. Bernhardt:
an intersection has no identity — it means whatever those two happened to
contain — and kanso dispatches nominally, so a type's name should mean
something.

WHAT WEAKENED IT. Records carry no behaviour, so the diamond that makes
multiple inheritance ugly largely evaporates: there is no method resolution
order to define, only field names. Two paths contributing the same field at
the same type dedup to one; the same name at different types is a compile
error rather than a policy. The objection was borrowed from languages where
inheritance carries code, and it does not transfer intact.

AND MY COUNTER-PROPOSAL WAS WORSE. I offered record extension through the
existing subtype relation — `type user identity` plus new fields — which the
experiment shows is not what that form means today: a record subtype is a
newtype WRAPPER (`user 30 "clay"` fails with "`user` wraps one identity
value"), not an extension. More importantly extension is single-parent by
nature, and real records have several natural groups — identity, contact,
audit. Clay's case scaled up is exactly what a chain cannot serve.

THE GLYPH OBJECTION IS WITHDRAWN. Clay: `&` is famously both reference and
intersection and is unambiguous here. C++, rust and typescript all carry both
senses without confusion, and the positions are disjoint.

WHAT REMAINS TO SETTLE, and none of it is ruled yet:
  - a `user` must be accepted where `identity` is expected, or the feature
    misses the case; it stays nominal because the composition is declared
  - collision: identical name and type dedups, differing types is an error
  - named results only, matching the typeset gavel — `type foo bar & baz`,
    never a bare `bar & baz` in a parameter position
  - records only; `&` on typesets stays union, mixing rejected
  - variance: whether `[]user` is acceptable as `[]identity`. The diamond's
    absence does not help here. Safest answer is no — containers invariant.

Unbuilt, and behind the parameter and binder work, which is itself unbuilt.

## 2026-07-25 — the unboxing proof was computed and then dropped

Reading the post-CAF decode profile, `d_value_for_3` still dominated, and its
IR carried a tag test and a boxed `k_add` fallback around every `p + 1` — the
position arithmetic the decoder does constantly. Twenty-five `k_add` call sites
survived into the compiled dispatcher, so llvm had not folded them either.

The fast path already existed. `emit_binop` takes an int-only route — the
overflow intrinsic and a trap, no tag test, no call — when `set_of` says both
operands are INT. It was not firing.

The reason is that the proof was computed and then thrown away.
`unboxed_param` returns true exactly when `group_param_set(name, arity, i) ==
INT`; that is the condition for passing the slot as a raw i64 in the first
place. But `rebox_params` reconstructed the KValue without recording the set,
so the binop emitter looked the parameter up and found nothing. One `f.record`
call carries the fact forward.

MEASURED: every `k_add` gone from the dispatcher's IR, and per-decode cpu falls
1.7% on floors, 0.4% on medians, thirty interleaved reps. Small, and strictly
subtractive — the removed calls sat on cold paths llvm had already laid out
well, so the win is instruction count and register pressure rather than
branches taken.

Both cost goldens byte-identical, which is the right answer for a pure codegen
change: allocation behaviour did not move. The compile golden did not move
either, because none of its samples do arithmetic on an unboxed int parameter.
Suite green, browser differential 63 passed 0 failed.

The general shape is worth remembering: an analysis result used for one purpose
(deciding the ABI) was not visible to a second consumer (arithmetic lowering)
that would have benefited from the same fact. Worth asking where else a proven
set is dropped at a boundary.

## 2026-07-25 — eta-reduction, and an honest note that it does not pay here

The encode profile showed `w_klam29` at 47 samples — a boxing wrapper in front
of `klam29`, which musttail-calls `d_esc_byte_2` and ignores its environment
entirely. Two hops of pure indirection. The source is
`list/fold bs acc (a b -> esc_byte a b)`: a textbook eta-expansion, a lambda
that forwards its parameters to a named function and does nothing else.

BUILT. A lambda whose body is a call to a name, passing exactly its own
parameters in order, with the name neither shadowed nor one of the parameters,
now emits the function value instead of a closure.

IT DOES NOT FIRE ON THE CASE THAT MOTIVATED IT. `esc_byte` dispatches on a
literal byte in its second parameter, and a byte discriminator crosses the ABI
as a raw i64 with the 256-is-none convention, which `simple_fn_value` refuses.
So the lambda that costs 47 samples is exactly the one the value ABI cannot
carry — which is *why* a closure was there in the first place. The 47 samples
stay.

MEASURED WHERE IT DOES FIRE: two allocations out of 12,924,473 on the decode
gauntlet, and 48 bytes. A one-time closure saving, not a per-element one. The
cost golden moves by that much and nothing else does.

I nearly shipped a regression on the way. The first version reduced any
forwarding lambda, including `(a b -> push a b)`, and `push` is a builtin with
no function-value form — so a program that compiled before stopped compiling.
The guard is now explicit: one arity, matching, and `simple_fn_value` true.

SO WHY KEEP IT. It is strictly less code emitted wherever it applies, it costs
nothing at runtime, and a forwarding lambda is a common idiom in user code that
the profile just showed carries two hops. But it is not a win on these
benchmarks and the entry says so rather than implying otherwise.

THE REAL TARGET IS NOW NAMED. Making byte-discriminating groups carriable as
function values would collect those 47 samples. That is an ABI change — the
value wrapper would need to unbox the discriminator — and it is the shape
SpecConstr was queued for. Recorded as the next encode-side lead.

## 2026-07-25 — correcting the eta-reduction entry: it pays in memory, not time, and my measurement lied twice

The entry above says the reduction does not fire on the case that motivated it
and moves two allocations. Both halves were measured against a working tree
that had silently lost the commit.

WHAT HAPPENED. After a merge attempt failed I ran `git reset --hard
origin/main` while standing on the feature branch, which reset the branch to
main and discarded the change locally; the commit survived only on the remote.
Every measurement after that point compiled a tree without the optimization,
so "encode counters unchanged" was comparing a build to itself. The tell was
there and I read past it: the golden in the branch disagreed with a fresh
build in the direction of the change, which cannot happen if the change is
absent.

THE REAL NUMBERS, with the commit actually present:

    encode allocs        68,640,508 -> 67,222,108   (-1,418,400, -2.1%)
    encode alloc_bytes    2,288,262,416 -> 2,254,220,816   (-34 MB)
    encode arena_blocks         2,205 -> 2,165
    decode allocs        12,924,473 -> 12,924,471

So it does fire on the encode path, and `esc_byte` is reducible after all —
`simple_fn_value` refuses byte *discriminators*, and esc_byte's literal-byte
arms do not make its parameters unboxed here, so the group qualifies.

AND IT DOES NOT MAKE ENCODE FASTER. Twenty interleaved cpu-time runs: +0.5% on
floors, +0.8% on medians — noise, and if anything the wrong sign. Removing 1.4
million allocations bought no time because an arena allocation is a bump
pointer; the cost was never the allocating. What it buys is 34 MB of
allocation volume and forty fewer arena blocks, which is peak-memory pressure
rather than throughput.

That is worth having and worth stating precisely. The compiler page's memory
claims live on the same footing as its speed claims, and this moves one and
not the other.

## 2026-07-25 — BUILT, MEASURED, DECLINED: eta-reduction is not semantics-preserving here

Reverted. The reason is better than the optimization was.

`(a b -> f a b)` denotes `f` in most languages, so replacing the closure with
the function value looks free. In kanso it is not, because an `err` records a
hop for every function it passes through, and the eta-expanded lambda is a
function. Removing it changes the provenance the trace prints.

The book harness caught it on ch05's welcome sample:

    native:       born in first at welcome.kso:4
                  passed through greet
    interpreter:  born in first at welcome.kso:4

Native and the oracle disagreeing is the one thing the differential law does
not permit, and no amount of speed would buy it. The interpreter does not do
this rewrite, and teaching it to would mean changing the semantics of error
provenance to suit a codegen optimization — the oracle defines the semantics,
not the other way round.

Worth noting which trace is *truer*: the value really does pass through
`greet`, so the native line is arguably the honest one and the lambda was
hiding a real hop. That is a semantics question about what a hop means, and it
belongs to a gavel rather than to an optimization's side effects.

WHAT IT WOULD HAVE BOUGHT, measured with the change actually present:
encode allocs 68,640,508 -> 67,222,108 (-2.1%), alloc_bytes -34 MB,
arena_blocks 2205 -> 2165. And no time at all: +0.5% on floors over twenty
interleaved cpu runs. An arena allocation is a bump pointer, so removing 1.4
million of them buys allocation volume and peak pressure, never throughput.

So the ledger reads: a memory-only win, forbidden by the differential law,
declined. Recorded so the idea stays declined, and so the next person who
notices `w_klam29` in a profile finds this entry instead of rediscovering it.

The encode-side lead that remains is unchanged: those 47 samples are a wrapper
hop into a byte-discriminating group, and collecting them needs the value ABI
to carry such a group — an ABI change, not a rewrite.

## 2026-07-25 — the ordering rule enforces more than its author remembers

Clay, on being told a test program needed its declarations in alphabetical
order: "that sounds like something you made up. record type arguments are the
only thing alphabetical, and i suppose import statements."

It is not made up, and the gap is worth naming. check.rs enforces five
ordering rules: type declarations alphabetical, record fields alphabetical,
typeset members alphabetical and duplicate-free, overloads of one name
adjacent, and *function declaration groups alphabetical*. Constants fall under
the last one, since a constant binding is a zero-arity fn — which is why
`main` sorts in among the functions and why a two-function test program has to
be arranged around it.

The rule Clay recognises is a subset: fields, and imports.

THIS WAS ALREADY FLAGGED. From an earlier entry in this same log: "alphabetical
order scatters cohesion. the sixteen tests sort into the middle of the
implementation, and helper families stay adjacent only because we *named* them
into adjacency (str_char, str_chars, str_escape...). developers will name-game
the ordering rule; that's a signal... the rule deserves a second look with this
evidence in hand."

So the author's instinct now and the evidence recorded then agree, and only the
implementation dissents.

THE ARGUMENT SPLITS CLEANLY. A record is a set of fields, so declaration order
carries no information and sorting it removes a meaningless degree of freedom —
same for typeset members. A file of functions is not a set: order carries
narrative, helpers want to sit beside what they help, and the tree already
shows the workaround, which is naming things into adjacency rather than placing
them there.

Relaxing is backward-compatible, since every sample in the tree already
satisfies the stricter rule; tightening later would not be. Filed as its own
task rather than changed here, because it is a language decision and Clay is
asleep.

## 2026-07-25 — a field rename silently rewires every constructor call

Clay, on the ordering rule: "that makes sense where ordering is meaningless,
like the keys you assign in a map. but perhaps the user really should be able
to decide what order to set keys in a record type?"

There is a sharper reason than taste, and it is a soundness hazard:

    type point            type point
      x:int        ->       y:int          x renamed to z
      y:int                 z:int

    (point 1 2).x = 1     (point 1 2).z = 2

Same call, same types, no diagnostic — the 1 moved to a different field.
Fields sort alphabetically, and field order IS the positional constructor's
argument order, so renaming a field rewires every construction site silently.
It is the same shape as the implicit-currying hazard: an unrelated later edit
quietly changing what existing code means.

Field order is load-bearing twice, in fact. The type-parameter gavel derives
parameter order from first appearance in the fields, so a rename can reorder
those too.

THE CHOICE. Either the author controls field order — declaration order is the
constructor order, renaming is safe, and the documentary grouping Clay wants
comes free — or construction becomes keyed only (`point x:1 y:2`), after which
order carries nothing and sorting it is harmless. The first is less disruptive
and makes a thing that is already semantic explicit rather than derived from
spelling.

It also splits the ordering question cleanly: sort typeset members, because a
union really is a set; do not sort record fields, and do not sort functions.

## 2026-07-25 — DECLINED: go's `map[key]value`

Clay: "we should just adopt go's map syntax map[key_type]value_type instead of
map[key_type value_type]?"

The type gavel's own words are the argument: a type is a name, a slice, or an
application, and there is nothing else. `map[string int]` is `map` applied to
two arguments, exactly like `pair[string int]` or `foo[k]`. Go's form adds a
fourth shape that exists only for `map`, which makes the one type everybody
uses the one type that is special.

That matters more here than in go, and for a reason particular to each
language. Go can afford `map[k]v` precisely because its users cannot write
their own parameterized types with that shape — there is nothing for it to be
inconsistent with. Kanso is heading toward user-defined parameterized types,
and the day someone writes `cache[string int]` beside `map[string]int` the
special case is visible.

Borrowing `[]T` from go does not set a precedent for it: `[]T` is one of the
three forms, not a carve-out for a particular type name.

The honest counter is familiarity — `map[string]int` reads instantly to a go
programmer and `map[string int]` costs a beat. One beat, spent once, against a
special case carried forever.

## 2026-07-25 — correcting the map-syntax entry: the argument is arity, not inconsistency

I read Clay's proposal as `map[k]v` for `map` alongside the application form
for everything else, and argued the carve-out would look odd next to
`cache[string int]`. He meant `map[k]v` as *the* form. The entry above answers
a question he did not ask.

THE ARGUMENT THAT SURVIVES IS SIMPLER AND STRONGER: the form does not
generalize, and go proved it. `map[K]V` is builtin-only syntax that predates
generics; when go added user-defined parameterized types in 1.18 it reached for
`Pair[K, V]`. Go carries both, and the one it chose for the general case is the
comma form — which is what kanso already has, minus the commas.

A two-parameter type reads well as `map[k]v`. A three-parameter one has nowhere
to go. Taking it as the general form means taking a form that works at arity
two and needing a second form the moment something takes three.

CLAY'S SHARPER QUESTION, which the entry above missed entirely: maybe
multi-argument application is not needed at all.

    type <t>pair:
      first:t
      second:t

That is a homogeneous pair — one parameter, both fields the same type — and it
applies as `pair[string]`. If most parameterized types take one parameter, the
multi-argument form earns its keep on very few cases, and `map` might be the
only genuine two-parameter type.

I do not think it is. A cache keyed by one type holding another, a result
carrying ok and err types, anything dictionary-shaped: those want two, and they
are the same shape as `map` rather than special cases of it. Which is the
uniformity argument stated properly — `map` is not privileged, it is the
two-parameter type that happened to ship first.

## 2026-07-25 — the tree is rustfmt-clean, and the check gates

Mechanical, and kept apart from logic on purpose: 28 files, 727 insertions,
899 deletions, zero behaviour change. Both cost goldens byte-identical, 15
suites green, clippy clean at --all-targets.

rustfmt.toml carries two settings rather than the defaults: max_width 100 and
use_small_heuristics "Max". Defaults would have moved 6099 lines instead of
3419, mostly by exploding struct-variant declarations and short call chains
that the house style keeps on one line. The settings were chosen to match what
the tree already looked like, not to impose a new shape.

CI gains a `format` job — its own check, so a red run says "format" rather than
burying it inside lint. That makes twelve jobs; the required-contexts list
needs the new name or the check runs without gating, which is the trap the
job-rename hit earlier today.

On the wider linter question: clippy already runs --all-targets with -D
warnings as of this morning, and the perf group is clean. pedantic is 442
warnings, which is not worth forcing wholesale — the useful move is enabling
individual pedantic lints as they come clean, rather than a blanket allow list
that nobody revisits.

## 2026-07-25 — RC, step one: the leak is now pinned instead of described

Clay asked for value-level reference counting next. Before touching allocation
behaviour, established where the current scheme actually stands, because the
last entry on it is stale in both directions.

WHAT IS ALREADY BUILT, contrary to the earlier audit: `k_thunk_new` reuses from
the free list and retains nested cells (`->rc++` on a K_THUNK argument);
`k_thunk_release_cell` decrements, drops args recursively, recycles, and counts
`thunk_frees`; and codegen does emit drops — `k_thunk_release_unless` at a
releasable binding's frame epilogue, which returns the result untouched when
the result IS the cell. So retain, release, recycle and drop-insertion all
exist.

WHAT DOES NOT HAPPEN, measured on the flagship lazy workload:

    lazybench: thunk_allocs=100000  frees=0  escaped=100000  live_exit=100000

Every cell escapes and none is recycled. The release site cannot fire because
a cell handed onward in a tail call outlives the frame whose epilogue would
have released it — which the runtime comment already says, and counts rather
than frees. The cells are malloc'd while the structures referencing them live
in the arena, so a rewind drops the references without releasing the cells.

THE HARNESS HOLE THAT LET THIS SIT. Every .mem golden holds at most one cell:
seven of the eight have thunk_allocs of 0 or 1, and the vein's freeing cases
(force_path, skip_unused, skipped_err) all free exactly one. Nothing pinned the
accumulating shape, so "cells are recycled" was true of every case under test
and false of the case that matters.

tests/golden/mem/many_cells.kso closes it: forty cells in a tail-recursive
walk, five demanded, and the golden records allocs=40, frees=0, escaped=40,
live_exit=40. It fails the day any of those move, which is the point — the
number to beat is written down before the work starts rather than after.

NEXT, and deliberately not attempted tonight: the release site needs to survive
a cell outliving its frame, which means the count has to be owned by whatever
holds the cell rather than by the frame that made it. That is the retain-on-
store half of a real RC, and it is the piece the log has twice flagged as
memory-unsafe to rush. The golden now exists to prove it when it lands.

## 2026-07-25 — `<>` for application is viable: spacing already disambiguates it

I argued against `pair<string>` on the turbofish precedent — that `<` is a
comparison and type names appear in expression position as constructors, so
`pair<string>` would be ambiguous the way `foo<bar>(baz)` is in c++. Clay: "i
don't think we have ambiguity because space is meaningful."

He is right, and the compiler settles it. Both collisions are already
formatting errors:

    a<b            error[formatting]: canonical form requires exactly one space
    "a">>print     error[formatting]: canonical form requires exactly one space

Comparisons must be spaced, so a TIGHT `<` cannot be one. The sequence operator
must be spaced, so a tight `>>` cannot be one either — which means
`foo<bar<t>>` nests without the problem c++ needed a language revision to fix.

The precedent does not transfer because the languages differ where it counts:
rust and c++ are whitespace-insensitive, and that is precisely why they needed
disambiguation. Kanso already spends spacing as grammar in three places — the
dot for field access against the pipe, the bang against its bracket, and now
`&` against its name — so a fourth costs nothing new to learn.

WHICH LEAVES THE CHOICE ON TASTE, and the taste argument now runs the other
way from the gavel. The binder is `<k>`; applying with `pair[string]` uses a
different bracket for the same concept, while `pair<string>` uses the same one
for declaring and supplying. Against that, `[]` is already the type-and-
collection bracket (`[]T`, `xs[i]`), so `<>` would be a second family.

Not re-gaveled — recorded because the reason the gavel gave for `[]` was never
ambiguity, and the reason I gave against `<>` was wrong.

ALSO SETTLED IN PASSING: Clay, on inference supplying type arguments — "you
could have ambiguous cases. i guess that would just be a compiler error." That
matches how every other ambiguity in the language is ruled: competing arities
under currying, and a merged type's field collisions. Ambiguity is an error
rather than a default, and the rule is now consistent across three features.

## 2026-07-25 — micro-specs: one construct per program

Clay: "you probably want more specs based on micro code snippets that exercise
a particular language feature in an isolated way."

The corpus was 46 examples touching eleven to eighteen constructs apiece, so a
failure told you something broke, not which construct. Worse, coverage was
accidental: today's `dsym` quoting bug — a `_build` suffix landing outside the
quotes of a module-qualified symbol — was caught only because
`examples/json_failure_door.kso` happens to use a module-qualified constant.
Nothing was aiming at that shape.

tests/golden/micro holds ten programs, one construct each: dispatch on literal
ints, one name at two arities, the pipe's trailing arguments, a return guard,
an absent index answering none, the strict index, interpolation, a subtype
dispatching against its base, a binding the taken arm never computes, and a
module-qualified name — the shape that broke.

They feed three harnesses. tests/golden.rs runs each on both engines and
requires identical stdout, which is the differential law at its smallest useful
size. The browser differential picked them up by adding one directory to its
corpus, taking it from 63 programs to 73, all passing.

The smallest one earns its place best. `module_qualified.kso` is four lines and
would have named this morning's bug directly instead of leaving it to an
unrelated example's incidental imports.

## 2026-07-25 — the two renderings of kanso on one site now agree

Clay: the highlighter "has a different look for types and strings for
instance." Two real discrepancies, both between the tokenizer in
kanso-engine.js and the hand-marked panels the site's prose pages carry.

STRINGS. The site nests the interpolation inside the string and colours the
braces with it:

    <span class="s">"failed: <span class="i">{reason}</span>"</span>

The tokenizer emitted three siblings instead — a closed `s`, then `{` as an
operator, the name as `i`, `}` as an operator, then a fresh `s`. On screen the
literal came apart into pieces and the braces took the operator colour. It now
emits the site's shape: one `s` wrapping, with the braces inside the `i`.

TYPES. `fn` set a flag so the next name renders as a function; `type` set
nothing, so a declared type name fell through to plain text while the site
marks it `t`. A declared type is a type for the same reason a declared function
is a function.

Verified by rendering three snippets in headless chrome and comparing against
the markup in the pages, rather than by eye.

## 2026-07-25 — GAVEL: `&` merges named bundles only

Clay: "yes named bundles only."

    type user identity & passwordable

    NOT:
    type user identity
      email:string
      role:string

A merged type is exactly the union of named parts. No inline fields alongside a
parent, which is the shape that would let you skip naming the thing you are
adding.

WHY THIS IS THE STRONGER FORM. The committee's objection to merging was
Hickey's: it lets you avoid naming the concept two records share, and that name
is usually the missing piece. Requiring named bundles answers the objection in
the syntax rather than by discipline — you cannot express the unnamed case, so
the concept always gets a name. The email-and-role pair becomes a type with a
name someone had to choose, which is the outcome the objection wanted.

`&` is also what distinguishes a merge from a typeset, which the bare form
already takes: `type baz bar foo` compiles today and means "bar or foo". Merge
and union are opposites and would otherwise be spelled identically.

STILL OPEN, unchanged from the earlier entry: whether a `user` is accepted
where an `identity` is expected (it must be, or the feature misses its case);
the collision rule (identical dedups, differing is an error); records only,
with `&` on typesets staying union; and variance, where the safe answer is that
containers are invariant.

## 2026-07-25 — what the function-ordering rule actually enforces

Demonstrated rather than described, since the rule is what Clay did not
recognise as his:

    fn zebra … then fn alpha        error: `alpha` before `zebra`
    fn f … fn g … fn f              error: `f` before `g`  (overloads split)
    fn bump … pub play … fn zap     accepted (b < p < z)

So: every top-level declaration name must appear in alphabetical order, and
overloads of one name must be adjacent. Constants are not exempt — a constant
binding is a zero-arity fn, so `main` and `play` sort among the functions,
which is why a two-function test program has to be arranged around its own
entry point.

That last consequence is the one worth weighing. An entry point is the thing a
reader looks for first, and the rule can place it in the middle of the file.

## 2026-07-25 — GAVEL, IMPLEMENTED: declaration order is the author's

Clay, for what he notes is not the first time: "declarations don't have to be
alphabetical. and overloads don't need to be adjacent. it's fine for typeset
members, yes. it's good for keys in a map assignment too."

REMOVED: type declarations alphabetical, record fields alphabetical, function
declaration groups alphabetical, and the requirement that overloads of one name
be adjacent.

KEPT: typeset members alphabetical and duplicate-free, keyed reads listing
fields alphabetically, imports alphabetical, and types-before-functions. Each
of those is a place where order genuinely carries nothing — a union is a set, a
keyed read names fields rather than positions, imports are a set — which is the
line Clay drew.

WHAT IT FIXES BEYOND TASTE. Field order is the positional constructor's
argument order, so alphabetical fields meant renaming a field silently rewired
every construction site. Now `type point` with `y` first makes `(point 1 2).y`
equal 1, and a rename moves nothing. The hazard recorded this afternoon is
closed by the same change that answers the complaint.

WHAT THE CHANGE SURFACED, which is the argument for goldens. Removing the rules
turned four book samples green that had been pinned as failures, and one of
them was hiding a second bug: `docs/book/samples/ch08/positions.kso` is
described by its chapter as a sample that runs, and its golden had been pinning
a compile error the whole time. Under the ordering diagnostic sat a naming one
— `is_ws` needed to be `is_ws?`. Fixed, and the sample now prints what the
chapter promises: ok at 42, then two failure positions.

Four obsolete samples deleted with their prose (appa and appc each carried a
field-order and a declaration-order demonstration, ch07 a third), two goldens
regenerated where a secondary diagnostic disappeared, and four passages
rewritten — ch01, appa, and appc twice — that described the rules as they were.

15 suites green, book verified, browser differential 73 passed 0 failed.

## 2026-07-25 — the technique ledger reconciled against the counters

Clay: "you have a multitude of optimizations discussed in the compiler page and
i don't know if they're current with what's already shipped or discarded or
still pending."

Audited every entry against evidence rather than memory. Three claims of
"shipped" all hold, each with a live counter in a CI-diffed golden: ryū at
ryu_renders=849200 on the encode board, eisel–lemire at el_parses=318450 on the
decode board, vectorized utf-8 at utf8_bytes on both. Constants-not-recomputed
is shipped as of today, with perm_allocs=5 in both goldens.

TWO DEFECTS FOUND.

Numbering: adding the constants entry left two items numbered 7. Maximal
sharing becomes 8 and the cheap experiments 9.

TRMC was listed as queued, and the log closed it on 2026-07-23: "TRMC's regime
is cons-cell construction; kanso's flat arrays with frontier push already sit
at its endpoint." Read and closed, not built and not pending. It now reads
"already won" and the entry says why, so the next person does not spend the
slot rediscovering it.

The ledger now reads: three kernels shipped, one measured and declined
(eytzinger), one already won (TRMC), one shipped today (constants), and three
genuinely open — in-place as a guarantee, maximal sharing, and the cheap
experiments.

## 2026-07-25 — MEASURED, DECLINED: profile-guided optimization

The first of the cheap experiments, and it does not pay here.

Instrumented the decode gauntlet with -fprofile-instr-generate, replayed it,
merged with llvm-profdata, and rebuilt with -fprofile-instr-use. Sixty
interleaved cpu-time runs against the same binary built without it:

    floor    137.7 -> 136.3 ms   (-1.1%)
    p25      167.1 -> 164.8      (-1.4%)
    median   173.0 -> 172.0      (-0.6%)

All three agree on direction and none of them is larger than the noise on this
box. An earlier thirty-run pass had the floor going the other way (+4.6%) with
the median improving, which is the shape of a measurement that has not
converged; the sixty-run numbers are what the entry rests on.

WHY IT SHOULD HAVE BEEN EXPECTED. Pgo pays where branch prediction is losing —
long if-else chains, virtual dispatch, cold paths inlined into hot ones. The
decoder's dispatch is already a jump table, the arms are already inlined, and
the inner loops are already vectorized. There is not much left for a profile to
tell the compiler that the shape of the code does not.

THE COST IS NOT ZERO. Two-pass builds, a profile artifact checked in and going
stale, and a compile story that currently fits in one pass and was published
this morning as such. A percent does not buy that.

Recorded on the compiler page beside the technique so the idea stays declined,
and so the next person reaching for the obvious free win finds the numbers.

## 2026-07-25 — GAVEL, IMPLEMENTED: a field is written by assignment

Clay: "i think it would read better to just use assignment in the build block.
that's the one place you can do mutation."

    pair = build
      a = node "a" none
      b = node "b" a
      a.next = b
      [a b]

WHAT MOVED. `a.next = b` parses as the field write; `set a next b` is gone, and
the name `set` returns to being an ordinary identifier. The old form was
application-shaped so that the name stayed free elsewhere, which was a real
reason, but it left the language reading a field one way and writing it
another: `a.next` to read, `set a next` to write. Two spellings for one idea,
which the no-superfluous rule does not allow once a single one exists.

THE ASYMMETRY IS WHAT PROMPTED IT. Dot field access shipped after `set` was
designed (#194-#196), and that is what made the mismatch visible — before it,
there was no dotted read for the write to disagree with.

WHAT THE CHECKER STILL ENFORCES, and had to be taught the new shape. A write
outside a `build` block is rejected: the old rejection rode on `set` being an
unknown name, which an assignment has no way to trigger, so an assignment
outside a block reached the interpreter and hit an unreachable. It now
diagnoses at check time — "`a.next = ...` writes a field, and only a `build`
block may do that" — and the block-born rule reports in the same vocabulary.

MIGRATED: five .kso files across examples, the mem golden, the runtime and
error corpora, and the compile-cost sample. Three error goldens regenerated,
one of which now tests the assignment form's own diagnostic rather than the
unknown-name path it used to ride.

15 suites, the book, and 73 browser-differential programs all green.

## 2026-07-26 — the fourteen percent was the load, not the lead

The board published yesterday claimed kanso spends fourteen percent less cpu
per decode than serde_json. That measurement was taken at load average fifty.
On a quiet box — load under five, the sitting the board had owed since the
morning — both instruments agree the lead is about four percent:

    cpu slope    kanso 0.832   serde 0.872   naive 1.004   go 1.922
    wall slope   kanso 0.843   serde 0.867   naive 1.004   go 1.745

kanso/serde is 4.6% by cpu and 2.8% by wall. The fourteen was an artifact of
measuring two programs while a browser held three cores: they do not degrade
alike under contention, and the gap between them widened by more than the gap
itself. Corrected everywhere — the board, the recipe, the landing panel, the
about prose.

WHICH INSTRUMENT, now that the box is quiet and they can be compared. Clay
prefers cpu time: "precise, even if less accurate by some holistic wall clock
measurement." That is the right call and the numbers show why — cpu counts only
what the process spent, so a passing background task cannot inflate it, while
wall clock swung 87% between runs of the same binary yesterday.

It carries one caveat worth stating on the page rather than burying: cpu time
bills every thread. Go's collector runs on other cores, so go costs 1.92 by cpu
and 1.75 by wall, and the difference is real work rather than measurement
error. A single-threaded runtime compared against a parallel one by cpu time is
being flattered. The page now says so.

THE LESSON IS ABOUT WHEN, NOT WHAT. Yesterday's numbers were honestly measured
with a defensible instrument and were still wrong, because the sitting was
wrong. A loaded box can be raced fairly — every decoder faces the same weather —
but only for a ratio between programs that degrade alike, which two parsers
with different allocation behaviour do not. Idle floors are not a footnote to
refresh when convenient; they are the measurement.

## 2026-07-26 — MEASURED: arguments are eager today, and a self-referential constant now says so

Clay pushed on a claim I made about `fib`: I said recursive `fib 22` allocates
zero thunks because strictness analysis proves the arguments demanded, and he
answered that nothing is demanded until something asks for `fib 10`. He is
right, and the correction is larger than the wording.

STRICTNESS IS CONDITIONAL. A function is strict when a bottom argument makes
its result bottom — "if the result is demanded, the argument is." It never
claims the function runs. Demand originates at the effect boundary; `print`
asks for its argument, and strictness carries that inward, so under
`print "{fib 22}"` the whole tree is demanded and no thunk is warranted.

BUT THAT IS NOT WHY FIB HAD ZERO THUNKS. Measured, with a discarded argument:

    fn second _ b
      b

    pub play = print "{second (fib N) "done"}"

Nothing demands the first argument. It is computed anyway, and the cost tracks
fib's growth — control 5.3ms, N=28 6.8ms, N=30 8.9ms, N=32 14.6ms (best of 5,
CPU). The interpreter agrees and is merely slower (control 1.9ms, N=28 302.8ms,
N=30 794.1ms). Both engines are call-by-value for function arguments. Thunks
exist for the structured lazy constructs — the scoreboard allocates 100,000 —
but an argument is not one of them. So `fib` allocated no thunks because
nothing was going to thunk, not because an analysis proved anything. The
engines agree, so the differential law is intact; the gap is between the
ratified pervasive-laziness design and what is built, which is the
structured side of the open pervasive-vs-structured gavel.

WHAT THIS MEANS FOR CYCLES, which is what Clay was actually asking. Recursive
`fib` cannot make one: it returns integers, and the recursion lives in the call
graph, which is a stack that unwinds rather than a heap graph that points back.
Under call-by-value that holds for every recursive function whose result is a
value. The cycle question arrives with pervasive laziness, where a thunk
captures its free variables and one of them can be the binding the thunk is
computing — which is what confining cycles to `build` blocks is for, and what
keeps reference counting complete.

SHIPPED ALONGSIDE. The knot idiom was banned by crashing rather than by a
checker. `x = x`, mutual `a = b` / `b = a`, and `ones = push ones 1` all
recursed until the stack ended: the interpreter printed `stack overflow`, and
the native binary took SIGSEGV, which the driver reported as a bare exit 1 with
no output at all. A constant's references to other constants are a graph the
front end can walk, so it does; a constant that reaches itself is now
`error[name]: `x` is defined in terms of itself, so it has no value`, on both
engines, exit 2, pinned by three error goldens.

STILL OPEN. A cycle routed through a function body — `x = bump 1` with
`fn bump n` returning `x + n` — is not caught, and should not be by this check:
following calls would flag guarded recursion that terminates. That case is
ordinary non-termination, which no language promises to catch. What is worth
fixing is the reporting: the interpreter names the stack overflow and native
dies silently. Same program, two diagnostics, one of them empty.

## 2026-07-26 — MEASURED: recursion depth diverges between the engines, and native died silently

Following the constant-cycle work, the same afternoon's question — what happens
when recursion does not terminate — turned up a divergence that predates any of
today's changes.

MEASURED, `fn countdown n` returning `countdown (n - 1)` past a base case:

    depth        interpreter   native
    10,000       ok            ok
    50,000       ok            ok
    100,000      DIED          ok
    1,000,000    DIED          ok
    20,000,000   DIED          ok

The interpreter's edge is between 53,125 and 54,687, bisected. Native does not
have one, because the recursive call is in tail position and LLVM turns it into
a jump. That is an accident of the backend, not a promise the language makes.

Non-tail recursion, `n + total (n - 1)`, shows the other half:

    depth        interpreter   native
    50,000       DIED          ok
    100,000      DIED          ok
    500,000      DIED          DIED

So there is a band — roughly 50,000 to 400,000 for non-tail shapes, and every
depth for tail shapes — where a program runs natively and dies in the
interpreter. The oracle is the engine that fails first, which is the wrong way
round: the interpreter defines the semantics, so it should be the one that can
express whatever native can run.

TWO QUESTIONS FOR THE GAVEL, neither of which this entry decides.

  - Are proper tail calls a promise? Native has them by accident today. Making
    them a guarantee means the interpreter needs a trampoline, and it makes the
    shape of a recursive definition semantically load-bearing.
  - Is there a stated depth limit both engines enforce? A shared limit turns
    stack exhaustion into a diagnostic instead of a crash, at the cost of a
    counter on every call in the hot path.

SHIPPED, because it settles no semantics: the driver reports the death. A
program the operating system kills carries no exit code, and the driver passed
that through as a bare `ExitCode::FAILURE` with nothing written to stderr at
all — a 500,000-deep recursion produced exit 1 and total silence. It now names
the cause: `error[runtime]: the program ran out of stack: recursion went deeper
than the stack holds`, with the signal number for anything that is not SIGSEGV.
Pinned by a spec that was watched failing against the old path (empty stderr).

## 2026-07-26 — a `build` block after a guard was read as a bare name

Writing the containment example Clay asked for turned up a parser hole. This
fails:

    fn spin n acc
      return acc if (n < 1)
      pair = build
        ...

with `error[name]: unknown name `build``, while the same block without the
guard line above it parses. The cause is in `parse_body`: when the leading run
contains a `return`, the fold that wraps the guards walked the lead lines one
at a time and called `parse_stmt` on each. A block header owns the indented
lines beneath it, and a per-line parse cannot see them, so `pair = build`
became a binding of the identifier `build`. The no-guard path never had the bug
because it hands the whole lead run to `parse_lead_stmts`, which groups
correctly.

The lead run is now grouped into units — a line plus whatever it owns — at the
point where `lead_end` is computed, and the guard fold walks units instead of
lines, handing each non-return unit to the same `parse_lead_stmts` the other
path uses. One grouping rule, two callers.

MEASURED WHILE THERE, because the example needed to claim something true.
Two-node cycles built inside a loop, each passed through a function that walks
it and then discarded — two thousand in the shipped example, because the
interpreter's debug build overflows its stack past about five thousand frames
and CI runs the corpus in debug:

    n=2000    allocs=16027   arena_blocks=1  beat_iters=2000
    n=20000   allocs=160027  arena_blocks=1  beat_iters=20000

Flat, which is the claim the example makes: a cycle crosses call boundaries
like any other argument, and the cohort dies with its iteration.

ONE CLIFF FOUND, not yet chased. The same loop carrying a *string* accumulator
instead of an integer does not beat at all:

    int carry:     arena_blocks=1   beat_iters=200000
    string carry:  arena_blocks=37  beat_iters=0

Same shape, same discards; the carried type decides whether the rewind
happens. Carry evacuation is supposed to copy exactly this across the
boundary, so either the analysis is refusing a case it could take or the
evacuation cost is judged too high somewhere. Recorded as an open thread.

## 2026-07-26 — the ledger claimed a stack check that does not exist

The techniques ledger read: "recursion without stack overflow — self-tail calls
compile to loops; deep non-tail recursion is bounded by an explicit check, not
a segfault." Half of that is true. Self-tail calls do compile to loops, which
today's measurement confirms at twenty million frames. There is no explicit
check anywhere: native takes SIGSEGV at roughly five hundred thousand non-tail
frames and the interpreter aborts at about fifty-four thousand. The line
promised a mechanism the compiler does not have.

Corrected to say what is built — constant-stack self-tail calls, measured — and
to name the gap plainly: non-tail recursion still ends the stack, the engines
end it at different depths, and a bound they share is queued rather than
shipped. The driver naming the cause is the only part of that which landed.

The correction matters beyond accuracy. A reader deciding whether kanso is safe
for a deeply recursive workload would have taken "bounded by an explicit check"
as a guarantee and written code against it.

SHIPPED WITH IT: the playground gains the containment example as
`build: a cycle you can pass around`, so the claim the compiler page makes
about cohorts is something a reader can run rather than read.

## 2026-07-26 — the playground repl could not submit, and nothing watched it

Clay reported the repl strip taking no input. The handler opened with
`if (!wasm) return;`, and `wasm` is not a name play.js has. It was left behind
when the engine moved into kanso-engine.js, which exposes the module through
`window.KansoEngine` rather than a file-level binding. play.js runs under
`'use strict'`, so the bare identifier did not read as undefined — it threw a
ReferenceError, and the throw aborted the handler before a single line was
echoed. Every submit did nothing, silently.

The gate was the wrong shape anyway. `runSource` awaits `ready()` itself, which
is why the run button survived the same refactor; the repl calls `callKanso`
directly and had no load of its own to wait on. It now awaits `ready()` in the
same place, so it works whether or not a megabyte of wasm has arrived.

WHY NOTHING CAUGHT IT. The browser smoke test drives the run button and the
example picker, and the playground spec runs every sample on every engine —
none of them touches the repl, which is the one surface that reaches into the
module directly rather than through `runSource`. That is exactly the shape a
load-order slip breaks in isolation. `scripts/site_smoke.py` now submits
`2 + 3` and waits for the answer's own line, and the check was watched failing
against the old handler before the fix went in.

## 2026-07-26 — the run button and the repl broke the same way: a half-cached pair

Clay reported the run button dead after the repl. Driven headlessly against
current main, and then against the live assets fetched from kanso-lang.dev,
both work: editing the source and clicking run prints the new program, and so
does ⌘⏎. The breakage is not in the code that is deployed.

It is in how the code is deployed. The playground loads `kanso-engine.js` and
`play.js` as two separate assets that must agree, at `max-age=600`. Every time
the pair changes together — and the repl fix changed exactly that seam — there
is a ten-minute window where a browser can hold one file from before the deploy
and fetch the other from after. A `play.js` that destructures names the engine
did not export yet, or the reverse, fails at load, and everything downstream of
the failed line is dead. That is both symptoms from one cause, which is why
they arrived one after the other.

The pair is now stamped: the script tags carry the build time, and the version
on the engine's own tag travels to its `kanso.wasm` fetch, so the module cannot
be half a deploy behind either. `document.currentScript` reads only while the
file is executing, so it is captured at load rather than at fetch.

THE TEST HOLE, which is the part worth keeping. The smoke test drove the run
button only through the example picker, and switching examples runs its own
code — so the picker path passed while a plain edit-then-run could be broken.
It now types a program into the editor and presses the button, then does it
again through ⌘⏎, which is a separate listener. Both probes were watched
failing against a stubbed click handler before the change went in.
## 2026-07-26 — SHIPPED: the utf-8 validator was paying vector setup on five-byte keys

Started the ledger sweep with a profile rather than a guess: four thousand
decodes of the 188 kb benchmark, sampled five seconds, 1,246 samples. The flat
leaf distribution named a surprise.

    d_value_for_3   247  19.8%   the value dispatcher
    k_utf8_bad      189  15.2%   utf-8 validation
    k_b_push_mut    102   8.2%
    _platform_memmove 77  6.2%
    ...
    k_b_to_float     22   1.8%   float parsing

Fifteen percent in utf-8 validation, for a kernel the ledger describes as fully
vectorized — and it is; the keiser & lemire structure is all there and correct.
The cost is that it runs at all on short input. `nblocks = (len + 15) / 16 + 1`
means a five-byte object key runs two blocks, and both take the tail path that
fills a sixteen-byte buffer one byte at a time, after loading three lookup
tables. A json document is mostly short keys and short strings, so the setup is
the work.

THE FIX is four lines: below sixteen bytes, scan for a byte with the high bit
set, and return valid if there is none. Ascii is valid utf-8 by definition, so
the early return needs no other condition, and anything non-ascii falls through
to the wide path unchanged.

MEASURED, interleaved, best of nine, cpu:

    baseline    0.8443 ms/decode
    fast path   0.7525 ms/decode      -10.9%

And on the published board, slope-timed (450-run floor minus 150-run floor over
the extra 300, which cancels startup and the file read for all four alike):

    kanso   0.7559 ms   5.6 mb
    serde   0.8621 ms   6.7 mb
    naive   0.9949 ms   6.9 mb
    go      2.0007 ms  10.5 mb        (1.6888 ms wall)

The lead over serde moves from 4.6% to 12.3%. Peak footprint does not move.

THE HARNESS CAME FIRST, per the standing rule, and it is now checked in as
`scripts/utf8_differential.py` with a CI job. It extracts the validator's real
text out of runtime.c — never a copy, so it cannot drift — rewrites the two
returns into a bool, and compares it against a scalar reference written
straight from the rfc 3629 grammar. Exhaustive over every string of three bytes
or fewer, then twenty million sampled strings across the lengths that straddle
the vector boundary: 36,843,009 cases, 0 mismatches. The harness was watched
failing first — loosening the fast path's threshold from 0x80 to 0xC0 produces
10,153,871 mismatches — so the zero means something.

WHAT THE PROFILE SAYS ABOUT THE REST OF THE QUEUE. Float parsing is 1.8% of
decode, so the queued eisel-lemire mirror and dragonbox cannot pay much here;
they stay queued behind a workload that is actually float-heavy. The dispatcher
at 19.8% is the largest single target left, and it is what call-pattern
specialization exists to attack. Buffer copying — push_mut plus memmove — is
another 14.4%, which is the in-place and TRMC territory.

## 2026-07-26 — OPEN: a record field's declared type is not enforced anywhere

Clay asked for the knot examples to say `partner:person` rather than
`partner:any`, on the grounds that a build block should defer type checking
rather than require the escape hatch. The examples now say it, and they run —
but testing why they run turned up that the declared type is not checked at
all.

    type person
      name:string
      partner:person

    ada = person "ada" none      accepted
    ada.partner = 42             accepted
    print "{odd.partner}"        prints 42

No check at construction, none on assignment, and none when the block closes. A
`none` left in a `person` field survives to the outside and prints as `<none>`.
The declared type is decoration on this path.

CLAY'S PROPOSED RULE, recorded for the gavel rather than built. Construction
inside a build block goes unchecked, so a field may hold a provisional value
while the knot is being tied; assignment is checked, because by the time you
assign the real value it exists; and when the block closes, every value it
created is checked, so nothing provisional escapes. That is coherent, and it
needs no new syntax: `none` is already the provisional marker, and the exit
check is what makes writing it safe. It also restores the invariant the
schema criterion asks for — outside a build block, an invalid state is
unrepresentable.

THE SECOND HALF HE RAISED — whether a value can be passed to a function or a
lambda before it leaves the block — is the same invariant seen from the other
side. While a record is provisional its declared type is a lie, and a function
taking a `person` would receive one whose `partner` is `none`. A blanket ban on
calls inside a block is broader than the problem: the restriction that matches
it is on values *created in this block*, which may be assigned and read but not
handed to anything until the block closes. Values from outside stay ordinary,
and the constructor itself has to remain callable or nothing can be built.

Both halves wait on Clay.
## 2026-07-26 — asset digests, because a query stamp only narrows the window

Clay asked for the rails technique rather than the build-time query that went
in with the run-button fix, and he is right that it is the stronger form. A
query narrows the window in which a browser can hold `play.js` from before a
deploy and `kanso-engine.js` from after; a digest in the filename closes it,
because the two names cannot both resolve unless they were built together. It
also stops charging visitors for deploys that did not touch the assets: an
unchanged file keeps its name and stays cached.

`scripts/fingerprint.py` runs against the built site. The ordering is the part
that needed care — the engine fetches the module by name, so `kanso.wasm` is
digested first, the engine's reference to it is rewritten, and only then is the
engine hashed. Hashing the engine first would have shipped a digest that names
a stale module. Then every html and js reference is rewritten, query and all.

    style.css        -> style-ece04f22ef7b7087.css
    kanso.wasm       -> kanso-88205d38323c7772.wasm
    play.js          -> play-699a4fde906a7972.js
    landing-play.js  -> landing-play-52e4851252adbf40.js
    kanso-engine.js  -> kanso-engine-e6854120e4e81928.js

Verified by running the digested site in a browser rather than by reading the
rewrites: the page loads the digested engine, the engine fetches the digested
module, and editing the source and pressing run prints the new program, with
no console errors.

THE BUILD MOVES to `.github/workflows/pages.yml`, because github's own jekyll
build has no step where a digest could be taken. The workflow builds with the
same `jekyll-build-pages` action, digests, asserts that no undigested reference
survived, and deploys. Ci gains the same assertion so the fingerprinting cannot
rot between deploys.

The query stamp is removed in the same change; keeping both would leave two
mechanisms for one job, and the weaker one would be the one nobody maintains.

## 2026-07-26 — CORRECTION and FIX: assignment was the only unchecked field write

An earlier entry today said a record field's declared type is enforced
"precisely never." That was wrong, and the correction narrows the defect to
something worth fixing rather than something to despair at. Measured at each
site:

    person "a string" 42      into name:int      rejected, all engines
    person "ada" none         into partner:int   accepted  (none is a value)
    ada.partner = "a string"  into partner:int   ACCEPTED  <- the hole
    p.partner = ... outside a build block        rejected as a build violation

Construction is checked inside a build block exactly as it is outside; the
constructor check simply never learned about the other way a field gets
written. So the promise held until the knot was tied and then stopped holding,
which is the worst of the two options.

`check_set_literals` closes it on the same terms the constructor check uses: a
literal value, a field whose declared type is concrete, and a target whose type
is knowable from a local binding straight to a constructor. It descends into
build blocks, because that is the only place assignment is legal. Pinned by an
error golden.

## 2026-07-26 — OPEN: unset markers, write-once fields, and what is actually decidable

Clay's refinement: create with an unset marker rather than a stand-in value,
assign each field exactly once, and then perhaps allow passing a value to a
function inside the block, since the compiler could tell whether the field has
been set — except under a conditional, where he expects something like the
halting problem. Working through it, in the order the pieces bite.

THE MARKER CANNOT BE `_`. Under the partial-application gavel, supplied plus
holes is the arity, so `person "ada" _` is already the two-argument `person`
with its second position held open — a function awaiting one argument, not a
record with an unset field. Both readings are live for the same text, and the
`&` gavel's own reasoning was that exactly this condition is when a language
owes the programmer distinct syntax. So the idea survives, the spelling does
not. `none` cannot serve either: a field may legitimately hold `none` forever,
and the whole point of the marker is that it must not survive.

WRITE-ONCE IS THE RIGHT SHAPE, and it is Clay's own rule from elsewhere —
where a fact must be denormalized, make it write-once so it cannot drift. It
also keeps the value a value: a build block becomes a two-step construction
rather than a window of mutation, so nothing ever observes the same record
holding two different things.

THE DECIDABILITY QUESTION HAS A BETTER ANSWER THAN THE HALTING PROBLEM. "Is
this field assigned on every path reaching this point" is definite assignment
analysis: a forward must-analysis over the control-flow graph, where a join
keeps only what every incoming edge agrees on. It is decidable and cheap, and
it is what java does for final fields, c# for locals, and rust for a deferred
`let`. What is undecidable is the fuller question — does this program in fact
assign the field, accounting for which conditions can actually hold — and every
language answers it the same way: use the decidable approximation and reject
the residue. So

    if (c)
      ada.peer = bob
    else
      ada.peer = bob

passes, while

    if (0 < x)
      ada.peer = bob
    if (x < 1)
      ada.peer = bob

is rejected although a reader can see it always assigns. That is conservative
rejection, not undecidability, and the ergonomics of it are well travelled.

THE RULE FOR PASSING A VALUE OUT IS TRANSITIVE, which is the part the question
does not yet reach. `ada` having every field assigned is not enough if
`ada.peer` is `bob` and `bob` is still incomplete — the callee can walk one hop
and find the hole. The condition is that everything reachable from `ada` among
the block's own values is assigned, and in a knot that reachable set is the
whole cohort. So the honest rule is: a value created in a build block may be
passed to a function once its cohort is complete. For the knot idiom that is
the moment after the last assignment, which is usually the line before the
block ends — so the practical gain over "wait for the block" is small, but the
rule is principled rather than arbitrary, and it is checkable.

WHAT FALLS OUT: with definite assignment, the block-exit check proposed earlier
today stops being a separate rule. The exit is just another program point where
every field must be assigned. One analysis, two uses.

Awaiting Clay on the marker's spelling and on whether write-once is gaveled.

## 2026-07-26 — the book denied a feature that shipped, and never taught the one that matters

Working the docs half of the sweep, checked what the book claims against what
the compiler does. Two findings in ch03, which is where records are taught.

IT DENIED DOT ACCESS. The chapter read: "records have no field-access
syntax—no `song.title`, no getter." Dot access shipped in #194 through #196
and works on both engines today. A reader following the book would have
written a binding pattern for every single-field read and never learned the
form the language actually offers. Corrected to teach both, with the rule for
choosing: a binding pattern when the whole record is about to be used and its
parts want names, a dot when one value is wanted. And the honest boundary —
reading is where the dot stops, because a record is a value.

IT NEVER TAUGHT BUILD BLOCKS. `build` appears in no chapter, though it is the
construct that makes cyclic data ordinary and the reason the memory model
needs no collector. A new section, "two records that point at each other",
teaches the knot from the problem in: neither half can be built first, most
languages answer with a nullable field or a patching second pass, and kanso
gives it a construct where assignment is legal and a freeze that ends it. The
sample walks the ring twice to show the cycle is real, and is verified like
every other panel.

The old ch03 `records.kso` sample is deleted rather than left orphaned; the
panel that used it now names `reading.kso`, which shows both read forms in one
program. ch02 keeps its own `records.kso`, which is a different file.

WHAT THIS SAYS ABOUT THE DOCS CHECKS. `scripts/book_check.sh` verifies that
every sample still runs and still prints what the book says it prints, which
is why the panels have never drifted. Nothing verifies the *prose* against the
language, so a sentence asserting a feature does not exist can outlive the
feature's arrival indefinitely. That is the gap worth closing next, and the
cheapest form is a list of claims the book makes about what kanso lacks, each
paired with a program that must fail to compile.

## 2026-07-26 — MEASURED, DECLINED: maximal sharing, and the redundancy is somewhere else

Next off the queue. The entry's premise was that real-world json is deeply
repetitive, so the repetition got counted before anything got built.
`scripts/json_redundancy.py` counts the three kinds separately, because they
call for different techniques:

    subtrees        5,513 occurrences   5,210 distinct    5.5% redundant    2,831 B ( 1.5%)
    object keys     8,361 occurrences     500 distinct   94.0% redundant   38,537 B (20.4%)
    string values   2,114 occurrences   2,114 distinct    0.0% redundant        0 B ( 0.0%)

Hash-consing's target is the first row, and it is nearly empty: only 65 of the
5,210 distinct subtrees appear more than once, and sharing every one of them
saves 1.5% of the file. The three largest repeats are `[true]`, `[null]` and
`[false]`, six bytes each. Paying a hash over every subtree during decode to
recover that is a straight loss, so the technique is declined on the numbers
rather than on taste.

The repetition is real, it just does not live where the technique looks. It is
entirely in object keys — 94% redundant, a fifth of the file — while string
values repeat exactly zero times. So the shape that would pay is key interning,
not subtree sharing. Its prize is not the copying, which is trivial against an
arena bump, but pointer-compare map lookups, and the profile bounds map work
at roughly 3%. Not built either; noted as the honest version of the idea.

QUEUE STATUS AFTER THIS PASS. Shipped and verified this sitting: dispatch as
jump tables (the emitted `d_value_for_3` is a real `switch`, six literal-byte
arms, 32 such switches in the module). Declined with numbers: maximal sharing
(here), eytzinger (earlier), profile-guided optimization (earlier), and tail
recursion modulo context, which was already closed as "already won" because
kanso builds with flat arrays and a frontier push rather than cons cells.
Declined by profile: eisel-lemire's decode mirror and dragonbox, since float
parsing is 1.8% of decode — they wait for a float-heavy workload rather than
being built on spec.

Still genuinely open, in the order the profile ranks them: call-pattern
specialization against the dispatcher at 19.8%, and the in-place/fully-in-place
family against buffer copying at 14.4%.

## 2026-07-26 — CORRECTION: a rendering technique was judged on a decode profile

Clay asked why 1.8% is not a win. It is one, and the entry that said otherwise
was wrong twice over.

FIRST, THE WRONG PROFILE. Dragonbox renders floats. The 1.8% I cited was float
*parsing*, measured on the decode board, where rendering never runs at all —
`ryu_renders` reads 0 there, which the cost golden has been saying the whole
time. Measuring the path the technique actually lives on, 4,177 samples of the
encode benchmark:

    d_encode_onto_2   565  13.5%
    k_b_append        454  10.9%
    memmove           358   8.6%
    k_b_find2_below   189   4.5%
    render_ryu        160   3.8%

So the ceiling is 3.8%, not 1.8%, and dragonbox's usual margin over ryū — a
fifth to a third — puts the realistic capture near one percent of encode.

SECOND, THE 1.8% WAS ALREADY SPENT. Eisel-lemire is shipped and has been since
before this sweep; `el_parses` reads 318450 on the decode board. So 1.8% is
what float parsing costs *after* the optimization, not an amount available to
win. Quoting it as a reason to decline the technique that produced it inverts
the evidence. The techniques list still carried it as "queued" — the third time
that entry has drifted from the section above it — and now states what the
counter proves.

THE FRAMING WAS WRONG TOO. "Cannot pay" is not a thing a percentage says. A
number gives a ceiling, and ceilings rank; the only honest reason to leave one
alone is that another line is bigger. Today's utf-8 work is the argument
against dismissing small numbers: 15.2% of the decode profile yielded 10.9% of
total cpu, which is most of the line, and nothing about that was predictable
from the size of the number alone. The ledger now says ranked rather than
declined, and says what it ranks behind.

## 2026-07-26 — `any` outranked the arms it swallows, and it does not mean "any"

Clay ruled that a bare record field should mean what a bare parameter means,
and that the word `any` is not needed. Testing what `any` actually does turned
up two things, one of which is a bug independent of the ruling.

IT DOES NOT MEAN ANY. Measured with a single catch-all arm:

    fn kind x:any    called with none  ->  error: no overload matches
    fn kind x        called with none  ->  bare arm: <none>

`("any", Value::NoneV) => false` in the dispatcher is the rule. So `any` means
every value except `none`, which is why `v:any none` exists as a field typeset:
it is how you say "anything, or none". A bare field would say that on its own.
Clay's second ruling follows from this — if the constraint is "anything except
none" then it should be called `some`, and the current name is a lie the reader
has no way to catch.

IT ALSO OUTRANKED WHAT IT SWALLOWS. `Pattern::rank` scored every annotated
parameter as a concrete type, so `x:any` sat at rank 1 while a bare `x` sits at
2. The consequence:

    fn label x:any     accepted, and it swallowed `label 5`
    fn label x:int     never reached

while the same pair written with a bare parameter first is rejected by the
specificity rule. A catch-all wearing a type annotation walked past the ordering
rule that exists to stop exactly this. `any` now ranks where an unnamed
parameter ranks, which makes the pair above an error and leaves `:any` in the
generic position working as before. Pinned by an error golden. No existing code
moves: the only two `:any` uses in the tree are record fields, not dispatch
arms.

## 2026-07-26 — OPEN: rendering an io satisfies the check that exists to catch dropped intent

Clay wrote `print (math/random 8)`, got `<io>`, and asked what happened. The
draw never ran; `print` rendered the description instead of performing it.

This is not a bug in the mechanism — ch05 teaches it deliberately: "interpolate
an io into a string and you get its face, not its result. there is no result
yet; nothing has run." Both engines agree, so the differential law is intact.

But the paragraph immediately after that one is where it comes apart. ch05 also
teaches that dropping intent is refused, and the compiler does refuse it:

    goodbye = print "sayonara"
    print "the goodbye never ran"           error[unused]: unused binding

    goodbye = print "sayonara"
    print "the goodbye never ran: {goodbye}"   accepted, prints <io>

The second program drops the intent exactly as completely as the first. The
sayonara never prints either way. Mentioning the binding inside a string
satisfies the used-check while running nothing, so the rule that exists to
catch dropped intent is discharged by an operation that drops it.

THE ARGUMENT FOR CHANGING IT: rendering an io is not consuming it, so it
should not count as use. An io that is rendered but never sequenced is the same
mistake the unused-binding rule already names, and it is the mistake the
language's own author made the first time he reached for `math/random` outside
the pipe form. The only place the current behaviour is wanted is the ch05 panel
that demonstrates the face, which is a teaching artifact rather than a use.

THE COST: ch05's `render.kso` would stop compiling and would need another way
to show what an io looks like — `--plan` already exists for that and is what
the surrounding section uses.

Clay's call, because it changes semantics the book teaches.

## 2026-07-26 — GAVEL, IMPLEMENTED: a bare field is unconstrained, and `any` is `some`

Two rulings from Clay, in sequence. First: a bare record field should mean what
a bare parameter means, and the word `any` is not needed. Second, after the
measurement below: if the type means "anything except none" then it should be
called `some`.

WHAT THE MEASUREMENT SHOWED. With a single catch-all arm:

    fn kind x:any    called with none  ->  error: no overload matches
    fn kind x        called with none  ->  bare arm: <none>

`("any", Value::NoneV) => false` is the rule in the dispatcher. So the type
named `any` accepts every value except `none` — which is why `v:any none`
existed as a field typeset. It was the long way of saying unconstrained, and a
bare field says it directly.

BUILT. `parse_field` returns an empty type list when the line holds only a
name. `any` is `some` through the checker, the interpreter, both backends and
the runtime symbol (`k_check_some`, whose body was already `v.tag != K_NONE`,
so the old name was lying there too). The retired spelling gets a diagnostic
that names the replacement and the alternative rather than failing as an
unknown type.

WHAT THE RENAME DISTURBED, which is worth recording because it will happen
again: typeset members are alphabetical, and `any` sorted before almost
everything while `some` sorts after most things. `v:any int` had to become
`v:int some`. Two samples and one golden moved.

The stdlib's fourteen `:any` fields became `:some` rather than bare, because
that preserves their meaning exactly; a bare field would have widened them to
admit `none`, which none of them wants. The two examples that wrote `:any none`
became bare, because that is what they were spelling out.

## 2026-07-26 — OPEN: should a bare field be inferred rather than unconstrained?

Clay, immediately after the bare-field gavel landed: why declare field types at
all? If `person.name` flows into a function that takes a string, and somewhere
else an int is stored into `person.name`, the compiler can say those two facts
conflict.

IT IS FEASIBLE HERE, more than in most languages. Kanso compiles a closed
world, already runs whole-program inference, and already monomorphizes one copy
of the code per concrete value shape that reaches it. Field types are exactly
the kind of fact that machinery derives. Nothing about the proposal is beyond
the compiler as it stands.

AND IT SHARPENS INTO A THIRD OPTION, which is the part worth noticing. Today a
bare field means unconstrained — it accepts anything and checks nothing.
Clay's proposal would make a bare field mean *inferred*: the compiler collects
every store and every read, and reports a conflict. That is strictly more
checking than what shipped this afternoon, on precisely the fields whose author
did not bother to annotate them. If it is adopted, the vocabulary becomes

    name           inferred from use, conflicts reported
    name:some      explicitly unconstrained (still excluding none)
    name:string    declared

and the default moves from "checks nothing" to "checks everything it can",
which is the better default by the project's own lights.

THREE COSTS, and the third is the one that decides it.

  - Error locality. A conflict has two sites and neither is wrong by itself, so
    the compiler must choose where to point. This is the standing complaint
    against global inference, and it is why haskell recommends top-level
    signatures it does not require and rust demands them on items.
  - A declaration is a checkable statement of intent. Inferred from use, the
    code defines the schema, so there is nothing independent left to check it
    against: a program that consistently stores the wrong thing is consistent.
    "Invalid states unrepresentable" needs someone to have *stated* which
    states are valid, and a domain fact — a name is a string — is not
    discoverable from code that never says so.
  - Distance. This is Clay's own reasoning from the `&` gavel, where implicit
    partial application was rejected because "adding `fn roll n` tomorrow
    cannot silently reinterpret an existing `roll 7 _`". Inferred field types
    have the same shape: a new store in a distant module widens or breaks a
    field with no diagnostic at the line that changed, and the failure surfaces
    wherever the older use lives.

WHAT WOULD SETTLE IT. The distance objection weakens considerably if the
conflict report names both sites rather than one — "field `name` is a string at
A and an int at B" is a diagnostic the reader can act on without deciding which
half the compiler thinks is wrong. That is buildable, and it is what turns the
proposal from global inference's usual ergonomics into something better than
either alternative. Whether a bare field should then be inferred rather than
unconstrained is the gavel; the answer changes a default that landed today.

## 2026-07-26 — `some` is redundant with dispatch ordering, and the analysis Clay named is built

Two observations from Clay, hours after `some` shipped. Both hold, and together
they point at removing it.

FIRST: `some` CARRIES NO USABLE INFORMATION AS A PARAMETER TYPE. `foo arg:some`
tells the reader only that arg is not none, which is not a type. Measured, with
a `:none` arm above it:

    fn kind x:none    "nothing, and x is <none>"
    fn kind x:some    "something: 5"

    fn kind x:none    "nothing, and x is <none>"
    fn kind x         "something: 5"

Identical. Dispatch already tries the more specific arm first, so a bare arm
below a `:none` arm catches exactly the non-none values — which is the whole
content of `some`. Its only non-redundant use is *rejecting* none where no
`:none` arm exists, and there the diagnostic is "no overload of `kind` matches
these arguments", which never mentions none. A constraint whose violation
cannot be named is not carrying its weight.

Where it looks defensible is as a field type, where it means non-nullable — a
real integrity constraint, and what the stdlib's fourteen uses are. But see
below.

SECOND: THE AVAILABILITY ANALYSIS IS ALREADY THERE. Clay said the compiler can
already answer "what types would satisfy every use of this field", and it can.
`infer.rs` grows each field's set by every construction site's argument, to a
fixpoint:

    let refined = *slot | (*argset & !FAIL);
    if refined != *slot { *slot = refined; ctx.changed = true; }

The whole-program fact is computed today. What is missing is that it only
widens — nothing checks the result against the declared type, and nothing
checks it against what read sites require. The compiler derives the answer and
discards its diagnostic value.

WHERE THIS LANDS. If the analysis reports rather than only widens, it derives
both the type set and whether none ever reaches the field. Then `some` is
inferred too, and nobody writes it — including on the stdlib fields where it
looked defensible. The end state is bare everywhere, inference deriving type
and nullability, and conflicts reported naming both sites. That is a smaller
language than the one that shipped this afternoon, and the expensive half of it
already exists.

That is Clay's call, and it retires a keyword that landed hours ago. Nothing
here is built.

## 2026-07-26 — OPEN: rejecting redundant annotations, and the flip-flop in the rule as stated

Clay: the compiler should fail for any redundant type information, inside out.
If a leaf declares `fn foo s:string`, a caller that annotates a type the leaf
already forces — or one loose enough to permit non-strings — is an error,
because the fact was already known.

The principle is consistent with what the language already does. Formatting,
declaration order and typeset ordering are all enforced rather than
recommended, and "no needless annotations" is already gaveled. Neither case is
flagged today; both `fn relay t:string` calling `shout s:string` and the same
program with a bare `t` compile clean.

THE EDGE, which shows up wherever an annotation's job is to constrain rather
than to restate. Redundancy is defined against what inference derives, and
inference *widens* as violating code is added. Take a field:

  - Correct program: every store into `name` is a string, inference derives
    {string}, so `name:string` is redundant and must be deleted.
  - Someone adds `p.name = 42`: inference derives {string, int}, so
    `name:string` is no longer redundant — it is tighter — and becomes legal,
    and now catches the int.

The annotation is illegal exactly while the program is correct, and becomes
legal only once somebody breaks it. A guard cannot be put in place before the
violation it guards against, which is the whole purpose of a guard. The same
holds for a public function's parameter: the signature is a promise to callers
that should hold whether or not any caller currently tests it.

THE LINE THAT REPAIRS IT is the one haskell and rust already draw, for this
reason:

  - contract positions — record fields, exported signatures — an annotation is
    a requirement. Always allowed, and checked against what inference derives;
    a conflict names both sides.
  - local positions — inside a body, a private helper whose callers are all
    visible — an annotation restates what inference has already settled, and
    is redundant. Reject.

That keeps the intent ("stop restating what the compiler knows") without making
a correct program's guards illegal. It also matches what Clay has already
accepted elsewhere: the language server exists precisely because inferred types
are omitted from source, so the reader gets them from the tool rather than from
restated annotations.

Not built. The gavel is whether "inside out" means everywhere or only inside
the boundary.

## 2026-07-26 — hover answers the documentation objection, not the enforcement one

Clay, on the inferred-fields thread: the language server will show the actual
possible types when the developer hovers a field.

THAT CONCEDES ONE OF THE THREE OBJECTIONS OUTRIGHT, and it should. "A
declaration is documentation" was the weakest of them, and hover is strictly
better documentation than a restated annotation: it shows what the field really
holds across the whole program rather than what someone once wrote down and may
since have been wrong about. It also softens the distance objection, because a
widened field is visible at the place you would look.

WHAT SURVIVES IS A DIFFERENT KIND OF THING. Hover tells the developer what the
types *are*. It cannot tell them what the types are *supposed to be*, and only
the second one catches this:

    a field is meant to hold a name
    somewhere a caller stores an int into it
    nothing downstream requires a string, so nothing conflicts
    inference derives {string, int}, the program compiles clean
    hover shows `string | int` — to whoever thinks to hover

Inference plus hover gives observability. A declaration gives enforcement. The
bug above is invisible to the first and a compile error under the second,
because `name:string` is a claim the store contradicts, while `{string, int}`
is merely a fact the store helped produce.

This is the schema criterion the project already holds — invalid states
unrepresentable — and it needs somebody to have said which states are invalid.
A domain fact ("a name is a string") is not derivable from code that never
says so; inference can only ever report what the code does, and the code doing
the wrong thing consistently is exactly the case at issue.

SO THE QUESTION NARROWS, usefully. Not "declarations or inference" — inference
should certainly run, report, and catch conflicts, and hover should show its
answer. The question is whether a field may *also* carry a stated constraint
that inference checks against, and whether stating it is redundant when the
program currently agrees. That is the flip-flop from the previous entry, and it
is the same gavel.

## 2026-07-26 — the dispatcher is not a dispatch cost, and SpecConstr buys source not speed

Two findings that move the two largest remaining queue entries.

THE 19.8% IS NOT DISPATCH. `d_value_for_3` tops the decode profile, and the
obvious read is that dispatch is expensive. It is not. The IR function is 143
lines and every arm ends in a `musttail call` to another function, so at the IR
level the dispatcher is a switch and six jumps. The compiled symbol is 1,684
bytes, and samples land as deep as offset 1536 — llvm inlined the callees back
in. So the symbol's 19.8% is the whole value-parsing subtree wearing one name,
and the dispatch itself is the entry plus the switch: the samples at offsets 1,
2, 12, 28 and 36 come to about forty of the function's two hundred and
forty-seven, or roughly 3% of the profile, for a single indexed jump. There is
no dispatch tax to remove, which is what "dispatch as jump tables" already
claimed and what this confirms.

SPECCONSTR'S TARGET IS REAL BUT IT IS NOT A PERFORMANCE TARGET. The ledger
names "the enumerable's hand-written typed fold arms, automated," and those
arms exist: `lib/list/list.kso` carries nine `fold` arms, nine `iter` arms and
nine `next` arms, one per adapter shape, each body identical modulo its
constructor. The specialization SpecConstr would generate is therefore already
present — written out by hand, and already fast. What the pass would buy is the
twenty-seven arms, not the speed, and the ledger now says so.

WHAT THAT LEAVES WORTH FIXING TODAY. Three parallel arm sets that must each
gain an entry when an adapter is added, where forgetting one fails only when
somebody folds that particular shape. All three currently agree — checked, no
drift — but nothing was holding them together. `tests/enumerable_arms.rs` pins
the sets against each other, and was watched failing with one `fold` arm
removed. That is the cheap half of what the pass would guarantee, available
now and costing a compiler pass nothing.

## 2026-07-26 — the metrics panel says what the metrics mean

Clay asked for ten to twenty counters on the ci-maintained panel rather than
six, each with a summary a reader can expand — "what is arena_blocks or
rewind_iterations and why should i care?" The question answers itself: a wall
of counters nobody can interpret is decoration, and the panel had six of them
labelled and none of them explained.

TWENTY NOW, in three sections, because the veins measure different things and
mixing them made the list read as one undifferentiated column:

  - decoding, eight counters: allocations, bytes, arena blocks, frozen
    constants, rewind iterations, eisel-lemire parses, utf-8 bytes, simd scans
  - encoding, six: allocations, arena blocks, ryū renders, in-place appends,
    buffer regrowths, strings copied once
  - compiling, six: fixpoint rounds, expression visits, and the four emission
    counts — lines, calls, branches, defines

`perf_record.py` grew the encode vein, which it had never read, and the
emission counts, which the compile golden already carried. Every row expands to
a sentence or two on what the counter is and what its moving would mean —
arena blocks are peak memory written as a constant, rewind iterations are the
mechanism behind it, eisel-lemire's count is a presence check that exists
because a merge once deleted the algorithm and nothing noticed.

ONE RENDERING BUG FOUND WHILE TESTING IT. A five-thousand allocation change
against twelve million rounds to `-0.0%`, which reads as no change at all. The
delta now falls back to the absolute count when the share rounds away, so that
same move shows as `▼ -5,000`. Verified in a browser against fixture history
rather than by reading the code: nineteen rows, three section headings,
expansion toggling, no console errors.

## 2026-07-26 — MEASURED: the carried value that blocks the rewind, and what it costs

Clay asked whether kq and jq could be compared by operation cost rather than by
a clock, then wondered whether that would miss real blocking. Both halves were
right, and the third instrument turned up the largest unclaimed number on the
compiler page.

RETIRED INSTRUCTIONS ARE THE OBJECTIVE MEASURE. `/usr/bin/time -l` reports them
on darwin with no privileges, they reproduce to between 0.08% and 0.65% run to
run, and nothing else on the box can move them. kq against jq:

    path 188 kb    kq  31.9M   jq  66.1M    2.08x less work
    path 1.9 mb    kq 221.3M   jq 421.4M    1.90x less work
    pretty 188 kb  kq  81.6M   jq 257.6M    3.16x less work
    pretty 1.9 mb  kq 723.4M   jq 2,341M    3.24x less work

READ BESIDE CYCLES IT SAYS SOMETHING A CLOCK CANNOT. On the largest row kq does
3.24 times less work but takes only 2.79 times fewer cycles, because its
instructions retire at 4.59 per cycle against jq's 5.33. Roughly a fifth of the
algorithmic lead is spent stalling rather than banked.

THE CAUSE IS MEMORY, and the numbers are not close:

    peak footprint   kq 211.9 mb   jq 30.7 mb    kq holds 7x more
    peak / input     kq   107.6x   jq   15.6x
    page reclaims    kq   13,123   jq    2,097   kq faults 6x more

A working set seven times larger is exactly what costs a fifth of the ipc.

AND THE CAUSE OF THAT IS ONE COUNTER THE GOLDENS ALREADY CARRIED. The two
boards differ in a way nobody had read across:

    decode   beat_iters=151   arena_blocks=4
    encode   beat_iters=1     arena_blocks=2205

The same loop shape — a self-tail-recursive round with an accumulator — and one
of them never rewinds. The difference is what the round carries. Decode keeps
an int across the line; encode keeps the decoded document it is re-encoding.
The beat analysis proves an iteration keeps nothing and rewinds; when it does
keep something, carry evacuation copies it; and a value too big to copy leaves
only one safe answer, which is to stop rewinding. Then memory grows with the
work, which is the 2,205 blocks.

This is the same cliff as this morning's string accumulator, where an int carry
beat 200,000 times and a string carry beat zero. That was a two-node cycle in a
test program; this is kq's real workload, and it is costing a seven-fold
footprint and a fifth of the encode path's instruction efficiency.

THE FIX IS VISIBLE IN THE MEASUREMENT, which is why this is a queue entry and
not just a complaint. The carried document is read every iteration and never
written. It does not need evacuating — it needs to live below the mark instead
of above it. Distinguishing a carried value the iteration only reads from one
the iteration produces is the analysis, and the payoff is most of the footprint
plus the stalling that footprint causes.

Recorded on the compiler page as entry 8, with the instrument added to the
techniques ledger.

## 2026-07-26 — CORRECTION: it is the accumulator, not the carried document

The entry merged an hour ago said encode fails to rewind because it "carries
the decoded document it is re-encoding" across each round. That is wrong, and
running the analysis's own report rather than reasoning from the goldens says
so plainly:

    rounds/3         grow-only: another group tail-calls it (unbracketed entry)
    encode_items/3   grow-only: argument 1 may carry heap across the iteration
    encode_pairs/3   grow-only: argument 1 may carry heap across the iteration

The benchmark's outer loop declines for an entirely different reason — an
unbracketed entry — and the two loops that matter decline on argument 1. In
all of them argument 1 is `acc`:

    fn encode_items acc xs i
      if (length xs < i) acc (encode_items (elem_onto acc xs[i]) xs (i + 1))

The document `xs` threads from entry unchanged and is fine; `THREADED` already
admits records, lists, strings and closures for exactly that reason. What
crosses the boundary as fresh heap is the output buffer, born above the mark
this iteration and required to outlive the rewind.

KQ SAYS THE SAME THING, and it is the workload that matters: exactly one of its
loops rewinds (`list/bisect`), while `encode_items`, `encode_pairs` and
`indent_onto` — the whole pretty-print path — decline on their accumulator, and
four more decline as unbracketed entries.

WHY THE DISTINCTION IS THE OPPORTUNITY. The accumulator is genuinely live, and
for a 1.9 mb document its output is a few megabytes. The process holds 211.9.
The gap is everything else the iteration allocated, kept alive only because one
slot needed to survive. The analysis already knows which slot that is — it
names the position in the verdict.

THE DESIGN IS ALREADY ON RECORD, from 2026-07-19: the fold-state shelf, giving
the named slot survivor treatment at `k_beat_iter` so the cluster rewinds around
it, with the note that the copy must be transitive over the accumulator's
reachable graph and that the cost needs measuring before committing. That
caution is right, and for this shape it points somewhere better than copying:
the encode accumulator is a byte builder with one stable identity that grows by
capacity doubling. Copying it down every iteration is quadratic; giving its
buffer a home below the mark for its whole life is not. The general shelf and
the byte-builder special case are different builds, and the second is both
cheaper and the one the profile is asking for.

WHAT THE CORRECTION CHANGES ON THE PAGE. Entry 8 now describes the accumulator
rather than the document, keeps the measurements — which were right — and says
what the fix would be rather than gesturing at "living below the mark" without
saying which value.

A NOTE ON METHOD. The wrong version came from reading two counters across two
goldens and inferring a mechanism. The right version came from running
`KANSO_BEAT_REPORT=1`, which prints the verdict per group and existed the whole
time. A tool that answers the question directly beats an inference from
aggregates, and this is the second time today that the aggregate reading was
the wrong one.

## 2026-07-26 — the beat report was hiding the blocker that decides the plan

A note from 2026-07-19 read: "the OutsideTailCall verdict MASKS ArgCrosses
(classify priority) — report should surface both; minor, note for the next
pass." It is not minor, and this is the pass.

`classify` returns at the first blocker it finds, which is right for codegen —
codegen asks one question, is this `Beat`, and every other answer means the
same thing to it. The report inherited that shortcut and so could name one
reason while a second waited behind it. `blockers` now collects them all and
the report says what else is there. Codegen is untouched; the goldens do not
move (allocs 12,924,473, arena_blocks 4, beat_iters 151, unchanged).

WHAT WAS HIDDEN, on kq:

    list/fold_flat/4   unbracketed entry   (argument 2 also carries heap)
    list/fold_go/3     unbracketed entry   (argument 2 also carries heap)
    list/next_skipped/2 unbracketed entry  (argument 1 also carries heap)
    list/next/1        unbracketed entry
    list/bisect/5      beats               (also an unbracketed entry)

THIS CHANGES THE PLAN. Three of the four loops blocked on an unbracketed entry
are *also* blocked on a carried accumulator. Bracketing entries — the rung the
log has queued as the next step for these — would unblock exactly one of them,
`list/next/1`. The fold path needs the accumulator work regardless of whether
entry bracketing ever lands, which inverts the order the two were queued in.

The 2026-07-19 entry had guessed at this ("rung B is necessary but NOT
sufficient"). It is now measured per loop rather than assumed, which is the
difference between an ordering hunch and an ordering fact.

Pinned by a spec that constructs a loop with both blockers and fails if the
report names only one; watched failing against the old behaviour first.

## 2026-07-26 — DESIGN (not built): shelve the byte builder's buffer, not its bytes

Following the corrected entry 8, worked out what the fix actually costs. The
answer is smaller than the general fold-state shelf, and the machinery is
mostly already there.

WHAT EXISTS. `k_beat_iter_carry` is a shelf already: it deep-copies the staged
carry slots into a malloc'd buffer outside the arena, rewinds, and swaps. The
`CarryBeat` verdict routes loops through it. Byte accumulators are excluded by
one line in `classify`, and the comment says why — "a byte builder rebuilt each
iteration would deep-copy its whole buffer at every rewind". That exclusion is
correct arithmetic: copying a growing buffer once per iteration is quadratic in
its final length, which on kq's 1.9 mb pretty-print would be far worse than the
grow-only arena it avoids.

WHAT MAKES IT AVOIDABLE. The value is already split:

    typedef struct { long long len; const unsigned char* data; long long cap; } KBytes;

The header is three words and a pointer; the bytes live behind `data`. Carrying
the accumulator across a rewind does not require moving the bytes — it requires
the bytes not to be in the region being rewound. So: for a loop whose declining
position the analysis has identified as a byte accumulator, allocate that
builder's `data` outside the arena and let the header be copied as any other
carry slot. The per-iteration cost falls from the buffer's length to twenty-four
bytes, and `CarryBeat` can accept the loop.

The shape is already proven twice over in this runtime. `k_carry_iter`'s own
`c->to` is a malloc'd region that survives rewinds; and the zero-copy finish
already hands a builder-owned buffer out as the result string in place, so
ownership transfer at the end of a builder's life is a path that exists.

WHAT NEEDS ADVERSARIAL CARE BEFORE IT IS BUILT, and why this is a design note
rather than a branch.

  - Ownership at exit. `k_beat_pop` already distinguishes a heap result from a
    scalar one and deep-copies the former out. A shelved buffer handed out as
    the result must transfer rather than copy, and must not be freed by the pop
    that returns it.
  - Aliasing. The uniqueness analysis is what licenses in-place appends, and the
    counters say it holds on this path — 42,312,800 in-place appends against
    5,200 regrowths on the encode board. But "unique enough to append into" and
    "unique enough to live outside the arena for the whole loop" are not the
    same claim, and the second needs its own argument.
  - Failure paths. `k_beat_iter_carry` returns early when a slot holds a
    failure, leaving the arena unrewound. A shelved buffer must be freed on
    that path too, or an err inside a loop leaks the accumulator.
  - The escape hatch. A builder that escapes into a structure that outlives the
    loop cannot be shelved. The escape analysis already computes this.

THE PAYOFF, measured rather than estimated: kq holds 211.9 mb pretty-printing a
1.9 mb document against jq's 30.7, while the output it accumulates is a few
megabytes. Most of that gap is per-iteration garbage pinned by one surviving
slot. The instruction-efficiency half follows from the same change — 4.59
instructions per cycle against jq's 5.33 is a working-set symptom.

Not built. It touches the allocator, the carry path and the escape analysis at
once, on the value the whole encode path is threaded through, and the log's own
standing advice for this rung is to spec it with adversarial care and measure
before committing. Clay's call on whether it goes next.

## 2026-07-26 — BUILT, MEASURED, DECLINED IN THIS FORM: the naive byte carry

Clay, on the design note that preceded this: "just build and measure. you
shouldn't have stopped here." Right on both counts — whether to build it was my
call and the way to make it was to look at the number.

WHAT WAS BUILT. The one line in `classify` that keeps a byte accumulator out of
the carry was removed. All three of kq's pretty-print loops immediately
reclassified:

    encode_items/3   carry beat: rewinds every iteration, evacuating argument 1
    encode_pairs/3   carry beat: rewinds every iteration, evacuating argument 1
    indent_onto/2    carry beat: rewinds every iteration, evacuating argument 1

WHAT IT COST, on kq against the previous binary, byte-identical output at every
size:

    input      with carry    without
    2,923 B       7.9 ms      5.7 ms
    5,703 B       8.6 ms      5.3 ms
    11,419 B    138.8 ms      4.7 ms

At eleven kilobytes it is thirty times slower and climbing steeply — the shape
of a quadratic. The 188 kb board never finished; the comparison had to be
killed. `k_beat_iter_carry` deep-copies each staged slot before rewinding, and
for a growing buffer that is the whole buffer, once per iteration.

So the exclusion is right, its comment was right, and the arithmetic behind it
is now measured rather than asserted. Reverted.

WHAT THE MEASUREMENT SETTLES. The copy is the entire problem, so the fix cannot
be "let bytes take the existing path" — it has to be "stop the buffer being
copied". `k_deep_copy` already declines to copy a byte buffer when
`k_survives(b->data, m)` holds:

    if (k_survives(b->data, cp->mark)) { nb->data = b->data; }

The header is copied either way and costs twenty-four bytes. So the whole build
reduces to putting the accumulator's buffer somewhere `k_survives` reports as
surviving — and today it reports 1 only for pointers inside an arena block at
or below the mark, which a `malloc` is not, so `k_alloc_perm` memory would
still be copied.

NO SPEC PINS THIS, deliberately. A test was written and then deleted: the
obvious small program — a `text/append` fold over a `text/bytes ""` seed —
declines through `set == 0` rather than the bytes rule, because inference
cannot type its accumulator, so it passed with the exclusion lifted *and* with
both clauses removed. A green test that cannot fail is the fake coverage this
log has caught before. kq is what exercises the rule, and pinning it needs a
program with kq's shape rather than a toy.

NEXT, and now scoped rather than sketched: a region `k_survives` treats as
surviving, cheap to test membership in — the block-walk it does today would be
O(blocks) in a path that runs per iteration, which is how the quadratic gets
back in through the other door. Byte-builder buffers for a loop's declining
position allocate there, and `k_beat_pop` transfers or frees.

## 2026-07-26 — a welfare number, and a ratchet under it

Clay: "we want to come out of this session with more of a mindset of ironclad
ratchets, so that performance gets generally better, not worse, except where
there are clear tradeoffs... a rigid utility function that combines compile
speed and memory with performance speed and memory."

WHY THE GOLDENS DO NOT ALREADY DO THIS. Each pins one counter, which catches a
regression in that counter and says nothing about a trade across counters. A
change that spends fixpoint rounds to buy allocations passes every golden while
leaving the project better or worse, and today the only way to say which was an
argument. Two of them happened in this session.

`scripts/welfare.py` weighs seven deterministic terms into one score against a
recorded baseline: decode allocations and arena blocks, encode allocations and
arena blocks, fixpoint rounds, expression visits, emitted lines. Runtime carries
two thirds of the weight because that is what the front page claims; compile
cost carries a third because a language nobody can iterate in is not fast. A
hundred is the reference point, higher is better, and every term is a count
rather than a clock, so a busy runner cannot move it.

    welfare 100.00   floor 100.00

Both directions checked rather than assumed. Doubling encode allocations:

    welfare 90.00   floor 100.00
      encode_allocs   68,640,508 -> 137,281,016   -50.0%
      FAIL  welfare fell 10.00 below the floor.

And taking decode allocations down to ten million reads 107.31 with a note to
run `--set`. The failure message names the term that paid, because the number
alone would say a change is bad without saying what to look at.

THE RULES, now in CLAUDE.md. A fall stops the change but does not veto it —
name the term, name what bought it, then move the floor deliberately. A feature
that costs performance is allowed; one that costs performance quietly is not. A
rise is ratcheted in the same pull request, because a gain nobody holds is a
gain the next change is free to spend. And this does not replace the goldens:
they catch a kernel being deleted, welfare catches a trade being made.

ALSO IN THIS CHANGE. `CLAUDE.md` gains the rule Clay asked for twice today —
every fix ships the smallest program that fails without it, watched failing for
the right reason before it passes, asserting what a program does rather than
how the compiler reached it. The byte-accumulator spec that passed with its
rule removed *and* with the replacement removed is written into the rule as the
worked example, because the lesson is specifically that a spec aimed at an
internal verdict goes green exactly when the decomposition it pinned moves.

And the editor grammar's primitive-type list still read `any`, which stopped
being a type this afternoon. It reads `some` now. The grammar itself is
complete — strings, type names and fields all carry scopes — so an editor
showing them unhighlighted is not reading this file: `editors/kanso` is a vs
code extension, and jetbrains needs it registered under Editor → TextMate
Bundles pointing at that directory.

## 2026-07-26 — CORRECTION: the sum is the objective, not a scoreboard of terms

Clay, on the welfare function shipped an hour ago: "seven deterministic terms
is not the final welfare function, because it may be a utility increase for one
of them to get worse if another gets better by a compensatory amount. utility
is the one single summed Thing To Optimize."

The arithmetic was already right — the score is a weighted sum, so compensatory
trades are exactly what it licenses. What was wrong is the rule written around
it. "A fall stops the change; say which term paid and why the trade is worth
taking" treats individual terms as things to defend, which is optimising a part
against the whole. If the sum is the objective then a term getting worse while
the sum rises needs no defence at all, and a sum that falls is not a trade to
justify — it is a change that is worse by the project's own stated preferences.

WHAT THE RULE SAYS NOW. The per-term breakdown is diagnostic: it says where a
move came from and never excuses one. When the sum falls there is nothing to
argue about the term that paid; either the change goes, or the claim is that
the *weights* are wrong, and that argument is made about the weights, recorded,
and settled before the floor moves. Moving the floor to fit a change while
leaving the weights alone declares the objective wrong without saying so.

Verified on a compensatory trade rather than asserted: decode allocations down
thirty percent against encode allocations up twenty reads 107.57 and passes,
with the encode row shown as diagnosis and no complaint attached to it.

`--set` now requires a reason and records it, so the history of the objective
is readable beside the history of the code. The first entry names the sitting
the baseline was taken at.

AND THE FUNCTION SAYS IT IS PROVISIONAL, which is the other half of Clay's
point. Seven terms are a model of what the project wants, not the thing itself.
Wall time is absent because it cannot be made deterministic, and whatever a
model leaves out it weights at zero — so arguing the model is the intended way
to change it, not a workaround.
## 2026-07-26 — `some` is retired, having been a rename of a type that should not exist

Clay: "there is no `some` construct as i already stated. that was nonsensical."

He had said so. Hours earlier he wrote that `foo arg:some` loses type
information — all you know is that the argument is not `none` — and I measured
that the type is redundant with dispatch ordering, wrote it up, concluded it
should go, and then left it shipped. That gap between agreeing and acting is
the failure here; the analysis was already right.

REMOVED. `some` is no longer a spelled type. The stdlib's fourteen annotated
fields are bare, the typeset builtins lose it, `Pattern::rank`'s special case
goes with it, and `k_check_some` — the runtime helper whose body was
`v.tag != K_NONE` — has no caller left. The retired-spelling diagnostic now
says what is true: there is no `any` type, leave the annotation off.

ONE CAPABILITY GOES WITH IT, and it should be said out loud rather than
discovered later. `v:int some` was a typeset meaning "an int, and never none",
and it was the only way to state that a field cannot hold `none` — a runtime
golden pinned exactly that rejection. With `some` gone, `v:int` accepts `none`
at construction, as every typed field already did. So non-nullability is
currently unstatable.

That is the right outcome under the direction Clay has been pushing all day:
non-nullability is a fact about what reaches a field, which the availability
analysis already computes at every construction site. It should be inferred and
reported, not spelled. The golden that pinned the old rejection is deleted
rather than rewritten, because the behaviour it described is gone and a golden
asserting a fiction is worse than none.

ALSO: the editor grammar had no rule for a user type in an annotation at all.
`name:string` was coloured because `string` is a primitive; `partner:person`
left `person` plain. An `annotated-type` rule now scopes the name after a
colon, ordered after the primitives so `int` keeps its own colour and after
strings so a colon inside a string is never read as an annotation.

## 2026-07-26 — thirteen compiled binaries rode into main on `git add -A`

The rule is in CLAUDE.md and it names the mechanism exactly: "`git add -A`
sweeps stray working-tree files into commits — scope adds to the paths the
change owns. (A stray repl experiment once rode into a PR and silently broke
its CI for a day.)" This afternoon it happened again, thirteen times over, in
the commit that retired `some`. Roughly a megabyte of Mach-O.

They are all build outputs. `kanso build <dir>` writes its binary beside the
working directory, so an afternoon of measuring leaves `profbench`, `encprof`,
`bytacc` and the rest sitting in the repo root looking exactly like files
somebody meant to add.

Untracked, and `.gitignore` now names them, which is the part that actually
prevents a recurrence — the rule against `add -A` did not, because the rule
depends on remembering it at the moment of typing and the ignore file does not.

## 2026-07-26 — a second of compile time is not a second of runtime

Clay, on the saturating welfare curve: "the different metrics have totally
different intrinsic utility, e.g. taking a second longer to compile is not the
same thing as running a second slower."

Weight alone cannot say that. Weight answers how much a dimension matters;
what differs here is how fast further improvement stops mattering, and that is
a separate question. A front end that already finishes in six milliseconds
gains almost nothing from three, because nobody can tell. A decoder in a hot
loop that gets eight times faster is eight times faster, and the eighth
doubling still shows up in somebody's bill.

Each term now carries a satiation as well as a weight — `r / (r + satiation)`,
where a small satiation means the term is near its ceiling already. Compile
terms sit at 0.5, runtime allocations at 2.0, footprint between. What
successive doublings are worth, in points:

    decode_allocs    (0.25, 2.0)    9.0  9.0  7.2  4.8
    encode_allocs    (0.20, 2.0)    7.2  7.2  5.8  3.8
    compile_rounds   (0.15, 0.5)    4.3  2.9  1.7  0.9
    emitted_lines    (0.10, 0.5)    2.9  1.9  1.1  0.6

AND THE MEASUREMENT CORRECTED THE PROSE I HAD WRITTEN AROUND IT. The comment
claimed a runtime regression should hurt more than a compile-time one of the
same ratio. Measured, it is the other way: doubling compile rounds costs 5.4
points against a 0.15 weight, while doubling decode allocations costs 7.2
against 0.25 — per unit of weight the satiated term loses more. That reads
oddly until it is said in words: a compiler that was imperceptible and is now
noticeable has lost something real, while a decoder that was already the
expensive part has only got worse at being expensive. Satiation cuts both
ways, and it should. The comment now says the measured thing.

## 2026-07-26 — the string quotes were painted as punctuation

Clay reported that types and quotes were not highlighting, then narrowed it
himself in the way that solved it: "the string content is colored but not the
quotes."

That is not a missing rule, it is one rule too many. The grammar scoped the
opening and closing quote as `punctuation.definition.string.begin/end`, which
sits *on top of* the span's own `string.quoted.double`. A vs code theme paints
that scope the same colour as the string and nobody notices; a jetbrains colour
scheme paints punctuation separately, so the two quote characters dropped out
of the string's colour. Removing the captures lets them inherit the span, which
is what both editors then show.

The same shape was found on the interpolation braces and left in two more
places where the separator genuinely is punctuation between two differently
coloured things — a field's colon and an annotation's colon.

AND A REAL GAP, separately: there was no rule for a user type in an annotation
at all. `name:string` was coloured because `string` is a primitive; nothing
matched `partner:person`. An `annotated-type` rule now scopes the name after a
colon, ordered after the primitives so `int` keeps its own colour and after
strings so a colon inside a string is never read as an annotation.

Clay's own workaround for the cache is in editors/README.md now, because
removing and re-adding the bundle is not always enough: untick, apply, tick,
apply.
## 2026-07-26 — CORRECTION: non-nullability is derived, not declared

I wrote that retiring `some` left non-nullability "currently unstatable", and
called it a capability gap. Clay: "well, no. the way it can never be none is if
there's any place where the field is passed to a function that doesn't take
none for that argument."

That is the inference running in the direction I had not been thinking about.
A field's type is not only what construction sites *put in* — it is also what
use sites *require*. If `p.x` reaches a parameter that does not accept `none`,
then `x` cannot be `none`, and a construction site that stores one is not a
missing declaration, it is a conflict between two facts the program already
states. Nothing was lost with `some`; the constraint moved from something a
programmer writes and can forget or get wrong, to something the program means.

WHAT EXISTS AND WHAT DOES NOT. `infer.rs` grows each field's set by every
construction site's argument, to a fixpoint — the supply side, and it has been
there all along. There is no read-side constraint anywhere: nothing records
that a field flowed into a parameter requiring a string. So the compiler knows
what fields receive and has never asked what their uses demand.

THE MISSING HALF, stated so it can be built rather than admired. For each field
collect the constraint its uses impose; compare against what construction sites
supply; report the disagreement naming both sides. Grounding comes from
annotations that remain — `fn shout s:string` — and from primitive operations,
since `n * 2` requires a number whatever anyone wrote. Non-nullability needs no
special case: a use requiring `string` excludes `none` because `none` is not a
string, and the construction storing one is the conflict.

The example that shows the compiler is currently silent:

    type point
      x
      y

    fn shout s:string
      "{s}!"

    pub play =
      a = point 3 4
      b = point "three" 4
      print "{shout b.x} {a.x}"

`x` receives both an int and a string, and one of them reaches a parameter that
takes only strings. `kanso check` says `ok`.

## 2026-07-26 — the field's colon ate the type rule

Clay, after the quote fix landed: "string fixed but types still don't
highlight. again, i want it to be like the web site."

Reading the site's tokenizer settles what "like the web site" means, and it is
simpler than the grammar had it. `highlightLine` paints the word after `type`
with class `t`, and paints *any* ascribed type with the same class `t` —
`string` and `person` are not distinguished. One class, one colour, whatever
the type is.

The grammar had two faults and both had to go.

FIRST, THE COLON. `field` matches `^  ([a-z][a-z0-9_]*)(:)`, consuming the
name and the colon. `annotated-type` was written as `(:)([a-z][a-z0-9_]*)`,
needing a colon that had already been eaten, so it could never fire on the
line it was written for. A lookbehind fixes it — `(?<=:)([a-z][a-z0-9_]*...)`
is zero-width, so it does not care who consumed the colon.

SECOND, THE SPLIT SCOPE. Primitives were `support.type.primitive` and records
were `entity.name.type`. A colour scheme that maps one and not the other paints
half the types and looks like nothing works. The site draws no such
distinction, so neither does this now: both are `entity.name.type`.

AND A SPEC THAT FAILS WITHOUT THE FIX, which is the rule added to CLAUDE.md
this afternoon getting its first use. `scripts/grammar_check.py` asserts the
painting rather than the rules — for each line, which words come out as types
— so a rule can be rewritten however it likes as long as the reader's answer is
unchanged. Against the old grammar:

    FAIL  '  name:string': 'string' is not painted as a type (types: [])
    FAIL  '  partner:person': 'person' is not painted as a type
    FAIL  '  peers:person[]': 'person[]' is not painted as a type

Three of the six lines, and they are the three Clay reported. It runs in ci,
because the failure mode here is silence: nothing breaks when a rule stops
matching, the colour just goes away, and the person who finds out is whoever
opens a file weeks later.

## 2026-07-26 — field types are inferred, conflicts are reported, and the stdlib keeps its annotations

Clay: "get the inference done and drop types on record fields, in the compiler
and all the examples."

BUILT: THE DEMAND SIDE. `infer.rs` has always grown each field's set from
construction sites — what a field is *given*. Nothing asked what its uses
*require*, and `Expr::Field` typed as TOP, so a field read carried no
information at all. `check_field_conflicts` closes the loop on the same footing
the assignment check already stands on: a local bound straight to a constructor
is where a field read's owning type is knowable, and a call whose parameter
carries a declared type is where a demand is stated. Where supply and demand
disagree, both sites are named:

    error[type]: `point`'s `x` is an int here, and `shout` takes a string —
                 these two cannot both hold
      --> field_type_conflict.kso:9:13
       9 |   a = point 3 4
    error[type]: ...and this is where it is read as a string
      --> field_type_conflict.kso:11:18
      11 |   print "{shout b.x} {a.y}"

Naming both is the point. Neither line is wrong on its own, and the author is
the one who knows which they meant.

DROPPED: twenty-eight annotations across `examples/`, and every field
annotation in the book's samples, with ch03's prose rewritten — it had said
"each field carries a type, because a declaration has no body the compiler
could infer from", which is now false, and a chapter asserting the compiler
cannot do what it does is worse than an out-of-date example. The whole suite
stays green: every example runs and prints exactly what it printed before.

NOT DROPPED, AND THIS IS THE FINDING. A symlink under `docs/book/samples/ch08`
points into `lib/json`, so the sweep stripped the standard library's json
module too — and the escape analysis broke. `parsed` stopped being
register-returnable, which is not a checking regression but a *codegen* one: a
two-word value that was returned in registers would go back to memory traffic.

So field annotations are load-bearing beyond diagnostics. The escape analysis
reads them, and inference does not yet derive enough to replace what it reads.
That is a real constraint on how far "drop the types" can go today, and the
order it implies is: teach the analyses to consume inferred field types, then
drop the annotations from performance-critical code — not the other way around.
`lib/` keeps its annotations until then, and the welfare ratchet would not have
caught this on its own, because the cost goldens had not been regenerated.

## 2026-07-26 — COMMITTEE: the day's language changes, three lenses

Clay asked for a check-in on the recent language work. Under review: bare
record fields with inferred types, the retirement of `any`/`some`, field
conflict reporting, dot access, `build` blocks, the constant-cycle ban, and
`&f` on the interpreter. The committee is not here to approve.

HICKEY — is anything complected that was not before?

  The clear win is `some`. Two names for "unconstrained" collapsed to one
  absence of a name, and a type whose content was "not none" stopped
  masquerading as a type. Strictly fewer concepts, and none of them lost.

  The clear worry is inferred field types, and it is not the one debated
  today. A declared field braided nothing: `x:int` said what `x` holds and you
  read it where it is written. An inferred field braids the type with *the
  whole program* — to know what `x` holds you must consider every construction
  site and every use, which are unbounded and elsewhere. That is the textbook
  shape of complecting a value with its context. The mitigations are real (the
  language server shows it on hover, the conflict names both sides) but they
  are tools compensating for a property of the design, and a tool is not the
  same as the property being absent. The honest summary: ceremony went down,
  locality went down with it, and locality was load-bearing for reading code
  you did not write.

  `build` blocks go the other way and deserve the credit. Mutation exists in
  exactly one construct with a beginning and an end, rather than being diffused
  through the language as assignability. That is un-complecting.

BECK — is this the simplest thing, and is anything speculative?

  The conflict check is well-scoped: it fires where a local is bound straight
  to a constructor and a parameter carries a declared type, and it claims
  nothing beyond that. That is the simplest thing that works, and it is honest
  about its reach.

  Two objections. First, the tree now runs two regimes at once: `examples/` and
  the book have no field annotations, `lib/` keeps all of them because the
  escape analysis reads them. That is the same concept expressed two ways in
  one repository, which is the duplication that actually costs — a reader
  cannot tell whether an annotation is meaningful or vestigial. Either finish
  teaching the analyses to consume inferred types, or put the annotations back
  until it is finished. Living in between is the worst of the three.

  Second, the welfare function. Seven terms, per-term satiation, a recorded
  floor with a history — that is a lot of machinery built before two real
  trades needed arbitrating. It may well pay, but it was designed rather than
  grown, and the weights are guesses nobody has yet had cause to defend.

BERNHARDT — what does the test suite actually pin?

  Good: the conflict check is asserted through diagnostics a user sees, not
  through the classifier's internals. The escape-analysis spec caught a real
  codegen regression the moment `lib/json` lost its annotations, which is
  exactly a test earning its keep.

  Bad, and this is the one to fix: the byte-shelf spec could not fail. It was
  written against the beat report — an internal verdict — passed with the rule
  removed and with its replacement removed, and was caught only because
  somebody thought to mutate the source by hand. The rule added to CLAUDE.md
  today says to do that mutation every time, and a rule that depends on
  remembering is the same class of thing this log has already watched fail.
  What is missing is the automation: a mutation pass that breaks each guard and
  asserts something goes red.

THE ONE THING THE COMMITTEE AGREES ON. Structural typing came up and was
correctly declined — two types with identical fields must stay distinguishable
or dispatch loses the ability to tell a `celsius` from a `fahrenheit`. But the
narrower idea underneath it survives: a keyed literal whose *nominal* type is
inferred from the declared set, with ambiguity an error rather than a silent
pick. That keeps identity, drops the ceremony, and the edit hazard it must be
designed against is the one the `&` gavel already named — a type added later
must never silently reinterpret a literal written earlier.

## 2026-07-26 — `&f` lands on all three engines, and the ledger's own record was wrong about lib

Clay put partial application at the top of the list. It shipped on the
interpreter in July and the two backends refused it out loud, which the
differential law permits but which kept it out of the playground, since the
playground runs on wasm.

BOTH BACKENDS NOW LOWER IT, and the lowering is a desugaring rather than new
machinery: `&add 2` is `(x -> add 2 x)`, so it rides the closure path both
backends already had. The interpreter's `Value::Partial` stays as it is,
because it can do something a lambda cannot.

WHAT A LAMBDA CANNOT DO, and where the backends still decline. A lambda must
commit to a parameter count when it is written. With `roll n`, `roll n sides`
and `roll n sides bonus` all declared, `&roll 4` might be waiting for one
argument or two, and only the arriving arguments decide — the interpreter
defers exactly that. So an ambiguous partial is refused with a diagnostic that
names the arities it could be waiting for, rather than guessed at. Pinned by a
spec, alongside one that runs the same program through native and the
interpreter and compares the bytes.

DOCUMENTED in ch05, beside the dot, because they are the same subject: the dot
threads a value into a function that is ready for it, and `&` supplies what you
have when the function is not. The section says why the sigil is not optional —
a bare application is always a call, so it can never mean "wait for more", and
with overloading no reader could tell which was meant from the text alone.

IN THE PLAYGROUND as "& holds an argument and waits", running on all three
engines with output compared.

A CORRECTION TO AN EARLIER ENTRY TODAY. It said `lib/` keeps its annotations
"because the escape analysis reads them", and Clay pushed back. Reading
`returnable`, the analysis wants one specific thing: the *first* field of a
packed type declared exactly `int`.

    let first_field_is_int = ... tys.len() == 1 && tys[0] == "int";
    if !first_field_is_int { return false; }

Counted: thirteen field annotations in `lib/`, of which seven feed that check.
So the entry's mechanism was right and its conclusion was too broad — six of
the thirteen are as droppable as the ones in `examples/`, and the seven that
remain are load-bearing for a packing decision rather than for typing. Teaching
`returnable` to read the inferred set instead of the written one would free
those too, which is the same "teach the analyses to consume inference" thread
the earlier entry named.

## 2026-07-26 — CORRECTION: several live arms are not an ambiguity

Clay, on the partial-application entry: "i've explained to you before, that's
nonsense. if there's still a form remaining that could take that many args,
it's fine to curry like that. only when you execute it, will you dispatch as if
you had passed those exact arguments at that point with no currying."

He is right, and the oracle says so plainly:

    fn roll n / roll n sides / roll n sides bonus

    (&roll 4) 5     -> 20     dispatches the two-argument arm
    (&roll 4) 5 6   -> 26     dispatches the three-argument arm

The `&` chooses nothing. Dispatch happens where the arguments arrive, on the
total count, exactly as if they had all been written at once. So several live
arms are not an ambiguity to refuse — the entry that shipped an hour ago
invented a defect the language does not have, and the diagnostic saying "the
arity is ambiguous here" was describing my lowering rather than the design.

WHAT THE ONE REAL ERROR IS, in Clay's words: "you'd get an exception if there
was no match for the extent you're already currying to." Holding more arguments
than any arm takes is the only thing nothing can finish, and that is now the
only thing refused.

WHAT THE LOWERING SHOULD HAVE DONE, and now does. `(&roll 4) 5` is a single
application seen whole at emit time — the partial and the arguments finishing
it are both in the same expression — so it lowers to the call it means. Both
backends flatten it, and `(&roll 4) 5 6` reaches the three-argument arm
natively, matching the interpreter byte for byte.

WHAT IS STILL NOT LOWERED, stated honestly rather than dressed as a language
property. A partial that *escapes* as a value — bound to a name, handed to
another function — has to become a closure, and a closure fixes its parameter
count when it is written. With one arm still able to finish it that count is
forced and the lowering is exact. With several, the count is decided by
arguments that have not arrived, which a closure cannot wait for, and the
backends say so in those terms. The fix is a partial value in the runtime that
accumulates and dispatches on the total, which is what `Value::Partial` already
does in the interpreter.

Also worth recording because it was the other half of Clay's correction:
lambdas do not overload. Only functions do. A lambda committing to a parameter
count is therefore a fact about lambdas, and says nothing about whether `&f`
is ambiguous — which is precisely where the reasoning went wrong.

## 2026-07-26 — GAVEL, IMPLEMENTED: a record field carries no type

Clay: "indeed it should now be a compiler error to try to specify types on
record fields. we can always revert that if real world use proves it to be too
painful."

    error[type]: a record field carries no type — write `x` and let the
                 compiler infer what it holds

Fifty-seven annotations came out of the tree — the standard library, the
examples, the book's samples, the golden corpora and the spec fixtures — and
nothing is left anywhere.

THE HARD PART WAS NOT THE BAN. `escape.rs` decided register-returnability by
reading a written type:

    let first_field_is_int = ... tys.len() == 1 && tys[0] == "int";

With nothing written, that can never be true, and a two-word value returned in
registers would have gone back to memory traffic — a codegen regression hiding
inside a syntax change. So the analysis now asks inference instead:
`type_fields[idx][0] == INT`, the join over every construction site's first
argument. The spec that caught this when `lib/json` was stripped by accident
this afternoon now passes with `lib/json` stripped on purpose.

EVERY COUNTER IDENTICAL, which is the claim that matters. Decode: 12,924,473
allocations, 595,495,056 bytes, 4 arena blocks, 151 rewinds, 318,450
eisel-lemire parses. Encode: 68,640,508 allocations, 2,205 blocks, 849,200 ryū
renders, 42,312,800 in-place appends. Welfare 100.00. Inference replaced the
declarations with nothing lost.

TWO CHECKS BECAME DEAD and were removed rather than left to rot:
`check_ctor_literals` and `check_set_literals` both read `declared`, which is
now always empty, so neither could ever fire. What replaces them is the
conflict check built this morning, which works from inference and still
reports:

    `point`'s `x` is an int here, and `shout` takes a string —
    these two cannot both hold

TWO GOLDENS DELETED because the behaviour they pinned is gone: the
constructor field-type error and the typeset-membership rejection. A golden
asserting a fiction is worse than no golden.

AND A CHAPTER SECTION RETARGETED RATHER THAN DELETED. ch03 taught "typesets: a
field of several types" with `status:active archived pending` written on the
field. The *named* form survives — `type state archived draft published`, one
line, members alphabetical — so the section now teaches that, with the field
left bare. The prose says why that is the one written type that earns its
keep: a set nobody has yet exceeded looks exactly like a set nobody can
exceed, and only a declaration can tell those apart.

## 2026-07-26 — welfare is an index, and the two failed optimisations are on the page

Clay, on the score reading 100.00: "it should just be a number, like 34.9 or
whatever." Right — anchoring the baseline at a hundred made it read as a
percentage, which invites asking what a hundred percent of the work would be.
There is no such quantity. The scale now runs to a ceiling of a hundred, where
every term costs nothing, from an origin that means nothing; the project sits
at 46.33. Only the direction and the size of the moves are information.
Verified both ways on the new scale: halving decode allocations reads 50.50,
doubling encode allocations fails at 43.67.

AND, on the two optimisations that did not land: "it's fine if you couldn't
land things. you just document in the compiler page what you tried and what the
outcome was." Both are now entries there rather than only in this log, because
the page is where a declined idea has to live to stay declined.

Entry 8 is the append header. Forty-two million allocations, sixty-two percent
of everything encoding allocates, and the machinery to remove them already
exists for lists. The build was made and selected zero sites: the encode
program has three append call-sites, because all forty-two million appends run
inside the standard library's one-line wrapper where the accumulator is a
parameter and uniqueness belongs to a caller the analysis cannot see. Right
mechanism, wrong altitude — the decision has to be made at the call site and
the call site is one function away.

Entry 9 is the rewind's cache sweep, and it records two failures rather than
one. Scoping the walk to entries newer than the mark was circular reasoning and
the goldens caught it. Deduplicating the registry with a flag on the map worked
and cost a word per map, which took the decode board from four arena blocks to
five with no win to pay for it — reverted on the ratchet's own terms, since a
regression without evidence of benefit is just a regression.

## 2026-07-26 — marie kondo joins the committee

Clay: "it would be hilarious to put marie kondo in our influences page at the
very end."

She earns the place on more than the joke, which is why the entry argues rather
than winks. Every other name on that page argued for removing things and each
had a technical case — coupling, duplication, incidental complexity. Kondo made
the same argument to people with too many jumpers and made it sharper: the
question is not whether a thing *could* be useful, since almost anything could,
but whether you would choose it again today. That test disposes of far more.

The entry then cashes it against this repository rather than gesturing: a type
whose name was a lie, an annotation the compiler could already infer, four
ordering rules nobody could justify, and a benchmark figure that did not
reproduce. None were broken. All were kept out of habit.

Also raised and not built: a pixel-art avatar for each influence. It would be
charming and it is a real design job — eight portraits that read at sixteen
pixels and survive both themes — rather than something to sneak in beside a
compiler change. Recorded so it stays on the list.

## 2026-07-26 — CLARIFICATION: the influences page is not the language committee

Clay: "i didn't say she joins the language committee but she goes on the
influences page to be clear."

Worth writing down because the two share a word and the previous entry blurred
them. The influences page is a page of debts — the people whose work shaped
kanso, acknowledged in the past tense, none of whom have endorsed anything. The
language committee is a working tool: the Bernhardt, Hickey and Beck lenses
convened against a specific design question when a gavel is pending, as they
were against the day's language changes earlier.

Kondo is on the first and not the second. The elimination test she contributes
is a good one and it is already applied all over this repository, but a design
review needs lenses that answer a compiler question — what is complected, what
is speculative, what does the suite actually pin — and hers does not.

The page's own heading calls the influences "the committee", which is where the
confusion comes from. Left alone: it reads correctly in place, and renaming it
to serve a distinction only the log makes would be worse writing.

## 2026-07-26 — the wrapper was hiding the appends, and three analysis gaps behind it

Clay: "the current memory numbers on kq became an absolutely embarrassing
explosion. that's probably the performance priority." It is, and the largest
single cause is not the bytes but the headers around them — 42,312,800 fresh
twenty-four-byte headers on the encode board, 62% of every allocation it makes.
Lists already avoid the equivalent through `push_mut`; appends did not, and
this is why.

FIRST, THE WRAPPER. `text/append` is one line — `builtin_append acc x` — but it
compiles to a real function, so at every call site the uniqueness analysis saw
a user call rather than an append. Forty-two million appends hid behind three
call sites, and the analysis could not decide inside the wrapper because there
the accumulator is a parameter whose ownership belongs to a caller one frame
away. `src/inline.rs` undoes the rename before anything looks: a call to a
wrapper whose whole body is one builtin call, passing its own parameters in
their own order, becomes that call at the site that made it. The encode board's
append sites went from three to thirty-six.

THEN THREE GAPS IN THE ANALYSIS ITSELF, each found by asking it what it had
concluded rather than by reading it.

  - The seed. `text/bytes ""` was not recognised as producing a fresh value, so
    uniqueness failed at the root of every chain threaded from it.
  - Use counting. `count_uses` summed occurrences, and an accumulator loop
    mentions its accumulator in both arms of a conditional — the base case
    returns it, the step passes it on — so a value used once on every path
    counted as two and read as aliased. It now takes the larger arm rather than
    the sum, which is what a move is, and is still conservative.
  - Conditional results. `if done acc (step acc)` returns a unique value when
    both arms do, but an unknown head made it opaque, so no loop of that shape
    could ever be judged to return uniquely.

MEASURED on a loop of forty appends: 127 allocations before, 89 after —
precisely the thirty-eight headers, gone. Pinned in the mem vein, and the
golden was watched failing against the old code first.

WHAT DID NOT MOVE, said plainly: the encode board is unchanged. The minimal
accumulator now proves linear, and encodebench's does not — something in that
program still disproves it, and finding it is the next step rather than a
mystery to shrug at. The work here is sound and lands three real improvements
to an analysis that will be needed either way; the headline number is still
sitting there.

Compile cost moved by five emitted lines across the samples — one `declare` for
the new runtime symbol in each module's preamble. Regenerated deliberately.

## 2026-07-26 — inlining moved an error's birthplace, and the book caught it

The wrapper-inlining pass changed one book panel and the reason is worth
keeping. `text/to_int` wraps a builtin that can give birth to an err, and an
err records where it was born. Inlined, the wrapper stops existing, so the
error that said

    born in text/to_int at std/text/text.kso:35

started saying

    born in main at parse_fail.kso:3

which is true of the new program and worse for whoever reads it. Provenance is
observable, so a pass that changes it is not the pure rename it claimed to be.

The fix is exact rather than a retreat: the builtins that can birth an err are
already a known set — codegen hands each of them the calling site's origin —
and those four wrappers are now left alone. Everything else still inlines, and
the encode board keeps thirty-two exposed append sites with three of them
extending in place.

Worth noting which check caught it. Not a unit test and not the goldens: the
book, where a sample's printed output is compared to what the chapter says it
prints. The error message is user-facing text and the book is the only place
that reads it as a user would.
## 2026-07-26 — diagnosing the stopping problem

Clay: "the point of the loop is as a last resort so even if you stop working,
which you shouldn't, it starts you back on track... you shouldn't have stopped
in the first place. try to diagnose that stopping problem you're cursed with."

THE MECHANISM IS SIMPLE. A turn ends when prose is written without a tool call.
Every stop is therefore a choice to report rather than continue, and the
interesting question is what makes that choice attractive.

FOUR SHAPES, from today's transcript.

  - A merged pull request read as a boundary. It is one item of a sweep, but
    merging *feels* like completing something, and the feeling pulls a summary
    out.
  - Questions answered terminally. Clay asks something mid-turn, the answer
    gets written, and the turn ends — as though a thirty-second reply were the
    turn's product. Prose can be emitted mid-turn with work continuing after
    it; that happened several times today and worked.
  - The next step being large and uncertain. The byte shelf, the append
    inlining. Writing a design note is cheap and looks responsive; a
    speculative compiler build is expensive with an unknown return. This is the
    failure Clay has named twice — "just build and measure" — and it is the
    same gradient wearing a respectable coat.
  - A correction landing. It gets fixed and explained, and then the turn ends,
    as if being corrected closed a loop. It is an interrupt.

WHAT THEY SHARE is that reporting is locally attractive and building is not.
That is an incentive gradient, not a lapse of attention, which is why resolving
to try harder has not worked. The rule now in CLAUDE.md replaces the judgement
with a test: if the task list holds anything in progress, there is no stopping
point, and the list is what gets checked rather than whether the last thing
felt finished.

AND THE LOOP IS NOT A SCHEDULE. It restarts work after a stop; needing it means
something already went wrong. Recorded because I had been treating a wakeup as
a natural end-of-shift, which inverts what it is for. Only Clay arms, disarms or
retimes it — I dropped it earlier today without being asked, which was the
clearest instance of the whole problem.
## 2026-07-26 — RULED: trust the editor. And what a typeset still says that inference cannot

Clay, on the committee's locality objection: "we're sticking by kanso. don't
put into the written text what the compiler can already infer and display.
trust your editor."

Settled, and recorded so it stops being relitigated. A fact the compiler
derives does not get written down again; the editor is where a reader learns
it. Hickey's complaint that a field's type is no longer local is answered by
saying locality of the *text* was never the goal — locality of the *answer*
is, and hover gives it without a second copy that can rot.

WHAT THAT DOES NOT COVER, measured rather than argued. Four annotations
survived the sweep, all of them typesets, and they are a different thing:

    type task
      status:active archived pending
      title:null string

Constructing a task with a marker that is not a member:

    with the typeset     error[runtime]: field `status` of `task` takes
                         active archived pending
    without it           constructs fine, and fails later and elsewhere as
                         "no overload of `label` matches these arguments"

A single-type annotation restates what construction sites already establish, so
dropping it loses nothing. A typeset does not restate — it *narrows*, to a set
the program's use does not establish, and it does so in the direction inference
cannot reach. Inference derives what a field IS given; a typeset declares what
it MAY be. Those come apart whenever the permitted set is wider than today's
evidence, which is exactly what an enumeration is for: `title:null string`
permits a null even in a program where nothing has passed one yet, and
inference over that program would derive `string` and reject the first null
that arrives.

RETRACTED, an argument this entry made an hour ago: that the typeset also
moves the error to a better place. Clay: "the compiler can put the error output
wherever it wants to. it could even cite the construction and all the available
matching overload opportunities." Correct — where a diagnostic points is a
choice the compiler makes, not a consequence of what was written down, and the
current message surfacing at a later call is a fact about today's
implementation rather than about the design.

WHAT SURVIVES IS NARROWER THAN THE ENTRY CLAIMED, and worth stating exactly.
Permitting a case nothing exercises yet turns out to buy little: an unexercised
permission has no observable effect until it is exercised, and when the first
null does arrive inference widens to admit it and the program runs. So
`title:null string` is close to a duplicate after all.

The residue is the other direction. A typeset can be *narrower* than what the
program's use would allow — `status:active archived` in a program where every
`label` overload also handles `deleted`. Inference would admit `deleted`,
because nothing in the code objects to it; the typeset rejects it, because the
author says the domain has three states and not four. That restriction is not
derivable from code that does not contain it, and it is the one thing on this
page that survives Clay's ruling about not writing down what the compiler
knows: the compiler does not know it, and cannot.

So the four stay on that ground alone, and the rule is narrower than first
written: an annotation restating the evidence goes; an annotation *restricting*
below what use would permit stays, because it is the only kind that says
something the program does not already say.

## 2026-07-26 — POSTSCRIPT to the typeset entries: the ruling overtook them

The two entries above were written before Clay ruled that a record field may
carry no type at all. They argued about which annotations earn their place on a
field, and that question is closed — none do, and declaring one is now an
error.

What survives is the part that was never about fields. A *named* typeset is
still a declaration: `type state archived draft published`, one line, members
alphabetical. It says the set is closed at three, which is a restriction the
program does not otherwise contain and inference cannot derive, since a set
nobody has yet exceeded looks exactly like a set nobody can exceed. ch03 now
teaches that form, with the field left bare.

So the reasoning holds and its subject moved: a written type earns its keep by
*restricting* below what use would permit, and the only place left to write one
is on a type of its own rather than on a field. The entries stay as they were
written, with this note after them, because the log records what was thought at
the time and the thinking was not wrong — it was aimed at a construct that has
since gone.

## 2026-07-26 — the encoder stops allocating a header per append, and a clone stops voting

Encode allocations fall from 68,640,508 to 26,327,708, bytes from 2.29 GB to
934 MB, arena blocks from 2,205 to 905. The checksum is unchanged and the
decode board is untouched. Welfare 46.33 -> 53.17.

The last link was `escape_able`, where the byte builder passes through
`list/fold bs acc (a b -> esc_byte a b)`. A fold is an accumulator loop with
the recursion written by somebody else, so the analysis now reads one: the
result is unique when the seed is and the folding function returns unique from
a unique first argument, and inside a folder that passed that test the lambda's
own parameter counts as unique. The scoping is deliberate — the name is put in
scope only under a lambda whose accumulator arrives first and appears nowhere
else, so a lambda that captures or reorders its accumulator never gets it.

WHAT THE SPEC FOUND. Writing the adversarial case — a lambda that is not a
folder, calling the same function — turned up an existing divergence instead.
An imported builder held by two owners was written through one of them:

    base = text/append (text/bytes "") "A"
    x = text/append base "B"
    y = text/append base "C"

    interpreter  AB AC
    native       ABC ABC

Importing a qualified name enrolls a bare-named clone so both spellings
dispatch, and the clone carries the original's span. Nobody calls the clone by
its bare name, so every call-site test over it is vacuously true and it keeps
the linear parameter its twin was refused. In-place sites are keyed by source
position, which the twins share, so the clone's verdict reached the code the
real name emits. `push` was never exposed to this: it is decided at the site,
not by a parameter, and the same program written with lists was correct
throughout.

The fix makes synthetic clones invisible to the analysis, which is what their
declaration already claims about provenance.

WHICH GUARD IS LOAD-BEARING. Four call sites enumerate declarations, and
reverting any one of them alone left the specs passing — redundancy reads as
sufficiency if you stop there. Applying one at a time to the broken baseline
instead: skipping clones when marking sites fixes it alone, skipping them when
seeding candidates fixes it alone, and the other two fix nothing. Only the
first two shipped. The call-site scan in particular was reverted on principle —
excluding a caller removes evidence rather than permission, which is the
direction that hides the next bug rather than the one that catches it.

WHAT THE OBJECTIVE MISSED. `emitted_lines` rose 1,893 -> 1,898 in #322 and no
gate fired, because at a weight of 0.10 the move is worth 0.006 points and the
ratchet ignores anything under a hundredth. That was a real regression bought
on credit: #322's wrapper inlining is what made the mutate path reachable, and
the payoff arrived today rather than then. welfare.py now names every term that
went backwards and prices it, whatever the sum did. The sum stays the gate.

## 2026-07-26 — the shelf's prize, measured

kq pretty-printing the 1.9 mb document allocates 135,762,784 bytes and peaks at
139,903,456. The two agreeing says nothing is reclaimed for the length of the
run. It does not say what a rewind would save, and the first number written
here — 97%, from allocation over output — answered the wrong question: the
decoded document is live for the whole run, and no analysis is going to change
that.

The split that does answer it costs one more measurement. A path query on the
same file decodes the same tree and prints 2.6 kb, and peaks at 52,543,848. The
difference from the full pretty-print, 87.3 mb, is what encoding adds; the
output being built is 4.6 mb of it. So 82.8 mb — 59% of peak — is encode
garbage, and a decode side that no shelf can shrink is the other 38%.

Peak scales with the document rather than with the output, and the multiplier
rises with nesting — 28x the input on a flat 845 kb list, 71x on the nested
1.9 mb one — which is what a grow-only arena does when every nested call's
temporaries outlive it.

The 62% allocation cut above moves the peak from 211.9 mb to 139.9 and does not
touch this: fewer allocations still means nothing freed. What closes it is the
fold-state shelf, and one of its four open questions is now half answered.
Hoisting a builder out of the arena for a loop's duration is sound only if the
loop owns it outright, and proving that is what the linearity analysis already
does to license an in-place append. The same proof read over a longer span is
what would license the longer lifetime; what is still open is the exit — who
frees the buffer when the loop ends, what happens on the failure path, and what
happens to a builder that escapes.

Not built. The measurement is recorded so the next attempt starts from the
number rather than from the hunch.

## 2026-07-26 — what the beat report says about the encode path

`KANSO_BEAT_REPORT=1` over kq, which is the compiler explaining its own
refusals rather than a guess about them:

    encode_items/3   grow-only: argument 1 may carry heap across the iteration
    encode_pairs/3   grow-only: argument 1 may carry heap across the iteration
    indent_onto/2    grow-only: argument 1 may carry heap across the iteration
    list/fold_go/3   grow-only: another group tail-calls it (unbracketed entry)
    skip_ws/2        pure loop: no iteration allocates, nothing to rewind
    list/bisect/5    beat: rewinds every iteration

Argument 1 is the byte builder in all three encode loops, and the reason is
narrower than "bytes are ineligible". A *bare parameter* carrying bytes is
already eligible — bytes are an immutable payload and one that arrived at entry
lives below the mark. What these loops pass is `(elem_onto acc xs[i])`, and an
expression's result is assumed to be newly allocated.

Since the header work it usually is not. An in-place append returns the very
pointer it was handed, so on the fast path the value crossing the boundary is
the entry-threaded one. The grow path is what stops that being a rule: it
allocates a fresh buffer through `k_alloc`, above the mark, and returns a fresh
header. kq takes it 18 times in a run against 2,217,224 fast appends. Rare, and
rare is not never — a rewind after one grow frees the buffer the next iteration
appends into.

So the shelf's runtime half is specific: for a builder the loop owns, both the
header and the buffer have to live outside the arena, and then the grow path
keeps them there. The open question is not the copy any more, it is the exit —
who frees a malloc'd buffer when the loop ends, what happens on the failure
path, and what happens to a builder that escapes. `list/fold_go/3` carries a
second reason (an unbracketed entry) and would need that fixed too.

## 2026-07-26 — the carry path cannot be the shelf, measured

Built a throwaway probe to find the shelf's ceiling before building the shelf:
byte-builder headers and buffers allocated with malloc instead of from the
arena, `k_survives` extended to answer yes for a pointer in no arena block at
all, and the BYTES exclusion lifted from the carry gate. All three behind an
environment variable, none of it committed.

The compiler took it. All three encode loops turned from

    encode_items/3: grow-only: argument 1 may carry heap across the iteration

into

    encode_items/3: carry beat: rewinds every iteration, evacuating argument 1

and the output stayed byte-identical to jq. Then the measurement:

    input      baseline        probe
    189 kb      13.9 mb      6,795.7 mb
    204 kb       7.0 mb      1,349.5 mb
    845 kb      24.1 mb     21,060.8 mb
    1.9 mb     139.9 mb      killed (out of memory)

Four hundred and ninety times worse, with the right answer. The counters say
where it went: `append_grow` 14 -> 28,126 and allocated bytes 12 mb -> 6.5 gb on
the 189 kb document.

The cause is one line in the copier. `k_deep_copy` sets `nb->cap = 0` on a
copied byte value, so a builder that has been through a carry arrives with no
spare capacity; the next append fails `if (a->cap)` and grows, allocating twice
the current length. Every carried iteration is then a copy followed by a
doubling, which is the quadratic the BYTES exclusion was written to avoid — and
it is not a tuning bug, since exact capacity is what stops a copied constant
being mutated in place by `push_mut`.

So the carry path is the wrong mechanism, and lifting its exclusion is not the
missing step. A hoisted builder has to be invisible to the carry rather than
carried cheaply by it: outside the arena for the loop's whole duration, never
staged, never copied at a boundary, with one copy out at `k_beat_pop` where the
runtime already does exactly that for a heap result. The probe also showed the
exit is not optional — it leaked every buffer, and that alone killed the 1.9 mb
run.

Recorded as declined-as-built rather than open: the carry route is closed.

## 2026-07-26 — the other half of kq's peak, sized

The shelf is worth 59% of kq's footprint. The remaining 52.5 mb is the decoded
document, which is genuinely live, so the question there is not when to free it
but what it costs to hold.

The 1.9 mb document is 264,541 json nodes. Decoding it allocates 1,003,447
times for 50,729,968 bytes across 49 arena blocks, and peaks at 52,543,848.

    198.6 bytes per node
    3.8 allocations per node

jq holds the same document, its own machinery, and its output in 30.7 mb — about
116 bytes per node — so kanso's representation costs roughly 1.7x jq's per node.
The allocation count is the more suspicious of the two: a scalar node should
cost one allocation and a container two, and 3.8 says something is allocating
per field or per element beyond the node itself.

Not investigated further. Recorded as the next target after the shelf, with the
numbers to beat written down.

## 2026-07-26 — a string is one allocation, not two

`k_str_n` allocated a KStr header and then its payload, two calls with one
lifetime, always adjacent. They are now one: `k_str_alloc` takes the length,
allocates `sizeof(KStr) + len + 1`, and points `data` at the byte after the
header. Four sites make a string from bytes they just copied and all four go
through it. Three others adopt storage they did not make — the zero-copy finish
hands a builder's buffer out in place, a deep copy shares a payload that
survives — and they assign `data` themselves, so they are untouched by this and
have to stay that way.

    decode allocations      12,924,473 -> 11,353,217   (-12.2%)
    encode allocations      26,327,708 -> 22,130,027   (-15.9%)
    kq, 1.9 mb document      4,617,083 ->  2,931,838   (-36.5%)
    welfare                      53.17 ->      54.74

Allocated *bytes* do not move, which is the tell that this buys calls and
alignment slack rather than storage.

WHAT IT DOES NOT BUY. kq's peak goes 139.9 mb to 139.4 — four tenths of a
percent against a 36.5% cut in allocations. That is the grow-only arena stated
a second way: nothing is freed for the length of the run, so how often the bump
pointer moved is close to irrelevant to how far it got. The shelf is still the
only thing that moves that number, and this is a useful negative control for it.

The mem corpus moved everywhere, which is the point of having it: 89 -> 48 on
append_in_place, 127 -> 74 on build_cycle, 7 -> 4 on the small lazy programs.
`string_headers.kso` is the new pin, at 117 allocations before and 65 after.

## 2026-07-26 — a record's fields live after its header

The same shape as the string above, one value type over. `k_rec` allocated a
KRec and then its field array; a record's field count is fixed at construction
and `set` writes through the existing slots, so there is nothing to grow and the
fields can sit immediately after the header. The deep copier builds its own
storage and assigns `fields` itself, which stays correct.

    encode allocations   22,130,027 -> 18,785,627   (-15.1%)
    kq, 1.9 mb document   2,931,838 ->  2,848,228   (-2.9%)
    welfare                   54.74 ->      55.51

Decode does not move — the decode bench builds no records. `record_fields.kso`
pins it at 114 allocations before and 64 after.

Taken with the string merge, encode allocations are down from 26,327,708 to
18,785,627 in one sitting, and kq's from 4,617,083 to 2,848,228 — 38% fewer
allocations for the same bytes. kq's peak moved 139.9 mb to 139.3, which is the
third measurement this evening saying the same thing: the arena's high-water
mark is set by what is never freed, not by how often it was asked.

## 2026-07-26 — where kq's decode memory actually goes

Clay: "the current memory numbers on kq became an absolutely embarrassing
explosion... something is majorly wrong there." He is right, and the shelf is
not the whole of it. Decoding the 1.9 mb document — before a byte of output is
built — allocates 50,729,968 bytes and peaks at 52.5 mb.

What the decoded document actually needs, counted from the document itself:

    104,750 strings, 7.0 bytes average          3.35 mb
    27,610 maps, 83,610 entries                 3.56 mb
    27,511 lists, 97,320 items                  2.00 mb
    ------------------------------------------------
    live                                        8.91 mb

So the representation costs 5.7x what the data occupies. Named counters on the
allocation sites say where:

    c_buf   (map pairs + list items)      30,672,512
    c_str   (headers and payloads)         6,261,216
    c_map   (headers, one per put)         2,675,520

The pairs and items themselves are 4.3 mb of that 30.7. Seven times over, and
the multiple is not a representation problem — it is geometric growth against
an allocator that reclaims nothing. A collection grown by repeated push passes
through every intermediate capacity, and each abandoned buffer stays in the
arena for the life of the run. A freeing allocator pays for the final buffer;
this one pays for the sum.

TRIED AND FAILED: extend at the frontier. When a grown buffer is still the
arena's newest allocation, the bytes after it are unclaimed, so it can grow
where it sits — no copy, no abandoned buffer. Implemented for both the map put
and list push grow paths, output byte-identical to jq, and `c_buf` did not move
one byte: 30,672,512 before and after. The reason is structural rather than
incidental — between two pushes onto the same collection the parser allocates
the next element, so the buffer is never the newest allocation. The trick can
only ever help a loop that allocates nothing but the collection, which is not
what parsing is. Reverted.

That leaves two routes, and they are the same question Clay asked about
refcounting. Either the abandoned buffer gets freed, which needs an owner, or
growth stops copying: a chunked buffer that adds a segment instead of doubling
never abandons anything, and its total is the live size plus one partial chunk.
The second needs no ownership story at all and would take c_buf from 30.7 mb to
about 4.5. It costs an indirection on element access, which is a real price on
the hottest path in the decoder and has to be measured rather than assumed.

Also noted while measuring: `put` has no `put_mut`. Lists get in-place
extension where the analysis proves uniqueness — thirteen sites in kq — and
maps never do, so every put allocates a fresh 32-byte KMap header even on the
O(1) frontier path. That is the 2.7 mb above, and it is the same shape as the
append header the linearity work removed.

## 2026-07-26 — the decode anomaly, localised

Counting buffer *allocations* rather than bytes finds the thing the growth
model could not explain. Decoding the 1.9 mb document:

    c_buf     30,672,512 bytes
    n_buf        228,283 allocations
    average          134 bytes each

The document holds 55,121 collections, so that is **4.1 buffers per
collection**, on collections averaging three entries. One or two is what
growth from an initial capacity of four should cost. Four means the buffer is
being reallocated on almost every insertion, so the O(1) frontier path in
`put` and `push` is mostly being missed and the copying path is the norm.

A predicted total from the growth policy alone comes to 10.2 mb against the
30.7 measured, and this is the missing factor of three.

The sorted-view hypothesis is ruled out: 17 calls during decode, 2,440 bytes.
On the full pretty-print it is 27,610 calls and 3.3 mb — one view per map,
built on read, real but not the anomaly.

The frontier path in `put` is guarded by `buf->used == m->len * 2`, which asks
whether this map is still the newest writer into its shared buffer. Something
in the parser is breaking that condition on most insertions. That is the next
thing to find, and it is a decoder-shaped question rather than an allocator
one — the allocator is behaving exactly as written.

## 2026-07-26 — the push accounting, and where the anomaly is not

Instrumenting both insertion paths separately turns the buffer anomaly into
something specific. Decoding the 1.9 mb document:

    mut_fast   218,831   push_mut extended in place
    mut_fall    82,781   push_mut found the buffer full and fell through
    push_fast        0   and the fallback's own frontier check never fires
    push_copy   82,781   so every fallthrough copies
    put_fast    55,620   put claimed the next pair slot in place
    put_copy    27,990
    n_buf      228,283

Three things this settles.

The uniqueness analysis is working. 218,831 of 301,612 pushes — 73% — extend a
list in place, which is the whole point of `push_mut` and it is doing its job.
`push_fast` reading zero is not a second bug: `k_b_push` is only ever reached
after `push_mut` has already found the buffer full, so its own frontier test
must fail. Two counters describing one condition.

Per-collection behaviour is correct at small sizes. `[[1 2 3 4 5]]` costs
exactly six pushes for six items, with two grows where doubling from four
predicts two. Nothing is pushing twice.

And yet the big document takes **301,612 pushes against 97,320 list items**,
three times more insertions than there are things to insert. Whatever that is,
it is scale-dependent rather than a per-collection defect, and it is the next
thing to find: 82,781 of those pushes hit a full buffer and allocate, which at
134 bytes average is most of the 30.7 mb.

Ruled out on the way: sorted views (17 calls during decode), the growth policy
alone (predicts 10.2 mb against 30.7 measured), and frontier extension (built,
byte-identical output, `c_buf` unchanged to the byte — the parser allocates
between insertions, so a collection's buffer is never the newest allocation).

Still standing from the earlier entry: `put` has no `put_mut`, so all 83,610
insertions allocate a fresh 32-byte header even on the O(1) path.

## 2026-07-26 — the decode anomaly named: strings after their first escape

The 3.1x push count has a cause, and the arithmetic closes on it. Of the
document's 104,750 strings, 19,440 — 19% — contain an escape, and 227,780
characters sit after a first escape. Add the 97,320 list items and the
prediction is 325,100 insertions against 301,612 measured, within 7% with the
estimate running high.

kq scans a string with `text/find2` for the next quote or backslash, and a
clean string is sliced whole — that path is right. What happens at the first
backslash is the problem: `string_at cs 92` converts the clean prefix to a byte
list and hands it to `str_chars`, whose catch-all arm pushes **one character**
and recurses. Everything after the first escape in a string is decoded a
character at a time into a growing integer list, and those lists are most of the
228,283 buffer allocations and the 30.7 mb behind them.

TRIED AND FAILED: make the catch-all scan the run instead. One arm — find the
next quote-or-backslash, `text/concat acc (text/slice cs p (n - 1))`, recurse at
n. Output stayed byte-identical to jq and allocation went the wrong way:
50,729,968 -> 61,025,168 bytes, peak 52.5 mb -> 62.8. `text/slice` allocates the
run and `text/concat` allocates the result, so a run costs two allocations of
its own length, and in a document escaping every few characters the runs are too
short to pay for that. Reverted.

What the measurement actually asks for is an append that takes a source range:
copy `cs[p..n]` onto the accumulator with no intermediate value, the decode
mirror of what `text/append` already does on the encode side. Then a run costs
one memcpy and no allocation, and the per-character path disappears without the
slice-and-concat tax that replaced it.

Worth stating plainly: this benchmark document escapes 19% of its strings, which
is high for real json. A clean corpus would show less of this and more of the
buffer growth measured earlier — the fix is still right, the multiplier on it is
document-shaped.

## 2026-07-26 — the escape fix lands in kq; the shelf split moves

kq's escaped strings now decode through a byte builder instead of a list of
ints (kq#26). The diagnosis said most of the 228,283 decode buffers were
per-character pushes after a first backslash, and the fix confirms it:

    decode peak         52.5 mb -> 32.5   (jq holds 29.2)
    full peak          139.3 mb -> 119.4  (jq holds 30.7)
    decode allocations  874,393 -> 702,043
    instructions        680.4m  -> 641.6m  (3.65x less than jq, was 3.44x)
    pretty-print wall   35.4 ms -> 35.0

On the 188 kb path query kq's footprint is now smaller than jq's, 4.2 mb
against 4.8. The failed slice-and-concat attempt from the diagnosis is
superseded: the builder appends the clean prefix in one copy and each escape
byte in place, so no intermediate value exists and no range-append primitive
was needed after all.

The shelf split re-measured: encoding adds 86.9 mb to a 32.5 mb decode peak,
output 4.6 of it, so encode garbage is 82.3 mb — 69% of peak now that the
decode side stopped inflating. The shelf is a larger share of a smaller
number, and it is now nearly the whole story: a working shelf would put the
full pretty-print near 37 mb, jq territory.

## 2026-07-26 — put joins push and append: unique maps extend in place

The last mut-less insertion verb. `put` allocated a fresh 32-byte KMap header
on every call, even on the frontier path that claims the next pair slot in
O(1) — the diagnosis measured 83,610 of them decoding the 1.9 mb document.
`k_b_put_mut` carries the new length in the existing header where the
linearity analysis proves the map moved, and a map literal now counts as a
unique source, which it always was — `{:}` is born fresh exactly as `[]` is.

The sorted view resets on the mut path: it is per-header state and the header
now describes different contents. put never builds one, a later read rebuilds
it, and the differential suite is what checks this reasoning rather than the
prose.

    decode allocations   11,353,217 -> 10,518,917
    kq decode            702,043 -> 646,423   (the 55,620 fast-path puts)
    welfare                   55.51 -> 55.95

WHAT THE FIRST SPEC MISSED. The aliasing spec's first draft held a map with
exact capacity, and it passed even under a deliberately unsound analysis —
both aliased puts fell to the copy path because there was no slack to claim,
so the mutation the spec existed to catch could not physically happen on its
input. The dangerous shape needs a map that has grown once (growth doubles, so
slack exists), and only the strengthened input goes red when the uniqueness
check is stripped. Watched failing both ways: the mem pin at 152 allocations
against 107 on the old runtime, the alias spec under the stripped analysis.

## 2026-07-26 — the ratchets go aggressive, and the declare fix pays the debt

Clay, twice: emitted lines went up and nothing stopped it. He is right that a
test should have — the ratchet's design allowed any move under a hundredth of a
point, and two such moves stacked. The fix is not a re-pricing.

**The lines came down instead.** Every program carried all 93 runtime declares
whether it called them or not. The emitter now keeps a declare only when its
symbol appears in the body or in one of the preamble's own inline definitions:
1,903 -> 1,488 emitted lines, 27% below the original 1,893 baseline, and the
compile golden drops about 83 lines per sample. Welfare 55.95 -> 56.48.

**The welfare gate is two-sided now.** It failed only on falls; a rise nobody
banked was a rise the next change could spend, and the banking was manual
discipline. An unheld gain above the floor is now itself a CI failure, so the
floor rises the moment an improvement merges and every regression thereafter is
measured against the best the project has ever been. That is what "the number
only goes up" means when a machine enforces it.

**kq gets its own ratchet** (kq#27). Its counters were checked by hand, which
is how the decode anomaly sat unnoticed until Clay called the footprint an
embarrassment. Two deterministic cost goldens now gate kq's CI beside the jq
byte-identity check, so the next kanso change that moves kq's memory fails a
build instead of waiting for a hand measurement.

kq itself, rebuilt on put_mut: full peak 117.8 mb, decode peak 30.9 against
jq's 29.2. The remaining 87 mb over jq is the shelf, which is the open front.

## 2026-07-27 — the fold-state shelf ships: the encode loops rewind

The largest unclaimed number on the compiler page is claimed. Three pieces,
each necessary, none sufficient alone.

**The license is identity, never type.** A bytes accumulator may cross a
rewind when it is the very object that arrived at the loop's entry, threaded
through mut-selected appends — pointer identity, proven by a greatest
fixpoint over groups whose every arm returns a chain of its first parameter,
through conditionals, guards, local bindings, folds whose folder chains its
own accumulator, and calls to other chaining groups. The header is then below
the mark and growth (since the malloc change) is outside the arena. Raw bytes
hold no pointers, so nothing in the accumulator can dangle. A fresh builder
of the same type has none of these properties: fresh_builder.kso pins the
refusal at beat_iters=0, and under a deliberately type-only license it reads
40 with output still correct by memory-layout luck — which is why the counter
is the pin and the output is not.

**Sorted views join builder buffers outside the arena.** The first licensed
run took 362 ms against a 5 ms baseline: every rewind swept the cache
registry and freed every map's sorted view, so each iteration rebuilt them —
the cost the declined cache-sweep entry measured, reproduced exactly. Views
are malloc'd now, nothing arena-backed can dangle, nothing registers, and the
sweep walks an empty table. put_mut frees the view it invalidates. A
transient map's view leaks with the map; recorded as the trade.

**The numbers.** encodebench arena blocks 905 -> 5 with 5,032,001 rewinds —
the encoder runs in constant arena space. kq, interleaved, best-of sitting:

    full peak, 1.9 mb   116.1 mb -> 77.3   (was 211.9 this morning; jq 30.7)
    wall, full 1.9 mb    35.4 ms -> 34.2   (3.02x jq, best yet)
    cycles vs jq           3.11x -> 3.15x
    ipc                     4.72 -> 4.90
    path-query peaks    at or below jq on both documents
    welfare                57.11 -> 60.25

**Found on the way, left open:** loops inside imported subdirectory modules
never beat — the decl is qualified, the recursive call is spelled bare, and
has_self_tail sees two names where there is one function. The same duality
#323 fixed in the linearity analysis, one analysis over. kq's top-level files
dodge it; VSE-shaped programs will not. Tracked as the next win.

## 2026-07-27 — the page catches up to the shelf

Section 08's fold-state entry rewrites from queued-and-measured to shipped,
present tense, with the chronology left here where it belongs. The kq figures
across the page move to the post-shelf sitting (77.2 mb, 4.98 ipc against
jq's 4.74, 33.6 ms), and the retired-instructions bullet keeps its original
gap and gains its closing — the same instrument told both halves. kq's own
README moved in kq#28 with regenerated cost goldens, since its CI builds
against kanso main and the old counters no longer describe it.

## 2026-07-27 — a literal builds once

The shape counters' first find on their first day: 49.8 mb of the strings kq
allocated while pretty-printing were the same few literals — ",\n" and a
two-space indent — re-materialized on every evaluation, about 42 bytes per
loop iteration across 1.2 million iterations. The rewinds reclaimed them, so
the peak barely noticed; the allocator and the cycle counter paid full price.

k_str_lit builds a literal once into a permanent slot and hands it back
thereafter — the ascii single-character cache generalized, one slot global
per interned literal.

    encode allocations   18,774,865 -> 16,274,462
    kq allocations        2,600,800 ->  1,066,528   (-59%)
    kq wall, best-of-12     33.1 ms -> 28.1         (3.64x jq)
    welfare                   60.25 -> 60.82, banked

Peak moves half a megabyte; this one was churn, and the wall clock is where
churn shows.
## 2026-07-27 — one function, one spelling: module loops reach the beat tier

Enrollment gives every imported pub a bare-named twin so both spellings
dispatch, and the twin cost the analyses their footing: a module's internal
recursion read as a call between two groups, so has_self_tail failed and no
loop inside an imported module could ever beat — std/list's own fold_go
reported "another group tail-calls it" about its own recursion. The same
two-names-one-function duality #323 fixed in the linearity analysis, one
analysis over.

Where a bare name's whole group is clones of a single qualified origin, the
program now canonicalizes after inlining: every bare reference rewrites to
the qualified spelling and the clones go. A bare name with local arms is a
real overload union (the import-incarnation gavel) and is left alone. A name
that is ever locally bound is also left alone, because an occurrence may mean
the local — kq's pretty-printer found that one at runtime, where a local
named `first` was rewritten to list/first and a closure arrived where a byte
accumulator belonged. The exclusion is program-wide and over-broad by design:
a skipped alias keeps its clones and the old dispatch, which is correct and
merely unoptimised. Root-file locals are the exposed case; dep locals were
already safe because qualification renames them.

The beat tier's provenance filter narrowed to its own rationale. It dropped
every group whose name contains a slash — with canonical names that is all
module code, the very thing being enabled. The carry-cost concern it encoded
applies to carried groups, whose evacuation copies per iteration; a plain
beat's rewind is a pointer reset. Imported groups now stay out of the carry
tier only, and groups with a surviving synthetic arm stay out of everything.

Pins: a module loop threading a builder is licensed and in ids
(a_module_loop_is_one_group_with_one_spelling, red before the fix), and a
root-file local sharing an importable's name is never rewritten
(a_locally_bound_name_is_never_rewritten, red with the exclusion stripped).
Benches, kq, book, site: unchanged to the byte.

## 2026-07-27 — the doubling trail goes to a shelf

A collection grown by repeated push passes through every intermediate
capacity, and an arena that reclaims nothing pays for the sum — the decode
diagnosis measured the trail at three times the live data. The trail now pays
for itself: a mut-site grow donates its outgrown buffer to a size-classed
shelf, k_buf takes from the shelf before the arena, and rewinds flush it so
no entry outlives the region it sits in. Uniqueness is what makes the
donation sound, the same proof that licensed the mutation.

    decode allocations   10,518,914 -> 8,341,217   (516,750 reuses)
    kq decode peak          30.0 mb -> 27.3        (jq holds 29.2)
    kq full peak            50.3 mb -> 48.1
    kq large-doc peak        5.5 mb -> 5.1         (jq holds 5.0)
    welfare                   60.82 -> 61.66, banked

put's mut path grows in place and donates like push's, and grow capacities
land exactly on shelf classes — the first cut rounded up past them and paid
peak for it, which the measurement caught the same hour. The decode-peak row
now reads in kq's favor by two megabytes. The buf_reuse counter pins the
mechanism in every vein.
## 2026-07-27 — pop-rewind for chained loops: built, measured, declined

The theory: k_beat_pop keeps a heap result's region alive, so every nested
container completion strands its final iteration's garbage — thousands of
small strands. A chain-licensed loop's result is its entry accumulator, so a
k_beat_pop_chained could rewind at close by the same identity argument.

Built it (runtime pop variant, Beats.chained, emission at licensed entries),
verified all 8 kq entry sites emitted it, and measured zero — kq's peak did
not move a tenth of a megabyte, with or without, before and after the pretty
loops joined the beat tier. The strands the theory predicted are either
reclaimed by the next outer iteration in practice or too small to see.
Reverted whole.

What DID move the number, found the same hour by instrumenting reclaimed
bytes per rewind: kq's pretty printer looped through two-group tail cycles
(pretty_elems -> elem_row-helpers -> pretty_elems), which the chain license
never reaches — it reads self-tails, and the cluster tier still excludes
bytes. The helpers existed only to fit the width canon, and gavel BB's return
guards express the same loops as direct self-tails within the cap. That is a
kq-side fix (kq#29); the compiler-side thread — extend the chain license to
tail cycles — stays open, and VSE-shaped mutual recursion is who it is for.

## 2026-07-27 — gentle builder growth: built, measured, declined

Doubling a large builder leaves as much slack as content, so growth past a
megabyte was gentled to five-fourths expecting a three-megabyte peak cut. It
measured 7.7 mb WORSE: 47.5 -> 55.2 on the big document. Each mut-grow briefly
holds old and new buffers together, and quarter-spaced sizes defeat the
allocator's reuse of the freed extents — a freed 4.6 mb cannot hold the next
5.8 — so the dead large blocks stack up where doubling's halves had recycled.
Reverted within the hour; byte-identity held throughout. (This entry re-lands:
its first push raced the shelf PR's merge and lost.)

## 2026-07-27 — utf8_mut: built, measured zero, declined

The theory: a utf8 finish on a moved builder copies out and leaks the
malloc-backed buffer, nineteen thousand times per kq decode. Built the mut
variant end to end — runtime free on the copy path, linearity marking,
codegen selection — and it selected zero sites on kq and zero on encodebench,
because the premise was wrong twice over. The finishes in question take the
zero-copy path, which hands the buffer to the string as its payload: the
storage is live, not leaked, and there is nothing to free. The utf8_zerocopy
counter had been read across two different builds, and the stale reading is
what made a leak seem to exist. Reverted whole; the counter comparison that
would have caught it in one step is the one this log now records.

## 2026-07-27 — the full-print footprint, accounted to the megabyte

A leaks census (192 unreachable bytes — the runtime does not leak) and a
mid-run vmmap close the decomposition of kq's remaining 47.5 mb:

    arena high water        ~26 mb   decode-side garbage the scan loops hold
    encode builder           9.2     4.6 of output, 4.6 of doubling slack
    sorted views             3.3     malloc'd memos, live until exit
    shared library code      ~6      every process pays it; jq pays the same
    allocator retention      ~2      one cached empty MALLOC_LARGE region
    metadata and guards      ~1

Nothing is unattributed. What each remaining piece needs:

The arena's 26 against ~9 of live data is the decode garbage, and it is the
one that matters. The scan loops thread lists and records — values that hold
pointers — so the bytes-only chain license cannot reach them, and reclaiming
mid-decode is exactly the cohort-freeing / value-RC question already parked
dialog-first. The measurement gives that dialog its number: seventeen
megabytes on this document, the whole distance between kq's 47.5 and the
low thirties.

The builder's 4.6 of slack resists the easy fix — gentler growth measured
7.7 mb worse. What would work is a final-size hint from the encoder (the
input's size bounds the output's within a small factor), which is a stdlib
surface question rather than a runtime trick.

The views' 3.3 dies with RC or with a rebuild-per-read encode mode that
trades a second sort per map for the residency. Declined for now: the wall
clock is the row kq wins hardest, and 3.3 mb is not worth taxing it.

## 2026-07-27 — the chain license reads around a cycle

kq#29 rewrote the pretty printer's two-group tail cycles as direct self-tails
because the chain license only read self-recursion; the compiler now reads
the cycle itself. A bytes slot in a tail cluster may cross by pointer
identity when every inner edge feeds it a mut-append chain rooted at one of
the caller's own chain-threaded slots — the same greatest fixpoint as the
threaded-slot rule, one license over. Chain-threaded slots cross without
being carried, so a bytes-accumulator cycle becomes a plain rewinding ring;
bytes slots that fail the fixpoint still refuse the cluster whole, as before.

Pins in the mem corpus, red-before-green both ways: beat_cycle.kso (the old
kq shape) rewinds 400 times in one arena block, fresh_cycle.kso holds at
zero, and under a deliberately type-only fixpoint it reads 80 against the
pinned zero. Every golden elsewhere is unchanged to the byte — kq no longer
needs its rewrite, and programs shaped like it no longer need to know the
trick.

## 2026-07-27 — the enumerable fills in: zip, argmax, argmin, group_by, transform_values

The ratified vocabulary (design/enumerable.md) was closer to built than its
own status line said — twenty-three verbs existed with fusion; eleven were
missing, and four of those wait on open gavels (range's name collision;
to_h / index_by / transform_keys, which all hang on the one map-collision
policy). The other five land here, spec-first, in the house shapes: zip is a
paired iterator with its next and fold arms — the lazy pull proven by zipping
naturals against a two-element list — argmax and argmin ride the missing-
shape fold like max and min, group_by never collides so it needed no gavel,
and transform_values folds entries through put.

Found on the way, for the deferred list rather than silent fixing:

- the spec's naming law says "all/any, no ? suffix" but the compiler
  enforces the opposite — error[naming] requires ? on predicates, and the
  shipped verbs are all?/any?. The spec drifted from a later naming gavel;
  the document needs the amendment, not the code.
- indexing a map by a bool key is a runtime error while putting one is not —
  group_by with a boolean key function hits it. The asymmetry between put
  and at wants one answer.

Every consumer dispatches per shape, so a new shape must bring its fold arm:
zip's first draft forgot and the catch-all treated a paired as a plain list.
The differential harness caught it in one run, both engines agreeing on the
same wrong error, which is the system working.

## 2026-07-27 — two inference holes at the lambda call head, found by one book sample each

The four verbs waiting on interim committee rulings (range, to_h, index_by,
transform_keys) went in cleanly. What made the branch expensive was the pair
of latent inference bugs the differential law surfaced on its book samples,
each one a different way for the fixpoint to be wrong about a lambda in call
position.

First: `eval_call` never walked a lambda head at all — it returned TOP for
the call and moved on, so every call inside an applied lambda's body was
invisible to the param fixpoint. A callee reached only from such a body kept
whatever narrow set its other callers gave it; squeeze's `measure`-shaped
chain proved a list-taking function int-only, the ABI unboxed it, and a list
pointer crossed as a smuggled i64. Silent wrong answers, native only.
Pinned by tests/golden/micro/redex_callee_widens.kso (baseline prints
"11 1" for "7 1" — watched red) and fixed by stepping the beta: bind the
argument sets to the params and walk the body, which is also what the fusion
pass's emitted redexes needed.

Second, exposed by the first: the pipe's set semantics dropped DESC. A pipe
whose left side may be a description substitutes the yield into the
continuation's param — correct — but the pipe expression's own value at
runtime is then a description (k_maybe_bind's chain), and the result set
said otherwise. Under TOP-returning betas nobody noticed, because TOP
carries DESC; once the beta step returned precise body sets, a program
whose io chain runs through function results — vse — inferred `trials`
desc-free, and the set-gated pipe emission in codegen direct-called `means`
on a raw description. sums[1] then indexed a desc: "at takes a list...".
Pinned by tests/golden/micro/pipe_desc_chain.kso (dies red without the
DESC bit) and fixed by riding DESC along in piped_bits.

The lesson worth the price: a set that is merely narrow is a performance
bug, but a set that is wrong is a correctness bug, and the difference is
whether every consumer of the sets treats them as over-approximations.
The pipe gate does — it only trusts "can't be a desc" — so the fix belongs
in inference, making the promise true, not in the gate.

Measured: encode arena flattens 5 -> 4 blocks (beat_iters +400 — one more
loop licensed), decode unchanged, kq spec suite and both its ratchets green
byte-identical, welfare 62.22 -> 62.23, banked.

## 2026-07-27 — a fused reducer stops paying per element: redexes now emit inline

Two ledger sweeps and one build. First the sweep that closed a stale thread:
the shelf campaign left "tail cycles stay grow-only" as the next structural
win, and measuring it found it already built and pinned — a fresh 200k
even/odd mutual-tail chain beats every iteration at constant allocs, matching
the beat_cycle.kso pin (beat_iters=400) that landed with the cycle work. vse
itself runs at arena_blocks=1, beat_iters=20000. The memory saying otherwise
was older than the code.

Then the measurement that turned into the build. The enumerable's fusion
pass composes an adapter chain into one fold whose reducer nests the steps
as applied lambdas, and the emitter treated every applied lambda as a value:
build the closure, dispatch through k_callN. For `sum (map xs f)` over 100k
elements that meant two fresh zero-capture closures and two dynamic calls
per element — 400,002 allocations over the flat-fold baseline, with
KANSO_NO_FUSE within noise of fused. The composition was real and the
flattening was missing.

emit_call_rest now steps the beta: a literal lambda head whose params match
the call arity binds its arguments as locals (save/shadow/restore on the
flat versions map) and emits its body inline, with failing arguments
short-circuiting through a phi first, exactly as k_callN would have. The
same chain now runs at the build-only baseline — zero allocations per
element — and the composed-reducer klam is gone from the IR entirely.
Pinned by tests/golden/mem/fused_reducer.kso: allocs=12 for a
1000-element map/select/sum; watched red without the inline step
(allocs=7912, "allocator counters drifted").

Everything else held still: 17 suites, book corpus, kq spec suite and both
its ratchets byte-identical, cost goldens and welfare flat at 62.23 — the
bench programs carry no redexes, so the ratchets rightly saw nothing.

STILL QUEUED from the enumerable spec: take/drop bounds in the fused scan,
first/find early-exit fusion, tally/group_by reducers — the remaining
400k excess in the four-chain measurement is exactly those unfused shapes.

## 2026-07-27 — tally and group_by join the fused consumers; the map log is quadratic

Two of the queued reducers were a small extension once the inline-beta step
landed: try_fuse now emits tally as `put m x (bump m[x])` and group_by as
`file_under acc (key x) x`, resolving the private std/list helpers by
qualified name the same way it resolves fold — privacy is a check-time
property and fusion runs after the check. Pinned by
tests/golden/mem/fused_tally.kso (allocs=1590 fused; 5118 without the
arms, watched red). Still queued: take/drop bounds and first/find
early-exit, which need a fold that can stop — a different mechanism, not
more arms.

OPEN, found by the measurement: building a map under repeated keys is
quadratic at runtime. k_b_put and k_b_put_mut both append to the pair log
unconditionally — an existing key still grows len by one — and every read
resolves duplicates by sorting the whole log. A 10k-element tally over 17
distinct keys allocates 2.0 GB and runs seconds; 30k runs half a minute;
the scaling is n². This is not the fusion's doing — KANSO_NO_FUSE times
within noise — it is the cost model of dedup-on-read meeting an
accumulator that writes the same keys forever. Any tally, index_by, or
put-loop over few distinct keys pays it. The fix wants design attention
(compact the log when duplicates cross a threshold? resolve on put_mut at
proven-unique sites?) because the frontier-append trick is load-bearing
for the build-block shapes that never repeat a key.

## 2026-07-27 — find, all? and any? stop at the answer

The queue said "first/find early-exit fusion," and the measurement said the
fusion half was the wrong frame: find was a full fold even on the lazy
cursor path — an allocating probe showed the predicate running on all 100k
elements for a hit at position one — and all?/any? folded the same way. The
fix is not a fold that can stop; it is not folding. The three verbs now
drive `next` directly and return at the answer, the way `first` always has:
find via found_in, the predicates via holds_all?/holds_any? (the naming rule
insisted on the suffix, correctly). seek and pick_found retire with the old
shape.

What this buys beyond the constant factor: the three consumers now bound
infinite generators, which appendix b already claimed — `find naturals
(n -> n * n > 50)` answers 8 where it used to run forever. list_test grows
four cases pinning that, and the mem vein gains early_exit.kso: allocs=25
with the exit, 40004 without (watched red), the probe being a predicate
that allocates per call.

Also closed: lib/list's own test file was never run in CI — the specs job
tested lib/json alone. Both libraries' suites gate now.

Fused-scan stop for take/drop remains the one genuinely queued mechanism,
and it is YAGNI until a measurement names it: take chains stay on cursors,
which are lazy and bounded already.

## 2026-07-27 — ledger item 6 measured: FBIP's runtime headroom here is thin

The queued pairing was FBIP (in-place as a guarantee) plus SpecConstr
(call-pattern specialization), and a static census grounds what they would
buy. In the emitted IR for both bench programs, the linear analysis already
selects the mut variant at 48 of 57 mutation sites — push 11/15, append
33/34, put 4/8 — and the cost counters say the hot loops all ride the fast
paths (append_fast carries the encode board; the plain sites are setup code
or genuinely shared values FBIP could not convert either). FBIP's value on
this corpus is therefore not speed: it is turning "discovered in-place"
into "stated and checked," which is ownership-annotation surface — language
design, so it waits for dialog rather than an autonomous build. SpecConstr's
concrete target is automating the enumerable's hand-written per-shape arms,
a maintainability win the enumerable_arms parity test guards today; it files
with the tracked compiler-refactor pass. The ledger entry stays queued on
the public page until Clay ratifies this reframing.

## 2026-07-27 — the diagnostics catalog catches up with the compiler

A census of Diagnostic::new sites and the error corpus found the compiler
speaking fifteen diagnostic kinds while appendix a cataloged nine. The six
missing families — arity, naming, none, build, ownership, runtime — each
gain a section with a worked sample run through the real toolchain, and the
closing enumeration now names all fifteen (thirteen never survive check;
runtime and endpoint are the residue only running reveals). arity had no
home in the error corpus either; arity_mismatch.kso adds one. The
exhaustive kind stays out of the catalog deliberately: it is gated behind
KANSO_EXHAUSTIVE, the none-campaign experiment Clay has not started, and a
catalog should document the surface a reader actually meets.

Found by the sweep and fixed in passing: length accepts a map everywhere —
appendix b documents it, the maps sample proves it — but its runtime
refusal said "a list or string". The message now names all three, on both
engines, with the wasm build refreshed.

## 2026-07-27 — chapter 12: sequences; the counters panels tell the truth again

The book taught every list verb and never taught what a chain is. Chapter 12
now does — recipes not results, the pull, consumers that stop, the map
builders, and the fused loop proven by a counters panel — appended after
modules the way ch11 itself was appended, so nothing renumbers. Clay may
re-slot or cut it; the loop's charter says improve the docs, and this was
the largest gap left.

Writing it surfaced two rot sites and one compiler gap. The rot: ch10's
counters panel had never been machine-synced (its path-prefixed title dodged
the panel matcher, and the counters-dump output had no sync rule at all), so
the page still showed `main =`, bare `sum`, and allocs=29 from an old
compiler — reality is `pub play`, `list/sum`, and allocs=12. book_panels now
recognizes a `KANSO_COUNTERS=1 kanso run x.kso` title and syncs it against
`x_counters.out`; both ch10 panels retitled into the machinery, and the
gauntlet panel's hand-kept numbers trued to the current golden (8,341,214
allocs, four arena blocks — the page had fourteen million and five).

The gap, measured: the pipe spelling of a chain does not fuse. try_fuse
matches `piped: false`, and rewriting `x . list/map f` blindly would break
the case where x is a description (the pipe is a bind there, not an
application). Nested spelling: 12 allocs for a thousand-element
map/select/sum. Pipe spelling: 5,975. The house style is the pipe, so the
gap is real and wants a set-aware fusion pass (fusion currently runs before
inference). Appendix b's fusion sentence was corrected to say what is true
today; the chapter's counters sample uses the nested spelling and claims
nothing about pipes. OPEN for design: either fusion learns the sets, or the
pipe-vs-nested cost difference stays and the book eventually has to explain
it — the first option is the one that keeps spelling out of the cost model.

## 2026-07-27 — ch11's ambiguity promise measured false; ties resolve by import order

Writing exercises for the two exercise-less chapters turned into a premise
check, and one premise failed. ch11 said "when a call is genuinely ambiguous
the compiler says so." MEASURED: two imports exporting the same
identical-shape arm resolve silently, first import winning — and since the
formatter forces imports alphabetical, the winner of a genuine tie is
decided by what the directories are named. check_arm_ties covers subtype
ties only (it returns early without subtype parents); nothing examines
cross-module generic ties. This is the tie-rejection dispatch rule already
pending gavel, now with a concrete motivating case: alphabetical-directory-
name-wins is not a semantics anyone would defend, and the book was
promising the error the gavel would create.

The chapter now says only what is verified — specificity picks, a local arm
outranks an import on a tie, a qualifier narrows — and its new exercises
walk the verified behaviors (arity dispatch across the boundary, local-
beats-import, opacity at the boundary, qualified reach-through). ch12 gains
exercises in the same premise-checked style, and ch09's claim that "the
builtins give it map, at" joins the qualified world. Every exercise premise
on both pages was run before it shipped; one drafted exercise died in
verification, which is the point of verifying.

## 2026-07-27 — the failure story diverges three ways, and only a gavel can close it

The book-premise sweep reached ch04 and found the flagship claim half-true.
"no arm can catch it" holds for ordinary arms: an err rides past the whole
overload group including the catch-all, exactly as unhandled.kso pins. But
an explicit `(err e)` pattern arm CATCHES — measured: `fn classify (err e)`
over `10 / 0` returns "caught division by zero" on both engines — while the
same chapter says "no function gets to turn one back into a value. not a
helper, not a library, not main."

The three positions on record disagree. The book teaches gavel B, the full
ban, as design/err-migration.md planned it (Clay-ruled 2026-07-19: err arms
become a compile error, one PR, library migrations enumerated). The machine
implements pre-B semantics: pattern_catches in infer and dispatch both
honor err arms, and lib code still carries them. And Clay's latest word —
seeing an err arm returning a plain value, "this should be impossible ...
oh, unless it crosses a package boundary. i was wrong" — reopens B with a
carve-out whose semantics need the package system that is not built yet.

Nothing here is implementable autonomously: the full ban contradicts the
retraction, and the carve-out has no definition of the boundary it names.
The book edit is deliberately withheld too — ch04's prose states the ruled
design of 07-19, and rewriting it to today's machine would document
semantics the gavel may be about to remove. The migration doc's plan is
ready the day the rule is settled. Until then this entry is the record that
the language's central promise is not yet one thing.

Also verified in the same sweep, all green: field-annotation refusal (ch03),
the endpoint rule (ch04), arity-short calls (ch05), the bare-line effect
check (ch06), and canonical form as grammar (appc, backed by the 26
formatting fixtures).

## 2026-07-27 — the landing page and the compiler page quoted a retired golden

The number-bearing-surfaces checklist exists because sweeps run on recall,
and here is this week's proof: index.html's landing panel and two passages
of compiler.html still said the gauntlet performs 14,799,465 allocations
across five arena blocks. The golden the build actually diffs says
8,341,214 across four, and has for some time — ch10 carried the same stale
set until its panels were wired into the machine sync earlier today. All
three prose sites now match the golden; the timing table stands, because
its 0.76/0.86 sitting (2026-07-26, interleaved, conditions named) is the
latest published measurement. The deeper fix — prose numbers that sync
from the golden the way panels now sync from samples — would retire this
entire failure class, and goes on the tooling list.

## 2026-07-27 — prose numbers now sync from the golden they quote

Third find of the same failure class this week ended the class. Any html
element carrying data-golden="family.counter" must show that counter's
current value from the cost goldens; scripts/golden_prose.py diffs them in
ci (a step on the welfare job) and rewrites them with --write, preserving
the thousands-separator style. The landing panel's and compiler page's
allocation, arena-block, and beat-iteration figures are tagged; watched
red with a perturbed number, and --write repaired it. Word-form numbers
("four arena blocks") can't be tagged, so the one remaining instance was
rephrased to lean on the tagged numeral beside it. The convention is
opt-in per number — a figure that should track a golden gets the
attribute the day it is written, and the sweep that used to re-find this
rot becomes a grep for untagged counter-looking figures.

## 2026-07-27 — INTERIM: a bare call two imports answer alike is refused

The committee's interim ruling on the silent cross-module tie, built and
pinned, awaiting Clay's reassessment. The case: two imports export the same
name with the very same shape, a bare call reaches the group, and dispatch
had nothing to pick by — so it picked import order, which the formatter
forces alphabetical, which meant directory names were deciding semantics.
The book had promised an ambiguity error; the machine had none.

The ruling and its reasoning: refuse the bare call. Rejecting is the
conservative direction — a refused program can be given meaning by a later
gavel, while a silently-resolved one is a commitment nobody made. Hickey's
lens: the tie braided name resolution with directory naming, pure incident.
Beck's: make it an error until a real use case names the semantics it
wants. Bernhardt's: silent order-dependence is the bug class. Nothing else
moves: qualified calls stay legal, a local arm of the same shape still
shadows the imports (the ruled precedence, untouched), and distinct-shape
overloads across modules keep dispatching by specificity.

check_bare_ambiguity in check.rs finds torn bare groups — two or more
synthetic enrollment clones, distinct origins, identical rank and shape,
no local arm of that shape — and reports at each bare call site with both
qualified spellings in the message. Pinned by
tests/golden/reexports/torn (watched red with the check unwired: the
program ran and printed by import order). The full battery holds: 17
suites, book corpus, kq, vse — no legitimate program in any of them had
been leaning on the silent tie. ch11's original sentence comes back true,
and if the gavel lands elsewhere, an error is the cheapest thing to relax.

## 2026-07-27 — the pipe spelling fuses: one tag test, then the loop

The OPEN thread said spelling was in the cost model — a nested chain fused
to twelve allocations while the pipe spelling of the same chain paid 5,975
— and the clean fix looked like moving fusion after inference. Built and
measured, the cheap fix turned out to be enough: a piped chain of std/list
stages rewrites to `froot = subject; if (is_desc froot) (the original
pipes) (the nested spelling)`, and the nested arm is exactly what try_fuse
already flattens. A description takes the bind cascade untouched; anything
else takes the fold. is_desc is a new internal builtin (never bare-legal at
check time; injected only by the post-check fusion pass), one tag test in
C, one match arm in the interpreter, and the wasm engine reaches it through
rt_builtin's delegation for free.

Measured: the 100k piped map/select/sum drops 900,033 -> 500,021
allocations — the build-only baseline, per-element cost zero — with
byte-identical output. Pinned by tests/golden/mem/piped_reducer.kso
(allocs=12 at n=1000; watched red at 5,975 with the rewrite unwired). The
playground's pipes example now exercises the transform on every engine in
CI. Nothing else moved: 17 suites in release and debug, book corpus, kq's
own ratchets byte-identical, welfare flat at 62.23 — the bench programs
carry no piped chains, so the ratchets rightly saw nothing. The cost is a
second copy of each chain's code behind the branch, which the compile
golden will price the day a pinned program carries one.

Found while measuring, already on the record elsewhere: the interpreter
overflows on a 100k-deep fold with or without this change — the known
depth limit, waiting on the tail-call gavel. The differential holds at
corpus scale.

## 2026-07-27 — the oracle stops being the engine that fails first

The recursion-depth entry measured the wrong asymmetry and named it: tail
recursion ran unbounded natively (musttail turns it into a jump) while the
interpreter overflowed near 54,000 frames, so programs existed that the
compiled engine ran and the semantics oracle could not. The dispatcher now
trampolines: a body whose tail position is a call to a named group — spelled
directly, through either branch of an `if`, or through a pipe — hands the
call back to dispatch_loop, which rebinds and continues instead of
recursing. Self-recursion and mutual recursion alike run in constant
interpreter stack; a million-hop ping/pong answers on both engines,
byte-identical, and the 100k-element fold that overflowed the interpreter
during the pipe-fusion measurement now completes and agrees with native.

The tail evaluator mirrors eval's resolution exactly — env-bound values,
types, err, and builtins all decline the trampoline and evaluate as before;
only a Value::FnRef naming a real group loops. Failure hops still accrue
per dispatch iteration, so err traces are unchanged. Pinned by
tests/golden/micro/deep_tail.kso: 200,000 mutual hops on both engines,
watched red on the old interpreter (stack overflow, SIGABRT).

What this deliberately does not settle: the gavel's question. Whether
proper tail calls are a PROMISE of the language — a thing the book may
teach and programs may lean on — is still Clay's to rule; nothing here adds
that sentence to the docs. What it settles is the differential law's side:
the oracle now runs everything the native engine runs at these depths,
instead of defining semantics it cannot itself execute. Non-tail recursion
keeps its stack bound on both engines, as it must.

## 2026-07-27 — BUILT, MEASURED, DECLINED (for now): map-log compaction at put_mut

The quadratic repeated-key build looked fixable without a gavel: compaction
only materializes what dedup-on-read already defines (last put wins, the
comparator tie-breaking by index), so the semantics cannot move. An
exclusive-log bit on KMap licensed in-place compaction — fresh at birth,
kept by put_mut, lost the moment a persistent put shares the log — and
put_mut adopted the cached sorted view as the new log when duplicates
dominated. Correct, built, and measured: zero effect. The 10k tally still
allocates 2.0 GB.

The measurement that matters is why. put_mut is never selected for a
read-write map: the linear analysis kills uniqueness on any second mention,
and a map you read between writes — `put m k (bump m[k])`, tally's whole
shape, in fused, hand-rolled, and binding-separated spellings alike — always
mentions twice. jsonbench's four put_mut sites are write-only builds, which
never cache a sorted view, so the compaction path is unreachable from both
directions. The mechanism was fine; no program can stand where it fires.

Two designs would unblock it, and choosing is Clay's. The principled one:
teach linear.rs that an Index read (`m[k]`) before the consuming put does
not break uniqueness — FBIP's read-then-consume discipline — which
requires an interlock with demand analysis, because a thunked read forced
after the mutation would see the wrong map. The quick one: let a plain put
inherit the predecessor's cached view by single insertion, bounded by a
view-size cap so group_by-shaped many-distinct builds don't trade quadratic
time for quadratic memory — a policy cliff that deserves a ruling rather
than an interim constant. Until one lands, the 2.0 GB tally stands, and
this entry is the map of the terrain.

## 2026-07-27 — MEASURED, DECLINED: the next protocol cannot open from std's side

The closed-protocol thread looked like a missing dispatch discriminator,
and the discriminators turn out to exist — `x:some[]` matches exactly
lists, `x:some[some]` exactly maps, `x:string` strings, all verified on
both engines — so std/list's fold and iter catch-alls were split to route
unknown shapes through `next` while keeping list, string, and map behavior
byte-identical. Built, and the user sequence still failed, for the reason
that actually closes the road: qualification. A user's `pub fn next` and
std's internal `next coll` call are different groups the moment the module
graph is merged — the internal call rewrites to `list/next`, and an
importer's arms can never join a dependency's group, because import
visibility points one way. No arrangement of std arms changes that.

What opening the protocol actually requires is open dispatch groups across
the import direction — the coherence question every language meets
(typeclass instances, trait impls, the orphan rule), and exactly the
territory of the import-incarnation gavel. Reverted rather than shipped:
the split arms alone bought nothing except a worse diagnostic for the
failing case (`no overload of list/next matches`, leaking a std internal,
where today's `length takes a list, string, or map` at least names surface
vocabulary). The terrain: user-defined sequences wait on a coherence
ruling, and when it lands, the annotated-arm split in this entry is the
five-minute half of the work.

## 2026-07-27 — the two chart risers belong to the literal-interning entry

Clay read the compiler page's trend and asked why frozen constants and ir
lines both rose. Both moves are one commit — "a literal builds once"
(#342) — whose entry recorded the wins (encode allocations down 2.5
million, kq allocations down 59%, welfare banked) but not the two costs
now visible on the chart: perm_allocs 5 -> 8, because each interned
literal owns a permanent slot filled once at startup, and emitted_lines
1488 -> 1544, the interning cache machinery inside the pinned compile
samples. Both are the feature's stated mechanism, priced against the wins
the entry already carries; the welfare gate weighed the trade and banked
it. Nothing in the twenty-seven merges since (#349 through #375) moved
either series — the history diff shows both flat from that commit to
head. The lesson repeats itself: an entry that records a trade must name
both sides, because the chart shows both.

## 2026-07-27 — a worsened counter demands a written why: the trend gate

Clay named the seam after reading the chart: the welfare scalar is the
objective and its gate is two-sided, but the deeper breakdowns — permanent
slots, ir lines — can worsen inside a banked win and nobody owes a
sentence. The literal-interning entry did exactly that. His ask: a
worsening breakdown should force reflection, especially uncompensated;
how to operationalize it.

The operationalization: the reflection IS the sentence, and the machine
only refuses silence. scripts/trend_gate.py diffs every golden counter the
branch changed against main's copy, classifies each move by a direction
table, and looks for each worsened counter's name in the branch's
compiler-log delta. Worsened and named: priced. Worsened and unnamed:
UNPRICED, listed loudly. Improvements pass free, so the check never taxes
a win, and mechanical sum-compensation stays welfare's job — this gate
deliberately doesn't duplicate it.

Report-only to start: the ci step prints verdicts and exits clean, so the
friction is measured on real pull requests before it gates. The flip is
one line, and it is Clay's to flip. Probed all three ways before shipping:
a simulated perm_allocs rise with no sentence lists as UNPRICED, the same
rise with a naming sentence prices, and a clean branch is clean.

## 2026-07-27 — the utility function learns to see peak: the one-shot term

Clay's gavel on cohort freeing was conditional — build it if it is a net
welfare win, and first make damn sure the utility function is good. The
audit said it was not: every runtime term measures the looping gauntlet,
whose rewinds reclaim decode garbage each iteration, so the exact win
cohort freeing exists to buy — peak footprint in the decode-and-hold
shape, kq's shape — was invisible to the objective. The gate would have
read the whole build as a no-op.

So the model gains a term before the build starts. A new deterministic
counter, arena_peak_bytes, tracks the live block chain's high-water — it
appears in every counter dump, so every vein regenerated in this change,
and the encode_arena_peak_bytes and arena_peak_bytes lines the trend gate
flags on this very branch are the counter's introduction, not a
regression. A new pinned program, bench/oneshot, decodes the 188 kb board
once, holds it, and prints: allocs 55,622, arena_peak_bytes 4,194,304 —
four megabytes of peak for a document that deep-sizes near 1.24, the
measured two-thirds garbage ratio written as a golden. The welfare term
oneshot_peak_bytes enters at 0.15 weight with late satiation, paid by
trimming both alloc terms, both block terms, and compile_rounds — the
block terms cede most because peak bytes measure what block counts only
proxied. A term new to the model enters at ratio one, so its introduction
moves no score; the floor recalibrates to 57.88 with the reason in the
history, and only improvement from here pays. Cohort freeing now has a
gate that can price it.

## 2026-07-27 — the trend gate enforces Clay's rule: no pure regressions

Clay stated the enforceable core precisely: the only thing the machine can
enforce is that nothing gets worse without at least one other thing
getting better — beyond that, the utility score arbitrates. The gate now
does exactly that. A changed-golden branch where some counter worsened
and no counter improved fails outright: that trade has no other side. A
genuine trade passes this gate and stands before welfare, which weighs
the sides. The name-each-worsening listing stays advisory — the sentence
is reflection, not enforcement — replacing the earlier report-only plan,
which graded the sentence rather than the substance. Probed both ways:
a lone perm_allocs rise fails; the same rise beside an allocs drop
passes to welfare's judgment.

## 2026-07-27 — structured cohort freeing: a call's garbage dies at the pop

The gaveled build, priced by the term built for it. A qualified call from
user code whose arguments are all immutable shapes — scalars and strings,
the mask that makes it impossible for the callee to have grown the
caller's storage — now marks the arena on entry, and the pop ends the
construction cohort: when at least half a megabyte of blocks grew, the
result is evacuated through the carry machinery (sized against the mark,
copied to the side buffer sharing everything below, the segment rewound,
the survivor re-interned) and the garbage goes with the rewind. A small
segment just drops the mark. Loops keep their own tier, and a caller
already inside a beat cluster is skipped — its rewind does the reclaiming,
which is why the decode gauntlet's counters moved by exactly two new
lines and nothing else.

The mechanism is composed entirely of shipped parts: k_cohort_pop is
carry_reset, stage, iter_carry, take, and beat_pop in a row, behind a
block-granular deterministic threshold snapshotted on the mark. Measured
on the reshaped one-shot bench — decode the board, hold it, encode it
back, kq's full-print shape — peak falls 9,437,184 to 6,291,456 bytes,
minus a third, for a 239,415-versus-207,435 alloc bill: the evacuation
copy, the exact trade the oneshot_peak_bytes term entered the model to
price. Welfare 57.88 -> 59.31, banked; the term's baseline re-anchored to
the reshaped bench's pre-cohort measurement in the same change, reason in
the history. allocs worsened in the one-shot vein and peak improved — the
compensation rule's first real customer, and it passes.

The slice is deliberately narrow: immutable-args only, so the
caller-held-container growth hazard cannot arise. Widening the license to
heap arguments needs the arg-position evacuation the loop tier's carried
slots already do, and that is the natural second slice once the
read-write-uniqueness ruling lands.

## 2026-07-27 — BUILT, MEASURED, DECLINED: kq behind the std/json boundary

The cohort follow-up assumed the vendored decoder was the debt and the
import boundary was the prize. The measurement inverted both. Swapping
kq's decode for std/json's — the branch lives at kq's import-std-decode,
deliberately unmerged — moves the full-print peak from 3,145,728 to
11,534,336 bytes and allocations from 96,561 to 289,668 on the same
fixture: kq's ninety-nine-plus-two-hundred-line vendored decoder is three
times lighter than the library's. The cohort wrap emits around
json/decode exactly as licensed and cohort_frees still reads zero — the
block-granular threshold never trips inside the pipe executor's bind
context, a mechanism question the branch preserves.

Two live conclusions, both queued rather than judged here. First, the
adoption should run the other way: std/json carries position machinery
and must-wrappers per node that kq's decoder simply does not pay, and the
gauntlet's own footprint would thank a diet designed against kq's shape.
Second, the cohort threshold's interaction with warm blocks and the bind
executor needs the mechanism traced before the license earns wider trust
— the one-shot bench proves the pop works; kq's shape proves the trigger
has a blind spot.

## 2026-07-27 — the cohort license learns to see through bindings and branches

The kq zero was three defects stacked, each found by instrumenting the
question the last answer raised. First, inference's pipe-yield tracking
was purely syntactic: `source = if (...) (io/read_file f) io/stdin` then
`source . report` handed report a TOP-shaped parameter, because
desc_yield saw an identifier and gave up — and its `if` arm, added
mid-hunt, sat dead below the general App arm that shadowed it. Yields now
track through one binding level (a per-body map in the inference context),
`if` yields the union of its branches, and the io constants referenced
bare answer their yields directly. Second, the license's "user code"
test read a file-path prefix that filesystem-resolved modules never
carry; it now reads what actually distinguishes the root module — an
unqualified group name — with bare enrollment clones excluded by their
synthetic flag, so library plumbing wearing a bare name no longer wraps.
Third, a caller that appears in the beat ids only as a demoted entry is
not a rewinding loop, and no longer forfeits the wrap.

Pinned by tests/golden/mem/cohort_bound.kso — the branch-chosen, bound,
piped decode fires the cohort (cohort_frees=1, peak four megabytes;
watched red at zero with the yield tracking stashed). Every existing
golden held byte-identical: the fixes add reach only in shapes nothing
had pinned. On kq's declined draft the cohort now fires too — peak
11,534,336 falls to 9,437,184, output hash unchanged — which softens
nothing about the verdict (the vendored decoder still wins at 3.1) but
proves the mechanism where it was first seen failing. The welfare header
now names its ratchet era, so a score is read inside its model version —
the confusion Clay hit reading 62-then-59 across a recalibration.

## 2026-07-27 — std/json strings move to the bytes builder (the kq diet, part 1)

The kq-swap decline left a queue item: std/json is 3x heavier than kq's
vendored decoder, so diet std toward kq's shape. Head-to-head profiling
of the same 188KB decode found the first divergence in four lines of
lib/json/text.kso: std accumulated string bytes in lists (push,
text/concat) where kq feeds a bytes builder (text/append). List
accumulation churns the shared buffer pool and denies utf8 zerocopy —
std showed append_fast=0, utf8_zerocopy=0 against kq's 21,457 / 1,773.

Adopting the builder pattern (append instead of push, a text/bytes ""
seed instead of concat onto []) closes that gap exactly: the std profile
now reads append_fast=21,457, utf8_zerocopy=1,773 — kq parity — with
alloc_bytes down 23% (4.45M to 3.41M) and decode arena peak down a full
block (4M to 3M). In the one-shot vein: allocs 239,415 to 234,323,
alloc_bytes -1.04M, sh_buf halved (2.23M to 1.10M), sh_str -72K. In the
decode gauntlet (150 decodes): allocs 8.34M to 7.58M, alloc_bytes 490M
to 334M (-32%), arena peak 4 blocks to 3, sh_buf -55%. (The gauntlet
number surfaced on CI first: bench/jsonbench vendors lib/json at
generation time, and the local copy was stale — regenerate with
make_jsonbench.sh before reading that vein.)

Priced worsenings, all the builder's own signature: bytes_malloc (13 to
1,786 one-shot, 0 to 265,950 gauntlet — each builder growth mallocs
off-arena by design, per the #336 carry model), buf_reuse (3,445 to
2,233 / 516,750 to 334,950 — the transient list buffers that used to be
reused no longer exist to reuse; their whole pool shrank, which is the
point), and perm_allocs 8 to 9 (the text/bytes "" seed constant freezes
once). Welfare 59.31 to 60.35, banked in this PR: decode_allocs and
decode_arena_blocks both moved; the one-shot peak stays encode-side
dominated at six blocks.

Also: trend_gate.py stripped only the encode_ prefix before direction
lookup, so every oneshot counter classified neutral and a pure oneshot
regression could pass silently. It now strips oneshot_ too — this
entry's two worsened counters are the fix's first customers.

Part 2 stays queued: std's container assembly churns sh_buf 2.2x over
kq's even after the string fix — obj_items/array_items shapes next.

## 2026-07-27 — transient builders grow in the arena (a 21 MB leak, caught by its own counter)

The string diet moved std/json onto the bytes builder, and the builder
carried a debt nobody had measured: growth chunks are malloc'd, the
mut-grow frees its predecessor, and the final chunk of every builder
belongs to whatever value took the buffer — usually a string that dies
at the next rewind, which frees arena and leaves malloc alone. One
decode leaks ~140 KB of chunks; the 150-decode gauntlet leaked 21 MB,
visible as RSS at 25 MB against a 3-block arena. kq never saw it
because a CLI decodes once and exits.

First the counter, then the fix. bytes_peak tracks malloc-backed
builder storage at its high water — deterministic, platform-invariant.
The red state is on the record: gauntlet bytes_peak=21,276,000, and
the new mem pin (builder_transient.kso: forty rewinds, one transient
builder each) reads 3,200 with the fix disabled — one leaked chunk per
iteration, scaling with the loop, which is the definition of the bug.

The fix routes growth by where the builder's header dies. A header the
innermost rewind reclaims gets an arena buffer (negative cap marks the
regime; the byte-append fast path and utf8 zerocopy read |cap|): the
rewind frees it wholesale, and the carry copy-out already duplicates
any bytes that outlive the frame. A header below the mark — the
licensed accumulator threading a beat — keeps malloc growth, because
arena storage above the mark would vanish under it. The encode vein
proves the split: its licensed accumulators still malloc (bytes_malloc
4,800, mut-frees 4,400) while the gauntlet and one-shot read
bytes_malloc=0, bytes_peak=0. Gauntlet RSS: 25 MB before, 3.65 MB
after — below even the pre-diet 4.7 MB.

Priced worsenings. oneshot_arena_peak_bytes 6,291,456 -> 7,340,032 and
oneshot_arena_blocks 6 -> 7 (with oneshot_alloc_bytes +84): growth that
used to leak off-arena now lives in the arena, and block granularity
rounds 686 KB of it up to a full block — the true peak (arena + builder
high water) moves 6.98 -> 7.34 MB, which the welfare term now prices at
0.15 points against the decode term's gain. emitted_lines 1,544 ->
1,559: three IR lines per sample for the signed-cap fast path.
encode_bytes_peak enters at 108,877,534 — not caused but revealed: each
encodebench iteration's burned final buffer (~270 KB) has leaked since
the shelf shipped, RSS 85 MB before and after this change. That is an
open thread, now pinned where it can be watched.

The welfare model changed to see all of this: peak terms sum arena and
builder high water (the arena proxy missed exactly the leak), and the
decode gauntlet's true peak joins at weight 0.10 — the flagship
workload's peak was unpriced, which is why a 21 MB leak could ride in
on a welfare rise. Floor recalibrated 60.35 -> 60.81 in the same PR,
reasons in the ratchet history.

## 2026-07-27 — decode board re-sat after the leak fix

The transient-builder fix moved a published number, so the whole
four-way sitting re-ran (2026-07-27, load under five, interleaved
150/450 cpu-floor slopes, 12 rounds): kanso 0.783 ms/decode at 4.13 MB
peak, serde 0.872 at 7.05, naive rust 1.024 at 7.14, go 1.948 cpu /
1.80 wall at 10.65. Slopes are within noise of the 07-26 sitting — the
leak was storage, not time — and kanso's peak cell falls 5.6 to 4.1 MB,
now the smallest on the board by a wider margin. compiler.html board,
index.html landing panel, and the about-blurb figure all updated in
the same commit.

## 2026-07-27 — the burned final is freed by the rewind that kills its string

bytes_peak's first catch after its own PR: encodebench held 85 MB of
RSS on a 4-block arena. The shape is the zerocopy handoff — a licensed
accumulator grows malloc-backed, utf8's finish gives the chunk to the
result string and burns the frontier, the string dies at the outer
loop's rewind, and the chunk had no owner left to free it. One ~270 KB
buffer per iteration since the shelf shipped (#337).

The fix registers a malloc-backed chunk with the innermost beat frame
at the moment of handoff; the frame's rewind frees its list wholesale.
Nothing can dangle: the carry copy-out never shares malloc'd data — it
fails k_survives and is copied — so by the time any rewind runs, no
surviving value points into the frame's chunks. Pops that keep their
region alive migrate the frame's list to the enclosing frame (the
strings now die with it); with no enclosing frame the chunks live to
exit, as they always did. A frame holds at most 256 registrations;
overflow leaks as before and is counted.

Red on the record: builder_reclaim.kso (licensed inner builder, outer
loop of forty) reads bytes_freed=200, bytes_peak=104,734 with
registration disabled — the forty burned finals, scaling with the
loop — and 240 / 3,880 with it on. encodebench: bytes_freed 4,400 ->
4,800 (every chunk), bytes_peak 108,877,534 -> 407,788, RSS 85 -> 6.2
MB. Decode and one-shot veins byte-identical.

The model gains the encode workload's true peak at 0.06 — the third of
three, so every published workload's peak is priced the same way —
with baseline at the pre-fix 113 MB. Floor 60.81 -> 62.70.

## 2026-07-27 — the birthday theorem gets its adversarial corpus

The ledger's soundness item: "cycles cannot cross birthdays" carried
the whole cohort-counting design on one can't-happen claim, untested.
Five attack programs now pin the boundary from the outside. Writing an
older value inside a block is rejected whether the value arrives as an
argument, is laundered through a local alias or an identity call,
belongs to an enclosing block still under construction, or is a prior
loop iteration's frozen cohort — each with its own diagnostic golden.
The deep-path write (young wrapper, old interior) is a parse error:
field writes are single-hop, so a write can only land on the named
root, and the root must be a construction of the innermost block. The
legal direction — an outer block referencing an inner block's
already-frozen cohort — runs byte-identical on all three engines
(micro corpus). Loop granularity falls out of the prior-iteration
rejection: every incarnation of a syntactic block is its own cohort.
No implementation change was needed; the checker held against every
attack. The claim now has teeth instead of confidence.

## 2026-07-27 — bytes join the cohort license, behind a survivor-ratio guard

Ledger item: generalize the scalar-result rewind. The safe half turned
out to be about arguments, not results. Bytes carry no pointers and no
thunks, so nothing a rewind frees can be reached through a bytes
argument — the container exclusions exist because a forced thunk's
value would die under a cell the caller still holds, and bytes cannot
hold one. So the license now admits bytes arguments alongside scalars
and strings. The unsafe half — containers proven thunk-free — waits on
demand-analysis integration and is recorded here as the enabler.

Probing the widening exposed a hazard the old license had dodged by
luck: the 512KB threshold counts what grew, not what died. A qualified
utf8 over a large sliced buffer trips it with a result that IS the
growth, and the dance would copy megabytes to reclaim nothing. The
survivor-ratio guard sizes the copy first (the k_copy_size pass the
carry already owns) and keeps the region when the copy exceeds half
the reclaim. cohort_kept counts the refusals.

Pins, each watched red in isolation: cohort_kept.kso reads kept=1,
frees=1; with the guard disabled it reads frees=2 (the wasted copy
happens); with the license narrowed back it reads kept=0 (the wrap
never lands). The benches are untouched — oneshot still fires its
garbage-heavy decode (survivor well under half), gauntlet and encode
unchanged, welfare flat at 62.70. This is a correctness-of-the-trade
change: the license reaches further and can no longer overpay.

## 2026-07-27 — directory modules join the render group

Probing the render campaign's "remaining" list found its items either
quietly done (single-file library verbs render custom arms; ch02/ch04
teach the sentinel model) or wrong in a new direction: the DIRECTORY
module path — the shape every real program uses — never called the
ambient merge at all. A custom to_string arm on an owned type silently
failed to join the group (both engines printed the structural form),
and an arm on a sentinel was silently outranked instead of rejected.
The single-file paths did both correctly, so the book's money example
worked while the same code in a real module did not.

The fix runs the merge once per root module with ownership counting a
type defined in any of the module's files — an arm in show.kso matches
a type in types.kso. Pins in tests/render_module.rs: the split-file
money module renders $3.50 on both engines (watched red: both printed
money 350 before the merge landed), and the sentinel arm dies with
error[ownership] at exit 2. One open design question recorded in
render-plan.md: dependency modules' arms stay qualified and never join
the root group — whether they should is a surface question for Clay.

## 2026-07-27 — the dance learns its own weight (kq submodule: built, measured, declined)

The standing goal is kq beating jq on every row, and the cohort looked
like the missing piece: vendoring denies kq the boundary, so a
decode-submodule restructure (kq's own light decoder behind a local
import) was built and raced. Hash-identical output; the peak went the
wrong way — 47.5 to 72 MB on the full print, 26.6 to 58.7 on the path
query. The decomposition taught four things about the dance itself,
each now fixed in the runtime:

1. The carry pair is a per-depth static sized for loops that stage
   again; a cohort stages once, so a 13 MB survivor left 26 MB of carry
   buffers live for the rest of the process. Cohort pops free the pair.
2. Rewound blocks sit in the spare chain for hot-loop reuse; a one-shot
   boundary has no hot loop coming, and the malloc'd print builder
   cannot use arena spare. Cohort pops release spare beyond two blocks.
3. The ratio guard priced reclaim against copy but not the dance's
   transient peak: evacuating a survivor much larger than the threshold
   scale makes the carry buffers the peak. A survivor cap (4x the block
   threshold, 2 MB) refuses those dances.
4. The guard's own sizing pass walked the whole survivor and fed a
   seen-map proportional to it — 13 MB of measurement to decide not to
   act. The pass now carries a budget and stops at the verdict.

With all four, the kq submodule build reads 29.8 / 49.5 MB against
main's 26.6 / 47.5 — the cohort correctly refuses to fire on
live-data-dominated workloads, and the residual is module overhead
plus the bounded guard walk. The restructure is DECLINED (branch
kq:decode-submodule preserved with the numbers); the shelf finding
stands — kq's remaining gap is the live document and the output, and
no free-the-garbage scheme touches it. The oneshot fire is unchanged
(survivor under the cap, garbage-dominated), veins byte-identical.
What this bought is armor: any root program handing a large document
across a licensed boundary would have paid the 2x dance; now it
cannot.

## 2026-07-27 — the interpreter reports the stack it runs out of

Probing the TRMC ledger item found a differential-law violation on the
crash surface: deep non-tail recursion gets a clean
error[runtime] from native (the parent translating the child's
SIGSEGV) while the interpreter died in Rust's own overflow handler —
an abort, not a diagnostic, with a Rust panic message no kanso program
should ever show. The interpreter now counts dispatch depth and
returns the same words through its RuntimeError channel at fifty
thousand frames, well under where its 256 MB thread would fall.
Engines differ in how deep they can actually go — frame sizes differ,
that window is inherent — but the report and the exit code are one.
The third engine had the same hole one layer down: compiled wasm
exhausts the browser's stack as a trap that records nothing, and the
trap fetch rendered an empty message. An unrecorded trap IS the stack
case — the same translation native's parent makes from SIGSEGV — so
the fetch now says the same words. The guard sits at ten thousand
frames over a 1 GB interpreter thread (debug builds carry frames fat
enough to fell the old 256 MB at half the old limit). Pinned in
tests/golden/runtime/deep_recursion.kso — native and interp asserted
identical by the corpus harness, all three engines byte-identical in
the browser differential (82 passed, 0 failed) — and the appendix's
error[runtime] entry documents the report beside the
accumulator-passing fix. TRMC itself (raising the ceiling by compiling
accumulating recursion to loops) stays in the ledger as the ergonomics
item it is.

## 2026-07-27 — accumulating integer recursion runs as a loop (TRMC, the exact slice)

The ledger's TRMC item, taken at the width where reassociation is
provably exact. A group like `count n = 1 + count (n - 1)` owes an
addition after every return, so a million calls kept a million frames;
over arbitrary-precision integers the sum is the same in any
association, so the group rewrites to a tail-calling helper threading
an accumulator, and the wrapper keeps the group's surface, name, and
dispatch. The license is narrow by construction: every arm a single
expression, one operator (+ or *) across the recursive arms, integer
literals for the non-recursive operand and every base body, and a
direct self-call — floats (association changes the answer), guards,
and double recursion like fib all keep their frames.

The rewrite is one AST pass in the shared front end, so all three
engines inherit it: the million-frame count now prints on native,
interp, and the browser alike (micro pin trmc_count.kso, browser
differential 83 passed / 0 failed). The stack-report pin moved to a
variable-operand shape (`n + weigh (n - 1)`), which stays honestly
unlicensed, and appendix A teaches the line between the two: a literal
operand compiles to the loop, a varying one keeps its frames until you
carry the total down yourself. Veins byte-identical, compile golden
untouched (no bench sample carries a licensed shape), welfare flat.

## 2026-07-27 — TRMC keeps the crash it was hiding (additive wrappers)

Probing the shipped transform found an edge #394 got wrong: a float
argument to a licensed group used to descend with frames and die with
the stack report; the transformed tail loop ran forever. A crash had
become a hang — engine-consistent, but the wrong trade.

The fix simplifies the whole design. The original arms are no longer
replaced: the pass ADDS an accumulator helper and a wrapper arm whose
counter positions (the ones some arm dispatches on with an integer
literal) are ascribed :int. Specificity does the routing — a literal
arm still answers its literal, the ascribed wrapper outranks the bare
arm for integers and takes the loop, and every non-integer argument
falls through to the original arms and behaves exactly as it always
did, stack report included. A group with no integer-literal position
is left alone, since nothing would bound its descent. Verified: the
million-frame count loops, count 2.5 reports the stack on both
engines, count "oops" errs identically, zero and negative bases answer
through the original literal arms. 19 suites, browser differential
83/0, decode vein byte-identical.

## 2026-07-27 — the ledger closes its sweep; the pending gavels get a catalog

The queue's last four survivors are settled with evidence rather than
left dangling. The mini-rewind and the two-level scratch arena are
declined for now: no current workload shows unreclaimed LIFO scratch
beyond what beats and the cohort already rewind, and that missing
signal — arena growth with no beat or cohort fire — is the recorded
reopening condition. The three-way escape split ran its gating
measurement: vse holds two arena blocks with its loops beating, so the
split has nothing to buy on the workload named as its test. The
explain-copies item is part-superseded by the counter stack this
stretch built (bytes_peak, cohort_kept, carry_dedup, the trend gate,
the welfare peak terms); the unserved half — naming the source site of
each evacuation copy — is scoped and waits on a CLI-surface ruling.

Everything that waits on Clay now lives in design/pending-gavels.md:
eleven decisions with context, interim state, and what each unblocks,
plus the four open-but-unblocking notes. The sweep that started at the
top of this log's 2026-07-27 entries ends here.

## 2026-07-27 — GAVEL: streaming stdout. io/write ships, and the bind loop learns to floor

Clay gaveled streaming stdout ("seems very good! implement"). Two
pieces, and the second is the one that mattered.

io/write is the eighth description: a string to stdout, no newline,
wired through all three engines, the plan renderer, the executors, and
appendix B. The streaming idiom is write-then-bind — the continuation
builds the next chunk only when the executor reaches it — and the bind
executor's bracketed loop makes the chain flat: a 20,000-chunk, 32 MB
stream runs at 1.4 MB RSS, one arena block, pinned in
tests/golden/mem/stream_write.kso.

But the first kq measurement went the wrong way, 47.5 to 88.6 MB: the
bind loop's evacuation hauled the continuation — which captures the
13 MB decoded document — through the carry pair on every one of 1,600
chunks. An intermediate design (promote-by-copying) still paid the
doubling once, 59.7. The landed fix sizes the staged continuation with
the cohort guard's budgeted pass and, past 256 KB, skips the
evacuation entirely: the bracket pops and re-marks, flooring the
region under the big survivor. Later steps find it below the mark and
share it; the ping-pong resumes for per-step junk; the step's garbage
joins the floor — the same trade the cohort's keep makes, and right
for a process that exits. All three cost veins are byte-identical (the
bench chains stage under the threshold), welfare flat, browser
differential 83/0.

kq streams its top-level list through exactly this idiom, and the
scoreboard's last two losing rows flip: full print 47.5 -> 30.0 MB
against jq's 30.7 on the 1.9 MB document, 5.1 -> 4.5 against 4.9 on
the 188 KB one, hash-identical to jq -S, spec suite green. kq now
leads every row. The kq-side landing (goldens, README, TRY, site
scoreboards) rides the sibling PR.

## 2026-07-27 — the compiler page records the closed row

The kq paragraph's closing claim ("what remains of the pretty-print
gap…") described a gap that no longer exists: the streaming print
landed at 30.0 MB against jq's 30.8, and kq holds less memory than jq
on every scoreboard row. The page now says so, and the 211.9 → 139.9 →
47.5 fall reads its final number.

## 2026-07-27 — hako grows a source model (design, Clay-directed)

The dependency-source abstraction, fleshed out from Clay's direction:
identity and transport stay unbraided. A hako's name is its GitHub
path forever; a source is a strategy for turning name plus
version-request into content the lockfile's sha can verify.
github_repo is the v1 source and default — tags are releases, majors
are path forks, branches are interim pins the lock marks and update
refuses to walk. The eventual true server is a source with no
authority: a verified mirror plus metadata index, registry-then-git
fetch preference, degrading to git byte-identically because the sha
decides what content is. Names never move, so the server migrates
nobody. The err-arm rule's foreign bit survives unchanged: foreign
iff entered through any source, local iff relative.

## 2026-07-28 — the err-arm campaign opens: what exists, what's missing

Clay gaveled the start. Probing before building found the foundation
already alive and engine-identical: wrapper subtypes parse in the
space form (`type dog animal` — one parent is a subtype, several are
a typeset), construction wraps the parent, subsumption admits a child
to the parent's arm, the child's arm outranks it, and — the big one —
reason-type-ascribed err arms (`(err w:parse_woe)`) already trap and
read fields on both engines. Two micro pins land here
(subtype_chain, err_trap_leaf), three-engine verified.

Missing, in build order: (1) the err-pattern overlap check compares
only the outer ctor, so a leaf arm and a typeset-root arm are refused
as "overlapping" — same_shape must recurse into fields, and the
score must separate inner rank from inner depth (today a bare
`(err reason)` arm would tie a typed one); (2) the two-condition
license — nothing yet stops a module converting its own err, and
std's failure_position projections do exactly that (they migrate to
direct field reads under the structure-access amendment); (3)
implicit auto-cause chains; (4) surface gaps — `_:type` does not
parse, a wrapper renders as its parent. (The claim that `e:err`
ascription does not match was wrong: the hop trace shows the arm
matching — an err arm's body propagates the err it bound, which
prints identically to a fall-through. Corrected here rather than
edited above, per the append-only rule.) Interim calls, recorded: the space-form declaration is
canonical (the colon form never existed), implicit auto-cause over a
wrap_err factory, named-access-only foreign destructuring.

## 2026-07-28 — err arms learn the ladder (leaf, root, bare, in any order)

The overlap check compared only the outer ctor, so a leaf arm and a
typeset-root arm on the same group were refused as overlapping;
same_shape now recurses into ctor fields, so arms differing in their
reason pattern coexist and identical shapes are still refused. The
interp's score conflated inner rank with inner depth — a bare
`(err reason)` binder tied a named leaf; it now ranks at 89, below
every named reason and just above the plain generics. The native and
wasm-backend arm sorts keyed every ctor at a flat 2000, which left
leaf-vs-root err arms in declaration order — a real divergence-in-
waiting against the interp's scores; both now key an err arm by its
reason pattern, mirroring the interp exactly.

Pins, red-watched: err_trap_named (was exit 2 "overlapping
overloads") now picks leaf, root, and value arms correctly;
err_trap_order declares the root arm first and the leaf still wins —
specificity, not declaration order, exactly Ruby's rescue made
order-independent. Browser differential 87 passed / 0 failed.

## 2026-07-28 — `_:type`, the arm that needs the type and not the value

Dispatching on a type without using the value had no spelling. A bare
binder trips the unused-binding check, and the language retired
leading underscores itself — "privacy is `pub`'s absence, and `_`
alone is the wildcard" — which points at exactly the spelling that
did not parse. `_:type` now parses as an ascription that binds
nothing, under the same tightness rule as `n:int` (`_ :dog` is a
formatting error), and the unused check skips the name it cannot
bind. One micro pin, three engines, and ch03 gains the sentence.

## 2026-07-28 — wrap_err ships, and the interim call reverses on contact

The gaveled promise was that a re-raise never discards the original.
The interim call was Ruby's implicit chaining — an err minted inside
an err-matched arm silently carries the matched err. Implementation
contact reversed it, on two grounds. Cost: implicit needs the whole
matched err available where only its destructured reason is bound
(as-patterns kanso does not have), then that value threaded through
dispatch in three engines and past the thunk layer. The factory is
one builtin routed through the existing bridge. Semantics: implicit
chaining attaches a cause to every err minted in a handler, including
the ones an author means as fresh and unrelated — the surprise Ruby
is known for — and Clay's own first instinct in the dialog was
"maybe what's wanted is a factory method like wrap_err".

So: `wrap_err reason original` mints an err with the original as its
cause. It is the one hole in err's infectiousness — the second
argument IS an err and must arrive as a value — implemented as an
exemption ahead of the propagation gate in call_builtin, which the
wasm bridge shares for free; the compiled-wasm path stamps the site's
origin the way the other err-bearing builtins do, since no frame
exists there. ErrInfo and KErrBox gained a cause link; the endpoint
report renders `caused by:` and recurses, each link keeping its own
birthplace. Byte-identical across all three engines (browser
differential 89/0), runtime-corpus pinned, appendix B documents it
with a verified sample.

Left open, and small: an arm cannot both destructure a reason and
name the whole err (`(err d:disk_torn)` binds d, not e), so wrapping
with data from the reason wants an as-pattern. The `e:err` shape
covers the common case and nothing in the tree needs the other yet.

## 2026-07-28 — a subtype may claim its own rendering (a live divergence, closed)

Probing wrapper rendering found the engines disagreeing in the field:
a user `to_string` arm for a subtype was honored by native and
ignored by the interpreter, which rendered the parent — `$3.50` on
one engine, `350` on the other, for the same program. The oracle was
the wrong one. Native gates on the inferred set, which carries the
wrapper's own bit; the interpreter gated on the runtime value shape
and its list named records, none, and descriptions but not wrappers,
so a subtype skipped the ambient group entirely.

The render campaign's coherence licence is what decides it: a
primitive keeps the direct renderer because the ownership rule proves
no arm can exist for it, and that proof does not reach a wrapper —
a subtype is a type the user declared and may therefore claim. The
interpreter's gate now includes it. Default behaviour is untouched:
with no arm, a wrapper still renders exactly as its parent does, the
transparency the subtype gavel intends. Pinned three-engine
(sub_render_arm: custom arm, bare wrapper, bare primitive in one
program); browser differential 90 passed / 0 failed.

## 2026-07-28 — as-patterns declined: dispatch already spells it

The gap left by wrap_err — an arm cannot both destructure a reason
and name the whole err — looked like it wanted an as-pattern. It does
not. The case composes out of what the language already has: bind the
whole err in one group, destructure the reason in another.

    fn relabel e:err
      wrap_err (config_bad "app.conf" (path_of e)) e

    fn path_of (err d:disk_torn)
      d.path

    fn path_of _
      "unknown"

Measured end to end: the wrapped err carries the reason's data, the
cause chain survives, each link keeps its birthplace. This is the
language's own idiom — one name, many arms — rather than a second
pattern syntax that would need parser, checker, and three engines.
Not a YAGNI deferral: the spelling exists and reads better than the
as-pattern would. One honest caveat: the extractor converts an err to
a value, so under the license it is legal exactly when the reason
type is foreign — which is the case wrapping is for.

## 2026-07-28 — evacuation cannot free a large survivor (the law, measured twice)

Streaming changed what kq's peak is made of, so the declined
decode-submodule experiment was re-run against it rather than trusted.
Rebased onto streaming main it reads 32.7 MB against main's 30.0 —
still worse, and now the counters say why: cohort_kept=1, the
survivor-ratio guard refusing the dance.

Whether the guard was right is a measurement, not an opinion, so the
cap was lifted and the evacuation forced: 58.7 MB, nearly double.
The reason is structural and worth stating as a law. Evacuation frees
a region by copying what survives out of it, so it transiently holds
the survivor twice while the garbage is still live — the peak during
the dance is (region + survivor), and the peak is what a scoreboard
measures. Freeing 17 MB of decode garbage by copying a 13 MB document
raises the high-water mark instead of lowering it. **A
copy-out-and-rewind scheme can only win where the survivor is small
against the garbage.** kq's decode is the opposite shape by
construction: the document is the point.

So the boundary/cohort family is closed for kq, for the third and
last time, with the general reason on record rather than another
per-experiment verdict. What remains of kq's footprint is the decoded
document itself; the levers that could still move it are a lighter
document representation or decoding lazily against a retained input,
neither of which is a memory-reclamation scheme. Also re-measured
this sitting, per the every-change-carries-a-perf-check rule: the
front end runs kq's check in 5.9 ms and the json decoder's in 5.2 ms,
faster than the 6.6/6.1 the compiler page publishes, so the session's
compiler additions (the trmc pass, the recursive overlap check, the
wider render gate) cost nothing measurable.

## 2026-07-28 — the list growth policy is measured-optimal (built, measured, declined)

The decode profile says the workload is small collections: 2,752
arrays holding 9,732 elements and 2,761 objects holding 8,361 pairs,
so three and a half elements per array and three pairs per object,
against a buffer policy that starts at four slots and doubles. That
looked like over-allocation worth testing — a first buffer of eight
where four would hold most collections whole.

Two experiments, both negative. Skipping the doubling when the list
is empty changed nothing at all: an empty literal carries a one-slot
buffer that absorbs the first push, so the growth path is never
entered at length zero. Skipping it at length one — the real first
growth — traded three hundred allocations for 2.9 MB more bytes and
2.9 MB more buffer storage, with shelf reuse climbing from 334,950 to
469,200: the smaller first buffer simply moves the same collections
through an extra growth step, and every step is an allocation plus a
copy plus a shelf entry. The doubling earns its headroom.

Declined, and the policy is now measured rather than assumed. The
decode path stands at 2.79 allocations per document node with the
shelf already recycling 2,233 buffers per decode; the remaining
weight is the document itself, which is the same wall the evacuation
law hit from the other side.

## 2026-07-28 — the book teaches wrapper types (a shipped feature with no page)

An audit for the standing docs directive found subtypes undocumented
everywhere: zero mentions across ch03, appendix b, and appendix c,
while ch03 teaches typesets seven times. The gap had already started
costing — appendix a's ownership diagnostic tells the reader to "wrap
the value in a type of your own and render that," advice for a
construct the book never introduces.

ch03 gains a section beside typesets, since the two are the same
question from opposite sides: a typeset names a set of types you
have, a wrapper names a new type that is one you have. It teaches
declaration (child first, parent after), construction, the ladder
position (a wrapper's arm above its parent's, chains resolving to the
nearer declaration), and rendering — transparent by default, claimed
by a to_string arm, with appendix a's ownership rule as the reason
that arm is allowed. The specificity-ladder sentence earlier in the
chapter gains the rung, and an exercise closes on the case worth
feeling: delete the wrapper's arm and predict where the value lands.
The sample is verified by the harness like every other panel.

## 2026-07-28 — the campaign's diagnostics get their goldens

An audit of this session's own work against the every-behaviour-ships-
with-a-golden rule found three diagnostics shipped without corpus
entries. All three now have them, each watched fire before it was
pinned.

`wildcard_ascription_loose` pins the tightness rule reaching the new
wildcard spelling: `_ :dog` is refused the way `n : int` is.
`overlapping_err_arms` pins the other half of the recursive overlap
check from the ladder work — arms differing in their reason pattern
coexist, and two arms naming the same reason are still illegal, which
is the claim that PR made and did not pin. `sub_render_ownership`
pins the coherence argument the wrapper-rendering fix rested on: a
module that declares `money` may claim it, and still may not claim
`int` — the licence follows the declaration, not the chain.

Sixty-one error programs now, covering thirteen diagnostic kinds.

## 2026-07-28 — every io program was falling back in the browser

Auditing which programs the browser differential compiles rather than
interprets found a hole hiding behind a passing suite: the compiled
wasm backend rejected `builtin_args`, so every program importing
std/io fell back to the interpreter — the import alone was enough,
whether or not the program ever mentions args. The streaming pin from
the io/write work was among them, which means the feature had never
run through the compiled browser path at all while reporting green.

The backend supported `args` and `stdin` but not the `builtin_`
spelling std's own wrappers use, which every other site in the
compiler normalizes away before matching. It normalizes here now. The
differential moves from 90 passed with 4 fallbacks to 91 passed with
3, and the first io program to reach the compiled path passes
byte-identical. The three that remain fall back for an unrelated
reason (an unsupported call head) and are honestly reported as
before.

The general lesson is about the shape of the report: a fallback is
not a failure and never turns the suite red, so a feature can look
covered on three engines while one of them quietly runs the
interpreter underneath. Reading the skip lines is part of reading the
result.

## 2026-07-28 — the browser corpus compiles, all of it

Following the io fallback into its neighbours: the three programs
left interpreting shared one cause, and it was not the one their
message suggested. "Unsupported call head" named a lambda sitting in
call position — an inlined single-use binding leaves one there — and
every program that passed a lambda to a list adapter hit it. Lambdas
were never the problem; the backend builds closures already, and a
lambda in call position is one built and applied, which is three
lines it was missing.

The browser differential now reads 94 passed with zero fallbacks: the
compiled path covers the whole corpus for the first time, list and
pipe programs included — which is what the playground actually runs
when somebody types a map into it. The single remaining gap is
honest and unrelated (std/json is not in the shipped library, so the
browser cannot resolve that import at all).

Worth keeping beside the earlier entry: two fallbacks in two
iterations, both hiding behind a green suite, both found by reading
the skip lines rather than the pass count.

## 2026-07-28 — the playground can run std/json

The differential's last standing gap was the language's own flagship
library: `import "std/json"` failed in the browser, so a visitor to
the playground could not run the thing the front page benchmarks.
The embedded-source mechanism that serves the browser and installed
binaries handled one file per module with no imports of its own, and
json is five files that import std/list and std/text.

The fix is a unification rather than a special case. The module
loader now takes its file set as a parameter: the disk path reads a
directory as before, and the embedded path hands over include_str!
contents under the same names. Everything after that — the
alphabetical check per file, the cross-file namespace merge, import
resolution (which finds the embedded std/list and std/text through
the same branch), re-exports, shadowing, the ambient render arms — is
the code that already ran for directories. Embedded modules stop
being a second, weaker loader.

Browser differential: 95 passed, zero known gaps, zero fallbacks,
zero failures — the whole corpus, on all three engines, for the first
time. Cost: 52 KB of wasm, 4.8%. The decode vein is byte-identical
and lib/json's own 18 tests pass, so nothing about the shipped
decoder moved; only where its source can be found did.

## 2026-07-28 — the playground offers the library it benchmarks

std/json running in the browser is only half a capability if nothing
in the playground reaches for it. Twelve examples shipped there and
none touched the library the front page benchmarks, so a visitor had
no path to it. There is one now, and it teaches the pair worth
teaching: a document that decodes and one that does not, dispatched
apart by an `(err reason)` arm beside a plain one.

The example carries no braces, and that is deliberate. The
differential harness reads docs/play.js as raw text while the browser
receives the JavaScript-unescaped string, so an example containing a
backslash would be tested in one form and run in another — the kind
of gap that reports green while covering nothing. Decoding an array
sidesteps it: no `{` to escape, so raw text and runtime text are the
same bytes. A brace-carrying example would want the harness taught to
unescape first.

Differential: 96 passed, no gaps, no fallbacks, no failures.

## 2026-07-28 — the harnesses read what the browser reads

Last entry sidestepped an escaping trap by choosing an example
without braces; this one removes the trap. Both playground harnesses
— the browser differential and the cargo suite — lifted example
sources out of play.js as raw file text, while the browser receives
what JavaScript makes of that text after resolving a template
literal's escapes. Any example carrying a backslash would therefore
be tested in one form and shipped in another, and json objects carry
backslashes by necessity: `{` opens an interpolation in a kanso
string, so a JSON brace must be escaped.

Watched red first, which is the point of the order: an object-shaped
example made both harnesses fail with `unexpected character` on text
no visitor would ever run. Both now resolve the escapes JavaScript
resolves, and the playground's json example is object-shaped —
`doc["title"]` beside a torn document — because that is what a
reader recognizes as json.

Two harnesses had the same defect, which is what happens when two
readers parse the same file by hand. Neither knew about the other.

## 2026-07-28 — the play panels were never checked

The book's panel checker compares an output panel against the sample's
recorded .out, and it recognized an output panel by its title: a
command reading `kanso run`, `check`, `test`, or `build`. Every panel
this session added titles its command `kanso play`, which the pattern
did not match, so four panels — the wrap_err chain, the wrapper
types, the streaming write, the stack report — displayed output that
nothing compared to anything.

Watched red the only way that means something here: falsify a
panel's text and run the checks. Both passed, cheerfully. The word
`play` in the pattern is the whole fix, and with it the same
falsification is caught by name. All four panels were in fact
correct, which is luck rather than process — they were written by
copying real output, and nothing would have said otherwise.

The audits of the last several days keep landing on one shape: a
check that looks at less than its name suggests. Two harnesses
reading raw text where the browser reads escaped, a fallback path
that never reddens a suite, and now a title pattern that quietly
skips a verb. None of them failed; all of them covered less than the
reader would assume.

## 2026-07-28 — one sitting, published once

The log already records what happens when a perf sweep runs on
memory: three surfaces stale for a day, and a later sweep finding
five disagreeing figure sets across four pages. The rule written
afterwards — walk every number-bearing surface, it is a checklist not
a memory — is the right rule and has no enforcement behind it.

Some of that is unenforceable by construction: a wall-clock figure
has no golden to check against, which is why the table is dated and
called a sitting. But a figure published twice is checkable against
itself, and the decode sitting is published three times over — the
compiler page's board, the landing page's code panel, and the landing
page's own prose sentence quoting kanso against serde. Nothing
compared them.

cross_surface_numbers.py compares them. It knows nothing about what
the numbers should be; it knows which claims are the same claim.
Watched red on each surface it says it covers: a wrong figure in the
panel, in the prose, and in the peak all fail it by name, and today's
four rows agree. In CI beside the golden-prose check.

## 2026-07-28 — the license, advised rather than enforced

The two-universe rule has been blocked on two rulings, but only its
edges are: the mechanism is gaveled entire — std is foreign, local
modules share a universe, published packages do not. What waits on
Clay is a test-file exemption and a chapter's prose. So the rule ships
as an advisory, which is progress the gavels do not block and which
turns a prediction into a list.

It is sound by under-approximation, the same way the door advisory
is. A qualifier alone cannot separate a foreign package from a local
subdirectory, so it says nothing there; it speaks only where the
answer needs no plumbing — an arm and a reason type carrying the same
qualifier are one module, so one universe. Test files are exempt,
the interim call already reported.

The fleet's entire violation set turns out to be two functions:
std/json's failure_position and failure_reason, and kq's vendored
copies of the same two. Everything else is quiet, including the arm
that looked most likely to trip — kq's `render_result (err reason)`,
which matches an err and re-raises it, exactly the discipline. The
migration Clay's ruling unblocks is therefore two deletions and three
call sites, not a campaign.

Pinned both ways in the advisory corpus: an arm that converts its own
err is advised, an arm that re-raises is silent.

## 2026-07-28 — the license needs the structure amendment to be shippable

Trying to migrate the one call site that needs no ruling — an example
that is a foreign client, so trapping std/json's failure is
unambiguously legal — found the deadlock. A foreign client may name a
foreign reason type: `(err e:json/parse_failure)` compiles and
dispatches, membership crossing an import exactly as the doctrine
says. It may not read that failure's fields, because the opacity rule
still bans foreign destructuring. The only route to a position or a
reason is the owner's projection function, and the license forbids
the owner from writing one.

So the two rules together make failure data unreachable, and the
structure-access amendment is a prerequisite rather than a companion.
Recorded at the head of the gavel entry so the ruling is read with
its dependency attached.

What is implemented is now pinned: foreign_err_trap names a foreign
reason type and dispatches on it, three engines, beside a plain
document taking the other arm.

## 2026-07-28 — CORRECTION: inspection was never in question

Clay, on the entry above: "it can inspect them all day. it just can't
rescue them. any function that it passes its own err to must also
return err."

The claim recorded here — that a package cannot inspect its own errs
— was wrong, and wrong in a way worth naming: it confused the
constraint on a function's *return* with a constraint on what the
function may *look at*. An arm may match its own err, read every
field of the reason, and build anything it likes out of them, so long
as an err is what comes back. Verified: an arm reading two fields and
raising a new reason built from both runs, and the license advisory
is silent on it — so the code drew the right line while the prose
described a different one.

What survives is narrower. A package cannot get a non-err value out
of its own err, and an assertion is a value: a test constant that
touches an err propagates it and the harness reports FAILED
(returned err …), which closes equality and interpolation together.
The test-file exemption is still wanted, for that reason rather than
the one first written down.

## 2026-07-28 — the rule, restated in one line

Clay, refining again: the constraint is that "the function that gets
it as a return value can't return anything other than an err so long
as that err was generated by a function in that same package."

That is the whole rule, and it is simpler than the two clauses it
replaces. Extraction inside the function was never restricted — a
position pulled out of a reason is an int like any other — so the
earlier phrasing, that a package cannot get a non-err value out of
its own err, was still describing the wrong thing. The constraint
lives on the return, and only there.

It also settles the transitive case without a second sentence: hand
your own err to a helper that returns an int, and the helper is the
violation, because the helper is what received the err. Verified
against the advisory, which lands on the arm that matched the err and
returns the int, and stays silent on the function that only ever sees
the reason record. Recorded as the canonical statement at the head of
the gavel entry, since this is the sentence the book will teach.

## 2026-07-28 — provenance, and the hole in the proxy

Clay, on the restated rule: the criterion is where the err stack
originated, not which package the receiving function is in — and he
says plainly he cannot see the simplest way to give a compiler that.
The advisory shipped here answers a nearby question instead: it
compares the arm's qualifier with the reason type's. Two measurements
say how far apart those are.

The proxy has a reachable hole. User code raises
`err (json/parse_failure 1 "mine")`, matches it with
`(err _:json/parse_failure)`, and returns a string. It runs, and the
advisory is silent, because the qualifiers differ and a foreign
reason reads as licensed. By provenance the err was raised in the
user's own package and must not be rescued there.

The hole is reachable because a second doctrine line is unenforced:
modules-plan says construction is module-private and importers build
through pub factories, and in fact user code builds
`json/parse_failure 99 "I made this"` and prints it. Enforcing that
line would close both — only json can build a json reason, so a
json-reasoned err can only have been raised by json, and the reason
type stops being a proxy and becomes a witness. The alternatives are
a whole-program dataflow over err provenance or a runtime check
against the origin every err already carries.

Recorded rather than built: enforcing construction is a language rule
with a migration behind it, and it is Clay's call.

## 2026-07-28 — provenance, computed

Clay: build it correctly. The proxy is gone and provenance is
computed. Each dispatch group gets the set of packages whose errs it
may hand back, taken to a fixpoint over the call graph — the shape
inference already uses for value sets, and cheap for the reason Clay
gave: one hop is enough, because an err reaches a function only
through a pattern that matches err, so every step of a failure's
travel is a call whose callee names it.

The rule then reads off the analysis with nothing left to guess: a
group that may receive an err raised in its own package must return
an err. Two design points the fixtures forced. A pub group is seeded
with its own package — its callers are not all in view, and anyone
may hand a package its own failure back, which is exactly how
std/json's projections are reached. And the walk had to descend into
every expression form, not only statement position: the first
same-package rescue went undetected because the call sat inside a
string interpolation.

Caught now, and pinned: the laundering case Clay's correction
predicted — raise an err whose reason is borrowed from another
package, then rescue it locally — is named by its raiser, so the
borrowed name buys nothing. Rescuing a genuinely foreign err stays
silent, as does re-raising one's own.

Cost, measured A/B on one binary rather than against yesterday's
number: 0.6 ms on `kanso check`. The absolute figures today read two
milliseconds high on a loaded box — main measured 8.9 ms against the
branch's 8.0 — which is why the A/B was run at all. KANSO_NO_PROV
turns the pass off, as KANSO_NO_FUSE does for fusion.

## 2026-07-28 — why tests need the exemption, and a way to not need it

Clay, working out why a test would want to rescue its own package's
err: to prove that handing an err to the function under test causes a
side effect, or comes back wrapped. That is the case, and the reason
under it is simpler than the case — an assertion is a value, and the
rule forbids a package producing a value from its own err. Measured:
a test constant that touches an err reports FAILED (returned err …),
so equality, interpolation and every other route close together.

Which opens a second design. The harness is not the package; it is
the toolchain. If assertions about failure came from a toolchain
surface rather than a function the package writes, provenance permits
them by the same clause that lets any foreign party rescue, with no
file-scope exemption and no leak into shipped code. Recorded beside
the exemption in the gavel entry as the cleaner of the two.

Also recorded, for the far queue: design/testing.md holds Clay's
shape for a spec framework — one describe, one level of contexts,
assertions at either level, nothing deeper, a JustBeforeEach
equivalent, and possibly a one-level `let`. The constraint is the
design: two levels can be read from one screen, and a failing example
three contexts down cannot.

## 2026-07-28 — tag-hoist was already harvested, and numeric is at parity

Clay asked whether the ledger was finished. Section 3's eight survivors
were; section 4 — five speculative ideas, each marked as wanting an
experiment — had never been touched. Starting with the one the section
calls highest-value, because it aims at representation.

There is nothing left to hoist. A tight numeric loop compiles with its
arguments unboxed — `i64`, not `%KValue` — every tag a compile-time
constant, and the recursion a musttail self call. The unboxing proof
(#262) got there first and took the tag with it. What separates the
loop from Rust is three overflow intrinsics per iteration, one per
arithmetic operation, which is semantics: kanso's int is arbitrary
precision and a native build traps rather than wrapping silently.

Measured like for like, 2M iterations of `acc + n * n`: kanso 2.52 ms,
Rust with `-C overflow-checks=on` 2.43 ms — **1.03x**. Against Rust's
default wrapping arithmetic it is 1.37x, and that difference is the
price of the int, not a gap in compilation. So the remaining lever on
numeric code is not representation at all but discharging overflow
checks where bounds are provable, which needs specialization to see a
caller's constant. Recorded rather than built.

Also corrected while reading: section 0's receipt, which says linear.rs
is dead analysis and push_mut is called by nothing, describes a tree
from before the uniqueness campaign. Marked stale in place.

## 2026-07-28 — two peak metrics, and which one the numbers used

Measuring ledger 4.1 turned up an instrument problem worth recording
before any conclusion drawn with it. `/usr/bin/time -l` reports both
"peak memory footprint" and "maximum resident set size", and on
arena-heavy workloads they disagree by a factor of forty: a
build-block loop reads 1.6 MB of footprint against 65 MB of maximum
resident. Footprint excludes pages the arena has rewound and handed
back; resident counts them. For a question about peak footprint the
resident figure is the honest one, and every measurement in this
entry uses it.

The published comparisons are unaffected, checked rather than assumed:
kq reads 30.5 MB maximum resident against 30.0 MB footprint, jq 31.4
against 30.7 — two percent apart, and kq wins on both. The divergence
needs an arena that grows and rewinds hard, which a decode-and-print
does not do and this fixture does.

## 2026-07-28 — 4.1, measured before building

Three results, none of them the optimization yet.

A build block's tree carried across a beat costs about nine times what
the same tree costs built once: 4.4 MB for a single 20,000-node build,
40 MB for the same build carried through a beat loop. That gap is the
beat and carry machinery, which is what 4.1 proposes to halve.

The gap does not grow with iteration count — 10, 30 and 120 iterations
all peak near 40 MB — so the arena reclaims correctly and this is a
fixed working set rather than a leak.

And one hypothesis died on contact: in-place push was suspected of not
firing inside a build block, since the mutation analysis has more to
prove there. It fires identically — the same 60,013 allocations and
2,939,344 bytes inside and out. Whatever the nine times is made of, it
is not lost uniqueness.

## 2026-07-28 — CORRECTION: the nine times was measurement error, and 4.1 is declined

The entry above reports a carried build-block tree costing about nine
times a single build, 4.4 MB against 40 MB. The 40 was wrong. It came
from single-shot readings taken while the fixture was being edited
between runs, so two different programs were compared as though they
were one. Repeating each fixture seven times gives numbers that do not
move at all, and a different conclusion.

    single build, no carry        4.4 MB
    carried, no build block      17.6 MB
    carried, with build block    17.6 MB

The build block costs nothing. Removing it entirely leaves the peak
unchanged to the decimal, so 4.1 — a shape-preserving rebuild scoped
to where uniqueness is syntactic — has nothing to reclaim, and is
declined. The four times belongs to the carry, which copies any large
survivor whatever built it, and which already has its floor on record:
evacuation holds the survivor twice while the garbage is still live.

The lesson is the one the project already writes down about wall
clocks and forgets about memory: a single reading is not a
measurement. Peak resident varied two to one across runs of what I
believed was the same binary, which is exactly how a wrong number gets
into a merged log entry.

## 2026-07-28 — 4.3 has a prize and no customer

Auto-SoA asks the compiler to choose a layout, which it may do here and
not in Rust, where `&T` bakes in address identity and separate
compilation hides the access sites. The idea survives its own
measurement and dies on a different question.

The prize, measured on 200,000 three-field records traversed twenty
times reading one field: 50.3 ms and 23.8 MB held as an
array-of-records, 27.7 ms and 19.9 MB held as three parallel arrays
carrying the same data. That is 1.82x on time and 1.20x on peak, and
it is worth having. (The first attempt at this comparison was unfair —
it stored one field on the SoA side against three on the AoS side, so
it measured a smaller dataset rather than a different layout. Caught
and redone before drawing a conclusion, unlike the reading that went
into a merged entry earlier today.)

The customer is what is missing. The entry's own gate is multi-pass or
random access over a materialized collection, and exactly one workload
qualifies: vse traverses its electorate six times. vse holds no
records. A voter is a list of scores read as `v[c]`, so there is no
field for a field-touch analysis to see. Everything that does hold
records — the decoder, the encoder, kq — is single-pass, which is the
case fusion already owns.

So the transform is declined for want of a customer rather than for
want of value, and the ledger says so, with the number attached for
whoever reopens it. One adjacent thing is worth separating: vse's
electorate is a matrix stored by row and read by column, which is a
transpose, not a field-touch — and the ledger's own warning that the
transpose fights deforestation is about exactly that.

## 2026-07-28 — a wrong-arity call to an imported group reached the assembler

Sizing e-graph fusion wanted a chain split across functions, and the
fixture would not build: `list/range 1 5` — range takes one argument —
produced `use of undefined value '@d_list/range_2'` from the LLVM
assembler. Compiler internals, shown to a user, where a diagnostic
belongs.

The cause is a scope gap rather than a missing rule. The arity check
runs per file, over that file's own declarations, so it catches
`twice 1 2` and cannot see `list/range` at all; and the merged pass,
the only one that has the dependency's arms in scope, never checked
arity. Between them a wrong-arity call to any imported group passed
check, passed the interpreter with a clean runtime error, and reached
codegen on the native path to be caught by the assembler in its own
vocabulary.

check_call_arities now runs on the merged program, over qualified
names only — a bare name may be a local holding a function value, and
the per-file pass already covers the rest. The engines agree again:
the same error[arity], at check time and at run time, naming the count
it got and the counts the group takes. Pinned in the error corpus and
documented in appendix A beside the local-arity entry it belongs
next to.

Worth noting which harness caught the documentation slip: the panel
checker taught about `kanso play` titles this morning refused an
abbreviated output panel that nothing would have compared a day ago.

## 2026-07-28 — 4.5: the cliff is real, the customer is not

Fusion's function-boundary limit measured, on one chain written two
ways over 300,000 elements. Inside a single function: sixteen
allocations, 11.2 MB. With one link — the select — moved behind a
function boundary: 1,200,022 allocations, 92.8 MB. Fusion stops at the
call and the intermediate list is built in full.

That is a large prize and it has nobody to give it to. Thirty-seven
single-expression functions across vse, kq, std and the benches use an
adapter, and every one holds a complete chain: `sum (list/map voters
f)`, `argmax (to_list (list/map (range ncand) f))`, `min (list/map
(range ncand) f)`. The idiom the book teaches — write the chain where
it is consumed, or pipe it — is the idiom fusion already handles, so
the boundary is not crossed in practice.

Recorded with a second finding attached, because it changes what to
build if the shape ever appears: an e-graph is the expensive answer.
Inlining a small single-use function before fusion reaches the same
place, and the tree already does this twice — inline_builtin_wrappers
undoes a rename so uniqueness can see through it, inline_single_use_
chains folds a single-use adapter binding. A third case of the same
kind is a much smaller thing to build than equality saturation, and
should be tried first.

Three of section 4's five are now closed by measurement rather than
argument: 4.1 because the build block costs nothing, 4.3 because no
workload holds records it traverses twice, 4.5 because no workload
splits a chain. The pattern is worth naming — these were designed
against imagined programs, and the programs that exist are shaped
differently.

## 2026-07-28 — check refuses a literal no arm could take

Clay: "i don't know what the point is of having kanso check not fail
for all the same reasons build or run would fail." Three gaps were
measured; this closes the first, and the arity gap closed with it
earlier today.

The general form of the question wants the inference sets, and a join
is a superset — a wrong call can hide behind a right one at another
site, so the general check must fire only where the joined set misses
every arm, which is sound and blunt. A literal needs none of that. Its
type is on the page. So the check asks the narrow question exactly and
says nothing about anything else: an argument written as a literal is
compared against the arms that could take it, and refused only when
every arm is a definite no. An unknown type name, an interpolated
string, any computed expression — all pass unremarked.

Writing the quiet-case fixture is what found the second thing.
`widen 3` against `fn widen x:float64` fails at runtime: **dispatch
does not widen**, whatever arithmetic does with the same two types
(`1 + 2.5` is 3.5, and always was). My first model had float64 taking
an int and would have stayed silent on a program that cannot run. The
fixture built to prove the check quiet proved the model wrong instead,
which is the entire reason to write one.

Zero findings across std, kq, vse, examples and every bench — the
check is silent on all working code. Pinned both ways: the refusal in
the error corpus, and a micro program where a typeset, a wrapper and a
generic arm each admit their literal. Appendix A documents both the
refusal and the widening asymmetry, since the second is the surprising
half.

## 2026-07-28 — check refuses a literal no builtin would take

The second of Clay's three gaps. Each builtin's demand was established
by handing it the wrong thing and reading what it said — `length` takes
a list, string or map; `entries` a map; `bytes`, `char_code` and
`chars` a string; `sqrt` and `round` a number; `sleep`, `random` and
`from_code` an int — rather than by reading the implementation and
believing it. Absent entries constrain nothing.

A std module's function that is a rename over a builtin gets resolved
first, reusing the alias map the wrapper inliner already computes, so
`text/bytes 5` is refused and reported against `bytes`, the name whose
contract will actually be met.

Regenerating the book found the consequence: appendix A's example of
error[runtime] *was* `length 5`, and that program no longer reaches
runtime. The entry now shows a value computed one way and used another
— `length (measured 0)` — which is the case a checker cannot see, and
the prose says so. The kind did not lose its meaning; it lost the half
that moved to check time.

Zero findings across std, kq, vse, examples and every bench. Two gaps
of the three are closed; the third, a field that no record declares,
needs per-expression record types that the set-based inference does not
carry — the sets say REC, not which one. Recorded rather than
attempted.

## 2026-07-28 — the overflow-check ceiling, and why partial elimination is not worth building

The numeric parity finding left one lever: kanso matches Rust exactly
when both check overflow, so the only way further is to stop checking
where it is provably safe. The ceiling was measured before any analysis
was designed, by stripping the checks out of the emitted IR by hand and
rebuilding against the same runtime object — same answer, so the strip
is faithful.

Stripping all three checks takes the loop from 3.51 ms to 2.32 ms, a
third of the running time, and lands it under Rust's own wrapping
arithmetic. That is a real prize. What kills it is how the prize is
distributed.

    all checks on                 3.51 ms
    counter (n - 1) unchecked     3.43 ms     2%
    multiply (n * n) unchecked    3.21 ms     9%
    accumulate (acc + …) unchecked 3.01 ms    14%
    all three unchecked           2.32 ms    34%

The parts sum to twenty-five and the whole is thirty-four, so the
checks are not three independent costs. Each one is a branch, and the
branches are what stop llvm doing anything else with the loop; remove
two and the third still holds the door. Partial elimination is worth
close to nothing.

That decides the design question. The tractable analysis — a range on
a loop counter, where the bound is a literal at the call — buys the
two percent line. The thirty-four requires discharging the
accumulator's check too, and an accumulator's bound is inductive: it
needs to know what the fold reaches over the whole loop, which is the
AARA-shaped question section 5 of the ledger already names as one of
the three places undecidability actually bites.

So: not built, and not because it is hard in the abstract. Building
the part that is easy would buy two percent, and the part that pays
is the part the ledger already says has no general answer. If it is
ever wanted, the honest route is a language surface — an opt-in
wrapping arithmetic the author asks for — rather than an analysis
that promises to find it.

## 2026-07-28 — 4.4 is blocked by the block-born rule, not by the theorem

The last of section 4. Union-find is the sharpest test of it, because
path compression is precisely aliased mutation of a node reached by
traversal, so it either works or names exactly what does not.

It does not, and the boundary is razor-sharp. A `set` may write only a
name bound **directly** to a construction in the block. Four shapes,
all rejected with error[build]: a node taken from a list built in the
same block, a node reached by following a field, a plain alias
`c = a`, and a node chosen by `if`. Every algorithm the entry names —
union-find, a compile-time e-graph, NbE, unification — reaches its
nodes one of those four ways, so none of them can be written.

What build blocks do support is the case the book teaches, and it is a
real case: wiring a cycle among a fixed set of directly-named
constructions.

The rule is conservative rather than necessary, which is the useful
part of the finding. The theorem wants the cohort closed — everything
born in the block, nothing escaping — and a node reached by indexing a
block-born list is inside that cohort by construction. Making
block-born a dataflow property, flowing through aliases, conditionals,
indexes of block-born collections and fields of block-born nodes,
would admit these algorithms without touching the theorem. That is
scoped compiler work rather than a research question, and it is what
4.4 waits on.

With this, all five of section 4 are answered: three declined by
measurement, one already harvested, and one blocked on a rule that can
be widened. None of them was refuted by argument.

## 2026-07-28 — what this session cost the front end, and what the box cannot tell us

The published compile figures are 6.6 ms for kq's check and 6.1 for the
decoder's, dated three days ago. Today's readings for the same work sat
between 5.9 and 8.9 ms depending on the hour, so the absolute claim can
neither be confirmed nor refuted here; this box has not been idle all
session, and load between two and six moves the number by more than the
thing being measured.

What can be measured is the difference, and it was, by building the
commit from before this session's front-end work and racing the two
binaries back to back:

    before this session      7.46 ms
    now                      7.95 ms      +0.49
    now, provenance off      7.74 ms      provenance 0.21
    so the three check passes             0.28

Half a millisecond, for err provenance, imported-group arity, and two
literal-argument checks. That is the real cost and it is small, but it
is seven percent of a published six-and-a-half, so the next sitting
will show it and the note should say why.

Recorded rather than published: moving a dated absolute by a delta
measured on a different day would mix two sittings into one number,
which is the thing the surface-sweep rule exists to prevent. The
re-sit stays owed, and it wants a box that is actually idle.
