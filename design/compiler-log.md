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

## 2026-09-04 — THE ROW IS RE-SAT, AND THE BASELINE MOVES WITH IT SO NOBODY BANKS A DEFINITION

**DONE.** CI counted the compiler's own frame on Zen 4 (family0x19-model0x11),
sha 4dc725bdb40d:

    whole process   41,845,704   what the row used to read, to the instruction
    kanso::main     41,379,840   what it reads now
    dropped            465,864   loader and stack guard

That is 742 from the container's 465,122 on a different toolchain and binary,
which is the agreement worth having: the quantity being dropped is process
startup, and it is the same size in both places.

**The other two chips' rows are removed rather than carried.** The definition
changed for all of them and CI can only re-sit one chip per run, so Zen 3 and
Intel are re-sittings when they next refuse. This file has done exactly that
twice before, for the same reason: a value measured under the old definition is
worse than no value. The pair the AMD rows carried is not re-sat either —
whether it survives is for CI to say, and the layout term it came from is most
of what this change removes.

**The welfare baseline moves by the same 465,864**, 57,029,831 -> 56,563,967.
The startup half is an additive constant the old compiler paid too, so
subtracting it is what the baseline would have read under the new definition.

**And that raises the score by 0.01, which is not a win and is recorded as
not one.** 73.5162 -> 73.5256. Removing a constant from both sides of a ratio
raises it: 1.362866 becomes 1.366938, because the constant was diluting a
measured improvement. The compiler did not get faster. The measurement got more
sensitive by about a third of a per cent of its own term, and the `--set`
reason says so in those words. A reader who finds this rise in the history and
looks for the commit that earned it will find this entry instead.

**What the trend gate will report on the next run is a fall of 465,864 in
`compile_instructions`, listed as better.** It is not better. It is the same
compiler measured from a different frame. The gate has no way to tell a
definitional re-basing from a win — it can tell a MINTED counter, which has no
baseline at all, and this one has a baseline that moved underneath it. Whether
that deserves machinery is a real question and not one this change answers.


## 2026-09-04 — A COMMENT LINE SPLIT IN HALF BECAME A ROW, AND THE DUPLICATE CHECK CAUGHT IT

**DONE, and the defect was mine.** The re-sitting entry above put its note into
`bench/compile_instructions_by_cpu.txt` by inserting at the first occurrence of
`family0x19-model0x11` in the file. That occurrence is inside a comment
PARAGRAPH written months ago, not at the row, and the insertion cut the line
after its `# ` — leaving

    family0x19-model0x11 rather than carry a value measured against the previous

as a bare line with no `#`. The file's parser reads a key and a value off any
non-comment line, so the chip acquired a second row whose value is the word
`rather`.

CI refused, precisely and by name:

    bench/compile_instructions_by_cpu.txt has more than one row for a chip:
        family0x19-model0x11
    Only the first is ever read, so the rest say nothing while looking like a
    sitting.

That check went in on 2026-09-03 for a different reason — a re-sitting that
appended instead of editing — and it caught a case nobody had in mind: a
comment turning into a row by accident. **The first row is the one welfare
reads, so had the check not existed, `compile_instructions` would have been the
string `rather` and the failure would have surfaced somewhere much further from
its cause.**

The note is above the row now, the comment line is whole, and `compile_ir_row.sh`
reads 41,379,840 on the key.

**The refused run had already taken its sitting, and it was a different chip.**
`compile_sample cpu="cpu family 0x19 model 0x1" sha=4dc725bdb40d row=41379840`
— Zen 3, same binary as the Zen 4 sitting, same value to the instruction. So
the file gains a second row from a run that failed: the measurement is printed
before the comparison, which is what makes it usable. The two AMD models agree
on this binary, the fourth on which they have, and the keys stay separate for
the reason the header already gives.


## 2026-09-04 — THE GATE CALLED A DEFINITION CHANGE A WIN, AND THE SENTENCE IS IN THE MERGE

**DONE.** The entry above this one moved `compile_instructions` from 41,845,704
to 41,379,840 by counting the compiler's own frame instead of the whole process,
and subtracted the same 465,864 from welfare's baseline so the ratio stayed
comparable. Nothing in the compiler got faster. What the trend gate printed on
that merge, verbatim:

    improved: compile_instructions 41,845,704 -> 41,379,840  (bench/compile_instructions_golden.txt)
    every changed counter is priced (or improved).

The entry noted the sensitivity and left the question of machinery open. It is
answered by the run: the gate misreports today, and the misreport is permanent
record.

**The signal is welfare's baseline, and it is exact.** `welfare --set` moves the
FLOOR and never the baseline, so a baseline value moves only when somebody
decides the old reading and the new one are not of the same quantity. A counter
whose golden moved and whose baseline moved in the same diff is therefore
neither better nor worse — it takes the third state kanso#1200 built for minted
counters, prints as `re-based`, and counts toward neither side of the
pure-regression rule. It still owes a sentence: the same naming check a
worsening gets, because the value it landed on is what a write-up of a re-basing
states.

**WHAT THE MATCH REACHES, and what it does not.** Welfare renames the runtime
counters on the way into the objective — `jsonbench` is `decode_instructions` in
the baseline — so matching a golden row to a baseline key by name covers the
compile vein and misses the runtime one. Rather than copy welfare's rename table
into a second place to go stale, a baseline that moved and matched no golden row
is listed by its welfare name under RE-BASED, unclaimed, and refused unless the
log delta names it. A runtime re-basing is then visible and priced even though
the gate cannot say which row it belongs to.

**Reaching the rest is not "ask welfare for the mapping", and the measurement
says why. OPEN.** Written that way in this entry's first draft, then checked:
`welfare --counters` prints 27 counters, and matching each one's value against
every row of every golden the trend gate walks leaves 8 unmatched. Two are
ambiguous — `decode_peak_bytes` reads 2,097,152, which is also
`arena_peak_bytes` in the oneshot, basket and wide goldens — and six have no row
carrying that value anywhere, because `peak_of` SUMS three pools:
`arena_peak_bytes + held_peak_bytes + perm_peak_bytes`. A `*_peak_bytes` term is
a derived quantity, not a row.

So the link is not a rename table and cannot be a bijection: it is many-to-one
for every memory term, and welfare does not know the trend gate's prefixes
(`work_jsonbench` is the gate's spelling of the row welfare calls
`decode_instructions`). What would close it is an explicit table naming, for
each objective counter, the gate keys it is derived from and how — identity or
sum — in one place, with a spec that replays the derivation and asserts it
reproduces welfare's own reading. That is a real piece of design rather than a
plumbing change, and this entry records the measurement that rules out the
cheap version.

**Watched, both directions.** The gate at f3047edd against 1a0cb51e reproduces
the `improved:` line above; the same pair under the new gate reads `re-based:`
and lists the counter. The mutation
`a_re_basing_that_pays_for_a_regression` moves the compile golden and the
baseline together beside a plain `work_jsonbench` worsening, with both moves
written up: green under the old gate at exit 0 — a real regression paid for by a
definition change — and refused under the new one by the pure-regression rule.
Deleting the mutation's log paragraph turns that refusal into UNPRICED, which is
red for the wrong reason, so `tests/a_re_basing_row_stays_a_pure_regression.rs`
holds the paragraph to the values the `sed` lines write. Both of its assertions
were watched red. The orphan path was watched too: moving
`decode_instructions` alone is refused as UNSTATED and passes once a sentence
names it.

## 2026-09-04 — A BENCHMARK JOINED THE OBJECTIVE AND THE GATE THAT WATCHES THE OBJECTIVE COULD NOT SEE IT

