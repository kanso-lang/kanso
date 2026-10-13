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


## 2026-10-10 — an oversize block that cannot be reused is freed when it misses

A loop that appends a byte a pass to a one-megabyte string ran 6,000 passes
with an arena peak of 3,157,760 bytes and a resident peak of 777 MB. The arena
gives a value over a megabyte a block of exactly its size. When the loop
rewinds, that block goes to the spare list, and the next pass asks for a block
a few bytes larger, which no spare can serve. So every pass opened a new block,
and the old ones stayed in the spare list until the loop ended. Valgrind
counted 99 mallocs and 82 frees at 300 passes, nearly all of the frees at the
loop's end. Main reaches 394 MB on the same program, and the five port fixes
had roughly doubled it, because they rewind more often.

`k_arena_push` now frees the oversize spares smaller than the request when an
oversize request misses. The standard 1 MiB spares are kept, since they serve
ordinary refills. The program's resident peak falls from 777 MB to 10 MB, and
it runs in 0.96 seconds against 1.2.

No counter could see this. `arena_peak_bytes` counts blocks in use and
`arena_blocks` counts mallocs, and both read the same before and after: 752
blocks and a peak of 3,157,760. A new counter, `spare_peak_bytes`, records the
most the spare list held, and it is lower-is-better in the trend gate. It is
counted only in counting builds, so no program pays for it. Every cost golden,
every mem fixture and the two book samples that print counters gain the line
and nothing else.

`tests/golden/mem/a_string_that_grows_each_pass_reuses_its_block` is the
reduction at 400 passes. Its spare_peak_bytes is 2,097,984; with the miss path
left as it was, it reads 52,440,000. The run program's spare_peak_bytes is
2,621,456. runbench reads 248 instructions more, measured in one worktree
against the same tree without the fix, and is projected at 1,152,754,683.

One reading from this container does not agree with CI. Measured in the
carrier's own worktree, runbench read 1,152,573,288, below the lap wait's
figure; CI reads the carrier at 1,152,754,435, which agrees with the wait's
cost. That stray reading is not explained here, and the projection above is
taken from CI's row plus the 248.

A second shape retains far more and is not fixed here. When a loop builds the
new string by interpolating the old one directly, as in `"{st.body}{x}"`, the
arena peak grows a megabyte a pass on main (420,568,976 bytes at 400 passes)
and half that on the carrier. Every version of the string stays in a block in
use. Slicing the string first, as the fixture does, keeps the peak flat at
3 MB.

The mechanism is the repair. The interpolation lets the record-reuse analysis
write each new state into the old record's storage, and that record was built
before the loop, below its mark. At a stage the record survives but its new
string does not, so the walk repairs it: `k_repaired_settle` copies the string
into the arena and raises the mark over the copy, because a surviving node
points at it. The next pass writes a newer string into the same record, and the
settled copy is dead under the mark until the loop ends. A trace of the beat
carry shows the mark's block moving 48 bytes up at each of the 200 stages of a
400-pass run, with a megabyte more live beneath it each time. Raising the mark
is right for a node repaired once. What it lacks is a way to take back a
settled region when the same node is repaired again, and that is a separate
change.

CI's rows replace the projection. runbench reads 1,152,754,609, which is 174
instructions above the carrier's row rather than the 248 measured here.
Measured against main, the rows that end worse on this branch, the carrier's
changes included, are codegen_instructions_dev at 118,955,169,
codegen_instructions_release at 454,306,200, browser_run_instructions at
34,124,990, work_encodebench at 2,398,370,414, work_livebench at
1,474,871,090, work_oneshot at 13,224,195 and text at 4,835,550. Against the
carrier alone, the release row falls by 120,818 and the dev row rises by
3,943; every benchmark's runtime text grows 176 bytes, which is the spare
list's accounting and its drop path.

The carrier took main's retired `build` and CI's rows for it, and this branch
took the carrier in a merge commit. Each conflicted row is projected as this
fix's own CI delta laid on the carrier's CI row: codegen_instructions_dev lands
on 118,958,562, work_encodebench on 2,398,370,022, work_livebench on
1,474,871,748, work_oneshot on 13,224,265 and work_runbench on 1,152,754,602,
and `text` totals 4,829,278. CI's reading replaces each of them.

CI's rows replace the projection. codegen_instructions_dev lands on
118,939,055 and codegen_instructions_release on 454,517,107, the release tier
230,743 above the projection. work_encodebench lands on 2,398,369,553,
work_livebench on 1,474,871,531, work_oneshot on 13,223,698 and work_runbench
on 1,152,754,392. `text` totals 4,829,278, as projected.

## 2026-10-10 — a counting loop keeps every pass's key; copying keys out measured and declined

A loop that counts into a map it was handed, under a key it interpolates each
pass, never rewinds. `put m k (m[k] + 1)` is an in-place put and replaces the
count, so the key string is garbage once the put has found its slot. The beat
licence refuses the loop anyway, because a key the put does store would be
freed under the map by the next rewind. At 100,000 passes over ten keys the
arena peak is 4,194,304 bytes, 32 bytes a pass. A loop that builds a new string
value each pass into a map, `put m "body" "{m["body"]}{x}"`, keeps every
version the same way: 105,913,696 bytes of arena at 100 passes of a one-megabyte
string, 420,568,976 at 400, on main and on every branch in flight.

The fix tried was to let the put copy a key it inserts out of the arena, and
license the loop. The copy is registered with the map's pairs and freed when
they are, so a function that builds a map of fresh keys in a loop does not leak
across calls: peak RSS held at 10 MB from 2,000 to 200,000 calls. Inside a loop
that rewinds, a put with a string key calls the runtime instead of the emitter's
inline fast path, which would store the key uncopied. Routing every string-key
put to the runtime whenever any beat was live cost jsonbench 16.6% and runbench
8.2%; routing only the puts inside the newly licensed loop cost nothing
measurable on the instruction rows. The counting loop's arena peak fell to
1,048,576 bytes and stays there.

