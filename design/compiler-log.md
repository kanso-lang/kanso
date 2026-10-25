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

## 2026-10-24 — a record returned by value keeps its shape across a pipe

#2091's first item, from the mustache port. `half` answers a two-field
record or an err, and escape analysis returns a record of that shape by
value, as the three words of `%parsed`. `half 4 .> shown` lowers to a test
of the value's tag, a call of `shown` on the success path, and a phi joining
the two paths as `%KValue`s. The failure edge named the value as it came,
three words, and clang refused the module: "'%t1' defined with type
'%parsed' but expected '%KValue'". The interpreter printed `4/2`.

The boxing rewrite that turns a by-value record into a `%KValue` wherever a
line names it as one reads `%KValue %t1`, and a phi's operand list names its
values bare, so the rewrite never reached this line. The pipe now boxes the
value for the failure edge before it branches, and boxes the call's answer
when that comes back by value too. The call itself still takes the three
words, since `shown` takes them by value.

`micro/a_record_returned_by_value_is_piped_on` runs the success and the
failure paths. Main stops it in clang; every engine now prints `4/2` and
`failed: zero`.

CI's rows: the emitter writes the box where it wrote nothing, so
emit_instructions goes from 5,606,362 to 5,606,408, and the tab, which
emits the same way, takes browser_compile_instructions from 216,735,094 to
216,735,256. No benchmark builds a pipe of that shape, and nothing at run
time moved.

## 2026-10-24 — a loop counter knows where it starts

The ports' re-sit counted the same complaint in about ten journals: a guard
proves a list read only when it is written `i < 1 or length xs < i` (httpd
F3, make F4, sat F6, xsv F1, regex F3, diff F3). A loop over a list starts
its counter at one and only adds to it, and its author writes the guard on
the far end alone: `return acc if length xs < at`. The checker read that as
a bound above and none below, so `xs[at]` could be a none, and the read had
to be resolved where it could not miss.

Inference now gives a parameter a floor when every value it can hold is in
sight (`counter_floors` in src/infer.rs). The group is not pub, so no caller
outside the program exists. It is never named except as the head of a call
with an arity it declares, so it never travels as a value or a partial.
Every call from outside the group passes an integer literal at that position.
Every call from inside passes the parameter itself plus a literal that is not
negative, and no arm rebinds the parameter. The floor is the least literal,
and it enters each arm as a fact beside the ones guards add, so `in_range`
reads `at >= 1` from it and the guard on the far end finishes the proof.

`micro/a_loop_counter_started_at_one_reads_in_range` is the shape, and main
refuses it with "this can be a none and `double` has no arm for it". Two
error goldens pin where the floor stops: a loop entered at zero
(`errors/a_loop_counter_started_at_zero_can_miss`) and a counter rebound in
the arm (`errors/a_rebound_counter_loses_its_floor`). A loop called from a
lambda, whose arguments come from out of sight, is refused as before.

A proven read carries no none in inference's sets, so inference settles
sooner where std/text reads one: compiling `text/uppercase` alone now takes
223 expression visits, one fewer, and
`tests/an_import_reads_what_the_program_names.rs` says so.

What it does not reach: the about 130 reads #2088's named-none check refuses
in the tree. With the floor under that check the count is still 130. Most of
them read behind a bound passed through a boolean, `applied dir list at acc
(at > length list)` with the read in the `false` arm, where the fact lives in
the caller and no floor on the counter covers it.

CI's rows. Every compile asks the call graph where each counter starts, and
the tab compiles with the same front end: compile_instructions 24,230,258 ->
24,488,495, entry_instructions 71,606,414 -> 72,335,545,
library_instructions 71,254,677 -> 72,011,050, emit_instructions 5,606,408
-> 5,637,678, compile_allocs 13,789 -> 13,815 and
browser_compile_instructions 216,735,256 -> 218,080,833. The interpreter's
corpus asked the allocator for 52 more blocks (interp_allocs 401,539 ->
401,591) and ran 401 fewer instructions (385,500,095 -> 385,499,694). Nothing
native moved. Welfare fell by less than a hundredth, and the floor follows
under the language exception: the ruling asks for index reads to be proven,
and this proves more of them.

## 2026-10-24 — a field read is asked the none question the same way on every route

