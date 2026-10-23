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

## 2026-10-21 — a JSON key goes out in one claim

lib/json wrote an object's key as three appends around a call: the opening
quote after the brace or comma, `escape_onto` for the key, and the closing
quote and colon as one word. Each append made its own ownership and room
tests on the builder. `builtin_append_key acc k` now writes `"k":` in one
call. The decoder shares every string of four to seven bytes as a token, and
each token records whether its bytes need escaping, so for a clean token the
emitter's inline path claims the builder once and writes three stores: the
quote, the token's whole eight-byte word, and `":` over the bytes past the
key. Any other key, and any build that counts, goes to the C, which writes
the same bytes the long way. The interpreter writes the same bytes.

The inline path finds a token's clean flag from where its bytes sit in the
token store, dividing by the size of a token. A token is a `KStr` and eight
bytes, which is 24 bytes natively and 20 on wasm32, where a pointer is four.
The first version used 24 everywhere. Two `WASM32_LINES` rows now give the
tab its own bound and divisor. `retarget_wasm32` searched only
`getelementptr` lines for rows to rewrite, so the clang-built wasm32 harness
kept the native numbers while the playground's translator, which matches
every line, took the new ones; it now searches for each row by its own text.
The micro golden `a_dirty_key_beside_a_clean_one_is_escaped` places `ke"y`
in the slot whose 24-byte reading on wasm32 is `ammo`'s. Before the rows,
the wasm32 build printed the quote unescaped.

Measured here, the run program read 1,068,720,545 -> 1,049,790,947
(-1.77%) with the same output, all of it in `encode_map`; the C path ran
82,800 instructions. The interpreted corpus read 463,213,463 -> 402,112,576
instructions and 502,624 -> 423,390 allocations, because the interpreter's
builtin writes into the builder where `escape_onto` and three appends ran as
kanso.

CI's rows: work_runbench 1,068,721,732 -> 1,049,792,134 (-1.77%),
work_livebench 1,356,883,942 -> 1,272,755,954 (-6.2%), work_oneshot
12,207,627 -> 11,997,046 (-1.7%) and work_encodebench 2,456,797,406 ->
2,456,796,902; jsonbench and widebench carry their own copy of lib/json and
hold. interp_instructions 463,213,463 -> 402,108,616 (-13.2%),
interp_allocs 502,624 -> 423,390 and interp_peak_bytes 617,367 -> 616,817.
The C path adds 3,264 bytes of text to every program (text runbench 715,289
-> 717,129, jsonbench 364,137 -> 367,401), and codegen_instructions_dev
115,922,261 -> 115,971,597 and emit_instructions 6,283,165 -> 6,307,623 rise
with the helper. codegen_instructions_release 422,291,078 -> 422,279,602,
compile_instructions 24,286,340 -> 24,227,124, entry_instructions
75,053,355 -> 75,007,797, library_instructions 74,749,069 -> 74,686,591,
compile_allocs 13,465 -> 13,434, compile_peak_bytes 575,715 -> 575,051 and
front_end_visits 7,063 -> 7,047 fall. The
counters: perm_allocs falls by one in run (101 -> 100), live (8 -> 7) and
oneshot (9 -> 8), the closing pair's literal, and append_fast rises in the
same three (run 5,735,070 -> 5,737,050, live 25,489,200 -> 25,498,000,
oneshot 63,723 -> 63,745). emitted_lines 13,078 -> 13,089 for the decoder and
+11 in every other program, +32 in runbench (56,807 -> 56,839), oneshot and
livebench, where emitted calls fall by three (runbench 7,380 -> 7,377) and
emitted branches rise by three (6,434 -> 6,437); in all, emitted_other_lines
158,441 -> 158,647 and emitted_other_branches 16,954 -> 16,963. text sums to
5,124,302 -> 5,165,726. The compile corpus's lines 1,412 -> 1,456 and the
module compile's module_lines 1,135 -> 1,146 carry the same comment. In the tab,
browser_run_instructions 34,153,178 -> 33,548,279 (-1.8%) and
browser_compile_instructions 413,098,267 -> 412,842,483 fall, and
browser_compile_peak_bytes rises 735,031 -> 738,682. The welfare sum rose,
and the floor moves 91.92 -> 92.03.

The micro golden `a_key_goes_out_quoted_with_its_colon` decodes and encodes
keys at the edges twice, so the second pass reads shared tokens: empty, one
byte, seven and eight bytes, a two-byte character, an escaped `\u00e9`, and
tokens with a quote, a backslash or a tab. With the inline path's length
short by one, the clean keys lost their colons; with the C's colon replaced
by a quote, the other keys did. Two ratchet rows:
`a_key_claimed_one_byte_at_a_time` sends every key to the C and turns the
emitted gate red, and
`a_token_measured_at_its_native_size_on_wasm32` gives the tab the native
numbers back and turns the translator's micro corpus red. The two byte-pair
rows went blind: the encoder no longer writes two byte appends in a row,
since the brace or comma before a key is one append now. The micro fixture
`two_bytes_appended_at_once` appends a backslash and a letter itself, and
`a_byte_pair_appended_one_at_a_time` is gated by a perf_ratchet spec,
`two_byte_appends_are_one_call`, that reads the merged call in the emitted
code; each mutation turns its witness red. The room-test spec
counts ten frontier branches now. The mem rows move because a clean key no
longer passes through `escape_onto`: `find2_calls` falls to 0 in three
programs, and `perm_allocs` falls by one, the closing pair's literal.

Measured and dropped on the way, recorded so they stay declined:

- A fused `builtin_append_key acc lead k` that also wrote the brace or comma,
  as a C entry. The run program read 1,068,720,545 -> 1,181,687,667: the C
  cost 135 instructions a call against the inline path's check and copy,
  and `k_region_pop` grew 32.4M because `encode_map` then handed its value
  to `encode_onto` directly.
- Passing the key's tail into `escape_onto` (+18.5M) and a copy of the key
  arms (+19M).

Open: `encode_map` still copies a map's keys and values into two columns per
map, 25M on the run program with the region around it.

## 2026-10-21 — a map is written from its entries in place

`encode_map` took a map's two columns, `keys m` and `values m`, which
`k_b_columns` built as two lists in one allocation per map, and indexed both.
Three library builtins now read a map where it stands: `entry_count m`, `key_at
m i` and `value_at m i`. A map's sorted view is `[k v k v ...]`, so where the
view is built the count is a field and each read is a bounds test and one load,
written inline; a missing or marked view goes to the C, which builds the view
first. The encoder takes the map and its count and reads each pair by position,
and no list is made.

The count stays an argument at the call in `encode_onto`, `encode_map acc m
(builtin_entry_count m)`, where the columns were. The beat analysis treats
`entry_count` as allocating, since it may build the view, so the region around a
map's encoding stays one per map. Two other shapes were measured on the way.
With the count bound inside `encode_map`, the pair loop became a beat loop and
`k_beat_pop` cost 17.9M more. With `entry_count` treated as pure, the regions
went (-10.7M) but the output builder grew by an arena copy at each doubling
instead of `realloc` (+24.7M memcpy): inside the region its header sits below
the innermost mark and stays in the malloc regime, and without it the header is
above the encode loop's mark.

A map past eight entries is a B-tree in the interpreter, where reaching the i-th
entry walks i of them. The interpreter keeps the entries in order for the last
such map `key_at` or `value_at` read, holding the map weakly, so the count `put`
asks before writing in place is unchanged, and a `put` that writes in place at a
site linearity proved empties the cache first. No fixture reaches that write
after a read: linearity refuses the `put` that follows a read of the same map,
natively and in the interpreter. The native side-view test is in the same
position: a side forms only when an in-place `put` lands in the middle of a
built view of 64 entries or more, and three loop shapes tried here did not write
in place.

Measured here, on top of the key builtin, the run program read 1,049,790,947 ->
1,039,464,472 (-0.98%) with the same output: `k_b_columns` (25.0M) is gone and
`encode_map` rose 14.3M with the reads.

CI's rows: work_runbench 1,049,792,134 -> 1,039,465,659 (-0.98%), work_livebench
1,272,755,954 -> 1,219,792,784 (-4.2%) and work_oneshot 11,997,046 -> 11,902,783
(-0.79%). work_encodebench rises 2,456,796,902 -> 2,456,797,749, by 847, with
the runtime's layout; it carries its own copy of lib/json. The allocation
counters fall where objects are encoded: run allocs 1,696,484 -> 1,447,994,
alloc_bytes 289,219,396 -> 249,236,356 and sh_buf 119,198,704 -> 87,167,344;
live allocs 1,118,314 -> 13,914, alloc_bytes 413,053,904 -> 235,351,504 and
sh_buf 143,161,328 -> 799,728; oneshot allocs 10,292 -> 7,531, alloc_bytes
2,234,774 -> 1,790,518 and sh_buf 1,155,632 -> 799,728. interp_instructions
402,108,616 -> 401,548,849 and interp_allocs 423,390 -> 420,781 fall, and
interp_peak_bytes rises 616,817 -> 616,944, the cached entries of a large map.
The three inline paths add 320 bytes of text to every program and about 3,400 to
the three that encode objects (text runbench 717,129 -> 720,681), and
codegen_instructions_dev 115,971,597 -> 116,025,482,
codegen_instructions_release 422,279,602 -> 422,550,641, emit_instructions
6,307,623 -> 6,341,758, entry_instructions 75,007,797 -> 75,050,737 and
library_instructions 74,686,591 -> 74,724,744 rise with them.
compile_instructions 24,227,124 -> 24,218,897, compile_allocs 13,434 -> 13,427
and front_end_visits 7,047 -> 7,010 fall, and compile_peak_bytes rises 575,051
-> 575,175. Emitted code: the decoder's emitted_lines 13,089 -> 13,098;
runbench, oneshot and livebench gain 5 defines and lose 27 branches (runbench
56,839 -> 56,741 lines, 6,437 -> 6,410 branches); the compile corpus's lines
1,456 -> 1,492 and module_lines 1,146 -> 1,155; in all, emitted_other_defines
2,154 -> 2,169 and text 5,165,726 -> 5,179,630. In the tab,
browser_run_instructions 33,548,279 -> 32,722,014 (-2.5%) and
browser_compile_peak_bytes 738,682 -> 735,564 fall, and
browser_compile_instructions rises 412,842,483 -> 412,895,911. The welfare sum
rose, and the floor moves 92.03 -> 92.07.

