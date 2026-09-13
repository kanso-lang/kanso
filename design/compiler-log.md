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

## 2026-09-09 — the compile side has three changes of measurement, not two

The 2026-09-08 entry on the compile term said the workload had been
re-measured twice, and named a four-row table whose third row was a
compiler change rather than a change of measurement. Scanning all 500
rows of the perf history for an adjacent move over 20% in
`compile_instructions` finds three:

    row  commit    what happened                     instr    allocs    peak
    446  5f5561c   #1291: lib/json drops std/list   0.4586   0.4490   0.5177
    476  dfd118a   #1321: lib/json -> corpus        2.7232   2.7207   2.1047
    485  8535884   #1331: sums both compile paths   4.3612   1.0000   1.0000

The middle row reproduces the three factors already recorded in the
floor file, to the digit, which is what says the factors can be read
off the history at all. The last row moves `compile_instructions`
alone, as `bench/objective_sources.txt` implies — one gate key each
for allocs and peak, two for instructions.

The first row is the one nobody wrote down, and the 2026-09-08 entry
supplies its own reason for counting it: "a term measured on a library
moves whenever that library changes what it imports: #1291 dropped
std/list from lib/json and halved the compile veins with the compiler
untouched." That is the disease #1321 was opened to cure, so it owes a
factor like the other two.

The record already disagreed with the entry. kanso#1347 item 2 asks
for "the epoch table from the 2026-09-08 entry, so each of the FOUR
compile epochs is scored against a baseline scaled to its own
measurement" — and four epochs need three boundaries. The count was
wrong against the record before it was wrong against the measurement,
which is the cheaper of the two to check.

