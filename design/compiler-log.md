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

## a transient map's view has an owner now, and it is the rewind

The oldest open memory thread in this log closes. Three entries diagnosed it and
the last one got the shape right: the problem was never that the view is built,
nor that it is read twice, nor where it lives. It is that a malloc'd view had no
owner, because the arena has no notion of an object dying.

Re-measured before touching anything, because a day-old number is a claim:

    200,000 transient maps    view_allocs=200000    view_frees=0

Total. Every view ever built for a map that died was still held at exit.

THE OWNER IS THE REWIND. A map header born inside a beat dies when the beat
rewinds, and that is the one moment anything knows its view became garbage. So
`k_viewreg` holds the headers registered at each beat depth, flushed in
`k_beat_rewind` and migrated at a pop that keeps its region — the shape
`k_chunkreg` already uses for string chunks, because the two leaks are the
same leak.

IT DOES NOT COPY THE FIXED BANK, and the corpus is why. The first version
mirrored `k_chunkreg` down to its 256 slots and overflow counter, and CI came
back with `view_frees=0 -> 256` on the oneshot benchmark — exactly the cap,
which is what a cap looks like when it is doing the wrong thing. The excess
silently went back to leaking, which is the behaviour this change exists to
remove. Growing the registry instead turned 256 into 2,761: the fixed bank was
dropping nine of every ten views it was supposed to own. A cap that quietly
stops working is worse than no cap.

HEADERS ARE REGISTERED, NOT VIEWS, and that choice does two jobs. A view that
outgrows its buffer is replaced by the grow path, and registering the header
follows the replacement with no fixup. And freeing through the header nulls it,
so a header registered twice — built, invalidated, rebuilt — frees once. No flag
is needed on `KMap`, which matters: the comment above `k_view_alloc` records
that a decode allocates 828,000 headers and builds no view, so a field there
would be paid for by every map to serve the few that are read.

    200,000 transient maps    view_allocs=200000    view_frees=200000

The answer the program prints is unchanged, all 45 test binaries pass, and the
repair path now frees the view it was dropping on the floor.

WHAT IT DOES NOT TOUCH, said plainly: a map built outside any beat. There is no
rewind, so there is no moment to free at, and `k_viewreg_add` returns without
registering when the depth is zero. kq's decode is mostly that shape — its
views are bounded by document size and released at exit, which is what the
earlier entries measured as 132 KB on the 188 KB fixture. Four of them do sit
inside a beat and are now freed; kq's decode golden moves `view_frees` 0 -> 4
and wants regenerating when its pin advances.

WELFARE CANNOT SEE THIS, and that is a finding rather than a complaint. The
index reads 75.68 either side and no cost golden moved, because its two memory
terms are `arena_peak_bytes` and `bytes_peak` and a view is in neither — the
transient-map fixture reports `bytes_peak=0` while allocating twenty thousand
views. The objective is blind to malloc'd memory that is not a string chunk.
`view_allocs - view_frees` is exactly the live count and the comment at the top
of runtime.c already says so, so the term is available; wiring it in moves the
floor and belongs in its own change.

THE ONESHOT GOLDEN MOVES, and it is the number the earlier entries chased:
`view_frees` 0 -> 2,761 on the 188 KB fixture, which is the 132 KB they
measured as leaked. The trend gate prices it as an improvement and welfare is
unmoved for the reason above.

GOLDEN: tests/golden/mem/a_transient_maps_view_is_freed, watched red by
disabling the registration — `view_frees=0` against `20000` with every other
counter in the file identical, which is the narrowest a mem golden gets.

## the spent licence, removed on purpose rather than left to be inert

#745's `sibling-goldens-move` is on main, which is the shape #724 left and
#736 was written to make harmless. It is harmless: it names a merged branch,
`goldens-move-licenses.sh` returns 1 for anything else, and
`a_file_left_behind_licenses_nobody` pins exactly that.

It is deleted anyway. A rule that fails safe is not a reason to leave litter,
and a reader who finds a licence on main has to go and check which of the two
it is before they can trust the gate. The self-expiry is the floor, not the
standard.

## the memory counter measured less than its own comment claimed

#745 removed a leak unbounded in a loop's iteration count and welfare did not
move a hundredth of a point. That is the finding, not the fix: the objective
claims to weigh run memory and could not see a whole class of it.

The counter's own documentation was already right —

    Malloc-backed builder storage currently live, and its high-water mark.
    Arena storage rewinds away; these bytes only leave through free(), so a
    peak that scales with iteration count is a leak by definition.

— and a map's sorted view is malloc-backed storage that only leaves through
`free()`. It was simply never counted. So the definition covered views and the
implementation did not, which is the worst way for a measurement to be wrong:
the gap reads as a zero rather than as a gap.

HOW WIDE, measured across the corpus. Eleven fixtures reported exactly zero
held bytes while holding some:

    oneshot        0 ->  311,728     growing_map    0 ->  49,120
    encode   407,788 ->  719,516     fused_tally    0 ->   7,744
    basket         0 ->   71,136     repeated_key   0 ->   6,112

The oneshot figure is the one three earlier entries chased by hand. It was
sitting in a counter reading zero the whole time.

THE COUNTER IS RENAMED, and that is the substance rather than a tidy-up.
`bytes_peak` meant builder bytes; `held_peak_bytes` means every malloc the
runtime holds. Keeping the old name would leave a number in the golden history
that means one thing before this commit and another after, which is precisely
the silent drift the cost goldens exist to catch — and every consumer
(welfare's `peak_of`, the trend gate's direction table, two book panels) now
has to opt in by name rather than inherit a wider meaning without noticing.
It also makes the trend gate's reading truthful rather than lucky: one counter
retires, another arrives, and the gate sees both sides instead of a phantom
regression.

RE-BASELINED SO IT BANKS NOTHING, the same method as #729 and #741. Each
affected baseline is scaled by exactly the factor its measurement grew —
oneshot x1.042470, basket x1.033920, encode x1.067736 — so every ratio is
restored to the digit (encode +2357.0%, oneshot -4.9%, basket +8150.0%) and
welfare reads 75.68 either side. A wider measurement judged against a narrower
baseline would penalise every future change for a reference that measured less.
The assumption is stated: the baseline runs held view memory in the same
proportion, which is reasonable for the same programs and is not verifiable.

EVERY COUNTER THAT MOVED, by name, because the gate asks which fixture and not
only which counter. Each is the same move: a name retiring at its old value and
a wider name arriving at the value that was always true.

    a_transient_maps_view_is_freed_held_peak_bytes
    an_accumulator_loop_reclaims_its_garbage_held_peak_bytes
    append_in_place_held_peak_bytes
    basket_held_peak_bytes
    beat_builder_held_peak_bytes
    beat_cycle_held_peak_bytes
    builder_reclaim_held_peak_bytes
    builder_transient_held_peak_bytes
    encode_held_peak_bytes
    fresh_builder_held_peak_bytes
    fresh_cycle_held_peak_bytes
    fused_tally_held_peak_bytes
    growing_map_held_peak_bytes
    map_put_held_peak_bytes
    oneshot_held_peak_bytes
    readwrite_map_held_peak_bytes
    repeated_key_shape_held_peak_bytes
    stream_write_held_peak_bytes
    tally_shape_held_peak_bytes

The fixture from #745 now says it best in one number. Twenty thousand views
allocated, `held_peak_bytes=80` — one view ever held at a time.

## the second spent licence, removed the same way

#747's escape file, deleted for the reason #746 states: it names a merged
branch and licenses nobody, and a reader who finds one on main should not have
to check which kind it is.

Two of these in two changes says the shape is recurring rather than
exceptional. A licence is needed whenever a compiler change moves a sibling's
counters at all, which is common, so the tidy-up is now part of the change
rather than a thing to notice afterwards.

## the chart already extends itself, and the rename nearly took a line off it

Two findings about #116, and the second is a defect I introduced an hour before
finding it.

THE TASK'S PREMISE IS STALE. It records that CI redraws the chart but "gains no
new points there", because `/usr/bin/time -l` reports no instructions-retired
line inside a GitHub macOS runner. The series has since moved to deterministic
counters and nobody updated the task. Measured on the perf-history branch:

    498 rows
    allocs                    498/498
    compile_visits            498/498
    cpu                         0/498
    instructions                0/498
    newest 50 rows: all four dimensions, 50/50

The two rows at the end are this session's own commits. CI extends the chart on
every merge, on an ordinary runner, and the expensive resolution the task
preferred — an hour of builds replaying 474 commits — buys nothing that exists
to be bought.

What no backfill could have delivered anyway: `arena_peak_bytes` first appears
in the source on 2026-07-27 and `allocs` on 2026-07-18. A replay cannot recover
a number the compiler never computed, so a complete four-dimension series from
the first commit was never available by any route. The series starts where the
counters start, and that is the whole of it.

THE DEFECT, mine. #747 renamed `bytes_peak` to `held_peak_bytes` and
`scripts/perf_record.kso` still watched the old name, so the row it wrote
simply had no memory field:

    bytes_peak       360/498
    held_peak_bytes    0/498

Nothing went red. The watcher selected the lines whose names it recognised and
a name matching nothing dropped out in silence, which is the same shape as
every other blindness in this log — a gap that reads as an absence rather than
as an error.

So the completeness check is now separate from the extraction and refuses:

    perf_record watches a counter the compiler no longer emits, so the row it
    writes would quietly lose a dimension and the chart would plot a shorter
    line without anything going red.
      not emitted: bytes_peak bytes_peak bytes_peak

Watched by pointing it back at the retired name: exit 2, three groups named.
With the current name the row carries 34 fields and the memory dimension is
back — `oneshot_held_peak_bytes` 311,728, which is the figure #747 surfaced.

## a sibling change is not done when it merges, and forcing a rerun proved it

Three CI runs were forced by hand this sitting — an empty commit on one branch,
a rebase and another empty commit on a second. Clay's read is right: needing to
force one means the process was wrong, not the runner.

WHAT ACTUALLY HAPPENED. Kanso's gating `kq specs` check clones kq's MAIN. A
kanso change that moves a kq counter therefore runs a five-step loop: the kanso
PR opens and the kq gate fails, a coordinated kq branch and a licence make it
pass, kanso merges, kq's pin advances, kq merges. Between step three and step
five the two mains disagree about the counter's name, and every PR opened in
that window fails a gate for a reason that has nothing to do with it. Two were
opened in that window and both needed forcing.

SO THE RULE, which is the existing no-fire-and-forget rule with its endpoint
moved: a change that moves a sibling's counters is finished when the SIBLING's
pin bump merges, not when kanso's PR does. Nothing unrelated opens in between.

The sharper version of the same mistake: the branch that retired the spent
licence carries an entry saying the tidy-up "is now part of the change rather
than a thing to notice afterwards", and it was opened as a separate PR
regardless — in the window, needing a forced rerun. The lesson was written and
broken in one sitting, which is the strongest argument for writing it here
rather than remembering it.

## kanso gates on kq's correctness, and reports kq's stored numbers

