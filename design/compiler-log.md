# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

## 2026-08-25 — two parameter forms name their type without the fixpoint

The increment before this refused a field read when the type was plain to read
off a statement: a construction written where it stands, or a local bound to
one. It said a parameter answers nothing, and left that to the whole-program
set the census sized.

Two parameter forms are not a guess and need none of it.

An **annotation** says the type outright — `fn shown m:money` — and a
**constructor pattern's as-binding** is whatever dispatch matched, since
`fn sized r@(money c)` reached that arm because the value IS a money. Both
seed the same per-body map the locals use, before the walk starts.

    fn shown m:money        m.text    money has no field text
    fn sized r@(money c)    r.width   money has no field width

A parameter that is neither still answers nothing, which is the honest state
until the fixpoint carries a type.

`Pattern::Annotated`'s `ty` may name a typeset or a subtype, and those are not
in the plain-record table, so the lookup misses and the check stays quiet. The
conservative direction falls out of the table rather than needing a rule.

### It costs no memory and 40,430 instructions

`compile_allocs` is 64,950 either side — the same figure the previous increment
landed on. The seeding reuses the map that was already being built per function
body, so there is no new allocation anywhere. Rounds, visits and peak are
identical too.

`compile_instructions` is not: 59,732,726 to **59,773,156**, a rise of 40,430,
or 0.068%. `check_merged` carries 31,147 of it and `memcmp` another 7,029,
which is the parameter loop and the type names it compares.

Worth saying plainly, because the first draft of this entry said the increment
"costs nothing" on the strength of the counters this box can measure. Four of
them did not move and the fifth is host-pinned, so a container cannot see it
and CI can. A claim about cost that only covers the measurable half is the
shape of the mistake made twice already today, once about stale benchmark
binaries and once about a golden figure taken before twelve merges.

### Watched red

`tests/golden/errors/field_of_an_annotated_parameter.kso` covers both forms,
each reading a DIFFERENT absent field so the golden tells them apart. Without
the seeding it prints the old `error[runtime]` and exits 0, where the error
corpus requires exit 2.

## 2026-08-25 — the field fence types none of the fleet, and the first census that said otherwise was invalid

Two increments of gavel 1b's read half shipped today, and before building the
third — the whole-program fixpoint, which touches every arm of `eval_expr` —
the question worth answering is how many field reads it would newly answer.

**142 declared field reads across the fleet. The fence types zero of them.**

    lib/regexp                       reads=100  typed=0
    scripts/ratchet                  reads= 24  typed=0
    scripts/welfare                  reads= 11  typed=0
    bench/pendbench                  reads=  4  typed=0
    scripts/perf_record              reads=  3  typed=0
    lib/json, lib/list, lib/text     reads=  0

So the five shapes that answer — a construction where it stands, a local bound
to one, an annotated parameter, a constructor pattern's as-binding, and a field
no type declares — cover the shape a MISTAKE takes and not the shape the IDIOM
takes. Real code reads a field off a bare parameter, which is the one case only
the fixpoint can see.

That is worth stating plainly against any impression that the fence is mostly
built. What is built catches the beginner's error, where a value is constructed
and then misread three lines later. The fixpoint is not an increment on top of
that; it is where the fence starts earning its keep on programs people wrote.

### The first census was invalid, for a reason already in this log

The first attempt walked `Expr::Field` over merged programs and reported fifty
field reads in the whole tree, four of them answerable. A grep of the sources
finds hundreds. The AST census was measuring nothing.

The reason is recorded in the 2026-08-19 withdrawal of the per-dependency
`check_merged` removal: *"the rewriting passes run inside the dependency's own
compile, before it returns: by the time the entry merges, a field read has been
desugared into a shape the check that looks for it can no longer see."* A
merged program is exactly the wrong place to count field reads, and that is why
the per-dependency pass is not redundant.

The instrument that works is the check itself, which runs per dependency where
the shape still exists. Counting there gives the 142 above, and it was verified
against the two error goldens first — they report typed, as they must, so a
zero elsewhere is a fact rather than a broken probe.

This is the second time today a measurement was taken under one configuration
and read as evidence about another. The first was a set of counter gates
reading benchmark binaries built before the change. Both were caught by
cross-checking against something independent rather than by noticing at the
time.

### The same reads answer to a cheaper instrument

A second probe asked what a rule with no inference in it would win of the same
ground. Take the set of fields read off one value in one body. If no declared
record holds all of them together, the program is wrong whatever that value's
type turns out to be.

Across the same eight roots:

    bases with at least one field read       60
    bases reading two or more fields         30
      ... pinning exactly one declared type  30
      ... pinning none, or more than one      0

Every base that reads two fields determines its type completely. Two of the
thirty match a pair of names — `row` and `ratchet/row` — which is one
declaration under a qualified and an unqualified spelling rather than two
types.

The refusal's reach was measured by substitution: for every base, and every
field name declared anywhere in that root that the base does not already read,
does the rule refuse the base with that name added? It refuses 1,992 of 2,123,
or 93.8%. Every substituted name is declared on some type, so the fence shipped
in #1029 — which refuses a field no type declares at all — says nothing about
any of them, and the 93.8% is ground it does not hold.

Soundness needs the reads to happen together, which costs almost nothing here.
A second gather kept only the reads that certainly execute: never inside a
lambda, which may never be called; never in a deferred arm of `if`; never under
a name a nested binder has shadowed. That gather loses one base of the thirty
and none of the refusals.

This does not replace the fixpoint. A base with a single field read is out of
reach, and so is the 6.2% where some other record happens to hold the confused
pair. What it changes is the price of the 142: a walk of each body against the
type table, rather than a second value threaded through every arm of
`eval_expr`.

## 2026-08-25 — a value read for fields no one record declares is refused, and no inference is behind it

The measurement in the entry above said the fence types none of the fleet and
that a rule with no inference in it reaches 93.8% of the same confusions. This
is that rule.

**The argument.** `Expr::Field` in `eval.rs` reads a field off `Value::Record`
and errors on everything else, and a subtype reads through to its base record,
so a value a body reads for two fields is one record and a record this program
declares. When no declaration holds every field read off a value, those reads
cannot all be of the same value, and the program is wrong whatever type it
would have had. Nothing is inferred; the type table and one walk of the body
answer it.

    fn width m
      m.to - m.wanted

    error[name]: `m` is read for `to` and `wanted`, and no record type has both

`to` is declared on `span` and `wanted` on `node`, so the fence shipped in
#1029 — which asks only whether any record declares the name — passes both
reads through and the program dies at run time.

**Which reads may be pooled.** Only the ones that run whenever the body runs.
Three shapes defer: a lambda, which may never be called; the two arms of an
`if` or a guard, of which one is taken; and the right operand of `and` or `or`,
which the left may already have decided. A binding of the same name ends the
run, because the reads after it are of a different value, and a `build` block
runs its statements so a binding inside one ends a run too. `set p.x = v` is
counted as a read, since a build block runs its sets and the field must exist.

The restriction is nearly free. Measured over the fleet before building this:
the certain-only gather loses one base of the thirty that read two or more
fields, and none of the refusals.

**The odd one out.** When dropping exactly one field leaves the rest sitting on
a record together, the diagnostic points at that read, because it is the one to
change. `n.kind + n.from + n.to` points at `kind`, since `from` and `to` are a
`span`.

**Cost, and a second walk that CI caught.** `compile_allocs` on lib/json is
64,950 before and after, byte for byte, and rounds and visits are unmoved. The
first draft cost 157 allocations, all of them a set built per binding statement
whether or not anything was open; a walk that carries the open bases and asks
the pattern about each one allocates nothing until a body actually reads a
field, and lib/json reads none.

Allocations were the wrong dimension to stop at. The draft was a SECOND
traversal of every body beside the one the #1029 fence already does, and the
vein built for exactly this reported it: `compile_instructions` rose 59,773,156
to 60,090,679 on the runner, **+317,523 for a module with no field reads in
it**. Nothing else moved, because a walk that finds nothing allocates nothing
and decides nothing.

The fix is one walk. The two questions a field read raises — does this field
exist at all, and can the fields read off this value belong to one record —
differ only in where they may look: the first is asked wherever the read
appears, the second only where the read certainly runs. So the walk carries
`certain` and stops recording under the shapes that defer, rather than
traversing twice.

**The runner reads 59,717,892 against 59,773,156, a fall of 55,264.** So the
refusal lands with the front end doing LESS work than before it existed: the
gather rides a walk that was already there, and folding the two tightened the
one that remains — `walk_children` 1,703,108 to 1,631,080, its recursive twin
965,191 to 940,511, `check_merged` 1,793,255 to 1,771,785. The golden takes the
new figure and this is the sentence beside it. `compile_peak_bytes` held at
864,300 on the same run, and `compile_instructions` is not a welfare term, so
the floor does not move.

The container host agrees on direction and not on magnitude, which is why the
row is host-pinned: 60,404,144 against 60,625,645 there, each repeated to the
instruction.

### Watched red

`tests/golden/errors/fields_that_no_one_record_declares.kso` on the compiler as
it stood exits **1** with `error[runtime]: `span` has no field `wanted``. With
the check it exits 2 with two diagnostics, one for the two-field message and
one for the three-field message. That is the hole this closes: a language that
refuses before anything runs was deferring this one to run time.

### Watched red three more times, for the quiet cases

A refusal that fires is half the behaviour; the other half is staying silent
where a program is correct, and that half is where a false refusal would live.
`tests/golden/micro/a_field_read_the_body_may_skip_is_not_pooled.kso` is three
correct programs the rule would refuse if it pooled too much, and each clause
was watched red by breaking its own guard:

    the two arms of an `if` pooled     `x` is read for `from` and `kind`
    a lambda body pooled with outside  `m` is read for `kind`, `from` and `to`
    a rebinding did not end the run    `p` is read for `from`, `to`, `kind`
                                       and `wanted`

Each of the three is a program that runs and prints, so the fixture is a micro
golden rather than an error golden — it pins the output, and any of the three
guards going away turns the print into a refusal.

The rebinding case took a second attempt to write at all. A body cannot rebind
a name after a bare effect line — the grammar puts every binding before every
effect — so the rebinding lives in a constant that returns a string and `play`
prints it. That is worth knowing: the shape this guard exists for cannot be
written in an effectful body in the first place.

### What it does not reach

A value read for exactly one field, where that one name is the wrong one. The
93.8% above is measured by ADDING a confused name to what a value is already
read for, which is why single-read bases are mostly covered: a second, wrong
read makes a two-set and the two-set is usually homeless. A *replacement* on a
base read once leaves a one-set and nothing to compare. Thirty of the sixty
bases are read for exactly one field, carrying 46 of the 142 reads, and that is
the fixpoint's real prize — along with the 6.2% where some other record happens
to hold the confused pair.

### What the fixpoint's room costs, measured before building it

`Set` in src/infer.rs is a `u16` with fourteen kind bits used, so bits 14 and
15 are all that is spare and the fleet declares 143 record types. Carrying a
type identity in the lattice means widening the word. That price can be read
without spending any of it: change `pub type Set = u16` to `u32`, use none of
the new bits, and measure.

    compile_allocs         64,950 -> 64,950
    front_end_rounds           40 -> 40
    front_end_visits       17,786 -> 17,786
    compile_peak_bytes    864,300 -> 866,908      +2,608
    welfare                 84.89 -> 84.88

So the room is nearly free, and the fixpoint's cost is entirely in the
propagation rather than in the representation. The figures are the container
host's, and only the peak is host-pinned, so the runner's own would need
re-reading before any of this is banked.

That also corrects the shape of the cost. The note left on this task said
`eval_expr` returns a `Set`, so carrying a record identity means threading a
SECOND value through every arm. It does not, if the identity rides in the high
bits of the widened word: an arm that passes a set through unchanged needs no
edit, and a kind test masks against a low-bit constant and still works.

What a packed identity does break is the JOIN, because two identities cannot be
bitwise-ORed into a third. That count comes off the compiler rather than a
grep — a `Set` newtype that still masks and compares but implements no `BitOr`,
which makes rustc report exactly the sites a packed design must rewrite:

    Set | Set                             65
    |= on a Set                           12
    the same through a trait bound         7
    integer |= Set                         2
                                      ------
    join sites                            86

    expected `Set`, found integer         90    the empty set written as `0`
    borrow and iterator shapes            11
                                      ------
    total errors                         187

**86 join sites** across infer.rs, codegen.rs, beat.rs, check.rs, dispatch.rs
and provenance.rs, each of them a place that would have to decide what joining
`span` with `node` means — a record whose identity is unknown, which is a
lattice question rather than a bitwise one. The other hundred are mechanical:
a real implementation names the empty set instead of writing `0`.

Against the interner's 365 for one AST field, 86 is a different order of thing,
and the prize is a refusal rather than speed. This is priced, not declined, and
not yet taken: 46 reads and 6.2% of confusions, for 86 lattice decisions and
0.01 welfare of room.

## 2026-08-25 — the interned symbol is DECLINED, and the churn was counted by making the name opaque

The economics have been settled since the "how big an interner would have to
be" entry: around 400 distinct names per program, 622 across the build, against
roughly 11,000 `String::clone`s. What was left open was the churn, and the note
left for whoever started said to count the print sites in the back ends and the
interpreter first.

A grep would have given a number nobody could check. The instrument that
answers it is the compiler. Make the name type opaque — a newtype in a private
module with a constructor and no way to read the text — change one AST field to
it, and every site rustc reports is a site an interner's table would have to
reach.

    pub Expr::Ident(String, Span)  ->  Expr::Ident(Name, Span)

**365 errors in the library, 366 including the test target.** One field.

    codegen  63     linear   36     demand    9     advisory  7
    check    62     infer    28     parser    8     provenance 4
    lib      42     trmc      9     inline    7     dispatch  1
    beat     42     escape    9     eval      5     ast       1

That is a lower bound twice over. Errors cascade, so fixing this wave uncovers
sites the first one hid; and `ast.rs` holds 28 other String-typed name fields
that a real move would have to convert too. The back-end and interpreter half
the note asked for is codegen's 63 and eval's 5, with `wasm_backend.rs`
reporting none — it works off a lowering that has already resolved names.

### What the objective can see

Welfare's compile terms are `front_end_rounds`, `front_end_visits`,
`emitted_lines` and `compile_peak_bytes`. An interner moves none of the first
three: it changes what the compiler allocates, not what it decides. The one
term it does touch is the one it would likely make worse, because a table holds
every name for the whole compile where a String dies with its scope.

So the index would read the same or lower for 365-plus sites of churn.

That is not the whole argument, and saying it is would be dishonest about the
model. Wall time is absent from welfare deliberately, and what a model leaves
out it weights at zero — the borrow-the-names work on 2026-08-24 took a quarter
off a pass's time with every gate in the tree reporting nothing, which is
exactly the dimension an interner is aimed at. `compile_allocs` is pinned in a
vein of its own for that reason and would fall.

### The ruling

DECLINED as a whole-AST move. The win is real and invisible to the objective;
the cost is 365 sites for one field of twenty-nine, on a front end that already
finishes lib/json in about six milliseconds. Under the satiation the project
chose for compile cost — 0.5, where successive doublings are worth 4.3, 2.9,
1.7 and 0.9 points — a compiler that is already imperceptible has little left
to win.

What survives is narrower and was already isolated by the churn analysis. Two
questions do genuine text work: "is this qualified?" (21 `contains('/')` sites)
and "make a qualified name" (26 `format!` sites, 23 of them in lib.rs's
qualification machinery). A name carrying its module and base answers both
structurally, and it is a fraction of the conversion. Whoever returns to this
should measure that, not the interner.
## 2026-08-25 — gavel: welfare measures what compiling costs, not what it counts

Clay ruled the sweep's first entry, and against its recommendation. Told
that welfare's compile-speed terms are front_end_rounds, front_end_visits
and emitted_lines — counts of what the compiler decided to do rather
than what doing it cost — his words: "then you have a MASSIVE deficiency
in your welfare metric. my god." The recommendation was to leave the
model alone and note the gap in prose; the ruling is the opposite.

The ruling: welfare's compile-cost model measures actual cost. The terms
that stand in for compile speed become the measured ones — instructions
retired and allocator traffic, the two deterministic host-pinned veins
that already exist (bench/compile_instructions_golden.txt and
bench/compile_allocs_golden.txt) — so that making the compiler genuinely
faster or leaner always moves the score, and a 26% front-end improvement
can never again land silent. compile_peak_bytes stays as the memory
term; the no-tolerance-bands gavel already un-stales its reading. The
proxy counters stay pinned in their goldens as regression tripwires;
they stop being scored.

