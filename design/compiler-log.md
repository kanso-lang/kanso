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
## 2026-10-07 — the program's inline threshold stays at 2000

The comment above `-inline-threshold=2000` in src/main.rs declined 4000 on
2026-09-19 for a 3.64% rise in `codegen_instructions_release`. That rise was
measured while the gate found clang 18, which kanso#1795 corrected, so I
measured 4000 again under clang 19 on the tree after kanso#1796.

The run gains hold. Against 2000, in this container:

    runbench    1,129,589,618 -> 1,122,248,562   -7,341,056  (-0.650%)
    encodebench 2,384,337,036 -> 2,367,226,282  -17,110,754  (-0.718%)
    livebench   1,473,485,636 -> 1,460,513,428  -12,972,208  (-0.880%)
    jsonbench     860,548,095 ->   854,565,945   -5,982,150  (-0.695%)
    digestbench     5,566,368 ->     5,510,253      -56,115  (-1.008%)

oneshot falls 73,560, and the other rows move by fewer than 2,000 or not at
all. Summed `.text` grows 187,376 bytes, 4.1%, of which runbench accounts for
89,296.

On the codegen corpus, `clang -cc1` and ld.lld read byte-identical at the two
thresholds, 206,785,011 and 250,049,700. Scored on those rows, welfare would
read 90.07 against a floor of 90.04.

Building runbench gives the other side. Traced the same way, with clang 19, a
release build of bench/runbench at 4000 costs `clang -cc1` 11,766,666,246
instructions against 8,481,293,726 (+38.7%) and ld.lld 27,062,341,881 against
20,412,779,482 (+32.6%). The corpus is 62 lines of kanso and has no function
whose inlining cost falls between 2000 and 4000, so the threshold can move
without the corpus seeing it. Runbench has many such functions.

So 4000 stays declined. Taking it would have banked a run gain whose build cost
the objective has no row for.

I ran the same check on correlated propagation and early-cse, which kanso#1796
priced on the corpus. Building runbench, ld.lld reads 19,947,820,412 before
them and 20,412,779,482 after, +2.33%, where the corpus read +3.37%, and
`clang -cc1` moves 0.13%. For those passes the corpus priced the change in
proportion.

Open: the release codegen row reads one small program. A change whose build
cost grows with function size, such as an inline threshold or an unroll count,
can move it by nothing. Counting runbench's own build beside it would close
that, at about fifty-five times the corpus build's instruction count under
callgrind.

## 2026-10-07 — the interpreter reads a local where it last found it

A name the interpreter evaluates is resolved through `lookup`, which walks the
environment from the innermost frame outward and compares the name against
every binding it passes. On the interpreted corpus that walk ran 654,409 times
for 45,066,454 instructions of its own, 69 a call, and was the third-largest
function in the profile after the dispatch loop and `eval`.

Each name node already keeps a `Resolution`: 0 until it first runs, 1 for a
local, and a stamp for a global. The stamp is sound because the frames a node
sees are fixed by where the node sits in the text. That property also fixes
where the binding sits, so the node now keeps that too. A local found at frame
`d` above the innermost, in slot `s` of that frame, keeps `3 + 256d + s`,
which lies between plain local and the global stamps. A later run follows `d`
parent pointers, compares the one name in slot `s`, and takes the value. A
miss walks the frames the old way and keeps the new place. A frame deeper than
127 or a slot past 255 keeps plain local and always walks. Debug builds check
every kept place against a full walk, so the specs check the property on
every program they carry.

`interp_instructions` reads 585,141,444 -> 573,873,591 in this container, a fall
of 11,267,853 (-1.93%), where main reads 585,141,444 on the row. The kept
place still costs about 45 instructions a reference, most of it the clone of
the value it returns. `interp_peak_bytes` and `interp_allocs` do not move,
since the place lives in a word the node already had.

The ratchet row `local_place` keeps plain local for every reference, and the
interpreted row is its witness. Under that mutation the row reads 613,859,041,
above main as well, since every reference then takes the out-of-line walk.

## 2026-10-07 — a local's scalar copied inline costs more than the clone

After the interpreter learned to read a local at the place it last found it,
`lookup_at` still spent about 45 instructions a reference, most of them in the
derived `Value::clone`, which is a call for every variant. A small int, a float
and the four unit values hold no count to raise, so `lookup_at` was given a
match that writes those six out directly and sends everything else through
`clone`. `interp_instructions` rose 573,873,591 -> 586,635,237 (+12,761,646,
+2.22%) in this container. The output was unchanged. The match was not
isolated from what inlining it did to the surrounding code, and it was taken
back out.

`docs/compiler.html` §198 and §199 cover this entry and the two before it.

## 2026-10-08 — two more ideas measured and declined

The interpreter's tail calls each allocate an argument vector, and the
dispatcher frees it once the arguments are bound, so a chain of tail calls
allocated and freed one vector a hop. Keeping the emptied vector in a `Cell` on
the interpreter for the next tail call to fill raised `interp_instructions`
573,873,591 -> 576,314,674 (+2,441,083, +0.43%). Many of the vectors `eval_tail`
fills end in an ordinary call rather than a hop, so the spare was often empty,
and the take and the set cost more than the allocations they saved.

The release link's loop unroll threshold was raised through
`-Wl,-mllvm,-unroll-threshold`. At 600 runbench read 176 instructions higher
and `.text` grew 512 bytes; at 4000 runbench fell 2,445 and `.text` grew
19,264. The loops runbench spends its time in are not held back by the
threshold.

## 2026-10-08 — the gates name the allocator's host settings

The standing row "A welfare counter reads three parts per billion" ends on a
pointer: the frames that moved between its corpus arms were the allocator
committing pages, `_mi_os_commit_ex`, `mi_bitmap_setN` and `_mi_prim_commit`.
What decides that path is in mimalloc's start-up. It opens
`/proc/sys/vm/overcommit_memory` and, when the setting is 0 or 1, maps with
`MAP_NORESERVE` and lets arenas commit on demand; it reads
`/sys/kernel/mm/transparent_hugepage/enabled` to decide whether huge pages
are on offer. Both are the runner's settings, both reach the interpreted and
compile rows through the allocator those rows run on, and neither appears in
any job log. This container reads `overcommit=0 thp=madvise`.

mimalloc reads them with raw `syscall(SYS_open)` and `syscall(SYS_read)`, so
the gate's `LD_PRELOAD` cannot hand them a fixed value, and this container
cannot test the hypothesis by changing them. `scripts/gates/dispatch.sh name`,
which every instruction gate already calls to print its cpu, now prints an
`allocator:` line with both settings before the `silicon:` line. When two
runners of one commit next read a row apart, the two job logs say whether
these settings differed. `the_silicon_a_row_was_counted_on` requires the line
with a loader and without one.

## 2026-10-08 — two ways to shrink a decoded document, and neither moves the run peak

`run_peak_bytes` reads 3,899,936: the arena's 3,670,032, the held pool's
197,704 and the permanent pool's 32,200. The arena is counted in 1 MiB blocks
plus the index shape's 524,304-byte oversize block, so a change that saves
memory moves this row only when it frees a whole block. The 2026-09-24 entry
found that the top-level `doc` keeps two of the three blocks for the whole run.

One decode of bench/large.json allocates 1,355,840 bytes in 9,288 allocations.
Counting them by size names three shapes. The 2,761 objects take 208 bytes
each, 574,288 in all, because `{}` opens with room for five pairs and the
document's objects hold one to five keys. The 2,752 arrays take 128 bytes
each, 352,256 in all, for one to six elements. The 1,773 strings that contain
an escape take 112 bytes each from `string_scan`, 198,576 in all, for text that
is mostly under 32 bytes, plus a 16-byte header from `str_run`.

Two changes were measured against that.

- Smaller seeds, three pairs for a map and four slots for a list where they were
  five and six. The arena peak held at 3,670,032 and runbench rose 4.18%
  (1,129,589,604 -> 1,176,795,989), because the commonest sizes now grow once.
- A string that takes its builder's buffer in place gives the unused room back
  when that buffer ends at the arena's frontier. It fires on the decoder's
  escaped strings and returns 32 bytes for each, about 57 KB a decode. The run
  program's counters came back byte-identical, and the only vein that moved was
  widebench's beat iterations, 18 -> 15. No objective term moved, and it adds a
  test to every built string, so it was taken back out.

Objects and arrays cannot get the same treatment at the point they close. Each
is allocated when it opens and its children are allocated after it, so a
container is at the frontier when it closes only if nothing inside it reached
the heap.

## 2026-10-08 — the tab's compiler keeps small blocks for itself

The browser side's compile row was the one welfare term worse than its
baseline: `browser_compile_instructions` read 514,922,959 against 343,917,485.
A native profile of the same work, `bench/interp_corpus` compiled and lowered
to wasm, put about 13% of its instructions in malloc and free, from 76,354
allocations. Native kanso runs on mimalloc, which keeps freed blocks on a list
for each size. `docs/kanso.wasm` runs on `std::alloc::System`, which on wasm32
is dlmalloc, and dlmalloc has no such lists.

The allocator under the compiler's tally now keeps them on wasm32. A request of
512 bytes or fewer is rounded up to a multiple of eight. A freed block goes on
the list for its size, and the next request of that size takes it before
dlmalloc is asked. `realloc` was the trait's default before, an allocation, a
copy and a free. It now keeps a small block in place when the new size falls
in the same class, and hands two large blocks to dlmalloc's own `realloc`,
which can grow one where it stands.

`browser_compile_instructions` reads 514,922,959 -> 453,713,751, a fall of
61,209,208 (-11.89%). The `realloc` change alone accounts for 1,614,339 of it.
A ceiling of 256 bytes cost 0.8% more than 512, and one of 1024 saved 0.3%
more while keeping twice as much on the lists. The other three browser rows do
not move. `browser_compile_peak_bytes` holds at 829,597 because the tally
counts requested bytes before any of this, and a `realloc` is counted as the
allocation, copy and free it replaced. The browser rows belong to the rustc
that built the artifact, which is 1.98.1 here as on CI.

Native kanso is untouched. Taking `realloc` through `mi_realloc` there was
measured first and made the compile rows worse: `compile_instructions` +45,622,
`entry_instructions` +156,320, `emit_instructions` +166,452. So the override
exists only on wasm32, and `compile_instructions` and `emit_instructions` read
their goldens with it in place.

The ratchet row `small_blocks` sets the ceiling to nothing, so every block goes
to dlmalloc. The browser compile row reads 513,308,620 under it.

## 2026-10-08 — the emitter writes its lines without the formatter

`FnEmit::write` appended each line of a function body with
`writeln!(self.out, "  {text}")`. The line is already a string, so the only
work is two spaces, the text and a newline, and the formatter spent about 540
instructions a line getting there. On the browser corpus that was 8,571 lines
for 4,657,137 instructions. The line, the block label `start_block` writes and
the stack slots `body` hoists into the entry block now go onto the buffer with
`push_str`.

The module text is the same; `machine_code` and `emitted_code` agree.
`emit_instructions` reads 24,923,517 -> 23,997,530 in this container, a fall of
925,987 (-3.72%), and CI's reading replaces it if it differs.
`browser_compile_instructions` reads 453,713,751 -> 450,123,996 (-0.79%) over
the tab's small-block free list, which belongs to the rustc that built the
artifact, 1.98.1 here as on CI. On main before that list it read
514,922,959 -> 511,333,204, so the saving is the same 3.6 million either way. The
`kanso check` rows do not move, since they stop before codegen.

The ratchet row `emitted_lines` sends the body line through `writeln!` again,
and the emit row reads 24,766,221 under it.

## 2026-10-08 — a dispatch stops dropping values that own nothing

When the interpreter dispatches a call, `bind_moved` takes each argument a
plain parameter binds and leaves `NoneV` in its place. The argument vector was
then cleared with `args.clear()`, which hands every element to `Value`'s drop.
`Value` has nineteen variants and its drop is an out-of-line function, so
each placeholder cost a call that only found out there was nothing to free. On
the interpreted corpus `dispatch_loop` made 1,384,579 of those calls, about 17
instructions each.

The vector is now emptied through `release`, which matches the variants that
own nothing (a small int, a float, the booleans, `none`, `done` and a table
slot) and forgets them, and drops the rest as before. `interp_instructions`
reads 573,873,591 -> 568,708,684 in this container, a fall of 5,164,907
(-0.90%), and CI's reading replaces it if it differs. Forgetting only `NoneV`
read 570,633,594, so the other trivial variants account for a further 1.9
million.

Two neighbours were measured and are not built. Releasing the pooled
frame's old bindings the same way, in place of `binds.clear()`, read
578,197,383 with the argument change beside it: the pop loop over pairs cost
more in `dispatch_loop` than the calls it saved. And in the browser's
translator, adding up integer literals of eighteen digits or fewer in an
`i64` without checks, ahead of the checked `i128` path, moved
`browser_compile_instructions` from 450,123,996 to 452,683,338. The `i128`
loop was not the cost.

The ratchet row `arguments_released` puts `drop(arg)` back in the loop, and
the interpreted row reads 576,822,655 under it.

## 2026-10-08 — the tab compiles only what its entry reaches

The emitter wrote a dispatcher for every group in the program, and on a
program with an entry `Written::finish` then pruned whatever nothing reached
from it. On `bench/interp_corpus` the prune dropped 132 definitions and 127,415
bytes of the 369,868 the tab had written, almost all of them std/list
functions the corpus never calls. The text was held beside the program until
the end of emission, which is the most the tab's compile holds. Every analysis
before the emitter also read those functions: inference, linearity, the beat
loops, escape and demand each walked the whole of std/list.

The tab now walks the names each body mentions, from the entry, and takes
every group the walk does not reach out of the program before any analysis
runs. The walk is `call_graph`, which the cohort brackets already use. Some
groups are reached without a body naming them. An interpolated value
dispatches to `render`, a value-wrapped `print` renders through
`render/to_string`, an operator finds a user's arm by asking whether the
program declares one, and fused map columns stand in for `keys` and `values`
only when the program declares neither. Those names are seeded.

The first version of this skipped unreached groups in the emitter and kept
them in the program, and it caught a miss by looking for a skipped group's
symbols in the module. That check cannot work once the group is gone, because
the emitter stops writing a call to something that is not there. The decisive
case is an operator. With a user's `+` arm left out, the emitter asks whether
`+` is declared, hears no, and sends two records down the numeric path. The
module names nothing of `+` either way. So every question the emitter asks
about a group by name now goes through one index, `group_named`, and a
question about a left-out group sets a flag. A flagged module, a module naming
a left-out group's symbols, and an emit that fails are all sent back with the
program whole, so a miss costs a second emit and cannot change what the tab
runs. A unit spec reads the emitter's source and fails if it scans its groups
by name anywhere else. Across the 98 entry programs in the tree that compile
alone, none needed the second emit.

Native builds are unchanged and keep emitting every group. Turning the walk
on for them as well moved `emit_instructions` from 23,994,397 to 15,189,091 on
`pkg/codegen_corpus`, and the meta by about 0.01, because the emitting term
is already near the top of its curve. That gain does not pay for renumbering
every native module and re-reading the machine-code goldens, so it waits. The
tab's module differs from the dev tier's in the numbering of lambdas, constant
cells and strings, and in what inference concludes once the unreached callers
are gone. The browser differential agrees on all 575 programs it compares,
with the same six known gaps.

`browser_compile_instructions` reads 450,123,996 -> 395,882,998 (-12.1%) and
`browser_compile_peak_bytes` 829,597 -> 673,727 (-18.8%), on rustc 1.98.1 here
as on CI. Skipping in the emitter alone had reached 433,427,683 and 698,157, so
taking the groups out before the analyses is two thirds of the instruction
saving. `browser_run_instructions` moves by 61, 33,510,647 -> 33,510,708. The
ratchet row `reached_groups` turns the walk off and the browser rows go back
up, and `left_out_asked` stops noting the question, which the `+` spec catches.

The index helps native builds as well. Every lookup of a group by name used
to scan the whole declaration list, and now it reads the index the emitter
already kept: `emit_instructions` falls 23,997,530 -> 23,393,086 (-2.52%) on
CI. Three front-end rows rise on CI: `compile_instructions` 26,182,238 ->
26,200,562, `entry_instructions` 86,381,596 -> 86,445,364 and
`library_instructions` 86,927,430 -> 86,991,343. Those rows run `kanso check`,
which never reaches the emitter. Between main and this tree in one container
the move sits inside the lexer, whose source did not change, so it is inlining
redrawn around a changed codegen.rs.

## 2026-10-08 — a dispatch ranks its candidates on the stack

Arm selection ranks every candidate that matches into a score, one byte a
parameter, and keeps the best. The score was a `Vec<u8>`. Every entry to
`dispatch_loop` allocated one and freed it on the way out, and when the first
candidate won, `std::mem::take` moved its buffer into `best` and left the next
candidate to grow a new one from nothing. On `bench/interp_corpus` that growth
was 44,905 calls to `RawVec<u8>::grow_one`.

`Score` now holds twelve ranks inline and spills the rest into a vector that
stays empty for any group of twelve parameters or fewer. A dispatch compares
two scores of the same length, one rank per argument, so comparing the inline
part and then the spill orders them as the vector did. A unit spec generates
pairs of scores that differ in one position, inline or spilled, at every length
from one to sixteen, and checks `beats` against the vector's comparison. With
the spill left out of the comparison it fails on a thirteen-rank pair.

`interp_instructions` reads 568,709,639 -> 559,187,740 in this container
(-1.67%), with the corpus printing the same 874 lines, and CI read the same
number. `interp_allocs` falls 873,371 -> 752,938, 120,433 fewer calls to the
allocator, and the peak does not move.
The ratchet row `scores_inline` gives the score a heap buffer at the start of
every dispatch again, which puts back about 4.5 million of the 9.5.

The bindings vector takes the same move when the first candidate wins, and its
growth is the other half of what `dispatch_loop` grows. It is not changed here:
the winner's bindings become the environment frame, so a second buffer has to
exist whenever a later candidate is tried.

## 2026-10-08 — a dispatch hands its second bindings buffer back

The entry before this one left the bindings vector alone because the winner's
bindings become the environment frame. When the first candidate matches,
`std::mem::take` moves its buffer into `best`, and any later candidate the
dispatch tries matches into a new vector. That vector was allocated for the
dispatch and freed at its end, whether the later candidate won or not.

The interpreter now keeps a short list of those buffers, at most 64. A
candidate that finds `binds` empty pops one from the list before it matches,
and at the end of the dispatch whatever buffer is left in `binds` is cleared
and pushed back. The winner's buffer still leaves with the frame, so the list
only ever holds buffers no frame kept.

Measured in this container on `bench/interp_corpus`, printing the same 874
lines: `interp_instructions` 559,187,740 -> 553,555,448 (-1.01%), and
`interp_allocs` 752,938 -> 714,647, 38,291 fewer calls to the allocator. The
peak does not move. CI's readings replace these if they differ.

Taking a buffer from the list at the start of every dispatch, in place of the
`Vec::new()` that `binds.reserve` then grows, read 556,139,823. Most dispatches
never try a second candidate, and each of those paid for a pop and a push back.

The ratchet row `spare_binds` turns the hand-back off, so each buffer goes to
the allocator as before, and the row reads 558,551,917.

## 2026-10-08 — a short string is held inside the value

