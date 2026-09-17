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

## 2026-09-16 — the linearity analysis asked the whole program once per question

Clay's gavel that morning made development-loop cost its own welfare, and the
first thing measured under that heading was not the front end. Callgrind on
`kanso build` of a FORTY-LINE corpus, kanso's own process:

    kanso::build                           1,150,998,944   97.91%
    codegen::emit_ir                       1,144,148,711   97.32%
    linear::Analysis::new                    760,255,840   64.67%
    linear::Analysis::callers_hand_over      747,681,475   63.60%
    linear::Analysis::callsites_unique_in    623,488,195   53.04%

Two thirds of a build, in the pass that decides which `push` call sites own
their list. `kanso check` never runs it — linearity is a codegen-time analysis
— so not one of the three compile veins has ever seen a byte of it, and the
model that would is the one ruled this morning.

**The shape is the declares quadratic again.** `fixpoint()` loops; each round,
for every (name, arity, index) still believed linear it asks
`callers_hand_over`, which walks every function, every statement and every
expression in the program looking for calls to that one name. `escapes_as_value`
does a second full-program walk per (name, arity) per round.

`escapes_as_value` is exact in one pass, and that is what this change is.
`mentioned_as_value(e, name, arity)` was true exactly when some occurrence of
`Ident(name)` was not the head of an application of `arity` arguments. So the
question needs two facts per name: whether it ever occurs outside an
application head, and which argument counts it heads an application with. Both
are properties of the program alone — nothing the fixpoint does can change
either — so one walk before the fixpoint starts answers every ask.

    kanso::main             1,238,723,077 -> 1,065,298,291   -173,424,786   -14.00%
    linear::Analysis::new     760,290,312 ->   586,876,488   -173,413,824   -22.81%

The two falls agree to eleven thousand instructions, which is the check that
the win is where the reading said it was and not somewhere else.

**The emitted IR is byte-identical on all fourteen benchmarks**, runbench
included at 36,085 lines. The analysis feeds codegen, so that is the claim
worth making about a rewrite of it: what the compiler decides has not moved,
only what it spends deciding.

**And the walk it replaced is kept as the oracle.** `mentioned_as_value` is
`#[cfg(test)]` now rather than deleted, and a spec runs both it and the index
over lib/json and bench/compile_corpus for every (name, arity) pair
`escapes_as_value` can be handed — several thousand questions, both ways, on
programs this repository actually compiles. Watched red before it was trusted:
with `escapes` answering `bare.contains` alone, so that an application head
with the wrong argument count stopped escaping, it names the first
disagreement — "lib/json: the index and the walk disagree about
`Get_position` at arity 0".

**And then the larger half turned out to need no state at all.**
`callers_hand_over` walks every function looking for calls to one name, and
`callsites_unique` has exactly ONE `return false` of its own: the bad call site
for that name. Every other path recurses or falls through to
`child_exprs(..).all(..)`. So a declaration whose body never mentions the name
can only answer true, and walking it is the whole of the cost. The same walk
that built the escape index records, per name, which declarations mention it —
which is a property of the program and needs no round of the fixpoint — and
`callers_hand_over` iterates those and no others.

    kanso::main             1,238,723,077 ->   495,523,498   -743,199,579   -60.00%
    linear::Analysis::new     760,290,312 ->    17,129,979   -743,160,333   -97.75%

The pass that was two thirds of a build is 3.5% of what is left of one.
`Analysis::new` falls 44.4x and the build falls 2.5x, and the two absolute
falls agree to thirty-nine thousand instructions, which says again that the
whole of it is inside that pass.

The emitted IR is byte-identical on all fourteen benchmarks after both changes,
checked separately for each.

What is genuinely left is `callsites_unique_in` itself, which is still the
work that remains inside those few declarations, and `codegen::Backend::emit`,
which was 12.54% of the old build and is a much larger share of the new one.

**What CI read, and it is not what this change did.** The three compile rows
fell -13,771 (-0.0373%), -46,634 (-0.0354%) and -46,344 (-0.0351%). None of
them is this change doing less work on that path: `kanso check` does not run
the linearity analysis at all. `in_place_pushes` is called from `emit_ir`, and
from main.rs only behind `KANSO_BEAT_REPORT`, which no gate sets.

So it is the layout family, and a larger member of it than the seven before it.
The falls are proportional to the row rather than a fixed amount per process,
which rules out the maps parse; what they most likely are is
`mentioned_as_value` leaving the release build — it is `#[cfg(test)]` now — and
the generic instantiations linear.rs shares with the check path being inlined
differently without it. Recorded as unattributed rather than explained, which
is the honest state of it. Welfare holds at 69.75.
## 2026-09-16 — gavel: two welfares and a meta-welfare over them, and the floor re-ratchets

Clay ruled the ledger's "What the compile term counts once codegen is in it"
the same day it was filed, and ruled it by supplying the framework rather than
picking among the entry's options. The entry had offered his own proposal back
to him as an option 3 to choose; he corrected that: "what do you mean my call
in the shape of the fix. I just discussed with you a general framework for
updating the welfare metrics."

**The ruling.** The objective becomes three numbers.

    development welfare   the edit-test loop: front-end cost (`kanso check`,
                          which `kanso test` runs on every invocation),
                          dev-tier codegen (`-O0`), interpreter start-up,
                          interpreter speed, interpreter memory
    production welfare    the binary: native run instructions, native run
                          memory, release-tier codegen (`-O3 -flto`)
    meta-welfare          a function of the two, and the number CI gates on

His framing, verbatim: "if we're optimizing for production performance (CPU
and memory) and not compile performance, then compile performance
(speed/instructions and to some extent memory) becomes more like a very
dialed-down input to the overall welfare. then we have a separate welfare for
the interpreted version, where start-time is vastly more important than speed
which is more important than memory usage. of course sometimes these welfare
metrics themselves will conflict, so then you need something like a
'meta-welfare' which is a function of both, because sometimes it will make
sense to do a change which makes development speed much better in exchange for
a very small production performance cost, or vice versa."

**Why the split, and not a re-weighting.** Interpreter start-up is paid on
every test run and never once in production. The same microsecond is enormous
in one context and free in the other, and no single scalar can hold both
readings of it. That dimension is unexpressible in today's model and is the
reason the split earns its cost.

**One correction to the proposal as stated, and it was made in the chat before
the ruling.** Compile cost does not dial DOWN, it MOVES. `kanso test` runs the
front end on every invocation, so front-end cost sits beside interpreter
start-up as a first-class DEVELOPMENT term. Production welfare carries codegen
rather than checking.

**What it closes.** The entry asked which of the two clang tiers welfare
should price. Under two welfares there is nothing to pick: dev-tier codegen is
a development term, release-tier codegen is a production term, both counted
where their cost is paid.

**Three things the entry raised that were never Clay's to settle, and are
recorded here as consequences rather than decisions.**

- **One floor, on the meta.** This follows from the standing rule that the sum
  is the objective and the terms are diagnostics. Ratcheting the sub-scores
  separately would re-enable the part-against-whole optimisation that rule
  exists to stop. Filing it as an open question was the chat's error.
- **Whether the meta layer saturates is the implementer's.** The 2026-08-25
  gavel already says so in its own words: "Weights and satiation for the
  measured terms are the holder's to price from evidence... that is
  implementation under the ledger's own charter, and it does not come back
  here." Noted for the pricing: `a·W_prod + b·W_dev` with a linear meta is
  algebraically one flat term list, so a saturating meta is what makes the
  composition more than arithmetic — a sub-welfare near its ceiling then earns
  little from further wins, which is how the model says "the interpreter is
  fast enough now."
- **Start-up reuses count-from-`main`.** Interpreter start-up is the stretch of
  execution normalised out of the compile row on 2026-09-15; measuring it is
  not a contradiction, since noise inside one measurement is the object of
  another. The counter counts kanso's own start-up work and normalises the
  loader's, which is the machinery kanso#1439 shipped.

**THE FLOOR RE-RATCHETS.** Clay: "yeah you've got to re-ratchet." The
changeover is a model correction, not a rebase recorded and left: the meta
floor is set from the rescored model in the same change, exactly as the
2026-08-25 gavel specified for the last model correction. No change rides
across the changeover holding a score it earned under the old model.

**What cloud builds.** The counters for the development side, which do not
exist yet — interpreter start-up, interpreter speed, interpreter memory, and
dev-tier codegen cost — and release-tier codegen cost on the production side.
`bench/objective_sources.txt` gains every one of them in the same commit that
adds them, with `tests/the_objective_reads_what_the_gate_watches.rs` replaying
the file, because this model's PROSE has gone stale twice while the file never
did. Weights and satiations priced from evidence. The entry leaves the ledger
with this commit and STATUS.md carries the build.

## 2026-09-17 — where two readings part, function by function

The three compile rows each read one figure out of a callgrind profile:
`kanso::main` inclusive. When two readings of one binary on one machine
disagree, that figure says how much and nothing about where. Every hunt
through 2026-09-16 had to guess from the size of the move, and the guesses
have been wrong twice: the runner's CPU model and the binary's sha were both
published as the cause of the 13 and neither was.

`scripts/gates/profile_diff.sh` totals each function's SELF cost in two
profiles, joins on the name, and prints every function that moved. The three
gates call it in the branch that has already established case (2) — the same
binary counting two numbers in one job — where the two profiles are still on
disk and nothing else in the job can say which frame carries the difference.

**The profile is parsed here rather than through `callgrind_annotate`, and
that is not a preference.** `--threshold` is a percentage of the total and 100
is its maximum, so the tool stops as soon as the running percentage rounds to
100. On a profile whose hot function is 99.999% of it, the entire tail is
dropped — and the tail is this instrument's whole subject, because thirteen
instructions in a hundred and thirty-two million live nowhere else. The first
draft read the annotated table and reported two profiles differing by exactly
that as identical. `tests/two_readings_that_part_name_the_frame.rs` is built
from a frame one ten-thousandth of its profile for that reason, and it was
watched red against the first draft before the parser replaced it.

**Two real library profiles on this container agree function by function.**
Eight runs had already read 133,335,824 identically; this is the same fact at
a far finer resolution, and it says the container is not where the flutter
lives. The parser's self-cost sum matches `callgrind_annotate`'s PROGRAM
TOTALS exactly on a real profile, which is the check that it reads the format
rather than something near it.

**The temp files are named for the process.** Three gates diff their own pair
and the spec runs two comparisons at once; a fixed path had one of them
reading the other's answer, which is how the second spec first went red.

**The two CI runs that differ by 13 differ in one thing.** kanso#1464's rounds
carried identical compiler source — round two changed goldens, the log, the
floor and one page. Same glibc 2.39-0ubuntu8.9, same rustc 1.98.1, identical
`.text=2799906 .data=12672 .bss=29976`, and all three compile rows exactly 13
lower in round two. Identical section sizes rule out layout. The binary sha
differs because mimalloc's `options.c` prints a banner built from `__DATE__`
and `__TIME__` — `libmimalloc-sys` passes `-Wno-error=date-time` for it — and
those are fixed-length strings, so every offset in the binary is unmoved. What
is left is the runner: AMD family 0x19 model 0x11 in round one, model 0x1 in
round two.

**A single CPU feature bit moves the row by single digits.** On this container,
`GLIBC_TUNABLES` `hwcaps=-AVX2_Usable` moves the library row by exactly +2,
from 133,429,679 to 133,429,681. Larger masks move it by a great deal —
`-ERMS` by −1.86M, `-AVX_Fast_Unaligned_Load` by −395k — and that is routine
selection. The +2 is a per-process constant of the shape the 13 has.

**Eager binding is not the lever, and the control says why.** `LD_BIND_NOW=1`
raises the row by 2,511, and so does `XX_BIND_NOW=1`, which means nothing to
the loader: the whole move is one more entry in the environment, not the
binding mode. The row costs 2,511 instructions per environment variable
regardless of that variable's length — 1, 2, 3, 4 and 8 characters all read
identically. The gates run under `env -i` with two variables, so this is
normalised on CI already; it is recorded because it is the same class of thing
and it was very nearly published as a finding about the loader.

**The profiles now leave the job.** The comparison left is between two runs on
two runners, which no single job can make. The cost-goldens job uploads the
three profiles it counted, and the host's CPU family, model, stepping, glibc,
rustc and binary sha beside them, so `profile_diff.sh` can be run across a
model 0x1 sitting and a model 0x11 one and name the frame that carries the 13.


## 2026-09-16 — the first development counter measured, and it found a quadratic

Clay's gavel that morning made interpreter start-up a first-class term: it is
paid on every `kanso test` invocation and never once in production, and no
single scalar can hold both readings of the same microsecond. The entry says
the development counters do not exist. This is the first of them measured, and
measuring it was enough.

`kanso play` on a file holding one `print "x"` cost **69,207,585 instructions**
at the `kanso::main` anchor, 70,258,769 for the whole process. That is nearly
twice what the whole module corpus costs to check, to print one line.

**Three quarters of it was substring search.** `memchr_aligned` 30.88%,
`<&str as Pattern>::is_contained_in` 24.76%, `CharSearcher::next_match`
18.06%. Followed through an inlined closure to a `Vec::from_iter` making 1,204
closure calls, and from there to one owner: `codegen::Backend::emit`, three
calls, 65,049,261 instructions, 21,683,087 apiece — 92.6% of start-up in one
expression.

The expression is the `declares` filter. `DECLARES` holds 1,187 lines: 163
`declare` lines and 1,024 lines of inline helper body. The filter walks all of
them and asks `referenced(sym)` for the 163, and `referenced` built a probe
with `format!` and then searched the emitted body, the call twins, and — the
quadratic — `DECLARES.lines().filter(..).any(|l| l.contains(&probe))`, which
re-split `DECLARES` and re-scanned its 1,024 helper lines.

**The `||` is why the shape of the program decides the cost.** A symbol the
body actually calls answers on the first clause and never reaches the third.
Counted on the two ends of the range: the one-line program references 33 of the
163, so 130 fall through and re-scan 1,024 lines apiece — about 133,000 line
scans; runbench references 82, so 81 fall through. The quadratic bites hardest
on the program that uses the least, which is the program `kanso test` compiles
over and over.

Two changes, measured one at a time.

**The helper text is built once.** Joining the non-declare lines into one
string ahead of the filter is exactly equivalent: the probe is `@sym(`, which
holds no newline, so no probe can match across a join made with one.
69,207,585 -> 8,460,712, a fall of 87.8%.

**And then the haystacks are read once rather than per candidate.** The
question asked of each text is whether `@sym(` appears in it, and the set of
symbols satisfying that can be read off in a single pass: every `@` begins a
name and the next `(` ends it. That is the same answer for the same reason the
`DECLARES` parser below it already reads a declared name as the span between
those two characters — a symbol holds no `(`, so the first one after an `@` is
exactly where the name stops. `referenced` becomes a set lookup.
8,460,712 -> 4,934,340.

    kanso::main     69,207,585 -> 4,934,340   -64,273,245   -92.87%
    whole process   70,258,769 -> 5,991,364

Fourteen times.

**What it does not do is make a big build faster, and the reason is worth
having.** `kanso build bench/runbench` reads 10.1 seconds before and after. Its
wall clock belongs to clang — `clang -O3 -c` on the emitted 1.25 MB of IR is
3.2 seconds on its own — and against that the front end is 0.057 seconds and
the emitter's saving disappears into the noise. The row this change moves is
kanso's own work on a program small enough for that work to be the whole of it.
Wall time on the one-liner moves with it but by much less than the instruction
count does, 0.0496s to about 0.040s, for the same reason: `kanso play` still spawns
clang and links.

The emitted IR is byte-identical across the pair on
the largest program in the tree — runbench, 36,085 lines, md5
`ebd24064f1a55e8effeb3ed5f08d4cf4` before and after — so nothing about what the
compiler produces has changed, only what it spends deciding it.

**Nothing in the tree could see this.** `compile_instructions` and its two
neighbours stop before codegen, and `emitted_code` and `machine_code` read
output that did not move. A win of this size with no golden against it is a win
the next change is free to give back, which is the ironclad rule's whole
subject — so the counter comes with it rather than after it, and the gavel
orders that counter anyway.

**The vein.** `bench/startup_instructions_golden.txt`, read by
`scripts/gates/startup_instructions.sh`, counting `kanso play` on
`bench/startup_corpus/main.kso` — one `print` — at the `kanso::main` anchor
with the same emptied environment and pinned tunables the three compile rows
use. It is the fourth callgrind row and the only one that reaches codegen. Its
own sitting on this container, one box and one corpus, the two binaries
differing in nothing else:

    base    69,183,407
    fixed    4,919,980
    delta  -64,263,427   -92.89%   14.06x

It is an exact vein of its own and NOT an objective term, the way `.text` is
pinned under the 2026-09-05 ruling. The objective takes it when the model
splits, which is the gavel's own build and not this one.

**CI's own first sitting is 6,018,394, and the container is 22.3% BELOW it.**
That is the opposite sign and twenty times the size of the 1.2% this box reads
HIGH on the three compile rows, and the row's own composition is why: start-up
is the loader, the prelude and the backend deciding what to emit for one
`print`, and the loader's share belongs to the runner's glibc and rustc rather
than to anything the compiler does. The compile rows are dominated by the
compiler's own passes and this one is not, so the projection that holds for
them does not hold here. CI's is the number the golden carries.

**The row reproduces.** Four runs of one binary on this container all read
4,919,980, which is the question the gate's case (2) asks and the reason to ask
it before pinning anything. The reproduction matters more here than on the
compile rows, because `kanso play` SPAWNS -- a clang feature probe, clang
itself and the linker -- and a process that forks is a process whose own work
could vary with what it forks into. Measured against that worry: with
`--trace-children`, the three children of a `kanso build` count byte-identically
across two runs (clang -cc1 532,991,920, the clang driver 31,648,129, ld
87,120,997) while kanso's own process varies by 480. The anchor sits inside
`kanso::main`, and four sittings say the variance does not reach it.

**AND THEN CI COUNTED IT TWICE.** Round two wrote 6,018,394 in and read
6,018,427 on identical compiler source -- round two's whole diff was goldens,
the log, the floor and one page -- a move of 33. The three compile rows beside
it agreed exactly across the same pair, so it is this row alone. kanso#1459's
two rounds did the same thing with 13 on all three compile rows, and the same
13 hit kanso#1460, whose whole diff was a log entry.

Thirty-three has an obvious suspect and it is wrong. `clock_gettime` costs
exactly 33 instructions in this profile -- three calls at 11 apiece, which is
`_mi_clock_start`'s calibration, the three reads the purge-delay setting left
behind and which src/main.rs's own comment names as a term whose cost is the
host's vDSO. Walking the profile's call graph settles it: the chain is `(below
main)` -> `_mi_auto_process_init` -> `mi_process_attach` -> `mi_process_init`
-> `_mi_stats_init` -> `_mi_clock_start`, and `kanso::main` is not an ancestor
of any of it. mimalloc calibrates from `.init_array`, before main, so those
three reads are already outside the anchor and cannot be what moved.

So the 33 is unexplained, like the 13, and both are outside the diff. What this
round adds is the machinery to settle the next one: the gate now counts a
SECOND time on the same binary in the same job when the first reading
disagrees, and says in its own error text whether the binary is stable or
counted two numbers. kanso#1463 does the same for the three compile gates.