The micro golden `a_map_is_encoded_from_its_entries_in_place` covers an empty
map, two maps past eight entries read one after the other, and a small and a
large map read, extended by a `put` and read again; with the interpreter's cache
keyed by nothing, the second large map printed the first's entries. The mem
fixture that pinned the fused columns through the encoder is renamed
`a_written_map_allocates_no_columns` (3,022 -> 2,022 allocations), and
`a_maps_two_columns_are_one_allocation` now calls `keys` and `values` itself, so
the fusion keeps a witness; `the_two_columns_asked_apart` still turns the mem
vein red. Its rows are re-based, since the name now holds a different program:
a_maps_two_columns_are_one_allocation_allocs 3,022 -> 4,005, a thousand pairs of
lists and the maps they come from; and
a_maps_two_columns_are_one_allocation_append_fast 12,007 -> 0,
a_maps_two_columns_are_one_allocation_append_rendered 2,000 -> 0,
a_maps_two_columns_are_one_allocation_push_mut_fast 995 -> 0,
a_maps_two_columns_are_one_allocation_bytes_freed 8 -> 0 and
a_maps_two_columns_are_one_allocation_utf8_zerocopy 1 -> 0, since it encodes
nothing. The book's chapter 08 panel showed the map going in as its two columns,
and book_quotes refused it once the encoder stopped naming `keys` and `values`;
it now shows the map going in whole with its count. The ratchet row
`a_map_entry_read_through_the_c` sends `key_at` to the C and turns the emitted
gate red.

## 2026-10-22 — a JSON string value goes out in one claim

The encoder wrote a string value as an opening quote, `escape_onto` and a
closing quote. `escape_onto` scanned the string for a byte that needs escaping
and either appended it as it stands or handed it to `append_escaped`, which
scanned it again as it escaped it. On the run program's document 1,773 of every
2,114 string values need escaping. They average 16 bytes with 2.6 escapes each,
so most values paid for both scans. `builtin_append_string acc s` now writes the
quotes and the escaped bytes in one call. Where the builder is at its frontier
with room for the worst case, the C writes the quote, escapes straight into the
builder and writes the closing quote. A value the decoder shared as a clean
token takes the same inline path as a key, with a closing quote where a key has
`":`. The interpreter's arm writes the same bytes.

Measured here, on top of the map read, the run program read 1,039,464,472 ->
1,028,684,506 (-1.04%) with the same output, and the interpreted corpus read
401,548,849 -> 384,232,761 instructions and 420,781 -> 400,588 allocations.

CI's rows: work_runbench 1,039,465,659 -> 1,028,685,693 (-1.04%), work_livebench
1,219,792,784 -> 1,172,094,931 (-3.9%) and work_oneshot 11,902,783 -> 11,785,186
(-0.99%); work_encodebench 2,456,797,749 -> 2,456,797,637. The counters:
find2_calls falls in run (1,667,049 -> 1,488,669), live (807,837 -> 15,037) and
oneshot (17,019 -> 15,037), since a value no longer passes through escape_onto's
scan, and append_fast falls in the same three, the quote appends no longer made
one at a time: run_append_fast 5,737,050 -> 5,328,270, live_append_fast
25,498,000 -> 23,681,200 and oneshot_append_fast 63,745 -> 59,203.
interp_instructions 401,548,849 -> 384,232,761, interp_allocs 420,781 -> 400,588
and interp_peak_bytes 616,944 -> 608,798. The library is smaller to compile:
compile_instructions 24,218,897 -> 23,713,618, entry_instructions 75,050,737 ->
74,424,236, library_instructions 74,724,744 -> 74,085,135, compile_allocs 13,427
-> 13,179, front_end_visits 7,010 -> 6,914 and compile_peak_bytes 575,175 ->
565,175, and codegen_instructions_release 422,550,641 -> 422,357,890. The
runtime's C entry adds about 5,000 bytes of text to every program (text
jsonbench 367,721 -> 372,777, runbench 720,681 -> 721,209), and
codegen_instructions_dev 116,025,482 -> 116,064,263 and emit_instructions
6,341,758 -> 6,364,404 rise with the path. Emitted code: the decoder's
emitted_lines 13,098 -> 13,102; runbench, oneshot and livebench lose
escape_onto's steps (runbench defines 782 -> 778, calls 7,376 -> 7,357, lines
56,741 -> 56,666) and gain 5 branches (6,410 -> 6,415); the compile corpus's
lines 1,492 -> 1,508 and module_lines 1,155 -> 1,159; in all,
emitted_other_branches 16,882 -> 16,897 and text 5,179,630 -> 5,236,430. In the
tab, browser_run_instructions 32,722,014 -> 32,241,456 (-1.5%),
browser_compile_instructions 412,895,911 -> 406,640,369 (-1.5%) and
browser_compile_peak_bytes 735,564 -> 732,776 fall. The welfare sum rose, and
the floor moves 92.07 -> 92.13.

The micro golden `a_string_value_goes_out_between_its_quotes` decodes and
encodes string values at the edges twice: empty, one byte, seven and eight
bytes, `été`, an escaped `\u00e9`, and tokens with a quote, a backslash or a
tab. With the inline path's length short by one, the clean tokens lost their
closing quotes; with the C's closing quote replaced, the values it wrote showed
the replacement. The ratchet row `a_string_value_claimed_three_times` writes
values the old way and turns the emitted gate red. The book's chapter 08 panel
showed the string arm as `_escape_onto` and called its sample the library's
escape functions verbatim, which stopped being true when escaping moved to the
runtime; it now shows the one call and says what the runtime does.

The ratchet row `a_clean_token_scanned_anyway` went blind with this change.
It turns off the clean-token skip in `find2_below`, which served
`escape_onto`, and the run program's values no longer reach `escape_onto`.
encodebench's own copy of lib/json still sends every string through it, so the
row now gates on the encode vein, where the mutation moves `find2_calls`
803,275 -> 4,200,475.

## 2026-10-22 — a map entry's key and value are read with one test

`entry_onto` reads a pair as `pair_onto acc (builtin_key_at m i)
(builtin_value_at m i)`, and each read tested the map's view on its own: the
tags, the view pointer and its mark, and the bounds. Where one call's
arguments read a key and then a value of the same local map at the same local
position, the emitter now calls `k_b_entry_pair_fast` once. It tests the view,
returns the key and stores the value into a slot in the caller's frame, which
the call's next argument loads. A view that is missing or marked goes to the
C for both reads, as each read did alone. The pair is found in both places the
emitter writes a call's arguments, the plain call and the tail call, and
`entry_onto`'s call is a tail call.

Measured here, the run program read 1,028,684,506 -> 1,018,405,282 (-1.0%)
with the same output.

CI's rows: work_runbench 1,028,685,693 -> 1,018,406,469 (-1.0%),
work_livebench 1,172,094,931 -> 1,126,408,749 (-3.9%) and work_oneshot
11,785,186 -> 11,669,620 (-1.0%). The three programs that encode objects lose
one define, two calls and three branches each and are smaller (text runbench
721,209 -> 720,921, oneshot 380,633 -> 380,393, livebench 384,409 ->
384,169; emitted runbench lines 56,666 -> 56,634, oneshot 13,898 -> 13,866,
livebench 14,221 -> 14,189). Every other program carries the helper's two
lines (emitted_lines 13,102 -> 13,104 for the decoder, and +2 in the others),
the compile corpus's lines move by two each (recursion 449 -> 451, dispatch
348 -> 350, guards 305 -> 307, records 406 -> 408; lines 1,508 -> 1,516) and
module_lines 1,159 -> 1,161.
codegen_instructions_dev 116,064,263 -> 116,067,612,
codegen_instructions_release 422,357,890 -> 422,361,029 and
emit_instructions 6,364,404 -> 6,377,706 rise with them. In the tab,
browser_run_instructions 32,241,456 -> 31,991,504 and
browser_compile_instructions 406,640,369 -> 406,222,752 fall, and
browser_compile_peak_bytes rises 732,776 -> 733,266. The welfare sum rose, and
the floor moves 92.13 -> 92.17.

The map golden `a_map_is_encoded_from_its_entries_in_place` is the pair's
witness for its bytes: with the value loaded from the key's slot, every value
printed as its key. The ratchet row `an_entry_read_twice` never sees the pair
and turns the emitted gate red. The row `a_map_entry_read_through_the_c` patched
`key_at_fast` and read BLIND once the pair took every key the encoder reads;
it now sends the pair to the C, which reads the run program 935,900,947 ->
967,870,853, and it gates the instructions vein.

## 2026-10-22 — a builder at the arena's frontier grows where it stands

A builder that outgrew its buffer took a new one and copied, in the arena or
from malloc depending on where its header sat against the innermost mark. When
the header lies above that mark, the buffer is arena storage, and when the
buffer is also the last thing the arena handed out, it now grows by moving
the frontier past it: no new buffer and no copy. A header above the innermost
mark was allocated after it, and a buffer at the frontier after the header, so
the extension dies with the header at the same rewind. The grow also requires
the header to own the buffer's frontier, since a grow is how an append reaches
a buffer another header has written past. The extension's bytes count in
`alloc_bytes`.

