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

## 2026-09-02 — the compile row moved 5,081 with nothing changed, and four entries above are wrong about why

This pull request changes `docs/compiler.html` and `design/compiler-log.md`.
CI read `compile_instructions=41,500,177` against a golden of 41,495,096.

The compiler's inputs are byte-identical between the two commits. `git diff`
is those two files; neither is compiled in, neither is reached by
`include_str!`, and `measured_on` confirmed the same rustc and the same
glibc. Every other vein in the same job matched to the digit: eleven work
rows, fourteen emitted counters, eleven text rows, `compile_allocs`,
`compile_memory`. One row moved, and nothing in the diff can have moved it.

**So the row's floor on this runner pool is at least 5,081.** The gate already
suspects this — it reads `scripts/gates/dispatch.sh` whenever the row moves
and says to re-run until the job lands on the recorded silicon — and kq spent
three pull requests in August learning the same thing about its own rows.
`measured_on` pins the toolchain, and the toolchain was never the whole host.

**This corrects four entries above, all filed today.** I recorded +6,087,
−416, +8,032 and −3,289 on this row as layout effects and wrote a causal
sentence around each: that the prelude's length moves where the bytes after it
land, and that the sign does not follow the direction of the edit. Two of
those readings are smaller than the 5,081 a no-op produced, and the other two
are the same size. The layout mechanism is real — a longer string in the
binary does move what follows it — but none of those four measurements
separates it from the runner, and I wrote as though they did. The honest form
of all four is "the row moved, inside a band the pool can produce on its own".

The pattern to notice is that the prose got more confident as the readings
piled up. By the fourth I was writing that four consecutive commits had moved
the row in an unpredictable direction and that this was what a layout effect
looks like — a story that explains the data and was never tested against the
null. The test cost one docs-only pull request and it was available all day.

**What would fix it.** Not a tolerance: a band is a guess that stays green
through the change it was written to catch. Record the dispatch block beside
the row the way `kq/bench/instructions_golden.txt` does, so a move gets asked
which silicon counted it before anybody explains it. That is a real piece of
work and it is not this pull request's; it is filed here so the next reading
of this row has the question in front of it.

**The re-run closed it.** The next job on this branch, with the golden
unchanged at 41,495,096 and nothing in the compiler touched, read 41,495,096
and went green. So the row is 41,495,096 / 41,500,177 / 41,495,096 across
three consecutive CI runs of the same compiler — bracketed, not a one-off.
The spread is 5,081 and it belongs to the pool rather than to any diff.

Writing that down matters more than the finding. The entry above corrects
four readings I took as evidence on a single measurement each; leaving this
one at a single measurement would have repeated the error inside the
correction.

## 2026-09-03 — the rewind's empty test paid a frame, and it is a third of a beat loop

Clay asked whether the fused chain operators had been swept in. They had not.
The day's four twins were driven by the eleven benchmarks' profiles, and a
fused adapter chain — `map`, `select`, `sum` deforested into one flat loop —
is not well represented in those. So I built one: 200,000 elements through
`list/map . list/select . list/sum`, and profiled it.

`k_beat_rewind` was 32.92% of it. Not the loop body, not a builtin — the
per-iteration rewind that reclaims the iteration's garbage.

That was not a property of the fixture. On escapebench the same function is
**43.66%** of the program: 80,970,118 of 185,475,445 instructions across
1,206,001 rewinds, 67 instructions each. On basket, 16.21%. The one function
was larger than any seam the four twins reached.

**Where the 67 went.** The disassembly says it in the first ten instructions:
six callee-saved pushes and a stack sub, before any work. The frame is there
for four loops — the buffer shelf's flush, and the chunk, view and permanent
registries — none of which runs on any of those 1,206,001 rewinds. Then
`k_buf_flush` unconditionally memsets the twelve-pointer shelf, six `movaps`
clearing a shelf that was already clear. What the common case actually needs
is three stores: `k_arena`, `k_arena_left`, `k_seek_str`.

**The change.** Two pieces, both idiomatic here. The shelf gets a dirty flag,
so its memset becomes one test for a program that never donates a buffer. And
`k_beat_rewind` splits: an inline empty test at the four call sites, with the
frame and the four loops left behind in `k_beat_rewind_slow`. That is the same
split `k_permreg_flush`/`_held` and `k_viewreg_migrate`/`_held` already make
one level down; it is worth more here because a beat loop rewinds once per
iteration where those run once per pop. `k_beat_iter` is now a leaf with no
frame at all, ~39 instructions on the fast path against 74 before.

| benchmark | before | after | |
|---|---|---|---|
| escapebench | 185,475,844 | 143,403,373 | −22.684% |
| basket | 45,028,689 | 41,211,873 | −8.476% |
| encodebench | 6,057,833,454 | 5,886,750,857 | −2.824% |
| oneshot | 34,844,691 | 34,417,154 | −1.227% |
| deepbench | 677,481,898 | 677,017,353 | −0.069% |
| indexbench | 5,242,731 | 5,242,087 | −0.012% |
| scanbench | 1,423,437,774 | 1,423,437,268 | −0.000% |
| pendbench | 715,729,140 | 715,731,365 | +0.000% |
| jsonbench | 2,533,005,144 | 2,533,091,327 | +0.003% |
| digestbench | 81,237,955 | 81,252,347 | +0.018% |
| widebench | 61,843,521 | 61,857,884 | +0.023% |
| **all eleven** | **11,821,160,841** | **11,603,412,888** | **−1.842%** |

The three small rises are programs with few beat iterations paying the inline
test at their pops where the old code paid a call. 86,183 instructions on a
2.5-billion-instruction decode is the largest of them. These are container
readings; CI supplies the rows that land in the golden.

`.text` rises on every row, 432 to 816 bytes — four copies of the test. The
header of `bench/text_golden.txt` says which benchmark paid most and which
gained most, and they are the same one.

**No counter moves, and the corpus was asked how much of that it can see.**
The three flush loops and the block retire are the only code in the rewind
that touches a statistic, so the fast path being taken exactly when all four
would do nothing means every cost golden stays byte-identical. All nine are.
But "byte-identical counters prove equivalence" is a claim about what the
corpus can observe, so I dropped each of the six conditions in turn and ran
the nine counter gates and the golden suite against each.

Three are pinned. Without the shelf's dirty test five counter gates
**segfault** — a shelf entry outliving its arena region hands a freed pointer
to the next grower, which is exactly the failure the flag exists to prevent.
Without the chunk registry's test, encode's counters move. Without the block
test, encode's and scan's move and four goldens fail.

Three are not: `k_chunkreg_spill`, `k_viewreg_n`, and the permanent registry's.
Dropping any of them leaves every gate and every golden green. The reason is
one fact, and I found it by instrumenting the rewind rather than guessing:
**on every program in the corpus, a rewind that finds a registry non-empty is
also a rewind that has moved on from its mark's block**, so the block test
reaches the slow path first and the registry tests never decide anything.
Counted over the benchmarks: `chunkreg_spill` is non-zero at no rewind at all;
escapebench has 3,000 rewinds with a non-empty permanent registry and the same
3,000 have changed block; encodebench has 400 with a non-empty chunk registry.

I wrote a fixture to isolate the view registry — a map born inside a beat and
asked for its sorted view — and it registered 500 views at 500 rewinds and
**still** did not catch the mutation, because those 500 rewinds had also taken
a new block. A fixture that cannot fail is worse than none, so it is not in
this pull request. The conditions stay: that the two travel together is a
property of the programs in the corpus, not of the code, and the next program
need not oblige.

**What is left.** The naming is the finding. `k_beat_rewind` did not show up in
any earlier profile sweep because the sweeps were reading the benchmarks whose
profiles the twins were chasing, and the beat machinery is not a builtin. The
question that found it was Clay's, about a construct the benchmark set does
not cover well. That is an argument for the fused chain having a benchmark of
its own, which it does not have and which the eleven do not substitute for.

**The fixture that started it, measured after.** The 200,000-element
`map`/`select`/`sum` chain goes 38,883,703 → 32,083,715, **−17.49%**, and
`k_beat_rewind` is off its profile altogether. What is left of the rewind is
`k_beat_iter` at 8,000,000 — 40 instructions an iteration where rewind and
iter together were 74. It is still 24.93% of that program.

So the seam is not finished, and the shape of what remains is visible: of
those 40, about twenty are the six condition tests and three are the stores
they guard. A single summary word per depth — maintained at the four
registration sites, read once here — would collapse six loads and six
branches into one. That is a second change and it waits for this one to land.
It is written down here rather than built now because the branch rule is one
open pull request at a time, and because the number above is what makes the
case for it.

**Two corrections to the paragraph above, both from building it rather than
reasoning about it.** The summary word collapses *four* conditions into one,
not six: the buffer shelf's flag is global rather than per depth, and the
block test is about arena state that no registration site can summarise. So
the fast path goes from six tests to three, not to one.

And the prototype was built and measured rather than left as a plan. A single
`k_beat_enc[depth]`, set at the four registration sites and cleared where the
slow rewind empties all three registries together, with all nine counter gates
green: escapebench 143,403,373 → 130,167,353, **−9.23%**; the fused chain
32,083,715 → 30,483,715, **−4.99%**; basket 41,211,873 → 40,299,752, −2.21%.
It is a second pull request, held behind this one by the branch rule, and it
is worth having.

**The ratchet caught the move, which is what it is for.** `k_seek_str = NULL`
went from one site to two — the slow rewind and the inline fast path — at
different indentations, and the cursor mutation's own guard reported that its
target had moved rather than silently deleting one of the two. Deleting one
would have left the running path resetting the cursor and the mutation would
have gone green while proving nothing. It now removes both, and reddens the
beat differential at 16 of 96 layout pairs, the figure its comment already
records.

**CI's rows, and they agree with the container.** The instruction vein is
regenerated from the runner, where the container readings above were a
direction rather than a pin. Every row lands within about four hundred
instructions of what the container said, which is the offset those two hosts
have carried all along.

| benchmark | before | after | |
|---|---|---|---|
| escapebench | 185,475,844 | 143,403,772 | −22.6833% |
| basket | 45,028,689 | 41,212,286 | −8.4755% |
| encodebench | 6,057,833,454 | 5,886,751,256 | −2.8241% |
| oneshot | 34,844,691 | 34,417,553 | −1.2258% |
| deepbench | 677,481,898 | 677,021,033 | −0.0680% |
| indexbench | 5,242,731 | 5,242,500 | −0.0044% |
| scanbench | 1,423,437,774 | 1,423,437,681 | −0.0000% |
| pendbench | 715,729,140 | 715,731,751 | +2,611 |
| jsonbench | 2,533,005,144 | 2,533,091,740 | +86,596 |
| digestbench | 81,237,955 | 81,252,746 | +14,791 |
| widebench | 61,843,521 | 61,858,297 | +14,776 |
| all eleven | 11,821,160,841 | 11,603,420,615 | −1.8420% |

Four rows worsen and each is stated here as required: **jsonbench** rises
**86,596**, **digestbench** rises **14,791**, **widebench** rises **14,776**,
**pendbench** rises **2,611**. All four are programs with few beat iterations
— 151, 56, 40 and 134 — that now pay the inline empty test at their pops
where they used to pay a call, and none of the four is a tenth of a per cent.
They are the price of escapebench's 42 million and encodebench's 171 million.

**compile_instructions** reads **41,500,974** against a golden of 41,495,096,
a rise of **5,878** with nothing in the front end touched by this change. The
entry above it in this log established that this row moves 5,081 on a
docs-only pull request with byte-identical compiler inputs, across three
consecutive runs of the same compiler. 5,878 is that band. It is regenerated
rather than explained, and the reason it is not explained is the finding
recorded yesterday: nothing here separates a layout effect from the runner
pool, and writing a causal sentence around a number this size is the error
that entry corrects.

**The five worsened counters, named as the gate spells them.**
`work_jsonbench` lands on **2,533,091,740**, `work_digestbench` on
**81,252,746**, `work_widebench` on **61,858,297**, `work_pendbench` on
**715,731,751**. Those four are the programs with 151, 56, 40 and 134 beat
iterations: too few for the fast path to repay, so they pay the inline empty
test at their pops where the old code paid a call, and the largest of the four
is 0.018% of its program. `compile_instructions` lands on **41,500,974**,
inside the pool band the entry above measured.

`text` lands on **1,023,750** against 1,016,742 — the eleven binaries together
grow **7,008 bytes**, 432 to 816 each, which is four copies of a twenty-
instruction test at the four rewind call sites. That is the trade this change
is: seven kilobytes of machine code for 217,740,226 instructions.

## 2026-09-03, LATER — the compile row's move is glibc's allocator, and that corrects the task filed for it

The entry above pinned `compile_instructions` at 41,500,974 from CI. The next
run of the **same commit's front end** read **41,495,850**. Five readings now
exist for a compiler nothing has touched:

    41,495,096   41,500,177   41,495,096   41,500,974   41,495,850

Two clusters about 5,228 apart, each internally within 800. Yesterday's entry
called this "the pool" and declined to explain it, and filed a task to record
the dispatch block beside the row the way kq does, so a move could be asked
which silicon counted it.

**That task's premise is wrong, and this is the measurement that says so.**
CI prints the profile on every run, so the two runs can be compared function
by function:

| symbol | 41,500,974 | 41,495,850 | delta |
|---|---|---|---|
| `kanso::infer::eval_expr'2` | 1,616,100 | 1,616,100 | 0 |
| `kanso::check::check_merged` | 1,598,577 | 1,598,577 | 0 |
| `_int_malloc` | 1,554,268 | 1,551,398 | **−2,870** |
| `_int_free` | 1,516,161 | 1,516,378 | **+217** |
| `__memcmp_avx2_movbe` | 1,346,853 | 1,345,486 | **−1,367** |
| `HashMap::insert` | 1,302,885 | 1,302,885 | 0 |
| `kanso::infer::infer` | 1,234,092 | 1,234,092 | 0 |
| `malloc` | 1,113,407 | 1,113,407 | 0 |
| `kanso::infer::eval_expr` | 876,900 | 876,900 | 0 |
| `kanso::lexer::lex_line` | 857,892 | 857,892 | 0 |
| `free` | 711,340 | 711,340 | 0 |
| `kanso::parser::parse` | 591,666 | 591,666 | 0 |
| `kanso::mentions_in_expr'2` | 533,848 | 533,848 | 0 |

**Every symbol in the compiler is identical to the instruction. Three glibc
symbols move and nothing else does.** And both runs took the same dispatch —
`__memcmp_avx2_movbe` on each — so recording the dispatch block would have
found the two runs indistinguishable and explained nothing. The silicon
hypothesis is dead, and it is dead by the same test that killed the layout
one: comparing against the null instead of fitting a story to a number.

What is left is glibc's allocator. `_int_malloc`'s bin walks depend on the
heap's starting layout, and the allocation *sizes* cannot be what differs
because the compiler's own counts are byte-identical. The most likely
remaining cause is that the two runs ran binaries whose data and bss differ
slightly, moving the initial break — which changes malloc's work without
changing a single instruction the compiler executes.

**The row is red on main too.** `bench/compile_instructions_golden.txt` holds
41,495,096 and this runner reads 41,495,850, so a pull request that changed
nothing at all would fail this gate on this runner. It is not this change's
regression, and this branch does not carry a bump for it: the golden is left
at main's value.

**What would actually fix it**, and it is a decision rather than a repair:
the row should count the instructions attributed to the kanso binary rather
than the process. That is what the gate's own header says it measures — "what
the FRONT END costs to run" — and it is the part that is deterministic. On
this container three consecutive runs of the box give 41,904,811 exactly, and
the per-object split is 33,586,490 in the compiler against 7,982,541 in libc;
it is the second number that moves on the runner. Changing what a published
counter measures is Clay's call, not a session's, and it is filed as one
rather than done here.