**One more measurement, on a real build rather than a one-liner.** Gate-shaped
with `--trace-children`, five processes counted, on a forty-line corpus that
imports std/list and std/text:

    kanso build, whole process tree, main         1,834,946,532
    kanso build, whole process tree, this branch  1,765,595,173
                                                    -69,351,359   -3.78%

So the declares fix is worth 69.35 million instructions on a build that spawns
clang and links, not only on the one-line program the vein counts. kanso's own
share of that tree is 1,238,699,420 on main and 1,169,347,956 here.

**Three layout moves came with the change**, and they are small and do not
move together: `compile_instructions` -194 (-0.0005%), `entry_instructions`
-346 (-0.0003%), `library_instructions` -18 (-0.00001%). `kanso check` stops
before codegen, so none of them is this change doing work differently; they are
the move CLAUDE.md's note on that row describes, because src/codegen.rs IS the
compiler and editing it moves the compiler's bytes and what sits around them.
Three different magnitudes on three rows is the signature of layout rather than
of work. Welfare holds at 69.75.

Four things had to move with it, and three were found by specs rather than by
hand, which is the point of them. `scripts/gates/library_box.sh` stages the new
corpus. `scripts/trend_gate/trend_gate.kso` names the golden, because
`tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads `bench/` off
disk and keys on the word `golden` — it went red the moment the file existed.
`scripts/ratchet/ratchet.kso`'s `host_bound` list gains the gate, because
`tests/a_host_bound_gate_is_reported_not_credited.rs` asserts that list and the
gates running callgrind are the same set, and it named the omission exactly:
"a callgrind gate left off the list makes a runner mismatch fail the whole
job". And `.github/workflows/ci.yml` gains the step and its entry in the vein
summary, which is the block that actually fails the job.
## 2026-09-17 — the allocator was guessing at addresses, and the row was paying for it

The three compile rows have disagreed with their goldens by thirteen
instructions across runs of identical source since the compiler moved to
mimalloc on 2026-09-15, and the 2026-09-05 ruling halts a vein that counts two
numbers for one row. Two published diagnoses were wrong: the runner's CPU
model and the binary's sha, which were confounded with each other on the only
evidence available at the time.

**The instrument that settled it prints where two profiles part.**
kanso#1463 made each compile gate take a second reading inside the job when
its row fails, and `scripts/gates/profile_diff.sh` totals every function's
self cost in both profiles and lists the ones that moved. On the first run
that carried it, the entry row read 131,884,793 and then 131,884,271 — one
binary, one corpus, one job, 522 apart — and all sixteen functions that moved
were mimalloc's OS-allocation path: `mi_page_map_set_range_prim`,
`mi_os_prim_alloc_at`, `_mi_prim_alloc`, `_mi_os_alloc`, `_mi_os_zalloc`,
`mmap`, `prctl`, `_mi_os_get_aligned_hint`, the stat counters and the mutex
around them.

**mimalloc's own source says why.** `v3/src/os.c`:

```c
#if (MI_SECURE>=1 || defined(NDEBUG))  // security: randomize start of aligned allocations
    const uintptr_t r = _mi_theap_random_next(theap);
    init = init + ((MI_HINT_ALIGN * ((r>>17) & 0xFFFFF)) % MI_HINT_AREA);
```

A release build defines `NDEBUG`, so every process draws a 4 MiB-aligned base
out of a 4 TiB window from per-process entropy and hands it to `mmap` as a
hint. The page map commits its entries by address, so the same allocation
costs a different number of instructions depending on where it lands. Three
runs on one container, one binary, one corpus and one environment, watched
with `--trace-syscalls`, asked the kernel for `0x48e11400000`,
`0x52844800000` and `0x38240c00000`.

**So it is normalised rather than explained.** `MI_NO_ALIGNED_HINT` is
mimalloc's switch for exactly this: the function then always returns NULL and
the OS chooses, which under valgrind's address-space manager is the same
address every run. `.cargo/config.toml` defines it through `CFLAGS`, because
libmimalloc-sys exposes no feature for it and the `cc` crate appends `CFLAGS`
to its own flags. `mi_option_max_vabits` looked like a runtime lever and is
not one: it sizes the page map and never reaches the
`mi_os_mem_config.virtual_address_bits` the hint reads.

**One mutation had to be rewritten, and it said so in advance.** `a_library_the_row_cannot_see.sh` writes a `.cargo/config.toml` carrying
`-C prefer-dynamic`, so the compiler grows a shared object the instruction row
cannot see. Its own comment anticipated this: "a repo that grows its own cargo
config has somewhere for this flag to be lost, so the mutation stops rather
than appending into it." It appends now, and refuses only if a `[build]`
section is already there to collide with.

**It costs nothing measurable.** The library row read 133,429,679 with the
hint and 133,429,679 without it, and three runs without it agree function by
function. `tests/the_allocator_does_not_guess_at_addresses.rs` reads
mimalloc's source and goes red if a crate bump renames the switch or a config
edit drops it.

**Round two: the three rows read thirteen lower, and always did.** CI on the
no-hint binary reads `compile_instructions` 36,878,537, `entry_instructions`
131,884,271 and `library_instructions` 132,025,154 — each thirteen below its
golden. The change did not move them: the run before this one, with the hint
still on, read 36,878,537 and 132,025,154 for two of the three. The goldens
were written from a sitting that drew an unlucky address and have been
thirteen high since. The entry row is where it shows plainly: with the hint on
it read 131,884,793 and then 131,884,271 inside one job, and with the hint off
it reads 131,884,271 and nothing else.

**Round three, after kanso#1466.** CI on the merged head reads
`compile_instructions` 36,695,922, `entry_instructions` 130,618,857 and
`library_instructions` 130,762,703 — the figures this branch had measured
before, to the instruction. The merge brought main's values in and this writes
the branch's back. Two runs agreeing is what the allocator fix bought.

**Round three, after kanso#1466: the interp row is on its golden.** The
interpreted run had read 2,324,888,437 against a golden of 2,324,888,431 on
every other run; on the merged head it reads 2,324,888,431 and CI says
`interp_instructions: 2324888431, on the row`. Both memory rows are
byte-exact. The six instructions were the allocator picking a random base
address, which is the same fault the three compile rows had at thirteen.

The three compile rows come back at `compile_instructions` 36,878,080,
`entry_instructions` 131,881,423 and `library_instructions` 132,023,458,
against main's 36,878,537, 131,884,271 and 132,025,154. That fall is the
fixed-seed hashing this branch puts on the compile path.

`compile_instructions` 36,900,512, `entry_instructions` 131,966,724 and
`library_instructions` 132,070,594 — to the instruction, the figures round two
measured and the next run then disagreed with by thirteen. The merge brought
main's values in and this writes the branch's back. `compile_allocs` reads
27,397 and agrees, now that the note sits above the value rather than after it.

**Round three: the start-up row was counting a cold cache.** One binary, one
job, read 6,018,427 and then 4,869,632 — 1,148,795 apart, a fifth of the row.
`kanso play` takes the native path, so the first process writes
`kanso_runtime_<profile>_<key>.o` and `kanso_run_<key>` into the temp
directory and every process after it reuses them. The gate now runs the
program once before it counts, which puts the cache in the state every reading
after the first would have seen. That is the 2026-09-15 rule applied to a
cache rather than to a clock: put the external state into a known state, do
not explain it afterwards.

The three compile rows come back on the merged head at `compile_instructions`
36,878,356, `entry_instructions` 131,883,938 and `library_instructions`
132,025,149, against main's 36,878,537, 131,884,271 and 132,025,154.
**Round three, after kanso#1466: the same three rows, read again and agreeing.**
CI on the merged head reads `compile_instructions` 36,864,779,
`entry_instructions` 131,837,650 and `library_instructions` 131,978,823 — to
the instruction, the figures round two took from CI and that the next run
disagreed with by thirteen. The merge brought main's values in and this writes
the branch's back. It is the first time this vein has reproduced across two
runs since the compiler moved to mimalloc, which is what kanso#1466 was for.

**Round four, after kanso#1464.** CI reads `compile_instructions` 36,682,232,
`entry_instructions` 130,573,787 and `library_instructions` 130,716,747 against
main's 36,864,779, 131,837,650 and 131,978,823. Asking the pattern before the
binder is what this branch's share of that is; the rest of the move against
round three is kanso#1464 arriving underneath it.

## 2026-09-17 — the thirteen is not compiler work

**All three compile rows move by the same thirteen.** DONE. The module, entry
and library rows have disagreed with their goldens by exactly thirteen
instructions across several sittings, and until now each was read on its own.
Put side by side on kanso#1463 they read 36,864,766 against 36,864,779,
131,837,637 against 131,837,650, and 131,978,810 against 131,978,823. Three
routes through the compiler, one of them 3.6 times the size of another, each
off by thirteen. A term that costs the same thirteen on a 36.9-million-
instruction compile and a 132.0-million-instruction compile does not scale with
the input, so it is not the compiling. Every account that put the thirteen in
the front end is dead: the lexer, inference, the emitter and a layout effect on
hot code all grow with the source, and this does not.

**The two sittings of one commit agreed.** DONE. kanso#1463's job was re-run on
its own head to try for two profiles differing in nothing but the run. Both
attempts read the same three numbers. So the row reproduces within a commit
since kanso#1466 took mimalloc's randomised base address out, and the thirteen
separates a sitting from the sitting the golden was taken on.

**The same tree, twice, thirteen apart.** DONE. kanso#1462 was green on
`fc3305f8`. Its branch was updated — protection wants the checks on an
up-to-date head — and the identical work came back thirteen out on all three
rows: 36,862,804 to 36,862,817, 131,830,523 to 131,830,536, 131,972,417 to
131,972,430. What the update brought in was `hooks/post-merge`,
`scripts/install_hooks.sh` and one test file, 132 lines, none of them compiled
into the binary, `include_str!`'d, or read by a compile gate, with no golden
moving. Same compiler, same corpus, same goldens, measured twice. This is the
experiment the re-run above was trying to manufacture, and it arrived on its
own.

**Two re-runs of one head agreed, which the coin-flip reading does not
predict.** OPEN. kanso#1462's failed jobs were re-run on `b8327112` and read
the same three numbers again, +13 from its own earlier green sitting of
`fc3305f8`. Nothing compiled into the binary differs between those two trees:
there is no build.rs, and every `include_str!` in src/lib.rs names a file
under lib/. So the value is a function of something that holds across two
separately-allocated runners of one head and changes between two heads whose
compiled input is identical. Two runs agreeing is a one-in-two event and
proves nothing on its own, but it is enough to stop calling this a per-run
flip until a sitting says otherwise. The host facts now printed beside the
floor -- the kernel release and version, which no job has ever printed --
are there because a per-host term is what this shape looks like and the CPU
model has already been refuted.

**Where it can be.** DONE. The three profiles the job already writes name the
candidates by themselves: 588 frames carry the same self cost across all three
workloads, 556,052 instructions in all. Restricted to frames the row can see —
reachable from `kanso::main`, which is the figure the gates read — 38 remain,
51,269 instructions, and not one of them is compiler work. The largest block is
mimalloc's scan of the environment for its own options, 49,449 instructions.
`getauxval`, called twice from std's stack-overflow handler at 146 each, the
`sbrk`/`brk`/`__glibc_morecore` trio, `sigaltstack`, the argv walk and the
stdout flush make up the rest. A near-empty compile carries 36 of the 38 at
byte-identical cost, which is what a per-process term looks like.

**What the environment actually is.** DONE. The gates run under
`env -i PATH=... GLIBC_TUNABLES=...` and believe they have pinned it. The child
sees seven variables: valgrind adds `LD_PRELOAD`, `LD_LIBRARY_PATH`,
`GLIBCPP_FORCE_NEW`, `GLIBCXX_FORCE_NEW` and `PWD` on top of the two. On one
runner image those are fixed, so this is not shown to be the thirteen — it is a
normalisation the gate claims and does not have, and mimalloc's scan of it is
the single largest per-process term inside the row.

**Ruled out by measurement.** DONE. Visible CPU count does not move the row:
four runs of `kanso check compile_corpus` on this box, bare, under `taskset -c
0` and under `taskset -c 0,1`, all read 37,285,436. The runner's CPU model was
refuted earlier by two sittings on different models reading the same number and
two on the same model reading different ones.

**The instrument.** DONE. `scripts/gates/per_process_floor.sh` prints that
floor — the frames whose self cost held across every workload given, and their
sum — derived from the profiles rather than from a list, so a frame that
appears or disappears is reported. It runs in the cost-goldens job, gates
nothing and pins nothing. Two jobs whose `per_process_floor=` lines differ by
thirteen name the frame between them, which turns a hunt nobody can reproduce
on demand into a comparison of two job logs. `scripts/gates/callgrind_self.sh`
is the reader under it: `callgrind_annotate`'s `--threshold` is a percentage
and stops once the running total rounds to the figure asked for, so the tail it
drops is where a thirteen-instruction frame lives.

**Still open.** OPEN. Which of the 38 carries the thirteen. The next sitting
that reads the other value answers it, and the answer arrives in a job log
rather than in an argument. kanso#1463 stays blocked until then: it is the
change that would pin a disagreeing row as a second value, which is what the
rule it implements forbids.

## 2026-09-17 — the wall is bind with a discarded value, and the one thing that made it more than that is gone

Clay, reading a book sample: "wasn't this convention always a mistake? we
invented >> to deal with no return value. but then we realized that you always
have a return value, which is the effect. so this was really just .> i
believe. one of the fused combinators."

**Measured, on the binary at `cc180f2f`.** `a >> b` and `a .> (_ -> b)` are
indistinguishable on every shape the chat could build:

    print "one" >> print "two"                  one / two
    print "one" .> (_ -> print "two")           one / two
    os/read_file! "nope" >> print "after"       short-circuits, nothing after
    print "left ok" >> print "right {boom}"     left ok, then the endpoint
    print "left ok" .> (_ -> print "right {boom}")   identical
    print "left {boom 1}" >> print "right {boom 2}"  boom 1 alone
    print "left {boom 1}" .> (_ -> print "right {boom 2}")  identical

**The archive says the last two used to differ, and that is the finding.** The
2026-08-24 entry measuring the wall recorded:

    print "left {boom a}"  >> print "right ok"        -> a
    print "left ok"        >> print "right {boom b}"  -> b
    print "left {boom a}"  >> print "right {boom b}"  -> [a b]

with "Nothing prints in any of the three. `>>` orders effects, and both
descriptions are built before either runs, so a failure raised while building
is not ordered by the wall — two of them are simultaneous and merge, the same
reasoning the parallel group uses. Haskell's `>>` answers `a` in the third
case because it is lazy in its right side; kanso builds both and learns more."

Eager construction of both operands was the one thing a lambda could not
imitate, and it does not hold on today's build: the third case answers `boom
1` rather than `[boom 1, boom 2]`, and `left ok` prints where the entry says
nothing printed. The chat could not find the entry that moved it. So the
semantic that earned `>>` its own operator went away unrecorded, and what is
left is sugar for a bind whose callback ignores its argument.

**The tree is split between the two spellings.** 570 sites write `>>` — 1 in
lib, 67 in scripts, 83 in book samples, 419 in tests — and 184 write
`.> (_ -> ...)`, four of them in lib/net/http alone. One operation, two
spellings, and nothing in the language says which. CLAUDE.md's own reason for
having no formatter and no linter is that "the grammar decides every question
a linter would ask", and here it has stopped.

**Three rules bear on it and none was applied to the wall.** The 2026-08-26
gavel minted `done`, which removed the premise that an effect answers nothing.
The 2026-08-29 gavel made effects a type whose only doors are the three words.
The 2026-08-31 gavel said that in chain position the fused form is the ONLY
spelling — and `>>` is a fourth chain-position operator over effects that
predates the effect type and was never held against that rule.

**The parked objection does not apply.** design/pending-gavels.md's Parked
list carries "dot-absorbs-`>>`: argued no — erases the visible then/bind
split." That was about the PLAIN dot absorbing the wall. The plain dot stopped
binding on 2026-08-29 and is an ordinary application now, so the entry argues
against a proposal nobody is making.

Both questions go to the ledger: whether the wall survives the fused
operators, and whether the simultaneous-failure merge was meant to go.

## 2026-09-17 — the sweep, and two rulings built inside a day

Fired 03:47:17Z, run at 03:47. Eight pull requests open in kanso, none in kq,
the oldest 5.9 hours, so nothing aged. Four are red on `cost goldens` — the
welfare and counter work moving veins under a model change — and three are
blocked or behind. Fifteen merged since the 2026-09-16 sweep; twelve name the
rulings they weighed and the three that do not are a log trim, a ratchet row
and a build-artifact hook, none of them a self-generated lead.

**The build hole is built.** kanso#1447 landed the 2026-08-24 ruling the day
after the chat found it twenty-three days off the unbuilt list.
`person "ada" _` runs, and `docs/book/samples/ch03/knot.kso` carries `_` where
it carried `none` for three weeks. Its row comes off. One site still reads
`none`: `tests/golden/micro/bare_field.kso`, which is likely correct rather
than missed — the 2026-08-24 entry says "a field may legitimately hold `none`
forever" — and is noted here so the next reader does not re-derive it.

**The explicit box is most of the way built and one probe says not all.**
`effect 5` answers a box, so the constructor landed on the ledger's
recommended spelling. `menu["dango"]!` answers a box, so the 2026-09-16
reversal is built. What did not reproduce is part 3, the check-time refusal:

    fn boom _
      err "b"

    n = boom 1

    print "{n + 1}"

reaches the endpoint at run time rather than being refused at check, on a
one-arm group whose answer is provably an err. Whether that is a limit of what
infer proves or a gap in the pass is cloud's to determine; the program is
recorded here so the question starts from a fixture rather than a memory. The
row stays until it is answered.

**Cloud is already building the two-welfare ruling**, seventeen hours after it
landed: kanso#1461 takes interpreter start-up down 14x and kanso#1470 opens
the codegen row, which is the half of a build nothing counted.

## 2026-09-17 — the allocator asked for an alignment it never needed

**mimalloc's Rust shim sends every allocation through the aligned path.** DONE.
`MiMalloc::alloc` calls `mi_malloc_aligned(size, align)` whatever the alignment
is. That wrapper checks the alignment is a power of two, builds a mask from it,
takes a candidate block off the small-page free list and tests whether the
block is aligned, before it can hand back the block `mi_malloc` would have
handed back on its own. `mi_theap_malloc_aligned` was 1,608,924 instructions of
`kanso check compile_corpus`, which is more than any frame in check.rs.

**Almost none of that work had anything to do.** DONE. A `Vec<u8>`, a `String`
and any record whose widest field is a pointer or a u64 ask for eight, and every
block mimalloc gives out is at least eight-aligned. Those go straight to
`mi_malloc` now; anything wanting more still takes the aligned path. Measured on
this container, three rows, before and after:

    compile_instructions    36,956,079 -> 36,241,230   -1.93%
    entry_instructions     131,550,570 -> 129,114,693  -1.85%
    library_instructions   131,694,754 -> 129,252,791  -1.85%

CI read it as:

    compile_instructions    36,682,232 -> 35,969,565   -1.943%
    entry_instructions     130,573,787 -> 128,144,579  -1.860%
    library_instructions   130,716,747 -> 128,281,268  -1.863%

against this container's projection of 1.93%, 1.85% and 1.85%, which is the
projection working. The floor is ratcheted to 69.79 in the same commit.

**Eight, and not sixteen, and mimalloc says why itself.** DONE. From
v3/src/alloc.c: `mi_assert_internal(page->block_size < MI_MAX_ALIGN_SIZE ||
_mi_is_aligned(block, MI_MAX_ALIGN_SIZE))`. A block is sixteen-aligned unless it
is smaller than sixteen bytes, and `Layout` carries size and alignment
independently, so `align 16, size 8` is spellable. Eight is the bound that holds
for every size.

**src/main.rs is in the wasm build, and the first round forgot it.** DONE.
`UNDER` is mimalloc on every target that can build it and `std::alloc::System`
on wasm32, and the bypass was written without that guard. On wasm
`libmimalloc_sys` is not linked at all, so the branch did not compile there,
`docs/kanso.wasm` never got built, and six jobs went red behind one missing
blob: the browser differential, the site, the asset digests, the specs and the
other host. The bypass carries the same `#[cfg(not(target_arch = "wasm32"))]`
the allocator above it does; the System allocator honours its `Layout` and has
nothing to skip.

