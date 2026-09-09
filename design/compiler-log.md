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

## The corpus says the reshape is not ready, and the earlier reading hid it

Applied on top of e12bfaba and built, `cargo test --release --test golden` reports
nine of ten tests passing and one failing. That reads like one fixture and it is
not: `error_corpus_reports_each_golden_diagnostic` asserts inside a loop, so it
stops at the first mismatch and says nothing about the rest. Driving all 193
fixtures by hand — the same staged-entry harness the test uses — puts the count
at **44 changed, 149 byte-identical**, in three classes.

**31 carry an `.imported.stderr` golden** and simply move from the loader's
`(module X)` suffix to the `--> file:line:col` form. That is the reshape working
as designed: the check now runs at the top, so the whole-program renderer writes
the diagnostic instead of the module one. (The comment above that test says 23
fixtures gain the suffix. There are 34 on disk. The comment is stale.)

**Four leak a qualified name into the message a user reads.** At the top a
dependency's declarations carry their module prefix, and these messages print
the declaration's name:

    golden:  `point` takes 2 argument(s), and a list element is one atom …
    actual:  `constructor_in_a_list/point` takes 2 argument(s), …

    golden:  these `open_start?` arms tie …
    actual:  these `an_arm_set_with_no_settling_arm/open_start?` arms tie …

with `field_of_the_wrong_record/point` and
`field_of_an_annotated_parameter/money` the same shape. The user wrote `point`.

**Three name the wrong file outright**, which is worse:

    golden:  no import offers a pub `nonexistent` to re-export
             --> a_reexport_of_a_name_nothing_offers.kso:3:5
    actual:  no import offers a pub `nonexistent` to re-export
             --> std/text/text.kso:3:5

The line and column are the user's; only the file name is wrong, so the excerpt
quoted underneath is std/text's line 3 under the user's error. `a_wall_whose_
right_side_is_a_name` lands on `std/io/io.kso:10:27` the same way.

The attribution patch is what should have prevented this, and the way it fails
is worse than not attributing at all. It hangs a file on each diagnostic through
`diag::attributing(&program.fns)`, a guard held while walking one declaration.
Every site that raises inside a `fn` walk is attributed and lands on the right
file — which is why the 31 above are correct.

The re-export check raises outside any walk, and it does not fall back: it
INHERITS. `render_across` prefers `d.file` whenever it is set, and the
thread-local still holds whatever the last walk left in it. Reproduced directly
on the smallest program — an entry importing the fixture — with the phase report
beside it:

    load a_reexport_of_a_name_nothing_offers.kso
    load std/text

    error[name]: no import offers a pub `nonexistent` to re-export
      --> std/text/text.kso:3:5

The name is the LAST MODULE LOADED. A guard that had simply been absent would
have left the file the renderer was handed standing; a leaked one overwrites it
with an unrelated library, and quotes std/text's line 3 underneath the user's
error. `src/lib.rs:3287` and `src/lib.rs:3206` are the two sites, both inside
`apply_reexport` under the re-export elevation.

So the fix is not "attribute these two sites". Reading the guard says why it
leaks, and it is a drop-order bug rather than a missing feature.
`Attributed::next` is

    self.held = Some(attributed_to(item.file()));

and the right-hand side runs first. `attributed_to` sets the thread-local to
this item's file and captures the PREVIOUS one; only then is the old guard in
`self.held` dropped, and its `Drop` writes ITS previous back. Item one sets the
file to A with previous None. Item two sets it to B with previous A, then drops
guard one — which restores None. The thread-local oscillates through the walk,
and when the iterator itself drops at the end it restores whatever the last
guard happened to be holding rather than what was live before the walk began.
That is how a std/ path is still standing when the re-export check runs.

One guard for the whole walk fixes it: `attributing` reads what is attributed
before it starts, `next` just sets the current file, and `Attributed`'s own
`Drop` restores the value it captured. A walk that ends, or returns from inside
the loop, then leaves exactly what it found — None at the top level, which makes
the re-export diagnostic fall back to the file the renderer was handed.

BUILT AND MEASURED. The corpus goes from 44 changed to 38, and all three
wrong-file fixtures come back byte-identical:

    error[name]: no import offers a pub `nonexistent` to re-export
      --> a_reexport_of_a_name_nothing_offers.kso:3:5
       3 | pub nonexistent

I had expected the fallback to name the generated `run_<fixture>.kso` and said
so. It does not: the re-export check runs during the fixture module's OWN
compile, where the file the renderer is handed is already the fixture. The stale
attribution was overriding a correct answer, not standing in for a missing one.

What is left is 38, and it divides cleanly. THIRTY-FOUR are the designed move
from the loader's `(module X)` suffix to `--> file:line:col` — 31 carrying an
`.imported.stderr` golden and three (`a_wall_whose_right_side_is_a_name`,
`fields_that_no_one_record_declares`, `sequencing_takes_two_descriptions`) whose
plain golden holds the module suffix without an imported twin, which is corpus
bookkeeping rather than a compiler question. FOUR are the qualified-name leak,
and that is the whole of what is still wrong:

    `constructor_in_a_list/point`            for `point`
    `an_arm_set_with_no_settling_arm/open_start?`  for `open_start?`
    `field_of_the_wrong_record/point`        for `point`
    `field_of_an_annotated_parameter/money`  for `money`

The guard fix is held as $S/419_guard_fix.patch.

So two bounded gaps stand between the reshape and a corpus that agrees: attribute
the module-level check sites the way the declaration walks already are, and print
a declaration's name as the user wrote it rather than as the merge qualified it.
Neither is a performance question, and neither was visible while the golden test
stopped at the first of forty-four.

The diagnostic attribution built alongside it changes no output today and is
held rather than shipped. `Diagnostic` gains an optional file filled from a
thread-local; `diag::attributing` is an iterator that holds the attribution
guard and replaces it per item, so a walk that returns early restores what it
found; `render_across` quotes the right file's line and `render` delegates to
it with an empty map. A field nothing reads is weight, and nothing reads it
while the checks stay per module.

## The check verb infers twice, and nothing sets the toggle that would skip it

`kanso check` runs `infer::infer` over the whole program a second time, at
`src/main.rs:275`, after the front end has already inferred it inside
`check_merged`. The second one is not a duplicate — it is taken after the
rewrites, so its answer differs — and its only reader is the provenance refusal
three lines below it.

