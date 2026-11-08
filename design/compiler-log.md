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

## 2026-11-01 — the corpus forks

Corpus first, before #2268. #2268 makes a server's loop hold what one
request holds, and with CI's rows its welfare read just under the floor: no
benchmark forked, so the objective saw the cost of the new runtime code and
none of the memory the change saves. This adds that shape on main, so the
fix is measured against it.

`bench/runbench/runbench/forked` runs 16,000 passes of a loop that forks a
small handler beside the rest of the loop, the way a server forks per
accept. `bench/interp_corpus` runs the same loop for 2,000 passes before it
prints. Neither printed line moves.

Rows, from CI where they are instructions. runbench rose to work_runbench
892,161,749, the shape's 45,830,571 instructions or 5.14% of the program,
and its text row to 849,809. Its arena grew from 4 blocks to 15:
arena_peak_bytes 14,680,064, and spare_peak_bytes 14,155,792. The
interpreted corpus rose to interp_instructions 622,671,293, interp_allocs
814,155 and interp_peak_bytes 1,953,942. In the tab, the corpus reads
browser_compile_instructions 349,229,303, browser_compile_peak_bytes
855,995, browser_run_instructions 79,318,152 and browser_run_peak_bytes
3,801,088, measured with the rustc 1.98.1 the golden names. runbench's
emitted rows grew by the shape's 15 functions. The floor is re-based to what
these rows score, since the change is only to what is measured.

The run program's other counters move with the 16,000 groups and the
handlers' pages: run_allocs 1,930,504, run_alloc_bytes 237,915,969,
run_arena_blocks 15, run_arena_peak_bytes 14,680,064, run_perm_allocs 131,
run_beat_iters 211,300, run_survive_slots 4,703, run_push_mut_fast
1,195,311, run_push_mut_slow 1,645,465, run_bytes_malloc 7,868,
run_str_scans 19,402, run_str_scan_bytes 18,667,266, run_buf_reuse 157,
run_ten_frees 0, run_ten_handups 0, run_sh_str 17,954,080, run_sh_buf
92,421,040 and run_spare_peak_bytes 14,155,792. runbench's 15 new functions
take the emitted rows of the other benchmarks to emitted_other_defines
2,327, emitted_other_calls 21,198, emitted_other_branches 18,260 and
emitted_other_lines 174,304, and the text rows sum to text 5,730,080.

## 2026-11-01 — a server's loop holds what one request holds

The second half of #2263. A server forks each request's handler beside the
rest of its loop, so every pass is a group the next pass is a member of, and
the groups nest as deep as the server has run. Both engines kept every one.
tests/golden/mem/a_server_loop_holds_what_one_request_holds.kso is that
shape without the network: natively 4,000 passes held 4 arena blocks and
16,000 held 13; interpreted, the peaks were 1,420,281 and 5,581,819 bytes.

Three things held the memory, and each fix was watched red on its own
before the spec passed.

A group whose other members have finished cleanly, and whose join was all
its fiber had left, will yield exactly what its last member yields. The
scheduler now hands that member to the parent group and frees the group's
slot; the next join takes it. Nothing observable moves. The settled node the
group would have handed back ran next in any case, at the same position,
because every fiber ahead of it waits until later, so a failure lands in
the parent at the same point in the order it did before. The interpreter
learns that the join was the fiber's whole work from a rejoin that is
absent; the runtime learns it from a continuation that is its settled node.

Natively, the scheduler ran under one beat, so everything its fibers
allocated stayed in the arena until it returned. It now stages the way the
bind chain does. Once the arena has drifted a quarter megabyte past the last
stage (or twice what that stage copied), the fibers' work and the live
groups' continuations and results go out through the carry pair as one list,
and the beat is rewound. A continuation's settled node is found again by
walking the copy, since binds and rescues keep what they wrap in `x`, rather
than carried as a root of its own; two roots could be copied apart, and the
group's result would then be written into a node its continuation no longer
holds.

In the interpreter, a global's resolution slot was keyed by the syntax node.
The pipe builds its callback's body by cloning the head on every evaluation,
so each pass's clone took a new slot until the table reached its cap of
65,535, after which every new node resolved by name. A name now keeps the
slot it first took. An effect chain of 16,000 links peaked at 1,134,478
bytes and now at 21,608, one byte more than at 4,000.

After: one arena block at either size natively, each stage copying about
350 bytes (beat_iters 49 and evac_bytes 17,488 in the mem golden), and
interpreted peaks of 41,986 and 41,987 bytes.
tests/a_server_loop_holds_no_more_the_longer_it_runs.rs compares the two
sizes on both engines. #2263 is done.

Rows, from CI, against the corpus with the forking shape. runbench's arena
falls from 15 blocks to run_arena_blocks 4, run_arena_peak_bytes 4,718,608
and run_spare_peak_bytes 2,621,456. The stages that buy that copy a little:
run_beat_iters 211,349, run_evac_allocs 2,156, run_evac_bytes 4,599,264,
run_allocs 1,930,602, run_alloc_bytes 237,920,257 and run_sh_buf
92,424,544. The interpreted corpus peaks at interp_peak_bytes 1,366,143
against 1,953,942, with interp_instructions 621,869,223 and interp_allocs
810,137, and the tab runs it in browser_run_instructions 78,361,551 with
browser_run_peak_bytes 2,621,440.

The runtime gains the stage and the group bookkeeping, 1,792 bytes of text
in each benchmark (runbench's text row 851,601, and the text rows sum to
text 5,755,168). runbench itself falls to work_runbench 891,782,287. No
other benchmark forks, so none runs the new code, and their work rows move
by layout: work_deepbench rose to 199,830,851, work_widebench to
28,766,254, work_encodebench to 2,335,341,102, work_pendbench to
154,125,558, work_oneshot to 9,980,977, work_livebench to 938,655,609,
work_jsonbench to 549,338,202, work_digestbench to 4,104,932,
work_readbench to 4,584,342, work_escapebench to 40,309,377, work_basket to
18,296,047, work_indexbench to 1,888,161 and work_scanbench to 268,197.
clang compiles the larger runtime in codegen_instructions_dev 116,951,400
and codegen_instructions_release 371,843,999, and entry_instructions reads
86,797,006.

## 2026-11-01 — a module may name a function a derivation writes into it

#2265. An entry that derives loads its dependencies twice: once to describe
them to its derivations, and again with the arms those derivations defined,
each joined to its module as one more file. The first load refused any name
nothing had written yet. A module could still call such a name on an
argument, because a call on a value a function was handed travels to the
module that owns the value's type and is resolved later. Passed as a value,
as in `list/map rows post_from`, the same name was refused. senro's
guestbook hit it with a reader `models/tables` could have written for each
model.

The first load now sets an unknown name aside instead of refusing it, and
records that it did. The second load runs whenever a name was set aside,
even when no derivation defined anything, and refuses whatever is still
unknown with the same words as before. An entry that derives nothing loads
once, as it did.

Two goldens in tests/golden/derive pin it on both engines:
a_module_calls_a_name_its_derivation_defines prints `["hello" "again"]`,
and a_module_calls_a_name_nothing_defines is refused at `post_form` with
the message an unknown name always had. With the setting-aside turned off,
the first goes red, refused as an unknown name at `post_from`.

Rows, from CI on the tree merged with #2268. The tab compiles its corpus in
browser_compile_instructions 349,228,936, 367 fewer. The compiler's own rows
rise by the set-aside check and the layout it moves: compile_instructions
1,098 to 24,098,635, library_instructions 1,097 to 72,619,875 and
entry_instructions 1,961 to 86,798,967. Welfare falls 0.000004, and the
floor is lowered by exactly that to 95.61270397: the feature is #2186's
ruling that derived declarations join the program as if written.

## 2026-11-01 — gavel: a pool is a channel without order

#2264 asked how fibers hand each other a value, since a connection pool needs
one fiber to wait for another to give a connection back. Clay ruled for a
pool: "a generic structure similar to a channel which is basically just a
channel without order and doesn't care particularly about connections or
anything like that. it can be used on any arbitrary thing."

- A pool holds up to n values of any kind. Taking waits while it is empty and
  answers some value in it, with no order promised. Putting waits while it is
  full. Both are effects, parked and woken by the scheduler that #2263
  reworked.
- No order is promised to the program, but the scheduler stays deterministic,
  so a program takes values in the same order on every run and every engine.
- FIFO channels are not added. A later need for them comes back as its own
  question.
- The public names go to Clay as one question before they ship: #2274, which
  recommends `std/pool` with `make`, `take`, `give` and `with`.

Owed: the pool on both engines and in the tab; goldens for a take waiting on
an empty pool, a give waiting on a full one, the hand-out order on every
engine, and a deadlock reported rather than hung; and senro's per-request
connections moved onto a pool.

## 2026-11-01 — gavel: a server rescues its handler's failure in library code

#2252 proposed endpoint policy as an ordinary function, `on_failure report`.
Clay declined it as a language feature. senro and the application are
separate hakos, and rescue works across hakos, so senro's serve loop can
already rescue a handler's failure and answer 500. Clay: "if the server code
itself needed to rescue, then you should have just been using some
alternative to exceptions in the first place because you don't want to use
exceptions for control flow." A failure a hako expects to handle itself is a
value (`none` or a marker), not a boxed err. A plain program's top-level
report is unchanged.

Owed in senro: the serve loop rescues a handler's failure and answers 500,
with a spec showing the server keeps serving afterwards.

## 2026-11-01 — gavel: an HTTP status is a value, never an err

#2253 proposed classifying a failure by where it was born and a `retry n` on
events. Clay closed both: "an HTTP code is not an exception. it's a thing that
can actually happen." A status is part of the response, which the program
dispatches on, so an HTTP client never turns a status into an err; only a
round trip that could not complete fails. This follows from #2087's ruling
that expected outcomes are values. Whether to try again is the author's
decision in ordinary code, because the author knows whether the request is
idempotent. A library `retry` comes back as a question when an application
needs one.

The checks it asked for come back clean. std/net/http has no client yet, only
the server, so nothing turns a status into an err. Chapter 04 and the rest of
the site carry no prose built on the retired page's status-code example.

## 2026-11-01 — a long chain inside a fiber costs no stack and keeps no garbage

#2276: a chain of `.>` steps that never waits runs within one step of its
fiber, and both engines walked it by recursion, one frame per link. Natively
that ran out of stack at a hundred thousand links once a second fiber stood
beside the chain. The interpreter's gigabyte stack held out until three
million. A chain in a program's only fiber never reached this code, because
`k_exec` already walks a chain in a loop.

Both steps now go round a loop: a link that answers another description
becomes the next thing to step, and the interpreter's three chain words share
one arm where they had three. The order of the program cannot change, since
the whole chain still runs within the step it ran in.

The loop exposed the native step's second problem. The scheduler reclaims
only between steps, so a long chain kept every link's garbage until it ended:
140 bytes a link, 425 MB at three million. Every 256 links the step now
asks whether the region has drifted as far as the scheduler lets it before
staging. When it has, the step returns the rest of the chain as a new kind of
step. The scheduler keeps the fiber's wake time, stages at the top of its
loop, and picks the same fiber again, since nothing else moved. Peak memory
for the reproduction is 10 MB at a thousand links and at three million.