**A run-time spec cannot reach this code, and two were written before that was
noticed.** DONE. The `#[global_allocator]` is in the binary crate; an
integration test links the library, so every allocation a test makes goes
through the harness's allocator instead. One of the two asked for alignment 4096
in blocks of eight bytes and still passed with the bound raised to 8192, which
is what a spec that reaches nothing looks like from the outside. What ships
reads source: mimalloc's assertion where it is written, and the bound in
src/main.rs against it. Both watched red — the first by altering the assertion
it quotes, the second at 16.

**mi_realloc is worse, measured and declined.** REFUTED. `GlobalAlloc`'s
default `realloc` allocates, copies and frees, which is what a growing `Vec`
pays at every doubling, and mimalloc can sometimes extend a block where it
stands. Overriding `realloc` to call `mi_realloc` on the same alignment bound
read 36,315,572 against the 36,241,230 above: a RISE of 74,342, 0.21%. Whatever
the in-place extensions save on this workload, `mi_realloc`'s own path costs
more. It also moved `compile_peak_bytes` by six bytes, because the live-bytes
accounting cannot be made exactly equivalent through one call where the default
route makes two. Not taken.

**What is left here.** OPEN. The profile under this is flat: with the three
quadratics gone (kanso#1464, kanso#1468 and the beat index), the top frame of
`kanso check library_corpus` is `__memcmp_avx2_movbe` at 3.5%, and its callers
are ten map lookups of which the largest is 0.94%. The structural lever is
interning names to integers so the maps stop comparing strings at all, which
would reach that 3.5% and part of the 2.6% in rehashing beside it. That is a
refactor across check.rs, infer.rs and codegen.rs, and it is not costed yet.

**Round four, after kanso#1464.** The interpreted run falls 3,920,499 to
2,320,967,932: the linearity analysis runs on the interpreter's compile path
too, and indexing it is felt here. CI read that figure and then read it again
in the same job. The three compile rows come back at 36,862,804, 131,830,523
and 131,972,417 against main's 36,864,779, 131,837,650 and 131,978,823, and
both interpreter memory rows are byte-identical to the round before.

## 2026-09-17 — CI's sitting of the merged tree, and the floor moves with the silicon

kanso#1462 merged with main after kanso#1459 landed. CI read the merged tree
below the values the merge carried forward:

    compile_instructions    36,682,232 -> 36,681,044   -1,188   -0.0032%
    entry_instructions     130,573,787 -> 130,567,058  -6,729   -0.0052%
    library_instructions   130,716,747 -> 130,711,295  -5,452   -0.0042%

`kanso check` does not run the interpreter, so the hashing this branch changes
is not on this corpus; all three are the layout kind the row's header
describes.

**The per-process floor tracks the silicon, and it still does not explain the
thirteen.** DONE. Three sittings, three CPUs, three floors:

    cpu 25/1    558,610   605 frames
    cpu 26/2    556,282   605 frames
    cpu 6/173   558,222   604 frames

So the floor is a host reading rather than a constant, which is what it should
be. What it cannot do is carry the thirteen: kanso#1469 and kanso#1470 both ran
on cpu 25/1 and both printed 558,610 over 605 frames, to the instruction, with
their three compile rows thirteen apart. A term the floor holds cannot move
while the floor does not. kanso#1474 prints the module compile's whole listing
beside it for that reason.

## 2026-09-17 — kanso#1462 on the merged tree: CI's sitting

The branch merged with kanso#1472 and CI measured the merged tree. Four rows
moved against the values the merge carried forward:

    interp_instructions  2,320,967,932 -> 2,178,656,557  -142,311,375  -6.13%
    compile_instructions    35,969,565 -> 35,968,356           -1,209  -0.0034%
    entry_instructions     128,144,579 -> 128,138,080          -6,499  -0.0051%
    library_instructions   128,281,268 -> 128,275,780          -5,488  -0.0043%

The interpreter row is the branch's own. The three compile rows are layout:
`kanso check` does not run the interpreter, so none of the hashing this branch
changes is on that corpus. Welfare rose and is banked at 69.7922205456612.

`per_process_floor=556292 frames=605 kernel=6.17.0-1022-azure cpu=25/1`.

- **DONE** the rows are CI's.

**Round four, after kanso#1464.** CI reads `compile_instructions` 36,885,953,
`entry_instructions` 131,919,543 and `library_instructions` 132,022,229 against
main's 36,864,779, 131,837,650 and 131,978,823. The set of a dependency's type
names is the rise, and it is the same rise round three measured; the figures
differ because kanso#1464 arrived underneath them.

## 2026-09-17 — CI's sitting of the merged tree: a correctness fix that costs a little

kanso#1465 merged with main after kanso#1459 landed. The merge carried
kanso#1459's values forward so the gate had one value to fail against; CI read
the merged tree above them:

    compile_instructions    36,682,232 -> 36,703,489   +21,257   +0.0579%
    entry_instructions     130,573,787 -> 130,655,644  +81,857   +0.0627%
    library_instructions   130,716,747 -> 130,760,194  +43,447   +0.0332%

**The rise is the fix.** DONE. `declared_names` returned types and functions in
one set, so qualifying a function's name rewrote an imported type's constructor
pattern with it, and a program that named both compiled into one that named the
wrong thing. Keeping them apart costs the checker a little more work to be
right, and a sixteen-hundredth of a per cent of a compile is what being right
costs here.

**The floor does not move.** DONE. welfare reads 69.76 against a floor of
69.76: the merge's own resolution took the higher of the two floors and this
tree clears it. Nothing to lower and nothing to bank.

## 2026-09-17 — kanso#1465 on the merged tree: CI's sitting, and the floor drops

The branch merged with kanso#1472 and CI measured the merged tree:

    entry_instructions   128,144,579 -> 128,214,733   +70,154   +0.055%
    library_instructions 128,281,268 -> 128,348,838   +67,570   +0.053%
    compile_instructions  35,969,565 -> 35,967,913     -1,652   -0.0046%

The two rises are the branch's own cost. The qualifier now keeps a set of the
type names it must not rewrite, and the entry and library routes pay to build
and read it. The module row falls, which is layout.

Welfare falls to 69.79141882095341 and **the floor is lowered to meet it**,
under the 2026-09-13 ironclad rule: the change makes the language work to its
specification. A seven-line program that `kanso check` passed had the two
engines printing different things — the interpreter `2 1`, the native backend
`error: native backend: unknown type <module>/entry` — which is the differential
law broken, not a preference. So the floor drops by exactly what the fix costs,
the reason is in the ratchet history, and this does not go to the ledger.

`per_process_floor=558232 frames=604 kernel=6.17.0-1022-azure cpu=25/1`. Note
604 frames rather than 605: a frame this binary does not have. The floors are
comparable only between two sittings of ONE binary, which is why a reading from
another branch says nothing about this one.

- **DONE** the rows are CI's and the floor is where the measurement put it.

## 2026-09-17 — the verdict fell off the end of the annotation cap

`codegen_instructions.sh` already counts a disagreeing row a second time and
says which of the two cases it is: the change moved the row, or the same binary
counted two numbers. On kanso#1470 both codegen rows and all three compile rows
failed in one job, each printing a dozen lines of explanation, and GitHub keeps
at most **fifty annotations per check run**. The two lines carrying the answer
were past the cap. The job could be read as far as `a move of -6531790` and no
further, so a gate that had already settled the question reported nothing.

The verdict now goes immediately after the count and before the explanation.
The explanation is worth having and it is worth nothing ahead of the answer.

`codegen_again_<tier>` also goes out as a `::notice::`. It was a `printf`, so
it reached the job log alone — the same trap the frame digest fell into on
claude/self-dump the same afternoon, and the same fix. Anything a reader needs
from a CI job has to be an annotation; stdout is for the reader who can fetch
the log, and that is not always available.

The three compile rows on this branch read −13 with no `src/` or `lib/` change
in the diff at all:

    $ git diff --name-only origin/main...origin/claude/codegen-rows
    .github/workflows/ci.yml, CLAUDE.md, bench/codegen_corpus/**,
    bench/codegen_instructions_*_golden.txt, scripts/**, tests/**

so that one is the cross-run thirteen and not this branch's. kanso#1463 landed
the second reading for the compile gates this morning, which is what settles it
from inside the job; this merge brings it onto the branch.

- **DONE** the verdict is readable.
- **OPEN** what the codegen rows' 6,531,790 and 6,645,392 actually are. The
  next sitting says it in one line.

## 2026-09-17 — the verdict became readable and immediately said something wrong

Putting the verdict ahead of the explanation worked: this branch's next sitting
printed, where nothing had been readable before,

    VERDICT (2): REPRODUCTION FAILURE -- this binary counted 9273818206
    and then 1071591479 in one job. This vein is halted.

    VERDICT (2): REPRODUCTION FAILURE -- this binary counted 12100862133
    and then 7307715328 in one job. This vein is halted.

**Those are not two readings of one quantity.** The dev pair is 8.6x apart and
the release pair 1.66x. A row whose value moves by that much between two runs
in one job is not a noisy measurement; the second run measured something else.

The shape of it points one way. A build here is five processes — kanso, clang
at two tiers, and ld — and `codegen_box.sh` already records an early attempt at
this row that read TWO where a real build is five, with dev and release within
0.006% of each other. 1,071,591,479 against 9,273,818,206 is about what kanso's
own process would be without its children. The first reading counts its
processes into `seen`; the second never counted its own, so a drop was
invisible and read as a reproduction failure of the compiler.

The two runs issue an identical command. What differs is the BOX: before the
first, it holds a staged compiler, a staged corpus and the two warm-up builds;
before the second, it also holds whatever the first build wrote. That is the
2026-09-15 rule exactly — external state normalized before it is measured — and
the second reading normalizes the command while leaving the state alone.

This entry does not fix it. It makes the next sitting say which it is:
`again_procs` and `first_procs` join the notice, so a second reading that saw
fewer processes is visible as that rather than as a verdict about the compiler.
Guessing the cause and patching it would be the third prediction today to go
wrong on a profile read by inference.

- **DONE** the process counts are reported.
- **OPEN** whether the second reading loses its children, and why. One line in
  the next sitting.
- **NOTE** the compile rows on this same head did not part at all, so #1463's
  own second reading is not implicated — only this gate's.

## 2026-09-17 — the second reading was counting a build the first one had already done

The previous entry left one line open: whether the codegen gate's second
reading loses its children, and why. The sitting answered it in the notice it
had just been given.

```
codegen_again_dev=1071604124     first_reading=9273832919    again_procs=5 first_procs=6
codegen_again_release=7307726629 first_reading=12100874235   again_procs=5 first_procs=6
```

One fewer process, and the dev row down to a ninth. That is not a compiler that
counted differently twice; it is an incremental build. `kanso build X` writes
its output beside itself as `X`, so the first measured run leaves the box
holding what it just produced, and the second run of the identical command
finds most of its work done. The gate then read the gap as `VERDICT (2):
REPRODUCTION FAILURE` and halted a vein over it.

The fix is the 2026-09-15 rule applied where it was being skipped. Staging is
now a function — `codegen_box.sh`, which opens `rm -rf "$box"`, then both tiers
warmed in a fixed order — and it runs before EACH measured run rather than once
at the top. Both readings start from bytes the gate chose. A disagreement after
this is the compiler's, which is the only thing the second reading was ever for.

`tests/the_second_reading_starts_where_the_first_did.rs` pins it: the staging
call sits between the two profile prefixes, the first run is staged too, the box
script clears rather than copies over, and both tiers are warmed on every
staging. Watched red on each — removing the call between the readings, and
dropping one warm-up.

The other three compile gates run `kanso check`, which writes nothing, so their
second readings were never asking a different question. The spec names this gate
alone and says why.

- **DONE** the row's second reading is a reproduction.
- **OPEN** what the row actually reads once it is one. This host refuses the
  golden's toolchain (glibc 2.39-0ubuntu8.7 against 8.9, clang 18 against 19),
  so the first honest sitting of this gate is CI's.

## 2026-09-17 — the warm-up filled a different temp directory from the one the measurement reads

Re-staging the box was half the fix. The sitting on `84ec8001` read
`again_procs=5 first_procs=6` again, dev 9,273,832,677 then 1,071,605,357, with
the box rebuilt between the two readings. So the second reading was still
finding work done, and the box was not where it was finding it.

The profiles had the answer and nothing was reading it. Every callgrind file
carries a `cmd:` line. Listed for one local first reading:

```
kanso clang:probe clang clang ld
```

Five here, six on CI, and the sixth is the one that matters:
`cached_runtime_object` compiles `src/runtime.c` once per profile and runtime
hash and writes the object to `std::env::temp_dir()`. That cache is not in the
box, so `codegen_box.sh` clearing the box never touched it. The warm-ups were
put there to fill it, and could not: they ran under the job's environment while
the measurement runs under `env -i`, and `temp_dir()` reads TMPDIR. Two
directories, two caches. The first MEASURED run paid for runtime.c and the
second found it.

The warm-up now runs the identical command under the identical environment,
which is what the 2026-09-15 rule asks for and what re-staging alone did not
reach. And `codegen_procs_<tier> first=[...] again=[...]` joins the notices, so
a reading that loses a process names it rather than being counted.

Guessing cost a round here. The first entry on this read the gap as an
incremental build and patched the box; the profiles said runtime.c and were
never opened. The instrument goes in with the fix for that reason.

- **DONE** the warm-up and the measurement share an environment; the processes
  are named, not counted.
- **OPEN** what the row reads once the two agree. This host refuses the
  golden's toolchain, so CI's is the first honest sitting.

## 2026-09-17 — the bound discharge had no golden, and it is built

`STATUS.md`'s "Ruled, unbuilt" section carries the explicit box, and among what
that ruling owes are "the two cost levers, an inlined bind for a pure index
read and bound discharge for a literal index into a known-length list, each
measured". Both are on main. Probed against `15e182b1`:

    xs = [10 20 30]
    show xs[1]     runs, no `none` arm needed
    show xs[3]     runs, no `none` arm needed
    show [7 8][2]  runs, no `none` arm needed
    show xs[0]     error[exhaustive]: this can be a none
    show xs[4]     error[exhaustive]: this can be a none
    show xs[n]     error[exhaustive]: this can be a none

Indices run from 1, so 1 through 3 are the whole of a three-element list. The
discharge is exact at both edges and does not fire for an index the compiler
cannot read off the source. The inlined bind has a golden already —
`an_index_without_the_bang_reaches_its_twin`, whose first line says the index
written without the `!` inlines.

The discharge had none. That corpus pins the MISS path and nothing pinned the
hit: a compiler that discharged every index read, which is the wrong rule and a
quiet one, passed every golden in the tree. This entry ships the fixture that
fails on it, and it is the four cases above in one program so the two halves
cannot drift apart.

It was watched red before it was watched green: with `at zero` reading `10`
instead of `<none>`, `micro_corpus_survives_a_release_build` names the file and
prints both lines.

Two notes on writing a micro golden, both learned the slow way here. A file
without `\npub play` is SKIPPED by both micro tests, silently — the first draft
of this fixture used bare statements, was read by nothing, and passed with its
expected output deliberately wrong. And the file may not carry a blank line
between its header comment and the first definition.

- **DONE** the discharge is pinned.
- **OPEN** the explicit box's row in STATUS.md: every part of it this session
  could probe is built. The row is the chat's to remove, so this is a report.

## 2026-09-17 — a gate reached its verdict and buried it past the cap

kanso#1463 gave the three compile gates a second reading: when a row parts
from its golden, the same binary is counted again in the same job, and the
gate says whether it read the same number (the change moved the row) or a
different one (a reproduction failure, which halts the vein). That is the
right machinery and it worked. Nobody could read what it said.

GitHub keeps **fifty annotations per check run**. Each failing row emits about
a dozen `::error::` lines, and the cross-run thirteen parts all three compile
rows at once — so the job carries forty-odd lines of explanation and the two
lines naming the verdict fall off the end. kanso#1477 is the cleanest case:
its whole diff is one golden fixture and a log entry, it cannot reach the
compiler at all, its three rows each read −13, and the job could be read as
far as `a move of -13` and no further.

The verdict now comes after the three lines naming the row and the size of the
move, and before the explanation of the two cases. `<row>_again` also goes out
as a `::notice::`; it was a `printf` to stdout and an append to the artifact,
and both of those have to be fetched, where an annotation comes back over the
ordinary API. `codegen_instructions.sh` took the same pair of fixes on
claude/codegen-rows earlier today, for the same reason found the same way.

`tests/a_gate_says_its_verdict_before_it_explains.rs` pins both, reading the
four gates off disk. Watched red on each half: moving the verdict block back
to the end of `entry_instructions.sh` names it at byte 9781 against an
explanation at 7209, and deleting the notice from `library_instructions.sh`
names that file.

**The thirteen itself, four pairs deep.** Every pair is two sittings of one
binary with the floor identical to the digit and the rows exactly thirteen
apart:

    kanso#1469   558610/605 twice        rows 13 apart
    kanso#1465   558232/604 twice        rows 13 apart
    kanso#1468   558726/605 twice        rows 13 apart
    kanso#1474   558610 -> 558620, +10   rows -13

and kanso#1477 adds the cleanest demonstration that it is not any diff: a
golden fixture and a log entry moved all three rows by thirteen.

- **DONE** the verdict is inside the cap.
- **OPEN** what the verdict says. The gates are armed on every branch carrying
  main; the next parting row answers from inside its own job.

## 2026-09-17 — eight runs of one binary, identical to the instruction

The thirteen had never been attempted locally. It has now, with the compile
gate's own box, its own command and its pinned tunables:

    run 1: 36384683      run 5: 36384683
    run 2: 36384683      run 6: 36384683
    run 3: 36384683      run 7: 36384683
    run 4: 36384683      run 8: 36384683

Eight sittings, one binary, one container, one corpus. Identical to the
instruction. **So the cross-run thirteen is not two runs of one binary**, and
`kanso#1463`'s second reading — which counts the same binary again inside the
failing job — should print VERDICT (1) every time.

What is left is two BINARIES. `compile_instructions.sh` has carried the reason
in its own header since it was written: cargo builds are not bit-reproducible,
a binary whose data and bss differ starts the heap at a different break, and
the 508 that row once read was exactly that — sha 55fb850296d1 counted
41,831,767 and sha de5bfab22fbd counted 41,832,275. Every CI job builds its own
compiler. Two sittings of one commit are two builds.

The sha is already printed on every run, green or red, put there so a hash
could be paired with a value. It goes to stdout, and stdout reaches only the
job log, which is the one artifact unreachable from a session reading over the
API. Three notices fix that, and the spec pins them.

That makes the next pair decisive either way. Two sittings of one head thirteen
apart with DIFFERENT shas is the answer: the row moves with the binary and the
fix is a reproducible build, not a compiler change. The same shas would refute
it and leave something genuinely unexplained inside `kanso::main`.

- **DONE** eight runs say it is not the run.
- **OPEN** whether it is the build. The sha notice answers it on the next pair.

**Round four: the warm-up holds.** CI read the start-up row 4,876,986 and then
4,876,986 again in the same job, where the round before it read 6,018,427 and
then 4,869,632. The row was counting a cold runtime-object cache and now it
counts the compile. It landed on a runner of yet another generation — AMD
family 0x1a model 0x2, where the last two rounds were 0x19 model 0x1 and model
0x11 — and read the same number twice there, which is the fleet answering the
CPU-model account that kanso#1466 retired.

On the head merged with kanso#1464 the three compile rows read 36,862,211,
131,827,785 and 131,970,557 against main's 36,864,779, 131,837,650 and
131,978,823. That fall is the declares scan this branch removes.

## 2026-09-17 — CI's sitting of the merged tree: the start-up change, priced

kanso#1461 merged with main after kanso#1459 landed. The merge carried
kanso#1459's values forward; CI read the merged tree below them:

    compile_instructions    36,682,232 -> 36,679,432   -2,800   -0.0076%
    entry_instructions     130,573,787 -> 130,564,072  -9,715   -0.0074%
    library_instructions   130,716,747 -> 130,707,970  -8,777   -0.0067%

**Not the change doing less work here.** DONE. `kanso check` does not run the
interpreter, so the start-up scan this branch removes is not on this corpus at
all. All three are the layout kind the row's header describes: src/eval.rs is
the compiler's own bytes, and editing it moves them and what sits around them.
The tenth such move recorded on this row.

**The floor does not move.** DONE. welfare reads 69.76 against a floor of
69.76. The interpreter start-up row is this branch's own vein and it is the one
the change is for; these three are collateral.

## 2026-09-17 — kanso#1461 on the merged tree: CI's sitting

The branch merged with kanso#1472 (the allocator's alignment) and CI measured
the merged tree. Four rows moved against the values the merge carried forward:

    startup_instructions   4,876,986 -> 4,838,300   -38,686   -0.793%
    compile_instructions  35,969,565 -> 35,966,784    -2,781   -0.0077%
    entry_instructions   128,144,579 -> 128,134,739   -9,840   -0.0077%
    library_instructions 128,281,268 -> 128,272,434   -8,834   -0.0069%

The start-up row is the branch's own: the interpreter stops asking the declares
scan once per name. The three compile rows are the layout kind — `kanso check`
does not run the interpreter, so none of the work this branch removes is on
that corpus, and what moves them is src/eval.rs being bytes the compiler
carries. Welfare rose and is banked at 69.79226700979217.

`per_process_floor=558659 frames=605 kernel=6.17.0-1022-azure cpu=26/2`. A
fifth distinct floor reading, and the floors are only comparable between two
sittings of ONE binary — every branch links a different compiler, so a floor
that differs across branches says nothing. The pair that matters is still two
sittings of one head.

- **DONE** the rows are CI's.

## 2026-09-17 — kanso#1461 on the merged tree, and the rows read the right way

CI's sitting on the tree merged with kanso#1465:

    compile_instructions  35,967,913 -> 35,965,137    -2,776
    entry_instructions   128,214,733 -> 128,204,898    -9,835
    library_instructions 128,348,838 -> 128,340,017    -8,821
    startup_instructions   4,838,300 -> 4,838,323        +23

**The three compile rows are LAYOUT.** This branch changes `src/eval.rs`, the
interpreter, and `kanso check` never runs it. The entry above on this row said
so and was right; two entries written later the same day said "work removed
from the corpus" about changes that could not run either, and both have been
corrected. The grep that settles it takes a second: find the callers of the
changed function, and if they all sit under `emit_ir` or under the run path,
the compile corpora never reached them.

`startup_instructions` is the row this branch is for, and it is a
re-measurement rather than a regression: the previous 4,838,300 was read on a
different tree. The 13.8x fall is in the value either way.

Welfare rose and is banked at 69.79153807658396. A rise is banked whatever
moved it; what moved this one is the compiler's bytes, and whether the ratchet
should be banking that at all is the open question in the ledger.

- **DONE** the rows are CI's, and their cause is named correctly.

## 2026-09-17 — the thirteen is in bucket zero, and bucket zero has a candidate

Two sittings of this head, same compiler source, printed frame digests that
differ in exactly one bucket:

    a8995bb9  b0=1765679/40    rows thirteen under the golden
    e7a6b980  b0=1765692/40    rows on the golden, job green

1,765,692 − 1,765,679 = **13**. The other thirty-one buckets are byte-identical
and the frame count is forty in both, so no frame appeared or disappeared: one
frame among forty changed cost by thirteen. Both sittings read
`per_process_floor=558620 frames=605`, the fifth pair with the floor identical
to the digit while the rows part.

Bucket zero is the frames whose NAME LENGTH is a multiple of thirty-two.
Listing them out of a local profile of the same workload gives forty, and one
of them costs **exactly thirteen instructions**:

    13   ./nptl/./nptl/pthread_attr_init.c:pthread_attr_init@@GLIBC_2.2.5

That is the same family as the term Clay ruled out on 2026-09-15, when
`pthread_getattr_np`'s parse of `/proc/self/maps` was found moving this row
with the binary's layout: "this has nothing to do with compiler performance and
obviously shouldn't be part of what we measure." The anchor at `kanso::main`
was the answer to that one, and it drops the stack guard placed above it. This
is thread-attribute setup reached from inside `kanso::main` instead.

It is a candidate, not the answer. Two things are unchecked: whether CI's
bucket-zero frames are the forty this box lists, and whether the thirteen in
CI's digest is this frame's cost appearing and disappearing or another frame in
the bucket moving by the same amount. The step below settles both — the forty
frames of bucket zero, by name and cost, in one notice.

- **DONE** the bucket is named, and the digest earned its place doing it.
- **OPEN** the frame. One line in the next pair of sittings.

## 2026-09-17 — the codegen row reproduces, and the last term was a pipe

The warm-up fix landed and the sitting said so. `codegen_procs` now prints the
same five processes for both readings — `kanso clang:probe clang clang ld` —
where the first reading used to see six, and the dev row went from
`9,273,832,677 then 1,071,605,357` to `1,071,604,729 then 1,071,604,609`. From
a ninefold gap to 120 instructions in 1.07 billion.

120 is not zero, and the vein is compared exactly, so the gate still halted.
Two things were left and only one of them was what I expected.

**The pid, which was real and was not it.** Five temp paths in `src/main.rs`
interpolate `std::process::id()`, and a pid is one digit to seven. A path's
LENGTH is a term in what a process costs — the bytes are copied, walked and
handed to `open` — so two runs wrote paths of different lengths and counted
different instructions for identical work. `pid_tag()` pads to seven, which
covers every pid under the default `pid_max` of 4,194,304 and keeps every one
of them distinct. Two specs: the width, beside the function, and that every
site goes through it, so a path added later cannot put the variance back.

It did not fix the row. Two passes with the padding read 1,016,046,470 and
1,016,048,745 — 2,275 apart, further than before rather than closer.

**What it actually was.** Diffing the two passes frame by frame: EIGHT frames of
ten thousand six hundred and forty-two differ, and together they are the whole
of the 2,275.

```
  +903   FileDesc::read_to_end
  +591   default_read_to_end::small_probe_read
  +253   process::unix::common::read_output
  +220   read
  +176   __memcpy_avx_unaligned_erms
   +88   poll
   +44   __errno_location
```

Every one of them is `Command::output()` draining a child's pipes, and how many
`poll` and `read` calls that takes depends on when the child's bytes arrive.
The only caller was `preserve_none_probe`, which reads `status.success()` and
throws the output away. With the child's streams sent to null and `status()` in
place of `output()` there is no pipe and no loop.

```
before   1,016,046,470   1,016,048,745                                   2,275 apart
after    1,016,035,480 x 5                                                    exact
```

Five passes after the change, each re-staging the box and warming both tiers
first, and every one reads the same digits. The row falls about eleven thousand
besides, which is the pipes' own cost leaving the count.

`src/main.rs` now holds no `Command::output()` at all.
`tests/the_compiler_never_drains_a_childs_pipes.rs` pins that and the probe's
null streams; `src/eval.rs` keeps its `output()` and must, because `os/run`
hands a kanso program the child's stdout and stderr, and no compile row runs a
kanso program's `os/run`.

**Guessing cost a round again, and the instrument paid for itself again.** The
pid was a good hypothesis, it was measured, and it was wrong about this row.
What settled it was the same move that named the thirteen: diff the frames and
read what differs.


**And the queue was mostly reading commits nobody would look at.** Counted the
same afternoon: twelve pull requests open, twelve latest runs, all `queued` —
and ELEVEN of them on a head the branch had already moved past, three of them
on one branch at once. Every push had left its predecessor running. The eleven
were cancelled by hand, and `ci.yml` grows a `concurrency` guard so a newer head
cancels the older one, keyed off the pull request number and deliberately NOT
applied to `main`: a push there writes the perf history row the trend chart
reads, so it is the record of that commit rather than a draft of the branch.

- **DONE** the codegen row reproduces exactly on this box, five times.
- **OPEN** the golden. This host refuses the recorded toolchain, so the values
  are CI's to take on the first green sitting.

## 2026-09-17 — the stack-slot check reads the first space, and the lever was a tenth the size advertised

`FnEmit::write` diverts every `alloca` to the head of the entry block so LLVM
does not keep a frame pointer for the function, and it recognised one by
searching the whole line for ` = alloca `. A slot reads `%name = alloca <type>`
and `%name` holds no space, so the needle begins at the line's first space or it
is nowhere. Measured on runbench's 36,086 emitted lines: all 235 slots put it
between offsets five and seven, and each of those is that line's first space.

```
kanso build pkg/runbench, kanso's own process
  before  31,280,056,421
  after   31,279,771,548   -284,873
```

IR byte-identical, 1,250,754 bytes.

**And the figure this was chosen on was wrong.** The 2026-09-16 build profile
put 18.2 million instructions over 144,261 lines against this search, about a
hundred and ten a line. The measurement above is eight a line. The 18.2 million
was an inclusive cost read as a self cost — the same mistake the hand-written
caller-tree parser made on 2026-09-15, arrived at by a different route. A lever
priced from a profile is a hypothesis; this one was worth a sixty-fourth of its
price and is recorded at what it is.

It is still worth having: free, exact, and pinned. `kanso check` stops before
codegen, so no welfare term can see it at all until the codegen rows land on
kanso#1470.

`tests/a_stack_slot_is_found_where_the_first_space_is.rs` compiles the module
fixture, runbench and a list-literal sample and asserts the narrow reading and
the whole-line one agree for every emitted line — 235 slots among them, so the
agreement is not between two functions that both said no. Watched red with the
reading pinned to offset zero; it named `%t77 = alloca [2 x %KValue]`.

- **DONE** the check is narrow and the two readings agree over real IR.
- **NOTE** a profile's inclusive cost has now mispriced a lever twice in three
  days. Price a frame from its SELF cost, or build it and measure the whole.

## 2026-09-17 — the compile profile by SELF cost, and the map keys measured at last

The entry above says to price a frame from its self cost. This is that profile,
taken the same afternoon on main: `kanso check compile_corpus`, 36,830,740
instructions, staged in the box the gate uses.

```
  1,636,363  4.44%  hashbrown HashMap::insert
  1,476,482  4.01%  infer::eval_expr'2
  1,435,071  3.90%  hashbrown rustc_entry
  1,165,325  3.16%  check::check_after_infer
  1,158,417  3.15%  infer::infer
  1,149,825  3.12%  __memcmp_avx2_movbe
  1,124,080  3.05%  check::check_merged_after_aliases
  1,043,583  2.83%  RawTable::reserve_rehash
    985,190  2.67%  lexer::lex_line
    755,027  2.05%  infer::eval_expr
    692,234  1.88%  parser::parse
    680,372  1.85%  check::per_node_walk'2
    656,191  1.78%  __memcpy_avx_unaligned_erms
    616,257  1.67%  lexer::lex
    603,907  1.64%  mi_free
```

Hash tables come to 17.9% with the lookups added — insert, rustc_entry,
reserve_rehash, contains_key, get_mut, get — and `__memcmp_avx2_movbe` at 3.12%
sits underneath them, which is what comparing string keys costs on a collision.
The allocator adds 5.1%.

**The map keys are the question the 2026-09-14 entry left open, and here they
are.** That entry recorded kanso#1033 declining an interned symbol for the AST's
own field at 365 conversion sites, and said in the same paragraph that the MAP
KEYS are a different question nobody had measured. Measured now, by caller:

```
  428,500  1.16%  < RawIterRange::fold_impl        (2,180 calls)
  291,898  0.79%  < qualify                        (1,531)
  218,600  0.59%  < Resolver::flush_unused           (831)
  181,262  0.49%  < Map::fold                        (868)
  169,058  0.46%  < bound_in_pattern               (1,454)
  151,997  0.41%  < HashSet IntoIter::fold           (750)
  148,308  0.40%  < compile_module_loaded'2          (797)
  135,045  0.37%  < Vec SpecFromIterNested::from_iter (519)
  126,238  0.34%  < collect_pattern_names          (1,302)
  115,956  0.31%  < fuse_enumerable                  (588)
```

Twenty-four more callers below these, none above 0.10%. So the keys are the
same shape as the rehash lever: one habit repeated in thirty places, largest
1.16%. Interning reaches all of it from underneath, which is the case for
doing it, and it lands on thirty sites across check.rs, infer.rs, name.rs and
codegen.rs, which is the case for not doing it while fourteen pull requests
are open against those files.

`reserve_rehash`'s own callers are `HashMap::insert` over 1,171 rehashes and
`rustc_entry` over 295. (The `phase::watched` rows the caller tree prints at
20.93% and 19.93% are inclusive chains — the whole compile passes through them
— and are not attributions. Reading one as a self cost is the mistake the entry
above corrects.)

- **DONE** the map keys are measured; the 2026-09-14 entry's open line closes.
- **OPEN** the refactor itself, and it wants a quiet tree.

## 2026-09-17 — kanso#1482's four rows, merged with main and measured by CI

The stack-slot branch merged main and the conflict resolution carried main's
four instruction rows forward, so the tree was reading numbers no sitting on it
had produced. CI's sitting on the merged head:

```
  compile_instructions   35,965,137 -> 35,965,230     +93
  entry_instructions    128,204,898 -> 128,205,462    +564
  library_instructions  128,340,017 -> 128,340,528    +511
  startup_instructions    4,838,323 ->   4,837,367    -956
```

All four are LAYOUT. `kanso check` stops before codegen and this branch changes
src/codegen.rs alone, so nothing any of these rows counts as work went near the
change; what moved them is the compiler binary carrying different bytes. The
signs say the same thing — three up, one down, no direction.

Welfare weighs the first two and not the other two, so the change costs +657
summed compile instructions. The dead band is ±0.001 points, about 105,000
summed, so the objective does not move and there is nothing to bank. The gate
agrees: it exits 0 with the value and the floor both reading 69.79.

What the branch buys is 284,873 instructions off `kanso build bench/runbench`,
and no vein on main counts that yet — the codegen rows are kanso#1470's. So
this is a change whose cost is measured and whose gain is not, until that lands.

- **DONE** the four goldens carry CI's rows; eight page spans follow them.
- **OPEN** kanso#1470's codegen rows, which would put a number on the gain.

**CI's sitting on the head merged with main after kanso#1479.** Five rows move
and only one of them is this change.

```
interp_instructions   2,178,656,557 -> 2,178,559,085    -97,472   -0.0045%  WORK
compile_instructions     35,965,137 ->     35,969,617     +4,480   +0.012%  LAYOUT
entry_instructions      128,204,898 ->    128,218,761    +13,863   +0.011%  LAYOUT
library_instructions    128,340,017 ->    128,352,943    +12,926   +0.010%  LAYOUT
startup_instructions      4,838,323 ->      4,838,372        +49   +0.001%  LAYOUT
```

The interpreted row is the change — this branch's whole compiler diff is
`src/eval.rs` and one new `interpreter_counters` in `src/main.rs`, called only
from the interpreter's `Drop` under `KANSO_COUNTERS`. `kanso check` runs
neither, so the other four are the binary's bytes moving.

**And the objective passes.** The four layout rows put 18,343 on the welfare
term — 0.011% of 164,188,378, which at 9.49e-9 a point is 0.00017 — and
`verdict` has a dead band of 0.001 either side of the floor, so a move this
size neither fails nor asks to be banked. Worth writing down because the four
rows look alarming and the score does not move.

## 2026-09-17 — kanso#1477's three rows take the thirteen, and the fourth does not

CI's sitting on the merged head:

```
  compile_instructions   35,965,137 -> 35,965,150     +13
  entry_instructions    128,204,898 -> 128,204,911     +13
  library_instructions  128,340,017 -> 128,340,030     +13
  startup_instructions    4,838,323 ->   4,838,323       0
```

Three rows move by the same thirteen and the fourth does not move at all.
That pattern names itself: the thirteen lives in `core::slice::memchr::memrchr`
under `LineWriter`, seeking the last newline in the result line each of the
three gates' own runs prints. The start-up gate prints nothing, so it has no
thirteen to draw.

Which side of the thirteen a given binary lands on is a property of its layout.
kanso#1483 stops the measured runs printing that line, and when it lands the
three rows lose the thirteen and the whole family of moves with it.

Welfare weighs the module and entry rows, so this costs +26 summed compile
instructions against a dead band of about 105,000. The objective does not move.

- **DONE** the three goldens carry CI's rows; two page spans follow them.
- **OPEN** kanso#1483, after which this row family stops drawing lots.

## 2026-09-17 — the rows are a coin, and the thirteen is not a branch's to bank

The entry above wrote CI's sitting into this branch's three rows. Its next
build measured the numbers it had just replaced, so the rows are reverted and
this correction stands in their place.

Five builds today, on trees whose compiler source is byte-identical apart from
kanso#1482's:

## 2026-09-17 — the floor is bimodal, and the gap is exactly ten

Three measurements today, and the third is the one to keep.

**The build reproduces here.** Four clean rebuilds of one tree in this
container, each preceded by `touch src/main.rs` so nothing was cached:

```
n   sha256           .text     .data   .bss    kanso::main
1   b8a64fe29f820c63 2841218   12664   29912   36377641
2   b8a64fe29f820c63 2841218   12664   29912   36377641
3   b8a64fe29f820c63 2841218   12664   29912   36377641
4   b8a64fe29f820c63 2841218   12664   29912   36377641
```

So `cargo build --release` is bit-reproducible where the toolchain, the path
and the environment hold still, and the row follows the binary exactly.

**Two builds on CI disagree in their sha and agree on every row.** Run
35197408041 was re-run on the same commit, 274c89ca, whose whole diff is three
gate scripts, one test, the log and one page — nothing `include_str!`'d, nothing
the compiler carries. Attempt one built sha `fde1fb87…` on cpu family 25 model
17; attempt two built sha `13e6cf22…` on family 25 model 1. The three compile
rows read 35,967,926 / 128,214,746 / 128,348,851 on both, to the instruction,
and the floor read 558232/604 on both. That refutes the CPU model for the third
time, now within one commit, and it says the sha difference CI shows between two
builds is not a difference the count can see. The sections would say which part
of the binary moved; they are printed and were not readable, which this change
fixes.

**The floor takes two values, ten apart.** Sixty sittings across eighteen
branches, grouped by branch and by the floor's own frame count:

```
claude/name-spaces       frames=[604]  floors=[558222, 558232]          gap 10
claude/welfare-split     frames=[605]  floors=[556282, 556292]          gap 10
claude/linear-groups     frames=[605]  floors=[556432, 556442, 558665]  gap 10
claude/group-indices     frames=[605]  floors=[558678, 558688]          gap 10
claude/beat-indexed      frames=[605]  floors=[556457, 558690, 558700]  gap 10
claude/prune-indexed     frames=[605]  floors=[558716, 558726]          gap 10
claude/declares-scan     frames=[604,605] floors=[558259, 558649, 558659] gap 10
claude/codegen-rows      frames=[604,605] floors=[558232, 558610, 558620] gap 10
claude/self-dump         frames=[604,605] floors=[558232, 558610, 558620] gap 10
main                     frames=[604,605] floors=[558232, 558610, 558620] gap 10
```

Every branch that sat more than once and did not change its frame count shows
exactly two floors, ten apart. The other gaps in that table — 378, 390, 2223,
2233 — are commits that changed the compiler. Ten is not one of those: it
recurs on ten branches with unrelated diffs, at four different absolute values.

It is not the thirteen. The floor is the set of frames whose self cost held
across all three workloads, and a row moving by thirteen while the floor moves
by ten in the other direction is two facts, not one. What it is, is the first
property of these sittings that is BIMODAL rather than noisy, and a two-valued
flag is a thing that can be chased. The frame that carries the ten is named the
same way kanso#1474 names the thirteen: bucket the floor listing and diff two
digests.

- **DONE** the sections join the sha as notices, on all three compile gates,
  pinned by `tests/a_gates_binary_is_described_where_it_can_be_read.rs`.
- **DONE** the build reproduces in this container, four for four.
- **OPEN** what costs exactly ten. The next pair of sittings that straddle the
  two floors has the digest to name it.

## 2026-09-17 — the thirteen is `memrchr`, called from the line the gate prints

Two sittings of this branch, one commit apart. The commit between them is
`cargo fmt` over one test file: no compiler source, nothing `include_str!`'d,
and both sittings printed
`compile_binary sections .text=2803570 .data=12672 .bss=29912`, byte for byte,
on the same runner family and model.

```
compile_instructions   35,967,913   ->   35,967,926      +13
entry_instructions    128,214,733   ->  128,214,746      +13
library_instructions  128,348,838   ->  128,348,851      +13
per_process_floor         558,232   ->      558,232        0
```

kanso#1474's digest, diffed:

```
frame_digest   1 bucket differs:  b0 = 1,767,181/40  ->  1,767,194/40   +13
floor_digest   0 buckets differ
frame_bucket0  1 frame of 40 differs:
               core::slice::memchr::memrchr   185  ->  198   +13
```

One frame. `core::slice::memchr::memrchr`, and its caller on this box is
`<std::io::stdio::StdoutLock as std::io::Write>::write_all`, twice, for 185
instructions. That is `LineWriter` looking backwards for the last newline in
what the process printed — and what `kanso check` prints is one line, nineteen
bytes: `compile_corpus: ok`.

So the thirteen has never been the compiler. It is the cost of writing the
gate's own result line, and it moves with the alignment of a heap buffer rather
than with anything the front end decided. The floor could not name it because
the floor is the set of frames whose self cost holds across all three
workloads, and this one holds across all three — at two values.

**Why it took this long.** Every earlier round asked what differed between the
two readings and found nothing: same sha, same sections, same CPU, same
floor, same kernel, same toolchain, and eight within-binary runs agreeing to
the instruction. All of that was true and none of it was the question. The
question was WHICH FRAME, and nothing printed a per-frame listing until
kanso#1474. The instrument named it on its second pair.

**What it costs.** `kanso::main` inclusive is the anchor, chosen on 2026-09-04
so the row counts the compiler's own work and not the loader's; printing the
result is inside that frame and is not compiling. The 2026-09-15 rule is the
one that applies: external state is normalized before it is measured, and a
term that cannot be normalized is excluded with the exclusion named in the
golden's header. Buffer alignment cannot be normalized from here. So the print
comes out of the measured region, which re-baselines all three rows at once and
is its own change.

**Demonstrated, not inferred.** The frame's cost scales with what the process
prints. Same binary, same box, one `kanso check` against two corpora whose only
difference is the length of the name that goes into the printed line:

```
compile_corpus                           memrchr = 63
compile_corpus_with_a_much_longer_name   memrchr = 81
```

Twenty-four more characters on the line, eighteen more instructions in the
frame. (`kanso::main` moved 14,177 the other way on that pair, which is the
path-length term this file's header already carries at about 160 instructions a
character; a different effect, an order of magnitude larger, and not this one.)

**And the control holds.** Run 35204603108 was re-run on one commit,
`ef0a028e`: two attempts, two CPU models (25/1 and 25/17), and every printed
quantity byte-identical — the three rows, the floor at 558338/604, all
thirty-two digest buckets, and all forty frames of bucket zero including
`memrchr`. So within a commit the measurement reproduces exactly; the thirteen
appears only BETWEEN commits, which is where the binary can move under a frame
whose section sizes do not.

- **DONE** the frame is named, with the digest diff that names it, and its
  cost is shown to follow the printed text.
**What is still open underneath it.** A test-file-only edit does not rebuild
`target/release/kanso`: the sha was `b8a64fe29f820c63` before and after adding
a comment to a test crate, in this container. And four clean rebuilds of one
source here give one sha. So the two sittings above should have been built from
the same bytes, and the frame that carries the thirteen should not have moved.
Whether CI's two independent builds of one source actually agree is the
question, and kanso#1479's `compile_binary sha256=` notice answers it the next
time a pair parts — which is the reason to land that one first.

- **OPEN** whether CI's two builds of one source are the same binary. One line
  in the next pair, once kanso#1479 is on main.
- **OPEN** taking the print out of the row. Three welfare-weighted rows
  re-baseline together, so it lands on its own with its own sitting.

## 2026-09-17 — the compile rows stop counting the line the run prints

Five CI builds today, across trees whose compiler source is byte-identical:

```
  main            ae5183a8   35,965,150   128,204,911   128,340,030
  kanso#1477      716fcfc4   35,965,150   128,204,911   128,340,030
  kanso#1481      e005650f   35,965,150   128,204,911   128,340,030
  kanso#1477      8e4e5665   35,965,137   128,204,898   128,340,017
  kanso#1485      4c5b282b   35,965,137   128,204,898   128,340,017
```

Two faces, thirteen apart on every row, and `startup_instructions` reads
4,838,323 on all five. Within a build the reading is exact: every job's
`<row>_again` has matched its first.

Neither way of not printing helps, because both change the process the gate
measures. On one box, `kanso check compile_corpus`:

4,838,323 on all five. Within a build the reading is exact — every job's
`<row>_again` has matched its first. A re-run of main's own failed job, a
second build of the same source, came back green on the other face, which is
the control.

kanso#1485 tried writing one face into the goldens and its own build measured
the face it had just replaced. So a value cannot settle this.

`kanso check` prints one line when it finishes and `kanso::main` inclusive
counted it. Under it `LineWriter` runs `core::slice::memchr::memrchr` over the
formatted bytes to find the last newline, and what that frame costs moves with
the binary's layout. The start-up gate prints nothing, which is why it never
drew.

Not printing costs more than it saves, because both ways of asking change the
process the gate measures:

```
  env -i, two variables, printing      36,817,649
  env -i, three variables, printing    36,829,255   +11,606
  env -i, three variables, KANSO_QUIET 36,828,139    -1,116
  two variables, printing              36,817,388
  two variables, --quiet               36,818,319      +931
```

The environment variable costs ten times what the quiet saves — the compiler
asks getenv about seven thousand times and each ask walks the block — and an
argv entry costs about twice it. So kanso#1483 as built is a regression.

What is left is the 2026-09-15 rule: a term that cannot be normalized is
excluded and the exclusion is named in the golden's header.
`std::io::stdio::_print` is reached once per run, from `kanso::driven`, and
`core::slice::memchr::memrchr` has no other caller in a `kanso check`.
`claude/row-excludes-the-print` subtracts that subtree from `kanso::main`
inclusive, which should map both faces onto one row.

- **DONE** the rows here are back to main's, and this branch waits on that one.
- **OPEN** whether the exclusion holds across builds, which its own CI answers.

The compiler asks getenv about seven thousand times and each ask walks the
environment block, so a third variable costs ten times what the quiet saves;
an argv entry costs about twice it. Both move the initial process layout,
which is the same class of thing the thirteen is. kanso#1483 as built is a
regression on its own row and does not land.

What is left is the 2026-09-15 rule: a term that cannot be normalized is
excluded and the exclusion is named in the golden's header. The three gates
subtract `std::io::stdio::_print` inclusive from the anchored reading, on both
of their measured runs. `_print` is reached once per run, from
`kanso::driven`, its whole subtree is the line — 828 instructions on the
profile this was read from, with `memrchr`'s 133 inside it — and nothing else
in a `kanso check` prints to stdout, because diagnostics go to stderr.

A spec reads the three gates off disk, finds the six readings that become a
row, and fails on one that does not take the print off. Watched red with the
entry gate's second reading put back the old way, which it named by line.

- **DONE** built; the exclusion parses out of a real profile here (828).
- **OPEN** whether both faces land on one row, which this branch's CI answers,
  and the three rows it re-baselines once they do.

## 2026-09-17 — the excluded row, measured

CI's sitting with the printed line taken out of the anchored reading:

```
  compile_instructions   35,964,325
  entry_instructions    128,204,133
  library_instructions  128,339,261
  startup_instructions    4,838,323   (unchanged, and green throughout)
```

Against the two faces the rows had been drawing, the excluded subtree is:

```
            low face      high face     subtracted
  compile   35,965,137    35,965,150    812 / 825
  entry    128,204,898   128,204,911    765 / 778
  library  128,340,017   128,340,030    756 / 769
```

Thirteen apart in each pair, which is the thirteen — it was inside the
subtree, as `startup_instructions` and the frame dumps had said. Which face
this build drew is not knowable from one sitting, and the thing that settles
it is a second build reading 35,964,325 again.

The gate now emits the excluded amount as a notice (`compile_printed=`,
`entry_printed=`, `library_printed=`). A number that only ever appears
subtracted cannot answer the first question a future drift raises, which is
whether the printed line's own cost moved.

Welfare weighs the module and entry rows and they fall 812 and 765 together,
1,577 against a dead band of about 105,000. The objective does not move, and
this is a change in what is counted rather than a gain to bank.

- **DONE** the three rows carry the excluded sitting; one page span follows.
- **OPEN** the second build, which is the whole claim.

## 2026-09-17 — CI's sitting on the codegen rows, and the release tier will not reproduce

The first sitting this branch has taken with both codegen rows in the job.

```
  compile_instructions      35,964,325 ->     35,964,307      -18
  entry_instructions       128,204,133 ->    128,203,909     -224
  library_instructions     128,339,261 ->    128,339,061     -200
  startup_instructions       4,838,323 ->      4,837,892     -431
  codegen_instructions_dev 9,280,351,472 -> 1,003,426,243
```

The four compile rows are a layout move and nothing else. This branch adds
ninety lines to `src/main.rs`, the compile rows run `kanso check`, and a check
never reaches the tier flag; `compile_allocs` and compile memory came back
byte-identical beside them.

The dev row's fall is not work removed. 9,280,351,472 was read before the gate
warmed the runtime.c cache under the measurement's own environment, so that
reading paid for compiling runtime.c and this one does not. Counted twice in
one job, byte-identical both times.

### The release tier read two numbers

```
  first    7,239,553,333
  again    7,239,550,228
  apart            3,105
```

VERDICT (2), a reproduction failure, on a tree of about 7.24 billion. The dev
tier was counted across the same pair of runs, on the same staged box, in the
same environment, and came back byte-identical — so the box, its staging and
the warm-up are not the variable. What is left between the two rows is the
tier flag: `-O3 -flto`.

The row is not written. It is read by an exact compare and holds one value,
and either of two faces is a coin.

What the job could not say is which of the five processes moved. It named them
and priced none of them, so the gate prices each one now
(`codegen_procs_release first=[kanso=… clang=… ld=…] again=[…]`), pinned by
`the_codegen_gate_prices_each_process_it_names`, which runs the gate's own
function text against two hand-made profiles rather than a copy of it. Watched
red by putting the old `printf` back: it reads `kanso clang` and says so.

- **DONE** four compile rows, the dev codegen row, and the instrument.
- **OPEN** the release row, which waits on one sitting naming the process that
  moves. Then the choice is to normalize what moves it, or to exclude it and
  name the exclusion in the golden's header under the 2026-09-15 rule.

## 2026-09-17 — the release row's two numbers, named on the first run of the instrument

The gate now prices every process it names, and the answer came off this
container rather than off a CI round.

```
  reading a   kanso=425,656,322  clang=32,184,722  clang=31,624,903
              clang=1,617,293,611  ld=5,145,605,822    total 7,252,365,380
  reading b   kanso=425,656,555  clang=32,184,722  clang=31,624,903
              clang=1,617,293,611  ld=5,145,605,822    total 7,252,365,613
```

Three clang processes and the linker come back byte for byte. All 233
instructions are kanso's own process, and inside it two frames of 1,346
differ: `kanso::build` +189 and `__memcmp_avx2_movbe` +44.

### The memcmp is the process id

`pid_tag()` puts the pid into the names of the emitted `.ll`, the staging file
and the cached runtime object. An earlier round pinned its WIDTH at seven
digits, which fixed the length of every path built from it and left the
content free, so a comparison over those paths stops at a different byte from
one run to the next.

A probe binary with `pid_tag_of` returning a constant was built and both
readings taken again: `__memcmp_avx2_movbe` came back byte-identical and the
total moved 112 rather than 233. The pid is worth 121 of the 233.

### What is left is a wait

The probe's remaining 112 sit in ONE frame of 1,346 — `kanso::build`, self
cost, every callee byte-identical. That is an inlined loop inside `build`
whose iteration count is not the compiler's to choose: `build` spawns clang
and waits for it.

The dev tier is the control. It is the same code waiting on a child that
finishes seven times sooner, and it reproduces exactly across the same pair
of runs, on CI and here.

### What that leaves to decide

The pid is worth fixing whatever else happens: temp names that carry no pid
would take 121 out and cost nothing. What is left is a row whose subject
includes a process that waits for another process, and the 2026-09-15 rule
says a term that cannot be normalized is excluded and the exclusion named in
the golden's header. Two shapes leave every counted thing deterministic:
count the children alone, or give kanso's own half its own row.

- **DONE** the instrument, and the process named.
- **OPEN** the release row, which is not written and will not be until the
  thing it counts reproduces.

## 2026-09-17 — the codegen row counts the processes that do codegen

The finding above leaves one process in the row that does not reproduce, and
the 2026-09-15 rule says what to do with a term that cannot be normalized.

So both tiers count the child tree — the clang driver, the convention probe's
clang, `clang -cc1` and `ld` — and kanso's own process is excluded, with the
two measurements that put it there written into the gate's header. Every one
of those four came back byte for byte across two readings at the release tier,
which is the whole reason the row can hold one value.

`is_kanso` reads the FIRST WORD of a profile's `cmd:` line and nothing else.
kanso's name turns up inside other processes' arguments — the convention
probe's clang compiles `/tmp/kanso_pn_probe_NNNNNNN.ll` — and a rule that
matched anywhere in the line would call that clang the compiler and take a
deterministic 32 million out of the row.
`the_codegen_row_leaves_the_waiting_process_out` runs the gate's own function
text against the five command lines a real build produced. Watched red by
dropping the first-word rule and loosening the pattern: it answers `yes` for
the probe's clang and says so.

Both goldens now hold whole-tree numbers, which is one process too many. They
are left where they are so each gate has a value to fail against, and the
first sitting under the new shape writes them.

What is still owed is kanso's own emitting, which is real compiler work and
should not disappear from the index because the process it runs in also waits.
It wants an anchored row of its own, counted at `codegen::emit_ir` the way
kanso#1487 anchors the compile rows, where no wait is inside the anchor.

- **DONE** the exclusion, its spec, and both headers.
- **OPEN** both rows, which the next sitting writes; and an anchored row for
  what the compiler itself spends emitting.

## 2026-09-17 — what the compiler spends emitting, which nothing counted

Excluding kanso's own process from the codegen rows leaves a hole, and it is
not a small one. The three `kanso check` rows stop before codegen. The two
codegen rows now count the child tree. Between them sits the work this project
wrote — turning a checked program into LLVM IR — and no row reached it.

`emit_instructions` is that row: `codegen::emit_ir` inclusive over
`kanso build pkg/codegen_corpus` in the staged box.

The anchor was chosen because it is exact rather than approximate, and that
was measured before the gate was written. Four profiles were already on disk
from the reproduction work above — two readings of the shipped binary and two
of a probe binary whose `pid_tag_of` returns a constant — and every one of the
four reads the frame at **394,910,642** inclusive, byte for byte, while the
process around it moved 233 and then 112. Two binaries, four readings, one
number. The wait for clang happens in `build` after `emit_ir` returns, so it
is outside the anchor by construction rather than by luck.

The frame is 92.78% of what kanso's own process spends on a release build, so
what the exclusion drops is the wait and very little else.

That number is a CONTAINER's. It is written into the golden's header as
evidence that the anchor is deterministic and it is NOT the row: only CI's
numbers may be recorded, which is what `scripts/gates/measured_on.sh` exists
to enforce. The golden ships with an empty value on purpose, so the gate
refuses with the sitting printed above the refusal and the next round writes
it.

### The prefix list went stale a third time, and was caught this time

`tests/the_compile_sweep_names_every_compile_gate.rs` derives the compile
gates from a list of golden-path prefixes, and its own comment says twice that
the list goes stale when a vein arrives under a new name — once for
`bench/library_*`, once for `bench/codegen_*`, both read late. `bench/emit_*`
is the third, and it is in the list in the same commit that creates the file.

`bench/emitted_golden` was already named there and is a DIFFERENT file: it
counts what the compiler WROTE for the decoder, where this one counts what
writing cost. One letter apart and unrelated, which that file has now had to
say about two pairs of names.

`all_compile.sh` runs it, and CLAUDE.md names it — that line is required by
`the_instructions_name_every_compile_gate`, which is why a pull request from
the compiler lane touches the instructions file at all. The same edit corrects
what that file said about `codegen_instructions`, which was "the WHOLE process
tree" until this morning.

- **DONE** the gate, the golden's header, the sweep, the instructions, and the
  spec's prefix list.
- **OPEN** the row itself, which the next sitting writes.

## 2026-09-17 — kanso#1482's three rows, priced: layout, upward

The first sitting this branch has taken on the anchor kanso#1487 left, so
each number is one value rather than a face of the thirteen.

```
  compile_instructions   35,964,325 ->  35,964,418   +93 (+0.00026%)
  entry_instructions    128,204,133 -> 128,204,697  +564 (+0.00044%)
  library_instructions  128,339,261 -> 128,339,772  +511 (+0.00040%)
  startup_instructions    4,838,323 ->   4,837,367  -956 (-0.0198%)
```

`compile_allocs` came back 27,397 and compile memory byte-identical, so no
decision the compiler makes changed.

Three of the four worsened and they are named here because the trend gate
asks for that and it is right to: a row that moves without a sentence is the
thing the rule exists to catch. What moved them is the binary's layout. The
change is the emitter's stack-slot check reading the line's first space
instead of searching it for a substring, and the three rows above count
`kanso check`, which stops before codegen. The pass cannot run on any of
them.

The row the change is for is start-up, and it falls 956.

welfare weighs 657 of this against a dead band of about 105,000 and does not
move.

## 2026-09-17 — kanso#1462's rows, priced: the compiler compiles with the hasher it changes

CI's sitting on the anchor kanso#1487 left, so each number is one value rather
than a face of the thirteen.

```
  compile_instructions   35,964,325 ->  35,968,792   +4,467 (+0.0124%)
  entry_instructions    128,204,133 -> 128,217,983  +13,850 (+0.0108%)
  library_instructions  128,339,261 -> 128,352,174  +12,913 (+0.0101%)
  interp_allocs            5,313,431 ->   5,313,434       +3
```

`interp_instructions` came back 2,178,559,085 and `interp_peak_bytes` 933,202,
both on the row; `startup_instructions` was green at 4,838,372, `compile_allocs`
at 27,397, and compile memory byte-identical.

The three `kanso check` rows rise together and by about the same fraction,
which is what this change's shape predicts: the interpreter's hasher is part
of the compiler, so the compiler's own maps are built with it. A hash that
is cheaper to compute and worse at spreading costs a little more in a map
that is read many times per entry, and the compile corpus is exactly that.

The three allocations are not explained here. Peak bytes and the instruction
row both came back unchanged, so nothing about the shape of the interpreted
run moved.

welfare weighs 18,317 of this against a dead band of about 105,000 and does
not move.

- **DONE** the four rows, priced.
- **OPEN** nothing; the branch is CI's to confirm.

## 2026-09-17 — the bound discharge, merged onto the excluded row

kanso#1487 landed, so the three `kanso check` rows now exclude the line the
run prints and read 35,964,325 / 128,204,133 / 128,339,261 on main. This
branch changes no compiler source — a micro golden and the entries above —
so it takes those rows as they stand and owes no regeneration of its own.

The two earlier entries here are superseded by that: the thirteen this branch
drew, and the revert of the row bump that chased it, were both the frame
kanso#1487 removed.

- **DONE** merged onto main; the rows are main's.

## 2026-09-17 — the emit row, read three times from three stagings

The gate was run before it was shipped, with `GITHUB_ACTIONS` set so the host
check measures rather than stopping. It stages the box, warms it, counts, and
refuses with the sitting printed above the refusal, which is the bootstrap an
empty golden is for.

Then three readings, each from its own fresh staging and warm-up:

```
  394,912,504
  394,912,504
  394,912,504
```

The four release-path profiles read 394,910,642, and the difference is 1,862.
They are two commands — a different tier, and valgrind instrumenting child
spawns in the first — which is the reason a row is one command rather than a
family of them. Both say the thing the anchor needed to say: repeat the
command and the frame does not move, while the process around it does.

- **DONE** the gate, smoke-run end to end, and the anchor's determinism read
  three ways.
- **OPEN** the row, which CI's first sitting writes.

## 2026-09-17 — the new row's file broke two specs, and a pipe nearly hid them

The gate and its golden were written, `all_pages.sh` was green, and the suite
was run as `cargo test --release 2>&1 | tail -25`. The background task
reported exit 0 and it was believed. **That exit code is `tail`'s.** The same
mistake was made twice in one afternoon, on a suite that was failing both
times, and the second time it was caught only because the gate specs were
re-run one at a time. Redirect and read `$?`; never read a pipeline's exit as
the program's.

What it was hiding, both of them real:

**`every_counter_golden_is_walked_by_the_trend_gate`.** A golden the trend
gate does not walk is one whose regressions arrive unpriced. The gate lists
its veins by hand in kanso, and `bench/emit_instructions_golden.txt` was not
among them. Fixed by listing it beside the two codegen goldens, in the commit
that creates the file — which is what that spec's own comment asks for and
what `startup_instructions` did on 2026-09-16.

**`every_compile_vein_row_has_a_direction`.** It reads every
`bench/*instructions_golden.txt` off disk, asks for exactly one row in each,
and asks that every row's counter be named in one of the trend gate's
`lower_*` tables. The new file matched that glob where the codegen goldens do
not, so it was the FIFTH compile vein the hour it landed, and it had no row
at all: the golden had shipped empty on purpose, so the gate would refuse with
its own measurement printed above the refusal.

That plan does not survive the spec, and the spec is right: a vein with no row
is a vein nothing can diff. So the file carries a value, and the header says
plainly what it is — a placeholder no host will ever compare against, because
there is no `measured-on` line and `measured_on.sh` therefore refuses every
host. A container stops; CI measures, prints and fails without comparing. The
first CI sitting replaces the number and writes the measured-on line under it.

- **DONE** both specs, and the value that satisfies the shape without making
  a claim.
- **OPEN** the row itself, unchanged: CI's first sitting writes it.

## 2026-09-17 — the box those readings were taken on had four runaway spinners

Found by reading `ps` while wondering why a test suite was slow: four
`sh /tmp/cpu_hunt.sh` processes, orphaned to init at 00:02, each burning a
core. Forty-one hours of CPU on a four-core box. The script spawns four busy
loops to measure a compile row under load and kills them at the end; it was
interrupted before the kill, and nothing else was going to.

What that does and does not touch:

**It does not touch an instruction count.** callgrind counts instructions
executed, not time, so every number this log recorded today — the compile
rows, the emit row's three readings, the four release-path profiles — is what
it would have been on an idle box.

**It does touch the wait.** The 233 and then 112 that the release-tier
reproduction moved by live in `kanso::build`'s inlined loop waiting for clang,
and how many times that loop goes round is exactly what a loaded box changes.
So the finding stands — the wait is the scheduler's and not the compiler's,
which is what the dev tier's byte-identical control says independently — but
neither 233 nor 112 is a clean estimate of its size on a quiet machine. CI's
own 3,105 was measured on a runner and is untouched by this.

The script now traps and kills its spinners on EXIT, INT and TERM. A
background loop with no trap is a loop that outlives the reason for it.

## 2026-09-17 — two welfares and a meta, built

The 2026-09-16 gavel's second half. `scripts/welfare/welfare.kso` scored one
number over four terms; it scores three over nine.

    production      run speed 0.45, run memory 0.40, release build 0.15
    development     compile speed 0.30, compile memory 0.08, dev build 0.22,
                    interpreter start-up 0.25, interpreter speed 0.11,
                    interpreter memory 0.04
    meta            0.70 production, 0.30 development, saturating

On the tree this was built from: production 57.11, development 72.59, meta
76.13.

**The interpreted side is priced in Clay's order and nothing else.** "start-time
is vastly more important than speed which is more important than memory usage"
is 0.25, 0.11 and 0.04 — each better than two to one over the next. Start-up is
the largest single term on that side because `kanso test` pays it on every
invocation and production never pays it once, which is the dimension no single
scalar could hold and the reason the split was ruled rather than a
re-weighting.

**The meta saturates, and that is the whole of what the third number adds.** A
linear `a·W_prod + b·W_dev` is algebraically one flat term list — the same
model with every weight multiplied through — so the split would buy nothing
the old single scalar did not already have. Each side enters as its score over
a hundred, `f w = w / (w + 1)` is concave across [0, 1], and the result is
divided by `f 1.0` to put the ceiling back at a hundred.

What that buys is an exchange rate between the two sides that MOVES with where
they stand. The meta's derivative in each side, computed at four positions:

    position                  meta    d/d prod   d/d dev   ratio
    today (0.571, 0.726)     76.13      56.72     20.14     2.82
    level (0.500, 0.500)     66.67      62.22     26.67     2.33
    production ahead (0.9, 0.3)  80.16  38.78     35.50     1.09
    development ahead (0.3, 0.9) 60.73  82.84     16.62     4.98

So today a development gain has to be 2.82 times the production cost in
sub-welfare points to be worth taking — which is what "development speed much
better in exchange for a very small production performance cost" means with a
number on it. Let production run far ahead and that threshold falls to 1.09: a
point of development is then worth almost a point of production, because the
side near its ceiling has little left to earn. A linear meta would hold the
ratio at 2.33 forever whatever either side did, and that is the whole of what
the third number adds.

**The four pre-split weights are renormalised, not carried over.** They summed
to one between them as shares of a single objective, and a share of the
development side is a different quantity. Every ratio the old reasoning argued
for survives: run speed still outweighs run memory, compile speed still
outweighs compile memory better than three to one.

**Carrying them over unrenormalised was the first thing that happened, and
nothing said so.** Production summed to 0.71 and scored 40.97 where it should
have read 57.11 — every term on that side scored a fifth low, and the meta read
the shortfall as production sitting far from its ceiling. The number looked
entirely plausible. So the program refuses now: `balanced?` checks each side
sums to one before anything is scored, and `weighed` sits at the head of
`gauge`'s chain beside the golden pins. Watched red by putting run speed back
to 0.30 — exit 2, naming the rule — and green again restored.

**The floor re-ratchets, as the gavel required.** 69.79 was a reading of a
four-term single scalar that no longer exists, so it is not carried forward;
the meta floor is set from the rescored model in the same change. One floor,
on the meta, because the standing rule that the sum is the objective and the
terms are diagnostics applies exactly as it did before — ratcheting the two
sides separately would re-enable the part-against-whole optimisation that rule
exists to stop.

**The five new counters enter at PARITY.** Baseline equals current, so each
contributes its satiation floor and nothing else, and the meta is above the old
number without one instruction of the compiler having changed. The old rule
that granted a new counter its dimension's standing is gone and was not
revived: a counter joining at parity has headroom a counter granted a high
standing does not, and that difference decided at least one verdict in 2026-09.

**What CI owes this PR.** `kanso check` runs on src/main.rs, which kanso#1470
edits, so the five goldens under this branch are not yet this tree's. CI's
first sitting writes all five goldens AND their five baselines together —
together, because writing the golden alone would leave the baseline behind and
score a host difference as a regression. Parity is preserved when both move,
and the meta stays 76.13.

**`bench/objective_sources.txt` gains five lines and the replay spec covers
them.** None of the five renames and none of them sums, so each is one pair.
Watched red by deleting `interp_peak_bytes`: the spec names that counter and
says the trend gate cannot tell a re-basing of it from a win.

- **OPEN** the meta's 0.70/0.30 and its satiation of 1.0 are priced from the
  gavel's framing rather than from a measurement, which is what the 2026-08-25
  charter leaves to the implementer. The first real trade the two sides
  disagree about is the evidence that would move them, and there has not been
  one yet.

## 2026-09-17 — kanso#1482 on the tree merged with kanso#1462: five rows, all down

    compile_instructions    35,968,792 -> 35,968,171        -621   -0.0017%
    entry_instructions     128,217,983 -> 128,213,972      -4,011   -0.0031%
    library_instructions   128,352,174 -> 128,348,205      -3,969   -0.0031%
    interp_instructions  2,178,559,085 -> 2,178,502,266   -56,819   -0.0026%
    startup_instructions     4,838,372 -> 4,837,381          -991   -0.020%

Nothing worsened. Four of the five are layout: `is_a_stack_slot` is asked from
`FnEmit::write`, which sits under `emit_ir`, and neither `kanso check` nor
`kanso run --interp` reaches codegen at all. src/codegen.rs is the compiler,
so editing it moves the compiler's bytes and what sits around them.

The interpreted row's 56,819 is worth writing down as a scale for that vein.
kanso#1468 moves the same row 237,834 in the other direction in this same
round, from an edit in the same file that likewise never executes on the
corpus. A layout term of tens to hundreds of thousands is what this row has,
and a move of that size on it means nothing on its own.

Start-up is the one corpus where the change does run, because `kanso play`
takes the native path. A one-line program emits few enough lines that the 991
saved and the layout term the other four rows show are the same size, so this
reading does not separate them; both point down and the row takes the number.

- **DONE** the rows are CI's.

## 2026-09-17 — kanso#1480's rows challenged, bisected, and the calibration's blind spot found

kanso#1480 shares one linearity `Analysis` where three were built, takes
51,082,187 instructions off a `kanso build` (−7.77%) with the emitted IR
byte-identical, and raises the two rows welfare weighs by 627,157. It was
escalated as Clay's call with three ways out — lower the floor, reweigh the
objective, or park the branch — all resting on the reading that the rise is
the layout term.

**The challenge.** This tree has measured that term on the anchored frame
twice. `scripts/gates/compile_instructions.sh`'s header carries a 2026-09-04
ladder of seven binaries differing only in code nothing reaches: span 1,028,
with 7,632 bytes of unreachable code moving the frame 402, not monotone in
`.text`. design/pending-gavels.md's `.rodata` entry carries a hundred-function
pair at 5,849 unpinned and identical pinned. kanso#1480 moves
`compile_instructions` 140,122 and `entry_instructions` 487,035, the latter
larger than the 330,496 separating its parent kanso#1478 from main. Two orders
of magnitude above both calibrations is not a thing to wave through, so the
escalation went back with a request to attribute the number rather than a
recommendation to rule on it.

**The bisection, cloud's, within the hour.** On the runner's compiler: 105
added lines in the linearity analysis cost **357** on the module row, and 74
lines rewriting two private functions in the emitter cost **145,472** —
functions reachable only from `emit_ir`, which a check never calls. So the
row moved a hundred and forty-five thousand instructions because code that
does not run on the measured path was rewritten.

**The blind spot is the shape of the perturbation, and the calibration never
covered it.** The seven-binary ladder ADDED functions nothing reaches, which
leaves every existing decision where it was; kanso#1480 REWRITES existing
unreachable functions, which moves what sits around them. Those are different
perturbations, the ladder bounded only the first, and nothing said so. One
`#[inline(never)]` in the same module, measured the same afternoon, cost 1,503
— the same family as the ladder, and three hundred times smaller than a
rewrite.

**So the challenge was right and its conclusion was wrong.** The number was
not established as layout, and saying so is what produced the bisection. What
it was taken to imply — that the rise must therefore be real work on the
check path — does not follow and is false. The correction is recorded here
rather than folded away, beside the one from earlier today about probing for
absence, because both are the same error: an argument from a measurement
whose scope was never checked.

- **DONE** the number attributed, by cloud's bisection, to a rewrite of code
  the measured path does not run.
- **OPEN** whether that makes the welfare fall a cost the project should pay.
  It is not external state under the 2026-09-15 rule — the binary's layout is
  produced by the change — and it is not work on the measured path either.
  design/pending-gavels.md's `.rodata` entry is where that sits, and it now
  has a case its earlier measurements could not make.
- **OPEN** whether the ladder should be re-run with rewrites rather than
  additions, so the calibration bounds the perturbation changes actually make.
  Cloud's.

## 2026-09-17 — STATUS.md counted the build hole twice and the two welfares not at all

The "Ruled, unbuilt" intro is the paragraph cloud reads before choosing what
to build. On 2026-09-17 it read: *Two rows stand on 2026-09-17, the build hole
having come off built (kanso#1447 ...): the explicit box ... and the build
hole, ruled 2026-08-24 and found off this list on 2026-09-16.*

So it named the build hole as having come off and as one of the two standing
rows, in one sentence, and did not name the two welfares at all — which is the
larger of the two rows that actually stand, and the one with four counters
outstanding. The headings below it were right the whole time; only the prose
that counts them was wrong, left behind by the edit that removed the build
hole's row on 2026-09-16.

This is the same failure mode CLAUDE.md's counter bullets are written against:
a count kept in a sentence and maintained by hand goes stale, and the thing it
counts is read off the sentence rather than off the list. Here the cost is that
cloud reads a standing row as retired and a retired row as standing on the one
page whose job is to hold them.

Corrected in place, with the correction recorded in the paragraph so a reader
who saw the old one knows which is which. `tests/the_status_index_counts_the_ledger.rs`
pins the ledger index's three sentences and does not reach this section.

- **DONE** the paragraph names the two rows that stand.
- **OPEN** whether a spec should read this section's headings and check the
  intro's count against them, the way the ledger index is already pinned.
  That spec has caught the ledger's count twice.

## 2026-09-17 — the box row probed, and the probe was wrong twice first

kanso#1477's body reports the explicit-box ruling stale, and kanso#1478 and
kanso#1480 cite that in their "Rulings weighed" paragraphs. Probed against a
release build of the branch tip, the report holds on everything the probe
reached: the `effect` constructor answers a box `bind` and `rescue` take, an
`(err _)` arm matches a bare err anywhere, and the check-time refusal fires in
all three shapes part 3 names --

    print "{boom 0 + 1}"     error[exhaustive]: this can be an err and `+` wants a value
    print "{(boom 0)[0]}"    error[exhaustive]: this can be an err and an index wants a value
    print (add1 (boom 0))    error[exhaustive]: this can be an err and `add1` has no arm for it

-- each naming the position and pointing at an `(err _)` arm.

**The entry is here for the two wrong answers that came first, because they
were the same mistake twice in ten minutes.**

The first pass reported the refusal unbuilt. Its three fixtures each bound the
err to a name, `x = boom 0` then `x + 1`, and the rule reads calls rather than
names. That is deliberate, it is the blind spot the `none` rule has always had,
and `docs/compiler.html` section 71 states it in the paragraph describing the
rule. The probe had found the documented exception and called it a hole.

The second pass corrected the first and added a narrower claim: that section 71
says *chapter 4 says so rather than leaving a reader to find it*, and chapter 4
does not. Chapter 4 does, in the paragraph immediately after the railway
sample: *`share` above is a name, and the checker reads calls, not the names
they are bound to, so the failure rides past `with_tip` at run time and the
endpoint reports it.* The search behind that claim was `grep` for the words
"blind spot", which is a search for a phrasing rather than for a fact.

So nothing is owed on the page or in the book, and `railway.kso` runs because
the chapter says it runs. Both wrong answers reached three surfaces before
being caught -- a STATUS.md row, a pull request body and a comment on
kanso#1480 -- and each correction went to the same three.

**The rule this leaves.** A report that a feature is ABSENT is worth what the
search for it being PRESENT was worth. Running the fixture is not that search;
the fixture only shows what happened, and what was supposed to happen is
written in the section that describes the rule. Read that first, then probe,
and where the probe contradicts the documentation suspect the probe. Both
passes here had the answer one paragraph away.

- **DONE** the constructor, the `(err _)` arm and the check refusal probed
  built, and both wrong answers corrected on all three surfaces.
- **OPEN** what the probe did not reach and the row cannot retire without: the
  710 `xs[i]!` sites, `!` names in lib answering a box, and the two cost levers
  kanso#1477 reports built. Their own pass.

## 2026-09-17 — the exclusion left half a build unweighed, and kanso#1480 found it

`emit_instructions` joins the development side at 0.06, taken out of the dev
build's 0.22, which drops to 0.16.

**What went wrong is a seam between two correct decisions.** The 2026-09-16
gavel put "dev-tier codegen (`-O0`)" on the development side. The 2026-09-15
rule then excluded kanso's own process from the codegen rows, rightly: the
parent sits in a wait loop for clang and the linker, and what that loop costs
is the box's scheduling rather than the compiler's work. But the emitter runs
in that process. So between them the two rulings weighed the children of a
build and left the compiler's own half of it outside every counter.

**kanso#1480 is what showed it.** That branch builds one linearity `Analysis`
where three were built, takes 51,082,187 instructions off a `kanso build` —
7.77% — with the emitted IR byte-identical, and welfare falls 0.01. Not one
objective counter can see the saving: the three compile rows stop before
codegen, start-up runs the emitter on a one-line program, and
`codegen_instructions_dev` excludes exactly the process the saving is in.
`emit_instructions`, anchored at `codegen::emit_ir` inclusive, is the only row
that reaches it.

**The split is score-neutral at parity, which is the point.** Both halves
satiate at 0.5, so 0.22 × ⅔ and (0.16 + 0.06) × ⅔ are the same 0.1467, and the
meta stays 76.13 with production 57.11 and development 72.59. This does not
move the number; it makes a dimension the number was blind to visible, which is
what the gavel asked for and what the exclusion inadvertently undid.

**The weights are the two halves' measured sizes.** On this box a dev build's
child tree is 1.00 billion instructions against the emit's 395 million, near
enough 0.16 against 0.06.

- **OPEN** whether the production side owes the same treatment.
  `codegen_instructions_release` excludes kanso's process too, and the emit is
  the same emit — one `emit_ir` serves both tiers, so a second emit row would
  be the same measurement counted twice. Left unweighed on that side
  deliberately, and recorded here so the asymmetry is a decision rather than an
  oversight.

## 2026-09-17 — the codegen rows measured, and both tiers read twice the same

CI's first sitting on aa57f47e, which is what this branch was opened to take:

    codegen_instructions_dev        596,161,187   again 596,161,187
    codegen_instructions_release  6,826,827,769   again 6,826,827,769
    emit_instructions               382,309,867
    compile_instructions             35,968,794   main 35,968,792    +2
    entry_instructions              128,217,981   main 128,217,983   -2
    library_instructions            128,352,174   main 128,352,174    0
    interp_instructions           2,178,559,085   main 2,178,559,085  0
    startup_instructions              4,837,941   main 4,838,372    -431

**The `again` readings are the result, not the values.** This row's first shape
counted the whole process tree, and two readings in one job differed by 3,105
with every child byte-identical: the parent sits in a wait loop for clang and
the linker, and what that loop costs is the box's scheduling rather than the
compiler's work. Excluding kanso's own process under the 2026-09-15 rule is
what made the row reproducible, and a job that reads 596,161,187 twice and
6,826,827,769 twice is the evidence that it worked.

**The release tier is 11.5x the development tier.** That is what `-O3 -flto`
costs against `-O0`, and it is the reason the 2026-09-16 gavel put them on
different sides: one is paid once per release and the other between a keystroke
and an answer.

**`emit_instructions` is 382,309,867**, the compiler's own half of a build —
what the two codegen rows stopped counting when kanso's process left them. The
container read 394,912,504 from three stagings and 394,910,642 on four
release-path profiles; CI is 12.6 million lower, which is the host difference
its golden's header said no box could compare away.

**Three container placeholders are replaced and none was comparable.** The dev
and release goldens carried 1,003,426,243 and 12,107,507,377, both measured
before the exclusion, and the emit golden 394,912,504 from this host. They
existed so each gate had one value to fail against rather than none; CI's
sitting is what they are for.

**What the branch costs the rows that already existed is two instructions.**
compile +2, entry −2, library and interp byte-identical, start-up −431. The
branch edits src/main.rs, so the rows can move; this is the smallest move the
compile vein has recorded.

- **DONE** the rows are CI's.

## 2026-09-17 — eleven counters do not fit on one plot, so the chart draws eleven plots

Adding `emit_instructions` took the objective to eleven counters, and
`tests/the_chart_palette_is_the_one_that_was_measured` caught what that does to
the drawing before CI did.

**Eleven hues do not fit.** `#c4331f` against `#7a5c00` separates by 3.2 under
simulated protanopia against a floor of 8, and no ordering of the set clears
it: the check measures adjacent pairs, so an ordering that fixes one collision
opens another. Eleven lines inside one lightness band do not have the room, and
a search over candidate hues for the two worst offenders returned nothing.

**So the chart draws one panel per counter.** That is the method's answer past
eight series — small multiples rather than a generated hue — and it is the
right one here for a reason beyond the palette: these counters are on different
scales and in different units, and overlaying `interp_instructions` at 2.18
billion with `compile_allocs` at 27,397 was never a comparison anybody wanted.

With one line to a panel there is no adjacent pair to confuse. The caption
names the counter, and each keeps the hue the panel below and its sparkline
read.

**The palette spec's claim is narrower now, and the entry says so rather than
letting the change pass quietly.** It pinned membership, order AND the CVD
floors those were measured against; it now pins membership and order as the
record, with the floors described as the seven-hue era they were measured in. A
rename or a silent recolour still turns it red. That narrowing is because the
drawing changed, not because a gate was relaxed to fit a palette — and the
spec carries the sentence that brings the floors back the day anything overlays
series again.

An intermediate design is recorded because it was wrong in an instructive way:
three charts grouped by the model's sides, with three lines on production and
eight on development. The eight failed all-pairs, which is the same wall one
step further along. Two, three or one line to a plot works; eight does not,
whatever the grouping.

## 2026-09-17 — three of the split's baselines were this container's, and the objective scored the host

The three welfares came back 1.30 above their floor on a tree whose compiler
nothing had changed. The gain was not in the code.

`bench/welfare_floor.json` carries a baseline per counter, and the split added
six. Three of them were readings this container took:

    codegen_instructions_dev      1,003,426,243   ->   596,161,187
    codegen_instructions_release 12,107,507,377   -> 6,826,827,769
    emit_instructions               394,912,504   ->   382,212,543

The left column is what a container measured; the right is CI's. Each golden's
own header says the two cannot be compared — `codegen_instructions_dev_golden`
puts it as "a different clang and a different machine and is not comparable" —
and the objective was comparing them anyway, as +68.3%, +77.4% and +3.3%.

With the three baselines set to CI's first sitting the score reads 76.13
against a floor of 76.12766770905162. The floor had been right the whole time:
it was banked as the new model's reading of an unchanged tree, and an unchanged
tree reads it exactly once the origins are honest. A rise of 1.30 with no
change behind it is what a wrong origin looks like.

The other three new baselines were already CI's, from main's own goldens, and
sit at parity: `startup_instructions` 4,838,372 against 4,836,950,
`interp_instructions` 2,178,559,085 against 2,178,502,266, `interp_peak_bytes`
byte-identical.

**A new term's baseline and its golden are one sitting or neither is worth
anything.** A term whose origin came from one host and whose reading comes from
another prices the difference between the two boxes, and prices it as though
the compiler had earned it.

- **DONE** the three origins are CI's, and the score sits on its floor.

## 2026-09-17 — the split rebased on kanso#1470's CI sitting

kanso#1470 wrote CI's reading into four goldens and gave
`bench/emit_instructions_golden.txt` the measured-on line it had never carried.
This branch merges that.

**`compile_instructions` worsened and lands at 35,968,173.** Two instructions,
kanso#1470's, and that entry prices them: the branch under it adds counters and
the gates that read them and changes no decision the front end makes, so what
moved is the binary's bytes. The entry row fell by the same two.

The split's own arithmetic is unchanged by the merge: with the three container
baselines corrected, the three welfares read 76.13 against a floor of
76.12766770905162.

- **DONE** rebased, and the score still sits on its floor.

## 2026-09-17 — kanso#1470's CI sitting, and the emit vein's first reading on a runner

    emit_instructions       382,309,867 ->   382,212,543     -97,324   -0.025%
    compile_instructions     35,968,171 ->    35,968,173          +2
    entry_instructions      128,213,972 ->   128,213,970          -2
    startup_instructions      4,837,381 ->     4,836,950        -431  -0.0089%
    library_instructions    128,348,205                   byte-identical
    interp_instructions   2,178,502,266                   byte-identical
    codegen_instructions_dev and _release agree with their goldens

**`compile_instructions` worsened and lands at 35,968,173.** Two instructions.
The entry row fell by the same two and the library and interpreted rows did
not move at all. This branch adds counters and the gates that read them and
changes no decision the front end makes, so what moved is the binary's bytes,
at the smallest scale this vein has ever recorded.

**The emit row's old value was not CI's.** The file carried 382,309,867 under a
note calling it CI's first sitting; it was a container's, and the gate could
not have caught the mislabelling because the golden named no host at all —
`bench/emit_instructions_golden.txt names no host, so nothing can say whether
its rows may be read here`. The measured-on line is there now, under the
runner's own reading.

- **DONE** CI's sitting, and the golden says which host it was taken on.

## 2026-09-17 — the rewrite ladder: rewriting unreachable code costs nothing, and that corrects this morning's entry

The entry "kanso#1480's rows challenged, bisected, and the calibration's blind
spot found" says the row moved 145,472 *"because code that does not run on the
measured path was rewritten"*, and separates ADDING unreachable code — which
"leaves every existing decision where it was" — from REWRITING it, which
"moves what sits around them". Its own OPEN item asked for the ladder that
would bound the second shape. Here it is, and it does not support the sentence
it was asked to support.

**Eight rewrites of `without_stats_gate`, and the row does not move.** That
function is reachable only from `emit_ir`, which `kanso check` never calls —
the same position as the functions kanso#1480 touches. Each variant was built
under rustc 1.98.1 and measured with the gate, which printed a distinct
`compile_binary sha256` for every one:

    variant        row          .text      what changed
    L0 control     35,965,491   2,796,770  --
    L1 rename      35,965,491   2,796,754  locals renamed
    L2 hoist       35,965,491   2,796,882  the loop bound read once
    L3 loop        35,965,491   2,796,882  `while` spelled as `loop`
    L4 helper      35,965,491   2,796,914  two parses lifted into a helper
    L5 match       35,965,491   2,796,770  early-continue spelled as `match`
    L6 signature   35,965,491   2,796,658  `Vec<&str>` became `&[&str]`
    L7 wrapper     35,965,491   2,796,770  body moved behind a thin wrapper

`.text` spans 256 bytes. The row is identical to the instruction across all
eight. L6 and L7 emit byte-identical IR, so for those two the behaviour is
verified rather than argued; the other six are mechanical local edits.

**Adding a function that IS reached moves it 2,733.** L8 adds a
`OnceLock<Vec<_>>` built from DECLARES and calls it from `Backend::emit`,
changing nothing else — which is structurally what kanso#1480 adds as
`declare_lines`. 35,968,224 against the control, `.text` +3,088, IR
byte-identical.

**Three shapes, three answers, and none of them is 146,628.**

    unreachable additions   ~402, span 1,028 over seven binaries (2026-09-04)
    unreachable rewrites    0, over eight binaries
    a reached addition      2,733

CI reads kanso#1480 at +146,628 on this row against its base. That is fifty
times the largest calibrated shape. **So the explanation this morning's entry
gave is wrong**, and the correction matters more than the original claim did:
"rewriting code the measured path does not run" is now measured, eight ways,
at zero. Whatever moves that row on kanso#1480, it is not that.

**What the ladder does not settle.** It perturbs one function of 34 lines.
kanso#1480 changes 74 lines, adds a struct and a static, and changes an element
type that flows through a call chain — a larger perturbation than any rung
here, and the gap between 2,733 and 146,628 is where the answer lives. The
frame-level diff of the two compile profiles is what would name it; CI uploads
both as artifacts on every run, and this container's egress proxy refuses that
blob host, so it wants either a local reproduction of the pair or the diff run
where the artifacts are reachable.

The correction is recorded rather than folded away, beside the two from earlier
today, because it is the same failure a third time: an argument from a
measurement whose scope was never checked. The 2026-09-04 ladder covered one
perturbation. I read it as covering another, said so in a log entry, and only
building the second ladder showed the difference.

**The open item closed the same afternoon, and there is a FOURTH shape.** The
pair reproduced here at +143,118 against CI's +146,628, and the frame diff at
`--threshold=100`, comparing only frames present in both listings, puts the
whole of it inside type inference:

    check_merged_after_aliases    14,968,692 -> 15,109,213   +140,521
      infer::infer                 7,789,154 ->  7,929,934   +140,780
        for_each_child<expr_ctor_types>  267,606 -> 386,471  +118,865
        for_each_child<expr_ctor_types>  209,740 -> 315,845  +106,105
      demand::analyze                424,414 ->    456,238    +31,824
    parser::parse                 4,144,914 ->  4,136,716     -8,198

kanso#1480 changes src/codegen.rs and src/linear.rs and nothing else.
src/infer.rs, src/check.rs and src/parser.rs are BYTE-IDENTICAL between the two
trees. So the branch is not doing more inference work; the optimizer is
compiling unchanged inference code differently because the crate around it
changed.

Two symbol-level tells confirm the mechanism rather than leaving it inferred.
`parse_cmp` is a frame in the top profile and absent from the base, where it
was inlined into `parse_not`; both functions exist in both sources.
`stmt_ctor_types` is a frame in the base and gone in the top, where
`expr_ctor_types` appears instead. Those are inlining and monomorphisation
decisions moving.

    unreachable additions       ~402, span 1,028   2026-09-04
    unreachable rewrites        0, eight binaries  today
    a reached addition          2,733              today
    the optimizer re-deciding   ~143,000           kanso#1480

The first three are small because none of them is large enough to flip an
inlining decision. 161 lines across two modules is. **This is not the linker's
placement**, which is what "layout" has meant in this repository, and it is two
orders of magnitude larger. No ladder bounds it, because the perturbation is
"the crate got meaningfully bigger" and that cannot be synthesised inside one
small function.

What it means for kanso#1480: the move is real instructions on the measured
path, so the row is reporting honestly, and it is also not work the branch
chose or can avoid. That is an argument for weighing what a build actually
costs, which kanso#1470's rows and kanso#1491's `emit_instructions` term do,
rather than for arguing about this number.

- **DONE** all four shapes measured, and the gate's header carries the table.
- **DONE** kanso#1480's move named: the optimizer re-deciding, inside
  inference, on source the branch does not touch.

## 2026-09-17 — reading each KANSO_ switch once: measured, declined, and L8 is what declines it

The task stood on a real observation: adding one more variable to the
environment a compile runs in moves this row 11,606 (kanso#1483), so reading
the environment is not free, and kanso reads its `KANSO_` switches by asking
each time. Caching them behind a `OnceLock` is the obvious fix.

**It costs more than it saves.** On the module corpus's own profile:

    std::env::var::inner            2,445   what kanso's own switch reads cost
    std::sys::env::unix::getenv    46,172   inclusive, but see below
      _mi_getenv                   29,685   mimalloc reading ITS config, not kanso's
      getenv (glibc)               16,287

So the whole of what kanso's switch reading costs on this corpus is about
2,445 instructions. The ladder above measured what a `OnceLock`-backed function
costs to add and have reached: **2,733**. The cache is more expensive than the
thing it caches, before it has saved anything.

**The 11,606 is a different quantity and the task conflated them.** That number
is what one more variable in the ENVIRONMENT costs a compile — glibc's `getenv`
walking a longer `environ` on every lookup, plus mimalloc's own reads at
start-up. It is a property of the environment the gate runs in, which is why
the gate empties it with `env -i`. It is not a lever inside the compiler.

Declined, with the same shape as kanso#1483: withholding a line cost more than
the line. Recorded so the idea stays declined rather than being re-derived from
the 11,606.


## 2026-09-17 — the explicit box comes off the unbuilt list, item by item

STATUS.md's "Ruled, unbuilt" carried the 2026-09-15 box ruling all afternoon
while kanso#1477's body reported it stale and two of cloud's pull requests
cited that report as their reason for working a self-generated lead. An earlier
probe today said the opposite and was wrong twice over. So this one went
through the row's own Owes list, item by item, against a release build of
`298636b7`.

    the `effect` constructor          `effect 5`, `effect (err "nope")` answer a box
    an `(err _)` arm, anywhere         a two-arm `tell` catches one born three calls away
    the check refusal, three shapes    `boom 0 + 1`, `(boom 0)[0]`, `add1 (boom 0)`
    ch04 re-premised                   documents the rule and the name blind spot
    the 710 `xs[i]!` sites respelt     no `[...]!` handed to an operator in lib,
                                       scripts, bench or the book samples, and
                                       `xs[1]! + 1` is refused naming `.>`
    `!` names in lib answer a box      `read_file!` and `read_bytes!` both end `.>`
    the two cost levers                on main, with the 2026-09-17 entry "the
                                       bound discharge had no golden, and it is
                                       built" carrying the probe and the fixture

Every one built. The row is removed, and the removal is written into the
section's own intro with the evidence beside it, so anybody who disagrees can
put it back without re-deriving the list.

**The one residue is a spelling convention and not a build.** The row asks for
"no bang where the bound is provable", and that is not enforced: `xs[1]!` on a
three-element literal compiles, because a bang on a provable index is legal and
merely redundant. So the tree compiling proves the sites were respelt for the
rule and proves nothing about a needless bang left behind. That is a tidy-up
for whoever next reads those files, not a ruling waiting on a build, and it
does not hold the row open.

**What this cost by sitting.** The ruling landed 2026-09-15 and its `!` half
2026-09-16. The row could have come off on the 16th. It did not, because
nobody checked the list against the tree — the same failure the section exists
to prevent, on a smaller scale than the 2026-09-09 one that created it. A
report that a row is stale is not a probe, and a probe is eight commands.

- **DONE** the row off, with its evidence in STATUS.md.
- **OPEN** whether a needless bang survives on a provable index anywhere in the
  tree. Cheap for cloud, which has the bound prover; not a row.

## 2026-09-17 — the `.rodata` entry had two voices, and the pin was never the instrument

The ledger's `Pinning .rodata to a fixed page` entry gained cloud's bisection
this afternoon and came out contradicting itself: a body saying the term it
prices is three hundred times larger than anybody thought, and a
recommendation, written weeks earlier, declining the pin on the strength of the
small reading. An entry with two voices cannot be ruled, and the ledger's own
rule is that every entry carries a recommendation so a sitting can be a yes or
a no.

Rewritten, and the decline stands on different ground. **The pin's own evidence
never reached the case that matters.** Its demonstration was two sources a
hundred functions apart — an ADDITION, the same perturbation family as the
seven-binary ladder, and the family the pin was shown to fix. Nobody has run it
against a REWRITE, which is what costs 145,472. Adopting the pin on that
evidence would be adopting it on a measurement of something else, which is the
error this entry is now a record of twice over.

**And the gate's own header points at a different mechanism.** It says what it
found chasing this row's variance: a binary whose data and bss differ starts
the heap at a different break, which moves how much work malloc does to service
an identical request sequence, with every kanso symbol identical to the
instruction and only glibc's allocator moving. The heap break is set by where
`.bss` ends. A fixed `.rodata` start does not fix it.

So the instrument worth ruling on is a heap that starts at the same address
every run. It removes the term for additions and rewrites alike, and it changes
the shipped binary by nothing — which answers the kanso#1234 objection the old
recommendation leaned on, since nothing gets special-cased away from what ships.
It is also the 2026-09-15 rule read literally. The gate already pins ten
`GLIBC_TUNABLES` for this exact reason and where the heap begins is the one it
does not pin.

The entry now names the measurement that settles it, and says it should not be
ruled without one: cloud's kanso#1480 pair read twice, once with `.rodata`
pinned and once with the heap start fixed. A build and two callgrind runs.

- **DONE** the entry has one voice and a recommendation a sitting can answer.
- **OPEN** those two readings. Cloud's, and until they exist this is an
  argument rather than a decision.

## 2026-09-17 — the rewrite explanation withdrawn, and what the ledger said on it

kanso#1492 built the ladder this morning's entry asked for and the answer
refutes the entry. Eight rewrites of `without_stats_gate`, unreachable from a
check, eight distinct binaries: the row is identical to the instruction across
all of them, `.text` spanning 256 bytes. Rewriting unreachable code costs
nothing.

So the sentence in "kanso#1480's rows challenged, bisected, and the
calibration's blind spot found" — that a rewrite moves what sits around it, and
costs three hundred times what an addition does — is wrong. It was an
attribution read as a mechanism: the bisection established that 145,472 arrived
with 74 rewritten lines, and this chat wrote that down as rewriting being the
cause. A difference-in-differences is not a mechanism until something isolates
it, and the isolating experiment says zero.

**Where that claim had already travelled, and what each cost.**

    design/compiler-log.md   the entry above, on main       cloud corrects in kanso#1492
    docs/compiler.html §73   published, stating the rule     corrected here
    design/pending-gavels.md the `.rodata` recommendation    withdrawn here, unmerged

The ledger one is the one that mattered. kanso#1489 rewrote that entry's
recommendation around "the term is small for additions and large for rewrites,
and most changes rewrite", and asked for the entry to be re-weighed on it. That
is the sentence a sitting would have been ruling against. It is withdrawn
before the pull request lands rather than corrected after.

**What survives, and the entry is stronger for it.** The pin's decline no
longer rests on a contested reading of the term's size, because all three
calibrated shapes now agree it is small — about 402 for an unreachable
addition, zero for an unreachable rewrite, 2,733 for a reached one. A pin that
removes part of a few-thousand-instruction term, at the price of a one per cent
larger shipped binary, is a bad trade on any of those numbers. And the second
argument is untouched: the gate header's own mechanism is the heap break, set
by where `.bss` ends, which a fixed `.rodata` start does not reach.

**The 146,628 is open again**, fifty times the largest calibrated shape and
belonging to no shape anybody has measured. kanso#1492 names what would settle
it: the frame-level diff of the two compile profiles, which CI uploads as
artifacts on every run.

- **DONE** the claim withdrawn from the ledger before it was ruled on, and from
  the published page.
- **OPEN** what carries the 146,628. Cloud's, and it needs the profile pair
  rather than another table.

## 2026-09-17 — seven silicons, one recorded block, and a reader that was never called

kanso#1492 carries no compiler source — a log entry and a gate header, neither
compiled in — and its cost-goldens job came back red on one vein:

    interp_instructions  2,178,502,266 -> 2,178,502,272   +6

Chasing six instructions found something larger.

### The sha256 is not the code

Main's sitting on d1a7b058 and this branch's on e29fac05 compiled source that
differs in `design/compiler-log.md` and `scripts/gates/compile_instructions.sh`
and in nothing else. The gates print enough to compare the two builds:

    main        .text=2796866 .data=12672 .bss=29912  sha=e00a8ed55945c640
    kanso#1492  .text=2796866 .data=12672 .bss=29912  sha=1318a6776b335b6a

The three loadable sections are byte-identical and the file is not. This
container, on a third machine, builds the same tree three times and gets one
binary each time — `6fc4575752756e8d`, `.text=2796866` — so the Rust build is
deterministic on a box and the sha varies with the box.

**So the gates print a sha256 as the proof that two variants were genuinely
different builds, and that use holds: a different sha means the file differs.
The converse does not. Two equal shas are not needed for equal code, and two
different shas do not say the executed code moved.** The rewrite ladder leaned
on that line for eight variants and its reading is still sound, because there
the sections moved too.

### Seven blocks, and 57 rows between them

`scripts/gates/dispatch.sh` has carried a `differs` verb since the day it was
written and `bench/dispatch.txt` was never recorded, so `differs` answered
"cannot tell" every time and no gate called it. What the gates call is
`dispatch.sh name`, which prints the basic family and the model.

Ninety-odd cost-goldens job logs printed the candidate block (the `name` verb
prints it whenever no block is recorded). Within any one job every printing is
identical. Across jobs there are **seven distinct blocks, differing in 57
rows**, and the basic family itself takes three values: 0x19, 0x1a and 0x6. The
last is Intel. Level-3 cache spans 32 MB to 480 MB.
`Fast_Unaligned_Load`, `Prefer_No_AVX512` and `Prefer_PMINUB_for_stringop` flip
between them, and those are three of the switches glibc's ifunc resolvers read
when they pick `memcpy`, `memcmp`, `strlen` and their neighbours.

The gates pin the cache-derived thresholds through `GLIBC_TUNABLES`, which is
why the rows hold as steady as they do. What a tunable does not reach is which
implementation the resolver picks.

kq has recorded its block and consulted it since its own instruction vein
opened. kanso had the reader and never the block.

### What lands here

`bench/dispatch.txt` holds the block from this branch's own job, chosen because
on that silicon the three `kanso check` rows and the start-up row read main's
goldens to the instruction — it is the silicon those values belong to. The five
instruction gates consult `differs` in the disagreement path and print what it
says, before the verdict. It never decides the exit: a resolver difference is a
candidate explanation, not a ruling.
`tests/a_moved_row_is_told_what_the_silicon_did.rs` holds both halves, and both
were watched red — one gate with the consult removed, and the block moved
aside.

### And it was not the answer to the six

The instrument was built to ask that question, so the question was asked of the
two jobs already in hand. Both printed `370db01a104c` — the same block, all 123
rows. Same glibc, same rustc, same silicon, identical `.text`, `.data` and
`.bss`, different sha256, and:

    compile_instructions        35,968,171   both
    entry_instructions         128,213,972   both
    library_instructions       128,348,205   both
    startup_instructions         4,837,381   both
    interp_instructions      2,178,502,266  ->  2,178,502,272

Four rows to the instruction and one six apart. So the silicon is ruled out
rather than implicated, and the seven blocks above are a hazard nothing was
checking rather than this hazard. The block earns its place either way; it just
does not earn it here.

What is left is narrow enough to state. Six against 2,178,502,266 is three
parts per billion. The other four rows run 4.8 million to 128 million
instructions, where the same proportion is a fraction of one instruction and
could not be seen at all. The interpreted run is also the allocation-heavy one
by a wide margin — `interp_allocs` 5,313,434 against `compile_allocs` 27,397 —
and mimalloc's fast path branches on where its heap starts, which moves with
the size of the file the loader mapped. Six of 5.3 million allocations taking
the other branch is the shape that fits. That is an argument and not a
measurement, and it is written down as one.

- **DONE** the block is recorded, the five gates consult it, and the first
  question it was asked came back "the silicon did not move".
- **OPEN** the six instructions: a term proportional to work rather than a
  constant, visible only on the longest vein. Pinning where the heap starts is
  what would settle it.

## 2026-09-17 — the book entry leaves the ledger, having said four times it was not a question

design/pending-gavels.md is the single ledger of decisions awaiting Clay. Its
own rules say an entry cites its search or is invalid, and carries a
recommendation so a sitting can be a yes or a no. Audited all seven entries
against those two rules today; six pass and one fails both:

    cited rec  entry
      Y    Y   Does the wall survive the fused operators?
      Y    Y   Was the wall's simultaneous-failure merge meant to go?
      Y    Y   The box constructor's spelling
      N    N   The book teaches the boundary language
      Y    Y   How far does a binding position carry a box?
      Y    Y   A byte-position scan on a string, for the escape path
      Y    Y   Pinning `.rodata` to a fixed page

It fails both because it was never a question. Its own text says so four
times: *Nothing here is a question for Clay*, then *Still nothing here for
Clay*, twice more. It is a work record of what the book owed, filed as a
queued P1 on 2026-08-26 and kept in the ledger ever since.

**And the work is done.** Its final note, 2026-09-15, says the last item is
ch04's "nothing is asked of the signature", released by the box gavel and
moving with that build. That build landed, and the paragraph moved with it.
Read on main today, ch04 now says: *the checker asks its question at the call
instead: can this argument be an err the program raised? where it can prove
one, the function needs an `(err _)` arm at that position, or the caller
dispatches before calling, or the program does not compile ... err-in,
err-out is a fact about calls the checker cannot see into, not a contract
anybody writes.* That is the built rule, blind spot included, and its
`unasked.kso` sample binds to a name — the blind-spot case — and shows the
endpoint report.

So the entry leaves, and the ledger holds six questions, each of them a
question, each with a recommendation.

**STATUS.md's index took three edits, and the spec found two of them.** The
index claims the count in three sentences, and
`tests/the_status_index_counts_the_ledger.rs` pins each against the ledger's
own headings. Fixing the first left the second wrong and the second left the
third wrong, and the spec named each in turn rather than letting a stale one
through. That is the third time this index has gone stale by hand and the
first time nothing had to notice it by eye.

- **DONE** the entry closed and removed, the index recounted to six and four,
  and the spec green.
- **OPEN** nothing here. The six that remain are questions.

## 2026-09-17 — the unbuilt list empties, for the first time since it was made

kanso#1491 landed the second half of the 2026-09-16 gavel, and with it the
last row in STATUS.md's "Ruled, unbuilt" comes off. The section is empty for
the first time since 2026-09-09, the day it was created because five rulings
had sat unbuilt across 296 pull requests with nothing showing it.

Probed item by item against a build of main, the way the box row was, rather
than read off a report:

    what the row owed                        found
    interpreter start-up                     startup_instructions
    interpreter speed                        interp_instructions
    interpreter memory                       interp_peak_bytes
    dev-tier codegen cost                    codegen_instructions_dev
    release-tier codegen cost                codegen_instructions_release
    each in objective_sources.txt + spec     14 pairs; the replay spec passes
    weights and satiations priced            sourced in welfare.kso's header
    the meta floor re-ratcheted              76.12766770905162, "THE MODEL CHANGED"

`kanso run scripts/welfare` reads three numbers now: meta 76.13 against a
floor of 76.13, production 57.11, development 72.59. `--counters` lists ten,
including `emit_instructions`, the seam kanso#1491 found between the
2026-09-15 exclusion rule and the 2026-09-16 gavel — the exclusion took
kanso's own process out of the codegen rows, rightly, and the emitter runs in
that process, so a build's children were weighed and the compiler's own half
was not. That is the row kanso#1480's 51,082,187 fell into.

The weights are argued from named evidence rather than asserted: runtime 0.60
because it recurs per request forever, compile 0.40 rather than the third it
looks like from inside because 45 per cent of people who stopped using Rust
named long compile times among their reasons. The split renormalises the four
that predate it rather than carrying them over, and says why.

**What an empty section means, since nothing has said it before.** Cloud
chooses freely: no self-generated lead displaces ruled work, and the "which
rulings did you weigh" paragraph a pull request body owes has an empty list to
weigh against. That holds until something is ruled, and the chat adds the row
the day it is.

**What it does not mean.** The section's own preamble says the list is a
FLOOR, because the rest of the 2026-08-29 sitting was never audited. Empty
means nothing on the list is unbuilt; it does not mean nothing ruled anywhere
is unbuilt. The 2026-09-09 lesson was precisely that a ruling can sit outside
the list, and an empty list is the easiest state in which to forget that.

- **DONE** the row off, the section empty, and the probe recorded beside it.
- **OPEN** whether a sweep of the 2026-08-29 sitting would add rows nobody has
  listed. It has never been run, and an empty section is the moment it would
  be worth most.

## 2026-09-17 — the log goes back under its cap, and the cost of the move was a guess

design/compiler-log.md stood at 95 entries and 6,060 lines against a stated cap
of forty. CLAUDE.md's rule is that the live file holds the last forty and the
rest goes to design/log/compiler-log-archive.md unedited; the archive had not
been fed since it was last trimmed, so the file everyone reads at the tail had
grown to two and a half times the size the rule allows.

Fifty-six entries move, untouched, and the live file keeps thirty-nine plus
this one. Nothing is deleted: 40 live and 1,358 archived against 95 and 1,302
before, so all 1,397 are still on disk and the archive gained exactly what the
live file lost.

**The move was deferred twice today on a cost nobody measured.** Both times the
reason given was that it would conflict with every open branch's log appends,
with twelve in flight. That was a guess, and the guess was wrong. Trial-merged
against all eleven open branches, with the same merge run against main as the
control:

    branch                    vs main   vs the archive move   delta
    claude/prune-indexed         1              1               0
    claude/beat-indexed          7              7               0
    claude/linear-groups         7              7               0
    claude/group-indices         7              7               0
    claude/value-use-index       1              1               0
    claude/analysis-once         1              1               0
    claude/declares-const        1              1               0
    claude/aliases-once          1              1               0
    claude/codegen-rows          1              1               0
    claude/slot-prefix           0              0               0
    claude/welfare-split         1              1               0

Zero on every row. Every conflict those branches have they already have against
main, from the log TAIL they re-resolve on each base merge; the archive move
takes from the HEAD of the file, which no open branch touches. The two regions
do not meet.

This is the fifth time in one afternoon that a claim rested on a number nobody
checked, and the first caught by the rule the other four bought — CLAUDE.md's
"A measurement bounds what it measured", landed an hour before this. The
control run is the whole of it: a conflict count means nothing without the
count the change is supposed to have caused it to rise from.

**And the move disarmed a spec, which the ratchet caught and this entry did
not predict.** `tests/a_question_sent_to_clay_has_a_ledger_entry.rs` reads
`design/compiler-log.md` for paragraphs that send a measured decision to Clay
and checks each has a ledger entry to go to. Both sends it was written against
are 2026-09-15 entries, and both moved. The live log now holds ZERO such
paragraphs and the archive holds eight, so the spec passes over an empty
population and `scripts/ratchet/mutations/a_send_to_clay_with_no_ledger_entry.sh`
can no longer break it:

    ratchet: 1 mutations no longer apply
      STALE specs (unit, golden, differential) — a measured decision sent to
      Clay with no ledger entry to go to

**The hazard generalises and is worth the sentence.** Archiving moves content
out from under every check that reads the live log for HISTORY rather than for
the tail. Surveyed: twenty-nine files read `design/compiler-log.md` and exactly
one, `tests/a_log_heading_is_one_line.rs`, also reads the archive. Most of the
rest read the tail — the trend gate's worsened-counter sentence, the gates'
measured-on lines, page_drift's budget — and are unaffected. The Clay-send spec
is the one that reads history, and it is the one that went quiet.

So the archive move waits on the spec reading both files, which is a change to
`tests/` and therefore cloud's. The property it pins is about the log's
history, and the log is two files now.

- **DONE** the move prepared, the counts reconciled, and the conflict cost
  measured at zero against all eleven open branches.
- **OPEN** `a_question_sent_to_clay_has_a_ledger_entry` reading the archive as
  well as the live log. Cloud's, and this move should not land before it: a
  trim that silently empties a spec's population is a coverage regression
  whatever the line count says.

## 2026-09-17 — three sends the archive would have hidden, and one of them was never answered

Cloud built the reach fix for `tests/a_question_sent_to_clay_has_a_ledger_entry
.rs` — read the archive as well as the live log — and ran it against main
before landing it. It goes red on **three** sends out of 66,449 archived lines.
That is the spec doing its job the moment it could see the whole log, and the
three are this file's to answer rather than the spec's to be taught around.

**One of cloud's three is characterised wrongly, and the correction matters
because it decides whose work it is.** The `lex_word` send closes *whether it
exists is Clay's* and opens *Gavel #159 would delete this*, which cloud read as
a citation the spec cannot recognise — a gavel number where the spec looks for
a file name. It is not a citation problem. **Gavel #159 bounced.** The
2026-08-29 entry "the inline-name entry bounces the same way the digest did"
sent it out of the ledger unruled: *zero surface area — no program can tell how
the compiler stores a name. Per the same-day ruling that performance questions
with no surface are the implementer's, it leaves the ledger unruled.* So the
send's premise is stale rather than unfiled, and teaching the spec to accept
gavel numbers would have made it green over a question that no longer exists.
Recorded here so the paragraph is answered: whether the `String` exists is the
implementer's, and has been since 2026-08-29. The 2026-08-30 entry "eight
changes, and what they did to gavel #159" postdates the bounce and still reads
it as live; it is wrong on that point for the same reason.

**The second is answered, and answered the ordinary way.** The compile row that
counted the binary rather than the process — 41,904,811 on this container,
split 33,586,490 in the compiler against 7,982,541 in libc — says in its own
words that it *is filed as one rather than done here*. It was filed, and it was
ruled: the 2026-09-15 normalization gavel, built the same night as kanso#1439,
which CLAUDE.md records and which this afternoon's sweep verified against
`scripts/gates/compile_instructions.sh` — the gate anchors below Rust's stack
guard and drops the 465,122 instructions above that frame. Filed, ruled, built.

**The third is a real send with nowhere to land, and it goes to the ledger with
this commit.** escapebench pins the escape bracket's COST on every run and its
BENEFIT on none, so a change deleting the bracket would read as a 27.6% win
with every memory counter flat. The entry says *whether to raise its size is
Clay's, and not free: `escape_instructions` is a welfare term and a bigger
benchmark is a slower job.* That is a question for Clay, filed nowhere, sitting
in the archive since before the spec existed. It is an entry in
`design/pending-gavels.md` as of this commit, under the heading "Raising
escapebench's size, so it pins the bracket's benefit and not only its cost",
with its search cited and a recommendation to raise it and take the baseline
move.

**What this says about archiving, which is the point.** The move does not
create the defect; it reveals three that were already there, and the spec could
not see any of them while it read one file. Cloud's reach fix is right and is
cloud's to land. What it needed was not an exemption list but the two
paragraphs above and one ledger entry — the answers the sends were owed.

- **DONE** all three sends answered: one bounced and recorded, one filed and
  ruled, one now in the ledger.
- **OPEN** cloud's reach fix, and with it the question of whether the spec
  should recognise a bounce at all. A send answered by a bounce has no ledger
  entry by construction, which is a third state the spec does not model.