It is declined on memory. Licensing the loop moves its map's pairs out of the
arena into malloc'd storage, which the objective counts, and in the run program
nothing came back from the arena to pay for it: run_peak_bytes rose from
4,948,512 to 5,202,176, and welfare fell from 91.0318 to 91.0163. basket also
read 901,269 more instructions. A version worth trying keeps the pairs in the arena and
copies the key into the region below the loop's mark, which needs an allocation
point the arena does not have.

## 2026-10-10 — a record is reused in place only above the loop's mark

The retention at the end of the previous entry has a smaller fix than taking
back settled regions. Record reuse writes a new state into the old record's
storage, and inside a beat loop that storage can sit below the loop's mark,
where a rewind cannot reach it. `k_rec_reuse` now reuses a record in place only
when the record lies in the mark's own block, between the mark and the arena's
current position. Anywhere else it builds the record fresh, above the mark,
and the old one is left for the rewind. Outside a beat loop nothing changes.

The cost of building fresh is one record allocation a stage. The cost it
removes was a megabyte a pass: at 400 passes of the interpolating loop the
arena peak falls from 211 MB on the carrier to 3 MB.

`tests/golden/mem/an_interpolated_state_string_is_not_kept_under_the_mark`
runs the shape for 100 passes, `"{st.body}{st.count % 10}"` on a one-megabyte
string. Its arena_peak_bytes is 3,145,984; with the check removed it reads
54,529,792. It makes 213 allocations and evacuates 52,435,792 bytes, which is
the string copied at each stage and is the price of carrying it.

Four mem fixtures and the basket golden move, each by records that are now
built fresh where they were overwritten:

- `record_reuse_shape`: allocs 4 to 6, alloc_bytes 176 to 304, evac_allocs 3
  to 5, evac_bytes 80 to 144, survive_slots 8 to 4, sh_rec 64 to 192.
- `a_long_chain_answers_survival_from_its_index`: allocs 240,008 to 240,004,
  chain_finds 477,919 to 95,601, survive_slots 500,024 to 100,004, evac_bytes
  448 to 608, sh_rec 224 to 608.
- `a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap`: allocs 180,003
  to 180,002, alloc_bytes 12,160,400 to 12,160,432, evac_bytes 6,128 to 6,160,
  survive_slots 8 to 4, sh_rec 2,560,128 to 2,560,192.
- `an_evaluator_carrying_any_value_round_its_cycle_rewinds`: allocs 91,429 to
  91,445, alloc_bytes 4,114,384 to 4,115,408, sh_rec 2,560,192 to 2,561,216.
- `bench/cost_golden_basket.txt`: allocs 27,263 to 27,265, alloc_bytes
  7,522,129 to 7,522,257, evac_allocs 3 to 5, evac_bytes 192 to 256,
  survive_slots 4 to 0, sh_rec 130,048 to 130,176.

The chain fixture's chain_finds falling to a fifth is the walk no longer
following records that were rewritten under it.

Instructions were measured in one worktree against the spare-list branch this
one sits on. basket reads 60,732 more, livebench 602 more; runbench reads 392
fewer, encodebench 280 fewer and oneshot 644 fewer. They are projected into
`bench/instructions_golden.txt` from that branch's figures, and CI's rows
replace them.

CI read every instruction row exactly as projected. Three rows end worse than
main, the two branches below this one included: codegen_instructions_dev at
118,957,002, codegen_instructions_release at 454,309,037 and text at 4,836,670.
Against the spare fix alone the dev row rises 1,833 and the release row 2,837,
and each benchmark's runtime text grows 80 bytes, which is the check before a
reuse.

By the trend gate's keys, the rows this change worsens land at basket_allocs
27,265, basket_evac_allocs 5, basket_evac_bytes 256 and basket_sh_rec 130,176;
record_reuse_shape_allocs 6, record_reuse_shape_alloc_bytes 304,
record_reuse_shape_evac_allocs 5, record_reuse_shape_evac_bytes 144 and
record_reuse_shape_sh_rec 192; and, projected, work_encodebench 2,398,370,134,
work_livebench 1,474,871,692 and work_oneshot 13,223,551. Each record row is a
record built fresh above the mark where it used to be written below it.

The branch took the spare fix after that fix took the carrier, and each
conflicted row is projected as this branch's CI delta on the spare fix's row.
Measured against the spare fix, the rows that end worse are
codegen_instructions_dev on 118,960,395, codegen_instructions_release on
454,289,201, work_basket on 30,873,237, work_livebench on 1,474,872,350 and
`text` on 4,830,398, 80 bytes a binary for the mark check. basket_alloc_bytes
lands on 7,522,257. Three mem fixtures that keep a record across a cycle read
it as a fresh allocation where they used to reuse it.
a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap_alloc_bytes lands
on 12,160,432,
a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap_evac_bytes on 6,160
and a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap_sh_rec on
2,560,192. a_long_chain_answers_survival_from_its_index_alloc_bytes lands on
19,672,832, a_long_chain_answers_survival_from_its_index_chain_finds on 95,601,
a_long_chain_answers_survival_from_its_index_evac_bytes on 608 and
a_long_chain_answers_survival_from_its_index_sh_rec on 608.
an_evaluator_carrying_any_value_round_its_cycle_rewinds_alloc_bytes lands on
4,115,408, an_evaluator_carrying_any_value_round_its_cycle_rewinds_allocs on
91,445 and an_evaluator_carrying_any_value_round_its_cycle_rewinds_sh_rec on
2,561,216. CI's reading
replaces the projected rows.

