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
## 2026-10-10 — a node an inner pop repairs stays on the outer loop's list

The lox port's `tests/functions.lox` crashed natively once kanso#1824 let its
evaluator loops carry. The cause is older than that branch. A record built
before the evaluator's loop held a string in the outer loop's tenure block.
An inner loop returned a value that reached the record, and its pop copied
the result out of its carry pair. The pop's walk counts nothing outside the
arena as a survivor, so it copied the string to the arena's frontier and
repaired the record in place to point at the copy. The frontier was in the
evaluator's lap. At the evaluator's next stage the walk reached the record
through an older node whose own fields all survived, stopped there, and
rewound the lap with the copy in it. The record then pointed at freed memory.

Main's compiler crashes on the reduction too, so the bug did not need
kanso#1824. `tests/golden/mem/a_node_repaired_by_an_inner_pop_outlives_the_outer_stage`
is three nested loops and a box built before the middle one. Built against
main's runtime it segfaults, and its stdout golden fails.

The pop now keeps the nodes it repaired that lie below the enclosing mark, on
a list beside `k_repaired`. Each stage walks the entries below its own mark as
extra roots before it rewinds, sized with the carried slots, so a node that
points above the mark is repaired into the carry and settled back under the
raised mark like any other. That makes the node safe for the depth that
staged and not for the one outside it, so an entry below the outer mark stays
on the list, and so does a node the stage itself repaired below the outer
mark. A rewind frees what lies above its mark, and the list must not keep an
address that has been handed out again. While the list holds anything, a bit
in `k_buf_dirty` sends every rewind to the slow path, and the slow path drops
the entries above its mark, and a rewind to the outermost mark clears the list,
since no stage outside it will revisit anything. The fast path reads nothing
new.

I first tried narrowing the pop's walk instead: an enclosing depth's tenure
outlives the pop, so the pop could leave a pointer into it alone. That fixed
`functions.lox` and broke `binary_trees.lox`. The lox interpreter writes into
tenured environment maps in place, so an enclosing tenure node can hold a
pointer into the inner carry pair, and the copy-out has to see it.

With the fix, main's compiler passes all 28 lox fixtures on the interpreter,
a dev build and a release build. On a tree carrying kanso#1827 and kanso#1824
as well, the same 28 pass under the lap rule and with every edge staging. The
twelve cost veins and the mem tier are unchanged apart from the new fixture's
own file: no program in the corpus repairs a node below an enclosing mark.

The list costs something when it is empty. `k_beat_iter_carry` now asks its
depth for an outer mark and walks the nodes it repaired against it, and the
pop does the same, at about fourteen instructions a call whether anything is
pushed or not. deepbench stages 52,310 times and pops 52,003 times, so it pays
the most, 0.40%. The runtime's text is 2,208 bytes larger in every binary,
which the `text` total and both codegen rows carry. CI's rows, old to new:

    work_deepbench               365,792,692 -> 367,269,411
    work_basket                   32,375,114 ->  32,505,156
    work_runbench              1,131,250,979 -> 1,131,470,937
    work_pendbench               190,390,436 -> 190,396,306
    work_livebench             1,472,859,260 -> 1,472,861,300
    work_jsonbench               860,548,891 -> 860,549,545
    work_encodebench           2,383,861,305 -> 2,383,861,896
    work_digestbench               5,560,569 ->   5,561,143
    work_indexbench                2,456,650 ->   2,457,185
    work_scanbench                   268,468 ->     268,793
    work_widebench                28,094,541 ->  28,094,810
    work_readbench                 4,577,050 ->   4,577,104
    work_oneshot                  13,176,487 ->  13,176,528
    work_escapebench              39,045,280 ->  39,045,300
    text                           4,612,254 ->   4,643,166
    codegen_instructions_dev     118,896,762 -> 118,918,430
    codegen_instructions_release 453,707,138 -> 453,994,118
    browser_run_instructions      33,493,063 ->  33,507,353

Welfare fell by less than a hundredth, and the floor moves down with it under
the rule for a change that makes the language do what it says: the program
crashed.

**Main's repair list, merged into the lap carry.** The two branches touch
different parts of the carry, and the code merged without a conflict. One golden
moved. `a_node_repaired_by_an_inner_pop_outlives_the_outer_stage.mem` was
written before the lap carry, when every edge staged, and under the lap carry
the same program stages less: `evac_allocs` 237 -> 121, `evac_bytes` 8,288 ->
4,128, `survive_slots` 82 -> 40, `allocs` 302 -> 246, `alloc_bytes` 7,088,284
-> 7,086,268, and the new `chain_finds` row reads 0. Its output is unchanged.
The fixture still does its job on the merged tree: with
`k_pend_keep_repaired` returning at once, the program segfaults. The goldens
both branches moved were combined as the sum of the two deltas, and CI's rows
for the merged head follow.

CI's rows for the merged head. The projection from the two branches' deltas was
close but not exact: each benchmark's text is 672 bytes larger than the sum, basket
runs 129,985 instructions fewer, deepbench 104,915 more and runbench 14,885 more.
Against main, every row that worsened, with the value it landed on:

    run_ten_frees                                6 ->               1
    run_ten_handups                              2 ->               1
    compile_instructions                25,832,340 ->      25,858,191
    entry_instructions                  85,435,525 ->      85,514,540
    library_instructions                85,988,213 ->      86,067,450
    codegen_instructions_dev           118,918,430 ->     118,945,861
    codegen_instructions_release       453,994,118 ->     454,057,202
    browser_compile_instructions       388,181,803 ->     390,304,777
    browser_compile_peak_bytes             673,727 ->         675,725
    browser_run_instructions            33,507,353 ->      34,098,461
    work_digestbench                     5,561,143 ->       5,562,545
    work_encodebench                 2,383,861,896 ->   2,383,883,896
    work_escapebench                    39,045,300 ->      39,045,867
    work_indexbench                      2,457,185 ->       2,459,004
    work_jsonbench                     860,549,545 ->     860,559,280
    work_livebench                   1,472,861,300 ->   1,472,863,852
    work_oneshot                        13,176,528 ->      13,207,890
    work_readbench                       4,577,104 ->       4,577,539
    work_scanbench                         268,793 ->         270,025
    text                                 4,643,166 ->       4,763,230

The lap carry stages once per 256KB of drift where it staged at every edge, which
is what the work rows above pay for; the runtime grows by the chain index and the
lap bookkeeping, which is the text row. Against that, runbench falls to
1,129,571,152, basket to 30,405,533 and deepbench to 366,705,890, the run
program evacuates 6,441,632 bytes where it evacuated 9,789,344, and the browser
run peaks at 1,179,648 bytes. `run_ten_frees` and `run_ten_handups` count work on
tenured blocks, and they fall because the program now tenures one block where it
tenured six. Welfare rises from 90.2505 to 90.2618, and the floor is raised to
hold it.

## 2026-10-10 — a `tie` node holds its references in a list

Built the gavel "a field may hold a list of `tie` references". The maker check
in `src/check.rs` walked a constructor's arguments with a flag saying each one
was a field, and handed every other expression's children down with the flag
cleared. A list literal was one of those other expressions, so `ref` called
inside `town "ash" [(ref "birch") (ref "cedar")]` was refused as a read. A list
literal or map literal that is itself a field now passes the flag to its
elements, and for a map to its values only, since a key is read to place its
pair. Anything else that holds the list is refused as before: binding it to a
name first, measuring it, indexing it or handing it to a function.

Neither engine needed a change. The interpreter's `resolve_tied` and native's
`k_tie_resolve` already walk lists and maps when they swap each cell for its
node, because a record field's value was always allowed to be any value once
the tie returned.

The diagnostic now says the reference goes straight into a field "alone or in a
list or map written there", and the four tie goldens that quote it moved with
it. `tests/golden/micro/a_graph_holds_its_roads_in_a_list` keeps each town's
roads as a list and a map of named roads beside it, and prints the same text on
the interpreter, a dev build and a release build; the compiler on main refuses
it at `town "ash" [(ref "birch") ...]`.
`tests/golden/errors/a_tie_maker_measures_a_list_of_references` binds the list
to a name and measures it, and is refused at both references.

`a_graph_tied_from_a_map_of_edges` keeps its chain of `road` records. Its edges
come from a map, so its list of references would be built by a call, `list/map
names ref` or a fold, and a reference handed to a function from std is out of
the check's sight. The gavel's text admits a literal, and the ruling's own
Owes asked for this fixture to drop the chain, which it cannot do under that
text. That question has gone to the ledger as "May a list built from data hold
`tie` references?". `docs/compiler.html` §193 says which form each case takes.

The book teaches cyclic structures with `build` and does not mention `tie`, so
the place `tie` is taught is its own comment in `lib/list/list.kso`. That
comment now names the list form and shows `node id [(ref "b") (ref "c")]`.

CI's rows for this branch, with main's repair list merged in. The check walks
two more expression kinds in a constructor's arguments, and `lib/list/list.kso`
carries two more comment lines, which every compile of the library lexes:

    compile_instructions             25,832,340 ->  25,832,534
    entry_instructions               85,435,525 ->  85,436,634
    library_instructions             85,988,213 ->  85,988,060
    browser_compile_instructions    388,181,803 -> 388,192,201

The ruling is a part of the language, so the floor follows whatever these cost.

## 2026-10-10 — a constructor handed over as a function builds

The lox port found that a record constructor passed where a function is wanted
checks and runs on the interpreter and stops the native build with "`pair` as
a bare value is not yet supported" (lox F20). The port wrapped each one in a
lambda, `(l o r -> logical l o r)`, which is what native now does by itself.
When codegen lowers a name that is not a function group, a builtin or one of
the words with a value of its own, and the name is a record type with one to
four fields, it lowers the lambda that calls the constructor. Four is the
native lambda's limit, so a record of five or more fields handed over is still
refused at build with the same message.

`tests/golden/micro/a_constructor_handed_over_as_a_function` hands a
one-field constructor to `list/map`, a two-field one to a function that calls
it and a three-field one to another, and prints the same line on the
interpreter, a dev build and a release build. The compiler on main refuses to
build it. The ratchet row "a constructor refused as a value" puts the refusal
back and the micro corpus goes red.

`tests/a_type_name_as_a_bare_value.rs` had pinned the refusal for a
two-field record printed bare. It now pins that both engines print `<fn>`.

CI's rows for this branch, with main's repair list merged in. Two rows moved,
both by the size of the new lowering in the emitter:

    emit_instructions                7,062,299 ->   7,062,491
    browser_compile_instructions   388,181,803 -> 388,266,184

Welfare falls from 90.2505 to 90.2504, and the floor follows it down under the
rule for a change that makes the language do what it says: a program that
checks and runs on the interpreter now builds natively.

The tie-list ruling landed first. Carried over it, the browser compile row
lands at 388,276,582, the constructor's 84,381 on top of main's 388,192,201,
and the floor at 90.25039901483147.

## 2026-10-10 — two values a carry handed back while they were still held

