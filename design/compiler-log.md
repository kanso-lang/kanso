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

A second row went blind with this change: "a character read remembering a
position into a string a beat has freed", gated by the beat differential.
Its probe loop ran six small laps, and it found a string reusing a freed
address only because every fresh loop staged on its first lap, which is the
behaviour this entry removes. Each probe lap now leaves 3,000 short strings
in the arena by a recursion that is not a loop, so every lap passes the
wait and rewinds. Unmutated the sweep reads 0 of 96 disagreeing; under the
mutation, 20. The mutation itself was red for the wrong reason on main: it
deleted the reset lines, and the inline rewind's reset is the body of an
unbraced `if`, so the arena's own rewind became conditional and the sweep
died on "a beat mark and the arena disagree". It now blanks each store.

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
run_sh_map 12,160, run_bytes_malloc 7,885 and run_ten_blocks 1, with
run_ten_frees 2 -> 1 and run_ten_handups 6 -> 1 for the block that is no
longer taken. run_view_allocs and run_view_frees fall 769 -> 399 and
run_cohort_frees 6 -> 5; which fix moves the map views is not isolated
either. Of its peaks,
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
