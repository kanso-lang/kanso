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


## 2026-10-20 — a carried list keeps its buffer outside the arena

#2014 left the run program holding one list buffer of 8,192 slots at exit,
the bag phase's: run_perm_live_bytes 15,825 -> 146,913 (#2015). The buffer is
registered with beat 0, whose rewind would free it, and that rewind reaches
the registration. By then, though, the list's items field points at an arena
copy of exactly 4,300 slots. A watchpoint on the field under gdb caught the
write: the pop of the bag's own beat deep-copies its carried record, and
`k_repair_interior` copied the list's items into the arena because they were
not under the beat's mark. A malloc'd buffer never is. The copy dropped the
only pointer to the buffer, and the registration found an arena buffer in the
field and passed over it. The string and bytes arms of the same function
already leave malloc'd storage where it is. The list arm now does the same
and walks only the slots.

`k_interior_survives` has the same blind spot: it reports a list with
malloc'd items as not surviving, which is what sends the list to the repair.
Changing it as well moved no fixture and no benchmark counter, so it stays as
it is.

The mem fixture `a_carried_list_frees_its_buffer` fills a bag record inside
a loop's beat four times. Without the fix it reads perm_live_bytes 8,208 and
perm_peak_bytes 16,416, one lap's buffer kept to exit and held beside the
next; with it 0 and 8,208. `a_list_held_in_a_record_grows_where_it_stands` no
longer copies its 20,000-element list: evac_bytes 384,848 -> 560 and
alloc_bytes 1,642,720 -> 1,258,448, with perm_live_bytes 655,552 -> 655,408;
with nothing copied there is no outgrown buffer to reuse or shelve,
so a_list_held_in_a_record_grows_where_it_stands_buf_reuse reads 1 -> 0 and
a_list_held_in_a_record_grows_where_it_stands_sh_buf 624 -> 704.
On the run program run_perm_live_bytes 146,913 -> 15,825, run_bytes_freed
9,009 -> 9,010, run_evac_bytes 9,340,096 -> 9,271,280 and run_alloc_bytes
289,288,212 -> 289,219,396. Why #2014's unescape changed whether that pop
copies is not traced; the copy was wrong whichever change exposed it.

The test costs a list the repair is about to copy two loads and a branch.
Asked before the survival test it ran on every list the repair visited, and
deepbench, which repairs a captured list of lists on every bind, read 192,014
instructions more; asked after it, only a list that does not survive pays.
CI's rows: work_runbench 1,137,672,504 -> 1,137,617,275 and
work_deepbench 406,723,866 -> 406,819,866, with work_pendbench 194,088,527 ->
194,089,023, work_widebench 28,740,070 -> 28,740,098, work_oneshot 12,776,869
-> 12,776,876, work_encodebench 2,464,678,221 -> 2,464,678,011 and
work_livebench 1,370,942,122 -> 1,370,942,108. Every program's machine code
grows 16 bytes (runbench text=741,081 -> 741,097), and the corpus's text
5,283,518 -> 5,283,966.
codegen_instructions_release 453,685,379 -> 453,686,073 counts the larger
runtime object; codegen_instructions_dev 116,000,233 -> 116,000,561 is that
and the link directory's order, which #2016 fixes.

A rewrite of the unescape loop was measured and dropped on the way. Reading
a string's last few bytes as a masked word, and looking escapes up in a table
instead of a switch, saved the run program 394,787 instructions (0.035%).
About 65 of the 217 instructions an escaped string costs are the call's fixed
work, and with a backslash every four or five bytes the loop does about the
same work per run whatever its width.

## 2026-10-20 — a plain JSON key goes straight to its colon

An object key used to go through `parse_string`: `builtin_plain_string`, a
dispatch on what it answered, a `parsed` record holding the position after the
key and the key itself, and then `obj_key` taking that record apart to look
for the colon. A key almost never has an escape in it, so `obj_key_start` now
calls `builtin_plain_string` itself, and `key_at`'s string arm works out the
position after the closing quote and goes straight to `obj_key_end`. Anything
else goes through `plain_at` and `obj_key` as before, so an escaped key, a
`\u` escape, an unterminated key and bad utf-8 are read and reported by the
code that read them before.

The same shape for a string value after a colon read the run program 14.8M
instructions higher than keys alone, because the extra byte dispatch in front
of every value cost the numbers more than it saved the strings, and was
dropped.

Over 100,000 fuzzed documents decoded byte for byte alike on native, the
interpreter and #2014's build, and the harness went red when the key's end
was put one byte early.

CI's rows. The work vein: work_runbench 1,137,672,504 -> 1,104,562,811
(-2.91%), work_jsonbench 812,108,477 -> 763,196,627 (-6.02%), work_oneshot
12,776,869 -> 12,450,454 and work_livebench 1,370,942,122 -> 1,370,617,065.
The interpreter reads its corpus's keys the same way: interp_instructions
484,702,222 -> 463,217,423 and interp_allocs 539,460 -> 502,624, with
interp_peak_bytes 614,115 -> 617,367. The library grows by the two arms of
`key_at`, and every program that imports std/json compiles them: compile_instructions
24,089,825 -> 24,321,293, entry_instructions 75,139,973 -> 75,148,170,
library_instructions 74,835,835 -> 74,843,912, compile_allocs 13,382 ->
13,465, compile_peak_bytes 572,289 -> 575,715 and front_end_visits 6,938 ->
7,063. The decoder's emitted code reads
emitted_defines 134 -> 136, emitted_calls 1,249 -> 1,282, emitted_branches
1,336 -> 1,374 and emitted_lines 12,690 -> 13,010, and the other programs
that decode emitted_other_defines 2,148 -> 2,154, emitted_other_calls 18,298
-> 18,397, emitted_other_branches 17,145 -> 17,259 and emitted_other_lines
156,673 -> 157,633. Machine code: text 5,283,518 -> 5,286,574, 880 bytes on
each program that decodes JSON and 416 on runbench. In the tab the run falls
and the compile rises with the library: browser_run_instructions 35,361,799
-> 34,316,072, browser_compile_instructions 408,301,693 -> 413,585,272 and
browser_compile_peak_bytes 749,207 -> 757,553.

On one branch with #2014 and #2018, and on main with #2016's fixed link order,
CI reads work_runbench 1,104,507,218, work_jsonbench 763,196,627,
work_oneshot 12,450,839, work_livebench 1,370,616,792 and work_encodebench
2,464,678,011; the corpus's machine code is text 5,287,022, the two runtime
changes' 32 bytes a program on top of the key's; and
codegen_instructions_dev reads 116,001,189 under the fixed order, the three
changes' cost over main's 115,822,620.

## 2026-10-20 — the ratchet in four files

