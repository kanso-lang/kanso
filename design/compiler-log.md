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

**Round three: the counter the local sweep never ran.** CI's cost-goldens
job, once the runner's apt mirror stopped returning a mismatched index, read
`utf8_bytes` up on eight runtime veins by the size of the input each reads:
188,698 on the decoder, encode, oneshot, wide, read, live and run programs,
8,192 on the digest. That is the ruling's own cost made visible. `read_file`
validates every byte it hands back now, where native used to hand bytes
through unread, and the counter that counts validated bytes counts the input
file. The allocation counters beside it are byte-identical. The twelve cost
goldens were regenerated with `all_counters.sh --write` on this branch, which
the first two rounds skipped — the emitted sweep ran and the counter sweep did
not, and CI found the difference. The retired-instruction rows CI measured are
copied in: runbench 2,369,642,706 -> 2,369,917,628 (+274,922, +0.0116%), the
decoder +274,897, encode +274,968, oneshot +274,897, read +274,804, live
+274,897, wide +90,524, digest +6,153; basket, deepbench, escapebench,
pendbench, indexbench and scanbench read no file and hold to the instruction.
That is 1.46 instructions a byte for the validator, the word-at-a-time arm the
2026-09-07 entry measured. Every `.text` row rose too, 1,040 bytes on the
eight readers and about 3,056 on the six that do not read, since the reader
pair and the refusal are runtime code every program links. The compile rows:
compile_instructions 48,749,059 -> 48,751,741, entry 162,051,772 ->
162,061,812, library 162,846,242 -> 162,857,425, each the two definitions
`std/os` gained. Welfare reads 66.3705 against a floor of 66.3715, a fall
of 0.0009 that the gate's band holds; the floor is re-set to it with
`--set` all the same, because the trend gate's pure-regression rule reads
this branch as worse on twenty-six counters and better on none, and lets
that through only when welfare_floor.json's history names the change that
spent it. It is the differential-law exception welfare.kso states: a
`read_file` that answers the same on three engines is not a trade.

**Priced, row by row, for the trend gate.** The counters the branch moved and
the values they landed on: `utf8_bytes` 11,164,198, `run_utf8_bytes`
24,415,348, `encode_utf8_bytes` 75,741,068, `live_utf8_bytes` 75,741,068,
`oneshot_utf8_bytes` 450,566, `wide_utf8_bytes` 289,183, `read_utf8_bytes`
188,698, `digest_utf8_bytes` 8,192; `work_jsonbench` 1,485,334,799,
`work_encodebench` 3,963,988,526, `work_oneshot` 21,737,118, `work_widebench`
35,332,240, `work_digestbench` 10,426,549, `work_readbench` 4,561,941,
`work_livebench` 3,481,899,703, `work_runbench` 2,369,917,628;
`emitted_defines` 142, `emitted_calls` 1,236, `emitted_lines` 9,245,
`emitted_other_defines` 2,353, `emitted_other_calls` 20,516,
`emitted_other_lines` 133,241; `text` 1,523,196; `compile_instructions`
48,751,741, `entry_instructions` 162,061,812, `library_instructions`
162,857,425. Each is the validator, the two readers or the refusal, and the
paragraph above says which.

## 2026-09-09 — an err answers `.reason`, `.cause` and `.origin`, on every engine

The 2026-08-29 gavel "an err has readers" (archive), built. STATUS.md had
carried it as unbuilt since the sitting: `annotate e (err -> "config:
{err.reason}")` — the gavels' own sample — was refused at check time with
`no record type has a field reason`, and a callback holding an err could
look at nothing inside it.

**Built.** A field read desugars to a getter call, `Get_reason e`, as every
field read does, so the reader lives where a getter is entered. Each engine's
dispatcher answers an err at its ENTRY, before any arm is tried: the
interpreter in `dispatch_loop_inner`, native in the prologue `emit_reader_hole`
writes for both dispatcher shapes (`k_is_err` then `k_err_read`), the page
with `rt_err_read` in `emit_dispatcher`. `reason` is the value the err was
raised with; `cause` the err it wrapped, or none; `origin` the "{fn} at
{file}:{line}" it was born at, or none for an executor-born one. The
interpreter's `err_read` is the oracle and the wasm host calls it; native's
`k_err_read` mirrors it arm for arm. The entry is the only place that works:
a placeholder arm `(err Read)` matched a foreign err before the failure
pass-through and would have answered the reason to `.cause`, and an
own-hako err is one no arm may see, where a reader has to see both.

**The group has to exist.** A read of a field no record declares resolves to
no getter at all, so `desugar_field_reads` now synthesises one arm per reader
field nobody declares. Synthesised per module, as the record getters are, it
produced one identical arm in every module and the merge refused the overlap;
it runs once over the merged program instead. The checker's declared-field
set gains the three names so the read passes the `no record type has a field`
fence, and a record reaching a reader group still gets the field error every
getter gives.

**Fixture.** `tests/golden/micro/an_err_has_readers.kso`, settled failures
only, so the page runs it too: a plain err's reason, an annotated err's reason
and its cause's reason, `.cause` of an unwrapped err (`<none>`), a cause read
through a second `rescue`, the origin's function name on both, and json's
decode failure read as `e.reason.reason` — the case where a reader and a
record field share a name. Watched red on the checker first. The origin names
the raising function qualified when the program is imported, so the fixture
carries an `.imported.out` twin like the record-printing ones. Two things the
fixture taught while it was being written: `print none` writes `<none>`, and a
named group handed the err passes it through as ever, so the readers are
applied inside the lambda and the group gets the piece.

**Veins, CI's rows.** The readers sit at every dispatcher's entry and the
checker's field set grew by three names, so the front end carries them:
`compile_allocs` 29,606 -> 29,714 (+108), `compile_instructions` 48,751,741 ->
48,820,126 (+68,385, +0.14%), `entry_instructions` 162,061,812 -> 162,734,847
(+673,035, +0.42%), `library_instructions` 162,857,425 -> 163,022,357
(+164,932, +0.10%). The compile peak (773,818), every runtime vein, the
emitted, text and machine-code rows are byte-identical. Priced, row by row,
for the trend gate: `compile_allocs` 29,714, `compile_instructions`
48,820,126, `entry_instructions` 162,734,847, `library_instructions`
163,022,357. Welfare reads 66.36 against the 66.37 floor, a fall of 0.01 the
readers pay in compile cost with nothing offsetting it; the 2026-08-25 ruling's
language clause applies, so the floor moves down to the reading, 66.3596,
by hand in `bench/welfare_floor.json` with its history entry (`--set` refuses
a fall of this size by design, and says the file is the door), and no
optimisation rides along to hide the price.

---

## 2026-09-10 — gavel: rows 15..390 stay unscored, and re-measurement is a lead with a probe in front of it

On "The reconstruction the 2026-09-07 ruling ordered has two usable phases,
not four, for 376 of the 439 rows", Clay: "I'm okay being pragmatic and
taking the first one." Ruled: option (1). Rows 15..390 carry no run terms and
get none reconstructed; they stay scored on the compile terms they carry, at
the coverage the rescore already stamps on them, and the chart keeps drawing
the coverage boundary at 2026-09-03 the way kanso#1346 draws every boundary.
The eight-phase half (rows 391..438) is built and stays. Nothing further is
owed on this entry; it leaves the ledger with this ruling.

Clay's second sentence is a question, and it is answered here so it is not
re-derived: "isn't it pretty trivial to just rerun the current metrics on the
old versions?" Mechanical, not trivial, and one unknown decides whether it is
possible at all. The mechanics: for each of 376 commits, `cargo build
--release` at that commit, `kanso build bench/runbench` with the compiler it
produced, one callgrind run for `run_instructions` and one counters run for
`run_peak_bytes`, on ONE host in ONE sitting that also re-measures HEAD, so
every row is on the same ruler by construction. About four minutes a commit
with a warm cargo cache, so a day of serial machine time for all 376, or an
afternoon sampling every fourth. The unknown: whether a compiler from
2026-08-10 accepts today's bench/runbench source. The surface moved between
then and now — the bang choosing the channel, `done`, the consolidated run
program itself — and nobody has tried. So the lead is filed with its probe
first: build the compiler at row 15's commit, compile today's runbench with
it, and read the answer. Yes means the sweep is a script and the rows come
back as `run_source: remeasured`. No means a runbench pinned to the old
surface, which measures a different program and would need its own ruling.
This is a lead, not a ruling: it goes on cloud's list as a lead and stays
off the "Ruled, unbuilt" section unless Clay says the word.

## 2026-09-09 — block-born is a proof, not a spelling

Built: the 2026-08-29 ruling "block-born is the whole cohort" (STATUS.md,
now removed; the archive entry of that name, Clay: "okay whole cohort it
is"). A field write's target had to be a name bound directly to a
construction in the same `build`; `twin = ada` followed by `twin.partner =
bob` was refused with the syntactic fence, and the 2026-07-28 measurement
found the same refusal on a node chosen by an `if`, a node taken out of a
list the block built, and a node reached through a field. The theorem never
asked for the fence. It asks that the cohort be closed, and every one of
those four values is inside it.

**What the checker proves now.** `check_build_blocks` used to carry a set
of names; it carries a cohort. Each value the walk proves born is an entry,
and a name, a field or an element holds an entry. A construction is born,
and its entry records which of its fields hold born values, read off the
constructor's arguments by position. A name bound to a born name shares the
entry, which is what makes an alias an alias: a write through `twin` is a
write to `ada`'s entry, and `ada.partner` read afterwards is the value that
was written. A list or map literal is born, and its elements share what
every element has; an empty literal shares nothing. An index of a born
literal is that shared entry, a field of a born value is the field's entry,
and a constructor pattern hands each position the matching field. An `if`
whose arms are both born is the meet of the two: a field or an element read
through it is read through to both arms and is born only when both answer,
so a later write to either arm is seen. A write made through the choice
reached whichever arm was taken, so both arms forget the field and the
choice remembers the write. That last rule is the one a reader is likely
to question, and `build_write_a_field_an_if_may_have_overwritten` pins it:
`other = cell "other" young`, then `chosen = if … other young` and
`chosen.link = old`, and `other.link` is no longer proved, because at run
time it may hold `old`.

A write inside an `if` arm's statement list may not have happened, so it
takes the field's proof away rather than supplying one. Anything a call
answers is not proved, a parameter is not, a name from an enclosing block
or an earlier iteration is not, and the six escape fixtures from July stand
unchanged beside four new ones: an `if` with an older arm, an element
beside an older one, a field a constructor filled with an older value, and
the overwritten field above.

**Two spellings of a field read.** The module route rewrites `b.up` to its
getter, `Get_up b`, before the check runs, and the play route after it, so
the walk reads both: `kanso check` on the fixture said ok while the same
program imported was refused at `over.id = 10`, and the getter arm is why
it is not.

**The write form is unchanged.** `target.field = value` with a name on the
left is still the one form, per the 2026-07-19 design; what widened is how
the name's birthday is proven. `ring[2]!.id = 20` does not parse and does
not need to: `middle = ring[2]!` then `middle.id = 20` is the same write.

**What it does not reach, and what that leaves.** Every algorithm the
2026-07-28 entry named — union-find's path compression, an e-graph's
rewire, unification binding the variable it found — reaches its node
through a call: `find` is a recursive function and its node is a parameter.
The four flows admit the shapes the entry measured and not the algorithms,
and design/memory-frontier-research.md's 4.4 row says so. Birth flowing
through a call is the next widening of this analysis, mine, and it is not a
new question.

**Spec.** `tests/golden/micro/a_build_writes_what_it_can_prove_was_born`
runs on all three engines: the alias, the field a constructor filled, the
field a write set, the indexed element and the chosen node, each written
through, then printed. Refused by the old compiler at the first of them.
The four escape fixtures are in the error corpus with their imported twins.
The diagnostic scan reads 313, none newly unpinned; the message is the one
July wrote, since a parameter is still not a construction made in the
block.

**Veins.** CI's sitting on the host-keyed compile rows, copied in:
`compile_instructions` 48,820,126 -> 48,820,567 (+441, +0.0009%),
`entry_instructions` 162,734,847 -> 162,736,962 (+2,115, +0.0013%),
`library_instructions` 163,022,357 -> 163,024,919 (+2,562, +0.0016%). The
cohort walk is a few hundred instructions more than the set walk on every
build block the corpus compiles, and the three routes carry the same
checker. `compile_allocs` and `compile_peak_bytes` did not move; nothing on
the runtime side did. Welfare reads 66.3596 either side, a fall of 0.00002
that the trend gate reads as a pure regression, so the fall is recorded in
bench/welfare_floor.json's history under the 2026-08-25 language clause,
the floor 66.35962 -> 66.35960.

## 2026-09-09 — a qualified name is its module's declaration

The 2026-08-29 ruling of that name (archive), built. STATUS.md carried it as
unbuilt; the log's 2026-09-08 entry "the reorder refuses a valid program"
declined it twice for want of the mechanism, and module_differential carried
the shape as its one `known_wrong` entry.

**What was wrong.** A module that declares `pub fn join` while importing
`std/text` holds two kinds of declaration under the bare name: its own arms and
the bare-enrollment twin of text's `join`. The loader qualified both to
`dep/join`, first writer won, and the first writer was the twin whenever the
import loaded first — so the module's own `pub` read as private from outside,
and the compiler's answer to that was an opacity refusal, "an import of `dep`
exports `join` too and took the name". With the refusal lifted the hazard was
worse than the refusal: a consumer's `dep/join ["x" "y"] "-"` reached std's arm
under dep's name and answered `x-y`, while dep's own arm could never be reached
at all.

**What the ruling says.** `dep/join` is dep's own `join`, and only that. Inside
dep, a bare `join` still dispatches over dep's own arms and the import's
together, because that is what an import is for.

**How.** The bare overload space of a mixed name moves to a spelling no
consumer can write: `dep/~join`, `ast::bare_space`, the mark being `~`, a
character the lexer never makes part of a name. In `qualify`: the module's own
bare declarations and its imports' twins are collected, `mixed` is their
intersection, and `owned` becomes a map from a bare name to the spelling its
call sites are rewritten into — `dep/~join` for a mixed name, `dep/join` for
every other. A twin in a mixed group takes the bare-space spelling, loses
`pub`, and holds no claim on the qualified one; the module's own arm keeps
`dep/join` and gets a synthetic non-pub clone in the bare space, so an
inside call finds both. The two-claims bookkeeping (`shadowed`, the
"took the name" message, the `Some(false), false` arm) goes, and `Loaded`
loses its third element. `Diagnostic::new` runs every message through
`ast::spoken`, which strips the mark, so a sentence about the bare-space
group says `dep/join`, which is what the author wrote.

**Spec, watched red on the pre-ruling compiler.** module_differential's
`known_wrong` is empty for the first time; its one entry is now case c30, a
module's own pub sharing a name with a dependency's, printing `OWN(x,y)` on
both engines. Two cases beside it: c31, a list handed to `dep/join` where dep's
arm takes an int, refused by the check as `no arm of `dep/join` takes a list
here (arms take int)` rather than dispatched into the import; and c32, a bare
call inside the module reaching the import's arm (`x-y`) beside the qualified
call reaching dep's own (`OWN(1,2)`). On the pre-ruling compiler all three fail
with the opacity refusal, 32 modules 3 wrong. The same hazard is pinned byte
for byte in `tests/golden/errors_module/a_list_handed_to_a_modules_own_int_arm`,
two diagnostics at columns 17 and 27, where the old compiler wrote the opacity
refusal at column 8. The page's §12 paragraph, which described the "took the
name" diagnostic as the settled answer, now describes the ruling.

**Verified on the container.** clippy, rustfmt, the errors_module corpus, the
golden suite, reexports, the wasm engine walk on a fresh blob, the unit tests,
the diagnostic scan (313 literal diagnostics either side, 0 newly unpinned), `all_counters.sh`
(the twelve cost veins and the lazy tier agree), `all_pages.sh` (three gates
agree; the §12 paragraph is the page edit), module_differential 36 modules 0
wrong.

**Veins.** `emitted_code` moved on two of the thirteen others, scanbench
19,752 -> 19,764 and runbench 34,806 -> 34,828 lines (the summed key
`emitted_other_lines` 133,241 -> 133,275). Against round one's tree, built
on this box for the comparison, the difference is dead text: each program
interns the raw name of a bare-space group (`scanbench/~split`, `split/~split`,
`runbench/~total`) beside the spoken form, and nothing in the IR names the
constant; runbench also carries a second copy of the capture-free thunk for
pend's `cards -> spent cards`, the lambda in `pend/total`, whose twin sits in
runbench's `~total` group, and neither copy is named either. `emitted_other_defines`
2,353 -> 2,354 and `emitted_other_calls` 20,516 -> 20,517 are that thunk.
Defines, calls and branches hold on scanbench. For the trend gate, the landed
host rows, CI's sitting on round two's commit: `compile_instructions`
48,820,567 -> 48,849,164 (+28,597, +0.0586%), `entry_instructions`
162,736,962 -> 162,730,512 (−6,450, −0.0040%), `library_instructions`
163,024,919 -> 163,431,666 (+406,747, +0.2495%). Round one's sitting read
48,784,668, 162,434,379 and 163,223,647; the reorder and the file test cost
the difference. The lines are the string table: `std/regexp`
declares private `first`, `spread` and `repeat` while importing `std/list`,
which exports all three, so those are mixed groups now, and the own-only
`regexp/spread` group's name and dispatch sentence are interned beside the
bare-space group's. Before the scrub the same table carried `regexp/~first`
and `no overload of `regexp/~first` matches these arguments`, which is how
the mark was found to reach a message a program can print. The decoder's
golden did not move. The host-keyed compile rows are CI's: `qualify` walks
one more set per module and clones an arm per mixed name, and the same-box
pair below is the container's projection, not the row.

Same-box pair, callgrind on the gate's boxes, #461's tree against this one:
module 50,441,411 -> 50,397,314 (−44,097, −0.0874%), entry 167,441,542 ->
167,533,649 (+92,107, +0.0550%), library 168,198,150 -> 168,348,197
(+150,047, +0.0892%); `compile_allocs` 29,714 -> 29,262 (−452: −465 for the
two-claims bookkeeping and the `shadowed` set gone, +13 for the vectors the
reorder builds), `compile_alloc_bytes`
4,806,203 -> 4,826,630 (+20,427, the clones), `compile_peak_bytes` 777,308
identical, rounds 66 and visits 22,133 identical. The allocation fall is
worth about 0.03 on the index by the per-term arithmetic in the 2026-09-09
read_bytes entry and the instruction move is inside the band, so the
projection is a small rise, to be banked with `--set` once CI's rows are in.

**Round one's kq failure, and two fixes on the bare space.** CI's kq job died
in the scale gate with `` `-` is not defined for these values ``: a bare call
inside a module that declares an arm over a name one of its imports exports
reached the import's arm. Dispatch tries a group's arms in declaration order,
and before the ruling the module's own arms stood ahead of the twins
`enroll_bare` appends, so a bare call both could take reached the module's
own. The clones were appended after the twins, which flipped that. c34 is the
shape: a module declaring `sum` over `list/sum` and calling `sum` bare from
`twice` printed `12` where the ruling's compiler prints `-12`, watched red.
The clones go in ahead of the twins now, each group of them in front of the
first twin of its name. Putting the two side by side found the second defect:
`check_constants` walks consecutive runs of one name, and a module's own
constant `bytes` beside the twin of `text/bytes` (the fold fixture #1349
added, run as a library by the golden suite) is a run holding an arity-0 arm,
refused as `a constant admits no overloads`. It had passed by accident, the
clone and the twin never adjacent. The check reads the arms the module wrote,
since the bare space is a union nobody declared and the module's constant
already stands alone under `dep/bytes`; c35 pins the shape and is red under
the reorder alone. The reorder found a third, in the golden suite's library
twins of `an_operand_that_varies_still_loops` and `trmc_count`: trmc writes
its loop wrapper as a synthetic arm under the module's own name, and qualify
read every synthetic bare arm as an import's twin, so `weigh` was a mixed
group in a module that imports nothing and the wrapper went to the bare space
as if it were std's. With the twin last, the bare call still reached it; with
the module's own arms ahead, the plain recursion ran first and the stack ran
out on every engine. A twin is now a synthetic bare arm from a file the module
does not own, `own_files` being the files of its non-synthetic bare
declarations; the wrapper keeps its qualified spelling and gets its clone
like any own arm. c36 pins it, a counted recursion inside a module called bare
a million deep, red under the reorder alone. The three compile rows below are
CI's reading of round two's commit, copied in.

**What stays in STATUS.md's "Ruled, unbuilt", and why.** Read whole before
this PR was chosen: block-born landed in #1359; this row, records printing
qualified and the partial over a value are a stack of three, built and
verified in worktrees, landing in that order; `done` and the fused chain
operators follow, with the plain dot untouched. Three rows are built and
cannot open yet. The effect type's two halves (the plain dot as an
application, the `<t>effect` spelling) and the exhaustiveness check on arm
match are one design with the fused operators: each breaks binds or arms in
kq the way #1357 did, kq's respellings live in kq, and kq is outside this
session's reach, so they wait on that respell and their rows stay. The
pure-fallibility rider waits on the ledger's "where the box wraps" ruling.

