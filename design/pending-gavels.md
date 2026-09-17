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

### Does the wall survive the fused operators?

**Cited:** the live log's "the wall is bind with a discarded value" (2026-09-17),
which measures the equivalence; the archive's "gavel: the fused chain
operators" (2026-08-31), which says the fused form is the only spelling in
chain position; "gavel: the July letters close" (2026-08-26), letter D, which
minted `done` and removed the wall's premise; and this file's Parked line
"dot-absorbs-`>>`: argued no", which is about the plain dot and does not reach
this question.

**The question.** `a >> b` and `a .> (_ -> b)` are the same program. 570 sites
spell it one way and 184 the other. Which spelling does the language keep?

1. **The wall goes.** `.> (_ -> ...)` is the spelling. About 570 sites are
   respelt, the book's prose moves in ch02 through ch09, and the wall's
   surface leaves src/ast.rs, check.rs, codegen.rs, eval.rs, lexer.rs and
   lib.rs. Every effect sequence a learner reads gets four characters longer
   at each step.
2. **The wall stays as the only discard spelling.** An inline
   `.> (_ -> ...)` is refused where it is written, with "write the wall". 184
   sites shorten, the book is untouched, the compiler keeps the wall and gains
   a refusal. The gap: `a .> shown` where `shown` ignores its argument stays
   legal, because a group arm cannot be refused for what it does with a
   parameter — so the rule catches every discard visible at the site and
   leaves those that are not.
3. **The wall is renamed into the family**, a fourth fused form. Costs what 1
   costs and buys only that four operators look like four operators.

**Recommendation: 2.** It keeps the short spelling for the most common effect
operation and still makes the grammar decide, which is the principle at stake;
it costs 184 sites against 570 and eight chapters. 3 pays 1's price for
appearance.

### Was the wall's simultaneous-failure merge meant to go?

**Cited:** the archive's 2026-08-24 entry measuring the wall, which records
that `>>` built both operands before running either, so two failures raised
during construction merged — `print "left {boom a}" >> print "right {boom b}"`
answering `[a b]` — "the same reasoning the parallel group uses", and which
names the collision as open: "Whether that is the rule to keep is #141, and it
collides with #105 wanting the right side lazy for an unrelated reason:
laziness would buy back Haskell's answer and lose the merge. That is the
trade, and it is Clay's."

**The finding.** It never reached this ledger, and the behaviour changed while
it sat unasked. On the binary at `cc180f2f` the third case answers `boom 1`
alone and `left ok` prints, where the entry says nothing printed in any of the
three. The chat could not find the entry that moved it.

**The question.** Two halves, and the first may answer the second.

1. Was the merge removed deliberately? If some change ruled it out and said so,
   this entry closes on a citation and the archive's #141 is settled.
2. If it went unrecorded, the trade is live and unchanged: keep eager
   construction and the merge, which tells a reader about both failures; or
   make the right side lazy, which buys Haskell's answer and loses the merge.

**Recommendation:** find the commit before ruling. This is cloud's to bisect
and it does not block the wall question, which is answered the same way
whichever way this goes: if the merge is gone, the wall is pure sugar, and if
it comes back, it comes back as a property of bind rather than of a fourth
operator, since `.>` can be made eager in its right side and a lambda's body
is the only thing deferring it.

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

### A byte-position scan on a string, for the escape path

**Cited:** the live log's "the per-call floors, mapped after the inlines"
(2026-09-15), whose closing paragraph measures this change and says in its own
words that it "goes to Clay with this number and is not built here" — and then
no entry was ever filed here, so it went to nobody. Searched this ledger, the
live log and `design/log/compiler-log-archive.md` for `byte-position`,
`find2_below_str` and the escape path: those two log paragraphs are the only
mentions, and the question has never been asked. Also read: kanso#1291's escape
scan, which skipped and then iterated, and kanso#1276's proven length, both of
which worked inside the view rather than removing it.

**The question.** `escape_onto` looks through a string for the three bytes JSON
escapes. It cannot look at the string: `text/find2_below` takes bytes, so
`escape_onto` builds a thirty-two-byte bytes view of the string first, every
time, and drops it unused when the string is clean, which is nearly always.
That is seventeen instructions and thirty-two arena bytes per string,
16,026,750 instructions a run.