That made a second change pay. On 2026-10-21 `entry_count` was left
allocating to the beat analysis, which kept a region around each map the
encoder descends into: inside the region the builder's header sat below the
mark and grew by realloc, and without it the builder grew in the arena by
doubling and copying (+24.7M memcpy). `entry_count` is now pure, as `key_at`
and `value_at` are. The view it may build comes from malloc, and only a sort's
scratch index lands in the arena, where the enclosing loop's rewind takes it.
The region is gone, and the builder now grows in place.

Measured here, the run program read 1,018,405,282 -> 999,948,734 (-1.81%)
with the same output: `k_region_pop`'s 10.7M is gone, and `encode_list` and
`encode_map` fall 3.5M and 3.8M.

CI's rows: work_runbench 1,018,406,469 -> 999,949,921 (-1.81%),
work_livebench 1,126,408,749 -> 1,043,615,776 (-7.3%), work_oneshot
11,669,620 -> 11,443,335 (-1.9%) and work_encodebench 2,456,797,637 ->
2,456,796,655. work_widebench rises 28,571,443 -> 28,619,441 (+0.17%); it
carries its own copy of lib/json and grows its builders in the arena too. The
run program's held_peak_bytes falls 200,978 -> 111,920, and run allocs
1,447,994 -> 1,446,734, alloc_bytes 249,236,356 -> 221,966,536, append_grow
1,260 -> 1,080, append_fast 5,328,270 -> 5,328,540, bytes_malloc 9,140 ->
7,880 and bytes_freed 9,010 -> 7,750. The one-shot and live programs'
builders never leave the arena: held_peak_bytes 200,978 -> 0 in both,
bytes_malloc and bytes_freed 14 -> 0 and 5,600 -> 0, oneshot allocs 7,531 ->
7,517, alloc_bytes 1,790,518 -> 1,487,520, append_grow 14 -> 12, append_fast
59,203 -> 59,206; live allocs 13,914 -> 8,314, alloc_bytes 235,351,504 ->
114,152,304, append_grow 5,600 -> 4,800, append_fast 23,681,200 ->
23,682,400. Without the region runbench, oneshot and livebench make three
fewer calls (runbench calls 7,355 -> 7,352, lines 56,634 -> 56,636; oneshot
calls 1,484 -> 1,481, lines 13,866 -> 13,868; livebench calls 1,523 -> 1,520,
lines 14,189 -> 14,191). The runtime's text grows 336 bytes in every program
(text jsonbench 372,777 -> 373,113) and shrinks where the region went (text
runbench 720,921 -> 719,977, oneshot 380,393 -> 379,641, livebench 384,169 ->
383,177). codegen_instructions_dev 116,067,612 -> 116,068,279 and
codegen_instructions_release 422,361,029 -> 422,367,718 rise, and
emit_instructions 6,377,706 -> 6,377,307 falls. In the tab,
browser_run_instructions 31,991,504 -> 31,623,429,
browser_run_peak_bytes 1,441,792 -> 1,376,256, browser_compile_instructions
406,222,752 -> 406,034,117 and browser_compile_peak_bytes 733,266 -> 732,887
fall. The welfare sum rose, and the floor moves 92.17 -> 92.25.

The frees fall with the mallocs they answered: run_bytes_freed 9,010 -> 7,750,
oneshot_bytes_freed 14 -> 0, live_bytes_freed 5,600 -> 0 and
a_literal_appended_across_a_rewind_bytes_freed 40 -> 0. Over all programs,
emitted_other_lines 158,180 -> 158,186 and text 5,235,662 -> 5,236,670.

The mem corpus moves where builders grow in the arena:
`a_literal_appended_across_a_rewind` allocs 136 -> 96, alloc_bytes 11,920 ->
9,040, bytes_malloc and bytes_freed 40 -> 0, held_peak_bytes 152 -> 0;
`three_storage_regimes_share_one_append` allocs 1,640 -> 1,040, alloc_bytes
186,274 -> 114,274; `append_of_a_slice_boxes_nothing` allocs 14 -> 12,
alloc_bytes 2,396 -> 2,156. The micro golden
`a_builder_grows_where_it_stands_until_something_follows_it` grows a builder
in place, appends a string made after the buffer, and grows again by copying,
300 rounds inside a loop against one built outside any; with the in-place
write one byte early it printed `0 of 300 rounds agree`. Two ratchet rows:
`a_builder_copied_at_the_frontier` turns the mem corpus red, and
`a_maps_count_held_as_allocating` puts the region back and turns the run
counters red.

## 2026-10-22 — a call into a loop keeps its cohort bracket under a cycle

A cohort bracket marks the arena before a call that crosses into another
module with immutable arguments, and its pop gives back what the call left.
The license refused every caller a cycle reaches (`cycle_reached`), since such
a caller can run once per element of something and its calls are usually
small. It now admits one when the callee reaches a cycle that allocates
(`reaches_cycle`: the members of every cycle whose members the beat analysis
finds allocating, and everything that mentions them, followed backward) and
answers no heap value. A call into a loop pays
its push and pop once against the whole loop, and with no heap value to keep
the pop rewinds without copying anything out.

On the run program the index phase builds 1.5 MB of text to answer an int,
and nothing marked the arena around it, so the string lived until the program
ended. `tally` is reached by a cycle, so the license refused the call. Measured
here, arena_peak_bytes 4,718,608 -> 3,670,032 and the run read 999,948,734 ->
999,793,761 with the same output. The peak now sits at two live megabyte
blocks, the file read and its decoded document, and the 1,572,880-byte string
the phase builds; the program needs all three at that moment.

Three other shapes were measured on the way. Seeded from every cycle, the
reach admitted widebench's whitespace skip, a loop that builds nothing, called
once per value from its parser; CI read widebench 28,619,441 -> 34,955,663
(+22%) and encodebench 2,456,796,655 -> 2,461,704,983, 4.0M of it
`k_cohort_pop` with nothing to give back. The ratchet row
`a_loop_that_builds_nothing_bracketed` puts that seed back and turns the
instructions gate red. Without the heap-value condition
one bracket evacuated a heap result through the carry, and `k_deep_copy` and
`k_copy_size` cost 1.7M more. A region of the beat analysis's kind, marked
before the arguments, made `a_long_chain_answers_survival_from_its_index`
evacuate its 100,000 rows on every lap (evac_allocs 25 -> 480,009), because
the rows its argument built landed inside the mark.

CI's rows: work_runbench 999,949,921 -> 999,794,948 (-0.02%), and no other
benchmark's work moves. The run program's run_arena_peak_bytes 4,718,608 ->
3,670,032 and run_cohort_frees 1 -> 4; run_allocs 1,446,734 -> 1,446,735,
run_alloc_bytes 221,966,536 -> 221,966,568, run_evac_allocs 48,953 -> 48,955,
run_evac_bytes 9,271,280 -> 9,271,408, run_survive_slots 4,856 -> 4,854,
run_str_scans 1,023 -> 1,024, run_str_scan_bytes 17,996,094 -> 17,996,158 and
run_sh_rec 3,776,192 -> 3,776,128. The run program makes 24 more calls
(emitted_other_calls 19,064 -> 19,088, emitted_other_lines 158,186 ->
158,210) and its machine code grows 719,977 -> 722,233 bytes (text 5,236,670
-> 5,238,926 over all the binaries). compile_instructions 23,713,618 ->
23,736,301, entry_instructions 74,424,236 -> 74,485,199, library_instructions
74,085,135 -> 74,147,760 and emit_instructions 6,377,307 -> 6,423,478 rise,
and interp_instructions 384,232,761 -> 384,228,496 falls. In the tab,
browser_compile_instructions 406,034,117 -> 401,989,712 falls. The welfare
sum rose, and the floor moves 92.25 -> 92.28.

`tests/cohort.rs` `a_call_into_a_loop_gives_back_what_it_built` runs a new
two-module fixture, `tests/golden/cohort/phases`: each pass of a loop calls
`grow/total` twice, and each call builds a megabyte of text to answer its
length. It pins cohort_frees=6 and arena_peak_bytes=2097168; under the old
license it read 0 and 3145760. The ratchet row
`a_call_into_a_loop_left_unbracketed` takes the license back and turns the run
counters red.

## 2026-10-22 — the JSON helpers store their call's site only where they can fail

A failure names its place through `k_site`, which the emitter stamps before a
call that can stop the program. Since #1975 an inline twin that takes the site
as its last argument stores it only on the branch that reaches the C, and its
caller skips the stamp. Five twins the encoder calls once per pair or item
took none: `append_key`, `append_string`, `entry_count`, `key_at` and
`value_at`. They now take the site and store it in their slow blocks. The run
program executed 5,384,347 `k_site` stores and now 4,366,872, and read
999,793,761 -> 997,774,101 here with the same output.

Passing the string index (`s[i]`, 690,000 calls in the index phase) its site
the same way, through a C entry that stored it only before refusing, was
measured and dropped: -275k, since the extra argument costs what the store
did.

CI's rows: work_runbench 999,794,948 -> 997,775,288 (-0.20%), work_livebench
1,043,615,776 -> 1,033,729,698 (-0.95%) and work_oneshot 11,443,335 ->
11,424,128 (-0.17%). The run program's machine code grows 722,233 -> 722,249
bytes (text 5,238,926 -> 5,238,942 over all the binaries), and
emit_instructions rises 6,423,478 -> 6,426,328. In the tab,
browser_run_instructions falls 31,623,429 -> 31,602,327, and
browser_compile_instructions 401,989,712 -> 402,017,479 and
browser_compile_peak_bytes 732,887 -> 733,103 rise. The welfare sum rose, and
the floor moves 92.276 -> 92.284.

