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

## 2026-10-27 — gavel: templates are Slim-like, and they compile

#2178. Clay, on how the framework's pages are written: "well you can find a
way to make the templates compilable but they should be like slim or Haml".
A template is markup structured by indentation, in the Slim family, and it
compiles to kanso, so the checker reads its field accesses and its
parameters the way it reads any other code. How a database gets in and what
the first application is stay open on #2178.

The prototype is on branch claude/kata-prototype under design/kata, a
generator written in kanso. A template becomes one function. Each `- if`
branch and each `- for` body becomes a helper taking exactly the names in
scope it uses, because kanso refuses an unused parameter, and a body that
uses none becomes a constant. A misspelled field in the sample page is
refused by `kanso check` at the generated line, and both engines print the
same page.

## 2026-10-27 — a dependency's diagnostic quotes its own line

The merged check names the file a diagnostic is about, and a module compile
rereads its files on the failing path so that each diagnostic quotes its
line. The root compile in src/lib.rs handed the renderer only the entry's
text, so a field misspelled in an imported module printed a header and a
location with nothing under them. The template prototype found it, when its
typo landed in a generated view. `render_merged` now rereads each file a
diagnostic names, on the path that is about to fail. An embedded std file
is named bare and is never read from disk, because a file of that name in
the working directory belongs to somebody else.

tests/golden/errors_module gains a_field_read_in_a_modules_second_file,
which on main printed no line. field_read_in_a_deep_library and
two_owners_tie_on_a_traveling_call gain the line their goldens had pinned
as missing.

## 2026-10-26 — gavel: an entry reads its key and value by name

#2163, option 1. An entry is an ordinary record, so `e.key` and `e.value`
read by name on every engine and are checked like any record's fields. The
pattern `(entry k v)` keeps working. Clay asked about Ruby's two-argument
blocks (`|k, v|`); destructuring the entry in a parameter already gives that,
and a two-argument walk over maps would need a twin of every list function.

## 2026-10-27 — an entry reads its key and value by name

#2163. A field read becomes a call on the field's getter, and getters were
made only for fields some type in the program declares. `entry` is built
into the language, so `e.key` had no getter: the checker refused it, and
`e.value` got past the checker only when std/list happened to declare a
`value` of its own, then failed in the interpreter. The checker now counts
`key` and `value` as declared, the way it counts an err's three readers.
`prune_unused_getters` already gathers every getter a program names, and
when `Get_key` or `Get_value` is among them it writes that group an `entry`
arm, keeping one a dependency already wrote. A program that never reads the
fields builds neither arm. The first version built both in every compile and
pruned them after, which CI read as 63 more allocations in the interpreter's
corpus and 502,143 more browser compile instructions.

tests/golden/micro/an_entry_reads_its_key_and_value_by_name reads both
fields beside a local type that declares a `key` of its own; on main it was
refused at `e.key`. tests/golden/errors/an_entry_has_no_field_but_its_key_and_value
pins the refusal of a field an entry does not have. Appendix B names the two
fields.

CI's rows, on main after #2179. The tab compiles the corpus in fewer
instructions, browser_compile_instructions 353,638,408 -> 353,555,358, and
the interpreter's run allocates less, interp_allocs 653,377 -> 653,375, as
does the front end, compile_allocs 13,940 -> 13,939; nothing here isolates
why any of the three fell. The compile rows rise with the new code in the
checker and the prune: compile_instructions 24,871,095 -> 24,874,251,
entry_instructions 73,693,860 -> 73,701,590 and library_instructions
73,331,088 -> 73,338,936. Welfare rises within its second decimal and is
banked.

## 2026-10-27 — the compiler is built as one codegen unit, with fat LTO

The release profile had no settings, so rustc split the crate into sixteen
codegen units by module and optimized each without the others. Cargo.toml
now asks for one unit and fat LTO. The optimizer sees the whole crate and
its dependencies when it inlines, and a release build takes about a minute
longer, 2m34s against 1m41s on this container.

It came out of #2137. Moving call emission out of emit.rs had been declined
because the browser row moved 52 units with no code changed, and the guess
was the file path in panic strings. Neither a longer path nor a shifted line
number moves the row. The units do: they are cut along module lines, so a
move between modules changed what was inlined. With one unit that coupling
is gone, and the split can be retried.

The interpreter gate anchors its count on `run_interpreted_on_stack`, and the
one-unit build inlined it into its thread's closure, so the gate read a drop
glue carrying its name. It is now `#[inline(never)]`, as `emit_ir_for` is.

CI's rows, on main after #2180. Every row the compiler's own build reaches
falls: compile_instructions 24,874,251 -> 24,057,877 (-3.28%),
entry_instructions 73,701,590 -> 71,165,551, library_instructions
73,338,936 -> 70,852,827, emit_instructions 5,715,797 -> 5,597,787,
startup_instructions 52,528 -> 50,254, interp_instructions 577,237,254 ->
571,649,741 and, in the tab, browser_compile_instructions 353,555,358 ->
343,131,848 (-2.95%). codegen_instructions_release moves 371,551,607 ->
371,551,576; it counts clang and the linker, and nothing here isolates why
they did 31 fewer. No allocation, memory or runtime row moved. Welfare rises
95.55 -> 95.57 and is banked.