`scripts/ratchet/ratchet.kso` was 6,316 lines, the largest source file in the
tree (#2021). It is now a directory module, `scripts/ratchet/ratchet/`: the
types, helpers and runner in `ratchet.kso`, and the three chains of rows,
`rows_a`, `rows_b` and `rows_c`, in files of their own. Every line is where it
was, in the same order, with one import of std/list added to each new file.
The tests that read the rows now read every file of the module through
`tests/support/ratchet.rs`, and the one that stages the ratchet copies the
whole directory. `every_ratchet_row_is_run` went red when `rows_c` stopped
joining its last row, so it reads the new files.

## 2026-10-20 — an interpolated int counts its digits once

`"{i}"` on an int goes through `k_b_render_value`, which sized the string
with `ryu_declen` and then called `k_itoa`, which ran the same ladder of
comparisons again before writing the digits. The run program's pend phase
renders 200,000 ints this way. The digit writer is now `k_digits_into`,
which takes the end its caller worked out, and `k_itoa` is a short wrapper
over it. The other kinds of value move to an out-of-line function, so the
int path no longer saves six registers for paths it never takes. Here the
run program read 1,104,506,031 -> 1,100,103,349 instructions (-0.40%), and
`k_b_render_value` 18,309,116 -> 13,884,015. CI's rows: work_runbench
1,104,507,218 -> 1,100,104,536 (-0.40%) and work_pendbench 194,089,023 ->
176,534,752 (-9.04%), the benchmark the run program's pend phase is drawn
from; the rest move by layout alone, work_oneshot 12,450,839 -> 12,451,007,
work_livebench 1,370,616,792 -> 1,370,617,002, work_encodebench
2,464,678,011 -> 2,464,677,731, work_basket 31,162,888 -> 31,162,668,
work_widebench 28,740,098 -> 28,740,070 and work_deepbench 406,819,866 ->
406,819,829. Machine code shrinks: text 5,287,022 -> 5,281,870.
codegen_instructions_dev 116,001,189 -> 116,002,877 and
codegen_instructions_release 453,686,073 -> 453,797,745 count the runtime
object's new shape. In the tab browser_run_instructions reads 34,316,072 ->
34,136,382.

The micro golden `an_interpolated_int_renders_at_every_width` renders each
width where the digit count changes, signs, and both ends of the word, and
reads `+1` where it wants `-1` when the sign is written wrong.

Replacing the `floor()` in the float renderer's integral test with a round
trip through int64 was measured and dropped: glibc picks its SSE4.1 `floor`
at load time, which costs about three instructions, and the round trip made
`k_b_append_rendered` 755,725 instructions dearer.

Writing an int straight into the builder in `k_b_append_rendered`, with the
float path moved out of line the same way, was measured and dropped too: it
read the run program 1,100,103,349 -> 1,104,459,315 and encodebench
2,464,678,011 -> 2,484,039,482, because every float paid the extra call
level and floats are half of what the encoder renders.

## 2026-10-21 — a carried record returns in three words

A two-field record whose first field is an int, like the decoder's
`parsed`, goes back to its caller in registers. It used to take two words:
the int shifted above the value's tag in the first, the value's payload in
the second. Every pack checked that the int survived the shift, and every
unpack branched on a marker for the record built on the heap when it did
not. On the run program that was 725,703 checked packs and 1,791,207
unpacks (#2024). The record now takes three words. Both tags share the
first, the value's payload is the second and the int's word is the third,
so every int fits, a bignum crosses as its pointer, and nothing spills.
`k_parsed_spill` and the two readers of a spilled record are gone. A
failure still travels in the first two words, which read as a `%KValue`.

The first build read the run program 1,100,103,349 -> 1,093,152,335.
`obj_key_end` grew by 3,136,716: with the int's tag read out of a word,
LLVM no longer knew it was an int's, and tested it for a thunk on every
record. An `llvm.assume` that the tag is K_INT or K_BIG gives that back.
Here the run program read 1,100,103,349 -> 1,085,159,551 (-1.36%), and
every decoder function in the profile fell.

CI's rows: work_runbench 1,100,104,536 -> 1,085,160,738 (-1.36%),
work_jsonbench 763,196,627 -> 749,951,027 (-1.74%), work_oneshot
12,451,007 -> 12,362,488, work_widebench 28,740,070 -> 28,580,079,
work_livebench 1,370,617,002 -> 1,370,528,510 and work_encodebench
2,464,677,731 -> 2,464,443,931. Machine code shrinks in every benchmark:
text runbench 741,161 -> 737,929 and jsonbench 382,825 -> 379,049, the
rest by 240 to 3,840 bytes. The decoder's emitted code reads calls 1,282 ->
1,261, branches 1,374 -> 1,332 and lines 13,010 -> 12,909, and the other
programs that decode JSON fall the same way. codegen_instructions_dev
116,002,877 -> 115,990,552, codegen_instructions_release 453,797,745 ->
453,689,156 and emit_instructions 6,233,826 -> 6,232,336. In the tab,
browser_compile_instructions 413,585,272 -> 412,332,402 and
browser_compile_peak_bytes 757,553 -> 752,985 fall, and
browser_run_instructions 34,136,382 -> 34,370,238 rises by 0.69%. The
tab runs the emitter's IR through its own translator, which drops the
assume's call but computes the test that feeds it, and returns a struct
through memory, now three words of it. How the rise divides between those
has not been measured. The welfare sum rose, and the floor moves 91.78 ->
91.84.

The x86 rewrite to `preserve_nonecc` flattens a struct parameter into
words, and it assumed every struct was two. A `%parsed` parameter lost its
third word, the decoder's tail calls stopped matching, and the run program
ran out of stack. `flat_words` now gives each type's width, and the other
counts read from it.

A constant is built once into a cell that holds a `%KValue`, and its
symbol returns what the cell holds. A constant whose body built a carried
record still had a builder returning `%parsed`, and callers that asked for
one: the cell kept the two words as if they were a value, and it read back
because the shapes matched. Three words do not, and
`a_constant_naming_a_record_constant` printed 14, the thunk tag, where it
prints 1. A constant now always returns a `%KValue`, which costs nothing,
since the cell boxed it anyway, and the sample leaves `ir_verifier`'s list
of carried samples.

The micro golden `an_int_of_any_width_crosses_in_its_own_word` sends ints
either side of the old 56-bit line, both ends of the word, a bignum and a
word that overflows into one through four groups that return the record by
value. It joins the samples `ir_verifier` requires to stay carried. The
ratchet's two mutations for the old shape, an unsigned shift back and a
wide int packed anyway, went with the code they broke. Two new ones drop
the bignum tag on the way in and on the way back, and the new golden reads
a pointer as a number under each.

Passing the key's closing `":` down into `escape_onto`, so that each arm
of its test could append its own tail, was measured and dropped. The run
program read 1,103,713,022 against 1,085,159,551. The tail arrived as a
string value, and the emitter's one-word store for a short literal no
longer applied. Bound to a name instead of written in the arms, the plain
arm read `acc` twice, the append stopped writing in place, and the run
program read 1,365,286,224.

## 2026-10-21 — a position read beside its byte is in range, and a length

The decoder hands a byte on together with the position it was read at:
`array_delim cs cs[p + 1] (p + 1) acc`. In an arm that takes the byte as a
byte, either through a literal or after an arm that took `none`, the read
succeeded, so the position indexes `cs`. Adding one to it cannot leave the
machine word, but each `p + 1` carried an overflow test and a way to the
bignum body. `src/read_at.rs` finds the groups where every call passes a
collection, a read out of it and the read's position, the collection and
the index written alike in both places. A partial, a piped call or a bare
mention of the name refuses the group. In each arm that took the byte as a
byte, the emitter tells LLVM the position is an int whose word lies between
-2^62 and 2^62, and LLVM drops the tests. A read or a position that may
arrive deferred is left alone, because a thunk passes the `none` arm's tag
test and may still force to `none`.

Six decoder groups qualify: `array_delim`, `array_open`, `obj_delim`,
`obj_key_start`, `obj_key_end` and `obj_open`. On the run program
15,265,858 `jo`/`jno` executed before and 8,754,624 after, and here it read
1,085,159,551 -> 1,073,408,863 (-1.08%). LLVM inlined differently with the
tests gone: `parse_value` fell 42.3M and `array_delim` rose 20.8M, and
`parse_number` left `parse_value` as a call of its own.

`assume_length` said a length is never negative. It now says a length is
below 2^62, and the inline `length` load uses it, so a guard like
`return acc if i < 1 or length xs < i` lets LLVM step `i + 1` without a
test. Here the run program read 1,073,408,863 -> 1,068,720,545 more, with
`sha256/digested` and json's encode loops the gainers.

CI's rows: work_runbench 1,085,160,738 -> 1,068,721,732 (-1.51%),
work_jsonbench 749,951,027 -> 731,748,677 (-2.43%), work_digestbench
5,992,543 -> 5,241,183 (-12.5%), work_livebench 1,370,528,510 ->
1,356,883,942, work_encodebench 2,464,443,931 -> 2,456,797,406,
work_oneshot 12,362,488 -> 12,207,627, work_widebench 28,580,079 ->
28,571,443 and work_basket 31,162,668 -> 31,152,439. work_scanbench rises
274,236 -> 274,277, by 41. Machine code shrinks in every benchmark: text
runbench 737,929 -> 715,289 and jsonbench 379,049 -> 364,137, the rest by
256 to 24,576 bytes. codegen_instructions_release 453,689,156 ->
422,291,078 (-6.9%) and codegen_instructions_dev 115,990,552 ->
115,922,261 fall with the code LLVM no longer keeps. emit_instructions
6,232,336 -> 6,283,165 rises by the analysis and the assumes it writes.
compile_instructions 24,321,293 -> 24,286,340, entry_instructions
75,148,170 -> 75,053,355, library_instructions 74,843,912 -> 74,749,069 and
interp_instructions 463,217,423 -> 463,213,463. In the tab,
browser_run_instructions 34,370,238 -> 34,153,178 and
browser_compile_peak_bytes 752,985 -> 735,031 fall, and
browser_compile_instructions 412,332,402 -> 413,098,267 rises by 765,865.
The assumes are calls in the emitted IR, so the counts that read them rise:
emitted_calls 1,261 -> 1,394 and emitted_lines 12,909 -> 13,078 for the
decoder, emitted_other_calls 18,298 -> 19,150 and emitted_other_lines
157,080 -> 158,441 for the other programs, and the module compile's
module_calls 109 -> 111 and module_lines 1,131 -> 1,135. Emitted branches
fall in every program that decodes JSON. The welfare sum rose, and the
floor moves 91.84 -> 91.92.

The micro golden `a_position_read_beside_its_byte_stays_exact` runs one
group linked at every call, from positions at both ends of the word, past
it and below zero, and one group whose second call passes a literal byte
beside the largest word. With the call sites' intersection taken out, that
second group is linked anyway and a release build's `p + 1` wraps to
-9223372036854775808 where the interpreter prints 9223372036854775808. The
dev tier never acts on the assume, so the ratchet's row reads the
release-built corpus. The ratchet carries that as
`a_read_position_assumed_past_a_literal`, and `a_read_position_left_unbounded`
drops every link, which the emitted goldens read.

A dead-value pass in `src/ir_wasm.rs`, which drops pure instructions nothing
reads and counts an `llvm.assume` as reading nothing, was measured and
dropped. It took browser_run_instructions 34,370,238 -> 33,477,523 (-2.60%)
against browser_compile_instructions 412,332,402 -> 414,726,069 (+0.58%),
and welfare fell below the floor: the browser run term is saturated, with a
baseline of 2,331,914,518, so the compile cost outweighs the run's gain.

TBAA tags on the IR append helpers, for the header, the buffer's frontier
word, its bytes and the `k_site` stores, were measured and dropped: the run
program's binary came out byte-identical. The three appends `encode_map`
makes for each pair cannot share their header loads because each append's
slow path may hand back a different builder, so the next append's header
pointer is a merge of the two paths. Aliasing was not what kept them apart.

## 2026-10-21 — a JSON key goes out in one claim

lib/json wrote an object's key as three appends around a call: the opening
quote after the brace or comma, `escape_onto` for the key, and the closing
quote and colon as one word. Each append made its own ownership and room
tests on the builder. `builtin_append_key acc k` now writes `"k":` in one
call. The decoder shares every string of four to seven bytes as a token, and
each token records whether its bytes need escaping, so for a clean token the
emitter's inline path claims the builder once and writes three stores: the
quote, the token's whole eight-byte word, and `":` over the bytes past the
key. Any other key, and any build that counts, goes to the C, which writes
the same bytes the long way. The interpreter writes the same bytes.

The inline path finds a token's clean flag from where its bytes sit in the
token store, dividing by the size of a token. A token is a `KStr` and eight
bytes, which is 24 bytes natively and 20 on wasm32, where a pointer is four.
The first version used 24 everywhere. Two `WASM32_LINES` rows now give the
tab its own bound and divisor. `retarget_wasm32` searched only
`getelementptr` lines for rows to rewrite, so the clang-built wasm32 harness
kept the native numbers while the playground's translator, which matches
every line, took the new ones; it now searches for each row by its own text.
The micro golden `a_dirty_key_beside_a_clean_one_is_escaped` places `ke"y`
in the slot whose 24-byte reading on wasm32 is `ammo`'s. Before the rows,
the wasm32 build printed the quote unescaped.

Measured here, the run program read 1,068,720,545 -> 1,049,790,947
(-1.77%) with the same output, all of it in `encode_map`; the C path ran
82,800 instructions. The interpreted corpus read 463,213,463 -> 402,112,576
instructions and 502,624 -> 423,390 allocations, because the interpreter's
builtin writes into the builder where `escape_onto` and three appends ran as
kanso.

CI's rows: work_runbench 1,068,721,732 -> 1,049,792,134 (-1.77%),
work_livebench 1,356,883,942 -> 1,272,755,954 (-6.2%), work_oneshot
12,207,627 -> 11,997,046 (-1.7%) and work_encodebench 2,456,797,406 ->
2,456,796,902; jsonbench and widebench carry their own copy of lib/json and
hold. interp_instructions 463,213,463 -> 402,108,616 (-13.2%),
interp_allocs 502,624 -> 423,390 and interp_peak_bytes 617,367 -> 616,817.
The C path adds 3,264 bytes of text to every program (text runbench 715,289
-> 717,129, jsonbench 364,137 -> 367,401), and codegen_instructions_dev
115,922,261 -> 115,971,597 and emit_instructions 6,283,165 -> 6,307,623 rise
with the helper. codegen_instructions_release 422,291,078 -> 422,279,602,
compile_instructions 24,286,340 -> 24,227,124, entry_instructions
75,053,355 -> 75,007,797, library_instructions 74,749,069 -> 74,686,591,
compile_allocs 13,465 -> 13,434, compile_peak_bytes 575,715 -> 575,051 and
front_end_visits 7,063 -> 7,047 fall. The
counters: perm_allocs falls by one in run (101 -> 100), live (8 -> 7) and
oneshot (9 -> 8), the closing pair's literal, and append_fast rises in the
same three (run 5,735,070 -> 5,737,050, live 25,489,200 -> 25,498,000,
oneshot 63,723 -> 63,745). emitted_lines 13,078 -> 13,089 for the decoder and
+11 in every other program, +32 in runbench (56,807 -> 56,839), oneshot and
livebench, where emitted calls fall by three (runbench 7,380 -> 7,377) and
emitted branches rise by three (6,434 -> 6,437); in all, emitted_other_lines
158,441 -> 158,647 and emitted_other_branches 16,954 -> 16,963. text sums to
5,124,302 -> 5,165,726. The compile corpus's lines 1,412 -> 1,456 and the
module compile's module_lines 1,135 -> 1,146 carry the same comment. In the tab,
browser_run_instructions 34,153,178 -> 33,548,279 (-1.8%) and
browser_compile_instructions 413,098,267 -> 412,842,483 fall, and
browser_compile_peak_bytes rises 735,031 -> 738,682. The welfare sum rose,
and the floor moves 91.92 -> 92.03.

The micro golden `a_key_goes_out_quoted_with_its_colon` decodes and encodes
keys at the edges twice, so the second pass reads shared tokens: empty, one
byte, seven and eight bytes, a two-byte character, an escaped `\u00e9`, and
tokens with a quote, a backslash or a tab. With the inline path's length
short by one, the clean keys lost their colons; with the C's colon replaced
by a quote, the other keys did. Two ratchet rows:
`a_key_claimed_one_byte_at_a_time` sends every key to the C and turns the
emitted gate red, and
`a_token_measured_at_its_native_size_on_wasm32` gives the tab the native
numbers back and turns the translator's micro corpus red. The two byte-pair
rows went blind: the encoder no longer writes two byte appends in a row,
since the brace or comma before a key is one append now. The micro fixture
`two_bytes_appended_at_once` appends a backslash and a letter itself, and
`a_byte_pair_appended_one_at_a_time` is gated by a perf_ratchet spec,
`two_byte_appends_are_one_call`, that reads the merged call in the emitted
code; each mutation turns its witness red. The room-test spec
counts ten frontier branches now. The mem rows move because a clean key no
longer passes through `escape_onto`: `find2_calls` falls to 0 in three
programs, and `perm_allocs` falls by one, the closing pair's literal.

Measured and dropped on the way, recorded so they stay declined:

- A fused `builtin_append_key acc lead k` that also wrote the brace or comma,
  as a C entry. The run program read 1,068,720,545 -> 1,181,687,667: the C
  cost 135 instructions a call against the inline path's check and copy,
  and `k_region_pop` grew 32.4M because `encode_map` then handed its value
  to `encode_onto` directly.
- Passing the key's tail into `escape_onto` (+18.5M) and a copy of the key
  arms (+19M).

Open: `encode_map` still copies a map's keys and values into two columns per
map, 25M on the run program with the region around it.

## 2026-10-21 — a map is written from its entries in place

`encode_map` took a map's two columns, `keys m` and `values m`, which
`k_b_columns` built as two lists in one allocation per map, and indexed both.
Three library builtins now read a map where it stands: `entry_count m`, `key_at
m i` and `value_at m i`. A map's sorted view is `[k v k v ...]`, so where the
view is built the count is a field and each read is a bounds test and one load,
written inline; a missing or marked view goes to the C, which builds the view
first. The encoder takes the map and its count and reads each pair by position,
and no list is made.

The count stays an argument at the call in `encode_onto`, `encode_map acc m
(builtin_entry_count m)`, where the columns were. The beat analysis treats
`entry_count` as allocating, since it may build the view, so the region around a
map's encoding stays one per map. Two other shapes were measured on the way.
With the count bound inside `encode_map`, the pair loop became a beat loop and
`k_beat_pop` cost 17.9M more. With `entry_count` treated as pure, the regions
went (-10.7M) but the output builder grew by an arena copy at each doubling
instead of `realloc` (+24.7M memcpy): inside the region its header sits below
the innermost mark and stays in the malloc regime, and without it the header is
above the encode loop's mark.

A map past eight entries is a B-tree in the interpreter, where reaching the i-th
entry walks i of them. The interpreter keeps the entries in order for the last
such map `key_at` or `value_at` read, holding the map weakly, so the count `put`
asks before writing in place is unchanged, and a `put` that writes in place at a
site linearity proved empties the cache first. No fixture reaches that write
after a read: linearity refuses the `put` that follows a read of the same map,
natively and in the interpreter. The native side-view test is in the same
position: a side forms only when an in-place `put` lands in the middle of a
built view of 64 entries or more, and three loop shapes tried here did not write
in place.

Measured here, on top of the key builtin, the run program read 1,049,790,947 ->
1,039,464,472 (-0.98%) with the same output: `k_b_columns` (25.0M) is gone and
`encode_map` rose 14.3M with the reads.

CI's rows: work_runbench 1,049,792,134 -> 1,039,465,659 (-0.98%), work_livebench
1,272,755,954 -> 1,219,792,784 (-4.2%) and work_oneshot 11,997,046 -> 11,902,783
(-0.79%). work_encodebench rises 2,456,796,902 -> 2,456,797,749, by 847, with
the runtime's layout; it carries its own copy of lib/json. The allocation
counters fall where objects are encoded: run allocs 1,696,484 -> 1,447,994,
alloc_bytes 289,219,396 -> 249,236,356 and sh_buf 119,198,704 -> 87,167,344;
live allocs 1,118,314 -> 13,914, alloc_bytes 413,053,904 -> 235,351,504 and
sh_buf 143,161,328 -> 799,728; oneshot allocs 10,292 -> 7,531, alloc_bytes
2,234,774 -> 1,790,518 and sh_buf 1,155,632 -> 799,728. interp_instructions
402,108,616 -> 401,548,849 and interp_allocs 423,390 -> 420,781 fall, and
interp_peak_bytes rises 616,817 -> 616,944, the cached entries of a large map.
The three inline paths add 320 bytes of text to every program and about 3,400 to
the three that encode objects (text runbench 717,129 -> 720,681), and
codegen_instructions_dev 115,971,597 -> 116,025,482,
codegen_instructions_release 422,279,602 -> 422,550,641, emit_instructions
6,307,623 -> 6,341,758, entry_instructions 75,007,797 -> 75,050,737 and
library_instructions 74,686,591 -> 74,724,744 rise with them.
compile_instructions 24,227,124 -> 24,218,897, compile_allocs 13,434 -> 13,427
and front_end_visits 7,047 -> 7,010 fall, and compile_peak_bytes rises 575,051
-> 575,175. Emitted code: the decoder's emitted_lines 13,089 -> 13,098;
runbench, oneshot and livebench gain 5 defines and lose 27 branches (runbench
56,839 -> 56,741 lines, 6,437 -> 6,410 branches); the compile corpus's lines
1,456 -> 1,492 and module_lines 1,146 -> 1,155; in all, emitted_other_defines
2,154 -> 2,169 and text 5,165,726 -> 5,179,630. In the tab,
browser_run_instructions 33,548,279 -> 32,722,014 (-2.5%) and
browser_compile_peak_bytes 738,682 -> 735,564 fall, and
browser_compile_instructions rises 412,842,483 -> 412,895,911. The welfare sum
rose, and the floor moves 92.03 -> 92.07.

The micro golden `a_map_is_encoded_from_its_entries_in_place` covers an empty
map, two maps past eight entries read one after the other, and a small and a
large map read, extended by a `put` and read again; with the interpreter's cache
keyed by nothing, the second large map printed the first's entries. The mem
fixture that pinned the fused columns through the encoder is renamed
`a_written_map_allocates_no_columns` (3,022 -> 2,022 allocations), and
`a_maps_two_columns_are_one_allocation` now calls `keys` and `values` itself, so
the fusion keeps a witness; `the_two_columns_asked_apart` still turns the mem
vein red. Its rows are re-based, since the name now holds a different program:
a_maps_two_columns_are_one_allocation_allocs 3,022 -> 4,005, a thousand pairs of
lists and the maps they come from; and
a_maps_two_columns_are_one_allocation_append_fast 12,007 -> 0,
a_maps_two_columns_are_one_allocation_append_rendered 2,000 -> 0,
a_maps_two_columns_are_one_allocation_push_mut_fast 995 -> 0,
a_maps_two_columns_are_one_allocation_bytes_freed 8 -> 0 and
a_maps_two_columns_are_one_allocation_utf8_zerocopy 1 -> 0, since it encodes
nothing. The book's chapter 08 panel showed the map going in as its two columns,
and book_quotes refused it once the encoder stopped naming `keys` and `values`;
it now shows the map going in whole with its count. The ratchet row
`a_map_entry_read_through_the_c` sends `key_at` to the C and turns the emitted
gate red.

## 2026-10-22 — a JSON string value goes out in one claim

The encoder wrote a string value as an opening quote, `escape_onto` and a
closing quote. `escape_onto` scanned the string for a byte that needs escaping
and either appended it as it stands or handed it to `append_escaped`, which
scanned it again as it escaped it. On the run program's document 1,773 of every
2,114 string values need escaping. They average 16 bytes with 2.6 escapes each,
so most values paid for both scans. `builtin_append_string acc s` now writes the
quotes and the escaped bytes in one call. Where the builder is at its frontier
with room for the worst case, the C writes the quote, escapes straight into the
builder and writes the closing quote. A value the decoder shared as a clean
token takes the same inline path as a key, with a closing quote where a key has
`":`. The interpreter's arm writes the same bytes.

Measured here, on top of the map read, the run program read 1,039,464,472 ->
1,028,684,506 (-1.04%) with the same output, and the interpreted corpus read
401,548,849 -> 384,232,761 instructions and 420,781 -> 400,588 allocations.

CI's rows: work_runbench 1,039,465,659 -> 1,028,685,693 (-1.04%), work_livebench
1,219,792,784 -> 1,172,094,931 (-3.9%) and work_oneshot 11,902,783 -> 11,785,186
(-0.99%); work_encodebench 2,456,797,749 -> 2,456,797,637. The counters:
find2_calls falls in run (1,667,049 -> 1,488,669), live (807,837 -> 15,037) and
oneshot (17,019 -> 15,037), since a value no longer passes through escape_onto's
scan, and append_fast falls in the same three, the quote appends no longer made
one at a time: run_append_fast 5,737,050 -> 5,328,270, live_append_fast
25,498,000 -> 23,681,200 and oneshot_append_fast 63,745 -> 59,203.
interp_instructions 401,548,849 -> 384,232,761, interp_allocs 420,781 -> 400,588
and interp_peak_bytes 616,944 -> 608,798. The library is smaller to compile:
compile_instructions 24,218,897 -> 23,713,618, entry_instructions 75,050,737 ->
74,424,236, library_instructions 74,724,744 -> 74,085,135, compile_allocs 13,427
-> 13,179, front_end_visits 7,010 -> 6,914 and compile_peak_bytes 575,175 ->
565,175, and codegen_instructions_release 422,550,641 -> 422,357,890. The
runtime's C entry adds about 5,000 bytes of text to every program (text
jsonbench 367,721 -> 372,777, runbench 720,681 -> 721,209), and
codegen_instructions_dev 116,025,482 -> 116,064,263 and emit_instructions
6,341,758 -> 6,364,404 rise with the path. Emitted code: the decoder's
emitted_lines 13,098 -> 13,102; runbench, oneshot and livebench lose
escape_onto's steps (runbench defines 782 -> 778, calls 7,376 -> 7,357, lines
56,741 -> 56,666) and gain 5 branches (6,410 -> 6,415); the compile corpus's
lines 1,492 -> 1,508 and module_lines 1,155 -> 1,159; in all,
emitted_other_branches 16,882 -> 16,897 and text 5,179,630 -> 5,236,430. In the
tab, browser_run_instructions 32,722,014 -> 32,241,456 (-1.5%),
browser_compile_instructions 412,895,911 -> 406,640,369 (-1.5%) and
browser_compile_peak_bytes 735,564 -> 732,776 fall. The welfare sum rose, and
the floor moves 92.07 -> 92.13.