Weights and satiation for the measured terms are the holder's to price
from evidence, the way the 2026-08-01 weights session did — that is
implementation under the ledger's own charter, and it does not come
back here. The floor re-ratchets from the rescored model in the same
change, recorded as a model correction.

The entry leaves the ledger with this commit. Fifteen of the sweep's
sixteen remain with Clay.

## 2026-08-25 — gavel: an arm never sees its own err, and it was never advisory

Clay ruled the sweep's own-origin entry, emphatically: "it has certainly
never been 'advisory' whatever the hell that means." The doctrine — no
arm may match an err born in its own hako — was ruled with gavel 24 and
was never optional; the warning-only enforcement that ships today
(src/provenance.rs raising advisory[license] while the program compiles
and runs) is a defect against the ruling, not a design state.

The ruling ratifies the committee's derivation from design/testing.md:
clause 1 is dispatch semantics. At match time, an err whose origin hako
equals the arm's hako does not match the arm — infectiousness carries it
onward exactly as if the arm were absent. The pub self-seed retires; the
static refusal stays for what provenance proves without it; when_failed
and the blessed generic foreign rescuers keep working with no exemption
written for them, because a foreign err still matches. Build it now:
the advisory becomes the semantics, and a program that rescues its own
failure stops being expressible.

The entry leaves the ledger with this commit. Fourteen of the sweep's
sixteen remain.

## 2026-08-25 — gavel: the floor is absolute against refactorings, permeable to the language

Clay ruled the blocking entry, and with a sharper principle than either
framing on offer: "welfare can absolutely fall if you are considering
language functionality as part of the welfare score. of course the more
correct way of expressing this is just that welfare can fall in order to
implement a language feature. it can't fall from a refactoring so to
speak."

The ruling: the welfare floor is absolute against behavior-preserving
change — a refactoring that costs welfare is worse code and stays
refused, no exceptions. A change to what the language IS — a feature, a
doctrine-compelled migration — may move the floor down, with the fall
recorded and attributed to the change that spent it, because the thing
being measured changed. Hunting a compensating optimization in the same
pull request to hold the number flat stays forbidden as gaming the
index.

