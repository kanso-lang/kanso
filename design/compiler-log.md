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


## 2026-10-13 — a list concatenated onto in a fold grows where it stands

From #1892's kv port. A fold that grew its accumulator with `concat` rather
than `push`, `list/fold ns [] (acc _ -> text/concat acc chunk)`, built a fresh
list every lap and kept them all until the fold returned: 4,000 laps of 25
ints, 100,000 elements, peaked at 3,061 MB and took 4.6 seconds natively. The
same fold written with `push` already ran in place.

`concat` now joins the accumulator writes the linearity analysis knows. A call
whose first argument nobody else holds is marked like an in-place push, and a
folder whose body is `concat acc xs` is a folder like one whose body is
`push acc x`. Native emits `k_b_concat_mut`, which puts each element of the
second list on the first one's frontier through the in-place push, so growth,
permanent storage and kept bignums follow the rules push already has. The
interpreter extends its list in place at the same sites. `concat acc acc`
names the accumulator twice and is never marked. kv's reduction now peaks at
10 MB and finishes in under a tenth of a second.

`tests/golden/mem/a_list_concatenated_onto_in_a_fold_grows_where_it_stands`
folds 2,000 laps of 25 ints. Main allocates 800,467,632 bytes and holds 1,070
arena blocks; with the change it allocates 1,401,904 and holds two. The same
fixture folds onto a seed its caller reads afterwards, and both engines print
the seed's length unchanged. The ratchet row "a concat accumulator copied
every lap" stops marking the sites and turns the mem corpus red.

`a_digest_holds_every_block_it_walked` moved too. std/sha256 builds its table
of round constants from nested `concat` calls, and the outer call's first
argument is the inner call's fresh list, so those calls now extend in place.
A push onto a full buffer grows it by doubling where the copying concat
allocated the exact length, so it allocates less often and more each time. Its
arena peak is unchanged.

The cost veins move the same way. On digestbench, which hashes with
std/sha256, the copied lists become pushes onto a growing buffer:
digest_alloc_bytes lands on 697,025, 2,240 higher; digest_sh_buf on 579,648,
2,304 higher; digest_push_mut_slow on 337, six more growths. Its allocs fall
four. The fixture's own rows move the
same amounts: a_digest_holds_every_block_it_walked_alloc_bytes lands on
18,657, a_digest_holds_every_block_it_walked_sh_buf on 12,288 and
a_digest_holds_every_block_it_walked_push_mut_slow on 33. encodebench grows
output lists with `concat`, so it gains more: encode_allocs 5,327,424 to
5,323,947 and encode_alloc_bytes 665,563,968 to 665,384,048, with
encode_push_mut_fast up to 34,874. encode_buf_reuse lands on 2,386, down 726,
because lists that were copied and freed each lap, leaving a buffer for the
next to reuse, now stay where they are. The instruction rows come from CI.

CI's rows, on the tree before kanso#1925. The runtime gains `k_b_concat_mut`, 3,104 bytes of text in every
benchmark. `work_runbench` lands on 1,170,036,819 (+1,739) and
`work_digestbench` on 5,605,158 (+2,040), the doubling growth's copies;
`work_encodebench` falls 99,808 to 2,398,273,418 and `work_livebench` 399 to
1,474,874,672, `work_oneshot` rises 161 to 13,227,091. The compiler carries
the new site: `compile_instructions` 25,846,237 (-35,314), `entry_instructions`
85,273,377 (-107,936), `library_instructions` 85,828,356 (-107,226),
`emit_instructions` 7,011,826 (-1,651), `codegen_instructions_dev` 119,010,910
(+8,081), `codegen_instructions_release` 454,481,233 (+301,820), and
`browser_compile_instructions` 391,400,519 (+89,017). The interpreter's
in-place extend reads `interp_instructions` 488,691,869 (-47,947),
`interp_allocs` 587,352 (+1) and `interp_peak_bytes` 689,271 (+2,112).
digestbench and runbench each emit one more line, and emitted_other_lines lands
on 148,240.

kanso#1925 joined the fold to the run program as its `stack` phase, measured
before this fix. On main after it, the run vein's allocation rows fall:
run_alloc_bytes lands on 386,136,876 (-2,581,984), run_sh_buf on 172,766,944,
run_sh_rec on 3,791,872 and run_allocs on 1,816,757. run_arena_blocks goes 9 ->
6, run_arena_peak_bytes 9,961,488 -> 6,815,760 and run_spare_peak_bytes
7,864,336 -> 4,718,608. run_push_mut_fast rises to 1,189,304, run_buf_reuse to
134 and run_push_mut_slow by twelve to 1,550,128. run_chain_finds goes back to
0, the value it had before kanso#1925 added the phases; the index's own counter
stays pinned by `a_long_chain_answers_survival_from_its_index` at 95,601. The
run program's instruction rows come from CI.

On main after #1904 the tab's compile reads `browser_compile_instructions`
392,470,701, up 92,326 from main's 392,378,375 on the same tree.

With main merged after #1922, browser_compile_instructions lands on
393,453,573.

## 2026-10-12 — gavel: arms travel with the value, and a file names only what it can see

Clay ruled the namespace half of #1883 in dialog, on 2026-10-11 and 2026-10-12.
It restates the 2026-08-29 gavel "arms travel with the type, under the
ownership rule" at full width: that gavel was built for `to_string` and
operator arms, and ordinary functions were never brought under it.
`enroll_bare` in src/lib.rs still gives every pub function of every import a
bare twin whatever the call passes, which is how a test file's
`import "std/list"` turned a module's own `step 3` into `list/step` on main.

Clay: "it's fine to call that method on an argument that's passed to a
function because its arms will travel with it. you just can't call it on your
own internal things if you haven't imported anything that defines it." And on
a name a file never imported: "you can't reference a type if you don't know
what version of the library it even came from... there could be a million
different libraries that all define that same thing."

**The ruling.**

1. A value carries the arms its type's owner wrote. Code holding the value
   calls them bare, without importing the owner: a function handed a `circle`
   may write `area x`. A value reached through one that arrived counts as
   arriving with it: a field of a parameter, an element of a list passed in,
   the result of a traveling arm called on it.
2. On values a file made itself (literals, its own records, what it built from
   them) a name resolves only if the file can see it, declared in its own
   module or in a module it imports. Otherwise it is `error[name]`.
3. A qualified call `geo/area x` needs the import, takes any arguments, and
   reaches only `geo`'s own arms.
4. An arm is legal only when one of its parameter types belongs to the module
   that writes it, so a value's traveling arms come from exactly one place,
   its type's owner. In a mixed call such as `combine x 3`, `x`'s owner's arms
   answer. Two owners' arms equally specific for one call are refused, as the
   2026-08-29 gavel on the ambiguous bare call already says, and a local arm
   of the same shape shadows a traveling one.

`kanso check` stays exact: a library is checked alone with `area x` on a
parameter accepted and `area 3` refused, and the program that passes the value
confirms an arm exists or is refused at its own call site.

Owes, on #1894: bare enrollment replaced by the two rules above; the
reproduction (a module's `step 3` hijacked by a test file's import) as an
error golden, seen red on main; a golden for an arm traveling into a library
that never imported its owner; an error golden for an internal value calling
a name the file cannot see; the corpus, samples, lib and the sibling
repositories moved to qualified spellings where they leaned on the old reach;
ch07 and ch11 rewritten to state the rule.

## 2026-10-12 — gavel: a test file is an outside consumer of its module

Clay ruled the other half of #1883: "kanso test certainly cannot see private
definitions. that would hugely violate the point of privacy." On main a
`*_test.kso` is a member of its module: it reads private names, its
definitions join the module's namespace, and `kanso run` checks it, so a
formatting slip in a test stopped the program it tests from running.

**The ruling.** A test file is compiled as an outside consumer that imports
the module it sits beside. It sees only pub names, adds nothing to the module,
and only `kanso test` reads it; `kanso run`, `check` and `build` ignore it.

Owes, on #1883: the loader keeps `*_test.kso` out of the module and compiles
each as an importer; goldens for a test that reads a private name (refused), a
test helper sharing a production name (no collision) and a malformed test
beside a program that still runs; existing tests that read private names
moved onto the pub surface.

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

Each benchmark binary now empties stdout before it reads its input, and the
benches that read or wait more than once pay it each time. On CI's rows
`work_deepbench`, `work_escapebench`, `work_indexbench`, `work_scanbench` and
`work_widebench` each rose 77 instructions, to 393,579,721, 39,049,235,
2,462,563, 273,600 and 28,178,586. `work_jsonbench` and `work_digestbench`
rose 100, to 860,563,084 and 5,603,218; `work_pendbench` and `work_readbench`
123, to 190,054,473 and 4,581,089; `work_basket` 130, to 30,876,618;
`work_livebench` 310, to 1,474,875,381; and `work_oneshot` 373, to 13,227,303.

On the merged tree the ratchet called "a warm play that dies in silence"
blind. Since #1900 the binary names its own stack overflow, so the warm play's
wording of the signal never runs for one. The mutation now also stops the
binary watching its stack, and the second play prints nothing.

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

With #1896 underneath, the rest of CI's rows on this tree:
`codegen_instructions_dev` 119,060,047, `browser_run_instructions` 34,125,731
and `text` 4,845,118. Among the benches, `work_pendbench` lands on 191,667,218
(+0.85%) and `work_widebench` on 28,274,761 (+0.34%); `work_basket`
30,876,728, `work_digestbench` 5,603,270, `work_jsonbench` 860,563,126,
`work_oneshot` 13,227,357, `work_readbench` 4,581,129 and `work_scanbench`
273,675 move by a few hundred instructions each.
The emitted totals: `emitted_other_branches` 16,476, `emitted_other_calls`
17,511 and `emitted_other_defines` 2,012.

This is the toml item on #1892.

On CI's rows for the tree merged with #1913: interp_instructions lands on
495,372,671, emit_instructions on 7,089,710, compile_instructions on
25,881,675, entry_instructions on 85,381,437 and library_instructions on
85,935,705. browser_compile_instructions lands on 393,094,499, up 1,782,997
or 0.46%, since the page's translator turns each new jump into a tail call.

On CI's rows for the tree merged with #1923: compile_instructions lands on
25,882,102, entry_instructions on 85,381,864, library_instructions on
85,936,071 and interp_instructions on 495,372,854, each a few hundred
instructions up. The merge brought twelve changed lines in the interpreter's
builtins and nothing else in `src/`; nothing here isolates which of the two
moved these rows.

On main after #1925 the emitted counts for the other programs land on
emitted_other_branches 16,559, emitted_other_calls 17,666 and
emitted_other_defines 2,037.

With main merged after #1937, browser_compile_instructions lands on 394,156,258
and browser_run_instructions on 34,126,073.

On CI's rows for the tree merged with main after #1937: interp_instructions
lands on 494,060,975, codegen_instructions_dev on 119,058,990,
codegen_instructions_release on 475,482,049, emit_instructions on 7,120,495 and
text on 4,850,094. Among the benches, work_pendbench lands on 191,674,062,
work_runbench on 1,179,917,338 and work_widebench on 28,274,796; work_basket
30,946,295, work_digestbench 5,604,610, work_escapebench 39,058,332,
work_jsonbench 860,566,126, work_livebench 1,474,875,027 and work_oneshot
13,227,755 move by a few hundred instructions each. The emitted totals for the
other programs: emitted_other_branches 16,563, emitted_other_calls 17,668 and
emitted_other_defines 2,038.

## 2026-10-13 — a lazy sequence handed where a list goes is refused at check

