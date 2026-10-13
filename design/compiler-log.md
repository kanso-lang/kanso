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

91.11.

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
