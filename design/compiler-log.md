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




## 2026-10-25 — a slice that starts before the value answers none

From #2088's last item, bc's port. Under the 2026-10-13 gavel on #1890, a
`text/slice` whose range runs past the end of the value, or starts past it,
answers `none`. A range that started before position 1 answered the empty
result on every engine: `text/slice "abc" 0 2` was `""`, and so was a start
of -1, where `"abc"[0]` answers `none`. bc's journal called the edges
inconsistent. A shortened answer at the front is the silent loss the gavel
ruled against at the back, so a start before 1 now answers `none` too, and
`slice!` refuses it. The empty range at the start, `1 0`, is inside the value
and still answers empty. `text/to_span` already answered `none` for a start
before 1, so a span and a slice now agree on where a range may sit.

The native test is one unsigned compare, `from - 1 > len`, which a start
below 1 fails by wrapping to the top of the range. It replaces `from > len +
1` in `k_b_slice` and `k_b_slice_raw`, so the hot path gains no instruction.
The cheaper test made LLVM inline `k_b_slice` into the four fused doors'
fallbacks, and CI read every binary 7,216 bytes larger for code that runs
only on a bad range, so `k_b_slice` is marked out of line. On this container
indexbench's text is 64 bytes smaller than before the change, and the run
program 961 instructions lighter.

CI's rows, against #2106's. The library's `past?` reads one more comparison
and the interpreter's builtin one more test, so the front end and the
interpreter move a little: compile_instructions 24,664,014 -> 24,664,426,
entry_instructions 72,772,292 -> 72,794,039, library_instructions 72,435,518
-> 72,459,948, browser_compile_instructions 218,703,716 -> 218,730,086,
interp_instructions 385,420,176 -> 385,422,816 and browser_run_instructions
33,345,263 -> 33,347,903. Programs that slice emit the extra comparison too:
the run program's emitted_other_branches 17,310 -> 17,312 and
emitted_other_lines 163,950 -> 163,955. Natively every binary's machine code
is 64 bytes smaller, text 5,523,025 -> 5,522,129 over the fourteen, and
building it costs codegen_instructions_dev 116,456,350 -> 116,455,729 and
codegen_instructions_release 370,087,521 -> 370,086,817. The run rows move by
under 0.02% each: work_runbench 797,623,329 -> 797,622,606, work_encodebench
2,327,445,961 -> 2,327,403,174, work_widebench 28,445,165 -> 28,413,165,
work_livebench 936,030,663 -> 936,030,838, work_oneshot 10,050,755 ->
10,050,461 and work_digestbench 4,122,953 -> 4,122,823. No allocation counter
moves. Welfare falls by less than a hundredth, and the floor follows: this
builds the 2026-10-13 ruling on #1890 at the start of the range, which #2088
asked for.
The interpreter reads a start no machine word holds as 0, as before, and 0 is
now outside. `past?`, which the compiler asks before reading a range that
`slice!` hands straight to a door, adds `from < 1`. std's own `slice_in`
calls all start at 1 or above.

`micro/a_slice_that_starts_before_the_value_answers_none` covers text, wide
text, a list and bytes from 0, a start of -1, the empty range at the start,
and `slice!` from 0. On main every line prints `[]` or `<>`.

Five micro goldens had pinned the old edge, and each line that moves is a
start below 1 going from the empty result to `none`: `slice_at_its_edges`,
`utf8_of_a_byte_slice_at_its_edges`, `an_append_of_a_slice_reads_the_range_in_place`
(`below one` and `both zero`), `a_number_read_from_a_slice_reads_the_range_in_place`
(`int below one` and `float below one`, which read the empty string and
answered an err) and `a_slice_bang_chain_opens_its_door_in_place`, where
`slice! "kanso" 0 5` now fails inside its box. The interpreter, the native
build and the wasm translator agree on all five.

## 2026-10-25 — a list handed to put is refused wherever the list is certain

#2091's fifth item, the `put` half. `put` writes a key of a map, and on a
list it stops at run time on every engine with "put takes a map, a key, and
a value". The literal check refused `put [1 2 3] 2 9`, because the list is
written in place, and passed `xs = [1 2 3]` followed by `put xs 2 9`. diff's
and inflate's ports met the second shape. #2086 asks whether `put` should
write a list at an index, and says this inconsistency is cloud's to fix
whatever it rules.

The after-inference walk now refuses a list handed to `put`, `delete`,
`keys`, `values` or `entries` when the list is certain: a name the same
declaration binds once to a list literal, or a call of a group whose every
arm answers a list and nothing else. A parameter, or a name bound to
anything less certain, is left to the run, as the walk's other kind checks
leave theirs. The message is the literal check's: "`put` takes a map here,
not a list". The callee's name is tested first, by its last byte and then
the word where it ends, and the declaration's list bindings are read only
after a call of `put` or its kin turns up. The first version split every
callee's name on `/` and collected every declaration's list bindings up
front, which cost the front end 130,307 instructions on compile_instructions;
this one costs 24,741.

`errors/a_list_handed_to_put` holds both shapes. Main checks it as ok. No
program in the tree, the book's samples or kq is refused. One run-time
golden met the new check: `runtime/delete_takes_a_map_at_run_time` handed
`delete` a list through an identity function, which inference sees answer
nothing but a list in that program. It hands an int now, and its stderr is
unchanged.

CI's rows, against #2107's: compile_instructions 24,664,426 -> 24,689,167,
entry_instructions 72,794,039 -> 72,851,547, library_instructions
72,459,948 -> 72,517,015 and browser_compile_instructions 218,730,086 ->
218,996,814. No run row, allocation counter or mem golden moves. Welfare
falls by less than a hundredth and the floor follows: check now refuses what
every engine refuses at run time, which #2086 named a checker bug whatever
it rules.

## 2026-10-25 — two cliffs join the run program and the interpreted corpus

Profiling the run program after #2106 turned up two shapes no corpus held.
Both are quadratic, and each is measured here on main before its fix, under
the 2026-09-05 corpus-first gavel.

`branched` is a list held in a record and pushed onto from one arm of an
`if` whose other arm answers the record whole:
`if (k == 0) b (fill (bag (push b.items k) (b.n + 1)) (k - 1))`. kanso#1926
moves the list out of the record when every mention of the record is a
field read, and #2106 let a guard's early answer off that rule. The `if`
arm still counted, so every push copied the list, on both engines: 80,000
pushes took 14 seconds natively and 30 on the interpreter, where the
guard's spelling and the two-arm spelling took a tenth of a second.

`padded` lays out a column with `text/pad_start`. The library grew the
padding one copy of the fill at a time and counted its characters on every
step, so a width of 40,000 allocated 801 MB natively.

In the run program `branched` has a knee like `guarded`'s: linear at about
260 instructions a push below 22,290 pushes, and about a million and a half
a push past 22,310. The count is 22,305. `padded` pads 75 rows to widths of
37 to 2,775 characters. Against #2106's tree with both phases,
880,945,062, the columns are 40,556,036 (4.60%) and 42,367,338 (4.81%).

The interpreted corpus runs both too, at 680 pushes and 18 rows: 392,521,332
with the code and neither run, 425,717,899 with both. The tab runs the same
corpus, so its run row moves with it.

CI's rows for the corpus with both shapes. work_runbench 797,623,329 ->
880,946,052. The run program's emitted code grows by the two phases,
runbench defines 771 -> 781, calls 7,818 -> 7,956, branches 6,791 -> 6,886,
lines 60,113 -> 60,916, and its machine code 779,308 -> 785,212 bytes. Over
every benchmark that is emitted_other_defines 2,178 -> 2,188,
emitted_other_calls 19,778 -> 19,916, emitted_other_branches 17,310 ->
17,405, emitted_other_lines 163,950 -> 164,753 and text 5,523,025 ->
5,528,929. Its
allocation vein: run_allocs 1,444,950 -> 1,541,511, run_alloc_bytes
213,685,416 -> 290,828,216, run_arena_blocks 4 -> 5, run_arena_peak_bytes
4,718,608 -> 5,767,200, run_spare_peak_bytes 2,621,456 -> 3,670,048,
run_cohort_frees 5 -> 68, run_perm_allocs 101 -> 102, run_beat_iters 89,113
-> 111,493, run_evac_allocs 8,626 -> 8,970, run_evac_bytes 4,822,160 ->
13,592,736, run_str_scans 1,023 -> 53,763, run_str_scan_bytes 17,930,494 ->
67,106,410, run_sh_str 16,193,568 -> 66,689,376, run_sh_rec 3,906,544 ->
5,268,656 and run_sh_buf 86,615,088 -> 111,082,496. The interpreted corpus:
interp_instructions 385,420,176 -> 425,717,899, interp_allocs 401,614 ->
445,515 and interp_peak_bytes 637,429 -> 735,045. The tab compiles and runs
the larger corpus: browser_compile_instructions 218,703,716 -> 239,148,990,
browser_compile_peak_bytes 532,354 -> 610,900 and browser_run_instructions
33,345,263 -> 38,837,644.

Each welfare term these move keeps its ratio to its baseline, as the four
#2089 phases did: run_instructions, run_peak_bytes, interp_instructions,
interp_peak_bytes, browser_compile_instructions, browser_compile_peak_bytes
and browser_run_instructions. The floor's history records each old and new
baseline. The fixes score against them.

## 2026-10-25 — three quadratics in the core fixed

The fixes for the two shapes the entry above added to the corpus, and one
for a map taking keys in no order, which the run program's `scatter` phase
has held since kanso#1820.

**A list in a record grows in place from an `if` arm.** kanso#1926 moves a
list out of a record when every mention of the record is a field read, and
#2106 let a guard's early answer off that rule because it runs instead of
the push. An arm of `if c a b` runs instead of the other arm in the same
way, so `FieldTally` now tallies each arm on its own and lets an arm that
never reads the field keep only its refusal. 80,000 pushes written with the
`if` went from 14.2 seconds to 0.068 natively and from 30.3 to 0.11 on the
interpreter. `mem/a_list_in_a_record_grows_from_an_if_arm` pins it: 20,000
pushes made 51,814 allocations of 9,160,000,528 bytes on main and make 13 of
1,044,944.

**Padding doubles its fill.** `text/pad_start` and `text/pad_end` grew
their padding one copy of the fill at a time and counted its characters on
every step. The fill now doubles until it covers the width and the front of
it is kept, which answers the same string. A width of 300,000 takes 0.06
seconds. `mem/a_wide_padding_doubles_its_fill` pins a width of 40,000 with
a two-character fill: 20,005 allocations of 400,600,128 bytes on main, 21
of 211,696 now.

**Two int keys compare directly, and a map's side run grows longer.**
`k_key_cmp` unwrapped subtypes and tested for a bignum before it compared
two machine ints, and every probe of a view search and every step of a
merge asks it. The side run that takes keys out of order merged into the
view once it passed the square root of the view's length. An insert into
the side is one memmove and a merge walks both runs, so the side now waits
for four times the square root. 2,340 scrambled int keys went from
7,703,440 instructions to 6,039,707 with the compare alone and to 4,575,844
with both; eight times the square root read 4,631,595. The longer side
holds more: `a_tally_of_scattered_keys_merges_its_view` reads
held_peak_bytes 273,328 where it read 206,448.

On this container the run program goes from 880,945,062 with the two new
phases to 799,175,414 with the three fixes, and the interpreted corpus from
425,717,899 to 399,112,446.

CI's rows, against #2111's tree with #2107 and #2108 under it. The run
program falls 9.3%, work_runbench 880,946,052 -> 799,174,946, and the two
phases cost what their neighbours do. The interpreted corpus falls 6.2%,
interp_instructions 425,717,899 -> 399,115,104, interp_allocs 445,515 ->
420,622 and interp_peak_bytes 735,045 -> 734,933, and the tab running it
11.2%, browser_run_instructions 38,837,644 -> 34,483,426. Of the other run
rows work_basket falls 31,039,077 -> 30,619,856 on the int compare, and
work_oneshot 10,050,755 -> 10,098,604, work_livebench 936,030,663 ->
936,077,833 and work_encodebench 2,327,445,961 -> 2,327,451,296 rise by
layout. Over every benchmark the emitted code lands on
emitted_other_calls 19,914, emitted_other_branches 17,407 and
emitted_other_lines 164,739, the two phases' code less what the arm rule
lets the emitter drop. Three map fixtures keep a longer side and allocate
for it: growing_map_allocs 1,615 -> 1,617, growing_map_alloc_bytes 189,312
-> 200,608, growing_map_held_peak_bytes 55,408 -> 63,536,
a_tally_of_scattered_keys_merges_its_view_allocs 29 -> 31,
a_tally_of_scattered_keys_merges_its_view_alloc_bytes 536,368 -> 620,752,
a_tally_of_scattered_keys_merges_its_view_held_peak_bytes 206,448 ->
273,328, fused_tally_allocs 66 -> 68, fused_tally_alloc_bytes 33,872 ->
36,192 and fused_tally_held_peak_bytes 7,648 -> 8,464. #2106's fixture
does the same: a_read_handed_to_a_call_stays_in_place_allocs 15 -> 17,
a_read_handed_to_a_call_stays_in_place_alloc_bytes 16,800 -> 19,904 and
a_read_handed_to_a_call_stays_in_place_held_peak_bytes 6,112 -> 7,184.

The run program's allocation vein: run_allocs 1,541,511 -> 1,445,901,
run_alloc_bytes 290,828,216 -> 215,212,648, run_arena_blocks 5 -> 4,
run_arena_peak_bytes 5,767,200 -> 4,718,608, run_spare_peak_bytes 3,670,048
-> 2,621,456, run_cohort_frees 68 -> 5, run_evac_allocs 8,970 -> 8,626,
run_evac_bytes 13,592,736 -> 4,822,160, run_str_scans 53,763 -> 1,885,
run_str_scan_bytes 67,106,410 -> 18,352,322, run_sh_str 66,689,376 ->
16,642,592, run_sh_rec 5,268,656 -> 3,906,672 and run_sh_buf 111,082,496 ->
86,615,200. run_push_mut_slow rises 1,557,480 -> 1,579,785, by exactly the
22,305 pushes `branched` now makes in place, and run_bytes_malloc 7,885 ->
7,893 and run_bytes_freed 7,755 -> 7,763 with them. The list it grows lives
in a permanent buffer, so run_perm_peak_bytes rises 212,465 -> 736,769, and
the longer side run holds more, run_held_peak_bytes 233,760 -> 254,240 and
run_view_allocs and run_view_frees 399 -> 403. The run program's peak still
falls, from 6,213,425 bytes to 5,709,617.