## 2026-10-27 — call emission moves from emit.rs into calls.rs

#2137. The methods from `emit_call_full` to the end of emit.rs form a second
`impl Backend` block in src/codegen/calls.rs, read with include! right after
emit.rs, so they stay in the codegen module. emit.rs falls from 4,324 lines
to 3,001.

The same cut was declined before #2181 because the browser row moved 52
units with no code changed. Built as one codegen unit, the compiler binary
now differs only in the file names its panic messages carry, and on this
container, which reads CI's rows exactly, compile_instructions, the entry,
library, emit and interpreter rows and the browser row all read what main's
goldens hold.

## 2026-10-27 — the unbuilt-arm prune moves from codegen.rs into unbuilt.rs

#2137. `without_unbuilt_arms`, `kept_arms`, `Given` and `UnreadPositions`
move into src/codegen/unbuilt.rs, read with include! where they stood, so
the items keep their order and their module. codegen.rs falls from 4,231
lines to 3,987. One ratchet mutation patched the prune by its text and now
names unbuilt.rs. As with calls.rs, the compile, entry, library, emit,
interpreter and browser rows read what main's goldens hold on this
container.

## 2026-10-27 — FnEmit moves from codegen.rs into fn_emit.rs

#2137. `FnEmit`, the impl that keeps it, and the helpers beside it
(`release_cells`, `writes_state`, `is_a_stack_slot`) move into
src/codegen/fn_emit.rs, read with include! where they stood. codegen.rs falls
from 3,987 lines to 3,443. Three ratchet mutations patched FnEmit's methods
by their text and now name fn_emit.rs. The compile, entry, library, emit,
interpreter and browser rows read what main's goldens hold on this container.

## 2026-10-28 — gavel: senro, with kata inside it, Postgres over the wire, and a guestbook first

#2178. Clay ruled four things about the web framework:

- It is named senro (線路, railway track).
- kata, the Slim-style template compiler, lives inside senro as its view
  layer.
- The database is Postgres, reached by a client written in kanso that speaks
  the wire protocol over a socket. Clay: "I just want to do postgres. I don't
  know why any kind of sea binding is necessary." Development starts against
  Docker, and TLS for hosted connections comes from the runtime, backed by
  OpenSSL natively and a Rust TLS library in the interpreter.
- The first application is a guestbook.

