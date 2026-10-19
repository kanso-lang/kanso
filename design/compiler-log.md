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

## 2026-10-13 — `list/concat` joins two lists

#1906 moves concat to the module of the type it works on. `list/concat a b`
joins two lists, or the byte values `text/bytes` answers, and is the same
builtin underneath that `text/concat` was, so every engine runs it as
before. `text/concat` is gone from `lib/text/text.kso`, and every caller in
the tree moved: the library, the scripts, the benchmarks, the examples, the
book and the goldens. Files that imported std/text only for concat now
import std/list instead.

Two strings join by interpolation, `"{a}{b}"`, and a list of them by
`text/join`. The checker refuses a string handed to `list/concat` when it
can see one: a string literal, a template, or a call whose every answer is a
string. A string it cannot see still reaches the runtime, which raises
"concat takes two lists" on both engines. A call to `text/concat` is
refused at check with the unknown-name diagnostic, which now names
`list/concat` and interpolation. Each refusal has an error golden.

Appendix B's entry is rewritten under std/list. std/text lost three lines
above `to_float` and `to_int`, so the line numbers that goldens and the book
quote from `std/text/text.kso` moved with it.

kq's one caller moves on the kq branch of the same name.

On CI's rows the front end pays for the new refusal and for the moved imports,
and I have not split the cost between the two. compile_instructions went from
28,002,980 to 28,257,376, entry_instructions from 92,393,206 to 93,065,372,
library_instructions from 91,989,327 to 92,696,544, `compile_peak_bytes` from
703,170 to 703,187 and browser_compile_instructions from 413,936,384 to
416,674,694. The interpreted run's `interp_peak_bytes` went from 722,393 to
725,610 while its instructions fell to 495,318,943 and its allocations to
589,643. compile_allocs fell to 15,937 and emit_instructions to 7,239,817. On
the module path `module_visits` went from 2,909 to 2,912. The ratchet's
mutation for a concat accumulator anchored on the line naming `text/concat`,
so it now names `list/concat`, and the mem spec still goes red under it. The
floor follows the score, since #1906 builds a ruled part of the language.

## 2026-10-13 — six names for the library gaps, and `list/to_sorted`

#1914 rules nine names. Six of them are built here, in kanso, in
`lib/text/text.kso` and `lib/list/list.kso`, so every engine runs the same
source and no engine needs a new builtin:

- `text/uppercase` and `text/lowercase` map the twenty-six ASCII letters and
  pass every other character through. The walk takes `chars` once, looks
  each letter up by its offset in a 26-letter string, and joins the pieces.
  Case outside ASCII would need the Unicode tables, which no engine carries.
- `text/starts_with?` and `text/ends_with?` compare one slice against the
  prefix or suffix after checking the length, so the slice never runs past
  the end. Every string begins and ends with the empty one.
- `text/pad_start` and `text/pad_end` repeat the fill and cut it at the
  width. A string already that wide, or an empty fill, comes back unchanged.
- `list/sort_by xs key` merges the way `sort` does and takes from the right
  half only when its key is strictly smaller, so tied keys keep their order.
  `key` runs at each comparison; caching it would cost a list of pairs.
  `list/reverse` reads the list by index from the end.

`recase` has a `none` arm it never takes. The walk stops before it indexes
past the end, but the checker cannot see that bound and asks for the arm.

Each name has a micro golden, which the interpreter, the native build and
the page run alike. Appendix B gains entries for the six text names and a
sentence for the two list verbs, and drops its counts of the list verbs,
which a second count in the library would have had to track.

#1903's ruling is built beside them. `list/to_sorted xs` answers
`sorted (sort xs)`, so a caller can reach the arm of `find` that bisects, and
the list is always in order because the same call sorts it. The bisect tests
are back in `list_test.kso` through it, and a micro golden checks that `find`
agrees on the plain list and on `to_sorted`. The mem golden
`find_bisects_a_list_built_by_to_sorted` pins the bisect: a scan over its
4,096 elements builds a `step` record per element, and `sh_rec` reads 208
bytes with the bisect against 512,400 when `find` loses its `sorted` arm.

The other three rulings need the runtime: `delete map key`, `io/read_line`
and `os/modified`. They follow in their own change. `os/write_bytes` was
ruled out, so there is nothing to build for it. The ports that worked around
these gaps live on the unmerged `claude/ports-corpus` branch, and moving
them onto the names belongs to that branch.