The front end reads the doubled padding in the library it includes and the
new arm rule: compile_instructions lands on 24,695,561, entry_instructions
72,875,969, library_instructions 72,547,701 and browser_compile_instructions
239,691,017, with browser_compile_peak_bytes 610,760, and
codegen_instructions_dev on 116,456,086 and codegen_instructions_release on
370,239,646. Welfare rises 0.32, which this pull request banks.

## 2026-10-25 — a word count joins the run program and the interpreted corpus

A probe built each list, text and map function natively at two sizes, four
times apart, and counted the instructions of each. Every one grew linearly
or as n log n except `list/tally` over keys that come round more than once:
4× the items cost 15× the instructions, with string keys and with int keys
alike. Tallying 20,000 ints, each seen twice, took 1.19 billion
instructions natively, and 80,000 took 1.49 s against 0.11 s for 20,000.
The interpreter keeps its maps in a B-tree and is linear on the same
program, 0.15 s and 0.44 s. Under the 2026-09-05 corpus-first gavel the
shape joins the corpus here, measured on main at kanso#2112 before its fix.

`counted` is a word count, `list/tally` over 2,000 words that each come
round twice in no order. The tally reads a word's count and puts it back one
higher. When the map already holds the key, the in-place put sets the
value in the map's sorted view and then walks the map's append log back
from its end to set the same key's entry there, so a word's second sighting
costs the number of words already counted.

On this container the run program reads 843,728,377 with the phase and
799,281,633 without it, a column of 44,446,744 (5.27%). The interpreted
corpus counts 1,000 words, 440,075,578 → 462,532,645. That corpus is the
tab's program too, and the tab runs the native runtime built for wasm32,
where the walk is the same.

CI's rows for the corpus with the phase. work_runbench 799,174,946 ->
843,623,597, a rise of 44,448,651. The interpreted corpus reads
interp_instructions 399,115,104 -> 424,511,367, interp_peak_bytes 734,933 ->
741,380 and interp_allocs 420,622 -> 457,070. The tab compiles and runs the
same corpus: browser_compile_instructions 239,691,017 -> 263,967,351,
browser_compile_peak_bytes 610,760 -> 699,180, and browser_run_instructions
34,483,426 -> 60,166,290, where 1,000 words cost three quarters as much again
as the rest of the corpus because the tab's runtime walks the log too. The
run program's emitted code grows by the phase: emitted_other_defines 2,188 ->
2,196, emitted_other_calls 19,914 -> 19,957, emitted_other_branches 17,407 ->
17,435, emitted_other_lines 164,739 -> 165,000 and text 5,492,433 ->
5,496,337. Its allocation vein: run_allocs 1,445,901 -> 1,449,911,
run_alloc_bytes 215,212,648 -> 215,555,576, run_perm_allocs 102 -> 103,
run_push_mut_slow 1,579,785 -> 1,579,791, run_put_mut_grow 22 -> 30,
run_sh_buf 86,615,200 -> 86,746,592, run_sh_rec 3,906,672 -> 3,906,784 and
run_sh_str 16,642,592 -> 16,769,952. Welfare re-bases the six terms the phase
moves, each keeping its ratio to its baseline, and the floor stays at 94.33.

## 2026-10-25 — an overwrite leaves the log a duplicate

A native map keeps an append log of every put and, once something reads it,
a sorted view holding each key's last value. An in-place put of a key the
view already held set the view's slot and then walked the log back from its
end to set that key's entry too, so the second sighting of a word in a word
count cost the number of words counted so far. #2115 put the shape in the
corpus.

The put now sets the view's slot and appends the pair to the log, which is
the state a persistent put already leaves: the log may hold a key more than
once, and the view decides. A log that has to grow starts again from the
view. Copied in order it holds each key once, sorted, so it becomes the
map's view and the old view is freed; overwrites after that are set in the
log itself. The restart takes room for as many keys again, because a key
arriving out of order gives the map a separate view back and the overwrites
after it append, so the next restart is that many puts away. Without that
room a restart could follow on the very next put.

Two smaller changes came out of measuring it. #2112's compare tested for
two machine ints before anything else, and kq's keys are strings, so every
compare in kq paid the test. Keys of one kind now compare directly, two
ints or two strings. On this container kq's print_small reads 24,631,248,
against 24,695,778 on main and 24,648,174 on main with the int test taken
out. A search of
the view tests an int key's kind once rather than at every probe, which
took the scattered-key tally under main again after the restart had moved
`k_map_find` out of line.

Probes at two sizes, natively, instructions:
- `list/tally` over 20,000 ints each seen twice: 1,193,001,970 → 42,779,901.
- The same over 20,000 strings: 3,829,711,628 → 76,451,701.
- 5,000 strings over 500 keys: 93,576,007 → 9,291,779.

The mem vein moves where maps are overwritten, because a log now keeps a
duplicate until it restarts and a restart takes room. tally_shape_put_mut_grow
3 -> 5, tally_shape_allocs 19 -> 21, tally_shape_alloc_bytes 133,280 ->
139,456 and tally_shape_sh_buf 2,128 -> 8,304. fused_tally_put_mut_grow 5 ->
6, fused_tally_allocs 68 -> 69, fused_tally_alloc_bytes 36,192 -> 44,400 and
fused_tally_sh_buf 9,568 -> 17,776. readwrite_map_put_mut_grow 1 -> 3,
readwrite_map_allocs 308 -> 310, readwrite_map_alloc_bytes 11,056 -> 12,624
and readwrite_map_sh_buf 448 -> 2,016. repeated_key_shape_put_mut_grow 4 ->
6, repeated_key_shape_allocs 7,387 -> 7,389, repeated_key_shape_alloc_bytes
248,160 -> 260,480 and repeated_key_shape_sh_buf 4,080 -> 16,400.
a_read_handed_to_a_call_stays_in_place takes 100 keys in no order while it
overwrites them, so a key out of order keeps handing the map a separate view
back: a_read_handed_to_a_call_stays_in_place_put_mut_grow 5 -> 8,
a_read_handed_to_a_call_stays_in_place_allocs 17 -> 26,
a_read_handed_to_a_call_stays_in_place_alloc_bytes 19,904 -> 50,544 and
a_read_handed_to_a_call_stays_in_place_sh_buf 8,192 -> 24,336. It now frees
every view it allocates, 17 of 17 where main freed 7 of 9, and its
held_peak_bytes falls 7,184 -> 6,112. Every fixture that moved runs in fewer
instructions than on main.

A differential over 120 generated programs builds maps by in-place puts in
four key orders, with ints and strings and from one key to every key
distinct. It reads each map back by length, keys, values, a persistent put,
a delete and equality, on the interpreter and on both native tiers: 0 of
120 disagree. With the restart copying from the log instead of the view,
30 of 120 disagree.

CI's rows. work_runbench 843,623,597 -> 801,847,883 (-5.0%), work_basket
30,619,856 -> 18,214,855 (-40.5%), and browser_run_instructions 60,166,290
-> 41,265,571 (-31.4%), since the tab runs the same runtime. run_peak_bytes
falls 5,709,617 -> 5,671,457. The run program's allocation vein:
run_evac_allocs 8,626 -> 1,760, run_held_peak_bytes 254,240 -> 216,080,
run_put_mut_grow 30 -> 32, run_sh_buf 86,746,592 -> 86,829,600,
run_buf_reuse 150 -> 149 and run_carry_dedup 1 -> 0. basket's:
basket_put_mut_grow 18 -> 24, basket_allocs 27,270 -> 27,272,
basket_alloc_bytes 6,162,417 -> 6,176,273 and basket_sh_buf 689,808 ->
703,664, with basket_held_peak_bytes 71,136 -> 64,016. The runtime's machine
code grows by 7,728 bytes in every benchmark binary, text 5,496,337 ->
5,604,529, from the compare and the search inlined where they are called,
and building that code costs codegen_instructions_dev 116,456,086 ->
116,458,530 and codegen_instructions_release 370,239,646 -> 370,260,066.
Welfare rises 94.33 -> 94.49 and the rise is banked.

## 2026-10-25 — a drained map and an admitting set join the corpus

The probe that found the word count timed six more accumulation shapes at
two sizes: a string joined onto itself, a list concatenated onto, a map of
maps, a record carrying a list through a fold, a map drained by `delete`,
and a set kept as a map that admits a key only when it does not hold it.
The first four grow linearly. The last two do not, on either engine.

