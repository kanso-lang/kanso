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

## Open, not blocking

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


### Riders under the err gavel (the three-combinator model, 2026-08-15)

**Cited: gavel 1, archive 2026-08-15, listed six riders. Four have since
closed — the test surface (archive 2026-08-17, assertions are ordinary
foreign rescue), ch08's pedagogy (scoped into the 1b migration, archive
2026-08-17), and the three small July spellings (archive 2026-08-19, "the
July spellings"). These two are what remain.**

- **Spelling**: names and syntax for annotate and rescue — combinator
  call vs marked arm on a chain — and whether the existing chain
  err-arm syntax is annotate's surface.

  **The question is where err-handling LIVES, not how tightly it is
  contained.** That is Clay's framing, 2026-08-26, and it reframes what
  the two worlds below are about. Containment was settled by the
  round-trip ruling: a foreign party may hand a reason back to its
  raiser, "and if the caller wants to pass it back to you, so be it".
  What is open is whether access to a wrapped value goes through
  dispatch or through a named function.

  **What the rule is today, from the language's side.** A bare parameter
  refuses a failure: `fn f x` does not match when the argument is an
  err, so the body never runs and the err leaves with a hop. An arm
  takes one only by writing `err` in the parameter, and such an arm
  matches only a FOREIGN err. Your own package's err never enters your
  own arm — dispatch skips it and the failure passes as though the arm
  were not written, which is why `provenance.rs` reports it with
  `error[license]`.

  Verified on both engines rather than read off the passes: a bare
  parameter passed the err through with a hop; `(err w:dep/woe)` in
  another package matched and returned a string; the same arm inside the
  raising package matched nothing, not even its own `_` fallback.

  **World A — err-handling lives in dispatch.** What ships. Every
  pattern match asks whether its argument is a failure and whose. The
  cost is not the rule, it is where the rule sits: `match_one` carries
  an err case on every pattern shape, `match_params` and the dispatch
  loop thread the arm's package through every match, `provenance.rs`
  exists solely to answer "may this arm see its own err" by fixpoint
  over the call graph, and native and wasm each carry a
  `k_not_own_err` beside an interned package literal per declaration.
  The failure mode is a silently dead arm: written, never fired, and
  only a whole-program pass can tell you.

  `_:some` swallowing a foreign failure (fixed 2026-08-26, #1052) is
  this world's characteristic bug. It needed no `err` anywhere: an
  annotation meaning "not none" reached an err because
  `type_match_depth` had to know about errs at all.

  **World B — err-handling lives in named functions.** `bind` unwraps,
  hands the value to a continuation, re-wraps. `annotate` and `rescue`
  hand out the err or its reason and enforce the package constraint
  themselves, in one place, at the call. Dispatch goes back to being
  dumb: a parameter is a parameter, and an err never reaches a user arm
  because propagation short-circuits before dispatch. Everything in the
  paragraph above is deleted. The refusal becomes a diagnostic at the
  rescue site naming the raising package, rather than an arm that can
  never match.

  No rule of the form "a function that takes an err in any argument
  position must return an err" is needed, because a function cannot
  receive one it did not ask a combinator for.

  **What does not change either way.** Propagation stays implicit —
  `f (g x)` is `f (g x)` whether or not `g` failed — so err-in/err-out
  remains a FACT about calls; it stops being a rule anybody writes down
  or a checker enforces. And the merge semantics are propagation rather
  than access: the construction divergence of 2026-08-26 (native and
  wasm returned the first failing field where the interpreter merged)
  was three engines implementing propagation separately, and World B
  does not collect that win.

  The chain's err arm survives as surface in either world: desugar it to
  `bind`/`annotate` calls rather than to dispatch arms, and the spelling
  programs already use and the book already teaches is untouched.

  **RECOMMENDATION: World B.** This is the second reversal in a day and
  the reason is structural rather than about containment, which is what
  the first two arguments got wrong. World A buys nothing World B
  cannot state more directly, and it pays for it in every dispatch.

- **Construction enforcement**: reason building module-private is
  stated by the doctrine, unnecessary for soundness now that provenance
  is computed, and unenforced.

  **RECOMMENDATION: strike it.** It was the proxy for provenance, and
  provenance is computed (archive 2026-07-28, "Clay: build it
  correctly"). A rule that buys nothing and costs a fleet migration is
  doctrine the code has outgrown.

- Downstream of the spelling, not a separate question: the arm-based
  advisory migrates onto whatever surface is chosen. Implementation.

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
The assembled argument for why the combinator words stay off the
surface is compiler.html entry 23; the open dispatch-vs-elaborator
question above decides only the machinery, not this surface.

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

The July design doc ruled its A1–X and BB (those rulings are in the log
and archive; the doc's full text is in git history). Letters that never
closed:

- **C — pure/yield**: does the fold-yield idiom need a named primitive
  (`out >> yield store`) or does a plain value on `>>`'s right
  auto-lift? Asked before `>>` deferred its right side.
  **Cited: the search found no ruling; the deferral gavels that changed
  its premise are archive 2026-08-15 onward.
  RECOMMENDATION: strike as asked, re-ask if the idiom reappears. Under
  deferral the right side of `>>` is a description that gets demanded,
  which is the lifting the question wanted, so the primitive has nothing
  left to do.**

- **D — what a succeeded effect yields**: `none` today, whose silent
  railway-skip is a footgun; a `done` marker was the alternative.
  **Cited: the search found no ruling. The nearest is archive 2026-08-25,
  `>>` stops at the first run-time failure, which settles sequencing and
  not the yielded value.
  RECOMMENDATION: mint `done`. The footgun is real — a succeeded effect
  yielding `none` means a chain that tests for `none` cannot tell success
  from absence — and it is the one place in the language where a value
  means two things.**

- **G — eta-reduction as canon**: ban the forwarding lambda
  (`map (c -> fetch c)` → `map fetch`) plus the composition rules a
  dispatch group held as a value still owes
  (design/function-values.md).
  **Cited: premise answered, archive 2026-07-25 "BUILT, MEASURED,
  DECLINED: eta-reduction is not semantics-preserving here" — an `err`
  records a hop per function, so the two forms print different
  provenance and native stops agreeing with the oracle. Two forms that
  trace differently cannot be canonicalised into each other.
  RECOMMENDATION: strike G on that reason. One word closes it; the
  composition-rules half stays.**

- **Z — errors without exceptions**: presumed declined — the 2026-08-15
  err gavel kept err with the foreign-only rescue license, which is the
  world Z1 abolished.
  **Cited: archive 2026-08-15, gavel 1.
  RECOMMENDATION: confirm declined. One word and this line moves to the
  log.**

- **AA — newtype dispatch acceptance**: the live half is typeset
  acceptance as the idiom vs explicit cast only. The 2026-08-19 ruling
  covered the declaration and ctor form, not acceptance.
  **Cited: archive 2026-08-19, "the July spellings" — `type
  post_body:string` declares, `post_body ""` constructs, and acceptance
  is untouched by it.
  RECOMMENDATION: explicit cast only. A subtype that is accepted wherever
  its base is accepted is a comment, and the reason to mint one is that
  the compiler refuses the mix-up.**

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