`kanso check m` passed a module that `kanso test m` refused (#2091, item 3).
inflate's port has the shape: `fn go (req none)` answers a request whose
path is a none, and the arm below it hands `r.path` to `os/read_bytes`. The
test file beside it holds only `test_one = 1 == 1`. ugit's port had the same
split, and toml's had it between `check lib` and `check main.kso`.

The none question in `after_infer.rs` asks whether an argument can be a none.
It answers no for a field read, so a none carried in a field stays the
runtime's sentence. But the module and play routes run the check before
field reads are rewritten into getter calls, and the entry and test routes
run it after. On those two the argument was a call to `Get_path`, whose
return set is every value the field was built from, and the first arm's
claim on the none never reaches that set. The question now answers no for a
getter call too, so all four routes give a field read the same answer.

`tests/golden/test_verb/claims_a_field` is inflate's shape, and
`a_test_file_checks_its_module_as_check_does` in `tests/test_verb.rs` runs
it; main refuses it. `check_gives_a_library_and_its_entry_one_verdict_on_a_field_read`
in `tests/verbs_agree.rs` is toml's, where main's `check main.kso` refused
what `check lib` passed.

Whether a none carried in a field should be refused at check is #2088's
question, for every route at once.

CI's rows. A getter call now answers the none question before the table
lookup it used to make: compile_instructions 24,488,495 -> 24,486,012,
entry_instructions 72,335,545 -> 72,317,212, library_instructions
72,011,050 -> 71,993,114 and browser_compile_instructions 218,080,833 ->
218,016,527. Welfare rose, and the rise is banked.

## 2026-10-24 — a test file's helper is checked like its constants

Under the 2026-10-12 gavel (#1883) a test file sees the pub names of the
module beside it and nothing else. A test constant that names a private
function is refused at check. A helper function in the test file that names
one was not: xsv's port called the module's private `hidden` from a helper,
`kanso check` passed it, and `kanso test` failed that test at run time with
"error[runtime]: unknown name `~hidden`", a spelling the author never wrote.
(#2091, item 2.)

The check does refuse the name. It sends a call it cannot resolve inside a
function to a traveling group, which the program's root fills from every
module that writes the name for a type it owns, or refuses. A library
compile leaves that to whatever imports it, so `compile_library_beside`
dropped the shades and moved on. A test file is compiled on the same path,
and nothing imports it. It now settles the groups where it is compiled, with
`enroll_travelers`, as the entry and module paths do, and the helper's call
is refused at check with the same `unknown name` error the constant gets.

`tests/golden/test_verb/reaches` is the reduction, and
`a_test_helper_reaching_a_private_name_is_refused` in `tests/test_verb.rs`
pins its stderr. On main it prints the run-time error above.

A library compile takes the path it took before, and the test verb has no
row. The rows this moved are counted with the batch it landed in, at the end
of "an import stays in the file that writes it".

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

`qualify` asks each dependency which names its module declares. The first
cut cloned every declared name into a set, 305 allocations; the set now
borrows them. The rows are counted with the batch this landed in, at the end
of "an import stays in the file that writes it".

## 2026-10-25 — a module's own type beats an imported function of its name

From #2090, mal's port. A module declared `type tally` beside
`import "std/list"`, which exports a one-argument `list/tally`. Every
construction `tally a b` was refused with "no 2-argument arm of `tally`
(arms take 1)": the import's function, enrolled under its bare name, took
the call. datalog (`repeat`), make (`run`) and diff (`drop`) met the same
shape. diff's `drop` is a record in std/list too, and main built std/list's
record for `drop 3 4`, whose field `d.a` the run then could not read.

`own_types_beat_twins` already let a module's own type beat an import's type
of the same name. It now does the same for an import's function: a
synthetic bare function named like a type the module declares is dropped,
at any arity. A module cannot declare a function and a type of one name, so
such a function can only be an import's twin. The import's function stays
reachable as `list/tally`.

`tests/golden/micro/a_type_named_like_an_imported_function_is_built` builds
`tally` and `drop` and calls `list/drop` beside them, on every engine. Main
refuses it.

## 2026-10-25 — push reads the bytes text/bytes answers, on every engine

From #2091's fifth item, kv's port. Appendix b declares `text/bytes string ->
int[]`, and the 2026-10-12 entry made `list/concat` read a bytes value as its
byte values on both engines. `push` was left out: `push (text/bytes "a") 2`
stopped with "push takes a list and a value" on the interpreter and natively,
and `kanso check` passed it. A push onto bytes now answers the list of their
values with the item after them, on both engines. The bytes it came from are
unchanged, and a list is still never read as bytes.

Native code takes this path out of line, behind the tag test the push already
made, so a push onto a list runs the instructions it ran before.

`tests/golden/micro/a_push_onto_text_bytes_answers_a_list` pushes onto
`text/bytes`, onto that answer again, and onto a slice of bytes. Main stops at
the first line.

## 2026-10-24 — an arm names only a type its file can see

mal's port wrote `shown (held v)` in a module that cannot see `held`, a type
private to another module (#2091, item 4; raytracer F15 is the same shape).
`kanso check` passed it. The interpreter passed the arm over and printed "got
nothing", and native stopped with "native backend: unknown type `held`", with
no file or line.

`patterns()` in `check_annotation_names` left this case to dispatch on
purpose. Its comment said the obvious check, a constructor pattern whose type
`program.types` does not hold, calls `std/list`'s `(entry k v)` wrong. Two
more sets turned out to be needed. A type an import exports is named bare
(`(file_not_found _)` from std/os in bc and datalog), and the file's imported
globals hold it by its constructor. A type whose bare name a function of the
file takes, as `fn step` beside std/list's `step` record, stays a type in a
pattern; the import's enrolled twins hold that one. `unseen_ctor` now refuses
a parameter pattern whose type is in none of those, is not built in, is not
qualified, and is not `entry`. The caret sits on the first field, as the
other constructor diagnostics' do.

`errors/an_arm_takes_apart_a_type_nobody_declared` is the one-file case and
`errors_module/an_arm_takes_apart_another_modules_private_type` is mal's;
main passes both. The stdlib, the twenty ports and kq check as before.

## 2026-10-25 — a test that fails inside a library names the library's line

From #2091's sixth item, chip8's port. A test whose helper handed
`text/split` a none failed inside std/text, and `kanso test` printed
"split takes two strings" with `--> m/m_test.kso:38:3`, std/text's line
number under the test file's path. In the port, line 38 of the test file was
a different test, one that had passed. The runner rendered every failure's
span against the test file's own source.

A failure that knows its place now prints it the way `run` and `play` do,
`failed in text/split at std/text/text.kso:38`, under the 2026-10-15 ruling
(#1975) that a failure names the line it happened on. A failure with no place
keeps the rendered span.

`tests/golden/test_verb/fails_in_std` is the reduction, and
`a_test_failing_inside_a_library_names_the_librarys_line` in
`tests/test_verb.rs` pins both streams.

## 2026-10-25 — a binding refused for another file's name says which file

#2085 asks whether a local may reuse a name declared only in another file of
its module, and notes that the message is cloud's to fix whatever the
ruling. rg's port had `` `escaped` is already a declaration; rename the
binding `` with the caret in `m/a.kso` and the declaration in `b.kso`, which
the message did not name. The module path now looks the name up in the
module's other files when it reports one of these, and says where it is:
`` `escaped` is already a declaration, in m/b.kso at line 1; rename the
binding ``. The lookup runs only on a refused compile.

`errors_module/a_binding_named_like_a_declaration_in_another_file` is rg's
shape, and main prints the message without the place.

## 2026-10-25 — a function's arms split across files are counted together

From #2090, toml's port. A module's files share their declarations, and
toml's `lookup` had a 2-argument arm in one file that called a 3-argument
arm in the next. The per-file check counted only the arms in the file it was
reading and refused the call with "no 3-argument arm of `lookup` (arms take
2)". The same two arms in one file ran.

The module path now records, for each function its files declare, whether
more than one file declares arms of it, and the per-file arity check leaves
such a name to the merged check, which sees every arm. A count no file's arms
take is still refused there, and the message names every arity: "no
4-argument arm of `lookup` (arms take 2, 3)", where the per-file check had
said "arms take 2".

`tests/arms_split_across_files.rs` runs toml's shape on both engines and
pins the refusal; main fails both.

The module path now counts, per name, which files declare it. The rows are
counted with the batch this landed in, at the end of "an import stays in the
file that writes it".

## 2026-10-25 — a field is read without importing its owner, on every path

From #2090, regex's port. Chapter 03 says nothing has to be imported to read
a field, and the 2026-10-12 gavel on #1894 says a value carries the arms its
type's owner wrote, a field of a parameter included. A module `u` that
imports nothing and reads `h.start` and `h.end` on a value module `m` built
was refused on check, run and run --interp with "no record type has a field
`start` (module u)": the module was checked alone before the program that
imports it, and no type it could see declares the field.

A library compile now leaves to the program a field read on one of the
function's parameters when no type the library can see declares the field,
as it leaves a call on a handed value that only another module writes. The
same holds for two fields read on one parameter when one of them is unseen.
A read on any other value, a local built from a visible type say, is refused
where it is written, as before, and so is a parameter read for fields that
visible types each declare but none holds together. The root (an entry, a
play file, a test file) refuses a field no type in the whole program
declares.

`tests/a_field_read_needs_no_import.rs` runs regex's shape on both engines
(main refuses it) and pins the root's refusal of a misspelt field.

A library's field reads reach the root already rewritten into calls on their
readers, so the root now asks a reader's call the same question.
`errors_module/field_read_in_a_deep_library` keeps its refusal at the same
file and line, now said by the root. The root quotes no line from another
file, as it does for every diagnostic about one, and the dependency's
`(module deep)` suffix goes with the dependency's check.

## 2026-10-25 — an import stays in the file that writes it

From #2090, xsv's port. A module's file `join.kso` declared a private
`matches` and imported nothing; its sibling `search.kso` imported std/regexp,
which exports `matches` too. The module's merged program held std/regexp's
bare twin of `matches` beside the private arms, so the call in `join.kso`
dispatched over both, and the check refused `matches index k != []` because
one of std/regexp's arms can answer an err. The 2026-10-12 gavel says a file
names only what it can see, and `join.kso` never imported std/regexp.

The 2026-08-29 ruling keeps the mixed group where it is asked for: a file
that imports the module and writes the bare name dispatches over both the
module's arms and the import's. So the module path now drops an import's bare
twin of a name the module declares only when no file importing that module
writes the bare name. The import's function stays reachable by its qualified
spelling, `regexp/matches`. A module whose importing file does write the bare
name keeps the mixed group, and a module of one file is unchanged.

`tests/an_import_stays_in_its_file.rs` runs xsv's shape with a qualified call
beside it, on both engines; main refuses it.

CI's rows, for the ten fixes that landed together (#2105). The front end
asks more of every program: whether an arm's type is one its file can see,
whether a module's own type or function shadows an import's twin, which
files declare each name, which files import each exporter, and whether a
field read stands at the root. compile_instructions 24,486,012 -> 24,681,406
(+0.80%), entry_instructions 72,317,212 -> 72,814,555, library_instructions
71,993,114 -> 72,478,036, compile_allocs 13,815 -> 13,836, emit_instructions
5,637,678 -> 5,637,990, and in the tab browser_compile_instructions
218,016,527 -> 218,660,965. The interpreter's rows, which count its run of
the corpus front end included, move by less than a thousandth each:
interp_instructions 385,499,694 -> 385,496,174 and interp_allocs 401,591 ->
401,614. The runtime gains `k_push_onto_bytes`, so
every benchmark's machine code grows 320 bytes, text 5,498,444 -> 5,502,924
over the fourteen, and building compiles it: codegen_instructions_dev
116,447,795 -> 116,453,133 and codegen_instructions_release 370,219,514 ->
370,245,362. The run rows move with the layout around that code, by under
a thousand each: work_runbench 788,981,005 -> 788,981,075, work_encodebench
2,325,240,037 -> 2,325,240,758, work_livebench 936,030,078 -> 936,030,540 and
work_oneshot 10,050,617 -> 10,050,106. Welfare falls by less than a hundredth, and
the floor follows under the rule for the language doing what it says: the
2026-10-12 ruling that a file names only what it can see and that a test
file is an outside consumer, the 2026-08-29 ruling that the author's own
name wins, and the 2026-08-03 ruling that a module's files share their
declarations.

Four ratchet mutations anchored on lines these fixes rewrote and no longer
applied: `the_library_program_is_checked_twice` (the library body moved into
`compile_library_beside`), `the_entry_program_is_checked_twice` and
`an_entry_program_rewritten_twice_before_it_is_checked` (the entry check took
a `root` argument and a line of its own), and
`a_local_arm_shadows_every_shape` (`sibling_fns` became a map). Each is
re-pointed, and each was dry-run on a copy whose mutated tree builds. Every
other mutation still applies to this tree.

## 2026-10-25 — four cliffs from the ports join the run program

#2089's first item, entered in the corpus before its fix under the
2026-09-05 corpus-first gavel. toml, datalog, diff, bc and inflate each read
a map entry into a name and handed the name to a function inside the write:
`seen = m[key]` then `put m key (counted seen)`. The 2026-10-10 entry
(kanso#1822) discounts such a name only when it reaches the write through
arithmetic and comparison, so here `m` counts as used twice and every put
copies the map. toml's reduction at 8,000 keys took 4.2 seconds in release
against 0.012 with the read written inline.

The run program gains a `handed` phase: 380 updates over 100 keys through
such a name. Built alone on main it costs 41,508,775 instructions above the
empty program. The phase prints the same total on both engines.

#2089's second item joins it. markdown's port pushed onto a list held in a
record under a guard, `return st if i > n` above `loop (state (push
st.events i) i) (i + 1) n`, and natively 64,000 pushes took 8.9 seconds.
kanso#1926 moves such a list out of its record only when every mention of the
record is a field read, and the guard's `return st` is a mention that is not,
though it runs instead of the push. The same loop written as two arms grows
the list in place. The `guarded` phase pushes 3,050 times. Its cost is a
cliff whose knee depends on what the arena already holds: built alone the
knee is at 4,096 elements and the phase costs 746,860 instructions, while
inside the run program the knee falls between 2,000 and 3,000 and the phase
costs 43,734,573, the whole program with it less the whole without.

#2089's fourth item is the third phase. chip8's port kept its memory as a
map of 16 keys in a record, read one key and overwrote another each lap, and
8,000 laps took 2.4 seconds natively against 0.1 on the interpreter. Every
put that cannot write in place copies the map's append log, which keeps each
overwrite until something sorts it, so the copy grows by an entry a lap. The
`overwrite` phase runs 380 laps, 42,238,603 instructions built alone above
the empty program, and prints the same total on both engines.

On this container the whole program reads 917,589,885, where main reads
788,981,054: `handed` is 4.52% of it, `guarded` 4.77% and `overwrite` 4.60%.

#2089's third item is the fourth phase. lox's statement loop re-enters itself
when a Lox constructor builds a tree, carrying a persistent trie that each
lap writes one leaf into, and the reduction took 7.0 seconds natively for
4,094 writes. The lap rule stages a carry once the arena has drifted a
quarter megabyte, measured from where the last stage at the loop's depth
left it. A fresh entry measured from the previous loop's position, which the
rewind had already passed, so every entry staged on its first lap and copied
the whole trie. The `reentered` phase writes 62 leaves. Its cost is a knee
like `guarded`'s: 1,236,018 built alone with the empty program's start-up,
and 70,582,705 inside the run program, where each stage also walks what the
earlier phases left. Depth 3 sits below the knee and adds 1,254,367. With it
the whole reads 988,276,908 and `reentered` is 7.14% of that. Both engines
print 62.

CI's rows. The run program with all four phases reads work_runbench
788,981,075 -> 988,172,222, the four together 199,191,147 instructions, or
20.2% of the new whole. The program is larger, so the code it emits grows:
emitted_other_defines 2,148 -> 2,178, emitted_other_calls 19,353 -> 19,778,
emitted_other_branches 16,926 -> 17,310, emitted_other_lines 160,895 ->
163,953, and its machine code 762,432 -> 777,824 bytes, text 5,498,444 ->
5,518,316 over the fourteen. No compile row and no browser row moves. Under
the 2026-09-05 corpus-first gavel this is a re-basing, as when rewind,
stack and bag joined: the run_instructions baseline keeps its ratio to the
run program, 3,610,619,845 -> 4,522,179,743, so welfare holds at 93.28 and
the fixes in #2106 score against these rows as a rise.

The run program's allocation vein moves with it, and the peak it holds is the
run-memory term: run_held_peak_bytes 111,920 -> 2,397,872. That term is
re-based the same way, run_peak_bytes 3,928,865 -> 6,214,817 against a
baseline that moves from 1,434,645,006 to 2,269,371,987. The other rows land
on run_allocs 1,451,390, run_alloc_bytes 248,577,384, run_beat_iters 89,113,
run_evac_allocs 13,208, run_evac_bytes 15,563,184, run_perm_allocs 101,
run_push_mut_slow 1,554,430, run_put_mut_grow 17, run_sh_buf 115,790,064,
run_sh_map 24,320, run_sh_rec 3,920,752, run_survive_slots 395,834,
run_ten_blocks 2.

The fixes follow in their own pull requests and will score as rises.


On this container the whole program reads 874,292,848, where main reads
788,981,054; `handed` is 4.75% of it and `guarded` 5.00%. Both fixes follow
in their own pull requests and will score as rises.

empty program, 5.00% of the whole program's 830,558,275 on this container,
where main reads 788,981,054. The phase prints the same total on both
engines. The fix follows in its own pull request and will score as a rise.

## 2026-10-25 — a read handed to a call keeps the write in place

The fix for the `handed` phase the previous entry put into the run program
(#2089, first item). `seen = m[key]` then `put m key (counted seen)` copied
the map on every put, because `linear.rs` discounts a name bound to a read of
the map only when every later use reaches the put through arithmetic: a use
anywhere else could force the read after the write.

That can happen only if the binding is a thunk, and `demand::analyze` makes a
binding a thunk only when its expression is expensive, which a plain index
read with a key that calls nothing and compares nothing is not. Both engines
read the same verdict, so `seen` holds the element before anything below its
line runs. `read_in_block` now discounts such a binding's read of the map
whatever the name does next. Pooled names, which the work-ahead pass always
makes cells, are left out.

`tests/golden/mem/a_read_handed_to_a_call_stays_in_place` is the shape at
2,000 updates over 100 keys: 15 allocations and 16,800 bytes, where main
makes 6,006 and 80,274,944. No other mem golden moved, and
`micro/a_read_named_before_a_write_sees_the_old_value`, whose names are used
after the write, still prints the old value beside the new on every engine.
On this container the `handed` phase built alone falls from 41,838,242
instructions to 1,090,527, each including the empty program's 329,467; CI's
rows follow.

## 2026-10-25 — a guarded loop grows its record's list in place

The fix for the `guarded` phase (#2089, second item). kanso#1926 lets a push
through `st.events` write in place when the record is handed over and every
mention of it in the arm reads a field. markdown's loop answered
`return st if i > n` above the push, and that early `st` is a mention that
reads no field, so the move was refused and every lap copied the list. The
guard's early answer runs instead of the lines below it, never beside them.
`FieldTally` now walks a guard's early answer apart from its rest, and when
the early answer reads no field of the record it does not count against the
move below. One that does read the field is counted as before.

`tests/golden/mem/a_list_in_a_record_grows_under_a_guard` pushes 20,000
times under such a guard: 14 allocations and 1,045,008 bytes, where main
makes 51,816 and 9,160,000,672. markdown's reduction at 64,000 pushes runs in
0.005 seconds natively, where main took 8.9.
On this container the run program with the `guarded` phase falls from
874,292,848 instructions to 831,115,619 and prints the same total; CI's rows
follow.

## 2026-10-25 — a copied map leaves its dead entries behind

The fix for the `overwrite` phase (#2089, fourth item). A map keeps an append
log of its pairs, overwrites included, and sorts and deduplicates it into a
cached view on the first read. A put that cannot write in place copies the
map, and `k_b_put` copied the whole log. chip8's memory, 16 keys read and
overwritten once a lap, therefore carried every earlier lap's entries into
each copy: 8,000 laps allocated 1,783,786,288 bytes and took 2.1 seconds
natively.

When the map's sorted view is current and shorter than its log, the copy now
starts from the view, which holds each key's last value, and the new pair goes
after it. The answer reads the same either way. The chip8 shape at 8,000 laps
falls to 9,286,608 bytes and 0.015 seconds, and prints what the interpreter
prints.

`tests/golden/mem/a_map_overwritten_each_lap_is_copied_without_its_dead_entries`
runs 2,000 laps: 8,172 allocations and 2,324,416 bytes, where main makes
8,130 and 86,135,584. `fused_tally.mem` moves with it: alloc_bytes 33,872 ->
33,088, held_peak_bytes 7,648 -> 7,392 and sh_buf 9,952 -> 9,568, because its
tally's map is copied from a view too.

## 2026-10-25 — a loop entered from its own lap stages by its own drift

#2089's third item, the `reentered` phase. lox's statement loop re-enters
itself when a Lox constructor builds a tree, and its reduction took 7.0
seconds natively at depth 10, 4,094 writes, where the interpreter writes
16,382 leaves in 0.4.

Two things in the lap rule made every entry copy the whole tree. A loop
measured its drift from the position the last stage at its depth had left,
and when a fresh loop began, that position was below a rewind already. So
the first lap of every entry staged. The push now records the loop's own
entry position, and depth 10 fell to 0.099 seconds.

That left a quadratic with a smaller constant: depth 12 took 0.80 seconds
against the interpreter's 0.37. The wait between stages grows to twice what
the last stage copied, but that size was read off the carry pair, which the
push empties. Each fresh entry therefore waited the default quarter
megabyte and then copied a tree of several megabytes. The size is now kept
per depth, across loops, and depth 12 takes 0.28 seconds. Skipping a stage
is always sound; what the longer wait costs is garbage held until the next
stage, bounded by twice the depth's last copy.

`a_loop_entered_from_its_own_lap_copies_its_tree_rarely` pins it in the mem
vein at depth 8: evac_bytes 25,762,640 on main and 3,004,432 with the fix,
evac_allocs 805,083 and 93,889, beat_iters 8,177 on both.

Fourteen other mem goldens move. Eleven fall, because a fresh loop no longer
stages on its first lap: `record_reuse_shape`, for one, evacuates 80 bytes
where it evacuated 144. Three rise, the fixtures that pin a repaired node
below a loop. A pop after a skipped lap walks its whole value, and these
loops now skip where they used to stage, so they copy 160, 432 and 288 bytes
more. `a_map_overwritten_each_lap_is_copied_without_its_dead_entries` holds
32 bytes more at its peak. On this container the run program with the
`reentered` phase reads 988,276,908 on main and 881,351,361 with this change
alone. That is below the 917,694,203 main reads without the phase, because
other phases' loops stopped staging on their first lap too.

The ratchet's row "a thunk's captures unsized by the walk that evacuates
survivors" reported BLIND on this branch's tree: its mutation, which stops
the sizing walk looking inside a thunk, left its gate green. It is blind on
main as well, at cdc6be5c and at a70d2458, and this branch is the first since
then to touch `core.c` and so select the row. Its witness,
`micro/a_thunk_capture_survives_the_cohort`, was written to grow past the
cohort threshold and read its captures after the rewind; its program no
longer grows that far (cohort_frees=0), and a read of reclaimed memory goes
wrong only when something has written over it. Under the mutation the
survivor copy writes past the end of the carry buffer the walk sized, and
nothing reads the damage. `k_copy_alloc` now refuses a copy that outruns the
walk, "a carry copied more than its sizing walk counted", and under the
mutation `mem/work_ahead_pools_a_call_on_a_builtin_read`, whose pooled cell
carries its captured list through a stage, dies with that message. The row's
gate is the mem corpus now, and the mutation's comment names the new witness.
No program reaches the refusal on an unmutated runtime, so
tests/golden/unpinned_diagnostics.txt lists it with that reason.

The bound check is one compare and a cold call per carry allocation, and CI
reads it on that head. Every benchmark's machine code grows 112 bytes, text
5,521,457 -> 5,523,025 over the fourteen. work_runbench 797,618,426 ->
797,623,329, work_deepbench 197,973,136 -> 197,978,249, work_encodebench
2,327,444,715 -> 2,327,445,961, work_pendbench 155,737,782 -> 155,739,092,
work_livebench 936,030,277 -> 936,030,663, work_widebench 28,445,045 ->
28,445,165, work_digestbench 4,122,908 -> 4,122,953, work_indexbench
1,837,223 -> 1,837,250, work_scanbench 274,291 -> 274,312, work_jsonbench
557,386,421 -> 557,386,433, work_readbench 4,581,986 -> 4,581,998,
work_escapebench 40,308,398 -> 40,308,401, work_basket 31,039,074 ->
31,039,077 and work_oneshot 10,050,789 -> 10,050,755. The codegen rows
move with the runtime's text: codegen_instructions_dev 116,452,600 ->
116,456,350 and codegen_instructions_release 370,226,904 -> 370,087,521. The
tab runs the same runtime compiled to wasm, so browser_run_instructions
33,345,113 -> 33,345,263. No allocation counter or mem golden moves.

CI's rows for the four fixes together (#2106), against the run program that
already holds their shapes (#2104). work_runbench 988,172,222 -> 797,618,426,
a 19.3% fall, which is the rise this pull request banks. The other run rows
move by layout and by the carry change: work_deepbench 198,788,407 ->
197,973,136, work_widebench 28,461,019 -> 28,445,045, work_basket 31,062,419
-> 31,039,074, work_encodebench 2,325,240,758 -> 2,327,444,715,
work_oneshot 10,050,106 -> 10,050,789, work_livebench 936,030,540 ->
936,030,277, work_digestbench 4,122,778 -> 4,122,908 and work_escapebench
40,308,387 -> 40,308,398. The tab runs its corpus in
browser_run_instructions 33,345,113. The linearity changes cost the front
end little and the binary's layout moves the rest: compile_instructions
24,681,406 -> 24,664,014, entry_instructions 72,814,555 -> 72,772,292,
library_instructions 72,478,036 -> 72,435,518, emit_instructions 5,628,759,
browser_compile_instructions 218,660,965 -> 218,703,716,
codegen_instructions_dev 116,452,600 and codegen_instructions_release
370,226,904. The runtime grows, so every benchmark's machine code does too,
text 5,518,316 -> 5,521,457 over the fourteen, and the run program's emitted
code loses three lines, emitted_other_lines 163,953 -> 163,950.

The mem vein moves as the carry entry above says. The three fixtures that
pin a repaired node copy a little more:
- a_node_repaired_by_an_inner_pop_outlives_the_outer_stage_evac_allocs 165,
  a_node_repaired_by_an_inner_pop_outlives_the_outer_stage_evac_bytes 5,728;
- a_repaired_node_below_a_loop_that_copies_nothing_allocs 273,
  a_repaired_node_below_a_loop_that_copies_nothing_alloc_bytes 7,058,721,
  a_repaired_node_below_a_loop_that_copies_nothing_evac_allocs 157,
  a_repaired_node_below_a_loop_that_copies_nothing_evac_bytes 5,440,
  a_repaired_node_below_a_loop_that_copies_nothing_survive_slots 22;
- a_repaired_node_below_a_loop_that_returns_a_number_allocs 115,
  a_repaired_node_below_a_loop_that_returns_a_number_alloc_bytes 3,532,993,
  a_repaired_node_below_a_loop_that_returns_a_number_evac_allocs 41,
  a_repaired_node_below_a_loop_that_returns_a_number_evac_bytes 1,344,
  a_repaired_node_below_a_loop_that_returns_a_number_survive_slots 10.

a_long_chain_answers_survival_from_its_index_chain_finds 95,601 -> 95,597
moves with one fewer stage.

The run program's allocation vein, read by the counter sweep and matching CI's
allocs and alloc_bytes: run_allocs 1,451,390 -> 1,444,950, run_alloc_bytes
248,577,384 -> 213,685,416, run_evac_bytes 15,563,184 -> 4,822,160,
run_evac_allocs 8,626, run_survive_slots 5,891, run_put_mut_grow 22,
run_push_mut_slow 1,557,480, run_sh_buf 86,615,088, run_sh_rec 3,906,544,
run_sh_map 12,160, run_bytes_malloc 7,885 and run_ten_blocks 1. Of its peaks,
run_held_peak_bytes falls 2,397,872 -> 233,760, while run_arena_peak_bytes
rises one block, 3,670,032 -> 4,718,608, and run_perm_peak_bytes 146,913 ->
212,465. Which of the four fixes moves the arena and permanent peaks is not
isolated here. The basket vein loses two evacuations: basket_allocs 27,270,
basket_alloc_bytes 6,162,417, basket_evac_allocs 3, basket_evac_bytes 192 and
basket_sh_rec 130,112.

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
