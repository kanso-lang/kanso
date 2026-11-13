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

## 2026-11-12 — a derived file puts its types first (#2472)

A module's derived file was written in the order its definitions arrived,
so a derivation that wrote a function into a module ahead of another that
wrote a record there made a file the formatter refused: "canonical order
places type declarations before functions; move `thing` up". The app had
done nothing wrong, and the fix it was left with was to reorder its
`derive` lines. senro's pantry carried a comment explaining why
`models/tables` had to run first.

`load_with_definitions` now keeps a module's record types apart from the
rest of its definitions and writes them first. A definition joins the
types when its first line past its imports, blanks and comments opens
`type` or `pub type`. Within each half the definitions keep the order they
arrived in, so a file that was accepted before is written the same way.

The spec is `tests/golden/derive/a_type_defined_after_a_function_comes_first`:
one derivation writes `shown`, which takes a `thing`, and a second writes
the `thing` record and its maker. Without the fix both engines refuse the
derived file with the formatter's message above; with it they print
`thing 3`.

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

**Rows, from CI on 5c30213f8.** Every row below rose, and none of them
counts a program running: `compile_instructions` 22,642,405 to
22,778,659, `entry_instructions` 119,108,774 to 119,972,835,
`library_instructions` 63,609,875 to 64,026,941, `compile_peak_bytes`
546,568 to 546,826, `compile_allocs` 13,503 to 13,508,
`emit_instructions` 5,632,125 to 5,636,195, `interp_instructions`
741,096,980 to 741,118,066 (the interpreter's run includes its check of
the corpus), `interp_allocs` 1,067,941 to 1,067,977 and
`browser_compile_instructions` 412,081,396 to 412,743,106.
`browser_run_instructions` fell from 100,428,884 to 100,428,858. The
runtime veins did not move. The check now reads the program as it runs,
so the floor is lowered by what that costs, to 95.391044, under the rule
for the language working as specified.

## 2026-11-12 — gavel: senro's fit, builds 5b to 6 (#2468)

The rulings the committee made on #2468 after the first entry, each built
and merged in senro.