The builtins are internal to the standard library and every call in lib/json
passes the types they take, so no program reaches the branches that store the
site and no output moves. The emitted counts do not move either: each twin
loses a store at its call and gains one in its slow block. The ratchet row
`a_sited_call_stamped_anyway` stamps every sited call as well; the run
program then reads 1,012,744,021, and the instructions gate goes red.

## 2026-10-22 — a short key is found in the token cache without a call

The decoder reads an object's key through `k_b_plain_string_fast`, which finds
the closing quote and, when no backslash came first, calls
`k_b_utf8_slice_raw` for the string. A key of four to seven bytes is almost
always in the token cache, a direct-mapped table of strings keyed by the
token's bytes and length, so the call usually did a bounds test, a hash and
two loads. The helper's IR now does the hash and the compare itself and loads
the cached string when the slot's key matches; anything else, including a key
that went to the slot beside its own, still takes the call. `k_token_key` and
`k_token_cache` lose `static` so the emitted code can read them.

The run program's calls into the slice from the key path fall from 784,179 to
17,501, and it read 997,774,087 -> 977,794,290 here (-2.0%) with the same
output.

Lifting `k_b_utf8_slice_raw` into the hot unit beside `k_b_find2_raw`, so the
LTO link could inline it, was tried first and read 997,020,816, because the
link kept the call. The emitted IR spells the return type `%KValue` and
clang's unit a literal `{ i64, i64 }`, which may be what stopped it; that was
not isolated.

CI's rows: work_runbench 997,775,288 -> 977,795,537 (-2.00%), work_jsonbench
731,748,677 -> 699,198,721 (-4.45%), work_oneshot 11,424,128 -> 11,233,739
(-1.67%) and work_livebench 1,033,729,698 -> 1,033,539,043. The four
programs that decode objects carry the inline path and grow: text jsonbench
373,113 -> 374,057, oneshot 379,641 -> 380,585, livebench 383,177 ->
384,121 and runbench 722,249 -> 723,129, text 5,238,942 -> 5,242,654 over
all the binaries. Every program declares the two tables: emitted_lines
13,104 -> 13,134 and emitted_branches 1,305 -> 1,307 for the decoder,
emitted_other_lines 158,210 -> 158,320 and emitted_other_branches 16,888 ->
16,894, module_lines 1,161 -> 1,163, and each compile sample two lines more
(recursion 451 -> 453, dispatch 350 -> 352, guards 307 -> 309, records 408
-> 410; lines 1,516 -> 1,524 in all). emit_instructions 6,426,328 -> 6,435,278 and
codegen_instructions_release 422,367,718 -> 422,642,173 rise, and
codegen_instructions_dev 116,068,279 -> 116,064,144 falls. In the tab, browser_run_instructions
31,602,327 -> 31,449,729 falls, and browser_compile_instructions 402,017,479
-> 402,486,449 and browser_compile_peak_bytes 733,103 -> 735,005 rise. The
welfare sum rose, and the floor moves 92.28 -> 92.36.

The micro golden `a_short_key_is_found_in_the_cache_in_place` decodes two
objects with the same fifteen keys, the second reading what the first filled.
Two of its keys, `aaaz` and `abae`, hash to one slot, so the second goes to
the slot beside it. Reading the string from the neighbouring slot, or taking
a slot as a hit without comparing its key, answers one of the two for the
other, and the golden goes red under both; the first is the ratchet row
`a_short_key_read_from_the_slot_beside_it`. The row
`a_short_key_left_to_the_call` sends every key to the call again and turns
the instructions gate red.

## 2026-10-22 — a plain number is read a word at a time

`number_span` finds where a JSON number's bytes end by classifying sixteen
bytes a step against the five characters a number holds besides digits. The
run program's 417,483 numbers are four to nine bytes long and all of one
shape: an optional minus, digits, and an optional point with digits after
it. `k_b_number_span_raw` now reads that shape first, finding the end of each
run of digits eight bytes at a time and checking that the byte after the
number is one no number holds. A leading zero, an exponent, a point with no
digit after it, any other byte a number may hold after the run, and any number
within forty bytes of the end of its document go to the sixteen-byte scan,
which starts again from the beginning. That scan moved into its own function,
`k_number_span_scan`, because the registers it wants made every call build a
frame. `k_nondigits8` moved from numbers.c to text.c, which comes first in the
runtime, so both files use one copy.

The float parse had the same shape of cost. `k_to_float_text` answers the
`[-]I.F` shape from two words and parses everything else with a scan that ends
in a strtod fallback, and the fallback's buffer made every call build a frame.
The slice doors now call that scan out of line, and the shaped case is a
leaf. `k_b_to_float`, the door for a string, keeps it inline: a short string
has no room for the shape's two words and always scans, and with the scan out
of line there CI read widebench 28,619,441 -> 28,939,500.

Measured here, the run program read 977,794,290 -> 966,447,210 (-1.16%) with
the same output: the span 977,794,290 -> 968,235,476, `k_b_number_span_raw`
34,243,704 -> 24,644,367, and the float split 968,235,476 -> 966,447,210,
`k_b_to_float_slice_raw` 28,270,242 -> 26,482,302. Moving the slice that an
out-of-range float or integer builds out of line as well read 966,243,735, a
further 203,475, and made the integer door slower by 217,306; it was dropped. The new
path and the old function, both extracted from the source, agreed on
50,000,000 random byte strings and positions. The harness saw a float answered
as an integer and a leading zero let through, each in about 2,000 of a
million cases.

CI's rows: work_runbench 977,795,537 -> 966,449,322 (-1.16%), work_jsonbench
699,198,721 -> 682,006,171 (-2.46%), work_oneshot 11,233,739 -> 11,118,597
(-1.02%), work_livebench 1,033,539,043 -> 1,033,424,160 and
work_encodebench 2,456,796,655 -> 2,456,796,277. Every binary's machine code
shrinks by 1,376 bytes, the float scan no longer inlined into the slice doors
(text 5,242,654 -> 5,223,390 over all of them; runbench 723,129 -> 721,753).
codegen_instructions_release 422,642,173 -> 422,580,983 falls and
codegen_instructions_dev 116,064,144 -> 116,068,622 rises. In the tab, browser_run_instructions 31,449,729 ->
31,370,234 falls. The
welfare sum rose, and the floor moves 92.36 -> 92.41.

The micro golden `a_number_far_from_the_end_is_read_a_word_at_a_time` decodes
documents whose numbers sit ahead of a long string, so they are within reach
of the new path: plain numbers of both kinds beside exponents, a sixteen-digit
run and `-0`, then eight malformed numbers. The ratchet row
`a_point_forgotten_on_the_fast_path` answers a float as an integer there and
turns the golden red; `a_number_scanned_sixteen_bytes_at_a_time_anyway` turns
the path off and turns the instructions gate red. The row
`a_digit_word_that_reads_a_colon_as_ten` patched `k_nondigits8` in
numbers.c and would not apply once it moved; it patches text.c now.

## 2026-10-22 — a bind chain measures its drift from where it began

A bind chain stages a step's continuation (sizes it, copies it through the
carry pair and rewinds) only when the arena has drifted a quarter megabyte
since the chain's last stage; otherwise the continuation stays where it is.
`k_exec` set the carry's `at_arena` to null when it pushed the chain's beat,
because the depth's last stage, if any, was another chain's. That made every
chain's first step size and stage its continuation however little it had
allocated, and the run program pushes a chain 7,091 times: 7,162 stages,
15,288,786 instructions in `k_beat_iter_carry` and 6,099,136 sizing in
`k_copy_size`. The push now records the arena's position and block, so the
drift is measured from where this chain began. A skipped step was already
sound everywhere, since the 2026-09-07 drift gate made the copy-out at a
carried pop walk deep.

Measured here, the run program read 966,447,210 -> 935,900,947 (-3.16%) with
the same output, and `k_beat_iter_carry` left the profile.

CI's rows: work_runbench 966,449,322 -> 935,902,128 (-3.16%) and
work_deepbench 406,819,829 -> 203,571,400 (-49.96%). work_pendbench
176,534,752 -> 176,531,937, work_readbench 4,583,965 -> 4,582,072,
work_jsonbench 682,006,171 -> 682,004,279 and work_livebench 1,033,424,160
-> 1,033,424,099 fall a little, and work_encodebench 2,456,796,277 ->
2,456,796,762, work_oneshot 11,118,597 -> 11,118,893, work_widebench
28,619,441 -> 28,619,457 and work_digestbench 5,241,183 -> 5,241,199 rise
by a few hundred instructions or less. Every binary's machine code grows 16
bytes (text 5,223,390 -> 5,223,614 over all of them; runbench 721,753 -> 721,769).
codegen_instructions_dev 116,068,622 -> 116,068,667 and
codegen_instructions_release 422,580,983 -> 422,581,877 rise. In the tab, browser_run_instructions 31,370,234 ->
31,370,376 rises. The
welfare sum rose, and the floor moves 92.41 -> 92.53.