The lap carry (kanso#1827) let the toml port parse a comment holding a
control character and then lose the error it meant to report. The program
died with an unhandled err whose record had no type, born in `toml/fail`.
The interpreter printed `toml/toml_error 10 1 "control character 7 in
comment" "a = 1 # xy"`.

The bug is older than the lap carry. `end_at` reads the byte after a `#`,
calls `skip_comment`, and hands what comes back to `newline_at` in tail
position. The two form a loop that rewinds the arena on the edge back into
`newline_at`. The cursor crosses that rewind uncopied, because inference
counts a slot that holds an int or an err as one that may cross a rewind
the way a number does. On the lap that meets the control character,
`skip_comment` answers an err wrapping a `parse_failure` record, and both
were built in the lap's own storage. The rewind handed that storage back,
`newline_at` answered the err by adding a hop to it, and the hop was
allocated over the record. A hardware watchpoint on the record's type word
caught `k_err_hop` writing it. Main has the same hole: the fixture below
dies on main's compiler in the same way. toml reached it only once #1827
admitted its loop, which main refused because the loop's carried byte may
be bytes.

The carry path already had the answer. `k_beat_iter_carry` returns without
rewinding when a carried slot holds an err, since the callee only passes
the failure on. A plain rewind asked nothing of the arguments it lets
cross. Codegen now tests each argument whose set includes a failure, and
skips the rewind when one of them is an err. That applies on a carrying
edge too, for the arguments the carry does not take. The test is a tag
compare and a branch at each rewinding edge where such an argument exists.

`tests/golden/micro/an_err_built_in_a_lap_outlives_its_rewind.kso` is the
toml loop cut down to three functions. Main's compiler prints `unhandled err
... record born in f/skip`. With the test in place it prints `failed at 3:
stopped` on both engines and in a release build.

The toml port's `many-tables` document showed the second fault once the
first was fixed: the last header's path came back as the path of a key read
two lines later. A carry stage copies its slots in order, and the first time
the walk meets a node that has already been through one copy it switches to
tenured storage for the rest of the stage. A node copied into the carry pair
before that switch could then be shared by a node tenured after it, through
the map that makes a node shared twice copy once. The tenured parent
survived the next stage, the stage's walk stopped there because everything
the parent held directly survived too, and the stage after that wrote over
the pair the shared node lay in. Main has this one as well. When the map
answers with a copy in the pair and the walk is tenuring, the node is now
copied again into tenure and the map points at that copy from then on.

`tests/golden/micro/a_tenured_node_shares_nothing_with_the_carry.kso` builds
the shape directly: a loop carrying a fresh list, last lap's tree, and a new
tree node wrapping the list, with enough garbage per lap that every lap
stages. Main's compiler prints `[200]` in three places where `[400]`, `[600]`
and `[800]` belong.

Neither fix moves an allocation counter. The test before a rewind adds a
compare and a branch at each rewinding edge with an argument that may be an
err, which the emitted code shows: runbench gains 22 branches and 125 lines,
the decoder 4 branches and 15 lines, and seven other benchmarks between 2 and
13 branches.

CI's rows, measured with main's constructor change merged in. The code the
compiler writes grows with the new tests: emitted_branches lands on 1,105 and
emitted_lines on 10,070 for the decoder, emitted_other_branches on 16,143 and
emitted_other_lines on 144,731 across the other benchmarks, and the machine
code total, text, on 4,644,478 bytes, from 16 bytes more on scanbench to 368
on runbench. The work rows move with the change, and which branch costs what
has not been isolated: work_encodebench lands on 2,393,926,456, up 10,064,560
or 0.42%; work_runbench on 1,131,539,079, up 68,142; work_digestbench on
5,597,963, up 36,820; work_jsonbench on 860,549,995 and work_oneshot on
13,176,892, up 450 and 364; and livebench falls 259. Building costs a little
more: codegen_instructions_dev lands on 118,918,569, up 139, and release
codegen falls 8 to 453,994,110. In the browser, browser_compile_instructions
lands on 388,494,007, up 217,425, and browser_run_instructions on 33,508,472,
up 1,119. Both changes fix what programs print, so the floor follows them,
from 90.25040 to 90.24995.

## 2026-10-10 — gavel: `build` retires, and `tie` is the one way to write a cycle

Clay, on the ledger entry "Does `build` still earn its place beside `tie`?":
"retire build". The entry recommended it once `tie` was built and its
compile-time check on literal data was pinned, and both are on main: `list/tie`
runs on every engine (log, 2026-10-05), a broken link in literal data is
refused at check, and ten thousand tied rings dropped in a loop hold one arena
block.

**What leaves the language.** The `build` block, the hole `_` that only a
`build` block could fill, the field write that filled it, and the
`error[build]` family that policed all three. A cycle is written with
`list/tie` or `list/tie!`, whether its nodes are two names in the source or a
graph read from a file. The 2026-10-01 gavel's reason for there being no loop
construct is unaffected, and so is the top-level constant knot
(`ring = node 1 ring`), which is a separate construct and is not part of this
ruling.

**Why.** `tie` does everything `build` did. With literal keys and links the
compiler sees the whole graph and refuses a broken link before the program
runs, which was the one thing `build` caught earlier, and a tied call's nodes
are one birth cohort, so the birthday theorem and the cohort's memory
guarantee hold as they did for a block. Two spellings of one thing is what
kanso declines to offer.

Owes: the parser, the checker and all three engines drop `build`, `_` and the
field write; the `error[build]` goldens and `tests/golden/mem/build_cycle.kso`
move to `tie` or go; every `build` in the corpora, the samples, the examples
and lib is rewritten as a `tie`; ch03's "two records that point at each other"
teaches the cycle with `tie`; appendix A loses `error[build]`; the compiler
page and the playground stop offering it. A program that still writes `build`
gets one diagnostic naming `list/tie` as the replacement.

## 2026-10-10 — the four port fixes' floor question closes on the corpus-first gavel

The ledger's Blocking entry "May four port fixes lower the welfare floor?",
filed today, asked whether kanso#1820, kanso#1822, kanso#1824 and kanso#1825
may each lower the floor by a few thousandths. Each fixes a native cliff a port
journal found (sat's map put loop, lox's loop entered from a tail cycle,
`list/tally` over many distinct strings, mustache's fold onto its accumulator),
and each pays only in compile rows because no benchmark contains its shape.

That is the case the archive's 2026-09-05 "gavel: corpus first — a blind
corpus is repaired, never excused" ruled. Clay declined, then, the
recommendation to move the floor with "the corpus is blind" as the reason:
**the remedy is to add the benchmark the objective could not see, measure its
baseline on the pre-fix code, land the fix on top, and let it score**, so the
floor rises rather than falls. The entry's search looked for a ruling on fixes
that make native finish what the interpreter finishes and missed this one,
because the ruling is about what the corpus can see. The entry leaves the
ledger in this commit on that citation.

So no floor moves for these four. Each PR promotes its fix's shape into the
benchmark corpus as a run-speed or run-memory phase under the granted-baseline
machinery, with its baseline measured on main without the fix, and banks the
rise with `--set`. The entry's option 1 also asked to widen the 2026-09-13
exception to every change that lets native finish a program the interpreter
finishes; under corpus-first such a change scores as a gain once the corpus
holds its shape, so the widening is not needed, and it is not made here.

## 2026-10-10 — four port shapes join the run program, measured before their fixes

The corpus-first gavel, applied today to kanso#1820, kanso#1822, kanso#1824 and
kanso#1825, says none of the four lowers the floor: the shape each fixes goes
into the benchmark corpus first, measured on the compiler without the fix, and
the fix then scores against it. This entry is the first half. The run program
gains four phases, each the PR's own reduced fixture with its count lifted into
a parameter, and nothing in the compiler changes. The columns are
instructions on main with each phase built alone, less the empty program's
6,662,610, and the shares are of the whole program's 1,245,042,917.

    scatter  2,340 keys counted in scrambled order   50,901,814  4.09%
    bump     350 updates through a named read        53,247,360  4.28%
    machine  20,000 passes of a loop in a cycle       8,825,906  0.71%
    fold     800 pieces folded onto one string          858,398  0.07%

`scatter` counts keys that arrive out of order (kanso#1820), `bump` updates a
map through a read named on the line before the write (kanso#1822), `machine`
runs a loop entered from inside a tail cycle (kanso#1824), and `fold` folds
pieces onto a string (kanso#1825). The counts follow the rule the header
already states for the stress shapes: `scatter` and `bump` are quadratic on
main, and each count lands its phase near five per cent of the program.
`machine` and `fold` cost little in instructions and a great deal in memory,
so their counts are sized to add about one decoded document to the peak.

The header keeps its changeover table as it was. That table is the mix
bench/runbench_phases.txt replays when history from before 2026-09-06 is
reconstructed, and a phase with no counter in that history has no place in it.
The four new phases have a table of their own below it, and
`tests/the_phase_map_matches_the_run_programs_header.rs` now stops reading at
the changeover table's total.

Every row the run program carries moves, because it now does more work. In the
run vein, run_allocs lands on 1,773,132, run_alloc_bytes on 326,862,802,
run_arena_blocks on 8, run_perm_allocs on 97, run_beat_iters on 27,564,
run_put_mut_grow on 15, run_str_scans on 229 and run_str_scan_bytes on
17,932,036. The shape counters follow the new values the phases build:
run_sh_str lands on 10,188,896, run_sh_rec on 1,500,528, run_sh_buf on
119,041,376 and run_sh_map on 11,200. `scatter` reads its map between puts, so
the run now builds and frees sorted views: run_view_allocs and run_view_frees
are both 360. The run program's machine code grows to 602,217 bytes, which
moves the text total to 4,651,966, and its emitted code to 684 defines, 6,519
calls, 5,938 branches and 50,994 lines, which moves emitted_other_defines to
2,002, emitted_other_calls to 17,393, emitted_other_branches to 16,393 and
emitted_other_lines to 146,696.

A counter that measures something new is re-based so that landing it costs the
floor nothing, as the codegen rows were on 2026-10-07 when their gates began
counting the clang the benchmarks use. `run_instructions` and `run_peak_bytes`
keep their ratio to their baselines: each baseline is multiplied by the reading
with the phases over the reading without them, both on main's compiler.
work_runbench lands on 1,245,043,324 on CI, against main's 1,131,539,079,
so run_instructions' baseline goes from 3,043,743,748 to 3,349,060,500. The run peak, which the objective reads as the arena, held and
permanent peaks summed, goes from 3,899,936 bytes to 12,167,368:
run_arena_peak_bytes lands on 8,912,912 and run_held_peak_bytes on 3,222,256,
and run_peak_bytes' baseline goes from 156,818,380 to 489,255,962. Both
baselines are rounded up. The score is unchanged, and the four fixes now
have something to score against.

## 2026-10-10 — `build` retires, and `tie` is the one way to write a cycle

The gavel of the same day retired the `build` block and everything that existed
to serve it: the hole `_` in a construction's argument, the field write
`a.f = v`, and the checker's proof that a write reaches only a record born in
the block. This entry is the build.

The parser no longer produces `Expr::Build`, `Expr::Hole` or `Stmt::Set`, and
the three variants are gone from the AST, so every pass that matched them lost
an arm. The checker loses the block-born walk and its cohort table. The
interpreter loses its field write and the native backend loses the
`k_set_field` call it emitted for one, along with the runtime function behind
it. A program written for the old form meets one syntax diagnostic for each
spelling, and each one names `list/tie`: a `build` header, a bare `_` where a
value goes, and `a.f = v`. The wildcard pattern and a lambda's `_` parameter
are read before the expression parser is asked, so they keep their meaning.

The error corpus loses the twenty-six fixtures that pinned the block's own
refusals and gains three for the retired spellings,
`a_build_block_is_retired`, `a_hole_is_retired` and
`a_field_write_is_retired`. Each was seen red with the diagnostic's wording
changed. `uncertain_reads_are_not_counted` loses its block-bound case and
keeps its other six. The micro corpus loses four fixtures that existed only for
the block. `a_description_rides_in_a_field` now builds its record with the
description in place, and the re-export spec for a door spelling loses its
`build` position and keeps the other four. The ratchet loses the seven rows
that disarmed a hole refusal, with their mutation scripts, and the construct
census in `tests/golden/shapes.txt` loses `Build`, `Hole` and `s:Set`.

The examples, the ch03 sample and the knot-equality golden were rewritten with
`tie` earlier on this branch. The mem fixture that pinned a cycle's cost moves
from `build_cycle` to `tie_cycle`, and `tie` costs more than the block did for
the same two cycles: 38 allocations against 18, 2,096 bytes against 784,
evac_allocs 15 against 13 and evac_bytes 560 against 496. sh_buf goes from 144
to 928 and sh_map from 0 to 192, which are the key list and the map `tie`
answers; sh_str falls from 192 to 176.

ch03 teaches `tie` in the section on records that point at each other, and
shows a field holding a list of references, which the same day's gavel "a field
may hold a list of `tie` references" owes the book. Appendix A replaces its `error[build]` section with `error[tie]`, the check
that refuses a literal link to a key the list does not hold. The playground's
two knot samples are rewritten with `tie`, and the compiler page's account of
the memory model now describes a cycle as a tie's birth cohort.
design/build-blocks.md is deleted; the log keeps its history.

Moving the cycle fixture onto `tie` found a disagreement between engines.
The interpreter builds a tie's nodes with thunk cells and settles each record's
fields once the knot is closed, but `resolve_tied` settled only a field that
was itself a cell. A cell inside a list or a map held by a field stayed a cell,
and every later read forced it again: `tie_cycle` counted 5 `thunk_forces` on
the interpreter and 0 natively. `settled_link` now replaces a forced cell
inside a list or map field with the node it holds, and the two engines agree on
the fixture. The spec is `mem_corpus_interp_matches_the_semantic_counters` in
`tests/oracle.rs`, which runs every mem fixture on the interpreter and compares
its semantic counters with the golden's. With the list and map arms of
`settled_link` removed it goes red on `tie_cycle`, reading 5 forces against 0.
The mem golden alone stays green, because it is read off the native binary.

`tests/compile_cost.rs` timed a `build_block` sample, which no longer parses,
so the sample and its row in `bench/compile_golden.txt` are gone. A `tie`
sample cannot take its place there, because the harness compiles a file with no
imports and `tie` lives in `list`. Removing the passes that served the block
moves the compile, entry, library, emit and interpreter rows and the size of
every binary. On CI every one of them falls. compile_instructions lands on
25,712,749, entry_instructions on 85,036,744, library_instructions on
85,590,951, emit_instructions on 7,038,004 and interp_instructions on
488,375,858. codegen_instructions_dev lands on 118,906,508 and
codegen_instructions_release on 453,887,359. browser_compile_instructions
lands on 388,285,810. Every binary is 448 bytes smaller, and the text total
lands on 4,645,694. work_runbench lands on 1,245,043,177, as projected, with
work_encodebench on 2,393,925,910, work_livebench on 1,472,860,747 and
work_oneshot on 13,176,864. No row rises, so the floor does not have to move to
admit the retirement, and the score rises from 90.24995 to 90.2512, which is
banked.

Three diagnostics lost their golden with the block. The unused-expression rule
lives in two walks, the function body's and the block's, and
a_build_body_line_goes_nowhere was the only program that reached either: in a
body, consecutive expression lines fold into one effect group, and a binding
after an expression line is refused first by "bindings precede the effects in a
body". The indented-block rule never had a reachable program. It was counted as
pinned by tests/golden/play/decls_after_statements.stderr, a file no test reads
and whose fixture now runs and prints `first`. Dropping `build` from the
message's wording took the stale copy out of agreement. All three are listed in
tests/golden/unpinned_diagnostics.txt with the programs that were tried.

## 2026-10-10 — gavel: GitHub issues are the tracker, and hygiene is continuous

Clay, on the proposal to track a cleanup list as GitHub issues: "why on Earth
not just use githubs issues feature? seems it would be a much more methodical
way to track and prioritize work." And on keeping the repository clean: "the
instructions should explicitly maintain ongoing refactoring so that things stay
hygienic over time and I don't have to explicitly remind you."

**The tracker.** Open work and open decisions are GitHub issues in
kanso-lang/kanso, labelled by kind (`needs-clay`, `ruled`, `cleanup`, `perf`),
lane (`lane:chat`, `lane:cloud`) and priority (`p1` to `p3`). A pull request
that finishes one closes it with `Fixes #N`. The log stays the record of what
was ruled and measured. STATUS.md's "Ruled, unbuilt" section and
design/pending-gavels.md retire; #1852 carries the steps, including the two
specs that keep their counts in step. The case for the change was made the same
day the two newest "Ruled, unbuilt" rows were found already built and never
taken off, and a ruling of 2026-10-07 was found never to have had a row.

Opened the same day: #1842, the one open decision (whether `list/map xs ref`
may build a node's list of references), and #1843, the three-parts-per-billion
row, the last standing "Ruled, unbuilt" entry; and nine `cleanup` issues from a
hygiene review, #1844 to #1852.

**Hygiene.** CLAUDE.md gains a section, "Hygiene is continuous": each pull
request leaves the files it touches cleaner; a fact has one home; cloud lands a
`cleanup` pull request for every five others; cloud sweeps the repository
weekly and files what it finds; a refactor is a pure move; and instructions
state rules, with their history in the log.

## 2026-10-10 — the log goes back under its cap a second time

design/compiler-log.md held 526 entries and 25,301 lines against a cap of
forty. The archive was last fed on 2026-09-18, and in the three weeks since
every change appended to the live file and nothing moved the older end.

This closes #1850. The 487 oldest entries move to design/log/compiler-log-archive.md
unedited, appended after its last entry. The live file keeps the newest 39 plus
this one. Nothing is deleted. Before the move there were 1,456 archived and 526
live entries; after it there are 1,943 archived and 39 live, so 1,982 entries
either way. The sorted heading lists of the two files match before and after
the move, and so do their non-blank lines, so the move changed no content.

The readers that broke last time were checked first. `page_drift` and the
trend gate count only added lines of the live log (`+## ` and `+`), so moving
lines out costs neither of them anything.
`a_question_sent_to_clay_has_a_ledger_entry` and its ratchet mutation read
both files since 2026-09-18, and
`a_log_heading_is_one_line` always has.

- **DONE** live log at forty entries.
- **OPEN** the cap has no gate. It was restored by hand on 2026-09-17 and again
  today, and both times it had run to many times the limit. A spec that counts
  the live file's headings and fails above forty would hold it. That spec is
  `tests/`, so it is cloud's: #1854.

## 2026-10-10 — three design notes retire, and a July ruling reaches the log

This closes #1851. CLAUDE.md says a design note whose work has shipped is
deleted, and that no file but the tracker carries pending-decision text. Five
notes were read against the tree.

**`design/enumerable.md` is deleted.** Its vocabulary is in `lib/list` and is
taught in ch12 and appendix B, including the collision rule. The two rulings
its status line called interim were ratified by Clay on 2026-08-14 (`range` is
unbound and the statistic is `spread`; map builders are last-write-wins, like
`put`), and §6 of the note already said so. Its one item marked open,
`first coll n` beside `take n . to_list`, was settled by what shipped:
`lib/list` has `first coll` alone, so `take` and `to_list` is the one way to
take n. Its §7 described build blocks, which retired today.

**`design/testing.md` is deleted.** The `testing` hako ships in `lib/testing`,
`lib/json/json_test.kso` uses `when_failed`, and ch07 and ch08 teach it. The
note's deferrals stay deferred and are recorded here so they keep a home: the
2026-07-28 describe/context sketch is superseded until real suites show the
need, with its two-deep limit still the right cage if they do; effectful tests
are out of scope until io testing gets its own design against the boundary
language; and two small observations were noted and not proposed, that a lambda
cannot carry arms (so dispatching on a reason inside `when_failed` takes a named
group) and that `when_failed` answering false on success folds "did not fail"
into "failed the wrong way".

**The work-ahead engine was ruled on 2026-07-22 and the ruling was never
logged.** It lived only in `design/workahead-engine.md`. Recorded now: when a
fiber parks on io, the scheduler runs proven-needed work in first-use order,
then speculative work, keyed off logical scheduler state so the counters stay
deterministic; v1 is proven work alone. The wall-credit substrate and the
`bench/workahead/` rig shipped, and the engine did not. The note's
implementation detail moves to #1856 (`ruled`, `lane:cloud`), and the note is
deleted. docs/compiler.html's section remains the public description.

**`design/hako.md` stays.** It is the only specification of `kanso install`,
`list` and `update`, which ship, and `src/hako.rs` cites it. Its open-questions
section pointed at the ledger, which is retiring under #1852; it now says the
`std/` question was ruled on 2026-08-29 (std ships inside the binary) and keeps
signing and monorepo hakos as parked.

**`design/memory-frontier-research.md` stays.** Its table is the record of
which memory ideas shipped, which were declined on measurement and which were
refuted, and its refuted list is what stops an idea from being rebuilt. Rows
4.1 and 4.4 concerned build blocks and are marked moot.

The oldest live entry moves to the archive with this one, so the live file
stays at forty.

- **DONE** #1851.
- **OPEN** #1856, the work-ahead engine, cloud's.

## 2026-10-10 — CLAUDE.md states its rules once

This closes #1849. CLAUDE.md had grown to 927 lines, most of them the history
of its own rules: dated stories of how each was learned, counts with the dates
they went stale, and corrections to earlier corrections. Its design-flow rule
says chronology belongs in the log, and its hygiene section says instructions
are rules. It is now 498 lines.

Every rule the old file stated with a bold lead is still there, checked by
listing the old file's bold leads and finding each one in the new text. What
went is the stories, the session tables and the dated counts. The full earlier
text is `git show be71feb8:CLAUDE.md`, and the stories are in the log entries
of their own dates.

Three things changed beyond trimming:

- The two-grep ruling count was written as one pipeline that fed the first
  count into the second grep, so it printed one meaningless number. It is two
  commands now.
- "Branches are free" moved from "Do not stop" to "Access and permissions",
  beside the other rules about what needs Clay's permission.
- The chat's lane names the design notes, which it has edited since the lanes
  were drawn and which #1851 was filed against.

Two specs read CLAUDE.md's wording, and both still pass. One wants the
cost-golden count word in "all TWELVE cost goldens,", the one count left in
the file, and #1846 asks cloud to retire it. The other wants every compile
gate named in the bullet containing "are separate gates". It went red during
the rewrap when that phrase broke across a line, which confirmed it reads the
phrase literally.

The oldest live entry moves to the archive with this one.

## 2026-10-10 — a pop after a skipped lap walks the whole result

With the two carry faults of kanso#1834 fixed, the sat port still failed under
the lap carry: `rand75_unsat` reported a satisfying assignment, and mal's
self-hosted runs died with a segmentation fault. Both came back right with the
pop's copy-out made deep, so the search went there.

When a carrying loop returns, `k_beat_pop_slow` copies its result out of the
loop's storage. That walk has no mark, so it counts every arena node as a
survivor, and it stops at a node whose direct interior survives as well. Under
the old rule every lap staged, so whatever the result held in the depth's carry
pair sat under the pair's own nodes, and the walk reached and copied it. The
lap carry lets laps go by without staging. Such a lap builds its new nodes in
the arena, around values that are still in the pair. The pop's walk meets the
fresh outer node first, finds its interior fresh too, and stops. The value in
the pair is left where it was. The next loop entered at the same depth stages
into the same pair and writes over it.

`tests/golden/micro/a_pop_after_a_skipped_lap_keeps_what_the_lap_built.kso` is
the shape in a dozen lines: a loop that wraps a box four times, of which only
the first lap stages, followed by a loop at the same depth that builds twenty
thousand elements a lap. The second loop's list replaced the first loop's
innermost box, and the program printed `<<<record>>>` where the interpreter
prints `<<<<[1 2 3]>>>>`.

The carry now records when a lap skipped its stage, in the lap carry's drift
test and in the chain's. A stage clears the record, and so does the push that
opens a depth. A pop that finds it set walks the whole result, as it already
did when a write had reached the carry. A pop after a loop whose last lap
staged is unchanged.

mal still crashed after that, in a map lookup reading a key whose length word
held a pointer. A hardware watchpoint on the key showed it written twice by
stage copies into a carry buffer and then overwritten by a later stage that
reused the buffer. The deep walk at the pop had found a map whose header and
pairs lay in the arena and whose keys lay in the pair. It copied the keys out
and rewrote the pairs in place. The map's sorted view is a second, malloc'd
copy of the pairs that the walk never reaches, so it went on naming the old
keys, and the next lookup searched it. The walk now drops the view whenever
it moves a key or a value in a map it repairs in place, and the next read
sorts again.

`tests/golden/micro/a_sorted_view_follows_the_keys_a_pop_moves.kso` builds
the shape: a loop that adds a key a lap and reads the map back, staging every
tenth lap, followed by a second run of the same loop with keys of the same
length. Without the change the program dies with a segmentation fault; with
it, it prints what the interpreter prints. Main does not reach this path,
because its pops copy deep only after a write has reached the carry.

The deep walk first cost memory where it should not have. It meets a string
builder whose header survives and goes straight to the in-place repair, which
asks only whether the builder's storage lies in the arena. A builder's storage
is malloc'd, so that answer was no, and the repair copied it into the arena and
dropped the builder's capacity, leaving a plain string the next append had to
copy again. The survival test the shallow walk uses already passes a builder,
and a malloc'd byte buffer, for this reason. The repair now passes them too.
With that, `string_builder_shape`, `builder_counts_once` and
`a_builder_handed_on_is_still_a_builder` are back to main's counts; with the
repair copying, `string_builder_shape` had evacuated 4,096 bytes where it
evacuates 80.

Against main, the counters that see a pop fall or hold. basket allocates
27,263 nodes, one fewer, and asks 4 survival questions where it asked 16,002;
the run program's survive count falls from 105,679 slots to 4,854 and it
evacuates 6,441,632 bytes where it evacuated 9,789,344. `record_reuse_shape`
asks 8 survival questions where it asked 16,006, and `effect_push_shape` 10
where it asked 15.

The ports' long-running loops, measured by peak resident memory on this branch
with main's fixes merged in, against main alone. mal's `(fib 25)` held 764 MB
on main and holds 9 MB here, and a 40,000-step tail call that held 147 MB
holds 9 MB; both print what they printed before. That closes mal's F8. lox's
F18, a loop that builds a `Point` a pass, is not closed by this change alone:
it holds 1,131 MB at 20,000 passes on both trees. With kanso#1824 added as
well it holds 24 MB at 20,000 passes and 88 MB at 80,000, which is what the
interpreter holds, 31 MB and 93 MB. The lox port keeps every object in a heap
of cells that it never collects, so that growth is the program's own. What
stays slow is time: each stage copies the whole live heap again, and 200,000
passes still had not finished after 120 seconds.

CI measured the rest against main, and most of it costs. Every benchmark's
machine code grows by between 9,200 and 9,472 bytes, the runtime's share of
the lap carry; runbench's is 611,769 bytes and the fourteen together 4,781,774.
work_deepbench does the most extra work, landing on 393,576,371 against
main's 367,269,411, 7.2% more. runbench, with the four phases kanso#1836 added, does 1,247,098,999 against main's 1,245,043,324, 0.17% more; encodebench
2,398,358,806, 0.19% more; livebench 1,473,968,154; jsonbench 860,559,756;
oneshot 13,210,643; work_widebench 28,175,109; digestbench 5,599,654; escapebench
39,045,879; indexbench 2,459,068; scanbench 270,069; readbench 4,577,565.
basket does less, 30,415,829 against 32,505,156, and so does pendbench,
190,050,778 against 190,396,306. The deep walk at the pop and the record of a
skipped lap are what this change adds to those paths, but no build has
separated the two, so the delta arrived with the change and its split between
them is open. On the compile side, release codegen counts 454,310,397, dev
codegen 118,946,233, compile_instructions 25,858,385, entry_instructions
85,515,649 and library_instructions 86,067,297, each a little higher, and
emit_instructions falls to 7,058,267. In the browser, compiling the corpus
takes 390,605,563 instructions with a peak of 675,725 bytes, and running it
34,102,877.

Together those cost the objective 0.0035: the meta reads 90.2465 against a
floor of 90.2499. The change is a correctness fix, because a loop that holds
764 MB to compute `(fib 25)` is a program that fails on a larger input, and
the floor goes down by exactly that much under the rule for changes that make
the language work as specified.

## 2026-10-10 — a fold that joins onto its accumulator writes in place

The mustache port renders a template by folding over its pieces, and its
folder answers `"{acc}{piece}"`. Natively, every step copied the whole string
built so far, so 20,000 pieces held 2.77 GB before the program finished. The
interpreter was not affected.

The string-builder analysis already wrote a join in place when the
accumulator is carried by a recursive function. A lambda handed to a fold was
never looked at, because `walk_for_builder` stops at every lambda. A fold's
folder runs once per element on the accumulator the fold threads through, so
when `fold_owns_accumulator` says the fold owns its seed, a join onto the
folder's first parameter has the same shape as the recursive case. The walk
now marks those joins: in the folder's body, in either arm of an `if`, and
through the wrapper lambda that `body_is_folder` already accepts.

A folder passed by name, as in `list/fold xs "" piece`, is handled the same
way under three conditions. Every arm of `piece` must end in a join onto its
first parameter. Every direct call of `piece` must hand that argument over.
Every use of the name as a value must be the folder of a fold whose seed
nothing else holds. A partial application of `piece` keeps what it was given,
so it disqualifies the group.

The runtime's in-place join stopped the program when its accumulator was not
already a builder. A fold's seed is usually a literal, so `k_concat_arr_mut`
now converts it on the first join, the way a builder parameter's caller does.

`tests/golden/mem/a_fold_that_joins_onto_its_accumulator_grows_one_string`
folds 2,000 pieces with a named folder and with a lambda. `alloc_bytes` falls
from 21,839,568 to 1,221,150 and `arena_peak_bytes` from 22,020,096 to
2,097,152. `tests/golden/micro/a_fold_joins_onto_its_own_accumulator` covers
the ways the string can be shared: a seed bound to a name, one fold's result
folded twice more, and a direct call of the named folder. Making
`fold_owns_accumulator` answer yes unconditionally turns it red. The two folds
over one lambda result then wrote into the same buffer, and both printed
`[1][2][3]+5-6`.

No benchmark folds a string, so every allocation counter agrees with main.
The runtime's join grows 320 bytes of machine code, and CI's rows move with
it:

- `text` grows 320 bytes a binary, to 4,593,438 in all.
- `work_basket` lands on 32,404,790, `work_runbench` on 1,129,589,259,
  `work_livebench` on 1,473,486,345, `work_encodebench` on 2,384,338,137,
  `work_oneshot` on 13,027,190, `work_digestbench` on 5,566,917 and
  `work_scanbench` on 281,806, each within 22,030 instructions of main.
- The analysis costs `compile_instructions` 25,857,543,
  `entry_instructions` 85,513,471 and `library_instructions` 86,066,360.
  `emit_instructions` falls to 7,058,306, `interp_instructions` to
  488,868,354, and both codegen rows fall.
- `browser_compile_instructions` rises from 388,182,464 to 391,515,764,
  0.86%, and the browser run row falls 6, to 33,510,702. The rise does not
  come from the new pass: with `named_folder_joins` switched off the row reads
  392,251,588, higher still, so it follows the wasm binary's layout rather
  than the work done.

On main as it stood before kanso#1836, welfare fell from 90.25811251 to
90.25519022 on those rows, because no benchmark folded a string.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `fold` phase: 800 pieces folded onto one
string, measured on main before this fix at 858,398 instructions. With main
merged in, runbench falls from 1,245,043,324 to 1,244,977,121, a projection
from this container's 1,244,976,714 plus the 407 instructions by which CI read
main above it; CI's row replaces it. The phase's memory is where the fix pays.
run_arena_peak_bytes falls from 8,912,912 to 7,864,336, run_arena_blocks from
8 to 7, run_alloc_bytes from 326,862,802 to 325,331,283, run_allocs from
1,773,132 to 1,772,340 and run_sh_str from 10,188,896 to 8,648,960. Two run
counters rise with the fix: run_str_scans goes from 229 to 1,021 while
run_str_scan_bytes falls to 17,930,429, and run_bytes_malloc rises by 7 to
9,127. Nothing here isolates which step of the in-place join each comes from.
The rows the runtime's 320 bytes moved above
carry onto main's values: `text` totals 4,656,446, work_basket lands on
32,527,186, work_livebench on 1,472,861,727, work_encodebench on
2,393,926,750, work_oneshot on 13,176,997, work_digestbench on 5,598,179 and
work_scanbench on 268,886. The analysis's rows carry the same way:
compile_instructions lands on 25,857,737, entry_instructions on 85,514,580,
library_instructions on 86,066,207 and browser_compile_instructions on
391,827,307. All of these are projected until CI reads them. Welfare rises from 90.2499
to 90.3104 on those rows, and the rise is banked.

CI's rows replace the projection. work_runbench lands on 1,244,977,121, as
projected. The other rows CI corrects are small: `text` totals 4,656,478,
work_encodebench lands on 2,393,926,869, work_livebench on 1,472,861,230 and
browser_compile_instructions on 391,826,076. The run program's emitted code
reads one call and two lines more than this container wrote, so
emitted_other_calls lands on 17,394 and emitted_other_lines on 146,698.
Welfare reads 90.3105 on CI's rows, and the rise is banked.

The conversion in `k_concat_arr_mut` made `k_b_adopt` redundant, and the
ratchet found it on the carrier. Two rows mutated the adoption: "a join that
never adopts its seed" removed it, and "a lazy seed taken for a builder"
handed it the forced value. Both stayed green, because a seed the adoption
misses now reaches the first join without room and is made into a builder
there. Neither mechanism could be removed and seen red while the other stood.
The two rows are replaced by "a first join that never makes its builder",
which restores the old refusal in `k_concat_arr_mut`. Under it
`a_fold_joins_onto_its_own_accumulator` prints nothing from native and the
micro corpus goes red. `k_b_adopt` is left in place for now and goes in a
change of its own, since removing it moves the text and codegen rows and
needs CI to read them.

## 2026-10-10 — a loop entered from a tail cycle gives its garbage back

The lox and mal ports both ran their interpreters out of memory. A Lox `for`
loop of fifty thousand passes that builds two small objects per pass held
3.0 GB at its peak natively, and mal's tail-recursive `sum2` kept 3.7 KB per
iteration until exit. The interpreter under `--interp` ran the same programs
in about 11 MB.

Each port's machine loop is a self-recursive function, the shape the beat
analysis rewinds, and neither rewound. The lox loop, `spin`, failed two
tests. The first was its entry. `exec` tail-calls `spin` from its while arm,
and `exec` sits in a tail cycle of its own, since a block runs its first
statement and tail-calls itself on the rest. The analysis demoted a tail
entry to a plain call only when the caller was in no tail cycle at all, on
the reasoning that a plain call inside a cycle grows the stack. That holds
only when the cycle runs through the demoted edge, which happens when the
loop can reach its caller back by tail calls. `spin` returns its state to
`exec` and never tail-calls it, so the plain call holds one frame while the
loop runs and the caller returns before its cycle goes round again. The
condition now asks exactly that: the entry is demoted unless the loop
tail-reaches the caller.

The second was the loop's own arguments. `spin` hands its body and condition
on unchanged, and both come out of a record field, which inference reads as
any value at all. A slot of unknown type was never carried through a
rewind, in case it hid a growing accumulator behind a helper call. A
parameter every self-tail-call passes on as itself cannot grow: it arrived at
entry, lives below the mark, and the evacuation shares it rather than copying
it. Such a position is now carried whatever its type.

With both changes the lox loop rewinds every pass. At fifty thousand passes
the release binary peaks at 77 MB, down from 3.0 GB, and runs in 1.17 s
against 3.01 s. What remains is the port's own heap of instance fields, which
it never frees by design. The fixture
`a_loop_entered_from_a_tail_cycle_gives_its_garbage_back` builds the same
shape in thirty lines and pins `arena_peak_bytes` at 1,048,576 for twenty
thousand passes; main holds 3,145,728 for it.

mal's machine is not fixed by this. Its `ev` is one member of a tail cycle
with `ret`, `throw` and `settle`, which makes it a cluster, and a cluster
carries only scalars and threaded parameters through its rewind. Carrying a
heap value round a cluster is the next piece of work.

No benchmark's run changed: every runtime cost vein agrees with its golden.
The analysis is dearer to run, on CI's rows after merging main:
`compile_instructions` rises to 25,861,592, `entry_instructions` to
85,526,009 and `library_instructions` to 86,078,947, while
`emit_instructions` falls to 7,032,277 and `interp_instructions` to
488,876,843. `browser_compile_instructions` rises from 388,182,464 to
390,287,782 with this tree's rustc. On main as it stood before kanso#1836,
welfare fell from 90.25811251 to 90.25621395, because no benchmark ran a loop
entered from a cycle.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `machine` phase: 20,000 passes of a loop in
a cycle, measured on main before this fix at 8,825,906 instructions. Rewinding
the loop costs instructions and gives memory back. With main merged in,
work_runbench rises from 1,245,043,324 to 1,263,682,936, a projection from
this container's 1,263,682,529 plus the 407 instructions by which CI read main
above it; CI's row replaces it. run_arena_peak_bytes falls from 8,912,912 to
6,815,760 and run_arena_blocks from 8 to 6. The rewind's own work shows in the
rows that rise: run_beat_iters lands on 47,564, run_evac_allocs on 128,717,
run_evac_bytes on 11,709,440, run_alloc_bytes on 326,929,762, run_allocs on
1,773,139, run_sh_buf on 119,108,240 and run_buf_reuse on 134. Carried onto
main's rows, compile_instructions lands on 25,861,786, entry_instructions on
85,527,118, library_instructions on 86,078,794 and
browser_compile_instructions on 390,599,325, all projected until CI reads
them. The objective weighs the peak above the instructions: welfare rises from
90.2499 to 90.3087 on those rows, and the rise is banked. The fix's own
fixture, `a_loop_entered_from_a_tail_cycle_gives_its_garbage_back`, reads
four fewer evacuations on main's runtime: evac_allocs lands on 80,013 and
evac_bytes on 1,920,368.

CI's rows replace the projection. work_runbench lands on 1,263,682,936, as
projected. The rewind's code in the run program's machine phase shows in its
emitted counts: emitted_other_calls lands on 17,405, emitted_other_branches on
16,399 and emitted_other_lines on 146,723. `text` totals 4,652,398 and
browser_compile_instructions lands on 390,597,579. Welfare reads 90.3087 on
CI's rows, the score already banked.

## 2026-10-10 — a read named before the write keeps the write in place

Three ports hit the same cliff. The sat solver, the toml parser and the lox
interpreter each read a map entry into a name and then wrote the map:

```
v = m[i]
bump (put m i (v + 1)) ...
```

Written inline as `put m i (m[i] + 1)`, the put happens in place. Written
with the name, `linear.rs` counted `v = m[i]` as a second use of `m`, so `m`
was not unique at the put and every put copied the map. The sat port's
reduction, 20,000 steps over 100 keys, ran in 27.5 seconds natively with the
name and 0.010 seconds without it.

The two programs are the same as far as the map is concerned. The read
answers an element, and `v` is used only as an operand of the put's own
arguments, which are evaluated before the put writes. `effective_uses` now
discounts a binding's mentions of the map when the bound name meets three
conditions: neither the name nor the map is rebound in the block, the
binding does not hold the map itself, and every later mention of the name is
reached from the sibling arguments of a `put`, `push` or `append` on that
map through arithmetic and comparison alone. A mention anywhere else (inside
a lambda, a list, another call, or a second binding such as `w = v * 2`)
could force the read after the write, and the binding still counts. A loop's
bindings usually sit under its `return ... if` guard, so the search goes down
through the guard's rest.

`tests/golden/mem/a_read_named_before_the_write_stays_in_place.kso` is the
sat reduction at 2,000 steps. Its golden pins 8 allocations; the compiler
before this change made 6,013 on it.
`tests/golden/micro/a_read_named_before_a_write_sees_the_old_value.kso`
covers the other side: a name used after the write, a name stored in the
list the put writes, and a name that feeds a second binding all keep the
copy, and each line prints the old value beside the new on every engine.

The analysis costs the front end a little on every route: compile_instructions
lands on 25,857,737, entry_instructions on 85,514,349, library_instructions on
86,065,981, emit_instructions on 7,065,059 and interp_instructions on
489,220,284. browser_compile_instructions lands on 392,737,085. These are
carried onto main's rows from this branch's own readings and are projected
until CI reads them.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `bump` phase: 350 updates through a named
read, measured on main before this fix at 53,247,360 instructions. With main
merged in, runbench falls from 1,245,043,324 to 1,192,648,013, a projection
from this container's 1,192,647,606 plus the 407 instructions by which CI read
main above it; CI's row replaces it. run_held_peak_bytes falls from 3,222,256
to 197,704 and run_arena_peak_bytes from 8,912,912 to 7,864,336, because the
phase no longer keeps a copy of its map per update. run_alloc_bytes falls to
322,971,202, run_allocs to 1,772,081 and run_sh_map to 0. run_buf_reuse falls
by one to 136, and run_view_allocs and run_view_frees fall from 360 to 10 with
the fix; nothing here isolates why the scatter phase's views moved.

CI's rows replace the projection. work_runbench lands on 1,192,648,013, as
projected. The front-end rows land on entry_instructions 85,514,580,
library_instructions 86,066,207, interp_instructions 488,962,208 and
emit_instructions 7,059,706, and browser_compile_instructions on 392,738,164.
`text` totals 4,652,222. The run program's emitted code reads one line fewer,
50,993. Welfare reads 90.6830 on CI's rows, and the rise is banked.

## 2026-10-10 — a map's view takes keys out of order without moving itself

Three ports found the same cliff from three directions. xsv's
`stats --cardinality` spent 2.6 of its 2.7 seconds counting a column of
distinct values, ugit's Myers diff on a 3,000-line file took 4.0 seconds
against the interpreter's 0.4, and `list/tally` over 40,000 distinct
strings took 4.95 seconds natively where the interpreter took 0.2. All of
them read a map between puts. A read builds the map's sorted view, and
from then on each put of a new key was inserted into the view in order,
which for a key landing in the middle meant moving every pair after it
one slot along. Callgrind on the ugit diff put 80% of the time there.

A key that would land in the middle of a view of more than sixty-four
pairs now goes to a second sorted run beside it, which the runtime calls
the side. A key past the view's last still goes on its end, so a map
filled in ascending order never starts one. Reads search the view and
then the side, so a read between two puts merges nothing. When the side
holds more keys than the square root of the view, the two are merged,
from the back and into the view's own buffer when it has room. Anything
that wants the whole order, such as printing, `entries` or equality,
merges first.

The side costs the map header nothing. A view's buffer is preceded by a
header slot whose first word is its capacity, and the second word, which
was unused, now points to the side. A mark on the map's view pointer
says a side is waiting, so a read that finds the bit clear asks nothing
more, and the emitted fast put, which only tests the pointer for null,
still takes the call when a view exists. The side is kept, emptied, after
a merge, so the next key out of order does not allocate.

`list/tally` over 40,000 distinct strings went from 4.95 seconds to 0.21,
and xsv's reproduction from 4.75 to 0.17. `tests/golden/mem/
a_tally_of_scattered_keys_merges_its_view.kso` pins the shape: 4,000
scrambled keys, each read before it is put, take 15 view allocations where
main took 12, and hold 206,448 bytes at peak where main held 196,576.
Two mem goldens move with it. `fused_tally` takes one more allocation (65
to 66, alloc_bytes 41,536 to 42,064), and `growing_map` two more (1,613 to
1,615), with held_peak_bytes 49,120 to 55,408 and alloc_bytes 181,952 to
189,312.
`tests/golden/micro/a_map_written_out_of_order_past_its_room.kso` puts 211
keys in scrambled order with reads, a length, a missing key and a print in
between, and one in-place loop visits every key twice, so its second visit
reads and replaces keys that are still in the side. Every engine prints the
same. Leaving the side out of the search turns it red: the loop reads a key
it has already put as missing, puts it again, and prints 217 keys.

The first version of that fixture could not fail. Its loop was also handed
a map the play went on reading, so the linearity analysis gave none of its
puts the in-place write, no view ever grew a side, and removing the side
search changed nothing. A separate loop that only ever receives a fresh map
is what made the side reachable.

A put that replaces a key already in the map still walks the pairs log
backwards to find its last write, so a tally whose keys repeat pays for
the distance back to that key's last occurrence. That walk is untouched
here.

The first round of CI put welfare 0.0000 below its floor, and all of the
fall was in the run program: runbench rose from 1,129,588,698 to
1,130,533,679. Callgrind on both builds put the whole difference in one
function, `k_b_columns`, which `keys` and `values` reach on every call, and
its disassembly showed why. It asks `k_map_sorted` for the view, and the
check before that call grew from one test (is there a view?) to two (is
there a view, and does it have a side?), five instructions where main had
two. The side mark was the pointer's low bit.

The mark now sits in the pointer's top bit. A 64-bit address never sets it,
so read as a signed number the view pointer is positive with no side, zero
with no view and negative with a side, and one test against zero asks both
questions. `k_b_columns` came back to main's instructions with one alignment
nop added before its copy loop. Measured locally from one directory against
a build of main's runtime, runbench is 207,423 instructions above main and
livebench 903,510, down from 945,000 and 4,208,000. wasm32 keeps the low
bit, because a 32-bit address can have its top bit set.

CI's rows for this head with main's constant storage merged in, against
main: work_runbench rises from 1,131,250,979 to 1,131,457,856 and
work_livebench from 1,472,859,260 to 1,473,762,140, the remains of the check
described above. work_basket rises from 32,375,114 to 32,926,353, 1.7%,
work_encodebench to 2,383,871,632, work_oneshot to 13,189,990 and
work_pendbench to 190,390,734. Every binary's text is about 4,800 bytes
larger, which sums to text 4,612,254 to 4,679,230. browser_run_instructions
goes from 33,493,063 to 33,502,036, codegen_instructions_dev from 118,896,762
to 118,916,117 and codegen_instructions_release from 453,707,138 to
454,001,319, all carrying the larger runtime. basket builds maps and reads
them between puts, so it is the benchmark that meets the side; which of its
maps grows one is not isolated here. The mem rows are the ones named above:
fused_tally_allocs 65 to 66 and fused_tally_alloc_bytes 41,536 to 42,064;
growing_map_allocs 1,613 to 1,615, growing_map_alloc_bytes 181,952 to 189,312
and growing_map_held_peak_bytes 49,120 to 55,408. Welfare falls from
90.25153 to 90.25054. The gate passes that, because it allows 0.001 for
disagreement between hosts, but every term here is deterministic, so the fall
is real. It raised the floor question for this fix and three other port
fixes, and the paragraph below records how that question closed.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `scatter` phase: 2,340 keys counted in
scrambled order, measured on main before this fix at 50,901,814 instructions.
With main merged in, runbench falls from 1,245,043,324 to 1,201,828,599, a
projection from this container's 1,201,828,192 plus the 407 instructions by
which CI read main above it; CI's row replaces it. The run's held peak falls
from 3,222,256 bytes to 3,166,752, run_alloc_bytes falls to 326,749,138 and
run_allocs rises by two to 1,773,134. Welfare rises from 90.2499 to 90.4099 on
those rows, and the rise is banked.

CI's rows for the merged tree replace the projection and agree with it on the
run program: work_runbench lands on 1,201,828,611. The merge had carried main's
rows for the other benchmarks, and CI restores this fix's own cost on them.
The view code adds 4,784 bytes to every binary, so `text` totals 4,718,942.
work_basket lands on 33,056,395, work_livebench on 1,473,763,578,
work_encodebench on 2,393,936,650, work_oneshot on 13,190,150 and
work_pendbench on 190,396,604. codegen_instructions_dev lands on 118,940,067
and browser_run_instructions on 33,517,389. Welfare reads 90.4099 on CI's rows,
the score already banked.

## 2026-10-10 — the five port fixes land together

kanso#1820, kanso#1822, kanso#1824, kanso#1825 and kanso#1827 each merged main
and scored against the run program on their own. Each also moves the runbench
row and the floor, so landing them one at a time would cost a CI round per pull
request. This branch merges all five onto main and is measured as one tree.

The fixes do not add up. Measured separately against main, their runbench
deltas sum to -74,980,545, which would put the row at 1,170,062,384. The
combined tree reads 1,152,573,228 in this container, 17,489,149 lower, so it is
projected at 1,152,573,635 until CI reads it. The loop rewind (kanso#1824) and
the lap carry (kanso#1827) both change what a rewound loop keeps, and the loop
fixture `a_loop_entered_from_a_tail_cycle_gives_its_garbage_back` shows the
overlap most plainly: with both fixes evac_allocs falls from 80,013 to 49 and
evac_bytes from 1,920,368 to 1,232. Which of the two fixes accounts for the
runbench difference is not isolated here.

The run program's counters land on the combined tree as run_beat_iters on 47,564, run_buf_reuse on 132, run_bytes_malloc on 9,127, run_sh_buf on 119,108,240, run_str_scans on 1,021, run_ten_frees on 1, run_ten_handups on 1, run_view_allocs on 12, run_view_frees on 12. The mem
fixtures the map fix moved land as fused_tally_alloc_bytes on 42,064, fused_tally_allocs on 66, fused_tally_view_allocs on 16, growing_map_alloc_bytes on 189,312, growing_map_allocs on 1,615, growing_map_held_peak_bytes on 55,408, growing_map_view_allocs on 12, growing_map_view_frees on 10. The benchmarks' work rows carry
each fix's own delta onto main's row until CI measures them together:
work_deepbench on 393,576,370, work_digestbench on 5,599,739, work_encodebench on 2,398,369,413, work_escapebench on 39,045,874, work_indexbench on 2,459,056, work_jsonbench on 860,559,605, work_livebench on 1,474,870,880, work_oneshot on 13,223,306, work_readbench on 4,577,564, work_scanbench on 270,158, work_widebench on 28,175,107. So do the compile and browser rows: compile_instructions on 25,938,043, entry_instructions on 85,762,025, library_instructions on 86,314,325, codegen_instructions_dev on 118,966,405, codegen_instructions_release on 454,110,241, browser_compile_instructions on 400,285,361, browser_compile_peak_bytes on 675,725, browser_run_instructions on 34,111,788. `text` totals
4,853,950. Every mem fixture that predates the lap carry gains its
`chain_finds=0` line.

Welfare reads 91.0246 on these rows against the five floors banked separately,
the highest of which was 90.6830, and the rise is banked.

Main moved under the branch when kanso#1837 retired `build`, and the carrier
took main in a merge commit. The compiler's front end lost the passes `build`
needed, so the rows the carrier's analysis had raised land below where they
stood before the merge and above main's: compile_instructions lands on
25,715,103 against main's 25,712,749, entry_instructions on 85,044,877 against
85,036,744, and library_instructions on 85,599,176 against 85,590,951.
interp_instructions lands on 488,792,115 against 488,375,858. The two codegen
tiers land on 118,954,619 for codegen_instructions_dev and 454,407,182 for
codegen_instructions_release, against main's 118,906,508 and 453,887,359.
browser_compile_instructions lands on 390,036,773 against 388,285,810. Each
runtime binary sheds 448 bytes with `build` gone, and `text` totals 4,826,814
against main's 4,645,694. The work rows CI read on the merged tree are
work_encodebench on 2,398,369,791, work_livebench on 1,474,871,454 and
work_oneshot on 13,223,698; work_runbench lands on 1,152,754,428, against
main's 1,245,043,177. Welfare reads 91.0337 on CI's rows against the floor of
91.0317, and the rise is banked.

## 2026-10-10 — a lap waits for twice what the last stage copied

Testing the combined tree on the lox port's F18 loop found two things the five
fixes did not do on their own. Peak memory fell as expected: at 40,000 passes
the loop held 45 MB against main's 2,333 MB. But past about 135,000 passes the
run slowed sharply, from 2.7 seconds at 130,000 to 99 seconds at 160,000. Stack
samples put the time in `k_copy_size` and `k_deep_copy` under
`k_beat_lap_carry`, and the counters showed evac_allocs jumping from 5.8
million at 130,000 passes to 139 million at 140,000. Tenure stops at
`K_TEN_CAP`, 64 MiB, and lox's heap map keeps every object it allocates, so
from that point on every stage copied the whole live state, once per quarter
megabyte of drift.

A smaller program then showed a worse case. The loop of
`a_loop_entered_from_a_tail_cycle_gives_its_garbage_back`, with a map in its
state that keeps one entry a pass, ran 20,000 passes in 0.01 seconds on main
and in 3.2 seconds with kanso#1824 alone, allocating 9.5 GB. kanso#1824 is what
makes this loop rewind at all. Each stage copies the map out of the arena, the
next put copies it back, and that copy is a quarter megabyte of drift on its
own, so a stage followed almost every pass.

`k_beat_lap_carry` now waits for twice what the last stage copied before it
stages again, and never less than the quarter megabyte it waited before.
Copying then stays proportional to the drift it answers, which is how a copying
collector sizes its heap. When the arena has opened blocks since the last stage
and the wait has grown past the quarter megabyte, the drift is measured across
them: the rest of the old block, every block between, and what the newest
holds. A loop whose stages copy little behaves as it did, including staging
when the arena opens a block. The first version counted the newest block at
its full size, and with a 2 MiB block that staged on most passes.

`tests/golden/mem/a_growing_map_in_a_rewound_loop_waits_for_its_copy` is that
small program at 20,000 passes. Its alloc_bytes is 44,612,576; without the wait
it reads 9,165,495,888. No other mem fixture and no cost golden moves. On the
lox loop, 160,000 passes now take 5.3 seconds and 320,000 take 33.

Time still grows faster than the pass count, and the counters say where.
Allocation is linear: alloc_bytes reads 2,457,232,021 at 40,000 passes,
5,168,271,765 at 80,000 and 10,642,119,589 at 160,000, about 66 KB a pass.
evac_bytes is not. It reads 63,965,040, 132,744,688 and then 1,296,200,208,
ten times the step before, while ten_blocks only goes from 8 to 9. That is the
pass count at which the port's heap outgrows tenure. Below it, tenure holds the
heap and a stage copies only what the pass made. Above it, each stage copies the
live heap, and the wait keeps that to half of the drift at most, so copying is
bounded by allocation but no longer small next to it. The cost per byte copied
also rises as the heap grows, because the copy keeps a pointer map for shared
values and the map grows with the heap. Removing it would mean letting tenure
grow past `K_TEN_CAP` for a loop whose survivors keep surviving. The cap exists
so that no loop holds storage that only the pop frees, and changing it is a
separate piece of work.
runbench reads 180,786 instructions more for the wait's arithmetic and is
projected at 1,152,754,421 until CI reads it. Welfare falls from 91.0246 to
91.0239, which this fix pays to remove a regression the carried kanso#1824
would otherwise ship.

CI has now read the combined tree, lap wait included. runbench reads
1,152,754,435, fourteen instructions over the projection, so the wait's
180,786 holds. The rows the projections had carried land as follows.
work_encodebench lands on 2,398,370,183, work_escapebench on 39,045,884,
work_indexbench on 2,459,212, work_livebench on 1,474,870,796, work_oneshot on
13,223,628 and work_scanbench on 270,249. work_basket falls to 30,812,505.
compile_instructions lands on 25,832,558, entry_instructions on 85,436,280,
library_instructions on 85,987,742, emit_instructions on 7,037,238 and
interp_instructions on 489,012,183. codegen_instructions_dev lands on
118,951,226 and codegen_instructions_release on 454,427,018.
browser_compile_instructions lands on 391,401,241, browser_compile_peak_bytes
on 675,725 and browser_run_instructions on 34,124,829. The run program's
emitted code gains 29 lines, which moves emitted_other_branches to 16,399,
emitted_other_calls to 17,406 and emitted_other_lines to 146,724, and `text`
totals 4,833,086. Welfare reads 91.0317 on CI's rows, and the rise is banked.

## 2026-10-10 — an oversize block that cannot be reused is freed when it misses

A loop that appends a byte a pass to a one-megabyte string ran 6,000 passes
with an arena peak of 3,157,760 bytes and a resident peak of 777 MB. The arena
gives a value over a megabyte a block of exactly its size. When the loop
rewinds, that block goes to the spare list, and the next pass asks for a block
a few bytes larger, which no spare can serve. So every pass opened a new block,
and the old ones stayed in the spare list until the loop ended. Valgrind
counted 99 mallocs and 82 frees at 300 passes, nearly all of the frees at the
loop's end. Main reaches 394 MB on the same program, and the five port fixes
had roughly doubled it, because they rewind more often.

`k_arena_push` now frees the oversize spares smaller than the request when an
oversize request misses. The standard 1 MiB spares are kept, since they serve
ordinary refills. The program's resident peak falls from 777 MB to 10 MB, and
it runs in 0.96 seconds against 1.2.

No counter could see this. `arena_peak_bytes` counts blocks in use and
`arena_blocks` counts mallocs, and both read the same before and after: 752
blocks and a peak of 3,157,760. A new counter, `spare_peak_bytes`, records the
most the spare list held, and it is lower-is-better in the trend gate. It is
counted only in counting builds, so no program pays for it. Every cost golden,
every mem fixture and the two book samples that print counters gain the line
and nothing else.

`tests/golden/mem/a_string_that_grows_each_pass_reuses_its_block` is the
reduction at 400 passes. Its spare_peak_bytes is 2,097,984; with the miss path
left as it was, it reads 52,440,000. The run program's spare_peak_bytes is
2,621,456. runbench reads 248 instructions more, measured in one worktree
against the same tree without the fix, and is projected at 1,152,754,683.

One reading from this container does not agree with CI. Measured in the
carrier's own worktree, runbench read 1,152,573,288, below the lap wait's
figure; CI reads the carrier at 1,152,754,435, which agrees with the wait's
cost. That stray reading is not explained here, and the projection above is
taken from CI's row plus the 248.

A second shape retains far more and is not fixed here. When a loop builds the
new string by interpolating the old one directly, as in `"{st.body}{x}"`, the
arena peak grows a megabyte a pass on main (420,568,976 bytes at 400 passes)
and half that on the carrier. Every version of the string stays in a block in
use. Slicing the string first, as the fixture does, keeps the peak flat at
3 MB.

The mechanism is the repair. The interpolation lets the record-reuse analysis
write each new state into the old record's storage, and that record was built
before the loop, below its mark. At a stage the record survives but its new
string does not, so the walk repairs it: `k_repaired_settle` copies the string
into the arena and raises the mark over the copy, because a surviving node
points at it. The next pass writes a newer string into the same record, and the
settled copy is dead under the mark until the loop ends. A trace of the beat
carry shows the mark's block moving 48 bytes up at each of the 200 stages of a
400-pass run, with a megabyte more live beneath it each time. Raising the mark
is right for a node repaired once. What it lacks is a way to take back a
settled region when the same node is repaired again, and that is a separate
change.

CI's rows replace the projection. runbench reads 1,152,754,609, which is 174
instructions above the carrier's row rather than the 248 measured here.
Measured against main, the rows that end worse on this branch, the carrier's
changes included, are codegen_instructions_dev at 118,955,169,
codegen_instructions_release at 454,306,200, browser_run_instructions at
34,124,990, work_encodebench at 2,398,370,414, work_livebench at
1,474,871,090, work_oneshot at 13,224,195 and text at 4,835,550. Against the
carrier alone, the release row falls by 120,818 and the dev row rises by
3,943; every benchmark's runtime text grows 176 bytes, which is the spare
list's accounting and its drop path.

The carrier took main's retired `build` and CI's rows for it, and this branch
took the carrier in a merge commit. Each conflicted row is projected as this
fix's own CI delta laid on the carrier's CI row: codegen_instructions_dev lands
on 118,958,562, work_encodebench on 2,398,370,022, work_livebench on
1,474,871,748, work_oneshot on 13,224,265 and work_runbench on 1,152,754,602,
and `text` totals 4,829,278. CI's reading replaces each of them.

CI's rows replace the projection. codegen_instructions_dev lands on
118,939,055 and codegen_instructions_release on 454,517,107, the release tier
230,743 above the projection. work_encodebench lands on 2,398,369,553,
work_livebench on 1,474,871,531, work_oneshot on 13,223,698 and work_runbench
on 1,152,754,392. `text` totals 4,829,278, as projected.

## 2026-10-10 — a counting loop keeps every pass's key; copying keys out measured and declined

A loop that counts into a map it was handed, under a key it interpolates each
pass, never rewinds. `put m k (m[k] + 1)` is an in-place put and replaces the
count, so the key string is garbage once the put has found its slot. The beat
licence refuses the loop anyway, because a key the put does store would be
freed under the map by the next rewind. At 100,000 passes over ten keys the
arena peak is 4,194,304 bytes, 32 bytes a pass. A loop that builds a new string
value each pass into a map, `put m "body" "{m["body"]}{x}"`, keeps every
version the same way: 105,913,696 bytes of arena at 100 passes of a one-megabyte
string, 420,568,976 at 400, on main and on every branch in flight.

The fix tried was to let the put copy a key it inserts out of the arena, and
license the loop. The copy is registered with the map's pairs and freed when
they are, so a function that builds a map of fresh keys in a loop does not leak
across calls: peak RSS held at 10 MB from 2,000 to 200,000 calls. Inside a loop
that rewinds, a put with a string key calls the runtime instead of the emitter's
inline fast path, which would store the key uncopied. Routing every string-key
put to the runtime whenever any beat was live cost jsonbench 16.6% and runbench
8.2%; routing only the puts inside the newly licensed loop cost nothing
measurable on the instruction rows. The counting loop's arena peak fell to
1,048,576 bytes and stays there.

It is declined on memory. Licensing the loop moves its map's pairs out of the
arena into malloc'd storage, which the objective counts, and in the run program
nothing came back from the arena to pay for it: run_peak_bytes rose from
4,948,512 to 5,202,176, and welfare fell from 91.0318 to 91.0163. basket also
read 901,269 more instructions. A version worth trying keeps the pairs in the arena and
copies the key into the region below the loop's mark, which needs an allocation
point the arena does not have.

## 2026-10-10 — a record is reused in place only above the loop's mark

The retention at the end of the previous entry has a smaller fix than taking
back settled regions. Record reuse writes a new state into the old record's
storage, and inside a beat loop that storage can sit below the loop's mark,
where a rewind cannot reach it. `k_rec_reuse` now reuses a record in place only
when the record lies in the mark's own block, between the mark and the arena's
current position. Anywhere else it builds the record fresh, above the mark,
and the old one is left for the rewind. Outside a beat loop nothing changes.

The cost of building fresh is one record allocation a stage. The cost it
removes was a megabyte a pass: at 400 passes of the interpolating loop the
arena peak falls from 211 MB on the carrier to 3 MB.

`tests/golden/mem/an_interpolated_state_string_is_not_kept_under_the_mark`
runs the shape for 100 passes, `"{st.body}{st.count % 10}"` on a one-megabyte
string. Its arena_peak_bytes is 3,145,984; with the check removed it reads
54,529,792. It makes 213 allocations and evacuates 52,435,792 bytes, which is
the string copied at each stage and is the price of carrying it.

Four mem fixtures and the basket golden move, each by records that are now
built fresh where they were overwritten:

- `record_reuse_shape`: allocs 4 to 6, alloc_bytes 176 to 304, evac_allocs 3
  to 5, evac_bytes 80 to 144, survive_slots 8 to 4, sh_rec 64 to 192.
- `a_long_chain_answers_survival_from_its_index`: allocs 240,008 to 240,004,
  chain_finds 477,919 to 95,601, survive_slots 500,024 to 100,004, evac_bytes
  448 to 608, sh_rec 224 to 608.
- `a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap`: allocs 180,003
  to 180,002, alloc_bytes 12,160,400 to 12,160,432, evac_bytes 6,128 to 6,160,
  survive_slots 8 to 4, sh_rec 2,560,128 to 2,560,192.
- `an_evaluator_carrying_any_value_round_its_cycle_rewinds`: allocs 91,429 to
  91,445, alloc_bytes 4,114,384 to 4,115,408, sh_rec 2,560,192 to 2,561,216.
- `bench/cost_golden_basket.txt`: allocs 27,263 to 27,265, alloc_bytes
  7,522,129 to 7,522,257, evac_allocs 3 to 5, evac_bytes 192 to 256,
  survive_slots 4 to 0, sh_rec 130,048 to 130,176.

The chain fixture's chain_finds falling to a fifth is the walk no longer
following records that were rewritten under it.

Instructions were measured in one worktree against the spare-list branch this
one sits on. basket reads 60,732 more, livebench 602 more; runbench reads 392
fewer, encodebench 280 fewer and oneshot 644 fewer. They are projected into
`bench/instructions_golden.txt` from that branch's figures, and CI's rows
replace them.

CI read every instruction row exactly as projected. Three rows end worse than
main, the two branches below this one included: codegen_instructions_dev at
118,957,002, codegen_instructions_release at 454,309,037 and text at 4,836,670.
Against the spare fix alone the dev row rises 1,833 and the release row 2,837,
and each benchmark's runtime text grows 80 bytes, which is the check before a
reuse.

By the trend gate's keys, the rows this change worsens land at basket_allocs
27,265, basket_evac_allocs 5, basket_evac_bytes 256 and basket_sh_rec 130,176;
record_reuse_shape_allocs 6, record_reuse_shape_alloc_bytes 304,
record_reuse_shape_evac_allocs 5, record_reuse_shape_evac_bytes 144 and
record_reuse_shape_sh_rec 192; and, projected, work_encodebench 2,398,370,134,
work_livebench 1,474,871,692 and work_oneshot 13,223,551. Each record row is a
record built fresh above the mark where it used to be written below it.

The branch took the spare fix after that fix took the carrier, and each
conflicted row is projected as this branch's CI delta on the spare fix's row.
Measured against the spare fix, the rows that end worse are
codegen_instructions_dev on 118,960,395, codegen_instructions_release on
454,289,201, work_basket on 30,873,237, work_livebench on 1,474,872,350 and
`text` on 4,830,398, 80 bytes a binary for the mark check. basket_alloc_bytes
lands on 7,522,257. Three mem fixtures that keep a record across a cycle read
it as a fresh allocation where they used to reuse it.
a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap_alloc_bytes lands
on 12,160,432,
a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap_evac_bytes on 6,160
and a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap_sh_rec on
2,560,192. a_long_chain_answers_survival_from_its_index_alloc_bytes lands on
19,672,832, a_long_chain_answers_survival_from_its_index_chain_finds on 95,601,
a_long_chain_answers_survival_from_its_index_evac_bytes on 608 and
a_long_chain_answers_survival_from_its_index_sh_rec on 608.
an_evaluator_carrying_any_value_round_its_cycle_rewinds_alloc_bytes lands on
4,115,408, an_evaluator_carrying_any_value_round_its_cycle_rewinds_allocs on
91,445 and an_evaluator_carrying_any_value_round_its_cycle_rewinds_sh_rec on
2,561,216. CI's reading
replaces the projected rows.

CI's rows for this branch were read on the spare fix before k_b_adopt was
retired beneath it. With that retirement's deltas added, the rows land on
codegen_instructions_dev 118,935,145, codegen_instructions_release
454,454,756, work_basket 30,873,237, work_encodebench 2,398,370,771,
work_livebench 1,474,870,922, work_oneshot 13,224,076 and work_runbench
1,152,728,036. Against the spare fix the dev row rises 1,901, the release row 2,593, basket
60,732, encodebench 1,120 and oneshot 371, and livebench falls 679. With the
browser row the spare fix carries, welfare scores 91.0338407, 0.0000035 below
the floor the spare fix banked and inside the gate's 0.001 allowance, so the
floor stays where the spare fix left it.
`text` totals 4,825,470, 80 bytes a binary above the spare fix for the mark
check.

CI then read this branch on the spare fix with k_b_adopt retired, and those
rows replace the ones above. codegen_instructions_dev lands on 118,940,032,
codegen_instructions_release on 454,512,392, work_basket on 30,873,237,
work_encodebench on 2,398,369,266, work_livebench on 1,474,870,859,
work_oneshot on 13,224,090 and work_runbench on 1,152,729,429, and `text`
totals 4,825,438. Against the spare fix's CI rows, the dev row rises 2,514, the
release row 2,552, basket 60,732, oneshot 189 and runbench 686; encodebench
falls 756 and livebench 343. Welfare scores 91.0338012, 0.0000081 under the
91.0338093 the spare fix banks from its CI rows and inside the gate's 0.001
allowance, so the floor stays where the spare fix left it.

## 2026-10-11 — the first join makes its own builder, and `k_b_adopt` goes

A builder seed that is not a string reached its first join through
`k_b_adopt`, which rendered it and made the rendering the builder. The fold
join (kanso#1825) taught `k_concat_arr_mut` to do the same for any seed that
arrives with no room, and from then on the adoption had nothing left to
convert. The ratchet showed it: both rows that mutated the adoption stayed
green on the carrier, and were replaced by one row that restores the old
refusal in `k_concat_arr_mut`. This change removes the call, its declaration
and the runtime function.

Measured in this container against the carrier, runbench reads 26,076
instructions fewer, because the run program's builder loops no longer call
the adoption on their first join; its emitted code loses one call and two
lines. `codegen_instructions_dev` falls by 5,811, `codegen_instructions_release`
by 64,944 and `emit_instructions` by 479, and each runtime binary sheds 352
bytes of text. encodebench reads 98 more, livebench 70 more and oneshot 7
more, with no change to what they call. The front-end rows and every
allocation counter agree with the carrier. CI's rows replace these.

This change rides kanso#1839, the oversize-spare fix, rather than a pull
request of its own. On CI's rows the spare fix alone scores 91.0337007,
below the carrier's floor of 91.0337226, most of the gap in
`codegen_instructions_release`, which lands 109,925 above the carrier at
454,517,107. The adoption's retirement is projected onto those rows from the
deltas above, and the two together score 91.0338379. The browser tab compiles
with the same emitter, so browser_compile_instructions falls 7,415 to
390,029,358, measured here with the pinned rustc that builds
`docs/kanso.wasm`. With that row the floor this pull request banks is
91.0338442. Against the carrier, three rows still read worse:
codegen_instructions_release on 454,452,163, work_livebench on 1,474,871,601
and work_oneshot on 13,223,705, the release row from the spare fix's layout
and the two run rows 147 and 7 above the carrier. CI's rows for the pair
replace the projection.

CI's rows for the pair replace the projection. codegen_instructions_dev lands
on 118,937,518 and codegen_instructions_release on 454,509,840, the release row
57,677 above the projection. work_encodebench lands on 2,398,370,022,
work_livebench on 1,474,871,202, work_oneshot on 13,223,901 and work_runbench
on 1,152,728,743. runbench's text falls 32 bytes to 615,417, so `text` totals
4,824,318, and its emitted code is one call and two lines shorter than
projected; emitted_other_lines lands on 146,722, two below the carrier. Welfare
on these rows is 91.0338093, which is 0.0000867 above the
carrier's floor and 0.0000349 below the 91.0338442 banked from the projection.
The floor now records CI's figure. Banking the projection before CI had read
it was the error; a projected floor can only be wrong in a direction nobody
has checked.

## 2026-10-11 — the two sweeps print their own lists

Two specs checked CLAUDE.md's wording. `every_counter_gate_is_in_the_sweep`
read the word before "cost goldens," and `the_compile_sweep_names_every_compile_gate`
read one bullet for every compile gate's name. Each kept a fact in two places
and used a test to make them agree, which ties the instructions' prose to the
tree and blocks rewriting it (kanso#1846).

Both sweeps now take `--list`. `all_counters.sh --list` prints each vein and
its golden, and `all_compile.sh --list` prints each gate, both before building
anything. The two specs that read CLAUDE.md are replaced by
`the_sweep_lists_every_cost_golden` and `the_sweep_lists_every_compile_gate`,
which run the script with `--list` and compare its output with the files on
disk in both directions. Each went red with one entry dropped from the list it
reads, naming the golden or gate it had lost.

So the scripts are the single source for which veins and gates exist, and
CLAUDE.md no longer has to carry either list or a count.

## 2026-10-11 — the welfare verdict compares exactly, and kanso#1840's shape joins the run program

The welfare verdict passed any score within 0.001 of the floor, on either side.
Its comment said the margin was for compile peak bytes, which read 56 bytes
apart on linux and macos. That stopped applying when welfare began scoring the
committed goldens: both hosts read the same file. The script combines its terms
with `+`, `-`, `*` and `/` alone, and IEEE 754 rounds each of those exactly, so
two hosts score one tree to the same double. `math/round` appears only in the
history note `--set` writes. kanso#1820 fell 0.00099 and passed inside the
margin, which is what kanso#1845 reported.

The verdict now compares exactly. A score below the floor fails as a fall, and
a score above it fails as a rise nobody banked. `--set` writes the score through
`json/encode`, and a plain run reads it back to the same bits: a floor banked
on this branch held against its own tree with nothing to spare. Two specs in
`tests/a_welfare_fall_is_bought.rs` stage the floor a billionth of a point above
and below the recorded one. With the margin restored, both runs exited clean
and both specs went red.

`--set` still banks a fall smaller than 0.01. That path writes the new floor
and its reason into `bench/welfare_floor.json`, where a reviewer reads both,
and this change leaves it alone.

Main sat 0.0000081 below its floor when this was written, inside the margin.
kanso#1840 cost that much on the rows the objective reads, and what it bought
was in a shape the run program never ran. The corpus-first gavel of 2026-09-05
puts the shape into the corpus, measured on the compiler without the fix, and
the fix then scores against it. kanso#1840 had already merged, so the compiler
without it is main at kanso#1839, 30ffadf4, the tree kanso#1840 was cut from.

The new phase, `restate`, is the mem fixture
`an_interpolated_state_string_is_not_kept_under_the_mark` with its pass count
lifted into a parameter and its seed string cut from a megabyte to 64 KiB. At
128 passes, on kanso#1839's compiler, it raises the arena peak from 4,718,608
to 6,815,760, which is two more one-megabyte blocks and about one decoded
document, sized the way `machine` and `fold` were. It raises the program's
instructions from 1,152,728,396 to 1,166,079,984. Built alone, less the empty
program's 6,760,257, it costs 13,151,636, which is 1.13% of the program. The
header's table of port phases gains that row.

On main the arena peak stays at 4,718,608, and the program reads 1,170,030,652.
That is 3,950,668 more than kanso#1839's compiler spends on the same program,
so the fix costs instructions on this shape as well as saving memory. The
mechanism was not isolated.

Welfare's two run baselines are re-based the way the four port phases were, so
that adding the phase costs kanso#1839's compiler nothing. Each is multiplied
by the reading with the phase over the reading without it, both on kanso#1839's
compiler. run_instructions goes from 3,349,060,500 to 3,387,851,317, by
1,166,079,984 over 1,152,728,396. The run peak, which the objective reads as
the arena, held and permanent peaks summed, goes from 4,948,512 to 7,045,664
with the phase, so run_peak_bytes' baseline goes from 489,255,962 to
696,599,932. Both baselines are rounded up. The instruction pair was read in
this container. Both readings come from one compiler on one host, so the ratio
carries no difference between hosts that matters. The peak pair is counted and
is the same on every host.

The run vein moves by the phase's own work. run_alloc_bytes lands on
330,057,404, run_allocs on 1,771,562, run_beat_iters on 47,708,
run_bytes_malloc on 9,134, run_evac_allocs on 49,183, run_evac_bytes on
14,916,544, run_perm_allocs on 98, run_sh_rec on 1,508,784, run_sh_str on
17,048,976, run_str_scans on 1,023 and run_str_scan_bytes on 17,996,094. The
evacuated bytes are the string copied at each stage, which the mem fixture's
entry already names as the price of carrying it.

CI read the rest. work_runbench lands on 1,170,030,999, 347 above this
container's 1,170,030,652. The phase adds eight functions to the run program,
so its machine code grows to 618,633 bytes, which moves text to 4,828,574, and
its emitted code moves emitted_other_defines to 2,010, emitted_other_calls to
17,507, emitted_other_branches to 16,468 and emitted_other_lines to 147,279.
Welfare on CI's rows is 91.1114 against main's floor of 91.0338, and the floor
banks that.

`tests/the_run_program_carries_the_shapes_unchanged.rs` lists `restate` with
the four port phases that, like it, came from a fixture rather than from a
benchmark.

## 2026-10-11 — the interpreter row reads the same on every runner

kanso#1843 asked for `interp_instructions` to read one number on every CI host,
after two jobs at one source read 2,178,502,266 and 2,178,502,272 on
2026-09-15. Every cost-goldens job prints an `interp_sample` notice carrying
its CPU, the binary's hash and the row, so the question can be answered from
the annotations rather than by a new measurement.

The annotations of 144 jobs survive, from 2026-10-08 to this morning. They
hold 20 distinct rows, one for each interpreter source those jobs built. 19 of
the 20 were read on more than one CPU model, and none of them was read as two
numbers. The six models are AMD family 0x19 models 0x1 and 0x11, family 0x1a
model 0x2, and Intel family 0x6 models 0x6a, 0xad and 0xcf. 488,913,120 alone
was read by 45 jobs on all six.

Two pairs of rows sit close together, 488,869,908 against 488,869,940 and
568,708,684 against 568,709,639. Each value in both pairs was read on two or
more CPU models, and each step from one value to the next is a commit that
moved the golden: 4119f057 for the first pair and 3b981430 for the second. The
older jobs' annotations had expired, so the record starts eight days after the
2026-09-30 change that took the heap's address out of the interpreter's cache
keys.

So the row reads identically across runner hosts, which is what kanso#1843
asked for, and nothing needs excluding from the golden's header.

## 2026-10-11 — where the restate phase's extra instructions go

Main's compiler spends 3,950,668 more instructions on the run program with
`restate` than kanso#1839's compiler does. A per-function callgrind diff of the
two builds puts 3,870,494 of them in glibc's `memmove`, and every other
function moves by under 17,000. That is the stage evacuation copying each
pass's 64 KiB string below the loop's mark, which the 2026-10-10 entry on
record reuse names as the price of carrying it: run_evac_bytes reads
14,916,544 on main against 6,442,592 without the phase. Built alone, the phase
costs fewer instructions on main than on kanso#1839's compiler: the run program
with every other count set to zero reads 17,451,186 against 19,911,907. Why the
phase costs more in the full program and less alone was not looked into.

Handing the string's block down instead of copying it is not available: an
allocation gets a block of its own only above 1 MiB, so a 64 KiB string shares
a block with whatever else the stage allocated. Keeping survivors where they
were built would need a different region discipline, and nothing here builds
one.

## 2026-10-11 — the live log holds forty entries

CLAUDE.md says this file holds the last forty
entries, and nothing checked it. By 2026-10-10 the file held 526 entries in
25,301 lines. The pull request fixing kanso#1850
moved the older end by hand that day, and by the time this branch took main the
count had climbed back to 58.

`tests/the_live_log_holds_forty_entries.rs` now pins the count at exactly
forty, checks that no heading appears in both files, and checks that the
archive's last entry is no later than the live log's first.
`scripts/archive_log.sh` moves the oldest entries, unedited, from the top of
this file to the end of design/log/compiler-log-archive.md until forty remain.
A branch that appends an entry runs it in the same commit, and the spec's
failure message names the script and the first entry it will move.

The first run moved 18 entries. The two files together hold the same lines
before and after, compared as sorted multisets, and the line counts sum to the
same 102,212.

## 2026-10-11 — the specs stay one binary each

kanso#1847 asked whether folding the 228 spec files into a few test crates
would cut build and link time. Measured on main fd6167f7, release profile,
four cores: after `touch src/lib.rs`, `cargo test --release --no-run` takes
90.9 s, and the library alone is 35.5 s of it. The 230 test executables hold
487,797,788 bytes.

A prototype put every file in one crate as a `#[path]` module, with no spec
edited. It compiled, and the same rebuild took 68.5 s. Its one binary is
15,185,664 bytes. Run, it failed
`bind_chain_depth::the_measurement_sees_a_shape_that_nests`, which passes
alone. That spec reads its children's peak RSS from `getrusage`, and in one
process the peak includes every other spec's children. Separate binaries run
one after another, so no spec has shared a process with another file's spec
before. 151 of the 227 files spawn processes or read rusage.

Folding only the other 76 saves about 7 s and 160 MB. They would have to move
to a subdirectory to keep autotests, and other files name 29 of them by path,
including source comments, golden headers, two pages and four workflows. The
spec binaries already link with rust-lld, so the linker offers nothing either.
Neither fold is built, and the issue is closed as not planned.

## 2026-10-11 — the carry-repair spec had stopped reaching the repair

`tests/carry_repair.rs` was the regression test for kanso#823: a repair raised
a mark's pointer over the bytes it copied and left the mark's room behind. Its
program gathered four files of the repository's prose in a loop. With
`m->left = k_arena_left;` deleted from `k_repaired_settle`, the spec passed on
main, with those files and with fixed-size copies of them. A probe in
`k_repaired_settle` printed nothing: the program no longer reaches the repair.
Its check also lived in the rewind until kanso#1259 moved it to the push on
2026-09-05, where it reads the mark it has just written and cannot see a stale
one.

Three mem fixtures do reach the repair:
a_carried_value_written_into_an_older_node ten times,
a_repaired_node_below_the_mark_holds_tenure eight times and
a_node_repaired_by_an_inner_pop_outlives_the_outer_stage once. With the line
deleted, the last of them exits 1 and its stdout and counters change, so the
mem vein was already the real guard. The ratchet row `repaired_mark` now
deletes the line and expects that red, and the spec is gone.

## 2026-10-11 — gavel: work-ahead holds a pooled failure until the value is used

Clay ruled #1862, taking the recommendation: option 1 now, option 2 if
divergence turns out to matter.

The 2026-07-22 ruling made work-ahead invisible because pooled work has no
effects. That holds when the work succeeds. A pooled computation that fails or
never returns can show its failure before output the program as written
prints first: `bench/workahead/aswritten.kso` prints `delivery: made` after its
sleep and then runs `grind`, and a pooled `grind` that failed would raise its
error during the sleep, before that line.

**The ruling.** A pooled computation that fails stores its failure in the cell,
and the failure is raised at the cell's first use, where the program as
written would have raised it. For every program that terminates, the
transcript is identical to the one it would produce without work-ahead. A
pooled computation that never returns may still stall before output that the
program as written prints first. That case is accepted for v1, and a second
thread with its own arena (#1862's option 2) is the remedy if real programs
show it matters.

Owes, on #1856: a way to catch a runtime failure inside a pooled evaluation
and resume the scheduler, on every engine that pools (native `k_die` exits
today); a golden in which a pooled computation fails and the transcript
matches the program run without work-ahead, seen red with the failure raised
at the park; the page's work-ahead section saying what holds for a failure
and what holds for divergence.

## 2026-10-11 — gavel: a field may hold a list of `tie` references built from data

Clay ruled #1842, taking the recommendation as corrected on the issue.

The 2026-10-10 gavel "a field may hold a list of `tie` references" admitted a
list or map literal. A node whose links come from data cannot write a literal.
`list/map` is lazy (`pub fn map coll f` is `mapped f (iter coll)`), so
`list/map roads ref` in a field would store a record holding `ref`, and every
`ref` call would happen after `tie` had returned, where `resolve_tied` and
`k_tie_resolve` have no cell to swap.

**The ruling.** `list/to_list (list/map xs ref)`, and the same with
`(k -> ref k)`, written as a constructor's field, is admitted: every `ref` is
called inside the maker, and the field holds a list of cells the engines
already resolve. A bare `list/map` of references in that position is refused,
with a diagnostic naming `list/to_list`. The rule is unchanged: during making,
a reference is stored and never read.

Owes, on #1842: the check admits the forced form; an error golden for the bare
`list/map` with the new diagnostic; `a_graph_tied_from_a_map_of_edges` drops its
chain of `road` records; the comment in `lib/list/list.kso` where `tie` is
taught shows the data-sized form.

## 2026-10-11 — the big source files split by `include!`, not by module

kanso#1848 and kanso#1867 asked for codegen.rs, main.rs, eval.rs, check.rs and
lib.rs to be cut along their seams. The first cut moved codegen.rs's runtime
declarations into a submodule, `mod declares`. On CI that moved the browser
compile row by +60,813 instructions, the emit row by +873, and the compile and
entry rows by one each, and welfare fell by 0.000052. The release profile
builds sixteen codegen units and rustc partitions them by module, so a new
module changes which functions are compiled together and what gets inlined.
The 2026-08-25 gavel refuses a refactoring that costs welfare, however little.

`include!` keeps the moved items in the module they came from. Across
kanso#1873, kanso#1875, kanso#1878 and kanso#1879 the moved pieces are
codegen.rs's runtime declarations and dispatcher methods, main.rs's tail-call
rewrites, eval.rs's executor, builtins, values and run loop, check.rs's
`list/tie` checks, after-infer pass, arm and shadow checks, and lib.rs's
enumerable fusion, module path, imports and hoisting. Each file sits in a
directory named for the file that includes it. No golden moved, and CI's cost
rows on every head equal main's. eval.rs went from 6,742 lines to 3,950,
check.rs from 5,291 to 3,322 and lib.rs from 4,166 to 2,295.

Three things the move exposed: `cargo fmt` reaches only files named by `mod`,
so CI's format job now runs rustfmt on every file under a subdirectory of src/;
`scripts/diagnostic_coverage` listed `src/` alone and saw none of the
refusals that moved, until kanso#1875 made it descend; and four doc comments
sat above the wrong item, each left behind when a function was inserted
between a doc and the item it described.

**Declined: codegen.rs's call emission.** Moving `emit_call_full` through
`emit_call_rest`, 1,088 lines, into `src/codegen/calls.rs` left the native and
wasm instruction multisets identical to main's and moved
`browser_compile_instructions` from 390,029,358 to 390,029,410. Under the
2026-08-25 gavel it did not ship. The wasm module is 24 bytes larger. Cloud's
attribution is that panic locations embed the source path and a longer path
shifts the data after it, changing the alignment the metered `memcpy` and
`memcmp` loops see. That is unisolated. If it holds, the browser row is
reading a file's name, which the 2026-09-15 normalization rule says is
normalized or excluded. #1880 isolates it and, if it is confirmed,
normalizes the browser build before the split is retried.

## 2026-10-11 — STATUS.md and the ledger are deleted

This closes #1852. The 2026-10-10 gavel made GitHub issues the tracker and left
STATUS.md and design/pending-gavels.md as pointers until the two specs that
read them retired. Both have retired, and nothing else reads either file, so
both are deleted. Comments in four specs, two scripts and ci.yml still mention
them; those are cloud's and are cosmetic.

The ledger's Parked section was the one thing in it with no other home. It is
recorded here, unedited in substance:

- `<<` labels: walls cover staircases; revive on real DAG demand.
- Labeled nameless patterns: parked 2026-08-19, needing a fresh look against
  the post-24 language. Group headers stay behind it.
- dot-absorbs-`>>`: argued no, because it erases the visible then/bind split.
- Postfix index on `)`: `(sort xs)[1]` stays illegal; bind, then index.
- `;` as an inline separator: the borrow if inline groups are ever demanded.
- `&` as bitwise: orthogonal, someday.
- `serve` and processes: the executor-loop primitive and the next design
  campaign, where three investigations already end. The July reification form
  died with gavel 1, and the campaign starts from the three combinators.
- Hako tag-signing and checksum policy: parked in design/hako.md.
- Monorepo hakos: parked in design/hako.md.
- The survivor cap of four times the block threshold: the multiplier is a
  judgment call, and the principle is in the log.

A parked item becomes a `needs-clay` issue when a real case arrives.

## 2026-10-12 — a test that runs out of stack fails alone

`kanso test` ran its tests on the main thread, whose stack is 8 MiB. The
interpreter's depth limit is sized for the 1 GiB thread `run --interp` starts,
so a test that recursed too far overflowed Rust's stack before the limit
reported it, and the process aborted with every later test unreported. The lox
port met this from a recursion 700 calls deep. The runner now starts the same
deep thread, and the test fails with the interpreter's diagnostic, "the
program ran out of stack".

`tests/test_verb.rs` runs `deep_test.kso`, one shallow test and one that
recurses a million deep, and expects the first to pass, the second to fail and
the run to exit 1. The ratchet row "a test run on the main stack" gives the
thread 8 MiB and the spec sees the abort.

This is part of the recursion item on #1892. The native side of that item is
open: a deep enough recursion through the lox port's trie write still ends in
a segmentation fault with no message.

## 2026-10-12 — an operator answers what its arms answer

The ray tracer port's release build printed a few rows of its image and then
stopped with "no overload of `-` matches these arguments". The interpreter and
the dev build drew the whole image. Linking the release IR at `-O0` drew it
correctly, and so did the full release link with every `llvm.assume` removed.
Bisecting the 148 assumes left one: after reading a hit record's `p`, release
assumed the value carried the float tag.

The cause was in inference. Native's `emit_binop` and the interpreter send a
record on either side of an operator to the program's own arms for that
operator, and inference read `a + b` as arithmetic alone. It never widened the
arms' parameters and never added what they return. The ray tracer's `at` is
`origin + direction * t`, and `direction` is sometimes a float drawn from the
generator, so inference concluded that `at` answers a float, and so does the
`p` field that holds its result.

Inference now treats an operator with a record operand as a call to the
operator's group when the program declares one: the arms' parameters widen
with the operands, and the arms' answers join the operator's. The micro
fixture `an_operator_answers_what_its_arms_answer` is the smallest program
that showed it. Without the fix its release build prints a `v2` through the
number arm. The ratchet row "an operator answering arithmetic alone" removes
the fix and expects the micro corpus's release build to fail.

Inference does more work for each operator with a record operand, and every
compile row rose on CI: compile_instructions to 25,761,215 (+46,112),
entry_instructions to 85,207,562 (+162,685), library_instructions to
85,760,222 (+161,046), emit_instructions to 7,016,409 (+2,717) and
browser_compile_instructions to 391,974,060 (+1,944,702). The welfare floor
drops from 91.1114 to 91.1094 under the rule that a change making the engines
agree is not priced.

This is the ray tracer item on #1892.

## 2026-10-12 — a runtime diagnostic names the err it was handed

The inflate port destructured a value that was sometimes an err, and native
reported "the program ran out of stack" where the interpreter said "cannot
destructure err ... as `pair`". There was no recursion. `k_die_destructure`
rendered its value with `k_render`, which hands an err back unrendered so that
it can ride through a program, and then read the err as a string. The read
faulted, and the parent reports any segmentation fault as stack exhaustion.

The diagnostics that name a value they were handed now render through
`k_shown`, which renders an err in full: the destructure and keyed-read
refusals, the field-read refusals and `k_die_value` and `k_die_got`. The
interpreter's message is the one both engines print.

`tests/golden/runtime/an_err_cannot_be_destructured` is the port's shape: a
group that answers a record or an err, its answer bound with `pair a b = p`.
The ratchet row "a diagnostic handed an unrendered err" puts the plain render
back, and native's stderr reads out of stack again.

The first version of `k_shown` called `k_render_at` with the err flag set.
That gave `k_render_at` six callers outside itself where it had one,
`k_render`, and clang stopped inlining it there, so every print paid for a
call: basket rose 111,561 instructions and runbench 172,212 on this
container. `k_shown` now unwraps a thunk or a subtype itself, renders an err
through `k_render_whole` and hands anything else to `k_render`; it is cold and
out of line, so the functions that call it on their refusal path stay the size
they were. Measured the same way, basket is 14 instructions from main and
runbench 315.

The out-of-line `k_shown` is about 80 bytes of `.text` in every native
binary, and the rows that count a whole program carry it. On CI's rows,
`text` went from 4,828,574 to 4,829,694, `codegen_instructions_dev` to
118,944,914, `work_encodebench` to 2,398,369,910, `work_livebench` to
1,474,870,950 and `work_runbench` to 1,170,031,314. `work_oneshot` fell to
13,223,887.

## 2026-10-12 — a built binary names its own stack overflow

A recursion deeper than the stack ends in a fault on the guard page below it.
`kanso run` reports a child that died that way as having run out of stack, but
a binary run directly died with the bare signal and printed nothing. The lox
port is run directly, and a Lox recursion 20,000 calls deep ended in
"Segmentation fault" with exit 139 on this tree.

The runtime now installs a SIGSEGV handler at start-up, on a 64 KiB stack of
its own because the program's is the one that ran out. A fault whose address
lies between the top of the main stack and one MiB past its limit prints the
interpreter's diagnostic, "the program ran out of stack", and exits 1. Any
other fault restores the default handler and faults again, so a wild pointer
still dies by the signal. With no stack limit there is no floor to measure
from, and the handler is not installed. The wasm runtime has no signals and is
left alone.

`tests/a_built_binary_names_its_stack_overflow.rs` builds a recursion a
million deep that no engine can turn into a loop, runs the binary directly and
reads its stderr and exit status. The ratchet row "a binary that does not watch
its stack" removes the installation and the spec reads an empty stderr.

This closes the native half of the recursion item on #1892. The depth itself
still differs by engine: the interpreter stops at 10,000 calls and native at
whatever the stack holds.

Installing the handler costs every native program the same start-up work: a
`getrlimit`, a `sigaltstack` and a `sigaction`, about 3,300 instructions, and
368 bytes of `.text`. On CI's rows for the tree merged with #1899, `text`
went to 4,834,846 and `codegen_instructions_dev` to 119,002,829. The work rows
moved by that constant: `work_basket` to 30,876,488, `work_deepbench` to
393,579,644, `work_digestbench` to 5,603,118, `work_encodebench` to
2,398,373,226, `work_escapebench` to 39,049,158, `work_indexbench` to
2,462,486, `work_jsonbench` to 860,562,984, `work_livebench` to 1,474,875,071,
`work_oneshot` to 13,226,930, `work_pendbench` to 190,054,350,
`work_readbench` to 4,580,966, `work_runbench` to 1,170,035,080,
`work_scanbench` to 273,523 and `work_widebench` to 28,178,509.
`codegen_instructions_release` fell to 454,179,413, which outweighs the rest,
and the rise is banked.

## 2026-10-12 — stdout reaches its reader, and a closed pipe stops every engine quietly

Two runtime items on #1892 were about stdout.

A reader that stops early, as `head` does, closes the pipe the program is
writing to. A built binary is ended by SIGPIPE and says nothing. The
interpreter met the same write as an error and panicked with two Rust
backtraces, because Rust sets SIGPIPE to ignored before `main`, and
`kanso play` reported a native program's end as "the program was ended by
signal 13". kanso now restores the default disposition at start-up, and when
the program it ran was ended by SIGPIPE the driver ends the same way, silently.
`tests/a_closed_pipe_stops_every_engine_quietly.rs` reads one line from each
engine and drops the pipe.

A native program writes stdout through stdio's buffer, and stderr goes out
unbuffered. Text printed before a write to stderr reached a shared pipe after
it, which the lox port saw on its error path, and a program killed while it
waited lost everything it had printed, which is how the httpd port's access
log disappeared. The interpreter does neither. The runtime now empties the
buffer before anything goes to stderr, before it reads stdin or a connection,
before it sleeps, and before it starts a child. Every runtime diagnostic opens
with the error colour from `k_c_err`, so the flush there puts a program's
output ahead of any error it ends with. `tests/stdout_reaches_its_reader.rs`
pins the three cases.

The rows moved by small amounts, read off CI. The signal call at start-up
costs 123 instructions in every compile: compile_instructions lands on
25,761,338, entry_instructions on 85,207,685, library_instructions on
85,760,345 and startup_instructions on 52,519. The flushes are new runtime
code. On the tree merged with #1899, codegen_instructions_dev lands on
118,956,771, work_encodebench on 2,398,369,956, work_livebench on
1,474,871,792, work_oneshot on 13,224,134 and work_runbench on 1,170,031,874,
and browser_run_instructions was 34,125,823 before the merge. Release codegen
falls to 454,318,432, and the other benches and the text sizes fall a little
with the runtime's new layout.

The macOS runner failed the interpreter's half of the closed-pipe spec. With
SIGPIPE's default put back, the interpreter's thread there still met the
write to a closed pipe as an error, and `println!` panicked on it. The
executor's `print` and `write` now write to stdout themselves and turn a
broken pipe into SIGPIPE. On Linux the signal arrives first and that path
does not run; with the start-up reset removed, the interpreter still stops
quietly through it, and the ratchet row goes red on the native run instead,
whose driver cannot re-raise an ignored signal.
Writing through a stdout lock with an error arm cost the interpreted corpus
688 instructions on CI's row: `interp_instructions` went to 488,792,803.
On the tree merged with #1900, CI's rows put codegen_instructions_dev at
119,012,401, codegen_instructions_release at 454,260,490 and work_runbench at
1,170,035,058, and every bench's text is 1,744 bytes smaller. The welfare
floor drops to 91.10933 under the rule that a change making the engines agree
is not priced.

## 2026-10-12 — a lambda called in tail position costs no stack

The toml port kept `put` in place by passing a continuation: each insertion
step took a `then` and called it last. A 300 KB document then ran the
interpreter out of stack, and the dev build died with a segmentation fault.
Appendix A says a tail call costs no stack at all; it held for calls to named
functions and arms and failed for a call to a function value.

The interpreter's tail position already handed a named group's call back to
the dispatcher's loop. A call to a closure now comes back the same way, as
`Flow::Closure`, and runs in whichever loop called the body: the dispatcher's,
or a loop in `call_closure` for a lambda called from outside a tail.

Native code calls a closure through `k_call{n}`, which calls the lambda's
wrapper, so every step held two frames. A closure now also carries a pointer to
the lambda's own body, which is on the tailcc convention. A call to a local
function value in tail position becomes a musttail jump to `k_tcall{n}`, which
checks that it holds a closure of the right arity with no failing argument
and a body, then jumps to that body. Anything else (a closure the runtime
built, a partial, a reference to a group) goes to `k_call{n}` as before. The
arity and capture count became 32-bit fields so the pointer fits in the 32
bytes the closure already took.

Release narrows the convention to functions on a musttail edge, and the jump
through a pointer names no symbol, so the narrowing would have stripped it and
LLVM refused the module. Every lambda keeps the convention in a module that
has the jump, and the jump keeps it too.

`tests/golden/micro/a_lambda_called_in_tail_position_costs_no_stack` runs
100,000 steps of the port's loop on every engine. Before the fix the
interpreter reported running out of stack and the dev build exited 139. The
ratchet rows "a lambda called as a nested call" and "a lambda called without
a jump" remove each half and expect the micro corpus to fail.

The clang-to-wasm32 layout harness leaves the fixture out: by its own header
it compiles a call returning a `KValue` as an ordinary call, and a loop that
goes through a pointer cannot be inlined into one. The page's translator makes
the jump a tail call, and `ir_to_wasm` runs the fixture.

CI's rows, on the tree carrying #1896: codegen_instructions_release rises
4.7% to 475,481,653, because every lambda in a module with the jump keeps the
tail convention. interp_instructions rises 1.4% to 495,424,970,
work_runbench 0.17% to 1,171,985,904 and work_deepbench 2.5% to 403,499,508;
work_encodebench falls 0.49% to 2,386,714,424. The emitter writes fewer lines
(runbench's IR 51,577 to 50,505) and spends 1.1% more doing it,
emit_instructions 7,090,315. The three compile rows fall about 0.13%. The
welfare floor drops to 91.08971 under the rule that building the language is
priced at what it costs.

This is the toml item on #1892.