- **Templates name routes as actions do** (senro#132). A template writes
  `a href=(path_to "notes/show" r.id)`, and kata writes the path out while
  it derives the view, from the table `routes` builds, so the browser's
  views module imports nothing from the server. A name that is no route,
  or the wrong number of ids, is refused at the template's line. A paging
  link names only its query, `?page=2`, and `img src=(blob_path e.photo)`
  is written from `web/blob_path`.
- **One form of `path_to`** (senro#133). The record form, `path_to note`,
  found an arm only for a record declared beside a `show` action, and a
  scaffold keeps its record in the model's module. It goes, with the arms
  `rest` wrote for it. A path is named by the route's name and its ids.
- **The secret is found, not passed** (senro#134). An action is handed a
  `web/request` that carries the app's secret beside the parsed request.
  The secret is `pub secret` in the session module, which `routes/rest`
  puts in each row of the route table, and `SENRO_SECRET` replaces it when
  set. Twelve functions lose the argument. An app that keeps sessions and
  declares no secret is refused at the call. A secret never rides in a
  header, where an app that echoes headers would print it.
- **Signing in is one call** (senro#134). `accounts/attempt` checks the
  password, keeps the lock, counts wrong and right passwords, and answers
  `signed`, `inactive`, `locked` or `wrong`. Whether an account may sign in
  is a field, `active`, on the record the app builds, as Devise asks the
  model. The pantry writes through `item/update`.

Two fixes in kanso came out of building these: a module's own arm answers
its bare call ahead of an import's twin (#2473), and a derived file puts
its record types first (#2472). senro pins kanso d582d16c, which carries
both (senro#136). PRINCIPLES.md gained the section the review found
missing, "One way to do each thing" (senro#135).

## 2026-11-12 — senro answers a component's action as its page (#2455)

The last two items owed on #2455 are built. senro's design is cloud's (#2280), so the shape below is recorded here rather than put to the committee.

Auth in the framework: `senro generate authentication` writes confirmation by mail and the unlock link (senro#137, #138, #139). `accounts` signs both links, an account signs in once its address is confirmed, and the wrong password that locks an account mails the link that unlocks it. A test reads the mail its requests queued with `visit/mailed`. The guides followed in #2480.

Authorization at the page: a component's server action is now answered as the page it was posted from, as a provider call already was (senro#140). The page's script sends its path with the action. The server answers only from a page whose module leans on the component's, behind that page's sign-in, and only for a viewer its `admits` lets in. `derive kata/views` writes a `component` record into each server component's module, with the modules that lean on it, and an app lists them: `web/components routes [staple/component]`. That replaces `web/component act view`, which named every component's action and view by hand. The record's type is declared in the component's module, with no import, so a browser build of the module carries nothing of the server's. spec/serve/admitted acts on a tally from each page; with the check removed, its four refusals answer 200.

Rows: senro's bench/pages moved in CI. server_records went from 344,151 to 344,207, server_data from 1,130,160 to 1,130,145, and server_html from 3,087,836 to 3,087,821; the browser rows held. The bench's component runs in the browser, so it gets no record. The likeliest cause is kata/views now importing `data`, which has not been isolated. No kanso row moved.

Open: browser components still go through `browser/component act view`; giving them the same derived record is a small follow-up.

## 2026-11-12 — a literal applied as a function is refused where it is written (#2479)

A list written across lines takes one element per line, so a line holding
two strings reads as the first string applied to the second. No literal can
be called, and `kanso check` passed the line; both engines failed with
"`confirmations` is not callable" when it ran. In senro the same shape, in a
generator's list of modules, came out natively as an error naming an
unrelated function.

The checker now refuses a string, number, list or map at the head of an
application, at the literal's line and column. Inside a list the message
says the list takes one element per line:

```
error[type]: this element of the list is a string applied to 2 arguments, and a string cannot be called. A list written across lines takes one element per line
```

Elsewhere it reads "an int cannot be called, and here it is applied to 1
argument". A value called through a name, as in the runtime corpus's
`a_plain_value_is_not_callable`, still checks clean and fails when it runs,
because its value is not on the page. Two error goldens pin the refusal: the
issue's list and `x = 3 4`. main's binary checks both as `ok`.

**Rows.** The first cut ran the check as a pass of its own at every node and
cost the entry corpus's check 184,858 instructions in CI. The list case now
rides `call_shaped_at`, which already walks a list's elements, and the call
case rides `literal_argument_at`, which already takes an application apart.
What is left is the price of the refusal: `compile_instructions`
22,778,659 -> 22,780,737, `entry_instructions` 119,972,835 -> 119,985,418,
`library_instructions` 64,026,941 -> 64,031,825 and
`browser_compile_instructions` 412,743,106 -> 412,766,768. The check
refuses a program the language says is wrong, so the floor is lowered by
what it costs, under the rule for the language working as specified, as for
#2473: 95.39104366 -> 95.39101636. No runtime row moved. The compiler page
has a section for it, §303.

## 2026-11-12 — a browser component is listed by its record (senro#141)

The follow-up the last entry left open. A server component was listed by
the record `derive kata/views` writes into its module, and a browser
component was named by hand, `browser/component planner/act planner/view`,
under a name typed again in a map. kata now writes the record for every
component, and a browser module lists them as an entry lists its server
components: `browser/answer_pages [planner/component] views/rendered`.
`browser/component` is gone. A browser component's record names no pages,
since its actions never reach the server.

The record names the one template that draws a component's state, so a
component with two templates that bind actions is now refused on both
sides, where only a server component was before.
`spec/errors/component_drawn_by_two` is a browser counter with two such
templates; senro's main checks it clean.

Rows: senro's bench/pages moved. The first cut folded the list into a map
before the module read its event and read about 7,000 instructions more on
every browser row. Looking the component up when its event arrives, and
recording no pages for it, brought them to browser_start 13,643 -> 16,873,
browser_page 3,667,266 -> 3,670,780 and browser_act 115,229 -> 119,357, with
server_data 1,130,145 -> 1,130,451 and server_html 3,087,821 -> 3,088,133.
The derived code differs from main's only by the record and its type, and
taking out `std/list` or `browser`'s own type left start-up where it was, so
what remains is building a record of five fields where the module built one
of two and a map. No kanso row moved. The site's views guide quotes the new
lists, and the pin is d247faa.

## 2026-11-12 — gavel: the analysis, and no lint, carries a handed-out function's types (#2433)

A function handed out as a value has its parameters sent to every type,
because a value can be called with anything. A decoder whose position is
then not known to be an int keeps its record on the heap for its direct
callers too. Today the author's remedy is `p:int`, and nothing says it is
missing. The issue offered two directions, and the committee took the first.

**The lint is declined.** check.rs says kanso has no linter layer and refuses
what a linter elsewhere would warn about. A program without the annotation is
correct and only slower, so a refusal would refuse a program that runs.
Knowing that an annotation would change the answer also needs inference to
keep the direct callers' join apart from the value's, which is most of the
fix.

**Decision.** Inference keeps a handed-out group's types as its direct
callers prove them, and the value's wrapper takes anything. Under the
corpus-first gavel, the baseline lands first:
`a_decoder_handed_out_without_an_annotation_boxes_every_read` is
`a_decoder_handed_out_still_answers_its_callers_by_value` with `:int` taken
off `at`. It pins `sh_rec=3200` and `allocs=57` against its twin's 512 and
15, which is the issue's table. The fix scores as that fixture falling to its
twin. No other golden moved, and welfare does not read the mem vein. The
issue stays p3 because no benchmark pays this today (the measurement on
#2433, 2026-11-11).

**Copying the group does not reach it (measured the same day).** The
cheapest form of the decision is a source pass beside `flat::flatten`. It
copies a group that is both handed out and called from elsewhere, and sends
every mention by value to the copy. Built that way, the pass chose `at` and
the fixture still read `sh_rec=3200`. Inference joins a record type's fields
over every place that builds one. The copy builds `read bs[p] (p + 1)` with a
position of any type, so `read.next` widens, and `summed` hands that position
back to the direct `at`. No other mem golden moved, so the copy would only add
compile work, and it was not kept. What the fixture needs is for a function
value's parameters to hold what is actually passed to it at the calls made
through values, in place of every type. That is a flow analysis of function
values.

## 2026-11-12 — the dev tier stays at -O0 (measured)

The dev tier compiles a program at `-O0` and links the cached optimized
runtime. Its run of runbench costs three times the release build's, and
`dev_run_instructions` has improved least of any term (+13% on its
baseline). So I measured what optimizing the program module would buy, on
runbench's dev `.ll` (main at a759d16b, this container, clang 19.1.1). The
codegen column is the tool's own instructions on that one module.

| pipeline | codegen | run |
|---|---|---|
| `clang -O0` (today) | 2,886,553,773 | 3,057,358,794 |
| `clang -O1` | 14,923,691,533 | 1,847,473,765 |
| `llc -O0` | 2,891,930,318 | 3,148,019,518 |
| `opt -O1`, then `llc -O0` | 4,862,823,427 + 4,302,492,751 | 2,782,998,016 |
| `opt` with sroa, early-cse, instcombine and simplifycfg, then `llc -O0` | 1,103,876,794 + 2,815,114,850 | 3,267,742,793 |
| `llc -O1` (the same at `-O2`) | 7,474,749,318 | 2,365,328,093 |
| `llc -O0 -fast-isel=false` | 3,511,909,510 | 3,350,254,256 |

Most of what `-O1` buys comes from the backend: selection DAG and the greedy
allocator in place of fast instruction selection. A middle-end-only pipeline
recovers a fraction, and the short pass list makes the run slower. The
cheapest pipeline that moves the run is the backend at `-O1`: 25% off the run
for 2.6 times the codegen. At today's ratios that is about +0.007 on the
development side from `dev run speed` against about -0.017 from `dev
codegen`, whose satiation is 0.5. Plain `-O1` is worse, at -0.025. Under the
weights as written the dev tier stays at `-O0`.

## 2026-11-12 — gavel: senro ships as releases

Asked whether every senro change was on the site, the audit found the guides
level with senro's main except in one place. senro's README told an app to
get senro with `kanso install`, and that command refused it:
`kanso-lang/senro publishes no vX.Y.Z release tag`. `kanso install` resolves
a hako's highest release tag, and senro had none. Pinning to the branch with
`--from kanso-lang/senro@main` worked, and the README's example app checked
clean against what it fetched. Clay, 2026-11-12: "it definitely needs
release tags."

So senro is released. A session's push of a tag is refused with HTTP 403,
so senro tags itself: `VERSION` names the release, `0.1.0` to start, and a
workflow on each push to main tags the commit that first carries a version
(senro#143). Its first run tagged `v0.1.0` at dba26a72. A plain `kanso
install .` in a fresh app now prints `kanso-lang/senro v0.1.0`, and the
README's example app checks clean against it. senro#142's README says
`kanso install` fetches the newest release.

CLAUDE.md gains the rule that a merge changing what an app writes or runs
raises `VERSION`, and that the site's `senro-version` names a tagged commit.
The site now pins dba26a72. The command-line guide gains a section on
naming senro by repository, and its summary of `authentication` now names
the confirmation and unlock links senro#139 added.

## 2026-11-12 — directive: legacy route aliases are code

Clay, reading the routing guide: "I would prefer a legacy route aliases to
use code rather than the strings so that they are analyzed that compile time
obviously. get, put etc should be methods." An alias was a string map,
`{ "GET /signbook":"entries/index" }`, so the compiler saw neither the
action nor the method.

senro#144 makes each alias a call, `(routes/moved "/signbook"
entries/index)`, in the module's `legacy_route_aliases` list. The action is
a name, so the compiler resolves it. `routes/rest` reads each call through
the module's `calls`, as it reads `path_to`, and refuses a mistake at the
call. The methods are not functions. `put` and `delete` are taken by the
built-ins (`error[name]: the name put is already taken`), so two of the five
cannot be written, and an alias's action already fixes its method: the alias
takes the method of its action's route. The old map form is refused at the
declaration. senro's `VERSION` goes to 0.2.0 with it, since an app on 0.1.0
must rewrite its aliases.

The same question asked how senro spells member and collection routes,
deeper nesting and the rest of a Rails routes file. The routing guide gains
two sections. "actions beyond the seven" shows a member action as a nested
singular resource (`publication`, with `create` and `destroy` taking
`post_id`) and a collection action as a resource of its own (`searches`),
quoted from senro's new `spec/runs/actions_as_resources`. "what senro's
routing does not have" lists namespaces, renamed paths, nesting past one
level, constraints and hand-declared routes. A Readwise article on `derive`
and the conventions built on it went to Clay's reader.

## 2026-11-12 — the runner image moved to glibc 2.39-0ubuntu8.12

The cost-goldens job on kanso#2493, a docs change, refused every glibc-keyed
gate: the rows were measured on glibc 2.39-0ubuntu8.9 and the runner now
carries 8.12, with clang 19.1.1 and rustc 1.98.1 unchanged. main's last run
was on 8.9 and green, so main would refuse the same way if it ran again, and a
branch opened to carry the re-measure lands on the same new image. It rides
here, as the 8.8 -> 8.9 re-measure rode with its PR in September.

CI measured every row on 8.12 and none rose. Of the fifteen work rows six are
byte-identical and nine fell by one instruction (jsonbench, encodebench,
oneshot, widebench, digestbench, readbench, livebench, runbench and
runbench_dev). `codegen_instructions_dev` fell 117,027,868 -> 117,027,866 and
`codegen_instructions_release` 372,208,803 -> 372,208,799. The compile,
entry, library, start-up, interpreter, interpreter-memory and emit rows read
exactly their goldens. The rows are copied from the job log and the ten
measured-on lines name 8.12. The welfare floor is re-based by the rise the
revision gives, as a measurement of outside state (#2017), and its history
says so.

## 2026-11-12 — the book gains a chapter on derivations, with senro as its case study

Clay asked whether derivations had solid documentation. The book had one
section in the modules chapter: a routing example and three dense paragraphs.
Chapter 16, "conventions at compile time", is now the home. It builds a
command registry from nothing (`docs/book/samples/derive`, run and refused
by book_check), lists everything a derivation is handed and the five
declarations it may answer, states the rules it runs under, and reads four of
senro's derivations (routes, links and aliases through calls, records from
the schema, templates, job names) from senro's own source. A last section sets
it beside Haskell's deriving and Template Haskell, Rust's derive and
procedural macros, F# type providers and Zig's comptime. The modules chapter
keeps its example and points to the new chapter, so the details have one
home.

The senro excerpts are titled `senro/<path>`, which no file in this
repository holds, so book_quotes leaves them alone. senro_quotes now reads
`docs/book/derive.html` as well as `docs/senro`, checking only its `senro/`
panels against the pinned senro commit. Watched red with one word changed in
the `fn refused` excerpt, which it named by page and title.

The idea's origin is the #2186 ruling, 2026-10-28: Clay asked for "some
language construct that can allow you to do compile time mapping" so that
senro could place routes and tables by name and the compiler could check the
result.

## 2026-11-13 — specs written as blocks: the file is the describe (#2495)

**The question.** Clay asked what it would take for specs to read more
simply. His points: no `test_notes = describe …` wrapper, the outer
describe implied by the file, one level of `context`, and something in
the place of RSpec's `let` that a context can redefine while the setup
above it still sees the new value.

**The search.** #966 and #2464 kept test words out of the compiler, and
nothing in `std/spec` played the part of `let`. The effects arc in the
archive: the 2026-08-16 effect block made `=` name what an effect
yields, and the 2026-08-29 gavel, "effects are types, and the words are
the only doors", replaced it with no automatic bind. Chapter 02's
no-shadowing rule and its reason. An entry runs every binding before any
bare effect line, and a bound effect is a box.

**The decision**, by the committee, on #2495:
- A `*_test.kso` file whose top level holds `it` or `context` is a spec.
  Its subject is the module it tests.
- It nests one level.
- `it` takes an indented body. Each bare line of the body is a check,
  and the example passes when every check answers true.
- A binding at the top or in a context is a given, evaluated per example
  and only when read.
- A context may restate a given. Everything evaluated for an example in
  that context sees the restated value, and a restatement reads the value
  it replaces.
- An `around` may name what it hands on.

The one-level limit is what keeps the no-shadowing rule's reason:
there are two places to look for a name. Whether a given whose right side
is an effect names what it yields would reverse part of Clay's gavel, so
that went to him as #2496. Nothing here waits on it.

**Built.** `src/spec_form.rs` lowers a spec file, on the lexer's lines,
onto `std/spec`'s values:
- each example becomes a function holding the givens it resolved, in an
  order where each follows what it reads, then its body;
- the structure becomes a `describe` list;
- each hook's expression becomes a constant.

Tokens keep their spans, so a diagnostic about the author's code lands on
their line. The lexer counts `it "…"` and `context "…"` among its block
headers, so their indented lines stay lines. `std/spec` gains `every`,
which answers the first check that is not true.

**Pinned.**
- `tests/golden/test_verb/spec_blocks`: a given at the top built from a
  context's restatement, and a failing second check.
- `spec_hooks`: a named `around` read by givens.
- `spec_too_deep` and `spec_unread_given`: the two refusals.

Watched red: with a file given reading the file's own `stack` in place of
the context's, the restatement went unread and was refused at its line.
The book's `specs that nest` section and its replayed sample now teach
the block form.

**Rows, from CI.** `lib/spec/spec.kso` is compiled into the binary, so
three compile rows rose: `compile_instructions` to 22,781,295 (+558),
`entry_instructions` to 119,987,059 (+1,641) and `library_instructions`
to 64,033,359 (+1,534). Nothing fell.

The first CI run also read every work row 130 lower and both codegen rows
lower, and I took those rows and banked a rise. The second run, on a
tree that differed only in goldens and prose, read all of them at main's
values. The drop was that runner's host, so the claim is withdrawn and
those rows stay as they were. The spec form is a committee decision on
the language (#2495), so the floor moves down by exactly its cost. senro's own specs, and its testing guide that quotes
them, move when senro's pin includes this.
