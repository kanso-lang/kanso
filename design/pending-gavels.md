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

### The compile-instructions row is bimodal, and a per-chip pin cannot hold it (2026-09-03)

**BLOCKING kanso#1232 and every branch after it.** You ruled on 2026-09-02
(log: "the row moves with the CPU, so record it") that
`bench/compile_instructions_by_cpu.txt` keys the front end's instruction count
by silicon. That premise is falsified. Four readings, all with the glibc
tunables pinned:

| when | key | binary sha | rustc | counted |
| --- | --- | --- | --- | ---: |
| 12:33 | family0x6-model0xcf | 55fb850296d1 | 1.98.0 | 41,832,275 |
| 13:05 | family0x6-model0xcf | 55fb850296d1 | 1.98.0 | 41,831,767 |
| 16:25 | family0x6-model0xcf | de5bfab22fbd | 1.98.1 | 41,831,767 |
| 16:35 | family0x19-model0x1 | de5bfab22fbd | 1.98.1 | 41,832,275 |

Two values 508 apart. One chip produced both on one binary; two chips produced
different values on one binary. **Neither the key nor the binary picks a mode.**

What moves is measured, not guessed: the two runs at 12:33 and 13:05 printed
byte-identical 123-line CPU feature blocks, every kanso symbol agreed to the
instruction, and the whole difference sits in `_int_malloc` (+580), `_int_free`
(+19) and `memcmp-avx2-movbe` (-66) — an alignment difference downstream of a
heap layout difference. Pinning the tunables took the spread from 5,064 to 508
and did not close it. Five consecutive container runs on one binary read one
value every time, so it is not run-to-run jitter within a host.

**Why this blocks, and it is worse than a flaky row.** An exact row is red
about half the time on a chip that produces both modes — but the reference row
is also the bare `compile_instructions=` that welfare, golden_prose and the
TREND GATE read. A mode flip there reads to the trend gate as a counter that
worsened with nothing traded, and it refuses the branch in those words:

    worsened: compile_instructions 41,831,767 -> 41,832,275
    FAIL  a pure regression: something got worse and nothing got better.

It is right by its own rules and the claim is false; nothing in that branch
touches the front end. So the bimodality does not only make one gate flaky, it
makes **the objective's own regression detector fire on noise**, and the only
thing standing between it and a false regression today is which chip happens to
be first in the file. That is not a property to leave load-bearing. The file's own header set out what these runs would test —
"if the chips still disagree, pinning was the wrong explanation and the tunables
come out again" — and this is that answer.

The options, none of them free:

1. **Find the term.** Something outside the pinned tunables moves the heap
   layout. Cost: unbounded, and three sessions have now spent time on it.
2. **Pin the pair.** Record both modes per chip and accept either. Cheap and
   honest, and it weakens the vein to catching moves larger than 508 — the
   dimension exists because a change once moved a quarter of the compiler's
   work with every other counter silent, and 508 is far below that.
3. **Drop the exact pin for a trend.** Contradicts the no-tolerance-bands
   ruling of 2026-08-24 head on, and that ruling has a measurement behind it.
4. **Retire the vein.** It has caught real moves; this is the expensive option
   to be sure about.

**RECOMMENDATION: 2.** It keeps an exact comparison against a known set of
values rather than a band, states the bimodality instead of hiding it in slack,
and leaves 1 open as research rather than a blocker.

Filed with it, not blocking: `measured_on.sh` reads rustc as major.minor.patch,
and the 1.98.0 -> 1.98.1 bump reddened three gates while moving no allocation
or peak counter. Its own header says to pin what has been shown to matter. One
observation; the next point release is the second.

The section stays so the next entry has somewhere to land.
(The sha256 digest question sat here briefly and was bounced on
2026-08-29: performance questions with no surface area are the
implementer's, per this file's own charter. The log carries the
research mandate it left with.)


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
