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

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

## 2026-08-31 — the measurement the entry above asked for: the exclusion costs eleven lines

Stripping the imported-group exclusion entirely — `.filter(|_d| false)` in
place of the path-prefix test — and reading every vein the tree has, on this
container:

    sha256 via std/, arena_peak_bytes   90,177,536 -> 1,048,576   (86x)
    sha256 via std/, allocs                713,523 ->   713,807   (+284)

    compile_allocs        25,394   unchanged
    compile_peak_bytes   713,606   unchanged
    compile_rounds            40   unchanged
    compile_visits        16,806   unchanged

    decode, encode, one-shot, basket, escape, wide and pending-cell
    counters: all seven byte-identical
    machine code: byte-identical
    kq's three allocation cost goldens: byte-identical
    welfare: 74.33, unchanged

    emitted code: scanbench calls 3,743 -> 3,753, lines 20,019 -> 20,030.
    Every other benchmark byte-identical.

Eleven emitted lines in one benchmark is the whole measurable price, and the
+284 allocations are the evacuation copies the carry rewind pays for. The
compile veins do not move at all, which answers the worry the entry above
raised about `kanso check lib/json`: the library's own groups either never
wanted a carry or were already getting one.

The rule's stated hazard — a shared library driver threading its caller's
invariant source through the loop, so carrying copies an unbounded value per
iteration — does not appear anywhere in the corpus. That is a finding in its
own right and cuts both ways: it is evidence the exclusion is over-broad, and
it is evidence that nothing in the tree would catch the hazard if it were
real. A fixture that exercises it is owed either way, and it does not exist.

So the naive strip is measured and cheap, and it is deliberately NOT what
ships next. What ships next is the marker: `d.file` is a diagnostic field and
reading it as a semantic one is what made a directory name change a program.
Whether the exclusion then survives on a real marker is the question the
fixture above has to be written before answering, because a rule kept for a
hazard nothing demonstrates is a rule kept on faith.

## 2026-08-31 — CORRECTION: the exclusion has a pin, and it is about correctness

The entry above says the rule's stated hazard "does not appear anywhere in the
corpus" and reasons from there that the exclusion is over-broad. That is wrong,
and the way it was wrong is worth more than the conclusion was.