The counter veins move where chains no longer stage their first step. The
run program's run_evac_allocs 48,955 -> 1,624, run_evac_bytes 9,271,408 ->
7,416,368, run_allocs 1,446,735 -> 1,438,645, run_alloc_bytes 221,966,568 ->
221,297,400, run_beat_iters 92,018 -> 84,938, run_survive_slots 4,854 ->
4,560, run_buf_reuse 134 -> 139 and run_sh_buf 87,167,344 -> 86,585,920; no
peak moves. pend_allocs 805,977 -> 805,967, pend_alloc_bytes 45,563,312 ->
45,539,584, pend_beat_iters 101 -> 99, pend_evac_allocs 2,225 -> 2,210,
pend_evac_bytes 381,648 -> 378,048, pend_survive_slots 78,611 -> 77,616,
pend_buf_reuse 502 -> 505 and pend_sh_buf 15,860,144 -> 15,838,096;
read_beat_iters 201 -> 200, read_evac_allocs 19 -> 16 and read_evac_bytes
656 -> 576; the decoder's beat_iters 151 -> 150, evac_allocs 19 -> 16 and
evac_bytes 688 -> 576.
The mem corpus moves in nineteen fixtures, each where a chain's first step no
longer stages: a_carried_value_written_into_an_older_node_alloc_bytes 2,595,040 -> 2,568,080, a_carried_value_written_into_an_older_node_beat_iters 11 -> 9, a_carried_value_written_into_an_older_node_evac_allocs 1,697 -> 1,692, a_carried_value_written_into_an_older_node_evac_bytes 174,224 -> 168,896, a_carried_value_written_into_an_older_node_survive_slots 407 -> 404, a_carried_value_written_into_an_older_node_carry_dedup 446 -> 621, a_carried_value_written_into_an_older_node_buf_reuse 781 -> 783, a_carried_value_written_into_an_older_node_sh_buf 682,096 -> 657,104; a_loop_invariant_capture_is_copied_every_rewind_beat_iters 502 -> 501, a_loop_invariant_capture_is_copied_every_rewind_evac_allocs 10 -> 4, a_loop_invariant_capture_is_copied_every_rewind_evac_bytes 8,336 -> 144; a_repaired_node_below_the_mark_holds_tenure_allocs 34,047 -> 34,049, a_repaired_node_below_the_mark_holds_tenure_alloc_bytes 2,493,952 -> 2,510,528, a_repaired_node_below_the_mark_holds_tenure_beat_iters 10 -> 9, a_repaired_node_below_the_mark_holds_tenure_evac_allocs 2,469 -> 2,468, a_repaired_node_below_the_mark_holds_tenure_evac_bytes 162,960 -> 171,840, a_repaired_node_below_the_mark_holds_tenure_carry_dedup 46 -> 80, a_repaired_node_below_the_mark_holds_tenure_sh_buf 567,232 -> 579,536; a_slice_bang_chain_builds_no_slice_beat_iters 2 -> 0, a_slice_bang_chain_builds_no_slice_evac_allocs 15 -> 8, a_slice_bang_chain_builds_no_slice_evac_bytes 528 -> 288; an_inner_beat_opens_its_tenure_in_the_block_outside_allocs 180,196 -> 179,380, an_inner_beat_opens_its_tenure_in_the_block_outside_alloc_bytes 14,926,832 -> 14,712,048, an_inner_beat_opens_its_tenure_in_the_block_outside_beat_iters 60 -> 54, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_allocs 48,411 -> 45,168, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_bytes 3,002,288 -> 2,788,720, an_inner_beat_opens_its_tenure_in_the_block_outside_survive_slots 2,004 -> 11,204, an_inner_beat_opens_its_tenure_in_the_block_outside_carry_dedup 209 -> 459, an_inner_beat_opens_its_tenure_in_the_block_outside_buf_reuse 3,895 -> 3,907, an_inner_beat_opens_its_tenure_in_the_block_outside_sh_buf 5,070,416 -> 4,887,632; an_insisting_form_that_settled_builds_no_step_beat_iters 2 -> 0, an_insisting_form_that_settled_builds_no_step_evac_allocs 18 -> 9, an_insisting_form_that_settled_builds_no_step_evac_bytes 560 -> 288; effect_push_shape_allocs 56 -> 52, effect_push_shape_alloc_bytes 2,544 -> 2,320, effect_push_shape_beat_iters 5 -> 0, effect_push_shape_evac_allocs 50 -> 14, effect_push_shape_evac_bytes 1,696 -> 464, effect_push_shape_survive_slots 10 -> 4, effect_push_shape_sh_buf 768 -> 608; stream_write_beat_iters 1,601 -> 1,600, stream_write_evac_allocs 12 -> 6, stream_write_evac_bytes 480 -> 240, stream_write_held_peak_bytes 16,000 -> 16,160; the_same_capture_built_below_the_mark_is_shared_beat_iters 502 -> 501, the_same_capture_built_below_the_mark_is_shared_evac_allocs 10 -> 6, the_same_capture_built_below_the_mark_is_shared_evac_bytes 8,352 -> 8,192, the_same_capture_built_below_the_mark_is_shared_survive_slots 504 -> 4; tie_cycle_beat_iters 1 -> 0, tie_cycle_evac_allocs 15 -> 12, tie_cycle_evac_bytes 560 -> 464; work_ahead_forces_what_follows_a_park_beat_iters 2 -> 0, work_ahead_forces_what_follows_a_park_evac_allocs 11 -> 7, work_ahead_forces_what_follows_a_park_evac_bytes 368 -> 240; work_ahead_leaves_a_chain_never_run_beat_iters 1 -> 0, work_ahead_leaves_a_chain_never_run_evac_allocs 5 -> 4, work_ahead_leaves_a_chain_never_run_evac_bytes 192 -> 144; work_ahead_leaves_a_literal_where_it_is_beat_iters 2 -> 0, work_ahead_leaves_a_literal_where_it_is_evac_allocs 11 -> 7, work_ahead_leaves_a_literal_where_it_is_evac_bytes 368 -> 240; work_ahead_leaves_an_extending_builtin_beat_iters 4,002 -> 4,000, work_ahead_leaves_an_extending_builtin_evac_allocs 13 -> 9, work_ahead_leaves_an_extending_builtin_evac_bytes 448 -> 320; work_ahead_lifts_a_parking_lambda_beat_iters 1,001 -> 1,000, work_ahead_lifts_a_parking_lambda_evac_allocs 12 -> 4, work_ahead_lifts_a_parking_lambda_evac_bytes 384 -> 144; work_ahead_moves_a_binding_into_the_park_beat_iters 2 -> 0, work_ahead_moves_a_binding_into_the_park_evac_allocs 11 -> 7, work_ahead_moves_a_binding_into_the_park_evac_bytes 368 -> 240; work_ahead_pools_a_call_on_a_builtin_read_beat_iters 3,002 -> 3,000, work_ahead_pools_a_call_on_a_builtin_read_evac_allocs 15 -> 11, work_ahead_pools_a_call_on_a_builtin_read_evac_bytes 496 -> 368; work_ahead_pools_a_call_on_a_local_beat_iters 1,002 -> 1,000, work_ahead_pools_a_call_on_a_local_evac_allocs 13 -> 9, work_ahead_pools_a_call_on_a_local_evac_bytes 416 -> 288; work_ahead_reads_a_moved_binding_beat_iters 2 -> 0, work_ahead_reads_a_moved_binding_evac_allocs 11 -> 7, work_ahead_reads_a_moved_binding_evac_bytes 368 -> 240. The only peak among them is stream_write's held peak, 160 bytes higher. The ratchet row `a_chain_staged_on_its_first_step` puts the null back
and turns the run counters red. `tests/a_document_read_from_a_file_keeps_its_loop.rs`
counted one beat per iteration and one for the program; the program's own
chain no longer stages, and its four programs read 200, 800, 200 and 200.

## 2026-10-22 — five JSON decoder states skip a blank run out of line

Each state of the JSON decoder read whitespace with an arm that called the
state again on the next byte, so every state was a loop. Five of them
(`array_delim`, `array_open`, `obj_delim`, `obj_key_end`, `obj_open`) now skip
the run with `skip_ws` and read the byte after it with a second group of the
same arms and no blank one, so they no longer loop. `obj_key_start` keeps the
arm that calls itself.

The lead came from the per-instruction profile of the stacked tree. Each
decoder frame loaded three constants in its prologue that its hot path never
used: a `movabs` and two `lea`s, about four instructions a call. Turning off
LLVM's machine LICM at the LTO link removed them and read the run program
935,901,767 -> 927,399,704 (-0.91%), but as a global flag it was a trade:
jsonbench -1.98% and encodebench -2.27%, against escapebench +3.20% and
widebench +1.35%, whose real loops lost their hoisting. It was declined. The
narrower LICM switches (`-disable-postra-machine-licm`,
`-sink-insts-to-avoid-spills`, `-disable-hoisting-to-hotter-blocks=all`) moved
the run program by less than half a million instructions either way.

Taking the loops out of the states moved far more than the hoisted constants.
LLVM folds the decoder's states into one another differently once they stop
looping, and the run program fell 39.4M with all six states out of line. The
effect of each state was measured by looping it alone against all six out:
`array_delim` +13.8M, `array_open` +26.6M, `obj_open` +1.9M, `obj_delim` +1.2M,
`obj_key_end` -4.6M and `obj_key_start` -18.1M. The effects do not add: the
two key states looped together read +13.6M, and with `obj_delim` as well
+22.4M. The shape taken is the best one measured. A cycle-free `value_for`
(its eight arms copied into a second group) moved the run program by 1,239
instructions and added 14 KB of machine code; it was dropped.

Measured here, every benchmark in the instructions gate: the run program
935,901,781 -> 878,353,218 (-6.15%), jsonbench 682,003,932 -> 599,977,782
(-12.03%), oneshot 11,118,560 -> 10,571,976 (-4.92%) and livebench
1,033,423,752 -> 1,032,876,960 (-0.05%); the other ten move by fourteen
instructions or less. The output is the same.

The decoder's emitted code rises by the five second groups and their twins:
emitted_defines 136 -> 157, emitted_calls 1,394 -> 1,557, emitted_branches
1,307 -> 1,346 and emitted_lines 13,134 -> 14,330. oneshot, livebench and
runbench rise by the same 21 defines and 163 calls, so over the other
benchmarks emitted_other_defines 2,154 -> 2,217, emitted_other_calls 19,088
-> 19,577, emitted_other_branches 16,894 -> 17,011 and emitted_other_lines
158,320 -> 161,908. No allocation counter moves.