## 2026-09-09 — records print qualified, everywhere

The 2026-08-29 gavel of that name (archive), built. STATUS.md carried it as
unbuilt: `tests/entry_file.rs` still ignored the test the gavel said would
flip, and it still expected the unqualified form.

**What was wrong.** A record printed its type's spelling as the compiler
held it, and the compiler held two: a module reached through an import has
its declarations qualified (`lane/slow_lane`), and the same file run directly
does not (`slow_lane`). So `slow_lane 7` printed one way from `kanso play
lane.kso` and another from an entry importing `lane`, and the micro and mem
corpora carried `.imported.*` twins for exactly that divergence.

**The ruling.** Go's `main.T`: a record prints its module-qualified name
whatever the entry path, and the root module takes the name an importer
would write for it — a file's stem, a directory's name.

**How.** `ast::Program` gains `root`, set where each compile path returns:
the entry and play paths, the library path and the module path, from the
file's stem or the directory's name. Rendering is the only reader.
`eval::set_root`, called when an interpreter is built over a program,
records the root and the set of bare types the program declares;
`eval::shown_type` qualifies a type in that set and leaves every other
spelling alone, which is how the compiler's own `entry` record — the map
entry, declared by no module — stays `entry` on every engine. The
interpreter's `render` and the page's host (which renders through it) read
that; native gets a second table beside `k_type_name`, `k_type_shown`,
emitted by codegen and read only by `k_render_at`, so the identity the
runtime matches on and the field-error messages are untouched. Diagnostics
are untouched everywhere: a checker sentence names the type the author
wrote.

**Watched red.** Before the root reached the entry path, `kanso play
alone/lane.kso` printed `slow_lane 7` while `kanso run main.kso` printed
`lane/slow_lane 7`, on both engines; with the root threaded, both print
`trouble: [err lane/slow_lane 7] lane/slow_lane 7`. The first cut prefixed
every bare type, and the interpreter and the page printed `maps/entry` where
native printed `entry` — the wasm walk caught it on `examples/maps.kso`; the
set of declared types is the fix.

**Spec.** `tests/entry_file.rs`: `a_record_prints_qualified_whatever_the_entry_path`
replaces the ignored test, runs the fixture's direct twin (`alone/lane.kso`,
bare statements) and its imported form on both engines, and asserts all four
print the same qualified line. The fixture is rewritten to the modern shape:
the old one caught its own err with an arm, which the 2026-08 err rulings
retired, and `pub play` files refuse `kanso play`. Five example goldens
regenerate (`build_blocks`, `field_typesets`, `markers`, `none_is_a_value`,
`record_render`), each a prefix and nothing else; one book sample
(`appb/wrap_err`, an unhandled `config_bad`) and its panel. The micro, mem,
runtime and error corpora did not move: their `pub play` fixtures were
already reached through a staged import, which is where the twins came
from.

**Verified on the container.** rustfmt, clippy, the golden suite (eleven
tests with the twin spec), entry_file, errors_module, reexports, the unit
tests, the wasm engine walk on a fresh blob (twelve), `book_check.sh` (every
sample verified), the diagnostic scan (311 literal diagnostics, 0 newly
unpinned), `all_counters.sh` (the twelve cost veins and the lazy tier agree),
`all_pages.sh` (three gates agree).

**Veins.** `emitted_code` moved on every program by two lines and nothing
else — the decoder's `emitted_lines` 9,245 -> 9,247, the thirteen others'
summed `emitted_other_lines` 133,275 -> 133,301 (+2 each), defines, calls
and branches identical — and `compile_cost`'s five samples by one line
each, `lines` 6,094 -> 6,099, with `module_lines` 5,268 -> 5,269. The lines are `k_type_shown`: a program whose
root declares no bare type prints every record under the identity's
spelling, so the table is an alias of `k_type_name` rather than a second
switch, and the benchmarks are all that shape. The first cut emitted the
switch unconditionally and cost each program a define, a branch and thirty
to sixty lines. Every runtime vein is byte-identical: the render calls one
table where it called the other. The host-keyed compile rows are CI's;
`qualify` is untouched, and the same-box pair against #1360's tree is below.

Same-box pair, callgrind on the gate's boxes, #1360's tree against this one:
module 50,397,314 -> 50,386,179 (−11,135, −0.0221%), entry 167,533,649 ->
167,577,852 (+44,203, +0.0264%), library 168,348,197 -> 168,341,016 (−7,181,
−0.0043%): the root's name and the shown-name table are a few thousand
instructions either way, inside the noise of a rustc relayout.

**The build_cycle fixture is measured through the import now.** Folding
its `.imported.mem` twin leaves `tests/golden/mem/build_cycle.mem` holding
the numbers the staged import always produced, so the trend gate reads a
re-basing rather than a move: `build_cycle_alloc_bytes` 2,576 -> 3,168,
`build_cycle_perm_allocs` 7 -> 9, `build_cycle_sh_str` 1,936 -> 2,528,
`build_cycle_sh_buf` 176 -> 208, `build_cycle_thunk_allocs`,
`build_cycle_thunk_evals` and `build_cycle_thunk_live_exit` 0 -> 1 each,
`build_cycle_carry_dedup` 0 -> 1. The direct route's numbers were the
twin's other half, and the direct route is gone.

**A seventh twin, met on the rebase.** #1356 landed
`tests/golden/micro/an_err_has_readers.imported.out` while this sat in its
worktree: the fixture printed an err's origin with its file stripped, and an
import qualifies the raising function's name, so `boom` read
`an_err_has_readers/boom` through the harness's staged entry. The twin
went the way of the other six. The probe strips the module's slash too,
since what it asks is which function raised the err, and the qualified
spelling by route is the runtime corpus's accepted divergence, kept there
in its `.imported.stderr` twins.

One spec had pinned the old spelling for the root's own records:
`tests/a_type_name_as_a_bare_value.rs` expected a nullary record declared
in the entry file to print bare, `unit`. Under the ruling the root is a
module like any other and its name is the file's, so the line reads
`nullary_native/unit` on native and `nullary_oracle/unit` on the oracle,
one per staged file name; the expectation moved, found when the queue was
stacked on one worktree on 2026-09-10.

**Round one's compile rows, and the got file CI could not show.**
`compile_instructions` 48,849,164 -> 48,866,385 (+17,221, +0.0353%),
`entry_instructions` 162,730,512 -> 162,751,831 (+21,319, +0.0131%),
`library_instructions` 163,431,666 -> 163,473,663 (+41,997, +0.0257%), CI's
sitting on round one, copied in and noted in each golden. Three rows in one
band, on a change whose only per-program work in the front end is the root's
own String: the layout reading the compile goldens' older notes carry.
`compile_allocs` also disagreed, and its value is not in this entry because
nothing could reach it. The gate refuses to measure on a host the golden does
not name — this container is rustc 1.94.1 against the golden's 1.98.1, and it
says in as many words to let CI measure and copy the rows out of the job log.
The job log's own step for that, "the rows as measured here, to copy into the
goldens", named three got files by hand and there are five;
`allocs_got.txt` was one of the two it missed, and the gate's own message sits
fourteen gates and two callgrind dumps from the end, past what the log API's
tail returns. So the step globs `*_got.txt` now. The hand-written list
arrived with the first compile vein in #1214 and has been extended by hand
twice since, at #1330 for the entry row and #1337 for the library row, and
#1337 paid a round for the omission; the glob is what stops a third.
`compile_allocs` stays deliberately red this round, and round two prints the
number this one could only have guessed.

**Round two read the number, and found four refusals beside it.**
`compile_allocs` 29,262 -> 29,276 (+14, +0.0478%): the root's own String and
the set of declared bare types, paid once per compile. That row compared. The
four beside it did not. Rounds two and three landed on a runner whose
toolchain the goldens do not name, and `library_instructions` says so in its
own words -- "the sitting above was counted on a toolchain
bench/library_instructions_golden.txt does not name, so it is not a
reproduction of the recorded build and says nothing about the value". The
tell is arithmetic rather than prose: `compile_instructions` measured
48,866,385 against a golden holding 48,866,385 and still failed, and
`entry_instructions` and `library_instructions` did the same. A gate whose
measured value equals its golden and still fails is refusing, not
disagreeing.

Which gates refuse follows from what each golden names.
`bench/instructions_golden.txt` names glibc; the three compile-instruction
goldens name glibc and rustc; `compile_allocs` and `compile_memory` name
rustc alone. Every glibc-keyed gate refused on this runner and both
rustc-keyed gates compared, which is the split the measured-on machinery is
for. So the work vein did not move: `deepbench` 389,214,251 and `runbench`
2,369,917,611 are that runner's sitting, not a regression, and
`bench/instructions_golden.txt` is left alone.

Round one landed on a runner the goldens do name -- its three compile rows
came back as real comparisons, "counted X against Y", and those are the rows
copied above. The refusal is not this change's and no edit here can clear it:
the remedy the gate names is to re-measure every row on the new image in one
go and update the measured-on lines, which is its own change and not a ruling
this PR should fold in.

`tests/the_job_log_prints_every_got_file.rs` pins the property rather than
the glob: every `*_got.txt` any script under scripts/gates writes must be one
the printing step will cat. Watched red with the hand list restored, where it
names both files that list missed, `allocs_got.txt` and
`compile_libraries_got.txt`. The glob satisfies the property today; what the
spec is for is the next session writing the list out by hand again.

**Round five re-measures the four refusing veins, and it rides here.** The
paragraph above says the re-measure is its own change. That was wrong. The
runner pool has rolled forward, so a branch opened to carry the fix would land
on the new image and refuse in the same four places, and there is no tree that
goes green on both. It rides here.

The image is glibc 2.39-0ubuntu8.9 against the goldens' 2.39-0ubuntu8.8, and
nothing else moved with it: `compile_memory` printed "measured-on rustc=1.98.1;
here rustc=1.98.1" and compared, and the `machine_code` gate compares against
clang=19.1.1 and passed. Every run through 14:21Z compared and every run from
14:32Z has refused, four rounds of this PR straddling the change. main's last
run is 14:15Z and green, so main is green on the record and would be red if it
ran again.

The revision costs almost nothing, and round one is what makes that a
measurement rather than two sittings read side by side. Round one counted this
branch's content on 8.8 and round four counted it on 8.9 -- same commit
content, same clang, same rustc, same benchmark sources -- and the work vein
compared GREEN in round one, so every row sat exactly on its golden with this
change already in the tree. Of the seventeen glibc-keyed values, fifteen are
byte-identical across the pair: `compile_instructions` read 48,866,385 both
times, `entry_instructions` 162,751,831, `library_instructions` 163,473,663,
and twelve of the fourteen work rows did not move at all. The two that moved
are `work_deepbench` 389,214,232 -> 389,214,251, a RISE of 19 (+0.000005%), and
`work_runbench` 2,369,917,628 -> 2,369,917,611, a fall of 17 (-0.000001%). Both
are
the revision and neither is this change.

Both rows are copied and the four measured-on lines take 8.9:
`bench/instructions_golden.txt`, `bench/compile_instructions_golden.txt`,
`bench/entry_instructions_golden.txt` and `bench/library_instructions_golden.txt`.
This is the shape the runbench note in the first of those already recorded for
8.7 -> 8.8, where one row moved 1,014 instructions and the rest did not, and it
is smaller. Two ubuntu revisions have now been measured across and each moved
one or two of the allocator-heavy rows by tens of instructions; what a revision
costs is a property of the revision, and the gate refuses either way, which is
why both of these were read instead of assumed.