**DONE.** Found while mapping welfare's counters onto the trend gate's keys for
the entry above: `bench/cost_golden_read.txt` is not in the gate's golden list.
readbench joined the objective the day before (kanso#1240) with
`read_arena_blocks` and `read_peak_bytes` as welfare terms, the golden was
written, and the one program whose job is to watch the objective's inputs was
never told about the file. Either row could have moved by any amount in
silence.

This is kanso#1046's finding one benchmark later — *"half the score's inputs
were invisible to the gate that exists to watch the score's inputs"* — and it
was found the same way both times, by asking which files in `bench/` the gate
names. That is now the only method that has ever found one of these, and the
list is short by one every time a benchmark lands.

Setting `arena_peak_bytes` in the read golden to 9,999,999,999 produces NO
OUTPUT WHATEVER from the gate at f3047edd, exit 0. With the file listed:

    worsened: read_arena_peak_bytes 1,048,576 -> 9,999,999,999  (bench/cost_golden_read.txt)

and a refusal. `a_read_counter_worsens_for_nothing` is that mutation, rowed.

`bench/compile_libraries_golden.txt` is the only other file in `bench/` the
gate does not walk, and it belongs there: it holds five sonames rather than
counters, and its own gate diffs it byte for byte.

## 2026-09-04 — THE LINK IS WRITTEN DOWN, AND THE RUNTIME HALF OF THE RE-BASING CHECK WORKS

**DONE, and it closes the OPEN in the entry two above.** That entry said the
many-to-one link between welfare's counters and the trend gate's keys "is a real
piece of design rather than a plumbing change". Built and it is a table:
`bench/objective_sources.txt`, 41 rows for 27 counters, `<objective counter>
<gate key>` a line, several lines for a counter that is a sum.

Twelve counters are one row each (`decode_instructions work_jsonbench`), eight
more are identities on the compile goldens and the arena blocks, and seven
`*_peak_bytes` are three rows apiece because `peak_of` adds the arena, held and
perm pools. Written and then replayed against `welfare --counters`: 27 counters,
0 unreproduced.

**A prefix, not a name.** The first cut classified nothing in the runtime vein
and I nearly recorded that as a limit again. `unchanged` was handing `shifted?`
the counter with its golden's prefix STRIPPED — `jsonbench`, where the link file
names `work_jsonbench` — so the check worked only for the four goldens whose
prefix is empty, which is exactly the compile vein it was first written for. The
probe that caught it was moving `jsonbench` and `decode_instructions` together
and watching the gate still say `improved`. It says `re-based` now.

**The spec is the whole of it, because a written link is one a rename breaks
silently.** `tests/the_objective_reads_what_the_gate_watches.rs` reads the
gate's own golden-and-prefix bindings, sums each counter's rows out of those
files, and asserts the total is what welfare prints — for every counter welfare
scores and no counter it does not. Watched red four ways: a counter dropping out
of the file, a row renamed on the golden side, a name the file invents, and a
nonzero pool removed from a sum.

**What it cannot see, said rather than left to be found.** The check is on
totals, so a pool reading nought contributes nothing and dropping its row costs
the sum nothing. Every `held_peak_bytes` and `perm_peak_bytes` in the goldens is
nought today, which makes twelve of the 41 rows unfalsifiable right now. The
direction that matters is covered: a pool added to `peak_of` and not to the file
is nonzero by the time anyone cares, and a dropped zero row starts failing the
day its pool carries a byte. The spec's own header says so.

The `RE-BASED, unclaimed` listing survives with a narrower meaning: a baseline
that moved while not one of the golden rows it is made of did — a ratio moving
with no measurement behind it — and it still owes a sentence.

## 2026-09-04 — A THIRD CHIP, AND IT READS WHAT THE OTHER TWO READ

**DONE.** CI refused kanso#1242 on `family0x1a-model0x2`, a key
`bench/compile_instructions_by_cpu.txt` did not carry:

    nothing in bench/compile_instructions_by_cpu.txt was counted on
    family0x1a-model0x2, so this run's 41379840 cannot be compared to anything.

The branch's diff is scripts, tests, bench data and this log — `git diff
origin/main -- src/ lib/ Cargo.toml Cargo.lock` is empty — so the front end is
untouched and this is a chip new to the pool rather than a row gone stale. Its
sitting is **41,379,840**, the same figure to the instruction as Zen 4 and
Zen 3 on the same binary. Three keys, one number.

The row goes LAST. The first row is what welfare and
`compile_instructions_golden.txt` read, and moving that authority to a chip
because it happened to be today's runner is how a value ends up published for a
reason nobody chose. Checked after adding it: all three keys resolve to
41,379,840 and an invented key still refuses.

That the keys agree is not an argument for merging them — the file's header
gives the reason, and this is the fifth binary on which two or more have agreed
while the noise the key exists for stays real.

**A clippy warning in the same round.** `manual_is_multiple_of` on the
comma-grouping helper in `a_re_basing_row_stays_a_pure_regression.rs`. Fixed;
`cargo clippy --all-targets --all-features` is at zero warnings.

## 2026-09-04 — THE LOG IS BACK TO FORTY, AND ONE OPEN THREAD WENT WITH THE FIVE

**DONE.** Five entries move to `design/log/compiler-log-archive.md`, unedited
and in order: the whole 2026-09-03 chip series from *a recorded chip
disagreeing with itself* through *the bimodality reaches the trend gate*. The
live file was at 44 against a budget of forty.

**One of them is still OPEN and this entry is where it stays findable.** The
first of the five holds *the third cause*: `family0x6-model0xcf` counted
41,832,275 at 12:33 and 41,831,767 at 13:05 — 508 apart, same key, same binary,
no front-end change in the diff — so the key did not determine the row, and the
per-chip table rested on a weaker premise than kanso#1226 claimed for it.

**THE MECHANISM IS ALREADY MEASURED, AND THE REDEFINITION DOES NOT REMOVE IT.**
I first wrote that the 508 was glibc parsing `/proc/self/maps`, which #1241 now
drops, and that the thread had therefore got cheaper. The archive says
otherwise, in an entry that corrects the very guess I was repeating. The two
profiles differ in three rows and all three are glibc's allocator:

    _int_malloc        1,551,384   1,551,964   +580
    _int_free          1,522,333   1,522,352    +19
    memcmp-avx2-movbe  1,353,408   1,353,342    −66

Every kanso symbol agrees to the instruction across the pair. `_int_malloc` and
`_int_free` are calls the compiler makes, so they sit INSIDE `kanso::main` and
the new definition counts them exactly as the old one did. The thread is as open
as it was.

**What #1241 did give it is a name for the mechanism.** Its seven-binary table
found the same signature — `__memcmp_avx2_movbe` moving 402 while every other
in-frame symbol held — and traced it: growing `.text` moves the end of `.bss`,
the kernel starts the break after it, and every allocation lands at a different
alignment. The 508 is that phenomenon between two runs rather than between two
binaries, which sharpens the open question to a single one: what moves the heap
base when the binary sha, the 123-line cpu feature block, glibc, valgrind and
the environment all agree? Pinning the malloc tunables took the cluster from
5,064 to 508 and did not remove it, and no tunable pins where the break starts.

A third chip was recorded on kanso#1242 — `family0x1a-model0x2`, reading
41,379,840, the same to the instruction as both AMD models. That says the keys
agree on this binary and says nothing about whether one of them disagrees with
itself.

## 2026-09-04 — THREE OPEN MARKERS THE RECORD HAS ALREADY ANSWERED, AND ONE THAT MISLEADS

**DONE.** Searched the live log and `design/log/compiler-log-archive.md` before
filing: the archive carries the 2026-09-03 allocator measurement and the chip
series moved there this afternoon, and nothing in either closes these by name.
The live file carried five `OPEN` markers. Three are answered, and one of the
three is worse than stale.

**A LIVE ENTRY ASSERTS A HYPOTHESIS ITS OWN CORRECTION SITS IN THE ARCHIVE.**
The ninth-entry thread reads *"glibc parsing `/proc/self/maps` before `main` is
still the only mechanism that fits"*. It is not. The archived entry that
corrects it diffs the two profiles and finds three rows moving, all of them the
allocator:

    _int_malloc        1,551,384   1,551,964   +580
    _int_free          1,522,333   1,522,352    +19
    memcmp-avx2-movbe  1,353,408   1,353,342    −66

Every kanso symbol agrees to the instruction across the pair, and the entry
rules the cpu out on 123 byte-identical feature lines.

**I repeated the error today, which is the evidence that the shape is the
problem rather than the wording.** Writing kanso#1243 I restated the maps
hypothesis from the live entry, concluded that kanso#1241 had removed the
mechanism, and pushed that. The archive corrected me on the read. A reader of
the live log alone would make the same move, because the hypothesis is here and
its refutation is forty thousand lines away with no pointer from this side. The
trim rule says the archive keeps the older end and this file keeps the last
forty; it does not say a live entry may go on asserting what an archived one
disproved. When a trim splits a correction from what it corrects, the live half
owes a line saying so — and this is that line.

**The rewiring is done by another mechanism.** That thread describes
`--toggle-collect` and what it would owe. Clay dropped the toggle, and
kanso#1241 reached the same end by reading `kanso::main` inclusive out of the
profile instead. Both debts were paid in that change: the guard exists (the gate
refuses a profile carrying no such frame, and prints the frames it does carry),
and the welfare baseline moved 57,029,831 -> 56,563,967 in the same commit, so
the instrument change could not be banked.

**The two-readings thread came out BOTH ways, which its dichotomy did not
allow.** It said other chips holding their own values would mean the key
separates nothing, and a second value on a recorded chip would mean neither
suspect was the term. Since then: Zen 4 and Zen 3 read 41,379,840 on sha
4dc725bdb40d; Intel `family0x1a-model0x2` read 41,379,840 this afternoon;
`family0x6-model0x6a` read 41,379,840 on this branch, whose diff is `design/`
alone; and `family0x6-model0xcf` produced two values 508 apart on one binary.
Four keys agree wherever they have shared a binary AND one key disagrees with
itself. Both branches fired, so the disjunction was wrong rather than either
answer being.
What survives is the narrower question the entry above already carries: what
moves the heap base when everything else agrees.

**What stays open is the corpus.** The five effect builtins are in no benchmark,
so a socket read going from a 260 MB peak to 2 MB scores zero. readbench does
not close it — that benchmark splits a string 200 times, which is the read
beat's repair and not the builtins. It is open on purpose, with its reason
written where it was filed.

## 2026-09-04 — where a program claims its stack

Searched first: the archive's 2026-08-31 entry on `k_b_append_grow` is the
nearest thing, and it is about a runtime helper's callee-saved prologue rather
than about the emitted module. Nothing in the log, the archive or the compiler
page mentions where an `alloca` stands. This is new.

The emitter wrote each call's argument array in the block that filled it. A
record built inside an `if` arm put its array in that arm; a dispatch arm the
same. Sixty-eight of the decoder's seventy-three stack slots stood outside
their function's first block, and LLVM reads a slot placed there as a dynamic
stack object whatever its size: the function keeps a frame pointer it would
otherwise omit, restores `rsp` through it on every return path, and claims the
slot again on each pass instead of once.

Callgrind, on jsonbench before the change: `push`/`pop`/`leave` are 252,410,292
of 2,098,864,058 instructions, 12.03% of the decode. The frame pointer's own
share — `push %rbp`, `mov %rsp,%rbp`, `pop %rbp` and the `lea -0xN(%rbp),%rsp`
that restores the stack — is 51,317,285 of those. The rest is callee-saved
registers, which a fixed frame does not remove.

The fix holds every `alloca` back at a single choke point in `FnEmit` and
writes them at the head of the entry block, so a site nobody has written yet
gets the placement too. Hoisting them by hand in the `.ll` measured the same
number as the compiler change does, which is how the size of the move was known
before any Rust was written.

```
    jsonbench     2,098,864,058 -> 1,914,624,003   -8.778%   (container)
    encodebench   5,847,000,948 -> 5,808,061,875   -0.666%
    oneshot          31,431,613 ->    30,109,010   -4.208%
    basket           40,300,171 ->    39,933,683   -0.909%
    widebench        59,384,053 ->    58,446,988   -1.578%
    deepbench       675,925,724 ->   678,046,033   +0.314%
    escapebench     130,170,750 ->   130,170,344   -0.000%
    pendbench       715,732,721 ->   702,115,194   -1.903%
    indexbench        5,242,362 ->     5,241,703   -0.013%
    scanbench     1,423,437,575 -> 1,425,333,592   +0.133%
    digestbench      81,256,592 ->    80,996,019   -0.321%
    readbench     2,000,657,821 -> 2,000,657,400   -0.000%
```

These are the container's numbers and its glibc is not the runner's, so
`bench/instructions_golden.txt` is regenerated from CI's own sitting rather
than from here. CI counted, on the same commit:

```
    jsonbench     2,098,864,471 -> 1,914,624,416   -8.778%   (runner)
    encodebench   5,847,000,948 -> 5,808,062,274   -0.666%
    oneshot          31,431,613 ->    30,109,409   -4.207%
    basket           40,300,171 ->    39,934,096   -0.908%
    widebench        59,384,053 ->    58,447,401   -1.578%
    deepbench       675,925,724 ->   678,049,713   +0.314%
    escapebench     130,170,750 ->   130,170,743   -0.000%
    pendbench       715,732,721 ->   702,115,580   -1.903%
    indexbench        5,242,362 ->     5,242,116   -0.005%
    scanbench     1,423,437,575 -> 1,425,334,005   +0.133%
    digestbench      81,256,592 ->    80,996,418   -0.320%
    readbench     2,000,657,821 -> 2,000,657,813   -0.000%
```

**work_deepbench rises to 678,049,713 and work_scanbench to 1,425,334,005.**
Those are the two the container also had rising, to the same third decimal
place, and they are the two whose `.text` grows most — scanbench by 3,424
bytes and deepbench by 128. Neither is a change to what those programs do:
every allocation counter in both is byte-identical, the emitted line count is
byte-identical, and the source they run is untouched. What moved is the layout
LLVM had to schedule against. They are bought by the ten that fall, which the
welfare number prices at 73.53 -> 73.73.

**compile_instructions 41,379,840 -> 41,377,711**, a fall of 2,129: the emitter
holds a `Vec` of slot lines per function and splices it, and that is cheaper
than what it replaced. The per-chip table is re-sat down to the one row CI
measured, Zen 3. The other three keys were all reading 41,379,840, and they
measure a compiler that no longer exists — removed rather than carried with the
delta applied, the same call the Zen 4 row got on 2026-09-03. Ten rows fall and two rise. The two that rise are the two whose
`.text` grows most, which is the shape to expect: a fixed frame gives LLVM a
different layout to schedule against and it inlines differently in both
directions.

Not one allocation counter moves. All ten counter gates are byte-identical, and
so is `bench/emitted_golden.txt` — the same lines are written either way and
only their order changes. That is the reason this needed a spec of its own:
`tests/a_stack_slot_stands_in_the_entry_block.rs` builds a record in each arm
of an `if` and reads the emitted `.ll`. Watched red before it was watched green,
reporting two of three slots adrift in `d_slot/pick_1`.

**text 1,070,072 -> 1,074,552.** Six rows fall, six rise, none by more than
2.4%, and the largest single move is scanbench 145,106 -> 148,530. jsonbench
spends 672 bytes here for 184 million instructions.

**welfare 73.53 -> 73.73**, floor set in the same commit.

**A ratchet row went blind on this branch, and the row was right.**
`a_re_basing_that_pays_for_a_regression` writes a re-basing beside a plain
worsening and requires the trend gate to refuse under the pure-regression rule.
Here it stayed green: the branch's own eleven falling work rows sat on the other
side of the mutation's rise, so the gate read a trade. The rule is about the
whole branch and the mutation was only about its own three files, which is a
hole any improving branch would have opened. It now resets `bench` and
`tests/golden/mem` to the base before it edits, so the three values it writes
are the only moves the gate can see. Watched red afterwards, and for the row's
own reason: `RE-BASED ... compile_instructions` followed by the pure-regression
refusal.

**OPEN — what the callee-saved registers cost.** Removing the frame pointer
leaves 201 million instructions of `push`/`pop` on jsonbench, 9.6% of the
decode. The decoder is a chain of mutually tail-calling functions, and a tail
call pops the whole frame before its `jmp` and the target pushes it again. That
is a different question from this one and nothing here answers it.

## 2026-09-05 — the ascii test comes out from behind the wide pass

Searched first: yesterday's entry on the entry-block stack slots is the nearest
neighbour and is about placement rather than about this. The archive's
2026-08-31 entry on `k_b_append_grow` is the same shape one layer down — a
cheap path trapped behind an expensive function's prologue — and its conclusion
was taken back by the welfare number. This one is not.

`k_utf8_bad` counted the bytes, ran the ascii word-test, and then ran the wide
SIMD pass, all in one function. LLVM will not inline that function anywhere:
the wide pass is seven constant loads and two zeroed accumulators before its
first block, and a table of three sixteen-byte lookups after it. So every token
a json document holds paid a call to reach a two-load answer — 83,092,800
instructions for 1,571,250 answers on jsonbench, 53 apiece for a mean run of
seven ascii bytes.

The front door is `always_inline` now and holds the counter and the test; the
pass is `k_utf8_bad_wide` and is reached only by a run carrying a byte with the
high bit set. Caller and callee are both in `runtime.c`, which is why a C
attribute reaches this one at all — see the rider below for where it does not.

```
    jsonbench     1,914,624,003 -> 1,898,797,203   -0.827%   (container)
    oneshot          30,109,010 ->    30,003,499   -0.350%
    widebench        58,446,988 ->    58,318,988   -0.219%
    basket           39,933,683 ->    39,915,674   -0.045%
    encodebench   5,808,061,875 -> 5,807,958,536   -0.002%
```

Seven rows are byte-identical and none rises. Every allocation counter is
unchanged and so is the emitted line count: this is entirely inside the runtime.

**text 1,074,552 -> 1,081,288.** Five rows rise 1,168 to 1,392 bytes and seven
hold exactly still: the ascii test is written at each of the validator's four
call sites now instead of once inside the pass, and the grammar walk is a real
function on every target rather than dead code behind an `#else`. The seven
that hold are the programs that never reach utf-8 validation. jsonbench spends
1,392 bytes for 36.9 million instructions.

**The harness caught the rename, which is what it is for.**
`scripts/utf8_differential` extracts the validator's text out of `runtime.c`
rather than carrying a copy, and the split moved the signature it looks for: it
answered `missing index 2` from `body_of` and stopped. It reads three pieces
now — `k_all_ascii` verbatim, the wide pass as `harness_utf8_wide`, and the
front door as `harness_utf8_ok` with its call renamed — so what is checked is
the composition. 45,189,025 validator checks and 8,346,016 counter checks
against the independently written reference, zero mismatches. Watched red
first: dropping the door's call to the pass gives 30,746,942 mismatches.

**RIDER on yesterday's OPEN thread — why the callee-saved 9.6% sits at entry.**
The entry-block entry left 201 million instructions of `push`/`pop` unexplained.
LLVM's shrink-wrapping cannot move them: it needs one save point dominating
every use and one restore point post-dominating them, and these functions have
three to eight returns spread across blocks. In jsonbench's emitted module
`parse_value_2` has 3 returns and 1 `musttail`, `obj_key_start_4` 7 and 1,
`string_at_4` 6 and 2, `array_step_3` 4 and 1, `scan_at_5` 8 and 7; the module
holds 125 `musttail` sites across 15 functions with more than one. A `musttail`
exit needs the epilogue before its `jmp`, so each of those blocks is a restore
point and the prologue has to dominate all of them. The general remedy is
`preserve_none`, the calling convention LLVM added for this shape, and it
landed in LLVM 19. The pinned toolchain is Ubuntu clang 18.1.3, which rejects
`preserve_nonecc` at the parser while accepting `tailcc` on the same file. The
fix is named and gated on a toolchain bump. What stays inside reach is reducing
what is live across the non-tail calls in the hot dispatchers, which is a
per-function matter.

**And then the third arm, in the same change.** With the ascii runs turned away
the wide pass was 60,769,050 instructions over 203,700 calls, 3.29% of the
decode: 13% of the document's strings are not ascii, and each cost 298 to
validate about seven bytes. The setup is the whole of it, and its own comment
already said so. The portable arm of that pass walked the grammar a byte at a
time and nothing but a host without simd ever reached it, so it became a
function and the door chooses on the length.

```
    jsonbench     1,898,797,203 -> 1,877,751,303   -1.108%   (container)
    oneshot          30,003,499 ->    29,863,200   -0.468%
    widebench        58,318,988 ->    58,270,995   -0.082%
```

Every arm returns the same sentence, so which one answered is not observable,
and the harness reads a third piece now — with the threshold taken out of the
line that defines it rather than written down twice. Watched red: dropping the
surrogate bound on `0xED` gives 2,048 mismatches, exactly `ED A0-BF` by
`80-BF`, so the fuzzer reaches the arm that moved.

**DECLINED on the way: the lead byte's table.** Replacing the eight-comparison
chain that decides a lead byte's width with a 256-entry table costs 1,428,150
instructions instead of saving any — 1,877,751,303 against 1,879,179,453 on
jsonbench. The chain is predictable, and the ascii bytes that dominate a
mostly-ascii run are tested before it and never reach either. Recorded so it is
not rediscovered.

**CI's sitting, which is what the goldens carry.** The rows above are the
container's; the runner counts a few hundred higher on every one of them and
its numbers are the ones pinned. Five work counters move and none rises:
`work_jsonbench` lands on 1,877,751,716 (−1.926%), `work_oneshot` on 29,863,599
(−0.816%), `work_widebench` on 58,271,408 (−0.301%), `work_basket` on
39,914,087 (−0.050%) and `work_encodebench` on 5,807,819,641 (−0.004%). The
other seven are byte-identical.

**And `compile_instructions` rose 682, to 41,378,393, which is layout.** The
front end did not change on this branch — the diff is `src/runtime.c`, the
differential harness, the goldens and this file. `src/runtime.c` is
`include_str!`'d into the compiler, so editing the runtime moves the compiler's
own bytes: 1,435 more of them here. `kanso check lib/json` emits nothing, so
the validator that changed cannot run during the measurement. This is the
seventh time this vein has moved for layout and the second time a runtime edit
has done it — kanso#1226 moved it −5,621 the same way, held on one chip with
both binary shas printed. `compile_allocs`, `compile_peak_bytes`, rounds and
visits are byte-identical.

The per-chip table was re-sat down to one row for the same reason: both AMD
rows measured a binary that no longer exists, and the first CI run handed out
Emerald Rapids (`family0x6-model0xcf`), new to the table under any binary. One
reading on a new chip and a new binary cannot split the 682 between them, and
the row said so.

A second run settled it. Zen 3 (`family0x19-model0x1`) refused on the same
branch and printed 41,378,393 — the same to the instruction as Emerald Rapids
on this binary, and Zen 3 had read 41,377,711 on the one before it. One chip,
two binaries: the whole 682 is the binary and none of it is the chip. Zen 3's
row goes back last, because the first row is what welfare and the golden read
and that authority stays where it was.
## 2026-09-05 (second) — two doors take scalars instead of boxes

Searched first: this morning's entry on the utf-8 door is the nearest
neighbour and is about a function's prologue rather than about its arguments.
The archive's 2026-08-31 entry on `k_b_append_grow` is the other half of the
same subject — what an `always_inline` in `runtime.c` can and cannot reach —
and it is cited below rather than repeated.

**An `always_inline` in `runtime.c` cannot reach a generated caller.** The
emitted module declares the source-level type, `%KValue @k_b_find2(%KValue,
%KValue, %KValue, %KValue)`; clang compiles the same C function to the
ABI-lowered `{i64,i64} @k_b_find2(i64 x6, ptr byval(%struct.KValue))`. LTO
sees two function types and keeps the call. Marking `k_b_find2` moved
jsonbench by zero instructions and left the symbol in the binary. The machine
ABI does agree — this is an inlining barrier and not a miscompile — and the
lever is the prelude shim, the mechanism already carrying `k_force_fast`,
`k_b_append_byte`, `k_b_length_fast` and ten others.

**find2.** Four KValues do not fit the six integer registers the SysV ABI
has: three fill them and the fourth arrives as a pointer to the caller's
stack, so the caller stores it and the callee loads and unpacks it before it
can splat the byte. That was fourteen of the function's fifty-four
instructions on jsonbench, where the scan itself was ten, and the entry
guards were seventeen more. `k_b_find2_raw` takes a pointer, a length and
three integers; `k_b_find2_fast` tests the four tags, reads the header, and
hands it those five.

```
    jsonbench     1,877,751,303 -> 1,827,146,403   -2.695%   (container)
    oneshot          29,863,200 ->    29,525,834   -1.130%
    encodebench   5,807,819,242 -> 5,807,486,906   -0.006%
```

Nine rows are byte-identical and none rises.

**slice.** The bytes arm builds a view — a header, a pointer and a length —
and reaching it cost three failure guards, two tag tests and the unboxing of
three KValues. `k_b_slice_raw` takes a pointer, a length and two integers and
holds the four compares and the add; `k_b_slice_fast` tests the three tags.

```
    jsonbench     1,827,146,403 -> 1,806,581,553   -1.126%   (container)
    widebench        58,270,995 ->    57,886,995   -0.659%
    oneshot          29,525,834 ->    29,388,735   -0.464%
    encodebench   5,807,486,906 -> 5,807,186,902   -0.005%
```

**And four rows rise, which is the shim's own cost showing.** They are the
programs that slice something other than bytes: every such call now pays
three tag tests before falling through to the same C function it always
reached. scanbench 1,425,333,592 -> 1,427,829,097, a rise of 2,495,505 and
0.175%; digestbench +256; basket +13; indexbench +2. Three rows hold exactly
still.

The objective takes the trade. Read on the container's own sitting for both
sides, so the comparison is against itself: 73.82 with find2 alone, 73.85
with slice beside it. The scan's 0.175% is real and it is smaller than what
the decode gains.

**What both cost in code.** The prelude grows one define, two calls, one
branch and twenty-five to thirty-one lines in every program whether it calls
the builtin or not, which is what the emitted goldens move by; the linker
drops the shim wherever nothing calls it, which is why only the programs that
use it grow `.text`.

**Both ship with a fixture watched red.** `find2_at_its_edges` covers a hit
at the first byte and the last, `from` below one and past the end, the
earlier of two bytes winning, no match, and on a twenty-four byte string a
hit inside the sixteen-byte vector step and one only the scalar tail reaches.
Changing the raw door's clamp from `from < 1 ? 0 : from - 1` to
`from - 1 < 0 ? 0 : from` moves three of its lines on native with the
interpreter untouched. `slice_at_its_edges` covers each end of the range on
bytes and a list and a string falling through; swapping `from` and `to` in
the shim's call empties the forward slices and fills the backwards one.

**And the one that did not need a shim.** `k_str_n` copies n bytes into a
fresh `KStr` and terminates it. It is `static` in `runtime.c` and so is every
caller, so an `always_inline` here does reach them — the barrier the two shims
went around is a barrier between the emitted module and the runtime, not
inside the runtime.

```
    indexbench        5,241,705 ->    4,791,711   -8.585%   (container)
    scanbench     1,427,829,097 -> 1,406,297,911   -1.508%
    jsonbench     1,806,581,553 -> 1,791,154,032   -0.854%
    oneshot          29,388,735 ->   29,275,330   -0.386%
    pendbench       702,115,194 ->  700,501,409   -0.230%
```

basket, widebench, encodebench and digestbench fall too. Two rows rise by
twelve and fourteen instructions. One rises for real: readbench
2,000,657,400 -> 2,038,393,555, a rise of 1.886%. That benchmark reads a file
and slices it, so it holds few short strings and many long ones, and an
inlined copy loop loses to a called one when the copy dominates.

**Its price is machine code, and it is the largest of the three by far.** All
twelve `.text` rows rise, 400 to 5,024 bytes and 34,320 in total, where the
two shims grew four rows and eight. A prelude twin is one function the linker
drops where nothing calls it; this is a copy loop written out at every site
that makes a string. jsonbench spends 4,320 bytes for 15,427,521 instructions
and scanbench 5,024 for 21,531,186. indexbench spends 1,952 for 449,994,
the worst rate of the twelve, and still falls 8.585%.

**The objective cannot see that price.** welfare weighs allocations, arena
blocks, instructions, fixpoint rounds, expression visits and emitted lines,
and has no term for the size of the machine code — so 73.85 -> 73.90 is the
sum over everything it does weigh, and the 34,320 bytes are priced only in
`bench/text_golden.txt`. Whether the index should carry a `.text` term is a
question about the weights and it is left as one here, with the measurement
that raises it. Nothing about these three changes turns on the answer: each
of them rises on the current model and each of them rises on any model that
weighs `.text` at less than what the instruction falls are worth.

**Every code counter the two twins moved, with the value it landed on.** Two
prelude twins are two `define`s, four `call`s, two `branch`es and about fifty
lines written into every module the emitter produces, whether the program
calls the builtin or not; the linker drops what nothing calls, so the machine
code grows only where it is used. `bench/compile_golden.txt` reads `defines`
169, `calls` 217, `branches` 262, `lines` 4,147, and its `rounds` and `visits`
do not move at all — the emitter writes more and deciding what to write costs
the same. `bench/compile_golden_modules.txt` reads `module_defines` 93,
`module_calls` 777, `module_branches` 413, `module_lines` 4,997.
`bench/emitted_golden.txt` reads `emitted_defines` 176, `emitted_calls` 1,855,
`emitted_branches` 1,198, `emitted_lines` 12,273, and
`bench/emitted_golden_others.txt` reads `emitted_other_defines` 1,565,
`emitted_other_calls` 14,869, `emitted_other_branches` 8,856 and
`emitted_other_lines` 90,688. `text` lands on 1,117,272, of which 34,320 is
`k_str_n` and the rest the two twins.

**CI's sitting, which is what the goldens carry.** The rows above are the
container's. Nine of the runner's twelve fall: `work_jsonbench` lands on
1,791,154,445 (−4.612% against main), `work_indexbench` on 4,792,124
(−8.584%), `work_scanbench` on 1,406,298,324 (−1.335%), `work_oneshot` on
29,275,729 (−1.968%), `work_widebench` on 57,839,394 (−0.741%),
`work_pendbench` on 700,501,795 (−0.230%), `work_basket` on 39,875,796,
`work_encodebench` on 5,802,859,663 and `work_digestbench` on 80,995,637.
scanbench's is the number to read for the slice twin's cost: that twin took
2,495,505 off it on its own and `k_str_n` gave back more, so the vein records
a fall where the per-change readings record a rise and then a larger fall.

**Three rise, and all three are named here because the trend gate refuses a
move with no sentence.** `work_deepbench` lands on 678,049,725 and
`work_escapebench` on 130,170,757 — twelve and fourteen instructions, the
twins' tag tests on the few calls those two programs make. `work_readbench`
lands on 2,038,393,968, a rise of 37,736,155 and 1.886%, and it is `k_str_n`:
that benchmark reads a file and holds few short strings and many long ones,
where an inlined copy loop loses to a called one. `compile_instructions` lands
on 41,378,619, a rise of 226 — held on ONE chip across both binaries, Emerald
Rapids reading 41,378,393 before the twins and 41,378,619 after, so none of it
is the silicon. The prelude gained two twins and `runtime.c` two raw doors;
`kanso check lib/json` emits nothing, so the front end does the same work.

**welfare 73.77 -> 73.90**, banked with `--set` in the same commit.

## 2026-09-05 (third) — the argument that would not fit

`utf8` applied to `slice` has been lowered as a single call since the fusion
landed, and that call carries three `KValue`s and a pointer to the wrapper's
origin: seven register-sized arguments where the SysV ABI has six. The seventh
spills to the caller's stack and the callee reloads it before it can put the
origin in an err. `k_b_utf8_slice_raw` takes a pointer, a length, two integers
and the origin — five — and the `k_b_utf8_slice_fast` twin in the prelude
tests the three tags and calls it. This is the third door to take the shape
kanso#1247's §48 describes, and the first where the spill rather than the
unpack was the thing to remove.

**Held on one host, in one directory, against a clean build of the commit it
sits on.** jsonbench 1,791,154,032 -> 1,783,182,882, a fall of 7,971,150 and
0.4450%. oneshot 29,275,330 -> 29,222,188, 53,142 and 0.1815%. encodebench
5,802,859,264 -> 5,802,805,722, 53,542 and 0.0009%. widebench's binary changed
and its count did not move at all. The other eight benchmark binaries came out
BYTE-IDENTICAL to the ones before the change, so those eight rows could not
have moved and were not measured twice. Nothing rises.

**`.text` grows 32 bytes, 16 each on jsonbench and oneshot**, and the other
ten rows are unchanged because their binaries are. The twin is `internal` and
the linker drops it where nothing calls it, so the whole cost lands on the two
callers. Beside `k_str_n`'s 34,320 bytes across all twelve rows in kanso#1247,
this is the same mechanism reading the other way, and it is one more case for
the `.text`-in-welfare question sitting in design/pending-gavels.md rather than
an answer to it.

**The fixture is tests/golden/micro/utf8_of_a_byte_slice_at_its_edges.kso, and
the door had none.** tests/golden/runtime/utf8_of_a_slice_names_the_wrapper.kso
reaches the same lowering with a LIST, which falls through to the general slice
and the general `utf8`; the bytes arm — the arm the fusion exists for, and the
one the raw door now holds — was pinned by nothing. The fixture walks every
range the clamp refuses and both sides of the validity test, including a
two-byte sequence taken whole, the same sequence cut in half, and its
continuation byte alone. Watched red twice: with `k_utf8_bad` dropped from the
raw door the two invalid cases print raw bytes instead of refusing, and with
`to > blen` dropped from the clamp the output goes to binary.

**A measurement mistake, recorded because the gate now carries the fix.** The
first A/B built the baseline in a scratch worktree and the twin in the repo
and measured each where it was built. escapebench and digestbench each read
+14, and both binaries were byte-identical to the baseline's. The same
escapebench counts 130,170,344 under one path and 130,170,358 under the other:
the kernel puts the exec path on the new process's stack beside the
environment, and libc walks that before main. `scripts/gates/instructions.sh`
already empties the environment for this reason and said nothing about the
path; it does now. CI always runs from the repo root, so no golden was ever
wrong — only a local comparison across two directories, which is how this vein
gets measured while a change is being decided.

**Every code counter the twin moved, with the value it landed on.** One prelude
twin is one `define`, two `call`s, one `branch` and twenty-six lines written
into every module the emitter produces, whether the program reaches `utf8` over
a `slice` or not. `bench/emitted_golden.txt` reads `emitted_defines` 177,
`emitted_calls` 1,857, `emitted_branches` 1,199 and `emitted_lines` 12,298;
`bench/emitted_golden_others.txt` reads `emitted_other_defines` 1,576,
`emitted_other_calls` 14,891, `emitted_other_branches` 8,867 and
`emitted_other_lines` 90,971. `bench/compile_golden.txt` reads `defines` 174,
`calls` 227, `branches` 267 and `lines` 4,277, with `rounds` and `visits`
unmoved at 12 and 115 — the emitter writes more and deciding what to write
costs the same. `bench/compile_golden_modules.txt` reads `module_defines` 94,
`module_calls` 779, `module_branches` 414 and `module_lines` 5,023, its
`module_rounds` and `module_visits` also unmoved. `text` lands on 1,117,304.

**CI's sitting, which is what the goldens carry, and it agrees with the
container to the instruction.** `work_jsonbench` lands on 1,783,183,295,
`work_oneshot` on 29,222,587 and `work_encodebench` on 5,802,806,121 — falls of
7,971,150, 53,142 and 53,542, each of them the SAME NUMBER the container
measured on its own pair of builds. Nine rows hold exactly still. That
agreement is worth naming: the two hosts count different totals for every row
and have never before matched on a delta, let alone on three.

**`compile_instructions` lands on 41,377,663, a FALL of 956**, and this one
needs no second chip either: family0x19-model0x11 counted 41,378,619 on the
binary before the twin and 41,377,663 on the one with it, so none of the 956 is
the silicon. `kanso check lib/json` emits nothing, so the door that changed
cannot run during the measurement; the compiler's own bytes moved, which is
layout for the eighth time on this vein and the first time it has gone down for
a runtime edit. The Emerald Rapids and Zen 3 rows measured the binary before
this change and are removed rather than carried with the delta applied.

**welfare 73.90 -> 73.91**, banked with `--set` in the same commit.

**And the callee-saved question got its missing number, measured on this
binary.** The log has carried "9.6% at entry" since the LLVM 19 investigation,
with the per-function split recorded as unmeasured. It is 5.569% —
99,299,946 instructions of 1,783,182,882 — counted by taking each symbol's
pushes of rbx, rbp and r12-r15 from `objdump`, multiplying by callgrind's call
count for that symbol, and doubling, because every call runs all the pushes and
the taken exit runs the matching pops (a `musttail` jump restores them first,
so it counts too). Where it sits: `d_jsonbench/parse_value_2` 32,569,200
(1.826%, 2,714,100 calls over six registers), `obj_key_start_4` 15,049,800,
`k_b_utf8_slice_raw` 10,442,400, `k_b_slice_raw` 5,391,000, `number_done_4`
5,060,400, `k_buf` 5,026,200, `array_step_3` 4,953,600, `k_b_to_int` 3,769,200,
`obj_items_3` 3,360,000, `k_b_append_grow` and `str_char_4` 3,191,400 each,
`k_rec` 3,076,500, `k_str_lit` 2,127,616, `k_b_bytes` 1,064,400,
`k_map_view_insert` 1,026,000. The generated decoder carries 3.6 points of the
5.569 and the runtime's C doors carry 1.969, so a `preserve_none` that reached
only the emitted functions would still be the larger half. The toolchain is
still a version short of it and shrink-wrapping is still ruled out by the exit
counts, so nothing here is actionable yet; what changes is that the size of the
prize is a measurement rather than an estimate, and it is smaller than the
estimate was.

## 2026-09-05 (fourth) — the largest spill, and the census that found it

Three doors had been twinned by reading the profile and picking what looked
expensive. The census asks the question directly instead: which of the doors
the emitter declares take more than the six integer argument registers the
SysV ABI has? A `KValue` is two of them, so the answer is countable from
`src/codegen.rs` alone, and it is five — `k_call4` and `k_b_find2_below` at
ten, `k_call3` and `k_b_find2` at eight, `k_b_utf8_slice` at seven.

`k_b_find2` and `k_b_utf8_slice` were already done. `k_call3` and `k_call4`
appear in no benchmark profile at all. That leaves `k_b_find2_below`, which
carries 481,347,200 instructions of self cost in encodebench — 8.3% of it —
and spills FOUR arguments, more than any other door in the tree.

**The measurement, held on one host in one directory against a clean build of
the commit it sits on.** encodebench 5,802,805,722 -> 5,563,212,122, a fall of
239,593,600 and 4.1289%. oneshot 29,222,188 -> 28,623,204, 598,984 and 2.0498%.
widebench's binary changed and its count did not move at all; jsonbench's
binary is byte-identical, and so are the other eight, so those nine rows could
not have moved. Nothing rises. `.text` grows 224 bytes on each of the three
programs whose binaries changed and nothing on the other nine.

The twin takes about half of what the door spent. Four spilled arguments are
four stores in the caller and four loads and unpacks in the callee, on every
call, in front of a scan that is a compare and a branch per byte.

**And the census is worth keeping as a method.** `k_b_slice` fits in six
registers and its twin still won, on the guards rather than the spill, so the
overflow test is necessary for THIS mechanism and not sufficient for finding
every candidate. What the count does give is a closed list: after this one,
the only doors left that overflow are two the corpus never calls, so this
family is finished unless a new door is declared or the two call doors start
appearing in a benchmark.

**tests/golden/micro/find2_below_at_its_edges.kso**, twelve cases. The one
line in `bytes_are_their_own_value` took a hit on the first byte of a
four-byte string and nothing else was pinned: not the clamp, not the limit
guard that chooses between the sixteen-byte vector step and the scalar loop,
not the tail the vector step cannot reach. The fixture covers each of the
three ways a byte can match, both ends of the range, a hit inside the vector
step and one only the tail reaches, and both ways the limit guard turns the
vector path off — a zero limit, where the below-test can never fire, and a
limit past any byte, where it fires on the first. Native and the interpreter
agree byte for byte. Watched red: dropping the below-limit test from the raw
door's scalar tail moves exactly the two cases that turn on it.

**A counter nearly doubled in silence.** The body extracted into the raw door
already opened with `k_stat_find2_calls++`, and the prologue written in front
of it added a second. `find2_calls` is pinned in the cost goldens, so the
first CI run would have refused a counter that moved for no reason anybody
could name. It was caught by reading the extracted function before building
it, which is the only reason it is a footnote rather than an entry of its own.

**Every code counter the twin moved, with the value it landed on.** A fourth
prelude twin is one `define`, two `call`s, one `branch` and thirty-five lines
written into every module the emitter produces, whether the program reaches
`find2_below` or not. `bench/emitted_golden.txt` reads `emitted_defines` 178,
`emitted_calls` 1,859, `emitted_branches` 1,200 and `emitted_lines` 12,332;
`bench/emitted_golden_others.txt` reads `emitted_other_defines` 1,587,
`emitted_other_calls` 14,913, `emitted_other_branches` 8,878 and
`emitted_other_lines` 91,353. `bench/compile_golden.txt` reads `defines` 179,
`calls` 237, `branches` 272 and `lines` 4,452, with `rounds` and `visits`
unmoved at 12 and 115. `bench/compile_golden_modules.txt` reads
`module_defines` 95, `module_calls` 781, `module_branches` 415 and
`module_lines` 5,058, its `module_rounds` and `module_visits` also unmoved.
`text` lands on 1,117,976 — 672 bytes over three programs, and the other nine
rows do not move because their binaries are byte-identical to the ones before
this change.

**CI's rows, and the two hosts agreed to the instruction again.**
`bench/instructions_golden.txt` reads `work_encodebench` 5,563,212,521 and
`work_oneshot` 28,623,603 — falls of 239,593,600 and 598,984, which are the
container's two deltas exactly, on a different host with different absolute
values. That is the second consecutive change on which this vein has recorded
exact agreement between the runner and the container; the utf8-over-slice twin
was the first. The other ten rows hold to the digit.

**The compile row moved and the table is one row again.** `src/runtime.c` is
`include_str!`'d into the compiler, so a door added to it moves the compiler's
own bytes and the layout under them, and every chip's row goes stale at once.
CI drew `family0x19-model0x1`, found no row for it — the previous entry's block
had removed the stale one — and refused rather than comparing against somebody
else's number. `compile_instructions` lands on 41,377,855 where the golden
carried 41,377,663, and the Zen 4 row it replaces measured the binary before
this one and is removed rather than carried forward with the delta applied.
`kanso check lib/json` emits nothing, so the scan door that changed cannot run
during the measurement; what moved is layout, for the ninth time on this vein.
`compile_allocs` 25,490, `compile_peak_bytes` 715,275, `rounds` and `visits`
are byte-identical.

**welfare 73.99**, from 73.91, and `--set` in this same commit.

## 2026-09-05 (fifth) — a lambda that captures nothing is one value

**Searched first**, as the filing gate requires: `grep -n "closure" design/compiler-log.md
design/log/compiler-log-archive.md` returns the closure-call dispatch work
(kanso#1119, the builtin's count at the front door) and the arity divergence
(kanso#1229), neither of which touches how a closure is BUILT. `k_closure` has
allocated a header and an environment per evaluation since the emitter first
wrote one, and nothing in the log or the archive proposes otherwise.

**The find came out of the profile the find2_below twin left.** After that
change encodebench sits at 5,563,212,122 and `k_closure` is 709,200 calls and
48,225,600 instructions of it, 0.87%, with 39,006,110 of that its own body.
Every call is TWO arena allocations — a `KClosure` and an environment, sized
`ncaps ? ncaps : 1` so the empty one is allocated too — and 1,418,400 of
encodebench's 16,249,027 allocations are that pair. Two of the three
`k_closure` sites the emitter writes for encodebench pass `ncaps` 0.
`escape_able_2` is the hot one: it allocas a one-slot array, stores nothing
into it, and hands `list/fold` a closure that never differs.

**A lambda that captures nothing is the same value every evaluation, which is
what `str_const` already says about a string literal** — "a literal is the
same value every evaluation, so it builds once into a permanent slot instead
of allocating per visit". `k_closure_lit` is `k_str_lit` with a closure in it:
a per-site `KValue` cell, `k_alloc_perm` on the first visit, the cell returned
on every visit after. Codegen emits it whenever the capture list comes out
empty, and the alloca goes with the allocations.

**The measurement, one host, one directory, the base binaries swapped in place
so the exec path cannot move the count.**

    encodebench   5,563,212,122 -> 5,527,056,277   -36,155,845   -0.6499%
    digestbench      80,995,238 ->    80,558,930      -436,308   -0.5387%
    oneshot          28,623,204 ->    28,533,106       -90,098   -0.3148%
    scanbench     1,406,297,911 -> 1,406,247,294       -50,617   -0.0036%
    deepbench       678,046,045 ->   716,507,476   +38,461,431   +5.6724%
    basket, widebench, pendbench, readbench and jsonbench rise by 3,200 or
    less each; escapebench and indexbench do not move at all.

**deepbench's rise is code layout and callgrind says so with call counts.**
The carry walk visits the SAME nodes: `k_copy_size'2` is entered 859,622 times
before and 857,697 after, `k_copy_size` 1,534,544 and 1,535,314, `k_ptrmap_at`
1,571,990 and 1,571,239 — every one within 0.2%. What moved is the cost of a
visit: 273,639,699 instructions over 2,394,166 entries is 114.3 apiece against
301,674,351 over 2,393,011, which is 126.1. Twelve instructions more per call,
in a function this change does not touch, on a program that enters it two and
a half million times. Removing the allocas from the emitted module moved
clang's inlining budget inside `k_copy_size` under LTO.

**Two attempts to steer it, both declined by measurement.** The first marked
every literal closure with a shared sentinel environment and short-circuited
it to "survives" in `k_slots_survive`, `k_interior_survives`, `k_copy_size`
and `k_deep_copy`, on the theory that permanent storage answering no to
`k_survives` was forcing copies. deepbench went to +6.0799% and took basket,
widebench and pendbench up with it — more code in the same hot walk, and the
theory was wrong anyway: deepbench's own `k_closure` only falls 26,840,055 to
23,980,088, so most of its closures capture something and were never
literals. The second marked `k_closure_lit` `noinline, cold`: deepbench stayed
at +5.63% and encodebench's gain shrank from 0.6499% to 0.2805%. A probe that
added the same function under a name codegen never emits left every benchmark
binary BYTE-IDENTICAL, which is the useful negative — the layout move needs
the emitted module to change, not just runtime.c to grow.

**The objective takes the trade.** welfare reads 73.99170779271341 against
74.00198125390742 on the container's own sitting for both sides, a rise of
0.01027. It is a thin one, and the reason it is thin is that RUNTIME
ALLOCATION COUNTS ARE NOT IN THE OBJECTIVE — only peak bytes and arena blocks
are — so the 1,418,400 allocations encodebench stops making score exactly
zero. The instruction rows are the whole of what welfare sees here.

**tests/golden/micro/a_lambda_that_captures_nothing_is_one_value.kso**, ten
cases, because sharing closure identity is the new risk and nothing pinned it.
Two capture-free lambdas in one body each need their own slot; a lambda that
DOES capture must still get a fresh closure per call. Watched red twice.
Routing a one-capture lambda through the literal path moves exactly the three
capture lines — "a capture of five" 6 to 1, "a capture of nine" 10 to 1, "five
again, after nine" 6 to 1, because the capture is never stored — and leaves
the seven capture-free lines alone. Giving every lambda site ONE cell moves
"two lambdas in one body" 115 to 20, since `b` resolves to `a`, and turns the
fold's two-argument lambda into `error[runtime]: this function takes 1
argument(s), got 2`.

**Every counter that worsened, with the value it landed on.** The emitter
writes one global and one call where it wrote an alloca and a store, and the
globals are lines nothing removes: `emitted_lines` 12,344, `module_lines`
5,068, `emitted_other_lines` 91,445, `text` 1,119,560. `emitted_defines`,
`emitted_calls` and `emitted_branches` hold at 178, 1,859 and 1,200, and the
five programs in `bench/compile_golden.txt` hold entirely — none of them
writes a lambda. `.text` FALLS on the two programs with no capture-free
lambda to convert, escapebench 52,658 to 52,466 and indexbench 56,002 to
55,810, and rises on the other ten.

**The mem vein moves in both directions and the trade is the point.** Fifteen
files trade arena allocations for permanent ones, one pair per lambda site,
paid once for a process rather than once per visit, and each lands on a value
worth writing down: a_digest_holds_every_block_it_walked_perm_allocs 34,
a_repaired_node_below_the_mark_holds_tenure_perm_allocs 20,
fused_tally_perm_allocs 9, effect_push_shape_perm_allocs 6,
fold_push_shape_perm_allocs 6, fused_map_shape_perm_allocs 6,
early_exit_perm_allocs 5, fused_select_shape_perm_allocs 5,
skip_shape_perm_allocs 5, sort_shape_perm_allocs 5, take_shape_perm_allocs 5,
a_loop_invariant_capture_is_copied_every_rewind_perm_allocs 4,
tally_shape_perm_allocs 4, fused_reducer_perm_allocs 3 and
piped_reducer_perm_allocs 3. Against that, `allocs` falls in every one of
the fifteen — 1,980 to 1,850 in the digest, 5,137 to 5,133 in the repaired
node, 23 to 19 in early_exit — and `alloc_bytes` with it.

The work rows and welfare wait on CI's sitting, and the compile row goes stale
with them: both `src/codegen.rs` and `src/runtime.c` changed, so the compiler's
own bytes moved for the tenth time on that vein.

**CI's sitting, and the two hosts agreed on ten of twelve rows.** Four work
rows fall: work_encodebench 5,527,056,676 (−36,155,845, −0.6499%),
work_digestbench 80,559,329 (−436,308, −0.5387%), work_oneshot 28,533,505
(−90,098, −0.3148%) and work_scanbench 1,406,247,707 (−50,617). Every one of
those four deltas is the container's TO THE INSTRUCTION, as are
work_jsonbench's, work_basket's, work_readbench's, work_widebench's and both
unmoved rows — ten of twelve, the third consecutive change on which this vein
has recorded exact agreement between runner and container.

**The two the hosts disagree on are exactly the two whose rise is layout.**
work_deepbench 716,506,831 (+38,457,106, +5.6724%) against the container's
+38,461,431, and work_pendbench 700,529,602 (+27,807) against +27,780. A
layout effect is a property of the binary a host built, so a per-host
difference there is the shape to expect; a work difference would not have
been. The remaining rises are the same thing smaller: work_basket 39,878,996
(+3,200), work_readbench 2,038,397,546 (+3,578), work_jsonbench 1,783,183,673
(+378), work_widebench 57,839,754 (+360). work_escapebench 130,170,757 and
work_indexbench 4,792,124 do not move at all, because neither program has a
capture-free lambda to convert.

**Every counter vein moved the same way, and the trade is one shape.** Nine
cost goldens swap arena allocations for a permanent pair per lambda site.
`allocs` falls in all nine — decode 4,999,965, encode 14,830,625 (−1,418,402),
oneshot 75,822, basket 28,170, pend 4,007,400, scan 3,975,888, wide 144,027,
digest 213,707, read 615 — and `alloc_bytes` with each. Against that,
`perm_allocs` lands on decode_perm_allocs 6, encode_perm_allocs 14,
oneshot_perm_allocs 12, basket_perm_allocs 41, pend_perm_allocs 25,
scan_perm_allocs 51, wide_perm_allocs 8, digest_perm_allocs 36 and
read_perm_allocs 5. Three more pending-cell counters move with the shape of
what the evacuation now walks: pend_evac_bytes 501,056, pend_survive_slots
118,477 and pend_sh_buf 32,134,704.

**compile_instructions 41,379,503**, re-sat on family0x19-model0x11 after CI
refused an unrecorded chip. This move is not only layout: the emitter writes
one global and one call where it wrote an alloca and two stores, so
`kanso check lib/json` really does emit different bytes. compile_allocs holds
at 25,490 and compile_peak_bytes at 715,275.

**Two book panels went with the counters** and nothing but the full check
caught them: docs/book/samples/ch10/counters_counters.out and
docs/book/samples/ch12/fused_counters.out both print `allocs` and
`perm_allocs`, and both move 12 to 10 and 1 to 3. The ch10 counters sample is
named in CLAUDE.md's list of veins to regenerate together, and this session
regenerated the .mem files and the four code goldens and missed it.

**welfare 74.00**, from 73.99, and `--set` in this same commit.

**Three chips counted the same compile row and all three agreed exactly.** The push
that landed CI's sitting moved only goldens, the log and the pages, so the
compiler binary was byte-identical to the one Zen 4 had counted. CI then drew
family0x19-model0x1, found no row and refused, as an unrecorded chip must.
It read 41,379,503 — Zen 4's value to the instruction. The next round drew
family0x6-model0xcf, the Intel, and read 41,379,503 as well.

**That meets the condition this vein set for itself.** The block written when
the utf8-over-slice twin landed said that three keys agreeing once is one
binary, and that a later change showing the same three agree again is the
evidence for asking whether the table should collapse — an argument made about
the table, with the readings behind it, rather than a tidy-up. This is that
second binary. Emerald Rapids, Zen 3 and Zen 4 now agree to the instruction on
two of them, and the reason is legible: kanso#1241 cut this row down to the
compiler's own frame, and the glibc ifunc dispatch the key was invented to
separate lives in the loader and libc, outside that frame.

The argument is not made in this change, which is a codegen change carrying a
fixture and nine goldens; the second reading is recorded so it can be made
from two rather than from one. What the current design cost here is worth
recording beside it: this branch spent THREE CI rounds adding three rows that
all say the same number, because the pool hands out a chip at random and every
unrecorded one is a refusal. The rows are appended last, because the first is
the one the golden carries.

## 2026-09-05 (sixth) — ten cost goldens, and the file that said four

**Searched first**, as the filing gate requires. `grep -n "cost golden"
design/compiler-log.md design/log/compiler-log-archive.md CLAUDE.md` returns the
counter-regeneration rule in several places and no entry proposing a single
local sweep; `ls scripts/gates/*counters*.sh` returns ten files and nothing in
the tree runs them together.

### What the gap cost

kanso#1249's first push regenerated the mem vein, the four code goldens and the
wasm blob, and missed nine `bench/cost_golden*.txt` files and two book panels.
CI found them a round late. The cause was not carelessness about the list, it
was the list: CLAUDE.md's "Counters changed → regenerate every vein in the same
PR" said **all four cost goldens**, and there are ten. A session that read the
instruction and followed it exactly would still have missed six.

### The sweep

`scripts/gates/all_counters.sh` builds the benchmarks once, then walks a table
of ten `vein:program:golden` rows and diffs each measured file against its
golden with the comment headers stripped. It does not stop at the first
disagreement — the point of having one command is to learn about all ten in one
run — and it prints `counters moved: …` naming every vein that differs.
`--write` rewrites the data rows in place while preserving each golden's
comment header, which a `cp` of the measured file destroys;
`bench/cost_golden_wide.txt` and `bench/cost_golden_pend.txt` both carry one.

### The spec, watched red

`tests/every_counter_gate_is_in_the_sweep.rs`, three tests.

The first compares the sweep's table against `scripts/gates/*_counters.sh` on
disk, so a gate added without a row turns it red. Its first version demanded a
row for a vein called "all", because the glob matched `all_counters.sh` itself;
that is recorded in the test's doc comment as how it proved it could fail. The
second checks that every row names a golden and a program that exist.

The third extracts the word immediately before "cost goldens" in CLAUDE.md and
compares it to the number of `bench/cost_golden*.txt` files. With "four"
restored it reads *there are 10 cost goldens and CLAUDE.md counts them "four"*.
It was written wrong the first time: it asserted that the claim contained "10",
which is true with "four" written, because the same line names the ch10
counters sample. A spec that cannot fail is what this log has caught more than
once, and this one was caught by trying to break it before believing it.

### The sweep run against two real trees

Green on a clean tree, and green again on a tree carrying an unrelated
runtime.c change that moves instructions but no counted event — which is the
answer that change needed and could not otherwise have had cheaply.

Watched red by perturbing `allocs` in two goldens at once. It reports BOTH:

    === encode (bench/cost_golden_encode.txt)
    1c1
    < allocs=999999
    ---
    > allocs=14830625
    === pend (bench/cost_golden_pend.txt)
    1c1
    < allocs=888888
    ---
    > allocs=4007400
    counters moved: encode pend
    run with --write to regenerate them, then say why in design/compiler-log.md

Exit 1 when anything moved, 0 when nothing did. Not stopping at the first
disagreement is the property worth having: a session that learns about one
moved vein per build learns slowly.

### Two CI rounds this branch could not have avoided

The first was mine: the row check asserted `bench/<program>` exists, and
`bench/jsonbench/` is gitignored and written by `bench/make_jsonbench`, so it
is in any tree that has run the benchmarks and in no fresh clone. It read
state the repository does not carry, which is the shape of the bug this whole
file is about. The fix accepts either the directory or its generator.

The second was the compile row. CI drew `family0x6-model0x6a`, an Ice Lake-SP
with no row in `bench/compile_instructions_by_cpu.txt`, and refused — on a
value of 41,379,503, which is what the three recorded chips read. That is
four keys reading one number on one binary, and nothing here touches the
compiler: CLAUDE.md, a shell script and a Rust test file are none of them
`include_str!`'d. The row is recorded with the reading beside it. The
argument for collapsing the table still has the 5,064-instruction
falsification to answer and is not made here.

## 2026-09-05 (seventh) — a condition is asked, not read

Every work row in the corpus falls and none rises, both sides counted by CI:

    digestbench     80,559,329 ->     77,290,591    -3,268,738   -4.0576%
    encodebench  5,527,056,676 ->  5,322,691,304  -204,365,372   -3.6975%
    oneshot         28,533,505 ->     27,733,120      -800,385   -2.8051%
    pendbench      700,529,602 ->    681,319,796   -19,209,806   -2.7422%
    jsonbench    1,783,183,673 ->  1,745,058,173   -38,125,500   -2.1381%
    indexbench       4,792,124 ->      4,692,123      -100,001   -2.0868%
    widebench       57,839,754 ->     57,201,224      -638,530   -1.1040%
    scanbench    1,406,247,707 ->  1,395,728,077   -10,519,630   -0.7481%
    basket          39,878,996 ->     39,737,538      -141,458   -0.3547%
    deepbench      716,506,831 ->    714,674,831    -1,832,000   -0.2557%
    escapebench    130,170,757 ->    130,170,757             0    0.0000%
    readbench    2,038,397,546 ->  2,038,397,546             0    0.0000%

escapebench and readbench do not move by a single instruction, and that is
the control rather than a disappointment: neither has a comparison in a hot
loop — one tagged comparison in escapebench's whole emitted IR, two in
readbench's — so a change that only touches comparisons must leave them
exactly alone, and it does.

Nothing in the runtime or the libraries changed. This is `src/codegen.rs`.

### What the `if` was doing with a comparison

`b < 32` in the json escape path emitted an `icmp slt` and then a `select` on
it, building a KValue tagged 2 or 3. A phi merged that with `k_cmp`'s answer
from the guarded slow arm, and the `if` then called `k_not_failure` and
`k_truthy` on the phi and branched on the result. LLVM folds the pair away
where the select reaches the branch directly; through the phi it cannot,
because `k_cmp` may answer with a failure. What survived to machine code was

    xor %eax,%eax ; cmp $0x20,%r15 ; setl %al ; xor $0x3,%rax
    xor %edx,%edx ; cmp $0x2,%rax  ; jne

`setl` becomes a tag and two instructions later the tag is compared back.
`cmp $0x20,%r15 ; jge` is the whole of the work.

### How it was found, and how much that one site cost

Task #304 asked where the 66 instructions a byte in the escape fold go.
callgrind with `--dump-instr=yes`, merged with objdump, attributes the
ordinary-byte path — 9,834,000 of 11,658,800 steps — to the instruction: 20
for the frame, 13 for value plumbing, 25 for the inlined byte append of which
13 are guards, and 11 for the `b < 32` test. Those eleven are 108,174,000
instructions where 19,668,000 would do, **1.6013% of encodebench from one
comparison**.

The same 66 also answered #304's own premise. Nothing is heap-boxed: a KValue
is two words in registers, so "boxes both operands" named a tag
materialisation rather than an allocation. The step's real costs are the
indirect dispatch through the closure's stored `w_klam17` pointer, the frame
(six callee-saved pushes for a function that writes one byte, which is #290's
dimension), and this tag round trip.

### The change

`emit_cond` replaces `emit_expr` on the `if`'s condition. Where the condition
is a comparison and neither operand routes to a user arm, the guarded compare
branches straight to the then and else labels from its own i1 and builds
nothing. The slow arm is unchanged — `k_cmp`, a failure test, a truthiness
test — and its failure becomes one more arm of the `if`'s phi. Both `if` sites
go through it: the value form, where a failing condition joins the merge, and
the tail form, where it returns.

### What it costs

No allocation counter moves; all ten cost goldens are byte-identical. Emitted
code and machine code both fall: module_lines 5,068 -> 4,970, module_calls
781 -> 769, emitted_lines 12,344 -> 12,134, emitted_calls 1,859 -> 1,835,
emitted_branches 1,200 -> 1,163, emitted_other_lines 91,445 -> 90,198,
emitted_other_calls 14,913 -> 14,728, emitted_other_branches 8,878 -> 8,669,
and text 1,119,560 -> 1,117,736.

ONE counter worsens, and it is the one the change is made of:
**module_branches 415 -> 420**. Five branches is what a select and a phi and
two calls become when the comparison decides the branch itself, and it buys
twelve fewer calls and ninety-eight fewer lines in the same file, along with
every fall listed above. The trend gate asked for it by name and by the value
it landed on, which is 420.

compile_allocs and compile_memory hold: 25,490 allocations and 715,275 peak
bytes, both unchanged. compile_instructions RISES by 519, from 41,379,503 to
**41,380,022** on family0x19-model0x1 — 0.00125% of the front end.

`kanso check lib/json` emits no LLVM IR, so the front end runs no line this
change touched. What moved is the compiler's own layout: codegen.rs gained a
routine and everything after it sits at a different address. This repo has
measured that artifact before, when adding and removing allocas moved clang's
inlining budget inside untouched functions; 519 instructions is its size here.
The next run drew family0x6-model0xcf, an Emerald Rapids, and read the same
41,380,022 — a different vendor and a different microarchitecture landing on
the instruction. Two rows are re-sat and two are stale, and they stay stale
until CI draws them, one per run; the golden's bare line tracks the first of
those, so it does not move yet either.

That second reading matters beyond this change. The per-chip key exists
because two AMD models once read 41,503,893 and 41,498,829 on ONE binary, and
that divergence predates kanso#1241, which cut this row down to the compiler's
own frame and left the ifunc dispatch the key was invented to separate outside
the measurement. Every reading since has agreed: four chips on the old binary,
and now two on the new one before a third has been asked.

### The fixtures, and the pair that passed for the wrong reason

`tests/golden/micro/a_condition_is_asked_rather_than_read` runs the fast arm
in both `if` forms, a text comparison through the slow arm, and a `<` declared
over a record that must still reach its arm. Two runtime fixtures pin a
failing comparison, one per `if` form.

Both runtime fixtures were written first with the failure handed in as an
ARGUMENT, and passed. A failing argument short-circuits the arm before its
body runs, so the comparison was never reached: breaking the slow arm's
failure test left them green. Binding the failure inside the body reaches it,
and the endpoint trace shows the difference by losing its "passed through"
line. Found by trying to break them, which is the only reason it was found.

Three breaks, three reddenings. Inverting the fused branch moves the two
fused rows and leaves the text and record rows alone. Making the slow arm
skip its failure test reddens both runtime fixtures. Fusing where a record
routes to a user arm kills the micro program.

## 2026-09-05 (eighth) — the library's encoder gets a watcher

**DONE.** `bench/livebench` runs encodebench's program — decode
`bench/large.json` once, encode it four hundred times — against the library
that ships. It prints encodebench's exact checksum, 74072800, so the two do the
same work, and the difference between them is the only thing that separates
them: which copy of lib/json they hold.

**work_livebench 5,312,541,181**, MINTED. The baseline carries no reading for
it, so the first one is a measurement rather than a move. Beside encodebench's
5,322,690,905 that is a gap of 10,149,724, or **0.19%** — the frozen control's
drift from the shipped library since the snapshot was taken at 20ab931d on
2026-08-07. `live_instructions` joins the objective as a granted baseline,
entering at its dimension's standing, so welfare holds at 74.1474896668362 on
landing day and the term pays and charges from here.

### Why a benchmark that duplicates one already in the corpus

`bench/encodebench` and `bench/widebench` vendor frozen snapshots of lib/json,
and their READMEs say why: with the library held still, a move there is the
compiler's. kanso#1230 is the recorded case where that control did the
separating, and it stays.

What the freeze costs is that nothing then watches the library's own encoder at
load. `bench/jsonbench` is generated from lib/json but only decodes.
`bench/oneshot` imports the real library and encodes exactly once, inside a
program whose bulk is the decode beside it. So the whole encode side of the
shipped library was watched by a single encode of a 185 KB document.

That is not a hypothetical. A twelve-line change to lib/json's escape path that
skips the proved-clean prefix — the scan has already shown those bytes need no
escaping, so they leave in one copy instead of one at a time through a call —
falls 471,213 instructions on oneshot, 1.70%, and the objective priced it at
+0.0039 before the compile veins were regenerated and −0.02 after. It is being
held for this benchmark rather than shipped against the old corpus.

### The mutation, and what each gate saw

The ratchet row for the new vein carries a LIBRARY defect on purpose:
`escape_clean` re-encodes the string and folds it byte by byte instead of
taking the fast path when nothing needs escaping. Applied at 643399da:

    live_counters      RED    allocs 14,819,276 -> 18,300,076, +23.5%
                              append_fast 42,334,257 -> 56,462,657
    oneshot_counters   RED    the same defect, at one encode's scale
    encode_counters    GREEN  the frozen copy cannot see it at all

The third line is the argument for this benchmark and the argument for keeping
the frozen copy, in one reading.

The first draft of that mutation deleted the fast path outright and left `s`
and `n` unused, which the language refuses — so the tree did not compile, the
gate never ran, and the row would have been credited for a build failure. It
re-encodes `s` instead. `benchmarks` setup rebuilds the compiler first, which
matters here for a reason worth writing down: `lib/*.kso` is `include_str!`'d
into the compiler, so a library edit without a `cargo build` changes nothing.
In a worktree built before the edit, `kanso build` succeeded with
`lib/json/text.kso` holding outright syntax garbage.

### The sweep, and the veins it does not read

The cost goldens are eleven now, not ten. `scripts/gates/all_counters.sh` gains
its row and CLAUDE.md its count, both pinned by
`tests/every_counter_gate_is_in_the_sweep.rs` and both watched red: dropping
the row reports `no row for ["live"]`, and leaving the count at "ten" reports
`there are 11 cost goldens and CLAUDE.md counts them "ten"`.

CLAUDE.md also gains what this thread found the hard way: **the sweep reads the
runtime cost goldens and nothing else.** `machine_code`, `emitted_code`,
`compile_memory`, `compile_allocs` and `compile_instructions` are separate
gates, two of their counters are welfare terms, and a library change moves all
of them because the library is compiled into the compiler. The escape change
above read as a welfare RISE with those veins stale and a FALL once they were
regenerated, and nothing in the sweep would have said so.

### Round one: CI's own sitting, and a third chip goes bimodal

Two rows moved on the first CI round and neither is a regression.

**work_livebench 5,312,541,181 -> 5,312,541,628** is CI's sitting replacing the
container's; every other work row matched to the digit. The 447 is the same
per-host offset the gate's header documents.

**family0x19-model0x11 is pinned as a pair now, 41,379,503 41,380,022.** CI
counted the second value on a run whose diff against main touches nothing
compiled into the compiler — this branch adds a benchmark, a gate and a log
entry, and `git diff origin/main -- src/ Cargo.toml Cargo.lock lib/` is empty —
so the binary is the one kanso#1251 landed green at 41,379,503 on that same
family and model. Same silicon, same source, the other value.

That makes THREE of the four recorded chips seen reading both numbers, and the
two values are the same two every time. A key that separates chips which all
read the same pair is a key that is not separating anything, and the collapse
argument that has been waiting on evidence now has its third reading.

### Round two: two specs that were doing exactly their job

The suite caught what the counter gates could not, and both failures are the
same shape as the ones this benchmark exists to prevent.

**`every_benchmark_in_the_work_vein_has_a_direction`** reported livebench "in
`bench/instructions_golden.txt` and in no direction table, so a rise in any of
them reads as UNCLASSIFIED drift and the trend gate exits green". Its own
header records digestbench arriving in the vein a day before it arrived in the
table, and a 6.5x regression passing green in between. livebench is now in
`lower_j` beside digestbench, scanbench and readbench. A benchmark added to
make a fall visible, whose rises read as drift, would be worse than not adding
it.

**`the_score_says_what_it_was_made_of`** pins the objective's counter set by
hand rather than deriving it, so that a row can carry exactly what the formula
reads. `live_instructions` joined the model and not that list.

Neither was found locally, because the checks run before pushing were the
counter sweep, the ratchet, the trend gate, welfare, format, clippy and the two
specs the change obviously touched — not `cargo test`. The gates all agreed;
the suite did not. On a change that adds a row to a vein and a counter to the
model, the suite is the check that matters, and it costs sixteen minutes
against the two CI rounds it would have saved.

## 2026-09-05 (ninth) — the other half of the sweep

`scripts/gates/all_counters.sh` shipped three entries ago and reads the eleven
runtime cost goldens. It does not read the compile side, and a change under
`lib/` moves every vein there, because `lib/*.kso` is `include_str!`'d into the
compiler: a line added to lib/json is a line the compiler carries and compiles.

Earlier today that gap cost a reading. A twelve-line library change scored a
welfare RISE of +0.0039 with the compile veins stale, and a FALL of −0.02 once
they were regenerated. Two of the compile counters — `compile_allocs` and
`compile_peak_bytes` — are welfare terms, so the objective was being read
against numbers that no longer described the tree.

`scripts/gates/all_compile.sh` runs the set. Six gates: `machine_code`,
`emitted_code`, `compile_memory`, `compile_allocs`, `compile_instructions`,
`compile_libraries`.

Running them by hand is what it replaces, and the reason is not typing. Three
of the six refuse on a container whose glibc or rustc does not match the
golden's measured-on line, and a refusal exits non-zero exactly like a
regression. A session that runs them raw sees three failures and cannot tell
which kind it has. So the sweep reads `host_gate.sh`'s own refusal sentence —
"so the two cannot be compared", the only thing in the tree that prints it —
and separates three verdicts:

```
machine_code           AGREED
emitted_code           AGREED
compile_memory         REFUSED
compile_allocs         REFUSED
compile_instructions   REFUSED
compile_libraries      AGREED

not compared here (CI measures these): compile_memory compile_allocs compile_instructions
compile veins: nothing moved that this host can see
```

A gate that MOVED prints its diff inline. A gate that moved AND cannot compare
exits at the move first, before the host gate runs, so a real diff never
carries the refusal line.

It is not a gate. CI runs the six directly, each its own step with its own row
in the summary, and that is what fails a pull request. This exists so a
container can tell a moved vein from an unmeasurable one before pushing.

### The list is six, and I first wrote five

The five names were the ones CLAUDE.md carries, and they are what I put on the
line. Deriving the set instead — every script under `scripts/gates` that reads
a `bench/compile_*golden`, `bench/text_golden` or `bench/emitted_golden` —
turned up `compile_libraries` as well. Two scripts read those goldens and are
not gates: `compile_ir_row` takes four arguments and `compile_instructions`
calls it, and `build_benchmarks` says what it is in its own first line. Both
exclusions are written into the spec rather than left as a filter someone has
to reconstruct.

`tests/the_compile_sweep_names_every_compile_gate.rs` replays that derivation
against the tree, so a seventh gate added later is a red spec rather than a
vein nobody sweeps. It pins the same property in CLAUDE.md's own bullet, which
is the list a session actually reads before touching lib/ — that bullet named
five for as long as it was written down, which is the second count in that file
to be wrong on exactly those terms. The cost-golden count said four when there
were ten and was pinned this morning for the same reason.

All three tests were watched red before they were trusted: dropping
`compile_libraries` from the sweep gives `has no entry for
["compile_libraries"]`, adding a `compile_nothing` gives both `which reads no
compile golden` and `which is not in scripts/gates`, and dropping the name from
CLAUDE.md gives `does not name ["compile_libraries"]`.

## 2026-09-05 (tenth) — the pair was two binaries, and so was the objective

`bench/compile_instructions_by_cpu.txt` pinned `family0x19-model0x11 41379503
41380022`. `compile_ir_row.sh` says what a pair is in its own words: "a key and
the TWO values one chip has been seen to read ON ONE BINARY." These were two
binaries.

Read off git:

```
31c54078   all four chips 41,379,503                the pre-#1251 binary
643399da   #1251 changes codegen.rs; CI re-sits model0x1, then model0xcf,
           both to 41,380,022. model0x11 and model0x6a left stale, and
           #1251's own entry says so
65725d05   CI draws model0x11 on the post-#1251 binary and counts
           41,380,022 -- against a row still carrying the stale value
```

I read the leftover as a second mode and wrote the pair. The 519 gap is what
made it plausible: the within-binary residual recorded earlier on this vein was
508, so a difference of that size between two binaries looks exactly like the
thing the pair exists for.

### What it cost

The golden's bare line follows the first row's first value, so
`bench/compile_instructions_golden.txt` read 41,379,503 — the pre-#1251
number. `compile_instructions` is a welfare term and a trend-gate counter, so
from #1251 until now the objective was scored against a value no chip had
counted on this binary. Nothing went red, because the pair admitted the true
value on every run that drew that chip.

#1251's own entry saw the shape and wrote it down: "the golden's bare line
tracks the FIRST row, which is one of the stale three, so it does not move yet
either." Recording that the golden is stale is not the same as it being right,
and a day is what the difference cost.

### The fix

model0x11 reads 41,380,022 alone. `family0x6-model0x6a` is removed rather than
corrected — it has never been measured on this binary, and the rule written six
times in that file's header is that a value counted against the old binary is
worse than no value. Three rows remain, all 41,380,022, all CI's own readings
on this binary.

Welfare goes 74.14748966683695 -> 74.14745031572936, a fall of 0.0000394, and
the floor is re-set with that reason. It is a correction rather than a
regression: the compiler did not change, the number recorded for it did.

### The spec, watched red on the defect itself

Every pair written before this one cites its binary — "same chip 0x19/0x11,
same binary sha 0e081d4c2c96: 41,845,704". The convention was there and nothing
checked it, and the entry I wrote cites no sha because there was none to cite.

`tests/a_paired_chip_row_names_one_binary_twice.rs` asks for it: a row pinning
two values must sit under a dated entry naming both values beside a binary sha.
It went red on main as it stood, which is the strongest form of watching a spec
fail — the failing case is the defect rather than a mutation of it.

Two drafts of it passed on that defect first, and both are worth writing down
because both are the shape this repo keeps catching. The first asked for eight
hex characters and found them in `41379503`, which is eight hex characters: the
check answered yes using the very number it was demanding a citation for. The
second scoped the search to "a run of comment lines", which in a file that is
one long header followed by its rows is the whole header, so a sha from July
justified a pair from September. The file's real unit is the dated entry.

### What it does to the collapse argument (#303)

The "third reading" — one chip seeing both values on one binary — does not
exist. What the record supports is four chips agreeing at 41,379,503 on the
pre-#1251 binary and three agreeing at 41,380,022 on the post-#1251 one. That is
a stronger argument for collapsing the key than the one it replaces: unanimity
per binary, rather than a chip disagreeing with itself.

And it puts a question to the pair itself, which is on the ledger for Clay:
`compile_instructions.sh` already records that the 508 the pair was ruled a
fallback for "actually was: two binaries." Both recorded pairs are now two
binaries, and neither is the within-binary bimodality the mechanism was built
for.

### The same CI run drew a fifth chip, and it agrees

kanso#1253 went red on `compile instructions`, and the failure is a refusal
rather than a regression: CI drew `family0x1a-model0x2` — AMD family 0x1a, a
generation this pool had never produced — and no row named it. Its own log:

```
compile_sample cpu="cpu family 0x1a model 0x2" sha=31ccb3e99dde row=41380022
##[error]nothing in bench/compile_instructions_by_cpu.txt was counted on
##[error]family0x1a-model0x2, so this run's 41380022 cannot be compared
```

41,380,022 is what the three recorded chips read on this binary, first time of
asking, from a generation none of them belongs to. Five keys, three AMD
generations and one Intel, one binary, one number.

Two things follow. The correction above no longer rests only on reading git:
the golden said 41,379,503 and a chip that had never been asked counted
41,380,022. And the collapse argument (#303) has its evidence from a direction
nobody arranged — the key is not separating chips, because five of them across
three microarchitectures cannot be separated.

### And the ratchet admitted a counter, which a spec was waiting to catch

`welfare_saturates_each_counter` went red on the `--set` above, and its own
header had described the case in advance: a minted counter enters the floor's
baseline at the next ratchet rather than at the merge that mints it, so the
pull request that adds one usually leaves the spec green and the NEXT `--set`
turns it red.

kanso#1252 minted `live_instructions`. This change is the next `--set`, and it
touches nothing about the run-speed half — it is a correction to
`compile_instructions` — so the admission arrived here. Ten guards became
eleven and the guard runaway went 49.00 to 48.91, which is the arithmetic the
header pins: `((n-1)/3 + 1024/1026) / n * 0.15` reads 0.059971 at ten and
0.059064 at eleven, and 0.00091 of weight is 0.09 of score. Both neighbours
held, for the reasons already written there — parity does not depend on the
count, and the advertised runaway still has two rows.

The numbers are recomputed rather than derived, as that header insists.

### The failure it wore first was a full disk

Before any of that, all three of its tests failed with `the floor reads: Os {
code: 2, kind: NotFound }`. The container was at 97% with 1.5 GB free, and the
fixture copies the whole of `bench/` into a staging directory three times. The
copy that lost the race was silent because `std::fs::copy` had already
returned; what failed was the read of a file that never landed.

Deleting 3.7 GB of stale scratch worktrees changed the message to the real
assertion. Worth knowing, because "NotFound" on a file the fixture just copied
reads as a bug in the fixture and is not one, and because I called it a
race between two concurrent test runs before checking — it fails alone.

### Two gates the correction owed, and only one of them was foreseen

CI went red a second time on this branch, and neither failure was a counter.

`page_drift` counted five log entries against a budget of three. Today's
campaign put five entries in the log and moved the page not at all, which is
exactly what that gate is for — its own words are "several entries with no page
edit means the presented design has fallen behind what the compiler does". §52
is the entry the log owed: one section for the campaign rather than one per
commit, covering the two sweeps, why a library edit moves the compile veins,
what the frozen benchmark cannot see, and the stale row.

`golden_prose` then caught what the correction itself had done. The page quotes
`compile_instructions` in a `data-golden` span, so moving the golden to
41,380,022 made the prose stale by 519 the moment it was right. That is the
gate working as designed: a number on the page and a number in a golden are the
same claim, and only one of them had been edited.

Worth writing down that the drift gate reads `git log -1 -- docs/compiler.html`,
so an uncommitted page edit is invisible to it. It went on reporting five
entries ahead until the edit was committed, which reads like the fix not
working.

## 2026-09-05 (eleventh) — the ratchet's touched mode keys on the guard

`kanso run scripts/ratchet -- touched origin/main` proves the rows a branch
could have made inert. It decided which by reading each mutation's whole script
and keeping any row that MENTIONED a changed path, and that is the wrong
question. A mutation carries `grep -q '<literal>' <file>` so that it refuses
rather than silently applying when the text it patches has moved. That guard is
the row's dependency on the tree: a change can only make a mutation inert by
moving what the guard matches. Everything else a script mentions is incidental.

Measured against three merged diffs, on the 73 mutations on disk today:

```
kanso#1249, 45 changed files    naming 32 rows -> guard 14
kanso#1252, 21 changed files    naming  7 rows -> guard  1
kanso#1253, 10 changed files    naming  8 rows -> guard  1
```

The gap is goldens. A branch that regenerates them has made no mutation inert,
because a golden guard cannot go stale from a regeneration: over the whole
corpus exactly two carry a value-shaped number and both are POST-conditions
checking what the mutation itself just wrote. Every other golden guard matches
a key name and a digit class — `^jsonbench [0-9]`, `^defines=999999` — which a
new number still satisfies. So a row whose only touched guard is a golden has
nothing to prove, and the rule drops it.

### The comment it replaces was wrong by an order of magnitude

It said the intersection is "usually empty and occasionally one to three rows",
and cited three branches that touched one source file each. True of those, and
false of any branch that regenerates goldens — which is most performance work.
The replacement gives a range and names the case it does not help: `src/runtime.c`
alone carries twelve guards, so a runtime branch selects a large block however
the rule is written.

### The numbers I shipped were not the numbers I had measured

The change was prepared against 72 mutations and cited 33 → 12 and 8 → 1.
kanso#1252 added `an_encoder_that_walks_a_clean_string.sh`, and re-measuring on
today's tree reads 32 → 14 and 7 → 1. Nothing about the rule changed; the corpus
did. Both readings are in the comment now, with their mutation counts, because a
figure taken against a moving corpus is an order of magnitude rather than a pin
— and that is exactly why the three specs beside it assert no count at all.

### The specs

`tests/a_golden_in_the_diff_selects_no_extra_row.rs` drives the rule through a
new `select` mode, which takes its file list from the command line rather than
from `git diff`. `touched` asks git, so a spec for it needs a repository shaped
for the check; `select` is the same rule with the list handed in, so the diffs
are ones the spec wrote down.

Three properties, each false under the naming rule:

- a goldens-only diff selects nothing (naming: 11 rows)
- adding goldens to a source diff selects exactly what the source file selects
  alone
- every row a source file selects guards on that file — reading the mutations
  directly rather than trusting the selection, because a rule that keyed on
  something other than the guard could satisfy the first two and still be wrong

All three were watched red against the rule they replace, and each failed with
its own message: `14 rows were selected and 12 mutations guard on src/runtime.c`,
`a goldens-only diff selected 11 rows`, and the equality between the two
selections.

---

## 2026-09-05 (twelfth) — the negative render writes behind its sign

**DONE.** `k_render_at`'s shortest-round-trip branch rendered a negative
double into a 63-byte stack scratch and then `strcpy`'d the whole rendering
one byte right, behind the minus the caller had just written. It now hands
`render_ryu` the address one past the sign and lets it write there directly.

Half of what encodebench renders is negative — 430,400 of its 849,200 ryū
renders, counted with a probe — so the copy was paid on every other value.

### The measurement, container, both sides in the repo root

Same directory, same paths, benchmarks rebuilt in place between the two.
`scripts/gates/instructions.sh` records why that matters: the exec path lands
on the new process's stack and libc walks it before main, so a binary measured
from a different directory is off by a fixed amount that reads like a small
regression.

    encodebench   5,322,690,905 -> 5,310,898,414   -11,792,491   -0.2216%
    livebench     5,312,541,181 -> 5,300,748,646   -11,792,535   -0.2220%
    oneshot          27,732,721 ->    27,702,706       -30,015   -0.1082%
    basket           39,737,125 ->    39,736,877          -248
    jsonbench     1,745,057,760 -> 1,745,057,595          -165
    deepbench       714,675,476 ->   714,675,319          -157
    escapebench     130,170,358 ->   130,170,201          -157
    readbench     2,038,397,133 -> 2,038,397,263          +130
    widebench        57,200,811 ->    57,200,932          +121
    digestbench      77,290,192 ->    77,290,383          +191
    pendbench       681,319,383 ->   681,319,612          +229
    indexbench        4,691,710 ->     4,692,009          +299
    scanbench     1,395,727,664 -> 1,395,727,963          +299

Ten of the thirteen move by under 300 in either direction, which is layout:
the by-cpu file's own reading puts a `.text` change's reach on a count at about
a thousand. The three that move are the three that render floats. 27.4
instructions saved per negative render.

livebench is the confirmation the frozen control cannot give. It runs
encodebench's program against the library that ships rather than the vendored
snapshot, and it falls by the same 11.79M to within 44 instructions — which is
what a RUNTIME change should do to both, where a library change moves only the
live row.

### The fixture, watched red on the off-by-one it risks

`tests/golden/micro/a_negative_double_renders_behind_its_sign.kso`: sixteen
negatives across the shortest-round-trip path — both exponent extremes,
seventeen significant digits, the thirds, 1e-07 — and a positives line that
takes the other branch and must not move. The language has no exponent
literal, so the wide exponents are built by multiplying a 31-digit literal.

Broken by sending the rendering to `buf` instead of `buf + 1`, which is the
exact off-by-one this arrangement risks, it fails with all sixteen negatives
missing their sign and the positives line untouched. That mutation is now
`scripts/ratchet/mutations/native_drops_a_negative_sign.sh` with a row on the
micro corpus: `native_renders_a_float_wider` covers the integral fast path and
stops there, so seventy-three mutations had never broken a sign.

### The buffer claim, measured rather than argued

`render_ryu` never writes a sign for a nonzero double — it is handed `-d`,
which is positive — and `-0.0`, the one value that would collide, is caught
upstream by the integral fast path and never reaches this branch.
`a_whole_float_keeps_its_point` already pins that `-0.0` renders `-0.0`.

Longest rendering, probed with a max-strlen counter inside `render_ryu`: 8 on
encodebench's own data, 23 on the fixture. The analytic worst case is 23 plus
the terminator. The scratch it replaces was 63 and the caller's buffer is 64,
so writing behind the sign has forty bytes to spare rather than one.

### The veins

`all_counters.sh`: all eleven agree. The change removes instructions rather
than counted events — no allocation, no render, no arena block moves.

`all_compile.sh`: `machine_code` MOVED and is regenerated. Every benchmark's
`.text` falls 48 bytes, except escapebench and indexbench at 32. That is the
63-byte scratch and its `strcpy` call going away, and it is a fall to bank.

`bench/compile_instructions_by_cpu.txt` is emptied. `src/runtime.c` is
`include_str!`'d into the compiler, so all four rows counted a binary that no
longer exists, and this file's rule — written nine times in its own header now
— is that a value counted against the old binary is worse than no value. CI
supplies the first row of the next series and the golden's bare line follows
it.

### And a spec I wrote yesterday was too strict about that

`the_golden_follows_the_first_row` said `.expect("the table has a row")`. An
empty table is a state the design HAS: it is where the table sits between a
change to the compiler's bytes and CI's first sitting, and `compile_ir_row.sh`
handles it in its own first branch, printing the row to add. The spec now asks
nothing when there is no first row, rather than failing with a sentence about
a row that is absent on purpose. Watched red both ways: the old form panics
`the table has a row` against today's emptied table.

### Two counts in the ratchet's own comment, corrected

The comment #1254 landed says "src/runtime.c alone carries twenty guards". It
was counting guard LINES, and the rule selects ROWS: twelve mutations guarded
on the file across twenty-one lines, and the row added here makes it thirteen
across twenty-three. The 32→14 / 7→1 / 8→1 figures beside it were taken at 73
mutations and there are 74 now; both readings stay, with their counts, for the
reason that paragraph already gives.

**OPEN.** The work vein (`bench/instructions_golden.txt`) cannot be
regenerated here — this container is glibc 2.39-0ubuntu8.7 against the golden's
8.8, and `host_gate.sh` refuses. CI measures it; its rows and the compile
sitting land in a follow-up commit on this branch, and welfare moves with them.

---

## 2026-09-05 (thirteenth) — CI's rows for the entry above, and the two hosts agree to the instruction

**DONE**, closing the OPEN thread in the twelfth entry.

`bench/instructions_golden.txt` is regenerated from the CI job that measured it.
The container could not: it is glibc 2.39-0ubuntu8.7 against the golden's 8.8,
and `host_gate.sh` refuses rather than letting a reader compare rows counted on
two libcs.

    jsonbench     1,745,058,173 -> 1,745,058,008        -165
    encodebench   5,322,691,304 -> 5,310,898,813   -11,792,491   -0.2216%
    oneshot          27,733,120 ->    27,703,105       -30,015   -0.1082%
    basket           39,737,538 ->    39,737,290          -248
    widebench        57,201,224 ->    57,201,345          +121
    deepbench       714,674,831 ->   714,674,674          -157
    escapebench     130,170,757 ->   130,170,600          -157
    pendbench       681,319,796 ->   681,320,025          +229
    indexbench        4,692,123 ->     4,692,422          +299
    scanbench     1,395,728,077 -> 1,395,728,376          +299
    digestbench      77,290,591 ->    77,290,782          +191
    readbench     2,038,397,546 -> 2,038,397,676          +130
    livebench     5,312,541,628 -> 5,300,749,093   -11,792,535   -0.2220%

**Every delta is the container's delta to the instruction.** The absolute rows
differ — CI's encodebench is 5,310,898,813 where the container counted
5,310,898,414, a fixed 399 that is the exec path on the process's stack — but
the thirteen differences are identical on both hosts. That is worth having
written down: it says the offset is additive and the vein's deltas survive a
host change even though its rows do not.

### The compile row

CI drew `family0x19-model0x1` on a table the change had emptied, refused rather
than comparing to anything, and printed the row to add:

    compile_sample cpu="cpu family 0x19 model 0x1" sha=fe268554d931 row=41381326

So the new series opens at 41,381,326 against the old series' 41,380,022, a rise
of 1,304 from a change that touches only the runtime. `kanso check lib/json`
emits nothing, so the code that changed cannot run during the measurement; what
moved is layout, for the tenth time on this vein. `compile_allocs` holds at
25,490 and `compile_peak_bytes` at 715,275 across it.

### Welfare

74.14745031572936 -> 74.15241668591877, a rise of 0.00496637, and `--set` in
the same change. Two encode instruction terms fall by ~0.22% against one compile
instruction term rising 1,304 on a satiated dimension; the sum goes up and that
trade is what the weights are for.

The page's `compile.compile_instructions` span moves 41,380,022 -> 41,381,326
with the golden, because a number on the page and a number in a golden are one
claim.

### The six rows that rose, priced one by one

The trend gate asks for the counter's key and the value it landed on, and it is
right to: a paragraph that names a counter without its number is a blanket
permit for that counter for the whole branch, which is how kanso#1205's
explanation once licensed a mutation to set the same row to 999,999,999 and
leave the gate green.

Six work rows rose, all by between 121 and 299 instructions:

    work_widebench    57,201,224 -> 57,201,345    +121
    work_digestbench  77,290,591 -> 77,290,782    +191
    work_pendbench   681,319,796 -> 681,320,025   +229
    work_indexbench    4,692,123 -> 4,692,422     +299
    work_scanbench 1,395,728,077 -> 1,395,728,376 +299
    work_readbench 2,038,397,546 -> 2,038,397,676 +130

None of them renders a float, and the change is confined to the negative arm of
`k_render_at`. What reaches them is the `.text` move: every benchmark's machine
code fell 48 bytes (32 for escapebench and indexbench), and a `.text` change of
that size reaches a count at about a thousand instructions, which is the reading
`bench/compile_instructions_by_cpu.txt` records for the same effect on the
compile row. All six are inside that. The three rows that fall — encodebench,
livebench and oneshot — fall by 11.79M, 11.79M and 30,015, which is two to five
orders of magnitude more, and they are the three that render.

### A second chip on the new series, and it agrees

The re-run drew `family0x6-model0x6a`, the Ice Lake-SP, found no row for it on
the fresh table and refused:

    compile_sample cpu="cpu family 0x6 model 0x6a" sha=fe268554d931 row=41381326

41,381,326 — what family0x19-model0x1 read on the same binary. Two chips, two
vendors, one number, on the first binary of the new series. That is the tenth
reading in the unanimity record the header of that file now keeps.

### A third chip, and the key's price per binary is now a measured figure

The round after that drew `family0x6-model0xcf`, Emerald Rapids, the second
Intel. No row, refusal, 41,381,326 — the same number for the third time.

    family0x19-model0x1   41,381,326   AMD Zen 3
    family0x6-model0x6a   41,381,326   Ice Lake-SP
    family0x6-model0xcf   41,381,326   Emerald Rapids

Three keys, two vendors, one binary, one number, and it took THREE CI rounds to
learn because each chip refuses once. That is what the per-chip key costs per
binary that touches the compiler's bytes, and it is now a figure rather than an
impression: one red round per unrecorded chip, and the pool has produced five
distinct keys since the table was introduced.

It belongs beside the collapse argument (#303) as the cost side, not as evidence
for it. The evidence for it is the unanimity: eleven readings now, across three
binaries, and every reading on a binary agrees with every other reading on that
binary.

---

## 2026-09-05 (fourteenth) — two things the counters could not see, one of them a decline

Both came out of reading the encode profile on the binary kanso#1256 landed
(c072c8b5), looking for the next candidate. Neither is a change. Both are
recorded because an unrecorded negative result gets rediscovered.

### The counter run is a different execution, and nothing said so

**DETERMINED, no defect.** `k_b_append_mut_byte` and every other in-place fast
arm opens with

    %bso = load i32, ptr @k_stats_on
    %bcount = icmp ne i32 %bso, 0
    br i1 %bcount, label %slow, label %bfast

so `KANSO_COUNTERS` does not add arithmetic to a run — it sends every append,
push and put through the C. One binary, two runs under callgrind:

    counters off   5,310,898,414
    counters on    6,450,750,955
    delta          1,139,852,541   +21.4625%

The golden reports `allocs=14,830,625` for the same program, and 1,139,852,541 /
14,830,625 = 76.86 instructions per counted event, which is the size of one call
into the C with its frame. The arithmetic fits the mechanism.

**I first read this as the allocation counters overcounting** — the bail
degrading a mutating append into an allocating one, which would have meant the
objective was weighing allocations production never makes. `k_b_append_into`
kills that: its slow arm is `k_b_append_mut`, mutate=1, which on the same
condition the fast arm tests writes in place, returns `acc`, allocates nothing
and increments `k_stat_append_fast`. The C reproduces the fast arm's
classification rather than defeating it, and `push_mut_fast` and `put_mut_fast`
are the same shape.

So the counted events are honest and each vein is honest about what it claims:
the cost goldens count events, the instruction golden counts production work.
What is worth writing down is only that the two veins measure two DIFFERENT
EXECUTIONS of the same binary, 21.46% apart on encodebench, and that this
explains rather than is explained by a pattern in the record — kanso#1221
through #1224 each moved the instruction vein with no allocation counter
moving, because the fast paths they added are invisible to a run that does not
execute them.

### The utf-8 validity bit, DECLINED by arithmetic before it was built

`k_utf8_bad_wide` is 111,966,800 instructions on encodebench, 2.108%, from 400
calls of `k_b_utf8` — once per iteration over all 188,698 bytes of the finished
document at 1.4834 instructions a byte. The ascii fast path cannot answer
because 5,865 of those bytes have the high bit set. The encoder is validating
utf-8 it produced itself: structural characters, the ascii output of `k_itoa`
and `render_ryu`, and strings that were already `str`.

The obvious fix is a "still valid utf-8" bit on the bytes header, held by an
append of a `K_STR` or an ascii byte and cleared by anything else, with
`k_utf8_finish` skipping the pass when it holds. The surface is small: `KBytes`
already has eight spare bytes per header, because the arena stride is 32 and the
struct is 24; two C constructors set the tag; five sites write bytes; one site
reads the answer.

**It does not pay, and the encode counter already said so.**

    append_fast=42312800

42,312,800 appends across 400 iterations is 105,782 an iteration for 188,698
document bytes — 1.78 bytes an append. Maintaining the claim costs a load, an
and and a store on that arm:

    3 x 42,312,800 = 126,938,400 to maintain
                     111,966,800 saved
                     ------------
                     +14,971,600, a net LOSS of 0.28% on encodebench

Even at two instructions an append it is 84.6M against 112M, a margin a fourth
header field's cache pressure would erase.

**The general reason is worth more than the number.** The validator reads every
byte once at 1.4834 instructions apiece, and the appends already touch every one
of those same bytes. Per-append tracking cannot be cheaper than a per-byte scan
of the same bytes; it is the same work moved earlier and paid whether or not
anyone asks for it. Skipping a validation means proving validity, and the proof
costs what the check costs.

Two variants die the same way. ORing each byte into an accumulator and testing
the high bit at the end IS the ascii test, which this document fails by 5,865
bytes. Letting the byte arm clear unconditionally and only strings hold the bit
leaves the bit clear at every read, because the encoder appends bytes constantly.

What would change the answer is a shape where bytes arrive in far fewer, far
larger appends — well above 1.78 bytes an append. Neither the encoder nor any
benchmark in the corpus is that shape.

---

## 2026-09-05 (fifteenth) — a small record copies its fields; the same trick on strings was declined by the objective

**DONE**, and the shape of the result matters as much as the number: the change
was built as one thing, measured, DECLINED by welfare, split in two, and the
half that survives is the half that ships.

### Where it started

The encode profile's fourth line is `__memcpy_avx_unaligned_erms` at 296,718,038
instructions, 5.59%. Asking who calls it splits into two populations:

    105,821,532 (1.99%)  < k_b_append_grow        (10,400x)   10,175 each
     61,271,600 (1.15%)  < encode_onto_2'2     (4,185,200x)     14.64 each
     52,089,200 (0.98%)  < escape_clean_4      (3,480,400x)     14.97 each
     40,132,800 (0.76%)  < k_rec               (3,344,400x)     12.00 each
     23,795,613 (0.45%)  < k_str               (1,686,801x)     14.10 each
     13,007,200 (0.24%)  < render_ryu            (849,200x)     15.32 each

`k_b_append_grow` copies whole buffers and the vector path earns its entry
there. The rest are twelve million calls moving a handful of bytes each, where
glibc's entry sequence and size classification cost more than the move.

### What was built, and what the objective said about it

Two sites, both C: `k_rec`'s field copy, and `k_str_n`'s payload copy through a
small-length helper doing two OVERLAPPING loads and stores of the widest type
that fits — a pair covering `[0,w)` and `[len-w,len)`, which never writes past
the end and needs no capacity slack.

    encodebench   5,310,898,414 -> 5,283,224,747   -0.5211%
    livebench     5,300,748,646 -> 5,273,072,605   -0.5221%
    pendbench       681,319,612 ->   658,524,726   -3.3457%
    readbench     2,038,397,263 -> 2,000,661,674   -1.8512%
    widebench        57,200,932 ->    57,072,940   -0.2238%
    oneshot          27,702,706 ->    27,667,778   -0.1261%
    basket           39,736,877 ->    39,685,682   -0.1288%
    jsonbench     1,745,057,595 -> 1,750,209,508   +0.2952%
    scanbench     1,395,727,963 -> 1,409,772,188   +1.0062%
    indexbench        4,692,009 ->     4,998,687   +6.5362%

Seven fall and three rise, and **welfare reads 74.14 against a floor of
74.15**. That is a fall, and the rule is not negotiable: the change goes, or the
claim is that the weights are wrong. The weights are not wrong here — the
decoder is what rose.

### The split, and why it was the obvious next measurement

The rises are all decode-side, and only one of the two sites is on the decoder's
hot path: `k_str_n` builds a string for every token. Its payload is a json token,
often longer than the sixteen bytes the helper handles, so the added length
dispatch is a tax paid on every token to help none of them. Removing that half
and keeping `k_rec`'s:

    encodebench   5,310,898,414 -> 5,297,520,814   -0.2519%
    livebench     5,300,748,646 -> 5,287,371,046   -0.2524%
    pendbench       681,319,612 ->   666,111,812   -2.2321%
    oneshot          27,702,706 ->    27,669,262   -0.1207%
    basket           39,736,877 ->    39,728,728   -0.0205%
    scanbench     1,395,727,963 -> 1,395,727,874         -89
    jsonbench, widebench, deepbench, escapebench, indexbench,
    digestbench, readbench                          exactly 0

Nothing rises. **welfare 74.16 against the same 74.15 floor.**

So `k_rec` ships and `k_str_n` does not, and the reason is legible rather than
lucky: a record has two or three fields far more often than many, and the count
is small enough that copying whole KValues beats a call. A json token is not
that shape.

### What it costs

Machine code rises 64 bytes per benchmark — the unrolled copy is bigger than the
call it replaces — and `bench/text_golden.txt` is regenerated with that rise
stated. Whether the objective should weigh code size at all is the open question
in design/pending-gavels.md; today it does not, and this is the kind of trade
that would move if it did.

`k_rec_reuse` a few lines above already copied fields with the same loop, which
is precedent rather than coincidence: the shape was already known to be right
where the count is small, and only the fresh-record path still paid the call.

### The ratchet row

The memcpy could not be wrong about its count, because the count was an
argument. A loop can be. `a_small_record_drops_its_last_field` changes `i < n`
to `i < n - 1` and the micro corpus goes red on the first fixture it reaches —
watched, restored, green. The loop is written over a local `dst` so that the
guard and the sed name one site: `k_rec_reuse`'s loop is textually identical
otherwise.

### CI's rows, and every delta matching the container's

The work vein could not be regenerated here — this container is glibc
2.39-0ubuntu8.7 against the golden's 8.8 and `host_gate.sh` refuses. These are
CI's, copied out of the job log, and every one of the thirteen differs from the
container's reading by the same fixed 399 the exec path costs:

    work_encodebench   5,310,898,813 -> 5,297,521,213   -13,377,600   -0.2519%
    work_livebench     5,300,749,093 -> 5,287,371,493   -13,377,600   -0.2524%
    work_pendbench       681,320,025 ->   666,112,225   -15,207,800   -2.2321%
    work_oneshot          27,703,105 ->    27,669,661       -33,444   -0.1207%
    work_basket           39,737,290 ->    39,729,141        -8,149   -0.0205%
    work_scanbench     1,395,728,376 -> 1,395,728,287           -89

The other seven rows do not move at all. NO WORK ROW RISES.

**THE COMPILE ROW.** CI drew family0x19-model0x1 on the table this branch
emptied, refused rather than comparing against nothing, and printed the row:

    compile_sample cpu="cpu family 0x19 model 0x1" sha=77aec04ae49f row=41379381
    compile_sample cpu="cpu family 0x6 model 0xcf" sha=77aec04ae49f row=41379381

The second is Emerald Rapids, drawn on the next round; it found no row on the
new series, refused, and counted the same number to the instruction. Two chips,
two vendors, one binary, one number — and two CI rounds to learn it, because
each chip refuses once.

`compile_instructions` lands on 41,379,381 against the old series' 41,381,326, a
FALL of 1,945 from a change that touches only the runtime. `kanso check lib/json`
emits nothing, so the code that changed cannot run during the measurement; what
moved is layout, for the eleventh time on this vein. compile_allocs holds at
25,490 and compile_peak_bytes at 715,275.

**AND `text` WORSENED, which is this change's real price.** The trend gate went
red on it and was right to: the counter is `text`, it lands on **1,117,768**
from 1,117,192, a rise of 576 across the corpus. That is the inline loop's
machine code, 64 bytes in each of nine benchmarks, and it is what the branch
BUYS the 13.4M encode instructions with. Naming the fall without naming the rise
is the shape kanso#1205 caught, so both are here with their values.

**WELFARE 74.15241668591877 -> 74.15975334184456**, a rise of 0.00733666, `--set`
with its reason in the same change. The page's
`compile.compile_instructions` span moves with the golden, because a number on
the page and a number in a golden are one claim; `golden_prose` reads 0 drifted.

## 2026-09-05 (sixteenth) — a short append copies its own bytes, and the corpus could not see the mutation that proves it

**DONE**, and the interesting half is the spec rather than the number: the
mutation this change ships with left the micro corpus GREEN the first time,
which said the corpus had a hole where this arm should be.

### Where it started

kanso#1258's k_rec fix took one family of small copies out of the encoder. The
profile on the binary it landed still put **256,585,238 instructions, 4.84% of
encodebench, in `__memcpy_avx_unaligned_erms`**, and the caller tree divides it
into two different things:

    k_b_append_grow  105,821,532 over     10,400 calls   10,175 each
    encode_onto_2'2   61,271,600 over  4,185,200 calls     14.6 each
    escape_clean_4    52,089,200 over  3,480,400 calls    14.97 each
    k_str             23,795,613 over  1,686,801 calls     14.1 each
    render_ryu        13,007,200 over    849,200 calls     15.3 each

The first row is a buffer growing and is real work. The rest are calls into
glibc to move a handful of bytes, where most of the instructions are spent
deciding how wide a move to make.

The two big ones are the SAME SITE. `k_b_append_mut_byte` is emitted by
`src/codegen.rs` and marked `alwaysinline`, so it lands inside whichever
function appends; its string arm's `swrite` block held the only
`llvm.memcpy` in emitted IR, at a runtime length. What it copies is an object
key, a `"true"` or a `"null"`.

### What it does

Sixteen bytes or fewer are copied by a pair of overlapping loads — the first
eight bytes and the last eight, each end read once and written once, no loop
and no count — with a four-byte pair below eight and three single bytes below
four. Anything longer keeps the call, where the vector path earns its entry.

**MEASURED**, thirteen benchmarks under callgrind from the repo root, both
sides, against the k_rec tree:

    encodebench   5,297,520,814 -> 5,241,342,014   -56,178,800   -1.0605%
    livebench     5,287,371,046 -> 5,231,192,246   -56,178,800   -1.0625%
    oneshot          27,669,262 ->    27,528,815      -140,447   -0.5076%
    widebench        57,200,932 ->    57,104,953       -95,979   -0.1678%

The other nine rows do not move at all. NO ROW RISES. encodebench and livebench
fall by the same 56,178,800 to the instruction, which is what a change to what
the compiler EMITS does to a program and to the same program built against the
shipped library.

**WELFARE 74.16 -> 74.18** with kanso#1258 already in.

### What it costs, and where the linker disagrees with the emitter

Every program gains **54 emitted lines**, because the ladder is written once
into each module's intrinsic prelude. Only FOUR gain machine code:

    jsonbench     92,434 ->  92,722    +288
    encodebench  112,978 -> 113,650    +672
    oneshot      114,866 -> 115,826    +960
    widebench    117,010 -> 118,482  +1,472

and the other eight are byte-identical. The linker drops the arm in the
programs that never append a string, so the emitted count and the .text count
disagree about what this change costs — which is the reason both veins exist
rather than one.

SEVEN counters worsen and every one is named by its KEY with the value it lands
on, because a paragraph that prices only the fall is the shape kanso#1205
caught — and naming a counter loosely is the same failure, which this entry
committed on its first draft and the gate caught:

    text                    1,117,768 -> 1,121,160    +3,392
    emitted_lines              12,134 ->    12,188       +54
    emitted_calls               1,835 ->     1,836        +1
    emitted_branches            1,163 ->     1,171        +8
    emitted_other_lines        90,198 ->    90,792      +594
    emitted_other_calls        14,728 ->    14,739       +11
    emitted_other_branches      8,669 ->     8,757       +88

The first draft wrote "the decoder's `emitted` lands on lines=12,188" and left
the other five unnamed, and `scripts/trend_gate` refused all six: the key is
`emitted_lines`, not `lines`, exactly as it is `work_widebench` and not
`widebench`. The gate is right to be literal about this. A paragraph that names
a counter by a nickname reads as pricing to a human and as silence to the gate,
and the whole point of the rule is that the gate is the one that cannot be
talked round.

The decoder's three and the other eleven programs' three move for one reason:
the ladder is written once into each module's intrinsic prelude, 54 lines and 8
branches and one call apiece. That is what the encode's 56,178,800 instructions
are bought with, and the objective says the trade is worth making.

### The mutation was right and the corpus was blind

The obvious mutation is the copy-paste this shape invites: the eight-byte
pair's two stores differ in one character, so write the head word where the
tail word belongs. Applied, rebuilt, ran the micro corpus — **green**.

The corpus's only whole-string append fixture,
`an_in_place_append_takes_a_whole_string`, appends a byte and the five-byte
`"-mid-"`. Five bytes take the four-byte pair, which copies `[0,4)` and `[1,5)`
and is correct either way. Nothing in the corpus reached the eight-byte arm at
all, so a mutation that corrupts every append of nine to sixteen bytes had
nothing to fail.

`an_in_place_append_crosses_its_width_arms` appends 1, 3, 5, 8, 9, 12, 16, 17
and 24 bytes, one on each side of every boundary, with a byte through the same
call site so the byte arm keeps answering. Under the mutation it reads
**9,602 656,002** where it should read **9,602 697,194**: the length is right
and the bytes are wrong, which is the whole point of summing them. Restored,
green. Ratchet row `short_append`.

**The general lesson is about coverage, not this fixture.** A boundary that no
existing test crosses is a boundary the mutation corpus cannot defend, and the
way to find that out is to write the mutation FIRST and watch what it does to a
green suite. A mutation that passes is not a bad mutation; it is a report.

### CI's rows, and four predictions that came back exact

The work vein could not be regenerated here — this container is glibc
2.39-0ubuntu8.7 against the golden's 8.8 and `host_gate.sh` refuses. These are
CI's, and each of the four moved rows lands on the container's delta applied to
the previous golden, to the instruction:

    work_encodebench   5,297,521,213 -> 5,241,342,413   -56,178,800   -1.0605%
    work_livebench     5,287,371,493 -> 5,231,192,693   -56,178,800   -1.0625%
    work_oneshot          27,669,661 ->    27,529,214      -140,447   -0.5076%
    work_widebench        57,201,345 ->    57,105,366       -95,979   -0.1678%

The other nine do not move. `emitted` and `machine code` agreed with the goldens
regenerated here without a round of their own, which is what a deterministic
vein should do.

**A CHIP THIS VEIN HAD NOT SEEN.** CI drew `family0x19-model0x11` — Zen 4, whose
row was removed unmeasured on 2026-09-01 rather than carried — found no row on
the emptied table, refused, and counted:

    compile_sample cpu="cpu family 0x19 model 0x11" sha=... row=41378194

The new series opens at **41,378,194** against the old series' 41,379,381. When
only that reading existed the difference was NOT attributable and this entry said
so: a different binary AND a different chip, which is the pair the per-silicon
key exists to stop anyone separating from one number. It recorded a prediction —
that the next chip to refuse would read 41,378,194 too — and the very next round
settled it, because the chip CI drew was **family0x19-model0x1**, the Zen 3 that
had counted the old series:

    family0x19-model0x11   41,378,194   Zen 4, new binary
    family0x19-model0x1    41,378,194   Zen 3, new binary
    family0x19-model0x1    41,379,381   Zen 3, OLD binary

So two things are now settled that one reading could not settle. The two chips
agree to the instruction, as every pair has on every binary so far. And the same
chip on both binaries makes the move attributable: **compile_instructions falls
1,187** on this change, with the silicon held fixed. `kanso check lib/json`
emits nothing, so the ladder cannot run during the measurement; what moved is
layout, for the twelfth time on this vein.

The general point is the one the table was built for. A single refusal on a
fresh chip carries a binary change and a silicon change in one number and can
attribute neither; a second refusal on a chip already in the record separates
them for free. That is worth waiting a round for, and it is the answer to what
the per-chip key BUYS — the cost side (one CI round per chip per binary) has
been measured for weeks and sits in design/pending-gavels.md, and this is the
first time the benefit has been written down with a number beside it.

**WELFARE 74.15975334184456 -> 74.18425551910869**, a rise of 0.02450218,
`--set` with its reason. The page's `compile.compile_instructions` span moves
with the golden; `golden_prose` reads 0 drifted.

## 2026-09-05 (seventeenth) — the compile vein the ladder moved, and eight calls that were prose

Two things, both found by the macos/arm job going red on the entry above while
every runtime counter agreed.

### The vein the sweeps do not read

`bench/compile_golden.txt` and `bench/compile_golden_modules.txt` are read by
`tests/compile_cost.rs` and by nothing else. `all_counters.sh` walks the eleven
runtime veins and `all_compile.sh` walks the six compile GATES; this pair is a
spec, so neither sweep touches it, and the append ladder moved it:

    recursion    lines 890 -> 944   branches 55 -> 59
    dispatch     lines 886 -> 940   branches 54 -> 58
    guards       lines 883 -> 937   branches 55 -> 59
    records      lines 934 -> 988   branches 58 -> 62
    build_block  lines 859 -> 913   branches 50 -> 54
    module       lines 4,970 -> 5,024   branches 420 -> 424

The same 54 lines and the same four conditional branches in every sample,
including the three that never append a byte: the emitter writes the ladder into
each module's prelude and the linker drops it where nothing calls it. That is
the same shape `emitted_lines` moved by, measured on a different corpus. `rounds`
and `visits` do not move, and `defines` does not either — the ladder is blocks
inside a function that already existed.

Summed the way the trend gate reads them, `lines` lands on 4,722 and `branches`
on 292 across the five samples; `module_lines` lands on 5,024 and
`module_branches` on 424. All four rise, and they are the price of the append
ladder's 1.06% off encodebench in the entry above — the ladder is written once
per module whether the module appends anything or not.

The gap is worth naming because CLAUDE.md already warns that the counters sweep
does not read the compile veins, and this is a third set that neither sweep
reads. A run of `cargo test --release` catches it. CI did, a round late, on the
one job whose meta targets run.

### Eight calls that were never emitted

Both counters read `calls` as a substring search over the whole IR text —
`grep -c 'call '` in `scripts/gates/emitted_code.sh`, `ir.matches(" call ")` in
`tests/compile_cost.rs` — and the prelude is commented. "a call into glibc's
memcpy", "a real call on every `if` condition and constructor", "each paying a
call into the runtime and a second call into the": the word appears eight times
in comment lines and nine times as an occurrence, because one of those lines
uses it twice.

I found this by rewording one comment and watching `calls` move by one with the
emitted code byte-identical. A counter that moves when prose moves cannot say
what it is there to say, so both counters drop comment lines before counting:

    emitted (decoder)   calls 1,835 -> 1,828     (-7: one prose line gained
                                                  by the ladder, eight dropped)
    emitted (each of
    the eleven others)  calls -7, e.g. encodebench 1,694 -> 1,687
    compile samples     calls -8, e.g. recursion 48 -> 40, module 769 -> 761

The two columns fall by different amounts on the same change because the gate
counts LINES and the spec counts OCCURRENCES, and one comment line carries the
word twice. No emitted instruction changed; `defines`, `branches` and `lines`
are untouched by the fix, since those three already anchor at the start of a
line and never could see a comment.

`tests/the_emitted_call_counter_ignores_comments.rs` pins it. The spec lifts the
stripper and the counting pipeline out of the gate script rather than restating
them, builds a two-comment one-call module, and asserts `calls=1`. Watched red
with the stripper replaced by `cat`: it read `calls=3`, the one real call plus
two comments. Restored, green.

The ratchet corpus has a row for a counter that goes blind. This is the other
direction — a counter reading high on text that is not code — and the eight it
was reading are constant, so no gate ever fired. It came out only because a
change touched the prose beside the code.

## 2026-09-05 (eighteenth) — a check that answered the same question every iteration

`k_beat_iter` is 3.08% of livebench and **28.65% of escapebench**, at about
thirty instructions a rewind. Eight of those thirty were one comparison,
answered once and asked forever.

### What the check is, and what it can see

A beat loop rewinds the arena to its entry mark between iterations. The rewind
restores two words from the mark and then asserted that they still met the end
of the mark's block:

    k_arena = m->ptr;
    k_arena_left = m->left;
    if (k_blocks && k_arena + k_arena_left != end(k_blocks)) k_die(...);

A mark whose pointer and remaining count disagree with its block hands out
memory past that block's end, and the damage surfaces later in an unrelated
allocation — a glibc abort on linux and silence on macOS — so the check earns
its place. What it does not earn is its position. The predicate reads
`m->ptr`, `m->left` and `m->block`, and the block test three lines above has
already established `k_blocks == m->block`; every term belongs to the mark. A
mark is written in `k_beat_push` and never written again, so the rewind was
re-deriving an answer fixed at the push.

The check now sits in `k_beat_push`, immediately after the mark is written. It
catches the same bad marks — the predicate is identical, over the same three
values — once per loop entry instead of once per iteration, and a mark already
broken when the push takes it is now reported at the push rather than one
rewind later.

### The comment said one comparison; on x86-64 it is eight instructions

    4f8e:  test %rdx,%rdx        4f9d:  add  $0x10,%rax
    4f91:  je   4fa6             4fa1:  cmp  %rax,%rsi
    4f93:  add  %rax,%rsi        4fa4:  jne  4fa7
    4f96:  mov  0x8(%rdx),%rax
    4f9a:  add  %rdx,%rax

`k_beat_iter` was thirty instructions on the taken path and is twenty-two now.
The block end is a load, an add and an add before the comparison can happen,
and the null guard is a test and a branch ahead of that. A comment that
under-reports a hot cost by eight times is the kind of thing only a
disassembly settles, which is why this entry carries one.

### Thirteen benchmarks under callgrind, both binaries run from one directory

    escapebench    130,170,260 ->   120,585,257   -9,585,003   -7.3634%
    basket          39,728,726 ->    38,816,622     -912,104   -2.2958%
    livebench    5,231,192,305 -> 5,208,574,712  -22,617,593   -0.4324%
    encodebench  5,241,342,073 -> 5,218,724,482  -22,617,591   -0.4315%
    oneshot         27,528,874 ->    27,472,340      -56,534   -0.2054%
    deepbench      714,675,378 ->   715,116,883     +441,505   +0.0618%

**deepbench rises, and the reason is the whole shape of the change.** The check
did not vanish; it moved to the push, so a program pays for it once per beat
ENTRY rather than once per beat ITERATION, and a program that enters more beats
than it iterates comes out behind. 441,505 is 55,188 pushes at eight
instructions, which is deepbench's excess of entries over rewinds. Every other
row falls or holds, and encodebench and livebench fall by the same 22,617,59x —
the same runtime linked into a program and into that program built against the
shipped library. Nothing in `bench/cost_golden*.txt` moves: the counters count
allocator events, and no allocator event changed.

Those are container readings and sit a constant 340 below CI's; the goldens
carry the same deltas. Priced by key: `work_deepbench` lands on **715,116,179**,
the one work row that rises, and `text` lands on **1,121,352**, which is 192
bytes of machine code across twelve programs.

**WELFARE 74.18425551910869 -> 74.21933255238363**, `--set` with its reason.
The trade the objective accepted is stated above and not hidden in the sum.

### The pin, and why the obvious one is not available

A diagnostic that never fires and a diagnostic that was deleted look identical
from outside, so moving one needs a spec that the moved copy still runs. There
is no kanso program that can corrupt a mark, so the corpus cannot reach the
failing case from the front door. The ratchet can: `beat_mark` inverts the
comparison, which makes it fail at every push instead of never, and every
program with a beat loop dies at its first one. Watched: escapebench, rebuilt
under the mutation, prints `error[runtime]: a beat mark and the arena disagree
about the room that is left` and produces nothing. Restored, green.

The fold rewrite that led here is DECLINED and recorded on the compiler page
as item 13 of §06. The escape path's per-byte closure is real — 712,277,200
instructions, 13.59% of the frozen encode board — and removing it by walking
the string by index costs livebench 5,231,282,203 -> 5,388,806,908, +3.01%.
Every allocation counter holds and `beat_iters` goes 5,032,401 -> 16,691,201,
one new beat per byte: the index walk satisfies the beat analysis where
`fold`'s inner loop did not. Removing the closure saved 279M and the beat cost
436M. That is where the thirty instructions came to be counted at all.

### CI's rows, and thirteen predictions that came back exact

Every one of the thirteen work rows landed on the container's delta applied to
the previous golden, to the instruction — the whole file, not a subset. `emitted`,
`machine code` and all eleven counter veins agreed with the goldens regenerated
here without a round of their own.

CI drew **family0x19-model0x1** on the emptied chip table, which is the chip
that counted the previous binary:

    family0x19-model0x1   41,378,194   old binary
    family0x19-model0x1   41,377,644   this binary

Same silicon, both binaries, so the fall of **550** is attributable without
waiting for a second chip. `kanso check lib/json` emits nothing and runs no
beat loop, so what moved is layout, for the thirteenth time on this vein.

Four more rounds drew chips with no row on the new series, and all four read
**41,377,644** as well:

    family0x19-model0x1     AMD Zen 3
    family0x6-model0xcf     Intel Emerald Rapids
    family0x1a-model0x2     AMD Zen 5
    family0x19-model0x11    AMD Zen 4
    family0x6-model0xad     Intel Granite Rapids

Five keys, two vendors, five CI rounds, one number. Two of them are silicon
this vein had never seen: AMD family 0x1a, where every AMD key before it was
0x19, and Intel model 0xad. Both agreed to the instruction on generations the
pool had not produced before, which is the table working, and five rounds is
what it cost — the number design/pending-gavels.md now carries on the cost
side.
**WELFARE 74.21933255238363 -> 74.21937425493788**, a second `--set` with the
compile row's own reason.

### The beat that looked profitless, and the scale that said otherwise

The entry's own finding — a rewind that reclaims nothing — was chased to the
group that owns it and then declined. `d_escapebench/more_4` holds 1,206,000 of
escapebench's 1,206,002 rewinds, and that is `filled`; a probe says 99.75% of
them find `k_arena` exactly where the mark left it. Refusing that group its beat
by hand, beside the `PureLoop` test `beat.rs` already has:

    span=400   n=3000   -33,263,355 instructions (-27.58%), every memory
                        counter byte-identical
    span=10000 n=300    every memory counter byte-identical
    span=60000 n=60     arena_blocks 1 -> 2, peak 1,048,576 -> 3,145,744

**Two scales agreed that the bracket bought nothing and the third disagreed.**
The first two were measuring a workload that fits in one arena block either
way; at the third the growing accumulator's superseded buffers exceed a block
and the rewind is the only thing holding the peak down. The bracket is doing
its job. Recorded on the page as item 14 of §06, with the three runtime
attempts that failed beside it, so none of them is tried again from the profile
alone.

What survives is about the corpus. escapebench pins this bracket's COST on
every run and its BENEFIT on none, so a change deleting it would have read as a
27.6% win with every memory counter flat — which is the failure its own README
exists to prevent, one level in. Whether to raise its size is Clay's, and not
free: `escape_instructions` is a welfare term and a bigger benchmark is a
slower job.

`bench/text_golden.txt` moves on all twelve rows, most of them down —
escapebench -32, readbench -16, jsonbench +16, pendbench +64. The check's eight
instructions are gone from one place and present in another, and where the
linker puts what is left is not something this change chose.

---

## 2026-09-05 (nineteenth) — the clean run in front of the first escape, declined four ways

A fresh callgrind read of the merged corpus, looking for what sits on top now
that the append ladder, `k_rec`'s copy and the beat check have all landed. On
encodebench and its live twin the top of the board is the escape path, and it
holds two entries:

    d_json/encode_onto_2'2   832,741,200   15.99%
    w_klam17                 712,277,200   13.67%   11,658,800 calls, 61.1 each
    d_json/escape_clean_4    680,992,000   13.07%    4,190,000 calls, 162.5 each
    render_ryu               467,153,200    8.97%      849,200 calls, 550 each
    k_b_find2_below_raw      271,083,600    5.20%    4,190,000 calls, 64.7 each

`w_klam17` is the escape fold's lambda, and item 13 of the page already carries
why it cannot be reduced away. What it does not carry is that the fold walks
bytes it has no business walking.

### The run the scan already proved

`escape_onto` scans a string once with `find2_below` for a quote, a backslash
or a control byte, and `escape_clean` takes the whole-string copy when the scan
found nothing. When it did find something the fold runs over **every byte of
the string**, including the bytes in front of the first escape — bytes the scan
has just proved need nothing.

Counted on `bench/large.json`, which is the board:

    strings                10,475
    with an escape          1,773   16.93%
    bytes in those         29,147
      clean prefix          7,690   26.38%
      from the first esc   21,457   73.62%
    mean prefix              4.34 bytes
    mean escaping string    16.44 bytes

16.93% and 16.44 reproduce the profile's 709,201 fold entries over 4,190,000
encodes and its 11,658,800 byte steps to the digit, so the model and the
measurement are reading the same thing. **A quarter of the fold's work is a run
already known to need nothing.**

### It works, and the objective declines it

Appending that run in one copy and folding a slice of the rest, four ways:

                              livebench    oneshot   compile row   welfare
    two helpers, guarded run   -3.0621%   -1.4450%      +390,025    -0.02
    one helper, guarded run    -3.0621%   -1.4450%      +268,455    -0.01
    no helper, no guard        -2.7457%   -1.2955%      +172,457    -0.00
    no helper, guarded run     -3.0842%   -1.4555%      +213,359    -0.01

The best runtime is 5,208,664,610 instructions to 5,048,017,249 on livebench
and 27,562,238 to 27,161,080 on oneshot, with the frozen encodebench twin
byte-identical and both checksums unchanged at 74072800. The compile column is
`kanso check lib/json` measured by `scripts/compile_row_probe.sh` against the
same baseline binary each time — 41,879,349 — and projected onto the golden;
the runtime columns are container callgrind, whose deltas the log has repeatedly
found to match CI's to the instruction.

**The exchange rate is legible in the last two columns.** The guarded run costs
40,902 more compile instructions than the unguarded one and buys 17,635,200
more runtime instructions, a ratio of 431 to 1, and the index prefers the
compile side. Two satiations produce that: compile cost satiates at 0.5 and
sits near its baseline where the curve is steep, while `live_instructions`
satiates at 2.0 and entered the corpus that morning as a granted baseline at
its dimension's standing, where *r* is already 5.95 and a three per cent fall
moves the term score by 0.0058.

`compile_allocs` also rises and this host cannot measure it, so the true
readings are at or below the ones above.

### What paid, and what it says

The three-helper shape's counters, for the record — the other shapes move the
same way and were not swept:

    live      allocs 14,819,276 -> 16,104,076   +8.67%
              alloc_bytes 817,803,648 -> 863,471,648
              sh_bytes 100,755,936 -> 131,591,136
              append_fast 42,334,257 -> 39,833,857
    compile   front_end_visits on lib/json 17,169 -> 17,460   +1.69%
    emitted   jsonbench defines 178 -> 180, lines 12,188 -> 12,284
    text      oneshot 115,842 -> 116,290

Two slices per escaping string is where the allocations come from, and
`append_fast` falling is the fold doing less.

**This is an argument about the weights rather than about the change**, and it
is filed as one in design/pending-gavels.md. A library change that buys runtime
by growing the library charges the least satiated term in the model to feed one
of the most satiated, and a benchmark that enters at its dimension's standing
enters satiated — so the objective can never pay much for improving it. The
code is reverted and item 15 of docs/compiler.html section 06 carries the
decline so the next reading of this profile finds it.

**This closes the thread the livebench entry left open**, and does not edit it.
That entry — 2026-09-05, "Why a benchmark that duplicates one already in the
corpus" — ends "It is being held for this benchmark rather than shipped against
the old corpus", with the change priced at +0.0039 against stale compile veins
and −0.02 once they were regenerated. The benchmark arrived, the change was
re-measured against it, and the answer did not change. A reader who reaches that
sentence first should read this entry next.

### The vein that would have watched it, and could not

The sweep turned up a real gap. `bench/text_golden.txt` had twelve rows and
`bench/emitted_golden_others.txt` eleven, and livebench was in neither: the one
benchmark that watches the shipped library's encode path was built, instruction
counted and weighed for a day with nothing counting its machine code or what
the compiler wrote for it.

`scripts/gates/machine_code.sh` already carries the note that "scanbench and
digestbench joined the corpus after this gate was written and nobody extended
the list". livebench was the third, on two gates at once, which is what turns a
convention into a spec. `tests/every_benchmark_is_in_the_objective.rs` gains
four checks that derive each gate's list from the gate itself and its rows from
the golden, and both new coverage checks were watched red naming `livebench`
before the lists were fixed. The rows enter as measurements rather than moves:

    livebench text=116290
    livebench defines=179 calls=1846 branches=1176 lines=12239

---

## 2026-09-05 (twentieth) — ryū's digits come out in pairs, and the split that said where to look

The corpus profile after the escape thread put `render_ryu` at 467,153,200
instructions, **8.95%** of the encode board, over 849,200 renders — 550 each.
The page had been carrying that line as a dragonbox candidate with an estimated
margin and no measurement of what was inside it.

### The split, by noinline

`ryu_d2d` is static with one caller and LTO folds it in, so the profile could
not tell the digit core from the `%g` format layer. Marking it `noinline` for
one build separates them exactly:

    ryu_d2d      396,958,400   467.5 per render   85%
    render_ryu    85,485,600   100.7 per render   15%

**The core is 85% of the line.** That settles the dragonbox estimate the page
had been guessing at — its margin applies to nearly all of `render_ryu` rather
than part of it — and it says where to look first, which is the core's own
tail.

### The tail

The digits came out one at a time:

    while (output > 0) { tmp[n++] = '0' + output % 10; output /= 10; }
    for (int a = 0; a < n; a++) dig[a] = tmp[n - 1 - a];

One 64-bit division per digit into a scratch buffer, then a reversal. The
replacement takes the length from a ladder and writes pairs out of a 100-entry
table straight into `dig`, which is what reference ryū does and this port had
not carried across. Four rows fall and NINE HOLD EXACTLY STILL:

    encodebench   5,218,724,822 -> 5,150,924,022   -67,800,800   -1.2992%
    livebench     5,208,575,100 -> 5,140,774,300   -67,800,800   -1.3017%
    oneshot          27,472,680 ->     27,303,178      -169,502   -0.6170%
    widebench        57,105,059 ->     57,059,796       -45,263   -0.0793%

encodebench and livebench fall by the SAME 67,800,800 — the same program on the
same input, one against a frozen library and one against the shipped one — and
the nine that hold render no float at all. That they are exactly still is the
check that the four falls are the change rather than the weather.

No allocation counter moves, no emitted line moves, and every `.text` row rises
by exactly **112 bytes**. Welfare 74.2194 -> 74.2493, banked with `--set`.

### The domain the old loop had and the new one did not

The harness caught a real narrowing before CI could. The old loop was total:
handed any `uint64_t` it wrote that many digits. The new one walks DOWN from
the length the ladder gives it, so a ladder that stops at seventeen digits and
is handed an eighteen-digit value writes at a NEGATIVE INDEX. A double's
shortest form never needs more than seventeen, so nothing in `ryu_d2d` can
reach it — but a helper that writes out of bounds on an input it merely
believes it cannot get is the shape this project's rules exist to stop. Three
more rungs make it total, on a path nothing takes.

Those three rungs are 96 bytes of unreachable code and they moved two counts
that cannot execute them: encodebench by 802,800 and widebench by 499,900
against the shape without them. That is the linker putting everything else
somewhere else, and it is the same size of layout effect this vein recorded on
deepbench a fortnight ago.

### The harness

`tests/the_shortest_digits_come_out_in_pairs.rs` cuts the table, the ladder and
the extraction block out of `src/runtime.c`, compiles them beside a `snprintf`
reference, and sweeps **120,000,057** values: every value to eight digits one
at a time, every power of ten and its neighbours to the top of the range, and
twenty million from a fixed seed across the whole of `uint64_t`. Zero
disagreements. Watched red by moving one rung — `v < 1000` returning 4 — which
reports the first disagreement at 100 and 903 in total.

The text is lifted rather than copied, for the reason the harness ethos gives:
a copy goes green on code nobody ships. `scripts/render_differential` covers 86
values and `bench/large.json` 2,123, which is where the confidence for a change
to a precision kernel would otherwise have stopped.

### What this says about where to work

The declined library change in the entry above cost 172,457 to 390,025 compile
instructions and fell by the objective. This one costs **269** — inside the
binary-to-binary drift band `scripts/compile_row_probe.sh` documents — because
`src/runtime.c` is compiled by clang when a program is built, not by the
compiler when `kanso check lib/json` runs. A runtime improvement and a library
improvement of the same runtime size are not the same price, and the difference
is three orders of magnitude.

---

## 2026-09-05 (twenty-first) — the literal's slot was behind a call

Reading down the encode walker's callees for the first time turned up a name
nobody had looked at: `k_str_lit`, **2,500,000 calls** on encodebench for
45,000,672 instructions.

A string literal is the same value on every evaluation, so the emitter builds it
once into permanent storage and a slot hands it back thereafter — the page has
said so for weeks and the runtime comment says so above the function. What
neither said is that the handing back is a CALL across the module line. After
the first evaluation the whole body is: load the slot's tag, compare it to
`K_STR`, load two words, return. Eighteen instructions apiece, and most of them
are the call rather than the work.

### The door

The emitter already writes `alwaysinline` shims for exactly this shape —
`k_b_find2_fast`, `k_b_length_fast`, the slice and utf8 twins — each an inline
fast path over a runtime call kept for the cases the fast path cannot serve.
The literal gets the same treatment: read the tag, return the slot if it is
built, call the runtime if it is not.

TWELVE ROWS FALL AND ONE RISES BY TWO INSTRUCTIONS:

    widebench       57,059,796 ->  56,003,869   -1,055,927   -1.8506%
    livebench    5,140,774,300 -> 5,084,687,524 -56,086,776   -1.0910%
    encodebench  5,150,924,022 -> 5,094,896,927 -56,027,095   -1.0877%
    deepbench      715,116,179 ->  707,820,204   -7,295,975   -1.0202%
    oneshot         27,303,178 ->   27,103,536     -199,642   -0.7312%
    basket          38,817,037 ->   38,562,384     -254,653   -0.6560%
    jsonbench    1,745,056,955 -> 1,736,104,978  -8,951,977   -0.5130%
    indexbench       4,692,386 ->    4,692,194         -192   -0.0041%
    pendbench      666,111,554 ->  666,093,569      -17,985   -0.0027%
    scanbench    1,395,728,266 -> 1,395,689,898     -38,368   -0.0027%
    digestbench     77,290,402 ->   77,290,245         -157   -0.0002%
    readbench    2,038,396,069 -> 2,038,392,076       -3,993   -0.0002%
    escapebench    120,585,597 ->  120,585,599           +2   +0.0000%

**This is the widest single change in the vein's history.** Every other one has
reached the two or three benchmarks that exercise one path; a string literal is
in every program, so the fall tracks how many literals a program evaluates and
how often. widebench leads because its frozen json copy names its keys inline.
escapebench barely has literals, and its two instructions are layout.

No allocation counter moves — the door allocates nothing the call did not.
Welfare 74.2493 -> 74.2879, banked with `--set`.

### What it costs

Every `.text` row grows, from **64 bytes on escapebench to 2,304 on scanbench**,
in proportion to literal sites. That is the door inlined at each one, and it is
the trade the objective priced: 13,888 bytes across the corpus against twelve
falling rows, where `k_str_n`'s inlining a week ago cost 34,320 for a narrower
win.

The compile row moved **-584** on `kanso check lib/json`, inside the
binary-to-binary drift band, because this is a `src/codegen.rs` change and the
compile row does not compile emitted IR.

### The vein all_compile.sh cannot see

`tests/compile_cost.rs` went red and the six-gate sweep did not: the module and
micro compile goldens are asserted by that test rather than by a gate, so
`sh scripts/gates/all_compile.sh` reports nothing about them. Both moved by
exactly the door — `defines` +1, `calls` +1, `branches` +1, and eleven lines on
the module against twelve on each micro — with `rounds` and `visits` unchanged,
which is the front end doing no more work for the extra emitted lines.

Anyone following CLAUDE.md's compile sweep alone would have pushed a red branch.
The sweep's own paragraph already warns that it reads the runtime cost goldens
and that the compile gates are separate; this adds that two compile goldens are
behind a `cargo test` rather than behind either. Regenerate them with
`KANSO_REGEN_COMPILE_GOLDEN=1 cargo test --release --test compile_cost`.

## 2026-09-05 (twenty-second) — the compile row rose 1,120 on CI and fell 315 here, and the commit that recorded it was wrong

Searched the live log and the archive before filing. The archive's 2026-09-03
entry "CI read the compile row, and the fall is not the compiler" records this
mechanism already and says it plainly: "The container moves that term the other
way on the same source change." What is new is only that the disagreement now
has both halves measured for one pair of binaries, and that a commit message on
this branch got it wrong before the second half was read.

**The correction.** Commit 4fc9d8d0 said the +1,120 CI counted was the two
source changes on this branch costing the front end 1,120 instructions to
compile, on the ground that the twelfth series' Zen 4 sitting read the same
41,377,644 as its Zen 3 sitting and so carried no offset of its own. That
argument is about the ABSOLUTE agreement between two chips on one binary, and
it does not license reading a delta across two binaries off one chip. The
differential below says the opposite sign.

**Both binaries, one host, everything else held.** Built here from
`origin/main` (e02ffe61) and from this branch's head, probed with
`scripts/compile_row_probe.sh`, which prints the whole-process row and the
compiler's own frame separately:

    main   sha fe6295808553   row 42,344,513   program 41,879,349   maps 112,586
    head   sha fc014d0fba0e   row 42,344,192   program 41,879,034   maps 112,580

    row -321   program -315   maps -6

So the compiler does 315 fewer instructions to check `lib/json` with ryū's pair
table and the literal's door in it than without, and the whole-process row
follows the frame almost exactly. This host is `family0x6-model0xcf`, Emerald
Rapids, on a toolchain `scripts/gates/host_gate.sh` refuses to compare against
CI's — so the absolute numbers here are not CI's and are not meant to be. The
DIFFERENCE between two builds on one machine does not care whose rustc built
them, which is why the differential is the readable thing and the absolutes are
not.

**Two things differ between the two readings and neither can be held fixed.**
CI drew Zen 4 on CI's glibc; this is Emerald Rapids on the container's. glibc
resolves memcpy, memcmp and strlen by ifunc at load, which is the whole reason
the row is keyed by silicon at all, and the container's toolchain is a second
difference on top. Nothing here separates them, and 1,435 is not a residual
worth a theory: the probe's own header records the row moving up to 3,963
between binaries whose sources differ only in code or data no execution
reaches, with the frame moving 1,028 of it. +1,120 sits inside that band, and
so does -321.

**What the record should say.** The two changes cost the front end nothing
measurable. The +1,120 is the thirteenth series' first sitting and is what the
gate will hold Zen 4 to; it is not an attribution. DONE.

**OPEN, small.** The twelfth series had five chips agree to the instruction on
one binary, which is the strongest evidence this vein has produced that the key
is doing less work than its header feared. If a second chip sits the thirteenth
series and reads 41,378,764 as well, that is another unanimity and the pool's
absolute agreement is holding while its deltas are not — which is a sharper
statement of what the key buys than the header currently makes. Worth one line
in the header when the second row lands, and not before.

## 2026-09-05 (twenty-third) — the chip CAN be held fixed, and the entry above said it could not

Searched the live log and the archive before filing. This corrects the entry
immediately above, which is one round old and was wrong about what the record
could support. Nothing else in either file is affected.

**What the entry above said.** "Two things differ between the two readings and
neither can be held fixed. CI drew Zen 4 on CI's glibc; this is Emerald Rapids
on the container's." True when written. The next CI round drew
`family0x6-model0xcf` — Emerald Rapids, the container's own family and model —
and the twelfth series has a sitting for that key. So CI has now counted the
same chip on both binaries, on one toolchain:

    41,377,644   twelfth series, family0x6-model0xcf
    41,378,764   thirteenth series, family0x6-model0xcf   +1,120

The chip is held fixed and the toolchain is held fixed. The +1,120 is what the
front end costs on CI across this branch, and calling it unattributable was
premature by one round.

**The container still reads the other way, and the reason is narrower now.**
Same family and model here, and `kanso::main` inclusive falls 315. What differs
between the two is the toolchain the row is counted under, which is the one
thing `scripts/gates/host_gate.sh` refuses over. The row carries glibc's
allocator and its ifunc-resolved string routines, so a rise there on one libc
and a fall on another is the shape the archive's 2026-09-03 entry described.
The compiler's own frame falling while the row rises is that entry's finding
restated, and it is now on the same chip rather than across two.

**Both chips on the new binary agree to the instruction.** Zen 4 and Emerald
Rapids, two vendors, two independent CI builds, 41,378,764 twice. The twelfth
series managed five keys on one number; the thirteenth has two after two
rounds. The OPEN item in the entry above asked for exactly this and can be
closed: the pool's ABSOLUTE agreement within a binary is holding, and the
per-chip key is buying less than its header feared. DONE.

**And the build is byte-reproducible, which the header does not say.** Nothing
under `docs/` or `design/` is `include_str!`'d, there is no `build.rs`, and no
`env!` or git state reaches the binary; a forced rebuild of every `.rs` here
produced the same sha256 fc014d0fba0e. So a commit touching only docs, design
or bench cannot move this row. The table's header still offers "5,081
instructions from an edit the compiler cannot see" as its headline measurement
of the effect, and that reading predates the sorts and `setarch` of kanso#1234.
Left alone here rather than rewritten on one container's evidence — the header
already concedes "Nothing in them separated the change from the chip," and the
five-key twelfth series is the better argument. OPEN, and cheap: the next
person editing that header should price the 5,081 as chip, not as edit.

## 2026-09-05 (twenty-fourth) — seventeen counters moved and this branch had not named one of them

Searched the live log and the archive before filing. The archive's 2026-09-05
entries for this branch name the runtime rows that fell and the compile row
that rose; none of them names the emitted-code counters, and that is the gap
the trend gate found.

**And I read the job wrong for five rounds.** Three times I said every vein was
green except `compile instructions`. That was read off the cost-goldens job's
own vein summary — the `for vein in "emitted:success" ...` block — which lists
eighteen gates and DOES NOT LIST THE TREND GATE. The trend gate is step 10 of
that job, `nothing worsens unless something improves`, it runs `if: always()`,
and it had been failing since the first round of this branch. The compile row's
refusal masked it: the job was already red, so nothing drew attention to a
second red step. The vein summary is not the job.

**What moved, and the value each landed on.** The runtime side is in the
entries above — twelve work rows fell, encodebench 5,094,896,927 and livebench
5,084,687,524 among them. This is the other side of that trade, which nothing
had written down:

    branches                297     (bench/compile_golden.txt)
    calls                   202
    defines                 184
    lines                 4,780
    module_branches         425     (bench/compile_golden_modules.txt)
    module_calls            762
    module_defines           96
    module_lines          5,035
    emitted_branches      1,172     (bench/emitted_golden.txt)
    emitted_calls         1,829
    emitted_defines         179
    emitted_lines        12,199
    emitted_other_branches   8,768  (bench/emitted_golden_others.txt)
    emitted_other_calls     14,662
    emitted_other_defines    1,598
    emitted_other_lines     90,913
    text              1,134,488     (bench/text_golden.txt)
    compile_instructions 41,378,764 (bench/compile_instructions_golden.txt)
    work_escapebench    120,585,599 (bench/instructions_golden.txt)

**Why each moved.** All of the emitted-code movement is the literal's door.
`k_str_lit_fast` is one `define` per emitted program, carrying one branch (the
tag test) and one call (the slow path), and eleven lines. That is exactly the
shape of the deltas: +1 define, +1 branch, +1 call and +11 lines on the decoder
itself, and +11 of each across the twelve other programs, eleven of which emit
it. The micro and module compile goldens move for the same reason and no other.

`text` rises 13,136 bytes over thirteen benchmarks. That is the shim inlined at
every literal site plus ryū's 201-byte pair table and the length ladder beside
it — machine code bought deliberately, and the twelve falling work rows are
what it bought. The objective does not weigh machine-code size, which is a
question already with Clay in design/pending-gavels.md and not re-opened here.

`work_escapebench` rises 2 instructions on 120 million. escapebench has 47
defines and 118 calls and barely touches a string literal, so it gains the
shim's presence and almost none of its benefit. Two instructions is the
honest cost of that and is not worth a fix.

`compile_instructions` rises 1,120, which the two entries above this one argue
about at length and settle: the chip is held fixed on CI across both binaries,
so it is real, and the container's differential says the compiler's own frame
falls 315. DONE.

**The gate was right and the branch was silent.** Every one of these is a
regression by the gate's definition and every one is the price of a trade the
branch made on purpose. Saying so is the whole requirement — `movement is fine;
silence is not` — and five rounds of CI went by with the sentence unwritten.

## 2026-09-05 — gavel: one row, one value; and no term for machine-code size

Searched the live log and the archive before filing, as the gate requires. The
compile row's history is in this file (the thirteenth series, and the two
entries above it arguing about what +1,120 meant) and in the archive from
2026-08-31 onward; the machine-code-size question was filed by kanso#1247 and
carried its own search. Both entries leave design/pending-gavels.md with this
commit, and so does the residual-layout entry, which was marked ANSWERED on
2026-09-04 and had no business still sitting in a ledger of open questions.

**RULING ONE — the compile-instructions row pins ONE ROW AND ONE VALUE.** The
pinned pair goes. The per-chip key goes. There is no binary stamp. Every move
in the row is attributed to the change under test and handled by the ordinary
ratchet, like every other counter in the tree.

Consistency is verified by REPRODUCTION rather than by keying: the same build
on any runner must count the same number. A reproduction failure halts the
vein and is hunted to its source. That is not a new demand — it is what the
`/proc/self/maps` finding already did once, when the row's drift turned out to
be Rust's stack guard parsing the process map at startup and the answer was to
force the measurement to be consistent rather than to widen what the row would
accept. What is forbidden is the other two answers: pinning a second value, or
recording the difference as a mode.

**What the key produced before it was retired, because it is evidence for the
ruling rather than against it.** The thirteenth series ran three CI rounds and
drew three chips — `family0x19-model0x11`, `family0x6-model0xcf` and
`family0x19-model0x1`, AMD Zen 4, Intel Emerald Rapids and AMD Zen 3 — and all
three counted **41,378,764**, to the instruction. The twelfth series had five
keys on 41,377,644. That is eight within-binary sittings across two binaries
agreeing exactly and none disagreeing, on two vendors and four generations. A
key that never separates anything is a key that costs rounds and buys nothing,
and the rounds were real: five for the twelfth series, three for the
thirteenth, each one a red CI round spent adding a row rather than reviewing a
change.

The 508 the pair was built to hold is the one reading the record cannot
reproduce, and under this ruling it is a reproduction failure to hunt rather
than a mode to keep. Two candidate explanations are already ruled out: the
loader's directory order, and the build itself, which is byte-reproducible —
no `build.rs`, no `env!` or git state in the crate, nothing under `docs/` or
`design/` reaching the binary, and a forced rebuild of every `.rs` giving the
same sha256. `setarch -R` was applied to one of those two runs and not the
other, which is where the next reader should start.

**RULING TWO — no machine-code-size term in welfare.** `.text` stays in
`bench/text_golden.txt`, its own exact vein, pinned per benchmark and moved
deliberately. The objective does not weigh it, and
`tests/the_objective_does_not_weigh_machine_code_size.rs` now stands on a
ruling rather than on the absence of one. The question was live because the
inline doors buy runtime work with emitted bytes — 13,136 of them across
thirteen benchmarks in kanso#1260 alone — and the index could not see the
price. It still cannot, on purpose: an exact vein already refuses a silent
rise, and a term would need a satiation and a weight argued from cases that
do not exist yet.

What this costs is stated rather than hidden: a change that trades machine
code for speed scores as a pure win in the index and pays in a golden the
index cannot read. That is the same shape as every other counter outside the
model, and the model says so in its own preamble — what a model leaves out it
implicitly weights at zero.

---

## 2026-09-05 — one row, one value, built; and two ratchet rows that had never run

**DONE — the ruling in the entry above is implemented.**
`bench/compile_instructions_by_cpu.txt` and `scripts/gates/compile_ir_row.sh`
are deleted, with the three ratchet mutations and two specs that guarded the
keying: `an_unrecorded_chip_is_waved_through`,
`a_second_row_for_one_chip_is_read_by_nobody`, `a_pinned_pair_grows_into_a_band`,
`tests/a_compile_row_is_read_against_its_own_chip.rs` and
`tests/a_paired_chip_row_names_one_binary_twice.rs`.
`bench/compile_instructions_golden.txt` keeps its 41,378,764 unchanged and is
now the whole of the row.

The comparison moved back inside `scripts/gates/compile_instructions.sh`: read
the golden's single `compile_instructions=`, read the sitting, compare exactly.
The refusal names the two outcomes the ruling separates and says how each is
settled — a move attributed to the change under test goes through the ordinary
ratchet with a sentence in this file, and a disagreement between two runs of one
build is a reproduction failure that halts the vein and is hunted to its source.
The `compile_binary sha256` and `compile_sample cpu=... sha=... row=...` lines
stay and are named in the refusal as where a hunt starts, because they are what
tells the two apart: one sha counting two rows is the failure, two shas is the
change under test until the pair is built and both are read.

`scripts/gates/host_gate.sh` and `scripts/gates/dispatch.sh` both stay.
host_gate is the same-build check the ruling's "same build, any runner" clause
rests on, and its answer 3 — measure, print, fail — is why an unnamed toolchain
still leaves a sitting in the job log. dispatch is reporting only: the row is
not read against what it prints, and what it prints is the first question about
two numbers from one build.

`tests/the_compile_row_holds_one_value.rs` pins what is left: one value line,
bare digits, no chip table on disk, no gate naming one. Watched red three ways
before it was watched green — a second number appended to the line, a second
value line, and the table recreated with the gate reading it — each naming its
own defect. `scripts/ratchet/mutations/the_compile_row_pins_a_second_value.sh`
is the first of those as a ratchet mutation, because writing the disagreeing
number down beside the value it disagreed with is exactly the repair the ruling
forbids.

**FOUND, while removing one of them — two ratchet rows were written and never
joined the chain the harness runs.** `scripts/ratchet/ratchet.kso` binds each
row and then assembles them into `rows` through a chain of `text/concat`s.
`compile_ir_pair` was bound on the day the pair was ruled and never added to
that chain, so the guard against a third value beside a pinned pair never ran
once in its life; it was deleted here having proved nothing. `live_counters`
was bound on 2026-09-05 with livebench, and is live — it is added to the chain
in this change rather than deleted.

That is the same defect this repo keeps finding and a cheaper version of it.
kanso#1199 found two rows blind, kanso#1229 found three more and four checks
that could not fail — and every one of those was at least being run.
`tests/every_ratchet_row_is_run.rs` walks the concat chain from `rows` and
names any binding it does not reach. It was watched red against both orphans
before either was fixed.

**No counter moves.** Nothing in this change is `include_str!`'d into the
compiler: the gate scripts, `ratchet.kso`, the specs, `docs/compiler.html` and
the goldens' own headers are all outside `src/lib.rs`'s `lib/*.kso` and
`hako/*.kso` and outside `src/main.rs`'s `src/runtime.c`. The eleven runtime
cost goldens, the six compile gates and the welfare floor are untouched.

**OPEN — the 508 is still unexplained and is now a hunt rather than a mode.**
The record's two candidate explanations are already out: the loader's directory
order, and the build, which is byte-reproducible. `setarch -R` was applied to
one of those two runs and not the other. If it reappears, it stops this vein.

---

## 2026-09-05 — the bytes view inlines, and seventeen counters pay for it

**DONE — encodebench −3.4079%, livebench −3.4153%, oneshot −1.6997%.**
`bytes` is a string's byte view. `k_b_bytes` read two fields out of the KStr
and wrote a three-field header into the arena, and the arena bump was already
inline in `k_b_append_byte` — so the thirty instructions it cost per call were a
call, a tag ladder and a bump the emitter can write itself. `escape_onto_2`
calls it once per string encoded: 4.19M calls in encodebench.

`k_b_bytes_fast` is the shape the other doors use. One comparison does the whole
guard: a failure carries a tag that is not K_STR and goes to `%slow`, where the
C entry owns the message and the propagation exactly as before. The counting
path goes to `%slow` too, so `k_stat_sh_bytes` can never be dropped — the
arrangement `k_b_append_byte` uses, for the reason the presence-counter rule
gives.

Measured on one host, one pair of binaries, thirteen benchmarks under callgrind.
Nothing rises:

    encodebench   5,094,896,528 -> 4,921,267,313   -3.4079%
    livebench     5,084,687,077 -> 4,911,031,267   -3.4153%
    oneshot          27,103,137 ->    26,642,454   -1.6997%
    widebench        56,003,456 ->    55,731,441   -0.4857%
    jsonbench     1,736,104,565 -> 1,732,114,716   -0.2298%
    basket           38,561,971 ->    38,533,957   -0.0726%
    digestbench      77,289,846 ->    77,289,830        -16
    deepbench, escapebench, pendbench, indexbench, scanbench, readbench: flat

**ATTRIBUTED, and it beats its own arithmetic by four times.** The estimate was
~42M, about 0.83% of encodebench; the fall is 173M. `callgrind_annotate` over
the two profiles sums to −173,633,630 against the −173,629,215 the program
moved:

    k_b_bytes                       125,712,030 ->           0   -125,712,030
    d_encodebench/encode_onto_2'2   821,723,227 -> 683,026,827   -138,696,400
    d_encodebench/escape_clean_4    680,992,000 ->           0   -680,992,000
    d_encodebench/escape_onto_2               0 -> 771,766,800   +771,766,800

The call's own cost goes entirely: 125,712,030 over 4,190,000 calls is exactly
30.0 instructions a call, which is what the disassembly counted. The other 48M
is LLVM reshaping the escape path around the inlined body — the `escape_clean_4`
specialised clone folds away and its work lands in `escape_onto_2`. Read those
two rows together or neither makes sense.

**WHAT IT COSTS, and every counter is named here because the trend gate reads
this paragraph.** The shim's IR is written into every program the compiler
emits, so the emitted and machine-code veins all rise and none of it is front-end
work. `bench/compile_golden.txt`: branches 312, calls 207, defines 189, lines
5,015. `bench/compile_golden_modules.txt`: module_branches 428, module_calls
763, module_defines 97, module_lines 5,082. `bench/emitted_golden.txt`, the
decoder: emitted_branches 1,175, emitted_calls 1,830, emitted_defines 180,
emitted_lines 12,245. `bench/emitted_golden_others.txt`, the twelve programs
beside it: emitted_other_branches 9,981, emitted_other_calls 16,521,
emitted_other_defines 1,790, emitted_other_lines 103,721 — a uniform +3 / +1 /
+1 / +46 on each of the twelve, which is the shim written once per program.
`bench/text_golden.txt`: text 1,256,346, a rise of 3,360 bytes over thirteen
programs, encodebench 115,522 -> 116,242.

That trade is what the 2026-09-05 machine-code ruling said the objective cannot
see, and this is the first change to make it since. The exact veins see it;
welfare does not, and does not need to for the sum to be right.

**No allocation counter moves.** The shim claims the same 32 bytes from the same
arena `k_bytes_view` did, so all eleven runtime cost goldens are byte-identical.

**Also in this change: the page's 7.75% is dated.** `render_ryu` sat at 7.75% of
encode in three places on docs/compiler.html with no sitting attached. It is a
2026-09-02 reading. On 2026-09-05 the same line reads 9.20% before the pair
table and 8.09% after, and the pair table's own price at the function level is
480,160,400 instructions to 412,359,600, a fall of 14.12%. The two days are not
comparable — three encode changes landed between them and each moved the
denominator — so the page now names the day for each figure.

**CI'S SITTING, and the two hosts agree on every delta to the instruction.**
The container may measure the runtime vein but may not record it — its glibc is
2.39-0ubuntu8.7 against the golden's 2.39-0ubuntu8.8 — so `bench/instructions_
golden.txt` carries CI's rows. The absolute numbers differ from the container's
by a constant few hundred, which is the host offset this vein has always had;
the DELTAS are identical, benchmark by benchmark, which is the reproduction the
one-row-one-value ruling asks for:

    work_jsonbench    1,736,104,978 -> 1,732,115,129   -0.2298%
    work_encodebench  5,094,896,927 -> 4,921,267,712   -3.4079%
    work_oneshot         27,103,536 ->    26,642,853   -1.6997%
    work_basket          38,562,384 ->    38,534,370   -0.0726%
    work_widebench       56,003,869 ->    55,731,854   -0.4857%
    work_digestbench     77,290,245 ->    77,290,229        -16
    work_livebench    5,084,687,524 -> 4,911,031,714   -3.4153%
    work_deepbench 707,820,204, work_escapebench 120,585,599,
    work_pendbench 666,093,569, work_indexbench 4,692,194,
    work_scanbench 1,395,689,898, work_readbench 2,038,392,076: unmoved

**The compile row falls too: compile_instructions 41,377,380**, from 41,378,764,
a fall of 1,384. src/codegen.rs is the compiler's own source, so a shim added to
it moves the compiler's bytes and the layout under them; the front end does the
same work. This is the first move of that row under the one-row-one-value
ruling, and it is settled the way the ruling says: attributed to the change
under test, regenerated, and named here.

**welfare 74.2879 -> 74.3720**, and the floor is set in this change.

**OPEN — the escape reducer is 61.1 instructions a byte.** After this change
`w_klam17` is 14.47% of encode: 11,658,800 calls for 712,277,200 instructions.
Its compiled body carries eighteen copies of the append twin's `neg`/`cmp`/
`cmovs`/`jg` cap-sign shape, which is the twin's `%capneg`/`%isneg`/`%capa`
sequence inlined eighteen times. Deciding the sign once rather than per site is
the next thing to measure. The double forwarder is NOT the cost and does not
need measuring again: `klam17` never appears in the profile, LLVM collapsed both
frames into `w_klam17`.

---

## 2026-09-05 — the escape reducer's frame costs more than its work, and three repairs died first

**MEASURED, nothing built.** After the bytes view inlined, `w_klam17` — the
C-ABI wrapper `k_closure_lit` points at, called through a function pointer by
`list/fold` — is the second-largest line in the encode profile: 11,658,800
calls, 712,888,240 instructions, **61.15 a byte** of every string escaped.
callgrind `--dump-instr=yes` over 244 distinct instructions:

    11,658,800 x 17 = 198,199,600  (27.80%)  every call
    11,658,000 x 11 = 128,238,000  (17.99%)  every call but 800
     9,834,000 x 24 = 236,016,000  (33.11%)  the clean byte, 84.3% of calls
     9,833,200 x  5 =  49,166,000  ( 6.90%)  clean
    ~450,000 x ~40  =  68,800,000  ( 9.66%)  the escape cases, 3.9% of calls

A clean byte costs about fifty-three and the escape machinery is under a tenth
of the line.

**The twenty-eight fixed instructions are the frame**, disassembled and named
rather than inferred. Prologue: `push %rbp %r15 %r14 %r13 %r12 %rbx`,
`sub $0x18,%rsp`, two argument moves, the failure test, the byte test and the
range ladder. Epilogue: two stores, `add $0x18,%rsp`, six pops, `ret`. Six
callee-saved registers pushed and popped per input byte is 12 x 11,658,800 =
**139,905,600, 2.84% of encodebench**; with the stack slot and the ret,
174,882,000, 3.55%. The tag work is about eleven instructions.

The frame is the body's doing, and the program's three lambda wrappers give the
relationship in one disassembly pass: 692 instructions of body pushes six
registers, 68 pushes four, 53 pushes three. So the clean path pays for the
escape path's register pressure. That makes this a number for the `preserve_none`
thread, which wants LLVM 19 against CI's clang 18.1.3 and stays blocked. The
unblocked shape is the split `k_b_append_grow` already uses in C — a small hot
leaf tail-calling a cold half — and it is not measured.

**THREE REPAIRS DIED BEFORE ANY WAS BUILT**, which is the part worth keeping.
The append twin normalises a builder's capacity with a `neg`, a compare and a
conditional move, and that shape appears eighteen times in the reducer's machine
code.

1. I read eighteen as a per-byte cost. They are eighteen inlined CALL SITES and
   one byte takes one path, so the normalisation costs about three of the 61.
   The ceiling on removing it entirely was 0.7% of encodebench, not 5%.
2. It cannot be removed. The sign of `cap` records the storage regime — positive
   from the arena, negative malloc'd and permanent so a rewind cannot reach it.
   `k_buf_perm` writes `-cap`, `k_buf_cap` normalises with the same abs, and
   `k_buf_donate` refuses a negative one to the arena's shelf.
3. It cannot be hoisted to the fold's entry either, which was the surviving
   idea. `k_b_append_grow` writes `a->cap = marked` on the mutating path and
   picks the sign per call from the beat depth and whether the header survives
   the innermost mark, so a live accumulator's regime flips mid-fold on every
   growth. There is no invariant.

All three came out of reading two functions. The first is the one to remember:
a static instruction count is not a dynamic one, and I had the arithmetic
backwards before the disassembly corrected it.

**Also recorded, so nobody re-measures them.** The emitter writes two forwarders
for one lambda — `klam17` musttailing the body and `w_klam17` wrapping it — and
they cost nothing: `klam17` never appears in the profile, LLVM collapsed both
frames. And removing the escape fold's closure outright was measured earlier at
livebench +3.01%; the beat is why the closure form is cheaper.

---

## 2026-09-05 — the split is measured now, and it does nothing

**Closes the OPEN thread in "the escape reducer's frame costs more than its
work, and three repairs died first"**, which said the library-level split had
not been measured. It has, an hour later, and it is a no-op. This file is
append-only, so that entry stands as written; `docs/compiler.html` §54 is not,
and its one clause is corrected in the same change.

`lib/json/text.kso`'s `esc_byte` has seven literal arms differing only in the
byte after the backslash, each writing `text/append (text/append acc 92) c`.
Each inlines two copies of `k_b_append_mut_byte`, which is where the reducer's
694 instructions come from. The experiment gave the seven arms one shared body:

    fn esc_pair acc c
      text/append (text/append acc 92) c

Against merged main built beside it:

    work_livebench    4,911,031,267 -> 4,911,031,267   identical
    work_oneshot         26,642,454 ->    26,642,454   identical
    work_jsonbench    1,732,114,716 -> 1,732,114,716   identical
    work_encodebench  4,921,267,313 -> 4,921,267,313   identical, frozen copy
    .text on all four: identical to bench/text_golden.txt

LLVM inlines the shared body back into all seven arms and emits the same code
byte for byte. Reverted; nothing of it is in the tree.

**What it settles.** The inliner reassembles whatever the source separates, so
no source-level restructuring shrinks that body. Keeping the cold arms out of
line needs a way to SAY so, and kanso has no `noinline` for user code — which
makes it a language question rather than a performance patch. kanso#1186 did
the equivalent by hand in `k_b_append_grow`, where the C attribute was
available. So #290's two paths are the toolchain (`preserve_none`, LLVM 19
against CI's clang 18.1.3) and an emitter-level notion of a cold dispatch arm.

**A guard on the measurement, because it nearly went the other way.**
`w_klam17` is 692 instructions in encodebench and 786 in livebench. Those are
different binaries and the sizes are not comparable across them; reading the
786 as growth is the same cross-binary mistake this log recorded on the compile
row two days ago. The instruction counts and the `.text` sizes are what settled
it, and neither moved.

## 2026-09-05 — the entries pair goes on the stack

**DONE.** `k_b_entries` built each map entry's two fields in an arena block and
handed that block to `k_rec`. Since kanso#1258 `k_rec` copies its arguments into
the storage that follows the record header and keeps no reference to them, so
the block was written once, read once and dropped. It is a `KValue fields[2]`
now.

3,344,400 of those a run in encodebench — `k_rec`'s own comment records the
same count from the other side, since `k_b_entries` is where nearly all of them
come from. That is 22.55% of every allocation the benchmark makes.

    encode_allocs       14,830,625 -> 11,486,225   -3,344,400   -22.55%
    encode_alloc_bytes 819,040,144 -> 712,019,344 -107,020,800  -13.07%
    live_allocs         14,819,276 -> 11,474,876   -3,344,400   -22.57%
    live_alloc_bytes   817,803,648 -> 710,782,848 -107,020,800  -13.09%
    oneshot_allocs          75,822 ->     67,461       -8,361   -11.03%
    oneshot_alloc_bytes  4,349,484 ->  4,081,932     -267,552    -6.15%

Three instruction rows fall and ten hold exactly still:

    work_encodebench 4,921,267,712 -> 4,893,408,112  -27,859,600  -0.5661%
    work_livebench   4,911,031,714 -> 4,883,172,114  -27,859,600  -0.5673%
    work_oneshot        26,642,853 ->    26,573,204      -69,649  -0.2614%

encodebench and livebench fall by the same 27,859,600 because they run one
program over one input against two copies of the library. 8.33 instructions an
entry, which is the arena bump, the two stores into it and the reads back out.
The ten that hold never take a map through `entries`; jsonbench only decodes.
That they are exactly still is the check that the three falls are the change.

Welfare 74.37196081105081 -> 74.38489702262615, ratcheted in this PR.

**The `.text` vein separates two questions.** Four rows fall by exactly 112
bytes — encodebench, oneshot, widebench, livebench — and widebench is the pair
worth reading: its machine code shrinks and its work row does not move by an
instruction. Its program names `entries`, so the function is linked in and gets
smaller; its input never reaches the encoder, so nothing runs. jsonbench holds
in both veins because the linker drops the function entirely.

**The spec, and what the corpus already had.**
`tests/golden/mem/a_map_walk_builds_no_scratch_pair.kso` walks an eight-key map
a thousand times and pins `allocs=10010`. Restoring the block reads 18010 and
`alloc_bytes` 256,000 higher — watched red before it was watched green. The
corpus was not blind to this: `an_accumulator_loop_reclaims_its_garbage` calls
`entries` once on a one-key map and its `allocs` falls by exactly 1 here. One is
not a pin anybody would read, which is why the new fixture exists. No new
ratchet row — `mem_shapes` already proves this gate catches a mutation.

**On the instruction rows above.** The container is one glibc revision off the
runner and may not record this vein, so those were its own A/B deltas applied to
CI's landed values: it measured 4,921,267,372 -> 4,893,407,772 and 26,642,513 ->
26,572,864 on its own pair of builds, the fixed 340 being the exec-path offset
the golden's header warns about. CI's sitting is the record, and it landed on
all three predictions TO THE INSTRUCTION -- 4,893,408,112, 4,883,172,114 and
26,573,204 -- which is the fourth consecutive change the two hosts have agreed
exactly on. The `.text` rows matched too.

**ONE COUNTER WORSENS, and it is the compiler reading itself.**
`compile_instructions` lands on **41,378,464**, a rise of 1,084 from 41,377,380.
`src/runtime.c` is `include_str!`'d into the compiler by `src/main.rs`, so every
line added to it is a line the compiler carries and writes out for clang, and
this change added a nine-line comment to `k_b_entries` on top of the code. 1,084
instructions to explain why a pair goes on the stack, against 27,859,600 saved
at run time. The container cannot compare this vein -- it refuses on three of
the six compile gates -- so the value above is CI's, taken from the job log and
written in rather than measured here.

## 2026-09-05 — one block for a map's records, and three repairs that died first

**DONE.** `k_b_entries` carved each entry's record out of its own arena bump.
The records were already landing next to each other, because `k_alloc` only
moves a pointer, so it takes one block for all `n` and carves it. encodebench
builds 3,344,400 records over 1,104,400 calls, which means two bumps in every
three were bookkeeping for a block the one before them had already reserved.

    encode_allocs  11,486,225 -> 9,246,225   -2,240,000   -19.50%
    live_allocs    11,474,876 -> 9,234,876   -2,240,000   -19.52%
    oneshot_allocs     67,461 ->    61,861       -5,600    -8.30%

    work_encodebench 4,893,408,112 -> 4,739,440,912  -153,967,200  -3.1463%
    work_livebench   4,883,172,114 -> 4,729,204,914  -153,967,200  -3.1530%
    work_oneshot        26,573,204 ->    26,188,286      -384,918  -1.4485%

**`alloc_bytes` does not move by a byte, on any of the three.** That is the
whole claim made checkable: the records occupy the same arena, in the same
order, at the same stride. Only the arithmetic that reserved them moved. The mem
fixture says it a second way — `a_map_walk_builds_no_scratch_pair` reads `allocs`
10,010 -> 3,010, exactly eight records a walk becoming one block, with
`alloc_bytes` and `sh_rec` byte-identical.

68.7 instructions a bump saved. encodebench and livebench fall by the same
153,967,200 because they run one program over one input against two copies of
the library; jsonbench, escapebench and scanbench are byte-identical.

**Which line died.** Before, callgrind on encodebench read `k_rec` at
200,717,340 and `k_b_entries` at 110,939,200 — 311,656,540 between them. After,
`k_rec` does not appear in the profile at all and `k_b_entries` reads
157,636,000: the record construction moved inside it, so the arm that used to
call out per entry is a header write. 311,656,540 - 157,636,000 = 154,020,540
against the 153,967,200 the whole benchmark fell, and the 53,340 between them is
`k_rec` still serving the few sites that are not `entries`, now under the
threshold that prints.

Welfare 74.38489702262615 -> 74.45755497201294, ratcheted here.

**`.text` rises 288 bytes on the four rows that fell 112 in the entry above**,
and they are the same four for the same reason: the binaries that link
`entries` at all. The vein lands on **1,257,050** bytes across its thirteen
rows, from 1,255,898. The slot stride, the inline header write and an arm for a
failing field cost that. This vein is exact and its own — welfare weighs no
machine-code term, ruled today — so the rise is stated rather than defended.

**THREE REPAIRS DIED ON THE WAY, all built and measured against the entry
above.** The profile that found this one also named `k_b_find2_below_raw` at
271,083,600 over 4,190,000 calls — prologue and guards 14 apiece (21.6%), the
SSE loop only 409,600 iterations (2.0%) because 91% of calls never enter it, and
the scalar tail 203.5M (75.1%) over 18,354,000 byte-steps. The mean run is 5.9
bytes, so no wider pass reaches it, and both narrow repairs cost:

- the floor test in the byte domain, to kill the per-byte re-widening of a byte
  the compare above had already zero-extended: **+14,956,000, +0.306%**. clang's
  replacement shuffle costs more than the `movzbl` it removes.
- sinking the wide pass's three splats behind an `i + 16 <= len` test, the shape
  that worked at the utf-8 door in kanso#1246: **+10,101,600, +0.206%**.

The third was `k_rec`, 60 instructions a record and fully straight-line, of
which ten are five callee-saved push/pops — and `r12`/`r13` are touched only by
two cold blocks, one of them the one-time `getenv` that `always_inline`
`k_alloc` carries into every allocation site in the runtime. Outlining that into
a `noinline, cold` helper did not shrink the frame and split the rows:
encodebench -19,655 against **jsonbench +1,063,826 (+0.061%)**. A decode-side
rise for an encode-side rounding error; the objective would decline it.

**`compile_instructions` is CI's**, as the entry above says: this container
refuses three of the six compile gates.

**One question does NOT go to Clay, and the search is why.** kanso#1264 left
"an emitter-level notion of a cold dispatch arm" as a ledger item.
`design/log/compiler-log-archive.md`, "2026-09-02 (fourteenth) — that last
question was mine, not Clay's", already withdrew the identical filing three days
ago, citing pending-gavels.md's charter: an entry goes to him because it is
about the language a user meets, and how a dispatch group is emitted is not
something a user meets. An inferred cold arm is that same shape, so it stays
mine and #290's remaining path is implementation. A user-written `noinline`
would be different — that is new surface — but nothing measured yet says the
inference is insufficient, so there is nothing to ask.