The micro golden `a_string_value_goes_out_between_its_quotes` decodes and
encodes string values at the edges twice: empty, one byte, seven and eight
bytes, `été`, an escaped `\u00e9`, and tokens with a quote, a backslash or a
tab. With the inline path's length short by one, the clean tokens lost their
closing quotes; with the C's closing quote replaced, the values it wrote showed
the replacement. The ratchet row `a_string_value_claimed_three_times` writes
values the old way and turns the emitted gate red. The book's chapter 08 panel
showed the string arm as `_escape_onto` and called its sample the library's
escape functions verbatim, which stopped being true when escaping moved to the
runtime; it now shows the one call and says what the runtime does.

The ratchet row `a_clean_token_scanned_anyway` went blind with this change.
It turns off the clean-token skip in `find2_below`, which served
`escape_onto`, and the run program's values no longer reach `escape_onto`.
encodebench's own copy of lib/json still sends every string through it, so the
row now gates on the encode vein, where the mutation moves `find2_calls`
803,275 -> 4,200,475.

## 2026-10-22 — a map entry's key and value are read with one test

`entry_onto` reads a pair as `pair_onto acc (builtin_key_at m i)
(builtin_value_at m i)`, and each read tested the map's view on its own: the
tags, the view pointer and its mark, and the bounds. Where one call's
arguments read a key and then a value of the same local map at the same local
position, the emitter now calls `k_b_entry_pair_fast` once. It tests the view,
returns the key and stores the value into a slot in the caller's frame, which
the call's next argument loads. A view that is missing or marked goes to the
C for both reads, as each read did alone. The pair is found in both places the
emitter writes a call's arguments, the plain call and the tail call, and
`entry_onto`'s call is a tail call.