Three forced CI reruns in one sitting, all for the same reason and none of them
a real failure. Clay named the shape twice: first that forcing a rerun means the
process is wrong, then that my answer to it was over-complicated —

    why does it matter if anything is "stale". you're not doing a "diff",
    you're just doing absolute performance metrics.

That is the correction. The counters are absolute facts about the code and
nothing about them goes stale. What goes stale is kq's stored EXPECTATION, and
not one of the three failures was "kq got slower" — all three were "a counter
changed name". A golden diff conflates which counters exist with what they
measure, and a rename moves the first while saying nothing about the second.

THE SPLIT IS BY WHAT A CHECK COMPARES ITSELF TO, not by what it measures, and
that distinction does the whole job:

  computed in the same run, cannot be stale, gates everywhere
    kq's unit tests
    its twelve fixture goldens against jq
    its scale gate — every counter linear in the input, which is a
    PERFORMANCE property with no stored expectation at all

  compared against a committed file, true only for the pinned compiler
    the two cost goldens
    the published-numbers stamp

kanso now runs the suite with `KQ_STORED=report`, which prints the second group
and gates on the first. My earlier proposal — build kq twice per PR and diff
the two — bought the same property for an extra build, and is dropped.

WHY THE FIRST GROUP IS THE RIGHT GATE, from the incident that created it: #110
recorded that carry_dedup rose from 1 to 2,721 in kq and nothing caught it
BECAUSE KQ COULD NOT BUILD. The gate's value was never the stored numbers. It
was that a real program still compiles and still answers what jq answers.

THE NUMBERS ARE NOT UNWATCHED, which is the failure mode this project has hit
before under the name "recorded as harmless". kq pins its compiler precisely so
it controls when it absorbs a change, and its own CI gates those numbers at the
pin bump — where the compiler that produced them is the compiler being judged,
so they cannot be stale by construction. The upstream report is a preview; the
pin bump is the gate.

Verified in both directions before shipping: a doctored cost golden exits 1 by
default and 0 under report, and a broken FIXTURE golden exits 1 under report
too, which is the assertion the whole split rests on.

## the numbers land on the run page, which is the whole of technique three

Clay asked how CI-updated artifacts actually work and then ruled on the answer:
"#3 and #4 are the only sensible ones to use."

The four, and why two survive:

  1  COMMIT BACK TO THE PR BRANCH — rejected. A GITHUB_TOKEN push does not
     trigger further workflows, fork PRs get a read-only token, it races the
     author's own pushes, and every diff carries a regenerated blob.
  2  upload-artifact — rejected. The URL is authenticated and zipped, so it
     cannot be hot-linked or embedded. Fine to download, useless to see.
  3  $GITHUB_STEP_SUMMARY — kept. Markdown on the run page, tables and mermaid
     rendered natively, no hosting, no commits, no token.
  4  DATA ON A SIDE BRANCH, SERVED THROUGH PAGES — kept. Appended on push to
     main rather than on a PR, which sidesteps the fork-token problem
     entirely; Pages serves correct content types where raw.githubusercontent
     hands SVG back as text/plain and nothing renders.

I had also proposed a sticky PR comment and it was the weakest of the four:
identical markdown to the step summary, with `pull-requests: write`, an
upsert-by-marker loop that is a known source of duplicate-comment bugs, and no
working token on a fork. Dropped without being built.

THE TWO KEPT ONES DO NOT OVERLAP, which is why both. Three is per-run and
ephemeral — what did THIS change do. Four is durable and cumulative — what has
happened over time. That is exactly the existing split between the trend gate
and the chart, and four was already built: history.jsonl appended on merge, the
compiler page served by Pages.

SO ONLY THREE WAS MISSING. Welfare's breakdown, the trend gate's per-counter
listing and kq's suite all printed into a job log where reading them means
opening the run and scrolling. They now also write to the step summary, which
costs a redirect per script and no new mechanism.

The shape preserves the verdict, which is the part worth checking rather than
assuming: run the script with `set +e`, capture the exit code, emit the summary,
then `exit $verdict`. Verified in both directions before shipping — a non-zero
status propagates and a zero one does not become a failure.

This is also what stops #750 from meaning "ignored". That change downgraded kq's
stored numbers from gating to reporting, and a reported number needs somewhere
it is read.

## four questions were only in a chat log

design/pending-gavels.md opens by saying it holds "every decision waiting on
Clay, in one place". Four were not in it, including the one blocking the most:

  13  what an imported record prints as
  14  are kanso's bytes a type or a convention
  15  should `>>` defer its right side
  16  should block-born widen to a dataflow property

They existed in a task list and in turn-end summaries, which is to say they
existed wherever somebody happened to be looking. Number 13 gates the harness
rework and decides three micro samples, and it has been reported as blocking in
several consecutive summaries without ever reaching the file whose whole purpose
is to be the place you read when you sit down to rule on something.

The failure is not that they were forgotten — they were repeated constantly.
It is that repeating them in prose is not the same as recording them, and the
difference only shows when the person reading is not the person who wrote the
summary. A question living in a chat log is a question its owner has to be
reminded of; a question in the file is one they can find.

Each entry carries what the others do: the question, where it came from, the
interim state, and what it unblocks. Two of them already have interim work
shipped and say so — the `>>` diagnostic names the operator on all three
engines without making the loop run, and the bytes divergence is pinned by a
test carrying its unwritable half `#[ignore]`d, which is not an unfinished
acceptance criterion but an assertion nobody can write down yet.

## GAVEL: welfare cannot fall, and the gates have two severities

Clay, 2026-08-03, three rulings in one sitting, recorded together because they
are one design.

