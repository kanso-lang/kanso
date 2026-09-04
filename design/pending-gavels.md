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

Nothing. The section stays so the next entry has somewhere to land.
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

## The welfare model cannot see the yield hole, because the corpus was written around it

**Searched** design/compiler-log.md (the 2026-09-03 entry names this exact
fix and defers it), design/log/compiler-log-archive.md (2026-07-28 and
2026-07-29 on `desc_yield`'s missing arms and the corpus gap they left) and
every design/*.md. Nothing rules on what follows.

`desc_yield` answered a chain's yield from a table keyed on the head's bare
name. Eight std effect wrappers were absent from it and a loop past any of
them ran on the grow-only arena. The fix carries the yield per declaration in
the inference fixpoint. It works: the `os/read_file!` twin of
`tests/golden/read_beat/reading.kso` goes from `beat_iters=1` to 201.

It costs the front end **+0.2587%** — 42,239,175 to 42,348,436 retired, same
host, same binary layout question as ever, of which about 66,000 is
attributable work and the rest is layout (`demand::analyze` and the parser
move, and this change touches neither). Every runtime counter is
byte-identical. So welfare falls about **0.008**, which is eight times the
gate's tolerance.

**Why this is not the ordinary case the floor rule already answers.** The rule
says a fall means either the change goes or the weights are wrong. Here the
weights are fine and the corpus is blind. The one program in it that reads a
file at runtime is jsonbench, and `bench/make_jsonbench` wrote its main out
into a `fed` pair *specifically to route around this hole* — the reason is in
that file's own comment, with the numbers: 2 arena blocks against 248, 2 MB
peak against 260 MB. This branch deletes the workaround and writes the natural
`os/read_file! "bench/large.json" . go`, and **every counter is unchanged**.
That is the proof: the corpus measures the workaround, so the fix it buys back
is worth exactly zero to the model.

The correctness carve-out in `scripts/welfare/welfare.kso` does not cover
this. It is for the differential law — engines disagreeing — and all three
engines agreed before and after. This is a performance hole, which is
precisely what welfare is supposed to price.

**Recommendation: ship it and move the floor, recording this entry as the
reason.** The alternative readings, both worse: keep the hole (a natural
program pays 130x the peak, and the next wrapper anyone adds pays it too), or
add a benchmark whose only purpose is to make this change score — which is
writing the test after the answer.

What is genuinely open is whether "the corpus is blind here" is an admissible
reason to move a floor at all, given the rule was written to stop exactly that
sentence being used loosely. If it is not, the answer is a corpus change first
and this fix second, and that ordering is the ruling to make.
