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

### What the compile term counts once codegen is in it

**Cited:** the archive's "gavel: welfare measures what compiling costs, not
what it counts" (2026-08-25), which this builds; the live log's entry of
2026-09-16 naming the defect; the archive's "gavel: no term for machine-code
size" (2026-09-05), which declined a size term because "a term would need a
satiation and a weight argued from cases that do not exist yet"; and the
2026-09-15 rule that external state is normalized before it is measured.

**The question.** The compile term must count codegen. Two things are
undetermined and both change the numbers, so cloud cannot pick baselines
without them.

**One — how far down the pipeline.** `kanso build` writes `.ll` and then runs
clang in a subprocess.

1. kanso's own emitter only. Fully in-process, countable under callgrind the
   way the front end already is, and independent of the host toolchain.
2. The emitter and the clang invocation. What a person waiting on a build
   actually waits for. Clang's instruction count is deterministic per clang
   version, and pinning that version in the golden's measured-on line is what
   the external-state rule already asks of every other vein; `host_gate.sh`
   refuses a comparison on a host that does not match.

**Two — which tier.** `dev_clang` runs `-O0`, `release_clang` runs `-O3 -flto`
with `-mllvm -inline-threshold=2000`.

1. Price the dev tier. Dev is what iteration pays, release optimization stays
   free, and the objective's asymmetry becomes deliberate: everything in the
   edit-test loop is priced, everything that runs once at release is not.
2. Price the release tier. The number is what CI and deploys pay, and it puts
   a budget on optimizer work that reading 1, by design, does not.
3. Price both, as separate counters with their own weights.

**Three — and this one dissolves Two.** Clay proposed, the same hour this
entry was filed, that the objective split in two with a meta-welfare over
them: "if we're optimizing for production performance (CPU and memory) and
not compile performance, then compile performance becomes more like a very
dialed-down input... then we have a separate welfare for the interpreted
version, where start-time is vastly more important than speed which is more
important than memory usage... sometimes it will make sense to do a change
which makes development speed much better in exchange for a very small
production performance cost, or vice versa."

Under that structure the tier question has no answer to pick, because both
tiers are counted in different places:

    development welfare   front-end cost (`kanso check`, run by `kanso test`
                          on every invocation), dev-tier codegen (`-O0`),
                          interpreter start-up, interpreter speed, interpreter
                          memory
    production welfare    native run instructions, native run memory,
                          release-tier codegen (`-O3 -flto`)
    meta-welfare          a function of the two

One correction to the proposal as stated: compile cost does not dial DOWN, it
MOVES. `kanso test` runs the front end every time, so front-end cost belongs
beside interpreter start-up as a first-class development term, and production
welfare carries codegen rather than checking.

**Recommendation: 3, with one floor.** It prices every counter exactly once,
in the model whose user pays for it, and it is the only reading that makes
interpreter start-up expressible at all — a cost paid on every test run and
never in production, which no single scalar can weigh. Failing a ruling on 3,
the fallback is 2 and 1: the whole pipeline including clang, on the dev tier,
with the clang version pinned in the golden header.

**Three things the ruling should settle with it.**

- **The meta layer needs a job.** `a·W_prod + b·W_dev` with a linear meta is
  algebraically one flat term list; it buys legibility and nothing else.
  Saturating the meta makes the composition real: a sub-welfare near its
  ceiling then earns little from further wins, which is how the model would
  say "the interpreter is fast enough now."
- **One floor, not three.** Ratcheting the sub-scores separately re-enables
  the part-against-whole optimization that "the sum is the objective; the
  terms are diagnostics" was written to stop.
- **Start-up is the region normalized out of the compile row on 2026-09-15.**
  Measuring it is not a contradiction — noise inside one measurement is the
  object of another — but the counter must count kanso's own start-up work
  and normalize the loader's, which is the count-from-`main` machinery
  kanso#1439 already built.

**What follows either way.** The floor re-ratchets as a model correction, per
the 2026-08-25 gavel's own instruction. The run terms stay measured on the
release tier, because that is what production runs. The model's surface
roughly doubles under 3 — about eight counters against today's five — and
every new term joins `bench/objective_sources.txt` and its replay spec in the
same commit, because this model's PROSE has gone stale twice while the file
never did.

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