The obvious tidy is to move it inside that reader's guard, so a run with
`KANSO_NO_PROV` set does not infer for nobody. That was written and built. It is
not being shipped, because the guard's condition is dead:

    $ grep -rn KANSO_NO_PROV --include=*.sh --include=*.yml --include=*.rs \
        --include=*.kso --include=*.toml .
    ./src/main.rs:276:        if std::env::var_os("KANSO_NO_PROV").is_none() {

Read in one place, set in none. No gate, no script, no test, no benchmark takes
that path, so the change saves nothing anything in this repository ever runs,
while still moving `src/main.rs` — and kanso#1325 spent two rounds learning that
an edit to the compiler's Rust moves `compile_instructions` by layout alone,
where a rise with nothing falling is a pure regression the trend gate refuses.
A coin flip on a round, for a win of zero.

So it stays out until either something sets the toggle or the second inference
can be made to pay for itself some other way. The 11,803,600 instructions that
`infer::infer` costs on the fixed corpus are what makes the second call worth
returning to; the toggle is not the way in.

## Remembering a compiled module costs more than compiling it again

bench/compile_corpus is a diamond. It imports std/text directly, and it imports
std/json, which imports std/text. `KANSO_PHASES=1 kanso check bench/compile_corpus`
prints `load std/text` twice, and the compiler really does lex, parse and check
that module once per path to it. `visited` is a cycle stack — inserted on the way
in, removed on the way out — so it never held an answer to hand back.

The memo was built: `visited` became a `Load` carrying both the stack and a map
from canonical path to the finished program, and a second visit returned a clone
of the first. Every compile counter the objective weighs got worse.

    counter                base         with the memo   delta
    compile_instructions   51,094,624   51,756,188      +661,564  (+1.29%)
    compile_allocs         29,940       32,149          +2,209    (+7.38%)
    compile_peak_bytes     777,057      1,073,098       +296,041  (+38.10%)
    compile_visits         23,723       23,522          -201
    compile_rounds         62           58              -4

(Read on this container, which sits about 400,000 above CI's row for the same
tree; the comparison is against its own base.)

The profile says why, frame by frame. The second compile of std/text is worth
about 1.13 million instructions — `compile_module_loaded'2` falls 27,087,117 to
25,961,126, `lexer::lex` 4,330,753 to 4,092,322, `parser::parse` 4,761,741 to
4,560,696. Handing the answer back costs more than that: `Vec::clone` rises
263,773 to 2,294,028 across its two frames and `Expr::clone` 328,175 to
1,592,439. A module's finished program carries every declaration its own dependencies
contributed, demoted and qualified, so the deep copy runs about 1.6 times the
compile it replaces. The peak rise is the other half:
one program per module stays alive for the whole build where before each one was
dropped as its importer finished with it.

The two counters that improved are not objective terms, and they say how small
the saving is: 201 expression visits out of 23,723, and four fixpoint rounds.

That number is the finding. The merged program is unchanged by the memo, and the
reason is where the diamond's cost actually sits: in the second copy of the
declarations, which every pass downstream then walks. Counting names in the corpus's merged program that another name
reaches through a further qualifier (`json/text/append` beside `text/append`):

    module                 declarations   reached twice
    bench/compile_corpus   428            99
    std/json               196            17

Ninety-nine of 428. Collapsing them
would mean making two spellings of one declaration into one name, and a
qualified spelling is permanent identity in this compiler today. It is
sound in principle — the lock read at the module root makes an import path
resolve to one module for the whole build — but it changes what a qualified name
means, so it is its own piece of work rather than a tidy on this one.

Declined, reverted, nothing shipped.

## What the per-module reshape costs the diagnostics, fixture by fixture

The reshape is `src/lib.rs`'s `outermost` branch: a dependency runs
`check::check_own_declarations` — the three checks that skip any name carrying a
slash — plus `canonicalize_bare_aliases`, `finish_program` and `trmc::rewrite`,
and hands its declarations up. Everything else runs once, at the outermost
compile. Against the 193 fixtures in `tests/golden/errors`, run as libraries
behind a generated entry the way `tests/golden.rs` drives them:

    155 agree   38 move   0 without a golden

Thirty-one of the thirty-eight carry an `.imported.stderr` golden, which the
corpus already keeps for fixtures whose names spell qualified through an import.
The seven with a plain golden are the ones worth reading, and they are two
kinds.

Three are an improvement. `a_wall_whose_right_side_is_a_name`,
`fields_that_no_one_record_declares` and `sequencing_takes_two_descriptions`
traded the `(module X)` suffix for a `--> X.kso:line:col`, which is the
attribution work doing what it was written to do.

Four print a qualified name where the old output printed a short one:

    fixture                          was                 is now
    constructor_in_a_list            `point`             `constructor_in_a_list/point`
    an_arm_set_with_no_settling_arm  `open_start?`       `an_arm_set_with_no_settling_arm/open_start?`
    field_of_the_wrong_record        `point`             `field_of_the_wrong_record/point`
    field_of_an_annotated_parameter  `money`             `field_of_an_annotated_parameter/money`

Measured both ways on the same fixtures rather than inferred: the base binary
prints the short name for all four when they run as libraries, so the reshape
introduces this and does not inherit it.

The mechanism is the reshape itself. Today a module's own `check_merged` runs
inside its own compile, before its declarations are qualified for an importer,
so a message that names a declaration reads the spelling the source wrote. Move
that check to the outermost compile and the same message sees the name after
qualification.

Where a fix goes. Every diagnostic now carries the file it belongs to, and a
module's qualifier is that module's short name, so one rule at render time
covers it: print a name without the qualifier that names the diagnostic's own
module, since that is the spelling the file being pointed at uses. One site,
one rule.

It does not cover all four. `an_arm_set_with_no_settling_arm` attributes to
`run_an_arm_set_with_no_settling_arm.kso`, the generated entry, because a
dispatch tie is reported at a call rather than at the arms it is about. The
qualifier there is not the diagnostic's own module and the rule would leave it
alone. That is a second defect, in where a tie points, and it wants its own
fixture.

That rule was then written and measured. `diag::spelled_in` runs at the one
place a message is rendered: it derives the qualifier from the path the
diagnostic points at — the file's stem and its parent directory's name, which
covers a module that is a directory and a module that is one file — and removes
that qualifier inside backticks only. A message quotes a name in them and a
single span can hold more than the bare name (`(mod/point …)` and `&mod/point`
are both how a message spells the fix), so the span is what it works on; prose
outside the backticks is left alone.

Three of the four then read exactly as the base spells them, spelling for
spelling:

    error[name]: `point` has no field `name`
      --> field_of_the_wrong_record.kso:21:12

The fourth is unchanged, as predicted. `an_arm_set_with_no_settling_arm` still
says `an_arm_set_with_no_settling_arm/open_start?` because the diagnostic points
at the generated entry, so the qualifier it carries is not the one this rule
derives. Where a dispatch tie points is the second defect, and it stays open.

The whole corpus was re-run with the rule in: 155 agree, 38 move, the same
numbers as without it. It repairs three messages and moves nothing else.

Then the fourth. The tie's span was already an arm's — `b.span`, the second of
the pair — so the file it named was wrong rather than the line. The walk reaches
`program.fns` by index, through a group table built earlier, and the attribution
the walks carry rides on `diag::attributing`, an iterator. An indexed walk never
touches it, so the diagnostic reads whatever attribution was last set and the
render falls back to the file it was handed. Taking `b`'s file explicitly at the
push site fixes it, and all four fixtures then read as the base spells them:

    error[dispatch]: these `open_start?` arms tie: each is the more specific one
    somewhere, and a call could match both — write the arm that is most specific
    in every position
      --> an_arm_set_with_no_settling_arm.kso:7:4

That is a class, not one site, and the class has two halves.

The near half is walks that reach declarations by index rather than through the
iterator: `check_constants` reads `arms[1].span` out of a slice it indexed,
`check_overlapping_arms` walks a filtered `Vec<&FnDecl>`, and
`check_overload_ranks` walks `windows(2)`. None goes through `diag::attributing`,
so none carries a file. The corpus surfaced only the tie because only the tie has
a fixture that crosses files; the other three want fixtures before fixes.

The far half is larger and was found by chasing the one remaining leak.
`sub_of_none` comes from `check_sub_parents`, which walks `program.types` with a
plain `for` — the shape `attributing` was written for. It cannot use it.
`stamp_file` stamps `program.fns` and nothing else, `TypeDecl` has no `file`
field, and `HasFile` is implemented for `FnDecl` alone. So no diagnostic about a
type declaration can carry a file however it is walked, and the attribution
covers half the declarations in a program.

Closing that is a real change rather than a call-site repair: a field on
`TypeDecl`, a second loop in `stamp_file`, an impl, and one `Arc` refcount bump
per type declaration — the same cost the fns side already pays, and the same
shape kanso#1324 measured when it made a module's path shared. It is the next
step on this thread.

Then the goldens, which is where the reshape actually stands or falls. All 38
movers were regenerated with the harness's own staging — `pub play` files behind
a generated entry, everything else run in place, which is what
`run_kanso_as_library` does — and each candidate compared to its golden with the
`(module X)` suffix and the `-->` block removed, so the comparison is of message
TEXT alone. Three classes came out:

    30  message text identical: only the location changed
     6  one diagnostic became two
     2  the message text itself changed

The thirty are the reshape's improvement, in bulk. The six wanted reading, and
reading them changed the count above: they are the same diagnostic twice, at the
same file, line and column.

    error[name]: no record type has a field `name`
      --> field_missing.kso:6:12
    error[name]: no record type has a field `name`
      --> field_missing.kso:6:12

Two reports of one source location is a declaration present twice in the merged
program, which is the finding the module memo turned up above — 99 of the
corpus's 428 merged declarations are a second copy reached through a further
qualifier. The reshape did not create those copies; it moved the check that
walks them from per module, where each saw one copy, to once at the end, where
one walk sees both. Under the old suffix the two reports read as different
messages, so nothing noticed.

That makes the duplicate declarations a blocker for the reshape rather than a
performance question beside it, and it is where this thread now goes.

The two are the blockers, and they are different from each other.

`sub_of_none` still leaks: `sub_of_none/missing cannot derive from none yet`,
pointing at `run_sub_of_none.kso:1:6`, the import line in the generated entry.
That is the same class as the dispatch tie — a check that raises without a file,
so `spelled_in` derives the qualifier from the entry and leaves the name alone.
It is the fourth member of the class this entry already names, and it has a
fixture, which the other three do not.

`builtin_arg_type` looked like the serious one. Its own error disappeared and a
different one took its place:

    was  error[type]: `length` takes a list, a map, or a string here, not an int
    then error[name]: `builtin_arg_type/play` is internal to the standard
         library — import its module

Diagnosed, and the first reading was wrong: this belongs to the entry below
rather than to the reshape. `resolve_name` stripped `builtin_` from a qualified
name, so a fixture named `builtin_arg_type` had every reference to it refused.
That bug predates the reshape by as long as the refusal has existed; the fixture
survived only because its own error used to be raised first and stop the compile.
With the prefix check restricted to bare names, the fixture reports its own
error again, and better than before:

    error[type]: `length` takes a list, a map, or a string here, not an int
      --> builtin_arg_type.kso:1:27

`sub_of_none` was the other, and it is fixed: `check_sub_parents` walks
`program.types`, `stamp_file` stamped only `program.fns`, and giving `TypeDecl`
a file — a field, a second loop in `stamp_file`, an `impl HasFile`, and the walk
through `diag::attributing` — makes it read `missing` at
`sub_of_none.kso:1:6`. With that in, every one of the thirty-eight is either
location-only (32) or the doubled report below (6). No message text is worse
than it was.

Nothing of the reshape shipped. The rules are dead code on main — no diagnostic
carries a file until the attribution patch lands — so they belong to its bundle
rather than to changes of their own. Five patches held; the candidates are
written out beside them. What remains before it lands is `sub_of_none`, the
thirty-seven goldens, and the compile veins measured for the whole bundle.

## A module named for what it holds could be imported and never used

`builtin_` names are how the standard library reaches the engine, and a program
that writes one for itself is refused. `resolve_name` did that by stripping the
prefix and asking whether what remained was a builtin. It asked the same
question of a QUALIFIED name: `builtin_shapes/circle` became `shapes/circle`,
which is not a builtin anyone has, so the reference was refused as internal to
the standard library. Every use of a module whose own name began with those
bytes met the same refusal, so such a module could be imported and never used.

Reproduced on an unpatched tree, two files:

    builtin_probe/builtin_probe.kso   pub hello = "hi"
    main.kso                          import "./builtin_probe"

                                      print builtin_probe/hello

    error[name]: `builtin_probe/hello` is internal to the standard library
    — import its module

The check applies to bare names now. A qualified name is a declaration in
another module, and that module's name is its own business.

The fixture is `tests/golden/micro/builtin_prefixed_names_are_not_builtins.kso`,
which the micro corpus runs twice — once directly and once as a library behind a
generated entry, which is the path that reaches the refusal. It was watched red
for the right reason: the library run produced empty stdout because the compile
was refused. The whole golden suite is green with the fix, including
`a_builtin_the_standard_library_keeps_to_itself`, the fixture that pins the bare
case this refusal exists for.

How it was found is worth recording, because the first reading was wrong. It
turned up while regenerating the per-module reshape's error goldens, where
`builtin_arg_type` had lost its own type error and gained this one instead — a
diagnostic disappearing, which the entry above called a stop. The reshape had
not lost anything. The fixture is named `builtin_arg_type`, its own error used
to be raised first and stop the compile, and moving that check later let this
refusal reach the reader ahead of it. The bug was already there and had been
since the refusal was written.

## 2026-09-08 — a projection off one box carries no sign

The `builtin_` fix went to CI three times and the compile-instructions row said
something different each time. Worth writing down, because the reasoning that
produced the wrong prediction was not careless.

Round one asked for the slash in front of the `BUILTINS` lookup:

    name.strip_prefix("builtin_").filter(|_| !name.contains('/'))

CI read 50,691,635 against a golden of 50,685,288 — a rise of **6,347**. That
one is real work, and the diagnosis was easy: every builtin reference the
standard library makes now scans its own name for a slash, and the library makes
a great many.

Round two moved the question onto the refusing arm, which a correct program
never reaches. The container this was written on read 51,095,253 for that shape
and 51,095,253 for the round-one shape — identical, to the instruction. Two
source shapes that measure the same locally, with the work provably removed from
the hot path, and the container's own delta against its baseline was a rise of
**629**. So the residue was called irreducible layout and the PR body said the
fix costs 629 instructions.

CI read 50,684,921. A **fall of 367**.

The magnitude was about right and the sign was backwards. Layout is a property of the toolchain and
the host — this container is rustc 1.94.1 on glibc 2.39, the goldens are measured
on 1.98.1 — and a layout residue measured on one box says nothing about the same
residue on another. The correct reading of the local measurement was that the
6,347 was gone and the remainder was unpredictable; instead it was read as a
number.

`bench/compile_instructions_golden.txt` is regenerated to 50,684,921 and the two
`data-golden="compile.compile_instructions"` spans on the compiler page follow
it. The rule already in CLAUDE.md — project a compile-instructions move from CI
or take the red round, never write down that it moved before CI has said so —
now has this as its worked example, and the sign is the part it costs a round to
learn.

## 2026-09-08 — the whole-program check ran over declarations the next pass deletes

`enroll_bare` gives every exported declaration of every imported module a twin
under its short name, so `import "std/list"` puts both `list/next` and `next`
into the merged program. `canonicalize_bare_aliases` then takes most of those
twins straight back out: where a bare name has exactly one qualified target and
is never locally bound, it rewrites the references to the qualified spelling and
drops the clone.

On `bench/compile_corpus` that pass declines nothing. All 83 twins go, out of
394 declarations — 21% of the merged program. And it ran at src/lib.rs:3599,
seventeen lines after `check_merged` at 3581. So the whole-program check, and
every pass reading its results, ran over 83 declarations that were about to be
deleted.

Moving `canonicalize_types` and `canonicalize_bare_aliases` in front of the
check, on CI:

    compile_instructions   50,684,921 -> 48,757,859   -1,927,062  (-3.80%)
    compile_allocs              29,941 ->     29,606        -335  (-1.12%)
    compile_peak_bytes         819,217 ->    773,818     -45,399  (-5.54%)
    front_end_visits            23,723 ->     22,426      -1,297  (-5.47%)

welfare 60.04 -> 60.21.

The container measured the instruction row at 51,095,251 -> 49,162,592, a fall
of 1,932,659, from the gate's own valgrind recipe minus its host-comparability
check — this box cannot be compared against CI's golden, but it can be compared
against itself across two builds, which is what an A/B needs. The two hosts
disagree by 5,597 on a delta of nearly two million. On a work fall of this size
the sign and the magnitude both carry across hosts; on the 629 the entry above
records, neither did, and the difference is that this one is not layout. The
allocation row agrees to the unit, 29,606 on both, because allocations count the
compiler's own algorithm.

The census that found this was looking for something else. Task #427 recorded
"99 of 428 merged declarations are a second copy reached through a further
qualifier" — the diamond's duplicates, a module reached by two import paths
contributing its declarations twice. On the compile corpus there are none of
those: `collapse_diamonds` already drops them, and every one of the 83 pairs the
census turned up is a bare twin beside its qualified original. The number was
right and the reading of it was wrong.

Nothing else changes. The alias pass removes a twin only where the bare name has
one target and no local binding, so an ambiguous bare name keeps both copies and
`check_bare_ambiguity` still sees them. The full golden suite is green,
including all 173 error fixtures and the micro corpus run twice.

## 2026-09-08 (third) — three page gates in three CI jobs, and no sweep over them

Searched the log, the archive and design/ before filing. `golden_prose` appears
once in the live log (kanso#1300's round three, where it caught a span) and
seventeen times in the archive; the closest entry is 2026-09-02, "the sweep the
page owed after four twins", which is about prose figures carrying no
`data-golden` tag at all. Neither asks whether the page gates have an entry
point.

**DONE.** `scripts/gates/all_pages.sh` runs the three gates that read the
published pages, and `tests/every_page_gate_is_in_the_sweep.rs` pins the list to
the tree.

The three are `golden_prose` (the `data-golden` spans against the goldens they
name), `page_drift` (docs/compiler.html against the log's budget of unpublished
entries) and `prose_check` (the three mechanically detectable slop families over
all 29 pages). They run in three different CI jobs — welfare, docs, and its own —
so a session editing a page has three commands to remember and none to run.

kanso#1328 is what that costs. Round two left the page's `front_end_visits` span
at 23,723 where its golden had moved to 22,426. page_drift and prose_check both
pass on that tree, because neither reads a `data-golden` span, and those are the
two I ran before pushing. golden_prose turned its own job red and welfare with
it: welfare runs golden_prose as its last step, so it reported ALREADY RED and
could prove nothing about the three rows sharing that gate.

The sweep reads all three and reports all three. Stopping at the first objection
would rebuild the failure it exists to prevent, which is the rule
`all_counters.sh` already carries for the same reason. It costs 31 seconds, 28 of
them prose_check reading 29 pages through a regexp engine written in kanso; the
figure is in the script's header so a reader waiting on it knows it has not hung.

The spec is the half that does not go stale. Eight programs under scripts/ hold a
literal `docs` path: three are the page gates and five read one for something
that is not the prose, so each of the eight has to appear as a row or in an
`elsewhere` list carrying a reason. `book_panels` and `book_quotes` are excused
to `scripts/book_check.sh`, which already runs both before it replays the sample
outputs, and the spec opens that file to check the claim rather than believing
the comment. A fourth assertion pins the property the round actually turned on:
something the sweep runs has to read a `data-golden` span, whatever it ends up
being called.

All four were watched red. Dropping golden_prose from the table fires two of
them and the message names golden_prose. Claiming page_drift takes `--write`
fires the third. Pointing book_panels' excuse at `scripts/build_wasm.sh` fires
the fourth — and the first attempt at that mutation reported GREEN, because my
`sed` anchor missed the row: it is the first line of the table and carries the
`elsewhere="` prefix. I came within one command of recording a spec that cannot
fail. A mutation is evidence only once the file has actually changed, and the
check for that is to read the line back.

Nothing in ci.yml moves. The three gates already run there, and the spec rides in
the specs job with every other `cargo test`.

## 2026-09-08 (fourth) — the entry path's reorder costs a diagnostic, and is reverted

Searched the log, the archive and design/ before filing: the 2026-09-08 entry
above records the same reorder on the MODULE path (kanso#1328), and kanso#1120
records the rule this breaks — a module is named the way an import writes it.
Neither anticipates the interaction.

**REVERTED.** `compile_parsed_entry` has the same ordering kanso#1328 fixed on
the module path, and moving its alias pass in front of the whole-program check
is worth 1,674,396 instructions. It also renames a diagnostic, so it does not
ship in that shape.

The measurement first, because it is real and the idea is worth returning to.
`enroll_bare` is called at src/lib.rs:2477, inside `load_dependencies`, so
`dep_program` carries a synthetic twin for every exported declaration of every
import — and both paths merge that program. On the entry path the twins landed
in `merged`, `check_merged` walked them, and `canonicalize_bare_aliases` deleted
them four lines later. Hoisting the two canonicalize calls out of the success
arm reads, in the box:

    entry_instructions   165,184,791 -> 163,510,395   -1,674,396  (-1.0136%)

and CI counted 162,218,854 against the container on the same tree, the two boxes
0.79% apart where the module row sits 0.81% apart on that pair.

**What it costs.** `scripts/module_differential` went from 0 wrong to 2:

    a call from the entry at the wrong arity
      refused, but not with 'error[arity]: no 2-argument arm of `one` (arms take 1)':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

The alias pass rewrites a bare reference to its qualified spelling, so once it
runs before the check, the arity refusal quotes `m/one` where the program says
`one`. That is a diagnostic naming a spelling the user did not write, which
kanso#1120 settled the other way, and it is a worse trade than a per-cent of
compile work is worth. Reverting the reorder alone takes the differential back
to 0 wrong, which is what says the reorder is the whole cause.

The module path does not have this problem because a dependency's own call
sites are already qualified by the time they are merged; the entry's are the
ones the user wrote. That asymmetry is why one of the two reorders shipped.

**A GREEN SUITE SAID NOTHING ABOUT IT, and the reason is worth having.** Before
pushing I ran the error corpus, both micro corpora, the .mem vein and
tests/reexports.rs, all green, and reported that as the correctness evidence.
`scripts/module_differential` is a kanso program run by the diagnostics-
differential CI job; `cargo test` does not run it, so no amount of the suite
would have found this. It is the same shape as the page gates — a check that
lives outside the harness a session reaches for by habit. The nine differential
sweeps each have this property.

What would make the reorder shippable is deciding what the arity refusal should
quote when the pass that rewrote the name has already run: either the check
reads the pre-canonical spelling, or the pass records what it rewrote. That is a
design question about diagnostics rather than about ordering, and it is where
this thread now sits.

## 2026-09-08 (fifth) — the entry path compiles and nothing counted it

Searched the log, the archive and design/ before filing: `compile_parsed_entry`
appears in the 2026-09-08 entries for kanso#1324, #1325 and #1326 and in
kanso#1329's revert, and every one of them measures it on a corpus that lived in
a session's temporary directory. None of them asks why there is no vein.

**The two compiles.** `kanso check <directory>` is a module and goes through
`compile_module_inner`. `kanso check <file>` with a top-level expression is an
entry and goes through `compile_parsed_entry`, which merges the imports itself
and runs its own whole-program check at src/lib.rs:160. Proved by probe, not by
reading: an `eprintln!` at the entry site fires once for
`bench/entry_corpus/main.kso` and not at all for `kanso check
bench/compile_corpus`.

Every compile gate in the tree checks a directory. So the call at line 160 was
watched by nothing, and `KANSO_PHASES` cannot separate the two — `load_dependencies`
compiles each import through the module path, so a phase report over an entry is
the union of both.

**What that cost.** kanso#1326 projected a RISE of 629 from this container and CI
read a FALL of 367. kanso#1324 and #1325 landed on numbers no CI job could
reproduce. Two further findings — the synthetic-twin skip below and kanso#1329's
reverted reorder — were measured and could not be landed against anything.

**The vein.** `bench/entry_corpus/main.kso` names ten imports and uses each,
following bench/compile_corpus's rule that a workload is named rather than
inherited; it imports ten where the compile corpus imports four, because the
entry path's own work is the merge and the check over everything the imports
bring and a corpus with one small import measures mostly the module path
underneath it. `scripts/gates/entry_instructions.sh` counts it the way
`compile_instructions.sh` counts its own — same box, same emptied environment,
same pinned tunables, same `kanso::main` anchor — and every reason for those is
left in the original rather than restated.

`bench/entry_instructions_golden.txt` opened holding zero, because the row is
CI's and this container reads high against CI's rustc. Round one was red on
purpose and CI answered **163,886,731**, on binary sha 3c53d0acdbcb — the same
sha that counted `compile_instructions=48,757,859` in the same job, so both
rows answer for one build. The container had projected 165,183,406 from the
same recipe minus the host check: 1,296,675 high, or +0.79%, which is the
offset already recorded between rustc 1.94.1 here and CI's 1.98.1. The
projection was right about the size and could not have been recorded as a row.

**And the trend gate did not walk the new golden.** Found by asking which
files in bench/ `scripts/trend_gate/trend_gate.kso` names, which its own
comments say is the only method that has ever found one of these. The list has
been short five times: three cost goldens nobody entered, then readbench —
whose golden the gate could not see while two of its rows were welfare terms —
then livebench, then the consolidated run program. This would have been the
sixth, in the very PR that exists because a compile the gates could not see
went unpriced.

Two files in bench/ are unwalked and one of them belongs that way:
`bench/compile_libraries_golden.txt` holds five sonames rather than counters
and its own gate diffs it byte for byte. So the excuse list is one line long,
and `tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads bench/
off disk, asks the gate which files it names, and fails on anything neither
walked nor excused. Its three assertions were each watched red: dropping the
entry golden from the gate names it in the failure; an excuse for a file that
is not there fires the second; an excuse for a golden the gate already walks
fires the third. Finding this by hand a sixth time was not a plan.

**The ratchet row separates the two veins, measured.** The mutation asks the
entry's whole-program check twice. In the box:

    entry_instructions   165,183,406 -> 190,382,616   +25,199,210  (+15.25%)
    compile_instructions  49,162,592 ->  49,162,592   byte-identical

A vein whose defects another vein already catches would not be worth its
callgrind run. This one is worth it: the compile row cannot move for a defect on
this path however much work it does.

**MEASURED, NOT SHIPPED — the synthetic-twin skip.** `enroll_bare` gives every
exported declaration of an imported module a twin under its short name, cloning
the whole declaration, body and all. On the entry corpus that is 145 of 882
declarations and 155 of 1,035 statements, and the whole-program check walks both
copies. It is visible as a defect only under kanso#1329's reshape, where six
error fixtures reported one diagnostic twice at the same line AND the same
column — `field_missing/play` and its twin `play`, both at span 4 of the same
file, each answering `check_field_exists` once. The module path has not had this
since kanso#1328: `canonicalize_bare_aliases` takes the twins out before the
check there, which is why `compile_instructions` reads synthetic=0 and this whole
thread is invisible to it.

Skipping synthetic declarations in the three checks that are pure body walks —
`check_build_blocks`, `check_none_in_collections`, `check_field_exists` — was
built and measured in the box:

    entry_instructions   165,183,406 -> 164,922,557   -260,849  (-0.1579%)
    compile_instructions  49,162,592 ->  49,170,337     +7,745  (+0.0158%)

The entry row falls 34 times what the compile row rises, and the compile row's
rise is the branch itself plus layout — the module path has no twins to skip.
**Welfare reads the compile row and not the entry row**, so by the objective as
it stands today this change is a small loss. That is a question about the
objective's inputs rather than about the change, and it is not settled here: the
skip is left out of this PR, and what lands is the vein it would be measured
against. Recorded as OPEN.

The remaining 0.85% of kanso#1329's reverted reorder is inference, which is 22.6%
of the entry compile against `check_merged`'s 38.3%. `infer` indexes declarations
positionally — `vec![0; program.fns.len()]`, groups by index — so it does not take
a `continue`, and skipping the twins there is a different change from this one.

- **DONE** — the entry vein: corpus, gate, golden with CI's row, CI step and
  summary row, ratchet row, sweep membership, the trend gate's own list and
  the spec that keeps it honest. The derivations in
  `tests/the_compile_sweep_names_every_compile_gate.rs` walked past
  `bench/entry_*` and now do not, and `the_compile_row_holds_one_value` covers
  both instruction goldens rather than one, because the one-row-one-value ruling
  is about the shape of a row and not about a filename.
- **OPEN** — the synthetic-twin skip, measured above, held on the objective
  question: should welfare's compile term read the entry path as well as the
  module path? Two compiles, one term.
- **OPEN** — the twins inside `infer`, worth most of the remaining 0.85%.

## 2026-09-08 — the corpus change banked a six-point fall, and the precedent says it should not have

Clay, reading the published chart on kanso-lang.dev/numbers: why didn't the new
corpus ruling go retroactive, so there is no drop? The answer is that kanso#1321
left the three compile baselines where they were, and the score fell 6.29 points
for a change that touched no compiler code.

    2026-09-08 01:53   welfare 66.0241 -> 59.7360      #1321
    compile_instructions   19,316,962 -> 52,603,220
    compile_allocs             11,613 ->     31,596
    compile_peak_bytes        375,222 ->    789,740

**The reason #1321 gives.** "The baselines those three are divided by were taken
on lib/json and are left where they are: the index has an arbitrary origin and
only its direction and the size of its moves mean anything, so re-deriving a
historical compiler's cost on a corpus that did not exist then would buy
nothing. This is a change of origin, not a regression."

**The origin is arbitrary; the move is not.** That sentence is the argument
against leaving it. A reader of the chart sees a six-point fall, and a fall is
what the index says a change made worse. CLAUDE.md is explicit: "Moving the
floor to accommodate a change while leaving the weights alone is declaring the
objective wrong without saying so."

**Re-deriving history was never what the precedent asks for.** It asks for one
measured factor per row, taken on the same head under both definitions -- which
#1321 already measured and recorded. Scaling each baseline by its own factor
restores each ratio exactly:

    row                     factor   ratio left as-is   ratio re-based   before
    compile_instructions    2.7232         1.0753           2.9282       2.9282
    compile_allocs          2.7207         1.9658           5.3483       5.3483
    compile_peak_bytes      2.1047         1.0373           2.1833       2.1833

    baselines: 56,563,967 -> 154,032,855 · 62,110 -> 168,985 · 819,217 -> 1,724,228

Every ratio returns to the digit it held before the corpus moved, so the score
does not move and the chart is flat across the change.

**Three precedents, all in this repository.** The archive: "RE-BASELINED THE
SAME WAY #729 WAS: `basket_allocs` scaled by exactly the factor", and
"RE-BASELINED SO IT BANKS NOTHING, the same method as #729 and #741." The floor
file's own history at 73.53: "the compile row counts the compiler's own frame;
the baseline is re-based by the same 465,864." And at 84.51, scanbench entering
the corpus: "The score does not move on entry, by design."

**What is not in dispute.** #1321 is right that a term measured on a library
moves whenever that library changes its imports, and the fixed corpus is the
ruling of 2026-09-08. Nothing here argues against the corpus. The question is
only whether the change of measurement banks a fall, and the answer this
project has given three times is that it does not.

**OPEN, and cloud's**, since the baselines and the floor are code. Two readings
are available and both were taken on the changeover head, so no re-measurement
is needed. If the fall is kept deliberately, that is a claim about the weights,
which CLAUDE.md says is settled before the floor moves rather than after.

## 2026-09-08 (second) — the compile term read one compile out of two

Searched the log, the archive and design/ before filing: the re-basing precedent
is the 2026-09-05 entry for kanso#1242 and the archive's #729 and #741; the entry
vein opened in kanso#1330, whose own entry above closes with this as an OPEN
question — "should welfare's compile term read the entry path as well as the
module path? Two compiles, one term." This answers it.

**The term summed one path.** `kanso check <directory>` takes
`compile_module_inner`; `kanso check <file>` with a top-level expression takes
`compile_parsed_entry`, which merges the imports itself and runs its own
whole-program check. Every `kanso run` takes the second, and nothing counted it
until kanso#1330. The compile term now adds the two rows:

    compile_instructions   48,757,859 + 163,886,731 = 212,644,590

**The baseline moves with it, so the score does not.** The entry vein has no
reading at this objective's epoch, because its corpus did not exist then, so its
baseline is imputed at the ratio the module row holds:

    r = 154,032,855 / 48,757,859 = 3.1591390221
    entry baseline    163,886,731 * r = 517,740,967
    summed baseline   154,032,855 + 517,740,967 = 671,773,822
    summed ratio      671,773,822 / 212,644,590 = 3.1591390216

The preservation is algebraic — `(cb + ec*r) / (cc + ec) = r` for any `ec` — and
it was measured rather than trusted: welfare reads 66.29 against a floor of 66.29
before and after.

**What the shape of the change turned out to be.** The plan recorded for this
work said it was one line in `bench/objective_sources.txt`, and that was wrong.
welfare does not build its counters from that file; it reads the goldens itself,
in `fn measured` and a reader chain, and objective_sources is the LINK that the
trend gate's `shifted?` and `tests/the_objective_reads_what_the_gate_watches.rs`
replay. Both halves are needed and they are different files. The spec was watched
red before it passed: with the second key removed it reports `compile_instructions
reads 212644590 from welfare and 48757859 from compile_instructions`.

Two smaller things the edit forced. The compile veins now reach `measured` as one
list rather than as four positional arguments, because a fourth `compile[4]!` at
the call site overflows the 80-column rule by three characters; the next golden
to join is now one list entry and one binding. And the binding is `ent`, because
`entry` is bare-enrolled from an import and the resolver refuses to shadow it.

**golden_prose needed the same golden and would not have said so.** Its
`golden_for` answers for `decode`, `encode` and `compile` and returns `[]` for
any other family, so a page span written `data-golden="entry.entry_instructions"`
resolves against an empty golden — a span nothing watches, which is the exact
failure that gate was widened to fix in kanso#1047. The entry row joins the
`compile` family instead, whose own comment already licenses it: the key names do
not collide.

**Still to ship: the twin skip.** kanso#1330 measured it and deliberately left it
out, because the objective could see the module row's +7,745 and not the entry
row's −260,849. Under the sum it is a fall of 253,104, −0.1190%. It is a separate
change because it moves two goldens whose values are CI's, and this one moves no
counter at all.

## 2026-09-08 (third) — the twin skip ships, now that the objective can see it

Searched the log, the archive and design/ before filing: this thread is the OPEN
item at the end of the kanso#1330 entry above ("the synthetic-twin skip, measured
above, held on the objective question"), and the entry above that, for kanso#1331,
answered the question it was held on. The archive's prior art is `enroll_bare`
and `canonicalize_bare_aliases` in the kanso#1328 entry. Nothing else is new.

`enroll_bare` clones every exported declaration of an imported module under its
short name — body and all, `synthetic = true`. On `bench/entry_corpus` that is
145 of 882 declarations and 155 of 1,035 statements. Three checks in
`check_merged` are pure body walks and were the only three of sixteen that read
those clones: `check_build_blocks`, `check_none_in_collections` and
`check_field_exists`. Thirteen others already skip them. These three now do too.

CI's rows, which are the ones this vein may hold:

    entry_instructions   163,886,731 -> 163,612,976   -273,755  (-0.1671%)
    compile_instructions  48,757,859 ->  48,761,165     +3,306  (+0.0068%)

Under kanso#1331's summed compile term that is

    212,644,590 -> 212,374,141   -270,449  (-0.1272%)

and welfare moves 66.2874470488728 -> 66.28984813328917, banked in this PR.

**compile_instructions RISES to 48,761,165 and that is the change's own doing.**
The module path has had no twins to skip since kanso#1328 put
`canonicalize_bare_aliases` in front of the check there, so the walk does the
same work and now pays for a test that can never say yes; src/check.rs is the
compiler, so its bytes and the layout under them move with the edit as well.
The row is traded against the entry row's fall, which is 83 times it.

**The container projected the deltas and got the digits wrong in both
directions**, which is the kanso#1326 lesson again. It read 165,183,406 ->
164,922,557 for the entry row (-260,849) against CI's -273,755, and +7,745 for
the module row against CI's +3,306. Sign and order of magnitude carried across
rustc 1.94.1 here and 1.98.1 there; nothing finer did. Both rows are copied out
of the job log.

The compile row rises because the module path has no twins left to skip — since
kanso#1328 `canonicalize_bare_aliases` deletes them before the check there — so
what that row records is the branch itself plus layout. Under kanso#1331's summed
term the entry row's fall is 34 times it, and the trade lands the right way up.

A twin's body IS the original's body under a second name, and both copies are in
the same merged program, so a twin can answer nothing the original answers
differently. That is why no diagnostic moves. The doubling was visible once, under
kanso#1329's reverted reorder: six error fixtures reported one diagnostic twice at
the same line and the same column, `field_missing/play` and its twin `play`. On
current main the dependency's own compile refuses first, so the second copy never
reaches a reader — which is why this change has counters and no fixture.

`infer` is deliberately not given the skip. It indexes declarations positionally
(`vec![0; program.fns.len()]`, groups by index), so a `continue` misaligns it.
That is the rest of kanso#1329's reverted reorder and stays open.

## 2026-09-08 (fourth) — a compile left its IR behind, and the guard for it read the litter

Searched the log, the archive and design/ before filing: the archive names
`cached_program_binary` once, in the entry that introduced the per-pid IR path
after concurrent builds segfaulted inside clang. Nothing records the leak, and
nothing else in the tree measures what a `kanso run` leaves in the temp
directory.

`kanso run` caches its binary under a hash of the IR and `runtime.c`. On a miss
it writes the IR to `kanso_run_<key>_<pid>.ll`, hands that to clang, and renames
the staging binary into place. The `.ll` was never removed. One cold run in an
isolated TMPDIR leaves four files and exactly one of them is a leak:

    kanso_run_<key>                     185 KB   the binary cache, intentional
    kanso_run_<key>_<pid>.ll             42 KB   THE LEAK, one per cache MISS
    kanso_runtime_dev_<hash>.c          415 KB   content-keyed, shared, bounded
    kanso_runtime_dev_<hash>.o          258 KB   likewise

At 42 KB a miss this reached about 112,000 files in one long-lived container,
which is the whole of a session's disk allowance, and it was the true cause of
four "spec failures" chased as real during kanso#1330. The staging path needs no
removal because `rename` consumes it.

**The obvious fix blinds a real guard, which is why this took two attempts.**
`tests/concurrent_build.rs::two_builds_of_one_program_do_not_share_a_file`
proved that two concurrent builds are handed different paths BY FINDING THE
LEFTOVER `.ll` AND READING A PID OUT OF ITS NAME. Delete the file and the guard
has nothing to look at; the first draft of this change shipped the removal, the
guard went green on an empty set, and the whole thing was backed out. A guard
resting on a bug fails the moment the bug is fixed.

So the guard now watches during the build rather than counting what is left. The
IR is written before clang starts and removed after it returns, and a cold run of
that fixture is ~133 ms with ~100 ms of that window, against a one-millisecond
poll. It cannot pass vacuously: seeing no IR file at all is a failure with its
own sentence, because "the race never happened" and "the race happened and was
safe" must not look alike.

**Why the sibling is not enough on its own**, measured rather than assumed. With
the pid stripped back out of the path, this guard caught the defect in 10
sittings of 10 and again in 5 of 5 after the rebase, where
`many_builds_of_one_program_all_answer` caught it in 9 of 10 — it passed once
with the bug in place, because whether two processes actually overlap on the
file is the race, and the race is not owed to anyone. One of the two is a
corruption that may or may not happen; this one is the decision that allows it.

No counter moves. `kanso run`'s temp handling is not on any measured path: the
compile veins run `kanso check`, which never reaches `cached_program_binary`.

## 2026-09-08 (fifth) — the entry reorder re-derived, and the alias pass refuses a valid program

Searched the log, the archive and design/ before filing: the 2026-09-08 entry
"the entry path's reorder costs a diagnostic, and is reverted" is this same
change, and the entry after it records the vein that now measures it. The
archive's `canonicalize_bare_aliases` entries are about what the pass costs, not
about when it runs. What neither has is the failure set as it stands today.

**DECLINED, a second time, on wider grounds than the first.** Hoisting
`canonicalize_types` and `canonicalize_bare_aliases` out of
`compile_parsed_entry`'s success arm is worth, on 9ec9f7e9 in this box:

    entry_instructions   164,922,557 -> 163,362,291   -1,560,266  (-0.9461%)
    compile_instructions  49,170,337 ->  49,170,337            0

against kanso#1329's -1,674,396 (-1.0136%) for the same edit a few merges
earlier. The module row is byte-identical here, which is what a
`compile_parsed_entry`-only edit should read; CI has not been asked, so that is
a projection.

**The failure set has moved since the revert.** `scripts/module_differential`
reads 0 wrong on 9ec9f7e9 and 2 wrong with the reorder, and only one of the two
is the one kanso#1329 recorded:

    a call from the entry at the wrong arity
      error[arity]: no 2-argument arm of `m/one` (arms take 1)
      where the program says `one`

    a type and a function sharing a name
      expected it to compile; got
      error[opacity]: `m/thing` is foreign -- only `m` builds a `thing`

The sibling-arity case kanso#1329 also lost now passes. In its place is a
program that compiled before the reorder and does not compile after it, which is
a worse thing than a diagnostic quoting the wrong spelling. `m/b.kso` declares
`pub fn thing _`, the entry writes `print "{thing 0}"`, and the answer should be
`m/thing 0`.

**Both objections are the alias pass, not the type pass.** Moving
`canonicalize_bare_aliases` alone and leaving `canonicalize_types` in the success
arm leaves the differential at the same 2 wrong, and reads slightly BETTER than
moving both:

    entry_instructions   164,922,557 -> 163,353,361   -1,569,196  (-0.9515%)

8,930 better than the pair, because the alias pass deletes the twins before
`canonicalize_types` walks them rather than after. `thing` in `print "{thing 0}"` is a CALL, so the alias pass rewrites it to
`m/thing` like any other bare name, and the opacity check then reads a qualified
name as a foreign type construction. One pass, one mechanism, two checks that
read a name after something else has rewritten it.

**What would make it shippable**, stated more narrowly than kanso#1329 could:
`check_merged`'s opacity and arity checks have to see the spelling the program
used. The obvious inverse map from `aliases` is unsound -- it is keyed by name,
so it would also rewrite the diagnostic for a call the user really did write
qualified. The per-site record was then built as a probe and measured, and it is
affordable:

    alias-only reorder, no record   163,353,361
    with the per-site record        163,380,706   +27,345

1.7% of the prize, leaving -1,541,851 (-0.9349%) against 164,922,557. And the
27,345 is not the recording. THE PASS REWRITES NOTHING ON ANY MEASURED CORPUS: a
counter at the rewrite site reads 0 sites on bench/entry_corpus, 0 on
bench/compile_corpus and 0 on lib/json, against 1 on the `m/thing` fixture that
draws the opacity refusal. The vector never allocates, so what the 27,345 buys is
an extra parameter carried through a recursive walk over every expression in the
program, and a shape that hangs the recorder off a walker rather than threading
it should cost less. Two things a real implementation must handle that the probe
did not: the reader half in check.rs, and the second caller of the same walker at
src/lib.rs:2616, which walks with the door map.

**And the two readers want different things, which reading `foreign_constructions`
settles.** Its own comment states the invariant the reorder breaks, at
check.rs:1847: "A qualified name can never be a local binding, so unlike the
arity walk beside it this needs no shadowing set: the slash IS the foreignness."
That holds only while every slash in the merged program was written by a person.
After the alias pass has run a slash also means the pass put one there, and the
check fires on `m/thing 0` -- a call of an imported function -- as though it were
a construction of the imported type of the same name. So opacity does not want a
spelling to quote. It wants to SKIP a head the pass rewrote, because that head
was never a construction. Arity is the one that wants the spelling. One record,
two uses, and a fix that handed both readers the old name would leave the opacity
refusal exactly where it is.

This is not a gavel: the
substance was ruled in kanso#1120, a diagnostic names what the import writes, and
which mechanism satisfies it is the implementer's.

**How this came to be built twice, since the answer is a process one.** The task
list carried it as BUILT AND PROVEN with a full `cargo test` behind it. The
kanso#1329 entry names that exact evidence as worthless here --
`scripts/module_differential` is a kanso program run by the diagnostics-
differential CI job and `cargo test` never invokes it -- so the suite was green
both times and said nothing both times. The filing search caught it before the
branch was pushed, which is what the search is for.

- **DECLINED** — the entry path's alias-pass reorder, in any shape that leaves a
  check reading a rewritten name. Re-measured, re-refused, and this time the
  opacity refusal is on the record beside the arity one.
- **OPEN** — the pre-canonical spelling for `check_merged`'s two name-reading
  checks. Worth -1,569,196 on the entry row in the alias-only shape, and
  -1,541,851 with the per-site record that makes it sound. What is unbuilt is
  the reader half: the two checks in check.rs that have to consult the record
  instead of the node, and a fixture for each.
- **OPEN, unchanged** — the twins inside `infer`, which is the other half of the
  reorder's value and is blocked on a different thing: `infer` indexes
  declarations positionally, and a group keyed by (name, arity) is a dispatch
  group, so the twin is what lets a bare name resolve.

## 2026-09-08 — the welfare column spans four measurement epochs and the rewrite scores them on one ruler

Clay, reading kanso-lang.dev/numbers after kanso#1331 landed: "I still have
no clear accounting of why the welfare went down and the website still does
not look great." The account, read off the rewritten column on
origin/perf-history:

    2026-09-06 11:26   91.57 -> 58.96   -32.62   #1284  one consolidated run program
    2026-09-08 01:53   70.03 -> 67.75    -2.27   #1321  the compile corpus is named
    2026-09-08 10:17   67.91 -> 66.29    -1.63   #1331  the compile term sums both compiles

None of the three is the compiler getting worse. Each is a change in what is
measured, and the column still steps at each one because
`scripts/welfare_rescore` scores every row against the single baseline the
floor file holds today.

**The mechanism.** The rewrite exists so the column is "rewritten under one
formula whenever the formula moves, which is what makes two points on it
comparable" (docs/numbers.html). One formula does make rows comparable when
the WEIGHTS move. It does not when the MEASUREMENT moves, because the counters
change magnitude while the baseline does not. Today's compile baseline is
671,773,822, and the rows it divides come from four epochs:

    epoch                           compile_instructions   ratio    term
    lib/json, one compile                 19,316,962       34.78   0.9858
    compile_corpus, one compile           52,603,220       12.77   0.9623
    compile_corpus, module row            48,757,859       13.78   0.9650
    compile_corpus, both summed          212,644,590        3.16   0.8634

Adjacent epochs differ by a factor that is the workload and never the
compiler, and the term falls across each boundary by that factor. The -2.27
and the -1.63 are those two falls, weighted. #1331 re-based the floor so the
row it wrote is right; it could not re-base the rows before it, because the
rewrite has no notion of an epoch to re-base them to.

**The run side is the same disease and the larger cliff.** #1284 re-based the
run counters to parity at the changeover, so every row before it scores its
run terms against a baseline it never carried. Clay ruled the repair on
2026-09-07: share-weighted phases, renormalised over the phases a row carries,
based at row 70 (2026-08-10), and the ruling closes with "the rewrite is
cloud's." As of this entry, `scripts/` holds no share-weighted reconstruction.
The -32.62 on the chart is that ruling unbuilt.

**What makes the column flat across a change of measurement.** The rewrite
needs an epoch table: for each change of measurement, the head it happened at
and the per-row factor measured there. The corpus move's factors are already
recorded in the floor file (2.7232, 2.7207, 2.1047) and the summing's is
computed in kanso#1331. A row is then scored against the baseline scaled to its
own epoch, which is one re-basing per epoch, applied in the rewrite rather than
only in the floor file. It is what the three floor-file precedents did by hand
for the current row, done for every row.

**OPEN, cloud's, two items.** Build the 2026-09-07 ruling for the run side.
Apply the same per-epoch scaling to the compile side's two changes of
measurement. Until both land, the chart shows the history of what was measured
rather than the history of the compiler, and no reader can tell which.

## 2026-09-08 (sixth) — the pre-canonical spelling, and the entry reorder ships

Searched the log, the archive and design/ before filing: the entry above
("the entry reorder re-derived") is this thread's own, and leaves exactly this
as OPEN with the recorder measured and the reader half unbuilt. kanso#1329
records the first revert, kanso#1328 the module path's reorder, kanso#1120 the
ruling both readers have to satisfy. This builds what that OPEN item names.

**The reorder ships, with the two readers that make it honest.**
`canonicalize_bare_aliases` runs in front of `check_merged` on the entry path
now, so the whole-program check no longer walks the synthetic twins the pass is
about to delete. `canonicalize_types` stays in the success arm, which is the
cheaper of the two shapes by 8,930 instructions.

The pass returns a `Rewrites` — line and column to the bare name it replaced —
and `check_merged_after_aliases` hands it to the two checks that read a call's
name. Every other caller runs the pass after the check and passes an empty
record, where both readers behave as they always did.

**The two readers want different things, and that is the whole finding.**

    check.rs arity (two sites)   quotes the recorded bare name
    foreign_constructions        SKIPS a head the pass rewrote

Arity is a wording question and kanso#1120 settles it: the diagnostic names what
the import writes. Opacity is not. Its own comment states the invariant, at
check.rs:1847 — "A qualified name can never be a local binding, so unlike the
arity walk beside it this needs no shadowing set: the slash IS the foreignness."
That holds only while every slash was written by a person. After the pass, a
slash also means the pass put one there, and the check fires on a call of an
imported function as though it were a construction of the imported type of the
same name. No wording of that message is right; the site is not a construction
at all.

**On scripts/module_differential: 0 wrong, from the 2 wrong the reorder cost
before.** Both objections are gone, and both readers were watched red on their
own:

    opacity skip disabled   1 wrong -- `m/thing` is foreign, on a program that compiles
    arity spelling disabled 1 wrong -- quotes `m/one` where the source says `one`

Each mutation loses exactly its own fixture and no other, so neither reader is
dead code and neither is doing the other's work.

**What it costs, in this box.**

    entry_instructions   164,922,557 -> 163,499,802   -1,422,755  (-0.8627%)
    compile_instructions  49,170,337 ->  49,207,870      +37,533  (+0.0763%)
    summed                214,092,894 -> 212,707,672  -1,385,222  (-0.6470%)

The module row rises for the same reason kanso#1332's did: the path pays for
something it cannot use. Its record is always empty, and what it pays is a
parameter carried through `arity_walk_expr` and `foreign_constructions`'s walk,
both recursive over every expression. Neither lookup runs on a clean module
compile -- the arity one sits inside the refusal branch and the opacity one
behind a name being in the foreign set -- so the cost is the threading, not the
reading. Under kanso#1331's summed compile term the trade is 38 to 1 in favour,
and the sum is what the objective reads.

Against the reorder measured WITHOUT the readers (163,353,361), the readers cost
the entry row 146,320. The probe in the entry above put the recorder alone at
27,345; the rest is the two further walkers now carrying the same parameter.

These are container numbers and none of them is a row. CI counts both compile
veins, and this branch expects a deliberate red first round for exactly that.
**CI's rows, and what the container got wrong about them.**

    entry_instructions   163,612,976 -> 162,170,772   -1,442,204  (-0.8814%)
    compile_instructions  48,761,165 ->  48,791,172      +30,007  (+0.0615%)
    summed               212,374,141 -> 210,961,944   -1,412,197  (-0.6650%)

Welfare 66.2898 -> 66.30, ratcheted in the same change.

The container projected -1,422,755 and +37,533. Sign and order right on both,
digits wrong on both, and the two errors ran the same way: it UNDERSTATED the
entry fall by 19,449 and OVERSTATED the module rise by 7,526. Its standing
offset is +0.8% on the LEVEL of each row, so the naive expectation was that it
would overstate a fall; a level offset between toolchains does not carry to a
delta, and this pair is the demonstration. Under the summed term the trade is
48 to 1 in favour, against the 38 to 1 the container projected.


**The module path had both defects live, and nothing in the tree asked it.**
kanso#1328 moved the same pass in front of the same check on the module path
three days before this, and handed the check nothing. So on main today:

    kanso check <a module>   opacity REFUSES a program that compiles
    kanso check <a module>   arity quotes `m/one` where the source says `one`

Reduced, that is a module whose sibling declares `pub type thing` beside
`pub fn thing _`, importing it and calling `thing 0`. The entry-path form of
exactly that program is c7 in scripts/module_differential, and it was watched
through both reverts of the entry reorder; the module form had no case at all,
so the sweep read 0 wrong on a defect it could not see. The fix is the entry
path's, and both programs go into the sweep as c26 and c27 -- watched red on the
pre-fix compiler for the two messages above, verbatim, before they went green.

Threading the module path costs almost nothing because it was already paying:
`check_merged` built an empty `Rewrites` on every call, and the change replaces
that construction with the real one. Entry +121, compile +63 against the
readings in the table above, both already folded in.

**And the third path is now watched before it moves.** `kanso check` on a single
library file takes `compile_library`, which still checks before it canonicalizes
-- so both readers are right there today. c28 and c29 say so, and they were
watched red by making exactly the reorder the OPEN item below proposes for that
path: both go wrong together, with the same two messages. `compile_one` carries
a byte-identical block, so the mutation is one edit applied twice and the two
paths answer as one.

**What this says about where a defect gets found.** The reorder was reverted
twice on the entry path for objections the sweep caught within a round, because
the entry path had cases. The same reorder shipped on the module path and its
two objections sat for three days. The corpus decides what a sweep can see, and
a path with no case in it reads clean whatever it does.

- **DONE** — the OPEN item the entry above filed. The reorder, the record, both
  readers, both mutations, and the differential back to 0 wrong.
- **OPEN, and now priced** — src/lib.rs:348 and :425 still check before
  canonicalizing. compile_one is reached only from `compile_repl`
  (src/repl.rs:290) and compile_library only from `kanso check <a library
  file>`. Both merge `dep_program`, so both see the twins, and both would break
  the way the entry path did -- they were never blocked on a measurement, they
  were blocked on this.

  Measured on this box, on `kanso check bench/compile_corpus/compile_corpus.kso`
  with the reorder and the record applied to both sites:

      library_instructions   50,244,948 -> 48,681,802   -1,563,146  (-3.111%)

  Larger in proportion than the entry path's -0.8627%, on a path no vein
  watches. The differential stays 29 cases 0 wrong through it, which is what
  says the record makes the reorder correct there and not merely cheaper; c28
  and c29 go red on the same edit with the record left out. The baseline
  reproduced to the instruction on a second run. What is owed before it ships
  is the vein, since a fall nothing counts is a fall nothing keeps.
- **OPEN, unchanged** — the twins inside `infer`, the other half of the
  reorder's value. `infer` indexes declarations positionally and a group keyed
  by (name, arity) is a dispatch group, so the twin is what lets a bare name
  resolve.

---

## 2026-09-08 (seventh) — the third compile path gets a row

`kanso check` routes a single file by its content and the three routes are
three different compiles. A directory is a module and takes
`compile_module_inner`, which is what `bench/compile_corpus` and
`compile_instructions` watch. A file with bare statements is an entry and takes
`compile_parsed_entry`, which `bench/entry_corpus` and `entry_instructions`
have watched since kanso#1330. A file of definitions alone is a library and
takes `compile_library` — and nothing in the tree counted it.

That path is not a corner. `kanso test` takes it on every run, and so does
`kanso check` on any single file that is not an entry, which is most files
here.

`bench/library_corpus/library_corpus.kso` names ten imports and uses each, the
shape `bench/entry_corpus` has and for the same reason: `compile_library`
merges the dependency program and runs its own whole-program check over
everything the imports bring, so a corpus with one small import would measure
mostly the work underneath it. The directory is named to the same length as
`compile_corpus`, because the count tracks the length of the path the compiler
is handed at about 160 instructions a character.

The container projected 165,589,540 and CI wrote the row: **164,253,088**, on
binary sha 9bc8f829af68 in the job that also counted
compile_instructions=48,791,172 and entry_instructions=162,170,772, so all
three answer for one build. The projection is 1,336,452 high, +0.8136%, which
lands on the offset the other two rows already carry between this box's rustc
1.94.1 and CI's 1.98.1. Only CI may write the row, and the reason is that
offset.

CI's summary named exactly one failing vein and eighteen green, which is what
round one was for.

**A CORRECTION, made the round after the claim.** This entry and round two's
commit message both said the ratchet job proved `library_ir` on the runner.
Read the job log: it did not, and could not have. The ratchet's second pass is
`ratchet -- touched origin/main`, which selects only rows patching a file the
branch changed, and it reported

    ratchet: 1 rows patch a file this branch changed
      the ratchet (every gate has a mutation that turns it red)

-- one row, the ratchet's own. This branch touches ci.yml, CLAUDE.md, three
gate scripts, the ratchet, the trend gate, a spec, the log and two new bench
files, and no `src/`; `the_library_program_is_checked_twice.sh` patches
`src/lib.rs`, so the touched guard correctly passed it over. What CI did run is
the first pass, `every mutation still matches the source it patches`, which
does read the new mutation's anchor against `src/lib.rs` and found it. So the
anchor holds on the runner and the row's provability there is untested; it was
proved in the container, 165,589,540 -> 190,698,277. The next change to
`src/lib.rs` -- the reorder -- is the branch that will select this row and
prove it on CI.

**THE SPEC PREDICTED ITS OWN FAILURE MODE AND THIS IS THE INSTANCE.**
`tests/the_compile_sweep_names_every_compile_gate.rs` derives the sweep's list
from goldens matching `bench/compile_*` and `bench/entry_*`, and its own doc
comment says: *a prefix list is exactly the shape that goes stale when a vein
is added under a new name.* `bench/library_instructions_golden.txt` matches
neither prefix, so both derivations in that file walked straight past it and
the sweep would have looked like coverage while missing the newest vein. Both
are widened here, in the commit that adds the vein.

The trend gate's own coverage spec did NOT have that hole:
`tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads `bench/` off
disk and keys on `contains("golden")`, so it went red the moment the file
existed and named what was missing. Two coverage specs over the same tree, one
keyed on a prefix and one on a substring, and only the substring one survived a
new name.

A third coverage spec found the other half of the same gap. The ratchet keeps a
`host_bound` list of the gates that count under callgrind, so that a runner the
golden does not name is reported as unproven rather than credited as a
regression, and `tests/a_host_bound_gate_is_reported_not_credited.rs` derives
that list from the gates that actually run the tool. It went red naming `sh
scripts/gates/library_instructions.sh` as soon as the gate existed. Three specs
over one tree: the substring-keyed pair spoke, the prefix-keyed one did not.

The mutation is `the_library_program_is_checked_twice.sh`, the library twin of
the entry one, and its anchor takes two steps rather than one:
`let merged_diags = check::check_merged(&program, false);` appears twice in
`src/lib.rs` because `compile_one` carries a byte-identical block, so the
function is found by its signature and the duplicate goes in at the first such
call after it. What it proves is the argument for the row: the same edit leaves
`compile_instructions` and `entry_instructions` green. It rides in the ratchet
as `library_ir`, beside `compile_ir` and `entry_ir`.

Proved rather than assumed, in the order the rule asks for: it applies (two
calls become three, and the third is inside `compile_library` at 432 with
`compile_one`'s at 355 untouched), it compiles, and the row it moves goes
165,589,540 -> 190,698,277, a rise of 25,108,737 or 15.16%, against a gate that
asserts equality. Counted here with the host check bypassed on purpose, because
this container may not compare the row and the question was whether the
mutation moves it rather than what the value is. Restored, rebuilt, clean.

Still open, unchanged by this: `src/lib.rs`'s two remaining callers check
before they canonicalize. That reorder is measured — 50,244,948 -> 48,681,802,
**−1,563,146 / −3.111%** — and was blocked on this vein. It is not in this
commit, so the row this one opens is the pre-reorder baseline and the next PR
is what spends it.

---

## 2026-09-08 (eighth) — the last two callers check before they canonicalize

kanso#1328 put `canonicalize_bare_aliases` in front of the whole-program check
on the module path and kanso#1335 did it on the entry path. `src/lib.rs` kept
two callers running the old order: `compile_one`, reached only from
`compile_repl`, and `compile_library`, reached from `kanso check` on a file of
definitions. Both merge `dep_program`, so both see the twins, and both would
break the way the entry path did. They were never blocked on a measurement.
They were blocked on a vein, and the vein opened yesterday.

Measured on the library corpus, both readings on this container, the recipe
`scripts/gates/library_instructions.sh` uses minus the host check:

    library_instructions   165,589,540 -> 164,300,594   -1,288,946   -0.7784%

Read twice, on two builds whose shas differ, and identical to the instruction.

**The recorded −3.111% is a different workload and does not belong to this
row.** The 2026-09-08 (sixth) entry measured `kanso check
bench/compile_corpus/compile_corpus.kso` treated as a library file, 50,244,948
-> 48,681,802, and that corpus is a third the size of `bench/library_corpus`,
so the same absolute saving reads as four times the proportion there. The row
this change is watched by falls 0.7784%.

**The record was watched red before it was watched green.** The reorder without
it takes `scripts/module_differential` from 29 modules 0 wrong to 2:

    a bare call to a shared name, from a library file
      expected it to compile:
      error[opacity]: `m/thing` is foreign -- only `m` builds a `thing`;
      ask it for one through a pub function

    a call from a library file at the wrong arity
      refused, but not with
      'error[arity]: no 2-argument arm of `one` (arms take 1)':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

The first REFUSES A PROGRAM THAT COMPILES; the second is the spelling
kanso#1120 settled. Both are the library-path twins of what the entry path had,
which is why the record ships with the reorder rather than after it. With
`check_merged_after_aliases(&program, false, &rewritten)` at both sites the same
run is 29 modules, 0 wrong.

**THE ROW COULD NOT BE SELECTED AT ALL, and this branch is what found out.**
kanso#1337 said its mutation was verified only as an anchor that still matches,
never run, because the touched pass selects rows patching a file the branch
changed and that branch touched no `src/`. This one rewrites `src/lib.rs`, so
the row should have been selected. It was not. CI's touched pass named three
rows and neither `library_ir` nor its entry twin was among them:

    ratchet: 3 rows patch a file this branch changed
      cost goldens — a front-end pass that owns the program's names instead of
        borrowing them
      welfare — a number the page states about the present drifting from its
        golden
      diagnostics differential — a name a module keeps private crossing an
        import anyway

THE SELECTION KEYS ON GUARD LINES, NOT ON THE PATCHED FILE. `read_it` collects
the lines of a mutation that spell `grep -q` and asks whether any of them names
a file the branch changed. That is deliberate and kanso#1254 argued it well: a
guard IS the dependency, and reading the whole script instead selected 33 rows
where 12 were at risk. But both compile-path mutations reach `src/lib.rs`
through `"$file"` and assert with `grep -c`, so the path appears on no guard
line and neither row could ever be selected, whatever a branch touched.

`entry_instructions` has been in that position since kanso#1330 and
`library_instructions` since kanso#1337 — the same pair the trend gate had left
unclassified, found the same afternoon by two different means.

Both mutations now carry a guard line that spells the path, and the selection
goes from three rows to five with the entry and library rows both named. Watched
blind first: the three-row listing above is this branch before the repair.

**EIGHT MORE ARE IN THE SAME POSITION**, and the count is off disk rather than
from memory: of 112 mutations, 77 name a path under `src/` and 69 of those name
one on a `grep -q` line. The two repaired here were among the ten that did not.
The eight left are

    a_module_rewritten_twice_before_it_is_checked.sh
    a_path_copied_once_per_declaration.sh
    a_validator_that_skips_its_tail.sh
    a_validator_the_reference_disagrees_with.sh
    an_entry_program_rewritten_twice_before_it_is_checked.sh
    clippy_bait.sh
    misformatted_source.sh
    the_wasm_engine_answers_something_else.sh

and each is unselectable for the same reason: it reaches its file through a
variable and asserts with `grep -c`, `grep -cF` or `grep -A1`. Repairing them is
its own change, and it owes a spec that reads the mutations off disk — which is
why one is not shipped here. Written and watched red, it named all ten; with a
green list of eight it would only be a list, and a list is the shape that goes
stale.

`canonicalize_types` stays where it is. On the entry path moving it too read
8,930 instructions worse, because the alias pass deletes the twins before that
one would have walked them, and there is no reason to expect the other
direction here.

**CI's row.** The vein summary named exactly one failing counter against
eighteen green, which is what round one was for:

    library_instructions   164,253,088 -> 162,970,167   -1,282,921   -0.7811%

The container projected a fall of 1,288,946, or 0.7784%. It overstated the
saving by 6,025 instructions — half a per cent of the fall — while reading
0.8136% high on the LEVEL of the row. A level offset between toolchains does
not carry to a delta, and this is the second sitting to say so: kanso#1335
recorded the same thing on the entry and module rows, where the projections
also came out conservative.

**THE OTHER TWO COMPILE ROWS DID NOT MOVE AT ALL**, and that is worth writing
down because this change edits `src/lib.rs`, the compiler's own Rust.
`compile_instructions=48,791,172` and `entry_instructions=162,170,772` are
byte-identical to their goldens in the job that counted the new row. CLAUDE.md's
prior is that `compile_instructions` usually moves on such an edit through
layout alone; it holds, and this is the second recorded change small enough to
leave it alone, after kanso#1285.

**The mutation's anchor went stale in this same change, and pass one of the
ratchet is what said so.** `the_library_program_is_checked_twice.sh` anchors on
`let merged_diags = check::check_merged(&program, false);`, which is the exact
line the reorder rewrites, at both sites. So round one's ratchet job failed
`every mutation still matches the source it patches` and skipped the touched
pass underneath it — the pass this branch exists to reach. The anchor moves to
`check::check_merged_after_aliases(&program, false, &rewritten)` and the
duplicate lands at 465, inside `compile_library` (390), with `compile_one`'s
call at 372 untouched.

A mutation anchored on a line a change is about to rewrite goes stale in that
change. That is the ordinary case rather than a surprise, and the first pass
exists to report it on the runner before the row it guards is ever read.

**A FIFTH READER, found by running the trend gate on this change's own diff.**
The fall printed as UNCLASSIFIED:

    UNCLASSIFIED — no direction table names these, so they count
    toward neither side of the pure-regression rule:
      library_instructions

The gate walks the golden and has no direction for the counter in it, so it can
report the move and cannot say which way is better. A rise of the same size
would have printed identically and counted toward neither side of the
pure-regression rule. `entry_instructions` has been in the same position since
kanso#1330 — two compile veins unclassified for as long as they have existed.

This is the digestbench failure one vein down, which
`tests/every_benchmark_in_the_work_vein_has_a_direction.rs` exists to prevent
and could not see here: it reads `bench/instructions_golden.txt` and stops. Its
own argument carries straight across — each compile golden holds retired
instructions for one compile and fewer is better in all of them, so every row
has a direction and none of them is a presence counter whose direction means
nothing alone.

`tests/every_compile_vein_row_has_a_direction.rs` asserts it over every
`bench/*instructions_golden.txt` other than the work vein's, read off disk, so a
fourth compile path opening a vein under a fourth name is covered on the day the
file lands. Watched red first, naming both: `["entry_instructions",
"library_instructions"]`. Both join `lower_gg` and the gate now prints
`improved: library_instructions 164,253,088 -> 162,970,167`.

That makes five readers a compile vein owes: its gate, `all_compile.sh`'s
`gates=` line, the trend gate's walk, `golden_prose`, and the trend gate's
direction table. kanso#1337 wired three of them.

---

## 2026-09-08 (ninth) — the twins inside infer, sized at last

The log has carried this OPEN item since 2026-09-08 (fifth) and twice since:
*the twins inside `infer`, the other half of the reorder's value.* It has never
had a number. It has one now, and the number is small.

`enroll_bare` (src/lib.rs) clones every exported declaration of every imported
module under its short name, and those clones are real declarations that infer
and check both walk. On `bench/library_corpus` the top-level enrollment makes
**292 function twins and 45 type twins against a merged program of 1,437
functions** — one declaration in five. infer is 22.5% of that compile.

**THE OBVIOUS INSTRUMENT DOES NOT WORK, and the way it fails is the blocker
arriving as a diagnostic.** An env-gated early return in `enroll_bare`, so the
twins are never made, stops the compile at

    error[name]: `first` is already a declaration; rename the binding
      --> std/regexp/regexp.kso:654:3

Inside the standard library, not at the corpus's call sites -- `library_corpus`
writes every call qualified and needs no bare twin of its own. The bare space is
load-bearing inside std. That is the positional-index blocker already recorded
for this thread: `group_members`, `ctx.current_index` and the reader bitmap are
all indexed by position in `program.fns`, and beat.rs, check.rs and codegen.rs
all read `inference.returns[i]` by that same position.

**The instrument that does work: skip only the BODY WALK.** In infer's fixpoint
sweep, beside the dirty test, skip a declaration whose `synthetic` is set. Every
twin stays in `program.fns`, so no index moves. All readings on one probe
binary, so the added branch cancels:

    corpus                walked        skipped        delta
    library_corpus   167,284,685   166,685,167     -599,518   -0.3584%
    entry_corpus     164,832,926   164,512,869     -320,057   -0.1942%
    compile_corpus    49,403,663    49,412,576       +8,913   +0.0180%

infer itself: 38,972,582 -> 38,342,754, -1.6161%.

**One declaration in five is one instruction in sixty-two.** A count-based
estimate assuming infer is linear in the declaration count says 20.3% of infer,
4.57% of the row; the measurement refutes that by 12.6x. The reason is the
fixpoint: only DIRTY declarations are revisited, and a twin nothing calls goes
clean after its first visit and is never re-dirtied again.

**The module row goes the wrong way, and that is a finding about the probe.**
Skipping the walk leaves `returns[twin]` at its default instead of the answer
the walk would have written, so other declarations infer different values and
the dirty sets and round count move with them. The delta is a mix of walks not
taken and a fixpoint doing different work, and on `compile_corpus` the second
term wins. So these three numbers size the thread; they are not a ceiling in the
strict sense.

**Where that leaves it.** The shipping shape has to copy the original's answer
into `returns[twin]` rather than leave it at the default, and that copy costs
something none of this pays. So the honest reading is a couple of tenths of a
percent before the copy, against the alias-pass reorder's 0.7784% on the same
row in the same session. The thread goes to the back of the queue: not refuted,
not worth building next, and no longer unpriced. What is owed first if it is
ever picked up is recording the twin/original pairing at the `enroll_bare` clone
site, which is the cheap half of the copy.

## 2026-09-08 (tenth) — a host-bound row was dropped on every host, including the one that could prove it

`type bound` in scripts/ratchet/ratchet.kso has said since kanso#1228 that a row
sharing a host-bound gate is skipped where the runner cannot answer it and
**"on a run that lands on the golden's silicon the row is proved normally"**. The
code has never done the second half. `kept_provable` dropped every row whose
gate is on the `host_bound` list, and that list is a property of the GATE — its
golden pins exact counts and the runner pool is not one machine — which says
nothing about whether THIS run landed on the golden's silicon.

So `library_ir`, `entry_ir`, `compile_ir` and `work` could be selected and never
proved, anywhere. Not on a container, where the gate refuses and the drop is
right. Not on CI, where the gate compares and the drop is wrong.

kanso#1338's ratchet job is the instance, and it is the first branch that could
ever have produced one — the guard-line repair it carried is what made the two
compile rows selectable at all:

    ratchet: 5 gates green before any mutation
    ratchet: 5 rows patch a file this branch changed
      ... a front-end pass that owns the program's names instead of borrowing them
      ... work on the entry path, where the compile row measures a module
      ... work on the library path, which the entry and module rows both walk past
      ... a number the page states about the present drifting from its golden
      ... a name a module keeps private crossing an import anyway
    ratchet: 3 rows

Five gates green. The baseline had just run `entry_instructions.sh` and
`library_instructions.sh` on that runner and both compared. Then three rows ran,
and the two compile rows left the report with no line at all — not a finding, not
a skip notice, nothing. The same job's cost-goldens run read `entry instructions`
and `library instructions` green on their own steps, which is the same fact from
the other side.

THE BASELINE ALREADY ASKS THE RIGHT QUESTION and its answer was being thrown
away. It runs every distinct gate on an unmutated worktree before any mutation,
and pushes a finding for each one it could not answer. A host-bound gate with a
finding is `UNPROVEN THIS RUN` and its row must be dropped; a host-bound gate
with no finding was green on this machine, minutes ago, and a red under mutation
in a sibling worktree on the same machine is the mutation's doing.

`finding` gains the gate it is about, and `kept_provable` reads it back. Four
lines of decision where there was one, and the static list keeps its job: it
still decides whether a red baseline FAILS the run or is excused.

Watched red, with the whole program rather than a piece of it. The spec copies
scripts/ratchet to a temp directory with one constant changed — `work_gate`
points at `scripts/gates/python_free.sh` instead of `instructions.sh` — which
makes a two-git-grep gate host-bound, the property four callgrind gates otherwise
have and only a runner can exercise. Then it runs the real `prove python-free`
against a worktree of HEAD. On the old rule:

    ratchet: 1 gates green before any mutation
    ratchet: no row on this runner could be proved; none was claimed
    ratchet: 0 rows

on the new one:

    ratchet: 1 gates green before any mutation
    ratchet: 1 rows
      red   python-free (the harnesses stay kanso) — a python call creeping back into a harness
    ratchet: every row turned its gate red

Ten seconds for the file's four fixtures. It lives beside
`a_gate_red_before_the_mutation_is_refused_rather_than_credited`, whose baseline
pass this reads, and shares that file's mutex because `prove` names its scratch
worktrees by fixed paths. The other three fixtures stayed green throughout,
which is what says the baseline pass itself did not move.

What this does not do is prove a compile row on a runner. That is CI's to say,
and the reading is in the next ratchet job on a branch touching src/: five rows
selected should now be five rows proved.

## 2026-09-08 (eleventh) — the per-module check, built and refuted as specified

§59 prices the per-module whole-program check at 20.38% gross and reasons down
to "about 8.5 million instructions, roughly 16.5% of the row" for a version that
moves the check up and leaves six slash-guarded passes where they are. This
built that version. The ceiling is larger than the page says and the change is
not reachable, and both halves are worth writing down.

Repriced first, with an env-gated skip of `check_merged` for every module that
is not the root, both readings on one probe binary:

    corpus                        checked        skipped         delta
    compile_corpus (module)     49,209,611     40,189,876    -18.3292%
    library_corpus (library)   164,306,354    126,030,042    -23.2957%

The library row gains more because 67.8% of that compile is
`load_dependencies` and its corpus names ten imports where the module corpus
names four. The shape §59 describes is most of what the library path does.

**§59 NAMES ONE MECHANISM AND THERE ARE THREE.** The page's account is the
slash: six passes skip any declaration whose name carries one, so run at the
root they apply to the root and pass over every dependency. That is right and it
is not the whole list.

The second is a message that QUOTES A DECLARATION. With only the three
slash-guarded checks kept per module, `scripts/module_differential` reads 29
modules and 2 wrong:

    a call to a sibling at the wrong arity
      refused, but not with 'error[arity]: no 2-argument arm of `one`':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

    an arm no call can reach
      refused, but not with 'error[dispatch]: overlapping overloads of `twice`':
      error[dispatch]: overlapping overloads of `m/twice` are illegal

Neither `check_call_arities` nor `check_overlapping_arms` is slash-guarded. What
makes them per-module is that the message names a declaration and the merge has
already qualified it — the spelling kanso#1120 settled. Keeping those two per
module returns the sweep to 29 and 0.

The third is ATTRIBUTION, and it is the one that closes the route. With five
checks per module the suite is 122 binaries and four fail, five tests:

    a_library_at_fault_is_reported_through_the_program_that_imports_it
    error_corpus_reports_each_golden_diagnostic
    the_wasm_engine_agrees_with_the_golden_corpus
    the_wasm_engine_complains_the_way_the_others_do
    the_front_end_infers_the_whole_program_four_times

The first four are one defect. A diagnostic raised on a dependency's
declarations at the root loses the file, the span and the module suffix that the
dependency's own compile supplied:

    error[naming]: `silly` answers only true or false: name it `silly?`
      --> deep_library_error/main.kso:4:8

    error[naming]: `silly` answers only true or false: name it `silly?`
      (module deep_library_error/deep)

It points at `main.kso`, which does not contain the fault. Detection is
unaffected — the dependency's declarations are all in the merged program — so
what is lost is attribution. The fifth failure is the saving showing up: a spec
pins how many times the front end infers the whole program, and running the
check once at the root is what moves it.

So any check that can fire on a dependency's declarations stays per module
unless a root-raised diagnostic can name where the declaration came from, and
that is nearly all of them. `infer::infer` is 11.8M of `check_merged`'s 19.1M
and runs for any check that reads inference, so the saving leaves with them.
18.33% and 23.30% are a ceiling reachable only with provenance on merged
declarations, which is a larger piece of work than a two-way split and is what
this thread owes next.

kanso#1003 walked this route once and withdrew it as "the per-dependency
check_merged is not redundant" without naming what made it so. §59 named one
thing. There are three, and they are written down now. Nothing shipped: the
probe is reverted and the page is corrected to say what the tree says.

## 2026-09-09 — the touched pass could not see eight mutations, in three different ways

kanso#1338 repaired two mutations the ratchet's `touched` pass could never
select and counted eight more in the same position. Reading all eight says they
are not one kind, which is why a repair that treats them alike would have been
wrong.

The pass selects the rows a branch could have blinded by intersecting the files
the branch changed with the paths each mutation names on a guard line, and
`guarding` recognised one spelling of grep.

**THREE guarded correctly and were still invisible.**
`a_module_rewritten_twice_before_it_is_checked`, `a_path_copied_once_per_declaration`
and `an_entry_program_rewritten_twice_before_it_is_checked` all do this:

    n=$(grep -cF "$target" src/lib.rs)
    [ "$n" -eq 1 ] || { echo "moved or multiplied ($n)" >&2; exit 1; }

src/lib.rs is spelled right there, on a grep line. That guard is STRICTER than
the `grep -q` its neighbours use — it catches the anchor multiplying as well as
vanishing — and `guarding` keyed on the literal `grep -q`, so the three
mutations that guard best were the three it could not read.

Widening the predicate to `grep -q` or `grep -c` is measured rather than
assumed: over the 112 mutations on disk it takes the satisfying count from 69
to 72, and the three are exactly the three that were blind. Nothing else moves,
so it cannot over-select. That is the whole repair for this shape, and it is one
line rather than three edited scripts.

**THREE reach their file through a variable** — `grep -qF '<anchor>' "$f"` with
`f=src/runtime.c` or `f=src/wasm.rs`. A grep line with no path on it. Exactly the
shape kanso#1338 repaired, repaired the same way: one guard line spelling the
path.

**TWO carry no grep at all**, because they append rather than anchor:
`clippy_bait` and `misformatted_source` both `printf ... >> src/lib.rs`. They
cannot go stale the way an anchored mutation does, which is why they had no
guard — and they are NOT exempt. An `allow(clippy::ptr_arg)` anywhere in
src/lib.rs makes the bait inert with the gate green, and a crate-level rustfmt
escape does the same to the other. Both now assert the escape hatch is absent,
which is a guard worth having on its own and puts the path where the pass can
read it.

77 of 77 mutations naming a src/ file now name it on a guard line.

**The spec asserts the property, not the list.** kanso#1338 wrote the
list-shaped version, watched it red naming ten, and declined to ship it: with
the ten repaired it would have been a green list, and a list goes stale the next
time somebody adds a mutation.
`tests/every_mutation_names_its_file_on_a_guard.rs` reads the directory off
disk. Watched red on one of each repairable shape, naming both by file and by
path:

    a_validator_that_skips_its_tail.sh names ["src/runtime.c"] and guards none of them
    clippy_bait.sh names ["src/lib.rs"] and guards none of them

## 2026-09-09 (second) — the chart drew the counters the objective had stopped reading

docs/numbers.html is the long view: one row per merged commit, and a chart over
it whose own subject line reads "what a run costs, what compiling costs, and the
welfare score they roll up into". Five of its six lines drew counters the
objective had retired.

    line              read                                        status
    run instructions  instructions + encode_instructions          retired 09-06
    run memory        oneshot_arena_peak_bytes                    retired 09-06
    binary size       text_bytes                                  never a term
    compile work      compile_rounds+compile_visits+emitted_lines retired 09-03
    compile memory    compile_peak_bytes                          a term
    welfare           welfare                                     the score

The 2026-09-06 gavel made the objective's runtime one consolidated program, and
`run_instructions` and `run_peak_bytes` replaced the thirteen work rows and
twelve memory rows the run side used to weigh. The 2026-09-03 rebuild had
already retired fixpoint rounds, expression visits and emitted lines.
`scripts/perf_record` writes a row's objective counters straight out of
`welfare --counters`, unfiltered, so both counters the gavel minted have been in
every row written since that day. Neither had a line.

Read off the five hundred rows on the history branch:

    counter                     rows   from         distinct   span
    run_instructions              57   2026-09-06         18   3.04e9 -> 2.39e9
    run_peak_bytes                57   2026-09-06          5   156.8 -> 46.7 MB
    compile_instructions         105   2026-09-03         52
    compile_allocs               349   2026-08-09         45
    compile_peak_bytes           349   2026-08-09         22
    oneshot_arena_peak_bytes     349   2026-08-09          2   the drawn one

The last row is what a reader was looking at. `run memory` took two values, 2
MiB and 3 MiB, across every row that holds it, so that line was flat with one
step in it over the span where the counter the score reads fell 3.4x.

**THE CHART DRAWS THE OBJECTIVE'S FIVE TERMS NOW**, in the order
`bench/objective_sources.txt` lists them, plus welfare and plus binary size.
`.text` stays because the page has nowhere else to show it and the 2026-09-05
ruling is a fact worth publishing: no machine-code-size term in welfare, and
`.text` keeps its own exact vein. It is drawn grey and the legend says why.

`tests/the_chart_draws_the_objective.rs` replays `bench/objective_sources.txt`
against the TREND array in both directions, and was watched red on the old page
naming exactly the two failures:

    the objective reads ["compile_allocs", "compile_instructions",
      "run_instructions", "run_peak_bytes"] and the chart draws no line for them

    the chart draws ["compile_rounds", "compile_visits", "emitted_lines",
      "encode_instructions", "instructions", "oneshot_arena_peak_bytes"],
      which the objective does not read

Two keys are allowed past the second test by name, each with its reason written
beside it: `welfare` is the score rather than a counter it reads, and
`text_bytes` carries the 2026-09-05 ruling. A third arriving without a reason
turns the spec red.

THE SITE SMOKE FIXTURE HAD GONE STALE THE SAME WAY. Its stub history was six
hand-picked rows, and the keys picked were the ones the chart happened to read
in august: on 2026-09-09 it held neither `run_instructions` nor `run_peak_bytes`
and could not have caught a chart that drew neither. It is generated from the
newest real row now, every key of it, each value stepped so no series can draw
flat and pass for a drawn one. The count it asserts went from six series to
seven, and the protective property was watched: dropping `run_instructions` from
one stub row alone reads `[5,6,6,6,6,6,6]` and fails.

THE PAGE'S ROW ACCOUNTING WAS WRONG, and it is the same kind of error one layer
up. It said 349 of the five hundred rows are scored on the compile counters
alone and the other 151 have no score at all, which accounts for every row and
leaves out the 57 that are scored on all five. The real shape is 244 rows on two
counters at coverage 0.28, 48 on three at 0.44, 57 on all five at 1.00, and 151
with none. The paragraph is restated in dates rather than counts. Those counts
perish on the next merge — the file grows a row per commit, and "the five
hundred rows" was already a number waiting to go stale.

## 2026-09-09 (third) — the entry row and the library row are the same measurement

`bench/objective_sources.txt` said the instruction term "sums BOTH compile
paths: `kanso check <dir>` is a module and `kanso check <file>` is an entry".
`kanso check <file>` is TWO paths — it routes by content, and a file of
definitions alone is a library taking compile_library — so the sentence names
two of three and the term takes two of three, which is right for a different
reason than the one written down.

Whether the library row should join is answered by measurement rather than by
a gavel. Callgrind on this container, `kanso check` on each corpus:

    frame                     module        entry       library
    kanso::main           49,226,188  163,918,585  164,230,408
    load_dependencies     29,179,760  112,141,854  112,163,002
    compile_module_inner  41,345,039  100,795,947  100,804,487
    check_merged_after..  17,585,954   62,110,471   62,242,504
    infer::infer          10,931,355   36,679,783   36,496,937

bench/entry_corpus and bench/library_corpus name the IDENTICAL ten imports —
bits, io, json, list, math, path, regexp, render, sha256, text — and the two
totals sit 311,823 apart out of 164M, which is 0.19%. Their dependency loads
agree to 21,148, which is 0.019%. Every shared frame agrees to under half a
per cent; the paths diverge only in the roughly 21M above load_dependencies,
where compile_parsed_entry reads 142.58M against compile_library's 143.18M.

The term is already 66% dependency loading: 29.2M for the module corpus's four
imports and 112.1M for the entry corpus's ten, out of 213.1M. Admitting the
library row would count that same 112.1M a third time and weight loading three
to one against the compiler's own passes, for a dimension the entry row already
carries. The module row is redundant with neither, at four imports against ten
and 49.2M against 164M.

`bench/library_instructions_golden.txt` is unaffected and still fails CI when
it moves. A vein the objective does not weigh is still a vein — that is what
kanso#1337 opened it for.

**THE ATTRIBUTION PLUMBING §59 OWES WAS ALREADY PROTOTYPED, and the prototype
lives nowhere the tree can see.** kanso#1340 refuted moving check_merged to the
root because a root-raised diagnostic loses the file, the span and the
`(module …)` suffix, and named provenance on merged declarations as what the
thread owes next. That session built the plumbing and left it in a scratch
directory: a `diag::HasFile` trait implemented for `ast::FnDecl` that returns
`&self.file`, and a `diag::attributing(&program.fns)` wrapper standing in for
`&program.fns` at each per-declaration check, so a diagnostic raised inside the
loop carries the declaration's own file. Both patches are stale against main —
they fail at src/check.rs:724 and src/lib.rs:3578, where kanso#1338's reorder
moved under them — so what survives is the shape, which is written down here
because a container is not a record.

Four facts make that shape the right one, and they were checked rather than
assumed. `FnDecl` already carries `file: Arc<str>` and `span`, so the
provenance is in the tree. The merge does not re-stamp it: `stamp_file` runs
per parse, per file, and the merges are plain `extend`, so a merged
dependency's declaration keeps its own file. The file cannot ride on `Span`,
because kanso#1135 made a span two u32 for a 7.1% peak win and an `Arc<str>`
there hands it back; `Diagnostic` is the place, since diagnostics are built
only on errors. And the `(module …)` suffix needs no new state at all —
`split_qual` on the merged declaration's qualified name recovers it.

The scope is 23 checks in `check_merged_after_aliases` and 65 `Diagnostic::new`
sites in check.rs. The prize is the ceiling kanso#1340 repriced: −18.33% on the
module row and −23.30% on the library row.

**A CORRECTION, WRITTEN THE SAME NIGHT.** The paragraph above says the patches
fail "at src/check.rs:724 and src/lib.rs:3578". That conflates two of them:
`419_attribution.patch` fails only at check.rs:724, in four hunks that are all
the same substitution, and the lib.rs failure belongs to the wider
`419_all_six.patch`. Rebasing the first is mechanical — four lines — and it
builds.

Building it says the shape is further from shipping than the paragraph above
implies, in two specific ways, and both were found by running the suite rather
than by reading.

THE PROTOTYPE IS NOT INERT. The obvious shipping order — land the attribution
first, doing nothing until `check_merged` moves to the root, then move it —
does not work: `cargo test --release --test golden` goes red on
`error_corpus_reports_each_golden_diagnostic`.

    fixture: tests/golden/errors/a_reexport_of_a_name_nothing_offers.kso
    got:  error[name]: no import offers a pub `nonexistent` to re-export
            --> std/text/text.kso:3:5
    want: error[name]: no import offers a pub `nonexistent` to re-export
            --> a_reexport_of_a_name_nothing_offers.kso:3:5
             3 | pub nonexistent
                       ^

THE ATTRIBUTION IS DYNAMICALLY SCOPED, so it names whoever is iterating rather
than what the diagnostic is about. That refusal is raised at src/lib.rs:3348,
about a re-export in the user's own file, and it came out attributed to
std/text. The `Attributed` iterator holds its last item's guard until the
iterator itself drops, so a walk whose iterator outlives the raise site leaks
its attribution forward. A thread-local read by a constructor cannot tell the
declaration in hand from the declaration some other loop last held.

AND `render_across` IS NEVER CALLED. The lib.rs half of the patch is a single
`Diagnostic::new` conversion; no call site passes it the sources map. Every
cross-file attribution therefore renders against an empty map, which is the
second half of that fixture's diff — the quoted source line is gone.

So the next step is not a rebase and a measurement. It is: attribute at the
raise site rather than through a thread-local, and wire `render_across` at the
`compile_*` call sites. Then the reorder is measurable. kanso#1340 called this
"a larger piece of work than a two-way split" and was right; this is what it
consists of.

## 2026-09-09 — the chart's palette was picked for one of the two pages it is drawn on

**DONE.** kanso#1343 fixed *which* counters the trend chart draws and left
*how* it draws them alone. Clay, on the result: "both blue lines have gone up."
The rise was real and already answered — two corpus re-basings, kanso#1321 and
kanso#1331, with the baseline moving under them so the floor went 59.74 →
66.3024 — but the reading was harder than it needed to be, because three of the
seven lines were in the blue band. This is the colour half.

Colour on a categorical chart is computable, so it was computed. Run against
the two surfaces the page actually paints (`--bg`, light `#fcfbf7` and dark
`#0c0c0f`), the shipped palette failed four checks:

    lightness band      all seven outside it
    chroma floor        #94a3b8 at 0.035 — reads as grey, so as gridline
    CVD separation      #38bdf8 <-> #a78bfa, 5.2 deutan against a floor of 8
    normal-vision floor #fb923c <-> #facc15, 14.6 against a floor of 15

And the finding nobody had looked for: **one palette was shipped for two
surfaces, and it had been chosen for the dark one.** On the light page every
one of the seven lines sat under 3:1 — yellow at 1.48:1. The blue-band
collision was the visible half of that; the light page was worse and unremarked.
The grey was mine, from #1343, and it was the chroma failure.

Each mode now takes its own steps, validated against its own surface. The
ordering was searched rather than chosen: of the 5,040 orderings of seven hues,
536 clear the adjacent-pair gates in both modes, and 216 of those also clear
them among the three compile lines taken all-pairs — which is the comparison a
reader of this chart makes, and which the adjacent pairlist does not cover. The
one kept scores 9.2 worst-case either way (OKLab ΔE ×100, floor 8) with a
within-group normal-vision margin of 24.6 against a floor of 15.

Two orderings show why the second gate was worth adding. `aqua blue orange
violet yellow magenta green` passes the adjacent gate identically — 16.3 light,
13.0 dark — and puts orange beside yellow *inside* the compile group, where
they separate by 4.8 under deuteranopia and 10.6 under normal vision. The
adjacent check cannot see it, because those two lines are not adjacent. Every
ordering that moves yellow onto binary size, which is where the weakest-contrast
hue belongs, drops the adjacent margin to 6.9 — the warn band. That refinement
was rejected: it costs more than it buys.

Binary size draws dashed instead of grey. The dash says what the grey was
trying to say — this is the one line the score does not read — without failing
the chroma floor. The legend key is drawn as the mark, dash included.

Three light-mode steps still sit under 3:1 (#1baf7a 2.72, #eda100 2.09,
#e87ba4 2.60). That is a documented relief rather than a pass, and the relief
has to be real: a faint line is readable only if its number is written down
somewhere. It was not. **The panel under the chart carried no row for any of
the seven series** — it listed `compile_rounds`, `compile_visits` and
`emitted_lines`, three counters the 2026-09-03 rebuild retired from the
objective, and nothing the chart drew. So the panel now leads with the chart's
lines, built from `TREND` rather than from a second list written by hand, in the
chart's own order.

Two specs, both watched red first.
`tests/the_chart_palette_is_the_one_that_was_measured.rs` pins the steps and
the sequence in both modes and refuses a raw hex in `TREND` — four assertions,
each shown failing on its own, including the hex one, which the token assertion
short-circuits past under the obvious mutation. `missing_panel_rows` in
scripts/site_smoke reads both the chart's keys and the panel's rows off the
rendered page in a browser and asserts they are equal: dropping the spread
turns it red, and so does reversing it, since containment would let the two
orders drift while a reader matches a faint line to its number by position.

Also here: welfare's chart reader stops rounding to hundredths. The rounding
existed to survive a `Math.round` the vertical axis never needed — the axis is
per-series and floating — and it cost the panel the last two digits of every
move.

OPEN, unchanged by this: whether a chart row should mark the commits where the
baseline was re-based. `bench/welfare_floor.json` holds 224 history entries and
exactly two begin "re-basing, not a gain"; both join to their rows through the
`kanso#NNNN` in their prose, which works and breaks silently. The version worth
building stamps the baseline into the row itself, beside `scored_by` and
`scored_weight`, and back-fills the two historical steps once.

## 2026-09-09 (second) — page_drift reads committed history, so it under-reports on a dirty tree

**DONE.** kanso#1345 ran `sh scripts/gates/all_pages.sh` twice before pushing and
both times read `page drift 3/3` — at the budget, green. CI on the same content
read `the log is 4 entries ahead, and the budget is 3` and failed the
cost-goldens job, taking `welfare` with it as a skip.

Neither reading is wrong. The gate asks git for the last commit that touched
`docs/compiler.html` and diffs that commit against HEAD, so it sees only what
is COMMITTED. Locally the new log entry was still in the working tree, so it
was not in the diff and the count was the three that came before it. The commit
turned the same tree into 4/3.

It cuts both ways, and the second direction is the one that costs a round: an
uncommitted page edit does not discharge the debt either. Adding §63 to
docs/compiler.html and re-running the sweep still reported the failure, because
the edit was not yet a commit. Run this gate after committing, or read its
number as a lower bound.

Two other things this found. The gate keys on `docs/compiler.html` alone, so
editing docs/numbers.html — which is what kanso#1345 did, twice over — moves no
count at all. That is the gate working as written rather than a defect: the two
pages are different things, and the settled-design page is the one the log is
supposed to stay level with. And the budget is genuinely cumulative across
sessions: the four entries named were kanso#1341, #1343, #1344 and #1345, of
which two are the chart campaign, so §63 was written for that campaign as the
gate's own message invites.

## 2026-09-09 (third) — a merged-check diagnostic on the module path had no location at all

**DONE.** kanso#1340 refused moving `check_merged` to the root because a
root-raised diagnostic loses the file, the span and the `(module …)` suffix,
and named provenance on merged declarations as what the thread owed next. That
plumbing was prototyped on 2026-09-08 and left in a scratch directory with two
defects, both recorded on the entry above. This is the repair, and the second
defect turned out to be worse and more useful than the record had it.

**DEFECT ONE: the attribution was dynamically scoped.** The prototype read the
file from a thread-local set by the walk, and its `Attributed` iterator held
its last item's guard until the iterator itself dropped, so a walk outliving
the raise site leaked one file's attribution onto a diagnostic raised somewhere
else. `Diagnostic` now carries `file: Option<Arc<str>>` set AT THE RAISE SITE,
from the declaration in hand, by `Diagnostic::about(&decl.file)`. A value
passed in has no guard to outlive it. The whole golden suite passes, including
`error_corpus_reports_each_golden_diagnostic`, the test the prototype turned
red — so the plumbing is inert where the prototype's was not, and a check opts
in one call at a time.

**DEFECT TWO IS NOT "render_across is never called".** On the module path a
merged-check diagnostic was formatted as kind, message and the `(module …)`
suffix, with the span and the source line DROPPED ENTIRELY — the loop built
that string by hand and never touched the renderer. kanso#1340's blocker was
not a thing to build; it was sitting in `compile_module_loaded` being done.

The corpus already held the proof, in a pair nobody had read side by side.
`tests/golden/errors/let_binding` carries both variants of one program:

    .stderr           error[name]: `let` is not a type …
                        --> let_binding.kso:2:7
                         2 |   let x = 1
                                   ^
    .imported.stderr  error[name]: `let` is not a type … (module let_binding)

Same program, same error, two routes through the front end, and the module
route reported no location. `compile_module_loaded` now renders through
`render_across`, which picks each diagnostic's own source and falls back to
naming a file with no quoted line when its text is not in hand. Where that
source comes from is the next section: the first two answers both cost the
objective, and the third costs nothing.

The suffix stays at the end of the header line, so an existing message is
byte-identical and simply gains the two lines under it. That is what keeps the
blast radius small: one check wired, and exactly two goldens move, both by
addition. Measured on a real two-file module, a dependency's error caught only
by the merged check now reads `--> …/inner/core.kso:5:9` with the line quoted,
where it used to read the message and nothing else.

`check_binding_patterns` is the one check wired, as the caller that keeps this
from being plumbing with nothing behind it.

**Two things found on the way.** The per-file checks were never broken: the
`unused` refusal already names a dependency's file and line correctly, so the
loss is specific to the whole-program check over the merged program. And the
two goldens that moved are single files compiled as modules, so they prove the
module ROUTE gained a location and NOT that a dependency's file is named — the
`errors_module` fixtures still pass untouched, because their errors come from
checks not yet wired. A cross-file golden is still owed and is listed below
rather than claimed here.

OPEN, in order: `.about(&decl.file)` at the remaining raise sites in
`check_merged_after_aliases`; the entry and library paths' merged renders,
which have the same shape; a cross-file fixture under tests/golden/errors_module;
the forty-four module goldens regenerated; and then the reorder this was always
for, at kanso#1340's repricing.

**AND THE SIZE OF THAT FIRST ITEM IS NOT TWENTY-TWO.** This entry said so
until the survey behind it was redone. The driver calls 23 checks; 14 raise in
their own body, 17 sites between them, 1 wired here, so 13 checks and 16 sites
remain. Three have a declaration with `.file` already in hand at the raise
site; the rest raise inside a closure.

The redo also found what a count of the callees cannot see. NINE of the 23
raise nothing themselves -- `check_boolean_equality`, `check_build_blocks`,
`check_call_arities`, `check_call_shaped_list`, `check_decidable_failures`,
`check_err_as_value`, `check_field_exists`, `check_if_arity` and
`check_literal_arguments` -- and delegate to helpers that do. check.rs holds 65
`Diagnostic::new` against the 17 inside the driver's direct callees, so wiring
the checks is not the whole job and the helper sites need the file reaching
them too.

Two bad surveys preceded the good one, and both failed silently. The first was
a boundary scan by line that mis-sliced any body holding a nested `fn`, and
reported zero raise sites for four checks against 65 in the file. The second
matched `\nfn NAME` and found NOTHING AT ALL, because the driver is `pub fn` --
a scan that returns an empty set reads like an answer. A survey whose result
is a count wants a total it can be checked against; 65 is that total here.

**THE FIRST TWO SHAPES BOTH COST THE OBJECTIVE, AND THE THIRD IS FREE.** The
renderer needs the text of the file a diagnostic is about, and the merge loop
had been eating `parsed` — so the obvious repair is to keep the text across the
merge. Round one did that, with a `Vec<(String, String)>` built as the loop
consumed the triples, and CI turned five compile gates red:

    compile_allocs          29,606 ->      29,613     +7
    compile_peak_bytes     773,818 ->     774,847     +1,029
    compile_instructions 48,791,172 ->  48,744,634    -46,538
    entry_instructions  162,170,772 -> 162,528,521    +357,749
    library_instructions 162,970,167 -> 162,823,672   -146,495

The +7 is exact and derived: that vector is one allocation per module loaded,
and `KANSO_PHASES=1 kanso check compile_corpus` prints seven `load` lines
(compile_corpus, std/json, std/text, std/list, std/testing, std/text again,
std/render — std/text twice because a module is compiled once per path to it,
which "Remembering a compiled module costs more than compiling it again"
measured and declined). This container read 29,613 and 774,847 too, agreeing
with CI to the unit on both, as those two rows always have.

The three instruction rows are layout: three routes on ONE binary sha moving
+357,749, −46,538 and −146,495 in the same job cannot be seven allocations.
`welfare` priced the whole thing at −0.01. A fall means the change goes or the
weights are argued, and the right answer here was a third one: the shape was
wrong.

Shape two: keep `parsed` itself alive and take each program out of it in
place, so nothing new is allocated at all. allocations went back to 29,606 and
peak went the OTHER way, 773,818 -> 775,730 — worse than shape one by 883
bytes, because a `(String, String, Program)` triple is about three times the
width of a pair and holding that vector holds the wider one. Priced on the
objective, the 7 allocations saved are worth about a twentieth of what the 883
bytes cost. Declined.

Both shapes are answering the wrong question, and the two measurements
together say so: the rise in each is the size of the VECTOR and not of the text
it points at — the same files held two ways, 1,029 bytes and 1,912 bytes, where
holding the corpus's actual source would be tens of kilobytes.

Shape three ships, and it holds nothing at all. The loader is now
`module_sources`, and
the diagnostics branch CALLS IT AGAIN. A clean compile runs the code it always
ran, byte for byte; a compile that is about to print an error opens its own
files a second time, which nothing anywhere measures. On this container both
rows read exactly main's numbers — `compile_allocs=29606`,
`compile_peak_bytes=773818` — and `welfare` reads 66.30 against the floor of
66.30. If the second read fails the diagnostics still print, without their
source lines.

The general form is worth keeping. A repair that hangs state on the success
path to serve a failure that usually does not happen has bought the wrong
thing, and doing the work again on the failing path costs nothing anyone
measures. The three compile veins said so within a round.

**CI SAID, AND THE ANSWER IS A BETTER PROOF THAN ROUND ONE'S.** All three
instruction rows moved on the shipping shape too, and this time all three
fell together:

    row                    golden        CI            move
    compile_instructions   48,791,172    48,746,831    -44,341   (-0.0909%)
    entry_instructions    162,170,772   162,044,531   -126,241   (-0.0778%)
    library_instructions  162,970,167   162,840,377   -129,790   (-0.0796%)
    compile_allocs             29,606        29,606          0
    compile_peak_bytes        773,818       773,818          0

The last two rows are what make this worth writing down. The shipping shape
adds NO work to any successful compile, and the two counters that measure the
front end's work say so in the same job that counted the three that moved. So
the layout reading is not an inference from the size of the change here; it is
a measurement with the alternative already excluded.

Round one is the control. Seven allocations and a kilobyte moved those same
three rows -46,538, +357,749 and -146,495 -- three directions on one binary
sha. Zero allocations moved them -44,341, -126,241 and -129,790. A row that
answers differently to two shapes of one change while the work counters hold
still is the compiler's own bytes, and nothing about the corpus.

Summed on the objective's compile term the fall is 170,582 (-0.0809%), so
`welfare` rose and the floor is held at 66.30 in this PR with the reason
recorded. It is banked as layout and claimed as nothing else: the front end
did not get faster at anything, and the next change is not free to spend this.

## 2026-09-09 (fourth) — the chart drew two differently-scored populations as one line

**DONE.** The design chat's entry of the same day, "the cliff is the run terms
joining the score, and the chart draws a coverage change as a fall", diagnosed
what Clay has been looking at and named three pieces. This is the third of
them, the one that makes the page honest today rather than right.

`scripts/welfare_rescore` scores a row on the counters it carries and
renormalises the weights that remain, and it writes `scored_weight` into every
row so that a reader is not fooled. The chart never read the field. Across the
500 rows there are five runs of it and four boundaries:

    rows       scored_weight   welfare
    0..30      0.28            74.64 -> 73.77
    31..181    0.00            no score, the line has a gap here
    182..390   0.28            82.82 -> 89.72
    391..438   0.44            91.67 -> 91.57
    439..499   1.00            58.96 -> 66.30

Two of those boundaries are steps in the line, and neither is the compiler.
Compile instructions joining on 2026-09-03 takes the score 89.72 -> 91.67. The
run counters joining on 2026-09-06 take it 91.57 -> 58.96, because the compile
terms carry the advantage they have accumulated since august while the run
terms start at parity against a baseline measured on the day they joined. That
second one is the whole of the "dramatically worse" the page has been showing.

The chart marks every boundary with a dashed muted rule labelled with the
coverage to its right, and splits the welfare polyline per run, drawing it
faded wherever the coverage is below 1.00. Each segment reaches one point into
the next run so the step itself is drawn rather than left as a gap the eye
closes by guessing. Coverage gets no colour of its own: it is not an entity,
and a categorical hue would have made it an eighth series.

**`scored_weight` IS TEXT, AND THE FIRST CUT OF THIS DID NOTHING.** Every row
in the history carries the string "0.00", "0.28", "0.44" or "1.00", the same
way `welfare` is text and has always been read through `parseFloat`. A
`typeof r.scored_weight === 'number'` guard read all 500 rows as unscored,
found one run, drew no rule, and split nothing -- a change that ships, passes
its own eye test, and leaves the picture exactly as wrong as before. The runs
are keyed on the text now, which is canonical to two places and so compares
exactly, and the number is parsed only to decide whether the coverage is full.

The spec is in the site smoke, which renders the page in a browser and reads
the marks off the DOM. Its stub carried six rows at one coverage and could not
have seen any of this, so it now carries the shape the real history has: four
older rows without the run counters at 0.44, two with them at 1.00. That makes
`missing_series` a real assertion -- [2 2 2 5 6 6 6 6], the two run lines
short, the four old counters full, and welfare in TWO strokes -- and adds
`missing_bounds` for the one rule and its label.

Both were watched red first, and they fail differently, which is what says
they are testing two things. Under the `typeof` guard: `marks: []` and series
[2,2,6,6,6,6,6], both checks red, the real bug reproduced. With the split
removed but the guard correct: series red, `marks: ["coverage 1.00"]` still
right. Restored, green.

The prose said the score falls "from about 75 to about 52" and the column has
read 91.57 -> 58.96 since the compile epochs moved under it. Corrected, with
both steps named. `sh scripts/gates/all_pages.sh` green on all three. The
labels were checked for collision rather than eyeballed: four rules, tightest
gap 16px, right edge 1097 of 1200.

OPEN, both the chat's and both still cloud's: reconstructing the run terms for
the 439 earlier rows so they score on all five, and the compile-side epoch
table. When those land the boundaries stop being steps and these rules stop
having anything to mark.

## 2026-09-09 — the cliff is the run terms joining the score, and the chart draws a coverage change as a fall

Clay, 2026-09-09: "the latest welfare metric still looks like it has gotten
dramatically worse. I do not understand this. the only things you should have
done recently to the corpus had to do with applying the metric consistently.
that shouldn't have had anything to do with the compiler being worse in any
kind of way."

He is right, and the column agrees with him: it reads 66.30 across the last
fourteen commits with every counter byte-identical. What looks dramatically
worse is the shape, a peak of 91.67 on 2026-09-03 and 66.30 today, and the
whole of that gap is the step at 2026-09-06 11:26. This entry corrects the
mechanism the 2026-09-08 entry gave for that step, because the correction
changes what a reader should conclude from the picture.

**The 91 was never a score of the compiler.** `scripts/welfare_rescore` scores
a row on the counters it carries, drops a term whose counters are absent, and
renormalises the weights that remain; its own header says a row scoring well
on what it has "reads the same as a full row scoring well." `run_instructions`
and `run_peak_bytes` exist in none of the 439 rows before 2026-09-06 11:26 and
in all 61 after. So every earlier row is scored on the two compile terms alone,
at `scored_weight` 0.44, and reads high because the compile ratios are large.
The first row after carries all five at `scored_weight` 1.00, and the run terms
enter at parity, r = 1 against satiation 2.0, so each contributes a third of
its weight and pulls the total down:

    2026-09-06 09:43   scored_weight 0.44   welfare 91.57   compile terms only
    2026-09-06 11:26   scored_weight 1.00   welfare 58.96   all five, run at parity

The 2026-09-08 entry said the earlier rows "score against a baseline they never
carried." They do not score the run terms at all. The remedy is unchanged; the
reading is different. The chart draws two differently-scored populations as one
line.

**The page already says this, and the chart does not.** docs/numbers.html:
"read `scored_weight` before reading a step in the line ... the step where the
run counters arrive is a change in what was recorded rather than in what the
compiler costs." That sentence is correct, and finding it is the reader's job.
The line draws 288 rows at coverage 0.28 or 0.44 and 61 at 1.00 in one stroke,
one colour, with no mark at the boundary. The rescore writes `scored_weight`
into every row so that, in its own words, "a reader needs to not be fooled,"
and the chart does not read the field. That is why the site does not look
right: the safeguard is in the data and in the prose and nowhere in the
picture. The prose is also stale, "from about 75 to about 52," against a column
that now reads 91.57 to 58.96, because the compile epochs moved under it.

**What Clay's principle requires, in three pieces.** A change in what is
measured is neutral to the score, so the line is flat across every one of
them.

1. The run side: the 2026-09-07 ruling, still unbuilt. Reconstruct the run
   terms for the 439 earlier rows from the per-benchmark counters they do
   carry (`instructions` and `encode_instructions` in 424 of them),
   share-weighted, based at 2026-08-10, so those rows score on all five terms
   and the boundary disappears.
2. The compile side: the epoch table from the 2026-09-08 entry, so each of
   the four compile epochs is scored against a baseline scaled to its own
   measurement.
3. The chart: draw `scored_weight`. A lighter or dashed stroke below 1.00, or
   a marker at each coverage boundary, so that until 1 and 2 land a step that
   is a coverage change looks like one. This is an afternoon, and it makes the
   page honest today; 1 and 2 make it right.

**OPEN, all three cloud's.** The rescore and the chart are code and the page
is cloud's surface.

## 2026-09-09 — the merged check has four routes, and the fourth is the repl

§60 and CLAUDE.md both say `kanso check` routes a single file by content and
that the three routes are three compiles: a DIRECTORY is a module, a file of
bare STATEMENTS is an entry, a file of DEFINITIONS alone is a library. That is
true of `kanso check` and it is not the whole census.
`check::check_merged_after_aliases` has FOUR call sites in src/lib.rs:

    line   caller                    route
     168   compile_parsed_entry      entry      (a file with bare statements)
     372   compile_one               THE REPL   (not reachable from kanso check)
     465   compile_library           library    (a file of definitions alone)
    3666   compile_module_loaded     module     (a directory)

`compile_one` has one caller, `compile_repl`, which has one caller,
src/repl.rs:290. Its own doc comment says it serves both `kanso play` — the
playground's convention — and the repl prompt, assembling imports and units
into one source. So the fourth route is a user-facing surface that the website
runs, and no census keyed on `kanso check` could see it, which is why three
separate readings of this code have said three.

**The module row's name is one level off too.** CLAUDE.md says the module route
takes `compile_module_inner`. It does — but the raise site is
`compile_module_loaded`, which `compile_module_inner` calls at src/lib.rs:3375.
The entry point and the raise site are different functions, and a survey
grepping for the raise site finds the second name while the doc names the first.

**What this owes.** kanso#1346 repaired the attribution on the module path and
wired one check (`check_binding_patterns`); the `.about()` wide pass is 13
checks and 16 sites. This adds a fourth render to that pass rather than three,
and the repl's is the one with a user watching: a diagnostic in the playground
that loses its file and span loses it in a browser. Not measured yet — whether
the repl route renders locations today is the next question, and it is asked
here rather than assumed either way.

**How the count went wrong before.** Recorded on 2026-09-08 in this log: a
survey whose product is a count wants a total to check against, because an
empty or short result set reads like an answer. The route census had a total
available and did not use it — `kanso check`'s three branches — and the
function has four callers. Grep for the callee, count the call sites, and
reconcile against the routes; do not derive the call sites from the routes.

---

## 2026-09-09 — a fold's in-place write was licensed without asking about its seed

**DONE.** Native disagreed with the interpreter on a fifteen-line program, and
the disagreement was silent: a stale value, printed as an answer.

**How it surfaced.** Building the compile epoch table (kanso#1347 item 2),
`scripts/welfare_rescore` walks the five hundred history rows carrying a
per-counter divisor and stores each row's divisor in a map keyed by commit. It
read 1.0 for every row. The rescored file came back byte-identical to the one
the old tool wrote, which is what sent me looking: a change that does arithmetic
on every row and moves no digit is either a no-op or a lie.

**The defect.** `src/linear.rs` decides which `put` and `push` sites may write
in place. Inside a fold it grants the folder's own accumulator parameter that
licence, on the strength of `folder_is_unique` — which inspects the LAMBDA and
nothing else. A fold writes into its SEED on the first step, so the licence also
needs the seed to be uniquely owned, and that half was never asked. Both grant
sites had the hole: the licence walk (`callsites_unique_in`) and the marking
walk (`walk_for_push_in`).

`unique_in_with`'s own fold arm has always asked both, so "is this fold's RESULT
unique" was answered correctly the whole time while "may this folder write" was
not. The two questions sit forty lines apart in the same file.

**The shape that reaches it** needs two folds. An outer fold's accumulator
carries a value and a record of that value at each step; the inner fold hands
the value straight back on a step with nothing to do, so the stored copy IS the
accumulator; the next step's write lands in both. Neither fold alone does it — a
write outside a loop traces back to a literal, and a folder whose seed is built
where it stands owns it.

    native: first: 2   second: 2   fresh: 2
    interp: first: 1   second: 2   fresh: 2

**The fix** asks both conditions, in one place, because the two walks must agree
about it and this is exactly where they had drifted:

    fn fold_owns_accumulator(&self, args, ctx, scoped) -> bool {
        self.folder_is_unique(&args[2], ctx, scoped)
            && self.unique_in(&args[1], ctx, scoped)
    }

**What it costs: nothing on the runtime side, and three layout-sized falls on
the compile side.** `sh scripts/gates/all_counters.sh` reads the twelve cost
veins and the lazy tier and every one is byte-identical, so no in-place site in
the benchmarked code was standing on a non-unique seed. The compile sweep saw
nothing move on this host, with six of the nine gates host-bound. CI then read
all three instruction rows:

    counter                golden          CI            delta
    compile_instructions   48,746,831      48,746,192      -639   (-0.0013%)
    entry_instructions    162,044,531     162,042,653    -1,878   (-0.0012%)
    library_instructions  162,840,377     162,839,321    -1,056   (-0.0006%)

with `compile_allocs` 29,606, `compile_peak_bytes` 773,818 and the machine-code
row byte-identical beside them. Welfare's floor rises 66.3039170230475 ->
66.30393941879086 and is ratcheted in this PR.

**Ratcheting a golden is a page edit.** The three rows are quoted by five
`data-golden` spans in compiler.html, and moving the goldens without walking the
pages left all five stale — `golden_prose` caught it as the LAST step of the
welfare job, so the job read red with the number itself green at 66.30 and its
floor met. CLAUDE.md's rule says a page edit ends with
`sh scripts/gates/all_pages.sh`; the trigger is wider than the rule's wording,
because a golden that moves silently re-points every span that quotes it. One of
the five needed prose rather than a swap: the sentence said the library vein
"fell 1,282,921 instructions, or 0.78%, to" that row, and a row that moves again
makes the arithmetic false. The landing is now a fixed figure and the span
carries today's reading beside it.

**The direction is not evidence the fix is cheaper, and the entry says so.**
Two mechanisms could each produce a move this size, and 639 instructions cannot
separate them: the analysis does MORE work at every fold site (one extra
`unique_in` on the seed) and LESS at a site the new condition rejects, because
the marking walk then never descends into the folder's body. A condition that
is strictly added cannot make the analysis cheaper on its own. The work
counters staying put is the usual layout signature, and that is what the rows
are recorded as.

**The fixture** is `tests/golden/micro/a_folds_seed_is_held_by_something_else`,
which `micro_corpus_agrees_across_engines` runs on native and on the oracle
against one golden, and the wasm and browser sweeps run on the third engine.
Watched red through that harness before the fix landed: it named the fixture,
the native run, and `left: "first: 2"` against `right: "first: 1"`. Its last
line is the legitimate case — a seed born at the call, which the fold may still
write through — so a licence simply switched off would not satisfy it.

**All three accumulator kinds were reachable, and the fixture carries all
three.** The grant is per fold rather than per write, so one hole covered
`put`, `push` and `append` — but that is a claim about the code, and the
cheap way to settle it is to write the other two and look. Built, and on the
unfixed compiler native answered every one of them wrong:

    kind    native            the oracle
    map     first: 2          first: 1
    list    first: [1 9]      first: [1]
    bytes   first: [120 121]  first: [120]

A fixture exercising one of the three would have left the other two resting on
the claim.

**What generalises.** A two-part condition split across two call sites is one
edit away from disagreeing, and nothing here would have caught the disagreement:
the analysis has no differential of its own, and the corpus had no program whose
answer depended on it. The counters could not see it either — an unsound licence
makes a program FASTER and wrong. What found it was arithmetic that had to move
and did not.

## 2026-09-09 — the compile side has three changes of measurement, not two

The 2026-09-08 entry on the compile term said the workload had been
re-measured twice, and named a four-row table whose third row was a
compiler change rather than a change of measurement. Scanning all 500
rows of the perf history for an adjacent move over 20% in
`compile_instructions` finds three:

    row  commit    what happened                     instr    allocs    peak
    446  5f5561c   #1291: lib/json drops std/list   0.4586   0.4490   0.5177
    476  dfd118a   #1321: lib/json -> corpus        2.7232   2.7207   2.1047
    485  8535884   #1331: sums both compile paths   4.3612   1.0000   1.0000

The middle row reproduces the three factors already recorded in the
floor file, to the digit, which is what says the factors can be read
off the history at all. The last row moves `compile_instructions`
alone, as `bench/objective_sources.txt` implies — one gate key each
for allocs and peak, two for instructions.

The first row is the one nobody wrote down, and the 2026-09-08 entry
supplies its own reason for counting it: "a term measured on a library
moves whenever that library changes what it imports: #1291 dropped
std/list from lib/json and halved the compile veins with the compiler
untouched." That is the disease #1321 was opened to cure, so it owes a
factor like the other two.

The record already disagreed with the entry. kanso#1347 item 2 asks
for "the epoch table from the 2026-09-08 entry, so each of the FOUR
compile epochs is scored against a baseline scaled to its own
measurement" — and four epochs need three boundaries. The count was
wrong against the record before it was wrong against the measurement,
which is the cheaper of the two to check.

**DONE — `bench/compile_epochs.txt`**, one boundary a line in
`bench/runbench_phases.txt`'s style, read by `scripts/welfare_rescore`
and applied to the baseline before the compile term is scored. Four
epochs: A rows 0..445 (lib/json with std/list), B 446..475 (without),
C 476..484 (compile_corpus, module row alone), D 485..499 (module and
entry summed, today's). A row's divisor is the product of the factors
of every boundary at or after it.

Either side of each boundary, welfare before the table and after it:

    boundary                     before             after
    a23b005 -> 5f5561c    59.0275 -> 61.0313   56.7964 -> 57.0280
       step                     +2.0038             +0.2316
    e12a68e -> dfd118a    70.0258 -> 67.7549   66.0242 -> 66.0242
       step                     -2.2709             +0.0000
    fd3144d -> 8535884    67.9134 -> 66.2874   66.2875 -> 66.2874
       step                     -1.6260             -0.0001

The two pure re-basings flatten. dfd118a and 8535884 touch no src/ and
no lib/, and their steps go to zero; the -0.0001 is the floor file's
four-place 2.7207, which is the precision the factor was recorded at.

**The first boundary keeps +0.2316, and that is the answer rather
than a residual.** 5f5561c is #1291, which halved the compile workload
and made the escape scan skip-then-iterate for runbench -3.3884%.
Taking the ruler out leaves the runtime win standing. A boundary that
flattened to zero here would have been the table erasing a real
improvement.

**One part of this is independent evidence and the rest is not.** The
446 and 485 factors were derived from the counter jumps, so applying
them and recovering 1.0000 is the same arithmetic inverted. The 476
factors were measured by another session at the time of the change and
written into the floor file; they land at 1.0000 against numbers they
were never fitted to. That is the check worth having.

**The table could not be measured at all until the fold-seed
miscompilation was fixed** (kanso#1349, same day). The divisor map read
1.0 for all 500 rows because every stored divisor aliased the last one:
`list/fold`'s folder was granted an in-place write licence without
anybody asking whether the fold's seed was uniquely owned. With the fix,
`oldest_divisor` reads compile_allocs 1.2215943, compile_instructions
5.446526138624, compile_peak_bytes 1.08960319 — the products above, to
the digit. The table's first working run was the miscompilation's first
reproduction.

**The boundaries a history carries must be a SUFFIX of the table**, and that
one rule covers three situations that look alike. It took three cuts to find.

The first refused any history missing a boundary. That turned three spec files
red for a reason none of them was about — `the_reconstruction_refuses_a_splice_that_moved`,
`a_row_predating_a_counter_is_scored_on_what_it_has` and
`the_score_says_what_it_was_made_of` all stage synthetic histories — and
seventeen fixtures would have had to carry three commits they say nothing
about, with every future one after them.

The second was all-or-nothing: refuse a partial match, give a history carrying
none of them no epochs. That is right about fixtures and wrong about the file
CI actually rewrites. `history.jsonl` is bounded to the newest 500 rows, so the
oldest boundary leaves the window one day while the table still names it — and
under all-or-nothing that is a partial match, so main goes red for a file doing
exactly what it is meant to do. Read off origin/perf-history on 2026-09-09
the three sit at rows 444, 474 and 483 of 500, so the first departure is 444
merges out — long enough that it would have arrived with nobody expecting it.
The position DROPS BY ONE WITH EVERY MERGE, because the window keeps the newest
five hundred, which is why this reading is dated: it was 446/476/485 two merges
earlier, and a fixed count written down here is a count that goes stale between
the measuring and the writing. It already did once, in this entry's own first
draft.

So: all present, the table applies. A hole in the middle, or the newest gone
while an older one stays, is a stale table and refuses. None at all is the
whole table fallen off the front — a fixture — and gets no epochs. The
boundaries that left the window are dropped from the product, because a
boundary older than every row scales no row.

**DONE — the spec, and the first cut proved nothing.**
`tests/the_compile_epochs_flatten_their_own_boundaries.rs` holds four.
The first stages, per boundary, a fixture carrying every boundary's
commit — so the epoch table applies at all — where only the
pair under test holds counters: 10,000 before and 10,000 x factor
after, so the four-place factors make every number an exact integer and
nothing rounds inside the fixture. The second replays the counter names
against `bench/objective_sources.txt`, so a typo cannot sit in the
table scaling nothing. The third leaves one boundary out of a fixture at a
time — never the oldest, so an older one is always still present — and requires
the refusal by name.

**The fourth is the one whose first cut proved nothing, for the second time in
this entry.** It scores the same rows twice, once against a fixture carrying
every boundary and once against a slid window, and requires the two to agree.
The obvious assertion was that the boundary still FLATTENS, and that survives
the mutation the test exists to catch: if the departed boundaries keep their
factors in the product, every divisor shifts by the same amount, so every step
is still right and a pair either side of a boundary agrees exactly as before.
What moves is the absolute column, against a floor that is ratcheted. Watching
the mutation pass is what found it; the rewritten test reads 97.1221 against
the whole-window figure and goes red.

The first cut passed under a mutation that moved both numbers. The
welfare column is a STRING — `fixed` renders it and `put` stores what
it rendered — and the helper walked digits from the colon, hit the
opening quote, and answered the empty string for every row: the
comparison was `"" == ""`. The helper now reads between the quotes and
asserts what it read is a number. The mutation that works is
`epoch_value v d[k] true` in `at_epoch`, which takes the identity arm
for every counter and still compiles; `1.0 * v / 1.0` does not, because
`f` goes unused and the tool dies before reading stdin, which the spec
then reported as a broken pipe rather than as the tool's own
diagnostic. The harness lets that write fail and prints what the tool
said.

**OPEN — the re-basing marks (task #449).** Stamping the baseline into
each row and marking where it moved cannot be derived from the
counters, and the measurement says why: the compile side has three
adjacent moves over 20% and the run side has ZERO across
`run_instructions`, `instructions` and `encode_instructions` over all
500 rows. The run baseline did move — kanso#1284 re-based the run
counters to parity, which is the -32.62 step — and no counter moved
with it, because only the ruler changed. A workload change shows up in
the rows and a baseline reset does not, so deriving the marks from the
counters would find three of the four and silently miss the largest.
Queued behind this entry because it edits the same file.

**Owed from kanso#1346, recorded here (closes the kanso#1339 thread).**
The ratchet's touched pass selected ELEVEN rows on kanso#1346's branch,
against kanso#1338's five selected and three proved. kanso#1346 merged
without recording it.

**ANSWERED — what else the fold-seed hole reached.** `scripts/welfare_rescore`
was miscompiled and nothing pinned its output, so the question is whether any
other tool in `scripts/` was in the same position. Fourteen were run on native
and on the oracle and their output compared: `welfare`, `page_drift`,
`golden_prose`, `diagnostic_coverage`, `fingerprint`, `book_quotes`,
`grammar_check`, `stale_a_panel`, `perf_record`, `trend_gate` and `ratchet`
agree byte for byte. `prose_check` and `book_panels` were not compared — the
oracle does not finish either inside five minutes. `site_smoke` disagrees, and
it is the divergence `tests/a_file_that_is_not_text.rs` already pins: native
reads `docs/kanso.wasm` and hands the bytes back, the oracle refuses with "the
bytes are not text". Reduced to seven lines, that is the whole of it. Whether
`read_file` should be byte-transparent on every engine is filed as a design
question and is not re-asked here.

## 2026-09-09 — the rescore read its own arithmetic back as a measurement

Main went red at 06:48 on the `perf history` job, one of nineteen, and
stayed red: `the splice rows disagree on a phase they share`. The cause
is kanso#1346, merged the run before.

`scripts/welfare_rescore` rebuilds `run_instructions` for the rows
measured before runbench existed, and it WRITES the rebuilt count into
the row it hands back. ci.yml feeds it
`origin/perf-history:history.jsonl` — its own previous output — appends
one row and rescores the lot. So a rebuilt count comes back as an input,
and nothing in the file says which counts were measured.

The numbers, read off three successive history files:

    file       all-eight rows   run-bearing   carry BOTH
    dc658d65   48 (last 438)    62 (first 439)     0
    81ffc9e5   48 (last 437)    63 (first 438)     0
    22ca0fb3   48 (last 436)   112 (first 389)    48

The first two are the design: the phases were the measurement until
runbench arrived, the consolidated count took over, and the two sets
meet without overlapping. The third is one run later. Forty-eight rows
gained a rebuilt count at once, the oldest of them became the splice
anchor at line 389, and it was compared against the newest all-phase row
at 436 — forty-seven commits apart, so of course every phase disagreed.
The refusal was right. What it was refusing was this tool's own output.

**DONE — a measured run count is not the same as a row carrying one.**
A row carrying every phase is from before runbench, when the phases WERE
the measurement and the consolidated count did not exist to be taken, so
a count sitting on such a row is this tool's arithmetic. The splice
anchor now selects rows that carry a count and NOT the full phase set.
That is checkable against the record rather than asserted, which is what
the table above is for.

**And the rebuild recomputes rather than trusting what it finds.** A
count written on an earlier run was scored against whatever anchor that
run picked, and keeping it lets one run's arithmetic outlive the
reasoning behind it. Recomputing also repairs the file already carrying
the rebuilt counts, with no hand edit: the corrupted history rescores
clean, and a second pass over that output is byte-identical.

**The property is idempotence, and it is what CI relies on.** "Rewrite
the whole column every push" is only safe if feeding the tool its own
output gives the same answer, and nothing asserted that. The spec is in
`tests/the_reconstruction_refuses_a_splice_that_moved.rs` beside the
refusal it belongs with: rescore a three-row history, feed the output
back, require success and an identical column. It reproduces main's
failure message in a postcard, and it would have caught this on the day
kanso#1346 landed rather than one merge later.

**What generalises.** A tool whose output is its own next input needs
that property pinned, and this one had a guard against exactly the error
it went on to commit — the splice check exists because a rebuilt row
spliced onto a row that moved slides the whole history smoothly and
plausibly. The guard fired correctly and pointed at the wrong culprit,
because the file it reads cannot say which numbers were measured. A row
that does not record where its number came from will eventually be asked
to answer for it.

**And the page was describing a coverage level that no longer exists.**
`docs/numbers.html` said a row holding three of the five counters scores
0.74 — it said 0.44, and named the two rises as 0.28 to 0.44 and 0.44 to
1.00. Read off the file today the levels are 0.00 (151 rows), 0.28 (237),
0.74 (48) and 1.00 (64), and what each holds is:

    0.28   compile_allocs, compile_peak_bytes
    0.74   those two, compile_instructions, run_instructions
    1.00   those four and run_peak_bytes

The forty-eight at 0.74 are the reconstructed rows. kanso#1346 gave them a
rebuilt run count, which took them from three of five to four, and the prose
was not followed — the same omission that let the rebuilt counts be read back
as measurements. The figures moved twice over: once when the reconstruction
landed and again under the epoch table.

**None of the four is a `data-golden` span**, so `golden_prose` cannot see
them and CI stays green with the page wrong. That is the gap kanso#1338 closed
for the library row by wiring it into the gate, and it is still open for
everything the chart's prose quotes. The figures are dated in the sentence
now, which is a weaker guard than a span and the honest one to have while they
are unwatched.

The two rises read 0.28 -> 0.74 with the score 88.28 -> 62.67, and 0.74 ->
1.00 with 64.95 -> 56.73, measured against origin/perf-history rescored by
this tree — which is what CI writes on the next push to main.

## 2026-09-09 — the blob stops being committed, and the guard covered two entries of eleven

`docs/kanso.wasm` is what the playground runs and what every spec in
`tests/wasm_engine.rs` runs, and it was a build artifact that was also
committed. It is not committed any more, and the guard that was supposed to
make that safe was covering two of the eleven places the artifact is opened.

**The criterion was already written down and the answer was already in.**
ci.yml carried a step, deliberately not a gate, whose own comment said: "Once
a few runs have said the same thing, the answer decides whether this becomes a
gate or the committed blob stops being committed." It has said DIFFERS every
time. The measurement is not needed to reach that answer either — the
committed blob's last commit is `6f8c876e`, from 2026-09-06, and main has
merged past it many times since, so it is built from older source and cannot
reproduce whatever the toolchain does. Rebuilt on this container it is
1,736,492 bytes against the committed 1,719,102, a difference of 17,390. Both
steps are gone; the rebuild that every job already ran stays.

**Four jobs rebuilt it and a fifth read it, and the first sweep here said
five and called that every consumer.** ci.yml's specs, other-host and site
jobs, pages.yml and `scripts/browser_differential.sh` do rebuild. The asset
digests job does NOT: it runs jekyll over `./docs` and fingerprints the `_site`
that copy produces, with no rebuild anywhere in it. So the sentence "fingerprint
reads `_site`, built after the rebuild" was true of pages.yml and false of the
job in ci.yml with the same shape.

Found by running it rather than by reading it. A `_site` copied from a `docs/`
with no blob gives `missing asset: kanso.wasm` and exit 1 out of
`scripts/fingerprint`, and `undigested_references.sh` then fails behind it on
five surviving references. That would have been a red round. The job rebuilds
first now — before jekyll rather than before the fingerprint, because jekyll is
what copies the file — and the repaired sequence reads fingerprint exit 0 and
gate exit 0 on a staged `_site`.

The lesson is the one the tree keeps relearning: a list of consumers assembled
by grep names the files that mention the artifact, and the job that breaks is
the one that reads it through something else.

**The guard was at two call sites and there are eleven.** `freshness()` was
called from `the_wasm_engine_agrees_with_the_golden_corpus` and
`the_wasm_engine_complains_the_way_the_others_do`. The other nine —
four playground-prompt specs, two page-failure specs, the builtin-count
refusal, the error corpus and the partial-over-a-value limit — called
`Toolchain::load` with no check at all, so a stale blob let nine specs run an
old engine and pass. The guard's own doc comment said a stale artifact "would
let this whole file pass while proving nothing about the source"; that was
true of nine twelfths of the file it was written in.

Measured rather than reasoned: with the blob moved aside, the suite reported
eleven failures, two of them naming `scripts/build_wasm.sh` and nine saying
only "the wasm artifact reads". The check now lives in `Toolchain::load`,
where all eleven pass, and re-running with the blob absent gives the same
sentence eleven times.

**The checkout arm is deleted with the thing that caused it.** kanso#1180 added
an arm for the case where the blob is a few milliseconds older than every
source file, because a fresh clone writes `docs/` before `src/` and so tripped
the guard on every checkout. A checkout no longer writes the blob, so that arm
cannot fire; keeping it would leave a paragraph of explanation for a state the
tree can no longer reach.

**And it closes the mtime guard's most common false pass.** kanso#107 kept the
guard an mtime comparison and wrote down that content it cannot see may be
stale. The way that bit in practice was `git checkout -- docs/kanso.wasm`,
which stamps a NEW mtime on OLD bytes: the guard then passes and the engine
runs a blob built from an older compiler. kanso#1350's own validation
paragraph is a case of it — two wasm specs failing on this container for
exactly that reason. With no committed copy there is nothing to restore, so
that trigger is gone. The guard is still mtime-based and still cannot see
content; this removes the one routine way of fooling it, not the weakness.

**What this does not claim.** The pack keeps every historical version of the
blob — 382 of them, 26,886,564 bytes — and removing the file from the tip does
not shrink an existing clone. The saving is prospective: the blob stops gaining
a version per rebuild that anyone remembers to commit.

## 2026-09-09 — a row says where its run count came from, and the mark is sticky

The 2026-09-09 red main was a provenance failure wearing an arithmetic
costume. `welfare_rescore` wrote rebuilt `run_instructions` into the rows it
handed back, ci.yml fed it `origin/perf-history:history.jsonl` — its own
previous output — and with nothing on the row saying which counts were
computed, the rebuilt ones read as measured and one became the splice anchor.
kanso#1350 fixed the tool: the anchor now takes rows carrying a count and NOT
the full phase set, and the rebuild recomputes rather than trusting what it
finds. That closes the loop for this tool. It does not help the next reader,
and the file still could not answer the question.

It can now. A row that this tool rebuilt carries `run_source: "rebuilt"`; a row
whose count it did not write carries `"measured"`; a row with no count carries
nothing. Read off the live history the split is 48 rebuilt, 65 measured, 387
unmarked, and the mark agrees with `scored_weight` on every row — all 48
rebuilt sit at coverage 0.74, all 65 measured at 1.00, and no row has a count
without a mark. The welfare column and every `run_instructions` are
byte-identical to what CI wrote, and the file stays a fixed point under its own
tool over three passes.

**The mark carries no factor, and that is the design rather than an omission.**
The compile side is RE-BASED by an exact divisor a row could name;
this side is REBUILT from phase shares, and there is no divisor because no
ruler moved. A `1.0000` written here for symmetry would read as "measured
against an unmoved baseline", which is the precise misreading the mark exists
to prevent.

**THE FIRST CUT STRIPPED THE MARK AND RE-DERIVED IT, and that was wrong.** It
looked like the welfare column, which is recomputed every pass, so it was
treated the same way. It is not the same kind of fact. Whether a count was this
tool's arithmetic is a fact about where the number CAME FROM, and nothing
re-measures a commit from three weeks ago, so it cannot expire. The failure is
visible in the degraded pass: with no row available as an anchor nothing is
rebuilt, and a mark re-derived from scratch then sees only a row carrying a
count and writes `"measured"` on it. Measured on a history with the anchors
removed:

    fixture            rebuilt   measured
    the strip          0         48
    the sticky mark    48        0

Forty-eight rows relabelled from true to false, by the field whose whole job is
to stop that confusion. The mark is sticky now: a row already claiming
"rebuilt" keeps it, and "measured" is only ever written onto a row that has a
count and no claim. With an anchor present the rebuild recomputes anyway, so
the sticky path is the degraded one, and there it degrades toward the truth.

`a_rebuilt_count_stays_marked_rebuilt_when_the_anchor_is_gone` pins it, entered
through the real tool on a three-row fixture and then on the same rows with the
anchor dropped. Watched red under the mutation that restores the strip, which
reddens that spec and leaves the other three green.

**Still owed, and it is the other half of the task.** The compile side has an
exact per-counter divisor in `bench/compile_epochs.txt`, and a row does not yet
name the epoch it was scored in. That wants the boundary commits tracked
through `crossed` rather than only their product, so it is a real change and
not a line. The two marks answer different questions and must not be made to
look alike.

---

## 2026-09-09 — a row names the epoch its baseline was measured in

**DONE.** `bench/compile_epochs.txt` names three commits that changed how the
compile counters are measured. `scripts/welfare_rescore` divides the baseline
by every factor from a row's epoch forward, so two rows either side of a
boundary are scored against two different baselines. Which one a given row sat
on was not written anywhere. A reader comparing a row from March against one
from September was comparing two numbers over two rulers and had nothing on the
page to tell them apart.

Every rescored row now carries `scored_base`. The fold that computes the
divisors already walked the boundaries; it now carries the list of boundaries
still ahead of the row alongside the divisor map, and stamps each row with the
first one left. A row past all three reads `as measured`.

**The boundary row belongs to the NEWER epoch.** Its own factor is already
divided out — it is the first row measured the new way — so it is named by the
next boundary, not by itself. The first cut named from the list before the
row's own commit was removed, and stamped row 443 `before 5f5561c` when 5f5561c
is the commit that row IS. That reads as though the row predates a change it is
the first to carry, which is the one misreading the column exists to prevent.
The spec pins the whole sequence rather than a shape, so the off-by-one turns
it red wherever it shifts.

Over the live 500-row history: 442 rows `before 5f5561c`, 30 `before dfd118a`,
9 `before 8535884`, 19 `as measured`. The boundaries land at rows 443, 473 and
482, which cross-checks against the positions recorded independently on
2026-09-07. Every other column is byte-identical to what the previous tool
wrote over the same input, and the output is a fixed point under itself.

**Two specs, both watched red first.** `every_row_names_the_epoch_its_baseline
_was_measured_in` fails under the off-by-one with the whole sequence shifted by
one row. `a_history_under_no_epochs_at_all_is_scored_as_measured` holds the
second copy of the naming — `divisors` has an arm for a history carrying no
boundary at all, which builds the map in one pass instead of folding across
boundaries, and the naming is written there too. Without that spec the second
copy could say anything.

**A new column reddens seven specs in a file that pins whole rows, and that
is the corpus working.** `tests/a_row_predating_a_counter_is_scored_on_what_it
_has.rs` compares each rescored row byte for byte rather than reading one
field out of it, so adding `scored_base` broke five scoring cases and both
no-column cases at once. Nothing was wrong with those specs. The rows this
tool writes are read by the chart and by the tool itself on its next pass, so
a field added to them is a change to what those readers see, and a spec that
could not tell would be the defect. Their expectations carry the new column
now, and the file says why every fixture row in it reads `as measured`.

**What this does NOT give the run side.** A rebuilt `run_instructions` has no
factor: it is a ratio to an anchor, not a division by a recorded constant. The
run half of this thread (kanso#1351) marks such a row `run_source: rebuilt`
instead. Writing a fabricated `1.0000` there to make the two sides symmetric
would read as "measured against an unmoved baseline", which is exactly the
misreading the mark prevents.

---

## 2026-09-09 — an explanation of the error model, checked against the interpreter, found three gaps

OPEN, all three cloud's; the first two are the language as designed, not shipping. Clay asked for a written explanation of how failure
works in kanso for a friend. Every claim in it was run through
`target/release/kanso play` before it went out, and three did not hold as the
design says they should. None of them is in the book, which uses only the
spellings that work; each is a place where a reader who trusts a ruling
rather than a page gets a wrong answer.

**1. The possible-none check is gated behind an environment variable, and
the gate is the defect.** `check_none_exhaustive` in src/check.rs is the
diagnostic the book's own story implies — "this can be a none and `describe`
has no arm for it — resolve it here, or give `describe` a `none` arm" — and it
runs only under `KANSO_EXHAUSTIVE`, where the 2026-07-24 none campaign left
it while it waited on per-arm return sets. Clay, on reading that here:
"KANSO_EXHAUSTIVE is of course total nonsense. you're just describing how the
language works. the exhaustiveness when you're looking for a match on an arm
has always been the way the language works since like the first couple of
days of designing it." So there is nothing to rule and nothing to wait for:
a call whose argument can be a none, made to a group with no `none` arm, is
refused at check, and the flag comes out. The program the explanation used is
the book's menu sample with the `none` arm deleted and the call moved into a
function body:

    fn describe price
      "{price} yen"

    fn quote menu
      describe menu["pocky"]

    print (quote { "dango":350 "taiyaki":500 })

    kanso play                       <none> yen, exit 0
    KANSO_EXHAUSTIVE=1 kanso play    error[exhaustive] at the argument, exit 2

The second line is the language; the first is what ships. With the arm typed
`price:int` the bare run instead dies at execution time: `error[runtime]: no
overload of `describe` matches these arguments`, and a literal `none` handed
to that typed arm fails the same way at runtime, where a literal string
handed to it is refused at check (`literal_arg_type`). The 2026-08-15 sitting
recorded the same rule as "8: exhaustiveness dissolves into per-call
coverage": each call's inferred value set is checked for an unambiguously
matching arm, provable gaps are compile diagnostics, unprovable calls keep
the runtime err. The 2026-07-24 entry "what the last exhaustiveness report
is, and is not" says the one false report left was a group-level return set
where a per-arm one would be exact, and that is an implementation detail for
whoever ungates it. The explanation as sent states the check as the language,
without the flag.

**2. The 2026-08-29 gavel "effects are types, and the words are the only
doors" is unbuilt on the point measured: a `.` over an io still binds
automatically, so a `.` step headed by `rescue` or `annotate` is swallowed.**
The ruling: there is NO automatic bind; `<t>effect` is a box that can be
passed as data; `bind`, `annotate` and `rescue` are the sole eliminators,
ordinary functions taking the effect first. Under it, `effect . rescue orders`
is the ordinary pipe supplying the first argument — `rescue effect orders` —
because the dot no longer opens the box. On the shipped interpreter the dot
still opens it: the step synthesises a bind around `rescue orders`, bind skips
on failure, and the callback is never called.

    os/read_file! "no-such-file.txt" . rescue orders . print
      error[endpoint]: unhandled err reached the executor: "cannot read ..."

    rescue (os/read_file! "no-such-file.txt") orders . print
      no orders yet

    os/read_file! "/etc/hostname" . shout . print
      vm!!                       (the retired automatic bind, still shipping)

Both of the first two parse. The first is what the ruling describes and its
handler never runs; the second is what every fixture and both book chapters
use, and it is also the ruled prefix form, so nothing published is wrong. An
earlier draft of this entry filed the piped form as a fresh ledger question,
"build the rider or retire it"; Clay's correction the same hour — "we got rid
of that rule when we agreed that you have to use explicit combinators" — is
the gavel above, and a ruled question is never re-asked. What is owed is the
build: the dot stops binding over an effect, a box where a value is expected
is refused, and the two book chapters that teach the automatic railway (ch04's
call-site short-circuit, ch05's "piping into an io is bind") move with it —
the ledger's "The book teaches the boundary language" entry already holds
that half. The ruled chain, in the shape the 2026-08-29 gavel "the chain line
keeps its dot" gave it — "the combinators look and act like regular
functions", so a continuation spells them `. rescue orders` like any other
function —

    os/read_file! "the-orders-file-that-is-not-there.txt"
      . rescue orders
      . bind shout
      . bind print

parses today and dies at the executor with the handler never called, for the
same reason as the one-line form. Under "A ruling outranks a lead" the gavel
is at the front of the queue with item 1.

**3. A stale entry in the lexer's borrowed-keyword table names `rescue`.**
`kanso_form_for` in src/lexer.rs lists `try | catch | except | rescue` with the
message "a failure rides the same rails as a value; an arm names the err",
written when the language had no such word. The table is consulted by the
needless-continuation check, so a statement headed by `rescue` and continued
on `.` lines that would fit in eighty characters reports `error[syntax]: kanso
has no `rescue``. Lengthen the first line past the fold and the same program
runs:

    rescue (os/read_file! "the-orders-file-that-is-not-there.txt") orders
      . shout
      . banner
      . print

    *** no orders yet!! ***

Remove `rescue` from that arm of the table; `try`, `catch` and `except` stay.
The error corpus has no fixture for a worded step at a statement head, which is
why the message survived the word's arrival.

**One thing the check confirmed rather than broke**, recorded because it is
the sharpest demonstration of the own-err rule found so far and belongs in the
book's boundary chapter when that is written. Annotating a foreign failure
makes it yours:

    rescue (annotate (os/read_file! "no-such-file.txt") said) orders . print
      error[endpoint]: unhandled err reached the executor: "orders file: ..."
        born in the entry at s2.kso:13
        passed through orders

The raw read error was foreign and `orders`' `(err _)` arm caught it a line
earlier. `annotate` raised a new err in the entry's own module, so the same arm
is now walked past. That is the rule working exactly as ruled, and it is the
example to teach it with.
The two rises read 0.28 -> 0.74 with the score 88.28 -> 62.67, and 0.74 ->
1.00 with 64.95 -> 56.73, measured against origin/perf-history rescored by
this tree — which is what CI writes on the next push to main.

## 2026-09-09 — a number is written into the accumulator, and an arm is told what the switch decided

**DONE.** `append acc "{n}"` is how the JSON encoder writes every number, and
the template rendered `n` into a string for the one purpose of copying its
bytes into `acc` and dropping it: 379,530 strings a runbench, built to be
copied once. The render alone was 5.50% of the run program's instructions
(`k_b_render_value` inclusive, 130.1M), and the ryū and itoa digit cores
under it are about 111M of that and stay — what goes is the string around
them, the dispatch that reached it and the copy out of it.

Two changes, and the first does nothing without the second.

**The emitter fuses the pair.** `append acc "{x}"` with `x` proved a number
emits one call to `k_b_append_rendered`, which writes the digits into a
64-byte stack buffer through `k_render_number` — the int and float arms of
`k_render_at`, extracted so the render and the door share one writer — and
copies them into the accumulator from there. Where the accumulator has 24
bytes to spare the copy is three words whatever the length (a number is at
most 24 bytes, `-1.7976931348623157e308`, and the buffer holds 64), so the
call into memcpy for a handful of digits is not made. A value the ambient
`render/to_string` group could claim keeps the dispatch the template would
have made, so a user arm is never skipped, and every other tag the door
hands to `k_render` itself, so the bytes cannot differ from the unfused
spelling; the differential goldens hold that across all three engines.

**An arm is told what the switch decided.** The first build of the fusion
emitted zero fused sites on runbench. `encode_onto`'s `n:int` arm is reached
through the tag switch (§ the tag switch, 2026-09-05), which has already
proved the tag is 0 — and inside the arm the discriminator still carried the
whole group's set, REC and DESC and THUNK included. So the body forced `n`
again, asked whether a user `to_string` arm could claim it, and took the
generic door on every builtin it handed `n` to. `arm_tags_set` now records
the case's tags as the discriminator's set for the arm's body — INT for the
int arm, FLOAT for the float arm, REC for a record arm, the nullary's tag for
its arm — and restores the whole set after. Both number arms fuse, and the
`k_force_fast` in front of each is gone.

On the container: runbench **2,368,295,010 -> 2,347,625,055** (−20,669,955, −0.8728%),
the same bytes out, `append_rendered` 379,530, and `append_fast`,
`append_grow`, `ryu_renders` and every allocation and peak counter identical.
The two mutations measured alone say which half is which: the fusion off
with the arms narrowed reads 2,366,863,311 (−1,431,699, the narrowing's
own gain on the rest of the program: the forces and dispatches it removes
from every switch arm), and the arms un-narrowed with the fusion on reads
2,368,105,221 (−189,789) — with the whole set inside the arm the fusion is
never eligible and every site takes the fallback, which is the byte twin the
generic path always took. Neither half is the win; the pair is, because the
first cannot fire until the second tells it the tag.

**Two wrong cuts on the way, both in the profile.** The first build read
+0.15%: `k_render_number` and `k_b_append_range` were both out of line, so
the fused door paid two calls where the old path paid one, and
`k_b_append_range` alone rose 3.2M -> 22.2M. Both inline now. The second was
the fallback arm — the door the fusion takes when the value is not a number
or a user arm could claim it — which called `k_b_append_mut` directly and
paid 22,789,710 in `k_b_append_wide` on runbench, where the generic path
would have reached the byte twin, whose string arm copies inline. The
fallback takes the twin. Neither mutation could be read until that was
fixed: both mutants measured +0.9% over main, the same 21M, for a reason
that had nothing to do with what they mutate.

**Rows and pricing.** `append_rendered` joins the trend gate's higher-is-
better list beside `append_fast`. Ratchet rows `append_rendered` and
`arm_narrowed`, mutations `a_scalar_rendered_into_a_string_and_copied_again`
(the fusion predicate reads false) and `an_arm_not_told_what_the_switch_
decided` (the narrowing filters to nothing), both on the instructions gate.

**What #456 found, recorded here because this is its first consequence.**
Read against the objective's own marginals, the queue since 2026-09-08 had
been working the dimension holding a tenth of the headroom. A ten per cent
improvement is worth 0.76 points on run speed, 0.64 on run memory, 0.19 on
compile speed and 0.19 on compile memory; the remaining points are 18.34,
9.70, 3.46 and 2.20. The last 26 merges moved welfare 0.1115 in total, and
25 of them were compile-side. The run program's profile puts 23.2% of its
instructions in two loops, `encode_onto` (13.38%) and `value_for` (9.86%),
and this entry is the first of what that map says to do.

**The round CI returned, and the sweep defect under it.** Three jobs went
red on the first run. The cost goldens read `46a47 > sh_bytes=...` on all
twelve: main had gained `sh_bytes` between the branch and the merge, the
merge kept this branch's goldens, and `all_counters.sh --write` then reported
`rewrote` twelve times and changed no file. Its rewrite replaced data rows
line for line and stopped at the golden's last row, so a counter the runtime
had just gained — one more row than the golden held — was dropped, silently.
The rewrite is now `scripts/gates/keep_header.sh`, which appends the measured
file's remaining rows, and `tests/the_sweep_write_keeps_a_new_row.rs` runs it
on a postcard-sized golden and was watched red on exactly that case. The
book's two counters panels drifted by the `append_rendered` row and are
rewritten. The diagnostic scan found `k_render_number: not a number` with no
golden; it is the render's `default:` arm and neither caller can reach it,
so it is listed in `tests/golden/unpinned_diagnostics.txt` with the control-
flow argument. CI's rows: runbench 2,392,210,251 -> 2,369,642,706 (−0.9434%),
encodebench −1.9672%, livebench −2.7667%, oneshot −1.1334%, widebench
−2.0173%, jsonbench −0.0192%. The three compile rows rose, layout moves on
a compiler whose emitter grew: `compile_instructions` 48,746,192 ->
48,749,059, `entry_instructions` 162,042,653 -> 162,051,772,
`library_instructions` 162,839,321 -> 162,846,242. The machine code grew
with it, `text` 1,487,564 -> 1,494,508 summed over the fourteen binaries,
6,944 bytes for the fused door and its fallback twin. Welfare 66.30 ->
66.37, banked.

## 2026-09-09 — read_file is text and read_bytes is bytes, on every engine

**Built:** the 2026-08-29 ruling (archive, "gavel: read_file is text,
read_bytes is bytes, per precedent"), after eleven days on the unbuilt list.
`read_file` refuses a file whose bytes are not utf-8, with one sentence on
all three engines: `cannot read {path}: the bytes are not text`. Until now
native handed the bytes through as a string and the interpreter refused them
with its own words, so the same program answered differently by engine.
`read_bytes` is the other reader: it hands the bytes back as they are, and
takes the same two doors as `read_file` — `read_bytes` answers
`file_not_found` as data and `read_bytes!` insists. `write_file` and
`net_write` accept bytes, so a program can read a binary and write it back
or serve it.

**Where the bytes go.** The http library's `rendered` interpolated the body
into one string, which renders a bytes body as a list. The status line and
headers are now a preamble, `delivered` writes a string body in one write
and a bytes body in two, and the content-length is measured on `as_bytes`,
which is a text body's utf-8 or a bytes body itself. The browser differential
and the fingerprint script read `docs/kanso.wasm` through `read_bytes!`
rather than through a text read that only worked because nothing checked.

**Fixtures.** `tests/golden/micro/a_file_that_is_not_text.kso` reads a
five-byte file — `ff fe 00 41 80`, the first byte one no utf-8 sequence
begins with — both ways and prints the refusal and the five bytes; the
harness runs it on both engines and through a release build.
`tests/golden/runtime/read_bytes_takes_a_path_string.kso` pins the builtin's
own refusal of a non-string path, reached through a binding the check cannot
see through. The 63-entry arity table, the 59-name builtin list and the
56-call codegen table each grew by one, and the descriptor tag is 30.

**Two things found on the way.** A definition added to `std/os` above
`insisted` moved the line four runtime goldens quote (`os.kso:113`), so the
new readers sit below it. And a helper named `head` in `lib/net/http`
collided with four parameters of that name; the check refuses the shadowing,
which is the right answer, and the helper is `preamble`.

**The emitted code moved, and the sweep saw it.** `all_compile.sh` read
`emitted_code` MOVED on eight programs, every one that imports `std/os`: the
decoder itself in `bench/emitted_golden.txt` (defines 140 -> 142, calls
1,232 -> 1,236, lines 9,219 -> 9,245) and seven of the thirteen in
`bench/emitted_golden_others.txt` — encodebench, oneshot, widebench,
digestbench, readbench, livebench and runbench — each up two defines, four
to six calls and 26 to 28 lines, with every branch count identical. The two
defines are `read_bytes` and `read_bytes!`, which a program importing the
module carries whether or not it calls them; none of the eight does. A rise
on this vein is a regression to explain, and this is the explanation: the
library grew two definitions and the emitter carries a module's every
definition, as it did when lib/json dropped std/list and the veins halved.
Both goldens are regenerated with their headers kept. The first reading of
the sweep's output saw only the second file's diff and wrote that the
decoder had not moved; the re-run after regenerating that file said
otherwise, which is what the re-run is for.

**Rulings weighed.** STATUS.md's "Ruled, unbuilt" list (kanso#1353) holds
twelve rows. This is the smallest that touches every engine, and the one a
reader hits first: the book's boundary chapter cannot describe two readers
until both exist, and `read_file` handing bytes through on native while the
interpreter refused them was a divergence the differential law forbids. The
other eleven — effects as types, err readers, the fused chain operators,
pure fallibility boxed, `done`, exhaustiveness without the flag, a
qualified name as its module's declaration, records printing qualified,
the backends' partial over a value, block-born as the whole cohort, and the
boundary chapter — stay in the order the list gives them; the next build is
taken from it.