Measured here, the run program read 1,028,684,506 -> 1,018,405,282 (-1.0%)
with the same output.

CI's rows: work_runbench 1,028,685,693 -> 1,018,406,469 (-1.0%),
work_livebench 1,172,094,931 -> 1,126,408,749 (-3.9%) and work_oneshot
11,785,186 -> 11,669,620 (-1.0%). The three programs that encode objects lose
one define, two calls and three branches each and are smaller (text runbench
721,209 -> 720,921, oneshot 380,633 -> 380,393, livebench 384,409 ->
384,169; emitted runbench lines 56,666 -> 56,634, oneshot 13,898 -> 13,866,
livebench 14,221 -> 14,189). Every other program carries the helper's two
lines (emitted_lines 13,102 -> 13,104 for the decoder, and +2 in the others),
the compile corpus's lines move by two each (recursion 449 -> 451, dispatch
348 -> 350, guards 305 -> 307, records 406 -> 408; lines 1,508 -> 1,516) and
module_lines 1,159 -> 1,161.
codegen_instructions_dev 116,064,263 -> 116,067,612,
codegen_instructions_release 422,357,890 -> 422,361,029 and
emit_instructions 6,364,404 -> 6,377,706 rise with them. In the tab,
browser_run_instructions 32,241,456 -> 31,991,504 and
browser_compile_instructions 406,640,369 -> 406,222,752 fall, and
browser_compile_peak_bytes rises 732,776 -> 733,266. The welfare sum rose, and
the floor moves 92.13 -> 92.17.