CI's rows for this branch were read on the spare fix before k_b_adopt was
retired beneath it. With that retirement's deltas added, the rows land on
codegen_instructions_dev 118,935,145, codegen_instructions_release
454,454,756, work_basket 30,873,237, work_encodebench 2,398,370,771,
work_livebench 1,474,870,922, work_oneshot 13,224,076 and work_runbench
1,152,728,036. Against the spare fix the dev row rises 1,901, the release row 2,593, basket
60,732, encodebench 1,120 and oneshot 371, and livebench falls 679. With the
browser row the spare fix carries, welfare scores 91.0338407, 0.0000035 below
the floor the spare fix banked and inside the gate's 0.001 allowance, so the
floor stays where the spare fix left it.
`text` totals 4,825,470, 80 bytes a binary above the spare fix for the mark
check.

CI then read this branch on the spare fix with k_b_adopt retired, and those
rows replace the ones above. codegen_instructions_dev lands on 118,940,032,
codegen_instructions_release on 454,512,392, work_basket on 30,873,237,
work_encodebench on 2,398,369,266, work_livebench on 1,474,870,859,
work_oneshot on 13,224,090 and work_runbench on 1,152,729,429, and `text`
totals 4,825,438. Against the spare fix's CI rows, the dev row rises 2,514, the
release row 2,552, basket 60,732, oneshot 189 and runbench 686; encodebench
falls 756 and livebench 343. Welfare scores 91.0338012, 0.0000081 under the
91.0338093 the spare fix banks from its CI rows and inside the gate's 0.001
allowance, so the floor stays where the spare fix left it.

## 2026-10-11 — the first join makes its own builder, and `k_b_adopt` goes