Reading a variable in the interpreter clones its value, and `Value::Str` held a
`String`, so every read of a string allocated a copy. On `bench/interp_corpus`
that was 83,612 calls to `String::clone`, 7.9 million instructions inside the
clone before the allocator and the matching free are counted. An entry on
2026-09-18 found the strings this corpus copies average four bytes. An
`Rc<str>` was measured afterwards and declined at 2.84 million instructions
worse, because building one copies the bytes a second time.

`Value::Str` now holds a `Text`. A string of up to fifteen bytes is kept in
place, with its length, and copies as plain bytes; a longer one is a `String`
as before. Every string this corpus clones fits. `Text` dereferences to `str`
and compares, orders and hashes as its `str` does, so the rest of the
interpreter reads it the way it read the `String`.

A map key holds a `Text` as well. With the value changed and the key left a
`String`, putting a string key into a map converted one to the other, and an
inline key allocated where the old code had moved its `String` across.
`tests/a_shared_seed_is_copied_once.rs` caught it: 300 more laps of its
builders cost 3,947 allocations against the 3,647 it pins, one more a lap.
With both sides `Text` it reads 3,647 again.

The width is fifteen because that is what fits beside a `String` without
making `Text` bigger. The first version held 22 bytes. That cost `Text` the
niche in `String`'s capacity field, `Text` grew to 32 bytes and `Value` to 40,
and every move and drop of every value paid for it: the corpus ran 566,595,621
instructions, 13 million more than before, with the allocations gone. At
fifteen, `Value` is 32 bytes again.

Wrapping the two forms in a private enum inside a `pub struct Text`, so that no
code outside it can build an inline string from bytes that are not UTF-8, read
547,075,871: the derived `Clone` on the wrapper added 5.7 million instructions
inside `Value::clone`. A `Clone` written by hand, matching the two forms and
copying the inline one directly, reads the same as the bare enum did.

Measured in this container, printing the same 874 lines, with values and map
keys both changed: `interp_instructions` 553,555,448 -> 540,215,448 (-2.41%),
and `interp_allocs` 714,647 -> 626,209, 88,438 fewer calls to the allocator.
Values alone had read 542,300,428 and 636,769. The peak does not move. CI's
readings replace these if they differ.

Two unit specs pin the shape. One asserts that `Text` is the size of a `String`
and `Value` is 32 bytes; it fails at an inline width of 22. The other reads
strings back at every length from zero to nineteen bytes, in one-byte and
two-byte characters, and checks that any two compare and order as their `str`s
do; it fails when the inline read drops a byte. The ratchet row
`short_strings` sets the inline width to zero, so every string but the empty
one goes to the heap again, and the row reads 550,877,389.

The browser engine links the interpreter, so its binary changed too.
`browser_compile_instructions` moved 395,882,998 -> 396,604,174, a rise of
721,176 (+0.18%), read on rustc 1.98.1 as CI builds it. With the value change
alone it had moved 1,238, so most of the rise arrived with the map keys. What
in the tab's compile reads them is not isolated here.

## 2026-10-08 — a kept local is answered before eval builds its frame

`Interp::eval` held every expression form in one function, so its stack frame
was sized for the largest of them. Every call pushed six callee-saved
registers, reserved 760 bytes, and undid both on the way out. The line holding
the function's signature carried 15.2 million instructions on
`bench/interp_corpus` and its closing brace 11.1 million, about twenty
instructions a call.

Most calls did not need that frame. The commonest expression is a local name
whose place the interpreter has already kept, 653,358 of them on the corpus,
and reading one walks a frame or two and clones the value. `eval` is now a
small function that answers exactly that case and hands everything else to
`eval_node`, which is the old body unchanged. A global, or a local whose kept
place no longer holds it, falls through and is resolved as before.

The gain depends on the compiler inlining the small function where it is
called, so that a local read there makes no call. Forced out of line with
`#[inline(never)]` the corpus read 542,946,770, worse than with no door at
all. `#[inline(always)]` read 512,496,800, which is what the compiler chose
without being told, so the attribute is left off.

Measured in this container on the same 874 lines of output, outside the gate:
539,217,785 -> 512,403,757 (-26,814,028, -4.97%). CI's gate reading is
`interp_instructions` 540,215,448 -> 513,401,792, a fall of 26,813,656
(-4.96%). Nothing allocates differently, so the allocation counters are where
they were.

The ratchet row `front_door`, mutation `a_name_read_through_the_whole_node`,
makes the range test refuse every name, so each one goes through `eval_node`
again. With it applied the corpus reads 538,764,987.

Adding integer literals to the door, which `eval_node` answers with one
clone, read 543,053,630: 30.6 million worse than the door with names alone,
and worse than no door at all. Where it went is not isolated; the out-of-line
copy of the door has the same prologue in both builds, five registers and 32
bytes.
Sending every other name to an out-of-line identifier path instead of
`eval_node` read 515,440,309, three million worse than the door as built.

Two neighbours of this were measured the same day and declined. Both
recycled argument vectors instead of allocating them. A pool fed by `eval` and
`eval_tail` and refilled by the dispatcher read between 537,922,216 and
540,566,075 depending on how it fell back when empty, and a version that also
took the vector back from every builtin's `arity` read 539,054,147 and
543,162,566 for its two ways of emptying it. mimalloc's fast path costs about
63 instructions for an allocation and its free, and the pool cost about 45 to
take a vector and 40 to give it back, so the spread across variants is layout
and none of it is a gain.

## 2026-10-08 — globals and integer literals answered at the door

With the front door built, 730,565 nodes still reached `eval_node` on
`bench/interp_corpus`. Counted by form, the largest group was global names:
299,294 of them, ahead of applications at 137,928 and integer literals at
122,017. Most globals the corpus reads name a function, and the value such a
name reads as never changes: a `FnRef` holding the name.

Each global slot now has a neighbour in a second table, `fixed`, holding that
value when it is fixed: a function reference, `true`, `false`, `none` or
`done`. A constant gets nothing there, because the first read of a constant
runs its body. `eval` asks a small out-of-line function, `fixed_global`, for
any name that is not a kept local, and only a miss goes on to `eval_node`.
Counted over the first 300,000 such reads, 298,949 were answered from the
table. The 1,051 misses were names read before their slot had been stamped,
which is the read that stamps it.

Measured in this container on the same 874 lines of output, outside the gate:
512,403,757 -> 500,796,787 (-11,606,970, -2.27%) with the table alone, and
498,287,606 with integer literals as well, below. CI's gate reading for the
whole change is 513,401,792 -> 499,300,948 (-14,100,844, -2.75%), and four
compile rows moved with the binary's layout by between 1,090 and 68,472.
The table costs memory: `interp_peak_bytes` 739,436 -> 742,431 (+2,995,
+0.41%) and `interp_allocs` 626,209 -> 626,217, read by the gate on a host whose glibc and
rustc are the golden's. The objective weighs that against the instructions
and comes out ahead.

The ratchet row `fixed_globals`, mutation
`a_fixed_global_read_through_the_whole_node`, keeps function references out of
the table, so every one of them is asked for and missed. With it applied to the
build that also carries the literal arm, the corpus reads 524,244,086, worse
than before the table existed, because each global then pays for the miss as
well as for `eval_node`.

Integer literals were the next group, 122,017 of them. Sent from the door to a
one-line out-of-line helper that builds the value, they first read
533,252,591, thirty-two million worse than the table alone. The profile says
why: in that build `eval` ran as a function of its own, 48,560,872
instructions of it, where in the builds that gained it ran almost entirely
inlined into its callers. The extra arm had taken the door past what the
compiler would inline unasked. With `#[inline(always)]` on the door the same
arm reads 498,287,606, 2.5 million better than the table alone, and the
attribute now states what the door depends on.

Applications were the group after that, 137,928 nodes. Moving their arm out of
`eval_node` into its own function and sending them there from the door read
503,651,651, 5.4 million worse, and it is not built.

## 2026-10-08 — the front end's peak, lowered at the two places it was set

`compile_peak_bytes` and `interp_peak_bytes` are both set by the front end:
the interpreter's run never holds more than its check did. Printing live and
peak bytes around each front-end phase on `bench/compile_corpus` put the peak
inside `canonicalize_bare_aliases`, the pass that rewrites a bare alias to the
qualified name it stands for. At its top-level call it held two maps sized for
every declaration in the program: 415 of them, of which 94 were synthetic
aliases. Only those 94 are ever asked about. A name nothing synthetic carries
cannot be an alias, and a site no synthetic declaration stands at is never
looked up.

The pass now counts the synthetic declarations first and returns at once when
there are none, which is most of its calls. Otherwise it keys the site index
by the synthetic sites alone and fills in the qualified declarations that
stand there, and it keeps a name in the second map only when a synthetic
declaration carries it; a qualified declaration of the same name only marks
it as not an alias. The site index is dropped as soon as the names are
resolved, and the set of locally bound names as soon as the aliases are
chosen. On every target tried, the 56 of them that include each bench corpus,
lib/json, kq and vse, `KANSO_ALIAS_REPORT` prints the same aliases before and
after: 76 on the interpreter corpus.

That alone moved the peak by 291 bytes, because the peak was set in two
places. The second was the lexer. Each line's tokens were gathered in a vector
that started at eight and doubled, and that vector was kept for the whole
parse with up to half of it unused. A line's tokens are now gathered in a
buffer kept between lines and moved out into a vector of exactly their length.
The buffer is as long as the file's longest line and is released when the file
is lexed; kept until the end, it held 3,072 bytes under the peak.

Lowering the lexer's share by itself raised the peak by 3,076, to 732,705,
because the alias pass then set it alone. Together, in this container:

    compile_peak_bytes   729,629 -> 706,828   (-22,801, -3.13%)
    compile_allocs        14,742 ->  14,515   (-227, -1.54%)
    interp_peak_bytes    739,444 -> 726,051   (-13,393, -1.81%)
    interp_allocs        626,209 -> 625,986   (-223)

CI's rows replace these before the floor moves.

The move out of the buffer is a `Vec::append` into a vector reserved at the
line's length, which copies the line in one block. A first version used
`drain(..).collect()`, which reached the same counters natively and moved
each token separately: in the browser engine it read
`browser_compile_instructions` 396,604,174 -> 397,453,894, and the lexer
half alone 397,627,379. With `append` the browser row reads 396,335,868
(-268,306), the lexer half alone 396,509,353, and its peak, which is set
after the front end, does not move.

`shrink_to_fit` on each line's vector was tried before either. It reached
the same peak, 706,828, and cost 723 more allocations, because each shrink
is a reallocation.

Two ratchet rows hold this. `line_exact`, mutation
`a_line_of_tokens_grown_by_doubling`, hands the buffer itself to the parse, so
every line grows from empty again: `compile_allocs` reads 15,196. `alias_sites`,
mutation `every_declaration_indexed_for_its_alias`, indexes every declaration
by site again, which changes no alias. Before the change below it read
`compile_peak_bytes` 712,674. After it, the peak is set at a point where the
index is no longer alive, and under the mutation the peak reads 674,684, the
same as without it. The ratchet's shard on CI found that the row had gone
blind. It now watches `compile_allocs`, which reads 14,534 under the mutation
against 14,529, and `compile_instructions` reads 25,933,852 against
25,842,553. On the finished change, then, the alias pass saves five
allocations and 91,299 instructions and no longer lowers the peak.

With the peak no longer set by a pass, it is set by what the program is: the
syntax tree, which is live from the parse to the end of the compile. `Expr`
was 56 bytes on a 64-bit target and `Stmt`, which holds one, 120. The size of
an enum is the size of its largest variant, and the largest was `Guard`, the
only variant holding a vector inline; every other one fits 48. The statements
below a guard now sit behind a box, `Rest`, which makes every expression 48
bytes and every statement 112:

    compile_peak_bytes   706,828 -> 674,716   (-32,112, -4.54%)
    interp_peak_bytes    726,051 -> 693,467   (-32,584, -4.49%)

The first build of that was slower everywhere. Checking the entry corpus went
from 87,188,776 instructions to 89,458,310, spread across every walk over the
tree at about 100,000 each. At 48 bytes the compiler no longer gave `Expr` a
tag byte of its own: it hid the tag in a spare range of one of the fields, and
every `match` over an expression decoded it. `#[repr(u8)]` puts the tag back
in a byte of its own, and `App` lists `piped` first so that byte's neighbour
holds it and the variant still fits 48 bytes. The entry corpus then reads
86,720,594, below the 56-byte tree.

Neither change helps on wasm32, where a pointer is four bytes and `Guard` was
never the largest variant. The box alone took `browser_compile_instructions`
to 397,184,456 and the box with the explicit tag to 398,423,756, against
396,335,868 without either. Both are applied on 64-bit targets only, and the
browser row reads 396,233,999, with the `App` reordering kept everywhere.

Two more ratchet rows hold these. `guard_rest`, mutation
`a_guards_statements_held_in_every_expression`, holds the statements inline
again: `compile_peak_bytes` reads 706,832. `expr_tag`, mutation
`an_expressions_tag_hidden_in_its_fields`, removes the explicit tag: the
entry corpus reads 89,458,575.

The interpreted run's peak is set after the front end, while the program
runs. Probing the heap around the interpreter put 32,768 bytes of it in two
direct-mapped tables `Interp::new` allocates and keeps for the whole run: 1,024
callee slots and 512 frame slots. They were sized for speed when the peak was
not counted. At 256 slots each the run holds 22,528 bytes less, and the
corpus misses often enough to cost about 720,000 more instructions, 0.13%,
by the count in the table's last column:

    RECENT_CALLEES / RECENT_FRAMES   interp_peak_bytes   instructions here
    1,024 / 512                      709,850             536,115,019
    512 / 512                        697,562             536,348,115
    512 / 256                        693,466             536,843,473
    256 / 256                        687,322             536,835,497

The objective weighs the peak above the instructions: development welfare
reads 91.33 against 91.30 with the projected rows. The ratchet row
`callee_slots`, mutation `the_callee_cache_sized_for_speed`, gives the callees
1,024 slots again and the run peaks at 705,757.

CI's instruction rows for the change, which this host's gates reproduce:
`compile_instructions` 26,181,647 -> 25,842,553, `entry_instructions`
86,376,944 -> 85,213,654 and `library_instructions` 86,922,871 -> 85,769,613.
`emit_instructions` rose, 23,391,657 -> 23,409,658, 18,001 instructions. The
rise arrived with the change, and what in it moved the emitter is not
isolated.

The gate's own interpreter row for the whole change, which CI read on a
runner of family 0x6 model 0x6a and this container read the same:
`interp_instructions` 499,300,948 -> 500,281,043, a rise of 980,095
(+0.1963%). Main reads 499,300,948 here, so the rise belongs to the change
and not to the silicon. `startup_instructions` did not move.

## 2026-10-09 — a lone arm of bare names is bound without a contest

The interpreter's dispatcher spent 136 million of the corpus's 537 million
instructions in `dispatch_loop` itself, about 850 a dispatch over 159,352
dispatches. Two parts of that did no work for the answer.

A group with one arm whose parameters are all bare names has nothing to
choose between. A name refuses a failure and takes anything else, so the arm
wins exactly when no argument is a failure, and with no rival its score is
never read. Such a group now skips the candidate loop: the dispatcher checks
the arguments for a failure, pushes the names, and binds as before. A group
whose one arm holds a pattern, an annotation or a literal still goes through
the loop, and so does a call that hands a bare-name arm a failure, which
reaches the error path the same way it did.

The second part was the bookkeeping for the leader. The best candidate so far
was an `Option<(Score, &FnDecl, Bindings)>`, 80 bytes, and every candidate
that took the lead moved its score and bindings into it and the old leader's
buffers back out. The `Option` was dropped once a dispatch. The dispatcher
now keeps two score buffers and two binding buffers in fixed slots: the next
candidate matches into slot 0, slot 1 holds the leader, and a new leader
swaps them.

The first version named the leader's slot with an index that flipped, which
avoided the swap. It was slower than the swap by 3,766,635 instructions,
because every access through a computed slot paid for its address and its
bounds check, and there are more accesses than swaps.

In this container, where main reads its golden exactly:

    interp_instructions   500,281,043
    lone arm alone        496,212,125   (-4,068,918)
    with a flipping index 495,182,469   (-1,029,656)
    with fixed slots      491,415,834   (-3,766,635)

-8,865,209 together, -1.77%. `interp_peak_bytes` and `interp_allocs` read
their goldens. The browser rows do not move: neither the tab's compile nor
the program it emits runs the interpreter's dispatcher. CI's reading
replaces these if it differs.

Two ratchet rows hold this, both on the interpreter's instruction gate.
`lone_arm`, mutation `a_lone_arm_scored_like_its_rivals`, sends the lone arm
through the loop again and reads 496,945,703. `leader_slots`, mutation
`a_leaders_slot_chosen_at_run_time`, hands the score's swap a slot chosen at
run time, which is the cost the index had, and reads 492,342,137.

What led here. Priced against the objective as it stands, by lowering one
golden row 1% at a time on a scratch tree, runbench is worth 0.045 points a
percent, browser compilation 0.0033, the interpreted run 0.0025 and the
compile row 0.0004. Runbench came first and its profile offered nothing new
past what the log already declines: the decoder's key loop is 81 instructions
a key, most of them spills around an inlined map append; the number writer
already takes `ryu_short`, warm-started from the last float's place count;
and `s[i]` over multibyte text is 58 instructions a call, all of them from one
phase written to keep indexing linear. The interpreted run was next in line.

## 2026-10-09 — an in-place write keeps its container's allocation

Where `linear::in_place_pushes` or `linear::moved_writes` proves that the only
other holder of a list, map or byte buffer is a binding nobody reads again,
the interpreter used to take the contents out of the shared `Rc` and box them
in a new one: one allocation and one move for every push, put and append the
analysis cleared. It now writes through the pointer, as the compiled engine
has since kanso#1359, and hands back the `Rc` it was given. The dead binding
sees the grown container where it used to see an empty one, and it never
looks. An append of a byte buffer onto itself still takes the contents,
because writing through the pointer would read the buffer it is extending.

In this container, where the base reads its golden exactly:

    interp_instructions   491,415,834 -> 488,869,940   (-2,545,894, -0.52%)
    interp_allocs             626,007 ->     587,281   (-38,726, -6.19%)
    interp_peak_bytes         687,319 ->     687,159   (-160)

`tests/a_shared_seed_is_copied_once.rs` pins the allocations of 300 extra laps
of three builders, and the number fell from 3,647 to 2,747: one box a lap for
each builder. A ratchet row, `kept_box`, holds it. Its mutation
`a_write_in_place_boxes_afresh` boxes the pushed list again, and the spec
reads 3,047.

Literal constants folded at their use, measured and declined. A zero-arity
function whose whole body is one integer literal, such as runbench's
`span = 400`, compiles to a lazily built cell that every use checks. Emitting
the literal at each use removed the cell and its check, and the IR changed in
just those lines. digestbench fell 25.3% and deepbench 0.17%, but runbench,
the weighted row, rose 536,678 (+0.048%). Two things paid for it, and the fold
controls neither. In the escape phase, once the loop's first bound was a
constant, LLVM rewrote `n * 7` as a running offset beside a second
down-counter, one more instruction per push: escapebench rose 1,249,393. In
the rest of the run `_int_malloc`, `memcpy` and `free` rose by 3.4 million
together with every allocation counter unchanged, which is the heap laid out
differently once the cell's own allocation was gone.

## 2026-10-09 — a native build emits only what its entry reaches

