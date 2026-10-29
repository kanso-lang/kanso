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




## 2026-10-26 — the runtime's tags and doors are read from its C

Fixes #2130. `build.rs` already joins the runtime's parts, and it now reads
the tag enum and the functions defined with `K_DOORCC` out of the joined
text into `$OUT_DIR/runtime_abi.rs`. The seven tags `codegen.rs` restated,
`K_ERR_TAG`, the door list in `codegen/module.rs`, and the two specs that
held those copies to the C are gone. A door the reader cannot parse stops
the build, since a door missing from the list would be called on the C
convention; the ratchet row `a_door_the_build_cannot_read` breaks one and
watches `cargo build` go red.

CI's rows, against main at #2154. The derived door list is in the runtime's
include order, and `through_doors` checks doors in list order, so
`emit_instructions` reads 5,687,958, 224 fewer. Compiling the runtime reads
the edited comment in `core.c`, which moves `codegen_instructions_dev` to
116,762,560 (52 more) and `codegen_instructions_release` to 371,663,962 (71
fewer). Both read as layout of the clang input, since the program's code
is unchanged, and no runtime row moved.

## 2026-10-26 — a guard inside a bound branch leaves the branch natively

#2166. `return x if c` inside an `if` branch that is bound to a name leaves
the branch: the interpreter binds the name to the early answer and the
function goes on. The native backend refused to build any such program with
"a return guard sits only in tail position", although `kanso check` accepts
it, so the engines disagreed. The emitter now reads a guard for its value as
it reads an `if`: the condition asked as a question, the early answer one arm,
the rest of the block the other, and a phi over the two and any failure the
condition carried out. `a_guard_leaves_the_branch_it_is_written_in` holds
#2166's program with the guard firing and not, in both branches, and prints
`11 10` and `36 18` on both engines; on main the native build stops.

CI's rows, against main at #2149. No program in the corpora has the shape, so
nothing runtime moves, and only the emitter's new arm shows, natively and in
the tab: emit_instructions 5,687,958 -> 5,688,137 and
browser_compile_instructions 344,137,759 -> 344,146,623. Welfare falls by less
than a hundredth, and the floor is lowered by exactly that, since this makes native
build a program the language allows.

## 2026-10-26 — an err is born boxed

The build of the #2087 gavel. `err "…"` and every builtin that fails answer
a failure in a box on all three engines. Natively the box is `K_ERR` and the
err a rescue takes out of it is a new tag, `K_ERRV`; the interpreter has
`Value::Unboxed` for the second. A failure passes every function it reaches
and every arm, the three err patterns included, and the endpoint or a
foreign rescue ends it. A rescue's callback and an annotate's are handed the
unboxed err, which renders as `err <reason>`, compares by reason, and goes
back in a box through `effect`. A parameter annotated with an effect type,
`e:<some>effect`, takes the box itself, through `k_check_box`, which is how
std/testing reads a package's own failures.

The checker loses the raise analysis of #1884: the raised bit, `raised_err_at`,
the raisers table and the err-arm requirement are gone. In their place a
failure counts as a box, so a group whose every answer is a failure, or a
tail that calls `err`, is refused where a value is read, as an io is. A group
that can also answer a value passes, as one that can answer none does.

The library classified its bare errs. `text/to_int` and `text/to_float`
answer none for text that is not a number, and `to_int!` and `to_float!` in
std/text answer the box with the reason they used to give. `json/decode`
answers none and `json/decode!` the box carrying the `parse_failure`.
`text/utf8` keeps its failure. The checker's refusal of a literal that is not
a number moved to the bang forms. The hand-back arms in std/json, std/regexp
and the frozen benchmark copies are gone, since a failure passes them anyway.
std/testing's two functions take the box through an effect-annotated arm.

The corpora followed. Error fixtures pinning the raise analysis were deleted
and two moved to the runtime corpus, where the failure now passes the
operator or group they named. A micro sample that leaned on a plain
conversion failing uses the bang form, or a local function with a mixed arm
so its reason is unchanged. `an_arm_sees_its_own_hakos_err` and
`a_typeset_arm_sees_its_own_hakos_err` pinned the 2026-09-15 rule and now pin
its reversal as `a_failure_passes_its_own_hakos_arm` and
`a_failure_passes_its_own_hakos_typeset_arm`. `.cause` can be read only on an
err another package annotated, so it moved to a two-module golden,
`tests/golden/cause`. ch04, ch05, ch08 and appendix B were rewritten where
they described a bare err as data, and ch08's mini decoders answer their
`parse_failure` record as a value, the shape the ruling gives an outcome the
caller handles. kq and kanso-json carry the same changes on branches of this
name.

`an_err_as_pattern_keeps_a_carried_slot_unboxed` lost its shape: its scanner's
failure now passes the err arm, the loop becomes a beat, and the carry builds
the records whatever the arm does, so the exemption's mutation survived. The
fixture's walk now stops before the scanner fails, and the mutation reads
sh_rec 0 -> 64,000 again. A slot that can hold an unboxed err beside a
two-field record is passed boxed: answering a scanner's end with
`text/to_bytes [300] .? (e -> e)` reads sh_rec=64,064. Nothing in the tree
answers that shape, and the two-word convention could carry `K_ERRV` as it
carries `K_ERR`, which is an open lead.

Open: the tree's scripts still carry dozens of `e@(err _)` hand-back arms
that no longer fire; the ports carry hundreds; #2088's migration can now use
the bang forms at reads that must succeed.

CI's rows, against main at #2168. The checker loses the raise analysis, so
compiling gets cheaper: `compile_instructions` reads 24,604,939,
`entry_instructions` 72,571,754, `library_instructions` 72,232,245 and
`compile_allocs` 13,930, and the front end holds less while it checks:
`compile_peak_bytes` 607,213 -> 598,492, and `front_end_visits` reads 7,397
where it read 7,373. The interpreter no longer carries the raised bit:
`interp_instructions` 560,993,797, `interp_allocs` 630,584 and
`interp_peak_bytes` 1,306,819. In the tab, `browser_compile_instructions`
reads 339,964,747, `browser_compile_peak_bytes` 832,433 and
`browser_run_instructions` 61,425,132. The decoders lost their hand-back
arms, so the emitter writes less: `emitted_lines` 14,150, `emitted_calls`
1,519, `emitted_branches` 1,293, `emitted_other_lines` 168,876,
`emitted_other_calls` 20,311, `emitted_other_branches` 17,723 and `text`
5,619,835, and compiling it moves `codegen_instructions_dev` to 116,733,865
and `codegen_instructions_release` to 371,551,519. `emit_instructions` rises
to 5,689,859, 1,722 more; against main at #2162 it fell 491, and #2154's
fixpoints, which drop an entry as its check fails, sit between the two
readings, so the rise is recorded unisolated. `emitted_defines` rises to 160
and `emitted_other_defines` to 2,243 because `json/decode!` and the bang
conversions are functions of their own beside the plain forms; encodebench
and scanbench each lose one define with their hand-back helper. The run
rows: `work_jsonbench` 549,333,568, `work_runbench` 819,572,053,
`work_oneshot` 9,978,432, `work_livebench` 938,653,995 and `work_scanbench`
274,243 fall. `work_widebench` rises to 28,685,100, 80,002 more: five for
each element of its 16,000, and two besides, for the `written` arm the chunk
now passes, since `text/utf8` can answer none. `work_encodebench` rises to
2,335,339,266 (143,306 more, 0.006%) and `work_readbench` to 4,583,165 (7
more); nothing has isolated either, and they are recorded as moves of that
size. The mem sample `a_failure_crosses_a_beat_carry_in_two_words` imports
std/testing to read its own failure, and the import costs six allocations of
32 bytes: `a_failure_crosses_a_beat_carry_in_two_words_allocs` reads 1,009
and `a_failure_crosses_a_beat_carry_in_two_words_alloc_bytes` 64,352.
`an_err_as_pattern_keeps_a_carried_slot_unboxed` was rewritten, as above,
and reads 3 allocs and 112 bytes.