**DONE — `bench/compile_epochs.txt`**, one boundary a line in
`bench/runbench_phases.txt`'s style, read by `scripts/welfare_rescore`
and applied to the baseline before the compile term is scored. Four
epochs: A rows 0..445 (lib/json with std/list), B 446..475 (without),
C 476..484 (compile_corpus, module row alone), D 485..499 (module and
entry summed, today's). A row's divisor is the product of the factors
of every boundary at or after it.

Either side of each boundary, welfare before the table and after it:

    boundary                     before             after
    a23b005 -> 5f5561c    59.0275 -> 61.0313   56.7964 -> 57.0280
       step                     +2.0038             +0.2316
    e12a68e -> dfd118a    70.0258 -> 67.7549   66.0242 -> 66.0242
       step                     -2.2709             +0.0000
    fd3144d -> 8535884    67.9134 -> 66.2874   66.2875 -> 66.2874
       step                     -1.6260             -0.0001

The two pure re-basings flatten. dfd118a and 8535884 touch no src/ and
no lib/, and their steps go to zero; the -0.0001 is the floor file's
four-place 2.7207, which is the precision the factor was recorded at.

**The first boundary keeps +0.2316, and that is the answer rather
than a residual.** 5f5561c is #1291, which halved the compile workload
and made the escape scan skip-then-iterate for runbench -3.3884%.
Taking the ruler out leaves the runtime win standing. A boundary that
flattened to zero here would have been the table erasing a real
improvement.

**One part of this is independent evidence and the rest is not.** The
446 and 485 factors were derived from the counter jumps, so applying
them and recovering 1.0000 is the same arithmetic inverted. The 476
factors were measured by another session at the time of the change and
written into the floor file; they land at 1.0000 against numbers they
were never fitted to. That is the check worth having.

**The table could not be measured at all until the fold-seed
miscompilation was fixed** (kanso#1349, same day). The divisor map read
1.0 for all 500 rows because every stored divisor aliased the last one:
`list/fold`'s folder was granted an in-place write licence without
anybody asking whether the fold's seed was uniquely owned. With the fix,
`oldest_divisor` reads compile_allocs 1.2215943, compile_instructions
5.446526138624, compile_peak_bytes 1.08960319 — the products above, to
the digit. The table's first working run was the miscompilation's first
reproduction.

**The boundaries a history carries must be a SUFFIX of the table**, and that
one rule covers three situations that look alike. It took three cuts to find.

The first refused any history missing a boundary. That turned three spec files
red for a reason none of them was about — `the_reconstruction_refuses_a_splice_that_moved`,
`a_row_predating_a_counter_is_scored_on_what_it_has` and
`the_score_says_what_it_was_made_of` all stage synthetic histories — and
seventeen fixtures would have had to carry three commits they say nothing
about, with every future one after them.

The second was all-or-nothing: refuse a partial match, give a history carrying
none of them no epochs. That is right about fixtures and wrong about the file
CI actually rewrites. `history.jsonl` is bounded to the newest 500 rows, so the
oldest boundary leaves the window one day while the table still names it — and
under all-or-nothing that is a partial match, so main goes red for a file doing
exactly what it is meant to do. Read off origin/perf-history on 2026-09-09
the three sit at rows 444, 474 and 483 of 500, so the first departure is 444
merges out — long enough that it would have arrived with nobody expecting it.
The position DROPS BY ONE WITH EVERY MERGE, because the window keeps the newest
five hundred, which is why this reading is dated: it was 446/476/485 two merges
earlier, and a fixed count written down here is a count that goes stale between
the measuring and the writing. It already did once, in this entry's own first
draft.

So: all present, the table applies. A hole in the middle, or the newest gone
while an older one stays, is a stale table and refuses. None at all is the
whole table fallen off the front — a fixture — and gets no epochs. The
boundaries that left the window are dropped from the product, because a
boundary older than every row scales no row.

**DONE — the spec, and the first cut proved nothing.**
`tests/the_compile_epochs_flatten_their_own_boundaries.rs` holds four.
The first stages, per boundary, a fixture carrying every boundary's
commit — so the epoch table applies at all — where only the
pair under test holds counters: 10,000 before and 10,000 x factor
after, so the four-place factors make every number an exact integer and
nothing rounds inside the fixture. The second replays the counter names
against `bench/objective_sources.txt`, so a typo cannot sit in the
table scaling nothing. The third leaves one boundary out of a fixture at a
time — never the oldest, so an older one is always still present — and requires
the refusal by name.

**The fourth is the one whose first cut proved nothing, for the second time in
this entry.** It scores the same rows twice, once against a fixture carrying
every boundary and once against a slid window, and requires the two to agree.
The obvious assertion was that the boundary still FLATTENS, and that survives
the mutation the test exists to catch: if the departed boundaries keep their
factors in the product, every divisor shifts by the same amount, so every step
is still right and a pair either side of a boundary agrees exactly as before.
What moves is the absolute column, against a floor that is ratcheted. Watching
the mutation pass is what found it; the rewritten test reads 97.1221 against
the whole-window figure and goes red.

The first cut passed under a mutation that moved both numbers. The
welfare column is a STRING — `fixed` renders it and `put` stores what
it rendered — and the helper walked digits from the colon, hit the
opening quote, and answered the empty string for every row: the
comparison was `"" == ""`. The helper now reads between the quotes and
asserts what it read is a number. The mutation that works is
`epoch_value v d[k] true` in `at_epoch`, which takes the identity arm
for every counter and still compiles; `1.0 * v / 1.0` does not, because
`f` goes unused and the tool dies before reading stdin, which the spec
then reported as a broken pipe rather than as the tool's own
diagnostic. The harness lets that write fail and prints what the tool
said.

**OPEN — the re-basing marks (task #449).** Stamping the baseline into
each row and marking where it moved cannot be derived from the
counters, and the measurement says why: the compile side has three
adjacent moves over 20% and the run side has ZERO across
`run_instructions`, `instructions` and `encode_instructions` over all
500 rows. The run baseline did move — kanso#1284 re-based the run
counters to parity, which is the -32.62 step — and no counter moved
with it, because only the ruler changed. A workload change shows up in
the rows and a baseline reset does not, so deriving the marks from the
counters would find three of the four and silently miss the largest.
Queued behind this entry because it edits the same file.

**Owed from kanso#1346, recorded here (closes the kanso#1339 thread).**
The ratchet's touched pass selected ELEVEN rows on kanso#1346's branch,
against kanso#1338's five selected and three proved. kanso#1346 merged
without recording it.

**ANSWERED — what else the fold-seed hole reached.** `scripts/welfare_rescore`
was miscompiled and nothing pinned its output, so the question is whether any
other tool in `scripts/` was in the same position. Fourteen were run on native
and on the oracle and their output compared: `welfare`, `page_drift`,
`golden_prose`, `diagnostic_coverage`, `fingerprint`, `book_quotes`,
`grammar_check`, `stale_a_panel`, `perf_record`, `trend_gate` and `ratchet`
agree byte for byte. `prose_check` and `book_panels` were not compared — the
oracle does not finish either inside five minutes. `site_smoke` disagrees, and
it is the divergence `tests/a_file_that_is_not_text.rs` already pins: native
reads `docs/kanso.wasm` and hands the bytes back, the oracle refuses with "the
bytes are not text". Reduced to seven lines, that is the whole of it. Whether
`read_file` should be byte-transparent on every engine is filed as a design
question and is not re-asked here.

## 2026-09-09 — the rescore read its own arithmetic back as a measurement

Main went red at 06:48 on the `perf history` job, one of nineteen, and
stayed red: `the splice rows disagree on a phase they share`. The cause
is kanso#1346, merged the run before.

`scripts/welfare_rescore` rebuilds `run_instructions` for the rows
measured before runbench existed, and it WRITES the rebuilt count into
the row it hands back. ci.yml feeds it
`origin/perf-history:history.jsonl` — its own previous output — appends
one row and rescores the lot. So a rebuilt count comes back as an input,
and nothing in the file says which counts were measured.

The numbers, read off three successive history files:

    file       all-eight rows   run-bearing   carry BOTH
    dc658d65   48 (last 438)    62 (first 439)     0
    81ffc9e5   48 (last 437)    63 (first 438)     0
    22ca0fb3   48 (last 436)   112 (first 389)    48

The first two are the design: the phases were the measurement until
runbench arrived, the consolidated count took over, and the two sets
meet without overlapping. The third is one run later. Forty-eight rows
gained a rebuilt count at once, the oldest of them became the splice
anchor at line 389, and it was compared against the newest all-phase row
at 436 — forty-seven commits apart, so of course every phase disagreed.
The refusal was right. What it was refusing was this tool's own output.

**DONE — a measured run count is not the same as a row carrying one.**
A row carrying every phase is from before runbench, when the phases WERE
the measurement and the consolidated count did not exist to be taken, so
a count sitting on such a row is this tool's arithmetic. The splice
anchor now selects rows that carry a count and NOT the full phase set.
That is checkable against the record rather than asserted, which is what
the table above is for.

**And the rebuild recomputes rather than trusting what it finds.** A
count written on an earlier run was scored against whatever anchor that
run picked, and keeping it lets one run's arithmetic outlive the
reasoning behind it. Recomputing also repairs the file already carrying
the rebuilt counts, with no hand edit: the corrupted history rescores
clean, and a second pass over that output is byte-identical.

**The property is idempotence, and it is what CI relies on.** "Rewrite
the whole column every push" is only safe if feeding the tool its own
output gives the same answer, and nothing asserted that. The spec is in
`tests/the_reconstruction_refuses_a_splice_that_moved.rs` beside the
refusal it belongs with: rescore a three-row history, feed the output
back, require success and an identical column. It reproduces main's
failure message in a postcard, and it would have caught this on the day
kanso#1346 landed rather than one merge later.

**What generalises.** A tool whose output is its own next input needs
that property pinned, and this one had a guard against exactly the error
it went on to commit — the splice check exists because a rebuilt row
spliced onto a row that moved slides the whole history smoothly and
plausibly. The guard fired correctly and pointed at the wrong culprit,
because the file it reads cannot say which numbers were measured. A row
that does not record where its number came from will eventually be asked
to answer for it.

**And the page was describing a coverage level that no longer exists.**
`docs/numbers.html` said a row holding three of the five counters scores
0.74 — it said 0.44, and named the two rises as 0.28 to 0.44 and 0.44 to
1.00. Read off the file today the levels are 0.00 (151 rows), 0.28 (237),
0.74 (48) and 1.00 (64), and what each holds is:

    0.28   compile_allocs, compile_peak_bytes
    0.74   those two, compile_instructions, run_instructions
    1.00   those four and run_peak_bytes

The forty-eight at 0.74 are the reconstructed rows. kanso#1346 gave them a
rebuilt run count, which took them from three of five to four, and the prose
was not followed — the same omission that let the rebuilt counts be read back
as measurements. The figures moved twice over: once when the reconstruction
landed and again under the epoch table.

**None of the four is a `data-golden` span**, so `golden_prose` cannot see
them and CI stays green with the page wrong. That is the gap kanso#1338 closed
for the library row by wiring it into the gate, and it is still open for
everything the chart's prose quotes. The figures are dated in the sentence
now, which is a weaker guard than a span and the honest one to have while they
are unwatched.

The two rises read 0.28 -> 0.74 with the score 88.28 -> 62.67, and 0.74 ->
1.00 with 64.95 -> 56.73, measured against origin/perf-history rescored by
this tree — which is what CI writes on the next push to main.

## 2026-09-09 — the blob stops being committed, and the guard covered two entries of eleven

`docs/kanso.wasm` is what the playground runs and what every spec in
`tests/wasm_engine.rs` runs, and it was a build artifact that was also
committed. It is not committed any more, and the guard that was supposed to
make that safe was covering two of the eleven places the artifact is opened.

**The criterion was already written down and the answer was already in.**
ci.yml carried a step, deliberately not a gate, whose own comment said: "Once
a few runs have said the same thing, the answer decides whether this becomes a
gate or the committed blob stops being committed." It has said DIFFERS every
time. The measurement is not needed to reach that answer either — the
committed blob's last commit is `6f8c876e`, from 2026-09-06, and main has
merged past it many times since, so it is built from older source and cannot
reproduce whatever the toolchain does. Rebuilt on this container it is
1,736,492 bytes against the committed 1,719,102, a difference of 17,390. Both
steps are gone; the rebuild that every job already ran stays.

**Four jobs rebuilt it and a fifth read it, and the first sweep here said
five and called that every consumer.** ci.yml's specs, other-host and site
jobs, pages.yml and `scripts/browser_differential.sh` do rebuild. The asset
digests job does NOT: it runs jekyll over `./docs` and fingerprints the `_site`
that copy produces, with no rebuild anywhere in it. So the sentence "fingerprint
reads `_site`, built after the rebuild" was true of pages.yml and false of the
job in ci.yml with the same shape.

Found by running it rather than by reading it. A `_site` copied from a `docs/`
with no blob gives `missing asset: kanso.wasm` and exit 1 out of
`scripts/fingerprint`, and `undigested_references.sh` then fails behind it on
five surviving references. That would have been a red round. The job rebuilds
first now — before jekyll rather than before the fingerprint, because jekyll is
what copies the file — and the repaired sequence reads fingerprint exit 0 and
gate exit 0 on a staged `_site`.

The lesson is the one the tree keeps relearning: a list of consumers assembled
by grep names the files that mention the artifact, and the job that breaks is
the one that reads it through something else.

**The guard was at two call sites and there are eleven.** `freshness()` was
called from `the_wasm_engine_agrees_with_the_golden_corpus` and
`the_wasm_engine_complains_the_way_the_others_do`. The other nine —
four playground-prompt specs, two page-failure specs, the builtin-count
refusal, the error corpus and the partial-over-a-value limit — called
`Toolchain::load` with no check at all, so a stale blob let nine specs run an
old engine and pass. The guard's own doc comment said a stale artifact "would
let this whole file pass while proving nothing about the source"; that was
true of nine twelfths of the file it was written in.

Measured rather than reasoned: with the blob moved aside, the suite reported
eleven failures, two of them naming `scripts/build_wasm.sh` and nine saying
only "the wasm artifact reads". The check now lives in `Toolchain::load`,
where all eleven pass, and re-running with the blob absent gives the same
sentence eleven times.

**The checkout arm is deleted with the thing that caused it.** kanso#1180 added
an arm for the case where the blob is a few milliseconds older than every
source file, because a fresh clone writes `docs/` before `src/` and so tripped
the guard on every checkout. A checkout no longer writes the blob, so that arm
cannot fire; keeping it would leave a paragraph of explanation for a state the
tree can no longer reach.

**And it closes the mtime guard's most common false pass.** kanso#107 kept the
guard an mtime comparison and wrote down that content it cannot see may be
stale. The way that bit in practice was `git checkout -- docs/kanso.wasm`,
which stamps a NEW mtime on OLD bytes: the guard then passes and the engine
runs a blob built from an older compiler. kanso#1350's own validation
paragraph is a case of it — two wasm specs failing on this container for
exactly that reason. With no committed copy there is nothing to restore, so
that trigger is gone. The guard is still mtime-based and still cannot see
content; this removes the one routine way of fooling it, not the weakness.

**What this does not claim.** The pack keeps every historical version of the
blob — 382 of them, 26,886,564 bytes — and removing the file from the tip does
not shrink an existing clone. The saving is prospective: the blob stops gaining
a version per rebuild that anyone remembers to commit.

## 2026-09-09 — a row says where its run count came from, and the mark is sticky

The 2026-09-09 red main was a provenance failure wearing an arithmetic
costume. `welfare_rescore` wrote rebuilt `run_instructions` into the rows it
handed back, ci.yml fed it `origin/perf-history:history.jsonl` — its own
previous output — and with nothing on the row saying which counts were
computed, the rebuilt ones read as measured and one became the splice anchor.
kanso#1350 fixed the tool: the anchor now takes rows carrying a count and NOT
the full phase set, and the rebuild recomputes rather than trusting what it
finds. That closes the loop for this tool. It does not help the next reader,
and the file still could not answer the question.

It can now. A row that this tool rebuilt carries `run_source: "rebuilt"`; a row
whose count it did not write carries `"measured"`; a row with no count carries
nothing. Read off the live history the split is 48 rebuilt, 65 measured, 387
unmarked, and the mark agrees with `scored_weight` on every row — all 48
rebuilt sit at coverage 0.74, all 65 measured at 1.00, and no row has a count
without a mark. The welfare column and every `run_instructions` are
byte-identical to what CI wrote, and the file stays a fixed point under its own
tool over three passes.

**The mark carries no factor, and that is the design rather than an omission.**
The compile side is RE-BASED by an exact divisor a row could name;
this side is REBUILT from phase shares, and there is no divisor because no
ruler moved. A `1.0000` written here for symmetry would read as "measured
against an unmoved baseline", which is the precise misreading the mark exists
to prevent.

**THE FIRST CUT STRIPPED THE MARK AND RE-DERIVED IT, and that was wrong.** It
looked like the welfare column, which is recomputed every pass, so it was
treated the same way. It is not the same kind of fact. Whether a count was this
tool's arithmetic is a fact about where the number CAME FROM, and nothing
re-measures a commit from three weeks ago, so it cannot expire. The failure is
visible in the degraded pass: with no row available as an anchor nothing is
rebuilt, and a mark re-derived from scratch then sees only a row carrying a
count and writes `"measured"` on it. Measured on a history with the anchors
removed:

    fixture            rebuilt   measured
    the strip          0         48
    the sticky mark    48        0

Forty-eight rows relabelled from true to false, by the field whose whole job is
to stop that confusion. The mark is sticky now: a row already claiming
"rebuilt" keeps it, and "measured" is only ever written onto a row that has a
count and no claim. With an anchor present the rebuild recomputes anyway, so
the sticky path is the degraded one, and there it degrades toward the truth.

`a_rebuilt_count_stays_marked_rebuilt_when_the_anchor_is_gone` pins it, entered
through the real tool on a three-row fixture and then on the same rows with the
anchor dropped. Watched red under the mutation that restores the strip, which
reddens that spec and leaves the other three green.

**Still owed, and it is the other half of the task.** The compile side has an
exact per-counter divisor in `bench/compile_epochs.txt`, and a row does not yet
name the epoch it was scored in. That wants the boundary commits tracked
through `crossed` rather than only their product, so it is a real change and
not a line. The two marks answer different questions and must not be made to
look alike.

---

## 2026-09-09 — a row names the epoch its baseline was measured in

**DONE.** `bench/compile_epochs.txt` names three commits that changed how the
compile counters are measured. `scripts/welfare_rescore` divides the baseline
by every factor from a row's epoch forward, so two rows either side of a
boundary are scored against two different baselines. Which one a given row sat
on was not written anywhere. A reader comparing a row from March against one
from September was comparing two numbers over two rulers and had nothing on the
page to tell them apart.

Every rescored row now carries `scored_base`. The fold that computes the
divisors already walked the boundaries; it now carries the list of boundaries
still ahead of the row alongside the divisor map, and stamps each row with the
first one left. A row past all three reads `as measured`.

**The boundary row belongs to the NEWER epoch.** Its own factor is already
divided out — it is the first row measured the new way — so it is named by the
next boundary, not by itself. The first cut named from the list before the
row's own commit was removed, and stamped row 443 `before 5f5561c` when 5f5561c
is the commit that row IS. That reads as though the row predates a change it is
the first to carry, which is the one misreading the column exists to prevent.
The spec pins the whole sequence rather than a shape, so the off-by-one turns
it red wherever it shifts.

Over the live 500-row history: 442 rows `before 5f5561c`, 30 `before dfd118a`,
9 `before 8535884`, 19 `as measured`. The boundaries land at rows 443, 473 and
482, which cross-checks against the positions recorded independently on
2026-09-07. Every other column is byte-identical to what the previous tool
wrote over the same input, and the output is a fixed point under itself.

**Two specs, both watched red first.** `every_row_names_the_epoch_its_baseline
_was_measured_in` fails under the off-by-one with the whole sequence shifted by
one row. `a_history_under_no_epochs_at_all_is_scored_as_measured` holds the
second copy of the naming — `divisors` has an arm for a history carrying no
boundary at all, which builds the map in one pass instead of folding across
boundaries, and the naming is written there too. Without that spec the second
copy could say anything.

**A new column reddens seven specs in a file that pins whole rows, and that
is the corpus working.** `tests/a_row_predating_a_counter_is_scored_on_what_it
_has.rs` compares each rescored row byte for byte rather than reading one
field out of it, so adding `scored_base` broke five scoring cases and both
no-column cases at once. Nothing was wrong with those specs. The rows this
tool writes are read by the chart and by the tool itself on its next pass, so
a field added to them is a change to what those readers see, and a spec that
could not tell would be the defect. Their expectations carry the new column
now, and the file says why every fixture row in it reads `as measured`.

**What this does NOT give the run side.** A rebuilt `run_instructions` has no
factor: it is a ratio to an anchor, not a division by a recorded constant. The
run half of this thread (kanso#1351) marks such a row `run_source: rebuilt`
instead. Writing a fabricated `1.0000` there to make the two sides symmetric
would read as "measured against an unmoved baseline", which is exactly the
misreading the mark prevents.

---

## 2026-09-09 — an explanation of the error model, checked against the interpreter, found three gaps

OPEN, all three cloud's; the first two are the language as designed, not shipping. Clay asked for a written explanation of how failure
works in kanso for a friend. Every claim in it was run through
`target/release/kanso play` before it went out, and three did not hold as the
design says they should. None of them is in the book, which uses only the
spellings that work; each is a place where a reader who trusts a ruling
rather than a page gets a wrong answer.

**1. The possible-none check is gated behind an environment variable, and
the gate is the defect.** `check_none_exhaustive` in src/check.rs is the
diagnostic the book's own story implies — "this can be a none and `describe`
has no arm for it — resolve it here, or give `describe` a `none` arm" — and it
runs only under `KANSO_EXHAUSTIVE`, where the 2026-07-24 none campaign left
it while it waited on per-arm return sets. Clay, on reading that here:
"KANSO_EXHAUSTIVE is of course total nonsense. you're just describing how the
language works. the exhaustiveness when you're looking for a match on an arm
has always been the way the language works since like the first couple of
days of designing it." So there is nothing to rule and nothing to wait for:
a call whose argument can be a none, made to a group with no `none` arm, is
refused at check, and the flag comes out. The program the explanation used is
the book's menu sample with the `none` arm deleted and the call moved into a
function body:

    fn describe price
      "{price} yen"

    fn quote menu
      describe menu["pocky"]

    print (quote { "dango":350 "taiyaki":500 })

    kanso play                       <none> yen, exit 0
    KANSO_EXHAUSTIVE=1 kanso play    error[exhaustive] at the argument, exit 2

The second line is the language; the first is what ships. With the arm typed
`price:int` the bare run instead dies at execution time: `error[runtime]: no
overload of `describe` matches these arguments`, and a literal `none` handed
to that typed arm fails the same way at runtime, where a literal string
handed to it is refused at check (`literal_arg_type`). The 2026-08-15 sitting
recorded the same rule as "8: exhaustiveness dissolves into per-call
coverage": each call's inferred value set is checked for an unambiguously
matching arm, provable gaps are compile diagnostics, unprovable calls keep
the runtime err. The 2026-07-24 entry "what the last exhaustiveness report
is, and is not" says the one false report left was a group-level return set
where a per-arm one would be exact, and that is an implementation detail for
whoever ungates it. The explanation as sent states the check as the language,
without the flag.

**2. The 2026-08-29 gavel "effects are types, and the words are the only
doors" is unbuilt on the point measured: a `.` over an io still binds
automatically, so a `.` step headed by `rescue` or `annotate` is swallowed.**
The ruling: there is NO automatic bind; `<t>effect` is a box that can be
passed as data; `bind`, `annotate` and `rescue` are the sole eliminators,
ordinary functions taking the effect first. Under it, `effect . rescue orders`
is the ordinary pipe supplying the first argument — `rescue effect orders` —
because the dot no longer opens the box. On the shipped interpreter the dot
still opens it: the step synthesises a bind around `rescue orders`, bind skips
on failure, and the callback is never called.

    os/read_file! "no-such-file.txt" . rescue orders . print
      error[endpoint]: unhandled err reached the executor: "cannot read ..."

    rescue (os/read_file! "no-such-file.txt") orders . print
      no orders yet

    os/read_file! "/etc/hostname" . shout . print
      vm!!                       (the retired automatic bind, still shipping)

Both of the first two parse. The first is what the ruling describes and its
handler never runs; the second is what every fixture and both book chapters
use, and it is also the ruled prefix form, so nothing published is wrong. An
earlier draft of this entry filed the piped form as a fresh ledger question,
"build the rider or retire it"; Clay's correction the same hour — "we got rid
of that rule when we agreed that you have to use explicit combinators" — is
the gavel above, and a ruled question is never re-asked. What is owed is the
build: the dot stops binding over an effect, a box where a value is expected
is refused, and the two book chapters that teach the automatic railway (ch04's
call-site short-circuit, ch05's "piping into an io is bind") move with it —
the ledger's "The book teaches the boundary language" entry already holds
that half. The ruled chain, in the shape the 2026-08-29 gavel "the chain line
keeps its dot" gave it — "the combinators look and act like regular
functions", so a continuation spells them `. rescue orders` like any other
function —

    os/read_file! "the-orders-file-that-is-not-there.txt"
      . rescue orders
      . bind shout
      . bind print

parses today and dies at the executor with the handler never called, for the
same reason as the one-line form. Under "A ruling outranks a lead" the gavel
is at the front of the queue with item 1.

**3. A stale entry in the lexer's borrowed-keyword table names `rescue`.**
`kanso_form_for` in src/lexer.rs lists `try | catch | except | rescue` with the
message "a failure rides the same rails as a value; an arm names the err",
written when the language had no such word. The table is consulted by the
needless-continuation check, so a statement headed by `rescue` and continued
on `.` lines that would fit in eighty characters reports `error[syntax]: kanso
has no `rescue``. Lengthen the first line past the fold and the same program
runs:

    rescue (os/read_file! "the-orders-file-that-is-not-there.txt") orders
      . shout
      . banner
      . print

    *** no orders yet!! ***

Remove `rescue` from that arm of the table; `try`, `catch` and `except` stay.
The error corpus has no fixture for a worded step at a statement head, which is
why the message survived the word's arrival.

**One thing the check confirmed rather than broke**, recorded because it is
the sharpest demonstration of the own-err rule found so far and belongs in the
book's boundary chapter when that is written. Annotating a foreign failure
makes it yours:

    rescue (annotate (os/read_file! "no-such-file.txt") said) orders . print
      error[endpoint]: unhandled err reached the executor: "orders file: ..."
        born in the entry at s2.kso:13
        passed through orders

The raw read error was foreign and `orders`' `(err _)` arm caught it a line
earlier. `annotate` raised a new err in the entry's own module, so the same arm
is now walked past. That is the rule working exactly as ruled, and it is the
example to teach it with.
The two rises read 0.28 -> 0.74 with the score 88.28 -> 62.67, and 0.74 ->
1.00 with 64.95 -> 56.73, measured against origin/perf-history rescored by
this tree — which is what CI writes on the next push to main.

## 2026-09-09 — a number is written into the accumulator, and an arm is told what the switch decided

**DONE.** `append acc "{n}"` is how the JSON encoder writes every number, and
the template rendered `n` into a string for the one purpose of copying its
bytes into `acc` and dropping it: 379,530 strings a runbench, built to be
copied once. The render alone was 5.50% of the run program's instructions
(`k_b_render_value` inclusive, 130.1M), and the ryū and itoa digit cores
under it are about 111M of that and stay — what goes is the string around
them, the dispatch that reached it and the copy out of it.

Two changes, and the first does nothing without the second.

**The emitter fuses the pair.** `append acc "{x}"` with `x` proved a number
emits one call to `k_b_append_rendered`, which writes the digits into a
64-byte stack buffer through `k_render_number` — the int and float arms of
`k_render_at`, extracted so the render and the door share one writer — and
copies them into the accumulator from there. Where the accumulator has 24
bytes to spare the copy is three words whatever the length (a number is at
most 24 bytes, `-1.7976931348623157e308`, and the buffer holds 64), so the
call into memcpy for a handful of digits is not made. A value the ambient
`render/to_string` group could claim keeps the dispatch the template would
have made, so a user arm is never skipped, and every other tag the door
hands to `k_render` itself, so the bytes cannot differ from the unfused
spelling; the differential goldens hold that across all three engines.

**An arm is told what the switch decided.** The first build of the fusion
emitted zero fused sites on runbench. `encode_onto`'s `n:int` arm is reached
through the tag switch (§ the tag switch, 2026-09-05), which has already
proved the tag is 0 — and inside the arm the discriminator still carried the
whole group's set, REC and DESC and THUNK included. So the body forced `n`
again, asked whether a user `to_string` arm could claim it, and took the
generic door on every builtin it handed `n` to. `arm_tags_set` now records
the case's tags as the discriminator's set for the arm's body — INT for the
int arm, FLOAT for the float arm, REC for a record arm, the nullary's tag for
its arm — and restores the whole set after. Both number arms fuse, and the
`k_force_fast` in front of each is gone.

On the container: runbench **2,368,295,010 -> 2,347,625,055** (−20,669,955, −0.8728%),
the same bytes out, `append_rendered` 379,530, and `append_fast`,
`append_grow`, `ryu_renders` and every allocation and peak counter identical.
The two mutations measured alone say which half is which: the fusion off
with the arms narrowed reads 2,366,863,311 (−1,431,699, the narrowing's
own gain on the rest of the program: the forces and dispatches it removes
from every switch arm), and the arms un-narrowed with the fusion on reads
2,368,105,221 (−189,789) — with the whole set inside the arm the fusion is
never eligible and every site takes the fallback, which is the byte twin the
generic path always took. Neither half is the win; the pair is, because the
first cannot fire until the second tells it the tag.

**Two wrong cuts on the way, both in the profile.** The first build read
+0.15%: `k_render_number` and `k_b_append_range` were both out of line, so
the fused door paid two calls where the old path paid one, and
`k_b_append_range` alone rose 3.2M -> 22.2M. Both inline now. The second was
the fallback arm — the door the fusion takes when the value is not a number
or a user arm could claim it — which called `k_b_append_mut` directly and
paid 22,789,710 in `k_b_append_wide` on runbench, where the generic path
would have reached the byte twin, whose string arm copies inline. The
fallback takes the twin. Neither mutation could be read until that was
fixed: both mutants measured +0.9% over main, the same 21M, for a reason
that had nothing to do with what they mutate.

**Rows and pricing.** `append_rendered` joins the trend gate's higher-is-
better list beside `append_fast`. Ratchet rows `append_rendered` and
`arm_narrowed`, mutations `a_scalar_rendered_into_a_string_and_copied_again`
(the fusion predicate reads false) and `an_arm_not_told_what_the_switch_
decided` (the narrowing filters to nothing), both on the instructions gate.

**What #456 found, recorded here because this is its first consequence.**
Read against the objective's own marginals, the queue since 2026-09-08 had
been working the dimension holding a tenth of the headroom. A ten per cent
improvement is worth 0.76 points on run speed, 0.64 on run memory, 0.19 on
compile speed and 0.19 on compile memory; the remaining points are 18.34,
9.70, 3.46 and 2.20. The last 26 merges moved welfare 0.1115 in total, and
25 of them were compile-side. The run program's profile puts 23.2% of its
instructions in two loops, `encode_onto` (13.38%) and `value_for` (9.86%),
and this entry is the first of what that map says to do.

**The round CI returned, and the sweep defect under it.** Three jobs went
red on the first run. The cost goldens read `46a47 > sh_bytes=...` on all
twelve: main had gained `sh_bytes` between the branch and the merge, the
merge kept this branch's goldens, and `all_counters.sh --write` then reported
`rewrote` twelve times and changed no file. Its rewrite replaced data rows
line for line and stopped at the golden's last row, so a counter the runtime
had just gained — one more row than the golden held — was dropped, silently.
The rewrite is now `scripts/gates/keep_header.sh`, which appends the measured
file's remaining rows, and `tests/the_sweep_write_keeps_a_new_row.rs` runs it
on a postcard-sized golden and was watched red on exactly that case. The
book's two counters panels drifted by the `append_rendered` row and are
rewritten. The diagnostic scan found `k_render_number: not a number` with no
golden; it is the render's `default:` arm and neither caller can reach it,
so it is listed in `tests/golden/unpinned_diagnostics.txt` with the control-
flow argument. CI's rows: runbench 2,392,210,251 -> 2,369,642,706 (−0.9434%),
encodebench −1.9672%, livebench −2.7667%, oneshot −1.1334%, widebench
−2.0173%, jsonbench −0.0192%. The three compile rows rose, layout moves on
a compiler whose emitter grew: `compile_instructions` 48,746,192 ->
48,749,059, `entry_instructions` 162,042,653 -> 162,051,772,
`library_instructions` 162,839,321 -> 162,846,242. The machine code grew
with it, `text` 1,487,564 -> 1,494,508 summed over the fourteen binaries,
6,944 bytes for the fused door and its fallback twin. Welfare 66.30 ->
66.37, banked.

## 2026-09-09 — read_file is text and read_bytes is bytes, on every engine

**Built:** the 2026-08-29 ruling (archive, "gavel: read_file is text,
read_bytes is bytes, per precedent"), after eleven days on the unbuilt list.
`read_file` refuses a file whose bytes are not utf-8, with one sentence on
all three engines: `cannot read {path}: the bytes are not text`. Until now
native handed the bytes through as a string and the interpreter refused them
with its own words, so the same program answered differently by engine.
`read_bytes` is the other reader: it hands the bytes back as they are, and
takes the same two doors as `read_file` — `read_bytes` answers
`file_not_found` as data and `read_bytes!` insists. `write_file` and
`net_write` accept bytes, so a program can read a binary and write it back
or serve it.

**Where the bytes go.** The http library's `rendered` interpolated the body
into one string, which renders a bytes body as a list. The status line and
headers are now a preamble, `delivered` writes a string body in one write
and a bytes body in two, and the content-length is measured on `as_bytes`,
which is a text body's utf-8 or a bytes body itself. The browser differential
and the fingerprint script read `docs/kanso.wasm` through `read_bytes!`
rather than through a text read that only worked because nothing checked.

**Fixtures.** `tests/golden/micro/a_file_that_is_not_text.kso` reads a
five-byte file — `ff fe 00 41 80`, the first byte one no utf-8 sequence
begins with — both ways and prints the refusal and the five bytes; the
harness runs it on both engines and through a release build.
`tests/golden/runtime/read_bytes_takes_a_path_string.kso` pins the builtin's
own refusal of a non-string path, reached through a binding the check cannot
see through. The 63-entry arity table, the 59-name builtin list and the
56-call codegen table each grew by one, and the descriptor tag is 30.

**Two things found on the way.** A definition added to `std/os` above
`insisted` moved the line four runtime goldens quote (`os.kso:113`), so the
new readers sit below it. And a helper named `head` in `lib/net/http`
collided with four parameters of that name; the check refuses the shadowing,
which is the right answer, and the helper is `preamble`.

**The emitted code moved, and the sweep saw it.** `all_compile.sh` read
`emitted_code` MOVED on eight programs, every one that imports `std/os`: the
decoder itself in `bench/emitted_golden.txt` (defines 140 -> 142, calls
1,232 -> 1,236, lines 9,219 -> 9,245) and seven of the thirteen in
`bench/emitted_golden_others.txt` — encodebench, oneshot, widebench,
digestbench, readbench, livebench and runbench — each up two defines, four
to six calls and 26 to 28 lines, with every branch count identical. The two
defines are `read_bytes` and `read_bytes!`, which a program importing the
module carries whether or not it calls them; none of the eight does. A rise
on this vein is a regression to explain, and this is the explanation: the
library grew two definitions and the emitter carries a module's every
definition, as it did when lib/json dropped std/list and the veins halved.
Both goldens are regenerated with their headers kept. The first reading of
the sweep's output saw only the second file's diff and wrote that the
decoder had not moved; the re-run after regenerating that file said
otherwise, which is what the re-run is for.

**Rulings weighed.** STATUS.md's "Ruled, unbuilt" list (kanso#1353) holds
twelve rows. This is the smallest that touches every engine, and the one a
reader hits first: the book's boundary chapter cannot describe two readers
until both exist, and `read_file` handing bytes through on native while the
interpreter refused them was a divergence the differential law forbids. The
other eleven — effects as types, err readers, the fused chain operators,
pure fallibility boxed, `done`, exhaustiveness without the flag, a
qualified name as its module's declaration, records printing qualified,
the backends' partial over a value, block-born as the whole cohort, and the
boundary chapter — stay in the order the list gives them; the next build is
taken from it.

**Round three: the counter the local sweep never ran.** CI's cost-goldens
job, once the runner's apt mirror stopped returning a mismatched index, read
`utf8_bytes` up on eight runtime veins by the size of the input each reads:
188,698 on the decoder, encode, oneshot, wide, read, live and run programs,
8,192 on the digest. That is the ruling's own cost made visible. `read_file`
validates every byte it hands back now, where native used to hand bytes
through unread, and the counter that counts validated bytes counts the input
file. The allocation counters beside it are byte-identical. The twelve cost
goldens were regenerated with `all_counters.sh --write` on this branch, which
the first two rounds skipped — the emitted sweep ran and the counter sweep did
not, and CI found the difference. The retired-instruction rows CI measured are
copied in: runbench 2,369,642,706 -> 2,369,917,628 (+274,922, +0.0116%), the
decoder +274,897, encode +274,968, oneshot +274,897, read +274,804, live
+274,897, wide +90,524, digest +6,153; basket, deepbench, escapebench,
pendbench, indexbench and scanbench read no file and hold to the instruction.
That is 1.46 instructions a byte for the validator, the word-at-a-time arm the
2026-09-07 entry measured. Every `.text` row rose too, 1,040 bytes on the
eight readers and about 3,056 on the six that do not read, since the reader
pair and the refusal are runtime code every program links. The compile rows:
compile_instructions 48,749,059 -> 48,751,741, entry 162,051,772 ->
162,061,812, library 162,846,242 -> 162,857,425, each the two definitions
`std/os` gained. Welfare reads 66.3705 against a floor of 66.3715, a fall
of 0.0009 that the gate's band holds; the floor is re-set to it with
`--set` all the same, because the trend gate's pure-regression rule reads
this branch as worse on twenty-six counters and better on none, and lets
that through only when welfare_floor.json's history names the change that
spent it. It is the differential-law exception welfare.kso states: a
`read_file` that answers the same on three engines is not a trade.

**Priced, row by row, for the trend gate.** The counters the branch moved and
the values they landed on: `utf8_bytes` 11,164,198, `run_utf8_bytes`
24,415,348, `encode_utf8_bytes` 75,741,068, `live_utf8_bytes` 75,741,068,
`oneshot_utf8_bytes` 450,566, `wide_utf8_bytes` 289,183, `read_utf8_bytes`
188,698, `digest_utf8_bytes` 8,192; `work_jsonbench` 1,485,334,799,
`work_encodebench` 3,963,988,526, `work_oneshot` 21,737,118, `work_widebench`
35,332,240, `work_digestbench` 10,426,549, `work_readbench` 4,561,941,
`work_livebench` 3,481,899,703, `work_runbench` 2,369,917,628;
`emitted_defines` 142, `emitted_calls` 1,236, `emitted_lines` 9,245,
`emitted_other_defines` 2,353, `emitted_other_calls` 20,516,
`emitted_other_lines` 133,241; `text` 1,523,196; `compile_instructions`
48,751,741, `entry_instructions` 162,061,812, `library_instructions`
162,857,425. Each is the validator, the two readers or the refusal, and the
paragraph above says which.

## 2026-09-09 — an err answers `.reason`, `.cause` and `.origin`, on every engine

The 2026-08-29 gavel "an err has readers" (archive), built. STATUS.md had
carried it as unbuilt since the sitting: `annotate e (err -> "config:
{err.reason}")` — the gavels' own sample — was refused at check time with
`no record type has a field reason`, and a callback holding an err could
look at nothing inside it.

**Built.** A field read desugars to a getter call, `Get_reason e`, as every
field read does, so the reader lives where a getter is entered. Each engine's
dispatcher answers an err at its ENTRY, before any arm is tried: the
interpreter in `dispatch_loop_inner`, native in the prologue `emit_reader_hole`
writes for both dispatcher shapes (`k_is_err` then `k_err_read`), the page
with `rt_err_read` in `emit_dispatcher`. `reason` is the value the err was
raised with; `cause` the err it wrapped, or none; `origin` the "{fn} at
{file}:{line}" it was born at, or none for an executor-born one. The
interpreter's `err_read` is the oracle and the wasm host calls it; native's
`k_err_read` mirrors it arm for arm. The entry is the only place that works:
a placeholder arm `(err Read)` matched a foreign err before the failure
pass-through and would have answered the reason to `.cause`, and an
own-hako err is one no arm may see, where a reader has to see both.

**The group has to exist.** A read of a field no record declares resolves to
no getter at all, so `desugar_field_reads` now synthesises one arm per reader
field nobody declares. Synthesised per module, as the record getters are, it
produced one identical arm in every module and the merge refused the overlap;
it runs once over the merged program instead. The checker's declared-field
set gains the three names so the read passes the `no record type has a field`
fence, and a record reaching a reader group still gets the field error every
getter gives.

**Fixture.** `tests/golden/micro/an_err_has_readers.kso`, settled failures
only, so the page runs it too: a plain err's reason, an annotated err's reason
and its cause's reason, `.cause` of an unwrapped err (`<none>`), a cause read
through a second `rescue`, the origin's function name on both, and json's
decode failure read as `e.reason.reason` — the case where a reader and a
record field share a name. Watched red on the checker first. The origin names
the raising function qualified when the program is imported, so the fixture
carries an `.imported.out` twin like the record-printing ones. Two things the
fixture taught while it was being written: `print none` writes `<none>`, and a
named group handed the err passes it through as ever, so the readers are
applied inside the lambda and the group gets the piece.

**Veins, CI's rows.** The readers sit at every dispatcher's entry and the
checker's field set grew by three names, so the front end carries them:
`compile_allocs` 29,606 -> 29,714 (+108), `compile_instructions` 48,751,741 ->
48,820,126 (+68,385, +0.14%), `entry_instructions` 162,061,812 -> 162,734,847
(+673,035, +0.42%), `library_instructions` 162,857,425 -> 163,022,357
(+164,932, +0.10%). The compile peak (773,818), every runtime vein, the
emitted, text and machine-code rows are byte-identical. Priced, row by row,
for the trend gate: `compile_allocs` 29,714, `compile_instructions`
48,820,126, `entry_instructions` 162,734,847, `library_instructions`
163,022,357. Welfare reads 66.36 against the 66.37 floor, a fall of 0.01 the
readers pay in compile cost with nothing offsetting it; the 2026-08-25 ruling's
language clause applies, so the floor moves down to the reading, 66.3596,
by hand in `bench/welfare_floor.json` with its history entry (`--set` refuses
a fall of this size by design, and says the file is the door), and no
optimisation rides along to hide the price.

---

## 2026-09-10 — gavel: rows 15..390 stay unscored, and re-measurement is a lead with a probe in front of it

On "The reconstruction the 2026-09-07 ruling ordered has two usable phases,
not four, for 376 of the 439 rows", Clay: "I'm okay being pragmatic and
taking the first one." Ruled: option (1). Rows 15..390 carry no run terms and
get none reconstructed; they stay scored on the compile terms they carry, at
the coverage the rescore already stamps on them, and the chart keeps drawing
the coverage boundary at 2026-09-03 the way kanso#1346 draws every boundary.
The eight-phase half (rows 391..438) is built and stays. Nothing further is
owed on this entry; it leaves the ledger with this ruling.

Clay's second sentence is a question, and it is answered here so it is not
re-derived: "isn't it pretty trivial to just rerun the current metrics on the
old versions?" Mechanical, not trivial, and one unknown decides whether it is
possible at all. The mechanics: for each of 376 commits, `cargo build
--release` at that commit, `kanso build bench/runbench` with the compiler it
produced, one callgrind run for `run_instructions` and one counters run for
`run_peak_bytes`, on ONE host in ONE sitting that also re-measures HEAD, so
every row is on the same ruler by construction. About four minutes a commit
with a warm cargo cache, so a day of serial machine time for all 376, or an
afternoon sampling every fourth. The unknown: whether a compiler from
2026-08-10 accepts today's bench/runbench source. The surface moved between
then and now — the bang choosing the channel, `done`, the consolidated run
program itself — and nobody has tried. So the lead is filed with its probe
first: build the compiler at row 15's commit, compile today's runbench with
it, and read the answer. Yes means the sweep is a script and the rows come
back as `run_source: remeasured`. No means a runbench pinned to the old
surface, which measures a different program and would need its own ruling.
This is a lead, not a ruling: it goes on cloud's list as a lead and stays
off the "Ruled, unbuilt" section unless Clay says the word.

## 2026-09-09 — block-born is a proof, not a spelling

Built: the 2026-08-29 ruling "block-born is the whole cohort" (STATUS.md,
now removed; the archive entry of that name, Clay: "okay whole cohort it
is"). A field write's target had to be a name bound directly to a
construction in the same `build`; `twin = ada` followed by `twin.partner =
bob` was refused with the syntactic fence, and the 2026-07-28 measurement
found the same refusal on a node chosen by an `if`, a node taken out of a
list the block built, and a node reached through a field. The theorem never
asked for the fence. It asks that the cohort be closed, and every one of
those four values is inside it.

**What the checker proves now.** `check_build_blocks` used to carry a set
of names; it carries a cohort. Each value the walk proves born is an entry,
and a name, a field or an element holds an entry. A construction is born,
and its entry records which of its fields hold born values, read off the
constructor's arguments by position. A name bound to a born name shares the
entry, which is what makes an alias an alias: a write through `twin` is a
write to `ada`'s entry, and `ada.partner` read afterwards is the value that
was written. A list or map literal is born, and its elements share what
every element has; an empty literal shares nothing. An index of a born
literal is that shared entry, a field of a born value is the field's entry,
and a constructor pattern hands each position the matching field. An `if`
whose arms are both born is the meet of the two: a field or an element read
through it is read through to both arms and is born only when both answer,
so a later write to either arm is seen. A write made through the choice
reached whichever arm was taken, so both arms forget the field and the
choice remembers the write. That last rule is the one a reader is likely
to question, and `build_write_a_field_an_if_may_have_overwritten` pins it:
`other = cell "other" young`, then `chosen = if … other young` and
`chosen.link = old`, and `other.link` is no longer proved, because at run
time it may hold `old`.

A write inside an `if` arm's statement list may not have happened, so it
takes the field's proof away rather than supplying one. Anything a call
answers is not proved, a parameter is not, a name from an enclosing block
or an earlier iteration is not, and the six escape fixtures from July stand
unchanged beside four new ones: an `if` with an older arm, an element
beside an older one, a field a constructor filled with an older value, and
the overwritten field above.

**Two spellings of a field read.** The module route rewrites `b.up` to its
getter, `Get_up b`, before the check runs, and the play route after it, so
the walk reads both: `kanso check` on the fixture said ok while the same
program imported was refused at `over.id = 10`, and the getter arm is why
it is not.

**The write form is unchanged.** `target.field = value` with a name on the
left is still the one form, per the 2026-07-19 design; what widened is how
the name's birthday is proven. `ring[2]!.id = 20` does not parse and does
not need to: `middle = ring[2]!` then `middle.id = 20` is the same write.

**What it does not reach, and what that leaves.** Every algorithm the
2026-07-28 entry named — union-find's path compression, an e-graph's
rewire, unification binding the variable it found — reaches its node
through a call: `find` is a recursive function and its node is a parameter.
The four flows admit the shapes the entry measured and not the algorithms,
and design/memory-frontier-research.md's 4.4 row says so. Birth flowing
through a call is the next widening of this analysis, mine, and it is not a
new question.

**Spec.** `tests/golden/micro/a_build_writes_what_it_can_prove_was_born`
runs on all three engines: the alias, the field a constructor filled, the
field a write set, the indexed element and the chosen node, each written
through, then printed. Refused by the old compiler at the first of them.
The four escape fixtures are in the error corpus with their imported twins.
The diagnostic scan reads 313, none newly unpinned; the message is the one
July wrote, since a parameter is still not a construction made in the
block.

**Veins.** CI's sitting on the host-keyed compile rows, copied in:
`compile_instructions` 48,820,126 -> 48,820,567 (+441, +0.0009%),
`entry_instructions` 162,734,847 -> 162,736,962 (+2,115, +0.0013%),
`library_instructions` 163,022,357 -> 163,024,919 (+2,562, +0.0016%). The
cohort walk is a few hundred instructions more than the set walk on every
build block the corpus compiles, and the three routes carry the same
checker. `compile_allocs` and `compile_peak_bytes` did not move; nothing on
the runtime side did. Welfare reads 66.3596 either side, a fall of 0.00002
that the trend gate reads as a pure regression, so the fall is recorded in
bench/welfare_floor.json's history under the 2026-08-25 language clause,
the floor 66.35962 -> 66.35960.

## 2026-09-09 — a qualified name is its module's declaration

The 2026-08-29 ruling of that name (archive), built. STATUS.md carried it as
unbuilt; the log's 2026-09-08 entry "the reorder refuses a valid program"
declined it twice for want of the mechanism, and module_differential carried
the shape as its one `known_wrong` entry.

**What was wrong.** A module that declares `pub fn join` while importing
`std/text` holds two kinds of declaration under the bare name: its own arms and
the bare-enrollment twin of text's `join`. The loader qualified both to
`dep/join`, first writer won, and the first writer was the twin whenever the
import loaded first — so the module's own `pub` read as private from outside,
and the compiler's answer to that was an opacity refusal, "an import of `dep`
exports `join` too and took the name". With the refusal lifted the hazard was
worse than the refusal: a consumer's `dep/join ["x" "y"] "-"` reached std's arm
under dep's name and answered `x-y`, while dep's own arm could never be reached
at all.

**What the ruling says.** `dep/join` is dep's own `join`, and only that. Inside
dep, a bare `join` still dispatches over dep's own arms and the import's
together, because that is what an import is for.

**How.** The bare overload space of a mixed name moves to a spelling no
consumer can write: `dep/~join`, `ast::bare_space`, the mark being `~`, a
character the lexer never makes part of a name. In `qualify`: the module's own
bare declarations and its imports' twins are collected, `mixed` is their
intersection, and `owned` becomes a map from a bare name to the spelling its
call sites are rewritten into — `dep/~join` for a mixed name, `dep/join` for
every other. A twin in a mixed group takes the bare-space spelling, loses
`pub`, and holds no claim on the qualified one; the module's own arm keeps
`dep/join` and gets a synthetic non-pub clone in the bare space, so an
inside call finds both. The two-claims bookkeeping (`shadowed`, the
"took the name" message, the `Some(false), false` arm) goes, and `Loaded`
loses its third element. `Diagnostic::new` runs every message through
`ast::spoken`, which strips the mark, so a sentence about the bare-space
group says `dep/join`, which is what the author wrote.

**Spec, watched red on the pre-ruling compiler.** module_differential's
`known_wrong` is empty for the first time; its one entry is now case c30, a
module's own pub sharing a name with a dependency's, printing `OWN(x,y)` on
both engines. Two cases beside it: c31, a list handed to `dep/join` where dep's
arm takes an int, refused by the check as `no arm of `dep/join` takes a list
here (arms take int)` rather than dispatched into the import; and c32, a bare
call inside the module reaching the import's arm (`x-y`) beside the qualified
call reaching dep's own (`OWN(1,2)`). On the pre-ruling compiler all three fail
with the opacity refusal, 32 modules 3 wrong. The same hazard is pinned byte
for byte in `tests/golden/errors_module/a_list_handed_to_a_modules_own_int_arm`,
two diagnostics at columns 17 and 27, where the old compiler wrote the opacity
refusal at column 8. The page's §12 paragraph, which described the "took the
name" diagnostic as the settled answer, now describes the ruling.

**Verified on the container.** clippy, rustfmt, the errors_module corpus, the
golden suite, reexports, the wasm engine walk on a fresh blob, the unit tests,
the diagnostic scan (313 literal diagnostics either side, 0 newly unpinned), `all_counters.sh`
(the twelve cost veins and the lazy tier agree), `all_pages.sh` (three gates
agree; the §12 paragraph is the page edit), module_differential 36 modules 0
wrong.

**Veins.** `emitted_code` moved on two of the thirteen others, scanbench
19,752 -> 19,764 and runbench 34,806 -> 34,828 lines (the summed key
`emitted_other_lines` 133,241 -> 133,275). Against round one's tree, built
on this box for the comparison, the difference is dead text: each program
interns the raw name of a bare-space group (`scanbench/~split`, `split/~split`,
`runbench/~total`) beside the spoken form, and nothing in the IR names the
constant; runbench also carries a second copy of the capture-free thunk for
pend's `cards -> spent cards`, the lambda in `pend/total`, whose twin sits in
runbench's `~total` group, and neither copy is named either. `emitted_other_defines`
2,353 -> 2,354 and `emitted_other_calls` 20,516 -> 20,517 are that thunk.
Defines, calls and branches hold on scanbench. For the trend gate, the landed
host rows, CI's sitting on round two's commit: `compile_instructions`
48,820,567 -> 48,849,164 (+28,597, +0.0586%), `entry_instructions`
162,736,962 -> 162,730,512 (−6,450, −0.0040%), `library_instructions`
163,024,919 -> 163,431,666 (+406,747, +0.2495%). Round one's sitting read
48,784,668, 162,434,379 and 163,223,647; the reorder and the file test cost
the difference. The lines are the string table: `std/regexp`
declares private `first`, `spread` and `repeat` while importing `std/list`,
which exports all three, so those are mixed groups now, and the own-only
`regexp/spread` group's name and dispatch sentence are interned beside the
bare-space group's. Before the scrub the same table carried `regexp/~first`
and `no overload of `regexp/~first` matches these arguments`, which is how
the mark was found to reach a message a program can print. The decoder's
golden did not move. The host-keyed compile rows are CI's: `qualify` walks
one more set per module and clones an arm per mixed name, and the same-box
pair below is the container's projection, not the row.

Same-box pair, callgrind on the gate's boxes, #461's tree against this one:
module 50,441,411 -> 50,397,314 (−44,097, −0.0874%), entry 167,441,542 ->
167,533,649 (+92,107, +0.0550%), library 168,198,150 -> 168,348,197
(+150,047, +0.0892%); `compile_allocs` 29,714 -> 29,262 (−452: −465 for the
two-claims bookkeeping and the `shadowed` set gone, +13 for the vectors the
reorder builds), `compile_alloc_bytes`
4,806,203 -> 4,826,630 (+20,427, the clones), `compile_peak_bytes` 777,308
identical, rounds 66 and visits 22,133 identical. The allocation fall is
worth about 0.03 on the index by the per-term arithmetic in the 2026-09-09
read_bytes entry and the instruction move is inside the band, so the
projection is a small rise, to be banked with `--set` once CI's rows are in.

**Round one's kq failure, and two fixes on the bare space.** CI's kq job died
in the scale gate with `` `-` is not defined for these values ``: a bare call
inside a module that declares an arm over a name one of its imports exports
reached the import's arm. Dispatch tries a group's arms in declaration order,
and before the ruling the module's own arms stood ahead of the twins
`enroll_bare` appends, so a bare call both could take reached the module's
own. The clones were appended after the twins, which flipped that. c34 is the
shape: a module declaring `sum` over `list/sum` and calling `sum` bare from
`twice` printed `12` where the ruling's compiler prints `-12`, watched red.
The clones go in ahead of the twins now, each group of them in front of the
first twin of its name. Putting the two side by side found the second defect:
`check_constants` walks consecutive runs of one name, and a module's own
constant `bytes` beside the twin of `text/bytes` (the fold fixture #1349
added, run as a library by the golden suite) is a run holding an arity-0 arm,
refused as `a constant admits no overloads`. It had passed by accident, the
clone and the twin never adjacent. The check reads the arms the module wrote,
since the bare space is a union nobody declared and the module's constant
already stands alone under `dep/bytes`; c35 pins the shape and is red under
the reorder alone. The reorder found a third, in the golden suite's library
twins of `an_operand_that_varies_still_loops` and `trmc_count`: trmc writes
its loop wrapper as a synthetic arm under the module's own name, and qualify
read every synthetic bare arm as an import's twin, so `weigh` was a mixed
group in a module that imports nothing and the wrapper went to the bare space
as if it were std's. With the twin last, the bare call still reached it; with
the module's own arms ahead, the plain recursion ran first and the stack ran
out on every engine. A twin is now a synthetic bare arm from a file the module
does not own, `own_files` being the files of its non-synthetic bare
declarations; the wrapper keeps its qualified spelling and gets its clone
like any own arm. c36 pins it, a counted recursion inside a module called bare
a million deep, red under the reorder alone. The three compile rows below are
CI's reading of round two's commit, copied in.

**What stays in STATUS.md's "Ruled, unbuilt", and why.** Read whole before
this PR was chosen: block-born landed in #1359; this row, records printing
qualified and the partial over a value are a stack of three, built and
verified in worktrees, landing in that order; `done` and the fused chain
operators follow, with the plain dot untouched. Three rows are built and
cannot open yet. The effect type's two halves (the plain dot as an
application, the `<t>effect` spelling) and the exhaustiveness check on arm
match are one design with the fused operators: each breaks binds or arms in
kq the way #1357 did, kq's respellings live in kq, and kq is outside this
session's reach, so they wait on that respell and their rows stay. The
pure-fallibility rider waits on the ledger's "where the box wraps" ruling.

## 2026-09-09 — records print qualified, everywhere

The 2026-08-29 gavel of that name (archive), built. STATUS.md carried it as
unbuilt: `tests/entry_file.rs` still ignored the test the gavel said would
flip, and it still expected the unqualified form.

**What was wrong.** A record printed its type's spelling as the compiler
held it, and the compiler held two: a module reached through an import has
its declarations qualified (`lane/slow_lane`), and the same file run directly
does not (`slow_lane`). So `slow_lane 7` printed one way from `kanso play
lane.kso` and another from an entry importing `lane`, and the micro and mem
corpora carried `.imported.*` twins for exactly that divergence.

**The ruling.** Go's `main.T`: a record prints its module-qualified name
whatever the entry path, and the root module takes the name an importer
would write for it — a file's stem, a directory's name.

**How.** `ast::Program` gains `root`, set where each compile path returns:
the entry and play paths, the library path and the module path, from the
file's stem or the directory's name. Rendering is the only reader.
`eval::set_root`, called when an interpreter is built over a program,
records the root and the set of bare types the program declares;
`eval::shown_type` qualifies a type in that set and leaves every other
spelling alone, which is how the compiler's own `entry` record — the map
entry, declared by no module — stays `entry` on every engine. The
interpreter's `render` and the page's host (which renders through it) read
that; native gets a second table beside `k_type_name`, `k_type_shown`,
emitted by codegen and read only by `k_render_at`, so the identity the
runtime matches on and the field-error messages are untouched. Diagnostics
are untouched everywhere: a checker sentence names the type the author
wrote.

**Watched red.** Before the root reached the entry path, `kanso play
alone/lane.kso` printed `slow_lane 7` while `kanso run main.kso` printed
`lane/slow_lane 7`, on both engines; with the root threaded, both print
`trouble: [err lane/slow_lane 7] lane/slow_lane 7`. The first cut prefixed
every bare type, and the interpreter and the page printed `maps/entry` where
native printed `entry` — the wasm walk caught it on `examples/maps.kso`; the
set of declared types is the fix.

**Spec.** `tests/entry_file.rs`: `a_record_prints_qualified_whatever_the_entry_path`
replaces the ignored test, runs the fixture's direct twin (`alone/lane.kso`,
bare statements) and its imported form on both engines, and asserts all four
print the same qualified line. The fixture is rewritten to the modern shape:
the old one caught its own err with an arm, which the 2026-08 err rulings
retired, and `pub play` files refuse `kanso play`. Five example goldens
regenerate (`build_blocks`, `field_typesets`, `markers`, `none_is_a_value`,
`record_render`), each a prefix and nothing else; one book sample
(`appb/wrap_err`, an unhandled `config_bad`) and its panel. The micro, mem,
runtime and error corpora did not move: their `pub play` fixtures were
already reached through a staged import, which is where the twins came
from.

**Verified on the container.** rustfmt, clippy, the golden suite (eleven
tests with the twin spec), entry_file, errors_module, reexports, the unit
tests, the wasm engine walk on a fresh blob (twelve), `book_check.sh` (every
sample verified), the diagnostic scan (311 literal diagnostics, 0 newly
unpinned), `all_counters.sh` (the twelve cost veins and the lazy tier agree),
`all_pages.sh` (three gates agree).

**Veins.** `emitted_code` moved on every program by two lines and nothing
else — the decoder's `emitted_lines` 9,245 -> 9,247, the thirteen others'
summed `emitted_other_lines` 133,275 -> 133,301 (+2 each), defines, calls
and branches identical — and `compile_cost`'s five samples by one line
each, `lines` 6,094 -> 6,099, with `module_lines` 5,268 -> 5,269. The lines are `k_type_shown`: a program whose
root declares no bare type prints every record under the identity's
spelling, so the table is an alias of `k_type_name` rather than a second
switch, and the benchmarks are all that shape. The first cut emitted the
switch unconditionally and cost each program a define, a branch and thirty
to sixty lines. Every runtime vein is byte-identical: the render calls one
table where it called the other. The host-keyed compile rows are CI's;
`qualify` is untouched, and the same-box pair against #1360's tree is below.

Same-box pair, callgrind on the gate's boxes, #1360's tree against this one:
module 50,397,314 -> 50,386,179 (−11,135, −0.0221%), entry 167,533,649 ->
167,577,852 (+44,203, +0.0264%), library 168,348,197 -> 168,341,016 (−7,181,
−0.0043%): the root's name and the shown-name table are a few thousand
instructions either way, inside the noise of a rustc relayout.

**The build_cycle fixture is measured through the import now.** Folding
its `.imported.mem` twin leaves `tests/golden/mem/build_cycle.mem` holding
the numbers the staged import always produced, so the trend gate reads a
re-basing rather than a move: `build_cycle_alloc_bytes` 2,576 -> 3,168,
`build_cycle_perm_allocs` 7 -> 9, `build_cycle_sh_str` 1,936 -> 2,528,
`build_cycle_sh_buf` 176 -> 208, `build_cycle_thunk_allocs`,
`build_cycle_thunk_evals` and `build_cycle_thunk_live_exit` 0 -> 1 each,
`build_cycle_carry_dedup` 0 -> 1. The direct route's numbers were the
twin's other half, and the direct route is gone.

**A seventh twin, met on the rebase.** #1356 landed
`tests/golden/micro/an_err_has_readers.imported.out` while this sat in its
worktree: the fixture printed an err's origin with its file stripped, and an
import qualifies the raising function's name, so `boom` read
`an_err_has_readers/boom` through the harness's staged entry. The twin
went the way of the other six. The probe strips the module's slash too,
since what it asks is which function raised the err, and the qualified
spelling by route is the runtime corpus's accepted divergence, kept there
in its `.imported.stderr` twins.

One spec had pinned the old spelling for the root's own records:
`tests/a_type_name_as_a_bare_value.rs` expected a nullary record declared
in the entry file to print bare, `unit`. Under the ruling the root is a
module like any other and its name is the file's, so the line reads
`nullary_native/unit` on native and `nullary_oracle/unit` on the oracle,
one per staged file name; the expectation moved, found when the queue was
stacked on one worktree on 2026-09-10.

**Round one's compile rows, and the got file CI could not show.**
`compile_instructions` 48,849,164 -> 48,866,385 (+17,221, +0.0353%),
`entry_instructions` 162,730,512 -> 162,751,831 (+21,319, +0.0131%),
`library_instructions` 163,431,666 -> 163,473,663 (+41,997, +0.0257%), CI's
sitting on round one, copied in and noted in each golden. Three rows in one
band, on a change whose only per-program work in the front end is the root's
own String: the layout reading the compile goldens' older notes carry.
`compile_allocs` also disagreed, and its value is not in this entry because
nothing could reach it. The gate refuses to measure on a host the golden does
not name — this container is rustc 1.94.1 against the golden's 1.98.1, and it
says in as many words to let CI measure and copy the rows out of the job log.
The job log's own step for that, "the rows as measured here, to copy into the
goldens", named three got files by hand and there are five;
`allocs_got.txt` was one of the two it missed, and the gate's own message sits
fourteen gates and two callgrind dumps from the end, past what the log API's
tail returns. So the step globs `*_got.txt` now. The hand-written list
arrived with the first compile vein in #1214 and has been extended by hand
twice since, at #1330 for the entry row and #1337 for the library row, and
#1337 paid a round for the omission; the glob is what stops a third.
`compile_allocs` stays deliberately red this round, and round two prints the
number this one could only have guessed.

**Round two read the number, and found four refusals beside it.**
`compile_allocs` 29,262 -> 29,276 (+14, +0.0478%): the root's own String and
the set of declared bare types, paid once per compile. That row compared. The
four beside it did not. Rounds two and three landed on a runner whose
toolchain the goldens do not name, and `library_instructions` says so in its
own words -- "the sitting above was counted on a toolchain
bench/library_instructions_golden.txt does not name, so it is not a
reproduction of the recorded build and says nothing about the value". The
tell is arithmetic rather than prose: `compile_instructions` measured
48,866,385 against a golden holding 48,866,385 and still failed, and
`entry_instructions` and `library_instructions` did the same. A gate whose
measured value equals its golden and still fails is refusing, not
disagreeing.

Which gates refuse follows from what each golden names.
`bench/instructions_golden.txt` names glibc; the three compile-instruction
goldens name glibc and rustc; `compile_allocs` and `compile_memory` name
rustc alone. Every glibc-keyed gate refused on this runner and both
rustc-keyed gates compared, which is the split the measured-on machinery is
for. So the work vein did not move: `deepbench` 389,214,251 and `runbench`
2,369,917,611 are that runner's sitting, not a regression, and
`bench/instructions_golden.txt` is left alone.

Round one landed on a runner the goldens do name -- its three compile rows
came back as real comparisons, "counted X against Y", and those are the rows
copied above. The refusal is not this change's and no edit here can clear it:
the remedy the gate names is to re-measure every row on the new image in one
go and update the measured-on lines, which is its own change and not a ruling
this PR should fold in.

`tests/the_job_log_prints_every_got_file.rs` pins the property rather than
the glob: every `*_got.txt` any script under scripts/gates writes must be one
the printing step will cat. Watched red with the hand list restored, where it
names both files that list missed, `allocs_got.txt` and
`compile_libraries_got.txt`. The glob satisfies the property today; what the
spec is for is the next session writing the list out by hand again.

**Round five re-measures the four refusing veins, and it rides here.** The
paragraph above says the re-measure is its own change. That was wrong. The
runner pool has rolled forward, so a branch opened to carry the fix would land
on the new image and refuse in the same four places, and there is no tree that
goes green on both. It rides here.

The image is glibc 2.39-0ubuntu8.9 against the goldens' 2.39-0ubuntu8.8, and
nothing else moved with it: `compile_memory` printed "measured-on rustc=1.98.1;
here rustc=1.98.1" and compared, and the `machine_code` gate compares against
clang=19.1.1 and passed. Every run through 14:21Z compared and every run from
14:32Z has refused, four rounds of this PR straddling the change. main's last
run is 14:15Z and green, so main is green on the record and would be red if it
ran again.

The revision costs almost nothing, and round one is what makes that a
measurement rather than two sittings read side by side. Round one counted this
branch's content on 8.8 and round four counted it on 8.9 -- same commit
content, same clang, same rustc, same benchmark sources -- and the work vein
compared GREEN in round one, so every row sat exactly on its golden with this
change already in the tree. Of the seventeen glibc-keyed values, fifteen are
byte-identical across the pair: `compile_instructions` read 48,866,385 both
times, `entry_instructions` 162,751,831, `library_instructions` 163,473,663,
and twelve of the fourteen work rows did not move at all. The two that moved
are `work_deepbench` 389,214,232 -> 389,214,251, a RISE of 19 (+0.000005%), and
`work_runbench` 2,369,917,628 -> 2,369,917,611, a fall of 17 (-0.000001%). Both
are
the revision and neither is this change.

Both rows are copied and the four measured-on lines take 8.9:
`bench/instructions_golden.txt`, `bench/compile_instructions_golden.txt`,
`bench/entry_instructions_golden.txt` and `bench/library_instructions_golden.txt`.
This is the shape the runbench note in the first of those already recorded for
8.7 -> 8.8, where one row moved 1,014 instructions and the rest did not, and it
is smaller. Two ubuntu revisions have now been measured across and each moved
one or two of the allocator-heavy rows by tens of instructions; what a revision
costs is a property of the revision, and the gate refuses either way, which is
why both of these were read instead of assumed.

Welfare reads 66.38 against the 66.37750307886598 floor and is not re-set. The
17 instructions `work_runbench` gives back are worth about a ten-millionth of a
point, and they are the revision's rather than this change's. Ratcheting them
would pin a glibc package revision into the floor, so a pool that rolled back
would read fractionally under it and redden CI for a reason no pull request
caused. The rule that a rise is banked is for a gain a change earned; this one
is left where it is, said out loud here rather than passed over.


## 2026-09-09 — the backends build the partial over a value

Built: the 2026-08-29 ruling "the backends build the partial over a value"
(STATUS.md, now removed; the archive entry of that name, Clay: "BUILD IT").
`&f 2` where `f` is a parameter, a local or a lambda settles its arity when
the arguments arrive, on every engine. Until now native and the page refused
it as a limit of their own — "`f` is a value here, and a partial over a
value settles its arity when its arguments arrive — this backend fixes it
where the closure is written" — because both lowered a partial to the lambda
it is equivalent to, and a lambda fixes its parameter count where it is
written. tests/partial.rs and tests/wasm_engine.rs each pinned that refusal
as a limit rather than a judgement about the program; both specs are flipped
to agreement with the oracle.

**Native.** A partial over a value is a closure with no body: `arity` is -1,
which marks it, and the environment holds the callee first and the held
arguments after it, `ncaps` counting both, so the copy and tenure walks that
size an environment by `ncaps` carry it unchanged. `k_partial0..4` build one
— `&f a b` evaluates the callee and then each argument and the first failure
among them is the answer, callee first, the interpreter's order in its
App-with-Partial arm; a bare `&f` wraps whatever `f` holds. `k_call0..4`
gain one branch in front of their arity test: a body-less closure gathers
held and fresh, asks the callee's arity (a closure's field, a fnref's field,
anything else answers nothing, which is what the interpreter's `arities_of`
answers for a partial over a partial or a value that is not callable), and
dispatches through `k_callN` when the count is an arm's, grows when it is
short, and dies past every arm with the oracle's sentence, `no 3-argument
arm of `<fn>` (arms take 2)`. `()` on one runs it or says how many it is
short by, `this function takes 1 argument(s), got 0`, zero when it holds
more than the callee takes, as the oracle says it. `k_call_decided` routes
a partial to `k_call1` for the same reason the oracle's decided path reaches
`call`. The emitter's inlined fast arm tests `arity == n` before it calls,
so a partial never reaches a body through it and the hot path is
byte-identical: `all_compile.sh` on the branch reads nothing moved that this
host can see, and `all_counters.sh` agrees to the counter.

**The emitter.** `partial_lambda` no longer refuses a value; the two call
sites that reach it — `Expr::Partial` and an application whose head is one
— ask `declared` first and route a value to `emit_partial_value`, which
emits the callee as a value, the supplied arguments, and one call to
`k_partial{n}`. The flattening of `(&f 2) 5` into one call, which is right
for a declared group because dispatch happens on the total count, is kept
for groups only: over a value the total is the callee's to settle at run
time, so the held arguments build the partial and the rest reach it through
`k_call`, which is where the count is settled.

**The page.** `rt_partial` builds a slot the same shape — `Slot::C` with a
`PARTIAL` arity of -4, the environment holding the callee and the held
arguments — and `call_closure` and `call_decided` route it to
`partial_apply`, the interpreter's `apply_partial` and `run_partial` in one
function. A group or builtin handed out as a value used to carry arity -1,
which told a partial nothing about when it was finished, so the wrapper
now carries the counts its arms take as a mask below `MASKED_ARITY`
(-1000): `arities_of` reads the bits back, a lambda answers its one count,
and a partial or a cell answers nothing. `call_closure`'s own arity test
reads `arity >= 0`, so a masked wrapper still dispatches on the count it is
handed, as it did.

**Spec.** `tests/golden/micro/a_partial_over_a_value.kso` runs on all three
engines: a parameter holding a three-arm group finished in one call and in
two steps, a two-arm group, `()` on a complete partial, a partial rendered
as `<fn>`, and a lambda held in a local as the callee. Two runtime fixtures
pin the two sentences on native and the oracle:
`a_partial_over_a_value_past_every_arm` and `a_partial_over_a_value_run_short`.
tests/partial.rs gains the four run-time shapes on both engines, and the
page's flipped spec runs the program the two of them agree on and reads
`14`. The two refusal sentences leave the diagnostic scan.

**The page's call head, met on the rebase.** The page had declined any call
whose head is not a name, a keyword or a lambda with `unsupported call head`,
which is exactly the shape a partial over a value arrives in: `(foo add) 5
7`, a call whose answer is the callee. The backend now computes any such head
as a value and calls it, the runtime naming what it cannot call, so the two
page specs and the runtime fixture run on the third engine.

**CI's rows, round one.** Five veins moved and the job named all five.

The three compile rows rise: `compile_instructions` 48,866,385 -> 48,879,362
(+12,977, +0.0266%), `entry_instructions` 162,751,831 -> 162,822,303 (+70,472,
+0.0433%), `library_instructions` 163,473,663 -> 163,543,712 (+70,049,
+0.0429%). `src/runtime.c` is `include_str!`'d into the compiler, so
`k_partial0..4` and the branch each of `k_call0..4` gains are bytes the
compiler carries and compiles, and `src/codegen.rs` and `src/wasm_backend.rs`
change beside it. `compile_allocs` 29,276 and `compile_peak_bytes` 773,818 are
byte-identical: the front end does no more work, it only has more text.

The whole `.text` vein rises, and it was not in the prediction this branch was
pushed with. Every one of the fourteen rows gains 2,096 bytes or 2,432 --
`text_jsonbench` 97,986 -> 100,418, `text_encodebench` 118,834 -> 120,930,
`text_oneshot` 109,858 -> 112,290, `text_basket` 112,018 -> 114,226,
`text_widebench` 124,098 -> 126,194, `text_deepbench` 74,306 -> 76,402,
`text_escapebench` 55,442 -> 57,874, `text_pendbench` 90,738 -> 92,834,
`text_indexbench` 59,458 -> 61,890, `text_scanbench` 158,642 -> 160,738,
`text_digestbench` 109,554 -> 111,650, `text_readbench` 55,970 -> 58,402,
`text_livebench` 110,434 -> 112,866, `text_runbench` 245,858 -> 247,954. The
runtime is linked into every program, so a runtime that grows grows all of
them; the 336-byte spread is the arms a program's own dispatch makes
reachable. The PR body predicted the host-keyed compile rows and stopped
there, which was the wrong list: a change to src/runtime.c moves the compile
rows AND the text vein, and nothing on this container could see either.

**And the work vein FALLS, on ten rows of fourteen, with none rising.**
`work_runbench` 2,369,917,611 -> 2,369,679,773 (-237,838, -0.0100%),
`work_deepbench` 389,214,251 -> 387,470,247 (-1,744,004, -0.4481%),
`work_widebench` 35,332,240 -> 35,268,228 (-64,012, -0.1812%), `work_pendbench`
590,971,748 -> 590,970,940 (-808), and a uniform -8 on `work_jsonbench`,
`work_encodebench`, `work_oneshot`, `work_digestbench`, `work_readbench` and
`work_livebench`. `work_basket`, `work_escapebench`, `work_indexbench` and
`work_scanbench` are byte-identical.

That is the opposite of what the change looks like it should do. It adds a
branch in front of every `k_call0..4` arity test, and the naive reading is that
every native call pays one more test. What moved is the emitter's inlined fast
arm: it tests `arity == n` before it calls now rather than after, which is both
what keeps a partial from ever reaching a body through it and a cheaper test
than the one it replaced. deepbench and widebench build the deepest and widest
structures in the corpus and call hardest, and they carry most of the fall. The
uniform -8 is one instruction on a path six programs take eight times and the
four flat rows never take.

The fall is small and it is measured rather than argued: both sittings are
CI's, on the same glibc 2.39-0ubuntu8.9 image, and the only difference between
them is this commit.

Summed, the vein the gate weighs is `text` 1,523,196 -> 1,554,668, a rise of
31,472 (+2.0662%). That is the whole of the runtime's growth counted once per
program across the fourteen. Machine-code size carries no welfare term, ruled
2026-09-05, so it is watched exactly and priced here rather than traded.

**Welfare falls, and the floor moves under the language clause.**
66.37750307886598 -> 66.37657452327063, a spend of 0.00093. The run term
improves and the compile term pays more: `work_runbench` falls 0.010% where
the three compile rows rise 0.039% together, and the smaller relative move on
the heavier, later-satiating term does not cover the larger one on the lighter.
`compile_allocs` and `compile_peak_bytes` are byte-identical, so the front end
does no more work; it has more text to compile.

The floor moves to the reading because a RULED feature spent it, which is the
2026-08-25 clause and the same ground #1359 and #1356 moved on. It is not a
claim that the weights are wrong: the objective is reporting the trade
correctly, and a partial over a value on every engine is worth 0.00093 of it
by Clay's own "BUILD IT". The first cut of this entry recorded the move as a
`--set` with a reason that read like a banked gain, which was wrong about the
direction; the history entry says the fall and its size now.
## 2026-09-09 — a succeeded effect yields `done`

Built: the 2026-08-26 ruling "the July letters close", letter D (STATUS.md's
row "`done` is minted", now removed): a succeeded effect yields `done`, and
`none` means absence and nothing else. Until now a print, a write, a sleep, a
`make_dir`, a `write_file`, a kill or a socket close yielded `none` on every
engine, so a chain that bound the yield could not tell a finished effect
from a missing value, which is the railway-skip the ruling closed.

**The value.** `done` is the fourth nullary beside `true`, `false` and
`none`: a literal, an arm's pattern, an annotation's type word and a typeset
member. It renders `<done>`, the way `none` renders `<none>` and a
description `<io>` — a value that is not data prints in brackets, so a
yield that reaches an interpolation is visible for what it is rather than
passing as a word. It equals itself and nothing else; `done == none` is
false on every engine. Infer carries it as its own bit, `DONE`, above the
thunk bit, and the twelve effects that used to yield the empty set yield
`DONE` now: the empty set was how the decided dispatch chose the catch-all
arm for a yield, and with the bit in place a `done` arm is reached on
native the way the oracle reaches it. Native's tag is `K_DONE`, appended
after `K_SUB` so every existing tag keeps its number; the page spells the
literal `Lit::Done` and answers type code 9 for it; its effects run through
the interpreter's executor, so the page's yields moved with the oracle's.

**Twelve effects on two executors.** The interpreter's `execute` and
native's `k_exec` each answer `done` for print, write, write_err, sleep,
the nil description a sleep settles into, make_dir, write_file,
write_bytes, kill, send, send_bytes and the socket close. A read that finds
no file, an environment variable that is not set and `read_bytes` on a
missing path still answer `none`, since those are absences.

**`done` was already a name, twice.** `std/list` declared `pub type done`
with one field, `drained`, and every construction in the module was `done
true`: a record carrying no information, matched by `(done _)` in eleven
arms and re-answered as `done d`. design/enumerable.md §8 had spelled the
iterator's end bare, `next src . (done -> acc)`, all along. So the record
is gone and `next` answers the nullary when a source is drained; the
arms read `done` and the constructions are the word. `std/net/http`'s
`done value` carries the value a serve loop stops with, which is a
different thing, so it is `stopped value` and `http/stop` builds it, with
both callers in scripts/ unchanged since they call `http/stop`.
`std/regexp` used `done` as the name of a repetition count in one arm
family, twenty-five sites; it is `reps`. examples/next_protocol.kso reads
`fn drain done acc`. Anything outside this repository that matched
`(done _)` on a list iterator reads `done` now, and a binding named `done`
is refused with the sentence every other taken name gets,
`done_is_a_value_not_a_name` in the error corpus.

**Spec.** `tests/golden/micro/a_succeeded_effect_yields_done` on all three
engines: a print's yield and a write's yield bound and printed, `<done>`,
equal to `done`, not `none`, and dispatched to an arm that names `done`
beside the arm that does not. The old compiler refuses the file at the
arm, `unused binding done`, since `done` was a parameter name to it.

**What moved.** The list record was one allocation per drained source, so
three runtime cost goldens fall by exactly the drains their programs make:
basket 28,169 -> 28,166 allocs, pend 4,007,549 -> 4,007,349, run
6,936,567 -> 6,936,517, and four `.mem` fixtures with them
(`a_carried_value_written_into_an_older_node` 100,550 -> 100,149, the
other three the same shape). Eight programs that import std/list emit less
now that eleven arms match a tag instead of destructuring a record:
encodebench 1,638 -> 1,607 calls and 11,197 -> 11,105 lines, basket,
widebench, deepbench, pendbench, scanbench, digestbench and runbench each
31 to 40 calls and 92 to 103 lines fewer, branches down 8 or 9 apiece;
the decoder does not import it and is byte-identical. The module compile
golden reads lines 5,269 -> 5,177, calls 758 -> 727, branches 437 -> 430
and `module_visits` 2,409 -> 2,471, and `compile_memory`'s
`front_end_visits` 22,426 -> 22,562, the page's span rewritten: a nullary
in an arm is a cheaper emit and a
slightly longer inference, since `DONE` is a bit the fixpoint carries
where a record was a type it did not. **Re-measured on the base this opens against.** The rows above were first
counted on main 3c7a163b and this change opens against dc980ddf, three merges
later, so `all_compile.sh` was re-run and the two veins it can see were
regenerated here: `emitted_code` on the eight programs and `compile_cost`'s
module row. The deltas are dn's own and did not move -- 31 to 40 calls and 92
to 103 lines fewer, visits 2,409 -> 2,471 -- but two LEVELS did, by the two
lines a program kanso#1361's alias added: encodebench's lines read 11,197 ->
11,105 where the first sitting read 11,195 -> 11,103, and the module row
5,269 -> 5,177 where it read 5,268 -> 5,176. `front_end_visits` 22,426 ->
22,562 is measured rather than projected: `compile_memory` checks rounds and
visits before it asks the host anything, because those count the compiler's
own algorithm and are the same everywhere, and only its `compile_peak_bytes`
row refused.

**CI's sitting, and a prediction that was six-sevenths right.** Round one named
SEVEN veins and this entry had named six of them: the four host-keyed compile
rows, `compile_memory`'s peak, and `machine_code`. The seventh was `work`, the
runtime instruction vein, and the miss is instructive because the entry's own
record implies it: a change that removes one allocation per drained source
removes the instructions that allocation cost. The seventh name written above
was the `text` vein, which is not a seventh gate at all -- `machine_code.sh`
diffs bench/text_golden.txt, so the two are one. Six gates predicted, seven
named, and the arithmetic was wrong in both directions at once.

Every one of the thirty-nine moved keys, by name:

    work           encodebench 3,963,988,518 -> 3,963,988,451        -67
                   basket         34,694,178 ->     34,690,245     -3,933  -0.0113%
                   widebench      35,268,228 ->     35,268,161        -67
                   deepbench     387,470,247 ->    387,470,234        -13
                   pendbench     590,970,940 ->    583,758,224 -7,212,716  -1.2205%
                   scanbench     726,019,157 ->    726,019,079        -78
                   digestbench    10,426,541 ->     10,426,515        -26
                   runbench    2,369,679,773 ->  2,367,877,484 -1,802,289  -0.0761%

    text           encodebench       120,930 ->        120,098       -832
                   basket            114,226 ->        113,170     -1,056
                   widebench         126,194 ->        125,362       -832
                   deepbench          76,402 ->         75,538       -864
                   pendbench          92,834 ->         91,346     -1,488
                   scanbench         160,738 ->        159,842       -896
                   digestbench       111,650 ->        110,530     -1,120
                   runbench          247,954 ->        246,402     -1,552
                   jsonbench         100,418 ->        100,434        +16
                   oneshot           112,290 ->        112,306        +16
                   escapebench        57,874 ->         57,890        +16
                   indexbench         61,890 ->         61,906        +16
                   readbench          58,402 ->         58,418        +16
                   livebench         112,866 ->        112,882        +16

    compile_allocs                     29,276 ->        29,000       -276  -0.9427%
    compile_instructions           48,879,362 ->    48,572,851   -306,511  -0.6271%
    entry_instructions            162,822,303 ->   161,836,689   -985,614  -0.6053%
    library_instructions          163,543,712 ->   162,541,723 -1,001,989  -0.6127%
    compile_peak_bytes                773,818 ->       769,071     -4,747  -0.6135%
    front_end_visits                   22,426 ->        22,562       +136  +0.6064%

jsonbench, oneshot, escapebench, indexbench, readbench and livebench do not
move on the work vein at all, and they are the six that rise exactly 16 bytes
on the text vein. That is the whole shape of the change in two lines: the +16
is `K_DONE`, the tag appended after `K_SUB` in src/runtime.c, which links into
every program whether or not the program can produce the value; the eight that
fall are the eight that import std/list, where eleven arms match a tag instead
of destructuring a one-field record. pendbench takes 1.22% of the work vein
because it drains one source per pending cell, where the rest drain a handful.

The five compile rows fall in one band -- 0.9427%, 0.6271%, 0.6053%, 0.6127%,
0.6135% -- which reads as work removed rather than as the layout move the
compile-row notes usually record: a declaration the front end used to lex,
parse, infer and check is gone. `front_end_visits` goes the other way by 136,
and that is deliberate: `DONE` is a bit the inference fixpoint carries where a
record was a type it did not, so a nullary in an arm is a cheaper emit and a
slightly longer inference. Only the peak is a welfare term.

Welfare 66.38 -> 66.42, banked with `--set` in this same PR. All five terms
improved: `run_instructions` and `compile_peak_bytes` by the deletions above,
`compile_instructions` (the objective's sum of the module, entry and library
rows) by 2,294,114, and `compile_allocs` by 276.

**A blank line cost a round.** Round two took all seven veins' rows and CI came
back with six green and `compile allocations` still red -- on a golden whose
value matched CI's to the digit. The diff was `1d0` against a blank: the note
above the row had been separated from the note before it by an empty line, and
`compile_allocs.sh` builds its expected file with `grep -v '^#'` and nothing
else, so the blank survived into the comparison. The other four compile goldens
carry blanks today and their gates do not mind. Round three deletes the blank
and writes the trap into that golden's own header, because the habit it broke --
separate notes with an empty line -- is right in every other file here.
## 2026-09-09 — the fused chain operators

Built: the 2026-08-31 gavel "the fused chain operators" (archive; STATUS.md's
row, now removed). In chain position each of the three words has one
spelling, the chain dot plus one character of channel: `.>` is `bind`, `.!`
is `annotate`, `.?` is `rescue`.

    settled = json/decode text
      .> count_of
      .! (e -> "settled: {e.reason.reason}")
      .? (e -> "fell back on {e.reason}")

**What the parser learned.** The lexer reads `.` pressed against `>`, `!` or
`?` as one token carrying the word it stands for, and lets that token lead a
continuation line the way the bare dot does. `.!` and `.?` desugar to the
ordinary applications `annotate x f` and `rescue x f`, which every engine
already speaks prefix-style (kanso#1116), so a desugared step reaches the
same `k_b_annotate`, `Desc::Rescue` and `rt_rescue` the prefix form reaches
and the emitters, the runtime and the page are untouched. The right-hand
side is one function, a lambda, a name or a group: `.> f x` is refused with
the partial named, since the gavel's common case is the bare function and a
held argument already has a spelling, `&f x`. A `. bind f`, `. annotate f`
or `. rescue f` step is refused with the fused form named. In chain position
that spelling retired, as ruled, superseding the 2026-08-29 keep-the-dot
ruling for the three words. The words remain prefix functions everywhere
else, and every fixture that calls them that way is unchanged.

**`.>` desugars to the piped step, and the measurement that decided it.**
The first cut desugared `x .> f` to the application `bind x f` like the
other two. On the interpreter that is the same program: `bind` on a
description builds `Desc::Bind`, on a settled failure skips the callback,
on a value calls it, which is what the automatic bind does at a piped step
in each case. Native did not agree. The beat keys on a piped lambda — `App
{ piped: true, head: Lambda }` is the shape beat.rs and infer.rs read as a
chain step whose body is the loop — and a lambda handed to `bind` as an
argument is an escaping closure to both, so the arena stopped rewinding:
with the tree respelled (below) the decoder's `beat_iters` read 151 -> 1
and its `arena_peak_bytes` 2,097,152 -> 251,658,240, runbench's 45,944,528
-> 209,522,384, and welfare fell 9.23 points. So `x .> f` desugars to the
piped application, `App { head: f, args: [x], piped: true }`, the node the
automatic bind has always been, and every pass reads it as it did. The
gavel's sentence holds — `x .> f` IS `bind x f` on every input — and the
node that carries it is the one the compiler already optimises. When the
effect type ends the automatic bind, the plain-dot step over a box will
mean something else and this node will need its own flag; that build owns
the flag, and the emitter's `k_b_bind` stays the prefix word's.

**Every automatic bind is spelled `.>`.** With the spelling in, infer's
piped-over-DESC branch was instrumented to print the file and the head
position of every step it bound automatically, and the tree was checked
program by program (every scripts/ directory, hako, bench, the examples,
the book samples and the golden corpora; the play files through `kanso
play`): 519 sites, 372 with a lambda on the right and 147 with a bare name,
none with a held argument. A script rewrote the dot at each head position
to `.>` — 498 dots, since a chain of several steps reports one site per
step — and a second census under the same instrument read zero. That zero
was over the programs the sweeps compile, and it was not the tree. A dot
followed by a lambda or a name is a grep, and compiling every file that
grep names under the same instrument found 44 more dots in 22 files that
no sweep reaches: the benchmark sources `build_benchmarks.sh` builds and
nothing checks (`total 4000 . (t -> io/write "{t}\n")`, the deep, pend
and wide bodies, and the main text `make_jsonbench` writes out), the
book's ch09 `vse` sample, the trace-demo example, the two workahead
programs, and 56 more in the programs thirteen Rust specs under tests/
carry as string literals, which no census of `.kso` files could reach,
and one in the playground's dice example in docs/play.js. Those are
respelled too — 599 dots in 132 files — and the grep census reads zero. The lesson is the one §30 already records: a dynamic census
counts the programs that ran, so the check is the static list of
candidates, each one compiled. Four statements crossed 80 columns and
wrap onto continuation lines; nothing else changed, and no golden moved:
the piped node is the one those goldens were measured on. The plain-dot
steps whose subject infer reads as a value keep their dot — `"kanso" .
greet . print`, `expect 42 . to (equal 42)` — because under the effect
type a plain-dot step over a value stays an application and only the
steps over a box change meaning. The book's panels were regenerated and
the sentences beside them that named the dot as the way into an effect
now name `.>`; ch04 and ch05's teaching of the railway itself waits on the
effect type, as the ledger says. The book's highlighter tokenised `.>` as a
dot and a `>` and lost the name after it, so `scripts/book_panels` reads
the three fused spellings as one operator token and a name in front of one
as a head; the 29 panels that carry one are re-rendered. The panel check
compares a panel's text with its sample, markup stripped, so a highlighter
change moves no panel on its own; four panels in ch03 and appa carry
markup the highlighter cannot produce (an ascription's type, a name after
an operator, a bare name on its own line), which is why the check stays
on the text.

**Spec.** `tests/golden/micro/a_chain_step_names_its_channel.kso` runs on all
three engines, settled failures only: a group as the bare right-hand side of
`.?` and `.>`, two lambdas, a value bound twice, and the chain above on a
success and on a failure. `tests/golden/chainwords/a_fused_chain_over_an_effect.kso`
is the log's `. rescue orders` program in the ruled spelling: a missing file,
`.? orders .> shout .> print`, prints `no orders yet!!` on native and the
oracle. Two error fixtures pin the refusals, `a_chain_word_is_spelled_fused`
(`. rescue told`) and `a_fused_step_takes_one_function` (`.> told 3`). The
compiler page's combinators section says what landed.

**Three libraries changed, so the compile rows are CI's.** `lib/net/net.kso`,
`lib/net/http/http.kso` and `lib/os/os.kso` respell their chain steps, and
`lib/*.kso` is `include_str!`'d into the compiler, so those are bytes the
compiler carries and compiles. The two compile-side veins this host can read
are unmoved -- `emitted_code` on all fourteen programs and `compile_cost`'s
module and modules rows -- which is what a respelling that changes no emitted
code does. The six host-keyed rows (`machine_code`, `compile_memory`,
`compile_allocs`, `compile_instructions`, `entry_instructions`,
`library_instructions`) refuse here and are expected to move on CI, by layout
at minimum; whatever it reports is copied in from its sitting rather than
projected from this container, which takes clang 18.1.3 against CI's 19.1.1.

**Cherry-picked onto 0d7b3e57, not rebased.** The branch sat on `local/done`,
whose content main now carries in squashed form. The pick asked for two
resolutions, both the log's and the archive's both-append hunks;
`lib/net/http/http.kso` auto-merged, because the collision integration had
predicted there was `done`'s `stopped` against this change's `.>` on one line
and `done` is on main now. Re-verified on the picked tree rather than on the
branch's own worktree: that worktree carries `done`'s code against `done`'s
pre-change module golden, since dn's own commit moves the row and the
regeneration happened later, so `compile_cost` is red there and green here.

**CI named THREE of the six, and the three that held say what the change is.**
The prediction above reasoned from the library edit alone -- three `lib/*.kso`
files change, `lib/*.kso` is `include_str!`'d into the compiler, so all six
host-keyed rows are in play -- and that is right about which rows CAN move and
silent about which will. What moved:

    compile_instructions   48,572,851 ->  48,393,437   -179,414  -0.3693%
    entry_instructions    161,836,689 -> 161,314,264   -522,425  -0.3228%
    library_instructions  162,541,723 -> 162,023,537   -518,186  -0.3188%

and `compile_allocs` (29,000), `compile_peak_bytes` (769,071) and the whole
`text` vein are BYTE-IDENTICAL in the job that counted those three. The front
end allocated exactly the same and held exactly the same doing this compile, and
the linker emitted the same machine code, so nothing about the work changed;
what changed is src/lexer.rs and src/parser.rs, and the three instruction rows
are the layout veins CLAUDE.md's prior describes. Three rows falling together in
one narrow band with the allocation row still is the layout signature; real work
removed would have moved allocs with them, the way `done` did an hour earlier.

Welfare 66.42 held and re-set: the objective's compile term sums the three rows,
so it takes the whole 1,220,025, and the rise is under a hundredth of a point.
Five compiler.html spans quoting the three rows were rewritten by
`golden_prose --write`.

## 2026-09-10 — a record built into its own first field

Found while writing a mem fixture for the carry tier: a loop that chains
records, `grow (node n i) (i + 1)`, came out cyclic on native and correct on
the oracle, on main as on the branch. The emitter builds a constructor into
a record the arm has finished with — `shift (n - 1) (point (p.x + 1) p.y)`
reads every field it needs before the constructor runs, so by then `p` is
done with and its storage is free — and `sole_finished_record` judged
finished by counting mentions: the parameter read in this constructor's
arguments and nowhere else in the arm, and handed over by every caller. A
bare mention counted as a read. `node n i` mentions `n` once, in the
constructor, and every caller hands it over; the emitter passed it to
`k_rec_reuse` as the victim, and the runtime overwrote the victim's fields
with arguments the first of which was the victim. The new node's `next`
pointed at its own storage, and `show` walked it until the stack ran out.

**The fix** is in the analysis. Finished means read from, and a field read
is the only read that leaves nothing behind: `field_reads_in` counts the
mentions that are the base of a `var.x`, and a parameter whose mentions in
the arguments are not all field reads is not a victim. A bare mention stores
the record itself, whether directly or inside another constructor, and a
record something is about to hold is not free. The runtime is unchanged;
`k_rec_reuse` still trusts its caller, which is the arrangement the
2026-09-04 attribution (k_rec 30.39% of pendbench) chose.

The first cut of that rule declined every reuse in the tree — basket's
`sh_rec` 130,192 -> 386,192, `record_reuse_shape`'s 4,006 allocations back
where 3 had been — and the reason is worth a line for the next pass written
against this AST. `desugar_field_reads` runs before the linear analysis,
and after it there is no `Expr::Field` anywhere: `p.x` is `Get_x p`, the
getter applied to the record. A count of `Expr::Field` reads zero on every
program. `field_reads_in` reads both spellings now, the getter application
by `getter_field` on the head's name, and the veins agree.

One site in the tree was the defect's shape and never showed it.
lib/json/scan.kso's `fail p reason` builds `err (parse_failure p reason)`,
and `reason` — mentioned once, bare, in the constructor's arguments, handed
over by every caller — was the victim at that site. It is a string at every
call, and `k_rec_reuse` allocates fresh when the victim is not a record of
the constructor's width, so the program was right by the runtime's guard
rather than by the analysis; a caller passing a two-field record as the
reason would have built the failure into its own `reason` field. The site
emits `k_rec` now, which is the one line the emitted veins lose.

**Spec.** `tests/golden/micro/a_loop_that_chains_records_keeps_each_node`
on all three engines, `3>2>1>0>end`; watched red on native (the stack ran
out) with the oracle green before the fix, both green after.

**Cost.** None at run time: `all_counters.sh` reads the twelve cost veins
and the lazy tier agreeing with their goldens, and welfare holds at 66.31.
The emitted veins each lose the `k_rec_reuse` line at lib/json's `fail`,
landed at: emitted_lines 9,135 (from 9,136), and in the others vein
encodebench 11,142, oneshot 9,064, widebench 12,124, pendbench 7,033,
scanbench 19,728, livebench 9,181 and runbench 34,673 lines, one fewer
each; defines, calls and branches hold, because the `k_rec_reuse` declare
that goes is not a define and its call is replaced by a `k_rec` call. The
six host-keyed compile rows are refused on this container and copied from
CI's sitting.

**CI's five host-keyed rows, and what each one says.** The container refuses
five of the veins this change moves, so round one was red on all five and CI's
sitting is what lands. Two go down and three go up.

The runtime rows fall in four of fourteen programs and hold in ten: basket
34,690,245 -> 34,690,216 (-29), pendbench 583,758,224 -> 583,755,724 (-2,500),
scanbench 726,019,079 -> 726,018,879 (-200), runbench 2,367,877,484 ->
2,367,876,664 (-820). Machine code falls in nine and holds in five, jsonbench
100,434 -> 100,130 the largest at -304. NINE against the emitted vein's EIGHT:
basket loses sixteen bytes of text without losing an emitted line, because the
`k_rec_reuse` declare it drops was already text some other program shared.

The three compile rows RISE, together and by nearly the same fraction:
compile_instructions 48,393,437 -> 48,412,144 (+18,707, +0.0387%),
entry_instructions 161,314,264 -> 161,360,451 (+46,187, +0.0286%),
library_instructions 162,023,537 -> 162,069,092 (+45,555, +0.0281%). That is
the price of the answer. `sole_finished_record` used to count mentions, which
costs nothing; `field_reads_in` asks of each mention whether it is the base of
a field read, which walks. Every compile route runs the linear pass, so all
three rows move, and a per-declaration check that moves them by the same
fraction is what a uniform cost looks like.

**The trade, and the objective's verdict.** A rise anywhere is a thing to
state rather than defend, and this one is real: the compiler does 110,449 more
instructions summed across the three routes to buy 3,549 fewer at run time on
the four programs that reach the shape. Stated that way it sounds like a bad
bargain, and by the raw counts it is. The objective disagrees, because the
counts are not what it weighs: welfare reads **66.42 against a floor of 66.42,
held exactly**, since a 0.03% rise on a compile term measured in hundreds of
millions moves the saturating curve by less than the floor's own precision.
The change is a MISCOMPILATION fix besides — a program that came out cyclic
now does not — and that is not a term the objective has at all.

## 2026-09-10 — three library shapes off the run program's profile

Three arms, each one a shape the profile pointed at, each one measured on
today's main rather than on the base they were first cut against.

**A cap around a count is a range.** pendbench's churn fuses to a fold over
`take naturals n`, and the general capped arm pulled every element through
`next`, building a count, a step, a cap and another step apiece — four records
read once and dropped. `fold (capped left (counting at))` counts instead, the
sibling of the window arm already beside it.

**A class asks by the byte.** std/regexp's character classes walked a string
per candidate; the arm reads the byte.

**The colon is the arm before the error.** lib/json's `obj_key` ran `skip_ws`
to the colon, took back only its position, and then had `expect_char` load the
same byte a second time to compare it. `obj_key_end` is the shape `array_delim`
and `obj_delim` already take: the colon is an arm, the whitespace is the arm
before it, and the error is the arm after.

**A deletion at the head of a library file moves every diagnostic below it.**
Dropping `import "std/text"`, `expect_char` and both `expect_check` arms takes
eleven lines off the top of lib/json/scan.kso, so `fail` moves from line 13 to
line 2 and the endpoint diagnostic that names its birthplace moves with it:
`tests/golden/runtime/a_lone_surrogate_is_half_a_character.stderr` now reads
`born in json/fail at std/json/scan.kso:2`. Two tests read that one golden —
`runtime_corpus_reports_endpoint_violations` in tests/golden.rs and
`interpreter_reports_each_runtime_endpoint_violation` in tests/oracle.rs — and
both went red until it was regenerated. Nothing else in the tree quotes a
scan.kso line. Worth writing down because no counter, gate or sweep can see it:
the change is a deletion of dead helpers, and it moved a user-visible sentence.

**What they cost, on this tree over origin/main ef2f4ea4.** The arms were cut
against 32080acf and re-measured after #1366 merged in. Not one runtime counter
moved between the two bases: #1366 changed which constructor `fail` emits, not
what anything allocates.

    pendbench   allocs      4,007,349 ->   807,149   (-79.86%)
                alloc_bytes   257 MB  ->     65 MB   (-74.66%)
                arena peak      2 MB  ->      1 MB
                sh_rec    192,067,248 ->    54,448   (-99.97%)
    scanbench   allocs      3,975,884 -> 3,011,152   (-24.26%)
                arena peak    198 MB  ->    161 MB   (-18.52%)
                sh_str     37,039,216 ->     1,632
    runbench    allocs      6,936,517 -> 5,958,961   (-14.09%)
                arena peak   45.9 MB  ->   39.7 MB   (-13.70%)

`sh_rec` falling by a factor of three and a half thousand on pendbench is the
counted fold: those four records an element were most of what the benchmark
allocated. `sh_str` on scanbench is the byte class, which is why that row goes
to almost nothing rather than down a fraction.

**And the instructions, which this container could not see at all.** The work
vein is host-keyed and refused here, so welfare read main's row and reported
the floor merely held; CI's sitting is where the change actually shows.

    pendbench    583,755,724 ->   226,874,035   -356,881,689  -61.1355%
    scanbench    726,018,879 ->   587,488,478   -138,530,401  -19.0808%
    runbench   2,367,876,664 -> 2,262,265,575   -105,611,089   -4.4602%
    jsonbench  1,485,334,791 -> 1,470,973,791    -14,361,000   -0.9669%
    oneshot       21,737,110 ->    21,641,370        -95,740   -0.4404%
    deepbench    387,470,234 ->   387,090,234       -380,000   -0.0981%
    basket        34,690,216 ->    34,672,719        -17,497   -0.0504%
    livebench  3,481,899,695 -> 3,481,803,955        -95,740   -0.0027%

Eight of the fourteen fall and none rises. The other six are byte-identical:
encodebench, widebench, escapebench, indexbench, digestbench and readbench
reach none of the three arms. pendbench losing three fifths of its instructions
is the counted fold alone -- the four records an element were not only
allocated but walked.

**Welfare 66.42 -> 67.56**, the largest single move since the 2026-09-03
rebuild, banked with `--set` in this same PR. Both the allocation terms and
run_instructions pay for it; the container's blind reading had put the gain at
0.86 and the whole of that was the memory side.

**The compile side pays, and the emitted code grows.** Three new arms is three
more declarations for the front end to visit: front_end_visits 22,562 -> 22,727
(+165, +0.7313%) with rounds holding at 62, the arms being wider rather than
deeper so nothing re-converges. Every program that compiles std/list carries
the counted arm, so the emitted rows rise across the board — encodebench 11,104
-> 11,232 lines, scanbench 19,673 -> 19,922, runbench 34,726 -> 34,979. The
decoder is the one row that gains lines while LOSING calls: 9,246 -> 9,272 with
calls 1,236 -> 1,226, because it reaches the colon arm and neither of the other
two. Four rows are byte-identical — escapebench, indexbench, digestbench and
readbench reach none of the three. That is the trade stated plainly: more code
written, much less allocated.

**Every counter that went the wrong way, with the value it landed on.**

The byte class trades string work for byte work, so two counters that were
zero on scanbench are no longer zero and the same pair rises on runbench, which
runs split: `scan_find2_calls` 0 -> **501,505**, `scan_sh_bytes` 0 ->
**24,072,240**, `run_find2_calls` 3,017,520 -> **3,109,759**, `run_sh_bytes`
36,862,800 -> **41,290,272**. Against them `scan_sh_str` falls 37,039,216 ->
1,632 and `scan_sh_buf` 24,105,200 -> 33,024. The bytes a class reads are the
bytes it used to read as a string, and it stops copying them.

The compile-module vein carries the three new declarations: `module_defines`
100 -> **101**, `module_calls` 727 -> **754**, `module_branches` 430 -> **446**,
`module_lines` 5,177 -> **5,304**, `module_visits` 2,471 -> **2,534**, beside
`front_end_visits` 22,562 -> **22,727**.

Three more declarations cost the compiler itself, and these are CI's rows:
`compile_instructions` 48,412,144 -> **49,091,884** (+679,740, +1.4040%),
`entry_instructions` 161,360,451 -> **164,041,672** (+2,681,221, +1.6616%),
`library_instructions` 162,069,092 -> **164,324,373** (+2,255,281, +1.3910%),
`compile_allocs` 29,000 -> **29,350**, `compile_peak_bytes` 769,071 ->
**774,660** (+5,589, +0.7267%). The objective's compile term is module plus
entry, 209,772,595 -> 213,133,556, a rise of 1.60% against a run-instruction
fall of 4.46% on a term that satiates late; the trade is not close.

The `text` vein sums 1,543,836 -> **1,547,852** across the fourteen, and it
moves both ways by families: **+800 or so** where the
counted fold lands (deepbench 75,538 -> 76,338, digestbench 110,530 ->
111,330, encodebench 119,826 -> 120,642, widebench 125,090 -> 125,906,
runbench 246,082 -> 246,914, pendbench 91,090 -> 91,778, basket 113,154 ->
113,650), **-400** where the colon arm does (jsonbench 100,130 -> 99,730,
oneshot 112,002 -> 111,602, livebench 112,578 -> 112,178), and scanbench
159,602 -> **159,570**, the one row that carries the byte class and comes out
32 bytes smaller. escapebench, indexbench and readbench are byte-identical.
The machine code grows where an arm is added and shrinks where three helpers
go, which is what the emitted vein says in lines.

The emitted veins are the same three arms written out: `emitted_branches` 816
-> **819** and `emitted_lines` 9,246 -> **9,272** on the decoder;
`emitted_other_defines` 2,354 -> **2,366**, `emitted_other_calls` 20,245 ->
**20,445**, `emitted_other_branches` 12,639 -> **12,800**, `emitted_other_lines`
132,530 -> **133,802** across the thirteen beside it. `emitted_calls` on the
decoder FALLS, 1,236 -> 1,226.

Two rows of the inner-beat tenure fixture move with the counted fold, which is
what that fixture folds:

    an_inner_beat_opens_its_tenure_in_the_block_outside_survive_slots 2,004 -> 32,404
    an_inner_beat_opens_its_tenure_in_the_block_outside_ten_frees 3 -> 2

with `ten_blocks` 3 -> 2 beside them. The fold stops building four records an
element, so far less
is allocated inside the beat (allocs 512,494 -> 177,420, alloc_bytes 41.2 MB ->
16.9 MB) and the beat runs a third as many iterations (158 -> 68); what survives
a longer-lived block is counted in more slots and freed in one fewer.

**Five veins are CI's.** machine_code, compile_allocs, compile_instructions,
entry_instructions and library_instructions are refused on this container, and
compile_memory_golden.txt is refused whole (measured-on rustc=1.98.1, here
1.94.1) even though the visits row it carries is host-invariant. Round one is
red on those and CI's sitting is what lands.

## 2026-09-10 — a validated string knows its length

The utf-8 validator already classifies every byte on its way through. Counting
the continuation bytes while it does gives the character count for free on the
ascii path and for two vector instructions a block on the wide one, and that
count seeds the memo `k_str_chars` would otherwise fill by scanning the string
a second time. `length` on a validated string never walks it.

Three doors seed: `read_file`, `k_b_utf8`, and the whole-string check. The
decoder's token door in `k_b_utf8_slice_raw` deliberately does not. Those are
861,498 slices on runbench and none of them is ever asked its length, so
seeding each one measured 7,728,237 instructions against the 13,280,580 the
other three save. The comment at that call site says so, because the omission
looks like an oversight and is not.

**The counters.** `str_scans` goes to zero on the encode, live and oneshot
programs (400, 400 and 1 scans, 75,479,200, 75,479,200 and 188,698 bytes), and
on the run program falls 254 -> 163 with `str_scan_bytes` 22,644,612 ->
5,473,094. Nine lazy-tier fixtures move those two rows and nothing else: no
allocation counter, no peak, no evacuation counter differs anywhere. The ninth
is kanso#1367's own `a_class_asks_by_the_byte`, which lands 7 scans and 12
scanned bytes where it read 8 and 2,212 — the byte class validates a string
and then asks its length, so the two changes meet on one fixture.

**Instructions, measured here under callgrind.** Both compilers built in the
same worktree, both binaries run from the same directory under the same
filename with the environment emptied, so the fixed fourteen-instruction offset
that a two-worktree A/B puts on every row is not in these numbers.

    livebench    3,484,129,797 -> 3,452,753,847   -31,375,950  -0.9006%
    encodebench  4,084,100,523 -> 4,052,767,921   -31,332,602  -0.7672%
    pendbench      225,398,305 ->   220,436,506    -4,961,799  -2.2015%
    runbench     2,241,481,220 -> 2,232,013,849    -9,467,371  -0.4224%
    jsonbench    1,438,095,130 -> 1,436,454,329    -1,640,801  -0.1141%
    oneshot         21,417,863 ->    21,396,929       -20,934  -0.0977%
    readbench        4,562,212 ->     4,631,248       +69,036  +1.5132%
    widebench       36,429,055 ->    36,509,001       +79,946  +0.2194%
    deepbench      395,527,105 ->   395,911,106      +384,001  +0.0971%
    basket          34,861,960 ->    34,891,911       +29,951  +0.0859%
    indexbench       3,226,185 ->     3,226,248           +63
    digestbench     10,497,723 ->    10,497,757           +34
    scanbench      600,502,339 ->   600,502,367           +28
    escapebench     85,489,183 ->    85,489,184            +1

runbench is the objective's whole run-speed term and it falls 0.4224%.

**The same fourteen deltas, to the instruction, on both bases.** This was
measured once over ef2f4ea4 and again over 6c32079a with kanso#1367's three
library arms in between, and every one of the fourteen absolute deltas is
identical: -31,375,950 on livebench both times, -4,961,799 on pendbench, +69,036
on readbench. The percentages move because the bases did — pendbench reads
-2.2015% here against -0.8568% before, since kanso#1367 took three fifths of
that program away and the same saving is now a larger share of what is left.
Worth writing down: the two changes touch disjoint work, so neither measurement
had to be redone for correctness, only for its denominator.

**readbench is the pure-cost case, and it is worth naming rather than
averaging away.** Its +69,036 is `k_utf8_bad_wide` and nothing else:
274,748 -> 343,792 under `callgrind_annotate`, which is the whole delta to
within eight instructions. readbench reads one file whose bytes take the wide
path and never asks its length, so it pays the two vector instructions a block
and collects nothing. widebench, deepbench and basket rise for the same reason
in smaller amounts. The counting is cheap where the answer is wanted and not
free where it is not, and four benchmarks are on the wrong side of that.

**Welfare cannot be read for this change on this container.** `run_instructions`
comes from `bench/instructions_golden.txt`, which is host-keyed and refused
here, so the objective reads main's row whatever the tree does — a change whose
entire effect is instructions is invisible to a local `welfare` run. The number
that matters is CI's, on CI's rows.

**CI's sitting, and the twelve counters that rose.** The container refuses the
work vein, the text vein and the three compile rows, so round one was red on
all five and CI's numbers are written into the goldens here. Six of the
fourteen work rows fall — `work_encodebench` 3,932,651,503 (-0.7905%),
`work_livebench` 3,450,423,659 (-0.9012%), `work_runbench` 2,252,446,969
(-0.4340%), `work_pendbench` 221,912,236 (-2.1870%), `work_jsonbench`
1,468,801,090 (-0.1477%), `work_oneshot` 21,616,888 (-0.1131%). Eight rise, and
they are the accumulator the validator carries for strings whose count nobody
asks for: `work_readbench` 4,630,969 (+69,036), `work_deepbench` 387,474,235
(+384,001), `work_widebench` 35,316,107 (+47,946), `work_basket` 34,698,668
(+25,949), `work_indexbench` 3,265,849 (+63), `work_digestbench` 10,426,549
(+34), `work_scanbench` 587,488,506 (+28), `work_escapebench` 85,558,106 (+1).

`text` 1,547,852 -> 1,551,324, a rise of 3,472 bytes spread over all fourteen
rows — jsonbench 100,050, encodebench 120,962,
oneshot 111,922, basket 114,082, widebench 126,226, deepbench 76,354,
escapebench 57,922, pendbench 92,034, indexbench 62,162, scanbench 159,842,
digestbench 111,362, readbench 58,434, livebench 112,498, runbench 247,474 —
most by 320 bytes and runbench by 560: the
three utf-8 arms carry a fourth parameter and a conditional store, and they
live in src/runtime.c, which every program links. The three compile rows rise
by the same fraction and for the same reason — `compile_instructions`
49,097,584 (+5,700, +0.0116%), `entry_instructions` 164,060,471 (+18,799,
+0.0115%), `library_instructions` 164,342,505 (+18,132, +0.0110%). The front
end does no more work; runtime.c is carried inside the compiler, so its bytes
and the layout under them move when it changes. Welfare reads 67.59 against a
floor of 67.56 and is ratcheted to it.

**Eight of the fourteen deltas match the container to the digit, six do not.**
deepbench +384,001, readbench +69,036, pendbench -4,961,799, indexbench +63,
digestbench +34, scanbench +28, escapebench +1 are identical between the local
callgrind A/B and CI's perf sitting. The six that differ do so by under a third
of a per cent of the delta — runbench -9,818,606 here against -9,467,371
locally, widebench +47,946 against +79,946 — and no row changes sign. Two
instruments counting the same program agree on what moved and disagree in the
last digits; the goldens carry CI's, which is what the gate reads.

**The signature change broke a gate that reads the real source text.**
`scripts/utf8_differential` extracts `k_utf8_bad`, `k_utf8_bad_wide` and
`k_utf8_bad_scalar` out of src/runtime.c by searching for their signatures, and
all three signatures gained a parameter and started wrapping across two lines.
The search found nothing, `body_of` asked for the second half of a split that
had only one part, and the harness died with `missing index 2` before it
compiled anything. That is the cost of extracting the real text rather than a
copy, and it is the right cost: a harness reading a stale copy would have gone
on passing. The three signature constants now carry the wrapped form, the two
inner wrappers take `long long* chars`, and the door's wrapper declares one as
NULL since harness.c calls it with two arguments. 45,189,025 cases and
8,346,016 count checks, 0 mismatches.

## 2026-09-11 — exhaustiveness on arm match, and the inference it needed

Searched the log, the archive and design/ before filing: `check_none_exhaustive`
appears in the 2026-07-24 none-campaign entries that built it and in the
2026-08-15 sitting that recorded the rule, and nowhere since. The archive's
last campaign report blames the group-level return set for the migration cost
and leaves it there; STATUS.md's row said that set was "the implementer's to
sharpen". This entry is the sharpening and the flag's removal together.

**The ruling.** Clay, 2026-09-09: "the exhaustiveness when you're looking for a
match on an arm has always been the way the language works." `KANSO_EXHAUSTIVE`
comes out of src/check.rs and the check runs on every compile, on every route.
The refusal fixture is the book's menu sample with its `none` arm deleted:
`describe menu["pocky"]` where `fn describe price` interpolates. Until now that
program ran and printed `<none> yen`.

**A wildcard is NOT a `none` arm, and getting that wrong is silent.** The first
cut counted `Pattern::Wildcard` as an arm that handles a none, on the reasoning
that `_` does bind one. It does — and so does a bare name, which is what the
ruling's own fixture writes. Counting either leaves the rule with nothing to
say: the refusal fixture went green, the whole tree checked clean, and the only
thing that noticed was the golden's empty stderr. Only a pattern that NAMES a
none — `none` or `x:none` — is an arm for one.

**Two under-refusals are load-bearing, deliberately.** The check reads a call's
ARGUMENTS, not its head, so `nothing 1` where `nothing = none` is permitted.
And a lambda's call has no `Expr::Ident` head to look up, so `(_ -> none) 0` is
not proof either. Both are the safe direction — the rule refuses less than it
could rather than more — and both are what keep two runtime fixtures reachable
at all: tests/golden/runtime/calling_none_names_it.kso and
to_float_names_what_it_takes.kso exist to run a none into a builtin and read
the runtime's sentence, which the rule would otherwise refuse at compile time.

**Unknown is not proof.** A set holding every value bit is infer's don't-know,
and it carries the none bit with the rest. A field read through a variable is
TOP; a strict index `xs[i]!` is every value but a thunk. Without the guard a
group that hands either back reads as proof of a none it never produces, and
half the tree is refused for nothing. Getters are skipped for a different
reason: a getter is synthesized from a field read, so nobody can give it an
arm, and the play route checks before the read is rewritten into one while the
module route checks after — `xs[i].x` would be refused through an import and
run direct.

**The phantom, and the fix the rule actually needed.** `scripts/fingerprint`
was refused at `sha256/hex (as_bytes raw)`, for a none that cannot happen.
Traced with a debug dump of every group's return set: the source is
`os/read_file!`, whose set carries NONE. Its body is
`builtin_read_file path .> (r -> insisted path r)`, and `insisted` is two arms —
`insisted path none` answering the missing file, `insisted _ text` handing the
text back. The first arm answers every none. The second inherits it anyway,
because `widen_param` widens EVERY arm's parameter by the whole argument set
with no account of what the arms above it already took, so `text` carries NONE,
the arm hands it back, and `read_file!` reads as an answer that could be a
none — all the way down to whatever the caller did with it.

infer now carries a per-parameter `shadow` table beside `params`: an arm
earlier in its group takes the bits it NAMES at a position, and `widen_param`
subtracts them. The narrowing is sound only when the earlier arm's OTHER
positions accept anything, because `f 1 none` catches a none at position two
for a 1 alone and says nothing about the arm below it; the table is built once
from the group table rather than per call, so the fixpoint pays nothing for it.
`pattern_catches` already existed for the FAIL pass-through on the same
reasoning — this is the same question asked of the parameter instead of the
result.

Watched red first: tests/golden/micro/an_arm_below_a_none_arm_is_never_handed_one.kso
is `kept none` / `kept s` fed a group that answers a literal none, its result
handed to a `shouted` with no `none` arm. With the subtraction disabled the
module is refused with the exhaustiveness diagnostic naming `shouted`; with it
the program compiles and prints `nothing!` / `7!`. Note what the fixture could
NOT be: a lenient index is TOP in infer, so `word[9]` would have been caught by
the unknown guard and proved nothing — the none source has to be narrow.

Every other site the rule refused was re-verified by reverting it and
re-checking: all genuine, none of them a phantom the narrowing would have
removed.

**What the rule forced.** One library shape: `lib/list`'s `bisect` carried a
`none` seed through `list/fold`, and `fold` has no `none` arm. It carries the
INDEX now and answers through `found_at`, which is shorter and retires nothing
the module needed. `lib/regexp`'s `gathering_slots` had the same shape and
seeds with the first value, retiring `or_blank`. `hako/remote`'s `highest`
seeds with the first release and retires `later`. Four programs outside lib
resolve at the site: `examples/trace_demo` and `scripts/browser_differential_run`
take the strict index they meant, `scripts/welfare_rescore` replaces a
boolean-flag arm with a `none` arm, and eleven vendored benchmark files under
bench/encodebench and bench/widebench take strict indexes and `none` arms to
match the library they were copied from.

Five corpus fixtures were reshaped rather than excused, because each existed to
run a none into a generic arm — which is exactly the program the rule refuses.
They name the none now and assert the same output.

**The veins.** The narrowing pays where a scrutinee stops carrying a none the
arms above it already answered: the emitter drops the arm's none test and the
force in front of it. The decoder loses 16 calls, 24 branches and 109 lines;
of the thirteen rows beside it six fall, four rise by seven lines apiece (the
`none` arms in std/list, in programs that reach none of the narrowing shapes)
and three hold. Summed: emitted_other_calls 20,445 -> 20,386,
emitted_other_branches 12,800 -> 12,689, emitted_other_defines 2,366 -> 2,364,
emitted_other_lines 133,802 -> 133,375. The front end's own work falls too:
front_end_visits 22,727 -> 22,452 (-1.2100%), and on the module corpus
module_visits 2,534 -> 2,511. **module_lines 5,304 -> 5,310** is the one rise
— six lines, the `none` arms the rule forced into lib/list and lib/regexp, and
the price of every fall above. Every runtime cost counter and the lazy tier are
byte-identical: `all_counters.sh` reports the twelve cost veins agree.

Welfare reads 67.59 = floor here, and cannot say more: `run_instructions`,
`compile_instructions`, `compile_allocs` and `compile_peak_bytes` all come from
goldens this container's host gate refuses, so the objective sees no movement
until CI writes its own sitting in. Round one expects red on those rows.

**CI's sitting, and what the ruling costs.** The three compile rows rise
together: compile_instructions 49,097,584 -> 50,747,925 (+1,650,341 /
+3.3614%), entry_instructions 164,060,471 -> 168,851,345 (+4,790,874 /
+2.9202%), library_instructions 164,342,505 -> 169,613,006 (+5,270,501 /
+3.2070%). With them compile_allocs 29,350 -> 29,483 (+133) and
compile_peak_bytes 774,660 -> 777,126 (+2,466 / +0.3184%).

Split three ways on this container, one build per reading: the head measures
51,460,049, the head with `check_none_exhaustive` not called measures
50,564,701, and the head with the shadow mask off as well measures 50,288,026.
So the check is 895,348 instructions and the shadow table 276,675, against a
container total of 1,172,023; the rest of CI's rise is the library source the
rule forced and layout. The check is the larger half and could not be smaller:
it sat behind `KANSO_EXHAUSTIVE`, the flag was set nowhere, and a ruling that
costs nothing to carry is a ruling that answers nothing. The container reads
about 1.4% high against CI on this row, so read the split as a ratio rather
than as CI instructions.

In the work vein two of fourteen rows move and twelve are byte-identical:
encodebench 3,932,651,503 -> 3,958,779,263 (+0.6644%) and widebench 35,316,107
-> 35,202,913 (-0.3205%), both the vendored benchmark sources taking strict
indexes and `none` arms rather than anything in the runtime. **runbench, the
objective's whole run term, does not move**, and neither does run_peak_bytes.
In the .text vein seven rows move: four fall by the same 176 bytes (jsonbench,
oneshot, livebench) and runbench by 160 with their work rows identical, which
is the shadow table reaching the emitter — an arm below a `none` arm loses the
case it was compiled with. encodebench +1,360, widebench +464 and scanbench
+16 are the vendored sources again.

**Welfare falls 67.58619 -> 67.52, and that is Clay's call, not mine.** Every
term that moved is a compile term and every one of them got worse; nothing
improves. `--set` refuses a fall this size by design and the floor file is
edited by hand, which is what the 2026-08-25 language clause has meant three
times before (#1355, #1356, #1359) — but those spent 0.001 to 0.01 and this
spends 0.07, an order of magnitude more than any ruled feature has taken from
the objective. Sent to Clay rather than banked: the change is a ruling and
cannot go, so the only question left is whether the objective should record
what the ruling costs. Not filed in design/pending-gavels.md here — that
ledger's own rule is that its edits ride small, promptly-merged PRs and never
a feature branch, and this is one.

Named for the trend gate, which asks a worsened row for its landed value:
work_encodebench 3,932,651,503 -> 3,958,779,263 is the vendored encode
benchmark's own source, and `text` 1,549,100 -> **1,550,252** is the .text vein
summed — up 1,152 bytes across fourteen programs, where seven rows move and
the two vendored ones carry all of the rise. That pair read 1,551,324 ->
1,552,476 until the trend gate refused round two: the rise of 1,152 was right
and both endpoints were a base behind, because kanso#1372 moved the .text vein
by 2,224 between this branch being measured and being merged with main. The
goldens on this branch are CI's round-two rows and these are now read off
them.

## 2026-09-11 — the ruling's compile cost, paid down by a third

The entry above priced per-call exhaustiveness at +3.3614% on
compile_instructions and sent the welfare fall to Clay. Two of the structures
the rule added were doing the same work twice, and profiling the same box the
three-way split was read on names both.

`check_none_exhaustive` kept two maps: `returns`, keyed by (name, arity), and
`handles`, keyed by (name, arity, position) and holding one bool. Both keys
start with the declaration's name, so building `handles` hashed that string
once per PARAMETER and reading it hashed it once per ARGUMENT, on top of the
`returns` hash the same call site already paid. The two are one map now, the
per-position bool a bitmask beside the return set, so a call site pays one
hash and a declaration pays one insert. `check_merged_after_aliases` falls
2,519,015 -> 2,293,470 and the module row falls 276,320.

`widen_param` is three lines and LLVM inlined it at every caller until the
shadow mask was added, at which point it outlined: the profile read 584,011
instructions under a name main spends nothing on. That figure is the call
overhead. Pinned `#[inline(always)]` the symbol disappears, the load stays,
and the module row falls another 223,326.

Together, on this container, one build per reading:

    module compile    50,199,441 main    51,758,263 ruled    51,258,065 now
    entry compile    166,705,591 main   171,650,496 ruled   169,903,991 now

The objective's compile term is those two summed. It reads +2.9984% against
main as the rule shipped and +1.9626% now, so 34.5% of the rise is recovered.
What is left is the check's own walk and the shadow load, and the rule needs
both to do its job.

Nothing the rule refuses moved. Each structure was watched red, and the two
mutations fail on different fixtures in opposite directions: with the mask
never learning a position, `foreign_destructure` is refused though it has a
`none` arm; with every position reading as handled, the ruling's own fixture
stops being refused at all. `emitted_code`, `compile_cost` and the twelve
runtime cost veins are byte-identical, so no decision moved — only what
deciding costs.

A position past the mask's width reads as handled, which is the same
under-refusing direction as the two gaps the entry above records. The widest
group in lib/ takes five parameters against a width of sixty-four.

CI's sitting, on the run that read `9d737dec`. Step 30 names four failures and
no others: `work`, `emitted`, `machine code` and `compile memory` all agree, so
no runtime counter and no emitted line moved, and `compile_peak_bytes` holds at
777,126. The four rows that did move, each against main and against the value
the entry above landed them on:

    compile_instructions   49,097,584 main   50,747,925 ruled   50,228,060 now
    entry_instructions    164,060,471 main  168,851,345 ruled  167,038,742 now
    library_instructions  164,342,505 main  169,613,006 ruled  167,802,001 now
    compile_allocs             29,350 main       29,483 ruled       29,473 now

So compile_instructions falls 519,865 (-1.0244%) from where the ruling left it,
entry_instructions 1,812,603 (-1.0735%), library_instructions 1,811,005
(-1.0677%), and compile_allocs 10 (-0.0339%) as the second map's table goes.
All four still stand above main, and that residue is the check's walk and the
shadow load.

The objective's compile term is the module and entry rows summed: 213,158,055
on main, 219,599,270 as the ruling shipped (+3.0218%), 217,266,802 now
(+1.9276%). **36.2% of the rise is recovered.** The container projected 34.5%
off its own three readings and was pessimistic by a point and a half, which is
the usual direction for this box.

The welfare question in the entry above stands with a smaller number in it,
and CI prices it.
---

## 2026-09-10 — the plain dot is an application, and a box where a value is expected is refused

Built: the first half of the 2026-08-29 gavel "effects are types, and the
words are the only doors" (archive; STATUS.md's row, now shortened to what
is still owed). Two things the row measured as unbuilt on 2026-09-09: the
automatic bind, and the refusal of a box where a value is expected.

**The plain dot opens nothing.** `x . f a` is `f x a`, an ordinary
application — the parser folds a `.` step into the same node the prefix
spelling makes, so a box handed through it arrives as a box, a settled
failure handed through it dispatches to the arm that names it as a prefix
call would, and `.>` is the one step that binds. That is the whole of the
change in the parser, and it changed no program's meaning: the previous
entry respelled every step the compiler bound automatically as `.>`, keyed
on infer's own judgement, so no plain-dot step over a description was left
for this to change. That is true now and was not on the first build here:
`build_benchmarks.sh` died inside `make_jsonbench` with `write_file` handed
a box, because the benchmark sources are built by that script and compiled
by nothing the census ran, and the 44 dots the previous entry now records
were found and respelled from this branch before anything else was
measured. Three consequences fell out. `effect . rescue orders`,
the sentence STATUS.md held up as the ruling's unbuilt point, is refused
in chain position by the previous entry and spelled `effect .? orders`,
which works. The enumerable fusion no longer needs its piped copy for a
plain chain: `xs . list/map f . list/length` is the prefix chain now and
fuses through the plain path, where before it took `try_fuse_piped`'s
`is_desc` test and a second copy of the chain. And the beat reads the
piped node as the loop step it always was, since only `.>` makes one.

**A box where a value is expected is refused.** `check_box_where_value`
reads infer's return sets the way the none check does. A box is provable
when a group's joined return set holds the description bit and no value
bit, when a `.>` step's subject is one, when the expression is a wall, or
when a constant holds one — `os/args`, `math/random 6`, `io/write "x"`.
Handing one to an operator, an index, a field read, `if`'s condition, a
builtin that reads values, or a group none of whose arms binds anything at
that position is refused with one sentence: `this is an effect — a box the
words open — and `length` takes a value; open it with `.>``, the reader
named. Holding is not opening, so a parameter that binds anything takes the
box (`held e` above), `print` and an interpolation render it as `<io>`,
`is_desc` asks about it, `push` and `put` store it, and `err` and
`wrap_err` carry it as a reason. A name the declaration binds itself is that
binding whatever declaration shares its spelling: the first cut refused
`length args` in scripts/welfare, where `args` is a parameter and the
constant it shadows is `os/args`.

**What the tree said.** Under the check, every scripts/ directory, hako,
the library tests, the golden corpora and the play files (through `kanso
play`) answered twelve refusals, all deliberate: eleven runtime fixtures
that hand a description to an operator, an index, a field, `if` or a
comparison to pin the runtime's sentence, and one micro fixture handing
one to `push`. `push` holds, so that one passes as written. The eleven
now route the box through a list — `opaque v` answers `(push [] v)[1]!`,
and a strict index is every value but a thunk, which no check can call a
box — so the runtime sentences they pin stay pinned on every engine, and
the errors corpus gains `a_box_where_a_value_is_expected`: six readers
refused in one file, and the two holders that pass beside them.

**What moved.** `emitted_code`, and down: a plain-dot step over a value
used to lower through the piped node's runtime test of its subject, and
it is a direct call now. `escapebench` defines 51 → 48, calls 117 → 106,
branches 122 → 120, lines 1,638 → 1,584; `scanbench` defines 336 → 331,
calls 3,280 → 3,265, branches 2,143 → 2,136, lines 19,922 → 19,821;
`indexbench` defines 57 → 54, calls 158 → 146, branches 139 → 135, lines
1,954 → 1,893; `runbench` defines 597 → 594, calls 5,981 → 5,968, branches
3,481 → 3,474, lines 34,979 → 34,905. These are this branch's sitting on
3c1b9e59; an earlier draft carried the same deltas read against an older
main, and the deltas are what transfer. The decoder's row and the other nine
are byte-identical, and the twelve cost veins and the lazy tier agree with
their goldens: the runtime work is the same because the test those defines
carried always answered the same way. The host-keyed compile rows are
CI's to measure; parser.rs and check.rs both change.

**CI's sitting.** The four programs that lost emitted code lost retired
instructions and machine code with them, and the other ten are
byte-identical in all three veins — the same four, three ways, which is
what one per-program test going away looks like:

    escapebench  work     85,558,106 -> 85,558,078     text 57,922 -> 57,490
    indexbench   work      3,265,849 -> 3,265,819      text 62,162 -> 61,730
    scanbench    work    587,488,506 -> 587,488,450    text 159,842 -> 158,930
    runbench     work  2,252,446,969 -> 2,252,446,915  text 247,474 -> 247,026

The falls are tens of instructions because the test ran once per program
rather than inside a loop. The compile rows RISE, and that is the refusal's
own cost: check_box_where_value is a new whole-program pass reading infer's
return sets.

    compile_instructions    49,097,584 -> 50,824,229   +1,726,645  +3.5167%
    entry_instructions     164,060,471 -> 170,001,743  +5,941,272  +3.6214%
    library_instructions   164,342,505 -> 170,286,677  +5,944,172  +3.6166%
    compile_allocs              29,350 -> 29,396              +46  +0.1567%

compile_peak_bytes holds at 774,660. The objective's compile term is the
module and entry rows summed: 213,158,055 -> 220,825,972, +3.5970%.

**So welfare falls 67.59 -> 67.52, and the floor is Clay's to move.** Every
term that moved is a compile term, the fall is 0.07, and the 2026-08-25
language clause is what has covered a ruled feature's compile cost three
times before (#1355, #1356, #1359). The hand edit that clause calls for is
refused in this session by the permission classifier, exactly as on #1369,
so this branch and that one now wait on the same permission.

**The differential the plain dot broke, and the three it hid.**
`scripts/effects_differential` writes its 31 programs as source strings,
and 25 of their effect binds were spelt with the plain dot — so with the
dot an application they stopped binding and 15 of the 31 went wrong. They
are respelt `.>` here, the same respell kq#102 made and this change already
made in the fixtures. Three of the 31 kept passing and are respelt too:
`(_ -> 7) (io/write "")` happens to answer what the bind answered, so
`re_enter`, `computed` and `group_of_binds` were green for the wrong
reason and had stopped testing what they are named for. The step's failure
also SKIPPED `dispatch_differential`, `module_differential` and
`diagnostic_coverage`, which had therefore never run on this branch; all
three are green (22 cases, 36 modules, 319 diagnostics).

**Spec.** `tests/golden/micro/a_plain_dot_hands_the_box_over.kso` on native
and the oracle: `math/random 6 . held` rendered as `held <io>`, the same
box bound with `.>` after the plain step, and a missing file's read handed
through `held` and rescued with `.?`. The error fixture above. The eleven
runtime fixtures, rewritten, and the pre-change binary's answers on the
micro fixture and the error fixture are the watched-red half.

**Owed.** Two things: the `<t>effect` spelling, which the canonical-spacing
rule refuses today, and ch04 and ch05, per the ledger's "The book teaches the
boundary language". NOT the drop question — a draft of this paragraph listed
it as a third, and the same 2026-08-29 sitting had already closed it, in the
archive's "gavel: the drop question closes — explicitness IS the guarantee".
It closed by declining to mint anything: an unused binding is already a
compile error, so a dropped effect is already unspellable, and Clay ruled the
premise backwards. "No new checker rule and no io-edge rule is minted."

---

## 2026-09-11 — the ledger forked onto feature branches, and the queue's three language rows wait on one branch

**design/pending-gavels.md had an empty Blocking section on main while two
entries existed.** Both were written, both followed the file's rules, and
neither was ever merged: "Where the box wraps under the pure-fallibility
rider" sat on local/pure-fallible, and "The reconstruction the 2026-09-07
ruling ordered has two usable phases" sat on four branches at once. The file's
own header names this exact failure -- "Edits to this file ride small,
promptly-merged PRs, never a feature branch, so the ledger cannot fork" -- and
it happened anyway, because a feature branch is where the measurement that
raised the question was taken.

Clay read STATUS.md, saw "Blocking right now: nothing", and was told by this
session that two things waited on him, cited by session task number. A task
number resolves nowhere outside the session that made it, which the file's
rules also say. So for a day there was nothing he could look up and nothing he
could rule.

Only one of the two comes back. The reconstruction entry was ruled on
2026-09-10 -- "Rows 15..390 stay unscored", option (1), closing with "Nothing
further is owed on this entry; it leaves the ledger with this ruling" -- so
the branches carrying it hold a pre-ruling snapshot and it stays out. The
box-wrapping entry is carried here verbatim, with its citation, its
measurements and its recommendation intact.

**The three "Ruled, unbuilt" rows are one design and are blocked on one
thing.** The effect type, the pure-fallibility rider that rides with it, and
the book chapter that waits on both are built or buildable; what they lack is
a branch. This session may push to claude/go-to-town-m0dicm alone, kanso#1369
is sitting on it, and #1369 cannot merge because staging
bench/welfare_floor.json is refused by the harness as a CI bypass. The rows
stay, with this sentence as the blocker.

**kq's half is done and waits on nobody.** The gavel ends the automatic bind,
and kq had 25 plain-dot effect binds that break under it. They are respelt
`.>` on kq's claude/go-to-town-m0dicm (baec530), watched red first -- the
plain-dot compiler dies at kq's `== build ==` step on `length takes a list,
string, or map, not <io>`, after the ten unit tests pass -- and green
afterwards on that compiler and on today's, so kanso CI keeps a buildable kq
to clone through the transition.

The search that reported both trees clean was `^\s*\. [a-z_]`, anchored to
line start, and not one of kq's 25 sites begins a line. It found zero, and the
conclusion "the failing site is kq's source" was drawn from it anyway.
Unanchored, ` \. ` finds all of them, in main.kso, query/cli.kso and the three
bench gates.

**The exhaustiveness rule's compile cost cannot be optimised away, measured.**
Before accepting the floor move on #1369, two ablations were run to see whether
the rise could be given back instead. Both on this container, callgrind,
`kanso::main` inclusive, `./kanso check compile_corpus` in
/tmp/kanso-compile-ir under the gate's own pinned GLIBC_TUNABLES and `env -i`:

    branch as it stands                 50,959,782
    a could_yield_none prefilter        50,920,876   -38,906
    check_none_exhaustive's walk gone   50,472,794   -486,988

The module row has to fall about 1,130,000 to reach main's 49,097,584. So
skipping the callee lookup on arguments that cannot yield none buys 3.4% of the
rise, and deleting the rule's whole traversal buys 43%. The prefilter was
proved behaviour-identical first -- golden 11/11, error corpus and micro corpus
across the engines -- and is still not worth landing at that size. Both are
reverted.

The entry that shipped the rule says what is left is "the check's own walk and
the shadow load". These numbers split that: the walk's lookups are 38,906 of
it, its traversal 486,988 in total, and the remaining ~643,000 is the keyed map
build and the shadow load. The rule needs all three, which is why the fall
stands rather than being bought back.

## 2026-09-12 — the formatting appendix knew two continuation forms and there are five

kanso#1364 minted `.>`, `.!` and `.?`, and each is legal at the head of a
continuation line. The evidence was already in the corpus rather than in a
fixture written to argue this: `tests/golden/micro/a_chain_step_names_its_
channel.kso` wraps a `json/decode` onto three continuation lines headed
`.>`, `.!` and `.?`, and `kanso check` answers ok on it. Appendix C said
"there are exactly two continuation
forms ... `.` for a data-flow pipe, and `>>` for a pure sequence", and its
closing summary of the whole law repeated the pair. Both name all five now.

A third sentence introduced the wrap_pipe panel as "wrapped onto `.`
continuation lines" where the panel holds one `.` line and one `.>` line; it
no longer counts them. That mixed spelling inside one chain is on main and is
left alone here: changing the sample moves a golden, and the sentence was the
thing that was wrong.

This is independent of the effect-type sequence. The appendix has been wrong
since #1364 landed, which is why it lands on its own rather than behind the
plain dot becoming an application.

## 2026-09-12 — the box check's second map hashed the same name again

Searched the log, the archive and design/ before filing: the two-maps-on-one-key
shape appears in the 2026-09-11 entry for kanso#1369, which found it in the
exhaustiveness pass and collapsed it. Nothing had looked for the same shape in
`check_box_where_value`, which is the pass kanso#1372 adds.

**What it was.** The pass built two tables and both keys started with the
declaration's name:

    returns: Map<(&str, usize), Set>
    binds:   Map<(&str, usize, usize), bool>

`returns` answers what a group's arms return; `binds` answers whether the group
binds anything at one position. So the name was hashed once per PARAMETER to
build the second table and once per ARGUMENT to read it, on top of the hash the
same call site already paid for the return set. A call of arity three cost four
hashes of one string where it needed one.

**What it is now.** One table, `Map<(&str, usize), (Set, u64)>`: the per-position
bool is a bitmask beside the set. Building is one hash a declaration, reading is
one hash a call site. A position past the mask's width reads as BINDING, the
under-refusing direction — the same choice kanso#1369 made for its own mask, and
for the same reason: a refusal this pass cannot justify is worse than one it
declines to make. The widest group in lib/ takes five parameters against a width
of sixty-four.

**The measurement.** Read in this container, which counts high against CI's
rustc but reads a delta that carries:

    compile_instructions  51,679,441 -> 51,403,565   -275,876   -0.534%
    binary sha            8655e4c48e8a -> a81dbd7e5b03

CI read four rows on 23dd9733, and they are what the goldens now carry. The
entry and library paths fall harder than the module path, which is the shape to
expect: the pass walks every call site in the merged program, and the entry
corpus names ten imports where the compile corpus names four.

    compile_allocs         29,396 ->      29,386          -10   -0.0340%
    compile_instructions   50,824,229 ->  50,544,369  -279,860   -0.5507%
    entry_instructions    170,001,743 -> 168,998,559 -1,003,184   -0.5901%
    library_instructions  170,286,677 -> 169,284,150 -1,002,527   -0.5887%

`compile_peak_bytes` held, every runtime vein held, and `emitted` held. The
objective's compile term is the first two summed: 220,825,972 -> 219,542,928,
-1,283,044.

**Watched red first, in both directions.** The errors corpus fixture
`a_box_where_a_value_is_expected` exercises the mask on both sides in one
program: `fn told 0` / `fn told 1` take literal patterns, so position zero does
not bind and the call is refused; `fn held e` takes a Var, so it binds and the
call is not. Building the mask with the bit never set makes `held` gain a
refusal it should not have. Removing the pattern test, so every position sets
its bit, loses the `told` refusal entirely. Both were run and read before the
change was restored.

**What it does NOT do.** It does not pay for the pass. `check_box_where_value`
costs the compile rows about 3.5% and this returns about a sixth of the welfare
that costs: the compile term's saturating factor moves from 0.8588 to 0.8595,
worth roughly 0.011 points against a 0.07 shortfall. The floor still moves by
hand under the 2026-08-25 language clause, and the entry it moves under names
this paydown, so a reader can see what was tried before the floor moved.

**Every row of "Ruled, unbuilt" waits on this one, and the reorder that would
avoid that does not exist.** Written here because a blocked row owes a sentence
naming its blocker, and until now those sentences lived only in a session's
task list, which resolves nowhere outside that session.

Four rows stand in STATUS.md. Two are builds waiting on the welfare floor and
nothing else: this one, and kanso#1369 for the 2026-08-15 exhaustiveness
ruling. The floor shows up in three CI jobs rather than one — welfare, specs
and the macos host — because `tests/the_digest_is_priced_on_both_sides.rs`
runs welfare against undoctored goldens and asserts it exits 0, so it fails
wherever welfare does, and `cargo test --no-fail-fast` carries that target onto
the other host. One cause, three reds, and they clear together.

The third row, the book's ch04/ch05, waits because `book_check` executes every
panel and compares its output: the prose cannot describe the effect type before
it ships. The fourth, the pure-fallibility rider, waits twice — on this PR, and
on the ledger's Blocking entry asking where the box wraps.

The way to keep the section moving while the floor sits with Clay would be to
land the `<t>effect` spelling first, since the dependency recorded for it is
that it "edits a golden this one creates". That reading is too narrow and the
diff says so. Cherry-picking the spelling onto main conflicts in src/check.rs,
and every line of the conflict is a change to `check_box_where_value`: the
map's value type, the group members threaded through it, and the arm that lets
`e:<int>effect` take a box. The spelling amends the pass this PR introduces, so
the order is fixed by the code rather than by a golden, and the section stays
behind the floor.

**The box check asks a question the program has already answered, and skipping
it is 43% of the pass.** DONE. Searched the log, the archive and design/ for a
prior entry on `check_box_where_value`'s cost: there is none — the pass landed
in kanso#1372's step 1 and nothing had priced it.

The pass is the whole of this branch's welfare fall, and the fall is entirely
the compile term. Against main, runbench moves 54 instructions of 2.25 billion,
both peak terms and `compile_allocs` are flat within 36, and the two compile
instruction rows carry all of it: `compile_instructions` 49,097,584 ->
50,544,369 and `entry_instructions` 164,060,471 -> 168,998,559, summing
+6,384,873 on the objective's compile term. Gating the pass behind an
environment variable and measuring both ways in the box puts its whole cost at
1,491,247 on the module corpus against CI's +1,446,785, so the pass IS the rise
and its own cost is the ceiling on recovering it.

Where it goes, by profile diff of the two runs: 587,749 in the walk's own
`site` loop, 367,382 in `for_each_child`, 314,707 in the binder set
(`HashSet::insert` building it and `contains_key` reading it), 75,689 in
`memcmp` under those hashes, 73,455 building the returns table, 51,740 in
`for_each_param_name`. The binder set and its lookups are a quarter of the
pass, and they exist for two arms of `yields_box` that ask whether a name or a
call head answers a box. When no declaration in the program answers one, that
table lookup is false for every entry by construction, so both arms answer no
without asking — and the set they consult is then never read, so the second
walk of every body that builds it is never taken either. What survives is the
chain, which the expression says on its own.

`any_boxed` is that question, asked once over the returns table. On the module
corpus the pass falls 1,491,247 -> 849,940 and the compile reads 51,403,565 ->
50,766,270 (−637,295); on the entry corpus 171,792,376 -> 170,483,527
(−1,308,849). Summed, 1,946,144 of the 6,384,873 — 30% of the fall, measured in
the box; CI has still to price it and the goldens here are CI's to write.

The remaining 70% is the walk itself, and it is not reachable the same way: the
check has to visit every expression to find a chain in a value position, and
after the hoist that walk is what is left. Two shapes were measured and are
NOT worth carrying. Skipping the synthetic twins, which thirteen other checks
in check.rs do, reads +26,323 rather than a saving — the twins' bodies are
shared but they are not where this pass spends. And narrowing the walk to
declarations that contain a chain needs a walk to answer, which is the walk.

Behaviour is unchanged by construction rather than by measurement: the guarded
arms return exactly what the table would have returned, and the set is read
only from inside them. `tests/golden/errors/a_box_where_a_value_is_expected`
takes the other branch — `os/args` and `math/random` answer boxes — and all six
of its refusals still fire. Ratchet row `box_check_hoist` with mutation
`the_box_check_asks_when_nothing_answers_a_box`: answering `any_boxed` `true`
puts the module corpus back to 51,467,472 (+701,202), and the gate asserts
equality, so it turns red.

This does NOT take the branch green. The floor still has to move, by less; the
ledger's "The welfare floor cannot be staged from this session" is unchanged
and still the blocker.

**CI's sitting for the hoist.** Four host-keyed compile veins moved, all falls,
every runtime vein byte-identical and `compile_memory` unmoved at 774,660.
`compile_instructions` 50,544,369 -> 49,937,088 (−607,281 / −1.2015%),
`entry_instructions` 168,998,559 -> 167,770,493 (−1,228,066 / −0.7267%),
`library_instructions` 169,284,150 -> 168,056,074 (−1,228,076 / −0.7255%),
`compile_allocs` 29,386 -> 29,374 (−12). The objective's compile term is the
first two summed: 219,542,928 -> 217,707,581, a fall of 1,835,347, against the
1,946,144 this container projected — 6% high, the direction and the size the
container's offset has had on every compile row.

The entry and library rows fall within ten instructions of each other on the
same change, which is kanso#1344's finding restated: those two corpora name the
identical ten imports and are the same measurement.

Against main the compile term now stands at +4,549,526 rather than +6,384,873,
so 28.7% of the fall is recovered and the floor still has to move for the rest.

**Two implementations that agreed exactly were reported as disagreeing, and
the fault was rounding a rounded number.** DONE. Searched the log, the archive
and design/ for a prior entry on the score comparison: there is none.
`the_score_says_what_it_was_made_of` has compared welfare's banner against the
rescorer's column since the column was minted, and has been wrong at a boundary
the whole time without anything reaching one.

The hoist above put welfare at 67.54499292290286, which is 67.5450 in the four
places the history column carries and 67.54 in the two the banner prints. The
spec read the column, rounded it to two, and got 67.55. Anything in
[67.5445, 67.5450) reads that way; nothing had landed there before. The macos
job failed on two targets rather than one for this reason, and the first
reading of that job here called them one cause, which was wrong.

`welfare --score` prints the column's own precision and the spec compares the
two as they are written. That is a hundred times tighter than what it replaced
rather than looser: perturbing the rescorer's satiation by one part in ten
thousand now reads 67.5432 against 67.5450 and turns the spec red, where the
old two-place comparison rounded both to 67.54 and passed. Watched both ways —
red on the real defect before the fix, red on the injected drift after it.

The flag reports and cannot ratchet, which is what the file's existing
`asking_what_was_scored_does_not_move_the_floor` exists to hold for
`--counters`; `--score` reads the same value the banner does and writes
nothing.

**A call asks its arguments before it asks the table, and the binder walk was
not the cost.** DONE. Searched the log, the archive and design/ for a prior
entry on `check_box_where_value`'s per-site cost: there is none.

The effect-type pass walks every expression in every declaration and asks, at
each call site, whether an argument is a box where a value is wanted. It asked
by hashing the callee's name into the returns table first, then looking at the
arguments. A call whose arguments are literals, arithmetic or field reads can
never be refused, and there are a great many more of those than there are box
arguments, so the hash was paid on almost every call in the corpus to learn
nothing. Asking the arguments first is a match on an enum; the table is now
consulted only where a refusal is actually in question.

    compile_instructions   50,766,270 -> 50,516,758  (−249,512 / −0.4915%)
    entry_instructions    170,480,464 -> 169,706,205  (−774,259 / −0.4542%)
    summed                221,246,734 -> 220,222,963  (−1,023,771)
    compile_allocs            29,374 -> 29,374        (unchanged)

Both rows read at container levels, which sit about 0.8% above CI's on every
compile vein. The summed fall is 22.5% of this branch's +4,549,526 excess over
main. Welfare 67.54499292 -> 67.55402761, closing 21.9% of the 0.04120172 gap
to the floor; 0.03216703 still stands and the floor still has to move for it.

**The binder walk was the hypothesis and it was wrong.** Before building this,
the per-declaration walk that collects bound names looked like the cost: it
runs once per declaration whenever any group returns a box, and it is a second
full traversal of the body. Ablating it — `if any_boxed {` to `if false {` —
read 50,773,306 against the 50,766,270 baseline, slightly WORSE. A lazy or
on-demand binder set would have gained nothing at all. One build, before any
design.

Where the cost actually is: `check_box_where_value` is 862,916 instructions of
the module compile (50,766,270 with it, 49,903,354 with the whole pass ablated),
which is essentially the entire module-side rise this branch carries. The entry
compile carries the other 82% of the excess and is not this function.

The guard is load-bearing and was watched red: replacing it with an
unconditional `return` loses exactly the two call-arm refusals in
`tests/golden/errors/a_box_where_a_value_is_expected.kso` — ``told`` at 19:19
and ``length`` at 17:17 — and leaves the four the BinOp, Index and Field arms
raise independently. Six refusals before, six after.

**The table answers before the binder set does, and the guard nothing could
fail.** DONE. Searched the log, the archive and design/ for a prior entry on
`yields_box`'s lookup order and on coverage for the shadowing guard: there is
none.

`yields_box` asked two hashes of the same name — is it locally bound, then does
the returns table hold it as a box — and asked them in that order. A name the
table does not hold, or holds as something other than a box, is not a box
whoever bound it, so the shadowing question only has to be asked of the few
names that come back boxed. Boxed names are rare; locally bound names asked at
these arms are not as common as the old order assumed.

    compile_instructions   50,516,758 -> 50,437,442  (−79,316 / −0.1570%)
    entry_instructions    169,706,205 -> 169,234,614  (−471,591 / −0.2779%)
    summed                220,222,963 -> 219,672,056  (−550,907)

With the argument test above, this branch has now paid back 1,574,678 of its
+4,549,526 excess over main, 34.6%. Welfare 67.55402761 -> 67.55887; the floor
still has to move for the rest.

**The guard had no coverage anywhere, and writing the fixture found a
divergence.** Deleting `&& !bound.contains(name)` from both arms leaves the
whole golden suite green: eleven tests, error corpus included, and lib/json
still compiles. The check was load-bearing and nothing could fail if it went.
`tests/golden/micro/a_bound_name_is_its_binding_not_the_group_it_spells.kso`
closes that: `fn doubled args` multiplies its own parameter, an import makes
the bare `args` reach `os/args`, and without the guard the line is refused.
Watched red — the corpus fails on that sample alone under the deletion.

The fixture was first written to cover both arms and the second half would not
run. `fn sized random n` with `random n` in the body dispatches to
`math/random`, not to the parameter, so the program answers `<io>` where the
effect check has already decided the parameter wins. The bare name and the call
head disagree: `args` as a value is the parameter, `random n` as a call is the
import. That predates this pass — nothing here can change dispatch — and it
means the call-head arm of the guard declines a refusal the program would have
earned. It is written into the fixture's header rather than pinned, because
which side is right is a language question.

**The binder set is built on first ask, and most declarations never ask.**
DONE. Searched the log, the archive and design/ for a prior entry on the effect
pass's binder walk: the entry above is the only one, and it refuted a different
hypothesis about the same walk.

The walk is a second full traversal of every declaration's body. `bound_in_expr`
visits every expression to find the names lambdas introduce, and the only reader
is the shadowing test — which the entry above moved behind the returns table, so
it now runs for the few names the table holds as a box. Filling the set at the
first of those asks answers that ask with exactly the set the eager build would
have handed over, because the fill happens before the answer rather than after.

The walk was priced by running it twice on an otherwise unchanged binary:
169,800,207 against 169,234,614, so one walk is **565,593** instructions of the
entry compile. The lazy fill recovers 516,408 of that, 91%.

    compile_instructions   50,437,442 -> 50,451,376  (+13,934 / +0.0276%)
    entry_instructions    169,234,614 -> 168,718,206  (−516,408 / −0.3052%)
    summed                219,672,056 -> 219,169,582  (−502,474)
    compile_allocs            29,374 -> 29,374        (unchanged)
    compile_peak_bytes       774,660 -> 774,660       (unchanged)

**The module row rises, and the reason is that it had nothing to save.** The
module corpus imports std/json, std/list, std/testing and std/text and no
effect-bearing module, so `any_boxed` is false there and the eager walk was
already skipped for every declaration. What the module row pays is the
measurement itself: `site` takes a `&dyn Fn` where it took a `&HashSet`, a fat
pointer instead of a thin one, on every expression in the corpus. The first cut
constructed that closure per EXPRESSION and read +21,363; hoisting it to once
per declaration brought it to +13,934. The entry corpus, which does name
effects, pays the same and saves the walk, so the sum falls by 502,474 — a 37:1
trade, and the objective sums the two rows.

Across this branch the pass has now paid back 2,077,152 of its +4,549,526
excess over main, 45.7%. The floor still has to move for the rest.

Watched red: with the fill never taken (`if b.loaded != i` to `if false`) the
set stays empty, `shadows` answers false everywhere, and the micro fixture above
fails on that sample alone.

**What the effect check costs when it costs as little as it can, measured
rather than argued.** DONE. Searched the log, the archive and design/ for a
prior ceiling on this pass: the three entries above are the only ones, and none
of them asked this question.

Three ablations on the shipped build, container levels, `kanso::main` inclusive:

                              with pass     without pass      the pass
    module (compile_corpus)   50,463,222    49,922,292         540,930
    entry  (entry_corpus)    168,748,436   166,725,054       2,023,382
    summed                                                   2,564,312

The remaining excess over main is 2,472,374, and those two agree to within
92,000 — about 20,000 of it the container's standing 0.8% offset from CI, the
rest the eight lines this branch adds to src/parser.rs, and layout. The excess
is this pass and essentially nothing else. Twice on this branch that was
guessed otherwise, so it is now measured.

Splitting the pass into its traversal and its per-node work:

    entry, walk + site   168,793,445
    entry, walk only     168,124,479
    site's own work         668,966
    traversal + table     1,354,416

The first attempt at that split put `std::env::var_os` inside the walk loop and
both readings came back ABOVE the un-ablated baseline, the ablated one highest —
a missing key walks the whole environ on every expression, and a present one
stops early. Hoisting the flag out of the loop gives the reading above. A gate
read per node measures the gate.

**The traversal is the reachable part and the check is not.** Twenty-odd
whole-program checks in `check_merged_after_aliases` each walk every expression,
three of them holding the same `&inference`; fusing this one into a neighbour
recovers roughly the 1,354,416 on entry plus its share of the module row. What
stays is the match on each expression, the table lookup on each call and the
refusal — the check itself, about 0.9M summed. A language feature that refuses a
box where a value is wanted costs the front end something to decide, and the
objective reads that as a fall however it is arranged.

So the floor edit is needed, and the paydown has taken it from 4,549,526 to
2,472,374 with a fusion plausibly reaching ~800,000. The fusion is filed as its
own lead rather than ridden here: it reorders diagnostics across two dozen
checks and regenerates the error corpus, which should stand on its own.

**CI's sitting for the three paydown rounds.** The three entries above quote
container readings; these are the landed values the goldens now carry, read off
the cost-goldens job on 35847282.

    compile_instructions   49,937,088 -> 49,626,153   (−310,935 / −0.6226%)
    entry_instructions    167,770,493 -> 166,036,229  (−1,734,264 / −1.0337%)
    library_instructions  168,056,074 -> 166,320,043  (−1,736,031 / −1.0330%)
    compile_allocs            29,374 -> 29,374        (unchanged, green)
    compile_peak_bytes                                (unchanged, green)

The objective's compile term is the first two summed: 217,707,581 ->
215,662,382, a fall of 2,045,199 against the 2,077,152 this container projected
— 1.5% high, the direction and rough size the container's offset has had on
every compile row.

Eighteen of the twenty-one veins in that job were green before this
regeneration and the three that were not are these. Nothing else moved: the
work rows, the machine-code rows, the memory rows and the run program's
counters are all byte-identical, which is what a change confined to one
whole-program check should look like.

The entry and library rows fall within 1,767 instructions of each other, which
is kanso#1344's finding for the third time on this branch: those two corpora
name the identical ten imports and are the same measurement.

Five spans on compiler.html quote these goldens and `all_pages.sh --write`
rewrote them. A sixth thing on that page was stale in a way no gate can see —
the library row's paragraph said two changes had moved it since, and there are
now five — so that sentence is edited by hand rather than regenerated.

## 2026-09-12 — the exhaustiveness rule pays down two of its three costs

kanso#1369 is built and blocked on the floor, and while it waits the pass it
adds is the largest single compile cost on either open branch. Two changes,
each measured on its own, on the branch rather than on main.

**A call asks its arguments before it asks the returns table.** The walk
consulted the table at every call site with an identifier head, and that
lookup hashes the callee's name where the none question is a match on the
argument's shape. Most call sites hand over literals, arithmetic or field
reads and answer no on the match alone. Summed 220,373,766 -> 220,182,802,
-190,962 (-0.0867%). The same shape kanso#1372's round four found in the
effect check; the three early returns are the same three conditions
reordered, so no diagnostic moves.

**The shadow table accumulates instead of re-deriving.** It said, for every
arm and every position, what every arm above takes there, walking each
earlier arm's whole parameter list once per position. The answer grows by one
arm at a time, so each arm now reads the running total and folds in its own,
and whether an arm settles a position is one count of its parameters rather
than one scan per position. Summed 220,182,802 -> 220,117,045, -65,757.

The interesting part of that second one is the first cut, which measured
294,981 WORSE. Skipping single-arm groups is what makes it pay: the work the
running total saves lives in long groups, which are rare, and the per-arm
count it adds lands on every group, and most groups are one arm. The entry
of 2026-09-11 above put the keyed map build and the shadow load together at
about 643,000; this pays down the build side of that pair and leaves the
load, which measures 610 and is not worth a shape.

**A figure that is available and does not ship.** Isolating either loop by
ablation needs the mask in `widen_param` ablated too, or the shadow values
move and the fixpoint moves with them. Under that barrier the old build
reads 984,991 and the new one 440,448 — and those are not the change's
delta, because `black_box` there changes how the whole of infer inlines, and
that function's inlining already carries a pinned attribute and a measurement
saying why. The shipping numbers above are end-to-end with the mask live and
the two tables proven identical.

Watched red first against the derivation rather than a downstream effect:
the old loop was kept beside the new one and the two tables asserted equal
over the whole compile corpus, where the table holds dozens of live entries.
An off-by-one letting an arm read its own contribution trips it on the first
module. The scaffold is removed; golden 11/11.

The whole pass, ablated, is 2,500,754 of the branch's compile cost and the
shadow machinery another 985,601, against roughly 3.9M the branch carries at
container levels. What is left of that is the traversal, which is kanso#487's
and not this pass's alone.

**CI's rows for the two paydowns**, landed the round after. compile_instructions
50,228,060 -> 50,022,458 (-205,602), entry_instructions 167,038,742 ->
166,387,431 (-651,311), library_instructions 167,802,001 -> 167,177,778
(-624,223). The objective's compile term is the first two summed: 217,266,802
-> 216,409,889, **-856,913**.

That is 3.3x the -256,721 this container read for the same two commits, and the
direction of the disagreement is worth writing down rather than smoothing over.
Both figures are before-and-after on one host, so neither is a host offset in
the usual sense; what differs is the toolchain (container rustc 1.94.1 against
CI's 1.98.1), and compile_instructions is a layout vein whose deltas move with
inlining. The container sized the change and got the sign right; CI priced it.
Neither number is wrong and only CI's is the row.

compile_allocs ROSE, 29,473 -> 29,485, +12. The running-total rewrite needs one
scratch vector where the derivation it replaces used a scalar, and that vector
is the only allocation the change introduces -- which makes it the candidate
and not a proven cause, since nothing has measured the two apart. It sits in
the same welfare term as the -205,602, so the objective reads the pair
together; the compile corpus is one file importing four modules, so the "one
vector per module" story that would explain a twelve does NOT fit it, and that
is the reason this is written as an open attribution rather than an answer.
## 2026-09-12 — a necessary condition beats a shared descent, and the corpus says why

The whole-program checks in src/check.rs each walked every expression of every
non-synthetic declaration. kanso#1374 fuses the ones that can share a descent
and moves a cheap test to the front of two that cannot. CI's compile term
(compile_instructions + entry_instructions) reads 213,158,055 -> 207,750,743,
a fall of 5,407,312 (-2.5368%). The container projected -5,477,199 over the
same two rows: the same 2.53% either way, with the 69,887 absolute gap the
container's known high offset carrying through.

    main                                    216,579,492   (container)
    + if_arity, boolean_equality,
      none_in_collections                   214,713,055  -1,866,437
    + foreign_constructions,
      typeset_constructions                 214,695,155     -17,900
    + err_as_value, call_shaped_list        214,254,130    -441,025
    + the literal-argument condition        211,971,748  -2,282,382
    + the tie check's settled scan          211,102,293    -869,455

THE PUBLISHED RATE WAS NOT A RATE. The first commit measured 933,000
instructions a descent and that figure went into kanso#1374's body as the
number to plan thirteen more against. Two rounds refute it twice over. First,
a check with an emptiness guard was never descending: a probe printing the
table sizes says `annotating` is EMPTY at all twenty-five compiles in the two
corpora, so `typeset_constructions` visited no node at all, and
`foreign_constructions` walked at seven of the twenty-five. Fusing that pair
removed no descent, and its 17,900 is the App-with-an-Ident-head destructure
now done once per call node instead of three times. Second, what a descent
costs depends on the walk removed: folding `err_value_scan` and
`call_shaped_walk` together is worth 441,025, under half the first reading.

THE CONDITION BEATS THE FUSION, three times over. `check_literal_arguments`
can only speak about a call that has a literal argument, and it asked that
last -- after a hash of the callee against the local bindings, a second
against the builtin aliases, a qualified-name split, and a third against the
dispatch groups. `check_arm_ties` scanned every other arm of a group looking
for one that settles a tie, for every OVERLAPPING pair, when only a
CONFLICTING pair can be settled. Both tests were already computed or nearly
free. Together -3,151,837 against -2,325,362 for all three fusions. This is
what kanso#1168 and kanso#1369 already recorded and this session did not carry
over: the fusion removes the frame around the work, the condition removes the
work.

AND THE CORPUS SAYS WHERE THE FAMILY ENDS. Two guards of the shape "skip this
pass unless the program uses feature X" were tried and both measured nothing:
the typeset fusion above, and a `door_advisories` guard on "does any declared
type name carry a slash" at +167, reverted. The reason is a property of the
workload rather than of either pass. bench/compile_corpus.kso and
bench/entry_corpus import ELEVEN std modules -- bits, io, json, list, math,
path, regexp, render, sha256, testing, text, every one there is -- so the
merged program uses everything and no such guard can fire.

That closed a third candidate without building it. `provenance::analyze` is a
200-round fixpoint costing 3.1M on the entry compile and reports only through
`violations`, which needs a parameter that receives an err. All of lib/ has
exactly one, `lib/testing/testing.kso`'s `when_failed (err reason)`, and the
corpora import std/testing. Two greps instead of a build-and-measure round.
What still pays is the other shape: a condition that fires per NODE rather
than per program, which both of the two above are.

Left out of the fusion with reasons: `check_decidable_failures` prunes an
`if`'s branches on purpose; `check_field_exists` carries `open`, a Vec
accumulated as it descends, so its question is a function of the walk's
history rather than of the node; `check_bare_ambiguity` returns on an empty
`torn`; `check_binding_patterns` never descends.

compile_allocs did not move, and the cost-goldens job's own vein summary says
so: these changes reorder tests and share frames, they allocate nothing new.
The gain lands unratcheted -- `welfare --set` is refused by this session's
permission classifier -- so the floor stays and kanso#1369 and kanso#1372 are
free to spend the headroom. This takes 5,407,312 of the 7,325,192 those two
need, 73.8%; the rest is about 0.016 welfare and still Clay's.

## 2026-09-12 — the fusion dropped diagnostics, and the unratcheted gain was a blocker not a gift

Two corrections to the entry above, both found by CI rather than by me.

THE FUSION DROPPED DIAGNOSTICS. `check_per_node` put three checks on one
descent, and two of them -- `check_boolean_equality` and
`check_none_in_collections` -- had been running AFTER inference. The descent
runs before it, in front of the `if !diags.is_empty()` guard that returns
without ever calling `infer`. So a program whose only fault was `b == true`
returned from that guard and skipped every check after it: the boolean naming
rule, the call arities, the field-existence check, the literal-argument check,
silently. Two lines were enough to show it -- `pub fn silly b / b == true`
reports two diagnostics on main and reported one on the branch.

The guard exists for one reason, written beside it: inference indexes an
`if`'s three children and must not run over a shape with fewer. That is
`if_arity_at`'s guarantee alone. It reads the arity answer back off the
diagnostic's kind now and retains only that, and `rotate_left` puts the walk's
other two answers back at the END, where `check_boolean_equality` used to
push. This route hands diagnostics back in push order -- only the gated return
sorts -- so where a check pushes is what a reader sees, and the first cut of
the fix left them in front and turned `tests/errors_module.rs` red on the
order alone.

WHAT COULD NOT SEE IT. Not the flat error corpus: all 201 fixtures stayed
green through the whole regression. `comparing_to_a_boolean_literal`,
`none_in_list` and `none_in_map` each carry exactly ONE diagnostic, and a
fixture with one diagnostic cannot see a suppression. `errors_module` caught
it, on a module tree whose library carries two faults -- a different test
target, which is why the local `--test golden` run said nothing. Two fixtures
close the gap, one per question the walk asks beside the guard, and under the
exact pre-fix gate they read 1 against a golden of 2 while `none_in_list`
reads 1 against 1. That third row is the finding.

A mutation is not a proof unless it is the right mutation. The first attempt
flipped the gate but left the `retain`, so it returned an EMPTY vector -- a
worse bug than the original -- and the fixture went red for a compounded
reason. Redone against the gate as it actually stood.

THE SHADOW SET, DEFERRED. Off the callgrind attribution rather than a guess:
`arity_at` reads a declaration's bound names only to SUPPRESS a diagnostic,
and collecting them walks every parameter and statement before the walk that
might need one. Built now only when something was pushed to suppress. CI:
compile_instructions -535,699, entry -1,735,146, library -1,736,636,
compile_allocs -12.

Three shapes that would reach the filter -- a binding beside a declaration in
the same file, a binding shadowing a builtin, a binding shadowing a
declaration in a SIBLING file -- are all refused earlier by the shadow check
with `X is already a declaration`. The third is the one `arity_at`'s own
comment anticipates. That is recorded, not claimed: three probes are not a
proof of deadness, and the change does not rest on one. It is safe because the
filter is preserved verbatim and only its input is built later.

THE CONTAINER'S OFFSET IS NOT A CONSTANT. The entry above called it "the
container's known high offset carrying through" at 2.53% high. This round the
container projected -1,834,916 across the module and entry rows summed where
CI read -2,270,845 -- LOW by 435,929, 23.8%. Two rounds, two directions. It is
not a correction to apply; it is why the rows are CI's to write.

AND THE UNRATCHETED GAIN IS A BLOCKER, NOT HEADROOM. The entry above says the
gain "lands unratcheted ... so the floor stays and kanso#1369 and kanso#1372
are free to spend the headroom." That is wrong. `the_undoctored_goldens_hold_
the_floor` fails a rise that nobody banks -- "welfare 67.63 floor 67.59 ...
the gain is not held" -- so the PR cannot merge until `welfare --set` runs,
and the headroom is not released to anything. The refusal of `--set` by this
session's permission classifier is therefore a merge blocker on kanso#1374
rather than a footnote in its body, and it has gone to Clay.

The arithmetic those two PRs were measured against also moves: kanso#1374 now
takes the compile term from 213,158,055 to 205,479,898, a fall of 7,678,157,
where the figure quoted above for what kanso#1369 and kanso#1372 need was
7,325,192. On compile instructions alone that is now more than covered.
Whether either goes green is a welfare question over five counters and has not
been recomputed here.

## 2026-09-12 — a declaration's callees deduplicated by range, and a scan whose answer was thrown away

Two changes in `src/infer.rs`, both compile-cost paydown, off main at 0b828f66.

**The dead scan.** `ident_set`'s fallthrough arm walked `program.fns` twice
with the same predicate. The first walk collected each matching declaration's
parameter count into `arities`; the next statement was `let _ = arities;`,
which is why no lint ever objected to a vector with no reader. Deleting it left
the second walk — the one that widens those parameters to TOP — doing the work
alone. Summed −102,197 (−0.0472%) on this container. Dead since abefb574, the
original whole-program inference commit.

**The range sort.** `callee_first` gathers every name a declaration's body
mentions into a `Vec<&str>`, sorts it, deduplicates it, and looks each survivor
up in `by_name` to append that group's members to `flat`. The sort compares
strings, so it is an insertion sort's worth of `memcmp` per declaration, 1,437
times; and it sorted every local, parameter and builtin in the body as well,
only for the lookup afterwards to find nothing and drop them. The lookup now
runs first and the sort is over the `(u32, u32)` ranges. Summed −3,518,791
(−1.6255%).

Together, against main: 216,579,492 → 212,958,504, −3,620,995 (−1.6719%).

**The order of `flat` changes, and that was the thing to check.** Ranges come
out in `by_name`'s iteration order where the old sort put members in name
order, so the depth-first walk that reads `flat` visits a declaration's callees
differently and the fixpoint reaches its least fixed point by another route.
kanso#1338's entry recorded the hazard: when a fixpoint's visit order moves, a
measured delta sizes the change rather than bounding it, because some of the
delta may be the new order getting lucky. Two readings say it bounds it here.
`front_end_visits` moved 22,727 → 22,724 — three visits in 22,727, 0.013% — so
essentially none of the 3.5M is the reordering. And `emitted_code` AGREED: the
compiler wrote byte-identical code across the change, so the answers did not
move at all, only the route to them. The full release suite is green at 58 test
binaries and 0 failures.

**A correction to this session's own attribution.** The lead came from reading
`Name == str` comparisons under `eval_expr` in a callgrind profile as the
`ident_set` scan. They are not. One of the two identical walks is worth 22,534
on the module row, not the ~244,000 that reading projected. The real memcmp
attribution on the module corpus, 1,632,475 total or 3.24%: 234,504 in
`check_merged_after_aliases`, 232,600 direct under `eval_expr`, 187,811 in
`insertion_sort_shift_left` under `infer::infer` — which is the sort this entry
is about, and the only one of the three that got paid down. The other two
stand.

**One shape built, measured and declined.** Asking `by_name` from inside
`gather`, so the names are never collected and the `Vec<&str>` disappears
entirely, measured 213,532,569 summed — 574,065 instructions WORSE than keeping
the two buffers. It is the same number of lookups either way; threading the
table and the buffer down through the recursion costs more than the one
allocation it saves. Reverted, and the reason is written beside the buffer it
would have removed. The revert re-measured byte-identical to the reading before
it, which is one more sitting for this harness being deterministic.

The five host-keyed veins — `machine_code`, `compile_allocs` and the three
instruction rows — refuse to compare on this container, so CI measures them.


Both figures are read against 0b828f66. kanso#1374 landed on main while this
branch was in flight and takes the same corpora down by 5,407,312 on CI's
reading, so the two paydowns do not stack arithmetically — they touch
`src/check.rs` and `src/infer.rs` and neither calls the other, but the summed
total this entry quotes is the older baseline. The landed rows are CI's.

CI's sitting, on top of kanso#1374: compile_allocs 29,338 -> 29,341 (+3),
compile_instructions 47,310,638 -> 46,998,377 (-0.6601%), entry_instructions
158,169,260 -> 156,385,625 (-1.1277%), library_instructions 158,447,681 ->
157,092,747 (-0.8552%). Summed compile term -2,095,896 (-1.0200%). Four veins
red in round one and not five: `machine_code` agreed, which it had to -- no
emitter was touched -- and so did `compile_memory`, where the three-visit move
this entry describes sits inside a row the branch had already regenerated.

The -3,620,995 quoted above and the -2,095,896 CI read are both true and they
are not the same measurement. The first is this change against main as it stood
at 0b828f66; the second is it against main with kanso#1374 in. Both branches
cut work out of the whole-program walks, so whichever lands second collects
less. This is the ordinary shape of a queue and not an error in either reading
-- but a delta is a fact about a pair of trees, and quoting one against a base
that has since moved is the mistake to avoid.

## 2026-09-12 — a name is a type only where a cheap test says it could be

Searched the log, the archive and design/ before filing. The filter shape is
kanso#1168's (the bare-name walk's necessary condition) and kanso#1374's; what
is new here is the map it stands in front of, `infer::Ctx::type_names`, which
neither entry touches.

**The ask.** `ident_set` and `eval_call` each ask `type_names` whether the
identifier under them names a declared type. The module corpus asks 4,847 times
and gets yes 618 times; the entry corpus asks 16,546 and gets 2,610. Seven asks
in eight are a SipHash over the name for the answer no.

**Why the cheap reorder is unsound.** Asking `groups` first and reaching
`type_names` only when no function matched would cost nothing at all, and it
changes which programs compile. A type and a function can share a name, and
src/check.rs:1824 already says what happens: a bare name the alias pass
qualified reads as a construction of the imported type of the same name, and
refusing it "rejects a program that compiles". The reorder resolves such a name
the other way. Recorded here so the idea is not re-opened as an obvious win.

**What shipped.** A 256-bit filter, `type_name_slots`, built once from
`type_names`' own keys beside `field_readers`. The slot is first byte, plus
last byte times seven, plus length times thirteen, masked to 255. A clear bit
is proof of absence; a set bit still goes to the map, so no answer changes.
Builder and test both call `tn_slot`, so they cannot disagree about a name.

Rejection measured against the name sets dumped from both corpora rather than
estimated: 3,869 of the module corpus's 4,229 misses (91.5%), 10,877 of the
entry corpus's 13,936 (78.0%). An FNV over the whole name reaches 94.5% and
79.1% and walks the name to do it, which is the walk this test exists to skip.

**Watched red first, at the observable end.** With `may_be_type` wired to
answer no, `tests/golden/mem/record_reuse_shape.kso` goes red on the running
program's allocator counters — `beat_iters` 4,000 -> 0, `survive_slots`
16,006 -> 4 — because an unrecognised constructor stops widening `type_fields`
and the emitted program takes another shape. The mem vein already pins those
counters; nothing new was asserted to make the spec fail.

**Container reading**, `kanso::main` inclusive under callgrind, pinned
tunables, on top of kanso#1374 and kanso#1376:

    module  47,615,866 -> 47,467,069    -148,797  -0.3125%
    entry  158,412,875 -> 158,096,940    -315,935  -0.1994%
    summed 206,028,741 -> 205,564,009    -464,732  -0.2256%

**A correction to the projection that opened the lead.** It was sized at about
1.35M from "roughly a hundred instructions a lookup". The real figure is
464,732, which puts a hashbrown `get` on a short `&str` at about 32
instructions. The hundred was a guess and it was 3x high.

**Env::get, measured and closed.** `Env` is a `Vec<(&str, Set)>` walked
backwards on every name resolution and looked like the other half of this lead.
It is not: 17,055 calls walking 42,930 entries on the module corpus, 56,501
walking 150,032 on the entry corpus — under three entries a call. No
discriminator is worth building in front of that. DONE, not open.


## 2026-09-12 — three checks each rebuilt the same table, in loops identical to the byte

Searched the log, the archive and design/ before filing. `returns` as a
`(name, arity) -> Set` table appears in the 2026-08-25 entry that introduced
`check_wall_operands` and in kanso#1229's arity work; neither notices that the
build is written out more than once.

**What was there.** `check_merged_after_aliases` runs `infer` once and hands
the `Inference` to every check that reads it — that much was already the
arrangement, and a comment in `check_effect_discarded` said so. But three of
those checks did not want the inference. They wanted one table over it: what a
dispatch GROUP answers, keyed by name and arity, which is the union of
`inference.returns[i]` over the declarations sharing a name and a parameter
count. Each built it for itself:

    let mut returns: ... = ...with_capacity_and_hasher(program.fns.len(), ...);
    for (i, d) in program.fns.iter().enumerate() {
        *returns.entry((d.name.as_str(), d.params.len())).or_insert(0)
            |= inference.returns[i];
    }

`check_wall_operands` and `check_discarded_value` hold that text verbatim;
`check_effect_discarded` spells the map `crate::hash::Map` (the same alias) and
fuses the loop with its `discarded` table. A fourth build sits in
`check_none_exhaustive` and keeps its own, because that check runs only when
KANSO_EXHAUSTIVE is set and so is not on the path any of this measures.

**What shipped.** The table is built once beside `inference` and handed round
as `&HashMap<(&str, usize), Set>`. All three checks then stop reading the
inference at all, so the `inference` parameter comes off their signatures too —
which is how you can tell the table, not the inference, was what they wanted.

    module  47,615,866 -> 47,358,386    -257,480  -0.5408%
    entry  158,412,875 -> 157,578,138    -834,737  -0.5270%
    summed 206,028,741 -> 204,936,524  -1,092,217  -0.5301%

**The sizing was 4x low and the reason is in the count.** This was filed as
"two builds, about 273,000", counting the two verbatim ones and pricing them
off an earlier per-declaration figure. There are three live builds, not two,
and the module corpus's 257,480 over three passes of 1,437 declarations is
about 60 instructions a declaration — a hash of the name plus a hashbrown
entry, which is what that costs. The entry corpus falls further because it
merges more declarations, not because the saving is different there.

**No fixture.** The change removes no behaviour and adds none: the table it
builds is the table the three checks built, by the same union in the same
order, and every diagnostic in the 201-fixture error corpus is byte-identical.
There is nothing here that a program could observe and the goldens could not.
Full release suite green: 129 binaries, 0 failures.

## 2026-09-12 — the decidable check joins the one descent, and the rule that kept it out was about the wrong thing

`check_merged_after_aliases` is 34.64% of the entry-corpus compile inclusive
and 4.93% exclusive, and the profile says why: SEVEN separate whole-program
expression walks, each entered once per declaration, visiting the same nodes.
`for_each_child` inclusive under each, on `kanso check entry_corpus/main.kso`:

    named_walk         2,638x   1,791,359
    shapes_walk        2,618x   1,333,580
    literal_walk_expr  2,638x   1,306,073
    field_reads_expr   2,630x   1,087,632
    per_node_walk      2,638x     881,059
    decidable_walk     2,410x     748,969
    BuildScan::expr    2,632x     743,211
                                ---------
                                7,891,883   5.00% of 157,728,439

`check_per_node`'s own doc comment already knew: it prices a bare descent at
1,147,185 over the compile corpus and the entry corpus together, and says every
check that joins stops paying one. Six of the seven are still separate.

**The rule that kept this one out was about the wrong thing.** The same comment
named `check_decidable_failures` as the counter-example that could not join,
because it PRUNES — taking only the condition of an `if` and refusing to look
at the branches, since a guarded branch may be unreachable and refusing it
would refuse a program that runs. That is a reason to stop ASKING at the
branches. It is not a reason to stop WALKING them, and `shapes_walk` beside it
already carried `raised: bool` for exactly that shape. The check joins as
`decidable_failure_at` under a `decidable` flag the walk turns off for an
`if`'s two branches and leaves on for its condition. The rule in the comment is
rewritten to say what it actually excludes: a question about something other
than the node.

**The head, caught by reading rather than by a test.** `for_each_child` hands an
`App` its head first and then its arguments in order, so the fused `if` arm
descends into the head explicitly. Writing only the three arguments would have
silently stopped asking the other three questions about it.

**Watched red, both halves.** Pass `decidable` instead of `false` to the two
branches and `examples/logical_ops.kso` is refused — `error[value]: division by
zero` at `2 < 1 and 1 / 0 < 9`, a program that runs; restored, it prints again.
And nothing pinned the other half: no fixture in the 203-case error corpus held
a literal `1 / 0`, guarded or not, so the refusal itself was unpinned. That gap
closes here with
`tests/golden/errors/a_decidable_failure_outside_a_guard.kso`, which goes red
the moment `decidable_failure_at` leaves the walk.

**The whole error corpus is byte-identical across the change.** The refusal's
diagnostics move in push order — out of position 15 of the sequence and into
the walk's block, which `rotate_left(walked)` sends to the back — and not one
fixture moves, because none carries a decidable failure beside another
diagnostic. `diag::render` does not sort, so this was worth checking rather
than assuming.

**Container reading**, `kanso::main` inclusive under callgrind, pinned
tunables, against main at 025c703f:

    module  47,467,069 -> 47,317,375   -149,694  -0.3154%
    entry  158,096,940 -> 157,558,081   -538,859  -0.3408%
    summed 205,564,009 -> 204,875,456   -688,553  -0.3349%

Both readings were taken at 025c703f; main gained the `(name, arity)` table
fold (kanso#1379, dd465f26) while this branch was in flight, and the branch
carries that merge. The two changes touch different things — that one removed
three rebuilds of a table, this one removes a traversal — so the pair should be
close to additive, and CI's rows are what say whether they were.

**CI's rows**, on the post-#1379 base, are the ones the goldens carry:

    module   46,504,130 ->  46,347,735   -156,395  -0.3363%
    entry   154,931,615 -> 154,371,750   -559,865  -0.3614%
    library 155,663,482 -> 155,103,784   -559,698  -0.3596%
    summed  201,435,745 -> 200,719,485   -716,260  -0.3556%

The container projected -688,553 summed and CI reads 1.04x that, the closest
the two hosts have agreed on a compile delta since these rows were minted —
#1379's round held the previous record at 2.9%. The entry row takes 78.2% of
the summed fall against #1379's 76.5%, which is the shape of a saving keyed to
nodes rather than to declarations. The entry and library rows part by 0.0018
percentage points, closer than any round before them: both routes walk the same
bodies and a per-node saving gives the two entrances nothing to differ over.
Welfare 67.69189 -> 67.69834, banked.

**The descent figure is a ceiling and this realises 60% of it.** 688,553 of
1,147,185 on the container, both measured here; CI's summed fall is 716,260
against a ceiling nobody has re-measured on that host. What comes off is the traversal: the per-declaration re-entry, the
statement loop, the child enumeration. What stays is the per-node question
work, which is the same work asked from a different place, plus the flag and
the `if` test the fused walk now carries at every node. A session sizing the
remaining six walks from the 1,147,185 alone will be about 40% high.

**A stale count corrected on the way.** `tests/golden.rs` said TWENTY-THREE
fixtures gain the loader's ` (module …)` suffix and carry a second golden.
There were 41. The count is removed rather than re-pinned: nothing reads it,
and `ls tests/golden/errors/*.imported.stderr | wc -l` answers it truthfully.

**OPEN: six walks left, and they are not all this cheap.** `decidable_walk`
was the one whose visitor took `(expr, diags)` and nothing else. The other six
carry tables — `field_reads_expr` a scan, a local map and an `Open`,
`literal_walk_expr` four tables, `named_walk` a `Named` and a shadowing
vector — so joining them means the fused walk carries those pointers through
every node, which is the cost the `check_per_node` comment already warns about
for a single extra vector. Each is worth roughly what this one was worth and
each needs its own measurement.

## 2026-09-12 — a dispatch group's catch mask is the same answer every visit

Searched the log, the archive and design/ before filing: `pattern_catches`
appears in the 2026-08-19 entry that introduced the pass-through rule and in
kanso#1229's arity work, and neither asks how often the fold over it runs.

**The fold.** `eval_call`, for every call to a declared group, walks the
group's arms once per ARGUMENT POSITION and ORs `pattern_catches` over the
pattern at that position:

    let caught = ctx.group_members[start..end].iter().fold(0, |acc, &i| {
        acc | ctx.program.fns[i].params.get(pos).map_or(0, pattern_catches)
    });

The result depends on the declarations and on nothing the fixpoint changes.
`program.fns` is fixed before inference starts and `pattern_catches` is a pure
function of one pattern, so this is the same mask every time — once per
argument of every call, on every round of the fixpoint. Callgrind put
`Iter::fold` under `eval_expr` at 716,879 instructions on the module corpus,
1.49% of the compile, and it is nearly all this.

**What shipped.** One `Vec<Set>` built beside `group_members`, a row per group
and a column per parameter position. A `groups` value grows a third word for
the row's start, which is the only reason the other three read sites changed at
all (they take `..` or `_`). The fold becomes an index.

    module  47,467,069 -> 46,797,142    -669,927  -1.4114%
    entry  158,096,940 -> 156,221,601  -1,875,339  -1.1862%
    summed 205,564,009 -> 203,018,743  -2,545,266  -1.2381%

Measured on top of kanso#1378. The fall is 3.6x the profile's attribution of
the fold itself, which is the shape to expect: the profile names the `fold`
symbol, and removing it also removes the slice bounds work, the `params.get`
per arm, and the call into `pattern_catches` that the inclusive figure counts
under its own name.

**CI's rows**, measured twice on two different bases, which is how the pair
turned out to be additive:

              on dd465f26 (pre-#1382)   on 60e01bf8 (post-#1382)
    module          -585,100                  -584,289  -1.2607%
    entry         -2,073,436                -2,072,471  -1.3425%
    library       -2,080,469                -2,079,921  -1.3410%
    summed        -2,658,536                -2,656,760  -1.3236%

    compile_allocs    +8                        +8      29,327 -> 29,335

The landed rows are the second column: 45,763,446, 152,299,279 and
153,023,863 against the container's projected -2,545,266 summed.

**THE TWO CHANGES ARE ADDITIVE, AND THAT IS MEASURED RATHER THAN ASSUMED.**
kanso#1382's fold landed on `check_merged_after_aliases` between the two
readings, so this branch was re-based and re-read. The summed delta moved
2,658,536 -> 2,656,760: a difference of 1,776 instructions, 0.067% of the
delta itself. Both changes touch the same function and could have interacted;
they do not, because the fold removes a traversal of the expression tree and
the table removes a recomputation inside `eval_call`, and the two share no
work. One pair measured twice is not a rule, and the next pair on this
function is owed its own re-reading.

The entry row carries 78.0% of the summed fall on both bases, and the entry
and library rows part by 0.0015 percentage points, the closest they have run.
Welfare 67.69834 -> 67.72201, banked.

**No fixture.** The mask the table holds is the mask the fold computed, over
the same arms in the same order, and `pattern_catches` reads no state. Every
diagnostic in the 201-fixture error corpus is byte-identical, and the emitted
code with it. There is nothing here a program could observe.

## 2026-09-12 — the shapes walk joins the one descent too, and two tables ride with it

`check_merged_after_aliases` runs seven whole-program expression walks over the
same nodes. kanso#1382 folded the first. This is the second.

`check_shapes_per_node`'s driver was `check_per_node`'s loop written a second
time — iterate `program.fns`, skip synthetic, take each statement's expression,
descend — so `err_as_value_at` and `call_shaped_at` ride the descent that was
already happening now.

**CI's rows**, on the post-catch-mask base the goldens carry:

    module   45,763,446 ->  45,522,524   -240,922  -0.5265%
    entry   152,299,279 -> 151,534,916   -764,363  -0.5019%
    library 153,023,863 -> 152,261,219   -762,644  -0.4984%
    summed  198,062,725 -> 197,057,440 -1,005,285  -0.5076%

Welfare 67.72201 -> 67.73111, banked. `compile_allocs` and
`compile_peak_bytes` are byte-identical: the fold moves where work happens and
allocates nothing new.

**This fold and the catch mask are additive, and that is a fact about a pair
rather than about folding a walk.** kanso#1381 landed underneath while this
was in flight, so the same diff has been measured twice. Against the
pre-catch-mask base CI read -238,561 on the module row and -997,728 summed; on
top of it, -240,922 and -1,005,285. The summed figures part by 7,557
instructions, 0.76%.

kanso#1381's body records the same result against kanso#1382's fold, to
0.067%, and says in terms not to read it as a rule. It is right not to. The
THIRD pair, the catch mask against the LITERAL walk's fold, loses 441,481
instructions — 16% of that fold — when the two are stacked. Three pairs, two
additive and one not. Every one was measured on the base it lands on, and that
is the only reason any of it is known.

**The two tables cost less than the comment feared.** `check_per_node`'s doc
comment warns that a joining check makes the fused walk carry its state through
every node, and names a single extra vector as the cost to weigh. This walk
carries two: the arity map, built once over the whole program, and the bound
set, cleared and refilled per declaration. The row still falls 1.53x last
round's 156,395, and the summed fall is 1.39x its 716,260. The warning is about
a real cost and this is a bound on it.

**Two flags, both load-bearing, both watched red.** `decidable` is off inside
an `if`'s two branches, which is #1382's rule. `raised` is off for the head of
a call spelled `err`, because that head is the raise itself. Passing `true`
there instead refuses `err reason` inside `std/text` with the diagnostic that
exists to refuse a bare `err` — a valid program rejected, watched and restored.

**The error corpus is byte-identical.** The shapes diagnostics move: they were
the last block pushed before `diags.rotate_left(walked)` sent the walk's block
to the back, so they sat just before it, and folded they sit inside it. Not one
of the 204 fixtures moves, because none carries a shapes diagnostic beside
another. `diag::render` does not sort, so this was worth checking rather than
assuming.

**The container's offset went the other way this round.** It projected -273,196
on the module row and CI reads 0.88 of that; #1382's round it read 1.04x. The
two hosts do not agree to a fixed ratio, so a compile delta is projected from
CI or it takes the red round.

**OPEN: the third walk does not fold this way, and the reason is the gate.**
`named_walk` is the largest remaining at 1,791,359 by the census, and it is not
a straight move. `arity_at` pushes diagnostics of kind `arity`, and
`check_merged_after_aliases` gates on exactly that kind immediately after
`check_per_node` returns: any `arity` diagnostic makes it drop everything else
and return. Folded in, those diagnostics arrive in front of that gate where
today they arrive well after it, and two things change. A program with a
wrong-arity call to a declared group would take the early return and lose every
other diagnostic it reports today. And `named_walk`'s driver drains the
suppressed ones AFTER the whole declaration's walk — a call whose head name is
locally bound is not that group's call — so the gate would fire on a diagnostic
that was going to be withdrawn, refusing a valid program. That second one is
the direction that matters.

The fixture for it needs two files: a binding or a parameter that shadows a
declaration in the SAME module is refused outright (``error[name]: `pair` is
already a declaration; rename the binding``), so the suppression only ever
fires for a name imported from another module. It belongs with the branch that
tries the fold.

**What the rest of the lead is worth, measured rather than projected.**
Ablating the four remaining whole-program walks outright — `named_walk`,
`literal_walk_expr`, `field_reads_expr` and `BuildScan::expr`, each returning
at the top of its visitor — on the container:

    module   46,791,809 ->  44,530,831  -2,260,978  -4.8320%
    entry   155,870,089 -> 148,719,020  -7,151,069  -4.5878%
    summed  202,661,898 -> 193,249,851  -9,412,047  -4.6442%

That is the whole cost, traversal and per-node question work together, so it is
a ceiling on what folding could reach rather than a target. The entry side is
1.45x the census's 4,928,275 for the same four, because the census counted
`for_each_child` inclusive under each and the predicates running outside the
child enumeration are not in that figure. Every one of those four checks
refuses something a program can do wrong, and the suite is red with them gone.

**Re-measured on main with the four folds underneath it, and the welfare fall
did not move.** kanso#1381's catch mask and the kanso#1382/#1383/#1384 folds
all take instructions off the same compile term this rule adds to, so the
question was whether the fall the floor decision is about survives a base that
much lower. CI's rows on the merged tree:

    module   45,522,524 ->  46,059,799    +537,275  +1.1802%
    entry   151,534,916 -> 153,540,262  +2,005,346  +1.3233%
    library 152,261,219 -> 154,267,894  +2,006,675  +1.3179%
    summed  197,057,440 -> 199,600,061  +2,542,621  +1.2903%

`compile_allocs` 29,335 -> 29,359, a rise of 24. Against the pre-fold base the
same diff rose 1.9626% summed; it rises 1.2903% now. The RELATIVE cost fell by
a third and welfare still reads 0.02 below the floor — 67.71 against 67.73,
where before the merge it read 67.56 against 67.59. The merge moved the score
+0.15 and the floor +0.14.

That is the objective behaving as written rather than a surprise. The compile
term is `r / (r + satiation)` with `r` the baseline over the current reading,
so a fold that lowers the current reading raises the score, and the ratchet
raises the floor to hold it. What the rule costs is a SHARE of that term, and a
share does not shrink because the denominator did. No amount of paydown
underneath this branch dissolves the decision; only a ruling on the weights, or
dropping the rule, does.

Worth having tried: the alternative was to leave a stale 1.9626% standing as
the number the decision rested on.
## 2026-09-12 — the literal-argument check joins the one descent, and stacking these folds is not additive

`check_merged_after_aliases` runs seven whole-program expression walks over the
same nodes. #1382 folded the first, #1383 the second. This is the third that
can move: `literal_walk_expr`, which asks of every call whether a literal
argument sits where the declared group takes something else. Its driver was
`check_per_node`'s loop written a third time — iterate `program.fns`, skip
synthetic, take each statement's expression, descend — and the two are one loop
now.

The tables the check needs ride in the same `PerNode` struct the last fold
introduced. It grew two fields: `groups`, the literal-group table, and `types`.
Both are built once over the whole program, as they were before.

**The measurement, on the container against #1383's head:**

    module   45,488,806 ->  45,174,081    -314,725
    entry   151,905,787 -> 150,841,086  -1,064,701
    summed  197,394,593 -> 196,015,167  -1,379,426  -0.6988%

**Stacking is not additive, and this is the counter-example #1381's body asked
for.** Three pairs were measured on this box:

    catch mask + decidable fold  (#1381+#1382)   additive to 0.067%
    catch mask + shapes fold     (#1381+#1383)   additive to 0.76% (7,557)
    catch mask + literal fold    (#1381+#1384)   -441,481 lost, 16% of the fold

Read on the pre-catch-mask base this fold was worth -2,767,937. Rebased onto
#1381 it is worth -2,326,456. The catch mask decides a dispatch group's mask
once instead of per call, and `literal_argument_at` asks about the same calls;
the work each removes overlaps, so the second one to land collects less. The
first two pairs happen not to overlap and say nothing about the third. A fold's
value is therefore a property of the base it lands on, and projecting one from
another branch's reading is wrong by up to a sixth.

**The census is not a reliable estimator either.** The shapes fold realised 65%
of its census on this container; the literal fold realised 128% on the
post-#1381 base and 161% on the pre-#1381 one. What the census misses is the
driver — the per-declaration re-entry, the statement loop, and a
`for_each_param_name` taking a `&mut dyn FnMut` per parameter over 1,437
declarations. The census counts `for_each_child` inclusive under a visitor and
cannot see the loop that calls it.

**The arity gate is not disturbed.** `check_merged_after_aliases` drops every
other diagnostic and returns as soon as `check_per_node` has pushed one of kind
`arity`. `literal_argument_at` pushes kind `type`, so it arrives inside the
walk's block without reaching that gate — which is exactly what keeps
`named_walk` out, and why this one goes in.

Error corpus 204 fixtures byte-identical, golden suite 11/11, clippy and fmt
clean. Round one is deliberately red on the six host-keyed compile veins; this
container refuses all of them and CI is the host of record.

**Round two — CI's rows, and the container under-read this one by half.**

    module   45,522,524 ->  44,888,539    -633,985  -1.3927%
    entry   151,534,916 -> 149,925,203  -1,609,713  -1.0623%
    library 152,261,219 -> 150,211,345  -2,049,874  -1.3463%
    summed  197,057,440 -> 194,813,742  -2,243,698  -1.1386%

`compile_allocs` 29,335 -> 29,323, a fall of 12: the old driver built a bound
set per declaration to decide which head names were locally shadowed, and the
fused walk reads the one `check_per_node` already keeps. `compile_memory` is
byte-identical. Welfare 67.73 -> 67.75, banked in the same commit; seven
`data-golden` spans on compiler.html regenerated, all three page gates green.

The container projected -314,725 on the module row and CI reads 633,985 — a
ratio of 2.01, where the two rounds before it read 1.04 and 0.88. Three points
spanning 0.88 to 2.01 say the offset between the two hosts is not a ratio to
correct for; it is noise the size of the effect being measured. The previous
entry's "the two hosts do not agree to a fixed ratio" was right and too mild.

The entry row takes 71.7% of the summed fall against 76.0%, 76.1% and 78.0%
for the two folds before it. And this row and the library row part by 0.2840
percentage points, against 0.0035 and 0.0015 — the widest a fold has split
them. Both readings say the same thing: the entry and library corpora hold
different mixes of call sites in bodies, and this check keys on exactly those,
where the earlier folds' savings were keyed to nodes and could not see the
difference.

CI's rows, and what each landed on. The tables above are container readings
taken while the pass was being paid down; these are the numbers the goldens
carry and the ones the decision rests on. They are RE-MEASURED against a main
that now carries the seven folds and the arity hash (kanso#1379, #1381-#1386,
#1387) -- an earlier version of this paragraph priced the rule against the
pre-fold main and every figure in it was stale by about a per cent:

```
compile_instructions   44,234,005 ->  44,767,714    +533,709  +1.2065%
entry_instructions    147,756,205 -> 149,755,117  +1,998,912  +1.3529%
library_instructions  148,091,856 -> 150,092,130  +2,000,274  +1.3507%
compile_allocs             29,314 ->      29,338         +24  +0.0819%
```

The three instruction rows move together because the check runs on every
route, and `compile_peak_bytes` is byte-identical at 774,660: the pass reads
tables it builds once and allocates twenty-four blocks doing it. Summed over
the module and entry corpora the compile term rises 191,990,210 -> 194,522,831,
a rise of 2,532,621 (+1.3191%), which is the whole of the welfare fall.
Nothing else the objective weighs moved -- every runtime vein is
byte-identical, and the cost-goldens job's own summary named exactly these
four veins and no others.

The folds did not shrink the rule's share, and that is the arithmetic worth
keeping. The compile term is a RATIO, so a rule whose cost is a share of it
pays the same share whatever the denominator: measured against the pre-fold
main the rise was +1.3132%, and against a main 1.3% cheaper it is +1.3191%.
Paying down the denominator was worth doing on its own and was never going to
buy this.

**The floor moves, and it is not a decision.** Clay, 2026-09-13, verbatim:
"you don't need to ask my permission to lower the welfare floor if it is in
service of making the language actually work for the specification. this is an
ironclad rule." welfare reads 67.7540 against a floor of 67.77800065192253, so
the floor goes to 67.7540 and `bench/welfare_floor.json` carries the reason.
`--set` refuses to lower and says so; the hand edit is the path it names, and
the same clause covered kanso#1355, kanso#1356 and kanso#1359. This entry and
CLAUDE.md's welfare section both now say that the rule governs a ruled feature
and the ordinary fall rule still governs everything the specification did not
buy.

CI's rows, and what each landed on. The container tables above were taken
while the rule was being paid down; these are the numbers the goldens carry:

```
compile_instructions   44,888,539 ->  45,642,466    +753,927  +1.6795%
entry_instructions    149,925,203 -> 151,799,014  +1,873,811  +1.2498%
library_instructions  150,211,345 -> 152,595,931  +2,384,586  +1.5875%
compile_allocs             29,323 ->      29,458        +135  +0.4604%
```

The module row rises hardest of the three because the rule's per-call question
is asked once per argument and the module corpus is the denser of the two in
call sites; the entry and library rows part by 0.34 points for the same
reason. `compile_allocs` gains 135 blocks, which is the shadow table: one
`Vec` per group with a parameter an earlier arm already names. Summed over the
module and entry corpora the compile term rises 2,627,738 (+1.3489%), and that
is the whole of the welfare fall — `front_end_visits` FELL 22,724 -> 22,449
because the narrowing re-dirties fewer declarations, and every runtime counter
is byte-identical.

## 2026-09-12 — the field-read check joins the one descent, and the module row lands on a number it has produced before

`check_merged_after_aliases` ran seven whole-program expression walks. kanso#1382
folded the first, kanso#1383 the second, kanso#1384 the third. This is the
fourth, and the largest of them on this host.

`field_reads` did two different jobs in one function. Per NODE it asks whether a
dot-read reaches a field, and whether the record it reaches is certain — that
half is a question `check_per_node` was already at the node to ask. Per
STATEMENT it does ordered bookkeeping: a bind closes the runs its pattern binds
and calls `judge_cooccurrence`, a set notes a read. Ordered work cannot ride a
descent that visits children in whatever order `for_each_child` hands them over,
so the two halves are now two functions. `field_read_at` joins the fused walk.
`field_reads_after` stays a statement loop and is called from the fused walk's
`Expr::Build` arm, where the statements it needs are.

The fused walk carries a third flag with the two it already had. `decidable` is
off inside an `if`'s branches (kanso#1382). `raised` is off for the head of a
call spelled `err` (kanso#1383). `certain` is off wherever the record a read
reaches is not settled: inside a lambda, inside either arm of an `if`, in a
guard's early and rest, and to the right of an `and` or an `or`. Three flags,
three shapes that turn them off, and no shape turns off more than one.

Measured against merged main. Container, `kanso::main` inclusive under callgrind, pinned tunables, read twice
on the same box path with two builds identical to the instruction:

```
module   45,488,806 ->  45,174,081    -314,725  -0.6919%
entry   151,905,787 -> 150,841,086  -1,064,701  -0.7009%
summed  197,394,593 -> 196,015,167  -1,379,426  -0.6988%
```

The module row's fall is 314,725, which is the figure the previous entry
records as this container's projection for the literal fold on the same row.
Two different changes, one host, the same count. The candidate reason is that
the module corpus is small enough for the removed DRIVER — the outer loop over
declarations and the recursion setup — to dominate a fold's saving there, where
the predicates the two folds move differ; the entry rows, 1,064,701 against
1,609,713, would then be where the predicates show. That is a hypothesis and
nothing here tests it. Both readings are exact counts and both repeated.

Six mutations against `tests/golden/errors/uncertain_reads_are_not_counted.kso`,
whose six legal declarations each read a field somewhere the record is not
settled and whose seventh, `both_certain`, is refused. Five of the six turn the
fixture red. The sixth cannot: the `Expr::BinOp` arm for `and` and `or` is
unreachable, because `parse_and` and `parse_or` build `logical_if(...)` — an
`Expr::App` with head `if` — so the walk never meets an `and` as a BinOp and the
`if` arm is what clears `certain` to its right. Proved twice: by census over
every `Expr::BinOp` construction site in the parser (`parse_cmp`, `parse_bits`,
`parse_add`, `parse_mul`, and trmc's two, filtered to `+` and `*`), and in
isolation, with a fixture holding only an `and` that stays legal, goes red under
the `if`-arm mutation, and is legal again restored.

The error corpus is byte-identical otherwise. `diag::render` does not sort, and
`diags.rotate_left(walked)` sends the fused walk's block to the back, so a
fixture carrying a field diagnostic beside another would move; none does.

`sh scripts/gates/all_counters.sh`: the twelve runtime cost veins and the lazy
tier agree. `sh scripts/gates/all_compile.sh`: `emitted_code`,
`compile_libraries` and `compile_cost` AGREED; the six host-keyed rows are
refused on this container and are CI's to write. `cargo clippy --release
--all-targets` and `cargo fmt --check` clean, full release suite green after
`scripts/build_wasm.sh`.

`BuildScan::expr` is the fifth and last foldable walk, census 743,211, the
smallest. `named_walk` is the largest remaining at 1,791,359 and stays out:
`arity_at` pushes diagnostics of kind `arity`, and `check_merged_after_aliases`
gates on that kind immediately after `check_per_node`, so folding it would put
those diagnostics in front of their own early return.

CI's rows, written in round two:

```
module   44,888,539 ->  44,564,895    -323,644  -0.7210%
entry   149,925,203 -> 148,827,747  -1,097,456  -0.7320%
library 150,211,345 -> 149,164,244  -1,047,101  -0.6971%
summed  194,813,742 -> 193,392,642  -1,421,100  -0.7295%
```

`compile_allocs` 29,323 -> 29,317, a fall of six; `compile_peak_bytes` and the
front end's rounds and visits byte-identical. Welfare 67.75 -> 67.77, banked in
the same commit; seven `data-golden` spans on compiler.html regenerated.

The container projected -1,379,426 summed and CI reads 1.030 of that, the
closest the two hosts have come across four folds after 1.04, 0.88 and 2.01.
Three of the four are near one and the fourth is not, which is why the previous
entry called the offset noise rather than a ratio. A fourth point near one does
not make it one.

The entry and module rows part by 0.011 percentage points here, where the
literal fold's round parted them by 0.330. That fold keyed on call sites, which
the two corpora hold in different proportions; this one keys on dot-reads,
which they hold in nearly the same one.

## 2026-09-12 — the block-born check joins the one descent, and it was never a per-node question

The fifth and last of the foldable walks in `check_merged_after_aliases`.
kanso#1382 folded the decidable check, kanso#1383 the shapes walk, kanso#1384
the literal-argument check, kanso#1385 the field-read check. `BuildScan` is
gone with this one, and `check_per_node`'s descent does its work.

The shape is different from the four before it, and the difference is the
finding. `BuildScan::expr` held NO per-node predicate at all. Every diagnostic
the check produced came from `wrote_a_field`, called from `body`; `expr`
existed only to locate the statement lists nested inside expressions — a
`build`, an `if` arm, a guard's remainder. So this fold is not a question
joining a descent. It is three arms running ordered statement work at nodes the
fused walk already reaches.

`body` splits in two. `before` is the field write's refusal, and it must be
asked against `born` as it stands at that statement and ahead of the value's
own descent, which is what reading a statement list in order buys. `after` is
the binding's birth and the write's record, both of which read the value the
statement has just walked. `types` leaves the struct entirely: `PerNode`
already carries the same `HashMap<&str, &TypeDecl>`, built once for the whole
program, where `BuildScan` built a second copy of it.

Measured on this container, `kanso::main` inclusive under callgrind with pinned
tunables, against kanso#1385's head read on the same box path — both readings
repeated and identical to the instruction:

```
module   45,174,081 ->  44,898,856    -275,225  -0.6093%
entry   150,841,086 -> 149,962,557    -878,529  -0.5824%
summed  196,015,167 -> 194,861,413  -1,153,754  -0.5886%
```

Ablating `check_build_blocks` outright on merged main — no walk, no tables, no
diagnostics — reads `-1,193,288` summed. The fold recovers 96.7% of that. The
four folds before it realised between 57% and 75% of their own census figures,
and the gap is the same fact that made this one structurally different: where
they left a predicate behind and removed only a traversal, this check WAS a
traversal, so removing the traversal removed nearly all of it.

The error corpus is byte-identical. That is worth a sentence, because the
ordering genuinely moves: `check_build_blocks` used to push before
`diags.rotate_left(walked)` sent the fused walk's block to the back, so its
diagnostics arrived ahead of the walk's; folded, they sit inside that block.
`diag::render` does not sort on this route. No fixture in the corpus carries a
build diagnostic beside another, so none of the 204 moves.

Two of the seven mutations stayed green, for two different reasons. Five of
them turn the corpus red: the build arm opening with nothing
born, its `before`, its `after`, the `if`-arm block arm existing at all, and
the top level asking `before`. Two do not.

Removing `conditional += 1` from the block arm WAS a corpus gap, and this
branch closes it. The counter is live and the arm is reachable —
`tests/golden/errors/a_field_write_inside_an_if_arm.kso` exercises it — but no
fixture distinguished a field whose birth was recorded inside an arm from one
recorded outside, which is the only thing the counter changes.
`a_birth_recorded_inside_an_if_arm.kso` does: both arms answer `outer` so the
`if`'s value is born, but `outer.link` was filled only in the arm that ran
conditionally, so the field is not proved born and the write through it is
refused. Watched both ways — the fixture is red today and the program compiles
with the counter removed.

Removing the guard arm's `after` leaves the corpus green because the arm is
UNREACHABLE, like the `and`/`or` arm kanso#1385 recorded, and the proof is the
parser's.

`Expr::Guard` has exactly one construction site: `parser.rs:866`, inside
`parse_body`. `parse_build_body` never calls `parse_body` — it reads each line
with `parse_stmt` and hands blocks to `parse_block_construct` — so a build
body's own statements are never a guard. That leaves the nested route, and
`parse_body`'s own stray check closes it. The leading run is a maximal prefix
of returns and binds, each carrying the deeper lines beneath it; the check that
follows it,

```rust
if let Some(stray) = body[lead_end..].iter().find(|l| is_return(l)) {
```

scans FLAT. It does not skip deeper indents the way the lead scan does, so a
`return` at any depth below a line that is neither a return nor a bind is
refused with "a `return` sits with the bindings, before the effect chain". A
`build` header is never either of those — `q = build ...` is refused outright
with "`build` answers nothing to bind `q` to" — so the lead run always breaks
at or before a build, and every `return X if C` anywhere inside that build is a
stray.

Four spellings confirm it from the other side: the guard leading a build body,
following a binding in one, following a field write in one, and leading an `if`
arm inside one, each refused at the `return` line itself. The arm stays as
documentation of a shape the walk would otherwise have to think about; it costs
one match arm, the same trade kanso#1385 made for `and`/`or`.

CI's rows, round two:

```
module   44,564,895 ->  44,291,724    -273,171  -0.6130%
entry   148,827,747 -> 147,951,808    -875,939  -0.5886%
library 149,164,244 -> 148,288,999    -875,245  -0.5868%
summed  193,392,642 -> 192,243,532  -1,149,110  -0.5942%
```

`compile_allocs` falls 3, from 29,317 to 29,314 — `BuildScan` loses its `types`
field and is built once per program either way. `compile_peak_bytes` is
byte-identical at 774,660, and the twelve runtime cost veins and the lazy tier
do not move: the fold is in the front end and emits the same code. Welfare
67.77 -> 67.78, banked.

The container projected -1,153,754 summed and CI reads 0.9960 of it, the
closest of the five folds after 1.04, 0.88, 2.01 and 1.028 on the module row.
Four of those five now sit within 3% of one. That does not make the offset a
ratio — one point off by a factor of two is what "noise the size of the effect"
looks like, and the reading kanso#1384 wrote down stands: a compile delta is
projected from CI or it takes the red round.

`named_walk` is the only whole-program walk left in
`check_merged_after_aliases`, and it does not fold this way. `arity_at` pushes
diagnostics of kind `arity`; the function gates on exactly that kind
immediately after `check_per_node` returns, with a `retain` and an early
return. Folding would put those diagnostics in front of their own gate, where a
wrong-arity call to a declared group would take the early return and lose every
other diagnostic the program reports today — and `named_walk`'s driver drains
its suppressed diagnostics after the whole declaration's walk, so the gate
would fire on one that was going to be withdrawn. That second one refuses a
valid program. The fixture for it needs two modules, because a binding
shadowing a declaration in the same module is already refused outright, so it
belongs with the branch that tries the fold rather than with this one.
## 2026-09-13 — the sixth walk is the wrong target, and the measurement says where the money is

Five folds landed in two days: kanso#1382 the decidable check, kanso#1383 the
shapes walk, kanso#1384 the literal-argument check, kanso#1385 the field-read
check, kanso#1386 the block-born check. Each entry closed by naming
`named_walk` as the one left, at a census figure of 1,791,359, and saying the
arity gate puts it out of reach. Before building the way around that gate I
measured what the fold would actually buy. It is the smallest of the six, and
the check's cost is somewhere else.

Four readings on merged main plus the block-born fold, `kanso::main` inclusive
under callgrind with pinned tunables, module and entry corpora summed:

```
full                                194,861,413
tables built, walk skipped          191,131,486
walk runs, three predicates no-op   192,298,663
check_named_per_node gone outright  189,839,799
```

Which decomposes the whole check, 5,021,614 (2.577% of the compile):

```
table construction   1,291,687   25.7%
traversal            1,167,177   23.2%
per-node predicates  2,562,750   51.0%
```

**A fold removes the traversal and nothing else.** 1,167,177 is the ceiling,
and the four folds that left a predicate behind realised 57% to 75% of their
own figures, so the fold is worth roughly 0.67M to 0.88M summed — the smallest
of the six, against a gate whose doc comment records a bug where sharing it
silently skipped every check after. The census's 1,791,359 was the walk's
inclusive cost under `for_each_child`, which carries the predicates it calls;
it was never the fold's figure.

The table slice is hazard-free and it is almost entirely one function.
`Arities::of` reads 1,345,353 inclusive on the entry corpus alone — more than
the summed table figure, because the ablation deltas carry allocator and layout
effects the per-function reading does not.

It is called **18 times** on one entry compile, against one call to
`check_file_shadow`. `check_merged_after_aliases` runs per module, so the
entry corpus's seventeen dependencies plus the entry itself each rebuild every
table the check needs, over their own merged program. 74,742 instructions a
call.

That is the shape to look at next, and it is bigger than the fold: the five
folds each removed a traversal that was running eighteen times, which is why
they paid what they did. What has not been asked is whether the per-module
rebuild is necessary — each module's merged namespace is genuinely different,
so this is not a free deduplication, and kanso#1003 already withdrew one claim
that the per-dependency merged check was redundant. The question here is
narrower: whether the TABLES a module's check needs can be derived from its
dependencies' rather than rebuilt from the merged program.

`Arities::of` also hashes each declaration name up to three times: once to
count, once to read the range back, and once more through `get_mut` when the
arity is new. The last two are a `get` and a `get_mut` of the same key, and the
slot the first returns is the slot the second wanted. Collapsing them:

```
module   44,898,856 ->  44,837,968     -60,888  -0.1356%
entry   149,962,557 -> 149,755,885    -206,672  -0.1378%
summed  194,861,413 -> 194,593,853    -267,560  -0.1373%
```

One hash per declaration, 267,560 instructions — a quarter of what the whole
fold could reach, for six lines and no gate to think about. That is the shape
of the rest of this: the check is re-entered eighteen times, so anything per
declaration is paid eighteen times over.

The fold is not refused, it is ranked: it is the smallest remaining and the
only one carrying a gate hazard, so it goes behind the table work rather than
in front of it. What ships here is the hash collapse; the rest is four
ablations and one call count, and they say where to look next.

CI's rows for the collapse, measured on merged main rather than this
container:

```
module   44,291,724 ->  44,234,005    -57,719  -0.1303%
entry   147,951,808 -> 147,756,205   -195,603  -0.1322%
library 148,288,999 -> 148,091,856   -197,143  -0.1329%
summed  192,243,532 -> 191,990,210   -253,322  -0.1318%
```

The container projected -267,560 summed and CI reads 0.9468 of it. Across six
rounds the ratio now runs 1.04, 0.88, 2.01, 1.028, 0.9960, 0.9468 — five
within 6% of one and a sixth off by a factor of two, which is the reading
those entries have carried all along: a compile delta is projected from CI or
it takes the red round.

Three veins moved and three did not. `machine_code`, `compile_memory` and
`compile_allocs` all agreed, which is what a refactor that changes neither
what is allocated nor how much code is emitted should look like. That is worth
one sentence against `bench/compile_instructions_golden.txt`'s own warning
that this row usually moves on any edit to the compiler's Rust because the
layout shifts under it: here the row moved because the work moved, and the
layout held still enough to leave `.text` byte-identical.

Welfare 67.77569149595541 -> 67.77800065192253, banked in the same commit.

**What CI landed, and what the rule costs.** The rule shipped in kanso#1369 on
merged main. Six veins moved, and every one that got worse is named here with
the value it landed on, because the trend gate reads this file and nothing else:

```
compile_allocs         29,338 ->      29,473      +135  +0.4602%
compile_instructions   44,767,714 -> 45,523,131  +755,417  +1.6874%
entry_instructions    149,755,117 -> 152,087,783 +2,332,666  +1.5576%
library_instructions  150,092,130 -> 152,459,094 +2,366,964  +1.5770%
```

`compile_memory` agreed and the fourteen work rows agreed: the rule costs the
front end and costs the run program nothing, which is what a check should do.

**The emitted and machine-code veins moved too, and that is the library, not the
check.** Making the rule unconditional means the shipped library has to satisfy
it, so `lib/list`, `lib/regexp` and `hako/remote` gained arms. Those arms are
compiled, so `emitted` and `.text` move with them. runbench's emitted line count
goes 34,905 -> 34,773 and its `.text` 247,026 -> 246,866: both FALL, because the
added arms replaced fall-through paths the emitter had been expanding. That
direction was not predicted and is worth recording — the obvious expectation is
that more source means more code.

**The floor drops 67.754 -> 67.7149**, by hand, under Clay's 2026-09-13 ironclad
rule: lowering it in service of the specification is never his call. The
container projected the fall at 0.01 and CI read 0.04, so the projection was
four times light — a reminder that the compile rows are a CI-host measurement
and the container's are not a substitute for them.

## 2026-09-13 — one instruction in ten of the run program is a register save, and the inline threshold is the lever

**SHIPPED.** `release_clang` passes `-mllvm -inline-threshold=1000`, four times
clang's default of 250. CI's rows are in the "What CI read" section at the
bottom; everything before it is this container's, measured against a container
baseline, and the two must not be compared across.

**The finding.** runbench retires 2,232,013,849 instructions, of which the
binary's own code (everything but libc) is 2,185,625,151. `push`, `pop`, `ret`
and `leave` alone are **215,229,225** of that — 9.85% of the binary's code and
9.64% of the whole program. One instruction in ten is saving or restoring a
callee-saved register.

That was measured rather than guessed. `callgrind --dump-instr=yes` gives a
per-address cost, `objdump -d` gives each address its mnemonic, and joining the
two sums the frame instructions exactly. The dump changes nothing it measures:
the instrumented run read 2,232,013,849, the same total to the digit.

**The cost is flat, which is why no previous round found it.** `encode_onto`
spreads its 298,748,819 self instructions over 588 distinct addresses, the
hottest carrying 0.80%; `value_for` 233,828,199 over 491, hottest 1.49%;
`obj_key_start` 151,499,007 over 217, hottest 0.52%. There is no loop to
tighten — the 2026-09-02 entry on the byte arm found the same thing about two
functions, and this says it about the whole program. What there is instead is per-call overhead, and the
functions paying most of it are small and hot rather than long:

```
k_map_sorted            49.56% frame     6,517,497 self, 160 addrs
d_json/string_at_4      20.02%          69,083,185 self, 158 addrs
d_json/entry_onto_2'2   20.00%          41,341,950 self,  65 addrs
d_json/esc_byte_2       19.26%          27,717,120 self, 175 addrs
k_b_append_rendered     16.97%          24,604,654 self, 114 addrs
k_map_lit               16.66%          18,048,293 self,  78 addrs
k_b_utf8_slice_raw      16.51%          52,188,939 self, 134 addrs
k_b_at                  14.51%          61,798,244 self, 226 addrs
```

`obj_key_start`'s prologue disassembles as six callee-saved pushes, a 152-byte
frame, and a move of an incoming STACK argument into its own frame — the
function takes more arguments than the ABI has registers, and 827,739 calls
pay for it.

**Inlining is what removes a frame, so the threshold is the lever.** Measured
by hand-linking `runbench.ll` against the same cached runtime object at three
settings, every binary in one directory and run from the repo root so the
exec-path offset cancels:

```
threshold  runbench          .text
250        2,229,257,603     324,110     (clang's default)
1000       2,184,283,786     355,790     −2.0174%, +9.8% text
5000       2,141,173,484     507,998     −3.9513%, +56.7% text
```

1000 ships. 5000 is declined here: twice the win for six times the code.

**The whole benchmark set, container A/B.** Both binary sets copied into one
directory, `base_` and `inln_` the same length so the path costs the same:

```
runbench     2,232,013,849 -> 2,187,040,032   −44,973,817  −2.0149%
encodebench  4,052,767,921 -> 3,976,859,553   −75,908,368  −1.8730%
digestbench     10,497,757 ->     10,316,938      −180,819  −1.7225%
jsonbench    1,436,454,329 -> 1,427,971,381    −8,482,948  −0.5905%
escapebench     85,489,184 ->     85,483,165        −6,019  −0.0070%
readbench        4,631,248 ->      4,631,250            +2  +0.0000%
```

**A CORRECTION, made mid-measurement and worth writing down.** The first pass
compared these container readings against the COMMITTED goldens and read
encodebench as a RISE of 1.1241%. The goldens are CI's host: main's runbench
golden is 2,252,446,969 where this container reads 2,232,013,849, 0.9% apart.
Against a container baseline encodebench falls 1.87%. A container reading and a
golden are not comparable, and the gap is bigger than most of the effects this
log records.

**What it does not cost.** `all_counters.sh` reports the twelve cost veins and
the lazy tier all AGREE: inlining moves no allocation counter, which is what it
should do. What it does cost is `.text`, +9.8%, and machine-code size has no
welfare term — Clay ruled that on 2026-09-05 — though the vein still watches it
exactly.

**Owed in round two.** CI's fourteen work rows, its `machine_code` and
`emitted_code` veins, and `welfare --set` once the goldens carry them. The
eight benchmarks not measured here (oneshot, basket, widebench, deepbench,
pendbench, indexbench, scanbench, livebench) are CI's to report; round one is
deliberately red on `bench/instructions_golden.txt`.


**What CI read, three times, against three different bases.** The first sitting
was taken against main at 5982c60a, before kanso#1372 landed the effect type;
the second against 12e73890 after it; the third against 047efca9 after
kanso#1369 landed the exhaustiveness rule and changed `lib/list`, `lib/regexp`
and `hako/remote`. Only the third is in the goldens. The three were never
composed by arithmetic — a different library changes what inlines, and deltas
measured on different trees do not add — which is why each base change cost a
full re-measure rather than a subtraction.

```
                  vs 5982c60a      vs 12e73890      vs 047efca9 (landed)
runbench          −63,128,569      −63,128,542      −57,470,167
percentage         −2.8027%         −2.8027%         −2.5515%
widebench            +32,011          +32,011          +16,019
readbench                 +2               +2               +2
.text total          +6.2209%         +6.2209%         +7.3629%
```

The first two agree to five significant figures and the third does not, and
that is the finding rather than an inconvenience. kanso#1372 moved the library
without changing what the linker could inline; kanso#1369 added arms to three
shipped modules, and those moved it. Nine per cent of the win went with them.
A threshold's effect is a property of the program it is applied to, and the two
sittings that agreed were the coincidence.

runbench **2,252,446,915 -> 2,194,976,748**, a fall of 57,470,167 (−2.5515%).
The full third sitting, twelve of fourteen work rows falling:

```
jsonbench    1,468,801,090 -> 1,449,421,842   −19,379,248  −1.3194%
encodebench  3,958,779,263 -> 3,882,689,256   −76,090,007  −1.9221%
oneshot         21,616,888 ->     21,164,390      −452,498  −2.0933%
basket          34,698,668 ->     34,693,472        −5,196  −0.0150%
widebench       35,202,913 ->     35,218,932       +16,019  +0.0455%
deepbench      387,474,235 ->    378,118,216    −9,356,019  −2.4146%
escapebench     85,558,078 ->     85,537,054       −21,024  −0.0246%
pendbench      221,912,236 ->    221,101,809      −810,427  −0.3652%
indexbench       3,265,819 ->      3,265,392          −427  −0.0131%
scanbench      587,488,450 ->    562,456,145   −25,032,305  −4.2609%
digestbench     10,426,549 ->     10,199,161      −227,388  −2.1809%
readbench        4,630,969 ->      4,630,971            +2  +0.0000%
livebench    3,450,423,659 -> 3,320,972,422  −129,451,237  −3.7517%
runbench     2,252,446,915 -> 2,194,976,748   −57,470,167  −2.5515%
```

**The rows that got worse, each named with the value it landed on.** Two work
rows rise: `work_widebench` **35,218,932** (+16,019, +0.0455%) and
`work_readbench` **4,630,971** (+2). The `text` vein rises with them, to
**1,664,396**. The objective weighs the sum and the sum went up, so none of the
three is a decision to defend on its own, but a rise that nobody names is the
thing this log exists to catch. widebench is the larger of the two work rows,
and its cause is the same as its .text rise: a wider inline threshold
specialises more call sites and a few of them were better off shared. It is
half what the second sitting read, which is the base change again.

**A CORRECTION to this entry.** The trend gate refused it for a naming miss. It
wants each worsened counter written with the key its golden uses, and the entry
said `widebench` and `readbench` where the goldens say `work_widebench` and
`work_readbench`, and gave no value at all for `text`. Fixing that turned up a
second fault the gate could not see. Both tables above had kept the sitting
taken against 5982c60a, so escapebench, indexbench, scanbench and runbench read
a few dozen instructions off on each side of the arrow, the .text vein totalled
1,551,324 -> 1,647,884 rather than 1,549,100 -> 1,645,468, and the compile
paragraph below named the four pre-kanso#1372 rows, and the welfare pair read
67.78 -> 67.98 where main's floor is 67.754 and `--score` says 67.9598.
kanso#1372 moved every one of them. Each figure in this entry is now read off
the committed goldens and the committed floor.

Twelve of fourteen .text rows rise, the vein `text` totalling
1,550,252 -> **1,664,396** (+114,144, +7.3629%) — less than the +9.8% projected from runbench.ll alone, because most
benchmarks link less of the library than the run program does:

```
jsonbench     99,874 -> 106,530  +6.66%      escapebench  57,490 ->  57,298  −0.33%
encodebench  122,322 -> 129,778  +6.10%      pendbench    92,034 ->  94,786  +2.99%
oneshot      111,746 -> 117,970  +5.57%      indexbench   61,730 ->  61,170  −0.91%
basket       114,082 -> 118,258  +3.66%      scanbench   158,946 -> 182,402 +14.76%
widebench    126,690 -> 135,682  +7.10%      digestbench 111,362 -> 114,098  +2.46%
deepbench     76,354 ->  82,402  +7.92%      readbench    58,434 ->  58,434   0.00%
                                             livebench   112,322 -> 119,554  +6.44%
                                             runbench    246,866 -> 286,034 +15.87%
```

Machine-code size has no welfare term — Clay ruled that on 2026-09-05 — so
nothing here scores. The vein is exact all the same, which is the point: the
growth is watched even though it is not paid for.

**One prediction in the pull request was wrong, and CI corrected it.** Round one
was opened expecting `emitted` to go red alongside `machine code` and `work`.
It did not: `emitted` came back SUCCESS, and the emitted golden is byte-identical
across the change. It counts what the COMPILER wrote, and the inline threshold is
read by clang at link time, long after the compiler has finished writing. The
three veins that can see a linker flag are work, machine code, and nothing else.

**The compile side is untouched and that is not a coincidence.** compile_allocs
29,473, compile_instructions 45,523,131, entry_instructions 152,087,783,
library_instructions 152,459,094 — all four AGREED with their goldens. The flag
is on `release_clang`, which links benchmark binaries; `kanso check` never
reaches it.

**Welfare 67.7149 -> 67.9019**, banked with `--set` in this same commit. The gain is
run_instructions', which satiates late (2.0) and carries the objective's whole
run-speed term.

**What is left of the frame cost, and why it is not the next change.** Re-profiled
under the new threshold, the frame is 192,992,914 — 8.82% of the program, down
from 9.64%, so the threshold took about 22M of frame directly and 45M in total:
inlining's second-order optimisations are roughly half the win. `k_map_sorted`
and `entry_onto` have left the profile entirely.

**Where it actually sits, measured rather than guessed.** An earlier draft of
this paragraph said the residual sits in the `tailcc` mutual tail cycle. A
per-instruction profile of the linked binary, with every address mapped to its
opcode through `objdump`, says otherwise, and the frame total reproduces to
2,791 instructions (192,995,705 against 192,992,914) so the two readings are of
the same thing. Two of the cycle's four members — `str_chars_3` and
`string_scan_3` — are not in the profile at all: the wider threshold inlined
them away. Of the two that survive, `string_at_4` carries 13,832,389 frame
instructions, 7.2% of the program's 193M, and it is the THIRD owner rather than
the first. The frame is spread:

```
d_json/encode_onto_2   30,952,350   8.3% of its own 372,343,104
d_json/value_for_3     22,114,451   9.0% of its own 244,520,545
d_json/string_at_4     13,832,389  17.1% of its own  80,843,551
d_json/obj_key_start_4 10,760,607   6.8% of its own 158,344,918
k_b_at                  8,970,000  14.5% of its own  61,798,244
k_b_utf8_slice_raw      8,614,980  16.5% of its own  52,188,939
```

The four big decode and encode drivers spend 6–9% of their own instructions on
the frame, which is what a large function that spills its callee-saved
registers costs. The interesting column is the second one: five small hot
functions — `string_at_4`, `k_b_utf8_slice_raw`, `k_b_append_rendered` (17.0%),
`number_done_4` (15.9%) and `k_b_at` — each spend about one instruction in six
on prologue and epilogue, and threshold 1000 did not inline any of them. Those
five hold 39.4M of frame between them. Whether a targeted `alwaysinline` on
that shortlist buys part of threshold 5000's extra 1.93% without its 56% of
code is an open lead, and it is a different question from the cycle.

What IS settled about the cycle is that the obvious lever does
not reach it: `preserve_none` cannot be swapped in for `tailcc` here.
src/codegen.rs records that a `musttail` call may cross an arity or a type only
under `tailcc`, and this cycle does exactly that — a
`(KValue, i64, i64, i64) -> %parsed` function musttails into a
`(KValue, i64, KValue)` one. Dropping `tailcc` for the cycle is therefore not a
tuning question but a correctness one, and the same comment records why the
convention is narrowed rather than universal: a non-tail `call tailcc` whose
arguments do not all fit in registers is miscompiled on arm64. Call instructions are only 25,174,058 against
193M of frame, which is 7.7 frame instructions per call: these are callee-saved
register spills, not frame-pointer setup, and `-O3` already omits the frame
pointer.

**A THIRD CORRECTION, and a third sitting.** kanso#1369 landed on main while
this branch sat in CI, and it changed `lib/list`, `lib/regexp` and
`hako/remote` — three files the run program links. Both tables above now carry
the third sitting, measured by CI against 047efca9; the second sitting's
numbers survive only in the three-column comparison, where they are the point.
Composing the old delta onto the new base by arithmetic would have been the
error this entry already recorded once, and it would have been wrong by
5,658,375 instructions on runbench alone.

The twelve allocation veins and the lazy tier were re-read on the merged tree
and every one agrees byte for byte. Neither an exhaustiveness rule nor a linker
flag can move a counter that counts allocator calls, so that is the expected
answer; it is written down because a sweep that is run and not reported is a
sweep nobody can check.

## 2026-09-13 (second) — an inline decision's value belongs to the whole optimisation state, so no rule over functions can find it

**MEASURED AND DECLINED, and the decline is the finding.** kanso#1389 left an
open lead: one instruction in ten of the run program is a register save, and
the threshold took only part of it. The obvious next step is to inline
particular functions rather than raise a global knob. Fifty-one link-and-count
measurements later the answer is that a per-function rule cannot exist, and the
reason is not that the right predicate was not found.

**The harness.** `runbench.ll` off merged main, one `alwaysinline` added by sed
to one `define tailcc`, relinked with the shipped recipe —
`clang -O3 -flto -mllvm -inline-threshold=1000 -mssse3` against the cached
runtime object — and counted under callgrind. The control relink is
BYTE-IDENTICAL to the shipped `runbench`, so this is the real link and not an
approximation of it. Base 2,187,039,574 instructions, `.text` 278,850.

**Thirty-two functions one at a time: seven win, four lose, twenty-one are
exact no-ops.** Nothing about a function predicts which. Not its IR instruction
count, not its block count, not its call-site count, not whether every call is
a `musttail` — all seven winners are, and three of the no-ops are not — and not
the frame fraction that produced the original shortlist. `array_delim_4` is
worth 11,083,644 alone and the frame profile never flagged it, 4.7x either
function the profile did pick out.

**The union is not the sum.** Five of the seven, the ones whose own `.text`
fell, are worth 24,556,754 together: 3,725,371 MORE than their single-function
sum. Adding the other two costs 4,305,504 instructions and 1,840 bytes, and
`value_for_3` — the largest single win in the whole sweep at 16,541,819 — is
the single worst thing to add to the set.

**Two greedy rounds found three more improvers, and every one had measured
nothing alone.** `string_scan_3` was an exact zero and is worth 1,037,025.
`str_low_5` the same, worth 175,527. `array_open_3` buys 48 bytes for no
instructions, measured twice. `str_chars_3` read +2 on the five — noise — and
is worth 175,525 with 688 bytes on the eight. The best set is nine functions at
2,161,094,743 and `.text` 274,802: −25,944,831 (−1.1863%) with 4,048 fewer
bytes, both axes moving the right way.

`str_chars_3` and `str_surrogate_4` are a useful pair. Both move `.text` by the
identical −688 and their instruction counts by ±175,5xx. The two read the same
thing, since a surrogate pair is decoded inside the character walk, and
inlining either shrinks the same code while only one of them wins.

**Then the same nine, linked at three thresholds, against the plain file at
each:**

```
threshold   plain            nine             the set is worth
1000        2,187,039,574    2,161,094,743    −25,944,831   .text  −4,048
1250        2,166,321,995    2,156,158,161    −10,163,834   .text  −1,744
2000        2,141,224,406    2,151,254,594    +10,030,188   .text  −7,088
```

At the shipped threshold the nine are worth 26 million instructions. At 2000
they COST ten million. Nothing about the annotations changed; the state they
landed in did. So the value of an inline decision is a property of neither the
function nor the set but of the whole optimisation state, which is why no
predicate over functions could have worked and why the set is worthless as an
artifact: any change to the threshold, the library or the emitter invalidates
it.

**The knob is not monotone either.** The plain file at five thresholds:

```
1000   2,187,039,574   .text 278,850
1250   2,166,321,995   .text 289,458   −0.9473%
1500   2,168,898,667   .text 299,794   −0.8295%
2000   2,141,224,406   .text 315,698   −2.0948%
3000   2,135,106,996   .text 391,842   −2.3746%
```

1500 is WORSE than 1250 on instructions while costing 10,336 more bytes. A
wider threshold admits a different set of inlinings, and a superset of
decisions is not a better program — the same non-composition one level up.

**Nothing ships from the search, and one thing ships from the ladder.** The
emitter has never decided this attribute for a user's function: all thirty-six
`alwaysinline`s in `src/codegen.rs` sit inside hand-written prelude shims, and
no code path writes the attribute onto emitted code. Adding one would need a
predicate that the measurements above rule out. **Threshold 2000 is the live
follow-up**: −2.0948% for +13.2% `.text` is close to the trade kanso#1389
already took (1000 over clang's 250 was −2.0174% for +9.8% on this same file),
machine-code size carries no welfare term under the 2026-09-05 ruling, and it
is one line. What this container cannot answer is what CI reads across all
fourteen benchmarks, which is exactly the lesson kanso#1389's three sittings
taught, so it goes as its own pull request that takes CI's rows.

**And CI answered while this entry was in review, so the figure above is a
projection rather than the result.** kanso#1391 measured runbench
2,194,976,748 -> 2,168,019,757 on CI, a fall of 26,956,991 (−1.2281%), for a
`.text` sum of 1,783,980 against 1,664,396 (+7.1848%). That is 59% of the
−2.0948% this container read, where eight days earlier the same pair of boxes
disagreed the other way round. Every absolute number in this entry is this
container's, which the ladder was always for; the shape of the ladder is what
it establishes and the steps are not CI's.

**A trap reproduced cleanly, worth the paragraph.** Twenty of the twenty-seven
greedy candidates came back at exactly −14 instructions, and twenty
coincidences is not a result. The candidates linked as `bin_00` and the control
was `bin_c`: one character of path.
`scripts/gates/instructions.sh` says the kernel puts the exec path on the new
process's stack for libc to walk before main, and the check is direct —
relinking the control as `bin_zz` gives a byte-identical binary reading
2,162,482,820 against 2,162,482,834. So −14 meant NO CHANGE, and every figure
in this entry is taken at matched name lengths.

## 2026-09-13 (third) — the inline threshold goes to 2000, and the container over-projected the win by 70%

**DONE.** kanso#1389 put `-mllvm -inline-threshold=1000` on `release_clang`,
eight days after the frame cost that motivated it was first measured. This
raises the same knob to 2000.

The value was measured rather than reasoned, because the ladder is not
monotone. On `runbench.ll` in this container, every step read against the 1000
that kanso#1389 shipped:

    1250   -0.9473%
    1500   -0.8295%    worse than 1250, and +10,336 bytes of .text to be worse
    2000   -2.0948%
    3000   -2.3746%

1500 loses to 1250 while costing more code. A ladder with that shape cannot be
walked by choosing a direction and following it, so each rung was linked and
counted. 2000 is where the return per byte flattens: 3000 buys a further
0.2798% for roughly nineteen thousand more bytes.

**CI read 59% of the projection, and the projection has missed in both
directions now.** This container projected runbench 2,187,039,574 ->
2,141,224,420, a fall of 45,815,154 (−2.0948%). CI measured 2,194,976,748 ->
2,168,019,757, a fall of 26,956,991 (−1.2281%). Eight days earlier the same
pair of boxes disagreed the other way: the container read −2.0149% for the
1000 and CI read −2.5515%. Each reading is correct for the box that took it. An inline threshold changes
what every later pass in the pipeline is handed, and those passes are a
different clang build on a different chip in the two places. This vein has no host key,
so the rows here are CI's and the container's numbers are only ever a
projection of the sign.

CI's sitting, against 6c385e2d:

    jsonbench    1,449,421,842 -> 1,413,392,590  -36,029,252  -2.4858%
    scanbench      562,456,145 ->    538,401,061  -24,055,084  -4.2768%
    runbench     2,194,976,748 ->  2,168,019,757  -26,956,991  -1.2281%
    digestbench     10,199,161 ->      9,903,103     -296,058  -2.9028%
    widebench       35,218,932 ->     34,386,921     -832,011  -2.3624%
    oneshot         21,164,390 ->     20,924,668     -239,722  -1.1327%
    deepbench      378,118,216 ->    376,926,218   -1,191,998  -0.3152%
    basket          34,693,472 ->     34,590,226     -103,246  -0.2976%
    encodebench  3,882,689,256 ->  3,882,553,572     -135,684  -0.0035%
    livebench    3,320,972,422 ->  3,320,922,225      -50,197  -0.0015%
    pendbench      221,101,809 ->    221,102,601         +792  +0.0004%
    escapebench, indexbench, readbench byte-identical

Ten of the fourteen fall, three hold to the instruction, and one rises. The
row that rises is `work_pendbench`, which landed on **221,102,601** — 792
instructions, four ten-thousandths of a per cent, and the only worsened row in
the vein.

`.text` is the price, and it also came in under projection. The sum across the
fourteen binaries landed on **1,783,980**, up 119,584 from 1,664,396
(+7.1848%), against the +13.2% that `runbench.ll` alone projected. runbench
itself takes 286,034 -> 315,522 (+10.3093%); jsonbench, oneshot and livebench
each take roughly nineteen thousand bytes; basket, escapebench, indexbench and
readbench do not move at all, and pendbench and digestbench each lose a
handful. A binary that links less of the library has less to inline. Machine-
code size carries no welfare term by the 2026-09-05 gavel, and the vein stays
exact so the growth is watched rather than scored.

No allocation counter moved. `all_counters.sh` reads the twelve cost goldens
and the lazy tier and every one agrees byte for byte, which is the expected
answer — a linker flag cannot change how often the allocator is called — and is
written down because a sweep that is run and not reported is a sweep nobody can
check. The four compile rows are identical too (`compile_allocs` 29,473,
`compile_instructions` 45,523,131, `entry_instructions` 152,087,783,
`library_instructions` 152,459,094): `kanso check` stops before codegen, so the
release link is not on that path at all.

Welfare 67.90189170478736 -> 67.99163092139815, banked. The run term is the
only one that moved and it moved the right way, so the trade the objective sees
is a 1.2281% fall in run instructions against a `.text` growth it does not
weigh. The objective is blind to machine-code size by the 2026-09-05 gavel, so
the +7.1848% is a real cost the score cannot express. It is the gavel that
makes this an acceptable trade, and the score only confirms the half of it the
gavel left weighable.

**OPEN.** 3000 reads −2.3746% in this container and was not taken, on the
byte-per-instruction argument above. If `.text` ever gains a welfare term the
argument changes shape and both rungs want re-measuring on CI rather than here.

## 2026-09-13 (fourth) — a binary nobody is going to count is built without the counters

**BUILT.** Every inlined fast path the emitter writes — the byte append, the
whole-string append, the list push, the map insert, five more — begins by
loading `k_stats_on` and branching. The shortcut bypasses the runtime call that
would have counted, so when counting is on it has to bail. When counting is off,
which is every run that is not a cost golden, the load and the branch are paid
for a question whose answer was fixed before `main`.

`kanso build --counters` keeps the gates. A plain build strips them.

**What it is worth, measured three ways that agree.** First by patching the
eight `load i32, ptr @k_stats_on` lines in `runbench.ll` to a constant and
relinking with the shipped recipe, control and variant at matched binary-name
lengths:

    control (both gates live)     2,141,315,030          -   0.0000%   .text      +0
    emitted eight folded          2,115,346,210 -25,968,820  -1.2128%   .text  -2,048
    runtime twenty-seven folded   2,128,127,196 -13,187,834  -0.6159%   .text  -2,944
    both folded                   2,102,158,376 -39,156,654  -1.8286%   .text  -4,992

The control reads 2,141,315,030, which is the shipped `runbench` to the
instruction, so the harness is the real link. The two halves are exactly
additive in instructions and in bytes, and stdout is byte-identical in all
three variants.

Second by a static join: the disassembly's 607 instructions at 303 `k_stats_on`
sites, weighted by their own execution counts out of the callgrind dump, sum to
37,740,165. The measurement came in above that because folding also lets LLVM
simplify around the branch.

Third by this change itself, which is the emitted half and nothing else:

    --counters   2,141,315,030   .text 315,698
    default      2,115,346,210   .text 313,650
                  -25,968,820      -1.2128%      .text -2,048

The compiler reproduces the hand-patched figure exactly. THIS CONTAINER'S
NUMBERS; CI measures its own and they go in a second round, because the
instructions gate refuses to compare a row measured on another glibc.

**Two ways out that do not work, so nobody spends the afternoon again.**
`!invariant.load` on the eight loads is arguably sound — `k_stats_on` is written
once, before any emitted code runs — and it does CSE them. It measures WORSE:
+1,875,636 (+0.0877%) and 288 bytes MORE `.text`. Keeping the value live costs
more than the reload saved.

And the gate cannot simply be deleted with the fast path counting for itself.
`k_b_append_into`'s in-place arm does `k_stat_append_fast++` unguarded, so that
counter is free — but the emitted fast path also takes its 32-byte header from
the arena, where the slow path's `k_bytes_owned` reaches `k_alloc` and
increments `k_stat_allocs` and `k_stat_alloc_bytes` UNDER the gate. Dropping the
gate blinds two counters the cost goldens pin. Keeping them exact means three
unconditional read-modify-writes against the gate's two instructions.

**Two readers of the same programs, and repointing one was not enough.** The
twelve `*_counters.sh` gates name their program, and `all_counters.sh` has its
own `vein:program:golden` table naming it again. Repointing only the gates left
the sweep running the bare, gate-free binaries under `KANSO_COUNTERS=1`, and it
reported all twelve veins moved at once. The `.mem` vein had the same shape and
a different cause: `tests/golden.rs` drives a build through the library with
`KANSO_COUNTERS` already set in the child's environment and no way to pass a
flag. So a build running under `KANSO_COUNTERS` keeps the gates as well — a
process that is itself counting is going to count what it builds.

`sh scripts/gates/all_counters.sh` now reads: the twelve cost veins and the
lazy tier agree with their goldens. No counter moved.

**What it costs.** `build_benchmarks.sh` builds twelve of the fourteen
benchmarks twice. deepbench and indexbench have no counter gate — their rows
live in the instructions vein — so they are built once. That is twelve extra
release links in the cost-goldens job, and CI wall time is not a welfare term,
so the objective cannot see the price. It is written here instead.

**OPEN.** The runtime's own twenty-seven sites are the other 13,187,834
(0.6159%) and are untouched. They would want `runtime.c` compiled twice and the
counting object linked into the counting binaries, which is a second object in
the cached-runtime key rather than a second flag on the emitter.