Since 2026-10-08 the playground tab has dropped every function its entry
cannot reach before inference, linearity and the emitter read the program.
Native builds now do the same. Before this, a native build analysed and
emitted every function the program imported, and the prune in
`Written::finish` deleted the unreached ones from the module afterwards.
The walk is the tab's: it follows the names each body mentions from the
entry, and if the emitter asks for a function the walk left out, the
function goes back in its place and the program is emitted again. The
`only_reached` switch that kept native builds out of the walk is gone, since
every caller now passes the same answer.

Measured against goldens that read their own rows exactly, with the two
codegen rows taken from CI, whose clang and linker are the ones they count:

    emit_instructions              23,409,658 ->   7,035,904   (-69.9%)
    codegen_instructions_release  457,532,287 -> 453,747,433   (-0.83%)
    codegen_instructions_dev      118,872,382 -> 118,854,794   (-17,588)
    runbench                    1,129,588,781 -> 1,129,589,356   (+575)
    compile_instructions           25,842,553 ->  25,855,719   (+13,166)

Both tiers handed clang the pruned module before this change, so neither
codegen fall comes from less text to parse, and nothing here isolates where
they came from. The benchmark rows are pinned to one AMD part, and this
container is Intel, so each was read here on main and on the branch and the
difference added to main's golden. runbench rises 575, pendbench 700 and
encodebench 117; jsonbench falls 301 and oneshot 480; the other nine move by
two or less. An earlier draft of this entry compared this container's
reading with the golden directly and reported eleven rows falling about 347
each and deepbench rising 11,049. Those were the two machines differing, and
main reads the same offsets here.
The compile, entry and library rows rise by 13,166, 43,783 and 43,734: the
check described next, run once per record type.

The first push of this change was wrong, and the diagnostics differential
said so: `list/to_list (list/take (list/repeat 5) 3)` failed natively with
"no overload of `list/next` matches these arguments" and printed `[5 5 5]`
in the interpreter. A fuzz batch of 300 programs found six more. Two faults,
one cause. A record whose first field is an int may go back to its caller
in two words instead of a pointer, and `escape` granted that to a group when
any arm produced the record. The other arms were assumed to answer
failures, which cross the two words intact. `list/next` answers `done`
beside its `step`, and `list/iter` answers a `cursor` from one arm and a
`capped` from another; both came back read as the record. Neither could
happen while every program carried all of std/list, because functions
nobody called kept `step` and `capped` boxed. The tab has walked since
2026-10-08 and could have met the same thing.

`escape` now asks each arm of a group that answers the record. Every branch
of its tail must build the record, call a group that answers it, or end in
`err` or a call to a group whose arms answer only failures, such as json's
`fail`; an arm whose inference says it answers only failures passes as it
stands. The inference cannot do this alone, because it has one bit for
every record. Runbench keeps exactly the thirty-three two-word functions it
had, and all six fuzz programs agree with the interpreter. Two micro
fixtures hold the two faults, each red on the compiler before the check and
green after it.

While chasing the first fault I built a cache that boxed a two-word value
once per block instead of once per use, which took skip_shape from 41
allocations to 39. With the check in place `list/drop` no longer returns two
words, skip_shape reads its old 35, and no golden sees the cache, so it is
not in this change.

Two ratchet rows hold the rest. `native_reached`, mutation
`a_native_build_emits_every_group`, empties the walk's answer, and the emit
row reads 23,606,376 against 7,035,904. `parsed_arms`, mutation
`a_group_answering_other_values_goes_by_value`, skips the check, and both
fixtures go red.

Two older rows had to move. `reached_groups`, the tab's walk, patched a line
this change folded into the one both builds now share, so its mutation edits
that line and the browser row reads 446,869,426 under it. `thunk_box` went
blind: its fixture, a_lazy_record_call_nobody_reads_still_builds, reached a
lazy cell's site through a function nobody calls, and native builds no longer
emit such a function. a_lazy_record_call_somebody_makes_still_builds reaches
the same site from `play`, through a record type of its own, and clang
refuses it under the mutation with the error the first fixture was written
for.

`dead_cycle` went blind the same way. Its spec's summing program used to
emit std/list's merge cycle and rely on the prune to strike it; the walk now
leaves the merge out before anything is emitted. The prune still strikes a
group that a body names and the module never calls, and the shortest one is a
fused call: `text/to_float (text/slice ...)` is emitted as
`k_b_to_float_slice`, so std/text's `to_float` survives the walk and nothing
calls it. A third test builds that program; with every definition marked live
the module defines `d_text/to_float_1` and the test goes red.

Compiling the browser corpus in the tab rose 482,055 instructions with this
change, from 396,233,999 to 396,716,054 (+0.12%), and the interpreted run fell
32, to 488,869,908. The arm check is not the source of the first: a build with
the check skipped reads 396,716,403, 349 more. The tab already used the walk,
so its emitted modules are the same; what moved is the compiler's own code,
and nothing here isolates where. Welfare rises from 90.2314 to 90.2512.

The rows that rose, each by name and with the value it now reads:
`browser_compile_instructions` 396,716,054; `entry_instructions` 85,257,437 and
`library_instructions` 85,813,347, the arm check run once per record type;
`module_calls` 130 and `module_lines` 1,284 in the modules vein, and
`emitted_calls` 1,007 and `emitted_other_calls` 17,094 in the emitted veins,
where the line counts fell to 10,055 and 144,096 because the walk drops what
nothing calls. Of the benchmark rows, `work_runbench` reads 1,129,589,356,
`work_pendbench` 190,397,733, `work_encodebench` 2,384,337,486 and `work_basket`
32,404,398, all projected onto the recorded silicon as described above.

Three interpreter and runtime ideas, measured and declined:

Argument vectors pooled. The interpreter allocates a vector for every call's
arguments, and mimalloc spends about 63 instructions to allocate and free one.
A pool held in a `RefCell` cost about 45 instructions to take a vector and 40
to give it back, which leaves little to win. Five variants on
`bench/interp_corpus`, from a base of 539,217,785, read between 537,922,216
and 543,162,566. That spread is the layout band this row has shown before, so
none of the five is evidence of a gain. Releasing bound names one at a time
instead of clearing the vector read 494,036,859 against 491,415,834.

A two-way table of recent callees. After kanso#1813, `callee_missed` rose
from 826,313 to 2,380,466 with the cache unchanged, because the cache keys a
name by its address inside the source slice, and that address moves when the
run allocates in a different order. Checking a second slot on a miss raised
the row by 1,060,893 and the misses to 3,549,075. The misses are not pairs of
names fighting over one slot, so a second way does not catch them.

The run program's arena peak, one block lower. The peak is 3,670,032 bytes:
two full 1 MiB blocks and the index phase's 1,572,880-byte string, which needs
contiguous room. The data the run keeps is about 1.35 MB, of which 799,728
bytes are list and map buffers sized for six items and ten pairs while the
document's lists hold 3.5 and its maps 3. Smaller seeds moved nothing: the
outgrown buffers came back from the byte-buffer classes at the same total.
Sizing each container exactly would bring the kept data under one block on
paper, but a trim at close copies into fresh space and leaves the old buffer
behind, and the top-level decode never rewinds it. Exact sizing needs the
size when the container is allocated, which means a two-pass decode or a
scratch stack. Not built; a block lower would be worth about 0.17 welfare.

## 2026-10-09 — the run peak's second block belongs to the index phase

The entry above put a value on the run program's arena peak dropping a
block: about 0.17 welfare, reached by sizing the decoded document's
containers exactly. Two variants of runbench, built with `--counters` on
`daa60f90`, show that exact sizing alone would not reach it.

    variant                               arena_peak_bytes
    main                                         3,670,032
    index_chars = 1                              3,145,728
    doc = json/decode "[1]"                      2,621,456

Without the index phase's long string, the peak moves to the decode phase:
the kept document's two blocks and a third for a decode round. Without the
kept document, the peak is one block and the 1,572,880-byte string. The
index phase builds that string with `text/join [s s] ""`, and while the
join copies, its 786,448-byte input is still live and needs contiguous room
in a block. On main that room is the slack in the document's second block.

So a smaller document lowers the peak only if the document and that input
fit in one block together, which means a document under about 262,000
bytes. By arithmetic, not measurement: the kept document is about 1.35 MB,
and sizing its 799,728 bytes of container buffers to the counts the
document actually holds would leave it near 0.95 MB. The input would still
take a second block, and the peak would stay at 3,670,032. A block lower
needs both exact sizing and a join that does not hold its input beside its
result, and the second is not available while the input may have another
holder.

The same afternoon, the random-program differential ran against `daa60f90`,
the first main with kanso#1814's walk and its arm check. 1,300 generated
programs ran on the interpreter and both native tiers: 1,068 agreed, three of
them by stopping on the same runtime error, 223 were refused by the checker
the same way on every engine, 9 were rejected as malformed, and none
diverged. 250 more went through the browser engine, which
compiles in `docs/kanso.wasm` and runs on wasmi, against the native engine.
All 250 agreed.

## 2026-10-09 — the tab's translation reads the module more cheaply

The playground compiles a program to native IR text and then translates that
text to wasm (`src/ir_wasm.rs`). The translation tokenizes every line of the
module and parses the tokens into instructions. On `bench/interp_corpus`,
the program `browser_compile_instructions` compiles, that is 6,370 lines.
Three costs in it were per byte or per token rather than per line:

- `name_char` tested each byte of every name with `is_ascii_alphanumeric`
  and a four-way match. It is now a 256-entry table.
- `Locals::number` read a name like `t17` or `L4` with two `strip_prefix`
  calls, two filters and `str::parse`, and asked both prefixes every time.
  It is now one pass over the bytes that returns the prefix and the number,
  with the same rule: nine digits at most and no leading zero except `0`.
- `P::next` cloned each token out of the line, which copied every owned name
  and byte string. The parser reads each token once and only forward, so the
  token is now moved out with `mem::replace`.

    browser_compile_instructions   396,716,054 -> 389,803,790   (-1.74%)
    browser_run_instructions        33,510,708 ->  33,510,708

The run rows are byte-identical, so the module the tab builds did not change.
Natively, the same compile, emit and translation of the corpus read
129,985,288 instructions before and 126,938,798 after, a fall of 2.3%.
`browser_compile_instructions` is the one term in the objective still below
its baseline (343,917,485), and this narrows the gap by about an eighth.

The compile sweep moved one row on this host: `entry_instructions`
85,257,437 -> 85,257,442, five instructions, from the compiler's own layout,
since the native entry path does not run the translation. The two codegen
rows read what this container read for main in the entry above, so they
stay at CI's values.

## 2026-10-09 — a bignum multiplies with its shorter operand outside

A differential run through the playground's own compile path found this. The
harness hands a generated program to `docs/kanso.wasm` the way the tab does,
compiles it with `kanso_compile_native`, runs the module against the wasm32
runtime under wasmi, and compares stdout, stderr and the exit code with the
interpreter's. It is a scratch test rather than a spec, because the
generator lives outside the tree. Over 1,177 programs that touch no files,
stdout and the exit code agreed on every one. stderr differed only in the
path a trace names, `defs.kso` against `defs/defs.kso`, because the harness
hands the module's one file under a shorter name than the interpreter reads
it from. Three programs ran out of wasmi's two billion units of fuel. Five
more generators, the box, first-order, nesting, NaN and minus ones,
added 2,240 programs, and every one agreed on the same terms.

All three run in about a tenth of a second interpreted and over a second
native. Each holds a group like this one, which doubles an accumulator a
hundred thousand times and never reads it:

    fn acc46 n acc
      5 + acc46 (n - 1) (acc * 2)

The accumulator grows past 3,000 limbs, so each step multiplies a long
number by a one-limb `2`. `k_int_mul` ran its schoolbook rows over the first
operand and its columns over the second, which put the long number outside.
Every limb then paid for a whole row, its setup and its carry, to multiply
by a row one limb long: about 25 instructions a limb. It also cleared the
result before the first row. num-bigint, under the interpreter, takes about
three.

`k_int_mul` now puts the shorter operand outside. The first row writes its
products, where the later rows add to theirs, so the result is not cleared
first. The two views are swapped by pointer, because a small int's limbs
are stored inside its own view, and a swap by value leaves the short one
pointing at the other view's storage. The algorithm and the count of limb
products are unchanged.

That alone took s900624 from 4,586,777,277 instructions to 1,084,873,711,
with the multiply itself at 1,028,787,419, about 6.6 instructions a limb.
The row then went two limbs at a time as one little-endian 64-bit word, with
a 128-bit product, on x86-64 and arm64. A row that writes costs 4.0
instructions a limb that way against 6.5, and one that adds 4.8 against 9.0.
wasm32 has no 64 x 64 -> 128 multiply and keeps the limb loop. The first
attempt at the wider row put the write and the add in one loop behind a test
on the row number and read 1,498,505,558, slower than the narrow loop. As one
inlined row called with the choice fixed at each site, it reads:

    program    before          after
    s900233    4,601,351,409   710,285,391
    s900624    4,586,777,277   695,632,879
    s901212    4,551,721,733   660,655,483

The multiply falls from 3,919,227,713 to 639,546,587 on s900624. Each
program prints the same bytes as before and as the interpreter, and through
the tab all three now finish inside the fuel budget. A program that builds
3^100000 three times, by repeated `acc * 3`, falls from 5,069,017,487 to
3,214,914,845; it still costs more natively than interpreted, since
num-bigint keeps 64-bit limbs where the runtime stores 32-bit ones.

`tests/golden/micro/a_bignum_product_reads_the_same_from_either_side.kso`
asks for seven products both ways round: long by one limb, by two limbs and
by long, with every pairing of signs. Swapping the views by value turns
all fourteen lines red, and dropping the row's own word from the wide add
turns twelve of them red.

The runtime is compiled into every binary, so three veins moved. Each
benchmark's machine code grew by 1,136 bytes, and the text vein's total
went from 4,595,198 to 4,611,102. `codegen_instructions_release`
rose from 453,747,433 to 453,756,115 (+8,682) and `codegen_instructions_dev`
from 118,854,794 to 118,855,947 (+1,153). Those are CI's readings. This
container read the narrow version 488,366 below CI on the release row and
14,082 above it on the dev row, where on kanso#1816 the two had agreed, so
neither codegen row was taken from here. The objective moves by less than
the gate's 0.001 either way and the floor stays. The work vein, which counts
each benchmark's instructions, moved on four rows although none of them
multiplies a bignum: `work_encodebench` +497 to 2,384,337,983,
`work_oneshot` +308 to 13,027,092, `work_livebench` -371 to 1,473,485,624
and `work_runbench` -14 to 1,129,589,342, read by CI, with where the larger
runtime's code landed. This
container's counter sweep had read them unmoved. The allocation counters,
the lazy tier and the compile rows did not move.

## 2026-10-10 — native bignums store 64-bit limbs

Clay asked why the runtime's bignum limbs were 32 bits wide when the
interpreter's num-bigint uses 64. Nothing recorded a reason. The 32-bit form
came in with the bignum itself, `baaa3b0e`, and it is the convenient one in
portable C: a limb product fits a `uint64_t`, and Knuth's division needs a
64-by-32 divide, which C has. Both native targets multiply 64 by 64 into 128
bits in one instruction, so the convenience was costing division, printing
and multi-limb products.

The runtime now stores 64-bit limbs behind an eight-byte sign word. A limb
product is an `unsigned __int128` multiply, which wasm32 lowers to
compiler-rt's `__multi3`, and `wasm/shim.c` already supplies that. A
quotient digit comes from `k_div_2by1`, which is one `divq` on x86-64 and
Hacker's Delight's `divlu` elsewhere. That keeps any 128-bit division out of
the runtime, since wasm32's compiler-rt has no `__udivti3`. Printing divides
by 10^19 a pass and reading digits takes nineteen at a time. The copy and
rewind paths can leave a bignum's bytes at any alignment, so a limb is read
through a type that promises none.

Instructions on the dev tier, before and after, each program printing the
same bytes on both and on the interpreter:

    program                               before        after
    3000! printed                         12,994,553     7,889,063
    digit sum of 1500!, by % 10 and / 10  42,436,815    10,657,709
    gcd of the 4000th and 3999th Fibonacci
      numbers, 3,998 remainders           30,223,900    19,554,376
    2 doubled 60,000 times                238,579,642   237,443,192
    3^30000 by repeated `acc * 3`          96,093,717    95,517,656

The last two barely move because x86-64 already multiplied a long number by
one limb two 32-bit limbs at a time. The interpreter takes 395, 149 and 143
ms on the first three, and the native binaries 11, 30 and 16.

The limb functions were checked against GMP before any of this was trusted.
A harness pulls `k_mag_cmp`, `k_mag_add`, `k_mag_sub`, `k_mag_row`,
`k_div_2by1` and `k_mag_divmod` out of `src/runtime.c` as text and compares
them with `__gmpn_tdiv_qr`, `__gmpn_mul`, `__gmpn_add` and `__gmpn_sub` on
operands from one to 300 limbs. The operands are built to hit the corners:
all-ones limbs, dividends just under and just over a multiple of the
divisor, and dividends of the form (v - 1) * 2^64k plus low limbs, where the
remainder's top limb equals the divisor's after the first quotient digit.
It runs twice, once as x86-64 compiles it and once with the `divq` branch
removed, which is the code arm64 and wasm32 run. Both read 10,000,000 cases
and no failures. Removing the add-back's decrement fails 30,614 cases in
200,000, a wrong correction in the portable divide fails 105,409, and a
wrong estimate in the equal-top case fails 92,732. Skipping the refinement
loop in the equal-top case fails none; the add-back corrects every estimate
it leaves one too large, and no generated case left one two too large.

A differential fuzzer built 150 programs of random bignum arithmetic, each
with twelve operand triples from one to 4,000 bits, and ran each on the
interpreter, the dev binary and the release binary. All 150 agree.
`tests/golden/micro/a_long_division_meets_the_divisor_at_its_top_limb.kso`
divides at two- and three-limb divisors with the dividends above. Giving the
equal-top estimate a wrong digit turns its first lines red.

The entry before this one said a program building 3^100000 three times
cost more natively than interpreted because of the limb width. That was a
guess, and the change it predicted has now been made: the program's
instructions moved from 3,038,963,635 to 3,033,193,581, under 0.2%, and the
binary still takes about 1.3 seconds against the interpreter's 0.44. The
gap is memory. The binary peaks at 1,949,852 KB with 487,053 minor page
faults, where the interpreter peaks at 10,044 KB with 348. The loop
`pw (acc * 3) (n - 1)` is a self tail call that allocates nothing but its
bignum, and `beat.rs` gives a loop a rewind only when it allocates, counting
arithmetic as free. So every intermediate power stays in the arena until
the loop ends. s900624 from the entry before, which doubles an accumulator
inside a non-tail recursion, peaks at 618,392 KB with 154,187 faults for the
same reason. The claim also went into a Readwise summary and a reply to
Clay; both are corrected.

CI's first reading of the branch put the browser engine's compile at
391,614,023 instructions against main's 389,803,790, a rise of 1,810,233
(0.46%), and this host reads the same figure from the same rustc. The new
analysis was not what cost it. With `widens` answering false before it read
anything, so that every loop was judged exactly as on main, the row read
391,491,971; with the new `Beats` field alone and main's `beat.rs` otherwise,
it read 389,807,903; and moving the cheap test ahead of `crossing_positions`,
or walking the program once instead of once per loop, each moved it by less
than 200,000. Marking `group_param_set` and `tail_exprs` `#[inline]` put it at
387,724,060, which is 2,079,730 (0.53%) under main. Both helpers gained a
caller in this change, and `group_param_set` is called in
`crossing_positions`'s loop over every parameter, so the likeliest reading is
that the extra caller cost them their inlining there. That reading has not
been isolated: the measurement shows the row moving with the attribute and
nothing else.

