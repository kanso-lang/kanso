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