Applied: kanso#1034 implements gavels 1b and 24, so it merges and the
floor moves to 84.79, recorded as a language-change fall. The instrument
question the entry raised — the library's compile golden charging the
test file's dependencies to the library — folds into the already-ruled
welfare rebuild ("welfare measures what compiling costs, not what it
counts", this log, 2026-08-25): measure the thing the golden claims to
measure. It does not return to the ledger as its own entry.

The entry leaves the ledger with this commit.

## 2026-08-25 — the floor gavel, stated precisely by Clay

Clay refined the ruling within the hour, and the refinement replaces the
word "refactoring" with what he meant: "technically they aren't
refactorings necessarily — they can actually have real impacts on
performance both in terms of compilation speed and resources and actual
run time performance. but those are things that can be improved
independently, where if one gets sufficiently better another can get a
little bit worse as long as the overall welfare goes up. but you cannot
just reject an actual language feature in the name of welfare score."

Two clauses, precisely:

- **Non-feature work trades freely inside the index.** A change that is
  not a language feature — performance work, implementation work — may
  worsen any individual dimension (compile speed, compile memory, run
  time) if another improves enough that the AGGREGATE welfare rises.
  The floor binds the composite, never a single term. This is what the
  weighted index is for.
- **A language feature is never hostage to the score.** A feature or
  doctrine-compelled change cannot be rejected in the name of welfare.
  When it costs, the floor moves down with the fall recorded and
  attributed. The index measures the implementation; it does not govern
  the language.

The forbidden move stands: packaging a compensating optimization into a
feature's own pull request to hide its cost is index-gaming either way —
the feature's fall and the optimization's gain each deserve their own
attributed record.
## 2026-08-26 — the error corpus had no ratchet row, and now has one

The ratchet's rule is one row per CI job, and `specs (unit, golden,
differential)` has carried one since the file existed. Which command that row
actually runs is a different question from whether the rule is satisfied, and
it was worth asking while claiming ratchet cover for a new error golden.

Four rows run `cargo test --test golden`. Two point at
`micro_corpus_agrees_across_engines`, one at
`micro_corpus_survives_a_release_build`, one at
`mem_corpus_pins_native_allocator_counters`. The error corpus — 164 fixtures
pinning byte for byte every diagnostic the language emits — had none. Nothing
in this tree had ever watched it fail.

That is the file's own stated reason for existing, in its header: a gate nobody
has watched fail has no evidence it works, and five checks went green in one
day while checking nothing.

**The row.** `error_corpus`, gate `cargo test --release --test golden
error_corpus`, job `specs (unit, golden, differential)`, mutation
`a_diagnostic_stops_being_raised`, setup `release` — the same shape as
`thunk_walk`, which reads a golden through a release build.

**The mutation** is the regression the corpus is for, which is a check that
quietly stops firing rather than a golden that drifts. The none-in-a-list walk
still runs and still visits every list, and finds nothing in any of them:

    for item in items.iter().take(0).filter(|i| is_none_lit(i))

It compiles without a warning, so no other gate in the tree objects to it, and
the corpus is left to notice on its own.

### Watched red before it was wired

The mutation was applied by hand first, `cargo build --release` came back
clean, and `cargo test --release --test golden error_corpus` FAILED. Only then
was it reverted and the row written. The row is therefore written against a
mutation already known to work, rather than a mutation guessed at to fit a row.

The filter is proven non-empty by the same run. A gate that matches no test
passes forever, which is the failure this file is named after, and
`cargo test --release --test golden error_corpus` reported "0 passed; 1 failed;
9 filtered out" — exactly one test, and it went red.

### And the row that had been proving nothing since 2026-08-24

Running `prove` for the new row turned up a second thing, which is the better
find of the two. It reported **1 rows proved nothing**:

    BROKE cost goldens — the pending-cell shape erased from a strictness
    change: the mutation would not apply

`pending_cells_proven_strict` anchors on an exact line of src/demand.rs:

    self.lazy_binds.contains(&(fn_name.to_string(), arity, stmt_index))

#1015 made the demand pass borrow the names its keys are made of, so that line
now reads `&(fn_name, arity, stmt_index)`. The mutation's `awk` found no match,
exited 3, and the row has been unable to introduce its defect ever since.

The `pend_counters` row was therefore green for a reason that had nothing to do
with the gate working.

The anchor now matches the borrowed form, and the row was watched red by hand
to be sure the fix restores the substance rather than the sed: applied, built,
and `scripts/gates/pend_counters.sh` exits 1 with the lazy tier gone —

    thunk_allocs   200 -> 0
    thunk_forces   100 -> 0
    thunk_evals    100 -> 0

which is precisely what the mutation's own comment says it should do. Reverted,
rebuilt, gate green again.

### The machinery worked. Nobody answered it.

The first draft of this entry said nothing watched the mutations, and that was
wrong in a way worth correcting rather than quietly fixing. `prove` exits 1 on
a row that proves nothing — `told bad false` writes the count and calls
`os/exit 1` — and `.github/workflows/ratchet.yml` runs it on a schedule every
morning at 09:00 UTC.

So the timeline is not a blind spot:

    2026-08-24 22:52 UTC   #1015 lands, dropping the .to_string()
    2026-08-25 09:27 UTC   nightly ratchet run 14 fires
    2026-08-25 09:37 UTC   run 14 FAILS, naming this exact mutation
    2026-08-26 00:50 UTC   still red, still unanswered

The ratchet caught it the very next morning and said so precisely — `BROKE
cost goldens … the mutation would not apply` — in the first red run that
workflow has had in fourteen. It was found today only because `prove` was run
by hand for an unrelated reason.

That reframes the lesson. The gap is not instrumentation, which did its job on
schedule; it is that a nightly nobody reads is a nightly that does not exist.
A per-PR signal gets answered because it blocks a merge, and a scheduled one
competes with whatever else the morning holds. Worth saying plainly because the
obvious fix — build more watching — is the wrong one here. What this needed was
for the red run to reach somebody.

### A count in a comment, drifting

`tests/errors_module.rs` opened by saying the corpus holds 161 fixtures. It
holds 164. Nothing checks that number and it goes stale every time somebody
adds a mistake to the corpus, so the sentence now says a fixture per mistake
and names no figure. Where a count carries weight it belongs in a golden, where
CI reads it.

## 2026-08-26 — the cheap half of the ratchet moves to where it gets answered

The entry above found `pending_cells_proven_strict` unable to apply since
#1015, recorded that the nightly had caught it and that nobody answered, and
said the fix — making a red nightly reach somebody — was a preference rather
than a defect to settle alone. That was the wrong shape to leave it in.

The two things `prove` does have different prices. **Applying a mutation costs
a sed. Proving it reddens its gate costs a build.** Only the second needs a
nightly. The failure that actually bit was the first kind, and the first kind
is cheap enough to run on every change.

So `cover` — the per-PR half — now creates one throwaway worktree of HEAD,
applies each mutation in turn, restores between rows, and refuses any that no
longer matches the source it patches. Twenty-nine mutations in **seven
seconds**, against a job that already pays a compile.

### Watched red, on the exact historical failure

Not a synthetic defect: the anchor was set back to the pre-#1015 spelling,
which is precisely what that commit invalidated.

    ratchet: 1 mutations no longer apply
      STALE cost goldens (deterministic ratchet, no clocks) — the pending-cell
            shape erased from the corpus by a strictness change

Exit 1. Restored, green again. #1015's own pull request would have gone red on
this, and its author would have fixed the anchor in the same change rather than
leaving a row proving nothing for two days.

### What it reads, and what it therefore does not

Like `prove`, this walks a worktree of HEAD rather than the working tree, so a
mutation edited but not yet committed is not what it checks. That is right for
CI, which checks out the commit, and it caught me out twice while building
this: the first watched-red stayed green because the break was unstaged, and
the reset that removed the break took the feature with it. The property is
worth stating rather than discovering — the ratchet answers for what a commit
carries, never for what is sitting unsaved beside it.

### What is still not built

Making a red NIGHTLY reach somebody. That remains a question about how the
owner wants to be told, and this change does not answer it — it only removes
the failure mode that had no business waiting for a nightly at all. A gate that
stays green under its mutation still costs a build to detect, still runs at
09:00, and still has no addressee.

## 2026-08-26 — the floor is permeable to the language, and now the trend gate knows it

Clay ruled on 2026-08-25 that the welfare floor is absolute against
refactorings and permeable to the language: a feature or a doctrine-compelled
change is never rejected for the score. It lands, the floor moves, and the fall
is recorded against the change that spent it. Compensating-optimization
bundling stays forbidden in both directions.

The ruling landed in the log yesterday and one gate had not heard it.

### The fixture is #1034

Obeying gavels 1b and 24 costs lib/json four counters and improves none of
them: `front_end_rounds` 40 -> 42, `front_end_visits` 17,786 -> 17,886,
`compile_peak_bytes` 864,300 -> 870,289, `compile_instructions` 59,717,892 ->
60,772,083. The welfare floor was moved by hand to 84.79 with the fall
attributed in `bench/welfare_floor.json`'s history, which is what the ruling
asks for. CI then said:

    FAIL  a pure regression: something got worse and nothing got
    better. that trade has no other side, and it is the one move
    the gate refuses outright.

A branch obeying a gavel could not merge. The gate was written before the
ruling and was right for what it knew — the pure regression it refuses is a
refactoring that quietly spends the score — but a language change spending the
score is exactly the case the ruling carves out, and the gate had no way to
tell the two apart.

### What tells them apart

The attribution itself. A pure regression now passes when the branch adds an
entry to the history array in `bench/welfare_floor.json`; without one it is
refused as before. That is the same posture the gate already takes toward the
compiler log — it checks that a sentence exists, not that the sentence is
right — and it puts the escape exactly where the ruling puts the record.

It is not a hole a refactoring can crawl through by accident. To use it a
change must move the floor and write down what it spent, in the ledger whose
whole readership is Clay, beside fifty-odd entries that say what each earlier
move bought. A refactoring that does that has not evaded the rule; it has
signed its name to breaking it.

The narrower reading — accept the fall only when the floor went DOWN — was
considered and dropped. A rise cannot reach this code at all: welfare reads the
same counters, so a branch that raised the number improved one of them, and
`better` is non-empty before the refusal is reached. The extra condition would
never fire, and a condition that cannot fire is a claim about the code that the
code does not make.

### Watched red, then green, on the same worsening

    compile_peak_bytes=999999999, no history entry     exit 1, refused
    compile_peak_bytes=999999999, history entry added  exit 0, attributed

Same counter, same direction, same magnitude; only the record differs. The
first of those is now a ratchet row — `a_counter_worsens_for_nothing` — because
the trend gate had none. It is a step inside the cost-goldens job, and the
ratchet's own coverage check is per JOB, so a step-level gate hides behind the
rows its neighbours carry. That is a general gap and this closes one instance
of it, not the gap.
## 2026-08-26 — json stops reading its own failure (written 2026-08-21, landed today)

The work below was done on 2026-08-21 and sat in a draft pull request until
today. It is filed at the tail rather than at its own date, because this log
is append-only and the entries between were written first.

Gavel 1b said only a type's owner constructs one; gavel 24 said your own
failures only bubble. Between them, `json/failure_position` and
`json/failure_reason` were arms inside lib/json matching an err lib/json
had raised, which is the thing the second rule forbids. They are gone.

The suite reaches the same facts from where the rule allows. `std/testing`
is foreign to json, so `when_failed` may hold json's err, and it hands the
bare reason record to a continuation:

    test_error_position =
      decoded = decode "[1, nope]"
      testing/when_failed decoded (r -> r.position == 5)

Watched red twice before it was believed, because one direction is not
enough here. With the position wrong the example fails, which is the
ordinary case. With the input CHANGED TO PARSE it also fails — `when_failed`
answers false on a value, so a test that expected a failure and got an
answer cannot pass quietly. That second failure is the one the old
projection could not produce: `failure_position` answered 0 for a
non-failure, and `0 == 5` is a perfectly good false that says nothing about
why.

`test_must_wraps_defect` needed the same door and a named local group, since
a lambda carries no arms — `a_defect?` rather than the `defect_reason` the
design note guessed at, because the naming rule wants the question mark on
anything answering only true or false. The note now says what shipped.

The example that demonstrated the old door demonstrates the new one, and it
is a better example for having two branches instead of one: a consumer
outside json writes an ordinary arm on `err r` and reads `r.position`,
because json's failure is foreign there. Destructuring the reason in the
pattern is refused — opacity, and rightly, since the record's shape does not
cross an import — so the arm names the err and the field read does the rest.

The book taught the rule this change contradicts. Chapter 04 said "no
function gets to turn one back into a value. not a helper, not a library,
not `main`", and its personified err said "nobody catches me". Both are
about who raised the err rather than about nobody, and both now say so.
Chapter 08's api panel showed the projection that no longer exists, its
suite panel showed sixteen tests where there are twenty, and both used
parentheses the grammar has since refused. Three surfaces, all moved.

## 2026-08-26 — the floor moves for the language, and the fall is attributed

Clay ruled the blocking entry: the welfare floor is absolute against work that
leaves the language alone, and permeable to work that changes it. A feature or
a doctrine-compelled migration is never rejected for the score — it lands, the
floor moves, and the fall is recorded against the change that spent it.

Applied here. #1034 implements gavels 1b and 24 — json may not match an err
json raised — and the score falls **84.89 to 84.79**.

**Where the cost is, isolated rather than guessed.** Not the deletion: the
library shrank, and `bench/emitted_golden.txt` records it, 11,595 emitted lines
to 11,585. The cost is the one line the test file gained. `kanso check
lib/json` compiles lib/json's TEST file, so `import "std/testing"` pulls a
whole module into the program the compile golden measures:

    front_end_rounds              40 -> 42
    front_end_visits          17,786 -> 17,886
    compile_peak_bytes       864,300 -> 870,289
    compile_allocs            64,950 -> 65,543
    compile_instructions  59,717,892 -> 60,772,083

Reverting `json.kso` alone and keeping the new test file gives 65,801
allocations and 42 rounds, so all of the rise is the test file and the deletion
claws a little back.

**One of those figures was first written down wrong, and the error names its
own cause.** The peak row went in at 870,263, which is the CONTAINER's reading;
the runner reads 870,289. The gap is twenty-six bytes, and twenty-six bytes is
exactly the container-runner divergence this golden's own header documents from
2026-08-24 (864,274 against 864,300). Two hosts, both deterministic, a constant
offset between them — and the row whose header says to read the runner's number
out of the job log had the container's pasted into it. The measured-on line
caught it on the next run, which is the second time that machinery has earned
its place before a bad paste rather than after one.

No compensating optimization was hunted for this pull request. The same ruling
forbids it in both directions: a feature's fall and an optimization's gain each
deserve their own attributed record, and bundling them hides both.

The instrument question this exposed — a library's compile golden charging its
test file's dependencies to the library — does not return to the ledger. It
folds into the already-ruled welfare rebuild, which is the next thing built.

## 2026-08-26 — welfare measures what compiling costs, and the goldens stop charging the library for its tests

Building the gavel of 2026-08-25. Compile speed was `front_end_rounds`,
`front_end_visits` and `emitted_lines` — how many times a fixpoint went round,
how many expressions a pass looked at, how many lines came out. All three are
the compiler's bookkeeping about itself, and the case against them is already
in the tree: on 2026-08-24 a change took `kanso check lib/json` from 90.9
million retired instructions to 67.2 million with every one of them
byte-identical. A quarter of the front end's work went away and the dimension
whose job is compile speed scored the change at zero.

It reads `compile_instructions` and `compile_allocs` now, both counted rather
than inferred.

### The proxies stay, as tripwires

They keep their goldens and still turn CI red when they move. A dropped
fixpoint round is news whatever the score thinks, and `emitted_lines` is the
only vein that watches what the compiler WROTE rather than what it allocated —
the decoder gained twenty per cent more calls over a fortnight with every
allocation counter identical. A tripwire and a term are different jobs, and
these three were only ever good at the first.

### The weights did not move, and that is the argument

0.28 and 0.5 stand. They were priced for the DIMENSION — what teams shipping
software pay for, with 45% of the people who stopped using rust naming compile
times among their reasons — and the dimension has not moved. A better
thermometer is not a change of mind about the temperature. Repricing here
would have smuggled a preference change in behind an instrument change, which
is the move the ledger exists to make visible.

### The instrument fix, folded in

`kanso check lib/json` compiles lib/json's TEST file, so every dependency the
suite imports enters the program the compile goldens measure. #1034 is how
that surfaced: moving json's assertions onto `std/testing` cost the LIBRARY's
golden two rounds, a hundred visits, 5,989 peak bytes and a million retired
instructions, none of it the library getting more expensive to compile.

`scripts/gates/library_box.sh` stages the library without its tests, at the
fixed path the instruction count already needed for its own reason, and all
three compile gates now read the same staged program rather than two of them
reading the checkout:

    compile_allocs           65,543 -> 62,110
    front_end_visits         17,886 -> 16,818
    front_end_rounds             42 -> 40
    compile_peak_bytes      870,289 -> 825,664
    compile_instructions 60,772,083 -> 56,848,763

Two of those the container and the runner agreed on to the digit, which is
worth recording because the host gate refuses the comparison rather than
making it: `compile_allocs` and `compile_peak_bytes` came out identical on
both. `compile_instructions` did not — 57,524,712 here against the runner's
56,848,763, a bit over one per cent — so the row and its welfare BASELINE both
carry the runner's figure. A baseline holding a number no CI host will ever
measure would have the term entering at a ratio against a fiction.

The visits fall is nine hundred more than #1034 added, because json_test.kso
was always in the measured program — #1034 only made the charge large enough
to notice.

### The floor is re-set, not lowered

84.79 to 84.14. The precedent is the 66.6 entry, where run speed stopped being
three allocation counters: a term with no baseline is filled from the present
and enters where its dimension already stands, so compile speed gives up the
credit its proxies had banked. Nothing got slower. Scores either side of this
entry are not comparable, and the entry says so in the file rather than only
here.

Of the 0.79 the swap cost on the old goldens, the instrument fix gave 0.14
back before the floor was set, which is the honest order: fix what is measured
first, then read the number.
## 2026-08-26 — a merged err came out of a hop nested on native and flat everywhere else

Three failures answer three reasons however they were grouped. That is the
documented rule, written in a comment directly above the struct that breaks it.

    merged = boom 1 + boom 2
    three  = through merged + boom 3

    interpreter   ["e1" "e2" "e3"]
    native      [["e1" "e2"] "e3"]

`through` has no err arm, so the middle failure only hops. `k_hop` rebuilds the
err box, and it set four of the five fields — `merged` was left to whatever the
arena had at that address. `k_alloc` bumps a pointer and does not zero, so the
flag read as a fresh page's zero, the reason list stopped being a list OF
reasons, and the next merge nested it instead of folding it.

### Four constructors, three of them incomplete

Reading the rest of the family found the same shape three more times:

    k_err          reason origin hops cause merged      complete
    k_b_wrap_err   reason origin hops cause             merged unset
    k_hop          reason origin hops cause             merged unset
    k_deep_copy    reason origin hops                   cause AND merged unset

There is now one constructor, `k_err_box_new`, taking all five. The
evacuation copy still allocates through the copy arena rather than the main
one, so it assigns the two it was missing directly. A field added to `KErrBox`
tomorrow can no longer be forgotten by three call sites at once.

### What is proved and what is not

The hop path has a reproduction, in `tests/golden/micro`, and it was watched
red first: with the `merged` copy taken back out, native answers 2 where the
interpreter answers 3. It reads the count through `std/testing`'s
`when_failed`, which is foreign to the sample and is therefore the licensed
party that separates the reason from the failure.

The other three are fixed by reading. `k_b_wrap_err`'s omission needs the
arena to hand back non-zero bytes at that offset to bite, which is not a thing
a fixture can arrange on demand. The evacuation copy's dropped `cause` should
lose a "caused by" line from the endpoint report, and the obvious shape — a
qualified call returning a wrapped err — does not evacuate, so no reproduction
exists for it yet. That gap is filed rather than papered over: the fix is
right by inspection, and inspection is what this entry can claim for it.

### Only native

`wasm_rt`'s `rt_err_hop` calls the interpreter's own `hop`, which copies
`merged` like every other field, so the browser engine was never wrong. The
divergence was one engine against two, which is the shape the differential law
exists to surface — and it surfaced only because somebody went looking, since
no fixture in the corpus merged a failure and then hopped it.

### What the fix costs, to the byte

Every emitted binary grows **48 bytes** — eight of eight, the same number each
time, which is what a fix that adds one store to a constructor and two to a
copy should look like.

`compile_instructions` also moves, and the way it moves is worth recording.
`src/runtime.c` is `include_str!`'d into the compiler, so a longer runtime
lengthens a static and shifts the code and data around it; `kanso check
lib/json` never reads that string. The number is therefore pure layout, and it
behaves like layout. Three measurements of one diff:

    runner, unstaged library    60,772,083 -> 60,772,747     +664
    runner, staged library      56,848,763 -> 56,849,156     +393
    container, staged library   57,524,712 -> 57,517,949   -6,763

Same edit, opposite signs, an order of magnitude between the two hosts that
agree on direction.

That is not a reason to distrust the row — it is exact for the host and the
program its header names, and both figures above are exact for theirs. It is a
reason to say plainly what a move of this size means when the diff is bytes of
embedded text: nothing about the front end's work, and the row is regenerated
because it is exact, not because the compiler got slower or faster.

## 2026-08-26 — the trend gate could not see two of the four compile veins

The gate that exists to refuse silent counter movement read
`bench/compile_golden.txt` and `bench/compile_memory_golden.txt` and neither of
the two MEASURED compile veins. So `compile_instructions` and `compile_allocs`
could move by any amount and the listing said nothing.

That is not hypothetical. It happened twice today. #1034's instruction rise of
1,054,191 never reached the gate, and #1040's runtime edit moved the same row
three separate times with the gate silent each time. Only the per-golden exact
diff caught them, which is a different job: the diff says a row moved, the
listing says WHICH WAY every counter went and refuses a change where everything
went the wrong way.

The blindness mattered more from 2026-08-26, because those two veins became
welfare's compile-speed terms that morning. The gate meant to watch the score's
inputs could not see half of them.

### Measured, not argued

Same branch, same worsening — `compile_instructions` forced to 99,999,999, a
rise of forty-three million:

    the gate as it stands on main   exit 0, no output at all
    the gate with the two veins in  exit 1, names the counter, refuses

### The ratchet row moves to the vein that was blind

`a_counter_worsens_for_nothing` used to worsen `compile_peak_bytes`. It worsens
`compile_instructions` now. Each vein already has its own exact-diff gate with
its own row; what this row proves is that a vein reaches the LISTING, and
pointing it at the counter that was invisible is what keeps it from going
invisible again. Proved red in a worktree of HEAD, not only in the working
tree.

## 2026-08-26 — three differential sweeps were covered by their neighbours

Searched the log and `log/compiler-log-archive.md` for `render_differential`,
`behaviour_differential` and `dispatch_differential`: each is recorded arriving,
and none of the three is recorded gaining a row.

The ratchet's `cover` check asks whether every JOB in ci.yml carries a mutation.
The diagnostics differential job runs nine sweeps as nine steps, and two rows —
`diagnostics` and `accumulator` — satisfied the check for all nine. So seven
sweeps could have been deleted, or quietly reduced to nothing, and `cover`
would still have said the job was proven.

That is the same blindness the file was written to refuse, one level down. A
gate nobody has watched fail has no evidence it works, and a step whose row
belongs to its neighbour has never been watched at all.

### Three rows, and what each was watched doing

Each mutation was applied in a worktree, built, and its own sweep run. The
sweep named the defect in its own words before it was restored:

    native_renders_a_float_wider          9 of 68 values disagree
      integral floats printed with two decimals in k_render only, so 1.0
      reads as 1.00 on native. Reported as `0.0`, `1.0`, `-1.0`, `100.0`,
      `1000000.0`, `-0.0`, `2.0 * 3.0`, and inside a list and a record.

    native_rounds_a_half_to_even          2 of 66 calls disagree
      llround to llrint, so `math/round 0.5` answers 0 against the oracle's
      1 and `math/round 2.5` answers 2 against 3. Nothing complains, no
      counter moves, and no golden prints a half.

    a_literal_arm_ranks_below_a_wildcard  22 of 22 cases wrong
      the emitter's arm ladder ranks a literal below a bare binder, so the
      first case the sweep names — "a literal beats a later wildcard" — takes
      the wildcard on native and the literal on the oracle.

All three were green again on restore.

### One mutation was replaced because it went red for the wrong reason

The first attempt at the behaviour row broke `text/slice`'s ascii fast path by
one character. The sweep did go red, and reported `the probes do not run:
neither engine printed 1 for the sample` — its own canary, which trims a
string and so slices one. A gate that goes red because the harness stopped
working is not a gate that caught the defect, and a row credited for it would
be proving the canary. `math/round` was chosen instead because nothing in the
compiler's own path calls it.

### The remaining six steps

`trmc_differential` has the `accumulator` row and `diagnostic_differential` has
`diagnostics`. That leaves `numeric_differential`, `effects_differential`,
`module_differential` and `diagnostic_coverage` rowed only by their neighbours.
They are the same debt this entry pays down, and they are OPEN.

## 2026-08-26 — the trend gate could not read three runtime veins, one of them at all

Searched this file and `log/compiler-log-archive.md` for `instructions_golden`,
`text_golden` and `emitted_golden` beside the trend gate: the entry earlier
today added the two MEASURED COMPILE veins and named no runtime one. This is
that entry's argument applied one vein down, and the same fixture found it.

`bench_goldens` held eight files. It did not hold
`bench/instructions_golden.txt`, `bench/text_golden.txt` or
`bench/emitted_golden.txt` — so the listing that exists to say WHICH WAY every
counter went could not see what a program costs to run, how big its machine
code is, or what the compiler wrote for the decoder.

Four of `instructions_golden`'s eight rows are welfare's run-speed terms.
Half the score's inputs were invisible to the gate meant to watch the score's
inputs, which is word for word the case the compile veins were added on.

### One of the three was not merely unlisted — it was unreadable

The reader takes tokens containing exactly one `=`. `emitted_golden` is
`calls=1777` and `text_golden` is `jsonbench text=80450`, so both would have
parsed the moment they were listed. `instructions_golden` writes `jsonbench
2912170881`, which contains no `=` at all, so every line yielded no pairs — and
no pairs is indistinguishable from no movement. The file could have been
listed and still read as empty.

### Measured, not argued

Same branch, `jsonbench` forced from 2,860,478,794 to 2,999,999,999, a rise of
139,521,205:

    the gate as it stands on main   exit 0, no output at all
    the gate with the three veins   exit 1, names work_jsonbench, refuses

The same forcing on the other two, with the reader fixed: `emitted_lines
11,585 -> 99,999` and `text 647,120 -> 667,120`, both named as worsenings.

### What the names are, and why text sums

Direction is looked up on the counter with its FILE PREFIX STRIPPED, which the
first draft of this change got wrong: `work_jsonbench` went in the table, the
lookup asked about `jsonbench`, and a 139-million rise was reported as an
improvement. Caught because the fixture was run rather than reasoned about — a
gate that reports the wrong direction is worse than one that says nothing,
because it tells you the thing you should be worried about is fine.

`emitted_golden`'s four counters needed no table entry at all: `lines`,
`calls`, `branches` and `defines` are there already for `compile_golden`. They
do need the file prefix, so the decoder's IR line count and a compile sample's
line count are two sums rather than one — otherwise a rise in either could hide
a fall in the other.

`text_golden`'s eight rows share the field name `text`, so it sums to a single
number over the binaries. That is the treatment `compile_golden`'s five samples
already get, and the per-golden exact diff is what says which binary moved.

### A second row rather than a repointed one

`a_counter_worsens_for_nothing` stays on `compile_instructions`.
`a_runtime_counter_worsens_for_nothing` is new and worsens `jsonbench`. Two
rows, because the two files are read by DIFFERENT CODE — one by the field
reader that has always worked, one by the bare-pair reader added here — so a
row proving the first proves nothing about the second. Both proved red in a
worktree of HEAD and green on restore.

### Three more, found by asking the question properly

Listing `bench/*.txt` and subtracting the files this program names turned up
three others nobody had listed: `cost_golden_escape.txt`, `cost_golden_scan.txt`
and `cost_golden_wide.txt`. All three are `name=value` like their four listed
siblings, so nothing had to be taught to read them and they go in here rather
than into a note — the list was short by three, which is a plainer failure than
the unreadable vein above and turned up the same way.

`scan_allocs` forced from 3,977,890 to 4,977,890 is now named and refused; on
main it was silent. Every file in `bench/` is read by this gate as of this
entry, which is a property worth stating because it is the one that decays.

## 2026-08-26 — a number the page stated about the present had been wrong for three changes

Searched this file and `log/compiler-log-archive.md` for `golden_prose` and for
`data-golden`: the gate is recorded arriving, to close the class where "the
landing page and the compiler page each quoted an allocation count the ratchet
had long since moved past". Nothing records which goldens it can reach.

It reaches two. `decode.` keys read `bench/cost_golden.txt` and `encode.` keys
read `bench/cost_golden_encode.txt`, and there is no third family. Six numbers
in the whole site carry the attribute — three on compiler.html, three on
index.html — and all six are decode counters.

So compiler.html said, in the present tense, that the library's front end
**does 17,786 expression visits**, while `front_end_visits` went to 17,886
(#1034's test-file import), then 16,818 (#1041 staging the library without its
test file), then 16,806 (#1043 shedding `must` and `defect`). The gate reported
`0 golden-quoting number(s) drifted` on every run through all three, and it was
right about the question it was asked.

### The class, not the instance

`compile.` now reads all three compile veins as one family —
`compile_memory_golden`, `compile_allocs_golden` and
`compile_instructions_golden`, whose key names do not collide, so which file a
counter came from is a fact the page has no reason to carry.

The sentence is fixed as history rather than restated: 17,786 and 23,224 were
both true when the reader-index change was measured, so they read as past
tense, and the current figure sits beside them as a tagged number that moves
on its own. `--write` was run in both directions to check the rewrite
round-trips and keeps the comma style.

### The gate had no row either

`golden_prose` runs as the second step of the welfare job, and `welfare_floor`
satisfied `cover` for both steps — the same shape as the differential sweeps
three entries above, and with a live failure attached rather than a
hypothetical one. `a_published_number_drifts` anchors on the attribute rather
than the value, because the counter moves whenever the front end does. Proved
red in a worktree of HEAD and green on restore.

### What is still untagged

Most of what compiler.html quotes is narrative about a particular change, which
is history and correctly left alone. Anything phrased about the present should
carry the attribute; this entry fixes the one that was caught. A sweep of the
page for other present-tense claims is OPEN.

## 2026-08-26 — an arm cannot see an err its own hako raised, on all three engines

Gavel 24, clause 1, built. Clay's words at the sitting: *it was never advisory.*
The rule is dispatch semantics now — at match time a failure does not enter an
arm its own hako raised, and infectiousness carries it onward exactly as if the
arm were not written. "Your own failures only bubble", executing itself instead
of being warned about.

### What an err carries, and what an arm knows

An err records the package that RAISED it, beside the trace line it already
carried. On all three engines the two are read off the same frame, so no
construction site can set one and forget the other.

Native and wasm take that further: codegen emits ONE literal per raise site,
`"{hako}\0{fn} at {file}:{line}"`, and the runtime splits it — the match rule
reads the first half, the endpoint report the second. Nineteen runtime
signatures in `runtime.c` carry an origin and nine codegen sites emit one;
threading a second argument through all of them to move a package name was not
worth it, and one literal cannot drift apart from itself.

The guard is emitted only where a pattern CAN hold an err — `(err …)`, an
`:err` annotation, or a typeset with err among its members. Everything else
refuses failures already, so a check there would cost a call per match on a hot
path to learn nothing.

### The granularity question the build exposed, and Clay's ruling

`package_of` answered `std` for every shipped module. Invisible until an err's
raiser became part of dispatch, and then wrong at once: `std/testing` and
`std/json` came out the same package, so `when_failed` could not rescue a
failure `decode` raised and the harness could not report a test failure.

Clay ruled it the same day: **a package is a directory, and its import path
names it** — Go's rule, named as Go's. `std/json` and `std/testing` differ;
`std/json/json.kso` and `scan.kso` are one; `std/net` and `std/net/http` are
two. It applies to a program's own modules too, and that is what makes the rule
teachable rather than merely enforceable: a decoder module and the module that
reports its failures are two packages, so the reporting arm is licensed exactly
where a reader would put it.

### What it cost the corpus and the book, which is the honest measure of it

A package's own failure is now completely opaque to that package. A bare binder
already refused failures; the two err-admitting patterns now refuse own-origin
ones. So you cannot pass your own failure to your own function at all — every
"render my own possibly-failed value" helper moves one package over.

- **`lib/json` lost `must` and `defect`.** `must` converted json's own parse
  failure into a defect, which json may not read. It had exactly one caller —
  its own test — and its documented purpose in ch08 was the case that no longer
  works. The caller writes it now, in their package, where json's failure is
  foreign; ch08 says so and shows the panel as a caller's file.
- **Nine corpus fixtures migrated.** Seven micro, the typeset entryfile fixture,
  and `wrap_cause`, which now wraps `std/text`'s failure because wrapping is
  something you do to somebody else's.
- **Four book samples restructured.** ch07's teahouse moves its reporting to
  the entry; ch08's `using` splits `describe` from `told`; `literal` and
  `positions` become two-module programs — a decoder and a reporter — which is
  how you would write them anyway and now the only way you can.
- **The pub self-seed retired.** It assumed a published err parameter sees its
  own package's failures because the callers are not all in view. Right while
  this pass was the only enforcement; wrong now, and under it every pub
  bare-err arm was a violation, `when_failed` included. What survives is what
  the written call sites prove, and `kanso check` REFUSES it rather than
  mentioning it: an arm that can never fire is dead code wearing the shape of
  error handling.

### Watched red on three engines at once

`tests/golden/micro/an_arm_cannot_see_its_own_hakos_err.kso` prints three
lines: the arm on a value, a foreign rescue, and the arm on an own err. With
the rule taken back out of the interpreter, native and wasm together, the third
line reads `false` instead of `foreign reads 99` — the arm matched, and
`when_failed` on the resulting string answers false. Restored, all three agree.

### The front end got cheaper doing it

`lib/json` shed `must` and `defect`: expression visits 16,818 to 16,806,
allocations 62,110 to 61,981, peak 825,664 to 822,004. Welfare rises, and the
rise is banked in this change rather than left for the next one to spend.

### What the arms cost, in the decoder and in the binary

Two runtime veins move, and they move for the same reason from opposite
directions. In the decoder's IR: calls 1775 to 1777, branches 1175 to 1176,
lines 11585 to 11574. Three `k_not_own_err` calls and a declare go in — the
arms that can now be skipped ask before they match — while `lib/json`'s
`defect` record comes out, taking six string constants, one call and its arm in
the field dispatch with it. `defines` does not move: `defect` was a type, and
`must` had already been dead since #1034 stopped calling it.

In the machine code every binary grows, and by a different amount each:
jsonbench +544, encodebench +624, oneshot +592, basket +256, widebench +624,
deepbench +160, escapebench +80, pendbench +224. Both costs scale with the
program rather than with the runtime — a call and a branch per skippable arm, a
package name per raise site — which is why the spread runs from 80 bytes to 624
where #1040's constructor fix landed on 48 for all eight.

The spread was measured in the container, and the container reproduces main's
whole `.text` table exactly, which is what makes it usable: `bench/text_golden`
pins `clang=18.1.3` and nothing else, because what compiles the emitted IR is
clang and what rustc built is only the program that wrote it.

### What it costs to run, and what it costs to compile

The runtime instruction vein, from the runner:

    jsonbench    2,860,478,794 -> 2,912,170,881   +51,692,087   +1.81%
    oneshot         46,596,968 ->    46,941,571      +344,603   +0.74%
    widebench       84,817,033 ->    85,113,625      +296,592   +0.35%
    basket          57,400,154 ->    57,408,155        +8,001   +0.01%
    encodebench  9,727,166,055 -> 9,727,535,960      +369,905   +0.004%
    deepbench, escapebench, pendbench unmoved

Three rows do not move at all, and that is the finding rather than a footnote.
The cost is paid per err-dispatching arm that runs, not per instruction
executed, so the decoder's inner loop — which is made of such arms — pays 1.81%
while the three programs whose hot paths hold none of them pay nothing.
encodebench renders rather than dispatching on err, and its rise is four
thousandths of a per cent across nine billion.

`compile_instructions` rises 56,849,156 to 57,490,077, and that row is part
work and part layout: the front end now carries a package name on every raise
site and asks at every skippable arm, and `src/runtime.c` grew, which shifts
the compiler's own code around a static it never reads. The entry above this
one measured that same effect at +664, +393 and −6,763 on three sittings of a
single diff, so the two halves are not separable from this row alone. What is
separable is every counter that measures ONLY the front end's work, and all
three of those fell: visits 16,818 to 16,806, allocations 62,110 to 61,981,
peak 825,664 to 822,004 — the pub self-seed retired and `lib/json` shed `must`
and `defect`.

### The floor moves, by hand, because the tool refuses

welfare goes 84.14 to 84.12. Gavel 24 clause 1 is a language change, and Clay's
ruling of 2026-08-25 governs it: the floor is absolute against refactorings and
permeable to the language, so a doctrine-compelled change lands, the floor
moves, and the fall is recorded against the change that spent it. The 84.79
entry has this exact shape and is the precedent.

`welfare --set` refuses a fall of more than 0.01 and refused this one. That
refusal is the design — its own comment names hand-editing
`bench/welfare_floor.json` as the single override, precisely so the move
appears in a diff a reviewer reads rather than behind a flag. So the floor was
edited by hand and the entry carries the whole attribution. No compensating
optimization was hunted, which the same ruling forbids.

### Each counter, by the name the gate prints

`work_jsonbench` rises 51,692,087 and is the one that matters: the decoder's
inner loop is made of arms that can hold an err, and each now asks before it
matches. `work_oneshot` rises 344,603 and `work_widebench` 296,592 for the same
reason at smaller volume. `work_encodebench` rises 369,905, four thousandths of
a per cent across nine billion, and `work_basket` 8,001, a hundredth of one —
both render rather than dispatching on err, and both are at the size where
layout accounts for as much as work. `work_deepbench`, `work_escapebench` and
`work_pendbench` do not move at all.

`emitted_calls` rises 2 and `emitted_branches` 1: the guard's three calls into
the decoder, less the one that went with `lib/json`'s `defect`. `emitted_lines`
falls 11 against them, because a record that goes takes six string constants
with it. `text` rises 3,104 over the eight binaries, spread from 80 to 624
rather than uniform, since both costs scale with the program.

Every one of these is bought by the same thing: an arm cannot see an err its
own hako raised, and the ruling says that is dispatch rather than a check. The
sum is priced in welfare and the floor moves with it.

### The trend gate said nothing about any of the runtime rows

It named four compile counters and refused the one whose sentence was missing.
It did not mention jsonbench's 1.81%, or any `.text` row, or the three emitted
counters, because `bench/instructions_golden.txt`, `bench/text_golden.txt` and
`bench/emitted_golden.txt` are not in `bench_goldens`. Only the per-golden
exact diff caught them.

That was the blindness #1044 closed for the two measured compile veins, still
open one vein down — and it mattered more here, because `instructions_golden`
is where welfare's four run-speed terms come from, so half the score's inputs
were invisible to the gate meant to watch the score's inputs.

Filed as its own change rather than folded in, because it touches the gate this
branch was being judged by, and closed by the entry directly above this one
while this branch waited on CI. The finding stands as recorded: the numbers in
this entry are the ones that went past the gate unnamed, and they are why that
change exists.

## 2026-08-26 — the last four differential steps get rows, and one of them found a hole

Named OPEN in the entry three above this one, which rowed the first three. The
diagnostics differential job runs nine sweeps as nine steps and `cover` is per
JOB, so a row on any one satisfied it for all nine.

All nine are rowed now. Each mutation was applied in a worktree, built, and its
own sweep run, and each was green again on restore.

    native_floors_a_negative_modulo          -1 % 2 -> 1, -1 % 7 -> 6
      integer `%` floors on native where the oracle truncates. Both are a
      language's defensible choice — C and Rust truncate, Python and Haskell
      floor — and kanso may only have one. No diagnostic is raised, every
      allocation counter is flat, and no golden prints a negative modulo.

    native_sequences_two_effects_backwards   10 of 10 shapes disagree
      `>>` runs its right side first, so `io/write "a" >> io/write "b"`
      prints `ba`. The executor is written twice with no shared code, and
      only this sweep asks what happens when two effects are SEQUENCED.

    a_diagnostic_arrives_without_a_golden    1 newly unpinned
      a `Diagnostic::new` with literal text and no golden, appended as a
      function nobody calls so the tree still builds. What is under test is
      whether the SCAN finds it, not whether the compiler can raise it.

    a_private_name_crosses_an_import         1 no longer as recorded
      the check that refuses a private name at an import looks itself up
      under a key nothing declares, so `pub` stops meaning anything across a
      module boundary.

### The module mutation is in the shared front end, and that is the point

It changes no engine's behaviour relative to the other — both go on agreeing
perfectly, because both read the same front end. What moves is the VERDICT, and
`module_differential` is the only gate that asks that question of a real
directory on disk. Agreement alone cannot see a rule that is wrong in both
engines, which the sweep's own header says it has had.

### The hole it found, and the case that closes it

The first time it was proved, it went red through the KNOWN-DEFECT ledger
rather than the case list: `0 wrong; 1 known defects, 1 no longer as recorded`.
`w1` — a module's own pub shadowed by a dependency's name, filed against task
#51 — was the ONLY case in the sweep touching the import-privacy refusal at
all, and it is recorded as a defect rather than passing.

So the row worked and its grip was on an entry meant to be deleted the day task
#51 is settled. A mutation whose red depends on a recorded defect stops proving
anything the moment the defect is fixed.

The rule itself is fine. Asked directly, the compiler refuses exactly as it
should:

    error[opacity]: `secret` is private to module `dep`
                    — only pub names cross an import

What was missing was a case asking. Every other module in the sweep uses a pub
name, stays inside the module, or is `w1`. So the rule `pub` exists for — a
plain private name reached across a plain import, nothing else going on — had
no passing test on the one surface that runs whole modules from disk.

`c17` is that case. With it the sweep reads 17 modules, and the mutation now
goes red through the case list where it belongs:

    17 modules, 1 wrong; 1 known defects, 1 no longer as recorded
      a private name reached across an import
        expected a refusal saying 'error[opacity]: `secret` is private
        to module `dep` — only pub names cross an import', but it compiled

`w1` stays exactly as it is, recorded as it behaves.

### numeric's row carries the argument CI runs

`-- 0`, no random rounds. Every random draw lands past the native ceiling, so
the rounds spend ten minutes re-confirming what the 561 edge cases already say.
A row whose gate string differed from the workflow's would be proving a
different program.

## 2026-08-26 — the rest of the page's present-tense claims

The entry two above this one closed the class and left the sweep OPEN: which
other figures on docs/compiler.html are stated about the PRESENT rather than
about a particular change. Swept. Two, and only two.

`§08`, the borrowed-names paragraph, said the json library's front end
**allocates** 64,884 times and **retires** 59,527,334 instructions. Both verbs
are present tense about the compiler in the tree, and both figures are what
that change measured, not what the compiler does now: `compile_allocs` reads
61,981 and `compile_instructions` 57,490,077. The sentence keeps 91,185 and
66,961,255 as the before-figures they always were, puts the two after-figures
in past tense as what that change reached, and carries the current pair as
tagged numbers that move on their own.

`§08`, the compile-memory band paragraph, said the runner **holds** 864,300.
That one is narrative — the whole paragraph is the story of the band being
ruled away — so the fix is the tense rather than a tag: it *held* 864,300, on
the day the band came off. `compile_peak_bytes` is 822,004 now, and a reader
who took "holds" for the present was forty thousand bytes out.

### What is deliberately left alone

Everything else the page quotes is narrative about a particular change, and
correctly reads as history: the encode allocation chain (42,312,800 to
68,640,508 to 26,327,708), the 1,874,992 allocations, 5.25 instructions per
cycle against jq's 5.20, the 365 conversion sites the interned symbol would
have cost, the 87/79/67 type counts. None of those claims anything about the
compiler as it stands, and none has a golden to be pinned to.

So the page now has five tagged numbers on the compile veins and three on
decode, and every remaining figure is dated by its own sentence. The rule that
falls out of doing this twice: a number about the present carries the
attribute, and a number about a change carries a past-tense verb. The gate
enforces the first; only reading enforces the second.

## 2026-08-26 — two failures in a record merge, and `some` is not a failure

Writing up the err spelling as two worlds (#1051) meant stating what the rule
does from the language's side rather than from the pass that enforces it. Two
programs written to check the write-up came back disagreeing with it.

### A record's fields are one operation, and the compiled engines forgot

Clay ruled on 2026-08-05 that two failures in one operation merge, the way a
parallel group had always merged them. The interpreter does that for a record
construction. Native and wasm returned the first failing field:

    operated: ["a" "b"]     both engines
    built:    a             native
    built:    ["a" "b"]     the interpreter, which is the oracle

Nothing in the corpus had ever built a record out of two failures, so three
engines disagreed and every gate stayed green. `k_rec`, `k_rec_reuse` and
`rt_mkrec` merge now. So does `emit_parsed_construction`, the
register-returnable construction the codegen writes in tail position, which
tested the first field, returned if it failed, and never evaluated the second
— its own comment claimed it propagated "exactly as `k_rec` would have", which
had quietly stopped being true.

### `some` was an unmarked rescue

`some` means a value that is not none, and `type_match_depth` answered
`("some", _) => true`, which takes an err. So

    pub fn caught _:some
      "some-arm swallowed it"

took a foreign failure and answered whatever it liked, with no `err` written
anywhere and no destructuring form in sight. That is the one door the
two-universe rule exists to watch, standing open in an annotation that names
no failure. An err is not "some value" — it is the absence of one — and the
arm refuses it now the way a bare parameter does, on all three engines.

Native could not compile the annotation at all: `unknown type `some``, at
build time, in a message about a backend, while `check` had already passed the
program because `some` is in its built-in list. The `declare i64
@k_check_some` sat in the codegen preamble with nothing behind it and nothing
calling it. It is written and wired now.

### The prices, and the one that had to be paid back

`emitted_calls` 1,777 -> 1,785 and `module_calls` 748 -> 749: the merged bail
is one call where the two early returns were none. Against them
`emitted_branches` 1,176 -> 1,168 and `emitted_lines` 11,574 -> 11,527, because
one bail block replaces two at eight decoder sites. `text` 650,224 -> 650,928:
the merge inlines at each site.

The vein that mattered was `work`, and only CI could see it. `k_rec` is 16.9%
of pendbench — every record construction walks its fields — and the first
version folded the merge into that walk. A/B on one host, so the container's
glibc offset cancels:

    pendbench   main 988,706,173   folded 1,020,722,638   +3.24%
    oneshot     main  46,941,172   folded  47,374,597     +0.92%

Three per cent of a benchmark for work that runs only when a field has already
failed. The scan keeps its early exit now and hands off to a `cold`,
`noinline` `k_merge_rest` from the index that failed: pendbench comes out at
987,905,773, 0.08% BELOW where it started, and oneshot at 47,276,631, +0.71%
that no function in the profile accounts for — the top fifteen are
byte-identical to main's, so what is left is code layout rather than work.

### What CI measured, and what it cost

The runner's rows, which the container cannot produce — its glibc is
2.39-0ubuntu8.7 against the runner's 8.8 — so they are copied from the gate's
own diff:

    work_jsonbench     2,912,170,881 -> 2,910,241,430   -0.066%
    work_encodebench   9,727,535,960 -> 9,866,843,909   +1.43%
    work_oneshot          46,941,571 ->     47,277,030   +0.71%
    work_basket           57,408,155 ->     57,416,154   +0.014%
    work_widebench        85,113,625 ->     85,209,624   +0.11%
    work_pendbench       988,706,559 ->    987,906,159   -0.081%
    compile_instructions  57,490,077 ->     57,490,136   +59

deepbench and escapebench do not move. The container reproduces the deltas
exactly where it was checked: oneshot measured 46,941,172 on main here and
47,276,631 on the branch, +335,459, and the runner's two rows differ by
335,459.

`work_encodebench` is the one that costs anything, and it is not work the
change added. Nothing in the encoder's profile moved: `k_b_append_mut` still
holds 23.24% and `k_rec` does not appear in its top five at all. The scan that
finds a failing field is the same instruction sequence it was, the merge is
`cold` and out of line behind it, and marking `k_pair_failure` cold as well
moved encodebench by fourteen thousand instructions out of nine billion. What
is left is an inlining shift from two functions arriving in runtime.c, which
is also why jsonbench and pendbench moved the other way. Welfare falls 0.00 —
below the display's resolution — and the floor was re-set with that reason.

Both fixtures were watched red first, which is how the second one was caught
at all: `some_is_a_value_not_a_failure` prints `failure: false` with the fix
out, because the arm swallowed the err and `when_failed` found no failure to
read. The construction fixture needed the wasm engine rebuilt with the fix
reverted to prove the corpus reaches it — the wasm corpus compares against
native rather than against the `.out` file, so corrupting the golden proves
nothing there. Worth knowing before the next person tries it.

## 2026-08-26 — `../` was reaching the diagnostics, and the first fix
cost 164,130 instructions that were not work

`resolve_import` strips a leading `./` with a comment saying why: the dots are
import syntax rather than part of the name, and carrying them into the
resolved path would put them in every diagnostic the module raises. `../` is
the same syntax and was never taken back out. A module reached as `../deep`
from `mid/` named itself `mid/../deep` — in its diagnostics, in the origin its
errs recorded, and in the hako `package_of` reads off that origin.

It survived because both module-error goldens pinned the unfolded spelling.
The corpus had been regenerated from whatever the code printed and nobody read
the path. That is the third defect this week whose shape is the same: a golden
regenerated without being read pins the bug instead of catching it. The
counters have the trend gate for this; the text goldens have nothing.

### The measurement that changed the implementation

The first version joined `base` with `../deep` and walked the components
afterwards taking `x/..` pairs out. CI charged the front end 164,130
instructions, +0.29% — on a program that never calls it, because `lib/json`
imports nothing relative. The container measured the same source 1,925
instructions CHEAPER than main:

    container   main 61,333,649   folded 61,331,724   -1,925
    runner      main 57,490,136   folded 57,654,266   +164,130

A change cheaper on one toolchain and 0.29% dearer on another is not doing
more work; it is an inlining outcome. The deltas usually carry between the two
hosts — oneshot's +335,459 matched to the instruction earlier the same day —
so a delta that does NOT carry is itself the finding.

So the fold is gone. Each leading `../` is now a step taken while resolving:
`base` loses a component, the name loses the prefix, the path is built once
from the result. No Vec, no second walk, no function to place. The runner
charges 18 instructions for it — one `strip_prefix` test per relative
import — and the container reads it 37 under main.

### A pure regression, attributed

Eighteen instructions worse and nothing better is what the trend gate refuses
outright. The escape is the one the gavel of 2026-08-25 put there: the floor
is permeable to the language, so the fall is recorded in
`bench/welfare_floor.json`'s history against the change that spent it. The
alternative was leaving import syntax in the language's own error messages to
protect an eighteen-instruction counter.

## 2026-08-26 — the third pattern gavel 24 named never got its guard on native

Gavel 24 names three patterns that can hold an err: `(err …)`, an `:err`
annotation, and a typeset with err among its members. The first two carried the
own-hako guard on every engine. The third did not, on native.

`emit_pattern` splits `Pattern::Annotated` into two arms — a typeset arm and a
general one — and only the general one asked `admits_err`. A typeset name never
reaches it. So the guard the third case was promised was never emitted, and a
package could rescue its own failure by naming a typeset instead of naming err:

    type maybe err int

    fn look _:maybe
      "the typeset arm took it"

    own = look (boom "a")

    interp   own: a                          the oracle passes it through
    native   own: false                      the arm took it

`admits_err` was already right — it recurses through a typeset's members — and
wasm was already right too, because `wasm_backend` handles typesets INSIDE the
one annotated arm and calls `admits_err` before anything else. Native's split
was the whole bug.

So the fix is not the guard added to the second arm; it is the two arms made
one, the way wasm has always had them. A plain annotation is a typeset with a
single member, which is what `wasm_backend` writes and what native now writes.
The shape that caused this cannot recur, because there is no longer a second
arm for a guard to be missing from.

The measurement said the same thing. Patching the typeset arm read +421,503
instructions in the container against the runner's +2,562 — a delta that does
not carry, which by the rule written the same day means layout rather than
work, since `kanso check` never runs codegen at all. The merged arm reads 16
BELOW main and the runner charges 456, down from 2,562.

Nothing else moved: the guard is emitted only where a typeset admits err, no
benchmark declares one, and emitted, text and every counter vein are
byte-identical. Four hundred and fifty-six worse and nothing better is the
pure regression the trend gate refuses outright, so the fall is attributed in
`bench/welfare_floor.json`'s history under the rule that the floor is
permeable to the language — the same escape #1053 used, for the same kind of
reason.

`a_typeset_arm_cannot_see_its_own_hakos_err` pins both halves: the failure
passes through, and the arm still takes a member that is not a failure.

## 2026-08-26 — a list can hold a failure, and native alone disagreed

The access side of the err rule was swept three times today and gave up three
bugs. This is the propagation side, and the sweep was the same: name every
site where a failure can reach an operation, then ask all three engines.

Five agreed — an operator merges two failures, a comparison merges, an
interpolation with two failing holes takes the first, a call takes the first,
a list literal builds rather than propagates. The sixth did not.

    listed = [(boom "a") (boom "b") 3]

    interp   length: 3   listed: [err "a" err "b" 3]
    wasm     the same, through eval::render
    native   error[endpoint]: unhandled err reached the executor: ["a" "b"]

Both engines BUILD the list — `length` is 3 everywhere — so the divergence is
rendering, which is one of the three surfaces this project already knows is
divergence-prone. `k_render` opened with

    if (v.tag == K_ERR) return v;

which is right at the top of a render and wrong inside one. Twenty lines down
the same function has `case K_ERR: return k_concat(k_str("err "), ...)` — the
oracle's answer, and dead code, because the early return caught every err
first. Two places deciding one thing, and the one that ran was not the one
that was right. Third instance of that shape today, after the `some` arm and
native's two annotated arms.

Printing a list of results handed the merged failure to the endpoint and took
the successes with it, which is the behaviour a program would notice.

### What the fix cost, and the shape that made it cheap

Splitting `k_render` into a top-level entry and a held-value body cost 1,424
bytes of .text per binary — 1.76% on jsonbench — because two bodies is two
bodies however the symbols are marked; `static` moved nothing. One body with a
`held` flag says the same thing for 128 bytes. Eleven times cheaper, and the
error message the reader sees is identical.

### What it costs, measured rather than assumed

The `held` flag is one comparison per render call, and the veins say so:

    work_jsonbench     2,910,241,430 -> 2,910,241,403     -27
    work_encodebench   9,866,843,909 -> 9,866,843,910      +1
    work_oneshot          47,277,030 ->     47,277,078     +48
    work_basket           57,416,154 ->     57,436,155 +20,001
    work_deepbench       807,094,292 ->    807,094,318     +26
    work_escapebench     258,574,070 ->    258,574,097     +27
    work_pendbench       987,906,159 ->    987,907,097    +938
    widebench                                          unchanged
    compile_instructions  57,490,610 ->     57,492,931  +2,321
    text, every binary                                    +128 to +144

basket is the one worth reading: +20,001 is 0.035%, and the shape of the
number says what it is — one comparison in a render that runs twenty thousand
times. jsonbench came out 27 BELOW, so this is not a pure regression and the
gate does not need the floor's escape, though the entry recording it stands.

What it buys is the differential law holding on a surface where two engines
already agreed and the third was alone.

## 2026-08-26 — the same fix, missing the map, and why the fixture let it

#1057 gave `k_render` a `held` flag so a failure inside a container renders
rather than propagating. It reached lists and records. It missed the map
beside them.

Two mistakes stacked. The edit was applied over a fixed-size window of the
function, and the map's key and value renders sit past the end of that window,
so they kept calling the top-level entry. And the fixture held a list alone,
so the corpus agreed with the half-fixed compiler and CI went green on it.

    mapped = { "k":(boom "v") }

    interp   mapped: { "k":err "v" }
    native   error[endpoint]: unhandled err reached the entry: "v"

Live on main for the length of one merge. The fixture now holds a list, a map,
and a map nested in a list, because a spec that covers one member of a family
proves nothing about the others — which is the general form of what went
wrong, not a detail of this function.

Sixteen bytes of .text a binary, and the work vein moves by tens: jsonbench
+23, basket +23, pendbench +38, oneshot -21. The map render is on no
benchmark's path, so what these are is the one comparison arriving at a
slightly different address.

## 2026-08-26 — what a failure does at every site, pinned

The rule is one sentence: a failure reaching an operation carries past it. What
that means differs by site, and each site is implemented separately in three
engines. Asking one question of all three — what happens HERE — produced four
divergences in an afternoon (#1052 twice, #1056, #1057 with #1058), every one
a site where two engines agreed and the third did not.

None of them was hard to find. What was missing was the question being asked
of each site in turn, and a place to write the answer down. Twelve sites, one
fixture:

    merge      operator, comparison, constructor, `&` join
    first      call with no matching arm, interpolation, `>>` sequence
    holds      list literal, map literal — a container is not its contents
    through    field read, index, builtin

`what_a_failure_does_at_every_site` runs all twelve and pins what each answers.
A site that drifts on any engine goes red; a site nobody has thought about is
a line missing from the file, which is a thing a reader can notice.

Worth naming because the same shape recurred four times: the right code was
already written and something shadowed it. `type_match_depth` answered `some`
before it answered `err`. Native's typeset arm returned before reaching the
annotated arm's guard. `k_render` returned an err before reaching the case that
renders one. And the map render sat outside the window an edit was applied
over. In each the correct behaviour existed in the file and did not run — which
is what a rule spread across pattern kinds, container kinds and three
implementations costs, and it is the concrete form of the argument for putting
err-handling in one named place.

## 2026-08-27 — which patterns can hold a failure, as a table

Gavel 24 rules on matching. Three patterns can hold an err — `(err …)`, an
`:err` annotation, a typeset with err among its members — and none of them
holds an err its own hako raised. Two of yesterday's divergences were cells of
that table: `_:some` took a foreign failure on the interpreter and on wasm
while native refused the annotation at build time (#1052), and a typeset arm
took its own hako's failure on native alone (#1056).

The rule was written down; the table was not. Ten pattern forms against two
err origins is twenty cells, and three fixtures covered three of them. The
bugs were in the seventeen nobody had asked about.

    pattern   own   foreign
    _         past  past
    v         past  past
    v:err     past  took
    (err r)   past  took
    v:maybe   past  took
    v:some    past  past
    v:int     past  past
    v:string  past  past
    v:none    past  past
    1         past  past

`took` means the arm fired; `past` means the failure went by as though the arm
were not written. The foreign column names the three patterns the gavel names.
The own column holds for every spelling, not only for the three that had been
thought about.

Adding a pattern form to the language adds a row here. A form with no row is
one the question was never asked of.

Fixture only — no counter moves.

## 2026-08-27 — the fourth container, and a sentence that was wrong about it

`a_container_can_hold_a_failure` opened by naming three containers and a
record. It then tested three. The header even says why that matters — "a spec
that covers one member of a family proves nothing about the others" — which
is how #1058 got found.

The record takes a step to reach, and the sentence describing it was wrong.
"A record's field once the record exists" cannot happen: a constructor merges
its arguments' failures, so `held (boom "a") 2` is a failure and not a record
holding one, which #1059 pins as the `built` row. What a record can hold is a
field that is a list or a map holding one — the constructor sees a container,
and the failure is inside it.

    fielded: held [err "f"] 2
    keyed:   held { "k":err "g" } 3
    buried:  [held [err "h"] 1]

Byte-identical on the interpreter, native and wasm.

What these lines do NOT pin is the `held` flag on `k_render`'s record branch.
Reverting that one argument to 0 changes none of them, because the field is a
container and the container branch sets the flag again for its own items. The
flag is load-bearing only for a field that is itself an err, and no program can
build one. So the lines pin routing — that a record's fields reach rendering by
the same road its items do — and the comment now says so rather than claiming a
mutation they would catch.

## 2026-08-27 — the render differential had never rendered a failure

The project names rendering divergence-prone and gave it a harness for that
reason. Two rendering divergences shipped this week anyway — #1057 and #1058,
both `k_render` answering differently from the oracle for a container holding
a failure — and the harness could not have caught either, because not one of
its 68 values was a failure.

Eighteen more, built the way the rest of the corpus is built: a failure in a
list, in a map, in a map beside a value, nested twice, with a reason that is
an int, a float, `none`, `true`, a list, a map, an escaped string, an empty
string, and a constructor whose merged failure renders in a list.

    68 values rendered, 0 disagree      before
    86 values rendered, 0 disagree      after

Watched red for the right reason. With #1057's `held` flag reverted — the
early `if (v.tag == K_ERR) return v;` restored — the sweep answers

    86 values rendered, 16 disagree

and names each one: native hands the failure to the endpoint where the oracle
prints the container. The two that still agree under the bug are the two whose
value is a bare failure, which both engines propagate.

This harness compares native against the oracle. wasm's rendering is covered
by the browser differential over the golden corpora, where #1059 and the
container fixture live.

## 2026-08-27 — eight programs emitted code nobody counted

`bench/emitted_golden.txt` exists because 7.6% of decode speed leaked away
between 2026-07-27 and 2026-08-07 with every allocation counter
byte-identical: the decoder gained 20% more calls and 23% more branches for
the same work, and nothing watched the dimension that moved.

It watches the decoder. `scripts/gates/build_benchmarks.sh` builds nine
programs, and the machine-code gate beside it reads eight. So the leak that
was found once could have happened in any of the other eight, in silence,
including `scanbench` — the largest at 20,023 lines, and absent from
bench/text_golden.txt as well.

    encodebench defines=153 calls=1663 branches=1014 lines=10466
    oneshot     defines=154 calls=1779 branches=1159 lines=11469
    basket      defines=118 calls=1324 branches=726  lines=7181
    widebench   defines=169 calls=1884 branches=1116 lines=11507
    deepbench   defines=92  calls=840  branches=520  lines=5046
    escapebench defines=29  calls=97   branches=48   lines=708
    pendbench   defines=106 calls=1250 branches=611  lines=6169
    scanbench   defines=307 calls=3745 branches=2216 lines=20023

A second file rather than eight more lines in the first, for two reasons. The
decoder's golden IS its history — eleven dated entries explaining every move —
and summing eight programs into it would let a rise in one hide a fall in
another. And the counters are read from kanso's own IR before any linker runs,
so they are host-independent: `sh scripts/gates/emitted_code.sh` reproduces
the committed jsonbench numbers exactly in this container, which is how these
eight could be generated here at all.

Two ratchet mutations now, one per golden. A gate that reads two files and is
only ever proved against one is proved for one.

### The trend gate cannot tell a wider vein from a worse program

Found while deciding whether `scanbench` could join bench/text_golden.txt in
the same change. It cannot:

    worsened: text 652,128 -> 777,794  (bench/text_golden.txt)
    FAIL  a pure regression: something got worse and nothing got better.

Nothing got worse. Those 125,666 bytes are scanbench's `.text` counted for the
first time.

The first guess was that `judged` reads a name missing from the base as zero,
and a fix for that was written. It changes nothing here, and measuring said so
before it shipped: `text` is not a missing name. The gate sums a golden's
counters BY FIELD NAME across samples, deliberately — the file's own comment
says `text` is "one number across the eight binaries" and the per-golden diff
is what names which. So a ninth binary raises the sum, and a rise is a rise.
Wider and worse are the same shape to a sum.

The emitted counters above escaped only because a golden with no copy on main
is skipped outright, which is the second reason the eight went into a file of
their own rather than beside the decoder.

Whether a summed vein should be compared per sample is a question about what
this gate is for, not a bug in how it reads. Left open here rather than
answered in a change about emitted code.

## 2026-08-26 — the argument gets a page: why there is no bind

Clay lost the thread and asked for it back: he remembered devising a
way to avoid explicit combinator keywords and could not reconstruct
it. The device was gavel 24's three moves — own errs unreceivable,
foreign handling as ordinary arms, signature-directed elaboration —
and it lived only in this log's archive, which is exactly how it got
lost. compiler.html gains entry 23, "why there is no bind", stating
the assembled argument as settled surface: the combinator words stay
off the surface in every variant under consideration, and the entry
fences the genuinely open half — whether the failure machinery lives
in dispatch or in the elaborator — as the design ledger's question,
not the page's. The reconstruction-from-log episode is named in the
entry itself as the reason it exists.

## 2026-08-27 — the clone that owns a qualified name, measured at last

`module_differential`'s known defect `w1` — a module's own `pub` reading as
private because one of its imports exports the same name — was filed on
2026-07-27 against "the question task #51 holds a gavel over". Task #51 was
ruled on 2026-08-17 as gavel 51 and built the same day. The defect was never
revisited, and gavel 51 does not settle it: gavel 51 is one module reached by
two paths, and this is two modules colliding inside one namespace.

Three things the ledger did not have.

**Who claims the name.** An instrumented `qualify` says it plainly:

    PROBE key=dep/join taken=None       is_pub=false synthetic=true  file=std/text/text.kso
    PROBE key=dep/join taken=Some(false) is_pub=true  synthetic=false file=.../dep/dep.kso

The first writer is a bare-enrollment clone of a NON-PUB ARM of std/text's
`join` group. The map is first-writer-wins, so `dep`'s own `pub` never gets to
set the flag.

**When the clone survives.** Only when the importing module declares the same
name. With no `join` in `dep`, `canonicalize_bare_aliases` folds the clone
away and `dep/join` is an ordinary unknown name. The collision is the
condition for the bug, not the enrollment.

**What the one-line fix costs.** Letting `dep`'s `pub` win the flag makes the
refusal go away and makes `dep/join` reach std/text's arm. Written so that
`dep`'s own arm cannot match —

    pub fn join a:int b:int          in dep
    print "{dep/join ["x" "y"] "-"}" in app

    x-y        native
    x-y        interpreter

`dep` never exported that function. The archive recorded this hazard in
2026-07-27 without a case; here is the case, on both engines. The refusal is
the only thing between a program and a silent re-export of a dependency under
a name its author never wrote.

So the flag and the dispatch are one question, and it is a design question:
making `dep/join` mean `dep`'s declaration requires the enrolled clones out of
`dep`'s qualified namespace, and `dep`'s own bare call sites are rewritten
INTO that namespace during qualification. The bare overload space would need a
spelling of its own that a consumer cannot write. Filed in
design/pending-gavels.md under "Which claim owns `dep/join`", with a
recommendation. `w1` stays recorded as it behaves; its label stops citing a
gavel that has fallen.

## 2026-08-27 — one cell of five, and two wrong answers on the way to it

§18 of docs/compiler.html named an open item: a loop threading a map retains
71.9 MB where the scalar form holds 1.5 MB, "and the fix is to give a threaded
container storage outside the rewound region the way a byte builder already
has."

`git log -S` dates that sentence to 2026-08-01, in #664. The archive holds four
entries from 2026-08-02 that build exactly it and revert it three times, every
attempt moving the arena counter and none moving the process — the best got the
counter down 69x while peak RSS went 71.8 MB to 75.2 MB. The page has carried a
declined fix as its plan for twenty-five days, which is what "negative results
are recorded on the compiler page so ideas stay declined" exists to prevent.

Then measuring it took three goes, and the two wrong answers are the useful
part of this entry.

FIRST WRONG ANSWER. A loop threading a map beside a scalar brackets and holds
1.7 MB, where the map-only loop I wrote held 69.7 MB. I concluded the scalar
parameter was the difference and wrote it up. It was not: my map-only loop
recursed through a helper group and the other did not, so the comparison
carried two changes at once.

SECOND WRONG ANSWER. Controlling for that — a self-recursive, map-only,
two-argument loop — brackets, at 1.7 MB. So the container is not the axis
either, and the page's "a map or a list" was wrong in both directions.

WHAT IS ACTUALLY TRUE, at 1.6 million iterations with the same string built and
dropped each time round (`sh_str` reads 72,282,272 in all five):

    carried    recursion         beat_iters    arena peak    peak RSS
    scalar     self              1,600,000      1,048,576      1.7 MB
    map        self              1,600,000      1,048,576      1.7 MB
    scalar     through a helper  3,200,000      1,048,576      1.7 MB
    list       through a helper  3,200,000      1,048,576      1.7 MB
    map        through a helper          0     72,351,744     69.7 MB

One cell of five: a map carried by a loop that recurses through a second group,
and nothing else. A list through the same helper brackets, and the same map in
a self-recursive loop brackets. The refused group draws no line from
KANSO_BEAT_REPORT at all, so the analysis never reaches it.

That is why the storage fix could not have paid — it aimed at what the loop
carries, and carrying a map is not the problem.

WHY MAPS ARE OUT, from `beat.rs` itself:

    /// Maps stay out: the first read caches a freshly allocated sorted view —
    /// an above-the-mark pointer — into the below-mark header. Instant dangle.
    const THREADED: Set = SCALAR | STR | BYTES | FN | REC | DESC | LIST;

So the exclusion is deliberate and the hazard is real. What the table adds is
that four of the five shapes bracket anyway, the map-under-self-recursion among
them. Crossing is not what separates them: a probe on the crossing test answers
`loop/go/2 pos1 not_crossing=false` for exactly the self-recursive loop that
brackets, so the map crosses the iteration boundary there too and the loop
still rewinds. The narrowed question is why a helper hop changes the answer.

A program can have the forty-fold win today by writing the loop so it calls
itself, which nothing in the tree said.

WHERE IT HAPPENS, and it is a priced refusal rather than an oversight.
Instrumenting `eligible_clusters` on both loops:

    map  through a helper   entries=1  edges_ok=true  carried=[go[1], onward[1]]
    list through a helper   entries=1  edges_ok=true  carried=[]

`cluster_edges_ok` SUCCEEDS in both, so the edge check is not what refuses the
map. What differs is what becomes of the slot. LIST is in THREADED, so the list
slot is threaded and nothing is carried. MAP is not:

    const THREADED: Set = SCALAR | STR | BYTES | FN | REC | DESC | LIST;

so the map slot falls through to `carried` — a slot the loop evacuates at every
rewind — and then this fires:

    // A demoted entry buys a plain beat and nothing more. A carried
    // slot is evacuated at every rewind, and a cluster reached only
    // by a tail call is one whose cost nobody has measured — the
    // json string scanner pays 8 GB of copies for the licence.
    if !entries.is_empty() && !carried.is_empty() { continue; }

So the forty-fold difference is a trade somebody already priced, on a different
program. A draft of this entry called it a missing licence, by reading the type
sets and never the selector; the probe above is what corrected it, and that is
the fourth reading of this loop a measurement has overturned.

WHAT IS ACTUALLY OPEN is whether the price is right for THIS program. A loop
carrying a one-key map evacuates a handful of bytes per rewind where the string
scanner evacuates a buffer, and the rule cannot tell them apart. Two ways out:
make a map threadable the way a list is, which returns to the sorted-view
hazard the threadable set was drawn to avoid; or make the carried-slot refusal
read the size of what it would copy. Both are licensing changes rather than bug
fixes, which is why this entry stops at naming them.

## 2026-08-27 — four acceptance tests had started passing, and nothing said so

Five tests in the tree carry `#[ignore]`. Each was written to fail, as the
acceptance criterion for something not built, and each says in as many words:
delete this attribute in the change that builds it. `cargo test -- --ignored`
says four of the five now pass.

    a_bare_list_is_or_is_not_bytes   the bytes ruling landed; four sites agree
    accumulator_growth               the rewind is built for this shape
    view_cache_is_returned  (x2)     the view has an owner

None is vacuous, and each was checked rather than trusted.

`view_cache_is_returned` guards a leak measured at 76,800,048 bytes over 1.6
million iterations — forty-eight bytes a map, unbounded in the iteration count,
and invisible to every arena counter because the view is malloc'd. It reads
`view_allocs` against `view_frees`:

    20,000 transient maps      view_allocs=20000   view_frees=20000

The gap is zero, where the test allows a small constant. The leak is closed.

`accumulator_growth` asserts a hard equality on arena peak across a sixteenfold
change in iterations. Measured at 256x instead:

    n           arena_peak_bytes    beat_iters
    5,000              1,048,576         5,000
    80,000             1,048,576        80,000
    1,280,000          1,048,576     1,280,000

Flat. That loop is self-recursive, threads a map, `put`s into it every
iteration and reads it with both `m["k1"]!` and `entries` — so the map is
re-derived, not merely passed along, and it brackets anyway. `beat.rs` has a
rule for exactly this, `is_scalar_map_chain`. What the cluster path lacks is a
counterpart to that rule, which is the finding recorded in the entry above.

`a_bare_list_is_or_is_not_bytes` was ignored pending "the ruling on whether
bytes are a type or a convention". The ruling landed — the interpreter has a
real bytes value and a list of small integers is never quietly one — and all
four sites answer identically on both engines.

`entry_file`'s remains ignored and remains red, which is the point of the
check: one of the five is still a live defect.

UN-IGNORING TWO OF THEM MADE A THIRD FAIL, which is a thing worth writing
down. Both `view_cache_is_returned` tests ask for 20,000 maps, cargo runs them
on separate threads, and `views` named its scratch directory for the size
alone — so one test removed the other's program mid-run. They never collided
while both were ignored. The directory is unique per call now.

## 2026-08-27 — a file module could be imported and could never import

Found while trying to repair the one `#[ignore]`d test that still fails. Its
fixture needed a third module, the third module would not resolve, and the
refusal was wrong about the facts:

    error: cannot resolve import "./helper" — a dot-prefixed path names a
    module beside the importing one, and there is no such directory or `.kso`
    file there

`helper.kso` sat beside it. Reduced to three files:

    main.kso    import "./a"          a/play
    a.kso       import "./b"          pub play = print "{b/v}"
    b.kso                             pub v = 1

`main.kso` imports the file module `a.kso` and that works. `a.kso` importing
the file module beside it does not. The same shape as directories — `a/a.kso`
doing `import "../b"` against `b/b.kso` — has always worked.

WHY. `compile_module_loaded` hands `load_dependencies` its own `dir` as the
base for resolving imports, and for a file module `dir` is the FILE. So
`resolve_import` looked for `a.kso/b` and `a.kso/b.kso`, neither of which can
exist, and reported the sibling missing while it sat there. One line:

    let base = match dir.is_file() {
        true => dir.parent().unwrap_or(dir),
        false => dir,
    };

The self-import guard behaves the same either way afterwards — a file module
importing itself and a directory module importing itself both answer
`import cycle through …`, where before this the file form could not get far
enough to say anything.

`module_differential` gains the case, which is the corpus for whole modules on
disk. It reads 18 modules now, and with the fix reverted it goes red naming
the case and quoting the diagnostic rather than through the known-defect
ledger:

    18 modules, 1 wrong; 1 known defects, 0 no longer as recorded
      a file module importing the file module beside it
        expected it to compile: error: cannot resolve import "./helper" …

WHAT IT COSTS. `compile_instructions` 57,492,931 -> 57,493,961, a rise of
1,030 on 57.5 million — 0.0018%, and it is the stat calls: one `is_file` per
module load. The counters that measure only the front end's work do not move
at all, `compile_allocs` at 61,981 and `compile_peak_bytes` at 822,004 both
identical, and no runtime vein is touched. The trend gate refuses a rise that
buys nothing, so this change takes the language escape and records itself in
bench/welfare_floor.json's history. The floor does not move: welfare reads
84.12 either way, because a thousand instructions in fifty-seven million is
below the hundredth the gate can see.

## 2026-08-27 — two questions the residual sweep could not see

The 2026-08-25 sweep walked the log, the archive and every design doc for
questions asked and never answered, and said so: "The intent is that this is
the whole of it." Today turned up two it missed, and both were missed the same
way — each was recorded in a TEST rather than in prose.

The first is `w1` in `module_differential`'s known-defect ledger: which claim
owns a qualified spelling when a module declares a name one of its imports
also exports. Filed today with its measurement.

The second is `tests/entry_file.rs`, whose `#[ignore]` reason reads "the two
conventions collide; the rule is a gavel, not a fix". The archive entry behind
it — "an err's reason renders with the compiler's spelling, not the program's",
2026-08-02 — ends with the sentence "That is a gavel." It never reached the
ledger either.

    run the file directly     trouble: slow_lane 7
    import it                 trouble: lane/slow_lane 7

Same program, same value, and which spelling you get depends on how the program
was entered. The bare-name fix was built and reverted because two deliberate
pins went red — `cross_module_fields` asserts `` `geo/label` `` in a diagnostic
and `lib/pair 6 "v"` as output, both correct for an imported type. So the rule
wanted is "render the name the asking module would write", and render has no
idea who is asking.

Filed with a recommendation: qualified everywhere, on the grounds that Go's
package rule is already ruled here and Go prints `main.T` rather than `T`, and
that the defect is entering the same code two ways and reading two things. The
cost is stated in the entry rather than glossed.

THE RULE THAT CHANGES. STATUS.md's filing gate said an entry cites its search
of the log, the archive and every design doc. It now says the tests too. A
sweep that reads only prose cannot see a question a spec is carrying, and two
of them were sitting in specs the whole time.

## 2026-08-27 — three diagnostics the coverage gate could not see

The scan that pins every diagnostic to a golden matches a message by its
leading literal run: the text before the first interpolation, which is the only
part a golden can match on. It required twelve characters of that run.

Three messages have exactly ten.

    function `{name}` has no body          src/parser.rs:581
    constant `{name}` has no value         src/parser.rs:614, 648
    the name `{}` is already taken         src/check.rs:2229, 2240

Each is raised by a program anybody could write: `fn nobody x` with nothing
under it, `answer =` with nothing under it, `type err`, a function named after
a type in the same module. None had a golden, and the gate read 81 diagnostics
and called the corpus complete.

Lowering the floor to ten admits four sites. Three are those. The fourth is
`no arm of `, which literal_arg_type already pins — I read the golden to check
the match was the message itself and not a coincidence — so the widening cost
no false pins. The gate reads 85 now, with three new fixtures under
tests/golden/errors and both arms of the already-taken message in one of them.

Nine characters is still invisible, and one message sits there: `private `{}`
is never used in its module`. No program reaches it. Both loops in
`check_unused_private` test `starts_with('_')`, the lexer refuses a leading
underscore before the parser sees the word — `leading_underscore` in the corpus
pins that refusal — and `_` alone lexes as the wildcard rather than a name, so
`type _` is turned away with "expected a type name". Its type-side sibling has
been on the excused list since the gate was turned on, which is how the check
went on being compiled after the convention it serves was retired. Deleting it
moves the compile veins, so it is a separate change.

The floor is a number the gate now depends on, so it gets a mutation:
`a_ten_character_diagnostic_arrives_without_a_golden` appends a bait whose run
is `unpinned ` and a backtick, exactly ten characters. It stops being seen the
moment the floor moves back up.

Then the excused list itself, which carries four messages and a paragraph
saying why each cannot be reached. Those paragraphs are readings of the source.
One had already been wrong once: the inline-constant claim missed heads that
open blocks and left the list on 2026-08-20. Writing the program is cheap, so I
wrote one for each of the remaining claims.

    expected a top-level declaration    REACHABLE
    expected a constant name            holds — the top-level line rule speaks first
    unused expression: …                holds — consecutive expressions fold into one Seq
    private type `                      holds — see the next entry

The indented-line excuse said the indentation rule or the blank-line rule
always takes the line first. Both do when a declaration precedes it. Put the
indented line at the top of the file and nothing precedes it, so it reaches the
top-level loop with a non-zero indent and is refused there by name. Pinned at
`an_indented_first_line`; the line leaves the list. The gate reports the other
direction too — with the line still there it says "now pinned, delete its
line" — and I watched it do so.

Goldens and two scripts — no counter moves.

## 2026-08-27 — the check a retired convention left behind

`check_unused_private` reported a `_`-prefixed declaration that nothing in its
module used. Leading underscores were retired from the language, and the check
stayed.

Both its loops test `name.starts_with('_')`. The lexer refuses any word longer
than one character that begins with `_`, before the parser sees it; `_` alone
comes back as the wildcard rather than as a word, so each of the three name
positions turns it away with its own message:

    type _      expected a type name
    fn _ x      expected a function name
    _ = 1       a top-level line must begin with `fn`, `type`, or a constant binding

No declared name in a kanso program can begin with an underscore, so neither
loop could fire and neither of its two diagnostics could be raised. The
type-side one has been on the excused list since the coverage gate was turned
on, listed as unreachable — correctly, and for a reason that made the check
itself unreachable, which nobody drew out. The fn-side one was never on any
list: its literal run is nine characters, under even the floor of ten that
today's earlier entry set, so the gate has never seen it.

The premise is pinned on both halves now. `leading_underscore` covers the long
words; `an_underscore_is_the_wildcard_not_a_name` covers the three name
positions and the message each raises. The check is deleted, its two call sites
with it, and the excused line goes.

That leaves the shape worth naming. A convention can be retired in the lexer
and leave its enforcement standing three passes away, and the thing that ought
to have noticed — a gate whose whole job is to find diagnostics nobody can
reach — had one of the two on a list of tolerated exceptions and could not see
the other at all.

The veins, from the runner's log: `compile_instructions` 57,493,961 ->
57,486,466, a fall of 7,495 (0.013%), banked. What went away is the walk —
both loops visited every declared function and every declared type on every
compile to ask a question the lexer had already made unanswerable.
`compile_allocs` holds at 61,981 and `compile_peak_bytes` at 822,004, which is
what a deletion of work that allocates nothing should read as. Welfare rises
from 84.11750548393506 to 84.11785353572103 and the floor moves with it.

The page's tagged figure moves with the golden, and it took `golden_prose`
going red on CI to say so: I regenerated the golden, re-ran welfare and the
trend gate, and carried forward a "0 drifted" from a run taken before the
regeneration. The entry above this one records the same check catching the same
omission on the same figure four hours earlier. Checking the surfaces from
memory is the failure; the checklist exists because recall does not work here.

## 2026-08-27 — two of the ratchet's excuses were to-do notes

Every gating CI job carries either a mutation that turns it red or a written
reason it has none. Eight jobs carried reasons. Six of them name something the
scratch worktree does not have — headless chrome, a jekyll build, a checkout of
kq, a second machine. Two named the mutation that had not been written:

    json decoder end-to-end   "wants a decoder answering a wrong checksum,
                               so a mutation to lib/json"
    utf-8 validator           "wants a validator the independent reference
                               disagrees with"

Both are written now, and both were watched red.

The decoder's array accumulator pushes each element twice. Every array in the
tree doubles, the top level answers three hundred and twenty rather than a
hundred and sixty, and the checksum reads 48000 where the gate wants 24000. The
element stays used, so the tree still compiles and the gate fails on the number
rather than on the build. `lib/json` is what `make_jsonbench` copies, so
patching the library reaches the built binary.

The validator's ascii prologue walks while the bytes are under 0x80 and answers
valid if that reaches the end. The bound is what makes it an ascii test.
Raising it by one admits 0x80 itself, a continuation byte with nothing in front
of it. The sweep is exhaustive under four bytes, so it reports on the first
length: `MISMATCH len=1 bytes=80 got=1 want=0`, 330,442 mismatches over
36,843,009 strings. The mutation sits in the scalar prologue rather than in
either vector body, so it reads the same on x86 and on arm — a ratchet that
depends on its host proves less than it claims.

Six excuses remain, and every one of them is a capability the worktree lacks
rather than work nobody has done.

## 2026-08-27 — the scratch worktree has chrome

Two more of the ratchet's excuses said the same thing: `site` and `browser
differential` have no mutation because the scratch worktree the ratchet builds
lacks headless chrome.

Nothing about a worktree can lack a browser. `git worktree add` makes a
directory of tracked files on the machine that already has one, the ratchet's
prove job runs on ubuntu-latest where `/usr/bin/google-chrome` sits — a path
`browser_differential_run` already searches, alongside `KANSO_CHROME` — and the
harness drives the browser itself, with no node and no `node_modules` anywhere
in the tree. So the claim was about the worktree and the obstacle it named
belongs to the machine.

Measured rather than argued: a detached worktree of HEAD, built, wasm made, and
the sweep run inside it.

    334 programs: library 211 play 76 run 47
    the tab: 334 answers, { "wasm":334 }
    PASS  327 agree, 7 known gaps, 0 disagree

Both jobs have mutations now.

`kanso_exec_main` is behind `#[cfg(target_arch = "wasm32")]`, so appending one
byte to what it hands back reaches the engine in the page and no other, which
is what makes it a divergence rather than a change of behaviour everywhere:
`FAIL 276 disagree (51 agree, 7 known gaps)`. The gate reads the corpus byte
for byte, so a defect that reaches every program is reported for every program.

docs/index.html shows an editable sample and, beside it, the output the page
promises. The two are written by hand and only a browser can compare them.
Changing the greeting leaves the promise standing and makes it false:
`FAIL the landing sample did not run: {"out":"goodbye, kanso\n"}`. Each was
watched alone, with the other reverted and the wasm rebuilt between them,
because two mutations at once prove one thing about two gates.

Adding the rows exposed a way the prove job could have lied. Both gates rebuild
`docs/kanso.wasm`, `build_wasm.sh` needs the `wasm32-unknown-unknown` target,
and the nightly job installs no targets — so the gate would have gone red on
the build rather than on the defect, and a red gate is what the ratchet reads
as proof. The job installs the target now, the way every ci.yml job that needs
it already does, and the browser row carries `release` as setup so a Rust build
failure is UNBUILT rather than red. The rule was already written down for the
ten rows that carry `release`; a new row is where it gets forgotten.

The third excuse fell to the same question. `playground examples` was excused
as "the same corpus and engines as `specs`, so a row proves the corpus", and
tests/playground.rs reads its programs out of `docs/play.js` — the EXAMPLES
object the browser tab offers a visitor — rather than from tests/golden. Two
different sets of programs, and `specs` never opens play.js. Pointing the
`hello` example at a name nothing declares gives `the interpreter failed on the
hello example: error[name]: unknown name `nobody``, with the browser-backend
test failing beside it and the golden corpus untouched.

Three excuses remain, and none of them is now a claim about which corpus is
which. The macos host runs `specs`'s own suite on another machine, so its
mutations are `specs`'s. The jekyll build is a docker action a shell in a
worktree cannot invoke. kq is not checked out beside the repository.

Four excuses remain. Two are arguments about redundancy — the macos host and
the playground corpus both run what `specs` already proves, on another machine
or through another engine. Two are genuine absences: the jekyll build is a
docker action a shell in a worktree cannot invoke, and kq is not checked out
beside the repository.

## 2026-08-27 — the loader's refusals are not diagnostics, so nothing pinned them

The coverage gate keys on `Diagnostic::new(`. The module loader and the driver
write `error: …` as plain text, print it and exit, so the scan walks past every
one of them. There are thirty-one such sites in `src/`.

Four of them are module refusals, reached by trees anybody could build:

    a.kso beside a/          import "./a" names both a directory and a `.kso` file
    a module importing itself  import cycle through …/m/m.kso
    two modules in a cycle     import cycle through …/p
    an import naming nothing   cannot resolve import "./nope" — a dot-prefixed path …

They belong on the module surface rather than in the error corpus, because the
corpus compiles one file and each of these needs a tree. `module_differential`
reads 22 modules now. Each was watched red by perturbing its expected text: the
sweep names the case and quotes what the loader actually said, so the case
cannot pass by asserting nothing.

The self-import and the mutual cycle answer the same sentence with different
tails — a file path for the first, a directory for the second — which is why
both are here rather than one standing for the pair.

Twenty-seven sites remain unseen by the gate. Some cannot be reached from a
test at all (`cannot invoke clang`, `cannot open the terminal`), and some are
driver messages a corpus of programs cannot express. Widening the scan to a
second opener would say which is which; that is a separate change, and it wants
the answer written down rather than guessed, the way the excused list's four
claims did.

## 2026-08-26 — the book answers the signature question

Clay asked the question every checked-exceptions reader asks — pass a
failure as one of many arguments, is the function forced to return an
err? — and on hearing the answer again ruled it book-worthy at high
priority. ch04 gains "nothing is asked of the signature", between the
railway and the arm rule: the call short-circuits, so the callee never
receives the failure and has no signature to infect; err-in err-out is
a fact about calls, not a contract; one compiled function serves the
failing and the clean call site alike. The panel is a new sample,
unasked.kso, whose own trace line — "passed through label" — is the
language testifying that label never ran. The multi-failure sentence
points at the compiler page's pinned table rather than re-teaching
it. The effect half of the same story — call-site lifting by the
elaborator — stays out of the book until the elaborator exists,
because the book speaks in the present tense; the queued story
carries both halves.

## 2026-08-27 — the driver's refusals are diagnostics too

The coverage gate keys on `Diagnostic::new(`. The loader and the driver write
`error: …` as plain text, print it and exit — thirty-one sites in src/ — and
the scan walked past every one of them. #1078 pinned four of the module ones on
the module_differential surface because the gate could not see them; this makes
the gate see them.

A second opener, `"error: `, read the same way as the first: cut the literal at
its closing quote, take the leading run before the first interpolation, keep it
if it is ten characters or more. The count goes 84 to 98, and the eight newly
unpinned matched the hand measurement exactly.

**Then the false-pin trap.** Six of the fourteen read as pinned and four were
false. `no .kso files in` matched tests/golden.rs's own `assert!` message,
`clang failed on` a doc comment, `cannot write` the oracle's unrelated refusal,
`cannot execute` a panic the wasm spec writes for itself. Every one came from
tests/*.rs, and every one runs long — fourteen to sixteen characters — so the
length floor was never the mechanism. The corpus was.

So the corpora split. A Diagnostic's text is pinned by a .stderr file or, for
the handful a corpus of single programs cannot express, by a Rust test, so
tests/*.rs stays in its corpus. The driver's corpus is .stderr plus
module_differential — a loader refusal needs a tree on disk, which the error
corpus cannot express. `known?` dispatches on the site's kind. That dropped
`cannot resolve import` and `import cycle through` off the unpinned list, since
#1078's cases are now visible to the gate that motivated them.

Six of the fourteen end up with a real pin. Two already had one, from #1078's
module_differential cases — pinned by hand days earlier for exactly the reason
this change removes. Four are new. `a_module_that_moved` is the first driver
message ever in the error corpus: `std/random` moved to `std/math`, and the
loader keeps the old path answering with the new one named.
module_differential gained c23, a directory holding only a README, and c24
and c25 below.

Eight excused, each with what was tried. Two are pinned by a Rust test, and one
of those tests did not exist when I wrote the citation for it —
`tests/a_plan_needs_an_io.rs` is written now, watched red against a shortened
message and green against the real one. Five fire on an io error the container
cannot produce, running as root with clang installed. One fires when clang
rejects the emitted C.

**The tenth was going to be an excuse and turned out to be a bug in my
reading.** `a module cannot import itself` had survived three constructions,
each taken first by a different check, and the honest thing to write looked
like "unreachable, or I have not found the shape". Asking a fourth time was
cheaper than writing that: the guard tests `!ENTRY_COMPILE`, and that flag is
set around the WHOLE of `compile_entry`, dependencies included. No `kanso run`
can reach it, whatever the shape. `kanso check <directory>` can — the same door
`an_empty_branch_is_refused` uses — and both arms answer there, the embedded one
for a directory named `list` importing `std/list`, the filesystem one for a
member reaching back through `../`. Both are module_differential cases now, both
watched red on a perturbed expectation. 25 modules, 0 wrong.

So the driver's excused list is eight, not nine, and the count of things I
claimed from reading the source and got wrong today is four.

The ratchet gains a row for the new arm, proven by hand first: an unpinned
`error:` write in src/main.rs takes the gate to `1 newly unpinned`, exit 1.

## 2026-08-27 — a third way the compiler writes an error

The same question one level out, asked because the second opener had just paid
off: what else writes to stderr that neither opener catches? Forty-two
`eprintln!`/`eprint!` sites in src/. Most are trace output behind a flag. The
rest are a third family, and the one that hid longest: `error[kind]: …`
written as plain text.

That is what a rendered Diagnostic looks like on a terminal. So these read to a
user exactly like a message the corpus pins, and the scan — keyed on
`Diagnostic::new(`, then on `"error: ` — saw none of them. Twenty-odd sites:
the runtime's endpoints, the stack-depth refusal, the exit-code refusals, the
repl's name lookups, the license advisory. **98 to 108 literal diagnostics.**

**This family reads the WIDE corpus, and that is measured rather than assumed.**
The driver's four false pins were short generic phrases — `cannot write`,
`cannot execute` — that a Rust test holds for a hundred unrelated reasons. An
`error[kind]:` string is a rendered diagnostic, so a test holding one is
asserting output. Six matched .stderr files (every one checked: deep_recursion,
endpoint_none, endpoint_trace, run_cannot_start and the rest); three more
matched Rust tests, and all three were checked by hand and all three were true.

Four had no pin. Two do now, and each lives where it does because the corpus it
belongs to cannot hold it:

- `error[name]: nothing named ` — the repl's `:delete` and `:show`, both doors,
  which build the message separately. tests/repl.rs; there is no repl corpus.
- `error[runtime]: the program was ended by signal 15` —
  tests/a_program_the_system_killed.rs. NOT the runtime corpus: that harness
  asserts native and `--interp` write identical stderr and both exit 1, and a
  signalled program does neither, because under `--interp` there is no second
  process to signal.

Both watched red by perturbing the SOURCE rather than the expectation, and the
repl perturbation reddened the coverage gate too, which is what proves the third
arm reads src/repl.rs at all.

`error[license]: ` was already pinned by tests/advisory.rs and is excused naming
it. The last is excused as unreachable on unix, and unlike the excuses this week
kept getting wrong, that is a claim about control flow: `ended_by_signal` is
called only from the `None` arm of `code.code()`; on unix `code()` answers None
exactly when a signal ended the process; in exactly that case `signal()` answers
Some. So the `None` arm inside `ended_by_signal` cannot be taken. It stays
because a match on an Option must be exhaustive, and its `cfg(not(unix))` twin
returns the same sentence on Windows, which CI does not run.

Third ratchet row on the job, proven by hand before it was written.

**What the widening costs.** The gate reads the same forty-two files three ways
now. Three runs each, same box, same build: 915/923/960 ms on the one-opener
version against 1086/1028/1041 ms on this one — about 119 ms, or 13%. Wall
clock, so indicative rather than pinned, and it buys twenty-five diagnostics the
gate could not see. Stated because a number that moves without a sentence is the
thing to catch.

**And one message no opener could ever see — built, measured, declined.** The
scan matches on the LEADING literal run, so a message opening with an
interpolation has none, whatever openers get added. Exactly one in src/ is in
that position: `kanso test` on a file declaring none answers `{file}: no tests
found (a test is a constant named `test_*`)`. That opening also makes it the
only driver refusal a reader cannot recognise as one; every other starts
`error: `.

Spelling it `error: no tests found in {file} ...` fixes both, and I built it —
message, `tests/a_file_with_no_tests.rs` watched red on a perturbed source, the
excused-list entry, the lot. Then the trend gate priced it:

    worsened: compile_instructions 57,486,466 -> 57,486,633
    FAIL  a pure regression: something got worse and nothing got better

**That is the correct answer and the change is reverted.** The counters cannot
see message consistency, and a change whose entire gain is invisible to them
does not get to spend them. Arguing the model is a real move and it is Clay's,
not something to do inline to unblock a pull request.

Recorded so it stays declined, and so the next person who notices the
inconsistency finds the measurement rather than repeating it.

**The measurement that killed it, and two wrong answers on the way.** The
reword is the only compiled change and it sits on a path no compile executes, so
`compile_instructions` should not move. Measured rather than assumed, under
callgrind in the fixed box, deterministic on repeat:

    this container, reword reverted   58,154,705   (= its origin/main build)
    this container, with the reword   58,154,668   — a FALL of 37
    the CI runner, with the reword    57,486,633   — a RISE of 167

**The two hosts move in opposite directions**, which settles what it is: work
that genuinely went away would go away on both. What changed is the binary's
size, and the count is of a process, so it includes what runs before `main`. A
move of a few hundred on 57.5 million can be one string literal — that is the
floor of this vein's sensitivity, and the reason to read a small move before
calling it anything.

Getting there took two wrong answers, both worth writing down because the trap
is easy and either would have banked a fake result.

The first was comparing against a build in a DIFFERENT DIRECTORY. `library_box`
already warns that the count tracks the length of the directory the compiler
RUNS in — about 160 instructions per character — so a build directory sounded
like the same hazard, and 37 sounded like the size of it. It is not: the same
tree built at `/tmp/samehead` and at `/home/user/kanso` gives the identical
58,154,668. The hypothesis was plausible, cheap to test, and false.

The second was measuring a binary I had not confirmed was fresh. The revert
build reported 1.49s, which read as "cargo did nothing", and the number came
back equal to this branch — which looked like proof the reword was free. Redone
with `md5sum` on the binary at each step, the reverted build is a different
binary and answers 58,154,705. The 1.49s was real: `main.rs` is a thin crate
over the library, so relinking it is fast. **A build time that looks too short
is a thing to check, not a thing to conclude from.**

## 2026-08-27 — three things a page does that no program ever asked it to do

`tests/golden/wasm_gaps.txt` is where a page's divergences are stated once and
checked twice — tests/wasm_engine.rs under the embedded interpreter, and
scripts/browser_differential_run under headless Chrome. It covered the
filesystem and the process families, each with programs naming them.

Three capabilities had no program at all, on either harness: `io/stdin`,
`os/args`, `time/now`. Nothing in the micro or runtime corpus read any of them.
So three things a page does went unchecked, and writing the programs found that
all three do something other than what the source says.

**`io/stdin`.** src/wasm.rs and src/wasm_rt.rs each carry `Err("the playground
has no stdin")`, written separately, meant to decline by name the way the
filesystem and process refusals do. Neither fires. A page answers
`error[runtime]: unknown builtin `stdin``, so it reports a missing BUILTIN
where every other declined capability reports a missing capability, and the
sentence written for the case has never been reached by anything.

**`os/args`.** Declared `pub args = builtin_args`, exactly the shape `stdin`
has, and it answers the same way: `error[runtime]: unknown builtin `args``.
Native and the interpreter both answer the empty list. A page honestly has no
arguments and could say so; it says something else.

**`time/now` is not a defect, and checking that is what kept the finding
honest.** A page reads zero deliberately — "no clock the differential could
agree on", said in a comment in both engines since they were written. My first
reading of the other two was that every zero-argument builtin descriptor is
routed through `call_builtin` in the compiled runtime and so cannot reach the
executor. `now` disproves it: same declaration shape, and it REACHES the
executor and gets the designed zero. Whatever routes `stdin` and `args` into
`unknown builtin` is narrower than the class, and finding it is the next step
rather than a thing to assert here.

What ships is three micro fixtures and three ledger entries, each recording what
the engine does rather than what it should — the file's own rule, and the reason
a fix turns the line red. `io/stdin` at EOF and `os/args` with none are both
deterministic and identical on native and `--interp`, which is what those
fixtures pin for the two engines that work.

    PASS  327 agree, 10 known gaps, 0 disagree   (browser, headless chrome)
    7 passed                                     (tests/wasm_engine.rs)
    micro corpus green on native and --interp

## 2026-08-27 — the kq row's excuse was wrong about the mechanism

The ratchet's rule is that every CI job carries a mutation that turns it red,
or a written reason. Three reasons were left. This is one of them, and it was
wrong — not about whether the row could be proven, but about what the job does.

    "kq specs (a real program, gating)"  —  "needs kq checked out beside this
                                            repository"

The job does not want kq beside the checkout. It CLONES it: ci.yml runs
`sh .github/clone-sibling.sh kq /tmp/kq` and then `cd /tmp/kq`. Nothing is
expected to be sitting anywhere.

Refuted by running it. In a detached worktree of HEAD, the clone works, jq is
already on the box, and kq's whole suite comes back green — unit tests, twelve
fixture goldens against jq, three cost goldens, the scale gate and the
published-numbers stamp. So the row is provable here and always was.

One real constraint the excuse never mentioned: **the clone directory must be
named `kq`.** `kanso build <dir>` names the binary for the directory, which is
the package rule, and spec.sh invokes `./kq`. Cloning to `/tmp/kqprobe` built
`./kqprobe` and the suite died with `./kq: not found`. CI already clones to
/tmp/kq, so this bites only whoever writes the row.

**Finding the mutation took three tries, and the two failures were mine.** kq
is a jq clone with its OWN JSON — query/json.kso, query/number.kso,
query/scan.kso, query/text.kso — and it never imports std/json. So corrupting
`lib/json`'s tab escape and then its exponent parser changed nothing kq runs,
and both times its suite came back green. That is not a gap in kq's coverage,
which is what it looked like before I checked; it is a mutation in code the
program does not execute.

What kq does share is `std/text`: `text/append` appears seventy times in its
query sources. So the mutation goes where ci.yml says the row's value lies —
`k_b_append_into`'s fast path, the in-place append, zeroing the first byte of
every multi-byte write. Right length, right counters, wrong contents, which is
the shape of the bug that made this job gate: an in-place concat that printed
267 nul bytes at exactly the right length. Under it kq dies with `invalid
utf-8`, born in text/utf8, and the gate exits 1.

**What the row does not claim.** `specs` catches the same mutation — three
golden tests fail under it. This proves the gate runs and reddens, not that kq
sees what the others miss. A mutation only kq catches would be a better row;
the historical one took an incident to find, and saying so is better than
implying this one is it.

Two reasons remain, and both hold: the macos row adds a second machine rather
than a mutation of its own, and the asset-digest row needs a jekyll build that
the worktree cannot do — there is no Gemfile in the tree and the CI job uses
the `actions/jekyll-build-pages` container to produce `_site`.

The mutation is written with `sed` rather than a heredoc because
`scripts/gates/python_free.sh` exists precisely to catch python creeping back
in through mutation heredocs, and it names that as the history. I wrote the
python version first and the gate would have caught it.

## 2026-08-27 — the page can read its own arguments now

#1080 pinned what a page does with `args`, `stdin` and `now`, and two of the
three answered `error[runtime]: unknown builtin`. The mechanism, traced end to
end: the wasm backend emits every builtin as a `RT_BUILTIN` call
(`src/wasm_backend.rs:947` handles the three identically), `src/wasm_rt.rs:809`
lands that on `call_builtin`, and `call_builtin` had an arm for `now` and none
for the other two. Native and `--interp` never went through that door — they
reach all three through `eval_ident`, which has had the arms since the
builtins were written.

So `now` working on wasm was a coincidence of coverage. That is worth saying
plainly because the first reading of this was "every zero-argument builtin is
broken on wasm", and `now` disproved it; the rule is narrower and duller.

The fix is one match arm covering all three, returning the descriptors
`eval_ident` already returns. What it changed:

    args    error[runtime]: unknown builtin `args`  ->  args holds 0 of them
    stdin   error[runtime]: unknown builtin `stdin` ->  the playground has no stdin
    now     the clock is past the epoch: false      ->  unchanged

`args` is fully closed: a page and native now agree byte for byte, and the
entry left `tests/golden/wasm_gaps.txt`. `stdin` stays a gap, because a page
genuinely has no stdin — but it is now the honest capability refusal that
`src/wasm.rs` and `src/wasm_rt.rs` have each carried since they were written
and that nothing had ever reached. It sits in the same family as "the
playground has no filesystem" instead of reporting a missing builtin.

Watched red first, which is the point of recording the before-state in its own
PR: with the arms in and the old entries still in place, `wasm_engine` failed
with `args_are_empty_without_any.kso is a known gap answering ... and it now
answers `args holds 0 of them` — close it or restate it`. That message is the
ledger doing its job.

    PASS  328 agree, 9 known gaps, 0 disagree   (browser, headless chrome)
    7 passed                                   (tests/wasm_engine.rs)

The gap count fell by one, which is the whole visible effect: 327/10 -> 328/9.

The compile vein moved by 251 instructions, downward, and it is layout rather
than work: `call_builtin` is the interpreter's door and `kanso check lib/json`
never enters it. Allocations and peak are identical at 61,981 and 822,004.
Banked in `bench/compile_instructions_golden.txt` with that reading written
beside it, the same way the +167 of the previous day was.


## 2026-08-27 — a hash that remembers every block it has read

`scripts/fingerprint` was OOM-killed digesting the site. The kernel's report
names the cost exactly: anon-rss 13,954,684 kB for a run whose largest input is
`docs/kanso.wasm` at 1,604,098 bytes. Ten thousand bytes of live memory for
every byte hashed.

The cost is `sha256/hex`, and nothing else on that path. Measured with
`KANSO_COUNTERS=1`, deterministic to the byte across three runs of each size:

    message   arena_peak_bytes   per byte
      1,024          7,340,032      7,168
      2,048         14,680,064      7,168
      4,096         27,262,976      6,656
      8,192         54,525,952      6,656

Twice the message is twice the peak, exactly. `text/bytes`, `text/split` and
`os/read_file` were each measured separately over the same range and are all
linear with a small constant — `text/bytes` is 9 allocations and one copy.

A hash consumes 64 bytes at a time and carries eight words of state, so its
peak should be flat in the message length. Per 64-byte block this holds a
constant 633 kilobytes and never gives any of it back.

TWO WRONG READINGS ON THE WAY, both worth recording. The first was that
`sha256/hex raw` — the string form — was cheap and flat, so the byte-list form
was the problem. There is no string form: `sha256/hex` takes a byte list, the
program errored, and a program that fails allocates nothing. The counters were
measuring a failure. The second was that the in-place append never fires, read
off `put_mut_fast=0` and `put_mut_grow=0`. Those are a different counter pair.
The ones that answer for `push` read `push_mut_fast=1,904,531` against
`push_mut_slow=125,541` at 25,000 bytes, so 93.8% of appends already take the
fast path and the optimisation is not the story.

What the counters do say: `cohort_frees=0`, and `alloc_bytes` (246,642,065)
lands within half a per cent of `arena_peak_bytes` (247,463,936). That is one
fact said twice — every byte allocated is still live when the program ends. Of
`sh_buf` reads 220,980,512 against that peak and it is TEMPTING to call that
89% of the live set. It is not: `sh_*` count bytes allocated by shape over the
whole run, and a loop whose arena stays at the one-block floor still runs
`sh_buf` up linearly. The reading that survives is the first one — nothing is
reclaimed — and the shape counters say only where the bytes went, not what is
still holding them.

EIGHT HYPOTHESES, EACH KILLED BY MEASUREMENT. Every one of these was built as a
small program and measured over three sizes, and every one holds the arena at
the one-block floor while `alloc_bytes` runs to several hundred kilobytes — so
the rewind works in all of them and none of them is the cause:

  - building the byte list at all (9 allocations, one copy)
  - a 64-element list built and discarded once per iteration
  - a list read by index while being appended to, which is `schedule`'s shape
  - a long-lived message list that every iteration indexes into
  - the same work moved behind a module boundary
  - sixty-four eight-element list literals per iteration, `compress`'s shape

Two more were tested inside `lib/sha256/sha256.kso` itself, by editing it and
rebuilding — the module is `include_str!`'d into the compiler, so a measurement
taken without a rebuild measures the old text, and the first attempt at both of
these did exactly that:

  - FORCING THE STATE ACCUMULATOR. `blocked` was given a fourth argument and
    two literal arms to dispatch on, so the folded state is demanded once per
    block rather than handed on unforced. Peak, allocations and digest all
    byte-identical. A wildcard arm does not force, which cost one more rebuild
    to learn.
  - REMOVING THE PER-BLOCK THUNK. `thunk_allocs` and `thunk_live_exit` both
    read exactly one per 64-byte block, never freed, which looked like the
    answer. Passing the schedule as a parameter instead of binding it takes
    both counters to ZERO — and peak stays at 14,680,064 and allocations at
    59,044, unchanged to the digit. The thunk-per-block was one let-binding per
    block being counted, not the memory being held.

So the cause is not any of these constructs on its own. That is worth having:
it is eight fewer places for the next person to look, and it says the leak
needs the real combination rather than any single shape in it.

The archive's entry for this module (`A digest, and the import path that broke
it`) states the design rationale: "a builtin would buy speed on a path that
runs once per built file and nothing else." That entry measured the wall clock
— 2.6 seconds — and did not measure memory. The premise is not wrong about
speed; it is silent about the dimension that turned out to matter. The same
entry records `docs/kanso.wasm` at 1,299,484 bytes, so the blob has grown 23%
since, and at seven kilobytes of arena per byte that growth cost about two
gigabytes.

The asset-digests job passes on CI, so the runner has headroom this container
did not. Nothing in the tree was watching that headroom. `tests/sha256_peak.rs`
watches it now, pinning both figures exactly and asserting the doubling; it was
watched red against a padding change before it was believed.

What to do about it is a decision rather than a patch — reclaim inside a long
call chain, restructure the module to thread one buffer, or make the digest a
builtin after all — and it is filed in design/pending-gavels.md with this
table.

## 2026-08-27 — a file that is there, readable, and not text

Three bytes: `a`, `0xFF`, `b`. Native reads them and writes them back exactly.
The interpreter refuses, and until today it said this:

    cannot read /tmp/bad.bin: no such file or unreadable

About a file three bytes long that is sitting right there. `read_file_text` in
src/eval.rs threw the reason away — `map_err(|_| ...)` — so the one thing the
message needed to say was the one thing it could not. The `|_|` was written to
CLOSE a divergence, and the comment above it says so: the interpreter used to
leak libc's `No such file or directory (os error 2)` where native said its own
fixed sentence. Fixing that by discarding the error kind traded a divergence
for a falsehood.

The two engines genuinely differ here and the difference is structural.
`runtime.c` opens the file `"rb"`, takes the bytes and hands them back;
`std::fs::read_to_string` gives Rust a `String`, which cannot hold bytes that
are not utf-8. The interpreter cannot follow native there without changing what
a kanso string is on that engine.

The differential law allows an engine to speak less than another only when the
quieter one REFUSES with a clear diagnostic. So the refusal now names the real
cause, and `ErrorKind::InvalidData` is Rust's own classification rather than a
host string, so the wording stays fixed for the reason the original comment
gives.

    cannot read /tmp/bad.bin: the bytes are not text

FOUND SIDEWAYS. `scripts/fingerprint` reads `docs/kanso.wasm`, and running it
under `--interp` reported that file as missing while native hashed it. That was
a detour off the memory measurement in the entry above, and it is the second
time today that running a shipped script by hand turned up something no gate
watched.

WHERE THE FIXTURE LIVES, and why not in the corpus. `tests/golden/runtime/`
pins a diagnostic by its stderr, and there is no diagnostic to pin: on native
the program SUCCEEDS. A corpus entry asserts one answer, and the whole finding
is that there are two. `tests/a_file_that_is_not_text.rs` holds both, asserts
each engine's own answer, and says in its own comment that it pins what the
engines DO rather than what they should — so whichever way the design question
below is ruled, one of its two assertions goes red and asks to be rewritten.

AND THE SPEC FAILED ON THE OTHER HOST, for a reason worth keeping. It wrote
its program with the fixture's ABSOLUTE path interpolated into the source, so
the length of a line of kanso became a property of the host's temp directory.
`/tmp/...` on linux fits inside the eighty characters the language allows;
macOS hands out `/var/folders/df/djsxfhc17x95674wsm_g8s980000gn/T/...` and the
line came to 99, so the run died on a formatting refusal before it reached
anything the spec meant to test. Reproduced here by pointing `TMPDIR` at a path
of the same length — 91 characters, and red. The fixture uses a relative path
and runs from its own directory now, and passes under that `TMPDIR`.

Swept for others rather than assumed unique. Six tests write generated kanso
source; the other five interpolate expressions and numbers, whose length does
not move with the host, and the ten path interpolations elsewhere in `tests/`
are environment variables, panic messages and one stderr rewrite — none of them
reaches a line the compiler will measure. So this was the only one, and there
is no gate here worth building.

WHAT IT COST, and the vein that keeps moving without work. `compile_instructions`
rose 1,954 (57,486,215 -> 57,488,169), and it is layout rather than work —
provably, this time, rather than by resemblance. `read_file_text` has exactly
one caller, the executor's `read_file`, which is an EFFECT; `kanso check
lib/json` compiles a library and runs no program, so the measured path cannot
reach the edited function at all. The counters that do measure the front end's
work are identical, allocations 61,981 and peak 822,004, and the profile's own
rows moved the way layout moves them — `__memcmp_avx2_movbe` fell 327 while the
total rose.

That makes three movements of this vein in two days from an untouched call
graph: +167, -251, +1,954. The trend gate refuses a pure regression, so this
one is attributed in `bench/welfare_floor.json` under the branch the gate
documents for a doctrine-compelled change — the differential law requires an
engine that speaks less to refuse with a CLEAR diagnostic, which is what made
this a fix rather than a preference. The attribution says plainly that nothing
was spent: welfare reads 84.12 before and after.

The design question — whether `read_file` is byte-transparent on every engine,
or text-only with a bytes reader beside it — is filed in
design/pending-gavels.md. Today the library has one reader and no way to say
which you meant.