CI's rows for the tree as pushed: `compile_instructions` 25,855,719 to
25,829,822, `entry_instructions` 85,257,442 to 85,178,721 and
`library_instructions` 85,813,347 to 85,734,452, all lower; `emit_instructions`
7,035,904 to 7,062,270, 26,366 or 0.37% higher, and `interp_instructions`
488,869,908 to 488,913,120. The emit row is where the attribute shows on
native: the codegen pass runs the helpers it inlined. The runtime rows are the
projection exactly.

The trend gate's names for what rose, each at the value it landed on:
`basket_alloc_bytes` 7,522,273; `basket_allocs` 27,264; `basket_buf_reuse`
119; `basket_sh_buf` 689,952; `branches` 53; `calls` 88; `lines` 1,589;
`interp_instructions` 488,913,120; `emit_instructions` 7,062,270;
`work_basket` 32,362,738; `emitted_other_branches` 16,032;
`emitted_other_calls` 17,104; `emitted_other_lines` 144,174; `text`
4,586,926; `a_map_whose_keys_arrived_in_order_is_its_own_view_beat_iters`
1,000; `force_path_beat_iters` 2,000; `lazy_verdict_is_per_arm_beat_iters`
2,000; `many_cells_beat_iters` 140; `returned_thunk_beat_iters` 2,000;
`stream_fold_beat_iters` 10,000;
`the_length_of_an_indexed_character_needs_no_scan_beat_iters` 4,021;
`two_bound_columns_are_one_allocation_beat_iters` 3,000.

Taken together the two sides net to a fall in welfare of 0.0000025, from
90.25702372 to 90.25702120. The floor is lowered by that much. Clay asked
why the two engines stored integers differently, and there was no reason;
this branch is the answer to that question, so the floor follows it.

## 2026-10-10 — four faults the ports found

Twenty programs were ported to kanso to see where the language gets in a
programmer's way, and their journals carry bug reductions as well as
complaints. Four of the reductions are fixed here, each with the fixture that
fails without its fix.

**A tie settled by an arm that never meets it.** `check_arm_ties` calls two
arms tied when each is more specific somewhere, unless a third arm covers
both. It compared patterns by rank alone, so `"+"` and `"eq"` ranked equal,
and an arm for `"eq"` could settle a tie that only `"+"` calls fall into.
The lox port had two arms of `apply` that could both take `"+"` with two
floats, settled on paper by an `"eq"` arm, and the two engines picked
different arms at run time. The comparison now answers "disjoint" for
patterns no single value matches: two different literals, a literal and a
marker, a literal and a type that cannot hold it, a marker and a type it is
not. A disjoint pair cannot tie and cannot settle anything.
`errors/a_literal_arm_cannot_settle_a_tie_it_never_meets.kso` is the lox
shape and passed `kanso check` on main;
`micro/arms_that_never_meet_are_not_a_tie.kso` is the other direction, two
arms main called tied that no call can reach together. The bc port's
`option` group has a real tie the old rule hid, `"--quiet" opts` beside
`arg (options ...)`, and the stricter check now reports it. So does this
repository's own `scripts/golden_prose`: `writing_back false _ _ _` and
`writing_back _ false _ _` both take `false false`, settled on paper by the
`true true` arm. Both answer 0, so the program never misbehaved; the third arm
is now `true false`.

**`[if b "x" "y"]`.** The checker refuses a named function written bare in a
list literal, since a list element is one atom, but it read `if`'s count from
a table that leaves `if` out, so the interpreter built a four-element list and
native refused with no location. `call_shaped_at` now knows `if` takes three
and reads the ambient builtins' counts. `errors/an_if_in_a_list_is_one_call.kso`.

**`\D`, `\W` and `\S`.** std/regexp read an escape it did not know as the
letter itself, so `o\W` matched an o and a capital W. The three negated
classes are now the classes `\d`, `\w` and `\s` with the negation flag set.
`micro/regexp_negated_class_escapes.kso`.

**A name bound to a lazy err.** A match arm that binds a name sees a thunk,
not what it forces to, so the thunk passes the arm's check and can still
force to an err later. Native recorded every name a pattern bound as unable to
fail, and a read that forced the thunk skipped its check: the toml port's
reduction printed `size 6`, the err's payload read as a list length, where
the interpreter stopped with `refused 5`. The pattern now keeps the FAIL bit
on a name whose value is a thunk. Inference makes the same assumption in
`bind_pattern`, but reverting it there leaves the fixture correct, so it is
left alone until a program shows it matters.
`runtime/a_name_bound_to_a_lazy_err_keeps_its_check.kso`; reverting the
codegen line turns it red.

Rows for this branch, from CI, against main as it stands after the bignum
loop rewind. Every compile row that reads the larger regexp module or the
checker's new disjointness test rises: compile_instructions from 25,829,822 to
25,832,340, entry_instructions from 85,178,721 to 85,435,525,
library_instructions from 85,734,452 to 85,988,213, and emit_instructions
from 7,062,270 to 7,062,336. compile_allocs goes from 14,529 to 14,530. The
interpreter's run asks the allocator twice more, interp_allocs 587,281 to
587,283, with its peak unchanged. The browser compiles the same library and
checker, and browser_compile_instructions rises from 387,724,060 to
388,182,464.

Four benchmark binaries changed. basket, scanbench and runbench grew and
pendbench shrank by one call and three lines, which sums to
emitted_other_branches 16,032 to 16,079, emitted_other_calls 17,104 to 17,128
and emitted_other_lines 144,174 to 144,409. The text is 4,586,926 to 4,588,958
summed, all of it in the three that grew. Runtime work rises in two of them:
work_basket from 32,362,738 to 32,382,760 and work_runbench from 1,129,588,698
to 1,129,588,845, 147 instructions. Which of the four fixes moved which binary
is not isolated here. Welfare falls, and the floor follows under the
2026-09-13 rule.

## 2026-10-10 — two native crashes the ports found in the arena

The toml port grew a list by calling a helper that pushed onto it, and the
native binary died at sixty-four elements, dev and release alike. Sixty-three
worked on every engine, and so did pushing directly in the tail call.

The pop that hands a loop's result back to its caller asks whether the
result's interior survives the rewind. For a list of `K_ISV_MIN` (64) or more
elements it asks a memo table, `k_isv_list`, keyed against the outermost
beat's mark, and the table is opened when that mark's position differs from
the one it last saw. Both start as NULL. A program whose outermost beat began
before the arena held anything compares NULL with NULL, never opens the
table, and the first lookup grows zero slots to zero and probes with a mask
of all ones. gdb put the fault on that probe. A list grown through a helper
reaches the branch because the carry repairs the list's interior every
iteration rather than copying it, so the list header is old and the pop asks
the memo about it. The table now opens on its first use as well as on a new
mark.

The fault needs an entry file, because an entry's statements are what leave
the arena empty when the first beat begins; the same functions run from a
`play` did not crash. `tests/golden/entryfile/a_list_pushed_through_a_helper_past_sixty_four`
builds a 64-element list first and a 1,000-element one after it, and
`tests/entry_file.rs` checks native against the interpreter. Removing the
new condition turns the spec red on the crash itself.

The first suspect was the arena's free shelf. The carry's copy of a list has
room for exactly its length, so every push after a carry grows the list, and
the outgrown storage is donated to the shelf when its capacity is a power of
two. Tracing every grow showed that storage was ordinary arena memory, and at
64 elements the last grow is from a capacity of 63, which the shelf does not
keep. That change was reverted.

The port's other native report, an arm that ran with an err when its sibling
had `_` in the same position, is the lazy-err check from the entry before
this one: `grown` is a thunk that forces to the err, and with that change the
binary stops at `refused 5` as the interpreter does.

The diff port's native binary died on a three-step effect chain after two
file reads, sometimes as "ran out of stack" and sometimes as `free(): invalid
pointer`. valgrind put the bad free in `k_beat_rewind_slow`, on an address
13,328 bytes inside an arena block, and a trace of the view registry showed
the path. Myers' search carries its two frontier maps through a loop and
reads them, which builds each map a sorted view. The headers live in the
loop's carry buffer, which is outside the arena, so `k_born_this_beat` calls
them new and the views register at the loop's depth. When the loop's beat
popped it copied its result out of the carry buffer and then migrated every
registration to the enclosing depth, the same as it would for a region it
keeps. A later loop at the same depth overwrote the carry buffer, and when
the enclosing depth finally rewound it flushed a header that held other
data. A map at the same address was registered again at the loop's depth
before the end, which is what the trace caught.

On a pop that copies out of the carry pair, entries whose header lies in
that depth's carry buffers are now released then and there, while the
headers are still intact, and the rest migrate as before. The permanent-
storage registry has the same shape, since it registers a field inside a
header, and gets the same treatment. The port's own reduction, a 217-line
module and an eight-line entry, is the fixture:
`tests/golden/entryfile/a_view_left_in_a_carry_buffer`, run from a scratch
directory because it writes two files. Removing the release turns the spec
red on the crash. The diff port's `check.sh` passes all 54 fixtures on the
three engines with this compiler. Three smaller programs that carry and read
maps did not reproduce the crash, because their puts happened in place and
left the maps in the arena.

The two fixes cost a little everywhere the runtime is measured, most of it in
the second registry pass a carried pop now makes. On CI's rows:

- `text` grows 1,472 bytes a binary, to 4,609,566 in all.
- `work_deepbench` rises 103,990 to 370,195,223 and `work_runbench` 14,986 to
  1,129,603,831. `work_encodebench` lands on 2,384,338,163,
  `work_oneshot` on 13,027,293, `work_widebench` on 28,094,348,
  `work_indexbench` on 2,458,507 and `work_scanbench` on 281,715, each within
  a few hundred instructions of main.
- `codegen_instructions_dev` rises to 118,867,296 and
  `codegen_instructions_release` to 453,787,838.
- `browser_run_instructions` rises to 33,519,546.

Welfare falls from 90.25811251 to 90.25801113, and the floor follows under the
2026-09-13 rule: two programs that crashed natively now run.

## 2026-10-10 — gavel: a field may hold a list of `tie` references

Clay, on whether a node built by `list/tie` may hold its links as a list: "of
course field can be an array". The 2026-10-04 gavel "a data-sized cycle is tied
with `list/tie`, and `list/tie!` insists" says a reference may be stored in a
constructor's field and nothing else. The build read that strictly: a list
literal is not a field, so `node id [(ref "b") (ref "c")]` was refused, and the
graph fixture held each node's edges as a chain of one-reference records ending
in a `no_road` marker.

**The clarification.** A field may hold any value, and a list of references in
a field is storage. The 2026-10-04 rule was about reading, and a list that
holds references reads none of them. A list or map literal whose elements are
references, written as a constructor's argument, is admitted. Everything the
rule refused stays refused: during making, nothing may index, measure, walk or
pass that list anywhere, because each of those reads through it.

Owes: the check admits a collection literal of references as a field's value,
with an error golden for a read of such a list in the maker; the graph fixture
holds each node's neighbours as a list and drops the chain of records; ch03 or
wherever `tie` is taught shows the list form.

## 2026-10-10 — a carried rewind waits for the lap to leave something behind

The mal port ran out of memory natively. Its tail-recursive `sum2` held
729 MB at two hundred thousand iterations, where the interpreter holds a few
megabytes. kanso#1824's entry left it there: mal's evaluator is one tail cycle
of about forty groups, and a cycle carried only scalars and threaded
parameters through its rewind.

Two refusals in `beat.rs` kept the cycle from rewinding at all. A carried slot
whose type set included bytes was refused, in case it hid a byte builder that
would be copied whole at every rewind. mal's value register can hold anything,
so its set includes bytes. A cluster entered by a tail call was refused if it
carried any slot, because the json string scanner once paid 8 GB of copies for
that licence. mal's evaluator is entered by a tail call. A slot that can only
be bytes is still refused, and so is a slot inference cannot type.

Lifting the bytes refusal admitted one cycle that must stay out. golden_prose
walks its directory tree through `os/is_dir here .> (d -> entered ...)`, and
`expand_tail` reads a tail inside a piped lambda as the caller's own, which is
true when the pipe inlines and false when the piped value is an effect: `k_exec`
calls the lambda later, after `onward` has returned its description. Carrying
the cycle's slots across that edge freed strings a pending continuation still
held, and the gate died in `k_concat_arr`; the runtime reports any segfault as
running out of stack, which is what it printed. A carrying cluster with an edge
through a piped lambda is now refused, and golden_prose goes red without that
refusal.

Lifting the entry refusal admitted a second. grammar_check's capture loop is a
three-member cluster entered by a tail call, and it grows its carried list with
`grown = push acc x` and hands on `grown`. Growth in a carried slot refuses a
cluster, but the check read only the argument itself, so `push acc x` written in
place was refused and the same push behind a name was carried. The gate died in
`k_deep_copy` under `k_repair_interior`, reading a heap pointer of 6, with the
lap rule and with a stage at every edge alike. The check now reads through a
name the body bound. grammar_check runs again, and
`a_cluster_growing_its_carried_list_through_a_name_stays_out` fails without the
lookthrough. I could not reduce the crash itself: a thirty-line copy of the
loop, with nested capture maps and sliced group text, carries and rewinds
cleanly. Why carrying that list corrupts a node is still open. The lookthrough
costs the tab compiler 8,440 instructions, to 390,305,438.

Lifting the refusals alone made mal ten times slower. A cluster rewinds at
every edge between its members, and mal crosses about fifty edges for each
call it evaluates. Each rewind sized the carried slots, copied them into the
carry pair and reset the arena: 0.47 s at ten thousand iterations against
0.067 s with no rewind at all. Near eighteen thousand iterations tenure
opened its tenth block and the copy per rewind began to grow with the count,
so forty thousand iterations took 7.2 s.

Compiled loops now call `k_beat_lap_carry` in place of `k_beat_iter_carry`. It
stages only once the arena has drifted a quarter megabyte past where the last
stage left it, in the same block, with nothing in the depth's registries. That
is the test the bind chain already applies to its own steps. Skipping a stage
is always sound, since the carried values stay where they are and the next
stage copies whatever is still reachable; the test bounds the garbage held in
between. The registries are asked because their storage is malloc'd and the
block test cannot see it grow. A first version that skipped while more than
256 KiB of the block remained raised the run program's `perm_peak_bytes` from
32,200 to 81,544, and pend's 450 KB laps pushed it into a second arena block.
The cohort pop and the chain still call `k_beat_iter_carry` directly.

mal at two hundred thousand iterations now runs in 0.68 s at 21 MB, against
1.15 s and 729 MB. It still grows by about 80 bytes an iteration: 209,210 map
views are built and 10,519 freed, because a view is registered for release only
when its map was allocated in the current beat. The next entry fixes that.

`tests/golden/mem/an_evaluator_carrying_any_value_round_its_cycle_rewinds`
builds the shape in forty lines and pins `arena_peak_bytes` at 1,048,576 for
twenty thousand steps; main's compiler holds 4,194,304 for it and never
rewinds. Putting either refusal back turns it red, and so does staging at
every edge, which copies 4,754,864 bytes where the lap rule copies 3,488.

Two cost veins move with the lap rule. The run program's `evac_bytes` falls
from 10,018,720 to 6,671,008 and its tenure from six blocks to one, and the
basket program's carried rewinds stage less often. Of the tab compiler's rise
in `browser_compile_instructions`, 1,710,393 arrived with the lambda-edge
refusal. The next entry prices the whole change on CI's rows.

The ratchet's tenure-sharing row gated on the run program's counters, and with
one tenure block there instead of six the mutation stopped moving them. It now
gates on the mem corpus, where
`an_inner_beat_opens_its_tenure_in_the_block_outside` goes red.

## 2026-10-10 — a map's view belongs to the beat that frees the map

A map read by key, rendered, compared or measured gets a sorted view, and the
view is malloc'd. It is released when the beat whose rewind reclaims the map's
header rewinds. Until today the view was registered for release only when the
map was born in the innermost beat. A map built in an outer loop and read
inside an inner one was never registered, so its view was never freed. mal's
evaluator builds its environments in its tail cycle and looks symbols up one
region call further in, and at two hundred thousand iterations it built
209,210 views and freed 10,519.

`k_viewreg_own` now asks each depth from the innermost outward whether its
rewind reclaims the header, and registers the view at the first that does. A
header outside every arena block, in a carry buffer or a tenure block,
registers at the innermost depth, which rewinds no later than that storage is
released. A frozen constant keeps its view for the life of the program.

That alone made mal fifteen times slower. The lap carry stages whenever a
registry at its depth holds anything, and mal's views now registered at the
cycle's own depth, so every edge staged again. The view registry no longer
forces a stage. A view is never larger than its map, and the map is either
garbage the drift test already counts or still live, so a skipped lap holds no
more view memory than the test bounds.

mal now holds 9 MB at two hundred thousand iterations and at four hundred
thousand, where lap carry alone held 21 MB and 39 MB. It runs in 0.63 s and
1.26 s, the same as lap carry alone.

Two fixtures pin the two halves.
`a_view_built_one_beat_down_is_freed_by_the_beat_that_built_its_map` builds
twenty thousand views one beat below their maps and frees all of them; with
the old registration it frees none and holds 2,240,000 bytes of views.
`a_cycle_that_sorts_a_map_each_step_still_waits_for_its_lap` copies 6,224
bytes in its carried rewinds; letting the view registry force a stage copies
3,200,304.
Merged with main's constant storage it copies 6,128, since the constants it
names are no longer copied, and its one tenure block is gone. The evaluator
fixture's lap rule copies 3,360 where it copied 3,488, for the same reason.

The native cost veins and the lazy tier do not move: no benchmark builds a
view one beat below its map. The tab's run does. `bench/interp_corpus` builds
1,320 views and now frees them all, so `browser_run_instructions` rises
517,253 to 34,038,136 and `browser_run_peak_bytes` falls 131,072 to 1,179,648.
Natively the same corpus costs 336,930 more instructions, about 240,000 of
them in glibc's `free`. The browser side of welfare rises from 83.42 to 83.55.
A fast path that registered a head-block header without the walk cost more on
this corpus than it saved, because its maps sit below the innermost mark, and
was dropped.

CI's rows for the three pieces together, against main. The work rows that fall
are `work_runbench`, 1,129,603,831 to 1,127,640,368, and `work_basket`,
32,382,758 to 30,339,466. The ones that rise are `work_encodebench` by 2,866 to
2,384,341,029, `work_oneshot` by 2,810 to 13,030,103, `work_indexbench` by 514
to 2,459,021, `work_scanbench` by 309 to 282,024 and `work_livebench` by 3,146
to 1,473,488,334: none of them carries a cluster, and the rise is the larger
runtime each binary links. The run program's tenure falls with its staging:
`run_ten_blocks` from 6 to 1, `run_ten_frees` from 6 to 1 and
`run_ten_handups` from 2 to 1. `text` grows
1,072 bytes in every binary, to 4,624,574 in all. `codegen_instructions_dev`
rises 8,776 to 118,876,072, `compile_instructions` 25,851 to 25,858,191,
`entry_instructions` 79,015 to 85,514,540 and `library_instructions` 79,237 to
86,067,450, for the slot checks and the lambda-edge marking in `beat.rs`.
`codegen_instructions_release` falls 15,674 to 453,772,164, `emit_instructions`
4,229 to 7,058,107 and `interp_instructions` 47,893 to 488,865,227. The tab
reads `browser_compile_instructions` 390,305,438, up 2,122,974, with
`browser_compile_peak_bytes` up 1,998 to 675,725, `browser_run_instructions` up
518,590 to 34,038,136 and `browser_run_peak_bytes` down 131,072 to 1,179,648.
Welfare rises from 90.25801113 to 90.26974705, and the floor follows it.

