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

### Riders under the err gavel (the three-combinator model, 2026-08-15)

**Cited: gavel 1, archive 2026-08-15, listed six riders. Four have since
closed — the test surface (archive 2026-08-17, assertions are ordinary
foreign rescue), ch08's pedagogy (scoped into the 1b migration, archive
2026-08-17), and the three small July spellings (archive 2026-08-19, "the
July spellings"). These two are what remain.**

- **Spelling**: names and syntax for annotate and rescue — combinator
  call vs marked arm on a chain — and whether the existing chain
  err-arm syntax is annotate's surface (the chain's value arm and err
  arm are bind's and annotate's callbacks already, spelled as dispatch
  arms).

  **What the rule already is, stated from the language's side.** A bare
  parameter refuses a failure. `fn f x` does not match when the argument
  is an err, so the body never runs and the err comes back out with a hop
  recorded naming the group it did not enter. Closures, constructors and
  builtins have no arms to select between and short-circuit before entry,
  which lands in the same place. Those merge: two failing operands of `+`
  come back as one err carrying both reasons
  (tests/golden/micro/two_failures_in_one_operation_merge.out). A declared
  group that no arm matches hands back the FIRST failing argument instead,
  hopped. Worth knowing before the spelling is settled: the merge is a
  property of operators and constructors, not of calls. (`wrap_err` is the
  one hole in the refusal, because its second argument IS the err it
  wraps.)

  An arm takes an err only by writing `err` in the parameter — `fn f (err
  reason)`, or `fn f (e err)` for the whole failure. Such an arm matches
  only a FOREIGN err, one raised in a hako other than the one the arm's
  file sits in, and having matched it may return anything at all. An err
  from your own package never enters your own arm: dispatch skips it and
  the failure passes as though the arm were not written, which is why
  `provenance.rs` reports such an arm with `error[license]` — it can
  never fire. That pass states the rule in one line: *a group that may
  receive an err raised in its own package must return an err.*

  So: must a function that takes an err return an err? Inside your own
  package, yes, and there is no way to write the function that swallows
  your own failure. Outside it, no — a foreign function that asks for an
  err at the parameter may return whatever it likes, and that is the
  whole of rescue.

  Run on both engines to check the paragraphs above rather than reason
  them: a two-package program where `dep` raises and `app` reads. A bare
  parameter in `app` passed the err through, hop recorded. An `app` arm
  written `(err w:dep/woe)` matched and returned a string, identically on
  native and the interpreter. The same arm written inside `dep` itself
  matched nothing — not even its own `_` fallback, which refuses failures
  too — and the err reached the entry having passed through the group.

  **World A — rescue is a keyword.** `err` may be named only inside a
  `rescue` construct; a parameter may never spell it. The word is already
  reserved and refused by the lexer (src/lexer.rs:481, beside try, catch
  and except), so taking it breaks no program. What it buys: one grep
  finds every place a package handles somebody else's failure, and there
  is a single node for any later rule to hang off. What it costs: rescue
  stops being a function. `when_failed` in design/testing.md is a two-arm
  function that the harness partially applies and passes around; under a
  keyword it cannot be written in kanso at all, and every generic
  combinator over failure — bind and annotate included — needs a keyword
  form or a builtin beside it. The construct also needs its own way to
  select on the reason's shape, which is the arm selection the language
  already has.

  **World B — rescue is an ordinary function, licensed by provenance.**
  What ships today. What it buys: no second calling convention, and
  rescue composes — it partially applies, it can be passed to `map`, it
  can be wrapped. The license needs no annotation because it is computed,
  which is the part a marker cannot do: a package can raise an err whose
  reason type belongs to somebody else and then rescue its own failure
  through the borrowed name. It is enforced twice, by dispatch refusing
  the match and by the license check reporting the dead arm. What it
  costs: the marking is per-parameter, so "every rescue in this package"
  is a grep for `err` in parameter position rather than for one word. And
  a package cannot declare its errs unrescuable by anybody — the only ban
  is on rescuing your own, which is what the round-trip ruling settled:
  "you ensured it would bubble up to the caller, and it did. if the
  caller wants to pass it back to you, so be it."

  **RECOMMENDATION: World B, and the existing err arm is annotate's
  surface.** This reverses the recommendation this entry carried before
  the two worlds were written out, which was that rescue earns a keyword.
  The keyword's price is that rescue can no longer be a value, and
  `std/testing` is the standing proof the project needs it to be one: the
  harness's `when_failed` is a plain function today and the whole test
  surface rests on it being one. The visibility a keyword buys is real
  but smaller than it looks, because an err-taking parameter is already
  written at the declaration where a reader meets it.

- **Construction enforcement**: reason building module-private is
  stated by the doctrine, unnecessary for soundness now that provenance
  is computed, and unenforced.

  **RECOMMENDATION: strike it.** It was the proxy for provenance, and
  provenance is computed (archive 2026-07-28, "Clay: build it
  correctly"). A rule that buys nothing and costs a fleet migration is
  doctrine the code has outgrown.

- Downstream of the spelling, not a separate question: the arm-based
  advisory migrates onto whatever surface is chosen. Implementation.

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