CI's rows: work_runbench 935,902,128 -> 878,353,565 (-6.15%), work_jsonbench
682,004,279 -> 599,978,129 (-12.03%), work_oneshot 11,118,893 -> 10,572,323
(-4.92%) and work_livebench 1,033,424,099 -> 1,032,877,307 (-0.05%). The
machine code grows where the decoder is linked in: text 5,223,614 ->
5,340,654 over every binary, runbench 721,769 -> 750,713, jsonbench 372,697
-> 402,057, oneshot 379,225 -> 408,601 and livebench 382,761 -> 412,121.

The compiler pays for the new groups in every program that imports
`std/json`: compile_instructions 23,736,301 -> 25,221,632 (+6.26%),
entry_instructions 74,485,199 -> 74,568,239, library_instructions 74,147,760
-> 74,230,433, compile_allocs 13,179 -> 13,789, compile_peak_bytes 565,175 ->
598,984 and front_end_visits 6,914 -> 7,251. The interpreter's corpus
reads interp_instructions 384,228,496 -> 385,357,985, interp_allocs 400,588
-> 401,539 and interp_peak_bytes 608,798 -> 637,429. codegen_instructions_dev,
codegen_instructions_release and emit_instructions do not move.
In the tab, browser_compile_instructions 402,486,449 -> 428,211,402 (+6.39%)
and browser_compile_peak_bytes 735,005 -> 768,853 rise with the compiler's
share, and browser_run_instructions 31,370,376 -> 30,540,337 falls. The
welfare sum rose, and the floor moves 92.53 -> 92.73.

The micro golden `a_blank_run_in_json_is_read_by_each_state` puts a run of
blanks before every byte each state can meet, refusals included, and prints
each refusal's position. Main's decoder prints the same 23 lines. The ratchet
row `a_blank_run_looped_at_the_comma` gives `array_delim` its looping arm back,
which reads the run program 13.8M higher, and turns the instructions vein red.

The benchmark documents hold almost no blanks outside their strings, so the
run program's document was indented two spaces a level (458,223 bytes,
271,763 of them blanks) and decoded once: 16,561,895 -> 16,456,195 (-0.64%).

Also measured and declined: `k_render_number` tests whether a double is
integral with `floor`, a PLT call; a cast round trip in its place read the run
program +181,613. The `floor` it calls here is glibc's `__floor_sse41`,
which the profile counts at 1.5 instructions a call.

Also measured and declined, closing #2011: the span's fast path worked out the
value of the number it found and kept it, keyed by the sixteen bytes from the
number's first, its length and which conversion it answers, and the
conversion doors answered from it. Every number hit. `k_b_to_float_slice_raw`
fell 26,482,302 -> 5,504,994 and `k_b_to_int_slice_raw` 13,492,972 ->
6,464,568, but `k_b_number_span_raw` rose 24,644,367 -> 36,078,570 and the
float fill cost 16,259,562, and the run program read 935,901,781 ->
935,589,998. With the int fill out of line it read 939,328,415. The
conversion's arithmetic costs about the same in the span as in the door.

## 2026-10-22 — an escaped JSON string is read in one walk

A JSON string with an escape in it was read twice. `plain_string`'s emitted
twin found the first backslash, then walked to the closing quote with a
`find2` from each backslash, and answered the quote's position; the decoder
then called `unescape` on the same range, which read the bytes again to undo
the escapes. The twin now hands the first backslash to `k_json_escaped`, which
undoes the escapes while it looks for the quote. It writes the undone bytes at
the arena's frontier, and at the quote it allocates the string `unescape`
would have allocated, at the same length, which lands where the bytes already
are; so no allocation counter moves. It keeps the string, keyed by the bytes'
address and the range, and `unescape` answers it first when asked for that
range, before a frame. A `u` escape, an escape JSON does not have, or a
frontier without room leaves nothing kept, and the old walk finds the quote.

The fuzzer was built first, against the joined runtime with its `main`
renamed, so it ran the real functions. Its reference is the walk the emitter
wrote before followed by `k_b_unescape_raw`. Half its calls cut the arena's
frontier short, so every fallback runs. Unstressed it ran 48,354,448 escaped
strings and found 19 failures: when the frontier ran out in the middle of a
run, the walk handed a plain byte's position to `k_json_quote_after`, which
takes a backslash's and skips the byte after it. The stress found the same bug
2,952 times in 2,000,000 cases. With `k_json_quote_from` for that case, 25
seeded runs of 2,000,000 cases each read 48,354,997 escaped strings, 8,013,430
of them answered from the kept string, and found no failure.

Measured here, the run program 878,353,521 -> 871,922,860 (-0.73%) with the
same output. `k_b_unescape` fell 38,534,463 -> 3,335,013, and the decoder
frames that inlined the twin lost the rescan loop (`array_items` -6,489,450,
`parse_value` -5,370,849, `obj_key_start` +929,412), against 39,591,486 in
`k_json_escaped`. find2_calls falls where the walk no longer calls
`find2`: run_find2_calls 1,488,669 -> 1,037,031, the decoder's find2_calls
2,255,550 -> 1,571,250, oneshot_find2_calls and live_find2_calls 15,037 ->
10,475. The decoder's emitted code sheds the loop: emitted_branches 1,346 ->
1,344 and emitted_lines 14,330 -> 14,319; every program gains the new door's
declare line, so emitted_other_lines rises by one a program where the decoder
is absent.

CI's rows. run_instructions 878,353,565 -> 871,922,904 (-0.73%), and the
other programs that decode an escaped string fall with it: jsonbench
599,978,129 -> 593,998,829, oneshot 10,572,323 -> 10,531,404, livebench
1,032,877,307 -> 1,032,837,487, encodebench 2,456,796,762 -> 2,456,796,258.
Machine code: the programs with the decoder shrink (jsonbench text 402,057 ->
401,417, runbench 750,713 -> 750,041, livebench 412,121 -> 411,481, oneshot
408,601 -> 407,961) and every other program grows by 1,504 bytes
(encodebench 342,473 -> 343,977). Building the run program:
codegen_instructions_dev 116,068,667 -> 116,128,678,
codegen_instructions_release 422,581,877 -> 422,378,106, and
emit_instructions 6,435,278 -> 6,432,635. The machine-code total, text, is
5,340,654 -> 5,353,102. The compile corpus's programs carry the declare line
too: lines 1,524 -> 1,528 over its four, and module_lines 1,163 -> 1,164. In the tab, which carries the same
library: browser_compile_instructions 428,211,402 -> 428,034,601,
browser_compile_peak_bytes 768,853 -> 768,205, browser_run_instructions
30,540,337 -> 30,427,332. The welfare floor rises 92.73 -> 92.76.

The micro golden `an_escaped_string_is_undone_in_one_walk` puts escapes at
the front, the back and each side of a word boundary, two backslashes
together, a document that ends a byte after the string, a `u` escape, an
unknown escape and two strings that run out. The interpreter prints the same
26 lines, and an `\n` undone as a tab turns it red. The ratchet row
`an_escaped_string_undone_twice` keeps nothing, so `unescape` reads every
string again; the run program reads 910,139,124.

Measured on #2043's shape and declined. Looping one more decoder state as
well as `obj_key_start` reads higher every time: `obj_delim` 901,567,246,
`obj_open` 882,623,604, `array_open` 889,848,391, `obj_key_end` 910,134,560,
`array_delim` 892,170,102, against 878,353,218. Dropping the five `_past`
helpers by binding the skipped position in each blank arm cut the corpus check
38,158,594 -> 37,784,461 and read the run program 930,125,639. Three encoder
shapes read higher too: the list's body written in `encode_onto`'s arm
885,700,026, the map's 885,870,393, and the first element folded into
`encode_items` behind a separator group 885,304,273.

Also declined: an int of up to eight digits read inline by `to_int`'s slice
twin, through a hot-unit helper LTO folds in, with the call kept for every
other shape. `k_b_to_int_slice_raw` fell 13,492,972 -> 640,000, but the
inlined read cost 50 instructions an int in `array_items` and `parse_value`,
against 65 for the call, and the run program read 878,353,521 -> 876,146,778
(-0.25%), before what inlining it at every site costs the release build.

A container that restarted came up with clang 18, where the probe for
`preserve_none` answers no and the emitter writes another convention: main
emitted 2,135 fewer lines for jsonbench there than its golden says. The rows
above were taken under clang 19, as CI installs it.

## 2026-10-22 — the region row has a witness again

The ratchet row `a region that keeps what it made` went blind when std/json
began writing a map from its entries in place (c2b48e57). Its mutation makes
`k_region_pop` keep everything the region allocated, and its witness was the
mem fixture `a_nested_map_gives_back_its_entries`, whose arena peak doubled
under it. Once the encoder stopped building `keys` and `values` for each map,
its calls had no allocating argument, so the emitter made no region at all:
no program in the benchmark corpus or the mem vein calls `k_region_pop` now,
and with the mutation applied the whole mem corpus stayed green on main. The
row came up on #2047, whose diff touches `core.c`, and read BLIND.

The new mem fixture `a_descent_gives_back_the_keys_it_built` keeps the
encoder's old shape: a walk that builds a map's keys as the argument of its
call into the map and answers bytes. The emitter makes that call a region.
It reads arena_peak_bytes 8,388,608 and arena_blocks 8; with the mutation,
12,582,912 and 12, and chain_finds 0 -> 9. The interpreter prints the same
total and the oracle's counters agree. The old fixture's comment, which still
described the region it no longer has, says what it pins now.

## 2026-10-22 — JSON escapes are written and undone sixteen bytes at a time

