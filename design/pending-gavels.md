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

## Open, not blocking


### The compile row's residual layout term (was blocking; no longer)

**ANSWERED on 2026-09-04 by building what Clay asked for, not by a ruling on
the shapes this entry listed.** He read the finding that the row's drift was
Rust's stack guard parsing `/proc/self/maps` and said to force the measurement
to be consistent instead. So the row now reads `std::rt::lang_start::
{{closure}}` inclusive — the compiler's own work, every libc call it makes
included — and drops the 465,122 instructions of loader and stack guard above
it. That is not the "stop counting glibc" ruled out on 2026-09-03; it is
counting the program rather than the process.
`bench/compile_libraries_golden.txt` pins the five sonames beside it, because
growing a dependency is the one compiler change that moves the dropped half.

**What is left, measured on seven binaries whose sources differ only in code or
data nothing reaches:**

    variant           .text     row         maps     program
    baseline          2550854   42,344,081  112,580  41,878,959
    +50 dead fns      2552534   42,348,024  114,845  41,879,987
    +200 dead fns     2558486   42,347,128  112,586  41,879,361
    +400 dead fns     2565174   42,348,044  110,341  41,879,922
    +3 KiB .bss       2550854   42,346,221  114,720  41,878,959
    +64 KiB .bss      2550854   42,346,221  114,720  41,878,959
    +64 KiB .rodata   2550854   42,344,099  112,598  41,878,959

The row spans 3,963 and the frame spans 1,028. Where the difference is data the
drop is total; where it is code it is partial — 7,632 bytes of `.text` nothing
calls moves the frame 402, and not monotonically. An earlier reading of four
binaries saw only the data cases and read invariance into them, which the
`.text` probe corrects.

**So the question shrank; it did not close.** A ratchet on this row can still
bank about a thousand instructions of link luck. The smallest front-end move
this log records is kanso#1161's -0.07%, about 29,000 instructions, so the
residual sits at a twenty-eighth of the smallest real change where it used to
sit at a seventh. That is why this is filed here rather than under Blocking:
nothing waits on it, no gate is red for it, and the shapes the old entry
listed — welfare reading each row against its own first value, or the compile
term leaving the ratchet — are still available if the residual ever matters.



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