CI's rows price the new library text. Every program that imports std/text or
std/list compiles the whole module, so the 114 lines added there are paid for
in lexing, parsing, inference and the check, split across those phases in
proportion to the text rather than concentrated in one. compile_instructions
rises from 28,002,980 to 31,634,022 (+13.0%), entry_instructions from
92,393,206 to 100,520,915 (+8.8%), library_instructions from 91,989,327 to
100,042,203 (+8.8%), emit_instructions from 7,241,390 to 7,617,766 (+5.2%),
compile_allocs from 15,984 to 17,990, browser_compile_instructions from
413,936,384 to 434,142,466 (+4.9%) and browser_compile_peak_bytes from 686,192
to 726,939. The interpreter row reads 496,099,798 against 495,525,421, with
interp_allocs 592,223 against 589,689 and interp_peak_bytes 794,345 against
722,393. Welfare falls from 91.34 to 91.27. These names are ruled (#1914,
#1903), so the floor drops by that cost. A compiler that left an unreached
library function unparsed and unchecked would stop this cost from growing with
each library addition; that is open as a lead.

Two more rows move with the library's size. interp_instructions lands on
496,099,798 from 495,525,421, and the module sample's module_visits, the
inference fixpoint's declaration visits in bench/compile_golden_modules.txt,
lands on 3,663 from 2,909: the fixpoint visits each new declaration on every
round it is dirty.

The compile-memory gate reads the same text. On `compile_corpus`,
`front_end_visits` went from 8,133 to 9,326, `front_end_rounds` stayed at 19,
and `compile_peak_bytes` went from 703,170 to 791,610, 12.6% more held while the
corpus is checked, because the two modules it imports are 114 lines longer and
their whole text is held through the check. It costs the welfare meta about
0.02, and the floor drops by that under the language rule. Checking only what
an import reaches would take this back, and #1952 tracks it.

## 2026-10-13 — hex and binary integers, and floats with an exponent

#1889 builds the gavel "hex and binary integers and exponent floats". The
lexer reads `0x` followed by hex digits, in either case, and `0b` followed by
binary digits as integers: `0xff` is 255 and `0b1010` is 10. A literal past
64 bits becomes a big integer, as a long decimal does. A decimal followed by
`e`, an optional sign and digits is a float: `1e-8`, `6.02e23`, `2.5e3`. A
minus in front reads as it does for a decimal, so `-0x10` is -16. The
prefixes and the `e` are lower case only, as every other word in the
language is.

Three malformed spellings stop at lexing, each with an error golden: `0x`
with no digits after it, a digit other than 0 or 1 in a binary literal
(`0b2`), and an `e` with no exponent digits (`1e`). Each message names the
literal and says how to write it.

Every engine reads the same tokens because they share the lexer. The micro
golden `hex_binary_and_exponent_literals` runs on the interpreter, the
native build and the page. The editor grammar and the playground's
highlighter colour the new forms, and chapter 02 says what each spelling
means. The byte literal in #1889's title was not ruled and is not built.

On CI's rows the lexer's new arms cost the emitter 408 instructions,
emit_instructions 7,240,982 -> 7,241,390, and the tab's compile 10,978,
browser_compile_instructions 413,925,406 -> 413,936,384. The rest fell with the
binary's layout: compile_instructions by 9,355 to 28,002,980,
entry_instructions by 30,753 to 92,393,206, library_instructions by 30,149 to
91,989,327 and interp_instructions by 3,960 to 495,525,421. The floor follows
the score, since #1889 builds a ruled part of the language.

## 2026-10-14 — `delete`, `io/read_line` and `os/modified`

These are the three #1914 names that need the runtime. #1948 carries the six
written in kanso. All three run on the interpreter, both native tiers and the
page's translator.

`delete map key` answers the map without the key. A key that is not there
answers an equal map, and the map it was handed is not changed. It sits beside
`put` as an ambient name, so a program that declares its own `delete` is now
told the name is taken; kq, vse and kanso-json declare none. Natively the
answer is built from the map's deduplicated sorted view, so a key written twice
is gone after one `delete`. A first argument that is not a map is refused at
check time, and a float key at run time.

`io/read_line` is a description that yields the next line of standard input
without its `\n` or `\r\n`, or `done` at the end. A lone `\r` is kept, since
only those two endings were ruled. It flushes standard output before it waits,
so a prompt written ahead of it reaches the terminal first. Bytes that are not
text are refused the way `os/read_file` refuses them. A read error counts as
the end of input, which is what `io/stdin` already did natively. The page has
no standard input and refuses `read_line` as it refuses `stdin`.

`os/modified path` yields when the file last changed, in milliseconds since
the epoch, or `file_not_found`. A failure other than absence is reported in
`os/read_file`'s words. In the page it yields `file_not_found`, matching
`os/exists`.

New goldens: four micro programs, one error and two runtime diagnostics, and a
stdin corpus under `tests/golden/stdin/` that `tests/read_line.rs` feeds to
native and the interpreter. One test there waits for each prompt before typing
the answer, so an unflushed prompt fails it. Each was watched red: native and
interpreter `delete` keeping the key, `\r` left on the line, and the flush
removed.

CI's rows price the three builtins. Every native binary carries their runtime
code, 2,688 bytes more text per bench, so `text` went from 4,932,142 to
4,969,774 across the fourteen. Each bench also runs about 234 more
instructions, the same on every one, so the rise is outside any decode loop:
work_basket 30,946,278, work_deepbench 403,499,544, work_digestbench 5,909,247,
work_escapebench 39,058,340, work_indexbench 2,462,571, work_jsonbench
858,186,231, work_livebench 1,474,854,142, work_oneshot 13,205,901,
work_pendbench 191,674,024, work_readbench 4,580,963, work_runbench
1,178,202,929, work_scanbench 273,675 and work_widebench 28,274,676, while
work_encodebench fell to 2,386,636,337. The compiler checks and lowers three
more names and carries the larger runtime text: compile_instructions
28,044,444, entry_instructions 92,512,951, library_instructions 92,130,190,
emit_instructions 7,245,801, codegen_instructions_dev 119,400,046 and
codegen_instructions_release 477,099,466. The interpreter's run went to
interp_instructions 495,589,622, and the tab to browser_compile_instructions
414,205,634 and browser_run_instructions 34,183,945. The floor drops by the
cost under the language rule, since #1914 rules these names.

Carried together over main 859ad275 in #1953, the six names and the three
builtins land on one set of CI rows, each read against main. The front end pays
for both: compile_instructions 31,948,363 from 28,257,376, entry_instructions
101,372,705 from 93,065,372, library_instructions 100,887,003 from 92,696,544,
compile_allocs 17,941 from 15,937, emit_instructions 7,622,247 from 7,239,817
and module_visits 3,666 from 2,912. The compile corpus holds more while it is
checked, compile_peak_bytes 789,911 from 703,187, with front_end_visits at
9,326. The interpreter's run reads interp_instructions 496,215,717,
interp_allocs 592,174 and interp_peak_bytes 792,646, against 495,318,943,
589,643 and 725,610. The tab's compile reads browser_compile_instructions
437,381,890 and browser_compile_peak_bytes 726,939, against 416,674,694 and
686,192, and its run browser_run_instructions 34,183,945. Most of the compile
cost is the 114 lines the two library modules grew, which every program
importing them reads whole; #1952 is the change that would take it back. Both
halves build ruled names (#1914, #1903), so the floor drops by the combined
cost under the language rule.

## 2026-10-14 — kq's rise at the 2169a0ff pin is clang 18 linking without `deadargelim`

When kq's pin moved to 2169a0ff (kq#190), its instruction rows rose: print_small
by 5.7% and path_small by 13.5%. CI builds kq with the runner's clang, which
is 18. Under clang 19 the same pin move is +1.5% on print_small and +0.07% on
path_small.

A bisect of #1938's carrier under clang 18 stops at its first step, #1898
with #1896. That step lets clang 18 link through `LTO_O3_WITHOUT_DEADARGELIM`,
which clang 19 already used. Clang 18's own `lto<O3>` aborted in `deadargelim`
on kq once closure tail calls became `musttail`.

To isolate the pipeline, I applied only the probe and pipeline change to
38fd2e60, the tree before the carrier. Built that way, kq reads print_small
27,704,820 and path_small 11,933,888, against 26,297,450 and 10,519,854
without the change. The new pin reads 27,793,443 and 11,934,905, so the
pipeline accounts for all of the rise except 88,623 on print_small and 1,017
on path_small. All six readings were taken in this container under clang 18
with `--counters`.

So the cost is clang 18's `deadargelim`: the pass that dropped the dead
arguments cannot run once tail calls are `musttail`. kanso's own cost goldens
build with clang 19, and kq's rows were counted under 18.
kq#191 records the attribution in kq's instructions golden and selects clang 19
in kq's spec job, so its rows describe the build the rest of the project
measures. CI read them there as print_small 25,716,192, print_big 245,768,599,
path_small 10,603,649 and path_big 104,470,133, between 7.5% and 11.3% under
the clang 18 rows at the same pin.

## 2026-10-14 — a linux arm64 release build keeps every tail call a jump

#1912. An arm64 release build took `tailcc` away from every arm wider than
eight argument words (`narrow_tailcc`), so a loop through a wide arm spent a
frame per hop. Dropping the narrowing had failed the macOS arm job:
`a_match_holds_the_literal_its_pattern_spells` printed nothing.

That failure reproduces off the arm job. The release IR, without the narrowing,
built for `aarch64-linux-gnu` with clang 19 and run under `qemu-aarch64`, dies
on SIGSEGV at -O1 and above and prints all eight lines at -O0. Running the
middle end at -O1 and `llc` at -O0 prints them too, so the backend is at fault.
`llc -opt-bisect-limit` puts the first bad step at CodeGenPrepare on
`regexp/cutting_6`, an arm of six `%KValue`s, twelve words. Its last action on
one path is an ordinary call to `k_b_push`. The middle end had marked that call
`tail`, CodeGenPrepare duplicated the return to make it a sibling call, and the
arm64 backend gets the stack wrong for a sibling call out of a `tailcc` function
whose own arguments arrived on the stack.

Marking every plain call in the wide arms `notail` after the middle end fixes the
program. Before it does not, because the emitter's IR carries no `tail`
markers: the middle end adds them, and inlining brings calls in with their own.
The function attribute `"disable-tail-calls"="true"` is read at codegen, after
both. On every `tailcc` function wider than eight words it turns those sibling
calls back into calls and returns. It leaves `musttail` alone. With it, the
fixture prints all eight lines at -O1 and -O3. In its assembly, 47 `musttail`
edges in the marked functions are a `b` to their callee. The others I read had
been inlined, and the jump that remained was a `b` or, through a closure, a
`br`.

On the macOS arm job the attribute fixed that fixture and the three wide-arm
tests in `tests/a_big_object_decodes_in_a_release_build.rs`, and broke another:
a release build of `a_spilling_tail_call_named_with_a_mark` printed nothing,
where the interpreter prints `false`. That job builds with Apple's clang. The
same program's IR, marked the same way, runs correctly under qemu built for
`aarch64-linux-gnu` by clang 19 and by clang 18, and clang 19's assembly for
`arm64-apple-macos` matches its linux assembly in the marked functions, so I
have no reproduction of the Apple failure here.

So `wide_tailcc_keeps_its_jumps` replaces `narrow_tailcc` on a linux arm64
release build, and an Apple host keeps the narrowing. There a wide arm still
takes the C convention, and the three tests stay off Apple arm64. On linux a
wide arm gives up only its sibling calls into helpers, which become a call and a
return. x86-64 builds are unchanged. #1912 stays open for the Apple half.

## 2026-10-14 — the body emitter moves to `src/codegen/emit.rs`

#1947, the codegen half. The largest `impl Backend` block in `src/codegen.rs`,
3,867 lines that turn one function's body into IR (patterns, binds, calls,
conditions and the arithmetic paths), moves unchanged to
`src/codegen/emit.rs`, which `codegen.rs` reads with `include!` as it already
reads `declares.rs` and `dispatchers.rs`. `codegen.rs` goes from 9,796 lines to
5,929.

The move is pure. Built from main and from this tree, the release IR for
runbench and encodebench is byte for byte the same. Thirty-one ratchet
mutations edit lines that moved, so each now names `src/codegen/emit.rs`, and
every one was run against a copy of the tree to see that it still applies. The
test that every wasm32 rewrite matches a line the emitter writes reads the new
file beside the other two. The compile rows can move with the binary's layout,
and CI's rows will say whether they did.

`runtime.c` is the other half of #1947 and stays for its own change. Its text
is embedded with `include_str!` in five places and edited by 119 mutations, so
a split has to be concatenated in Rust rather than joined with `#include`.

On CI's rows the move is layout and nothing else. The emitted IR and every
native and interpreter row are unchanged; the one row that moved is the tab's
compile, browser_compile_instructions 437,381,994 from 437,381,890, 104
instructions, because the wasm engine is built from the same source and its
code moved with the files. The floor follows that score, as it has for earlier
layout-only moves.

## 2026-10-14 — gavel: `text/slice!` takes a slice the caller knows is in range

Clay ruled #1943. Building #1890 (a `text/slice` past the end answers `none`)
showed that the checker now refuses `text/utf8 (text/slice cs a b)` and
`text/append acc (text/slice cs a b)` until the `none` is dispatched. Those
are the two spellings the compiler fuses, and the JSON decoder reads every
string through them. Its ranges come from a scan of the same bytes, so its
`none` can never occur, and a two-arm helper that dispatches it loses both
fusions. Measured in a container by callgrind on the #1890 branch: jsonbench
+33.0%, livebench +38.1%, runbench +28.0%, oneshot +149.7%.

**The ruling.** `text/slice! s a b` takes a slice the caller insists is in
range. Like every `!` name it answers a box (archive, 2026-09-16, "gavel,
reversed the same day: `!` answers a box, at the index and at the name"):
the slice, or the out-of-range failure, which bubbles. The caller opens it
with `.>`, so the decoder writes `text/slice! cs a b .> text/utf8` and
`.> text/append acc`. A caller that has not bounded its range writes
`text/slice` and dispatches on `none`, as ruled on #1890.

The decoder's speed comes from fusing those chains as the nested calls are
fused today. That is emitter work with no surface, and it is cloud's, as the
09-16 entry assigned the inlining of a bind whose callback is applied at once.
#1956 asked whether `slice!` was the one bang that answers a bare value; the
09-16 ruling already answers that it is not.

Owes, on #1943: `text/slice!` answering a box on every engine; the emitter
fusing `slice! ... .> utf8`, `.> append acc`, `.> to_int` and `.> to_float`;
the decoder's call sites moved onto those chains; a micro golden for an
in-range `slice!` and an error golden for one out of range; appendix B.

## 2026-10-14 — a box handed to a library function that reads it is refused at the call

#1957. The checker refuses a box handed to a builtin that takes a value, so
`length (os/read_file! "x")` is refused where it is written. Handed to
`text/utf8` instead, the same box checked clean, and both engines died with
"utf8 takes a list of byte values". `text/utf8` is a library function whose
body calls the builtin as `builtin_utf8`, and the table of builtins that read
a value was asked under that spelling, which it does not hold.

The table is now asked with the prefix taken off. That alone moved the
refusal into std/text, naming `builtin_utf8` and a line the program's author
cannot change. So when a box goes into a standard-library function whose body
reads that parameter as a value, the refusal comes at the program's call and
names the function called. An arm that takes an effect at that position is
where a box belongs, and a group with one is left alone. A box handed to the
program's own functions is still followed into the body, as ruled on
2026-09-30. `a_box_handed_to_a_library_wrapper` pins the refusal and was red
with the change taken back out.

`browser_compile_instructions` moved 437,381,994 -> 437,416,347 for the
extra walk over a std group's arms, under a ten-thousandth of a point. CI
read `compile_instructions` 31,948,363 -> 31,940,603, `entry_instructions`
101,372,705 -> 101,349,935 and `library_instructions` 100,887,003 ->
100,864,357, a fall of a few hundredths of a percent that this change does not
explain, so it is named as layout.

## 2026-10-14 — checking only what a module reaches, measured and set aside

#1952 asked whether the merged check could skip the dependency functions a
module never reaches, since every program that imports std/text or std/list
now checks 114 more lines of them. The branch
`claude/check-only-what-an-import-reaches` sets those functions aside while the
merged check runs and puts them back afterwards. They have to go back because
the merged program is also what an importer of the module receives, and an arm
the module never calls can still travel to one that does.
`two_owners_tie_on_a_traveling_call` in the module error corpus caught the
first version, which dropped them for good.

Measured in this container on the tree that carries #1948 and #1950:
compile_instructions fell from 31,948,363 to 30,683,796, but
entry_instructions rose from 101,372,705 to 108,124,116 and
library_instructions from 100,887,003 to 105,435,412, with compile_allocs at
18,650 against 17,941. The walk that finds what is reached took 1.5 to 1.9 ms
of a 19 ms entry compile, and inference fell only from 3.02 to 2.79 ms. Lexing
and parsing the larger libraries are where most of their cost lands, and a
smaller check does not reach either. Not reading an unreached function at all
would mean parsing a library a declaration at a time, which is a larger change;
#1952 stays open with these numbers on it.

## 2026-10-14 — `text/slice` past the end answers `none` on every engine

This builds the gavel "`text/slice` past the end answers `none`" (#1890). A
range that ends past the value, or starts more than one past its end, answers
`none` in the interpreter, the native runtime and the tab. A range that ends
exactly at the end is the tail, and an inverted range is still the empty
result. The checker puts `none` in what `slice` answers, so handing a slice to
a function with no `none` arm is refused where it is written.
`tests/golden/errors/a_slice_handed_on_without_its_none` pins that refusal,
and it checks clean with the `none` taken back out of the type.
`tests/golden/micro/a_slice_past_the_end_answers_none` pins the three ranges
the ruling names, on strings, bytes and lists.

The fused doors were the hard part. The emitter writes `append acc (slice …)`,
`utf8 (slice …)`, `to_int (slice …)` and `to_float (slice …)` as single calls
that read the range in place. A program can no longer write any of them: the
outer call has no `none` arm, so the checker refuses the pair. The JSON
decoder writes all four, and its ranges come from a scan of the same bytes,
so they never leave the value. The library now names those slices `builtin_slice_in`, a
standard-library-only spelling typed without `none`.
After the check it is renamed to `builtin_slice`, so every engine and every
fused door sees the same call it saw before. The doors keep their old
handling of a range past the end, because nothing outside the library can
reach them.

The first version wrapped each library slice in a helper with a `none` arm.
That defeated every door and cost jsonbench 860,566,126 -> 1,141,613,753
instructions here. With the measured spelling the decoder's own rows come
back. Checking the doors for a range past the end anyway cost runbench about
6 million instructions in `k_b_utf8_slice_raw`. It also moved register
allocation in the encoder's inlined `escape_onto`, so the checks went.

`bench/jsonbench` used to copy lib/json into a user module, and a user module
cannot name `builtin_slice_in`. `bench/make_jsonbench` now writes a program
that imports `std/json`. The two declines of 2026-09-27 and 2026-09-28 named
that as the change this shape of library needs. The row falls because the benchmark
now measures the decoder as `std/json` compiles it.

The standard library is not put through the exhaustiveness check that
refuses a program's unhandled `none`, so its slices were read by hand rather
than by the checker. Three in lib/regexp could leave the text: the body after a
pattern's flags, the flag letters themselves, and the one character a
`replace_all` steps over after an empty match at the end. The first raised
"chars takes a string" on `(?i`, which
`an_unclosed_pattern_answers_an_err` caught. The third printed the word `none`
into the replaced text, and nothing in the corpus reached it.
`an_empty_match_at_the_end_replaces_nothing_past_it` pins it now, red before
the fix. Each of the three reads "" where the range leaves the text, which is
what the clamp gave.

The library functions that arrived from main while this was built slice
only ranges they have measured: `starts_with?`, `ends_with?`, the two
recasing walks and the padding. They use `builtin_slice_in` too, so none of
them gains a `none` in what it answers.

Code that leaned on clamping now dispatches on `none`: lib/text's `trim`,
lib/path's `basename`, lib/regexp's character reads, the scripts, hako, the
book's ch08 positions sample and its panel, and the frozen decoders in
encodebench and widebench. Golden programs that sliced inside their range gained a `none` arm, and the
two that printed a range past the end now print `<none>`. The older
`slice_past_the_end` fixture, which pinned the clamp, is gone, since the
new one covers each of its ranges. The mem fixture `append_of_a_slice_boxes_nothing`
now reaches the fused append through `std/json`, since a program cannot spell
it. Its forty escape runs read allocs=12 and sh_bytes=48, and 54 and 1056 with
the fusion switched off. `a_cycle_that_allocates_nothing_needs_no_bracket`
copied its runs the same way; it now copies them a byte at a time and still
reads allocs=32 with no beat.

encodebench and widebench lost most of their cost back to one more change.
Their frozen decoders now ask whether a slice is none before they decode it,
so the decode is the unfused `utf8`, and only the fused door consulted the
token cache. A value of four to seven bytes now takes its string from the
cache on both doors. `a_short_decoded_slice_is_built_once` pins it: a word
decoded fifty times reads allocs=53 and sh_str=32 with the cache on this
door, and 103 and 1632 without. The cache had no counter of its own, but
every fill shows in perm_peak_bytes, which each cost golden carries.

#1943 then ruled an insisting `text/slice!` for callers that know their range
is inside, and the 2026-09-16 ruling makes it answer a box like every `!`
name. The decoder moves onto `text/slice! … .> text/utf8` and its siblings
once the emitter fuses those chains. Until then it keeps `builtin_slice_in`,
which no user program can name.

The rows below are CI's, except compile_peak_bytes and the browser rows,
which were read here on the rustc each golden names. Each is priced by putting
main's golden back and scoring again.

- runbench 1,178,202,929 -> 1,177,696,285 instructions, worth +0.0018.
  jsonbench reads 858,186,231 -> 830,048,931 because it now measures
  `std/json`; it is not a welfare term. encodebench rose 448,210 and
  widebench 176,756, the unfused `utf8` the frozen decoders now call.
- compile_instructions 31,940,603 -> 32,407,659 (-0.0006) and
  entry_instructions 101,349,935 -> 103,133,571 (-0.0025). The library grew:
  `none` arms in lib/text, lib/path and lib/regexp, and a second slice
  builtin to type. library_instructions moved 100,864,357 -> 102,649,522 for
  the same reason and is not a term.
- compile_allocs 17,941 -> 17,980 (-0.0002), compile_peak_bytes 789,911 ->
  795,133 (-0.0012). front_end_visits went 9,326 -> 9,225 and rounds held at
  19.
- interp_instructions 496,215,717 -> 495,119,692 (+0.0006) and
  interp_peak_bytes 792,646 -> 797,868 (-0.0010).
- release codegen 477,099,466 -> 476,979,642 (+0.0001). Dev codegen moved
  119,400,046 -> 119,402,816 and `emit_instructions` 7,622,247 -> 7,655,444,
  both under a ten-thousandth of a point.
- the tab: `browser_compile_instructions` 437,416,347 -> 439,552,195 and
  `browser_compile_peak_bytes` 726,939 -> 728,883, the same larger library
  compiled in wasm, together -0.0020.
- the machine-code golden fell by 2,800 to 6,896 bytes on every benchmark,
  Encode allocations rose 5,323,947 -> 5,332,649 and its arena took a fourth
  block, which arrived with the frozen decoder's unfused `utf8`; no
  allocation counter is a welfare term.

The counters below are not welfare terms, and each moved with the change:

- The encode vein, with the frozen decoder's unfused `utf8`: `encode_allocs`
  5,332,649, `encode_alloc_bytes` 665,662,512, `encode_sh_bytes` 101,072,232,
  `encode_arena_blocks` 4, `encode_arena_peak_bytes` 4,194,304 and
  `encode_spare_peak_bytes` 3,145,728.
- Instructions on the other benchmarks: `work_encodebench` 2,387,084,547,
  `work_widebench` 28,451,432, `work_basket` 30,958,481, `work_oneshot`
  13,207,833, `work_digestbench` 5,911,344, `work_indexbench` 2,462,589 and
  `work_scanbench` 273,717. The last five moved by under 0.05%.
- Rows that moved with the token cache, which now keeps some values of four
  to seven bytes that were viewed before: `wide_perm_live_bytes` and `wide_perm_peak_bytes`
  175, `wide_utf8_zerocopy` 15,993; `fresh_builder_perm_live_bytes`,
  `fresh_builder_perm_peak_bytes`, `fresh_cycle_perm_live_bytes`,
  `fresh_cycle_perm_peak_bytes`, `three_storage_regimes_share_one_append_perm_live_bytes`
  and `three_storage_regimes_share_one_append_perm_peak_bytes` 25 each;
  `fresh_builder_utf8_zerocopy` and `fresh_cycle_utf8_zerocopy` 39,
  `three_storage_regimes_share_one_append_utf8_zerocopy` 0, and
  `a_scan_keeps_its_place_in_the_text_seek_resumes` 407.
- The mem fixture that now reaches the fused append through `std/json`, which
  also runs the decoder: `append_of_a_slice_boxes_nothing_allocs` 12,
  `append_of_a_slice_boxes_nothing_alloc_bytes` 1,307,
  `append_of_a_slice_boxes_nothing_sh_str` 272,
  `append_of_a_slice_boxes_nothing_beat_iters` 40,
  `append_of_a_slice_boxes_nothing_find2_calls` 42,
  `append_of_a_slice_boxes_nothing_str_scans` 3,
  `append_of_a_slice_boxes_nothing_str_scan_bytes` 165,
  `append_of_a_slice_boxes_nothing_perm_allocs` 5,
  `append_of_a_slice_boxes_nothing_bytes_malloc` 3 and
  `append_of_a_slice_boxes_nothing_append_grow` 2.
- The emitted code: `emitted_lines` 10,165, `emitted_other_defines` 2,089 and
  `emitted_other_branches` 16,526.
- `interp_allocs` 592,317 and `codegen_instructions_dev` 119,402,816. The
  interpreter's peak and dev codegen are priced above.

Welfare goes 91.2447 -> 91.2395, a fall of 0.0052. This builds a ruled part
of the language, so the floor drops by exactly that.

## 2026-10-14 — src/runtime.c in seven parts

#1947. src/runtime.c had grown to 13,235 lines. It is now a list of seven
`#include` lines, and the parts live in src/runtime/: core.c (values, thunks,
regions, beats, carry), values.c (wide integers, failures, rendering,
equality), effects.c (boxes, io, processes, work-ahead), collections.c (lists,
maps, closures), text.c (utf-8, strings, bytes), numbers.c (number parsing,
bit operators, float tables) and main.c. Each part is a contiguous range of
the old file, cut at a seam the file already had, and the parts joined in
order are the old file byte for byte.

clang compiles src/runtime.c by path as before, and the includes resolve
beside it. The compiler embeds the runtime instead and writes it out when a
build needs it, so build.rs reads the include lines and joins the parts into
`kanso::RUNTIME_C`, which every Rust reader takes in place of
`include_str!("runtime.c")`. The digest, the cached object and the hot unit
see the bytes they saw before. build.rs refuses any line in the list that is
not a comment, a blank or a part, so clang and the compiler cannot be handed
different runtimes. `the_constant_is_the_digest_of_the_bytes_it_names` now
reads the parts from disk, joins them in the listed order and compares both
the text and the digest, which catches a part build.rs failed to watch.

121 mutations named src/runtime.c. Each was run on the old file, its
rewritten form was run on the parts, and the joined results were compared;
every one matched byte for byte. The two preserve_most sweeps patch several
parts, and each part they patch is spelled on a guard line. Three patch a line
of src/main.rs that read `include_str!("runtime.c")` and now reads
`kanso::RUNTIME_C`. The branch ratchet selects per part from here on: a branch
that touches text.c proves the rows whose mutations guard on text.c, where it
used to prove every row that guarded on the one file.

The other readers moved in the same change. Nine specs that cut function text
out of the runtime read `kanso::RUNTIME_C`. The utf-8 differential reads
src/runtime/text.c, which holds all five pieces it extracts, and passes with
45,589,025 checked. The asan sweep's `--prove` patches the joined parts; this
container has no sanitizer runtime, so the patch and its compile were checked
here and the sanitized run was not. build_runtime_wasm.sh compiles by path and
its output is byte-identical before and after, 84,137 bytes. Diagnostic
coverage descends into src/runtime/ and reads the same 310 diagnostics. Three
comments said the runtime was 450,100 bytes; it is 608,301, and they no longer
state a size.

One row moved, by layout: `entry_instructions` 103,133,571 -> 103,133,577
against 01161dbc, and 104,020,985 -> 104,020,991 once main carried
kanso#1962, six instructions both times. compile_instructions and
library_instructions held, and so did every runtime and machine-code row.

## 2026-10-14 — `text/slice!` answers a box

This builds the program-facing half of #1943's gavel, as the 2026-09-16 ruling
shapes it: `text/slice! s a b` answers a box holding the slice, or an err when
the range leaves the value. The err bubbles: `.>` passes it by, `.?` and a
foreign `rescue` reach it, and unhandled it stops the program at the endpoint.
An inverted range stays inside the value and is the empty slice, as it is for
`text/slice`.

It is written in lib/text over `slice`, the way `math/round!` is written over
`round`, so every engine runs the same definition and no builtin was added.
The definition sits at the end of the file because the runtime corpus quotes
lib/text's line numbers, and a definition near the top moved them.

`a_bang_slice_fails_inside_its_box` pins seven ranges on strings and lists on
the interpreter and both native tiers; with the range check taken out, the
out-of-range cases printed `<none>` and the program exited 0.
`a_slice_bang_past_the_end_reaches_the_endpoint` pins the unhandled failure.
Appendix B lists `text/slice!` beside `text/slice`.

A generator of forty random ranges per program, over strings with multi-byte
characters, lists and bytes, ran 150 programs through `text/slice` and 150
through `text/slice!`. The interpreter, a dev build and a release build printed
the same bytes for all 300.

The other owed item, moving the JSON decoder onto `slice! … .>` chains, is
#1960: a chain answers a box, so the move would make `json/decode` answer one.
Until it is answered the decoder keeps `builtin_slice_in`.

The rows move because lib/text grew a definition, and lib/text is compiled
into every program that imports it. CI on f50c9ec4 read:

- `compile_instructions` 32,407,659 -> 32,770,085, `entry_instructions`
  103,133,571 -> 104,020,985 and `library_instructions` 102,649,522 ->
  103,487,067.
- `compile_allocs` 17,980 -> 18,250, `compile_peak_bytes` 795,133 -> 804,635
  and `front_end_visits` 9,225 -> 9,242.
- `emit_instructions` 7,655,444 -> 7,704,002.
- `interp_instructions` 495,119,692 -> 496,124,589, `interp_allocs` 592,317
  -> 592,648 and `interp_peak_bytes` 797,868 -> 807,370.
- `browser_compile_instructions` 439,552,195 -> 441,096,528 and
  `browser_compile_peak_bytes` 728,883 -> 731,449. The browser's run rows did
  not move.

No runtime counter moved. Welfare goes 91.2395 -> 91.2300, a fall of 0.0095.
This builds a ruled part of the language, so the floor drops by exactly that.

## 2026-10-14 — `kanso test .` finds the module it is run inside

A test file imports the module beside it as `../<name>` (the 2026-10-12
gavel on #1883). Run as `kanso test .` from inside the module, that import
failed with "cannot resolve import \"../kanso-json\"", and kanso-json's CI
runs its tests that way. kanso-json main went red against kanso main from
the carrier that landed the gavel, #1938.

The resolver takes each leading `../` as a step up from the importing
file's directory, lexically. From `.` the step went to `Path::parent(".")`,
which is the empty path, so the import looked for `./kanso-json` inside the
module. A base with no name left to drop now gains a `..`. Nothing in the
corpus ran the test verb on `.`, and the one fixture that would have was
saved by a file named after its directory: `shared_name/shared_name.kso`
answers to the wrong path too.

`a_module_tested_from_inside_as_dot_imports_itself` in `tests/test_verb.rs`
runs `kanso test .` inside `tests/golden/test_verb/tested_as_dot`, where no
file shares the directory's name. It failed with the same error before the
fix. kanso-json's nineteen tests pass under the fixed compiler. The change is
in the resolver's path walk, so it moves no runtime row; the compile rows
are CI's to read.

CI moved one row: `browser_compile_instructions` 441,096,528 -> 441,097,975,
up 1,447, the same rise it read before main carried kanso#1962. The resolver
is compiled into the tab's engine, and the walk changed its code, so this is
layout in the wasm build. No other row moved. Welfare falls under a ten-
thousandth of a point and the floor is set to it.

## 2026-10-14 — An import reads only what the program names

A program that imported a std module lexed, parsed, inferred and checked all
of it (#1952). The first attempt, recorded on the issue, set unreached
functions aside for the merged check only, and its reach walk cost more than
the check it saved. This one works before the lexer: a shipped module's `fn`
declarations whose names no reached text spells are blanked out of the
source.

Reach is by word. The roots are every word in the program's own files and
five names the compiler calls without the program spelling them: `fold`,
`bump`, `file_under`, `to_string` and `tie`. Each declaration kept adds its
own words, over a module's files together, and a name kept keeps every arm
of it. A blanked line stays an empty line, so every span points where it
did. Blank-line policy and the unused-import check are skipped on a pruned
file; `tests/every_shipped_module_checks_clean.rs` still checks each module
whole.

When the pruned compile fails, the compile runs again on the whole library
and that run reports, so no diagnostic changes. A debug build panics when the
whole library accepts a program the pruned one refused, and the suite ran
clean under it. The one fixture that needed an unreached function,
`list/spread` in a codegen unit test, now names it in a comment.

`tests/an_import_reads_what_the_program_names.rs` compiles
`print (text/uppercase "kanso")`. On main the program held 48 declarations,
`text/pad_start` among them, and inference visited 627 expressions. It now
holds 12 and visits 224.

Rows, from CI: `compile_instructions` 32,770,085 -> 22,632,999,
`entry_instructions` 104,020,991 -> 74,989,410, `library_instructions`
103,487,067 -> 74,674,363, `emit_instructions` 7,704,002 -> 5,438,857,
`compile_allocs` 18,250 -> 12,160, `compile_peak_bytes` 804,635 -> 552,167,
`front_end_visits` 9,242 -> 6,363, `codegen_instructions_dev` 119,402,816 ->
112,581,257, `codegen_instructions_release` 476,979,642 -> 441,727,359,
`interp_instructions` 496,124,589 -> 492,626,313, `interp_allocs` 592,648 ->
584,659, `interp_peak_bytes` 807,370 -> 590,970,
`browser_compile_instructions` 441,097,975 -> 381,571,779,
`browser_compile_peak_bytes` 731,449 -> 643,478 and
`browser_run_instructions` 34,183,924 -> 34,181,240. The module sample in
`bench/compile_golden_modules.txt` reads visits 3,666 -> 1,368, rounds 8 ->
4 and lines 1,288 -> 1,106.

Six benchmarks emit less code, because the emitter still wrote some std code
nothing called: encodebench, widebench, deepbench and digestbench lose 157
lines each, scanbench 30 and runbench 32, and their machine code shrinks with
them. Three runtime rows move with the smaller binaries: `work_deepbench`
403,499,516 -> 403,115,516, `work_scanbench` 273,717 -> 273,812 and
`work_runbench` 1,177,696,285 -> 1,177,696,572. The two rises are 95 and 287 instructions on
programs whose work did not change, so they are layout. Every allocation
counter holds.

The ratchet's row "a door matched by the prefix of its name" proved nothing
on this branch, and it proves nothing on main either: with the mutation
applied, `micro_corpus_agrees_across_engines` stays green at dfc9fe2f. The
mutation hands the door's convention to `k_b_utf8_slice` and
`k_b_utf8_slice_raw`, and since a slice past the end answers `none` a program
reaches them only through the JSON decoder's `string_ok`, for a string with
no escape in it. Every decoded string in the micro corpus had an escape.
`a_plain_decoded_string_is_cut_in_place` decodes three plain strings; under
the mutation the release build prints `["" "" ""]` and the row goes red.

Lazy parsing by declaration, the larger fix #1952 names, has not been built. This
change narrows what is read; it does not make one declaration cheaper to
read.

## 2026-10-14 — An Apple release build keeps its wide arms through the link

kanso#1951 kept `tailcc` on arm64 functions whose arguments overflow the
eight argument registers, and marked them so the backend makes no sibling
call there (`wide_tailcc_keeps_its_jumps`). Linux arm64 ran correctly. An
Apple arm64 host did not: on the macOS job,
`a_spilling_tail_call_named_with_a_mark` printed nothing, though the same IR
ran under qemu for linux arm64. So Apple kept `narrow_tailcc`, which gives a
wide function the C convention. Each hop through one then spends a frame, and
the three tests that recurse deeply through a wide arm were gated off on
Apple: `a_self_call_through_a_thirteen_word_arm_runs_in_one_frame`,
`a_cycle_through_a_twelve_word_arm_runs_in_one_frame` and
`an_object_of_300_000_keys_decodes` (#1912).

The cause is ld64's LTO. It runs LLVM's default pipeline over the linked
module, and that pipeline includes dead-argument elimination. Once LTO
internalizes a wide `tailcc` function, the pass is free to drop the arguments
it never reads. On the macOS job it cut `reached?_5_w` in the mark test to one
argument and a plain `ret`, while its caller still ran `sub sp, sp, #0x10`
after the call for the stack arguments it expected the callee to pop. The stack
pointer came back sixteen bytes off. The Linux link takes the pass out already
(`LTO_O3_WITHOUT_DEADARGELIM`, since kanso#1705), which is why the same IR was
fine there.

A function named in `@llvm.used` is never internalized, so the pass leaves its
signature alone. That custom pipeline applies only when clang is 18 or 19 and
lld takes the link, so every arm64 release build now marks its wide functions as
before and also lists each `tailcc` definition with more argument words than
the registers hold in `@llvm.used`, in the `llvm.metadata` section. A program
with no such function gets no global. Two unit tests in `tail_calls.rs` pin the
list: only a function past the registers is named, and a program without one
names none. The three tests above run ungated, and on the macOS job all three
pass.

`narrow_tailcc` and its helper `called_symbol` are gone, and the
tail-call-given-up mutation now strips both host arms. No row moved. The
change is confined to the arm64 host arm, and every Linux row the branch's CI
read matched main's goldens, `compile_instructions` included.

## 2026-10-14 — Work bound ahead of the first sleep moves into it

The 05:25 comment on #1856 measured a gap in the first cut of work-ahead. A
program that binds its work as a plain value before the sleep,

```
total = spin 150000000 0
sleep 2000 .> (_ -> print "delivery: made") .> (_ -> print "report: {total}")
```

took 3.01 s against 2.01 s for the rig, which binds the same work as a chain.
An entry binding is computed at its own statement, so the work ran before the
sleep began and the pool had nothing left to force.

`src/ahead.rs` now moves such a binding into the park. The move applies when
every statement ahead of the first sleep is a binding, since a binding performs
no effect and the sleep is then the first thing the program does that can be
seen. A binding qualifies when its value is closed (a literal, or a call of the
program's own functions on closed arguments) and no other binder in the entry
uses its name. It becomes a constant named `ahead~moved~N`, its reads in the
statements after it read the constant, and the pool forces these constants
first, in the order they were written. A read of a moved binding counts as
closed, so a later binding built from one moves too, and so does a call after
the sleep whose arguments name only moved bindings: `spin n 0`, with
`n = 150000000` bound ahead of the sleep, is lifted like `spin 150000000 0`.

The 2026-10-11 gavel on #1862 says a pooled failure is raised where the
program as written raises it. For a moved binding that place is the park,
because nothing printed between the binding and the sleep. The pool reads a
moved constant through `k_ahead_try` like any other; when the try reports a
death, the pool reads it once more outside the try, so the failure is raised
there with its message on stderr. The work is pure, so the second read meets
the same failure. A constant lifted from a continuation keeps its old
treatment: the pool stops and the failure comes where the value is read.

On this container the program above went from 3.80 s to 2.08 s under
`kanso play`, compile included, with the same two lines printed; the `spin n 0`
form went from 4.04 s to 2.09 s.
`tests/golden/mem/work_ahead_moves_a_binding_into_the_park` pins
`ahead_forced=1`, `tests/golden/mem/work_ahead_reads_a_moved_binding` pins
`ahead_forced=2` for the moved count and the call that reads it, and `a_moved_binding_fails_at_the_park` in
`tests/work_ahead.rs` pins a moved binding that overflows the stack: nothing on
stdout and the overflow on stderr, the interpreter's transcript. Without the
second read the native build printed `first` before the overflow. Three ratchet
rows cover the move, the second read, and a moved binding read as closed.

Two cleanups ride along. The mutable child walk was written twice, in
`src/inline.rs` and `src/lib/hoist.rs`; it is now `for_each_child_mut` in
lib.rs beside `for_each_child`. Taking its callback as `&mut dyn FnMut`, as
hoist's copy did, cost the tab's compile of its corpus 2,229,038 more wasm
instructions on this container than the two private copies, because the copy
in inline.rs had been inlined into its two callers and devirtualized there.
The shared walk takes its callback as a type parameter instead, as
`for_each_child` does. A pattern's binders were listed twice, by
codegen's `pattern_names` and trmc's `bound_names`; both call
`Pattern::each_name` now, and trmc's list gains the as-pattern's name it used
to leave out.

CI moved six rows. `browser_compile_instructions` went from 381,571,779 to
382,410,206, up 838,427, the same reading this container gave. A bisect here
put 305,579 of that on the shared walk, 32,693 on `each_name` and 500,155 on
the move, of which the move's codegen change accounts for 3,436. The tab's
corpus never sleeps, so most of the rest is how rustc divided the wasm build
into codegen units. The native front end rose with it: `entry_instructions`
74,989,410 -> 75,110,529, `library_instructions` 74,674,363 -> 74,797,007,
`compile_instructions` 22,632,999 -> 22,665,231. `interp_instructions` fell
756 to 492,625,557 and `emit_instructions` fell 9 to 5,438,848. No runtime row
moved. Welfare fell 0.00094 to 91.52438, and the floor is lowered by exactly
that, since this builds a ruled part of the language.

Still not built for #1856: an entry with more than one effect line, which
never pools (the next entry corrects this line), calls whose arguments name
locals that cannot move (a lambda's parameter, a value read from io), which
need the (site, args) pool, and the cost gate on admission.

## 2026-10-14 — The pool takes only work

This is the grain piece of #1856. The pool reads each constant inside
`k_ahead_try`, whose `sigsetjmp` saves the signal mask with a system call. A
literal bound ahead of the first sleep moved into a constant like any closed
binding, so the pool made that call to build a literal. `ahead::admits` now
lets a constant into the pool only when its value calls one of the
program's functions, directly or inside a string or list it builds. That is
the test the lazy cost gate puts to a cell (`demand::expensive`). The literal
still moves, because a call lifted after the sleep may name it, and it is
built where it is first read.

`tests/golden/mem/work_ahead_leaves_a_literal_where_it_is` binds a literal
and a call ahead of the sleep, and `ahead_forced` reads 1 where main read 2.
`work_ahead_reads_a_moved_binding` goes from 2 to 1 for the same reason. The
ratchet row "a literal handed to the pool" admits every constant and turns
the first golden red.

The previous entry listed "work bound after a statement with an effect" as
unbuilt. No program can be written that way. The parser refuses a binding
after an effect line, joins a body's effect lines into one group, and runs
every binding before the group starts. The runtime runs the pool only at a
sleep outside a group, so an entry with two effect lines never pools at all.
`tests/golden/workahead/a_binding_before_a_print_fails_before_it` pins the
transcript of such an entry whose binding fails: nothing on stdout, the
overflow on stderr. A ratchet mutation that let the park be any sleep was
tried and left the fixture green, because the group hides the sleep from
`ahead` before the search runs, so that row was dropped.

CI moved five rows, all of them compile cost. `browser_compile_instructions`
fell 255,236 to 382,154,970, the same reading this container gave. The tab's
corpus never sleeps and the tab compiles the pool out, so the pool decides
nothing there; nothing in this change isolates the drop further.
`compile_instructions` fell 15 to 22,665,216, `entry_instructions` 25 to
75,110,504 and `library_instructions` 25 to 74,796,982. `emit_instructions`
rose 72 to 5,438,920; the emitter now passes the program's declarations
through `admits`, the one change on that path. No runtime
row moved. Welfare rose 0.00022 to 91.52460, and the floor is banked there.

Still not built for #1856: pooling inside an effect group, and calls whose
arguments name locals that cannot move, which need the (site, args) pool.

## 2026-10-14 — A group of effect lines pools while it waits

#1856's work-ahead pool ran only at a bare sleep, and a body with two or more
effect lines has none: the parser joins the lines into one group, and the
scheduler runs a sleep inside it by parking that side. So `print "start"`
followed by a sleep with work behind it never pooled.

The scheduler now runs the pool at the group's first idle point, the first
time every side is waiting on a timer, and the wall-credit it already keeps
counts the pool's time against that wait. Which work runs there is fixed at
compile time and the idle point is a fact of the schedule, never of the
clock, so the transcript and the counters are the same on every run. On the
compiler side, `ahead` reads each side of the group that begins with a sleep
the way it reads a lone statement. A failure in the pool is dropped and
raised where the value is read, as before, so a program in a group prints
the transcript it prints without the pool. Bindings still move only when the
park is the body's single effect line.

`print "start"` and a 2-second sleep followed by a report of `spin 150000000
0`, run with `kanso play` on this container: 3.80 s and 3.96 s on main, 2.08 s
and 2.01 s on this branch, with the same output.

Pinned by:
- `tests/golden/mem/work_ahead_forces_inside_a_group`: `ahead_forced=1`, where
  main printed no work-ahead counters at all;
- `tests/golden/workahead/a_pooled_overflow_in_a_group_fails_where_it_is_read`:
  "start", "first", then the overflow, on both engines;
- ratchet rows "a group that never pools" and "a group's side left unread",
  each seen red on the new golden.

`k_schedule` grew 16 bytes on CI, and every benchmark in
`bench/text_golden.txt` grew by the same 16, so `text` went from 4,878,382 to
4,878,606. `browser_compile_instructions`
went from 382,154,970 to 382,155,032, though the tab compiles the pool out.
The front end rose: `compile_instructions` 22,665,216 -> 22,679,084,
`entry_instructions` 75,110,504 -> 75,152,899, `library_instructions`
74,796,982 -> 74,839,906, `emit_instructions` 5,438,920 -> 5,440,488 and
`interp_instructions` 492,625,557 -> 492,635,997. Codegen rose 967 at dev,
`codegen_instructions_dev` 112,581,257 -> 112,582,224, and 1,390 at release,
`codegen_instructions_release` 441,727,359 -> 441,728,749.

Three of the four CI runs landed on Intel parts, and the fourth on the AMD
part `bench/instructions_golden.txt` is recorded on. It moved four work rows:
`work_encodebench` 2,387,084,547 -> 2,387,083,945, `work_oneshot` 13,207,833
-> 13,207,567, `work_livebench` 1,474,835,144 -> 1,474,835,683 and
`work_runbench` 1,177,696,572 -> 1,177,697,090. This container, an Intel part, read the same
four differences between main and this branch built from the same inputs.
No benchmark sleeps, so no pooled work runs in them, and nothing here
isolates where the difference comes from. Welfare fell 0.00009 to 91.52451,
and the floor is lowered by exactly that, since this builds a ruled part of
#1856.

## 2026-10-14 — A call on a function's locals is pooled as a cell

#1856's pool took only constants: calls whose arguments name nothing local,
lifted out of the entry. A function that sleeps and then reports on its own
parameter had no constant to offer, so its work ran after the wait:

```
fn report n
  sleep 2000
    .> (_ -> print "delivery: made")
    .> (_ -> print "report: {spin (150000000 + n) 0}")
```

This is the "Discovery" piece of the ruling, the (site, args) pool. A
function whose result is a chain beginning with a sleep, and which is the
only arm of its name and count, has each certain call on its locals bound
ahead of the chain as a cell. `emit_cell` already builds a cell from a site
and the values it captures; `ahead::pool` names the binding, `demand` makes
it a cell whatever its uses, and the chain's sleep carries the cells in its
descriptor. The runtime queues them when that sleep parks and forces them, in
order, under the same try as the constants. A chain that is never run parks
nowhere, so its cells are made and never forced, and no work runs early that
the program would not have run. The interpreter does not pool.

A cell the pool forces is not a lazy binding the program has, and the thunk
counters are what both engines must agree on, so `k_thunk_new_pooled` builds
it with a flag those counters skip. `ahead_forced` counts it instead.

The program above, run with `kanso play` on this container: 3.87 to 4.07 s on
main, 2.01 s on this branch, with the same output. Built and run directly,
the dev binary takes 3.95 s against 2.00 s, and the release binary 2.20 s
against 2.00 s.

Pinned by:
- `tests/golden/mem/work_ahead_pools_a_call_on_a_local`: `ahead_forced=1`,
  and the thunk counters at zero on both engines;
- `tests/golden/mem/work_ahead_leaves_a_chain_never_run`: a chain built and
  never run, `ahead_forced=0`;
- `tests/golden/workahead/a_pooled_cell_fails_where_it_is_read`: the
  overflow is raised after "first", on both engines;
- five ratchet rows, each seen red.

`a_group_that_never_pools.sh` anchored on a comment and a call this change
rewrote, so it is rewritten to the new ones; every other mutation still
applies.

Still not built for #1856: a lambda's body, which has no statements to bind a
cell in; a function with more than one arm of its name and count; and the cost
gate's view of a pooled call, which admits any call of the program's own.

CI's rows for the branch. The runtime rows were read before main's IR-scan
move was merged in, and the compiler's own rows after kanso#1976 was. The
runtime grew by the queue and its drain, so every benchmark's machine code
rose by 2,192 bytes: jsonbench text=336,265, encodebench text=345,129,
oneshot text=351,145, basket text=313,289, widebench text=354,393,
deepbench text=288,025, escapebench text=279,113, pendbench text=296,841,
indexbench text=277,625, scanbench text=462,073, digestbench text=318,537,
readbench text=279,273, livebench text=354,889, runbench text=652,697, and
the `text` total 4,878,606 -> 4,909,294. Work moved on five:
`work_encodebench` 2,387,083,945 -> 2,387,084,099, `work_oneshot` 13,207,567
-> 13,207,693, `work_pendbench` 191,674,024 -> 191,674,224, `work_livebench`
1,474,835,683 -> 1,474,835,606 and `work_runbench` 1,177,697,090 ->
1,177,696,062.
The compiler's own rows: `compile_instructions` 22,664,963 -> 22,595,494,
`entry_instructions` 75,111,766 -> 74,872,489, `library_instructions`
74,798,192 -> 74,559,617, `emit_instructions` 5,439,036 -> 5,449,223,
`interp_instructions` 492,629,961 -> 492,636,904,
`codegen_instructions_dev` 112,582,224 -> 112,677,448 and
`codegen_instructions_release` 441,728,749 -> 441,885,670. The tab, read
here on rustc 1.98.1 after the merge: `browser_compile_instructions`
382,297,788 -> 381,261,098, `browser_compile_peak_bytes` 643,478 -> 643,605
and `browser_run_instructions` 34,181,240 -> 34,181,355. Welfare rises, and
the rise is banked here.

## 2026-10-14 — Neither verb sends a reader to the other

The 2026-10-14 audit of #1893 found one file shape that both running verbs
refuse while pointing at each other. A file that declares a function, runs a
statement and imports a neighbour is a library to `kanso run`, which
suggested `kanso play`; `kanso play` takes the stdlib and nothing else, and
suggested `kanso run`. Both refusals now give the route that runs the file:
move the statements into a main.kso beside it, mark the definitions they use
`pub`, and run the directory. A play file of statements alone that imports a
neighbour is still sent to `kanso run`, which takes it.

`tests/golden/play/beside/sum.kso` is the reduction. Its `.stderr` and
`.run.stderr` pin what each verb says, and
`definitions_beside_a_local_import_are_sent_to_a_module` in
`tests/play_verb.rs` builds the module the messages describe and runs it.

No test read the `.stderr` files in `tests/golden/play`. The archive noted
this for one of them on 2026-10-10. `every_play_golden_is_read` now reads
every golden in that directory: each `.stdout` on both engines, each
`.stderr` under `play`, and each `.run.stderr` under `run`. Two goldens
were stale. `no_local_imports.stderr` spelled the import without its
`./`. `decls_after_statements.stderr` held a refusal the play parser
dropped when it began sorting a file's lines by kind; that fixture now
prints `first`, so its golden is a `.stdout`.

The audit's other findings on #1893 are open there. An insisting form's
failure names the library line instead of the call, and neither a wrong
kind reaching a builtin or operator nor a stack overflow has a location.

CI's rows for this branch fall, and the change reaches no decision they
count, so the move is the binary's layout: `compile_instructions`
22,679,084 -> 22,664,963, `entry_instructions` 75,152,899 -> 75,111,766,
`library_instructions` 74,839,906 -> 74,798,192, `emit_instructions`
5,440,488 -> 5,439,036, and `interp_instructions` 492,635,997 ->
492,629,961. The tab's compile rises: `browser_compile_instructions`
382,155,032 -> 382,297,788, on rustc 1.98.1. The arms this change adds run
only when a file is refused, and no file in the corpus is. Marking
`local_import` cold moved the row by 1,529, so the rise is not that
function. The browser rise outweighs the five falls by less than 0.0001 of
welfare, and the floor moves to the score with the reason recorded, as it
did for kanso#1959 and kanso#1963.

## 2026-10-14 — The work-ahead rig builds again

The compiler page tells a reader to build `bench/workahead/aswritten.kso` and
`overlapped.kso`. Each file declares `grind` and `spin` beside its statements,
and since files of that shape became libraries, `kanso build` refuses both.
Nothing else reads the rig, so nothing went red. The two programs are now
modules, `bench/workahead/aswritten` and `bench/workahead/overlapped`, and
both import the functions from `bench/workahead/rig`.

The panel's rows are re-sat: 15 interleaved heats each on the build
container. As-written reads a minimum of 2,004 ms where the panel said
2,913, because the pool now builds `grind 150000000` during the sleep.
Overlapped reads 2,004 ms where the panel said 2,008. The two differ in their
output: overlapped prints the report before the delivery, and as-written
keeps the order it was written in.

## 2026-10-14 — A pooled call may read its arguments through a builtin

The pool from kanso#1978 took a call on a function's locals only when every
argument was a local, a literal, an operator over those, or a call of the
program's own. A call that measured its argument first was left after the
wait:

```
os/args .> (a -> wait a)

fn wait a
  sleep 2000
    .> (_ -> print "delivery: made")
    .> (_ -> print "report: {spin (150000000 + length a) 0}")
```

`length a` is a builtin, so `spin` ran after the sleep. With the length
taken before `wait` is called, the same program already pooled: 2.01 s
against 4.38 s on main.

A pooled call's arguments may now apply a builtin that reads a value and
answers a new one: `length`, `keys`, `values`, `entries`, `chars`, `bytes`,
`char_code`, `from_code`, `to_int`, `to_float`, `round`, `sqrt`, `sum` and
the six `bit_` words. The words that extend a value stay out. The beat pass
may write the result of `push`, `put`, `concat` and the rest into the
storage of the value they were given, and a pooled cell runs before the
program's own reads of that value.

The program above, run with `kanso play` on this container: 4.07 to 4.57 s
on main, 4.01 to 4.21 s on kanso#1978's branch, and 2.01 to 2.08 s on this
one, with the same output.

Pinned by:
- `tests/golden/mem/work_ahead_pools_a_call_on_a_builtin_read`:
  `ahead_forced=1`;
- `tests/golden/mem/work_ahead_leaves_an_extending_builtin`: a `push` inside
  the argument, `ahead_forced=0`;
- two ratchet rows, each seen red on its golden.

## 2026-10-14 — A lambda whose body parks first is pooled like a function

A lambda has no statements to bind a cell in, so the pool never reached one:

```
k = 150000000
os/args .> (a -> sleep 2000 .> (_ -> print "r: {spin (k + length a) 0}"))
```

`pool` now lifts such a lambda into a function of its own, `ahead~lam~N`,
whose parameters are the locals the body reads (`k` here) followed by the
lambda's own. The lambda stays where it was and calls the function. The
lift is kept only when the function pools something; a lambda with nothing
to pool keeps its body, since the call would buy nothing. A body that holds
an `if` block or a guard is left alone, because the walk does not follow
the names those bind.

The program above, with `kanso play` on this container: 3.71 to 4.09 s on
main, 2.02 to 2.09 s on this branch, with the same output.

Pinned by `tests/golden/mem/work_ahead_lifts_a_parking_lambda`
(`ahead_forced=1`) and the ratchet row "a parking lambda never lifted",
seen red on that golden.

The walk for a parking lambda runs only when the program reaches
`time/sleep`, the one function that parks; a program that never calls it
has had it pruned. Without that guard the walk cost the tab 366,863
instructions on a corpus with no sleep in it.

Rows against kanso#1978, the browser row read here on rustc 1.98.1 and the
rest by the compile sweep on this container, where `compile_instructions`
read what CI read for the unguarded branch: `emit_instructions` 5,449,223 ->
5,452,431 for the guard and its check. `browser_compile_instructions`
381,261,098 -> 382,590,239. Of that rise, 1,320,831 stays with the walk
switched off entirely, and the builtin-read commit alone leaves the row
where kanso#1978 put it, so it arrives with the commit that adds the lift
while none of its code runs; marking `pool` and `lift` never-inline did not
move it. The front end's rows rose with the binary, since `kanso check`
never reaches the pass: `compile_instructions` 22,595,494 -> 22,699,901,
`entry_instructions` 74,872,489 -> 75,214,425 and `library_instructions`
74,559,617 -> 74,901,006; `interp_instructions` fell with it, 492,636,904 ->
492,620,187. The runtime and machine-code rows did not move. Welfare falls,
and the floor moves by that amount under #1856's ruling, as it did for the
group pool in kanso#1972.

A pool-shaped differential ran 400 generated programs, each with a sleep
whose continuations read locals, lambda parameters and builtins of them,
some inside a function and some in nested lambdas. The interpreter and the
native build agreed on all 399 the front end accepted; 85 of the first 100
forced at least one pooled call.

## 2026-10-15 — gavel: a record field carries no type, and kanso has no type parameters

Clay ruled #1946. Two July rulings could not both stand. On 2026-07-25 a type
could declare a parameter (`type <k>pair` with fields `first:k` and
`second:k`), and on 2026-07-26 a written field type became an error. The
parameter means nothing unless a field can write `:k`.

**The ruling.** No field carries a written type of any kind, and kanso has no
type parameters. The archive's "GAVEL (amendment): the binder stays" and the
"CORRECTION" after it are reversed. The reason is the one the 07-26 ruling
gave for concrete types. kanso compiles the whole program, so the compiler
sees every place two fields' values meet. If something needs them to agree,
a disagreement is refused at that use. If nothing needs them to agree, no
program depends on it. Declaring the agreement only moves where the error is
reported. Relating types was already gone from function signatures for the
same reason, and now it is gone from type declarations too.

Owes: #1941 closed with a pointer here; `sorted` stays a plain type, and the
#1903 entry's mention of `<k>sorted` no longer describes anything to build.

## 2026-10-15 — gavel: a span is a range a check has proven inside its string, and slicing one cannot miss

Clay ruled #1960. `text/slice!` answers a box (2026-09-16), and a box spreads
to every caller, so the JSON decoder written with it would make `json/decode`
answer a box. The insisting form could not give a caller that knows its
bounds a plain string. Proving ranges by inference was weighed and set aside:
a reader cannot see whether a slice is proven, and an edit to the scan
breaks the proof far from where the error lands.

**The ruling.** A span is a range of positions inside one particular string,
and only a function that has checked the bounds makes one, the way only
`list/to_sorted` makes a `sorted`. `text/slice` on a span answers the string,
with no `none`, because the range cannot leave it. A slice from bare numbers
answers `none` past the end, as ruled on #1890, and `text/slice!` insists
with a box, as ruled on #1943. The word is span, after C++'s `std::span` and
the source ranges compilers carry: a value holding two positions into a
string that already exists, holding none of its characters.

Owes, on #1960: the span value in std/text, built only by bounds-checked
functions; the scanning functions the decoder uses (`find2` and its
siblings) answering spans or `none`; `text/slice` arms for a span; the
fused doors (`utf8`, `append`, `to_int`, `to_float`) on a sliced span; the
decoder moved off `builtin_slice_in` onto spans; micro goldens on every
engine. The name of each new public function goes to Clay as one question
before it ships.

## 2026-10-15 — gavel: a run-time failure names the line it happened on

Clay ruled #1975. All twenty ports met failures that named no file or line:
a wrong kind reaching an operator or builtin (`` `*` is not defined for these
values ``, `write takes a string`) and a stack overflow. The interpreter has
the span and drops it, because native has nothing to print beside it and the
engines must agree byte for byte.

**The ruling.** Every run-time failure names the file and line it happened
on, on every engine. Compiled code records a constant `file:line` before each
runtime call that can stop the program, and the runtime prints it when it
dies; the interpreter prints the span it already holds, and the tab shares
the C runtime. An insisting form names its caller's line rather than the
library's. The cost is one store per such call, measured at no more than
0.42% of runbench before the build; Clay accepted it, and the floor drops by
what CI measures, recorded with this entry as the reason. A refusal at check
where the wrong kind is certain costs nothing at run time and goes in beside
it.

Owes, on #1975: the stamp in codegen and the runtime's print; the
interpreter printing its span; insisting forms stamping the caller; error
goldens on every engine for an operator, a builtin and a stack overflow;
CI's rows and the floor moved by them.

## 2026-10-15 — An operator handed a literal string, list or map is refused at check

The first piece of #1975, and the one that costs nothing at run time. `+`,
`-`, `*`, `/` and `%` take numbers, and an operator arm routes only a record,
so a string, a list or a map stops the run on every engine. The check now
refuses one at the operand, in `wrong_kind_at_an_operator` in
`src/check/after_infer.rs`, beside the `list/concat` refusal.

It refuses only an operand that cannot hold an err: a literal with nothing
interpolated, or a call of such literals into a group whose every arm answers
a string, a list or a map and none fails. The first draft read the callee's
answer set alone, and `tests/golden/micro/a_merged_err_stays_flat_across_a_hop`
caught it: `through merged + e3` calls a group whose arms all answer strings,
but `through` has no err arm, so the err it is handed hops through it and the
two errs merge at `+`. An operator hands an err on, so anything that might
carry one is left to the run.

That leaves the ports' case, `twice "x"` where `twice n` is `n * 2`, to the
run, which is what the rest of #1975 locates. Pinned by
`tests/golden/errors/an_operator_handed_a_value_it_never_takes`, seen red with
the call removed; the ratchet row `operator_kind_unchecked` runs the same
mutation.

The walk costs compile time, which CI measured: `compile_instructions`
22,699,901 -> 22,715,476, `entry_instructions` 75,214,425 -> 75,262,121,
`library_instructions` 74,901,006 -> 74,948,228 and
`browser_compile_instructions` 382,590,239 -> 382,741,447. The run rows did
not move. This builds a ruled part of the language, so the welfare floor
moved down by exactly that cost, to 91.52, with this entry as the reason.

## 2026-10-17 — gavel: every run-time failure names its line except a stack overflow

Clay ruled #1987. The #1975 ruling accepted naming every run-time failure's
line at a bound of 0.42% of runbench. Built in kanso#1985, CI measured
+1.70% on runbench, +3.01% on livebench and +4.36% on encodebench. The
bound had counted only calls into the C runtime. It missed the stores on
inlined fast paths, which moved onto the slow branch, the stamp before a call
that can match no arm, and the store on entry to every function of a
recursion cycle that records an overflow's place. The overflow's place also
broke the differential law: an engine names the cycle it entered last, and
the engines run out of stack at different depths, so a program that keeps
entering a second cycle gets two different answers.

**The ruling.** Every run-time failure names its file and line on every
engine, except a stack overflow, which keeps its sentence with no line, as
main prints it today. A call that matches no arm names the caller's line.
Dropping the overflow's entry stores saves about 0.8% of runbench and 1.5%
of livebench and encodebench, and the divergence goes with them. Naming an
overflow's place by the outermost active cycle is possible and is its own
issue, measured from scratch; nothing in this ruling waits on it. The floor
drops by what CI measures for the rest, under the #1975 gavel.

Owes, on #1987: kanso#1985 without the overflow's entry stores; CI's rows
and the floor moved by them; a follow-up issue for the outermost-cycle rule.

## 2026-10-17 — gavel: the names for spans

Clay ruled #1983, the names the #1960 gavel said would come to him before
any shipped.

**The ruling.** In std/text:

- `span` is the type, with readable fields `whole` (the string), `from` and
  `to` (inclusive, the way `text/slice` counts). An empty run has `to` one
  less than `from`. Only std/text builds one.
- `text/span s from to` answers a span when the range lies inside `s`, and
  `none` otherwise.
- `text/span_to cs p a b` answers the span from `p` up to the byte before the
  first `a` or `b`, or to the end of `cs`, and `none` only when `p` lies
  outside `cs`. `text/span_to_below cs p a b lim` also stops at any byte
  below `lim`. The names say what they answer. `find2` and `find2_below`
  keep answering a position.
- `text/slice run`, an arm of `slice` that takes a span, answers the string
  with no `none`.

Owes, on #1983 and #1960: the five names on every engine; the fused doors
reading a sliced span; the decoder's four `builtin_slice_in` sites moved
onto spans; micro goldens; appendix B; kq's decoder following.

## 2026-10-18 — A run-time failure names the line it happened on

The rest of #1975. Every run-time failure but a stack overflow now prints
its place under its message, on every engine, in the form an err's `born in`
line uses:

    error[runtime]: `*` is not defined for these values
      failed in calc/twice at calc.kso:2

**The interpreter** fills the place in as a failure leaves the body that
raised it. Every expression in a body shares the body's frame, so the frame
changes on the way out only where a declaration's body, a lambda's body or a
lazy binding is run, and each of those places a failure that has no place
yet. Three paths raise a failure outside any expression, and each places it
itself: a statement's destructure, a tail call whose dispatch matches no arm,
and a tail call through a function value that takes the wrong count. The
first version placed at every expression, which cost the interpreter row
about 3.7M of the 15M it rose.

**Compiled code** stores the same literal an err born at the site would
carry into `k_site` before each call that can stop the program, and
`k_died` prints it. The stamped calls are the operators' slow paths, builtin
calls, condition tests whose value is not provably a boolean, field and index
reads, destructures, keyed reads, joins, calls through a function value, and
calls of a group that can match no arm. A store to `k_site` is exempt from
`writes_state`, since running a body over repeats it harmlessly.

Two things keep the store off the hot path. An inline twin (`length`, `at`,
`append`, the bit operations, `push`, `put`, `slice`, the scans) takes the
site as its last argument and stores it only on the branch that reaches the
C entry, which is the only branch that can stop the program; `takes_site`
reads which twins do from their definitions in `declares.rs`. With the store
in front of every inlined twin, CI read runbench +6.4% and digestbench +18%.
And a call into a group is stamped only when the arguments' proven kinds
might fail every arm: the arms are read together, so `f x true` and
`f x false` take a proven boolean, an annotation the dispatcher tests by kind
alone takes that kind, and a by-value record is taken by the arm that
destructures its type.

**A stack overflow** names no place. The engines run out of stack at
different depths, so no frame is the one the overflow happened in. The first
version named the cycle the program last entered, which was wrong in a way
no fixture showed: a recursion that enters a second recursion at every level
ran out inside `small` on native and on a call to `big` in the interpreter,
and the two printed different places. A rule that holds still (the outermost
active cycle names it) needs the record cleared when the entering call
returns, and a library wrapper that tail-calls into a recursion has nowhere
to clear it. The overflow keeps its sentence alone, under "2026-10-17 —
gavel: every run-time failure names its line except a stack overflow".
`a_recursion_inside_a_recursion_overflows_alike` pins the case.

**An insisting form** names the line that called it. A form whose name
ends in `!` turns an err into a failure, and the err was born inside the
library, so `os/read_file!` named `os/insisted at std/os/os.kso:113`. Each
call of such a form now hands its answer back through one step that points
an err born in the form's own package at the caller's line; an err born
anywhere else passes through unchanged. Native does it in
`k_b_insisted`, and an answer that is still an effect gets the step wrapped
around it so the err it later yields is pointed too. The interpreter does
the same with `restamped`, and with a `Desc::Insisted` node for an effect.

A lazy binding that demands its own value still names no place. The
interpreter has no source position for it, and native clears `k_site`
before saying so, so the two agree on saying nothing.

Pinned by the runtime corpus, which native, the interpreter and the tab
now print byte for byte, and two new fixtures:
`an_operator_through_a_parameter_names_its_line` (the ports' case) and
`a_recursion_inside_a_recursion_overflows_alike`. `read_missing_file` and
`a_slice_bang_past_the_end_reaches_the_endpoint` moved to name the caller.
The ratchet rows `death_unplaced`, `oracle_unplaced`, `insisted_native`
and `insisted_oracle` each take one piece away and watch the corpus go red.

**Rows**, CI's against main. The run time, from the stamps that remain and
the slow-branch site arguments: `work_basket` 30,958,481 -> 31,158,162,
`work_deepbench` 403,115,516 -> 407,387,521, `work_digestbench` 5,911,344 ->
5,992,785, `work_encodebench` 2,387,084,099 -> 2,464,675,698,
`work_escapebench` 39,058,340 -> 39,058,403, `work_indexbench` 2,462,589 ->
2,482,608, `work_jsonbench` 830,048,931 -> 832,975,500, `work_livebench`
1,474,835,606 -> 1,498,216,646, `work_oneshot` 13,207,693 -> 13,286,600,
`work_pendbench` 191,674,224 -> 194,087,911, `work_readbench` 4,580,963 ->
4,582,588, `work_runbench` 1,177,696,062 -> 1,189,193,317, `work_scanbench`
273,812 -> 274,086, `work_widebench` 28,451,432 -> 28,628,235.

The interpreter, placing at the body: `interp_instructions` 492,620,187 ->
500,108,350. The emitter and the code it writes: `emit_instructions`
5,452,431 -> 6,061,105, `codegen_instructions_dev` 112,677,448 ->
115,010,801, `codegen_instructions_release` 441,885,670 -> 449,091,786,
`emitted_calls` 1,023 -> 1,024, `emitted_lines` 10,165 -> 10,362,
`emitted_other_calls` 17,641 -> 17,648, `emitted_other_lines` 145,594 ->
149,547, `text` 4,909,294 -> 5,011,246, `lines` 1,356 -> 1,368,
`module_lines` 1,106 -> 1,130. The front end fell with the binary's layout:
`compile_instructions` 22,721,761 -> 22,693,337, `entry_instructions`
75,287,373 -> 75,197,395, `library_instructions` 74,973,469 -> 74,884,283.
The tab: `browser_compile_instructions` 382,767,025 -> 389,899,281,
`browser_compile_peak_bytes` 643,605 -> 675,444, `browser_run_instructions`
34,181,355 -> 35,216,926.

Every benchmark reads its input through `os/read_file!`, whose effect now
comes back inside a description holding the caller's line, so each allocates
3 more and 96 bytes more and evacuates the same: `run_alloc_bytes`
300,934,796 -> 300,934,892, `run_allocs` 1,812,043 -> 1,812,046,
`run_evac_allocs` 49,221 -> 49,224, `run_evac_bytes` 14,917,984 ->
14,918,080, `alloc_bytes` 175,182,800 -> 175,182,896, `allocs` 1,390,963 ->
1,390,966, `evac_allocs` 18 -> 21, `evac_bytes` 640 -> 736,
`encode_alloc_bytes` 665,662,512 -> 665,662,608, `encode_allocs` 5,332,649
-> 5,332,652, `encode_evac_allocs` 24 -> 27, `encode_evac_bytes` 912 ->
1,008, `oneshot_alloc_bytes` 2,375,978 -> 2,376,074, `oneshot_allocs` 12,064
-> 12,067, `oneshot_evac_allocs` 18 -> 21, `oneshot_evac_bytes` 624 -> 720,
`wide_alloc_bytes` 5,590,880 -> 5,590,976, `wide_allocs` 128,017 -> 128,020,
`wide_evac_allocs` 125 -> 128, `wide_evac_bytes` 516,000 -> 516,096,
`digest_alloc_bytes` 697,025 -> 697,121, `digest_allocs` 7,193 -> 7,196,
`digest_evac_allocs` 48 -> 51, `digest_evac_bytes` 4,176 -> 4,272,
`read_alloc_bytes` 198,768 -> 198,864, `read_allocs` 413 -> 416,
`read_evac_allocs` 18 -> 21, `read_evac_bytes` 608 -> 704,
`live_alloc_bytes` 409,410,992 -> 409,411,088, `live_allocs` 1,120,086 ->
1,120,089, `live_evac_allocs` 18 -> 21, `live_evac_bytes` 608 -> 704.

This builds a ruled part of the language, so the welfare floor moves down by
exactly what the rows cost, under the #1975 gavel as narrowed by the
2026-10-17 gavel: 91.5235 -> 91.4587.

## 2026-10-18 — A module's own function beats a type an import declares

Building std/text's `span` turned up a check that read the wrong name. In a
merged module check, the bare name of a type another module declares reached
the construction check before a function of the same name in the program's
own module. `fn sorted a b` beside `import "std/list"` was refused as a
construction of std/list's `sorted` with too many fields, and any program
with a helper named `span` would have been refused the same way once std/text
declares the type. A type and a function cannot share a name inside one
module, so a bare name that is both is the module's function. `arity_at` now
skips the construction check for a name that is a known function group.

The fixture is `a_function_named_like_an_imported_type_is_the_function` in
the micro corpus. Without the change, `kanso check` refuses it with the arity
error. With the change, it prints `3` on every engine.

Rows, from CI: `compile_instructions` 22,715,476 -> 22,716,927,
`entry_instructions` 75,262,121 -> 75,265,987, `library_instructions`
74,948,228 -> 74,952,083 and `browser_compile_instructions` 382,741,447 ->
382,750,920. The check does no new work on these programs, so most of the
movement is the binary's layout. The floor drops by the cost to 91.52, under
the rule that a change making the language work as specified lowers it by
exactly what it costs.

## 2026-10-18 — A module's own type beats an imported type's twin

The type-side partner of the entry above. The loader clones each exported
type under its short name, so a program may write `sorted` for
`list/sorted`. A module that declared its own `type sorted` kept the twin
beside its declaration, and the twin answered: the module's constructions
were refused as foreign, with the import's field count held against them.
Once a module's types join the merged program, `own_types_beat_twins` drops
a synthetic short-named twin that shares a name with one of them, and the
import's type is still reachable by its qualified name. std/text's `span`
met this in `tests/golden/entryfile`'s diff module, which declares its own
six-field `span`.

The fixture is `a_type_named_like_an_imported_type_is_the_modules` in the
micro corpus, refused by both `kanso check` and a run before the change and
printing `2` on both engines after it. The first build kept a set of the
module's type names on every compile, 23 allocations in `compile_allocs` and
`interp_allocs` for a collision almost no program has; the filter now
allocates only when it finds a twin to drop, and both rows are where they
were.

Rows, from CI: `compile_instructions` 22,716,927 -> 22,721,761,
`entry_instructions` 75,265,987 -> 75,287,373, `library_instructions`
74,952,083 -> 74,973,469, and `browser_compile_instructions` 382,750,920 ->
382,767,025: each compile asks every type whether it is a beaten twin. The
floor drops by the cost to 91.52 under the rule for a change that makes the
language work as specified.

## 2026-10-18 — gavel: the span maker is `text/to_span`

Clay ruled #1992. The 2026-10-17 gavel on #1983 named a type `span` and a
function `text/span s from to`. A type's name is its constructor, so the
constructor `span whole from to` and the function share a name and an arity,
and the checker refuses the pair.

**The ruling.** The function is `text/to_span s from to`, answering a span
when the range lies inside `s` and `none` otherwise. A `to_` name means the
answer is a different kind of thing (2026-10-13, #1914), and `list/to_sorted`
builds a checked `sorted` from a list the same way. The type stays `span`,
and `span_to`, `span_to_below` and the span arm of `text/slice` keep the
names ruled on #1983.

Owes, on #1992: `text/to_span` in place of `text/span` wherever #1983's build
uses it; appendix B.

## 2026-10-18 — std/text answers spans, and the decoder takes them

#1960, under the gavels on #1983 and #1992; the second named the maker
`text/to_span`, because a function cannot share the type's name. std/text
declares `span`, with readable fields `whole`, `from` and `to`, and answers
one from `text/to_span s from to`, `text/span_to cs p a b` and
`text/span_to_below cs p a b lim`. `text/slice run` is an arm of `slice` that
takes a span and answers its bytes with no `none`. The type and its functions
live in `lib/text/span.kso`, so the line numbers err goldens quote from
text.kso stay where they are. Appendix B has an entry for each, and ch08
quotes the decoder's new shape.

The slice arm turned up a pass in the wrong place. `builtin_slice_in` was
renamed to `slice` while the parse was finished, before the check read it, so
a function answering a measured slice was typed as answering `none`. The
rename now runs where a checked program is handed to the engines.
`a_span_runs_to_the_byte_before_a_stop` in the micro corpus is refused
without the move.

The decoder's five `builtin_slice_in` sites read spans now, and
`string_scan` is gone. Moved as written, the decoder read 1,645M
instructions on jsonbench against 833M: every string built a span record,
195M of it in `k_rec`, and `text/slice run` reached none of the fused doors.
A new pass, `src/flat.rs`, runs beside the rename. A group whose arms take a
record at one position, or `none`, and only read its fields or hand it on to
another such group, gets a twin that takes the fields there. A call handing
a producer's answer straight to such a group (a producer is a one-arm group
of guards answering `none` and then a construction, the shape of
`text/span_to`) becomes a call to a copy of the producer that calls the
twin. `text/slice run` inside a twin becomes `builtin_slice` on the fields,
so the fused doors fire again. The twins keep their file and spans, and
`ast::spoken` takes the `Flat{n}_` mark off their names, so a failure inside
one names the function the author wrote. The producer's copy enters a twin
only when the twin's arms cannot miss, so a miss never names the producer's
line.

A bare construction is not a producer. The first build counted `keep` in
`a_long_chain_answers_survival_from_its_index` as one, and the loop ran half
on fields and half on records: 9,993 more allocations and 10,000 more beat
iterations. That shape is the escape analysis's, which hands a record back by
value where it can.

The twins' positions were boxed at first, because a group takes words only
when it is a member of a cycle, and jsonbench read 938M. A group the pass
made now takes its int fields as words too: 864M with the string sites moved
and 868M with the number sites, on this container. What is left is the range
check in `span_to` and `to_span` and the `- 1` and `+ 1` around an inclusive
`to`. The mem corpus reads the same counters as before the move, and
`a_span_handed_on_is_never_built` pins that a span handed from `span_to` to a
carrier builds no record: `sh_rec` 0 and 12 allocations, against 720 bytes
and 21 without the pass.

Two leaks of an imported type's short name surfaced on the way and landed
first, as #1993 and #1995. kq's copy of the decoder follows once kq repins.

Rows, from CI, against the tree with #1985 and #1996 in it. The decoder:
`work_jsonbench` 832,975,500 -> 871,576,578, `work_runbench` 1,189,193,317 ->
1,215,497,872, `work_oneshot` 13,286,600 -> 13,544,585, `work_livebench`
1,498,216,646 -> 1,498,473,868, and `work_encodebench` 2,464,675,698 ->
2,464,674,663. The others move by the size of a std/text with more in it:
`work_basket` 31,158,162 -> 31,158,240, `work_digestbench` 5,992,785 ->
5,992,876, `work_indexbench` 2,482,608 -> 2,482,712, `work_pendbench`
194,087,911 -> 194,087,989, `work_readbench` 4,582,588 -> 4,582,666,
`work_scanbench` 274,086 -> 274,177 and `work_widebench` 28,628,235 ->
28,628,313. The twins are more code: `emitted_defines` 111 -> 127,
`emitted_calls` 1,024 -> 1,216, `emitted_branches` 1,104 -> 1,300,
`emitted_lines` 10,362 -> 12,289, `emitted_other_defines` 2,089 -> 2,137,
`emitted_other_calls` 17,648 -> 18,224, `emitted_other_branches` 16,418 ->
17,006, `emitted_other_lines` 149,547 -> 155,354, `text` 5,011,246 ->
5,111,966, `emit_instructions` 6,061,105 -> 6,100,683,
`codegen_instructions_dev` 115,010,801 -> 115,149,899 and
`codegen_instructions_release` 449,091,786 -> 449,262,284. The front end
reads 51 more library lines: `compile_instructions` 22,693,337 -> 24,958,388,
`compile_allocs` 12,151 -> 13,741, `compile_peak_bytes` 552,167 -> 591,860,
`front_end_visits` 6,363 -> 7,309, `front_end_rounds` 19 -> 17,
`entry_instructions` 75,197,395 -> 75,588,739 and `library_instructions`
74,884,283 -> 75,259,150. The interpreter makes one more call per string:
`interp_instructions` 500,108,350 -> 571,513,912, `interp_allocs` 584,645 ->
651,909 and `interp_peak_bytes` 590,970 -> 632,433. The tab:
`browser_compile_instructions` 389,899,281 -> 420,614,367,
`browser_compile_peak_bytes` 675,444 -> 752,488 and `browser_run_instructions`
35,216,926 -> 37,494,108. Welfare falls 0.20, to 91.26, and the floor drops by
exactly that under the rule for building a ruled part of the language.

## 2026-10-18 — A local binding hides an alias only where it is bound

`canonicalize_bare_aliases` rewrites a bare call to an import's export into
the qualified name, and keeps a name bare wherever a local of that spelling
might mean the local. It kept the name bare in every declaration of the
program when any one of them bound it. A parameter called `run` in one
module therefore sent every other module's `run` through the loader's
short-named clone. An err born in that `run` said `born in run` where the
same program without the other module says `born in refusals/run`, and
basket, once std/text's `slice run:span` bound the name, read 6,113M
instructions against 31M, with 4,010 view allocations and 256MB held against
17 and 71KB. Both reproduce on main with no spans in them.

A declaration that binds an alias's name now keeps the bare spelling and the
clone it dispatches through, and every other declaration is rewritten.
`tests/bound_elsewhere.rs` runs `tests/golden/bound_elsewhere` on both
engines and fails on main with `born in run`. The first build walked each
declaration's bindings into a set of its own and cost entry and library
about 2.5%; the binding walker hands each name to a callback now, so one
walk asks the aliases about every bound name and builds no set.

Rows, against main with #1985 in it: `compile_instructions` 22,693,337 ->
22,742,937, `entry_instructions` 75,197,395 -> 75,386,824,
`library_instructions` 74,884,283 -> 75,073,678, `emit_instructions`
6,061,105 -> 6,063,934, `compile_allocs` 12,160 -> 12,151, `interp_allocs`
584,659 -> 584,645, and `browser_compile_instructions` 389,899,281 ->
390,131,214. The floor drops by the cost under the rule for a change that
makes the language work as specified.

## 2026-10-19 — a `slice!` handed straight to a door reads the range in place

#1943's last owed item: the emitter fusing `slice! … .> utf8`, `.> append
acc`, `.> to_int` and `.> to_float` as it fuses the nested spellings (the
2026-10-14 gavel, corrected the same day to the chain spellings). The decoder
moved onto spans in #1994 and no longer needs it, so what is left is the
spelling a program uses for a range it has already bounded.

Under the corpus-first gavel the shape went into the run program first, as a
phase called `insist`: 5,000 laps reading a year, a month and a word out of one
line. Measured on main at kanso#1994, it costs 11,955,088 instructions built
alone, 0.97% of the program, and adds 25 bytes to the peak. Welfare's run
baselines were re-based by the readings with the phase over without it on that
compiler, 1,227,450,174 over 1,215,496,727 and 5,063,225 over 5,063,200, so
adding the phase cost that compiler nothing.

The rewrite runs at the start of the flatten pass. `text/slice! s from to .>
text/utf8` becomes `if (text/past? s from to) (text/slice! s from to) (effect
(text/utf8 (builtin_slice s from to)))`, and the emitter fuses the door in the
third argument. `past?` is std/text's statement of where `slice` answers none,
and the prune that drops a shipped function nobody names keeps it whenever
`slice!` is kept. A range that leaves the value is still the `slice!` the
author wrote, failing on their line. The arguments are read twice, so the
rewrite takes only names, integers and `+`, `-` and `*` on them.

The first version was a `!` helper in std/text per door. It saved the slice
but was itself an insisting form, so an err the door raised, `"ka" is not an
integer`, was pointed at the caller's line where the two steps leave it in
`text/to_int`. An optimisation should not change what a program reports, so
the helper went.

Three faults turned up on the way, each with a golden that goes red without
its fix:

- The `to_int` and `to_float` doors gave an err the call site's origin. The
  oracle runs the wrapper, so the err is born in `text/to_int`. A rescue on
  the caller's own line lets through a failure born on that line, so on native
  alone the err escaped `.?`. The doors now take the wrapper's line, as the
  `utf8` door did. `a_slice_bang_chain_opens_its_door_in_place` reaches the
  endpoint on native without it.
- The `utf8` door stamped no site, so a value it cannot take stopped the
  program naming whatever line had stamped last (`text/past?`). It stamps the
  wrapper's line unless the value is known to be bytes.
  `a_door_a_slice_bang_opens_fails_inside_the_door` pins it.
- `.> (&text/append acc)` was refused: the formatter called the parentheses
  redundant, and without them the chain step refuses held arguments. After a
  chain operator the step is one atom, and the formatter now says so.

The new mem fixture `a_slice_bang_chain_builds_no_slice` reads 45 allocations
against 95 for the two steps. Built alone, the phase reads 6,081,704 against
11,955,088. Each step still builds an `effect` box and the closure for the
next step, which is what a chain costs.

Rows, from CI. The phase's own work moves the run program: work_runbench
lands on 1,221,576,962 against 1,215,497,872 without it, and its `.text` row,
text, on 5,132,910 in total. The run vein lands on run_allocs 1,872,053,
twelve a lap, run_alloc_bytes 303,335,164, run_beat_iters 92,018,
run_evac_allocs 49,265, run_evac_bytes 14,919,712, run_perm_allocs 101,
run_perm_live_bytes 15,825, run_perm_peak_bytes 146,913 (the line, 25 bytes),
run_sh_bytes 4,217,520 and run_utf8_bytes 424,489. The emitted code for the
other benchmarks, runbench's among them, lands on emitted_other_lines 156,609,
emitted_other_calls 18,424, emitted_other_branches 17,158 and
emitted_other_defines 2,169. compile_instructions 25,083,410,
entry_instructions 76,579,726 and library_instructions 76,283,760 rise for the
rewrite and `past?`, and browser_run_instructions lands on 37,494,129, 21
more. emit_instructions 6,095,522, interp_instructions 570,698,408 and the
tab's compile, 419,334,691, fall, and oneshot and livebench fall by 1,099 and
7, by mechanisms not isolated. Welfare rises 0.02 and the floor is banked at
91.28.

## 2026-10-19 — json/decode refuses a number RFC 8259 refuses

#1999. `json/decode` took `1.`, `2.e5`, `01`, `-01`, `00` and `-.5`, and
`[1., 2]` decoded as `[1.0 2]`. RFC 8259 §6 refuses all of them. The decoder
finds a number's bytes with `text/number_span` and hands them to `to_int` or
`to_float`, and the converters are more lenient than JSON. They already refuse
`1e`, `1..2`, `1+2` and `--1`, so what they take that JSON does not comes down
to three rules: a leading 0 has no digit after it, a minus has one, and so
does a point.

`number_span` answers zero when a number breaks one of them. The first two are
reads at the number's start. The third uses the masks the sixteen-byte walk
already builds: a point whose next bit is not a digit, with the block's last
byte checked against the next block. The interpreter keeps the same three
rules, so `number_span` answers the same on every engine for every input.
lib/json needs no new arm: a zero sends the integer path to ask `to_span` for
a range that ends before it starts, which answers none, and the decoder fails
as `invalid number`.

The first version checked the rules in lib/json with a `find2_below` for the
point, and jsonbench read 948M against 872M, about 240 instructions a float.
A full grammar walk inside `number_span` read 942M, since jsonbench's numbers
are long. The three rules on the masks read 885.5M, 22 instructions a number.

A differential fuzzer against the RFC's grammar written as a regular
expression decoded 200,000 random strings over the number alphabet, each bare
and again padded past a block, on both engines, with 0 mismatches. Main read
1,317 mismatches on the first 20,000. `number_span` agrees across engines at
every position of 3,000 random strings. `a_json_number_follows_rfc_8259` pins
the cases, and `a_number_span_ends_where_the_byte_walk_ends` gains the zero
answers, with a bare point at a block's last byte and in the walk.

kq carries its own decoder and keeps a number as the decimal its author
wrote, with jq as its oracle, so what it accepts is jq's question and this
change leaves it alone.

Rows, from CI. work_jsonbench rises 13,919,550 to 885,496,128, work_runbench
9,186,504 to 1,230,763,466, work_oneshot 93,714 to 13,637,200,
work_livebench 93,070 to 1,498,566,931 and work_encodebench 693 to
2,464,675,356, all in `number_span`. The tab's run, browser_run_instructions,
rises to 37,652,074, and its compile to browser_compile_instructions
419,365,540 with browser_compile_peak_bytes 752,376. text lands on
5,136,942, 288 bytes a binary for the check. compile_instructions
25,085,857, entry_instructions 76,583,774, library_instructions 76,287,809,
interp_instructions 570,745,964 and codegen_instructions_release 449,265,297
rise for the edit to the compiler's Rust. Every allocation counter agrees with
main. The floor drops by the cost under the rule for a change that makes the
language work as specified, a JSON decoder that refuses what RFC 8259 refuses:
welfare falls 0.03, to 91.25.
