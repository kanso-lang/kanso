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


## 2026-10-22 — five JSON decoder states skip a blank run out of line

Each state of the JSON decoder read whitespace with an arm that called the
state again on the next byte, so every state was a loop. Five of them
(`array_delim`, `array_open`, `obj_delim`, `obj_key_end`, `obj_open`) now skip
the run with `skip_ws` and read the byte after it with a second group of the
same arms and no blank one, so they no longer loop. `obj_key_start` keeps the
arm that calls itself.

The lead came from the per-instruction profile of the stacked tree. Each
decoder frame loaded three constants in its prologue that its hot path never
used: a `movabs` and two `lea`s, about four instructions a call. Turning off
LLVM's machine LICM at the LTO link removed them and read the run program
935,901,767 -> 927,399,704 (-0.91%), but as a global flag it was a trade:
jsonbench -1.98% and encodebench -2.27%, against escapebench +3.20% and
widebench +1.35%, whose real loops lost their hoisting. It was declined. The
narrower LICM switches (`-disable-postra-machine-licm`,
`-sink-insts-to-avoid-spills`, `-disable-hoisting-to-hotter-blocks=all`) moved
the run program by less than half a million instructions either way.

Taking the loops out of the states moved far more than the hoisted constants.
LLVM folds the decoder's states into one another differently once they stop
looping, and the run program fell 39.4M with all six states out of line. The
effect of each state was measured by looping it alone against all six out:
`array_delim` +13.8M, `array_open` +26.6M, `obj_open` +1.9M, `obj_delim` +1.2M,
`obj_key_end` -4.6M and `obj_key_start` -18.1M. The effects do not add: the
two key states looped together read +13.6M, and with `obj_delim` as well
+22.4M. The shape taken is the best one measured. A cycle-free `value_for`
(its eight arms copied into a second group) moved the run program by 1,239
instructions and added 14 KB of machine code; it was dropped.

Measured here, every benchmark in the instructions gate: the run program
935,901,781 -> 878,353,218 (-6.15%), jsonbench 682,003,932 -> 599,977,782
(-12.03%), oneshot 11,118,560 -> 10,571,976 (-4.92%) and livebench
1,033,423,752 -> 1,032,876,960 (-0.05%); the other ten move by fourteen
instructions or less. The output is the same.

The decoder's emitted code rises by the five second groups and their twins:
emitted_defines 136 -> 157, emitted_calls 1,394 -> 1,557, emitted_branches
1,307 -> 1,346 and emitted_lines 13,134 -> 14,330. oneshot, livebench and
runbench rise by the same 21 defines and 163 calls, so over the other
benchmarks emitted_other_defines 2,154 -> 2,217, emitted_other_calls 19,088
-> 19,577, emitted_other_branches 16,894 -> 17,011 and emitted_other_lines
158,320 -> 161,908. No allocation counter moves.

CI's rows: work_runbench 935,902,128 -> 878,353,565 (-6.15%), work_jsonbench
682,004,279 -> 599,978,129 (-12.03%), work_oneshot 11,118,893 -> 10,572,323
(-4.92%) and work_livebench 1,033,424,099 -> 1,032,877,307 (-0.05%). The
machine code grows where the decoder is linked in: text 5,223,614 ->
5,340,654 over every binary, runbench 721,769 -> 750,713, jsonbench 372,697
-> 402,057, oneshot 379,225 -> 408,601 and livebench 382,761 -> 412,121.

The compiler pays for the new groups in every program that imports
`std/json`: compile_instructions 23,736,301 -> 25,221,632 (+6.26%),
entry_instructions 74,485,199 -> 74,568,239, library_instructions 74,147,760
-> 74,230,433, compile_allocs 13,179 -> 13,789, compile_peak_bytes 565,175 ->
598,984 and front_end_visits 6,914 -> 7,251. The interpreter's corpus
reads interp_instructions 384,228,496 -> 385,357,985, interp_allocs 400,588
-> 401,539 and interp_peak_bytes 608,798 -> 637,429. codegen_instructions_dev,
codegen_instructions_release and emit_instructions do not move.
In the tab, browser_compile_instructions 402,486,449 -> 428,211,402 (+6.39%)
and browser_compile_peak_bytes 735,005 -> 768,853 rise with the compiler's
share, and browser_run_instructions 31,370,376 -> 30,540,337 falls. The
welfare sum rose, and the floor moves 92.53 -> 92.73.

The micro golden `a_blank_run_in_json_is_read_by_each_state` puts a run of
blanks before every byte each state can meet, refusals included, and prints
each refusal's position. Main's decoder prints the same 23 lines. The ratchet
row `a_blank_run_looped_at_the_comma` gives `array_delim` its looping arm back,
which reads the run program 13.8M higher, and turns the instructions vein red.

The benchmark documents hold almost no blanks outside their strings, so the
run program's document was indented two spaces a level (458,223 bytes,
271,763 of them blanks) and decoded once: 16,561,895 -> 16,456,195 (-0.64%).

Also measured and declined: `k_render_number` tests whether a double is
integral with `floor`, a PLT call; a cast round trip in its place read the run
program +181,613. The `floor` it calls here is glibc's `__floor_sse41`,
which the profile counts at 1.5 instructions a call.

Also measured and declined, closing #2011: the span's fast path worked out the
value of the number it found and kept it, keyed by the sixteen bytes from the
number's first, its length and which conversion it answers, and the
conversion doors answered from it. Every number hit. `k_b_to_float_slice_raw`
fell 26,482,302 -> 5,504,994 and `k_b_to_int_slice_raw` 13,492,972 ->
6,464,568, but `k_b_number_span_raw` rose 24,644,367 -> 36,078,570 and the
float fill cost 16,259,562, and the run program read 935,901,781 ->
935,589,998. With the int fill out of line it read 939,328,415. The
conversion's arithmetic costs about the same in the span as in the door.

## 2026-10-22 — an escaped JSON string is read in one walk

A JSON string with an escape in it was read twice. `plain_string`'s emitted
twin found the first backslash, then walked to the closing quote with a
`find2` from each backslash, and answered the quote's position; the decoder
then called `unescape` on the same range, which read the bytes again to undo
the escapes. The twin now hands the first backslash to `k_json_escaped`, which
undoes the escapes while it looks for the quote. It writes the undone bytes at
the arena's frontier, and at the quote it allocates the string `unescape`
would have allocated, at the same length, which lands where the bytes already
are; so no allocation counter moves. It keeps the string, keyed by the bytes'
address and the range, and `unescape` answers it first when asked for that
range, before a frame. A `u` escape, an escape JSON does not have, or a
frontier without room leaves nothing kept, and the old walk finds the quote.

The fuzzer was built first, against the joined runtime with its `main`
renamed, so it ran the real functions. Its reference is the walk the emitter
wrote before followed by `k_b_unescape_raw`. Half its calls cut the arena's
frontier short, so every fallback runs. Unstressed it ran 48,354,448 escaped
strings and found 19 failures: when the frontier ran out in the middle of a
run, the walk handed a plain byte's position to `k_json_quote_after`, which
takes a backslash's and skips the byte after it. The stress found the same bug
2,952 times in 2,000,000 cases. With `k_json_quote_from` for that case, 25
seeded runs of 2,000,000 cases each read 48,354,997 escaped strings, 8,013,430
of them answered from the kept string, and found no failure.

Measured here, the run program 878,353,521 -> 871,922,860 (-0.73%) with the
same output. `k_b_unescape` fell 38,534,463 -> 3,335,013, and the decoder
frames that inlined the twin lost the rescan loop (`array_items` -6,489,450,
`parse_value` -5,370,849, `obj_key_start` +929,412), against 39,591,486 in
`k_json_escaped`. find2_calls falls where the walk no longer calls
`find2`: run_find2_calls 1,488,669 -> 1,037,031, the decoder's find2_calls
2,255,550 -> 1,571,250, oneshot_find2_calls and live_find2_calls 15,037 ->
10,475. The decoder's emitted code sheds the loop: emitted_branches 1,346 ->
1,344 and emitted_lines 14,330 -> 14,319; every program gains the new door's
declare line, so emitted_other_lines rises by one a program where the decoder
is absent.

CI's rows. run_instructions 878,353,565 -> 871,922,904 (-0.73%), and the
other programs that decode an escaped string fall with it: jsonbench
599,978,129 -> 593,998,829, oneshot 10,572,323 -> 10,531,404, livebench
1,032,877,307 -> 1,032,837,487, encodebench 2,456,796,762 -> 2,456,796,258.
Machine code: the programs with the decoder shrink (jsonbench text 402,057 ->
401,417, runbench 750,713 -> 750,041, livebench 412,121 -> 411,481, oneshot
408,601 -> 407,961) and every other program grows by 1,504 bytes
(encodebench 342,473 -> 343,977). Building the run program:
codegen_instructions_dev 116,068,667 -> 116,128,678,
codegen_instructions_release 422,581,877 -> 422,378,106, and
emit_instructions 6,435,278 -> 6,432,635. The machine-code total, text, is
5,340,654 -> 5,353,102. The compile corpus's programs carry the declare line
too: lines 1,524 -> 1,528 over its four, and module_lines 1,163 -> 1,164. In the tab, which carries the same
library: browser_compile_instructions 428,211,402 -> 428,034,601,
browser_compile_peak_bytes 768,853 -> 768,205, browser_run_instructions
30,540,337 -> 30,427,332. The welfare floor rises 92.73 -> 92.76.

The micro golden `an_escaped_string_is_undone_in_one_walk` puts escapes at
the front, the back and each side of a word boundary, two backslashes
together, a document that ends a byte after the string, a `u` escape, an
unknown escape and two strings that run out. The interpreter prints the same
26 lines, and an `\n` undone as a tab turns it red. The ratchet row
`an_escaped_string_undone_twice` keeps nothing, so `unescape` reads every
string again; the run program reads 910,139,124.

Measured on #2043's shape and declined. Looping one more decoder state as
well as `obj_key_start` reads higher every time: `obj_delim` 901,567,246,
`obj_open` 882,623,604, `array_open` 889,848,391, `obj_key_end` 910,134,560,
`array_delim` 892,170,102, against 878,353,218. Dropping the five `_past`
helpers by binding the skipped position in each blank arm cut the corpus check
38,158,594 -> 37,784,461 and read the run program 930,125,639. Three encoder
shapes read higher too: the list's body written in `encode_onto`'s arm
885,700,026, the map's 885,870,393, and the first element folded into
`encode_items` behind a separator group 885,304,273.

Also declined: an int of up to eight digits read inline by `to_int`'s slice
twin, through a hot-unit helper LTO folds in, with the call kept for every
other shape. `k_b_to_int_slice_raw` fell 13,492,972 -> 640,000, but the
inlined read cost 50 instructions an int in `array_items` and `parse_value`,
against 65 for the call, and the run program read 878,353,521 -> 876,146,778
(-0.25%), before what inlining it at every site costs the release build.

A container that restarted came up with clang 18, where the probe for
`preserve_none` answers no and the emitter writes another convention: main
emitted 2,135 fewer lines for jsonbench there than its golden says. The rows
above were taken under clang 19, as CI installs it.

## 2026-10-22 — the region row has a witness again

The ratchet row `a region that keeps what it made` went blind when std/json
began writing a map from its entries in place (c2b48e57). Its mutation makes
`k_region_pop` keep everything the region allocated, and its witness was the
mem fixture `a_nested_map_gives_back_its_entries`, whose arena peak doubled
under it. Once the encoder stopped building `keys` and `values` for each map,
its calls had no allocating argument, so the emitter made no region at all:
no program in the benchmark corpus or the mem vein calls `k_region_pop` now,
and with the mutation applied the whole mem corpus stayed green on main. The
row came up on #2047, whose diff touches `core.c`, and read BLIND.

The new mem fixture `a_descent_gives_back_the_keys_it_built` keeps the
encoder's old shape: a walk that builds a map's keys as the argument of its
call into the map and answers bytes. The emitter makes that call a region.
It reads arena_peak_bytes 8,388,608 and arena_blocks 8; with the mutation,
12,582,912 and 12, and chain_finds 0 -> 9. The interpreter prints the same
total and the oracle's counters agree. The old fixture's comment, which still
described the region it no longer has, says what it pins now.

## 2026-10-22 — JSON escapes are written and undone sixteen bytes at a time

The encoder writes a string with an escape in it through
`k_json_escape_into`, which copied the bytes before each escape a word at a
time and then a byte at a time. The strings the run program writes this way
average about 23 bytes with two or three escapes each, so most runs were
shorter than a word and went through the byte loop at nine instructions a
byte. The writer now reads sixteen bytes at a time on x86-64 and arm64,
stores all sixteen, and finds the first quote, backslash or byte below a
space with three compares and a mask. A string's last block is read through
`k_tail_window` when it is shorter than sixteen bytes, as the scanners read
theirs, and the bytes past its end are ignored. Where the window would cross
a page, and on targets without the vector compares, the word and byte walk
still runs; it is out of line, so its constants no longer take registers in
the block loop. The writer leaves sixteen bytes of slack past its worst case
instead of eight.