While building the fixtures I found that a file imported from a
subdirectory, `import "./lib/fx"`, loses its cycle's rewind: the evaluator
fixture above holds 4 MiB and never rewinds that way, and 1 MiB with 40,002
rewinds imported as `./fx`. Self-loops are unaffected. main reproduces it.
That is the next piece.

## 2026-10-10 — a survival question about an old block asks an index

The lap carry made the lox port's F18 loop quadratic. Its 40,000 passes took
2.5 s on main and 30.6 s on this branch, and 80,000 took 152 s. Memory was the
same on both, 2.3 GB at 40,000 passes, because lox's heap only grows; F18
itself is still open.

The time was in the survival checks. `k_survives` and `k_where` answer "does
this pointer outlive the rewind" by finding the arena block that holds it, and
they found it by walking the chain of blocks from the mark's block towards the
oldest. Lox holds every object it ever made, so by pass 10,000 the chain was
375 one-megabyte blocks long, and every check on an old object walked most of
it. Main asked few of those questions in this program. The lap carry admits
the clusters lox's evaluator runs in, so each stage and each carried pop asks
one for every carried node, and the walk grew with the heap. Rebuilding with
64 MiB blocks put the 40,000 passes at 1.9 s, which isolated the walk.

The live chain is now also kept sorted by address, and each block's header
records its position in the chain. Once the chain is longer than eight blocks
the two questions binary-search the index and compare positions. Shorter
chains keep the walk, so the programs in the benchmark corpus, none of which
reach eight blocks, take the same path as before. Lox runs 10,000, 20,000 and
40,000 passes in 0.47, 0.97 and 2.12 s, against main's 0.58, 1.26 and 2.48.
mal's 200,000 iterations still run in 0.66 s at 9 MB.

`chain_finds` counts the questions the index answered, and is zero on every
benchmark. `tests/golden/mem/a_long_chain_answers_survival_from_its_index`
keeps fifteen blocks of rows live under a carrying loop and reads 477,919;
raising the threshold past any chain reads 0 with every other counter equal,
and the ratchet row "a long chain walked instead of searched" makes that
mutation.

CI's rows for the branch with the index in it, after main's constant storage
merged in, against main: `work_runbench` 1,131,250,979 to 1,129,336,309,
`codegen_instructions_release` 453,707,138 to 453,867,566,
`codegen_instructions_dev` 118,896,762 to 118,918,413 and `emit_instructions`
7,062,299 to 7,058,075. The browser compiles in 390,304,777 instructions, up
2,122,974, and `browser_run_instructions` lands on 34,089,215, up 596,152. Every
benchmark's machine code is about 7,900 bytes larger than main's, so `text`
rises to 4,722,910 in all. Moving the index's two branches out of line saved
960 of those bytes and was not kept, so most of the growth is somewhere other
than the index; where has not been measured. The book's two counters samples
gain the `chain_finds=0` line.

Nine of the fourteen work rows rise, and `runbench`, `pendbench`, `widebench`,
`deepbench` and `basket` fall. The rises are small: `work_oneshot` lands on
13,207,117, up 30,630, `work_encodebench` on 2,383,882,573, `work_jsonbench` on
860,558,623, `work_livebench` on 1,472,862,368, `work_digestbench` on
5,561,968, `work_indexbench` on 2,458,407, `work_scanbench` on 269,663,
`work_readbench` on 4,577,482 and `work_escapebench` on 39,045,847. None of
those nine runs a loop the lap carry admits, and what moved them has not been
isolated. `basket` falls by 1,969,638 to 30,405,476, and its survivor slots
drop from 16,002 to 6 in the same change. Welfare rises to 90.26 and the floor is raised to hold it.

## 2026-10-10 — a directory called lib is an ordinary directory

The beat analysis keeps an imported library's loops out of the carry tier,
because a shared driver threads its caller's invariant source through the loop
and carrying it would copy an unbounded value at every rewind. It recognised a
library by its declaration's file name, which had to begin `std/` or `lib/`.
Shipped modules are filed as `std/<module>/<file>`. Nothing shipped begins
`lib/`, and every user directory called `lib` does, so a package kept in one
compiled to a program that never reclaimed a block. The 2026-08-31 entries
found this, measured it at five times the peak, and pinned it in
`tests/a_program_is_not_its_directory.rs` with the instruction to delete the
assertion when the two directories agreed.

The `lib/` arm is gone and the `std/` arm stays. The arm had one user in the
tree: the json beat unit test compiled `lib/json` as a root, and the arm made
that root look like an installed module. The test now reads the library
through `import "std/json"`, which is how a program meets it, and it still goes
red when the `std/` arm is removed: `json/array_open` and `json/obj_open`
become beats. The spec now asserts that `lib/app` and `elsewhere/app` both
peak at 1,048,576, and with the arm put back the `lib/` side reads 5,242,880.

I found it while building the fixtures for the lap carry. The evaluator
fixture, imported as `./lib/fx`, held 4 MiB and never rewound.

The tab compiler reads 661 fewer instructions, `browser_compile_instructions`
388,181,803. CI moved one other row: `emit_instructions` falls 37, to
7,062,299. Both codegen tiers held. The ratchet's mutation for this filter
patched the old two-arm line, so it now patches the one-arm line.

## 2026-10-10 — a constant lives in storage the carry knows is permanent

A map built before a carrying loop came out of the loop with keys the next
round's rewind freed. `tests/golden/micro/a_map_kept_across_a_carried_loop_keeps_its_keys`
is the shape: `spin` carries a map made by its caller, an outer loop calls
`spin` four times, and native printed `<none> <none>` where the interpreter
prints both values. Main has the defect; it does not depend on the lap carry.

The keys were string literals. The runtime builds a literal once, in malloc'd
storage, and hands the same string out for the rest of the run, and does the
same for one-character strings, for marker records with no fields and for
closures over nothing. None of that storage lies in an arena block, and the
copy walk answers "does this survive the rewind" for an outside pointer by
asking whether it is a short token, a tenure block or a frozen CAF. A literal
is none of those, so the walk took it for something it had to rescue, which
wasted a copy at every stage and did damage in one place. When a surviving
node points at something that does not survive, the walk leaves the node
where it is, copies what it points at, and settles the copy in the
current loop's region. The map's keys moved into `spin`'s region, the map
stayed in its caller's, and the outer loop's next rewind freed the keys under
it.

`k_alloc_const` now hands those constants out of chunks registered with the
frozen ranges, so the walk counts them as surviving, and the walk stops at
a node in one of those chunks without asking about its interior, since nothing
writes to a constant after it is built. The one-character and wide caches
keep their bytes inline, where they had a second `malloc`. A frozen CAF is
left out of the short stop: a proven in-place push can move a frozen list's
items out of frozen storage, and pendbench does exactly that, so a frozen list
still gets the repair it needs. Bignums moved to permanent storage stay on
`k_alloc_perm`, because they are the run's data and every outside pointer is
checked against the frozen ranges.

This is also why grammar_check died in `k_deep_copy` on the lap carry branch
with the growth lookthrough taken out, the crash the 2026-10-10 lap carry
entry left unexplained. The capture loop carried `caps`, a map from the
grammar file whose keys `"1"` to `"4"` come from the one-character cache.
The first stage repaired the map by moving its keys into the loop's region,
the region was reclaimed, and the next stage copied a key whose header the
arena had reused. With constants registered, that build of grammar_check
passes with the lookthrough removed. The lookthrough stays, because it
refuses a cluster for what growth would cost.

Every runtime counter that moved fell. `evac_allocs` and `evac_bytes` fall on
every benchmark, the run program's by 14,328 copies and 229,376 bytes, and
`carry_dedup` goes to zero wherever it was counting the same literal reached
twice. The cohort pin in `tests/cohort.rs` falls 96 bytes, to 800,672, because
three of the constants it used to copy are no longer copied.

The chunks start at 4 KiB and double up to 64 KiB. The first build opened one
64 KiB chunk, and CI showed the browser run taking one more 64 KiB page for it,
1,310,720 to 1,376,256 bytes. Most programs hold a few dozen constants: the
run program has seventy, in one chunk. With the small first chunk the browser
peak stays at 1,310,720 and its run falls 26,483 instructions.

The fix costs welfare 0.0065, which the floor gives up because the defect was
a miscompile. `runbench` rises 1,647,148 instructions, 0.15%. The copy walk
itself got cheaper by about 2.8 million: `k_copy_size`, `k_deep_copy`,
`k_copy_map_put` and `k_copy_alloc` together. What rose is `realloc`, under
`k_buf_perm_regrow`. That function makes the same 3,872 calls on both sides,
and they cost 561,440 instructions before and 3,248,608 after, because realloc
now copies buffers that it used to extend in place. The run makes seventy fewer
small mallocs, which changes where glibc puts each accumulator's buffer. The
location of the cost is measured. Why glibc places the buffers differently is
not established. Opening the constant chunk before anything else is allocated
left the count unchanged to the instruction, so the chunk does not sit in the
way. `encodebench` and `livebench` fall about 375,000 each with the smaller
chunk, and `deepbench` falls 4.4 million. Release codegen falls 80,700
instructions and dev codegen rises 29,466; each benchmark's text grows 192
bytes.

By key: `work_runbench` lands on 1,131,250,979, `work_oneshot` on 13,176,487,
`work_jsonbench` on 860,548,891, `work_escapebench` on 39,045,280 and
`work_widebench` on 28,094,541. `codegen_instructions_dev` lands on 118,896,762
and `text` on 4,612,254 across the fourteen binaries. `carry_dedup` falls to 0
wherever it counted: `run_carry_dedup` from 71, `encode_carry_dedup`,
`oneshot_carry_dedup`, `wide_carry_dedup`, `digest_carry_dedup`,
`read_carry_dedup` and `live_carry_dedup` from 3, and in the lazy tier
`a_maps_two_columns_are_one_allocation_carry_dedup` from 1,998 and
`a_nested_map_gives_back_its_entries_carry_dedup` from 29,990. A dedup is a
copy the walk found it had already made; constants are no longer copied, so
there is nothing to find twice.

## 2026-10-10 — a node an inner pop repairs stays on the outer loop's list

The lox port's `tests/functions.lox` crashed natively once kanso#1824 let its
evaluator loops carry. The cause is older than that branch. A record built
before the evaluator's loop held a string in the outer loop's tenure block.
An inner loop returned a value that reached the record, and its pop copied
the result out of its carry pair. The pop's walk counts nothing outside the
arena as a survivor, so it copied the string to the arena's frontier and
repaired the record in place to point at the copy. The frontier was in the
evaluator's lap. At the evaluator's next stage the walk reached the record
through an older node whose own fields all survived, stopped there, and
rewound the lap with the copy in it. The record then pointed at freed memory.

Main's compiler crashes on the reduction too, so the bug did not need
kanso#1824. `tests/golden/mem/a_node_repaired_by_an_inner_pop_outlives_the_outer_stage`
is three nested loops and a box built before the middle one. Built against
main's runtime it segfaults, and its stdout golden fails.

The pop now keeps the nodes it repaired that lie below the enclosing mark, on
a list beside `k_repaired`. Each stage walks the entries below its own mark as
extra roots before it rewinds, sized with the carried slots, so a node that
points above the mark is repaired into the carry and settled back under the
raised mark like any other. That makes the node safe for the depth that
staged and not for the one outside it, so an entry below the outer mark stays
on the list, and so does a node the stage itself repaired below the outer
mark. A rewind frees what lies above its mark, and the list must not keep an
address that has been handed out again. While the list holds anything, a bit
in `k_buf_dirty` sends every rewind to the slow path, and the slow path drops
the entries above its mark, and a rewind to the outermost mark clears the list,
since no stage outside it will revisit anything. The fast path reads nothing
new.

I first tried narrowing the pop's walk instead: an enclosing depth's tenure
outlives the pop, so the pop could leave a pointer into it alone. That fixed
`functions.lox` and broke `binary_trees.lox`. The lox interpreter writes into
tenured environment maps in place, so an enclosing tenure node can hold a
pointer into the inner carry pair, and the copy-out has to see it.

With the fix, main's compiler passes all 28 lox fixtures on the interpreter,
a dev build and a release build. On a tree carrying kanso#1827 and kanso#1824
as well, the same 28 pass under the lap rule and with every edge staging. The
twelve cost veins and the mem tier are unchanged apart from the new fixture's
own file: no program in the corpus repairs a node below an enclosing mark.

The list costs something when it is empty. `k_beat_iter_carry` now asks its
depth for an outer mark and walks the nodes it repaired against it, and the
pop does the same, at about fourteen instructions a call whether anything is
pushed or not. deepbench stages 52,310 times and pops 52,003 times, so it pays
the most, 0.40%. The runtime's text is 2,208 bytes larger in every binary,
which the `text` total and both codegen rows carry. CI's rows, old to new:

    work_deepbench               365,792,692 -> 367,269,411
    work_basket                   32,375,114 ->  32,505,156
    work_runbench              1,131,250,979 -> 1,131,470,937
    work_pendbench               190,390,436 -> 190,396,306
    work_livebench             1,472,859,260 -> 1,472,861,300
    work_jsonbench               860,548,891 -> 860,549,545
    work_encodebench           2,383,861,305 -> 2,383,861,896
    work_digestbench               5,560,569 ->   5,561,143
    work_indexbench                2,456,650 ->   2,457,185
    work_scanbench                   268,468 ->     268,793
    work_widebench                28,094,541 ->  28,094,810
    work_readbench                 4,577,050 ->   4,577,104
    work_oneshot                  13,176,487 ->  13,176,528
    work_escapebench              39,045,280 ->  39,045,300
    text                           4,612,254 ->   4,643,166
    codegen_instructions_dev     118,896,762 -> 118,918,430
    codegen_instructions_release 453,707,138 -> 453,994,118
    browser_run_instructions      33,493,063 ->  33,507,353

Welfare fell by less than a hundredth, and the floor moves down with it under
the rule for a change that makes the language do what it says: the program
crashed.

**Main's repair list, merged into the lap carry.** The two branches touch
different parts of the carry, and the code merged without a conflict. One golden
moved. `a_node_repaired_by_an_inner_pop_outlives_the_outer_stage.mem` was
written before the lap carry, when every edge staged, and under the lap carry
the same program stages less: `evac_allocs` 237 -> 121, `evac_bytes` 8,288 ->
4,128, `survive_slots` 82 -> 40, `allocs` 302 -> 246, `alloc_bytes` 7,088,284
-> 7,086,268, and the new `chain_finds` row reads 0. Its output is unchanged.
The fixture still does its job on the merged tree: with
`k_pend_keep_repaired` returning at once, the program segfaults. The goldens
both branches moved were combined as the sum of the two deltas, and CI's rows
for the merged head follow.

CI's rows for the merged head. The projection from the two branches' deltas was
close but not exact: each benchmark's text is 672 bytes larger than the sum, basket
runs 129,985 instructions fewer, deepbench 104,915 more and runbench 14,885 more.
Against main, every row that worsened, with the value it landed on:

    run_ten_frees                                6 ->               1
    run_ten_handups                              2 ->               1
    compile_instructions                25,832,340 ->      25,858,191
    entry_instructions                  85,435,525 ->      85,514,540
    library_instructions                85,988,213 ->      86,067,450
    codegen_instructions_dev           118,918,430 ->     118,945,861
    codegen_instructions_release       453,994,118 ->     454,057,202
    browser_compile_instructions       388,181,803 ->     390,304,777
    browser_compile_peak_bytes             673,727 ->         675,725
    browser_run_instructions            33,507,353 ->      34,098,461
    work_digestbench                     5,561,143 ->       5,562,545
    work_encodebench                 2,383,861,896 ->   2,383,883,896
    work_escapebench                    39,045,300 ->      39,045,867
    work_indexbench                      2,457,185 ->       2,459,004
    work_jsonbench                     860,549,545 ->     860,559,280
    work_livebench                   1,472,861,300 ->   1,472,863,852
    work_oneshot                        13,176,528 ->      13,207,890
    work_readbench                       4,577,104 ->       4,577,539
    work_scanbench                         268,793 ->         270,025
    text                                 4,643,166 ->       4,763,230

The lap carry stages once per 256KB of drift where it staged at every edge, which
is what the work rows above pay for; the runtime grows by the chain index and the
lap bookkeeping, which is the text row. Against that, runbench falls to
1,129,571,152, basket to 30,405,533 and deepbench to 366,705,890, the run
program evacuates 6,441,632 bytes where it evacuated 9,789,344, and the browser
run peaks at 1,179,648 bytes. `run_ten_frees` and `run_ten_handups` count work on
tenured blocks, and they fall because the program now tenures one block where it
tenured six. Welfare rises from 90.2505 to 90.2618, and the floor is raised to
hold it.

## 2026-10-10 — a `tie` node holds its references in a list

Built the gavel "a field may hold a list of `tie` references". The maker check
in `src/check.rs` walked a constructor's arguments with a flag saying each one
was a field, and handed every other expression's children down with the flag
cleared. A list literal was one of those other expressions, so `ref` called
inside `town "ash" [(ref "birch") (ref "cedar")]` was refused as a read. A list
literal or map literal that is itself a field now passes the flag to its
elements, and for a map to its values only, since a key is read to place its
pair. Anything else that holds the list is refused as before: binding it to a
name first, measuring it, indexing it or handing it to a function.

Neither engine needed a change. The interpreter's `resolve_tied` and native's
`k_tie_resolve` already walk lists and maps when they swap each cell for its
node, because a record field's value was always allowed to be any value once
the tie returned.

The diagnostic now says the reference goes straight into a field "alone or in a
list or map written there", and the four tie goldens that quote it moved with
it. `tests/golden/micro/a_graph_holds_its_roads_in_a_list` keeps each town's
roads as a list and a map of named roads beside it, and prints the same text on
the interpreter, a dev build and a release build; the compiler on main refuses
it at `town "ash" [(ref "birch") ...]`.
`tests/golden/errors/a_tie_maker_measures_a_list_of_references` binds the list
to a name and measures it, and is refused at both references.

`a_graph_tied_from_a_map_of_edges` keeps its chain of `road` records. Its edges
come from a map, so its list of references would be built by a call, `list/map
names ref` or a fold, and a reference handed to a function from std is out of
the check's sight. The gavel's text admits a literal, and the ruling's own
Owes asked for this fixture to drop the chain, which it cannot do under that
text. That question has gone to the ledger as "May a list built from data hold
`tie` references?". `docs/compiler.html` §193 says which form each case takes.

The book teaches cyclic structures with `build` and does not mention `tie`, so
the place `tie` is taught is its own comment in `lib/list/list.kso`. That
comment now names the list form and shows `node id [(ref "b") (ref "c")]`.

CI's rows for this branch, with main's repair list merged in. The check walks
two more expression kinds in a constructor's arguments, and `lib/list/list.kso`
carries two more comment lines, which every compile of the library lexes:

    compile_instructions             25,832,340 ->  25,832,534
    entry_instructions               85,435,525 ->  85,436,634
    library_instructions             85,988,213 ->  85,988,060
    browser_compile_instructions    388,181,803 -> 388,192,201