The first version cost runbench 2.1 million instructions (CI's reading,
891,782,287 to 893,890,434). Measured under callgrind in this container, which
read runbench within a hundred instructions of CI: the loop alone made it
1.28 million cheaper than main. Asking about drift after every link cost
650,000, and a separate loop in the scheduler for the new kind of step cost
2.5 million in the scheduler code around it, which is inlined into `k_exec`.
Asking every 256 links, and folding the new kind into the branch that already
handles a blocked fiber, leaves runbench 685,000 instructions under main
here.

Specs, each watched red against main:

- `micro/a_long_chain_beside_another_fiber_runs_to_the_end`, 150,000 links on
  every engine. On main the native run dies with "the program ran out of
  stack".
- `oracle.rs`'s `a_long_chain_in_a_fiber_needs_no_stack_per_link` runs the
  interpreter on that sample with an eight-megabyte stack. On main the thread
  overflows.
- `mem/a_long_chain_in_a_fiber_reclaims_as_it_goes`, 20,000 links:
  `arena_blocks=1` and `arena_peak_bytes=1048576`. On main it reads 4 blocks
  and 4 MB.

No existing allocation golden moves. With the test after every link, the
server loop and the run program each staged 1,152 bytes fewer; asked every 256
links, both read what they read on main.

CI's rows, read on the recorded AMD silicon: `work_runbench` falls 684,996 to
891,097,291. The interpreter's step loop costs the interpreted corpus 426,003
instructions, `interp_instructions` 622,295,226, the price of one loop in
place of three recursive arms. The runtime's machine code is 1,568 bytes
longer in every benchmark, `text` 5,777,120, and the dev tier compiles it in
`codegen_instructions_dev` 116,955,821 while the release tier falls to
371,732,303. The tab runs the same runtime and gains the same loop: it runs the
interpreted corpus 205,146 instructions cheaper, at 78,156,405. Welfare rises,
and the floor is banked with it.

## 2026-11-01 — the scheduler is kept out of line

`k_schedule` is static and has one caller, `k_exec`, so clang inlined it. The
scheduler's loop then sat inside `k_exec`, which is called once for every
effect a program runs, and every one of those calls paid for its wider frame
and prologue whether or not the program ever forked. Marking it `noinline`
takes runbench from 891,097,386 to 890,386,161 instructions under callgrind in
this container. Runbench calls `k_exec` about 122,000 times, so that is close
to 6 instructions a call.

The tab runs the same runtime compiled to wasm, and there the inlined loop is
the cheaper one: out of line, `browser_run_instructions` rose 42,057 to
78,198,462. The attribute applies only outside wasm, and the tab reads its
golden again.

