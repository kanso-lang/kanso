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

RECOMMENDATION: 2, narrowly — conditioned on the other three compile counters
being byte-identical, so it cannot cover a change that did real work. If that
condition feels like a crack, 1 is honest and the cost is small; what should
not stand is a spend ledger whose entries say no spend occurred.


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

**RECOMMENDATION: build it as its own design pass, after the err
spelling above is ruled.** The matcher surface reads failures, so its
shape depends on how a failure is spelled; designing it first would mean
designing it twice.

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