A scratch builtin `text/find2_below_str` looks through the string's own bytes
and answers 0 for a miss, and `escape_onto` builds the view only when it hits.
Container A/B on the kanso#1437 leaves, output byte-identical on both programs:
runbench 1,823,814,374 -> 1,801,576,724, −22,237,650 (−1.2193%), the decoder
unmoved. That is more than the view's own seventeen instructions because the
element loop's beat and the view's arena bytes go with it. The patch sits in
the session scratchpad as `escape_str.patch`.

What the measurement cannot settle is the surface. A string's positions are
codepoints everywhere else in `text`, and this primitive takes a byte floor and
answers a byte offset.

**Recommendation:** add it, spelled so the byte offset is never a position. What
this call site asks is where the clean prefix ends, and the answer is consumed
as a bound for building the view rather than as an index into the string; every
`text` operation a program can reach still counts in codepoints. If that reading
is too fine a distinction, the other answer is to keep the view and close the
question — 1.22% of the run term is the price, written down, and the queue
stops re-finding it.


### Pinning `.rodata` to a fixed page, so code growth stops moving the compile rows

**Cited:** the live log's 2026-09-15 entry "the maps parse is outside all three
compile rows", whose closing paragraph measures this and says the choice "is
Clay's, and goes to him with these numbers rather than to the ledger" — where
this ledger is the only channel a waiting decision has, so it went nowhere.
Searched this ledger, the live log and `design/log/compiler-log-archive.md` for
`rodata`, `section-start` and the page-pin family: that one log paragraph is the
only mention, and the question has never been asked here. Also read: kanso#1234,
which chased the same "by layout" noise to glibc's `/proc/self/maps` parse and
was ruled with `setarch` and sorts rather than a link change; and kanso#1404,
which took the checkout path out of the rows. Neither touches section placement.

**The question.** The three compile instruction rows move when code that sits
ahead of `.rodata` in the binary grows, because every literal after it shifts.
Most of this month's pull requests carry a line in their body naming some part
of their compile-row delta as "by layout", and that term is what those words
mean.

Pinning the section to a fixed address removes the term for anything growing
ahead of it. Built with `-C link-arg=-Wl,--section-start=.rodata=0x100000` on
two sources differing by a hundred functions: `program` reads 43,471,592 on
both, identical to the instruction, where the unpinned pair differed by 5,849.

The price is the gap the linker writes. The binary grows from 4,677,120 to
5,724,880 bytes at that address — about 1 per cent — or roughly 52 KiB at
0x40000, one page above today's `.rodata`, which fails the link loudly the day
the sections ahead of it outgrow it.

What the pin cannot reach is growth *inside* `.rodata`. `src/runtime.c` and
`lib/*.kso` are `include_str!`'d into it, so a runtime or library edit shifts
every literal after them whatever the section's start — and those are the edits
behind most of the "by layout" lines. So the pin buys the code-only case and
leaves the common one alone.

**Three more measurements of the term, gathered 2026-09-17, all pointing the
same way.** They matter because the pin is priced against how large the term
is, and every reading so far puts it small.

- `scripts/gates/compile_instructions.sh`'s own header carries a seven-binary
  ladder from 2026-09-04, sources differing only in code nothing reaches. The
  anchored frame spans 1,028 across all seven, 7,632 bytes of unreachable code
  moves it 402, and the movement is not monotone in `.text`. Data-only changes
  leave the frame identical to the instruction. The header's conclusion is that
  a difference near a thousand on this row is not evidence on its own.
- kanso#1473 and kanso#1478 measured the same three rows to the instruction on
  two different binaries, which says the term is quantized rather than noisy
  and that most changes do not move it at all.
- Against that, kanso#1480 moves `compile_instructions` 140,122 and
  `entry_instructions` 487,035 and was escalated as a layout move. 140,122 is
  two orders of magnitude above both calibrations, and 487,035 exceeds the
  330,496 separating that branch's parent from main — so attributing it to
  layout would make the term larger than the algorithmic effect of a whole run
  of indexing changes. Recorded here because an escalation reaching for this
  term is what the entry is about, and because the pin would not have helped:
  whatever carries 140,122 is not the thing the seven binaries measured.

**Recommendation:** decline it, and record the decline. A 1 per cent larger
shipped binary, or a measurement build linked differently from the shipped one,
is a real cost against a term the pin only partly removes; and this repository
has already ruled once, on kanso#1234, that the measurement should not be
special-cased away from what ships. The "by layout" lines are honest — they say
a row moved for a reason the change did not choose — and the rows are read as
deltas against a named base, which is what makes them useful either way. If the
answer is the other one, 0x40000 with the loud link failure is the shape to
take, not 0x100000.

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