Five ports hit the same failure: `list/map` answers a lazy sequence, `text/join`
takes a list, `kanso check` said ok, and every engine stopped at run time with
`error[runtime]: join takes a list of strings and a separator` and no file or
line (#1892). `length` and `push` had the same gap, with a run-time hint
naming `list/to_list` and still no location.

The check now refuses the call at the argument. After inference it reads each
call of a builtin, or of a std wrapper that forwards to one, whose
`builtin_demand` entry holds a list. When the argument is a call of a std/list
group whose every arm answers a record and nothing else, the argument is a lazy
sequence, and the diagnostic says so: "`text/join` takes a list here, and
`list/map` answers a lazy sequence — make it a list with `list/to_list`". A
sequence held by name, a field, or any answer inference cannot pin to a record
is left to the run. `print` is not touched, because its demand holds no list
and printing a sequence forces it (gaveled).

`join` had no `builtin_demand` entry, so `text/join 3 ""` checked clean while
`length 3` was refused. It has one now: a list, then a string, which is what
both engines refuse.

`tests/golden/errors/a_lazy_sequence_where_a_list_goes` and
`join_takes_a_list` pin the two refusals. Three run-time goldens wrote the
failing call out and now hold the value by name, so they still reach the run
and still pin its sentence: `join_takes_strings_and_a_separator`,
`length_points_at_to_list` and `push_points_at_to_list`.

The check found the same mistake in `bench/compile_corpus`: `workload` joins
`list/take ["a" "b" "c"] 2`, which stops both engines when run. Nothing runs
the corpus, so it went unseen. It now goes through `list/to_list`, so the
corpus compiles a little more and the compile rows move with it.

The ratchet row "a lazy sequence where a list goes checks clean" deletes the
call that asks, and the error corpus goes red on the new fixture.

Measured here, the tab's compile row went from 391,974,060 to 392,906,566.
With the new call kept and the check returning at its first line it read
392,914,374, and with the call removed 391,981,161. The rise is in how the
wasm build lays out the walk the call sits in, not in work the check does:
across `bench/interp_corpus` five calls get past its first test.

On CI's rows for the tree merged with #1913, browser_compile_instructions
lands on 392,430,393, up 1,118,891. compile_instructions lands on 26,006,341,
entry_instructions on 85,527,812 and library_instructions on 86,082,490,
rises of 124,790, 146,499 and 146,908 for the check's walk and the corpus's
extra `list/to_list`. emit_instructions lands on 7,013,581, up 104, and
compile_allocs on 14,637, up 39. front_end_visits lands on 7,995, up 13, the
front end walking the corpus's new call, and compile_peak_bytes lands on
676,277, up 1,593, for the same call's tree. The refusal is
#1892's item, which makes `kanso check` say what every engine already
refused, so the floor drops by what it costs.

On main after #1904 the tab's compile reads `browser_compile_instructions`
393,497,266, up 1,118,891 from main's 392,378,375 on the same tree.

## 2026-10-12 — a name the author declared wins over a builtin and an import

#1891's first item, from the ports. Three reductions broke the 2026-08-29
ruling that an author's own name wins over an import's.

- **A type named like a builtin.** mustache declared `type run` and built one
  with `run 1 2 3`, importing nothing. The checker refused it twice over, with
  the count of the builtin std/os wraps and with that builtin's demand for a
  string. chip8's `type random` drew the same arity refusal. The checker let a
  declared function shadow a builtin and forgot a declared type; it now asks
  both. Native code had the same gap on the play route, where the name arrives
  bare: the build stopped on the builtin's count. Through an import the name
  arrives qualified and native already built the record.
- **A function named like an imported type.** mustache imported std/list and
  declared `fn counting a b`. std/list exports a record `counting`, which the
  import enrols under its bare name, and the interpreter built that record
  before looking for the file's group. It stopped on the field count where
  native called the function. A declared group now wins over an imported
  type of the same bare name.

`tests/play_verb.rs` pins both on both engines:
`a_type_named_like_a_builtin_is_the_files_own` and
`a_function_named_like_an_imported_type_is_the_files_own`, with fixtures in
`tests/golden/play`. The first also runs through an import as
`tests/golden/micro/a_type_named_like_a_builtin_is_the_programs`, which adds
`type random`. Main refuses both play fixtures. Four ratchet rows break one
guard each, and each turns its spec red.

The test-file half of the item (a test file's `import "std/list"` replacing a
private constructor) is #1883's, in kanso#1907.

CI's rows, on the tree merged with #1913: the shadow check costs each compile
a little. compile_instructions lands on 25,888,376 (+6,825),
entry_instructions on 85,392,486 (+11,173) and library_instructions on
85,946,778 (+11,196); browser_compile_instructions on 391,315,220 (+3,718) and
interp_instructions on 488,739,924 (+108). emit_instructions falls 1,084 to
7,012,393. The welfare floor drops by what this costs, under the rule that
building the language is priced at its cost.

On main after #1904 the tab's compile reads `browser_compile_instructions`
392,382,093, up 3,718 from main's 392,378,375 on the same tree.

## 2026-10-12 — `&`, `|` and `^` mixed without parentheses are refused

From #1893, four ports. The three bit operators share one precedence and
read left to right, so `a & b | c` is `(a & b) | c` and `a | b & c` is
`(a | b) & c`. C binds `&` tighter than `^` and `^` tighter than `|`, and
code carried over from it computed a different number in the second shape
with no complaint. A chain that mixes two of them is now a syntax error that
asks for parentheses around the part that goes first. A chain of one
operator, `1 | 2 | 4`, is unchanged.

The rule that refuses parentheses which group nothing had to learn the case:
`(a & b) | c` parses the same without them, and without them it is refused.
Parentheses around a bit chain that meets a different bit operator are now
kept. Around one that meets the same operator they still group nothing and
are still refused.

std/sha256 relied on the old reading in three lines, `spun`, `choice` and
`majority`, each of which meant the C grouping. They gain parentheses, as do
the micro fixture that copies them and `bitwise_operators`'s chained line.
Both engines print the same bytes for all three fixtures before and after.
`tests/golden/errors/bit_operators_mixed_without_parentheses` pins the
refusal, which main accepts and prints 10 for. The ratchet row "bit operators
mixed freely" lets the chain through and turns the error corpus red.

On CI's rows for the tree merged with #1913, compile_instructions lands on
25,908,556, entry_instructions on 85,478,744 and library_instructions on
86,033,220, rises of 27,005, 97,431 and 97,638 for the parser's check and the
parentheses std/sha256 now carries. browser_compile_instructions lands on
391,456,491, up 144,989, because the tab runs the same parser.
emit_instructions falls 2,649 to 7,010,828. The first CI run, before the
merge, showed the browser compile falling 546,685; the merged tree does not
repeat it, so that fall was layout and is withdrawn. The refusal is #1893's
item, which the issue lists as needing no ruling; it stops C-shaped bit code
computing a different number, so the floor drops by what it costs.

## 2026-10-13 — `and` meets `or` in parentheses, and a file may open with a header comment

Built under #1887 (ruled 2026-10-13), the two parts that change the grammar.
The third part, types before functions, stands as ruled on 2026-07-25 and
needs nothing.

**`and` and `or`.** `parse_and` now reports where the first `and` of its chain
stands, and `parse_or` refuses an `or` that meets one, on either side, at the
second of the two operators: `a and b or c` at the `or`, `a or b and c` at the
`and`. The diagnostic shows `(a and b) or c`. Those parentheses used to be
refused as grouping nothing, since `and` binds tighter. The check that refuses
a pair now keeps one whose contents are an `and` chain with an `or` beside it,
the same exception #1915 made for `&`, `|` and `^`, and `(a and b) and c` is
still refused. The refusal is in the parser, which every engine and the tab
share.

**The header comment.** The lexer drops comment lines, so the blank line under
a header looked like a blank line at the head of the file, and the refusal
named line 2 as where the file began. The lexer now records the last line of
the comment block a file opens with, when only blank lines stand above it.
One blank line directly under that block is allowed. A second is refused as
"one blank line separates the header comment from the code", and a blank line
above the header keeps "the file may not begin with a blank line". This holds
where the blank-line policy holds, in libraries and modules. A play file has
no blank-line policy, and it accepted all three shapes before this change too.

**Goldens.** Five error fixtures: `and_meets_or_without_parentheses`,
`an_and_on_the_right_of_an_or`, `parentheses_around_an_and_beside_an_and`,
`a_header_comment_with_two_blank_lines_under_it` and
`a_blank_line_above_the_header_comment`. The micro fixture
`and_meets_or_in_parentheses` opens with a header and prints three conditions
on both engines. On the base, the two `and`/`or` fixtures printed `true`, the
micro stopped at the blank under its header, and the blank-above fixture
reported lines 1 and 3 instead of line 1. The redundant-pair fixture reads the
same on the base, since it pins that the exception stops at `or`. Two ratchet
rows, "and meets or freely" and "header comment refused", were each watched
turning their test red.

**The corpus.** `kanso check` over every `.kso` file here and in kq, vse and
kanso-json finds no `and` meeting an `or` without parentheses; the sweep was
checked against a planted mix first. Nothing needed respelling. Appendix C
states both rules with a panel each.

The compile rows move with the binary; CI's rows are priced here before
merge.

- **OPEN** #1887, until this lands.

## 2026-10-12 — a wide tail call keeps its jump

The book promises that a call in tail position takes no stack, in every
engine. A release build broke that for wide arms. `narrow_tailcc` took
`tailcc` and `musttail` away from every arm wider than a limit (twelve words on
x86-64 under `preserve_nonecc`, nine without it, eight on arm64), and each
call into such an arm became an ordinary call. LLVM turns some of those back
into jumps, but not one whose arm builds a record on the stack and hands its
address to the runtime. The regex port's Pike VM loop is thirteen words, and
`regex --steps '(a*)*b'` on a 40,000-character line ran out of stack in a
release build while the dev build and the interpreter printed
`steps: 440010`.

An x86-64 release build no longer narrows: every `musttail` the emitter wrote
reaches clang, arms of twelve words or fewer take the flat `preserve_nonecc`
signature, and wider ones keep `tailcc`, whose stack arguments x86-64 lowers
correctly. arm64 keeps the narrowing at eight words, because with it gone the
macOS job ran `a_match_holds_the_literal_its_pattern_spells` to an empty
stdout; the arm64 backend still miscompiles a `tailcc` call that spills past
x7. #1912 holds the arm64 gap.

`tests/a_big_object_decodes_in_a_release_build.rs` gains a seven-parameter
self call that builds a record each hop, a million deep. Main's release
binary prints "the program ran out of stack". The ratchet row `nine_words` now
strips `musttail` from calls into arms left on `tailcc`, and both wide-arm
tests fail with the overflow. The older row "a release build that gives up the
guaranteed tail call" lowered only the arm64 cap, which an x86-64 release no
longer reads, so CI's ratchet called it blind. Its mutation now takes `tailcc`
off every arm on both architectures, and `a_loop_carries_a_bignum_across_its_rewind`
loses its last line in the release-built micro corpus.

CI's rows: `work_runbench` 1,170,035,080 -> 1,170,626,374 and
`work_digestbench` 5,603,118 -> 5,905,902 (+5.4%). sha256's block and round
loops take ten and eleven parameters, and each hop now copies its stack
arguments where LLVM had built a loop. `text` rose for scanbench (448,985 ->
451,401), digestbench (319,241 -> 319,705) and runbench (619,081 ->
619,849), so the `text` total went from 4,838,622 to 4,841,502 on main after #1925, and the emitted line count fell by 540 for both scanbench and
runbench. The language fix lowers the welfare floor by exactly what it costs,
to 91.11.

Because this branch edits src/main.rs, the ratchet ran the rows that patch it,
and "a warm play that dies in silence" stayed green under its mutation. The
row went blind when a built binary began naming its own stack overflow: the
warm play no longer sees a signal from that program, so its own wording never
runs. `a_warm_play_ended_by_a_signal_names_it` reaches the wording another
way. The program waits on stdin, the spec sends it SIGTERM, and both plays
must print "ended by signal 15". Under the mutation the second play printed an
empty line. Linux only, since it finds the program through /proc.

## 2026-10-13 — a list or map too long for its line spans lines

Built under #1882 (gavel: a bracketed list or map may span lines). A literal
whose opening bracket ends its line continues one element to a line, each two
spaces deeper, and closes with its bracket alone at the indent of the line that
opened it. The lexer gathers the lines into the one statement they spell, so the
parser and every engine see the literal they saw before. A list element is
grouped the way a parenthesised one is, so `double 21` on its own line is one
element; a map element is one `key:value` entry. A list inside a list spans on
its own terms, and a comment line may sit between elements.

Canonical form decides which form is written. The lexer measures the literal
joined back onto one line, and a literal that fits in 80 columns is refused in
the spanning form with the width it would take. A line too wide that holds a
literal gets a second clause on the width error saying the literal can span.
The other refusals each name the rule: a closing bracket that shares its line
with an element, a map entry sharing a line, an element at the wrong indent, a
blank line inside the literal, and a literal the file ends inside.

Five error goldens pin those diagnostics and
`tests/golden/micro/lists_and_maps_span_lines` pins a list, a map, a nested
list and a list in a function body on both engines. On main's lexer every one
of them is refused with a different message, and the micro sample stops at an
indent error.

Chapter 02 shows the form beside lists and maps, and appendix C states the
rule beside the continuation line. trend_gate's `higher` table and
perf_record's two watched lists, which were assembled from halves joined by
`text/concat` to stay under the width, are now single spanning lists with
their comments inside.

CI's rows, on main after #1925: compile_instructions lands on 26,216,275
(+334,724), entry_instructions on 86,540,771 (+1,159,458), library_instructions
on 87,096,300 (+1,160,718) and browser_compile_instructions on 392,203,148
(+891,646). interp_instructions falls to 488,437,711 (-302,105) and
emit_instructions to 7,012,968 (-509).

Moving `higher` onto the spanning form blinded
tests/a_mutation_keeps_its_unclassified_counter.rs, which read the direction
tables one line at a time. With `thunk_forces` added to `higher` it stayed
green. It now reads the spanning table and asserts that both tables parsed, and
the same edit turns it red.

- **OPEN** #1882, until this lands.

91.11.

With main merged after #1937, browser_compile_instructions lands on
394,301,683.

On CI's rows for the tree merged with main after #1937: compile_instructions
lands on 26,183,028, entry_instructions on 86,449,300, library_instructions on
86,666,390, emit_instructions on 7,048,150 and compile_allocs on 14,644.

393,186,348.

On CI's rows for the tree merged with main after #1937: compile_instructions
lands on 26,064,732, entry_instructions on 86,313,974, library_instructions on
86,530,678 and interp_instructions on 487,131,185. emit_instructions falls to
7,046,962.

393,279,063.

## 2026-10-12 — a field holds a list of `tie` references read from data

#1842, under the 2026-10-11 gavel. A constructor field written inside a
`list/tie` maker may now hold `list/to_list (list/map xs ref)`, or the same
with a one-parameter lambda in place of `ref`. `list/to_list` calls `ref` for
every element while the maker runs, so the field holds the cells `list/tie`
resolves. A bare `list/map` of references in that position is refused with a
diagnostic that names `list/to_list`.

The check sees the form in two shapes. In the program's own code it is the
call as written. In a module's code it has been through fusion by the time an
importer's merged check reads it, and `list/to_list (list/map xs f)` reads as
`fold xs [] (acc x -> (a y -> push a y) acc (f x))`. `src/check/ties.rs`
admits both, and a lambda applied where a field is written, which is how
fusion leaves `(k -> ref k) x`.

The fused shape found a second gap. `push` forced the item it was handed, and
a `list/tie` cell still being made is a blackhole, so the push demanded the
value it was in the middle of making and stopped with "a lazy binding demands
its own value". A list literal already kept such a cell unforced. `push` now
does the same with its item, in the interpreter (`src/eval.rs`) and in native
code when the program ties (`force_unless_knot` in `src/codegen.rs`).

`a_graph_tied_from_a_map_of_edges` reads each town's roads from a map of edges
and drops its chain of `road` records, with the same output on both engines.
`a_tie_field_holds_a_lazy_map_of_references` is the refusal. The comment
where `lib/list/list.kso` teaches `tie` shows the data-sized form. Three
ratchet rows take away the check's admission, the interpreter's keep and the
native keep, and each turns the micro corpus red.

The rows that moved are CI's, on the tree merged with #1899. The check reads
every field written inside a maker, and the library comment and the new
fixtures are more text to compile: compile_instructions lands on 25,764,487,
entry_instructions on 85,217,023, library_instructions on 85,769,178,
emit_instructions on 7,016,791 and browser_compile_instructions on
392,024,143. The interpreter asks whether each item `push` is handed is a cell
still being made, and interp_instructions lands on 489,120,716. This builds a
ruled part of the language, so the welfare floor drops by what it costs.

On the tree merged with #1913 the same costs sit on main's new rows:
compile_instructions lands on 25,884,823, entry_instructions on 85,390,774,
library_instructions on 85,944,538, emit_instructions on 7,013,859,
browser_compile_instructions on 391,328,276 and interp_instructions on
489,068,417.

On main after #1904 the tab's compile reads `browser_compile_instructions`
392,396,387, up 18,012 from main's 392,378,375 on the same tree.

91.11.

With main merged after #1922, browser_compile_instructions lands on
393,286,002.

## 2026-10-13 — `merge` copies a record with named fields replaced

Built under #1886 (gavel: `merge` copies a record with named fields replaced).
`merge u { "role":"chef" }` answers a copy of `u` with the named fields
replaced and every other field carried over.

**How it runs.** After the checks, each `merge` becomes a call to a group the
compiler writes for the set of names it replaces, with one arm per record the
calling module declares that has every one of those fields:
`Merge_4role (user Kept0 Kept1 _) Given0 = user Kept0 Kept1 Given0`. Every
engine already runs a dispatch and a construction, so none of them learned
anything new, and the tab compiles the same group. A field read is built the
same way, through the getter groups. Two things surfaced on the way. A group
written for a library carried only its first arm into the program that
imports it, because its arms shared the call site's span; each arm now
carries its own type's span. And a value whose type has no arm misses the
group at run time, so the miss is reported as a miss of `merge` rather than
of the group's internal name.

**What the checker refuses.** The fields are named by plain string literals
in a map literal; a computed map is refused, since the copy is built from the
names. Where the value's type is known at the merge (a construction written
there, or a local bound to one), a name that type does not declare is refused
as `` `user` has no field `rank` ``, the field read's wording. Otherwise a name
no record declares is refused, and so is a set of names no record of this
module has all of. A set that only an imported record has is refused as an
opacity error, the construction rule's (2026-08-20). Five goldens pin these:
four error fixtures, one of them a module tree, and a runtime fixture for the
miss on both engines. `a_merge_carries_the_fields_it_does_not_name` pins the
copy as a library on both engines. On main every one of them stops at
`` unknown name `merge` ``.