The ratchet found two rows that prove nothing. `a chain callback's parameter
seeded as never failing` made inference seed a rescue or annotate callback's
parameter without the failure bit; a callback is now handed the err out of
its box, an ordinary value, so the seed protects nothing and the row and its
mutation are gone. Seeding the parameter as never failing is now correct,
and is an open lead. `an encoder that hands a clean string to the escape
path anyway` repointed `escape_onto`'s clean-string arm, which lib/json's
encoder has not called since strings went out through `append_string` in one
call (197f172b), so the row was blind on main too and was only rerun here
because this branch touches lib/json. Its mutation now makes the encoder
copy each string before it writes it: livebench writes the same bytes and
the live vein reads 858,314 allocations where it reads 8,314. `escape_onto`
and its two helpers are dead in the library.

## 2026-10-26 — a none carried through a name is checked

#2088. A none handed straight to a call is refused at check when the
receiver has no arm for it, and the same none bound to a name first passed
check and stopped at run time. Since #1890 a slice past the end answers none,
so the ports hit the hole wherever they slice: regex broke 42 of 43 fixtures
at run time through one named slice. A name bound once to something that
provably answers none now carries the none into the check, so binding a
value never changes what compiles. A builtin has no arms to give a none to,
so the positions its demand table constrains refuse one: `length`, `push`,
`print`, the os builtins and the rest of the table, called directly or
through a name.

The tree had about 130 reads the checker cannot bound, most of them a bound
decided in one function and handed to the next as a boolean, the shape the
house rule asks a branch to take. Each read must succeed, so each now goes
through a `none` arm that answers an err. Since #2087 that err is a box that
passes everything above it, so no caller gains an arm. Most scripts take one
helper, `present`, and the library, the book's escapes sample and five
benchmarks each take a small arm beside the read. A default value in the arm
would have hidden the bug the check exists to catch, and no site takes one.
Where a none was an expected outcome, welfare_rescore and grammar_check name
it with a marker or keep its arm. kq and kanso-json carry the same change on
branches of this name.

Six error fixtures pin the refusal, named and inline, for a group with no
arm, `length`, `push` and `list/concat`, each seen passing check on the
#2087 tree.

The test corpora read the same way. Nine micro samples and nine mem
fixtures route a read through a none arm, and none of their outputs or
counters moved. `a_box_where_a_value_is_expected` gains one leading
diagnostic, since `print os/args[1]` hands `print` a none. The test verb's
`fails_in_std` carries its slice in a record field so the none still reaches
`text/split` at run time, and its called-from line moves from 13 to 18. The
last item on #2088, bc's slice edges, was built on 2026-10-25: a start before
1 answers none, as a range past the end does.