**A sixth reading, and it repeats a previous one exactly.** The run after the
entry above read **41,500,974** — the same value, to the instruction, that a
run two heads earlier produced. Six readings of an untouched front end now
give four distinct values, two of them seen twice:

    41,495,096  ×2      41,495,850  ×1
    41,500,177  ×1      41,500,974  ×2

That is not continuous noise. Noise does not land on the same eight-digit
number twice in six tries. It is a small set of discrete environments, each
deterministic within itself — which is also what the container says, where
three consecutive runs of the same box give 41,904,811 every time.

So the row is deterministic per environment and the pool holds several. The
comparison in the entry above rules out the obvious discriminator: the two
runs it examined took the same `__memcmp_avx2_movbe` dispatch and differed
only inside glibc's allocator. Whatever separates the environments, it is
finer than the dispatch block and it does not touch a single instruction the
compiler executes — every `kanso::` symbol was identical across the pair.

This sharpens the question filed for Clay rather than changing it. A row that
is deterministic per environment and varies across a pool of them can be
pinned two ways: record the environment, or stop counting the part that
varies. The first needs a discriminator nobody has found yet — the dispatch
block is not it. The second is available now and is what the gate's header
already claims to measure.

**And it has already failed on main.** The claim above — that a pull request
changing nothing would fail this gate on some runners — was an inference from
the readings. It does not need to be: main's own `ci` run at **9541196f**, on
2026-09-02, failed with `compile instructions disagrees with its golden` and
every other row in that vein green. That commit is a merge that had just
regenerated the row, so the golden it was checked against was its own.

So the gate has already gone red on merged history, on a commit whose author
had just pinned the number it was checked against. Whatever this row is
measuring, it is not something a branch can be held responsible for, and the
seventh reading on this branch — 41,500,974 again — is the fourth head in a
row to say so.

That is the whole of the case for leaving the golden at main's value and
sending the question on rather than pinning a number that will be wrong on
the next runner.

## 2026-09-03, LATER STILL — the silicon hypothesis was right and I killed it on the wrong evidence

The gate now prints the binary that counted the row. It answered a different
question than it was aimed at, and the answer is that two entries above are
wrong.

The two CI runs compared earlier ran on **different CPUs**. The gate has been
printing the dispatch block all along and both blocks were in the job logs;
they differ:

| field | run at 41,500,974 | run at 41,495,850 |
|---|---|---|
| `features[0x2].cpuid[0x0]` | 0xa10f11 | 0xa00f11 |
| `features[0x5].cpuid[0x1]` | 0x30100015 | 0x30000015 |
| `level2_cache_size` | 0x100000 | 0x80000 |
| `rep_movsb_stop_threshold` | 0x100000 | 0x80000 |

**What I did wrong.** I compared which `memcmp` implementation glibc had
selected — `__memcmp_avx2_movbe` on both — and concluded the two runs were on
indistinguishable silicon. That is the wrong field. The selected function is
one output of the dispatch; the CPUID word and the cache-derived tunables are
others, and those differ. I then wrote that "the silicon hypothesis is dead,
killed by the same test that killed the layout one" — which is the same error
the layout entry corrects, committed in the act of claiming not to.

The pattern is now three for three: a number moved, I reached for a mechanism,
and the mechanism was asserted from evidence that did not separate it from the
alternative. Twice the mechanism was wrong. The one thing that has worked every
time is comparing two runs field by field and reading what actually differs.

**What follows.** Task #244 was right and its closure was not. Recording the
dispatch block beside the row is the fix, exactly as `kq/bench/instructions_golden.txt`
already does and for the reason kq#86 already gave. The row is deterministic
per CPU and the pool holds at least two; pinning it per recorded silicon is
the shape that works.

That also means the two invasive options are off the table, and Clay's
instinct to keep libc counted survives intact: libc's cost stays in the row
because it is a real cost of how this compiler allocates, and the row stops
lying because it says which chip counted it.

**What is NOT claimed here.** How a different L2 size produces 2,870 fewer
instructions inside `_int_malloc` is not established. `_int_malloc` does not
call memcpy, and the mechanism could run through heap addresses, alignment,
or something else entirely. What is observed is that the environments differ
in a way the gate already records, and that is enough to attribute the row
without inventing the chain. Inventing the chain is what went wrong twice.

**A second cause is live and I had not checked it either.** Before pinning a
row per chip, one thing had to be ruled out and was not: `src/runtime.c` is
`include_str!`'d into the compiler. Every commit on this branch changed it, so
every head built a different compiler binary — different bytes, different
place for the heap to start — without altering one instruction the front end
executes when it checks a library.

So two candidate causes are live at once, the silicon and the binary, and the
readings so far cannot separate them. Four values, two identified chips, and
several different binaries across the heads that produced them. Pinning per
chip would be building on the same kind of half-checked story that has now
been wrong twice, so it waits.

What is added instead is one greppable line per run — cpu, binary sha, row —
so three runs settle it: same cpu and same sha with different rows means it is
neither; same cpu with the sha tracking the row means it is the binary; the
same sha on two cpus tracking the row means it is the silicon. That is the
measurement the last two entries should have started from.

## 2026-09-03 — RETRACTION: the compile row IS this change's, by 5,621, and it falls

Three entries above say this row's movement is not this branch's, and one of
them says main's own history settles it. That was wrong, and the experiment
that shows it took four minutes and should have been the first thing run.

Same container, same toolchain, same chip, same library box, only `src/`
differing between the two arms, two runs each:

    my src/     sha 52b28b027c23    41,904,811    41,904,811
    main's src/ sha 1780ba089b7f    41,910,432    41,910,432

**−5,621, and it repeats to the instruction.** `src/runtime.c` is
`include_str!`'d into the compiler; this branch grew it by fifty lines; the
compiler's bytes moved and the front end's counted work fell. That is a real
movement of this row caused by this diff, and a fall is a win to bank — which
is exactly what the gate has been asking for since the first red.

**The first attempt at this experiment was also wrong and nearly got
announced.** It restored `src/` without rebuilding, so both arms measured the
same binary and agreed; the agreement looked like a null result. The fix was
to print the binary's sha on every measurement rather than trust that a
checkout implies a build. Every measurement above carries its sha for that
reason.

**What stands and what does not.** The chip variance is real: every head on
this branch from f8fd75cb onward carries identical `src/`, and CI read both
41,500,974 and 41,495,850 across them, twice each. So the row moves about
5,124 with the silicon and about 5,621 with this diff, the two are the same
size, and that is why they were confused. What does not stand is "not this
PR's": it is this PR's, and separately it is also the pool's.

**What that costs.** The golden was regenerated to 41,500,974 and then
reverted on the strength of the claim now retracted. The revert was the wrong
move. The row still cannot be pinned to a single value while the pool holds
two chips — #247 is unaffected — but this branch owed a regeneration and a
sentence saying the front end got cheaper, and it said the opposite instead.

**The pattern, stated once more because this is the fourth.** Four times today
a number moved, a mechanism was reached for, and the mechanism was asserted
before the experiment that separates it from the alternative. Layout, then
silicon-is-dead, then silicon-is-back, now this. The experiment has been cheap
every single time. The rule that would have caught all four: when a number
moves, change exactly one thing and measure both arms, before writing a word
about why.

## 2026-09-03 — the compile row gets a key, and a refusal you can watch

Searched the log tail and `design/log/compiler-log-archive.md` for prior
attempts at per-host instruction rows: kq#85 and kq#86 (the rows say which
silicon counted them), and the `dispatch.sh` header's record of a
"record one block, refuse elsewhere" design that CI killed in two runs. This
is the third shape and the first that neither refuses everywhere nor skips.

**What the row now is.** `bench/compile_instructions_by_cpu.txt` holds one
value per chip, keyed by `scripts/gates/dispatch.sh key` — family and model
and nothing else, so a firmware revision cannot move a row that is otherwise
right. The gate reads the row for the silicon it landed on.
`bench/compile_instructions_golden.txt` keeps its bare
`compile_instructions=`, because welfare, the trend gate and `golden_prose`
all read that file for one number, and the gate checks on every run that the
bare line equals the table's first row. That check costs two file reads and
catches the drift nothing else in the tree can see: welfare reads only the
golden, the gate reads only the table, and a re-sitting that updated one of
them would leave the objective tracking a number no chip counted.

**Four ways to be wrong, one way to be right, and none of the four is a
warning.** No rows at all; the golden drifted from the reference row; this
chip has no row; the row moved on the chip that counted it. The third is the
one the design turns on. Skipping there is the cheap answer and it is the
answer CI already rejected: three runs in four land on a cpu that is not any
given recorded one, so most regressions would go through, and this harness's
own mutations redden these same gates, so on those runs its rows would go
blind.

**Why not a band.** The chip moves this row about 5,124 and the beat-rewind
split moves it −5,621. A tolerance wide enough to swallow the first swallows
the second, which is the whole signal. Pin the number, key the noise.

**Watched failing.** The four refusals live in
`scripts/gates/compile_ir_row.sh` rather than inside the gate, because the
gate's own answer costs a callgrind run over the whole front end and refuses
outright on any host whose toolchain is not the recorded one — on a container
it is red before it reads a row at all, which is a gate no spec can drive.
Split out, it is two files and four strings.
`tests/a_compile_row_is_read_against_its_own_chip.rs` drives all five outcomes
in forty milliseconds. Three breaks, each reddening exactly the specs it
should and no others: waving an unrecorded chip through (1 red), dropping the
golden/table drift check (2 red), and turning the row comparison into a
±10,000 band (2 red). The first is now a ratchet row, `compile_ir_keyed`, on
the specs job.

**The front end got 5,621 instructions cheaper on this branch**, and this is
where that is stated rather than only inside the retraction above. The
same-chip experiment is in the previous entry. It is a fall and it is banked;
the golden and the table land on the value CI reads, one chip per run.

**What this costs, said plainly.** A change that moves the front end
invalidates every chip's row at once, and only CI may write them — the values
belong to its glibc and its rustc, which `measured_on.sh` pins on the table as
well as the golden. So such a change takes several red pushes to re-pin, each
printing the exact line to paste. The table therefore ships with no rows: the
first runs say what to add. The alternative that costs nothing is a row that
reads the runner.

## 2026-09-03, LATER — the first chip's row, and it is a fifth cpu

Searched the log tail and the archive for the pool's known cpus before calling
this one new: `dispatch.sh`'s header and §34 of the page name four — an AMD
EPYC Zen 3 (family 0x19 model 0x1), an Intel Ice Lake-SP (0x6/0x6a), an AMD
Genoa, and the Cascade Lake (0x6/0x55) this container is. The run that counted
today's row is **family 0x6 model 0xcf**, which is none of them. Five cpus in
the pool, not four, and the gate found the fifth by refusing rather than by
anybody going looking.

    family0x6-model0xcf 41500974

That is the first row of `bench/compile_instructions_by_cpu.txt`, and by the
rule in its header it is the reference series, so
`bench/compile_instructions_golden.txt` moves with it:

    compile_instructions 41,495,096 -> 41,500,974

**This is a re-basing and not a regression, and the difference matters.** The
old value was counted on a chip nobody recorded, on main. The new one is
counted on 0x6/0xcf, on this branch. Subtracting them is exactly the operation
the keying exists to forbid — 5,878 is the size of the chip effect and the size
of this branch's real move, which is why the two were confused for a day. What
this branch does to the front end was measured the only way that answers it:
same container, same toolchain, same chip, only `src/` differing, binary sha
printed on both reads, **-5,621 repeating to the instruction**. That
measurement stands and this row does not bear on it.

**What is now pinned and what is not.** One chip of five has a row. The other
four refuse until CI lands on them, printing the line to paste. Until then this
gate is red on four runs in five, which is the bootstrap cost the table's header
states rather than hides. `compile_binary sha256=74abf73ef677` and
`.text=2513746` are printed beside the row, so a later disagreement on this same
chip can be asked whether the binary moved.

**One thing to watch.** While the table lacks a chip's row the gate is red for a
reason no mutation caused, and the ratchet's `prove` reports BLIND only when a
gate stays GREEN — it never checks the gate was green before. So `compile_ir`
proves nothing on a nightly that lands on an unrecorded chip. That hole is older
than this change (`compile_ir_host_unpinned` has always had it, since
`measured_on` can refuse on its own) and it is filed rather than fixed here,
because a green-before run roughly doubles the nightly's setup-and-gate work and
wants that cost measured in its own pull request.

## 2026-09-03, LATER STILL — two chips, one commit, 5,124 instructions apart

Searched the log tail and the archive before writing this down as a
measurement rather than an inference: the 5,124 figure appears above as
"the row moves about 5,124 with the silicon", derived from CI reading
41,500,974 and 41,495,850 across heads that carried identical `src/`. That was
an inference from two unlabelled readings. This is the same quantity with the
labels attached.

Two runs, one commit — `22840458`, whose diff is a table, a golden and a log
entry, none of which the compiler reads:

    family0x6-model0xcf    41,500,974
    family0x19-model0x1    41,495,850

**5,124, and the chips are named this time.** An Intel Emerald Rapids against
an AMD EPYC Zen 3, one libc, one binary's worth of source, and glibc choosing
different code at load time on each. Nothing about the compiler differs
between those two numbers.

Set that beside what this branch does to the front end, measured on one chip
with the binary's sha printed on both reads: **-5,621**. The noise and the
signal are within ten per cent of each other in size. A band that covered the
first would have hidden the second entirely, which is the whole argument for
keying the row rather than widening it, now standing on a measurement instead
of on two readings and a guess.

Two of five chips have rows. The remaining three refuse until CI lands on them.

## 2026-09-03 — the summary word, and an inline the caller's budget took away

Searched the log, the archive and design/ for prior art on the rewind's
condition tests and on inlining budgets before filing: the archive carries the
`k_permreg_flush`/`_held` and `k_viewreg_migrate`/`_held` splits and #1217's
noinline finding, and neither covers this.

`k_beat_iter`'s fast path read four per-depth registries — `chunkreg_n`,
`chunkreg_spill`, `viewreg_n`, `permreg_n` — to answer one question: is there
anything at this depth. `k_reg_any[d]` answers it in one load, each registry
owning a bit and clearing its own. A summary any of them could clear goes stale
the first time a pop empties one and leaves another holding entries.

**The first working version was a regression, and the reason was not the change
itself.** encodebench rose 50,161,245 against a 13,079,983 fall on escapebench:
net +37M, +0.32% across the eleven. The profile diffed by function:

```
+50,719,623  k_viewreg_migrate      out of line
+41,898,819  k_permreg_migrate      out of line
-40,255,992  k_beat_iter            the inlined cost leaving
 -2,205,210  k_beat_pop
```

Both wrappers had been inlined into `k_beat_iter` and were now real calls.
Shrinking `k_beat_rewind` changed the inliner's budget IN ITS CALLER, so
removing work pushed two hot wrappers out of line and paid four times the gain.
Moving the summary's clears out of the flush bodies did not bring them back —
encodebench stayed at +50.2M. The trigger is the caller's budget, not the
callee's size, which is why a smaller callee did not help.

`always_inline` on the three wrappers states what the baseline was already
doing. CI's rows, all eleven:

| benchmark | before | after | |
|---|---|---|---|
| escapebench | 143,403,772 | 130,170,751 | −9.2278% |
| basket | 41,212,286 | 40,300,172 | −2.2132% |
| encodebench | 5,886,751,256 | 5,848,702,451 | −0.6463% |
| oneshot | 34,417,553 | 34,322,446 | −0.2763% |
| deepbench | 677,021,033 | 676,465,730 | −0.0820% |
| indexbench | 5,242,500 | 5,242,363 | −0.0026% |
| digestbench | 81,252,746 | 81,252,316 | −0.0005% |
| scanbench | 1,423,437,681 | 1,423,437,576 | −0.0000% |
| jsonbench | 2,533,091,740 | 2,533,092,019 | +279 |
| pendbench | 715,731,751 | 715,732,938 | +1,187 |
| widebench | 61,858,297 | 61,890,181 | +31,884 |
| **all eleven** | **11,603,420,615** | **11,550,608,943** | **−0.4551%** |