ONE. The welfare index is a HARD gate. It cannot regress — not with a reason,
not with a named trade, not for a new language feature. A change that would
lower it gets optimized until it does not, and if that is genuinely impossible
the work stops and the question goes to Clay in conversation. No feature-utility
term is added to the model ("we'd never want to just outright turn down a
feature, so probably better to just measure welfare and make whatever
optimizations are necessary"). `--set` therefore refuses every fall; the only
mechanical override is editing bench/welfare_floor.json by hand, where a
reviewer sees it in a diff. This closes a gate that had already been loosened
twice — first any sentence banked a fall, then any sentence naming a gain did.

TWO. Every PER-COUNTER check is the softer pair: no Pareto dis-improvement
(something worse with nothing better fails outright), and any single regression
requires investigation to make sure it is justified. That is the trend gate's
existing shape, now ratified as the intended severity rather than an interim.
The candidates floated for a harder class — presence counters hitting zero,
conservation gaps, fast-path abandonment, superlinear scaling — stay in the
soft class, and are fine to graph and to test at that severity.

THREE. The published graph is five series, all deterministic: the four core
metrics (run speed, run memory, compile speed, compile memory) plus the welfare
aggregate. The cross-language board is the one wall-clock surface that remains,
because a competitor's cost cannot be counted by kanso's counters — it is
published as a RELATIVE metric with the noise caveat stated, averaged over
several runs.

The foundational ask behind all three, in Clay's words: "i can always go to the
latest main and see whatever performance run output came from the most recently
merged PR, full stop. no ambiguity."

## the chart draws itself from counters, and the hardware path retires

The foundational ask, in Clay's words: "i can always go to the latest main and
see whatever performance run output came from the most recently merged PR,
full stop. no ambiguity." The chart was the surface furthest from that. It was
drawn from bench/long_view.tsv, whose rows need instructions-retired — a
counter GitHub's runners do not expose — so CI redrew a chart whose data it
could never extend, and the series only grew when a capable machine remembered
to run the replay.

NOW: long_view reads the perf-history series (deterministic counters, extended
by CI on every merge since #724's era), CI stages it plus the commit under
test's own row, redraws on every pull request, and commits the redraw to the
branch under test. Any runner extends it; none can smear it; a PR's chart ends
at the PR. perf_row.kso, backfill_history.kso and long_view.tsv are deleted in
the same change, per the porting rule: the replacement lands and the replaced
thing goes, so nothing compares the two.

THE FIVE LINES, per the ruling: run speed (the codec's two allocation counts),
run memory (the one-shot arena peak), compile speed (rounds + visits + emitted
lines), compile memory (the front end's peak), and the recorded welfare score.
Recorded, not recomputed — the score a commit shipped with is a fact about
that commit, and replaying history against today's baseline would rewrite it.

TWO EXCLUSIONS, both to keep every line meaning one thing for its whole life.
The basket is absent from the run-speed line because no history row recorded
it until today — perf_record simply never gained a basket group, found when
the line drew empty — and adding it mid-line would draw a step no change made.
The held-malloc peak is absent from run memory for the renamed-counter version
of the same reason. The welfare line carries both, because the recorded score
always did. perf_record now records the basket group, so the row is complete
even where the chart declines to draw it.

A series starts where its counters start: run speed reaches back 451 rows,
compile speed all 500, welfare 87. Lines with honest starting points beat a
complete-looking chart whose early half was reconstructed.

The long-view prose on the page is rewritten to describe this chart rather
than the replay it replaces, and the section on the gates now states the two
severities the gavel fixed: welfare hard, everything per-counter soft.

## the hand-sat board is ripped out, and ci is the only author left

Clay, on the quiet-box rows the previous entry proposed guarding: "sounds like
that old legacy stuff should just be ripped out?" And on the guard itself: "i
don't know what you mean 'stale'. nothing can go 'stale' anymore." Both right,
and the second sentence is the design: once every published number is written
by the run that measured it, on the pull request that merged it, there is no
number left that CAN age, and the stamp gate this entry was going to introduce
has no subject. It was not built.

WHAT WENT: the decode board's four hand-measured rows on the compiler page,
the same figures quoted in the landing panel, the landing prose and the about
page, and scripts/cross_surface_numbers.kso — the checker whose entire job was
keeping those hand copies agreeing with each other. With one CI-written
surface and no hand copies, the drift it guarded is unrepresentable, which
retires the checker the way the modules rework retired KNOWN_WRONG entries:
by construction rather than by vigilance.

WHAT THE BOARD IS NOW: two lanes, kanso and serde_json, because those are the
two CI runs on every merge. Five interleaved rounds — both lanes under the
same contention, so the ratio holds while the absolute milliseconds wobble —
averaged, spliced into the page by scripts/relative_board.kso, and committed
by the same job that commits the chart, so the two published surfaces move in
one commit by one author. The go and plain-rust comparisons move to the
recipe, where a reader runs them on their own machine; a number we cannot
remeasure is a number we no longer publish.

The prose on all three pages drops its absolute milliseconds for the relative
claim. What survives of the old framing is the part that was always true: no
lifetimes, no garbage collector, no annotations, and a recipe you can run.

## the board converges on the pull request, because main refuses every push

The first post-merge run on main failed with the answer to a question nobody
had asked yet:

    ! [remote rejected] HEAD -> main (protected branch hook declined)

Main's protection requires the status checks and refuses any direct push —
including CI's own. So "commit the wall-clock board on main, post-merge" was
never possible, and the churn problem it was dodging had to be solved rather
than relocated: an unconditional splice of a wall-clock number commits a
slightly different board every run, and a pull request's head churns forever,
each push orphaning the checks that ran before it. This branch produced that
churn live — three "remeasured" commits in twenty minutes, one arriving in the
middle of the fix for it.

BOTH CONSTRAINTS MEET IN THE SPLICER. relative_board now reads the board the
page already carries and skips the write when the fresh measurement is within
five percent per lane. A PR's first run writes the board; every rerun measures
within tolerance of it and goes quiet; a real compiler change moves the lanes
past tolerance and rewrites. Verified in all three directions before shipping:
write, skip, rewrite.

Five percent is chosen against what it must absorb and what it must not: the
wobble two runs on one runner class show sits inside it, and a change worth a
board update moves the ratio past it — or moves the counters, which gate
elsewhere and are exact. The ratio is the claim; the tolerance is the noise
floor stated as a number.

Main needs no second write. What the PR committed IS the measurement of the
code that merges, and the commit-back step now runs only where a head exists
to push to. The perf-history append is untouched — that branch is unprotected,
which is precisely why the series lives there.

## the third stale premise in one week, and the probe that measured a corpse

Read-write map uniqueness sat in pending-gavels as entry 2, waiting on Clay.
He declined it in a sentence — not his question, a compiler-logistics issue —
which matches the standing rule that internals are settled by measurement.
Measuring it closed it: the recorded 2.0 GB quadratic 10k tally is dead linear
today at ~1.8 allocations per iteration, flat peak, four sizes checked. The
read-side compaction killed it and nobody updated the entry.

That is the third open thread this week whose premise was stale — the chart
that "could not be extended by CI", the carry quadratic, and now this. The
mechanism is not the log's format, and Clay named the correction: every one of
the three closures WAS in this record — the carry repair, the deterministic-
series switch, the read-side compaction. An append-only log is event-sourced;
the current state of a thread is the fold over all its entries, and the
failure was reading one entry that said "open" and treating a single event as
the state. The procedure, stated so it binds: an entry claiming a thread is
open is a claim about its own date, and believing it requires searching
FORWARD for the entries that superseded it — grep the thread's nouns, not its
verdicts. The task list now holds only startable work and the gavels file
only gavels, which shrinks how often the question arises; the log itself
needed no change.

THE PROBE TRAP, kept because it nearly shipped a wrong conclusion the other
way: the first fixture read `m[k] + 1` over unseeded keys, so `none + 1`
refused on the first iteration and the counters measured a corpse — five
allocations, beautifully flat, meaning nothing. A measurement is not the
counters alone; it is the counters of a program whose OUTPUT was checked.
Flat-at-five was too good, which is the only reason it was caught.

## RULED: an imported record prints its qualified type name

Recorded here after Clay had to say it a fourth time, which is its own
finding: the ruling was given, survived nowhere durable, and the question kept
being re-derived from the raw divergence as if it were open. The gavel now
lives in the gavels file, the memory system, and this log, and the corpus
enforces it — the strongest of the three, because a golden cannot forget.

The rule: a record prints its type's QUALIFIED name. Where the type is
declared in the entry module there is no qualifier, so `point 3 4`; where it
arrived through an import, `sample/point 3 4`. The two runs differing is the
design. Err reasons follow the same rule.

The three micro samples excluded from the imported-corpus test now carry
`.imported.out` goldens with the qualified spelling, and the exclusion list is
deleted — all eighty exportable samples run both ways. The goldens were
generated with each sample imported under its own name, because the qualifier
IS the import name; the first generation used a staging alias and produced
`sample/slow_lane` where the harness sees `err_trap_named/slow_lane`, which
the test caught immediately.

This unblocks the chain that has been reported blocked for two days: the
harness rework, the corpus migration by area, and the compiler's five `play`
sites last.

## the harness owns the entry, which is where the play convention now lives

Step one of the ratified migration. `run_kanso_as_library` stages a sample's
directory, writes the two-line entry that imports it and names its exported
lambda, and runs that — the convention the compiler used to synthesise, now
authored by the thing that runs samples. A sample with no `pub play` is
already an entry file and runs directly, which is the same test the compiler's
`declares_play` applies; when the corpus finishes migrating, that test dies
with the other four sites.

Its first consumer is the imported-corpus test, which hand-rolled the same
staging and now dedupes into the runner — eighty samples through the entry
path on both engines, the three record-printing ones against their qualified
goldens, ten of ten golden suites green.

The corpus migration can now proceed area by area: each slice points its
suite at the library runner, drops `pub play` from its files where the parser
allows a bare statement, and CI stays green throughout. The compiler's five
sites go last, when nothing remains that needs them.

## the micro corpus runs as libraries, and its two tests become one

The first area slice of the play migration. The direct-run micro test and the
imported-corpus test collapsed into one: every sample runs through the
harness-generated entry, the entry-file samples run directly through the
runner's fallback, and the three record-printing samples assert their
qualified goldens per the ruling. Micro no longer touches the compiler's
entry synthesis at all, which is what migrating an area means.

Synthesis keeps its coverage from the areas not yet migrated — mem, runtime,
examples, errors — and loses a suite's worth each slice until the five
compiler sites have nothing left relying on them. Nine golden suites green.

## the queue held nothing anybody could start

Clay, looking at the visible task list: "it has been stuck on 108 tasks with the
top one being 're-sit the published compile-speed...' — so if you're actively
working on performance, then it should be 'performance'. you work off the
visible list."

He is right, and the diagnosis is worse than staleness. The five visible items
were, in order: a CONDITION (re-sit the numbers — needs an idle box, and load
was 4.77, 5.88, 4.00 and 3.97 on every check this sitting), a DEFERRAL (its own
title said QUEUED FOR LATER), and three RULINGS. Not one of them could be picked
up by anybody. They sat above roughly a hundred completed items, so the queue's
entire visible face was work nobody could start.

That is why performance kept falling off a list that was supposed to be ordered
by it. The performance items had all been DONE — the quadratics, the fused
reducer, the accumulator rewind, the view leak — and nothing replenished the
queue, because the only things left in it were things that could not move.

WHAT WENT WHERE, and the rule it comes from:

  a ruling                 design/pending-gavels.md
  a deferred design        the design doc that owns it
  a condition              whatever already fires on it
  a standing direction     the rule that makes it fire on contact
  work that could start    the queue

Twelve items moved out. Three questions gained numbered entries in the gavels
file (printing a lazy sequence, `pure` as a record type, whether io should
infect) and four more joined its terse list. Two conditions were closed against
the rules that already cover them: the performance-PR definition of done
requires re-running the decode floor on every change and moving the published
numbers when they shift, which fires on the next front-end change on whatever
box is running. A permanent task adds nothing to that except an unstartable
item at the top.

WHAT IS LEFT is the honest number, and it is small: one startable feature item
and one investigation. That is the real state, and a queue that says so is worth
more than a hundred-item list whose head nobody can act on.

The general form, which is the same failure as four others in this log: a
mechanism that cannot fail loudly gets ignored, and a queue whose top is always
blocked teaches you to stop reading it.

## the errors corpus runs as libraries

Second area slice. Swept first: of the 124 fixtures, 29 are entry files
already, 97 produce byte-identical diagnostics through the harness-generated
entry, and 23 differ only by the loader's ` (module …)` suffix — those carry
`.imported.stderr` goldens, the same shape as micro's `.imported.out`. The
suite runs every fixture through the library path and the corpus no longer
touches the compiler's entry synthesis.

## a fold one import hop deep loses its accumulator

The mem-corpus play slice found it: swept through the harness entry, 17 of
41 fixtures diverged, fused_tally by 16x in allocations and 500x in held
peak. Three passes recognized std/list's fold by the spellings "fold" and
"list/fold", so a caller one import deeper — "mid/list/fold", which is every
library user — never got the accumulator dispensation, and in-place push,
put and append all degraded to copies. Fold recognition is now
declaration-keyed (linear::fold_spellings: file std/list, short name fold,
arity 3, every spelling the module graph produced), used by linear.rs and
beat.rs both.

Two more pipeline divergences fell out of the same sweep. compile_one — the
direct-run path — was the only pipeline not running inline_builtin_wrappers,
so direct runs paid a dispatch hop per std wrapper that entry and directory
builds did not; it runs it now. And the inline pass itself rewrote calls to
multi-arm groups (text/split's empty-separator err arm), deleting the
dispatch that reaches the other arm — a latent defect on every path the
corpus did not cover; the rewrite now requires the forwarder to be its
group's only arm, while the type checker keeps the per-arm map. The
construction-cohort license generalized from "caller is unqualified" to
"the call crosses down into a nested module", which is the same boundary at
any depth.

Numbers: oneshot allocs 234,323 -> 128,528, arena peak 7.3 MB -> 3.8 MB,
beat_iters 1 -> 12,581 (the directory-built bench had been paying the
qualification tax all along). Welfare 75.68 -> 75.69, ratcheted. Decode,
encode, basket, compile veins unchanged. Eight mem goldens regenerated
smaller; build_cycle carries .imported goldens for the ruled qualified
rendering. The mem suite now enters through run_kanso_as_library.

## the runtime corpus runs as libraries

Third area slice, and the sweep again paid for itself twice. First find: the
native runtime's lazy hints (`push takes a list …`, `length takes …`) tested
record types with a bare "list/" prefix, so one import hop deeper the hint
vanished on native while the interpreter kept it — a real engine
differential, fixed in k_lazy_hint and the length arm to accept any
qualification depth, matching eval.rs.

Second find: a pass-order asymmetry. A dependency module ran
inline_builtin_wrappers at its own compile, so the importer's literal check
saw `fn wrapped x = text/bytes x` already lowered to `builtin_bytes x`, read
the author's own wrapper as an alias, and refused `wrapped 5` at compile
time — where the direct run of the same program dies at runtime, a boundary
the error corpus documents on purpose. Dependencies now return checked but
unlowered; only the root lowers. Costs the front end 41 visits on lib/json
(25874 -> 25915, regenerated); every runtime cost vein is byte-identical.

Of the 56 fixtures, 3 are entry files, 41 are byte-identical through the
harness entry, and 12 differ only by qualified names in messages and traces
(the ruled rendering: `endpoint_trace/report ← endpoint_trace/grade`) —
those carry .imported.stderr goldens. Both engines agree on every one. The
suite enters through run_kanso_as_library.

## the examples corpus runs as libraries

Fourth area slice, quiet this time: of 47 examples, 7 are entry files, 35
are byte-identical through the harness entry, and 5 differ only by the
ruled qualified record rendering (`record_render/point 3 4`) — those carry
.imported.stdout goldens beside the direct ones in tests/golden/examples.
No compiler change needed; the suite enters through run_kanso_as_library.

## a rename of a rename is still a rename, and a small fall still fails

The scripts slice stumbled on both. Running welfare read 75.68 against a
75.69 floor: #765's 41-visit front-end regression was priced at 0.003
points, never re-ratcheted, and the verdict's 0.01 dead band read it as
holding — CI green with the score below the floor. The band now covers only
what two hosts actually disagree about (56 bytes of compile peak, under a
ten-thousandth of a point): 0.001, with a spec staging a floor five
thousandths above the score and watching the plain run fail.

The visits themselves are reclaimed rather than paid. #765's
dependencies-stay-unlowered gate is reverted; the diagnostic asymmetry it
fixed is closed from the other side, by making inline::aliases transitive —
a single-arm wrapper whose body forwards to a known alias resolves to the
same builtin, so `fn wrapped x = text/bytes x` carries the builtin's
literal demand to its call sites on the direct path exactly as the imported
path always saw it. `wrapped 5` is now refused at compile time everywhere,
which is the better diagnostic; the error-corpus fixture's "deliberately
not refused" paragraph inverts, and the runtime fixture binds its literal
first to stay a runtime case. front_end_visits back to 25,874; welfare back
at its floor exactly.

## the scripts run as directory modules

Fifth area slice: every scripts/*.kso carried a single-line `pub play`, and
all twenty convert uniformly to the hako shape — scripts/x/ holds the
declarations and a main.kso whose one statement is what the play was. The
invocations (`kanso run scripts/welfare`) drop the extension at twenty-four
call sites across ci.yml, book_check.sh and the welfare gate's staging.
Outputs verified identical on welfare, page_drift, book_panels and the
dispatch differential; the rest exercise in CI. scripts/ now declares no
play at all.

## GAVEL: modules are Go-shaped, entries are files, play files are libraries

Ruled by Clay, 2026-08-03, in dialog:

1. Imports are PER-FILE, as in Go. Declarations still merge across a
   module's files; what pools no longer is the import list. Every file
   names what it uses; an unused import is a per-file finding. The pooled
   union dies.
2. An ENTRY FILE is a program, not a package member: a standalone file of
   statements with its own imports, never merged into a surrounding
   module. Importing a directory therefore never carries entry statements
   — which also retires the defect where the dependency path parsed
   main.kso with parse_entry.
3. TARGETING: `kanso run dir` means `dir/main.kso`; any file is targetable
   directly (`kanso run foo/special.kso`). There is no cmd/ convention
   because there is nothing to house — an entry is a file, and Go's cmd/
   tree exists only because Go entries must live inside packages.
4. A single file holding declarations plus a `pub play` is a LIBRARY. It
   runs through a real entry file that imports it and names its play — in
   the repo for book samples, generated by the harness for tests,
   synthesized behind the curtain by the playground. The entry is always
   real; it is simply not the thing a reader is shown. Entries stay
   statements-only.
5. hako's layout flips to match: main.kso moves out beside the module it
   imports.

This closes the book-presentation and playground questions in one stroke
and clears the path to deleting the compiler's five play sites.

## an entry is a file, not a member — the gavel lands

The ruled parts of the module-shape gavel, built: main.kso never joins a
module merge, whether the directory is a root or a dependency (the
parse_entry-in-deps defect dies with it); `kanso run dir` and `kanso check
dir` mean dir/main.kso; `kanso test dir` on a program directory descends to
its single module; an entry compile skips the self-import guards, because
an entry importing the module beside it is the ordinary case, and reads the
project's lock from its own directory. hako flips to the ruled layout —
main.kso beside hako/hako/ — and the embedded build compiles the entry
against embedded module files. The twenty scripts keep their single pub
play and their entries become the uniform two-liner. Fixtures that leaned
on member entries (arity, render_module, trace_demo, four book samples)
convert to the same shape; ch07's teahouse and ch08's using export the
surface their entries consume; ch11's sealed sample teaches the re-export
boundary in the qualified spelling now. Two long-standing recognizers were
blind one hop deep and surfaced under the new entries: io/exit's sentinel
(both engines) and book_panels' mis-named `applied?`. Welfare holds at its
floor; every vein byte-identical; 45 suites green.

Still PROPOSED, not ruled: the relaxed program file (declarations plus
statements in one unimportable file). Committee analysis in
design/pending-gavels.md; nothing of it is built.

## the benches and the module differential follow the entry gavel

The 769 CI reds were mine to find locally and did not get found, because
the cost-vein check reran binaries built before the branch. The four bench
programs convert to the ruled layout (jsonbench via its generator, the
tracked three by hand), the build verb gains the same directory targeting
as run and check, and every cost vein is byte-identical under the extra
hop — which is the fold fix doing exactly what it was built to do. The
module differential's member-era cases convert to entry-imports-module
shape; two expectations move to the ruled qualified rendering, and the one
real gap the suite caught is fixed: a bare enrolled type constructor now
marks its import used, so a program using only a module's types does not
read as unused.

## correction: no bare-type gap existed

The previous entry's last claim is wrong. No change to mark_bare_quals was
made or needed: the exports map always carried types, and the "unused
import" the differential showed came from a fixture file left outside the
module by the conversion, not from the check. The suite went green on the
fixture fix alone. Recorded because a log that quietly keeps a false
sentence is worse than one that visibly corrects it.

## a flat line draws flat

Twice in one day the range-scaled chart read as a cliff where nothing
happened: welfare on a 0.01 move, compile memory on fifteen bytes of an
819-kilobyte peak (819,173 -> 819,188 with #769, inside the documented
±56-byte host wobble). Each line is scaled to its own historical range, so
a near-flat series amplifies its noise to full plot height. The span a
line draws against is now floored at two percent of the series' magnitude.
Verified differentially on a synthetic two-row series: identical values
draw one y; the 7MB->3MB oneshot cliff spans the plot; the fifteen-byte
compile wiggle moves one pixel.

## GAVEL: the play verb runs little programs

Ruled by Clay, 2026-08-04, resolving pending-gavels entry 24. The relaxed
single file — declarations and statements together — exists, but only
behind a verb built to stay small: `kanso play foo.kso` runs it and will
not build it; its imports are the stdlib and nothing else, which is all
the web playground could offer anyway (the Go playground's contract); and
nothing can import a play file, so the form cannot leak into real
programs. Inside one, functions and types are defined right beside the
statements that use them — no `pub play`, no wrapper, no ceremony. The
browser playground buffer is a play file; book samples that teach single
ideas become play files shown whole.

This supersedes one clause of the earlier no-play-verb ruling: that ruling
kept the compiler from blessing a NAME; this verb is a harness for a FILE
SHAPE, and the name coming back to mean exactly what it says is the point.
`pub play` survives only where it belongs — library fixtures the test
harness runs through generated entries to pin the imported path.

Serving, for the last two python ports, follows net/http's shape: handler
functions, a mux, listen-and-serve, a file server — the minimal kanso
equivalent, designed after this lands.

## kanso play, built

The verb from this morning's gavel: `kanso play foo.kso` runs the relaxed
single file — constants and declarations first, then the statements that
use them — on either engine, and the browser gets the same door
(kanso_play beside kanso_run, sharing one execution tail). The handicaps
that keep the form small are all diagnostics with specs: `pub` is refused
("a play file exports nothing"), imports beyond the stdlib are refused
("a play file imports the stdlib and nothing else"), a declaration after
the statements is refused, and `build` never accepts one. The boundary
blank between the declaration half and the statements is a boundary, not a
trailing blank — the one parser subtlety worth recording. Four specs in
tests/play_verb.rs, a small golden corpus, every vein byte-identical,
welfare at floor.

## the playground buffer is a play file

The browser reaches the play door: kanso_play beside kanso_run for the
interpreter, kanso_play_wasm beside kanso_compile_wasm for the compiled
path, one lowering helper under both. playSource picks them; the
playground and the landing sample use it, and their twelve examples drop
`pub play` — a beginner's first screen is now a function and the lines
that use it.

Two rules the examples settled, after trying each the other way:

Declarations are `fn`, `type` and `import`. A binding stays part of the
run, exactly as in an entry file. Bindings-as-constants was tried first
and made the relaxed form MORE restrictive than the old wrapper: a
parameter could no longer take a constant's name (two teaching examples
broke on the shadow rule), and `build` — a statement form — could not be
bound at top level at all. A constant several functions share is a
library's job, and needing one is where a little program graduates to
`kanso run`.

The split is by kind, not position. Declarations are order-free
everywhere else in kanso and a relaxed file is not the place to invent an
ordering rule. The parser therefore hands the library parser a
non-contiguous half, which needs its blank lines synthesized: exactly one
per gap between consecutive declaration lines, taken from the original
blanks where the gap holds one. Comments never reach the parser at all —
the lexer drops them — so a gap's "next line" walk skips missing numbers
rather than only blanks.

## the book shows little programs whole

Forty-three single-idea samples drop their `pub play` and become play
files: a reader sees the functions and the lines that use them, with no
wrapper to explain. Each was converted and re-run before the change was
kept — the recorded output had to be byte-identical, and eighteen
candidates that moved were put back untouched.

book_check.sh picks the verb from the file's shape: a sample holding BOTH
definitions and a bare top-level statement is a play file. Definitions
alone stay a library, because appa/no_main is a sample whose whole lesson
is that `run` says so — the first heuristic (no `pub play` plus any
definition) rewrote that lesson, which is what a corpus is for. The
mode-suffixed samples (_check, _test, _build, _plan, _counters) keep the
library shape their recorded verb needs.

## a file names what it uses

The gavel's first clause, built: imports are per-file. Declarations still
merge across a module's files; the import list does not. Two findings per
file — an import nothing in it uses, and a qualified name no import in it
declares — and the second is the one the pooled loader could never make,
because a sibling's import resolved the name.

The corpus was nearly clean, which is the argument for the rule rather
than against it: authors already wrote the imports where they used them.
Three dead imports fell out — lib/json/json.kso, hako/hako/update.kso,
and the same line in kq's and kanso-json's mirrors of json.kso — each a
line that named a dependency the file did not have.

The check runs over the parsed files before the merge, so it reports the
file and the line rather than the module. Its own module's name and the
ambient render qual are not imports and are exempt.

## the order dependencies load in is the module's, not the file list's

Dropping that dead `import "std/list"` from lib/json/json.kso turned the
compile-memory golden red: 819,217 to 856,191, over the two per cent CI
allows. The per-file check was not the cause — deleting the whole block
left the number where it was, and the compiler's own contribution
measured at minus 158 bytes.

What moved was which file names std/list. json.kso is the module's first
file and text.kso its fourth, so removing the dead line moved std/list
from first in the import union to second. A 2x2 measurement, binary
against source: 819,402 with list named first, 856,349 with it named
second, and 819,376 when json.kso alone names it. Rounds and visits are
31 and 25,874 in every one of them — the same work, held differently.

Each dependency is compiled on top of everything loaded before it, so a
dependency's own peak stacks on the accumulated result of its
predecessors. Peak is therefore max over deps of (what came before +
what this one costs to compile), and the order decides it: compiling the
expensive dependency last, on the largest accumulator, is the worst
arrangement. An allocator trace of the top of each run says the peak is
a plateau of small live objects rather than one buffer, and the two runs
diverge gradually from about 102 KB on — B holding more of the same
throughout, not one table doubling.

The union is now sorted by path. Nothing about a module changed, so
nothing about what checking it costs should depend on which of its files
happened to name a shared import first. lib/json reads 819,218 against
the golden's 819,217, and welfare holds at 75.69.

tests/import_order.rs pins it: the same two declarations, split across
two files two ways, checked and compared. Watched red at 531,434 against
568,417 — the same 37 KB. The assertion allows a kibibyte rather than
demanding equality, because a module's declarations still merge in file
order and moving one between files costs ten bytes here; what it refuses
is a swing of order-scale.

## the compiler forgets the word play

The last clause of the module-shape gavel, built. `pub play` is an
ordinary exported function now: nothing in the compiler looks for that
name, and what runs one is the entry file that imports it. The five sites
that knew the word are gone, and what remains is deliberate — the `kanso
play` verb, and the diagnostic that redirects a reader who wrote `pub
play` and reached for `kanso play` instead of `kanso run`.

Every harness that had been running a corpus program directly now writes
the entry that imports it, which is what `kanso run` is handed. Three of
them wanted the same staging, so the shape is repeated in golden.rs,
oracle.rs and wasm_engine.rs; the oracle compiles by absolute path, so
its traces carry a staging prefix the runtime cases take back off before
comparing.

The wasm engine was the part that needed a decision. It has no
filesystem, so it could only ever be handed one file, and a library plus
its entry is two — the old play path is what had been hiding that.
`kanso_hand_source` hands the engine a module under the path an import
will name, and it compiles the entry beside it: the same two files the
native engine gets, neither of them read from disk. The browser needs
exactly this to run a book sample, which is what issue #82 asks for.

Three fixtures had to move rather than be rewrapped. make_dir's programs
became play files whose bindings run bottom-up, because a body binds in
order and each of those descriptions names the next. The `probe.kso` the
diagnostics sweep rewrites every iteration caught a staging bug worth
recording: the corpus directory is copied once, for the fixtures beside
a program, and the program itself every time — staging it once answered
every later probe with the first one's output, which read as a wasm
divergence on bits/shl and was nothing of the sort.

## a sweep proves it can run

Three of the differential sweeps spent a day reporting confident numbers
about nothing. Every probe they write was a `pub play` library handed to
`kanso run`, which refuses one now, so both engines refused identically,
the sweeps compared two refusals, and render said 68 values agree,
behaviour 66 calls, dispatch 22 cases. None of them had started an
engine.

Each already carried a shape check — one string compared to another,
asserting that a probe still interpolates. That check cannot notice this,
because the probe it inspects is a string and the failure is what happens
when the string is run. The effects sweep is the one that caught the
break, because it asserts the bytes each probe should print rather than
only that the engines agree with each other.

So each of the three now runs one probe whose answer is known, on both
engines, before any of them sweep, and stops with a word if the answer
does not arrive. The dispatch sample is built through the same template
the sweep uses, so a template that stops producing a runnable program is
caught by the same gate. Watched red three times, each by breaking the
template and the sample together — which is the shape the real failure
had, and the shape the string check is blind to.

## the book runs in your tab

Every sample in the book that can run in a browser is now an editor with a
run button, answered by the toolchain compiled into the page. A panel is
upgraded only when the program can actually run there: no filesystem, no
argv, no stdin, and no sibling import, so a sample that reads a fixture
keeps the output the book recorded from a machine that had one. Chapter
04 has fifteen panels and twelve of them are live.

A sample that exports `play` is a library, and what runs one is the entry
that imports it — which is why this waited on `kanso_hand_source`. The
engine is handed the library under the name the import will use and
compiles the entry beside it, the same two files the command line gets.

Two things had to be repaired to make it work at all. The engine fetched
`kanso.wasm` relative to the page, so it resolved only for pages at the
site root and a chapter under /book/ would have asked for
/book/kanso.wasm; it now resolves against the script's own URL. And a
panel's markup ends at the last character of the program, where a file
ends with exactly one newline — the first run answered with a formatting
diagnostic rather than the program's output.

site_smoke grew the case: it loads a real chapter, counts the panels that
went live, clicks the first one and requires the recorded failure back,
then types a different program into the same panel and requires that.
What it does not yet do is hold the book to the differential — the sweep
covers examples and the two golden corpora, so a panel that disagrees
with its recorded output would simply show a reader the difference. That
is the next task.

## the book joins the differential

The panels run in a reader's tab now, so the book belongs under the same
law as everything else: the sweep took 193 programs and takes 267. What
it added is every sample a panel actually runs — one level down, so a
member of a directory module is left out; recorded under `run`, so the
appendix's deliberately-broken programs are left out; and free of the
filesystem, argv and stdin a tab does not have, which is the same test
book-play.js applies before it upgrades a panel.

Three things had to be right for the two sides to compare programs
rather than refusals. The door is chosen by the file's shape now — a file
holding declarations beside statements is a play file wherever it lives,
where the harness had been asking whether the path was the playground
corpus. A test file is skipped, because `kanso test` runs one and the
report is its output. And the native side drops a staged directory whose
name matches the sample, because the modules chapter has both shop.kso
and a shop/ and an import cannot name both — the same collision
book_check hit earlier today, in the same place.

267 passed, 6 known gaps, 0 failed.

## the examples nobody could run

Forty of the forty-seven shipped examples could not be run by any verb.
`kanso run examples/guards.kso` said the file is a library and to run its
definitions beside their statements with `kanso play`; `kanso play` said
`pub play` is a library's export and to use `kanso run`. Two diagnostics
pointing at each other, on the first thing anybody types. It shipped in
the play migration and Clay found it.

They are single-file little programs, which is what the play verb is for,
so they became play files. Every one of the forty-seven then reproduced
its recorded golden byte for byte, which is what says the conversion
changed nothing but the door.

The harnesses that had been wrapping an example in a generated entry now
pick the door by the file's shape — declarations beside bare statements
is a play file — in golden.rs, oracle.rs and wasm_engine.rs, the same
rule the browser differential already uses. A binding is not a statement
there, or a file of `fn`s and `test_` bindings reads as a play file when
it is a test file with a verb of its own; the error corpus caught that
within a minute of the first attempt.

Two consequences worth naming. The five `.imported.stdout` goldens are
gone, because an example that runs directly prints unqualified names and
nothing reaches it through an import any more. And native.rs builds from
the micro corpus now rather than from examples, because a play file runs
and never builds — a binary is the first step of a real program, which is
what a library and its entry are for.

tests/examples_run.rs is the spec: every example, by the verb its shape
asks for, has to run. Watched red on all forty.

## http, and a cleaner desk

std/net/http, in Go's shape with one difference that matters: a handler is a
plain function from a request record to a response record. There is no writer
to hand it and no recorder to fake, so a test calls it and reads what it
answers, and injecting a different handler is passing a different function.
The mux is arms on the path — `fn route "/report" req` — which is what kanso
has instead of a switch, and adding a route is adding an arm.

A routed POST round-trips end to end on both engines: the program serves
itself, curl posts a body, and the handler's answer comes back.

The native runtime learned the five socket effects and, more importantly, the
two scheduling points: accept sets the listener non-blocking and yields when
nothing is waiting, and run forks and yields while await polls with WNOHANG,
draining both pipes each time so a child never blocks on a full one. The
diagnostics sweep covers the surface at 106 probes with nothing disagreeing.

Two findings from Clay in the same sitting. `kanso test lib/list/list_test.kso`
answered `unknown name find`: a test file is a module member, so testing one
compiles the module it belongs to. And nothing had ever run kanso's own
suites — the only `kanso test` in CI was the kanso-json mirror's, so four
standard suites could rot untouched, which is exactly what happened.

The desk: seven design notes described systems that have shipped, and they are
deleted rather than kept — the log carries history. The log itself was 17,935
lines, appended on every change and read only at the tail; it keeps the last
forty entries and the older end moves, unedited, to log/compiler-log-archive.md.
STATUS.md at the root says what is in flight and what waits on Clay, because a
task list he has to ask me to interpret is not a monitor.

## Two failures in one operation

Clay's ruling: when both sides of an operation fail, the answer carries both.
Before, every operator propagated a failing argument and `boom "a" + boom "b"`
answered `a`, discarding the other; a parallel group already merged. One rule
now covers both, on all three engines.

The mechanism was already there — the reason list a parallel group builds — so
the change is where it is reached from. In the interpreter, three sites that
found the first failing argument fold across all of them instead; the binary
operator matches on the pair. In the native runtime, nineteen paired guards
that returned whichever side failed first became one call.

Measuring the wall the same day turned up something that reads as a third rule
and is not:

    print "left {boom a}"  >> print "right ok"        →  a
    print "left ok"        >> print "right {boom b}"  →  b
    print "left {boom a}"  >> print "right {boom b}"  →  [a b]

Nothing prints in any of the three. `>>` orders effects, and both descriptions
are built before either runs, so a failure raised while building is not ordered
by the wall — two of them are simultaneous and merge, the same reasoning the
parallel group uses. Haskell's `>>` answers `a` in the third case because it is
lazy in its right side; kanso builds both and learns more. Whether that is the
rule to keep is #141, and it collides with #105 wanting the right side lazy for
an unrelated reason: laziness would buy back Haskell's answer and lose the
merge. That is the trade, and it is Clay's.

## A process can be ended

Headless chrome ignores `--virtual-time-budget` and runs until something kills
it — measured at the full forty seconds until an external `timeout` fired. The
two python scripts both handle this the same way, with `chrome.kill()` once the
page's report arrives over HTTP, and kanso had no equivalent: `io/run` starts a
process and waits for it, and nothing hands the program the handle.

So `io/start` answers the handle and `io/kill` ends what it names, which is
Go's `cmd.Start()` and `cmd.Process.Kill()` under the rule that the stdlib apes
Go. Both engines fork through one function now; the browser refuses, as it does
for every process effect. The spec observes what a killed child never got to
do: it writes a file two seconds in, the program kills it and waits three, and
asks whether the file arrived. Without the kill it says true.

Adding a name to the stdlib broke an example, which is the collision #53
describes: `examples/trace_demo/version.kso` declared its own `pub start` and
imports `std/io`, so the diagnostic told it to rename. The example's entry is
`announce` now.

The port also found `content-length` counting characters where the protocol
counts bytes — `rendered (ok "650 円")` promised seven bytes' worth of page and
claimed five, which truncates it at any client that believes the header. The
http module has its own suite now, and `text/bytes` is what the count runs
through.

## The wall was never supposed to merge

Clay asked what `>>` does with a function that answers either a description or
an err, and the answer exposed two defects behind the morning's merge.

The first: merging was pairwise cons, not a fold. Three failures in one
expression answered `[["a" "b"] "c"]`, and the same three grouped differently
answered a different shape, so the reasons a program reported depended on where
its parentheses fell. An err now carries a `merged` mark saying its reason is a
list *of reasons* rather than one reason that happens to be a list, and the
merge folds through it. `err ["a" "b"]` stays one reason, which is why the mark
cannot be read off the shape. Both associativity directions are pinned in the
micro corpus, which is the only way to see that a fold is a fold.

The second was worse. The sweep that gave every paired guard the merge gave it
to `k_seq` too, and `k_seq` is the one pair that must not have it: the wall is
ordered, so the first failure is the answer and what follows never speaks. The
interpreter still short-circuited, so native and interp disagreed on the same
program — `boom "a" >> boom "b" >> boom "c"` answered a merge on one and `"a"`
on the other, and only the book's own sample caught it. Chapter four states the
rule the sweep broke: short-circuit where there is order, accumulate where
there is none, and dependence decides. Both engines say that again.

The design thread this opened is #141, where the shape is a pair of monoids
over one carrier — adjacency associative and commutative with merge, the wall
associative with an absorbing zero — and the interesting part is that the laws
are what make grouping unobservable. Overloading either operator would mean
enforcing them, which the differential fuzzers can do.

## The wall says what it takes

`1 >> 2` passed `kanso check` clean and died at run time, though both operands
are literals and the wall's own rule names what it accepts. A call to a
function that can never answer an effect is the same case one step out, and the
fixpoint already knows which those are, so the judgement moved to `check`.

An err is a legitimate operand — propagating one is what the wall does when a
side fails — so the rule is narrower than it first looked: a side is refused
only when it can be neither an effect nor a failure. This morning's own micro
golden caught the wider version immediately, because its last line is
`boom "a" >> boom "b"`, two errs and entirely deliberate.

`tests/golden/runtime/sequencing_takes_two_descriptions.kso` moved to the
errors corpus. Its purpose was the runtime refusal, and the refusal is a
compile error now, which is where a case demonstrating it belongs.

## The history branch could not be reached

Main had been half-red for days: every push produced one green run and one red,
and the red was always `perf history`. The job redraws `docs/compiler.html`,
skips the commit step on main by design, and then `git checkout -B perf-history`
refuses to switch with the redrawn file dirty in the tree. So the branch it
exists to append to had not been appended to since 08-05, and the failure was
telling the truth about something nobody was reading.

The fix is `git checkout -- docs/compiler.html` before the switch, which is one
line and cost more to find than to write. It could not be verified locally — a
workflow change only runs when it runs — so it went in unproven and said so.
The proof is the row for `1e26bd1` in `history.jsonl`, the first since 08-05,
carrying its own PR's subject.

That row also prices the wall check from #144: `compile_allocs` 125679 →
128946, `compile_alloc_bytes` 6760131 → 6877123, with `compile_rounds` at 11
and `compile_visits` at 126 unmoved. The check walks the `returns` map the
fixpoint has already built, so it buys no extra round and no extra visit —
2.6% more compiler allocation for a diagnostic that moves `1 >> 2` from a
runtime death to a compile error. Welfare holds at 75.69, since it weighs
rounds and visits rather than compile allocations.

## What the copy costs, finally counted

Beats trade refcount traffic for a copy: survivors are evacuated out of a
region before it rewinds, and until now nothing counted the bytes that move.
That is the term the Perceus comparison turns on, because a refcounting runtime
pays none of it and pays per reference instead, so an argument about which is
cheaper had no number on one side of it.

`evac_allocs` and `evac_bytes` count at `k_copy_alloc`, the one point every
evacuated byte passes through — the loop carry, the beat pop, and the constant
freeze all route through it, so one counter covers three sites that would
otherwise need three.

The shape is far more lopsided than expected. Decode evacuates 11 allocations
and 464 bytes against 7,577,414 total allocations. Encode evacuates 19. The
basket evacuates nothing at all. Every one of those was assumed to be paying a
steady copying tax, and none of them is: a streaming shelf hands its survivor
straight out, and the region it rewinds held only garbage.

The one-shot shelf is the whole cost. 63,967 evacuation allocations against
128,528 total — half of every allocation the program makes is the copy-out —
and 1,991,456 bytes of 5,928,668. That is the same shelf the kq footprint work
found worst, and it now has a mechanism rather than a shrug: the full-print
path builds a result the beat cannot keep, so the beat copies it.

For the Perceus comparison this narrows the question usefully. On the streaming
shelves there is nothing to trade away, and a refcounting runtime would be
paying per-reference traffic for a copy cost of 464 bytes. The comparison has to
be made on the one-shot shelf or it is not a comparison at all.

The goldens were generated on darwin/arm64. Byte counts come from `sizeof` on
16-aligned allocations, so they should be identical on ubuntu/x86_64, but that
is a prediction and CI is what tests it.

## The board sits still

RULED (Clay, 2026-08-07): no auto-updated wall-clock artifact belongs on the
compiler page. The decode board is hand-sat and re-sat by hand when a release
goes out. Live racing stays where a jq comparison lives, in kq, because that
comparison cannot be made any other way. Everything else the page shows is
counted rather than timed — allocations, arena blocks, rewind iterations,
what the compiler spent deciding and emitting.

This reverses the ruling of 2026-08-03, which made CI the board's only author
on the grounds that a hand-sat number ages. It does age. What it does not do is
rewrite the page on every merge, and that turned out to cost more.

The splice was the root of the chart-commit churn. CI raced the decoders,
wrote a fresh board into `docs/compiler.html`, committed it back to the pull
request, and that bot commit spawned a `pull_request` run attributed to
`github-actions[bot]` — which waits at `action_required` until a human clicks
approve. No pull request could go green unattended. Two were approved by hand
in one sitting before the pattern was named.

The board had also quietly lost half of itself. CI's two-lane splice replaced a
four-lane board, so the page had been claiming kanso against serde_json alone
while the reasonably-written rust and go lanes sat in a second table below it,
saying almost the same thing at a different scale. There is one board now, four
lanes, sat 2026-07-27 on a quiet desktop, and the duplicate is gone.

`scripts/relative_board` is deleted rather than kept: nothing invokes it, it
splices a two-lane shape the page no longer has, and its header comment carried
the ruling that this entry reverses.

What this does not fix: the chart still commits once per pull request, and that
commit still spawns a run that needs approving. The redraw is deterministic now
that no clock feeds it, so a rerun writes the same bytes and the churn is gone
— but publishing at all still means a write, and main is protected. Killing the
last approval means building the page at deploy time instead of committing it,
which is a larger change and not this one.

## The board, re-sat, and what it says

Seven interleaved rounds on 2026-08-07, slope method — each lane built to run
150 times and 450 times, the difference divided by 300, so process startup and
the file read cancel out of both ends.

    lane                07-27    08-07     peak
    kanso                0.78     0.87    4.2 mb
    serde_json           0.87     0.90    6.8 mb
    reasonably rust      1.02     1.04    6.9 mb
    go                   1.95     2.05   11.8 mb

Every absolute number is up, because the box was not idle. The ratios are what
interleaving buys, and they say the drift was not uniform:

    kanso / serde   0.897 -> 0.967
    naive / serde   1.17  -> 1.16
    go    / serde   2.24  -> 2.28

Three lanes held their relationship to each other and one moved. Kanso's lead
over serde fell from about ten per cent to about three, which is a regression
of roughly seven points, and the field standing still is what makes it one
rather than weather.

The deterministic counters did not move: decode allocations are still
7,577,414 and the cost golden is green. So this is time per allocation rather
than a count, which puts representation, thunk forcing on the decode path, or a
fast path no presence counter covers ahead of anything allocation-shaped. The
page already says laziness spent a slice of the margin back, and that is the
first place to look. Peak memory improved slightly over the same window, so
nothing was traded for footprint.

Recorded as its own thread; the ratio against serde is the number to watch,
because it is the one contention cannot move.

## The chart draws itself

The chart is drawn in the browser now, from the same `history.jsonl` the
counter panel below it already fetches. Nothing writes it into the page.

The committed svg was 13,477 bytes of coordinates regenerated by CI on every
pull request and committed back to the branch under test. That commit is what
made a pull request need a human: a run on a bot-authored commit waits at
`action_required` until somebody clicks approve, so no branch could go green
unattended. Two were approved by hand in one sitting before the shape of it
got named.

The page was most of the way there already. The counter panel fetches the
history branch with `cache: 'no-store'` and draws sparkline polylines from the
parsed rows; the long view is the same operation at a larger scale with a
legend. `scripts/long_view` is deleted, along with the staging step, the
redraw, and the commit.

What this buys beyond removing the approval: the series now extends the moment
a row lands on the history branch, with no page redeploy at all. Publishing was
the reason the write existed, and there is nothing left to publish.

Verified by running the ported drawing over the real 500-row series: five
polylines, five legend keys, 3,154 coordinates, none outside the viewBox, and
the aria-label counting the rows it was handed. The page's own javascript
parses — checked through JavaScriptCore, since the box has no node.

## The value nobody reads

`42` on a line of its own was a compile error and `double 21` was a crash. The
parser catches the first because shape alone decides it — an integer literal is
not an effect, and the parser runs before inference so shape is all it has. A
call is the same line to the parser: `print x` and `double 21` are one shape,
and only the fixpoint separates them. So the second reached the runtime and
died inside the group's join, naming `&`, an operator the author never wrote
and cannot go find in their file.

The check belongs where the inference already sits, and `check_effect_discarded`
was the template. What made it awkward was not the pass.

Three call sites dropped every diagnostic whose kind is "unused" before anything
could read them, and the shape of that turned out to be worth the measurement.
Two of the three filtered the output of `check_merged`, which emits no "unused"
diagnostic at all — they removed nothing and always had. The third sat two lines
below a call to `check_unused_private` whose every finding it then discarded: the
front end walked the whole program to build diagnostics it threw away, on every
compile. Add-then-discard is not a policy. Not calling it is, and that is what
the play path does now.

So no retagging was needed. `tests/golden/errors/unused_expression.stderr`
already pins kind "unused" for exactly this finding one step in, and the new
case joins it as `unused_call`.

The pass took two attempts because the first was written against a shape that
does not exist by the time checks run. A body's adjacent lines are not a list of
statements there — they are one `Expr::Join` spine, which is why the runtime
message says `&`. `decl.body[..last]` is therefore almost always empty, and the
first version could not have fired on anything. The members of a group are the
leaves of that spine, and the last leaf of the last statement is the body's
result.

An err is a legitimate line on its own, since it propagates through the group,
so a call is refused only when it can be neither an effect nor a failure — the
same narrowing the wall check needed two entries above, learned the same way.

## The empty map is `{}`

RULED (Clay, 2026-08-07): the empty map literal is `{}`, spelled the way every
other language spells it. `{:}` is gone.

It had never been gaveled. It appears as one bullet in an archived batch note
and in no design doc or on the compiler page, and the only user-facing mention
was one line of appendix B. Nothing contends for `{}` in expression position:
`parse_atom` has exactly one `LBrace` arm and it goes straight to map parsing,
while the other brace uses are `import { … }`, a statement, and keyed
destructuring, which enters through a different door.

What made `{}` refuse before was not the parser but `required_gap` in the lexer,
which had no brace rule and so demanded a space between any pair — `x = {}`
answered `canonical form requires exactly one space here` and pointed at the
closing brace, a message that was wrong under either spelling. The rule now
says `(LBrace, RBrace) => 0`: a map's braces hold their contents apart, and the
empty pair has no contents to hold. That keeps `{ "a":1 "b":2 }` exactly as it
was and makes `{ }` non-canonical, so there is one spelling of the empty map
rather than two.

Both renderers print `{}`, and `{:}` now gets a diagnostic naming the
replacement. The appendix's sample printed the *spelling* through escaped
interpolation braces — `"empty {{:}}"` — so it never showed an empty map at
all; it binds one and prints it now.

## The decoder shows what it emits

Nothing counted the emitted code of `bench/jsonbench` — the one program whose
runtime the decode board publishes. `bench/compile_golden.txt` counts emitted
lines, calls and branches for five small samples; `perf_record` counts them for
`kanso check lib/json`, which is front-end cost. The decoder's own emitted code
was unwatched, and it grew:

    2026-07-27   calls 1957   branches 1067   defines 217   lines 11,080
    2026-08-07   calls 2346   branches 1311   defines 250   lines 13,329

Twenty per cent more calls for a program whose allocation counters never moved
— allocs reads 7,577,414 at both ends and at all 396 commits between.
`bench/emitted_golden.txt` pins it now, counted from the IR before the linker
runs, which makes it deterministic and blind to code layout.

**It does not explain the 7.6% decode regression, and the series is why.**
Emitted calls against measured time, both taken across the same window:

    07-27   calls 1957   0.806 ms
    #592    calls 2325   —
    #607    calls 2325   0.811 ms
    #618    calls 2325   —
    main    calls 2346   0.867 ms

The code grew nineteen per cent while the time held flat, and then the time
rose seven and a half per cent while the code grew one. The two are
anticorrelated over this window, so emitted-call growth is a real unwatched
dimension and a false explanation, and the cause of the slowdown is still
unknown.

Three claims were retracted getting here, all from the same root: believing a
number before running the control that could falsify it. #607 was named as a
six per cent regression from single builds that reproduced to 0.002 ms —
repeatability that was real and meaningless, since it measured one binary's
alignment twice; its IR is byte-identical to its parent's at 445,691 bytes. The
kanso/serde ratio was adopted as "contention-invariant" when serde is built
once and reused, so its drift leaks in rather than cancelling. And emitted-call
growth was published as the cause of the slowdown before the series that
disproves it had been plotted.

The controls that would have caught each are cheap: diff the emitted IR before
believing a per-commit timing; rebuild the accused commit with neutral padding
and see whether a no-op moves it as far; and plot a proposed cause against the
effect across the whole window before calling it a cause.

## The shipped engine is not byte-comparable, and that is the finding

`docs/kanso.wasm` is committed and the site serves the committed file, while
every wasm job rebuilds it before testing. So the spec measures a blob CI has
just made and never the one in the tree, and a stale committed blob would be
invisible.

A check refusing a committed blob that differs from the rebuild went red in CI
immediately — and it was the check that was wrong, not the blob. A fresh build
on this machine reproduces main's committed blob exactly; CI's linux build does
not. The wasm build is deterministic WITHIN a host and not ACROSS hosts, and
the earlier claim that it was reproducible came from two consecutive builds on
one machine, which tests only the first thing.

That also retires the finding that started this. The blob main shipped was not
behind its source; it was built somewhere else. There was no staleness to
catch.

What survives is narrower and still real: nothing exercises the artifact the
site actually serves, and no byte comparison can, because byte-identity is not
defined across hosts. The check that would work is behavioural rather than
textual — run the engine differential against the COMMITTED blob instead of a
freshly built one, and let it answer whether the shipped engine agrees with the
oracle. That is a different job from the one attempted here and is not built.

The mtime guard in `tests/wasm_engine.rs` looks better in this light than it
did. It cannot see content staleness, which is a real limit, but content
staleness is not measurable by comparison either.

## The wall is not overloadable

RULED (Clay, 2026-08-07): `>>` is not overloadable. He could think of no
meaningful overload, and that settles it — the wall stays a compiler special
form and the ordered-effects meaning stays with it rather than moving to a
default arm.

The laws survive the ruling. They describe the operators without being a
user-extensible interface: adjacency is associative and commutative with an
identity and a merge on failure, the wall is associative and not commutative
with the same identity and an absorbing zero. That answers the question that
opened the thread — whether the wall takes an array of results or two at a
time — with two at a time, because associativity makes the binary fold
indistinguishable from an n-ary form. No third operator, and the shape of the
grouping is unobservable in the result.

Associativity is also what kills the fractal rather than any flattening step.
Drop it and grouping becomes observable, which is the bug #143 fixed.

Also ruled the same day, on the book samples the decidable-failure fold turns
into compile errors: teach both, because a compile-time refusal and a run-time
failure are different lessons. That work is held, though, because of what
follows.

## Division by zero may not be a failure at all

Clay, contending rather than ruling: division by zero is not an error but a
standard type you have to handle.

Today it is an err — `3 / 0`, `3 % 0` and the float forms all raise into the
unhandleable channel and reach the endpoint. The contention is that this
mis-files it. An err means we did it wrong; dividing by zero is a question with
no answer, and `total / count` where the count is zero is an ordinary data
condition rather than a contract violation. That is what `none` is for, under
the gavel that already says none is a value and err is the failure.

What it would do to the decidable-failure fold is the interesting part: it
repurposes it rather than retiring it. The fold's job today is to refuse
`3 / 0` where it is written. If division answers none, the job becomes proving
which divisions CAN answer none — where the divisor is provably non-zero the
result carries no obligation at all, and where it is provably zero the value is
none and the existing none rules make the author handle it. Same analysis,
better outcome.

The cost is that arithmetic acquires a handling obligation it does not have
today, everywhere the divisor is not provably non-zero. Whether that is
acceptable is the question, and it is Clay's.

The seven book samples wait on it. Under the current design they teach catching
a failure; under the contention they teach handling a value, and writing them
twice is the one outcome nobody wants.
## 2026-08-08 — an operator asks both its operands

An arm naming its type in the second position — `fn + _ b:money` — compiled on
every engine and could never be called. All three gated dispatch on the LEFT
operand: `matches!(&left, Value::Record { .. })` in the interpreter,
`f.set_of(a) & REC` plus a tag test on `a` in codegen, one `rt_is_rec(a)` in
wasm. The only shipped example of an operator arm puts a record on both sides,
so nothing exercised the asymmetry.

The gate now asks both sides, through one predicate the three engines share:
a record routes, and so does a subtype of one. `2 + 3` is untouched — the
static half of the native gate still requires a REC in the inferred set, and
the whole branch only exists in programs that declare an arm for that
operator. The decode and emitted goldens are byte-identical, and welfare holds
at 75.69.

Writing the golden for it turned up three more, none of them the gate:

- **k_sub, k_div and k_mod did not unwrap a subtype**, where k_add, k_mul and
  k_cmp did. `money 350 - 1` answered 349 on the oracle and died on native —
  a differential-law violation of the worst kind, since the refusal reads as
  a type complaint rather than a missing feature. Fixed here, pinned by a
  micro golden that sends one subtype through every operator.
- **A subtype of a primitive owns no operator at all** (#162). `type money int`
  with `fn * _ price:money` compiles and never runs; the same program with
  `money` as a record does. Both engines agree, so it is a design question,
  and it is Clay's — it decides whether the operator propagation #159 asks for
  works, depending on which shape the math failures take.
- **Native dies on an operator arm that answers a string** (#163), on either
  side, on main as well. The message comes from the string-accumulator path,
  so the interpolation is lowered against a set that says the operand is a
  number.

A subtype also refuses to match its parent's *constructor* pattern —
`fn * n (money cents)` never sees a `sale_price`. A plain function refuses it
identically, so it is the pattern layer rather than the operator, and an
annotation (`_:money`) matches where the destructuring form does not.

## 2026-08-08 — the operator that had no callers

#163 above, chased and fixed in the same change rather than left filed. The
crash was not in the operator machinery at all: `linear::string_builders` marks
a parameter an accumulator when every caller hands it over uniquely, and it
looks for that by walking named calls. An operator is called by syntax, so
`money 350 * 3` is a `BinOp` node and the walk finds no calls to `*` — the
question answers yes over an empty set, the plain parameter is marked a string
builder, and codegen emits the in-place join for a seed that was never
converted. `k_concat_arr_mut` then found a string with no capacity and said so.

The same hole was already closed once, from the other side: `escapes_as_value`
refuses the marking for a group handed to a fold or curried, "because a
question about what every caller hands over then has no calls to look at and
answers yes for free". That is this bug's sentence, written before this bug.
The fix sits beside it — an operator group is refused for the same reason,
which is the honest answer when the analysis cannot see the call sites.

Only operators were reachable. The other ambient group, render/to_string, is
protected by the ownership rule: an arm must match on a type the module owns,
and at arity one that leaves no room for the bare parameter this needs. At
arity two an operator can name its type in one position and take a bare
variable in the other, which is exactly the shape that fails.

All three cost goldens are byte-identical and welfare holds at 75.69 — the
guard only ever declines a marking, and nothing shipped has an operator arm
that answers a string.

## 2026-08-08 — the carry keeps its copy

Built the copy-or-pin idea the shelf campaign left open, measured it, and
declined it. The change is reverted; this is the record.

`k_cohort_pop` already refuses a copy that buys too little — it sizes the
survivor and keeps the region when `(2 * survivor) > grown || survivor > cap`.
`k_beat_iter_carry` sizes the very same walk to allocate its buffer and then
always copies. Porting the cohort's ratio there is a four-line change.

Where the copies come from, split by a probe on bench/oneshot:

    cohort pop     1 call    ~31,986 copies    cohort_kept=0
    iter carry     2 calls    31,981 copies    need=995,600 bytes

The cohort's guard was never missing from this picture. It ran, priced the
trade and took the copy. And the loop carry fires twice, not once per element,
so the 63,967 evacuations were never the shape the name "copy every iteration"
suggests.

The port, across all four benchmarks:

                  evac_allocs      evac_bytes        arena peak
    jsonbench     11 ->  8         464 -> 352        flat
    encodebench   19 -> 19         624 -> 624        4 MB -> 5 MB
    oneshot       63,967 -> same   unchanged         flat
    basket        0 -> 0           0                 flat

It fires 352 times on encode, saves nothing, and costs a megabyte of peak.
One-shot does not move at all, because both its carries have `2 * need <=
grown` — the region that grew is more than twice the survivor, so the copy is
worth making by the project's own rule.

The comment written with the change argued that retention is self-limiting:
the garbage kept raises `grown`, the ratio falls, the rewind returns. Encode
falsifies it. Three hundred and fifty-two retentions in a row grew the arena by
a block rather than converging, because each one keeps so little that the
threshold never crosses.

The cohort tests a size floor before its ratio, which the port omitted. Adding
it gives `carry_kept=0` everywhere and numbers byte-identical to baseline. So
the guard is a loss without the floor and a no-op with it, and there is no
version of it in between.

The reason is a difference in distribution, not in code. The cohort decides
once, at a pop, where the region is large and the survivor is the whole result.
The carry decides every iteration, where the region is one iteration's garbage
and the survivor is the accumulator. Above the floor, no carry in the corpus
holds a survivor worth more than half its region; below it, the ratio fires on
trivia. One threshold does not serve both.

What this leaves for the frontier: those 63,967 copies are not waste any policy
here would remove. Avoiding them without retaining the garbage takes reference
counting, which is the Perceus measurement that has still never been run.
## 2026-08-08 — what crosses a beat boundary

The Perceus comparison has been blocked on a number nobody had: how much of
what the arena holds is still needed. A probe at the two sites where the live
set is exactly known — the loop carry's `need` and the cohort pop's `survivor`,
both already sized by `k_copy_size` to allocate a buffer — supplies it.

    shelf      boundaries   live crossing (max)   arena held then
    decode              1        112 bytes          1,048,576
    encode            401         80 bytes          4,194,304
    one-shot            3          995,504          3,145,728
    basket         10,000                0          1,048,576

Encode crosses a hundred and twenty-eight bytes in total across four hundred
and one boundaries. Basket crosses nothing at all across ten thousand. On three
of the four shelves the beat model retains almost exactly what it must, and a
refcounting runtime would pay a count on every one of 7.5, 16.2 and 0.03
million allocations to improve on that. One-shot is the exception at 31.6%,
which is the shelf the evacuation counter had already singled out.

What it establishes, and the earlier compaction-ratio framing blurred this: it
measures liveness AT the rewind, so it bounds retained garbage from above —
at encode's rewinds essentially the whole four megabytes held is dead. It does
not give a refcounting peak, because RC frees as values die rather than at
boundaries, so its peak is the maximum simultaneously-live set, a quantity
between eighty bytes and one block that this probe cannot see.

Two things bound the apparent gap in any case. The arena's floor is a block, so
eighty bytes is not a footprint any policy here could reach. And a count field
is a per-object header kanso does not have at all, which is the one advantage
over Koka that a hybrid must not trade away.

A route recorded earlier and now withdrawn: measuring live-at-peak on the
interpreter as a proxy for native. The interpreter has no arena and no beats,
so the two share the language and nothing about allocation. The measurement
site was in the native runtime the whole time.

## 2026-08-08 — the convention each profile can keep

`kanso build --release` produced a segfaulting binary for ten of the eighty-five
sample programs. Nine imported std/regexp and one imported only std/list and
std/io, which is what said the shape rather than the module was the trouble.
The default build ran all eighty-five correctly, and so did both other engines.

The IR is byte-identical between the profiles — `--release` only changes the
clang invocation — so nothing kanso wrote was at fault. -O0 works, -O1 and
above crash, optimising the runtime alone is fine, and the IR passes clang's
verifier. ASan named the failure as a jump to address 11: a value reached the
program counter. opt-bisect over 42,420 passes put the first break at CodeGen
Prepare on a seven-parameter `tailcc` arm holding a `musttail` to itself — six
KValues at two registers each plus an int, thirteen against arm64's eight.

That is the shape #587 already knew about. Its trampoline routes calls to such
functions through a plain wrapper, and exempts `musttail` because a guaranteed
tail call cannot survive one. Removing the exemption takes the failures from
ten to six, which is a real hole closed and not the cause: stripping every
`tailcc` and `musttail` from the module makes the same program answer correctly
at -O3, so the convention itself is what the optimizer breaks here.

Stripping it everywhere is not available either. At -O0 nothing turns a call
into a jump, so `deep_tail` — two hundred thousand hops through mutually
tail-calling arms — overflows the stack in the default build the moment the
convention goes.

So each profile keeps the convention exactly where it is the one that works.
The default build emits `tailcc` and `musttail` as before. The release build
emits neither, and -O3 performs the tail calls itself:

    default build, deep_tail                200,000 hops
    release build, deep_tail                200,000 hops
    release build, arms of differing arity  200,000 hops
    release corpus                          85 of 85, was 75
    decode, seven interleaved rounds        0.12 s either way

The arity case is measured because plain sibling-call optimization is not
obliged to take it and the beat machinery needs it taken.

What release gives up is the guarantee rather than the behaviour: -O0 promises
the jump in the IR where -O3 is observed to make it. A future LLVM that
declined would show as a stack overflow, which is loud and pinned by
`deep_tail` in the release gate below. Today's defect is a binary that jumps to
address 11 and says nothing.

The gate is the other half, and it is the reason this went unseen: nothing
release-built a program and ran it. CI release-builds only the benchmarks, none
of which import regexp, and the IR verifier builds the sample corpus without
the flag. `micro_corpus_survives_a_release_build` now runs all eighty-five
against the goldens the other engines answer to, because what a program prints
cannot depend on how hard it was optimized.

## the invariant moves into the emitter

A register-returned record travels as `%parsed`, two words in registers, and
only one consumer reads it in that shape: a carried argument slot. Every other
consumer names its operand `%KValue`, and handing it the two words instead
emits IR the host's clang refuses. That rule has been written in a comment
above `as_value` for months and enforced nowhere, so it was maintained one
consumer at a time: the binop operands, the thunk capture, the closure
capture, the nullary constant, the list and map elements. Five fixes, each
correct for the shape it was written against.

The sixth was a field read reached through a closure — `k_b_field` handed a
`%parsed` — and the seventh a return, `ret %KValue` on a name bound to a
carried call. Fixing those two the same way would have left an eighth.

So `FnEmit::line` boxes instead. A line about to name a parsed temp in a
`%KValue` operand position gets the record built back first and the operand
substituted; `box_parsed` moved onto `FnEmit`, with the type id riding in the
parsed map, so the repair needs nothing from the backend. Carried slots write
`%parsed` and are untouched. The delimiter set is `,`, `)` and end of line —
the last of those is the `ret` case, and `%t2` being a prefix of `%t20` is why
it is a delimiter test rather than a substring one.

    jsonbench emitted IR         byte-identical
    jsonbench build, 5 rounds    0.19 s either way
    welfare                      75.69, at the floor
    isolated consumer probes     9 of 9 agree, %parsed live in each

The IR being byte-identical on the heaviest `%parsed` user is the measurement
that matters: the chokepoint fires only where the program was already invalid,
so the register convention keeps everything it buys.

The probes are the other half. A sample exercising many consumers of one group
tests none of them — one escaping use boxes the whole group and the convention
never comes up — so each probe is its own program with its own type, and each
was checked for a live `define %parsed` before its answer was believed.

## what a constant that names itself is worth

Three engines held a self-naming constant three ways, and one of them made up
an answer. `ring = cell ring` printed `probe/cell 0` on native where the oracle
refused: the mention loads the constant's global while `k_caf_init` is still
building it, and a zeroed `KValue` is the integer zero. Nothing said so. A
program could read that field and compute with it.

Two changes, both small.

`k_render` had no case for a cell, so an unforced one fell through the switch
to `<value>` where the oracle says `<thunk>`. It now unwraps a forced cell and
names a pending one, which is what the oracle does — rendering demands nothing
on either engine.

And `k_caf_init` seeds every cell with a blackhole before it runs any builder.
The mention then loads a cell nobody has demanded rather than a zero, so the
fabricated integer is gone. What replaces it is `probe/cell <thunk>`, which is
still not the oracle's `error[runtime]: a lazy binding demands its own value` —
but it is honest about having no value, and demanding it refuses rather than
answering.

    jsonbench calls     2346 -> 2350
    jsonbench lines    13329 -> 13338
    defines, branches  unchanged
    welfare            75.69, at the floor

Four calls and four stores, once, before main — the decoder has four constant
cells. Nothing in a decode loop moved.

Two gaps stay open and are written down rather than left to be rediscovered.

The demand path is the larger one. The oracle forces where a value's identity
is needed; native forces only where the callee pattern-matches the argument,
so `x + 1` on a pending cell answers "`+` is not defined for these values"
instead of demanding it. Forcing both operands in `emit_binop` was built and
measured: it did not fix the case — a parameter's static set carries no thunk
bit, so the force is a no-op exactly where it is needed — and it changed the
decoder's emitted code, where it is not. That is a demand-analysis question,
not an emitter one, and it was declined here on the measurement.

The browser holds a deferral as the closure that would answer it, so it says
`<fn>` where the other two say `<thunk>`. Both are honest about having no value
yet and they do not use the same word, which is a gap recorded in
`tests/golden/wasm_gaps.txt` rather than a divergence hidden. Closing it means
teaching the renderer to ask a deferral what it is, and the renderer is the
interpreter's own.

## a builder can count itself now

`regexp/matches` over a freshly stitched chapter cost 45.4 seconds where the
same bytes read from a file cost 2.3, every match paid again, and crossing a
beat made it fast. Several ticks had gone at that with the wrong theories —
laziness, memoisation, evacuation, the matcher's shape — and every one of them
was wrong for the same reason: they were allocation theories, and every
allocation counter is byte-identical between the fast and slow positions.
9,205,650 allocs, 327,097 evacuations, 281,268 thunk evaluations, 1,056 beat
iterations, both ways. Twenty times the wall clock for identical allocation
work is a walk, not an allocation.

Two memos, and a builder could use neither.

`k_str_chars` caches a character count in `cap` as a negative number, and only
`if (cap == 0)`. A builder's `cap` is the room it may grow into, so the memo
was never written and every read walked the whole string. The seek cursor in
`k_b_slice` — one remembered position, which is what makes a forward sweep over
prose linear — is gated the same way, `s == k_seek_str && s->cap < 0`, and its
comment said why: a builder may grow under the cursor.

    position                    scans        bytes walked   wall
    before the io/write     2,433,691   136,831,850,690    45.4 s
    after the io/write              0                 0     2.3 s

136 GB of character counting for a book of a few hundred KB.

The count now lives in the eight bytes before a builder's data, where
`k_buf_of` keeps a list's header for the same reason, and it is kept as bytes
arrive rather than recomputed: each part counts its own characters once and
caches that itself, so the sum is free and nothing ever walks the accumulator.
The cursor's objection turns out not to hold — a builder only ever appends, and
the cursor is an offset into what is already there, which appending does not
move, not even across a grow, because the header is the same object and the
offset is a number rather than a pointer.

    45.4 s  ->  12.7 s   the count kept
    12.7 s  ->   2.30 s  the cursor opened to builders

which is the baseline and the file-read control, both 2.3. The report the
script prints is byte-identical.

A DESIGN BUILT AND MEASURED INSUFFICIENT, first: a sticky "every character is
one byte" bit in the top of `cap`, which answers what slice asks without
needing anywhere to put a count. It was hit zero times. The stitched chapter
has 158 multibyte characters in 70,773 bytes, so the condition never held —
the em-dashes the slice comment already warns about, in a case built to serve
the ascii path. A cheaper proxy for the count could not stand in for it.

`str_scans` and `str_scan_bytes` are new, and they are the point. This cost is
invisible to every allocation counter because walking allocates nothing, which
is exactly how it survived: the timing looks like layout noise and nothing else
moves. `tests/golden/mem/builder_counts_once.kso` pins them flat — 2 scans and
15 bytes for four hundred reads of a 7,200 byte string. With the memo read
deleted the same program answers 402 and 2,400,015, so the ratchet has been
seen red for its own reason.

Every vein moved by exactly two lines and nothing else: 41 `.mem` goldens and
all four cost goldens gained the two counters with every other number
byte-identical, and `bench/emitted_golden.txt` did not move at all — the
counter is a runtime one and the decoder emits the same code. Welfare 75.69,
at the floor. kq keeps veins of its own and will want the same two lines when
its pin moves.
