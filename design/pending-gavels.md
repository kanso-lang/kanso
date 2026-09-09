# Pending gavels

The single source of truth for decisions awaiting Clay — ruled so on
2026-08-23, unifying what had forked across four files. An entry is here
because it is about the language a user meets: surface, semantics,
observable behavior. Implementation details do not come here; whoever
holds the file decides them and answers for the decision in the log.

The lifecycle, restored from this file's own precedent of 2026-08-15:
an entry lives here while open; the ruling is recorded in
design/compiler-log.md, which is the history and nothing else; and the
entry leaves this file in the same commit. Everything that ever left is
one `git log -p -- design/pending-gavels.md` away.

Rules of the ledger:

- **An entry cites its search, or it is invalid.** Before filing, search
  design/compiler-log.md, design/log/compiler-log-archive.md and every
  design/*.md for the question. The entry then says what the search found
  — the ruling that partly covers it, the experiment that answers its
  premise — or states plainly that it found nothing. An entry with no
  citation line is not a pending decision; it is an unsearched one, and
  it does not go to Clay.
- **An entry carries a recommendation.** Every question below says what
  the holder of the file would do and why, so a sitting can be a yes or a
  no rather than a fresh design conversation. Where the recommendation is
  to close the question, one word does it.
- **A gaveled item carries its citation forever.** Where an entry
  survives because only part of it was ruled, the ruling's marker stays
  in the entry. This rule exists because gavel 1b's marker has now
  fallen out of this file twice — once caught and restored, once in the
  `e3052383` rewrite that made this single ledger — and each loss made a
  settled question look open. The log is append-only and cannot lose a
  fact; this file is maintained by hand and has. When the two disagree,
  the log wins.
- STATUS.md may index this file. It does not carry decision text.
- Sessions cite entries by the headings below, never by a session task
  id — a task list is private to its session and its numbers resolve
  nowhere else.
- Edits to this file ride small, promptly-merged PRs, never a feature
  branch, so the ledger cannot fork.

The residual sweep of 2026-08-25 walked the log, the archive and every
design doc for questions that were asked and never answered. What it
found is below: every remaining question asked once, with a recommendation,
so the list can be ruled in batched sittings and end. The intent is that
this is the whole of it. Six candidates the sweep turned up
were already answered by the shipped code or by a later gavel, and those
went to the log rather than here.

## Blocking — a fixture, gate, or merge is waiting

(The sha256 digest question sat here briefly and was bounced on
2026-08-29: performance questions with no surface area are the
implementer's, per this file's own charter. The log carries the
research mandate it left with.)

### The reconstruction the 2026-09-07 ruling ordered has two usable phases, not four, for 376 of the 439 rows

**Cited: the ruling itself, design/compiler-log.md, "the welfare
history's baseline" (kanso#1313), rulings (a) share-weighted and (b)
baseline at the earliest reconstructable row, shares renormalised over
the phases a row carries, no phase extrapolated from another. Also
`bench/runbench_phases.txt`, whose header states which benchmarks have no
phase, and the 2026-09-07 entry "the one-program gavel re-priced the
declined queue", which uses this machinery for a single re-pricing.
Nothing in a design doc or a spec speaks to the coverage question below.**

**The ruling's premise about the inputs is off by two.** It reads: "from
row 70 (2026-08-10) every row carries four run-side instruction counters
— `instructions`, which is the DECODE row, plus `encode_instructions`,
`oneshot_instructions` and `basket_instructions`." Two of those four map
to no phase. `bench/runbench_phases.txt` says so in its own header:
"widebench, livebench, readbench, oneshot and basket have no phase:
runbench does not do their work." So a row the ruling counted as
four-phase is a TWO-phase row — decode and encode, 68.97% of runbench's
measured mix — and 31.03% of the mix has nothing to reconstruct it from.

Read off origin/perf-history (history.jsonl, 500 rows), by phase counters
actually present:

    rows        n   phases carried                     coverage of the mix
    0..14      15   none                               0%      (stay unscored)
    15..390   376   decode, encode                     68.97%
    391..438   48   all eight                          99.98%
    439..499   61   real run_instructions              —

(The ruling's "row 70" and "69 rows before it" are that same boundary
under a different index; the date, 2026-08-10, agrees to the day.)

**Renormalising over the carried phases is extrapolation when the missing
phases move differently, and rows 439..499 are where that can be
checked.** Those 61 rows carry decode and encode AND the real
`run_instructions`, so the ruled construction can be run against the
answer:

    two-phase reconstruction vs real run_instructions, rows 439..499
      movement to track            27.24%
      movement recovered            4.42%
      max |error|                  17.73%   (row 474, 61f0540)
      mean |error|                 14.88%

It recovers about a sixth of the movement. The reason is visible in the
log: over that window the wins were the freeze, the drift-gated chain
step, the tenure walk — deep, escape, index, pend, split — and decode and
encode barely moved. A reconstruction blind to six phases is blind to the
work.

**The same experiment on the eight-phase rows says the mechanism is
sound and the coverage is what fails.** Dropping the same six phases from
rows 391..438, where all eight are present, costs max 5.04% and mean
2.78% against 31.66% of movement. Same construction, same six phases
dropped, 3.5x smaller error — because in that window the missing phases
moved less. **So the two-phase error is not a fixed property to correct
for; it is proportional to how much the unrepresented 31% moved in the
window, and for rows 15..390 nothing in the data says how much that was.**

**What this does NOT block, and what it does NOT fix.** Rows 391..438 carry
99.98% of the mix and have essentially nothing to extrapolate. That
reconstruction is inside the ruling as written, needs no further
decision, and is built — measured through the real pipeline (`welfare
--model` into `welfare_rescore`), 48 rows changed and not one outside
them, coverage 0.44 -> 0.74, and the segment reads 65.66 rising to 67.96
across 2026-09-03..09-06 where it read a flat 91.67.

**It moves the cliff rather than removing it, and an earlier draft of
this entry said otherwise.** Measured both ways:

    boundary     before      after
    390 -> 391   +1.9461    -24.0580
    438 -> 439  -32.6152     -9.0038
    row 390 to row 439, total   -30.7615 both ways

The total is IDENTICAL because it is caused by the 376 rows below, which
is the half this entry is about. Half the ruling does not half-fix the
picture: it redistributes one step into an earlier one. What remains at
438 -> 439 is -9.00, and that is `run_peak_bytes` joining — the term
ruling (c) says has nothing to reconstruct.

It ships anyway, and the reason is per-row truth rather than the
picture: those 48 rows now carry the term the objective actually weighs,
and the +1.95 at 2026-09-03 that currently reads as the compiler getting
better was a coverage change wearing a rise. Every boundary is drawn and
labelled either way (kanso#1346). **The fall Clay is looking at is
entirely the 376 rows' — it cannot be answered without ruling this
entry.**

**THE QUESTION, for rows 15..390 only.** The ruling says reconstruct and
says no phase is extrapolated from another. Under the corrected input
count those two cannot both hold for these rows.

- **(1) Leave them unscored, as rows 0..14 already are.** The chart keeps
  a boundary rule at 2026-09-03 and the line starts there.
- **(2) Score them from two phases and label them.** A third `scored_by`
  value marking the rows reconstructed at 68.97% coverage, drawn
  distinctly, with the 14.88% mean error recorded beside it.
- **(3) Re-measure instead of reconstruct.** Replay today's runbench
  against each old commit's compiler. This invents nothing, and it is the
  only route that produces a real number. Cost is a compiler build per
  sampled commit, and it is unproven that a 2026-08 compiler compiles
  today's runbench at all — untested, because the ruled method needed no
  build.

**RECOMMENDATION: (1).** The reason is `scripts/welfare_rescore`'s own
header — "Nothing is invented, and nothing is fetched at view time." A
line that recovers a sixth of the movement is not a faithful line, and
labelling it does not make a reader's eye discount it correctly; the
label would sit under a curve whose shape is wrong. Option (2)'s appeal
is that the chart looks continuous, which is the wrong thing to buy.
Option (3) is the honest way to get those rows and is worth doing on its
own schedule if the old-compiler question comes back yes — filed as a
lead, not as a condition on this entry. With the eight-phase half built,
the unscored region is the flat early history rather than the interesting
part.

## Open, not blocking


### The book teaches the boundary language (queued P1, Clay 2026-08-26)

**RE-PREMISED AGAIN 2026-08-29 by the effects-are-types gavel, which
supersedes the three-chain-words form.** The call-site story the book
owes is now: `<t>effect` as a first-class passable outcome type;
`bind`, `annotate`, `rescue` as ordinary effect-first functions and the
sole eliminators; no automatic bind — a box where the unwrapped type is
expected is refused, and propagation is bind's contract.

**BOTH HALVES WAIT ON THE IMPLEMENTATION. Measured 2026-09-08 on
`2abcedaf`.** Since 2026-08-29 this entry has said that half one — ch04's
"nothing is asked of the signature" — does not survive as written and can
be rewritten ahead of the surface. Two probes say otherwise. A lambda
handed a failing argument does not run its body, and the call answers the
failure: that is the railway ch04 teaches, live, word for word. And
`<int>effect` is not spellable: the canonical-spacing rule refuses the form
outright, and no checker ever sees a type, because there is no `effect` type
in the tree for one to see. The gavel retired the design. The engines still
run the railway, and the book is present tense, so a rewritten ch04 would
describe a language nobody can run.

What has landed is the smaller part, and the book already has it: `bind`,
`annotate` and `rescue` ship on all three engines (kanso#1116), and ch05
teaches them as ordinary two-argument functions taking the effect first.
Missing is the type — passing a box, and the refusal of a box where the
unwrapped value is expected.

**RECOMMENDATION: hold until `<t>effect` exists, then run the campaign
once.** ch04, ch05's framing and compiler.html entry 23 move together in
that pass. Nothing here is a question for Clay; the entry stays as the
record of what the book owes and what it is waiting for.

### The maps parse is 100% of the compile row's binary-to-binary drift

**Cited: the ruling of 2026-09-03 (NO EXCLUSION; the toggle dropped, sorts
plus `setarch` shipped instead, kanso#1234) and the archive entry that
measured this — "the mechanism, named and accounted to the instruction",
which closes by saying the fact goes to the gavel rather than into a gate.
Nothing in a design doc or a spec speaks to it.**

**THE MECHANISM IS NAMED NOW, AND IT IS A TERM ALREADY RULED ON.** callgrind's
call graph: `std::rt::lang_start_internal` calls `pthread_getattr_np`, which
parses `/proc/self/maps` with `getline` and `sscanf` to place Rust's stack
guard. Splitting each profile into that parse and the program:

    binary                      row          maps parse   the program
    9fcc6686dc47 baseline       42,344,081      112,580    41,878,959
    45c6dbed10bb +64 KiB .bss   42,346,211      114,710    41,878,959
    2a4e10fb2116 100 fns        42,345,904      112,586    41,880,776
    5e73453bcc7b 200 fns        42,343,660      110,317    41,880,801

The `.bss` probe adds no code and the compiler's work is **identical to the
instruction**. All 2,130 of the row's move is the parse.

kanso#1234 found this term and the ruling of 2026-09-03 was NO EXCLUSION, so
**nothing here asks to exclude it and nothing has been changed.** The new fact
is its size: 0.27% of the row and 100% of its binary-to-binary drift, with
`std::rt::lang_start::{{closure}}` sitting still through a change that moved the
published row by 2,130. The ruling was made when the term was known to exist
and not known to be the whole of the drift, which is the question this entry
asks: does the 2026-09-03 ruling stand on the new number?

**RECOMMENDATION: it stands, and this entry closes on a word.** The ruling
was that the row counts what the binary costs to start, term and all, and
0.27% is not a reason to reopen a decision made on principle. What the
number does change is how a session reads a 2,130 move on the row: as the
loader's, until `lang_start::{{closure}}` says otherwise.

**FILED WITHOUT A HEADING until 2026-09-08**, appended under Parked, where
the ledger's own navigation could not see it — sessions cite entries by
heading, STATUS.md indexes by heading, and neither could reach this one.

## Stale — the July campaign's unclosed letters (GAVELS.md, retired here)

EMPTY. Clay ruled the last five in one sitting on 2026-08-26 — C struck,
`done` minted for D, G struck on the July provenance measurement, Z
confirmed declined, AA explicit-cast only. Every letter A1–X, BB, C, D, G,
Z and AA now has a ruling in the log or the archive; the section stays as a
header so a reader looking for the campaign finds where it went.

## Parked — on the record, no action

- `<<` labels: walls cover staircases; revive on real DAG demand.
- Labeled nameless patterns: parked 2026-08-19 — needs a fresh look
  against the post-24 language, not pending. Group headers stay behind
  it.
- dot-absorbs-`>>`: argued no — erases the visible then/bind split.
- Postfix index on `)`: `(sort xs)[1]` stays illegal; bind-then-index.
- `;` inline separator: the borrow if inline groups are ever demanded.
- `&` as bitwise: orthogonal, someday.
- `serve` / processes: the executor-loop primitive; next design
  campaign — three investigations already terminate there. The July
  reification form (an err becoming an inert Failure record at the
  supervisory boundary) died with gavel 1; the campaign starts from
  the three combinators.
- Hako tag-signing and checksum policy: parked in design/hako.md until
  something is worth attacking. The lock already carries a sha.
- Monorepo hakos (several modules per repo): the path shape allows it;
  the lock-granularity decision waits for a real case.
- Survivor cap 4× block threshold: the multiplier is a judgment call;
  the principle (the dance's transient stays at threshold scale) is in
  the log.