**What the ruling named that cannot be built yet.** The owed list asks for
an error golden for "a value that a declared field typeset does not admit".
A field carries no declared type: `check_field_annotations` refuses every
field annotation ("a record field carries no type"), so there is no typeset
for a `merge` to check against. The item waits on fields carrying types.

`merge` is ambient now, which takes the name from every program;
`lib/list`'s private sort helper was `merge` and is `merge_runs`. kq, vse and
kanso-json declare no `merge`; two ports on `claude/ports-corpus` do and will
rename when they are next built. Chapter 02 shows `merge` beside
destructuring, and appendix B gives it an entry under records.

The compile rows move with the binary and the renamed helper; CI's rows are
priced here before merge.

- **OPEN** #1886, until this lands.

On main after #1904 the tab's compile reads `browser_compile_instructions`
396,842,603, up 4,464,228 from main's 392,378,375, and
`browser_compile_peak_bytes` 675,735, up 10, on the same tree.

91.11.

With main merged after #1922, browser_compile_instructions lands on 398,041,240
and browser_compile_peak_bytes on 675,735.

## 2026-10-12 — a test file is an outside consumer of its module

Built from the 2026-10-12 gavel on #1883. A test file, one named `*_test.kso`,
now imports the module it sits beside and sees its pub names only. The
module's loader leaves test files out, so `kanso run`, `check` and `build`
never read one, and nothing a test file declares or imports reaches the
module.

`kanso test` on a directory compiles each test file on its own and runs them
in name order, with one summary line. On a file it compiles that file.
`module_beside` decides what the file imports. A directory holding a library
file is a module, and the test file imports it under the directory's name, so
`price` and `teahouse/price` both reach the pub function. A directory holding
a program, a file that parses only as an entry or a play file, is a folder of
programs; the book's sample chapters are this kind. A test file there imports
nothing and tests what it declares, as `greet_test.kso` in chapter 01 does.

Three fixtures in `tests/golden/test_verb` pin the ruling, each watched red
on the shared-namespace runner:

- `peeks` reads a private function and is refused with `unknown name`. The
  old runner passed both of its tests.
- In `shared_name`, the test's helper `label` and a private `label` in the
  module each stay in their own file. The old runner refused the pair as
  overlapping overloads.
- In `runs_beside`, a test file that does not parse leaves the program beside
  it running on both engines. The old runner's `run` stopped on the syntax
  error.

Four suites read private names and moved onto the pub surface:

- teahouse's menu test.
- expect's three tests that inspect a mismatch. `be_true`, `mismatch` and
  `not_true` are pub now. The header already documented them as what a
  reader sees, and building them from outside is still refused.
- list's six bisect tests.
- kq's `kq_test.kso`, which needs `parse_path`, `pretty` and `walk` pub (kq,
  same branch name).

The bisect tests built a `list/sorted`, which only `list` may build, and no
pub function returns one. Since the 2026-09-30 opacity ruling, nothing
outside `list` has been able to reach the bisecting arm of `find`. The tests
now test `find` on a plain list, and #1903 asks Clay how a caller should get
a `sorted`.

The json suite turned up an interpreter bug once it imported its module. A
bare `json_null` from an importer printed and compared as the importer's own
type, while native code resolved it to `json/json_null`. That fix is #1904,
with a micro golden, and this branch carries it.

Rows from CI, on the tree merged with #1913. The loader drops test files and
the compiler grew `module_beside`, so the front-end rows rise:
compile_instructions lands on 25,882,462, up 911; entry_instructions on
85,381,376, up 63; library_instructions on 85,935,657, up 75; and
startup_instructions on 52,405, up nine. interp_instructions lands on
488,739,567, 249 fewer, which is #1904's fix. browser_compile_instructions
lands on 391,170,536, 140,966 fewer; the tab compiles no test files either,
though no run isolated how much of the fall that is. The floor drops by what
the gavel costs.

On main after #1904 the tab's compile reads `browser_compile_instructions`
392,378,710, up 335 from main's 392,378,375 on the same tree.

91.11.

With main merged after #1922, browser_compile_instructions lands on
393,361,582.

## 2026-10-12 — a number edit is not a page edit

`page_drift` counts log entries since docs/compiler.html last moved, and took
the page's newest commit as the move. `golden_prose --write` commits the page
whenever a golden it quotes moves, rewriting a span's number and no sentence,
so a row change paid the page's debt. On main the operator arms' compile and
browser rows (470ae17f) did that, and the gate read 3/3 where the page was
four entries behind its last prose edit (2cac608c). #1913's span rewrite
would have reset it to zero while #1905 still owed sections.

The gate now walks the page's commits from the newest and stops at the first
whose removed and added lines differ once digits, commas and points are taken
out. Diffs are read against the first parent, so a merge counts only for what
it brought. `tests/a_number_edit_is_not_a_page_edit.rs` commits a span-only
rewrite after four entries and expects the four to stand, and a prose edit
after them to clear the count. With the comparison forced false the first
reads 0/3 and fails.

Main reads 4/3 under the fixed gate, so every pull request is red on
`page_drift` until the page carries the sections #1905 asks for.

91.11.

The first version walked every commit that touched the page in date order.
Merged with main after #1922, it reached the commit on the page branch where
the sections were written, which is older than eight entries main logged while
that branch waited, and read the page eight entries behind. The walk now
follows first parents, so the merge that brought the sections in is where the
page moved. A third spec builds that history: sections on a side branch, four
entries on the trunk, then the merge, and expects 0/3. It read 4 entries ahead
before the change.

## 2026-10-12 — a name bound to a raising call carries its err into the check