The map golden `a_map_is_encoded_from_its_entries_in_place` is the pair's
witness for its bytes: with the value loaded from the key's slot, every value
printed as its key. The ratchet row `an_entry_read_twice` never sees the pair
and turns the emitted gate red. The row `a_map_entry_read_through_the_c` patched
`key_at_fast` and read BLIND once the pair took every key the encoder reads;
it now sends the pair to the C, which reads the run program 935,900,947 ->
967,870,853, and it gates the instructions vein.

## 2026-10-22 — a builder at the arena's frontier grows where it stands

A builder that outgrew its buffer took a new one and copied, in the arena or
from malloc depending on where its header sat against the innermost mark. When
the header lies above that mark, the buffer is arena storage, and when the
buffer is also the last thing the arena handed out, it now grows by moving
the frontier past it: no new buffer and no copy. A header above the innermost
mark was allocated after it, and a buffer at the frontier after the header, so
the extension dies with the header at the same rewind. The grow also requires
the header to own the buffer's frontier, since a grow is how an append reaches
a buffer another header has written past. The extension's bytes count in
`alloc_bytes`.

That made a second change pay. On 2026-10-21 `entry_count` was left
allocating to the beat analysis, which kept a region around each map the
encoder descends into: inside the region the builder's header sat below the
mark and grew by realloc, and without it the builder grew in the arena by
doubling and copying (+24.7M memcpy). `entry_count` is now pure, as `key_at`
and `value_at` are. The view it may build comes from malloc, and only a sort's
scratch index lands in the arena, where the enclosing loop's rewind takes it.
The region is gone, and the builder now grows in place.

Measured here, the run program read 1,018,405,282 -> 999,948,734 (-1.81%)
with the same output: `k_region_pop`'s 10.7M is gone, and `encode_list` and
`encode_map` fall 3.5M and 3.8M.

CI's rows: work_runbench 1,018,406,469 -> 999,949,921 (-1.81%),
work_livebench 1,126,408,749 -> 1,043,615,776 (-7.3%), work_oneshot
11,669,620 -> 11,443,335 (-1.9%) and work_encodebench 2,456,797,637 ->
2,456,796,655. work_widebench rises 28,571,443 -> 28,619,441 (+0.17%); it
carries its own copy of lib/json and grows its builders in the arena too. The
run program's held_peak_bytes falls 200,978 -> 111,920, and run allocs
1,447,994 -> 1,446,734, alloc_bytes 249,236,356 -> 221,966,536, append_grow
1,260 -> 1,080, append_fast 5,328,270 -> 5,328,540, bytes_malloc 9,140 ->
7,880 and bytes_freed 9,010 -> 7,750. The one-shot and live programs'
builders never leave the arena: held_peak_bytes 200,978 -> 0 in both,
bytes_malloc and bytes_freed 14 -> 0 and 5,600 -> 0, oneshot allocs 7,531 ->
7,517, alloc_bytes 1,790,518 -> 1,487,520, append_grow 14 -> 12, append_fast
59,203 -> 59,206; live allocs 13,914 -> 8,314, alloc_bytes 235,351,504 ->
114,152,304, append_grow 5,600 -> 4,800, append_fast 23,681,200 ->
23,682,400. Without the region runbench, oneshot and livebench make three
fewer calls (runbench calls 7,355 -> 7,352, lines 56,634 -> 56,636; oneshot
calls 1,484 -> 1,481, lines 13,866 -> 13,868; livebench calls 1,523 -> 1,520,
lines 14,189 -> 14,191). The runtime's text grows 336 bytes in every program
(text jsonbench 372,777 -> 373,113) and shrinks where the region went (text
runbench 720,921 -> 719,977, oneshot 380,393 -> 379,641, livebench 384,169 ->
383,177). codegen_instructions_dev 116,067,612 -> 116,068,279 and
codegen_instructions_release 422,361,029 -> 422,367,718 rise, and
emit_instructions 6,377,706 -> 6,377,307 falls. In the tab,
browser_run_instructions 31,991,504 -> 31,623,429,
browser_run_peak_bytes 1,441,792 -> 1,376,256, browser_compile_instructions
406,222,752 -> 406,034,117 and browser_compile_peak_bytes 733,266 -> 732,887
fall. The welfare sum rose, and the floor moves 92.17 -> 92.25.

