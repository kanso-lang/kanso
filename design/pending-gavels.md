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

### Obeying gavel 24 costs 0.10 welfare, and `--set` refuses a fall

**Cited: the welfare doctrine in CLAUDE.md — "a fall means the change is
worse by the project's own stated preferences... either the change goes, or
the claim is that the weights are wrong." The search found no ruling on what
happens when a change is compelled by a gavel and the index falls anyway.
kanso#999 moved the floor once before, but for a corrected figure rather
than a real fall.**

kanso#1034 deletes `json/failure_position` and `json/failure_reason`, which
gavel 1b and gavel 24 forbid between them, and reaches the same facts
through `std/testing`'s `when_failed`. It cannot merge as it stands.

    compile_allocs        64,950 -> 65,543
    front_end_rounds          40 -> 42            welfare term
    front_end_visits      17,786 -> 17,886        welfare term
    compile_peak_bytes   864,300 -> 870,263       welfare term
    compile_instructions  59,773,156 -> 60,818,284
    welfare                84.89 -> 84.79         floor 84.89

The cause was isolated rather than guessed: reverting `json.kso` alone and
keeping the new test file gives 65,801 allocations and 42 rounds, so all of
the rise is the test file and the deletion claws a little back. The test
file's one new line is `import "std/testing"`, which pulls a whole module
into the program the compile golden measures. The library itself shrank by
two functions.

Two framings:

1. **Move the floor to 84.79 and record why.** The change is compelled by
   two gavels and cannot go. The welfare model has no term for doctrine
   compliance, and what a model leaves out it weights at zero. No weight is
   wrong here — the model is silent, and this is the case that shows it.
2. **Fix the instrument.** The compile golden measures `kanso check
   lib/json`, which compiles lib/json's TEST file and so its test-only
   dependencies. Charging test cost to the library's compile golden may be
   the real defect. It already carried one test-only dependency (`std/text`),
   so this is a difference of degree.

**RECOMMENDATION: 1.** It is the smaller change and honest about why. Option
2 moves the objective itself and deserves its own sitting rather than riding
a doctrine fix. Not recommended either way: finding a compensating
improvement in the same pull request to hold the number flat, which is
gaming the index.

## Open, not blocking

### Welfare cannot see what the compiler costs to run

**Cited: the weights and satiations were argued once with reasons — archive
2026-08-01 "the four weights, decided from evidence" and archive 2026-07-26 "a
second of compile time is not a second of runtime". Nothing since revisits
them, and no entry anywhere proposes a compile-traffic term.**

Four merges on 2026-08-24 took `compile_allocs` from 148,073 to 87,290 and
the front end's retired instructions down about a quarter, and the welfare
score did not move: 84.87 before, 84.87 after, floor untouched.

That is not a bug in the script. Its model of compile cost is four counters,
and every one of them is flat across those changes:

    compile_speed_counters = ["front_end_rounds" "front_end_visits" "emitted_lines"]
    compile_memory_counters = ["compile_peak_bytes"]

    front_end_rounds        40 -> 40
    front_end_visits    17,786 -> 17,786
    emitted_lines        1,534 -> 1,534
    compile_peak_bytes 871,649 -> 871,649   (the golden welfare reads)

Rounds and visits count the work the compiler decided to do. Emitted lines
count what it wrote. Peak counts what it held. None of them counts what it
does — the allocator traffic, or the instructions retired getting there. What
a model leaves out it weights at zero, and the tree now has two veins,
`bench/compile_allocs_golden.txt` and `bench/compile_instructions_golden.txt`,
watching a dimension the objective scores at nothing.

The question is whether welfare should carry a compile-traffic term, and at
what weight and satiation.

**RECOMMENDATION: take the smaller variant — leave the weights alone and say
in the script's own prose that compile traffic is deliberately outside the
model.** The score already has four compile terms against three runtime
dimensions, and adding a fifth to score the work of the week it was proposed
is how an objective gets fitted to its history. A sentence in the script
means the next person to find a silent 26% reads that it was a choice. The
two veins already catch a deletion, which is the job the goldens do and
welfare does not.

One tangle worth naming: `compile_peak_bytes` DID move, 876,930 to 864,274.
Welfare did not see it because welfare reads the golden rather than the
compiler, and the band let the golden sit unmoved. The no-tolerance-bands
gavel of 2026-08-24 settles that half; once the gate asserts equality this
term reports live movement again, with no change here. Filed 2026-08-24.

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

  **RECOMMENDATION: the existing err arm IS annotate's surface, and
  `rescue` gets the one new word.** Two of the three combinators already
  have a spelling that programs use and the book teaches; minting names
  for them would be a migration that buys a symmetry nobody asked for.
  Rescue is the licensed door and the only one a reader needs to
  recognise on sight, so it earns a keyword of its own.

- **Construction enforcement**: reason building module-private is
  stated by the doctrine, unnecessary for soundness now that provenance
  is computed, and unenforced.

  **RECOMMENDATION: strike it.** It was the proxy for provenance, and
  provenance is computed (archive 2026-07-28, "Clay: build it
  correctly"). A rule that buys nothing and costs a fleet migration is
  doctrine the code has outgrown.

- Downstream of the spelling, not a separate question: the arm-based
  advisory migrates onto whatever surface is chosen. Implementation.

### An arm cannot see an own-origin err — semantics, or an advisory?

**Cited: derived by the committee in design/testing.md (2026-08-19), with a
veto window offered to Clay that never closed. The search found no ruling
either way, and the code answers a third way: src/provenance.rs raises
`advisory[license]` and nothing refuses.**

Gavel 24's clause 1 says no arm may match an own-origin err. The July record
also seeds every pub dispatch group's receivable set with its own hako, and
the committee found the two cannot both stand: the seeding would statically
refuse every pub bare-err arm, `when_failed`'s included, and with it the
generic foreign rescuers Clay blessed by name.

The derivation was that clause 1 is dispatch semantics rather than a static
check alone — at match time, an err whose origin hako equals the arm's hako
does not match, infectiousness carries it onward, and the doctrine executes
itself. The pub seed retires; the static refusal stays for what provenance
proves without self-seeding.

None of that is built. Today the case is an advisory, so a program that
rescues its own failure compiles and runs.

**RECOMMENDATION: ratify the derivation and build it.** An advisory is the
one shape the err thesis cannot afford, because the whole claim is that the
discipline is structural rather than advisory, and try/catch's failure is
named in the gavel as exactly this. Match-time skipping also keeps
`when_failed` working with no exemption written for it.

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
