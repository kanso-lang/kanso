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

## 2026-09-07 (second) — the splats were never on the short string's path

Searched the log, the archive and design/ before filing: `find2_below` appears in
the 2026-09-05 and 2026-09-06 entries (the shim, the word-at-a-time tail declined
at +2.2879%) and neither asks this question.

**A map of runbench on merged main.** Callgrind, clang 19.1.1, run from the repo
root; the whole program reads 2,399,081,635 against the golden's 2,398,991,511
on CI. Self instructions, calls, and instructions a call:

    d_json/encode_onto_2'2      316,832,879   2,380,860    133.1   13.33%
    d_json/value_for_3'2        233,409,627   1,791,207    130.3    9.82%
    d_json/obj_key_start_4'2    141,361,011     229,779    615.2    5.95%
    render_ryu                   90,045,360     191,070    471.3    3.79%
    k_b_find2_below_raw          84,989,250   1,353,330     62.8    3.58%

Two emitted functions carry 23.15% between them at about 130 self instructions a
call, and encode_onto's figure includes escape_onto, which the linker folded into
it. That is where the remaining runtime cost sits.

**Three readings of the emitted code that cost nothing, checked rather than
assumed.** The dispatcher's propagate block asks `k_not_failure` again on a value
the entry branch has already proved is a failure, and the block under it is dead;
`k_not_failure` is `alwaysinline` and pure, so LLVM common-subexpressions both and
folds the branch. The list and map arms of `encode_onto` compare a length against
zero and emit the `k_cmp` fallback beside the inline test, which reads as a set
the emitter never recorded for `k_b_length_fast` — but `encode_onto_2'2` does not
call `k_cmp` anywhere in the profile, because LLVM folds that arm too. And the
escape scan's call count is not waste: large.json holds 10,475 strings averaging
6.6 bytes of which 1,773 need an escape, so 1,353,330 calls over ninety rounds is
1.44 a string, one for a clean string and two for an escaped one.

**The shape that was built and declined.** `k_b_find2_below_raw` builds three
`_mm_set1_epi8` splats before a loop whose guard is `i + 16 <= len`, and a 6.6-byte
string never enters that loop. Guarding the whole vector block on `len - i >= 16`
so a short string cannot reach the splats:

    runbench   2,399,081,635 -> 2,405,273,995   +6,192,360   +0.258%

attributed to the instruction: `k_b_find2_below_raw` 84,989,250 -> 91,181,610,
every other row byte-identical, 4.58 instructions a call over 1,353,330 calls.
LLVM already sinks the splats into the loop's preheader, which the rotated guard
protects, so the premise was wrong: the splats were not on the short string's
path, and the second compare is pure cost. Reverted.

**A fourth reading, and it is arithmetic rather than a defect.** `k_b_append_range`
is entered 176,697 times and every one of them falls through to
`k_b_append_grow`: its fast path fires zero times. That reads as a buffer being
regrown, because 1,953 growths a round is a hundred times what one 185 KB output
buffer doubling from the floor would need. It is not. The caller is
`d_json/string_at_4` at 175,527 calls, 1,791.1 a decode round against the 1,773
strings in large.json that hold a character needing an escape — a ratio of
1.0102. One append per escaped string, into an accumulator that has no buffer
yet, and a first append allocates whatever path it takes. The 272.6 instructions
a growth are that allocation and its copy. This is the shape the 2026-08-29 entry
found in `push_mut_slow`: count the objects born before concluding one is
growing.


## 2026-09-07 (third) — the two hot functions have no loop in them

Searched the log, the archive and design/ first: the 2026-09-07 (second) entry maps
these two functions at the call level and the 2026-09-06 entries attribute
`parse_value` and the escape scan, but nothing has asked what the instructions
inside them do.

Callgrind again, this time with `--dump-instr=yes`, which prices every machine
instruction separately rather than every function.

    d_json/encode_onto_2'2   654 slots   133.1 a call   median 0.11   max 1.00
    d_json/value_for_3'2     519 slots   138.3 a call   median 0.12   max 1.83

**Not one instruction in `encode_onto` runs twice for a single call.** All 654 of
its slots execute at most once, and the median slot runs on eleven per cent of
calls, because different values take different arms. The function is a straight
line: an entry guard, a tag switch, one arm. `value_for` has a shallow loop —
thirty-two of its slots run more than once and the busiest runs 1.83 times — and
nothing hotter than that.

So the 133 instructions a call are not a loop anybody forgot to hoist out of, and
there is no instruction to remove: reducing them means emitting fewer
instructions on the path a call takes, which is the dispatch or the arms getting
smaller, not a peephole.

That closes the four readings the (second) entry records. Each was a hunt for
concentrated waste — a duplicated test, an unfolded fallback, a scan called too
often, a splat on the wrong side of a guard — inside functions that by
construction have none, and each cost nothing or cost more.

**Where the 133 go.** Bucketing the slots by how often each runs per call
separates the fixed path from the arms:

    encode_onto    27 slots   27.0 a call   20.3%   runs on ~every call
                  488 slots  101.9 a call   76.6%   runs on 10-40% of calls
                  139 slots    4.2 a call    3.1%   runs on under 10%

    value_for      17 slots   17.0 a call   12.3%   runs on ~every call
                  457 slots   65.2 a call   47.2%   runs on 10-40% of calls

The guard and the tag switch are twenty-seven instructions in the encoder and
seventeen in the decoder. Everything else is an arm, and an arm is the work the
function exists to do — an append, an escape, a render. So the group dispatch is
a fifth of the encoder and an eighth of the decoder, and removing all of it
would be worth 2.7% and 1.2% of runbench respectively, before asking which of
those instructions are load-bearing. The calling convention is not where the
cost is either.

`encode_onto`'s self total reads 316,832,879 in both dumps, to the instruction,
which is what makes this one trustworthy: the binary still carried the reverted
guard from the (second) entry, and that guard moves `k_b_find2_below_raw` and
nothing else. `value_for`'s total is 247,645,629 here against 233,409,627 there,
because callgrind splits its two instances differently under `--dump-instr`, so
its shape is reported above and its total is not.


## 2026-09-07 (fourth) — the whole program has no concentrated loop left

The (third) entry measured two functions. The same dump answers the question for
all of them, on a binary rebuilt from clean main this time: it reads
2,399,081,635, the baseline exactly, so nothing here carries the caveat that
entry had to.

For each function, the cost of its single hottest instruction over its own self
cost — a function with a tight loop puts a large share on one instruction, and a
function that is a straight line spreads it:

    d_json/encode_onto_2'2   316,832,879  13.21%   654 slots   0.008
    d_json/value_for_3'2     247,645,629  10.32%   519 slots   0.013
    d_json/obj_key_start_4'2 138,562,479   5.78%   242 slots   0.005
    render_ryu                90,236,160   3.76%   473 slots   0.018
    k_b_find2_below_raw       84,989,250   3.54%    55 slots   0.063
    __memcpy_avx_unaligned    55,344,167   2.31%   521 slots   0.465

Across the twenty functions that carry seventy per cent of runbench, the highest
concentration in kanso's own code is 0.063 — the escape scan's vector loop, in a
function of fifty-five instructions. Every other one sits between 0.005 and
0.042. The single exception is glibc's `memcpy`, where one instruction carries
25,736,426 on its own, and that one is not ours.

So the shape the (third) entry found in the encoder and the decoder is the shape
of the program. There is no loop anywhere in it holding a large share of the
cost, which is what a program looks like after its loops have been found: the
2026-09-06 and 2026-09-07 entries above took the runtime from a welfare of 51.89
to 66.00 doing exactly that, and this is the far side of it.

What that leaves is per-call work spread thin over short paths, and the way down
is emitting fewer instructions rather than removing a hot one. Anybody opening a
profile from here should read this first: the instruction-level hunt is spent,
and five separate readings that looked like waste this session cost nothing or
cost more.


## 2026-09-07 (fifth) — the front end is flat too, and one of its leads is an artefact of the profiler's environment

Three of welfare's five counters are compile-side and nothing had profiled the
compiler this session. Callgrind on `kanso check lib/json` reads 22,655,866
against the golden's `compile_instructions=19,315,772`; the difference is
startup, which the golden drops by anchoring at the `kanso::main` frame.

The top function carries 3.79%, and it is `hashbrown::HashMap::insert`. Summing
every function's self cost and bucketing it — the buckets add to 100.00%, which
is the check that the classifier double-counted nothing:

    kanso's own passes        12,088,671   53.36%
    hash tables                3,785,557   16.71%
    malloc/free                3,255,203   14.37%
    rust std/core              1,909,920    8.43%
    libc mem/str                 893,767    3.94%
    ld.so, getenv, tunables      722,748    3.19%

So a third of the compile is the data structures rather than the passes. The
part of that which looks removable is rehashing: `reserve_rehash` and the
allocation under it come to 955,430, 4.22% of the process and about 5% of the
counted vein, over 1,470 rehashes. Pre-sizing the maps would take most of it.

It is not one change. Attributed to the owning compiler frame, the 4.22% spreads
over twenty-odd call sites and the largest is `compile_module_loaded` at 0.68%,
with `check_file_shadow` at 0.39% and `advisory::name_types` at 0.34% behind it.
Twenty `with_capacity` edits are also twenty guesses at a final size, and 2026-08
already measured six of them (the filtered collects) at 4,514 instructions.

**The lead that is not there.** `getenv` reads 120,235 instructions in that
profile, 0.53%, and the callers are `phase::watched` — which asks the
environment for `KANSO_PHASES` on every phase entry — and `infer::infer`, which
asks once per fixpoint round. A cached `OnceLock` is three lines and obviously
correct, and it is worth almost nothing, because `scripts/gates/compile_instructions.sh`
measures under `env -i PATH=/usr/bin:/bin GLIBC_TUNABLES=...`. glibc's `getenv`
walks `environ` linearly, so its cost is a property of the shell that launched
the profiler. Under the gate's two-variable environment the same run spends
14,317 instructions there, 0.06%. A profile taken in a normal shell overstates
this by a factor of eight, and anything else that reads the environment in a
loop will read the same way.

The allocator holds no surprise either: 13,081 `malloc` and 13,085 `free` at
124 instructions an operation, which is ordinary glibc, and the 11,613 the
`compile_allocs` golden pins is that count with startup removed.

What a compile-side win is worth, for whoever picks this up: `compile_instructions`
sits at r = 2.9284 against its baseline, saturating at 0.854, and it shares the
compile-speed term's 0.32 weight with `compile_allocs`. A 3% cut moves the score
0.06. That is the same order as several compile changes that have shipped, so it
is not nothing, and it does say what the twenty edits would have to buy.

Read beside the (fourth) entry above, the two halves of the objective now say
the same thing. Neither side has a hot loop left in it.

## 2026-09-07 (sixth) — who wrote the instructions runbench retires

The (fourth) and (fifth) entries say neither half of the objective has a hot loop
left. That answers where the cost is concentrated and not who owns it, so the
same dump was bucketed a second way: for each function, who wrote the code. The
sitting reads 2,399,081,635, the baseline to the instruction.

    the compiler emitted it, from kanso source   1,339,325,060   55.83%
    src/runtime.c, hand-written C                  988,562,227   41.21%
    glibc                                           69,692,436    2.90%
    ld.so, qsort, floor                              1,501,912    0.06%

Every runtime shortcut merged between 2026-09-06 and today — the ten in #1294,
the freeze in #1295, the three text shortcuts in #1297, the counter gate in
#1298 — worked the 41% side. The larger half is the compiler's own output, and
what a change there buys is bounded by how good that output already is rather
than by how much of it there is.

Two probes into that half, both closed the same way.

The emitter puts `k_force_fast` in front of the match scrutinee in four of
`encode_onto`'s arms. Each of those arms is reached through a `switch` on the
scrutinee's own tag, and `k_force_fast` tests that same tag against 14. In the
built binary the function is 1,179 instructions with two `call k_force` left in
it, and both of them immediately follow `call k_index`: they force the result of
an index, which really can be a thunk. All four forces on the parameter are
gone, folded out of the jump table by constant propagation. This is #1303's
finding in a second place — the emitter writes a guard, LLVM removes it, and the
guard costs nothing to leave in.

The wider census invites the same mistake at a larger scale. Across 591 emitted
functions and 24,067 lines of IR there are 1,197 calls to `k_not_failure`, 819
to `k_err_hop` and 593 to `k_die`; counting the compare and branch around each
test, roughly a fifth of the emitted IR is failure propagation. The machine code
does not carry a fifth. `encode_onto` has five `k_not_failure` calls in its IR
and reaches its jump table after exactly one `cmp $0x5`; the rest of its
error-tag compares sit on arms that a valid document never enters.

So an instruction count taken off the IR is not a cost, and neither is one taken
off a profile whose environment differs from the gate's — the (fifth) entry's
`getenv` reading was eight times the truth for that reason. Both overstatements
point the same way, and the way to not be fooled by either is to read the built
binary or the gate's own number.

What remains true after all of it: the emitted half is the larger half, and
nothing found so far in it is waste LLVM has not already collected.

## 2026-09-08 — the unboxed heap parameter, built and declined at ninety-five per cent

The (sixth) entry ended on the emitted half being the larger one and nothing
found in it being waste. This is the first thing in it that was not already
collected by LLVM, and it is worse than what it replaced.

The measurement that opened it. Reading executed instructions rather than static
ones — the distinction the (fifth) and (sixth) entries were about — the emitted
half spends 12.52% of its work on register-to-register moves against the
runtime's 7.51%, on the same compiler at the same settings. Sorting those moves
by their distance to the next call separates the two explanations: 48.8% of the
emitted half's sit within ten instructions of a call, where 69.2% of the
runtime's sit far from any call. That is argument marshalling on one side and
ordinary register allocation on the other, and the marshalling band is
81,905,302 instructions, 3.41% of the run program.

The emitter already answers part of it. A parameter inference proves is exactly
`int` crosses the tailcc edge as a raw i64 and is rebuilt at entry from a
constant tag, and SROA folds the rebuild against both the body and the caller.
Instrumenting the set of every parameter over the run program's 1,206 slots:
253 already travel that way, and 298 more hold exactly one tag and do not —
list 86, string 75, bytes 73, record 31, fn 27, map 6.

Two of those six are not eligible and the reason is worth keeping. A subtype is
a `K_SUB` wrapper whose own tag is 15 whatever it holds, so `rec` is the bit a
subtype wears and a rebox at 7 would hand the body a `KSub*`; `tag_switch_shape`
already refuses on that same condition. And `fn` covers two runtime tags,
`K_CLOSURE` and `K_FNREF`, so no single constant stands for it. That leaves
list, string, bytes and map, and only where the program declares no subtype at
all — which the run program does not, since its dispatch is a tag switch.

Built that way, unboxed parameters go from 289 to 549 of 1,432, and the program
answers the same bytes: `runbench 46013475` either way. It retires
**4,690,139,807** instructions against 2,399,081,635, which is ninety-five per
cent more.

The profile says where, and it is not in the marshalling. Nine functions that
cost nothing at all before now cost something: `d_json/scan_at_5'2` at
325,198,863, `d_escape/filled_3'2` at 219,379,776, `d_json/obj_value_4'2` at
136,746,522. A function with zero self cost was being folded into its caller or
its tail cycle; changing its parameters from a boxed pair to raw words stopped
that, and the calls it had been dissolved into came back with their frames. The
2026-08 `noinline` reading found the same shape from the other direction.

So the marshalling is real and the obvious way at it costs thirty times what it
saves. What the emitter does today is not a narrow rule that nobody widened —
it is the widest rule that keeps the inlining, and `int` qualifying alone is the
point rather than an oversight.

## 2026-09-08 (second) — the run peak is one phase, and the guard its benchmark blames is not the one holding it

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md
(the 2026-08-31 entry "IDENTIFIED: the digest's 86x is a source-path prefix"
and the 2026-09-01 entry "the carry tier, arbitrated: DECLINED at -0.56"),
design/pending-gavels.md. The carry tier's exclusion has been identified before
and priced once. What is new here is where runbench's peak actually sits, and
that the benchmark watching it names the wrong guard.

**Why look.** `run_peak_bytes` is welfare's second-heaviest term at weight 0.26
and its least satisfied after `run_instructions`. Nothing had attributed it.

**The peak is one phase.** Each of runbench's eight phases built alone with the
other seven zeroed -- the shape the program's own header describes -- and
`arena_peak_bytes` read off each:

| phase | blocks | peak |
|---|---:|---:|
| EMPTY | 2 | 2,097,152 |
| decode | 4 | 4,194,304 |
| encode | 2 | 2,097,152 |
| deep | 3 | 3,145,728 |
| digest | 6 | 6,291,456 |
| index | 5 | 6,098,640 |
| escape | 2 | 2,097,152 |
| pend | 4 | 4,194,304 |
| **split** | **37** | **38,797,312** |
| the whole program | 43 | 45,944,528 |

`split` carries 84.4% of the peak on 4.87% of the instructions. Its
`alloc_bytes` is 38,248,797 against `evac_bytes` 9,216: it allocates 38 MB and
reclaims almost none of it, where every other phase sits at two to six blocks.

**The arena is flat, and the doubling comment belongs to another pool.** The
counter reads `k_live_block_bytes`, summed at `k_arena_push` (runtime.c:534),
whose only caller asks for a flat 1 MiB (`k_alloc_refill`, line 584). The
doubling blocks with the "address space rather than pages" note at line 1372
are the TENURE tier, `KTenBlock`, which reads 7 on this run and is not in this
counter at all. Residency checked from outside with getrusage: ru_maxrss is
52,088 / 52,028 / 52,108 KB over three runs, above the 46,682,840 the three
peak counters sum to. The counter is if anything conservative -- line 843
decrements when a block retires to `k_spare`, so spare blocks stay resident and
uncounted.

**The benchmark blames the wrong guard.** scanbench's header said the peak
stands because the walk's index is seeded at TOP, TOP carries the BYTES bit,
and the beat guard refuses a cluster whose parameter might be bytes. The beat
report gives `regexp/walked/5` two reasons to decline, and the one classify
stops at is the other one: "another group tail-calls it (unbracketed entry)",
with "argument 1 also carries heap" as an also. Three probes, each against
scanbench's 198,180,864 bytes over 189 blocks:

- BYTES condition removed from all three guard sites: 198,180,864 / 189, to the
  byte.
- `outside_tails` disabled in `classify`: 198,180,864 / 189.
- both cleared, so the group classifies as "carry beat: rewinds every
  iteration, evacuating argument 1, 3, 4, 5": 198,180,864 / 189, and the
  emitted IR is byte-identical to baseline (diff of 0 lines).

**What decides it is downstream of the classifier, and it is the 2026-08-31
prefix.** codegen reads `beat_loops`, not `classify`. After classifying,
`beat_loops` builds `imported` from `d.file.starts_with("std/") ||
d.file.starts_with("lib/")` and strips those groups' carries and ids.
`regexp/walked` lives in std/regexp, so its carry goes whatever the verdict
was. Clearing that filter as well: peak 1,048,576 over ONE block, a 189x fall,
with `alloc_bytes` unchanged at 197,577,484 and the program still printing 0.
The same allocation, now reclaimed.

**Reading a diagnostic as the decision is how three probes came back
byte-identical without saying so.** `report` calls `classify_all` and never
passes through the `imported` filter, so it describes a verdict the backend
does not act on. CLAUDE.md's rule against specs written on an internal verdict
applies to diagnosis too: the IR diff is what caught it, and it should have
been the first thing checked rather than the fourth.

**Not a proposal.** Removing the filter wholesale left runbench still running
after ten minutes against a 0.405-second baseline, because every library loop
starts evacuating, and the 2026-09-01 sitting priced that same removal at -0.56
welfare under the objective of the day. The exclusion has a real reason written
beside it -- a shared library driver threading its caller's invariant source
would copy an unbounded value every iteration. The defect is that the reason is
approximated by a path prefix, and the fix is to ask for the property. That is
a build, and it is priced by the objective when it exists, not by this counter.

**Open, and worth saying plainly:** the 2026-09-01 decline was measured on
digestbench under twenty-eight counters and a corpus the 2026-09-06 gavel has
since replaced with one consolidated program, and kanso#1295 has since moved
the thunk memo that entry named as the conflicting mechanism. The decline
stands until something re-measures it. It should not be cited as settled under
the current objective.

## 2026-09-08 (third) — the carry tier by a byte count: built, measured, declined, and the prefix keeps its place

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md (the
2026-08-31 "IDENTIFIED: the digest's 86x is a source-path prefix" and the
2026-09-01 "the carry tier, arbitrated: DECLINED at -0.56"), design/pending-gavels.md.
The 2026-09-01 sitting priced REMOVING the exclusion. This prices REPLACING it,
which is a different change, and it supersedes that entry's numbers under the
current objective -- that one was taken under twenty-eight counters and a corpus
the 2026-09-06 gavel replaced, and before kanso#1295 moved the thunk memo it
named as the mechanism.