I found it while pricing the pool (#2264), which adds to the scheduler loop
and so to the inlined body. It lands first so the pool's floor drop counts
only the pool.

CI's rows, read on the recorded AMD silicon. `work_runbench` falls 711,225 to
890,386,066, `work_deepbench` falls 1,971,289 to 197,859,562 and
`work_widebench` falls 79,936 to 28,686,318. The runtime's machine code is 16
bytes shorter in every benchmark, `text` 5,776,896.

Five rows rose by small amounts: `work_jsonbench` to 549,338,235,
`work_readbench` to 4,584,375, `work_digestbench` to 4,104,967,
`work_livebench` to 938,655,546 and `work_encodebench` to 2,335,340,787. That
is 33 to 581 instructions on runs of 4 million to 2.3 billion. Callgrind
shows no `k_schedule` frame in readbench or livebench, so none of this is the
scheduler's own work. I read it as code that moved when the function left
`k_exec`, without having isolated it. Clang does a little more work compiling
the runtime with the function kept apart: `codegen_instructions_dev` rises to
116,962,084 and `codegen_instructions_release` to 371,787,654. Welfare rises,
and the floor is banked with it.

## 2026-11-01 — std/pool: fibers share values through a pool

The pool #2264 ruled for is built on the interpreter, natively and in the tab.
It ships as `std/pool` with `pool/make n`, `pool/take p`, `pool/give p v` and
`pool/with p work`, the names #2274 recommended and Clay chose.

A take from an empty pool or a give to a full one parks its fiber. Natively
the step answers a kind of its own, and the scheduler gives the fiber a wake
time no clock reaches. A take or give that succeeds marks the pool as moved,
and before the scheduler next picks a fiber it wakes every parked one at the
current time. Each tries again in spawn
order, and one that still cannot proceed parks again. That is a retry for
every parked fiber each time any pool moves. For a few connections it costs
nothing worth measuring, and a program holding thousands of parked fibers
would want a wait list per pool.

When the earliest wake left is the parked mark, every fiber is waiting on a
pool and nothing can wake any of them. The scheduler then wakes them all once
more with the pool marked as failing, and each stuck take or give answers an
err that says which it was. The program ends with those reasons instead of
hanging; `every_fiber_parked_on_a_pool_is_reported` pins the report on both
engines.

The ruling promises no order of hand-out, and the scheduler keeps one anyway:
a take answers the value given last, on every engine.
`a_pool_hands_out_in_one_order` runs four fibers over two values and pins the
transcript.

Natively a given value is deep-copied into storage the pool allocates for
itself, and a take copies it back into the arena. The pool holds nothing in a
region a beat can rewind, so a value given in one request survives the
reclaiming that follows it.

`pool/with` hands a taken value to the work and gives it back however the work
ends. #2274 said it would use annotate (`.!`), and it cannot: a failure passed
as an argument goes straight through the call, so `give` would never run. It
rescues the work's failure with `.?` instead, which std/pool may do because
the failure is foreign to it, gives the value back, and raises the failure
again with `effect`, so the reason and birthplace are the ones the work gave
it. The issue now says so.

A stress test for the pool found #2276, a long chain inside a fiber that ran
out of native stack. It needed no pool to show, and #2278 fixed it first. That
fix lets a long chain stop partway through its step so the scheduler can
reclaim, and the pool has to respect it. A give made inside such a chain
would otherwise wake the waiting taker at the stop, before the chain had
finished, where the interpreter runs the whole chain first. The scheduler
leaves parked fibers alone until the chain's step is done.
`a_give_inside_a_long_chain_wakes_the_taker_after_it` pins the order: without
the guard, the native run printed the taker's line first.

The pool is a ruled feature, so the welfare floor drops by exactly what CI
measured, 0.00159831, to 95.61462007. The figures below are against main with
the scheduler kept out of line (#2279).

`work_runbench` rises 527,874 to 890,913,940, because the scheduler asks
whether a pool moved before each pick and tells a parked step from the others
after it. The first version cost more. Inlined into `k_exec`, the bigger
scheduler made every effect dearer, which is what #2279 fixed. Giving a
parked fiber its own step kind, in place of a blocked step with an
unreachable wake time, took about 320,000 more off runbench under callgrind
here.

The runtime carries the pool's code, so every benchmark's machine code is
3,712 bytes longer, and `text` sums to 5,828,864. The other work rows move by
layout: `work_oneshot` rises 325 to 9,979,993, `work_encodebench` 262 to
2,335,341,049, and `work_jsonbench` 549,338,238, `work_widebench` 28,686,321,
`work_digestbench` 4,104,970, `work_readbench` 4,584,378 and `work_livebench`
938,655,549 rise by 3. Clang compiles the bigger runtime in
`codegen_instructions_dev` 117,030,226 and `codegen_instructions_release`
371,871,683. The tab runs the interpreted corpus dearer, at
`browser_run_instructions` 78,338,381, and compiles it dearer, at
`browser_compile_instructions` 349,271,241. The compiler's rows move with the
bigger builtin tables and library: `compile_instructions` 24,102,309,
`entry_instructions` 86,809,695, `library_instructions` 72,627,848,
`emit_instructions` 5,599,665 and `interp_instructions` 622,391,507. No peak
and no allocation counter moves.

One CI run landed on an Intel runner (family 6), where every work row reads
130 instructions lower and the codegen rows differ too, because glibc picks
its string routines by CPU. Those rows went into the goldens for one push and
the next AMD run refused them. The rows above are the AMD readings, the
silicon `bench/dispatch.txt` records; a run whose `interp_sample` line names
another CPU family is not a source of rows.

## 2026-11-02 — gavel: the pool's names are make, take, give and with

#2264's ruling asked for the pool's public names as one question before they
shipped, and #2274 asked it. Clay chose the recommended option: `std/pool`
with `pool/make n`, `pool/take p`, `pool/give p v` and
`pool/with p (v -> work)`.

- `give` is the word for putting a value back, which keeps it apart from
  `put`, the map insertion.
- `with` takes a value, runs the work with it, and gives the value back
  whether the work succeeds or fails. A failure still rises with the reason
  and birthplace the work gave it.

Owed: nothing beyond #2264's list. The pool (#2277) already uses these names.

## 2026-11-03 — gavel: senro is built out under a mandate, and keeps old paths only as aliases

#2280 asked how a senro app keeps an old URL working once every route is
RESTful. Clay ruled on that and widened it.

- The declaration is `legacy_route_aliases`, a name that marks it as a
  compatibility measure. Every resource still gets its RESTful route, and an
  alias can only point at one. A wrong alias is refused while the app
  compiles.
- senro gets singular resources, as Rails has them: a resource with no
  `:id`, such as `/profile`.
- senro's design is delegated. Clay: "go to town and spend weeks expanding
  the framework with a clear idea of what my underlying philosophy and
  principles are, adding authentication a la devise, and basically making
  this kanso alternative to ruby on rails." Cloud makes senro's design calls
  and brings the whole to him for review. CLAUDE.md states the rule.

Under the mandate, an alias answers with a permanent 308 redirect to its
canonical path, so each resource keeps one URL.

Owed: senro's principles written down for Clay to correct, then
`legacy_route_aliases` and singular resources with specs.

## 2026-11-03 — gavel: templates are files of their own, and a library compiles them while the program compiles

senro's views were Slim files, each with the kanso file kata generated from it
checked in beside it. Clay: "i don't want to see actual slim files in here, i
just want a special kanso-specific format... designed from first principles,
and not paired with a kso file. it should stand on its own and work and have a
conversion/compilation step." On how: "probably this means kanso provides
some kind of templating layer where you can create an arbitrary format that
parses into normal kanso. i dunno. whatever you think best."

The layer is the derivation, which already runs a library's kanso while the
program compiles and can write source into a module (`derive/define`). Two
things were missing, and this change adds them:

- `derive/module` gains `files`: the text of each file in the module's
  directory that is not kanso source, by name. A convention reads its
  templates there.
- A directory that holds no kanso source is still a module when a derivation
  writes its functions. While the entry's dependencies load to be described,
  it loads empty; the second load compiles what was written, and refuses the
  directory if nothing was.

Specs: `a_derivation_writes_a_module_from_its_templates` and
`a_module_of_templates_nothing_writes` in the derive corpus, on both engines.
The first failed with `no .kso files in …/views` before the loader change.

The format is senro's to design under #2280's mandate. kata's templates
become `.kata` files and its syntax follows kanso: `takes` names the
parameters, holes are `{expr}`, control lines are bare `if`, `else` and `for`,
and `#` starts a comment. That work lands in senro.

Describing a program now costs a little more, because each module's record
carries its files and the loader asks one more question of an empty
directory. The rows rose, by CI's measure: entry_instructions to 86,840,306
(+30,611), browser_compile_instructions to 349,289,583 (+18,342),
library_instructions to 72,628,273 (+425), emit_instructions to 5,599,909
(+244), compile_instructions to 24,102,378 (+69) and interp_instructions to
622,391,510 (+3).
The ruling is a part of the language, so the welfare floor drops by exactly
that cost, to 95.61456.

## 2026-11-03 — a write to a peer that has gone is an err on both engines

A senro server died without a word when a page that had been listening for
changes closed. Native code wrote to the socket with `write()`, so the closed
peer raised SIGPIPE, which the driver leaves at its default so that a program
behind `head` ends quietly. The interpreter writes through Rust's std, which
suppresses the signal, and answered `cannot write: Broken pipe (os error 32)`,
a wording the host supplies. The engines disagreed, and the interpreter is the
oracle.

The native write now ignores SIGPIPE for its duration and fails with EPIPE.
The runtime runs one thread, so the handler it sets around the write is its
own. Both engines say `cannot write`. The micro corpus's
`a_write_to_a_peer_that_left_is_an_err` dials its own listener, closes the far
end and writes; before the fix the native run exited 141 with no output. The
tab has no sockets, so the program is listed in the native route's known gaps.

The two calls around the write cost code, by CI's measure. Every benchmark's
machine code grew by 64 bytes, so `text` rose to 5,829,760 across the
fourteen. Clang does more work on the runtime: codegen_instructions_dev to
117,032,350 and codegen_instructions_release to 371,872,769. Four run rows
moved with the layout: work_oneshot rose to 9,980,896 and work_runbench to
890,914,087, while work_encodebench fell to 2,335,339,866 and work_livebench
to 938,655,381. Welfare falls by a sliver.
Engines that agree are part of the language's specification, so the floor
drops by exactly that cost.

## 2026-11-03 — kanso build writes a module a browser can run

`kanso build page.kso --wasm` writes `page.wasm`: the emitted module
lowered by `ir_wasm` to a wasm side module, the one the playground links
against `runtime.c` built for wasm32. A page needs two numbers before it can
link the module, the data bytes and the table slots it asks for, and they
ride in the module as a custom section named `kanso.link`, two
little-endian u32s, so the file is all a page has to fetch. senro's views
compiled for the browser are built this way (#2280).

`tests/build_wasm.rs` builds a two-module program with the flag, reads the
section back, links the module against the runtime under wasmi and checks
that it prints. It failed on the unknown flag before the change.

The flag is a branch in the driver, and the binary's layout moved with it, by
CI's measure. Four front-end rows rose by a handful of instructions:
compile_instructions to 24,102,382, entry_instructions to 86,840,310,
library_instructions to 72,628,277 and startup_instructions to 50,267.
codegen_instructions_release rose to 371,872,843. emit_instructions fell to
5,599,243. Welfare falls by a sliver, and the
browser build is part of the senro mandate, so the floor drops by exactly
that cost.


## 2026-11-03 — an editor asks the compiler what a name is

Clay asked for a language server that works in RubyMine and in the
playground, whose hover shows the type the compiler worked with. `kanso lsp`
speaks the Language Server Protocol over stdin and stdout. It publishes the
diagnostics `kanso check` prints, answers a hover and jumps to a definition.
The playground runs the same `Server` through a wasm export, `kanso_lsp`,
which takes one message and returns the replies as a JSON array.

The diagnostics are read out of the check's own rendering, so an editor shows
the words the command line does, from every compile route, with no second list
to keep in step. A hover reads the program the check compiled. A parameter's
type is what it accepts: each call the parameter is handed to contributes the
union of what the arms it can reach take at that position, and the
parameter's uses are intersected. An arm that binds the position by name passes
on what that name accepts, so the answer follows a value through every call it
passes, as a greatest fixpoint over the whole program. With `fn bar s:string`
and `fn bar n:int`, `fn foo x` whose body is `bar x` shows `x: int | string`.
When the program's own calls hand the parameter something narrower, the
inference's `param` set, which the engines dispatch on, appears on a second
line. Unions are spelled the way typesets and appendix B spell them.

A use the walk does not read constrains nothing: an operator's operand, a
builtin's argument, a capture in a lambda. The answer can be wider than the
truth and never narrower. Builtins are the gap most readers will meet, since
`text/split s sep` hands both to `builtin_split` and so reads as accepting
anything; the interpreter knows each builtin's argument types only by refusing
the wrong ones at run time, and the table that would close the gap is a
follow-up. A buffer that does not compile has diagnostics and no hovers.

`tests/the_language_server_answers_an_editor.rs` replays three recorded
sessions from `tests/golden/lsp` and compares every reply byte for byte. The
overloads transcript went red when a call contributed only its first arm, and
green with the union.

The server is outside the compile path, and main.rs gains only the verb, so
the compile rows moved with the binary's layout, by CI's measure on the tree
merged with `kanso build --wasm`: compile_instructions to 24,099,445 (-2,937),
entry_instructions to 86,832,572 (-7,738), library_instructions to 72,621,853
(-6,424), startup_instructions to 50,264 (-3), emit_instructions to 5,600,882
(+1,639) and interp_instructions to 622,499,949 (+108,439).
codegen_instructions_release rose to 371,873,010 (+167), which this diff does
not explain: the emitted IR and the runtime are unchanged. The tab's compile
rose too, browser_compile_instructions to 349,339,330 (+49,747); the wasm
engine now carries a second caller of `compile_source`, which is a likely
reason, though nothing here isolates it. Welfare fell by less than 0.01.
Editor support for the language is part of making it usable, so the floor
drops by exactly that cost.

## 2026-11-03 — the usage text names build --wasm

`kanso --help` listed `build <file|dir> [--release|--counters]` after #2286
added `--wasm`. The usage line now names the flag and says what it writes,
and `tests/front_door.rs` holds the line; it failed on the old text.

Only a string in the driver changed. By CI's measure, on the tree merged
with the language server, codegen_instructions_release fell to 371,872,995
(-15) and no other row moved. The emitted IR is unchanged; the binary's
layout is the likely cause, though nothing here isolates it.

## 2026-11-04 — a password stretched and a digest carried join the run program

`sha256/pbkdf2` peaked at 1,525,448 kB resident for 100,000 iterations
(#2299). The issue reduced it to a tail loop that hands `sha256/digest`'s
answer to its next iteration and carries a list it never reads, and that
loop keeps every digest's garbage too. Under the 2026-09-05 corpus-first
gavel both shapes join the run program here, measured on main before the
fix.

`stretched` runs `sha256/pbkdf2` for 200 iterations and `carried` runs the
postcard for 600 digests. On this container the release-tier run program
reads 1,045,660,009 with both. Without `stretched` (one iteration) it reads
1,023,809,139, a column of 21,850,870 (2.09%); without `carried` (none) it
reads 1,013,906,580, a column of 31,753,429 (3.04%). Each phase keeps its
loop's work: `stretched` about ten kilobytes an iteration and `carried`
about five. The counts are sized so the program's peak sees it: with both,
arena_peak_bytes reads 6,815,760 where it read 4,718,608, and at 250
digests `carried` stayed under the old peak and the objective could not see
it.

The mem vein takes the two fixtures, each at today's growth:
`a_digest_hashed_again_round_a_loop` (600 digests, three arena blocks) and
`a_password_stretched_three_hundred_times` (five). Their outputs agree with
Python's `hashlib`.

The issue's control row, the same loop without `p` at 2,296 kB, does not
reproduce on this tree. `chain u left`, a public loop with no wrapper, and
the wrapper with its unread list removed both keep every digest: five arena
blocks at a thousand digests, like the postcard. The extra parameter is not
the cause. `sha256/digest` answers through `list/fold`, and inference gives
a library function one answer for every caller in the program, so the
digest reads as any value. A loop slot that may hold any value is never
carried across a rewind.

CI's rows for the two phases: work_runbench 890,914,087 -> 947,406,395
(+56,492,308, 6.0% of the new whole), run_allocs 1,930,602 -> 2,040,700,
run_alloc_bytes 237,920,257 -> 245,492,129, run_arena_blocks 4 -> 7,
run_arena_peak_bytes 4,718,608 -> 6,815,760, run_spare_peak_bytes 2,621,456
-> 4,718,608, run_cohort_frees 11 -> 13, run_perm_allocs 131 -> 133,
run_push_mut_fast 1,195,311 -> 1,302,366, run_push_mut_slow 1,645,465 ->
1,657,081, run_buf_reuse 157 -> 7,813, run_sh_rec 4,675,856 -> 4,723,232,
run_sh_buf 92,424,544 -> 98,439,088, run_sh_bytes 5,112 -> 5,160,
runbench's emitted lines 69,443 -> 71,622 and its text 856,929 -> 889,681,
which take the totals emitted_other_branches 18,260 -> 18,498,
emitted_other_calls 21,198 -> 21,507, emitted_other_defines 2,327 -> 2,359,
emitted_other_lines 174,304 -> 176,483 and text 5,829,760 -> 5,862,512.
run_beat_iters fell, 211,349 -> 195,099, though both phases add loops. This
container reads 195,099 with the phases present as well. Inference is
whole-program, so new callers of the shared library functions can change
another phase's beat verdicts; that is a likely cause, and nothing here
isolates it. Welfare re-bases run_instructions and run_peak_bytes (6,842,513
-> 8,939,665), each keeping its ratio to its baseline, and the floor moves
only by the rounding of the two baselines.

## 2026-11-04 — pbkdf2 and a rehashed digest give their arena back

#2299. Both shapes joined the run program in #2301, measured before this.

**pbkdf2.** `chained` handed its next iteration two lists, the latest HMAC
and the running XOR. A list built in one iteration and read in the next is
a value that crosses a rewind, and the carry tier that could copy it out is
closed to library loops by the 2026-09-01 and 2026-09-08 entries. So the
loop kept everything every HMAC built. It now carries the sixteen words as
arguments, as `compress` has carried its eight since 2026-09-23, and the
beat report reads "rewinds every iteration". Each word read out of a list
is masked with `whole`: an item of a list could be anything to inference,
and the mask, a no-op on a word, makes the argument a number. Without the
mask the loop kept its grow-only verdict, "argument 2 may carry heap".
`paired_xor` had no other caller and is gone.

**The postcard.** The issue's shape is a loop that hands
`sha256/digest u` to its next iteration. Its slot read `0x37fff`, every
bit, and `costly_to_carry` refuses a slot that reads as anything. The cause
is the digest's answer. It spelled its words as bytes through `list/fold`,
and inference gives a library function one return set for every caller in
the program, so the fold answers anything and so does the digest. `bytes_of`
is now a push loop of its own, the slot reads `0x220` (a list or an err),
and the loop carries it. The general case, any function answering through a
shared driver, is #2302.

A first attempt narrowed `costly_to_carry` to the set the self-tail-calls
hand the slot. It changed nothing, because the handed set was the digest's
TOP, and `infer::builtin_set` answers TOP for a name it does not know, so it
could not have told a builtin from a library function anyway. It was
reverted before commit.

**Measured on this container**, the same box before and after:

| `pbkdf2`, 100,000 iterations, built `--release` | before | after |
|---|---:|---:|
| wall | 4.24 s | 0.68 s |
| user / sys | 0.93 s / 3.29 s | 0.67 s / 0.00 s |
| peak resident | 1,523,724 kB | 2,300 kB |

Output is byte-identical to Python's `hashlib.pbkdf2_hmac`, and the 13
vectors in `lib/sha256/sha256_test.kso` pass, 4,096 rounds among them. The
interpreter does not move: 10,000 iterations take 22.9 s before and 22.5 s
after. The mem fixtures read one arena block each, where they read three
and five.

Their other rows move with the loop that now rewinds:
a_digest_hashed_again_round_a_loop_beat_iters 52,828 -> 53,428,
a_digest_hashed_again_round_a_loop_buf_reuse 2,401 -> 1,187,
a_digest_hashed_again_round_a_loop_evac_allocs 33 -> 47,
a_digest_hashed_again_round_a_loop_evac_bytes 3,728 -> 7,536,
a_digest_hashed_again_round_a_loop_push_mut_fast 61,274 -> 43,874,
a_password_stretched_three_hundred_times_beat_iters 13,827 -> 14,126,
a_password_stretched_three_hundred_times_buf_reuse 3,614 -> 1,816 and
a_password_stretched_three_hundred_times_push_mut_fast 70,276 -> 44,205.
A loop that gives its arena back each iteration copies what it carries out
first, which fits the evacuations, and has no buffer of the last
iteration's left to reuse; the byte loop's pushes replace the fold's. That
reading fits the rows, and nothing here isolates each counter.

**What an iteration costs now.** Callgrind on 2,000 iterations reads
202,252,987 instructions, about 101,000 an iteration for two compressions.
`digested`, with the rounds and the schedule inlined into it, is 50%;
`big_sigma0`, `big_sigma1`, `majority` and `choice` are 34% between them,
called rather than inlined; list pushes and the fold that spells the first
HMAC are about 10%. A C SHA-256 spends a few thousand instructions on a
compression, so this is roughly ten times that. **OPEN:** inlining the four
mixing functions is the next lead.

CI's rows, on the run program with both phases: work_runbench 947,406,395
-> 941,723,266, run_arena_blocks 7 -> 5, run_arena_peak_bytes 6,815,760 ->
5,767,184, run_spare_peak_bytes 4,718,608 -> 3,670,032, run_allocs
2,040,700 -> 2,022,734, run_alloc_bytes 245,492,129 -> 243,362,481,
run_cohort_frees 13 -> 12, run_beat_iters 195,099 -> 195,898,
run_evac_allocs 2,156 -> 2,190, run_evac_bytes 4,599,264 -> 4,608,512,
run_push_mut_fast 1,302,366 -> 1,267,595, run_push_mut_slow 1,657,081 ->
1,653,484, run_buf_reuse 7,813 -> 5,385 and run_sh_buf 98,439,088 ->
96,452,880. The sixteen-argument loop and the byte loop are more code:
runbench's emitted lines 71,622 -> 73,713 and its text 889,681 -> 892,849,
which take the totals emitted_other_branches 18,498 -> 18,759,
emitted_other_calls 21,507 -> 21,738, emitted_other_defines 2,359 -> 2,358,
emitted_other_lines 176,483 -> 178,574 and text 5,862,512 -> 5,865,680.
Compiling the longer library costs entry_instructions 86,832,572 ->
87,188,418 (+355,846) and library_instructions 72,621,853 -> 72,961,217
(+339,364), read the same on cpu family 0x19 and 0x1a. Welfare rises 95.61
-> 95.63 and is banked.

## 2026-11-04 — bytes nobody can predict

senro's sign-in needs random salts and tokens, and std had only
`math/random`, which `KANSO_SEED` replays (#2298, under #2280).
`os/random_bytes n` is a description that yields `n` bytes from the
operating system's secure source. Native code calls `getentropy` in
256-byte pieces, or `arc4random_buf` on macOS, as desc 40. The interpreter
makes the same libc call from Rust. In the tab, wasi-libc's `getentropy` is
WASI's `random_get`, which `kanso-engine.js` already answered with
`crypto.getRandomValues`. The interpreter build, `kanso.wasm`, had no call
out to the page: its seed and sources are pushed in through exports. A
pushed buffer cannot know how many bytes a program will ask for, so it
gained one import, `kanso_host.random_fill`, which the page fills from
`crypto.getRandomValues` in 65,536-byte slices. The wasmi hosts in `tests/`
define it through `wasm32::page_linker` and leave the zeros. The import is
wasm32's alone: macOS refused to link the native library with it unresolved,
which Linux had allowed. Off the page the browser executor takes the
machine's bytes.

Three specs, each seen red. `random_bytes_are_as_many_as_were_asked_for`
in the micro corpus asks for 32, 1 and 0 bytes and appends `"ok"`, which only
bytes accept, so it pins the length and the kind of value on every engine the
corpus reaches. It went red at `31 ok` with native broken to one byte short.
`random_bytes_takes_a_count` in the runtime corpus pins the refusal of a
string on both engines. `tests/random_bytes_ignore_the_seed.rs` runs one
program twice under `KANSO_SEED=7` and holds the dice equal and the bytes
different. It went red when the interpreter drew from its seeded generator.
Under node with the page's host function, the interpreter build replays the
dice and not the bytes, and it fills a 70,000-byte ask.

The rows are from a job on 0x19 model 0x11, on the tree merged with main.
Before the merge, a job on model 0x1, the silicon the goldens were recorded
on, read the same numbers as one on 0x11. Every binary's `.text` grew by 272 bytes, the new
case and helper, so text across the fourteen benchmarks rose to 5,869,488
(+3,808). The runtime instruction rows fell by 45 to 1,578, though no
benchmark runs the new case, so layout is the likely cause; nothing here
isolates it. codegen_instructions_release rose to 371,998,504 (+125,509) and
codegen_instructions_dev to 117,050,653 (+18,303), likely clang compiling the
larger runtime. The front end moved too, and its code changed only by the
table entries, so again layout is the likely cause:
compile_instructions to 24,100,614 (+1,169), entry_instructions to 87,192,183
(+3,765), library_instructions to 72,964,207 (+2,990) and emit_instructions
to 5,601,693 (+811). interp_instructions fell to 622,464,159 (-35,790). In
the tab, browser_compile_instructions rose to 349,357,398 (+18,068) and
browser_run_instructions to 78,344,393 (+6,012). Welfare fell by less than 0.0001. A secure source of randomness is required
for the sign-in the senro mandate asks for, so the floor drops by exactly
that cost.

The page's sections for `kanso build --wasm` and the language server were
owed, and this change was the fourth entry since the page last moved, so
docs/compiler.html gains all three as sections 281 to 283.

## 2026-11-04 — a parameter handed to a builtin reads as what the builtin takes

Kanso#2288. The hover in `kanso lsp` followed a parameter into every group
it was passed to and stopped at builtins, because nothing recorded what a
builtin takes. `fn parts s = text/split s ","` read `s` as the whole union.

`infer::BUILTIN_TAKES` now holds it, beside `infer::builtin_set`, which says
what a builtin answers. A row is a set per position over the full tag
vocabulary. The check's own table in `src/check/literals.rs`, written in
literal kinds, is deleted, and the literal check, the none check and the two
list checks read the new rows. The hover meets a parameter's set with the
row's set at each position it is handed to a builtin, so `s` reads
`string`. `print` keeps one rule in literals.rs, the language's own: the
check refuses a literal that is not a string, while every engine prints any
value a program computes.

Each row carries a third field, how many of its leading positions the
compile-time checks enforce. The first build had the checks read every
position, and the suite showed what that costs: the none check refused a
key that may be none in `scripts/welfare_rescore` (40 failures across the
rescorer's specs), and the literal check refused `delete m 2.5`, which the
runtime corpus pins as the interpreter's refusal. Refusing more at compile
time is a change to the language, which #2288 did not ask for, so the
checks enforce exactly the positions the old table held. The hover reads
every position.

`tests/a_builtin_refuses_what_its_row_leaves_out.rs` checks every row
against the interpreter. For each position a row constrains, it hands the
builtin a value of each of fourteen tags the row leaves out, hidden behind
an index into a one-element list so no check that reads the row answers
first, and requires `error[runtime]` under `kanso play --interp`. A control
with only fillers must run without a refusal. 1019 probes. It went red
with `BYTES` dropped from `concat`'s first position, and red again in the
control with a float as `split`'s filler. The golden transcript
`tests/golden/lsp/builtins` went red with the hover's lookup removed.

Measuring rewrote several rows. `concat`, `push`, `length` and `utf8` take
bytes, `utf8` takes a list, `write_file` and `net_write` take bytes as text,
and `put` and `delete` take an int or a string as the key. `length`'s
refusal now names bytes, which moved two error goldens and the book's
appendix A sample. The handle positions of `net/read`, `pool/take` and their
kin are gone from the table: std hands them a record's `handle`, and only
their own module can build the record, so no program reaches them.

The probe ran native first by mistake, since `kanso play` builds a binary,
and found native `text/find2` and `text/find2_below` answering for any value
in their int positions where the interpreter refuses. That is #2310.

The rows are from a job on family 0x19 model 0x1, the recorded silicon.
compile_instructions fell to 24,090,335 (-10,279), entry_instructions to
87,188,542 (-3,641), library_instructions to 72,961,337 (-2,870) and
browser_compile_instructions to 349,295,198 (-62,200). Nothing isolates the
cause; the check now reads one table through one function, and layout may
account for some of it. No runtime, text or allocation row moved. Welfare
rose from 95.627876 to 95.627948, banked.

## 2026-11-04 — find2 refuses what the interpreter refuses

`text/find2` and `text/find2_below` take bytes and then three or four ints. The
interpreter refused anything else in those positions; native answered (#2310).
The emitted shims `k_b_find2_fast` and `k_b_find2_below_fast` test every tag
and send any call that is not bytes and small ints to the out-of-line doors
`k_b_find2` and `k_b_find2_below` in runtime/text.c. Those doors tested only
the bytes, so a bool, a string or a float was scanned with its payload as the
number, and a bignum with its pointer. The doors now refuse anything but an
int, in the interpreter's words (`find2 takes bytes`, `find2_below takes
bytes`).

A tag test alone would have made native refuse a bignum, which the interpreter
reads. So the doors read one as the interpreter does: a start past a word
scans from the front, a byte keeps the low byte of its magnitude, and a floor
past a word sits above every byte or below them all. A negative small byte
still reads two ways, the magnitude's low byte in the interpreter and `& 0xff`
in native. That one is on the scanning path and is #2313.

Four specs, each seen red. Three runtime goldens pin the refusal of a bool
start, a string byte and a float floor on native, `--interp`, the oracle and
the tab; native wrote nothing to stderr before the fix. The micro golden
`a_bignum_reads_the_same_in_every_find2_position` puts a bignum in each
position, and with the fix reverted native printed nothing where the
interpreter prints `2 2 3 2 4`.

The rows are from a job on 0x19 model 0x11, on the tree merged with #2311,
which moved none of the same rows. No scan path changed, since the
new tests sit behind the shims' own. Every binary's `.text` grew by 288 bytes,
so text across the fourteen benchmarks rose to 5,873,520 (+4,032).
codegen_instructions_release rose to 372,000,820 (+2,316), likely clang
compiling the larger runtime, and codegen_instructions_dev fell to 117,050,435
(-218). The runtime instruction rows fell by 1,890: work_encodebench to
2,335,339,191 (-630), work_oneshot to 9,978,933 (-385) and work_runbench to
941,722,133 (-945), while work_livebench rose to 938,654,286 (+70). No benchmark reaches the
doors with a non-int, so layout is the likely cause of all four; nothing here
isolates it. The front end, the emitter, the interpreter and the tab did not
move. Welfare rose by less than 0.0001 and the floor is banked.

## 2026-11-04 — find2 reads a negative byte as the interpreter does

The interpreter reads each byte `find2` and `find2_below` look for as the low
byte of the int's magnitude, so -98 is 98. Native's two raw scans read
`a & 0xff`, so -98 was 158, and `find2 (bytes "abc") 1 -98 0` answered 4
where the interpreter answered 2 (#2313). The interpreter is the oracle, so
native changed. Both scans read `& 0xff` first and correct the two bytes
only when `(a | b) < 0`. At the emitted sites the bytes are literals and the
test folds away.

The first version computed both magnitudes on every call, through a helper
that the release tier's hot unit and `a_short_scan_tail_answers_like_the_byte_walk`
each lift out of the runtime by name. The second lift did not know the
helper and the spec failed on both hosts, so the magnitude is computed in
place. That version cost the browser run 92,400 instructions; the single
sign test costs 52,800.

The micro goldens `find2_at_its_edges` and `find2_below_at_its_edges` each
gain a negative byte in the scalar walk and one in the vector step. Before
the fix native answered 5, 25, 10 and 23 where the interpreter answers 2, 6,
2 and 3.

The rows are CI's, on the recorded host. Twelve binaries' `.text` grew by
128 bytes, and scanbench's and runbench's, which inline the raw scans most,
by 1,248 and 1,264, so text across the fourteen benchmarks rose to 5,877,568
(+4,048). work_runbench rose to 942,094,162 (+372,029
over main). Four other work rows moved by a few hundred instructions:
work_encodebench to 2,335,339,429 (+238), work_livebench to 938,654,937
(+651), work_oneshot to 9,979,423 (+490) and work_scanbench to 267,717
(+141). An earlier run on the first version read encodebench 3.8M lower,
and this one reads it within 238 of main, so neither reading is the change.
browser_run_instructions rose to 78,397,193 (+52,800), the sign test on the
wasm runtime's calls. codegen_instructions_dev rose to 117,058,158 (+7,723)
and codegen_instructions_release fell to 371,954,589 (-46,231). The front
end, the emitter and the interpreter did not move. Welfare fell by less than
0.01. The differential law requires the fix, so the floor was lowered by
exactly that, with the reason in bench/welfare_floor.json.

## 2026-11-04 — sha256's rounds carry raw words, and pbkdf2's chain skips its bytes

`sha256/pbkdf2` was about nineteen times slower than OpenSSL at the release
tier (#2304). A 5,000-round run retired 440M instructions, 82% of them in the
inlined `digested` loop: about 36,000 a 64-byte block, where a C round is
some thirty instructions and ours was 355.

The eight working words crossed from round to round as boxed KValues, and
every bit operation re-tested its operands' tags. Three reads in
`lib/sha256` kept inference from proving them ints. `words_from` handed the
state's eight words to `blocked` as bare list reads, and the schedule handed
two of its words to the sigma functions the same way; both are masked with
`whole` now. The third was the round constant, read from `rounds`, which is
built by `list/to_list (list/concat ...)`. Inference gives a library function
one return set for every caller (#2302), so a read of `rounds` may be a
failure, and the failure reached `compress`'s first and fifth parameters as
`INT | ERR`. The constants now arrive as an argument. An argument never holds
a failure: a bare name refuses one when the arm is chosen, on both engines,
so the body's parameters are typed without it. With all three gone the eight
words travel as raw i64, a round is 162 instructions, and the run fell to
270M.

The second change is in the chain. Each link is an HMAC of thirty-two bytes,
and both of its hashes continue from a state one block in, so the one padded
block is known word by word: the eight words, `0x80000000`, six zeros, and
768 for the length in bits. `after_eight` builds the schedule from those
words. The chain used to make a list of the words' bytes, push the padding
onto a copy and gather the bytes back into words, twice a link. The run fell
to 190M, 57% under main. The eight results go back through a call, `eight`,
rather than a list literal, because a literal whose items might fail widens
every list read in the program (`store_fails` in infer.rs).

Tried and dropped: the schedule fused into the rounds as a sixteen-word
window of arguments, so no list is built. The digest was right and the run
rose to 205M. A loop of twenty-six arguments goes through the arena's carry
(`k_beat_lap_carry` and `k_carry_stage`, 15% of that run), which costs more
than the 48 pushes it saves.

The digest is the same on every engine, a 50-round digest matches Python's
`hashlib.pbkdf2_hmac`, and `kanso test lib/sha256` passes. Allocations fell
in the digest and run counters (runbench 2,022,734 -> 2,007,972). In the
mem vein, the password fixture went from 14,126 beat iterations to 349 and
from 3,290,352 allocated bytes to 1,526,976, and the rehashed-digest fixture
from 53,428 beat iterations to 15,028.

Some counters read worse by their direction and are less work. A link no
longer builds a list of bytes, so it pushes and reuses buffers less:
run_buf_reuse fell to 3,390, run_push_mut_fast to 1,238,867,
a_password_stretched_three_hundred_times_buf_reuse to 19 and
a_password_stretched_three_hundred_times_push_mut_fast to 478, while every
allocation and byte counter beside them fell too. The emitter writes the new
functions: across the other benchmarks emitted_other_defines rose to 2,365,
emitted_other_calls to 21,933, emitted_other_branches to 18,972 and
emitted_other_lines to 180,154, all of it in runbench and digestbench.

The rows are CI's, on the tree merged with the find2 fix above.
work_runbench fell to 916,918,990 (-25,175,172, -2.7%) and
work_digestbench to 3,288,791 (-815,411); no other work row moved.
runbench's and digestbench's binaries are smaller (text 891,873, -2,800,
and 349,633, -4,224), and the emitter writes 201 more calls into runbench
for the new functions. The library is part of every compile, so
entry_instructions rose to 87,745,536 (+556,994) and library_instructions to
73,432,676 (+471,339). The browser rows did not move. Welfare rose by 0.06
and the floor is banked. At the release tier 100,000 rounds take 0.22 s in
this container, where they took 0.45 s; the dev tier, which `kanso run`
uses, barely moved, from 3.1 s to 3.0 s.

## 2026-11-04 — net/peer reads the address at the other end of a connection

A server that counts sign-in attempts per client needs to know which client
it is talking to, and nothing in `std/net` said. `net/peer c` answers the
address at the other end of a connection as text: `127.0.0.1`, or an IPv6
address in its short form. senro's sign-in rate limit is the first reader;
senro's accept loop already adds a `senro-at` header to each request, and the
peer goes in beside it, so `std/net/http`'s request record is unchanged.

It is description tag 41 natively, read with `getpeername` and written with
`inet_ntop`, and `Desc::Peer` in the interpreter, read from the stream's
`peer_addr`. A handle that is not an open connection answers "that is not an
open socket" on both engines, the native runtime's words for the same fault
in `net/port`. The tab and the scripted executor have no sockets and refuse,
as they refuse every other net word. The row goes last among the builtins
the interpreter dispatches, so no existing word's scan position moves.

The spec is in `tests/sockets_serve.rs`: a one-statement server accepts the
harness's connection and prints who called. Watched red on each engine in
turn, with the address replaced by `0.0.0.0`.

The rows are CI's. The runtime's new case adds 288 bytes of `.text` to every
binary: jsonbench 442,145, encodebench 368,056, oneshot 448,337, basket
366,024, widebench 382,142, deepbench 333,303, escapebench 323,335,
pendbench 342,395, indexbench 322,233, scanbench 528,076, digestbench
349,921, readbench 323,537, livebench 452,911 and runbench 892,161, so text across the fourteen rose to
5,874,576 (+4,032). Clang
compiles the larger runtime: codegen_instructions_dev 117,101,715 (+43,557)
and codegen_instructions_release 371,967,035 (+12,446). The table and the
library grew by one word each, so the front end and the engines read a
little more: compile_instructions 24,090,400 (+65), entry_instructions
87,746,589 (+1,053), library_instructions 73,433,509 (+833),
emit_instructions 5,602,736 (+1,043), interp_instructions 622,466,237
(+2,078), browser_compile_instructions 349,304,764 (+9,566) and
browser_run_instructions 78,403,189 (+5,996). No benchmark calls `net/peer`,
so the work rows move by layout: work_jsonbench 549,338,422, work_encodebench
2,335,340,285, work_oneshot 9,980,090, work_basket 18,296,275,
work_widebench 28,686,523, work_escapebench 40,309,541, work_pendbench
154,124,923, work_indexbench 1,888,327, work_scanbench 268,526,
work_digestbench 3,289,743, work_readbench 4,584,584, work_livebench
938,655,611 and work_runbench 916,954,250 (+35,260), while work_deepbench
fell to 197,755,729 (-103,256). Nothing here isolates a cause for those.
Welfare fell by less than 0.005; `net/peer` is the prerequisite for senro's
sign-in rate limit under #2280, as `os/random_bytes` was for its sign-in, so
the floor drops by exactly what CI measured, with the reason in
bench/welfare_floor.json.

## 2026-11-04 — a call to list/fold answers what its init and lambda can answer

#2302. Every arm of `list/fold` answers by calling `f`, a value it was
handed, and inference reads a call to a handed value as answering anything.
So every fold in a program answered anything, and so did any function that
answered through one. A tail loop that carried such an answer had a slot
reading TOP, and `costly_to_carry` in src/beat.rs never carries a TOP slot,
because it may hold bytes. The loop stayed grow-only and kept every
iteration's work.

A call to the shipped fold now answers the group's answer intersected with
what that call passes: its init, the answer of the lambda it hands as `f`,
a failure, or a thunk. A callback that is not a lambda literal keeps the
group's answer. The rule rests on a promise the fold arms keep (each answers
its init, an answer of `f`, or a failure), and lib/list/list.kso states it
above them.

The lambda is walked at the call, with its accumulator holding the init and
then whatever the walk answers, until that stops growing (`eval_fold_step`).
The first version used the Lambda arm's walk, which seeds every parameter as
any value. That read `list/group_by`'s `put acc k v` as a put into a list,
which can answer none, and the runtime golden
`a_library_failure_after_a_callback_names_the_outer_line` was refused at
compile time for printing a value that might be none. Its accumulator is only
ever the map it started with, and with the accumulator walked as that map the
golden reports its runtime failure again. A fold whose lambda really can
answer none now reads as possibly none where it read as anything, so the
checker asks for that none to be handled; before, it was let through.

The macOS host then refused scripts/welfare_rescore. Its `columned` answers
either `without row "welfare"`, a fold, or `put row "welfare" score`, and the
fold's answer had hidden the put's. `builtin_set` answered `LIST | NONE` for
a put on a value that might be a list, whatever the key, and a list handed a
text key is refused at run time on both engines ("put writes a list at an
int position"), so it never answers none. A put now admits the none only
when its key may be an int. The micro golden
`a_put_by_text_key_never_answers_none` is a program main refuses and this
branch prints, on both engines. `kanso check` over every directory in
scripts, lib, examples and bench reports the same errors on main's binary
and this one.

The new mem fixture `a_fold_answer_carried_round_a_loop` carries a fold's
answer round twelve thousand iterations. It held 4 arena blocks before and
holds 1. A user function that answers `list/to_list` of a fold over a
digest's words held 13 blocks at two thousand iterations and holds 1. Both
print the same on every engine.

`list/map` answers a `mapped` record, and `list/to_list` reads it through a
fold, so a function that answers `list/to_list (list/map xs f)` reads as a
list as well: the mem fixture `a_mapped_answer_carried_round_a_loop` holds 1
block where the compiler before this entry held 4.
A driver that answers one element, such as `list/find`, answers whatever the
elements can be. (Corrected after merge: this paragraph first said the other
drivers were not narrowed.)

The rows are CI's, taken on the tree merged with net/peer, on the recorded
silicon. runbench's work fell to 915,613,360 (-1,340,890) and its text to
871,297 bytes (-20,864); basket's text fell 96 bytes and both programs emit a
few fewer calls and branches. Some counters rose. work_basket rose to
18,296,297 (+22). emitted_other_lines rose to 180,275 (+121) and runbench's
emitted lines to 75,463 (+128) while its machine code shrank. interp_allocs
rose to 810,150 (+13). In the run program, run_beat_iters rose to 196,398
(+500), cohort_frees fell to 11 and run_perm_peak_bytes rose to 1,883,425
(+8,208). In the tab, browser_compile_instructions rose to 350,942,505
(+1,637,741), browser_compile_peak_bytes to 856,131 (+136) and
browser_run_instructions to 78,418,178 (+14,989); browser_run_peak_bytes fell
to 1,572,864. The front end visits 7,388 declarations on the compile corpus,
9 fewer, and the module compile row fell on every count, to 1,163 lines, 109
calls, 80 branches, 29 defines and 1,449 visits.

The compile rows are from two runs on AMD family 0x19 model 0x11, which agree
to the instruction and read every other row as the recorded silicon does:
compile_instructions rose to 24,210,964 (+120,564), emit_instructions to
5,612,352 (+9,616), entry_instructions to 88,288,229 (+541,640) and
library_instructions to 73,806,390 (+372,881).

The first version cost the front end 207K more instructions on the compile
corpus. A third of that was the test for the shipped fold, run on every arm
of every call; it now runs only where the call's shape is a fold's, and the
name test is a suffix compare. That left 122K, the second walk a lambda takes
when its answer widens its accumulator. Seeding each lambda from its last
visit's accumulator cost more than it saved and was dropped. Welfare rose by
0.0016 and the floor is banked.

## 2026-11-05 — the dev tier's run joins the development side

`kanso run` and a bare `kanso build` hand a developer a binary built at `-O0`,
and no welfare term counted how fast that binary runs. kanso#2304 is where the
gap showed: 100,000 rounds of PBKDF2 take 0.22 seconds at the release tier and
3.0 at the dev tier, and a fix for the dev tier had no term to score on. The
2026-10-06 ruling puts a term under every environment kanso runs in. Under the
2026-09-05 corpus-first gavel the term lands first, measured on the compiler
that still has the gap.

The workload is bench/runbench, the program the production side counts.
build_benchmarks.sh builds it a second time at the dev tier as `runbench_dev`,
and instructions.sh counts that binary beside the other rows. On CI the dev
build reads 2,955,847,773 instructions against the release build's
915,613,360, a little over three times as many; the other fourteen rows read
as their golden does, so the job counted on the recorded silicon.

The term is `dev_run_instructions`, at 0.10 of the development side and
satiation 1.0, the interpreter's. A dev run and an interpreted run are the same
wait from a developer's chair, so the new term carries about what interpreter
speed carries. The seven terms already on the side keep their shares
multiplied by 0.9, so every ratio between them is what it was.

There is no memory term. A dev build of runbench with `--counters` prints the
same counters, byte for byte, as bench/cost_golden_run.txt, which the release
build writes. The dev tier links the same runtime and allocates the same way,
so a second memory term would weigh `run_peak_bytes` twice.

The new term starts at its baseline, where it earns half its weight, while the
side's other terms sit far above theirs. So the meta falls by 0.65 with no
counter moving, and the floor re-bases by that amount; the `--set` records
why.

## 2026-11-05 — the dev tier stops paying a call for each bitwise op

A dev build leaves the runtime's helper twins out of line (`emit_ir_dev` has
the measurement), so at `-O0` every `&`, `|`, `^`, `bits/shl` and `bits/shr`
was a call into a twin that tested both tags and then did one instruction. In
sha256's rounds those calls were 48% of a dev run, and 100,000 rounds of PBKDF2
took 3.05 seconds at the dev tier against 0.24 at release (kanso#2304).

Two changes. A bitwise op whose operands the sets prove are ints is now the
machine op (`direct_bit_op` in src/codegen/words.rs). The only tests left are
the ones the sets cannot answer: a tag test for an operand that may be a
bignum, and a range test for a shift by an amount that is not a literal.
Anything that fails a test goes to the twin, whose C entry owns the message.
The word is kept beside its box, so the next op reads it directly. And a dev
build keeps `alwaysinline` on two helpers, `k_b_push_mut_known` and
`k_b_append_mut_int`, because inlined each saves the run more than it costs
the compile. Every helper was priced on runbench's own dev module; the others
cost far more to compile than they save, `k_b_at_fast` 203,771,636 instructions
of `clang -cc1` against 12,070,000 of the run. `KEPT_INLINE_AT_DEV` carries the
two numbers that decided it.

The rows are CI's, on a job whose unchanged rows read as their goldens do.
runbench_dev fell 133,722,370 to 2,822,125,403 (-4.52%), runbench fell 871 to
915,612,489 and digestbench 802 to 3,288,941. codegen_instructions_dev fell
249,464 to 116,852,251, since the dev module now writes an instruction where it
wrote a call. Machine code fell 384 bytes on runbench and 304 on digestbench.
Four counters rose: emit_instructions to 5,612,808 (+456),
browser_compile_instructions to 350,953,001 (+10,496), emitted_other_lines to
180,457 (+182) and emitted_other_branches to 19,018 (+50), the last two because
a tested op writes its test and its fallback inline. Welfare rose from 95.04 to
95.06 and the floor is banked. PBKDF2 at the dev tier now takes 1.7 seconds.

Goldens: `a_bitwise_op_on_words_skips_the_tags` in the micro corpus, and
`a_sum_past_the_word_meets_a_mask` and
`a_shift_by_a_computed_amount_past_the_word` in the runtime corpus, the two
refusals reached from the tested arm. All three went red when the tested arm
answered zero and the op was made an `or`.

What is left of the gap is `-O0` code itself: sha256's rounds spill every value
to the stack, and list reads whose container inference cannot prove a list
still call `k_b_at_fast`. Compiling the whole module at `-O1` cut PBKDF2's run
to 0.44 seconds and cost clang four times the compile, which the dev tier
exists to avoid.

## 2026-11-05 — a call that answers one kind meets the arms that take it

kanso#2331. senro's migration runner named a parameter `done`, which is the
nullary literal, so its one arm took only the value `done`. A call handing it
a map passed `kanso check` and failed on both engines with "no overload of
`pending` matches these arguments", ten edits from the cause. The literal
check already refuses `kept {}`, with "no arm of `kept` takes a map here
(arms take done)", but it reads only literals, whose kind is on the page.

The same refusal now reaches an argument that is a call of a group whose
every arm answers one kind: a map, a list, a string, an int or a float, with
a failure and a thunk set aside, since a thunk is forced before it is
matched. Inference's return sets are joins, so they may say more than a call
answers and never less; a group that answers a map and nothing else hands a
map, and an arm that takes no map cannot take it.

The check asks only when a call hands an argument that is itself a call,
and only at positions where every arm of the callee restricts. A group's
arms are read through the member list the box check threads, and the
program's types are collected the first time a refusal is likely.

Three tables in the same pass were keyed by the group: the none check's,
the box check's, and the discarded-effect check's, which was keyed by
position too and so hashed the name once per parameter to build. Each now
reads one `Group` built in one walk of the declarations, with the new
check's mask beside the others. The discarded-effect check also asks
whether an argument is a call before it hashes the callee, where it used
to hash every callee first. Measured here against the same tree with the
new check's call commented out, `kanso check` of the compile, entry and
library corpora ran 99k, 336k and 323k fewer instructions, so the check
and the merged table together cost less than the tree before them.

CI's rows against main agree in direction: compile instructions fell
96,410 (-0.40%), entry 327,237 (-0.37%), library 312,109 (-0.42%), the
tab's compile 665,638 (-0.19%), compile allocations 2 and the interpreted
run's allocations 6. emit_instructions rose 7, to 5,612,815, which is
layout, since the emitter runs no check. Welfare rose too little to show at two places and is banked.

A field read is left alone on either side of the call. A getter's arms do
not say what it takes or answers: on an err, `.reason` reads the err's own
reason, so `e.reason.position` hands a json failure record to a getter whose
return set, taken from the record's own `reason` field, says string. The
first version refused that line of `a_blank_run_in_json_is_read_by_each_state`
through the entry the micro corpus stages for a `pub play` library, and
`a_field_read_twice_through_an_err` is that case reduced.

Goldens: `a_map_a_call_answers_handed_to_a_done_arm` in the module error
corpus, the senro shape reduced, which checked clean before this change;
and the micro above, which prints `2` on every engine.

## 2026-11-05 — a lazy binding of an effect, handed back unread, still runs

A binding is lazy, so `change = models/updated ...` holds the call until
something reads it. A function that takes the binding and answers it unread,
as `fn owned _ write` does with `write`, answers the cell itself. Every place
that asked whether a value is an effect asked before forcing it, so the cell
read as a plain value: `owned row change .> (w -> ...)` handed `w` the
effect's description instead of running it, and the write never happened.
senro's nested scaffold found it, in an update guarded by a lookup of the
parent.

Both engines had the gap in the same three places:

- the subject of `.>`: the interpreter's piped application, in value and in
  tail position; natively `k_maybe_bind`, `k_b_bind`, and the two places
  codegen builds a bind inline;
- a callback's answer: the interpreter's `worded_step`, and natively the
  chain walks in `k_exec_unmarked`, the executor's loop and `k_step`;
- the subject of `.?` and `annotate` natively, in `k_sited_word`. The
  interpreter's builtin path already forced it.

Each now forces before it asks. The first version forced ahead of the
description test, and CI read deepbench 2,180,005 instructions (1.1%)
higher. Moving each test behind the description test left CI's reading
where it was, at +2,179,642. A profile of deepbench in this container put
the whole difference in two functions: `k_maybe_bind` (+1,308,003), where
every bind of a plain value now paid the cell test, and the executor's
loop (+871,637), which called `k_force` on every answer that was not a
description.

So the cell test now runs only where it can find something. Codegen calls
`k_bind_settled`, which has no cell test, wherever inference rules a cell
out, and `k_maybe_bind` only where the value's set holds a thunk. The inline
tail bind tests for a cell under the same condition. The executor's loops
test the tag before forcing, and the forcing sits in a cold helper out of
line. In this container deepbench then reads 198,401,946 against main's
197,862,309: +539,637 (0.27%), all in the executor's loop. That is one tag
test for each callback answer that is not a description, and it stays,
because whether a callback answers a cell is known only when it answers.
`.?`'s entry still tests first, once per rescue built.

Rows, from CI on the branch. The runtime's instructions rose with the cell
tests: `work_deepbench` 539,637 to 198,295,366, `work_widebench` 15,990 to
28,702,513, `work_runbench` 40,882 to 915,653,371, `work_encodebench` 496 to
2,335,340,781, `work_oneshot` 181 to 9,980,271, `work_pendbench` 143 to
154,125,066, `work_livebench` 90 to 938,655,701, `work_jsonbench` 7 to
549,338,429, `work_readbench` 7 to 4,584,591 and `work_digestbench` 6 to
3,288,947, while `work_runbench_dev` fell 34,240. The inline cell path adds
code where a value's set holds a thunk: `emitted_other_defines` 2,367,
`emitted_other_calls` 21,902, `emitted_other_branches` 19,043 and
`emitted_other_lines` 180,685, and every binary's text grows 1,536 bytes for
the runtime's new functions, `text` 5,877,065 in all. Compiling costs more
with the code it emits: `codegen_instructions_release` 58,837 to 372,025,872,
`codegen_instructions_dev` 3,473 to 116,855,724 and `emit_instructions` 582
to 5,613,397. The interpreter forces where it did not, and
`interp_instructions` rose 92,078 to 622,558,315. In the tab,
`browser_compile_instructions` rose 6,395 to 350,297,580 and
`browser_run_instructions` 8,001 to 78,426,179.

`a_lazy_effect_handed_through_a_function_still_runs` holds the three cases:
the binding as `.>`'s subject, as a callback's answer, and as `.?`'s subject
with a failure from std. Before the change both engines printed `<io>` for
each; after it they print what each effect does.

## 2026-11-05 — a declaration cannot take a nullary value's name

A library could declare a constant named `done`. std/list's iterators answer
`done` when they run out, and with the constant declared that answer read
the constant, so a fold elsewhere in the program found no arm for it and
failed at run time (#2338). senro's jobs module found it with a SQL string
named `done`. An entry already refused the name, because there it is a
binding and the shadow check holds the nullary values as globals; a library
declares it instead, and `collect_globals` checked declarations against the
ambient names and the types only.

A declaration named `done`, `none`, `true` or `false` is now refused as a
name already taken. The one program in the tree that declared one was
`sockets_serve`'s loop fixture, with `fn done _` as a callback; it is
`reported` now. `a_constant_named_like_a_builtin_value` in the error corpus
declares `done` and `none`, and checked clean before the change.

Rows: the new check costs every compile a little. `compile_instructions`
rose 409 to 24,114,963, `entry_instructions` 1,799 to 87,962,791,
`library_instructions` 1,497 to 73,495,778 and
`browser_compile_instructions` 3,822 to 350,291,185, and
`browser_run_instructions` did not move. The rule is part of the language,
so the welfare floor moves down by exactly the check's cost.

## 2026-11-05 — directive: a model's fields come from its table

Clay: "we need the fields for the model to be auto-magic like in ruby." He
named two ways: generate the model files the way the schema is kept, or a
primitive that maps data such as a schema to code the compiler can still
check. kanso already has the second. A derivation runs during the build,
reads the app's own data and answers declarations that are checked like
code someone wrote (#2186). senro's models derivation read the migrations
and checked each hand-written record against its table, so every column was
named twice.

`derive/define` writes source into a module as one more of its files, and the
source can be a `type`. What stopped a module from using one was the first
load of the app's modules, which describes them to the derivations before
any has run. A module that annotated `p:post` and read `p.title` was refused
there, because nothing had declared `post` yet. #2265 met the same problem
for functions and set an unknown name aside in that load. The load now also
sets aside an unknown type name and a field read no record has, in the
per-file checks and in the merged check; one helper, `set_aside`, holds all
of them. The second load, with the derived source joined in, refuses
whatever is still unknown with the message it always gave.

Three cases in tests/golden/derive pin it.
`a_module_reads_a_record_its_derivation_declares` prints its bylines on both
engines, and the binary before the change refused it with "no type is called
`post`". `a_module_reads_a_field_its_derived_record_lacks` and
`a_module_names_a_record_nothing_declares` are still refused.

senro builds on it: a module named for a table gets the table's record
declared in it, with a field for each column in the table's order, and a
module that declares the record keeps its own. `kanso derived` prints the
declared type, so the generated code can be read without being kept on disk.
Files written by a generator would drift from the schema until someone ran
it again; a derivation runs on every build.

Go's embedding was raised in the same message, as a way to put validations
and helpers on top of the generated fields. A kanso record carries no
methods, and the module's functions that take the record sit beside it, so
there is nothing to delegate through. The question and the committee's
reading are on #2356.

Rows: the describe load's added set-asides cost every derive-free compile a
little, because the helper runs on each file's diagnostics.
`compile_instructions` rose 290 to 24,115,253, `entry_instructions` 2,156 to
87,964,947 and `library_instructions` 617 to 73,496,395.
`browser_compile_instructions` fell 799 to 350,296,781, by layout. The
directive is a part of the language Clay asked for, so the welfare floor
moves down by exactly that cost.

## 2026-11-05 — a field read through an annotated name answers its own type

**DONE (#2350).** Two record types in one program that both have a `body`
field share one getter group, `Get_body`, with an arm for each type. Infer
gave the group the join of its arms, so when one type's `body` could be none,
reading `body` from the other type's value was refused as possibly none, even
where the parameter was annotated with the other type. A senro model module
with a `letter` and a `note` hit it, and `size_of` in http.kso was worked
around for it in #2351.

Infer now remembers the type of a binding that carries an annotation, and a
field read whose one argument is such a name considers only the arm whose
first pattern is that type's constructor. An unannotated read still joins
every arm, which is the old answer and still a sound one.

`tests/golden/field_reads/its_own_type` reads a `letters` body that is never
none beside a `notes` body that can be, and prints `8` and `7`; the compiler
before the change refused it. `another_types_none` reads the `notes` body
through a name of that type and is still refused, at the same line.
`tests/a_field_read_answers_its_own_type.rs` runs both.

Rows: the first version stored the annotation in each environment entry,
which grew the entry from 24 bytes to 40 and raised every compile row about
0.28%. The types now sit in a side vector that stays empty in a function with
no annotated parameters. Against main, `compile_instructions` fell 4,266 to
24,110,987, `entry_instructions` 47,358 to 87,917,589,
`library_instructions` 35,277 to 73,461,118 and `emit_instructions` 174 to
5,613,223. `browser_compile_instructions` rose 270,018 to 350,566,799, and
the side vector's first push is three more front-end allocations
(`compile_allocs` 13,937 to 13,940) and two more in the interpreted run
(`interp_allocs` 810,144 to 810,146). The sum falls by about 0.0002. The fix
lets a correct program compile, so the floor moves down by that cost.

## 2026-11-06 — a derived definition opens with the imports it needs

**DONE.** A module's files share their declarations and not their imports, so
an arm that `derive/define` added could call only what its own module
happened to import. senro's next convenience is a reader each way for every
foreign key, as Rails' `has_many` and `belongs_to` give, and those readers
call senro's `models`. A definition may now open with `import` lines. The
module's derived file opens with all of them, once each and sorted, since
the parser refuses imports anywhere else or out of order.

A relative path in a definition is written from the derivation's own
module, as that module's own imports are, and the compiler rewrites it from
the module the definition joins. senro writes `import "../models"` from
`models/`; the notebook's `pages`, two directories away from senro, reads
it as `../../models`. The derivation's path is the one the entry imports it
by (`written_from` in src/derive.rs), and the rewrite is `reread` in
src/lib/modules.rs.

`tests/golden/derive/a_defined_arm_opens_with_its_imports` gives a module
that imports nothing two arms: one calls a sibling of the derivation's
module through a path written from there, and one imports `std/list` and
`std/text`. Both engines print `["HELLO!" "AGAIN!"]` and `HELLO AGAIN`. The
compiler before the change refused it, at the import in the middle of the
derived file.

Rows: `entry_instructions` rose 12,540 to 87,930,129 and
`emit_instructions` 270 to 5,613,493; `browser_compile_instructions` fell
17 to 350,566,782. The compile and library rows did not move. The change
came with the binary and with four more comment lines in lib/derive, and
nothing here isolates one from the other. It builds the derivations ruling
(#2186), so the welfare floor moves down by exactly that.

## 2026-11-06 — directive: the language committee decides, and nothing blocks on Clay

Clay, on #2356: "nothing needs me you can just ask the language committee
like I said. I will give you feedback once I see what you've built perhaps
but you don't have to block anything for me."

A design question, including one about kanso the language, is now decided
by the committee and recorded on its issue and here. The work goes ahead on
that decision. Clay's later feedback overrides it and is recorded as a
ruling. `needs-clay` stays for what the committee cannot settle: a refusal
from the harness, a permission, or a question Clay has said he wants put to
him. CLAUDE.md states the rule under The tracker, and the places that sent
decisions to him now point there.

The first decision under it is #2356, whether an application model should
get Go-style embedding. The committee chose no implicit delegation. A model
is the record its table derives plus the functions in its module, and a
delegator is an ordinary function. A field group that several models share,
timestamps most likely, uses the `&` merge of the 2026-07-25 gavel, and its
open parts (acceptance where the parent is expected, the collision rule,
variance) are built when a second model shares one. Promotion was declined:
it needs depth and ambiguity rules, and it lets a function match a type it
was not written for by a rule the call site cannot see. #2356 is closed.

#2284, how browser code calls a JavaScript library, stays parked until an
app needs one, which is when the committee takes it up.

## 2026-11-06 — `kanso run` hands SIGTERM and SIGHUP to its program

**DONE.** `kanso run` builds a native binary and waits on it as a child. A
SIGTERM sent to the driver, which is what `kill` and process supervisors
send, ended the driver alone. The program went on running, reparented, and a
server went on listening on its port. senro's specs start each app with
`kanso run`, stop it with `kill`, and start the next on the same port, so a
later spec could reach a server an earlier one left behind. Four orphaned
servers were found on port 3000 in one container. They explain a
password-reset mail delivered to an old server's outbox and a JSON request
answered by a server built before the branch that added JSON.

`execute` in src/main.rs now spawns the program and, while it waits, passes
SIGTERM and SIGHUP on to it. SIGINT already reached the program, because a
terminal sends it to the whole process group. When the program dies of a
signal the driver handed on, the driver raises that signal on itself and
prints nothing, so whoever sent it sees the driver end by it. A program
killed by a signal the driver never received is reported as before; the
first version re-raised any signal and turned `a_program_the_system_killed`
red. The handlers are installed once. With no program running, the handler
puts the default back and raises the signal itself, so the driver never
resets them.

`tests/a_stopped_run_stops_its_program.rs` runs a native program that
writes `started`, sleeps three seconds and writes `finished`, and sends
SIGTERM to the driver once `started` appears. Before the change it failed
with "the program was still running after its driver was terminated"; after
it, `finished` never appears, the driver ends by SIGTERM, and stderr is
empty.

Rows: `startup_instructions` rose 243 to 50,507, the two `signal()` calls
the driver now makes on every run. The first version reset both handlers
after the wait and read +489. `codegen_instructions_release` fell 124 to
372,025,748 on both of CI's readings. Nothing in the diff reaches `kanso
build`, and the cause is not isolated. Welfare fell by 0.00009. The
committee decided on kanso#2370 that the change merges and the floor moves
down by exactly that, as it did for #2350: a driver that leaves its program
holding a port when it is stopped is the toolchain failing what `kanso run`
promises.

## 2026-11-07 — the entry corpus is shaped like an app (#2373)

`kanso check` of senro's pantry compiled its 27 modules 1,094 times, because
the loader compiled a module once for every import that reached it. No row
welfare reads could see that. `compile_corpus` is one module whose imports
each arrive once, and `bench/entry_corpus` imported std modules and two local
files that import nothing from each other.

This change gives `bench/entry_corpus` four local modules, models, views,
routes and mail. Each imports the corpus's own `shared` module and std/text
and std/list, the way a senro app's modules all import its models and pg.
The entry calls one function from each. On the compiler of this tree,
`KANSO_PHASES=1` prints 46 module loads for it.

`entry_instructions` rises by the new modules' cost, from 87,930,129 to
177,925,530 instructions on CI. It is a
corpus change and not a compiler change, so welfare's `compile_instructions`
baseline is scaled by the same ratio and the floor holds, as #2269 did for
the forking shape. The fix this baselines for is kanso#2374.

## 2026-11-07 — each module is compiled once per load (#2373)

`kanso run` of senro's pantry took 8.4 s to answer its first request after a
container restart. `kanso check .` alone took 2.0 s and 8,741,148,257
instructions. `load_dependencies` compiled a module for every import that
reached it, and every import inside that module the same way, so a module
cost one compile per path through the import graph. `KANSO_PHASES=1` printed
1,094 `load` lines for the pantry's 27 modules: std/text 235 times, std/list
212, std/bits 144, senro's pg 36.

The corpus could not see it. Every compile sample was one file, the module
sample imports one module, and `compile_instructions` checks `lib/json`,
which loads each module once. The first commit adds `kanso::module_loads()`,
a `loads` column in `bench/compile_golden_modules.txt`, and the sample
`tests/golden/compile/shared`: an entry importing two modules that both import
a third, all three importing std/text. Before the fix it read `loads=11` for 6
modules, in CI run 42413285691.

`compile_module_inner` now keeps a module the second time an import asks for
it and hands later importers a copy. The key is the canonical path,
`require_entry`, whether the sources came embedded, and the flags an importer
sets around a compile. A shipped std module is pruned against the words the
program has read so far, and that set only grows, so its key also holds the
set's size; a module from disk is not pruned and its key does not. The
outermost `load_dependencies` empties the cache when it returns, which bounds
it to one compile whether the root is an entry file or a directory.

The first version kept every module on its first compile. That took the
pantry to 96 loads and 0.75 s, but CI read `compile_instructions` at
25,213,290 against 24,110,987 and `compile_memory` at 859,218 bytes against
598,588. A module's program carries every declaration its imports brought in,
so keeping one is a clone of its whole subtree, and `lib/json` imports each
module once, so it paid every clone and used none. Keeping a module only on
its second request costs a set of keys. Locally, callgrind reads `kanso check
bench/compile_corpus` at 25,315,543 against 25,308,680 before the change.

On CI, against the app-shaped entry corpus #2375 added for this change,
`entry_instructions` falls from 177,925,530 to 155,859,192. The rows with
nothing to share pay for the cache's bookkeeping: `compile_instructions`
rises 6,406 to 24,117,393, `library_instructions` 43,941 to 73,505,059,
`compile_allocs` 16 to 13,956 and `compile_peak_bytes` 469 to 599,057. The
interpreter's run of its corpus loads modules too: `interp_allocs` rises 16
to 810,162 and `interp_peak_bytes` 344 to 1,366,487. Compiling in the tab
moves both ways, 3,466 fewer instructions and
`browser_compile_peak_bytes` up 208 to 856,339. `emit_instructions` falls
346, `codegen_instructions_dev` falls 677 and
`codegen_instructions_release` rises 108 to 372,025,856, which is the
binary's layout: the shipped runtime object below made it larger. The
module corpus mints `module_loads`, which reads 3. A cheaper key, the path's bytes in place of a
`PathBuf`, and keeping a module only on its third request were both measured
and dropped: neither brought those rows back, and the third request gave the
pantry back 64 of its loads.

The pantry now loads 164 times, and `kanso check .` takes 1.09 s where it took
1.94 s. Its IR is byte-identical, 2,314,435 bytes. `shared` still reads
`loads=11`: each of its modules is asked for at most twice under one key, so
nothing is handed out of the cache, and the rows match the corpus commit.

A cold `kanso build` of the pantry took 7.2 s, and 5.7 s of it was clang
compiling `runtime.c` for the dev tier into the temp directory, once per
container. build.rs now compiles that object, 377 KB, and the binary carries
it. When the key the compiler computes names the dev tier with no counters,
on the convention build.rs's clang took, a cold build writes those bytes
instead of running clang; any other recipe compiles as before. The dev tier's
`-O2` moved into build.rs, which writes it out for the compiler, so the
shipped object and a compiled one come from one list.
`tests/a_cold_dev_build_ships_its_runtime.rs` builds a program against an
empty temp directory behind a clang that logs its commands and counts compiles
of the runtime: 1 with the shipped object turned off, 0 with it. The codegen
gates warm the cache before they count, so no row they read moves.

With both changes, a cold `kanso build` of the pantry takes 1.8 s where it
took 6.5 to 7.0 s, and a warm one 1.7 s where it took 2.9 s.

## 2026-11-08 — a dev build folds small word functions into their callers (#2304)

A dev build compiles the program at `-O0`, where clang inlines nothing that
is not marked. PBKDF2's rounds call `sha256/spun`, a rotate, four times per
message-schedule step and six times per round, and each call pays for a
frame, the moves of its two words and the spills of its result. With #2376's
width bits, a dev build of 10,000 rounds spent 32% of its instructions in
`spun`.

A definition now goes out `alwaysinline` in a dev build when it takes only
words, calls no other definition of the program, and is at most 80 lines of
IR. `Backend::folded_at_dev` holds the rule. In the PBKDF2 program it folds
`spun`, `choice`, `majority` and `list/repeat`. The release tier is
untouched, because its inliner already decides for itself. Measured here
against main at 64f4c196, under callgrind:

| | main | this branch |
|---|---|---|
| `runbench_dev` | 2,822,197,755 | 2,774,768,832 |
| PBKDF2, 10,000 rounds, dev build | 2,379,121,527 | 1,651,368,753 |
| `codegen_instructions_dev` | 116,687,950 | 116,687,950 |

Three variants were measured and declined.

- Every small leaf, boxed parameters included, bought runbench's dev run
  2.0% and cost `codegen_instructions_dev` 4.5%, which welfare scores below
  the floor. Two leaves that take a `%KValue`, `list/next` and
  `render/to_string`, were the whole of that cost. Unpacking a `%KValue`
  inside a caller is an `extractvalue`, and clang's fast instruction
  selector at `-O0` hands the rest of that block to the slow one.
- A budget on lines times call sites, from 150 to 3,000, changed no row
  once boxed parameters were excluded.
- A 40-line cap misses `spun`, which is about 50 lines, and PBKDF2 keeps
  the whole cost. 80 is the smallest cap measured that reaches it.

On CI, `runbench_dev` falls from 2,822,091,163 to 2,774,662,240. The
compile side pays a little: `emit_instructions` rises to 5,630,728 and
`browser_compile_instructions` to 351,180,151. The tab compiles through the
same dev emitter, so it runs the check, and its translator ignores the
attribute, so its run gains nothing. That reading of the browser row has not
been isolated. `codegen_instructions_dev` and `codegen_instructions_release`
do not move. Welfare rises from 95.07 to 95.08 and is banked.

## 2026-11-07 — a mask bounds an int, and a bounded sum needs no overflow test (#2304)

PBKDF2's rounds spent most of their time on adds that could not overflow.
`h + mixed + fed & whole` masks to 32 bits, but inference knew each operand
only as an int, so every add carried an overflow branch to a bignum and the
result stayed boxed as int-or-bignum.

Inference now carries five width bits beside `INT`. Each says a word int in
the set may lie outside `[0, 2^k)`, for k of 32, 40, 48, 56 and 62. They say
what may be, like every other bit, so the join stays a union and an int
bounded on one path and not another is unbounded. `INT` is the word with all
five, so everything that produced an int before still produces an unbounded
one. Three rules remove bits:

- a literal carries the bits its value needs;
- `x & y` is no wider than its narrower operand, since a non-negative mask
  bounds the answer whatever the other side held, a negative word included;
  `|`, `^` and `bits/shr` are no wider than their wider operand;
- `a + b` of two words below 2^62 crosses only the tightest bound its
  operands had, so it moves up one bucket and cannot leave the word.

A bound that can climb has to be given up, or the fixpoint climbs with it.
The first build let a counter fed by `n + 1` widen one bucket a round, and
the module sample took ten rounds where it took four. Parameters, returns,
record fields and yields now join through `infer::joined`. A word arriving
with a bound above 2^32 is a sum, since masks and literals arrive below it,
and it widens at once to an int or a bignum, which is what it read before the
width bits. A word that grows wider than what is held loses its bound. Every
`compile_golden.txt` row reads what it read before. A rule for `a - b`, which
cannot leave the word either, went too: its answer had no bignum yet and the
next sum gave it one, a round later.

Codegen reads the same rules. A sum the rules bound is `add nsw` with no
overflow test and no bignum behind it, in the binop path and in the word
runs that chain arithmetic. Eight checks that asked whether a set was exactly
`INT` now ask whether it holds only a word, whatever its width.

`tests/golden/micro/a_masked_sum_stays_a_word.kso` holds the edges: masked
negative operands, sums at each bound, a sum of two words just under 2^62,
and a quadrupled one that must become a bignum. With the sum rule changed to
keep its operands' bound instead of moving up a bucket, the native engine
printed -4 for 18446744073709551612 while the interpreter printed the
bignum.

Measured here under callgrind, PBKDF2 at 10,000 rounds read 380,173,992
instructions and reads 347,030,678, with the same digest. Compiling costs a
little more: `kanso check` of `compile_corpus` read 24,629,306 and reads
24,664,277, and of `entry_corpus` 88,645,583 and 88,780,655. `sha256/compress`
fell from 212,561,254 to 179,417,940; `sha256/schedule` did not move, since
its adds read list elements, which carry no bound.

Tried and dropped, again: masking the four list reads in the rounds and the
schedule, `(ks[at] & whole) + (w[at] & whole)`. With the inline `&` path now
taking any operand that may be a word, the run read 366,492,605, worse than
without the masks. The inline path stays: it costs nothing where no mask is
written.

On CI, against main after #2374, the run rows fall: `digestbench` from
3,288,947 to 3,108,282 instructions, `runbench` from 915,653,371 to
913,657,054 and `runbench_dev` from 2,822,091,163 to 2,811,021,533; with
#2378's dev fold also on main, `runbench_dev` reads 2,763,752,866. The
compile rows pay for the wider sets and the rules that read them:
`compile_instructions` rises to 24,152,651, `entry_instructions` to
156,082,962 and `library_instructions` to 73,608,656. On the tree merged with
#2378, `emit_instructions` rises from 5,630,728 to 5,642,485 and
`browser_compile_instructions` from 351,180,151 to 352,264,415. The emitted
code grows where the inline bitwise path now takes an operand that may be a
word behind a tag test: `emitted_other_branches` reads 19,444 and
`emitted_other_lines` 182,342. The machine code shrinks by 832 bytes.
Welfare rises from 95.07 to 95.08 and is banked.

## 2026-11-08 — patterns and operators move out of emit.rs (#2377)

`src/codegen/emit.rs` had grown to 3,055 lines in one `impl Backend` block.
Two runs of methods in it shared nothing with the rest, so they moved to files
of their own, read with `include!` right after emit.rs as calls.rs already
is:

- `patterns.rs`, 444 lines: matching a pattern against a parameter, the
  `%parsed` returns and constructions, packed constructor arguments and the
  type-set checks.
- `operators.rs`, 471 lines: word runs, the general arithmetic they fall back
  to, `emit_binop` and the builtin operators.

emit.rs keeps bodies, binds, conditions and indexing, at 2,153 lines. The move
changes no text: the dev and release IR of `bench/runbench` are byte-identical
before and after. Five ratchet mutations patched lines that moved, and each
now names the file its line lives in. `tests/native_layout_on_wasm32.rs`
reads the two new files when it looks for the lines the wasm32 table
rewrites.