The ruling is a part of the language, so the floor follows whatever these cost.

## 2026-10-10 — a constructor handed over as a function builds

The lox port found that a record constructor passed where a function is wanted
checks and runs on the interpreter and stops the native build with "`pair` as
a bare value is not yet supported" (lox F20). The port wrapped each one in a
lambda, `(l o r -> logical l o r)`, which is what native now does by itself.
When codegen lowers a name that is not a function group, a builtin or one of
the words with a value of its own, and the name is a record type with one to
four fields, it lowers the lambda that calls the constructor. Four is the
native lambda's limit, so a record of five or more fields handed over is still
refused at build with the same message.

`tests/golden/micro/a_constructor_handed_over_as_a_function` hands a
one-field constructor to `list/map`, a two-field one to a function that calls
it and a three-field one to another, and prints the same line on the
interpreter, a dev build and a release build. The compiler on main refuses to
build it. The ratchet row "a constructor refused as a value" puts the refusal
back and the micro corpus goes red.

`tests/a_type_name_as_a_bare_value.rs` had pinned the refusal for a
two-field record printed bare. It now pins that both engines print `<fn>`.

CI's rows for this branch, with main's repair list merged in. Two rows moved,
both by the size of the new lowering in the emitter:

    emit_instructions                7,062,299 ->   7,062,491
    browser_compile_instructions   388,181,803 -> 388,266,184

Welfare falls from 90.2505 to 90.2504, and the floor follows it down under the
rule for a change that makes the language do what it says: a program that
checks and runs on the interpreter now builds natively.

The tie-list ruling landed first. Carried over it, the browser compile row
lands at 388,276,582, the constructor's 84,381 on top of main's 388,192,201,
and the floor at 90.25039901483147.


## 2026-10-10 — two values a carry handed back while they were still held