The build order is cloud's: `net` dials out and reads bytes, then the
Postgres client, then derivations (#2186), then routing with kata, then
models.

## 2026-10-28 — gavel: a framework's conventions are derivations

#2186. Clay asked for "some language construct that can allow you to do
compile time mapping", so that senro can map controller functions to routes
and model names to tables the way Rails does, and the compiler can check the
result. A derivation is a pure kanso function that a framework ships and the
compiler runs on the interpreter during the build. It reads a description of
the program and returns declarations built from constructors the compiler
defines (a route, a record type, a table binding), each naming the source
item it came from. The compiler adds them to the program and checks them like
any other code, and `kanso derived` prints them.

Inside a derivation any computation is allowed, including pluralizing a
model's name. What comes out is a finite list that is checked against real
functions and real tables, which is what makes an arbitrary convention safe
to run.

A pub function in `app/` that matches no route is a compile error. Clay
first agreed to a schema file written in kanso, then asked why Rails keeps a
schema file at all. Rails keeps `schema.rb` mainly because old Ruby
migrations stop running when the models they call change. A kanso migration
is a value built from a fixed set of changes, so it cannot break that way,
and the ruling became: no schema file by default, the schema folded from the
migrations at compile time, and `schema.sql` only for a migration that needs
raw SQL.

## 2026-10-28 — net dials out and reads bytes

#2178, the first item owed on the Postgres ruling. `net/dial host at` opens
a connection and answers the same `conn` record `accept` does, so `write`,
`read` and `close_conn` work on it unchanged. `net/read_bytes c` is one read
of what has arrived, handed back as bytes. A Postgres message is framed in
binary, and `net/read`, which answers text, would mangle a byte such as 0xff.

Both are effect builtins on both engines with sockets: `k_dial` resolves the
host with getaddrinfo and tries each address it gets, and the interpreter
uses `TcpStream::connect`. A refusal is worded "cannot connect to host:port"
on both, with nothing from the operating system in it, so the differential
law holds on the failure too. The browser has no sockets and refuses as it
already did for `listen`.

tests/a_program_dials_out.rs plays the server: it answers a greeting with
nine bytes framed like a Postgres message, one of them 0xff, and both engines
print the same nine values. Watched red with the native byte read decoding
as text (the 0xff arrived as U+FFFD) and with the interpreter's refusal
worded differently. tests/sockets_say_one_thing.rs gains `read_bytes` on a
handle that is not a connection.

Every binary links the runtime, so each benchmark's text grows by 752 bytes
(text 5,635,099 summed), and its instructions by a few hundred, which is
layout: no benchmark dials out. They land at work_basket 18,295,655,
work_deepbench 198,243,012, work_digestbench 4,104,311, work_encodebench
2,335,339,667, work_escapebench 40,308,822, work_indexbench 1,887,608,
work_jsonbench 549,337,631, work_livebench 938,655,028, work_oneshot
9,979,724, work_pendbench 154,124,432, work_readbench 4,583,812, work_runbench
825,011,048, work_scanbench 266,891 and work_widebench 28,701,733.

`lib/net` is embedded in the compiler, so the compile rows rise too:
compile_instructions 24,061,034 (+3,157), entry_instructions 71,170,086
(+4,535), library_instructions 70,857,236 (+4,409), emit_instructions
5,599,525 (+1,738), codegen_instructions_dev 116,797,274 (+63,365),
codegen_instructions_release 371,551,798 (+222), interp_instructions
571,786,910 (+137,169), browser_compile_instructions 343,160,078 (+28,230)
and browser_run_instructions 64,598,060 (+27). Welfare fell by less than
0.01, and the floor moves to 95.568 under the rule for a feature a ruling
requires.

## 2026-10-28 — derivations: a convention runs while the program compiles

#2186, the first slice of the ruling. An entry may write `derive
conventions/routing` after its imports. The compiler builds a
`derive/program` describing the app's own modules, which are the ones the
entry imports by a `./` path. Each module is listed with its pub functions,
their parameters and annotations, and the function itself, and with each
record type and its fields. The compiler calls the named function on the
interpreter and reads back a list of declarations. `derive/declare` makes a
constant in the entry, written out as the literal its value spells, with a
function written as its name, so a route table names real handlers and the
checker resolves each one. `derive/refuse` makes a compile error at the item
it names, in that item's own file.

The derivation's module is compiled as an ordinary dependency. No executor
is supplied, so a derivation that answers an effect is refused. A name in
the deriving module that reaches a module it describes is refused where it
is written, because the description is the whole of what a derivation may
know about the app; what it declares then follows from what `kanso derived`
shows. The modules it describes count as used imports, because the
convention reads them. `kanso derived` prints each declared constant with
functions spelled by name.

The derivation runs on an interpreter that copies at every container write.
The first build asked the linearity analysis which writes could go in place,
and that analysis covers every function the program holds: on
bench/entry_corpus it was 24M of the derivation's 25.8M instructions. Without
it the derivation costs 1.8M.

tests/golden/derive holds nine programs, each run on both engines: a route
table that dispatches to real functions, a misspelled action refused on its
own line, a library beside the app left out of the description, and six ways
a derivation can be wrong. It was watched red with function references
written as none, with `../` libraries described, and with a derivation that
imported the module it described. bench/entry_corpus now derives a small
route table, so the entry row, and with it welfare's compile term, prices
what a derivation costs.

The entry row rises to entry_instructions 83,728,840 (+12,558,754): the
corpus gained two modules and a derivation. `lib/derive` is embedded in the
compiler, so the other compile rows move by layout: compile_instructions
24,064,178 (+3,144), library_instructions 70,863,285 (+6,049),
codegen_instructions_release 371,551,844 (+46), emit_instructions 5,598,851
(-674), interp_instructions 571,786,909 (-1), compile_peak_bytes 598,588 (+96)
and browser_compile_instructions 343,190,365 (+30,287). Welfare fell by 0.018
and the floor moves to 95.550 under the rule for a feature a ruling requires;
the fall is past what `--set` takes, so the floor was edited by hand and
`--set` recorded the reason.

Still owed on #2186: the browser compile's derivation golden, a record type
or table binding as a declaration, and the senro derivations themselves.

## 2026-10-28 — a derivation reads constants, declares types, and runs in the page

#2186's second slice. A module in a derivation's description now carries
`constants`, mapping each pub constant's name to its value, and constants
leave `functions`. senro/models needs this: an app keeps its migrations as a
pub constant, and a constant's function reference could not be called, so
the fold had nothing to read. A routing convention also stops seeing a
constant as an action.

`derive/declare_type origin name fields` adds a record type to the entry,
with getters for its fields, and `kanso derived` prints it as `type name
fields`. A name or field a program could not write, or fields that are not a
list, is refused. With it the ruling's three declarations are all here: a
route and a table binding are constants holding a map, and a record type is
its own declaration.

The page runs a derivation. tests/wasm_engine.rs hands the route-table case's
convention and posts module to the compiler in the tab and compiles the
entry, which derives on the interpreter there, and the program prints the
native golden's output. It was watched red with derivations switched off:
"unknown name `routes`".

Chapter 11 of the book gains a section on `derive`, with the route table as
an executed sample, and the book's highlighter and the editor grammar paint
`derive` as a keyword.