The first version clamped every block to the string's end. With that
bookkeeping on each block the run program read 870,530,776; with the clamp
only on the last block it read 864,531,687. Moving the stack buffer the
writer uses for a builder without room into its own cold function took the
792-byte frame off every call.

The decoder's `k_json_escaped`, from the previous entry, copies the run
after each escape to the next quote or backslash. The run averages about
five bytes on the run program, and the word loop took one word and most of
another. It now reads a block of sixteen while sixteen bytes of the document
remain, and the word loop finishes the last fifteen. The run program read
864,531,687 -> 859,100,922 (`k_json_escaped` 39,591,486 -> 34,160,742).

Two more doors take preserve_none, measured one at a time as the first four
were: `append_string_mut` -2,228,335 and `to_float_slice_raw` -1,113,877.
`to_float_slice_raw` is the door the decoder's number twin calls when the
tags are plain; `to_float_slice`, on the list since the four were chosen, is
now its fallback. `to_int_slice_raw` read +472,341 and stays on the C
convention.

Measured here, `k_b_append_string_mut` fell 39,182,580 -> 31,498,560, and
the run program printed the same output.

CI's rows. run_instructions 871,922,904 -> 855,758,740 (-1.85%), and the
programs that encode or decode escaped strings fall with it: jsonbench
593,998,829 -> 584,083,829, livebench 1,032,837,487 -> 990,759,394, oneshot
10,531,404 -> 10,361,059. Two rose by amounts this entry does not account
for: work_encodebench 2,456,796,258 -> 2,456,797,378 (+1,120) and
work_readbench 4,582,072 -> 4,582,081 (+9). Machine code shrinks in every program,
2,544 bytes in most and 2,432 to 2,480 in the four with the decoder; text
5,353,102 -> 5,317,806. Building the run program:
codegen_instructions_dev 116,128,678 -> 116,141,087,
codegen_instructions_release 422,378,106 -> 422,507,483, and
emit_instructions 6,432,635 -> 6,433,837. The tab's runtime is built for
wasm32, where the writer has no blocks and takes the walk:
browser_run_instructions 30,427,332 -> 30,460,236 and
browser_compile_instructions 428,034,601 -> 428,036,106.

append_fast rises where a builder grows for the wider slack: the writer
reserves sixteen bytes past its worst case instead of eight, so a builder
short of that grows once on the out-of-line path and later appends find
room. run_append_fast 5,328,540 -> 5,329,260, live_append_fast 23,682,400 ->
23,685,600 and oneshot_append_fast 59,206 -> 59,214; in the mem vein,
`a_literal_appended_across_a_rewind` 2,120 -> 2,200,
`a_nested_map_gives_back_its_entries` 156,014 -> 156,026 and
`a_written_map_allocates_no_columns` 12,007 -> 12,015. With the old reserve
the first reads 2,120 again. The welfare floor rises 92.76 -> 92.83.

The fuzzers were built against the joined runtime with its `main` renamed,
so they run the real functions.
- The writer's reference is a byte-at-a-time escaper written from lib/json's
  rules. Every entry is driven: `append_string`, `append_escaped` and
  `append_key`, in place and copying, into builders with and without room.
  A quarter of the strings end on the last byte before an unmapped page.
  25 seeded runs of 2,000,000 cases each found no failure, 12,497,738 of the
  cases ending at the unmapped page. Without the window test it faults at
  that page, and without the clamp to the string's end it reads 54,373
  failures in 200,000 cases.
- The decoder's walk ran under the previous entry's fuzzer, arena stress
  included: 25 seeded runs of 2,000,000 cases each, 48,354,997 escaped
  strings, no failure. Reading blocks to within eight bytes of the end, which
  takes bytes past it, reads 13 failures in 500,000 cases.
- Both ran again cross-compiled for arm64 under qemu, where the NEON blocks
  run: the same 25 seeded runs of 2,000,000 cases each, no failure in
  either. Without the NEON block's control-byte compare the writer reads
  72,894 failures in 100,000 cases, and without its backslash compare the
  undo walk reads 33,697.

The micro golden `a_string_is_escaped_sixteen_bytes_at_a_time` decodes and
encodes again strings with escapes just before, on and just after a
sixteen-byte boundary, control bytes, a key and two-byte characters. The
interpreter prints the same twelve lines. With the control-byte compare
taken out of the block, the native build writes raw control bytes and the
golden goes red. The ratchet rows `an_escaped_string_written_a_word_at_a_time`
and `an_escaped_string_undone_a_word_at_a_time` take the blocks away from
each walk.

Declined: the block-less walk answering its position and letting the caller
move the output by the same amount, instead of writing through a pointer to
it. The run program read 865,760,286 against 864,531,687.

Also declined: carrying a joined string's character count into a string
built by a template. The 142 `length` scans on the run program are not
joins. Ninety are `length` on the encoder's output, counted 16 bytes a
compare at about a third of an instruction a byte, and making the builder
count as it appends costs more than the scan.

Also measured and not built: storing the site a death names as a small
index with an immediate, where the address of its literal now costs a `lea`
and a store. A prototype that stamped only the index read the run program
855,758,710 -> 853,297,005 (-0.29%). Building it would give every twin that
takes a site an index beside its origin pointer, and 4,374,601 stores were
the most it could have saved.

Kanso#2024 is closed. Its first lead shipped in #2025 and its third with
`append_key`. Of its second, 8,091,439 overflow checks still execute. Most
guard arithmetic on ints with no bound inside their own group.

## 2026-10-22 — a string index the checker proves goes straight to the text

Where the sets prove the container a string and the position an int, the
emitter now calls `k_b_at_str` with the string's pointer and the position.
It is the string arm of `k_b_at`, lifted out, and `k_b_at` calls it too. The
site used to emit a bytes test that a string never passes, then call
`k_b_at` with two KValues, which tested both tags again, then merge the slow
answer back with a select that rebuilt the byte form. The arm answers a
one-character string or `none`, and nothing in it stops the program but
running out of memory, so the site stamps no place for a death.

Measured here on #2047's build, the run program 855,758,710 -> 844,029,230
(-1.37%) with the same output. Every index in it of this shape is the index
phase's walk, 690,000 calls: the door fell 40,070,237 -> 34,500,017
(`k_b_at` against `k_b_at_str`), and `tally_4`, where the walk is inlined,
fell 22,887,971 -> 16,677,970 without the bytes test and the merge.

CI's rows. run_instructions 855,758,740 -> 844,029,138 (-1.37%), and
work_indexbench, which is the same walk alone, 2,326,925 -> 1,986,925
(-14.6%). Three moved by amounts this entry does not account for:
work_encodebench 2,456,797,378 -> 2,456,796,006, work_oneshot 10,361,059 ->
10,361,367 and work_livebench 990,759,394 -> 990,759,429. The emitted code
of the two programs with the walk shrinks: indexbench's branches 48 -> 42
and lines 787 -> 756, runbench's branches 6,451 -> 6,445 and lines 57,875 ->
57,843, so emitted_other_branches and emitted_other_lines fall. Machine code
grows by 1,040 bytes in most programs and 960 and 992 in the two with the
walk, since `k_b_at_str` is a function beside `k_b_at`: text 5,317,806 ->
5,332,238. Building the run program: codegen_instructions_dev 116,141,087 ->
116,162,655, codegen_instructions_release 422,507,483 -> 422,568,857, and
emit_instructions 6,433,837 -> 6,434,680. In the tab:
browser_compile_instructions 428,036,106 -> 427,996,730,
browser_compile_peak_bytes 768,205 -> 768,243 and browser_run_instructions
30,460,236 -> 30,450,747. The welfare floor rises 92.83 -> 92.88.

Declined: giving the index twin `k_b_at_fast` the same string arm, for
indexes whose sets do not prove a string. Alone it read the run program
-8,405, against a branch and a call at every unproven index site.

The micro golden `a_string_index_the_checker_proves_goes_straight_to_the_text`
walks strings of one- to four-byte characters forward, which steps the
cursor, and backward, which seeks, with positions past both ends. The
release build, which the corpus runs, takes `k_b_at_str` at both walks. With
the cursor stepping one byte instead of one character the walk splits the
wide characters and the golden goes red. The ratchet row
`a_proven_string_index_through_the_tag_tests` takes the arm away.

## 2026-10-23 — join keeps its list and separator in locals

`k_b_join` cost 12,305,923 instructions in the run program, about 61 for each
of its 200,000 pieces, most of them in the pend phase's `text/join digits " "`.
The copying loop read the list's length and items, and each piece's length,
through their structs after every copy, because a store through `char*` may
alias them; each piece's length was read twice. A one-byte separator went
through `k_copy_short`'s size ladder, five compares to store one byte. The
summing loop added the separator's length on every pass behind an `if (i)`.

The list, the separator and each piece's length are now read into locals. A
one-byte separator is stored from a byte held in a register, and the
separator's share of the total is added once after the summing loop, as
`(n - 1) * sl`. `k_b_join` falls to 7,308,590, measured here.

CI's rows. run_instructions 844,029,138 -> 839,032,001 (-0.59%) and
work_pendbench 176,531,937 -> 156,541,737 (-11.3%). work_basket fell
31,152,439 -> 31,104,262; it joins too. Five moved by amounts this entry does
not account for: work_encodebench 2,456,796,006 -> 2,456,797,210,
work_oneshot 10,361,367 -> 10,361,430, work_indexbench 1,986,925 ->
1,987,038, work_scanbench 274,277 -> 274,340 and work_livebench 990,759,429 ->
990,759,737. Machine code grows by 384 bytes in every program: text
5,332,238 -> 5,337,614. Building the run program compiles the runtime, so
codegen_instructions_dev 116,162,655 -> 116,163,063 and
codegen_instructions_release 422,568,857 -> 422,573,245. In the tab,
browser_run_instructions 30,450,747 -> 30,425,950.

Both engines print the same bytes for an empty list, one item, an empty
separator, one- and two-byte separators with empty pieces, pieces and a
separator past sixteen bytes, and multi-byte pieces and separator, whose
character count is read too. The change is a rewrite of the loop the work
vein already counts, so the vein is its witness.

## 2026-10-23 — the site a death names is stamped as an id