Welfare 75.95 → 76.01, ratcheted in the same commit. `text` falls
1,023,750 → 1,018,166, five and a half kilobytes across eleven binaries: the
code each call site gains by inlining is more than paid for by the three
wrappers no longer standing as functions of their own.

The pins are load-bearing and invisible to every vein but this one — the counter
gates are byte-identical whether a call is inlined or not, and the emitted
golden counts what the compiler wrote rather than what llvm did with it. So
`an_inline_pin_the_inliner_undoes.sh` strips all three, watched red before
green: encodebench 5,848,702,052 → 5,936,910,092 on the container.

**And the compile row moved without the front end moving.** `compile_instructions`
went 41,495,850 → 41,498,829 on the same chip, family0x19-model0x11, +2,979.
`src/main.rs` embeds the runtime with `include_str!("runtime.c")`, so a runtime
change changes the compiler binary while changing no compiler code: `.text`,
`.data` and `.bss` are byte-identical at 2,513,746 / 2,648 / 304 across the two
runs and only the sha differs. The embedded source grew, the rodata after it
shifted, and the front end does the same work at different addresses.

That is a second thing this row responds to, beside the silicon #1226 keyed it
by, and the two are not separable by keying: any runtime edit invalidates every
chip at once. The Intel and Zen 3 rows were removed rather than carried
forward, because a value measured against the old binary is worse than none.
They are re-sittings when they next refuse, not new chips, and the table's
header now says so.

Three counters worsened and are priced here. `work_jsonbench` lands on
2,533,092,019, up 279 instructions on a 2.5-billion-instruction program.
`work_pendbench` lands on 715,732,938, up 1,187. `work_widebench` lands on
61,890,181, up 31,884 — the largest of the three at 0.0515% of its own row.

All three are the same effect, and it is the one the fast path is for: a
program that rewinds rarely never reaches the case the summary word makes
cheap, so it pays the word's store at its registrations and collects nothing
back. widebench registers the most of the three, which is why it pays the most.
The eleven together fall 52,811,672 instructions, so the trade is 33,350 spent
against 52,845,022 saved — a ratio of about 1,585 to one, and the same shape as
kanso#1226's four risers.

One more thing the gate caught, and it is worth stating because the trend gate
cannot: `bench/compile_instructions_golden.txt` carries a bare line that
welfare, the trend gate and golden_prose all read, while the by-cpu table is
what the compile gate reads. Removing the Intel row left the bare line at
41,500,974 — a number no remaining chip had counted — and the gate refused on
exactly that, which is a coherence check worth having.

The bare line is 41,498,829 now, the first row of the table. To the trend gate
that reads as a fall of 2,145 and therefore an improvement, and it is not one:
the reference row changed identity from the Intel chip to the Zen 4, and the
two were never comparable. Nothing about the front end got faster. A gate that
reads one number cannot see a change of what the number is OF, which is the
same shape as the runner-versus-silicon confusion kanso#1226 fixed, one level
up.

The re-sitting falsified the claim this table's header had been carrying since
kanso#1226, which is the best thing a re-sitting can do. That header said the
family-and-model key was FINER than the effect, on the evidence that Zen 3
(0x19/0x1) and Zen 4 (0x19/0x11) both read 41,495,850 while the Intel read
41,500,974 — glibc's resolver picks memcpy and memcmp by feature set, so two
models sharing a feature set should share a number.

On this binary they read 41,503,893 and 41,498,829. They differ by 5,064.

The agreement was a property of that binary's layout rather than of the
silicon, and a key coarse enough to merge the two AMDs — which the header
argued against on other grounds, and which I had half-talked myself into —
would from this commit onward have carried one model's number for the other.
Two models sharing a value once is not evidence that they share a value. The
header now says that instead.