#1884, under part 3 of the 2026-09-15 gavel. The checker refused
`half 3 + 1` when `half` has an arm that answers an err, but accepted
`h = half 3` followed by `h + 1`: it read calls and never the names they were
bound to, so binding a value to a name switched the check off. Eighteen of the
twenty ports in the committee report met it. A local or a constant bound to a
call that can raise now carries that err to every operator, index and
argument position that reads it, and an inlined binding and an extracted one
compile alike.

`raising_names` in `src/check/after_infer.rs` collects the names a
declaration binds to such a call and the place each is bound; a name bound
twice in one declaration is left out, since which binding a read sees is not
settled by position alone. A constant is read through the arity-0 entries of
the raisers table. Errs from builtins, such as `text/to_int` on text the
program computes, are not raised errs, and they still ride to the endpoint.

Five error goldens pin it: a name reaching an operator, a name reaching a
group with no arm for it, a constant reaching an operator, a name bound in a
branch of an `if`, and a name bound past a `return` inside a branch. The
ratchet row "a name that forgets the err of its call" removes both halves and
watches error_corpus go red.

The first version collected bindings from a body's statements and from a
guard's rest, and never from a branch. A branch of an `if` is a block with
statements of its own, so `each = share cents people` followed by `each + 1`
inside a branch compiled. The collector now walks a branch's block as it walks
a body. Before collecting, the check asks whether any binding in the
declaration is handed a raising call at all, and most declarations answer no;
that look walks the same statements, branches and guards included. A look
that stopped at the top level cost 136,432 fewer instructions on the compile
corpus and let both branch fixtures through, which is how each fixture was
seen red.

The rule reached 26 programs in the tree, about 150 sites, and every one was
already refused on main once its binding was inlined. Each was repaired with a
pass-through `e@(err _)` arm, a `.>` step, or a helper with an err arm:

- `lib/json/text.kso` combines the four `\u` digits through `hex_on`, which
  hands the first bad digit on, nested two calls deep with nothing bound to
  a name. A self-recursive version counted one loop beat per digit, and kq's
  escapes run read 52,325 beats against 2; binding the first pair to a name
  made each one a thunk, 17,441 of them. The nested form moves no counter in
  kq. The decoder's emitted code grew 4 functions and 118 lines (10,070 to
  10,188), and `bench/emitted_golden.txt` and its sibling are regenerated.
- runbench routes the decoded document and its two raising totals through
  `.>`. It allocates 4 more blocks, 160 bytes, and no peak moved. A version
  with helper arms instead held the document as a thunk and raised the arena
  peak by 1 MiB, so it was dropped.
- oneshot takes the decoded document in a helper with an err arm, which
  costs no closure.
- interp_corpus binds the decoded document and sizes it through `.>`
  before it recurses. The first repair passed the document to a helper with
  an err arm, and that call was a tail call from `total`: the frame's region
  rewinds before a tail call, so the document was deep-copied out of it on
  every round. The native run of the corpus went from 16,005,829
  instructions to 19,483,131, and the browser's run row from 34,124,990 to
  46,702,454. Recursing from inside a `.>` step costs no copy, but the
  interpreter holds every round's document until the recursion returns, and
  its peak rose from 687,159 to 1,043,112 bytes on CI. The spelling kept
  sizes the document before the tail call. On CI it reads 34,172,021
  browser run instructions, 488,636,161 interpreted instructions and
  701,975 interpreter bytes, at the price of one more 256 KiB block of
  browser peak (1,179,648 to 1,441,792). The corpus's text is longer, so
  compiling it in the browser costs 402,835,440 instructions where it cost
  401,459,207, and its peak is 683,140 bytes where it was 681,745. Scored
  against the other two it gives up the least welfare.
- Thirteen scripts under `scripts/` were repaired and each was run against
  the same input with the old and the new compiler; where nothing failed the
  output was byte for byte the same.
- In the book, ch04's railway is built on an err the checker cannot prove, a
  `text/to_int` of text out of a list, and the rule paragraph says that a
  name bound to a raising call is checked like the call. `unasked` and
  `unhandled` take their err the same way, and teahouse (ch07) and the json
  sample (ch08) reach their raising values through `.>`.

On CI's rows for the tree merged with #1913, the check's new work lands
compile_instructions on 26,400,119, entry_instructions on 86,385,190 and
library_instructions on 86,983,929, rises of 518,568, 1,003,877 and 1,048,347.
compile_allocs lands on 14,688, up 90, and interp_allocs on 587,652, up 301.
browser_compile_instructions lands on 402,115,910, up 10,804,408, most of it
the longer corpus text described above; its peak is 683,140 bytes. The
interpreted corpus fell 156,512 instructions to 488,583,304. This builds the
ruling on #1884, so the floor drops by what it costs.

An explicit `e@(err _)` arm hands the err on without adding a frame to the
"passed through" trace; only the implicit pass-through wrote those frames. The
trace of an err the checker can prove is now shorter by every function that
needed an arm. `examples/trace_demo`'s broken-input run, which no test pins,
now reads `passed through text/to_int` where it read three functions.

The three provenance fixtures behind `tests/a_hop_is_recorded_for_a_name.rs`
handed `label` an err that `price` raised, and the rule now refuses that
before it runs. They hand it the err `text/to_int` answers for a slice of
text instead, which nothing raises, and the hops they record are unchanged.
The playground's redux example bound an `update` that can overdraw to a name
and passed it on; it now dispatches on the answer the way
`examples/redux.kso` does. `json/unicode_at` joins the decoder's string
scanners in `src/beat.rs`'s list of the library's beats.

The rows below are CI's, on the tree merged with #1899, and each worsened.
The front end does the new check and compiles the repaired library:
compile_instructions lands on 26,280,991, entry_instructions on 86,208,525,
library_instructions on 86,806,845, compile_allocs on 14,620,
compile_peak_bytes on 681,983, front_end_rounds on 19 and front_end_visits on
8,112. The interpreter and the browser run the corpus's new spelling:
interp_allocs lands on 587,584, interp_peak_bytes on 701,975,
browser_compile_instructions on 402,835,440, browser_compile_peak_bytes on
683,140, browser_run_instructions on 34,172,021 and browser_run_peak_bytes on
1,441,792. The native binaries' code, `text`, lands on 4,854,398.

The counters this moves, on this container: in `bench/cost_golden_run.txt`,
`run_allocs` to 1,771,566 and `run_alloc_bytes` to 330,057,564. In
`bench/emitted_golden.txt`, `emitted_calls` to 1,023, `emitted_defines` to
110 and `emitted_lines` to 10,188. In its sibling, `emitted_other_calls` to
17,603, `emitted_other_defines` to 2,037 and `emitted_other_lines` to
147,804. The instruction and browser rows come from CI.

On the tree merged with #1900, CI's work rows fell: `work_runbench` to
1,167,982,009, `work_jsonbench` to 858,183,234, `work_encodebench` to
2,398,361,941, `work_livebench` to 1,474,853,886 and `work_oneshot` to
13,206,113. `text` rose for runbench (626,473), jsonbench (335,721), oneshot
(350,793) and livebench (353,657) and fell for encodebench (342,089) and
widebench (350,249). The browser compile's rise outweighs the run's fall, and
this builds a ruled part of the language, so the welfare floor drops by what
it costs.

On main after #1925 the emitted counts for the other programs land on
emitted_other_calls 17,692, emitted_other_defines 2,049 and emitted_other_lines
148,325, and the `text` total on 4,845,150.

91.11.

On main after #1916 the tab reads `browser_compile_instructions` 403,992,263,
up 10,809,633 from main's 393,182,630, and `browser_run_instructions`
34,172,363, up 47,031 from 34,125,332, on the same tree.

On the tree merged with main, the tab's compile reads 404,068,547 instructions
against main's 393,209,858, and its peak reads 684,676 bytes against 677,261.
The floor drops by what that costs, under the #1884 ruling.

CI's rows on the merged tree, against main: `compile_instructions` from
26,463,317 to 26,988,479, `entry_instructions` from 88,129,856 to 89,160,295,
`library_instructions` from 87,707,726 to 88,767,419, `compile_allocs` from
15,358 to 15,454, `interp_instructions` from 486,939,316 to 488,604,425,
`interp_allocs` from 588,491 to 588,801 and `interp_peak_bytes` from 692,375 to
721,711. The emitted code for the other benchmarks grew: `emitted_other_calls`
from 17,664 to 17,760, `emitted_other_defines` from 2,036 to 2,063 and
`emitted_other_lines` from 148,239 to 148,764, and `text` from 4,838,990 to
4,858,430 bytes. The native runs fell: runbench by 2,052,628 instructions and
jsonbench by 2,379,750. The floor drops by the difference under the #1884
ruling.

Two goldens the merge with main left stale now read this tree:
`front_end_visits` on compile_corpus from 7,990 to 8,120 and
`compile_peak_bytes` from 694,044 to 701,567, and in the run program
`run_allocs` from 1,812,274 to 1,812,278 and `run_alloc_bytes` from 303,607,404
to 303,607,564.

## 2026-10-13 — work-ahead forces what follows a park, and holds a pooled failure

Builds #1856's v1 under the 2026-07-22 ruling and the 2026-10-11 gavel. Native
only; the interpreter is the oracle and does not pool.

**What is lifted.** `src/ahead.rs` reads the entry's statements. One whose
chain starts with a `sleep` has continuations that run, in order, once the
sleep ends. In each continuation, a call to one of the program's own functions
whose arguments name nothing local is certain to be evaluated when it is an
argument of `print` or `write`, or a piece of a string such an argument
interpolates. A continuation that names an entry binding is followed into that
binding's chain. Each such call becomes a constant named `ahead~N`, which no
author can spell, and the call is replaced by a read of it.

**What runs it.** A constant builds its own cell on first read, so forcing one
early fills the cell the continuation reads later. The compiler emits
`k_ahead_pool`, which forces the lifted constants in the order they were made;
the runtime runs it once, at the first bare sleep, and takes the time the pool
spent off the wait. Which work runs is fixed by the compiler, and the clock
decides only how long the process then sleeps. `bench/workahead/aswritten.kso`
sleeps 2,000 ms and then grinds for about 830 ms; it took 2.83 s on main and
2.01 s here, three runs each in this container, with the same output.

**A pooled failure.** An err the work answers is a value, held in the cell and
answered at the read like any other. A death is not a value: on main the
runtime prints and exits, and a pooled death would end the process before
output the program prints first. While the pool runs, every exit in the runtime
and the stack-overflow handler jump back to `k_ahead_try` instead, with stderr
pointed at `/dev/null`. The try clears the ready flag of every constant cell
still holding its builder's blackhole, restores the beat depth, and stops the
pool. The program then builds those cells again where it reads them and fails
there, which for every program that terminates is the transcript it produces
without the pool.

`tests/work_ahead.rs` pins two transcripts against the interpreter's. In one,
the pooled call recurses a million frames deep and runs out of stack; in the
other, a module's pooled call demands a constant that names itself, which
leaves two cells half-built. With the catch removed, neither prints `first`.
With the catch kept and the cells left as they were, the overflow fixture
reports "a lazy binding demands its own value" where the program ran out of
stack.

**The counter.** `tests/golden/mem/work_ahead_forces_what_follows_a_park` pins
`ahead_forced=1` and `ahead_dropped=0`. Both lines print only once the pool has
run, so no vein whose program never sleeps moved. That fixture's
`beat_iters` reads 2 where main's compiler reads 1,002: inside the constant's
builder the beat pass gives the integer loop no bracket, since it allocates
nothing to rewind. A loop that allocates keeps its bracket; with a string built
per lap, 100,000 laps read the same counters on both compilers, one arena block
included.

**The tab** runs no pool. Its runtime is linked apart from the program and its
sleep is the host's, so the pool call is compiled out of the wasm runtime.

Divergence is the case the gavel accepted: a pooled call that never returns
stalls before output the program prints first. The page's work-ahead section
owes a sentence on each case, under the gavel.