The encoder writes a string with an escape in it through
`k_json_escape_into`, which copied the bytes before each escape a word at a
time and then a byte at a time. The strings the run program writes this way
average about 23 bytes with two or three escapes each, so most runs were
shorter than a word and went through the byte loop at nine instructions a
byte. The writer now reads sixteen bytes at a time on x86-64 and arm64,
stores all sixteen, and finds the first quote, backslash or byte below a
space with three compares and a mask. A string's last block is read through
`k_tail_window` when it is shorter than sixteen bytes, as the scanners read
theirs, and the bytes past its end are ignored. Where the window would cross
a page, and on targets without the vector compares, the word and byte walk
still runs; it is out of line, so its constants no longer take registers in
the block loop. The writer leaves sixteen bytes of slack past its worst case
instead of eight.

The first version clamped every block to the string's end. With that
bookkeeping on each block the run program read 870,530,776; with the clamp
only on the last block it read 864,531,687. Moving the stack buffer the
writer uses for a builder without room into its own cold function took the
792-byte frame off every call.

The decoder's `k_json_escaped`, from the previous entry, copies the run
after each escape to the next quote or backslash. The run averages about
five bytes on the run program, and the word loop took one word and most of
another. It now reads a block of sixteen while sixteen bytes of the document
remain, and the word loop finishes the last fifteen. The run program read
864,531,687 -> 859,100,922 (`k_json_escaped` 39,591,486 -> 34,160,742).

Two more doors take preserve_none, measured one at a time as the first four
were: `append_string_mut` -2,228,335 and `to_float_slice_raw` -1,113,877.
`to_float_slice_raw` is the door the decoder's number twin calls when the
tags are plain; `to_float_slice`, on the list since the four were chosen, is
now its fallback. `to_int_slice_raw` read +472,341 and stays on the C
convention.

Measured here, `k_b_append_string_mut` fell 39,182,580 -> 31,498,560, and
the run program printed the same output.

CI's rows. run_instructions 871,922,904 -> 855,758,740 (-1.85%), and the
programs that encode or decode escaped strings fall with it: jsonbench
593,998,829 -> 584,083,829, livebench 1,032,837,487 -> 990,759,394, oneshot
10,531,404 -> 10,361,059. Two rose by amounts this entry does not account
for: work_encodebench 2,456,796,258 -> 2,456,797,378 (+1,120) and
work_readbench 4,582,072 -> 4,582,081 (+9). Machine code shrinks in every program,
2,544 bytes in most and 2,432 to 2,480 in the four with the decoder; text
5,353,102 -> 5,317,806. Building the run program:
codegen_instructions_dev 116,128,678 -> 116,141,087,
codegen_instructions_release 422,378,106 -> 422,507,483, and
emit_instructions 6,432,635 -> 6,433,837. The tab's runtime is built for
wasm32, where the writer has no blocks and takes the walk:
browser_run_instructions 30,427,332 -> 30,460,236 and
browser_compile_instructions 428,034,601 -> 428,036,106.

append_fast rises where a builder grows for the wider slack: the writer
reserves sixteen bytes past its worst case instead of eight, so a builder
short of that grows once on the out-of-line path and later appends find
room. run_append_fast 5,328,540 -> 5,329,260, live_append_fast 23,682,400 ->
23,685,600 and oneshot_append_fast 59,206 -> 59,214; in the mem vein,
`a_literal_appended_across_a_rewind` 2,120 -> 2,200,
`a_nested_map_gives_back_its_entries` 156,014 -> 156,026 and
`a_written_map_allocates_no_columns` 12,007 -> 12,015. With the old reserve
the first reads 2,120 again. The welfare floor rises 92.76 -> 92.83.

The fuzzers were built against the joined runtime with its `main` renamed,
so they run the real functions.
- The writer's reference is a byte-at-a-time escaper written from lib/json's
  rules. Every entry is driven: `append_string`, `append_escaped` and
  `append_key`, in place and copying, into builders with and without room.
  A quarter of the strings end on the last byte before an unmapped page.
  25 seeded runs of 2,000,000 cases each found no failure, 12,497,738 of the
  cases ending at the unmapped page. Without the window test it faults at
  that page, and without the clamp to the string's end it reads 54,373
  failures in 200,000 cases.
- The decoder's walk ran under the previous entry's fuzzer, arena stress
  included: 25 seeded runs of 2,000,000 cases each, 48,354,997 escaped
  strings, no failure. Reading blocks to within eight bytes of the end, which
  takes bytes past it, reads 13 failures in 500,000 cases.
- Both ran again cross-compiled for arm64 under qemu, where the NEON blocks
  run: the same 25 seeded runs of 2,000,000 cases each, no failure in
  either. Without the NEON block's control-byte compare the writer reads
  72,894 failures in 100,000 cases, and without its backslash compare the
  undo walk reads 33,697.

The micro golden `a_string_is_escaped_sixteen_bytes_at_a_time` decodes and
encodes again strings with escapes just before, on and just after a
sixteen-byte boundary, control bytes, a key and two-byte characters. The
interpreter prints the same twelve lines. With the control-byte compare
taken out of the block, the native build writes raw control bytes and the
golden goes red. The ratchet rows `an_escaped_string_written_a_word_at_a_time`
and `an_escaped_string_undone_a_word_at_a_time` take the blocks away from
each walk.

Declined: the block-less walk answering its position and letting the caller
move the output by the same amount, instead of writing through a pointer to
it. The run program read 865,760,286 against 864,531,687.

Also declined: carrying a joined string's character count into a string
built by a template. The 142 `length` scans on the run program are not
joins. Ninety are `length` on the encoder's output, counted 16 bytes a
compare at about a third of an instruction a byte, and making the builder
count as it appends costs more than the scan.

Also measured and not built: storing the site a death names as a small
index with an immediate, where the address of its literal now costs a `lea`
and a store. A prototype that stamped only the index read the run program
855,758,710 -> 853,297,005 (-0.29%). Building it would give every twin that
takes a site an index beside its origin pointer, and 4,374,601 stores were
the most it could have saved.

Kanso#2024 is closed. Its first lead shipped in #2025 and its third with
`append_key`. Of its second, 8,091,439 overflow checks still execute. Most
guard arithmetic on ints with no bound inside their own group.

## 2026-10-22 — a string index the checker proves goes straight to the text

Where the sets prove the container a string and the position an int, the
emitter now calls `k_b_at_str` with the string's pointer and the position.
It is the string arm of `k_b_at`, lifted out, and `k_b_at` calls it too. The
site used to emit a bytes test that a string never passes, then call
`k_b_at` with two KValues, which tested both tags again, then merge the slow
answer back with a select that rebuilt the byte form. The arm answers a
one-character string or `none`, and nothing in it stops the program but
running out of memory, so the site stamps no place for a death.

Measured here on #2047's build, the run program 855,758,710 -> 844,029,230
(-1.37%) with the same output. Every index in it of this shape is the index
phase's walk, 690,000 calls: the door fell 40,070,237 -> 34,500,017
(`k_b_at` against `k_b_at_str`), and `tally_4`, where the walk is inlined,
fell 22,887,971 -> 16,677,970 without the bytes test and the merge.

CI's rows. run_instructions 855,758,740 -> 844,029,138 (-1.37%), and
work_indexbench, which is the same walk alone, 2,326,925 -> 1,986,925
(-14.6%). Three moved by amounts this entry does not account for:
work_encodebench 2,456,797,378 -> 2,456,796,006, work_oneshot 10,361,059 ->
10,361,367 and work_livebench 990,759,394 -> 990,759,429. The emitted code
of the two programs with the walk shrinks: indexbench's branches 48 -> 42
and lines 787 -> 756, runbench's branches 6,451 -> 6,445 and lines 57,875 ->
57,843, so emitted_other_branches and emitted_other_lines fall. Machine code
grows by 1,040 bytes in most programs and 960 and 992 in the two with the
walk, since `k_b_at_str` is a function beside `k_b_at`: text 5,317,806 ->
5,332,238. Building the run program: codegen_instructions_dev 116,141,087 ->
116,162,655, codegen_instructions_release 422,507,483 -> 422,568,857, and
emit_instructions 6,433,837 -> 6,434,680. In the tab:
browser_compile_instructions 428,036,106 -> 427,996,730,
browser_compile_peak_bytes 768,205 -> 768,243 and browser_run_instructions
30,460,236 -> 30,450,747. The welfare floor rises 92.83 -> 92.88.

Declined: giving the index twin `k_b_at_fast` the same string arm, for
indexes whose sets do not prove a string. Alone it read the run program
-8,405, against a branch and a call at every unproven index site.

The micro golden `a_string_index_the_checker_proves_goes_straight_to_the_text`
walks strings of one- to four-byte characters forward, which steps the
cursor, and backward, which seeks, with positions past both ends. The
release build, which the corpus runs, takes `k_b_at_str` at both walks. With
the cursor stepping one byte instead of one character the walk splits the
wide characters and the golden goes red. The ratchet row
`a_proven_string_index_through_the_tag_tests` takes the arm away.

## 2026-10-23 — join keeps its list and separator in locals

`k_b_join` cost 12,305,923 instructions in the run program, about 61 for each
of its 200,000 pieces, most of them in the pend phase's `text/join digits " "`.
The copying loop read the list's length and items, and each piece's length,
through their structs after every copy, because a store through `char*` may
alias them; each piece's length was read twice. A one-byte separator went
through `k_copy_short`'s size ladder, five compares to store one byte. The
summing loop added the separator's length on every pass behind an `if (i)`.

The list, the separator and each piece's length are now read into locals. A
one-byte separator is stored from a byte held in a register, and the
separator's share of the total is added once after the summing loop, as
`(n - 1) * sl`. `k_b_join` falls to 7,308,590, measured here.