I also guessed wrong about which chip this run landed on, reasoning from the
arithmetic (41,503,893 - 41,500,974 = 2,919, close to the Zen 4's +2,979) that
it must be the Intel. It was the Zen 3, whose old row happened to sit 5,124
below. The gate prints the key; there was no reason to infer it.

## 2026-09-03 — the per-chip key was never sufficient, and the tunables are pinned

kanso#1226 shipped a compile_instructions row keyed by CPU family and model,
on the reasoning that glibc's ifunc resolver picks memcpy and memcmp by feature
set. kanso#1227 broke that. On ONE unchanged binary, CI produced exactly two
values:

  41,498,829   family0x19-model0x11 (Zen 4), family0x6-model0x6a (Ice Lake)
  41,503,893   family0x19-model0x1  (Zen 3), family0x19-model0x11 (Zen 4 again)

The same family and model counted both. The profiles fall into two clusters,
identical within each and differing in three places:

  _int_malloc          1,551,398   1,554,268
  _int_free            1,516,378   1,516,161
  __memcmp_avx2_movbe  1,346,206   1,347,513

The memcmp IMPLEMENTATION is the same in both — the resolver picked identically
— and its instruction count still moves, which is an alignment difference.
malloc moves too. So the variable is heap layout, upstream of dispatch, and the
feature-set story explained the wrong layer.

The gap is 5,064. The 2026-09-02 entry recorded this row moving 5,081 on a
docs-only PR with byte-identical inputs and read it as noise with a band. It was
neither noise nor the chip. Two wrong readings of the same number, a day apart,
and both times the mechanism was fitted to whatever data was in hand.

WHAT IS PINNED NOW. glibc sizes its malloc and string thresholds from the cache
sizes it reads out of the CPU, so two machines of one model with different
caches lay the heap out differently. compile_instructions.sh sets the cache
sizes, the three string thresholds, and four malloc knobs through
GLIBC_TUNABLES, so every run takes the same path on every host. Every recorded
row was cleared, because all of them predate the tunables.

This measures a configuration no user runs under, and that is the price. The
row exists to compare the compiler against itself across commits; a number that
cannot be compared measures nothing, and a number that silently means two
different things is worse than one that refuses.

WHAT THE NEXT RUNS TEST, and it is falsifiable either way. If pinning was the
right explanation, every chip lands on one value and the per-chip key becomes
vestigial — at which point the file collapses to a single golden and the
compile_ir_keyed ratchet row goes with it. If the chips still disagree, pinning
was wrong too and the tunables come back out. Clay had no preference between
this and keying by the full tunable block; this one is chosen because it removes
the variable rather than describing it, and because one CI run falsifies it.

FIRST READING UNDER THE PINNED TUNABLES, and it supports the diagnosis without
settling it. family0x19-model0x1 counts 41,631,006. Its profile reads

  _int_malloc          1,551,398
  _int_free            1,516,378
  __memcmp_avx2_movbe  1,346,206

which is the LOW cluster's three numbers to the instruction. That chip sat in
the high cluster before pinning, so the tunables moved it, and moved it onto
the other cluster rather than somewhere new. That is what a controlling
variable looks like. One chip is not convergence; the next distinct chip
decides it.

`compile_instructions` is priced here at 41,631,006, up 132,177 on the
41,498,829 the same measurement gave unpinned. The compiler did no more work:
the pinned thresholds are not any machine's native ones, so the run takes a
memcpy path no host would have chosen and __memcpy_avx_unaligned_erms enters
the profile's top fifteen at 619,555 where it was absent before.

THE OBJECTIVE'S BASELINE WAS RESCALED RATHER THAN THE FLOOR LOWERED. welfare
weighs baseline over current, so a changed instrument reads as a regression it
is not. The baseline moves 56,848,763 -> 57,029,831, the ratio it stands for
held to nine decimal places at 1.369888, and the number is 76.01 before and
after. Landing day moves it by nothing, which is the rule this file already
states for a counter entering at its dimension's standing.

Re-granting was tried first and rejected: dropping the baseline so welfare
re-grants at standing paid 1.37 points that no work earned. Granting at
standing is neutral for a counter ENTERING the model and a gift to one already
in it. The rescale is the honest form.

SECOND PINNED READING, and it does not decide the experiment. family0x6-model0xcf
counts 41,635,958 against family0x19-model0x1's 41,631,006 — 4,952 apart, where
the unpinned clusters were 5,064 apart. Similar magnitude, shifted level.

It is tempting to read that as the tunables having failed, and that reading
would be wrong. The defect being chased is ONE CHIP giving TWO values. What is
in hand now is two chips giving two values, which is what per-chip determinism
looks like. A single reading per chip cannot tell "pinning fixed the flapping"
from "pinning only moved the level", because both predict exactly this table.

The deciding observation is a repeat on a chip that already has a row: matching
its own recorded value is the success case, disagreeing with it is the failure
case, and the gate reports either without being asked. So the rows accumulate
and the next collision answers it. Naming the falsifier in advance is the point
— it is the same discipline as watching a mutation go red before it goes green,
and it is what was missing when the per-chip key was declared to work on two
agreeing chips.

THIRD PINNED READING, and it is the first real evidence. family0x19-model0x11
counts 41,631,006 — family0x19-model0x1's value to the instruction. Those two
AMD models DISAGREED before pinning, at 41,498,829 and 41,503,893, and each of
them had also produced the other's number on a different run. Under the pinned
tunables they agree.

  family0x19-model0x1    41,631,006   Zen 3
  family0x19-model0x11   41,631,006   Zen 4
  family0x6-model0xcf    41,635,958   Intel, 4,952 away

So the tunables removed a source of variation that was real, and what is left
is a stable Intel/AMD difference. That is the shape kanso#1226 originally
claimed and could not support: dispatch differing by feature set, with the two
AMDs together. The mechanism was not wrong so much as drowned — heap layout
moved the number by 5,064 and dispatch by 4,952, the two were the same size,
and with one reading per chip they were indistinguishable.

What is still NOT established is the thing named as the falsifier last commit:
no chip has yet been read twice under the tunables. Three chips with three
readings and two values is consistent with per-chip determinism and also with a
coin that has landed the same way twice. The next collision on a recorded chip
is what settles it, and until then this is evidence rather than a result.

THE FALSIFIER FIRED, AND IT PASSED. A run landed on family0x19-model0x1, which
already had a row, and counted 41,631,006 — its own recorded value. That chip
had produced BOTH 41,495,850 and 41,503,893 under the unpinned measurement. The
intra-chip flapping is gone, and the tunables are what removed it.

The outcome is neither of the two this experiment predicted. It was set up as
"all chips converge, so the per-chip key is vestigial" against "chips still
disagree, so pinning was wrong". What happened is the third case:

  pinning fixed the flapping, AND the per-chip key is still load-bearing.

  family0x19-model0x1    41,631,006   Zen 3   ┐ agree, and each repeats itself
  family0x19-model0x11   41,631,006   Zen 4   ┘
  family0x6-model0xcf    41,635,958   Intel   ← 4,952, stable

Two mechanisms of the same size were stacked on one number. Heap layout moved
it 5,064 and varied within a chip; dispatch moves it 4,952 and does not. Pinning
removes the first and leaves the second, which is a real property of the silicon
and exactly what kanso#1226 keyed for. So both pieces stay: the tunables make a
chip repeat itself, the key keeps two chips that genuinely differ from being
compared.

Neither piece is enough alone, and that is why this took five readings to see.
The tunables without the key would compare an Intel against an AMD. The key
without the tunables was kanso#1226, which passed on two agreeing chips and
broke on the third. The prediction that framed this — converge or fail — was
too coarse for the system it was about, which is its own lesson about naming a
falsifier: name one that can come back with an answer you did not list.

THE RATCHET NEVER ASKED WHAT THE GATE SAID FIRST. `prove` applies a mutation
in a worktree of HEAD, runs the gate, and reads a non-zero exit as the
mutation's doing. It never ran the gate before the mutation, so a gate already
red on HEAD was credited to every mutation sharing it — the rows pass, the run
reports "every row turned its gate red", and none of them proved anything.

That is the defect the whole table exists to catch, sitting inside the table's
own harness, and it is reachable rather than theoretical: `prove` builds in a
worktree with a shared target directory and scratch paths under /tmp that
ci.yml never uses, and a gate red for one of those looks identical to a gate
red for the mutation. The nightly would have said "every row turned its gate
red" either way.

`prove` now reads every gate once on an unmutated worktree and refuses to go
on if any is red. Deduplicated on (gate, setup), because the 62 rows name only
50 distinct pairs — nine share the diagnostic-coverage gate, four share the
trend gate — so the baseline costs 50 gate runs beside the 62 the proving
already pays. One worktree serves the pass, since nothing mutates it: only the
first setup's build is cold. The nightly has been running 24 to 29 minutes
against a 90-minute timeout, and the gates are the cheaper half of that.

The fixture had to enter where the ratchet enters, so it runs the real `prove`
over one row twice: once on a clean worktree of HEAD, once on a HEAD carrying
a tracked .py file, which is exactly what python-free refuses. The mutation
applies in both and the gate goes red in both. On the old code the second run
exits zero saying the row was proved; watched, then fixed. python-free is the
row because it is the cheapest end-to-end one in the table — no setup, a gate
that is two git greps — so both halves cost about five seconds.

Proving one row needed a way to name one row, so a scope word other than
`all` or `cheap` now selects by job name. That opened a second false green
immediately: a scope matching nothing selected no rows and reported that every
row turned its gate red. A typo was enough. It is refused now, with its own
fixture.

What the baseline does not cover, stated rather than assumed: it reads HEAD
before any row runs, so a mutation that dirties a shared scratch path and
reddens a LATER row's gate is still counted as that row's proof. No two rows
share a scratch path today. If any ever do, the answer is to re-read that one
gate, not to widen this pass.

Two counts in the workflow comments were stale, and reading them for this
change is how they were found. ci.yml said the per-PR `touched` step selects
thirteen rows and takes six minutes for a branch touching src/runtime.c; on
2026-09-03 it selects TWENTY-THREE and took 11m51s and 12m05s on two runs of
one branch, and nothing was watching that number grow. ratchet.yml said the nightly proves "a dozen" rows; it is
62. Both now say what they measure, and the nightly's six successful runs
between 2026-08-26 and 2026-09-02 took 24 to 29 minutes against its 90-minute
timeout.

The baseline for those 23 selected rows is 22 pairs, so the per-PR step gains
roughly its own gate half. That is a prediction rather than a measurement: the
next runtime-touching branch's job log settles it.

A WRONG-ARITY CALL WITH A FAILING ARGUMENT ANSWERED DIFFERENTLY ON EVERY
ENGINE. The entry of 2026-09-02 recorded this as OPEN, small and unpinned, and
said "no program in the corpus reaches it and I did not find a way to write
one". Here is one:

  fn boom what
    err what

  fn two_b g a what
    g a (boom what)

  lam1 = (x -> x * 10)

  bad = two_b lam1 3 "second"

The checker does not read a function-typed parameter's arity at the call site,
so passing a one-argument lambda into a body that applies two compiles, and the
count is settled at run time. There the three engines disagreed:

  interpreter   error[runtime]: this function takes 1 argument(s), got 2, exit 1
  native        prints "second", exit 0
  wasm          the same as native

`k_call1` through `k_call4` and wasm's `call_closure` tested every argument for
failure before they tested arity. `eval.rs` refuses the callable in `call` and
the count in `call_closure`, both above its argument test. The oracle is the
oracle, so the two compiled engines moved: the callable is asked whether it is
callable and whether its arity matches, and only then do failing arguments
propagate. A wrong-arity call is a broken program, and a value that happens to
fail must not be what hides the break.

The same reorder settles a second case nobody had asked about. A call on a
value that is not callable at all, with a failing argument, answered with the
failure on native and with "is not callable" on the oracle; the not-callable
refusal now comes first on all three.

WHY IT SURVIVED THE FIRST THREE PROBES, which is the part worth remembering:
`kanso run <dir>` COMPILES NATIVE. The interpreter is `kanso run <dir>
--interp`. Three probes written to compare the engines ran native twice, agreed
with themselves, and read as evidence that the divergence was unreachable. The
comment on `run_interpreted` says so in one line and the probes never asked it.

Cost, and it is only visible in one vein. All nine allocation gates and
`bench/emitted_golden.txt` are byte-identical, and so are all eleven rows of
`bench/instructions_golden.txt` — the reordered tests sit in a path the
benchmarks never take, since the emitted fast dispatch already tests tag, arity
and arguments in that order and falls through to `k_call{n}` only for the
shapes it declines. `bench/text_golden.txt` rises 128 bytes in total:
1,018,166 -> 1,018,294, sixteen bytes on each of eight benchmarks, and
jsonbench, escapebench and indexbench hold still because the linker drops the
helpers they never reach. The sixteen bytes are one extra copy of the argument
test: it had to move below the arity check in BOTH the closure arm and the
fnref arm of `k_call2`, `k_call3` and `k_call4`, where one copy above the tag
dispatch had served both.

The fixture is tests/golden/runtime/a_wrong_arity_call_carrying_a_failing_argument.kso,
watched red before green on native — it answered `error[endpoint]: unhandled
err reached the entry: "second"` where the oracle said the arity.

THE INDEX TWIN'S THUNK DEFERRAL CANNOT BE PINNED, AND THAT IS THE ANSWER. The
2026-09-02 entry for the index twin left a gap: `k_index_fast` defers a thunk
and a stored `none` to `k_index`, `index_holds_a_none.kso` pins the second, and
the first was "unreachable by any program I could write: no fixture in the tree
builds a list holding an unforced thunk and then indexes it".

Half of that is wrong and half of it is deeper than it looked.

A program reaches it: `x = [x]`. The element is a thunk until something demands
it, and native reports thunk_allocs=1, thunk_forces=1, thunk_evals=1 for a
program that does nothing but index it. So the shape is writable, and the
corpus now has it — three engines, three index forms, agreeing.

But the deferral has no observable, which is why no fixture pins it. Measured
two ways rather than argued:

  the twin's `%isthunk` test changed to compare an impossible tag, so a thunk
  is handed straight back where `k_index` would have forced it
    -> output identical, and all six thunk counters identical

  `k_force` removed from `k_index` entirely
    -> output identical

Every consumer of an indexed value forces it. The deferral decides WHERE the
force happens and never whether, so nothing a program can print or count tells
the two apart. A fixture asserting the deferral would be green with the
deferral removed, which is the shape this log has caught before and the reason
a spec gets watched red first.

What went in is therefore a differential pin and says so in its own comment: the
three engines agree on indexing a list that holds an unforced thunk, which none
of them had been asked about. The gap is closed by determining that there was
nothing there to pin, not by pinning it.

THE BASELINE PASS CAUGHT A BLIND ROW ON ITS FIRST RUNTIME-TOUCHING PULL
REQUEST. kanso#1228 landed at 07:35 and kanso#1229 was the next branch to
touch src/runtime.c. Its ratchet job went red:

  ratchet: the baseline is not green
  ratchet: 18 rows patch a file this branch changed
    ALREADY RED cost goldens (deterministic ratchet, no clocks)
    ALREADY RED utf-8 validator (differential vs an independent reference)

The utf-8 one is a real blind row and the diagnosis is exact. `validator_tail`
claims "a utf-8 validator that never looks at the end of a long run" and its
mutation is `a_validator_that_skips_its_tail.sh`, which points the tail read
back at the start. Its GATE read `whose "validator"`, which expands to
`sh scripts/gates/measured_on.sh bench/validator_golden.txt` — and there is no
validator vein. The file has never existed. `measured_on.sh` exits 2 on it with
`sed: can't read`, on every host, before and after any mutation, so `prove`
recorded the row as `red` and it proved nothing from the day it was written.

The gate is now the sweep the claim is about, the same one the row above it
uses, and the row was watched red for the first time in its life: with the
mutation applied, `scripts/utf8_differential` reports

  MISMATCH len=11 bytes=3f 16 55 56 66 56 4a 38 a8 27 0d got=1 want=0
  45189025 checked, 1097135 mismatches

against `0 mismatches` clean.

The cost-goldens one is not diagnosed yet, and the reason is a weakness in the
report I wrote: `ALREADY RED` named the JOB and not the GATE, and a job carries
up to seventeen gates. Guessing which from a job name is exactly the shape of
reasoning this log keeps catching, so the line names the gate now and the next
run says which rather than my inferring it.

Two things worth stating about the timing. The pass found this on its first
opportunity, which is the argument for it made concretely rather than
hypothetically. And it found it by being red on MY pull request — the check
cost its author a cycle before it cost anyone else one, which is the right way
round for a check like this to arrive.

---

## 2026-09-03 — THE RATCHET RAN OTHER JOBS' GATES ON A BOX THAT HELD NONE OF THEIR TOOLS

**DONE.** The gate-naming line added in the previous commit answered on its
first CI run, and the answer was a second blind row:

```
ALREADY RED cost goldens (deterministic ratchet, no clocks)
  gate: sh scripts/gates/instructions.sh
  red before any mutation, so no row sharing it is proof
```

`scripts/gates/instructions.sh` runs callgrind over the eleven benchmarks.
valgrind is installed at exactly one place in `.github/workflows/ci.yml` —
inside the cost goldens job, four steps before that gate. Neither the nightly
`prove` job nor the per-pull-request `touched` step installed it. Both run that
gate. `set -e` plus a missing binary is a non-zero exit, the gate was red before
any mutation was applied, and the ratchet reads a red gate as proof.

Two rows, not one. `scripts/gates/compile_instructions.sh` runs callgrind over
the front end and belongs to the same job, so `compile_ir` — "front-end work
that costs instructions and moves no other counter" — was blind by the same
mechanism. It was never selected on this branch because its mutation patches
front-end files, which is why the report named only the first.

A third, conditional on what a branch touches: `in_the_page`'s gate is
`bash scripts/browser_differential.sh`, which rebuilds `docs/kanso.wasm`.
`ratchet.yml` added `rustup target add wasm32-unknown-unknown` deliberately and
said why; the `touched` step in `ci.yml` was written later (#1201) and never got
it. A branch touching `src/wasm.rs` would have failed that row's gate on the
build.

**The shape is one thing, and it is not "somebody forgot valgrind".** Every
other job in ci.yml installs what its own gates need and nothing else, which is
right for that job. The ratchet runs ALL of their gates and had a base image.
Nothing related the two, so the ratchet's environment could fall behind any job
that grew a dependency, silently, one row at a time.

So the ratchet's setup is now `scripts/ratchet/toolchain.sh`, run by both
workflows, and `tests/the_ratchet_carries_what_its_gates_need.rs` keeps it a
superset: every package `ci.yml` apt-installs and every rust target it adds must
be installed there too. A package added to any job and not to the script turns
the specs job red.

**The first draft of that spec could not have failed.** It asked whether
`toolchain.sh` *contained the string* `valgrind` anywhere — and the script's
own header, which explains why valgrind is there, names it four times. With the
install commented out the check stayed green. That is the family kanso#1137
found four of: a comment is not a pin. The check reads install LINES on both
sides now, skipping comments, and a third test pins that a commented-out
install counts as installing nothing. Watched red both ways before it was
believed: `ci.yml installs ["jq", "valgrind"] ... and
scripts/ratchet/toolchain.sh does not`.

**Cost.** Two callgrind passes on the baseline and two more under mutation for
a runtime-touching branch, on top of the 11m51s/12m05s two runs of this branch
took — figures which undercounted because those gates could not run at all.
Both workflow comments now say so rather than carrying a number that was
measured with the gates absent.

**OPEN.** `bench/instructions_golden.txt` is not keyed per silicon the way
`compile_instructions` was in #1226, and the ratchet job lands on whatever CPU
the pool gives it. The eleven rows have held across the cost goldens job's own
runs, so the working assumption is that they are dispatch-insensitive in a way
kq's `print_small` is not — but the ratchet baseline is now a second, independent
sitting of those rows on a second, unrelated runner, and if that assumption is
wrong this is where it will say so. The first baseline that reports the rows
moved rather than the gate missing is the evidence to read.

---

## 2026-09-03 — THE SIXTH CHECK RESTING ON A MENTION, AND IT WAS THE ONE THAT CITED THE OTHER FIVE

**DONE.** Writing `tests/the_ratchet_carries_what_its_gates_need.rs` earlier today
produced a first draft that could not fail: it asked whether
`scripts/ratchet/toolchain.sh` *contained the string* `valgrind`, and the
script's own header, which explains why valgrind is there, names it four times.
That is the kanso#1137 family. Having walked into it, the obvious next question
was whether anything already in the tree had the same shape.

One did, and its header cites kanso#1137 by name:

```rust
// tests/the_objective_does_not_weigh_machine_code_size.rs
assert!(gate.contains("bench/text_golden.txt"), "…no longer diffs …, so nothing
        counts what the compiler emits…");
```

`scripts/gates/machine_code.sh` names the golden three times: the host check at
line 12, the diff at line 32, and one line of its `::error::` message at 37.
Replace the first two with nothing and the gate reads no golden at all — and the
spec passes, certifying the exact state its own failure text describes.
Measured, not reasoned:

```
--- remaining mentions:
37:  echo "::error::bench/text_golden.txt. Allocation counters cannot"
test the_machine_code_vein_is_still_gated ... ok
```

Both halves read operative lines now — comments dropped, and lines whose whole
job is to print a diagnostic dropped with them — and a third test holds the
counterexample so the file cannot drift back. Watched red on the gutted gate and
green on the restored one.

**Reading lines is still not running the gate**, and the entry says so where a
reader will find it: the behavioural proof is the ratchet's `machine_code` row,
which applies a defect nightly and refuses a gate that stayed green. The spec is
the cheap per-pull-request half of the same split the ratchet itself documents.

**The sweep found no seventh.** Every test that reads a repository file was
checked for a positive `contains` against prose-bearing text. Three others take
that shape and all three read data rather than source: `bench/welfare_floor.json`
(no comments), emitted LLVM IR, and a hako lock file. The rest assert on a
tool's output, which is where a spec should be entering anyway.

**The pattern worth carrying.** Both instances today were written by somebody
who knew the rule — this file's own header argues it, and the toolchain spec was
drafted an hour after reading it. Knowing the rule does not catch the case;
trying to make the check fail does. The cost of that attempt is one `sed` and
one `cargo test`.

---

## 2026-09-03 — THE COMPILE ROW MOVED 992 AND THE FRONT END DID NOT

**DONE.** CI refused the branch on the vein that is keyed by silicon:

```
the work the FRONT END does changed on family0x19-model0x1:
bench/compile_instructions_by_cpu.txt says 41631006 and this run counted 41631998.
The row is keyed by silicon, so the runner is not the answer here:
the same family and model counted both numbers.
```

`compile_instructions` **41,631,006 -> 41,631,998, +992 (+0.0024%)**, and the
cause is written in that file's own header from earlier the same day:
`src/main.rs` embeds the runtime with `include_str!("runtime.c")`, so a change
to the runtime changes the compiler binary even when it changes no compiler
code. The wrong-arity reorder added a few lines of C; the embedded source grew,
the rodata after it shifted, and the front end does the same work at different
addresses. `compile_allocs`, `compile_peak_bytes` and every fixpoint counter are
byte-identical, which is what says the work did not change.

The Zen 3 row (0x19/0x1) this run landed on is re-sat. The Intel (0x6/0xcf) and
Zen 4 (0x19/0x11) rows are removed rather than carried forward, on the rule
already recorded there: a value measured against the old binary is worse than no
value. They are re-sittings when they next refuse, and each costs one red run.

**This is the second time in one day this file has been emptied for the same
mechanism**, and the price is now visible rather than argued: a runtime change
of any size costs one red CI run per chip in the pool before the vein reads
clean again. Whether the vein should measure the front end against a binary
that embeds the runtime is a real question and it is not this change's to
answer; what is clear is that `include_str!` makes "the front end moved" and
"the runtime moved" indistinguishable to this row, and only the other three
compile counters can tell them apart.

Welfare is 76.01 before and after — 992 parts in 41.6 million is below the
hundredth the score is rounded to.

---

## 2026-09-03 — THE REPORT NAMED THE GATE AND NOT ITS REASON

**DONE.** Naming the gate in `ALREADY RED` answered the question it was built
for on its first CI run — `sh scripts/gates/instructions.sh`, red because
valgrind was not on the ratchet's box — and could not answer the second. With
valgrind installed by `scripts/ratchet/toolchain.sh` the same gate is **still
red on the baseline**, and a line that says only which gate leaves a reader
holding hypotheses with no way to choose between them. The gate wrote down why
it failed; the ratchet was throwing that away.

`ALREADY RED` and `UNBUILT` now carry the last eight lines the gate printed,
indented under the finding. Watched red on the fixture that commits a tracked
python file: without the gate's words the refusal does not contain
`crept_in.py`, and the spec says so.

**One hypothesis is already dead.** The obvious candidate was the worktree: the
baseline builds in `/tmp/kanso-ratchet-base` against a shared target symlink,
and paths baked into a binary would move an instruction count the way a run id
that gained a digit does. Built both ways and compared:

```
8a406faa2e27030c41bc8dd12ab9750fe5ddb408afe54cedd52ce66f5cd475d3  ./jsonbench
8a406faa2e27030c41bc8dd12ab9750fe5ddb408afe54cedd52ce66f5cd475d3  /tmp/kanso-pathtest/jsonbench
```

Byte-identical. The benchmark does not depend on the directory it was built in,
so whatever reddens that gate on the ratchet's runner is something else.

**OPEN, and the next run answers it.** The live candidate is the one recorded
two entries ago: `bench/instructions_golden.txt` is not keyed per silicon the
way `compile_instructions` is, and the ratchet job lands on whatever CPU the
pool gives it — a second, independent sitting of those eleven rows on a runner
unrelated to the cost-goldens job's. If the gate's own words turn out to be a
row diff, that settles it and the vein needs the treatment kanso#1226 gave the
compile row. If they are something else entirely, they will say so, which is
the whole point of printing them.

---

## 2026-09-03 — TWO SITTINGS OF THE SAME ELEVEN ROWS IN ONE COMMIT, AND THEY DISAGREE

**MEASURED.** Printing the gate's own words paid on its first run. The
baseline's refusal now reads:

```
ALREADY RED cost goldens (deterministic ratchet, no clocks)
  gate: sh scripts/gates/instructions.sh
  red before any mutation, so no row sharing it is proof
    digestbench 81252330
    the work the benchmarks do changed. A rise is a regression to explain…
```

`digestbench 81252330`. The cost-goldens job, **on the same commit**, measured
`81252316`. Fourteen instructions apart, two jobs, two runners
(1000054847 and 1000054838), one image and one glibc.

**This is the first time the eleven rows have been sat twice in one commit.**
Every earlier reading came from one job on one runner per run, so the vein has
only ever been compared against itself across commits — where a chip change and
a code change are the same event. The log has carried the question since
2026-09-01: *"These rows claim to be exact and the pool is not… nothing here
measures that fraction."* This measures it, at n=1: two runners, one row, +14.

**Two hypotheses are dead.** The benchmark binary is byte-identical between the
repo root and a worktree built against a shared target symlink
(`7779456c957c6cff…` both), and running the same binary from those two
directories gives the same count to the instruction (81,251,917 twice, in this
container). So neither the build directory nor the run directory moves it.

**What is NOT decided, and deliberately.** One row on one pair of runners is a
data point, not a design. The options are a golden per chip (the treatment
kanso#1226 gave the compile row, at eleven rows times the pool instead of one),
declaring the two callgrind gates unprovable by the ratchet (two rows blind by
declaration, which is what the ratchet exists to prevent), or something that
compares the mutated measurement against the baseline's own rather than against
a golden. Choosing between them on one number is the shape of reasoning this log
keeps catching, and the pending-gavels ledger is explicitly not for
implementation questions — this one is the file-holder's, answered here.

So the evidence gathers instead: the captured tail goes from eight lines to
twenty-four, which is enough for the cpu line `dispatch.sh name` prints and the
whole eleven-row diff. Every runtime-touching pull request from here is another
paired sitting, attributable to its silicon, at no extra cost. The decision gets
made on a table rather than on a pair.

---

## 2026-09-03 — A FOURTH SHAPE: REPORTED AND NOT CREDITED

**DONE, and it corrects the entry above.** That entry listed three ways out of
the paired-sitting problem — a golden per chip, declaring the two callgrind
gates unprovable, or comparing the mutation against the baseline's own
measurement — and said choosing between them on one data point was premature.
It was, and it still is. What the entry missed is that none of them has to be
chosen, because the question it was answering was the wrong one.

The ratchet's baseline asks *is this gate red for a reason other than the
mutation*. For `sh scripts/gates/instructions.sh` on a foreign runner the answer
is yes, and that is a fact about the ROW rather than about the branch. So the
row is **reported and not credited**: the baseline prints `UNPROVEN THIS RUN`
with the gate's own words and does not fail, and the proving pass drops every
row sharing that gate instead of applying a mutation to a gate already red.

**That is the opposite of the blindness kanso#1228 was built to catch, and the
difference is one word.** Before, a red gate was silently counted as PROOF. Now
it is silently counted as nothing, and says so out loud. On a run that lands on
the golden's silicon the row is proved normally, so this costs coverage only on
the runs where coverage was never available.

**The danger is the list, not the mechanism** — an entry excusing a gate that is
not silicon-bound turns a real failure into a note. So the list is pinned to a
property of the gates rather than to anyone's judgement:
`tests/a_host_bound_gate_is_reported_not_credited.rs` requires `host_bound` to
be exactly the set of gate scripts that invoke callgrind on an operative line. A
new callgrind gate left undeclared turns it red; a declared gate that runs none
turns it red too.

**That spec's own first draft could not fail, which makes three today.** It read
the `bound` BINDINGS rather than the `host_bound` LIST, so removing an entry
from the list left it green — the binding was still there and the list it was
absent from was never opened. Watched red the second time, and the failure names
both sides:

```
  left: ["sh scripts/gates/instructions.sh"]
 right: ["sh scripts/gates/compile_instructions.sh", "sh scripts/gates/instructions.sh"]
```

Three checks in one day whose first draft was satisfiable without the property
holding. The common thread is that each read something ADJACENT to the thing it
meant to assert — a comment beside an install, an error message beside a diff, a
binding beside a list. Trying to make the check fail is what found all three, and
it cost one `sed` each.

---

## 2026-09-03 — THE SKIP I ADDED AN HOUR AGO HAD A FALSE GREEN IN IT

**DONE.** `kept_provable` drops every row sharing a host-bound gate, and the
proving pass then runs the rest. When the rest is empty — a branch whose whole
selection is host-bound — the loop had nothing to do and the closing line said

```
ratchet: 0 rows
ratchet: every row turned its gate red
```

which congratulates a run that proved nothing. That is the exact shape the pass
exists to refuse, reintroduced by the fix for it, and it survived because the
`told` arms dispatch on whether any row FAILED and no arm asked whether any row
RAN.

The empty case says so now and does not fail, because no diff could have proved
those rows on this runner:

```
ratchet: no row on this runner could be proved; none was claimed
```

**Found by reading the summary path rather than by a spec**, and the reason
there is no fixture is worth stating: constructing one needs a scope whose every
selected row is host-bound, and `asked?` selects by job name, so the cheapest
such scope drags in seventeen rows and their builds. The behavioural proof is
the nightly. What is pinned per-pull-request is the list
(`tests/a_host_bound_gate_is_reported_not_credited.rs`), which is where a
mistake is actually likely.

**Four in one day now**, and the pattern has stopped being a coincidence: the
toolchain spec read a comment beside an install, the machine-code spec read an
error message beside a diff, the host-bound spec read a binding beside a list,
and this one read a failure count beside a run count. Every one of them was
adjacent to the property and satisfiable without it.

---

## 2026-09-03 — A NUMBER'S BYTES WERE WALKED TWICE

**SHIPPED.** `lib/json` read every number twice. `number_end` walked the bytes
asking where the number stopped, and `mark_from?` walked the same bytes again
asking whether a `.`, `e` or `E` had gone past, because the answer decides
`to_int` against `to_float`. The scan carries the mark as a boolean argument
now, so one walk answers both questions.

```
jsonbench   2,533,092,019 -> 2,428,220,306    -104,871,713, or 4.14%
```

Measured in the container, three runs, same digits. The runner's own rows are
what land in `bench/instructions_golden.txt`; this is the size of the move, not
the number to paste.

**The profile says exactly where it went.** `value_for` was the largest symbol
in the decode at 648,032,400, and it was a merged one — clang had inlined the
whole value-parsing path into it, the number scanner included. It reads
239,779,050 now, and the surviving scanner stands as its own symbol at
272,697,300. 648.0 - 239.8 - 272.7 leaves 135.5M, and the total fell 104.9M:
the difference is the arms the merged function no longer carries.

**What it cost.** The front end visits 17,169 expressions on `lib/json` where it
visited 16,806, a rise of 2.2%, and the emitted decoder gains 89 lines and 20
calls while losing one define. Two small walkers became one larger one with an
extra argument. `.text` FALLS 48 bytes on jsonbench and on oneshot, which is the
second scanner leaving. No allocation counter moves at all — the same slice, the
same `to_int` or `to_float`, per number — which is why this is a change the cost
goldens could not have seen and the instructions vein could.

**The counters, by name, and where they landed.** `front_end_visits` 16,806 ->
17,169. `emitted_calls` 1,808 -> 1,828 and `emitted_lines` 12,044 -> 12,133 for
the decoder; `emitted_other_calls` 14,532 -> 14,552 and `emitted_other_lines`
87,826 -> 87,915 for the eight beside it, all of that move being oneshot, which
imports the same library. `emitted_defines` and `emitted_other_defines` each
fall by one, and `text` falls 96 bytes over the eleven programs. Every one of
those five rises is the same fact: one scanner with an extra argument and more
arms, where there used to be two with none.

**The mark rides as an argument rather than in a record beside the end
position.** A record would have been an allocation per number: 4,217 of them per
decode, 632,550 over the benchmark, and that is a term welfare weighs where
instructions on this path are one it weighs more lightly. The boolean costs
nothing and the arms read the same.

**Two new arms had no coverage at all.** `e` and `E` never appeared in
`lib/json/json_test.kso` before today — the float tests were all `3.25` and
`2.5`, so the exponent forms went through a scanner nobody had exercised.
`test_decode_exponent`, `test_decode_exponent_upper`,
`test_decode_exponent_signed` and `test_decode_negative` are new, and each was
watched red first: dropping the `101` arm's `true` reds `test_decode_exponent`
with `invalid number` at position 1, and deleting the `69` arm reds
`test_decode_exponent_upper` with `unexpected trailing characters` at 2. The
third falsifier — a `digit_step` that drops the mark — the LANGUAGE refuses:
`marked` becomes an unused binding and the compiler will not build it.

**OPEN.** `bench/widebench/widebench/` and `bench/encodebench/encodebench/`
carry their own copies of the json library, and they differ from `lib/json`
already — widebench's has a `pretty.kso` the shipped library does not. They are
frozen fixtures rather than stale copies, and nothing in the tree says so. They
are left alone here; whether a benchmark that vendors a library should track it
is a question worth asking once rather than per change.

---

## 2026-09-03 — DECLINED: THE ESCAPED-STRING TAIL DOES NOT WANT A RUN SCAN

**DECLINED by measurement, +1.16%.** A clean json string is found with one
`find2` and copied with one slice. A string with an escape in it is not: past
the first backslash, `str_chars` walks the rest one byte at a time, through a
dispatch and a one-byte append each. Making the tail scan runs the way the head
does — `find2` to the next quote or backslash, then append the run in one
copy — reads like the obvious fix.

```
jsonbench   2,533,092,019 -> 2,562,563,906    +29,472,300, or 1.16%
```

**The distribution is why.** In `bench/large.json`, 1,773 of 11,057 strings
carry an escape, and they hold 4,562 runs between them totalling 16,895 bytes:
a mean run of **3.7 bytes**, with 1,029 of the runs empty because escapes come
back to back. The per-run fixed cost measured about 300 instructions — 113 in
the scanner's own glue, 75 in `find2`, 46 in `k_b_slice`, 49 in the wide append
— against the 50 instructions a byte the walk costs. The run has to be six bytes
before it breaks even and it is under four.

**A fused `append(acc, slice(cs, a, b))` does not rescue it.** `k_b_utf8_slice`
already exists for the same shape one line above, so the pattern is available;
it would take back the 46 instructions the slice allocation costs, which leaves
the change roughly 60M worse than doing nothing.

**What the probe found instead is worth keeping.** Two programs, a clean string
and one with a single leading escape, at 2,000 / 4,000 / 8,000 bytes:

```
clean    213,329   264,566   366,562     25.6 instructions a byte
escaped  316,640   469,253   773,152     76.0 instructions a byte
```

Linear in both, so there is no quadratic hiding here. But a byte in an escaped
string's tail costs about **50 instructions more** than the same byte in a clean
one, and that is the language's per-byte dispatch-and-append, not anything
`lib/json` chose. The disassembly of `str_char` is 41 instructions round the
loop: six to index, ten to box the byte and unbox it again for the dispatch,
eight to dispatch, thirteen for the in-place append including two loads of
`k_stats_on`, four for bookkeeping. Whoever wants this path faster should go
after those ten, not after the number of walks.

---

## 2026-09-03 — A BYTE SWITCH THAT REBUILT THE BYTE BEFORE IT LOOKED AT IT

**SHIPPED, and it is the largest single runtime move this log holds.** A group
whose arms discriminate on a byte read out of a byte string crosses the call
boundary as a raw `i64` — the byte, or 256 for the `none` a read past the end
answers. `rebox_params` then rebuilt a `KValue` from that raw value, and the
dispatch tree pulled the rebuilt struct apart again: tag out, compare to 0,
payload out, switch. The comment above the reconstruction said the round trip
"folds back into a raw switch". **It does not.** `str_char`'s loop:

```
5132:  cmp    $0x4,%rax
5136:  cmove  %rbp,%rcx        ; 256 for none, on the caller's side
513c:  cmp    $0x100,%rcx
5143:  sete   %dl
5146:  cmove  %r13,%rcx
514a:  shl    $0x2,%edx
514d:  test   %edx,%edx        ; the tag test the tree wrote
```

Seven instructions a byte to take apart a value that was never assembled, and
every byte-dispatching function in the decoder paid it. The tree switches on
`%xNr` directly now, with 256 as the `none` case.

```
jsonbench    2,533,092,019 -> 2,098,859,754   -17.1424%
oneshot         34,322,446 ->     31,427,168    -8.4355%
widebench       61,890,181 ->     59,506,049    -3.8522%
encodebench  5,848,702,451 ->  5,846,994,368    -0.0292%
indexbench       5,242,363 ->      5,241,950    -0.0079%
basket          40,300,172 ->     40,299,759    -0.0010%
deepbench      676,465,730 ->    676,462,050    -0.0005%
digestbench     81,252,316 ->     81,251,917    -0.0005%
escapebench    130,170,751 ->    130,170,352    -0.0003%
pendbench      715,732,938 ->    715,732,552    -0.0001%
scanbench    1,423,437,576 ->  1,423,437,163    -0.0000%
```

Those are the CONTAINER's, which is where the ratios in this entry come from
because a ratio needs both ends measured on one box. The runner's rows are the
ones the golden takes, and it reads the seven small ones as not moving at all;
the paragraph below has them.

**Nothing rises.** The jsonbench figure carries the single-pass number scan
above it as well; this change is 13.56% of it on its own, 2,428,220,306 to
2,098,859,754. **widebench is the clean attribution**: it vendors its own copy
of the json library, frozen, so the 3.85% there is the dispatch and nothing
else. The seven programs that do not dispatch on bytes move by four hundred
instructions or fewer, which is the compiler emitting a slightly different
module and the linker laying it out differently.

**The counters, against the branch point rather than against the entry above
it, because that is what the trend gate reads:** `emitted_calls` 1,808 ->
1,820, `emitted_branches` 1,210 -> 1,186, `emitted_lines` 12,044 -> 12,053,
`emitted_other_calls` 14,532 -> 14,526, `emitted_other_branches` 8,749 ->
8,673, `emitted_other_lines` 87,826 -> 87,659, `emitted_defines` 169 -> 168 and
`emitted_other_defines` 1,469 -> 1,468. Calls and lines rise because the number
scan above bought them; the two changes pull those two counters in opposite
directions and the scan pulls harder. **Branches only fall**, in the decoder
and in the eight beside it, and that is this change alone: nothing about the
number scan removes a branch. `text` falls 8,080 bytes over the eleven
programs — 1,936 on jsonbench, 2,080 on encodebench, 1,888 on oneshot and 2,080
on widebench, and not a byte on the other seven. Rounds and
visits on `lib/json` do not move at all, which is the check that this changed
what the backend writes rather than what the front end decides. No allocation
counter moves.

**One literal had to be guarded, and the fixture found it before CI did.** 256
is the sentinel, so a program that writes `fn kind 256` — an arm no byte can
ever reach — would send every read past the end of a byte string to that arm.
The boxed tree is immune because it tests the tag before it looks at the
payload. The divergence was real and I watched it:

```
--- native      --- interpreter
bracket         bracket
quote           quote
a byte that cannot be    some other byte
```

A group with any int literal outside 0..255 stays on the boxed path, where the
arm stays as dead as the oracle says it is.
`tests/a_byte_arm_no_byte_can_reach.rs` pins both halves — the impossible
literal and an ordinary byte group beside it, so a change that disabled the
fast path everywhere would not read as a pass. Watched red against the
unguarded draft, with the two engines' answers printed side by side.

**Why the emitted-line count is the presence counter here.** There is no
observable output that distinguishes a raw switch from a boxed one; what
distinguishes them is `emitted_branches`, which falls by 24 in the decoder and
76 across the eight beside it because the tag test and the none test are gone
from every byte-dispatching call. Revert the change and that counter goes straight back
up, which is what the vein is for.

**What CI's own sitting says, and what it cost the compiler.** The runner reads
the four moved rows 413 or 399 above the container and reads the other seven
EXACTLY where they were: jsonbench 2,098,860,167, oneshot 31,427,567, widebench
59,506,462, encodebench 5,846,994,767, and no movement at all in basket,
deepbench, escapebench, pendbench, indexbench, scanbench or digestbench. The
container's few-hundred-instruction falls on those seven were the container.

**The compiler pays, and the first attribution I wrote for it was wrong.**
Three compile counters rise — `compile_instructions` 41,631,998 -> 41,831,767
(+0.48%), `compile_peak_bytes` 713,606 -> 715,275 (+0.23%), `compile_allocs`
25,394 -> 25,485 (+0.36%) — and I priced all three against the raw byte switch,
because it was the larger change and the rises arrived with it. **`kanso check`
never runs the backend.** It lexes, parses, infers, runs provenance and the
advisories, and stops; a codegen change cannot reach those rows except through
the binary's layout. Every one of them belongs to the json library's extra arms.

Held rather than reasoned, because a reason that sounds right is what produced
the wrong version. With the codegen change reverted and lib/json untouched, the
container reads `compile_allocs=25485` and `compile_peak_bytes=715275` — the
same two numbers as the branch head, to the byte. And the instruction row,
three builds under the same tunables:

```
main                                 42,032,508
the library change alone             42,238,115   +205,607
the library change and the switch    42,235,790     -2,325
```

The switch gives 2,325 BACK, inside the layout band the by-cpu file documents in
thousands. Rounds hold at 40. The Zen 4 row is removed rather than carried
forward, per that file's rule about values measured against an old binary.
**Welfare 76.0100 -> 76.1700**: the objective takes the trade, and compile cost
satiates at 0.5 against runtime's 2.0, which is exactly the asymmetry it was
weighted for.

**The published scoreboard is dated now, and says which way it is stale.** The
per-decode floors in §08 — kanso 0.87 ms against serde_json's 0.90, naive rust's
1.04 and go's 2.05 — come from seven interleaved rounds on 2026-08-07, by the
slope method, and nothing on the page said so. They are re-sat at a release and
not on demand, because randomised-layout timing puts the spread within one tree
at about three per cent and this box is not idle. So the caption carries the
date, and a paragraph under the block says the decoder has moved a long way
since and names these two changes as 17.14% of it. Publishing a number I cannot
honestly re-measure would be worse than publishing a stale one that says it is
stale.

The lesson is the one this log keeps relearning from a different direction. Two
changes shipped together, one large and one small, and every unexplained number
attached itself to the large one. What separated them was not an argument about
mechanisms but two rebuilds and four counters.

**And the landing page was quoting a sitting nobody had re-read.** The
number-bearing surfaces are a checklist rather than a memory, so walking it
found `docs/index.html`'s receipts panel carrying `reasonably-written rust 1.02`
and `go encoding/json 1.95` where §08 reads 1.04 and 2.05. They are the
2026-07-27 figures. #756 replaced that panel's kanso and serde rows with a
pointer to the live board and left the two rows underneath alone, so the site has
disagreed with itself about two of its four lanes for five weeks with every gate
green. Both rows now read the 2026-08-07 sitting and the caption names its date,
the way §08's does. Nothing checks this: the two pages hold the numbers as prose
and `golden_prose` only reads what carries a `data-golden` attribute, which these
cannot, because a hand-sat wall clock has no golden to read.

**A chip the pool had never shown.** CI's compile-instructions gate went red
on `family0x6-model0xcf` — Emerald Rapids, absent from the four the by-cpu
file's header lists — with the refusal that an unrecorded chip is an unsat row.
It read 41,832,275 where Zen 3 read 41,831,767 on the same binary, sha
55fb850296d1 printed on both. 508 apart, against the 5,124 the header
decomposes between two other chips on one binary. The ifunc effect is a
property of the pair, not a constant, which is a second argument for the key
over a band: a band wide enough for 5,124 hides every front-end move this vein
has caught, and one narrow enough for 508 would still have refused this run.

**A comment claimed a property the machine code contradicted, and nothing in
the tree could see it.** That is the same family as #1137's four pins that
rested on prose — except that one was a spec reading a comment, and this was a
comment asserting an optimiser outcome. The optimiser is entitled to change its
mind between releases; a claim about what it will do belongs in a counter.

## 2026-09-03 — EIGHT BOOK PANELS QUOTED A LIBRARY NOBODY WAS CHECKING THEM AGAINST

The landing page says every code panel in the book is executed against the real
toolchain before it may appear. Ten were not, and four of those had drifted so
far they printed code the language cannot compile.

**How the gap is shaped.** `scripts/book_panels` regenerates a panel by
resolving its title to a sample it owns: `<samples>/<title>`, or a bare
filename one directory deeper. The deeper branch compares a path's LAST SEGMENT
to the whole title, so a title carrying a slash — `lib/json/number.kso` — can
never match, and the harness leaves it alone by design: "a name that is nowhere
is left alone: the panel may be quoting something this does not own." A
directory-module title falls in the same hole from the other side, because
`literal.kso` names `samples/ch08/literal/` rather than a file. Eight ch08
panels quote `lib/json`, two quote directory modules, and nothing read any of
them.

**What was in them.** The `lib/json/text.kso` panels wrote `at cs n` for the
index; there is no `at` under any spelling, so that panel has been printing a
name error for as long as `cs[n]` has been the syntax. They also wrote
`concat [] (slice ...)` where the library writes `append (bytes "")`. The
`scan.kso` panel named `_is_ws` for a function renamed `ws?`. The `value.kso`
panel named three byte-array constants and a `_word` that sliced the tail out
and compared arrays — the library replaced all of it with `rue?`, `alse?` and
`ull?` comparing bytes where they sit. The two directory-module panels still
showed a `pub play` and a `told`/`chosen` reporting pair from before those
samples were split into `main`/`lit`/`report`.

**Why the check is structural.** These panels are excerpts with editorial
changes: a private declaration gains a leading `_`, a module qualifier is
dropped, a field gains a `:type` the file leaves bare. So the text cannot be
compared and the panel cannot be regenerated. Two properties survive every one
of those changes:

  every declaration the panel shows is declared in the file it names, and
  every name the panel uses is a name that file can reach.

`scripts/book_quotes` checks both. The reachable set is read off the package
rather than listed: its own declarations and bindings, the tail of every
qualified name it calls, and every bare name its own code uses. So a panel may
name what the file could name, and there is no table of builtins here to go
stale in its turn — which is the failure mode a hand-written list would have
reproduced one level up.

It found eleven things across four panels and nothing else, and it is precise
in the direction that matters: run against the fixed book it is silent, and
renaming `ws?` in the library reddens it in two lines. That rename is the
ratchet mutation.

**And then the gap closed, because the claim about it was wrong twice.** The
first version of this entry said the three remaining panels each want a
different resolution rule, and that ch09's `vse/methods.kso` names a file in
the vse repository. Both are wrong. Two rules reach all three: ch10's titles
are written relative to `docs/book`, and ch07's and ch09's relative to a
sample directory one level in — `samples/ch07/teahouse/` holds `main.kso`
beside a package dir `teahouse/`, and `samples/ch09/vse/` the same. ch09's
`vse/` is a sample package in this repository.

Both rules are in, and they found four more panels in two more chapters:

- ch07's `teahouse/menu.kso` showed `fn describe (err reason)` — an arm naming
  its own package's err. The language refuses that and the sample dropped it.
  **The chapter contradicted itself on one page**: the panel three inches below
  demonstrates the replacement, `testing/when_failed pocky (r -> ...)`, and
  only that one was machine-checked, because book_panels owns it.
- ch07's `teahouse/menu_test.kso` showed a hand-written `err?` predicate where
  the file imports `std/testing` and calls `testing/failed?`.
- ch10's `pingpong.kso` wrote `even`/`odd` for the file's `even?`/`odd?`, and
  wrapped the statements in a `main =` the file does not have.
- ch10's `classify.kso` had the same `main` wrapper.

Fourteen panels across four chapters, on a page that says every one of them is
executed against the real toolchain before it may appear.

**ch09 is clean, and that was verified rather than assumed.** Six panels there
quote `vse/`, and silence from a checker is worth nothing until you have seen
it speak: inserting a declaration no file has into the ch09 panel reddens the
gate in two lines. So the silence is coverage.

**One false-positive class had to go first.** `true`, `false` and `none` are
literals the grammar provides, and the reachable set is read off the package's
own text, so a small file that happens never to write `false` was forbidding a
panel from showing an arm that answers it. They are always reachable now. That
is the only exception, and it is a list of spellings rather than of names —
which is what keeps it from becoming the builtin table this deliberately does
not have.

A title that resolves to a FILE beside its chapter stays skipped for a
different reason: book_panels owns it, and reading ch07's `shop.kso` panel
against its `shop/` directory is how this first went wrong.

**And the prose went with them.** Four paragraphs described the old
implementations — the byte-array constants, the four hex digits read inline
before `_str_hex4` existed, the sample's helpers as "verbatim from
number.kso" when they are the shape the library left behind. Nothing checks
prose against code and nothing here proposes to; what the gate buys is that
the CODE beside the prose can no longer drift silently, which is what made the
prose wrong.

## 2026-09-03 — a recorded chip disagreeing with itself, and a row nobody reads

Two findings on `bench/compile_instructions_by_cpu.txt`, both from one
branch whose diff was `design/compiler-log.md` and its archive.

**The third cause. OPEN.** `scripts/gates/compile_instructions.sh` prints a
`compile_sample cpu= sha= row=` line on every run for one purpose, and its
comment names it: *"same cpu and same sha with different rows would mean
something is moving that neither the key nor the diff can see."*

| when | key | binary sha | counted |
| --- | --- | --- | ---: |
| 12:33 | family0x6-model0xcf | 55fb850296d1 | 41,832,275 |
| 13:05 | family0x6-model0xcf | 55fb850296d1 | 41,831,767 |

508 apart, same key, same binary, and no front-end change in the diff. So the
key does not determine the row, and the premise the per-chip table rests on is
weaker than it claimed when #1226 built it.

The row moved to 41,831,767, which three readings say — Zen 3, Zen 4, and this
chip at 13:05 — against one that says 41,832,275. **That is a choice under
uncertainty and not a measurement.** The honest state is two readings of one
key on one binary, differing, and the golden's own note says so in those words
rather than dressing the move up as a re-sitting.

What it wants is a sitting: several runs on one Emerald Rapids runner, to see
whether the value is bimodal there the way it was across the two AMD models
before the tunables were pinned. If it is, the missing term is something the
pinned tunables do not cover, and the first candidates are the ones the file's
header already names as excluded from the key — cache sizes and derived
thresholds — reaching the row by a path other than the malloc and string
tunables the gate sets.

**A second row for one chip. DONE.** Correcting that row, I edited it in place
AND appended it, and every gate in the tree stayed green over a file holding
two `family0x6-model0xcf` lines. The lookup is
`awk '$1 == k { print $2; exit }'`: the first match answers and every later row
is dead. So the file had one authority and one decoration, and the decoration
sat at the bottom where the newest sitting goes, which is where a reader is
most likely to trust it.

`compile_ir_row.sh` refuses a doubled key now, before the lookup rather than
after, so a reader is told about the duplicate instead of being sent to re-sit
a front end that did not move — which would have written a third row into a
file that already had one too many. Five refusals where there were four.

Three specs, each watched red on the unfixed script: a duplicate that AGREES
with itself is still refused, because the defect is the second line rather than
its value; a duplicate whose second row is what the run counted is not reported
as a moved front end; and a duplicate on a chip this run did not land on is
refused too, since otherwise the file stays broken for every run but the
unlucky one. New ratchet row `compile_ir_doubled` with a mutation that deletes
the block and reddens exactly those three.

The gap was cheap to find only because a mistake walked into it. Nothing else
in the tree reads this file, so nothing else could have.

## 2026-09-03 (later) — the chip was not the answer, and a refusal nobody could act on

Two corrections to the entry above it, one of them to a claim it made.

**The 508 is glibc's allocator, and the cpu is ruled out. CORRECTS the entry
above.** That entry left the thread OPEN and proposed cache sizes and derived
thresholds as the candidates. That guess is wrong and the measurement was
available the whole time.

Both runs printed their full 123-line cpu feature block — no `bench/dispatch.txt`
is recorded, and `dispatch.sh name` dumps the block in CI while none is. The two
blocks are byte-identical: every feature word, the stepping, and
`data_cache_size` among them. Both ran glibc 2.39-0ubuntu8.8, callgrind 3.22.0
and rustc 1.98.0, on binary sha 55fb850296d1.

The same two job logs carry the profiles. Every kanso symbol agrees to the
instruction across the pair — `eval_expr'2` 1,633,593 both times, `check_merged`
1,589,240, `lex_line` 866,486, `parse` 589,004. Three rows differ:

| | 12:33 | 13:05 | |
| --- | ---: | ---: | ---: |
| `_int_malloc` | 1,551,384 | 1,551,964 | +580 |
| `_int_free` | 1,522,333 | 1,522,352 | +19 |
| `memcmp-avx2-movbe` | 1,353,408 | 1,353,342 | −66 |

All three are glibc. The front end executed the identical instruction sequence
and the allocator did not, which is the same shape the by-cpu file's header
records for the 5,064 cluster the tunables were pinned to remove: `_int_malloc`
and `memcmp` moving together, an alignment difference downstream of a heap
layout difference. Pinning the tunables shrank it from 5,064 to 508 and did not
remove it.

Five consecutive runs in a container on one binary read 42,235,790 every time,
so the count is deterministic per host and the 508 is not run-to-run jitter.

**Still OPEN**, with a narrower question: what moves the heap layout when the
binary, the cpu features, glibc, valgrind and the environment all agree? "Is it
the cpu" is answered, and the answer is no.

**A refusal nobody could act on. DONE.** The runner's rustc moved 1.98.0 ->
1.98.1 and all three compile veins refused at once. The refusal is right — the
rows belong to the toolchain that measured them. What follows it was not.

Every compile gate called `measured_on.sh` under `set -e`, so the mismatch
stopped the gate before it measured anything, and each then printed that
refusal's standing advice: *let CI measure it and copy the rows out of the job
log.* There were no rows in the job log. The only host allowed to produce them
is the one that just stopped, so a branch in this state could not be brought to
green by anybody — not by CI, which stopped, and not from a container, which
may never record its own numbers.

`scripts/gates/host_gate.sh` answers two questions where `measured_on` answered
one. A container may neither compare nor measure: its numbers going into a
golden over the runner's is the accident `measured_on` exists after, so it stops
and prints nothing. CI may not compare and MUST measure, because CI's sitting is
the only one that may ever be recorded — it measures, prints the rows under
`::error::`, and still fails. `dispatch.sh` already draws this line the same way
and for the same reason.

`compile_instructions` additionally does not read its row against any recorded
chip on such a host, since a row from an unnamed host has no chip to be read
against.

Four specs, both directions watched red: with the CI arm removed the run that
must measure stops, and with it always taken the container that must stop
measures. Ratchet row `host_measures`.

The three goldens are NOT re-sat here. This makes CI able to report the sitting
they need, which was the missing step; the numbers follow from the next run.

## 2026-09-03 (later still) — the rustc bump moved nothing, and it cost three gates

CI re-sat all three compile veins under rustc 1.98.1 on family0x6-model0xcf,
binary sha de5bfab22fbd, using the measure-and-refuse path built in the entry
above. Every number came back identical to the one recorded under 1.98.0:

| | 1.98.0 | 1.98.1 |
| --- | ---: | ---: |
| compile_allocs | 25,485 | 25,485 |
| compile_peak_bytes | 715,275 | 715,275 |
| compile_instructions | 41,831,767 | 41,831,767 |

So the four measured-on lines move and nothing else does. Welfare is unmoved
and is not re-set.

**One datum against the granularity these files pin.** `measured_on.sh`'s own
header says of rustc: *"the upstream version only, for the same reason clang
carries no package revision: nothing here shows a point release moving a count,
and pinning tighter than the evidence reds the gate on changes that are not
changes."* A patch bump that reddened three gates and moved no number is
exactly that shape, and glibc is the contrast the same header draws — its
Ubuntu revision is pinned because 2.39-0ubuntu8.7 against 8.8 was SHOWN to move
about four hundred instructions.

The pin is not loosened on one observation. Recorded so the next point release
is a second datum rather than a fresh surprise, and if it also moves nothing
the argument for reading rustc as major.minor is made on two.

**The two rows nobody re-sat are carried, and the file says so.** Zen 3 and Zen
4 hold 41,831,767 from 1.98.0. kanso#1230 removed a row rather than carry one
across a toolchain move; the difference here is that a measurement exists. The
toolchain changes the binary identically for every chip and the one chip re-sat
moved by nothing, so deleting two correct values to force sittings that would
reproduce them buys nothing. The assumption is written into the file and is
self-correcting: a carried row that is wrong goes red the first time CI lands
on that chip, which is the signal a missing row would have given one run later.

## 2026-09-03 (fourth) — the carry was wrong one run later, and the row is bimodal

**CORRECTS the entry above on two counts.** That entry said the rustc point
release "moved nothing" and defended carrying two chip rows across it. CI
answered both within ten minutes.

**The carry. DONE, the other way.** `bench/compile_instructions_by_cpu.txt`
already carried the rule — *a value measured against the old binary is worse
than no value* — and the entry above overrode it with an argument: a
measurement existed, the toolchain changes the binary identically for every
chip, and deleting correct values to force sittings that would reproduce them
buys nothing. CI landed on Zen 3 one run later and read 41,832,275 against the
carried 41,831,767. The rule was right and the argument was wrong. Zen 3 is
recorded from CI's own reading; **Zen 4 is removed**, because it has still
never been measured on this binary and carrying it a second time would be the
same mistake with the same argument.

**The row is bimodal, and it is CLAY'S CALL.** Four readings, tunables pinned:

| when | key | binary sha | rustc | counted |
| --- | --- | --- | --- | ---: |
| 12:33 | family0x6-model0xcf | 55fb850296d1 | 1.98.0 | 41,832,275 |
| 13:05 | family0x6-model0xcf | 55fb850296d1 | 1.98.0 | 41,831,767 |
| 16:25 | family0x6-model0xcf | de5bfab22fbd | 1.98.1 | 41,831,767 |
| 16:35 | family0x19-model0x1 | de5bfab22fbd | 1.98.1 | 41,832,275 |

Two values 508 apart. One chip produced both on one binary; two chips produced
different values on one binary. Neither the key nor the binary picks a mode,
so the 2026-09-02 ruling this file was built on — *the row moves with the CPU,
so record it* — rests on a premise that does not hold. The file's own header
had set out what these runs would test, and this is the branch it named: the
chips still disagree, so pinning was not the whole explanation.

An exact row is red about half the time on a chip that produces both modes.
That is a design question rather than a number to re-sit, and it is filed in
design/pending-gavels.md with four options and a recommendation. **It blocks
kanso#1232 and every branch after it**, because no value in that file makes CI
reliably green until it is ruled.

So the sitting's honest reading is narrower than the entry above claimed: the
allocation and peak counters reproduced exactly across the toolchain bump, on
every host that has run them, and the instruction count cannot be said to have
moved or held, because it has two values and the bump is not separable from
the bimodality. compiler.html §40 is corrected to match.

## 2026-09-03 (fifth) — the bimodality reaches the trend gate

Setting the bare `compile_instructions=` to the other mode made the trend gate
refuse the branch, in its own words:

    worsened: compile_instructions 41,831,767 -> 41,832,275
    FAIL  a pure regression: something got worse and nothing got better.

Right by its rules, and the claim is false — nothing in that branch touches the
front end. The number moved because the reference row moved from one mode to
the other.

**So the bimodality is not only a flaky row.** The first row of
`bench/compile_instructions_by_cpu.txt` is also the bare number welfare,
golden_prose and the trend gate read, so a mode flip there presents as a
counter that worsened with nothing traded. The objective's own regression
detector fires on noise, and what stands between it and a false regression is
which chip happens to be listed first.

The reference is family0x6-model0xcf, which has three readings against Zen 3's
one, and whose 41,831,767 is what main already carries. Zen 3 keeps its own
measured row below it. **The uncomfortable half is stated in the file**: this
ordering is also the one that makes CI green, and choosing a reference to quiet
a gate is exactly what that file exists to catch. It is not that move, because
the alternative asserts something untrue — but the reason has to be written
down rather than assumed, since the two are indistinguishable from the diff.

Added to the ledger entry as the strongest argument for ruling this rather than
living with it.

## 2026-09-03 (sixth) — the fifth reading, and three chips is enough

CI landed on Zen 4 — the row removed an hour earlier for never having been
measured on this binary — and read 41,831,767, the low mode, on sha
de5bfab22fbd. Recorded, because an unrecorded chip is exactly what the gate
asks for and because it is a measurement rather than a mode being chased.

|  | family0x6-model0xcf | family0x19-model0x1 | family0x19-model0x11 |
| --- | --- | --- | --- |
| on sha de5bfab22fbd | 41,831,767 | 41,832,275 | 41,831,767 |
| earlier, sha 55fb850296d1 | both | — | — |

Three chips, two values, one binary across all three. Intel reads low, Zen 3
reads high, Zen 4 reads low — and Intel read high as well, twenty minutes
before it read low, on one binary with byte-identical CPU feature blocks.

**So the chip does not select the mode**, and that is the whole premise
`bench/compile_instructions_by_cpu.txt` is named for. Two chips agreeing is
what the file already warns is not evidence they agree; three chips split two
against one, with the odd one out having previously read the other value, is
evidence of something else entirely.

Every row is CI's own most recent reading now and nothing is predicted from
another chip. Whether that is a stable arrangement or a coin flip per run is
the question in design/pending-gavels.md, unchanged by this reading except
that it is now five measurements rather than four.

## 2026-09-03 (seventh) — two of three chips have produced both values

A run against the fully recorded table — all three chips present, every row
from CI's own sitting — counted 41,831,767 and refused. The only row not
already holding that value is Zen 3, which read 41,832,275 eighteen minutes
earlier on the same binary.

| chip | on sha de5bfab22fbd |
| --- | --- |
| family0x6-model0xcf | both values, twelve minutes apart |
| family0x19-model0x1 | both values, eighteen minutes apart |
| family0x19-model0x11 | the low value once |

**So the mode is not a property of the silicon.** Two of three chips have
produced both, on one binary, with byte-identical CPU feature blocks where
those were compared. The modes are GLOBAL and the key
`bench/compile_instructions_by_cpu.txt` is built on separates nothing.

That simplifies what is filed rather than complicating it: what wants
recording is two acceptable values for the vein, not two per chip. The
ledger entry says so now.

**The rows are not flipped to the value just seen**, and this is the entry
that has to say why, because six readings in one afternoon is exactly the
pressure under which a session starts chasing. Setting each row to whatever
CI last read would make the gate green and would delete the finding, and the
finding is the only thing here worth having. They stay as measured.

Six readings is enough to decide on, so further ones go into the ledger's
table rather than earning entries here.

## 2026-09-03 (eighth) — what a failing test is allowed to tell you

`design/pending-gavels.md` has carried the assert hako since 2026-08-17 with
the gate lifted and a recommendation to build it. Built, and the measurement
that motivates it was taken first rather than assumed.

**What a failing `==` says.** A test is a constant and `==` is the assertion,
which decides pass or fail and nothing else:

    test_a_failure_on_purpose = decode "42" == 43
    test_a_failure_on_purpose ... FAILED (returned false)

The name, and no sign of what it got. The operands are gone by the time the
runner holds the boolean, so the runner cannot recover them — whatever carries
them has to be the assertion.

**The concept was already there.** The runner prints the value a test
returned, so a matcher answering a RECORD reports its own diff with no change
to the runner at all. The same wrong assertion, both ways:

    test_the_old_way ... FAILED (returned false)
    test_the_new_way ... FAILED (returned expect/mismatch 43 42)

That is the mushroom test passing rather than a feature being added. `lib/expect`
is `expect`, `to`, `equal` and `be_true` — about twenty lines, no builtin, no
runner change, and `expect` is the identity because the chain is the surface.

**Kept out of `lib/testing` deliberately.** That hako's header says "nothing
here adds a second way to write a test that can already be written", and a
matcher IS a second spelling of `==`. The distinction that earns it a package
of its own is that it is the only way to write a test that reports what it got;
the stance in `lib/testing` stays intact and the addition is opt-in.

**Watched red, and two of the four mutations the LANGUAGE refused.** Swapping
the mismatch record's fields reddens the two specs that assert its shape;
making the match arm answer a record instead of `true` reddens the two that
assert a match. The other two — a mismatch answering `false`, and a generic
first arm — do not compile: `error[unused]` on the now-dead bindings, and the
most-specific-first ordering rule. A gate the compiler enforces needs no spec,
and that is worth recording rather than counting as coverage.

**The surface shape is still Clay's**, and the ledger entry stays until he
rules on it. What has changed is that it is now a thing to read rather than a
thing to imagine, and the evidence for wanting it is a measurement.

## 2026-08-31 — rider: pure fallibility is boxed too

Clay raised `foo["bar"]!` against the effects-are-types gavel: an err
with no io underneath — value or effect? A crash-semantics reading
(insistence violated = defect, halt) was proposed and DECLINED by
Clay on the shipped mechanism: "you just get an error back and you
can handle it however you want and it bubbles up. so maybe it should
be an effect after all... since that's the only way to enforce the
bubbling." That entailment is the ruling:

- **Any operation whose answer includes an err yields `<t>effect`,
  io or not.** The box is defined by fallibility — the unresolved
  outcome — and io was never the criterion. One failure system.
- `foo["bar"]` stays the data form: the value or `none`, absence as
  ordinary data, no box (and post-`done`, `none` is unambiguous).
- `foo["bar"]!` yields `<v>effect`; the suffix-grammar contract ("a
  `!` function's answer typeset includes an err") re-reads as: a `!`
  operation answers a box.
- The bang family stays rescuable under the standing license — the
  map-collision reasoning holds (the err is raised in std, std is
  foreign to every caller). Nothing supersedes.
- Enforcement is why: under explicit elimination the box is the only
  carrier that makes bubbling mandatory. A bare err would need the
  retired railway; a box cannot be dropped or mistaken for a value.

Separately proposed and AWAITING GAVEL: the fused chain operators
(`.>` bind, `.!` annotate, `.?` rescue — reviving the archived `.>`
of 2026-08-16), bare-function right-hand sides, sole spelling in
chain position with the words remaining prefix functions. Not ruled;
recorded so the proposal is not re-derived.

## 2026-08-31 — gavel: the fused chain operators

Clay: "the fused operators are a Go." The three combinators gain
fused chain spellings — the chain dot plus one character of channel:

    config = io/read_file path
      .> json/parse                      # chain through bind
      .! (e -> "config: {e.reason}")     # chain through annotate
      .? when_failed                     # chain through rescue

- Each is pure sugar with a fixed desugaring: `x .> f` IS `bind x f`,
  `.!` annotate, `.?` rescue. Semantics, licenses, and the auto-
  rewrap live at the words; the parser learns three operators, not
  three meanings. `.>` revives the archived spelling of 2026-08-16.
- The right-hand side is a bare function — a lambda, a named
  function, or a dispatch group — no wrapper lambda for the common
  case: `.> json/parse`, `.? when_failed`.
- **In chain position the fused operators are the only spelling.**
  `. bind (f)` retires as a chain form, superseding the 2026-08-29
  keep-the-dot ruling for the three combinators specifically; plain
  `.` application chaining is untouched. The words remain ordinary
  prefix functions everywhere else (`rescue (foo["bar"]!) handler`),
  so each position has exactly one spelling.
- All three land together, per the no-yagni-in-language-design rule:
  a chain that can `.>` but must fall back to a word for annotate
  would be the inconsistency this family exists to remove.
- Pure fallibility benefits identically:
  `foo["bar"]! .? (e -> "anonymous")` is one-line handling with no
  ceremony, per the boxed-fallibility rider above.

## 2026-08-31 — directive: the welfare chart replays the current formula

Clay, seeing the #184 re-scoring as a cliff in the trend chart: the
graph should be continuous. The history rows store raw counters per
merge, so the chart's welfare line is to be RECOMPUTED — today's
formula, today's baseline, replayed over every stored row — rather
than plotting the number as it was computed at the time. The cliff
disappears because it was never a change in the compiler.

Rules of the replay:
- The recomputed series starts where its counters start. A segment
  computed over a subset of counters (before the instruction or
  text veins existed) is either omitted or visibly labeled; it is
  never passed off as the full formula.
- The floor history in bench/welfare_floor.json stays exactly as it
  is — it is the audit trail of when the objective moved, and it is
  not smoothed. The chart reads the replay; the audit reads the
  floor file.
- Every future formula or baseline change re-runs the replay in the
  same PR, so the chart is always one definition applied everywhere,
  never a splice.

## 2026-09-02 — gavel: the weights, tuned to the developer's order of noticing

Clay, asked whether the welfare weights were justifiably optimal,
restated the objective: "the core thing we're attempting to optimize
for here is ultimately developer happiness. performance should be
insane, but if a language doesn't compile wickedly fast, like go,
devs may choose not to use it in the first place." He took the
recommendation that followed. The weights argument, recorded as the
doctrine requires before the floor moves:

    term             was    now   satiation
    run speed        0.30   0.30  2.0   (unchanged)
    run memory       0.30   0.26  2.0
    compile speed    0.28   0.32  0.5
    compile memory   0.12   0.12  0.5   (unchanged)

- **Compile speed rises, funded from run memory.** A developer feels
  compile latency on every edit and feels peak memory only when
  something falls over. Compile latency is an adoption gate; the
  memory model is the identity, but the objective's job is to punish
  regressions in the order developers notice them. Compile memory
  stays at 0.12 — the compiler's own footprint is a CI-container
  guard, not something a developer perceives.
- **The run-speed term splits in two halves.** Half is the equal-
  weighted mean of the advertised workloads — decode (jsonbench) and
  encode (encodebench), the rows the front page makes claims about
  and the workload the language exists for. Half is the equal-
  weighted mean of every other run benchmark in the objective — the
  shape guards, today oneshot, basket, wide, deep, pending, scan and
  digest (the last two joined in #1215) — which stress the memory
  model and protect against pathologies. The rule is "advertised
  versus everything else", so a benchmark added later lands in the
  guard half without this entry needing an edit. A shape win no longer scores as if a real workload
  got faster; the guards stay fully armed against regression. Per-
  counter saturation (the #184 ruling) applies inside each half
  before its mean.
- **Satiation constants stand.** Compile at 0.5 already says the
  target is instant and past instant nothing more is bought, with
  the curve's asymmetry making a compile regression cost more than
  the equivalent gain earns — the adoption-gate shape. Runtime at
  2.0 says eight times faster is eight times faster.
- **The exclusions stand**: wall time (nondeterministic), binary
  size (measured pointing the wrong way, #1217/#1219, pinned by
  spec), and everything unmeasurable, which the floor-permeable-to-
  language rule keeps welfare from vetoing.

Lands as a weights change: recorded here, `--set` in the same PR with
this entry's reason, the floor re-set, and the chart replay re-run so
the history reads under one definition.

## 2026-09-03 (ninth) — the compile row was counting the host's memory map

**DONE (the finding). Clay ruled on the entry the same day, and the ruling is
option 1 with the term named: make the row deterministic with glibc still
counted — no pinned pair, no band, no exclusion.** His words: "if changes to the
compiler can interact with glibc in a way that means generally more/less work,
which the 'compiler's own instructions' measure would be blind to, then we need
a way to include glibc's instructions but make them consistent."

That is right, and it kills the fix this session had built. Collecting from
`kanso::main` and leaving startup out would have made the vein blind to exactly
the case he names: a compiler change that causes glibc to do more work. The
toggle is dropped.

What survives is the measurement, because the ruling asks for the term and this
is the term.

`kanso check` reads `/proc/self/maps` before it reads a line of kanso. glibc's
`pthread_getattr_np` does it on Rust's behalf, to find the main thread's stack:
open the file, read it in 1024-byte chunks, `sscanf` each line until the one
holding the stack pointer turns up. What that costs is a property of the host's
memory map — how many mappings it has, how long their pathnames are — and this
vein has been counting it since it was minted.

The container reads the row deterministically, which is what made this
findable: forty runs of the gate's exact command on one binary returned one
value forty times. So the knobs below move a number that does not otherwise
move.

    one more shared library in the process     +32,090

That is an `LD_PRELOAD` of an empty `.so`: four more lines for the parse to
walk, and thirty-two thousand instructions the compiler never executed.
Smaller edits to the same text give smaller steps. Lengthening the executable's
own file name from nine characters to ten moves the row +2,193, and the profile
diff across that pair names the maps parse and nothing else — `__vfscanf_internal`
+1,180, `____strtoul_l_internal` +576, `_IO_sputbackc` +88, then `getdelim`,
`getline`, `_IO_setb` and `pthread_getattr_np` itself. Zero kanso symbols move.

That is the signature the six CI readings recorded: "every kanso symbol agrees
to the instruction ... three rows differ and all three are glibc". A startup
term is global rather than per-chip, which is why two of three chips produced
both values, and why pinning the glibc tunables took the spread from 5,064 to
508 without closing it.

The 508 itself is the downstream half. What the parse allocates and frees
before `main` sits under everything the compiler allocates afterwards, so
`_int_malloc` walks different bins and `memcmp` compares at different
alignments for an identical request sequence. A second knob isolates that half:
`argv[0]` at 39 characters rather than 38 moves the row +480, all of it in
`__memcmp_avx2_movbe`.

**It can be measured out**, which is why this is an instrument repair and not a
question for Clay. Collecting from the compiler's own entry point rather than
from `exec`:

    valgrind --tool=callgrind --collect-atstart=no --toggle-collect=kanso::main

                                  raw        toggled
    one more shared library    +32,090             -6
    name 9 -> 10 characters     +2,193             +5
    36 name lengths, spread      2,900             90

Ten toggled runs on one binary returned one value ten times. The residual 90 is
`argv[0]`, which the box already holds fixed at `./kanso`.

**OPEN — the rewiring itself.** Changing what the gate collects invalidates
every row in `bench/compile_instructions_by_cpu.txt`, the bare golden the trend
gate and `golden_prose` read, welfare's baseline for the counter, and the pinned
figure in the compiler page. Only CI may write those values, one chip per run,
so it is its own change and lands red until it is re-sat. Two things it owes:
a guard, because a `--toggle-collect` whose function is missing yields
`summary: 0` and exit 0 — a silent zero that would be reported as the front end
moving; and a welfare re-baseline in the same commit, because the number falls
by about 1% without a single instruction of work being removed, and banking that
as a win would be recording an instrument change as an improvement.

## 2026-09-03 — gavel: the bimodal row is made deterministic, not excused

On "The compile-instructions row is bimodal" (#265), Clay declined
both the pinned pair and an exclusion of glibc from the count: "if
changes to the compiler can interact with glibc in a way that means
generally more/less work, which the 'compiler's own instructions'
measure would be blind to, then we need a way to include glibc's
instructions but make them consistent, like how rspec can run with a
seed... you run some instruction at the top to clear out the glibc
state in test mode. this is crucial to be specific about."

So the ruling is option 1 with the term named. The measured signature
— five consecutive runs in one container agree exactly, two runners
with the same binary and chip disagree — says the randomness is fixed
at container birth, not per exec. Two suspects fit that shape and
both are cheap to test:

1. **Directory read order.** `kanso check lib/json` reads a source
   directory; ext4's readdir order is a hash order seeded per
   filesystem instance, so a fresh runner disk orders the same files
   differently, consistently within one VM. If the loader does not
   sort entries before compiling them, the allocation sequence — and
   so the heap layout, and so the alignment paths in `_int_malloc`
   and `memcmp` — differs per VM. This is a build-determinism defect
   independent of the vein: the compiler's work must never depend on
   inode hashes. Fix: sort in the loader.
2. **ASLR.** Heap base and mmap addresses randomize per exec unless
   disabled; `setarch -R` around the measurement removes the term.

Environment (env -i), glibc tunables and glibc revision are already
pinned. Apply both fixes, measure on several fresh runners; if the two
modes collapse to one, glibc stays in the count under a single exact
pin — no pair, no band, no exclusion. Only a residual that survives
both reopens the question, and then the fallback is the pinned pair,
never blindness. The entry leaves Blocking with this commit; the
rustc-patch rider is the implementer's (the pin did its job;
regenerate with a sentence).
## 2026-09-03 (tenth) — the bimodal row is made deterministic, not excused

**DONE for the two suspects, OPEN for the answer.** Supersedes the OPEN
paragraph closing the entry above, which planned to exclude process startup from
the count. Clay ruled against that the same afternoon: "if changes to the
compiler can interact with glibc in a way that means generally more/less work,
which the 'compiler's own instructions' measure would be blind to, then we need
a way to include glibc's instructions but make them consistent." He is right,
and the toggle is dropped — it would have made the vein blind to exactly the
case he names. No pair, no band, no exclusion.

The ruling named two suspects, both fitting the measured signature: five
consecutive runs in one container agree exactly while two runners on one binary
and chip disagree, so the randomness is fixed at container birth rather than per
exec. Both are now tested and both answers differ from the guess.

**Readdir order was already sorted, and cannot be the term.** `src/lib.rs`
sorts the files a compile reads before reading any of them; `src/eval.rs` sorts
what `list_dir` answers. The four remaining `read_dir` calls are in
`src/main.rs` on the `test` path and order-independent by construction — three
are `.any()` or `.count()`, and the fourth is a vector whose only order-sensitive
use is a single-element match.

One was sorted anyway on the rule rather than the symptom, and then WITHDRAWN,
which is the more useful finding. It changed no answer and it changed the
binary: `.text` 2,514,718 -> 2,517,278, moving this row 41,831,767 -> 41,834,008
and invalidating every recorded value. That is the wrong price for a defensive
sort, and worse than its cost is what it did to the experiment — the ruling asks
for the modes to be measured away on several fresh runners, and a changed binary
makes every earlier reading incomparable, so `setarch -R` and a new layout would
have been confounded from the first run. With the sort out, the binary is
main's, the recorded rows stand, and setarch is the only variable. The
determinism fix is worth landing on its own once this vein is settled and the
row it moves is not the row under test.

**ASLR is disabled now, and on the container it changes nothing.** `setarch -R`
reads 42,235,790 against 42,235,790 without it, and forty unwrapped runs on one
binary returned one value forty times. Applied regardless: the modes have only
ever appeared on the runners, and a container cannot rule out what it has never
reproduced. The gate prints `compile_aslr disabled=` so the job log says whether
the wrapper took; CI reads `yes`.

**Nothing is priced, because nothing moved.** With the sort withdrawn this
branch changes `scripts/gates/compile_instructions.sh` and no compiler source,
so the binary is byte-identical to main's and every counter reads what main
records. The trend gate has nothing to say and welfare holds at 76.1742943805134
against its own floor. An earlier revision of this branch did move
`compile_instructions` +2,241 and spent 0.00015 of the score declaring it; that
is reverted, and the declaration with it.

**The evidence so far, and it is not yet an answer.** Two CI readings on binary
sha b0e5a906c73d, twelve minutes apart, both on family0x19-model0x11, both
41,834,008 — the first agreement on one binary since the pair appeared. That
binary is the withdrawn one, so the agreement is kept as history rather than as
a result: it says the value repeats, on a tree that no longer exists. Against
that, a control taken at the same time on kanso#1235, which runs the UNFIXED
gate: 41,831,767 on a design-only diff, then 41,832,275 with one HTML file
added, same binary, 508 apart. So the modes were live on the old gate minutes
before the new gate read the same value twice.

**OPEN.** Two agreeing readings on one chip is not enough. What settles it is
the other chips landing on their own values and holding them. If they do, glibc
stays counted under a single exact pin and the per-chip key separates nothing —
the file collapses back to one golden. If a second value appears on a recorded
chip, neither suspect was the term, and the entry above already names what is
left: glibc parses `/proc/self/maps` before `main` to find the stack bounds, one
more shared library in the process moves the row 32,090, and that cost belongs
to the host's memory map rather than to the compiler. That is what "make them
consistent" would then have to reach.

## 2026-09-03 (eleventh) — one binary, one chip, two values: the pair is pinned

**DONE.** Closes the OPEN half of the entry above, which had the two suspects
tested and the answer outstanding. Searched the live log and
`design/log/compiler-log-archive.md` for prior treatments of the compile row's
spread before filing: the archive carries the 5,064-apart clusters that the
glibc tunables closed, and the live log carries the ninth and tenth entries.
Nothing there proposes a pinned pair, so this is new.

**Both suspects are falsified, and `setarch -R` comes back out.** The ruling
applied it on the argument that the modes only ever appear on the runners and a
container cannot rule out what it has never reproduced. That was the right
reason to try it and CI has now answered: `e47e412d` printed
`compile_aslr disabled=yes` and counted 41,832,275 on a chip whose row held
41,831,767. It moved nothing on the container either — 42,235,790 against
42,235,790, and forty unwrapped runs returning one value forty times. A knob
measured twice to move nothing is not carried, so the gate loses it and keeps
the finding.

**What settles it is the sha the gate prints.** `compile_binary sha256=` was
added to pair a reading with the binary that produced it and had never yet
answered a question. It answers this one:

| commit | binary sha | cpu | counted |
|---|---|---|---|
| `fc993f83` | `de5bfab22fbd` | family 0x6 model 0xcf | 41,831,767 |
| `e47e412d` | `de5bfab22fbd` | family 0x6 model 0xcf | 41,832,275 |

One binary, one chip, two values, eight minutes apart. The second ran with
`setarch -R` and the first without, so the same pair is the falsifier for the
last suspect. A third run — main at `5b0f2eb1` — counted 41,832,275 on
`family0x19-model0x11` against a recorded 41,831,767, so the second chip has
shown both values on this binary too.

I got this wrong once in the middle of the investigation and it is worth
recording how. Two runs agreeing at 41,832,275 on one sha, against rows recorded
on an older sha, read as "the binary moved and the rows are stale" — a tidier
answer than bimodality, and I had reverted the pair machinery on it before
fetching the third log. `fc993f83` killed it: same sha, same chip, the low
value. Two points that agree are consistent with almost anything.

**So the row pins a pair, which the ruling pre-authorised**: "only a residual
that survives both reopens the question, and then the fallback is the pinned
pair, never blindness." `scripts/gates/compile_ir_row.sh` grows three refusals
— a row that pins neither one value nor two, a row that pins three, and a pair
whose halves are the same number — and the lookup takes every value on the row
and asks whether the count is among them. Two is a cap and not a convention: a
band wide enough to hold 508 also holds kanso#1226's -5,621, which was a real
change to the compiler. The golden's bare line stays the reference row's FIRST
value, so a mode flip cannot reach welfare or the trend gate as a regression;
`bench/compile_instructions_golden.txt` is byte-identical to main's on this
branch, which is the check that it cannot.

Six specs, all watched red first, and three ratchet mutations verified to redden
this suite: the new `a_pinned_pair_grows_into_a_band`, plus the two existing
mutations whose refusal-count guards had to move from five to seven.

**A fourth chip, recorded the same afternoon.** CI landed on
`family0x6-model0xad` — Granite Rapids, new to the pool — and the gate refused
rather than passing on another chip's number, which is the unrecorded-chip
design doing its job. It counted 41,832,275 on the same binary
`de5bfab22fbd`, so three of the four recorded chips have now read the high
value there and two of them have read both. It is recorded as a SINGLE,
because one reading is one reading and a row may not claim a mode nobody has
seen it take.

That standing is an argument the per-chip key is separating nothing on this
binary: the value looks like a property of the run rather than of the silicon.
Not acted on. The key costs nothing while it is still right about the
cross-binary case the file's header decomposes, and one binary is not enough
to retire it.

**OPEN, and stated as a measurement rather than a plan.** The ninth entry's
term — glibc parsing `/proc/self/maps` before `main`, at a cost that is a
property of the host's memory map — is still the only mechanism that fits, and
it is not established that two runners differ in their maps by the 508 this
needs. The row would have to print the map's line count beside the count to
say. Nothing rests on the answer now that the pair holds the gate green.


## 2026-09-03 — gavel: failures are for the exceptional; the bang chooses the channel

Clay, correcting a "map ten files and collect the results" example:
"if you're talking about files you expect to be there, then them not
being there is exceptional. if you know it's possible for them to not
be there, you wouldn't use an exception, you'd just return a
file_not_found type." Gaveled as doctrine:

- **A failure is for the exceptional. An anticipated outcome is
  data.** If an alternative is part of the operation's normal
  vocabulary, the answer is a typeset — `text | file_not_found` — and
  dispatch handles both arms like any values: no box, no bubbling, no
  rescue license. The failure channel, with its provenance and cause
  chain, is for what the program did not plan for.
- **The bang chooses the channel, everywhere.** The map rule already
  ruled — `foo[k]` answers `none` as data, `foo[k]!` answers a
  failure — generalizes to every operation with an anticipated
  alternative, io included: `io/read_file path` answers
  `text | file_not_found`; `io/read_file! path` is the caller
  insisting, and a violation bubbles as a failure. The suffix
  grammar's contract (a `!` name answers a box) is the same rule seen
  from the declaration side. The canonical chain reads
  `io/read_file! path .> json/parse .! ... .? when_failed`.
- **Downstream, not decisions:** containers may still hold results
  (test matchers, supervisors, #1057), since holding is neither
  proceeding-as-success nor an unmarked conversion; and the io
  boundary deep-demands the program's final value, so a failure
  buried in an output structure fails the run unless it was rescued
  into data first.

The renaming of the box from `effect` to `result` is recommended
beside this and awaits its own word.

## 2026-09-03 — rider: the replay is ruled; the page's rule yields to it

The cloud session held the chart replay because docs/numbers.html
says "the welfare line is recorded, not recomputed... replaying
history against today's baseline would rewrite it," and because the
counter set has grown (digest in #1198; scan, escape, index in #1215),
so a commit predating a counter has no value for it. Both points are
answered by the record as it stands:

- **The page's rule is older than the gavel and yields to it** —
  later replaces older. Clay ruled the replay on 2026-08-31 ("yes"),
  seeing the #184 re-scoring as a cliff that "was never a change in
  the compiler." The page's sentence is rewritten to say what the
  chart now shows: the current formula and baseline, replayed over
  the stored rows, so the line is one definition applied everywhere.
- **No backfilling.** The replayed series begins at the first commit
  for which every counter in the current formula exists. Earlier
  history is not invented through the granted-baseline machinery
  (which admits a counter going forward, never backward); it is
  either omitted or drawn from the recorded scores in a visibly
  distinct style and labeled as scored under earlier definitions.
  The directive of 2026-08-31 already said this; it is restated here
  so it cannot be read as under-determined.
- **The audit trail is untouched.** bench/welfare_floor.json keeps
  every step with its reason — the 87.85 -> 73.83 re-scoring
  included — and that file, not the chart, is where "what did a
  commit ship with" is answered.

## 2026-09-03 (twelfth) — the third chip shows both values, on one binary

**DONE, and it is the gate reporting rather than a change.** Searched the live
log and the archive before filing: the eleventh entry pins the pair and names
two chips that had shown both, and nothing there records a third.

CI on kanso#1235 counted **41,831,767** on `family0x19-model0x1`, whose row
pinned only 41,832,275, on binary sha `de5bfab22fbd` — the same binary every
reading in this sequence was taken on. The row becomes a pair. The refusal was
correct and the number was real, which is the whole point of a pin that admits
exactly what has been measured and nothing else.

The standing on this binary:

| chip | values seen |
|---|---|
| family0x6-model0xcf | both |
| family0x19-model0x1 | both |
| family0x19-model0x11 | both |
| family0x6-model0xad | 41,832,275 only |

**Three chips of four have produced both values and no chip has produced a
third.** Two claims follow, and only the first is made here. The mode is a
property of the run rather than of the silicon — three independent chips each
producing the same two numbers is hard to read any other way. The second, that
the per-chip key can therefore be retired, is NOT claimed: the fourth chip has
one reading, and a key is retired on evidence rather than on a pattern that
holds for three quarters of the rows.

What would settle it is stated so the next session does not have to design an
experiment: `family0x6-model0xad` reading 41,831,767. Every recorded row would
then hold the same pair, the key would demonstrably distinguish nothing on this
binary, and the file could collapse to one golden holding two values — the
outcome its own header predicted for the case where pinning worked and the
chips stopped disagreeing. Until then the key is kept, because it is still
right about the cross-binary case the header decomposes, where two chips read
5,124 apart.

The mechanism is still the open question and is unchanged by this: glibc's
`/proc/self/maps` parse fits the signature, and what is not established is that
two runs differ in their maps by the 508 this needs.