The frees fall with the mallocs they answered: run_bytes_freed 9,010 -> 7,750,
oneshot_bytes_freed 14 -> 0, live_bytes_freed 5,600 -> 0 and
a_literal_appended_across_a_rewind_bytes_freed 40 -> 0. Over all programs,
emitted_other_lines 158,180 -> 158,186 and text 5,235,662 -> 5,236,670.

The mem corpus moves where builders grow in the arena:
`a_literal_appended_across_a_rewind` allocs 136 -> 96, alloc_bytes 11,920 ->
9,040, bytes_malloc and bytes_freed 40 -> 0, held_peak_bytes 152 -> 0;
`three_storage_regimes_share_one_append` allocs 1,640 -> 1,040, alloc_bytes
186,274 -> 114,274; `append_of_a_slice_boxes_nothing` allocs 14 -> 12,
alloc_bytes 2,396 -> 2,156. The micro golden
`a_builder_grows_where_it_stands_until_something_follows_it` grows a builder
in place, appends a string made after the buffer, and grows again by copying,
300 rounds inside a loop against one built outside any; with the in-place
write one byte early it printed `0 of 300 rounds agree`. Two ratchet rows:
`a_builder_copied_at_the_frontier` turns the mem corpus red, and
`a_maps_count_held_as_allocating` puts the region back and turns the run
counters red.

## 2026-10-22 — a call into a loop keeps its cohort bracket under a cycle

A cohort bracket marks the arena before a call that crosses into another
module with immutable arguments, and its pop gives back what the call left.
The license refused every caller a cycle reaches (`cycle_reached`), since such
a caller can run once per element of something and its calls are usually
small. It now admits one when the callee reaches a cycle that allocates
(`reaches_cycle`: the members of every cycle whose members the beat analysis
finds allocating, and everything that mentions them, followed backward) and
answers no heap value. A call into a loop pays
its push and pop once against the whole loop, and with no heap value to keep
the pop rewinds without copying anything out.

On the run program the index phase builds 1.5 MB of text to answer an int,
and nothing marked the arena around it, so the string lived until the program
ended. `tally` is reached by a cycle, so the license refused the call. Measured
here, arena_peak_bytes 4,718,608 -> 3,670,032 and the run read 999,948,734 ->
999,793,761 with the same output. The peak now sits at two live megabyte
blocks, the file read and its decoded document, and the 1,572,880-byte string
the phase builds; the program needs all three at that moment.

Three other shapes were measured on the way. Seeded from every cycle, the
reach admitted widebench's whitespace skip, a loop that builds nothing, called
once per value from its parser; CI read widebench 28,619,441 -> 34,955,663
(+22%) and encodebench 2,456,796,655 -> 2,461,704,983, 4.0M of it
`k_cohort_pop` with nothing to give back. The ratchet row
`a_loop_that_builds_nothing_bracketed` puts that seed back and turns the
instructions gate red. Without the heap-value condition
one bracket evacuated a heap result through the carry, and `k_deep_copy` and
`k_copy_size` cost 1.7M more. A region of the beat analysis's kind, marked
before the arguments, made `a_long_chain_answers_survival_from_its_index`
evacuate its 100,000 rows on every lap (evac_allocs 25 -> 480,009), because
the rows its argument built landed inside the mark.

CI's rows: work_runbench 999,949,921 -> 999,794,948 (-0.02%), and no other
benchmark's work moves. The run program's run_arena_peak_bytes 4,718,608 ->
3,670,032 and run_cohort_frees 1 -> 4; run_allocs 1,446,734 -> 1,446,735,
run_alloc_bytes 221,966,536 -> 221,966,568, run_evac_allocs 48,953 -> 48,955,
run_evac_bytes 9,271,280 -> 9,271,408, run_survive_slots 4,856 -> 4,854,
run_str_scans 1,023 -> 1,024, run_str_scan_bytes 17,996,094 -> 17,996,158 and
run_sh_rec 3,776,192 -> 3,776,128. The run program makes 24 more calls
(emitted_other_calls 19,064 -> 19,088, emitted_other_lines 158,186 ->
158,210) and its machine code grows 719,977 -> 722,233 bytes (text 5,236,670
-> 5,238,926 over all the binaries). compile_instructions 23,713,618 ->
23,736,301, entry_instructions 74,424,236 -> 74,485,199, library_instructions
74,085,135 -> 74,147,760 and emit_instructions 6,377,307 -> 6,423,478 rise,
and interp_instructions 384,232,761 -> 384,228,496 falls. In the tab,
browser_compile_instructions 406,034,117 -> 401,989,712 falls. The welfare
sum rose, and the floor moves 92.25 -> 92.28.

`tests/cohort.rs` `a_call_into_a_loop_gives_back_what_it_built` runs a new
two-module fixture, `tests/golden/cohort/phases`: each pass of a loop calls
`grow/total` twice, and each call builds a megabyte of text to answer its
length. It pins cohort_frees=6 and arena_peak_bytes=2097168; under the old
license it read 0 and 3145760. The ratchet row
`a_call_into_a_loop_left_unbracketed` takes the license back and turns the run
counters red.

## 2026-10-22 — the JSON helpers store their call's site only where they can fail

A failure names its place through `k_site`, which the emitter stamps before a
call that can stop the program. Since #1975 an inline twin that takes the site
as its last argument stores it only on the branch that reaches the C, and its
caller skips the stamp. Five twins the encoder calls once per pair or item
took none: `append_key`, `append_string`, `entry_count`, `key_at` and
`value_at`. They now take the site and store it in their slow blocks. The run
program executed 5,384,347 `k_site` stores and now 4,366,872, and read
999,793,761 -> 997,774,101 here with the same output.

Passing the string index (`s[i]`, 690,000 calls in the index phase) its site
the same way, through a C entry that stored it only before refusing, was
measured and dropped: -275k, since the extra argument costs what the store
did.

CI's rows: work_runbench 999,794,948 -> 997,775,288 (-0.20%), work_livebench
1,043,615,776 -> 1,033,729,698 (-0.95%) and work_oneshot 11,443,335 ->
11,424,128 (-0.17%). The run program's machine code grows 722,233 -> 722,249
bytes (text 5,238,926 -> 5,238,942 over all the binaries), and
emit_instructions rises 6,423,478 -> 6,426,328. In the tab,
browser_run_instructions falls 31,623,429 -> 31,602,327, and
browser_compile_instructions 401,989,712 -> 402,017,479 and
browser_compile_peak_bytes 732,887 -> 733,103 rise. The welfare sum rose, and
the floor moves 92.276 -> 92.284.

The builtins are internal to the standard library and every call in lib/json
passes the types they take, so no program reaches the branches that store the
site and no output moves. The emitted counts do not move either: each twin
loses a store at its call and gains one in its slow block. The ratchet row
`a_sited_call_stamped_anyway` stamps every sited call as well; the run
program then reads 1,012,744,021, and the instructions gate goes red.

## 2026-10-22 — a short key is found in the token cache without a call

The decoder reads an object's key through `k_b_plain_string_fast`, which finds
the closing quote and, when no backslash came first, calls
`k_b_utf8_slice_raw` for the string. A key of four to seven bytes is almost
always in the token cache, a direct-mapped table of strings keyed by the
token's bytes and length, so the call usually did a bounds test, a hash and
two loads. The helper's IR now does the hash and the compare itself and loads
the cached string when the slot's key matches; anything else, including a key
that went to the slot beside its own, still takes the call. `k_token_key` and
`k_token_cache` lose `static` so the emitted code can read them.