A builder seed that is not a string reached its first join through
`k_b_adopt`, which rendered it and made the rendering the builder. The fold
join (kanso#1825) taught `k_concat_arr_mut` to do the same for any seed that
arrives with no room, and from then on the adoption had nothing left to
convert. The ratchet showed it: both rows that mutated the adoption stayed
green on the carrier, and were replaced by one row that restores the old
refusal in `k_concat_arr_mut`. This change removes the call, its declaration
and the runtime function.

Measured in this container against the carrier, runbench reads 26,076
instructions fewer, because the run program's builder loops no longer call
the adoption on their first join; its emitted code loses one call and two
lines. `codegen_instructions_dev` falls by 5,811, `codegen_instructions_release`
by 64,944 and `emit_instructions` by 479, and each runtime binary sheds 352
bytes of text. encodebench reads 98 more, livebench 70 more and oneshot 7
more, with no change to what they call. The front-end rows and every
allocation counter agree with the carrier. CI's rows replace these.

This change rides kanso#1839, the oversize-spare fix, rather than a pull
request of its own. On CI's rows the spare fix alone scores 91.0337007,
below the carrier's floor of 91.0337226, most of the gap in
`codegen_instructions_release`, which lands 109,925 above the carrier at
454,517,107. The adoption's retirement is projected onto those rows from the
deltas above, and the two together score 91.0338379. The browser tab compiles
with the same emitter, so browser_compile_instructions falls 7,415 to
390,029,358, measured here with the pinned rustc that builds
`docs/kanso.wasm`. With that row the floor this pull request banks is
91.0338442. Against the carrier, three rows still read worse:
codegen_instructions_release on 454,452,163, work_livebench on 1,474,871,601
and work_oneshot on 13,223,705, the release row from the spare fix's layout
and the two run rows 147 and 7 above the carrier. CI's rows for the pair
replace the projection.

CI's rows for the pair replace the projection. codegen_instructions_dev lands
on 118,937,518 and codegen_instructions_release on 454,509,840, the release row
57,677 above the projection. work_encodebench lands on 2,398,370,022,
work_livebench on 1,474,871,202, work_oneshot on 13,223,901 and work_runbench
on 1,152,728,743. runbench's text falls 32 bytes to 615,417, so `text` totals
4,824,318, and its emitted code is one call and two lines shorter than
projected; emitted_other_lines lands on 146,722, two below the carrier. Welfare
on these rows is 91.0338093, which is 0.0000867 above the
carrier's floor and 0.0000349 below the 91.0338442 banked from the projection.
The floor now records CI's figure. Banking the projection before CI had read
it was the error; a projected floor can only be wrong in a direction nobody
has checked.

## 2026-10-11 — the two sweeps print their own lists

Two specs checked CLAUDE.md's wording. `every_counter_gate_is_in_the_sweep`
read the word before "cost goldens," and `the_compile_sweep_names_every_compile_gate`
read one bullet for every compile gate's name. Each kept a fact in two places
and used a test to make them agree, which ties the instructions' prose to the
tree and blocks rewriting it (kanso#1846).

Both sweeps now take `--list`. `all_counters.sh --list` prints each vein and
its golden, and `all_compile.sh --list` prints each gate, both before building
anything. The two specs that read CLAUDE.md are replaced by
`the_sweep_lists_every_cost_golden` and `the_sweep_lists_every_compile_gate`,
which run the script with `--list` and compare its output with the files on
disk in both directions. Each went red with one entry dropped from the list it
reads, naming the golden or gate it had lost.

So the scripts are the single source for which veins and gates exist, and
CLAUDE.md no longer has to carry either list or a count.

## 2026-10-11 — the welfare verdict compares exactly, and kanso#1840's shape joins the run program

The welfare verdict passed any score within 0.001 of the floor, on either side.
Its comment said the margin was for compile peak bytes, which read 56 bytes
apart on linux and macos. That stopped applying when welfare began scoring the
committed goldens: both hosts read the same file. The script combines its terms
with `+`, `-`, `*` and `/` alone, and IEEE 754 rounds each of those exactly, so
two hosts score one tree to the same double. `math/round` appears only in the
history note `--set` writes. kanso#1820 fell 0.00099 and passed inside the
margin, which is what kanso#1845 reported.

The verdict now compares exactly. A score below the floor fails as a fall, and
a score above it fails as a rise nobody banked. `--set` writes the score through
`json/encode`, and a plain run reads it back to the same bits: a floor banked
on this branch held against its own tree with nothing to spare. Two specs in
`tests/a_welfare_fall_is_bought.rs` stage the floor a billionth of a point above
and below the recorded one. With the margin restored, both runs exited clean
and both specs went red.

`--set` still banks a fall smaller than 0.01. That path writes the new floor
and its reason into `bench/welfare_floor.json`, where a reviewer reads both,
and this change leaves it alone.

Main sat 0.0000081 below its floor when this was written, inside the margin.
kanso#1840 cost that much on the rows the objective reads, and what it bought
was in a shape the run program never ran. The corpus-first gavel of 2026-09-05
puts the shape into the corpus, measured on the compiler without the fix, and
the fix then scores against it. kanso#1840 had already merged, so the compiler
without it is main at kanso#1839, 30ffadf4, the tree kanso#1840 was cut from.

The new phase, `restate`, is the mem fixture
`an_interpolated_state_string_is_not_kept_under_the_mark` with its pass count
lifted into a parameter and its seed string cut from a megabyte to 64 KiB. At
128 passes, on kanso#1839's compiler, it raises the arena peak from 4,718,608
to 6,815,760, which is two more one-megabyte blocks and about one decoded
document, sized the way `machine` and `fold` were. It raises the program's
instructions from 1,152,728,396 to 1,166,079,984. Built alone, less the empty
program's 6,760,257, it costs 13,151,636, which is 1.13% of the program. The
header's table of port phases gains that row.

On main the arena peak stays at 4,718,608, and the program reads 1,170,030,652.
That is 3,950,668 more than kanso#1839's compiler spends on the same program,
so the fix costs instructions on this shape as well as saving memory. The
mechanism was not isolated.

Welfare's two run baselines are re-based the way the four port phases were, so
that adding the phase costs kanso#1839's compiler nothing. Each is multiplied
by the reading with the phase over the reading without it, both on kanso#1839's
compiler. run_instructions goes from 3,349,060,500 to 3,387,851,317, by
1,166,079,984 over 1,152,728,396. The run peak, which the objective reads as
the arena, held and permanent peaks summed, goes from 4,948,512 to 7,045,664
with the phase, so run_peak_bytes' baseline goes from 489,255,962 to
696,599,932. Both baselines are rounded up. The instruction pair was read in
this container. Both readings come from one compiler on one host, so the ratio
carries no difference between hosts that matters. The peak pair is counted and
is the same on every host.

The run vein moves by the phase's own work. run_alloc_bytes lands on
330,057,404, run_allocs on 1,771,562, run_beat_iters on 47,708,
run_bytes_malloc on 9,134, run_evac_allocs on 49,183, run_evac_bytes on
14,916,544, run_perm_allocs on 98, run_sh_rec on 1,508,784, run_sh_str on
17,048,976, run_str_scans on 1,023 and run_str_scan_bytes on 17,996,094. The
evacuated bytes are the string copied at each stage, which the mem fixture's
entry already names as the price of carrying it.

CI read the rest. work_runbench lands on 1,170,030,999, 347 above this
container's 1,170,030,652. The phase adds eight functions to the run program,
so its machine code grows to 618,633 bytes, which moves text to 4,828,574, and
its emitted code moves emitted_other_defines to 2,010, emitted_other_calls to
17,507, emitted_other_branches to 16,468 and emitted_other_lines to 147,279.
Welfare on CI's rows is 91.1114 against main's floor of 91.0338, and the floor
banks that.

`tests/the_run_program_carries_the_shapes_unchanged.rs` lists `restate` with
the four port phases that, like it, came from a fixture rather than from a
benchmark.

## 2026-10-11 — the interpreter row reads the same on every runner

kanso#1843 asked for `interp_instructions` to read one number on every CI host,
after two jobs at one source read 2,178,502,266 and 2,178,502,272 on
2026-09-15. Every cost-goldens job prints an `interp_sample` notice carrying
its CPU, the binary's hash and the row, so the question can be answered from
the annotations rather than by a new measurement.

The annotations of 144 jobs survive, from 2026-10-08 to this morning. They
hold 20 distinct rows, one for each interpreter source those jobs built. 19 of
the 20 were read on more than one CPU model, and none of them was read as two
numbers. The six models are AMD family 0x19 models 0x1 and 0x11, family 0x1a
model 0x2, and Intel family 0x6 models 0x6a, 0xad and 0xcf. 488,913,120 alone
was read by 45 jobs on all six.

Two pairs of rows sit close together, 488,869,908 against 488,869,940 and
568,708,684 against 568,709,639. Each value in both pairs was read on two or
more CPU models, and each step from one value to the next is a commit that
moved the golden: 4119f057 for the first pair and 3b981430 for the second. The
older jobs' annotations had expired, so the record starts eight days after the
2026-09-30 change that took the heap's address out of the interpreter's cache
keys.

So the row reads identically across runner hosts, which is what kanso#1843
asked for, and nothing needs excluding from the golden's header.

## 2026-10-11 — where the restate phase's extra instructions go

Main's compiler spends 3,950,668 more instructions on the run program with
`restate` than kanso#1839's compiler does. A per-function callgrind diff of the
two builds puts 3,870,494 of them in glibc's `memmove`, and every other
function moves by under 17,000. That is the stage evacuation copying each
pass's 64 KiB string below the loop's mark, which the 2026-10-10 entry on
record reuse names as the price of carrying it: run_evac_bytes reads
14,916,544 on main against 6,442,592 without the phase. Built alone, the phase
costs fewer instructions on main than on kanso#1839's compiler: the run program
with every other count set to zero reads 17,451,186 against 19,911,907. Why the
phase costs more in the full program and less alone was not looked into.

Handing the string's block down instead of copying it is not available: an
allocation gets a block of its own only above 1 MiB, so a 64 KiB string shares
a block with whatever else the stage allocated. Keeping survivors where they
were built would need a different region discipline, and nothing here builds
one.

## 2026-10-11 — the live log holds forty entries

CLAUDE.md says this file holds the last forty
entries, and nothing checked it. By 2026-10-10 the file held 526 entries in
25,301 lines. The pull request fixing kanso#1850
moved the older end by hand that day, and by the time this branch took main the
count had climbed back to 58.

`tests/the_live_log_holds_forty_entries.rs` now pins the count at exactly
forty, checks that no heading appears in both files, and checks that the
archive's last entry is no later than the live log's first.
`scripts/archive_log.sh` moves the oldest entries, unedited, from the top of
this file to the end of design/log/compiler-log-archive.md until forty remain.
A branch that appends an entry runs it in the same commit, and the spec's
failure message names the script and the first entry it will move.

The first run moved 18 entries. The two files together hold the same lines
before and after, compared as sorted multisets, and the line counts sum to the
same 102,212.

## 2026-10-11 — the specs stay one binary each

kanso#1847 asked whether folding the 228 spec files into a few test crates
would cut build and link time. Measured on main fd6167f7, release profile,
four cores: after `touch src/lib.rs`, `cargo test --release --no-run` takes
90.9 s, and the library alone is 35.5 s of it. The 230 test executables hold
487,797,788 bytes.

A prototype put every file in one crate as a `#[path]` module, with no spec
edited. It compiled, and the same rebuild took 68.5 s. Its one binary is
15,185,664 bytes. Run, it failed
`bind_chain_depth::the_measurement_sees_a_shape_that_nests`, which passes
alone. That spec reads its children's peak RSS from `getrusage`, and in one
process the peak includes every other spec's children. Separate binaries run
one after another, so no spec has shared a process with another file's spec
before. 151 of the 227 files spawn processes or read rusage.

Folding only the other 76 saves about 7 s and 160 MB. They would have to move
to a subdirectory to keep autotests, and other files name 29 of them by path,
including source comments, golden headers, two pages and four workflows. The
spec binaries already link with rust-lld, so the linker offers nothing either.
Neither fold is built, and the issue is closed as not planned.

## 2026-10-11 — the carry-repair spec had stopped reaching the repair

`tests/carry_repair.rs` was the regression test for kanso#823: a repair raised
a mark's pointer over the bytes it copied and left the mark's room behind. Its
program gathered four files of the repository's prose in a loop. With
`m->left = k_arena_left;` deleted from `k_repaired_settle`, the spec passed on
main, with those files and with fixed-size copies of them. A probe in
`k_repaired_settle` printed nothing: the program no longer reaches the repair.
Its check also lived in the rewind until kanso#1259 moved it to the push on
2026-09-05, where it reads the mark it has just written and cannot see a stale
one.

Three mem fixtures do reach the repair:
a_carried_value_written_into_an_older_node ten times,
a_repaired_node_below_the_mark_holds_tenure eight times and
a_node_repaired_by_an_inner_pop_outlives_the_outer_stage once. With the line
deleted, the last of them exits 1 and its stdout and counters change, so the
mem vein was already the real guard. The ratchet row `repaired_mark` now
deletes the line and expects that red, and the spec is gone.

## 2026-10-11 — gavel: work-ahead holds a pooled failure until the value is used

Clay ruled #1862, taking the recommendation: option 1 now, option 2 if
divergence turns out to matter.

The 2026-07-22 ruling made work-ahead invisible because pooled work has no
effects. That holds when the work succeeds. A pooled computation that fails or
never returns can show its failure before output the program as written
prints first: `bench/workahead/aswritten.kso` prints `delivery: made` after its
sleep and then runs `grind`, and a pooled `grind` that failed would raise its
error during the sleep, before that line.

**The ruling.** A pooled computation that fails stores its failure in the cell,
and the failure is raised at the cell's first use, where the program as
written would have raised it. For every program that terminates, the
transcript is identical to the one it would produce without work-ahead. A
pooled computation that never returns may still stall before output that the
program as written prints first. That case is accepted for v1, and a second
thread with its own arena (#1862's option 2) is the remedy if real programs
show it matters.

Owes, on #1856: a way to catch a runtime failure inside a pooled evaluation
and resume the scheduler, on every engine that pools (native `k_die` exits
today); a golden in which a pooled computation fails and the transcript
matches the program run without work-ahead, seen red with the failure raised
at the park; the page's work-ahead section saying what holds for a failure
and what holds for divergence.

## 2026-10-11 — gavel: a field may hold a list of `tie` references built from data

Clay ruled #1842, taking the recommendation as corrected on the issue.

The 2026-10-10 gavel "a field may hold a list of `tie` references" admitted a
list or map literal. A node whose links come from data cannot write a literal.
`list/map` is lazy (`pub fn map coll f` is `mapped f (iter coll)`), so
`list/map roads ref` in a field would store a record holding `ref`, and every
`ref` call would happen after `tie` had returned, where `resolve_tied` and
`k_tie_resolve` have no cell to swap.

**The ruling.** `list/to_list (list/map xs ref)`, and the same with
`(k -> ref k)`, written as a constructor's field, is admitted: every `ref` is
called inside the maker, and the field holds a list of cells the engines
already resolve. A bare `list/map` of references in that position is refused,
with a diagnostic naming `list/to_list`. The rule is unchanged: during making,
a reference is stored and never read.

Owes, on #1842: the check admits the forced form; an error golden for the bare
`list/map` with the new diagnostic; `a_graph_tied_from_a_map_of_edges` drops its
chain of `road` records; the comment in `lib/list/list.kso` where `tie` is
taught shows the data-sized form.

## 2026-10-11 — the big source files split by `include!`, not by module

kanso#1848 and kanso#1867 asked for codegen.rs, main.rs, eval.rs, check.rs and
lib.rs to be cut along their seams. The first cut moved codegen.rs's runtime
declarations into a submodule, `mod declares`. On CI that moved the browser
compile row by +60,813 instructions, the emit row by +873, and the compile and
entry rows by one each, and welfare fell by 0.000052. The release profile
builds sixteen codegen units and rustc partitions them by module, so a new
module changes which functions are compiled together and what gets inlined.
The 2026-08-25 gavel refuses a refactoring that costs welfare, however little.

`include!` keeps the moved items in the module they came from. Across
kanso#1873, kanso#1875, kanso#1878 and kanso#1879 the moved pieces are
codegen.rs's runtime declarations and dispatcher methods, main.rs's tail-call
rewrites, eval.rs's executor, builtins, values and run loop, check.rs's
`list/tie` checks, after-infer pass, arm and shadow checks, and lib.rs's
enumerable fusion, module path, imports and hoisting. Each file sits in a
directory named for the file that includes it. No golden moved, and CI's cost
rows on every head equal main's. eval.rs went from 6,742 lines to 3,950,
check.rs from 5,291 to 3,322 and lib.rs from 4,166 to 2,295.

Three things the move exposed: `cargo fmt` reaches only files named by `mod`,
so CI's format job now runs rustfmt on every file under a subdirectory of src/;
`scripts/diagnostic_coverage` listed `src/` alone and saw none of the
refusals that moved, until kanso#1875 made it descend; and four doc comments
sat above the wrong item, each left behind when a function was inserted
between a doc and the item it described.

**Declined: codegen.rs's call emission.** Moving `emit_call_full` through
`emit_call_rest`, 1,088 lines, into `src/codegen/calls.rs` left the native and
wasm instruction multisets identical to main's and moved
`browser_compile_instructions` from 390,029,358 to 390,029,410. Under the
2026-08-25 gavel it did not ship. The wasm module is 24 bytes larger. Cloud's
attribution is that panic locations embed the source path and a longer path
shifts the data after it, changing the alignment the metered `memcpy` and
`memcmp` loops see. That is unisolated. If it holds, the browser row is
reading a file's name, which the 2026-09-15 normalization rule says is
normalized or excluded. #1880 isolates it and, if it is confirmed,
normalizes the browser build before the split is retried.

## 2026-10-11 — STATUS.md and the ledger are deleted

This closes #1852. The 2026-10-10 gavel made GitHub issues the tracker and left
STATUS.md and design/pending-gavels.md as pointers until the two specs that
read them retired. Both have retired, and nothing else reads either file, so
both are deleted. Comments in four specs, two scripts and ci.yml still mention
them; those are cloud's and are cosmetic.

The ledger's Parked section was the one thing in it with no other home. It is
recorded here, unedited in substance:

- `<<` labels: walls cover staircases; revive on real DAG demand.
- Labeled nameless patterns: parked 2026-08-19, needing a fresh look against
  the post-24 language. Group headers stay behind it.
- dot-absorbs-`>>`: argued no, because it erases the visible then/bind split.
- Postfix index on `)`: `(sort xs)[1]` stays illegal; bind, then index.
- `;` as an inline separator: the borrow if inline groups are ever demanded.
- `&` as bitwise: orthogonal, someday.
- `serve` and processes: the executor-loop primitive and the next design
  campaign, where three investigations already end. The July reification form
  died with gavel 1, and the campaign starts from the three combinators.
- Hako tag-signing and checksum policy: parked in design/hako.md.
- Monorepo hakos: parked in design/hako.md.
- The survivor cap of four times the block threshold: the multiplier is a
  judgment call, and the principle is in the log.

A parked item becomes a `needs-clay` issue when a real case arrives.

## 2026-10-12 — a test that runs out of stack fails alone

`kanso test` ran its tests on the main thread, whose stack is 8 MiB. The
interpreter's depth limit is sized for the 1 GiB thread `run --interp` starts,
so a test that recursed too far overflowed Rust's stack before the limit
reported it, and the process aborted with every later test unreported. The lox
port met this from a recursion 700 calls deep. The runner now starts the same
deep thread, and the test fails with the interpreter's diagnostic, "the
program ran out of stack".

`tests/test_verb.rs` runs `deep_test.kso`, one shallow test and one that
recurses a million deep, and expects the first to pass, the second to fail and
the run to exit 1. The ratchet row "a test run on the main stack" gives the
thread 8 MiB and the spec sees the abort.

This is part of the recursion item on #1892. The native side of that item is
open: a deep enough recursion through the lox port's trie write still ends in
a segmentation fault with no message.

## 2026-10-12 — an operator answers what its arms answer

The ray tracer port's release build printed a few rows of its image and then
stopped with "no overload of `-` matches these arguments". The interpreter and
the dev build drew the whole image. Linking the release IR at `-O0` drew it
correctly, and so did the full release link with every `llvm.assume` removed.
Bisecting the 148 assumes left one: after reading a hit record's `p`, release
assumed the value carried the float tag.

The cause was in inference. Native's `emit_binop` and the interpreter send a
record on either side of an operator to the program's own arms for that
operator, and inference read `a + b` as arithmetic alone. It never widened the
arms' parameters and never added what they return. The ray tracer's `at` is
`origin + direction * t`, and `direction` is sometimes a float drawn from the
generator, so inference concluded that `at` answers a float, and so does the
`p` field that holds its result.

Inference now treats an operator with a record operand as a call to the
operator's group when the program declares one: the arms' parameters widen
with the operands, and the arms' answers join the operator's. The micro
fixture `an_operator_answers_what_its_arms_answer` is the smallest program
that showed it. Without the fix its release build prints a `v2` through the
number arm. The ratchet row "an operator answering arithmetic alone" removes
the fix and expects the micro corpus's release build to fail.

Inference does more work for each operator with a record operand, and every
compile row rose on CI: compile_instructions to 25,761,215 (+46,112),
entry_instructions to 85,207,562 (+162,685), library_instructions to
85,760,222 (+161,046), emit_instructions to 7,016,409 (+2,717) and
browser_compile_instructions to 391,974,060 (+1,944,702). The welfare floor
drops from 91.1114 to 91.1094 under the rule that a change making the engines
agree is not priced.

This is the ray tracer item on #1892.

## 2026-10-12 — a runtime diagnostic names the err it was handed

The inflate port destructured a value that was sometimes an err, and native
reported "the program ran out of stack" where the interpreter said "cannot
destructure err ... as `pair`". There was no recursion. `k_die_destructure`
rendered its value with `k_render`, which hands an err back unrendered so that
it can ride through a program, and then read the err as a string. The read
faulted, and the parent reports any segmentation fault as stack exhaustion.

The diagnostics that name a value they were handed now render through
`k_shown`, which renders an err in full: the destructure and keyed-read
refusals, the field-read refusals and `k_die_value` and `k_die_got`. The
interpreter's message is the one both engines print.

`tests/golden/runtime/an_err_cannot_be_destructured` is the port's shape: a
group that answers a record or an err, its answer bound with `pair a b = p`.
The ratchet row "a diagnostic handed an unrendered err" puts the plain render
back, and native's stderr reads out of stack again.

The first version of `k_shown` called `k_render_at` with the err flag set.
That gave `k_render_at` six callers outside itself where it had one,
`k_render`, and clang stopped inlining it there, so every print paid for a
call: basket rose 111,561 instructions and runbench 172,212 on this
container. `k_shown` now unwraps a thunk or a subtype itself, renders an err
through `k_render_whole` and hands anything else to `k_render`; it is cold and
out of line, so the functions that call it on their refusal path stay the size
they were. Measured the same way, basket is 14 instructions from main and
runbench 315.

The out-of-line `k_shown` is about 80 bytes of `.text` in every native
binary, and the rows that count a whole program carry it. On CI's rows,
`text` went from 4,828,574 to 4,829,694, `codegen_instructions_dev` to
118,944,914, `work_encodebench` to 2,398,369,910, `work_livebench` to
1,474,870,950 and `work_runbench` to 1,170,031,314. `work_oneshot` fell to
13,223,887.

## 2026-10-12 — a built binary names its own stack overflow

A recursion deeper than the stack ends in a fault on the guard page below it.
`kanso run` reports a child that died that way as having run out of stack, but
a binary run directly died with the bare signal and printed nothing. The lox
port is run directly, and a Lox recursion 20,000 calls deep ended in
"Segmentation fault" with exit 139 on this tree.

The runtime now installs a SIGSEGV handler at start-up, on a 64 KiB stack of
its own because the program's is the one that ran out. A fault whose address
lies between the top of the main stack and one MiB past its limit prints the
interpreter's diagnostic, "the program ran out of stack", and exits 1. Any
other fault restores the default handler and faults again, so a wild pointer
still dies by the signal. With no stack limit there is no floor to measure
from, and the handler is not installed. The wasm runtime has no signals and is
left alone.

`tests/a_built_binary_names_its_stack_overflow.rs` builds a recursion a
million deep that no engine can turn into a loop, runs the binary directly and
reads its stderr and exit status. The ratchet row "a binary that does not watch
its stack" removes the installation and the spec reads an empty stderr.

This closes the native half of the recursion item on #1892. The depth itself
still differs by engine: the interpreter stops at 10,000 calls and native at
whatever the stack holds.

Installing the handler costs every native program the same start-up work: a
`getrlimit`, a `sigaltstack` and a `sigaction`, about 3,300 instructions, and
368 bytes of `.text`. On CI's rows for the tree merged with #1899, `text`
went to 4,834,846 and `codegen_instructions_dev` to 119,002,829. The work rows
moved by that constant: `work_basket` to 30,876,488, `work_deepbench` to
393,579,644, `work_digestbench` to 5,603,118, `work_encodebench` to
2,398,373,226, `work_escapebench` to 39,049,158, `work_indexbench` to
2,462,486, `work_jsonbench` to 860,562,984, `work_livebench` to 1,474,875,071,
`work_oneshot` to 13,226,930, `work_pendbench` to 190,054,350,
`work_readbench` to 4,580,966, `work_runbench` to 1,170,035,080,
`work_scanbench` to 273,523 and `work_widebench` to 28,178,509.
`codegen_instructions_release` fell to 454,179,413, which outweighs the rest,
and the rise is banked.

## 2026-10-12 — a file names only what its module declares and its own imports bring

The second rule of the 2026-10-12 gavel on #1894. A module's files share
their declarations, and each file sees only the imports it writes. Before
this, the check enforced that only for qualified spellings: a file writing
`list/fold` without importing std/list was refused, and the same file writing
`fold` checked clean because a sibling's import had enrolled the bare name for
the whole module.

`compile_module_loaded` now hands each file of a several-file module the
build's names minus the ones it cannot see: a bare name surfaced only by
imports this file does not write, which no file of the module declares, and
a rename's spelling that only a sibling's import makes. The refusal names the
import that would bring the name. A module of one file sees exactly its
imports, so the work is skipped there.

Two book samples leaned on the old reach. The ch09 simulator's
`enumerable.kso`, in both `vse/` and `squeeze/`, called `fold` through
`methods.kso`'s import of std/list. Each now imports it, the ch09 panel shows
the line, and the paragraph before it says each file imports what it calls.
The vse repository holds the same file and gets the same line in vse#87. kq
and kanso-json have no file that leans on a sibling's import. ch07 states the
rule where it describes a module's files.

`tests/golden/errors_module/a_file_calls_a_name_only_its_sibling_imports`
pins the refusal: a two-file module where one file imports std/list and the
other calls `fold` on a list it built. Main checks it clean. The ratchet row
"a sibling's import seen from every file" hides nothing and turns the corpus
red.

The rest of #1894 is still owed: arms that travel with a value into a file
that never imported their owner, and the issue's reproduction, a test file's
import reaching the module it tests, which #1907 closes by keeping test files
out of the module.

CI's rows: `compile_instructions` 25,761,215 -> 25,881,551,
`entry_instructions` 85,207,562 -> 85,381,313 and `library_instructions`
85,760,222 -> 85,935,582, the cost of handing each file
of a module its own set of names. The browser compile fell 391,974,060 ->
391,311,502, emitting 7,016,409 -> 7,013,477 and the interpreter 488,792,115
-> 488,739,816. This change does not touch the code those rows run, so their
moves are put down to layout, though no run isolated it. The browser fall is
worth more than the three rises cost, and welfare banks the difference at
91.11. `compile_allocs` rose 68 to 14,598 and `interp_allocs` 68 to 587,351, and
the floor drops by what they cost, since this builds a ruled part of the
language.

## 2026-10-13 — a loop entered from the callee it calls rewinds

From #1892's bc port. A `for` loop in bc held every lap's allocation: 300,000
laps of `s += i` took 548 MB, and the binary counted 8 beat iterations for
20,000 laps. The port's interpreter has the shape every tree-walker has.
`loop_for` and `for_next` are one tail cycle, each lap runs the body through
`exec` with a plain call, and the `nested` arm of `exec` enters the cycle by a
tail call, because a loop inside the body is the same cycle again.

A cluster entered by a tail call gets its bracket by emitting the entering
edge as a plain call. Since #920 that demotion is refused when the cluster can
reach the caller again, because a caller the cycle re-enters with nothing held
would stack one frame per lap. The reach counted every mention, so the plain
call `r = exec body m` refused the entry. A plain call holds its own frame
while the callee runs, so the stack already grows with each level of nesting,
and demoting the entry adds one frame to each level and none to each lap. The
reach now follows what can enter the caller with no frame held: the heads of
tail calls, every name inside a lambda, and every name used as a value.

`tests/golden/mem/a_loop_entered_from_the_callee_it_calls_rewinds` is the
reduced program, 100,000 laps through a `nested` entry. Main counts no beat
and holds eleven arena blocks; with the change it counts 200,000 beat
iterations and holds one. No other mem golden moved. The bc port's 300,000
laps take 9 MB, and its 33 fixtures and 47 unit tests agree on all three
engines. The ratchet row "a cluster refused for a callee it reaches under a
frame" counts every mention again and turns the mem corpus red.

Two loops in the standard library gain a bracket the same way. std/json's
blank skip before a value, `value_for` and `value_blank`, is entered by a
tail call from `parse_value` and reaches it again only under the plain call
an array or object makes for each element. std/regexp's matcher cycle is in
the same position, and its groups lose the padding a wider tail group had
given them. Of the fourteen benchmarks only runbench and scanbench emit
different code. Measured here against main's compiler on the same host,
runbench ran 829 fewer instructions and scanbench 6 more, and every
allocation counter in the twelve cost veins is unchanged.
`a_cluster_the_entering_caller_is_reached_from_stays_grow_only` now hands the
re-entering function on as a value, which still refuses the entry, and
`a_cluster_reaching_its_caller_under_a_plain_call_is_a_beat` pins the plain
call.

CI's rows. The two benchmarks whose IR changed gain their brackets:
work_runbench lands on 1,170,034,251, 829 fewer, and work_scanbench on
273,515, eight fewer. The brackets are code: text lands on 4,840,798, up
5,952 (scanbench 3,888, runbench 2,064), emitted_other_calls on 17,531,
emitted_other_branches on 16,476 and emitted_other_lines on 147,327.
emit_instructions falls 522 to 7,012,955. The tab runs the same analysis,
which now walks each body for its frameless names:
browser_compile_instructions lands on 392,099,969, up 788,467, and
browser_compile_peak_bytes on 676,511, up 786.

Merged with main after #1904, the same rows land on `emitted_other_calls`
17,676, `emitted_other_branches` 16,559, `emitted_other_lines` 148,263 and
`browser_compile_instructions` 393,489,081.

## 2026-10-12 — concat reads the bytes text/bytes answers, on every engine

From #1891, the third item: the 2026-08-23 gavel says a list is never bytes,
and what `concat` declares decides which engine is right. Appendix b declares
`text/bytes string → int[]` and `text/concat` as appending two lists, so a
value `text/bytes` answered is a list to `concat`. Native code already read a
bytes value as its byte values on either side and answered a list. The
interpreter matched only two `List`s and refused anything else with "concat
takes two lists", which is what the httpd, mustache, chip8 and ugit ports hit.
The interpreter now reads a `Bytes` operand as its byte values, and the answer
is a list on both engines. The gavel's other direction is unchanged: a list is
still never read as bytes.

`tests/golden/micro/concat_reads_the_bytes_text_bytes_answers` puts bytes on
the left, on the right, on both sides, and as a slice of `text/bytes`. Main's
interpreter stopped at the first line.

Appendix b also says `concat` takes strings, and neither engine does: both
refuse two strings with the same message. #1906 asks which way that goes.

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