`delete` answers a fresh map (ruled 2026-10-13, kanso#1914), so each call
copies the whole map, and a loop that drains a map costs the square of its
size. Taking half the keys out of a map of 5,000 in no order took 1.26
billion instructions natively and 0.21 s on the interpreter; of 20,000,
27.9 billion and 3.76 s.

The admitting set is `if (m[k] == none) (put m k true) m`. The linearity
analysis counts the read in the condition as a second holder of the map, so
the put copies it. Natively the copy has no sorted view, and the next read
sorts the whole map again. Admitting 5,000 keys took 9.39 billion
instructions natively and 0.30 s on the interpreter; 20,000 took 178
billion and 5.1 s. The guard spelling, `return m if m[k] != none` above
`put m k true`, is refused the same way.

Under the 2026-09-05 corpus-first gavel both shapes join the corpus here,
measured on #2116's tree before their fixes. `drained` deletes 550 keys
from a map of 1,100 and `admitted` admits 430 keys. On this container the
run program reads 894,638,261 with both. Without `drained` it reads
848,089,560, a column of 46,548,701 (5.20%), and without `admitted`
847,672,459, a column of 46,965,802 (5.25%). The interpreted corpus drains
600 keys and admits 500, and reads 509,290,842 with both. The tab runs the
same corpus.

CI's rows for the corpus with both phases. work_runbench 801,847,883 ->
894,533,250 and run_peak_bytes 5,671,457 -> 6,780,801. The interpreted
corpus reads interp_instructions 424,511,367 -> 471,505,162,
interp_peak_bytes 741,380 -> 767,647 and interp_allocs 457,070 -> 499,002.
The tab compiles and runs it: browser_compile_instructions 263,967,351 ->
272,981,946, browser_compile_peak_bytes 699,180 -> 715,283,
browser_run_instructions 41,265,571 -> 456,882,925 and
browser_run_peak_bytes 1,376,256 -> 2,162,688. The tab's run grows elevenfold.
Its runtime is the native one built for wasm32, so both shapes reach it,
and the corpus runs them at 600 and 500 keys; which of the two takes the
most there is not measured here. The run program's emitted code grows by the two phases:
emitted_other_defines 2,196 -> 2,204, emitted_other_calls 19,957 -> 20,078,
emitted_other_branches 17,435 -> 17,572, emitted_other_lines 165,000 ->
165,915 and text 5,604,529 -> 5,606,161. Its allocation vein: run_allocs
1,446,913 -> 1,449,888, run_alloc_bytes 215,551,096 -> 248,821,464,
run_arena_blocks 4 -> 5, run_arena_peak_bytes 4,718,608 -> 5,767,184,
run_spare_peak_bytes 2,621,456 -> 3,670,032, run_held_peak_bytes 216,080 ->
276,848, run_beat_iters 111,493 -> 112,473, run_evac_allocs 1,760 -> 1,978,
run_evac_bytes 4,603,072 -> 7,175,760, run_put_mut_grow 32 -> 41, run_sh_buf
86,829,600 -> 116,309,488 and run_sh_map 12,160 -> 25,920. Welfare re-bases
the eight terms the phases move, each keeping its ratio to its baseline, and
the floor stays at 94.49.

## 2026-10-25 — a delete takes its key out in place, and a condition's read holds nothing

Fixes for the two shapes #2117 put in the corpus.

**The admitting set.** The linearity analysis counts how often a function
uses a value on its busiest path, and a put is made in place when the map
it writes is used once. `count_in_expr` already counted only the larger arm
of an `if` or a guard, but it counted every mention in the condition, so
`if (m[k] == none) (put m k true) m` used `m` twice. The condition is
finished before either arm runs, and a read `m[k]` whose key calls and
compares nothing answers the element and lets the map go. Those reads no
longer count, the same discount the read bound to a name already had
(`read_in_block`). The guard spelling, `return m if m[k] != none` above
`put m k true`, is proven by the same change. Admitting 20,000 keys took
177,856,097,875 instructions natively and now takes 61,905,716. The
interpreter took 5.1 s and now takes 0.02 s.

**The drained map.** `delete` joins `push` and the byte appends as a
builtin that writes into its first argument where nothing else holds it.
At such a site the interpreter removes the key from its tree, and the
native engine calls `k_b_delete_mut`. That makes the map's pairs its sorted
view, restarting them from the view as an overwrite's growth does, and then
closes the key's slot by moving the pairs after it down one. A delete still
answers a fresh map wherever the map has another holder, as the 2026-10-13
ruling on #1914 requires; that copy now finds the key by a search and moves
the pairs either side of it in two copies, where it compared every key.
Draining half of 20,000 keys took 27.9 billion instructions natively and
1.53 s; it now takes 2.42 billion and 0.066 s. The interpreter took 3.76 s
and now takes 0.03 s.

The native drain is still quadratic, because closing a slot in a sorted
array moves the pairs after it: 80,000 keys drain in 0.95 s. glibc moves
large blocks with `rep movsb`, which callgrind counts a byte at a time, so
the instruction count overstates the time by more than the wall clock
does. A view that marked deleted slots and skipped them would make each
delete a search, but every reader of the view would have to skip the marks.
That is #2118.

CI's rows, against #2117's corpus: the run program 894,533,250 -> 809,823,849
(-9.5%), the interpreted corpus 471,505,162 -> 430,978,258 (-8.6%), and the
tab's run 456,882,925 -> 42,417,870, which puts it near where it stood
before #2117 added the two phases. Against main, which holds neither the
phases nor the fix: work_runbench 801,847,883 -> 809,823,849,
interp_instructions 424,511,367 -> 430,978,258, interp_peak_bytes 741,380 ->
765,587, interp_allocs 457,070 -> 461,292, browser_run_instructions
41,265,571 -> 42,417,870, browser_run_peak_bytes 1,376,256 -> 1,310,720 and
browser_compile_instructions 263,967,351 -> 273,807,879, each the two phases
less what the fix takes back. The run program's allocation vein against
main: run_allocs 1,446,913 -> 1,446,946, run_alloc_bytes 215,551,096 ->
215,868,632, run_sh_buf 86,829,600 -> 86,960,912, run_put_mut_grow 32 -> 48,
run_beat_iters 111,493 -> 112,473, run_bytes_malloc 7,893 -> 7,901,
run_perm_live_bytes 15,825 -> 32,049 and run_perm_peak_bytes 736,769 ->
802,321. Both restarts, the delete's and the put's, take their pairs from
outside the arena when the map outlives the loop it changes in, and which of
the two phases accounts for the perm rows is not isolated here. The runtime's
machine code grows by `k_b_delete_mut` and the searching persistent delete,
2,848 bytes in every benchmark binary: text 5,604,529 -> 5,646,113,
emitted_other_lines 165,000 -> 165,914 with the phases' own code, and
work_oneshot 10,033,444 -> 10,034,004 from the layout that moves with it. The
front end's condition discount and the new builtin cost entry_instructions
72,875,969 -> 72,875,974, emit_instructions 5,628,759 -> 5,636,850 and
codegen_instructions_dev 116,458,530 -> 116,477,304, where
codegen_instructions_release falls 370,260,066 -> 370,206,363. Welfare rises
94.49 -> 94.83 and the rise is banked.

The interpreter's in-place delete takes a value on the holder count as push
and put do, so the ratchet's row "a shared seed copied every lap" now finds
five such sites in `src/eval/builtins.rs` where it found four, and disables
the delete's with the others. Its spec still turns red under it.

## 2026-10-25 — a deduped list joins the corpus

The same probe timed three more record and map shapes. A map of lists grown
by pushing onto `m[k]` scales linearly. A set kept in a record beside the
list of what it admitted does not: `if (st.seen[x] == none) (words (put
st.seen x true) (push st.out x)) st`, the distinct words of a text in the
order they first appear. 5,000 words took 2.22 billion instructions
natively and 20,000 took 44.7 billion; the interpreter took 0.13 s and
1.52 s. Two counts refused it. The read of the set in the condition counted
as a second read of the field the put moves, and the arm that hands the
record back unchanged counted the busier arm's two field reads against it,
so neither the record nor its fields were handed over and the set and the
list were copied for every word. A map of maps counted in place is the third
shape, and it is quadratic natively only; it waits for its own corpus
phase.

Under the 2026-09-05 corpus-first gavel `deduped` joins the corpus here,
measured on #2119's tree before its fix. It keeps 850 words once each. On
this container the run program reads 855,640,794 with it and 809,929,455
without it, a column of 45,711,339 (5.34%). The interpreted corpus keeps 800
words, 485,629,040 → 511,418,942.

CI's rows for the corpus with the phase. work_runbench 809,823,849 ->
855,535,787 and run_peak_bytes 5,737,009 -> 8,546,977, with
run_held_peak_bytes 216,080 -> 3,026,048. The interpreted corpus reads
interp_instructions 430,978,258 -> 469,727,924, interp_peak_bytes 765,587 ->
1,203,737 and interp_allocs 461,292 -> 500,455. The tab compiles and runs it:
browser_compile_instructions 273,807,879 -> 303,782,324,
browser_compile_peak_bytes 715,283 -> 722,748, browser_run_instructions
42,417,870 -> 280,790,947 and browser_run_peak_bytes 1,310,720 -> 2,097,152.
The run program's emitted code grows by the phase: emitted_other_defines
2,204 -> 2,209, emitted_other_calls 20,078 -> 20,164, emitted_other_branches
17,572 -> 17,649, emitted_other_lines 165,914 -> 166,511 and text 5,646,113
-> 5,647,585. Its allocation vein: run_allocs 1,446,946 -> 1,449,080,
run_alloc_bytes 215,868,632 -> 219,600,424, run_beat_iters 112,473 ->
113,323, run_sh_buf 86,960,912 -> 87,015,792, run_sh_map 12,160 -> 25,760
and run_sh_rec 3,906,784 -> 3,934,048. Welfare re-bases the eight terms the
phase moves, each keeping its ratio to its baseline, and the floor stays at
94.83.

## 2026-10-25 — a set in a record admits a word in place

The fix for `deduped`, the shape the entry above put in the corpus. Three
counts in `src/linear.rs` kept the record and its set from being written in
place, and each is now taken on the path it describes.

A field's element read in a condition, `st.seen[x]`, is finished before
either arm runs, as `m[k]` on a plain map already was. `FieldTally` no
longer counts it as a read of the field the arm below moves, and
`count_in_condition` no longer counts it as a use of the record.

An arm that answers a name whole, the `st` in `if c (words (put st.seen x
true) (push st.out x)) st`, is judged on its own path. The body's count
takes the busier arm, which reads `st` twice for its fields, so the bare
`st` was counted twice and never handed over. The path through the bare arm
uses the name at most that count less how many more times the other arm
does, and that is the discount it now gets. The bound holds because the
path through the other arm, with the same choices outside the `if`, is
among those the body's count already covers.

Whether every caller hands a record over is a greatest fixpoint, as
linearity is. It was computed once, after linearity, and a loop that hands
its record back whole from one arm hands it over exactly when its own
callers do, which a single pass could not see. `records_handed` now starts
from every parameter called by name and loses those whose call sites refuse
under what is left, in the same loop as the field facts. A parameter used
at most once on a path is unique there when its callers hand it over, which
is what `records_handed` says of a record some other path reads twice for
its fields.

Natively, keeping the distinct words of 5,000 took 2,221,969,523
instructions and now takes 7,363,697; of 20,000, 44,652,443,199 and now
35,005,980. The interpreter took 1.59 s at 20,000 and now takes 0.05 s.
The same set written through a helper that guards, `return st if
st.seen[x] != none` above the rebuilt record, is still copied: the helper's
own answer is decided in the linearity fixpoint, before the record facts
exist, and joining the two fixpoints is a larger change than this one.

`mem/a_set_in_a_record_admits_without_copying` keeps 2,000 of 4,000 words
in 33 allocations of 370,640 bytes, where the tree without the fix made
10,184 of 92,997,520. `micro/a_set_in_a_record_keeps_what_its_holders_saw`
runs the shape on a record only it holds, which is written in place, and on
records a caller reads again or whose set is read after the write, which
are not. It prints what main's interpreter prints. A generated differential
over 120 such programs, fresh seeds and shared ones, spelled with `if` and
with a guard, agrees with main's interpreter on the interpreter and both
native tiers; 15 of them write in place. Making every bare arm count as
unique leaves all of these green, because a field moves only when every
other mention of the record is a field read, so the arm's discount alone
cannot reach a record something else holds.

CI's rows, against #2120's corpus: the run program 855,535,787 ->
810,657,719 (-5.2%), the interpreted corpus 469,727,924 -> 455,385,272
(-3.1%), and the tab's run 280,790,947 -> 44,919,990, with
browser_run_peak_bytes 2,097,152 -> 1,310,720. Against main, which holds
neither the phase nor the fix: work_runbench 809,823,849 -> 810,657,719,
run_peak_bytes 5,737,009 -> 5,744,913, interp_instructions 430,978,258 ->
455,385,272, interp_peak_bytes 765,587 -> 1,222,789, interp_allocs 461,292
-> 490,981, browser_run_instructions 42,417,870 -> 44,919,990,
browser_compile_instructions 273,807,879 -> 308,823,716 and
browser_compile_peak_bytes 715,283 -> 722,756, each the phase less what the
fix takes back. The interpreter's peak stays where the phase put it, and
moves 1,203,737 -> 1,222,789 against #2120; nothing here isolates why. The
run
program's allocation vein against main: run_alloc_bytes 215,868,632 ->
215,955,960, run_put_mut_grow 48 -> 55, run_push_mut_slow 1,579,791 ->
1,580,216, run_sh_buf 86,960,912 -> 86,961,200, run_bytes_malloc 7,901 ->
7,910, run_perm_live_bytes 32,049 -> 48,273 and run_perm_peak_bytes 802,321
-> 810,225. The phase's code is in the corpus entry above: text 5,646,113 ->
5,647,825 and emitted_other_lines 165,914 -> 166,508 with it.

The analysis costs the front end more. `records_handed` is now settled over
rounds rather than in one pass, and every bare arm counts the other arm:
compile_instructions 24,695,561 -> 24,731,131 (+0.14%), entry_instructions
72,875,974 -> 72,970,344, library_instructions 72,547,701 -> 72,642,115 and
emit_instructions 5,636,850 -> 5,641,862, and the tab's compile
browser_compile_instructions 303,782,324 -> 308,823,716 against #2120.
Welfare rises 94.83 -> 95.01 and the rise is banked.

## 2026-10-25 — a map of maps joins the corpus

A count kept per pair of keys as a map of maps, `inner = or_map m[a]` then
`put m a (put inner b (or_zero inner[b] + 1))`, is quadratic natively (#2122).
Both puts are emitted as the persistent `k_b_put`. The outer one is refused
because `inner` reaches it through the nested put, and `fed_to_consumer`
accepts only arithmetic and comparison operands. The inner one is refused
because the outer map still holds the map it was read from. The copied inner
map sorts itself again on its next read. 5,000 counts over 50 inner maps took
189 million instructions and 20,000 took 2.28 billion. The interpreter counts
holders at run time and takes the inner map when only the outer slot and the
binding hold it, so it stays near linear.

Under the 2026-09-05 corpus-first gavel `nested` joins the corpus here,
measured on main at kanso#2121 before its fix. It keeps 1,700 counts. At
that size the inner maps hold about 34 keys each and the phase grows about
linearly, at some 24,000 instructions a count, most of it the copy. On this
container the run program reads 851,889,184 with it and 810,763,317 without
it, a column of 41,125,867 (4.83%). The interpreted corpus keeps 1,000
counts, 497,655,907 → 520,126,043, and the tab runs the same corpus on the
native runtime.

CI's rows, against #2121: work_runbench 810,657,719 → 851,781,946,
run_allocs 1,446,942 → 1,456,758, run_alloc_bytes 215,955,960 →
221,484,024, run_held_peak_bytes 216,080 → 4,120,448, run_sh_buf 86,961,200
→ 87,410,768 and run_sh_map 12,160 → 120,960. The interpreter reads
interp_instructions 455,385,272 → 478,102,371, interp_allocs 490,981 →
516,759 and interp_peak_bytes 1,222,789 → 1,233,920. The tab reads
browser_run_instructions 44,919,990 → 130,514,490 and browser_run_peak_bytes
1,310,720 → 4,325,376, and compiling the corpus there costs
browser_compile_instructions 308,823,716 → 314,821,332 and
browser_compile_peak_bytes 722,756 → 728,260. The new module
adds machine code and emitted lines: text 5,647,825 → 5,649,473,
emitted_other_lines 166,508 → 167,039, emitted_other_branches 17,649 →
17,724, emitted_other_calls 20,164 → 20,219 and emitted_other_defines 2,209 →
2,214. Each welfare term these move is re-based to keep its ratio to its
baseline, so the score stays 95.01.

## 2026-10-25 — a map of maps is written in place

The count per pair of keys from #2125, `inner = or_map m[a]` then
`put m a (put inner b (or_zero inner[b] + 1))`, now writes both maps in
place natively (#2122). Four changes in `src/linear.rs` do it.

- **The outer write.** `fed_to_consumer` discounted a read binding only when
  every later mention reached the write as an arithmetic or comparison
  operand. A sibling that writes into the binding, `put inner b …`, forces it
  before the outer write runs, and a binding forced once answers every later
  mention from its value, so the binding's read of `m` is discounted there
  too. This alone took the probe at 5,000 counts from 189 million
  instructions to 101 million.
- **Values alone.** `elems_params` is a greatest fixpoint over map
  parameters: every caller hands in a map literal of unique values, a write
  of a unique value into a map nobody else holds, or a parameter or binding
  that is one, and no arm keeps an element. An element read may be compared,
  interpolated or read further; any other read drops the parameter. A
  parameter may also be written into, counted with `length` or `keys`,
  answered whole, or handed to another parameter that keeps the promise.
  `returns_elems` carries the promise through a helper's answer, so a loop
  that calls `bump m a b` gets it from the helper.
- **The take.** `m[k]` read into a name that is written into and goes back
  into `m` at the same key, with nothing else mentioning the name but reads,
  moves the element out (`element_moves`). The map is linear, so the arm
  uses it once and the outer write is in place, and its values are alone, so
  the map's slot is the element's only other holder until the write replaces
  it.
- **Pass-through.** A function whose every arm answers one parameter,
  mentioned nowhere else, or a fresh literal answers a unique value wherever
  that argument is unique. `or_map` is one. It is asked per call, so one
  caller handing it a shared map refuses only that call; the first version
  needed `or_map`'s parameter linear for every caller, and a fixture that
  shared `or_map` across six loops proved none of them.

The seeding is gated on a take existing anywhere in the program, so a
program without the shape builds no map facts. The probe at 20,000 counts
took 2,275,554,111 instructions and takes 35,953,409, and 5,000 to 20,000 is
a ratio of 4.7. A mem fixture of 2,000 counts over 20 inner maps allocates
325 times and 401,472 bytes, where main allocated 12,202 times and 6,618,224
bytes, and at 1,000 counts 220 and 164,752 against 6,089 and 2,300,544.

`an_inner_map_held_elsewhere_keeps_what_it_held` gives each refusal a loop
of its own, because a parameter is proven for every caller or none and a
shared loop let one refusal hide another. Its output came from main's
interpreter. Three ratchet mutations each turn it red on both engines: a map
literal's values taken as alone whatever they are, an arm allowed to keep an
element, and a take put back at another key. A check that the take arm uses
the map once was removed rather than pinned: a written map parameter is
linear only if every arm uses it once, so the check could not fail.

A loop whose arms are spelled with `return m if i == 0` is proven the same
way. The interpreter writes in place at these sites on the analysis' word,
so a wrong proof would be wrong identically on both engines; the fixture's
expected output is main's interpreter for that reason.

Grouping into a map of lists, `put m k (push (or_list m[k]) i)`, is the same
take with `push` as the write, and it is proven by the same change: 8,000
numbers in 20 groups allocated 32,786 times and now 129.
`a_map_of_lists_groups_in_place` pins it.

CI's rows, against #2125. The run program reads work_runbench 851,781,946 →
814,679,425 (−4.4%), run_allocs 1,456,758 → 1,447,392, run_alloc_bytes
221,484,024 → 216,354,904, run_held_peak_bytes 4,120,448 → 248,688 and
run_sh_map 120,960 → 12,160. run_put_mut_grow rises 55 → 260, because the
inner maps now grow in place where each copy had been sized afresh. The
interpreter reads interp_instructions 478,102,371 → 473,127,289 and
interp_allocs 516,759 → 507,913, and interp_peak_bytes rises 1,233,920 →
1,263,998. The tab's run falls browser_run_instructions 130,514,490 →
47,716,164 and browser_run_peak_bytes 4,325,376 → 1,376,256.

The analysis costs the front end. The interpreted corpus holds a take, so
the tab's compile seeds the map facts and runs their fixpoint:
browser_compile_instructions 314,821,332 → 324,458,065 (+3.1%). Native
compiles read compile_instructions 24,731,131 → 24,741,552, entry_instructions
72,970,344 → 72,986,583, library_instructions 72,642,115 → 72,659,822 and
emit_instructions 5,641,862 → 5,655,748, and the machine code reads text
5,649,473 → 5,649,809. Welfare rises 95.01 → 95.15 and the rise is banked.

## 2026-10-25 — a list built by concat, and a map of records, join the corpus

A list built one number at a time with `list/concat acc [i]` costs about ten
times what `push` costs natively. The concat is made in place, but each lap's
one-element list is garbage, so the loop stages what it carries, and the
accumulator is not covered by the license that lets a list of numbers cross
a rewind by pointer: that license reads in-place pushes only. Each stage then
deep-copies the accumulator. On its own, 32,000 numbers took 14.6 million
instructions and 128,000 took 156 million, most of it in `k_deep_copy`, where
`push` takes 0.9 million and 2.6 million. The interpreter is linear on it.

Under the 2026-09-05 corpus-first gavel `appended` joins the corpus here,
measured on main at kanso#2126 before its fix. It keeps 64,000 numbers. In
the run program it grows linearly at about 218 instructions a number. On
this container the run program reads 828,713,625 with it and 814,784,428
without it, a column of 13,929,197 (1.68%). The interpreted corpus keeps
6,000 numbers, and the tab runs the same corpus.

`bucketed` joins beside it: numbers filed into a map of records, each a count
and a list, `b = or_bucket m[k]` then `put m k (bucket (b.hits + 1) (push
b.items i))`. Both puts are the persistent `k_b_put`. The record reaches the
put only through its fields, and the read discount accepts a name that
reaches the write through arithmetic or a nested write into it, so the map is
copied for every number. The lists are not copied: on its own the shape
stays linear as they grow, at about 8,300 instructions a number. It keeps
2,000 numbers; the run program reads 847,407,464 with both phases, a column
of 18,693,839 (2.21%), and the interpreted corpus keeps 1,500. With both, the
interpreted corpus reads 515,277,561 → 567,834,663 on this container.

CI's rows, against #2126. The run program reads work_runbench 814,679,425 →
847,302,475, run_allocs 1,447,392 → 1,585,686, run_alloc_bytes 216,354,904 →
224,179,080, run_push_mut_slow 1,580,216 → 1,644,216, run_beat_iters 113,323
→ 177,323, run_sh_buf 87,169,488 → 89,503,776, run_sh_map 12,160 → 76,160,
run_sh_rec 3,905,056 → 4,034,336, run_bytes_malloc 7,910 → 7,919 and
run_bytes_freed 7,768 → 7,777. Its memory rows read run_arena_blocks 4 → 5,
run_arena_peak_bytes 4,718,608 → 5,767,184, run_held_peak_bytes 248,688 →
1,863,712, run_perm_peak_bytes 810,225 → 1,858,817 and run_spare_peak_bytes
2,621,456 → 3,670,032. The interpreter
reads interp_instructions 473,127,289 → 525,831,073, interp_allocs 507,913 →
585,540 and interp_peak_bytes 1,263,998 → 1,281,294. The tab reads
browser_run_instructions 47,716,164 → 107,399,948, browser_run_peak_bytes
1,376,256 → 3,866,624, browser_compile_instructions 324,458,065 →
334,007,116 and browser_compile_peak_bytes 728,260 → 743,930. The two
modules add machine code and emitted lines: text 5,649,809 → 5,652,977,
emitted_other_lines 167,036 → 167,711, emitted_other_branches 17,724 →
17,809, emitted_other_calls 20,219 → 20,315 and emitted_other_defines 2,214 →
2,222. Each welfare term these move is re-based to keep its ratio to its
baseline, so the score stays 95.15.

## 2026-10-25 — entry and play files keep the blank-line policy

`check_blank_policy` judged libraries only. `parse_entry` never called it,
and `parse_play` called it on a declaration half whose blank lines it made
up, one per gap. So an entry or play file could begin with a blank line, hold
one inside a body, and put any number between two declarations or two
statements, and every engine ran it (#2143). Both now judge the file as
written. A top-level `x = …` there is a statement of the run, so it may pack
against its neighbours, and at most one blank line separates two statements.
An import below a statement is refused for its place, and the blank above it
adds nothing.

Eleven examples had a double blank between declarations, and two began with
a blank line; their output is unchanged. Four error goldens cover the new
refusals, each of which main ran: two blanks between a play file's
declarations, two between statements, a blank inside an entry's `if` body,
and a leading blank in an entry file. kq's three entries pass.

CI's rows. Five compile rows fell: compile_instructions 24,741,552 ->
24,657,263, entry_instructions 72,986,583 -> 72,734,084,
library_instructions 72,659,822 -> 72,405,561, emit_instructions 5,655,748
-> 5,654,777 and browser_compile_instructions 334,007,116 -> 333,912,133.
interp_instructions rose, 525,831,073 -> 525,925,462, since the corpus's
entry is now judged line by line. Nothing here separates the part of each
move that is the new check from the part that is the binary's layout.
Welfare holds the net rise at 95.15.

## 2026-10-25 — a concat of numbers crosses the rewind where it stands

`is_scalar_list_chain` in `src/beat.rs` now accepts an in-place `concat` whose
second argument is a list literal of scalars, as it accepted an in-place
`push` of one. Such a concat writes each item onto the accumulator's frontier,
so the accumulator still holds no pointer into the arena, and the license
that lets a list of numbers cross a rewind by pointer applies. The loop from
#2139 stops deep-copying its accumulator at every stage. On its own, 128,000
numbers took 156,368,201 instructions and take 19,718,704, and 40,000 took
22,454,668 and take 6,496,051. No allocation counter sees the copies the
stage made, so the run program's work row is where this shows.

The one-element list each lap is still built. Emitting such a concat as one
push per item would remove it, but a list literal keeps an item that fails
where `push` answers the failure, so the two would need to agree first.

`a_list_concatenated_in_place_keeps_its_items` concatenates numbers, pairs,
booleans and strings made on the lap, 20,000 laps each. Its output is main's
interpreter. The strings must stay refused: the ratchet mutation
`a_concat_of_strings_crosses_the_rewind` admits any literal, and the native
run of the fixture then dies reading the reclaimed strings ("ran out of
stack"), while the interpreter, which has no rewind, stays green.

## 2026-10-25 — a read a sibling forces lets the map go

`bucketed` from #2139, `b = or_bucket m[k]` then `put m k (bucket (b.hits +
1) (push b.items i))`, now writes the map in place. #2126 discounted a read
binding when a sibling of the write wrote into it. `forces` in
`src/linear.rs` generalizes that to any position that reads the value and
runs on every path before the write: a field or element read, an operand,
an interpolation, a condition, or an argument of `put`, `push`, `delete`,
`length` or `keys`. A binding forced once answers every later mention from
its value, so the write cannot change what the binding read. A bare name
handed to a function or a constructor does not count, because a lazy binding
travels there as an unforced cell; neither do one arm of an `if` (and so
`and` and `or`), a guard past its condition, or a lambda.

On its own, 16,000 numbers in 20 buckets took 132,724,010 instructions and
take 9,025,501. `a_map_of_records_files_in_place` pins 4,000 numbers: 8,133
allocations and 440,064 bytes, where main made 20,457 and 5,216,096.
`a_record_read_out_of_a_map_keeps_its_fields` keeps the record it read in a
carried list, in a lambda and under one arm of an `if`; its output is main's
interpreter, and both engines agree with it.

The read context is conservative and no program here shows it necessary. A
binding is lazy only when every use hands it bare to a function that may
discard it, and with the context dropped the probe that tried that shape
still printed main's answer, because its binding stayed strict. No ratchet
mutation is pinned for the context for that reason.

CI's rows for both fixes, against main at #2146. The run program reads
work_runbench 847,302,475 → 828,232,672 (−2.3%), run_allocs 1,585,686 →
1,579,529, run_alloc_bytes 224,179,080 → 221,800,088, run_sh_map 76,160 →
12,160, run_sh_buf 89,503,776 → 89,333,968, run_held_peak_bytes 1,863,712 →
248,688, run_arena_blocks 5 → 4, run_arena_peak_bytes 5,767,184 → 4,718,608
and run_spare_peak_bytes 3,670,032 → 2,621,456; run_put_mut_grow rises 260 →
264, where the bucketed map now grows in place. The interpreter reads
interp_instructions 525,925,462 → 520,276,149 and interp_allocs 585,540 →
578,088, and interp_peak_bytes rises 1,281,294 → 1,281,313. The tab's run
falls browser_run_instructions 107,399,948 → 54,006,705 and
browser_run_peak_bytes 3,866,624 → 1,507,328, its compile rises
browser_compile_instructions 333,912,133 → 334,848,599, and
browser_compile_peak_bytes falls 743,930 → 743,876. Native compiles read
compile_instructions 24,657,263 → 24,735,836, entry_instructions 72,734,084 →
72,954,229, library_instructions 72,405,561 → 72,625,200 and
emit_instructions 5,654,777 → 5,655,925: the two analyses cost the compiler
about 0.3%. The machine code reads text 5,652,977 → 5,653,121, all of it in
runbench. Welfare
rises 95.15 → 95.22 and the rise is banked.

## 2026-10-25 — two strings the interpreter makes quadratic join the corpus

Probing the standard library's text functions on the interpreter turned up
two shapes it runs in quadratic time, where native and the tab run them in
linear time. Both grow or walk one long string through a loop.

`joined_onto "{s}{i}," (i - 1)` builds a string a piece at a time.
`linear::string_builders` licenses that join, and native and the tab write
it in place, but the interpreter never asked the analysis. Its `Text` held a
long string as a plain `String`, so reading `s` copied it and the template
copied it again. Run alone, 3,000 pieces took 76,760,738 instructions and
12,000 took 1,551,921,916; 200,000 took 2.39 s against native's 0.08 s.

The second is a scanner: a loop that hands a long string to each step and
reads it a character at a time with `text/slice s i i`. Each call copied the
string, and `text/slice` counted the characters and then collected all of
them into a `Vec<char>` to take one. 2,500 pairs of characters took
497,826,665 instructions, 65% of it collecting, and an 80,000-character
walk took 6.9 s against native's 0.08 s.

Under the 2026-09-05 corpus-first gavel `grown 3000` and `scanned 1000` join
the interpreted corpus here, measured before their fixes. Both engines
print the same line for it.

CI's rows for the corpus with the two phases: interp_instructions
525,925,462 -> 636,804,390, interp_allocs 585,540 -> 651,249 and
interp_peak_bytes 1,281,294 -> 1,298,220. The tab compiles and runs the
same corpus: browser_compile_instructions 333,912,133 -> 343,748,020,
browser_compile_peak_bytes 743,930 -> 751,818, browser_run_instructions
107,399,948 -> 111,470,514 and browser_run_peak_bytes 3,866,624 ->
3,997,696. Welfare re-bases the six terms among these it weighs, each
keeping its ratio to its baseline, and the floor stays at 95.15.

## 2026-10-25 — a joined string grows where it stands on the interpreter

`linear::string_builders` licenses a template whose first part is a string
binding nothing reads afterwards, and native and the tab write those joins in
place. The interpreter now asks the same question. At a licensed site
`eval_template` renders the rest of the template first, so `"{s}{s}"` still
reads all of `s`, then takes the string out of its binding and pushes the
rest onto it. A string short enough to sit inline skips the question, since
copying fifteen bytes costs less than building the analysis.

The interpreter used to build `in_place_pushes` alone. It now calls
`linear::for_the_interpreter`, which answers the push sites and the join
sites from one analysis and drops it, so the run holds two sets and not the
analysis behind them. Holding the analysis had cost interp_peak_bytes
175,000 bytes in the first draft.

`grown 200000` ran in 2.39 s and runs in 0.06 s, against native's 0.08 s.
`a_string_joined_onto_itself_reads_itself_whole` pins `"{s}{s}"` at a
licensed site, where a join that took the string before rendering the rest
would print half of it.

CI's rows, against #2150 on main at #2146. The interpreter reads
interp_instructions 636,804,390 -> 629,524,864 and interp_allocs 651,249 ->
643,872, and interp_peak_bytes rises 1,298,220 -> 1,298,476. CI's
interpreter gate runs under pinned glibc tunables, where memcpy costs little
in the count, so the copies that took 2.3 seconds at 200,000 pieces read as
7.3 million instructions there. `kanso check` never builds the interpreter's
sets, so the compile rows move with the binary's layout:
compile_instructions 24,657,263 -> 24,694,028, entry_instructions 72,734,084
-> 72,837,734, library_instructions 72,405,561 -> 72,508,468,
emit_instructions 5,654,777 -> 5,657,388 and browser_compile_instructions
343,748,020 -> 343,857,683. Welfare stays at 95.15.

## 2026-10-25 — a scanned string is read where it stands on the interpreter

The interpreter's `Text` held a long string as a `String`, so handing one to
a function copied it. A long string is now an `Rc` around the bytes and a
flag saying whether they are all ASCII, computed once when the string is
made. A call shares it.

`text/slice` on an ASCII string indexes bytes, and so does `s.i`. On other
strings `text/slice` counted the characters and collected them all into a
`Vec<char>` to take one. It now walks from a cursor the interpreter keeps:
the last string sliced and the character and byte offsets where the slice
ended, so a scanner that reads the next character walks one character.

An 80,000-character scan ran in 6.9 s and runs in 0.089 s, against native's
0.08 s. A scan of non-ASCII text took 660 million instructions and takes 89
million. `a_scanned_string_reads_each_character` scans a string that mixes
ASCII with two- and three-byte characters, forwards and back, so a cursor
that kept a stale offset prints the wrong characters.

CI's rows, against #2151. The interpreter reads interp_instructions
629,524,864 -> 568,584,597 (-9.7%) and interp_allocs 643,872 -> 632,223,
and interp_peak_bytes holds at 1,298,476. The compile rows move with the
binary's layout: compile_instructions 24,694,028 -> 24,711,192,
entry_instructions 72,837,734 -> 72,889,740, library_instructions 72,508,468
-> 72,563,065, emit_instructions 5,657,388 -> 5,654,754 and
browser_compile_instructions 343,857,683 -> 343,738,623. Against #2151
welfare read 95.15 -> 95.17.

Merged with main at #2141, CI reads the three together against main. The
interpreter reads interp_instructions 520,276,149 -> 564,058,857,
interp_allocs 578,088 -> 624,771 and interp_peak_bytes 1,281,313 ->
1,298,495. The tab runs and compiles the two corpus phases:
browser_run_instructions 54,006,705 -> 61,205,256, browser_run_peak_bytes
1,507,328 -> 1,572,864, browser_compile_instructions 334,848,599 ->
344,676,704 and browser_compile_peak_bytes 743,876 -> 751,764. The compile
rows move by layout: compile_instructions 24,735,836 -> 24,747,458,
entry_instructions 72,954,229 -> 72,992,046, library_instructions
72,625,200 -> 72,664,658 and emit_instructions 5,655,925 -> 5,657,365. The
corpus re-base keeps the ratios #2150 measured against #2146. Welfare rises
95.22 -> 95.24 and the rise is banked.

## 2026-10-25 — gavel: a local may not reuse any name its module declares, and the refusal names where

Clay ruled #2085. Every function in a module is visible from every file of
it, so a local in one file that reuses a name declared in another file
shadows a callable name, which is what no-shadowing forbids. Eighteen of the
twenty ports met it as action at a distance: a declaration added in `b.kso`
refused a parameter in `a.kso`, and the refusal pointed at the parameter
without naming the declaration.

**The ruling.** The rule stands across files. The refusal names the
declaration it collides with, by file and line, so the fix is a rename made
knowing what clashed. Letting a local reuse a name declared only in another
file was declined, because inside the local's scope the name would mean two
things. File-scoped private names were declined, because they would end the
sharing of helpers between a module's files.

Owes, on #2085: the refusal names the colliding declaration's file and line;
an error golden for a two-file module.

## 2026-10-25 — gavel: `put` writes a list at an index

Clay ruled #2086. `xs[i]` reads a list the way `m[k]` reads a map, and
`put m k v` answers a map with the value at `k` replaced. Nothing wrote a
list, so the ports built vectors from maps with integer keys and paid for
keys they did not need.

**The ruling.** `put xs i v` answers the list with element `i` replaced,
1-based like `xs[i]`. An index outside the list answers `none`, as a read
there does. A list nothing else holds is written in place, as a map already
is.

Owes, on #2086: `put` on a list on every engine; the in-place write under
the existing uniqueness analysis; the checker accepting a list for `put`, and
refusing at check what it refuses at run time (diff F9, inflate F8); micro and
mem goldens; appendix B.

## 2026-10-26 — gavel: a failure inside the library also names the program's line that called it

Clay ruled #2099. Under #1975 a run-time failure names the line it happened
on, so a failure inside the standard library named a library line, such as
`failed in text/split at std/text/text.kso:38`, and nothing pointed at the
program. Every port asked for its own line.

**The ruling.** A failure inside the library keeps its library line and adds
`called from <file>:<line>`, the nearest line in the program's own files. The
library line is still what a library author needs, and the program's line is
what its author needs. Native records that line on each call from a program
file into a library function. Its cost is the cost of a ruled feature: the
floor moves by what CI measures, as CLAUDE.md's language exception says.

Owes, on #2099: the caller's line on every engine for a failure born in the
library; error goldens on every engine for an operator, a builtin and a
library function reached from a program file; CI's rows and the floor moved
by them.

## 2026-10-25 — a list walked by its rest joins the corpus

`rest = list/to_list (list/drop xs 1)` is how most of this repository's
scripts walk a list: the welfare script, the trend gate, `golden_prose` and
the differentials among them. A battery of list and text shapes run at two
sizes on both engines found it the worst of them. Each rest is built afresh,
so a walk costs the square of the list on any engine, and `list/drop` builds
it by stepping a cursor through the list one record at a time: `fold` has an
arm that reads a capped cursor by index, which `take` reaches, and none for
a skipped one. Walking 5,000 elements took 6.5 seconds natively and 23 on
the interpreter, about 260 instructions an element stepped natively and
12,700 interpreted.

Under the 2026-09-05 corpus-first gavel `rested` joins the run program with
500 elements and the interpreted corpus with 70, measured on main at #2148
before its fix. On this container the run program reads 888,090,230 with it
and 847,302,584 without it, a column of 40,787,646 (4.59%), and the
interpreted corpus reads 525,925,462 -> 559,915,367. Both engines print the
same line for it.

The rest of the battery is linear or n log n on both engines: `group_by`,
`index_by`, `tally`, `to_h`, `sort` and `text/split` at 5,000 and 20,000
elements. Taking a string's tail with `text/slice s 2 (length s)` is linear
natively and quadratic on the interpreter, 64 ms at 5,000 characters and
563 ms at 20,000. On #2152's build, which shares a long string and slices
an ASCII one by byte, the same two read 39 ms and 77 ms.

CI's rows, against main at #2141. The run program reads work_runbench
828,232,672 -> 869,011,503, and each rest's step records cost it memory as
well: run_allocs 1,579,529 -> 1,832,525, run_alloc_bytes 221,800,088 ->
242,831,624, run_sh_rec 4,034,336 -> 20,162,336, run_sh_buf 89,333,968 ->
94,217,168, run_arena_blocks 4 -> 22, run_arena_peak_bytes 4,718,608 ->
23,592,976, run_spare_peak_bytes 2,621,456 -> 21,495,824, run_perm_peak_bytes
1,858,817 -> 1,867,025, run_push_mut_slow 1,644,216 -> 1,646,861 and
run_bytes_malloc 7,919 -> 7,921. The new module adds machine code and
emitted lines: text 5,653,121 -> 5,657,953, emitted_other_lines 167,704 ->
168,561, emitted_other_branches 17,809 -> 17,916, emitted_other_calls 20,309
-> 20,437 and emitted_other_defines 2,222 -> 2,231. The interpreter reads
interp_instructions 520,276,149 -> 554,451,443, interp_allocs 578,088 ->
623,573 and interp_peak_bytes 1,281,313 -> 1,288,124. The tab reads
browser_run_instructions 54,006,705 -> 63,054,006, browser_run_peak_bytes
1,507,328 -> 2,490,368, browser_compile_instructions 334,848,599 ->
344,866,301 and browser_compile_peak_bytes 743,876 -> 806,972. Welfare
re-bases the eight terms among these it weighs, each keeping its ratio to
its baseline, and the floor stays at 95.22.

## 2026-10-25 — the rest of a list after a drop is one copy

`list/to_list (list/drop xs k)` now takes the rest of a list in one copy.
`to_list` has an arm for a skip over a cursor, the shape `list/drop` makes of
a list, and it asks a new internal builtin, `rest_from`. Given a list and a
position of at least 1, it answers the list's elements from that position
on, copied in one move. Given anything else it answers none, and std walks
that a step at a time as before, so a string still drops to its characters,
bytes to their numbers and a negative count to nothing. `fold` gains the
indexed arm a capped cursor already had, so a fold over a drop reads by
index. Before this, each rest was built a step record and an advanced cursor
per element.

Walking 5,000 elements by their rest took 6.5 s natively and 23 s on the
interpreter, and takes 0.125 s and 0.15 s. The walk still copies each rest,
so it is still quadratic: 20,000 elements take 2.1 s and 1.8 s. A rest that
shared its list's storage would be linear, but a list's items pointer is
also how a push onto it finds its buffer's header, so a list that points
into the middle of another's items needs a header of its own first.

`a_list_dropped_from_hands_on_its_rest` drops nothing, the whole list, past
the end and a negative count, from lists of numbers, records and lists, and
from a string, bytes and a map, which keep their step walk; it also folds
over a drop. Its output is main's interpreter, and both engines agree with
it. A first draft clamped a negative drop to the whole list, where main
answers none of it, and the fixture caught the difference. Moving the copy
one element either way, on either engine, turns it red.

On this container the run program reads 888,090,230 -> 847,406,017 and the
interpreted corpus 559,915,367 -> 527,644,513: all but about 100,000 of the
phase's 40.8 million instructions natively, and all but 1.7 million of its
34 million interpreted.

CI's rows, against #2156's phase. The run program reads work_runbench
869,011,503 -> 828,685,220, and its memory comes back: run_allocs 1,832,525
-> 1,581,589, run_alloc_bytes 242,831,624 -> 223,888,104, run_sh_rec
20,162,336 -> 4,098,336, run_sh_buf 94,217,168 -> 91,336,688,
run_arena_peak_bytes 23,592,976 -> 4,718,608, and run_push_mut_fast lands at
1,191,311 and run_push_mut_slow at 1,644,657. basket, which drops from a
list too, reads 18,214,855 -> 18,212,549 instructions and basket_allocs
27,272 -> 27,249. The interpreter read interp_instructions 554,451,443 ->
523,165,714 against the phase. Main at #2152 added `texts` to the
interpreted corpus, so against main the interpreter's rows carry the phase
and its fix together: interp_instructions 564,058,857 -> 565,857,214,
interp_allocs 624,771 -> 628,011 and interp_peak_bytes 1,298,495 ->
1,313,603. The tab's run reads browser_run_instructions 61,205,256 ->
61,369,685, and browser_run_peak_bytes stays at main's 1,572,864.

The longer std/list and the builtin cost every compile. Against main at
#2152: compile_instructions 24,747,458 -> 25,157,249, entry_instructions
72,992,046 -> 73,813,902, library_instructions 72,664,658 -> 73,484,685,
compile_allocs 13,836 -> 14,025, module_visits 1,368 -> 1,460,
emit_instructions 5,657,365 -> 5,698,770, codegen_instructions_dev
116,477,304 -> 116,483,659 and codegen_instructions_release 370,206,363 ->
370,222,776. The front end visits compile_corpus more often and holds more
while it does, front_end_visits 7,251 -> 7,373 and compile_peak_bytes
598,984 -> 607,213, read on this container, whose rustc is the one the
golden names. The tab compiles the corpus with `rested` in it as well, so
its compile rows carry the phase too: browser_compile_instructions
344,676,704 -> 359,922,625 and browser_compile_peak_bytes 751,764 ->
839,011, where the fix alone read 344,866,301 -> 349,801,935. Every benchmark's machine
code carries the new runtime function, about 704 bytes each: text
5,657,953 -> 5,674,129 over the fourteen, runbench 810,652 -> 815,164. The
two programs that never drop read work_encodebench 2,327,386,052 ->
2,327,386,353 and work_livebench 936,013,492 -> 936,013,758 by layout. The
emitted lines of the drop's new arms land at emitted_other_lines 168,907,
emitted_other_branches 17,946, emitted_other_calls 20,494 and
emitted_other_defines 2,239. `skip_shape` no longer pushes onto a fresh
list at all: skip_shape_push_mut_fast 8 -> 0, with its allocs 37 -> 13.
Welfare rises 95.24 -> 95.36 and the rise is banked.

## 2026-10-25 — a delete marks its pair dead where it stands

`k_b_delete_mut` took a key out of a map nobody else holds by moving every
pair after it down one slot. In the run program's `drained` phase that move
was 87% of the phase's instructions, and draining half of 80,000 keys took
0.92 s (#2118).

Now the deleted pair keeps its slot. Its key moves into the value's slot, and
the key's slot takes `K_GONE`, a tag no key has (`K_DONE` reused). The pairs
stay sorted by key, so the next delete finds its key with a binary search
that reads a dead pair by the key it kept. Dead pairs at the end of the map
are dropped at once. The header records the state in `sorted_len`, which is
negative while the map has no view: -(len + 1) while the pairs are what the
deletes left, and any other negative once an in-place put appends past them.
A lookup searches past the dead pairs. Every other reader closes the gaps
first, in one pass at the top of `k_map_sort_build`. A persistent put closes
them before it can share the pairs with a new header, and so do the deep copy
and the carry's evacuation, whose headers start with no view.

Measured alone, draining half of 1,100 keys took 7,630,173 instructions and
takes 1,172,819; half of 3,000 took 55,939,149 and takes 2,860,308; half of
6,000 took 220,510,861 and takes 5,749,048. Half of 80,000 keys takes 0.033 s.

`delete_marks` counts the pairs marked dead. It is printed only by a run that
marked one, so the veins that never delete in place, the sibling
repositories' among them, read as before. The run program reads 550 and
`a_map_drained_by_delete_loses_keys_in_place` reads 1,000.
`a_drained_map_reads_past_its_dead_pairs` meets a drained map with each
reader: a lookup, the length, the keys, an in-place put and a delete after
it, a delete of a key already gone, a drain to empty, and two persistent
puts beside the maps they copy. Three mutations turn it red: a search that
answers a dead pair, a persistent put that shares dead pairs, and gaps
closed as though nothing had been appended.

CI's rows, against main at #2158. The run program's drain reads
work_runbench 828,685,220 -> 822,295,072 (-0.77%), and its allocation vein
gains delete_marks=550; the drain's mem golden gains delete_marks=1000.
Every reader of a map now asks first whether it holds dead pairs, which the
programs that never delete pay for: work_basket 18,212,549 -> 18,278,697,
work_widebench 28,413,165 -> 28,445,119, and by a few hundred or less
work_deepbench 197,978,249 -> 197,978,593, work_digestbench 4,122,823 ->
4,122,987, work_encodebench 2,327,386,353 -> 2,327,387,236 and
work_escapebench 40,308,401 -> 40,308,403. The others move by layout:
indexbench 1,837,185, jsonbench 557,386,408, livebench 936,013,522, oneshot
10,033,523, pendbench 155,724,444, readbench 4,581,973 and scanbench
274,263. The runtime's machine code grows by the search past dead pairs and
the gap closing, 400 bytes in every benchmark binary: text 5,674,129 ->
5,679,729. codegen_instructions_dev reads 116,483,659 -> 116,504,899 and
codegen_instructions_release 370,222,776 -> 370,158,521, and the tab's run
browser_run_instructions 61,369,685 -> 61,465,433. Welfare rises 95.36 ->
95.38 and the rise is banked.

## 2026-10-26 — a refused binding names the line its name is declared on

The 2026-10-25 gavel on #2085 keeps no-shadowing across a module's files and
asks the refusal to name the declaration it collides with. One declared in
another file of the module was already named, with its file and line, since
de0b9aec, and `a_binding_named_like_a_declaration_in_another_file` pins it.
A collision inside one file still said only "`greet` is already a
declaration", so the reader had to find it. It now says "`greet` is already
a declaration, at line 1". A name no file declares, such as `length` from
the library or `done`, keeps the message without a line.

`shadowed_declaration` and `a_binding_may_not_take_a_name_that_resolves_without_it`
carry the new line, as does appendix A's panel. Adding a space to the message
turns both red.

CI's rows, against main at #2155. The rewrite runs only on a refusal, so
what moves is layout: compile_instructions 25,157,249 -> 25,156,620,
entry_instructions 73,813,902 -> 73,811,997, library_instructions
73,484,685 -> 73,482,762 and browser_compile_instructions 359,922,625 ->
359,922,521. Welfare rises by less than a hundredth and the rise is banked.

## 2026-10-26 — `put` writes a list at a position

The 2026-10-25 gavel on #2086 rules that `put xs i v` on a list answers the
list with element `i` replaced, 1-based as `xs[i]` reads it, and `none` for
a position outside the list. A position that is not an int stops the run,
on both engines, with "put writes a list at an int position"; a first
argument that is neither a map nor a list stops it with "put writes a map at
a key or a list at a position", where both used to say "put takes a map, a
key, and a value". A position past a word answers `none`, as a read there
does.

The interpreter replaces the element in a copy of the vector, or in the
vector itself at a site the linearity analysis licenses. The native runtime
does the same in `k_list_put`, which `k_b_put` and `k_b_put_mut` reach on a
list before their map arms. No analysis changed: `put` with three arguments
was already a write into its first argument wherever that argument was
unique, so a list built by in-place pushes and handed down a loop is
written where it stands.

A persistent push hands its answer the buffer it was given when the list is
that buffer's frontier, so two lists can share one buffer. I first had the
in-place write copy a list into a buffer of its own before writing, and
then wrote programs to make the copy matter: a push onto a list still held,
followed by a write into the push's answer, and a function that writes into
a literal on every call. Neither write was in place. The analysis calls a
push's answer unique only when the list pushed onto is, which is the case
where the push was in place and nothing shares the buffer, and a list
literal is built afresh on each evaluation. Both programs print the same
lines with the copy taken out, so I removed it.

Inference answers `LIST | NONE` for `put` on a list, `MAP` on a map, and
both where the first argument could be either. The checker's literal table
takes a list for `put`, and the after-inference walk that refused a list
handed to `put` (#2091's fifth item) now asks only of `delete`, `keys`,
`values` and `entries`, so diff F9 and inflate F8, which passed check and
stopped at run time, now pass both.

`a_list_written_at_a_position` (micro, both engines) writes inside the list,
at 0, one past the end, -1 and a bignum, into a literal, with a list as the
value, into an empty list, through a loop that fills six squares, through
the shared-buffer program and through the literal written on every call.
`a_list_written_in_place_is_not_copied` (mem) fills 4,000 squares into a
list built by pushes and reads allocs=10. `put_writes_a_list_at_an_int_position`
pins the refusal, and `put_takes_a_map_a_key_and_a_value` is renamed
`put_writes_a_map_or_a_list` with the new words. `errors/a_list_handed_to_put`
becomes `a_list_handed_to_delete`, the same two shapes handed to `delete`.
Three mutations turn the corpus red: the native in-place write made a copy
(the mem golden), the persistent write made through the original (the
micro, natively), and the interpreter writing at the wrong position (the
micro, interpreted). Appendix B's `put` entry gains the list signature and a
sample.

CI's rows, against main at #2160. The runtime's machine code grows by
`k_list_put` and the two puts' list arms, 752 bytes in every benchmark binary:
text 5,679,729 -> 5,690,257. The interpreter reads interp_instructions
565,857,214 -> 565,868,501, its `put` arm asking first whether it holds a
list, and interp_allocs 628,011 -> 628,003. The tab reads
browser_run_instructions 61,465,433 -> 61,508,652 and
browser_compile_instructions 359,922,521 -> 359,923,093. The rest move by
layout: compile_instructions 25,156,620 -> 25,156,569, entry_instructions
73,811,997 -> 73,811,949, library_instructions 73,482,762 -> 73,482,714,
codegen_instructions_dev 116,504,899 -> 116,505,396,
codegen_instructions_release 370,158,521 -> 370,153,673, and among the
benchmarks work_basket 18,278,697 -> 18,266,806, work_runbench
822,295,072 -> 822,288,845, work_encodebench 2,327,387,236 ->
2,327,387,586, work_livebench 936,013,522 -> 936,013,844, work_oneshot
10,033,523 -> 10,034,013 and work_readbench 4,581,973 -> 4,581,975. Welfare holds at 95.38 and is banked.

## 2026-10-26 — a failure inside the library names the program line that called in

The 2026-10-26 gavel on #2099 keeps a library failure's own line and adds,
under it, the nearest line of the program's own files that called in:

```
error[runtime]: split takes two strings
  failed in text/split at std/text/text.kso:38
  called from g.kso:11
```

A file is the library's when it is under `std/` or fetched under `.hako/`,
which `provenance::is_library_file` says and the runtime's
`k_library_place` repeats for a site's file. The line is added only under a
library line, so a failure in the program's own code reads as before.

The interpreter adds it as a failure leaves a call written by name, in a
program file, of a group the library declares. The first such call the
failure passes out of is the nearest, and the line rides in the place string
`failed in` already prints, so none of the 108 places that build a
`RuntimeError` changed. Natively the emitter saves `k_from` before each such
call, stores the call's `<file>:<line>`, and puts the saved one back when the
call returns; `k_died` prints it when the failing site's file is the
library's. A program function the library calls back sets and restores its
own, so a failure after the callback names the line that called the library
function, as the interpreter's frames do. The two fused slice doors, which
call no wrapper, save and restore around the door.

A call into the library in tail position is made rather than looped on, on
both engines, as an insisting form already is: a looped call would leave no
program frame to name in the interpreter and nothing to restore natively. A
call to a flattened twin is not counted as one. `flat` copies `text/span_to`
into a twin that carries the program's own continuation, so the twin can be a
step of the program's loop; the first draft made that call an ordinary one,
and `a_span_handed_on_is_never_built` read beat_iters 26 -> 17, the loop no
longer rewinding. Both engines skip twins, and it reads 26 again.

The emitted lines move by the store and its restore: the compile samples
read recursion 470 -> 471, dispatch 370 -> 371, guards 327 -> 328 and
records 426 -> 427 lines, and the module 1,181 -> 1,186.
Calls through a function value and steps a chain word runs are not counted on
either engine. A step that `.>` hands a box is run by the executor after the
line that wrote it has finished.

`a_library_failure_names_the_line_that_called_in` reaches `text/split`
through two program functions, the inner one calling it in tail position.
`a_library_failure_after_a_callback_names_the_outer_line` has `group_by` call
a key function that calls `text/trim`, and then fail inside the library's
own `list/file_under`. `an_operator_failing_in_the_library_names_the_line_that_called_in`
hands `pad_start` a width it subtracts from. Between them they cover the
ruling's three kinds of failure born in the library, a builtin's, a library
function's and an operator's, and all three are in the runtime corpus, which
every engine runs. Nineteen runtime goldens whose
failure is inside the library, and `test_verb/fails_in_std`, gain the line.
Four mutations turn the corpus red: the native restore made a no-op, the
interpreter's addition taken out, and the tail call kept on either engine.

CI's rows, against main at #2161. The interpreter's crossing test first
scanned both files' paths for a fetched hako on every tail call, and
`interp_instructions` read 582,178,759; a frame now carries the answer,
read off its path once, and it reads 571,553,496 against main's
565,868,501, the rest being the calls into the library made rather than
looped. `interp_allocs` reads 628,009 and `interp_peak_bytes` 1,313,667.
Natively the store and restore around each crossing move the run rows:
`work_encodebench` 2,335,195,960, `work_livebench` 938,703,709,
`work_widebench` 28,605,098, `work_deepbench` 198,242,568, `work_basket`
18,294,810, `work_readbench` 4,583,158, `work_jsonbench` 557,387,287,
`work_pendbench` 154,123,901 (down), `work_runbench` 822,286,353,
`work_oneshot` 10,033,544, `work_digestbench` 4,122,972,
`work_escapebench` 40,308,378, `work_indexbench` 1,837,160 and
`work_scanbench` 274,245. The emitter writes the stores:
`emitted_lines` 14,383, `emitted_other_calls` 20,496,
`emitted_other_lines` 170,009, `emit_instructions` 5,725,463, `text`
5,698,479, `lines` 1,597 and `module_lines` 1,186. Compiling them moves
`codegen_instructions_dev` to 116,762,508 and `codegen_instructions_release`
to 371,664,033. In the tab, `browser_compile_instructions` reads
360,520,776, `browser_compile_peak_bytes` 842,026 and
`browser_run_instructions` 61,510,637. The live vein's `cohort_kept` reads
1 where it read 0: a program-to-library tail call is now a call, so the
caller's beat stays open around it, and on its pop the survivor is more
than half of what grew, so the region is kept rather than copied out; with
that one arm of `emit_tail` taken back out it reads 0 again. Welfare is
banked at 95.38, the floor taking what this ruled feature measured.

## 2026-10-26 — gavel: an err is born boxed

Clay ruled #2087. Since #1884 a name bound to a call that can fail carries
the err into the check, and a function it reaches must take it with an
`(err _)` arm. Most of the arms the ports added only hand the err back:
site wrote 68, toml 37 and bc 22, and bc's came in three rounds because each
new arm made its callers need one. A bare err did two jobs that other things
already do. For an outcome the caller handles there are `none` and markers,
and for a failure that travels up there is the box.

**The ruling.** Clay: "maybe you can unbox an error and pass it around but
you can't generate an unboxed error." Every way of making an err makes a
boxed failure. `err "…"` answers an effect that bubbles to a foreign `rescue`
or the endpoint, and no expression produces a bare err. An err taken out of
its box, by `rescue` or by a chain callback handed the failure, is an
ordinary value: it can be passed, stored and dispatched on, and `effect` puts
it back in a box. An outcome the caller is expected to handle is an ordinary
value, `none` or a marker or a record carrying the reason. A plain form
answers one of those, and its `!` form answers the boxed err with its
message. A box cannot be used as a value without `.>`, so a failure still
cannot slip into arithmetic.

This supersedes part 2 of the 2026-09-15 gavel ("a bare err is data") and
the bare err in the 2026-09-16 list of what a non-bang form answers. The
issue's three options were declined in favour of this one.

Owes, on #2087: `err "…"` answers a boxed failure on every engine, and the
checker has no bare-err type left to track; each bare err in the library
classified, a mistake becoming a box and an expected outcome becoming `none`
or a marker on the plain form with the boxed message on the `!` form; an
unboxed err behaving as a value on every engine, with goldens; the ports'
hand-back arms removed, and #2088's migration using the `!` form at reads that
must succeed; ch04, ch05 and appendix B rewritten where they describe bare
errs as data, and the error goldens regenerated.

## 2026-10-26 — the linearity fixpoints drop an entry as its check fails

The linearity analysis settles three fixpoints over parameters, record fields
and map elements. Each collected a lap's refusals and applied them at the
end, so a refusal that cascades through a chain of calls took a lap per link:
`field_fixpoint` ran 18 laps on the interpreted corpus, re-asking the 54
records that had settled by the fourth, and `drop_elems` re-ran the main
fixpoint after each of its laps, nine runs that found nothing new. Each entry
now goes as soon as its check fails, so the next entry asked already sees it
gone. A check only turns from yes to no as the sets lose entries, so a no read
against a larger set is a no against the fixpoint, and the order cannot
change what is left. The emitted IR is byte-identical on 430 programs: the
examples, the micro corpus and every benchmark.

CI's rows, against main at #2162. The compile rows fall: compile_instructions
25,156,569 -> 25,048,015, entry_instructions 73,811,949 -> 73,539,324,
library_instructions 73,482,714 -> 73,209,499, emit_instructions 5,725,463 ->
5,688,182 and browser_compile_instructions 360,520,776 -> 344,137,759. The
interpreter runs the analysis before each program, so interp_instructions
falls 571,553,496 -> 563,210,772, and interp_allocs rises 628,009 -> 630,870,
the lists of entries a drop collects while it walks a set it is changing.
Welfare rises 95.38 -> 95.40 and the rise is banked.

## 2026-10-26 — the runtime's tags and doors are read from its C

Fixes #2130. `build.rs` already joins the runtime's parts, and it now reads
the tag enum and the functions defined with `K_DOORCC` out of the joined
text into `$OUT_DIR/runtime_abi.rs`. The seven tags `codegen.rs` restated,
`K_ERR_TAG`, the door list in `codegen/module.rs`, and the two specs that
held those copies to the C are gone. A door the reader cannot parse stops
the build, since a door missing from the list would be called on the C
convention; the ratchet row `a_door_the_build_cannot_read` breaks one and
watches `cargo build` go red.

CI's rows, against main at #2154. The derived door list is in the runtime's
include order, and `through_doors` checks doors in list order, so
`emit_instructions` reads 5,687,958, 224 fewer. Compiling the runtime reads
the edited comment in `core.c`, which moves `codegen_instructions_dev` to
116,762,560 (52 more) and `codegen_instructions_release` to 371,663,962 (71
fewer). Both read as layout of the clang input, since the program's code
is unchanged, and no runtime row moved.

## 2026-10-26 — a guard inside a bound branch leaves the branch natively

#2166. `return x if c` inside an `if` branch that is bound to a name leaves
the branch: the interpreter binds the name to the early answer and the
function goes on. The native backend refused to build any such program with
"a return guard sits only in tail position", although `kanso check` accepts
it, so the engines disagreed. The emitter now reads a guard for its value as
it reads an `if`: the condition asked as a question, the early answer one arm,
the rest of the block the other, and a phi over the two and any failure the
condition carried out. `a_guard_leaves_the_branch_it_is_written_in` holds
#2166's program with the guard firing and not, in both branches, and prints
`11 10` and `36 18` on both engines; on main the native build stops.

CI's rows, against main at #2149. No program in the corpora has the shape, so
nothing runtime moves, and only the emitter's new arm shows, natively and in
the tab: emit_instructions 5,687,958 -> 5,688,137 and
browser_compile_instructions 344,137,759 -> 344,146,623. Welfare falls by less
than a hundredth, and the floor is lowered by exactly that, since this makes native
build a program the language allows.

## 2026-10-26 — an err is born boxed

The build of the #2087 gavel. `err "…"` and every builtin that fails answer
a failure in a box on all three engines. Natively the box is `K_ERR` and the
err a rescue takes out of it is a new tag, `K_ERRV`; the interpreter has
`Value::Unboxed` for the second. A failure passes every function it reaches
and every arm, the three err patterns included, and the endpoint or a
foreign rescue ends it. A rescue's callback and an annotate's are handed the
unboxed err, which renders as `err <reason>`, compares by reason, and goes
back in a box through `effect`. A parameter annotated with an effect type,
`e:<some>effect`, takes the box itself, through `k_check_box`, which is how
std/testing reads a package's own failures.

The checker loses the raise analysis of #1884: the raised bit, `raised_err_at`,
the raisers table and the err-arm requirement are gone. In their place a
failure counts as a box, so a group whose every answer is a failure, or a
tail that calls `err`, is refused where a value is read, as an io is. A group
that can also answer a value passes, as one that can answer none does.

The library classified its bare errs. `text/to_int` and `text/to_float`
answer none for text that is not a number, and `to_int!` and `to_float!` in
std/text answer the box with the reason they used to give. `json/decode`
answers none and `json/decode!` the box carrying the `parse_failure`.
`text/utf8` keeps its failure. The checker's refusal of a literal that is not
a number moved to the bang forms. The hand-back arms in std/json, std/regexp
and the frozen benchmark copies are gone, since a failure passes them anyway.
std/testing's two functions take the box through an effect-annotated arm.

The corpora followed. Error fixtures pinning the raise analysis were deleted
and two moved to the runtime corpus, where the failure now passes the
operator or group they named. A micro sample that leaned on a plain
conversion failing uses the bang form, or a local function with a mixed arm
so its reason is unchanged. `an_arm_sees_its_own_hakos_err` and
`a_typeset_arm_sees_its_own_hakos_err` pinned the 2026-09-15 rule and now pin
its reversal as `a_failure_passes_its_own_hakos_arm` and
`a_failure_passes_its_own_hakos_typeset_arm`. `.cause` can be read only on an
err another package annotated, so it moved to a two-module golden,
`tests/golden/cause`. ch04, ch05, ch08 and appendix B were rewritten where
they described a bare err as data, and ch08's mini decoders answer their
`parse_failure` record as a value, the shape the ruling gives an outcome the
caller handles. kq and kanso-json carry the same changes on branches of this
name.

`an_err_as_pattern_keeps_a_carried_slot_unboxed` lost its shape: its scanner's
failure now passes the err arm, the loop becomes a beat, and the carry builds
the records whatever the arm does, so the exemption's mutation survived. The
fixture's walk now stops before the scanner fails, and the mutation reads
sh_rec 0 -> 64,000 again. A slot that can hold an unboxed err beside a
two-field record is passed boxed: answering a scanner's end with
`text/to_bytes [300] .? (e -> e)` reads sh_rec=64,064. Nothing in the tree
answers that shape, and the two-word convention could carry `K_ERRV` as it
carries `K_ERR`, which is an open lead.

Open: the tree's scripts still carry dozens of `e@(err _)` hand-back arms
that no longer fire; the ports carry hundreds; #2088's migration can now use
the bang forms at reads that must succeed.

CI's rows, against main at #2168. The checker loses the raise analysis, so
compiling gets cheaper: `compile_instructions` reads 24,604,939,
`entry_instructions` 72,571,754, `library_instructions` 72,232,245 and
`compile_allocs` 13,930, and the front end holds less while it checks:
`compile_peak_bytes` 607,213 -> 598,492, and `front_end_visits` reads 7,397
where it read 7,373. The interpreter no longer carries the raised bit:
`interp_instructions` 560,993,797, `interp_allocs` 630,584 and
`interp_peak_bytes` 1,306,819. In the tab, `browser_compile_instructions`
reads 339,964,747, `browser_compile_peak_bytes` 832,433 and
`browser_run_instructions` 61,425,132. The decoders lost their hand-back
arms, so the emitter writes less: `emitted_lines` 14,150, `emitted_calls`
1,519, `emitted_branches` 1,293, `emitted_other_lines` 168,876,
`emitted_other_calls` 20,311, `emitted_other_branches` 17,723 and `text`
5,619,835, and compiling it moves `codegen_instructions_dev` to 116,733,865
and `codegen_instructions_release` to 371,551,519. `emit_instructions` rises
to 5,689,859, 1,722 more; against main at #2162 it fell 491, and #2154's
fixpoints, which drop an entry as its check fails, sit between the two
readings, so the rise is recorded unisolated. `emitted_defines` rises to 160
and `emitted_other_defines` to 2,243 because `json/decode!` and the bang
conversions are functions of their own beside the plain forms; encodebench
and scanbench each lose one define with their hand-back helper. The run
rows: `work_jsonbench` 549,333,568, `work_runbench` 819,572,053,
`work_oneshot` 9,978,432, `work_livebench` 938,653,995 and `work_scanbench`
274,243 fall. `work_widebench` rises to 28,685,100, 80,002 more: five for
each element of its 16,000, and two besides, for the `written` arm the chunk
now passes, since `text/utf8` can answer none. `work_encodebench` rises to
2,335,339,266 (143,306 more, 0.006%) and `work_readbench` to 4,583,165 (7
more); nothing has isolated either, and they are recorded as moves of that
size. The mem sample `a_failure_crosses_a_beat_carry_in_two_words` imports
std/testing to read its own failure, and the import costs six allocations of
32 bytes: `a_failure_crosses_a_beat_carry_in_two_words_allocs` reads 1,009
and `a_failure_crosses_a_beat_carry_in_two_words_alloc_bytes` 64,352.
`an_err_as_pattern_keeps_a_carried_slot_unboxed` was rewritten, as above,
and reads 3 allocs and 112 bytes.

The ratchet found two rows that prove nothing. `a chain callback's parameter
seeded as never failing` made inference seed a rescue or annotate callback's
parameter without the failure bit; a callback is now handed the err out of
its box, an ordinary value, so the seed protects nothing and the row and its
mutation are gone. Seeding the parameter as never failing is now correct,
and is an open lead. `an encoder that hands a clean string to the escape
path anyway` repointed `escape_onto`'s clean-string arm, which lib/json's
encoder has not called since strings went out through `append_string` in one
call (197f172b), so the row was blind on main too and was only rerun here
because this branch touches lib/json. Its mutation now makes the encoder
copy each string before it writes it: livebench writes the same bytes and
the live vein reads 858,314 allocations where it reads 8,314. `escape_onto`
and its two helpers are dead in the library.

## 2026-10-26 — a none carried through a name is checked

#2088. A none handed straight to a call is refused at check when the
receiver has no arm for it, and the same none bound to a name first passed
check and stopped at run time. Since #1890 a slice past the end answers none,
so the ports hit the hole wherever they slice: regex broke 42 of 43 fixtures
at run time through one named slice. A name bound once to something that
provably answers none now carries the none into the check, so binding a
value never changes what compiles. A builtin has no arms to give a none to,
so the positions its demand table constrains refuse one: `length`, `push`,
`print`, the os builtins and the rest of the table, called directly or
through a name.

The tree had about 130 reads the checker cannot bound, most of them a bound
decided in one function and handed to the next as a boolean, the shape the
house rule asks a branch to take. Each read must succeed, so each now goes
through a `none` arm that answers an err. Since #2087 that err is a box that
passes everything above it, so no caller gains an arm. Most scripts take one
helper, `present`, and the library, the book's escapes sample and five
benchmarks each take a small arm beside the read. A default value in the arm
would have hidden the bug the check exists to catch, and no site takes one.
Where a none was an expected outcome, welfare_rescore and grammar_check name
it with a marker or keep its arm. kq and kanso-json carry the same change on
branches of this name.

Six error fixtures pin the refusal, named and inline, for a group with no
arm, `length`, `push` and `list/concat`, each seen passing check on the
#2087 tree.

The test corpora read the same way. Nine micro samples and nine mem
fixtures route a read through a none arm, and none of their outputs or
counters moved. `a_box_where_a_value_is_expected` gains one leading
diagnostic, since `print os/args[1]` hands `print` a none. The test verb's
`fails_in_std` carries its slice in a record field so the none still reaches
`text/split` at run time, and its called-from line moves from 13 to 18. The
last item on #2088, bc's slice edges, was built on 2026-10-25: a start before
1 answers none, as a range past the end does.

CI's rows against main at #2169. The check costs the front end:
compile_instructions 24,604,939 -> 24,798,984, entry_instructions 72,571,754
-> 73,507,587, library_instructions 72,232,245 -> 73,143,499 and the tab's
browser_compile_instructions 339,964,747 -> 340,825,316. indexbench reads its
bound at the guard on every step, `if (i > length s)`, because the prover
cannot see a bound handed down (#2174), so work_indexbench rises 1,837,160 ->
2,077,157, and work_runbench, which carries the shape at a larger size,
819,572,053 -> 829,234,755. The none arms the library and the benchmarks
gained move the rest by small amounts: work_jsonbench 549,333,568 ->
549,337,021, work_widebench 28,685,100 -> 28,701,099, work_basket 18,294,810
-> 18,294,814, work_encodebench 2,335,339,266 -> 2,335,338,865,
work_scanbench 274,243 -> 274,220 and interp_allocs 630,584 -> 630,592. The
emitted and text rows grow with the same arms: for the decoder
emitted_defines 160 -> 161, emitted_calls 1,519 -> 1,528, emitted_branches
1,293 -> 1,308 and emitted_lines 14,150 -> 14,240; for the other benchmarks
emitted_other_defines 2,243 -> 2,253, emitted_other_calls 20,311 -> 20,472,
emitted_other_branches 17,723 -> 17,764 and emitted_other_lines 168,876 ->
169,582; and text 5,619,835 -> 5,620,636 over every benchmark. The check is the 2026-09-15 gavel's part 3
carried to none, so the floor drops by what it costs, 95.42 to 95.38.

## 2026-10-26 — a length handed down a loop bounds its reads

#2174. A loop that walks a list by index usually hands its bound down once,
`walk xs 1 (length xs) 0`, and guards with `i > n`. The prover had two doors
for the upper bound of `xs[i]`, a guard written against `length xs` and the
length a literal list spells out, and the guard against `n` passed neither,
so a read like `twice xs[i]` was refused unless the loop read `length xs`
again on every step. #2088 makes that the common case, since a name bound to
such a read now carries its none too: bench/indexbench, migrated to `i >
length s`, reads 13% more instructions for it.

`handed_lengths` (`src/infer.rs`) is a third door. It proves that a
parameter holds another parameter's length for a whole arm on the terms
`counter_floors` proves a floor: the group is not pub and never travels as a
value, every call from outside passes `length X` at the bound's position and
the same name `X` at the list's, every self-call passes both on unchanged,
and no arm rebinds either. The arm starts with that fact, and a guard `i >
n` gives the read its upper bound through it.

`a_length_handed_down_bounds_the_read` (micro) prints `20 0` on both
engines, and main refuses it. `a_length_of_another_list_bounds_nothing` and
`a_bound_that_grows_bounds_nothing` (errors) keep the two near misses
refused. The ratchet row `a length handed down a loop proving nothing` drops
the fact and watches the micro corpus go red.

CI's rows against main at #2170. indexbench goes back to handing its bound
down, and work_indexbench falls 2,077,157 -> 1,887,164; work_runbench, which
carries the shape at a larger size, falls 829,234,755 -> 821,299,208. The
interpreter does a little less, interp_instructions 560,993,797 ->
560,933,132. The front end pays for the walk and the facts:
compile_instructions 24,798,984 -> 24,862,623, entry_instructions 73,507,587
-> 73,686,908, library_instructions 73,143,499 -> 73,325,258, compile_allocs
13,930 -> 13,941, emit_instructions 5,689,859 -> 5,702,866 and the tab's
browser_compile_instructions 340,825,316 -> 342,497,302. interp_allocs moves
630,592 -> 630,614, and nothing here isolates why. The emitted and text rows
fall with the guard's `length` call: emitted_other_calls 20,472 -> 20,468,
emitted_other_branches 17,764 -> 17,762, emitted_other_lines 169,582 ->
169,576 and text 5,620,636 -> 5,620,459. Welfare rises 95.38 -> 95.41 and the
rise is banked.

## 2026-10-26 — a list read for its last element joins the corpus

A battery of list shapes, each run at 5,000 and 20,000 elements on both
engines, found `list/last` read in a loop quadratic on both. `last` folds
the whole collection to find its last element, so each read costs the
length of the list: 5,000 reads of a 5,000-element list took 0.38 seconds
natively and 17.7 on the interpreter, and at 20,000 they took 4.6 seconds
natively, where the interpreter ran past two minutes. The other shapes,
`text/join`, `list/zip`, `list/count`, `list/reverse`, `list/sort_by`,
`list/tally`, `list/to_h`, `text/to_int` over a list and floats rendered by
interpolation, grow linearly or by n log n on both.

Under the 2026-09-05 corpus-first gavel `lasted` joins the run program with
800 reads and the interpreted corpus with 70, measured on main at #2158
before its fix. On this container the run program reads 865,303,604 with it
and 828,684,309 with it reading nothing, a column of 36,619,295 (4.23%), and
the interpreted corpus reads 565,857,214 -> 593,093,849. Both engines print
the same line for it.

The battery also found a field read that the engines disagree on: native
reads `e.value` from an entry, and the interpreter refuses it. #2163 asks
Clay which way it goes.

CI's rows, against main at #2175. The run program reads work_runbench
821,299,208 -> 859,187,448, and its allocation vein moves with the 800 reads:
run_allocs 1,581,589 -> 1,582,393, run_alloc_bytes 223,888,104 ->
223,955,352, run_beat_iters 177,272 -> 178,072, run_push_mut_slow 1,644,657
-> 1,645,457, run_sh_rec 4,098,336 -> 4,136,736, run_sh_buf 91,336,688 ->
91,336,800, run_perm_peak_bytes 1,867,025 -> 1,875,217, run_bytes_malloc
7,921 -> 7,924 and run_bytes_freed 7,779 -> 7,782. The interpreted corpus
reads interp_instructions 560,933,132 -> 588,905,757, interp_allocs 630,614
-> 656,976 and interp_peak_bytes 1,306,819 -> 1,327,257. The tab compiles
and runs the corpus too: browser_compile_instructions 342,497,302 ->
348,067,219, browser_compile_peak_bytes 832,433 -> 836,713 and
browser_run_instructions 61,425,132 -> 64,341,530. The run program's new
function is emitted code: emitted_other_lines 169,576 -> 170,115,
emitted_other_branches 17,762 -> 17,826, emitted_other_calls 20,468 ->
20,537, emitted_other_defines 2,253 -> 2,262, and text 5,620,459 ->
5,622,379. Under the corpus-first gavel each moved welfare term's baseline
is scaled so its ratio to what it reads holds, so the phase leaves welfare
where it was and the fix in #2165 scores as a rise.

## 2026-10-26 — the last element of a list is read at its end

`list/last` gains an arm for a list, `last xs:[]some`, which answers
`xs[length xs]`: the last element, or none for an empty list, as the fold did.
Anything that is not a list keeps the fold, so a lazy sequence, a string,
bytes and a map are walked to their last element as before.

5,000 reads of a 5,000-element list took 0.38 s natively and 17.7 s on the
interpreter, and take 0.07 s and 0.015 s. At 20,000 they take 0.09 s and
0.03 s, where the interpreter ran past two minutes.

`a_list_is_read_at_its_end` (micro, both engines) reads the last of a list of
numbers, of records and of lists, of one element and of none, and of a lazy
sequence, a string, bytes and a map. Its output is main's interpreter, and
both engines on main and on this branch print it. Reading one before the end
turns the micro corpus red.

The new arm moves std/list's lines below it down by four, so
`runtime/tie_takes_a_list_of_ids` names `list/tie` at line 602 where it named
598.

CI's rows, against #2164's phase on main at #2175. The run program's 800
reads cost what an index costs: work_runbench 859,187,448 -> 821,243,366,
below main's 821,299,208 without the phase, run_allocs 1,582,393 ->
1,581,593, run_alloc_bytes 223,955,352 -> 223,916,952 and run_sh_rec
4,136,736 -> 4,098,336, the 800 step records the fold built. The interpreted
corpus falls the same way: interp_instructions 588,905,757 -> 562,047,199
and interp_allocs 656,976 -> 631,738, while interp_peak_bytes reads
1,328,317, 1,060 more, for the library's extra arm. In the tab
browser_run_instructions falls 64,341,530 -> 61,478,759. The arm is library
code every compile reads, so the compile rows rise a little:
compile_instructions 24,862,623 -> 24,864,430, entry_instructions
73,686,908 -> 73,696,160, library_instructions 73,325,258 -> 73,344,720,
emit_instructions 5,702,866 -> 5,703,136, codegen_instructions_dev
116,733,865 -> 116,733,909, codegen_instructions_release 371,551,519 ->
371,551,607, browser_compile_instructions 348,471,187 and
browser_compile_peak_bytes 843,030. The run program emits the arm too:
emitted_other_lines 170,150, emitted_other_calls 20,540,
emitted_other_branches 17,831 and emitted_other_defines 2,262, and text
reads 5,622,043. Welfare rises 95.41 -> 95.52 and is banked.

## 2026-10-26 — a string handed on round a cycle joins the corpus

The corpus phase for the next fix, under the 2026-09-05 corpus-first gavel.
A battery of core-library shapes at 4,000 and 16,000 elements on both engines
found one cliff: natively, `regexp/replace_all` over 64,000 words took 49.8
seconds, where the interpreter took 4.8. Three separate causes sit under it,
each reduced to a loop of two groups that joins onto a string in place in the
first group and hands it to the second:

- `built` hands the join straight on, `built s (at + 1) "{acc}{s}x" n`.
- `marked` binds the join below a `return acc if …` guard, and the guard holds
  the statements after it.
- `tagging` is also declared with one parameter, as regexp's `swapped` is.

Natively, each hop copies the string built so far, so each loop costs the
square of its length: 80,000 steps of `built` took 13.9 seconds, of `marked`
11.4 and of `tagged` 2.3. The interpreter asks the same analysis and has the
third cliff too, at 1.57 seconds for 80,000 steps of `tagged` against 0.10
for 20,000, so `relayed 1500` joins the interpreted corpus: on this
container it reads 580,418,637 with it and 563,943,189 with it reading
nothing, a column of 16,475,448 (2.84%). The run program's module is
`bench/runbench/runbench/relayed`, at 3,000 steps for the first two and
1,500 for the third. On this container the run program reads 837,824,222
with it and 822,287,654 with it reading nothing, a column of 15,536,568
(1.85%). The run program's allocation vein moves with it. Welfare re-bases
each moved term once CI's rows are in, so the fix scores as a rise.

CI's rows, against main at #2165. work_runbench reads 821,243,366 ->
836,794,013 with the phase, and its allocation vein moves with the three
loops: run_alloc_bytes 223,916,952 -> 282,819,050, run_allocs 1,581,593 ->
1,596,597, run_arena_blocks 4 -> 5, run_arena_peak_bytes 4,718,608 ->
5,767,184, run_spare_peak_bytes 2,621,456 -> 3,670,032, run_beat_iters
178,072 -> 193,074, run_bytes_malloc 7,924 -> 13,926, run_evac_allocs 1,732
-> 1,744, run_evac_bytes 4,602,176 -> 4,635,024, run_perm_allocs 103 -> 106,
run_sh_str 16,769,952 -> 21,455,984, run_str_scan_bytes 18,352,322 ->
18,358,325 and run_str_scans 1,885 -> 1,888. The interpreted corpus reads
interp_instructions 562,047,199 -> 579,993,703, interp_allocs 631,738 ->
653,569 and interp_peak_bytes 1,328,317 -> 1,339,129, and the tab compiles
and runs it: browser_compile_instructions 348,471,187 -> 353,823,116,
browser_compile_peak_bytes 843,030 -> 847,565, browser_run_instructions
61,478,759 -> 64,252,748 and browser_run_peak_bytes 1,572,864 -> 2,621,440,
a page more for the copies. Under the corpus-first gavel each moved welfare
term's baseline is scaled so its ratio to what it reads holds, so the phase
leaves welfare where it was and the fix below scores as a rise.

## 2026-10-26 — a builder handed on round a cycle is still a builder

The fix for the corpus phase above. A string joined onto itself is a builder,
and the emitter converts a seed where it enters the group from outside, which
copies it. Three hops counted as entering when the string they carried was
the builder the join had just made:

- A join handed straight to the next group as an argument. The emitter knew
  a name bound to a join was carrying the builder, and asked nothing of the
  join itself.
- A join bound below a `return … if` guard. The guard holds the statements
  after it, and the walk for names bound to a join read only the function's
  own list.
- A group whose name is also declared at another count. A call at another
  count read as the group handed on as a value, called from somewhere no call
  site describes, so no caller could be proved to hand the string over. A
  call whose count is another declared group's calls that group, and only a
  count no group has still counts as the group escaping.

regexp's `replace_all` also handed `stepped_over` its accumulator beside the
join made from it, so the accumulator was read twice and could not be joined
in place; `stepped_over` never read it, and the argument is gone. Over 64,000
words `replace_all` took 49.8 seconds natively and takes 0.15. Each of the
three loops in `relayed` took between 2.3 and 13.9 seconds at 80,000 steps and
takes under 0.01.

Three mem fixtures pin the copies, each red with its own change undone and
green with the other two undone:
`a_join_handed_on_as_an_argument_is_still_a_builder` and
`a_join_below_a_guard_is_still_a_builder` read 198 bytes where main reads
5,337, and `a_name_at_two_counts_keeps_its_builder` 196 where main reads
1,936. `a_digest_holds_every_block_it_walked` hands a join on as an argument
too, and its vein falls: `allocs` 266 -> 123, `alloc_bytes` 18,657 ->
14,090, `bytes_malloc` 81 -> 10 and `sh_str` 1,344 -> 192. Three regexp
fixtures move too. `a_scan_keeps_its_place_in_the_text` reads `alloc_bytes`
106,537 (from 108,672) and `a_scan_that_finds_nothing_keeps_nothing`
2,781,801 (from 2,783,936), both with the argument change alone;
`a_split_stops_where_the_separator_does` reads 24,487 (from 24,639), and
undoing either the argument change or the count change puts part of that
back.

CI's rows, against #2171's phase on main at #2165. The run program reads
work_runbench 836,794,013 -> 825,009,976, 3,766,610 above main without the
phase, which is the three loops' linear work. The interpreted corpus reads
interp_instructions 579,993,703 -> 577,237,316: at 1,500 steps the copies
were 2,756,387 of the phase's 17,946,504, and `tagged` now grows linearly on
the interpreter as well, 0.04 s at 20,000 steps and 0.11 s at 80,000 where
main takes 0.12 s and 1.25 s. interp_allocs reads 653,378 and interp_peak_bytes 1,356,556. Natively the run
vein falls to run_alloc_bytes 224,043,124, run_allocs 1,582,921,
run_bytes_malloc 7,850, run_evac_bytes 4,602,208 and run_sh_str 16,816,384,
while run_beat_iters reads 193,074, run_perm_allocs 106, run_str_scan_bytes
18,358,325 and run_str_scans 3,387, a scan per lap where the copy used to
stand. regexp's argument change moves work_scanbench 274,220 -> 266,360 and
work_digestbench 4,122,972 -> 4,103,700, and their veins with it:
scan_allocs 149 -> 91, scan_alloc_bytes 11,069 -> 8,934, digest_allocs
7,194 -> 7,051 and digest_alloc_bytes 697,073 -> 692,506. The three
analyses cost the front end a little: compile_instructions 24,864,430 ->
24,873,029, entry_instructions 73,696,160 -> 73,712,018,
library_instructions 73,344,720 -> 73,349,447 and emit_instructions
5,703,136 -> 5,715,797, and the tab's browser_compile_instructions reads
353,650,925, browser_compile_peak_bytes 847,746 and browser_run_instructions
64,598,033. The emitted rows grow
with the new module: emitted_other_defines 2,262 -> 2,276,
emitted_other_calls 20,540 -> 20,700, emitted_other_branches 17,831 ->
17,925, emitted_other_lines 170,150 -> 171,135 and text 5,622,043 ->
5,624,571. Welfare rises 95.52 -> 95.55 and is banked.

## 2026-10-26 — lib/json drops the escape its encoder stopped calling

#2173. `escape_onto`, `escape_clean` and `escape_split` in lib/json had no
caller once a string value went out through `builtin_append_string`, and
nothing in this repository, kq or kanso-json calls `json/escape_onto`. They
are gone. The two ratchet mutations that wrote a string and a key the old
way now append the old escape to the library before calling it, and each
still turns its row red. A codegen comment and a runtime fixture that named
the escape describe what is still there.

CI's rows, against main at #2172. The library every compile reads is
smaller: compile_instructions 24,873,029 -> 24,871,095, entry_instructions
73,712,018 -> 73,693,860, library_instructions 73,349,447 -> 73,331,088,
compile_allocs 13,941 -> 13,940 and browser_compile_instructions
353,650,925 -> 353,638,408, browser_compile_peak_bytes 847,746 -> 847,738.
The interpreter does 62 instructions less, interp_instructions 577,237,254,
and one allocation less, interp_allocs 653,377. work_runbench moves
825,009,976 -> 825,010,172, 196 more; the run program never called the
escape, and nothing here isolates what moved it. Welfare rises within its
second decimal and is banked.