The run program's calls into the slice from the key path fall from 784,179 to
17,501, and it read 997,774,087 -> 977,794,290 here (-2.0%) with the same
output.

Lifting `k_b_utf8_slice_raw` into the hot unit beside `k_b_find2_raw`, so the
LTO link could inline it, was tried first and read 997,020,816, because the
link kept the call. The emitted IR spells the return type `%KValue` and
clang's unit a literal `{ i64, i64 }`, which may be what stopped it; that was
not isolated.

CI's rows: work_runbench 997,775,288 -> 977,795,537 (-2.00%), work_jsonbench
731,748,677 -> 699,198,721 (-4.45%), work_oneshot 11,424,128 -> 11,233,739
(-1.67%) and work_livebench 1,033,729,698 -> 1,033,539,043. The four
programs that decode objects carry the inline path and grow: text jsonbench
373,113 -> 374,057, oneshot 379,641 -> 380,585, livebench 383,177 ->
384,121 and runbench 722,249 -> 723,129, text 5,238,942 -> 5,242,654 over
all the binaries. Every program declares the two tables: emitted_lines
13,104 -> 13,134 and emitted_branches 1,305 -> 1,307 for the decoder,
emitted_other_lines 158,210 -> 158,320 and emitted_other_branches 16,888 ->
16,894, module_lines 1,161 -> 1,163, and each compile sample two lines more
(recursion 451 -> 453, dispatch 350 -> 352, guards 307 -> 309, records 408
-> 410; lines 1,516 -> 1,524 in all). emit_instructions 6,426,328 -> 6,435,278 and
codegen_instructions_release 422,367,718 -> 422,642,173 rise, and
codegen_instructions_dev 116,068,279 -> 116,064,144 falls. In the tab, browser_run_instructions
31,602,327 -> 31,449,729 falls, and browser_compile_instructions 402,017,479
-> 402,486,449 and browser_compile_peak_bytes 733,103 -> 735,005 rise. The
welfare sum rose, and the floor moves 92.28 -> 92.36.

The micro golden `a_short_key_is_found_in_the_cache_in_place` decodes two
objects with the same fifteen keys, the second reading what the first filled.
Two of its keys, `aaaz` and `abae`, hash to one slot, so the second goes to
the slot beside it. Reading the string from the neighbouring slot, or taking
a slot as a hit without comparing its key, answers one of the two for the
other, and the golden goes red under both; the first is the ratchet row
`a_short_key_read_from_the_slot_beside_it`. The row
`a_short_key_left_to_the_call` sends every key to the call again and turns
the instructions gate red.

## 2026-10-22 — a plain number is read a word at a time

`number_span` finds where a JSON number's bytes end by classifying sixteen
bytes a step against the five characters a number holds besides digits. The
run program's 417,483 numbers are four to nine bytes long and all of one
shape: an optional minus, digits, and an optional point with digits after
it. `k_b_number_span_raw` now reads that shape first, finding the end of each
run of digits eight bytes at a time and checking that the byte after the
number is one no number holds. A leading zero, an exponent, a point with no
digit after it, any other byte a number may hold after the run, and any number
within forty bytes of the end of its document go to the sixteen-byte scan,
which starts again from the beginning. That scan moved into its own function,
`k_number_span_scan`, because the registers it wants made every call build a
frame. `k_nondigits8` moved from numbers.c to text.c, which comes first in the
runtime, so both files use one copy.

The float parse had the same shape of cost. `k_to_float_text` answers the
`[-]I.F` shape from two words and parses everything else with a scan that ends
in a strtod fallback, and the fallback's buffer made every call build a frame.
The slice doors now call that scan out of line, and the shaped case is a
leaf. `k_b_to_float`, the door for a string, keeps it inline: a short string
has no room for the shape's two words and always scans, and with the scan out
of line there CI read widebench 28,619,441 -> 28,939,500.

Measured here, the run program read 977,794,290 -> 966,447,210 (-1.16%) with
the same output: the span 977,794,290 -> 968,235,476, `k_b_number_span_raw`
34,243,704 -> 24,644,367, and the float split 968,235,476 -> 966,447,210,
`k_b_to_float_slice_raw` 28,270,242 -> 26,482,302. Moving the slice that an
out-of-range float or integer builds out of line as well read 966,243,735, a
further 203,475, and made the integer door slower by 217,306; it was dropped. The new
path and the old function, both extracted from the source, agreed on
50,000,000 random byte strings and positions. The harness saw a float answered
as an integer and a leading zero let through, each in about 2,000 of a
million cases.

CI's rows: work_runbench 977,795,537 -> 966,449,322 (-1.16%), work_jsonbench
699,198,721 -> 682,006,171 (-2.46%), work_oneshot 11,233,739 -> 11,118,597
(-1.02%), work_livebench 1,033,539,043 -> 1,033,424,160 and
work_encodebench 2,456,796,655 -> 2,456,796,277. Every binary's machine code
shrinks by 1,376 bytes, the float scan no longer inlined into the slice doors
(text 5,242,654 -> 5,223,390 over all of them; runbench 723,129 -> 721,753).
codegen_instructions_release 422,642,173 -> 422,580,983 falls and
codegen_instructions_dev 116,064,144 -> 116,068,622 rises. In the tab, browser_run_instructions 31,449,729 ->
31,370,234 falls. The
welfare sum rose, and the floor moves 92.36 -> 92.41.

The micro golden `a_number_far_from_the_end_is_read_a_word_at_a_time` decodes
documents whose numbers sit ahead of a long string, so they are within reach
of the new path: plain numbers of both kinds beside exponents, a sixteen-digit
run and `-0`, then eight malformed numbers. The ratchet row
`a_point_forgotten_on_the_fast_path` answers a float as an integer there and
turns the golden red; `a_number_scanned_sixteen_bytes_at_a_time_anyway` turns
the path off and turns the instructions gate red. The row
`a_digit_word_that_reads_a_colon_as_ten` patched `k_nondigits8` in
numbers.c and would not apply once it moved; it patches text.c now.

## 2026-10-22 — a bind chain measures its drift from where it began

A bind chain stages a step's continuation (sizes it, copies it through the
carry pair and rewinds) only when the arena has drifted a quarter megabyte
since the chain's last stage; otherwise the continuation stays where it is.
`k_exec` set the carry's `at_arena` to null when it pushed the chain's beat,
because the depth's last stage, if any, was another chain's. That made every
chain's first step size and stage its continuation however little it had
allocated, and the run program pushes a chain 7,091 times: 7,162 stages,
15,288,786 instructions in `k_beat_iter_carry` and 6,099,136 sizing in
`k_copy_size`. The push now records the arena's position and block, so the
drift is measured from where this chain began. A skipped step was already
sound everywhere, since the 2026-09-07 drift gate made the copy-out at a
carried pop walk deep.

Measured here, the run program read 966,447,210 -> 935,900,947 (-3.16%) with
the same output, and `k_beat_iter_carry` left the profile.

CI's rows: work_runbench 966,449,322 -> 935,902,128 (-3.16%) and
work_deepbench 406,819,829 -> 203,571,400 (-49.96%). work_pendbench
176,534,752 -> 176,531,937, work_readbench 4,583,965 -> 4,582,072,
work_jsonbench 682,006,171 -> 682,004,279 and work_livebench 1,033,424,160
-> 1,033,424,099 fall a little, and work_encodebench 2,456,796,277 ->
2,456,796,762, work_oneshot 11,118,597 -> 11,118,893, work_widebench
28,619,441 -> 28,619,457 and work_digestbench 5,241,183 -> 5,241,199 rise
by a few hundred instructions or less. Every binary's machine code grows 16
bytes (text 5,223,390 -> 5,223,614 over all of them; runbench 721,753 -> 721,769).
codegen_instructions_dev 116,068,622 -> 116,068,667 and
codegen_instructions_release 422,580,983 -> 422,581,877 rise. In the tab, browser_run_instructions 31,370,234 ->
31,370,376 rises. The
welfare sum rose, and the floor moves 92.41 -> 92.53.