**What it costs**, from CI's rows. Every program now defines `k_ahead_pool`,
empty when nothing was lifted, because the runtime calls it at a sleep: one
define and five lines per benchmark (`emitted_defines` 106 -> 107,
`emitted_lines` 10,070 -> 10,075; over the thirteen others
`emitted_other_defines` 2,010 -> 2,023 and `emitted_other_lines` 147,279 ->
147,344), and the module compile row's `module_defines` 29 -> 30 and
`module_lines` 1,284 -> 1,288. `k_ahead_try` and its jump buffer put 320 bytes in
each binary (`text` 4,834,846 -> 4,839,326 summed). The compiler's rows rose:
`emit_instructions` 7,013,477 -> 7,137,222 (+1.76%), `codegen_instructions_dev`
119,002,829 -> 119,332,985 (+0.28%), `codegen_instructions_release`
454,179,413 -> 455,947,986 (+0.39%), `browser_compile_instructions`
391,311,502 -> 393,313,454 (+0.51%), `compile_instructions` 25,881,551 ->
25,887,870, `entry_instructions` 85,381,313 -> 85,406,213 and
`library_instructions` 85,935,582 -> 85,959,939. The run rows fell by 50 to
1,178 instructions each (`runbench` 1,170,035,080 -> 1,170,033,902) and
`interp_instructions` by 13,571 to 488,726,245; no benchmark sleeps, so these
are the binary's layout. Work-ahead is a ruled part of the language, so the
floor moves by what it costs.

- **OPEN** #1856, until this lands.

Each of the four compile-cost programs gains the pool's define and its four
lines, so `bench/compile_golden.txt` moves from 38 defines to 42 and from 1,340
lines to 1,356.

On main after #1904 the tab's compile reads `browser_compile_instructions`
393,668,581, up 1,290,206 from main's 392,378,375 on the same tree.

91.11.

With main merged after #1918, browser_compile_instructions lands on
393,374,069.

CI's rows against main: `compile_instructions` from 26,463,317 to 26,477,664,
`entry_instructions` from 88,129,856 to 88,171,192, `library_instructions` from
87,707,726 to 87,748,995, `emit_instructions` from 7,052,040 to 7,168,640,
`codegen_instructions_dev` from 119,002,656 to 119,334,030 and
`codegen_instructions_release` from 454,179,390 to 455,948,524. Every native
binary carries the pool's start and its forcing, so `text` grew from 4,838,990
to 4,843,470 bytes, `emitted_other_defines` from 2,036 to 2,049 and
`emitted_other_lines` from 148,239 to 148,304. `work_encodebench` landed at
2,398,406,493, up 559, and most other benchmarks fell by a few hundred
instructions. The floor drops by the difference under the 2026-10-11 ruling
(#1862).

## 2026-10-12 — arms travel with the value

The first rule of the 2026-10-12 gavel on #1894. A value carries the arms its
type's owner wrote, so a module handed a `circle` may call `area c` without
importing geo.

The check decides which calls travel. The resolver marks a local as arrived
when the function was handed it: a parameter, a name bound to a field or
element of one, a lambda's parameter when the lambda goes to a call that also
carries an arrived value, and the answer of a traveling call on one. A bare
call that names nothing the file can see, with an arrived argument, is
recorded instead of refused. The answer of a local function is not arrived,
as the gavel lists it, so `area (twice c)` stays an unknown name.

Each recorded call is renamed into a traveling group, `~area`, a spelling no
program can write. A dependency compiles with the call left there. The root
of the program, an entry file or a directory with `main.kso`, fills each group
it calls with clones of every pub arm of that name whose module owns one of
its parameter types, which is rule 4's ownership condition. When no owner
wrote the name, the call is refused where it was written: "no module in this
program writes `perimeter` for a type it owns, so no value carries it". Checked
alone, a library takes the call on a parameter and refuses it on a value it
made. Native code quotes a symbol with `~` in it, as it does one with `/`.

`tests/arms_travel.rs` runs `tests/golden/travel/area` on both engines: a
report module that never imports geo calls `area` on a circle it was handed
and on each circle of a list. Main refused both calls, from `kanso run` and
from `kanso check report`. Two error goldens pin the refusals:
`an_arm_called_on_a_value_the_file_made` and
`a_traveling_call_no_owner_answers`. The ratchet row "an arm left behind by
its value" lets no call travel and turns the run test red. ch11 states the
rule and ch07 points to it.

Still open on #1894: a local arm of the same name shadows the traveling ones
outright, where rule 4 asks for a local arm of the same shape to shadow and
the rest to stay reachable. A module that declares its own `area` for a
square and is handed a circle cannot reach geo's arm yet.

CI's rows put `compile_instructions` at 26,052,299 (+170,748, +0.66%),
`entry_instructions` at 86,286,307 (+904,994, +1.06%), `library_instructions`
at 86,505,153 (+569,571, +0.66%) and `emit_instructions` at 7,019,534
(+6,057). `compile_allocs` rose 75 to 14,605 and `interp_allocs` 74 to 587,357.
`interp_instructions` fell 402 to 488,739,414. The tab runs the same front end,
and `browser_compile_instructions` rose to 393,182,630 (+804,255, +0.20% on main after #1927).
This builds a ruled part of the language, so the welfare floor drops by what
it costs.

## 2026-10-12 — a local arm shadows only its own shape

The fourth rule of the 2026-10-12 gavel on #1894. Rule 1 left a gap: a module
that wrote `area` for its own square resolved every `area` call to its own
arms, so a circle it was handed found no arm and stopped at run time with "no
overload of `report/area` matches these arguments".

The resolver now also records an arrived call of a function the module itself
declares. The call keeps its name and resolves where it stands; the declaration
it sits in keeps a list of such calls, each by its span and the name it wrote.
When the program is whole, enrollment looks at each listed call whose name some
owner writes. If an owner's arm has a shape none of the module's arms has, the
call goes to a group of its own, `~report~area`, holding the module's arms of
that name and the owners' arms of other shapes. A shape is the type each
parameter matches, read through a bare clone to the type it copies. Every other
listed call stays on the module's own group, which is where most of them
belong: in the entry corpus 716 calls are listed and 77 name a function an
owner writes, all three of them (`fold`, `iter`, `next`) the library's own.
The wrapper rewrite leaves a listed call alone until enrollment has decided,
because the group it may join is not the wrapper's. A compile that never
enrolls, the REPL's, a library's alone and a module's without an entry, drops
the lists.

A module the program imports is checked before the program fills the groups.
Its listed calls are still on the module's own group when it is checked, so a
function only those calls reach is called; `tests/golden/travel/told` is the
postcard, a module whose `told?` only a call on a parameter reaches. The first
build renamed every such call into a traveling group and back, which cost the
entry check 7.8M instructions; leaving the names alone costs 1.74M there and
0.37M on the compile corpus, measured here against rule 1's branch.

Two fixes came with it. An owner reached only through another module read as
private, because a dependency's `pub` is demoted at each module boundary, so
its arms never traveled. A declaration now keeps `written_pub`, what its own
source said, and owners are judged on that. And a dispatch diagnostic on a
traveling group named `~combine`; `ast::spoken` now gives the name the call
wrote.

`tests/golden/travel/shade` hands a module that writes `area` for a square
both a circle and a square, and both engines print `12` and `25`.
`two_owners_tie_on_a_traveling_call` in the module error corpus is rule 4's
refusal: two owners' arms each more specific in a different position, one of
them reached through a module that imports it. The ratchet row "a local arm
that shadows every shape" keeps such calls off the traveling path and turns
the run test red. ch11 states the rule.

CI's rows, against #1916's: entry_instructions lands on 87,981,735, up
1,695,428, and library_instructions on 87,561,579, up 1,056,426, the listed
calls the resolver records and enrollment reads; compile_instructions lands
on 26,419,302, up 367,003. compile_allocs lands on 15,358, up 760, one list per
declaration that shades a call, and front_end_visits on compile_corpus
on 7,990, up eight. The interpreter's run of the corpus allocates more:
interp_allocs lands on 588,363, up 1,012, and interp_peak_bytes on 692,375, up
5,216, because a module's declarations carry their lists into the run.
emit_instructions lands on 7,028,480, up 8,946. interp_instructions falls
434,783 to 488,304,631. This builds rule 4 of the 2026-10-12 gavel, so the
floor drops by what it costs.

The front end holds 19,360 more bytes while it checks the compile corpus:
compile_peak_bytes lands on 694,044, up from 674,684. CI's cost job and this
container read the same number under rustc 1.98.1, the host the golden's
measured-on line names.

Merged with #1916's branch after #1904, the tab's compile lands on
`browser_compile_instructions` 393,209,858 and `browser_compile_peak_bytes`
677,261: 27,228 instructions and 1,536 bytes above #1916's reading on the same
main, for the shape check a local arm now makes.

On CI's rows for the tree merged with main after #1904: `compile_instructions`
25,803,156 -> 26,057,907, `entry_instructions` 85,149,258 -> 86,302,801,
`library_instructions` 85,703,128 -> 86,519,482, `emit_instructions` 7,035,926
-> 7,048,046 and `interp_allocs` 587,479 -> 587,485; `interp_instructions`
falls 298,363 to 487,131,077.

On CI's rows for the tree merged with main after #1937: compile_instructions
lands on 26,463,317, entry_instructions on 88,129,856, library_instructions on
87,707,726, emit_instructions on 7,052,040 and interp_allocs on 588,491.
interp_instructions falls to 486,939,316.

## 2026-10-12 — a bare imported type with no fields is its module's value

An importer may name a type with no fields bare: `import "std/json"` enrols
`json_null` beside `json/json_null`. Native code resolved the bare name to the
type it stands for. The interpreter built a record under the bare spelling, so
it printed as the importer's own type and compared unequal to `json/json_null`.
A constructor call already took the type's origin; the fieldless path in
`resolve` now does the same.

`tests/golden/micro/a_bare_imported_unit_is_its_module_s` prints the bare value
and compares it with the qualified one, inside a list and out. Before the fix
the interpreter printed `a_bare_imported_unit_is_its_module_s/json_null` and
`false` twice where native printed `json/json_null` and `true`.

json's own test suite found it. Under #1883 a test file imports its module,
and `test_decode_nested` compares a decoded document against a map holding a
bare `json_null`.

From CI, the interpreter's run of the corpus reads 249 instructions fewer,
interp_instructions 488,739,816 to 488,739,567 on the merged tree. No other
row moved, and welfare banks the rise.

## 2026-10-13 — three port shapes join the run program before their fixes

Three of #1892's items are fixed on open branches, and the run program was
blind to all three, so under the 2026-09-05 corpus-first gavel each shape goes
in first, measured on main, and the fix scores as a rise when it lands.

- `rewind`, 20,000 laps: bc's `for` statement, a tail cycle entered by a tail
  call from `exec`, which each lap calls. Main gives the cycle no beat, so the
  laps' records stay. kanso#1920 fixes it.
- `stack`, 115 chunks of 25 ints concatenated onto one list in a fold: the kv
  port's accumulator. Main keeps every earlier list until the fold returns.
  kanso#1924 fixes it.
- `bag`, 4,300 pushes onto a list held in a record field: the accumulator the
  lox, markdown, datalog and sat ports wrote. Main copies the list on every
  push. The record-field branch fixes it.

Built alone on main, each less the empty program's 6,766,088, `rewind` costs
4,020,895 instructions (0.33% of the whole), `stack` 2,621,918 (0.21%) and
`bag` 58,073,973 (4.70%). `rewind` and `stack` are sized by memory like
`machine` and `fold`: each adds 2 MiB to the peak alone. `bag` is quadratic and
sized near five per cent like `scatter` and `bump`; it adds nothing to the peak.
The run program's header carries the table.

Together they take `work_runbench` 1,170,035,080 -> 1,234,827,314 on CI (the
container projected 1,234,663,911) and the run peak
4,948,512 -> 10,191,392 bytes. `run_instructions` and `run_peak_bytes` are
re-based by the same ratios, so welfare scores what it scored, and the floor's
history says so; `run_arena_peak_bytes` is the part of the peak that moved,
4,718,608 -> 9,961,488.

The rest of the run vein moves with the work the three phases add, and each
phase built alone says which: `run_allocs` rises to 1,816,986, 40,000 of it
`rewind`'s records. `run_alloc_bytes` rises to 388,718,860, `run_evac_allocs`
to 50,009, `run_evac_bytes` to 28,709,472, `run_sh_buf` to 175,346,960 and
`run_sh_rec` to 3,791,936, nearly all of it `bag`'s copies. `run_beat_iters`
rises by `bag`'s 4,300 laps to 52,008. `run_arena_blocks` goes 4 -> 9 and
`run_spare_peak_bytes` to 7,864,336, which `rewind` and `stack` each raise
alone. `run_perm_allocs` rises by one to 99. `run_chain_finds` goes 0 ->
12,762 although no phase alone finds a chain; only the whole program does, and
nothing here isolated why.
The program's code grows with it: across the other benchmarks' emitted rows,
`emitted_other_defines` lands on 2,036, `emitted_other_calls` on 17,664,
`emitted_other_branches` on 16,555 and `emitted_other_lines` on 148,239, all
of it runbench's (51,577 -> 52,537 lines), and `text` on 4,838,622, runbench
619,081 -> 622,857 bytes.

## 2026-10-13 — a list held in a record grows where it stands

From #1892, the eleventh item: the lox, markdown, datalog and sat ports each
kept an accumulator in a record field and pushed onto it through the record,
`bag (b.count + 1) (push b.items i)`. The linearity analysis proved a list
unique only when a variable held it, so `push b.items i` copied the list while
the record still held it, and n pushes cost the square of n: 40,000 of them
allocated 37 GB, and the native run took 4.4 seconds where it now takes 0.06.

`src/linear.rs` now proves a field unique as well. A greatest fixpoint runs
over (function, arity, parameter, field), seeded with the fields some push,
append, put or concat writes into. A field stays in the set while every caller
hands that parameter over with the field alone: a constructor argument that is
itself unique, an `if` or guard whose arms all are, a parameter already in the
set, or a call to a function that returns the field alone. A field read moves
the list out of the record only when the record is handed over, every mention
of the parameter is a field read, the field is read exactly once, and no
mention sits under a lambda. Anything else keeps the copy.

`tests/golden/mem/a_list_held_in_a_record_grows_where_it_stands` runs 20,000
pushes through a record and four cases the analysis must refuse: a list two
records share, a field read twice, a record filled twice from one start, and a
record a later line reads again. It counts 73 allocations and 1,994,928 bytes
where main counts 53,881 and 9,160,278,336. The ratchet row "a record field
copied on every push" turns it red when the move is refused.

On the run program's `bag` phase, which #1925 added on main: `work_runbench`
1,234,827,314 -> 1,177,927,314 on CI, 56,900,000 fewer, the phase alone
58,073,973 -> 911,987 instructions in this container. The run vein
lands on `run_allocs` 1,812,272, `run_alloc_bytes` 335,416,908,
`run_evac_allocs` 49,188, `run_evac_bytes` 14,985,856, `run_sh_rec`
3,778,880, `run_sh_buf` 121,778,720, `run_survive_slots` 4,856,
`run_chain_finds` 11,948, `run_bytes_malloc` 9,138 and `run_bytes_freed`
9,007. Each push now writes in place onto a header born in an earlier lap, so
`run_push_mut_slow` counts it: 1,554,416, `bag`'s 4,300 more.

The list now outlives each lap's rewind, so it lives outside the arena:
`run_perm_peak_bytes` and `run_perm_live_bytes` land on 277,960, and the run
peak rises 10,191,392 -> 10,437,152 bytes. Most of that is the growth policy
rather than the list: 4,300 values fill 68,800 bytes, and a buffer that
doubles once more past sixteen slots holds 16,384 of them. A buffer outside
the arena already grows by `realloc`, so that second doubling buys it
nothing; dropping it there is a lead, not built here. `text` lands on
4,838,766, runbench 622,857 -> 623,001 bytes.

CI's compile rows. The analysis asks one more question of each field write,
and the interpreter skips the copy at the same sites:
`compile_instructions` 25,881,551 -> 25,803,156, `entry_instructions`
85,381,313 -> 85,149,258, `library_instructions` 85,935,582 -> 85,703,128,
`emit_instructions` 7,013,477 -> 7,035,926 (+22,449),
`interp_instructions` 488,739,816 -> 487,429,689, `interp_allocs` 587,351 ->
587,479 (+128), and `browser_compile_instructions` 391,311,502 ->
392,378,375 (+1,066,873, +0.27%).

## 2026-10-13 — a buffer outside the arena grows by one doubling

`k_b_push_grow` picks a list's next capacity as the smallest power of two that
holds it, doubled once more past sixteen slots. In the arena that second
doubling saves grows, because each grow there is a fresh buffer and a copy. An
accumulator's buffer outside the arena already grows by `realloc`, so the
second doubling only held memory: #1926's 4,300-element list sat in 16,384
slots. A buffer outside the arena now takes the smallest power of two.

On the run program `run_perm_peak_bytes` lands on 146,888 and
`run_perm_live_bytes` on 146,888 (277,960 before), and the run peak falls
10,437,152 -> 10,306,080 bytes. The extra regrows cost `work_runbench`
1,177,927,314 -> 1,177,965,832 on CI, 38,518 more, and `run_alloc_bytes` falls to 303,607,404 because the counter adds
each buffer's full size and the buffers are smaller. `run_allocs` lands on
1,812,274, `run_bytes_malloc` on 9,140 and `run_bytes_freed` on 9,009.
basket's permanent peak halves to 2,162,720 (`basket_perm_peak_bytes`, with
`basket_perm_live_bytes` the same), `basket_alloc_bytes` falls to 6,162,481,
and `basket_allocs` lands on 27,271, `basket_bytes_malloc` on 32 and
`basket_bytes_freed` on 13. escape regrows more often: `escape_allocs` 38 ->
63, `escape_alloc_bytes` 41,924,192, `escape_bytes_malloc` and
`escape_bytes_freed` 55. In the mem vein, five shapes' permanent peaks halve
and the record-field golden's falls 1,179,840 -> 655,552. Every benchmark's
machine code grows sixteen bytes, and `text` lands on 4,838,990.

The same regrows cost the mem vein's other accumulators more allocations (one
to sixteen), and of these only `early_exit` holds less for it:
`a_long_list_renders_in_one_pass_allocs` 2,018,
`a_long_list_renders_in_one_pass_alloc_bytes` 248,272,
`a_long_list_renders_in_one_pass_bytes_malloc` 5;
`a_loop_that_only_grows_its_list_takes_no_rewind_allocs` 42,
`a_loop_that_only_grows_its_list_takes_no_rewind_alloc_bytes` 8,373,392,
`a_loop_that_only_grows_its_list_takes_no_rewind_bytes_malloc` 36;
`an_accumulator_regrows_where_it_is_allocs` 83,
`an_accumulator_regrows_where_it_is_alloc_bytes` 577,072,
`an_accumulator_regrows_where_it_is_bytes_malloc` 60; `early_exit_allocs` 17,
`early_exit_bytes_malloc` 4; `fold_push_shape_allocs` 17,
`fold_push_shape_alloc_bytes` 214,992, `fold_push_shape_bytes_malloc` 5;
`fused_map_shape_allocs` 17, `fused_map_shape_alloc_bytes` 214,992,
`fused_map_shape_bytes_malloc` 5; `fused_reducer_allocs` 7,
`fused_reducer_alloc_bytes` 28,960, `fused_reducer_bytes_malloc` 3;
`fused_select_shape_allocs` 18, `fused_select_shape_alloc_bytes` 215,024,
`fused_select_shape_bytes_malloc` 5; `piped_reducer_allocs` 7,
`piped_reducer_alloc_bytes` 28,960, `piped_reducer_bytes_malloc` 3;
`skip_shape_allocs` 37, `skip_shape_alloc_bytes` 129,456,
`skip_shape_bytes_malloc` 5; `take_shape_allocs` 20, `take_shape_alloc_bytes`
215,152, `take_shape_bytes_malloc` 5; `tally_shape_allocs` 19,
`tally_shape_alloc_bytes` 133,280, `tally_shape_bytes_malloc` 5.

