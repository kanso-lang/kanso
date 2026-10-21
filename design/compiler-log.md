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


## 2026-10-14 — An import reads only what the program names

A program that imported a std module lexed, parsed, inferred and checked all
of it (#1952). The first attempt, recorded on the issue, set unreached
functions aside for the merged check only, and its reach walk cost more than
the check it saved. This one works before the lexer: a shipped module's `fn`
declarations whose names no reached text spells are blanked out of the
source.

Reach is by word. The roots are every word in the program's own files and
five names the compiler calls without the program spelling them: `fold`,
`bump`, `file_under`, `to_string` and `tie`. Each declaration kept adds its
own words, over a module's files together, and a name kept keeps every arm
of it. A blanked line stays an empty line, so every span points where it
did. Blank-line policy and the unused-import check are skipped on a pruned
file; `tests/every_shipped_module_checks_clean.rs` still checks each module
whole.

When the pruned compile fails, the compile runs again on the whole library
and that run reports, so no diagnostic changes. A debug build panics when the
whole library accepts a program the pruned one refused, and the suite ran
clean under it. The one fixture that needed an unreached function,
`list/spread` in a codegen unit test, now names it in a comment.

`tests/an_import_reads_what_the_program_names.rs` compiles
`print (text/uppercase "kanso")`. On main the program held 48 declarations,
`text/pad_start` among them, and inference visited 627 expressions. It now
holds 12 and visits 224.

Rows, from CI: `compile_instructions` 32,770,085 -> 22,632,999,
`entry_instructions` 104,020,991 -> 74,989,410, `library_instructions`
103,487,067 -> 74,674,363, `emit_instructions` 7,704,002 -> 5,438,857,
`compile_allocs` 18,250 -> 12,160, `compile_peak_bytes` 804,635 -> 552,167,
`front_end_visits` 9,242 -> 6,363, `codegen_instructions_dev` 119,402,816 ->
112,581,257, `codegen_instructions_release` 476,979,642 -> 441,727,359,
`interp_instructions` 496,124,589 -> 492,626,313, `interp_allocs` 592,648 ->
584,659, `interp_peak_bytes` 807,370 -> 590,970,
`browser_compile_instructions` 441,097,975 -> 381,571,779,
`browser_compile_peak_bytes` 731,449 -> 643,478 and
`browser_run_instructions` 34,183,924 -> 34,181,240. The module sample in
`bench/compile_golden_modules.txt` reads visits 3,666 -> 1,368, rounds 8 ->
4 and lines 1,288 -> 1,106.

Six benchmarks emit less code, because the emitter still wrote some std code
nothing called: encodebench, widebench, deepbench and digestbench lose 157
lines each, scanbench 30 and runbench 32, and their machine code shrinks with
them. Three runtime rows move with the smaller binaries: `work_deepbench`
403,499,516 -> 403,115,516, `work_scanbench` 273,717 -> 273,812 and
`work_runbench` 1,177,696,285 -> 1,177,696,572. The two rises are 95 and 287 instructions on
programs whose work did not change, so they are layout. Every allocation
counter holds.

The ratchet's row "a door matched by the prefix of its name" proved nothing
on this branch, and it proves nothing on main either: with the mutation
applied, `micro_corpus_agrees_across_engines` stays green at dfc9fe2f. The
mutation hands the door's convention to `k_b_utf8_slice` and
`k_b_utf8_slice_raw`, and since a slice past the end answers `none` a program
reaches them only through the JSON decoder's `string_ok`, for a string with
no escape in it. Every decoded string in the micro corpus had an escape.
`a_plain_decoded_string_is_cut_in_place` decodes three plain strings; under
the mutation the release build prints `["" "" ""]` and the row goes red.

Lazy parsing by declaration, the larger fix #1952 names, has not been built. This
change narrows what is read; it does not make one declaration cheaper to
read.

## 2026-10-14 — An Apple release build keeps its wide arms through the link

kanso#1951 kept `tailcc` on arm64 functions whose arguments overflow the
eight argument registers, and marked them so the backend makes no sibling
call there (`wide_tailcc_keeps_its_jumps`). Linux arm64 ran correctly. An
Apple arm64 host did not: on the macOS job,
`a_spilling_tail_call_named_with_a_mark` printed nothing, though the same IR
ran under qemu for linux arm64. So Apple kept `narrow_tailcc`, which gives a
wide function the C convention. Each hop through one then spends a frame, and
the three tests that recurse deeply through a wide arm were gated off on
Apple: `a_self_call_through_a_thirteen_word_arm_runs_in_one_frame`,
`a_cycle_through_a_twelve_word_arm_runs_in_one_frame` and
`an_object_of_300_000_keys_decodes` (#1912).

The cause is ld64's LTO. It runs LLVM's default pipeline over the linked
module, and that pipeline includes dead-argument elimination. Once LTO
internalizes a wide `tailcc` function, the pass is free to drop the arguments
it never reads. On the macOS job it cut `reached?_5_w` in the mark test to one
argument and a plain `ret`, while its caller still ran `sub sp, sp, #0x10`
after the call for the stack arguments it expected the callee to pop. The stack
pointer came back sixteen bytes off. The Linux link takes the pass out already
(`LTO_O3_WITHOUT_DEADARGELIM`, since kanso#1705), which is why the same IR was
fine there.

A function named in `@llvm.used` is never internalized, so the pass leaves its
signature alone. That custom pipeline applies only when clang is 18 or 19 and
lld takes the link, so every arm64 release build now marks its wide functions as
before and also lists each `tailcc` definition with more argument words than
the registers hold in `@llvm.used`, in the `llvm.metadata` section. A program
with no such function gets no global. Two unit tests in `tail_calls.rs` pin the
list: only a function past the registers is named, and a program without one
names none. The three tests above run ungated, and on the macOS job all three
pass.

`narrow_tailcc` and its helper `called_symbol` are gone, and the
tail-call-given-up mutation now strips both host arms. No row moved. The
change is confined to the arm64 host arm, and every Linux row the branch's CI
read matched main's goldens, `compile_instructions` included.

## 2026-10-14 — Work bound ahead of the first sleep moves into it

The 05:25 comment on #1856 measured a gap in the first cut of work-ahead. A
program that binds its work as a plain value before the sleep,

```
total = spin 150000000 0
sleep 2000 .> (_ -> print "delivery: made") .> (_ -> print "report: {total}")
```

took 3.01 s against 2.01 s for the rig, which binds the same work as a chain.
An entry binding is computed at its own statement, so the work ran before the
sleep began and the pool had nothing left to force.

`src/ahead.rs` now moves such a binding into the park. The move applies when
every statement ahead of the first sleep is a binding, since a binding performs
no effect and the sleep is then the first thing the program does that can be
seen. A binding qualifies when its value is closed (a literal, or a call of the
program's own functions on closed arguments) and no other binder in the entry
uses its name. It becomes a constant named `ahead~moved~N`, its reads in the
statements after it read the constant, and the pool forces these constants
first, in the order they were written. A read of a moved binding counts as
closed, so a later binding built from one moves too, and so does a call after
the sleep whose arguments name only moved bindings: `spin n 0`, with
`n = 150000000` bound ahead of the sleep, is lifted like `spin 150000000 0`.

The 2026-10-11 gavel on #1862 says a pooled failure is raised where the
program as written raises it. For a moved binding that place is the park,
because nothing printed between the binding and the sleep. The pool reads a
moved constant through `k_ahead_try` like any other; when the try reports a
death, the pool reads it once more outside the try, so the failure is raised
there with its message on stderr. The work is pure, so the second read meets
the same failure. A constant lifted from a continuation keeps its old
treatment: the pool stops and the failure comes where the value is read.

On this container the program above went from 3.80 s to 2.08 s under
`kanso play`, compile included, with the same two lines printed; the `spin n 0`
form went from 4.04 s to 2.09 s.
`tests/golden/mem/work_ahead_moves_a_binding_into_the_park` pins
`ahead_forced=1`, `tests/golden/mem/work_ahead_reads_a_moved_binding` pins
`ahead_forced=2` for the moved count and the call that reads it, and `a_moved_binding_fails_at_the_park` in
`tests/work_ahead.rs` pins a moved binding that overflows the stack: nothing on
stdout and the overflow on stderr, the interpreter's transcript. Without the
second read the native build printed `first` before the overflow. Three ratchet
rows cover the move, the second read, and a moved binding read as closed.

Two cleanups ride along. The mutable child walk was written twice, in
`src/inline.rs` and `src/lib/hoist.rs`; it is now `for_each_child_mut` in
lib.rs beside `for_each_child`. Taking its callback as `&mut dyn FnMut`, as
hoist's copy did, cost the tab's compile of its corpus 2,229,038 more wasm
instructions on this container than the two private copies, because the copy
in inline.rs had been inlined into its two callers and devirtualized there.
The shared walk takes its callback as a type parameter instead, as
`for_each_child` does. A pattern's binders were listed twice, by
codegen's `pattern_names` and trmc's `bound_names`; both call
`Pattern::each_name` now, and trmc's list gains the as-pattern's name it used
to leave out.

CI moved six rows. `browser_compile_instructions` went from 381,571,779 to
382,410,206, up 838,427, the same reading this container gave. A bisect here
put 305,579 of that on the shared walk, 32,693 on `each_name` and 500,155 on
the move, of which the move's codegen change accounts for 3,436. The tab's
corpus never sleeps, so most of the rest is how rustc divided the wasm build
into codegen units. The native front end rose with it: `entry_instructions`
74,989,410 -> 75,110,529, `library_instructions` 74,674,363 -> 74,797,007,
`compile_instructions` 22,632,999 -> 22,665,231. `interp_instructions` fell
756 to 492,625,557 and `emit_instructions` fell 9 to 5,438,848. No runtime row
moved. Welfare fell 0.00094 to 91.52438, and the floor is lowered by exactly
that, since this builds a ruled part of the language.

Still not built for #1856: an entry with more than one effect line, which
never pools (the next entry corrects this line), calls whose arguments name
locals that cannot move (a lambda's parameter, a value read from io), which
need the (site, args) pool, and the cost gate on admission.

## 2026-10-14 — The pool takes only work

This is the grain piece of #1856. The pool reads each constant inside
`k_ahead_try`, whose `sigsetjmp` saves the signal mask with a system call. A
literal bound ahead of the first sleep moved into a constant like any closed
binding, so the pool made that call to build a literal. `ahead::admits` now
lets a constant into the pool only when its value calls one of the
program's functions, directly or inside a string or list it builds. That is
the test the lazy cost gate puts to a cell (`demand::expensive`). The literal
still moves, because a call lifted after the sleep may name it, and it is
built where it is first read.

`tests/golden/mem/work_ahead_leaves_a_literal_where_it_is` binds a literal
and a call ahead of the sleep, and `ahead_forced` reads 1 where main read 2.
`work_ahead_reads_a_moved_binding` goes from 2 to 1 for the same reason. The
ratchet row "a literal handed to the pool" admits every constant and turns
the first golden red.

The previous entry listed "work bound after a statement with an effect" as
unbuilt. No program can be written that way. The parser refuses a binding
after an effect line, joins a body's effect lines into one group, and runs
every binding before the group starts. The runtime runs the pool only at a
sleep outside a group, so an entry with two effect lines never pools at all.
`tests/golden/workahead/a_binding_before_a_print_fails_before_it` pins the
transcript of such an entry whose binding fails: nothing on stdout, the
overflow on stderr. A ratchet mutation that let the park be any sleep was
tried and left the fixture green, because the group hides the sleep from
`ahead` before the search runs, so that row was dropped.

CI moved five rows, all of them compile cost. `browser_compile_instructions`
fell 255,236 to 382,154,970, the same reading this container gave. The tab's
corpus never sleeps and the tab compiles the pool out, so the pool decides
nothing there; nothing in this change isolates the drop further.
`compile_instructions` fell 15 to 22,665,216, `entry_instructions` 25 to
75,110,504 and `library_instructions` 25 to 74,796,982. `emit_instructions`
rose 72 to 5,438,920; the emitter now passes the program's declarations
through `admits`, the one change on that path. No runtime
row moved. Welfare rose 0.00022 to 91.52460, and the floor is banked there.

Still not built for #1856: pooling inside an effect group, and calls whose
arguments name locals that cannot move, which need the (site, args) pool.

## 2026-10-14 — A group of effect lines pools while it waits

#1856's work-ahead pool ran only at a bare sleep, and a body with two or more
effect lines has none: the parser joins the lines into one group, and the
scheduler runs a sleep inside it by parking that side. So `print "start"`
followed by a sleep with work behind it never pooled.

The scheduler now runs the pool at the group's first idle point, the first
time every side is waiting on a timer, and the wall-credit it already keeps
counts the pool's time against that wait. Which work runs there is fixed at
compile time and the idle point is a fact of the schedule, never of the
clock, so the transcript and the counters are the same on every run. On the
compiler side, `ahead` reads each side of the group that begins with a sleep
the way it reads a lone statement. A failure in the pool is dropped and
raised where the value is read, as before, so a program in a group prints
the transcript it prints without the pool. Bindings still move only when the
park is the body's single effect line.

`print "start"` and a 2-second sleep followed by a report of `spin 150000000
0`, run with `kanso play` on this container: 3.80 s and 3.96 s on main, 2.08 s
and 2.01 s on this branch, with the same output.

Pinned by:
- `tests/golden/mem/work_ahead_forces_inside_a_group`: `ahead_forced=1`, where
  main printed no work-ahead counters at all;
- `tests/golden/workahead/a_pooled_overflow_in_a_group_fails_where_it_is_read`:
  "start", "first", then the overflow, on both engines;
- ratchet rows "a group that never pools" and "a group's side left unread",
  each seen red on the new golden.

`k_schedule` grew 16 bytes on CI, and every benchmark in
`bench/text_golden.txt` grew by the same 16, so `text` went from 4,878,382 to
4,878,606. `browser_compile_instructions`
went from 382,154,970 to 382,155,032, though the tab compiles the pool out.
The front end rose: `compile_instructions` 22,665,216 -> 22,679,084,
`entry_instructions` 75,110,504 -> 75,152,899, `library_instructions`
74,796,982 -> 74,839,906, `emit_instructions` 5,438,920 -> 5,440,488 and
`interp_instructions` 492,625,557 -> 492,635,997. Codegen rose 967 at dev,
`codegen_instructions_dev` 112,581,257 -> 112,582,224, and 1,390 at release,
`codegen_instructions_release` 441,727,359 -> 441,728,749.

Three of the four CI runs landed on Intel parts, and the fourth on the AMD
part `bench/instructions_golden.txt` is recorded on. It moved four work rows:
`work_encodebench` 2,387,084,547 -> 2,387,083,945, `work_oneshot` 13,207,833
-> 13,207,567, `work_livebench` 1,474,835,144 -> 1,474,835,683 and
`work_runbench` 1,177,696,572 -> 1,177,697,090. This container, an Intel part, read the same
four differences between main and this branch built from the same inputs.
No benchmark sleeps, so no pooled work runs in them, and nothing here
isolates where the difference comes from. Welfare fell 0.00009 to 91.52451,
and the floor is lowered by exactly that, since this builds a ruled part of
#1856.

## 2026-10-14 — A call on a function's locals is pooled as a cell

#1856's pool took only constants: calls whose arguments name nothing local,
lifted out of the entry. A function that sleeps and then reports on its own
parameter had no constant to offer, so its work ran after the wait:

```
fn report n
  sleep 2000
    .> (_ -> print "delivery: made")
    .> (_ -> print "report: {spin (150000000 + n) 0}")
```

This is the "Discovery" piece of the ruling, the (site, args) pool. A
function whose result is a chain beginning with a sleep, and which is the
only arm of its name and count, has each certain call on its locals bound
ahead of the chain as a cell. `emit_cell` already builds a cell from a site
and the values it captures; `ahead::pool` names the binding, `demand` makes
it a cell whatever its uses, and the chain's sleep carries the cells in its
descriptor. The runtime queues them when that sleep parks and forces them, in
order, under the same try as the constants. A chain that is never run parks
nowhere, so its cells are made and never forced, and no work runs early that
the program would not have run. The interpreter does not pool.

A cell the pool forces is not a lazy binding the program has, and the thunk
counters are what both engines must agree on, so `k_thunk_new_pooled` builds
it with a flag those counters skip. `ahead_forced` counts it instead.

The program above, run with `kanso play` on this container: 3.87 to 4.07 s on
main, 2.01 s on this branch, with the same output. Built and run directly,
the dev binary takes 3.95 s against 2.00 s, and the release binary 2.20 s
against 2.00 s.

Pinned by:
- `tests/golden/mem/work_ahead_pools_a_call_on_a_local`: `ahead_forced=1`,
  and the thunk counters at zero on both engines;
- `tests/golden/mem/work_ahead_leaves_a_chain_never_run`: a chain built and
  never run, `ahead_forced=0`;
- `tests/golden/workahead/a_pooled_cell_fails_where_it_is_read`: the
  overflow is raised after "first", on both engines;
- five ratchet rows, each seen red.

`a_group_that_never_pools.sh` anchored on a comment and a call this change
rewrote, so it is rewritten to the new ones; every other mutation still
applies.

Still not built for #1856: a lambda's body, which has no statements to bind a
cell in; a function with more than one arm of its name and count; and the cost
gate's view of a pooled call, which admits any call of the program's own.

CI's rows for the branch. The runtime rows were read before main's IR-scan
move was merged in, and the compiler's own rows after kanso#1976 was. The
runtime grew by the queue and its drain, so every benchmark's machine code
rose by 2,192 bytes: jsonbench text=336,265, encodebench text=345,129,
oneshot text=351,145, basket text=313,289, widebench text=354,393,
deepbench text=288,025, escapebench text=279,113, pendbench text=296,841,
indexbench text=277,625, scanbench text=462,073, digestbench text=318,537,
readbench text=279,273, livebench text=354,889, runbench text=652,697, and
the `text` total 4,878,606 -> 4,909,294. Work moved on five:
`work_encodebench` 2,387,083,945 -> 2,387,084,099, `work_oneshot` 13,207,567
-> 13,207,693, `work_pendbench` 191,674,024 -> 191,674,224, `work_livebench`
1,474,835,683 -> 1,474,835,606 and `work_runbench` 1,177,697,090 ->
1,177,696,062.
The compiler's own rows: `compile_instructions` 22,664,963 -> 22,595,494,
`entry_instructions` 75,111,766 -> 74,872,489, `library_instructions`
74,798,192 -> 74,559,617, `emit_instructions` 5,439,036 -> 5,449,223,
`interp_instructions` 492,629,961 -> 492,636,904,
`codegen_instructions_dev` 112,582,224 -> 112,677,448 and
`codegen_instructions_release` 441,728,749 -> 441,885,670. The tab, read
here on rustc 1.98.1 after the merge: `browser_compile_instructions`
382,297,788 -> 381,261,098, `browser_compile_peak_bytes` 643,478 -> 643,605
and `browser_run_instructions` 34,181,240 -> 34,181,355. Welfare rises, and
the rise is banked here.

## 2026-10-14 — Neither verb sends a reader to the other

The 2026-10-14 audit of #1893 found one file shape that both running verbs
refuse while pointing at each other. A file that declares a function, runs a
statement and imports a neighbour is a library to `kanso run`, which
suggested `kanso play`; `kanso play` takes the stdlib and nothing else, and
suggested `kanso run`. Both refusals now give the route that runs the file:
move the statements into a main.kso beside it, mark the definitions they use
`pub`, and run the directory. A play file of statements alone that imports a
neighbour is still sent to `kanso run`, which takes it.

`tests/golden/play/beside/sum.kso` is the reduction. Its `.stderr` and
`.run.stderr` pin what each verb says, and
`definitions_beside_a_local_import_are_sent_to_a_module` in
`tests/play_verb.rs` builds the module the messages describe and runs it.

No test read the `.stderr` files in `tests/golden/play`. The archive noted
this for one of them on 2026-10-10. `every_play_golden_is_read` now reads
every golden in that directory: each `.stdout` on both engines, each
`.stderr` under `play`, and each `.run.stderr` under `run`. Two goldens
were stale. `no_local_imports.stderr` spelled the import without its
`./`. `decls_after_statements.stderr` held a refusal the play parser
dropped when it began sorting a file's lines by kind; that fixture now
prints `first`, so its golden is a `.stdout`.

The audit's other findings on #1893 are open there. An insisting form's
failure names the library line instead of the call, and neither a wrong
kind reaching a builtin or operator nor a stack overflow has a location.

CI's rows for this branch fall, and the change reaches no decision they
count, so the move is the binary's layout: `compile_instructions`
22,679,084 -> 22,664,963, `entry_instructions` 75,152,899 -> 75,111,766,
`library_instructions` 74,839,906 -> 74,798,192, `emit_instructions`
5,440,488 -> 5,439,036, and `interp_instructions` 492,635,997 ->
492,629,961. The tab's compile rises: `browser_compile_instructions`
382,155,032 -> 382,297,788, on rustc 1.98.1. The arms this change adds run
only when a file is refused, and no file in the corpus is. Marking
`local_import` cold moved the row by 1,529, so the rise is not that
function. The browser rise outweighs the five falls by less than 0.0001 of
welfare, and the floor moves to the score with the reason recorded, as it
did for kanso#1959 and kanso#1963.

## 2026-10-14 — The work-ahead rig builds again

The compiler page tells a reader to build `bench/workahead/aswritten.kso` and
`overlapped.kso`. Each file declares `grind` and `spin` beside its statements,
and since files of that shape became libraries, `kanso build` refuses both.
Nothing else reads the rig, so nothing went red. The two programs are now
modules, `bench/workahead/aswritten` and `bench/workahead/overlapped`, and
both import the functions from `bench/workahead/rig`.

The panel's rows are re-sat: 15 interleaved heats each on the build
container. As-written reads a minimum of 2,004 ms where the panel said
2,913, because the pool now builds `grind 150000000` during the sleep.
Overlapped reads 2,004 ms where the panel said 2,008. The two differ in their
output: overlapped prints the report before the delivery, and as-written
keeps the order it was written in.

## 2026-10-14 — A pooled call may read its arguments through a builtin

The pool from kanso#1978 took a call on a function's locals only when every
argument was a local, a literal, an operator over those, or a call of the
program's own. A call that measured its argument first was left after the
wait:

```
os/args .> (a -> wait a)

fn wait a
  sleep 2000
    .> (_ -> print "delivery: made")
    .> (_ -> print "report: {spin (150000000 + length a) 0}")
```

`length a` is a builtin, so `spin` ran after the sleep. With the length
taken before `wait` is called, the same program already pooled: 2.01 s
against 4.38 s on main.

A pooled call's arguments may now apply a builtin that reads a value and
answers a new one: `length`, `keys`, `values`, `entries`, `chars`, `bytes`,
`char_code`, `from_code`, `to_int`, `to_float`, `round`, `sqrt`, `sum` and
the six `bit_` words. The words that extend a value stay out. The beat pass
may write the result of `push`, `put`, `concat` and the rest into the
storage of the value they were given, and a pooled cell runs before the
program's own reads of that value.

The program above, run with `kanso play` on this container: 4.07 to 4.57 s
on main, 4.01 to 4.21 s on kanso#1978's branch, and 2.01 to 2.08 s on this
one, with the same output.

Pinned by:
- `tests/golden/mem/work_ahead_pools_a_call_on_a_builtin_read`:
  `ahead_forced=1`;
- `tests/golden/mem/work_ahead_leaves_an_extending_builtin`: a `push` inside
  the argument, `ahead_forced=0`;
- two ratchet rows, each seen red on its golden.

## 2026-10-14 — A lambda whose body parks first is pooled like a function

A lambda has no statements to bind a cell in, so the pool never reached one:

```
k = 150000000
os/args .> (a -> sleep 2000 .> (_ -> print "r: {spin (k + length a) 0}"))
```

`pool` now lifts such a lambda into a function of its own, `ahead~lam~N`,
whose parameters are the locals the body reads (`k` here) followed by the
lambda's own. The lambda stays where it was and calls the function. The
lift is kept only when the function pools something; a lambda with nothing
to pool keeps its body, since the call would buy nothing. A body that holds
an `if` block or a guard is left alone, because the walk does not follow
the names those bind.

The program above, with `kanso play` on this container: 3.71 to 4.09 s on
main, 2.02 to 2.09 s on this branch, with the same output.

Pinned by `tests/golden/mem/work_ahead_lifts_a_parking_lambda`
(`ahead_forced=1`) and the ratchet row "a parking lambda never lifted",
seen red on that golden.

The walk for a parking lambda runs only when the program reaches
`time/sleep`, the one function that parks; a program that never calls it
has had it pruned. Without that guard the walk cost the tab 366,863
instructions on a corpus with no sleep in it.

Rows against kanso#1978, the browser row read here on rustc 1.98.1 and the
rest by the compile sweep on this container, where `compile_instructions`
read what CI read for the unguarded branch: `emit_instructions` 5,449,223 ->
5,452,431 for the guard and its check. `browser_compile_instructions`
381,261,098 -> 382,590,239. Of that rise, 1,320,831 stays with the walk
switched off entirely, and the builtin-read commit alone leaves the row
where kanso#1978 put it, so it arrives with the commit that adds the lift
while none of its code runs; marking `pool` and `lift` never-inline did not
move it. The front end's rows rose with the binary, since `kanso check`
never reaches the pass: `compile_instructions` 22,595,494 -> 22,699,901,
`entry_instructions` 74,872,489 -> 75,214,425 and `library_instructions`
74,559,617 -> 74,901,006; `interp_instructions` fell with it, 492,636,904 ->
492,620,187. The runtime and machine-code rows did not move. Welfare falls,
and the floor moves by that amount under #1856's ruling, as it did for the
group pool in kanso#1972.

A pool-shaped differential ran 400 generated programs, each with a sleep
whose continuations read locals, lambda parameters and builtins of them,
some inside a function and some in nested lambdas. The interpreter and the
native build agreed on all 399 the front end accepted; 85 of the first 100
forced at least one pooled call.

## 2026-10-15 — gavel: a record field carries no type, and kanso has no type parameters

Clay ruled #1946. Two July rulings could not both stand. On 2026-07-25 a type
could declare a parameter (`type <k>pair` with fields `first:k` and
`second:k`), and on 2026-07-26 a written field type became an error. The
parameter means nothing unless a field can write `:k`.

**The ruling.** No field carries a written type of any kind, and kanso has no
type parameters. The archive's "GAVEL (amendment): the binder stays" and the
"CORRECTION" after it are reversed. The reason is the one the 07-26 ruling
gave for concrete types. kanso compiles the whole program, so the compiler
sees every place two fields' values meet. If something needs them to agree,
a disagreement is refused at that use. If nothing needs them to agree, no
program depends on it. Declaring the agreement only moves where the error is
reported. Relating types was already gone from function signatures for the
same reason, and now it is gone from type declarations too.

Owes: #1941 closed with a pointer here; `sorted` stays a plain type, and the
#1903 entry's mention of `<k>sorted` no longer describes anything to build.

## 2026-10-15 — gavel: a span is a range a check has proven inside its string, and slicing one cannot miss

Clay ruled #1960. `text/slice!` answers a box (2026-09-16), and a box spreads
to every caller, so the JSON decoder written with it would make `json/decode`
answer a box. The insisting form could not give a caller that knows its
bounds a plain string. Proving ranges by inference was weighed and set aside:
a reader cannot see whether a slice is proven, and an edit to the scan
breaks the proof far from where the error lands.

**The ruling.** A span is a range of positions inside one particular string,
and only a function that has checked the bounds makes one, the way only
`list/to_sorted` makes a `sorted`. `text/slice` on a span answers the string,
with no `none`, because the range cannot leave it. A slice from bare numbers
answers `none` past the end, as ruled on #1890, and `text/slice!` insists
with a box, as ruled on #1943. The word is span, after C++'s `std::span` and
the source ranges compilers carry: a value holding two positions into a
string that already exists, holding none of its characters.

Owes, on #1960: the span value in std/text, built only by bounds-checked
functions; the scanning functions the decoder uses (`find2` and its
siblings) answering spans or `none`; `text/slice` arms for a span; the
fused doors (`utf8`, `append`, `to_int`, `to_float`) on a sliced span; the
decoder moved off `builtin_slice_in` onto spans; micro goldens on every
engine. The name of each new public function goes to Clay as one question
before it ships.

## 2026-10-15 — gavel: a run-time failure names the line it happened on

Clay ruled #1975. All twenty ports met failures that named no file or line:
a wrong kind reaching an operator or builtin (`` `*` is not defined for these
values ``, `write takes a string`) and a stack overflow. The interpreter has
the span and drops it, because native has nothing to print beside it and the
engines must agree byte for byte.

**The ruling.** Every run-time failure names the file and line it happened
on, on every engine. Compiled code records a constant `file:line` before each
runtime call that can stop the program, and the runtime prints it when it
dies; the interpreter prints the span it already holds, and the tab shares
the C runtime. An insisting form names its caller's line rather than the
library's. The cost is one store per such call, measured at no more than
0.42% of runbench before the build; Clay accepted it, and the floor drops by
what CI measures, recorded with this entry as the reason. A refusal at check
where the wrong kind is certain costs nothing at run time and goes in beside
it.

Owes, on #1975: the stamp in codegen and the runtime's print; the
interpreter printing its span; insisting forms stamping the caller; error
goldens on every engine for an operator, a builtin and a stack overflow;
CI's rows and the floor moved by them.

## 2026-10-15 — An operator handed a literal string, list or map is refused at check

The first piece of #1975, and the one that costs nothing at run time. `+`,
`-`, `*`, `/` and `%` take numbers, and an operator arm routes only a record,
so a string, a list or a map stops the run on every engine. The check now
refuses one at the operand, in `wrong_kind_at_an_operator` in
`src/check/after_infer.rs`, beside the `list/concat` refusal.

It refuses only an operand that cannot hold an err: a literal with nothing
interpolated, or a call of such literals into a group whose every arm answers
a string, a list or a map and none fails. The first draft read the callee's
answer set alone, and `tests/golden/micro/a_merged_err_stays_flat_across_a_hop`
caught it: `through merged + e3` calls a group whose arms all answer strings,
but `through` has no err arm, so the err it is handed hops through it and the
two errs merge at `+`. An operator hands an err on, so anything that might
carry one is left to the run.

That leaves the ports' case, `twice "x"` where `twice n` is `n * 2`, to the
run, which is what the rest of #1975 locates. Pinned by
`tests/golden/errors/an_operator_handed_a_value_it_never_takes`, seen red with
the call removed; the ratchet row `operator_kind_unchecked` runs the same
mutation.

The walk costs compile time, which CI measured: `compile_instructions`
22,699,901 -> 22,715,476, `entry_instructions` 75,214,425 -> 75,262,121,
`library_instructions` 74,901,006 -> 74,948,228 and
`browser_compile_instructions` 382,590,239 -> 382,741,447. The run rows did
not move. This builds a ruled part of the language, so the welfare floor
moved down by exactly that cost, to 91.52, with this entry as the reason.

## 2026-10-17 — gavel: every run-time failure names its line except a stack overflow

Clay ruled #1987. The #1975 ruling accepted naming every run-time failure's
line at a bound of 0.42% of runbench. Built in kanso#1985, CI measured
+1.70% on runbench, +3.01% on livebench and +4.36% on encodebench. The
bound had counted only calls into the C runtime. It missed the stores on
inlined fast paths, which moved onto the slow branch, the stamp before a call
that can match no arm, and the store on entry to every function of a
recursion cycle that records an overflow's place. The overflow's place also
broke the differential law: an engine names the cycle it entered last, and
the engines run out of stack at different depths, so a program that keeps
entering a second cycle gets two different answers.

**The ruling.** Every run-time failure names its file and line on every
engine, except a stack overflow, which keeps its sentence with no line, as
main prints it today. A call that matches no arm names the caller's line.
Dropping the overflow's entry stores saves about 0.8% of runbench and 1.5%
of livebench and encodebench, and the divergence goes with them. Naming an
overflow's place by the outermost active cycle is possible and is its own
issue, measured from scratch; nothing in this ruling waits on it. The floor
drops by what CI measures for the rest, under the #1975 gavel.

Owes, on #1987: kanso#1985 without the overflow's entry stores; CI's rows
and the floor moved by them; a follow-up issue for the outermost-cycle rule.

## 2026-10-17 — gavel: the names for spans

Clay ruled #1983, the names the #1960 gavel said would come to him before
any shipped.

**The ruling.** In std/text:

- `span` is the type, with readable fields `whole` (the string), `from` and
  `to` (inclusive, the way `text/slice` counts). An empty run has `to` one
  less than `from`. Only std/text builds one.
- `text/span s from to` answers a span when the range lies inside `s`, and
  `none` otherwise.
- `text/span_to cs p a b` answers the span from `p` up to the byte before the
  first `a` or `b`, or to the end of `cs`, and `none` only when `p` lies
  outside `cs`. `text/span_to_below cs p a b lim` also stops at any byte
  below `lim`. The names say what they answer. `find2` and `find2_below`
  keep answering a position.
- `text/slice run`, an arm of `slice` that takes a span, answers the string
  with no `none`.

Owes, on #1983 and #1960: the five names on every engine; the fused doors
reading a sliced span; the decoder's four `builtin_slice_in` sites moved
onto spans; micro goldens; appendix B; kq's decoder following.

## 2026-10-18 — A run-time failure names the line it happened on

The rest of #1975. Every run-time failure but a stack overflow now prints
its place under its message, on every engine, in the form an err's `born in`
line uses:

    error[runtime]: `*` is not defined for these values
      failed in calc/twice at calc.kso:2

**The interpreter** fills the place in as a failure leaves the body that
raised it. Every expression in a body shares the body's frame, so the frame
changes on the way out only where a declaration's body, a lambda's body or a
lazy binding is run, and each of those places a failure that has no place
yet. Three paths raise a failure outside any expression, and each places it
itself: a statement's destructure, a tail call whose dispatch matches no arm,
and a tail call through a function value that takes the wrong count. The
first version placed at every expression, which cost the interpreter row
about 3.7M of the 15M it rose.

**Compiled code** stores the same literal an err born at the site would
carry into `k_site` before each call that can stop the program, and
`k_died` prints it. The stamped calls are the operators' slow paths, builtin
calls, condition tests whose value is not provably a boolean, field and index
reads, destructures, keyed reads, joins, calls through a function value, and
calls of a group that can match no arm. A store to `k_site` is exempt from
`writes_state`, since running a body over repeats it harmlessly.

Two things keep the store off the hot path. An inline twin (`length`, `at`,
`append`, the bit operations, `push`, `put`, `slice`, the scans) takes the
site as its last argument and stores it only on the branch that reaches the
C entry, which is the only branch that can stop the program; `takes_site`
reads which twins do from their definitions in `declares.rs`. With the store
in front of every inlined twin, CI read runbench +6.4% and digestbench +18%.
And a call into a group is stamped only when the arguments' proven kinds
might fail every arm: the arms are read together, so `f x true` and
`f x false` take a proven boolean, an annotation the dispatcher tests by kind
alone takes that kind, and a by-value record is taken by the arm that
destructures its type.

**A stack overflow** names no place. The engines run out of stack at
different depths, so no frame is the one the overflow happened in. The first
version named the cycle the program last entered, which was wrong in a way
no fixture showed: a recursion that enters a second recursion at every level
ran out inside `small` on native and on a call to `big` in the interpreter,
and the two printed different places. A rule that holds still (the outermost
active cycle names it) needs the record cleared when the entering call
returns, and a library wrapper that tail-calls into a recursion has nowhere
to clear it. The overflow keeps its sentence alone, under "2026-10-17 —
gavel: every run-time failure names its line except a stack overflow".
`a_recursion_inside_a_recursion_overflows_alike` pins the case.

**An insisting form** names the line that called it. A form whose name
ends in `!` turns an err into a failure, and the err was born inside the
library, so `os/read_file!` named `os/insisted at std/os/os.kso:113`. Each
call of such a form now hands its answer back through one step that points
an err born in the form's own package at the caller's line; an err born
anywhere else passes through unchanged. Native does it in
`k_b_insisted`, and an answer that is still an effect gets the step wrapped
around it so the err it later yields is pointed too. The interpreter does
the same with `restamped`, and with a `Desc::Insisted` node for an effect.

A lazy binding that demands its own value still names no place. The
interpreter has no source position for it, and native clears `k_site`
before saying so, so the two agree on saying nothing.

Pinned by the runtime corpus, which native, the interpreter and the tab
now print byte for byte, and two new fixtures:
`an_operator_through_a_parameter_names_its_line` (the ports' case) and
`a_recursion_inside_a_recursion_overflows_alike`. `read_missing_file` and
`a_slice_bang_past_the_end_reaches_the_endpoint` moved to name the caller.
The ratchet rows `death_unplaced`, `oracle_unplaced`, `insisted_native`
and `insisted_oracle` each take one piece away and watch the corpus go red.

**Rows**, CI's against main. The run time, from the stamps that remain and
the slow-branch site arguments: `work_basket` 30,958,481 -> 31,158,162,
`work_deepbench` 403,115,516 -> 407,387,521, `work_digestbench` 5,911,344 ->
5,992,785, `work_encodebench` 2,387,084,099 -> 2,464,675,698,
`work_escapebench` 39,058,340 -> 39,058,403, `work_indexbench` 2,462,589 ->
2,482,608, `work_jsonbench` 830,048,931 -> 832,975,500, `work_livebench`
1,474,835,606 -> 1,498,216,646, `work_oneshot` 13,207,693 -> 13,286,600,
`work_pendbench` 191,674,224 -> 194,087,911, `work_readbench` 4,580,963 ->
4,582,588, `work_runbench` 1,177,696,062 -> 1,189,193,317, `work_scanbench`
273,812 -> 274,086, `work_widebench` 28,451,432 -> 28,628,235.

The interpreter, placing at the body: `interp_instructions` 492,620,187 ->
500,108,350. The emitter and the code it writes: `emit_instructions`
5,452,431 -> 6,061,105, `codegen_instructions_dev` 112,677,448 ->
115,010,801, `codegen_instructions_release` 441,885,670 -> 449,091,786,
`emitted_calls` 1,023 -> 1,024, `emitted_lines` 10,165 -> 10,362,
`emitted_other_calls` 17,641 -> 17,648, `emitted_other_lines` 145,594 ->
149,547, `text` 4,909,294 -> 5,011,246, `lines` 1,356 -> 1,368,
`module_lines` 1,106 -> 1,130. The front end fell with the binary's layout:
`compile_instructions` 22,721,761 -> 22,693,337, `entry_instructions`
75,287,373 -> 75,197,395, `library_instructions` 74,973,469 -> 74,884,283.
The tab: `browser_compile_instructions` 382,767,025 -> 389,899,281,
`browser_compile_peak_bytes` 643,605 -> 675,444, `browser_run_instructions`
34,181,355 -> 35,216,926.

Every benchmark reads its input through `os/read_file!`, whose effect now
comes back inside a description holding the caller's line, so each allocates
3 more and 96 bytes more and evacuates the same: `run_alloc_bytes`
300,934,796 -> 300,934,892, `run_allocs` 1,812,043 -> 1,812,046,
`run_evac_allocs` 49,221 -> 49,224, `run_evac_bytes` 14,917,984 ->
14,918,080, `alloc_bytes` 175,182,800 -> 175,182,896, `allocs` 1,390,963 ->
1,390,966, `evac_allocs` 18 -> 21, `evac_bytes` 640 -> 736,
`encode_alloc_bytes` 665,662,512 -> 665,662,608, `encode_allocs` 5,332,649
-> 5,332,652, `encode_evac_allocs` 24 -> 27, `encode_evac_bytes` 912 ->
1,008, `oneshot_alloc_bytes` 2,375,978 -> 2,376,074, `oneshot_allocs` 12,064
-> 12,067, `oneshot_evac_allocs` 18 -> 21, `oneshot_evac_bytes` 624 -> 720,
`wide_alloc_bytes` 5,590,880 -> 5,590,976, `wide_allocs` 128,017 -> 128,020,
`wide_evac_allocs` 125 -> 128, `wide_evac_bytes` 516,000 -> 516,096,
`digest_alloc_bytes` 697,025 -> 697,121, `digest_allocs` 7,193 -> 7,196,
`digest_evac_allocs` 48 -> 51, `digest_evac_bytes` 4,176 -> 4,272,
`read_alloc_bytes` 198,768 -> 198,864, `read_allocs` 413 -> 416,
`read_evac_allocs` 18 -> 21, `read_evac_bytes` 608 -> 704,
`live_alloc_bytes` 409,410,992 -> 409,411,088, `live_allocs` 1,120,086 ->
1,120,089, `live_evac_allocs` 18 -> 21, `live_evac_bytes` 608 -> 704.

This builds a ruled part of the language, so the welfare floor moves down by
exactly what the rows cost, under the #1975 gavel as narrowed by the
2026-10-17 gavel: 91.5235 -> 91.4587.

## 2026-10-18 — A module's own function beats a type an import declares

Building std/text's `span` turned up a check that read the wrong name. In a
merged module check, the bare name of a type another module declares reached
the construction check before a function of the same name in the program's
own module. `fn sorted a b` beside `import "std/list"` was refused as a
construction of std/list's `sorted` with too many fields, and any program
with a helper named `span` would have been refused the same way once std/text
declares the type. A type and a function cannot share a name inside one
module, so a bare name that is both is the module's function. `arity_at` now
skips the construction check for a name that is a known function group.

The fixture is `a_function_named_like_an_imported_type_is_the_function` in
the micro corpus. Without the change, `kanso check` refuses it with the arity
error. With the change, it prints `3` on every engine.

Rows, from CI: `compile_instructions` 22,715,476 -> 22,716,927,
`entry_instructions` 75,262,121 -> 75,265,987, `library_instructions`
74,948,228 -> 74,952,083 and `browser_compile_instructions` 382,741,447 ->
382,750,920. The check does no new work on these programs, so most of the
movement is the binary's layout. The floor drops by the cost to 91.52, under
the rule that a change making the language work as specified lowers it by
exactly what it costs.

## 2026-10-18 — A module's own type beats an imported type's twin

The type-side partner of the entry above. The loader clones each exported
type under its short name, so a program may write `sorted` for
`list/sorted`. A module that declared its own `type sorted` kept the twin
beside its declaration, and the twin answered: the module's constructions
were refused as foreign, with the import's field count held against them.
Once a module's types join the merged program, `own_types_beat_twins` drops
a synthetic short-named twin that shares a name with one of them, and the
import's type is still reachable by its qualified name. std/text's `span`
met this in `tests/golden/entryfile`'s diff module, which declares its own
six-field `span`.

The fixture is `a_type_named_like_an_imported_type_is_the_modules` in the
micro corpus, refused by both `kanso check` and a run before the change and
printing `2` on both engines after it. The first build kept a set of the
module's type names on every compile, 23 allocations in `compile_allocs` and
`interp_allocs` for a collision almost no program has; the filter now
allocates only when it finds a twin to drop, and both rows are where they
were.

Rows, from CI: `compile_instructions` 22,716,927 -> 22,721,761,
`entry_instructions` 75,265,987 -> 75,287,373, `library_instructions`
74,952,083 -> 74,973,469, and `browser_compile_instructions` 382,750,920 ->
382,767,025: each compile asks every type whether it is a beaten twin. The
floor drops by the cost to 91.52 under the rule for a change that makes the
language work as specified.

## 2026-10-18 — gavel: the span maker is `text/to_span`

Clay ruled #1992. The 2026-10-17 gavel on #1983 named a type `span` and a
function `text/span s from to`. A type's name is its constructor, so the
constructor `span whole from to` and the function share a name and an arity,
and the checker refuses the pair.

**The ruling.** The function is `text/to_span s from to`, answering a span
when the range lies inside `s` and `none` otherwise. A `to_` name means the
answer is a different kind of thing (2026-10-13, #1914), and `list/to_sorted`
builds a checked `sorted` from a list the same way. The type stays `span`,
and `span_to`, `span_to_below` and the span arm of `text/slice` keep the
names ruled on #1983.

Owes, on #1992: `text/to_span` in place of `text/span` wherever #1983's build
uses it; appendix B.

## 2026-10-18 — std/text answers spans, and the decoder takes them

#1960, under the gavels on #1983 and #1992; the second named the maker
`text/to_span`, because a function cannot share the type's name. std/text
declares `span`, with readable fields `whole`, `from` and `to`, and answers
one from `text/to_span s from to`, `text/span_to cs p a b` and
`text/span_to_below cs p a b lim`. `text/slice run` is an arm of `slice` that
takes a span and answers its bytes with no `none`. The type and its functions
live in `lib/text/span.kso`, so the line numbers err goldens quote from
text.kso stay where they are. Appendix B has an entry for each, and ch08
quotes the decoder's new shape.

The slice arm turned up a pass in the wrong place. `builtin_slice_in` was
renamed to `slice` while the parse was finished, before the check read it, so
a function answering a measured slice was typed as answering `none`. The
rename now runs where a checked program is handed to the engines.
`a_span_runs_to_the_byte_before_a_stop` in the micro corpus is refused
without the move.

The decoder's five `builtin_slice_in` sites read spans now, and
`string_scan` is gone. Moved as written, the decoder read 1,645M
instructions on jsonbench against 833M: every string built a span record,
195M of it in `k_rec`, and `text/slice run` reached none of the fused doors.
A new pass, `src/flat.rs`, runs beside the rename. A group whose arms take a
record at one position, or `none`, and only read its fields or hand it on to
another such group, gets a twin that takes the fields there. A call handing
a producer's answer straight to such a group (a producer is a one-arm group
of guards answering `none` and then a construction, the shape of
`text/span_to`) becomes a call to a copy of the producer that calls the
twin. `text/slice run` inside a twin becomes `builtin_slice` on the fields,
so the fused doors fire again. The twins keep their file and spans, and
`ast::spoken` takes the `Flat{n}_` mark off their names, so a failure inside
one names the function the author wrote. The producer's copy enters a twin
only when the twin's arms cannot miss, so a miss never names the producer's
line.

A bare construction is not a producer. The first build counted `keep` in
`a_long_chain_answers_survival_from_its_index` as one, and the loop ran half
on fields and half on records: 9,993 more allocations and 10,000 more beat
iterations. That shape is the escape analysis's, which hands a record back by
value where it can.

The twins' positions were boxed at first, because a group takes words only
when it is a member of a cycle, and jsonbench read 938M. A group the pass
made now takes its int fields as words too: 864M with the string sites moved
and 868M with the number sites, on this container. What is left is the range
check in `span_to` and `to_span` and the `- 1` and `+ 1` around an inclusive
`to`. The mem corpus reads the same counters as before the move, and
`a_span_handed_on_is_never_built` pins that a span handed from `span_to` to a
carrier builds no record: `sh_rec` 0 and 12 allocations, against 720 bytes
and 21 without the pass.

Two leaks of an imported type's short name surfaced on the way and landed
first, as #1993 and #1995. kq's copy of the decoder follows once kq repins.

Rows, from CI, against the tree with #1985 and #1996 in it. The decoder:
`work_jsonbench` 832,975,500 -> 871,576,578, `work_runbench` 1,189,193,317 ->
1,215,497,872, `work_oneshot` 13,286,600 -> 13,544,585, `work_livebench`
1,498,216,646 -> 1,498,473,868, and `work_encodebench` 2,464,675,698 ->
2,464,674,663. The others move by the size of a std/text with more in it:
`work_basket` 31,158,162 -> 31,158,240, `work_digestbench` 5,992,785 ->
5,992,876, `work_indexbench` 2,482,608 -> 2,482,712, `work_pendbench`
194,087,911 -> 194,087,989, `work_readbench` 4,582,588 -> 4,582,666,
`work_scanbench` 274,086 -> 274,177 and `work_widebench` 28,628,235 ->
28,628,313. The twins are more code: `emitted_defines` 111 -> 127,
`emitted_calls` 1,024 -> 1,216, `emitted_branches` 1,104 -> 1,300,
`emitted_lines` 10,362 -> 12,289, `emitted_other_defines` 2,089 -> 2,137,
`emitted_other_calls` 17,648 -> 18,224, `emitted_other_branches` 16,418 ->
17,006, `emitted_other_lines` 149,547 -> 155,354, `text` 5,011,246 ->
5,111,966, `emit_instructions` 6,061,105 -> 6,100,683,
`codegen_instructions_dev` 115,010,801 -> 115,149,899 and
`codegen_instructions_release` 449,091,786 -> 449,262,284. The front end
reads 51 more library lines: `compile_instructions` 22,693,337 -> 24,958,388,
`compile_allocs` 12,151 -> 13,741, `compile_peak_bytes` 552,167 -> 591,860,
`front_end_visits` 6,363 -> 7,309, `front_end_rounds` 19 -> 17,
`entry_instructions` 75,197,395 -> 75,588,739 and `library_instructions`
74,884,283 -> 75,259,150. The interpreter makes one more call per string:
`interp_instructions` 500,108,350 -> 571,513,912, `interp_allocs` 584,645 ->
651,909 and `interp_peak_bytes` 590,970 -> 632,433. The tab:
`browser_compile_instructions` 389,899,281 -> 420,614,367,
`browser_compile_peak_bytes` 675,444 -> 752,488 and `browser_run_instructions`
35,216,926 -> 37,494,108. Welfare falls 0.20, to 91.26, and the floor drops by
exactly that under the rule for building a ruled part of the language.

## 2026-10-18 — A local binding hides an alias only where it is bound

`canonicalize_bare_aliases` rewrites a bare call to an import's export into
the qualified name, and keeps a name bare wherever a local of that spelling
might mean the local. It kept the name bare in every declaration of the
program when any one of them bound it. A parameter called `run` in one
module therefore sent every other module's `run` through the loader's
short-named clone. An err born in that `run` said `born in run` where the
same program without the other module says `born in refusals/run`, and
basket, once std/text's `slice run:span` bound the name, read 6,113M
instructions against 31M, with 4,010 view allocations and 256MB held against
17 and 71KB. Both reproduce on main with no spans in them.

A declaration that binds an alias's name now keeps the bare spelling and the
clone it dispatches through, and every other declaration is rewritten.
`tests/bound_elsewhere.rs` runs `tests/golden/bound_elsewhere` on both
engines and fails on main with `born in run`. The first build walked each
declaration's bindings into a set of its own and cost entry and library
about 2.5%; the binding walker hands each name to a callback now, so one
walk asks the aliases about every bound name and builds no set.

Rows, against main with #1985 in it: `compile_instructions` 22,693,337 ->
22,742,937, `entry_instructions` 75,197,395 -> 75,386,824,
`library_instructions` 74,884,283 -> 75,073,678, `emit_instructions`
6,061,105 -> 6,063,934, `compile_allocs` 12,160 -> 12,151, `interp_allocs`
584,659 -> 584,645, and `browser_compile_instructions` 389,899,281 ->
390,131,214. The floor drops by the cost under the rule for a change that
makes the language work as specified.

## 2026-10-19 — a `slice!` handed straight to a door reads the range in place

#1943's last owed item: the emitter fusing `slice! … .> utf8`, `.> append
acc`, `.> to_int` and `.> to_float` as it fuses the nested spellings (the
2026-10-14 gavel, corrected the same day to the chain spellings). The decoder
moved onto spans in #1994 and no longer needs it, so what is left is the
spelling a program uses for a range it has already bounded.

Under the corpus-first gavel the shape went into the run program first, as a
phase called `insist`: 5,000 laps reading a year, a month and a word out of one
line. Measured on main at kanso#1994, it costs 11,955,088 instructions built
alone, 0.97% of the program, and adds 25 bytes to the peak. Welfare's run
baselines were re-based by the readings with the phase over without it on that
compiler, 1,227,450,174 over 1,215,496,727 and 5,063,225 over 5,063,200, so
adding the phase cost that compiler nothing.

The rewrite runs at the start of the flatten pass. `text/slice! s from to .>
text/utf8` becomes `if (text/past? s from to) (text/slice! s from to) (effect
(text/utf8 (builtin_slice s from to)))`, and the emitter fuses the door in the
third argument. `past?` is std/text's statement of where `slice` answers none,
and the prune that drops a shipped function nobody names keeps it whenever
`slice!` is kept. A range that leaves the value is still the `slice!` the
author wrote, failing on their line. The arguments are read twice, so the
rewrite takes only names, integers and `+`, `-` and `*` on them.

The first version was a `!` helper in std/text per door. It saved the slice
but was itself an insisting form, so an err the door raised, `"ka" is not an
integer`, was pointed at the caller's line where the two steps leave it in
`text/to_int`. An optimisation should not change what a program reports, so
the helper went.

Three faults turned up on the way, each with a golden that goes red without
its fix:

- The `to_int` and `to_float` doors gave an err the call site's origin. The
  oracle runs the wrapper, so the err is born in `text/to_int`. A rescue on
  the caller's own line lets through a failure born on that line, so on native
  alone the err escaped `.?`. The doors now take the wrapper's line, as the
  `utf8` door did. `a_slice_bang_chain_opens_its_door_in_place` reaches the
  endpoint on native without it.
- The `utf8` door stamped no site, so a value it cannot take stopped the
  program naming whatever line had stamped last (`text/past?`). It stamps the
  wrapper's line unless the value is known to be bytes.
  `a_door_a_slice_bang_opens_fails_inside_the_door` pins it.
- `.> (&text/append acc)` was refused: the formatter called the parentheses
  redundant, and without them the chain step refuses held arguments. After a
  chain operator the step is one atom, and the formatter now says so.

The new mem fixture `a_slice_bang_chain_builds_no_slice` reads 45 allocations
against 95 for the two steps. Built alone, the phase reads 6,081,704 against
11,955,088. Each step still builds an `effect` box and the closure for the
next step, which is what a chain costs.

Rows, from CI. The phase's own work moves the run program: work_runbench
lands on 1,221,576,962 against 1,215,497,872 without it, and its `.text` row,
text, on 5,132,910 in total. The run vein lands on run_allocs 1,872,053,
twelve a lap, run_alloc_bytes 303,335,164, run_beat_iters 92,018,
run_evac_allocs 49,265, run_evac_bytes 14,919,712, run_perm_allocs 101,
run_perm_live_bytes 15,825, run_perm_peak_bytes 146,913 (the line, 25 bytes),
run_sh_bytes 4,217,520 and run_utf8_bytes 424,489. The emitted code for the
other benchmarks, runbench's among them, lands on emitted_other_lines 156,609,
emitted_other_calls 18,424, emitted_other_branches 17,158 and
emitted_other_defines 2,169. compile_instructions 25,083,410,
entry_instructions 76,579,726 and library_instructions 76,283,760 rise for the
rewrite and `past?`, and browser_run_instructions lands on 37,494,129, 21
more. emit_instructions 6,095,522, interp_instructions 570,698,408 and the
tab's compile, 419,334,691, fall, and oneshot and livebench fall by 1,099 and
7, by mechanisms not isolated. Welfare rises 0.02 and the floor is banked at
91.28.

## 2026-10-19 — an insisting form builds no closure, and no step once its box has settled

Since #1975 an insisting form names its caller's line. On native,
`k_b_insisted` did that by wrapping every box a `!` function answered in a
rescue step whose callee was a closure holding the caller's site: three
allocations on every call, whether or not anything failed. Measuring #1943's
first version found it, where a `!` helper cost 7 allocations a step against
4 for the same helper without the `!`.

The step now holds the site where a rescue holds its function, and
`k_worded_step` reads it on the failure path, so no closure is built. A box that
has already settled, which is what `effect` answers, is decided at once: its
value passes untouched and its failure is pointed at the caller then. That
covers `text/slice!` and every `!` function written as `effect` over a value.
`os/read_file!` answers a box that has not run, and keeps the step without
the closure, so each benchmark that reads its input with it allocates two
fewer: eight cost goldens move by that and nothing else.

The new mem fixture `an_insisting_form_that_settled_builds_no_step` reads 104
allocations against 164. The runtime corpus's two insisting goldens,
`a_slice_bang_past_the_end_reaches_the_endpoint` (a settled box) and
`read_missing_file` (one that has not run), name the same lines as before on
both engines.

The first build of this change read deepbench 2% higher with every counter
unchanged. `k_exec` and `k_step` had each shrunk by about the size of
`k_worded_step`: the insisting branch made the step big enough that the C
compiler stopped inlining it into the two chain walks. It is static and
always inlined now, and the insisting branch is a cold call out of line.

Rows, from CI. Every binary's `.text` is 1,824 bytes smaller. deepbench
falls 663,690 to 406,723,831, runbench 150,682 to 1,221,426,280 and
widebench 32,285 to 28,596,028; the run vein falls by two allocations,
run_allocs 1,872,051. work_livebench rises 36,885 to 1,498,510,746,
work_readbench 1,264 to 4,583,930, work_oneshot 297 to 13,543,783 and
browser_run_instructions 133 to 37,494,262, by mechanisms not isolated. The
cohort spec's `evac_bytes` pin reads 800,720, the closure's 48 bytes fewer.
Welfare rises, and the floor is banked.

## 2026-10-19 — json/decode refuses a number RFC 8259 refuses

#1999. `json/decode` took `1.`, `2.e5`, `01`, `-01`, `00` and `-.5`, and
`[1., 2]` decoded as `[1.0 2]`. RFC 8259 §6 refuses all of them. The decoder
finds a number's bytes with `text/number_span` and hands them to `to_int` or
`to_float`, and the converters are more lenient than JSON. They already refuse
`1e`, `1..2`, `1+2` and `--1`, so what they take that JSON does not comes down
to three rules: a leading 0 has no digit after it, a minus has one, and so
does a point.

`number_span` answers zero when a number breaks one of them. The first two are
reads at the number's start. The third uses the masks the sixteen-byte walk
already builds: a point whose next bit is not a digit, with the block's last
byte checked against the next block. The interpreter keeps the same three
rules, so `number_span` answers the same on every engine for every input.
lib/json needs no new arm: a zero sends the integer path to ask `to_span` for
a range that ends before it starts, which answers none, and the decoder fails
as `invalid number`.

The first version checked the rules in lib/json with a `find2_below` for the
point, and jsonbench read 948M against 872M, about 240 instructions a float.
A full grammar walk inside `number_span` read 942M, since jsonbench's numbers
are long. The three rules on the masks read 885.5M, 22 instructions a number.

A differential fuzzer against the RFC's grammar written as a regular
expression decoded 200,000 random strings over the number alphabet, each bare
and again padded past a block, on both engines, with 0 mismatches. Main read
1,317 mismatches on the first 20,000. `number_span` agrees across engines at
every position of 3,000 random strings. `a_json_number_follows_rfc_8259` pins
the cases, and `a_number_span_ends_where_the_byte_walk_ends` gains the zero
answers, with a bare point at a block's last byte and in the walk.

kq carries its own decoder and keeps a number as the decimal its author
wrote, with jq as its oracle, so what it accepts is jq's question and this
change leaves it alone.

Rows, from CI, read again after main took #2001's insisting forms. All of
the run rows' movement is in `number_span`: work_jsonbench lands on
885,495,792 (+13,919,550), work_runbench on 1,230,612,980 (+9,186,700),
work_oneshot on 13,636,587, work_livebench on 1,498,602,745 and
work_encodebench on 2,464,674,586. The tab's run, browser_run_instructions,
rises to 37,652,207, and its compile to browser_compile_instructions
419,365,540 with browser_compile_peak_bytes 752,376. text lands on 5,111,406,
288 bytes a binary for the check. compile_instructions 25,085,857,
entry_instructions 76,583,774, library_instructions 76,287,809,
interp_instructions 570,745,964, codegen_instructions_dev 115,143,517 and
codegen_instructions_release 449,248,021 rise for the edit to the compiler's
Rust. Every allocation counter agrees with main. The floor drops by the cost
under the rule for a change that makes the language work as specified, a JSON
decoder that refuses what RFC 8259 refuses: welfare falls 0.03, to 91.25.

## 2026-10-19 — a stack overflow names the outermost cycle still running

#1991, owed by the 2026-10-17 gavel (#1987): every run-time failure names its
line, and a stack overflow was the one that named none. kanso#1985 tried the
cycle the program entered last and was taken out, because the engines run out
of stack at different depths and a recursion that enters a second recursion at
each level named a different cycle on each engine.

The place is now the outermost cycle that grows the stack and is still
running. No depth decides it, so the engines agree. `src/deep.rs` finds the
cycles from the call graph, and a cycle's place is the first call written
inside it to a member outside tail position. An overflow prints the sentence
and then `failed in <function> at <file>:<line>`, as every other run-time
failure does.

Each engine keeps one record: `k_deep_site` in the runtime, `deep_now` in the
interpreter. A call that enters a growing cycle from outside it saves the
record, sets it when it is empty, and puts it back when the call returns. The
put-back is what keeps a returned cycle from naming a later overflow
elsewhere. Three entries needed more than that:

- A tail call into a cycle has nowhere to put the record back, and a one-line
  library wrapper that tail-calls its worker is the common case. The caller is
  not on the cycle, so the call is made as an ordinary one. It costs one frame
  per entry, not one per step.
- A lambda or a thunk is on its maker's cycle, since it can be a step of a
  loop the cycle runs through it. It keeps its tail calls, and a call it makes
  into another cycle only sets an empty record. Making those calls ordinary
  broke `a_lambda_called_in_tail_position_costs_no_stack` in the CPS
  translator, which is how this came to light.
- A group called as a value enters through its `w_` wrapper, which records as
  a call from outside does.

The pool of work-ahead puts the record back when pooled work overflows, so an
overflow it rescued names nothing later.
`pooled_work_that_overflowed_leaves_no_name_behind` goes red without it.

The tab traps when the wasm call stack runs out, so the runtime exports
`k_deep_place` and the page's host reads it after the trap.
`kanso::stack_exhausted` takes the place and prints the same two lines on
native's `kanso run`, the tab and the test host.

Goldens: `a_recursion_inside_a_recursion_overflows_alike` names `big` on every
engine. `an_overflow_entered_through_a_tail_call_names_its_cycle`,
`an_overflow_entered_through_a_value_names_its_cycle` and
`an_overflow_names_the_cycle_still_running` cover the three entries, and
`deep_recursion`, the five work-ahead overflows and
`a_built_binary_names_its_stack_overflow` now carry the place. Each fixture
was watched red without its part. Their recursions add floats, because an
integer `1 + f (k - 1)` is rewritten to a loop and never overflows.

The first CI run read 0 for compile, entry and library instructions and could
not read start-up, emit or the interpreted run. valgrind had read no symbols
from the compiler, so no gate found the frame it reads. It is the layout the
2026-09-13 entry met on this container: with RELRO, lld writes two writable
segments, and when the first ends on a page boundary they are contiguous in
the file and in memory, and valgrind 3.22 reads nothing from the executable.
This branch's binary landed there on CI; #2002's job, re-run on the same
runners, read every row. Building here at CI's checkout path with CI's
`CARGO_HOME` reproduced it, and linking with `-z norelro` removed it. build.rs
now links the compiler that way on Linux, which leaves one writable segment
whatever the binary's size, and `the_compiler_links_one_writable_segment`
goes red without the flag. The compiler gives up RELRO's write protection on
its relocated pointers. The rows that read the compiler move by layout with
it: on this container `kanso::main` on the compile corpus read 24,962,673 with
RELRO and 24,961,819 without.

With the symbols back, CI read the interpreted run 16.06M instructions higher,
570,698,408 to 586,760,939. A profile diff against main put 17.1M of it in
`eval_body_flow`: the additions to the dispatch loop had pushed it out of line,
and it and `eval_body_flow_unplaced` are now inlined into their one caller. The
body also stopped looking up its own cycle. Every overload of a group is on
the group's cycle, so the loop sets the running cycle on entry and on each tail
hop from the lookup it already makes. On this container that took the run from
585,706,096 to 577,416,551, against main's 569,645,202.

Rows, from CI, read again after main took #2001 and #2002. Every emitted
module declares `@k_deep_site`, one line more: the compile corpus lands on
lines 1,372 and module_lines 1,131, the decoder on emitted_lines 12,296, and
the other benchmarks on emitted_other_lines 157,010 and emitted_other_branches
17,160, the rest being the saves and puts-back around the calls that enter a
cycle. Machine code grows to text 5,119,726 in total. Those calls cost the
benchmarks a few instructions each: work_jsonbench lands on 885,497,327,
work_encodebench 2,464,678,788, work_oneshot 13,636,623, work_basket
31,162,888, work_deepbench 406,723,866, work_escapebench 39,058,437,
work_pendbench 194,088,527, work_indexbench 2,482,746, work_scanbench 274,236,
work_digestbench 5,992,543, work_readbench 4,583,965 and work_livebench
1,498,605,914. work_widebench rises most, to 28,740,070 (+144,042, 0.5%), and
work_runbench falls to 1,230,554,702, by mechanisms not isolated. Clang reads
the longer modules: codegen_instructions_dev 115,648,918 and
codegen_instructions_release 453,327,164. The emitter finds the cycles,
emit_instructions 6,178,365, and the tab's compile does the same,
browser_compile_instructions 422,972,128 (+3,606,588, 0.9%) and
browser_compile_peak_bytes 753,318; browser_run_instructions lands on
37,653,273.
The interpreter keeps the places and a cycle on every closure:
interp_instructions 578,503,674 (+7,757,710, 1.4%), interp_allocs 652,272 and
interp_peak_bytes 633,881. The compile rows fall with the relink:
compile_instructions 25,066,066, entry_instructions 76,561,037 and
library_instructions 76,265,989. Welfare scores 91.24 against a floor of
91.25; this builds a ruled part of the language, so the floor moves down by
that cost.

## 2026-10-20 — the prune moves out of codegen.rs

The last item of #2005. `prune_unnamed`, the name-level call graph and
`kept_out` move unedited into `src/codegen/prune.rs`, read with include! as
`narrow.rs` and `wasm32.rs` are since #2006; `src/codegen.rs` is 4,674 lines.
The ratchet mutation that strikes the prune's mark patches the new file.

#2005 asked whether this graph and the one `src/deep.rs` builds should be one.
They answer different questions. This one records every name a body mentions,
values and partials included, keyed by name, which is what the prune needs;
`deep.rs` records the calls a body makes, keyed by name and arity, with tail
position and the place of each. They stay two. What was doubled was inside
this file: `call_graph` and `kept_out` each defined the same walker over a
body's names, and it is `name_mentions` now, once.

The move alone shifted codegen_instructions_dev by 15 instructions, and with
the walker defined once it read main's 115,648,918 again. (Corrected: this
entry first put the 15 down to kanso's layout, but the row leaves out kanso's
own process. The order the link directory lists in moves gold by that much
from one runner to the next; see "the link directory lists in one order".) The tab's compile, which runs the prune in the wasm build,
reads browser_compile_instructions 422,962,643, 9,485 fewer, and welfare's
rise is banked.

## 2026-10-20 — three leads in the decoder, measured and declined

Each was built, measured on jsonbench or the run program, and dropped. They
are recorded so nobody builds them again without new evidence.

**Deciding a string's closing quote inside `string_at`.** Spans cost jsonbench
38.6M (833.0M to 871.6M, kanso#1994). Most of it is the string path, 243.3M
against 204.4M, which goes through one more call level per string: the
`span_to` twin, the `string_at` twin, then the `string_end` twin, each reached
by a musttail call. A guard that decided the closing quote in `string_at` read
1,183.7M, because the guard broke `parsed` being returned by value
(`sh_rec` 254M). An `if` with a small non-tail call read 879.6M. Both lost to
the arm dispatch that is there.

**`alwaysinline` on the flattened twins.** On every twin, jsonbench read
891.3M, the callers growing with what was inlined into them. On `string_at`'s
twin alone it read 871,576,231, unchanged to the instruction: the twins reach
each other through musttail calls, and LLVM does not inline across those.

**A short-number path in `number_span`.** On kanso#2002's tree the run program
decodes 417,483 numbers at 94 instructions a call. Most of large.json's
numbers are one or two digits, or decimals of seven to nine bytes, with no
exponent. Walking digits and one point a byte at a time ahead of the
sixteen-byte walk took every call, but at seven instructions a digit:
work_runbench read 1,230,763,105 to 1,236,361,572 and the function 39.25M to
44.85M. Reading each digit run eight bytes at once with `k_nondigits8` and a
count of trailing zeros cost 93 a call and saved 365,211 on the run program,
0.03%. 96,000 random strings agreed between native and the interpreter at
every position, and the harness went red when the fast path dropped the float
sign. About 45 of the 93 instructions are the call's fixed work: the failure
and tag checks, the bounds, and RFC 8259's lead rules. Only a builtin that
scans and converts in one call would remove them, and the conversion it would
replace is already a word-at-a-time parse.

## 2026-10-20 — a JSON number skips its tag ladders, and a long slice counts in blocks

Two changes on the run program's hot paths, in one pull request (#2012, part
of #2011).

The decoder reads a number with `number_span` and then `to_int` or `to_float`
on the span. Both went through the checked builtins, which test the failure
flag and every tag before the work. Fast twins in `src/codegen/declares.rs`
now test the tags inline and call raw entries on the bytes:
`k_b_number_span_raw`, and `k_b_to_int_slice_raw` and `k_b_to_float_slice_raw`,
which convert the span where it lies. Native and the interpreter agreed on
54,000 generated number strings at every position, and 100,000 random strings
matched the RFC 8259 grammar with no mismatch.

`k_slice_skip` counted characters eight bytes at a time. While the target is
more than sixty-four characters off it now hands whole sixty-four-byte blocks
to `k_utf8_chars`, and a block that ends inside a character steps back to that
character's first byte. The bound assumes one byte a character, so on wider
text the step repeats until the target is close. The micro golden
`a_long_slice_counts_whole_characters` puts a block's end inside two-, three-
and four-byte characters; without the step-back it answers three of its lines
wrongly. A differential fuzz of `text/slice` (24,000 cases, native and
interpreted, against Python's slicing under kanso's range rules) found no
mismatch, and 1,533 in 4,000 with the step-back removed. The ratchet mutation
that disables the skip now makes it return at once.

CI's rows. runbench 1,230,554,702 -> 1,215,752,133; jsonbench 885,497,327 ->
871,505,027; indexbench 2,482,746 -> 2,326,925; oneshot 13,636,623 ->
13,543,502; livebench 1,498,605,914 -> 1,498,512,457; encodebench
2,464,678,788 -> 2,464,678,410. The twins' declarations and the block count
add about 5.7 KB of machine code to every program: text 5,119,726 ->
5,201,182 over the corpus, runbench text=725,209 -> 731,497. The three twins
are defined in every module, so the compile goldens read lines 1,372 -> 1,396
and module_lines 1,131 -> 1,137, and the decoder's emitted code reads
emitted_defines 127 -> 130, emitted_calls 1,216 -> 1,222, emitted_branches
1,300 -> 1,303 and emitted_lines 12,296 -> 12,368; across the other programs,
emitted_other_defines 2,169 -> 2,178, emitted_other_calls 18,424 -> 18,442,
emitted_other_branches 17,160 -> 17,169 and emitted_other_lines 157,010 ->
157,286. codegen_instructions_dev 115,648,918 -> 115,822,000, codegen_instructions_release 453,327,164 ->
453,660,707, emit_instructions 6,178,365 -> 6,202,944,
browser_compile_instructions 422,962,643 -> 424,273,317,
browser_compile_peak_bytes 753,318 -> 758,624 and browser_run_instructions
37,653,273 -> 37,747,192 rise with the larger declarations. Welfare rises and
is banked.

## 2026-10-20 — a JSON string is read and written in one call

The decoder read a string through a span to the first quote or backslash, a
dispatch on the byte the span stopped at, and a second door for the utf8, and
a string with escapes then went a run and an escape at a time through kanso.
The writer escaped a string the same way. Three internal builtins replace
that for every string that needs nothing unusual (#2014).

`builtin_plain_string cs p` scans from the body's first byte. With no
backslash before the closing quote it answers the text in front of the quote;
with escapes it carries on to the closing quote and answers its position; a
string that runs out, or bytes that are not utf-8, answer none.
`builtin_unescape cs p e` undoes the eight one-letter escapes over that range
and answers none for a `\u` escape or any other. On none the decoder reads the
string the long way from its start, so `\u` escapes, surrogates, bad escapes
and unterminated strings are undone or named by the code that did it before.
The native twin loops around the inlined scan, and the unescape copies runs
eight bytes at a time and, like `built_text`, does not validate again what
came from a string. `length (bytes s)` now reads a string's byte length from
its header instead of building a view in the arena to measure.

`builtin_append_escaped acc s` appends a string the writer's scan found
something to escape in, in the forms lib/json has always written. Linearity
and the beat analysis knew `append` by name, and they now know this builtin by
the same rules, so a site proved unique writes into the builder in place. The
escape is written straight into the builder when it has room, and through a
stack buffer otherwise, so nothing lands in the arena that `append` would not
put there.

Over 100,000 fuzzed documents decoded and re-encoded byte for byte alike
natively, interpreted, and on #2012's build, and the harness went red when the
escaped path was handed a closing position one byte early, when the unescape's
word offset was halved, and when the writer's table mapped a tab to `n`.

CI's rows. The work vein: runbench 1,215,752,133 -> 1,137,672,504,
jsonbench 871,505,027 -> 812,108,477, livebench 1,498,512,457 ->
1,370,942,122, oneshot 13,543,502 -> 12,776,869, encodebench 2,464,678,410
-> 2,464,678,221. The interpreter reads its corpus's JSON through the same
builtins: interp_instructions 578,503,674 -> 484,702,222, interp_allocs
652,272 -> 539,460, interp_peak_bytes 633,881 -> 614,115; in the tab,
browser_run_instructions 37,747,192 -> 35,361,799. The compile corpus imports
std/json, so the front end's rows fall with the escaper's functions gone:
compile_instructions 25,066,066 -> 24,089,825, entry_instructions 76,561,037
-> 75,139,973, library_instructions 76,265,989 -> 74,835,835, compile_allocs
13,741 -> 13,382, compile_peak_bytes 591,860 -> 572,289,
browser_compile_instructions 424,273,317 -> 408,301,693,
browser_compile_peak_bytes 758,624 -> 749,207, front_end_visits 7,309 ->
6,938, and front_end_rounds 17 -> 19.

The counters show the paths that left. A decoded string with escapes was a
builder handed to `built_text` without a copy; it is now a string `unescape`
allocated, so the zero-copy count falls (run_utf8_zerocopy 175,617 -> 90,
utf8_zerocopy 265,950 -> 0 on decode, oneshot_utf8_zerocopy 1,774 -> 1,
live_utf8_zerocopy 2,173 -> 400), and the appends with it (run_append_fast
8,256,960 -> 5,735,070, append_fast 1,900,500 -> 0 on decode,
oneshot_append_fast 90,477 -> 63,723, live_append_fast 31,135,470 ->
25,489,200), and the string headers counted rise where the byte ones fall
(run_sh_str 17,048,976 -> 24,593,568 against run_sh_bytes 4,217,520 ->
4,872; decode sh_str 1,319,216 -> 12,750,416; oneshot_sh_str 196,368 ->
272,576; live_sh_str 196,320 -> 272,528). The writer's stack buffer and its
straight writes leave run_held_peak_bytes, oneshot_held_peak_bytes and
live_held_peak_bytes at 197,704 -> 200,978, and live_alloc_bytes 409,411,040 -> 413,053,904.
On the run program run_sh_buf reads 119,176,736 -> 119,198,704, run_buf_reuse
139 -> 134 and run_survive_slots 4,854 -> 4,856. run_perm_live_bytes 15,825 ->
146,913 and run_bytes_freed 9,010 -> 9,009 are one list buffer of 8,192 slots,
the bag phase's, grown outside the arena as it was before and now still held
at exit; the peak does not move, and why it is no longer handed back is not
isolated (#2015).

The compile goldens read lines 1,396 -> 1,412 and module_lines 1,137 ->
1,141 for the twins defined in every module, and the decoder's own code
emitted_defines 130 -> 134, emitted_calls 1,222 -> 1,249, emitted_branches
1,303 -> 1,336 and emitted_lines 12,368 -> 12,690, while the other programs
lose the escaper (emitted_other_lines 157,286 -> 156,673). Machine code: text
5,201,182 -> 5,283,518. codegen_instructions_dev 115,822,000 -> 116,000,233,
codegen_instructions_release 453,660,707 -> 453,685,379 and
emit_instructions 6,202,944 -> 6,233,826 rise with the declarations.
The dev row first read 116,000,241 and then 116,000,233 on the commit that
added only page prose and this log; the 8 instructions are not isolated.

The mem fixture `append_of_a_slice_boxes_nothing` pins the fused append of a
slice, which std/json made for the run before each escape. Its `\n` escapes
are now undone in one call, so it uses `\u0041` to keep reaching the pair, and
reads append_of_a_slice_boxes_nothing_allocs 12 -> 14,
append_of_a_slice_boxes_nothing_alloc_bytes 1,307 -> 2,396,
append_of_a_slice_boxes_nothing_bytes_malloc 3 -> 4,
append_of_a_slice_boxes_nothing_perm_allocs 5 -> 6,
append_of_a_slice_boxes_nothing_find2_calls 42 -> 83,
append_of_a_slice_boxes_nothing_str_scan_bytes 165 -> 169 and
append_of_a_slice_boxes_nothing_sh_str 272 -> 816 for the `\u` path's own
work; its append_fast stays at 80.

Two shapes of the writer were measured worse first and declined. A builtin
answering an escaped copy read the run program at 1,221,510,922 against
1,166,052,728 without it: the copy cost 415 instructions a string, and with
the escaper's loop gone the encoder's map call lost its region and its fused
`keys` and `values`. The append-like builtin read 1,416,821,993 before the
analyses knew its name, because every append in the encoder stopped writing in
place.

## 2026-10-20 — directive: all development is cloud's

Clay: "all development is for you. the chat session is only for me to talk
with and make decisions with." Cloud now writes the log, the design notes,
the page's prose and CLAUDE.md as well as the compiler, and keeps the page
within `page_drift`'s three entries of the log itself. The chat files, labels
and closes issues and writes rulings onto them, and logs a ruling's `— gavel:`
entry itself or leaves it to cloud's next pull request. CLAUDE.md's "Two sessions"
section states the split. The first page section written under it is §219,
which covers the four entries from the prune's move to the string builtins.

## 2026-10-20 — the link directory lists in one order

The dev codegen row read 116,000,241, then 116,000,233, then 116,000,241 on
three commits of #2014 that changed no code. The profiles put all 8
instructions in `ld.gold`; kanso and clang read the same each time. A second
reading in the same job always agreed with the first, so whatever moved was
fixed for a job and differed between jobs.

It is the order gold reads the link directory in. gold reads every `-L`
directory whole, and the codegen box stages one holding the five libraries the
link resolves. ext4 lists a directory by a hash of each name, seeded per
filesystem, so each fresh runner can list those five its own way. Linking the
same inputs on this container with the five listed three ways read
22,443,938, 22,443,953 and 22,443,989.

`scripts/gates/codegen_box.sh` now keeps the entries on tmpfs, which lists
them in the order they were made, and makes them in a fixed order; on Linux
`$box/link` is a link to `/dev/shm/kanso-codegen-link`, so the path the build
is given does not change. Two readings here agree, and making the entries in
the reverse order moves gold by 17, so the order is now the script's. The
staging refuses to run when `/dev/shm` is not a tmpfs. Under the new order CI
reads codegen_instructions_dev 115,822,000 -> 115,822,620 on main's tree, and
the floor is re-based by that cost under kanso#2017's gavel: a fix that
normalizes a measurement re-bases the floor without a ruling.

The prune's entry put an earlier move of this row down to kanso's layout. The
row leaves out kanso's process, so that cannot be the reason, and the entry
says so now.

## 2026-10-20 — gavel: a fix that normalizes a measurement re-bases the floor without a ruling

Clay ruled #2017. The dev codegen row counts `ld.gold`, which reads its `-L`
directory in readdir order. On ext4 that order is a hash seeded per
filesystem, so each CI runner drew its own, and the row moved by a few
instructions between commits that changed no code. #2016 puts the directory
on tmpfs, which lists in creation order, and the row now reads the same on
every run: 620 instructions above main's golden. Welfare reads 91.29 either
way, but the floor is compared exactly.

**The ruling.** The floor comes down by exactly what CI measures under the
fixed order, with the reason in its history. It is also the standing rule: a
change that only normalizes what a row reads re-bases the floor by what CI
measures after it, with no ruling. The banked value was a draw. Picking among
fixed orders by which reads lowest would choose the measurement by its
result, so the order is fixed by what is natural (creation order) and the
floor follows. CLAUDE.md's welfare section carries the rule.

Owes, on #2017: the floor re-based in #2016 by CI's measured cost, recorded
with this entry as the reason; #2014 and #2018 merging main after it.

## 2026-10-20 — directive: a required feature's cost is never asked about

Clay, after #2017: "it should never need to ask to lower the welfare for a
feature implementation that's required." The language exception already let
a ruled feature lower the floor by what it costs. #1987 still came to him
because the build measured several times its estimate. CLAUDE.md now says
that a cost above the estimate changes nothing: the floor moves by what CI
measures, and only a choice about the language itself, such as narrowing the
ruling when the full build is impossible or contradicts another ruling, goes
to Clay.

## 2026-10-20 — a carried list keeps its buffer outside the arena

#2014 left the run program holding one list buffer of 8,192 slots at exit,
the bag phase's: run_perm_live_bytes 15,825 -> 146,913 (#2015). The buffer is
registered with beat 0, whose rewind would free it, and that rewind reaches
the registration. By then, though, the list's items field points at an arena
copy of exactly 4,300 slots. A watchpoint on the field under gdb caught the
write: the pop of the bag's own beat deep-copies its carried record, and
`k_repair_interior` copied the list's items into the arena because they were
not under the beat's mark. A malloc'd buffer never is. The copy dropped the
only pointer to the buffer, and the registration found an arena buffer in the
field and passed over it. The string and bytes arms of the same function
already leave malloc'd storage where it is. The list arm now does the same
and walks only the slots.

`k_interior_survives` has the same blind spot: it reports a list with
malloc'd items as not surviving, which is what sends the list to the repair.
Changing it as well moved no fixture and no benchmark counter, so it stays as
it is.

The mem fixture `a_carried_list_frees_its_buffer` fills a bag record inside
a loop's beat four times. Without the fix it reads perm_live_bytes 8,208 and
perm_peak_bytes 16,416, one lap's buffer kept to exit and held beside the
next; with it 0 and 8,208. `a_list_held_in_a_record_grows_where_it_stands` no
longer copies its 20,000-element list: evac_bytes 384,848 -> 560 and
alloc_bytes 1,642,720 -> 1,258,448, with perm_live_bytes 655,552 -> 655,408;
with nothing copied there is no outgrown buffer to reuse or shelve,
so a_list_held_in_a_record_grows_where_it_stands_buf_reuse reads 1 -> 0 and
a_list_held_in_a_record_grows_where_it_stands_sh_buf 624 -> 704.
On the run program run_perm_live_bytes 146,913 -> 15,825, run_bytes_freed
9,009 -> 9,010, run_evac_bytes 9,340,096 -> 9,271,280 and run_alloc_bytes
289,288,212 -> 289,219,396. Why #2014's unescape changed whether that pop
copies is not traced; the copy was wrong whichever change exposed it.

The test costs a list the repair is about to copy two loads and a branch.
Asked before the survival test it ran on every list the repair visited, and
deepbench, which repairs a captured list of lists on every bind, read 192,014
instructions more; asked after it, only a list that does not survive pays.
CI's rows: work_runbench 1,137,672,504 -> 1,137,617,275 and
work_deepbench 406,723,866 -> 406,819,866, with work_pendbench 194,088,527 ->
194,089,023, work_widebench 28,740,070 -> 28,740,098, work_oneshot 12,776,869
-> 12,776,876, work_encodebench 2,464,678,221 -> 2,464,678,011 and
work_livebench 1,370,942,122 -> 1,370,942,108. Every program's machine code
grows 16 bytes (runbench text=741,081 -> 741,097), and the corpus's text
5,283,518 -> 5,283,966.
codegen_instructions_release 453,685,379 -> 453,686,073 counts the larger
runtime object; codegen_instructions_dev 116,000,233 -> 116,000,561 is that
and the link directory's order, which #2016 fixes.

A rewrite of the unescape loop was measured and dropped on the way. Reading
a string's last few bytes as a masked word, and looking escapes up in a table
instead of a switch, saved the run program 394,787 instructions (0.035%).
About 65 of the 217 instructions an escaped string costs are the call's fixed
work, and with a backslash every four or five bytes the loop does about the
same work per run whatever its width.

## 2026-10-20 — a plain JSON key goes straight to its colon

An object key used to go through `parse_string`: `builtin_plain_string`, a
dispatch on what it answered, a `parsed` record holding the position after the
key and the key itself, and then `obj_key` taking that record apart to look
for the colon. A key almost never has an escape in it, so `obj_key_start` now
calls `builtin_plain_string` itself, and `key_at`'s string arm works out the
position after the closing quote and goes straight to `obj_key_end`. Anything
else goes through `plain_at` and `obj_key` as before, so an escaped key, a
`\u` escape, an unterminated key and bad utf-8 are read and reported by the
code that read them before.

The same shape for a string value after a colon read the run program 14.8M
instructions higher than keys alone, because the extra byte dispatch in front
of every value cost the numbers more than it saved the strings, and was
dropped.

Over 100,000 fuzzed documents decoded byte for byte alike on native, the
interpreter and #2014's build, and the harness went red when the key's end
was put one byte early.

CI's rows. The work vein: work_runbench 1,137,672,504 -> 1,104,562,811
(-2.91%), work_jsonbench 812,108,477 -> 763,196,627 (-6.02%), work_oneshot
12,776,869 -> 12,450,454 and work_livebench 1,370,942,122 -> 1,370,617,065.
The interpreter reads its corpus's keys the same way: interp_instructions
484,702,222 -> 463,217,423 and interp_allocs 539,460 -> 502,624, with
interp_peak_bytes 614,115 -> 617,367. The library grows by the two arms of
`key_at`, and every program that imports std/json compiles them: compile_instructions
24,089,825 -> 24,321,293, entry_instructions 75,139,973 -> 75,148,170,
library_instructions 74,835,835 -> 74,843,912, compile_allocs 13,382 ->
13,465, compile_peak_bytes 572,289 -> 575,715 and front_end_visits 6,938 ->
7,063. The decoder's emitted code reads
emitted_defines 134 -> 136, emitted_calls 1,249 -> 1,282, emitted_branches
1,336 -> 1,374 and emitted_lines 12,690 -> 13,010, and the other programs
that decode emitted_other_defines 2,148 -> 2,154, emitted_other_calls 18,298
-> 18,397, emitted_other_branches 17,145 -> 17,259 and emitted_other_lines
156,673 -> 157,633. Machine code: text 5,283,518 -> 5,286,574, 880 bytes on
each program that decodes JSON and 416 on runbench. In the tab the run falls
and the compile rises with the library: browser_run_instructions 35,361,799
-> 34,316,072, browser_compile_instructions 408,301,693 -> 413,585,272 and
browser_compile_peak_bytes 749,207 -> 757,553.

On one branch with #2014 and #2018, and on main with #2016's fixed link order,
CI reads work_runbench 1,104,507,218, work_jsonbench 763,196,627,
work_oneshot 12,450,839, work_livebench 1,370,616,792 and work_encodebench
2,464,678,011; the corpus's machine code is text 5,287,022, the two runtime
changes' 32 bytes a program on top of the key's; and
codegen_instructions_dev reads 116,001,189 under the fixed order, the three
changes' cost over main's 115,822,620.

## 2026-10-20 — the ratchet in four files

`scripts/ratchet/ratchet.kso` was 6,316 lines, the largest source file in the
tree (#2021). It is now a directory module, `scripts/ratchet/ratchet/`: the
types, helpers and runner in `ratchet.kso`, and the three chains of rows,
`rows_a`, `rows_b` and `rows_c`, in files of their own. Every line is where it
was, in the same order, with one import of std/list added to each new file.
The tests that read the rows now read every file of the module through
`tests/support/ratchet.rs`, and the one that stages the ratchet copies the
whole directory. `every_ratchet_row_is_run` went red when `rows_c` stopped
joining its last row, so it reads the new files.

## 2026-10-20 — an interpolated int counts its digits once

`"{i}"` on an int goes through `k_b_render_value`, which sized the string
with `ryu_declen` and then called `k_itoa`, which ran the same ladder of
comparisons again before writing the digits. The run program's pend phase
renders 200,000 ints this way. The digit writer is now `k_digits_into`,
which takes the end its caller worked out, and `k_itoa` is a short wrapper
over it. The other kinds of value move to an out-of-line function, so the
int path no longer saves six registers for paths it never takes. Here the
run program read 1,104,506,031 -> 1,100,103,349 instructions (-0.40%), and
`k_b_render_value` 18,309,116 -> 13,884,015. CI's rows: work_runbench
1,104,507,218 -> 1,100,104,536 (-0.40%) and work_pendbench 194,089,023 ->
176,534,752 (-9.04%), the benchmark the run program's pend phase is drawn
from; the rest move by layout alone, work_oneshot 12,450,839 -> 12,451,007,
work_livebench 1,370,616,792 -> 1,370,617,002, work_encodebench
2,464,678,011 -> 2,464,677,731, work_basket 31,162,888 -> 31,162,668,
work_widebench 28,740,098 -> 28,740,070 and work_deepbench 406,819,866 ->
406,819,829. Machine code shrinks: text 5,287,022 -> 5,281,870.
codegen_instructions_dev 116,001,189 -> 116,002,877 and
codegen_instructions_release 453,686,073 -> 453,797,745 count the runtime
object's new shape. In the tab browser_run_instructions reads 34,316,072 ->
34,136,382.

The micro golden `an_interpolated_int_renders_at_every_width` renders each
width where the digit count changes, signs, and both ends of the word, and
reads `+1` where it wants `-1` when the sign is written wrong.

Replacing the `floor()` in the float renderer's integral test with a round
trip through int64 was measured and dropped: glibc picks its SSE4.1 `floor`
at load time, which costs about three instructions, and the round trip made
`k_b_append_rendered` 755,725 instructions dearer.

Writing an int straight into the builder in `k_b_append_rendered`, with the
float path moved out of line the same way, was measured and dropped too: it
read the run program 1,100,103,349 -> 1,104,459,315 and encodebench
2,464,678,011 -> 2,484,039,482, because every float paid the extra call
level and floats are half of what the encoder renders.

## 2026-10-21 — a carried record returns in three words

A two-field record whose first field is an int, like the decoder's
`parsed`, goes back to its caller in registers. It used to take two words:
the int shifted above the value's tag in the first, the value's payload in
the second. Every pack checked that the int survived the shift, and every
unpack branched on a marker for the record built on the heap when it did
not. On the run program that was 725,703 checked packs and 1,791,207
unpacks (#2024). The record now takes three words. Both tags share the
first, the value's payload is the second and the int's word is the third,
so every int fits, a bignum crosses as its pointer, and nothing spills.
`k_parsed_spill` and the two readers of a spilled record are gone. A
failure still travels in the first two words, which read as a `%KValue`.

The first build read the run program 1,100,103,349 -> 1,093,152,335.
`obj_key_end` grew by 3,136,716: with the int's tag read out of a word,
LLVM no longer knew it was an int's, and tested it for a thunk on every
record. An `llvm.assume` that the tag is K_INT or K_BIG gives that back.
Here the run program read 1,100,103,349 -> 1,085,159,551 (-1.36%), and
every decoder function in the profile fell.

CI's rows: work_runbench 1,100,104,536 -> 1,085,160,738 (-1.36%),
work_jsonbench 763,196,627 -> 749,951,027 (-1.74%), work_oneshot
12,451,007 -> 12,362,488, work_widebench 28,740,070 -> 28,580,079,
work_livebench 1,370,617,002 -> 1,370,528,510 and work_encodebench
2,464,677,731 -> 2,464,443,931. Machine code shrinks in every benchmark:
text runbench 741,161 -> 737,929 and jsonbench 382,825 -> 379,049, the
rest by 240 to 3,840 bytes. The decoder's emitted code reads calls 1,282 ->
1,261, branches 1,374 -> 1,332 and lines 13,010 -> 12,909, and the other
programs that decode JSON fall the same way. codegen_instructions_dev
116,002,877 -> 115,990,552, codegen_instructions_release 453,797,745 ->
453,689,156 and emit_instructions 6,233,826 -> 6,232,336. In the tab,
browser_compile_instructions 413,585,272 -> 412,332,402 and
browser_compile_peak_bytes 757,553 -> 752,985 fall, and
browser_run_instructions 34,136,382 -> 34,370,238 rises by 0.69%. The
tab runs the emitter's IR through its own translator, which drops the
assume's call but computes the test that feeds it, and returns a struct
through memory, now three words of it. How the rise divides between those
has not been measured. The welfare sum rose, and the floor moves 91.78 ->
91.84.

The x86 rewrite to `preserve_nonecc` flattens a struct parameter into
words, and it assumed every struct was two. A `%parsed` parameter lost its
third word, the decoder's tail calls stopped matching, and the run program
ran out of stack. `flat_words` now gives each type's width, and the other
counts read from it.

A constant is built once into a cell that holds a `%KValue`, and its
symbol returns what the cell holds. A constant whose body built a carried
record still had a builder returning `%parsed`, and callers that asked for
one: the cell kept the two words as if they were a value, and it read back
because the shapes matched. Three words do not, and
`a_constant_naming_a_record_constant` printed 14, the thunk tag, where it
prints 1. A constant now always returns a `%KValue`, which costs nothing,
since the cell boxed it anyway, and the sample leaves `ir_verifier`'s list
of carried samples.

The micro golden `an_int_of_any_width_crosses_in_its_own_word` sends ints
either side of the old 56-bit line, both ends of the word, a bignum and a
word that overflows into one through four groups that return the record by
value. It joins the samples `ir_verifier` requires to stay carried. The
ratchet's two mutations for the old shape, an unsigned shift back and a
wide int packed anyway, went with the code they broke. Two new ones drop
the bignum tag on the way in and on the way back, and the new golden reads
a pointer as a number under each.

Passing the key's closing `":` down into `escape_onto`, so that each arm
of its test could append its own tail, was measured and dropped. The run
program read 1,103,713,022 against 1,085,159,551. The tail arrived as a
string value, and the emitter's one-word store for a short literal no
longer applied. Bound to a name instead of written in the arms, the plain
arm read `acc` twice, the append stopped writing in place, and the run
program read 1,365,286,224.

## 2026-10-21 — a position read beside its byte is in range, and a length

The decoder hands a byte on together with the position it was read at:
`array_delim cs cs[p + 1] (p + 1) acc`. In an arm that takes the byte as a
byte, either through a literal or after an arm that took `none`, the read
succeeded, so the position indexes `cs`. Adding one to it cannot leave the
machine word, but each `p + 1` carried an overflow test and a way to the
bignum body. `src/read_at.rs` finds the groups where every call passes a
collection, a read out of it and the read's position, the collection and
the index written alike in both places. A partial, a piped call or a bare
mention of the name refuses the group. In each arm that took the byte as a
byte, the emitter tells LLVM the position is an int whose word lies between
-2^62 and 2^62, and LLVM drops the tests. A read or a position that may
arrive deferred is left alone, because a thunk passes the `none` arm's tag
test and may still force to `none`.

Six decoder groups qualify: `array_delim`, `array_open`, `obj_delim`,
`obj_key_start`, `obj_key_end` and `obj_open`. On the run program
15,265,858 `jo`/`jno` executed before and 8,754,624 after, and here it read
1,085,159,551 -> 1,073,408,863 (-1.08%). LLVM inlined differently with the
tests gone: `parse_value` fell 42.3M and `array_delim` rose 20.8M, and
`parse_number` left `parse_value` as a call of its own.

`assume_length` said a length is never negative. It now says a length is
below 2^62, and the inline `length` load uses it, so a guard like
`return acc if i < 1 or length xs < i` lets LLVM step `i + 1` without a
test. Here the run program read 1,073,408,863 -> 1,068,720,545 more, with
`sha256/digested` and json's encode loops the gainers.

CI's rows: work_runbench 1,085,160,738 -> 1,068,721,732 (-1.51%),
work_jsonbench 749,951,027 -> 731,748,677 (-2.43%), work_digestbench
5,992,543 -> 5,241,183 (-12.5%), work_livebench 1,370,528,510 ->
1,356,883,942, work_encodebench 2,464,443,931 -> 2,456,797,406,
work_oneshot 12,362,488 -> 12,207,627, work_widebench 28,580,079 ->
28,571,443 and work_basket 31,162,668 -> 31,152,439. work_scanbench rises
274,236 -> 274,277, by 41. Machine code shrinks in every benchmark: text
runbench 737,929 -> 715,289 and jsonbench 379,049 -> 364,137, the rest by
256 to 24,576 bytes. codegen_instructions_release 453,689,156 ->
422,291,078 (-6.9%) and codegen_instructions_dev 115,990,552 ->
115,922,261 fall with the code LLVM no longer keeps. emit_instructions
6,232,336 -> 6,283,165 rises by the analysis and the assumes it writes.
compile_instructions 24,321,293 -> 24,286,340, entry_instructions
75,148,170 -> 75,053,355, library_instructions 74,843,912 -> 74,749,069 and
interp_instructions 463,217,423 -> 463,213,463. In the tab,
browser_run_instructions 34,370,238 -> 34,153,178 and
browser_compile_peak_bytes 752,985 -> 735,031 fall, and
browser_compile_instructions 412,332,402 -> 413,098,267 rises by 765,865.
The assumes are calls in the emitted IR, so the counts that read them rise:
emitted_calls 1,261 -> 1,394 and emitted_lines 12,909 -> 13,078 for the
decoder, emitted_other_calls 18,298 -> 19,150 and emitted_other_lines
157,080 -> 158,441 for the other programs, and the module compile's
module_calls 109 -> 111 and module_lines 1,131 -> 1,135. Emitted branches
fall in every program that decodes JSON. The welfare sum rose, and the
floor moves 91.84 -> 91.92.

The micro golden `a_position_read_beside_its_byte_stays_exact` runs one
group linked at every call, from positions at both ends of the word, past
it and below zero, and one group whose second call passes a literal byte
beside the largest word. With the call sites' intersection taken out, that
second group is linked anyway and a release build's `p + 1` wraps to
-9223372036854775808 where the interpreter prints 9223372036854775808. The
dev tier never acts on the assume, so the ratchet's row reads the
release-built corpus. The ratchet carries that as
`a_read_position_assumed_past_a_literal`, and `a_read_position_left_unbounded`
drops every link, which the emitted goldens read.

A dead-value pass in `src/ir_wasm.rs`, which drops pure instructions nothing
reads and counts an `llvm.assume` as reading nothing, was measured and
dropped. It took browser_run_instructions 34,370,238 -> 33,477,523 (-2.60%)
against browser_compile_instructions 412,332,402 -> 414,726,069 (+0.58%),
and welfare fell below the floor: the browser run term is saturated, with a
baseline of 2,331,914,518, so the compile cost outweighs the run's gain.

TBAA tags on the IR append helpers, for the header, the buffer's frontier
word, its bytes and the `k_site` stores, were measured and dropped: the run
program's binary came out byte-identical. The three appends `encode_map`
makes for each pair cannot share their header loads because each append's
slow path may hand back a different builder, so the next append's header
pointer is a merge of the two paths. Aliasing was not what kept them apart.