CI's rows. run_instructions 844,029,138 -> 839,032,001 (-0.59%) and
work_pendbench 176,531,937 -> 156,541,737 (-11.3%). work_basket fell
31,152,439 -> 31,104,262; it joins too. Five moved by amounts this entry does
not account for: work_encodebench 2,456,796,006 -> 2,456,797,210,
work_oneshot 10,361,367 -> 10,361,430, work_indexbench 1,986,925 ->
1,987,038, work_scanbench 274,277 -> 274,340 and work_livebench 990,759,429 ->
990,759,737. Machine code grows by 384 bytes in every program: text
5,332,238 -> 5,337,614. Building the run program compiles the runtime, so
codegen_instructions_dev 116,162,655 -> 116,163,063 and
codegen_instructions_release 422,568,857 -> 422,573,245. In the tab,
browser_run_instructions 30,450,747 -> 30,425,950.

Both engines print the same bytes for an empty list, one item, an empty
separator, one- and two-byte separators with empty pieces, pieces and a
separator past sixteen bytes, and multi-byte pieces and separator, whose
character count is read too. The change is a rewrite of the loop the work
vein already counts, so the vein is its witness.

## 2026-10-23 — the site a death names is stamped as an id

Before every call that can stop the program, compiled code records the place
a death will name (ruled on #1975). It stored the interned literal's address
in `k_site`: a `lea` to materialise the address and a 64-bit store, 4,374,601
times a run. It now stores a 32-bit id as an immediate into `k_site_id`, one
`mov`. The program defines `k_site_name`, which maps an id back to its
literal through a table, and `k_died` reads it only when the program dies.
The 25 twin slow paths that receive the site as a `ptr %site` still store it
in `k_site`, and zero the id; `k_site_now` prefers the id when it is set. The
tab reaches `k_site_name` through an eighth hook in `wasm/hooks.c`.

The table went through three shapes, each measured. As `[N x ptr]` it put a
relocation per slot in the position-independent binary, and scanbench rose
3,107 instructions in `_dl_relocate_object`. As offsets into one
`@k_site_text` constant it needs none. A slot whose stamp was pruned with its
function holds zero and writes no text. Appended to the module's globals, the
table pushed that buffer past a doubling, and the tab compile's peak memory
rose 13,902 bytes; written into the module, which is allocated at its final
size after one counting pass, the rise is 1,142 bytes. Encoding the offsets as
a byte string rather than `[N x i32]` read the same to within 160 bytes, so
the table keeps the readable form.

CI's rows, against main with the join merged. run_instructions 839,032,001
-> 836,829,464 (-0.26%). The other work rows that fell most:
work_encodebench 2,456,797,210 -> 2,367,636,382 (-3.6%), work_livebench
990,759,737 -> 980,646,434 (-1.0%), work_deepbench 203,571,400 ->
201,843,399 (-0.85%), work_digestbench 5,241,199 -> 5,211,213,
work_pendbench 156,541,737 -> 155,739,870, work_oneshot 10,361,430 ->
10,346,159, work_basket 31,104,262 -> 31,068,983, work_widebench 28,619,457
-> 28,587,459, work_scanbench 274,340 -> 274,212 and work_readbench 4,582,081
-> 4,581,876. Two rose: work_jsonbench 584,083,829 -> 585,129,095 (+0.18%)
and work_escapebench 39,058,437 -> 39,058,492; work_indexbench moved by one,
1,987,038 -> 1,987,039. Every program gains the `k_site_name` definition,
and the stamps' literals leave the body: emitted_other_defines 2,217 ->
2,230 and emitted_other_lines 161,822 -> 161,270, and the decoder's
emitted_defines 157 -> 158 and emitted_lines 14,319 -> 14,296. The small
compile corpus, which has few stamps, gains more lines than it loses: its
defines 42 -> 46 and lines 1,528 -> 1,569, and the module build's
module_defines 29 -> 30 and module_lines 1,164 -> 1,171. Machine code falls,
text 5,337,614 -> 5,335,758. The emitter's scan for surviving stamps and the
table cost the compiler: emit_instructions 6,434,680 -> 6,521,335 (+1.3%),
codegen_instructions_dev 116,163,063 -> 116,434,829 and
codegen_instructions_release 422,573,245 -> 425,464,049 (+0.68%).
compile_instructions, entry_instructions and library_instructions each fell
by 15 to 25, which is layout. In the tab: browser_compile_instructions
427,996,730 -> 428,979,212, browser_compile_peak_bytes 768,243 -> 769,385 and
browser_run_instructions 30,425,950 -> 30,274,594. The run side outweighs the
compile side, and welfare rises.

jsonbench rises: `d_json/obj_key_start_4_w`'s IR differs from main by the one
stamp line, and with `rax` no longer needed for the address the register
allocator added a `mov r14,r10` / `mov r10,r14` pair on one path of its loop.

Declined: testing whether a float is integral with a cast and compare instead
of `floor`, in `k_b_append_rendered`. glibc's `__floor_sse41` costs three
instructions a call through the PLT, and the inline test cost four more, so the
run program rose 182,110.

The ratchet row `a_site_named_by_its_address_alone` has the runtime read only
`k_site`, and `runtime_corpus_reports_endpoint_violations` goes red: a death
behind an id stamp loses its `failed in` line.

## 2026-10-23 — the module writer moves out of codegen.rs, and the site table stops searching

`src/codegen.rs` was 4,680 lines. The code that writes the module around the
finished body now sits in `src/codegen/module.rs`, read with `include!` like
`narrow.rs`: `Written` and `finish`, the doors and `through_doors`, the call
twins, the type and site tables, `paired_appends` and `ir_bytes` (#2053). The
emitted IR of the run program is byte-identical once the checkout's path is
set aside. Four ratchet mutations and the wasm32 line check read the new
file. Alone, the move read browser_compile_instructions 428,979,212 ->
428,979,320 by layout, which put welfare just under its floor, so it lands
with the change below.

#2051's site table wrote text only for the stamps that survived the prune,
and finding them was a substring search over every piece of the body:
3,184,599 of the 96,470,945 instructions of a build of the browser corpus.
Almost every site survives. The table now writes every site the emitter
stamped, which adds 92 bytes of site text to that corpus and 802 to the run
program, and the search is gone.

CI's rows. emit_instructions 6,521,335 -> 6,449,595 (-1.1%) and
browser_compile_instructions 428,979,212 -> 427,904,481 (-0.25%);
browser_compile_peak_bytes rises 769,385 -> 769,408. Three front-end rows
return to what they read before #2051, by layout: compile_instructions
25,221,617 -> 25,221,632, entry_instructions 74,568,214 -> 74,568,239 and
library_instructions 74,230,408 -> 74,230,433. The run program's IR differs
only in the site constants, and four work rows moved with them:
work_runbench 836,829,464 -> 836,829,597, work_encodebench 2,367,636,382 ->
2,367,636,459, work_oneshot 10,346,159 -> 10,346,621 and work_livebench
980,646,434 -> 980,646,336.

## 2026-10-23 — the empty literals' C entries take no frame

`[]` and `{}` call `k_list_empty` and `k_map_empty`, and the decoder makes
557,000 of them on the run program. Each entry bumped the arena through
`k_alloc`, whose refill is a call, so the entry aligned the stack with a push
and a pop on every literal. The bump is now written in the entry, and the
counting path and a short arena go to a cold `*_slow` entry as a tail call;
the common path is sixteen instructions and no frame. The allocation counters
cannot move, since counting still goes through `k_alloc`.

CI's rows. run_instructions 836,829,597 -> 835,716,002 (-0.13%), and
work_jsonbench 585,129,095 -> 583,476,695 (-0.28%), work_oneshot 10,346,621 ->
10,335,157, work_deepbench 201,843,399 -> 201,739,409 and smaller falls in
seven more. work_escapebench rose 39,058,492 -> 39,058,532, which this entry
does not account for. Building compiles the runtime, so
codegen_instructions_dev 116,434,829 -> 116,444,792 and
codegen_instructions_release 425,464,049 -> 425,503,297, and machine code
grows by the slow entries: text 5,335,758 -> 5,338,222. In the tab, where a
wasm function has no frame to save, browser_run_instructions 30,274,594 ->
30,275,515.

Declined, both measured on CI first: the same bump written in the emitted IR
as helpers in DECLARES. Inlined at every literal, the run program fell 0.30%,
but the release build rose 4.4%, the dev build 1.0%, the tab compile 0.22% and
its peak memory 0.7%, which the objective scores as a loss. Defined once and
called, the run program fell 0.133% and the release build still rose 1.65%,
clang spending it on the new functions inside the decoder.

## 2026-10-23 — a string index answers its step and its cache hits in the entry

`k_b_at_str`, the proven string index, is called 690,000 times a run by the
run program's index phase, each call stepping the cursor one character. It
now steps the cursor and reads the ascii and wide-character caches in its own
body, and sends the seek, an ascii miss and a wide miss to functions of their
own. Making every one of those a tail call, so the entry would take no frame,
did not happen: clang emits plain calls on these paths whatever their
convention, so `k_b_at_wide_miss` keeps its `preserve_most`.

CI's rows, against main with the frameless empty literals merged.
run_instructions 835,716,002 -> 834,336,454 (-0.17%), and work_indexbench,
which is the same walk alone, 1,987,039 -> 1,947,078 (-2.0%); three other
work rows fell by under 300, and work_oneshot rose 10,335,157 -> 10,335,619,
which this entry does not account for. Machine code falls, text 5,338,222 ->
5,321,422. Building compiles the runtime, and both codegen rows fall a
little: codegen_instructions_dev 116,444,792 -> 116,430,160 and
codegen_instructions_release 425,503,297 -> 425,484,869.