CI's instruction rows. The other benchmarks pay for the extra regrows too:
`work_basket` 30,876,488 -> 30,946,055 (+69,567), `work_encodebench`
2,398,373,226 -> 2,398,405,934 (+32,708), `work_escapebench` 39,049,158 ->
39,058,255 (+9,097), `work_pendbench` 190,054,350 -> 190,061,194 (+6,844),
`work_jsonbench` 860,562,984 -> 860,565,984 (+3,000), `work_digestbench`
5,603,118 -> 5,604,458 (+1,340), `work_oneshot` 13,226,930 -> 13,227,314
(+384) and `work_widebench` 28,178,509 -> 28,178,544 (+35); `work_livebench`
falls 477 to 1,474,874,594. In the tab, where the same runtime runs the
interpreted corpus, `browser_run_instructions` 34,124,990 -> 34,125,332
(+342). `codegen_instructions_dev` reads 119,002,829 -> 119,002,656 and
`codegen_instructions_release` 454,179,413 -> 454,179,390.

The record-field fixture #1926 added takes the same regrows:
`a_list_held_in_a_record_grows_where_it_stands_allocs` lands on 76 and
`a_list_held_in_a_record_grows_where_it_stands_bytes_malloc` on 19, three each
above #1926's reading, while its peak falls as above.

## 2026-10-13 — the ratchet's own checkout reads its library again

Two faults on main kept the ratchet from reading what it was built to read.
Neither came from the branches that showed them.

The first came with #1927. Its mutation `a_fifth_push_takes_sixteen` anchors
on the grow's extra doubling, and #1927 rewrote that line as
`if (cap > 16 && !perm) cap <<= 1;`. The mutation stopped applying, so the
whole-tree ratchet job reported it stale on every branch merged with main;
#1904 was the first to show it. The mutation now anchors on the new line and
doubles every time again, and the mem corpus goes red under it
(`a_cap_around_a_count_is_a_range` reads `allocs=14` against 13).

The second is older and shows only on a branch that edits
`scripts/perf_record`, because only then does a shard pick the perf-history
row. The ratchet links each scratch worktree's `target` to
`/tmp/kanso-ratchet-target`, so the compiler it builds lives outside the
checkout. `in_shipped_library` looked for the library beside the binary
(`current_exe()/../../lib`, here `/tmp/lib`) and nowhere else, so
`kanso check lib/json` refused `builtin_built_text` as internal to a library
it was not in. With no compile counters on stderr, perf_record's row lost
`compile_alloc_bytes` and `compile_passes`, and the gate was red before any
mutation. #1931 showed it, since it rewrites perf_record's lists in the
spanning form. The import loader already had the right fallback:
`resolve_import` tries `lib/` beside the working directory when the
toolchain's copy is absent. The check now tries the same root.
`a_checkout_library_checks_from_any_binary` copies the compiler to a
directory of its own and checks `lib/json` from the checkout; it read the
refusal before the fix and reads nothing after.

The tab's build leaves the new root out, since it has no checkout to stand
in. With the root compiled in, `browser_compile_instructions` read 392,449,729
against 392,378,375; without it the browser row does not move.

## 2026-10-13 — the ruled queue lands in one carrier

Sixteen pull requests were open at once, most of them building rulings: #1842,
#1856, #1882, #1883, #1884, #1886, #1887, #1891, #1892 and #1893. Branch
protection is strict and CI runs one job at a time, so each merge sent the
rest back through CI for twenty minutes apiece. kanso#1938 merges all of them
onto main in one tree. Each change's own entry above says what it did and what
it cost alone; this entry prices the combined tree as CI measured it on
ce1297c0, against main at 8710ac11.

Compile work rose with the new checks and lowering:

- `compile_instructions` from 26,463,317 to 28,012,335
- `entry_instructions` from 88,129,856 to 92,423,959
- `library_instructions` from 87,707,726 to 92,019,476
- `compile_allocs` from 15,358 to 15,984
- `front_end_visits` from 7,990 to 8,133, and `compile_peak_bytes` from
  694,044 to 703,170
