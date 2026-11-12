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

## 2026-11-10 — a write that would wait yields its fiber

Fixes #2407.

**What was wrong.** `net/write` wrote to a blocking socket and came back
when every byte was taken. A fiber waiting to read or to accept yields to
the scheduler (#2208, #2405), and a write did not. While one client's
socket was full, no other fiber ran, on either engine. The issue was read
from the code and had no case. The case turned out worse than a stall: a
program that is both ends of a connection and writes more than the
sockets between them hold never ends, because the fiber that would read
is one of the fibers the write holds.

**What changed.** Inside the scheduler a write asks the connection for
what it will take without waiting. When it will take no more, the fiber
is left holding the write and the count of bytes gone, and goes back in
the queue. The scheduler's wait on sockets (`k_sched_await`, `awaited`)
asks about room for those fibers, where it asks about arrival for a
reader. The fiber picks up at the byte it stopped on, so no byte is
copied to remember the rest. Natively the part-done write is one more
description (42: the write, and how much has gone); the interpreter's is
`Desc::Sending`.

Outside the scheduler nothing changes. A write at the top of a program
has no other fiber to yield to and waits where it stands.

**TLS.** A TLS session writes whole records and has no call for "as much
as fits". A secured connection is asked only when its socket says it has
room, and is handed at most one record, 16,384 bytes, each time. That
write can still wait briefly if the room is less than a record. It is the
same on both engines.

**The specs.** `tests/golden/micro/a_writer_waits_for_its_reader` is the
program that is both ends: twelve megabytes written to a reader in the
same group. It never ended on either engine and now prints the count.
`tests/a_slow_reader_holds_only_its_own_fiber.rs` is the issue's case
with clients outside the program: one asks for twelve megabytes and
reads none of it, and a second is answered while the first still reads
nothing. With the yield taken out of both engines the second client's
read timed out natively, where the test stops, and the micro program was
killed at twenty seconds on each engine.

**The scheduler's loop.** The first build put the write into both
switches and cost programs that open no socket. Each cost had one cause,
and each was found by a row that had no reason to move.
- deepbench rose 1,867,728. 1.2 million of that was in `k_exec`, whose
  entry had gained one `mov`: a changed switch made clang keep the
  function's argument in a second register. `k_exec` is as it was. A write outside the scheduler has no
  other fiber to yield to, and a part-done write never reaches it,
  because only the scheduler steps a fiber it holds. The write's step is
  its own function, out of line.
- Every program rose 443. `send` with `MSG_DONTWAIT` says "what you can
  take now" in one call, and was one more name for the loader to find at
  every program's start. The write sets the socket not to wait with
  `fcntl`, writes, and sets it back; both names were already there.
- With those gone, runbench fell 383,692 at the release tier and rose
  415,722 at the dev tier, all of the rise in `k_schedule`. The wait on
  sockets was inlined into the scheduler's loop, and its body decided
  how the loop kept its own registers. Out of line and called with the
  addresses of the loop's fiber table and clock, it was better at the
  release tier (a fall of 800,791) and worse at the dev tier (a rise of
  767,174), because a variable whose address is taken lives in memory.
  Called with copies of the two, made only on the path that waits, it
  is better at both. Three builds of the same source that differ only
  in where the wait sits gave those three pairs.

**Rows.** The work rows are counts from this machine set by their
distance to main's reading here. Clang's rows and the tab's are CI's.
- `work_runbench` falls 1,216,098 to 952,666,853 and `work_runbench_dev`
  falls 320,187 to 2,805,647,188, for the wait leaving the loop.
- Three programs rise a little: `work_encodebench` 616 to 2,335,341,459,
  `work_livebench` 301 to 938,654,920 and `work_oneshot` 98 to
  9,979,851. The other ten are unmoved.
- The interpreter's step has one more arm to pass: `interp_instructions`
  rises 7,995 to 680,822,127. Its send is out of line and cold, as its
  wait on sockets already was.
- The write's step and the wait as a function are 928 bytes of machine
  code in every program, and the fourteen text rows sum to `text`
  5,919,865.
- Clang's rows, from CI: `codegen_instructions_release` falls 109,950 to
  372,070,546 and `codegen_instructions_dev` rises 5,675 to 116,944,920.
- The tab's run of the compiled corpus, from CI, rises 40,000 to
  `browser_run_instructions` 91,172,686. Its other three rows hold. I
  did not look for where the 40,000 are.

## 2026-11-10 — a lazy binding under two holders keeps its slots in tenure

Fixes #2415.

**What was wrong.** Natively, a lazy binding held by two values could be
read after its argument had been written over. The interpreter printed
the program's answer and the native build ran out of stack, on main and
on a compiler from 2026-11-07.

A cell is malloc'd and keeps its identity, so the copy walk copies its
slots (its arguments, and its result once forced) and writes them back in
place. The copy map then says the cell was seen, and a second meeting in
the same walk returned at once. An ordinary node met first from the carry
pair and again under tenure is copied a second time, into tenure, because
the tenured parent is pruned at by the next walk and the pair is written
over the stage after. The cell had no such second copy. Its slots stayed
in the pair.

`k_slots_survive` refuses to share a node that holds a cell directly, so
a tenured list of cells is walked again at every stage. It asks only about
the immediate interior. A tenured `[[c]]` has an interior of ordinary
survivors, and every later walk pruned there without reaching the cell.

**How it was found.** Read from the code while the registry was built
(#2408), and filed as a hazard without a program. Three programs later
it had one. The first two never tenured anything. The third took the
shape of a fixture whose counters show tenure, and still answered right.
A print on the cell arm showed the sequence happening in it all the
same: first meeting from the pair, second under tenure, and no walk
after. The answer had stayed right because later laps wrote less into
the pair than the lap that planted the cell. Carrying a list one string
longer each lap wrote past the argument, and the program died.

**The fix.** The walk keeps the slots of each cell it meets from the
pair, as it found them (`k_cellsave`, emptied when the copy map moves
on). A second meeting under tenure copies them again from that record,
into tenure, and the map then says the cell is settled. The originals
are needed because the first meeting wrote over the cell's own slots,
which an ordinary node's copy never does.

**The specs.** `tests/golden/mem/a_lazy_binding_under_two_holders_outlives_the_first`
is the program: one cell under `[c]` and `[[c]]`, the first dropped a lap
later, nineteen more laps each carrying a longer list. Its counters pin
that the chain staged and tenured (`ten_blocks=1`, `evac_allocs=1806`),
so a change that stops it reaching the path moves a number.
`tests/golden/micro/a_lazy_binding_under_tenure_is_read_twice` reads the
binding at lap 10 as well, when only the tenured holder has it, and
again at the end. Both were red on main natively.

**Rows.** Counts from this machine set by their distance to main's
reading here; `work_runbench`, clang's rows and the tab's are CI's.
- A walk pays about three instructions for each cell it meets from the
  pair. `work_runbench` rises 47,849 to 952,714,702 and
  `work_runbench_dev` 47,787 to 2,805,694,975 for the run program's
  16,050 cells. `work_livebench` rises 459 to 938,655,379,
  `work_pendbench` 289 to 154,094,164 and `work_scanbench` 2 to 267,927.
- Seven fall, by layout: `work_encodebench` 1,266 to 2,335,340,193,
  `work_deepbench` 1,098 to 198,293,720, `work_oneshot` 500 to
  9,979,351, `work_widebench` 23, `work_digestbench` 18, and
  `work_jsonbench` and `work_readbench` 3 each.
- The record is 496 bytes of machine code in every program, and the
  fourteen text rows sum to `text` 5,926,809.
- Clang compiles the record: `codegen_instructions_dev` rises 7,036 to
  116,951,956 and `codegen_instructions_release` 72,166 to 372,142,712.
- The tab runs the same runtime, and `browser_run_instructions` rises
  893 to 91,173,579.

**The floor.** Welfare reads 95.09928563621176 with these rows, under the
95.0994448300099 that #2414 banked. The objective's header sets a fix
that makes the engines agree outside the sum, so the floor moves to what
the fix costs and its history says so.

## 2026-11-10 — gavel: senro's page data and component state travel in a binary encoding

senro's design note said "JSON first; a binary encoding only if a
measurement says the JSON is the cost." Clay, overriding it: "of course it's
an extra cost. you KNOW it's a bottleneck."

Page data (a `senro-view` answer) and component state (the state a page
holds and the state posted to `/senro/act/<component>`) travel in an encoding
kata derives from the template's shape. It is positional: no field names on
the wire and no schema file. Both ends are compiled from the same templates,
so the shape is known on each, and a page built by one compile refuses data
from another. JSON stays wherever something outside senro reads the data.

Owed, on #2416: a benchmark of page updates and component actions baselined
under JSON first, as the 2026-09-05 corpus-first gavel asks; the encoder and
decoder derived for each page and component, on the server and in the
browser module; a differential golden that a page rendered from the encoded
data is byte-identical to the server's rendering; a fuzzer for the codec run
to silence; the guard against data from another compile; and the numbers
before and after.

## 2026-11-10 — gavel: keyed list matching and compiled updates are recorded, not built

Clay ruled that two ways of making senro's pages update faster are written
into senro's `design/live-views.md` as later work, and that neither is built
yet (#2418).

Keyed list matching: the page script's morph matches children by position,
so a list that gains an item at the top re-patches every element after it.
Matching on a stable key moves the existing elements, with `moveBefore()`
where the browser has it and `insertBefore` elsewhere. Compiled updates:
kata knows which fields each part of a template reads, so it could compile a
template into direct updates of the nodes that show a changed field. A
virtual DOM is declined, and the note says so.

Each is measured before it is built. The issue gives an order for when the
time comes: keyed matching, then the binary encoding, then compiled updates
if a benchmark shows the morph is the cost. The encoding is ruled and owed
on #2416, so cloud reads the order as placing the two later items around it
and builds the encoding now. The note went in with senro#105.

## 2026-11-10 — a built module's wasm instructions can be counted from outside the tests

#2416 owes a benchmark that counts what a page update costs in the browser,
and senro had no way to count it. kanso's own browser rows are counted by
`tests/support/wasm32.rs`, which runs a module against the runtime under
wasmi and counts the instructions executed, the same on every machine. That
host was reachable only from kanso's tests.

`scripts/count_module.rs` is a cargo example that runs it on any module
`kanso build --wasm` wrote:

    cargo run --release --example count_module -- runtime.wasm app.wasm \
        NAME=@file /path=@file

The module's output goes to stdout and `instructions=` and `grown_bytes=`
to stderr. `NAME=@file` is a variable read from a file, since a page event
has newlines in it. `/path=@file` is a file the module finds at that path
with `os/read_bytes`, because a variable cannot carry a zero byte or bytes
that are not utf-8, and the encoding #2416 asks for is binary. It is an
example because wasmi is a dev-dependency, and the `kanso` binary a user
installs does not carry a wasm interpreter. No row moves: nothing in the
binary or the library changed.

The host under it answered every filesystem call with ENOSYS. It now
serves the files it is given: one directory, `/`, and `path_open`,
`fd_seek`, `fd_read` and `fd_close` over them. A path it was not given
answers ENOENT, which reads in the program as `os/file_not_found`, as
native reads it.

`tests/a_built_module_is_counted.rs` builds a program that prints a
variable and the bytes of a file, runs the example's `answered` on it and
reads the count the host gives when called directly. Its first version ran
the example's binary and passed against an example edited to report one
instruction too many: `cargo test --test` does not rebuild examples, so it
read the binary an earlier build had left. A second version built the
example from inside the test, and each of `cargo test` and the nested
`cargo build` then rebuilt what the other had left, three minutes a run.
The spec now compiles the example's source into itself with `#[path]`, so
there is no binary to be stale, and with the edit it fails on 38,157
against 38,156.

The reader of a module's `kanso.link` section moved from
`tests/build_wasm.rs` into the shared support, where the example reads it
too.

A first reading with it, on senro's `spec/runs/pages` views and a list of
fifty posts (6,562 bytes of JSON): the browser module executes 3,991,398
instructions to turn the page's data into HTML. Splitting the event and
decoding the JSON, with nothing rendered, is 1,064,700 of them, and the
split alone is 145,817. Rendering is the larger part.

A second, on a tagged binary encoding written in kanso as a prototype,
field names still on the wire: the same page is 5,044 bytes and decodes in
1,708,928 instructions, where the JSON decodes in 918,883. By kind of
value, binary against JSON: 905 against 1,664 instructions for a small
int, 1,828 against 1,915 for a string of twenty bytes, and 17,729 against
14,203 for a map of five one-letter keys. std/json reads a key with a
builtin and keeps short tokens in a cache, and the prototype reads a key
as any other string and puts it in a map, so the page's two hundred and
fifty keys are where it loses. The scalars favour the binary encoding and
the maps do not, which is the case for the positional layout the ruling
asks for: a record's fields in order, with no key read and no map built.
These are this machine's numbers from a runtime built here, and the
benchmark #2416 owes will pin its own from CI.

## 2026-11-10 — senro's pages travel as bytes kata derives

#2416, built in senro (senro#107 for the benchmark, senro#108 for the
encoding). A page's data and a component's state leave the server in a
binary encoding, and JSON stays where something outside senro reads the
data.

kata knows a page's shape from the fields its template reads, so it writes
a record as those fields in a fixed order and a list of records as a count
and the records, with no name and no tag between them. Only the leaves are
tagged: one byte that says what follows, and that holds the whole of a
small int or the length of a short string. The server gets a packer for
each page. The browser module gets a record type and a reader for each
record a template reads, and draws the page with the template's ordinary
compile; the compile that read fields out of a map is deleted. The first
four bytes are a hash of every page's shape in the module, and bytes from
another build draw nothing.

`bench/pages` is a board of fifty records in a layout with one component.
Its rows are wasm instructions counted by `count_module`, taken from CI
under JSON and again under the encoding:

    row                                   JSON      binary
    page as data, bytes                 15,191       9,636   -36.6%
    component state, bytes                  43          36   -16.3%
    server: records, component drawn   405,267     423,263    +4.4%
    server: page answered as data    1,861,329   1,269,908   -31.8%
    server: page answered as HTML    5,720,720   5,074,507   -11.3%
    browser: module start               13,905      13,764    -1.0%
    browser: page drawn from data    7,092,359   6,302,146   -11.1%
    browser: one component action      172,445     230,189   +33.5%

A page is a third smaller and a third cheaper to write. Drawing it in the
browser falls by a ninth, because drawing the markup is most of a page
update and the encoding does not reach it.

A component action costs more than it did. Its state is held in the page
as base64 and read into a map, both in kanso code, where JSON was read and
written by the runtime. Measured apart on a 33-byte state: spelling it as
base64 costs 26,200 instructions and reading it back 28,000, against 15,600
and 30,700 for `json/encode` and `json/decode` of the same value, and
reading the tagged bytes into a map costs 58,300. Two issues carry what
that found. #2420 is base64 in the runtime. #2421 is what a decoder
written in kanso pays for each value it reads: a returned record costs
607 instructions a value when it is destructured in a parameter with its
int first, 1,013 with its value first, and 1,673 when it is bound to a
name and read by its fields, which is the only way a caller in another
module can take it.

The codec was fuzzed before it was trusted: `wire` and a second writing of
the format in JavaScript each read what the other wrote for 2,000,000
random values with no disagreement. Three breaks were watched red first.
One of them, a 64-byte string written behind the tag for 63, passed 2,000
values because no string was 64 bytes long, and the strings are now aimed
at exact byte lengths. The long run stopped once, at a string beginning
with U+FEFF that the reference's `TextDecoder` dropped, which was the
reference's fault.

Decided on the way, by the committee and recorded on #2416. Floats travel
as their decimal text, since kanso cannot read a float's bits and no page
in the examples sends one. Component state stays tagged: writing it
positionally needs a component to declare its state's type, and that waits
on what #2420 and #2421 leave to win. Only a program that names `rendered`
gets the readers and row types, so a server still refuses a misspelled
field at the template's line.

Nothing in kanso changed, so no row here moves. The site's views guide
describes the bytes, and its senro pin moves to 43a2ca5.

## 2026-11-10 — a session's trip through base64 joins the corpus

The corpus half of #2420, ahead of its fix. senro's binary encoding (#2416)
holds a component's state in the page as base64, and its benchmark showed
an action costing more than it had in JSON, with base64 the larger part.
senro also spells and reads base64 for every request that carries a
session. `std/base64` is written in kanso: it joins a string for each group
of three bytes, reads each letter out of its alphabet with a slice, and
builds the bytes it decodes by pushing onto a list. No benchmark held a
trip through it, so a fix would have scored as nothing.

`sealed` joins the run program: 800 passes, each spelling a cookie of about
sixty bytes and reading it back. Built with the phase and with it reading
nothing, the program retires 991,852,481 instructions against 952,721,301
in this container, so the phase is 39,131,180 of them, 3.95%, and a trip
costs about 49,000. The same trip joins the interpreter's corpus at 15
passes, where it costs 3.1 million instructions a trip.

CI's rows. work_runbench 952,714,702 -> 991,852,372 and work_runbench_dev
2,805,694,975 -> 2,894,015,349. interp_instructions 680,822,127 ->
727,482,475, interp_allocs 984,852 -> 1,043,722 and interp_peak_bytes
1,377,499 -> 1,489,302. In the tab, browser_compile_instructions
358,139,032 -> 413,857,607, browser_compile_peak_bytes 862,043 -> 975,975
and browser_run_instructions 91,173,579 -> 95,460,118, with
browser_run_peak_bytes still 1,572,864. The tab's compile rises by 15.6%:
the corpus now imports `std/base64` and through it `std/bits`, and the tab
compiles both. The compile rows that read other programs did not move.

The run program's counters rise with the phase: run_allocs 2,372,861,
run_alloc_bytes 261,860,897, run_perm_allocs 161, run_beat_iters 197,248,
run_push_mut_slow 1,658,292 (and run_push_mut_fast 1,398,651),
run_bytes_malloc 9,468, run_str_scans 52,203, run_str_scan_bytes
19,043,424, run_sh_str 19,617,504, run_sh_buf 102,043,040 and run_sh_bytes
43,560, with run_buf_reuse at 7,390. `std/base64` joins the program's
emitted code, so runbench reads defines=975 calls=10294 branches=9407
lines=80567 and the summed rows emitted_other_defines 2,406,
emitted_other_calls 22,268, emitted_other_branches 19,938 and
emitted_other_lines 185,688. Its machine code is 912,641 bytes, and the
summed text 5,954,425.

The floor is re-based and the score is not a gain. Seven baselines are
scaled by the new reading over the old, as the last corpus additions did,
so the meta moves only by their rounding, 95.09928563621176 to
95.09928566956496.

## 2026-11-10 — base64 is spelled and read by the runtime

#2420. `std/base64` was written in kanso, and a trip through it for a
cookie of about sixty bytes cost 49,000 instructions natively and 3.1
million interpreted. The runtime now does both halves: `k_base64_spell` and
`k_base64_read` in C for a built program, and `kanso::base64` in Rust for
the interpreter. The library is three wrappers and the two reasons
`decode!` gives.

What the library answers is unchanged. `decode` answers a list of byte
values or none, and `decode!` the same two reasons. Reading stays as
lenient as the kanso was, which is more lenient than its own comment said:
an `=` in a group's third place ends the group after one byte whatever the
fourth holds, a padded group may be followed by more groups, and the bits
a padded group leaves over need not be zero. One thing changes. `encode`
given a list holding a number that is not a byte spelled garbage, and now
stops the program with "base64/encode takes bytes", as `text/bytes` stops
for a value that is not a string.

The committee weighed writing it better in kanso instead, which is what
`std/sha256` got. senro's `wire` already had that version: one byte
builder, with each letter computed. For a 33-byte state there and back it
costs 54,200 wasm instructions, about a third of what the string-joining
library cost in the same place, and still about 800 a byte. What kanso
code pays for each value it reads is #2421's, and is worth taking out in
the compiler for every decoder. Base64 is a byte transcoding with one
right answer, the kind of kernel `std/json` already keeps in the runtime
for its escapes, and a server runs it on every request that carries a
session. So the kernels go in the runtime.

The harness came first. `tests/base64_agrees_with_its_reference.rs` holds
all four kernels to a reference written apart from them, which spells by
laying the bytes out as bits and taking six at a time: 2,000,000 spellings
and 2,000,000 readings for the interpreter's, and 400,000 of each for the
runtime's, whose text is lifted out of the runtime and given each case in
a buffer that ends at a page nothing may touch, in exactly the room the
runtime promises. The first writing used the address sanitizer, which CI's
clang cannot link. The texts
are true spellings, spellings with characters swapped for letters, pads
and the characters either side of each run of the alphabet, spellings cut
short or run long, and ragged strings from that pool. It was watched red
three ways: a wrong shift in the interpreter's second byte with a five-bit
mask on the runtime's third letter, both caught within three cases; the
runtime refusing a padded third place with the interpreter taking ragged
text, caught at the first case; and the runtime taking ragged text, which
stopped at a read past the end. With the fence in place of the sanitizer
it was watched red again: ragged text taken and a terminator written past
the speller's room each stop the harness with a fault, and a five-bit mask
on the third letter parts from the reference at byte 18.

The old library was the second oracle. A program holding it as a module of
its own compared it with the new one on 300,000 generated cases, bytes
spelled and texts read, and found no disagreement. Broken two ways, the
old copy disagreed on 98,002 and 95,029 of them.
`micro/base64_spells_and_reads_on_every_engine` pins RFC 4648's vectors
both ways, a list in place of bytes, and each edge of what reading takes,
and `runtime/base64_encode_takes_bytes` pins the refusal on both engines.

A trip now costs about 2,800 instructions natively, most of it building
the cookie and the list of sixty byte values `decode` answers. The run
program's work falls from 991,852,372 instructions to 954,999,153 and its
dev-tier build's from 2,894,015,349 to 2,808,131,450. The interpreter's
corpus falls from 727,482,475 instructions to 681,004,692, with
interp_allocs 1,043,722 -> 988,431 and interp_peak_bytes 1,489,302 ->
1,400,327. The tab compiles that corpus in 366,327,369 instructions where
it took 413,857,607, the library's kanso and its import of `std/bits`
being gone, with browser_compile_peak_bytes 975,975 -> 867,315, and runs
it in 91,307,704 where it took 95,460,118.

The run program's counters fall back toward what they were before the
phase joined: run_allocs 2,349,661, run_alloc_bytes 256,551,969,
run_perm_allocs 135, run_push_mut_slow 1,650,292, run_bytes_malloc 7,868,
run_str_scans 36,202, run_str_scan_bytes 19,043,360, run_sh_str
19,156,704, run_sh_buf 97,305,312 and run_sh_bytes 24,360, and
`base64_calls=1600` is the kernels' presence, printed only by a run that
calls them. Two counters fall that the trend gate reads as worse,
run_push_mut_fast to 1,238,867 and run_buf_reuse to 3,390. Both are
exactly what they read before the phase joined, so what the phase added to
them was the kanso library's pushes and buffer reuses, and those are gone
with it. runbench's emitted code loses the library's kanso: defines=959
calls=9986 branches=8989 lines=77949, and summed emitted_other_defines
2,390, emitted_other_calls 21,960, emitted_other_branches 19,520 and
emitted_other_lines 183,070.

Several rows rise. The runtime is linked whole, so every program's machine
code gains the kernels and their wrappers, 2,032 bytes each: runbench
falls to 889,489 with the library's kanso gone, the other thirteen rise,
and the summed text goes 5,954,425 -> 5,957,689. And the compiler knows two
more builtins: compile_instructions 24,152,633 -> 24,154,945,
entry_instructions 156,082,814 -> 156,095,335, library_instructions
73,608,570 -> 73,612,703 and emit_instructions 5,643,537 -> 5,644,746.

Clang and the linker do more for the dev tier and less for the release
tier: codegen_instructions_dev 116,951,956 -> 116,980,709, a rise of
28,753, and codegen_instructions_release 372,142,712 -> 372,052,045, a
fall of 90,667. A build now has less of runbench to compile and more
runtime to link; which of the two decides each tier was not isolated.
Three programs that never call base64 link the grown runtime and move by
layout: work_encodebench rises 350 to 2,335,340,543, work_oneshot rises
154 to 9,979,505 and work_livebench falls 644 to 938,654,735.

`runtime/base64_decode_takes_a_string` pins the other refusal, a value
that is not a string handed to `decode`, on both engines. Welfare rises
from 95.09928567 to 95.25752069, and the floor is banked there.

## 2026-11-10 — a decoder read across a module joins the corpus

#2421, corpus first. A decoder written in kanso answers each value with
the position after it, as a record of two fields, and no benchmark built
one. `framed` is such a decoder, a stream of numbers at one byte or two,
and `taken` reads it from another module. A record cannot be taken apart
across an import, so `taken` binds what `framed/at` answers to a name and
reads its two fields, which is how the readers senro derives call `wire`.
Each pass also has `framed` sum the stream itself, with the record taken
apart in a parameter, value first as its type declares it. The run program
makes 1,500 passes over 64 numbers, and the interpreter's corpus carries
the same two loops in its one module for 12.

The baseline already says something about the leads. Both loops build
every record on the heap: run_sh_rec goes from 4,723,232 to 17,011,296,
which is 1,500 passes of 128 reads at 64 bytes and one record more. So the
loop that takes the record apart in a parameter misses the by-value path
too, on the order of its fields.

The run program's work rises: work_runbench 954,999,153 -> 997,844,059
and work_runbench_dev 2,808,131,450 -> 2,924,577,667. Both are this
container's readings less 109, which is what every work row here reads
above CI's. The phase is 42,844,906 of the program, 4.29%. Its other
counters: run_allocs 2,541,664, run_alloc_bytes 268,840,291,
run_perm_allocs 136, run_beat_iters 390,812, run_evac_allocs 5,617,
run_evac_bytes 4,721,376, run_append_fast 5,329,378, run_append_grow
1,081, run_bytes_malloc 7,869 and run_sh_bytes 24,384. runbench's emitted
code reads defines=978 calls=10213 branches=9202 lines=79617, the summed
rows emitted_other_defines 2,409, emitted_other_calls 22,187,
emitted_other_branches 19,733 and emitted_other_lines 184,738, and its
machine code is 900,801 bytes, so the summed text is 5,969,001.

The interpreter's corpus rises: interp_instructions 681,004,692 ->
711,444,159, with interp_allocs 1,026,142 and interp_peak_bytes 1,487,351.
The tab's rows rise with it: browser_compile_instructions 366,327,369 ->
390,565,528, browser_run_instructions 91,307,704 -> 93,856,364 and
browser_compile_peak_bytes 867,315 -> 958,848. The first sizing was 40 passes,
which cost the interpreter 95 million instructions, about 18,500 for each
value read. That looked like a cliff and is not one: building the stream
costs it about 7,000 instructions a number at 64, 256 and 1,024 numbers
alike, the bare loop about 5,200 a value, the bound record 16,900 and the
record taken apart 12,500. The phase was sized down to 12.

The floor is re-based. The baselines of the seven rows that read the two
programs are scaled by what each rose, and welfare stays where it was,
95.25752068 before and 95.25752070 after.

## 2026-11-10 — a decoder's answer stays in registers

#2421, and #2428 on the way. The by-value return packs one int beside one
value into three words, and it existed for the json scanner's `_parsed`,
whose int comes first and whose callers take it apart in a parameter. A
decoder a library exports is written the other way round in four places,
and each kept its record on the heap.

The int had to be the first field. `read value next` declares the value
first, as a type that says what was read and where to go on would. The
escape analysis now asks inference which of the two fields is an int and
nothing else, the emitter builds and takes apart in that order, and the
runtime has `k_parsed_box_back`, `k_parsed_words_back` and
`k_parsed_int_back` for the second order. The three words are laid out the
same either way.

A guard's early return was not a tail. `return read b p if b < 128`
refused the type, because only a tail `if` and a tail construction were
recognised. A guard in tail position is read as the tail `if` it is.

A constant of the type, named bare in a tail, was not counted as answering
it, so a decoder that answers `lost` for a short read was said to answer
something else. It counts now. Returned from a by-value function, a kept
constant is a built record, and the return took any value it was handed
for a failure and sent its tag and pointer back as words; the constant's
words are read off it there.

A record bound to a name was built on the heap again for every field read
off it, since a getter is handed a record. `r.next` on a name that holds a
record by value reads the field out of the words, and a failure riding in
them comes back as itself. `.reason`, `.cause` and `.origin` are left to
the getter, which answers them for an err whatever record was expected.
The record was also built once more on its way into a call in any program
with a lazy binding, to ask whether it was a cell to force. It is not
asked.

The loop's carry was the fifth. A loop that rewinds its arena each lap
staged a by-value record built, a heap record a lap, and
`mem/a_failure_crosses_a_beat_carry_in_two_words` said so in its comment.
On the way that skips the rewind, taken when another argument may be a
failure, the record stayed three words, the two ways met in a phi of two
types, and clang refused the module: #2428, a program main cannot build.
The carry stages the record's two fields, a slot each, and packs them on
the far side, by value on both ways. When the fields would not fit the
buffer's eight slots the record is staged built and its words read back.

A record built at the loop's own call was the sixth, #2430, and it also did
not build on main. Written at the call for a by-value slot, a record is
packed into words in place and has no operand among the emitted ones. The
carry staged the emitted operand at every carried position and wrote
`call void @k_carry_stage(%KValue )`. The packed words join the emitted
operands at a carried slot and cross as the two fields. A record built on
the way into the loop from outside it, which is not a tail call, is still
built and read back, once for each entry.

A boxed answer was the seventh, #2432, and on main it is a wrong answer
with no complaint. A name a module declares can also be one of an
import's: `std/list` exports `first`, so a bare `first` in a module that
declares one calls into the module's own overload space, and a group
there stays boxed. A decoder that answers in registers and ends in such a
call is handed a built record. Its return took whatever value it was
handed for a failure, sent the record's tag and pointer on as the value,
and the caller read the whole record where the value belonged and 0 for
the position: main prints `shared/read 2 97 then 0` where the
interpreter prints `97 then 2`. The analysis now keeps the groups that
answer the type and stay boxed, a call to one is noted where it comes
back, and the return reads the record's words off a noted value. Any
other value still goes back as a failure's two words. Reading the words
off every value not known to be a failure was tried first and put 234
calls into the decoder's emitted code, since inference rarely knows.
`micro/a_bare_name_shared_with_an_import_answers_its_record` holds every
engine to the interpreter's two lines.

Measured. `mem/a_record_held_by_value_crosses_a_loop_s_rewind` did not
build and reads sh_rec 0. `mem/a_record_built_at_a_loop_s_call_crosses_its_rewind`
did not build either, and reads the one record its entry builds over 76
laps. `mem/a_record_whose_int_comes_second_goes_back_by_value`
read sh_rec 2,624 and reads 0. `mem/a_failure_crosses_a_beat_carry_in_two_words`
falls from 64,000 to 0, and three scanning fixtures lose a record each.
The run program's decoder phase builds none: run_sh_rec 17,011,296 ->
4,723,168, run_allocs 2,541,664 -> 2,349,662, run_alloc_bytes 268,840,291
-> 256,552,163, run_evac_allocs 5,617 -> 2,617 and run_evac_bytes
4,721,376 -> 4,625,408; scan_allocs 97 -> 96 and scan_sh_rec 1,872 ->
1,808. work_runbench falls 997,844,059 -> 982,946,782 and
work_runbench_dev 2,924,577,667 -> 2,894,927,376, with the phase at about
146 instructions a value where it was 223. The tab runs its corpus in
93,283,820 instructions where it took 93,856,364.

What rises. The runtime's three conversions are linked into every
program, 368 bytes each; scanbench falls 512 and runbench 800, and the
summed text goes 5,969,001 -> 5,972,105. The emitter asks more:
emit_instructions 5,644,746 -> 5,651,651. The tab's compile rises with
it, browser_compile_instructions 390,565,528 -> 393,730,774 and
browser_compile_peak_bytes 958,848 -> 965,297. Clang and the linker read
a little more: codegen_instructions_dev 116,980,709 -> 117,010,179 and
codegen_instructions_release 372,052,045 -> 372,177,641, both CI's
readings. The container's move added to CI's last reading had predicted
67 and 80,559 fewer, so that sum is a guide for these two rows and no
more. Three programs move by
layout with the grown runtime: work_encodebench rises 350 to
2,335,340,893, work_oneshot 854 to 9,980,359 and work_livebench 742 to
938,655,477; work_scanbench falls 84 to 267,843 with its record.
runbench's emitted code reads defines=975 calls=10169 branches=9172
lines=79632 and scanbench's defines=331 calls=3190 branches=3019
lines=26106. Welfare rises 95.25752070 -> 95.29096728, and the floor is
banked there.

Checked against the interpreter. `micro/a_record_held_by_value_reads_like_one_built`
walks a short read, an early answer, a field read off a bound name, the
record handed whole and kept in a list, a string carried round a rewinding
loop, a position too wide for a word and a failure where the record was
expected, on every engine and through the tab's translator. With the
constant's branch taken out it prints the record where its field should
be. The 2,163 programs of the numeric differential and the eight other
differentials agree.

What this does not reach. senro's `wire` is unmoved: its benchmark page
reads the same six rows with this compiler. Three things keep its `read`
on the heap, found by asking the analysis which test refused it. Inference
cannot say the position is an int: a count is read into the same `value`
field as every other value, so the position after a string, the position
of its bytes plus their count, may be a float, and the tag `bs[p]` is
unknown because nothing says `bs` is bytes. `wire` hands its decoder out
as a value, to read a list's items. And it passes the record whole to a
helper that binds it. The analysis is all or nothing for a type, so one
such use anywhere costs every read. The first is senro's to write
differently, with a record of two ints for a count and `:int` where a
position comes in from outside; the other two are the next piece of
#2421 here.

Two ratchet rows moved with the change, and the ratchet's own run found
both. `a thunk site returns a record unboxed` patched one line that is six
since the field order became an argument, so its mutation no longer
applied; it takes the call from its opening to its close now, and clang
refuses `micro/a_lazy_record_call_somebody_makes_still_builds` under it
as before. `a failure read as a record on the far side of a carry` had
gone blind. Its mutation takes out the ask `k_parsed_words` makes before
it reads a record's fields, and its fixture reached that ask through the
carry, which unpacks no built record now that it stages the two fields.
The ask is still needed wherever a built record is unpacked, and a
failure a boxed group hands back to a by-value return is one such place:
`mem/a_failure_handed_back_by_a_boxed_group_stays_a_failure` prints
`stopped: end of input` with the ask and stops on a runtime error
without it.

A third row ended CI's runner, twice. `a group answering other values by
value` skips the check that every arm of a group answers the record or a
failure. On main `micro/a_greedy_run_of_one_character_backs_off_in_order`
still prints its golden under that mutation. Here more groups are
candidates, regexp's among them, and the same fixture builds into a binary
that allocates without end. Nothing bounded a program a golden
runs, so the kernel ended it once the machine was out of memory, and on a
runner that ends the job with a shutdown signal and a log that cannot be
read. `run_kanso_env` in tests/golden.rs now hands each program 8 GiB of
address space on Linux, where the ratchet runs. Under the bound the binary
is refused an allocation and says `out of memory`, and the row reads red
in 24 seconds. `a_program_a_golden_runs_may_map_only_so_much` reads the
limit from a shell the program starts, on both engines, and reads
`unlimited` without the bound. The whole golden suite passes under it. A
test that starts kanso some other way has no bound yet (#2436). The spec's
first draft read `/proc/self/limits` and found that the native runtime
reads such a file as empty (#2435).

## 2026-11-11 — a decoder handed out joins the corpus

#2421 again, corpus first. The phase that reads a stream through an
exported decoder took the record off the heap in #2431 for the two ways it
was read: bound to a name across a module, and taken apart in a parameter.
senro's `wire` reads a third way, and it is the one that keeps its record
on the heap. A list's reader is handed the function that reads one item:
`wire` hands `at` to its own list reader, and kata hands that reader the
function it wrote for a record. No benchmark handed a decoder out.

`framed/each` is that reader. It is handed `at` and reads the stream
through it, and `taken` adds it to each pass. `at` says its position is a
whole number, `p:int`, because a function handed out can be called with
anything and the record goes by value only when its position is an int.
The interpreter's corpus carries the same loop as `through`.

The baseline shows what handing a decoder out costs. A function handed out
is called through a wrapper that answers one boxed word, so the analysis
keeps the type on the heap for every caller the decoder has. Handing `at`
out once puts all three readers back on the heap: run_sh_rec 4,723,168 ->
23,155,168, which is 1,500 passes of 192 reads at 64 bytes more, the 64
through the value and the 128 direct reads #2431 had freed. run_allocs
2,349,662 -> 2,637,662, run_alloc_bytes 256,552,163 -> 274,984,163,
run_beat_iters 390,812 -> 582,812, run_evac_allocs 2,617 -> 5,617 and
run_evac_bytes 4,625,408 -> 4,721,408.

The run program's work rises: work_runbench 982,946,782 -> 1,035,809,998
and work_runbench_dev 2,894,927,376 -> 3,028,058,391, both this
container's readings less 109. The reader is 52,863,216 of the program,
5.10%. runbench's emitted code reads defines=982 calls=10293
branches=9278 lines=80296, the summed rows emitted_other_defines 2,411,
emitted_other_calls 22,245, emitted_other_branches 19,794 and
emitted_other_lines 185,367, and its machine code is 905,969 bytes, so
the summed text is 5,978,073.

The interpreter's corpus rises: interp_instructions 711,444,159 ->
724,216,921, with interp_allocs 1,038,038 and interp_peak_bytes 1,496,663.
The tab's rows rise with it: browser_compile_instructions 393,730,774 ->
398,181,377, browser_compile_peak_bytes 965,297 -> 968,782 and
browser_run_instructions 93,283,820 -> 95,675,366.

The floor is re-based. The baselines of the seven rows that read the two
programs are scaled by what each rose, and welfare stays where it was,
95.29096728 before and 95.29096738 after.

## 2026-11-11 — a decoder handed out keeps its callers in registers

#2421, the piece senro's `wire` was waiting on. A group that answers a
record in registers could not be handed out as a value. A function value
is called through a wrapper that takes and answers boxed words, and the
wrapper knew one return shape, so the analysis refused the type outright
when any group answering it was named as a value anywhere in the program.
One reader handed the decoder, and every caller it had went back to
building a record.

The wrapper now calls the group the way a direct caller does and builds
the record from the three words, with `k_parsed_box` or its second-field
twin, and a failure riding in the words comes back as itself. Whoever
calls the value reads one boxed word as before. The analysis no longer
asks whether a group is handed out, and the walk that collected every
name used as a value is gone.

The ratchet row for this, `a function value returned in registers`, broke
the check the analysis no longer makes, so its mutation would not apply.
It now breaks the wrapper: called as though every group answered a boxed
word, `micro/a_decoder_handed_out_answers_like_one_called` prints nothing
natively and the row reads red.

What a handed-out decoder still needs is its position said: `at bs p:int`.
A function handed out can be called with anything, so inference gives its
parameters every type, the position worked out from one may be a float as
far as it can tell, and a record goes by value only when its position is
an int. The corpus and both fixtures write the annotation.

Tried and taken back: letting a record be passed whole to a slot that
binds it, boxed at the call. `wire` does that in two places and the rule
that refuses it is syntactic, since binding the record to a name first is
allowed. It was written, and `mem/a_carried_list_frees_its_buffer` went
from 22 allocations to 1,218 and from sh_rec 512 to 77,056. Its `bag` is
rebuilt in place by a loop that passes it whole, the relaxation let `bag`
go by value, and every lap then built a fresh record at the call where
none had been built at all. A type passed whole on its hot path is better
left on the heap, and the old rule keeps it there. senro binds its record
first in those two places.

Measured on the corpus. `mem/a_decoder_handed_out_still_answers_its_callers_by_value`
reads sh_rec 512, the eight reads that go through the value, where it
reads 3,200 with the refusal put back. The run program's direct reads
come off the heap again: run_sh_rec 23,155,168 -> 10,867,168, which
leaves the 64 reads a pass that go through the value, run_allocs
2,637,662 -> 2,445,662, run_alloc_bytes 274,984,163 -> 262,696,163,
run_evac_allocs 5,617 -> 2,617 and run_evac_bytes 4,721,408 -> 4,625,408.
work_runbench falls 1,035,809,998 -> 1,020,737,307 and work_runbench_dev
3,028,058,391 -> 2,997,794,416, both this container's readings less 109.
The tab runs its corpus in 95,097,638 instructions where it took
95,675,366.

What else moves. The emitter does less with the walk gone:
emit_instructions 5,651,651 -> 5,632,306. The tab's compile rises, since
it now emits the by-value paths the refusal used to skip:
browser_compile_instructions 398,181,377 -> 399,455,088 and
browser_compile_peak_bytes 968,782 -> 969,319. interp_instructions reads
724,216,927, six more. runbench's emitted code reads defines=981
calls=10271 branches=9262 lines=80364, the summed rows
emitted_other_defines 2,410, emitted_other_calls 22,223,
emitted_other_branches 19,778 and emitted_other_lines 185,435, and its
machine code is 906,017 bytes, so the summed text is 5,978,121. Welfare
rises 95.29096738 -> 95.32525328, and the floor is banked there.

Checked against the interpreter. `micro/a_decoder_handed_out_answers_like_one_called`
reaches a decoder by name, through a parameter, through a partial, out of
a list and from a function that answers one of three, with a short read,
a position too wide for a word and a failure where the record was
expected. With the wrapper's boxing taken out it stops at a field read of
a number.

senro, in a container with this compiler and `wire` written for it: a
record of two ints for a count, `:int` where a position comes in, and the
record bound before it is handed on. Its benchmark page draws in
3,845,447 wasm instructions where it took 4,254,682, and an action runs
in 115,251 where it took 125,306. Those rows are senro's to pin from its
own CI once it takes this compiler.

## 2026-11-11 — a page's values through an escape join the corpus

#2427, corpus first. kanso has no call that replaces one text with
another. The idiom is `text/join (text/split s from) to`, a list of the
pieces and then a string from the list, and the escape senro's templates
lean on runs it once for each character it rewrites. No benchmark replaced
text, so nothing priced the idiom or would have scored a call that did the
work in one pass.

`replaced` is that escape as senro writes it: three searches for the five
characters, a value with none handed back as it came, and a split and a
join for each search that found something. A row is the mix a listing page
has, which is a title holding four of the five characters, an excerpt
holding an apostrophe, and seven values with nothing to rewrite. The run
program escapes 4,000 rows and the interpreter's corpus 80.

The baseline. The phase is 47,129,776 instructions of the run program,
4.41%, measured as the whole with it less the whole without in this
container. It runs the split and the join 28,000 times, and 20,000 of
those find something to rewrite. run_allocs 2,445,662 ->
2,709,581, which is 66 allocations a row, run_alloc_bytes 262,696,163 ->
274,019,555, run_beat_iters 582,812 -> 614,812, run_find2_calls 1,060,857
-> 1,168,857, run_str_scans 36,202 -> 72,202 and run_sh_str 19,156,704 ->
25,232,528. The rest of the run's rows the phase moves: run_perm_allocs
136 -> 151, run_sh_buf 97,305,312 -> 98,585,312, run_sh_bytes 24,384 ->
888,384 and run_str_scan_bytes 19,043,360 -> 21,093,397.

The run program's work rises: work_runbench 1,020,737,307 ->
1,067,867,083 and work_runbench_dev 2,997,794,416 -> 3,065,058,505, both
this container's readings less 109. runbench's emitted code reads
defines=992 calls=10401 branches=9373 lines=81321 and its machine code is
911,345 bytes. Summed over every benchmark but the decoder,
emitted_other_defines 2,410 -> 2,421, emitted_other_calls 22,223 -> 22,353,
emitted_other_branches 19,778 -> 19,889 and emitted_other_lines 185,435 ->
186,392, and the summed text 5,978,121 -> 5,983,449.

The interpreter's corpus rises: interp_instructions 724,216,927 ->
744,978,389, interp_allocs 1,038,038 -> 1,074,427 and interp_peak_bytes
1,496,663 -> 1,527,692. The tab's rows rise with it:
browser_compile_instructions 399,455,088 -> 417,017,327,
browser_compile_peak_bytes 969,319 -> 985,620 and browser_run_instructions
95,097,638 -> 100,630,653.

The floor is re-based. The baselines of the seven rows that read the two
programs are scaled by what each rose, and welfare stays where it was,
95.32525328 before and 95.32525334 after.

## 2026-11-11 — one limit for every test and every ratchet gate

#2436, a cleanup. #2431 bounded what a program started by `tests/golden.rs`
may map, after one ratchet row ended CI's runner twice. Every other test
that starts kanso, and every ratchet gate that is a script, still ran with
no limit.

The issue asked for a helper in `tests/support/` that the tests starting
kanso would call. The committee's reading is on the issue: 153 files start
kanso, a helper covers the ones that remember to call it, and it leaves the
ratchet's script gates out. A limit is inherited by a child process, so it
is set once on the way in. `scripts/bounded.sh` sets it. Cargo runs every
test binary through that script on Linux (`runner` in `.cargo/config.toml`),
and the ratchet sources it before each command it runs in a worktree.
`tests/golden.rs` loses its own.

Two measurements changed what #2431 chose.

The kind of limit. #2431 limited address space. Headless Chromium does not
start under that at 8 GiB: Playwright's launch fails. Two ratchet gates run
a browser, so the same limit in the ratchet would have turned them red
before any mutation. Under a limit on data, `ulimit -d`, Chromium starts and
makes a wasm memory, and callgrind runs under either. The limit is on data.

The number. With the mutation that ended the runner applied,
`micro/a_greedy_run_of_one_character_backs_off_in_order` says `out of
memory` after 9.8 seconds and 8,140 MiB resident under 8 GiB, and after 4.4
seconds and 3,461 MiB under 4 GiB. A program that runs away has written to
nearly all of its limit by the time it is refused, so the limit is what one
such program costs the machine, and it should be no larger than the tests
need. A release build of the compiler, Chromium, callgrind, the compile
sweep, the counter sweep and the golden suite were each run under 4 GiB, and
the first push set the limit there. CI then failed one test:
`an_answer_above_two_gibibytes_reads_back` in `tests/wasm_engine.rs` grows a
wasm memory past two gibibytes inside the test process, and the engine
could not grow it. Run alone it fails under 4 GiB and passes under 5, 6 and
8. The limit is 6 GiB. The runaway fixture was not run again at 6. A second
program, one that pushes onto a list without end, was run under 1 GiB on
both engines: the native binary says `out of memory` and exits 1, and the
interpreter panics and exits 101.

It is the soft limit, so a spec can lift it. `tests/golden.rs` reads the
limit from a shell a program starts, on both engines, and reads `unlimited`
with the `runner` line removed. `tests/the_ratchet_bounds_what_it_starts.rs`
stages one row whose gate is green only under the limit and runs the real
`prove` over it, started with the limit lifted. With the ratchet's line
removed the baseline reads `ALREADY RED`, and the gate's own words are
`limit unlimited`. Neither spec has a ratchet row: the ratchet bounds the
gate that would read the first, and the second runs `prove`.

No golden moves. Left open as #2441: the steps in `ci.yml` that run a gate
script directly, the cost-goldens job among them, go through neither cargo
nor the ratchet and have no limit.

## 2026-11-11 — a step in CI starts under the limit too

#2441, the loose end of the entry "one limit for every test and every
ratchet gate". Cargo's runner bounds a test binary and the ratchet bounds
a gate it starts, but a `run:` step in a workflow goes through neither:
the cost-goldens job runs its gate scripts directly, and so do the
browser, site and book jobs.

`ci.yml` and `ratchet.yml` now name a default shell. It sources
`scripts/bounded.sh` when the checkout has put it there and then runs the
step's script as `bash -e`, which is what a step that names no shell
gets, so no step changes what it does. The steps before a job's checkout
start as they did.

Two things hold it. In CI, the specs job reads `ulimit -S -d` from inside
a step and compares it with the number `bounded.sh` sets. In the tree,
`tests/every_ci_step_starts_under_the_limit.rs` reads both workflows for
the default and refuses a step that names a shell of its own, which
would start without the limit. The ratchet row is `step_unbounded`: with
the default taken out of `ci.yml` the spec fails and names the workflow.

Whether everything a step starts fits under 6 GiB is CI's to say, as it
was for the tests: the release build with fat LTO, the wasm build and
clang at `-O3 -flto` had only been run under the limit by the ratchet.
`pages.yml` and `release.yml` are left as they are. They run on main and
on a tag, after CI has built the same tree under the limit.

No counter moved.

## 2026-11-11 — one text written as another, in one pass

#2427's first lead, on the corpus the entry before this one added.
`text/replace s from to` joins `std/text`. The committee's reading is on
the issue: the call answers what `text/join (text/split s from) to`
answers in every case, so a program can swap the idiom for it and nothing
it prints changes. Occurrences are found left to right and the search
takes up again after the end of each. A text that holds none comes back
as it was, with no copy. An empty text to find occurs everywhere and is an
err with its own reason, as an empty separator is for `split`.

The harness came first. `tests/replace_agrees_with_its_reference.rs` holds
both kernels to a reference that walks one byte at a time and asks at
each whether the text to find starts there: 4,000,000 cases against the
interpreter's and 1,000,000 against the runtime's, 0 failures. The C is
cut out of the runtime's text and run with each buffer ending at a page
nothing may touch. With the copy made to read one byte too many the
harness stops on that page, and with the interpreter's search made to
resume one byte on instead of after the match it names the first case.

The kernel counts the occurrences with the search `split` uses, allocates
the answer once at its length, and copies. Its presence counter is
`replace_calls`, 28,000 in `bench/cost_golden_run.txt`: the escape asks
seven times a row, and five of those find something.

Where the err is born took two tries. The first wrote the empty case as a
second arm in the library, as `split` does. A program that names the call
then keeps a group of two arms that `inline_builtin_wrappers` cannot
rewrite, and the interpreter's corpus cost 1,670,000 instructions more to
start whether or not it called the group; a guard inside one arm cost the
same. That is #2434, and `split` is written the same way. The builtin
answers the err itself now, as `from_code` does, and the library holds one
forwarding arm. Its wrapper stays a real call so the err is born in
`text/replace`, where a caller may rescue it, and the list of builtins
that can give birth to an err has one home, `inline::BIRTHS_ERR`, which
the emitter reads to hand each its origin.

Measured on the corpus. The phase is 39,101,174 instructions of the run
program where it was 47,129,776, in this container. `k_b_split` falls
5,994,654 -> 22,654 and `k_b_join` 18,580,590 -> 14,124,590, and
`k_b_replace` takes 4,228,000, 151 a call. run_allocs 2,709,581 ->
2,573,581, 34 fewer a row, run_alloc_bytes 274,019,555 -> 269,477,283,
run_sh_str 25,232,528 -> 24,785,824 and run_sh_buf 98,585,312 ->
97,305,312. work_runbench 1,067,867,083 -> 1,059,838,481 and
work_runbench_dev 3,065,058,505 -> 3,057,251,731, both this container's
readings less 109. The tab compiles its corpus in 416,189,723
instructions where it took 417,017,327 and runs it in 100,428,884 where
it took 100,630,653, with browser_compile_peak_bytes 985,620 -> 984,597.

interp_instructions rises, 744,978,389 -> 745,491,463, on a collision in
the interpreter's table of callees. The same compiler on the corpus as it
was, splitting and joining, reads 744,767,711. Against that the call costs 723,752 more,
which is 1,915,016 more in `Interp::callee_missed` and 1,191,264 less
everywhere else. The interpreter remembers the last callee in each of 256
slots, a name's slot comes from where its text was allocated, and with
this program's names two busy ones share a slot: the table misses 24,661
times where it missed 4,399. interp_allocs 1,074,427 -> 1,069,565 and
interp_peak_bytes 1,527,692 -> 1,524,780.

Two things tried on that table and taken back. Two entries a slot took
the misses to 190 and `callee_missed` to 36,362, and made every hit
dearer: `callee_of_ref` 12,103,951 -> 14,856,020, and the row read
745,934,437. Mixing the hash of the map behind the table left the row
where it was, at 745,519,639.

What else moves. Every program links the kernel, 1,360 bytes: the summed
machine code rises by that for each of the fourteen, and runbench's by
9,376 to 920,721 because the linker also copies the kernel into its seven
call sites. Summed, text 5,983,449 -> 6,010,505. runbench's emitted code
reads defines=993 calls=10404 branches=9373 lines=81337, and summed over
every benchmark but the decoder emitted_other_defines 2,421 -> 2,422,
emitted_other_calls 22,353 -> 22,356 and emitted_other_lines 186,392 ->
186,408. The compile rows rise with the builtin's rows
and the library's lines: emit_instructions 5,632,306 -> 5,633,371,
compile_instructions 24,154,945 -> 24,168,284, entry_instructions
156,095,335 -> 156,165,133 and library_instructions 73,612,703 ->
73,647,363. CI read codegen_instructions_dev at 117,013,839 and
codegen_instructions_release at 372,134,885. Three work rows
move with the layout of the grown runtime: work_encodebench up 154 to
2,335,341,047, work_oneshot down 854 to 9,979,505 and work_livebench
down 546 to 938,654,931. Welfare rises
95.32525334 -> 95.34299175, and the floor is banked there.

Checked against the interpreter. `micro/text_replace_answers_what_split_and_join_did`
prints the call beside the idiom on every engine: occurrences at either
end and two in a row, text to find that overlaps itself, text longer than
a byte, a replacement that holds the text again, and an empty text to
find rescued by its caller. `http/promised`, the one place kanso's own
library used the idiom, calls it.

The issue's second lead, an escape kernel, is not built. Most of what is
left of the phase is the templates that build each row's values:
14,124,590 in `k_b_join` and 3,744,000 in the joins under it, against
4,228,000 in `k_b_replace` and 2,436,774 in `memchr`. A kernel that knew
HTML could take the second pair and some of the escape's own searches, at
the price of HTML in the runtime.

## 2026-11-11 — a file that cannot say its size is read to its end

#2435. `os/read_file` and `os/read_bytes` disagreed between the engines on
any file that is not a regular file on disk. The interpreter reads until
the file ends. The native runtime sought to the end, asked where it was,
and read that many bytes. procfs answers 0 for a file that has bytes, so
`/proc/self/comm` read as the empty string. A pipe cannot seek, `ftell`
answers -1, and that went to `fread` as a count beside a buffer of no
bytes; it read nothing here. The spec for #2431's harness bound found it,
because its first draft read `/proc/self/limits`.

The size is now a first answer and no more. `k_told_size` reports what
the seek says, or -1 when the file cannot seek. A file that reports more
than nothing is read as before: one buffer of that size and one read, and
for `read_bytes` straight into the arena. Anything else goes to
`k_read_unsized`, which reads until the file ends and doubles its room
when it runs out. `read_bytes` copies that into the arena once the length
is known.

Two programs join the stdin corpus, which pipes each program its input.
`a_piped_file_is_read_to_its_end` reads `/dev/stdin` as text, 7,800
characters, so the reader's first 4,096 bytes are not enough.
`a_pipe_s_bytes_are_read_to_the_end` reads 5,000 bytes whose last is not
text. With the unsized branch taken out the native build prints
`0 bytes, last <none>` where the interpreter prints `5000 bytes, last
255`.

What moves. Every program links the reader, 320 bytes of machine code,
and summed over the fourteen text 6,010,505 -> 6,014,985.
The four benchmarks that read a file pay 16 instructions for the
question: work_jsonbench 549,338,015, work_widebench 28,701,603,
work_digestbench 3,107,795 and work_readbench 4,583,700. Five move with
the layout of the grown runtime: work_encodebench down 425 to
2,335,340,622, work_oneshot down 768 to 9,978,737, work_livebench up 324
to 938,655,255, work_runbench down 502 to 1,059,837,979 and
work_runbench_dev down 32 to 3,057,251,699. Clang compiles the longer
runtime: CI read codegen_instructions_dev at 117,031,054, up 17,215, and
codegen_instructions_release at 372,173,775, up 38,890. The work rows are
this container's readings less the 109 CI has read under it, and CI
agreed with every one. No allocation counter, compile
row, interpreter row or browser row moved here.

Welfare reads 95.34294930, under the 95.34299175 the kernel banked. The
objective's header leaves a fix that makes the engines agree outside the
sum, so the floor moves to what the fix costs and its history says so.

## 2026-11-11 — the reach reads code, and a pruning is compiled once

#2434, from its loose end. The issue measured 1.67M instructions for a
library group no program called and guessed that a word in a comment had
kept it. The reach that prunes the shipped library read every word a file
spells: code, comments and the text of strings alike. Three things came
of following that.

**A comment keeps nothing.** `reach_words` stops at a `#`, where the
lexer stops reading. Text holding a `"""` block keeps its comments'
words, because a block's lines are text whatever they open with.

**A string's text keeps nothing.** The issue's cost was one word. The
arm it added to `text/replace` answered `err "replace needs something to
find"`, and `find` kept every group of that name. Measured on the
interpreter's corpus: the arm costs 1,306,636 instructions to check with
that message and 153,551 with `something to look for`. So the reach
walks a string for its interpolations alone. The braces nest as the
lexer counts them and a backslash takes the byte after it. Derived code
is unaffected, because a derivation's text reaches the compiler as a
source file and is read as code there: senro's suite, whose views are
generated from strings, builds the same programs to the same rows.

**The key.** Leaving comments out made `entry_instructions` rise by
432,052, and that led to the largest of the three. A compile of a
shipped module was kept under the number of words the program had
reached so far (kanso#2373), because a module pruned against as many
roots was pruned against the same ones. That number grows with every
module a load reads. Two imports of `std/text` a few modules apart never
shared a compile, though nothing that arrived between them named
anything in it, and the entry corpus compiled `std/text` and `std/list`
ten times each. With fewer words reached up front the count changed at
one more import, which is where the 432,052 came from.

The key is now which of the module's prunings this is. The module is
pruned first, and the declarations it kept are what its compile depends
on: they name everything it reaches in the modules it imports, and the
roots only grow, so a compile kept under the same declarations holds
what a later import needs. Each distinct set is held once for a module
and the key carries its place in that list. An import that finds as many
roots as an earlier import of the module found asks the cache under that
pruning before it reads any source. The entry corpus compiles the two
modules six times each.

**Two versions taken back.** The first comment scan tracked the string
state on every byte and gave back half of what it saved: 288,000
instructions of bookkeeping against 577,000 of library no longer read.
The first key held the kept declarations themselves, a vector hashed and
copied at every lookup, and the module corpus read 11,140 instructions
more with it than with the place in a list that replaced it. Splitting
the scan into functions cost that corpus 65,929 until the one a word
goes through was inlined.

**Specs.** `an_import_reads_what_the_program_names` gains five programs:
a comment that says `pad_start`, above the code and beside it; a string
that says it; a `#` inside a string with the only `trim` after it; a
`trim` inside a string inside an interpolation; and a `lowercase` after
an escaped brace. With comments read the first held `text/pad_start`,
`text/padding` and `text/filled`, and with strings read as code the
second held `text/pad_start`. `tests/golden/compile/apart` is three modules that each trim
a text and bring words of their own: `loads=6`, and 7 under the old key.
Every corpus and module test passes in a debug build, where a pruned
compile the whole library would have accepted is refused out loud.

**Rows.** `entry_instructions` 156,165,133 -> 138,221,281, a fall of
11.5%. `library_instructions` 73,647,363 -> 71,598,427. The tab's
compile 416,189,723 -> 413,661,765. `interp_instructions` 745,491,463 ->
744,801,948, `interp_allocs` 1,069,565 -> 1,068,402 and
`interp_peak_bytes` 1,524,780 -> 1,512,615, from library the corpus's
comments and strings used to keep. `compile_instructions` 24,168,284 ->
24,164,654 and `emit_instructions` 5,633,371 -> 5,632,066.

Three rows rise. The cache keeps a module once a second import asks for
it under the same key, and a second import now shares a key where the
count used to separate it. On the module corpus that keeps copies no
third import comes for: `compile_allocs` 13,956 -> 14,016,
`compile_peak_bytes` 599,057 -> 612,423, and in the tab
`browser_compile_peak_bytes` 984,597 -> 985,156. Welfare weighs the falls
and the rises together and rises. The floor is banked once for this
entry and the two after it, and the last of them says where.

**Not done.** A module is still compiled once for each pruning the load
happens to pass through. Gathering every word the program's own modules
spell before the first shipped module is read would prune each against
the final roots and compile it once. That needs the import graph before
any compile, which the loader does not have. The reach itself measured
3.8% of a check of the module corpus, most of it hashing the words of
declarations it has kept.

## 2026-11-11 — a program's words are read before its first shipped module

#2434's open lead, built. A shipped module is pruned against the words
read so far, and a compile read them as it went. `std/text` imported from
three of a program's files was pruned three ways, each a little larger
than the last, and compiled three times.

A root compile now reads the program's own modules first. It follows
their `import` lines as text, which it can because imports open a file,
adds each module's words to the roots, and counts how many imports name
each module, shipped ones included. Two things follow. The imports of one
shipped module from the program's files ask for one pruning. And a module
two imports name is kept from its first compile: the cache kept a module
only once a second import asked for it, so the second import compiled it
again before anything was shared.

The survey passes over what it cannot read, and the compile still adds
words as it goes, so a module the survey misses costs the compile it would
have saved and nothing else. What the compiler emits is unchanged:
`emitted_code` and `machine_code` agree with their goldens.

**Rows.** The entry corpus loads 25 modules where it loaded 31, and
`entry_instructions` 138,221,281 -> 125,827,812, a fall of 9.0%.
`compile_instructions` 24,164,654 -> 23,782,898. `compile_allocs` 14,016
-> 13,767 and `compile_peak_bytes` 612,423 -> 606,357, the two rows the
entry before this one raised. In the tab `browser_compile_instructions`
413,661,765 -> 413,249,135. `interp_instructions` 744,801,948 ->
744,801,747 and `interp_allocs` 1,068,402 -> 1,068,177. In
`bench/compile_golden_modules.txt` the `shared` sample reads rounds=6
visits=1676 loads=6 where it read rounds=8 visits=2190 loads=11, and
`apart` loads=5 where it read 6. With the survey skipped that golden goes
red on `shared` at loads=11.

One row rises: `library_instructions` 71,598,427 -> 71,987,077, 0.5%.
Counting the imports shipped modules make of each other is what costs it.
Whole-process counts under callgrind, survey off, counting only the
program's own imports, and counting both: the library corpus reads
72,990,673, 73,020,598 and 73,309,958, and the compile corpus 25,395,776,
25,405,957 and 24,964,355. Counting both costs the library corpus 289,360
and saves the compile corpus 441,602, and it is what is kept.

**Tried and dropped.** Pruning every shipped module the program names to
a fixed point inside the survey, so each has one pruning and one compile.
The entry corpus fell to 19 loads. The pass itself cost 3.4 million
instructions on the compile corpus, 28,771,479 against 25,395,776, and the
entry corpus read 132,716,479 with it against 127,955,878 without.

**Not done.** `std/text` and `std/list` are still compiled more than once
in a load: on the entry corpus, list four times and text three. Every word
of the program's own is in hand before the first of those, so the words
that change the pruning come from declarations other shipped modules
kept. A trace of what each pruning kept shows it. `std/list` goes from 26
declarations to 33 by gaining the seven private helpers of its merge
sort, with neither `sort` nor `sort_by` kept: `span` is one of the
helpers, and `lib/text/span.kso` spells the word nine times. Reach is by
bare word over one set for the whole load. Letting a shipped module's
words keep another module's declarations only when they are written
qualified is #2442.

## 2026-11-11 — a shipped module's words keep its own declarations

#2442. Reach is by bare word, and the words were one set for a whole
load, so a word spelled inside `std/text` could keep a declaration of
`std/list`. Each import of a shipped module found more words than the
last, asked for a larger pruning, and compiled the module again.

**Whose words keep what.** Each shipped module has a set of words of its
own: the ones its kept declarations spell. A declaration is kept when its
name is among the program's words, the module's own, or the words of a
shipped module that imports it. A module can call only what it imports,
so a word needs to reach no further. The program's words keep what they
kept before, since a program may use an imported name bare.

The issue proposed that a shipped module's word keep another module's
declaration only when it is written qualified. Lending along the import
does that work without a second kind of word: `text/trim` spelled in
`std/json` is the words `text` and `trim`, and `std/text` is lent json's
words because json imports it.

**One pass, importers first.** Words now travel only down imports, and
the shipped modules' imports hold no cycle. The survey prunes every
shipped module the load will reach before the first of them compiles,
each after the shipped modules that import it. By the time a module is
pruned every word that can reach it has been read, so its first pruning
is its last, and the compile takes the module as the survey left it. If
the program has spelled more words since, which happens when a
derivation's text arrives as a source file, the surveyed pruning is
dropped and the module is pruned where it is loaded.

**Found on the way.** The survey read `import "path"` and missed
`import { join:joined } "std/text"`, which is how `std/path` and
`std/regexp` import. With one set of words nothing depended on the miss.
With a set for each module it left `std/text` without two importers'
words, the pruned compile refused (`std/text` exports no `join` to
rename) and the load fell back to the whole library: the entry corpus
read twice what it had. The scan takes the first quoted text after
`import `.

The survey also named a module's files `span.kso` at first, where the
loader names them `std/text/span.kso`. The checker knows a library file
by that name, so it refused `builtin_slice_in` as internal and the load
fell back again: 41.0 million instructions to check the module corpus
against 24.0. A release build takes that fallback without a word and a
debug build fails an assertion on it, so the loader's specs and the
golden corpus were run in a debug build after each change.

**The version before this one** kept the lending incremental: a module
was pruned where it was loaded, against the importers seen so far.
Whole-process counts under callgrind: the entry corpus 121,365,118 with
24 loads and the library corpus 72,984,999 with 17. One pass reads
122,174,697 with 19 and 65,076,973 with 11. The entry corpus reads a
little more over fewer loads because every importer now takes the module
as all of its importers need it, where the early ones took a smaller
one. The two corpora together read 7.1 million less with one pass.

**Two costs taken out after measuring.** The pruning loop asked the
program's words and each importer's for every declaration on every pass.
Those sets do not change while a module is walked, so each declaration
asks them once and the passes ask the module's own words alone: 78,426
instructions off a check of the module corpus and 147,189 off the
library corpus. The word sets also outlived the compile, which the
interpreter's corpus read as 18,377 bytes of peak. They belong to one
compile now, as the roots do.

**Spec.** `tests/golden/compile/lent` is three modules: two use
`std/list` and the one between them slices a text. `text/slice` has an
arm for a span, and `span` is also a private helper of the list
library's sort. The sample reads `loads=6`. With the loader as it was it
reads 7, `std/list` twice.

**Rows**, against the survey's. `compile_instructions` 23,782,898 ->
22,652,638, `entry_instructions` 125,827,812 -> 119,661,348 and
`library_instructions` 71,987,077 -> 63,632,847, falls of 4.8%, 4.9% and
11.6%. `compile_allocs` 13,767 -> 13,503, `compile_peak_bytes` 606,357
-> 546,568 and `front_end_visits` 7,388 -> 6,870. In the tab,
`browser_compile_instructions` 413,249,135 -> 412,080,126 and
`browser_compile_peak_bytes` 985,156 -> 984,870. `interp_allocs`
1,068,177 -> 1,067,948 and `interp_peak_bytes` 1,512,615 -> 1,509,557.

Four programs emit one line less: the decoder 14,240 -> 14,239, and
`oneshot`, `livebench` and `runbench`, so that summed over every
benchmark but the decoder emitted_other_lines reads 186,408 -> 186,405.
The table of call sites no longer
names the one inside `os/start`, which none of them calls. Another
module's `start` had kept it. Three of the four run a different count
with the shorter table, which this container could not read and CI did:
work_oneshot up 238 to 9,978,975, work_livebench down 679 to 938,654,576
and work_runbench down 133 to 1,059,837,846. The code they run is the
same. The likely cause is where the shorter table leaves the data after
it, and that was not isolated.

Two rows rise. `emit_instructions` 5,632,066 -> 5,632,125. Two builds of
this branch that hand the emitter the same program read 5,632,618 and
5,632,125, so the row is moving with the binary's layout.
`interp_instructions` 744,801,747 -> 744,907,234. Compared function by
function with the loader as it was, the compile half of that row falls
and `Interp::callee_missed` rises 214,571: the interpreter's table of
recent callees takes its slots from where each name's text was
allocated, and pruning the library differently moved the names. The
same table moved the row for `text/replace`. It is #2444.

**Welfare.** Over this entry and the two before it welfare rises
95.34294930 -> 95.39064215, and the floor is banked there.

**Not done.** `std/derive` is still compiled twice on the entry corpus,
and that was not looked into.

## 2026-11-11 — a derive line is an import the survey counts

The last repeat #2442 left: the entry corpus loaded `std/derive` twice.
The entry above says that was not looked into, and this is the look.

An entry with a `derive` line imports `std/derive` without writing the
import; `compile_entry_program` adds it. The module that declares the
derivation imports `std/derive` too. The survey counts imports by reading
the program's text, and a module two imports name is kept from its first
compile. It read one import where there were two, so the first compile
was not kept and the entry's own import compiled the module again.

`from_entry` now hands the survey the line the entry did not write, when
the entry's source has a line that starts with `derive `. Nothing else
changes: the import was already made, and the survey only counts it.

**Spec.** `tests/an_entry_that_derives_compiles_derive_once.rs` checks the
smallest program in the derive corpus with `KANSO_PHASES=1` and counts the
loader's lines: `std/derive` once, and 2 with the line withheld. A second
test counts `std/list`, so the first cannot pass on a loader that prints
nothing. The ratchet row is `derive_counted`.

**Rows.** `entry_instructions` 119,661,348 -> 119,206,191, and the entry
corpus loads 18 modules where it loaded 19. Two rows move with the
binary's layout and nothing they count: `compile_instructions` 22,652,638
-> 22,652,656 and `library_instructions` 63,632,847 -> 63,634,723.
Neither corpus has a derive line. The first version cloned the entry's
directory path whether or not it was needed, which the interpreter's
corpus read as one allocation more; the clone is made for an entry that
derives and for no other.

The browser's compile of its corpus reads the check:
`browser_compile_instructions` 412,080,126 -> 412,081,307, which is 1,181
instructions over the corpus for the search of each entry's text. With
the check withheld the row read 412,080,126 again. A version that read
only the entry's header, as the parser does, was measured and dropped.
No program's output differed between the two, and it read 412,081,343
there, 119,205,544 on `entry_instructions` and 63,641,726 on
`library_instructions`, each higher than the search of the whole text.

**Welfare.** 95.39064215 -> 95.39098937, banked.

## 2026-11-11 — a called name's place in the table comes from the program

#2444. The interpreter keeps what a function reference calls in a table,
and a declaration's frame in another, so that a call does not hash the
name's text or format a trace line. Both tables took their slots from
addresses: the address of the name's text and the address of the
declaration, each reduced to its place in a 64 KiB allocator slice. Two
names shared a slot when their texts were allocated a multiple of the
table apart, and then each put the other out on every call. Where a text
is allocated depends on everything allocated before it, so a change to
the front end alone moved how often the tables missed. The last two
entries on the loader and the one on `text/replace` each named a rise in
`interp_instructions` that came from there.

`resolve` now numbers a name the first time the run meets it, and the
number travels in `Value::FnRef` beside the name. The table of callees is
a vector indexed by it. `Value` is still 32 bytes. A declaration needs no
number of its own: every declaration the interpreter enters is an element
of `program.fns`, so its distance from the first is its index, and the
table of frames is a vector indexed by that. Neither table has a fixed
size or a second map behind it, and no two entries share a slot.

A reference made outside `resolve` has no number and is looked up by name
on each call. There are two: the one a derivation hands its deriver, and
the one a tie hands its maker.

Measured on the interpreter's corpus, in this container:

| | before | after |
| --- | --- | --- |
| `interp_instructions` | 744,907,234 | 741,096,980 |
| `interp_allocs` | 1,067,948 | 1,067,941 |
| `interp_peak_bytes` | 1,509,557 | 1,501,749 |

That is 0.51% fewer instructions. In the profile, `callee_missed` and
what it calls fell from 1,900,502 to 129,815, which is one lookup for
each name the corpus calls.

One version was measured and dropped. Keying the frame table by where
the source wrote the declaration, its line and column, read 744,370,410
with the callee table already numbered, against 742,958,287 without the
change: declarations at the same line of different files shared a slot,
and `frame_missed` went from 388,208 to 1,488,815.

Two counters, `interp_callee_lookups` and `interp_frames_made`, are
counted where a table is filled and printed under `KANSO_COUNTERS`.
`tests/a_reference_is_looked_up_by_name_once.rs` pins both at eight for
a program that calls eight names, at 300 laps and at 3,000. The ratchet
row is `looked_up_once`: with the table read past its end, the 300-lap
run reads 1,221 lookups.

What is not shown is the old tables moving under a perturbation made
from outside. Three were tried on the old binary and its count held
still under each: a block allocated before the run, a preload that pads
allocations (the compiler links mimalloc, which a preload does not
reach), and sixty unread program arguments. The evidence that the old
slots moved with the front end is the two per-function profiles on #2444.
That the new ones cannot is by construction: a slot is an index the
program decides.

## 2026-11-11 — a job's install reads a mirror that resolves

Not a compiler change. From 13:52 UTC the first mirror in a hosted
runner's apt list, `azure.archive.ubuntu.com`, stopped resolving on the
runners, and it was still not resolving two hours later. Every job that
installs a package died in the install with exit 100, before any test
ran: in kanso on two pull requests, and in senro and kq. The name
resolved from this container throughout, and GitHub's status page showed
no incident.

`scripts/apt_ready.sh` runs before each install in `ci.yml`, in
`pages.yml` and in the ratchet's `toolchain.sh`. When the first mirror
resolves it changes nothing. When it does not, it rewrites the runner's
list to every other mirror it knows whose name does resolve, and tells
apt to try a fetch again before giving up. apt checks what it downloads
against the archive's signature, so the mirror that served the bytes
does not change what is installed.

The first version took one other mirror. On the runners
`archive.ubuntu.com` did not resolve either, and `mirrors.kernel.org`
did. Every install step in that run passed but one: the specs job
installed clang from the one mirror and then failed to connect to it for
the next package. So the list now holds all that resolve, five known to
the script.

`tests/apt_reads_a_mirror_that_resolves.rs` runs the script against a
list and a resolver of its own, four ways: the first mirror answering,
the first away, both of Ubuntu's names away, and none answering. Watched
fail with the check for the first mirror forced to pass.

## 2026-11-11 — a measured link names its runtime plainly

`codegen_instructions_release` read 372,173,792 in CI on this branch,
twice, against 372,173,775 on main. Nothing on the branch reaches
codegen, and the IR the corpus compiles to is byte for byte what main
writes. Split by process, clang counted the same on both trees and the
linker counted 17 more.

The 17 came from a file name. A build compiles the runtime once and
keeps the objects in the temp directory, each named by a digest of the
recipe that made it, and the recipe for the hot unit includes the text
of `src/main.rs`. This branch edits `main.rs` to print two counters, so
the hot unit's object had a new name. The two links' command lines
differed in that name and in nothing else. lld keeps its inputs' names
in a hash table, and the work the table does depends on the characters
in them.

Under `KANSO_FIXED_TEMPS`, which the gate sets for a measured build, the
link is now handed copies of the cached objects under plain names:
`kanso_linked_runtime_release.o`, `kanso_linked_runtime_release_hot.o`
and `kanso_linked_runtime_dev.o`. A build without the variable links the
cached files as it did.

Plain names alone were not enough. With them, a comment added to
`main.rs` still moved the release row, by 64. An object records the name
of the file it was compiled from, and the cache wrote the runtime's
source to a file named by the same digest. So the cache now compiles
each object from standard input, and no object holds a name the digest
decides. With both changes, the gate's build read the same on both tiers
before and after a comment was added to `main.rs`. That was read in this
container, whose codegen numbers are not CI's.

**Rows.** In CI `codegen_instructions_release` 372,173,775 ->
372,208,803 and `codegen_instructions_dev` 117,031,054 -> 117,027,868.
The release row rose by 35,028 and the dev row fell by 3,186. In this
container both fell, by 114,443 and 3,535, so the size and the direction
of the move belong to the host's clang and linker. The change swaps a
source file clang opens for text it reads from a pipe, and changes the
names the link reads; which of the two the 35,028 is was not separated.
Both rows are re-based on CI's reading, as #2017 rules for a row that
stops reading outside state. No other compile row moved, and neither did
the interpreter's or the browser's.

**Spec.** `tests/the_measured_link_names_its_runtime.rs` reads the link's
line from a measured build of each tier and from an unmeasured one, and
searches every cached object for a name that holds a digest. The ratchet
row is `plain_link`: with `linked_as` answering the cached path, the
measured release link names `kanso_runtime_release_hot_8d55bbc03f0775ad.o`.
The row `runtime_source` is retired with its mutation, which wrote the
runtime's source onto the shared path in place. No build writes that
file now, so there is nothing for a second build to truncate, and
`tests/concurrent_build.rs` still races eight cold builds.

**Welfare.** 95.39229249 -> 95.39228290, a re-base. The fall is the
release row's 35,028 and the browser row's 1,181 from the derive check,
which the bank before this one had not read, less the dev row's 3,186.
Main's floor is 95.39064215, so the pull request as a whole is a rise.

## 2026-11-11 — directive: senro is to be a full alternative to Rails

Clay, to cloud: "there should be a [senro] binary that acts like rails
and has generators and can make a migration. the migrations need to have
a date time stamp thing in the file name so they can run in order and
create the actual schema deterministically." He asked for the generators
Rails has, "a resource that's restful making just a controller or
whatever", and "a facility to add other generators through plugins"; for
documentation that reads as "a fully formed alternative to rails or at
least as much functionality as you would have encountered in the early
days around rails three"; for auth in the framework, since "everyone
needs auth"; and for authorization that is "about getting to that page",
so that "once you checked [auth] for that page you don't need to recheck
for every data provider that it uses".

Recorded on #2455 with where senro stood and a checklist. The design is
cloud's (#2280) and is on the issue: a `senro` command; migrations as
files named by a stamp, still folded into the schema by the build;
generators for the parts and from outside senro; confirmation, lockout
and tracking moved from the guestbook into the framework; and one check
at the page that covers the providers the page calls.

## 2026-11-11 — a derivation is told which file declares a name

The first thing #2455 needs from the language. senro's migrations are to
be files that run in the order of their names. A derivation folds them
into the schema while the app compiles, and a derivation could not see a
file's name: a module's kanso files share one namespace, the description
handed over a module's constants with no file, and it left the module's
kanso files out of `files`.

The committee's reading: a convention that places code by its file's
name is the same kind of convention as one that places it by its
function's name, and the description already carries the second. So a
module's description gains `sources`, a map from each pub function's and
pub constant's name to the name of the file that declares it. The first
arm's file answers for a function of several. Nothing else changes, and
a derivation still only computes: the map is read off declarations the
compiler has already loaded.

**Spec.** `tests/golden/derive/a_derivation_reads_the_file_a_declaration_is_in`
keeps two constants in two files whose names sort the other way from the
constants' names, and a convention answers the names in the order of
their files: `["zebra" "apple"]`, on both engines. The ratchet row is
`declared_in`. Watched fail with every file's name left blank: the
convention falls back on the names and answers `["apple" "zebra"]`.

**Rows.** Three compile rows move with the binary's layout, each down:
`compile_instructions` 22,652,656 -> 22,652,542, `entry_instructions`
119,194,010 -> 119,166,280 and `library_instructions` 63,634,723 ->
63,634,563. None of the three corpora has a `derive` that reads the new
field. The interpreter's and the browser's rows did not move.

**Welfare.** 95.39228290 -> 95.39230414, banked.

## 2026-11-11 — the sanitizer sweep is a job

#2449, from the weekly sweep: `scripts/asan/sweep.sh` was named by no job,
script or test. It builds every program in the memory corpus and every
benchmark against a runtime compiled with AddressSanitizer and runs them.
The entry that added it (2026-09-22, in the archive) left it to be run by
hand.

Run by hand today, on clang 19, its own `--prove` failed: a read after
free put there on purpose went unreported. The compiler emits closures on
`preserve_none` when clang takes the convention, and builds its runtime
with `-DKANSO_PRESERVE_NONE` to match. The sweep built its runtime without
the flag, so a closure read its arguments from the wrong registers and the
fixture stopped with a run-time error of its own before it freed anything.
The sweep looked only for a sanitizer report, so a program that stops
that way read CLEAN.

Four changes to the script:

- The runtime is built on the convention the program's IR uses, read off
  the IR. Clang will not compile a `preserve_none` function with the
  sanitizer's fake stack, so that build turns use-after-return detection
  off and keeps the heap checks.
- A program has to finish. A fixture's output is compared with its
  `.stdout` golden, under the seed `tests/golden.rs` pins, and a benchmark
  has to exit zero. Either failing is `DIED` and fails the sweep.
- Benchmark directories are copied with their links followed. `runbench`
  links to the other benchmarks' files and was skipped as unbuildable.
- It lifts the 6 GiB data limit for itself. A sanitized binary reserves
  its shadow as private mappings, which that limit counts, and under it
  no sanitized program starts.

It is a job in `ci.yml` now, on clang 19 with `libclang-rt-19-dev`. The
ratchet row is `freed_read`: one byte read from a tenure block after
`k_ten_release` frees it. No golden sees that. The sweep reports it in
eight fixtures. The row replaces `--prove`, which made the same patch on a
copy of the runtime.

On this tree the sweep reads 163 programs clean and none reported. Ten are
skipped by name: the `work_ahead_*` fixtures hold definitions beside bare
statements, which `kanso play` runs and `kanso build` refuses, and the
sweep links what `build` emits. The guarded sixteen-byte tail read in
`k_b_find2_raw`, which `runbench` and `scanbench` reported in September,
was reported by no program. Why is not established.

The sweep cannot see a read that strays from one value into its neighbour
inside an arena block, because the whole block is live memory to the
sanitizer.

No counter moved.

## 2026-11-11 — the third sweep reads its gates, and the browser gate joins it

#2447, its fourth item. `all_interp.sh` listed its four gates by hand, and
`tests/every_golden_is_swept_by_something.rs` existed partly to keep that
list honest against what CI runs. The sweep now reads its gates from
`scripts/gates` the way `all_compile.sh` does: a script that names a
golden under `bench/` and that neither other sweep runs is its own.
`--list` prints them, and the test asks the sweep.

Deriving the list found a fifth. CI runs `browser_cost.sh` against
`bench/browser_golden.txt`, and no sweep ran it. The test should have
said so. It decides whether a gate reads a golden by scanning the script
for a file name, and the only place that script names its golden is a
comment that ends the sentence there, so the scan read
`browser_golden.txt.` and took the full stop for part of the name. With
the stop trimmed the test fails and names `browser_cost`; with the
derived list it passes.

The interpreter sweep is slower by a wasm build and the browser test. No
counter moved.

## 2026-11-11 — two limits live where they are enforced

#2448, from the weekly sweep. Not a compiler change.

The live log's limit was written in `scripts/archive_log.sh`, three
times in its test, in the test's file name and in CLAUDE.md. It is now
the script's `keep=` line and nowhere else:
`tests/the_live_log_holds_what_the_script_keeps.rs` reads the number
from the script, builds its scratch log two entries past it, and
CLAUDE.md names the script. Watched fail with `keep=41`: the test says
the log holds 40 and the script keeps 41. The page-drift budget is
`budget` in `scripts/page_drift/page_drift.kso`; CLAUDE.md and the
compiler page name it without the number.

Three stories were told twice, in a sweep's header and again in its
test's doc comment: the span that cost a round on kanso#1328, the
interpreted row no sweep read on 2026-09-18, and the library change of
2026-09-05. Each stays in the script, and the test points at it. The
trend gate's excuse for the one golden without counters is said in the
gate, and its test no longer repeats it.

The link's staged directory was said to hold "five libraries" in two
scripts, a test and the page. The list is the loop in
`scripts/gates/codegen_box.sh`, and the other three now say "the
libraries the link resolves". One copy is left as it is: the compiler
page gives the two greps that count rulings as part of a dated story,
with the counts they returned that day, and CLAUDE.md holds them as the
rule.

The chart's palette had a second copy too (#2447, its first item):
`tests/the_chart_palette_is_the_one_that_was_measured.rs` held all the
colours of `docs/numbers.html` and asserted the page equalled them. The
chart draws one panel to a counter, so colour tells no two lines apart.
The copy is gone, and `tests/a_chart_line_wears_a_token_both_modes_declare.rs`
checks the page against itself: both modes declare the same tokens, and
every line wears a declared token and no hex. Watched fail with a token
taken out of the dark block and with one line given a hex.

No counter moved.

## 2026-11-11 — senro gets its command, its migration files and a page's check (#2455)

The first four items of the 2026-11-11 directive are built in senro, and
the site's guides follow them. senro's design is cloud's (#2280), so each
choice below was made where it came up and is recorded here and on #2455.

**The command.** `bin/senro` is a shell launcher over `senro.kso`. `new`,
`generate` and `routes` are kanso; `server` and `migrate` are handed to
`kanso run .`, because a child kanso starts has no terminal (`os/run`
captures a child's output and `os/start` drops it). `senro routes` reads
the table from what `kanso derived` prints, so listing routes compiles the
app and opens no database. A generator from outside senro is a kanso
program at `generators/<name>.kso`, in the app or beside a library its
entry imports by path.

**Migrations are files.** `migrations/<stamp>_<name>.kso` holds one pub
constant, a change or a list of changes, and the stamp is fourteen digits
of UTC. The order is the files' names. That needed one thing of the
compiler, `sources` on a derivation's module (the entry above), since a
module's files share one namespace and nothing else says which file a
constant is in. The models derivation defines `migrations/all` from the
files, so an entry is written as before, and a migration is recorded under
its stamp. Two things were decided against. A stamp in the constant's name
would have ordered them with no compiler change and made every migration's
name ugly. A list the app keeps by hand was what senro had, and two
branches that each add a migration conflict on it every time.

Rolling a migration back is not built. A migration is a list of changes
with no inverse written down, and that design is open on #2455.

**A page's check.** A module's `admits action viewer db` says who may
reach its actions. A provider's request names its page, and the server
answers it as it would the page: the page's module must lean on a module
that asks for the provider (worked out from the app's calls), a page that
asks for a sign-in answers only a visitor who has one, and `admits` is
asked once for the batched request. Before this the provider route was
open to anyone, which the README did not say. A token signed when the
page renders was weighed and dropped: a permission taken away afterwards
would have kept working until the token aged out. The hook is `admits`
and not `admits?` because a `?` function answers a plain true or false,
and this one may read the database.

**Generators.** `controller`, `model`, `migration` and `mailer` join
`scaffold` and `authentication`. `new` now runs a mail worker beside the
server, so a mailer in an app without accounts has something to send its
letters.

**Still owed on #2455.** Confirmation, lockout, sign-in tracking and an
inactivity timeout are hand-written in the guestbook and belong in what
the `authentication` generator turns on. `job` and `resource` generators.
Server components' actions are not yet answered as their page is. Guides
for controllers, mailers, testing, configuration, deployment and security.

No counter moved in kanso: the work is in senro and on the site.

## 2026-11-11 — `kanso test` performs a test whose value is an effect (#2458)

senro's guides owe a page on testing an app (#2455), and an app's actions
take a database session and answer an effect. `kanso test` could not test
one. It evaluated each `test_*` constant and passed the ones whose value
was `true`, so a test whose value was an effect read
`FAILED (returned <io>)`.

**The search.** The log's entries on the test verb (2026-10-14, the module
a test is run inside; 2026-10-29, std/expect in the binary) say nothing of
effects. The archive's first design of the verb says "a test is a constant
named `test_*` whose value is `true`. no framework, no assertion dsl". The
book says a test's value is a boolean in chapter 01, chapter 07 and the
tour. No fixture under `tests/golden/test_verb` held an effect, and no
issue asked. What the runner printed for an effect was what it prints for
any value that is not `true`, and nobody had decided it.

**The decision**, by the committee, recorded on #2458. A test stays a
constant and `true` stays the only passing value. An effect is a value, a
description an executor performs, so a test whose value is a description
has not said true or false yet. The runner performs it, with the executor
`kanso run --interp` uses, and judges what it yields as it judges any
test's value. An err the effect ends in fails the test and prints its
reason and where it was born, as an entry's unhandled err does. Tests
still run one at a time in the order the file declares them.

Two alternatives were turned down. A second kind of test, under another
prefix or another verb, makes two ideas of one. Judging the plan in place
of performing it yields nothing to compare: a plan lists the effects that
would run, and a test of a query needs the rows.

**Built.** `run_tests_on_stack` executes a test whose value is a `Desc`
and matches on what comes back. `tests/golden/test_verb/performed` holds a
pure test, an effect that yields true, one that yields false, one that
yields a string, one that ends in an err, and one after the err to show
the runner went on; `performed.out` and `performed.stderr` are what it
prints. Watched red on the old runner: all five effectful tests read
`FAILED (returned <io>)`.

The book's effects chapter gains a paragraph after its two levels of
testing, and the tour's sentence on what a test is now allows an effect.

**Open.** Each effectful test gets an executor of its own, seeded as
`run --interp` seeds one: from `KANSO_SEED` when it is set, and from the
system when it is not. A suite has no setup or teardown; a test that needs
a clean table makes one.
senro's testing helpers are built on this next.

## 2026-11-11 — senro: jobs by convention, an API generator, an idle window, Secure cookies, four more guides (#2455)

More of the 2026-11-11 directive, built in senro and written up on the
site. senro's design is cloud's (#2280); each choice is recorded here and
on #2455.

**Jobs by convention** (senro#122). An app's own work had no convention:
it built a table of performers by hand, and a job's name was a string
nothing checked. A module named `<name>_job` is now the job `<name>` and
its `perform db args` is the work. `derive jobs/performers` gives the
entry the table, found from the modules it imports as `routes/rest` finds
actions. `jobs/perform_later db name args` queues one, and a name written
as a string that is no job's is refused at the call while the app
compiles, as `routes/path_to` refuses a route that is not there. `senro
new` writes a second worker for these beside the one that sends mail, and
`jobs/beside` starts no worker for a table with no performers, so an app
with no jobs pays nothing for the line. One queue, `default`; a job's own
queue is left until an app needs one.

**Lockout and tracking in what the generator writes** (senro#121). The
guestbook's hand-written lockout and sign-in tracking moved into an
`accounts` module, and `senro generate authentication` writes a sign-in
built on it: five wrong passwords in a row lock an account for an hour,
and each sign-in is counted and stamped. Five and an hour were chosen
because the generated app unlocks by time alone for now, and the address
limit beside it already allows ten tries in three minutes. The columns the
module writes are `accounts/columns`, read by the generator, so the list
has one home. Confirmation by mail and the unlock link are still the
guestbook's alone.

**A resource with no pages** (senro#124). `senro generate resource` writes
an API over a model, as `rails g scaffold --api` does: the model and its
migration as `scaffold` writes them, and five actions that answer JSON.
`web/input` reads a posted form or a JSON object by the request's content
type, and the generated module keeps only the fields it was told about.
Two faults turned up while driving it. An update answered the row as the
model guessed it would be, so an int sent as JSON came back as a string;
`pg/update_by_id` now returns the stored row. And an update that changed
no field built an SQL statement with nothing to set; it now answers the
row as it stands.

**An inactivity window** (senro#125). `web/idle_after held minutes` ends
a session after that long with no request, as Devise's timeoutable does.
The cookie carries when the session was last seen, and an answer rewrites
it once a second has passed, so a page that makes several requests at
once does not rewrite it for each. The alternative was to keep the last
moment in the database, which would cost every signed-in request a write.
The generator does not turn the window on.

**Secure cookies** (senro#123). Reading `web` to write the security guide
found that the session cookie was `HttpOnly` and `SameSite=Lax` and never
`Secure`. The server speaks HTTP behind a proxy, and cannot see the
scheme the visitor used. It now reads `X-Forwarded-Proto` and marks every
cookie in the answer `Secure` when the header says `https`. The
alternative was a setting, which every deploy would have to remember. A
client that sends the header falsely gets cookies its own browser will
not send back, so trusting it costs nothing.

**The guides.** Four are new: controllers and actions, mailers, security,
and configuration and deployment. The jobs guide gains the convention,
and the sessions guide's lockout and tracking sections now quote
`accounts`. Mail's transports are described in the mailers guide alone;
the sessions guide points at it. The generators and command-line guides
gain the resource generator and the sessions guide the inactivity window.
Every panel is checked against senro at 70dc382 by `senro_quotes`.

A standalone output panel had no style of its own: `.code-output` was
written for the lower half of a code panel, and the senro guides use it
by itself. It drew pale text on the page with no background, and a long
line widened the whole page: the jobs guide measured 2,344 pixels across
in a 1,280-pixel window. `docs/style.css` now makes one that stands
alone in an article a panel with its own horizontal scroll. Ten guides
were rendered through the layout and measured at 1,280 and 390 pixels,
and each now stays inside its window.

Clay, reading the site: "the web site has what looks like a not equal
sign. it should just be != i think. you don't need to render it in a
fancy way that's not what real code looks like." JetBrains Mono joins
`!=` and `->` into one glyph each. The stylesheet now turns the font's
ligatures off for code everywhere, and `site_smoke` asks the browser what
it settled on for a line of code under that stylesheet. Watched red
before the rule: `code is shown with ligatures (normal)`.

**Open.** A guide to testing waits on #2460 and on senro's testing
helpers. A component's server action answered as its page, rolling a
migration back, and confirmation and unlock mail in what the generator
writes are still owed.

## 2026-11-11 — a test file beside an entry reads what the entry reads (#2460)

A senro app could not be tested with `kanso test`. Two things stood in the
way, both found by trying it on `examples/notebook`. The verb took the
app's directory for one module, where it is an entry and the files it
imports one at a time, and the load then failed on `views/`, which is a
module only because the entry's `derive kata/views` writes its functions.
And a test file is a library, so it could not write `derive` lines, and had
no `note/create`, no `routes` and no `schema` to call.

**The search.** The log's entries on the test verb (2026-10-14,
2026-10-29) and the 2026-10-12 gavel (#1883: a test file is an outside
consumer of the module beside it) are all about a module that is a
directory. No fixture under `tests/golden/test_verb` had an entry or a
derivation in it, and no test file in kanso, kq or senro sat beside a
`main.kso`.

**The decision**, by the committee, recorded on #2460. A test file beside
a `main.kso` tests that program. It is compiled with the entry's imports
and the entry's derivations, and none of the entry's statements run. It
may add imports of its own, and only those can read as unused. The gavel
on #1883 carries over a word at a time: a test file reads the thing beside
it as a consumer would, and beside an entry that thing is the program.

The alternative was to let a test file write `derive` lines. The committee
first leaned that way and turned it down on two counts. `routes/rest`
describes every module the entry imports, so a test file would have to
repeat the entry's whole import list to derive the same table. And the
app's derivations would then be written once in `main.kso` and again in
every test file, which is one fact with several homes.

**Built.** `compile_test_file` looks for `main.kso` in the test file's
directory. When it is there, the entry is parsed, its imports are lent to
the test file, and its derivations run over the entry as they do when it
is the program, so a derivation sees none of the test file's helpers. What
they declare is declared for the test file, and an arm they define is
compiled into its module. The library is read as far as both texts reach.
An entry that does not parse, or whose derivation refuses, is reported at
the entry's own line.

Three fixtures under `tests/golden/test_verb`: `beside_an_entry` calls a
module the entry imports, reads the table the entry derives and calls an
arm the derivation wrote; `beside_a_broken_entry` and
`beside_a_refused_entry` pin the two reports. Watched red with the lookup
turned off: `unknown name` for each of the entry's names, and for the
broken entry the test file's own `unknown name` where the syntax error was
expected.

**The rows.** CI's first reading had `library_instructions` 2,304 above
its golden. A file's import list was copied on every library compile, to
know which imports the file wrote itself once an entry had lent it more.
The copy is now made only when an entry is beside the file, and the row is
63,634,563 -> 63,634,689: 126 instructions for the question "is there an
entry beside this file" on a path where the answer is no.
`entry_instructions` is 119,166,280 -> 119,166,322. Nothing on the entry's
path changed, so that is the binary's layout. The browser's compile row
went the other way by two, 412,081,307 -> 412,081,305.

The welfare floor is lowered by what this costs, under the exception for
building a ruled part of the language: #2460 is the committee's decision,
and the floor's history says so: 95.39230414088809 ->
95.3923041102071.

**Open.** Each test file beside an entry compiles the app again. One test
file beside senro's `examples/notebook` took 1.9 s in this container from
start to finish, and nothing here measures an app with many of them.

## 2026-11-11 — directive: specs that nest, with hooks, first class in senro

Clay, to cloud: "the framework needs to have something like what the rails
helpers do where your tests set up a transaction in a before block or
around block, so it gets reverted at the end, and nested specs work and so
forth. i don't know how far you've gotten on our kanso alternative to
rspec/ginkgo/gomega but it should be a first class citizen in senro."

Recorded as #2464. kanso's testing was Go-test shaped by his ruling on
#966, which ended "eventually we'll make something more like rspec". This
is that.

## 2026-11-11 — std/spec: a spec is a test whose value is an effect (#2464)

**The search.** #966 and its entry (the Go-test shape), `std/expect`'s
header (why a matcher answers a record), the two entries above on what
`kanso test` performs and reads, and the lexer's rule for a list that
spans lines.

**The decision**, by the committee, recorded on #2464. A spec is a value
and a test is still a constant. `describe name items` answers an effect
that runs the examples under it, prints them as an outline and yields true
when every one passed, and `kanso test` has performed a test whose value
is an effect since #2458. So the verb gains nothing, and the compiler
still carries no test vocabulary, which is what #966 took out of it.

Hooks are functions around the rest. `around` is handed the value so far
and the rest of the example as a function; `before` and `after` are
written with it. A group's hooks wrap those of the groups inside it,
every example gets its hooks run fresh, and an example is handed what
they built as its one argument, so nothing plays the part of RSpec's
`let`. A group inside a `describe` is `context`: a nested `describe`
would be an effect sitting in a list, which the library could not look
inside.

No syntax was needed. A list that spans lines takes one element to a
line, a line that is a call needs no parentheses, and a line that ends by
opening a bracket spans in turn, so a nested spec is written as RSpec
writes one with the grammar as it stood. The first prototype wrapped each
item in parentheses and was refused at `])`, and the lexer's own comment
on element lines is what showed the parentheses were the mistake.

**Built.** `lib/spec/spec.kso`, shipped as `std/spec`. `std/expect` gains
`contain`, `have_length`, `start_with`, `end_with`, `be_none`, `be_false`,
`satisfy` and `not_to`, which answer an `unmet` record of what was wanted,
in words, and what was got. `expect` and `to` take none, since `be_none`
is a matcher.

`tests/golden/test_verb/nested_spec` is a stack built by `before` hooks
three groups deep, with an `around` that prints a line on each side of
every example. Its golden holds the outline, the two examples made to
fail named by their full descriptions with what they answered, and the
count. One fault was found by it and is pinned by it: an example that
ended in a failure skipped the closing half of its `around`. A failure is
a box, and only an arm that asks for the box is handed one, so the arm
that was meant to catch it inside the hooks never ran and the catch
outside them did. Watched red: that example's `(closed)` line was missing.

The book's modules chapter gains "specs that nest", with a panel the book
check runs. Breaking the panel's expected output turned the check red.

**Rows.** The compiler's table of shipped modules holds one more name, and
the four rows that pass through it worsened: `compile_instructions`
22,652,542 -> 22,652,874, `entry_instructions` 119,166,322 ->
119,166,434, `library_instructions` 63,634,689 -> 63,634,765, and
`browser_compile_instructions` 412,081,305 -> 412,081,336. The welfare
floor is
lowered by what that costs, 95.3923041102071 -> 95.39230375032061: the
directive asked for the library, so the exception for a ruled part of the
language applies.

**Open.** One example cannot be run by name. The outline is printed by
the library and `kanso test` adds its own line for the constant, so a
file's count of tests is its count of specs, with the examples counted on
the line above. senro's hooks, generated specs and guide are next.

## 2026-11-11 — a test beside an entry reads the names the entry binds (#2463)

The first test written for a senro app needed the app as its entry wires
it: `signed`, the route table with the sign-in requirement around it, and
`connected`, the effect that opens a database session. Both are bindings
among the entry's statements, and #2460 lent a test file the entry's
imports and derivations and nothing else. The test would have had to wire
the app a second time, and would then be testing its own wiring.

**The search.** The entries on the test verb (2026-10-12, 2026-10-14,
2026-10-29, and the two above) and on the entry (`an entry names a group`,
2026-10-29; `a lazy binding of an effect, handed back unread, still runs`,
2026-11-05) say nothing on a test reading an entry's names. No fixture
under `tests/golden/test_verb` had a binding in an entry.

**The decision**, by the committee, recorded on #2463. A test file beside
an entry reads the entry's bindings as names. An entry's statements are of
two kinds: a binding gives a value a name, and a bare expression is
performed. An effect is a value, so a binding performs nothing even when
its value is one, and naming it in a test evaluates what the entry wrote.
#2460's "none of the entry's statements run" still holds.

Moving the wiring into a module a test could import was turned down:
every app would be laid out for the test verb's sake, and one that kept
its wiring in the entry, where `senro new` puts it, could not be tested
as wired.

**Built.** In `compile_library_beside`, each `name = value` among the
statements of the entry beside a test file becomes a constant of the test
file's program, stamped with the entry's file. A test file that declares
one of those names is refused at its own declaration and told where the
entry binds it. A pattern on the left lends no names.

Two fixtures. `reads_the_entrys_names` binds a string and an effect that
prints it, and performs the effect; its test file reads the string and
performs the effect itself, and the golden shows the line printed once,
between the two tests. `names_the_entry_bound` pins the refusal. Watched
red with the bindings left behind: `unknown name` for both names in the
first, and in the second the test file's own `front` stood alone and
passed.

Tried on senro's `examples/notebook`: a test file reads `signed`.

**`kanso check` agrees.** A test file was checked by its content, as any
other file is: one holding constants alone was an entry with unused
bindings, and one with a function was a library that knew none of the
names the test verb lends it. So an editor showed `unknown name` on every
line of a test that passed. `compile_source` now hands a `*_test.kso`
file under `check` to `compile_test_file`, which the check verb and the
language server both reach. Watched red on the two fixtures' test files,
which `kanso test` ran and `kanso check` refused.

**Open.** A binding that takes its value apart lends nothing, and a test
that names one of its parts is told only that the name is unknown.

## 2026-11-12 — a dependency's local in a list is not the importer's function (#2466)

senro's spec generator is a new module that declares `fn listed s` and
imports the scaffold generator, which binds a local `listed` below a guard
and holds it in a list. `kanso check` refused the scaffold's list, in a
file the change had not touched: "`listed` takes 1 argument(s), and a list
element is one atom".

**The fault.** `call_shaped_at` asks whether a bare name in a list is a
function that takes arguments, and reads a table built from every
declaration of the module being checked. That module's dependencies are
walked with the same table, so a dependency's local was looked up among
the importer's functions. The walk's bound names would have excused it,
but they hold parameters and top-level binds, and a bind below a guard is
neither. Inside one module the collision cannot arise, because binding a
name a declaration has is refused first.

**Built.** For a declaration that is not the module's own, a bare name is
not looked up in the table. The dependency's own functions are qualified
when it is loaded, and its own check reads its lists against them, so a
bare call in a dependency's list is still refused where it is written.

Two fixtures under `tests/golden/shadowing`. `held` is the postcard from
the issue, run on both engines; watched red with the refusal above naming
`held/pair.kso`. `called` pins that a dependency's own bare call in its
own list is refused at its line, which passed before and after.

**Rows.** Skipping the lookup for every declaration from a dependency makes
three compile rows cheaper, as CI measured them against the main before
std/spec: `compile_instructions` 22,652,874 -> 22,642,731,
`entry_instructions` 119,166,434 -> 119,108,712 and `library_instructions`
63,634,765 -> 63,609,813. `browser_compile_instructions` worsened, 412,081,336
-> 412,081,396. The welfare score rose, and the floor is set to it,
95.39230375032061 -> 95.39235548899775. The check verb's route for a test
file reaches no compile any row measures.

## 2026-11-12 — a spec of one example says "1 example" (#2469)

The home-page spec `senro new` writes has one example, and `std/spec`
ended its run with "1 examples, 0 failed". The summary now counts with the
noun's number, as RSpec prints it: `counted 1` is "1 example", and any
other count takes the plural. `tests/golden/test_verb/one_example` is a
spec of one example; watched red on main with the plural.

**Rows.** `lib/` is compiled into the binary, so the compile rows moved
with it: `compile_instructions` 22,642,731 -> 22,642,405, and two worsened,
`entry_instructions` 119,108,712 -> 119,108,774 and `library_instructions`
63,609,813 -> 63,609,875. The welfare score rose, and the floor is set to
it, 95.39235548899775 -> 95.39235569042921.

## 2026-11-12 — gavel: senro fitted to one way of doing each thing (#2468)

Clay asked the committee to go over senro as one would fit a bicycle:
everything an app needs, nothing it doesn't, and one convention for each
job. The committee's reading is on #2468. It is built from an inventory of
every public name in senro's seventeen modules, the three example apps,
and what the generators write. It rules in eleven sections and orders the
work as six builds.

**Rulings built so far.**

- One name for a notice. The views took `flash` while the server said
  `notice`, and the views now say `notice` too. A scaffold's views are named
  for the plural, `notes_show.kata` and `notes_form.kata`. `visit/sees?`,
  `visit/sent_to?` and `passwords/stand_in_at` leave the public surface.
  The other names the inventory found no caller for outside senro's own
  specs stay public: those specs are how senro is driven
  (kanso-lang/senro#127).
- One call starts an app. An entry is `derive app/conventions`, a
  connection from `pg/from_env`, and
  `app/run "<name>" routes connected derived`. The new `app` module sits
  above `web`, `models`, `jobs` and `mail`, since `web` is below all
  three. A derivation can declare only plain data, and only a record's own
  module may build one, so `derived` is a map, and each migration in it is
  its version and its SQL. The port is `PORT`, 3000 when unset. A mail
  worker starts when the app has a mailer or calls one
  (kanso-lang/senro#128).

**Read again while building, and settled.**

- `models/integer?` and `wire/whole?` were listed for removal. senro's
  specs call both. The search that missed them ended in a word boundary
  after `?`, which never matches. Both stay.
- The resource controller keeps reading a field with `sent[n]` instead of
  `web/value`. A field the request leaves out is left out of an update,
  and `web/value` would read it as "" and blank it.
- The scaffold's refusal arms stay. Dropping them needs a refused write to
  bubble past the arm that reads the record. kanso lets a rescue open only
  a failure another package raised, and `models` and `web` are one package.
  A wrapper in every writing action would trade the arms for a call in
  each action.

Builds 3 (one way to answer and to read a request) and 4 (tables that
supply their id and timestamps) are in review as kanso-lang/senro#129
and its successor. The site follows the senro pin.

## 2026-11-12 — a module's own arm answers its bare call (#2473)

senro's notebook declares `fn id_of` in a module that imports `tokens`,
which exports an `id_of` of its own. A bare `id_of` there calls the
module's own function, and both engines run it, but the check refused the
caller: "this can be a none and `wanted` has no arm for it". The import's
`id_of` can answer none, and the module's own never does.

**The fault.** A name a module declares and one of its imports exports is
one overload group inside the module, and dispatch tries its arms in
order. `qualify` puts the module's own arms ahead of the import's, so an
importer runs them in that order. The module's own check merges its
dependencies in first, so there the import's arm stood ahead. Inference
then read the call's answer as the union of every arm the arguments could
reach, the import's included, and the none question read that union.

**Built.** Two changes, each needed. Where a module and its dependencies
are merged for the module's check, an import's twin of a name the module
declares goes in behind the module's own arms, as `qualify` orders them.
Inference stops at the first arm whose parameters are all binders: a later
arm answers that call only for a failure it names. A call that passes over
arms is marked by the arms it reaches, through `none_free`, the per-call
mark `math/round` already uses.

`tests/golden/own_arms_first` is the postcard from the issue, run on both
engines by `tests/own_arms_first.rs`. It was watched red with each change
taken out alone, both times with the refusal above.

**What the first push cost.** CI's reading of the first push put the
front end's peak on `compile_corpus` up 12,643 bytes (546,568 to 559,211)
and its allocations up 28. Built locally with each half of the change
alone, the merge carried all of it: it split the dependency's functions
with `partition`, which moves every one into a new vector while the old
one is still held. `extract_if` takes out only the twins that go behind,
and CI's second reading put the peak 84 bytes above main's (546,652).

The instruction rows stayed up about 0.7%: `compile_instructions`
22,642,405 to 22,800,026 on that reading. A callgrind diff of the check
of `compile_corpus` against main split the 159,052 instructions between
the two halves. The merge spent about 64,000 building a set of every
function name the module declares, and inference about 84,000 testing,
on every visit to every call, whether each arm's parameters were all
binders. Both are facts about the declarations. Which arms a call passes
over is now worked out once per group when inference starts, and the
merge looks up the larger of the two name lists in a set of the smaller
and keeps only the names both hold, which most modules have none of.
Locally that took the check from 159,052 instructions over main to
138,754, for 174 bytes more peak, which `welfare --score` prefers by a
hair. What is left in `eval_expr` is the new branch in the call loop,
which the precomputed table does not remove.