CI's rows against main at #2169. The check costs the front end:
compile_instructions 24,604,939 -> 24,798,984, entry_instructions 72,571,754
-> 73,507,587, library_instructions 72,232,245 -> 73,143,499 and the tab's
browser_compile_instructions 339,964,747 -> 340,825,316. indexbench reads its
bound at the guard on every step, `if (i > length s)`, because the prover
cannot see a bound handed down (#2174), so work_indexbench rises 1,837,160 ->
2,077,157, and work_runbench, which carries the shape at a larger size,
819,572,053 -> 829,234,755. The none arms the library and the benchmarks
gained move the rest by small amounts: work_jsonbench 549,333,568 ->
549,337,021, work_widebench 28,685,100 -> 28,701,099, work_basket 18,294,810
-> 18,294,814, work_encodebench 2,335,339,266 -> 2,335,338,865,
work_scanbench 274,243 -> 274,220 and interp_allocs 630,584 -> 630,592. The
emitted and text rows grow with the same arms: for the decoder
emitted_defines 160 -> 161, emitted_calls 1,519 -> 1,528, emitted_branches
1,293 -> 1,308 and emitted_lines 14,150 -> 14,240; for the other benchmarks
emitted_other_defines 2,243 -> 2,253, emitted_other_calls 20,311 -> 20,472,
emitted_other_branches 17,723 -> 17,764 and emitted_other_lines 168,876 ->
169,582; and text 5,619,835 -> 5,620,636 over every benchmark. The check is the 2026-09-15 gavel's part 3
carried to none, so the floor drops by what it costs, 95.42 to 95.38.

## 2026-10-26 — a length handed down a loop bounds its reads

#2174. A loop that walks a list by index usually hands its bound down once,
`walk xs 1 (length xs) 0`, and guards with `i > n`. The prover had two doors
for the upper bound of `xs[i]`, a guard written against `length xs` and the
length a literal list spells out, and the guard against `n` passed neither,
so a read like `twice xs[i]` was refused unless the loop read `length xs`
again on every step. #2088 makes that the common case, since a name bound to
such a read now carries its none too: bench/indexbench, migrated to `i >
length s`, reads 13% more instructions for it.

`handed_lengths` (`src/infer.rs`) is a third door. It proves that a
parameter holds another parameter's length for a whole arm on the terms
`counter_floors` proves a floor: the group is not pub and never travels as a
value, every call from outside passes `length X` at the bound's position and
the same name `X` at the list's, every self-call passes both on unchanged,
and no arm rebinds either. The arm starts with that fact, and a guard `i >
n` gives the read its upper bound through it.

`a_length_handed_down_bounds_the_read` (micro) prints `20 0` on both
engines, and main refuses it. `a_length_of_another_list_bounds_nothing` and
`a_bound_that_grows_bounds_nothing` (errors) keep the two near misses
refused. The ratchet row `a length handed down a loop proving nothing` drops
the fact and watches the micro corpus go red.

CI's rows against main at #2170. indexbench goes back to handing its bound
down, and work_indexbench falls 2,077,157 -> 1,887,164; work_runbench, which
carries the shape at a larger size, falls 829,234,755 -> 821,299,208. The
interpreter does a little less, interp_instructions 560,993,797 ->
560,933,132. The front end pays for the walk and the facts:
compile_instructions 24,798,984 -> 24,862,623, entry_instructions 73,507,587
-> 73,686,908, library_instructions 73,143,499 -> 73,325,258, compile_allocs
13,930 -> 13,941, emit_instructions 5,689,859 -> 5,702,866 and the tab's
browser_compile_instructions 340,825,316 -> 342,497,302. interp_allocs moves
630,592 -> 630,614, and nothing here isolates why. The emitted and text rows
fall with the guard's `length` call: emitted_other_calls 20,472 -> 20,468,
emitted_other_branches 17,764 -> 17,762, emitted_other_lines 169,582 ->
169,576 and text 5,620,636 -> 5,620,459. Welfare rises 95.38 -> 95.41 and the
rise is banked.

## 2026-10-26 — a list read for its last element joins the corpus

A battery of list shapes, each run at 5,000 and 20,000 elements on both
engines, found `list/last` read in a loop quadratic on both. `last` folds
the whole collection to find its last element, so each read costs the
length of the list: 5,000 reads of a 5,000-element list took 0.38 seconds
natively and 17.7 on the interpreter, and at 20,000 they took 4.6 seconds
natively, where the interpreter ran past two minutes. The other shapes,
`text/join`, `list/zip`, `list/count`, `list/reverse`, `list/sort_by`,
`list/tally`, `list/to_h`, `text/to_int` over a list and floats rendered by
interpolation, grow linearly or by n log n on both.

Under the 2026-09-05 corpus-first gavel `lasted` joins the run program with
800 reads and the interpreted corpus with 70, measured on main at #2158
before its fix. On this container the run program reads 865,303,604 with it
and 828,684,309 with it reading nothing, a column of 36,619,295 (4.23%), and
the interpreted corpus reads 565,857,214 -> 593,093,849. Both engines print
the same line for it.

The battery also found a field read that the engines disagree on: native
reads `e.value` from an entry, and the interpreter refuses it. #2163 asks
Clay which way it goes.

CI's rows, against main at #2175. The run program reads work_runbench
821,299,208 -> 859,187,448, and its allocation vein moves with the 800 reads:
run_allocs 1,581,589 -> 1,582,393, run_alloc_bytes 223,888,104 ->
223,955,352, run_beat_iters 177,272 -> 178,072, run_push_mut_slow 1,644,657
-> 1,645,457, run_sh_rec 4,098,336 -> 4,136,736, run_sh_buf 91,336,688 ->
91,336,800, run_perm_peak_bytes 1,867,025 -> 1,875,217, run_bytes_malloc
7,921 -> 7,924 and run_bytes_freed 7,779 -> 7,782. The interpreted corpus
reads interp_instructions 560,933,132 -> 588,905,757, interp_allocs 630,614
-> 656,976 and interp_peak_bytes 1,306,819 -> 1,327,257. The tab compiles
and runs the corpus too: browser_compile_instructions 342,497,302 ->
348,067,219, browser_compile_peak_bytes 832,433 -> 836,713 and
browser_run_instructions 61,425,132 -> 64,341,530. The run program's new
function is emitted code: emitted_other_lines 169,576 -> 170,115,
emitted_other_branches 17,762 -> 17,826, emitted_other_calls 20,468 ->
20,537, emitted_other_defines 2,253 -> 2,262, and text 5,620,459 ->
5,622,379. Under the corpus-first gavel each moved welfare term's baseline
is scaled so its ratio to what it reads holds, so the phase leaves welfare
where it was and the fix in #2165 scores as a rise.

## 2026-10-26 — the last element of a list is read at its end

`list/last` gains an arm for a list, `last xs:[]some`, which answers
`xs[length xs]`: the last element, or none for an empty list, as the fold did.
Anything that is not a list keeps the fold, so a lazy sequence, a string,
bytes and a map are walked to their last element as before.

5,000 reads of a 5,000-element list took 0.38 s natively and 17.7 s on the
interpreter, and take 0.07 s and 0.015 s. At 20,000 they take 0.09 s and
0.03 s, where the interpreter ran past two minutes.

`a_list_is_read_at_its_end` (micro, both engines) reads the last of a list of
numbers, of records and of lists, of one element and of none, and of a lazy
sequence, a string, bytes and a map. Its output is main's interpreter, and
both engines on main and on this branch print it. Reading one before the end
turns the micro corpus red.

The new arm moves std/list's lines below it down by four, so
`runtime/tie_takes_a_list_of_ids` names `list/tie` at line 602 where it named
598.

CI's rows, against #2164's phase on main at #2175. The run program's 800
reads cost what an index costs: work_runbench 859,187,448 -> 821,243,366,
below main's 821,299,208 without the phase, run_allocs 1,582,393 ->
1,581,593, run_alloc_bytes 223,955,352 -> 223,916,952 and run_sh_rec
4,136,736 -> 4,098,336, the 800 step records the fold built. The interpreted
corpus falls the same way: interp_instructions 588,905,757 -> 562,047,199
and interp_allocs 656,976 -> 631,738, while interp_peak_bytes reads
1,328,317, 1,060 more, for the library's extra arm. In the tab
browser_run_instructions falls 64,341,530 -> 61,478,759. The arm is library
code every compile reads, so the compile rows rise a little:
compile_instructions 24,862,623 -> 24,864,430, entry_instructions
73,686,908 -> 73,696,160, library_instructions 73,325,258 -> 73,344,720,
emit_instructions 5,702,866 -> 5,703,136, codegen_instructions_dev
116,733,865 -> 116,733,909, codegen_instructions_release 371,551,519 ->
371,551,607, browser_compile_instructions 348,471,187 and
browser_compile_peak_bytes 843,030. The run program emits the arm too:
emitted_other_lines 170,150, emitted_other_calls 20,540,
emitted_other_branches 17,831 and emitted_other_defines 2,262, and text
reads 5,622,043. Welfare rises 95.41 -> 95.52 and is banked.

## 2026-10-26 — a string handed on round a cycle joins the corpus

The corpus phase for the next fix, under the 2026-09-05 corpus-first gavel.
A battery of core-library shapes at 4,000 and 16,000 elements on both engines
found one cliff: natively, `regexp/replace_all` over 64,000 words took 49.8
seconds, where the interpreter took 4.8. Three separate causes sit under it,
each reduced to a loop of two groups that joins onto a string in place in the
first group and hands it to the second:

- `built` hands the join straight on, `built s (at + 1) "{acc}{s}x" n`.
- `marked` binds the join below a `return acc if …` guard, and the guard holds
  the statements after it.
- `tagging` is also declared with one parameter, as regexp's `swapped` is.

Natively, each hop copies the string built so far, so each loop costs the
square of its length: 80,000 steps of `built` took 13.9 seconds, of `marked`
11.4 and of `tagged` 2.3. The interpreter asks the same analysis and has the
third cliff too, at 1.57 seconds for 80,000 steps of `tagged` against 0.10
for 20,000, so `relayed 1500` joins the interpreted corpus: on this
container it reads 580,418,637 with it and 563,943,189 with it reading
nothing, a column of 16,475,448 (2.84%). The run program's module is
`bench/runbench/runbench/relayed`, at 3,000 steps for the first two and
1,500 for the third. On this container the run program reads 837,824,222
with it and 822,287,654 with it reading nothing, a column of 15,536,568
(1.85%). The run program's allocation vein moves with it. Welfare re-bases
each moved term once CI's rows are in, so the fix scores as a rise.

CI's rows, against main at #2165. work_runbench reads 821,243,366 ->
836,794,013 with the phase, and its allocation vein moves with the three
loops: run_alloc_bytes 223,916,952 -> 282,819,050, run_allocs 1,581,593 ->
1,596,597, run_arena_blocks 4 -> 5, run_arena_peak_bytes 4,718,608 ->
5,767,184, run_spare_peak_bytes 2,621,456 -> 3,670,032, run_beat_iters
178,072 -> 193,074, run_bytes_malloc 7,924 -> 13,926, run_evac_allocs 1,732
-> 1,744, run_evac_bytes 4,602,176 -> 4,635,024, run_perm_allocs 103 -> 106,
run_sh_str 16,769,952 -> 21,455,984, run_str_scan_bytes 18,352,322 ->
18,358,325 and run_str_scans 1,885 -> 1,888. The interpreted corpus reads
interp_instructions 562,047,199 -> 579,993,703, interp_allocs 631,738 ->
653,569 and interp_peak_bytes 1,328,317 -> 1,339,129, and the tab compiles
and runs it: browser_compile_instructions 348,471,187 -> 353,823,116,
browser_compile_peak_bytes 843,030 -> 847,565, browser_run_instructions
61,478,759 -> 64,252,748 and browser_run_peak_bytes 1,572,864 -> 2,621,440,
a page more for the copies. Under the corpus-first gavel each moved welfare
term's baseline is scaled so its ratio to what it reads holds, so the phase
leaves welfare where it was and the fix below scores as a rise.

## 2026-10-26 — a builder handed on round a cycle is still a builder

The fix for the corpus phase above. A string joined onto itself is a builder,
and the emitter converts a seed where it enters the group from outside, which
copies it. Three hops counted as entering when the string they carried was
the builder the join had just made:

- A join handed straight to the next group as an argument. The emitter knew
  a name bound to a join was carrying the builder, and asked nothing of the
  join itself.
- A join bound below a `return … if` guard. The guard holds the statements
  after it, and the walk for names bound to a join read only the function's
  own list.
- A group whose name is also declared at another count. A call at another
  count read as the group handed on as a value, called from somewhere no call
  site describes, so no caller could be proved to hand the string over. A
  call whose count is another declared group's calls that group, and only a
  count no group has still counts as the group escaping.

regexp's `replace_all` also handed `stepped_over` its accumulator beside the
join made from it, so the accumulator was read twice and could not be joined
in place; `stepped_over` never read it, and the argument is gone. Over 64,000
words `replace_all` took 49.8 seconds natively and takes 0.15. Each of the
three loops in `relayed` took between 2.3 and 13.9 seconds at 80,000 steps and
takes under 0.01.

Three mem fixtures pin the copies, each red with its own change undone and
green with the other two undone:
`a_join_handed_on_as_an_argument_is_still_a_builder` and
`a_join_below_a_guard_is_still_a_builder` read 198 bytes where main reads
5,337, and `a_name_at_two_counts_keeps_its_builder` 196 where main reads
1,936. `a_digest_holds_every_block_it_walked` hands a join on as an argument
too, and its vein falls: `allocs` 266 -> 123, `alloc_bytes` 18,657 ->
14,090, `bytes_malloc` 81 -> 10 and `sh_str` 1,344 -> 192. Three regexp
fixtures move too. `a_scan_keeps_its_place_in_the_text` reads `alloc_bytes`
106,537 (from 108,672) and `a_scan_that_finds_nothing_keeps_nothing`
2,781,801 (from 2,783,936), both with the argument change alone;
`a_split_stops_where_the_separator_does` reads 24,487 (from 24,639), and
undoing either the argument change or the count change puts part of that
back.

CI's rows, against #2171's phase on main at #2165. The run program reads
work_runbench 836,794,013 -> 825,009,976, 3,766,610 above main without the
phase, which is the three loops' linear work. The interpreted corpus reads
interp_instructions 579,993,703 -> 577,237,316: at 1,500 steps the copies
were 2,756,387 of the phase's 17,946,504, and `tagged` now grows linearly on
the interpreter as well, 0.04 s at 20,000 steps and 0.11 s at 80,000 where
main takes 0.12 s and 1.25 s. interp_allocs reads 653,378 and interp_peak_bytes 1,356,556. Natively the run
vein falls to run_alloc_bytes 224,043,124, run_allocs 1,582,921,
run_bytes_malloc 7,850, run_evac_bytes 4,602,208 and run_sh_str 16,816,384,
while run_beat_iters reads 193,074, run_perm_allocs 106, run_str_scan_bytes
18,358,325 and run_str_scans 3,387, a scan per lap where the copy used to
stand. regexp's argument change moves work_scanbench 274,220 -> 266,360 and
work_digestbench 4,122,972 -> 4,103,700, and their veins with it:
scan_allocs 149 -> 91, scan_alloc_bytes 11,069 -> 8,934, digest_allocs
7,194 -> 7,051 and digest_alloc_bytes 697,073 -> 692,506. The three
analyses cost the front end a little: compile_instructions 24,864,430 ->
24,873,029, entry_instructions 73,696,160 -> 73,712,018,
library_instructions 73,344,720 -> 73,349,447 and emit_instructions
5,703,136 -> 5,715,797, and the tab's browser_compile_instructions reads
353,650,925, browser_compile_peak_bytes 847,746 and browser_run_instructions
64,598,033. The emitted rows grow
with the new module: emitted_other_defines 2,262 -> 2,276,
emitted_other_calls 20,540 -> 20,700, emitted_other_branches 17,831 ->
17,925, emitted_other_lines 170,150 -> 171,135 and text 5,622,043 ->
5,624,571. Welfare rises 95.52 -> 95.55 and is banked.

## 2026-10-26 — lib/json drops the escape its encoder stopped calling

#2173. `escape_onto`, `escape_clean` and `escape_split` in lib/json had no
caller once a string value went out through `builtin_append_string`, and
nothing in this repository, kq or kanso-json calls `json/escape_onto`. They
are gone. The two ratchet mutations that wrote a string and a key the old
way now append the old escape to the library before calling it, and each
still turns its row red. A codegen comment and a runtime fixture that named
the escape describe what is still there.

CI's rows, against main at #2172. The library every compile reads is
smaller: compile_instructions 24,873,029 -> 24,871,095, entry_instructions
73,712,018 -> 73,693,860, library_instructions 73,349,447 -> 73,331,088,
compile_allocs 13,941 -> 13,940 and browser_compile_instructions
353,650,925 -> 353,638,408, browser_compile_peak_bytes 847,746 -> 847,738.
The interpreter does 62 instructions less, interp_instructions 577,237,254,
and one allocation less, interp_allocs 653,377. work_runbench moves
825,009,976 -> 825,010,172, 196 more; the run program never called the
escape, and nothing here isolates what moved it. Welfare rises within its
second decimal and is banked.

## 2026-10-27 — gavel: templates are Slim-like, and they compile

#2178. Clay, on how the framework's pages are written: "well you can find a
way to make the templates compilable but they should be like slim or Haml".
A template is markup structured by indentation, in the Slim family, and it
compiles to kanso, so the checker reads its field accesses and its
parameters the way it reads any other code. How a database gets in and what
the first application is stay open on #2178.

The prototype is on branch claude/kata-prototype under design/kata, a
generator written in kanso. A template becomes one function. Each `- if`
branch and each `- for` body becomes a helper taking exactly the names in
scope it uses, because kanso refuses an unused parameter, and a body that
uses none becomes a constant. A misspelled field in the sample page is
refused by `kanso check` at the generated line, and both engines print the
same page.

## 2026-10-27 — a dependency's diagnostic quotes its own line

The merged check names the file a diagnostic is about, and a module compile
rereads its files on the failing path so that each diagnostic quotes its
line. The root compile in src/lib.rs handed the renderer only the entry's
text, so a field misspelled in an imported module printed a header and a
location with nothing under them. The template prototype found it, when its
typo landed in a generated view. `render_merged` now rereads each file a
diagnostic names, on the path that is about to fail. An embedded std file
is named bare and is never read from disk, because a file of that name in
the working directory belongs to somebody else.

tests/golden/errors_module gains a_field_read_in_a_modules_second_file,
which on main printed no line. field_read_in_a_deep_library and
two_owners_tie_on_a_traveling_call gain the line their goldens had pinned
as missing.

## 2026-10-26 — gavel: an entry reads its key and value by name

#2163, option 1. An entry is an ordinary record, so `e.key` and `e.value`
read by name on every engine and are checked like any record's fields. The
pattern `(entry k v)` keeps working. Clay asked about Ruby's two-argument
blocks (`|k, v|`); destructuring the entry in a parameter already gives that,
and a two-argument walk over maps would need a twin of every list function.

## 2026-10-27 — an entry reads its key and value by name

#2163. A field read becomes a call on the field's getter, and getters were
made only for fields some type in the program declares. `entry` is built
into the language, so `e.key` had no getter: the checker refused it, and
`e.value` got past the checker only when std/list happened to declare a
`value` of its own, then failed in the interpreter. The checker now counts
`key` and `value` as declared, the way it counts an err's three readers.
`prune_unused_getters` already gathers every getter a program names, and
when `Get_key` or `Get_value` is among them it writes that group an `entry`
arm, keeping one a dependency already wrote. A program that never reads the
fields builds neither arm. The first version built both in every compile and
pruned them after, which CI read as 63 more allocations in the interpreter's
corpus and 502,143 more browser compile instructions.

tests/golden/micro/an_entry_reads_its_key_and_value_by_name reads both
fields beside a local type that declares a `key` of its own; on main it was
refused at `e.key`. tests/golden/errors/an_entry_has_no_field_but_its_key_and_value
pins the refusal of a field an entry does not have. Appendix B names the two
fields.

CI's rows, on main after #2179. The tab compiles the corpus in fewer
instructions, browser_compile_instructions 353,638,408 -> 353,555,358, and
the interpreter's run allocates less, interp_allocs 653,377 -> 653,375, as
does the front end, compile_allocs 13,940 -> 13,939; nothing here isolates
why any of the three fell. The compile rows rise with the new code in the
checker and the prune: compile_instructions 24,871,095 -> 24,874,251,
entry_instructions 73,693,860 -> 73,701,590 and library_instructions
73,331,088 -> 73,338,936. Welfare rises within its second decimal and is
banked.

## 2026-10-27 — the compiler is built as one codegen unit, with fat LTO

The release profile had no settings, so rustc split the crate into sixteen
codegen units by module and optimized each without the others. Cargo.toml
now asks for one unit and fat LTO. The optimizer sees the whole crate and
its dependencies when it inlines, and a release build takes about a minute
longer, 2m34s against 1m41s on this container.

It came out of #2137. Moving call emission out of emit.rs had been declined
because the browser row moved 52 units with no code changed, and the guess
was the file path in panic strings. Neither a longer path nor a shifted line
number moves the row. The units do: they are cut along module lines, so a
move between modules changed what was inlined. With one unit that coupling
is gone, and the split can be retried.

The interpreter gate anchors its count on `run_interpreted_on_stack`, and the
one-unit build inlined it into its thread's closure, so the gate read a drop
glue carrying its name. It is now `#[inline(never)]`, as `emit_ir_for` is.

CI's rows, on main after #2180. Every row the compiler's own build reaches
falls: compile_instructions 24,874,251 -> 24,057,877 (-3.28%),
entry_instructions 73,701,590 -> 71,165,551, library_instructions
73,338,936 -> 70,852,827, emit_instructions 5,715,797 -> 5,597,787,
startup_instructions 52,528 -> 50,254, interp_instructions 577,237,254 ->
571,649,741 and, in the tab, browser_compile_instructions 353,555,358 ->
343,131,848 (-2.95%). codegen_instructions_release moves 371,551,607 ->
371,551,576; it counts clang and the linker, and nothing here isolates why
they did 31 fewer. No allocation, memory or runtime row moved. Welfare rises
95.55 -> 95.57 and is banked.

## 2026-10-27 — call emission moves from emit.rs into calls.rs

#2137. The methods from `emit_call_full` to the end of emit.rs form a second
`impl Backend` block in src/codegen/calls.rs, read with include! right after
emit.rs, so they stay in the codegen module. emit.rs falls from 4,324 lines
to 3,001.

The same cut was declined before #2181 because the browser row moved 52
units with no code changed. Built as one codegen unit, the compiler binary
now differs only in the file names its panic messages carry, and on this
container, which reads CI's rows exactly, compile_instructions, the entry,
library, emit and interpreter rows and the browser row all read what main's
goldens hold.

## 2026-10-27 — the unbuilt-arm prune moves from codegen.rs into unbuilt.rs

#2137. `without_unbuilt_arms`, `kept_arms`, `Given` and `UnreadPositions`
move into src/codegen/unbuilt.rs, read with include! where they stood, so
the items keep their order and their module. codegen.rs falls from 4,231
lines to 3,987. One ratchet mutation patched the prune by its text and now
names unbuilt.rs. As with calls.rs, the compile, entry, library, emit,
interpreter and browser rows read what main's goldens hold on this
container.

## 2026-10-27 — FnEmit moves from codegen.rs into fn_emit.rs

#2137. `FnEmit`, the impl that keeps it, and the helpers beside it
(`release_cells`, `writes_state`, `is_a_stack_slot`) move into
src/codegen/fn_emit.rs, read with include! where they stood. codegen.rs falls
from 3,987 lines to 3,443. Three ratchet mutations patched FnEmit's methods
by their text and now name fn_emit.rs. The compile, entry, library, emit,
interpreter and browser rows read what main's goldens hold on this container.

## 2026-10-28 — gavel: senro, with kata inside it, Postgres over the wire, and a guestbook first

#2178. Clay ruled four things about the web framework:

- It is named senro (線路, railway track).
- kata, the Slim-style template compiler, lives inside senro as its view
  layer.
- The database is Postgres, reached by a client written in kanso that speaks
  the wire protocol over a socket. Clay: "I just want to do postgres. I don't
  know why any kind of sea binding is necessary." Development starts against
  Docker, and TLS for hosted connections comes from the runtime, backed by
  OpenSSL natively and a Rust TLS library in the interpreter.
- The first application is a guestbook.

The build order is cloud's: `net` dials out and reads bytes, then the
Postgres client, then derivations (#2186), then routing with kata, then
models.

## 2026-10-28 — gavel: a framework's conventions are derivations

#2186. Clay asked for "some language construct that can allow you to do
compile time mapping", so that senro can map controller functions to routes
and model names to tables the way Rails does, and the compiler can check the
result. A derivation is a pure kanso function that a framework ships and the
compiler runs on the interpreter during the build. It reads a description of
the program and returns declarations built from constructors the compiler
defines (a route, a record type, a table binding), each naming the source
item it came from. The compiler adds them to the program and checks them like
any other code, and `kanso derived` prints them.

Inside a derivation any computation is allowed, including pluralizing a
model's name. What comes out is a finite list that is checked against real
functions and real tables, which is what makes an arbitrary convention safe
to run.

A pub function in `app/` that matches no route is a compile error. Clay
first agreed to a schema file written in kanso, then asked why Rails keeps a
schema file at all. Rails keeps `schema.rb` mainly because old Ruby
migrations stop running when the models they call change. A kanso migration
is a value built from a fixed set of changes, so it cannot break that way,
and the ruling became: no schema file by default, the schema folded from the
migrations at compile time, and `schema.sql` only for a migration that needs
raw SQL.

## 2026-10-28 — net dials out and reads bytes

#2178, the first item owed on the Postgres ruling. `net/dial host at` opens
a connection and answers the same `conn` record `accept` does, so `write`,
`read` and `close_conn` work on it unchanged. `net/read_bytes c` is one read
of what has arrived, handed back as bytes. A Postgres message is framed in
binary, and `net/read`, which answers text, would mangle a byte such as 0xff.

Both are effect builtins on both engines with sockets: `k_dial` resolves the
host with getaddrinfo and tries each address it gets, and the interpreter
uses `TcpStream::connect`. A refusal is worded "cannot connect to host:port"
on both, with nothing from the operating system in it, so the differential
law holds on the failure too. The browser has no sockets and refuses as it
already did for `listen`.

tests/a_program_dials_out.rs plays the server: it answers a greeting with
nine bytes framed like a Postgres message, one of them 0xff, and both engines
print the same nine values. Watched red with the native byte read decoding
as text (the 0xff arrived as U+FFFD) and with the interpreter's refusal
worded differently. tests/sockets_say_one_thing.rs gains `read_bytes` on a
handle that is not a connection.

Every binary links the runtime, so each benchmark's text grows by 752 bytes
(text 5,635,099 summed), and its instructions by a few hundred, which is
layout: no benchmark dials out. They land at work_basket 18,295,655,
work_deepbench 198,243,012, work_digestbench 4,104,311, work_encodebench
2,335,339,667, work_escapebench 40,308,822, work_indexbench 1,887,608,
work_jsonbench 549,337,631, work_livebench 938,655,028, work_oneshot
9,979,724, work_pendbench 154,124,432, work_readbench 4,583,812, work_runbench
825,011,048, work_scanbench 266,891 and work_widebench 28,701,733.

`lib/net` is embedded in the compiler, so the compile rows rise too:
compile_instructions 24,061,034 (+3,157), entry_instructions 71,170,086
(+4,535), library_instructions 70,857,236 (+4,409), emit_instructions
5,599,525 (+1,738), codegen_instructions_dev 116,797,274 (+63,365),
codegen_instructions_release 371,551,798 (+222), interp_instructions
571,786,910 (+137,169), browser_compile_instructions 343,160,078 (+28,230)
and browser_run_instructions 64,598,060 (+27). Welfare fell by less than
0.01, and the floor moves to 95.568 under the rule for a feature a ruling
requires.

## 2026-10-28 — derivations: a convention runs while the program compiles

#2186, the first slice of the ruling. An entry may write `derive
conventions/routing` after its imports. The compiler builds a
`derive/program` describing the app's own modules, which are the ones the
entry imports by a `./` path. Each module is listed with its pub functions,
their parameters and annotations, and the function itself, and with each
record type and its fields. The compiler calls the named function on the
interpreter and reads back a list of declarations. `derive/declare` makes a
constant in the entry, written out as the literal its value spells, with a
function written as its name, so a route table names real handlers and the
checker resolves each one. `derive/refuse` makes a compile error at the item
it names, in that item's own file.

The derivation's module is compiled as an ordinary dependency. No executor
is supplied, so a derivation that answers an effect is refused. A name in
the deriving module that reaches a module it describes is refused where it
is written, because the description is the whole of what a derivation may
know about the app; what it declares then follows from what `kanso derived`
shows. The modules it describes count as used imports, because the
convention reads them. `kanso derived` prints each declared constant with
functions spelled by name.

The derivation runs on an interpreter that copies at every container write.
The first build asked the linearity analysis which writes could go in place,
and that analysis covers every function the program holds: on
bench/entry_corpus it was 24M of the derivation's 25.8M instructions. Without
it the derivation costs 1.8M.

tests/golden/derive holds nine programs, each run on both engines: a route
table that dispatches to real functions, a misspelled action refused on its
own line, a library beside the app left out of the description, and six ways
a derivation can be wrong. It was watched red with function references
written as none, with `../` libraries described, and with a derivation that
imported the module it described. bench/entry_corpus now derives a small
route table, so the entry row, and with it welfare's compile term, prices
what a derivation costs.

The entry row rises to entry_instructions 83,728,840 (+12,558,754): the
corpus gained two modules and a derivation. `lib/derive` is embedded in the
compiler, so the other compile rows move by layout: compile_instructions
24,064,178 (+3,144), library_instructions 70,863,285 (+6,049),
codegen_instructions_release 371,551,844 (+46), emit_instructions 5,598,851
(-674), interp_instructions 571,786,909 (-1), compile_peak_bytes 598,588 (+96)
and browser_compile_instructions 343,190,365 (+30,287). Welfare fell by 0.018
and the floor moves to 95.550 under the rule for a feature a ruling requires;
the fall is past what `--set` takes, so the floor was edited by hand and
`--set` recorded the reason.

Still owed on #2186: the browser compile's derivation golden, a record type
or table binding as a declaration, and the senro derivations themselves.

## 2026-10-28 — a derivation reads constants, declares types, and runs in the page

#2186's second slice. A module in a derivation's description now carries
`constants`, mapping each pub constant's name to its value, and constants
leave `functions`. senro/models needs this: an app keeps its migrations as a
pub constant, and a constant's function reference could not be called, so
the fold had nothing to read. A routing convention also stops seeing a
constant as an action.

`derive/declare_type origin name fields` adds a record type to the entry,
with getters for its fields, and `kanso derived` prints it as `type name
fields`. A name or field a program could not write, or fields that are not a
list, is refused. With it the ruling's three declarations are all here: a
route and a table binding are constants holding a map, and a record type is
its own declaration.

The page runs a derivation. tests/wasm_engine.rs hands the route-table case's
convention and posts module to the compiler in the tab and compiles the
entry, which derives on the interpreter there, and the program prints the
native golden's output. It was watched red with derivations switched off:
"unknown name `routes`".

Chapter 11 of the book gains a section on `derive`, with the route table as
an executed sample, and the book's highlighter and the editor grammar paint
`derive` as a keyword.

Every compile that derives loads the larger std/derive, so the entry row
rises to entry_instructions 83,914,234 (+185,394); on one binary here
load_dependencies took 129K of a 188K rise and the merged check 37K. The
other compile rows move by layout: compile_instructions 24,063,854 (-324),
library_instructions 70,862,041 (-1,244) and browser_compile_instructions
343,187,975 (-2,390). Welfare fell by less than 0.001 and the floor follows
under the rule for a feature a ruling requires.


## 2026-10-29 — sha256 gains HMAC and PBKDF2, and base64 joins the library

Postgres logs a client in with SCRAM-SHA-256 (#2178), which needs HMAC,
PBKDF2 and base64. `std/sha256` gains `digest`, the hash as bytes, `hmac key
message` and `pbkdf2 password salt times size`, and the new `std/base64` has
`encode`, `decode` (none on input that is not base64) and `decode!`. HMAC
hashes each of its two key blocks once and keeps the state it leaves, so
every HMAC under one key starts from there, which is most of what PBKDF2
does: each of its rounds compresses two blocks instead of four.

The tests are the published vectors: FIPS 180-4 and the two lengths either
side of the padding boundary, RFC 4231's HMAC cases 1, 2 and 6 (6 has a key
longer than a block), PBKDF2 at one, two and 4096 rounds and RFC 7914's
two-block vector, and RFC 4648's base64 cases both ways. The guestbook
prototype logs in to Postgres 16 with these.

std/sha256 is imported by the entry and library corpora and by digestbench,
so their rows move. The compile rows rise with the module: entry_instructions
84,707,709 (+793,475) and library_instructions 71,324,471 (+462,430).
digestbench compiles four more functions (emitted defines=118 calls=871
branches=923 lines=7998) and its run rises to work_digestbench 4,104,363
(+52), text 341,377 (+16); runbench carries the same module and reads
work_runbench 825,011,765 (+717), text 819,297 (+16), with emitted defines=856
calls=8793 branches=7545 lines=67140. Summed over
the benchmarks the emitted counters read emitted_other_defines 2,278 (+2),
emitted_other_calls 20,709 (+9), emitted_other_branches 17,933 (+8) and
emitted_other_lines 171,268 (+133), and text 5,635,131 (+32). Welfare fell by about
0.001 and the floor follows under the rule for a feature a ruling requires.

## 2026-10-29 — a field read in an import no longer makes the importer's name private

#2189. A module that declares a name one of its imports also exports keeps
its own arms under the qualified spelling and moves the import's twin into
the bare overload space. `qualify` told the two apart by file: a twin is
cloned from the import's declaration and carries the import's file, while
everything the module writes carries one of its own. The getter for a field
counted as one of the module's own declarations, and its arms carry the file
of each type that has the field, an import's among them. So when `entries`
declared a record with a `name` field, `entries.kso` counted as one of
`home`'s files, `entries/index`'s twin read as `home`'s own `index`, took
`home/index` first, and was not pub: "`index` is private to module `home`".
Getters are now left out of the module's files.

The reduced program is three files in tests/golden/field_reads, run on both
engines by `a_field_read_in_an_import_leaves_the_importers_names_its_own`.
It was watched red with the old filter.

The change moves the compile rows by layout: compile_instructions 24,064,926
(+1,072), entry_instructions 84,709,988 (+2,279), library_instructions
71,325,972 (+1,501) and browser_compile_instructions 343,196,940 (+8,965).
The interpreted run allocates one value fewer, interp_allocs 653,374 (-1).
Welfare fell by less than 0.0001, and the floor follows under the rule for a
change that makes the language do what its specification says.

## 2026-10-29 — a server written as one statement waits for its client

Outside a parallel group, `net/accept` answered "nothing connected" at once
on both engines, on the reasoning that no other fiber could connect. A
client outside the program can, and a server is usually written as one
statement, so the guestbook prototype exited before a browser reached it.
`accept` there now waits the way accept(2) does. Natively, `k_accept_waiting`
clears the non-blocking flag a group's polling may have left and blocks; the
interpreter's executor gains `accept_waiting`, which does the same with
std::net. Inside a group `accept` is still a scheduling point.

tests/sockets_serve.rs runs a one-statement server on both engines and has
the harness say hello to it. It was watched red with the old behaviour: the
program exited before the harness could connect.

Every binary links the runtime, so each benchmark's text grows by 448 bytes
(text 5,641,403 summed), and its instructions move by layout: no benchmark
accepts a connection. They land at work_encodebench 2,335,339,772 (+105),
work_runbench 825,011,247 (-518), work_livebench 938,654,671 (-357) and
work_oneshot 9,979,360 (-364). Building costs more for the larger runtime:
codegen_instructions_dev 116,812,856 (+15,582) and
codegen_instructions_release 371,561,078 (+9,234). Welfare fell by less than
0.001, and the floor follows under the rule for a change the language needs:
a server written as its specification shows it, one statement, has to wait
for its client.

## 2026-10-29 — a sibling module's diagnostic quotes its line

#2188. `render_merged` took a bare file name for an embedded file and never
read it from disk, on the reasoning that a shipped module's files are the
bare ones. They are named under `std/`, and a bare name is a file beside an
entry run from its own directory, so a refusal in a sibling module printed
its location and no source line there. It now reads a bare name when the
entry's own name is bare too.

tests/golden/errors_module gains a sibling-file case, which a spec runs from
inside its directory; it was watched red with the line missing. One error
golden, a_field_read_of_a_list, had pinned the missing line: its fixture runs
as a library beside a generated entry, both bare, and it now quotes the line
it points at.

The browser has no disk to read the line from, so the page answered the same
refusal without it, and tests/wasm_engine.rs's error-corpus spec, which holds
the page to every error golden, went red on the other host. A file the
compiler was handed as source is now read from what it was handed before the
disk is tried. The spec was watched red without that, on
a_field_read_of_a_list, and passes with it.

The browser compile moves by two instructions, browser_compile_instructions
343,196,938 (-2), and no other row moves.

## 2026-10-29 — a derivation sees the calls a module makes, and can refuse one

#2186's routing convention owes a compile error at a `path_to` that names a
function that is not a route. A derivation could see an app's functions but
not their calls, so it could refuse only at a declaration. A module in the
description now carries `calls`: each call a function of the module makes to
a named function, with the caller, the callee, each argument's name ("" for
one that is not a name), and where it is written. `derive/refuse_call` makes
a compile error at one of them, in the caller's file.

Building the case for it found that a module the entry imports could arrive
described with nothing in it. When another of the entry's modules imports it
first and does not re-export it, its declarations come in already demoted,
and the description read each declaration's own pub flag. It now reads the
entry's exports, which say what reaches the entry as pub by any route.

tests/golden/derive gains a derivation that lists the calls between modules
and one that refuses a link to a function that is not a route, at the call.
The second was watched red twice: with no calls to read, and with
`posts/show` refused too before the exports fix.

bench/entry_corpus derives, so the entry row prices collecting the calls:
entry_instructions 85,153,572 (+443,584). `lib/derive` is embedded in the
compiler and grew, so the other compile rows move with it:
compile_instructions 24,083,702 (+18,776), library_instructions 71,386,812
(+60,840) and browser_compile_instructions 343,219,110 (+22,172). Welfare fell
by 0.002, and the floor follows under the rule for a feature a ruling
requires.

## 2026-10-29 — a parameter is not the function it shares a name with

The none check asks, at each call whose argument can be a none, whether the
callee's group has an arm for one. It found the group by name and arity
alone. std/list's merge sort takes its key as a parameter named `key` and
calls it on an element it reads by index, so a program with a one-argument
function named `key` had that parameter read as its own function, and the
read was refused twice inside std/list, at a line the program's author
cannot change. The proof that the index is in bounds usually kept the
question from being asked. A dispatch tie elsewhere in the program took that
proof away, so the tie arrived with two errors nobody wrote. The senro
routing prototype found it.

The check now asks first whether the declaration binds the name, the
question the box check already asks through the same per-declaration set.

tests/golden/errors gains the program: a function named `key`, a tie, and a
call into std/list. It was watched red through the corpus runner with the
test switched off, reporting the two refusals in list.kso, and passes with it.

The compile rows move by layout: compile_instructions 24,095,553 (+11,851),
browser_compile_instructions 343,289,574 (+70,464), entry_instructions
85,147,962 (-5,610) and library_instructions 71,380,536 (-6,276). Welfare
falls by less than 0.001, and the floor follows under the rule for a change
that makes the language do what its specification says.

## 2026-10-29 — six messages that named the wrong cause

Two messages a newcomer meets in the first hour named the wrong cause
(#2217). With no clang on `PATH`, `kanso run` and `kanso play` said
`cannot build: No such file or directory (os error 2)`, which reads as a
missing source file, and `kanso build` said it could not invoke clang
without saying where it had looked. All three now say clang is not on
`PATH`. `run` and `play` also name the way out that needs no clang:
`kanso run h.kso --interp`. The check runs only when the spawn reports
not-found and no `clang` resolves on `PATH`. Any other build failure
keeps the host's io text. `run` on a directory quotes the path as typed,
not the entry it was taken to.

`kanso test` on a file of bare statements was refused as a library with a
statement in it. The file declares nothing, so the message described its
shape and not the problem. It now says the file holds statements and no
tests, where tests live, and that `kanso run` runs it.

`kanso --help` now lists `--counters` beside `--release` on `build` (#2215).
Before this, the only place a user learned the flag was the refusal a
release binary prints when asked for counters it was built without.

The clang messages are asserted whole in
`tests/a_toolchain_the_path_cannot_reach.rs`, now three cases with `play`
added. They stay listed in `tests/golden/unpinned_diagnostics.txt`, which
says where their pin lives. The test message has a golden in
`tests/golden/test_verb/statements.stderr`, and `front_door` checks the
help line. Each spec was watched red on the compiler before the change.

Two more came from the hako commands (#2234). `kanso install` printed
`installed 0 hako(s):` above a list of the one it had pinned, because the
count it printed was the names left to walk, which is none by the time it
prints. It now counts the lines of its own report. And an import of a module
an installed hako does not have, such as `kanso-lang/vse/methods`, said the
hako was not in the cache and to run `kanso install`, which changed nothing.
The resolver now checks whether the locked tree is there before blaming the
cache, and when it is, names the modules the tree does have:
``error: cannot resolve import "acme/widgets/gadgets" — `acme/widgets` is
installed, and has no module `gadgets` (its modules: `acme/widgets`)``.
tests/hako.rs asserts both, and both were watched red.

The sixth is #2205's last item. A test run from inside its module imports
the module as `../<name>`, so a refusal in a sibling pointed at
`../t/p.kso` where the reader sees `p.kso`. A module file whose path climbs
out of the working directory and back in is now shown from the working
directory. The check is made only for a path that starts with `..`. The
refusal itself stands: `p.kso` holds a `pub fn` and a statement, which is
neither a library nor a play file. tests/test_verb.rs pins the message,
watched red with the old path.

With #2209's http change these move the compile rows by layout:
compile_instructions 24,096,605 -> 24,096,621, library_instructions
71,383,017 -> 71,383,019, startup_instructions 50,258 -> 50,260 and
codegen_instructions_release 371,573,437 -> 371,573,350. A first version
kept the path as typed for every verb, which cost the front end one
allocation; it is now kept only when a directory is taken to its entry,
and compile_allocs and interp_allocs are unchanged. Welfare falls by less
than 0.0001, and the floor follows under the rule for a change that makes
the tools do what the documentation says.

## 2026-10-29 — http answers any status, and its suite runs

#2209, filed by the docs worker. `std/net/http` could answer only 200 and
404, and `response` cannot be built outside the module, so an order service
answered a refused order with 200 and a sentence. `http/respond code headers
body` answers any status, with the reason phrase its code is registered with,
and an empty phrase for a code without one, which HTTP allows.

The suite beside it had never run. tests/std_suites.rs walked one level below
lib/, and lib/net/http is two, so `http_test.kso` passed or failed unread. The
walk now goes all the way down and pins the count at seven suites. With the
422 phrase misspelled, both of its specs went red on lib/net/http, which
neither could see before.

## 2026-10-29 — a reader lets the others run

#2208, filed by the docs worker. A program that is both ends of a connection,
dialling in one statement of a group and accepting in the next, never
finished on either engine: the client's `net/read` blocked the whole runtime
before the server's accept and write could run. `accept` already went back
in the queue when nothing was waiting. A read now does the same when nothing
has arrived: natively `k_step` peeks the socket without blocking and yields,
and the interpreter asks its executor whether the connection is readable
before it reads. A peer that has closed reads as ready, and the read answers
what it always did. Outside a group a read still blocks, as `accept` does,
because a server is often one statement and its client is outside the
program. `net/read_bytes` takes the same path; `net/write` does not yet.

tests/a_reader_lets_the_others_run.rs runs the issue's program with `read`
and with `read_bytes` on both engines under a ten-second deadline, in a
process group of its own so a hang is killed whole rather than leaving the
built program holding the test's streams. With the change reverted it failed
for the right reason, a native run that never finished.

The peek makes the runtime larger, so CI's rows move: codegen_instructions_dev
116,812,856 -> 116,821,778, codegen_instructions_release 371,560,989 ->
371,573,437, and each benchmark's text grows by 288 bytes, so text
5,641,403 -> 5,645,435. Run instructions
fall by a few hundred to two thousand a benchmark, runbench 825,011,247 ->
825,009,321. Welfare falls by less than 0.01, and the floor follows under the
rule for a change that makes the language do what its specification says.

## 2026-10-29 — the tab's list of machine calls has one home

#2219, a cleanup the docs worker filed. Which calls a tab cannot answer was
written twice: as `NEEDS_A_MACHINE` in docs/book-play.js, which decides which
book panels become live editors, and as a kanso list in the browser
differential, which decides which samples it covers. They drifted for months.
The differential now reads the list out of book-play.js, the way it already
reads the playground's examples out of play.js, and its own copy is gone. A
page without the list fails the run rather than covering every sample.

Read the same way, the page's list selects the same 717 programs the old copy
did. With `read_file` taken out of the page's regex the run selected 725, so
the differential follows the page.

## 2026-10-29 — gavel: `path_to` takes a record or a route's name

#2198. #2186 ruled that `path_to posts/show post.id` looks the route up in
reverse, and the gavel on equality refuses to compare two functions, which
the lookup needs at run time. Clay, ruling: "it could either take path_to
post, or the string form that specifies the path followed by the id... both
are available to the user. I'm not sure how you do that but you can be
clever."

So `path_to` has two forms. `path_to post` takes a record, and the record's
type picks its show route, with its `id` for `:id`. `path_to "posts/show"
post.id` names the route by a string. The form that takes a function is
dropped. The string form runs as a lookup in a map the routing derivation
declares, and the derivation refuses a string that names no route at the
call that writes it. The record form runs by dispatch on the record's type:
the derivation declares one `path_to` arm per model with a show route, which
needs a derivation that can declare a function arm (`derive/define`, planned
on #2198).

## 2026-10-29 — a call's description carries the strings it was written with

#2198's string form refuses `path_to "posts/shwo" id` at the call, so a
derivation has to read the string. Each `derive/call` now carries `strings`
beside `arguments`: an argument's text when it is a string written whole, and
"" for anything else, including a string with an interpolation in it.

tests/golden/derive gains a convention that refuses a link whose string
names no route, at the call, on both engines. On the compiler before this
change it was refused for a different reason, at the convention: no record
type had a field `strings`.

## 2026-10-29 — a derivation defines arms, and the module they belong to compiles them

#2198's record form runs `path_to post` by dispatch on the record's type, so
the routing convention has to write a `path_to` arm for each model. A
derivation can now answer `derive/define origin source`: an arm written out as
source, for the module `origin` names. A module may already add an arm to a
library's function for a type it declares, so the arm goes there and not in
the entry, which declares no models.

When any derivation defines an arm, the entry's dependencies load a second
time, and each module that was given arms reads them as one more of its own
files, `posts.derived.kso` beside `posts.kso`. The arm is qualified, formatted
and checked as code someone wrote in that module, and an error in it quotes
the line the convention wrote. An arm for a module the derivation was not
shown is refused at the `derive` line. `kanso derived` prints each arm under
the module it went to. In the tab the modules arrive as text and the arms join
them the same way.

tests/golden/derive gains three cases, each run on both engines: a `path_to`
arm defined for each record type in a module with a `show` action, an arm that
reads a missing field and is refused in the derived file, and an arm for
std/list, refused. With the second load switched off the first case printed
"no route" twice where it prints `/posts/3`. tests/wasm_engine.rs runs the
first case in the page; it was watched red with the derived files keyed so
the module could not find them.

The rows: bench/entry_corpus derives, so it pays for each call's strings and
for the larger `lib/derive`: entry_instructions 85,282,135 (+134,173),
compile_instructions 24,096,578 (+1,025), library_instructions 71,383,015
(+2,479), and browser_compile_instructions 343,289,572 (-2). The first build
keyed the derived files on every module load, which cost compile_allocs 14
and interp_allocs 12 on programs that derive nothing; the key is asked for
only while arms load, and neither row moves. Welfare falls by less than
0.001, and the floor follows under the rule for a feature a ruling requires.

## 2026-10-29 — a page split into words joins the run program

`regexp/split "[^a-z']+"` allocated about thirty times per field it answered
(#2214), and the run program never saw it: its one regexp phase, the scan
from scanbench, looks for a literal the subject lacks and stops early. Under
the 2026-09-05 corpus-first gavel the shape joins the run program here,
measured on main before its fix.

`worded` builds a page of 4,000 words from a fixed list, separated by
spaces, commas, full stops and newlines, and splits it with the pattern a
word count uses. On this container the release-tier run program reads
851,480,954 with the phase and 825,132,205 with it reading nothing, a column
of 26,348,749 (3.09%).

The interpreted corpus does not take the phase. Importing `std/regexp` there
made the tab compile the whole library on every run: CI read
browser_compile_instructions 343,289,560 -> 816,029,965 and
interp_instructions 571,786,909 -> 817,162,078, so the tab's compile term
would have measured one library's front end and swung with every edit to
it. The shape is the matcher's work at run time, which the run program
measures.

CI's rows for the phase: run_alloc_bytes 224,043,124 -> 231,013,169,
run_allocs 1,582,921 -> 1,700,863, run_arena_blocks 4 -> 5,
run_arena_peak_bytes 4,718,608 -> 5,767,184, run_beat_iters 193,074 ->
211,299, run_bytes_malloc 7,850 -> 7,867, run_evac_allocs 1,732 -> 1,736,
run_evac_bytes 4,602,208 -> 4,602,496, run_find2_calls 1,037,031 ->
1,060,857, run_perm_allocs 106 -> 127, run_push_mut_fast 1,191,311 ->
1,195,309, run_push_mut_slow 1,645,457 -> 1,645,463, run_sh_buf 91,336,800
-> 93,914,368, run_sh_bytes 4,872 -> 1,148,520, run_sh_rec 4,098,336 ->
4,675,424, run_sh_str 16,816,384 -> 16,930,368, run_spare_peak_bytes
2,621,456 -> 3,670,032, run_str_scan_bytes 18,358,325 -> 18,358,372,
run_str_scans 3,387 -> 3,402, work_runbench 825,009,321 -> 851,374,952,
emitted_other_branches 17,933 -> 18,066, emitted_other_calls 20,709 ->
20,888, emitted_other_defines 2,278 -> 2,292, emitted_other_lines 171,268 ->
172,430 and text 5,645,435 -> 5,658,091.
Welfare re-bases run_instructions and
run_peak_bytes, each keeping its ratio to its baseline, and the floor moves
only by the rounding of the two baselines.

## 2026-10-29 — a play file shows its plan

#2204, filed by the docs worker. `--plan` was accepted by `run` alone, and
`run` refuses a file that holds definitions, so a program small enough for
one file, which is how the book and the playground teach, had no plan to
show. Three of the book's panels are about what a plan shows and were getting
theirs through a stub entry no reader could type. `kanso play <file> --plan`
now prints the plan `run --plan` would, from the play file, and skips the
cached build.

Asking for it before had answered "`play` is not a verb". `play` was missing
from the list of verbs the usage message checks, and a known verb refused for
a flag fell to "wants a file or directory". The message now names the flag:
"`check` does not take `--plan`".

tests/a_play_file_shows_its_plan.rs runs the issue's program and compares the
plan with a golden; tests/front_door.rs gains `play` with no file and `check`
with `--plan`. Both were watched red on the compiler before the change.

## 2026-10-29 — check reads a play file as play

#2205's first item, filed by the docs worker. `kanso check` sent any file
with a declaration in it to the library compile, which refused a play file's
first statement and checked nothing else, so a play file had no check at all
and the book's `_check` samples had to stay `pub play` libraries. A file the
library parse stops at a statement, and for that reason alone, is now
checked as `kanso play` compiles it: a play file that would run reads `ok`,
and one that would not is refused with what `play` says. The library parse's
refusal is the one sign of the shape, so its wording now lives in one
constant that the parser says and `check` reads.

tests/check_reads_a_play_file.rs checks one play file that runs and one that
names an unknown name. The compiler before this change refused both with the
library's statement error.

## 2026-10-29 — an entry names a group

#2205's second item. ch06 teaches the named group, `prepare =` above
indented effect lines and then `prepare .> (_ -> ...)`, as the way to join
concurrent work, and it parsed only as a library constant. An entry or a
play file refused it with "only `if` opens an indented block", which named
nothing the reader had written. A binding whose header is `name =` alone,
above more than one indented line, now parses its body as a constant's body
is parsed, so its effect lines join into one group and run concurrently as
the constant's do.

tests/entry_file.rs runs an entry whose group sleeps in one member and prints
in the other, on both engines, and reads `fast`, `slow`, `done`, the order a
library constant prints. The compiler before this change refused it.

The three changes in #2240 move the compile rows by layout:
compile_instructions 24,096,605 (+27), library_instructions 71,383,017 (+2),
startup_instructions 50,258 (+4), codegen_instructions_release 371,560,989
(-89) and browser_compile_instructions 343,289,560 (-12). Welfare falls by
less than 0.0001, and the floor follows under the rule for a change that
makes the tools do what the documentation says.