Before every call that can stop the program, compiled code records the place
a death will name (ruled on #1975). It stored the interned literal's address
in `k_site`: a `lea` to materialise the address and a 64-bit store, 4,374,601
times a run. It now stores a 32-bit id as an immediate into `k_site_id`, one
`mov`. The program defines `k_site_name`, which maps an id back to its
literal through a table, and `k_died` reads it only when the program dies.
The 25 twin slow paths that receive the site as a `ptr %site` still store it
in `k_site`, and zero the id; `k_site_now` prefers the id when it is set. The
tab reaches `k_site_name` through an eighth hook in `wasm/hooks.c`.

The table went through three shapes, each measured. As `[N x ptr]` it put a
relocation per slot in the position-independent binary, and scanbench rose
3,107 instructions in `_dl_relocate_object`. As offsets into one
`@k_site_text` constant it needs none. A slot whose stamp was pruned with its
function holds zero and writes no text. Appended to the module's globals, the
table pushed that buffer past a doubling, and the tab compile's peak memory
rose 13,902 bytes; written into the module, which is allocated at its final
size after one counting pass, the rise is 1,142 bytes. Encoding the offsets as
a byte string rather than `[N x i32]` read the same to within 160 bytes, so
the table keeps the readable form.

CI's rows, against main with the join merged. run_instructions 839,032,001
-> 836,829,464 (-0.26%). The other work rows that fell most:
work_encodebench 2,456,797,210 -> 2,367,636,382 (-3.6%), work_livebench
990,759,737 -> 980,646,434 (-1.0%), work_deepbench 203,571,400 ->
201,843,399 (-0.85%), work_digestbench 5,241,199 -> 5,211,213,
work_pendbench 156,541,737 -> 155,739,870, work_oneshot 10,361,430 ->
10,346,159, work_basket 31,104,262 -> 31,068,983, work_widebench 28,619,457
-> 28,587,459, work_scanbench 274,340 -> 274,212 and work_readbench 4,582,081
-> 4,581,876. Two rose: work_jsonbench 584,083,829 -> 585,129,095 (+0.18%)
and work_escapebench 39,058,437 -> 39,058,492; work_indexbench moved by one,
1,987,038 -> 1,987,039. Every program gains the `k_site_name` definition,
and the stamps' literals leave the body: emitted_other_defines 2,217 ->
2,230 and emitted_other_lines 161,822 -> 161,270, and the decoder's
emitted_defines 157 -> 158 and emitted_lines 14,319 -> 14,296. The small
compile corpus, which has few stamps, gains more lines than it loses: its
defines 42 -> 46 and lines 1,528 -> 1,569, and the module build's
module_defines 29 -> 30 and module_lines 1,164 -> 1,171. Machine code falls,
text 5,337,614 -> 5,335,758. The emitter's scan for surviving stamps and the
table cost the compiler: emit_instructions 6,434,680 -> 6,521,335 (+1.3%),
codegen_instructions_dev 116,163,063 -> 116,434,829 and
codegen_instructions_release 422,573,245 -> 425,464,049 (+0.68%).
compile_instructions, entry_instructions and library_instructions each fell
by 15 to 25, which is layout. In the tab: browser_compile_instructions
427,996,730 -> 428,979,212, browser_compile_peak_bytes 768,243 -> 769,385 and
browser_run_instructions 30,425,950 -> 30,274,594. The run side outweighs the
compile side, and welfare rises.

jsonbench rises: `d_json/obj_key_start_4_w`'s IR differs from main by the one
stamp line, and with `rax` no longer needed for the address the register
allocator added a `mov r14,r10` / `mov r10,r14` pair on one path of its loop.

Declined: testing whether a float is integral with a cast and compare instead
of `floor`, in `k_b_append_rendered`. glibc's `__floor_sse41` costs three
instructions a call through the PLT, and the inline test cost four more, so the
run program rose 182,110.

The ratchet row `a_site_named_by_its_address_alone` has the runtime read only
`k_site`, and `runtime_corpus_reports_endpoint_violations` goes red: a death
behind an id stamp loses its `failed in` line.

## 2026-10-23 — the module writer moves out of codegen.rs, and the site table stops searching

`src/codegen.rs` was 4,680 lines. The code that writes the module around the
finished body now sits in `src/codegen/module.rs`, read with `include!` like
`narrow.rs`: `Written` and `finish`, the doors and `through_doors`, the call
twins, the type and site tables, `paired_appends` and `ir_bytes` (#2053). The
emitted IR of the run program is byte-identical once the checkout's path is
set aside. Four ratchet mutations and the wasm32 line check read the new
file. Alone, the move read browser_compile_instructions 428,979,212 ->
428,979,320 by layout, which put welfare just under its floor, so it lands
with the change below.

#2051's site table wrote text only for the stamps that survived the prune,
and finding them was a substring search over every piece of the body:
3,184,599 of the 96,470,945 instructions of a build of the browser corpus.
Almost every site survives. The table now writes every site the emitter
stamped, which adds 92 bytes of site text to that corpus and 802 to the run
program, and the search is gone.

CI's rows. emit_instructions 6,521,335 -> 6,449,595 (-1.1%) and
browser_compile_instructions 428,979,212 -> 427,904,481 (-0.25%);
browser_compile_peak_bytes rises 769,385 -> 769,408. Three front-end rows
return to what they read before #2051, by layout: compile_instructions
25,221,617 -> 25,221,632, entry_instructions 74,568,214 -> 74,568,239 and
library_instructions 74,230,408 -> 74,230,433. The run program's IR differs
only in the site constants, and four work rows moved with them:
work_runbench 836,829,464 -> 836,829,597, work_encodebench 2,367,636,382 ->
2,367,636,459, work_oneshot 10,346,159 -> 10,346,621 and work_livebench
980,646,434 -> 980,646,336.

## 2026-10-23 — the empty literals' C entries take no frame

`[]` and `{}` call `k_list_empty` and `k_map_empty`, and the decoder makes
557,000 of them on the run program. Each entry bumped the arena through
`k_alloc`, whose refill is a call, so the entry aligned the stack with a push
and a pop on every literal. The bump is now written in the entry, and the
counting path and a short arena go to a cold `*_slow` entry as a tail call;
the common path is sixteen instructions and no frame. The allocation counters
cannot move, since counting still goes through `k_alloc`.

CI's rows. run_instructions 836,829,597 -> 835,716,002 (-0.13%), and
work_jsonbench 585,129,095 -> 583,476,695 (-0.28%), work_oneshot 10,346,621 ->
10,335,157, work_deepbench 201,843,399 -> 201,739,409 and smaller falls in
seven more. work_escapebench rose 39,058,492 -> 39,058,532, which this entry
does not account for. Building compiles the runtime, so
codegen_instructions_dev 116,434,829 -> 116,444,792 and
codegen_instructions_release 425,464,049 -> 425,503,297, and machine code
grows by the slow entries: text 5,335,758 -> 5,338,222. In the tab, where a
wasm function has no frame to save, browser_run_instructions 30,274,594 ->
30,275,515.

Declined, both measured on CI first: the same bump written in the emitted IR
as helpers in DECLARES. Inlined at every literal, the run program fell 0.30%,
but the release build rose 4.4%, the dev build 1.0%, the tab compile 0.22% and
its peak memory 0.7%, which the objective scores as a loss. Defined once and
called, the run program fell 0.133% and the release build still rose 1.65%,
clang spending it on the new functions inside the decoder.

## 2026-10-23 — a string index answers its step and its cache hits in the entry

`k_b_at_str`, the proven string index, is called 690,000 times a run by the
run program's index phase, each call stepping the cursor one character. It
now steps the cursor and reads the ascii and wide-character caches in its own
body, and sends the seek, an ascii miss and a wide miss to functions of their
own. Making every one of those a tail call, so the entry would take no frame,
did not happen: clang emits plain calls on these paths whatever their
convention, so `k_b_at_wide_miss` keeps its `preserve_most`.

CI's rows, against main with the frameless empty literals merged.
run_instructions 835,716,002 -> 834,336,454 (-0.17%), and work_indexbench,
which is the same walk alone, 1,987,039 -> 1,947,078 (-2.0%); three other
work rows fell by under 300, and work_oneshot rose 10,335,157 -> 10,335,619,
which this entry does not account for. Machine code falls, text 5,338,222 ->
5,321,422. Building compiles the runtime, and both codegen rows fall a
little: codegen_instructions_dev 116,444,792 -> 116,430,160 and
codegen_instructions_release 425,503,297 -> 425,484,869.

## 2026-10-23 — an escaped block is classified once

The decoder's undo walk and the encoder's escape writer both copy a string
sixteen bytes at a time and stop at the next byte that needs work. After each
escape they loaded the next sixteen bytes and classified them again, though
most of those bytes had been classified a moment before. The run program
undoes 451,638 escapes and writes 407,520, about two and a half a string.
Both walks now keep the block's mask and answer every escape in the block
from it. After an escape the rest of the block is stored again behind what
the escape wrote, moved down by a shuffle through a 32-byte table: `pshufb`
on x86-64, `tbl` on arm64. The wasm32 runtime takes neither path, and
`docs/kanso.wasm` and `docs/kanso-runtime.wasm` rebuild byte-identical.

The undo walk also answers an escape inside its block loop instead of leaving
through `stop` for the loop that handles backslashes. When a block has
nothing left in it, the walk goes back to that loop at the byte after the
escape, where the loop's room test decides as it did before. So the walk keeps
its unescaped string in exactly the cases it did; the fuzzer's record counts
match main's on three seeds. With the mask loop mutated back to one block load
per escape, the run program reads 830,838,268 in the container, so the mask
is worth 0.70M of the undo walk's fall and handling the escape inside the
block loop is worth the other 2.44M. The writer's share is 0.98M.

The writer got a differential fuzzer of its own before the change: a
byte-at-a-time reference escaper, with each string ending a random distance
before an unmapped page and the bytes past the promised bound checked
untouched. Each walk ran fifty million cases on x86-64 and 300,000 on arm64
under qemu, with no failures. Two ratchet rows put back a block load at every
escape, one per walk.

CI's rows. run_instructions 834,336,454 -> 830,143,044 (-0.50%),
work_jsonbench 583,476,695 -> 578,721,695 (-0.81%), work_livebench
980,635,460 -> 975,722,811 (-0.50%) and work_oneshot 10,335,619 ->
10,291,652. work_encodebench fell 371; its frozen copy of the library escapes
strings in kanso, and this entry does not account for the 371. Building
compiles the runtime, so codegen_instructions_dev 116,430,160 -> 116,461,460
and codegen_instructions_release 425,484,869 -> 425,504,615, and every
benchmark binary's machine code grows 2,592 bytes: text 5,321,422 ->
5,357,710 summed over the fourteen.

## 2026-10-23 — a literal constant is its value at every mention

A top-level constant is built on first demand behind a cell with a ready flag,
and every mention called its accessor: test the flag, load the cell, check the
tag. A constant whose one arm is an integer literal that fits a word does no
work to build, so a mention of it is now that literal, the same
`{ i64 0, i64 N }` an integer literal emits. Anything else, a wider literal
included, still goes through the cell.

The run program's digest phase is where it shows. sha256's `whole` is read
about a dozen times a round, and a round fell from 295 instructions to 182 in
the container. The digest loop shrank enough to be inlined into the driver.
The escape phase went the other way: `span` is a literal constant too, and
once LLVM could see `filled`'s loop bound it reshaped the loop and spent one
more instruction a push, 1,552,672 over the run.

CI's rows. run_instructions 830,143,044 -> 828,053,655 (-0.25%),
work_digestbench 5,210,951 -> 4,150,769 (-20.3%) and work_deepbench
201,739,409 -> 201,098,912; work_escapebench rose 39,058,532 -> 40,308,322
(+3.2%) for the reshaped loop. The emitted code falls where constants were
read: runbench 799 -> 741 defines and 7,539 -> 7,393 calls, digestbench 135 ->
117 and 935 -> 872. emit_instructions 6,449,595 -> 6,449,884, and
browser_compile_instructions 427,904,481 -> 427,933,127. A ratchet row sends every mention back
through the cell; mutated, the run program reads main's count again.

## 2026-10-23 — a repaired node keeps what it points at through a lap rewind and a scalar pop

A native program could read freed memory where the interpreter printed the
right answer. The run program shows it with one constant changed: with
`deep_trials = 0`, main's build segfaulted in `k_copy_size` under
`k_beat_iter_carry`. Bisecting put the first bad commit at #1830, which added
the list of repaired nodes. A pop inside a loop can repair a node older than
the loop's mark so that it points at a copy at the arena's frontier, and the
frontier is in the loop's lap. #1830 put such a node on a list the loop's next
stage revisits before it rewinds, which carries the copy below the mark. Two
other ways out of a lap did not ask that list, and both handed the copy back
while the node still pointed at it.

A plain lap rewind, for a loop that carries nothing it has to copy, now runs
as a stage with nothing staged when the list holds a node below the loop's
mark. The stage revisits the node, copies what it points at into the carry,
rewinds and settles it back; `beat_iters` is corrected so the lap counts once.
A pop whose result is a scalar, and the region pop and the cohort with one,
keep the lap and hand it up to the enclosing loop when the list holds a node
below the mark, as a pop with a heap result already does. Both tests are in
out-of-line, cold functions, because a test inlined into `k_beat_pop` cost
encodebench 35M instructions.

Two mem fixtures pin the halves, and each fails on main and with only the
other half applied: `a_repaired_node_below_a_loop_that_copies_nothing` printed
`cover`'s bytes where the interpreter prints `name.`, and
`a_repaired_node_below_a_loop_that_returns_a_number` segfaulted. No other mem
golden moves, and the run program with `deep_trials = 0` runs to the end.

CI's rows. The list is empty in every program in the corpus, so what moves is
the test and the layout around it: run_instructions 828,053,655 -> 828,089,940
(+0.004%), and work_runbench with it. The other work rows move by under 4,000
each, oneshot and livebench falling and the rest rising to work_jsonbench
578,723,373, work_encodebench 2,367,625,387, work_basket 31,067,982,
work_widebench 28,587,526, work_deepbench 201,099,671, work_escapebench
40,308,367, work_pendbench 155,740,406, work_indexbench 1,947,279,
work_scanbench 274,328, work_digestbench 4,152,224 and work_readbench
4,581,904. Every benchmark's machine code grows 336 bytes: text 5,330,158 ->
5,334,862 summed over the fourteen. Building compiles the
runtime: codegen_instructions_dev 116,461,460 -> 116,473,612 and
codegen_instructions_release 425,504,615 -> 425,509,331.
browser_run_instructions 30,275,515 -> 30,276,840. Welfare falls by what the
fix costs, and the floor moves down with it under the rule for the language
doing what it says.

## 2026-10-23 — a template joins onto a record field in place

A template whose first piece is a field, `"{st.body}{d}"`, copied the whole
string on every pass, even where nothing else could see it. The run program's
restate phase does that to a 64 KiB string 128 times, and the beat carry
copied the result again. The string-builder analysis now takes a field read as
the builder when `field_moves` already proves the field alone: the record is
handed over whole, every mention of it in the arm reads a field, and this
field is read once. A template's first field joins the set of fields the field
fixpoint asks about. `k_concat_arr_mut` then writes into the string, and since
a builder's storage is malloc'd, the carry shares it instead of copying it.

For the restate phase's seed to count as alone, a parameter an arm reads only
into templates can now be an accumulator. `seed n acc = seed (n - 1)
"{acc}{acc}"` reads `acc` twice, and a template keeps none of its pieces, so
nothing is left holding it. The exemption covers only arms whose every mention
is a template piece.

CI's rows, against main with #2060's fix merged. run_instructions 828,089,940
-> 816,801,024 (-1.36%), and work_runbench with it. The run
program allocates 221,297,400 -> 212,891,960 bytes and evacuates 7,416,368 ->
4,591,200, and none of its peaks move. The mem vein's new
`a_record_field_joined_in_place` makes 10 allocations where main made 2,024,
and `an_interpolated_state_string_is_not_kept_under_the_mark` allocates
107,668,139 -> 2,796,683 bytes with its arena peak 3,145,984 -> 1,048,576.
Its scans rose,
`an_interpolated_state_string_is_not_kept_under_the_mark_str_scans` 2 -> 11,
while the bytes they read fell 1,048,677 -> 11, which this entry does not
account for. On
the compiler's side compile_instructions 25,221,632 -> 25,204,865,
entry_instructions 74,568,239 -> 74,522,559, library_instructions 74,230,433
-> 74,182,975 and emit_instructions 6,449,884 -> 6,459,502; interp_instructions
rose 385,357,985 -> 385,504,156, which is the interpreter's layout, since the
analysis does not run there. browser_compile_instructions rose 427,933,127 ->
433,668,787 (+1.34%). The same route compiled natively moves +0.10% and lowers
to the same module, and padding the tab's heap before the compile moved the
row by at most 6,000, so the rise is in the wasm build of the compiler and not
in allocator layout. I have not found where. Welfare rises 92.94 -> 92.99.

The same pull request moves the JSON string kernels out of `text.c` into
`src/runtime/json.c` (#2059) and removes `k_shareable`, `k_viewreg_add` and
`k_ten_holds`, which nothing called. On its own the move cost
codegen_instructions_dev 1,744 and codegen_instructions_release 570 by layout,
which put welfare under the floor, so it rides here. With both, the dev row
reads 116,473,612 -> 116,472,553 and the release row 425,509,331 ->
425,509,866.

## 2026-10-23 — seven leads measured and declined

**A number's value kept by its span.** The decoder asks `number_span` where a
number ends and then `to_int` or `to_float` what it is worth, and both read
the same bytes. The span's word path was made to leave the number's value,
keyed by its bytes, for the two slice doors to answer from. The doors fell
23.5M instructions on the run program, and the span rose 31.4M: it now built
a frame of six registers on every call, and the value's arithmetic only moved
from one function to the other. A second version kept only the number's
shape and raw bytes, so the doors skipped their own shape checks and still
did the digits. The doors fell 6.4M and the span rose 10.0M. The converters
were already close to their floor: `k_to_int_text` reads up to eight digits
as one word and `k_float_shape8` reaches Clinger's exact path, so the only
work the two calls repeat is the shape test, and keeping a record across the
calls costs more than that.

**The float renderer's integral test as a cast.** `k_render_number` asks
`d == floor(d)` before printing a float as an integer, which is a call into
libm on each of the run program's 191,070 floats. Asking whether
`(double)(long long)d == d` under 1e15 costs as many instructions inline as
the call it replaces, and the run program read 182,140 more.

**A digit count from the bit length.** `ryu_declen` counts a number's
decimal digits with a ladder of nineteen compares. Estimating the count from
the bit length and correcting it with one table compare is exact (checked at
every power of two and of ten, either side), but the numbers the run program
renders are short, the ladder leaves early for them, and the run program read
327,397 more: `k_b_append_rendered` rose 607,050 and `k_b_render_value` fell
235,522.

**A decoded document trimmed to its contents.** An empty map opens with room
for five pairs and an empty list with room for six, which is every size
bench/large.json uses, so the decoded document carries about 280 KB of unused
slots in its 1.0 MB. When the 1.5 MiB string the index phase builds is
allocated, the input and the document fill one 1 MiB block and 157,568 bytes
of a second, and the run program's counted peak is those two blocks and the
string's. A builtin that gives a finished container's room back when nothing
was allocated after it, with arrays opened at their first element and maps
taking their pairs at their first put, fit the input and the document in one
block: arena_peak_bytes 3,670,032 -> 2,621,456. The run-memory term is near
its ceiling, though, because its baseline was 1.43 GB, so a whole block is
worth +0.03 of welfare. The trim alone cost 37M instructions, 14M in the
kernel and the rest in how LLVM folded the decoder's states around the new
call. With the maps seeded late as well, the run program read 963.7M and its
evacuations rose from 1,624 to 70,982, which I did not trace further. One
percent of run_instructions is worth about 0.063
of welfare, so the trade loses by a wide margin.

**An object's scalar value read in the object's loop.** An array's elements
are parsed inside its loop, because LLVM inlines `parse_value` there, while
an object calls `parse_value` for every value, 827,739 calls a run at about
51 instructions each. Dispatching on the value's first byte after the colon,
and calling `parse_value` only for a nested array or object, read 890,453,082:
+62M. LLVM split the states it had folded into the object's loop into
separate functions, `obj_value_at`, `value_for` and `parse_string` among
them, and the calls between them cost more than the one they replaced. The
emitter has no say in which kanso functions LLVM inlines.

**Type-based alias tags on the in-place push.** The push twin reloads the
list's length and items pointer after every slot it writes, because LLVM
cannot tell the slot from the header. Tagging the twin's loads and stores
with three TBAA types left the run program at 828,157,323, forty fewer than
without them. Every loop that pushes also holds the twin's slow path, a call
that may change the header, so the loads stay where they are.

**Inlining chosen by hand.** An environment switch in the emitter marked one
named function `alwaysinline` or `noinline`, and each of the 84 functions in
std/json was tried both ways on the run program (baseline 828,157,901). Six
single changes saved 2.25M to 2.93M each, all on the object path:
`obj_colon`, `skip_ws` and `ws?` inlined, `obj_key`, `colon_bad` and
`obj_items` kept out of line, while `array_bad` kept out cost 3.6M. They do
not add. The best pair, `obj_colon` in and `obj_key` out, read 822,315,778
(-0.71%); all six together read 845,054,425 (+2.0%), and `obj_colon` with
`obj_items` read 843,779,017. These are LLVM's inliner landing on different
shapes rather than a property of the functions, and a set picked for the
lowest reading on today's tree is a draw the next change would reshuffle.

## 2026-10-23 — the browser rows count the instructions a module executes

The entry that built the browser side (2026-10-06, "the browser is a third
side of the objective") says wasmi spends one unit of fuel per executed wasm
instruction, so the instruction rows are a count. wasmi charges by the frame.
Its translator puts one fuel charge at the head of each function
body, loop body and `if` or `else` arm, sized to every instruction in that
frame, and a `block` gets no charge of its own: the code after a block's
`end` is billed at the head of whatever loop or function encloses it. A
`br_table` dispatch is a nest of blocks with one arm after each `end`, so a
loop around a dispatch pays for every arm on every pass.

**How it was found.** The tab's compile cost 3.8 times the native route's
instructions in the front end and the translation and 2.1 times in the
emitter. Neither 128-bit token parsing (16,025 digits in the whole IR) nor
SIMD explained it: kanso.wasm built with `+simd128`, run with wasmi's simd
feature, read 433,668,787 -> 433,177,592. A scratch profiler then copied
kanso.wasm with a counter per function and ran the tab's three phases
separately. Counting each instruction where control reaches it gave 29.9M,
52.1M and 31.0M (leaving out local moves and constants, which wasmi folds
into its own instructions). Charging each frame's instructions at its head,
the way wasmi does, gave 143.9M, 138.6M and 161.7M against fuel of 144.9M,
136.8M and 151.9M, so the frame charge is the fuel. One function shows it
most: `ir_wasm::tokenize_into`, the loop that splits the IR into tokens, was
charged 60.7M fuel for 6.1M instructions, 14% of the compile.

A change aimed at that fuel would move match arms out of their loops to suit
wasmi's billing, and a browser's compiled code does not reward that. The
2026-10-06 ruling leaves how a browser term is measured to the implementer, so
the measurement changes instead.

**What is counted now.** `counted` in tests/support/wasm32.rs copies a module
and adds an exported i64 global. It splits each function body into runs that
control can enter only at their first instruction, ending a run after any
instruction that branches, opens or closes a block, or traps, and each run adds
its length to the global as it begins. `tests/browser_cost.rs` loads the
counted copy of kanso.wasm, and `execute` runs counted copies of the runtime
and the translated module, so both instruction rows are every wasm instruction
control reached. A loop counted by hand in
`the_count_is_every_instruction_control_reached` reads 91; letting a run go on
through its `br_table` reads 95, and counting each run as one instruction reads
32. The ratchet row "a counted run that goes on
through a br_table" keeps the first.

    browser_compile_instructions  433,668,787 -> 293,067,500
    browser_run_instructions       30,276,840 ->  34,963,054

The run reads more than its fuel did because the count includes `local.get`,
`local.set` and constants, which wasmi fuses into its register instructions and
does not bill, and the runtime's C has few dispatch loops for the frame charge
to inflate.

**Welfare is re-based.** Each baseline is scaled by the ratio of the
two readings on this tree, so each term's ratio and the meta are what they
were: browser_compile_instructions 343,917,485 -> 232,414,785 and
browser_run_instructions 2,331,914,518 -> 2,692,845,529 in
bench/welfare_floor.json. The `--set` records the floor unchanged.
bench/compile_epochs.txt owes a line for each counter, naming the merge
commit's history row once perf-history carries it, so that
scripts/welfare_rescore scores the fuel-era rows against the old baseline.

Fuel stays in `execute` only as the stop for a translation that loops forever,
raised from two billion to ten. The heaviest micro program executes 927 million
instructions, and its counted copy spends 2.33 billion units of fuel. The
prose that said fuel is spent per executed instruction is corrected in
tests/browser_cost.rs, bench/browser_golden.txt, scripts/gates/browser_cost.sh,
bench/objective_sources.txt, scripts/welfare/welfare.kso, docs/numbers.html and
CLAUDE.md.

## 2026-10-23 — digits rendered straight into the builder, declined

`k_b_append_rendered` writes a number into a 64-byte stack buffer and then
copies three words onto the accumulator's frontier. Rendering into the
frontier itself, when 64 bytes were free there, read 818,467,737 on the run
program against main's 816,905,427 in the container. The renderer's stores now
went through a pointer into the arena instead of a buffer nothing else can
alias, and `k_b_append_rendered` rose 1,523,160. The first version, which
wrote the render call twice, also stopped `render_ryu` being inlined.

## 2026-10-23 — eight costs out of the tab's compile

Counted in executed instructions, the costliest function in the tab's compile
was `memcmp`: 9.5M of the front end's 68.8M and 17.5M of the emitter's 119.2M
on `bench/interp_corpus`. The `memcmp` wasm32 links by default is
compiler-builtins' loop of one byte per pass, and the compiler compares names
and keys constantly: hash-map probes on `&str` keys, `CharSearcher` checking a
found byte's neighbours, the inferencer matching names.

`src/compare.rs` compares eight bytes per pass and ends with the eight bytes
that finish the run, overlapping bytes already found equal, so no length needs
a loop of single bytes after the words. A run of four to seven bytes is read
as two four-byte words, and the first byte that differs is found from the
lowest set bit of the two words' difference. `src/main.rs` exports it as
`memcmp` and `bcmp` in the wasm build only; native builds keep the C library's.

    browser_compile_instructions  293,067,500 -> 274,134,296  (-6.46%)

The same code written in `main.rs` read 273,195,866, and in the library with a
plain `#[inline]` 275,072,726, because the call into `kanso::compare::bytes`
stayed out of line. `#[inline(always)]` takes most of that back. A first version
that finished with single bytes instead of the overlapping word read
281,953,624, because most compares are a few bytes long.

`tests/the_word_compare_agrees_with_bytes.rs` holds it to the standard
library's slice order: 4M random cases in CI, and 50M once with no failure, at
every length to 48 and every pair of offsets up to seven bytes out of
alignment, half the differing bytes straddling 0x80. Turning the lowest set bit
into the highest, dropping the overlapping word, comparing the bytes as signed
and dropping the second four-byte word each fail it. The ratchet row "a byte
loop compares in the tab" renames the two exports, and the browser row goes
back to 293,067,500.

**A switch found by its `=`.** The translator asks of every IR line whether
it opens a switch, which may run over several lines, and it asked with
`body.contains("= switch")`. `contains` builds a substring searcher before it
reads a byte, and on the tab's corpus that setup was 5.9M of the 71.0M
instructions the translation executed. A function body's line holds one `=`,
the one that names its result, so the check now finds it with memchr and
compares the words there.

    browser_compile_instructions  274,134,296 -> 268,290,909  (-2.13%)

Finding every `=` in the line instead of the first read 270,476,791, because
the second search runs to the end of the line. The ratchet row "a searcher
built for every line the tab translates" puts `contains` back and the row reads
274,134,296 again.

**The closure-jump twin read off the walk.** The narrowing searched the whole
module for `define internal tailcc %KValue @k_tcall` before it walked the
module's define lines, so that every lambda could keep `tailcc` where the twin
jumps through a closure's pointer. The walk visits every define line anyway, so
it now notes the twin and the lambdas as it passes and keeps the lambdas after.
The search was 739,426 instructions of the tab's route natively.

    browser_compile_instructions  268,290,909 -> 266,884,355  (-0.52%)

Nothing pinned the lambda rule, so a spec in src/codegen/narrow.rs now does:
beside the twin a lambda keeps `tailcc`, and without it the lambda is
narrowed. Noting no twin, or collecting no lambdas, each fail it, and the second
is the ratchet row "a lambda narrowed beside a closure jump". The row "the whole
module searched for the closure jump" puts the search back, and the browser row
reads 268,297,303.

**`@sN` found by its `@`.** `named_strings` looks for every `@sN` in the
module to decide which interned strings to write, and it searched with
`find("@s")`. A two-way searcher with a two-byte needle can skip almost
nothing, so it now finds each `@` by memchr and checks the byte after it.

    browser_compile_instructions  266,884,355 -> 264,519,748  (-0.89%)

**Line ends eight bytes at a time.** The translator ends each line of the
module with `find('\n')` while it reads the module's heads and with
`str::lines` while it reads a body. Compiled to wasm each of those pays a
searcher's setup, a call into memchr and a one-byte compare per line. A function
in src/ir_wasm.rs, `line_end`, reads eight bytes per pass and finds the newline
with the has-zero test, and both readers use it.

    browser_compile_instructions  264,519,748 -> 261,513,550  (-1.14%)

A fuzzer beside it checks `line_end` against `position` at every starting point
of 400,000 random texts built from newlines and the bytes one bit away from one,
and `Lines` against `str::lines`. Taking the highest set bit, dropping the
`& !w` that keeps a byte above 0x7f from passing for a zero, or dropping the
tail loop each fail it. Ratchet rows put each searcher back, and the browser row
reads 263,889,085 and 264,519,748 under them.

**Short searches as byte loops.** Two questions asked of every line find a
byte a few places in: the emitter's `is_a_stack_slot` looks for the space that
ends the line's first word, and the translator's switch check for the `=` that
names a result. `find` on a char pays a searcher's setup, a call into memchr
and a one-byte compare before it reads a byte, which is more than the few bytes
the loop reads.

    browser_compile_instructions  261,513,550 -> 259,644,771  (stack slot)
                                  259,644,771 -> 258,016,805  (switch)

The ratchet row "a stack slot found by a searcher" puts `find` back and the row
reads 259,885,584.

**No tally the tab never reads.** The counting allocator adds every
allocation's size and count to `compile_alloc_bytes` and `compile_allocs`,
which only the command line prints. The tab reads the live bytes and the peak,
so the wasm build now skips the other two.

    browser_compile_instructions  258,016,805 -> 256,933,580  (-0.42%)

The ratchet row "a tally the tab never reads" keeps the two counts in `alloc`
again, and the row reads 257,851,861.

Across the eight changes the tab's compile falls from 293,067,500 to
256,933,580 executed instructions, 12.3%.

**The native veins.** `is_a_stack_slot`'s byte loop runs natively too, and
the emitter's own count falls: emit_instructions 6,459,502 -> 6,249,687. Three
rows move by layout with the new code: compile_instructions 25,204,865 ->
25,205,087, entry_instructions 74,522,559 -> 74,523,222 and
library_instructions 74,182,975 -> 74,183,641. Natively the word compare and
the byte searches are not linked in, and memchr and memcmp stay the C
library's.

**The epoch the meter change owed.** bench/compile_epochs.txt now names
bda14b8, the first history row counted in executed instructions, with
0.6758 for browser_compile_instructions and 1.1548 for
browser_run_instructions. Rescored with it, the history rows for 15d6b27 and
bda14b8, one tree measured both ways, read 92.9893 each.

One idea measured and dropped: `lines_holding`, the narrowing's search for
`define `, `musttail call` and `tailcc ` across the whole module, found each
word by memchr on one of its bytes and compared there. It read 272,919,712
against 268,290,909. The two-way searcher skips a needle's length at a time
where the module's text cannot match, and memchr stops at every occurrence of
the anchor byte. Anchoring each word on its rarest byte in the module, `f` in
`define `, `m` in `musttail call` and `c` in `tailcc `, read 267,916,152
against 266,884,355: the searches took 8.3M where the two-way searcher took
6.8M.

## 2026-10-23 — the tab holds its module once

Five more changes to the tab's compile of `bench/interp_corpus`, three for its
instructions and two for the most it holds.

**Three more costs.** `Numbered`, the emitter's `%tN` and `LN` names, checked
its bytes were UTF-8 on every read, and it is built in one place from a `&str`
prefix and ASCII digits, so the read now skips the check, as `Name::as_str`
does: 256,933,580 -> 255,076,345. `paired_appends`, the narrowing and the prune
each compact a module's bytes around whole lines and then validated the whole
buffer again; a release build no longer does, and a debug build still checks:
-> 253,497,498. `line_end` became `kanso::compare::byte_from`, which finds any
byte eight at a time, and the emitter's three scans for `@` use it:
-> 250,608,006. Its fuzzer moved beside the word compare's and now tries five
target bytes, each with neighbours one bit away.

**Where the peak was.** A marker set at the start of each phase and read by the
allocator when a new peak was reached put the peak in `Written::finish`: it
allocated the module at its final size, 352,805 bytes, while every piece of the
body was still live, so the module was held twice. The emitter never splits a
definition between two pieces, so the tab now takes the pieces as they are.
`Written::finish_pieces` hands them over, `finish` joins them for clang, the
narrowing walks them in order and rewrites each where it stands, and
`ir_wasm::translate_pieces` reads every piece's heads and then lowers the
bodies a piece at a time, letting each piece go when its functions are done.

    browser_compile_peak_bytes     769,408 -> 724,714
    browser_compile_instructions  250,608,006 -> 251,137,543

Rewriting six pieces and slicing them costs 529,537 more instructions than one
join. Welfare read 93.05 against 93.04 without it, so the peak pays for them.

**What the walk left out.** With the join gone the peak moved into emission.
A census of the native route at its peak, every live allocation grouped by its
caller, held 330 KB of body text and about 300 KB of syntax. Part of the syntax
was the groups the entry cannot reach, kept so that a miss can put them back
and emit again. The tab now lets them go before it emits. A miss answers
`None`, and the tab compiles the source again and emits with them
(`emit_ir_tab_pieces_whole`), so a miss costs a second compile and never changes
the module. None of the 348 programs in the micro corpus misses.

    browser_compile_peak_bytes     724,714 -> 664,061
    browser_compile_instructions  251,137,543 -> 251,140,915

**Room the parser doubled into.** The same census showed statement and pattern
lists grown by `push` and `extend`, which keep the room they doubled into. The
tab's emit now gives that room back for every kept function's body and
parameters, and for the function list, before it emits. A native build skips
the copies, since nothing prices its holdings at that moment.

    browser_compile_peak_bytes     664,061 -> 598,093
    browser_compile_instructions  251,140,915 -> 251,201,816

Shrinking where the parser builds a body instead read 615,869 and cost 116K
instructions, because it also shrinks the groups the walk leaves out and every
native build pays for it.

**The narrowing's searches by one byte.** With `byte_from` inline, the
narrowing's three whole-module searches find `define `, `musttail call` and
`tailcc ` by their `f`, `m` and `c` and compare the word where one lands. The
same anchors read through `str::find` lost to the two-way searcher earlier
today, because each hit paid for a searcher and a call into memchr; inline,
the three cost less than the 7.1M the searcher took.

    browser_compile_instructions  251,201,816 -> 248,961,764

A spec builds 20,000 random texts from the three words, their anchors and near
misses, and holds `lines_holding_at` to `lines_holding` on each; comparing from
the anchor rather than from the word's start, or reporting a line once per
anchor in it, each fail it.

**Two bytes at a time.** `symbols_before_newline` walks the whole module for
each line's last `@`, a byte a pass, and that walk was 5.5M. It now asks
`compare::either_from` for whichever of `@` and a newline comes next, testing
each eight-byte word for both.

    browser_compile_instructions  248,961,764 -> 244,730,101

`called_symbols`, which reads from each `@` to its `(` or the line's end, uses
it too: -> 244,413,637.

`either_from` has a fuzzer of its own beside `byte_from`'s, for three pairs of
target bytes, and dropping the second test's `& !` fails it.

**Specs.** The interp corpus cut into pieces every seventh definition translates
to the same module as the whole text, and letting a piece go before its
functions are lowered fails it. The lean emit writes the pieces the whole emit
writes. Ratchet rows undo each change; the browser rows are the witness for
six of them.

**CI's rows.** The emitter's own count falls with its scans:
emit_instructions 6,249,687 -> 5,919,265 (-5.3%). interp_instructions moves
385,504,156 -> 385,492,276; no change here touches the interpreter's code.
compile_instructions, entry_instructions, library_instructions and both
codegen rows read what #2065 left them.

**Declined.** Body pieces of 8, 16 and 32 KiB instead of 64 moved the peak by
at most 5,564 bytes and cost 0.8M to 1.5M instructions. The same one-byte anchor for
`paired_appends`'s long needle saved 172,616 instructions and for
`branches_to`'s `label %` probe 286,197, too little to give either a helper of
its own.

## 2026-10-23 — a short float walks down by its zeros

`ryu_short` starts its search at the place count the last float took. When
the float passes there, it used to try each place below until one failed, so
a float with the same precision as the one before it paid two tries: one to
pass and one to fail. Each try multiplies, rounds, converts back and divides.
On the run program 172,979 of the 191,070 floats it renders take that path.

While the search's bound holds, the interval of decimals that read back as
the float is narrower than a unit at p places, so it holds at most one
decimal with p places. A decimal with p - 1 places in it is also one with p
places, so it is m / 10, and it exists exactly when m ends in a zero. The
walk down now takes the zeros off m instead of trying: it reaches the same
place count and the same m with no division of doubles. `k_tenfold` asks
whether m is a multiple of ten by multiplying by five's inverse modulo 2^64
and comparing, and the quotient is taken only for a zero that is there.

On this container, the run program from a fixed path:

| tree | runbench |
| --- | ---: |
| main | 816,801,133 |
| `m % 10 == 0`, quotient and remainder together | 816,057,840 |
| `k_tenfold`, quotient only on a zero | 815,624,925 |

That is −1,176,208 (−0.144%), with output byte-identical to main.
`tests/every_rendered_float_reads_back_as_itself.rs` lifts the search from
the runtime and sweeps 5,809,326 doubles with no failures. With the walk
down removed it reports 975,871 not shortest, the count it gave for the old
walk's removal. The ratchet row `float_walk` puts the tries back, and the
mutated tree reads the run program at main's 816,801,133.

**A switch line read by its first word.** The tab's translator asked of every
body line whether it held `= switch`, scanning for the `=` a byte at a time.
A switch is a terminator and names no result, and the emitter writes it as a
bare `switch`, so that test could never pass. The line's first word is now
the whole test: browser_compile_instructions 256,933,580 -> 256,026,859
(-0.35%) on the tree before #2068, and 244,413,637 -> 243,046,113 (-0.56%)
on the tree after it. The ratchet row `switch_line` now puts a substring searcher back
on every line, and the browser row is its witness, as before.

**CI's rows.** runbench 816,801,024 -> 815,624,816 (-0.144%). The other
benchmarks that render floats fall with it: encodebench 2,367,625,387 ->
2,362,394,628, livebench 975,721,913 -> 970,491,700, widebench 28,587,526
-> 28,539,533 and oneshot 10,291,264 -> 10,278,879. The rest do not move.
Each binary's text is 208 bytes smaller, since the float renderer is
inlined into it. codegen_instructions_dev 116,472,553 -> 116,472,194 and
codegen_instructions_release 425,509,866 -> 425,504,448 compile the smaller
runtime. On the tree after #2068, emit_instructions, compile_instructions,
entry_instructions and library_instructions read what #2068 left them. A run
of this branch before that merge had them moving by 3,000 to 87,000, which
the merge took away, so neither change here moves them.

## 2026-10-24 — emit_call_rest gives up its fused builtins and its declared call

`emit_call_rest` in `src/codegen/emit.rs` ran from line 3169 to the end of the
file, about 1,020 lines (#2071). Its fused builtin calls (`utf8` of a slice,
`length`, the three shapes of `append`, the one-argument doors and `bytes`)
are now `emit_fused_builtin`, and the call to a declared group is
`emit_declared_call`. The lines moved verbatim apart from the fused block's
returns, which answer `Some`, and the declared call's body, which lost one
level of indent and was formatted again.

The emitted IR of the thirteen benchmarks built from their directories is
byte-identical before and after. Two rows moved, and I read both as the
binary's layout, since the emitter writes the same text:
emit_instructions 5,919,265 -> 5,918,172 and browser_compile_instructions
243,046,113 -> 243,034,646. Every other row reads what main left it. The
ratchet mutations `a_call_into_a_loop_left_unbracketed` and
`a_proven_result_left_unsaid` follow their de-indented lines, and every
mutation that edits `emit.rs` applies and builds.

## 2026-10-24 — dev and tab bodies write no llvm.assume

The emitter told LLVM what it had proved with `llvm.assume`: a parameter's
tag where inference pins one kind, a length that is never negative, an int's
tag after a `%parsed` is unpacked, and the range of a byte position in a
dispatcher. Each one costs a compare and a call, and the tag ones an
`extractvalue` too. Only a release module is optimized. The dev tier compiles
at -O0, which drops the call without reading it, and the tab's translator in
`src/ir_wasm.rs` skips the call but still translates the compare and runs it.
The dev tier emits with the settings the tab does, and for
`bench/interp_corpus` it held 271 assumes, which with their feeding lines
came to 729 of its 9,964 lines; runbench's fell from 36,674 to 35,082. The
two assumes left are in the fixed helpers `DECLARES` carries.

`FnEmit::assumes` now answers false for a body built with `words`, which is
every dev and tab body, and the five sites that write an assume ask it first.
A dispatcher still records a byte position as an int, which the emitter reads
itself; only the lines for LLVM go. The release IR of all nineteen
benchmarks built from their directories is byte-identical before and after.

In this container, at the golden's rustc, the tab's compile of the corpus fell
from 243,034,646 to 235,063,784 instructions (-3.3%) and its peak from 598,093
to 532,354 bytes (-11%), and the run fell from 34,963,054 to 33,372,898
(-4.5%), the compares it no longer executes. On CI the dev build's two rows
fell with the emitted text: emit_instructions 5,918,172 -> 5,863,976 and
codegen_instructions_dev 116,472,194 -> 116,219,760. The codegen corpus's dev
IR lost ten of its twelve assumes. Every other compile row reads what main
left it. The ratchet row `an_assume_written_for_no_optimizer` writes them for
every body again.

## 2026-10-24 — the tab leaves its tail calls as written

`Written::finish_pieces`, which hands the tab its module, ran the same
tail-call narrowing as `finish`, which writes a native one. The narrowing
keeps `tailcc` only on the functions a `musttail` reaches and sends a call to
a function that spills its arguments through a trampoline. Both work around
an arm64 miscompile. The tab's translator skips `tailcc` and `ccc` as words,
so the pass searched the module for nothing it lowers. `finish_pieces` now
stops before it, and `finish` narrows what `assembled` gives it.

On `bench/interp_corpus`, in this container at the golden's rustc, the tab's
compile fell from 235,063,784 to 227,562,045 instructions (-3.2%). Its peak
and the run did not move, since the corpus's module holds no trampoline.
Native IR is unchanged. The ratchet row `the_tab_narrowing_its_tail_calls`
narrows the tab's pieces again. On CI, emit_instructions read 5,864,154
against the 5,863,976 the assume change left it, a move of 178; the dev
build that row counts still narrows its module, and writes the same text.
CI's browser rows matched this container's.

## 2026-10-24 — a short float written in one pass

`render_ryu`'s short path wrote a float from its scaled decimal m and place
count p by dividing m by 10^p, writing the whole part forward after counting
its digits, and then writing the fraction behind the point. It now writes m
right to left in one pass, two digits at a time: the p fraction digits, the
point, then what is left of m. The text goes into the top of a scratch buffer
and moves out as three words. On runbench 191,070 floats take this path, and
in this container the run program fell from 815,624,421 to 813,607,261
instructions (-0.25%), with byte-identical output.

`tests/every_rendered_float_reads_back_as_itself.rs` now checks the text as
well: the digits ryu chose, placed by the rule the interpreter's
`render_float` follows. Its other checks pass `.5` for `0.5`, which reads
back. With the whole part's zero dropped, it reported 141,779 text failures
and none of the other kinds. At ten times its size it read 50,807,850
doubles with no failure. The ratchet row `float_one_pass` puts the division
back.

## 2026-10-24 — the module's symbols read in one scan

The module writer asks two questions of every piece of the body: which
symbols it calls, written `@sym(`, to choose the declarations and call twins
it keeps, and which symbols end a line, written `@sym\n`, to choose the
closure cells. `called_symbols` and `symbols_before_newline` answered them
with two scans of each piece, and the second stopped at every newline as
well as every at. `called_and_ending` answers both in one pass over the ats:
from each at to the first `(` or newline, a `(` making a call and a newline
an ending when no other at comes first.

The same pass now marks the interned strings the body names, `@sN` and
`@sN_lit`, which `named_strings` found with a third scan of every piece; it
still reads the type tables on their own. The tab's compile of its corpus fell
from 227,562,045 to 224,770,876 instructions with the two scans merged and to
223,778,736 with the third folded in, -1.66% in all, in this container. Its
peak and the run did not move, and the tab's IR and the native IR of the
nineteen benchmarks in both tiers are byte-identical. The specs that held each
scan to a plain search now hold the merged one to it, with the ending side
asked only about symbols without a `(`, which is every symbol the writer asks
about; treating the end of the text as a line's end turned that spec red on
`"@a"`. A new spec holds the pass's string marks to `named_strings`'s, and
went red with the marking taken out. The ratchet row `one_scan` scans each
piece a second time and reads 225,543,228; `strings_in_scan` scans the body
for strings again and reads 224,854,630.

## 2026-10-24 — a pair of leaves held in place

The tab's translator asks a type for its scalar leaves, with their byte
offsets, at every load, store, call and return of an aggregate. A scalar
answered in place, and every struct built a vector, so each `%KValue`, two
`i64`s, cost an allocation and a free. `Ty::leaves` now answers a struct of
two scalars in place too. The tab's compile of its corpus fell from
223,778,736 to 222,368,963 instructions (-0.63%) in this container, and the
wasm it wrote is byte-identical. A spec holds `leaves` to the list `scalars`
writes for scalars, pairs, a nested pair, a triple and arrays; giving the
second leaf a fixed offset of 8 turned it red on `{i1, i16}`. The ratchet row
`pair_leaves` lists pairs in a vector again and reads 223,694,975.

**CI's rows for the three.** The run program fell from 815,624,816 to
813,606,641, and the other benchmarks that render floats with it:
encodebench 2,362,394,628 -> 2,353,427,661, livebench 970,491,700 ->
961,524,285, widebench 28,539,533 -> 28,476,897 and oneshot 10,278,879 ->
10,256,694. Every binary's text is 688 bytes smaller, the renderer's. The
browser rows matched this container's. emit_instructions fell from 5,864,154
to 5,690,029 (-3.0%), since `finish` runs `assembled` on every build and
reads the module once where it read it three times. compile_instructions
25,205,087 -> 25,172,884, entry_instructions 74,523,222 -> 74,435,723 and
library_instructions 74,183,641 -> 74,096,953 count `kanso check`, which
reaches none of the three changes, so I read them as the binary's layout.
codegen_instructions_dev moved 116,219,760 -> 116,218,430 and
codegen_instructions_release 425,504,448 -> 425,508,082; I have not isolated
either. Welfare rose from 93.12 to 93.14 and is banked.

Measured and set aside while looking: stripping the comment lines DECLARES
carries from dev and tab modules, 11,317 of the corpus module's 327,126
bytes, saved 85,914 instructions of the tab's compile (0.04%); and summing
short integer literals in an `i64` rather than an `i128` in the translator's
tokenizer saved 308,791 (0.14%). The wasm-side profile behind both is a
per-function count of `docs/kanso.wasm` built with its names: the tokenizer
was 6.6% of the compile and `bcmp` 4.2%, where natively they were 2.9% and
2.2%.

## 2026-10-24 — the tab's bcmp answers equality alone

The tab links its own `memcmp` and `bcmp` (`src/main.rs`), and both were
`kanso::compare::bytes`. A per-function count of `docs/kanso.wasm` built
with its names showed the result of one body under two names: every compare
in the tab's compile of its corpus called `memcmp`, and `memcmp` called
`bcmp`, 154,063 times, together 4.2% of the compile. `bcmp` now has a body of
its own, `compare::differ`, which answers equality alone. In the wasm built
from this tree nothing on that path calls `bcmp`; `memcmp` runs the compare
itself, and the call through to the second function is gone.

The compile fell from 222,368,963 to 221,598,648 instructions (-0.35%) in
this container. `tests/the_word_compare_agrees_with_bytes.rs` holds
`differ` to slice equality beside `bytes`, on its 4,000,000 random pairs and
on every position of every length up to 24; skipping the last word of a run
longer than eight turned both red. The ratchet row `equal_bcmp` gives `bcmp`
the ordering body again, and the compile reads 222,368,963.

The calls come from `==` on strings, which reaches `memcmp` even against a
short literal: the type names in `P::ty` and the attribute words in
`P::skip_attrs` alone made 10,124 of them. Matching those words as integer
keys built from their bytes cost more than it saved, 222,368,963 ->
222,475,311 on the tree before this change, and was set aside.

## 2026-10-24 — a name no query starts like is passed over

`queries_named` reads every `@name` in a text, measures it and looks it up in
a set of names the caller asks about. It runs over every definition block in
the prune and over every piece when the tab checks whether its emit named a
group it left out. Most of a module's ats are runtime calls, string cells and
intrinsics, `@k_`, `@s`, `@llvm.`, and the names asked about are the
program's own groups, so the callers now build a table of the bytes their
names start with, once per set, and the scan passes over a name whose first
byte is not in it before measuring or hashing it.

The tab's compile of its corpus fell from 221,598,648 to 220,923,375
instructions (-0.30%) in this container, and the tab's IR and the native IR
of the benchmarks in both tiers are byte-identical. Removing the check
outright would have saved 1,751,745, the ceiling for this scan in the tab's
miss check. Building the table inside `queries_named` instead, once per
block, cost 680,432 more than it saved. The ratchet row `query_leads` admits
every byte and reads 221,611,521.

On CI the dev build's emit fell with the prune's scan, emit_instructions
5,690,029 -> 5,651,244, and the browser rows of both changes matched this
container's. Every other row reads what main left it.

## 2026-10-24 — the string index inlined into the walk

The run program's index phase calls `k_b_at_str` 690,000 times from
`tally_4`, about 48 instructions a call. The release link inlines only what it
holds as bitcode, and the runtime is machine code there apart from the hot
unit `hot_source` builds. Linking runbench against the whole runtime as
bitcode, with the link's own flags, read 811,029,731 against the shipped
813,607,261, and the index was still a call: the emitted IR called it
returning `%KValue`, clang gives a C function returning `KValue` the type
`{ i64, i64 }`, and the inliner does not cross the two. With the call's type
matched and the definition `always_inline`, the same link read 807,234,717.

The change makes that hold without putting the runtime back in the link.
`k_b_at_str` is defined in the hot unit, guarded by `K_HOT_ELSEWHERE` in
`runtime.c` as the scanners are, with `k_at_char`, `k_cp_len` and
`k_as_str` cut from `runtime.c` beside it. The cursor, the two character
caches, the two seek counters and the three out-of-line arms it calls now
have external linkage. The emitter calls it returning `{ i64, i64 }` and
puts the two words back into a `%KValue`. The translator reads the two types
as one, and the tab's rows did not move apart from 59 instructions of its
compile.

In this container runbench fell from 813,607,261 to 809,811,674 instructions
(-0.47%), with byte-identical output, and `tally_4` no longer calls the
index. The ratchet row `index_inline` takes the `always_inline` away and
reads 813,606,898.

## 2026-10-24 — the number span inlined into the decoder

The same move for `k_b_number_span_raw`, the word-at-a-time scan the
decoder's number twin calls at every number, 417,483 times on runbench. It
returns an `i64`, so the call's type already matched, and only the definition
had to move. It is defined in the hot unit under the same `K_HOT_ELSEWHERE`
guard, with `k_nondigits8`, `k_digits_end` and the `k_number_bytes` table
beside it. `k_number_span_scan`, the RFC checks it hands everything else to,
stays out of line and now has external linkage.

In this container runbench fell from 809,811,674 to 804,150,430 instructions
(-0.70%), with byte-identical output, and no function in the program calls
the span any more. The ratchet row `number_span_inline` takes the
`always_inline` away and reads 809,811,999.

## 2026-10-24 — the slice parsers inlined into the decoder

The decoder's number twins hand each number to `k_b_to_int_slice_raw` or
`k_b_to_float_slice_raw`, 417,483 calls on runbench. A whole-runtime LTO link
had shown about 6.5M instructions there once both were inlined. Both now live
in the hot unit, with the integer reader, the float shape test, eisel-lemire
and their small tables beside them. Eisel-lemire's table of powers stays in
`runtime.c` and is read through an extern, and so are the two slow paths,
`k_b_to_int_slow` and `k_to_float_scan_out`. `k_int` and `k_float` are plain
functions in `runtime.c`, so the unit takes a static copy of each. Both
parsers return a `KValue`, so the twins call them as returning
`{ i64, i64 }` and rebuild the `%KValue`, as the string index does.

The float parser had been a `preserve_none` door, worth 1,113,877 when it
was a call. Inlined, its convention no longer matters, so it is back on the
C convention and off `PRESERVE_NONE_DOORS`.

In this container runbench fell from 804,149,817 to 797,455,326 instructions
(-0.83%), with byte-identical output, and neither parser is called anywhere
in the program. The ratchet row `slice_parsers_inline` takes both
`always_inline`s away and reads 805,258,746.

## 2026-10-24 — the hot unit's copies made inline-only

CI's first reading of this branch put the release codegen row at 468,610,053
against main's 425,508,082, +10.1%. All of it was in `ld.lld`. The corpus
program calls none of the new helpers, so none of it was inlining. The hot
unit defined each helper as an ordinary function, so every release link
optimized it and compiled it to machine code whether the program called it
or not. Main paid the same for the four helpers the unit already held.

Each helper is now compiled into the cached runtime as machine code, and the
hot unit's copy is a C99 inline definition, which clang emits as
`available_externally`. The link inlines it where the program calls it and
drops it everywhere else. Clang emits an inline definition only where its
unit refers to it, so the unit ends with a table that refers to all eight.
The table has external linkage, which keeps it through the unit's own
compile, and nothing reads it, so the link removes it. With `used` on the
table, the link kept all the bodies and optimized them. `K_HOT_ELSEWHERE`
has no users left and is gone.

Linking the codegen corpus alone, `ld.lld` read 252,430,133 on main,
295,287,065 with the eight helpers defined outright, and 187,756,033 with
the eight inline, 25.6% under main. runbench reads 797,155,085, 300,195
below the eight defined outright, with byte-identical output, and the program
calls none of the eight.

Two fixes came with it. The hot object's cache key was the runtime's digest
alone, so a change to `hot_source_of` reused an object the old recipe made;
the key now also carries a digest of `src/main.rs`. And the hot source was
built on every release build to be handed to a cache that nearly always had
it. It is built now only on a miss, and kanso's own process on the corpus
build fell from 74,931,992 to 30,665,523 instructions.

The four ratchet rows that took an `always_inline` away now put `noinline` in
its place, since an inline copy without the attribute can still be inlined at
the link's threshold of 2000. Dry-run on a copy against 797,155,085, with the
same output each time: `index_inline` reads 800,950,764,
`number_span_inline` 804,913,953, `slice_parsers_inline` 804,964,001, and
the byte scanners' row 822,443,871.

CI's rows for this branch against main, all taken into the goldens:

- runbench 813,606,641 -> 797,155,036 (-2.02%), jsonbench 578,723,373 ->
  560,280,234 (-3.19%), indexbench 1,947,279 -> 1,837,323 (-5.65%),
  deepbench 201,099,671 -> 200,299,696, oneshot 10,256,694 -> 10,133,710,
  livebench 961,524,285 -> 961,402,064 and widebench 28,476,897 ->
  28,461,000.
- Five work rows rose by 111 or fewer, which I have not isolated:
  `work_basket` 31,068,030 (+48), `work_digestbench` 4,152,335 (+111),
  `work_escapebench` 40,308,408 (+41), `work_readbench` 4,582,008 (+104)
  and `work_scanbench` 274,369 (+41).
- The release codegen row 425,508,082 -> 360,870,405 (-15.2%).
- `codegen_instructions_dev` 116,281,615 (+63,185) and `emit_instructions`
  5,652,483 (+1,239), from the rewrap lines the emitter now writes.
- The tab's rows rose for the same lines: `browser_compile_instructions`
  221,032,408 (+109,092) and `browser_run_instructions` 33,404,578
  (+31,680).
- `text` 5,419,472, up 97,154. Every binary grew by about 3,440 bytes, the
  runtime's own copies of the four helpers the hot unit used to define, and
  the four decoding benchmarks by 15,113 to 16,999 with the parsers inlined
  at their sites.
- `emitted_lines` 14,304 (+8) and `emitted_other_lines` 160,289 (+32): 4 to
  12 more lines in the five programs that index strings or read numbers.
- No allocation counter moved. Welfare 93.14 -> 93.24, banked.

Measured and set aside: the renderer's whole-float test without `floor`,
as a cast to `long long` and back. A sweep of 32,001,840 doubles agreed with
the floor form, and runbench rose 190,766. glibc resolves `floor` at load to
`__floor_sse41`, three instructions a call on this host, and the cast pair
cost one more per float.

Three ratchet mutations needed rewriting after the move. The door list is
five names now, and its mutation reads the length from the source instead of
writing it down. The cold helpers' mutation strips `preserve_most` from the
runtime's definitions, and the hot unit declares two of them, the index's
wide miss and the int parser's tail, with the attribute. Left alone, the
program would call a definition that no longer saves what its declaration
promises; the mutation now strips both declarations too, and runbench reads
801,182,221 with identical output. The row that inlined the index's two cold
arms back into it is retired. The program's index is now the hot unit's
copy, which holds only a declaration of the wide miss, so nothing in the
runtime can put that arm back in the walk's frame. Inlining both arms in the
runtime read 797,121,834, below the branch's 797,155,085.

The ratchet's shards found one more row the move left with nothing to
watch: the list test put back in front of `k_b_at`'s string arm. No
benchmark calls `k_b_at` any more, because the emitter sends every proven
string index to `k_b_at_str`. With the mutation the function's code
changes, and all fourteen work rows read the same as without it. The row
is retired; `a proven string index through the tag tests` still guards
the routing that moved those calls.

## 2026-10-24 — the empty list and map inlined into the decoder

The decoder opens 272,349 arrays and 273,339 objects on runbench, and each
called `k_list_empty` or `k_map_empty`: 15 and 17 instructions, mostly an
arena bump and a header written into it. Both now have inline copies in the
hot unit with what they reach (the seed layouts, `k_buf_set_cap`, `k_ptr`),
and the emitter calls them returning `{ i64, i64 }` through `call_pair`,
which the string index uses too. The cold paths for a short arena or a
counting build stay in the runtime as calls.

Without `always_inline` the link would not inline them. Its remark was
"conflicting attributes", which `alwaysinline` overrides; that is why the
earlier helpers all carry the attribute. The conflict is the hot unit's
`+ssse3`, which the LTO link's target does not carry: given
`-plugin-opt=-mattr=+ssse3`, the remark became an ordinary cost decision.
That flag moved runbench by +951 with byte-identical output, so the
attribute stays and the flag was set aside.

runbench 797,155,085 -> 795,184,756 (-0.25%), byte-identical output, and no
function in the binary calls either. The ratchet row `empty_inline` puts
`noinline` in place of both `always_inline`s and reads 797,156,333.

Measured and set aside: `k_rec` the same way, 266,978 calls at 24.9
instructions. runbench rose to 809,634,970 (+1.82%) with the same output.
Inlined, the record constructor made `json/obj_value` too large to inline
into `obj_key_start`, so `obj_value` became a call of its own and cost
58,253,679 where `obj_key_start` gave back 37,835,883.

CI's rows for this branch against the string index's, all taken into the
goldens:

- runbench 797,155,036 -> 795,184,707 (-0.25%), jsonbench 560,280,234 ->
  557,386,434 (-0.52%), deepbench 200,299,696 -> 200,095,696, digestbench
  4,152,335 -> 4,122,786 and oneshot 10,133,710 -> 10,114,082. The other
  work rows fell by 20 to 5,610. Two of them still sit 21 above main, from
  the string index's rise: `work_escapebench` 40,308,388 and
  `work_scanbench` 274,349.
- `codegen_instructions_release` 360,870,405 -> 370,135,769 (+2.6%), for
  the two bodies the link now inlines at every empty list and map.
- `codegen_instructions_dev` 116,420,279 (+138,664), `emit_instructions`
  5,666,973 (+14,490) and the compile corpus's `module_lines` 1,175 (+4),
  from the rewrap lines `call_pair` writes at each site.
- The tab's rows rose for the same lines: `browser_compile_instructions`
  221,541,805 (+509,397) and `browser_run_instructions` 33,432,724
  (+28,146).
- `text` 5,464,764 (+45,292), the inlined bodies at their sites.
- `emitted_lines` 14,368 (+64) and `emitted_other_lines` 160,865 (+576):
  every program that builds an empty list or map.
- No allocation counter moved. Welfare 93.24 -> 93.24, a rise under the
  second decimal, banked.

## 2026-10-24 — a builder append asks for room first

The emitter's in-place append twins tested whether the builder owned its
buffer, then whether the builder ended at the buffer's frontier, then whether
the bytes fit. The first test exists because a borrowed view has no buffer
header to read. But a view's capacity is zero, so the room test turns it away
on its own, and if it comes first the header is read only for an owned
buffer. Six twins add a constant to the length in their room test (one byte,
two bytes, a key's or a string's quotes, a word) and now ask for room first.
Two add only a length, which may be zero, and keep the ownership test: an
empty append onto an empty view would pass a room test alone.

Against main in this container, with byte-identical output: runbench
813,606,690 -> 807,626,521 (-0.74%) and encodebench 2,353,427,724 ->
2,325,267,437 (-1.20%). The encoder appends a separator or a bracket between
almost every value, so most of the fall is there.

The micro program `an_append_onto_a_view_never_claims_it` appends onto the
empty bytes, a literal's bytes and a slice, on all three engines. With every
one-byte room test weakened to admit an empty view it printed ` abB` where
the answer is `CBA abBA`. The ratchet row `room_first` puts the ownership test
back in front of the room test in `k_b_append_mut_int`, the twin the
encoder's separators go through, and reads runbench 811,580,683 and
encodebench 2,346,183,576.

CI's rows for this branch against the empty collections', all taken into
the goldens:

- runbench 795,184,707 -> 789,204,097 (-0.75%), encodebench 2,353,400,269
  -> 2,325,240,311 (-1.20%), livebench 961,382,268 -> 936,031,367 (-2.64%)
  and oneshot 10,114,082 -> 10,051,248.
- `work_widebench` rose 28,460,996 -> 28,508,996 (+48,000), which I have
  not isolated.
- The twins' text grew by a few lines where a block moved: the compile
  corpus's `lines` 1,593 (+24) and `module_lines` 1,181 (+6),
  `emitted_lines` 14,371 (+3) and `emitted_other_lines` 160,895 (+30).
  `codegen_instructions_dev` 116,426,119 (+5,840) and
  `codegen_instructions_release` 370,142,417 (+6,648) follow them.
- `emit_instructions` 5,665,979, `browser_compile_instructions`
  221,423,495, `browser_run_instructions` 33,345,250 and `text` 5,462,380
  fell from the empty collections' values. All but the tab's run row stay
  above main, from the two branches below this one.
- No allocation counter moved. Welfare 93.24 -> 93.27, banked.

## 2026-10-24 — lines found a word at a time, and IR trimmed as ASCII

A per-function count of the tab's compile of `bench/interp_corpus` put
`memchr_aligned` at 5,857,105 instructions and `trim_matches` at 1,689,320.
Both came from walking text one line at a time with the standard library.

- `ir_defines`, which splits emitted IR into definitions for the prune,
  stepped with `split_inclusive('\n')`. That calls memchr once a line, and on
  wasm32 memchr reads four bytes a step. It now finds each line with
  `compare::byte_from`, eight bytes a step, and trims only a line that opens
  with `}`.
- `prune_shipped`, which drops the library declarations a program does not
  reach, walked the library's source the same way and now steps the same way.
- The tab's translator trimmed every line of IR with `str::trim`, which
  decodes each character at both ends and asks the Unicode table. Emitted IR
  is ASCII, so both of its per-line trims are `trim_ascii` now.

Measured with the per-function count, which runs a named build and reads a
few hundred thousand above the browser row: 221,423,495 -> 219,980,398 for
`ir_defines`, -> 219,053,485 for the trims, and -> 218,810,755 for
`prune_shipped`, -2,612,740 (-1.18%) in all. Natively the entry check fell
from 75,017,916 to 74,717,928, because `prune_shipped` runs there too. CI's
rows replace these.

The ratchet row `prune_lines` puts the per-line `str::find` back in
`ir_defines` and reads 220,266,296 on the same count; `ascii_trim` puts
`str::trim` back on both of the translator's trims and reads 219,737,668.

CI's rows for this branch against room-first appends, all taken into the
goldens: `browser_compile_instructions` 221,423,495 -> 218,810,755 (-1.18%),
`compile_instructions` 25,172,884 -> 25,077,347, `entry_instructions`
74,435,723 -> 74,133,494, `library_instructions` 74,096,953 -> 73,794,737
and `emit_instructions` 5,665,979 -> 5,605,884. The native rows fell with
`prune_shipped` and `ir_defines`, which run there too. Nothing else moved.
Welfare rose under the second decimal, 93.27 as printed, and is banked.

## 2026-10-24 — the lexer reads a line before it walks it

The entry compile spent 9,277,662 instructions in the lexer, and about a
quarter of that came before any token: the work `lex` does on each raw line
to decide what kind of line it is. Five pieces of it are cheaper now, with
every diagnostic unchanged.

- The source is split into lines a word at a time, with
  `compare::byte_from`, where `str::lines` called memchr once a line. A
  `\r` before a newline still comes off, and one at the end of a file
  without a newline still stays.
- The tab test is `compare::find` on the bytes. `str::find('\t')` went
  through the char searcher and a `bcmp` for every line.
- The line is trimmed once, where it was trimmed twice.
- The width cap counts a line's characters only when its bytes exceed the
  cap, since a character is at least one byte.
- The text-block opener and the span opener each walked every byte of a
  code line, tracking quotes and stopping at `#`. A line with no quote and
  no `#` has no text block and its span opener is its last byte, so one
  scan for either byte, a word at a time, answers both for most lines.

Measured in this container at the tip of the lines-a-word-at-a-time branch:
`entry_instructions` 74,133,494 -> 71,498,670 (-3.55%) and
`compile_instructions` 25,077,347 -> 24,192,780 (-3.53%).

The ratchet row `lex_lines` puts `str::lines` back and reads
`compile_instructions` 24,363,416; `lex_openers` calls no line plain and
reads 24,561,382.

CI's rows, all taken into the goldens: `entry_instructions` 74,133,494 ->
71,498,670 (-3.55%), `compile_instructions` 25,077,347 -> 24,192,780
(-3.53%), `library_instructions` 73,794,737 -> 71,147,167 (-3.59%),
`browser_compile_instructions` 218,810,755 -> 216,626,340 (-1.00%), since
the tab lexes the same way, and `emit_instructions` 5,605,884 -> 5,603,524,
whose process lexes its program before it emits. Nothing else moved.
Welfare 93.27 -> 93.28, banked.

What is left in the lexer is `lex_line` itself, about 1,150 instructions a
code line, most of it a scanner that decodes each line into a `Vec<char>`
before it reads a token. A scanner over bytes would touch every token
method and the column arithmetic, so it is a change of its own.

Measured and set aside: the scanner reading an ascii line's bytes in place
instead of widening them into its `Vec<char>`, through accessors that test
`ascii` at every read. The widening costs 348,608 instructions of the entry
compile, and the test at every character cost more: `entry_instructions`
71,498,670 -> 71,603,369 and `compile_instructions` 24,192,780 ->
24,224,312.

Measured and set aside: an unchecked `u64` sum for the tab translator's
integers of eighteen digits or fewer, ahead of the checked `i128` loop.
`browser_compile_instructions` fell 187,947 (0.09%), too little to carry a
ratchet row of its own.

## 2026-10-24 — kq under the pin to 29dafd0f

kq's instruction rows rose when its pin moved to 29dafd0f, the string index
and its company: print_small 25,683,047 -> 25,774,281 (+0.36%) and
path_small 10,572,005 -> 10,663,669 (+0.87%), with the big rows in step. Two
CI runs on two different CPUs read the same four rows, and building kq here
with kanso before and after #2077 gives the same rise within a few hundred.

The rise is in kq's decoder, around the string scan. `Flat3_string_end`
keeps an overflow-checked increment the earlier build folded into a `lea`,
and `parse_string` keeps the same check and a test of a constant zero. The
IR kanso emits for kq is the same under both pins, apart from the
declarations #2077 retyped, so the folds are lost in the hot unit or at the
link. Every `find2` site in kq is inlined since #2077, where 13 were calls
before.

Measured and set aside: the two scanners without `always_inline`. The link
then left 6 calls, and kq read print_small 26,132,733 and path_small
10,847,487, against 25,776,035 and 10,663,837 with the attribute, so the
forced inlining is not what costs kq. The cause is not isolated.

## 2026-10-24 — a chain built from the left runs past the beat stack

The twenty ports of 2026-10-10 were run again on main (78f72a58). Every one
stopped at its first compile error, under rulings made since, so each was
migrated and its journal re-tested; the re-sit has its own entry. Two ports,
rg and site, hit the same native crash, and this entry fixes it.

rg walks a directory by folding `.>` onto an effect, once per entry:
`list/fold entries (effect 0) added`, where `added` answers
`chain .> (acc -> effect (acc + x))`. That builds a chain nested to the left,
and native code runs it by entering `k_exec` for the inner chain from the
outer one, a frame per link. The interpreter printed the sum at 100,000 links.
Native code stopped at 64 with "ran out of stack", on 8 MiB or on 64 MiB.

Two faults were behind it, and both were in `k_exec`.

- Each chain pushes a beat mark and then reads and writes
  `k_carries[k_beat_depth - 1]` without asking whether the depth is inside the
  stack's 64 marks. A sanitizer build named it: a global-buffer-overflow write
  at index 64 of `KCarry[64]`. The write lands on whatever is laid out past
  the array, and the crash comes later: gdb stops it in `k_beat_pop_slow`, in
  a loop that frees blocks, 11 frames deep. A chain entered past the last mark now
  runs in `k_exec_unmarked`, which does what a beat past the stack has always
  done: it keeps count and stages nothing, and its garbage waits for an outer
  mark.
- The socket read held a `char buf[65536]` inside `k_exec`'s switch, so every
  `k_exec` frame reserved 64 KiB, including the chain's own frames. 63 links
  needed 4,126 KiB of stack. The read is now `k_conn_read`, out of line, and
  10,000 links need 5,790 KiB on release and 3,607 KiB on dev, about 580 and
  360 bytes a link.

`micro/a_chain_built_from_the_left_runs_past_the_beat_stack` folds 1,000 links
and prints `total 500500` on every engine. Main stops it with "ran out of
stack". With the frame fix alone and the depth guard removed, it stops the
same way. A chain of 100,000 links still runs out of native stack: the
interpreter runs on a 1 GiB thread and native code on the 8 MiB the system
gives, as any deep recursion does. Running a left-built chain without
recursion means reassociating `(m .> f) .> g` at run time, and a rescue step
there would lose the failure it is owed, so it is not attempted here.

A sanitizer build of a dev program linked the way `scripts/asan/sweep.sh`
links one segfaults on main as well as on this branch, at 20 links, before
printing anything. The sweep's own `--prove` was not run, and nothing here
claims a clean sweep.

CI's rows. Every benchmark does less work, because a frame that reserved
64 KiB no longer does: work_runbench 789,204,097 -> 788,981,005,
work_deepbench 200,095,696 -> 198,788,407, work_widebench 28,508,996 ->
28,461,019, work_livebench 936,031,367 -> 936,030,078, work_encodebench
2,325,240,311 -> 2,325,240,037, work_pendbench 155,738,415 -> 155,737,991,
work_oneshot 10,051,248 -> 10,050,617, work_readbench 4,582,008 ->
4,581,989, work_jsonbench 557,386,434 -> 557,386,424, work_digestbench
4,122,786 -> 4,122,778, and work_basket, work_escapebench, work_indexbench
and work_scanbench one each, to 31,062,419, 40,308,387, 1,837,322 and
274,348. Every binary's text grows 2,576 bytes for the two new functions,
runbench's to 762,112, and text, the sum over fourteen binaries, from
5,462,380 to 5,498,444. Linking the codegen corpus compiles the larger runtime:
codegen_instructions_dev 116,426,119 -> 116,447,795 and
codegen_instructions_release 370,142,417 -> 370,219,514. Nothing else moved.

## 2026-10-24 — the author's own name wins on the paths the 10-12 fix missed

The 2026-10-12 entry "a name the author declared wins over a builtin and an
import" fixed three reductions from the ports. The re-sit found two more
paths the rule did not reach, from three ports.

- **Native code, a direct call.** ugit and sat declare `fn step a b` and
  import std/list, which exports a record `step` and enrols it under the
  bare name. Under `kanso play`, native code built the record:
  `print "{step 1 2}"` printed `list/step 1 2`, where the interpreter
  printed `3`. The 10-12 spec passed the function as a value
  (`apply counting`), which already worked; a call named directly reached
  `emit_call`'s constructor branch first. The branch now asks
  `constructs`, which gives an enrolled type way to a group the program
  declares, as the interpreter's `callee` does. std/list's own
  constructions are unchanged: `iterate`, `naturals` and `fold`, which
  build `step` and `counting` records, print the same on both engines
  with a local `step` and `counting` in the file.
- **The interpreter, a constant.** make declares `fn read_line at s`, and
  std/io exports `read_line`, a constant over `builtin_read_line`. The
  interpreter evaluates a call's head as a value, and `resolve` took the
  enrolled constant, and behind it the described builtin of the same
  name, before the file's own group. The call stopped with "`<io>` is not
  callable", and 57 of make's 82 fixtures went red on that engine. A group
  with an arm the program declares now wins both rungs.
- **Both engines, a type.** mustache declares `type env` and imports std/os,
  whose `env` forwards to a builtin. Two front-end passes rewrote the file's
  `env "x"` before either engine saw it: `canonicalize_bare_aliases` sent
  the bare name to `os/env`, because the only declarations it counted were
  functions, and `apply_wrappers` sent that on to `builtin_env`. The
  interpreter printed `<io>`; native code still built the record, by a path
  this did not reach. Both passes now leave a name the program declares as a
  type, and each alone leaves the interpreter printing `<io>`.

`tests/golden/play/a_function_named_like_an_imported_type` gains the direct
call, and main prints `2 list/counting 3 4` for it natively.
`tests/golden/play/a_function_named_like_an_imported_constant` is make's
shape; main stops it on the interpreter.
`tests/golden/play/a_type_named_like_an_imported_function` is mustache's, and
main prints `<io> 0` for it on the interpreter. `play_verb.rs` runs all three
on both engines, and the walk over the play corpus now reads twelve goldens.

One shape from the same family is not fixed here. kv's module form, a
function named `read_line` passed by value to `list/fold` in a module that
imports std/io, stops every engine with "`<io>` is not callable", on main and
on this branch. The module's own name is lost in `qualify`, a path none of
these fixes touch.

CI's rows. The front end asks the two new questions of every program it
compiles, and the tab compiles with the same front end:
compile_instructions 24,192,780 -> 24,230,258, entry_instructions 71,498,670
-> 71,606,414, library_instructions 71,147,167 -> 71,254,677,
emit_instructions 5,603,524 -> 5,606,362, interp_instructions 385,492,276 ->
385,500,095 and browser_compile_instructions 216,626,340 -> 216,735,094.
Nothing at run time moved. Welfare fell by less than a hundredth, and the
floor follows: this makes the engines agree, which the welfare script's own
header puts ahead of the score.

## 2026-10-24 — a module's own function beats an imported constant of its name

#2090's first item, from the kv port. A module that imports std/io and
declares `fn read_line acc x`, then hands `read_line` to `list/fold`, stopped
on every engine with "`<io>` is not callable". #2084 fixed the play-file form
of this in the interpreter's `resolve`. A module takes another path.

`qualify` gives a name the module declares and an import also enrols a
spelling of its own, the shared bare space (`m/~read_line`), where the
module's own arms join the import's twin as synthetic clones. #2084's test
for an arm the author wrote finds only clones there, so it never fires, and
std/io's `read_line` is a constant, which answers any mention of the name
read as a value. A constant has no arity a call could dispatch on, so it
cannot share a name with a function by overloading. `qualify` now drops an
import's constant twin wherever the module declares the name, and the name
takes its plain qualified spelling. A module that declares no `read_line`
still reads std/io's bare.

`micro/a_modules_function_named_like_an_imported_constant` prints `6` on
every engine, and main stops it with "`<io>` is not callable".