- `startup_instructions` from 52,396 to 52,528
- `emit_instructions` from 7,052,040 to 7,240,982
- `codegen_instructions_dev` from 119,002,656 to 119,351,105
- `codegen_instructions_release` from 454,179,390 to 477,040,687
- `browser_compile_instructions` from 393,209,858 to 413,925,406
- `browser_compile_peak_bytes` from 677,261 to 686,192

The interpreter, the browser's run and the native runs:

- `interp_instructions` from 486,939,316 to 495,529,381
- `interp_allocs` from 588,491 to 589,689
- `interp_peak_bytes` from 692,375 to 722,393
- `browser_run_instructions` from 34,125,332 to 34,173,068
- `work_deepbench` from 393,579,644 to 403,499,282
- `work_digestbench` from 5,604,458 to 5,909,013
- `work_pendbench` from 190,061,194 to 191,673,790
- `work_runbench` from 1,177,965,832 to 1,178,202,157
- `work_widebench` from 28,178,544 to 28,274,435

`work_encodebench` fell from 2,398,405,934 to 2,386,636,505 and
`work_jsonbench` from 860,565,984 to 858,185,997; the other benchmarks moved by
less than 22,000 instructions each.

The decoder's emitted code has `emitted_defines` 111 where it had 106 and
`emitted_lines` 10,161 where it had 10,070. Across the other benchmarks
`emitted_other_defines` lands on 2,078 from 2,036 and `emitted_other_calls` on
17,798 from 17,664, while branches and lines fell. The binaries' `text` grew
from 4,838,990 to 4,932,142 bytes.

The largest single moves are release codegen, up 5.0%, and the browser's
compile, up 5.3%. Which carried change produced each is not isolated here; the
branches' own readings are in their entries.

The run program prints the same `runbench 46132896` on main and on this tree.
Its counters read `run_arena_peak_bytes` 4,718,608 against main's 9,961,488 and
`run_allocs` 1,812,043 against 1,812,274. Three run counters rose:
`run_beat_iters` from 52,008 to 92,008, `run_evac_allocs` from 49,188 to 49,221
and `run_push_mut_slow` from 1,554,416 to 1,554,428.

Welfare reads 91.3353 on CI's rows against main's floor of 91.2916, so the
combined tree is a rise, and the floor is banked there.

## 2026-10-13 — gavel: `merge` copies a record with named fields replaced

Clay ruled #1886. Sixteen of the twenty ports complained that changing one
field of a record meant rebuilding it by position, and three of their bugs
were fields passed in the wrong order at such a site. Construction by field
name stays declined (2026-07-29, named arguments); this is an update form
only.

**The ruling.** `merge u {"role": "chef"}` answers a copy of the record `u`
with the named fields replaced and every other field carried over. The map is
a literal whose keys name fields of `u`'s type. The checker refuses a key that
is not a field of that type. It also refuses a value that the field's
declared typeset does not admit, where the field declares one. `merge` builds a
record, so it is allowed only where construction is, in the module that owns
the type (2026-08-20, "only a type's owner constructs one").

Clay chose this spelling over a keyword form (`u with role "chef"`), which
meant the same thing.

Owes, on #1886: the builtin on every engine; error goldens for an unknown
field, a value the field does not admit, and a merge outside the owning
module; a micro golden showing unnamed fields carried over; appendix B and
the chapter that teaches records.

## 2026-10-13 — gavel: a bracketed list or map may span lines

Clay ruled #1882. All twenty ports wrote a list or map too long for 80
columns, and nothing let one break. Text blocks (`"""`) already cover long
strings.

**The ruling.** A list or map literal may span lines. The opening bracket ends
its line, each element (or key and value) sits on a line of its own indented
one level, and the closing bracket stands alone at the opening line's indent.
Canonical form keeps one way to write each literal: one that fits in 80 columns
on one line is written on one line, and one that does not is written in the
spanning form. Operator chains, `if` and guards are not covered, and they are
still broken by naming their parts.

Owes, on #1882: the lexer and parser accept the spanning form on every
engine; error goldens for a short literal spanned, a long literal on one line
and a misplaced closing bracket; the samples that build long literals by hand
moved onto it; the chapter that teaches lists.

## 2026-10-13 — gavel: parentheses are required where `and` meets `or`, and refused wherever else they group nothing

Clay ruled the first part of #1887. The ports asked for parentheses to be
optional where two operators meet, for clarity. Clay: "not just no but hell no.
we are all about one right way and avoiding redundancy. you either require the
parentheses or they're not allowed." And on the rule below: "I'm fine doing
that as long as it's 100% consistent and not optional."

**The ruling.** Wherever `and` and `or` appear in one expression without
parentheses between them, the expression is refused, and the diagnostic shows
the parenthesized spelling: `(a and b) or c`. This is the same rule #1915
builds for `&`, `|` and `^`. Every other pair of parentheses that groups
nothing stays refused, as today: `not (f x)` because application always binds
tightest, `(a * b) + c` because that precedence is not misread. There is no
optional form: each expression has exactly one accepted spelling.

Owes, on #1887: the refusal and its diagnostic on every engine; error goldens
for `a and b or c`, for `(a and b) or c` accepted and for a redundant pair
still refused; the corpus, samples, lib and sibling repositories moved onto
the required spelling.

## 2026-10-13 — gavel: a file may open with a header comment and one blank line

Clay ruled the second part of #1887. A comment directly above a file's first
declaration is accepted today and reads as that declaration's comment. A
comment at the top of a file followed by a blank line is refused, and the
refusal is wrong about why: "the file may not begin with a blank line", at
line 2, because the check skips the comment when it finds the file's start.

**The ruling.** A file may begin with a block of comment lines followed by
exactly one blank line, and that block is the file's header, separate from any
declaration. Without the blank line the block is the first declaration's
comment, as today. Nothing else about comment placement changes.

Owes, on #1887: the formatting check accepts the header shape; error goldens
for a header followed by two blank lines and for a blank line before the
header; a golden that the header-plus-blank-line file checks clean on every
engine.

## 2026-10-13 — gavel: overload arms stay in specificity order

Clay ruled #1888, which asked whether the order rule for overload arms could go
now that the checker detects disjoint arms. Clay: "if you're talking about
ordering by the hierarchy of subtypes then yes you absolutely preserve
specificity order." The rule stands as built: an overload's arms are written
most specific first, a literal before a concrete type and a concrete type
before the catch-all, and the formatting check refuses any other order. The
2026-07-25 gavel that freed top-level declaration order does not reach arms.
Nothing is owed.

## 2026-10-13 — gavel: hex and binary integers and exponent floats

Clay ruled #1889. Seven ports needed numbers that kanso refuses as spacing
errors: an emulator wrote its opcodes and a TOML parser its constants in
hand-converted decimal.

**The ruling.** An integer literal may be written in hexadecimal (`0xff`) or
binary (`0b1010`) as well as decimal, and a float literal may carry an exponent
(`1e-8`, `6.02e23`). The value is the same whatever the spelling, so the author
picks the spelling that shows what the number means. The renderer is unchanged:
it already prints exponents where it needs to. A byte literal, also in the
issue, was not ruled.

Owes, on #1889: the lexer on every engine; micro goldens for each form,
including negative and upper-case-digit cases; error goldens for a malformed
literal (`0x`, `0b2`, `1e`); the chapter that teaches numbers.

## 2026-10-13 — gavel: `text/slice` past the end answers `none`

Clay ruled #1890. `text/slice "abc" 1 10` returned `"bc"`, quietly shorter than
the range asked for, and three ports called that silent data loss. An index
past the end already answers `none`.

**The ruling.** A `text/slice` whose range runs past the end of the value, or
starts past it, answers `none`, as an index past the end does, and the caller
dispatches on it. A range wholly inside the value is unchanged.

Owes, on #1890: the change on every engine; micro goldens for a range past the
end, a start past the end and a range that ends exactly at the end; the
exhaustiveness check sees `none` in `text/slice`'s answer; lib, samples and
sibling repositories that leaned on clamping moved to dispatch on `none`.

## 2026-10-13 — gavel: `list/concat` joins two lists, and strings join by interpolation

Clay ruled #1906 on the committee's view, on the condition that it rest on
first principles rather than on what the language already had. Appendix B
described `text/concat` as joining two lists or two strings; every engine
joined only lists, and a string argument stopped at run time.

**The ruling, and its reasons.** Joining two lists is `list/concat a b`, in
std/list, and `text/concat` goes. A function lives with the type it works on,
so a list operation belongs in `list`. There is one way to combine strings:
interpolation (`"{a}{b}"`), with `text/join` for a list of pieces. A `concat`
for strings would be a second way, so none is added.

Owes, on #1906: `list/concat` on every engine with `text/concat` removed and
its callers moved; the checker refuses a string argument to `list/concat`; an
error golden for a call to the removed name that points at `list/concat`;
appendix B's entry rewritten under std/list.

## 2026-10-13 — gavel: `list/to_sorted` builds the `sorted` that `find` bisects