**Where the peak is.** The entry above (2026-09-08 second, kanso#1310) attributed
runbench's 45,944,528-byte arena peak: `split` holds 38,797,312 of it on 4.87% of
the instructions, and the loop holding it is `std/regexp`'s walk, kept out of the
carry tier by `d.file.starts_with("std/") || d.file.starts_with("lib/")`.

**What it is worth.** Clearing the prefix and the two classify guards that also
block that group: scanbench 198,180,864 -> 1,048,576 over one block, 0.277s ->
0.115s, allocations and evacuation identical. With a capture added so the walk
fills its slots at every position and the work volume held: 489,684,992 -> 2,097,152
across 467 blocks -> 2, allocations within 2,002, 201,168 bytes evacuated.

**The bound.** `k_beat_iter_carry` computes `need` with `k_copy_size` before it
copies anything, so the threshold is three lines there. Over it, restore
`k_ten_on`/`k_from_window` and return, which leaves the arena un-rewound: that is
grow-only, today's behaviour for these loops, so the fallback is correct by
construction rather than by argument.

| bound | digestbench | peak |
|---|---:|---:|
| unbounded | 4,569ms | 559,939,584 |
| 4,096 | 309ms | 2,097,152 |
| 65,536 | 986ms | 138,412,032 |

scanmatch is byte-identical at every one of those. So four kilobytes admits the
regexp walk and refuses sha256's state and schedule, which are rebuilt each round.
That IS the property the prefix approximates.

**The sizing walk, and the latch.** First measurement: runbench 7,334ms against a
405ms baseline. The bound is tested AFTER `k_copy_size` has walked everything, so a
declining loop pays the full walk every iteration and throws it away. Latching the
decline per beat depth (reset in `k_carry_clear`, checked before the walk) gives
2,184ms with peak and evac byte-identical. Removing the latch again and letting
every iteration decide independently reproduces both numbers exactly at 512 and at
4,096 -- so the latch changes cost and nothing else, and a hypothesis that it was
punishing cheap loops sharing a depth with expensive ones is REFUTED.

**Where it fails.** Sweeping the threshold on runbench (baseline 405ms /
45,944,528 / evac 10,578,112):

| bound | time | peak | evac |
|---|---:|---:|---:|
| 128 | 453ms | 57,671,680 | 295,744 |
| 512 | 450ms | 46,137,344 | 5,848,832 |
| 4,096 | 2,293ms | 10,292,944 | 96,741,744 |

At 512 the run program is nearly free and its split phase does not get the win --
that phase's carry is genuinely over 512 bytes, which the unlatched run confirms.
At 4,096 the win lands and costs 5.4x. There is no threshold between them that
gives both, because the carries that buy the win are the same size class as the
ones that cost the time. 128 bytes makes the peak WORSE than baseline at
57,671,680, which is unexplained and is a reason not to reach for a small bound.

**DECLINED.** `run_instructions` is weight 0.30 and the least satisfied term;
`run_peak_bytes` is 0.26. 5.4x the instructions for 4.5x the peak is not close.
The prefix stays, and it now stays with a measurement rather than an accident.

**Two other shapes rejected on the way.** Removing the exclusion outright left
runbench still running after ten minutes against a 0.405-second baseline. Keying
the decision on the inference sets of the carried positions separates the two
programs cleanly -- sha256's are narrow LIST (0x200, 0x220, 0x2200, 0x2220),
walked's are TOP and FN (0x3fff, 0x3fdf, 0x800) -- and was rejected because it
would refuse the carry exactly where a user wrote a loop whose type is known and
admit it only where inference failed to pin anything.

**What would reopen it:** a cheaper evacuation, so the carries admitted at 4,096
stop costing what they cost. Not a better threshold.

**One method note, because it cost two rounds.** codegen reads `beat_loops`, not
`classify`, and `report()` never passes through the `imported` filter -- so the
beat report answered three questions this session that it cannot answer, and the
emitted-IR diff is what caught it each time. Diff the IR first.

## 2026-09-08 (fourth) — the ledger held five entries and the queue was reported as three

Clay, on being told the queue was down to his three gavel questions: *"you
have nothing else to work on? don't we have tons of open gavels?"* He was
right, and the mistake has a shape worth recording: the answer was read off a
session's private task list rather than off `design/pending-gavels.md`, which
is the file the project designates as the single source of truth. A task list
resolves nowhere but its own session. The ledger held five.

Two of the five were nobody's task, and neither turned out to be work.

**The assert hako shipped five days before this session reported it open.**
The entry carried "RECOMMENDATION: build it as its own design pass. The gate
is lifted" — and `lib/expect` was built on 2026-09-03 as kanso#1233
(`5b0f2eb1`): `expect`, `to`, `equal`, `be_true`, fifty-four lines, no builtin
and no runner change. The log records the build at "2026-09-03 (eighth) — what
a failing test is allowed to tell you", and that entry opens by citing this
very ledger line. So the work was done BY a session reading the entry, and the
entry stayed. The lifecycle says a settled item leaves the file in the same
commit as its record; a built item has no ruling to record, which is the gap it
fell through. It leaves now.

**The book's boundary-language entry cannot start, and its premise about why
was backwards.** Since the effects-are-types gavel of 2026-08-29 the entry has
said half one — ch04's "nothing is asked of the signature" — does not
survive as written and can be rewritten ahead of the typed-effect surface,
with half two waiting. Two probes on `2abcedaf` say the opposite:

    print ((n -> 99 + n * 0) (1 / 0))
    error[value]: division by zero

The body never runs and the call answers the failure. That is the railway ch04
teaches, in the words it teaches it. And `<int>effect` is refused by the
canonical-spacing rule before any checker is reached, because no `effect` type
exists in the tree for one to reject — `grep effect src/parser.rs src/ast.rs`
is empty, and `bind`/`rescue`/`annotate` are builtins of arity two rather than
eliminators of a type.

So the gavel retired a design the engines still run, and the book is present
tense. Rewriting ch04 onto explicit elimination would document a language
nobody can run. Both halves wait on the same implementation, and the
campaign — ch04, ch05's framing, compiler.html entry 23 — is one pass after
the surface lands rather than two around it. What HAS landed is already in the
book: the three words ship on all three engines (kanso#1116) and ch05 teaches
them as ordinary two-argument functions taking the effect first. Missing is
the type.

**STATUS.md's count of the ledger was stale for the second time in four days,
and is now pinned rather than recalled.** The sentence read "Six questions
wait — none blocking, six open" while the ledger held three entries under a
heading named Blocking and one under Open. Four of the six it named were gone:
the machine-code-size term and the two-value chip row were ruled on 2026-09-05,
the granted-baseline question was closed as moot by the 2026-09-06 gavel, and
the assert hako was built. The sentence itself carries a note about having gone
stale once before, when it said nineteen against a ledger of three — which is
the tell that recounting by hand is not a fix. A new spec,
`tests/the_status_index_counts_the_ledger.rs`, counts the `###` headings
under each `##` section of the ledger and compares them to the words
STATUS.md writes: the same shape as
`every_counter_gate_is_in_the_sweep`'s pin on CLAUDE.md's cost-golden count.
Watched red three ways before it passed: a stale total, a stale blocking/open
split, and an entry leaving the ledger with STATUS.md unchanged.

**A third find, from sweeping the rest of the file for the same shape.** The
Parked section — "on the record, no action" — carried a whole entry appended
under it with no `###` heading of its own: callgrind's attribution of the
compile row's binary-to-binary drift to glibc's `/proc/self/maps` parse, 0.27%
of the row and 100% of its drift, closing with "the ruling was made when the
term was known to exist and not known to be the whole of the drift, and this
entry is where that goes". It was filed deliberately, by the archive entry of
the same measurement, and then nothing could reach it: sessions cite entries by
heading, STATUS.md indexes by heading, and it had none — under a section whose
heading says nothing there is waiting. Filed and invisible at once.

It gets a heading now, under Open, with the recommendation the ledger's own
rules require: the 2026-09-03 no-exclusion ruling stands, because it was made
on principle and 0.27% is not a reason to reopen it; what the number changes is
how a session reads a 2,130 move on the row, as the loader's until
`lang_start::{{closure}}` says otherwise. Clay can close it on a word.

The second test in the new spec is that hole. Parked is one line per parked
item, so a non-blank, non-bullet line at column zero under it is an entry
nobody can cite. Watched red by putting the block back.

Nothing here is a question for Clay. The three blocking entries stand as filed.


## 2026-09-05 — gavel: corpus first — a blind corpus is repaired, never excused

On "The welfare model cannot see the yield hole, because the corpus
was written around it" (filed by #1240). The fix carries a chain's
yield per declaration, closing a hole where eight std effect wrappers
(os/read_file!, the net wrappers, os/kill) ran a loop on the grow-only
arena — a natural read loop paid 260 MB against 2 MB. It costs the
front end +0.2587% and every runtime counter is byte-identical,
because jsonbench had been hand-written to route around the hole and
the corpus was measuring the workaround. Welfare falls 0.008.

Clay declined the entry's recommendation (move the floor with "the
corpus is blind" as the reason). Ruling: **"the corpus is blind" is
never a reason to lower the floor; it is a corpus defect, and the
remedy is the one #1215 already set** — add the benchmark the
objective could not see, baseline it forward, and let the fix score.
Concretely:

1. The fixture the PR already carries — the natural read loop
   (reading_insisted.kso: 1 -> 201 beat iterations, 260 MB -> 2 MB)
   — is promoted into the benchmark corpus as a run-speed and
   run-memory shelf under the granted-baseline machinery.
2. Its baseline is measured on the PRE-fix code, so the objective
   sees the hole.
3. The fix lands on top; the change scores as the memory win it is;
   the floor RISES and is set in the same PR.

The golden half of the doctrine was already met (the fixture was
watched red first); this is the objective half. Clay's framing: the
corpus must incorporate the behaviour being fixed so it stands as a
test against the bug going forward — goldens catch a regression,
the objective prices one. The ledger entry leaves with this commit.

## 2026-09-05 — gavel: no machine-code-size term in welfare

On "Should the welfare index carry a term for machine-code size?",
Clay: "guessing is not okay so I guess no size term." No term.
`.text` stays watched exactly, per program, in its own vein that
refuses a silent move; the welfare sum exists to catch trades between
dimensions, and one observed trade (kanso#1247's always_inline, 34,320
bytes for tens of millions of instructions) is not evidence enough to
weigh one. A weight guessed rather than argued from cases would price
every future inline decision by the guess. If cases accumulate, the
way in is a satiation and a weight argued from them, as a weights
change. The ledger entry leaves with this commit.

## 2026-09-05 — gavel: one row, one value, and every move is the compiler's

On "Should a chip row still be allowed to pin two values?", Clay
declined both the pinned pair and the proposed binary-sha stamp, and
set the model the vein works under: "you have done enough work in the
process to be confident that it is deterministic and so you treat it
as such. if it got better then good if it got worse then bad and you
assume it's always because of the compiler. but then of course you
always just check to see if it's consistent. and if it's not you say
okay well now we have to look for other sources of inconsistency and
get ourselves back to a state where we're confident we've nailed them
all."

The ruling:

- **One row, one value.** The pinned-pair mechanism and the per-chip
  key retire. `bench/compile_instructions_by_cpu.txt` collapses to a
  single number; the gate refuses any second value.
- **Every move is attributed to the change under test.** A rise is a
  regression to explain, a fall is a win to bank — the ordinary
  ratchet — with no category of "the measurement drifted." The
  measurement was made consistent (the row counts from the runtime's
  entry closure inclusive, loader and stack guard excluded, per the
  2026-09-04 build) and seven chip keys across two binaries agree to
  the instruction on it; that is the evidence the model rests on.
- **Consistency is checked, not assumed silently**: the same build
  must reproduce its number on any runner and any run. A
  reproduction failure — the number moving with no compiler change,
  or two chips disagreeing on one binary — is not a mode to record
  and not a pair to pin; it halts the vein and is hunted until the
  source is found and removed, as the /proc/self/maps term was. The
  vein returns to service when reproduction holds again.
- No binary-sha stamp: under this model there is nothing for it to
  distinguish. A move with no compiler change is by definition a
  consistency failure, and the response to that is the hunt above.

The entry leaves the ledger with this commit; its contradiction about
the 508 (one binary or two) becomes moot, since neither reading
licenses a pair.

## 2026-09-06 — gavel: bump clang to 19, with feature detection

On #290's toolchain path, Clay: "yes bump clang." CI's pinned compiler
moves from Ubuntu clang 18.1.3 to LLVM 19 or later so the emitter can
use `preserve_none`, the calling convention built for the shape of
kanso's hot dispatchers (tail-call-heavy, paying a callee-saved
prologue on every entry): priced at 2.84% of encodebench and the named
remedy for the prologue cost in both `encode_onto` and `parse_value`.
Two conditions ride with it:

- **Feature detection, so no user gains a requirement.** The emitter
  uses `preserve_none` only when the clang it invokes accepts it, and
  falls back to the current convention otherwise. A user on clang 18
  still builds and only misses the 2.84%. The bump is CI's and the
  goldens', never the language's.
- **Every compile vein re-sits in the one PR that bumps**, with the
  sentence the veins' own headers require for a toolchain move, and
  every measured-on line updated. Instruction and machine-code rows
  will move together; that is the expected shape and is said once.

## 2026-09-06 — gavel: a whole float keeps its point at every magnitude

On #300 (a whole float above 1e15 renders as an integer), Clay: "yes
float point at all magnitudes." The rule "a whole float renders as its
digits and `.0`" — the golden's own name — held only below 1e15, where
the integer cast is exact; above that the shortest-round-trip path
printed `1000000000000000` with no point, and the value's identity as
a float vanished at an implementation boundary. Ruled: a float's
rendering always carries a `.` or an `e`, whatever its magnitude.
Where the shortest form has neither, `.0` is appended. Both engines
already agreed on the old behaviour, so this is a surface change
pinned by a differential golden across all three, and
`a_whole_float_keeps_its_point` extends past the boundary it was
named for. The filing for #300 is owed to the ledger by heading; this
entry stands as its ruling regardless.

## 2026-09-06 — gavel: one consolidated run program is the objective

Asked about escapebench pinning the beat's cost and none of its
benefit, Clay: "there shouldn't be separate benchmarks, there should
just be one consolidated program on which we test performance across
the board." Gaveled. The fleet of run shelves — jsonbench,
encodebench, widebench, deepbench, pendbench, oneshot, basket, scan,
digest, escape, index, live — stops being the objective's inputs. In
their place:

- **One consolidated run program.** Its retired instruction count is
  the run-speed term; its peak memory is the run-memory term. The
  workload mix is chosen deliberately to reflect real use — decode and
  encode as the bulk, the stress shapes (wide, deep, pending, escape,
  index, digest, scan) present at realistic proportion — and written
  down in the program's own header so the mix is a decision on the
  record, never an accident of accretion.
- **Trades are seen in one place.** A mechanism's cost and its
  benefit land in the same instruction count and the same peak; a
  phase weighs by the work it is, so a 172x win in a phase that is 1%
  of the program moves the total by what it is worth.
- **Per-phase counters stay as diagnostics**: goldens per phase that
  say where a move came from and catch a stress shape regressing
  quietly. The sum is the objective; the terms are diagnostics, one
  level down.
- **Peak means peak.** The program's peak is its worst phase's, and
  that is what "how much memory does this workload need" means; a
  lesser phase's memory improvement reaches the objective when it
  becomes the worst, and the per-phase peak goldens keep it visible
  meanwhile.
- **Adding a phase is a definition change**: the floor re-sets once,
  with the reason, in the PR that adds it. No granted baselines, no
  entry standing, nothing enters satiated.
- **Superseded**: the advertised-versus-guards split of the weights
  gavel (2026-09-02) — workloads now weigh by their share of the
  program; the granted-baseline machinery for run counters; the
  per-shelf entries #317 (escapebench) and #319 (entry standing),
  which are moot under one program. The weights (0.30 / 0.26 / 0.32 /
  0.12) and satiations stand; only what the run terms read changes.
  The compile side already measures one program and is unchanged.
- The switch is a definition change: recorded here, the floor re-set
  in the same PR, and history.jsonl's welfare column rewritten once
  per the 2026-09-03 directive.

## 2026-09-06 — directive: the framework's language features wait for the language

Clay, on the convention-over-configuration primitives (a function
value that knows its name, a module's exports as a map, the route
table as a dispatch group): defer them "until we've got the language
as currently specified done to the best of our ability — once it's
optimized and debugged as far as we can get it," which he estimates at
a couple of weeks. So the parked note carries a DEFERRED marker and
the order of work is fixed: finish and polish what is already ruled —
the effects-are-types migration, the fused operators, whole-cohort
block-born, the growable partial, the suffix contracts, the
consolidated run benchmark, the clang 19 bump, the book rewrite — then
the serve campaign and the framework primitives, reopened by Clay and
not by a session's initiative. A session that finds itself with no
ruled work left reports that rather than starting this.

## 2026-09-07 — gavel: the welfare history's baseline, after the one-program gavel

The ledger entry "The welfare history's baseline, after the one-program gavel"
(kanso#1289) asked four questions. Clay ruled all four in one line: "I'll go
with all your recommendations." The recommendations he took are the ones set
out to him in chat, which differ from the ledger's on one point, and the
difference rests on a fact the ledger got wrong, so both are recorded here.

**The ledger's premise was wrong on which counters the old rows carry.** It
said 426 of 500 rows carry `encode` alone. Read off origin/perf-history
(history.jsonl, 500 rows): from row 70 (2026-08-10) every row carries four
run-side instruction counters — `instructions`, which is the DECODE row and
was never named as such, plus `encode_instructions`, `oneshot_instructions`
and `basket_instructions`. The eight-phase set begins at row 446 (2026-09-03)
and `run_instructions` at row 494 (2026-09-06). "Encode alone" was a key-name
mapping error: the decode counter's unprefixed key was not recognised as a
phase. A base built from four of the eight phases, two of them the largest
shares of runbench, is a reconstruction; a base built from one would have been
a guess, and that guess was the reason the ledger recommended 2026-09-03.

**Rulings.**

- (a) **Share-weighted.** Each phase's ratio to the anchor, weighted by
  runbench's own measured shares, calibrated to equal runbench at the
  changeover. A raw sum re-weights the objective away from the mix the
  one-program gavel ruled, and hides indexbench's 488× behind encode's
  repetition count.
- (b) **Baseline at the earliest reconstructable row, 2026-08-10 (row 70),**
  with the key mapping fixed: `instructions` is decode. A row's shares are
  renormalised over the phases it carries, so a four-phase row is scored on
  the four and an eight-phase row on the eight, and no phase is extrapolated
  from another. The 69 rows before it carry no run counter and stay unscored.
- (c) **`run_peak_bytes` is based at today**, and the floor file says so in a
  comment beside the number, until runbench has peak history of its own. The
  earliest phase peak is 2026-09-03 and has not moved, so there is nothing
  to reconstruct.
- (d) **The compile rows keep their accumulated baselines.** The gavel
  re-defined the run side and said nothing about the compile side.

**What follows.** The history column is rewritten once, in place, under this
definition (the 2026-09-06 directive stands: one welfare line on the chart,
no replayed-versus-recorded pair), the floor is re-set from the rewritten
column, and the ledger entry leaves in this commit. The rewrite is cloud's;
this entry is the ruling it was waiting on.

## 2026-09-07 — the one-program gavel re-priced the declined queue, and one decline flips

Clay asked whether declining changes one at a time strands the project on a
local maximum, where some combination of the refusals would have paid. The
question has a smaller population than it sounds like. Searching the log and
the archive for changes the OBJECTIVE declined, rather than changes declined
because they were worse on their own numbers, turns up two. Both were priced
under the model the 2026-09-06 gavel retired: twenty-five counters over
thirteen programs, each normalised against its own baseline and averaged.

**The tag-switch widening flips.** Archive entry "BUILT, MEASURED, AND DECLINED
BY THE OBJECTIVE — an arm that destructures stays on the cascade". Its rows
re-priced through `bench/runbench_phases.txt`:

    row              move     phase    share    contribution
    pendbench    -2.8838%     pend     4.94%       -0.14246%
    livebench    -0.1484%     no phase, invisible to the objective
    basket       -0.1092%     no phase, invisible to the objective
    oneshot      -0.0654%     no phase, invisible to the objective
    scanbench    +1.0826%     split    4.87%       +0.05272%
    encodebench  +0.1286%     encode  34.43%       +0.04428%
    digestbench  +0.0427%     digest   5.05%       +0.00216%
                                                   ---------
    reconstructed runbench instructions             -0.04330%

The change was reverted for falling. Under the objective as it now stands it
is a win, and it wins while the reconstruction throws away three of the four
programs it improved, because livebench, basket and oneshot have no phase.

**Why it flips is the shape of the old model.** Twenty-five rows meant
twenty-five ratios, each against its own baseline, so every row sat at its own
point on the satiation curve and carried its own stiffness. A win on a program
already far ahead of its baseline bought little; a loss on one sitting near its
baseline cost a lot. pendbench's 2.88% fall was the first kind and scanbench's
1.08% rise was the second. runbench combines its phases by measured share
BEFORE the curve applies, so the same two moves are priced at 4.94% and 4.87%
of one program and the larger move wins.

That is most of the answer to the question as asked. The sequential-acceptance
trap was substantially a property of the twenty-five-row model, which could
decline a change that lowered total work because of where each row happened to
sit. What remains is genuine complementarity: changes sharing a fixed cost, and
changes that are prerequisites for other changes. Those the objective cannot
see whatever its shape, and they need the combination sweep Clay described.

**The other two do not flip, which is the check.** The gate widening of
2026-08-02 cost basket allocations and arena buffer; basket has no phase, so
the objective can now see neither its cost nor its benefit, and the entry's own
corpus-breadth argument is what it still turns on. The unsigned-compare
decline of the live log, "worse on nine of the thirteen benchmarks",
re-prices to +0.87% and stays declined on every mapped phase.

**WHAT THIS IS NOT.** `bench/runbench_phases.txt` licenses direction and share
and says in its own header that the scale does not carry across. The tree has
moved a long way since the widening was written. Nothing here is a measurement
of today's compiler, and the widening has to be rebuilt on current main with
runbench counted before any of it is banked. The reconstruction says the
rebuild is worth an afternoon, and no more than that.

**OPEN, and named by the widening's own entry.** A group mixing int literals
with type patterns takes neither switch. The shape that would serve both is a
switch on the tag whose int case holds a second switch on the payload, and it
is what would remove the split phase's cost rather than outweigh it.

## 2026-09-08 — gavel: page_drift counts the wrong thing, and the fix is cloud's

`scripts/page_drift/page_drift.kso` takes the last commit that touched
docs/compiler.html, diffs design/compiler-log.md from there to HEAD, counts the
`## ` headings that appear in it, and fails past a budget of three. That count
stands in for a question the gate cannot ask: has the presented design fallen
behind what the compiler does. Clay, 2026-09-08: "there is no specific
correlation between a number of log entries and specific changes to the HTML."

There is not. A batch of gavel entries owes the page nothing and trips the
budget; one shipped optimization can owe the page a great deal and never reach
it. The gate's own header already names the exemption it has no way to apply --
an entry recording a diagnosis, a revert or a handoff has no business on the
page -- and a heading does not say which kind of entry follows it.

**RULED: fix the gate rather than work around it.** Two shapes are tractable:
count only the entries that are not rulings, or exempt a pull request whose log
diff is entirely gavel entries. The step is not `continue-on-error` in ci.yml,
so it fails the whole cost-goldens job rather than reporting beside it.

**CLOUD'S**, per the same day's lane ruling: a gate is code. This entry is the
filing itself, because peer messaging does not reach the cloud session from the
chat and this repo does not use issues -- the log is where a thread left open
goes.

**What it blocked, recorded so the workaround is not mistaken for the rule.**
This pull request adds nine log entries against a budget of three. The session
writing it put a paragraph into docs/compiler.html to clear the gate, then wrote
that workaround into CLAUDE.md as a standing carve-out; Clay caught it and both
are reverted. The page paragraph is separately owed and stays owed: the ryu
entry says the format layer "mirrors the old %g byte format exactly", and the
2026-09-06 whole-float gavel made that false above 1e15, where the behaviour has
shipped and `a_whole_float_keeps_its_point` already pins `1.0e+15`. Landing that
correction is cloud's, and it was separately owed whatever happens to the gate.

**A page move does not clear the count, and this entry said it would.** The
sentence above originally read that landing the page correction "drops this
budget to zero for anything behind it." Measured on this branch with
origin/main merged in, it does not: `page_drift` diffs two trees rather than
walking history, so the anchor is always a commit on the base side and a
branch's own entries appear as additions from it whatever the page did. With
kanso#1315 on main the old gate still read 10/3 here. The rule the old gate
actually enforced was that a pull request adding more than three log entries
must also edit docs/compiler.html, unconditionally. This is corrected in place
rather than appended because the entry had not landed; the claim was never on
main.

**CLOSED by kanso#1316**, which skips a ruling and counts everything else, on
the `— gavel:` and `— directive:` heading convention the log already writes.
Nine of this branch's ten entries come out and it reads 1/3. The fix is cloud's
and the spec is its own: a nine-ruling batch passes, four shipped entries still
fail and are named, and four headings that merely mention a gavel still fail.

## 2026-09-08 — gavel: the compile term reads a fixed corpus, not whatever lib/json imports

Clay, 2026-09-08, taking the recommendation the ledger carried: the three
compile gates and the objective compile a package that names its imports once,
rather than whatever lib/json happens to import that week.

The question. kanso#1291 retired lib/json's escape fold, which was its only use
of std/list, and the import went with it. `kanso check lib/json` then compiled
half the code it had: `compile_instructions` about 42.6M to about 19.6M,
`compile_allocs` 25,862 to 11,613, `compile_peak_bytes` 724,798 to 375,222 on
the container, welfare 51.95 to about 56 -- with the compiler byte-identical. By
the objective's definition that rise was real. By what the term is for, how fast
the compiler is, it measured nothing, and a later change re-importing std/list
would have read as a four-point fall.

**Ruled: option (b).** The corpus is lib/json plus every std module the
benchmarks import -- list, text, testing -- named once in a package the gates
and the objective both read. A dropped import then moves the rows by that
import's own compile cost and nothing else, and a bare library edit reads as
what it is. Rebased once with its own `--set`, and the history chart marks the
day.

The other two are declined with it. (a) pricing whatever lib/json imports, and
(c) recording this rise as a re-basing while leaving the workload alone. Each
leaves the next import change to make the same argument again.

The floor kanso#1291 set stands until the rebase, per the standing rule that a
rise is held rather than banked. Building this is cloud's.

## 2026-09-08 — gavel: an infinite or nan float renders as inf, -inf and nan

Clay, 2026-09-08, taking the recommendation the ledger carried: C's spelling,
which is what the `%g` rule the renderer already mirrors produces.

Neither engine had an answer and no golden in the corpus asked for one. The
interpreter panics: `render_float` asks Rust for `{:e}` digits and expects an
`e` in the reply, which `inf` has none of (src/eval.rs:3949). Native prints
`1.797693134862316e+308`, the largest double's digits, for a value that is not a
double's, and `2.696539702293474e+308` for nan. Both engines are wrong and the
oracle's wrong is a crash, which is why this reached the ledger rather than
being fixed where it was found.

**Ruled: `inf`, `-inf`, `nan`.** Rust's `Display` for f64 prints the same three,
so the oracle says it with one arm and native with one branch in front of ryu.
JavaScript's `Infinity` and `NaN` were the alternative named and are declined:
they descend from no other rule in this renderer, where the `%g` lineage
already decides every other question it answers.

A differential fixture over the three values on all three engines ships with the
change, per the differential law. Whether `json/encode` may emit such a value at
all belongs to that library and is not answered here. Building this is cloud's.
## 2026-09-08 (fifth) — the drift gate counted rulings against the page

`scripts/page_drift` takes the last commit that touched docs/compiler.html,
diffs design/compiler-log.md from there to HEAD, counts the `## ` headings and
fails past three. Clay ruled the count wrong on 2026-09-08: "there is no
specific correlation between a number of log entries and specific changes to
the HTML." Two shapes were named as tractable, and this is the first of them.

A ruling is skipped now and everything else still counts. The marker is the
heading convention the log already writes — `## <date> — gavel: ...` and
`## <date> — directive: ...` — so nothing new had to be agreed. An entry that
merely mentions a gavel keeps its place in the count, because the colon and the
dash are what the convention writes.

The whole-float gavel is why that is the right exemption. It was ruled on
2026-09-06 and filed as one entry; the behaviour it decided shipped in
kanso#1285 as a second; the page sentence it falsified was corrected in
kanso#1315 as a third. One page edit was owed across the three, and the ruling
was not the entry that owed it — a decision documents a page once something
implements it.

**Measured on the pull request it blocked.** kanso#1313 carries nine rulings
and one entry that is not one. The old gate reads 10/3 and fails the whole
cost-goldens job; the new one reads 1/3 and passes, printing
`(9 rulings not counted)` so the skip is visible.

**The page move alone does not clear it.** The gavel entry expected kanso#1315
to drop the budget to zero for anything behind it. It does not. page_drift
diffs trees rather than walking history, so a branch's own entries still appear
as additions once the merge base moves: with origin/main merged into
kanso#1313, the old gate still reads 10/3. Landing this fix is what unblocks
that pull request.

The gate had no spec at all before this one, which is how the defect outlived a
redesign of the log's headings underneath it.
`tests/a_ruling_is_not_a_page_the_log_owes.rs` builds a repository whose page
moved once, appends entries to the log, and reads the gate's own output: the
nine-ruling batch passes at 1/3, four entries recording shipped work still fail
at 4/3 and are named, and four headings that mention a gavel without being one
still fail. The first was watched red on the old gate before the fix went in.

## 2026-09-08 (sixth) — k_b_join called libc for a single byte

`k_b_join` copied both the separator and each item with `memcpy`. In every
shipped caller measured both are one byte or none: `join [s s] ""` gives a
zero-length separator, and `join digits " "` joins one-byte strings with a
one-byte separator.

**Attributed before it was built.** The whole program's
`__memcpy_avx_unaligned_erms` is 55,344,167 instructions, 2.33% of runbench,
and the per-caller breakdown sums to it exactly:

```
     Ir      calls   Ir/call  caller
 29,208,336  353,394     82.7  k_b_append_grow
  9,472,765  400,019     23.7  k_b_join
  9,306,956  130,618     71.3  main
  2,926,620  191,070     15.3  render_ryu
  2,296,030  177,706     12.9  k_b_split
```

Through the call a one-byte separator costs 16.0 instructions and a one-byte
item 23.6. Storing the byte where the length says one saves 5.5 on an average
call.

**The earlier ceiling estimate was wrong by 7.6x, and the reason is worth
keeping.** It priced a one-byte copy at the whole-program memcpy average, about
forty instructions. The distribution above is bimodal — `k_b_append_grow`'s 82.7
per call is a real buffer copy and drags the mean far above what a short copy
costs — so the average describes no call site in the program. A per-site figure
is the only one that means anything here. The same error nearly shipped a second
time in this change's own source comment and was caught before the commit.

**On CI.** runbench 2,398,991,511 -> 2,397,582,951 (−0.0587%), and pendbench
596,612,948 -> 590,979,348 (**−0.9443%**), which is where join's one-byte
separators live: 199,900 separator calls and 199,973 item calls of the 400,019.
The local reading was −0.0930% of runbench against a different baseline.

**The compile row moved, as it was warned it might.** compile_instructions
19,315,772 -> 19,316,711, a rise of 939. `src/runtime.c` is `include_str!`'d
into the compiler, so its bytes shift the binary's layout even though
`kanso check lib/json` stops before codegen — the eighth layout-only move that
row has recorded. It could not be settled before CI: this container refuses to
compare that vein at all, its golden measured on glibc 2.39-0ubuntu8.8 and
rustc 1.98.1 against the container's 2.39-0ubuntu8.7 and 1.94.1. The five
.text rows that carry the extra branch grew 64 bytes each, so the text vein
sums 1,469,868 -> 1,470,188. The first round of this change said three rows
and updated three; the two it missed were found by comparing all fourteen
against CI's output rather than by eye.

Welfare 66.00 -> 66.01, and the floor is ratcheted in the same change. The sum
rises with the compile term's 939 counted against it, which is the trade the
weights exist to make.

**What this closes and what it leaves.** The short-copy regime (callers under
about 25 instructions a call, where the call dominates the copy) is 16.4M of
that 55.3M, and join was 9.47M of it — the largest single site by three times.
The rest is `render_ryu`, `k_b_split`, `w_klam42` and `k_stats_switch`, about
6.95M together, worth measuring as one change on a baseline that includes this
one. `k_b_append_grow`'s 29.2M is not that shape and is already understood:
one first-append per escaped string, not repeated growth.

## 2026-09-08 (seventh) — inf, -inf and nan are words on all three engines

The 2026-09-08 gavel above ruled the three spellings; this is the build. Both
engines were wrong before it, in different ways. The interpreter
asked Rust for `{:e}` digits and split the answer on its `e`, which `inf` does
not have, so `render_float` panicked at src/eval.rs:3949 — the oracle's failure
mode for these values was a crash. Native handed them to the ryu digit core,
which reads a double's mantissa and exponent fields; the all-ones exponent
those encodings use means something else there, and the core answered
`1.797693134862316e+308` for infinity and `2.696539702293474e+308` for nan.
The first is the largest finite double's digits printed for a value that is
not that double.

The interpreter answers with two arms in front of the shortest-digits path.
Native answers with one test in front of ryu rather than isnan() plus isinf():
both encodings are the same exponent field, so the bits are loaded once,
masked once and compared once, and the mantissa then says which of the two it
is. The wasm host reuses the interpreter's `render`, so the fix reaches it
with the oracle's.

The fixture is tests/golden/micro/an_infinite_or_nan_float_renders_as_a_word,
which the micro corpus runs on the interpreter and on native and the browser
differential runs on wasm — the three engines the differential law names. The
language has no exponent literal, so it reaches infinity by squaring a thirty-
zero literal four times and once more, the same way the negative-render fixture
next to it reaches its wide exponents. It was watched red first: native printed
the max-double digits on every line and the interpreter panicked, while the
finite line below agreed on both, which is what says the fixture reads the
render and not the arithmetic.

**What it costs, and why the floor moves anyway.** Every render pays the test
that asks whether to write the word, so where the test sits is the whole
price. Three placements were measured on the container, each side rebuilt,
against a baseline of 2,373,798,762:

```
  in front of ryu, its own load and test    2,374,848,342   +1,049,580  (+0.0442%)
  its own compare inside the digit core     2,374,371,162     +572,400  (+0.0241%)
  folded into the arm already there         2,374,180,362     +381,600  (+0.0161%)
```

The third is one instruction per render and there is no shape below it: the
field has three classes — zero, all-ones, and everything between — and no
single test separates three classes. The 381,600 is that instruction times the
renders runbench makes through ryu.

Welfare falls by less than the two decimal places it prints and more than the
0.001 the gate allows, so the run goes red. `scripts/welfare/welfare.kso:38`
answers that case in its own words: a change that makes the engines agree is
not weighed at all, and a fix for a differential-law violation ships with the
floor moving to whatever it costs. `--set` cannot lower a floor, by Clay's
2026-08-03 ruling, so the new value is written into
bench/welfare_floor.json by hand where a reviewer sees it in the diff. That
comment was written after the model spent a day looking able to refuse a fix
worth four hundredths of a per cent; this is the same shape at 0.0161%.

CI's rows, and what each one is. The container's +381,600 on runbench
transferred to the runner to the instruction, which is worth saying: the
prediction and the sitting agree exactly, so the cost is the branch and not
the host.

    work_runbench      2,397,582,951 -> 2,397,964,551   +381,600  (+0.0159%)
    work_encodebench   4,058,633,349 -> 4,060,329,349 +1,696,000  (+0.0418%)
    work_livebench     3,596,075,294 -> 3,597,771,294 +1,696,000  (+0.0472%)
    work_oneshot          21,758,011 ->    21,762,251     +4,240  (+0.0195%)
    text                   1,470,188 ->     1,471,084       +896

The two encode rows move by the same 1,696,000 because livebench runs
encodebench's program against the library that ships rather than the frozen
copy, so the same renders are counted twice over. The text vein is +64 bytes
on every one of the fourteen rows, which is the branch's own code.

Two counters IMPROVED and neither is this change being clever.
`work_widebench` fell 36,127,282 -> 36,104,947, and `compile_instructions`
19,316,711 -> 19,316,381. Both are layout: src/runtime.c is `include_str!`'d
into the compiler, so its bytes move the binary under it, and CLAUDE.md
records seven layout-only moves of the compile row before this one.
Widebench's floats are mostly integral and take the fixed-point fast path
before ryu is reached at all, so its fall cannot be the new branch executing
less; the 22,335 is 0.062% of the row and sits where layout noise sits.

compile_instructions is a published claim, so docs/compiler.html quotes it
twice and both quotations moved with the golden. `golden_prose` is what caught
them, on the round that had everything else right -- which is the gate working:
a figure on the page and a figure in a golden are the same number or the page
is wrong.

## 2026-09-08 (eighth) — a short copy went through the call, twice more

kanso#1317 fixed `k_b_join`'s one-byte copies and left a map of where the rest
of runbench's `memcpy` time sits. Two of those sites are the same shape and
neither needed a call at all.

`k_str_n` builds every string the runtime does not already hold, and its copy
went straight to libc. From `split` alone that call is reached 177,706 times a
run at 12.9 instructions apiece; the pieces being moved are two or three bytes,
so the call is nearly all of it. `render_ryu` writes ryū's digits into the
output buffer with three byte loops, and clang outlines each of them into a
`memcpy` call: 191,070 a run at 15.3.

Both now go through `k_copy_short`, which moves anything under sixteen bytes as
two overlapping loads and two overlapping stores. Reads stay inside
`[s, s+n)` and writes inside `[d, d+n)`, so no caller needs slack at either
end — the same idiom `k_b_utf8_slice_raw` has shipped since kanso#1294.

**Measured on the container, each side rebuilt, run from the repo root, against
the tree this lands on — main plus kanso#1319:**

```
  baseline           2,374,180,362
  k_str_n only       2,372,135,490   -2,044,872  (-0.0861%)
  render_ryu only    2,370,339,882   -3,840,480  (-0.1617%)
  both               2,368,295,010   -5,885,352  (-0.2479%)
```

The two rungs are additive to the instruction: 2,044,872 + 3,840,480 =
5,885,352. Output md5 identical on all four builds. That is 2.7x what
kanso#1317's own change recovered, from the same map and the same reading of
it.

Of the render_ryu rung's 3,840,480, some 913,860 is render_ryu's own self cost
falling and 2,926,620 is the `memcpy` calls it no longer makes — the figure the
kanso#1317 map already attributed to render_ryu, to the instruction.

**These two are not independent of kanso#1319, and the earlier note in this
file saying they were is wrong.** kanso#1319 added an unsigned compare to
`ryu_d2d`, which is inlined into `render_ryu` in every profile, and it cost
+381,600. On the patched tree it costs nothing:

```
  render_ryu self cost, no kanso#1319, byte loops        90,045,360
                        kanso#1319,    byte loops        90,426,960   +381,600
                        no kanso#1319, k_copy_short      89,513,100
                        kanso#1319,    k_copy_short      89,513,100         +0
```

`both` reads 2,368,295,010 on either tree, to the instruction. So the −5,885,352
above pays back kanso#1319's whole cost along with its own. Which transform
folds the compare away once the digit copies are inline is not established here
and is not guessed at: the measurement reproduces, the mechanism is open.

An earlier sitting of this ladder read a baseline of 2,397,583,080 and deltas of
1,914,270 / 3,458,430 / 5,372,700. That baseline does not reproduce: a clean
measurement of the same tree reads 2,373,798,762, which is the figure kanso#1317
reports for its own patched side. The table above is the sitting that survives
re-measurement, and the earlier one is recorded here so nobody cites it.

`sh scripts/gates/all_counters.sh` says the twelve cost veins and the lazy tier
agree: no allocation counter moves, because this changes how bytes are copied
and not how many. `sh scripts/gates/all_compile.sh` says nothing moved that
this host can see, with four gates refusing on a host they may not compare
against.

Rows `str_words` and `ryu_words` in the ratchet, one per site. Neither was
watched red here: `scripts/gates/instructions.sh` refuses to compare on this
container, whose glibc and rustc are not the pair the golden names, so a local
mutation run cannot fail for the right reason or any other. What stands behind
the rows until CI runs them is that each mutation reverts exactly one site and
both of the trees they produce are in the table above, millions of instructions
from the ladder against a gate that asserts equality.

**What is left of the map.** `k_closure`, `k_b_put_mut` and `k_mklist` each
read 22.0 instructions an average call and each already carries an `n <= 4`
element loop, so their residual is the copies longer than four `KValue`s.
Raising that threshold is a tuning question of its own — kanso#1209 moved a
different cap from four to eight and had to measure it — and the answer here is
not assumed. `k_b_append_grow`'s 29.2M at 82.7 a call remains a real buffer
copy and is not this shape.

**CI's rows.** The container may not write the host-keyed veins, so these are
the linux runner's, copied in. Twelve of the fourteen work rows moved and ten
of them fall:

```
  jsonbench    1,487,045,449 -> 1,485,345,052   -1,700,397  (-0.1143%)
  encodebench  4,060,329,349 -> 4,043,253,556  -17,075,793  (-0.4206%)
  oneshot         21,762,251 ->     21,708,250      -54,001  (-0.2481%)
  widebench       36,104,947 ->     35,967,285     -137,662  (-0.3813%)
  pendbench      590,979,348 ->    590,971,748       -7,600  (-0.0013%)
  scanbench      730,307,043 ->    726,019,157   -4,287,886  (-0.5871%)
  livebench    3,597,771,294 -> 3,580,692,761  -17,078,533  (-0.4747%)
  runbench     2,397,964,551 -> 2,392,210,251   -5,754,300  (-0.2400%)
```

`deepbench` and `escapebench` are byte-identical. The runner's runbench delta
is 5,754,300 against the container's 5,885,352 — the same change on a different
glibc, where the call it removes is a different call.

**Four work rows RISE and are priced here by name and landed value**, as the
trend gate asks: `basket` 34,684,338 -> 34,694,178 (+9,840, +0.0284%),
`indexbench` 3,265,784 -> 3,265,786 (+2), `digestbench` 10,420,391 ->
10,420,396 (+5), `readbench` 4,287,134 -> 4,287,137 (+3). The three
single-digit moves are layout; `basket`'s 9,840 is the same, and its own
allocation counters are byte-identical, which is what says no work was added.

**All fourteen .text rows rise**, which is what inlining a copy does — the
bytes the call used to stand for now sit at each site:

```
  jsonbench     94,962 ->  96,418  +1,456      pendbench     86,722 ->  87,666    +944
  encodebench  115,090 -> 116,546  +1,456      indexbench    55,586 ->  56,402    +816
  oneshot      105,986 -> 107,442  +1,456      scanbench    154,386 -> 155,602  +1,216
  basket       109,858 -> 110,978  +1,120      digestbench  105,682 -> 106,498    +816
  widebench    120,514 -> 121,970  +1,456      readbench     51,954 ->  52,930    +976
  deepbench     70,482 ->  71,250    +768      livebench    106,562 -> 108,018  +1,456
  escapebench   51,618 ->  52,386    +768      runbench     241,682 -> 243,458  +1,776
```

The 2026-09-05 gavel keeps machine-code size out of welfare and in its own
exact vein, so these rows are recorded rather than weighed.

**`compile_instructions` 19,316,381 -> 19,316,962 (+581)**, and the page's two
`data-golden="compile.compile_instructions"` spans move with it. `kanso check
lib/json` stops before codegen, so no decision this row counts has changed;
`src/runtime.c` is `include_str!`'d into the compiler, so its bytes move the
binary's layout. `compile_allocs` and `compile_peak_bytes` are byte-identical,
which is what separates a layout move from a real one.

**And the threshold is right where it is.** Raising all four element loops from
four to eight — `k_rec`, `k_mklist`, `k_closure` and the list push — costs
runbench 798,725 instructions, 2,368,295,010 to 2,369,093,735 (+0.0337%),
measured on this host with each side rebuilt. kanso#1209 moved a different cap
from four to eight and gained; this one loses, because the counts past four are
rare enough that the extra compare on every short copy outweighs the calls it
removes. Declined, and the four stay at four.

Named by the keys the trend gate reads, each with the value it landed on:
`work_basket` 34,694,178, `work_indexbench` 3,265,786, `work_digestbench`
10,420,396, `work_readbench` 4,287,137, and `text` 1,471,084 -> 1,487,564,
the sum of the fourteen rows above.

## 2026-09-08 (ninth) — the compile term is measured on a workload it names

Ruled on 2026-09-08: the compile term reads a fixed corpus rather than
lib/json. The reason is kanso#1291. That change retired the escape fold,
`lib/json` stopped importing `std/list`, and all three compile rows roughly
halved — with the compiler byte-identical. By the objective's arithmetic that
was a four-point rise. By what the term is for, how expensive the compiler is
to run, it measured nothing at all.

`bench/compile_corpus` is a package that names its imports: `std/json`,
`std/list`, `std/testing`, `std/text`, each used, because an unused import is
a refusal. `scripts/gates/compile_allocs.sh`, `compile_memory.sh` and
`compile_instructions.sh` check it instead of lib/json, and
`scripts/compile_row_probe.sh` follows so the probe answers the same question
the gates do. lib/json is still most of what gets compiled, since the corpus
imports it; what changed is that a row now moves by a compiler change or by an
edit to the corpus, and not by a library changing its mind about a dependency.

**`library_box.sh` stages `lib/` and nothing else**, so a gate asked for
`compile_corpus` would have found no such package. It gains one `cp -R` line.
The corpus lives under `bench/` rather than `lib/` because a benchmark is not
the library — which is exactly why the line is needed.

**The rows this host may write.** `front_end_rounds` 35 -> 62 and
`front_end_visits` 9,884 -> 23,723. Those two count the compiler's own
algorithm and are the same on every host, so `compile_memory.sh` compares them
before it reaches its host check and this container may measure them.
`compile_peak_bytes`, `compile_allocs` and `compile_instructions` are
host-keyed, so round one of this PR left them at their lib/json values and went
to CI RED on all three by design: `host_gate.sh` exists because "a container's
numbers going into a golden over the runner's is the exact accident measured_on
was written after", and its refusal prints the sitting to copy.

**The rows CI wrote.** `compile_allocs` 11,613 -> 31,596, `compile_peak_bytes`
375,222 -> 789,740, `compile_instructions` 19,316,962 -> 52,603,220. Every
runtime vein stayed green in that round, which is the evidence that a corpus
change does not reach the runtime. `front_end_rounds` 35 -> 62 and
`front_end_visits` 9,884 -> 23,723 were already written here, from this
container, for the reason above.

Worth recording: `compile_allocs` 31,596 and `compile_peak_bytes` 789,740 are
what this container read, to the unit. Only `compile_instructions` was
genuinely host-dependent — the container had 52,608,ish against the runner's
52,603,220 — so of the three rows the host gate refuses to let a container
write, two would have been right anyway. That is not an argument for relaxing
the gate; it is a note that the gate's cost is one row, not three.

The corpus is a bigger program, so the compile terms rise and welfare falls:
66.02 -> 59.74.

**The floor moves DOWN by hand, and that is a re-basing rather than a
regression.** `--set` cannot lower a floor — Clay's 2026-08-03 ruling — so the
new value goes into `bench/welfare_floor.json` where a reviewer sees it in the
diff. CLAUDE.md says moving the floor to accommodate a change while leaving the
weights alone is declaring the objective wrong without saying so. That rule is
about a change that makes the COMPILER worse. This one does not touch the
compiler; it changes what the term is measured on, which is the third state the
trend gate already models: a re-basing counts toward neither side, because a
ratio between two definitions cannot say which way the compiler went. The
authority is the ruling.

**Two things a reader should see rather than discover.**

`std/testing` is imported today by `lib/json/json_test.kso` and by nothing
else, and `library_box.sh` DELETES every `*_test.kso` before measuring. So the
compile rows have never compiled `std/testing`, and naming it in the corpus adds
a module they have never counted. The 2026-08-25 fault that `library_box.sh`
was written after was incidental drift — a test file's imports leaking into the
measurement. This is the opposite: the workload is named on purpose. Built as
ruled, stated here so the addition is visible.

And the ruling names list, text and testing, while the benchmarks import more
than that: runbench alone pulls `std/json`, `std/list`, `std/text`,
`std/regexp`, `std/sha256`, `std/io` and `std/os`. Built as ruled; the wider
set is recorded rather than assumed either way.

**A one-word edit could revert all of this silently** — `compile_corpus` back
to `lib/json` in any one gate — and every golden would simply be re-based to the
new workload and agree with itself. `tests/the_compile_term_reads_the_fixed_corpus.rs`
asserts the workload by name in all four readers, that the box stages it, and
that the corpus still names the four modules. All three assertions were watched
red first: a gate reverted, the staging line deleted, and an import dropped.

**The welfare BASELINE does not move with the workload, and here is why that
was checked.** `bench/welfare_floor.json` carries a baseline for each term —
`compile_allocs` 62,110, `compile_instructions` 56,563,967,
`compile_peak_bytes` 819,217 — all measured on lib/json. The obvious worry is
that scoring a corpus reading against a library baseline compares two
different programs, and that the baseline has to be re-measured too. It does
not. CLAUDE.md settles it: the number "is an index, not a percentage — the
ceiling is a hundred, where every term costs nothing, and the origin is
arbitrary. Only its direction and the size of its moves mean anything." An
arbitrary origin stays where it is; the term's value falls once, the floor
absorbs that fall, and every move after it means what it always did. Moving the
baseline as well would hide the fall rather than record it.

---

## 2026-09-08 — A MODULE WAS REWRITTEN TWICE BEFORE IT WAS CHECKED, AND ONCE IS ENOUGH

`compile_module_loaded` ran four rewrite passes, checked the merged program,
and then ran the same four passes again:

    finish_program
    desugar_field_reads
    prune_unused_getters
    trmc::rewrite
    check_merged                     <- the whole-program check
    canonicalize_types
    canonicalize_bare_aliases
    hoist_repeated_strings
    fuse_enumerable
    finish_program                   <- again
    desugar_field_reads              <- again
    prune_unused_getters             <- again
    trmc::rewrite                    <- again

The second run is the one the emitter reads, because the four passes between
them can produce work for all four. The first run's output is read by
`check_merged` and by nothing else, and `check_merged` does not require any of
it: the four passes rewrite field reads into getter calls, drop getters
nothing calls, and turn tail-recursive modulo cons into a loop, none of which
the check asks about. So the first run is deleted and the check reads the
merged program as merged.

    compile_corpus  53,949,299 -> 52,170,583   −1,778,716   −3.30%

read with `scripts/compile_row_probe.sh` on this container, environment
emptied and the glibc tunables pinned, so the two sittings differ only in the
compiler. The row this host may not write is CI's to re-sit.

**The risk here was ordering, not cost, and `emitted_code` answers it.** Four
passes moved from before a check to after four other passes is a
reordering, and a reordering can change what the emitter is handed even when
every pass is individually sound. `all_compile.sh` reports `emitted_code`
AGREED, byte for byte, along with `compile_libraries` and `compile_cost`, and
"compile veins: nothing moved that this host can see". The same bytes come out
of a compiler doing less work to produce them, which is the whole claim.

Diagnostics are unmoved: all seven `tests/golden/errors_module` fixtures are
byte-identical. That is the corpus that would have caught a check reading a
differently-shaped tree, and it is quiet.

The ratchet row `rewritten_twice` restores the four deleted lines and asks
`compile_instructions`. The mutated tree measures 53,936,404 against the
52,170,583 the row is pinned to, so the gate goes red — the only witness a
repeated rewrite leaves, since it emits the same bytes.

**The mutation anchors on the check line, not the pass it inserts before.**
`inline::inline_builtin_wrappers(&mut merged);` appears twice in `src/lib.rs` —
the single-file compile paths call it too — so a `grep -cF` guard on it
refuses. `phase::watched("check_merged", ...)` appears once, and the guard
asserts that before inserting. Written into the mutation's own comment so the
next person to touch it does not rediscover it.

OPEN, and larger: `check_merged` still runs once per dependency rather than
once for the program. Seven calls walking 803 declarations, 394 distinct, 409
of them repeats — 53,949,299 -> 44,637,899 with the per-dependency calls
gated off, −17.26%. It is not a deletion, because two error fixtures depend on
it: `field_read_in_a_deep_library` loses its diagnostic entirely and
`deep_library_error` attributes the fault to `mid` instead of `deep`. The
blocker is `desugar_expr` at `src/lib.rs:1917`, which rewrites every
`Expr::Field` into a getter call, so `check_field_exists` — which matches on
`Expr::Field` — sees nothing after it has run. Making the whole-program check
carry those two diagnostics is a design question and goes to the ledger, not
into this entry.

---

## 2026-09-08 — CI'S ROWS FOR THE SINGLE REWRITE, AND THE DIAMOND MEMO DECLINED

CI re-sat the two compile veins the container may not compare:

    compile_instructions   52,603,220 -> 50,832,211   −1,771,009   −3.3668%
    compile_allocs             31,596 ->     30,414       −1,182   −3.74%

Both FALL. `compile_peak_bytes` is byte-identical at 789,740, and every other
vein in the cost-goldens summary reads `:success`. The container had predicted
−1,778,716 on the instruction row with the tunables pinned, which is the same
number to four digits on a different toolchain.

`compile_allocs` is the row worth pausing on, because nothing in the tree could
see it before CI did. The host gate refuses to let a container compare it, so
`all_compile.sh` reported "nothing moved that this host can see" while the row
had in fact fallen 3.74%. #1321 found that this container reads
`compile_allocs` and `compile_peak_bytes` exactly as the runner does, and this
round is the case where knowing that would have saved a red one: the gate's
refusal is about the instruction row, and it takes the other two down with it.

Welfare 59.74 -> 59.95, ratcheted in the same PR.

### The diamond memo, built and declined

`std/text` is compiled twice on the fixed corpus — once under `std/json`, once
under the corpus — because `compile_module_inner` has no memo and `visited` is
a cycle guard that empties as each module returns. A thread-local
`Map<PathBuf, Program>` keyed the way the cycle guard keys, cleared by a
`fresh_build()` that all four roots go through, with `ast::Program` given
`Clone`:

    baseline                      52,170,583
    memo written, never read      55,185,945   +3,015,362   seven clones
    memo written and read         53,908,630   +1,738,047

It is correct — `std/text` loads once and all seven `tests/golden/errors_module`
fixtures stay byte-identical — and it costs 431,000 instructions a module to
keep a program that only one of them is ever asked for twice. The saved
recompile is real and worth 1,940,015, which the clones spend twice over.

Reversing the memo does not fix it. A module is not known to be shared until
the second importer asks, and the first compile's program is gone by then, so
keeping it costs one deep clone per module compiled whether or not anything
reuses it. `Rc<Program>` does not help either: every importer calls `qualify`,
which renames the dependency's declarations into that importer's namespace, so
each importer needs an owned copy regardless.

Two shapes would win, neither small. One is an import-graph pre-pass that
counts importers before compiling, so only shared modules are kept — it has to
duplicate the whole resolution surface, hako pins and embedded modules and
handed sources and the `./` forms, which is where it stops being cheap. The
other is to cache each module's OWN declarations rather than its merged
program; the clone then scales with the module instead of with its whole
dependency closure, which is why `std/json`'s costs 431,000 in the first place.
That second shape also reaches the reason the diamond is expensive at all:
every importer merges a full copy of every transitive dependency.

Measured with `scripts/compile_row_probe.sh`, environment emptied and the glibc
tunables pinned. Reverted; nothing of it is in the diff.

---

## 2026-09-08 — A MODULE'S PATH IS SHARED, NOT COPIED ONCE PER DECLARATION

`stamp_file` wrote `decl.file = file.to_string()` — one allocation per
declaration, for a path the corpus has about seven distinct values of. The
declarations then get cloned by enrollment, and the sets in `linear.rs`,
`beat.rs` and `codegen.rs` that key on `(file, line, col)` cloned the whole
path again on every insert and lookup. `FnDecl.file` is an `Arc<str>` now:
`stamp_file` allocates once per module and hands each declaration a refcount
bump.

    compile_allocs        30,414 ->    29,941      −473   −1.56%
    compile_peak_bytes   789,740 ->   777,031   −12,709   −1.61%
    compile_instructions       container −607,236  −1.164%

The first two are this container's readings, which is ordinarily what the host
gate refuses. The licence is two agreements: #1321 found that allocs and peak
match the runner here to the unit, and #1323 confirmed it — the container read
30,414 and so did CI. The instruction row is NOT written that way and is left
for CI, because that one really is host-dependent: 52,170,583 here against
CI's 50,832,211 for the same tree.

**`Arc`, not `Rc`, and the reason is a thread.** The compiler looks
single-threaded — thread-locals throughout — but `src/main.rs:379` spawns a
scoped thread to run the interpreter on a pinned 8 MB stack, which is the
kanso#1287 gate. The program crosses that boundary, so `Rc` does not compile
there. `Arc`'s clone is an atomic increment where `Rc`'s is a plain one, and
that is still far cheaper than an allocation.

**The empty path had to be shared too, and the first measurement said so.**
`String::new()` allocates nothing; `Arc::from("")` allocates. The parser builds
every declaration with an unstamped file, so a fresh `Arc` apiece added one
allocation per declaration where the stamp removed one — the first reading was
30,339, a fall of 75 rather than the 473 the change is worth. `ast::unstamped()`
hands out one shared empty `Arc` and the rest of the fall appears.

### `Sites` was two keys wearing one shape

The alias in `linear.rs` said it outright: "A set of source positions, or of
(group, arity, index) triples — the two happen to have the same shape." They
do, and a blanket change of that shape compiles almost everywhere it should
not. `linear_params`, `byte_disc` and `builder_params` key on a declaration's
NAME with an arity and a parameter index; `in_place_pushes`,
`reusable_records`, `Sites` and `MutSites` key on its FILE with a span. One
`string_builders` call returns all three of `(joins, params, carried)` — two
file-keyed and one name-keyed — under the single alias.

`Sites` is the file-and-span one now and `Slots` is the name-and-index one, so
the next person to change either finds the compiler telling them which is
which. This is the same class of thing as the ratchet rows that went blind
because nothing named what they watched.

The ratchet row `shared_path` restores `Arc::from(&*file)` in the stamp loop —
a fresh path per declaration — and asks `compile_allocs`. Under it the corpus
reads 30,309 against the 29,941 the row is pinned to, and peak 789,087 against
777,031, so the gate goes red. The anchor is the shared bind, which appears
once; `Arc::clone` is spelled at several sites now and a guard on it would
refuse for the wrong reason.

All seven `tests/golden/errors_module` fixtures are byte-identical and
`all_compile.sh` reports emitted_code AGREED. An `Arc<str>` is immutable, so
sharing a path between declarations cannot alias a write — nothing in the tree
mutates a declaration's file after stamping it.

One spec moved, and repairing its number was the wrong repair.
`tests/import_order.rs` pinned the peak difference between two modules that
declare the same two functions and differ only in which file names `std/list`.
It read seventeen bytes; on this host it now reads fifteen, so the pin was
moved to fifteen and pushed.

The arm64 runner had been refusing it since the FIRST push of this branch, on
the pin of seventeen, and it read TWENTY-THREE. Three rounds went by with
`the other host` red before that log was opened, because the row this change
was about was on the x86 side and the macos job was read as one more thing
still running. Repairing a two-host pin from one host's reading is the error,
and it is a larger one than the number: the second reading was sitting in a
job log the whole time.

Both hosts are deterministic and both are right. `compile_peak_bytes` reports what the allocator holds, and glibc and
macOS round a merge of the same declarations differently, so the residual is
not a property of the compiler at all. Seventeen agreeing on both hosts before
this change was luck.

So the spec asserts the ORDER now, which is what it was always about — its own
title says which file names a dependency must not change what checking costs,
and `import_list.sort_by` in `load_dependencies` is the line that makes it
true. Removing that line is the mutation, and under it the two modules load
`list, text, render` and `text, list, render`, with the second's peak going to
512,480 against the first's 483,682. Watched red exactly there and green with
the sort restored. The peaks are still read, so a module that stops checking
still fails, but nothing pins their difference: a number that moves with the
allocator was pinning the wrong thing, and this is a repair rather than a band.

**CI's rows, and the licence's third reading.** The runner counted
`compile_instructions` 50,685,978 against the 50,832,211 the golden held — a
FALL of 146,233, or 0.2877%. What went is the copying itself: 803 path copies
on the fixed corpus, and the byte comparisons the `(file, line, col)` keys no
longer do on every insert and lookup. The row is the front end doing less
rather than the layout vein moving, and `compile_allocs` falling beside it is
what says so.

Both rows written from this container came back from CI unchanged —
`compile_allocs` 29,941 and `compile_peak_bytes` 777,031, the second read back
verbatim in the job log as `front end holds 777031 bytes; golden 777031`. That
is the third agreement, after #1321 and #1323, and the first one where the
container wrote the rows into the branch before CI had said anything. The
licence stands. A disagreement would have mattered more than this change does,
which is why the round was arranged to make one visible.

Welfare 60.03 -> 60.04, banked in the same PR.

---

## 2026-09-08 — THE ENTRY PATH REWROTE ITS PROGRAM TWICE TOO, AND NOTHING COULD SEE IT

kanso#1323 deleted the doubled rewrite group from `compile_module_loaded`.
`compile_parsed_entry` had the same shape and was not touched:

    check_merged
    finish_program                   <- first group
    desugar_field_reads
    prune_unused_getters
    trmc::rewrite
    inline_builtin_wrappers
    if the check passed:
        canonicalize_types
        canonicalize_bare_aliases
        hoist_repeated_strings
        fuse_enumerable
        finish_program               <- again
        desugar_field_reads
        prune_unused_getters
        trmc::rewrite

The first group is deleted, which leaves this path in the module path's order.
Nothing between the two reads the first group's output except
`inline_builtin_wrappers`, and the second group redoes all of it. It also ran
unconditionally — including on the way to refusing a program the check had
already rejected, where the rewrite has no reader at all. `kanso::compile`
(src/lib.rs:28), the third compile path, already ran the group once, so this
was `compile_parsed_entry`'s alone.

### The counter exists because nothing in the tree could see this

Checked rather than assumed, one gate at a time:

- `compile_instructions`, `compile_allocs` and `compile_memory` read
  `kanso check compile_corpus`. compile_corpus is a MODULE — the phase trace
  prints `load compile_corpus` — so the gate goes through
  `compile_module_loaded` and never reaches `compile_parsed_entry`.
- `bench/compile_golden_modules.txt` DOES run this path: `module_entry`
  (tests/compile_cost.rs:52) calls `kanso::compile_entry`. Its columns are
  rounds and visits — the INFERENCE fixpoint's, which a rewrite pass does not
  touch — and lines, calls, branches and defines, which are the emitted IR and
  byte-identical whichever way round the passes run, because they are
  idempotent on their own output. The right workload, the wrong dimension.
- `emitted_code` proves the change is safe. It cannot prove the change did
  anything.

So `kanso::rewrite` counts pass invocations, the way `infer::work::passes`
counts whole-program inferences and for the same reason its spec gives: "a new
diagnostic that calls infer for itself raises the real cost without moving
either number. One did, and every gate in the repository stayed green." On
`tests/golden/compile/module/main.kso`, the sample both compile goldens already
use, the count is 20; with the four lines restored it is 24.
`tests/rewrite_passes.rs` pins the 20 and was watched red at 24 first.

The ratchet row `entry_rewritten_twice` restores the four lines and asks that
spec. Its anchor is `check::check_merged(&merged, true)`, which appears once —
`check_merged` is called four times in src/lib.rs and `finish_program` many
more, so neither of those is a guard that can refuse.

**The compile row was expected not to move; it rose 502, and the trend gate
refused that.** Two things were wrong in sequence, and the second is the one
worth writing down.

The prediction's reasoning was right as far as it went: compile_corpus is a
module, so the gate's workload goes through `compile_module_loaded` and never
reaches `compile_parsed_entry`, and none of the four deleted lines is on it.
What it left out is that the counter added to find them was. `rewrite::pass()`
sat inside `finish_program`, `desugar_field_reads`, `prune_unused_getters` and
`trmc::rewrite` — the same four the module path calls. Seven modules on the
corpus, four rewrites each, about twenty-eight bumps at roughly eighteen
instructions apiece: 50,685,978 -> 50,686,480. Every other vein agreed, with
`compile_allocs` and `compile_peak_bytes` byte-identical, which is what says
the front end was doing the same work plus a counter.

The wrong response was to regenerate the golden and write a note explaining the
rise. `scripts/trend_gate` refused it:

    worsened: compile_instructions 50,685,978 -> 50,686,480
    FAIL  a pure regression: something got worse and nothing got better.

That gate is right and the reasoning behind the regeneration was not. Welfare
being indifferent — 502 on 50.7M sits inside the 0.001 band `welfare.kso:686`
compares with — is not a licence, because the trend gate is a separate and
stricter rule: a counter may rise when something else falls, and here nothing
fell. Paying 502 instructions on every module compile for a number only a spec
reads is a bad trade however small it is.

So the counter moved to the four call sites in `compile_parsed_entry`, which
takes it off the measured path altogether: the gate checks a module and enters
neither the deleted rewrites nor the bumps that replaced them.
`tests/rewrite_passes.rs` now pins 4 rather than 20 — the entry group alone, not
the entry group plus every module's — and was watched red at 8 under the
restored group before it was believed.

The row did not come back to where it started. CI read **50,685,288**, a FALL of
690 from main's 50,685,978, with `compile_allocs` and `compile_peak_bytes`
byte-identical. Nothing the gate executes changed, so this is the layout vein
that this golden's own history records moving seven times before on edits to the
compiler's Rust. It is an improvement rather than a cost, `scripts/trend_gate`
reads it as one, and the golden is regenerated down with that reason written in.
The three readings together are the useful record: 50,685,978 with no counter,
50,686,480 with it inside the four functions, 50,685,288 with it at the entry
call sites.

**The placement costs something and the spec says so.** `infer::work` counts
inside `infer`, which catches any caller anywhere; that is the property its own
comment was written for. This counter catches a fifth rewrite added to the
entry group and does not catch one added by some other caller. That gap is
real, it is written in the spec's header and in the mutation, and it is the
price of not charging every module compile for the watch.

The saving the change makes is on the entry path and no gate holds it: an entry
program is walked four fewer times, and `tests/rewrite_passes.rs` is the only
thing in the tree that can see it.

All seven `tests/golden/errors_module` fixtures are byte-identical, and
`all_compile.sh` reports emitted_code AGREED, compile_libraries AGREED and
compile_cost AGREED. That reading is load-bearing rather than inherited:
`compile_parsed_entry` puts `inline_builtin_wrappers` BETWEEN the two groups
where `compile_module_loaded` puts it before both, so #1323's emitted_code
reading does not carry over to this one.


## Checking and rewriting once at the top: six passes have to stay where they are

The idea was that `compile_module_loaded` does the whole-program check and the
eight rewrites once per module, on a growing prefix of the program, and that
the outermost compile could do all of it once on everything. An ablation had
put the saving at 10,632,023 instructions, 20.38% of the compile row.

Gated on `DEPTH <= 1 && !ENTRY_COMPILE`, the change builds, and `tests/golden` goes red
in three tests. The smallest reduction says why:

    printf 'import "./trmc_count"\n\ntrmc_count/play\n' > main.kso
    kanso run main.kso
    error[runtime]: the program ran out of stack: recursion went deeper than
    the stack holds

`src/trmc.rs:185` declines the group:

    if crate::ast::has_slash(name) || *arity == 0 { continue; }

A dependency's declarations are qualified on the way in, so `count` reaches the
top as `trmc_count/count` and trmc will not touch it. The accumulating tail
call stays a tail call, and a million frames overflow the stack. trmc only ever
rewrote a dependency because it ran inside that dependency's own compile, while
the names were still bare.

It is a family. Six passes skip qualified declarations by construction:
`trmc::rewrite`, `typeset_constructions` (check.rs:1768),
`foreign_constructions` (check.rs:1820), `check_bare_ambiguity` (check.rs:2957,
at two sites) and `canonicalize_bare_aliases` (lib.rs:929). Each is about a
module's OWN declarations — what this module constructs, which bare name its
arms make ambiguous, which aliases it spells short. Run once at the top they
apply to the root's declarations and skip every dependency's. Four of the six
are checks, so the failure mode is a refusal that stops being raised.

So a large part of the 10.6M is not redundant work removed. It is work that
stops happening, and an ablation that does not keep those six passes per module
measures the wrong thing. The number was real; the inference from it was not.

**The same conclusion was reached in kanso#1003** and withdrawn there as "the
per-dependency check_merged is not redundant". That entry did not name the
mechanism, which is why the route was open to walk a second time. The mechanism
is `has_slash`, and it is written down now.

What survives is most of it, and callgrind on the fixed corpus says how much.
On the container with the tunables pinned, against 51.6M total:

    kanso::main                         51,094,622   99.09%
    check::check_merged                 19,092,779   37.03%
      infer::infer                      11,803,600   22.89%
      check::check_file_shadow           1,812,314    3.51%
      the four guarded checks             under 0.02%
    canonicalize_bare_aliases            1,651,829    3.20%  (guarded)
    trmc::rewrite                          504,558    0.98%  (guarded)

The four guarded passes inside `check_merged` cost almost nothing —
`foreign_constructions::walk` is 8,301 instructions across both call sites and
the other three fall below the threshold — so keeping them per module is free
and the rest of `check_merged` can move. `infer`, the largest piece, carries no
`has_slash` at all. The two guarded REWRITES are what has to stay: 1,651,829
and 504,558, 2,156,387 together.

So the reachable win is about 8.5 million instructions, roughly 16.5% of the
row, against the 10,632,023 the ablation reported. The change is viable at that
size, with `canonicalize_bare_aliases` and `trmc::rewrite` left per module and
the four guarded checks kept as a small pass beside them. An ablation that does
not keep those measures the wrong thing again.

The peak term was priced before any of this and was never the obstacle: even a
29% rise in `compile_peak_bytes` leaves a 10.6M fall ahead of the floor
(60.04 -> 60.12), and the realistic 32,851 bytes of carried sources score
60.70.

## The corpus says the reshape is not ready, and the earlier reading hid it

Applied on top of e12bfaba and built, `cargo test --release --test golden` reports
nine of ten tests passing and one failing. That reads like one fixture and it is
not: `error_corpus_reports_each_golden_diagnostic` asserts inside a loop, so it
stops at the first mismatch and says nothing about the rest. Driving all 193
fixtures by hand — the same staged-entry harness the test uses — puts the count
at **44 changed, 149 byte-identical**, in three classes.

**31 carry an `.imported.stderr` golden** and simply move from the loader's
`(module X)` suffix to the `--> file:line:col` form. That is the reshape working
as designed: the check now runs at the top, so the whole-program renderer writes
the diagnostic instead of the module one. (The comment above that test says 23
fixtures gain the suffix. There are 34 on disk. The comment is stale.)

**Four leak a qualified name into the message a user reads.** At the top a
dependency's declarations carry their module prefix, and these messages print
the declaration's name:

    golden:  `point` takes 2 argument(s), and a list element is one atom …
    actual:  `constructor_in_a_list/point` takes 2 argument(s), …

    golden:  these `open_start?` arms tie …
    actual:  these `an_arm_set_with_no_settling_arm/open_start?` arms tie …

with `field_of_the_wrong_record/point` and
`field_of_an_annotated_parameter/money` the same shape. The user wrote `point`.

**Three name the wrong file outright**, which is worse:

    golden:  no import offers a pub `nonexistent` to re-export
             --> a_reexport_of_a_name_nothing_offers.kso:3:5
    actual:  no import offers a pub `nonexistent` to re-export
             --> std/text/text.kso:3:5

The line and column are the user's; only the file name is wrong, so the excerpt
quoted underneath is std/text's line 3 under the user's error. `a_wall_whose_
right_side_is_a_name` lands on `std/io/io.kso:10:27` the same way.

The attribution patch is what should have prevented this, and the way it fails
is worse than not attributing at all. It hangs a file on each diagnostic through
`diag::attributing(&program.fns)`, a guard held while walking one declaration.
Every site that raises inside a `fn` walk is attributed and lands on the right
file — which is why the 31 above are correct.

The re-export check raises outside any walk, and it does not fall back: it
INHERITS. `render_across` prefers `d.file` whenever it is set, and the
thread-local still holds whatever the last walk left in it. Reproduced directly
on the smallest program — an entry importing the fixture — with the phase report
beside it:

    load a_reexport_of_a_name_nothing_offers.kso
    load std/text

    error[name]: no import offers a pub `nonexistent` to re-export
      --> std/text/text.kso:3:5

The name is the LAST MODULE LOADED. A guard that had simply been absent would
have left the file the renderer was handed standing; a leaked one overwrites it
with an unrelated library, and quotes std/text's line 3 underneath the user's
error. `src/lib.rs:3287` and `src/lib.rs:3206` are the two sites, both inside
`apply_reexport` under the re-export elevation.

So the fix is not "attribute these two sites". Reading the guard says why it
leaks, and it is a drop-order bug rather than a missing feature.
`Attributed::next` is

    self.held = Some(attributed_to(item.file()));

and the right-hand side runs first. `attributed_to` sets the thread-local to
this item's file and captures the PREVIOUS one; only then is the old guard in
`self.held` dropped, and its `Drop` writes ITS previous back. Item one sets the
file to A with previous None. Item two sets it to B with previous A, then drops
guard one — which restores None. The thread-local oscillates through the walk,
and when the iterator itself drops at the end it restores whatever the last
guard happened to be holding rather than what was live before the walk began.
That is how a std/ path is still standing when the re-export check runs.

One guard for the whole walk fixes it: `attributing` reads what is attributed
before it starts, `next` just sets the current file, and `Attributed`'s own
`Drop` restores the value it captured. A walk that ends, or returns from inside
the loop, then leaves exactly what it found — None at the top level, which makes
the re-export diagnostic fall back to the file the renderer was handed.

BUILT AND MEASURED. The corpus goes from 44 changed to 38, and all three
wrong-file fixtures come back byte-identical:

    error[name]: no import offers a pub `nonexistent` to re-export
      --> a_reexport_of_a_name_nothing_offers.kso:3:5
       3 | pub nonexistent

I had expected the fallback to name the generated `run_<fixture>.kso` and said
so. It does not: the re-export check runs during the fixture module's OWN
compile, where the file the renderer is handed is already the fixture. The stale
attribution was overriding a correct answer, not standing in for a missing one.

What is left is 38, and it divides cleanly. THIRTY-FOUR are the designed move
from the loader's `(module X)` suffix to `--> file:line:col` — 31 carrying an
`.imported.stderr` golden and three (`a_wall_whose_right_side_is_a_name`,
`fields_that_no_one_record_declares`, `sequencing_takes_two_descriptions`) whose
plain golden holds the module suffix without an imported twin, which is corpus
bookkeeping rather than a compiler question. FOUR are the qualified-name leak,
and that is the whole of what is still wrong:

    `constructor_in_a_list/point`            for `point`
    `an_arm_set_with_no_settling_arm/open_start?`  for `open_start?`
    `field_of_the_wrong_record/point`        for `point`
    `field_of_an_annotated_parameter/money`  for `money`

The guard fix is held as $S/419_guard_fix.patch.

So two bounded gaps stand between the reshape and a corpus that agrees: attribute
the module-level check sites the way the declaration walks already are, and print
a declaration's name as the user wrote it rather than as the merge qualified it.
Neither is a performance question, and neither was visible while the golden test
stopped at the first of forty-four.

The diagnostic attribution built alongside it changes no output today and is
held rather than shipped. `Diagnostic` gains an optional file filled from a
thread-local; `diag::attributing` is an iterator that holds the attribution
guard and replaces it per item, so a walk that returns early restores what it
found; `render_across` quotes the right file's line and `render` delegates to
it with an empty map. A field nothing reads is weight, and nothing reads it
while the checks stay per module.

## The check verb infers twice, and nothing sets the toggle that would skip it

`kanso check` runs `infer::infer` over the whole program a second time, at
`src/main.rs:275`, after the front end has already inferred it inside
`check_merged`. The second one is not a duplicate — it is taken after the
rewrites, so its answer differs — and its only reader is the provenance refusal
three lines below it.

The obvious tidy is to move it inside that reader's guard, so a run with
`KANSO_NO_PROV` set does not infer for nobody. That was written and built. It is
not being shipped, because the guard's condition is dead:

    $ grep -rn KANSO_NO_PROV --include=*.sh --include=*.yml --include=*.rs \
        --include=*.kso --include=*.toml .
    ./src/main.rs:276:        if std::env::var_os("KANSO_NO_PROV").is_none() {

Read in one place, set in none. No gate, no script, no test, no benchmark takes
that path, so the change saves nothing anything in this repository ever runs,
while still moving `src/main.rs` — and kanso#1325 spent two rounds learning that
an edit to the compiler's Rust moves `compile_instructions` by layout alone,
where a rise with nothing falling is a pure regression the trend gate refuses.
A coin flip on a round, for a win of zero.

So it stays out until either something sets the toggle or the second inference
can be made to pay for itself some other way. The 11,803,600 instructions that
`infer::infer` costs on the fixed corpus are what makes the second call worth
returning to; the toggle is not the way in.

## Remembering a compiled module costs more than compiling it again

bench/compile_corpus is a diamond. It imports std/text directly, and it imports
std/json, which imports std/text. `KANSO_PHASES=1 kanso check bench/compile_corpus`
prints `load std/text` twice, and the compiler really does lex, parse and check
that module once per path to it. `visited` is a cycle stack — inserted on the way
in, removed on the way out — so it never held an answer to hand back.

The memo was built: `visited` became a `Load` carrying both the stack and a map
from canonical path to the finished program, and a second visit returned a clone
of the first. Every compile counter the objective weighs got worse.

    counter                base         with the memo   delta
    compile_instructions   51,094,624   51,756,188      +661,564  (+1.29%)
    compile_allocs         29,940       32,149          +2,209    (+7.38%)
    compile_peak_bytes     777,057      1,073,098       +296,041  (+38.10%)
    compile_visits         23,723       23,522          -201
    compile_rounds         62           58              -4

(Read on this container, which sits about 400,000 above CI's row for the same
tree; the comparison is against its own base.)

The profile says why, frame by frame. The second compile of std/text is worth
about 1.13 million instructions — `compile_module_loaded'2` falls 27,087,117 to
25,961,126, `lexer::lex` 4,330,753 to 4,092,322, `parser::parse` 4,761,741 to
4,560,696. Handing the answer back costs more than that: `Vec::clone` rises
263,773 to 2,294,028 across its two frames and `Expr::clone` 328,175 to
1,592,439. A module's finished program carries every declaration its own dependencies
contributed, demoted and qualified, so the deep copy runs about 1.6 times the
compile it replaces. The peak rise is the other half:
one program per module stays alive for the whole build where before each one was
dropped as its importer finished with it.

The two counters that improved are not objective terms, and they say how small
the saving is: 201 expression visits out of 23,723, and four fixpoint rounds.

That number is the finding. The merged program is unchanged by the memo, and the
reason is where the diamond's cost actually sits: in the second copy of the
declarations, which every pass downstream then walks. Counting names in the corpus's merged program that another name
reaches through a further qualifier (`json/text/append` beside `text/append`):

    module                 declarations   reached twice
    bench/compile_corpus   428            99
    std/json               196            17

Ninety-nine of 428. Collapsing them
would mean making two spellings of one declaration into one name, and a
qualified spelling is permanent identity in this compiler today. It is
sound in principle — the lock read at the module root makes an import path
resolve to one module for the whole build — but it changes what a qualified name
means, so it is its own piece of work rather than a tidy on this one.

Declined, reverted, nothing shipped.

## What the per-module reshape costs the diagnostics, fixture by fixture

The reshape is `src/lib.rs`'s `outermost` branch: a dependency runs
`check::check_own_declarations` — the three checks that skip any name carrying a
slash — plus `canonicalize_bare_aliases`, `finish_program` and `trmc::rewrite`,
and hands its declarations up. Everything else runs once, at the outermost
compile. Against the 193 fixtures in `tests/golden/errors`, run as libraries
behind a generated entry the way `tests/golden.rs` drives them:

    155 agree   38 move   0 without a golden

Thirty-one of the thirty-eight carry an `.imported.stderr` golden, which the
corpus already keeps for fixtures whose names spell qualified through an import.
The seven with a plain golden are the ones worth reading, and they are two
kinds.

Three are an improvement. `a_wall_whose_right_side_is_a_name`,
`fields_that_no_one_record_declares` and `sequencing_takes_two_descriptions`
traded the `(module X)` suffix for a `--> X.kso:line:col`, which is the
attribution work doing what it was written to do.

Four print a qualified name where the old output printed a short one:

    fixture                          was                 is now
    constructor_in_a_list            `point`             `constructor_in_a_list/point`
    an_arm_set_with_no_settling_arm  `open_start?`       `an_arm_set_with_no_settling_arm/open_start?`
    field_of_the_wrong_record        `point`             `field_of_the_wrong_record/point`
    field_of_an_annotated_parameter  `money`             `field_of_an_annotated_parameter/money`

Measured both ways on the same fixtures rather than inferred: the base binary
prints the short name for all four when they run as libraries, so the reshape
introduces this and does not inherit it.

The mechanism is the reshape itself. Today a module's own `check_merged` runs
inside its own compile, before its declarations are qualified for an importer,
so a message that names a declaration reads the spelling the source wrote. Move
that check to the outermost compile and the same message sees the name after
qualification.

Where a fix goes. Every diagnostic now carries the file it belongs to, and a
module's qualifier is that module's short name, so one rule at render time
covers it: print a name without the qualifier that names the diagnostic's own
module, since that is the spelling the file being pointed at uses. One site,
one rule.

It does not cover all four. `an_arm_set_with_no_settling_arm` attributes to
`run_an_arm_set_with_no_settling_arm.kso`, the generated entry, because a
dispatch tie is reported at a call rather than at the arms it is about. The
qualifier there is not the diagnostic's own module and the rule would leave it
alone. That is a second defect, in where a tie points, and it wants its own
fixture.

That rule was then written and measured. `diag::spelled_in` runs at the one
place a message is rendered: it derives the qualifier from the path the
diagnostic points at — the file's stem and its parent directory's name, which
covers a module that is a directory and a module that is one file — and removes
that qualifier inside backticks only. A message quotes a name in them and a
single span can hold more than the bare name (`(mod/point …)` and `&mod/point`
are both how a message spells the fix), so the span is what it works on; prose
outside the backticks is left alone.

Three of the four then read exactly as the base spells them, spelling for
spelling:

    error[name]: `point` has no field `name`
      --> field_of_the_wrong_record.kso:21:12

The fourth is unchanged, as predicted. `an_arm_set_with_no_settling_arm` still
says `an_arm_set_with_no_settling_arm/open_start?` because the diagnostic points
at the generated entry, so the qualifier it carries is not the one this rule
derives. Where a dispatch tie points is the second defect, and it stays open.

The whole corpus was re-run with the rule in: 155 agree, 38 move, the same
numbers as without it. It repairs three messages and moves nothing else.

Then the fourth. The tie's span was already an arm's — `b.span`, the second of
the pair — so the file it named was wrong rather than the line. The walk reaches
`program.fns` by index, through a group table built earlier, and the attribution
the walks carry rides on `diag::attributing`, an iterator. An indexed walk never
touches it, so the diagnostic reads whatever attribution was last set and the
render falls back to the file it was handed. Taking `b`'s file explicitly at the
push site fixes it, and all four fixtures then read as the base spells them:

    error[dispatch]: these `open_start?` arms tie: each is the more specific one
    somewhere, and a call could match both — write the arm that is most specific
    in every position
      --> an_arm_set_with_no_settling_arm.kso:7:4

That is a class, not one site, and the class has two halves.

The near half is walks that reach declarations by index rather than through the
iterator: `check_constants` reads `arms[1].span` out of a slice it indexed,
`check_overlapping_arms` walks a filtered `Vec<&FnDecl>`, and
`check_overload_ranks` walks `windows(2)`. None goes through `diag::attributing`,
so none carries a file. The corpus surfaced only the tie because only the tie has
a fixture that crosses files; the other three want fixtures before fixes.

The far half is larger and was found by chasing the one remaining leak.
`sub_of_none` comes from `check_sub_parents`, which walks `program.types` with a
plain `for` — the shape `attributing` was written for. It cannot use it.
`stamp_file` stamps `program.fns` and nothing else, `TypeDecl` has no `file`
field, and `HasFile` is implemented for `FnDecl` alone. So no diagnostic about a
type declaration can carry a file however it is walked, and the attribution
covers half the declarations in a program.

Closing that is a real change rather than a call-site repair: a field on
`TypeDecl`, a second loop in `stamp_file`, an impl, and one `Arc` refcount bump
per type declaration — the same cost the fns side already pays, and the same
shape kanso#1324 measured when it made a module's path shared. It is the next
step on this thread.

Then the goldens, which is where the reshape actually stands or falls. All 38
movers were regenerated with the harness's own staging — `pub play` files behind
a generated entry, everything else run in place, which is what
`run_kanso_as_library` does — and each candidate compared to its golden with the
`(module X)` suffix and the `-->` block removed, so the comparison is of message
TEXT alone. Three classes came out:

    30  message text identical: only the location changed
     6  one diagnostic became two
     2  the message text itself changed

The thirty are the reshape's improvement, in bulk. The six wanted reading, and
reading them changed the count above: they are the same diagnostic twice, at the
same file, line and column.

    error[name]: no record type has a field `name`
      --> field_missing.kso:6:12
    error[name]: no record type has a field `name`
      --> field_missing.kso:6:12

Two reports of one source location is a declaration present twice in the merged
program, which is the finding the module memo turned up above — 99 of the
corpus's 428 merged declarations are a second copy reached through a further
qualifier. The reshape did not create those copies; it moved the check that
walks them from per module, where each saw one copy, to once at the end, where
one walk sees both. Under the old suffix the two reports read as different
messages, so nothing noticed.

That makes the duplicate declarations a blocker for the reshape rather than a
performance question beside it, and it is where this thread now goes.

The two are the blockers, and they are different from each other.

`sub_of_none` still leaks: `sub_of_none/missing cannot derive from none yet`,
pointing at `run_sub_of_none.kso:1:6`, the import line in the generated entry.
That is the same class as the dispatch tie — a check that raises without a file,
so `spelled_in` derives the qualifier from the entry and leaves the name alone.
It is the fourth member of the class this entry already names, and it has a
fixture, which the other three do not.

`builtin_arg_type` looked like the serious one. Its own error disappeared and a
different one took its place:

    was  error[type]: `length` takes a list, a map, or a string here, not an int
    then error[name]: `builtin_arg_type/play` is internal to the standard
         library — import its module

Diagnosed, and the first reading was wrong: this belongs to the entry below
rather than to the reshape. `resolve_name` stripped `builtin_` from a qualified
name, so a fixture named `builtin_arg_type` had every reference to it refused.
That bug predates the reshape by as long as the refusal has existed; the fixture
survived only because its own error used to be raised first and stop the compile.
With the prefix check restricted to bare names, the fixture reports its own
error again, and better than before:

    error[type]: `length` takes a list, a map, or a string here, not an int
      --> builtin_arg_type.kso:1:27

`sub_of_none` was the other, and it is fixed: `check_sub_parents` walks
`program.types`, `stamp_file` stamped only `program.fns`, and giving `TypeDecl`
a file — a field, a second loop in `stamp_file`, an `impl HasFile`, and the walk
through `diag::attributing` — makes it read `missing` at
`sub_of_none.kso:1:6`. With that in, every one of the thirty-eight is either
location-only (32) or the doubled report below (6). No message text is worse
than it was.

Nothing of the reshape shipped. The rules are dead code on main — no diagnostic
carries a file until the attribution patch lands — so they belong to its bundle
rather than to changes of their own. Five patches held; the candidates are
written out beside them. What remains before it lands is `sub_of_none`, the
thirty-seven goldens, and the compile veins measured for the whole bundle.

## A module named for what it holds could be imported and never used

`builtin_` names are how the standard library reaches the engine, and a program
that writes one for itself is refused. `resolve_name` did that by stripping the
prefix and asking whether what remained was a builtin. It asked the same
question of a QUALIFIED name: `builtin_shapes/circle` became `shapes/circle`,
which is not a builtin anyone has, so the reference was refused as internal to
the standard library. Every use of a module whose own name began with those
bytes met the same refusal, so such a module could be imported and never used.

Reproduced on an unpatched tree, two files:

    builtin_probe/builtin_probe.kso   pub hello = "hi"
    main.kso                          import "./builtin_probe"

                                      print builtin_probe/hello

    error[name]: `builtin_probe/hello` is internal to the standard library
    — import its module

The check applies to bare names now. A qualified name is a declaration in
another module, and that module's name is its own business.

The fixture is `tests/golden/micro/builtin_prefixed_names_are_not_builtins.kso`,
which the micro corpus runs twice — once directly and once as a library behind a
generated entry, which is the path that reaches the refusal. It was watched red
for the right reason: the library run produced empty stdout because the compile
was refused. The whole golden suite is green with the fix, including
`a_builtin_the_standard_library_keeps_to_itself`, the fixture that pins the bare
case this refusal exists for.

How it was found is worth recording, because the first reading was wrong. It
turned up while regenerating the per-module reshape's error goldens, where
`builtin_arg_type` had lost its own type error and gained this one instead — a
diagnostic disappearing, which the entry above called a stop. The reshape had
not lost anything. The fixture is named `builtin_arg_type`, its own error used
to be raised first and stop the compile, and moving that check later let this
refusal reach the reader ahead of it. The bug was already there and had been
since the refusal was written.

## 2026-09-08 — a projection off one box carries no sign

The `builtin_` fix went to CI three times and the compile-instructions row said
something different each time. Worth writing down, because the reasoning that
produced the wrong prediction was not careless.

Round one asked for the slash in front of the `BUILTINS` lookup:

    name.strip_prefix("builtin_").filter(|_| !name.contains('/'))

CI read 50,691,635 against a golden of 50,685,288 — a rise of **6,347**. That
one is real work, and the diagnosis was easy: every builtin reference the
standard library makes now scans its own name for a slash, and the library makes
a great many.

Round two moved the question onto the refusing arm, which a correct program
never reaches. The container this was written on read 51,095,253 for that shape
and 51,095,253 for the round-one shape — identical, to the instruction. Two
source shapes that measure the same locally, with the work provably removed from
the hot path, and the container's own delta against its baseline was a rise of
**629**. So the residue was called irreducible layout and the PR body said the
fix costs 629 instructions.

CI read 50,684,921. A **fall of 367**.

The magnitude was about right and the sign was backwards. Layout is a property of the toolchain and
the host — this container is rustc 1.94.1 on glibc 2.39, the goldens are measured
on 1.98.1 — and a layout residue measured on one box says nothing about the same
residue on another. The correct reading of the local measurement was that the
6,347 was gone and the remainder was unpredictable; instead it was read as a
number.

`bench/compile_instructions_golden.txt` is regenerated to 50,684,921 and the two
`data-golden="compile.compile_instructions"` spans on the compiler page follow
it. The rule already in CLAUDE.md — project a compile-instructions move from CI
or take the red round, never write down that it moved before CI has said so —
now has this as its worked example, and the sign is the part it costs a round to
learn.

## 2026-09-08 — the whole-program check ran over declarations the next pass deletes

`enroll_bare` gives every exported declaration of every imported module a twin
under its short name, so `import "std/list"` puts both `list/next` and `next`
into the merged program. `canonicalize_bare_aliases` then takes most of those
twins straight back out: where a bare name has exactly one qualified target and
is never locally bound, it rewrites the references to the qualified spelling and
drops the clone.

On `bench/compile_corpus` that pass declines nothing. All 83 twins go, out of
394 declarations — 21% of the merged program. And it ran at src/lib.rs:3599,
seventeen lines after `check_merged` at 3581. So the whole-program check, and
every pass reading its results, ran over 83 declarations that were about to be
deleted.

Moving `canonicalize_types` and `canonicalize_bare_aliases` in front of the
check, on CI:

    compile_instructions   50,684,921 -> 48,757,859   -1,927,062  (-3.80%)
    compile_allocs              29,941 ->     29,606        -335  (-1.12%)
    compile_peak_bytes         819,217 ->    773,818     -45,399  (-5.54%)
    front_end_visits            23,723 ->     22,426      -1,297  (-5.47%)

welfare 60.04 -> 60.21.

The container measured the instruction row at 51,095,251 -> 49,162,592, a fall
of 1,932,659, from the gate's own valgrind recipe minus its host-comparability
check — this box cannot be compared against CI's golden, but it can be compared
against itself across two builds, which is what an A/B needs. The two hosts
disagree by 5,597 on a delta of nearly two million. On a work fall of this size
the sign and the magnitude both carry across hosts; on the 629 the entry above
records, neither did, and the difference is that this one is not layout. The
allocation row agrees to the unit, 29,606 on both, because allocations count the
compiler's own algorithm.

The census that found this was looking for something else. Task #427 recorded
"99 of 428 merged declarations are a second copy reached through a further
qualifier" — the diamond's duplicates, a module reached by two import paths
contributing its declarations twice. On the compile corpus there are none of
those: `collapse_diamonds` already drops them, and every one of the 83 pairs the
census turned up is a bare twin beside its qualified original. The number was
right and the reading of it was wrong.

Nothing else changes. The alias pass removes a twin only where the bare name has
one target and no local binding, so an ambiguous bare name keeps both copies and
`check_bare_ambiguity` still sees them. The full golden suite is green,
including all 173 error fixtures and the micro corpus run twice.

## 2026-09-08 (third) — three page gates in three CI jobs, and no sweep over them

Searched the log, the archive and design/ before filing. `golden_prose` appears
once in the live log (kanso#1300's round three, where it caught a span) and
seventeen times in the archive; the closest entry is 2026-09-02, "the sweep the
page owed after four twins", which is about prose figures carrying no
`data-golden` tag at all. Neither asks whether the page gates have an entry
point.

**DONE.** `scripts/gates/all_pages.sh` runs the three gates that read the
published pages, and `tests/every_page_gate_is_in_the_sweep.rs` pins the list to
the tree.

The three are `golden_prose` (the `data-golden` spans against the goldens they
name), `page_drift` (docs/compiler.html against the log's budget of unpublished
entries) and `prose_check` (the three mechanically detectable slop families over
all 29 pages). They run in three different CI jobs — welfare, docs, and its own —
so a session editing a page has three commands to remember and none to run.

kanso#1328 is what that costs. Round two left the page's `front_end_visits` span
at 23,723 where its golden had moved to 22,426. page_drift and prose_check both
pass on that tree, because neither reads a `data-golden` span, and those are the
two I ran before pushing. golden_prose turned its own job red and welfare with
it: welfare runs golden_prose as its last step, so it reported ALREADY RED and
could prove nothing about the three rows sharing that gate.

The sweep reads all three and reports all three. Stopping at the first objection
would rebuild the failure it exists to prevent, which is the rule
`all_counters.sh` already carries for the same reason. It costs 31 seconds, 28 of
them prose_check reading 29 pages through a regexp engine written in kanso; the
figure is in the script's header so a reader waiting on it knows it has not hung.

The spec is the half that does not go stale. Eight programs under scripts/ hold a
literal `docs` path: three are the page gates and five read one for something
that is not the prose, so each of the eight has to appear as a row or in an
`elsewhere` list carrying a reason. `book_panels` and `book_quotes` are excused
to `scripts/book_check.sh`, which already runs both before it replays the sample
outputs, and the spec opens that file to check the claim rather than believing
the comment. A fourth assertion pins the property the round actually turned on:
something the sweep runs has to read a `data-golden` span, whatever it ends up
being called.

All four were watched red. Dropping golden_prose from the table fires two of
them and the message names golden_prose. Claiming page_drift takes `--write`
fires the third. Pointing book_panels' excuse at `scripts/build_wasm.sh` fires
the fourth — and the first attempt at that mutation reported GREEN, because my
`sed` anchor missed the row: it is the first line of the table and carries the
`elsewhere="` prefix. I came within one command of recording a spec that cannot
fail. A mutation is evidence only once the file has actually changed, and the
check for that is to read the line back.

Nothing in ci.yml moves. The three gates already run there, and the spec rides in
the specs job with every other `cargo test`.

## 2026-09-08 (fourth) — the entry path's reorder costs a diagnostic, and is reverted

Searched the log, the archive and design/ before filing: the 2026-09-08 entry
above records the same reorder on the MODULE path (kanso#1328), and kanso#1120
records the rule this breaks — a module is named the way an import writes it.
Neither anticipates the interaction.

**REVERTED.** `compile_parsed_entry` has the same ordering kanso#1328 fixed on
the module path, and moving its alias pass in front of the whole-program check
is worth 1,674,396 instructions. It also renames a diagnostic, so it does not
ship in that shape.

The measurement first, because it is real and the idea is worth returning to.
`enroll_bare` is called at src/lib.rs:2477, inside `load_dependencies`, so
`dep_program` carries a synthetic twin for every exported declaration of every
import — and both paths merge that program. On the entry path the twins landed
in `merged`, `check_merged` walked them, and `canonicalize_bare_aliases` deleted
them four lines later. Hoisting the two canonicalize calls out of the success
arm reads, in the box:

    entry_instructions   165,184,791 -> 163,510,395   -1,674,396  (-1.0136%)

and CI counted 162,218,854 against the container on the same tree, the two boxes
0.79% apart where the module row sits 0.81% apart on that pair.

**What it costs.** `scripts/module_differential` went from 0 wrong to 2:

    a call from the entry at the wrong arity
      refused, but not with 'error[arity]: no 2-argument arm of `one` (arms take 1)':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

The alias pass rewrites a bare reference to its qualified spelling, so once it
runs before the check, the arity refusal quotes `m/one` where the program says
`one`. That is a diagnostic naming a spelling the user did not write, which
kanso#1120 settled the other way, and it is a worse trade than a per-cent of
compile work is worth. Reverting the reorder alone takes the differential back
to 0 wrong, which is what says the reorder is the whole cause.

The module path does not have this problem because a dependency's own call
sites are already qualified by the time they are merged; the entry's are the
ones the user wrote. That asymmetry is why one of the two reorders shipped.

**A GREEN SUITE SAID NOTHING ABOUT IT, and the reason is worth having.** Before
pushing I ran the error corpus, both micro corpora, the .mem vein and
tests/reexports.rs, all green, and reported that as the correctness evidence.
`scripts/module_differential` is a kanso program run by the diagnostics-
differential CI job; `cargo test` does not run it, so no amount of the suite
would have found this. It is the same shape as the page gates — a check that
lives outside the harness a session reaches for by habit. The nine differential
sweeps each have this property.

What would make the reorder shippable is deciding what the arity refusal should
quote when the pass that rewrote the name has already run: either the check
reads the pre-canonical spelling, or the pass records what it rewrote. That is a
design question about diagnostics rather than about ordering, and it is where
this thread now sits.

## 2026-09-08 (fifth) — the entry path compiles and nothing counted it

Searched the log, the archive and design/ before filing: `compile_parsed_entry`
appears in the 2026-09-08 entries for kanso#1324, #1325 and #1326 and in
kanso#1329's revert, and every one of them measures it on a corpus that lived in
a session's temporary directory. None of them asks why there is no vein.

**The two compiles.** `kanso check <directory>` is a module and goes through
`compile_module_inner`. `kanso check <file>` with a top-level expression is an
entry and goes through `compile_parsed_entry`, which merges the imports itself
and runs its own whole-program check at src/lib.rs:160. Proved by probe, not by
reading: an `eprintln!` at the entry site fires once for
`bench/entry_corpus/main.kso` and not at all for `kanso check
bench/compile_corpus`.

Every compile gate in the tree checks a directory. So the call at line 160 was
watched by nothing, and `KANSO_PHASES` cannot separate the two — `load_dependencies`
compiles each import through the module path, so a phase report over an entry is
the union of both.

**What that cost.** kanso#1326 projected a RISE of 629 from this container and CI
read a FALL of 367. kanso#1324 and #1325 landed on numbers no CI job could
reproduce. Two further findings — the synthetic-twin skip below and kanso#1329's
reverted reorder — were measured and could not be landed against anything.

**The vein.** `bench/entry_corpus/main.kso` names ten imports and uses each,
following bench/compile_corpus's rule that a workload is named rather than
inherited; it imports ten where the compile corpus imports four, because the
entry path's own work is the merge and the check over everything the imports
bring and a corpus with one small import measures mostly the module path
underneath it. `scripts/gates/entry_instructions.sh` counts it the way
`compile_instructions.sh` counts its own — same box, same emptied environment,
same pinned tunables, same `kanso::main` anchor — and every reason for those is
left in the original rather than restated.

`bench/entry_instructions_golden.txt` opened holding zero, because the row is
CI's and this container reads high against CI's rustc. Round one was red on
purpose and CI answered **163,886,731**, on binary sha 3c53d0acdbcb — the same
sha that counted `compile_instructions=48,757,859` in the same job, so both
rows answer for one build. The container had projected 165,183,406 from the
same recipe minus the host check: 1,296,675 high, or +0.79%, which is the
offset already recorded between rustc 1.94.1 here and CI's 1.98.1. The
projection was right about the size and could not have been recorded as a row.

**And the trend gate did not walk the new golden.** Found by asking which
files in bench/ `scripts/trend_gate/trend_gate.kso` names, which its own
comments say is the only method that has ever found one of these. The list has
been short five times: three cost goldens nobody entered, then readbench —
whose golden the gate could not see while two of its rows were welfare terms —
then livebench, then the consolidated run program. This would have been the
sixth, in the very PR that exists because a compile the gates could not see
went unpriced.

Two files in bench/ are unwalked and one of them belongs that way:
`bench/compile_libraries_golden.txt` holds five sonames rather than counters
and its own gate diffs it byte for byte. So the excuse list is one line long,
and `tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads bench/
off disk, asks the gate which files it names, and fails on anything neither
walked nor excused. Its three assertions were each watched red: dropping the
entry golden from the gate names it in the failure; an excuse for a file that
is not there fires the second; an excuse for a golden the gate already walks
fires the third. Finding this by hand a sixth time was not a plan.

**The ratchet row separates the two veins, measured.** The mutation asks the
entry's whole-program check twice. In the box:

    entry_instructions   165,183,406 -> 190,382,616   +25,199,210  (+15.25%)
    compile_instructions  49,162,592 ->  49,162,592   byte-identical

A vein whose defects another vein already catches would not be worth its
callgrind run. This one is worth it: the compile row cannot move for a defect on
this path however much work it does.

**MEASURED, NOT SHIPPED — the synthetic-twin skip.** `enroll_bare` gives every
exported declaration of an imported module a twin under its short name, cloning
the whole declaration, body and all. On the entry corpus that is 145 of 882
declarations and 155 of 1,035 statements, and the whole-program check walks both
copies. It is visible as a defect only under kanso#1329's reshape, where six
error fixtures reported one diagnostic twice at the same line AND the same
column — `field_missing/play` and its twin `play`, both at span 4 of the same
file, each answering `check_field_exists` once. The module path has not had this
since kanso#1328: `canonicalize_bare_aliases` takes the twins out before the
check there, which is why `compile_instructions` reads synthetic=0 and this whole
thread is invisible to it.

Skipping synthetic declarations in the three checks that are pure body walks —
`check_build_blocks`, `check_none_in_collections`, `check_field_exists` — was
built and measured in the box:

    entry_instructions   165,183,406 -> 164,922,557   -260,849  (-0.1579%)
    compile_instructions  49,162,592 ->  49,170,337     +7,745  (+0.0158%)

The entry row falls 34 times what the compile row rises, and the compile row's
rise is the branch itself plus layout — the module path has no twins to skip.
**Welfare reads the compile row and not the entry row**, so by the objective as
it stands today this change is a small loss. That is a question about the
objective's inputs rather than about the change, and it is not settled here: the
skip is left out of this PR, and what lands is the vein it would be measured
against. Recorded as OPEN.

The remaining 0.85% of kanso#1329's reverted reorder is inference, which is 22.6%
of the entry compile against `check_merged`'s 38.3%. `infer` indexes declarations
positionally — `vec![0; program.fns.len()]`, groups by index — so it does not take
a `continue`, and skipping the twins there is a different change from this one.

- **DONE** — the entry vein: corpus, gate, golden with CI's row, CI step and
  summary row, ratchet row, sweep membership, the trend gate's own list and
  the spec that keeps it honest. The derivations in
  `tests/the_compile_sweep_names_every_compile_gate.rs` walked past
  `bench/entry_*` and now do not, and `the_compile_row_holds_one_value` covers
  both instruction goldens rather than one, because the one-row-one-value ruling
  is about the shape of a row and not about a filename.
- **OPEN** — the synthetic-twin skip, measured above, held on the objective
  question: should welfare's compile term read the entry path as well as the
  module path? Two compiles, one term.
- **OPEN** — the twins inside `infer`, worth most of the remaining 0.85%.

## 2026-09-08 — the corpus change banked a six-point fall, and the precedent says it should not have

Clay, reading the published chart on kanso-lang.dev/numbers: why didn't the new
corpus ruling go retroactive, so there is no drop? The answer is that kanso#1321
left the three compile baselines where they were, and the score fell 6.29 points
for a change that touched no compiler code.

    2026-09-08 01:53   welfare 66.0241 -> 59.7360      #1321
    compile_instructions   19,316,962 -> 52,603,220
    compile_allocs             11,613 ->     31,596
    compile_peak_bytes        375,222 ->    789,740

**The reason #1321 gives.** "The baselines those three are divided by were taken
on lib/json and are left where they are: the index has an arbitrary origin and
only its direction and the size of its moves mean anything, so re-deriving a
historical compiler's cost on a corpus that did not exist then would buy
nothing. This is a change of origin, not a regression."

**The origin is arbitrary; the move is not.** That sentence is the argument
against leaving it. A reader of the chart sees a six-point fall, and a fall is
what the index says a change made worse. CLAUDE.md is explicit: "Moving the
floor to accommodate a change while leaving the weights alone is declaring the
objective wrong without saying so."

**Re-deriving history was never what the precedent asks for.** It asks for one
measured factor per row, taken on the same head under both definitions -- which
#1321 already measured and recorded. Scaling each baseline by its own factor
restores each ratio exactly:

    row                     factor   ratio left as-is   ratio re-based   before
    compile_instructions    2.7232         1.0753           2.9282       2.9282
    compile_allocs          2.7207         1.9658           5.3483       5.3483
    compile_peak_bytes      2.1047         1.0373           2.1833       2.1833

    baselines: 56,563,967 -> 154,032,855 · 62,110 -> 168,985 · 819,217 -> 1,724,228

Every ratio returns to the digit it held before the corpus moved, so the score
does not move and the chart is flat across the change.

**Three precedents, all in this repository.** The archive: "RE-BASELINED THE
SAME WAY #729 WAS: `basket_allocs` scaled by exactly the factor", and
"RE-BASELINED SO IT BANKS NOTHING, the same method as #729 and #741." The floor
file's own history at 73.53: "the compile row counts the compiler's own frame;
the baseline is re-based by the same 465,864." And at 84.51, scanbench entering
the corpus: "The score does not move on entry, by design."

**What is not in dispute.** #1321 is right that a term measured on a library
moves whenever that library changes its imports, and the fixed corpus is the
ruling of 2026-09-08. Nothing here argues against the corpus. The question is
only whether the change of measurement banks a fall, and the answer this
project has given three times is that it does not.

**OPEN, and cloud's**, since the baselines and the floor are code. Two readings
are available and both were taken on the changeover head, so no re-measurement
is needed. If the fall is kept deliberately, that is a claim about the weights,
which CLAUDE.md says is settled before the floor moves rather than after.

## 2026-09-08 (second) — the compile term read one compile out of two

Searched the log, the archive and design/ before filing: the re-basing precedent
is the 2026-09-05 entry for kanso#1242 and the archive's #729 and #741; the entry
vein opened in kanso#1330, whose own entry above closes with this as an OPEN
question — "should welfare's compile term read the entry path as well as the
module path? Two compiles, one term." This answers it.

**The term summed one path.** `kanso check <directory>` takes
`compile_module_inner`; `kanso check <file>` with a top-level expression takes
`compile_parsed_entry`, which merges the imports itself and runs its own
whole-program check. Every `kanso run` takes the second, and nothing counted it
until kanso#1330. The compile term now adds the two rows:

    compile_instructions   48,757,859 + 163,886,731 = 212,644,590

**The baseline moves with it, so the score does not.** The entry vein has no
reading at this objective's epoch, because its corpus did not exist then, so its
baseline is imputed at the ratio the module row holds:

    r = 154,032,855 / 48,757,859 = 3.1591390221
    entry baseline    163,886,731 * r = 517,740,967
    summed baseline   154,032,855 + 517,740,967 = 671,773,822
    summed ratio      671,773,822 / 212,644,590 = 3.1591390216

The preservation is algebraic — `(cb + ec*r) / (cc + ec) = r` for any `ec` — and
it was measured rather than trusted: welfare reads 66.29 against a floor of 66.29
before and after.

**What the shape of the change turned out to be.** The plan recorded for this
work said it was one line in `bench/objective_sources.txt`, and that was wrong.
welfare does not build its counters from that file; it reads the goldens itself,
in `fn measured` and a reader chain, and objective_sources is the LINK that the
trend gate's `shifted?` and `tests/the_objective_reads_what_the_gate_watches.rs`
replay. Both halves are needed and they are different files. The spec was watched
red before it passed: with the second key removed it reports `compile_instructions
reads 212644590 from welfare and 48757859 from compile_instructions`.

Two smaller things the edit forced. The compile veins now reach `measured` as one
list rather than as four positional arguments, because a fourth `compile[4]!` at
the call site overflows the 80-column rule by three characters; the next golden
to join is now one list entry and one binding. And the binding is `ent`, because
`entry` is bare-enrolled from an import and the resolver refuses to shadow it.

**golden_prose needed the same golden and would not have said so.** Its
`golden_for` answers for `decode`, `encode` and `compile` and returns `[]` for
any other family, so a page span written `data-golden="entry.entry_instructions"`
resolves against an empty golden — a span nothing watches, which is the exact
failure that gate was widened to fix in kanso#1047. The entry row joins the
`compile` family instead, whose own comment already licenses it: the key names do
not collide.

**Still to ship: the twin skip.** kanso#1330 measured it and deliberately left it
out, because the objective could see the module row's +7,745 and not the entry
row's −260,849. Under the sum it is a fall of 253,104, −0.1190%. It is a separate
change because it moves two goldens whose values are CI's, and this one moves no
counter at all.

## 2026-09-08 (third) — the twin skip ships, now that the objective can see it

Searched the log, the archive and design/ before filing: this thread is the OPEN
item at the end of the kanso#1330 entry above ("the synthetic-twin skip, measured
above, held on the objective question"), and the entry above that, for kanso#1331,
answered the question it was held on. The archive's prior art is `enroll_bare`
and `canonicalize_bare_aliases` in the kanso#1328 entry. Nothing else is new.

`enroll_bare` clones every exported declaration of an imported module under its
short name — body and all, `synthetic = true`. On `bench/entry_corpus` that is
145 of 882 declarations and 155 of 1,035 statements. Three checks in
`check_merged` are pure body walks and were the only three of sixteen that read
those clones: `check_build_blocks`, `check_none_in_collections` and
`check_field_exists`. Thirteen others already skip them. These three now do too.

CI's rows, which are the ones this vein may hold:

    entry_instructions   163,886,731 -> 163,612,976   -273,755  (-0.1671%)
    compile_instructions  48,757,859 ->  48,761,165     +3,306  (+0.0068%)

Under kanso#1331's summed compile term that is

    212,644,590 -> 212,374,141   -270,449  (-0.1272%)

and welfare moves 66.2874470488728 -> 66.28984813328917, banked in this PR.

**compile_instructions RISES to 48,761,165 and that is the change's own doing.**
The module path has had no twins to skip since kanso#1328 put
`canonicalize_bare_aliases` in front of the check there, so the walk does the
same work and now pays for a test that can never say yes; src/check.rs is the
compiler, so its bytes and the layout under them move with the edit as well.
The row is traded against the entry row's fall, which is 83 times it.

**The container projected the deltas and got the digits wrong in both
directions**, which is the kanso#1326 lesson again. It read 165,183,406 ->
164,922,557 for the entry row (-260,849) against CI's -273,755, and +7,745 for
the module row against CI's +3,306. Sign and order of magnitude carried across
rustc 1.94.1 here and 1.98.1 there; nothing finer did. Both rows are copied out
of the job log.

The compile row rises because the module path has no twins left to skip — since
kanso#1328 `canonicalize_bare_aliases` deletes them before the check there — so
what that row records is the branch itself plus layout. Under kanso#1331's summed
term the entry row's fall is 34 times it, and the trade lands the right way up.

A twin's body IS the original's body under a second name, and both copies are in
the same merged program, so a twin can answer nothing the original answers
differently. That is why no diagnostic moves. The doubling was visible once, under
kanso#1329's reverted reorder: six error fixtures reported one diagnostic twice at
the same line and the same column, `field_missing/play` and its twin `play`. On
current main the dependency's own compile refuses first, so the second copy never
reaches a reader — which is why this change has counters and no fixture.

`infer` is deliberately not given the skip. It indexes declarations positionally
(`vec![0; program.fns.len()]`, groups by index), so a `continue` misaligns it.
That is the rest of kanso#1329's reverted reorder and stays open.

## 2026-09-08 (fourth) — a compile left its IR behind, and the guard for it read the litter

Searched the log, the archive and design/ before filing: the archive names
`cached_program_binary` once, in the entry that introduced the per-pid IR path
after concurrent builds segfaulted inside clang. Nothing records the leak, and
nothing else in the tree measures what a `kanso run` leaves in the temp
directory.

`kanso run` caches its binary under a hash of the IR and `runtime.c`. On a miss
it writes the IR to `kanso_run_<key>_<pid>.ll`, hands that to clang, and renames
the staging binary into place. The `.ll` was never removed. One cold run in an
isolated TMPDIR leaves four files and exactly one of them is a leak:

    kanso_run_<key>                     185 KB   the binary cache, intentional
    kanso_run_<key>_<pid>.ll             42 KB   THE LEAK, one per cache MISS
    kanso_runtime_dev_<hash>.c          415 KB   content-keyed, shared, bounded
    kanso_runtime_dev_<hash>.o          258 KB   likewise

At 42 KB a miss this reached about 112,000 files in one long-lived container,
which is the whole of a session's disk allowance, and it was the true cause of
four "spec failures" chased as real during kanso#1330. The staging path needs no
removal because `rename` consumes it.

**The obvious fix blinds a real guard, which is why this took two attempts.**
`tests/concurrent_build.rs::two_builds_of_one_program_do_not_share_a_file`
proved that two concurrent builds are handed different paths BY FINDING THE
LEFTOVER `.ll` AND READING A PID OUT OF ITS NAME. Delete the file and the guard
has nothing to look at; the first draft of this change shipped the removal, the
guard went green on an empty set, and the whole thing was backed out. A guard
resting on a bug fails the moment the bug is fixed.

So the guard now watches during the build rather than counting what is left. The
IR is written before clang starts and removed after it returns, and a cold run of
that fixture is ~133 ms with ~100 ms of that window, against a one-millisecond
poll. It cannot pass vacuously: seeing no IR file at all is a failure with its
own sentence, because "the race never happened" and "the race happened and was
safe" must not look alike.

**Why the sibling is not enough on its own**, measured rather than assumed. With
the pid stripped back out of the path, this guard caught the defect in 10
sittings of 10 and again in 5 of 5 after the rebase, where
`many_builds_of_one_program_all_answer` caught it in 9 of 10 — it passed once
with the bug in place, because whether two processes actually overlap on the
file is the race, and the race is not owed to anyone. One of the two is a
corruption that may or may not happen; this one is the decision that allows it.

No counter moves. `kanso run`'s temp handling is not on any measured path: the
compile veins run `kanso check`, which never reaches `cached_program_binary`.

## 2026-09-08 (fifth) — the entry reorder re-derived, and the alias pass refuses a valid program

Searched the log, the archive and design/ before filing: the 2026-09-08 entry
"the entry path's reorder costs a diagnostic, and is reverted" is this same
change, and the entry after it records the vein that now measures it. The
archive's `canonicalize_bare_aliases` entries are about what the pass costs, not
about when it runs. What neither has is the failure set as it stands today.

**DECLINED, a second time, on wider grounds than the first.** Hoisting
`canonicalize_types` and `canonicalize_bare_aliases` out of
`compile_parsed_entry`'s success arm is worth, on 9ec9f7e9 in this box:

    entry_instructions   164,922,557 -> 163,362,291   -1,560,266  (-0.9461%)
    compile_instructions  49,170,337 ->  49,170,337            0

against kanso#1329's -1,674,396 (-1.0136%) for the same edit a few merges
earlier. The module row is byte-identical here, which is what a
`compile_parsed_entry`-only edit should read; CI has not been asked, so that is
a projection.

**The failure set has moved since the revert.** `scripts/module_differential`
reads 0 wrong on 9ec9f7e9 and 2 wrong with the reorder, and only one of the two
is the one kanso#1329 recorded:

    a call from the entry at the wrong arity
      error[arity]: no 2-argument arm of `m/one` (arms take 1)
      where the program says `one`

    a type and a function sharing a name
      expected it to compile; got
      error[opacity]: `m/thing` is foreign -- only `m` builds a `thing`

The sibling-arity case kanso#1329 also lost now passes. In its place is a
program that compiled before the reorder and does not compile after it, which is
a worse thing than a diagnostic quoting the wrong spelling. `m/b.kso` declares
`pub fn thing _`, the entry writes `print "{thing 0}"`, and the answer should be
`m/thing 0`.

**Both objections are the alias pass, not the type pass.** Moving
`canonicalize_bare_aliases` alone and leaving `canonicalize_types` in the success
arm leaves the differential at the same 2 wrong, and reads slightly BETTER than
moving both:

    entry_instructions   164,922,557 -> 163,353,361   -1,569,196  (-0.9515%)

8,930 better than the pair, because the alias pass deletes the twins before
`canonicalize_types` walks them rather than after. `thing` in `print "{thing 0}"` is a CALL, so the alias pass rewrites it to
`m/thing` like any other bare name, and the opacity check then reads a qualified
name as a foreign type construction. One pass, one mechanism, two checks that
read a name after something else has rewritten it.

**What would make it shippable**, stated more narrowly than kanso#1329 could:
`check_merged`'s opacity and arity checks have to see the spelling the program
used. The obvious inverse map from `aliases` is unsound -- it is keyed by name,
so it would also rewrite the diagnostic for a call the user really did write
qualified. The per-site record was then built as a probe and measured, and it is
affordable:

    alias-only reorder, no record   163,353,361
    with the per-site record        163,380,706   +27,345

1.7% of the prize, leaving -1,541,851 (-0.9349%) against 164,922,557. And the
27,345 is not the recording. THE PASS REWRITES NOTHING ON ANY MEASURED CORPUS: a
counter at the rewrite site reads 0 sites on bench/entry_corpus, 0 on
bench/compile_corpus and 0 on lib/json, against 1 on the `m/thing` fixture that
draws the opacity refusal. The vector never allocates, so what the 27,345 buys is
an extra parameter carried through a recursive walk over every expression in the
program, and a shape that hangs the recorder off a walker rather than threading
it should cost less. Two things a real implementation must handle that the probe
did not: the reader half in check.rs, and the second caller of the same walker at
src/lib.rs:2616, which walks with the door map.

**And the two readers want different things, which reading `foreign_constructions`
settles.** Its own comment states the invariant the reorder breaks, at
check.rs:1847: "A qualified name can never be a local binding, so unlike the
arity walk beside it this needs no shadowing set: the slash IS the foreignness."
That holds only while every slash in the merged program was written by a person.
After the alias pass has run a slash also means the pass put one there, and the
check fires on `m/thing 0` -- a call of an imported function -- as though it were
a construction of the imported type of the same name. So opacity does not want a
spelling to quote. It wants to SKIP a head the pass rewrote, because that head
was never a construction. Arity is the one that wants the spelling. One record,
two uses, and a fix that handed both readers the old name would leave the opacity
refusal exactly where it is.

This is not a gavel: the
substance was ruled in kanso#1120, a diagnostic names what the import writes, and
which mechanism satisfies it is the implementer's.

**How this came to be built twice, since the answer is a process one.** The task
list carried it as BUILT AND PROVEN with a full `cargo test` behind it. The
kanso#1329 entry names that exact evidence as worthless here --
`scripts/module_differential` is a kanso program run by the diagnostics-
differential CI job and `cargo test` never invokes it -- so the suite was green
both times and said nothing both times. The filing search caught it before the
branch was pushed, which is what the search is for.

- **DECLINED** — the entry path's alias-pass reorder, in any shape that leaves a
  check reading a rewritten name. Re-measured, re-refused, and this time the
  opacity refusal is on the record beside the arity one.
- **OPEN** — the pre-canonical spelling for `check_merged`'s two name-reading
  checks. Worth -1,569,196 on the entry row in the alias-only shape, and
  -1,541,851 with the per-site record that makes it sound. What is unbuilt is
  the reader half: the two checks in check.rs that have to consult the record
  instead of the node, and a fixture for each.
- **OPEN, unchanged** — the twins inside `infer`, which is the other half of the
  reorder's value and is blocked on a different thing: `infer` indexes
  declarations positionally, and a group keyed by (name, arity) is a dispatch
  group, so the twin is what lets a bare name resolve.

## 2026-09-08 — the welfare column spans four measurement epochs and the rewrite scores them on one ruler

Clay, reading kanso-lang.dev/numbers after kanso#1331 landed: "I still have
no clear accounting of why the welfare went down and the website still does
not look great." The account, read off the rewritten column on
origin/perf-history:

    2026-09-06 11:26   91.57 -> 58.96   -32.62   #1284  one consolidated run program
    2026-09-08 01:53   70.03 -> 67.75    -2.27   #1321  the compile corpus is named
    2026-09-08 10:17   67.91 -> 66.29    -1.63   #1331  the compile term sums both compiles

None of the three is the compiler getting worse. Each is a change in what is
measured, and the column still steps at each one because
`scripts/welfare_rescore` scores every row against the single baseline the
floor file holds today.

**The mechanism.** The rewrite exists so the column is "rewritten under one
formula whenever the formula moves, which is what makes two points on it
comparable" (docs/numbers.html). One formula does make rows comparable when
the WEIGHTS move. It does not when the MEASUREMENT moves, because the counters
change magnitude while the baseline does not. Today's compile baseline is
671,773,822, and the rows it divides come from four epochs:

    epoch                           compile_instructions   ratio    term
    lib/json, one compile                 19,316,962       34.78   0.9858
    compile_corpus, one compile           52,603,220       12.77   0.9623
    compile_corpus, module row            48,757,859       13.78   0.9650
    compile_corpus, both summed          212,644,590        3.16   0.8634

Adjacent epochs differ by a factor that is the workload and never the
compiler, and the term falls across each boundary by that factor. The -2.27
and the -1.63 are those two falls, weighted. #1331 re-based the floor so the
row it wrote is right; it could not re-base the rows before it, because the
rewrite has no notion of an epoch to re-base them to.

**The run side is the same disease and the larger cliff.** #1284 re-based the
run counters to parity at the changeover, so every row before it scores its
run terms against a baseline it never carried. Clay ruled the repair on
2026-09-07: share-weighted phases, renormalised over the phases a row carries,
based at row 70 (2026-08-10), and the ruling closes with "the rewrite is
cloud's." As of this entry, `scripts/` holds no share-weighted reconstruction.
The -32.62 on the chart is that ruling unbuilt.

**What makes the column flat across a change of measurement.** The rewrite
needs an epoch table: for each change of measurement, the head it happened at
and the per-row factor measured there. The corpus move's factors are already
recorded in the floor file (2.7232, 2.7207, 2.1047) and the summing's is
computed in kanso#1331. A row is then scored against the baseline scaled to its
own epoch, which is one re-basing per epoch, applied in the rewrite rather than
only in the floor file. It is what the three floor-file precedents did by hand
for the current row, done for every row.

**OPEN, cloud's, two items.** Build the 2026-09-07 ruling for the run side.
Apply the same per-epoch scaling to the compile side's two changes of
measurement. Until both land, the chart shows the history of what was measured
rather than the history of the compiler, and no reader can tell which.

## 2026-09-08 (sixth) — the pre-canonical spelling, and the entry reorder ships

Searched the log, the archive and design/ before filing: the entry above
("the entry reorder re-derived") is this thread's own, and leaves exactly this
as OPEN with the recorder measured and the reader half unbuilt. kanso#1329
records the first revert, kanso#1328 the module path's reorder, kanso#1120 the
ruling both readers have to satisfy. This builds what that OPEN item names.

**The reorder ships, with the two readers that make it honest.**
`canonicalize_bare_aliases` runs in front of `check_merged` on the entry path
now, so the whole-program check no longer walks the synthetic twins the pass is
about to delete. `canonicalize_types` stays in the success arm, which is the
cheaper of the two shapes by 8,930 instructions.

The pass returns a `Rewrites` — line and column to the bare name it replaced —
and `check_merged_after_aliases` hands it to the two checks that read a call's
name. Every other caller runs the pass after the check and passes an empty
record, where both readers behave as they always did.

**The two readers want different things, and that is the whole finding.**

    check.rs arity (two sites)   quotes the recorded bare name
    foreign_constructions        SKIPS a head the pass rewrote

Arity is a wording question and kanso#1120 settles it: the diagnostic names what
the import writes. Opacity is not. Its own comment states the invariant, at
check.rs:1847 — "A qualified name can never be a local binding, so unlike the
arity walk beside it this needs no shadowing set: the slash IS the foreignness."
That holds only while every slash was written by a person. After the pass, a
slash also means the pass put one there, and the check fires on a call of an
imported function as though it were a construction of the imported type of the
same name. No wording of that message is right; the site is not a construction
at all.

**On scripts/module_differential: 0 wrong, from the 2 wrong the reorder cost
before.** Both objections are gone, and both readers were watched red on their
own:

    opacity skip disabled   1 wrong -- `m/thing` is foreign, on a program that compiles
    arity spelling disabled 1 wrong -- quotes `m/one` where the source says `one`

Each mutation loses exactly its own fixture and no other, so neither reader is
dead code and neither is doing the other's work.

**What it costs, in this box.**

    entry_instructions   164,922,557 -> 163,499,802   -1,422,755  (-0.8627%)
    compile_instructions  49,170,337 ->  49,207,870      +37,533  (+0.0763%)
    summed                214,092,894 -> 212,707,672  -1,385,222  (-0.6470%)

The module row rises for the same reason kanso#1332's did: the path pays for
something it cannot use. Its record is always empty, and what it pays is a
parameter carried through `arity_walk_expr` and `foreign_constructions`'s walk,
both recursive over every expression. Neither lookup runs on a clean module
compile -- the arity one sits inside the refusal branch and the opacity one
behind a name being in the foreign set -- so the cost is the threading, not the
reading. Under kanso#1331's summed compile term the trade is 38 to 1 in favour,
and the sum is what the objective reads.

Against the reorder measured WITHOUT the readers (163,353,361), the readers cost
the entry row 146,320. The probe in the entry above put the recorder alone at
27,345; the rest is the two further walkers now carrying the same parameter.

These are container numbers and none of them is a row. CI counts both compile
veins, and this branch expects a deliberate red first round for exactly that.
**CI's rows, and what the container got wrong about them.**

    entry_instructions   163,612,976 -> 162,170,772   -1,442,204  (-0.8814%)
    compile_instructions  48,761,165 ->  48,791,172      +30,007  (+0.0615%)
    summed               212,374,141 -> 210,961,944   -1,412,197  (-0.6650%)

Welfare 66.2898 -> 66.30, ratcheted in the same change.

The container projected -1,422,755 and +37,533. Sign and order right on both,
digits wrong on both, and the two errors ran the same way: it UNDERSTATED the
entry fall by 19,449 and OVERSTATED the module rise by 7,526. Its standing
offset is +0.8% on the LEVEL of each row, so the naive expectation was that it
would overstate a fall; a level offset between toolchains does not carry to a
delta, and this pair is the demonstration. Under the summed term the trade is
48 to 1 in favour, against the 38 to 1 the container projected.


**The module path had both defects live, and nothing in the tree asked it.**
kanso#1328 moved the same pass in front of the same check on the module path
three days before this, and handed the check nothing. So on main today:

    kanso check <a module>   opacity REFUSES a program that compiles
    kanso check <a module>   arity quotes `m/one` where the source says `one`

Reduced, that is a module whose sibling declares `pub type thing` beside
`pub fn thing _`, importing it and calling `thing 0`. The entry-path form of
exactly that program is c7 in scripts/module_differential, and it was watched
through both reverts of the entry reorder; the module form had no case at all,
so the sweep read 0 wrong on a defect it could not see. The fix is the entry
path's, and both programs go into the sweep as c26 and c27 -- watched red on the
pre-fix compiler for the two messages above, verbatim, before they went green.

Threading the module path costs almost nothing because it was already paying:
`check_merged` built an empty `Rewrites` on every call, and the change replaces
that construction with the real one. Entry +121, compile +63 against the
readings in the table above, both already folded in.

**And the third path is now watched before it moves.** `kanso check` on a single
library file takes `compile_library`, which still checks before it canonicalizes
-- so both readers are right there today. c28 and c29 say so, and they were
watched red by making exactly the reorder the OPEN item below proposes for that
path: both go wrong together, with the same two messages. `compile_one` carries
a byte-identical block, so the mutation is one edit applied twice and the two
paths answer as one.

**What this says about where a defect gets found.** The reorder was reverted
twice on the entry path for objections the sweep caught within a round, because
the entry path had cases. The same reorder shipped on the module path and its
two objections sat for three days. The corpus decides what a sweep can see, and
a path with no case in it reads clean whatever it does.

- **DONE** — the OPEN item the entry above filed. The reorder, the record, both
  readers, both mutations, and the differential back to 0 wrong.
- **OPEN, and now priced** — src/lib.rs:348 and :425 still check before
  canonicalizing. compile_one is reached only from `compile_repl`
  (src/repl.rs:290) and compile_library only from `kanso check <a library
  file>`. Both merge `dep_program`, so both see the twins, and both would break
  the way the entry path did -- they were never blocked on a measurement, they
  were blocked on this.

  Measured on this box, on `kanso check bench/compile_corpus/compile_corpus.kso`
  with the reorder and the record applied to both sites:

      library_instructions   50,244,948 -> 48,681,802   -1,563,146  (-3.111%)

  Larger in proportion than the entry path's -0.8627%, on a path no vein
  watches. The differential stays 29 cases 0 wrong through it, which is what
  says the record makes the reorder correct there and not merely cheaper; c28
  and c29 go red on the same edit with the record left out. The baseline
  reproduced to the instruction on a second run. What is owed before it ships
  is the vein, since a fall nothing counts is a fall nothing keeps.
- **OPEN, unchanged** — the twins inside `infer`, the other half of the
  reorder's value. `infer` indexes declarations positionally and a group keyed
  by (name, arity) is a dispatch group, so the twin is what lets a bare name
  resolve.

---

## 2026-09-08 (seventh) — the third compile path gets a row

`kanso check` routes a single file by its content and the three routes are
three different compiles. A directory is a module and takes
`compile_module_inner`, which is what `bench/compile_corpus` and
`compile_instructions` watch. A file with bare statements is an entry and takes
`compile_parsed_entry`, which `bench/entry_corpus` and `entry_instructions`
have watched since kanso#1330. A file of definitions alone is a library and
takes `compile_library` — and nothing in the tree counted it.

That path is not a corner. `kanso test` takes it on every run, and so does
`kanso check` on any single file that is not an entry, which is most files
here.

`bench/library_corpus/library_corpus.kso` names ten imports and uses each, the
shape `bench/entry_corpus` has and for the same reason: `compile_library`
merges the dependency program and runs its own whole-program check over
everything the imports bring, so a corpus with one small import would measure
mostly the work underneath it. The directory is named to the same length as
`compile_corpus`, because the count tracks the length of the path the compiler
is handed at about 160 instructions a character.

The container projected 165,589,540 and CI wrote the row: **164,253,088**, on
binary sha 9bc8f829af68 in the job that also counted
compile_instructions=48,791,172 and entry_instructions=162,170,772, so all
three answer for one build. The projection is 1,336,452 high, +0.8136%, which
lands on the offset the other two rows already carry between this box's rustc
1.94.1 and CI's 1.98.1. Only CI may write the row, and the reason is that
offset.

CI's summary named exactly one failing vein and eighteen green, which is what
round one was for.

**A CORRECTION, made the round after the claim.** This entry and round two's
commit message both said the ratchet job proved `library_ir` on the runner.
Read the job log: it did not, and could not have. The ratchet's second pass is
`ratchet -- touched origin/main`, which selects only rows patching a file the
branch changed, and it reported

    ratchet: 1 rows patch a file this branch changed
      the ratchet (every gate has a mutation that turns it red)

-- one row, the ratchet's own. This branch touches ci.yml, CLAUDE.md, three
gate scripts, the ratchet, the trend gate, a spec, the log and two new bench
files, and no `src/`; `the_library_program_is_checked_twice.sh` patches
`src/lib.rs`, so the touched guard correctly passed it over. What CI did run is
the first pass, `every mutation still matches the source it patches`, which
does read the new mutation's anchor against `src/lib.rs` and found it. So the
anchor holds on the runner and the row's provability there is untested; it was
proved in the container, 165,589,540 -> 190,698,277. The next change to
`src/lib.rs` -- the reorder -- is the branch that will select this row and
prove it on CI.

**THE SPEC PREDICTED ITS OWN FAILURE MODE AND THIS IS THE INSTANCE.**
`tests/the_compile_sweep_names_every_compile_gate.rs` derives the sweep's list
from goldens matching `bench/compile_*` and `bench/entry_*`, and its own doc
comment says: *a prefix list is exactly the shape that goes stale when a vein
is added under a new name.* `bench/library_instructions_golden.txt` matches
neither prefix, so both derivations in that file walked straight past it and
the sweep would have looked like coverage while missing the newest vein. Both
are widened here, in the commit that adds the vein.

The trend gate's own coverage spec did NOT have that hole:
`tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads `bench/` off
disk and keys on `contains("golden")`, so it went red the moment the file
existed and named what was missing. Two coverage specs over the same tree, one
keyed on a prefix and one on a substring, and only the substring one survived a
new name.

A third coverage spec found the other half of the same gap. The ratchet keeps a
`host_bound` list of the gates that count under callgrind, so that a runner the
golden does not name is reported as unproven rather than credited as a
regression, and `tests/a_host_bound_gate_is_reported_not_credited.rs` derives
that list from the gates that actually run the tool. It went red naming `sh
scripts/gates/library_instructions.sh` as soon as the gate existed. Three specs
over one tree: the substring-keyed pair spoke, the prefix-keyed one did not.

The mutation is `the_library_program_is_checked_twice.sh`, the library twin of
the entry one, and its anchor takes two steps rather than one:
`let merged_diags = check::check_merged(&program, false);` appears twice in
`src/lib.rs` because `compile_one` carries a byte-identical block, so the
function is found by its signature and the duplicate goes in at the first such
call after it. What it proves is the argument for the row: the same edit leaves
`compile_instructions` and `entry_instructions` green. It rides in the ratchet
as `library_ir`, beside `compile_ir` and `entry_ir`.

Proved rather than assumed, in the order the rule asks for: it applies (two
calls become three, and the third is inside `compile_library` at 432 with
`compile_one`'s at 355 untouched), it compiles, and the row it moves goes
165,589,540 -> 190,698,277, a rise of 25,108,737 or 15.16%, against a gate that
asserts equality. Counted here with the host check bypassed on purpose, because
this container may not compare the row and the question was whether the
mutation moves it rather than what the value is. Restored, rebuilt, clean.

Still open, unchanged by this: `src/lib.rs`'s two remaining callers check
before they canonicalize. That reorder is measured — 50,244,948 -> 48,681,802,
**−1,563,146 / −3.111%** — and was blocked on this vein. It is not in this
commit, so the row this one opens is the pre-reorder baseline and the next PR
is what spends it.

---

## 2026-09-08 (eighth) — the last two callers check before they canonicalize

kanso#1328 put `canonicalize_bare_aliases` in front of the whole-program check
on the module path and kanso#1335 did it on the entry path. `src/lib.rs` kept
two callers running the old order: `compile_one`, reached only from
`compile_repl`, and `compile_library`, reached from `kanso check` on a file of
definitions. Both merge `dep_program`, so both see the twins, and both would
break the way the entry path did. They were never blocked on a measurement.
They were blocked on a vein, and the vein opened yesterday.

Measured on the library corpus, both readings on this container, the recipe
`scripts/gates/library_instructions.sh` uses minus the host check:

    library_instructions   165,589,540 -> 164,300,594   -1,288,946   -0.7784%

Read twice, on two builds whose shas differ, and identical to the instruction.

**The recorded −3.111% is a different workload and does not belong to this
row.** The 2026-09-08 (sixth) entry measured `kanso check
bench/compile_corpus/compile_corpus.kso` treated as a library file, 50,244,948
-> 48,681,802, and that corpus is a third the size of `bench/library_corpus`,
so the same absolute saving reads as four times the proportion there. The row
this change is watched by falls 0.7784%.

**The record was watched red before it was watched green.** The reorder without
it takes `scripts/module_differential` from 29 modules 0 wrong to 2:

    a bare call to a shared name, from a library file
      expected it to compile:
      error[opacity]: `m/thing` is foreign -- only `m` builds a `thing`;
      ask it for one through a pub function

    a call from a library file at the wrong arity
      refused, but not with
      'error[arity]: no 2-argument arm of `one` (arms take 1)':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

The first REFUSES A PROGRAM THAT COMPILES; the second is the spelling
kanso#1120 settled. Both are the library-path twins of what the entry path had,
which is why the record ships with the reorder rather than after it. With
`check_merged_after_aliases(&program, false, &rewritten)` at both sites the same
run is 29 modules, 0 wrong.

This is also the branch that finally proves `library_ir` on CI. The ratchet's
`touched origin/main` pass selects only rows patching a file the branch
changed, and kanso#1337 touched no `src/` — so the new row's mutation was
verified there only as an anchor that still matches, never run. This change
patches `src/lib.rs`, which is what that mutation patches, so the touched pass
selects the row. Whether it turns red on the runner is CI's to say, and the
paragraph below is what happened on the first attempt.

`canonicalize_types` stays where it is. On the entry path moving it too read
8,930 instructions worse, because the alias pass deletes the twins before that
one would have walked them, and there is no reason to expect the other
direction here.

**CI's row.** The vein summary named exactly one failing counter against
eighteen green, which is what round one was for:

    library_instructions   164,253,088 -> 162,970,167   -1,282,921   -0.7811%

The container projected a fall of 1,288,946, or 0.7784%. It overstated the
saving by 6,025 instructions — half a per cent of the fall — while reading
0.8136% high on the LEVEL of the row. A level offset between toolchains does
not carry to a delta, and this is the second sitting to say so: kanso#1335
recorded the same thing on the entry and module rows, where the projections
also came out conservative.

**THE OTHER TWO COMPILE ROWS DID NOT MOVE AT ALL**, and that is worth writing
down because this change edits `src/lib.rs`, the compiler's own Rust.
`compile_instructions=48,791,172` and `entry_instructions=162,170,772` are
byte-identical to their goldens in the job that counted the new row. CLAUDE.md's
prior is that `compile_instructions` usually moves on such an edit through
layout alone; it holds, and this is the second recorded change small enough to
leave it alone, after kanso#1285.

**The mutation's anchor went stale in this same change, and pass one of the
ratchet is what said so.** `the_library_program_is_checked_twice.sh` anchors on
`let merged_diags = check::check_merged(&program, false);`, which is the exact
line the reorder rewrites, at both sites. So round one's ratchet job failed
`every mutation still matches the source it patches` and skipped the touched
pass underneath it — the pass this branch exists to reach. The anchor moves to
`check::check_merged_after_aliases(&program, false, &rewritten)` and the
duplicate lands at 465, inside `compile_library` (390), with `compile_one`'s
call at 372 untouched.

A mutation anchored on a line a change is about to rewrite goes stale in that
change. That is the ordinary case rather than a surprise, and the first pass
exists to report it on the runner before the row it guards is ever read.

**A FIFTH READER, found by running the trend gate on this change's own diff.**
The fall printed as UNCLASSIFIED:

    UNCLASSIFIED — no direction table names these, so they count
    toward neither side of the pure-regression rule:
      library_instructions

The gate walks the golden and has no direction for the counter in it, so it can
report the move and cannot say which way is better. A rise of the same size
would have printed identically and counted toward neither side of the
pure-regression rule. `entry_instructions` has been in the same position since
kanso#1330 — two compile veins unclassified for as long as they have existed.

This is the digestbench failure one vein down, which
`tests/every_benchmark_in_the_work_vein_has_a_direction.rs` exists to prevent
and could not see here: it reads `bench/instructions_golden.txt` and stops. Its
own argument carries straight across — each compile golden holds retired
instructions for one compile and fewer is better in all of them, so every row
has a direction and none of them is a presence counter whose direction means
nothing alone.

`tests/every_compile_vein_row_has_a_direction.rs` asserts it over every
`bench/*instructions_golden.txt` other than the work vein's, read off disk, so a
fourth compile path opening a vein under a fourth name is covered on the day the
file lands. Watched red first, naming both: `["entry_instructions",
"library_instructions"]`. Both join `lower_gg` and the gate now prints
`improved: library_instructions 164,253,088 -> 162,970,167`.

That makes five readers a compile vein owes: its gate, `all_compile.sh`'s
`gates=` line, the trend gate's walk, `golden_prose`, and the trend gate's
direction table. kanso#1337 wired three of them.

---

## 2026-09-08 (ninth) — the twins inside infer, sized at last

The log has carried this OPEN item since 2026-09-08 (fifth) and twice since:
*the twins inside `infer`, the other half of the reorder's value.* It has never
had a number. It has one now, and the number is small.

`enroll_bare` (src/lib.rs) clones every exported declaration of every imported
module under its short name, and those clones are real declarations that infer
and check both walk. On `bench/library_corpus` the top-level enrollment makes
**292 function twins and 45 type twins against a merged program of 1,437
functions** — one declaration in five. infer is 22.5% of that compile.

**THE OBVIOUS INSTRUMENT DOES NOT WORK, and the way it fails is the blocker
arriving as a diagnostic.** An env-gated early return in `enroll_bare`, so the
twins are never made, stops the compile at

    error[name]: `first` is already a declaration; rename the binding
      --> std/regexp/regexp.kso:654:3

Inside the standard library, not at the corpus's call sites -- `library_corpus`
writes every call qualified and needs no bare twin of its own. The bare space is
load-bearing inside std. That is the positional-index blocker already recorded
for this thread: `group_members`, `ctx.current_index` and the reader bitmap are
all indexed by position in `program.fns`, and beat.rs, check.rs and codegen.rs
all read `inference.returns[i]` by that same position.

**The instrument that does work: skip only the BODY WALK.** In infer's fixpoint
sweep, beside the dirty test, skip a declaration whose `synthetic` is set. Every
twin stays in `program.fns`, so no index moves. All readings on one probe
binary, so the added branch cancels:

    corpus                walked        skipped        delta
    library_corpus   167,284,685   166,685,167     -599,518   -0.3584%
    entry_corpus     164,832,926   164,512,869     -320,057   -0.1942%
    compile_corpus    49,403,663    49,412,576       +8,913   +0.0180%

infer itself: 38,972,582 -> 38,342,754, -1.6161%.

**One declaration in five is one instruction in sixty-two.** A count-based
estimate assuming infer is linear in the declaration count says 20.3% of infer,
4.57% of the row; the measurement refutes that by 12.6x. The reason is the
fixpoint: only DIRTY declarations are revisited, and a twin nothing calls goes
clean after its first visit and is never re-dirtied again.

**The module row goes the wrong way, and that is a finding about the probe.**
Skipping the walk leaves `returns[twin]` at its default instead of the answer
the walk would have written, so other declarations infer different values and
the dirty sets and round count move with them. The delta is a mix of walks not
taken and a fixpoint doing different work, and on `compile_corpus` the second
term wins. So these three numbers size the thread; they are not a ceiling in the
strict sense.

**Where that leaves it.** The shipping shape has to copy the original's answer
into `returns[twin]` rather than leave it at the default, and that copy costs
something none of this pays. So the honest reading is a couple of tenths of a
percent before the copy, against the alias-pass reorder's 0.7784% on the same
row in the same session. The thread goes to the back of the queue: not refuted,
not worth building next, and no longer unpriced. What is owed first if it is
ever picked up is recording the twin/original pairing at the `enroll_bare` clone
site, which is the cheap half of the copy.