The lap carry (kanso#1827) let the toml port parse a comment holding a
control character and then lose the error it meant to report. The program
died with an unhandled err whose record had no type, born in `toml/fail`.
The interpreter printed `toml/toml_error 10 1 "control character 7 in
comment" "a = 1 # xy"`.

The bug is older than the lap carry. `end_at` reads the byte after a `#`,
calls `skip_comment`, and hands what comes back to `newline_at` in tail
position. The two form a loop that rewinds the arena on the edge back into
`newline_at`. The cursor crosses that rewind uncopied, because inference
counts a slot that holds an int or an err as one that may cross a rewind
the way a number does. On the lap that meets the control character,
`skip_comment` answers an err wrapping a `parse_failure` record, and both
were built in the lap's own storage. The rewind handed that storage back,
`newline_at` answered the err by adding a hop to it, and the hop was
allocated over the record. A hardware watchpoint on the record's type word
caught `k_err_hop` writing it. Main has the same hole: the fixture below
dies on main's compiler in the same way. toml reached it only once #1827
admitted its loop, which main refused because the loop's carried byte may
be bytes.

The carry path already had the answer. `k_beat_iter_carry` returns without
rewinding when a carried slot holds an err, since the callee only passes
the failure on. A plain rewind asked nothing of the arguments it lets
cross. Codegen now tests each argument whose set includes a failure, and
skips the rewind when one of them is an err. That applies on a carrying
edge too, for the arguments the carry does not take. The test is a tag
compare and a branch at each rewinding edge where such an argument exists.

`tests/golden/micro/an_err_built_in_a_lap_outlives_its_rewind.kso` is the
toml loop cut down to three functions. Main's compiler prints `unhandled err
... record born in f/skip`. With the test in place it prints `failed at 3:
stopped` on both engines and in a release build.

The toml port's `many-tables` document showed the second fault once the
first was fixed: the last header's path came back as the path of a key read
two lines later. A carry stage copies its slots in order, and the first time
the walk meets a node that has already been through one copy it switches to
tenured storage for the rest of the stage. A node copied into the carry pair
before that switch could then be shared by a node tenured after it, through
the map that makes a node shared twice copy once. The tenured parent
survived the next stage, the stage's walk stopped there because everything
the parent held directly survived too, and the stage after that wrote over
the pair the shared node lay in. Main has this one as well. When the map
answers with a copy in the pair and the walk is tenuring, the node is now
copied again into tenure and the map points at that copy from then on.

`tests/golden/micro/a_tenured_node_shares_nothing_with_the_carry.kso` builds
the shape directly: a loop carrying a fresh list, last lap's tree, and a new
tree node wrapping the list, with enough garbage per lap that every lap
stages. Main's compiler prints `[200]` in three places where `[400]`, `[600]`
and `[800]` belong.

Neither fix moves an allocation counter. The test before a rewind adds a
compare and a branch at each rewinding edge with an argument that may be an
err, which the emitted code shows: runbench gains 22 branches and 125 lines,
the decoder 4 branches and 15 lines, and seven other benchmarks between 2 and
13 branches.

CI's rows, measured with main's constructor change merged in. The code the
compiler writes grows with the new tests: emitted_branches lands on 1,105 and
emitted_lines on 10,070 for the decoder, emitted_other_branches on 16,143 and
emitted_other_lines on 144,731 across the other benchmarks, and the machine
code total, text, on 4,644,478 bytes, from 16 bytes more on scanbench to 368
on runbench. The work rows move with the change, and which branch costs what
has not been isolated: work_encodebench lands on 2,393,926,456, up 10,064,560
or 0.42%; work_runbench on 1,131,539,079, up 68,142; work_digestbench on
5,597,963, up 36,820; work_jsonbench on 860,549,995 and work_oneshot on
13,176,892, up 450 and 364; and livebench falls 259. Building costs a little
more: codegen_instructions_dev lands on 118,918,569, up 139, and release
codegen falls 8 to 453,994,110. In the browser, browser_compile_instructions
lands on 388,494,007, up 217,425, and browser_run_instructions on 33,508,472,
up 1,119. Both changes fix what programs print, so the floor follows them,
from 90.25040 to 90.24995.

## 2026-10-10 — gavel: `build` retires, and `tie` is the one way to write a cycle

Clay, on the ledger entry "Does `build` still earn its place beside `tie`?":
"retire build". The entry recommended it once `tie` was built and its
compile-time check on literal data was pinned, and both are on main: `list/tie`
runs on every engine (log, 2026-10-05), a broken link in literal data is
refused at check, and ten thousand tied rings dropped in a loop hold one arena
block.

**What leaves the language.** The `build` block, the hole `_` that only a
`build` block could fill, the field write that filled it, and the
`error[build]` family that policed all three. A cycle is written with
`list/tie` or `list/tie!`, whether its nodes are two names in the source or a
graph read from a file. The 2026-10-01 gavel's reason for there being no loop
construct is unaffected, and so is the top-level constant knot
(`ring = node 1 ring`), which is a separate construct and is not part of this
ruling.

**Why.** `tie` does everything `build` did. With literal keys and links the
compiler sees the whole graph and refuses a broken link before the program
runs, which was the one thing `build` caught earlier, and a tied call's nodes
are one birth cohort, so the birthday theorem and the cohort's memory
guarantee hold as they did for a block. Two spellings of one thing is what
kanso declines to offer.

Owes: the parser, the checker and all three engines drop `build`, `_` and the
field write; the `error[build]` goldens and `tests/golden/mem/build_cycle.kso`
move to `tie` or go; every `build` in the corpora, the samples, the examples
and lib is rewritten as a `tie`; ch03's "two records that point at each other"
teaches the cycle with `tie`; appendix A loses `error[build]`; the compiler
page and the playground stop offering it. A program that still writes `build`
gets one diagnostic naming `list/tie` as the replacement.

## 2026-10-10 — the four port fixes' floor question closes on the corpus-first gavel

The ledger's Blocking entry "May four port fixes lower the welfare floor?",
filed today, asked whether kanso#1820, kanso#1822, kanso#1824 and kanso#1825
may each lower the floor by a few thousandths. Each fixes a native cliff a port
journal found (sat's map put loop, lox's loop entered from a tail cycle,
`list/tally` over many distinct strings, mustache's fold onto its accumulator),
and each pays only in compile rows because no benchmark contains its shape.

That is the case the archive's 2026-09-05 "gavel: corpus first — a blind
corpus is repaired, never excused" ruled. Clay declined, then, the
recommendation to move the floor with "the corpus is blind" as the reason:
**the remedy is to add the benchmark the objective could not see, measure its
baseline on the pre-fix code, land the fix on top, and let it score**, so the
floor rises rather than falls. The entry's search looked for a ruling on fixes
that make native finish what the interpreter finishes and missed this one,
because the ruling is about what the corpus can see. The entry leaves the
ledger in this commit on that citation.

So no floor moves for these four. Each PR promotes its fix's shape into the
benchmark corpus as a run-speed or run-memory phase under the granted-baseline
machinery, with its baseline measured on main without the fix, and banks the
rise with `--set`. The entry's option 1 also asked to widen the 2026-09-13
exception to every change that lets native finish a program the interpreter
finishes; under corpus-first such a change scores as a gain once the corpus
holds its shape, so the widening is not needed, and it is not made here.

## 2026-10-10 — four port shapes join the run program, measured before their fixes

The corpus-first gavel, applied today to kanso#1820, kanso#1822, kanso#1824 and
kanso#1825, says none of the four lowers the floor: the shape each fixes goes
into the benchmark corpus first, measured on the compiler without the fix, and
the fix then scores against it. This entry is the first half. The run program
gains four phases, each the PR's own reduced fixture with its count lifted into
a parameter, and nothing in the compiler changes. The columns are
instructions on main with each phase built alone, less the empty program's
6,662,610, and the shares are of the whole program's 1,245,042,917.

    scatter  2,340 keys counted in scrambled order   50,901,814  4.09%
    bump     350 updates through a named read        53,247,360  4.28%
    machine  20,000 passes of a loop in a cycle       8,825,906  0.71%
    fold     800 pieces folded onto one string          858,398  0.07%

`scatter` counts keys that arrive out of order (kanso#1820), `bump` updates a
map through a read named on the line before the write (kanso#1822), `machine`
runs a loop entered from inside a tail cycle (kanso#1824), and `fold` folds
pieces onto a string (kanso#1825). The counts follow the rule the header
already states for the stress shapes: `scatter` and `bump` are quadratic on
main, and each count lands its phase near five per cent of the program.
`machine` and `fold` cost little in instructions and a great deal in memory,
so their counts are sized to add about one decoded document to the peak.

The header keeps its changeover table as it was. That table is the mix
bench/runbench_phases.txt replays when history from before 2026-09-06 is
reconstructed, and a phase with no counter in that history has no place in it.
The four new phases have a table of their own below it, and
`tests/the_phase_map_matches_the_run_programs_header.rs` now stops reading at
the changeover table's total.

Every row the run program carries moves, because it now does more work. In the
run vein, run_allocs lands on 1,773,132, run_alloc_bytes on 326,862,802,
run_arena_blocks on 8, run_perm_allocs on 97, run_beat_iters on 27,564,
run_put_mut_grow on 15, run_str_scans on 229 and run_str_scan_bytes on
17,932,036. The shape counters follow the new values the phases build:
run_sh_str lands on 10,188,896, run_sh_rec on 1,500,528, run_sh_buf on
119,041,376 and run_sh_map on 11,200. `scatter` reads its map between puts, so
the run now builds and frees sorted views: run_view_allocs and run_view_frees
are both 360. The run program's machine code grows to 602,217 bytes, which
moves the text total to 4,651,966, and its emitted code to 684 defines, 6,519
calls, 5,938 branches and 50,994 lines, which moves emitted_other_defines to
2,002, emitted_other_calls to 17,393, emitted_other_branches to 16,393 and
emitted_other_lines to 146,696.

A counter that measures something new is re-based so that landing it costs the
floor nothing, as the codegen rows were on 2026-10-07 when their gates began
counting the clang the benchmarks use. `run_instructions` and `run_peak_bytes`
keep their ratio to their baselines: each baseline is multiplied by the reading
with the phases over the reading without them, both on main's compiler.
work_runbench lands on 1,245,043,324 on CI, against main's 1,131,539,079,
so run_instructions' baseline goes from 3,043,743,748 to 3,349,060,500. The run peak, which the objective reads as the arena, held and
permanent peaks summed, goes from 3,899,936 bytes to 12,167,368:
run_arena_peak_bytes lands on 8,912,912 and run_held_peak_bytes on 3,222,256,
and run_peak_bytes' baseline goes from 156,818,380 to 489,255,962. Both
baselines are rounded up. The score is unchanged, and the four fixes now
have something to score against.

## 2026-10-10 — `build` retires, and `tie` is the one way to write a cycle

The gavel of the same day retired the `build` block and everything that existed
to serve it: the hole `_` in a construction's argument, the field write
`a.f = v`, and the checker's proof that a write reaches only a record born in
the block. This entry is the build.

The parser no longer produces `Expr::Build`, `Expr::Hole` or `Stmt::Set`, and
the three variants are gone from the AST, so every pass that matched them lost
an arm. The checker loses the block-born walk and its cohort table. The
interpreter loses its field write and the native backend loses the
`k_set_field` call it emitted for one, along with the runtime function behind
it. A program written for the old form meets one syntax diagnostic for each
spelling, and each one names `list/tie`: a `build` header, a bare `_` where a
value goes, and `a.f = v`. The wildcard pattern and a lambda's `_` parameter
are read before the expression parser is asked, so they keep their meaning.

The error corpus loses the twenty-six fixtures that pinned the block's own
refusals and gains three for the retired spellings,
`a_build_block_is_retired`, `a_hole_is_retired` and
`a_field_write_is_retired`. Each was seen red with the diagnostic's wording
changed. `uncertain_reads_are_not_counted` loses its block-bound case and
keeps its other six. The micro corpus loses four fixtures that existed only for
the block. `a_description_rides_in_a_field` now builds its record with the
description in place, and the re-export spec for a door spelling loses its
`build` position and keeps the other four. The ratchet loses the seven rows
that disarmed a hole refusal, with their mutation scripts, and the construct
census in `tests/golden/shapes.txt` loses `Build`, `Hole` and `s:Set`.

The examples, the ch03 sample and the knot-equality golden were rewritten with
`tie` earlier on this branch. The mem fixture that pinned a cycle's cost moves
from `build_cycle` to `tie_cycle`, and `tie` costs more than the block did for
the same two cycles: 38 allocations against 18, 2,096 bytes against 784,
evac_allocs 15 against 13 and evac_bytes 560 against 496. sh_buf goes from 144
to 928 and sh_map from 0 to 192, which are the key list and the map `tie`
answers; sh_str falls from 192 to 176.

ch03 teaches `tie` in the section on records that point at each other, and
shows a field holding a list of references, which the same day's gavel "a field
may hold a list of `tie` references" owes the book. Appendix A replaces its `error[build]` section with `error[tie]`, the check
that refuses a literal link to a key the list does not hold. The playground's
two knot samples are rewritten with `tie`, and the compiler page's account of
the memory model now describes a cycle as a tie's birth cohort.
design/build-blocks.md is deleted; the log keeps its history.

Moving the cycle fixture onto `tie` found a disagreement between engines.
The interpreter builds a tie's nodes with thunk cells and settles each record's
fields once the knot is closed, but `resolve_tied` settled only a field that
was itself a cell. A cell inside a list or a map held by a field stayed a cell,
and every later read forced it again: `tie_cycle` counted 5 `thunk_forces` on
the interpreter and 0 natively. `settled_link` now replaces a forced cell
inside a list or map field with the node it holds, and the two engines agree on
the fixture. The spec is `mem_corpus_interp_matches_the_semantic_counters` in
`tests/oracle.rs`, which runs every mem fixture on the interpreter and compares
its semantic counters with the golden's. With the list and map arms of
`settled_link` removed it goes red on `tie_cycle`, reading 5 forces against 0.
The mem golden alone stays green, because it is read off the native binary.

`tests/compile_cost.rs` timed a `build_block` sample, which no longer parses,
so the sample and its row in `bench/compile_golden.txt` are gone. A `tie`
sample cannot take its place there, because the harness compiles a file with no
imports and `tie` lives in `list`. Removing the passes that served the block
moves the compile, entry, library, emit and interpreter rows and the size of
every binary. On CI every one of them falls. compile_instructions lands on
25,712,749, entry_instructions on 85,036,744, library_instructions on
85,590,951, emit_instructions on 7,038,004 and interp_instructions on
488,375,858. codegen_instructions_dev lands on 118,906,508 and
codegen_instructions_release on 453,887,359. browser_compile_instructions
lands on 388,285,810. Every binary is 448 bytes smaller, and the text total
lands on 4,645,694. work_runbench lands on 1,245,043,177, as projected, with
work_encodebench on 2,393,925,910, work_livebench on 1,472,860,747 and
work_oneshot on 13,176,864. No row rises, so the floor does not have to move to
admit the retirement, and the score rises from 90.24995 to 90.2512, which is
banked.

Three diagnostics lost their golden with the block. The unused-expression rule
lives in two walks, the function body's and the block's, and
a_build_body_line_goes_nowhere was the only program that reached either: in a
body, consecutive expression lines fold into one effect group, and a binding
after an expression line is refused first by "bindings precede the effects in a
body". The indented-block rule never had a reachable program. It was counted as
pinned by tests/golden/play/decls_after_statements.stderr, a file no test reads
and whose fixture now runs and prints `first`. Dropping `build` from the
message's wording took the stale copy out of agreement. All three are listed in
tests/golden/unpinned_diagnostics.txt with the programs that were tried.

## 2026-10-10 — gavel: GitHub issues are the tracker, and hygiene is continuous

Clay, on the proposal to track a cleanup list as GitHub issues: "why on Earth
not just use githubs issues feature? seems it would be a much more methodical
way to track and prioritize work." And on keeping the repository clean: "the
instructions should explicitly maintain ongoing refactoring so that things stay
hygienic over time and I don't have to explicitly remind you."

**The tracker.** Open work and open decisions are GitHub issues in
kanso-lang/kanso, labelled by kind (`needs-clay`, `ruled`, `cleanup`, `perf`),
lane (`lane:chat`, `lane:cloud`) and priority (`p1` to `p3`). A pull request
that finishes one closes it with `Fixes #N`. The log stays the record of what
was ruled and measured. STATUS.md's "Ruled, unbuilt" section and
design/pending-gavels.md retire; #1852 carries the steps, including the two
specs that keep their counts in step. The case for the change was made the same
day the two newest "Ruled, unbuilt" rows were found already built and never
taken off, and a ruling of 2026-10-07 was found never to have had a row.

Opened the same day: #1842, the one open decision (whether `list/map xs ref`
may build a node's list of references), and #1843, the three-parts-per-billion
row, the last standing "Ruled, unbuilt" entry; and nine `cleanup` issues from a
hygiene review, #1844 to #1852.

**Hygiene.** CLAUDE.md gains a section, "Hygiene is continuous": each pull
request leaves the files it touches cleaner; a fact has one home; cloud lands a
`cleanup` pull request for every five others; cloud sweeps the repository
weekly and files what it finds; a refactor is a pure move; and instructions
state rules, with their history in the log.

## 2026-10-10 — the log goes back under its cap a second time

design/compiler-log.md held 526 entries and 25,301 lines against a cap of
forty. The archive was last fed on 2026-09-18, and in the three weeks since
every change appended to the live file and nothing moved the older end.

This closes #1850. The 487 oldest entries move to design/log/compiler-log-archive.md
unedited, appended after its last entry. The live file keeps the newest 39 plus
this one. Nothing is deleted. Before the move there were 1,456 archived and 526
live entries; after it there are 1,943 archived and 39 live, so 1,982 entries
either way. The sorted heading lists of the two files match before and after
the move, and so do their non-blank lines, so the move changed no content.

The readers that broke last time were checked first. `page_drift` and the
trend gate count only added lines of the live log (`+## ` and `+`), so moving
lines out costs neither of them anything.
`a_question_sent_to_clay_has_a_ledger_entry` and its ratchet mutation read
both files since 2026-09-18, and
`a_log_heading_is_one_line` always has.

- **DONE** live log at forty entries.
- **OPEN** the cap has no gate. It was restored by hand on 2026-09-17 and again
  today, and both times it had run to many times the limit. A spec that counts
  the live file's headings and fails above forty would hold it. That spec is
  `tests/`, so it is cloud's: #1854.

## 2026-10-10 — three design notes retire, and a July ruling reaches the log

This closes #1851. CLAUDE.md says a design note whose work has shipped is
deleted, and that no file but the tracker carries pending-decision text. Five
notes were read against the tree.

**`design/enumerable.md` is deleted.** Its vocabulary is in `lib/list` and is
taught in ch12 and appendix B, including the collision rule. The two rulings
its status line called interim were ratified by Clay on 2026-08-14 (`range` is
unbound and the statistic is `spread`; map builders are last-write-wins, like
`put`), and §6 of the note already said so. Its one item marked open,
`first coll n` beside `take n . to_list`, was settled by what shipped:
`lib/list` has `first coll` alone, so `take` and `to_list` is the one way to
take n. Its §7 described build blocks, which retired today.

**`design/testing.md` is deleted.** The `testing` hako ships in `lib/testing`,
`lib/json/json_test.kso` uses `when_failed`, and ch07 and ch08 teach it. The
note's deferrals stay deferred and are recorded here so they keep a home: the
2026-07-28 describe/context sketch is superseded until real suites show the
need, with its two-deep limit still the right cage if they do; effectful tests
are out of scope until io testing gets its own design against the boundary
language; and two small observations were noted and not proposed, that a lambda
cannot carry arms (so dispatching on a reason inside `when_failed` takes a named
group) and that `when_failed` answering false on success folds "did not fail"
into "failed the wrong way".

**The work-ahead engine was ruled on 2026-07-22 and the ruling was never
logged.** It lived only in `design/workahead-engine.md`. Recorded now: when a
fiber parks on io, the scheduler runs proven-needed work in first-use order,
then speculative work, keyed off logical scheduler state so the counters stay
deterministic; v1 is proven work alone. The wall-credit substrate and the
`bench/workahead/` rig shipped, and the engine did not. The note's
implementation detail moves to #1856 (`ruled`, `lane:cloud`), and the note is
deleted. docs/compiler.html's section remains the public description.

**`design/hako.md` stays.** It is the only specification of `kanso install`,
`list` and `update`, which ship, and `src/hako.rs` cites it. Its open-questions
section pointed at the ledger, which is retiring under #1852; it now says the
`std/` question was ruled on 2026-08-29 (std ships inside the binary) and keeps
signing and monorepo hakos as parked.

**`design/memory-frontier-research.md` stays.** Its table is the record of
which memory ideas shipped, which were declined on measurement and which were
refuted, and its refuted list is what stops an idea from being rebuilt. Rows
4.1 and 4.4 concerned build blocks and are marked moot.

The oldest live entry moves to the archive with this one, so the live file
stays at forty.

- **DONE** #1851.
- **OPEN** #1856, the work-ahead engine, cloud's.

## 2026-10-10 — CLAUDE.md states its rules once

This closes #1849. CLAUDE.md had grown to 927 lines, most of them the history
of its own rules: dated stories of how each was learned, counts with the dates
they went stale, and corrections to earlier corrections. Its design-flow rule
says chronology belongs in the log, and its hygiene section says instructions
are rules. It is now 498 lines.

Every rule the old file stated with a bold lead is still there, checked by
listing the old file's bold leads and finding each one in the new text. What
went is the stories, the session tables and the dated counts. The full earlier
text is `git show be71feb8:CLAUDE.md`, and the stories are in the log entries
of their own dates.

Three things changed beyond trimming:

- The two-grep ruling count was written as one pipeline that fed the first
  count into the second grep, so it printed one meaningless number. It is two
  commands now.
- "Branches are free" moved from "Do not stop" to "Access and permissions",
  beside the other rules about what needs Clay's permission.
- The chat's lane names the design notes, which it has edited since the lanes
  were drawn and which #1851 was filed against.

Two specs read CLAUDE.md's wording, and both still pass. One wants the
cost-golden count word in "all TWELVE cost goldens,", the one count left in
the file, and #1846 asks cloud to retire it. The other wants every compile
gate named in the bullet containing "are separate gates". It went red during
the rewrap when that phrase broke across a line, which confirmed it reads the
phrase literally.

The oldest live entry moves to the archive with this one.

## 2026-10-10 — a pop after a skipped lap walks the whole result

With the two carry faults of kanso#1834 fixed, the sat port still failed under
the lap carry: `rand75_unsat` reported a satisfying assignment, and mal's
self-hosted runs died with a segmentation fault. Both came back right with the
pop's copy-out made deep, so the search went there.

When a carrying loop returns, `k_beat_pop_slow` copies its result out of the
loop's storage. That walk has no mark, so it counts every arena node as a
survivor, and it stops at a node whose direct interior survives as well. Under
the old rule every lap staged, so whatever the result held in the depth's carry
pair sat under the pair's own nodes, and the walk reached and copied it. The
lap carry lets laps go by without staging. Such a lap builds its new nodes in
the arena, around values that are still in the pair. The pop's walk meets the
fresh outer node first, finds its interior fresh too, and stops. The value in
the pair is left where it was. The next loop entered at the same depth stages
into the same pair and writes over it.

`tests/golden/micro/a_pop_after_a_skipped_lap_keeps_what_the_lap_built.kso` is
the shape in a dozen lines: a loop that wraps a box four times, of which only
the first lap stages, followed by a loop at the same depth that builds twenty
thousand elements a lap. The second loop's list replaced the first loop's
innermost box, and the program printed `<<<record>>>` where the interpreter
prints `<<<<[1 2 3]>>>>`.

The carry now records when a lap skipped its stage, in the lap carry's drift
test and in the chain's. A stage clears the record, and so does the push that
opens a depth. A pop that finds it set walks the whole result, as it already
did when a write had reached the carry. A pop after a loop whose last lap
staged is unchanged.

mal still crashed after that, in a map lookup reading a key whose length word
held a pointer. A hardware watchpoint on the key showed it written twice by
stage copies into a carry buffer and then overwritten by a later stage that
reused the buffer. The deep walk at the pop had found a map whose header and
pairs lay in the arena and whose keys lay in the pair. It copied the keys out
and rewrote the pairs in place. The map's sorted view is a second, malloc'd
copy of the pairs that the walk never reaches, so it went on naming the old
keys, and the next lookup searched it. The walk now drops the view whenever
it moves a key or a value in a map it repairs in place, and the next read
sorts again.

`tests/golden/micro/a_sorted_view_follows_the_keys_a_pop_moves.kso` builds
the shape: a loop that adds a key a lap and reads the map back, staging every
tenth lap, followed by a second run of the same loop with keys of the same
length. Without the change the program dies with a segmentation fault; with
it, it prints what the interpreter prints. Main does not reach this path,
because its pops copy deep only after a write has reached the carry.

The deep walk first cost memory where it should not have. It meets a string
builder whose header survives and goes straight to the in-place repair, which
asks only whether the builder's storage lies in the arena. A builder's storage
is malloc'd, so that answer was no, and the repair copied it into the arena and
dropped the builder's capacity, leaving a plain string the next append had to
copy again. The survival test the shallow walk uses already passes a builder,
and a malloc'd byte buffer, for this reason. The repair now passes them too.
With that, `string_builder_shape`, `builder_counts_once` and
`a_builder_handed_on_is_still_a_builder` are back to main's counts; with the
repair copying, `string_builder_shape` had evacuated 4,096 bytes where it
evacuates 80.

Against main, the counters that see a pop fall or hold. basket allocates
27,263 nodes, one fewer, and asks 4 survival questions where it asked 16,002;
the run program's survive count falls from 105,679 slots to 4,854 and it
evacuates 6,441,632 bytes where it evacuated 9,789,344. `record_reuse_shape`
asks 8 survival questions where it asked 16,006, and `effect_push_shape` 10
where it asked 15.

The ports' long-running loops, measured by peak resident memory on this branch
with main's fixes merged in, against main alone. mal's `(fib 25)` held 764 MB
on main and holds 9 MB here, and a 40,000-step tail call that held 147 MB
holds 9 MB; both print what they printed before. That closes mal's F8. lox's
F18, a loop that builds a `Point` a pass, is not closed by this change alone:
it holds 1,131 MB at 20,000 passes on both trees. With kanso#1824 added as
well it holds 24 MB at 20,000 passes and 88 MB at 80,000, which is what the
interpreter holds, 31 MB and 93 MB. The lox port keeps every object in a heap
of cells that it never collects, so that growth is the program's own. What
stays slow is time: each stage copies the whole live heap again, and 200,000
passes still had not finished after 120 seconds.

CI measured the rest against main, and most of it costs. Every benchmark's
machine code grows by between 9,200 and 9,472 bytes, the runtime's share of
the lap carry; runbench's is 611,769 bytes and the fourteen together 4,781,774.
work_deepbench does the most extra work, landing on 393,576,371 against
main's 367,269,411, 7.2% more. runbench, with the four phases kanso#1836 added, does 1,247,098,999 against main's 1,245,043,324, 0.17% more; encodebench
2,398,358,806, 0.19% more; livebench 1,473,968,154; jsonbench 860,559,756;
oneshot 13,210,643; work_widebench 28,175,109; digestbench 5,599,654; escapebench
39,045,879; indexbench 2,459,068; scanbench 270,069; readbench 4,577,565.
basket does less, 30,415,829 against 32,505,156, and so does pendbench,
190,050,778 against 190,396,306. The deep walk at the pop and the record of a
skipped lap are what this change adds to those paths, but no build has
separated the two, so the delta arrived with the change and its split between
them is open. On the compile side, release codegen counts 454,310,397, dev
codegen 118,946,233, compile_instructions 25,858,385, entry_instructions
85,515,649 and library_instructions 86,067,297, each a little higher, and
emit_instructions falls to 7,058,267. In the browser, compiling the corpus
takes 390,605,563 instructions with a peak of 675,725 bytes, and running it
34,102,877.

Together those cost the objective 0.0035: the meta reads 90.2465 against a
floor of 90.2499. The change is a correctness fix, because a loop that holds
764 MB to compute `(fib 25)` is a program that fails on a larger input, and
the floor goes down by exactly that much under the rule for changes that make
the language work as specified.

## 2026-10-10 — a fold that joins onto its accumulator writes in place

The mustache port renders a template by folding over its pieces, and its
folder answers `"{acc}{piece}"`. Natively, every step copied the whole string
built so far, so 20,000 pieces held 2.77 GB before the program finished. The
interpreter was not affected.

The string-builder analysis already wrote a join in place when the
accumulator is carried by a recursive function. A lambda handed to a fold was
never looked at, because `walk_for_builder` stops at every lambda. A fold's
folder runs once per element on the accumulator the fold threads through, so
when `fold_owns_accumulator` says the fold owns its seed, a join onto the
folder's first parameter has the same shape as the recursive case. The walk
now marks those joins: in the folder's body, in either arm of an `if`, and
through the wrapper lambda that `body_is_folder` already accepts.

A folder passed by name, as in `list/fold xs "" piece`, is handled the same
way under three conditions. Every arm of `piece` must end in a join onto its
first parameter. Every direct call of `piece` must hand that argument over.
Every use of the name as a value must be the folder of a fold whose seed
nothing else holds. A partial application of `piece` keeps what it was given,
so it disqualifies the group.

The runtime's in-place join stopped the program when its accumulator was not
already a builder. A fold's seed is usually a literal, so `k_concat_arr_mut`
now converts it on the first join, the way a builder parameter's caller does.

`tests/golden/mem/a_fold_that_joins_onto_its_accumulator_grows_one_string`
folds 2,000 pieces with a named folder and with a lambda. `alloc_bytes` falls
from 21,839,568 to 1,221,150 and `arena_peak_bytes` from 22,020,096 to
2,097,152. `tests/golden/micro/a_fold_joins_onto_its_own_accumulator` covers
the ways the string can be shared: a seed bound to a name, one fold's result
folded twice more, and a direct call of the named folder. Making
`fold_owns_accumulator` answer yes unconditionally turns it red. The two folds
over one lambda result then wrote into the same buffer, and both printed
`[1][2][3]+5-6`.

No benchmark folds a string, so every allocation counter agrees with main.
The runtime's join grows 320 bytes of machine code, and CI's rows move with
it:

- `text` grows 320 bytes a binary, to 4,593,438 in all.
- `work_basket` lands on 32,404,790, `work_runbench` on 1,129,589,259,
  `work_livebench` on 1,473,486,345, `work_encodebench` on 2,384,338,137,
  `work_oneshot` on 13,027,190, `work_digestbench` on 5,566,917 and
  `work_scanbench` on 281,806, each within 22,030 instructions of main.
- The analysis costs `compile_instructions` 25,857,543,
  `entry_instructions` 85,513,471 and `library_instructions` 86,066,360.
  `emit_instructions` falls to 7,058,306, `interp_instructions` to
  488,868,354, and both codegen rows fall.
- `browser_compile_instructions` rises from 388,182,464 to 391,515,764,
  0.86%, and the browser run row falls 6, to 33,510,702. The rise does not
  come from the new pass: with `named_folder_joins` switched off the row reads
  392,251,588, higher still, so it follows the wasm binary's layout rather
  than the work done.

On main as it stood before kanso#1836, welfare fell from 90.25811251 to
90.25519022 on those rows, because no benchmark folded a string.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `fold` phase: 800 pieces folded onto one
string, measured on main before this fix at 858,398 instructions. With main
merged in, runbench falls from 1,245,043,324 to 1,244,977,121, a projection
from this container's 1,244,976,714 plus the 407 instructions by which CI read
main above it; CI's row replaces it. The phase's memory is where the fix pays.
run_arena_peak_bytes falls from 8,912,912 to 7,864,336, run_arena_blocks from
8 to 7, run_alloc_bytes from 326,862,802 to 325,331,283, run_allocs from
1,773,132 to 1,772,340 and run_sh_str from 10,188,896 to 8,648,960. Two run
counters rise with the fix: run_str_scans goes from 229 to 1,021 while
run_str_scan_bytes falls to 17,930,429, and run_bytes_malloc rises by 7 to
9,127. Nothing here isolates which step of the in-place join each comes from.
The rows the runtime's 320 bytes moved above
carry onto main's values: `text` totals 4,656,446, work_basket lands on
32,527,186, work_livebench on 1,472,861,727, work_encodebench on
2,393,926,750, work_oneshot on 13,176,997, work_digestbench on 5,598,179 and
work_scanbench on 268,886. The analysis's rows carry the same way:
compile_instructions lands on 25,857,737, entry_instructions on 85,514,580,
library_instructions on 86,066,207 and browser_compile_instructions on
391,827,307. All of these are projected until CI reads them. Welfare rises from 90.2499
to 90.3104 on those rows, and the rise is banked.

CI's rows replace the projection. work_runbench lands on 1,244,977,121, as
projected. The other rows CI corrects are small: `text` totals 4,656,478,
work_encodebench lands on 2,393,926,869, work_livebench on 1,472,861,230 and
browser_compile_instructions on 391,826,076. The run program's emitted code
reads one call and two lines more than this container wrote, so
emitted_other_calls lands on 17,394 and emitted_other_lines on 146,698.
Welfare reads 90.3105 on CI's rows, and the rise is banked.

The conversion in `k_concat_arr_mut` made `k_b_adopt` redundant, and the
ratchet found it on the carrier. Two rows mutated the adoption: "a join that
never adopts its seed" removed it, and "a lazy seed taken for a builder"
handed it the forced value. Both stayed green, because a seed the adoption
misses now reaches the first join without room and is made into a builder
there. Neither mechanism could be removed and seen red while the other stood.
The two rows are replaced by "a first join that never makes its builder",
which restores the old refusal in `k_concat_arr_mut`. Under it
`a_fold_joins_onto_its_own_accumulator` prints nothing from native and the
micro corpus goes red. `k_b_adopt` is left in place for now and goes in a
change of its own, since removing it moves the text and codegen rows and
needs CI to read them.

## 2026-10-10 — a loop entered from a tail cycle gives its garbage back

The lox and mal ports both ran their interpreters out of memory. A Lox `for`
loop of fifty thousand passes that builds two small objects per pass held
3.0 GB at its peak natively, and mal's tail-recursive `sum2` kept 3.7 KB per
iteration until exit. The interpreter under `--interp` ran the same programs
in about 11 MB.

Each port's machine loop is a self-recursive function, the shape the beat
analysis rewinds, and neither rewound. The lox loop, `spin`, failed two
tests. The first was its entry. `exec` tail-calls `spin` from its while arm,
and `exec` sits in a tail cycle of its own, since a block runs its first
statement and tail-calls itself on the rest. The analysis demoted a tail
entry to a plain call only when the caller was in no tail cycle at all, on
the reasoning that a plain call inside a cycle grows the stack. That holds
only when the cycle runs through the demoted edge, which happens when the
loop can reach its caller back by tail calls. `spin` returns its state to
`exec` and never tail-calls it, so the plain call holds one frame while the
loop runs and the caller returns before its cycle goes round again. The
condition now asks exactly that: the entry is demoted unless the loop
tail-reaches the caller.

The second was the loop's own arguments. `spin` hands its body and condition
on unchanged, and both come out of a record field, which inference reads as
any value at all. A slot of unknown type was never carried through a
rewind, in case it hid a growing accumulator behind a helper call. A
parameter every self-tail-call passes on as itself cannot grow: it arrived at
entry, lives below the mark, and the evacuation shares it rather than copying
it. Such a position is now carried whatever its type.

With both changes the lox loop rewinds every pass. At fifty thousand passes
the release binary peaks at 77 MB, down from 3.0 GB, and runs in 1.17 s
against 3.01 s. What remains is the port's own heap of instance fields, which
it never frees by design. The fixture
`a_loop_entered_from_a_tail_cycle_gives_its_garbage_back` builds the same
shape in thirty lines and pins `arena_peak_bytes` at 1,048,576 for twenty
thousand passes; main holds 3,145,728 for it.

mal's machine is not fixed by this. Its `ev` is one member of a tail cycle
with `ret`, `throw` and `settle`, which makes it a cluster, and a cluster
carries only scalars and threaded parameters through its rewind. Carrying a
heap value round a cluster is the next piece of work.

No benchmark's run changed: every runtime cost vein agrees with its golden.
The analysis is dearer to run, on CI's rows after merging main:
`compile_instructions` rises to 25,861,592, `entry_instructions` to
85,526,009 and `library_instructions` to 86,078,947, while
`emit_instructions` falls to 7,032,277 and `interp_instructions` to
488,876,843. `browser_compile_instructions` rises from 388,182,464 to
390,287,782 with this tree's rustc. On main as it stood before kanso#1836,
welfare fell from 90.25811251 to 90.25621395, because no benchmark ran a loop
entered from a cycle.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `machine` phase: 20,000 passes of a loop in
a cycle, measured on main before this fix at 8,825,906 instructions. Rewinding
the loop costs instructions and gives memory back. With main merged in,
work_runbench rises from 1,245,043,324 to 1,263,682,936, a projection from
this container's 1,263,682,529 plus the 407 instructions by which CI read main
above it; CI's row replaces it. run_arena_peak_bytes falls from 8,912,912 to
6,815,760 and run_arena_blocks from 8 to 6. The rewind's own work shows in the
rows that rise: run_beat_iters lands on 47,564, run_evac_allocs on 128,717,
run_evac_bytes on 11,709,440, run_alloc_bytes on 326,929,762, run_allocs on
1,773,139, run_sh_buf on 119,108,240 and run_buf_reuse on 134. Carried onto
main's rows, compile_instructions lands on 25,861,786, entry_instructions on
85,527,118, library_instructions on 86,078,794 and
browser_compile_instructions on 390,599,325, all projected until CI reads
them. The objective weighs the peak above the instructions: welfare rises from
90.2499 to 90.3087 on those rows, and the rise is banked. The fix's own
fixture, `a_loop_entered_from_a_tail_cycle_gives_its_garbage_back`, reads
four fewer evacuations on main's runtime: evac_allocs lands on 80,013 and
evac_bytes on 1,920,368.

CI's rows replace the projection. work_runbench lands on 1,263,682,936, as
projected. The rewind's code in the run program's machine phase shows in its
emitted counts: emitted_other_calls lands on 17,405, emitted_other_branches on
16,399 and emitted_other_lines on 146,723. `text` totals 4,652,398 and
browser_compile_instructions lands on 390,597,579. Welfare reads 90.3087 on
CI's rows, the score already banked.

## 2026-10-10 — a read named before the write keeps the write in place

Three ports hit the same cliff. The sat solver, the toml parser and the lox
interpreter each read a map entry into a name and then wrote the map:

```
v = m[i]
bump (put m i (v + 1)) ...
```

Written inline as `put m i (m[i] + 1)`, the put happens in place. Written
with the name, `linear.rs` counted `v = m[i]` as a second use of `m`, so `m`
was not unique at the put and every put copied the map. The sat port's
reduction, 20,000 steps over 100 keys, ran in 27.5 seconds natively with the
name and 0.010 seconds without it.

The two programs are the same as far as the map is concerned. The read
answers an element, and `v` is used only as an operand of the put's own
arguments, which are evaluated before the put writes. `effective_uses` now
discounts a binding's mentions of the map when the bound name meets three
conditions: neither the name nor the map is rebound in the block, the
binding does not hold the map itself, and every later mention of the name is
reached from the sibling arguments of a `put`, `push` or `append` on that
map through arithmetic and comparison alone. A mention anywhere else (inside
a lambda, a list, another call, or a second binding such as `w = v * 2`)
could force the read after the write, and the binding still counts. A loop's
bindings usually sit under its `return ... if` guard, so the search goes down
through the guard's rest.

`tests/golden/mem/a_read_named_before_the_write_stays_in_place.kso` is the
sat reduction at 2,000 steps. Its golden pins 8 allocations; the compiler
before this change made 6,013 on it.
`tests/golden/micro/a_read_named_before_a_write_sees_the_old_value.kso`
covers the other side: a name used after the write, a name stored in the
list the put writes, and a name that feeds a second binding all keep the
copy, and each line prints the old value beside the new on every engine.

The analysis costs the front end a little on every route: compile_instructions
lands on 25,857,737, entry_instructions on 85,514,349, library_instructions on
86,065,981, emit_instructions on 7,065,059 and interp_instructions on
489,220,284. browser_compile_instructions lands on 392,737,085. These are
carried onto main's rows from this branch's own readings and are projected
until CI reads them.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `bump` phase: 350 updates through a named
read, measured on main before this fix at 53,247,360 instructions. With main
merged in, runbench falls from 1,245,043,324 to 1,192,648,013, a projection
from this container's 1,192,647,606 plus the 407 instructions by which CI read
main above it; CI's row replaces it. run_held_peak_bytes falls from 3,222,256
to 197,704 and run_arena_peak_bytes from 8,912,912 to 7,864,336, because the
phase no longer keeps a copy of its map per update. run_alloc_bytes falls to
322,971,202, run_allocs to 1,772,081 and run_sh_map to 0. run_buf_reuse falls
by one to 136, and run_view_allocs and run_view_frees fall from 360 to 10 with
the fix; nothing here isolates why the scatter phase's views moved.

CI's rows replace the projection. work_runbench lands on 1,192,648,013, as
projected. The front-end rows land on entry_instructions 85,514,580,
library_instructions 86,066,207, interp_instructions 488,962,208 and
emit_instructions 7,059,706, and browser_compile_instructions on 392,738,164.
`text` totals 4,652,222. The run program's emitted code reads one line fewer,
50,993. Welfare reads 90.6830 on CI's rows, and the rise is banked.

## 2026-10-10 — a map's view takes keys out of order without moving itself

Three ports found the same cliff from three directions. xsv's
`stats --cardinality` spent 2.6 of its 2.7 seconds counting a column of
distinct values, ugit's Myers diff on a 3,000-line file took 4.0 seconds
against the interpreter's 0.4, and `list/tally` over 40,000 distinct
strings took 4.95 seconds natively where the interpreter took 0.2. All of
them read a map between puts. A read builds the map's sorted view, and
from then on each put of a new key was inserted into the view in order,
which for a key landing in the middle meant moving every pair after it
one slot along. Callgrind on the ugit diff put 80% of the time there.

A key that would land in the middle of a view of more than sixty-four
pairs now goes to a second sorted run beside it, which the runtime calls
the side. A key past the view's last still goes on its end, so a map
filled in ascending order never starts one. Reads search the view and
then the side, so a read between two puts merges nothing. When the side
holds more keys than the square root of the view, the two are merged,
from the back and into the view's own buffer when it has room. Anything
that wants the whole order, such as printing, `entries` or equality,
merges first.

The side costs the map header nothing. A view's buffer is preceded by a
header slot whose first word is its capacity, and the second word, which
was unused, now points to the side. A mark on the map's view pointer
says a side is waiting, so a read that finds the bit clear asks nothing
more, and the emitted fast put, which only tests the pointer for null,
still takes the call when a view exists. The side is kept, emptied, after
a merge, so the next key out of order does not allocate.

`list/tally` over 40,000 distinct strings went from 4.95 seconds to 0.21,
and xsv's reproduction from 4.75 to 0.17. `tests/golden/mem/
a_tally_of_scattered_keys_merges_its_view.kso` pins the shape: 4,000
scrambled keys, each read before it is put, take 15 view allocations where
main took 12, and hold 206,448 bytes at peak where main held 196,576.
Two mem goldens move with it. `fused_tally` takes one more allocation (65
to 66, alloc_bytes 41,536 to 42,064), and `growing_map` two more (1,613 to
1,615), with held_peak_bytes 49,120 to 55,408 and alloc_bytes 181,952 to
189,312.
`tests/golden/micro/a_map_written_out_of_order_past_its_room.kso` puts 211
keys in scrambled order with reads, a length, a missing key and a print in
between, and one in-place loop visits every key twice, so its second visit
reads and replaces keys that are still in the side. Every engine prints the
same. Leaving the side out of the search turns it red: the loop reads a key
it has already put as missing, puts it again, and prints 217 keys.

The first version of that fixture could not fail. Its loop was also handed
a map the play went on reading, so the linearity analysis gave none of its
puts the in-place write, no view ever grew a side, and removing the side
search changed nothing. A separate loop that only ever receives a fresh map
is what made the side reachable.

A put that replaces a key already in the map still walks the pairs log
backwards to find its last write, so a tally whose keys repeat pays for
the distance back to that key's last occurrence. That walk is untouched
here.

The first round of CI put welfare 0.0000 below its floor, and all of the
fall was in the run program: runbench rose from 1,129,588,698 to
1,130,533,679. Callgrind on both builds put the whole difference in one
function, `k_b_columns`, which `keys` and `values` reach on every call, and
its disassembly showed why. It asks `k_map_sorted` for the view, and the
check before that call grew from one test (is there a view?) to two (is
there a view, and does it have a side?), five instructions where main had
two. The side mark was the pointer's low bit.

The mark now sits in the pointer's top bit. A 64-bit address never sets it,
so read as a signed number the view pointer is positive with no side, zero
with no view and negative with a side, and one test against zero asks both
questions. `k_b_columns` came back to main's instructions with one alignment
nop added before its copy loop. Measured locally from one directory against
a build of main's runtime, runbench is 207,423 instructions above main and
livebench 903,510, down from 945,000 and 4,208,000. wasm32 keeps the low
bit, because a 32-bit address can have its top bit set.

CI's rows for this head with main's constant storage merged in, against
main: work_runbench rises from 1,131,250,979 to 1,131,457,856 and
work_livebench from 1,472,859,260 to 1,473,762,140, the remains of the check
described above. work_basket rises from 32,375,114 to 32,926,353, 1.7%,
work_encodebench to 2,383,871,632, work_oneshot to 13,189,990 and
work_pendbench to 190,390,734. Every binary's text is about 4,800 bytes
larger, which sums to text 4,612,254 to 4,679,230. browser_run_instructions
goes from 33,493,063 to 33,502,036, codegen_instructions_dev from 118,896,762
to 118,916,117 and codegen_instructions_release from 453,707,138 to
454,001,319, all carrying the larger runtime. basket builds maps and reads
them between puts, so it is the benchmark that meets the side; which of its
maps grows one is not isolated here. The mem rows are the ones named above:
fused_tally_allocs 65 to 66 and fused_tally_alloc_bytes 41,536 to 42,064;
growing_map_allocs 1,613 to 1,615, growing_map_alloc_bytes 181,952 to 189,312
and growing_map_held_peak_bytes 49,120 to 55,408. Welfare falls from
90.25153 to 90.25054. The gate passes that, because it allows 0.001 for
disagreement between hosts, but every term here is deterministic, so the fall
is real. It raised the floor question for this fix and three other port
fixes, and the paragraph below records how that question closed.

The floor question closed on the corpus-first gavel, and kanso#1836 put the
shape into the run program as its `scatter` phase: 2,340 keys counted in
scrambled order, measured on main before this fix at 50,901,814 instructions.
With main merged in, runbench falls from 1,245,043,324 to 1,201,828,599, a
projection from this container's 1,201,828,192 plus the 407 instructions by
which CI read main above it; CI's row replaces it. The run's held peak falls
from 3,222,256 bytes to 3,166,752, run_alloc_bytes falls to 326,749,138 and
run_allocs rises by two to 1,773,134. Welfare rises from 90.2499 to 90.4099 on
those rows, and the rise is banked.

CI's rows for the merged tree replace the projection and agree with it on the
run program: work_runbench lands on 1,201,828,611. The merge had carried main's
rows for the other benchmarks, and CI restores this fix's own cost on them.
The view code adds 4,784 bytes to every binary, so `text` totals 4,718,942.
work_basket lands on 33,056,395, work_livebench on 1,473,763,578,
work_encodebench on 2,393,936,650, work_oneshot on 13,190,150 and
work_pendbench on 190,396,604. codegen_instructions_dev lands on 118,940,067
and browser_run_instructions on 33,517,389. Welfare reads 90.4099 on CI's rows,
the score already banked.

## 2026-10-10 — the five port fixes land together

kanso#1820, kanso#1822, kanso#1824, kanso#1825 and kanso#1827 each merged main
and scored against the run program on their own. Each also moves the runbench
row and the floor, so landing them one at a time would cost a CI round per pull
request. This branch merges all five onto main and is measured as one tree.

The fixes do not add up. Measured separately against main, their runbench
deltas sum to -74,980,545, which would put the row at 1,170,062,384. The
combined tree reads 1,152,573,228 in this container, 17,489,149 lower, so it is
projected at 1,152,573,635 until CI reads it. The loop rewind (kanso#1824) and
the lap carry (kanso#1827) both change what a rewound loop keeps, and the loop
fixture `a_loop_entered_from_a_tail_cycle_gives_its_garbage_back` shows the
overlap most plainly: with both fixes evac_allocs falls from 80,013 to 49 and
evac_bytes from 1,920,368 to 1,232. Which of the two fixes accounts for the
runbench difference is not isolated here.

The run program's counters land on the combined tree as run_beat_iters on 47,564, run_buf_reuse on 132, run_bytes_malloc on 9,127, run_sh_buf on 119,108,240, run_str_scans on 1,021, run_ten_frees on 1, run_ten_handups on 1, run_view_allocs on 12, run_view_frees on 12. The mem
fixtures the map fix moved land as fused_tally_alloc_bytes on 42,064, fused_tally_allocs on 66, fused_tally_view_allocs on 16, growing_map_alloc_bytes on 189,312, growing_map_allocs on 1,615, growing_map_held_peak_bytes on 55,408, growing_map_view_allocs on 12, growing_map_view_frees on 10. The benchmarks' work rows carry
each fix's own delta onto main's row until CI measures them together:
work_deepbench on 393,576,370, work_digestbench on 5,599,739, work_encodebench on 2,398,369,413, work_escapebench on 39,045,874, work_indexbench on 2,459,056, work_jsonbench on 860,559,605, work_livebench on 1,474,870,880, work_oneshot on 13,223,306, work_readbench on 4,577,564, work_scanbench on 270,158, work_widebench on 28,175,107. So do the compile and browser rows: compile_instructions on 25,938,043, entry_instructions on 85,762,025, library_instructions on 86,314,325, codegen_instructions_dev on 118,966,405, codegen_instructions_release on 454,110,241, browser_compile_instructions on 400,285,361, browser_compile_peak_bytes on 675,725, browser_run_instructions on 34,111,788. `text` totals
4,853,950. Every mem fixture that predates the lap carry gains its
`chain_finds=0` line.

Welfare reads 91.0246 on these rows against the five floors banked separately,
the highest of which was 90.6830, and the rise is banked.

Main moved under the branch when kanso#1837 retired `build`, and the carrier
took main in a merge commit. The compiler's front end lost the passes `build`
needed, so the rows the carrier's analysis had raised land below where they
stood before the merge and above main's: compile_instructions lands on
25,715,103 against main's 25,712,749, entry_instructions on 85,044,877 against
85,036,744, and library_instructions on 85,599,176 against 85,590,951.
interp_instructions lands on 488,792,115 against 488,375,858. The two codegen
tiers land on 118,954,619 for codegen_instructions_dev and 454,407,182 for
codegen_instructions_release, against main's 118,906,508 and 453,887,359.
browser_compile_instructions lands on 390,036,773 against 388,285,810. Each
runtime binary sheds 448 bytes with `build` gone, and `text` totals 4,826,814
against main's 4,645,694. The work rows CI read on the merged tree are
work_encodebench on 2,398,369,791, work_livebench on 1,474,871,454 and
work_oneshot on 13,223,698; work_runbench lands on 1,152,754,428, against
main's 1,245,043,177. Welfare reads 91.0337 on CI's rows against the floor of
91.0317, and the rise is banked.

## 2026-10-10 — a lap waits for twice what the last stage copied

Testing the combined tree on the lox port's F18 loop found two things the five
fixes did not do on their own. Peak memory fell as expected: at 40,000 passes
the loop held 45 MB against main's 2,333 MB. But past about 135,000 passes the
run slowed sharply, from 2.7 seconds at 130,000 to 99 seconds at 160,000. Stack
samples put the time in `k_copy_size` and `k_deep_copy` under
`k_beat_lap_carry`, and the counters showed evac_allocs jumping from 5.8
million at 130,000 passes to 139 million at 140,000. Tenure stops at
`K_TEN_CAP`, 64 MiB, and lox's heap map keeps every object it allocates, so
from that point on every stage copied the whole live state, once per quarter
megabyte of drift.

A smaller program then showed a worse case. The loop of
`a_loop_entered_from_a_tail_cycle_gives_its_garbage_back`, with a map in its
state that keeps one entry a pass, ran 20,000 passes in 0.01 seconds on main
and in 3.2 seconds with kanso#1824 alone, allocating 9.5 GB. kanso#1824 is what
makes this loop rewind at all. Each stage copies the map out of the arena, the
next put copies it back, and that copy is a quarter megabyte of drift on its
own, so a stage followed almost every pass.

`k_beat_lap_carry` now waits for twice what the last stage copied before it
stages again, and never less than the quarter megabyte it waited before.
Copying then stays proportional to the drift it answers, which is how a copying
collector sizes its heap. When the arena has opened blocks since the last stage
and the wait has grown past the quarter megabyte, the drift is measured across
them: the rest of the old block, every block between, and what the newest
holds. A loop whose stages copy little behaves as it did, including staging
when the arena opens a block. The first version counted the newest block at
its full size, and with a 2 MiB block that staged on most passes.

`tests/golden/mem/a_growing_map_in_a_rewound_loop_waits_for_its_copy` is that
small program at 20,000 passes. Its alloc_bytes is 44,612,576; without the wait
it reads 9,165,495,888. No other mem fixture and no cost golden moves. On the
lox loop, 160,000 passes now take 5.3 seconds and 320,000 take 33.

Time still grows faster than the pass count, and the counters say where.
Allocation is linear: alloc_bytes reads 2,457,232,021 at 40,000 passes,
5,168,271,765 at 80,000 and 10,642,119,589 at 160,000, about 66 KB a pass.
evac_bytes is not. It reads 63,965,040, 132,744,688 and then 1,296,200,208,
ten times the step before, while ten_blocks only goes from 8 to 9. That is the
pass count at which the port's heap outgrows tenure. Below it, tenure holds the
heap and a stage copies only what the pass made. Above it, each stage copies the
live heap, and the wait keeps that to half of the drift at most, so copying is
bounded by allocation but no longer small next to it. The cost per byte copied
also rises as the heap grows, because the copy keeps a pointer map for shared
values and the map grows with the heap. Removing it would mean letting tenure
grow past `K_TEN_CAP` for a loop whose survivors keep surviving. The cap exists
so that no loop holds storage that only the pop frees, and changing it is a
separate piece of work.
runbench reads 180,786 instructions more for the wait's arithmetic and is
projected at 1,152,754,421 until CI reads it. Welfare falls from 91.0246 to
91.0239, which this fix pays to remove a regression the carried kanso#1824
would otherwise ship.

CI has now read the combined tree, lap wait included. runbench reads
1,152,754,435, fourteen instructions over the projection, so the wait's
180,786 holds. The rows the projections had carried land as follows.
work_encodebench lands on 2,398,370,183, work_escapebench on 39,045,884,
work_indexbench on 2,459,212, work_livebench on 1,474,870,796, work_oneshot on
13,223,628 and work_scanbench on 270,249. work_basket falls to 30,812,505.
compile_instructions lands on 25,832,558, entry_instructions on 85,436,280,
library_instructions on 85,987,742, emit_instructions on 7,037,238 and
interp_instructions on 489,012,183. codegen_instructions_dev lands on
118,951,226 and codegen_instructions_release on 454,427,018.
browser_compile_instructions lands on 391,401,241, browser_compile_peak_bytes
on 675,725 and browser_run_instructions on 34,124,829. The run program's
emitted code gains 29 lines, which moves emitted_other_branches to 16,399,
emitted_other_calls to 17,406 and emitted_other_lines to 146,724, and `text`
totals 4,833,086. Welfare reads 91.0317 on CI's rows, and the rise is banked.

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