Clay ruled #1903. std/list declares `pub type sorted`, and `find` has an arm
for it that bisects instead of scanning (archive, 2026-07-24, "find gets its
fast arm"). Only std/list may build a `sorted`, and it exposed no function that
does, so no caller could reach the fast arm.

**The ruling.** `list/to_sorted xs` sorts `xs` by the order `list/sort` uses and
answers it as a `sorted`. It takes any list `list/sort` takes. Because it sorts
as it wraps, a `sorted` is always in order and the bisect cannot answer wrong.
No function wraps a list unchecked. When type parameters are built (#1941),
`sorted` becomes `<k>sorted` and this function's signature follows without a
change in meaning.

Owes, on #1903: `list/to_sorted` in std/list; the bisect tests back in
`list_test.kso` through it; a micro golden in which `find` on a `to_sorted`
answers the same as on the plain list; the counter that shows the bisect ran;
appendix B.

## 2026-10-13 — gavel: names for the library gaps the ports hit

Clay ruled #1914 one name at a time. Each name was decided from what it should
mean. Three of the answers overturn the issue's recommendations, which had
argued from resemblance to existing names.

**The rulings.**

- `io/read_line` is a description that yields the next line of standard
  input without its newline, or `done` at the end of input. An interactive
  program prompts between reads, and one call per line puts each read where
  the program says it happens.
- `delete map key` answers the map without that key. It is a bare builtin
  beside `put`, and the two names say the opposite things.
- `list/sort_by xs key` sorts by what `key` answers for each element, and
  `list/reverse xs` reverses. The key gets its own name because a second
  argument to `sort` would read as a comparator.
- `text/uppercase s` and `text/lowercase s`. A `to_` name means the answer is
  a different kind of thing (`to_int "42"` is a number). Changing case leaves
  a string, so the name is a verb for what happens to it, as `trim` and
  `sort` are.
- `text/starts_with? s prefix` and `text/ends_with? s suffix`. A trailing `?`
  means the function answers true or false.
- `text/pad_start s width fill` and `text/pad_end s width fill`. Left and
  right name where text lands on a screen, which depends on the script; start
  and end name the end of the string that grows, in the same words
  `starts_with?` uses.
- No `os/write_bytes`. The content passed to `os/write_file` is already a
  string or bytes, so the argument says which kind is written, and
  `write_file` has accepted both since 2026-09-09. Reading needs two names
  because a path cannot say which kind the caller wants back.
- `os/modified path` answers the file's last-modified time in milliseconds
  since the epoch, the unit `time/now` uses, and `file_not_found` as data
  for a missing file. There is no `os/stat`: one name answers one fact.

Owes, on #1914: each name on every engine with a micro golden; `read_line`
reaching `done` on an empty and an unterminated last line; `modified` on a
missing file; appendix B's entries; the ports that worked around a gap moved
onto the name.

## 2026-10-13 — gavel: an argument nobody reads is never computed, err or not

Clay ruled #1928. In the toml port's reduction, `e = boom 1` binds an err
lazily and `ignores 5 e` passes it to a parameter the body never reads.
Every engine prints the body's answer and the err never occurs. Chapter 04
says a catch-all arm declines an err, which read as if `ignores` should not
run; making it decline would mean computing every argument at every call to
learn whether it is an err, ending call-by-need at every call.

**The ruling.** A value nobody asks for is never computed, so an err nobody
asks for never occurs. A catch-all declines an err it is handed. Every
engine already does this, and nothing changes in the compiler. Chapter 04
gains the sentence, in this change. The port's fix is to dispatch on the
validation result before passing it on.

Owes, on #1928: the toml reduction kept as a micro golden pinning today's
output on every engine; #1891's second item closed with a pointer here.

## 2026-10-13 — `list/concat` joins two lists

#1906 moves concat to the module of the type it works on. `list/concat a b`
joins two lists, or the byte values `text/bytes` answers, and is the same
builtin underneath that `text/concat` was, so every engine runs it as
before. `text/concat` is gone from `lib/text/text.kso`, and every caller in
the tree moved: the library, the scripts, the benchmarks, the examples, the
book and the goldens. Files that imported std/text only for concat now
import std/list instead.

Two strings join by interpolation, `"{a}{b}"`, and a list of them by
`text/join`. The checker refuses a string handed to `list/concat` when it
can see one: a string literal, a template, or a call whose every answer is a
string. A string it cannot see still reaches the runtime, which raises
"concat takes two lists" on both engines. A call to `text/concat` is
refused at check with the unknown-name diagnostic, which now names
`list/concat` and interpolation. Each refusal has an error golden.

Appendix B's entry is rewritten under std/list. std/text lost three lines
above `to_float` and `to_int`, so the line numbers that goldens and the book
quote from `std/text/text.kso` moved with it.

kq's one caller moves on the kq branch of the same name.

On CI's rows the front end pays for the new refusal and for the moved imports,
and I have not split the cost between the two. compile_instructions went from
28,002,980 to 28,257,376, entry_instructions from 92,393,206 to 93,065,372,
library_instructions from 91,989,327 to 92,696,544, `compile_peak_bytes` from
703,170 to 703,187 and browser_compile_instructions from 413,936,384 to
416,674,694. The interpreted run's `interp_peak_bytes` went from 722,393 to
725,610 while its instructions fell to 495,318,943 and its allocations to
589,643. compile_allocs fell to 15,937 and emit_instructions to 7,239,817. On
the module path `module_visits` went from 2,909 to 2,912. The ratchet's
mutation for a concat accumulator anchored on the line naming `text/concat`,
so it now names `list/concat`, and the mem spec still goes red under it. The
floor follows the score, since #1906 builds a ruled part of the language.

## 2026-10-13 — six names for the library gaps, and `list/to_sorted`

#1914 rules nine names. Six of them are built here, in kanso, in
`lib/text/text.kso` and `lib/list/list.kso`, so every engine runs the same
source and no engine needs a new builtin:

- `text/uppercase` and `text/lowercase` map the twenty-six ASCII letters and
  pass every other character through. The walk takes `chars` once, looks
  each letter up by its offset in a 26-letter string, and joins the pieces.
  Case outside ASCII would need the Unicode tables, which no engine carries.
- `text/starts_with?` and `text/ends_with?` compare one slice against the
  prefix or suffix after checking the length, so the slice never runs past
  the end. Every string begins and ends with the empty one.
- `text/pad_start` and `text/pad_end` repeat the fill and cut it at the
  width. A string already that wide, or an empty fill, comes back unchanged.
- `list/sort_by xs key` merges the way `sort` does and takes from the right
  half only when its key is strictly smaller, so tied keys keep their order.
  `key` runs at each comparison; caching it would cost a list of pairs.
  `list/reverse` reads the list by index from the end.

`recase` has a `none` arm it never takes. The walk stops before it indexes
past the end, but the checker cannot see that bound and asks for the arm.

Each name has a micro golden, which the interpreter, the native build and
the page run alike. Appendix B gains entries for the six text names and a
sentence for the two list verbs, and drops its counts of the list verbs,
which a second count in the library would have had to track.

#1903's ruling is built beside them. `list/to_sorted xs` answers
`sorted (sort xs)`, so a caller can reach the arm of `find` that bisects, and
the list is always in order because the same call sorts it. The bisect tests
are back in `list_test.kso` through it, and a micro golden checks that `find`
agrees on the plain list and on `to_sorted`. The mem golden
`find_bisects_a_list_built_by_to_sorted` pins the bisect: a scan over its
4,096 elements builds a `step` record per element, and `sh_rec` reads 208
bytes with the bisect against 512,400 when `find` loses its `sorted` arm.

The other three rulings need the runtime: `delete map key`, `io/read_line`
and `os/modified`. They follow in their own change. `os/write_bytes` was
ruled out, so there is nothing to build for it. The ports that worked around
these gaps live on the unmerged `claude/ports-corpus` branch, and moving
them onto the names belongs to that branch.

CI's rows price the new library text. Every program that imports std/text or
std/list compiles the whole module, so the 114 lines added there are paid for
in lexing, parsing, inference and the check, split across those phases in
proportion to the text rather than concentrated in one. compile_instructions
rises from 28,002,980 to 31,634,022 (+13.0%), entry_instructions from
92,393,206 to 100,520,915 (+8.8%), library_instructions from 91,989,327 to
100,042,203 (+8.8%), emit_instructions from 7,241,390 to 7,617,766 (+5.2%),
compile_allocs from 15,984 to 17,990, browser_compile_instructions from
413,936,384 to 434,142,466 (+4.9%) and browser_compile_peak_bytes from 686,192
to 726,939. The interpreter row reads 496,099,798 against 495,525,421, with
interp_allocs 592,223 against 589,689 and interp_peak_bytes 794,345 against
722,393. Welfare falls from 91.34 to 91.27. These names are ruled (#1914,
#1903), so the floor drops by that cost. A compiler that left an unreached
library function unparsed and unchecked would stop this cost from growing with
each library addition; that is open as a lead.

Two more rows move with the library's size. interp_instructions lands on
496,099,798 from 495,525,421, and the module sample's module_visits, the
inference fixpoint's declaration visits in bench/compile_golden_modules.txt,
lands on 3,663 from 2,909: the fixpoint visits each new declaration on every
round it is dirty.

The compile-memory gate reads the same text. On `compile_corpus`,
`front_end_visits` went from 8,133 to 9,326, `front_end_rounds` stayed at 19,
and `compile_peak_bytes` went from 703,170 to 791,610, 12.6% more held while the
corpus is checked, because the two modules it imports are 114 lines longer and
their whole text is held through the check. It costs the welfare meta about
0.02, and the floor drops by that under the language rule. Checking only what
an import reaches would take this back, and #1952 tracks it.

## 2026-10-13 — hex and binary integers, and floats with an exponent

#1889 builds the gavel "hex and binary integers and exponent floats". The
lexer reads `0x` followed by hex digits, in either case, and `0b` followed by
binary digits as integers: `0xff` is 255 and `0b1010` is 10. A literal past
64 bits becomes a big integer, as a long decimal does. A decimal followed by
`e`, an optional sign and digits is a float: `1e-8`, `6.02e23`, `2.5e3`. A
minus in front reads as it does for a decimal, so `-0x10` is -16. The
prefixes and the `e` are lower case only, as every other word in the
language is.

Three malformed spellings stop at lexing, each with an error golden: `0x`
with no digits after it, a digit other than 0 or 1 in a binary literal
(`0b2`), and an `e` with no exponent digits (`1e`). Each message names the
literal and says how to write it.

Every engine reads the same tokens because they share the lexer. The micro
golden `hex_binary_and_exponent_literals` runs on the interpreter, the
native build and the page. The editor grammar and the playground's
highlighter colour the new forms, and chapter 02 says what each spelling
means. The byte literal in #1889's title was not ruled and is not built.

On CI's rows the lexer's new arms cost the emitter 408 instructions,
emit_instructions 7,240,982 -> 7,241,390, and the tab's compile 10,978,
browser_compile_instructions 413,925,406 -> 413,936,384. The rest fell with the
binary's layout: compile_instructions by 9,355 to 28,002,980,
entry_instructions by 30,753 to 92,393,206, library_instructions by 30,149 to
91,989,327 and interp_instructions by 3,960 to 495,525,421. The floor follows
the score, since #1889 builds a ruled part of the language.

## 2026-10-14 — `delete`, `io/read_line` and `os/modified`

These are the three #1914 names that need the runtime. #1948 carries the six
written in kanso. All three run on the interpreter, both native tiers and the
page's translator.

`delete map key` answers the map without the key. A key that is not there
answers an equal map, and the map it was handed is not changed. It sits beside
`put` as an ambient name, so a program that declares its own `delete` is now
told the name is taken; kq, vse and kanso-json declare none. Natively the
answer is built from the map's deduplicated sorted view, so a key written twice
is gone after one `delete`. A first argument that is not a map is refused at
check time, and a float key at run time.

`io/read_line` is a description that yields the next line of standard input
without its `\n` or `\r\n`, or `done` at the end. A lone `\r` is kept, since
only those two endings were ruled. It flushes standard output before it waits,
so a prompt written ahead of it reaches the terminal first. Bytes that are not
text are refused the way `os/read_file` refuses them. A read error counts as
the end of input, which is what `io/stdin` already did natively. The page has
no standard input and refuses `read_line` as it refuses `stdin`.

`os/modified path` yields when the file last changed, in milliseconds since
the epoch, or `file_not_found`. A failure other than absence is reported in
`os/read_file`'s words. In the page it yields `file_not_found`, matching
`os/exists`.

New goldens: four micro programs, one error and two runtime diagnostics, and a
stdin corpus under `tests/golden/stdin/` that `tests/read_line.rs` feeds to
native and the interpreter. One test there waits for each prompt before typing
the answer, so an unflushed prompt fails it. Each was watched red: native and
interpreter `delete` keeping the key, `\r` left on the line, and the flush
removed.

CI's rows price the three builtins. Every native binary carries their runtime
code, 2,688 bytes more text per bench, so `text` went from 4,932,142 to
4,969,774 across the fourteen. Each bench also runs about 234 more
instructions, the same on every one, so the rise is outside any decode loop:
work_basket 30,946,278, work_deepbench 403,499,544, work_digestbench 5,909,247,
work_escapebench 39,058,340, work_indexbench 2,462,571, work_jsonbench
858,186,231, work_livebench 1,474,854,142, work_oneshot 13,205,901,
work_pendbench 191,674,024, work_readbench 4,580,963, work_runbench
1,178,202,929, work_scanbench 273,675 and work_widebench 28,274,676, while
work_encodebench fell to 2,386,636,337. The compiler checks and lowers three
more names and carries the larger runtime text: compile_instructions
28,044,444, entry_instructions 92,512,951, library_instructions 92,130,190,
emit_instructions 7,245,801, codegen_instructions_dev 119,400,046 and
codegen_instructions_release 477,099,466. The interpreter's run went to
interp_instructions 495,589,622, and the tab to browser_compile_instructions
414,205,634 and browser_run_instructions 34,183,945. The floor drops by the
cost under the language rule, since #1914 rules these names.

Carried together over main 859ad275 in #1953, the six names and the three
builtins land on one set of CI rows, each read against main. The front end pays
for both: compile_instructions 31,948,363 from 28,257,376, entry_instructions
101,372,705 from 93,065,372, library_instructions 100,887,003 from 92,696,544,
compile_allocs 17,941 from 15,937, emit_instructions 7,622,247 from 7,239,817
and module_visits 3,666 from 2,912. The compile corpus holds more while it is
checked, compile_peak_bytes 789,911 from 703,187, with front_end_visits at
9,326. The interpreter's run reads interp_instructions 496,215,717,
interp_allocs 592,174 and interp_peak_bytes 792,646, against 495,318,943,
589,643 and 725,610. The tab's compile reads browser_compile_instructions
437,381,890 and browser_compile_peak_bytes 726,939, against 416,674,694 and
686,192, and its run browser_run_instructions 34,183,945. Most of the compile
cost is the 114 lines the two library modules grew, which every program
importing them reads whole; #1952 is the change that would take it back. Both
halves build ruled names (#1914, #1903), so the floor drops by the combined
cost under the language rule.
