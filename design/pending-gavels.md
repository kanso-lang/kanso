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

### The welfare history's baseline, after the one-program gavel

**Cited: the 2026-09-06 gavel ("the objective measures ONE consolidated
run program", log entry of that day, kanso#1284) and the directive that
followed it the same day: rewrite welfare history under the new
definition, instructions as the sum of the phase counters, peak as the
max of the phase peaks, baseline at the earliest reconstructed row,
re-set the floor. The #1284 entry left one marker OPEN -- whether the
compile rows re-baseline too. The archive holds the precedent for the
method (#729, #741, #746: "re-baselined so the corpus change banks
nothing", each scaled by the measured factor) and nothing on which row
the base sits at. No design doc and no spec speaks to it.**

Carrying out the directive measured four things, from origin/perf-history
(500 rows) and bench/welfare_floor.json, and three of them need a word:

1. The 75.50 -> 51.89 cliff is the run baseline re-based to today, not the
   chart: `run_instructions` and `run_peak_bytes` are based at their own
   current readings, so both run terms score at exactly 1.0 and every
   accumulated runtime win is discarded, while the three compile rows keep
   real history (compile_instructions 56,563,967 -> 42,061,735, ratio 1.34).
2. All eight runbench phases map one-to-one onto an old counter, but the
   repetition counts were chosen to hit target shares, so a raw sum has a
   different mix from runbench: encode is 53.16% of the raw sum against
   34.43% of runbench, index 0.06% against 4.97%, digest 0.93% against
   5.05%, decode 18.83% against 34.54%. Under a raw sum indexbench's 488x
   (#1172) is invisible. A share-weighted index -- each phase's ratio to
   the anchor, weighted by runbench's own measured shares, calibrated to
   equal runbench at the changeover -- reproduces the ruled mix.
3. Only 48 of 500 rows carry all eight phase counters (2026-09-03 on);
   426 carry `encode` alone, the earliest among them. Based at the
   earliest row overall (2026-08-10, one phase) the reconstruction reads
   2.1535 against today; based at the earliest eight-phase row
   (2026-09-03) it reads 1.3166 share-weighted, 1.3537 raw. The literal
   directive gives the first, resting entirely on encodebench
   extrapolated to a mix it is a third of.
4. The peak half recovers nothing: the earliest row carrying any phase
   peak is 2026-09-03, its max is scan_peak_bytes = 198,180,864, and that
   counter has not moved all window, so max-of-phase-peaks reconstructs
   to exactly 1.0000 and the memory term stays where it is.

**RECOMMENDATION, one word each.** (a) Share-weighted, because the raw
sum re-weights the objective away from the mix the gavel ruled.
(b) Baseline at the earliest eight-phase row, 2026-09-03, because a base
built from one phase of eight is a guess wearing a date; the rows before
it stay unscored, as the 151 already are. (c) `run_peak_bytes` stays
based at today with the floor file saying so, until runbench has history
of its own. (d) The compile rows keep their accumulated baselines -- the
gavel re-defined the run side and said nothing about the compile side,
and the index reads what it reads either way. Each answer is a one-line
change to the floor file; the reconstruction and the rescore follow
the answers, and nothing moves until they are recorded.

## Open, not blocking


### The book teaches the boundary language (queued P1, Clay 2026-08-26)

**RE-PREMISED AGAIN 2026-08-29 by the effects-are-types gavel, which
supersedes the three-chain-words form.** The call-site story the book
owes is now: `<t>effect` as a first-class passable outcome type;
`bind`, `annotate`, `rescue` as ordinary effect-first functions and the
sole eliminators; no automatic bind — a box where the unwrapped type is
expected is refused, and propagation is bind's contract. Half one (ch04
"nothing is asked of the signature") DOES NOT survive as written: its
short-circuit-at-the-call story describes the retired railway and needs
rewriting on explicit elimination. Half two lands when the typed-effect
surface is implemented, present tense as always. compiler.html entry 23
owes a rewrite or retirement in the same campaign.


### An assert hako

**Cited: the licence half is ruled — archive 2026-08-17, assertions are
ordinary foreign rescue. What is open is the surface shape only.**

A real assertion library in the rspec direction Clay sketched —
`(expect 1) . to (equal x)` — as its own small surface design, never
improvised inside a test fix. Its arms are foreign to every tested hako,
so the err license needs nothing special. Queued 2026-08-17.

**RECOMMENDATION: build it as its own design pass. The gate is lifted.**
The matcher surface reads failures, so its shape depended on how a
failure is spelled — that is ruled (three-forms gavel, 2026-08-26) and
built on all three engines (kanso#1116), so designing it now cannot mean
designing it twice. `rescue` is the word a matcher's own failure door
would use.


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
published row by 2,130. The ruling was made when the term was known to exist and
not known to be the whole of the drift, and this entry is where that goes.
