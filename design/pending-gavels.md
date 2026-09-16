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

### The box constructor's spelling

**Cited:** the live log's "gavel: the box is explicit, an err is a value, and
a bare err halts where it lands" (2026-09-15), which rules that a value or an
err can be boxed by hand and leaves the word unnamed; the archive's "gavel:
effects are types, and the words are the only doors" (2026-08-29), which
names the type `<t>effect` and the three eliminators `bind`, `annotate`,
`rescue`; and "gavel: the fused chain operators" (2026-08-31), which gave the
three words their chain spellings. Nothing names the introducer.

**The question.** What is the prefix word that boxes a value or an err by
hand, so that `<int>effect` can be built in pure code? It is an ordinary
one-argument function, effect-shaped in its answer and value-shaped in its
argument, and it needs a chain spelling only if a chain ever ends by boxing,
which nothing in lib does today.

**Recommendation:** `effect`, the type's own name in prefix position:
`effect 5` answers `<int>effect` holding 5, `effect (err "bad")` answers a
box holding the failure. A type spelled `<t>effect` and a constructor
spelled `effect` read as one thing, the way `err reason` builds an err. No
chain spelling until a chain wants one. Cloud builds against this unless
Clay names a different word; it does not block the build.

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

**ch05 and entry 23 done with the type's build, 2026-09-10.** ch05 gained
"a box has a type": `counted` holds a `<string>effect` and hands it on,
`unopened` shows the refusal of a box where `length` reads a value, and the
sentence "no effect type to declare" is gone. compiler.html's entry 23
points at the section. Sizing the refusal sample found the check blind to
a library function whose tail is a `.>` step (`os/read_file`), fixed in the
same build. ch04's "nothing is asked of the signature" still describes the
railway, which still runs, and waits on the 2026-08-31 rider.

**The campaign has run, 2026-09-14.** The recommendation above was to hold
until `<t>effect` exists and then run it once. It exists (kanso#1372, merged
2026-09-13) and the pass landed in four pieces: kanso#1392 gave ch05 the
three chain words and their fused spellings, kanso#1394 gave ch04 its rescue
collision and boundary panel, kanso#1406 gave ch05 `done`, and kanso#1412
gave ch04 the clause naming the type — the live remainder, by its own log
entry. Counted on merged main: ch04 carries the type once, eleven fused
operators and `done`; ch05 carries the type twice, thirty-five operators and
`done` five times.

What is left is the single paragraph this entry has named since 2026-08-29:
ch04's "nothing is asked of the signature", which describes the railway and
waits on the 2026-08-31 rider. STATUS.md's row for this ruling came off on
2026-09-14 and that paragraph moved into the rider's row, since the rider's
ruling is what releases it. Still nothing here for Clay.

**Released, 2026-09-15.** The rider is retired by the gavel "the box is
explicit, an err is a value, and a bare err halts where it lands" (the live
log). ch04's paragraph moves with that build. Still nothing here for Clay.

### How far does a binding position carry a box?

**Cited:** the live log's "a box handed to a binding parameter reaches the
dispatch, and the dispatch answers wrong" (2026-09-16), which names three
possible answers and rules none of them; the entry below it of the same date,
which measures the cheapest one; the archive's "gavel: effects are types, and
the words are the only doors" (2026-08-29), which says a box is opened by
`bind`, `annotate` and `rescue` and by nothing else, and does not say what a
box handed to an ordinary parameter does. Ten micro and runtime fixtures pin
today's answer on three engines, `a_description_reaches_a_dispatch` and
`a_plain_dot_hands_the_box_over` first among them.

**The question.** A box carried through a plain parameter is invisible to the
checker from then on. `encode_onto` handed a box directly is refused;
`elem_onto x` then `encode_onto x` inside that body is not, because nothing
says `x` holds a box. kq lost three unit tests to that shape and kanso-json
two sites, each a box arriving at a group with no arm that could match it, so
the program died at run time with a diagnostic about arguments rather than
about the box.

Refusing a box at every bare-binder position closes it and was built and
measured: one refusal across the whole tree's modules, and TEN in the corpora,
two of which are the ruled behaviour itself — a description reaching a
dispatch lands on the bare arm, and `held e` receives a box and hands it back.
So the blunt rule is not available without reversing those.

**Recommendation:** track the box through the parameter — a position bound to
a box carries a box into the body, and the existing refusal then fires at the
call inside it. That is a typing change rather than a check, it leaves every
one of the ten fixtures alone, and it is the answer the log entry that opened
this thread already called the honest one. Cloud does not build it until Clay
rules, because it changes what the checker proves about every program, and it
does not block anything in flight: every site in kanso, kq and kanso-json is
spelled today so that no box reaches a group with no arm for it.

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
