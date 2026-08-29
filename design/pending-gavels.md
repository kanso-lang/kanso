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

### What a digest costs, and whether it stays written in kanso

**Cited: searched design/compiler-log.md (no sha256 entry), the archive (five
entries, `A digest, and the import path that broke it` is the one that rules on
this), and every design/*.md (none mention it). The rationale below is quoted
from that archive entry; nothing has revisited it since.**

`sha256/hex` holds the whole message. Peak arena is linear in the input at
about six and a half thousand bytes per byte hashed, deterministic to the byte:

    message   arena_peak_bytes   per byte
      1,024          7,340,032      7,168
      2,048         14,680,064      7,168
      4,096         27,262,976      6,656
      8,192         54,525,952      6,656
     16,384         108,003,328      6,592
     32,768         216,006,672      6,592
     65,536         428,867,600      6,544

The per-byte figure falls slowly and converges near 6,544, which is what a
fixed per-block overhead amortising against a growing message looks like — one
more piece of evidence that the cost is per-block retention rather than
anything quadratic. At that rate `docs/kanso.wasm`, 1,604,098 bytes, predicts
about 10.5 GB for the hash ALONE. The kernel's out-of-memory report for the
whole `scripts/fingerprint` run read 13,954,684 kB, and the difference is the
rest of that run — the byte list, the padded copy, the other assets, the site's
pages. The two figures corroborate.

A hash reads 64 bytes at a time and carries eight words of state, so peak
should be flat. `cohort_frees=0` and `alloc_bytes` within half a per cent of
`arena_peak_bytes` say what is happening: nothing is reclaimed for the length
of the call. Eight candidate causes have been built as small programs and
measured, and all eight hold the arena at the one-block floor — the in-place
append (93.8% of appends already take the fast path), the module boundary, a
list read while appended to, a long-lived indexed message, per-iteration list
literals, and, tested inside the module itself, both forcing the state
accumulator and removing the per-block thunk entirely. That last one takes
`thunk_live_exit` from one-per-block to zero and moves peak by nothing. The log
entry carries the full table. So the leak needs the real combination, and
whoever takes this on has eight fewer places to look.

WHY IT BLOCKS. `scripts/fingerprint` digests `docs/kanso.wasm`, now 1,604,098
bytes, in the asset-digests CI job. That run was OOM-killed in a container at
anon-rss 13,954,684 kB. It passes on the runner, so the runner has more
headroom — but the headroom falls by about seven kilobytes for every byte added
to the blob, and the blob has grown 23% since the archive entry was written.
`tests/sha256_peak.rs` pins the figures so the next move is visible; it does
not buy any headroom back.

THE RATIONALE ON THE RECORD, verbatim from the archive: "a builtin would buy
speed on a path that runs once per built file and nothing else." That entry
measured the wall clock at 2.6 seconds and did not measure memory. The claim is
sound about speed and silent about the dimension that turned out to bind.

THREE ANSWERS, and this is the choice:

1. **Reclaim inside a long call chain.** The most general, and it would pay
   everywhere rather than here. Also the largest, and it touches the collector.
2. **Restructure the block loop** in `lib/sha256/sha256.kso`. Contained, and it
   keeps the module in kanso, which is the property the original entry was
   protecting. TRIED, TWICE, AND IT MOVES NOTHING — see the recommendation.
3. **Make the digest a builtin.** Smallest and surest, and it spends the thing
   the archive entry declined to spend.

RECOMMENDATION CHANGED, and the change is the point. It was 2, on the reasoning
that a contained fix inside the module was cheap to try. Two versions of 2 have
now been tried and both moved the peak by zero digits, which is what the eight
killed hypotheses above amount to: the cost is not in a shape the module
chooses, so rewriting the module is unlikely to reach it.

So: **1**, and 3 only if 1 is judged too large to be worth one digest. The
question that decides it is whether the arena's failure to rewind here is
specific to this call or general, and nothing in the tree answers that today —
which is itself an argument for looking, because a general answer is worth much
more than a hash.


## Open, not blocking

### Whether a chain line keeps its leading dot

**Cited: searched design/compiler-log.md for the three-forms gavel of
2026-08-26, its `bind is a word too` amendment and its `effect first, callback
second` rider; searched the archive and every design/*.md for a chain-line
grammar ruling and found none. The two sources below are the gavel's own
sample and its own sentence, and they disagree.**

The gavel's sample is dotless:

    io/read_file path
    bind (text -> json/parse text)

and the paragraph beside it says the first argument comes from "the chain rule
already in the language ... the same rule that makes `(expect 1) . to (equal
x)` feed `expect 1` into `to`". That rule is the dot. So either the dot stays
as the continuation marker and only stops being a step in its own right —

    . bind (text -> json/parse text)

— or a continuation line loses it. The second needs a parser change and it
collides with a rule already in the language: an indented line under an
argument-taking statement is one more argument (`src/lexer.rs`), and today the
leading `.` is the only thing telling the two apart. Removing it means an
indented line whose head is `bind`, `annotate` or `rescue` becomes a chain
step, which makes the three words syntax — contradicting the gavel's "nothing
about them is syntax".

346 lambda chain steps in the fleet respell either way, and the shape of the
migration pass differs entirely between the two. The three words already work
prefix-style on all three engines (kanso#1116, merged 2026-08-29); this
decides only how a chain line spells them, and it is the only thing the
migration waits on.

**Recommendation: keep the dot.** It is the smaller change, it keeps the three
words ordinary functions rather than parser-known heads, and it leaves the
existing threading form untouched. The cost is that a chain reads `. bind (f)`
rather than `bind (f)`, which is two characters against a grammar rule that
would otherwise have to know three names.

### Whether an err gains readers a callback can use

**Cited: searched design/compiler-log.md and the archive for `wrap_err`, for
err field access and for `reason` as a reader; found the "one hole in err's
infectiousness" (wrap_err's second argument arrives as a value) and nothing
that opens a second one. The three-forms gavel's own sample is the source of
the question.**

The gavel writes `annotate (e -> "config: {e.reason}")`. An err has no
`.reason` reader, and adding the field alone would not be enough: every
operation on an err propagates it, so an interpolation that mentions one
answers the err rather than a string. A lambda callback can therefore receive
an err and can do nothing with it but pass it on.

A group callback has no such problem — its arm destructures, binding the
reason as a value, and that is the gavel's primary story ("a dispatch group is
a legal callback and its arms match reason types polymorphically"). So the
words work today; it is the lambda spelling in the ruling's own sample that
does not.

**Recommendation: give an err the readers `.reason`, `.cause` and `.origin`,
and make reading one the second hole in infectiousness.** It is the same
carve-out `wrap_err` already has, it makes the gavel's sample compile, and it
keeps the alternative — telling readers to write a group for every annotation
— from becoming the house style by accident. If that is too much surface, the
narrower answer is to strike the lambda form from the sample and teach the
group.

### Whether a compile_instructions move that cannot be work needs an attribution

**Cited: searched design/compiler-log.md and the archive for
`compile_instructions`. The vein's own header in
bench/compile_instructions_golden.txt is the record of why it exists and is
quoted below; nothing has revisited the question this entry asks.**

The vein moved three times in two days from an untouched call graph:

    2026-08-26   +167     a reworded driver message in src/main.rs
    2026-08-27   -251     two match arms in the interpreter's call_builtin
    2026-08-27   +1,954   one ErrorKind match in read_file_text

In each case the edited function is unreachable from the measured path —
`kanso check lib/json` compiles a library and runs no program — and the
counters that measure the front end's work are identical across all three:
allocations 61,981 and peak 822,004. The compiler's own binary is being
rearranged by edits elsewhere in the crate, and the vein reads that as the
front end's work changing.

THE VEIN EARNS ITS PLACE and this is not a proposal to remove it. Its header
records the case: a change took the front end from 90.9M retired instructions
to 67.2M with every other gate reporting nothing. That is exactly what it is
for.

The question is narrower. A RISE with allocations, rounds, visits and peak all
identical, in a diff that does not touch the measured path, currently costs a
golden update, a page-figure update, a log paragraph, and — because the trend
gate calls it a pure regression — an entry in `bench/welfare_floor.json`
attributing a spend. Today's attribution had to say in its own text that
nothing was spent, which is an odd thing for a ledger of spends to contain.

TWO ANSWERS:

1. **Leave it.** The ritual is cheap per occurrence and the alternative is a
   rule that could be leaned on. Silence is the thing the vein exists to
   refuse, and three paragraphs in two days is not a crisis.
2. **Let the trend gate treat an instructions-only move as priced by the log
   sentence alone**, when every other compile counter is identical. The golden
   still moves and the sentence is still required; only the welfare_floor
   attribution is dropped, because there is nothing to attribute.

**A THIRD ANSWER, from 2026-08-29, which is better than either.** The
container this runs in cannot compare absolute numbers with the runner — a
glibc revision apart, about 410 instructions on kq's rows — but its DELTAS
match the runner's to the instruction. Four moves that day, each measured
both places:

    CI          container   what it was
    +144,031    +141,573    work: a scan of a table per call head
    +111,135    +110,534    work: the binary search that shipped instead
     +83,829         +12    layout: a strip_suffix on an unemitted path
      +2,138           0    layout: a guard in a function `check` never runs

Work reproduces within 2%. Layout does not reproduce at all: twelve
instructions against eighty-four thousand, and byte-identical against two
thousand. The two are not distinguishable inside one host and they separate
completely across two, which is a measurement rather than an inference from
the other counters.

So: **let the trend gate treat an instructions-only move as priced by the log
sentence alone when the log sentence carries a second-host delta**, and
require that delta rather than accepting an argument from the call graph.
Two callgrind runs, about a minute, and an attribution that used to be a
paragraph of reasoning is a number. Every layout attribution in the log
before that date argues from reachability; the ones after it measure.

RECOMMENDATION: the third answer. It gives option 2 the evidence it was
missing — a condition that cannot be leaned on, because producing it means
actually measuring — and it makes the log strictly more informative. If a
required measurement feels like too much ceremony for a golden bump, 2
narrowly conditioned on the other three counters is the fallback and 1 is
honest; the cost is small either way. What should
not stand is a spend ledger whose entries say no spend occurred.

### Whether `read_file` is text or bytes

**Cited: searched design/compiler-log.md and design/log/compiler-log-archive.md
for `read_file` and for `text/bytes`. The archive's `A digest, and the import
path that broke it` is the only entry that touches this and it asserts the
opposite of what is true today — "`io/read_file` carries binary content intact
and `text/bytes` exposes it" — which holds on native and not on the
interpreter. No design/*.md mentions it. Nothing has ruled on it.**

`lib/os/os.kso` has one reader, `read_file`, and it does not say what it reads.
On native it reads any file and preserves the bytes; on the interpreter a file
whose bytes are not utf-8 is refused, because the value it reads into is a Rust
`String`. As of today that refusal at least names the reason rather than
claiming the file is absent, which is what the differential law needs from an
engine that speaks less — but the two still answer differently for the same
program, and a caller has no way to say which behaviour it wanted.

THREE ANSWERS:

1. **Text-only, everywhere, and a separate bytes reader beside it.** Go's
   shape. Native would begin refusing files it reads today, which is a
   behaviour change to `scripts/fingerprint` among others, so it needs the
   bytes reader in the same change.
2. **Byte-transparent everywhere.** Needs the interpreter to hold a non-utf-8
   payload, which is a change to what a kanso string is on that engine, and
   the archive records a ruling that a bytes value is real and a list is never
   bytes — so the machinery may be closer than it looks.
3. **Leave it.** Native reads everything, the interpreter refuses clearly, and
   the law is satisfied. The cost is that the oracle cannot run every program
   native can, which is the thing the oracle is for.

RECOMMENDATION: 1. It is the only one where the library says what it does. It
is also the only one that makes `read_file`'s name true on both engines, and
the bytes reader it needs is the same surface `text/bytes` already implies.


### Which claim owns `dep/join` — the bare-enrollment clone, or `dep`

**Cited: archive 2026-07-27 filed this against "the question task #51 holds
a gavel over". Task #51 was RULED on 2026-08-17 as gavel 51, one module —
identity is the canonical path, one dispatch group per name — and built the
same day. The search found no revisit of this case after that ruling, and
gavel 51 does not settle it: gavel 51 is about ONE module reached by two
paths, and this is TWO modules whose names collide inside one namespace.
`module_differential`'s known-defect entry `w1` still records the behaviour,
still pointing at a gavel that has fallen.**

A module declares `pub fn join` and also imports `std/text`, which exports
`join`. From outside, `dep/join` is refused: "`dep` declares `join` pub, but
an import of `dep` exports `join` too and took the name."

Measured this session, which the ledger did not have:

  - `dep/join` is claimed FIRST by a bare-enrollment clone of a NON-PUB ARM
    of std/text's `join` group, carried into `dep` under the file
    `std/text/text.kso`. The exports map is first-writer-wins, so `dep`'s own
    `pub` reads as private.
  - The clone survives only when the importing module declares the same name.
    Otherwise `canonicalize_bare_aliases` folds it away, and `dep/join` is
    an ordinary unknown name. So the collision is the condition, not the
    enrollment.
  - Letting `dep`'s own `pub` win the flag — one line — makes the refusal go
    away and makes `dep/join` reach std/text's arm. With `pub fn join a:int
    b:int` in `dep` so that `dep`'s arm cannot match, `dep/join ["x" "y"] "-"`
    answers `x-y` on BOTH engines. `dep` never exported that function. The
    refusal is the only thing standing between a program and a silent
    re-export of a dependency under a name its author never wrote.

So the flag and the dispatch are one question. Making `dep/join` mean `dep`'s
declaration requires the enrolled clones to stop living in `dep`'s qualified
namespace — and `dep`'s own bare `join` call sites are rewritten INTO that
namespace during qualification, which is what puts them there. The bare
overload space would need a spelling of its own, per module, that a consumer
cannot write.

**RECOMMENDATION: rule that a qualified name is its module's declaration and
nothing else, and give the bare overload space its own namespace.** Go's rule
is already ruled here (a package is a directory named by its import path), and
under it `dep/join` can only mean `dep`'s. The alternative — keep the refusal
— costs an author the right to declare a name any of their imports happens to
export, which no other language charges for, and the diagnostic already has to
tell them to rename someone else's import.

Whichever way it goes, `w1` leaves the known-defect ledger in the same commit.

### What a record prints as, when its module is imported

**Cited: archive 2026-08-02, "an err's reason renders with the compiler's
spelling, not the program's", which ends "That is a gavel." The search found no
entry for it in this ledger, and the 2026-08-25 residual sweep did not carry it
over. `tests/entry_file.rs` holds the spec, ignored, and it is the one ignored
test in the tree that still fails.**

Qualification renames a module's declarations to keep them unique across a
merge, and that spelling reaches render:

    run the file directly     trouble: slow_lane 7
    import it                 trouble: lane/slow_lane 7

Same program, same value, two answers, and which one you get depends on how the
program was entered rather than on anything the program says.

The obvious fix was built and reverted, and the corpus is why. A bare-name
render turned two deliberate pins red: `cross_module_fields` asserts the
diagnostic `` `geo/label` has no field `x` `` and asserts `lib/pair 6 "v"` as
rendered output. Both are right for an IMPORTED type — `lib/pair` is what that
program wrote, and a diagnostic saying `label` where two modules declare one
tells the reader nothing. So the rule wanted is "render the name the asking
module would write", and render is called from the runtime with no idea who is
asking.

Two ways out, as the archive framed them. Either rendering carries the asking
module — wider than it sounds, since the context would have to reach every
runtime render site — or the qualified spelling is simply what a record prints
everywhere, and the fixture is wrong to expect otherwise.

**RECOMMENDATION: qualified everywhere.** Go's package rule is already ruled
here, and Go prints `main.T` for a root package's own type rather than `T`, so
the precedent this project has already taken says the name does not depend on
who is looking. It also fixes the actual defect, which is that entering the
same code two ways prints two things. The cost is real and should be said
plainly: every program that prints a record of its own type gains a prefix, the
root module needs a name for that prefix to exist, and the fixture's
expectation flips from `slow_lane 7` to the qualified form.

Whichever way it goes, `tests/entry_file.rs` stops being ignored — either its
expectation changes or it starts passing.

### `--explain-copies`

**Cited: part-superseded, archive 2026-07-27 — the counter stack
(`bytes_peak`, `cohort_kept`, `carry_dedup`, the trend gate, the welfare
peak terms) served the rest. What is open is only the shape below.**

The *where* half of the observability item — a diagnostic naming the
source site of each evacuation copy. Needs span plumbing through the
carry machinery.

**RECOMMENDATION: decline it until a copy surprises somebody.** The
counters already say how much is copied and when the number moves, and
nobody has yet asked which line did it. Span plumbing through the carry
machinery is a fortnight of work for a question that has not come up.

### The book teaches the boundary language (queued P1, Clay 2026-08-26)

Clay's directive after re-deriving the no-explicit-bind design from
the log a second time: the call-site story must live in the book, high
priority. Half one is DONE (ch04 "nothing is asked of the signature",
2026-08-26): the err half, present tense — short-circuit at the call,
nothing forced on signatures, one function for both call sites. Half
two is GATED on the elaborator build: teaching signature-directed
lifting for effects (`retry (fetch url)` unmarked, binds inserted at
value-demanding positions, collapse at io) — the book speaks in the
present tense, so this half lands with the elaborator, not before.
The assembled argument for the whole is compiler.html entry 23. Its
concluding claim — that the combinator words stay off the surface, "in
every variant under consideration" — was reversed by the three-forms
gavel of 2026-08-26, and the entry now says so: `bind`, `rescue` and
`annotate` are surface words, built on all three engines in kanso#1116.
The three decisions that entry rests on are untouched, and the open
dispatch-vs-elaborator question still decides only the machinery.

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

### A bare call two imports answer alike

**Cited: INTERIM committee ruling, archive 2026-07-27, "a bare call two
imports answer alike is refused" — built, pinned, and explicitly awaiting
Clay's reassessment. `check_bare_ambiguity` is live in src/check.rs today.
The search found no reassessment.**

Two imports export the same name with the same shape, a bare call reaches
the group, and dispatch has nothing to pick by. It used to pick import
order, which the formatter forces alphabetical, so directory names decided
semantics. The interim ruling refuses the call.

**RECOMMENDATION: confirm the interim as final.** Refusing is the
conservative direction — a refused program can be given meaning by a later
gavel, and a silently-resolved one is a commitment nobody made. It has
shipped for a month without a complaint. One word retires the "interim".

### Dependency modules' render arms stay out of the root group

**Cited: recorded once, archive 2026-07-27 — "whether they should is a
surface question for Clay" — in a render-plan.md that no longer exists. The
search found nothing else, and no ruling.**

A module's render arms join its own root group across its files, so an arm
in show.kso matches a type in types.kso. Arms from a *dependency* stay
qualified and never join. A program that imports a hako defining a money
type therefore does not get that hako's rendering for free.

**RECOMMENDATION: keep them out.** Rendering is a visible property of a
value, and a dependency silently changing how a caller's output looks is
the kind of action at a distance the hako boundary exists to stop. The
owning module can export a render arm deliberately, and then the caller can
read that it did.

### `first coll n`

**Cited: design/enumerable.md §9.3, open since the 2026-07-18 ratification.
The search found `take`/`first` in the archive only as the fusion note
("take/first never fuse, so infinite sources keep their meaning"), which
does not touch the question. lib/list ships `first coll` with no n.**

Is there a `first coll n` consumer convenience, or only `take` (adapter)
plus `to_list`? One-right-way says pick one.

**RECOMMENDATION: only `take`.** `first coll` answers a different question
— one element or `none` — and giving the same name a second arity that
returns a list would make the return shape depend on the argument count.

### Where `std/` comes from

**Cited: design/hako.md, "Open questions for the observation clause",
undated. The search found no entry anywhere on how std is distributed.**

Whether `std/` ships inside the toolchain binary or as a pinned hako. It is
a user-facing question: it decides whether a program can pin a std version,
and whether upgrading the compiler can change what a program does.

**RECOMMENDATION: inside the binary, and say so.** A pinnable std means a
matrix of compiler-and-std pairs behind every differential golden, and the
oracle law is expensive enough already. The compiler version becomes the
std version, which is one number for a user to report in a bug.

### Block-born as a dataflow property

**Cited: design/memory-frontier-research.md §4.4 — "Clay's call, since it
widens what the checker admits". The original block-born rule is archive
2026-07-23 (a set target must trace to a direct constructor binding). The
search found no later ruling on widening it.**

The birthday theorem needs the cohort closed — everything born in the block,
nothing escaping — and a node reached by indexing a block-born list is in
the cohort. The shipped rule is syntactic on the binding, which is
conservative rather than necessary. Making block-born a dataflow property
(through aliases, conditionals, indexes of block-born collections, fields of
block-born nodes) is scoped compiler work, and it admits programs the
checker refuses today.

**RECOMMENDATION: hold until a real program is refused.** The case the book
teaches — wiring a cycle among a fixed set of directly-named constructions —
is what build blocks are for, and nothing in the fleet has hit the fence.
Widening a checker is easy to do later and impossible to undo.

### The interpreter's 10,000-frame guard

**Cited: archive 2026-08-15 and 2026-08-19 both record it as a documented
limit alongside the OS stack ceiling. It has been a standing offer, not a
question, since it was chosen.**

Constant chosen to hold under debug builds on the 1 GB thread.

**RECOMMENDATION: close the offer and leave the constant.** It has stood
since it was set, the accumulator rewrite removed the case that used to hit
it, and both limits are documented. If a program ever needs more, that
program is the reason to revisit it.

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