The counter veins move where chains no longer stage their first step. The
run program's run_evac_allocs 48,955 -> 1,624, run_evac_bytes 9,271,408 ->
7,416,368, run_allocs 1,446,735 -> 1,438,645, run_alloc_bytes 221,966,568 ->
221,297,400, run_beat_iters 92,018 -> 84,938, run_survive_slots 4,854 ->
4,560, run_buf_reuse 134 -> 139 and run_sh_buf 87,167,344 -> 86,585,920; no
peak moves. pend_allocs 805,977 -> 805,967, pend_alloc_bytes 45,563,312 ->
45,539,584, pend_beat_iters 101 -> 99, pend_evac_allocs 2,225 -> 2,210,
pend_evac_bytes 381,648 -> 378,048, pend_survive_slots 78,611 -> 77,616,
pend_buf_reuse 502 -> 505 and pend_sh_buf 15,860,144 -> 15,838,096;
read_beat_iters 201 -> 200, read_evac_allocs 19 -> 16 and read_evac_bytes
656 -> 576; the decoder's beat_iters 151 -> 150, evac_allocs 19 -> 16 and
evac_bytes 688 -> 576.
The mem corpus moves in nineteen fixtures, each where a chain's first step no
longer stages: a_carried_value_written_into_an_older_node_alloc_bytes 2,595,040 -> 2,568,080, a_carried_value_written_into_an_older_node_beat_iters 11 -> 9, a_carried_value_written_into_an_older_node_evac_allocs 1,697 -> 1,692, a_carried_value_written_into_an_older_node_evac_bytes 174,224 -> 168,896, a_carried_value_written_into_an_older_node_survive_slots 407 -> 404, a_carried_value_written_into_an_older_node_carry_dedup 446 -> 621, a_carried_value_written_into_an_older_node_buf_reuse 781 -> 783, a_carried_value_written_into_an_older_node_sh_buf 682,096 -> 657,104; a_loop_invariant_capture_is_copied_every_rewind_beat_iters 502 -> 501, a_loop_invariant_capture_is_copied_every_rewind_evac_allocs 10 -> 4, a_loop_invariant_capture_is_copied_every_rewind_evac_bytes 8,336 -> 144; a_repaired_node_below_the_mark_holds_tenure_allocs 34,047 -> 34,049, a_repaired_node_below_the_mark_holds_tenure_alloc_bytes 2,493,952 -> 2,510,528, a_repaired_node_below_the_mark_holds_tenure_beat_iters 10 -> 9, a_repaired_node_below_the_mark_holds_tenure_evac_allocs 2,469 -> 2,468, a_repaired_node_below_the_mark_holds_tenure_evac_bytes 162,960 -> 171,840, a_repaired_node_below_the_mark_holds_tenure_carry_dedup 46 -> 80, a_repaired_node_below_the_mark_holds_tenure_sh_buf 567,232 -> 579,536; a_slice_bang_chain_builds_no_slice_beat_iters 2 -> 0, a_slice_bang_chain_builds_no_slice_evac_allocs 15 -> 8, a_slice_bang_chain_builds_no_slice_evac_bytes 528 -> 288; an_inner_beat_opens_its_tenure_in_the_block_outside_allocs 180,196 -> 179,380, an_inner_beat_opens_its_tenure_in_the_block_outside_alloc_bytes 14,926,832 -> 14,712,048, an_inner_beat_opens_its_tenure_in_the_block_outside_beat_iters 60 -> 54, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_allocs 48,411 -> 45,168, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_bytes 3,002,288 -> 2,788,720, an_inner_beat_opens_its_tenure_in_the_block_outside_survive_slots 2,004 -> 11,204, an_inner_beat_opens_its_tenure_in_the_block_outside_carry_dedup 209 -> 459, an_inner_beat_opens_its_tenure_in_the_block_outside_buf_reuse 3,895 -> 3,907, an_inner_beat_opens_its_tenure_in_the_block_outside_sh_buf 5,070,416 -> 4,887,632; an_insisting_form_that_settled_builds_no_step_beat_iters 2 -> 0, an_insisting_form_that_settled_builds_no_step_evac_allocs 18 -> 9, an_insisting_form_that_settled_builds_no_step_evac_bytes 560 -> 288; effect_push_shape_allocs 56 -> 52, effect_push_shape_alloc_bytes 2,544 -> 2,320, effect_push_shape_beat_iters 5 -> 0, effect_push_shape_evac_allocs 50 -> 14, effect_push_shape_evac_bytes 1,696 -> 464, effect_push_shape_survive_slots 10 -> 4, effect_push_shape_sh_buf 768 -> 608; stream_write_beat_iters 1,601 -> 1,600, stream_write_evac_allocs 12 -> 6, stream_write_evac_bytes 480 -> 240, stream_write_held_peak_bytes 16,000 -> 16,160; the_same_capture_built_below_the_mark_is_shared_beat_iters 502 -> 501, the_same_capture_built_below_the_mark_is_shared_evac_allocs 10 -> 6, the_same_capture_built_below_the_mark_is_shared_evac_bytes 8,352 -> 8,192, the_same_capture_built_below_the_mark_is_shared_survive_slots 504 -> 4; tie_cycle_beat_iters 1 -> 0, tie_cycle_evac_allocs 15 -> 12, tie_cycle_evac_bytes 560 -> 464; work_ahead_forces_what_follows_a_park_beat_iters 2 -> 0, work_ahead_forces_what_follows_a_park_evac_allocs 11 -> 7, work_ahead_forces_what_follows_a_park_evac_bytes 368 -> 240; work_ahead_leaves_a_chain_never_run_beat_iters 1 -> 0, work_ahead_leaves_a_chain_never_run_evac_allocs 5 -> 4, work_ahead_leaves_a_chain_never_run_evac_bytes 192 -> 144; work_ahead_leaves_a_literal_where_it_is_beat_iters 2 -> 0, work_ahead_leaves_a_literal_where_it_is_evac_allocs 11 -> 7, work_ahead_leaves_a_literal_where_it_is_evac_bytes 368 -> 240; work_ahead_leaves_an_extending_builtin_beat_iters 4,002 -> 4,000, work_ahead_leaves_an_extending_builtin_evac_allocs 13 -> 9, work_ahead_leaves_an_extending_builtin_evac_bytes 448 -> 320; work_ahead_lifts_a_parking_lambda_beat_iters 1,001 -> 1,000, work_ahead_lifts_a_parking_lambda_evac_allocs 12 -> 4, work_ahead_lifts_a_parking_lambda_evac_bytes 384 -> 144; work_ahead_moves_a_binding_into_the_park_beat_iters 2 -> 0, work_ahead_moves_a_binding_into_the_park_evac_allocs 11 -> 7, work_ahead_moves_a_binding_into_the_park_evac_bytes 368 -> 240; work_ahead_pools_a_call_on_a_builtin_read_beat_iters 3,002 -> 3,000, work_ahead_pools_a_call_on_a_builtin_read_evac_allocs 15 -> 11, work_ahead_pools_a_call_on_a_builtin_read_evac_bytes 496 -> 368; work_ahead_pools_a_call_on_a_local_beat_iters 1,002 -> 1,000, work_ahead_pools_a_call_on_a_local_evac_allocs 13 -> 9, work_ahead_pools_a_call_on_a_local_evac_bytes 416 -> 288; work_ahead_reads_a_moved_binding_beat_iters 2 -> 0, work_ahead_reads_a_moved_binding_evac_allocs 11 -> 7, work_ahead_reads_a_moved_binding_evac_bytes 368 -> 240. The only peak among them is stream_write's held peak, 160 bytes higher. The ratchet row `a_chain_staged_on_its_first_step` puts the null back
and turns the run counters red. `tests/a_document_read_from_a_file_keeps_its_loop.rs`
counted one beat per iteration and one for the program; the program's own
chain no longer stages, and its four programs read 200, 800, 200 and 200.

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

The tab's compile of its corpus fell from 227,562,045 to 224,770,876
instructions (-1.2%) in this container; its peak and the run did not move,
and the tab's IR and the native IR of the nineteen benchmarks in both tiers
are byte-identical. The specs that held each scan to a plain search now hold
the merged one to it, with the ending side asked only about symbols without a
`(`, which is every symbol the writer asks about; treating the end of the text
as a line's end turned that spec red on `"@a"`. The ratchet row `one_scan`
scans each piece a second time, and the browser row reads 226,415,714.