`beat::tests::json_decode_loops_stay_conservative` in src/beat.rs is the
corpus statement of it:

    assert_eq!(licensed, vec![("encode_items", 3), ("encode_pairs", 3)],
        "only the byte-builder encoders may rewind; scanners threading
         records or lists stay on the grow-only arena");

with a comment that argues safety rather than cost: the two encoders thread a
byte builder by pointer identity, "raw bytes hold no pointers, so nothing in
the accumulator can dangle across a rewind." Removing the exclusion admits
`list/bisect`, `list/found_in`, `list/holds_all?` and `list/holds_any?` to the
carry tier and turns that assertion red.

I missed it by searching design/compiler-log.md and the archive and not the
test suite. The filing gate in design/pending-gavels.md names three places to
search before calling a question unanswered, and all three are prose. A pin
lives in code, and this one carries an argument no log entry repeats.

So the naive strip is refused, and it should be. What looked like a
performance heuristic with no evidence behind it is a boundary on what may
rewind at all, and a carried list of records is on the far side of it. Whether
the carry machinery's evacuation already covers the case the comment worries
about — `k_carry_stage` and `k_carry_take` exist precisely so a carried value
survives a rewind — is a real question, and it is a question about pointer
lifetime that wants a differential fixture rather than my reading of a
comment.

What the measurements above still say, unaffected:

- Peak goes from linear in the message to flat when a digest's walk may
  rewind: 108,003,328 bytes to 1,048,576 at ten kilobytes, 426,770,448 to
  4,194,320 at forty-three, for 0.04% more allocations.
- At forty-four bytes — one padded block, nothing to reclaim — the same change
  costs 1,980 allocations to 5,259 on the interpreter and four natively. The
  rewinds are pure overhead when there is only one block.
- Only the digest fixture moves in the whole mem corpus, and only scanbench in
  the emitted vein (+10 calls) and the scan counters (beat_iters 15 -> 16).
- The compile veins, the seven runtime counter veins, machine code, kq's three
  allocation goldens and welfare are all unmoved.

And the defect is untouched and still real: `d.file.starts_with("lib/")` is a
relative path prefix, so a program built from a directory called `lib` gets a
different program. Fixing THAT without touching the boundary needs the marker,
because the `lib/` arm is what makes the repo's own `lib/json` behave like an
installed module — this very test compiles `lib/json` directly and depends on
it. A marker set where imports are resolved would have to answer for a module
compiled as a root as well as one reached through an import, and that is the
design question, stated properly at last.

Nothing shipped from this. The branch is reverted to main and the fixture, the
bisection and these measurements are the whole product.

## 2026-08-31 — the fixture the carry boundary was missing

The correction above says the carry exclusion is a boundary on what may rewind
and that the pin's argument — raw bytes hold no pointers, a list of records
might — wants a fixture rather than a reading of the comment. This is that
fixture, and it lands on its own because it is worth having whatever the
boundary turns out to be.

`tests/golden/micro/a_library_scanner_threads_records_across_a_rewind` carries
a window of eight records through four hundred rounds, shifting it each time,
and prints a field out of the element that has passed through the carry eight
times since the round that built it. `m/shifted/2` reads "beat: rewinds every
iteration", so the records really are live across a rewind, and both engines
answer `8 18974 19170`.

It is asked at the observable end. If a rewind ever left those elements
pointing into arena that had been given back, the printed field is where it
shows, and the micro corpus compares the interpreter against native and
against a release build — so a divergence appears as a diff rather than as a
plausible-looking number one engine invented.

Watched red before it was kept: shifting the window wrong by one slot
(`s[6]!` twice) turns both the library arm and the release-built arm red on
the right line. It passes today because these groups are not carried today;
it exists so the change that admits them has something to be wrong against.

The corpus had nothing in this shape. Every mem fixture pins allocation counts
and every micro fixture that touches records reads them without a rewind in
between.

## 2026-08-31 — the carry exclusion removed, measured properly, and REVERTED

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md,
design/*.md, and — after the correction two entries above — the test suite,
`beat.rs` itself, and the runtime functions the exclusion's comment makes
claims about.

The exclusion is `d.file.starts_with("std/") || d.file.starts_with("lib/")` in
`beat_loops`, and three things are wrong with it. Two of them survive this
entry; the third is the reason nothing shipped.

**The field is a diagnostic.** `file` is what error origins are built from, so
`std/sha256` and a user directory called `lib` read alike, and the same package
compiled from `lib/app` holds 27,262,976 arena bytes where the same sources
under `elsewhere/app` hold 1,048,576. `tests/a_program_is_not_its_directory.rs`
pins BOTH numbers now, in the shape `tests/sha256_peak.rs` already uses: the
defect asserted as a fact, so it cannot be lost while the fix waits.

**The premise the comment gives is false.** It says a shared library driver
threads its caller's invariant source through the loop, so carrying it copies
an unbounded value per iteration. `k_beat_iter_carry` copies only what lies
above the mark, and a loop's mark is pushed at entry, so the caller's source is
below it and shared. A fold over a list built through `std/list` allocates
556,768 bytes at two thousand elements and 2,223,856 at eight thousand.

**And it is not enforcing the pin that refused its removal.** With the test
gone, `lib/json`'s licensed set gains `list/bisect`, `list/found_in`,
`list/holds_all?` and `list/holds_any?` — three boolean predicates and a binary
search — and every group json itself declares reads the same on both sides.
json's scanners are refused by the classifier. The exclusion's whole live
effect is those four predicates and sha256's cluster.

WHY IT IS REVERTED. The trade is memory for time and the time is quadratic.
Native, ASCII message read from a file, each arm built in its own directory:

| message | before, time | before, peak | after, time | after, peak |
|--------:|-------------:|-------------:|------------:|------------:|
|   8,000 |      0.084 s |   79,691,776 |     0.309 s |   1,048,576 |
|  16,000 |      0.170 s |  159,383,552 |     1.152 s |   1,048,576 |
|  32,000 |      0.331 s |  316,669,952 |     4.367 s |   1,048,576 |
|  64,000 |      0.675 s |  633,339,920 |    16.535 s |   4,194,320 |
| 128,000 |      1.304 s |1,262,485,520 |    68.168 s |   4,194,320 |

The digest is linear in time today. With the carry it is quadratic — 52x at
128 KB, and CI said so before this table existed: the `asset digests` job
digests a 1,604,098-byte wasm and took 49 seconds on main, then sat past
twenty-five minutes on the branch. Trading 1.2 GB for 52x is not a trade this
project's weights would take, and welfare cannot arbitrate it because no
benchmark in the suite streams (**OPEN**, and the reason a digest benchmark is
worth building).

**HOW THE FIRST REPORT GOT IT WRONG, because the mistake is easy to repeat.**
Three separate A/B runs said the change was wall-clock neutral. All three were
invalid. `kanso build` caches the native binary, and rebuilding the RUST
compiler does not invalidate that cache — the key is over the sources and
`runtime.c`, not the compiler. So rebuilding `src/beat.rs` and re-running
`kanso build` in the same directory re-ran the SAME binary in both arms, and
the two columns agreed because they were one column. The numbers above come
from a fresh directory per arm. Any A/B on emitted or runtime behaviour has to
build into a directory the other arm never touched.

WHAT IS LEFT OF THE PEAK, run down while the branch was still alive and true
whichever way this goes: `sha256/padded_bytes` opens with `list/to_list b`, so
the message is held as a list of integers at sixteen arena bytes an input byte
for as long as `digested` indexes it. massif at 64 KB put 46.17% of the peak —
2,097,184 bytes — on `k_b_push_into_proven` under `d_list/fold_3`, the buffer
that list is grown into. A program that reads the same file and prints its
length holds one block at 400 KB with eight allocations, because a bytes value
never becomes a list. That is lib/sha256's to fix and nothing here is in the
way of it.

TWO THINGS FOUND ON THE WAY, both surviving the revert:

- **`thunk_evals` is not engine-shared and never was.** Measured on main: a
  1,000-byte sha256 reads `thunk_forces=1024 thunk_evals=1024` native and
  `1088`/`17` interpreted. `k_memo_outlives` declines the memo when the answer
  was built inside the innermost beat; the interpreter has no arena to rewind.
  `mem_corpus_interp_matches_the_semantic_counters` classes evals with allocs
  and forces as engine-shared semantics, and no fixture has ever asked. The
  classification is left alone here — a gate that is not failing is not
  weakened on the way past — and the measurement is recorded so the next
  fixture in that shape is not a surprise. **OPEN.**
- **Two tests in `a_file_that_is_not_text.rs` shared one temp directory and one
  `run.kso`, and raced.** Caught once as `an entry file needs at least one
  statement` about a file that has one. One directory per test now, the same
  fix kanso#1169 made for the playground pair. That is the only compiler-facing
  change that ships from this branch.

WHAT SHIPS: the race fix, the directory defect pinned with both its numbers,
and this entry. The removal is built, measured and declined — recorded so the
next attempt starts from the curve rather than from the idea.

**OPEN, and it is the whole question now:** where the quadratic is. Every
deterministic counter is linear across the same range — allocs, alloc_bytes,
beat_iters, evac_allocs, evac_bytes all double when the message doubles — so
the cost is in work no counter watches. `k_beat_iter_carry` sizes and copies
the carried slots per iteration and `k_copy_size` walks rather than counts;
that walk is where to look first.

## 2026-08-31 — the quadratic has a name: k_slots_survive, at 80.66%

The entry above left it open. callgrind, on the digest built WITHOUT the
exclusion, 8,000-byte ASCII message from a file, 3,871,213,343 instructions
total:

```
3,122,380,806 (80.66%)  k_slots_survive
  106,378,584 ( 2.75%)  k_index
   73,497,528 ( 1.90%)  k_b_bit_shr
   71,987,328 ( 1.86%)  d_thunk_eval
   67,448,520 ( 1.74%)  k_shift_of
```

Four fifths of the run, in one function, on a program whose every allocation
counter is linear.

`k_slots_survive(slots, n, m)` loops over a node's immediate interior asking
`k_survives_x` of each heap slot, and its answer decides whether `k_copy_size`
and `k_deep_copy` may SHARE the node instead of copying it. For a list that
loop is `l->len` long, and the list the digest's evacuation reaches is
`padded` — the whole message as a list of integers, from `list/to_list b` in
`sha256/padded_bytes`. So one ask is O(message), the evacuation asks per
iteration, and the iterations are O(message).

THERE IS ALREADY A MEMO ABOVE IT and it only helps in one direction. The
`K_LIST` arm of `k_interior_survives` caches, for lists of `K_ISV_MIN` (64) or
more, whether the list survives the OUTERMOST mark — keyed on the list pointer,
invalidated when `k_beat_stack[0].ptr` moves. When that cached answer is yes it
returns 1 and the scan never runs. When it is no it falls through to the full
`k_slots_survive` anyway, and stores the no. `padded` is built long after mark
zero, so it is never an outer survivor, so the memo answers no forever and
every ask pays the whole list.

So the shape of a fix is narrow rather than architectural: the negative answer
has to be worth something too. Whether a node's interior survives depends on
the mark it is asked about, and within one evacuation that mark is fixed —
which is the seam. Nothing is built here; this entry is the profile and where
it points.

WHAT ELSE IT SAYS. A counter for this would have caught the regression in the
cost goldens instead of in a CI timeout: `evac_bytes` counts what an evacuation
COPIES, and the whole cost here is in deciding not to copy. Slots examined per
evacuation is platform-invariant, algorithm-level, and exactly the missing
dimension. That is a presence counter this project's own rule already asks for
and it does not exist.

The 2026-08-30 note beside `k_is_heap` says `k_copy_size` is 36% of deepbench
and that a tenth `case` in the tag switch cost that benchmark 6.14%. The same
walk is 80.66% here. deepbench is the closest thing the suite has to this
shape and it is nowhere near it.

## 2026-08-31 — digestbench: the one benchmark whose peak is the point

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md,
design/*.md, and the benchmark corpus itself — `bench/` and
`scripts/gates/build_benchmarks.sh`, which is the list this joins.

The entries above this one describe a change that took an 8 KB sha256 digest
from 79,691,776 arena bytes to 1,048,576 and scored **exactly zero** against
welfare, and whose 52x wall-clock regression nothing in the corpus was shaped
to notice. Both blindnesses are the same gap: every program the suite weighs
either holds its whole subject on purpose — the decoder and the encoder do —
or is small enough to sit in the arena's first block. So "does the peak grow
with the input" is weighted at zero by omission.

`bench/digestbench` is that shape. It reads `bench/digest_input.txt` — 8,192
deterministic printable bytes — at runtime and digests it once. Runtime read
for the reason bench/make_jsonbench already gives: a compile-time fixture
would let the optimizer fold the digest and flatter every row. Once rather
than in a loop, because the peak of one run is the number this exists for and
a loop over a walk that never reclaims would measure the loop.

What it reads today:

```
allocs=652817  alloc_bytes=81846129
arena_blocks=79  arena_peak_bytes=82837504
```

82,837,504 arena bytes for 8,192 bytes of message — 10,112 bytes an input
byte. sha256 walks its message sixty-four bytes at a time carrying eight state
words, and everything a block builds is dead when the next one starts, so that
number is a property of the algorithm in a compiler that reclaims between
blocks and a property of the MESSAGE in one that does not. It is the second
case, and `scripts/gates/digest_counters.sh` says so where a reader will meet
it.

It costs 0.333 s, which sits between jsonbench at 0.283 and encodebench at
0.819 — the middle of what the suite already pays.

WHY THE EMITTED VEIN WORSENS, and the gate defect it exposed.
`emitted_other_defines` 1,163 -> 1,323, `emitted_other_calls` 12,739 ->
14,310, `emitted_other_branches` 7,484 -> 8,341 and `emitted_other_lines`
73,669 -> 82,751. Those four rows are SUMS over the benchmarks in
`bench/emitted_golden_others.txt`, and a tenth benchmark adds a tenth
program's code to each. No existing program emitted a line more than it did
before; digestbench's own row is 160 defines, 1,571 calls, 857 branches and
9,082 lines, and the four deltas are exactly it. The trend gate called that a
pure regression and refused the change.

The gate was right to sum and wrong to compare. `on_line` drops a line's
leading name so the fields add across a golden's samples, which is the
treatment compile_golden's five samples get and is correct for a fixed set —
and meaningless the moment the set changes, which is the error library_box.sh
names in its own domain. `against _ ""` already said a golden this branch
CREATED has nothing to be compared against; the gate now says the same one
level down, for a row added inside a golden that existed.

It drops the joining sample from the current side and sums both over the
samples they share. THE FIRST DRAFT SKIPPED THE WHOLE GOLDEN instead, and the
mutation caught it: with `scanbench calls` moved 3,743 -> 9,999 in the same
commit that adds a benchmark, the skip passed silently and the intersection
still fails. Three mutations were run and all three answer right — a rise with
no sample joining, a rise with one joining, and the real case where only a
sample joins.

STILL OPEN, and deliberately not done here: the welfare wiring. Adding
`digest_peak_bytes` to `run_memory_counters` and `digest_instructions` to
`run_speed_counters` re-baselines the index — every term is a ratio against
`bench/welfare_floor.json`'s baseline block, so a new counter needs a baseline
entry and the floor re-set with its reason. That is the change that makes the
objective able to arbitrate the trade it could not arbitrate today, and it
wants doing on its own with the ratchet read carefully rather than bolted to
the benchmark's arrival.

---

## 2026-09-01 — an unclassified counter could pay for a worsening

**DONE.** The trend gate refuses "a pure regression: something got worse and
nothing got better". `worsened?` reads a counter's direction out of two tables,
`lower` and `higher`, and answers **false** for a name in neither. The listing
then printed every move that was not worsened as `improved`, and `better` was
built by rejecting the worsened ones. So a counter no table classifies landed
in `better` whichever way it moved, and no worsening was ever alone.

Found while lifting the carry-tier prefix (see the entry above): that change
read `evac_allocs 27 -> 33,827`, `evac_bytes 1,520 -> 2,705,520` and
`thunk_evals 1 -> 64` as three improvements. Each of those is work done, and
each rose.

A move whose counter no table names is now a third state. It prints as `moved`
under an UNCLASSIFIED heading and counts toward neither side, so the
pure-regression rule is decided only by counters with a direction. The heading
also names them, which is the actionable half: a counter nothing classifies
wants classifying, and until it is, the gate says so out loud rather than
quietly crediting it.

The direction tables are deliberately NOT extended here. Adding
`evac_allocs`/`evac_bytes` to `lower` and calling it done would leave the same
hole for the next unnamed counter, and the tables are a judgement about what a
counter means that wants making one at a time rather than in a batch beside a
gate fix. **OPEN:** the mem corpus alone carries `cohort_frees`,
`cohort_kept`, `evac_allocs`, `evac_bytes`, `put_mut_fast`, `put_mut_grow`,
`push_mut_fast`, `push_mut_slow`, `thunk_evals`, `str_scans`,
`str_scan_bytes`, `view_allocs`, `view_frees`, `perm_live_bytes` and
`perm_peak_bytes` with no direction. The gate now lists them on any run that
moves one.

Ratchet row `trend_adrift`, mutation
`a_worsening_paid_for_by_an_unclassified_counter`: it raises `scanbench calls`
(in `lower_d`, a real worsening) and `evac_allocs` in the digest golden (in
neither table) in the same patch. A gate that reads the second as an
improvement goes green; this one stays red.

---

## 2026-09-01 — a memo declined inside a beat, and the third state that keeps it

**DONE.** Closes the 64x thread opened on 2026-08-31.

`k_force` memoized a thunk's answer only when `k_memo_outlives` said the
storage sat below the innermost beat mark. Inside a beat it usually does not,
and the memo was declined outright: the cell stayed unforced and the whole
computation ran again on the next force. In the streaming shape that is once a
block. On the 8,192-byte digest, `thunk_forces` and `thunk_evals` were both
8,256 — every single force re-evaluated.

**The third state.** The memo is kept and marked AT RISK instead of declined.
A rewind may take the answer away; a carry evacuation may move it somewhere a
rewind cannot reach, which `k_deep_copy`'s thunk arm already does on purpose
(`if (t->forced) t->result = k_deep_copy(t->result, cp)`). Which of the two
happened has a cheap answer — is the storage still in the live chain — and it
is asked at the next force, where it costs a walk of a block list instead of a
re-evaluation. `forced` carries three values now rather than two, so the flag
rides on the cell and there is no second structure to keep in step.

**Why not a register of at-risk cells checked at each rewind.** That was
written first and thrown away. A thunk cell is refcounted and can be freed and
handed out again while a list still names it, so the list would have to be
kept in step with the free list, and touching a freed cell to un-memo it is the
bug the register exists to prevent. The flag cannot go stale because it dies
with the cell.

The captures are NOT dropped for an at-risk memo: a cell that may be asked to
run again has to still have them. A kept memo drops them as before.

**Measured on the digest benchmark, exclusion untouched, main as the base.**

| counter | before | after |
|---|---:|---:|
| thunk_evals | 8,256 | **129** |
| allocs | 652,817 | **230,214** (-64.7%) |
| alloc_bytes | 81,846,129 | **54,149,841** (-33.8%) |
| arena_peak_bytes | 82,837,504 | **54,525,952** (-34.2%) |
| arena_blocks | 79 | 52 |
| push_mut_fast | 628,289 | 514,511 |
| push_mut_slow | 41,479 | 25,225 |
| sh_buf | 73,376,000 | 52,051,280 |

Wall clock, interleaved against a worktree of origin/main built in its own
directory, best of five, this container: **0.093 s -> 0.039 s**, 2.4x.

**`digest_buf_reuse` 24,768 -> 16,640 is the counter that fell the wrong way,
and it is the same fact.** A buffer is reused when a builder finds one to
reuse; 8,127 fewer evaluations build 8,128 fewer buffers, so there are fewer
reuses to count. Nothing lost a reuse it used to get.

**Every other vein is byte-identical**: decode, escape, pend, oneshot, basket,
encode, wide, scan counters, and the emitted-code golden. The machine-code
golden falls about a hundred bytes a binary — `k_force` gained a state and lost
the branch that declined a memo.

**What it cost, and why the first measurement of it was misleading.** CI on
the pinned host reported four benchmarks worse: deepbench +0.59%, oneshot
+0.46%, basket +0.44%, pendbench +0.34%, with every allocation counter on
those programs byte-identical. Nothing was doing more work, which is the shape
that says look at the code rather than the algorithm.

callgrind named it: `k_force` at 4,752,000 instructions on deepbench, a
function that does not appear in main's profile at all. It had outgrown what
the inliner would take, so every force paid a call and a prologue it used to
get for free. The delta was 4,375,985 and `k_force`'s own cost was 4,752,000 —
the same number twice.

So the cold half is out of line now (`k_force_slow`, `noinline`) and `k_force`
is the memo hit and nothing else, which is the shape it had before. deepbench
reads **726,483,240** against main's 726,483,254 on this container: fourteen
instructions BELOW main, measured on the same host with both binaries built in
their own directories.

`text` rises 740,226 -> 741,202 across the nine binaries, about 108 bytes
each.
`k_force_slow` is a real function now where the whole of `k_force` used to be
one, so there is a second prologue and a call site to pay for. That is the
price of the fourteen instructions above, and it is the trade the outlining
makes on purpose.

The lesson is not about thunks. A runtime function that grows past the
inliner's budget charges every caller, and no counter in the tree can see it —
allocations, arena blocks and evacuations were all identical while 0.6% of
deepbench went missing. kanso#1186 outlined `k_b_append_grow` for the same
reason; this is the second instance, and the first where the growth was
incidental rather than intended.

**The work vein after the split, measured by CI.** Nothing regressed and one
row fell properly:

| counter | before | after | |
|---|---:|---:|---:|
| `work_pendbench` | 946,378,074 | 937,566,473 | **-0.93%** |
| `work_encodebench` | 8,396,569,110 | 8,396,587,878 | +18,768 |
| `work_indexbench` | 5,243,094 | 5,243,104 | +10 |
| `work_widebench` | 63,997,213 | 63,997,231 | +18 |
| `work_jsonbench` | 2,838,415,853 | 2,838,415,815 | -38 |
| `work_basket` | 56,458,062 | 56,458,024 | -38 |
| `work_deepbench` | 726,486,934 | 726,486,920 | -14 |
| `work_escapebench` | 253,819,096 | 253,819,082 | -14 |
| `work_oneshot` | 43,094,978 | 43,094,978 | 0 |
| `work_digestbench` | — | 152,573,619 | joins |

`compile_instructions` 41,496,870 -> 41,496,028, a fall of 842 and free.

The deepbench row is fourteen below main, which is exactly what this container
measured before the push — the local method and CI agree to the instruction on
the one row both could see. pendbench's 0.93% is the memo doing its job on a
program that forces cells inside a beat and used to re-evaluate every one.

Welfare rises and is banked in the same change, per the rule that a gain
nobody ratchets is a gain the next change is free to spend.

**A fourth memo state, built on the misattribution and removed.****A fourth memo state, built on the misattribution and removed.** Before
callgrind was asked, the four rises were blamed on the at-risk check running
again and again on cells whose answer a rewind keeps taking, and a K_MEMO_SPENT
state was added so such a cell stops asking. It was measured after the split
and buys nothing: deepbench reads 726,483,240 with it and 726,483,240 without,
and every counter in every vein is byte-identical either way. No program in the
corpus exercises it. So it is gone, and the three states stand.

The comment it carried asserted the 4,375,985 as ITS motivation, which was the
misattribution written down as fact — the same number was `k_force`'s own
inlining cost, and a plausible story reached for it first. A state that costs
nothing to keep is still a state somebody has to read, and this one had a wrong
measurement attached.

**An intermediate state worth recording, because it nearly changed the
objective.** Before the split was found, the four rises were taken at face
value: welfare read 74.32 against a floor of 74.33, a fall with nothing on the
other side, and the argument being drafted was that the index is blind to a
streaming program's peak and should gain a digest term. That argument is still
true and still worth making one day — welfare cannot see this change's 64.7%
and 34.2% — but it was about to be made in service of a cost that did not
exist. A model is easiest to argue with when a measurement is embarrassing,
which is exactly when the measurement deserves another look first.

Also learned on the way: welfare already has a rule for a counter joining the
model — `baseline_of`/`entering` gives it the standing its dimension already
has, so a new term is score-neutral on the day it lands and only improvement
after that pays. Hand-writing a baseline into `bench/welfare_floor.json` was
the wrong instinct and the machinery was already right.

**Welfare is 74.3323 before and after**, and now for the right reason: nothing
regressed. No digest counter feeds the index, so a 64.7% fall in allocations
and a 34.2% fall in peak on a real program still scores zero — the omission
digestbench (kanso#1195) exists to make visible. Deliberately not wired here:
adding a term to the objective in the same change the term would reward is the
wrong order, and it is a weaker argument now that nothing needs it. **OPEN.**

`digestbench` joins `bench/instructions_golden.txt` in this change even so. A
benchmark counted on one axis and not the other is a trade the index cannot
see, and every change that buys the digest's memory spends something in work.
The vein is a tripwire; whether welfare gets a term from it is a separate
question and not this change's to answer.

**The reduced fixture that would not reduce.** Four attempts at a
postcard-sized program: a body binding used once, one used behind a two-arm
dispatch, one built outside the loop and passed in, and one built inside the
loop's own cluster and handed to a thirty-round inner loop — the shape sha256
has, where `w = schedule ...` in `blocked` is read by `compress` on each of
sixty-four rounds. Every one of them compiled strict: the demand analysis
proved the binding needed and no thunk was allocated at all. So what pins this
is bench/cost_golden_digest.txt, whose `thunk_evals` row is exact and which
went 8,256 -> 129 under this change and back again when it was reverted. That
is the observation the rule asks for, on a fixture larger than the rule wants.
**OPEN:** what keeps sha256's schedule lazy where four hand-written copies of
its shape are strict. Whoever answers that gets the postcard.

**What this does NOT close.** The carry-tier prefix removal, built and measured
on 2026-08-31 and on the entry above, is still not landed. This change removes
the reason it could not be: with the memo kept, lifting the prefix no longer
takes native's `thunk_evals` to 64 where the interpreter reads 1, which is the
differential the oracle refuses. Re-measure and land it next.

## 2026-09-01 — the digest joins the objective, on both sides

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md,
design/*.md, design/pending-gavels.md, and `scripts/welfare/welfare.kso`'s own
counter lists, which are the record of what the model has ever weighed.

The digest benchmark landed on 2026-08-31 with a CI gate, a ratchet row and an
emitted-code row, and welfare could not see a single one of its counters. So
the gap the benchmark was built to close was still open: a change that takes an
8 KB digest from 79,691,776 arena bytes to 1,048,576 still scored zero, and so
did the 52x that bought it.

**What the model reads now.** `bench/cost_golden_digest.txt` joins the eight
goldens welfare already chains through, and three counters come off it:

| counter | dimension | today |
|---|---|---:|
| `digest_instructions` | run speed (satiation 2.0, weight 0.30) | 152,573,619 |
| `digest_peak_bytes` | run memory (2.0, 0.30) | 54,525,952 |
| `digest_arena_blocks` | run memory | 52 |

The instructions row comes from the work vein rather than from the digest
golden, the same place every other speed counter comes from.

**Both sides, and this is the whole point of the entry.** The peak is the row
digestbench exists for. A term that priced only the peak would rank any change
that reclaims per block above one that does not, however long it takes — which
is precisely the trade the 52x slowdown was, scored as free in the other
direction. Pricing the work beside the retention turns that into a number the
model settles instead of a preference somebody argues. Adding only the memory
half would have been rigging the answer to a question still open.

**The score did not move, and that is the entering rule working.** A counter
new to the model enters at its dimension's standing, so run speed's eight
counters and run memory's eight leave both terms where they were. The floor
went from 74.33468320932070 to 74.33468268729726 — a fall of 5.2e-7, which is
the rounding in `math/round` on the three entering baselines and nothing else.
Ratchet 132 records why.

**One pinned number moved.** `one_counter_running_away_cannot_carry_its_term`
answered 49.52 and answers 49.16: the fixture puts one run-speed counter a
thousand times better than its baseline and the rest at parity, so the term is
`(7/3 + 1024/1026) / 8 * 0.30` where it used to be over seven. The number is a
property of the weights and the counter count, not of the compiler.

**The spec, watched red.** `tests/the_digest_is_priced_on_both_sides.rs` stages
`bench/`, doctors one row of one golden tenfold, and requires welfare to go red
naming the digest row that moved — once for the peak, once for the work. A
third fixture runs the undoctored goldens and requires green, so neither of the
first two can pass on a welfare that fails on everything. With `digest_work`
and `digest_memory` taken back out of the counter lists both doctored fixtures
go red and the control stays green; that is the failure that was watched, and
the message on each is the assert's own sentence rather than a parse error.

**What this unblocks.** The carry tier measured on 2026-08-31 trades 2.24x on
the clock for 68x less peak at 131,072 bytes, and 1.6x for 52x at 8,192. That
is a trade across two dimensions, which is the one thing the per-counter
goldens cannot arbitrate and the one thing the index is for. It is now a
question with an answer rather than a judgement call.

## 2026-09-01 — the carry tier, arbitrated: DECLINED at −0.56, and the reference that decides it

SEARCHED FIRST: design/compiler-log.md (the 2026-08-31 entries "the carry
exclusion removed, measured properly, and REVERTED" and "the quadratic has a
name"), design/log/compiler-log-archive.md, design/pending-gavels.md — whose
charter bounced the sha256 question on 2026-08-29 with "performance questions
with no surface area are the implementer's", so this decision is recorded here
rather than sent anywhere.

The entry above put digestbench's peak and work into the objective so this
trade could be settled by the model. It has been.

**The arms.** `origin/main` against `wip/carry3`, which removes beat.rs's
`std/`/`lib/` prefix exclusion so a library loop carries like any other, and
adds `k_isv_flat` — a mark-free memo on the interior-survives walk, keyed on
the items pointer, that answers "this list holds no heap slot and no thunk"
without rescanning. Each arm built in its own directory, every benchmark binary
deleted before rebuilding, both swept on this container. The main arm
reproduced all six allocation goldens byte-identically, which is what says the
sweep measured the compiler rather than a stale binary.

**What moved.**

| counter | main | carry |
|---|---:|---:|
| digest arena_peak_bytes | 54,525,952 | **1,048,576** |
| digest arena_blocks | 52 | **1** |
| digest allocs | 230,214 | 653,077 |
| digest thunk_evals | 129 | **8,256** |
| digest beat_iters | 56 | 16,826 |
| digest evac_allocs | 27 | 66,851 |
| digestbench instructions | 152,573,220 | **820,087,049** (+437.5%) |
| compile_instructions | 41,921,600 | 41,800,396 (−0.29%) |

Every other benchmark is within a tenth of a per cent, and `scan`'s
`beat_iters` moves 15 to 16. Nothing else in any vein.

**The at-risk memo and the carry tier are in direct conflict, and this is the
mechanism.** `thunk_evals` goes back to 8,256 — equal to `thunk_forces`, so
every force re-evaluates, which is the state before yesterday's memo. The memo
is correct and it is being correctly invalidated: the carry evacuation copies
what is reachable from the staged value and rewinds the rest, and a memoized
result the loop does not carry is exactly what the rewind takes. So the 52x
memory win is bought by discarding the memos, and the recomputation is where
the 5.4x work goes. Both changes are right on their own and each one's win is
the other's cost.

**The verdict: 74.31 → 73.75, a fall of 0.56.** Same host, same sweep, the
repo's own baselines. Welfare names `digest_instructions` as the term that
paid, at 0.373 points. **The carry tier is DECLINED.** wip/carry3 is not
merged and the prefix exclusion stays.

**A second measurement, which is the part worth arguing about.** Rerun both
arms with the three digest baselines set to the main arm's own values — every
digest ratio exactly one — and the answer reverses: main 70.14, carry
**72.99**, a rise of 2.85. The same change is worse by 0.56 or better by 2.85
depending on nothing but where the digest counters' reference sits.

The reference is not a measurement. `entering` chose it this morning so a new
counter would not move its dimension on landing day, which put
`digest_peak_bytes` at a ratio of 11.2 — the model now asserts that the
digest's memory has already improved elevenfold, and it has no history at all.
On a satiating curve that assertion is not free: a counter placed at 11.2 is
84.8% satisfied and a further 52x buys 0.15 of satisfaction, where the same 52x
from parity buys 0.63. The rule spends most of a new counter's headroom on the
day it enters, and the same placement makes the work counter's regression
proportionally dearer. Both effects push the verdict the same way.

**This is not grounds to move the reference now.** The rule was written for a
real problem — entering at parity costs welfare on landing day, and here it
costs 4.17 points — and a model is easiest to argue with when its answer is
inconvenient, which is when the measurement deserves another look first. That
was written in this log four days ago about this same benchmark and it applies
to me again. So the decline stands on the model as it is.

**The question the two numbers leave open, stated once and left open.** A
newcomer needs two things that `entering` supplies with one number: a reference
that reflects where the counter actually stands, and no score move on the day
it lands. They can be separated — enter at parity and absorb the landing-day
step in the floor, in the same PR, with the reason recorded. That is what
`--set` exists for, except that `--set` refuses a fall by ruling, so the step
would have to be a hand edit of bench/welfare_floor.json where a reviewer sees
it, which is the designed override and the right shape for "the model gained a
term". The cost is a visible four-point drop in the published number and an
honest one in place of an invented history. Nobody should settle this while a
particular change's verdict hangs on it; digestbench is the only counter that
has ever entered by this rule, so there is one instance and it is this one.

## 2026-09-01 — why sha256's schedule is lazy: an arm that ignores a parameter

SEARCHED FIRST: design/compiler-log.md — this closes the question the
2026-08-31 memo entry left open ("what keeps sha256's schedule lazy where four
hand-written copies of its shape are strict") —
design/log/compiler-log-archive.md, design/*.md, and
tests/golden/mem/a_digest_holds_every_block_it_walked.kso, whose comment named
the four copies.

`digested` binds `w = schedule ...` and hands it to `compress s w 1 false`,
which reads `w[at]!` sixty-four times. The binding is a thunk, and the reason
is one arm:

```
fn compress s _ _ true
  s

fn compress s w at false
  ...
```

The exhausted arm ignores `w`, so `w` is not demanded on every path into
`compress`, so the argument is thunked. Demand is decided per parameter across
all arms rather than per call site, and the call site's guard is the literal
`false`.

**A controlled pair, built to check it rather than to argue it.** Two programs
identical but for the exit arm: one written `fn walk s _ _ true` reads
`thunk_allocs=1`, `thunk_forces=8`, `thunk_evals=1`; the same program with the
exit arm reading `w` reads `thunk_allocs=0`. `allocs=5` in both. The four
hand-written copies were strict because their consumers read the parameter in
every arm.

**Costed, and no change is warranted.** callgrind on digestbench puts
`d_thunk_eval` at 1,151,583 of 152,573,220 — 0.75%, and that is the 129 real
evaluations of the schedule, which a strict compile would do too. `k_force`
does not appear in the profile at all: #1197 outlined its cold half and the
hot half is inlined, so 8,256 forces cost less than the annotator's threshold.
The obvious refinement — decide demand per call site when the guard argument is
a literal, which would make `w` strict here — buys a fraction of a per cent of
one benchmark and adds a case to a pass that already runs on every declaration.

The behavior is pinned where it already was:
`tests/golden/mem/a_digest_holds_every_block_it_walked.mem` reads
`thunk_allocs=1` and `thunk_forces=64` for a one-block message, so a demand
analysis that started answering differently moves that golden.

## 2026-09-01 — sixteen counters gain a direction, five keep none on purpose

SEARCHED FIRST: design/compiler-log.md (the 2026-08-26 entry adding the two
measured compile veins to these tables, and the 2026-09-01 entry adding
digestbench), design/log/compiler-log-archive.md, design/*.md, and the tables
themselves in scripts/trend_gate/trend_gate.kso, which are the record of what
has ever been classified.

Twenty-two counters across the goldens had no direction. The gate's third state
prints them under UNCLASSIFIED and counts them toward neither side of the
pure-regression rule, which is right for a counter whose direction is genuinely
unknown and wrong for one nobody had got round to. Sixteen were the second
kind.

**To `lower`, twelve.** `evac_allocs` and `evac_bytes`, what a carry evacuation
spends. `str_scans`, `str_scan_bytes`, `find2_calls`, `utf8_bytes`, bytes
walked. `perm_live_bytes` and `perm_peak_bytes`, malloc-backed storage the
process still holds — it only leaves through `free()`, so a peak that scales
with iteration count is a leak by definition, which is what the counter's own
comment in runtime.c says. `append_grow`, `push_mut_slow` and `put_mut_grow`,
the slow halves of three pairs whose fast halves were already classified.
`thunk_evals`, a lazy binding actually running.

**To `higher`, four.** `push_mut_fast` and `put_mut_fast`, the fast halves.
`bytes_freed`, what `bytes_malloc` is measured against. `carry_dedup`, a node
the carry found already copied and reused rather than copying twice.

**Five stay out, and the reason is written beside them.** `cohort_frees` and
`cohort_kept` are the two outcomes of the cohort dance and both scale with how
many cohorts ran, so what means anything is the ratio and this gate cannot say
ratios. `view_allocs` and `view_frees` are the same shape, and runtime.c says
so at the counter: the difference is memory the process is still holding, and
either side alone is not. `thunk_forces` counts asking a thunk for its value
rather than computing one — #1197 took `thunk_evals` from 8,256 to 129 on the
digest with `thunk_forces` byte-identical, which is what a working memo looks
like, and a change that made the same program strict would lower it while doing
identical work.

**The risk the `higher` table carries, stated rather than left to be found.** A
change that does strictly LESS of the work lowers a presence counter, and the
table reads that as a worsening. `append_fast` and `utf8_zerocopy` have sat
there since the table was written without it biting, because a change that
removes work almost always moves its slow twin the same way and the verdict
comes out mixed. A pure regression that is really a pure simplification is the
shape to watch for.

**A ratchet row was about to start passing for the wrong reason, and this is
the part worth remembering.** `a_worsening_paid_for_by_an_unclassified_counter`
raised `scanbench calls` and `evac_allocs` together: one classified worsening
beside one unclassified move, so a gate that intersects correctly refuses the
first and lets the second count toward neither side. Classifying `evac_allocs`
makes BOTH halves classified worsenings — which any pure-regression rule
refuses, so the row goes on turning the gate red while testing nothing it was
written to test. Verified by running it: before the repoint the listing had no
UNCLASSIFIED section at all.

The mutation now raises `thunk_forces`, which stays unclassified on purpose,
and `tests/a_mutation_keeps_its_unclassified_counter.rs` reads the counter out
of the mutation's own `sed` line and requires the tables not to name it. It was
watched red by putting `thunk_forces` into `lower_k`. That is the general
shape: a classification sweep can silently disarm a mutation that depends on
something being unclassified, and nothing else in the tree was watching that
edge.

## 2026-09-01 — two ratchet rows went blind on the same day, and nobody read the nightly

SEARCHED FIRST: design/compiler-log.md (the 2026-08-24 entry that fixed the
same `compile_allocs_unwatched` row for the same class of reason, and the
2026-08-31 entries), design/log/compiler-log-archive.md, design/*.md, and the
mutation corpus itself — 59 scripts in scripts/ratchet/mutations.

`scripts/ratchet -- prove` applies each gate's claimed defect and refuses a
gate that stays green. It runs nightly. It has been RED since 2026-08-30, with
two rows reported BLIND, and the two nights of failure were not read. What
follows is what they were.

**Row one: a mutation had been inserting itself into a comment.**
`a_string_the_builder_corrupted_in_place` matched `k_stat_append_fast++;`,
skipped one line and appended after it. kanso#1171 (2026-08-30) wrote a
three-line comment above the copy, so the skipped line became the comment's
opening and the injected statement landed inside `/* ... */`. It was not code.
The compiler built clean, kq's suite ran green, and the script's own
`grep -qF` found its text in the file and reported success.

Reproduced end to end: apply the mutation to a worktree at origin/main, build,
clone kq to /tmp/kq, run `KANSO=$K KQ_STORED=report sh spec.sh` — exit 0, "kq
specs: all green", and kq's own counters read `append_fast=242226`, so the
mutated path ran a quarter of a million times and corrupted nothing.

It substitutes the copy now:
`else memcpy(...)` becomes `else { memcpy(...); ...[a->len] = 0; }`. A
substitution cannot land in a comment, because the anchor is the code being
replaced. Watched red: the kq gate dies with `invalid utf-8, born in
text/utf8`, which is the sentence the mutation's own comment had been
promising and not delivering.

**Row two took two merges to kill and a rewrite to fix.**
`compile_allocs_unwatched` makes a front-end pass own the program's names
instead of borrowing them. kanso#1188 (2026-08-30) made an identifier a `Name`
rather than a `String`, so `out.insert(name.clone())` stopped type-checking and
the mutation became a compile error — the exact failure its own comment had
warned about once already, in different words.

**Repairing the type is not enough, and this is the part worth keeping.** With
`name.as_str().to_string()` the mutation builds and `compile_allocs` reads
25,394 either way, on binaries that differ. kanso#1157 (2026-08-30) gave the
walk an early return for any name that cannot be a getter's, and by its own
comment eleven thousand of lib/json's twelve thousand occurrences take it. An
owned insert behind that guard is reached almost never. A mutation can be
applied, compile, and still be inert.

So the row restores the shape the vein was built to catch rather than one line
of it: the guard goes and the names are owned. Measured on this container,
`kanso check lib/json` in a fixed box: **compile_allocs 25,394 -> 31,138**,
compile_alloc_bytes 3,950,766 -> 4,062,065.

**Both scripts now fail loudly rather than silently.** Each greps its anchor
before substituting and exits 1 with a sentence if it is gone; the second also
greps that the guard is GONE afterwards, so a mutation that applied and left
the code inert is an error rather than a green row. That is the durable half:
the two rots were both silent, and silence is what let a nightly failure sit
unread for two nights.

**Why the per-PR half did not catch either, which is sharper than it looks.**
That half does more than count rows: since the #1015 incident it also applies
every mutation to a worktree of HEAD and fails if one no longer matches the
source it patches — added, in its own comment's words, because the nightly
"said so the next morning — correctly, precisely, and to nobody". Both rots
slipped past it because both mutations still APPLIED. #1171's sed matched, and
put its statement in a comment. #1188's sed matched, and the build then failed.
Applying is not proving, and only proving costs a build.

So the gap is narrower and more specific than "the cheap half is cheap": what
is missing is a middle. A mutation names the source files its seds touch; a
pull request names the files it changes; the intersection is usually empty and
occasionally one to three rows. #1171 touched src/runtime.c and #1157 and #1188
touched src/lib.rs, so both would have been caught at merge time for one to
three extra builds on the pull requests that could break them, and none on the
rest. Both rows here were killed by merges dated 2026-08-30, which is to say a
single busy day put two of the repo's own gates to sleep.

## 2026-09-01 — the survivorship walk gained a counter, and minting it found a hole in the trend gate

**DONE.** `k_slots_survive` reads a node's whole immediate interior on every
ask, and nothing in the tree bounded how often it is asked. `evac_allocs` and
`evac_bytes` count what a carry evacuation COPIES; nothing counted what
deciding costs. On the branch that lifted the carry-tier prefix and was
declined for other reasons (kanso#1198), the 8,192-byte digest asked 33,024
times over 8,256 slots — **272,646,144 examinations finding zero heap slots**,
against 33,827 allocations that actually evacuated. Every counter beside that
one read the workload as nearly free.

`survive_slots` counts slots examined rather than calls made, because the calls
were linear and the slots were not; and slots rather than bytes, because it is
an algorithm-level step no platform can widen. Readings on main, where the
carry tier is not admitted: encodebench 129,873, pendbench 118,087, basket
16,002, widebench 16,000, and zero on the other five cost goldens — including
digestbench, which is the point: the pathology lives behind a prefix decision,
and the counter is what will show it the day that decision moves.

**What it costs, and a correction to what I first wrote.** The guard is
`k_stats_on > 0`, read inside the loop, and I measured it on basket and
pendbench, found two retired instructions per slot on both, and wrote that
down as the rule. CI measured all ten and two of them do not follow it:

| benchmark | slots | `work_*` delta | per slot |
|---|---:|---:|---:|
| basket | 16,002 | +32,004 | 2.00 |
| pendbench | 118,087 | +236,180 | 2.00 |
| encodebench | 129,873 | +5,104 | 0.04 |
| widebench | 16,000 | +3 | 0.0002 |
| deepbench | — | +1,456 | — |

The guard is loop-invariant, so a compiler may hoist it out; these are ten
separately compiled programs and it evidently did in two of them. The
supporting evidence is that `k_slots_survive` costs 31 instructions a slot in
basket and 12 in widebench with counters off — different code for the same
source. Reading the assembly to confirm the hoist is not done, so the
mechanism is an inference and the deltas are the measurement.

Accumulating once at the exit instead was written and measured on the two
benchmarks that pay: it is **not** cheaper — the same two per slot on both —
so the count stays where it is read.

As a fraction: `work_basket` +0.057%, `work_pendbench` +0.025%,
`work_deepbench` +0.0002%, `work_encodebench` +0.00006%, `work_widebench`
+0.000005%. `text` rises 741,202 -> 744,562, a flat +368 bytes of machine code
in every benchmark, which is the guard plus the extra argument the dump
marshals. `compile_instructions` FELL 41,496,028 -> 41,494,642, unattributed:
the front end does not run this code, and the only reachable connection is the
runtime.c text the compiler carries. Welfare 74.33 against a floor of 74.33.

**The hole it found.** The trend gate read the first reading as eight
worsenings and refused the branch as a pure regression. `or_zero` cannot tell a
counter that read nought from a counter the baseline does not carry at all, and
every golden in a vein gains the new row on the same commit — so a change whose
whole content is that the runtime now measures one more thing printed as a
tree-wide regression with nothing on the other side.

The gate already says this sentence one level up, for a benchmark that JOINS a
golden (`new_samples`, added when a tenth benchmark read as four simultaneous
worsenings). This is the transposed case: a row joining every sample rather
than a sample joining every row. `missing?` distinguishes absence from nought,
a minted counter is reported under MINTED and counts toward neither side of the
pure-regression rule, and the exemption lasts exactly one commit — the dumps
carry every counter on every run, zeros included, so a name absent from the
baseline was absent from the runtime. The reverse, present then gone, is a
deleted kernel and the hard golden diff already refuses that byte for byte.

`tests/a_minted_counter_is_not_a_regression.rs` stands up a scratch repository
whose committed goldens lack the row and whose working tree has it, runs the
gate against that commit, and requires a pass. Its second fixture raises a
counter the baseline DOES carry and requires the refusal, which is what stops
the rule widening into an escape hatch. Watched red both ways: with the mint
rule removed the first prints the eight worsenings; with the exemption widened
to every counter, both go red.

`survive_slots` is classified `lower` in the direction tables, so the next
change to move it has a direction to be judged against.

**OPEN.** The digest reads zero here because the carry tier is declined. Nobody
should read that as the walk being cheap — it is the counter being pointed at a
workload that currently does not enter it.

## 2026-09-01 — a branch proves the ratchet rows its own diff could have made blind

**DONE.** kanso#1199 repaired two rows that went blind on 2026-08-30 and asked
why the per-PR half had not caught either. The answer is sharper than "the
cheap half is cheap": that half applies every mutation to a worktree of HEAD
and fails if one no longer matches the source it patches, and **both mutations
still applied**. #1171's sed matched and put its statement in a comment;
#1188's sed matched and the build then failed. Applying is not proving, and
only proving costs a build.

So there is a middle, and it is cheap because the intersection is usually
empty. A mutation names the source files its seds patch; a branch names the
files it changes:

    kanso run scripts/ratchet -- touched origin/main

keeps the rows whose mutation script names a file the branch changed and proves
that handful. Naming is the test because a mutation patches what it writes
down. `touched origin/main list` names them and stops, for somebody deciding
whether to spend the runner.

**What it costs.** Measured on this container against the survivorship-counter
branch, which touches src/runtime.c — the worst realistic case, because that
file is patched by eight mutations: **thirteen rows, six minutes eight
seconds, every one red.** One of the thirteen is the kq row #1199 had just
repaired, so the mechanism selects exactly the row the incident was about. A
branch touching src/lib.rs selects the compile-allocs row that #1157 and #1188
made blind between them. A prose-only branch selects nothing and the step is
a second.

**What it does not cover, stated rather than left to be found.** A row can go
blind from a distance. The guard #1157 added is in the file its mutation
patches, which is why this catches it, but a change to what a GATE reads could
hollow a row while touching no line any sed matches. `cover` still runs on
every change and the nightly still proves the whole table; this is a third
thing between them, not a replacement for either.

`tests/a_branch_proves_the_rows_it_could_have_broken.rs` asserts the selection
rather than the proving, because the proving is a build per row and the
selection is the half that can go quietly wrong — a mutation whose paths stop
matching selects nothing, exits zero, and reads as a branch that broke no rows.
Three fixtures: a src/runtime.c branch selects the row #1171 killed and not the
python-free row; a src/lib.rs branch selects the row #1157 and #1188 killed; a
README-only branch selects nothing. Watched red both ways — with the naming
test never matching, the two positive fixtures fail; with it always matching,
the negative ones do.

**The first CI run of the step was red, for a reason the spec could not see.**
`git diff origin/main...HEAD` needs a merge base and `actions/checkout@v4`
takes a shallow clone, so the diff failed outright and the ratchet job went red
with nothing to do with a mutation. Fixed with `fetch-depth: 0` on that job,
and the refusal now carries git's own stderr — "would not run" sent a reader to
the ratchet where the answer was in the workflow. The lesson generalises past
this row: a spec that enters where a user enters still cannot see the shape of
the box CI enters from.

## 2026-09-01 — nine of the objective's counters stand on a rule, and nothing said which

**DONE.** `entering` gives a counter new to the model a baseline of
`now * standing`, where standing is the ratio whose satisfaction equals its
dimension's current mean. Landing day is therefore neutral, which is the whole
point: entering at parity instead — the rule before kanso#910 — makes a
measurement-only change spend the floor, and an objective that charges for
measuring is paying people not to measure.

**The rule is not neutral about anything after landing day, and that was not
written down.** Saturation is concave, so a counter granted a high standing has
little headroom left and one entering at parity has a great deal; how much a
later change to that counter is worth follows from where it entered. Measured
on the carry-tier arms of 2026-09-01: with the digest baselines at their
dimension's standing the trade scored **74.31 -> 73.75 and was declined**; the
same two arms with those baselines at parity score **70.14 -> 72.99, an
acceptance**. The entering rule decided that verdict.

**Nine of twenty-one.** The floor file's own history says which counters were
granted, because the commit that first wrote each baseline key is either an
ancestor of kanso#910 — which added `entering` on 2026-08-14 — or a descendant
of it:

| counter | first written | granted? |
|---|---|---|
| `wide_instructions` | 2026-08-14, kanso#887 | no, predates the rule |
| `deep_instructions` | 2026-08-15, kanso#912 | yes |
| `scan_arena_blocks`, `scan_peak_bytes` | 2026-08-17, kanso#945 | yes |
| `pending_instructions` | 2026-08-21, kanso#981 | yes |
| `compile_allocs`, `compile_instructions` | 2026-08-25, kanso#1041 | yes |
| `digest_arena_blocks`, `digest_instructions`, `digest_peak_bytes` | 2026-08-31, kanso#1198 | yes |

Everything from 2026-07-26 to 2026-08-14 was measured or hand-seeded. So nine
of the model's twenty-one counters have a reference no measurement produced,
and until this the floor file recorded them exactly like the twelve that do.

**The rule stays; the arbitrariness stops being invisible.** `granted` names
them in `bench/welfare_floor.json`, `--set` carries the list forward and adds
whatever this run had to grant, and the report prints a line naming them. The
score is unchanged — 74.33 against a floor of 74.33 — because nothing about the
computation moved. A reader comparing two counters' ratios is comparing unlike
things unless they know which, and until now nobody could.

`tests/a_granted_baseline_says_it_is_one.rs` pins both halves: the report names
all nine and does NOT name `decode_instructions`, which predates the rule by a
fortnight; and a run that has to grant a counter writes it into the floor
without losing the ones an earlier run granted. Watched red both ways — remove
the report line and the first fixture fails, drop the persistence and the
second does.

**OPEN, stated rather than buried.** Whether a granted baseline should be
replaced by real history once the counter has some. The argument for is that a
granted reference is a guess and a measured one is not; the argument against is
that re-basing a counter mid-life moves the objective without saying so, which
is the thing the ratchet exists to stop. Nothing here does it.

## 2026-09-01 — the runner pool is four CPUs, and the first fix for that was wrong

kq#85 established what moved four kq instruction rows between two runs: not a
toolchain. Both job logs printed rustc 1.98.0 (88d9e12ae), LLVM 22.1.8, image
ubuntu-24.04 20260823.283.1, glibc 2.39-0ubuntu8.8, valgrind 3.22.0-0ubuntu3,
gdb 15.1 — every version identical to the commit hash. The one field that
differed was `Azure Region`. Different silicon under the same image.

**The mechanism, measured twice.** glibc resolves memcpy, memcmp, strlen and
their neighbours by ifunc at load time, reading CPU features, so one libc runs
different code on different CPUs. The first measurement swapped the resolver's
choice on one host by tunable, on kq's `print_small` row:

| GLIBC_TUNABLES | Ir | memcpy chosen |
|---|---:|---|
| default | 76,742,430 | `__memcpy_avx_unaligned_erms` |
| `-AVX2_Usable` | 76,746,433 | `__memcpy_avx_unaligned_erms` |
| `-AVX_Fast_Unaligned_Load` | 76,488,416 | `__memcpy_sse2_unaligned_erms` |
| `-ERMS` | 76,262,756 | `__memcpy_avx_unaligned` |

0.63% from the dispatch alone, against a runner shift of 0.06% to 0.10%.

The second used a switch that actually differs between this container and a
runner, after CI printed the runner's block:

| GLIBC_TUNABLES | Ir | vs default |
|---|---:|---:|
| `rep_movsb_threshold=0x2000` (Intel, default) | 76,742,736 | — |
| `rep_movsb_threshold=0x840` (a runner's) | 77,523,061 | **+1.02%** |
| `non_temporal_threshold=0x1800000` (a runner's) | 76,744,279 | +0.00% |
| both together | 76,744,207 | +0.00% |

Byte-identical over two sittings each. One switch is worth ten times what the
vein saw. The pair nearly cancels because glibc derives
`rep_movsb_stop_threshold` from `non_temporal_threshold`, so no single line
predicts a row.

**The first fix was a pin, and CI killed it in two runs.** Record one host's
feature block; refuse anywhere it does not match, the way `measured_on.sh`
refuses a moved glibc. The first run refused and printed an AMD EPYC Zen 3
Milan (family 0x19, model 0x1). The second refused and printed an Intel Ice
Lake-SP (0x6/0x6a). The third, after the restructure below, named an AMD Genoa
(0x19/0x11). This container is a Cascade Lake (0x6/0x55). **Four CPUs in four
runs.** A check that refuses every run but one is red for a reason no pull
request causes, which is a gate nobody can act on, and it would have been
merged on the strength of a local verification that could not see the pool.

**And on that fourth CPU every kq row matched exactly.** That qualifies the
story rather than undoing it: kq#85's two runs really did differ by 0.06% to
0.10% with every version identical, and a Genoa really does count kq's four
rows byte for byte the same as whatever counted the golden. Both are what the
ifunc account predicts — most CPUs land on the same memcpy and the counts are
identical, and now and then one lands elsewhere and they are not. It also
means the pool's heterogeneity is survivable rather than fatal, which is the
difference between this vein reporting sometimes and being unusable.

**What the fix is instead.** `scripts/gates/dispatch.sh` never refuses. It
answers, and the two instruction gates ask only about a row that already moved:

- `name` prints this host's CPU family and model on every run, so the next
  divergence is one line of a job log rather than an afternoon of version
  archaeology — which is what kq#85 cost.
- `differs` answers 0 for the recorded silicon, 1 for other silicon with the
  differing lines named, and 2 when there is nothing to compare against.

A row landing on its recorded value is right whatever counted it, so the
question is worth asking only about a row that moved. On answer 1 the gate says
the run does not gate this vein and exits green — neither a pass nor a
regression, because the run cannot establish either. Calling that a regression
is exactly the mistake kq#85 spent a pull request undoing; this makes the
correction structural.

**`bench/dispatch.txt` is deliberately absent, in both repos.** It has to hold
a CPU on which the rows are known to verify, and no run has both named its
silicon and matched a golden — the naming only starts here. A guessed block
would be worse than none, because it would let a real regression on the true
recorded CPU read as other silicon. Answer 2 gates exactly as these veins
always have, so the absence costs nothing and the block goes in from the first
run that names its CPU and matches every row.

One more thing had to be built before any of it could work: a block may be
taken only from a run that BOTH names its CPU and matches every row, and a run
that matches never reaches `differs`, which was the only place a block
printed. A bootstrap with no first step. So while no block is recorded, CI
prints the whole thing beside the rows, and stops the moment the file exists.

`tests/the_silicon_a_row_was_counted_on.rs` pins all three answers plus three
properties: the pasteable block prints under `GITHUB_ACTIONS` and nowhere else
— `measured_on.sh`'s own header records a container printing a diff, somebody
pasting, and the container's numbers landing in a golden over the runner's —
an unknown verb answers 2 rather than a yes or a no nobody gave; and the
bootstrap print happens in CI while nothing is recorded and stops once there
is. Watched red five ways: a `differs` that always matches fails two fixtures,
one that treats an absent block as a match fails the third, printing the block
everywhere fails the fourth, and the bootstrap fails in both directions —
never printing, and never stopping. The spec also caught a live bug on its first run:
`grep -v '^#'` answers 1 on an all-comments file and `set -e` took the script
out before it could say which answer it meant.

**A second host was hiding in the spec.** The macos/arm job went red on the
first run that reached it: five of the seven fixtures called the x86 loader by
path and expected it to be there. Skipping them on aarch64 would have left that
host uncovered by the very check that says which host a number belongs to, so
each fixture states both arms instead. Where there is no loader the gate must
say the cpu is unnamed and answer 2 — never 0, which would let a moved row pass
as verified on silicon nobody read, and never 1, which would blame silicon
nobody read either. `dispatch.sh` takes its loader path from an environment
variable defaulting to the real one, which is what makes that arm reachable on
x86: a fixture that can only run on aarch64 is a fixture nobody watches fail.

**OPEN, and it is the real one.** These rows claim to be exact and the pool is
not. Three CPUs seen in one day means an instruction golden gates properly only
on the fraction of runs that land on its recorded silicon, and nothing here
measures that fraction. The honest options are a golden per CPU, or accepting
that the vein reports more often than it gates. Nothing is decided; what is
built refuses to lie about which case a given run is in.

## 2026-09-01 (later) — the silicon note was an excuse, and would have blinded the ratchet

The entry above shipped `scripts/gates/dispatch.sh` and wired it into both
instruction gates. On answer 1 — a row moved, and this is not the silicon the
rows were counted on — the gate printed a warning and **exited green**. That is
wrong, and the argument is arithmetic rather than taste.

**Most real regressions would have been waved through.** Four CPUs were seen in
four runs that day. A block records one of them, so roughly three runs in four
land somewhere else. On those runs any moved row — a genuine regression
included — got the warning and a green tick.

**And the ratchet's rows would have gone blind on the same runs.** Two of its
mutations exist to redden exactly these gates: `a_counter_worsens_for_nothing`
and the decoder's instruction row. Applied on a run that landed off the
recorded cpu, the gate would have exited 0 and the mutation would have proved
nothing. A row that proves nothing is a BLIND row, which is the single failure
the ratchet was built to catch — kanso#1199 repaired two of them a few hours
earlier. Shipping a mechanism that manufactures them is worse than shipping no
mechanism.

**What the answer is for.** The dispatch diff is a named cause printed beside a
failure, never instead of one. Both gates now fail on a moved row whatever
counted it, and when the silicon differs they say so and name the lines, so a
reader knows in one screen whether to re-run for the recorded cpu or start
reading the diff. Deciding that silicon accounts for a move is a person's job
in a pull request, with a re-run on the recorded cpu as the evidence — it was
never a thing a shell script should conclude on its own.

This also settles, in the safe direction, the OPEN the entry above recorded.
The vein does not report-instead-of-gate on a fraction of runs; it gates on all
of them and explains itself on the fraction where the explanation is available.
What is still unmeasured is how often the pool's CPUs actually move these rows
— kq#85 saw 0.06% to 0.10% on one pair, and an AMD Genoa matched kq's golden
exactly on another. If that turns out to be frequent, the answer is a golden
per cpu, not a gate that shrugs.

kq carries the same wiring and owes the same correction.

## 2026-09-01 (later still) — forty-seven instructions to store one byte

`k_b_append_mut` is 2,000,259,200 instructions of encodebench, 23.82% of the
run and the largest single symbol anywhere in the suite. It is called
42,318,000 times, which is 47.3 instructions a call. The callgrind file has
said so for weeks; reading it needed an id-to-name map, because callgrind names
a function only on its first `cfn=` line and a grep for the name undercounts
by two orders of magnitude — 461,200 against the true 42,318,000.

Disassembled, the forty-seven are honest: the fast path really does execute
that many instructions to put a comma in a buffer that already has room. Two
causes, and both are the common path paying for a rare one.

**The byte went to memory and came back.** The general path reaches its store
through `src`, a `const unsigned char*` that is a phi of three predecessors —
a string's data, a byte string's data, or the address of a one-byte local. So
a byte a caller passed in a register was stored to a stack slot and reloaded
four instructions later, and the frame that slot needs was built by every
append of every shape. Straight-lining the byte case ahead of the other two
keeps it in the register. In the same block, `a->len` was read three times
where one would do: a store through `unsigned char*` aliases every field of
the header it is stored into, so the compiler had to reload the length after
writing the byte. Read once into a local, it does not.

**And the frame served an error path.** `k_die` calls `exit`, and nothing said
so. Unmarked, clang inlined its fprintf and its exit into every runtime entry
that validates a tag — so the entry pushed the callee-saved registers that
error path needs, and built a frame, before it could test anything. Marked
`noreturn` and `noinline`, along with its eight siblings, it costs a call at
the point of death and nothing anywhere else.

| row | before | after | |
|---|---:|---:|---:|
| jsonbench | 2,838,415,815 | 2,781,834,881 | -1.99% |
| encodebench | 8,396,592,982 | 7,870,153,008 | **-6.27%** |
| oneshot | 43,094,978 | 41,401,995 | -3.93% |
| basket | 56,490,028 | 56,459,146 | -0.05% |
| widebench | 63,997,234 | 64,077,244 | **+0.13%** |
| deepbench | 726,488,376 | 717,299,279 | -1.27% |
| escapebench | 253,819,082 | 249,019,060 | -1.89% |
| pendbench | 937,802,653 | 930,587,850 | -0.77% |
| indexbench | 5,243,104 | 5,243,096 | -0.0002% |
| digestbench | 152,573,619 | 143,472,199 | **-5.96%** |

(CI's rows. The container this was developed in runs glibc 2.39-0ubuntu8.7
against the runner's 2.39-0ubuntu8.8, so `measured_on` refuses a local
regeneration and the numbers above come out of the instructions job.)

The two changes were measured apart. `noreturn` alone is digestbench -5.97%,
escapebench -1.89%, deepbench -1.27%, encodebench -1.27%, pendbench -0.77%,
widebench -0.48%, jsonbench -0.40% — it reaches everything, because every
benchmark runs runtime entries that validate a tag. The append split is the
rest of encodebench's fall and most of jsonbench's.

**work_widebench rises, and the objective was asked rather than told.** The
split
puts the string and byte-string cases behind a call into `k_b_append_wide`, and
widebench appends strings, so every one of its appends pays that call: 384,000
instructions. The alternative was built and measured — leave the byte case
inline and outline nothing — and it costs widebench nothing, but gives back
142M instructions on encodebench and 20M on jsonbench. It reads 74.47 where the
split reads 74.50. The model prefers the split by 0.03, so the split shipped
and the rise stands with its cause written down. This is the trade the weights
exist to license, and the counterfactual is here so that a later reader can
argue with the weights rather than with the measurement.

Every allocation counter is byte-identical: all nine counter gates pass
unchanged. That is the point of a separate instruction vein — a decode that
allocates identically and executes six per cent less work moves nothing else in
the tree. Every binary also falls about 2,000 bytes, 2.4%, which is the
inlined error text and call sequence leaving dozens of sites.

**compile_instructions rises 1,630 to 41,496,272, and it is layout for the
third time.** `kanso check lib/json` runs none of the runtime, so nothing the
front end does changed. What changed is `include_str!("runtime.c")`, a static in the binary
that grew 2,962 bytes and shifted what follows it. Measured rather than
assumed, because the same claim was made twice before on the strength of
elimination: build this branch's front end against main's runtime.c and
against this branch's, on one host, and read 41,922,834 and 41,925,168 — the
same rise with no Rust changed at all. compile_allocs, compile_peak_bytes,
rounds and visits hold.

Welfare 74.33 to 74.50, floor set. kq links this runtime and owes a pin bump;
its instructions vein will move and none of its allocation counters will.

## 2026-09-01 (later still, second) — naming a counter licensed it, and the listing only printed

The ratchet went red on the branch above, with two rows BLIND: `a counter
worsens for nothing` and `a runtime counter worsens for nothing`. Both are
trend-gate mutations, both had been proving something, and both stopped
because of the shape of that branch rather than anything wrong with them.

**A counter's name was a blanket permit.** The gate priced a worsening when
the branch's compiler-log delta mentioned the counter anywhere. The branch
above raised `compile_instructions` by 1,630 for a layout reason and wrote a
paragraph explaining it — and that paragraph then licensed the mutation to set
the same counter to `compile_instructions=999999999`. The gate printed `every
changed counter is priced` and exited 0. (Written the way the mutation writes
it, ungrouped: the rule below reads comma-grouped figures, and an entry that
quotes a sentinel in the gate's own spelling prices it.) Any branch that legitimately moves a counter and says so
disarms the gate for that counter, which is every branch that touches a
golden.

**And the listing was advisory.** With the counter unnamed the gate printed
UNPRICED, listed the row, and exited 0 anyway. So the runtime mutation — set
`jsonbench 9999999999` — was listed and still green. The pure-regression
rule beside it could not catch that either: it refuses a branch where
something worsens and nothing improves, and the branch above improves nine
rows and ratchets the floor, so the licence was already bought.

**What it takes now.** A worsening is priced when the log delta names the
counter AND quotes the value it landed on. `compile_instructions` names
41,496,272 above; the mutation's figure appears nowhere in the grouped form
the gate reads, so it is unpriced. And unpriced exits 1 rather than printing. No band, no tolerance:
the log already states the figure a move landed on, and this is the gate
reading what the log is for.

Both rows are red again, and the four other trend-gate mutations still are:
`a worsening hidden behind a joining sample` and `a worsening paid for by an
unclassified counter` were re-run against the new rule and both exit 1.

The cost is that a branch worsening a counter must now write the number, not
just the name. That is what the log's own rule already asks for — pin the
number, never a band — so the gate is asking for the record it was always
supposed to be reading.

## 2026-09-01 (later still, third) — sixty-four depths walked on every beat pop

`k_beat_pop` is 2,205,202 calls in encodebench at 282 instructions each, and
216 of the 282 are one loop. `k_ten_release` frees the tenure blocks at a
depth, then recomputes `k_ten_any` — a summary over all sixty-four depths of
whether ANY holds a block — by walking all sixty-four. It did that on every
pop, including the pops where this depth held nothing and the summary
therefore could not have changed.

An early return when the depth's list is already empty takes `k_beat_pop` to
66 instructions a call and removes 476,323,632 from encodebench, which is that
row's entire fall to the instruction. The argument is arithmetic: `k_ten_any`
is a disjunction over all depths, nothing at any depth changes on the early
path, so the summary that was correct on entry is correct on exit. And
`k_ten_bytes[d]` cannot be non-zero with `k_ten_blocks[d]` NULL, because the
only `+=` follows a push at that depth.

encodebench falls to 7,393,829,376 (-6.05%), oneshot to 40,210,971 (-2.88%),
deepbench to 705,258,631 (-1.68%), escapebench to 248,370,844 (-0.26%);
jsonbench, basket, pendbench and digestbench each fall by less than a
thousandth. Encodebench's fall is 476,323,632 on the runner and 476,323,632 in
this container, on two different cpus — the saving is the walk, so it is
host-invariant even where the totals are not.

**work_widebench reads 64,077,249 and work_indexbench 5,243,101, each five
instructions ABOVE the row it replaces, and the cause is silicon rather than
this change.** The rows on main were counted on the AMD Genoa that ran the
previous pull request; these were counted on an Intel the pool had not shown
before. Measured in this container, where both arms sit on one cpu, the change
takes 408 instructions off each of them. Five instructions on sixty-four
million is what a cpu change looks like at this scale, and the gate is right to
make that sentence exist rather than let two rises pass as noise — which is the
discipline the entry above shipped, applied to its own author.

**The same early-out in `k_chunkreg_migrate` was measured and dropped.** It is
a wash — deepbench -225,082, widebench +40, basket +21 — and the change is
smaller without it.

**And nothing in the tree could see the inverse.** Break `k_ten_release` on
purpose so it never frees and never recomputes, and: all nine allocation
counter gates stay green, because `k_ten_alloc` mallocs without touching
`k_stat_allocs`, `k_stat_bytes_malloc` or `k_stat_held_live`; the whole test
suite passes but for the two `docs/kanso.wasm` staleness guards, which fail on
the correct build too; and the instruction vein reads the leak as a further
WIN, 7,378,392,563 against the correct fix's 7,393,828,977, because skipping
the frees is cheaper than doing them. A change that leaked every tenure block
would have landed green and looked like an improvement.

So the fix ships with the counter that catches it. `ten_blocks` counts storage
claimed and `ten_frees` counts it given back, in all nine cost goldens and all
fifty-two mem fixtures, every one pinned exactly. The coverage is thin —
widebench and one mem fixture tenure a single block each and everything else
tenures none — and sufficient, because the broken version reads `ten_frees=0`
against a pinned 1. Both veins move purely additively: every line that was
there is byte-identical, which is the same statement the nine counter gates
make about this change. `ten_blocks` joins the lower table beside
the other allocation counters and `ten_frees` the higher table beside
`bytes_freed`, because with blocks held constant a fall in frees is a leak.

`text` rises 1,072 to 727,778 — the two counters and the early return, about
110 bytes a binary. `compile_instructions` rises 981 to 41,497,253, which is
`include_str!("runtime.c")` growing again and shifting what follows it in the
front end's binary, the same layout effect the entry above measured directly.
Those two are the whole cost of the change on any vein.

The run that produced these rows named a cpu nobody had seen: family 0x6 model
0xcf, an Intel that is neither the Cascade Lake this container is nor the four
the entry above counted. The pool holds at least five.

Welfare 74.50 to 74.59, floor set.

## 2026-09-01 (last) — the decode costs 98.3 instructions an input byte

jsonbench decodes a 188,698-byte document 150 times for 2,781,834,449
instructions, which is 18,545,563 a decode and 98.3 an input byte. Nothing in
the tree stated that figure; the page states it now. It is the number to quote
when somebody asks what a kanso decode costs, because it is independent of how
big the document is and of how many times the benchmark runs it.

**Where it sits, re-measured after the two runtime changes above.** Every
function attributed to where it came from: 1,729,183,050 instructions in
emitted kanso, 1,001,841,947 in `runtime.c`, 50,803,205 in libc — 62.2%, 36.0%
and 1.8%. The same measurement on 2026-08-31 read 1,728,709,950 emitted against
1,078,786,257 in the runtime. So the emitted half moved by 473,100 and the
runtime half fell by 76,944,310, which is what two days of runtime work looks
like from outside: `k_b_append_mut` went from 7.05% of the decode to 3.75%, and
the largest runtime entries are now `k_b_put_mut` at 5.00%, `k_utf8_bad` at
4.22% and `k_b_append_mut` at 3.75%.

`value_for` is still the largest single symbol at 23.30%, and still a merged
one — clang inlined `parse_string`, `parse_array`, `parse_object`,
`parse_number` and `skip_ws` into it and none of the five appears under its own
name. The 2026-08-31 entry said the runtime had become the smaller half of a
decode; it is smaller again, and what is left of the larger half is the
backend's output rather than anything the runtime can be asked to do better.

**Two things measured in that profile, one worth taking and one not.**
`k_utf8_bad` costs 117,522,450 instructions over 1,571,250 calls, and
10,975,500 bytes pass through it, so the mean run is SEVEN bytes — the comment
in the function claiming forty-one is wrong, and it is wrong in the direction
that matters. At seven bytes the eight-byte loop usually never runs: its head
executes 159,450 times against 1,571,250 calls, and the byte-at-a-time walk
underneath answers for nearly every token. That walk is 30,347,250
instructions, 25.8% of the function and 1.09% of the whole decode.

The second is declined by the same profile. `k_stat_utf8_bytes += len` is
ungated where every other counter tests `k_stats_on` first, and gating it
would be a regression: it compiles to one `add` to memory, executed 1,571,250
times, where the gate is a compare and a branch. Gating costs 1,571,250
instructions and saves none. The rule that every counter is gated is a rule
about counters expensive enough to gate.

## 2026-09-01 (last) — seven bytes, and a walk that answered for all of them

The comment over `k_utf8_bad`'s ascii prologue said the average token was
forty-one bytes. The counters say seven: jsonbench calls it 1,571,250 times
for 10,975,500 bytes. That number decides the shape of the function. At
forty-one the eight-byte loop does the work and the byte-at-a-time walk below
it is a remainder; at seven the loop's head executes 159,450 times against
1,571,250 calls and the walk answers for nearly every token in the document.
Callgrind at instruction granularity puts the walk at 30,347,250 instructions
— 25.8% of the function, 1.09% of the whole decode.

`k_all_ascii` answers with loads that overlap instead. Eight bytes or more:
read whole words while eight remain, then read the LAST eight, repeating bytes
the loop already saw rather than walking whatever is left and without any
arithmetic to work out how many that is. Under eight there is no word to read,
so four and two do the same a step down, and one byte is one compare. Nothing
reads outside `data[0..len)`, which is what makes the overlap free.

Measured on this box, before and after, same binary set:

```
jsonbench    2,781,834,036 -> 2,747,404,386   -34,429,650  -1.238%
oneshot         40,210,572 ->     39,981,042      -229,530  -0.571%
widebench       64,076,836 ->     63,873,599      -203,237  -0.317%
basket          56,457,221 ->     56,448,794        -8,427  -0.015%
encodebench  7,393,828,977 ->  7,393,599,846      -229,131  -0.003%
deepbench, escapebench, pendbench, indexbench, digestbench   identical
```

Five fall, five do not move, none rises. The whole of jsonbench's fall is
`k_utf8_bad` itself: 117,522,450 down to 83,092,800. Re-splitting the decode
by origin, the emitted half is byte-identical at 1,729,183,050 and the runtime
half goes 1,001,841,947 to 967,380,120, which is what a change confined to
`runtime.c` should look like from outside.

**A mutation the harness could not see.** The differential extracts the
validator's text from `src/runtime.c` rather than copying it, so it now
extracts `k_all_ascii` too — the piece where a width bug would show. Breaking
the four-byte overlap turns it red at once. Breaking the EIGHT-byte overlap
did not: 45,189,025 cases, zero mismatches, with a validator that never looked
past byte eight of a run. The sampled band ran from four to eight bytes, which
is exactly the region where the word loop does everything and the tail has
nothing to answer for, and `unsigned char buf[8]` was what held it there. The
band now runs to twenty-four, which also reaches past the sixteen-byte vector
boundary, and the same mutation fails on 1,097,135 cases. It is a ratchet row
of its own now, so the band cannot quietly shrink back.

**CI's rows, and a delta that reads across silicon.** The runner counts
jsonbench 2,781,834,449 -> 2,747,404,799 and the four other movers by
-229,530, -203,237, -8,427 and -229,131. Every one of those five deltas is
IDENTICAL to the container's, on a Genoa against a Cascade Lake. The absolute
rows differ by a few hundred as they always do; the differences do not differ
at all. A change with no dispatch-sensitive path in it can have its delta read
across silicon even where its rows cannot, and this is the cleanest instance
the vein has produced. Welfare 74.59 to 74.61, floor set.

**Two veins that could have moved and did not.** `.text` is byte-identical for
all nine benchmarks — a walk removed and a load ladder added come to the same
size, so the machine-code golden, which carried 17% of the last regression,
says nothing here and is right to. `compile_instructions` fell 161 (0.0004%),
banked as layout: `kanso check lib/json` never emits, never links and never
reads runtime.c's contents, but the compiler carries it as an `include_str!`,
so a longer one moves the binary underneath the measured path. Front-end
allocations and peak are identical.

**The other residual is declined, by the same profile.**
`k_stat_utf8_bytes += len` is ungated where every other counter tests
`k_stats_on` first. Gating it would be a regression: it compiles to one `add`
to memory, and the gate is a compare and a branch. The rule that every counter
is gated is a rule about counters expensive enough to gate.

## 2026-09-01 (last) — the growth path ran more often than the fast one

`k_b_put_mut` was the largest runtime entry left in the decode: 139,045,650
instructions over 1,254,150 calls, 110.9 each, 5.06% of jsonbench. The counters
say what the profile only implies. put_mut_grow read 669,750 against
put_mut_fast's 584,400 — more than half of every map insert reallocated the
pairs buffer, memcpy'd it and donated the old one. Encode was the same shape at
4,465 against 3,896.

The arithmetic was in the growth arm. It started at `cap = 4` KValues, which is
two pairs, and doubled from there, so a map of k keys grew at k = 1, 3, 5, 9,
17. A JSON object with two keys therefore paid a growth for its first insert
and a three-key object paid two, and the objects in this corpus are small
enough that the doubling never got going. Four is room for two pairs, and it is
the wrong four: the LIST path has never used it.

`k_b_push_into_proven` sizes a fresh list's buffer with `cap = 4`, doubles
while the length needs it, and then doubles once more unconditionally, so a
list holding its first element gets eight KValues. `k_b_put_mut` did the first
two steps and not the third, so a map holding its first pair got four. The two
containers grow the same way and started a factor of two apart, and this change
is the map taking the list's second doubling. Eight is not a constant tuned to
this corpus; it is the number the sibling path already used, and the ladder
below is the check rather than the choice.

**The ladder, measured rather than reasoned about.**

```
cap   jsonbench       vs 4       put_mut_grow  arena_blocks  peak_bytes
 4    2,747,404,386      —          669,750         2          2,097,152
 8    2,718,705,486   -1.044%       419,850         2          2,097,152
16    2,708,688,349   -1.409%       334,350         3          3,145,728
32    2,710,734,799   -1.335%       334,350         3          3,145,728
```

Sixteen is the instruction minimum and the objective refuses it. Welfare reads
74.14 there against a floor of 74.6054: the third arena block and the extra
megabyte of peak cost 0.47 points, where the 0.37% of instructions sixteen buys
over eight are worth a fraction of one. Eight reads 74.6146, a rise of 0.0092,
and thirty-two is slower than sixteen for the same growth count because the
memcpys it does are bigger.

This is the trade the index was built to settle, and it settled it against the
number a per-counter reading would have picked. The instruction vein alone says
sixteen; the sum says eight.

**What eight costs.** jsonbench's alloc_bytes goes 262,667,408 to 268,048,208,
two per cent more bytes requested, and its allocs FALL 5,334,608 to 5,334,308.
The extra bytes are transient — peak does not move on any benchmark, and
neither does any arena block count. On the small fixtures the bytes fall too:
`map_put.mem` reads 9,136 where it read 9,216, because one fewer allocation
outweighs one bigger buffer.

Four rows fall and six do not move: jsonbench -1.044%, oneshot -0.481%, basket
-0.040%, encodebench -0.003%. `.text` is byte-identical for all nine binaries
again, which is what a constant change should look like. (Three more rows move
once the tenure fix below joins the branch; the combined figures are at the end
of that entry.)

**The counters the shelf keeps.** A growth donates the outgrown buffer to the
shelf and a later allocation takes it back, so halving the growths halves both
halves of that trade. `buf_reuse` reads 85,350 on jsonbench where it read
334,950, `encode_buf_reuse` 1,780, `oneshot_buf_reuse` 569 and
`basket_buf_reuse` 98 — every one of them a buffer nobody had to hand back
because nobody outgrew one. `sh_buf` is the bytes the shelf saw pass through
and it rises with the buffers being bigger: 143,306,400 on jsonbench,
`encode_sh_buf` 73,270,544, `oneshot_sh_buf` 1,133,328. The same two bytes per
pair show up as `encode_alloc_bytes` 853,137,424 and `oneshot_alloc_bytes`
4,490,268. Basket goes the other way on both, because its maps are large enough
that eight is one doubling it no longer needs.

## 2026-09-01 (last, later) — the tenure block a survivor still pointed at

kanso#1209 could not go green. The cost-goldens job died with `the program ran
out of stack: recursion went deeper than the stack holds`, and the program that
died was the trend gate. The message was wrong twice over: the stack was eight
frames deep, and the fall was a SIGSEGV the parent translates to that sentence
because native cannot see its own recursion.

Reproduction, on origin/main at 21d5c933 with nothing else changed: move one
digit in `bench/cost_golden.txt` and run `scripts/trend_gate`. Native
segfaults; `--interp` prints the listing and exits 0. #1209 was the first
change in a while to move a counter in that file, which is why it surfaced
there and not earlier.

**Where it died.** gdb puts the fault in `k_copy_size` reading `s->data` for a
KStr at 0x7ffff7e45b10, an address in the hole between two mappings — a block
malloc had served with mmap and freed back to the kernel. A `free`-recording
wrapper named the site: `k_ten_release`, the tenure allocator's block release.
The path from the beat's result to the dead byte was a list in the arena, a
record in the arena, and a string in the freed block.

**Why the walk could not see it.** `k_survives_x` answers yes for a pointer
into a tenure block, and that answer is what lets the copy prune: a survivor
whose immediate interior survives is shared rather than copied, and the walk
stops there. For the arena the prune is sound, because arena allocation is
monotonic — a survivor can only point at storage older than itself, which is
therefore also a survivor. Tenure storage is younger than the survivors that
come to hold it, so an arena record can carry a tenured string with no arena
pointer anywhere on the path to say so. `k_beat_pop`'s copy-out walks the
result with a null mark, which does turn the tenure answer off, and prunes at
the arena list one level above the record. Then it freed the block.

**The fix.** A heap result keeps the region alive — `k_beat_pop` does not
rewind for one — so the blocks are handed to the depth outside instead of
freed, and released on the branch that does rewind, which is where everything
the beat allocated goes back. `k_ten_bytes` travels with the blocks, so
`K_TEN_CAP` still bounds what one depth may hold.

One case is narrower rather than closed, and the comment in runtime.c says so:
a node below that mark, repaired during the beat to hold a tenured pointer,
outlives the rewind that frees the block. `k_repaired_settle` is where it would
close — it exists to move repaired slots into the arena and leaves the tenured
ones where they are, because `k_survives_x` answers yes for those too. Making
that pass tenure-blind was built and did NOT change the gate's crash, so the
route this bug took is the one above; the residual is recorded rather than
patched on a guess.

**What it costs.** Across all nine benchmarks exactly one counter moves:
`scan_ten_frees` 1 -> 0. One 256 KiB block on scanbench is now handed up rather
than freed at the pop. No allocation counter, no arena block, no peak, and
widebench's `ten_frees` still reads 1 because its beat pops with a non-heap
result. Every binary's `.text` rises 144 bytes, deepbench 160, and the machine
code vein `text` lands on 729,106 across the nine.

**The rows, measured by CI on the recorded Genoa.** Seven fall and three hold:

```
jsonbench     2,747,404,799 -> 2,718,705,899   -1.045%
encodebench   7,393,600,245 -> 7,393,366,858   -0.003%
oneshot          39,981,441 ->    39,789,223   -0.481%
basket           56,449,207 ->    56,426,594   -0.040%
deepbench       705,258,631 ->   705,257,898   -733
pendbench       930,587,202 ->   930,587,200   -2
indexbench        5,243,101 ->     5,242,731   -370
```

widebench, escapebench and digestbench are byte-identical. The first four are
mostly the capacity change; the last three are this fix alone — a branch added
to the beat pop and a free taken off it, which is what a change that removes
work from a hot exit looks like when the exit runs a few thousand times.
`compile_instructions` falls 1,320 for the reason the vein has recorded four
times now: main.rs holds runtime.c as an `include_str!` and hashes it for the
build cache key, so the C source's length moves the front end's arithmetic
without moving its work. `compile_allocs` and `compile_peak_bytes` are
identical.

Welfare 74.6146 to 74.6196, and the floor is set.

**The fixture.** `tests/a_tenure_block_a_survivor_points_at.rs`. It took three
things at once and no fewer: an inner beat that builds a batch, an outer beat
that accumulates the batches so the batch nodes live a lap and are promoted,
and a SECOND pass over the accumulated list so a later evacuation walks the
promoted nodes after their block has gone. Every earlier attempt had two of the
three and read green — the dangling pointer was there, a detector saw it, and
nothing dereferenced it. Watched red on origin/main's runtime.c rebuilt in
place: the assertion fails in 0.69 seconds. Green here in 21, with the oracle
agreeing to the byte.

## 2026-09-02 — the dispatcher moves to the call site

**DONE** (kanso#PR). A call whose head is a value — a lambda, a parameter, a
bound function — compiles to `call @k_call{n}`, and the runtime dispatcher it
lands in is 26 instructions for arity two. Ten of them ask about the callable:
is it a failure, is it a closure, does its arity match. A fold passes the same
callable through its self-call unchanged, so once TailCallElim turns the
`musttail` recursion into a loop those ten are loop-invariant and LICM would
hoist them out. LICM cannot hoist across a call, and the dispatcher is a call.

Deleting every one of those ten checks from `k_call2` bounded the prize at
437,205 instructions on oneshot, 1.096%. That number corrected an earlier
estimate of 0.51%, which had counted only the fold's 29,147 applications;
oneshot makes about 43,720 `k_call2` calls in all.

**Two levers were tried before the one that shipped.** LTO is real here —
`cached_runtime_object` compiles runtime.c to genuine LLVM bitcode under
`-O3 -flto`, and `k_call2` is internalized in the linked module — but the
inliner declines it on cost. `__attribute__((always_inline))` on the C
definition is ignored without a warning, and `.text` came back byte-identical
at 100,530: the repo's working pattern is `static inline
__attribute__((always_inline))`, and `k_call2` cannot be static because the
emitted `.ll` calls the symbol by name.

**What shipped.** `call_twin` in codegen.rs writes an `internal alwaysinline`
twin per arity the program uses, in the module the optimizer is already in —
the same shape as `k_force_fast` and `k_b_append_byte`, which exist in
DECLARES for the same reason. The twin covers a closure of the arity written
with no failure in an argument, and everything else calls `k_call{n}`, which
re-asks the lot and answers as before. That is why the twin may test in a
different order from the runtime: the arm only fires where all the orders
agree.

The twins are generated against the body rather than carried in DECLARES,
because an unused `internal` definition is free after optimization and not
free in `bench/emitted_golden*.txt`.

**The rows, measured by CI.** Six fall and four hold.

```
digestbench     143,471,767 ->   137,057,629   -4.471%
pendbench       930,587,200 ->   912,184,212   -1.978%
encodebench   7,393,366,858 -> 7,257,716,458   -1.835%
oneshot          39,789,223 ->    39,450,097   -0.852%
deepbench       705,257,898 ->   701,525,898   -0.529%
basket           56,426,594 ->    56,139,380   -0.509%
```

This sitting is on family 0x6 model 0xad, not the recorded Genoa. The rows
were also measured locally, on a third chip and a different glibc, and every
delta agreed to within a per cent of itself — a per-row constant offset of
about 400 separates the two hosts, which is the process startup the empty
environment does not remove. The moves are two to four orders of magnitude
larger than that, so the silicon is not what is being read here.

**Welfare 74.6196 to 74.6928, and the floor is set.**

jsonbench, widebench, escapebench and indexbench are byte-identical.
jsonbench is the interesting one: it writes fifteen sites for the twins and
reaches none of them from its entry, so the linker drops both twins and its
`.text` does not move either. The decoder does not dispatch on a value.

**What it costs.** Six binaries gain 192 to 672 bytes of `.text`, 0.28% to
0.60%; the three that never link a live site gain nothing. Every emitted
program gains two defines, four calls, six branches and fifty-three lines.
No allocation counter moves, on any of the nine — all nine counter gates are
byte-identical. `compile_golden.txt` does not move; only the module sample in
`compile_golden_modules.txt` does, by the same two defines.

**The fixture.** `tests/golden/micro/a_callable_that_is_a_value.kso`, on all
three engines and again under a release build. Nineteen lines of output over
arity one and two: a lambda, a fnref, a capturing closure, a lambda that
ignores its argument, a failing argument in each position, both failing, and
a failing callable.

The failing arguments are computed inside a body, not passed by a caller. The
first version passed them in, and a declared group refuses a failing argument
before its body runs — so the dispatch was never reached and the whole family
read green under every mutation.

Four mutations, four verdicts. Dropping the argument failure test: red,
because a lambda that ignores its argument answers 42 where the dispatcher
answered the failure. Reading the env pointer or the fn pointer from the wrong
slot: red, and loudly — the capturing closures jump into nothing and the
program dies on stack exhaustion. Reading arity from the capture count: output
identical, caught instead by `bench/instructions_golden.txt`, where oneshot
rises to 40,090,932, above the baseline it started from.

**The disassembly says it worked the way the argument said it would.**
`d_list/fold_go_3` in oneshot now tests the callable's tag ONCE, at `3cc4`,
before the loop header at `3cd0` — and LLVM went further than hoisting: it
unswitched the loop on that test and emitted two copies of the body, one for
the closure case and one for everything else. The hot copy has no dispatch in
it at all. That is where the few hundred bytes of `.text` went, and it is why
`k_call2` has left oneshot's profile entirely; the top twenty is now
`d_json/value_for_3` at 10.95%, `k_b_append_mut` at 9.40% and the fold itself
at 3.31%.

`w_klam17` sits at 3.10% and was checked as the next candidate: the emitter
writes a plain-C wrapper beside every lifted lambda so `k_call{n}` has
something to call at the C convention, and the wrapper looked like a pure
`ccc`-to-`tailcc` hop worth deleting. It is not one. LLVM has inlined the
lambda's body into the wrapper, so the symbol IS the body — a JSON string
escape switch — and there is no forwarding frame to remove.

**A divergence the fixture found on its way in.** The wasm backend disagreed
with the other two engines about a value-headed call, in two ways, and neither
had anything to do with this change — nothing had ever asked.
`call_closure` in wasm_rt.rs read its arguments before its callable and handed
back the first failing argument. `k_call2` and the interpreter both name a
failing CALLABLE first, and both MERGE two failing arguments into one err
carrying both reasons. So `bad (boom "a") (boom "b")` answered `a` on the page
and `["a" "b"]` on the two engines that agree, and a call with a failure in
both the head and an argument named the argument on the page and the head
everywhere else.

Both are the same reading `rt_mkrec` already applies to a record's fields —
its comment says returning the first failure there "was a divergence from the
oracle that no fixture built" — and the fix is the same two lines: test the
callable first, then reduce the failing arguments with
`eval::accumulate_failures`. A single failure keeps its own handle rather than
a copy of its value. Watched red before it was watched green: the corpus test
prints both output strings side by side and names the sample.

**Pricing the thirteen counters that worsened**, so the trend gate has its
sentence and its number for each. Twelve of them count the same twenty-six
lines of IR the generator writes per arity, times the arities a program uses.
`emitted_defines` lands on 156 and `emitted_lines` on 11,580 for the decoder;
`emitted_branches` on 1,174 and `emitted_calls` on 1,789 beside them.
Across the other ten programs `emitted_other_defines` lands on 1,339,
`emitted_other_calls` on 14,342, `emitted_other_branches` on 8,389 and
`emitted_other_lines` on 83,175 — sixteen defines, thirty-two calls, forty-
eight branches and 424 lines for ten programs at two arities each. The module
sample in the compile golden lands on `module_defines` 78, `module_calls` 753,
`module_branches` 375 and `module_lines` 4,480, the same two twins once.

Those twelve count DEFINITIONS rather than code. The bodies are `internal` and
`alwaysinline`, so after optimization they exist only at their call sites; the
vein that says what actually got built is `text`, which lands on 730,978
across the nine — 1,872 bytes for six binaries and nothing for the three whose
sites the linker drops. Bought with 24.9 million instructions off the work
vein.

`compile_instructions` lands on 41,500,519, a rise of 4,747 on CI and a fall
of 20,230 on this container for the same diff. `kanso check lib/json` emits
nothing, so the code this change writes never runs during the measurement:
both numbers are the front end's own layout moving under a larger codegen.rs,
and the two hosts disagreeing on the sign is the clearest statement of that.
`compile_allocs`, `compile_peak_bytes`, rounds and visits are byte-identical.

**Not attempted.** Arity zero, three and four have twins and no call sites in
any benchmark. The generator writes them if a program asks; nothing measured
does.

**OPEN, small, unpinned.** `k_call2` tests its arguments for failure before it
tests arity; the interpreter tests arity first. A value-headed call with the
wrong arity AND a failing argument would therefore return the failure on
native and die with an arity message on the oracle. No program in the corpus
reaches it and I did not find a way to write one — a literal lambda's arity is
checked at compile time, and reaching the runtime check means passing a
callable of one arity into a body that applies another. Recorded rather than
guessed at.

## 2026-09-02 (later) — the record read and the record test follow the dispatcher

**DONE** (kanso#PR). #1210 moved the value-headed dispatcher to the call site
and the profile it left named the next two: `k_check_rec` at 2.16% of
encodebench's own instructions and `k_field` at 0.74%, before the call frames
at any of the 134 sites the compiler writes for them across the ten
benchmarks. A fold that matches a record pays both once a lap.

Both are the same shape as the twins DECLARES already carries for
`k_check_tag`, `k_check_int` and `k_check_bool`, so they go in beside them
rather than being generated against the body.

**The first version tested the wrong thing and three rows rose.** It asked
`tag == K_REC` and sent everything else to the runtime, which put every
"this value is not a record at all" answer through a call it did not need:
pendbench fell 18.9% and encodebench only 0.18%, while widebench rose 0.100%
and digestbench 0.482%. The runtime's own first line is `if (v.tag == K_SUB)`,
and asking that instead lets the twin answer every shape the runtime answers —
a record by reading it, anything else with a flat zero — leaving it only a
wrapper to walk. Nothing rises after that.

Reordering to test `K_REC` first and decide between the wrapper and the flat
no in the else arm was built and measured: byte-identical output, because LLVM
canonicalises the two orderings to the same code. The shorter form is the one
that shipped, because it is also the one the C is written in.

**The rows, measured locally against the same tree built both ways.** Seven
fall, three hold, none rises.

```
pendbench       912,183,826 ->   749,657,820   -17.817%
encodebench   7,257,716,059 -> 6,947,803,659    -4.270%
oneshot          39,449,698 ->    38,669,395    -1.978%
digestbench     137,057,230 ->   134,729,014    -1.699%
deepbench       701,522,218 ->   692,270,218    -1.319%
basket           56,138,967 ->    55,762,087    -0.671%
widebench        63,873,599 ->    63,601,599    -0.426%
```

pendbench carries it because its pending cells are records read once a lap in
a loop that does almost nothing else. jsonbench, escapebench and indexbench are
byte-identical: the decoder BUILDS records rather than matching them, and its
`k_field` sites sit in library code its entry never reaches.

`k_field` is gone from every profile — fully inlined. `k_check_rec` is not:
encodebench still spends 116,146,800 in it against 156,995,200 before, because
a good share of what it matches is a subtype and a subtype still walks its
chain in the runtime.

**What it costs, and this one is not small.** Six binaries gain 2,992 to 4,752
bytes of `.text`, 3.0% to 6.1% — the largest single move this vein has
recorded, and about six times what #1210 paid. A record read plus a record
pattern test is more code than a call to one, at 134 sites. No allocation
counter moves on any of the nine, and every emitted program gains two defines,
three calls, three branches and forty-nine lines.

**Welfare cannot see the bytes.** Its twenty-one terms are allocations, arena
blocks, peaks and instruction counts; `.text` is in none of them, so the index
will read this as pure gain. The text golden is the vein that watches the
half welfare is blind to, and this entry is where the trade is stated: 24.9
million instructions of runtime work for 22,704 bytes of machine code across
six binaries.

**Four mutations, four verdicts.** Moving `k_field`'s fields offset from 16 to
24 reddens a single record sample. The other three are invisible to one sample
and caught by the corpus: `K_SUB` 15 to 14 fails 2 of the golden suite's 10
tests, `K_REC` 7 to 6 fails 5, and reading `nfields` from offset 16 instead of
8 fails 5. Each was watched red before it was watched green.

**The rows CI counted**, on family 0x6 model 0xad rather than the recorded
Genoa. Every delta matched the container's to the instruction, the same way
kq#91's four rows did on the same day.

```
pendbench       912,184,212 ->   749,658,206
encodebench   7,257,716,458 -> 6,947,804,058
oneshot          39,450,097 ->    38,669,794
digestbench     137,057,629 ->   134,729,413
deepbench       701,525,898 ->   692,273,898
basket           56,139,380 ->    55,762,500
widebench        63,874,012 ->    63,602,012
```

`compile_instructions` lands on 41,495,470, a FALL of 5,049 — and the change
before this one ROSE 4,747 on the same host for the same reason. `kanso check
lib/json` emits nothing, so neither number is the front end doing more or less
work; both are its own code layout moving under a larger codegen.rs.
`compile_allocs`, `compile_peak_bytes`, rounds and visits are byte-identical.

**Welfare 74.6928 to 74.8069, and the floor is set.**

**Pricing the seventeen counters that worsened.** Sixteen count the same two
definitions the compiler now writes into every program, times the programs.
The single-file samples in the compile golden land on `defines` 104, `calls`
112, `branches` 87 and `lines` 1,787; the module sample on `module_defines`
80, `module_calls` 756, `module_branches` 378 and `module_lines` 4,529. The
decoder lands on `emitted_defines` 158, `emitted_calls` 1,792,
`emitted_branches` 1,177 and `emitted_lines` 11,629, and across the other ten
programs `emitted_other_defines` 1,359, `emitted_other_calls` 14,372,
`emitted_other_branches` 8,419 and `emitted_other_lines` 83,669.

The seventeenth is the one that counts code rather than definitions: `text`
lands on 752,930 across the nine, up 21,952. That is the number this entry is
really about, and the paragraph above says what bought it.

**OPEN.** The subtype walk is what `k_check_rec` still costs encodebench, and
the twin hands every subtype to it. Whether a one-level unwrap belongs in the
twin is a measurement nobody has taken: the chain is usually one deep, and
the loop is there for the case where it is not.

## 2026-09-02 (later still) — the subtype unwrap, and the number that sent me after it

The entry above closed with an open thread: `k_check_rec` still cost
encodebench 116,146,800 instructions, a subtype still walked its chain in the
runtime, and whether a one-level unwrap belonged in the twin was unmeasured.
Two of those three claims are wrong, and finding out which took a build of the
parent commit.

**The correction.** At aaaee3f6, encodebench spent 156,995,200 in
`k_check_rec` and 53,510,400 in `k_field`. That "before" figure is right. At
39e0683a neither symbol is entered once — not by encodebench, and not by any
of the other nine. The 116,146,800 does not appear in the profile at any
threshold, and I cannot reconstruct where it came from. The mechanism was
wrong too: encodebench declares no subtype, so nothing it matches could have
walked a chain.

**Why the corpus could never have answered the question.** Across
`bench/` and `std/`, and across all of kq, the count of subtype declarations
is zero. Forty-three appear in `tests/golden`, where they are pinned for
correctness, and three in the book. So the twin's slow arm is dead code in
every program any vein measures, and a profile row for the subtype path cannot
exist. The open thread was about coverage, and no amount of profiling the
corpus would have shown that.

**Built anyway, and measured against a program that uses the feature.** A
wrapper subtype over a two-field record, matched in a fold 200,000 laps deep,
runs 311,819,732 instructions at 39e0683a, with `k_field` at 1.67% and
`k_check_rec` at 1.28%. Two shapes of the unwrap were built. The first hoists
the payload extraction to the entry block and shares it between the plain and
subtype arms; the second keeps it inside each arm. Both answer identically,
both empty the slow arm, and both land within nine instructions of each other:

```
                        subtype program    ten-benchmark corpus
baseline                   311,819,732     (the pinned rows)
shared extraction          302,019,731     three rows rise, two fall
extraction in each arm     302,019,722     three rows rise, two fall
```

−3.14% where the feature is used. On the corpus, which does not use it:

```
                v1 (shared)          v2 (per-arm)
jsonbench            +0.000%              +0.000%
encodebench          -0.016%              +0.131%
oneshot              -0.007%              +0.059%
basket               +0.143%              +0.135%
widebench            +0.000%              +0.000%
deepbench            +0.000%              -0.008%
escapebench          +0.000%              +0.000%
pendbench            +0.642%              +0.214%
indexbench           +0.000%              +0.000%
digestbench          +1.207%              +1.201%
```

digestbench pays the most and pays it in one place: the whole
+1,626,177 sits in `d_list/fold_3`, the compiled fold, whose body carries
inlined copies of both twins. A block that never executes still costs the
function it sits in — it lengthens the body, moves the branches apart, and
changes what the register allocator has to keep live across the loop. My
first theory was the hoisted extraction, which is why v2 exists; v2 moved
pendbench and left digestbench where it was, so the cause is the size of the
twin rather than where inside it the extraction sits.

**DECLINED, by the objective.** With the v2 rows substituted into
`bench/instructions_golden.txt`, welfare reads 74.80 against a floor of 74.81
and the gate goes red. A change that costs the corpus a hundredth of a welfare
point to buy 3.14% on a program shape that appears nowhere the project
measures is worse by the project's own stated preferences, and the weights are
not what is wrong here. The twin keeps the shape #1211 gave it: every subtype
goes to the runtime, which walks the chain.

**What would change the answer.** A benchmark that declares a subtype. That
is a real gap — the feature has 43 correctness fixtures and no cost row — but
it is not a gap this entry should close on its own initiative, because adding a
row means a new vein, a welfare term, a weight and a saturation, and the case
for spending those on a feature no shipped program uses is a case somebody has
to make. Recorded here so the next session that reads a `k_check_rec` row does
not repeat the trip.

## 2026-09-02 (fourth) — six machine ops that were reached through a call

`k_b_bit_and` and its five siblings each take two ints, do one instruction to
them, and box the answer. Reaching one costs a call: arguments into the ABI
registers, a jump, a tag test the caller already knew the answer to, a return.
digestbench spent 10,619,517 instructions in the five it uses — 7.9% of the
whole benchmark — at about twenty-two instructions a call for work worth one.

The twin convention #1210 and #1211 built handles this without a new idea.
Six `define internal ... alwaysinline` bodies in `DECLARES`, each testing both
operand tags for `K_INT` and doing the op inline where they say yes. `shl` and
`shr` test the shift for 0..63 as well, because the runtime refuses anything
else and the twin has to refuse it the same way — which it does by handing the
call to the C entry that owns the message. `shr` is `ashr`: the runtime spells
an arithmetic shift by hand to avoid the implementation-defined `>>` on a
negative operand, and `ashr` is what that spelling computes.

**Two routes reach the same builtin, and taking only one leaves half the calls
in place.** `&`, `|` and `^` are operators and lower through the operator
table. `bits/xor`, `bits/complement`, `bits/shl` and `bits/shr` are kanso
functions in `lib/bits` over `builtin_bit_*`, and those lower through the
named-builtin path. Routing the operators alone left digestbench at
124,515,585 with 33,024 xor calls and 8,256 complement calls still going out
through `sha256/compress`, because sha256 spells complement and xor by name and
`&` by symbol. Routing both, and adding the unary `bit_not` twin, empties the
slow arm: no `k_b_bit_*` symbol is entered at all.

```
digestbench   134,729,014 -> 123,591,300   -8.267%
```

The other nine rows are byte-identical to the digit. No allocation counter
moves on any of the nine gates.

**What it costs.** Every emitted program gains six defines, seven calls, eight
branches and 117 lines, because `DECLARES` is written whole and a program that
does no bitwise work still carries the definitions. `.text` answers the
question that count cannot: digestbench goes 95,586 to 97,554 bytes, +2.06%,
and the other nine are byte-identical, because the linker drops what nothing
calls. Compile rounds and expression visits do not move — the front end does
no more work.

**The `.text` vein could not see the binary this landed on.** scanbench and
digestbench joined the corpus on 2026-08-31 and the list in
`scripts/gates/machine_code.sh` was never extended, so the two newest
benchmarks were the two with no `.text` row. Both are added here. The gap
mattered immediately: digestbench is the only binary this change moves, and
without the row the whole cost would have been invisible while the nine
byte-identical rows reported success.

**Four mutations, four caught, each by a fixture already in the corpus.**
Complementing with 0 instead of -1 turns `bits_surface`'s `comp: -13` into 12
and breaks all three sha256 vectors. `lshr` for `ashr` turns its `sign: -4`
into 4,611,686,018,427,387,900. Or-ing where the twin should and takes the
digest out of memory. Admitting a shift of 64 to the fast path replaces
`a_shift_past_the_word`'s refusal with a garbage address. Nothing new had to
be written: the pins were already there.

**Every counter this branch moved, and where it landed.** The four count
veins price the same six definitions four times over, once per corpus they
watch. In `bench/compile_golden.txt`, summed over its five samples: `defines`
104 -> 134, `calls` 112 -> 147, `branches` 87 -> 127, `lines` 1,787 -> 2,372.
In `bench/compile_golden_modules.txt`: `module_defines` 80 -> 86,
`module_calls` 756 -> 763, `module_branches` 378 -> 386, `module_lines`
4,529 -> 4,646. In `bench/emitted_golden.txt`, the decoder alone:
`emitted_defines` 158 -> 164, `emitted_calls` 1,792 -> 1,799,
`emitted_branches` 1,177 -> 1,185, `emitted_lines` 11,629 -> 11,746. In
`bench/emitted_golden_others.txt`, summed over its ten:
`emitted_other_defines` 1,359 -> 1,419, `emitted_other_calls`
14,372 -> 14,442, `emitted_other_branches` 8,419 -> 8,499,
`emitted_other_lines` 83,669 -> 84,833. And `compile_instructions`
41,495,470 -> 41,501,923, +6,453, which is what writing those definitions
costs the front end; rounds and visits do not move.

Every one of those rises is the same six definitions counted again, and the
`.text` row is what says whether any of it reaches a binary: digestbench
95,586 -> 97,554 and nine rows unchanged.

Welfare 74.81 -> 74.89.

## 2026-09-02 (fifth) — a bounds check and a load, reached through a call

`k_index` is what a demanded index compiles to — `l[i]!` — and it is
digestbench's largest single item at 29,195,608 instructions, 21.7% of the
benchmark, across 712,088 calls. Forty-one instructions a call for a bounds
check and a sixteen-byte load. `emit_at` already inlines the bytes case where
the sets prove it; the list case, which is what sha256 and every fold over a
list does, went out to the runtime every time.

A twin on the same pattern, used as the strict form's fallback so the existing
inline structure is untouched. It answers where the container is a list, the
key an int, the index in range, and the element a plain value. Three shapes go
to the runtime and each for its own reason: a failure or any other container
because the tag test fails, an out-of-range index because `k_index` owns the
missing-index err, and a thunk because forcing is the runtime's job.

```
digestbench   123,591,300 ->  98,968,855   -19.922%
deepbench     692,270,218 -> 677,478,218    -2.137%
basket         55,762,087 ->  54,722,493    -1.864%
oneshot        38,669,395 ->  38,048,160    -1.607%
pendbench     749,657,820 -> 749,618,528    -0.005%
encodebench 6,947,803,659 -> 7,014,919,659   +0.966%
```

jsonbench, widebench, escapebench and indexbench are byte-identical: they
index bytes and strings, which already had the inline path. `k_index` leaves
every profile.

**encodebench's rise is one function.** All +67,116,000 of it is in
`d_list/fold_3`, and nothing else moves at all. The fold does not index a list
— it carries inlined copies of a twin it never reaches, and a longer body is
a slower loop. This is the third time this vein has recorded that shape: the
declined subtype unwrap paid it on digestbench, kanso#1211 paid it as `.text`,
and here it is 0.97% of the largest benchmark. The trade is worth taking at
19.9% against 1.0%, and welfare agrees: 74.89 -> 75.09.

**`.text` goes both ways this time.** Five binaries shrink — encodebench
-144, oneshot -592, widebench -144, deepbench -176 — and four grow:
digestbench +2,272, scanbench +1,824, pendbench +640, basket +144. A call
replaced by a short inline body changes what the inliner does with everything
around it, and the direction is not predictable from the diff.

**Two fixtures the corpus did not have.** Five decisions in the twin, and
mutation found that only three of them were pinned by anything in the tree.

- The low bound. `index_missing.kso` pins index 9 on a three-element list; no
  fixture indexed at 0, and `>= 0` for `> 0` reads the word before the first
  element with every existing fixture green. `index_zero.kso` pins it.
- The stored `none`. A list literal holding one is refused where it is
  written, so it looks unreachable — but `push (push [] 1) none` builds one at
  run time, and `xs[2]!` on it is the missing-index err, because `at` answers
  "not found" with a none and `k_index` cannot tell the two apart. Without the
  deferral the twin answers `<none>`, which is a divergence from the
  interpreter that would have shipped. `index_holds_a_none.kso` pins it.

Both were watched failing: the mutated compiler reddens each golden with the
message the fixture exists to hold.

**The thunk deferral is not pinned, and I could not pin it.** No program I
wrote put an unforced thunk in a list and read it with `!`, and no benchmark
reaches one that way either — digestbench's 8,256 `k_force` calls come from
`sha256/compress` and pendbench's 100 from `keep_4`, none through `k_index`.
The guard mirrors the `k_force` that `k_index` itself does, and it stays:
removing it because I could not reach it would be a guess about the lazy
tier, and this log has one of those open already. The gap belongs to the lazy
tier's coverage rather than to this change.

**Every counter this branch moved, and where it landed.** One define, two
calls, three branches and thirty-nine lines on every emitted program, counted
four times over by the four count veins. In `bench/compile_golden.txt`, summed
over its five samples: `defines` 134 -> 139, `calls` 147 -> 157, `branches`
127 -> 142, `lines` 2,372 -> 2,572. In `bench/compile_golden_modules.txt`:
`module_defines` 86 -> 87, `module_calls` 763 -> 765, `module_branches`
386 -> 389, `module_lines` 4,646 -> 4,685. In `bench/emitted_golden.txt`:
`emitted_defines` 164 -> 165, `emitted_calls` 1,799 -> 1,801,
`emitted_branches` 1,185 -> 1,188, `emitted_lines` 11,746 -> 11,785. In
`bench/emitted_golden_others.txt`: `emitted_other_defines` 1,419 -> 1,429,
`emitted_other_calls` 14,442 -> 14,462, `emitted_other_branches`
8,499 -> 8,529, `emitted_other_lines` 84,833 -> 85,225. And `text`, the sum
over eleven binaries, 992,854 -> 996,678 — five of them smaller, four larger,
two unchanged, which the paragraph above breaks down.

**The rows this needed were in the middle of a job log.** The instructions
gate prints them, and then fourteen more gates run in the same job, two of
them dumping a hundred lines of CPU features and forty of callgrind. The log
API hands back a tail, so a session regenerating a golden either fetches the
whole log or writes down a number it did not measure. kq hit this on
2026-09-01 and fixed it by printing its rows after its own CPU dump; kanso's
version is a step at the end of the job that cats `work.txt`, `emitted.txt`,
`emitted_others.txt`, `text.txt` and `compile_ir_got.txt`, all of which are
still on disk. It runs on green as well as red, because thirty lines is
cheaper than the alternative and the numbers are worth having either way.

`compile_instructions` 41,501,923 -> 41,495,720, a fall of 6,203. The compiler
writes one more definition and does less work doing it, which is not a
contradiction: what `DECLARES` holds changes what the emitter's own string
handling does, and #1213 moved this the other way by 6,453 for the same
reason. Rounds and visits hold at 6 and 2,403; the front end decides nothing
different. No allocation counter moves on any of the nine gates.

## 2026-09-02 (sixth) — the index twin's rows, as the runner counted them

The entry above priced the index twin from container measurements, because
`measured_on` refuses a local regeneration of the instruction rows and says so.
The runner has now counted them, and this is what went into
`bench/instructions_golden.txt`:

```
jsonbench   2,718,705,899   unchanged
encodebench 6,947,804,058 -> 7,014,920,058   +0.966%
oneshot        38,669,794 ->    38,048,559   -1.607%
basket         55,762,500 ->    54,722,906   -1.864%
widebench      63,602,012   unchanged
deepbench     692,273,898 ->   677,481,898   -2.137%
escapebench   248,370,844   unchanged
pendbench     749,658,206 ->   749,618,914   -0.005%
indexbench      5,242,731   unchanged
digestbench   123,591,699 ->    98,969,254  -19.922%
```

Every delta matches the container's to within the constant offset the two hosts
have always carried, which is the fourth time they have agreed. The container
read encodebench's rise at +67,116,000 and the runner reads it at +67,116,000 —
the same number, not the same percentage, because the bases differ by 399
instructions of process startup.

**`work_encodebench` pays and `work_digestbench` is paid.** The rise is the dead inline body
again, the third recorded instance: `d_list`'s `fold_3` indexes nothing, reaches
the twin's slow arm never, and still carries its bounds check and its two loads
in the loop body. digestbench indexes a list on every lap and falls by a fifth.
The trade is worth taking on the sum — welfare goes 74.89 to 75.09 — but the
term that paid is named here rather than argued away.

**What is still unpinned.** The twin defers a thunk and a stored `none` to
`k_index`, and `tests/golden/runtime/index_holds_a_none.kso` pins the second.
The first is unreachable by any program I could write: no fixture in the tree
builds a list holding an unforced thunk and then indexes it. That gap belongs to
the lazy tier's coverage rather than to this change, and it is recorded so the
next session working the lazy tier finds it.

## 2026-09-02 (seventh) — what the index twin costs kq, which indexes no lists

kq is the corpus's one real program and it is not in the objective, so it gets
measured by hand at each pin bump. Built `--release` by the same compiler
either side of the index twin, in this container, four rows and the machine
code:

```
print_small  69,857,190 ->  70,033,408   +0.252%
print_big   709,519,790 -> 711,281,970   +0.248%
path_small   19,079,322 ->  19,081,118   +0.009%
path_big    198,937,158 -> 198,938,954   +0.001%
.text           114,242 ->     114,098     -144 bytes
```

Every row rises. kq indexes byte strings and maps, never a list in a loop, so
its fifteen index sites all take the twin's slow arm and pay the two tag reads,
the two compares and the branch on the way to the call they were making
anyway. The print rows carry it because printing walks every value; the path
rows index once per run and move by 1,796 instructions, the same number twice.

**This is the fourth sighting of the dead inline body**, and the first on a
program outside the corpus: the declined subtype unwrap cost digestbench 1.2%,
kanso#1211 paid for it in `.text`, this change pays for it in encodebench's
`fold_3`, and here it is again in kq's printer. The twin is routed at every
strict index the compiler emits, whatever the container is, because emit-time
routing asks no question about the type. Four measurements now say the same
thing about that, and the next move on this is to ask: `infer` already decides
a type for the collection expression, so a twin routed only where the answer is
a list would keep digestbench's fifth and give encodebench and kq their quarter
per cent back. That is a measurement to take, not a conclusion.

**The objective still says take the change**, and the objective is what
decides: welfare 74.89 -> 75.09 on the corpus that carries the weights. kq's
quarter of a per cent is real, it is recorded here rather than left for its pin
bump to discover, and it is the price of digestbench's fifth.

**The first run of this measurement was wrong and is worth saying why.** It
built kq without `--release` and read 100,687,166 for print_small against a
golden of 76,695,994, then reported the twin as costing kq nothing. kq's CI
builds with `--release` and the deltas do not survive the difference: the
unoptimised build hid the twin's cost entirely. A benchmark built differently
from the way it is published measures a program nobody runs.

## 2026-09-02 (eighth) — two ways to stop the twin costing encodebench, both measured, both dead

The entry above said routing the twin only where `infer` says a list can arrive
was a measurement to take. It is taken, and so is a second one, and neither
works. Both are recorded here so nobody builds them again.

**Route the twin by the container's typeset.** `emit_at` already has
`f.set_of(container)`; the change is four lines, sending a strict index to
`k_index` directly where `set & LIST == 0` and to the twin otherwise. It
reroutes real sites — encodebench moves two, kq moves seven of fifteen — and
the corpus does not move at all:

```
jsonbench 2,718,705,486   encodebench 7,014,919,659   oneshot 38,048,160
basket    54,722,493      widebench      63,601,599   deepbench 677,478,218
escapebench 248,370,445   pendbench     749,618,528   indexbench 5,242,318
digestbench  98,968,855
```

Every row identical to the digit with the twin routed everywhere. On kq it
recovers 8,940 instructions of the 176,218 the twin costs print_small — five
per cent — and shrinks `.text` by a further 112 bytes. Not worth four lines.

**Drop `alwaysinline` and let LLVM decide.** encodebench and digestbench come
back byte-identical again; basket rises 3,060 and pendbench 600. LLVM inlines
the twin at these sites whether or not it is told to, so the attribute is not
what puts the body there.

**What the profile actually says, and it is not what the words said.** The
+67,116,000 in encodebench sits entirely in `d_list/fold_3` — every other
function in the binary is identical to the instruction across the two builds,
`k_b_append_mut` at 1,204,501,600 and `w_klam17` at 488,086,400 either way. And
`k_index` appears nowhere in the no-twin profile at all: encodebench never
indexes a list at run time. `fold_flat` is the arm that writes `coll[i]!`, and
encodebench folds the lazy shapes, so `fold_go` runs and `fold_flat`'s index is
dead every lap.

So the cost is not paid at the index. It is paid by whatever the larger
`fold_3` does to register allocation and block layout in a loop that runs
millions of times, which is why neither routing nor the inline attribute
touches it — both change WHERE the body goes, and the body is not the problem
where it goes. A fix would have to keep `fold_flat` out of the same function as
`fold_go`, which is a question about how dispatch groups are emitted rather
than about this twin.

**And a note on measuring this.** The first attempt at the comparison above
annotated `/tmp/cg.encodebench`, a profile file left over from an earlier
session, and read a total of 8,396,592,003 against a benchmark that retires
7,014,919,659. A stale profile answers confidently and wrongly. Build the
binary, run it, annotate the file that run wrote, in one script.

## 2026-09-02 (ninth) — three benchmarks the objective could not see

`scripts/gates/instructions.sh` swept ten of the eleven benchmarks
`build_benchmarks.sh` builds. The eleventh was scanbench, and its absence was
the worse kind: its `arena_blocks` and `peak_bytes` are both welfare terms, so
a change that gave the arena back per position and spent ten times the
instructions doing it scored as a pure gain. That is the exact asymmetry the
digest paragraph in that gate warns about, written a day earlier about a
different benchmark.

Two more were missing at the other end. escapebench and indexbench had rows in
`bench/instructions_golden.txt` — the trend gate reads them, a regression in
either turns CI red — and `scripts/welfare/welfare.kso` read neither. Their
work was weighed at zero in the one number that decides whether a change is
worth having.

So: scanbench joins the gate, and `scan_instructions`, `escape_instructions`
and `index_instructions` join the model as an `edge_work` group.

**Landing day moves the number by nothing, and that is measurable rather than
asserted.** All three enter as granted baselines at their dimension's standing,
which reads +265.9% for each of them, the run-speed dimension's current
average. Welfare before: 75.09. Welfare after: 75.09. The rule that a granted
baseline enters neutral is doing exactly what #1198 said it would.

**A note on how this was nearly got wrong.** The first reading was 74.23, a
fall of 0.86, which would have been a real argument about the weights. It was
the placeholder: scanbench's row was `0` at that point, and a current value of
zero is better than any baseline, which sends that term through the guard for
division and out the other side distorted. With the real magnitude in place the
number does not move. A placeholder that is not obviously a placeholder is a
measurement waiting to be believed.

**And the spec, because the list was the problem.**
`tests/every_benchmark_is_in_the_objective.rs` reads the three files and holds
them together: every benchmark built has its instructions counted, every one
counted has a row, every row is read by the model. It carries no list of its
own — a list here would be the twelfth place to forget. Each of the three
assertions was watched failing: removing scanbench from the gate's loop, its
row from the golden, and escapebench from `worked` each redden exactly one and
name the benchmark.

scanbench's row is a container measurement and a labelled placeholder until the
runner replaces it; the gate is red on purpose until then, and the golden's
comment says so.

## 2026-09-02 (tenth) — the runner's scanbench row, and the spec that already existed

scanbench's instruction row, counted on the runner: **1,423,437,886**. It went
in as a placeholder at 1,423,437,473, what a container measured, so the gate had
something to diff against and printed the real row in its job log. The two
differ by 413, which is the same constant offset jsonbench carries between
those hosts.

**A spec I did not write caught what mine missed.**
`every_benchmark_in_the_work_vein_has_a_direction` reddened on the first push:
scanbench had a row in `bench/instructions_golden.txt` and no entry in the
trend gate's `lower_*` tables, so a rise in it would have read as UNCLASSIFIED
drift and the gate would have exited green. That spec exists because
digestbench was omitted from the same list on 2026-08-31 and a 6.5x regression
passed. It caught the next one in the pull request that created it, which is
the whole point of writing the spec rather than the note.

The coverage chain is now four links, not three: built → counted → rowed →
weighed, and rowed → given a direction. `every_benchmark_is_in_the_objective`
holds the first three and the older spec holds the fourth. Neither carries a
list of benchmarks; both read the files.

`work_scanbench` reports as MINTED rather than as a move, which is what
kanso#1200 built that state for.

## 2026-09-02 (eleventh) — the tenure residual is reached, and it is harmless

runtime.c has said since kanso#1209 that one case was narrower rather than
closed: a node below the beat's mark, repaired during the beat to hold a
tenured pointer, outliving the rewind that frees the block. It ended "Nothing
measured has reached it," and four attempts to write a program that did had
failed.

**Instrument the condition instead of guessing at programs.** A flag set for
the duration of `k_repaired_settle` and a counter in `k_survives_x` that fires
only when the yes came from `k_ten_holds` names the exact case. Across all
eleven benchmarks, the full spec suite, the trend gate and the welfare script
it fires zero times — and a second counter, incremented on every `k_survives_x`
call during a settle, proves the detector is live rather than dead: it fires on
encodebench and pendbench.

**The two halves are disjoint, which is why the attempts failed.**

```
                ten_blocks   settles
widebench            1          0
indexbench           1          0
scanbench            1          0
encodebench          0          1
pendbench            0          1
the other six        0          0
```

Three benchmarks tenure and never repair; two repair and never tenure. No shape
in the corpus does both, so no variation on a corpus shape was ever going to
reach it. Naming the two conditions separately is what made the combination
obvious.

**The combination.** Tenuring needs the value copied into the carry buffer, so
it must be built INSIDE the bind — above the mark, copied every lap, promoted
from the second. Repair needs a container the walk finds below the mark, so the
accumulator must be made BEFORE the first bind. Put an element of the first
into the second and a node below the mark holds a pointer into tenure. First
attempt with the list built outside the bind: repairs, `ten_blocks=0`. Moving
one binding inside: `RESIDUAL REACHED`, `ten_blocks=1`, `ten_frees=1`.

**And it is harmless, for the reason #1209 built.** The beat's result is a heap
value, so `k_ten_hand_up` gives the block to the depth outside instead of
freeing it at that pop, and it outlives every read of the repaired node. Native
and the interpreter print the same bytes; valgrind reports nothing.

`tests/golden/mem/a_repaired_node_below_the_mark_holds_tenure.kso` is the
fixture, pinning `ten_blocks=1` beside `ten_frees=1` and `survive_slots=403`.
It is the only program in the tree that reaches the case, so a change that
stops handing tenure blocks up turns it red rather than turning some later
program into the segfault in `k_copy_size` that started all of this. The
comment in runtime.c is corrected: it said nothing had reached this, and
something has.

## 2026-09-02 (twelfth) — a comment in runtime.c moves the compile row

The correction above turned `compile_instructions` red: 41,495,720 ->
41,495,304, a fall of 416, from a change that is entirely a comment.

`src/main.rs` reaches the runtime through `include_str!("runtime.c")`, so the
file's bytes are part of the compiler's data section. `kanso check lib/json`
never emits the runtime and never reads that string; the front end does exactly
the work it did before, at slightly different addresses, and the count comes
out 416 lower for it.

Worth writing down because the obvious reading of a move in this row is that
the front end changed, and here nothing in the front end was touched. A comment
in runtime.c is not free in this vein, the direction is not predictable from
the edit, and the number that fell is a layout artefact rather than a win to
bank. It is recorded as an improvement because that is what the row says, and
this paragraph is what stops the next reader crediting it to a pass.

## 2026-09-02 (thirteenth) — the 67 million is a lost specialisation, and the fix is declined

kanso#1214 left one thing unexplained: encodebench pays +67,116,000 for the
index twin, all of it inside `d_list/fold_3`, in a program that never indexes a
list. Routing by typeset and dropping `alwaysinline` were both measured as
no-ops, so the answer was not where the body goes.

**The compiler emits the same function either way.** `d_list/fold_3`'s IR is
byte-identical across the two builds, and so are `fold_flat_4` and
`fold_go_3`. Nothing the front end writes changed.

**The machine code got smaller and slower.** `fold_3` is 4,083 bytes without
the twin and 3,682 with it — 401 bytes less code running fourteen per cent more
instructions. Its call sites collapse with it: `k_call2` four to one, `k_index`
four to one, `k_b_length` two to one, `k_truthy_bad` two to one. LLVM had been
specialising the dispatcher into four copies; the twin's `alwaysinline` body,
inlined into `fold_flat_4`, pushed its inline cost past the point where that
paid, and one shared body with more branching replaced four specialised ones.

**Forcing the twin out of line restores it exactly.** With `noinline` on
`k_index_fast`, `fold_3` is 4,083 bytes again — the same number, not a similar
one — the four `k_call2` sites are back, and encodebench reads 6,947,803,659,
its pre-twin count to the instruction. That is not a plausible story about
inlining; it is the same object file's worth of decisions coming back.

**And the objective declines the fix.** `noinline` is a trade, not a win:

```
encodebench  -67,116,000   -0.957%
oneshot          -30,113   -0.079%
pendbench         +9,392
basket          +283,126   +0.517%
deepbench     +4,856,000   +0.717%
scanbench     +5,516,906   +0.388%
digestbench   +8,244,421   +8.330%
```

digestbench keeps only two thirds of what #1214 bought it, and welfare goes
75.09 -> 75.03. The sum of the raw counts falls by 48 million and the objective
still says no, which is the weights doing exactly what they are for: the
benchmark that pays is the one the change was built for.

**scanbench is on that list because kanso#1215 put it there**, four hours
earlier. Without it this trade would have scored 5,516,906 instructions
cheaper than it is, and the answer might have come out the other way. A
benchmark weighed at zero does not make a change look good; it makes the
NEXT change look good, which is worse, because nobody is looking at the
benchmark when they read the score.

The twin stays `alwaysinline`. What is now known and was not: the 67 million is
LLVM's specialisation threshold, the lever that reaches it is `fold_flat_4`'s
inline cost, and both are properties of a dispatch group emitted as one
function. Whether dispatch groups should be emitted so a cold arm cannot price
a hot one out of specialisation is a design question, and it is Clay's.

## 2026-09-02 (fourteenth) — that last question was mine, not Clay's

The entry above ended by calling the dispatch-group question a design question
and Clay's. design/pending-gavels.md says otherwise, in its second paragraph:
an entry goes to him because it is about the language a user meets — surface,
semantics, observable behavior — and "implementation details do not come here;
whoever holds the file decides them and answers for the decision in the log."

How a dispatch group is emitted is not something a user meets. Nothing about
`fold`'s meaning changes either way. So the question is mine, and this is the
answer.

**Not now.** The one lever measured — `noinline` on the twin — is declined by
the objective at 75.09 -> 75.03. The larger change, emitting a dispatch group
as a function per arm rather than one body, is unmeasured and would undo part
of what kanso#1140 built when it made a dispatch group a range; a change of
that size on a hypothesis this thin is exactly the design note the log keeps
telling sessions not to write.

**What would reopen it.** The instructions vein now covers all eleven
benchmarks, so the shape shows up on its own: a benchmark that does not use a
feature rising when that feature's twin lands, with its `.text` FALLING at the
same time. Down and slower together is the signature — it is what encodebench
did here, 401 bytes smaller and 14% dearer — and it does not look like anything
else. Two more sightings with the same signature and the change has a
measurement behind it instead of one.

The correction matters beyond this entry: a question filed to Clay that is not
his costs him a sitting and costs the ledger its meaning, and the rule against
it is written at the top of the file it would have gone in.

## 2026-09-02 (fifteenth) — the one vein the objective leaves out, and why

`bench/text_golden.txt` is the only deterministic vein `scripts/welfare` does
not read. It reads eleven golden files; that is not one of them.

It looks exactly like the gap kanso#1215 closed four hours ago — eleven
benchmarks measured by a gate, pinned in a golden, watched by the trend gate,
weighed at nothing by the objective — and the obvious repair is to give code
size a term. That would be wrong, and kanso#1217 is the measurement that says
so.

The index twin took encodebench's `.text` DOWN 144 bytes and its instruction
count UP 67,116,000 in the same change. Inside `d_list/fold_3` the effect is
starker: 4,083 bytes to 3,682, four hundred and one bytes less code running
fourteen per cent more instructions, because a four-way specialisation was
lost. A term rewarding smaller `.text` would have scored that regression as a
gain, twice over.

So code size is a diagnostic here, not a cost. It says a kernel arrived or
left, which is the job `scripts/gates/machine_code.sh` does and did when the
bit twins landed on digestbench with every other row holding. What it does not
do is stand in for what a program costs to run, and on this corpus it has been
measured pointing the wrong way.

**Written as a spec because kanso#1137 settled that prose is not a pin.** Four
claims in this tree rested on comments and none of them held.
`tests/the_objective_does_not_weigh_machine_code_size.rs` goes red if welfare
starts reading the vein, with the measurement in the failure message, and red
the other way if the gate stops diffing the golden — an exclusion from the
objective is not permission to stop counting the thing. Both halves were
watched failing.

## 2026-09-02 (sixteenth) — the log back to forty

116 entries and 6,030 lines, against the rule at the top of this file that it
holds the last forty. The oldest 76 move to `design/log/compiler-log-archive.md`
unedited, which takes the live file to 2,680 lines and the archive to 946
entries.

Nothing is rewritten and nothing is summarised. Checked rather than asserted:
986 entries before across the two files and 986 after, the forty kept are the
last forty byte-for-byte, the archive is its old contents followed by the moved
76 byte-for-byte, and the live file's header is unchanged. The header already
said "the last forty entries" while the file held 116; it is true again.

kanso#1166 moved 72 the same way on 2026-08-29 and kanso#1183 did it again on
2026-08-31, which is roughly one trim a day at the rate this log is being
appended to. That is the cost of the discipline working.

**One thing the trim surfaced, and it is not the log's.** The full suite came
back with two wasm specs refusing to run: `docs/kanso.wasm predates
codegen.rs`. Nothing in this change touches the compiler — `src/codegen.rs` is
byte-identical to main — but the `noinline` experiment of kanso#1217 edited it
and `git checkout` gave it a new mtime on the way back. The guard compares
timestamps, so a file whose content never changed reads as newer than the blob.
Rebuilding the blob produced a byte-identical `docs/kanso.wasm`, which is the
proof the content was never the issue.

That behaviour is known and was DECLINED with reasons: a content hash costs a
build to compute and the mtime comparison catches the case it exists for. This
entry records the false positive it does produce, so the next session that
meets it after reverting an experiment recognises it in one line instead of
hunting a compiler change that is not there.

## 2026-09-02 (seventeenth) — the page owes §35, and the drift gate said so

The log-trim branch went red on `scripts/page_drift`: four entries since
docs/compiler.html last moved, against a budget of three. The gate is right.
Three of those four are one argument — what the objective weighs, what it
refuses to weigh, and the measurement behind the refusal — and none of it had
reached the page.

§35 is that argument. `bench/text_golden.txt` is measured on every run, pinned
in a golden, diffed by CI and weighed at nothing; the index twin took
encodebench's `.text` down 144 bytes and its instruction count up 67,116,000 in
the same change, so a term rewarding smaller code would have scored that
regression as a gain twice over. The section carries the `d_list/fold_3`
numbers, the `noinline` restoration to the byte, the spec that pins the
exclusion, and the decline at 75.09 -> 75.03.

The fourth entry is the trim itself, which is housekeeping and owes the page
nothing. One section for the campaign is what the gate's own message asks for.

**The gate counts from the page's last commit, so the entry and the page edit
ride together.** `git diff <last page commit>..HEAD -- design/compiler-log.md`
reads empty when both land in one commit, which is why this entry does not put
the branch back over the budget it just cleared.

## 2026-09-02 (eighteenth) — the last append still paying a call

`k_b_append_mut` was the largest single symbol anywhere in the eleven
benchmarks: 1,204,501,600 instructions, 17.17% of encodebench, more than the
next two entries together. It was also the only append reaching the runtime
through a call.

`append` has had an inline twin since the DECLARES prelude was written —
`k_b_append_byte`, which claims a byte at the accumulator's frontier and
builds a fresh header in the arena. The routing gives it to every append the
linearity analysis has NOT proved unique. The proved-unique site, which is the
whole of an encoder's hot path, went to the C. So the append with the smaller
body — no header to allocate, the length written where it already sits — was
the one paying a frame.

`k_b_append_mut_byte` is `k_b_append_byte` with the header work removed. Same
four guards in the same order: bytes accumulator, int argument, owned buffer at
its frontier with room for one more. Anything else falls through to
`k_b_append_mut` inside the twin.

Measured in this container against `origin/main`, both arms built from the same
sources with every benchmark binary deleted first:

```
encodebench  7,014,919,659 -> 6,611,648,059   -403,271,600  -5.749%
oneshot         38,048,160 ->    36,606,462     -1,441,698  -3.789%
jsonbench    2,718,705,486 -> 2,673,669,936    -45,035,550  -1.657%
widebench       63,601,599 ->    63,809,599       +208,000  +0.327%
```

The other seven are byte-identical. `work_widebench` lands on **63,810,012** on
the runner and is the one row that rises; widebench carries one long list for
the whole run and appends to it out of an outer frame, which is the shape where
the extra inline body sits in a loop that mostly does not take the fast arm.

Every one of the nine allocation cost goldens is byte-identical, which is what a
pure inlining change should do and is the check that it was one.

**Where it costs, and every counter that moved.** The twin's body sits in the
DECLARES prelude, so every program emits it whether or not it reaches it — one
define, one call, five branches and 51 lines apiece, uniformly. That lands on
`emitted_defines` 166, `emitted_calls` 1,802, `emitted_branches` 1,193 and
`emitted_lines` 11,835 for the decoder, and on `emitted_other_defines` 1,439,
`emitted_other_calls` 14,472, `emitted_other_branches` 8,579 and
`emitted_other_lines` 85,732 across the ten beside it. The compile samples take
the same prelude: `defines` 144, `calls` 162, `branches` 167, `lines` 2,827, and
the module sample `module_defines` 88, `module_calls` 766, `module_branches`
394, `module_lines` 4,736.

`text` lands on **1,004,950**, and the four binaries that actually reach the
twin are all of the rise: jsonbench +976 bytes, encodebench +1,504, oneshot
+4,080, widebench +1,712. §35 of the compiler page, written four hours ago, is
the reason machine-code size is read here as a diagnostic rather than as a
cost — and this change is a second instance of what that section argues, code
growing while the work falls.

**The fixture, and what it does not reach.**
`tests/golden/micro/an_in_place_append_reaches_its_twin.kso` threads a byte
accumulator through a tail recursion, which is what makes the site in-place,
and sends a byte and a string through the same call site. It prints the length
and the sum of the bytes, because a length alone is right even when the content
is garbage — the first version of this fixture printed only the length and
caught one mutation in five.

Five mutations of the twin, each watched:

- **the tag test** dropped: the string's payload is truncated to a byte and
  stored, the sum reads 219,066 against 245,066. RED.
- **the in-place length write** dropped: 2002 bytes and 90,066 against 4002 and
  245,066. RED.
- **the capacity test** dropped: output identical, and valgrind reports two
  invalid accesses. The grow path never runs, so the writes go past the
  nominal buffer into arena slack that nothing else is using yet. Caught by
  memcheck, not by the program.
- **the owned test** dropped: caught by neither. With `cap` zero the capacity
  test fails anyway, so the fast arm is not entered; what the missing guard
  costs is a read of `data[-8]` on a borrowed buffer, which lands inside the
  string's own header and is in bounds. The C guards the same read the same
  way.
- **the frontier test** dropped: caught by neither, and the reason is
  structural. A byte string sitting behind its buffer's frontier is one whose
  storage another value has extended, which is what the linearity analysis
  excludes from in-place sites in the first place.

So the fixture pins two of the five arms, memcheck pins a third, and two are
transcriptions of the C's guards with no shape in the corpus that separates
them. That is written down rather than rounded up to "pinned".

**The page's decode attribution was two merges stale, and is re-sat here.**
§07's "where the decode cost actually sits" was measured 2026-09-01 on a tree
whose jsonbench totalled 2,747,369,705; #1213 and #1214 landed after it and the
paragraph never moved, so it named `k_b_append_mut` at 3.80% of a decode that
no longer existed. Re-measured by the same method — the classifier reproduces
that sitting's emitted figure of 1,729,183,050 to the instruction on
`origin/main`, which is what says it is the same method:

```
                 2026-09-01 (main)      2026-09-02 (this branch)
  emitted kanso  1,729,183,050  63.6%   1,783,922,550  66.7%
  runtime.c        941,714,306  34.6%     841,939,256  31.5%
  libc              47,808,090   1.8%      47,808,090   1.8%
  total          2,718,705,486          2,673,669,936
  per input byte          96.1                    94.5
```

The runtime half fell 99,775,050 and the emitted half rose 54,739,500, which is
the twin's body crossing the line between them; the difference is the win.
`k_b_append_mut` is off the list at any position. The largest runtime entries
are now `k_b_put_mut` at 4.48%, `k_b_push_mut` at 3.94% and `k_b_find2` at
3.18%.

**And the next one is already named.** In encodebench after this change the
largest runtime entries are `k_b_find2_below` at 7.28% (481,347,200),
`render_ryu` at 7.07%, `k_b_append_wide` at 5.92% (391,134,400) and `k_b_at` at
3.83%. `k_b_append_wide` is the string arm of the same builtin this change
inlined the byte arm of, reached by every `"true"`, `"null"` and object key the
encoder writes.

**CI confirmed all eleven predicted rows exactly**, which is the seventh
consecutive time the container-to-runner delta has transferred to the
instruction. The one row that had to come from CI is the compile vein:
`compile_instructions` lands on **41,501,391** against 41,495,304, a rise of
6,087. The whole of it is the twin's body in the DECLARES string, which is data
in the compiler's binary, so a longer prelude moves where everything after it
lands — the same mechanism kanso#1216 recorded for a comment in `runtime.c`.
`kanso check lib/json` never emits IR and never reads the prelude, so the front
end does exactly the work it did before. 0.015% of a compile for 403 million
instructions of encode.

**One spec went red, and it is a real find rather than this change's.**
`tests/welfare_saturates_each_counter.rs` asserts what a single runaway
counter can contribute to the run-speed term, and its number is a property of
how MANY run-speed counters there are. It read 49.16 over eight. This change's
`--set` was the first ratchet since kanso#1215 minted `scan_instructions`,
`escape_instructions` and `index_instructions` four hours earlier, so the
floor's baseline gained three names and the fixture — which takes its names
from that baseline — divides by eleven now: (10/3 + 1024/1026) / 11 * 0.30
plus the other three terms is **48.48**, which is what both hosts read.

**A minted counter enters the baseline at the next ratchet, not at the merge
that mints it.** So kanso#1215 left this spec green and the next `--set`
turned it red, whoever ran it. That latency is now written into the spec
beside the number, with what to recompute when it happens. The number stays
pinned rather than derived: a spec that recomputes what the tool computes is
asserting its own copy of the tool, which is the objection its own harness
comment already makes about re-reading the goldens.

Welfare **75.09 -> 75.19** on the runner rows, ratcheted.

## 2026-09-02 (nineteenth) — the twins learn what the inference knows, and the append learns strings

Three things, and the third only exists because the second was measured and
beaten.

**The non-strict index had no inline form at all.** `k_index_fast`, which
kanso#1214 built, is the STRICT index's fallback and knows lists only. Every
`xs[i]` written without the `!` went to the runtime by call — 7,237,200 of them
in encodebench at thirty-five instructions apiece, 253,302,000, 3.83% of the
benchmark. `k_b_at_fast` inlines the two containers that answer in one load: a
list slot and a byte, which keep their length at offset 0 and their data
pointer at offset 8. A map, a string, out of range, a failure: all of it falls
through, so `none` and the utf-8 seek stay where they were written.

**A site the inference has already decided keeps its call.** A container
narrowed to STR can only take the index twin's slow arm, because the utf-8 seek
does not inline. Those sites now call `k_b_at` directly and pay no tag test for
a question answered at compile time. That removes indexbench's cost EXACTLY:
5,342,321 back to **5,242,318**, the same number, with every other row
byte-identical.

**The same trick did NOT win for the append, and building both is how that was
found.** Routing known-string appends past the twin scored welfare 75.22.
Giving the twin a string arm that memcpys scored **75.25**. So the arm, not the
route.

**And the arm's first shape was wrong in a way jsonbench found.** Sharing the
two arms' guards through a phi costs the BYTE path two instructions per append
— a phi and a second branch — which is 15,357,900 of them inside jsonbench's
`str_char`, because the decoder appends bytes and nothing else. Split into two
arms that duplicate five loads each, the byte arm is byte-identical to what
kanso#1221 shipped (checked: the only diff is `b`-prefixed labels) and
jsonbench's cost halves.

```
                      shared guards      split arms
  jsonbench            +0.604%            +0.286%
  encodebench          -7.379%            -8.351%
  oneshot              -1.518%            -2.112%
  widebench            -1.631%            -1.931%
  welfare                75.25              75.27
```

**The eleven, in this container, against kanso#1221 (62f66f30):**

```
encodebench  6,611,648,059 -> 6,059,516,328  -552,131,731  -8.351%
oneshot         36,606,462 ->    35,833,347      -773,115  -2.112%
widebench       63,809,599 ->    62,577,544    -1,232,055  -1.931%
basket          54,722,493 ->    54,396,964      -325,529  -0.595%
jsonbench    2,673,669,936 -> 2,681,323,231    +7,653,295  +0.286%
```

deepbench, escapebench, pendbench, indexbench, scanbench and digestbench are
byte-identical. Welfare **75.19 -> 75.27**.

**jsonbench's residual 7,653,295 is not the guards.** The byte arm's IR is the
one kanso#1221 shipped, so whatever is left is downstream of LLVM's layout and
inlining decisions on a bigger prelude — the effect kanso#1217 measured and
named. It is recorded rather than explained away, and it is 0.286%.

**kq collects most, because its printer is nothing but string appends.**
Against its pin at 8dc6ec9e, container-measured:

```
print_small  70,032,882 -> 61,913,801  -11.59%
print_big   711,281,956 -> 629,325,358 -11.52%
path_small   19,081,110 ->  18,792,192  -1.51%
path_big    198,938,968 -> 195,641,255  -1.66%
```

kanso at 8dc6ec9e and at 14fef781 give kq byte-identical rows and `.text`, so
none of the six merges between them reaches kq and the whole move belongs to
kanso#1221 and this change.

**What pins what.** `an_index_without_the_bang_reaches_its_twin.kso` reads a
list and a byte string in range and out, a map with INT keys and a string with
a multi-byte character. Three of four mutations turn it red: the container test
(`intmap 2` reads 0, a KMap header read as a KList), the bounds test (`xs[0]`
answers `false`) and the list-versus-byte branch (`xs[2]` answers 0). The
int-key tag test does not, masked the way kanso#1221's owned test was.

`an_in_place_append_takes_a_whole_string.kso` alternates a byte and a five-byte
string through one call site. Two of five mutations turn it red — the string's
length read (802 and 48,996 against 2402 and 192,596) and the in-place length
write (402 and 30,996). The capacity test is caught by valgrind and not by the
output, exactly as the byte arm's was. The owned test and the string tag test
are caught by neither, and for the reasons kanso#1221 already recorded: the
capacity test masks the first and the corpus contains no shape that reaches
the second.

The two routing decisions are pinned by the instruction vein rather than by a
fixture, and correctly so — they change what a program costs, not what it
answers. Delete either and `bench/instructions_golden.txt` goes red on a row
whose number is in this entry.

**Every counter that moved, and what it landed on.** The two twins' bodies sit
in the DECLARES prelude, so every program emits them whether or not it reaches
them: one define, three calls, eight branches and 99 lines apiece for the
decoder — `emitted_defines` 167, `emitted_calls` 1,805, `emitted_branches`
1,201, `emitted_lines` 11,934 — and across the ten beside it
`emitted_other_defines` 1,449, `emitted_other_calls` 14,502,
`emitted_other_branches` 8,659, `emitted_other_lines` 86,723. The compile
samples take the same prelude: `defines` 149, `calls` 182, `branches` 207,
`lines` 3,312, and the module sample `module_defines` 89, `module_calls` 770,
`module_branches` 402, `module_lines` 4,832.

`text` lands on **1,010,694**, +5,744 for the two twins. `work_jsonbench`
lands on **2,681,323,644**, and its 7,653,295 is the layout residual described
above rather than work the guards do.

`compile_instructions` lands on **41,490,353**, a FALL of 11,038 against
41,501,391, measured on CI because a container cannot count this row. Two
things in the commit are data the front end never reads — the prelude grew by
the index twin and the append's second arm, and `arg_is_str` left `codegen.rs`
when the string arm made it dead — and between them the compiler's own
sections shifted. Rounds, visits and allocations are byte-identical: nothing
in the front end changed. Three consecutive commits have now moved this row
+6,087, −416 and −11,038, and none of the three touched a pass.

## 2026-09-02 — the born test that only fed a counter, and the seed map at two slots

Two one-line changes to `src/runtime.c`, both in the same place: what an
in-place mutation costs before it writes anything.

### the born test

`k_b_push_mut` is the in-place list push, emitted where the linearity analysis
has proved the list uniquely owned. Its fast arm asked three questions: is this
value on the frontier of its buffer, is there room, and was the header born
inside this beat. The first two are four loads and two compares. The third,
`k_born_this_beat`, reads the beat depth, the block chain and the mark on top
of the beat stack, and off the head block it walks the chain to settle tenure
exactly.

Fail the third and the call went to `k_b_push_into_proven(lv, item, 1, 1)`.
Read what that does with `mutate=1` and `proven=1`: the born line at its head
is skipped for being proven, the frontier branch claims `l->items[l->len]`,
bumps `buf->used`, bumps `l->len`, and returns the same `lv` the fast arm
would have returned. Two paths, one behaviour, and the second one paid for a
call, a beat-stack lookup, and sometimes the walk.

So the born test was deciding which of two counters to increment.
`k_stat_push_mut_fast` and `k_stat_push_mut_slow` are the only things
downstream of it, and both are read only when `k_stats_on`. It sits inside
that guard now, and the counters mean exactly what they meant: all nine cost
goldens were byte-identical across this change taken alone, which is the
evidence that nothing else moved.

escapebench is where it shows. Its cost golden reads `push_mut_fast=3000`
against `push_mut_slow=1200000`: 1.2 million frontier writes a run, every one
of them going the long way round because the accumulator's header predates the
beat. That is 32,097,000 instructions, 26.7 apiece.

### the seed map

`k_map_lit` gave an empty literal one pair of room — `k_buf(2 * (n ? n : 1))`
— where the grow path it hands off to starts at four. Nothing writes `{}`
except to put into it, so the second put always grew, and jsonbench's decoder
writes a fresh `{}` per JSON object. A third of every map insert in the
benchmark was a reallocation: `put_mut_grow` 419,850 against
`put_mut_fast` 834,300.

An empty literal starts at the grow path's own floor now. A literal with keys
in it is a finished value and still gets exactly the room it needs.

### what both did

| benchmark   | before        | after         | delta        |          |
|-------------|--------------:|--------------:|-------------:|---------:|
| escapebench |   248,370,844 |   216,273,844 |  −32,097,000 | −12.923% |
| basket      |    54,397,377 |    47,410,168 |   −6,987,209 | −12.848% |
| digestbench |    98,969,254 |    89,615,426 |   −9,353,828 |  −9.451% |
| pendbench   |   749,618,914 |   734,101,805 |  −15,517,109 |  −2.070% |
| jsonbench   | 2,681,323,644 | 2,627,824,194 |  −53,499,450 |  −1.995% |
| oneshot     |    35,833,746 |    35,476,818 |     −356,928 |  −0.996% |
| widebench   |    62,577,957 |    62,243,248 |     −334,709 |  −0.535% |
| encodebench | 6,059,516,727 | 6,058,924,662 |     −592,065 |  −0.010% |
| scanbench   | 1,423,437,886 | 1,423,437,854 |          −32 |        — |
| deepbench   |   677,481,898 |   677,481,898 |            0 |        — |
| indexbench  |     5,242,731 |     5,242,731 |            0 |        — |

Split between them: the born test is the whole of escapebench, digestbench,
pendbench and widebench, and −18,096,450 of jsonbench. The seed map is the
rest of jsonbench (−35,403,000), the whole of oneshot, and it cost basket
28,669 instructions where it saved basket four allocations. deepbench and
indexbench hold to the byte and their `.text` does not move: neither writes a
proven in-place push nor an empty map literal, so neither function is in the
binary.

**Allocations.** jsonbench `allocs` 5,334,308 -> **4,999,958** (−6.27%),
`alloc_bytes` 268,048,208 -> **259,660,208**, `sh_buf` 143,306,400 ->
**134,918,400**, `put_mut_grow` 419,850 -> **85,500** with `put_mut_fast`
834,300 -> **1,168,650**. oneshot and encodebench move the same way on the
same counters (`put_mut_grow` 2,799 -> **570**), basket by four allocations.
No counter in any of the nine goldens moves the wrong way.

The lazy tier's vein sees it too. Six `.mem` goldens move, every one of them
by exactly one grow: `fused_tally` allocs 96 -> 93, `growing_map` 1,615 ->
1,614, `map_put` 102 -> 101, `readwrite_map` 310 -> 309,
`repeated_key_shape` 7,389 -> 7,388, `tally_shape` 26 -> 25, and in each the
`put_mut_grow` count falls by one while `put_mut_fast` rises by one. Those are
programs of a few dozen puts, so a single reallocation is the whole of what
the change can save them, and a single reallocation is what it saves.

**What it cost.** `text` rises 1,010,694 -> **1,011,350**, +80 bytes on the
four binaries that take both changes and +64 on the five that take one. An
inlined frontier write is longer than a call to something that does it, and a
literal that asks for eight slots writes a different immediate. That is the
trade the objective cannot see, and §35 of the compiler page is about.

**Welfare 75.27 -> 75.54**, ratcheted twice. The born test alone was 75.50,
the seed map took it to 75.52 — 0.02 for six per cent of jsonbench's
allocations, which is what a satiated term pays — and the map insert the last
0.02.

The pin is `bench/instructions_golden.txt`. A fixture cannot catch the born
test: the old code and the new code write the same bytes into the same slot
and return the same value, which is the whole finding. Restore it to the fast
arm's condition and nine rows in that vein go red on the numbers above.

### the map insert

`k_b_put_mut` was the other 3.70% of jsonbench, and its cost is not in the
work it does. `objdump` shows a 312-byte stack frame and six callee-saved
pushes on entry, paid by every call, grow or not, because the growth arm and
the sorted-view insert live in the same function. jsonbench makes 1,254,150 of
those calls a run at 77.6 instructions apiece.

A map with no sorted view built is the whole of what a fast arm needs.
`k_map_replace` answers on one branch when `m->sorted` is NULL, the view
insert is a no-op, and the write is two slots at the frontier. jsonbench's
`view_allocs` reads zero, so every one of its inserts is that case.
`k_b_put_mut_fast` in the DECLARES prelude does it: tag `K_MAP`, key an int or
a string, value not a failure, counting off, no view, frontier and room, then
two stores and a length. A built view, a full buffer, anything else at all:
the call.

Against the two changes above, that is jsonbench 2,627,824,194 ->
**2,556,080,244**, −71,743,950 or **−2.730%**; oneshot −478,293 or −1.348%;
encodebench −478,293 or −0.008%; basket +12,792, its whole cost. The other
seven benchmarks are byte-identical, and so are all nine cost goldens.

### all three, against b05ee1b2

| benchmark   | before        | after         | delta         |          |
|-------------|--------------:|--------------:|--------------:|---------:|
| escapebench |   248,370,844 |   216,273,844 |   −32,097,000 | −12.923% |
| basket      |    54,397,377 |    47,422,960 |    −6,974,417 | −12.821% |
| digestbench |    98,969,254 |    89,615,426 |    −9,353,828 |  −9.451% |
| jsonbench   | 2,681,323,644 | 2,556,080,244 | −125,243,400 |  −4.671% |
| oneshot     |    35,833,746 |    34,998,525 |      −835,221 |  −2.331% |
| pendbench   |   749,618,914 |   734,101,805 |   −15,517,109 |  −2.070% |
| widebench   |    62,577,957 |    62,243,248 |      −334,709 |  −0.535% |
| encodebench | 6,059,516,727 | 6,058,446,369 |    −1,070,358 |  −0.018% |
| scanbench   | 1,423,437,886 | 1,423,437,854 |           −32 |        — |
| deepbench   |   677,481,898 |   677,481,898 |             0 |        — |
| indexbench  |     5,242,731 |     5,242,731 |             0 |        — |

**Every counter that moved, and what it landed on.** The put twin's body sits
in the prelude, so every program emits it: `emitted_defines` **168**,
`emitted_calls` **1,806**, `emitted_branches` **1,207**, `emitted_lines`
**12,000** for the decoder, and across the ten beside it
`emitted_other_defines` **1,459**, `emitted_other_calls` **14,512**,
`emitted_other_branches` **8,719**, `emitted_other_lines` **87,385**. The
compile samples take the same prelude: `defines` **154**, `calls` **187**,
`branches` **237**, `lines` **3,642**, and the module sample
`module_defines` **90**, `module_calls` **771**, `module_branches` **408**,
`module_lines` **4,897**. `text` lands on **1,012,534**.

Every one of those is the price of a prelude that now carries five twins, and
every one is paid by programs that never reach them — the trade #1217
measured and the objective declines to weigh. `compile_instructions` lands on
**41,498,385**, a rise of 8,032 on 41,490,353. CI read 41,497,526 for the
first two changes alone, so the twin is 859 of it. Rounds, visits and
allocations hold across all three: the front end was not touched, and what
moved is the length of a string the compiler carries and never reads.

**The page's decode attribution is re-sat on top of this.** §07 read
1,783,922,550 emitted against 841,939,256 runtime and 47,808,090 libc, a
sitting taken before kanso#1222's index twin landed and so already a day
stale when it shipped. It now reads 1,791,588,501 / 792,404,497 / 43,795,809
of 2,627,823,781, which is 68.2% / 30.2% / 1.7% and **92.8 instructions per
input byte** against 94.5. The runtime's share is falling for two reasons at
once: the twins move its one-liners into the emitted code, and a guard that
did nothing came out. `decode.allocs` on compiler.html and index.html moves
5,334,308 -> 4,999,958 with it.

`k_b_put_mut` is 3.70% of jsonbench and the next thing to look at. It has no
dead guard to remove — `k_map_replace` returns on one branch while the map has
no view built, and jsonbench's `view_allocs` is zero — so what is left there
is the insert itself.

## 2026-09-02 — the in-place list push inlines

`k_b_push_mut` was the largest runtime symbol left in the decode after
kanso#1223, at 3.41% and 87,223,800 instructions over 1,459,800 calls. Its
fast path is thirteen instructions. `objdump` shows what the other forty-seven
are: six callee-saved pushes and `sub $0xa8,%rsp` on entry, paid whether the
call grows the list or not, because the growth arm and the buffer bookkeeping
share the function.

Removing the born-this-beat test in #1223 left a guard small enough to write
in the prelude: tag `K_LIST`, counting off, `buf->used == l->len`,
`l->len < k_buf_cap(buf)`. Then one sixteen-byte store into the frontier slot
and two lengths. `k_b_push_mut_fast` does that; a grow, a full buffer, a value
that is not a list, all take the call.

| benchmark   | before        | after         | delta        |          |
|-------------|--------------:|--------------:|-------------:|---------:|
| escapebench |   216,273,844 |   185,475,844 |  −30,798,000 | −14.240% |
| digestbench |    89,615,426 |    81,237,955 |   −8,377,471 |  −9.349% |
| basket      |    47,422,960 |    45,028,689 |   −2,394,271 |  −5.049% |
| pendbench   |   734,101,805 |   715,729,140 |  −18,372,665 |  −2.503% |
| jsonbench   | 2,556,080,244 | 2,533,005,144 |  −23,075,100 |  −0.903% |
| widebench   |    62,243,248 |    61,843,521 |     −399,727 |  −0.642% |
| oneshot     |    34,998,525 |    34,844,691 |     −153,834 |  −0.440% |
| encodebench | 6,058,446,369 | 6,057,833,454 |     −612,915 |  −0.010% |
| scanbench   | 1,423,437,854 | 1,423,437,774 |          −80 |        — |
| deepbench   |   677,481,898 |   677,481,898 |            0 |        — |
| indexbench  |     5,242,731 |     5,242,731 |            0 |        — |

escapebench is the shape this suits: 1,203,000 in-place pushes a run and
almost nothing else, so the frame it stopped paying is nearly the whole of
what it does. All nine cost goldens are byte-identical — the twin claims the
same slot the C claims, and the counters bail it to the C when anyone is
counting.

Four twins have now been added to the prelude in a day, and the trade is the
same every time. **Every counter that moved, and what it landed on:**
`emitted_defines` **169**, `emitted_calls` **1,808**, `emitted_branches`
**1,210**, `emitted_lines` **12,044**; `emitted_other_defines` **1,469**,
`emitted_other_calls` **14,532**, `emitted_other_branches` **8,749**,
`emitted_other_lines` **87,826**; the compile sample's `defines` **159**,
`calls` **197**, `branches` **252**, `lines` **3,862**, and the module
sample's `module_defines` **91**, `module_calls` **773**, `module_branches`
**411**, `module_lines` **4,940**. `text` lands on **1,016,742**.

Six programs emit a twin they cannot reach and pay its bytes; five reach one
and pay nothing for it. The objective weighs the first at zero, which §35 of
the compiler page is about, and the second at the numbers in the table.

`compile_instructions` lands on **41,495,096**, a FALL of 3,289 measured on
CI. The prelude grew and the row went down — the fourth reading in a day from
a change the front end never runs, and the fourth time the sign has not
followed the direction of the edit.

**Welfare 75.54 -> 75.73**, ratcheted. Four ratchets in a day: 75.27, 75.50,
75.52, 75.54, 75.73.

**The page's decode attribution moves again**, and this time the shape of it
does. §07 now reads 1,849,923,051 emitted against 639,250,897 runtime and
43,795,809 libc of 2,533,004,731 — **73.0% / 25.2% / 1.7%**, and **89.5
instructions per input byte** against 92.8. The runtime's share was 31.5% two
entries ago. `k_b_put_mut`, `k_b_push_mut` and `k_b_at` led its list at the
start of the day and none of the three is in it now; what is left at the top
is `k_b_find2` at 3.36%, `k_utf8_bad` at 3.28% and `k_b_to_float` at 2.51%,
which are algorithms rather than call overhead.

The pin is `bench/instructions_golden.txt`, and it is the right one: the twin
writes the same bytes the C writes into the same slot and returns the same
value, so no fixture can tell them apart. Route the site back to `push_mut`
and nine rows in that vein go red on the numbers above.

## 2026-09-02 — the sweep the page owed after four twins

The number-bearing surfaces are a checklist rather than a memory, and this is
what walking them found after #1221 through #1224.

**§08 ranked dragonbox on a profile that no longer exists.** It read: ryū
takes 3.8% of encode, and sits behind an append-and-copy pair at 19.5% and the
encode walker at 13.5%, so dragonbox's margin would recover about one per cent
and the idea is ranked rather than queued. Every figure in that sentence moved.
encodebench fell 13.6% across the four twins, and it fell by removing the
appends: `k_b_append_wide` is off the profile entirely now, under a tenth of a
per cent, where it was 5.92%. The denominator shrank and ryū did not, so
`render_ryu` is **7.71%** of 6,057,833,055 — the fifth largest entry, ahead of
`k_beat_rewind` and the libc memcpy. What it sits behind is `encode_onto` at
13.83%, and nothing else.

The conclusion survives and its reasoning did not, which is the failure mode
this checklist exists for. A reader checking the ranking would have found ryū
twice the share the page gave it, behind one thing rather than two, and no way
to tell whether the ordering had been re-thought or just left. The paragraph
now says which sitting it belongs to and what moved it.

**§07's follow-on paragraph had arithmetic that stopped connecting.** It
explains #1221 by naming the sitting before it — 1,729,183,050 emitted against
941,714,306 runtime — and the fall and rise that took it to the next one. §07
above has since been re-sat twice, so the numbers a reader would add up landed
on a sitting the page no longer showed. The paragraph now says so in its first
clause. Nothing in it was wrong; it had quietly stopped being about the
paragraph above it.

Two surfaces checked and left alone. `docs/numbers.html`'s 2,545,249,871 and
2,762,364,162 are a named historical episode about counters versus
instructions, past tense and still true. The decode board's ms/decode column
is a dated hand sitting on a quiet box, which is a release step.

The mechanical gates cannot reach either defect. `golden_prose` reads
`data-golden` tags and both figures were untagged prose; `page_drift` counts
log headings against page commits and the page had just moved. A profile share
is not a golden, so nothing in the tree can diff it — which is why the rule is
to walk the list rather than remember it.

## 2026-09-02 — the compile row moved 5,081 with nothing changed, and four entries above are wrong about why

This pull request changes `docs/compiler.html` and `design/compiler-log.md`.
CI read `compile_instructions=41,500,177` against a golden of 41,495,096.

The compiler's inputs are byte-identical between the two commits. `git diff`
is those two files; neither is compiled in, neither is reached by
`include_str!`, and `measured_on` confirmed the same rustc and the same
glibc. Every other vein in the same job matched to the digit: eleven work
rows, fourteen emitted counters, eleven text rows, `compile_allocs`,
`compile_memory`. One row moved, and nothing in the diff can have moved it.

**So the row's floor on this runner pool is at least 5,081.** The gate already
suspects this — it reads `scripts/gates/dispatch.sh` whenever the row moves
and says to re-run until the job lands on the recorded silicon — and kq spent
three pull requests in August learning the same thing about its own rows.
`measured_on` pins the toolchain, and the toolchain was never the whole host.

**This corrects four entries above, all filed today.** I recorded +6,087,
−416, +8,032 and −3,289 on this row as layout effects and wrote a causal
sentence around each: that the prelude's length moves where the bytes after it
land, and that the sign does not follow the direction of the edit. Two of
those readings are smaller than the 5,081 a no-op produced, and the other two
are the same size. The layout mechanism is real — a longer string in the
binary does move what follows it — but none of those four measurements
separates it from the runner, and I wrote as though they did. The honest form
of all four is "the row moved, inside a band the pool can produce on its own".

The pattern to notice is that the prose got more confident as the readings
piled up. By the fourth I was writing that four consecutive commits had moved
the row in an unpredictable direction and that this was what a layout effect
looks like — a story that explains the data and was never tested against the
null. The test cost one docs-only pull request and it was available all day.

**What would fix it.** Not a tolerance: a band is a guess that stays green
through the change it was written to catch. Record the dispatch block beside
the row the way `kq/bench/instructions_golden.txt` does, so a move gets asked
which silicon counted it before anybody explains it. That is a real piece of
work and it is not this pull request's; it is filed here so the next reading
of this row has the question in front of it.

**The re-run closed it.** The next job on this branch, with the golden
unchanged at 41,495,096 and nothing in the compiler touched, read 41,495,096
and went green. So the row is 41,495,096 / 41,500,177 / 41,495,096 across
three consecutive CI runs of the same compiler — bracketed, not a one-off.
The spread is 5,081 and it belongs to the pool rather than to any diff.

Writing that down matters more than the finding. The entry above corrects
four readings I took as evidence on a single measurement each; leaving this
one at a single measurement would have repeated the error inside the
correction.

## 2026-09-03 — the rewind's empty test paid a frame, and it is a third of a beat loop

Clay asked whether the fused chain operators had been swept in. They had not.
The day's four twins were driven by the eleven benchmarks' profiles, and a
fused adapter chain — `map`, `select`, `sum` deforested into one flat loop —
is not well represented in those. So I built one: 200,000 elements through
`list/map . list/select . list/sum`, and profiled it.

`k_beat_rewind` was 32.92% of it. Not the loop body, not a builtin — the
per-iteration rewind that reclaims the iteration's garbage.

That was not a property of the fixture. On escapebench the same function is
**43.66%** of the program: 80,970,118 of 185,475,445 instructions across
1,206,001 rewinds, 67 instructions each. On basket, 16.21%. The one function
was larger than any seam the four twins reached.

**Where the 67 went.** The disassembly says it in the first ten instructions:
six callee-saved pushes and a stack sub, before any work. The frame is there
for four loops — the buffer shelf's flush, and the chunk, view and permanent
registries — none of which runs on any of those 1,206,001 rewinds. Then
`k_buf_flush` unconditionally memsets the twelve-pointer shelf, six `movaps`
clearing a shelf that was already clear. What the common case actually needs
is three stores: `k_arena`, `k_arena_left`, `k_seek_str`.

**The change.** Two pieces, both idiomatic here. The shelf gets a dirty flag,
so its memset becomes one test for a program that never donates a buffer. And
`k_beat_rewind` splits: an inline empty test at the four call sites, with the
frame and the four loops left behind in `k_beat_rewind_slow`. That is the same
split `k_permreg_flush`/`_held` and `k_viewreg_migrate`/`_held` already make
one level down; it is worth more here because a beat loop rewinds once per
iteration where those run once per pop. `k_beat_iter` is now a leaf with no
frame at all, ~39 instructions on the fast path against 74 before.

| benchmark | before | after | |
|---|---|---|---|
| escapebench | 185,475,844 | 143,403,373 | −22.684% |
| basket | 45,028,689 | 41,211,873 | −8.476% |
| encodebench | 6,057,833,454 | 5,886,750,857 | −2.824% |
| oneshot | 34,844,691 | 34,417,154 | −1.227% |
| deepbench | 677,481,898 | 677,017,353 | −0.069% |
| indexbench | 5,242,731 | 5,242,087 | −0.012% |
| scanbench | 1,423,437,774 | 1,423,437,268 | −0.000% |
| pendbench | 715,729,140 | 715,731,365 | +0.000% |
| jsonbench | 2,533,005,144 | 2,533,091,327 | +0.003% |
| digestbench | 81,237,955 | 81,252,347 | +0.018% |
| widebench | 61,843,521 | 61,857,884 | +0.023% |
| **all eleven** | **11,821,160,841** | **11,603,412,888** | **−1.842%** |

The three small rises are programs with few beat iterations paying the inline
test at their pops where the old code paid a call. 86,183 instructions on a
2.5-billion-instruction decode is the largest of them. These are container
readings; CI supplies the rows that land in the golden.

`.text` rises on every row, 432 to 816 bytes — four copies of the test. The
header of `bench/text_golden.txt` says which benchmark paid most and which
gained most, and they are the same one.

**No counter moves, and the corpus was asked how much of that it can see.**
The three flush loops and the block retire are the only code in the rewind
that touches a statistic, so the fast path being taken exactly when all four
would do nothing means every cost golden stays byte-identical. All nine are.
But "byte-identical counters prove equivalence" is a claim about what the
corpus can observe, so I dropped each of the six conditions in turn and ran
the nine counter gates and the golden suite against each.

Three are pinned. Without the shelf's dirty test five counter gates
**segfault** — a shelf entry outliving its arena region hands a freed pointer
to the next grower, which is exactly the failure the flag exists to prevent.
Without the chunk registry's test, encode's counters move. Without the block
test, encode's and scan's move and four goldens fail.

Three are not: `k_chunkreg_spill`, `k_viewreg_n`, and the permanent registry's.
Dropping any of them leaves every gate and every golden green. The reason is
one fact, and I found it by instrumenting the rewind rather than guessing:
**on every program in the corpus, a rewind that finds a registry non-empty is
also a rewind that has moved on from its mark's block**, so the block test
reaches the slow path first and the registry tests never decide anything.
Counted over the benchmarks: `chunkreg_spill` is non-zero at no rewind at all;
escapebench has 3,000 rewinds with a non-empty permanent registry and the same
3,000 have changed block; encodebench has 400 with a non-empty chunk registry.

I wrote a fixture to isolate the view registry — a map born inside a beat and
asked for its sorted view — and it registered 500 views at 500 rewinds and
**still** did not catch the mutation, because those 500 rewinds had also taken
a new block. A fixture that cannot fail is worse than none, so it is not in
this pull request. The conditions stay: that the two travel together is a
property of the programs in the corpus, not of the code, and the next program
need not oblige.

**What is left.** The naming is the finding. `k_beat_rewind` did not show up in
any earlier profile sweep because the sweeps were reading the benchmarks whose
profiles the twins were chasing, and the beat machinery is not a builtin. The
question that found it was Clay's, about a construct the benchmark set does
not cover well. That is an argument for the fused chain having a benchmark of
its own, which it does not have and which the eleven do not substitute for.

**The fixture that started it, measured after.** The 200,000-element
`map`/`select`/`sum` chain goes 38,883,703 → 32,083,715, **−17.49%**, and
`k_beat_rewind` is off its profile altogether. What is left of the rewind is
`k_beat_iter` at 8,000,000 — 40 instructions an iteration where rewind and
iter together were 74. It is still 24.93% of that program.

So the seam is not finished, and the shape of what remains is visible: of
those 40, about twenty are the six condition tests and three are the stores
they guard. A single summary word per depth — maintained at the four
registration sites, read once here — would collapse six loads and six
branches into one. That is a second change and it waits for this one to land.
It is written down here rather than built now because the branch rule is one
open pull request at a time, and because the number above is what makes the
case for it.

**Two corrections to the paragraph above, both from building it rather than
reasoning about it.** The summary word collapses *four* conditions into one,
not six: the buffer shelf's flag is global rather than per depth, and the
block test is about arena state that no registration site can summarise. So
the fast path goes from six tests to three, not to one.

And the prototype was built and measured rather than left as a plan. A single
`k_beat_enc[depth]`, set at the four registration sites and cleared where the
slow rewind empties all three registries together, with all nine counter gates
green: escapebench 143,403,373 → 130,167,353, **−9.23%**; the fused chain
32,083,715 → 30,483,715, **−4.99%**; basket 41,211,873 → 40,299,752, −2.21%.
It is a second pull request, held behind this one by the branch rule, and it
is worth having.

**The ratchet caught the move, which is what it is for.** `k_seek_str = NULL`
went from one site to two — the slow rewind and the inline fast path — at
different indentations, and the cursor mutation's own guard reported that its
target had moved rather than silently deleting one of the two. Deleting one
would have left the running path resetting the cursor and the mutation would
have gone green while proving nothing. It now removes both, and reddens the
beat differential at 16 of 96 layout pairs, the figure its comment already
records.

**CI's rows, and they agree with the container.** The instruction vein is
regenerated from the runner, where the container readings above were a
direction rather than a pin. Every row lands within about four hundred
instructions of what the container said, which is the offset those two hosts
have carried all along.

| benchmark | before | after | |
|---|---|---|---|
| escapebench | 185,475,844 | 143,403,772 | −22.6833% |
| basket | 45,028,689 | 41,212,286 | −8.4755% |
| encodebench | 6,057,833,454 | 5,886,751,256 | −2.8241% |
| oneshot | 34,844,691 | 34,417,553 | −1.2258% |
| deepbench | 677,481,898 | 677,021,033 | −0.0680% |
| indexbench | 5,242,731 | 5,242,500 | −0.0044% |
| scanbench | 1,423,437,774 | 1,423,437,681 | −0.0000% |
| pendbench | 715,729,140 | 715,731,751 | +2,611 |
| jsonbench | 2,533,005,144 | 2,533,091,740 | +86,596 |
| digestbench | 81,237,955 | 81,252,746 | +14,791 |
| widebench | 61,843,521 | 61,858,297 | +14,776 |
| all eleven | 11,821,160,841 | 11,603,420,615 | −1.8420% |

Four rows worsen and each is stated here as required: **jsonbench** rises
**86,596**, **digestbench** rises **14,791**, **widebench** rises **14,776**,
**pendbench** rises **2,611**. All four are programs with few beat iterations
— 151, 56, 40 and 134 — that now pay the inline empty test at their pops
where they used to pay a call, and none of the four is a tenth of a per cent.
They are the price of escapebench's 42 million and encodebench's 171 million.

**compile_instructions** reads **41,500,974** against a golden of 41,495,096,
a rise of **5,878** with nothing in the front end touched by this change. The
entry above it in this log established that this row moves 5,081 on a
docs-only pull request with byte-identical compiler inputs, across three
consecutive runs of the same compiler. 5,878 is that band. It is regenerated
rather than explained, and the reason it is not explained is the finding
recorded yesterday: nothing here separates a layout effect from the runner
pool, and writing a causal sentence around a number this size is the error
that entry corrects.

**The five worsened counters, named as the gate spells them.**
`work_jsonbench` lands on **2,533,091,740**, `work_digestbench` on
**81,252,746**, `work_widebench` on **61,858,297**, `work_pendbench` on
**715,731,751**. Those four are the programs with 151, 56, 40 and 134 beat
iterations: too few for the fast path to repay, so they pay the inline empty
test at their pops where the old code paid a call, and the largest of the four
is 0.018% of its program. `compile_instructions` lands on **41,500,974**,
inside the pool band the entry above measured.

`text` lands on **1,023,750** against 1,016,742 — the eleven binaries together
grow **7,008 bytes**, 432 to 816 each, which is four copies of a twenty-
instruction test at the four rewind call sites. That is the trade this change
is: seven kilobytes of machine code for 217,740,226 instructions.

## 2026-09-03, LATER — the compile row's move is glibc's allocator, and that corrects the task filed for it

The entry above pinned `compile_instructions` at 41,500,974 from CI. The next
run of the **same commit's front end** read **41,495,850**. Five readings now
exist for a compiler nothing has touched:

    41,495,096   41,500,177   41,495,096   41,500,974   41,495,850

Two clusters about 5,228 apart, each internally within 800. Yesterday's entry
called this "the pool" and declined to explain it, and filed a task to record
the dispatch block beside the row the way kq does, so a move could be asked
which silicon counted it.

**That task's premise is wrong, and this is the measurement that says so.**
CI prints the profile on every run, so the two runs can be compared function
by function:

| symbol | 41,500,974 | 41,495,850 | delta |
|---|---|---|---|
| `kanso::infer::eval_expr'2` | 1,616,100 | 1,616,100 | 0 |
| `kanso::check::check_merged` | 1,598,577 | 1,598,577 | 0 |
| `_int_malloc` | 1,554,268 | 1,551,398 | **−2,870** |
| `_int_free` | 1,516,161 | 1,516,378 | **+217** |
| `__memcmp_avx2_movbe` | 1,346,853 | 1,345,486 | **−1,367** |
| `HashMap::insert` | 1,302,885 | 1,302,885 | 0 |
| `kanso::infer::infer` | 1,234,092 | 1,234,092 | 0 |
| `malloc` | 1,113,407 | 1,113,407 | 0 |
| `kanso::infer::eval_expr` | 876,900 | 876,900 | 0 |
| `kanso::lexer::lex_line` | 857,892 | 857,892 | 0 |
| `free` | 711,340 | 711,340 | 0 |
| `kanso::parser::parse` | 591,666 | 591,666 | 0 |
| `kanso::mentions_in_expr'2` | 533,848 | 533,848 | 0 |

**Every symbol in the compiler is identical to the instruction. Three glibc
symbols move and nothing else does.** And both runs took the same dispatch —
`__memcmp_avx2_movbe` on each — so recording the dispatch block would have
found the two runs indistinguishable and explained nothing. The silicon
hypothesis is dead, and it is dead by the same test that killed the layout
one: comparing against the null instead of fitting a story to a number.

What is left is glibc's allocator. `_int_malloc`'s bin walks depend on the
heap's starting layout, and the allocation *sizes* cannot be what differs
because the compiler's own counts are byte-identical. The most likely
remaining cause is that the two runs ran binaries whose data and bss differ
slightly, moving the initial break — which changes malloc's work without
changing a single instruction the compiler executes.

**The row is red on main too.** `bench/compile_instructions_golden.txt` holds
41,495,096 and this runner reads 41,495,850, so a pull request that changed
nothing at all would fail this gate on this runner. It is not this change's
regression, and this branch does not carry a bump for it: the golden is left
at main's value.

**What would actually fix it**, and it is a decision rather than a repair:
the row should count the instructions attributed to the kanso binary rather
than the process. That is what the gate's own header says it measures — "what
the FRONT END costs to run" — and it is the part that is deterministic. On
this container three consecutive runs of the box give 41,904,811 exactly, and
the per-object split is 33,586,490 in the compiler against 7,982,541 in libc;
it is the second number that moves on the runner. Changing what a published
counter measures is Clay's call, not a session's, and it is filed as one
rather than done here.

**A sixth reading, and it repeats a previous one exactly.** The run after the
entry above read **41,500,974** — the same value, to the instruction, that a
run two heads earlier produced. Six readings of an untouched front end now
give four distinct values, two of them seen twice:

    41,495,096  ×2      41,495,850  ×1
    41,500,177  ×1      41,500,974  ×2

That is not continuous noise. Noise does not land on the same eight-digit
number twice in six tries. It is a small set of discrete environments, each
deterministic within itself — which is also what the container says, where
three consecutive runs of the same box give 41,904,811 every time.

So the row is deterministic per environment and the pool holds several. The
comparison in the entry above rules out the obvious discriminator: the two
runs it examined took the same `__memcmp_avx2_movbe` dispatch and differed
only inside glibc's allocator. Whatever separates the environments, it is
finer than the dispatch block and it does not touch a single instruction the
compiler executes — every `kanso::` symbol was identical across the pair.

This sharpens the question filed for Clay rather than changing it. A row that
is deterministic per environment and varies across a pool of them can be
pinned two ways: record the environment, or stop counting the part that
varies. The first needs a discriminator nobody has found yet — the dispatch
block is not it. The second is available now and is what the gate's header
already claims to measure.

**And it has already failed on main.** The claim above — that a pull request
changing nothing would fail this gate on some runners — was an inference from
the readings. It does not need to be: main's own `ci` run at **9541196f**, on
2026-09-02, failed with `compile instructions disagrees with its golden` and
every other row in that vein green. That commit is a merge that had just
regenerated the row, so the golden it was checked against was its own.

So the gate has already gone red on merged history, on a commit whose author
had just pinned the number it was checked against. Whatever this row is
measuring, it is not something a branch can be held responsible for, and the
seventh reading on this branch — 41,500,974 again — is the fourth head in a
row to say so.

That is the whole of the case for leaving the golden at main's value and
sending the question on rather than pinning a number that will be wrong on
the next runner.

## 2026-09-03, LATER STILL — the silicon hypothesis was right and I killed it on the wrong evidence

The gate now prints the binary that counted the row. It answered a different
question than it was aimed at, and the answer is that two entries above are
wrong.

The two CI runs compared earlier ran on **different CPUs**. The gate has been
printing the dispatch block all along and both blocks were in the job logs;
they differ:

| field | run at 41,500,974 | run at 41,495,850 |
|---|---|---|
| `features[0x2].cpuid[0x0]` | 0xa10f11 | 0xa00f11 |
| `features[0x5].cpuid[0x1]` | 0x30100015 | 0x30000015 |
| `level2_cache_size` | 0x100000 | 0x80000 |
| `rep_movsb_stop_threshold` | 0x100000 | 0x80000 |

**What I did wrong.** I compared which `memcmp` implementation glibc had
selected — `__memcmp_avx2_movbe` on both — and concluded the two runs were on
indistinguishable silicon. That is the wrong field. The selected function is
one output of the dispatch; the CPUID word and the cache-derived tunables are
others, and those differ. I then wrote that "the silicon hypothesis is dead,
killed by the same test that killed the layout one" — which is the same error
the layout entry corrects, committed in the act of claiming not to.

The pattern is now three for three: a number moved, I reached for a mechanism,
and the mechanism was asserted from evidence that did not separate it from the
alternative. Twice the mechanism was wrong. The one thing that has worked every
time is comparing two runs field by field and reading what actually differs.

**What follows.** Task #244 was right and its closure was not. Recording the
dispatch block beside the row is the fix, exactly as `kq/bench/instructions_golden.txt`
already does and for the reason kq#86 already gave. The row is deterministic
per CPU and the pool holds at least two; pinning it per recorded silicon is
the shape that works.

That also means the two invasive options are off the table, and Clay's
instinct to keep libc counted survives intact: libc's cost stays in the row
because it is a real cost of how this compiler allocates, and the row stops
lying because it says which chip counted it.

**What is NOT claimed here.** How a different L2 size produces 2,870 fewer
instructions inside `_int_malloc` is not established. `_int_malloc` does not
call memcpy, and the mechanism could run through heap addresses, alignment,
or something else entirely. What is observed is that the environments differ
in a way the gate already records, and that is enough to attribute the row
without inventing the chain. Inventing the chain is what went wrong twice.

**A second cause is live and I had not checked it either.** Before pinning a
row per chip, one thing had to be ruled out and was not: `src/runtime.c` is
`include_str!`'d into the compiler. Every commit on this branch changed it, so
every head built a different compiler binary — different bytes, different
place for the heap to start — without altering one instruction the front end
executes when it checks a library.

So two candidate causes are live at once, the silicon and the binary, and the
readings so far cannot separate them. Four values, two identified chips, and
several different binaries across the heads that produced them. Pinning per
chip would be building on the same kind of half-checked story that has now
been wrong twice, so it waits.

What is added instead is one greppable line per run — cpu, binary sha, row —
so three runs settle it: same cpu and same sha with different rows means it is
neither; same cpu with the sha tracking the row means it is the binary; the
same sha on two cpus tracking the row means it is the silicon. That is the
measurement the last two entries should have started from.