Welfare reads 66.38 against the 66.37750307886598 floor and is not re-set. The
17 instructions `work_runbench` gives back are worth about a ten-millionth of a
point, and they are the revision's rather than this change's. Ratcheting them
would pin a glibc package revision into the floor, so a pool that rolled back
would read fractionally under it and redden CI for a reason no pull request
caused. The rule that a rise is banked is for a gain a change earned; this one
is left where it is, said out loud here rather than passed over.


## 2026-09-09 — the backends build the partial over a value

Built: the 2026-08-29 ruling "the backends build the partial over a value"
(STATUS.md, now removed; the archive entry of that name, Clay: "BUILD IT").
`&f 2` where `f` is a parameter, a local or a lambda settles its arity when
the arguments arrive, on every engine. Until now native and the page refused
it as a limit of their own — "`f` is a value here, and a partial over a
value settles its arity when its arguments arrive — this backend fixes it
where the closure is written" — because both lowered a partial to the lambda
it is equivalent to, and a lambda fixes its parameter count where it is
written. tests/partial.rs and tests/wasm_engine.rs each pinned that refusal
as a limit rather than a judgement about the program; both specs are flipped
to agreement with the oracle.

**Native.** A partial over a value is a closure with no body: `arity` is -1,
which marks it, and the environment holds the callee first and the held
arguments after it, `ncaps` counting both, so the copy and tenure walks that
size an environment by `ncaps` carry it unchanged. `k_partial0..4` build one
— `&f a b` evaluates the callee and then each argument and the first failure
among them is the answer, callee first, the interpreter's order in its
App-with-Partial arm; a bare `&f` wraps whatever `f` holds. `k_call0..4`
gain one branch in front of their arity test: a body-less closure gathers
held and fresh, asks the callee's arity (a closure's field, a fnref's field,
anything else answers nothing, which is what the interpreter's `arities_of`
answers for a partial over a partial or a value that is not callable), and
dispatches through `k_callN` when the count is an arm's, grows when it is
short, and dies past every arm with the oracle's sentence, `no 3-argument
arm of `<fn>` (arms take 2)`. `()` on one runs it or says how many it is
short by, `this function takes 1 argument(s), got 0`, zero when it holds
more than the callee takes, as the oracle says it. `k_call_decided` routes
a partial to `k_call1` for the same reason the oracle's decided path reaches
`call`. The emitter's inlined fast arm tests `arity == n` before it calls,
so a partial never reaches a body through it and the hot path is
byte-identical: `all_compile.sh` on the branch reads nothing moved that this
host can see, and `all_counters.sh` agrees to the counter.

**The emitter.** `partial_lambda` no longer refuses a value; the two call
sites that reach it — `Expr::Partial` and an application whose head is one
— ask `declared` first and route a value to `emit_partial_value`, which
emits the callee as a value, the supplied arguments, and one call to
`k_partial{n}`. The flattening of `(&f 2) 5` into one call, which is right
for a declared group because dispatch happens on the total count, is kept
for groups only: over a value the total is the callee's to settle at run
time, so the held arguments build the partial and the rest reach it through
`k_call`, which is where the count is settled.

**The page.** `rt_partial` builds a slot the same shape — `Slot::C` with a
`PARTIAL` arity of -4, the environment holding the callee and the held
arguments — and `call_closure` and `call_decided` route it to
`partial_apply`, the interpreter's `apply_partial` and `run_partial` in one
function. A group or builtin handed out as a value used to carry arity -1,
which told a partial nothing about when it was finished, so the wrapper
now carries the counts its arms take as a mask below `MASKED_ARITY`
(-1000): `arities_of` reads the bits back, a lambda answers its one count,
and a partial or a cell answers nothing. `call_closure`'s own arity test
reads `arity >= 0`, so a masked wrapper still dispatches on the count it is
handed, as it did.

**Spec.** `tests/golden/micro/a_partial_over_a_value.kso` runs on all three
engines: a parameter holding a three-arm group finished in one call and in
two steps, a two-arm group, `()` on a complete partial, a partial rendered
as `<fn>`, and a lambda held in a local as the callee. Two runtime fixtures
pin the two sentences on native and the oracle:
`a_partial_over_a_value_past_every_arm` and `a_partial_over_a_value_run_short`.
tests/partial.rs gains the four run-time shapes on both engines, and the
page's flipped spec runs the program the two of them agree on and reads
`14`. The two refusal sentences leave the diagnostic scan.

**The page's call head, met on the rebase.** The page had declined any call
whose head is not a name, a keyword or a lambda with `unsupported call head`,
which is exactly the shape a partial over a value arrives in: `(foo add) 5
7`, a call whose answer is the callee. The backend now computes any such head
as a value and calls it, the runtime naming what it cannot call, so the two
page specs and the runtime fixture run on the third engine.

**CI's rows, round one.** Five veins moved and the job named all five.

The three compile rows rise: `compile_instructions` 48,866,385 -> 48,879,362
(+12,977, +0.0266%), `entry_instructions` 162,751,831 -> 162,822,303 (+70,472,
+0.0433%), `library_instructions` 163,473,663 -> 163,543,712 (+70,049,
+0.0429%). `src/runtime.c` is `include_str!`'d into the compiler, so
`k_partial0..4` and the branch each of `k_call0..4` gains are bytes the
compiler carries and compiles, and `src/codegen.rs` and `src/wasm_backend.rs`
change beside it. `compile_allocs` 29,276 and `compile_peak_bytes` 773,818 are
byte-identical: the front end does no more work, it only has more text.

The whole `.text` vein rises, and it was not in the prediction this branch was
pushed with. Every one of the fourteen rows gains 2,096 bytes or 2,432 --
`text_jsonbench` 97,986 -> 100,418, `text_encodebench` 118,834 -> 120,930,
`text_oneshot` 109,858 -> 112,290, `text_basket` 112,018 -> 114,226,
`text_widebench` 124,098 -> 126,194, `text_deepbench` 74,306 -> 76,402,
`text_escapebench` 55,442 -> 57,874, `text_pendbench` 90,738 -> 92,834,
`text_indexbench` 59,458 -> 61,890, `text_scanbench` 158,642 -> 160,738,
`text_digestbench` 109,554 -> 111,650, `text_readbench` 55,970 -> 58,402,
`text_livebench` 110,434 -> 112,866, `text_runbench` 245,858 -> 247,954. The
runtime is linked into every program, so a runtime that grows grows all of
them; the 336-byte spread is the arms a program's own dispatch makes
reachable. The PR body predicted the host-keyed compile rows and stopped
there, which was the wrong list: a change to src/runtime.c moves the compile
rows AND the text vein, and nothing on this container could see either.

**And the work vein FALLS, on ten rows of fourteen, with none rising.**
`work_runbench` 2,369,917,611 -> 2,369,679,773 (-237,838, -0.0100%),
`work_deepbench` 389,214,251 -> 387,470,247 (-1,744,004, -0.4481%),
`work_widebench` 35,332,240 -> 35,268,228 (-64,012, -0.1812%), `work_pendbench`
590,971,748 -> 590,970,940 (-808), and a uniform -8 on `work_jsonbench`,
`work_encodebench`, `work_oneshot`, `work_digestbench`, `work_readbench` and
`work_livebench`. `work_basket`, `work_escapebench`, `work_indexbench` and
`work_scanbench` are byte-identical.

That is the opposite of what the change looks like it should do. It adds a
branch in front of every `k_call0..4` arity test, and the naive reading is that
every native call pays one more test. What moved is the emitter's inlined fast
arm: it tests `arity == n` before it calls now rather than after, which is both
what keeps a partial from ever reaching a body through it and a cheaper test
than the one it replaced. deepbench and widebench build the deepest and widest
structures in the corpus and call hardest, and they carry most of the fall. The
uniform -8 is one instruction on a path six programs take eight times and the
four flat rows never take.

The fall is small and it is measured rather than argued: both sittings are
CI's, on the same glibc 2.39-0ubuntu8.9 image, and the only difference between
them is this commit.

Summed, the vein the gate weighs is `text` 1,523,196 -> 1,554,668, a rise of
31,472 (+2.0662%). That is the whole of the runtime's growth counted once per
program across the fourteen. Machine-code size carries no welfare term, ruled
2026-09-05, so it is watched exactly and priced here rather than traded.

**Welfare falls, and the floor moves under the language clause.**
66.37750307886598 -> 66.37657452327063, a spend of 0.00093. The run term
improves and the compile term pays more: `work_runbench` falls 0.010% where
the three compile rows rise 0.039% together, and the smaller relative move on
the heavier, later-satiating term does not cover the larger one on the lighter.
`compile_allocs` and `compile_peak_bytes` are byte-identical, so the front end
does no more work; it has more text to compile.

The floor moves to the reading because a RULED feature spent it, which is the
2026-08-25 clause and the same ground #1359 and #1356 moved on. It is not a
claim that the weights are wrong: the objective is reporting the trade
correctly, and a partial over a value on every engine is worth 0.00093 of it
by Clay's own "BUILD IT". The first cut of this entry recorded the move as a
`--set` with a reason that read like a banked gain, which was wrong about the
direction; the history entry says the fall and its size now.
## 2026-09-09 — a succeeded effect yields `done`

Built: the 2026-08-26 ruling "the July letters close", letter D (STATUS.md's
row "`done` is minted", now removed): a succeeded effect yields `done`, and
`none` means absence and nothing else. Until now a print, a write, a sleep, a
`make_dir`, a `write_file`, a kill or a socket close yielded `none` on every
engine, so a chain that bound the yield could not tell a finished effect
from a missing value, which is the railway-skip the ruling closed.

**The value.** `done` is the fourth nullary beside `true`, `false` and
`none`: a literal, an arm's pattern, an annotation's type word and a typeset
member. It renders `<done>`, the way `none` renders `<none>` and a
description `<io>` — a value that is not data prints in brackets, so a
yield that reaches an interpolation is visible for what it is rather than
passing as a word. It equals itself and nothing else; `done == none` is
false on every engine. Infer carries it as its own bit, `DONE`, above the
thunk bit, and the twelve effects that used to yield the empty set yield
`DONE` now: the empty set was how the decided dispatch chose the catch-all
arm for a yield, and with the bit in place a `done` arm is reached on
native the way the oracle reaches it. Native's tag is `K_DONE`, appended
after `K_SUB` so every existing tag keeps its number; the page spells the
literal `Lit::Done` and answers type code 9 for it; its effects run through
the interpreter's executor, so the page's yields moved with the oracle's.

**Twelve effects on two executors.** The interpreter's `execute` and
native's `k_exec` each answer `done` for print, write, write_err, sleep,
the nil description a sleep settles into, make_dir, write_file,
write_bytes, kill, send, send_bytes and the socket close. A read that finds
no file, an environment variable that is not set and `read_bytes` on a
missing path still answer `none`, since those are absences.

**`done` was already a name, twice.** `std/list` declared `pub type done`
with one field, `drained`, and every construction in the module was `done
true`: a record carrying no information, matched by `(done _)` in eleven
arms and re-answered as `done d`. design/enumerable.md §8 had spelled the
iterator's end bare, `next src . (done -> acc)`, all along. So the record
is gone and `next` answers the nullary when a source is drained; the
arms read `done` and the constructions are the word. `std/net/http`'s
`done value` carries the value a serve loop stops with, which is a
different thing, so it is `stopped value` and `http/stop` builds it, with
both callers in scripts/ unchanged since they call `http/stop`.
`std/regexp` used `done` as the name of a repetition count in one arm
family, twenty-five sites; it is `reps`. examples/next_protocol.kso reads
`fn drain done acc`. Anything outside this repository that matched
`(done _)` on a list iterator reads `done` now, and a binding named `done`
is refused with the sentence every other taken name gets,
`done_is_a_value_not_a_name` in the error corpus.

**Spec.** `tests/golden/micro/a_succeeded_effect_yields_done` on all three
engines: a print's yield and a write's yield bound and printed, `<done>`,
equal to `done`, not `none`, and dispatched to an arm that names `done`
beside the arm that does not. The old compiler refuses the file at the
arm, `unused binding done`, since `done` was a parameter name to it.

**What moved.** The list record was one allocation per drained source, so
three runtime cost goldens fall by exactly the drains their programs make:
basket 28,169 -> 28,166 allocs, pend 4,007,549 -> 4,007,349, run
6,936,567 -> 6,936,517, and four `.mem` fixtures with them
(`a_carried_value_written_into_an_older_node` 100,550 -> 100,149, the
other three the same shape). Eight programs that import std/list emit less
now that eleven arms match a tag instead of destructuring a record:
encodebench 1,638 -> 1,607 calls and 11,197 -> 11,105 lines, basket,
widebench, deepbench, pendbench, scanbench, digestbench and runbench each
31 to 40 calls and 92 to 103 lines fewer, branches down 8 or 9 apiece;
the decoder does not import it and is byte-identical. The module compile
golden reads lines 5,269 -> 5,177, calls 758 -> 727, branches 437 -> 430
and `module_visits` 2,409 -> 2,471, and `compile_memory`'s
`front_end_visits` 22,426 -> 22,562, the page's span rewritten: a nullary
in an arm is a cheaper emit and a
slightly longer inference, since `DONE` is a bit the fixpoint carries
where a record was a type it did not. **Re-measured on the base this opens against.** The rows above were first
counted on main 3c7a163b and this change opens against dc980ddf, three merges
later, so `all_compile.sh` was re-run and the two veins it can see were
regenerated here: `emitted_code` on the eight programs and `compile_cost`'s
module row. The deltas are dn's own and did not move -- 31 to 40 calls and 92
to 103 lines fewer, visits 2,409 -> 2,471 -- but two LEVELS did, by the two
lines a program kanso#1361's alias added: encodebench's lines read 11,197 ->
11,105 where the first sitting read 11,195 -> 11,103, and the module row
5,269 -> 5,177 where it read 5,268 -> 5,176. `front_end_visits` 22,426 ->
22,562 is measured rather than projected: `compile_memory` checks rounds and
visits before it asks the host anything, because those count the compiler's
own algorithm and are the same everywhere, and only its `compile_peak_bytes`
row refused.

**CI's sitting, and a prediction that was six-sevenths right.** Round one named
SEVEN veins and this entry had named six of them: the four host-keyed compile
rows, `compile_memory`'s peak, and `machine_code`. The seventh was `work`, the
runtime instruction vein, and the miss is instructive because the entry's own
record implies it: a change that removes one allocation per drained source
removes the instructions that allocation cost. The seventh name written above
was the `text` vein, which is not a seventh gate at all -- `machine_code.sh`
diffs bench/text_golden.txt, so the two are one. Six gates predicted, seven
named, and the arithmetic was wrong in both directions at once.

Every one of the thirty-nine moved keys, by name:

    work           encodebench 3,963,988,518 -> 3,963,988,451        -67
                   basket         34,694,178 ->     34,690,245     -3,933  -0.0113%
                   widebench      35,268,228 ->     35,268,161        -67
                   deepbench     387,470,247 ->    387,470,234        -13
                   pendbench     590,970,940 ->    583,758,224 -7,212,716  -1.2205%
                   scanbench     726,019,157 ->    726,019,079        -78
                   digestbench    10,426,541 ->     10,426,515        -26
                   runbench    2,369,679,773 ->  2,367,877,484 -1,802,289  -0.0761%

    text           encodebench       120,930 ->        120,098       -832
                   basket            114,226 ->        113,170     -1,056
                   widebench         126,194 ->        125,362       -832
                   deepbench          76,402 ->         75,538       -864
                   pendbench          92,834 ->         91,346     -1,488
                   scanbench         160,738 ->        159,842       -896
                   digestbench       111,650 ->        110,530     -1,120
                   runbench          247,954 ->        246,402     -1,552
                   jsonbench         100,418 ->        100,434        +16
                   oneshot           112,290 ->        112,306        +16
                   escapebench        57,874 ->         57,890        +16
                   indexbench         61,890 ->         61,906        +16
                   readbench          58,402 ->         58,418        +16
                   livebench         112,866 ->        112,882        +16

    compile_allocs                     29,276 ->        29,000       -276  -0.9427%
    compile_instructions           48,879,362 ->    48,572,851   -306,511  -0.6271%
    entry_instructions            162,822,303 ->   161,836,689   -985,614  -0.6053%
    library_instructions          163,543,712 ->   162,541,723 -1,001,989  -0.6127%
    compile_peak_bytes                773,818 ->       769,071     -4,747  -0.6135%
    front_end_visits                   22,426 ->        22,562       +136  +0.6064%

jsonbench, oneshot, escapebench, indexbench, readbench and livebench do not
move on the work vein at all, and they are the six that rise exactly 16 bytes
on the text vein. That is the whole shape of the change in two lines: the +16
is `K_DONE`, the tag appended after `K_SUB` in src/runtime.c, which links into
every program whether or not the program can produce the value; the eight that
fall are the eight that import std/list, where eleven arms match a tag instead
of destructuring a one-field record. pendbench takes 1.22% of the work vein
because it drains one source per pending cell, where the rest drain a handful.

The five compile rows fall in one band -- 0.9427%, 0.6271%, 0.6053%, 0.6127%,
0.6135% -- which reads as work removed rather than as the layout move the
compile-row notes usually record: a declaration the front end used to lex,
parse, infer and check is gone. `front_end_visits` goes the other way by 136,
and that is deliberate: `DONE` is a bit the inference fixpoint carries where a
record was a type it did not, so a nullary in an arm is a cheaper emit and a
slightly longer inference. Only the peak is a welfare term.

Welfare 66.38 -> 66.42, banked with `--set` in this same PR. All five terms
improved: `run_instructions` and `compile_peak_bytes` by the deletions above,
`compile_instructions` (the objective's sum of the module, entry and library
rows) by 2,294,114, and `compile_allocs` by 276.

**A blank line cost a round.** Round two took all seven veins' rows and CI came
back with six green and `compile allocations` still red -- on a golden whose
value matched CI's to the digit. The diff was `1d0` against a blank: the note
above the row had been separated from the note before it by an empty line, and
`compile_allocs.sh` builds its expected file with `grep -v '^#'` and nothing
else, so the blank survived into the comparison. The other four compile goldens
carry blanks today and their gates do not mind. Round three deletes the blank
and writes the trap into that golden's own header, because the habit it broke --
separate notes with an empty line -- is right in every other file here.
## 2026-09-09 — the fused chain operators

Built: the 2026-08-31 gavel "the fused chain operators" (archive; STATUS.md's
row, now removed). In chain position each of the three words has one
spelling, the chain dot plus one character of channel: `.>` is `bind`, `.!`
is `annotate`, `.?` is `rescue`.

    settled = json/decode text
      .> count_of
      .! (e -> "settled: {e.reason.reason}")
      .? (e -> "fell back on {e.reason}")

**What the parser learned.** The lexer reads `.` pressed against `>`, `!` or
`?` as one token carrying the word it stands for, and lets that token lead a
continuation line the way the bare dot does. `.!` and `.?` desugar to the
ordinary applications `annotate x f` and `rescue x f`, which every engine
already speaks prefix-style (kanso#1116), so a desugared step reaches the
same `k_b_annotate`, `Desc::Rescue` and `rt_rescue` the prefix form reaches
and the emitters, the runtime and the page are untouched. The right-hand
side is one function, a lambda, a name or a group: `.> f x` is refused with
the partial named, since the gavel's common case is the bare function and a
held argument already has a spelling, `&f x`. A `. bind f`, `. annotate f`
or `. rescue f` step is refused with the fused form named. In chain position
that spelling retired, as ruled, superseding the 2026-08-29 keep-the-dot
ruling for the three words. The words remain prefix functions everywhere
else, and every fixture that calls them that way is unchanged.

**`.>` desugars to the piped step, and the measurement that decided it.**
The first cut desugared `x .> f` to the application `bind x f` like the
other two. On the interpreter that is the same program: `bind` on a
description builds `Desc::Bind`, on a settled failure skips the callback,
on a value calls it, which is what the automatic bind does at a piped step
in each case. Native did not agree. The beat keys on a piped lambda — `App
{ piped: true, head: Lambda }` is the shape beat.rs and infer.rs read as a
chain step whose body is the loop — and a lambda handed to `bind` as an
argument is an escaping closure to both, so the arena stopped rewinding:
with the tree respelled (below) the decoder's `beat_iters` read 151 -> 1
and its `arena_peak_bytes` 2,097,152 -> 251,658,240, runbench's 45,944,528
-> 209,522,384, and welfare fell 9.23 points. So `x .> f` desugars to the
piped application, `App { head: f, args: [x], piped: true }`, the node the
automatic bind has always been, and every pass reads it as it did. The
gavel's sentence holds — `x .> f` IS `bind x f` on every input — and the
node that carries it is the one the compiler already optimises. When the
effect type ends the automatic bind, the plain-dot step over a box will
mean something else and this node will need its own flag; that build owns
the flag, and the emitter's `k_b_bind` stays the prefix word's.

**Every automatic bind is spelled `.>`.** With the spelling in, infer's
piped-over-DESC branch was instrumented to print the file and the head
position of every step it bound automatically, and the tree was checked
program by program (every scripts/ directory, hako, bench, the examples,
the book samples and the golden corpora; the play files through `kanso
play`): 519 sites, 372 with a lambda on the right and 147 with a bare name,
none with a held argument. A script rewrote the dot at each head position
to `.>` — 498 dots, since a chain of several steps reports one site per
step — and a second census under the same instrument read zero. That zero
was over the programs the sweeps compile, and it was not the tree. A dot
followed by a lambda or a name is a grep, and compiling every file that
grep names under the same instrument found 44 more dots in 22 files that
no sweep reaches: the benchmark sources `build_benchmarks.sh` builds and
nothing checks (`total 4000 . (t -> io/write "{t}\n")`, the deep, pend
and wide bodies, and the main text `make_jsonbench` writes out), the
book's ch09 `vse` sample, the trace-demo example, the two workahead
programs, and 56 more in the programs thirteen Rust specs under tests/
carry as string literals, which no census of `.kso` files could reach,
and one in the playground's dice example in docs/play.js. Those are
respelled too — 599 dots in 132 files — and the grep census reads zero. The lesson is the one §30 already records: a dynamic census
counts the programs that ran, so the check is the static list of
candidates, each one compiled. Four statements crossed 80 columns and
wrap onto continuation lines; nothing else changed, and no golden moved:
the piped node is the one those goldens were measured on. The plain-dot
steps whose subject infer reads as a value keep their dot — `"kanso" .
greet . print`, `expect 42 . to (equal 42)` — because under the effect
type a plain-dot step over a value stays an application and only the
steps over a box change meaning. The book's panels were regenerated and
the sentences beside them that named the dot as the way into an effect
now name `.>`; ch04 and ch05's teaching of the railway itself waits on the
effect type, as the ledger says. The book's highlighter tokenised `.>` as a
dot and a `>` and lost the name after it, so `scripts/book_panels` reads
the three fused spellings as one operator token and a name in front of one
as a head; the 29 panels that carry one are re-rendered. The panel check
compares a panel's text with its sample, markup stripped, so a highlighter
change moves no panel on its own; four panels in ch03 and appa carry
markup the highlighter cannot produce (an ascription's type, a name after
an operator, a bare name on its own line), which is why the check stays
on the text.

**Spec.** `tests/golden/micro/a_chain_step_names_its_channel.kso` runs on all
three engines, settled failures only: a group as the bare right-hand side of
`.?` and `.>`, two lambdas, a value bound twice, and the chain above on a
success and on a failure. `tests/golden/chainwords/a_fused_chain_over_an_effect.kso`
is the log's `. rescue orders` program in the ruled spelling: a missing file,
`.? orders .> shout .> print`, prints `no orders yet!!` on native and the
oracle. Two error fixtures pin the refusals, `a_chain_word_is_spelled_fused`
(`. rescue told`) and `a_fused_step_takes_one_function` (`.> told 3`). The
compiler page's combinators section says what landed.

**Three libraries changed, so the compile rows are CI's.** `lib/net/net.kso`,
`lib/net/http/http.kso` and `lib/os/os.kso` respell their chain steps, and
`lib/*.kso` is `include_str!`'d into the compiler, so those are bytes the
compiler carries and compiles. The two compile-side veins this host can read
are unmoved -- `emitted_code` on all fourteen programs and `compile_cost`'s
module and modules rows -- which is what a respelling that changes no emitted
code does. The six host-keyed rows (`machine_code`, `compile_memory`,
`compile_allocs`, `compile_instructions`, `entry_instructions`,
`library_instructions`) refuse here and are expected to move on CI, by layout
at minimum; whatever it reports is copied in from its sitting rather than
projected from this container, which takes clang 18.1.3 against CI's 19.1.1.

**Cherry-picked onto 0d7b3e57, not rebased.** The branch sat on `local/done`,
whose content main now carries in squashed form. The pick asked for two
resolutions, both the log's and the archive's both-append hunks;
`lib/net/http/http.kso` auto-merged, because the collision integration had
predicted there was `done`'s `stopped` against this change's `.>` on one line
and `done` is on main now. Re-verified on the picked tree rather than on the
branch's own worktree: that worktree carries `done`'s code against `done`'s
pre-change module golden, since dn's own commit moves the row and the
regeneration happened later, so `compile_cost` is red there and green here.

**CI named THREE of the six, and the three that held say what the change is.**
The prediction above reasoned from the library edit alone -- three `lib/*.kso`
files change, `lib/*.kso` is `include_str!`'d into the compiler, so all six
host-keyed rows are in play -- and that is right about which rows CAN move and
silent about which will. What moved:

    compile_instructions   48,572,851 ->  48,393,437   -179,414  -0.3693%
    entry_instructions    161,836,689 -> 161,314,264   -522,425  -0.3228%
    library_instructions  162,541,723 -> 162,023,537   -518,186  -0.3188%

and `compile_allocs` (29,000), `compile_peak_bytes` (769,071) and the whole
`text` vein are BYTE-IDENTICAL in the job that counted those three. The front
end allocated exactly the same and held exactly the same doing this compile, and
the linker emitted the same machine code, so nothing about the work changed;
what changed is src/lexer.rs and src/parser.rs, and the three instruction rows
are the layout veins CLAUDE.md's prior describes. Three rows falling together in
one narrow band with the allocation row still is the layout signature; real work
removed would have moved allocs with them, the way `done` did an hour earlier.

Welfare 66.42 held and re-set: the objective's compile term sums the three rows,
so it takes the whole 1,220,025, and the rise is under a hundredth of a point.
Five compiler.html spans quoting the three rows were rewritten by
`golden_prose --write`.

## 2026-09-10 — a record built into its own first field

Found while writing a mem fixture for the carry tier: a loop that chains
records, `grow (node n i) (i + 1)`, came out cyclic on native and correct on
the oracle, on main as on the branch. The emitter builds a constructor into
a record the arm has finished with — `shift (n - 1) (point (p.x + 1) p.y)`
reads every field it needs before the constructor runs, so by then `p` is
done with and its storage is free — and `sole_finished_record` judged
finished by counting mentions: the parameter read in this constructor's
arguments and nowhere else in the arm, and handed over by every caller. A
bare mention counted as a read. `node n i` mentions `n` once, in the
constructor, and every caller hands it over; the emitter passed it to
`k_rec_reuse` as the victim, and the runtime overwrote the victim's fields
with arguments the first of which was the victim. The new node's `next`
pointed at its own storage, and `show` walked it until the stack ran out.

**The fix** is in the analysis. Finished means read from, and a field read
is the only read that leaves nothing behind: `field_reads_in` counts the
mentions that are the base of a `var.x`, and a parameter whose mentions in
the arguments are not all field reads is not a victim. A bare mention stores
the record itself, whether directly or inside another constructor, and a
record something is about to hold is not free. The runtime is unchanged;
`k_rec_reuse` still trusts its caller, which is the arrangement the
2026-09-04 attribution (k_rec 30.39% of pendbench) chose.

The first cut of that rule declined every reuse in the tree — basket's
`sh_rec` 130,192 -> 386,192, `record_reuse_shape`'s 4,006 allocations back
where 3 had been — and the reason is worth a line for the next pass written
against this AST. `desugar_field_reads` runs before the linear analysis,
and after it there is no `Expr::Field` anywhere: `p.x` is `Get_x p`, the
getter applied to the record. A count of `Expr::Field` reads zero on every
program. `field_reads_in` reads both spellings now, the getter application
by `getter_field` on the head's name, and the veins agree.

One site in the tree was the defect's shape and never showed it.
lib/json/scan.kso's `fail p reason` builds `err (parse_failure p reason)`,
and `reason` — mentioned once, bare, in the constructor's arguments, handed
over by every caller — was the victim at that site. It is a string at every
call, and `k_rec_reuse` allocates fresh when the victim is not a record of
the constructor's width, so the program was right by the runtime's guard
rather than by the analysis; a caller passing a two-field record as the
reason would have built the failure into its own `reason` field. The site
emits `k_rec` now, which is the one line the emitted veins lose.

**Spec.** `tests/golden/micro/a_loop_that_chains_records_keeps_each_node`
on all three engines, `3>2>1>0>end`; watched red on native (the stack ran
out) with the oracle green before the fix, both green after.

**Cost.** None at run time: `all_counters.sh` reads the twelve cost veins
and the lazy tier agreeing with their goldens, and welfare holds at 66.31.
The emitted veins each lose the `k_rec_reuse` line at lib/json's `fail`,
landed at: emitted_lines 9,135 (from 9,136), and in the others vein
encodebench 11,142, oneshot 9,064, widebench 12,124, pendbench 7,033,
scanbench 19,728, livebench 9,181 and runbench 34,673 lines, one fewer
each; defines, calls and branches hold, because the `k_rec_reuse` declare
that goes is not a define and its call is replaced by a `k_rec` call. The
six host-keyed compile rows are refused on this container and copied from
CI's sitting.

**CI's five host-keyed rows, and what each one says.** The container refuses
five of the veins this change moves, so round one was red on all five and CI's
sitting is what lands. Two go down and three go up.

The runtime rows fall in four of fourteen programs and hold in ten: basket
34,690,245 -> 34,690,216 (-29), pendbench 583,758,224 -> 583,755,724 (-2,500),
scanbench 726,019,079 -> 726,018,879 (-200), runbench 2,367,877,484 ->
2,367,876,664 (-820). Machine code falls in nine and holds in five, jsonbench
100,434 -> 100,130 the largest at -304. NINE against the emitted vein's EIGHT:
basket loses sixteen bytes of text without losing an emitted line, because the
`k_rec_reuse` declare it drops was already text some other program shared.

The three compile rows RISE, together and by nearly the same fraction:
compile_instructions 48,393,437 -> 48,412,144 (+18,707, +0.0387%),
entry_instructions 161,314,264 -> 161,360,451 (+46,187, +0.0286%),
library_instructions 162,023,537 -> 162,069,092 (+45,555, +0.0281%). That is
the price of the answer. `sole_finished_record` used to count mentions, which
costs nothing; `field_reads_in` asks of each mention whether it is the base of
a field read, which walks. Every compile route runs the linear pass, so all
three rows move, and a per-declaration check that moves them by the same
fraction is what a uniform cost looks like.

**The trade, and the objective's verdict.** A rise anywhere is a thing to
state rather than defend, and this one is real: the compiler does 110,449 more
instructions summed across the three routes to buy 3,549 fewer at run time on
the four programs that reach the shape. Stated that way it sounds like a bad
bargain, and by the raw counts it is. The objective disagrees, because the
counts are not what it weighs: welfare reads **66.42 against a floor of 66.42,
held exactly**, since a 0.03% rise on a compile term measured in hundreds of
millions moves the saturating curve by less than the floor's own precision.
The change is a MISCOMPILATION fix besides — a program that came out cyclic
now does not — and that is not a term the objective has at all.

## 2026-09-10 — three library shapes off the run program's profile

Three arms, each one a shape the profile pointed at, each one measured on
today's main rather than on the base they were first cut against.

**A cap around a count is a range.** pendbench's churn fuses to a fold over
`take naturals n`, and the general capped arm pulled every element through
`next`, building a count, a step, a cap and another step apiece — four records
read once and dropped. `fold (capped left (counting at))` counts instead, the
sibling of the window arm already beside it.

**A class asks by the byte.** std/regexp's character classes walked a string
per candidate; the arm reads the byte.

**The colon is the arm before the error.** lib/json's `obj_key` ran `skip_ws`
to the colon, took back only its position, and then had `expect_char` load the
same byte a second time to compare it. `obj_key_end` is the shape `array_delim`
and `obj_delim` already take: the colon is an arm, the whitespace is the arm
before it, and the error is the arm after.

**A deletion at the head of a library file moves every diagnostic below it.**
Dropping `import "std/text"`, `expect_char` and both `expect_check` arms takes
eleven lines off the top of lib/json/scan.kso, so `fail` moves from line 13 to
line 2 and the endpoint diagnostic that names its birthplace moves with it:
`tests/golden/runtime/a_lone_surrogate_is_half_a_character.stderr` now reads
`born in json/fail at std/json/scan.kso:2`. Two tests read that one golden —
`runtime_corpus_reports_endpoint_violations` in tests/golden.rs and
`interpreter_reports_each_runtime_endpoint_violation` in tests/oracle.rs — and
both went red until it was regenerated. Nothing else in the tree quotes a
scan.kso line. Worth writing down because no counter, gate or sweep can see it:
the change is a deletion of dead helpers, and it moved a user-visible sentence.

**What they cost, on this tree over origin/main ef2f4ea4.** The arms were cut
against 32080acf and re-measured after #1366 merged in. Not one runtime counter
moved between the two bases: #1366 changed which constructor `fail` emits, not
what anything allocates.

    pendbench   allocs      4,007,349 ->   807,149   (-79.86%)
                alloc_bytes   257 MB  ->     65 MB   (-74.66%)
                arena peak      2 MB  ->      1 MB
                sh_rec    192,067,248 ->    54,448   (-99.97%)
    scanbench   allocs      3,975,884 -> 3,011,152   (-24.26%)
                arena peak    198 MB  ->    161 MB   (-18.52%)
                sh_str     37,039,216 ->     1,632
    runbench    allocs      6,936,517 -> 5,958,961   (-14.09%)
                arena peak   45.9 MB  ->   39.7 MB   (-13.70%)

`sh_rec` falling by a factor of three and a half thousand on pendbench is the
counted fold: those four records an element were most of what the benchmark
allocated. `sh_str` on scanbench is the byte class, which is why that row goes
to almost nothing rather than down a fraction.

**And the instructions, which this container could not see at all.** The work
vein is host-keyed and refused here, so welfare read main's row and reported
the floor merely held; CI's sitting is where the change actually shows.

    pendbench    583,755,724 ->   226,874,035   -356,881,689  -61.1355%
    scanbench    726,018,879 ->   587,488,478   -138,530,401  -19.0808%
    runbench   2,367,876,664 -> 2,262,265,575   -105,611,089   -4.4602%
    jsonbench  1,485,334,791 -> 1,470,973,791    -14,361,000   -0.9669%
    oneshot       21,737,110 ->    21,641,370        -95,740   -0.4404%
    deepbench    387,470,234 ->   387,090,234       -380,000   -0.0981%
    basket        34,690,216 ->    34,672,719        -17,497   -0.0504%
    livebench  3,481,899,695 -> 3,481,803,955        -95,740   -0.0027%

Eight of the fourteen fall and none rises. The other six are byte-identical:
encodebench, widebench, escapebench, indexbench, digestbench and readbench
reach none of the three arms. pendbench losing three fifths of its instructions
is the counted fold alone -- the four records an element were not only
allocated but walked.

**Welfare 66.42 -> 67.56**, the largest single move since the 2026-09-03
rebuild, banked with `--set` in this same PR. Both the allocation terms and
run_instructions pay for it; the container's blind reading had put the gain at
0.86 and the whole of that was the memory side.

**The compile side pays, and the emitted code grows.** Three new arms is three
more declarations for the front end to visit: front_end_visits 22,562 -> 22,727
(+165, +0.7313%) with rounds holding at 62, the arms being wider rather than
deeper so nothing re-converges. Every program that compiles std/list carries
the counted arm, so the emitted rows rise across the board — encodebench 11,104
-> 11,232 lines, scanbench 19,673 -> 19,922, runbench 34,726 -> 34,979. The
decoder is the one row that gains lines while LOSING calls: 9,246 -> 9,272 with
calls 1,236 -> 1,226, because it reaches the colon arm and neither of the other
two. Four rows are byte-identical — escapebench, indexbench, digestbench and
readbench reach none of the three. That is the trade stated plainly: more code
written, much less allocated.

**Every counter that went the wrong way, with the value it landed on.**

The byte class trades string work for byte work, so two counters that were
zero on scanbench are no longer zero and the same pair rises on runbench, which
runs split: `scan_find2_calls` 0 -> **501,505**, `scan_sh_bytes` 0 ->
**24,072,240**, `run_find2_calls` 3,017,520 -> **3,109,759**, `run_sh_bytes`
36,862,800 -> **41,290,272**. Against them `scan_sh_str` falls 37,039,216 ->
1,632 and `scan_sh_buf` 24,105,200 -> 33,024. The bytes a class reads are the
bytes it used to read as a string, and it stops copying them.

The compile-module vein carries the three new declarations: `module_defines`
100 -> **101**, `module_calls` 727 -> **754**, `module_branches` 430 -> **446**,
`module_lines` 5,177 -> **5,304**, `module_visits` 2,471 -> **2,534**, beside
`front_end_visits` 22,562 -> **22,727**.

Three more declarations cost the compiler itself, and these are CI's rows:
`compile_instructions` 48,412,144 -> **49,091,884** (+679,740, +1.4040%),
`entry_instructions` 161,360,451 -> **164,041,672** (+2,681,221, +1.6616%),
`library_instructions` 162,069,092 -> **164,324,373** (+2,255,281, +1.3910%),
`compile_allocs` 29,000 -> **29,350**, `compile_peak_bytes` 769,071 ->
**774,660** (+5,589, +0.7267%). The objective's compile term is module plus
entry, 209,772,595 -> 213,133,556, a rise of 1.60% against a run-instruction
fall of 4.46% on a term that satiates late; the trade is not close.

The `text` vein sums 1,543,836 -> **1,547,852** across the fourteen, and it
moves both ways by families: **+800 or so** where the
counted fold lands (deepbench 75,538 -> 76,338, digestbench 110,530 ->
111,330, encodebench 119,826 -> 120,642, widebench 125,090 -> 125,906,
runbench 246,082 -> 246,914, pendbench 91,090 -> 91,778, basket 113,154 ->
113,650), **-400** where the colon arm does (jsonbench 100,130 -> 99,730,
oneshot 112,002 -> 111,602, livebench 112,578 -> 112,178), and scanbench
159,602 -> **159,570**, the one row that carries the byte class and comes out
32 bytes smaller. escapebench, indexbench and readbench are byte-identical.
The machine code grows where an arm is added and shrinks where three helpers
go, which is what the emitted vein says in lines.

The emitted veins are the same three arms written out: `emitted_branches` 816
-> **819** and `emitted_lines` 9,246 -> **9,272** on the decoder;
`emitted_other_defines` 2,354 -> **2,366**, `emitted_other_calls` 20,245 ->
**20,445**, `emitted_other_branches` 12,639 -> **12,800**, `emitted_other_lines`
132,530 -> **133,802** across the thirteen beside it. `emitted_calls` on the
decoder FALLS, 1,236 -> 1,226.

Two rows of the inner-beat tenure fixture move with the counted fold, which is
what that fixture folds:

    an_inner_beat_opens_its_tenure_in_the_block_outside_survive_slots 2,004 -> 32,404
    an_inner_beat_opens_its_tenure_in_the_block_outside_ten_frees 3 -> 2

with `ten_blocks` 3 -> 2 beside them. The fold stops building four records an
element, so far less
is allocated inside the beat (allocs 512,494 -> 177,420, alloc_bytes 41.2 MB ->
16.9 MB) and the beat runs a third as many iterations (158 -> 68); what survives
a longer-lived block is counted in more slots and freed in one fewer.

**Five veins are CI's.** machine_code, compile_allocs, compile_instructions,
entry_instructions and library_instructions are refused on this container, and
compile_memory_golden.txt is refused whole (measured-on rustc=1.98.1, here
1.94.1) even though the visits row it carries is host-invariant. Round one is
red on those and CI's sitting is what lands.

## 2026-09-10 — a validated string knows its length

The utf-8 validator already classifies every byte on its way through. Counting
the continuation bytes while it does gives the character count for free on the
ascii path and for two vector instructions a block on the wide one, and that
count seeds the memo `k_str_chars` would otherwise fill by scanning the string
a second time. `length` on a validated string never walks it.

Three doors seed: `read_file`, `k_b_utf8`, and the whole-string check. The
decoder's token door in `k_b_utf8_slice_raw` deliberately does not. Those are
861,498 slices on runbench and none of them is ever asked its length, so
seeding each one measured 7,728,237 instructions against the 13,280,580 the
other three save. The comment at that call site says so, because the omission
looks like an oversight and is not.

**The counters.** `str_scans` goes to zero on the encode, live and oneshot
programs (400, 400 and 1 scans, 75,479,200, 75,479,200 and 188,698 bytes), and
on the run program falls 254 -> 163 with `str_scan_bytes` 22,644,612 ->
5,473,094. Nine lazy-tier fixtures move those two rows and nothing else: no
allocation counter, no peak, no evacuation counter differs anywhere. The ninth
is kanso#1367's own `a_class_asks_by_the_byte`, which lands 7 scans and 12
scanned bytes where it read 8 and 2,212 — the byte class validates a string
and then asks its length, so the two changes meet on one fixture.

**Instructions, measured here under callgrind.** Both compilers built in the
same worktree, both binaries run from the same directory under the same
filename with the environment emptied, so the fixed fourteen-instruction offset
that a two-worktree A/B puts on every row is not in these numbers.

    livebench    3,484,129,797 -> 3,452,753,847   -31,375,950  -0.9006%
    encodebench  4,084,100,523 -> 4,052,767,921   -31,332,602  -0.7672%
    pendbench      225,398,305 ->   220,436,506    -4,961,799  -2.2015%
    runbench     2,241,481,220 -> 2,232,013,849    -9,467,371  -0.4224%
    jsonbench    1,438,095,130 -> 1,436,454,329    -1,640,801  -0.1141%
    oneshot         21,417,863 ->    21,396,929       -20,934  -0.0977%
    readbench        4,562,212 ->     4,631,248       +69,036  +1.5132%
    widebench       36,429,055 ->    36,509,001       +79,946  +0.2194%
    deepbench      395,527,105 ->   395,911,106      +384,001  +0.0971%
    basket          34,861,960 ->    34,891,911       +29,951  +0.0859%
    indexbench       3,226,185 ->     3,226,248           +63
    digestbench     10,497,723 ->    10,497,757           +34
    scanbench      600,502,339 ->   600,502,367           +28
    escapebench     85,489,183 ->    85,489,184            +1

runbench is the objective's whole run-speed term and it falls 0.4224%.

**The same fourteen deltas, to the instruction, on both bases.** This was
measured once over ef2f4ea4 and again over 6c32079a with kanso#1367's three
library arms in between, and every one of the fourteen absolute deltas is
identical: -31,375,950 on livebench both times, -4,961,799 on pendbench, +69,036
on readbench. The percentages move because the bases did — pendbench reads
-2.2015% here against -0.8568% before, since kanso#1367 took three fifths of
that program away and the same saving is now a larger share of what is left.
Worth writing down: the two changes touch disjoint work, so neither measurement
had to be redone for correctness, only for its denominator.

**readbench is the pure-cost case, and it is worth naming rather than
averaging away.** Its +69,036 is `k_utf8_bad_wide` and nothing else:
274,748 -> 343,792 under `callgrind_annotate`, which is the whole delta to
within eight instructions. readbench reads one file whose bytes take the wide
path and never asks its length, so it pays the two vector instructions a block
and collects nothing. widebench, deepbench and basket rise for the same reason
in smaller amounts. The counting is cheap where the answer is wanted and not
free where it is not, and four benchmarks are on the wrong side of that.

**Welfare cannot be read for this change on this container.** `run_instructions`
comes from `bench/instructions_golden.txt`, which is host-keyed and refused
here, so the objective reads main's row whatever the tree does — a change whose
entire effect is instructions is invisible to a local `welfare` run. The number
that matters is CI's, on CI's rows.

**CI's sitting, and the twelve counters that rose.** The container refuses the
work vein, the text vein and the three compile rows, so round one was red on
all five and CI's numbers are written into the goldens here. Six of the
fourteen work rows fall — `work_encodebench` 3,932,651,503 (-0.7905%),
`work_livebench` 3,450,423,659 (-0.9012%), `work_runbench` 2,252,446,969
(-0.4340%), `work_pendbench` 221,912,236 (-2.1870%), `work_jsonbench`
1,468,801,090 (-0.1477%), `work_oneshot` 21,616,888 (-0.1131%). Eight rise, and
they are the accumulator the validator carries for strings whose count nobody
asks for: `work_readbench` 4,630,969 (+69,036), `work_deepbench` 387,474,235
(+384,001), `work_widebench` 35,316,107 (+47,946), `work_basket` 34,698,668
(+25,949), `work_indexbench` 3,265,849 (+63), `work_digestbench` 10,426,549
(+34), `work_scanbench` 587,488,506 (+28), `work_escapebench` 85,558,106 (+1).

`text` 1,547,852 -> 1,551,324, a rise of 3,472 bytes spread over all fourteen
rows — jsonbench 100,050, encodebench 120,962,
oneshot 111,922, basket 114,082, widebench 126,226, deepbench 76,354,
escapebench 57,922, pendbench 92,034, indexbench 62,162, scanbench 159,842,
digestbench 111,362, readbench 58,434, livebench 112,498, runbench 247,474 —
most by 320 bytes and runbench by 560: the
three utf-8 arms carry a fourth parameter and a conditional store, and they
live in src/runtime.c, which every program links. The three compile rows rise
by the same fraction and for the same reason — `compile_instructions`
49,097,584 (+5,700, +0.0116%), `entry_instructions` 164,060,471 (+18,799,
+0.0115%), `library_instructions` 164,342,505 (+18,132, +0.0110%). The front
end does no more work; runtime.c is carried inside the compiler, so its bytes
and the layout under them move when it changes. Welfare reads 67.59 against a
floor of 67.56 and is ratcheted to it.

**Eight of the fourteen deltas match the container to the digit, six do not.**
deepbench +384,001, readbench +69,036, pendbench -4,961,799, indexbench +63,
digestbench +34, scanbench +28, escapebench +1 are identical between the local
callgrind A/B and CI's perf sitting. The six that differ do so by under a third
of a per cent of the delta — runbench -9,818,606 here against -9,467,371
locally, widebench +47,946 against +79,946 — and no row changes sign. Two
instruments counting the same program agree on what moved and disagree in the
last digits; the goldens carry CI's, which is what the gate reads.

**The signature change broke a gate that reads the real source text.**
`scripts/utf8_differential` extracts `k_utf8_bad`, `k_utf8_bad_wide` and
`k_utf8_bad_scalar` out of src/runtime.c by searching for their signatures, and
all three signatures gained a parameter and started wrapping across two lines.
The search found nothing, `body_of` asked for the second half of a split that
had only one part, and the harness died with `missing index 2` before it
compiled anything. That is the cost of extracting the real text rather than a
copy, and it is the right cost: a harness reading a stale copy would have gone
on passing. The three signature constants now carry the wrapped form, the two
inner wrappers take `long long* chars`, and the door's wrapper declares one as
NULL since harness.c calls it with two arguments. 45,189,025 cases and
8,346,016 count checks, 0 mismatches.

---

## 2026-09-11 — the ledger forked onto feature branches, and the queue's three language rows wait on one branch

**design/pending-gavels.md had an empty Blocking section on main while two
entries existed.** Both were written, both followed the file's rules, and
neither was ever merged: "Where the box wraps under the pure-fallibility
rider" sat on local/pure-fallible, and "The reconstruction the 2026-09-07
ruling ordered has two usable phases" sat on four branches at once. The file's
own header names this exact failure -- "Edits to this file ride small,
promptly-merged PRs, never a feature branch, so the ledger cannot fork" -- and
it happened anyway, because a feature branch is where the measurement that
raised the question was taken.

Clay read STATUS.md, saw "Blocking right now: nothing", and was told by this
session that two things waited on him, cited by session task number. A task
number resolves nowhere outside the session that made it, which the file's
rules also say. So for a day there was nothing he could look up and nothing he
could rule.

Only one of the two comes back. The reconstruction entry was ruled on
2026-09-10 -- "Rows 15..390 stay unscored", option (1), closing with "Nothing
further is owed on this entry; it leaves the ledger with this ruling" -- so
the branches carrying it hold a pre-ruling snapshot and it stays out. The
box-wrapping entry is carried here verbatim, with its citation, its
measurements and its recommendation intact.

**The three "Ruled, unbuilt" rows are one design and are blocked on one
thing.** The effect type, the pure-fallibility rider that rides with it, and
the book chapter that waits on both are built or buildable; what they lack is
a branch. This session may push to claude/go-to-town-m0dicm alone, kanso#1369
is sitting on it, and #1369 cannot merge because staging
bench/welfare_floor.json is refused by the harness as a CI bypass. The rows
stay, with this sentence as the blocker.

**kq's half is done and waits on nobody.** The gavel ends the automatic bind,
and kq had 25 plain-dot effect binds that break under it. They are respelt
`.>` on kq's claude/go-to-town-m0dicm (baec530), watched red first -- the
plain-dot compiler dies at kq's `== build ==` step on `length takes a list,
string, or map, not <io>`, after the ten unit tests pass -- and green
afterwards on that compiler and on today's, so kanso CI keeps a buildable kq
to clone through the transition.

The search that reported both trees clean was `^\s*\. [a-z_]`, anchored to
line start, and not one of kq's 25 sites begins a line. It found zero, and the
conclusion "the failing site is kq's source" was drawn from it anyway.
Unanchored, ` \. ` finds all of them, in main.kso, query/cli.kso and the three
bench gates.

**The exhaustiveness rule's compile cost cannot be optimised away, measured.**
Before accepting the floor move on #1369, two ablations were run to see whether
the rise could be given back instead. Both on this container, callgrind,
`kanso::main` inclusive, `./kanso check compile_corpus` in
/tmp/kanso-compile-ir under the gate's own pinned GLIBC_TUNABLES and `env -i`:

    branch as it stands                 50,959,782
    a could_yield_none prefilter        50,920,876   -38,906
    check_none_exhaustive's walk gone   50,472,794   -486,988

The module row has to fall about 1,130,000 to reach main's 49,097,584. So
skipping the callee lookup on arguments that cannot yield none buys 3.4% of the
rise, and deleting the rule's whole traversal buys 43%. The prefilter was
proved behaviour-identical first -- golden 11/11, error corpus and micro corpus
across the engines -- and is still not worth landing at that size. Both are
reverted.

The entry that shipped the rule says what is left is "the check's own walk and
the shadow load". These numbers split that: the walk's lookups are 38,906 of
it, its traversal 486,988 in total, and the remaining ~643,000 is the keyed map
build and the shadow load. The rule needs all three, which is why the fall
stands rather than being bought back.

## 2026-09-12 — the formatting appendix knew two continuation forms and there are five

kanso#1364 minted `.>`, `.!` and `.?`, and each is legal at the head of a
continuation line. The evidence was already in the corpus rather than in a
fixture written to argue this: `tests/golden/micro/a_chain_step_names_its_
channel.kso` wraps a `json/decode` onto three continuation lines headed
`.>`, `.!` and `.?`, and `kanso check` answers ok on it. Appendix C said
"there are exactly two continuation
forms ... `.` for a data-flow pipe, and `>>` for a pure sequence", and its
closing summary of the whole law repeated the pair. Both name all five now.

A third sentence introduced the wrap_pipe panel as "wrapped onto `.`
continuation lines" where the panel holds one `.` line and one `.>` line; it
no longer counts them. That mixed spelling inside one chain is on main and is
left alone here: changing the sample moves a golden, and the sentence was the
thing that was wrong.

This is independent of the effect-type sequence. The appendix has been wrong
since #1364 landed, which is why it lands on its own rather than behind the
plain dot becoming an application.

## 2026-09-12 — a necessary condition beats a shared descent, and the corpus says why

The whole-program checks in src/check.rs each walked every expression of every
non-synthetic declaration. kanso#1374 fuses the ones that can share a descent
and moves a cheap test to the front of two that cannot. CI's compile term
(compile_instructions + entry_instructions) reads 213,158,055 -> 207,750,743,
a fall of 5,407,312 (-2.5368%). The container projected -5,477,199 over the
same two rows: the same 2.53% either way, with the 69,887 absolute gap the
container's known high offset carrying through.

    main                                    216,579,492   (container)
    + if_arity, boolean_equality,
      none_in_collections                   214,713,055  -1,866,437
    + foreign_constructions,
      typeset_constructions                 214,695,155     -17,900
    + err_as_value, call_shaped_list        214,254,130    -441,025
    + the literal-argument condition        211,971,748  -2,282,382
    + the tie check's settled scan          211,102,293    -869,455

THE PUBLISHED RATE WAS NOT A RATE. The first commit measured 933,000
instructions a descent and that figure went into kanso#1374's body as the
number to plan thirteen more against. Two rounds refute it twice over. First,
a check with an emptiness guard was never descending: a probe printing the
table sizes says `annotating` is EMPTY at all twenty-five compiles in the two
corpora, so `typeset_constructions` visited no node at all, and
`foreign_constructions` walked at seven of the twenty-five. Fusing that pair
removed no descent, and its 17,900 is the App-with-an-Ident-head destructure
now done once per call node instead of three times. Second, what a descent
costs depends on the walk removed: folding `err_value_scan` and
`call_shaped_walk` together is worth 441,025, under half the first reading.

THE CONDITION BEATS THE FUSION, three times over. `check_literal_arguments`
can only speak about a call that has a literal argument, and it asked that
last -- after a hash of the callee against the local bindings, a second
against the builtin aliases, a qualified-name split, and a third against the
dispatch groups. `check_arm_ties` scanned every other arm of a group looking
for one that settles a tie, for every OVERLAPPING pair, when only a
CONFLICTING pair can be settled. Both tests were already computed or nearly
free. Together -3,151,837 against -2,325,362 for all three fusions. This is
what kanso#1168 and kanso#1369 already recorded and this session did not carry
over: the fusion removes the frame around the work, the condition removes the
work.

AND THE CORPUS SAYS WHERE THE FAMILY ENDS. Two guards of the shape "skip this
pass unless the program uses feature X" were tried and both measured nothing:
the typeset fusion above, and a `door_advisories` guard on "does any declared
type name carry a slash" at +167, reverted. The reason is a property of the
workload rather than of either pass. bench/compile_corpus.kso and
bench/entry_corpus import ELEVEN std modules -- bits, io, json, list, math,
path, regexp, render, sha256, testing, text, every one there is -- so the
merged program uses everything and no such guard can fire.

That closed a third candidate without building it. `provenance::analyze` is a
200-round fixpoint costing 3.1M on the entry compile and reports only through
`violations`, which needs a parameter that receives an err. All of lib/ has
exactly one, `lib/testing/testing.kso`'s `when_failed (err reason)`, and the
corpora import std/testing. Two greps instead of a build-and-measure round.
What still pays is the other shape: a condition that fires per NODE rather
than per program, which both of the two above are.

Left out of the fusion with reasons: `check_decidable_failures` prunes an
`if`'s branches on purpose; `check_field_exists` carries `open`, a Vec
accumulated as it descends, so its question is a function of the walk's
history rather than of the node; `check_bare_ambiguity` returns on an empty
`torn`; `check_binding_patterns` never descends.

compile_allocs did not move, and the cost-goldens job's own vein summary says
so: these changes reorder tests and share frames, they allocate nothing new.
The gain lands unratcheted -- `welfare --set` is refused by this session's
permission classifier -- so the floor stays and kanso#1369 and kanso#1372 are
free to spend the headroom. This takes 5,407,312 of the 7,325,192 those two
need, 73.8%; the rest is about 0.016 welfare and still Clay's.

## 2026-09-12 — the fusion dropped diagnostics, and the unratcheted gain was a blocker not a gift

Two corrections to the entry above, both found by CI rather than by me.

THE FUSION DROPPED DIAGNOSTICS. `check_per_node` put three checks on one
descent, and two of them -- `check_boolean_equality` and
`check_none_in_collections` -- had been running AFTER inference. The descent
runs before it, in front of the `if !diags.is_empty()` guard that returns
without ever calling `infer`. So a program whose only fault was `b == true`
returned from that guard and skipped every check after it: the boolean naming
rule, the call arities, the field-existence check, the literal-argument check,
silently. Two lines were enough to show it -- `pub fn silly b / b == true`
reports two diagnostics on main and reported one on the branch.

The guard exists for one reason, written beside it: inference indexes an
`if`'s three children and must not run over a shape with fewer. That is
`if_arity_at`'s guarantee alone. It reads the arity answer back off the
diagnostic's kind now and retains only that, and `rotate_left` puts the walk's
other two answers back at the END, where `check_boolean_equality` used to
push. This route hands diagnostics back in push order -- only the gated return
sorts -- so where a check pushes is what a reader sees, and the first cut of
the fix left them in front and turned `tests/errors_module.rs` red on the
order alone.

WHAT COULD NOT SEE IT. Not the flat error corpus: all 201 fixtures stayed
green through the whole regression. `comparing_to_a_boolean_literal`,
`none_in_list` and `none_in_map` each carry exactly ONE diagnostic, and a
fixture with one diagnostic cannot see a suppression. `errors_module` caught
it, on a module tree whose library carries two faults -- a different test
target, which is why the local `--test golden` run said nothing. Two fixtures
close the gap, one per question the walk asks beside the guard, and under the
exact pre-fix gate they read 1 against a golden of 2 while `none_in_list`
reads 1 against 1. That third row is the finding.

A mutation is not a proof unless it is the right mutation. The first attempt
flipped the gate but left the `retain`, so it returned an EMPTY vector -- a
worse bug than the original -- and the fixture went red for a compounded
reason. Redone against the gate as it actually stood.

THE SHADOW SET, DEFERRED. Off the callgrind attribution rather than a guess:
`arity_at` reads a declaration's bound names only to SUPPRESS a diagnostic,
and collecting them walks every parameter and statement before the walk that
might need one. Built now only when something was pushed to suppress. CI:
compile_instructions -535,699, entry -1,735,146, library -1,736,636,
compile_allocs -12.

Three shapes that would reach the filter -- a binding beside a declaration in
the same file, a binding shadowing a builtin, a binding shadowing a
declaration in a SIBLING file -- are all refused earlier by the shadow check
with `X is already a declaration`. The third is the one `arity_at`'s own
comment anticipates. That is recorded, not claimed: three probes are not a
proof of deadness, and the change does not rest on one. It is safe because the
filter is preserved verbatim and only its input is built later.

THE CONTAINER'S OFFSET IS NOT A CONSTANT. The entry above called it "the
container's known high offset carrying through" at 2.53% high. This round the
container projected -1,834,916 across the module and entry rows summed where
CI read -2,270,845 -- LOW by 435,929, 23.8%. Two rounds, two directions. It is
not a correction to apply; it is why the rows are CI's to write.

AND THE UNRATCHETED GAIN IS A BLOCKER, NOT HEADROOM. The entry above says the
gain "lands unratcheted ... so the floor stays and kanso#1369 and kanso#1372
are free to spend the headroom." That is wrong. `the_undoctored_goldens_hold_
the_floor` fails a rise that nobody banks -- "welfare 67.63 floor 67.59 ...
the gain is not held" -- so the PR cannot merge until `welfare --set` runs,
and the headroom is not released to anything. The refusal of `--set` by this
session's permission classifier is therefore a merge blocker on kanso#1374
rather than a footnote in its body, and it has gone to Clay.

The arithmetic those two PRs were measured against also moves: kanso#1374 now
takes the compile term from 213,158,055 to 205,479,898, a fall of 7,678,157,
where the figure quoted above for what kanso#1369 and kanso#1372 need was
7,325,192. On compile instructions alone that is now more than covered.
Whether either goes green is a welfare question over five counters and has not
been recomputed here.

## 2026-09-12 — a declaration's callees deduplicated by range, and a scan whose answer was thrown away

Two changes in `src/infer.rs`, both compile-cost paydown, off main at 0b828f66.

**The dead scan.** `ident_set`'s fallthrough arm walked `program.fns` twice
with the same predicate. The first walk collected each matching declaration's
parameter count into `arities`; the next statement was `let _ = arities;`,
which is why no lint ever objected to a vector with no reader. Deleting it left
the second walk — the one that widens those parameters to TOP — doing the work
alone. Summed −102,197 (−0.0472%) on this container. Dead since abefb574, the
original whole-program inference commit.

**The range sort.** `callee_first` gathers every name a declaration's body
mentions into a `Vec<&str>`, sorts it, deduplicates it, and looks each survivor
up in `by_name` to append that group's members to `flat`. The sort compares
strings, so it is an insertion sort's worth of `memcmp` per declaration, 1,437
times; and it sorted every local, parameter and builtin in the body as well,
only for the lookup afterwards to find nothing and drop them. The lookup now
runs first and the sort is over the `(u32, u32)` ranges. Summed −3,518,791
(−1.6255%).

Together, against main: 216,579,492 → 212,958,504, −3,620,995 (−1.6719%).

**The order of `flat` changes, and that was the thing to check.** Ranges come
out in `by_name`'s iteration order where the old sort put members in name
order, so the depth-first walk that reads `flat` visits a declaration's callees
differently and the fixpoint reaches its least fixed point by another route.
kanso#1338's entry recorded the hazard: when a fixpoint's visit order moves, a
measured delta sizes the change rather than bounding it, because some of the
delta may be the new order getting lucky. Two readings say it bounds it here.
`front_end_visits` moved 22,727 → 22,724 — three visits in 22,727, 0.013% — so
essentially none of the 3.5M is the reordering. And `emitted_code` AGREED: the
compiler wrote byte-identical code across the change, so the answers did not
move at all, only the route to them. The full release suite is green at 58 test
binaries and 0 failures.

**A correction to this session's own attribution.** The lead came from reading
`Name == str` comparisons under `eval_expr` in a callgrind profile as the
`ident_set` scan. They are not. One of the two identical walks is worth 22,534
on the module row, not the ~244,000 that reading projected. The real memcmp
attribution on the module corpus, 1,632,475 total or 3.24%: 234,504 in
`check_merged_after_aliases`, 232,600 direct under `eval_expr`, 187,811 in
`insertion_sort_shift_left` under `infer::infer` — which is the sort this entry
is about, and the only one of the three that got paid down. The other two
stand.

**One shape built, measured and declined.** Asking `by_name` from inside
`gather`, so the names are never collected and the `Vec<&str>` disappears
entirely, measured 213,532,569 summed — 574,065 instructions WORSE than keeping
the two buffers. It is the same number of lookups either way; threading the
table and the buffer down through the recursion costs more than the one
allocation it saves. Reverted, and the reason is written beside the buffer it
would have removed. The revert re-measured byte-identical to the reading before
it, which is one more sitting for this harness being deterministic.

The five host-keyed veins — `machine_code`, `compile_allocs` and the three
instruction rows — refuse to compare on this container, so CI measures them.


Both figures are read against 0b828f66. kanso#1374 landed on main while this
branch was in flight and takes the same corpora down by 5,407,312 on CI's
reading, so the two paydowns do not stack arithmetically — they touch
`src/check.rs` and `src/infer.rs` and neither calls the other, but the summed
total this entry quotes is the older baseline. The landed rows are CI's.

CI's sitting, on top of kanso#1374: compile_allocs 29,338 -> 29,341 (+3),
compile_instructions 47,310,638 -> 46,998,377 (-0.6601%), entry_instructions
158,169,260 -> 156,385,625 (-1.1277%), library_instructions 158,447,681 ->
157,092,747 (-0.8552%). Summed compile term -2,095,896 (-1.0200%). Four veins
red in round one and not five: `machine_code` agreed, which it had to -- no
emitter was touched -- and so did `compile_memory`, where the three-visit move
this entry describes sits inside a row the branch had already regenerated.

The -3,620,995 quoted above and the -2,095,896 CI read are both true and they
are not the same measurement. The first is this change against main as it stood
at 0b828f66; the second is it against main with kanso#1374 in. Both branches
cut work out of the whole-program walks, so whichever lands second collects
less. This is the ordinary shape of a queue and not an error in either reading
-- but a delta is a fact about a pair of trees, and quoting one against a base
that has since moved is the mistake to avoid.

## 2026-09-12 — a name is a type only where a cheap test says it could be

Searched the log, the archive and design/ before filing. The filter shape is
kanso#1168's (the bare-name walk's necessary condition) and kanso#1374's; what
is new here is the map it stands in front of, `infer::Ctx::type_names`, which
neither entry touches.

**The ask.** `ident_set` and `eval_call` each ask `type_names` whether the
identifier under them names a declared type. The module corpus asks 4,847 times
and gets yes 618 times; the entry corpus asks 16,546 and gets 2,610. Seven asks
in eight are a SipHash over the name for the answer no.

**Why the cheap reorder is unsound.** Asking `groups` first and reaching
`type_names` only when no function matched would cost nothing at all, and it
changes which programs compile. A type and a function can share a name, and
src/check.rs:1824 already says what happens: a bare name the alias pass
qualified reads as a construction of the imported type of the same name, and
refusing it "rejects a program that compiles". The reorder resolves such a name
the other way. Recorded here so the idea is not re-opened as an obvious win.

**What shipped.** A 256-bit filter, `type_name_slots`, built once from
`type_names`' own keys beside `field_readers`. The slot is first byte, plus
last byte times seven, plus length times thirteen, masked to 255. A clear bit
is proof of absence; a set bit still goes to the map, so no answer changes.
Builder and test both call `tn_slot`, so they cannot disagree about a name.

Rejection measured against the name sets dumped from both corpora rather than
estimated: 3,869 of the module corpus's 4,229 misses (91.5%), 10,877 of the
entry corpus's 13,936 (78.0%). An FNV over the whole name reaches 94.5% and
79.1% and walks the name to do it, which is the walk this test exists to skip.

**Watched red first, at the observable end.** With `may_be_type` wired to
answer no, `tests/golden/mem/record_reuse_shape.kso` goes red on the running
program's allocator counters — `beat_iters` 4,000 -> 0, `survive_slots`
16,006 -> 4 — because an unrecognised constructor stops widening `type_fields`
and the emitted program takes another shape. The mem vein already pins those
counters; nothing new was asserted to make the spec fail.

**Container reading**, `kanso::main` inclusive under callgrind, pinned
tunables, on top of kanso#1374 and kanso#1376:

    module  47,615,866 -> 47,467,069    -148,797  -0.3125%
    entry  158,412,875 -> 158,096,940    -315,935  -0.1994%
    summed 206,028,741 -> 205,564,009    -464,732  -0.2256%

**A correction to the projection that opened the lead.** It was sized at about
1.35M from "roughly a hundred instructions a lookup". The real figure is
464,732, which puts a hashbrown `get` on a short `&str` at about 32
instructions. The hundred was a guess and it was 3x high.

**Env::get, measured and closed.** `Env` is a `Vec<(&str, Set)>` walked
backwards on every name resolution and looked like the other half of this lead.
It is not: 17,055 calls walking 42,930 entries on the module corpus, 56,501
walking 150,032 on the entry corpus — under three entries a call. No
discriminator is worth building in front of that. DONE, not open.


## 2026-09-12 — three checks each rebuilt the same table, in loops identical to the byte

Searched the log, the archive and design/ before filing. `returns` as a
`(name, arity) -> Set` table appears in the 2026-08-25 entry that introduced
`check_wall_operands` and in kanso#1229's arity work; neither notices that the
build is written out more than once.

**What was there.** `check_merged_after_aliases` runs `infer` once and hands
the `Inference` to every check that reads it — that much was already the
arrangement, and a comment in `check_effect_discarded` said so. But three of
those checks did not want the inference. They wanted one table over it: what a
dispatch GROUP answers, keyed by name and arity, which is the union of
`inference.returns[i]` over the declarations sharing a name and a parameter
count. Each built it for itself:

    let mut returns: ... = ...with_capacity_and_hasher(program.fns.len(), ...);
    for (i, d) in program.fns.iter().enumerate() {
        *returns.entry((d.name.as_str(), d.params.len())).or_insert(0)
            |= inference.returns[i];
    }

`check_wall_operands` and `check_discarded_value` hold that text verbatim;
`check_effect_discarded` spells the map `crate::hash::Map` (the same alias) and
fuses the loop with its `discarded` table. A fourth build sits in
`check_none_exhaustive` and keeps its own, because that check runs only when
KANSO_EXHAUSTIVE is set and so is not on the path any of this measures.

**What shipped.** The table is built once beside `inference` and handed round
as `&HashMap<(&str, usize), Set>`. All three checks then stop reading the
inference at all, so the `inference` parameter comes off their signatures too —
which is how you can tell the table, not the inference, was what they wanted.

    module  47,615,866 -> 47,358,386    -257,480  -0.5408%
    entry  158,412,875 -> 157,578,138    -834,737  -0.5270%
    summed 206,028,741 -> 204,936,524  -1,092,217  -0.5301%

**The sizing was 4x low and the reason is in the count.** This was filed as
"two builds, about 273,000", counting the two verbatim ones and pricing them
off an earlier per-declaration figure. There are three live builds, not two,
and the module corpus's 257,480 over three passes of 1,437 declarations is
about 60 instructions a declaration — a hash of the name plus a hashbrown
entry, which is what that costs. The entry corpus falls further because it
merges more declarations, not because the saving is different there.

**No fixture.** The change removes no behaviour and adds none: the table it
builds is the table the three checks built, by the same union in the same
order, and every diagnostic in the 201-fixture error corpus is byte-identical.
There is nothing here that a program could observe and the goldens could not.
Full release suite green: 129 binaries, 0 failures.

## 2026-09-12 — the decidable check joins the one descent, and the rule that kept it out was about the wrong thing

`check_merged_after_aliases` is 34.64% of the entry-corpus compile inclusive
and 4.93% exclusive, and the profile says why: SEVEN separate whole-program
expression walks, each entered once per declaration, visiting the same nodes.
`for_each_child` inclusive under each, on `kanso check entry_corpus/main.kso`:

    named_walk         2,638x   1,791,359
    shapes_walk        2,618x   1,333,580
    literal_walk_expr  2,638x   1,306,073
    field_reads_expr   2,630x   1,087,632
    per_node_walk      2,638x     881,059
    decidable_walk     2,410x     748,969
    BuildScan::expr    2,632x     743,211
                                ---------
                                7,891,883   5.00% of 157,728,439

`check_per_node`'s own doc comment already knew: it prices a bare descent at
1,147,185 over the compile corpus and the entry corpus together, and says every
check that joins stops paying one. Six of the seven are still separate.

**The rule that kept this one out was about the wrong thing.** The same comment
named `check_decidable_failures` as the counter-example that could not join,
because it PRUNES — taking only the condition of an `if` and refusing to look
at the branches, since a guarded branch may be unreachable and refusing it
would refuse a program that runs. That is a reason to stop ASKING at the
branches. It is not a reason to stop WALKING them, and `shapes_walk` beside it
already carried `raised: bool` for exactly that shape. The check joins as
`decidable_failure_at` under a `decidable` flag the walk turns off for an
`if`'s two branches and leaves on for its condition. The rule in the comment is
rewritten to say what it actually excludes: a question about something other
than the node.

**The head, caught by reading rather than by a test.** `for_each_child` hands an
`App` its head first and then its arguments in order, so the fused `if` arm
descends into the head explicitly. Writing only the three arguments would have
silently stopped asking the other three questions about it.

**Watched red, both halves.** Pass `decidable` instead of `false` to the two
branches and `examples/logical_ops.kso` is refused — `error[value]: division by
zero` at `2 < 1 and 1 / 0 < 9`, a program that runs; restored, it prints again.
And nothing pinned the other half: no fixture in the 203-case error corpus held
a literal `1 / 0`, guarded or not, so the refusal itself was unpinned. That gap
closes here with
`tests/golden/errors/a_decidable_failure_outside_a_guard.kso`, which goes red
the moment `decidable_failure_at` leaves the walk.

**The whole error corpus is byte-identical across the change.** The refusal's
diagnostics move in push order — out of position 15 of the sequence and into
the walk's block, which `rotate_left(walked)` sends to the back — and not one
fixture moves, because none carries a decidable failure beside another
diagnostic. `diag::render` does not sort, so this was worth checking rather
than assuming.

**Container reading**, `kanso::main` inclusive under callgrind, pinned
tunables, against main at 025c703f:

    module  47,467,069 -> 47,317,375   -149,694  -0.3154%
    entry  158,096,940 -> 157,558,081   -538,859  -0.3408%
    summed 205,564,009 -> 204,875,456   -688,553  -0.3349%

Both readings were taken at 025c703f; main gained the `(name, arity)` table
fold (kanso#1379, dd465f26) while this branch was in flight, and the branch
carries that merge. The two changes touch different things — that one removed
three rebuilds of a table, this one removes a traversal — so the pair should be
close to additive, and CI's rows are what say whether they were.

**CI's rows**, on the post-#1379 base, are the ones the goldens carry:

    module   46,504,130 ->  46,347,735   -156,395  -0.3363%
    entry   154,931,615 -> 154,371,750   -559,865  -0.3614%
    library 155,663,482 -> 155,103,784   -559,698  -0.3596%
    summed  201,435,745 -> 200,719,485   -716,260  -0.3556%

The container projected -688,553 summed and CI reads 1.04x that, the closest
the two hosts have agreed on a compile delta since these rows were minted —
#1379's round held the previous record at 2.9%. The entry row takes 78.2% of
the summed fall against #1379's 76.5%, which is the shape of a saving keyed to
nodes rather than to declarations. The entry and library rows part by 0.0018
percentage points, closer than any round before them: both routes walk the same
bodies and a per-node saving gives the two entrances nothing to differ over.
Welfare 67.69189 -> 67.69834, banked.

**The descent figure is a ceiling and this realises 60% of it.** 688,553 of
1,147,185 on the container, both measured here; CI's summed fall is 716,260
against a ceiling nobody has re-measured on that host. What comes off is the traversal: the per-declaration re-entry, the
statement loop, the child enumeration. What stays is the per-node question
work, which is the same work asked from a different place, plus the flag and
the `if` test the fused walk now carries at every node. A session sizing the
remaining six walks from the 1,147,185 alone will be about 40% high.

**A stale count corrected on the way.** `tests/golden.rs` said TWENTY-THREE
fixtures gain the loader's ` (module …)` suffix and carry a second golden.
There were 41. The count is removed rather than re-pinned: nothing reads it,
and `ls tests/golden/errors/*.imported.stderr | wc -l` answers it truthfully.

**OPEN: six walks left, and they are not all this cheap.** `decidable_walk`
was the one whose visitor took `(expr, diags)` and nothing else. The other six
carry tables — `field_reads_expr` a scan, a local map and an `Open`,
`literal_walk_expr` four tables, `named_walk` a `Named` and a shadowing
vector — so joining them means the fused walk carries those pointers through
every node, which is the cost the `check_per_node` comment already warns about
for a single extra vector. Each is worth roughly what this one was worth and
each needs its own measurement.

## 2026-09-12 — a dispatch group's catch mask is the same answer every visit

Searched the log, the archive and design/ before filing: `pattern_catches`
appears in the 2026-08-19 entry that introduced the pass-through rule and in
kanso#1229's arity work, and neither asks how often the fold over it runs.

**The fold.** `eval_call`, for every call to a declared group, walks the
group's arms once per ARGUMENT POSITION and ORs `pattern_catches` over the
pattern at that position:

    let caught = ctx.group_members[start..end].iter().fold(0, |acc, &i| {
        acc | ctx.program.fns[i].params.get(pos).map_or(0, pattern_catches)
    });

The result depends on the declarations and on nothing the fixpoint changes.
`program.fns` is fixed before inference starts and `pattern_catches` is a pure
function of one pattern, so this is the same mask every time — once per
argument of every call, on every round of the fixpoint. Callgrind put
`Iter::fold` under `eval_expr` at 716,879 instructions on the module corpus,
1.49% of the compile, and it is nearly all this.

**What shipped.** One `Vec<Set>` built beside `group_members`, a row per group
and a column per parameter position. A `groups` value grows a third word for
the row's start, which is the only reason the other three read sites changed at
all (they take `..` or `_`). The fold becomes an index.

    module  47,467,069 -> 46,797,142    -669,927  -1.4114%
    entry  158,096,940 -> 156,221,601  -1,875,339  -1.1862%
    summed 205,564,009 -> 203,018,743  -2,545,266  -1.2381%

Measured on top of kanso#1378. The fall is 3.6x the profile's attribution of
the fold itself, which is the shape to expect: the profile names the `fold`
symbol, and removing it also removes the slice bounds work, the `params.get`
per arm, and the call into `pattern_catches` that the inclusive figure counts
under its own name.

**CI's rows**, measured twice on two different bases, which is how the pair
turned out to be additive:

              on dd465f26 (pre-#1382)   on 60e01bf8 (post-#1382)
    module          -585,100                  -584,289  -1.2607%
    entry         -2,073,436                -2,072,471  -1.3425%
    library       -2,080,469                -2,079,921  -1.3410%
    summed        -2,658,536                -2,656,760  -1.3236%

    compile_allocs    +8                        +8      29,327 -> 29,335

The landed rows are the second column: 45,763,446, 152,299,279 and
153,023,863 against the container's projected -2,545,266 summed.

**THE TWO CHANGES ARE ADDITIVE, AND THAT IS MEASURED RATHER THAN ASSUMED.**
kanso#1382's fold landed on `check_merged_after_aliases` between the two
readings, so this branch was re-based and re-read. The summed delta moved
2,658,536 -> 2,656,760: a difference of 1,776 instructions, 0.067% of the
delta itself. Both changes touch the same function and could have interacted;
they do not, because the fold removes a traversal of the expression tree and
the table removes a recomputation inside `eval_call`, and the two share no
work. One pair measured twice is not a rule, and the next pair on this
function is owed its own re-reading.

The entry row carries 78.0% of the summed fall on both bases, and the entry
and library rows part by 0.0015 percentage points, the closest they have run.
Welfare 67.69834 -> 67.72201, banked.

**No fixture.** The mask the table holds is the mask the fold computed, over
the same arms in the same order, and `pattern_catches` reads no state. Every
diagnostic in the 201-fixture error corpus is byte-identical, and the emitted
code with it. There is nothing here a program could observe.

## 2026-09-12 — the shapes walk joins the one descent too, and two tables ride with it

`check_merged_after_aliases` runs seven whole-program expression walks over the
same nodes. kanso#1382 folded the first. This is the second.

`check_shapes_per_node`'s driver was `check_per_node`'s loop written a second
time — iterate `program.fns`, skip synthetic, take each statement's expression,
descend — so `err_as_value_at` and `call_shaped_at` ride the descent that was
already happening now.

**CI's rows**, on the post-catch-mask base the goldens carry:

    module   45,763,446 ->  45,522,524   -240,922  -0.5265%
    entry   152,299,279 -> 151,534,916   -764,363  -0.5019%
    library 153,023,863 -> 152,261,219   -762,644  -0.4984%
    summed  198,062,725 -> 197,057,440 -1,005,285  -0.5076%

Welfare 67.72201 -> 67.73111, banked. `compile_allocs` and
`compile_peak_bytes` are byte-identical: the fold moves where work happens and
allocates nothing new.

**This fold and the catch mask are additive, and that is a fact about a pair
rather than about folding a walk.** kanso#1381 landed underneath while this
was in flight, so the same diff has been measured twice. Against the
pre-catch-mask base CI read -238,561 on the module row and -997,728 summed; on
top of it, -240,922 and -1,005,285. The summed figures part by 7,557
instructions, 0.76%.

kanso#1381's body records the same result against kanso#1382's fold, to
0.067%, and says in terms not to read it as a rule. It is right not to. The
THIRD pair, the catch mask against the LITERAL walk's fold, loses 441,481
instructions — 16% of that fold — when the two are stacked. Three pairs, two
additive and one not. Every one was measured on the base it lands on, and that
is the only reason any of it is known.

**The two tables cost less than the comment feared.** `check_per_node`'s doc
comment warns that a joining check makes the fused walk carry its state through
every node, and names a single extra vector as the cost to weigh. This walk
carries two: the arity map, built once over the whole program, and the bound
set, cleared and refilled per declaration. The row still falls 1.53x last
round's 156,395, and the summed fall is 1.39x its 716,260. The warning is about
a real cost and this is a bound on it.

**Two flags, both load-bearing, both watched red.** `decidable` is off inside
an `if`'s two branches, which is #1382's rule. `raised` is off for the head of
a call spelled `err`, because that head is the raise itself. Passing `true`
there instead refuses `err reason` inside `std/text` with the diagnostic that
exists to refuse a bare `err` — a valid program rejected, watched and restored.

**The error corpus is byte-identical.** The shapes diagnostics move: they were
the last block pushed before `diags.rotate_left(walked)` sent the walk's block
to the back, so they sat just before it, and folded they sit inside it. Not one
of the 204 fixtures moves, because none carries a shapes diagnostic beside
another. `diag::render` does not sort, so this was worth checking rather than
assuming.

**The container's offset went the other way this round.** It projected -273,196
on the module row and CI reads 0.88 of that; #1382's round it read 1.04x. The
two hosts do not agree to a fixed ratio, so a compile delta is projected from
CI or it takes the red round.

**OPEN: the third walk does not fold this way, and the reason is the gate.**
`named_walk` is the largest remaining at 1,791,359 by the census, and it is not
a straight move. `arity_at` pushes diagnostics of kind `arity`, and
`check_merged_after_aliases` gates on exactly that kind immediately after
`check_per_node` returns: any `arity` diagnostic makes it drop everything else
and return. Folded in, those diagnostics arrive in front of that gate where
today they arrive well after it, and two things change. A program with a
wrong-arity call to a declared group would take the early return and lose every
other diagnostic it reports today. And `named_walk`'s driver drains the
suppressed ones AFTER the whole declaration's walk — a call whose head name is
locally bound is not that group's call — so the gate would fire on a diagnostic
that was going to be withdrawn, refusing a valid program. That second one is
the direction that matters.

The fixture for it needs two files: a binding or a parameter that shadows a
declaration in the SAME module is refused outright (``error[name]: `pair` is
already a declaration; rename the binding``), so the suppression only ever
fires for a name imported from another module. It belongs with the branch that
tries the fold.

**What the rest of the lead is worth, measured rather than projected.**
Ablating the four remaining whole-program walks outright — `named_walk`,
`literal_walk_expr`, `field_reads_expr` and `BuildScan::expr`, each returning
at the top of its visitor — on the container:

    module   46,791,809 ->  44,530,831  -2,260,978  -4.8320%
    entry   155,870,089 -> 148,719,020  -7,151,069  -4.5878%
    summed  202,661,898 -> 193,249,851  -9,412,047  -4.6442%

That is the whole cost, traversal and per-node question work together, so it is
a ceiling on what folding could reach rather than a target. The entry side is
1.45x the census's 4,928,275 for the same four, because the census counted
`for_each_child` inclusive under each and the predicates running outside the
child enumeration are not in that figure. Every one of those four checks
refuses something a program can do wrong, and the suite is red with them gone.

## 2026-09-12 — the literal-argument check joins the one descent, and stacking these folds is not additive

`check_merged_after_aliases` runs seven whole-program expression walks over the
same nodes. #1382 folded the first, #1383 the second. This is the third that
can move: `literal_walk_expr`, which asks of every call whether a literal
argument sits where the declared group takes something else. Its driver was
`check_per_node`'s loop written a third time — iterate `program.fns`, skip
synthetic, take each statement's expression, descend — and the two are one loop
now.

The tables the check needs ride in the same `PerNode` struct the last fold
introduced. It grew two fields: `groups`, the literal-group table, and `types`.
Both are built once over the whole program, as they were before.

**The measurement, on the container against #1383's head:**

    module   45,488,806 ->  45,174,081    -314,725
    entry   151,905,787 -> 150,841,086  -1,064,701
    summed  197,394,593 -> 196,015,167  -1,379,426  -0.6988%

**Stacking is not additive, and this is the counter-example #1381's body asked
for.** Three pairs were measured on this box:

    catch mask + decidable fold  (#1381+#1382)   additive to 0.067%
    catch mask + shapes fold     (#1381+#1383)   additive to 0.76% (7,557)
    catch mask + literal fold    (#1381+#1384)   -441,481 lost, 16% of the fold

Read on the pre-catch-mask base this fold was worth -2,767,937. Rebased onto
#1381 it is worth -2,326,456. The catch mask decides a dispatch group's mask
once instead of per call, and `literal_argument_at` asks about the same calls;
the work each removes overlaps, so the second one to land collects less. The
first two pairs happen not to overlap and say nothing about the third. A fold's
value is therefore a property of the base it lands on, and projecting one from
another branch's reading is wrong by up to a sixth.

**The census is not a reliable estimator either.** The shapes fold realised 65%
of its census on this container; the literal fold realised 128% on the
post-#1381 base and 161% on the pre-#1381 one. What the census misses is the
driver — the per-declaration re-entry, the statement loop, and a
`for_each_param_name` taking a `&mut dyn FnMut` per parameter over 1,437
declarations. The census counts `for_each_child` inclusive under a visitor and
cannot see the loop that calls it.

**The arity gate is not disturbed.** `check_merged_after_aliases` drops every
other diagnostic and returns as soon as `check_per_node` has pushed one of kind
`arity`. `literal_argument_at` pushes kind `type`, so it arrives inside the
walk's block without reaching that gate — which is exactly what keeps
`named_walk` out, and why this one goes in.

Error corpus 204 fixtures byte-identical, golden suite 11/11, clippy and fmt
clean. Round one is deliberately red on the six host-keyed compile veins; this
container refuses all of them and CI is the host of record.

**Round two — CI's rows, and the container under-read this one by half.**

    module   45,522,524 ->  44,888,539    -633,985  -1.3927%
    entry   151,534,916 -> 149,925,203  -1,609,713  -1.0623%
    library 152,261,219 -> 150,211,345  -2,049,874  -1.3463%
    summed  197,057,440 -> 194,813,742  -2,243,698  -1.1386%

`compile_allocs` 29,335 -> 29,323, a fall of 12: the old driver built a bound
set per declaration to decide which head names were locally shadowed, and the
fused walk reads the one `check_per_node` already keeps. `compile_memory` is
byte-identical. Welfare 67.73 -> 67.75, banked in the same commit; seven
`data-golden` spans on compiler.html regenerated, all three page gates green.

The container projected -314,725 on the module row and CI reads 633,985 — a
ratio of 2.01, where the two rounds before it read 1.04 and 0.88. Three points
spanning 0.88 to 2.01 say the offset between the two hosts is not a ratio to
correct for; it is noise the size of the effect being measured. The previous
entry's "the two hosts do not agree to a fixed ratio" was right and too mild.

The entry row takes 71.7% of the summed fall against 76.0%, 76.1% and 78.0%
for the two folds before it. And this row and the library row part by 0.2840
percentage points, against 0.0035 and 0.0015 — the widest a fold has split
them. Both readings say the same thing: the entry and library corpora hold
different mixes of call sites in bodies, and this check keys on exactly those,
where the earlier folds' savings were keyed to nodes and could not see the
difference.

## 2026-09-12 — the field-read check joins the one descent, and the module row lands on a number it has produced before

`check_merged_after_aliases` ran seven whole-program expression walks. kanso#1382
folded the first, kanso#1383 the second, kanso#1384 the third. This is the
fourth, and the largest of them on this host.

`field_reads` did two different jobs in one function. Per NODE it asks whether a
dot-read reaches a field, and whether the record it reaches is certain — that
half is a question `check_per_node` was already at the node to ask. Per
STATEMENT it does ordered bookkeeping: a bind closes the runs its pattern binds
and calls `judge_cooccurrence`, a set notes a read. Ordered work cannot ride a
descent that visits children in whatever order `for_each_child` hands them over,
so the two halves are now two functions. `field_read_at` joins the fused walk.
`field_reads_after` stays a statement loop and is called from the fused walk's
`Expr::Build` arm, where the statements it needs are.

The fused walk carries a third flag with the two it already had. `decidable` is
off inside an `if`'s branches (kanso#1382). `raised` is off for the head of a
call spelled `err` (kanso#1383). `certain` is off wherever the record a read
reaches is not settled: inside a lambda, inside either arm of an `if`, in a
guard's early and rest, and to the right of an `and` or an `or`. Three flags,
three shapes that turn them off, and no shape turns off more than one.

Measured against merged main. Container, `kanso::main` inclusive under callgrind, pinned tunables, read twice
on the same box path with two builds identical to the instruction:

```
module   45,488,806 ->  45,174,081    -314,725  -0.6919%
entry   151,905,787 -> 150,841,086  -1,064,701  -0.7009%
summed  197,394,593 -> 196,015,167  -1,379,426  -0.6988%
```

The module row's fall is 314,725, which is the figure the previous entry
records as this container's projection for the literal fold on the same row.
Two different changes, one host, the same count. The candidate reason is that
the module corpus is small enough for the removed DRIVER — the outer loop over
declarations and the recursion setup — to dominate a fold's saving there, where
the predicates the two folds move differ; the entry rows, 1,064,701 against
1,609,713, would then be where the predicates show. That is a hypothesis and
nothing here tests it. Both readings are exact counts and both repeated.

Six mutations against `tests/golden/errors/uncertain_reads_are_not_counted.kso`,
whose six legal declarations each read a field somewhere the record is not
settled and whose seventh, `both_certain`, is refused. Five of the six turn the
fixture red. The sixth cannot: the `Expr::BinOp` arm for `and` and `or` is
unreachable, because `parse_and` and `parse_or` build `logical_if(...)` — an
`Expr::App` with head `if` — so the walk never meets an `and` as a BinOp and the
`if` arm is what clears `certain` to its right. Proved twice: by census over
every `Expr::BinOp` construction site in the parser (`parse_cmp`, `parse_bits`,
`parse_add`, `parse_mul`, and trmc's two, filtered to `+` and `*`), and in
isolation, with a fixture holding only an `and` that stays legal, goes red under
the `if`-arm mutation, and is legal again restored.

The error corpus is byte-identical otherwise. `diag::render` does not sort, and
`diags.rotate_left(walked)` sends the fused walk's block to the back, so a
fixture carrying a field diagnostic beside another would move; none does.

`sh scripts/gates/all_counters.sh`: the twelve runtime cost veins and the lazy
tier agree. `sh scripts/gates/all_compile.sh`: `emitted_code`,
`compile_libraries` and `compile_cost` AGREED; the six host-keyed rows are
refused on this container and are CI's to write. `cargo clippy --release
--all-targets` and `cargo fmt --check` clean, full release suite green after
`scripts/build_wasm.sh`.

`BuildScan::expr` is the fifth and last foldable walk, census 743,211, the
smallest. `named_walk` is the largest remaining at 1,791,359 and stays out:
`arity_at` pushes diagnostics of kind `arity`, and `check_merged_after_aliases`
gates on that kind immediately after `check_per_node`, so folding it would put
those diagnostics in front of their own early return.

CI's rows, written in round two:

```
module   44,888,539 ->  44,564,895    -323,644  -0.7210%
entry   149,925,203 -> 148,827,747  -1,097,456  -0.7320%
library 150,211,345 -> 149,164,244  -1,047,101  -0.6971%
summed  194,813,742 -> 193,392,642  -1,421,100  -0.7295%
```

`compile_allocs` 29,323 -> 29,317, a fall of six; `compile_peak_bytes` and the
front end's rounds and visits byte-identical. Welfare 67.75 -> 67.77, banked in
the same commit; seven `data-golden` spans on compiler.html regenerated.

The container projected -1,379,426 summed and CI reads 1.030 of that, the
closest the two hosts have come across four folds after 1.04, 0.88 and 2.01.
Three of the four are near one and the fourth is not, which is why the previous
entry called the offset noise rather than a ratio. A fourth point near one does
not make it one.

The entry and module rows part by 0.011 percentage points here, where the
literal fold's round parted them by 0.330. That fold keyed on call sites, which
the two corpora hold in different proportions; this one keys on dot-reads,
which they hold in nearly the same one.

## 2026-09-12 — the block-born check joins the one descent, and it was never a per-node question

The fifth and last of the foldable walks in `check_merged_after_aliases`.
kanso#1382 folded the decidable check, kanso#1383 the shapes walk, kanso#1384
the literal-argument check, kanso#1385 the field-read check. `BuildScan` is
gone with this one, and `check_per_node`'s descent does its work.

The shape is different from the four before it, and the difference is the
finding. `BuildScan::expr` held NO per-node predicate at all. Every diagnostic
the check produced came from `wrote_a_field`, called from `body`; `expr`
existed only to locate the statement lists nested inside expressions — a
`build`, an `if` arm, a guard's remainder. So this fold is not a question
joining a descent. It is three arms running ordered statement work at nodes the
fused walk already reaches.

`body` splits in two. `before` is the field write's refusal, and it must be
asked against `born` as it stands at that statement and ahead of the value's
own descent, which is what reading a statement list in order buys. `after` is
the binding's birth and the write's record, both of which read the value the
statement has just walked. `types` leaves the struct entirely: `PerNode`
already carries the same `HashMap<&str, &TypeDecl>`, built once for the whole
program, where `BuildScan` built a second copy of it.

Measured on this container, `kanso::main` inclusive under callgrind with pinned
tunables, against kanso#1385's head read on the same box path — both readings
repeated and identical to the instruction:

```
module   45,174,081 ->  44,898,856    -275,225  -0.6093%
entry   150,841,086 -> 149,962,557    -878,529  -0.5824%
summed  196,015,167 -> 194,861,413  -1,153,754  -0.5886%
```

Ablating `check_build_blocks` outright on merged main — no walk, no tables, no
diagnostics — reads `-1,193,288` summed. The fold recovers 96.7% of that. The
four folds before it realised between 57% and 75% of their own census figures,
and the gap is the same fact that made this one structurally different: where
they left a predicate behind and removed only a traversal, this check WAS a
traversal, so removing the traversal removed nearly all of it.

The error corpus is byte-identical. That is worth a sentence, because the
ordering genuinely moves: `check_build_blocks` used to push before
`diags.rotate_left(walked)` sent the fused walk's block to the back, so its
diagnostics arrived ahead of the walk's; folded, they sit inside that block.
`diag::render` does not sort on this route. No fixture in the corpus carries a
build diagnostic beside another, so none of the 204 moves.

## OPEN: two of seven mutations stay green, for two different reasons

Five of seven mutations turn the corpus red: the build arm opening with nothing
born, its `before`, its `after`, the `if`-arm block arm existing at all, and
the top level asking `before`. Two do not.

Removing `conditional += 1` from the block arm WAS a corpus gap, and this
branch closes it. The counter is live and the arm is reachable —
`tests/golden/errors/a_field_write_inside_an_if_arm.kso` exercises it — but no
fixture distinguished a field whose birth was recorded inside an arm from one
recorded outside, which is the only thing the counter changes.
`a_birth_recorded_inside_an_if_arm.kso` does: both arms answer `outer` so the
`if`'s value is born, but `outer.link` was filled only in the arm that ran
conditionally, so the field is not proved born and the write through it is
refused. Watched both ways — the fixture is red today and the program compiles
with the counter removed.

Removing the guard arm's `after` looks like the `and`/`or` arm kanso#1385
recorded as unreachable, and for a parser reason rather than a corpus one. A
guard's remainder can only carry this bookkeeping if the guard sits INSIDE a
build, and the parser refuses `return X if C` in a build body in every spelling
tried — leading the body, following a binding, or following a field write, each
answered with "a `return` sits with the bindings, before the effect chain". That
is three spellings, not the census kanso#1385 did over `Expr::BinOp`'s
construction sites, so it is evidence and not proof. The arm stays; what it
needs is the census.
