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

## 2026-09-14 — the render side of the float pair had no round-trip harness either

kanso#1423 found `k_b_to_float` taking eisel-lemire's answer as certain on a
truncated significand, one ULP from the correctly-rounded double, on 221 of
1,405,451 cases. That function's whole corpus was 86 values. The same question
asked of the other direction — does the text `render_ryu` writes read back as
the double it was given — had no harness at all.

What existed checked two neighbouring things. `scripts/render_differential`
runs the interpreter's `render` against the C runtime's `k_render` and
requires them to agree; two implementations wrong the same way pass it, and
the interpreter's float rendering is not independent of ryū's. The sweep in
`the_shortest_digits_come_out_in_pairs` checks the block that writes chosen
digits into a buffer against snprintf; it says nothing about which digits were
chosen.

`tests/every_rendered_float_reads_back_as_itself.rs` asks the property
directly, with `strtod` as the independent reference. It lifts `ryu_d2d`,
`render_ryu`, their pow5 tables and the two helpers they call out of
`src/runtime.c` — the real text, never a copy — and sweeps 2,809,326 values in
three seconds:

    2,809,326 rendered, 0 do not read back, 0 length disagrees, 0 not shortest

The corpus is four groups, and the second one matters more than it looks.
Random 64-bit patterns spread their exponents uniformly over the whole field,
so almost none of them land where a json document's numbers live; `m`, `m/10`,
`m/1000` and `-m/100` for m below 200,000 name that range by hand. The other
two groups are both sides of every binary exponent including the subnormals,
and both sides of every power of ten.

**Watched red three ways, one per property, each leaving the other two
clean.** That separation is the evidence the three checks are independent
rather than one check written three times:

    output = vr + (...)  ->  output = vr        815,943 do not read back
    the removal loop breaks after one step      581,913 are not shortest
    return (o - buf)     ->  + 1                2,809,321 lengths disagree

**A counter of mine was wrong before the renderer was.** The first
shortest-ness check counted significant digits out of the rendered text, which
calls `"100"` two digits — the plain form pads with zeros to reach the decimal
point and those are not digits ryū chose. It reported 2,863 shortest failures,
every one of them the counter's. Taking `k` from `ryu_d2d` itself is both
correct and the more honest question, since `k` is what ryū claims.

Ratchet row `render_trip`, mutation `a_rendered_float_that_never_rounds_up.sh`.

**And the integer half of the same door.** `k_b_to_int` parses `[-]?digits` in
a bare loop when the digit run is eighteen or fewer, on the ground that
eighteen digits cannot overflow an i64, and hands everything else to strtoll.
That bound is the whole safety argument and nothing checked it. It is SOUND:
24,000,029 strings, 21,816,309 of them taking the fast path, 0 disagreeing
with strtoll.

Recorded as a spec rather than left as a measurement because the bound is one
character from wrong. Nineteen nines is 9,999,999,999,999,999,999 against
i64's 9,223,372,036,854,775,807, so widening the bound — the edit someone
optimising this would reach for — puts 50,249 wrong answers into the fast
path, the first at `"9223372036854775808"`, 2^63 exactly, where the loop wraps
to the negative and libc saturates and raises. That is the watched-red, and
`tests/the_int_fast_path_agrees_with_libc.rs` is what now sees it. Ratchet row
`int_bound`, mutation `an_eighteen_digit_bound_widened_to_nineteen.sh`.

## 2026-09-14 — the parse harness that found the bug was thrown away, so it ships

kanso#1423 found `k_b_to_float` calling a truncated significand certain by
building a differential harness against strtod, running it over 1,405,451
cases, and reading off 221 wrong answers. Then the harness was deleted and
the fix shipped with a six-line micro fixture.

That leaves the corpus where it was. CLAUDE.md asks for the fuzzer to be
the thing that ships — "build the differential fuzzer first, against an
independently-written reference... The harness extracts the real function
text from the source, never a copy" — and a bug with no home in the corpus
is a gap in the corpus, so adding the home is part of the fix. Float
RENDERING has had a harness since kanso#1424 and integer PARSING since the
same pull request. Parsing a float, the one kernel with an actual defect
against it, had none: `grep -l k_b_to_float tests/*.rs` came back empty.

`tests/every_float_literal_parses_like_strtod.rs` closes it. Two spans are
lifted out of src/runtime.c — the pow5 tables with `k_el_parse`, and the
scan out of `k_b_to_float` — and compiled with clang, so a change to either
cannot pass by leaving a stale duplicate behind. Six deliberately-chosen
groups: significands past nineteen digits with a nonzero tail, the same
with the point inside the run so the fraction branch drops rather than
trades, trailing zeros past nineteen digits which are NOT a truncation and
must stay fast, the exponent extremes where the table runs out and doubles
go subnormal, round-to-even boundaries, and the plain short decimals the
benchmarks actually parse.

    1,193,654 parsed    599,826 took the fast path    0 disagree

WATCHED RED, and the first attempt was watched red for the WRONG REASON,
which is worth writing down because the mistake is invisible when it
happens. The closing anchor of the lifted scan was the line
`if (ok && any && !cut && p == stop) {`. The mutation removes `!cut` from
that line. So under the mutation the anchor stopped matching, `cut` panicked
with "no longer ends with", and the spec went red having proved nothing
about the parser — it had proved its own anchor. A spec that fails loudly
for the wrong reason still passes a careless reading of "watched red".

The anchor now ends at the strtod fallthrough and names nothing under test.
With that, the mutation is caught properly: 2,099 of 1,186,978 fast-path
takes disagree, the first at `43270000000000000011e20`, one ULP low —
the same shape as `4409065699.4409065699e-2`, which is the case kanso#1423
was opened on. The rule the miss teaches: AN ANCHOR MAY NOT MENTION THE
THING UNDER TEST.

No counter moves; this file adds a test and touches nothing the compiler
builds.

## 2026-09-14 — a float a program writes down has few digits, and ryu took them off one at a time

`render_ryu` is 4.49% of the run program and 10.62% of encodebench, at
exactly 468.5 instructions a float in both — 191,070 calls in one and 849,200
in the other, and the same number per call to one decimal place. That
agreement is the first thing worth noticing: whatever the cost is, it does
not depend on which corpus the floats came from.

Instruction-level callgrind says where it goes. One sixteen-instruction block
at 0x12050 runs **9.41 times a call** and carries 127,916,800 of encodebench's
397,836,000 — 32% of the function. It is the general digit-removal loop:

    mov %rsi,%r9 / mov %rdx,%rcx / mov %rsi,%rax
    mul %rbp / mov %rdx,%rsi / shr $0x3,%rsi / inc %ebx
    mov %r8,%rax / mul %rbp / mov %rdx,%r8
    mov %rcx,%rax / mul %rbp
    shr $0x3,%r8 / shr $0x3,%rdx / cmp %rdx,%r8 / ja

Three multiply-highs and three shifts — `vp / 10`, `vm / 10`, `vr / 10` — a
counter, a compare and the branch. LLVM had already sunk the `vr % 10` and
the `round_up` out of the loop, because only the last trip's value survives.

Nine and a half trips is a lot, and the reason is the corpus rather than the
algorithm. `vr` starts with seventeen significant digits. A float a program
writes down — a price, a coordinate, a measurement — has three or four, so
thirteen or fourteen come off, and the loop takes them one at a time.

## the loop that replaced them costs twenty-one, not sixteen

Measured after the change, on a freshly built `runbench` — and the
rebuild is the point, because the binary sitting in the worktree
predated `src/runtime.c` by four minutes and profiling it read
1,994,173,231, the old shape's number. Rebuilt: 1,988,869,261, a fall of
5,303,970 (-0.2660%), which is the A/B figure recovered from a second
direction.

`render_ryu` is 84,209,130 instructions, 4.23% of runbench, 440.7 a
float over 191,070 calls. It was 4.49% and 468.5 before.

The new loop is block `0x39190`-`0x391d2`. It runs 1,022,310 times,
5.35 trips a float, and is 25.49% of the function at 112.4 instructions
a float. The old pair ran 9.41 trips at sixteen each, about 150.

**Twenty-one instructions a trip, not sixteen.** The premise this change
was built on is that the hundred-step and the ten-loop each cost
sixteen, so one trip taking two digits beats two taking one. That was
true of the code being replaced. The fused loop is not that body
unchanged: it is twenty-one instructions, and seven of them are
register moves that carry `vp`, `vm` and `vr` around the back edge.
Disassembled:

    39190:  mov %rsi,%r11          391b0:  mov %rcx,%rax
    39193:  mov %rcx,%r9           391b3:  shr $0x2,%rax
    39196:  mov %rdx,%r8           391b7:  mul %rdi
    39199:  mov %rsi,%r10          391ba:  mov %r8,%rax
    3919c:  shr $0x2,%r10          391bd:  shr $0x2,%rax
    391a0:  mov %r10,%rax          391c1:  mov %rdx,%rcx
    391a3:  mul %rdi               391c4:  mul %rdi
    391a6:  mov %rdx,%rsi          391c7:  shr $0x2,%rcx
    391a9:  shr $0x2,%rsi          391cb:  shr $0x2,%rdx
    391ad:  add $0x2,%ebx          391cf:  cmp %rdx,%rcx
                                   391d2:  ja  39190

So the trade is 21 against 32, not 16 against 32 — a narrower margin
than the sentence in the commit implies, and the measured -0.2660%
sizes it correctly either way. The correction is recorded because the
number 16 would otherwise be read back as this loop's cost.

Two things the disassembly settles that the C does not. The `vr % 100`
that feeds `round_up` does not appear in the loop at all: only the last
trip's value survives, so LLVM sank the modulo past the back edge. And
the three `mul %rdi` are the three divisions by 100, sharing one
reciprocal in `%rdi`.

**The next lead, not taken here.** Seven of the twenty-one are moves
the loop would not need if the body wrote its results into the
registers it reads. That is 37 instructions a float, 8.5% of
`render_ryu`, about 0.36% of runbench — real, and a separate change
with its own measurement.

The rest of the function, same sitting, by straight-line run:

    0x38fbf-0x39092   62 instrs   once a float    62.0   14.07%
    0x39570-0x3959d   13 instrs   2.79 trips      36.3    8.24%
    0x39770-0x3978f   11 instrs   2.88 trips      31.7    7.19%
    0x38e1e-0x38e85   22 instrs   once a float    22.0    4.99%
    0x3936e-0x393b9   26 instrs   0.71 trips      18.5    4.20%

The 62-instruction run is unconditional setup, once per float, and is
the largest single non-loop cost left in the function.

## the hundred-step was already there and fired once

    int round_up = 0;
    uint64_t vpd100 = vp / 100, vmd100 = vm / 100;
    if (vpd100 > vmd100) { ... removed += 2; }
    for (;;) { ... the ten-loop ... }

Two digits for the same three multiply-highs the ten-loop spends on one, and
it ran once. It is a `for (;;)` now. The shapes are otherwise identical: the
same rounding test on the two removed digits (`vrm100 >= 50` is "is the tail
at least half of a hundred", which is what `vrm >= 5` is for ten), and the
ten-loop still runs afterwards to take a last odd digit.

Measured on one container sitting, four binaries built from one tree with
equal-length names, all four run from the repository root:

    encodebench  3,747,072,758 -> 3,723,499,558   -23,573,200  -0.6291%
    runbench     1,994,263,401 -> 1,988,959,431    -5,303,970  -0.2660%

The bytes out are identical on both programs. That was checked by diffing the
output of all four binaries, and it is the only check that matters here: the
loop decides how fast the digits arrive, never which digits they are.

## the first reading was of two dead programs

The first output comparison ran the four binaries from the scratch directory
they were built into, and both "agreed" on fifteen bytes. Callgrind then read
315,755 instructions for a program that takes three and a half billion.

That is the second trap in `bench/instructions_golden.txt`'s own header,
written down after it cost somebody a reading before: the benchmarks resolve
their data relative to the working directory, so a run from anywhere else
dies at the first open and exits clean. Two dead programs agree about
everything. Re-run from the repository root, both produced `done: 74072800`
and the comparison meant something.


## CI's sitting, and the one vein that went the other way

CI measured the pair loop on the base kanso#1423 left. Five of the fourteen
work rows fall and nine hold to the digit:

    encodebench  3,641,023,556 -> 3,616,600,356   -24,423,200   -0.6708%
    livebench    3,115,992,776 -> 3,091,569,576   -24,423,200   -0.7838%
    runbench     2,003,046,621 -> 1,997,551,401    -5,495,220   -0.2743%
    oneshot         19,292,922 ->     19,231,864       -61,058   -0.3165%
    widebench       33,142,691 ->     33,134,691        -8,000   -0.0241%

encodebench and livebench fall by the same 24,423,200 because they are two
programs over one encode path and the floats in them are the same floats. The
container read -0.2660% on runbench against CI's -0.2743%, a ratio of 1.031.
An earlier sitting on the kanso#1418 base read the same five deltas to the
instruction; kanso#1423 moved the levels and the saving did not move with
them.

The machine-code vein rises: text 1,734,364 -> 1,735,036, +672 summed, and
EVERY ONE of its fourteen rows rises by exactly 48. That uniformity is the
finding. The fused loop lives in src/runtime.c, every binary links the same
runtime, and the `for (;;)` body is 48 bytes longer than the `if` it replaced
— so the vein records one number fourteen times. A change in the emitter would
spread unevenly across these rows instead, because the programs differ in what
they emit; this one cannot. The 2026-09-05 gavel keeps machine-code size out
of welfare, so nothing prices the 48 bytes against the runtime falls they buy.
The trade is stated here and the objective does not weigh it.

The three compile rows rise as layout: compile_instructions 42,870,366 ->
42,872,197 (+1,831), entry_instructions 144,035,949 -> 144,042,077 (+6,128),
library_instructions 144,836,225 -> 144,842,133 (+5,908). `kanso check` stops
before codegen and no decision they count can be altered by a runtime edit,
but src/runtime.c is compiled into the compiler and its bytes move what sits
where. On the kanso#1418 base the same diff read -8,125, -21,078 and -22,292
on the same three rows: same bytes, opposite sign, which is the whole of what
a layout delta carries. compile_allocs held at 27,937 and compile_memory is
byte-identical, the pair that really cannot move.

Welfare 68.73 -> 68.75, banked in the same commit.

## 2026-09-14 — the string arm is asked first, and a list index pays for it

`k_b_at` is what `at` compiles to, and it answers five container kinds by
asking their tags in order. The list arm stood first. runbench indexes text
690,000 times and lists 7,900, so the list test was two instructions the
common case paid to be told no.

The arms swap. The string arm is asked first; every other arm keeps its
place. Nothing else in the function changed, and the bytes out are the same,
so no golden but the work vein moves.

Measured on the container, `env -i` under callgrind, both binaries copied
into one directory under EQUAL-LENGTH names:

    runbench     1,994,173,291 -> 1,992,793,291  -1,380,000  -0.0692%
    indexbench       3,226,622 ->     3,186,624     -39,998  -1.2396%

The other twelve work rows are byte-identical. 1,380,000 is 690,000 calls
times exactly two instructions, which is the attribution the profile gave
before the change was written.

The whole vein was read rather than runbench alone, on purpose: a list index
now pays the two instructions a string index stopped paying, and a row that
rose would have been the trade to state. None rose.

**Three harness traps cost three wrong readings on the way, and all three are
already written into `bench/instructions_golden.txt`'s own header.** The
first: binaries named `base-runbench` and `new-runbench` differ by one
character of exec path, and the kernel puts that path on the new process's
stack for libc to walk before main — four benchmarks read exactly -14 and one
+14, which is the artifact and not the change. Equal-length prefixes
(`aaa-`/`bbb-`) fix it. The second: the benchmarks resolve their data
relative to the working directory, so running them from `/tmp/ab/aa` and
`/tmp/ab/bb` gave eight rows near 225,000 — a work row that small means the
program DIED, and running one by hand says so: `cannot read
bench/large.json: no such file`. One directory, `bench` symlinked beside the
binaries. The third is the container's own: `scripts/gates/instructions.sh`
refuses on this host (glibc 2.39-0ubuntu8.7 / clang 18.1.3 against the
golden's 8.9 / 19.1.1) and never measures at all, which is why these rows are
a direct callgrind sitting and the golden is regenerated from CI's.

Row `list_test_first`, mutation `a_list_test_every_string_index_pays.sh` —
an awk block swap that puts the list arm back in front, verified by sorting
both files to prove the mutation moves lines and writes none.

Under the mutation, on the same sitting and against the shipped binary:
runbench 1,992,792,731 -> 1,994,172,731 (+1,380,000 / +0.0692%) and
indexbench 3,186,064 -> 3,226,062 (+39,998 / +1.2555%). Exactly the negative
of the change, to the instruction.

**CI's rows, and the container projected half of both.** The goldens now hold
what the runner measured on the base kanso#1415 left:

    runbench     2,003,021,871 -> 2,000,261,871  -2,760,000  -0.1378%
    indexbench       3,265,296 ->     3,185,298     -79,998  -2.4500%

The other twelve are byte-identical, so the trade this entry went looking for
does not appear anywhere in the vein: no row rose.

The container's sitting above projected -1,380,000 on runbench, exactly half
of CI's -2,760,000, and -39,998 on indexbench against -79,998, half and one
less. That reading was taken on a host `instructions.sh` had already refused,
and the refusal is usually described as a levels problem — two hosts count the
same program differently, so a row measured on one cannot be compared against
a row measured on the other. This is the first case in the log where the
refusal moved a DELTA instead. The change removes a fixed cost per call to
`k_b_at`, so the delta is the call count times the saving, and the call count
is an inlining decision: the runner's clang reaches the reordered test twice
as often as the container's. From a refused host, project the sign, not the
size.

Machine code moved with it, in `bench/text_golden.txt`: twelve of fourteen
rows rise, +32 bytes on eight and +48 on four, with escapebench and readbench
byte-identical. The reorder lives in `src/runtime.c` and every program links
it, so the cost is shared; the spread is which arms a program's own code makes
reachable. That vein carries no welfare term.

Summed over the fourteen binaries that vein reads `text 1,734,268 ->
1,734,716`, a rise of 448 bytes: eight times 32 and four times 48.

The trend gate refused the branch until that figure was written down, and the
mechanism is worth recording. It searches the branch's log delta for a
worsened counter's name AND for the value it landed on, as two separate
matches. This entry used the word `text` three times and quoted only the
per-row deltas, so the name matched and the number did not. The number is the
half that carries the weight: a name on its own licenses every later move of
that counter on the branch, which is how a mutation once set a named counter
to 999,999,999 and left this gate green.

The three compile veins moved too, which is the layout prior CLAUDE.md
records for any edit to the compiler's own bytes: `src/runtime.c` is
`include_str!`'d into the compiler, so its length and contents move the
binary the compile gates measure even though `kanso check` never runs a line
of it. On the kanso#1415 base CI read compile_instructions +35, entry -1,854
and library +230 against that base's goldens — three digits on a
147-million-instruction row, which is layout and nothing else.
`compile_allocs` and `compile_peak_bytes` were both byte-identical, as they
have to be: no allocation site changed.

CI measured them again after kanso#1416 merged in, and read this:

    compile_instructions   44,031,424 ->  44,032,307    +883  +0.0020%
    entry_instructions    146,767,592 -> 146,765,356  -2,236  -0.0015%
    library_instructions  147,572,025 -> 147,569,147  -2,878  -0.0020%
    summed                338,371,041 -> 338,366,810  -4,231  -0.0013%

**The three rows disagree in sign**, and that is the tell worth keeping: the
module row rose while the other two fell, on one binary, from one change. A
decision the front end makes differently moves all three the same way, because
all three run the same passes. Layout does this instead. `compile_allocs`
held at 28,361 and `compile_peak_bytes` at 776,055, which is the other half of
the same statement — the compiler asks exactly what it asked before and pays
for it in a differently-arranged binary.

That sitting is superseded. kanso#1414 landed on main afterwards and took
compile_instructions to 43,910,543, entry to 146,573,721 and library to
147,378,070, so the three rows above are deltas against a base that is gone.
A layout delta does not carry across a rebuild the way a removed decision
does — the arrangement it describes is the arrangement of one binary — so the
three veins go back to main's values and CI measures the reorder again on the
merged base. Only the work vein's bank stands, and it stands because no
compile row here has ever reached the objective's resolution: four thousand
instructions on three hundred and thirty-eight million is a ten-thousandth of
a per cent.


## 2026-09-14 — the pow5 table's low word is loaded once a call, and LLVM had already sunk it

`k_el_parse` reads three fields out of `k_el_pow10[q - K_EL_POW10_MIN]`: the
high word and the binary exponent, which every call uses, and the low word,
which only the refinement arm uses. That arm runs when the truncated product
sits within 2^-9 of a rounding boundary — one call in 512 by construction.
The source loads all three at the top, so by reading it the low word looks
like a load per call paid to serve one call in five hundred.

Sinking it into the arm is a three-line edit and it buys nothing:

    runbench    1,994,263,341 -> 1,994,263,341
    jsonbench   1,233,334,045 -> 1,233,334,045

Byte-identical, both programs, callgrind on this container, equal-length
binary names in one directory. The load is from a `const` table with no
intervening store, so LLVM sinks it to its use without being asked, and the
source position of the declaration is not the position of the load.

The general form is worth keeping: on a `static const` table, moving a read
closer to its use is a comment, not an optimisation. What LLVM cannot sink is
a read whose address depends on something the compiler cannot prove
unchanging, or one whose sinking would cross a call it must assume writes
memory. Neither holds here.

This closes the first of the two shapes the decode number-path attribution
named. No counter moves, and nothing is committed to `src/`.

## 2026-09-15 — a float's leading zeros are skipped once, and the digit loops stop asking whether the first nonzero has landed

`k_b_to_float` scans a decimal into a nineteen-digit significand and a
power of ten, then hands the pair to eisel-lemire. Both digit loops counted
significant digits with `if (w) digits++`: the predicate is false until the
first nonzero digit lands and true forever after, so every digit past the
first re-asked a question with one answer. In the disassembly that was
`xor / setne / add`, three of the eighteen instructions a fraction digit
cost.

The loops count unconditionally now. Leading zeros are skipped in front of
the integer loop, and in front of the fraction loop when the integer part
was empty or all zeros; a zero after the point still moves `q` down, as it
did. The `(w, q)` handed on is identical for every input: a leading zero
never changed `w`, and a fraction zero only ever changed `q`.

**The harness first.** `tests/every_float_literal_parses_like_strtod.rs`
lifts the real scan out of src/runtime.c, so it read this shape and not a
copy:

    1,193,654 parsed    599,826 took the fast path    0 disagree

Measured on the container, `env -i` under callgrind, both binaries in one
directory under equal-length names, run from the repository root:

    runbench    1,987,513,451 -> 1,985,349,707   -2,163,744   -0.1089%
    jsonbench   1,233,280,889 -> 1,230,002,489   -3,278,400   -0.2658%

The whole of runbench's fall is inside `k_b_to_float`: 42,275,772 ->
40,112,028 inclusive, the same 2,163,744 to the instruction, 10.3 a call
over 210,177 calls. jsonbench's fall is the same function too, 64,054,200
-> 60,775,800, and its floats carry more digits: 150 decodes of the large
document, and the saving per decode is 21,856. Task #539 projected about eighteen a call from three
instructions on six digits; the corpus's floats carry fewer digits than
that, and the saving is what the digits there are worth. Output is
byte-identical on both programs.

**A reading that was wrong on the way, and why.** The first jsonbench A/B
read +5,637,632, and every rise was in `k_b_push_mut`, `k_mklist`,
`k_map_lit` and the slice helpers — functions this change does not touch.
The binary was 912 bytes larger than the base. A counter sweep had been
started in the same worktree while the A/B was still building, the sweep
built `jsonbench --counters`, and the A/B's `mv` took that binary as its
own. The counter sites the sweep instruments are exactly the functions
that rose. Rebuilt alone, the reading is the one above.

Row `leading_zeros`, mutation
`every_digit_asks_whether_the_first_nonzero_has_landed.sh`: it deletes
both skips and puts `if (w)` back on both loops, and the result diffs
against main's `k_b_to_float` in nothing but this change's comment.

**CI's sitting, on the base kanso#1417 left.** The work vein reads the
container's two deltas to the instruction: runbench 1,994,791,401 ->
1,992,627,657 (-2,163,744, -0.1085%) and jsonbench 1,250,475,761 ->
1,247,197,361 (-3,278,400, -0.2622%). Four more rows fall: widebench
33,134,691 -> 33,028,905 (-105,786), and encodebench 3,616,578,500,
livebench 3,091,547,720 and oneshot 19,210,008 each by 21,856, one decode
of the large document. Eight rows hold to the digit, since nothing in them
parses a float.

Six machine-code rows RISE by exactly 96 bytes, the two skip loops:
jsonbench text 118,450, encodebench 138,962, oneshot 128,482, widebench
147,970, livebench 130,034 and runbench 305,362; summed 1,735,484 ->
1,736,060 (+576). Eight rows do not move, because the linker keeps
`k_b_to_float` only where it is called.

The three compile rows RISE by layout, as every runtime-only change moves
them: compile_instructions 42,870,872 -> 42,871,412 (+540),
entry_instructions 144,037,300 -> 144,040,625 (+3,325),
library_instructions 144,837,840 -> 144,841,583 (+3,743). `kanso check`
never reaches the runtime; the bytes of src/runtime.c shift what sits
where. Regenerated from the base CI ran, stated, not reasoned from.
compile_allocs held at 27,937 and compile_memory is byte-identical.

## 2026-09-15 — the int parser's cold tail leaves the frame it was sizing

`k_b_to_int` parses a decimal in place: a digit loop over at most eighteen
bytes, then `strtoll` and two refusals for anything the loop did not
accept. The loop needs no stack at all, but the calls after it did:
`strtoll`, `k_str_n`, `k_concat` and `k_err` each clobber the argument
registers, so the compiler pinned the string's data, length and origin in
callee-saved ones, and the function opened with five pushes and a frame
reservation and closed with five pops. Every call paid them, and neither
benchmark ever reaches the tail — `k_b_to_int_slow` is called zero times
in runbench and zero in jsonbench. Twelve instructions a call, attributed
instruction by instruction with `--dump-instr=yes` in task #547.

The tail is its own function now, `noinline, cold`. The fast path's entry
is one `push %rax` for alignment, and the tail keeps the frame it always
needed, for the calls that reach it.

Measured on the container, `env -i` under callgrind, both binaries in one
directory under equal-length names, run from the repository root, on the
base kanso#1427 leaves:

    runbench    1,985,349,707 -> 1,982,862,035   -2,487,672   -0.1253%
    jsonbench   1,230,002,489 -> 1,226,233,289   -3,769,200   -0.3064%

The whole of both falls is `k_b_to_int`'s own count: 21,770,496 ->
19,282,824 over 207,306 calls in runbench, 32,985,600 -> 29,216,400 over
314,100 in jsonbench: twelve a call, to the instruction, in both. Output is
byte-identical on both programs.

**The float parser's tail, measured and declined.** `k_b_to_float` ends the
same way, with `strtod` behind the fast path, and the same cut was built
beside this one. It is worth 106,821 on runbench and 161,850 on jsonbench,
a twenty-third of the int tail's fall, and the seven pushes stay: the
eisel-lemire fast path is inlined into the function and pins its own
callee-saved registers, so removing the libc call removes nothing from the
entry. The int loop had no such neighbour. The shape is not shipped, and
the harness's closing anchor stays where it is.

Row `int_cold_tail`, mutation
`the_int_parser_s_cold_tail_shares_its_frame.sh`: it rewrites the tail's
attribute to `always_inline`, which puts the five pushes back. Under the
mutation runbench reads 1,985,349,707, the base's count to the
instruction, and `k_b_to_int` is back at 21,770,496.

**CI's sitting, on the base kanso#1427 left.** The work vein reads eleven
a call where the container read twelve: runbench 1,992,627,657 ->
1,990,347,291 (-2,280,366, -0.1144%) over 207,306 calls and jsonbench
1,247,197,361 -> 1,243,742,261 (-3,455,100, -0.2770%) over 314,100, both
to the instruction. The runner's clang keeps one instruction of frame the
container's discards. encodebench 3,616,555,466, livebench 3,091,524,686
and oneshot 19,186,974 each fall by 23,034, one decode of the large
document; nine rows hold to the digit.

Six machine-code rows FALL by exactly 976 bytes: jsonbench text 117,474,
encodebench 137,986, oneshot 127,506, widebench 146,994, livebench
129,058 and runbench 304,386; summed 1,736,060 -> 1,730,204 (-5,856). The
tail's two refusals were laid out twice inline and once as a function.

The three compile rows RISE by layout: compile_instructions 42,871,412 ->
42,872,288 (+876), entry_instructions 144,040,625 -> 144,041,140 (+515),
library_instructions 144,841,583 -> 144,841,869 (+286). compile_allocs
held at 27,937 and compile_memory is byte-identical.

## 2026-09-15 — the cold helpers stop clobbering their callers' registers

A hot runtime function that calls nothing on its usual path still opened
with a frame when its unusual path called something. `k_b_slice_raw` is
two comparisons and a sixteen-byte store, and it opened with three pushes
because `k_bytes_view` bumps the arena and the bump's refill is a call;
the compiler pinned the slice's bounds in callee-saved registers so they
would survive a call a run makes 1,278 times, and 417,483 calls paid the
pushes to keep them there. `k_map_lit`, `k_rec`, `k_b_append_slice` and
`k_b_push` had the same shape over the same refill.

The refill carries `preserve_most` now, and so do five more helpers a hot
function calls and nothing else: `k_alloc_perm`, `k_b_to_int_slow`,
`k_ascii_fill`, `k_b_at_wide_miss` and `k_b_at_rest`. A preserve_most
callee saves every register it touches, so a caller may leave its live
values in caller-saved registers across the call, and a function whose
only calls are to these helpers opens with no pushes. The helper pays the
saves instead: the refill went from 37 instructions a call to 49, over
1,278 calls. clang emits the convention on x86-64 and aarch64, which is
every host this runtime is compiled for; a wasm target would ignore it
with a warning, and the runtime is never compiled for one.

Three cuts went with it, each measured on its own:

- `k_b_at`'s two arms that allocate or refuse -- a wide character the
  cache does not hold, and a map or an unindexable value -- are separate
  cold functions. Alone that was runbench -3,794,934: the index lost its
  forty-byte reservation and one push, and kept five for `k_str_n`'s
  ascii-cache fill, which was inlined into it.
- The fill is its own preserve_most function, `k_ascii_fill`. With it out
  of line the inliner declined `k_str_n` at two sites, `k_b_slice` and
  `k_b_utf8_slice_raw`, and 205,098 calls a run paid twenty-three
  instructions each: the composite read -11,810,628 with 4,712,914 of
  frame given back in `k_str_n`'s own row.
- `k_str_n` is `always_inline`. Nothing emitted calls it, though the IR
  declares it. That took the last 1,984,491.

Measured on the container, `env -i` under callgrind, both binaries in one
directory under equal-length names, run from the repository root, on the
base kanso#1428 leaves:

    runbench    1,982,862,035 -> 1,969,066,916   -13,795,119   -0.6957%
    jsonbench   1,226,233,289 -> 1,211,697,168   -14,536,121   -1.1854%

The refill alone was runbench -5,195,902 and jsonbench -7,011,888, four
instructions a call to the instruction in each of the five functions
named above and two in `k_b_utf8`, with the binary the same size to the
byte. Output is byte-identical on both programs at every step.

**The two shapes this rules out.** Outlining a cold tail without the
attribute was kanso#1428, and it only clears the frame when the tail was
the frame's only reason; `k_b_at` kept five pushes that way. The attribute
without the outlining reaches only the helpers that already exist; the
index's arms and the ascii fill were inline, so there was nothing to mark.
Each half needed the other, which is why they ship together.

Rows `cold_registers` and `index_cold_arms`. The first mutation strips
`preserve_most` from all six helpers; the second inlines the index's two
arms back.

**CI's sitting, on the base kanso#1428 left.** The work vein reads
runbench 1,990,347,291 -> 1,977,087,740 (-13,259,551, -0.6662%) and
jsonbench 1,243,742,261 -> 1,229,738,040 (-14,004,221, -1.1260%), where
the container's A/B read -13,795,119 and -14,536,121: the runner's clang
keeps a little more frame than the container's, as it did on kanso#1428.
Eight more rows fall, deepbench -2,179,955, widebench -191,892,
encodebench -115,872, indexbench -99,535, livebench -97,289, oneshot
-90,495, digestbench -23,820, pendbench -7,166. Four RISE, and they are
the outlining's own price: work_basket 34,281,871 -> 34,433,106
(+151,235, +0.4412%), work_escapebench 82,969,017 -> 82,999,058 (+30,041),
work_scanbench 528,870,249 -> 528,872,901 (+2,652), work_readbench
4,629,745 -> 4,629,808 (+63). An index whose container is a map reaches `k_b_at_rest` by a call
now, the ascii fill is a call, and the refill saves every register it
touches on each call, so a program that takes those paths and holds little
live across them pays and collects nothing. Which of the three basket pays
was not attributed.

All fourteen machine-code rows FALL, 928 to 2,432 bytes each; summed
1,730,204 -> 1,707,852 (-22,352). The container sized the refill's
attribute alone as neutral and never sized the composite: the pushes and
pops a caller no longer opens with are bytes, and every binary calls the
refill.

The three compile rows FALL by layout: compile_instructions 42,872,288 ->
42,870,366 (-1,922), entry_instructions 144,041,140 -> 144,035,949
(-5,191), library_instructions 144,841,869 -> 144,836,225 (-5,644).
compile_allocs held at 27,937 and compile_memory is byte-identical.

## 2026-09-15 — a long copy is a cold call, and the slice door's rare arms are too

Two more frames of the shape the preserve_most entry above describes.

`k_map_lit` and `k_mklist` copy their items inline when there are a few
and call memcpy when there are more, and the call is why both opened with
three pushes: 273,339 map literals and 300,479 list literals a run paid
them for a copy 98 of them and 16,011 of them respectively make. The
same call sat inside `k_copy_short`, the string copy inlined into every
string builder, behind its sixteen-byte threshold. All three go through
`k_copy_cold` now, a preserve_most wrapper around memcpy, and the three
functions open with no pushes. The wrapper costs twenty-one a call over
26,551 calls, and `k_b_join`, whose copies are long and many, pays
199,822 of them alone.

`k_b_utf8_slice_raw`, the decoder's token door at 861,498 calls a run,
opened with three pushes for the two utf-8 validators it calls 16,929
times, and the validators could not carry preserve_most themselves:
`k_b_utf8` calls the scalar one 117,513 times a run on its own strings,
and measured that way the attribute cost 1,613,304 there for what it
saved here. So the slice door validates through `k_utf8_bad_rare`, the
same ascii test with the two validators behind preserve_most wrappers,
and its entry is one `push %rax`; the wrappers cost seventeen a call over
16,929.

Measured on the container, `env -i` under callgrind, both binaries in one
directory under equal-length names, run from the repository root, on the
base kanso#1429 leaves:

    runbench    1,969,066,916 -> 1,962,311,522    -6,755,394   -0.3431%
    jsonbench   1,211,697,168 -> 1,200,857,142   -10,840,026   -0.8946%

`k_b_utf8_slice_raw` falls five a call, `k_map_lit` six, `k_mklist` five,
`k_b_append_grow` two, to the instruction, and `k_utf8_bad_scalar` does
not move. Output is byte-identical on both programs.

Rows `cold_copy` and `rare_door`. The first mutation inlines the wrapper
back, the second sends the slice door through the plain validator door.

**CI's sitting, on the base kanso#1429 left.** The work vein reads
jsonbench 1,229,738,040 -> 1,218,898,014 (-10,840,026, -0.8815%), the
container's A/B to the instruction, and runbench 1,977,087,740 ->
1,970,132,223 (-6,955,517, -0.3518%), 200,123 deeper than the container's
-6,755,394. Seven more rows fall: deepbench -259,997, oneshot -72,222,
encodebench -70,214, livebench -63,843, work_escapebench 82,999,058 ->
82,993,058 (-6,000), scanbench -4,817, pendbench -3,407. Five RISE, and they are the wrapper's price:
work_digestbench 9,813,332 -> 9,944,479 (+131,147, +1.3364%), work_basket
34,433,106 -> 34,534,020 (+100,914, +0.2931%), work_widebench 32,837,013
-> 32,845,030 (+8,017), work_indexbench 3,085,763 -> 3,086,146 (+383),
work_readbench 4,629,808 -> 4,629,832 (+24). A copy of sixteen bytes or
more goes through `k_copy_cold` now, which saves every register it touches
before memcpy, so a program whose copies are long and frequent pays that on
each one and keeps no frame it did not already keep. The digest builds
such copies.

Machine code: six rows RISE by 96 or 64 bytes and four FALL by 16 or 32;
summed text 1,707,852 -> 1,708,284 (+432). The six that rise link the
slice door and its two new wrappers; the four that fall link
`k_copy_short` and lost the inline memcpy dispatch a call replaces.

The three compile rows RISE by layout, and to the instruction they are the
values the rows held before kanso#1429: compile_instructions 42,870,366 ->
42,871,412 (+1,046), entry_instructions 144,035,949 -> 144,040,625
(+4,676), library_instructions 144,836,225 -> 144,841,583 (+5,358).
compile_allocs held at 27,937 and compile_memory is byte-identical.

**Round three: the render harness lifts the copy it calls.**
`tests/every_rendered_float_reads_back_as_itself.rs` cuts `k_copy_short`
out of runtime.c by its declaration line and compiles it beside ryū. This
change made the short copy hand its long case to `k_copy_cold`, and the
lifted text called a function the harness never carried, so the spec
failed to compile on both hosts and the ratchet reported its gate ALREADY
RED. The cut now runs from `k_copy_cold`'s declaration to the short copy's
closing brace, one span, so a later change to either shape is still read
from the source and not from a copy. Green on the container in 4.09s.

## 2026-09-15 — the third cold-frame sweep: nine more rare arms, and a scan that `cold` cost its vector loop

kanso#1429 and kanso#1430 took the frames the arena refill, the index's
arms and the long copy were charging. This entry took the attribution
again on the cold-copy build, per function: callgrind's dynamic call
counts against each function's prologue push count, ranked by calls times
pushes. Seven hot functions still opened with five to seven pushes for
calls a run makes a few thousand times: `k_b_at` (six, 690,000 calls),
`k_b_entries` and `k_map_sorted` (six each, 248,490), `k_b_push_grow`
(six, 252,499), `k_b_slice` (seven, 183,682), `k_b_append_grow` (six,
176,697), `k_closure` (five, 243,978) and `k_copy_alloc` (seven, 74,551).

**Nine rare arms become preserve_most helpers.** The character scan
behind `k_str_chars` (`k_str_chars_scan`: an index and a slice ask it,
and it runs once per string, 163 times a run); a map view's first build
(`k_map_sort_build`, 2,761 of 248,490 asks, so `k_map_sorted` inlines into
`entries`); `entries`' failing-field record (`k_rec_cold`); a list grow's
permanent buffer, its registration and its release (`k_buf_perm`,
`k_permreg_add`, `k_buf_release`: 15,488, 15,488 and 11,616 of 252,499
grows); a bytes grow's malloc regime and the free of a malloced
predecessor (`k_bytes_buf_malloc`, `k_bytes_buf_release`: 1,080 and 990 of
176,697); and the tenure tier's block opener (`k_ten_block_open`, 14 of
74,551 carves). Two more shapes ride with them: `k_closure` copies up to
eight captures as inline words and hands longer environments to
`k_copy_cold` (memcpy had been a plain call on 92,235 of its calls), and
`k_b_slice`'s multibyte walk is its own function in tail position, so the
ascii and list arms open with no pushes where the inline walk cost seven.
`k_copy_short` is `always_inline` as well: `k_b_slice` was calling it out
of line, and inlining it is worth 236,052 against the same tree.

Prologues after, read off the linked run program: `k_b_at` 6 -> 1,
`k_b_slice` 7 -> 1, `k_copy_alloc` 7 -> 1, `k_closure` 5 -> 1,
`k_b_push_grow` 6 -> 4, `k_b_entries` 6 -> 3, `k_b_append_grow` 6 -> 3.
The three that keep pushes hold more values live than the nine
caller-saved registers can carry; that is pressure, and no attribute
reaches it. (This paragraph first said `k_b_at` 5 -> 1 and
`k_b_push_grow` 6 -> 0. The linked binary says six and four: the grow's
four pushes and an alignment slot sit below its doubling loop now, so
the entry opens with none, and every one of its 252,499 calls still
reaches them. Corrected in round two from the per-instruction profile.)

Container A/B, `env -i` under callgrind, equal-length names in one
directory, on the kanso#1430 leaves:

    runbench    1,962,311,522 -> 1,938,188,043   -24,123,479   -1.2293%
    jsonbench   1,200,857,142 -> 1,193,215,104    -7,642,038   -0.6364%

Per function on runbench: `k_b_slice` 18,864,651 -> 8,265,702, with the
walk's 8,027,391 now standing on its own two calls; `k_b_at` -7,633,165;
`k_map_sorted` 6,341,332 -> 0 against `k_map_sort_build`'s 710,389;
`k_b_entries` -3,236,127; `k_b_length` -3,207,837 where the scan it inlined
now stands as `k_str_chars_scan`'s 4,290,788; `k_b_append_grow`
-2,368,359; `k_b_push_grow` -2,152,826; glibc's memcpy -2,029,170;
`k_copy_alloc` 2,255,292 -> 1,360,457. The helpers' own price: `k_buf_perm`
557,572 over 15,488 calls, `k_buf_release` 313,632, `k_permreg_add`
+247,806, `k_copy_cold` +316,701 on 15,081 more calls. And `k_eq_rec`
+279,330, four pushes to seven: it asks `k_map_sorted` twice, and the inline
copy carries the build's call into a function that compares maps 92,252
times a run without ever building one. Output byte-identical on both
programs at every step.

**`cold` on a loop is a different decision from `cold` on a tail.** The
first cut marked `k_str_chars_scan` `cold` like the other helpers, and the
composite read runbench -7,518,671. The scan inside it is `k_utf8_chars`,
a word-at-a-time loop clang vectorizes at -O3; a cold function is compiled
for size, and the loop came out scalar: 17,797,006 instructions over 163
calls, against 4,290,788 for the same helper without the attribute. That
one word was 13,506,218 of runbench, more than half of what the sweep
found. The helper keeps `noinline, preserve_most` and nothing else, and
the rule this leaves: `cold` is for an arm that does a few instructions'
work, never for one that carries a loop worth vectorizing.

Rows `sweep_helpers` (the mutation strips `preserve_most` from the nine
and leaves kanso#1429's six), `slice_walk` (inlines the walk back) and
`closure_caps` (four inline words and memcpy above), each proved red
locally against 1,938,188,043: the first reads runbench 1,945,409,548
(+7,221,505) and jsonbench +4,035,972, the second 1,940,575,869
(+2,387,826), the third 1,940,317,281 (+2,129,238). `k_map_sorted` spelt
`static` without `inline` builds byte-identically, so the word stays for
the reader and decides nothing.

**Declined on the way.** `k_b_append_range`'s fast-path memcpy replaced by
the short ladder: five pushes to none on the standalone compile, and
runbench +328,995, jsonbench -863,400, encodebench -1,200; the objective
declines it. Outlining `k_b_to_float`'s strtod tail left it at seven
pushes; that frame is the parse body's and no tail reaches it.

**CI's sitting, on the base kanso#1430 left.** The work vein reads
runbench 1,970,132,223 -> 1,945,875,866 (-24,256,357, -1.2312%), 132,878
deeper than the container's -24,123,479, and jsonbench 1,218,898,014 ->
1,211,426,976 (-7,471,038, -0.6129%), 171,000 shallower than the
container's -7,642,038. Eleven more rows fall: livebench -40,670,538
(-1.3156%), encodebench -40,612,772 (-1.1230%), scanbench -20,527,342
(-3.8814%), deepbench -14,043,480 (-3.8064%), basket -577,785 (-1.6731%),
indexbench -190,206 (-6.1632%), oneshot -56,233, widebench -49,649,
pendbench -30,424, digestbench -3,137, readbench -123. Against main the
digest row still stands above where the chain found it, work_digestbench
9,813,332 -> 9,941,342: that is kanso#1430's wrapper price less this
sweep's 3,137, and it is priced in that entry. One RISES here:
work_escapebench 82,993,058 -> 83,901,717 (+908,659, +1.0949%). That is
the list grow's price. Its permanent buffer, its registration and its
release are preserve_most helpers now, and escapebench grows a list
12,000 times a run, 9,000 of them releasing a predecessor: on the
container the three helpers and the long copy cost it 1,752,029
(`k_permreg_add` 888,017, `k_buf_perm` 432,004, `k_buf_release` 243,004,
`k_copy_cold` 189,004) against 846,030 the push itself no longer pays,
905,659 summed where CI read 908,659.

Machine code: nine rows RISE and five FALL, summed 1,708,284 ->
1,708,652 (+368). Nine helpers are out of line now, each with a
preserve_most prologue and epilogue, and the callers they left keep no
frame; which side is larger is per program, and runbench falls 832.

The three compile rows FALL by layout: compile_instructions 42,871,412 ->
42,869,908 (-1,504), entry_instructions 144,040,625 -> 144,035,862
(-4,763), library_instructions 144,841,583 -> 144,837,205 (-4,378).
compile_allocs held at 27,937 and compile_memory is byte-identical.

## 2026-09-15 — an empty list literal opens with room for four

The per-instruction profile of kanso#1431's run program put `k_b_push_grow`
at 25,260,844 instructions over 252,499 calls, a hundred a call, and the
calls were nearly all one shape: 225,621 came from `array_delim` in
lib/json, and the copy loop inside the grow ran 238,750 times over 238,498
calls, one element a call. The decoder opens every array with `[]`, and an
empty literal's buffer held one slot, so the first push fit and the second
one copied one element into a buffer of four. Every JSON array of two or
more elements paid the grow once. A one-slot buffer is also below the size
classes the free list keeps (a class needs `tzcnt >= 2`), so the outgrown
buffer was garbage the moment it was left, where a four-slot one is
recycled.

The empty literal's buffer opens with room for four now: `k_mklist` asks
`k_buf` for four slots where it asked for one. Nothing else changes. A
second variant rounded every literal under four elements up to four as
well; it read identical to the first within 80 instructions on both
programs, so the one-, two- and three-element literals are not pushed into,
and the smaller change ships.

Container A/B, `env -i` under callgrind, equal-length names in one
directory, on the kanso#1431 leaves:

    runbench    1,938,188,043 -> 1,923,227,681   -14,960,362   -0.7719%
    jsonbench   1,193,215,104 -> 1,177,989,804   -15,225,300   -1.2760%

Output byte-identical on both. The run program's counters move in the
direction the shape predicts: `push_mut_slow` 1,777,129 -> 1,638,371 and
`push_mut_fast` 961,384 -> 1,100,142 (138,758 grows become in-place
pushes), `buf_reuse` 56,614 -> 145,461 (the four-slot buffers come back
through the free list), `sh_buf` 119,476,704 -> 111,160,896, allocs
5,958,961 -> 5,730,654, `evac_allocs` 74,551 -> 68,315 and `survive_slots`
159,393 -> 110,799 (fewer one-slot buffers survive a rewind to be carried).
`arena_peak_bytes` falls 39,653,072 -> 38,604,496, exactly one 1 MiB block
(`arena_blocks` 37 -> 36). Two counters rise: `alloc_bytes` 426,179,869 ->
459,964,461 (+7.93%), because an empty literal that stays empty costs 80
bytes where it cost 32, and `perm_peak_bytes` 10,272 -> 20,512, because a
list that escapes its beat now escapes with a four-slot buffer. The
objective reads the peak, not the bytes allocated, and the peak fell:
welfare 68.95 -> 69.10 on the container from `run_peak_bytes` 40,391,384 ->
39,353,048 alone, with the instruction rows still CI's from kanso#1431.
Eleven cost goldens and twenty-five mem fixtures regenerated by the sweep.

`tests/sha256_peak.rs` pins the arena peak of hashing a pushed-together
message, and the four-slot literal moved where that message's buffer lands:
at 65,536 bytes the arena peak rises two blocks (17,825,808 -> 19,922,976)
while the permanent peak falls 1,310,720; at 131,072 the arena falls eight
blocks (40,894,496 -> 32,505,888) and the permanent peak rises 2,621,440.
CI turned the spec red on round two and the pins are re-set with those
figures beside them.

Row `empty_room`, mutation `an_empty_list_literal_has_no_room_for_a_push`
(puts the one-slot buffer back). Under the mutation the source is
kanso#1431's to the byte outside comments, so its count is the base's:
runbench +14,960,362.

**Every counter the trend gate calls worse, with the value it landed on.** A four-slot buffer where a one-slot one stood is 48 more bytes for every empty literal that stays empty, so `alloc_bytes` and `sh_buf` rise on the programs whose empty lists never grow, `perm_peak_bytes` and `perm_live_bytes` rise where a list escapes its beat with the larger buffer, and `bytes_freed` falls where fewer grows meant fewer permanent buffers to free: run_alloc_bytes 426,179,869 -> 459,964,461, run_perm_peak_bytes 10,272 -> 20,512, encode_alloc_bytes 658,041,744 -> 658,094,320, encode_sh_buf 73,214,624 -> 73,267,200, basket_alloc_bytes 4,900,609 -> 7,503,425, basket_bytes_freed 12 -> 11, basket_perm_live_bytes 2,228,256 -> 4,259,872, basket_perm_peak_bytes 2,752,560 -> 5,308,464, escape_alloc_bytes 32,976,112 -> 65,760,112, escape_perm_peak_bytes 10,272 -> 20,512, escape_sh_buf 96,000 -> 240,000, scan_alloc_bytes 160,539,964 -> 160,587,964, scan_sh_buf 33,024 -> 81,024, a_class_asks_by_the_byte_alloc_bytes 432,271 -> 468,175, a_class_asks_by_the_byte_sh_buf 82,448 -> 118,352, a_loop_invariant_capture_is_copied_every_rewind_alloc_bytes 91,136 -> 102,064, a_loop_invariant_capture_is_copied_every_rewind_sh_buf 10,976 -> 21,904, a_pushed_call_keeps_the_sweep_alloc_bytes 6,595,280 -> 13,152,080, a_pushed_call_keeps_the_sweep_perm_peak_bytes 10,272 -> 20,512, a_pushed_call_keeps_the_sweep_sh_buf 19,200 -> 48,000, an_escaped_list_gives_its_buffer_back_bytes_freed 400 -> 200, an_escaped_list_gives_its_buffer_back_sh_buf 6,400 -> 16,000, an_inner_beat_opens_its_tenure_in_the_block_outside_allocs 177,420 -> 180,196, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_allocs 29,377 -> 48,531, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_bytes 2,339,344 -> 3,006,224, an_inner_beat_opens_its_tenure_in_the_block_outside_ten_blocks 2 -> 3, build_cycle_alloc_bytes 3,168 -> 3,264, build_cycle_sh_buf 208 -> 304, early_exit_alloc_bytes 44,368 -> 88,064, early_exit_perm_live_bytes 32,784 -> 65,552, early_exit_perm_peak_bytes 40,992 -> 81,952, early_exit_sh_buf 32 -> 80, effect_push_shape_alloc_bytes 3,264 -> 3,456, effect_push_shape_sh_buf 704 -> 896, fold_push_shape_bytes_freed 5 -> 4, fused_map_shape_bytes_freed 5 -> 4, fused_reducer_bytes_freed 4 -> 3, fused_reducer_sh_buf 32 -> 80, fused_select_shape_bytes_freed 5 -> 4, fused_tally_alloc_bytes 32,240 -> 42,880, fused_tally_perm_live_bytes 8,208 -> 16,400, fused_tally_perm_peak_bytes 10,272 -> 20,512, piped_reducer_bytes_freed 4 -> 3, piped_reducer_sh_buf 32 -> 80, skip_shape_bytes_freed 5 -> 4, sort_shape_perm_live_bytes 8,208 -> 16,400, sort_shape_perm_peak_bytes 10,272 -> 20,512, take_shape_bytes_freed 5 -> 4, tally_shape_bytes_freed 5 -> 4, tally_shape_sh_buf 2,016 -> 2,064, the_same_capture_built_below_the_mark_is_shared_alloc_bytes 91,040 -> 101,968, the_same_capture_built_below_the_mark_is_shared_sh_buf 10,976 -> 21,904, unsafe_wrap_alloc_bytes 128 -> 176, unsafe_wrap_sh_buf 32 -> 80. The inner-beat tenure fixture moves the other way for the same reason, its `allocs`, `evac_allocs`, `evac_bytes` and `ten_blocks` landing where the list above says. The peak is what the objective reads, and it fell.

**CI's sitting, on the base kanso#1431 left.** The work vein reads runbench
1,945,875,866 -> 1,930,572,613 (-15,303,253, -0.7864%), 342,891 deeper
than the container's -14,960,362, and jsonbench 1,211,426,976 ->
1,195,571,676 (-15,855,300, -1.3088%), 630,000 deeper than the container's
-15,225,300. Seven more fall: pendbench -11,284,328 (-5.1424%), basket
-270,465, encodebench -128,117, oneshot -105,043, digestbench -102,348,
livebench -96,070, widebench -83,118; indexbench and readbench hold.
Against main the digest row still stands above where the chain found it,
work_digestbench 9,813,332 -> 9,838,994: kanso#1430's wrapper price less
kanso#1431's and this entry's falls, priced in that entry. Three RISE
here: work_scanbench 508,340,742 -> 508,351,545 (+10,803), work_deepbench
354,898,747 -> 355,470,728 (+571,981, +0.1612%), and work_escapebench
83,901,717 -> 85,995,606 (+2,093,889, +2.4956%). escapebench's rise is the
container's to the instruction (83,868,738 -> 85,962,627) and it is the
accumulator's new lifetime: a list a loop pushes into stays in the arena
through four pushes where one push filled the old buffer, and every rewind
in between carries it, where before the second push moved it to permanent
storage. On the container that is memcpy +738,000, malloc +626,895, the
free path +441,000, `k_beat_rewind_slow` +183,000, `k_b_push_mut`
+162,000. deepbench pays the same carry on a smaller scale.

Machine code: every row FALLS by 112 or 144 bytes, summed 1,708,652 ->
1,706,956 (-1,696): the one-slot buffer sat below the free list's size
classes and the grow had a branch for it, laid out once per program.

The three compile rows RISE by layout: compile_instructions 42,869,908 ->
42,871,308 (+1,400), entry_instructions 144,035,862 -> 144,038,199
(+2,337), library_instructions 144,837,205 -> 144,838,028 (+823).
compile_allocs held at 27,937 and compile_memory is byte-identical.

The book's two counter panels (ch10 `counters`, ch12 `fused`) quote the
allocation counters of a sample that opens an empty list, and the sweep
does not regenerate them; CI's book job caught both, allocs 10 -> 9 and
alloc_bytes 43,888 -> 22,032 on each, and they are rewritten here.

## 2026-09-15 — the two byte scanners are inlined, and their constants fold

`k_b_find2_raw` and `k_b_find2_below_raw` are the decoder's inner scans:
sixteen bytes a step under SSE, looking for the first of two bytes. On
kanso#1432's run program the per-instruction profile put them at
58,181,994 and 84,989,250 self over 1,756,429 and 1,353,330 calls, and
the loop body ran 1.01 times a call: the hit is in the first sixteen
bytes 99.99% of the time. Half of each call was setup, the two bytes
broadcast into vector registers, `movd`, `shl`, `or`, `movd`, `movd`,
`pxor`, two `pshufb`, rebuilt from the argument registers on every call.
Every emitted caller hands those bytes as literals (`text/find2 cs p 34
92`, and the same 34 92 at `find2_below`), so inlined they are constant
vectors loaded from rodata.

The link is LTO, so `__attribute__((always_inline))` on the two doors is
enough: LLVM honours it at every emitted call site and the out-of-line
copies vanish from the binary. Nothing else changes.

Container A/B, `env -i` under callgrind, equal-length names in one
directory, on the kanso#1432 leaves:

    runbench    1,923,227,681 -> 1,898,278,815   -24,948,866   -1.2972%
    jsonbench   1,177,989,804 -> 1,171,809,954    -6,179,850   -0.5246%

The first door alone read runbench -18,619,812 and jsonbench -6,179,850;
the second adds -6,329,054 to runbench and nothing to jsonbench, which
never calls it. Output byte-identical on both. Per function, the two
doors' 143,171,244 become 118,222,370 inside their callers:
`encode_onto` +78,660,646, `obj_key_start` +17,251,938 and +958,320 on
its two clones, `str_escape` +7,891,587, `array_delim` +4,526,379 and
+88,506, `str_chars` +3,510,540, `parse_value` +3,029,202,
`in_class?` +2,295,180, `worth_trying?` +10,272. No counter moves: the
scan allocates nothing and the sweep agrees with every golden.

Row `scan_inline`, mutation
`the_byte_scanners_rebuild_their_constants_on_every_call` (strips both
attributes). Under the mutation the source is kanso#1432's to the byte
outside comments, so its count is the base's: runbench +24,948,866.

**CI's sitting, on the base kanso#1432 left.** The work vein reads runbench
1,930,572,613 -> 1,904,577,350 (-25,995,263, -1.3465%), 1,046,397 deeper
than the container's -24,948,866, and jsonbench 1,195,571,676 ->
1,187,809,626 (-7,762,050, -0.6492%), 1,582,200 deeper than the container's
-6,179,850. Four more fall: livebench -32,587,731 (-1.0682%), encodebench
-8,047,675 (-0.2251%), scanbench -7,521,638 (-1.4796%), oneshot -133,071
(-0.7055%). The other eight hold to the instruction; none of them calls
either scanner. Against main two work rows still stand above it, neither
moved here: work_escapebench 82,969,017 -> 85,995,606 (+3,026,589), the
accumulator's lifetime priced in kanso#1432's entry, and work_digestbench
9,837,152 -> 9,838,994 (+1,842), kanso#1430's wrapper price less the falls
since.

Machine code: seven rows RISE and seven hold, summed text 1,706,956 ->
1,718,924 (+11,968): runbench +3,536, oneshot +2,816, livebench +2,816,
jsonbench +2,128, scanbench +464, widebench +160, encodebench +48. Every
emitted call site carries its own copy of the sixteen-byte scan loop now,
with the two bytes as rodata vectors, where before it carried a call.
Against main the sum is a fall, 1,730,204 -> 1,718,924, on the chain's
earlier links.

The three compile rows RISE by layout, both on the base and against main:
compile_instructions 42,871,308 -> 42,873,185 (+1,877; main 42,872,288),
entry_instructions 144,038,199 -> 144,044,710 (+6,511; main 144,041,140),
library_instructions 144,838,028 -> 144,845,808 (+7,780; main 144,841,869).
compile_allocs held at 27,937 and compile_memory is byte-identical.

Welfare 69.16 -> 69.26, banked in this pull request; the three page spans
quoting the moved compile goldens were rewritten by `golden_prose --write`.

## 2026-09-15 — a scan's short tail is one masked load

`k_b_find2_raw` and `k_b_find2_below_raw` scan sixteen bytes a step and then
finish whatever is shorter than a vector one byte at a time. On kanso#1433's
run program the per-instruction profile put 8,042,860 bytes a run through
those byte loops (counted at the loops' `cmp $0x22`, which the vector path
never executes), at eight instructions a byte: 5,375,790 in `encode_onto`,
1,655,478 in `obj_key_start`, 509,751 in `parse_value`, 326,304 in
`array_delim`, 175,537 in `str_escape`. The loop is reached whenever fewer
than sixteen bytes remain from the scan position, which for a json key or a
short value is every time: the two copies in `encode_onto` entered 860,130
and 347,220 times and walked 4.9 and 3.7 bytes an entry. kanso#1294 had
declined a word-at-a-time tail for the below-floor scan at +2.2879%, because
the mask's setup per entry cost more than the few bytes it replaced. With the
scanners inlined since kanso#1433 the setup is different: the byte pair and
the floor are rodata vectors already, so a tail is one unaligned load, the
same three compares the loop does, and a mask over the bytes that are the
string's.

The load reads sixteen bytes from a string that may hold three, so it is
taken only when `k_tail_window` says the sixteen stay inside the page the
string's next byte is in (`(p & 4095) <= 4080`): the page holding a valid
byte is mapped whole, so the load cannot fault, and the bytes past the end
are loaded and then masked off, so nothing the answer depends on is read from
outside the string. A tail that does cross a page edge takes the byte walk it
always took. aarch64 gets the same shape with the shrn-by-4 mask, four bits a
byte.

Container A/B, `env -i` under callgrind, equal-length names in one directory,
on the kanso#1433 leaves:

    runbench    1,898,278,815 -> 1,867,578,425   -30,700,390   -1.6173%
    jsonbench   1,171,809,954 -> 1,172,409,354      +599,400   +0.0512%

jsonbench RISES by a twentieth of a per cent: the decoder's tails are a byte
or two long (the closing quote right after the key), where two trips of the
byte walk are cheaper than one masked load with its window test. The run
program's tails average four to five bytes and it is the objective's term.
Output byte-identical on both.

The harness `a_short_scan_tail_answers_like_the_byte_walk` lifts both
scanners and the window test out of src/runtime.c and sweeps them against a
byte-at-a-time reference: every length 0..40, every start position, four byte
pairs, five floors, with the string placed at every offset in the last 64
bytes of a page whose next page is PROT_NONE, 836,400 cases. Watched red two
ways: the mask's `- 1` removed disagreed on 259,631 cases; the window test
answering yes unconditionally died on the guard page with SIGSEGV.

Row `scan_tail`, mutation `a_short_scan_tail_walks_a_byte_at_a_time` (the
window test answers no, so the byte walk is the only tail again).

The harness assumed a 4,096-byte page and the macOS job refused it: Apple
silicon maps 16 KiB pages, so `mprotect` at +4096 came back unaligned and the
harness exited 2 before a single case ran. It asks `sysconf(_SC_PAGESIZE)`
now. The window test in the runtime keeps its 4,095 mask, which is
conservative on a larger page: a sixteen-byte load that stays inside a
4 KiB-aligned window stays inside any page that contains it.

**CI's sitting, on the base kanso#1433 left.** The work vein reads runbench
1,904,577,350 -> 1,871,522,584 (-33,054,766, -1.7356%), 2,354,376 deeper than
the container's -30,700,390, and jsonbench 1,187,809,626 -> 1,185,398,676
(-2,410,950, -0.2030%) where the container had read +599,400. Four more
fall, and the two the container never measured fall furthest: livebench
-138,629,673 (-4.5934%), encodebench -110,038,475 (-3.0844%), oneshot
-362,607 (-1.9360%), scanbench -502,488 (-0.1003%). An escape scan's runs
between specials are a few bytes each, so nearly every one of them was a
tail and ended in the byte walk. The other eight hold to the instruction.
Against main two work rows still stand above it, neither moved here:
work_escapebench 82,999,058 -> 85,995,606 (+2,996,548), the accumulator's
lifetime priced in kanso#1432's entry, and work_digestbench 9,813,332 ->
9,838,994 (+25,662), kanso#1430's wrapper price less the falls since.

Machine code: seven rows RISE and seven hold, summed text 1,718,924 ->
1,746,636 (+27,712): runbench +7,088, oneshot +6,368, livebench +6,368,
jsonbench +5,984, scanbench +976, widebench +496, encodebench +432. Each
inlined scan site carries the masked tail load beside its loop, and the
below-floor scanners carry it twice. Against main the summed `text` vein is
a RISE, 1,707,852 -> 1,746,636 (+38,784), the tail and kanso#1433's inlining
priced together and bought with the falls above. The three compile rows fall
on the base by layout, -3,385, -9,386 and -9,562, and against main
`library_instructions` reads 144,836,225 -> 144,836,246 (+21), the layout
vein's noise.

## 2026-09-15 — the counters a shipped binary was still counting

kanso#1396 put the runtime's counter sites behind `K_COUNTING`, the macro
that is 1 in a counted build and 0 in the binary a program ships as, and
recorded twenty-seven of them leaving. Forty more never did. They were the
bare increments, `k_stat_beat_iters++` and its kind, with no guard at all:
the macro guards the dump that reads them, so in a shipped binary the
counters are written and never read, and the assumption was that a store
nobody reads is removed at link time. It is not, at least not here: the
per-instruction profile of kanso#1435's run program shows 6,425,360 counter
increments a run executing in the shipped binary, `k_stat_beat_iters`
2,692,767 of them, `k_stat_find2_calls` 2,072,753, `k_stat_append_fast`
385,040, `k_stat_append_rendered` 379,530, `k_stat_el_parses` 210,177,
`k_stat_ryu_renders` 191,070, `k_stat_append_grow` 176,697,
`k_stat_utf8_zerocopy` 175,617, and eleven smaller. Each is a read-modify-
write of a global, on paths the emitter inlines into every caller.

Every one of the forty now reads `if (K_COUNTING) k_stat_x++;`. A counted
build increments exactly as before, so every cost golden and every `.mem`
fixture holds to the byte, and the counters sweep agrees with all of them.

Four more sites were not increments and the sweep for `++` walked past them:
`k_stat_utf8_bytes += len` at both utf-8 validators, `k_stat_evac_bytes +=
n` at the evacuation, `k_stat_str_scan_bytes += s->len` at the character
scan, and the arena peak's `if (live > peak) peak = live` at every block. The
disassembly of the shipped run program still named all four; they carry the
same guard now and it names none. Measured alone on the forty-site build,
runbench -1,012,853 (-0.0546%) and jsonbench -1,426,835 (-0.1231%), output
byte-identical; the sweep agrees with every golden. A shipped build carries
no counter at all now, and the check that says so is `objdump -d runbench |
grep -c k_stat_`, which reads 0.

Container A/B, `env -i` under callgrind, equal-length names in one directory,
on the kanso#1435 leaves:

    runbench    1,867,578,425 -> 1,854,886,339   -12,692,086   -0.6796%
    jsonbench   1,172,409,354 -> 1,158,934,066   -13,475,288   -1.1494%

Twice the increments counted. The other half is what the increments cost
around them: a memory read-modify-write on a global in the middle of an
inlined fast path holds a register and orders the stores either side of it,
and the code around each site got shorter when it left. Output
byte-identical on both.

Row `counters_out`, mutation
`the_beat_iteration_counter_is_counted_in_a_shipped_binary` (the two
`k_beat_iter` sites unguarded again, 2,692,767 increments a run).

Three harnesses lift runtime text and compile it on their own: the float
parse spec, the bytes-capacity spec and the utf-8 differential. The lifted
text now names `K_COUNTING`, which only src/runtime.c defines, so the first
and third failed to compile on CI and the second lost its anchor line. The
float and utf-8 harnesses define `K_COUNTING 0` in front of the lifted text,
as the scan-tail harness already did, and the capacity spec cuts from the
guarded line. Seen red on CI at 727cb321 and d7b6acad; the two specs and the
differential pass here (45,189,025 checked, 0 mismatches).

The guard blinded the utf-8 differential's two ratchet rows on CI a round
later, at ed7c51b5, and the define was not why. The differential builds the
door by text and strips the counter line, `k_stat_utf8_bytes += len;`, to
nothing; with the guard in front of it the strip left a bare
`if (K_COUNTING)` standing over the next statement, the ascii check, so the
harness's door skipped straight to the validators on every input and
`k_all_ascii` was never reached. Both mutations live in `k_all_ascii`, so
the gate stayed green under each. The strip takes the whole guarded
statement now; both rows read red again here (`MISMATCH len=1 bytes=80` and
`len=12 ... e4 5d 13`), and the clean sweep passes.

**CI's sitting, on the base kanso#1435 left.** All fourteen work rows fall:
work_jsonbench 1,185,398,676 -> 1,169,790,503 (-15,608,173, -1.3167%),
work_runbench 1,871,522,584 -> 1,857,535,269 (-13,987,315, -0.7474%),
work_livebench -6,326,619 (-0.2197%), work_encodebench -5,013,110
(-0.1450%), work_escapebench -1,215,014 (-1.4129%), work_deepbench -965,492
(-0.2716%), and the other eight by between 232 and 128,315. The container
had read runbench -13,704,939 and jsonbench -14,902,123 for the two commits
together; the runner reads both a little deeper. Against main two work rows
still stand above it, both falling here: work_escapebench 82,999,058 ->
84,780,592 (+1,781,534), the accumulator's lifetime priced in kanso#1432's
entry, and work_digestbench 9,813,332 -> 9,830,211 (+16,879), kanso#1430's
wrapper price less the falls since.

Machine code: all fourteen rows fall, summed `text` 1,746,636 -> 1,737,068
(-9,568), oneshot and livebench -1,824 each, runbench -1,808, jsonbench
-1,728; a guarded site compiles to nothing and the code around it shortens.
Against main the summed text vein is still a RISE, 1,707,852 -> 1,737,068
(+29,216), kanso#1433's inlining and kanso#1435's masked tail less this. The
three compile rows rise by layout on the base, compile_instructions
42,869,800 -> 42,871,759 (+1,959), entry_instructions 144,035,324 ->
144,040,457 (+5,133), library_instructions 144,836,246 -> 144,840,900
(+4,654); against main +1,393, +4,508 and +4,675, the compiler's bytes moving
with src/runtime.c. compile_allocs holds at 27,937 and compile_memory is
byte-identical. Welfare 69.39 -> 69.44, banked.

## 2026-09-15 — a capture is a load, an own-err check is a tag test

Two runtime calls sat at the top of the run program's call-count table on
kanso#1436's leaves, above every emitted function but `encode_onto`:
`k_not_own_err` 1,454,508 times and `k_env_get` 1,279,648 times. Neither does
anything a call should be paid for. `k_env_get` is `((KValue*)env)[i]`, three
instructions behind a call and a return, and the emitter called it once per
captured name in every lambda's entry block. `k_not_own_err` is the gavel-24
check in front of an arm that admits err, whether the value is an err its own
package raised; its first line is `if (v.tag != K_ERR) return 1`, and nearly
every value an arm is tried against is not an err at all, so the package
compare behind that line ran 1,454,508 times for the handful of errs the
program ever raises.

Both stop being calls. A capture read is a `getelementptr` and a `load` in
the lambda's entry block. The own-err check is an alwaysinline twin,
`k_not_own_err_fast`, that tests the tag in the caller and calls the C
function only for an err; the two sites the emitter writes it at call the
twin.

Container A/B, `env -i` under callgrind, equal-length names in one directory,
on the kanso#1436 leaves, each half measured alone by applying the other
half's mutation:

    both                runbench  1,854,886,339 -> 1,823,814,374  -31,071,965  -1.6751%
                        jsonbench 1,158,934,066 -> 1,124,895,296  -34,038,770  -2.9371%
    the capture load    runbench  -7,751,327 (-0.4179%)   jsonbench -20
    the own-err twin    runbench -23,077,569 (-1.2442%)   jsonbench -34,038,750 (-2.9371%)

The two sum to within 243,069 of the pair. The own-err half is worth more
than its 1,454,508 calls at a few instructions each: the decoder's
`obj_key_start`, `scan` and `str_escape` each dispatch on a value that has
just been switched on by tag, and with the check inline LLVM folds the twin's
tag test into the switch it already made, so the call, the argument moves
and the spill around them all leave. Output byte-identical on both programs at
every step. The decoder has no lambda with a capture, which is why the
capture half reads twenty instructions on jsonbench.

The emitted vein moves in every program, in one direction. The prelude gains
one define, one branch and one call, the twin's body; every capture read
that was a call is a load, so a program's `calls` fall by its capture reads
less one: runbench 5,943 -> 5,895, scanbench 3,260 -> 3,234, deepbench 841
-> 827, the decoder 1,210 -> 1,207. Summed over the thirteen programs
beside the decoder, emitted_other_defines 2,350 -> 2,363 and
emitted_other_branches 12,663 -> 12,676 (one each per program, the twin),
emitted_other_lines 133,020 -> 133,303 (its nine lines per program, less
the call lines that became loads), and emitted_other_calls 20,330 ->
20,187; the decoder alone, emitted_defines 142 -> 143, emitted_branches
793 -> 794, emitted_lines 9,142 -> 9,155, emitted_calls 1,210 -> 1,207.
The compile-cost goldens move the same way, one define and one branch per
program and lines up by the twin: bench/compile_golden.txt's five programs
each gain a define, a branch and eleven lines (recursion `lines` 1,195 ->
1,206, dispatch 1,187 -> 1,198, guards 1,180 -> 1,191, records 1,236 ->
1,247, build_block 1,161 -> 1,172, summed `lines` 5,959 -> 6,014), and the
module row reads module_lines
5,317 -> 5,334, module_defines 101 -> 102, module_branches 438 -> 439,
module_calls 754 -> 748. The
runtime cost veins and the lazy tier are byte-identical: nothing here
allocates. The three host-keyed compile rows and machine code are CI's until
round two.

Rows `capture_load`, mutation `a_capture_is_read_through_a_call` (the call
put back through the same slot pointer; gated on the emitted vein, which
sees the calls return), and `own_err_inline`, mutation
`an_own_err_check_is_a_call_on_every_value` (the bare call at both sites;
gated on the work vein, since the emitted text counts the same one call).

**CI's sitting, on the base kanso#1436 left.** Twelve work rows fall and two
hold: work_jsonbench 1,169,790,503 -> 1,130,225,294 (-39,565,209, -3.3822%),
work_scanbench 500,324,263 -> 460,784,763 (-39,539,500, -7.9028%),
work_runbench 1,857,535,269 -> 1,823,669,249 (-33,866,020, -1.8232%),
work_deepbench -9,376,000 (-2.6448%), work_widebench -448,019 (-1.3750%),
work_oneshot -263,777 (-1.4455%), and six more by between 8 and 305,191;
work_escapebench and work_indexbench hold, having no capture read and no
own-err check on their paths. The container had read runbench -31,071,965
and jsonbench -34,038,770; the runner reads both deeper, and scanbench,
which the container never measured, falls furthest: its scanners dispatch
on every byte class and each dispatch asked the own-err question through a
call. Against main two work rows still stand above it, neither moved here:
work_escapebench 82,999,058 -> 84,780,592 (+1,781,534), the accumulator's
lifetime priced in kanso#1432's entry, and work_digestbench 9,813,332 ->
9,830,203 (+16,871), kanso#1430's wrapper price less the falls since.

Machine code: twelve rows fall and two hold, summed `text` 1,737,068 ->
1,735,116 (-1,952), runbench -560, scanbench -256, widebench -224; a load is
shorter than the call it replaces and the inline tag test folds into the
dispatcher's switch. Against main the summed text vein is still a RISE,
1,707,852 -> 1,735,116 (+27,264), kanso#1433's inlining and kanso#1435's
masked tail less kanso#1436 and this. The three compile rows rise on the
base, a codegen change writing one more define per program and a load per
capture read: compile_instructions 42,871,759 -> 42,873,153 (+1,394),
entry_instructions 144,040,457 -> 144,046,325 (+5,868), library_instructions
144,840,900 -> 144,845,876 (+4,976); against main +2,787, +10,376 and
+9,651. compile_allocs holds at 27,937 and compile_memory is byte-identical.
Welfare 69.44 -> 69.58, banked.

## 2026-09-15 — the per-call floors, mapped after the inlines

Where the run program's instructions go on the kanso#1437 leaves, read off
the per-instruction profile and bucketed by how often each instruction
runs, so a per-call cost separates from a per-byte one. Every figure is a
count of instructions the shipped binary executes.

`d_json/encode_onto` is 382,082,442 of 1,823,814,374 (20.95%), 2,380,950
calls. Twenty-six instructions run on every call: fifteen of frame (six
pushes, the stack adjust, six pops, the return) and the tag switch. Fifty-five
run once per string (942,750): the in-place quote append, the thirty-two-byte
bytes view `escape_onto` builds for its scan, and the scan's setup. Sixty-six
run once per map pair past the first (504,000) and thirty-eight once per
list element past the first (628,200), each with a `k_beat_iter` beside it,
because the element loop allocates the view and is a beat. A hundred and
twenty run once per map (248,490), `k_b_entries` and the empty check. None of
these buckets holds a loop the code walks a byte at a time; each is a stack
of ten-instruction steps the library's shape asks for. The one bucket with a
removable part is the view: seventeen instructions and thirty-two arena
bytes per string, 16,026,750 a run (0.88%), and removing it needs the escape
scan to read a string's bytes without a view, which is a byte-position
primitive on strings that the library does not have. That is surface, so it
is written down here and not built. The frame was priced by kanso#1338
(outlining the arm that sizes it, +2.5582%) and is not retried.

The decoder's `obj_key_start` is 236,081,850 of the decode program's
1,124,895,296 (20.99%), 1,254,150 keys, and 158 of its instructions run on
every key: the frame, the quote test, the `find2` scan's setup and one
sixteen-byte step, the byte at the close quote, the `k_b_utf8_slice_raw` call
and its result checks, the colon, the `parse_value` call and its checks, the
map's in-place insert. Ten steps, none over twenty instructions, four of
them re-testing the input's bytes tag and four re-testing a result for
failure across block edges LLVM did not fold. `str_escape` runs 81 to 86 per
escape, in four copies, one per escape arm: the in-place append, the `find2`
to the next special, the fused slice-append of the clean run and the dispatch
on the byte found. `k_b_utf8_slice_raw` is 49 per call on a short ascii key:
the bounds clamp, two overlapping four-byte loads for the high-bit test, a
thirty-two-byte string and two overlapping stores. `k_b_to_float` is 104 per
float plus fourteen per digit, and the 104 are the Clinger exact path, one
`divsd` against a power of ten. `k_beat_iter` is 23 per iteration, 2,685,021
a run; two of the 23 are the call and return, and a settled top-of-stack
mark was declined at +0.3210% (kanso#1293).

So the two programs are at the floor their emitted shape sets: per-call
frames, per-step tag tests, and library steps of ten instructions each. The
next run-speed win of a per cent or more is a library or emitter shape, not a
runtime kernel, and the bytes-free escape scan above is the one with a
number on it.

Measured, not built: a scratch builtin `text/find2_below_str` that scans a
string's own bytes for the two specials below a floor and answers 0 for a
miss, with `escape_onto` building the view only when it hits. Container A/B
on the same leaves, output byte-identical on both programs: runbench
1,823,814,374 -> 1,801,576,724 (-22,237,650, -1.2193%), the decoder
unmoved. More than the view's seventeen instructions predicted, because the
element loop's beat and the view's arena bytes go with it. It is surface, a
byte-position scan on a string in a library whose string positions are
codepoints, so it goes to Clay with this number and is not built here. The
patch is in the session's scratchpad as escape_str.patch.
## 2026-09-15 — gavel: the box is explicit, an err is a value, and a bare err halts where it lands

Clay ruled the ledger's one Blocking entry, "Where the box wraps under the
pure-fallibility rider: at every err-carrying answer, or at the `!` name",
and ruled it by retiring the question. His words, after the chat put the
three parts below to him: "I think you have your gavel a wise one."

**What he said, in his own framing.** "none is just an ordinary value. it has
nothing to do with effects. the idea is that when IO gives you a value it is
wrapped in an effect but we've never discussed how you could already manually
box a value yourself if you wanted to. I suppose you might as well be able to
do that I guess why not. but in that case it seems like you would also want
some kind of box to contain the errors to present them as an effect as well.
we already have the combinators to unwrap them to get access to the
underlying value or error object."

**The ruling, in three parts.**

1. **The box is only ever explicit.** IO applies it for you. Anyone else
   applies it by hand, holding a value or holding an err, and the three words
   are the only way back out. There is no lifting at a declaration boundary:
   a function that passes a fallible answer through is boxed only if it boxed.
   The spelling of the hand-applied constructor is not ruled here; it is the
   ledger's new Open entry "The box constructor's spelling", with a
   recommendation, and cloud builds against that recommendation unless Clay
   objects.
2. **A bare err is data.** An `(err _)` arm matches it anywhere, the way an
   arm matches `none` or a marker like `file_not_found`, and exhaustiveness
   makes sure the arm is there. A pure function that wants to signal failure
   hands back a bare err and its caller dispatches on it. Boxing the err is
   what takes that option away from the caller and turns it into a failure
   only a foreign `rescue` can end. The distinction the errors page draws
   between "a valid pseudo-error" and "a true exception" is spelled by
   whether the err is bare or boxed, and one object serves both.
3. **A bare err arriving where a value is wanted is refused at check.** An
   operator, an index, or a call with no `(err _)` arm at that position does
   not compile, exactly as it does not compile today when a `none` can reach
   it. Clay, correcting the chat's first draft of this part, which said the
   program halts there at runtime: "well no that would just fail to compile
   obviously." The railway retires; nothing outside a box propagates.

**What it supersedes.** The 2026-08-31 rider "pure fallibility is boxed too",
read literally, lifted every err-carrying answer into the box at the
declaration boundary. That reading is retired. The 2026-08-29 gavel stands
whole: effects are types, the words are the only doors, the box is opaque to
dispatch, the foreign-only rescue license, and `!` at the call site as the
choice of channel.

**What it dissolves.** The ledger entry measured 738 of lib's 770
declarations becoming boxes under the literal rider. Under an explicit box
that number does not exist: a helper is boxed only where somebody wrote the
box.

**What it does not dissolve.** The in-range read. With a bare err refused at
check the way a `none` is, `xs[i]!` cannot be free in a kernel by halting on
a miss; whether the checker sees an err in its answer set, or a box, or
neither, is what the ledger's option 4 was asking and is still asking. It
goes back to the ledger as its own Blocking entry, "What `!` promises the
checker", with the two readings and a recommendation. The chat's first draft
of this entry said the halt dissolved it; it did not.

**What is left.** Cloud builds the constructor and retires the railway;
ch04's "nothing is asked of the signature" paragraph, the last piece of the
book ruling, is released by this and moves with the build. The ledger
carries the constructor's spelling as an Open entry so the build does not
wait on it, and the `!` question as a Blocking one, because the 710 sites
that hand `xs[i]!` to an operator today wait on its answer.

The entry left `design/pending-gavels.md` in this commit, four days after it
was filed. STATUS.md's "Ruled, unbuilt" row for the rider is replaced by a
row for this ruling.

## 2026-09-15 — gavel: the maps parse is external state, and the compile row is normalized so it is not counted

The ledger's "The maps parse is 100% of the compile row's binary-to-binary
drift", open since 2026-09-08, recommended that the 2026-09-03 NO EXCLUSION
ruling stand. Clay first asked why it was a question at all — "if the binary
changes in a way that makes it more costly to run it is more costly to run,
that is just an empirical fact right?" — and the chat recorded it as standing.
Then the chat explained what the term is: `pthread_getattr_np`, called from
`std::rt::lang_start_internal` to place the stack guard, parses
`/proc/self/maps` with `getline` and `sscanf`, and its cost follows the
number of lines in that file, which follows the binary's section layout. His
ruling, verbatim: "well then this has nothing to do with compiler performance
and obviously shouldn't be part of what we measure. as I've said to you
voluminously in the past you want to set up the run so that any external
State like this is normalized. you clear it out so it's identical every
single run or you do something that puts it into a persistent known initial
state."

**The ruling.** The compile row measures the compiler's work. A term whose
size follows the binary's layout rather than the code under test is external
state, and external state is normalized before it is counted, never counted
and explained. The 2026-09-03 NO EXCLUSION is superseded on this term. This
entry replaces one written an hour earlier in the same pull request that
recorded the opposite; it never reached main.

**What the fact is, for the build.** The parse is deterministic per binary:
the same binary parses the same file every run. What differs is the layout
of two binaries, so a 64 KiB `.bss` probe that adds no code moved the row
2,130 instructions with the compiler's own work identical to the instruction
(the archive's "the mechanism, named and accounted to the instruction").
Nothing can be cleared between runs. The normalization is one of two shapes,
and choosing is a build with a measurement in front of it: count from the
compiler's `main` rather than from process entry, so Rust's runtime startup
and the parse fall outside the row; or pin the layout so the file has a
known line count. kanso#1234 carried a toggle of the first shape and dropped
it under the 2026-09-03 ruling; it is the obvious starting point.

**The general rule, recorded so it stops being re-argued.** Before a
measurement is taken, every piece of state the code under test did not
produce is put into a known state — cleared, or fixed — so that two runs of
the same code read the same number and two runs of different code differ by
what the code did. Clay says he has said this "voluminously". It is in
CLAUDE.md now, beside the platform-invariance rule for counters.

The entry leaves the ledger in this commit, ruled. The build joins STATUS.md's
"Ruled, unbuilt" list.

## 2026-09-15 — the maps parse is outside all three compile rows, measured, and a spec holds it there

The 2026-09-15 gavel "the maps parse is external state, and the compile row
is normalized so it is not counted" left a build: count from the compiler's
`main` or pin the layout, chosen by measurement, and re-sit the goldens. The
first shape has been on main since kanso#1241 and is what every compile
golden has been sat on since. This entry is the measurement the gavel asked
for, taken against the gates as they stand, and the spec that keeps them
there. No golden moves.

**What the gates read.** `scripts/gates/compile_instructions.sh`,
`entry_instructions.sh` and `library_instructions.sh` each take the first
`kanso::main` line of `callgrind_annotate --inclusive=yes`, and `src/main.rs`
carries `#[inline(never)]` on `main` so the frame survives. The ledger entry
that became the gavel described the row as the whole process. It was filed
on 2026-09-08 against a reading kanso#1241 had retired four days earlier: its
`row` column is callgrind's summary line, which no gate has read since.

**Where the parse sits.** On this container's build, `pthread_getattr_np` is
called once, by `std::rt::lang_start_internal`, and nothing under
`kanso::main` reaches it. The call graph:

    112,592  < std::rt::lang_start_internal (1x)
    112,592  * pthread_getattr_np@@GLIBC_2.32
     98,386      __isoc23_sscanf (50x)
      9,049      getline (50x)
      2,312      fopen (1x)

Fifty lines of `/proc/self/maps`, parsed once, above the anchor.

**The ledger's table, re-read under the anchor.** Same probes as the archive's
"the mechanism, named and accounted to the instruction", rebuilt on main
689ff885 and read with `scripts/compile_row_probe.sh`: `row` is the whole
process, `maps` is `pthread_getattr_np` inclusive, `program` is what the
three gates read.

    variant           .text     .bss    row         maps     program
    baseline          2743798   312     43,942,132  112,592  43,472,369
    +64 KiB .bss      2743798   65848   43,944,260  114,720  43,472,369
    +1,600 B .bss     2743798   1912    43,942,132  112,592  43,472,369
    100 dead fns      2745398   312     43,947,012  110,323  43,478,218
    400 dead fns      2754550   312     43,953,573  112,604  43,478,598

The `.bss` row is the ledger's case. The whole-process count moves 2,128, the
parse moves 2,128, and the gates' row holds to the instruction: the parse is
not in it. The 1,600-byte probe says why the ledger's probe moved the parse
at all — the file gains a line when the RW segment grows past its file
mapping, which 64 KiB does and 1,600 bytes does not. Under valgrind the brk
base is fixed at the same page for every binary (read directly: `sbrk(0)` is
`0x403a000` with a .bss of 8, 1,632 and 65,568 bytes), so the heap's layout
is already normalized and no probe reached it.

**The one term still inside the anchor, named.** The dead-function rows move
`program` by 5,849 and 6,229, and the per-function diff of the two profiles
against the baseline puts all of it in one place: `__memcmp_avx2_movbe`
+5,868 and +6,227, with every kanso symbol identical to the instruction and
the rest of the movement (`_dl_relocate_object`, the maps parse) above the
anchor. The mechanism is the binary's own layout: the hundred functions add
relocations ahead of `.rodata`, which moves `.rodata` from 0x32d00 to
0x33680, so every string literal's offset within its page changes and glibc's
memcmp takes its page-crossing arm on a different set of them. Measured 2026-
09-04 at 402 on a 7,632-byte `.text` probe; 5,849 here on 1,600 bytes, and it
is not monotone in either, which is what an alignment term looks like.

**The second shape, priced.** Pinning `.rodata` to a fixed page removes that
term for anything that grows ahead of it. Built with
`-C link-arg=-Wl,--section-start=.rodata=0x100000` on the same two sources:
`program` reads 43,471,592 on the baseline and 43,471,592 on the hundred-
function probe, identical to the instruction where the unpinned pair
differed by 5,849. The price is the gap the linker writes: the binary grows
from 4,677,120 to 5,724,880 bytes at that address, and a pin one page above
today's `.rodata` (0x40000) would cost about 52 KiB and fail the link, loudly,
the day the sections ahead of it outgrow it. What the pin cannot reach is
growth inside `.rodata` itself — `src/runtime.c` and `lib/*.kso` are
`include_str!`'d into it, so a runtime or library edit shifts every literal
after them whatever the section's start — and those are the edits that have
moved the compile rows "by layout" on most of this month's pull requests. So
the pin buys the code-only case at a shipped-binary cost and leaves the
common case alone. It is not built here. Whether a 1% larger binary, or a
measurement build linked differently from the shipped one, is worth the
code-only case is Clay's, and goes to him with these numbers rather than to
the ledger.

**The spec.** `tests/the_compile_rows_start_at_the_compilers_main.rs` reads
the three gates for the `kanso::main` anchor, refuses any gate reading
callgrind's `summary:` line, and reads `#[inline(never)]` off `fn main`.
Watched red first: the ratchet mutation
`a_compile_row_that_counts_from_process_entry.sh` replaces one gate's
anchored read with the summary line, and two of the three tests fail, at the
anchor check and at the summary check. Row `compile_ir_from_main`. The
mutation guards on a gate this branch does not change, so the touched pass
does not select it on this pull request; it was proved by hand here and the
nightly proves it with the table.

**What this leaves.** The ruled normalization is in place and pinned; the
goldens need no re-sit because they were never sat on the other reading.
STATUS.md's row comes off when this lands, which is the chat's. The
`.rodata` pin is a measured option, recorded above, and not a question the
ledger needs.

## 2026-09-15 — the box built by hand: `effect v` on all three engines

The first slice of the 2026-09-15 gavel "the box is explicit, an err is a
value, and a bare err halts where it lands", part 1: the constructor. The
word is the ledger's Open recommendation, `effect`, the type's own name in
prefix position, and the build does not wait on the entry.

**What it is.** `effect v` is an ambient one-argument builtin that answers a
description settling to `v` when it runs. It is value-shaped in its argument
and box-shaped in its answer, so `effect 5 .> f` hands `f` a 5, and
`effect (err "bad")` is a box whose content is the failure: `.?` sees it,
`.>` skips it, `.!` annotates it, the same three doors a failure that a read
raised has. A box answered by `effect` is refused wherever a value is wanted,
the way a box a library function answers already was: `effect 5 + 1`,
`length (effect [1 2])` and a call of a function whose tail is the
constructor all read the effect diagnostic at check.

**Where it lives.** One arm in each engine and one in the checker. The
interpreter's `Desc` gains a `Settled(Value)` variant, executed by handing
the value back and rendered `settled` in `--plan`; the page runs the same
interpreter through the generic builtin bridge, so it needs nothing of its
own. Native gains `k_settled`, description tag 31, with `k_exec` answering
`d->x` for it, and the emitter calls `k_b_effect` like any other builtin. The
checker's box question, `yields_box`, answers yes for an application of the
unshadowed name before its short circuit, because a hand-built box is the one
case where a program with no boxed declaration still holds a box; and `infer`
gives the builtin the description bit and nothing else, since the argument's
failure bits are held as content rather than carried.

**Watched red.** The micro fixture `the_box_built_by_hand` starts a chain
from a value, from a failure through `.?`, from a failure through `.!` then
`.?`, holds a box through a function and an interpolation, and skips a `.>`
on a boxed failure; native and the interpreter agree byte for byte. The ratchet
row `hand_box` mutates the interpreter to hand the value over unboxed, and the
micro corpus goes red. The errors fixture
`a_hand_built_box_where_a_value_is_expected` pins the three refusals on both
the direct and the imported path.

**What it costs.** No runtime vein moves: no benchmark program builds a box by
hand. The text vein and the three compile rows move as they do for any edit
to runtime.c and the emitter, and CI's sitting is written in the next round.

**What is left of the gavel.** Part 2, a bare err matched by an `(err _)` arm
anywhere, retiring the own-origin skip; and part 3, a bare err refused at an
operator at check, on the sites that are not `!`. The `!` sites wait on the
ledger's Blocking entry "What `!` promises the checker".
## 2026-09-15 — a chain callback is handed the err itself, and native's guard on the group behind it was left out

Found while mapping the arm-dispatch sites for part 2 of the 2026-09-15
explicit-box gavel. A probe program with a rescue callback that hands the
err on to a bare-binder group printed `took mine` on native and died at the
endpoint on the interpreter with `passed through lam`. The interpreter is
the oracle, and it is right: a bare binder refuses every failure, and the
callback's parameter holds the err `rescue` handed it.

**Why native differed.** A group's entry guard is emitted only when
inference says the parameter can be a failure, and the parameter's set is
the union of what every call site hands it. The one call site here hands
the callback lambda's parameter, and inference seeds every lambda parameter
as never failing, because an ordinary call refuses to hand a closure a
failure. That seed is wrong for exactly two callers: `rescue` and
`annotate` hand their callback the failure on purpose. So the group's
parameter proved never-failing, the guard was left out, and the err walked
into the body. A direct call `lam (mine 1)` guards, because the argument's
set carries the failure, which is why no fixture had caught the shape.

**The fix.** Inference walks a lambda written as a rescue or annotate
callback with its parameter seeded to everything, err included. The group's
parameter then carries the failure bit and the emitter writes the guard.
Nothing changes for any other lambda.

**Watched red.** Micro fixture `a_chain_callback_is_handed_the_err_itself`:
two chains, one through `.?` and one through `.!`, each handing the err to a
bare-binder group, each read by std's `when_failed` so the fixture prints
what came out rather than dying. Native read `the callback's group took the
failure` and `passed lam, reason took mine` against the interpreter's
`passed lam, reason mine` twice; both engines print the interpreter's lines
now. Ratchet row `callback_err`, mutation
`a_chain_callbacks_parameter_never_fails`, puts the old seed back and the
micro corpus goes red on that fixture.

**What it costs.** An inference change moves the three compile rows and
possibly the compile allocations; CI's sitting is written in the next round.
The wasm engine runs the interpreter and needed nothing.
## 2026-09-15 — a bare err is data: the arm skip retires, and the rescue licence moves to the word

Part 2 of the 2026-09-15 ruling "the box is explicit, an err is a value, and
a bare err halts where it lands". Part 1, the `effect` constructor, is
kanso#1440. This entry builds the ruling's second sentence: "an `(err _)` arm
matches it anywhere, the way an arm matches `none` or a marker". Part 3, a
bare err refused at check where a value is wanted, is next and waits on
nothing.

### What retires

Since 2026-08-24 an err skipped every arm its own hako wrote. Three engines
carried it: the interpreter asked `own_failure` inside `match_one` with the
arm's package threaded through `match_params`, native emitted a
`k_not_own_err` call in front of every err-admitting pattern (an
alwaysinline twin since kanso#1437, the same morning), and wasm emitted an
`rt_not_own_err` guard. All three are gone. `(err r)`, an `:err` annotation
and a typeset with err among its members take a failure whoever raised it.

The static half went with it. `kanso check` ran a second fixpoint after
inference, provenance.rs, carrying per group the packages whose errs could
arrive at each parameter, to refuse an arm written for its own package's err
as dead code (`error[license]`). That arm is live now and there is nothing
to refuse, so the pass is deleted and provenance.rs keeps `package_of`
alone. The four advisory fixtures that only fed the refusal go with it, and
the licence entry leaves tests/golden/unpinned_diagnostics.txt.

### What stands, and where it moved to

The 2026-08-29 gavel's foreign-only rescue licence stands, and the ruling
says so in as many words. It is asked at the WORD now. `rescue` carries the
site it was written at, exactly as `annotate` already did: `Desc::Rescue`
gains a `Raised`, native's `k_b_rescue` takes the origin literal and builds
the same closure `k_b_annotate` builds (`k_rescue_wrap` beside
`k_annotate_wrap`, both through `k_sited_word`), wasm's `rt_rescue` takes
the origin literal and `Slot::Rescue` carries it. A `.?` written in the
package that raised the failure hands it on without entering its callback.
A failure with no package — merged out of several, or raised by a host with
no frame — passes the test, as the arm form always let it.

`annotate` keeps entering its callback on an own failure. It re-raises under
the site's own name, which is the shape a package uses to say what it was
doing when a failure reached it, and nothing in the ruling touches it.

### Two fixtures the licence caught, both this module rescuing itself

`a_chain_step_names_its_channel` ended a chain with `.!` and then `.?` in
one file, so the rescue was reading the failure the annotate had just
raised as this module's own. Under the licence that failure goes to the
endpoint. The fixture reads it through `std/testing` now, which is foreign
to it, and the chain ends at the annotate.

`an_err_has_readers` rescued `boom 1` in the file that raised it, eight
times. Every line is an `:err` arm now, which is the ruling's point: the
readers `.reason`, `.cause` and `.origin` are applied inside an arm that
took this module's own failure, and the one foreign failure (json's) is
still read through `rescue`. The fixture used to say "a named group handed
the err would pass it through"; that sentence was the rule this entry
retires.

### The pins

- `an_arm_sees_its_own_hakos_err` and `a_typeset_arm_sees_its_own_hakos_err`
  replace their "cannot see" twins: `mine own` answers "rescued 99", and the
  foreign rescuer handed a string answers false.
- `which_patterns_can_hold_a_failure`: the own column reads the same as the
  foreign one, `took` for the three err-admitting forms and `past` for the
  seven others.
- `a_rescue_hands_on_its_own_packages_failure` (micro): a bare own failure
  under `.?` with a LAMBDA callback goes by ("still failed, mine"), a bare
  foreign one is claimed, a boxed foreign one is claimed. The lambda is the
  discriminating half: a lambda has no arms to decline with, so before the
  licence moved every engine printed "claimed mine".
- `a_rescue_on_its_own_boxed_failure_reaches_the_endpoint` (runtime): the
  boxed own shape, `time/sleep 0 .> (_ -> mine 1) .? (e -> …)`, ends at the
  endpoint on both engines with the same report.
- ch04's `boundary` sample loses one line: the report no longer says
  `passed through kitchen/apologise`, because the callback is never entered.
- Ratchet: the `own_err_inline` row and its mutation retire with the sites
  they patched; `rescue_licence` deletes the interpreter's licence arm, and
  the micro corpus prints the callback's answer where the golden says the
  failure went by.

### The prose

ch04's "no arm can catch it" is "an arm has to name it": a catch-all
declines an err, an `(err _)` arm claims one wherever it is written, and
`.?` is licensed on failures that reached you from elsewhere. The
teahouse/kitchen example keeps its two opposite answers and loses the hop.
ch08's json section no longer says an arm in the library could never match;
it says the library chooses not to write one. appb's `wrap_err` paragraph
and the two ch08 report samples stop citing the arm rule. compiler.html's
§08 entry for the arm rule is marked retired with the date, §22's second
table reads `took` in both columns, and §23's first decision records the
move. design/testing.md's collision section records the retirement.

### What moved, measured on this container

The compile side falls because a whole-program fixpoint is gone.
`bench/compile_memory_golden.txt`: front_end_rounds 62 -> 47 (-24.19%),
front_end_visits 22,437 -> 15,076 (-32.81%); compile_peak_bytes reads
776,055 here, the recorded figure. compile_allocs reads 26,883 against the
golden's 27,937 (-1,054, -3.77%), a host-keyed row CI re-sits. The three
compile instruction rows are host-keyed too; this container's compile sweep
refused all of them and they will fall on CI by the pass's whole cost.
`bench/compile_golden.txt`: every sample loses eleven lines, one call, one
branch and one define, the twin's; the modules row 5,334 -> 5,323 lines.

The emitted code falls the same way in every program: the decoder 143 ->
142 defines, 1,207 -> 1,204 calls, 794 -> 791 branches, 9,155 -> 9,134
lines, and the other thirteen each lose the twin and their own-err call
sites (runbench 5,895 -> 5,892 calls, widebench 1,810 -> 1,807). Read
against origin/main, which does not yet hold kanso#1437, the thirteen
programs' summed lines RISE, 133,020 -> 133,110: kanso#1437's inline
capture reads add more lines than the retired guard takes away, and this
branch is cut on top of it. Against kanso#1437's own tip every one of those
rows falls, and that is the comparison that stands once it lands.

The twelve runtime allocation veins and the lazy tier agree with their
goldens: the guard never allocated. The work vein, the text vein and machine
code are host-keyed and will move on CI — the row kanso#1437 banked that
morning counted 1,454,508 own-err checks a run on the run program, and
every one of them is gone rather than inlined. Round one is deliberately red
on those, and CI's sitting is what gets written.

Welfare cannot see any of this until CI's rows land, because the priced
compile rows are host-keyed; the floor is banked after they do.

## 2026-09-15 — a bare err where a value is wanted is refused at check

Part 3 of the 2026-09-15 ruling "the box is explicit, an err is a value, and
a bare err halts where it lands", built on kanso#1442's tree (part 2). The
ruling's sentence: "An operator, an index, or a call with no `(err _)` arm
at that position does not compile, exactly as it does not compile today when
a `none` can reach it."

### What the checker proves, and what it does not

The none rule (kanso#1369) reads infer's answer sets: a call whose group can
answer `none`, handed to a group with no `none` arm at that position, is
refused. This rule reads the same sets for an err, with one bit added.
`RAISED` sits above `TOP` in infer, and only three things set it: the `err`
call, an `e:err` annotation, and an `(err _)` arm's catch. `bind_pattern`
now binds an as-pattern's name to what the pattern caught, so
`fn taken e@(err _) = e` hands the err on as a raised err and a caller that
reads `taken x` is refused like the raise itself. A strict index's miss and
a division's zero answer `ERR` without the bit: what the checker should make
of those is the ledger's Blocking entry "What `!` promises the checker",
and this rule does not pre-empt it. A description is skipped whatever it
carries, since a boxed failure is not a bare one.

The first cut refused 191 sites. Reading them: 43 were `text/split s "\n"`
and the like, refused because `split _ ""` raises and the group's joined
answer carries the bit whatever the separator. So infer's call join reads a
group one arm at a time and skips an arm a literal argument cannot reach:
`arm_can_run` compares a string or int literal against a literal pattern, a
module constant bound to a string literal counts as that literal (a
declaration's name cannot be rebound), and an interpolated string with fixed
text cannot match a shorter pattern. The check makes the same test at the
call. That took the count to 127, and the rest were real: every site left
was a raised err handed to a group with no arm for it.

**What the checker reads is calls, not names.** `x = decode s` then `f x`
compiles: a bare local binding drops the failure bits (`bind_pattern`'s
"generics never bind failures", the rule the none check already lives
with), so what a name holds is not something this rule sees. The
`some_is_a_value_not_a_failure` fixture pins the runtime's answer for that
shape on purpose, and says so.

### What the tree had to say to compile

std/regexp raises in one place, a variable-width lookbehind, and every entry
point could hand that err to its walk. Eight entry points now have an arm:
`taken` and `named` directly, and six through a wrapper (`gathered`,
`anchored`, `located`, `replaced`, `divided`, `begun`), so the walk is not
asked at every position whether its program is an err. `in?`'s
wide-character arm walked `text/split set c` and now walks the set's
characters, because the checker cannot see that `c` is never empty.

std/json's decoder threads a parse failure through `finish`, `array_step`,
`obj_key`, `obj_value` and `str_low`; each has an `e@(err _)` arm, last where
the group's other arms are constructor patterns and first where one is a
bare name. Its tests dispatch on what they decoded before comparing
(`decodes?`, `same?`, `encodes_back?`). The three vendored decoders
(encodebench, widebench, kq's query) take the same arms, kq's own
`obj_colon` included; kq#106 lands them first, since kanso's CI runs kq's
suite against the compiler on the pull request.

Four benchmark programs (encodebench, livebench, widebench, runbench), hako
and fourteen scripts bind a raised answer to a name before handing it on, or
give the receiving group an arm: 13 arms in lib, 22 in bench and kq, and
the rest bindings. The three micro fixtures that handed `json/decode "[1, 2"`
straight to an arm-less group to pin "a failure reaches none of the arms"
are reshaped: two write the err as an arm, the way the none rule had them
write `none` as one, and the third arrives by name. Two runtime trails lose a
`passed through` line, because an arm that answers an err is not a hop.

### Watched red

The three error-corpus fixtures, `an_err_reaches_an_operator`,
`an_err_reaches_an_index` and `an_err_reaches_a_group_with_no_arm_for_it`,
compile with `raised_err_at` deleted and refuse with it present; the ratchet
row `raised_err` carries that mutation.

### What the sweep found under the arms

The first counter sweep read the decode's `sh_rec` at 253,968,000 against
a golden of 0, with `allocs` 4,390,215 -> 8,358,465, and the run program
5,730,654 -> 8,348,673. Every scanner answer in the decoder travels in two
registers, the position packed above the value's tag in one word and the
payload in the other, and a consumer whose arm destructures `(parsed p v)`
reads those words with no record built. The escape analysis boxes a slot
whenever any arm at it names the whole value, since `r@(parsed p v)` wants
the record. The five `e@(err _)` arms this entry asked of the decoder sit at
exactly those slots, and the rule read them as as-patterns like any other.

An err as-pattern needs no record. The dispatcher reads the two words back
as one value before it matches, and on the failure path that value is the
failure that arrived; the name binds to it. The rule exempts `err`, and the
decode's counters read the golden to the byte again. The mem fixture
`an_err_as_pattern_keeps_a_carried_slot_unboxed` pins it at `sh_rec=0`; with
the exemption reverted it reads 64,000, a record for each of its thousand
scans (ratchet row `err_as_pattern`).

The fixture's first draft found something older. Written as a loop whose
groups hand back a value rather than a record, it is a beat, and the
scanner's answer crosses the rewind through the carry. The carry stages
boxed values, so the two words were built into a record on the way in and
read back out of one on the way out, and neither conversion asked whether
the words were a failure. A failure's words became a record whose second
field was the failure, `k_rec` merged that into a failure, the unpack read
two fields off it, and the consumer got a value whose first word was not
the err tag. `step`'s record arm matched it, and `+` was handed a garbage
word: native printed `error[runtime]: `+` is not defined for these values`
where the interpreter printed `stopped: end of input`. The #1393 compiler
does the same with a plain `(err _)` arm, so this is main's, and the
ruling makes it reachable everywhere an err arm now stands. `k_parsed_box`
and `k_parsed_words` in the runtime ask first and hand a failure through as
its own two words; the emitter calls them in place of the inline build and
the inline field reads. `a_failure_crosses_a_beat_carry_in_two_words` pins
the answer on both engines (ratchet row `carry_failure`). Its `sh_rec` reads
64,000: the carry still boxes a two-word value it could stage as words,
which is a gap left open here, not a regression.

The check also skips getters, as the none check has since kanso#1369 and
for the same reason: the play route checks before a field read is rewritten
into a getter call and the module route after, so `xs[i].x` would be
refused through an import and run direct. A field read of an err stays the
runtime's sentence on every route, and `accessor_hop_is_silent` keeps its
trail.

The harness found a third. Part 3 refuses `either? here (names_any? src
needles (at + 1))` in the browser differential's `hunted?`, since the
nested call can raise, and the entry's shape for that is to bind first:
`rest = names_any? ...` then `either? here rest`. The demand analysis
defers `rest`, because `either?`'s first arm never asks for it, and
`either? _ answer` hands the cell back unforced. That cell evaluates to
the next call's `rest`, which is another cell. `k_force_slow` stored what
a cell evaluated to and `k_force` tested the tag once, so the dispatcher
for `put_unless acc path src skip` was handed a cell, tested it against
`true` and `false`, and matched nothing; the harness died on every corpus
directory, and the pinned compiler 39442a53 does the same on the reduced
program, so this is main's too. The interpreter's force has always walked
the chain. `k_force_slow` forces what the cell answered before it stores
it, and the page's `forced` does the same before the write-back.
`a_deferred_answer_that_defers_again_forces_to_a_value` pins it on all
three engines (ratchet row `force_chain`).

### What moved, and which way

Every runtime allocation vein and the lazy tier agree with their goldens:
the arms cost the decoder nothing once the as-pattern rule admits them,
and the run program's counters are the ones kanso#1437 left. The emitted
code moves with the arms and the two runtime calls the emitter now makes:
the decoder's `emitted_lines` 9,134 -> 9,161 and `emitted_branches` 791 ->
795, `emitted_calls` 1,204 -> 1,209, defines 142 -> 141; over the other
thirteen programs `emitted_other_lines` 133,110 -> 133,514,
`emitted_other_branches` 12,653 -> 12,689 and `emitted_other_calls`
20,164 -> 20,231, defines 2,350 held. Four programs fall (basket,
pendbench, indexbench, readbench: a getter the check no longer walks emits
less), the rest rise by the arms std/regexp and std/json gained. The
front end's visits on the compile corpus read 15,076 -> 15,119 for the
same five decoder arms; rounds hold at 47. The three host-keyed compile
rows, the machine-code vein and the compile allocations are CI's to
measure, and welfare on this box reads 69.58 against a floor of 69.58
with the runtime side unmoved. A fall on CI
from the compile rows is the language's to pay under the 2026-09-13
clause and the floor moves with it in the second round.

### Left open, on purpose

- The runtime railway stays. "Halts where it lands" is the ruling's title
  and not one of its three built parts; it reads through the `!` entry, and
  retiring the railway before that entry is ruled would decide the entry.
- The containment idiom. `length (text/split hay needle) > 1` is how five
  scripts ask whether a string holds another, and each is a raise the
  checker sees. A `text/contains?` would be surface, so it is not added
  here; the scripts bind instead.
- Division's `ERR` and the strict index's miss, as above.

### CI's sitting, written in the second round (kanso#1444)

The stack of four landed as one pull request and CI sat it once, on the
kanso#1443 base. The compile term falls as the provenance fixpoint retires:
`compile_instructions` 42,873,153 -> 40,273,027, `entry_instructions`
144,046,325 -> 140,614,828, `library_instructions` 144,845,876 ->
140,855,393, `compile_allocs` 27,937 -> 27,173. The front end's peak rises,
`compile_peak_bytes` 776,055 -> 781,895 (+5,840, +0.7525%): the RAISED bit
rides in every inference set, the callback guard's seed table and the
provenance walk are held for the whole of inference, and the effect type is
one more declaration the front end holds. The run program pays for the
arms it gained and the two runtime calls the emitter makes at a rescue
site: `work_runbench` 1,823,669,249 -> 1,827,443,530 (+3,774,281,
+0.2070%), `work_jsonbench` 1,130,225,294 -> 1,133,644,520 (+0.3025%,
the decoder's five `(err _)` arms), `work_encodebench` 3,452,224,040 ->
3,452,269,515, `work_livebench` 2,872,789,146 -> 2,872,813,523,
`work_oneshot` 17,983,841 -> 18,006,613, `work_digestbench` 9,830,203 ->
9,830,546; five rows fall (basket, pendbench, readbench, scanbench,
widebench). Machine code rises with the arms, `text` 1,735,116 ->
1,737,148 summed over the fourteen. Welfare on CI's rows reads 69.58 ->
69.64 and is banked. The standalone scanbench takes the split's respell so
the shapes spec finds the pair identical, and two fixtures the ruling
refused — the register-convention failure and the module-boundary
reencode — bind the raised answer before handing it on, the shape every
other reshaped fixture took.

## 2026-09-16 — gavel: `!` is the value on the programmer's word, and a miss halts at runtime

Clay ruled the ledger's "What `!` promises the checker", filed 2026-09-15 the
hour the box-wrapping question was ruled, by taking its recommendation: "oh
yeah your recommendation is right i think."

**The ruling.** `!` tells the checker to drop the miss from the answer set.
`xs[i]!` reads as data: the element's type, no `none`, no err, no box. The
710 sites in the tree that hand `xs[i]!` straight to an operator, a group or
a field stand as written. A miss at runtime is an err value reaching an
operator, and it halts with the report the way `+` on a string halts today.
`!` is the recorded decision "I have checked this", and it is the one place
the checker takes a promise instead of a proof.

**What it closes.** The in-range read. sha256's compress and regexp's
scanner keep `s[5] + s[6]` at 9 allocations and 21 ms on two million
elements; the 8,000,014-allocation bind shape the retired entry priced is
never written. The literal rider's reading — `xs[i]!` as the manual box
applied at the read, every one of the 710 sites owing a `.>` — is declined,
and the bound-discharging checker that would have made it free is not on
the table.

**What it settles about the name.** `!` means the same thing at the index
and at the name: a promise the checker takes and the runtime enforces. A `!`
declaration still answers a failure, per the 2026-09-03 suffix contracts;
what it answers is a bare err, which its caller either matches with an
`(err _)` arm or hands on to something that halts.

**Together with the 2026-09-15 gavel.** The box is explicit and IO applies
it; a bare err is data; a bare err where a value is wanted is refused at
check unless `!` said otherwise, in which case the runtime enforces the
promise. That is the whole failure model, and nothing about it waits on a
ruling. The entry leaves the ledger in this commit. STATUS.md's row for the
explicit box drops the clause that said its `!` half was unruled.

## 2026-09-16 — a ruling from 2026-08-24 was never on the unbuilt list, and the sample it condemned still ships

Clay, reading ch03's knot sample: "this is still out of date ... you use _
not none." The archive's "gavel: a build hole is spelled `_`, and fills
exactly once" (2026-08-24) records him saying the same thing then — "build
doesn't work this way, as i said many times" — and records, in its own text,
that the shipping ch03 sample does the thing he rejected.

Read off main on 2026-09-16: `docs/book/samples/ch03/knot.kso` still reads
`ada = person "ada" none`, so does `tests/golden/micro/bare_field.kso`, and
the compiler refuses the ruled spelling — `person "ada" _` is `error[syntax]:
unexpected trailing tokens`. Twenty-three days, ruled and recorded and
unbuilt, and Clay found it by reading the book.

**Why the list did not catch it.** STATUS.md's "Ruled, unbuilt" was seeded on
2026-09-09 by probing the 2026-08-29 sitting, and its preamble said so: "the
list is a floor, since the rest of the 2026-08-29 sitting was not audited."
This ruling is from 2026-08-24, five days before the sitting the audit
started at. The floor was honest about its edge and nobody walked past the
edge. The row is on the list now, with the probe.

**The other row moved the other way.** The compile-row normalization was
ruled on 2026-09-15 at about 20:40Z and kanso#1439 landed it before 21:30Z
the same night — the maps parse outside all three compile rows, measured,
with a spec holding it there. Its row came off in this commit. A ruling
built in under an hour beside one unbuilt for twenty-three days is the
difference between a row on the list and a row off it.

## 2026-09-16 — gavel, reversed the same day: `!` answers a box, at the index and at the name

The morning's entry "`!` is the value on the programmer's word, and a miss
halts at runtime" is superseded. Clay ruled it on the chat's recommendation
and the chat's recommendation was a cost argument dressed as semantics. His
argument, verbatim: "the entire point of distinguishing the bang from the
non-bang form is to say whether this thing has an exception which bubbles up
or not. like none is something that is reached by an arm so it just uses
polymorphic dispatch. so if the return you're going to give needs to be able
to bubble up isn't that inherently an effect type?" It is. Ruled: "yes of
course you have to write it and get it up so whatever work the other cloud
thread is working on can minimize its waste."

**The ruling.** The bang is the choice of channel. A non-bang form answers a
value — `none`, a marker, a bare err — and an arm handles it where it lands,
by dispatch. A bang form answers something that bubbles: it cannot be
handled where it lands, only by a foreign `rescue` or the endpoint, and
bubbling is what the box is. So `xs[i]!` answers `<t>effect`, and so does
every `!` name, pure or io. `!` applies the box the way IO applies it, and
that is the one meaning of `!`. `rescue (menu["dango"]!) handler` is a
correct program.

**What the morning's reading got wrong.** A value on the programmer's word
with a runtime halt on a miss is bubbling with the rescue machinery cut off.
It made a `!` failure the one failure in the language nobody can catch,
which is backwards, and it did so to keep 710 sites in the tree compiling as
written. The 09-15 gavel stands whole; only the `!` half of what followed it
is reversed.

**The cost is cloud's, and the design does not pay it.** The 8,000,014
allocations on two million elements were measured on today's `.>`, where
every bind allocates a box, a closure, a bind node and a rewrap. Nothing in
the ruling requires that. Two levers, each a build with a measurement in
front of it: a bind whose box is a pure index read and whose callback is
applied at once is inlinable to nothing; and a literal index into a list of
known length, which is what sha256's `s[5] + s[6]` is, is provable in range
by the checker, so those sites drop the bang rather than gain a `.>`. Where
the levers come in short, the floor moves under the ironclad rule, because
this is the specification.

**What cloud builds.** The 710 sites that hand `xs[i]!` to an operator, a
group or a field are respelled: `.>` where the read can miss, no bang where
the bound is provable. `!` names in lib answer a box. The explicit-box row in
STATUS.md carries this; nothing waits on a ruling. This entry exists so the
worker stops building the morning's reading the moment it next reads the
list.
## 2026-09-16 — the box at the index: `xs[i]!` answers an effect, and the operator sites are respelled

Builds the reversal ruled the same morning (the entry "gavel, reversed the
same day: `!` answers a box, at the index and at the name", kanso#1446).
`xs[i]!` answers `<t>effect`, a box holding the element or the missing-index
err; the words open it, and an operator, a field read or a builtin handed the
box is refused at check with the effect sentence. The plain read `xs[i]`
answers the element or a `none` for an arm to handle, unless the `if`s around
it prove the index in range, in which case the miss cannot happen and the read
is its element.

**The prover.** infer's `Fact` set gained the shapes the tree writes its
bounds in: `return x if i < 1 or length xs < i` and `if (i < 1 or length xs
< i) x (... xs[i] ...)` both prove `xs[i]`, through the desugared `and`,
`or` and `not` of a condition, a comparison against `length xs` on either
side, and a literal offset (`xs[at + 1]` under `length xs < at + 1`). A read
needs a lower bound as well as an upper one: `xs[length xs]` alone is not
proven, `length xs < 1` beside it is. A literal list has a length the
prover can read, so `[4 5 6][1]` is proven by arithmetic and `xs = [1 2 3]`
in a body gives `xs` a length the guards below can use. The proof is recorded
per span, and the checker's none rule reads it: an unproven plain read handed
to a group with no arm naming `none` is refused as before, and a proven one
goes through. Micro fixture `a_guard_proves_the_read` pins six shapes on both
engines; errors fixture `a_strict_index_where_a_value_is_expected` pins the
four refusals of the box (`+`, a field, `==`, `length`). Ratchet rows
`bang_box` (the `yields_box` arm for a strict index), `guard_bound` (the
guard's discharge) and `literal_bound` (the literal list's length), each
watched red.

**The respell.** 155 files, +1,654 / −729. 745 `]!` sites lost their bang;
208 guards were written where the read's bound was not already spelled
beside it; 41 `]!` sites stand, each one opened by `.>` or annotated by `.!`.
lib/list's fold and the bounded steps ask their length where the read is,
so the proof can see it — the hoisted `len` of kanso#1278 is gone from
`fold_flat`, and the read is proven instead. sha256, regexp, json, hako,
twenty scripts, the four bench copies of the decoder, the book's six samples
and eleven pages, and the corpus follow the same shapes.

**Lever one, measured.** A `.>` whose subject is a strict index and whose
callback is a lambda in tail position was already inlined by the fused-bind
path (the callback's body becomes the caller's tail); the probe `xs[n % 3 +
1]! .> (v -> go (n - 1) (acc + v))` reads 63,696,216 instructions for
200,000 iterations with the emitter's new path on and with it off, and the
two IRs are identical. The new path in `emit_call_full` is for the other
shape, a named callback: `f = &step n acc` then `xs[n % 3 + 1]! .> f` at
2,000 iterations reads 913,876 instructions and 12,004 allocations without
it and 558,651 and 4,004 with it (−38.87%, six allocations an iteration to
two): the settled box, the closure and the bind node are never built. What
it costs: the inlined call is not a tail call, because the ruling wants the
callback's answer settled when it is a value, so at 200,000 iterations the
probe overflows the 8 MiB stack where the executor's bind survived it; the
interpreter overflows at that depth on both, so the differential law is
kept, and a loop written through a named callback was never a tail loop on
the oracle. No benchmark holds a `!` site, so the lever has no ratchet row;
this paragraph is its record.

**Lever two is a proof, not a shape.** The literal-list bound changes what
the checker accepts and emits the same read; it costs nothing at runtime
and is pinned by `a_guard_proves_the_read` and the `literal_bound` row.

**What the sweep found, and a bisect that lied first.** The first counter
sweep read the encode vein at 49,875,132 allocations against 7,557,132 and
`beat_iters` at 401 against 5,032,401: the encode loops had lost their
beat. A bisect built the bench decoder's old shape under the old and the
new compiler and read both as agreeing with the golden — because it ran the
binaries from /tmp, where `bench/large.json` does not exist, and all three
"agreed" on a run that died at the file read. Run from the tree, the old
shape under the new compiler matched the golden exactly, so the compiler
was not the cause; the respelled shape was, and `KANSO_BEAT_REPORT` named
the accumulator as "may carry heap". Two gaps, both older than this branch:

- A name bound below a `return x if c` guard is a statement of the guard's
  `rest`. linear.rs's `is_unique_source` and beat.rs's `local_binds` read a
  body's top level only, so `opened = ...` under `encode_list`'s guard was
  a binding neither could find, the accumulator handed on through it read as
  an alias, and every group in the chain lost its in-place append and its
  beat. Both lookups read through guards now (`bound_in`, `binds_into`). Mem
  fixture `a_local_bound_under_a_guard_keeps_the_beat` pins `allocs` 16 and
  `beat_iters` 8,000; under the linearity mutation it reads 17,016 and 0
  (`sh_bytes` 24 -> 408,024), under the chain mutation 16 and 7,000. Rows
  `guard_bind_linear` and `guard_bind_chain`.
- The boundary rule's licence for a byte builder crossing a rewind asked
  only the first parameter. `builder_transient`'s `assemble cs p acc`
  carries its builder third; while its loop was a two-group cycle no
  self-call argument was checked, and the respell into a direct self-call
  put the argument in front of the rule, which read it as heap: `beat_iters`
  1,360 -> 40, `bytes_malloc` 40 -> 0, `held_peak_bytes` 80 -> 0. The rule
  asks the position under test as well as the first parameter, the fixture
  reads 1,360 / 40 / 80 again, and the mutation that puts the first-only
  reading back turns it red. Row `bytes_acc_position`.

**What moved, priced.** digest `thunk_forces` 8,256 -> 16,512 and the run
program's `thunk_forces` 16,025 -> 32,025: sha256's `compress` guards
`length rounds < at` beside `rounds[at]`, and `rounds` is a deferred
constant, so each round forces it twice where it forced it once; every
force after the first is a memoised tag test. scan `allocs` 3,011,150 ->
3,011,149, `sh_rec` 1,552 -> 1,616, `sh_buf` 81,024 -> 80,976, and the run
program's `allocs` 5,730,654 -> 5,730,653, `sh_rec` 48,174,560 ->
48,174,624, `sh_buf` 111,160,896 -> 111,160,848: regexp's `spans` answers a
`bounds` record where it answered a two-element list, one allocation and 16
bytes fewer per quantifier parsed. The trend gate's keys for the two record
rows are `scan_sh_rec` at 1,616 and `run_sh_rec` at 48,174,624. `a_digest_holds_every_block_it_walked`
reads `thunk_forces` 64 -> 128 for the same reason as the digest. Every
other runtime vein and the rest of the lazy tier agree with their goldens.
The work, text and compile rows are CI's, written in the second round, and
the floor moves under the 2026-09-13 clause where they come in short.

**What the full suite found.** Nine failures, three of them defects of the
branch and one older than it.

- The oracle nested a frame for every turn of a guarded loop. `eval_tail`
  hands a call in tail position back to the dispatcher's loop, directly or
  through either branch of an `if`, and had no arm for a guard: the lines
  under `return x if c` ran as a nested block, so their last call was a
  Rust frame, and the interpreter's ceiling is ten thousand of those. Older
  than this branch (`kanso run` of a module with that shape overflows on
  main), and invisible until the respell wrote `fold_flat` in it: the
  tenure fixture's `list/map` over 16,800 records ran the oracle out of
  stack where native looped, because native's `emit_tail` has kept a
  guard's tail position since the guard existed. The tail evaluator takes
  the guard now, the rest's lead runs as any block's does and its last
  statement is evaluated in tail position. Micro fixture
  `a_guarded_tail_call_runs_in_constant_stack` counts to 30,000 and folds
  12,000 elements under a guard, past the ceiling on both counts; it reads
  the stack refusal on the old interpreter and `30000` / `72006000` on both
  engines now. Row `guard_tail_oracle`.
- welfare scored a golden that had lost its run row. `work["runbench"]!`
  was the pin, and the respell's plain read answered a `none` that
  `list/to_h` stored and the kept-counter filter dropped, so a golden with
  no runbench row scored 86.00 on what was left; the arm `live none -> 1`
  the checker asked for would have done the same for a counter missing at
  scoring time. The five weighed reads are opened in `gauge`'s effect chain
  now (`pinned`), before anything is scored, and a miss ends the run naming
  the index. `a_welfare_that_prints_no_score_is_named_rather_than_indexed`
  reads the refusal again.
- Four specs embed programs that read `xs[i]!` as a value
  (`accumulator_elements_survive`, `accumulator_growth`, `carry_escape`,
  `carry_repair`); an interpolated box printed `<io>` and a builtin handed
  one was refused. Respelled the way the tree was: the bang dropped where
  the value is wanted, and `paths[at]! .> (path -> os/read_file path .> on)`
  where the read feeds a builtin.
- The three new mutations spelled their file through `$f`, which the
  `touched` pass cannot see; the paths are literal now.

**The compile veins, re-sat after the two analysis fixes.** The decoder's
emitted code reads `calls` 1,209 -> 1,212, `branches` 795 -> 807, `lines`
9,161 -> 9,258, and every other benchmark's emitted rows rise with it
(runbench `lines` 35,087 -> 36,085): the guard-bound fixes give the
beat back to loops the respell had cost it, and a beat loop is more code
than a call. `front_end_visits` 15,119 -> 15,474 and the module compile
golden's `visits` 2,511 -> 2,656 with `lines` 5,302 -> 5,377: the linearity
and chain fixpoints walk a guard's rest where they stopped at it. Both
remain well under the base (22,437 visits before this branch). The trend
gate's keys for the rows that rose: `module_branches` at 448, `module_lines`
at 5,377, `module_visits` at 2,656, `emitted_other_branches` at 13,099,
`emitted_other_calls` at 20,323 and `emitted_other_lines` at 136,463.

---

## 2026-09-16 — A folder handed to `fold` is never called by name, and a walk that found no call site answered yes (DONE)

**The defect.** `param_is_linear` in src/linear.rs marked a parameter an
accumulator when every call site handed over a uniquely-owned value at that
position. It asked that by walking the program itself, which is the same walk
`callers_hand_over` does — and `callers_hand_over` runs two refusals in front
of it, for a group whose calls the walk cannot see. Because the walk was
written out a second time, those refusals sat in a caller half that nothing on
the granting path consulted.

`escapes_as_value` is the refusal that matters here, and its own doc comment
describes this case: a group handed to a fold is mentioned as a value and
never called by name, so the walk finds no call site to object to and answers
yes for free. On that silence the folder's first parameter was marked an
accumulator it may write through. The seed the caller still holds is then
written in place, and every reference to it reads the last write.

**Not new.** Reproduced on main's own binary at b7a85b6a and on kanso#1444's
tree. `callers_hand_over` grew the refusals; `param_is_linear` never had them.

**How it surfaced.** kanso#1446's respell hoisted `(m e -> put m e[2]
(m[e[2]] / e[3]))` out of `crossed` into a named `divided`, and
scripts/welfare_rescore started reading every compile epoch's divisor as 1.0:
one map, written in place, with every stored copy pointing at it.
`tests/the_compile_epochs_flatten_their_own_boundaries.rs` read 98.8758 and
99.4822 where two rows straddling a boundary must read the same number. The
oracle read 98.6441 twice throughout.

**The fix.** `param_is_linear` asks `callers_hand_over` for its caller half.

**What it costs: nothing measurable.** `all_counters.sh` reports the twelve
cost veins and the lazy tier all agreeing, so no runtime counter moves and the
floor holds. The reason is checkable rather than lucky: the shape has four
instances in the tree — `seen_once` in trend_gate, `tallest` in
diagnostic_coverage, `guarding` in trmc_differential, `one_of` in hako/install
— and none of them is under `lib/` or `bench/`, which is what the runtime
goldens measure. The grant was live rather than dormant: `one_of` writes `push
seen name` and `guarding` writes `push acc (...)`, both in place, and both were
right only because their seeds are held nowhere else. `divided` is where that
ran out. The compile veins are host-keyed and go to CI.

**The fixture.**
`tests/golden/micro/a_folder_handed_to_a_fold_is_never_called_by_name.kso`:
two folds over one seed, in all three accumulator kinds, beside an inline
lambda doing the same thing. Watched red — map `1 11` against `1 10`, list
`[1 9 8] [1 9 8]` against `[1 9] [1 8]`, bytes `[120 121 122]` twice against
`[120 121] [120 122]`. The lambda pair was right before the fix and after it:
the fold's own arm asks whether the seed is unique before licensing a write
inside a lambda, and only the named folder reached the grant by the other
route. Without that pair the fixture would pass under a licence that had
simply been switched off.

**CI's rows, and the floor.** All three compile veins FELL:
`compile_instructions` 42,873,153 -> 42,872,854 (−299), `entry_instructions`
144,046,325 -> 144,044,890 (−1,435), `library_instructions` 144,845,876 ->
144,844,867 (−1,009). `compile_allocs` held at 27,937 and `compile_memory` is
byte-identical, so the move is the walk that no longer runs plus the layout
under src/linear.rs. `callers_hand_over` puts `is_operator` and
`escapes_as_value` in front of the walk, and a group either one refuses now
stops there instead of walking the program to be told the same thing. Welfare
rose on the compile term and was banked: floor 69.57819815695791 ->
69.57821407078485. The runtime veins did not move at all, which
`all_counters.sh` reported before the round and CI agreed with after it.

**§69 on the compiler page.** A walk that finds no call site has two readings —
nobody does this, and nobody here can see who does — and an analysis that
cannot tell them apart grants on the second. That is the presented design this
change fixed, and the page owed it an entry.

**The family, swept.** §69's shape is an analysis that grants a licence when a
walk finds no objection, where the walk cannot see every use. Two whole-program
`.all()` walks exist in the compiler and only one had it. `src/escape.rs:185`
is the opposite polarity — `body_is_safe` REFUSES on any mention of the type
outside the tail, and every body is in `program.fns`, so a use the walk cannot
see cannot introduce a mention it would have objected to. `src/linear.rs` is
the one that granted, and after kanso#1448 `callsites_unique` has exactly one
caller, inside `callers_hand_over`; the three other grant sites (the sole
finished record, the carrying slot, the constructor candidate) already asked it
and each carries a stronger condition besides. The fold's own lambda arm grants
on evidence rather than silence: `fold_owns_accumulator` needs the folder
unique AND the seed unique before it licenses a write. Nothing else in the
compiler grants on a walk's silence.

**CI's sitting, and every row that worsened.** Round one measured the seven
host-keyed veins and this is what they landed on. Every before-value below is
this branch's base, kanso#1444's stack, which is where the goldens sat when
round one ran. The trend gate's own table compares against main instead, so it
reads five of these rows from a different starting point (`work_runbench`
1,823,669,249, `work_scanbench` 460,784,763, `work_widebench` 32,135,929,
`work_readbench` 4,629,430 and `text` 1,735,116 are main's).

The compile side is the prover: infer now records a bound proof per indexed span and the none rule
reads it, which is work the front end did not do before, on every `xs[i]` in
the corpus. `entry_instructions` 140,614,828 -> **144,655,874** (+4,041,046,
+2.87%) and `library_instructions` 140,855,393 -> **145,336,811** (+4,481,418,
+3.18%) are that work counted over the two compile paths; `compile_allocs`
27,173 -> **27,395** (+222) and `compile_peak_bytes` 781,895 -> **787,956**
(+6,061) are the per-span proof table the walk holds while it runs.
`compile_instructions` 40,273,027 -> **40,749,158** (+476,131) rises against
this branch's base and is a FALL of 2,123,696 against main, which has not yet
taken the base's retired provenance fixpoint.

The run side is the 208 guards. A guard the respell wrote is a real branch in
emitted code, and the programs that run one pay for it: `work_encodebench`
3,452,269,515 -> **3,497,149,260** (+44,879,745, +1.3000%) is the largest rise;
`work_widebench` 32,103,964 -> **33,516,094** (+1,412,130, +4.3986%) is the
steepest, the widest program taking the most guards per element;
`work_deepbench` 345,129,236 -> **347,289,236** (+2,160,000, +0.6259%);
`work_scanbench` 459,778,477 -> **462,269,296** (+2,490,819, +0.5417%);
`work_digestbench` 9,830,546 -> **9,967,039** (+136,493, +1.3885%);
`work_basket` 33,549,379 -> **33,678,746** (+129,367, +0.3856%); and
`work_pendbench` 208,133,415 -> **208,138,815** (+5,400, +0.0026%), the
smallest of them. `work_jsonbench`, `work_escapebench`, `work_indexbench` and
`work_readbench` are byte-identical.

Three rows fall. `work_runbench` 1,827,443,530 -> **1,821,933,936**
(-5,509,594, -0.3015%), `work_livebench` 2,872,813,523 -> **2,825,430,323**
(-47,383,200, -1.6494%) and `work_oneshot` 18,006,613 -> **17,888,155**
(-118,458, -0.6579%). Where the bound is proven the read emits no check at all,
and those three read more than they guard.

The `text` vein moves both ways and comes out +18,224 summed, 1,737,148 ->
**1,755,372**. Eight rows shrink between 480 and 1,152 bytes each; `scanbench`
+16,000 and `runbench` +9,472 grow, and those two are the whole of the rise. A
guard is an emitted branch and a retired `]!` site is a call that goes away, so
both directions are expected here; which one wins is per program and is not
attributed further.

The objective weighs the two sides together and comes out 0.0422 down. The
floor moves with it, 69.63579260553377 -> 69.59359812089627, under the
2026-09-13 ironclad clause, with the reason recorded in
`bench/welfare_floor.json`'s history entry beside the number.

**CORRECTION, and a reproduction failure: one binary counted two compile rows
on two CPUs.** The paragraph above writes round one's sitting, and round two
disagreed with it on the three compile rows: `compile_instructions` +149,
`entry_instructions` +41, `library_instructions` -127. The two rounds ran
identical source for everything the measurement reads — ten files changed
between them, seven goldens and `bench/welfare_floor.json`, this log and
`docs/compiler.html`, and nothing under `src/`, `lib/`, `hako/` or
`bench/*_corpus/`; there is no `build.rs`, `include_str!` reaches only `lib/`
and `hako/`, and `library_box.sh` stages the binary, `lib/` and the three
corpora at a fixed path with the environment emptied. All fourteen work rows
and all fourteen text rows agree to the instruction across the pair.

The gates print the hunt's first question and answer it:

    round one  library_sample cpu="cpu family 0x19 model 0x1" sha=75988d5b311a row=145336811
    round two  library_sample cpu="cpu family 0x1a model 0x2" sha=75988d5b311a row=145336684

One sha, two CPUs, two rows, which is case (2) by the gate's own text: a
reproduction failure, hunted to its source and never pinned as a second value.
Family 0x19 is Zen 3 and family 0x1a model 0x2 is Zen 5, new to the pool.
`compile_instructions.sh` rests on eight within-binary sittings across two
vendors and four CPU generations agreeing to the instruction, and those
sittings predate this silicon.

WHERE IT IS NOT. libc was the first guess and the profiles refute it. Every
libc frame the annotation shows is identical across the two chips:
`_int_free` 5,479,699, `_int_malloc` 5,428,597, `__memcmp_avx2_movbe`
4,656,489, `malloc` 4,015,271, `__memcpy_avx_unaligned_erms` 2,998,404. The
same memcmp implementation is selected on both and counts the same on both, so
the ifunc choice the tunables do not pin is not what moved, and neither are the
cache-derived thresholds the gate already pins. The one frame in the top
fifteen that differs is the compiler's own: `kanso::check::check_after_infer`
4,020,015 -> 4,019,967, -48, with the remaining -79 below the annotation
threshold. So the compiler's own instruction count moves with the host CPU
under valgrind, which is a sharper thing than a libc path and is not yet
attributed further.

So the three goldens here hold round one's numbers and are deliberately not
regenerated. The remedy reopens the 2026-09-05 ruling "one row, one value; the
pair and the per-chip key are retired", so it went to Clay. It exposes every
open pull request carrying a compile vein: which chip a round lands on decides
whether that vein is red.

**THE ATTRIBUTION ABOVE IS WRONG, AND THE CAUSE IS IN THIS PULL REQUEST.** The
two paragraphs before this one blamed the host CPU, and a third CI round
refuted them: at `11b282d1`, on source identical again, the rows read
`compile_instructions` 40,749,259, `entry_instructions` 144,655,962,
`library_instructions` 145,336,861. Three rounds, three distinct values on
every row, over a pool of two chips. A per-chip story predicts two.

The measurement that found it runs on one machine. Eight repeats of the library
gate's own command, same binary, same box, same tunables, gave 146,776,367 /
146,776,546 / 146,776,647 / 146,776,453 / 146,776,334 / 146,776,489 /
146,776,401 / 146,776,335 — a spread of 313 instructions with the process total
minus the `kanso::main` row constant at 470,409 throughout, so the variance is
inside the compiler's own frame. Five more under `setarch -R` varied the same
way, which rules out ASLR. Diffing two of those callgrind profiles function by
function returned one line:

    functions present in both that differ: 1
        +119  hashbrown::map::HashMap<K,V,S,A>::insert
    only in x4: 0   only in x5: 0
    sum of diffs: 119

Every other function in the profile is identical to the instruction. The whole
delta is one hash table's inserts.

Which table: this branch declared `proven` three times as
`std::collections::HashSet<crate::diag::Span>` — `src/infer.rs:61`,
`src/infer.rs:156`, `src/check.rs:803` — where the rest of the compiler uses
`crate::hash::Set`. `std` defaults to `RandomState`, keyed from the OS once per
process, so the probe sequence differs on every run and so does the work of
building the same set. The base, `claude/bare-err-refused`, has no such
container in either file. Spelling the three as `crate::hash::Set` put six
consecutive runs of the library measurement on 146,522,240, spread zero.

So the gate was right and its case (2) did its job: one binary counting two
numbers is a reproduction failure, and hunting it to its source is the rule.
The 2026-09-05 ruling "one row, one value" needs nothing; the escalation raised
against it is withdrawn. `src/hash.rs` says iteration order changing is
harmless because nothing observable depends on it, which holds for what the
compiler writes and not for what it costs — three goldens count the cost to the
instruction. `tests/the_compile_path_hashes_with_a_fixed_seed.rs` now reads
`src/` and fails on a std-hashed container outside a named exception, watched
red on all three sites before it went green.


**The ruling's two cost levers, censused: one is built and the objective cannot
see it, the other is DECLINED before building.** STATUS.md's explicit-box row
owes "an inlined bind for a pure index read and bound discharge for a literal
index into a known-length list, each measured". Both were named on 2026-09-16,
the same day as the respell, and the respell is what decides them.

LEVER ONE is built here. `emit_call_full` matches a `.>` whose subject is a
strict index and emits the read, the callback and the settle inline: the bind
node, its closure and the box under it are never constructed. It fires on ten
sites in the tree — four book samples, six goldens — and on five lines of
`scripts/welfare/welfare.kso`. Zero in `lib/`, `hako/` or `bench/`. Nothing the
objective measures writes `xs[i]! .>`, so the emission is right and no counter
in welfare can price it. It stays because it is the correct shape for the
construct, not because a row moved.

LEVER TWO reaches eighteen sites and not one of them is hot. Nine index a list
literal directly, nine index a name bound to one, and every one is a sample or
a golden demonstrating a MISS: `flavors[9]`, `xs[9]`, `prices[9]`, `xs[5]`,
`[10 20 30][9]`. The benchmarks' literal indexes are `xs[1]`, `es[1]` and
`bulk[100000]`, whose containers are parameters and built lists — no length the
checker could know. So discharge would add a length-tracking analysis to the
checker, paid for on all three compile rows, to fold a branch in nine programs
that run once. The objective reads that as a fall with no term to set against
it. Declined, before building.

Why both come out this way is the same fact. The respell took the shipped
corpus from 570 bang-index sites to five: `lib/` 107 to 0, `hako/` 55 to 1,
`bench/` 17 to 0, `scripts/` 391 to 7, and three of the eight survivors are
comments. That is the ruling's "no bang where the bound is provable" half doing
its work, and it removed the construct the levers were written to optimise. The
levers were sized against the corpus as it stood before the respell they
shipped beside.


**CI's sitting on the fixed head, and the floor corrected upward.** The rows
above were all measured against a randomly-seeded table, so none of them was
the respell's cost. With `proven` spelled `crate::hash::Set` the measurement
repeats, and CI on `647e58f7` reads `compile_instructions` 40,703,283,
`entry_instructions` 144,436,311 and `library_instructions` 145,118,874. Only
those three veins moved. `compile_allocs` held at 27,395, `compile_memory` at
787,956 byte-identical, and every work row, text row, emitted row and
machine-code row AGREED — which is what a hasher swap predicts, since the same
table is built with the same number of allocations and a different probe
sequence. The profile shows it directly: the top frames now carry
`hashbrown::map::HashMap<&str, (), BuildHasherDefault<kanso::hash::Fx>>::insert`
at 3,392,241 and no std-hashed table at all.

So the respell's cost against kanso#1444's base is smaller than round two
recorded: +430,256 on the module row (+1.068%, not +476,131), +3,821,483 on the
entry row (+2.718%, not +4,041,046), +4,263,481 on the library row (+3.027%,
not +4,481,418). The objective reads 69.5960 where the hand-set floor stood at
69.5936, so the floor is ratcheted to 69.59603943391699. Round two lowered it
under the 2026-09-13 ironclad clause, which was the right clause and the wrong
number: it was computed on one draw from a distribution.

`tests/the_score_says_what_it_was_made_of.rs` caught the gap between the two
before this entry was written — the rescored column read 69.5936 where welfare
scored 69.5960 — and goes green on the banked floor. Eight `data-golden` spans
on `docs/compiler.html` quoted the old rows and were rewritten by
`golden_prose --write`.

## 2026-09-16 — The lazy-verdict ratchet row proved nothing, and scan_counters had no row at all (DONE)

**The row reported UNBUILT, and it was main's.** kanso#1447's ratchet run
returned 113 rows, 112 red and one refused: `a lazy verdict leaking from one arm
of a group to another: it would not build`. The first thing to establish was
whose it was, because a row that fails only on a branch is that branch's work.
It is not: the same mutation on origin/main at 1e4c7129 fails identically, so
the row has been proving nothing on main for as long as the shape has been
there.

**What the mutation does now.** `lazy_verdict_leaks_between_arms.sh` flips
src/demand.rs's per-arm fold from `*seen = *seen && qualifies` to `||`, so a
bind goes lazy when ANY arm qualifies rather than every one. A lazy verdict is
keyed `(group, arity, index)` and codegen consults it with no arm to
disambiguate, which is what the fold's own comment says. Under `||` the emitter
builds a thunk in the arm that qualified and releases it at the group's merged
tail, which the other arm reaches. In the mutated scanbench.ll: `%t10 =
k_thunk_new` in block `arm0`, `%t123 = k_thunk_release_unless(%t10)` in block
`L24`, reached from `nomatch` as well. `opt -passes=verify` says `Instruction
does not dominate all uses!`; clang runs with `-disable-llvm-verifier`, so
instead of rejecting the module it SIGSEGVs in the Register Coalescer on
`@"d_regexp/braced_3"`, inside `llvm::LiveRange::join`. Ruled out on the way:
not LTO (it fails with `-flto` off), not the inline threshold (250, 1000 and
2000 all die), not the stack (64 MB does not help).

So the row's claim — thunk_allocs 0 becoming 501,502, peak RSS up about 92 MB —
is stale. The leak now kills `build_benchmarks.sh` before a counter exists, and
a setup that fails reads UNBUILT, which scores not-ok. The ratchet's own header
has said since #988 that a row whose defect IS a build failure carries
`no_setup` and builds inside its gate, and that is where this one belongs: the
micro corpus already catches it, in 1.56 seconds, at
`a_callable_that_is_a_value answers differently as a library`. The row is
re-pointed at `cargo test --release --test golden
micro_corpus_agrees_across_engines` as `scan_leak`.

**The smallest program that fails is fourteen lines**, and it is recorded here
rather than added to the corpus, because the corpus already has a home for this
defect and a second fixture that can never be the one that speaks proves
nothing. Two arms of one group, both binding at index 0, one qualifying and one
not:

    fn cost n
      n + n + n

    fn keep v
      v

    pub fn spend x false
      held = cost x
      keep held

    pub fn spend x true
      held = x + 1
      held + held

Built as a module with `print "{spend 3 false} {spend 3 true}"`, the real
compiler answers `9 8` and the mutated one writes `%t7 = k_thunk_new` in `arm0`,
`%t9` in `arm1`, and both releases in `L8`.

**And moving the row left `scan_counters` with none.** It was the only row that
named that gate, so re-pointing it would have traded a loud failure for a quiet
one — the thing this repository calls a gate nobody has proved can fail. A new
mutation, `the_carry_tier_admits_library_loops.sh`, clears beat_loops' `std/`
and `lib/` prefix filter so library loops enter the carry tier, which is the
mechanism bench/scanbench's own header names as the reason its peak grows with
the subject.

MEASURED 2026-09-16, and the header is stale on the numbers: clearing the filter
moves `beat_iters` 15 -> 16 and `survive_slots` 0 -> 2, and `arena_blocks` does
NOT move. The 2026-09-08 sitting the header records cleared the BYTES condition
and the unbracketed-entry check as well, and the filter is one of its three
parts; a third of the change is not a third of the effect. The gate diffs the
whole golden, so two moved rows turn it red exactly as arena_blocks would, and
the row's claim is written as what it does rather than as what the header
predicted.

REFUTED first, and recorded because the next session will think of it too:
making `k_beat_iter` stop rewinding leaves the scan gate GREEN. scanbench's
`beat_iters` is 15, so the beat rewind barely runs there and removing it moves
nothing the golden pins.

## 2026-09-16 — a box handed to a binding parameter reaches the dispatch, and the dispatch answers wrong (OPEN)

Found while respelling kq for this change, and it is this change's own, so it
is recorded here rather than filed elsewhere.

`check.rs`'s effect refusal covers an operator, an index, a field read, an
`if` condition and a builtin that reads a value. At a call it refuses a box
argument too — unless the callee's group BINDS at that position:

    if found.is_some() && (pos >= 64 || binds & (1u64 << pos) != 0) {
        continue;
    }

A bare binder can hold a box, so the licence is not wrong on its face. What
it does not ask is what the binder's own body then does with the name, and
one hop later the box is standing in front of a dispatch that was written for
values.

Eleven lines, and check says `ok`:

    fn seen 7
      "seven"

    fn seen _
      "other"

    fn step x
      seen x

    pub fn run xs
      step xs[1]!

`run [7]` prints **other**. Drop the bang and spell the bound — `return "empty"
if length xs < 1` then `step xs[1]` — and the same program prints **seven**.
The box misses the `7` arm, falls to `_`, and nothing anywhere says so.

Take the `_` arm away and it is louder but no earlier:
`error[runtime]: no overload of `g/seen` matches these arguments`, at run
time, from a program the checker passed. That is the shape kq hit: three of
its ten unit tests failed that way before the respell — `encode_onto`,
`pretty_onto`, `pretty_entry` — each one an `xs[i]!` carried through
`elem_onto`/`elem_row`'s binding parameter into a group with no arm for a box.

**Both halves are one question, and it is a design question rather than a
bug with an obvious patch: how far does a binding position licence a box?**
Three answers are available and they are not equally cheap. Refusing a box at
any position of a group whose other arms are literals would catch both halves
here and would also refuse `bind`-shaped code that means it. Tracking the box
through the parameter is the honest answer and is a typing change, not a
check. Leaving it and letting the dispatch answer is what ships today, and
the first half of this entry is what that costs: a wrong answer, silently.

Nothing here blocks the respell — every site in this tree and in kq is spelled
so that no box reaches a dispatch — so this is a hole in the checking rather
than in the change. It stays OPEN.

---

## 2026-09-16 — the bare-binder answer is measured and DECLINED; the import check could not read a qualified yield (DONE)

Two things, and the second is what the first turned up.

### The bare binder: measured, and it contradicts ten pinned fixtures

The entry above — "a box handed to a binding parameter reaches the dispatch,
and the dispatch answers wrong" — offered three answers. The cheapest is
already half built: `check.rs` keeps one bit per position per dispatch group
saying the group takes a box there, and three spellings set it, a bare name,
a `_`, and an arm annotated `e:<int>effect`. Taking a bare name off that list
is a five-line change. It was built and swept.

**Against the tree's modules it costs one site.** Every module under `lib/`,
`scripts/`, `bench/`, `hako/`, `examples/` and `docs/` checked against the
patched compiler and against an unpatched one: only
`scripts/module_differential` moves, where `laid`, `laying` and `made` thread
an unopened `os/run` chain so a directory's files are written in order.

**Against the corpora it costs TEN fixtures, and two of them are the ruling.**
`a_description_reaches_a_dispatch` pins that a description handed to
`seen v:cell` / `seen _` takes the bare arm on all three engines.
`a_plain_dot_hands_the_box_over` pins that `held e` receives the box and hands
it back, with the comment "the words are the only doors; the dot opens
nothing". Eight more say the same in other containers:
`a_container_does_not_run_what_it_holds`, `a_description_renders_in_an_interpolation`,
`_rides_in_a_constructor`, `_rides_in_a_field`, `_rides_in_a_list`,
`_rides_in_a_map`, `_rides_through_a_builtin`, and `the_box_built_by_hand`.

So carrying a box through a plain parameter is not the defect. It is the
language, pinned, on three engines. The first half of the entry above —
`step (os/read_file "missing.txt")` printing `step got: <io>` — is
`a_description_renders_in_an_interpolation` doing its job.

What is left is the second half, and it narrows to one sentence: **a box that
travels through a parameter into a LATER call is invisible to the checker.**
`encode_onto` handed a box directly is refused today; the same box handed to
`elem_onto x` and then to `encode_onto x` inside that body is not, because
nothing says `x` holds a box. That is the typing change the entry above called
the honest answer, and the ten fixtures are why the blunt substitute is not
available. Filed to the ledger as the design question it is; the entry above
stays OPEN, and this entry is its measurement.

### The import check read `<os/process>effect` as the module `<os`

Writing the annotation the declined rule would have needed turned up a defect
of its own, older than either branch. A module file that says

    fn laid root files at after:<os/process>effect

is refused:

    error[import]: `<os` is not imported here — a module's files share their
    declarations, not their imports

The import check asks each file which module qualifiers it uses and asked by
splitting a type name at its first slash, so the whole spelling
`<os/process>effect` answered `<os`. No import matches that, so the file is
refused for borrowing an import it wrote, and in the same run that import
reads as unused: two diagnostics, both wrong, for a program that compiles.
`map[string os/process]` splits the same way, and `[]os/process` would.

The check scans the runs of name characters now and marks the ones holding a
slash, so a shell of any shape carries its names through. Module fixture
`tests/golden/qualified_yield` with `tests/a_qualified_yield_inside_an_effect_type.rs`,
watched red on the unpatched compiler on both engines; ratchet row
`qualified_yield`.

Nothing reached this before because the only way to write a qualified yield
was to want one, and the annotation is rare. It is reachable from any module
that names another module's type inside a shell.

## 2026-09-16 — a measured decision was filed to nobody for a day, and STATUS.md disagreed with itself about the queue (DONE)

The 2026-09-15 entry "the per-call floors, mapped after the inlines" ends by
measuring a change and saying, in its own words, that it "goes to Clay with
this number and is not built here". No entry was ever written in
`design/pending-gavels.md`, so it went to nobody. The change is a byte-position
scan on a string for the JSON escape path, worth runbench 1,823,814,374 ->
1,801,576,724 on the kanso#1437 leaves, −22,237,650 and −1.2193%, and it sat
where only a reader of the log's middle would find it. It is filed now under
Open, not blocking, with the measurement and a recommendation.

**Found by re-deriving a map that was already right.** The queue's two biggest
run rows were profiled again on this branch to look for a fresh lead:
`d_json/encode_onto_2'2` reads 392,547,176 of 1,840,366,969 (21.33%) against
the 2026-09-15 sitting's 382,082,442 of 1,823,814,374 (20.95%), the same shape
one stack later, and `obj_key_start` the same. Both are mapped and both have
had their removable parts found — encode_onto's is the view above, and
obj_key_start's was largely refuted on 2026-09-14. The profile turned up no new
lead, which is the result: the run term's two largest rows are closed, and one
of them is closed on a question waiting for Clay.

Worth naming for a later session: callgrind shows `encode_onto_2'2` with three
kanso callees missing from its callee list — `escape_onto_2`, `encode_map_2`
and `encode_list_2` are all inlined into it under the 2000 threshold
kanso#1391 set. So the 21.33% row is the whole encoder, not one function's
overhead, and the 2026-09-15 map already reads it that way. A session that
takes the row for one function's dispatch will chase twenty-seven instructions
and find nothing.

The same reading applies one row down and in the other direction.
`d_runbench/tally_4` is 91,704,604 (4.98%) and `tally` is four lines of
benchmark harness, which invites reading five per cent of the objective's
headline term as bookkeeping. It is not: `escape/total`, `index/total`,
`split/total` and `digested` are all inlined into it, and the row calls `k_b_at`
690,000 times and `k_beat_iter` 1,552,821 times directly. Those are the
benchmarks. Nothing in the harness is worth removing, and a session that tried
would be editing the corpus to make a number smaller.

**And STATUS.md contradicted itself.** The file indexes the ledger three times
— an overview sentence near the top, a detail sentence with the split, and a
paragraph that lists the open entries one by one — and only the detail one was
pinned by `tests/the_status_index_counts_the_ledger.rs`. The overview read
"Three questions are waiting — one blocking" while the file's own opening
paragraph read "Blocking right now: zero" and the ledger held no Blocking
entry, and the list paragraph read "The two open, not blocking" while naming
an entry that had been ruled and built on 2026-09-15. All three are read off
the ledger now, each watched red on the stale text before it was corrected.

The gap this does NOT close: nothing checks that a log paragraph saying a
question goes to Clay has an entry to go to. A scan for the phrase would pass
over every historical entry that has since been ruled, so it would either be
noisy or would need a list of exemptions that goes stale the way the counts
did. The count spec catches an entry filed and miscounted; it cannot catch one
never filed. That is written down rather than guessed at.

## 2026-09-16 — the next three run rows, all closed by reading them (DONE)

Below `encode_onto` and `obj_key_start` the profile's next rows are
`array_delim` at 85,720,338 (4.66%), `scan` at 77,645,700 (4.22%) and
`str_escape` at 65,074,779 (3.54%). None of the three holds a lead.

**`array_delim` is the array and object openers inlined into it.** Its callee
list is `array_open` and `obj_open`, each in both recursion contexts, at
374,499,500 and 279,024,609 inclusive. The row is the container walk, and the
work under it is `scan` and `str_chars`, which have rows of their own.

**`scan` converts each number exactly once, and the counts prove it.** It calls
`k_b_to_float` 210,177 times and `k_b_to_int` 207,306, which reads like a
double parse until you add them: 417,483, and `k_b_slice_raw` is called 417,483
times, one slice per number. `number_done` dispatches on the mark the scan
carries and takes one arm. The corpus is about half floats. Per conversion the
float path is 190 instructions and the int path 93, both already worked by
kanso#1423, kanso#1427 and kanso#1428.

**`str_escape`'s residue is `k_b_utf8`, 175,527 calls at 170 instructions.**
That is one per escape event on the decode side, and it is 1.63% of the run
program. Small, and the escape path is where kanso#1291 and kanso#1367 have
already been.

So the run term's five largest rows are all read: two mapped with their one
removable piece now waiting on Clay, and three with nothing under them. The
queue's next run-side lead is not in this profile at this granularity.

## 2026-09-16 — CI's rows for the shell's several names, and what the early-out took back (DONE)

Round one priced the first shape of the fix and it was expensive: `mark` runs
on every identifier in every body, and scanning the runs of name characters
built a split iterator for each one. CI read library 145,118,874 ->
146,911,031, +1,792,157 and +1.23 per cent, with entry +1,572,425 and module
+484,887; every other vein AGREED.

The no-slash early-out in dd9ffcc9 answers the common name — one holding no
slash, and so no qualifier anywhere inside it — on one scan, and never reaches
the iterator. CI's round two, all three rows against this branch's base:

| row | base | round one | round two | recovered |
| --- | --- | --- | --- | --- |
| `compile_instructions` | 40,703,283 | +484,887 | **+127,436** (+0.3131%) | 357,451 (73.7%) |
| `entry_instructions` | 144,436,311 | +1,572,425 | **+363,524** (+0.2517%) | 1,208,901 (76.9%) |
| `library_instructions` | 145,118,874 | +1,792,157 | **+363,516** (+0.2505%) | 1,428,641 (79.7%) |

`compile_allocs` held at 27,395 and `compile_memory` is byte-identical; the
emitted, machine-code and every runtime vein agreed in both rounds.

The three rows land at `compile_instructions` 40,830,719,
`entry_instructions` 144,799,835 and `library_instructions` 145,482,390.
All three rise, and the rise is what the qualified-name scan costs after the
early-out has taken back three quarters of it.

**The local A/B tracked CI to 0.77 per cent.** This container refuses to
compare the absolute rows — other silicon, other glibc — so the fix was priced
here as a three-point delta on one box, same path, three builds: pre-fix
146,522,240, the run scan 148,311,311 (+1,789,071), the early-out 146,882,961
(+360,721). CI reads the same two deltas as +1,792,157 and +363,516. The
first pair agree to 0.17 per cent and the second to 0.77, which is the
cross-check that a host-refused local measurement is measuring what the gate
measures. Worth writing down because the refusal is easy to read as "this box
can say nothing": it can say the delta, and the delta is the claim.

The residue is what a slash-bearing name costs to scan properly, and that is
the fix rather than an overhead on it. The mutation still applies and the spec
still goes red under it with the same `error[import]: `<g` is not imported
here`, watched again after the early-out went in: the names the early-out lets
through are exactly the ones the fix protects.

**The floor comes down 69.59603943391699 -> 69.59152459326125**, a fall of
0.0045, under the 2026-09-13 ironclad clause. Effects are types is ruled
(kanso#1372, kanso#1395); a qualified name inside a type shell is how that
spelling is written; and a program the compiler accepts cannot be one the
import check refuses for an import the file wrote. That is the specification,
and this is what it costs.
## 2026-09-16 — a second decision found sitting in the log with nobody to receive it, and the check that now reads for them (DONE)

The entry above this one closed by naming a gap it did not fix: nothing checked
that a log paragraph saying a question goes to Clay had an entry in
`design/pending-gavels.md` to go to. It found one such paragraph, a
byte-position scan worth −1.2193% of the run term, filed a day late.

Sweeping the same file for the rest of the family found a second, from the
2026-09-15 entry "the maps parse is outside all three compile rows". Pinning
`.rodata` to a fixed page removes the "by layout" term from the compile rows
for anything growing ahead of it: on two sources differing by a hundred
functions, `program` reads 43,471,592 on both against an unpinned pair that
differed by 5,849. The paragraph prices it — about 1 per cent of binary at
0x100000, or 52 KiB at 0x40000 with a loud link failure when outgrown — and
closes "is Clay's, and goes to him with these numbers rather than to the
ledger". `design/pending-gavels.md` is the only channel a waiting decision
has. A session cites entries by heading, never by a task id, because task ids
resolve nowhere outside the session that made them, so "to him rather than to
the ledger" is a decision addressed to no one who can receive it. It is filed
there now, under Open, not blocking, with a recommendation to decline it.

Two instances two days apart is a process defect rather than a slip, so the
check is built: `tests/a_question_sent_to_clay_has_a_ledger_entry.rs`.

**What the check reads, and the two objections it had to answer.** The gap
paragraph's own words were that a scan for the phrase "would pass over every
historical entry that has since been ruled, so it would either be noisy or
would need a list of exemptions that goes stale the way the counts did". Both
halves are answered rather than worked around.

The noise is answered by what a send carries. A decision that goes to Clay goes
with its measurement — the filing rule says an entry carries the numbers behind
it — so a paragraph counts as a send only when it holds a grouped number of
five figures or a percentage. That is the whole difference between the two real
sends above and the paragraph that merely describes the phrase; the latter
names no number, and it is skipped for that reason rather than by name.

The exemption list is answered by not having one. The live log is append-only,
so the 2026-09-15 paragraph cannot be edited to cite an entry filed on
2026-09-16 — and it does not need to be. A send is satisfied when its own
paragraph names the ledger file, OR when a later paragraph names it and quotes
one of the send's own measurements, which is exactly the shape a filing entry
takes. So the byte-position send reads as answered by the entry that filed it,
through the shared 1,823,814,374, with nothing to keep up to date. This
paragraph does the same for the `.rodata` send, through 43,471,592.

Watched red first, and for the right reason: on the tree before this entry the
spec named exactly one paragraph, the `.rodata` one, and quoted it in full.
Ratchet row `clay_send_filed`.

**What it does not do.** It cannot tell a decision that is Clay's from one the
implementer should settle, and it does not try; it reads the log's own words
for a send and asks only that the send have somewhere to land. A question
settled without ever being written down as Clay's stays invisible to it. The
count spec catches an entry filed and miscounted, this one catches an entry
never filed, and neither catches a decision never written.

## 2026-09-16 — the railway's remainder: one false claim, and a page sentence that was owed and is paid

The 2026-09-15 ruling's "What is left" says cloud builds the constructor and
retires the railway. The constructor is kanso#1440. The railway's retirement
turned out to be mostly built already, inside kanso#1444, and reading main
rather than the part-3 tree is what made it look owed: ch04 carries the
checker's rule beside the metaphor now, §"nothing is asked of the signature"
already asks its question at the call, ch07's railway sentence is gone and
appendix B's "passes straight through, unlooked-at" with it.

What the survey found still owing, measured on the tree that holds part 3:

**ch08 said every caller, and the chapter's own library is the counter-example.**
"the railway from chapter 04 carries it out through every caller with its
position intact" was written before a caller the checker can see had to name
the err. lib/json now holds five hand-back arms for exactly those callers —
`finish` in json.kso, `array_step`, `obj_key` and `obj_value` in value.kso,
`str_low` in text.kso — so the sentence is refuted by the code the paragraph
is walking. It names them now and says which callers pay.

**ch04's metaphor outran the rule it introduces.** "no station on the line can
flag the train down" is followed immediately by a paragraph saying the checker
refuses the program where it can see a raised err arriving. A station stops
the train where the checker can see it coming, which is what the next
paragraph then explains.

**The compiler page's owing list had gone stale.** §"what the ruling leaves
owing" said chapter 4 "needs re-premising on explicit bind". kanso#1444 did
that re-premising; the sentence records it as done and names what the section
says now.

Every ch04 sample `kanso play` can run answers its committed `.out`
byte-for-byte on this tree, `railway.kso` among them: `share_of` raises,
`with_tip`'s `share + share / 10` compiles because `share` is a name and the
checker reads calls, and the endpoint reports it. The railway retires exactly
as far as the checker can see, which is the bound ch04 now documents and the
reason these three sentences were the whole of the remainder.

## 2026-09-16 — a build hole is spelled `_`, and the checker fills it exactly once

**DONE.** The 2026-08-24 gavel, "a build hole is spelled `_`, and fills
exactly once" in the archive, built end to end. It sat unbuilt for
twenty-three days because it was never on the unbuilt list; the entry above,
"a ruling from 2026-08-24 was never on the unbuilt list, and the sample it
condemned still ships", put it there, and this is the build.

**The rule.** Inside a `build` block, a construction's argument may be `_`:
a hole for a field the block fills later. A hole is filled by exactly one
field write, made through the name of the record built with it, before the
block freezes. `none` keeps its one meaning and never stands in for a field
that is coming.

**What the parser does.** `_` is an atom (`Expr::Hole`), and the parser
admits it where an argument starts; until now a bare `_` after a
constructor was `unexpected trailing tokens`, which is what STATUS.md's row
probed. The pattern refusal is untouched: `_` in a binding pattern still
says "omit fields with a keyed read", and appendix A's paragraph now says
the character has one job and a pattern is never where it goes.

**What the checker does.** The block-born walk (kanso#1359's proof, folded
into the one descent in kanso#1386) already knows which names a block made
and which fields they were made with. A construction bound to a name inside
a block pushes a hole per `_` argument, keyed by the record's birth and the
field. A field write asks the holes before it asks anything else, and there
are seven refusals, each with an errors fixture:

- `_` anywhere else — a top-level construction, a list element, an argument
  to a function — is refused where it stands: nothing could fill it
  (`a_hole_outside_a_build_block`, two spellings in one fixture).
- a write to a field that was built with a value is refused: the field the
  block fills is built with `_`
  (`a_field_written_that_was_not_left_as_a_hole`). This is the retired
  spelling, `ada = person "ada" none` then `ada.partner = bob`, and it is
  now a compile error rather than the idiom the book taught.
- a second write to a filled hole is refused (`a_hole_filled_twice`).
- a write inside an `if` arm may not run, and a hole is filled exactly once,
  so it is refused with "fill it outside the arm"
  (`a_birth_recorded_inside_an_if_arm`, which now reports three diagnostics
  where it reported one: the conditional fill, the write through a name the
  arm's answer left unproven, and the hole nobody filled).
- a write through a record an `if` chose, or an element a list literal
  holds, is two records to the checker, and would fill one hole and leave
  the other open; it is refused, and the hole it would have filled is
  reported unfilled at the freeze (`a_hole_filled_through_a_chosen_record`,
  `a_hole_filled_through_an_element`;
  `build_write_a_field_an_if_may_have_overwritten` gains this diagnostic
  ahead of the one it had).
- a hole nobody filled is refused when the block freezes, at the `_`'s own
  span (`a_hole_never_filled_before_the_freeze`).
- a write to a field the type never declared is refused before the hole
  question is asked, with the sentence a READ of that field gets, `` `node`
  has no field `nope` ``. That fixture,
  `a_field_write_names_a_field_the_type_lacks`, lived in the runtime corpus
  pinning the sentence native and the interpreter say when the write runs;
  it moves to the errors corpus, because no checked program reaches the
  runtime site now. The two `has no field` sites at the write in runtime.c
  stay, and the read path still pins their words.

**What the engines do.** Nothing. A hole is the `none` word on all three
engines until its fill runs — the emitter writes the none constant, the
interpreter binds `NoneV`, the page emits the `none` identifier — and the
checker is the whole of the enforcement. So a read of a hole before its fill
sees `none`, and `a_hole_read_before_its_fill_is_a_none` pins that on both
engines rather than leaving it to be discovered: the block
`ada = person "ada" _`, `early = ada.partner`, `ada.partner = bob` reads
`<none> bob`. The alternative, a runtime sentinel the engines would have to
carry and test on every field read, buys nothing the checker does not
already prove.

**Fill is by name, and an alias counts.** `born_of` resolves a name bound to
another born name, or a field read that lands on one born record, to that
record's birth, so `pair = ada` then `pair.partner = bob` fills ada's hole
exactly as `ada.partner = bob` would. What cannot fill a hole is a name
whose birth is `Either`: the checker cannot say which record the write
reaches, so it cannot say the hole was filled once.

**The corpus.** Beyond the errors fixtures:
`a_build_writes_what_it_can_prove_was_born` rewritten with holes where it
had `none` placeholders and an untouched `.out`;
`a_knot_equals_the_same_cycle_built_in_a_block`,
`a_description_rides_in_a_field`, `build_after_guard` and
`build_nested_cohort` respelled; the runtime fixture `build_set_err` respelled
with the err in the first field and the hole in the second; the mem fixture
`build_cycle` respelled, and its vein moved — `allocs` 70 -> 66,
`alloc_bytes` 3,264 -> 3,072, `sh_buf` 304 -> 144 — because the two `[]`
placeholders it built and then overwrote were two buffers the hole does not
allocate. Four examples respelled (`build_blocks`, `build_contained`,
`build_cyclic_eq`, `none_is_a_value`), stdout goldens unchanged. The book:
ch03's `knot.kso` reads `ada = person "ada" _`, its panel regenerated, and
the paragraph under it teaches the hole and the four refusals instead of
"`none` holds ada's place". The playground's `build` and `contained` samples
respelled. STATUS.md's row also names `tests/golden/micro/bare_field.kso` as
the same defect; it is not — `p = person "ada" none` there is a top-level
construction with a genuine absence and no write, and it stands as written.

**The ratchet.** Seven rows, one per refusal, each patching check.rs to
disarm one test and each proved by the applies pass: `hole_unfilled`,
`hole_twice`, `hole_placeholder`, `hole_in_arm`, `hole_chosen`,
`hole_outside`, `hole_type_lacks`.

**The counters.** CI's sitting on the kanso#1444 base, all three compile
rows falling: `compile_instructions` 40,273,027 -> 40,184,361 (−88,666,
−0.2202%), `entry_instructions` 140,614,828 -> 140,399,833 (−214,995,
−0.1529%), `library_instructions` 140,855,393 -> 140,641,362 (−214,031,
−0.1520%). The fall is the two `none` placeholders `build_cycle` used to
construct: a `none` argument is an expression the checker walks and the
emitter writes, and a hole is neither. `machine_code`, `compile_allocs`,
`compile_peak_bytes` and every runtime row agreed. The mem vein's three
moves are named above. Welfare rises 69.63579260553377 ->
69.63860211489504, banked with `--set` in this PR; eight page spans across
five paragraphs quote the three compile goldens and moved with them.

**ROUND THREE, and the first reading was measured on a tree the base had
moved out from under.** The rows above were read before main's kanso#1448
reached this branch through kanso#1444's tip. CI's fresh sitting on the
merged tree, against the same base, reads `compile_instructions` 40,273,027
-> 40,269,818 (−3,209, −0.0080%), `entry_instructions` 140,614,828 ->
140,695,826 (+80,998, +0.0576%), `library_instructions` 140,855,393 ->
140,935,957 (+80,564, +0.0572%). The base row is the same number on both
trees and CI verified it either side, so the base did not move; the two
single-file rows fell 214,995 and 214,031 in the earlier round and rise here.
kanso#1448 moves `src/linear.rs` and the layout under it, the hole's edit
lands in `src/check.rs` on top of that, and compile_instructions is a layout
vein — it has recorded layout-only moves before. So the hole's own effect on
these rows is smaller than one sitting made it look, and neither sitting is
wrong about the tree it was taken on.

`compile_allocs` held at 27,173, `compile_memory` is byte-identical, and
`machine_code`, `emitted_code`, `compiler_libraries` and all fourteen runtime
rows agreed. The summed compile term rises, so welfare falls 69.63860 ->
69.63510 and the floor moves with it, by hand, under CLAUDE.md's ironclad
clause: `_` is the 2026-08-24 gavel, and a change that builds a ruled part of
the language lowers the floor by exactly what it costs.

## 2026-09-16 — five pull requests land as one tip, and a priced counter went unpriced the moment its neighbour landed

**DONE.** kanso#1444 merged at 16:22Z as `fd789b8c`, and took kanso#1440,
kanso#1441 and kanso#1442 with it. The four branches left standing —
kanso#1447's build hole, kanso#1449's `!` respell, kanso#1452's qualified
yield with kanso#1453's ledger check on top of it, and kanso#1450's railway
remainder — are merged into `claude/ledger-reachable` and land as one tip.

**Why one tip.** Branch protection refuses a head that is behind main, so
every landing sends every other open pull request back for a fresh round. The
ratchet's touched pass is what prices that: it selects the rows patching a
file the branch changed and proves each by a release rebuild and a gate run,
and on this diff it selects 88 of the 143. kanso#1444's ran 2h26m, which is
88 rows at about 100 seconds apiece. Five landings are five of those sittings.
kanso#1444's own body made the same argument for the four it carried, and the
arithmetic has not changed.

**Eight conflicts, none of them blanket-resolved.** The log, the three
instruction goldens, `bench/welfare_floor.json` and `scripts/ratchet/
ratchet.kso` are all append-only, so both sides are kept and ordered. The
floor is merged entry by entry rather than by `max()` — 284 from the base,
four from the chain, two from the hole, baselines byte-identical — because
`max()` across a language change sets a floor the merged tree cannot reach,
which is written down at `c332f9c0`. The ratchet's fifteen new rows chain
through `rows_c1ya`, `rows_c1yb` and `rows_c1yc` where both sides had reached
for `rows_c1z`. `tests/golden/micro/a_build_writes_what_it_can_prove_was_born
.kso` takes the hole's side: the chain's only edit there dropped the bang from
`middle = ring[2]!`, and the hole deletes that line with `ring` and `chosen`
because a record an `if` chose and an element of a list are refused now and
live in the error corpus. `docs/compiler.html`'s five paragraphs differ only
in their `data-golden` spans and take the chain's, so `golden_prose` agrees
with the goldens beside them.

**A priced counter went unpriced the moment its neighbour landed.** The trend
gate refused this tree over three counters kanso#1449's own entry describes:
"the decoder's emitted code reads `calls` 1,209 -> 1,212, `branches` 795 ->
807, `lines` 9,161 -> 9,258". That sentence names each counter by its bare
suffix, and it satisfied the gate for as long as kanso#1444's entry sat beside
it in the same delta spelling `emitted_calls`, `emitted_branches` and
`emitted_lines` in full. kanso#1444 is on main now, its entry left the delta,
and the branch's added lines hold zero occurrences of any of the three names.
So a paragraph that prices a counter through a neighbour's spelling is priced
only until that neighbour lands, and nothing warns you: the gate was green on
kanso#1449 and is red here with the same words in the file.

The three land at `emitted_calls` 1,209 -> 1,212, `emitted_branches` 795 ->
807 and `emitted_lines` 9,161 -> 9,258, for the reason kanso#1449 gives: the
guard-bound fixes hand the beat back to loops the respell had cost it, and a
beat loop is more code than a call. `emitted_other_lines` 133,514 -> 136,463,
`emitted_other_calls` 20,231 -> 20,323, `emitted_other_branches` 12,689 ->
13,099 and `emitted_other_defines` 2,350 -> 2,342 are the same cause across
the thirteen benchmarks, and `text` 1,737,148 -> 1,755,372 is that code
arriving in the binary.

**What this round is expected to be red on.** The three compile instruction
goldens carry the chain's values and the merged tree is neither branch, so CI
measures them and round two writes them in. welfare reads those goldens, so it
is red with them, and the floor is left at 69.59152459326125 rather than set
from a projection: a floor banked before the goldens carry CI's rows freezes a
number this container guessed.

## Round two: CI's sitting on the merged tree, and three rows that fell

Round one came back red on exactly the three the body predicted and on nothing
else. The cost-goldens job's own vein summary — the authority, because its
nineteen counter steps are `continue-on-error` and the API's per-step
conclusions read SUCCESS either way — listed `compile instructions:failure`,
`entry instructions:failure`, `library instructions:failure` and `success` for
the other sixteen, `compile memory` and `compile allocations` among them.
Fifteen of the run's nineteen jobs were green, three were still running, and
welfare was skipped behind the goldens it reads.

The merged tree is neither branch, so CI measured the three fresh and all three
FELL:

| vein | the chain's golden | CI on the fold | |
| --- | --- | --- | --- |
| `compile_instructions` | 40,830,719 | 40,794,557 | −36,162 (−0.0886 per cent) |
| `entry_instructions` | 144,799,835 | 144,656,649 | −143,186 (−0.0989 per cent) |
| `library_instructions` | 145,482,390 | 145,339,594 | −142,796 (−0.0982 per cent) |

Summed, −322,144. Nothing in the fold set out to make the front end cheaper:
the tip carries the `!` respell, the build hole and the qualified-yield fix
together, and this is the layout under all three. `compile_allocs` holds at
27,395 and `compile_memory` is byte-identical, which is what says the fall is
layout rather than a pass doing less — an actual reduction in work would have
moved the allocation row with it. The three land at 40,794,557, 144,656,649
and 145,339,594.

The rows went in first, then `all_pages.sh --write` rewrote the eight
golden-quoting spans that name them (four `compile`, two `entry`, two
`library`), and only then was the floor banked: 69.59152459326125 ->
69.59317353129765. That order matters and is the rule — `--set` records
whatever score the committed goldens produce, so banking before they carry
CI's rows freezes a number this container projected rather than the one CI
measured. The rise is small and it is still a rise, and a gain nobody ratchets
is one the next change is free to spend.
## 2026-09-16 — the compile term never counted codegen, and the 2026-08-25 gavel says it must

Clay, told in the design chat that an optimizer taking twice as long is
invisible to welfare: "that was explicitly supposed to be one of the core
scalars going into the welfare function!!!!" He is right, and the archive
carries his ruling.

**The ruling it violates.** The archive's "gavel: welfare measures what
compiling costs, not what it counts" (2026-08-25). Told that the compile-speed
terms were `front_end_rounds`, `front_end_visits` and `emitted_lines`, his
words were "then you have a MASSIVE deficiency in your welfare metric. my
god." The ruling made the terms measured cost rather than proxy counts, with a
stated purpose: "so that making the compiler genuinely faster or leaner always
moves the score, and a 26% front-end improvement can never again land silent."

**How it went half-built.** The ruling named the two veins that already
existed, `bench/compile_instructions_golden.txt` and
`bench/compile_allocs_golden.txt`, and both measure `kanso check`. So the swap
from counts to costs happened and the question of WHICH compile was never
asked. Read off the gates on 2026-09-16, every compile counter the objective
weighs runs the front end and stops:

    compile_instructions   ./kanso check compile_corpus
    compile_allocs         ./kanso check compile_corpus
    compile_peak_bytes     ./kanso check compile_corpus
    entry_instructions     ./kanso check entry_corpus/main

`kanso check` stops before codegen, which CLAUDE.md already says in another
context. So the emitter, the `.ll` write and the clang invocation are outside
the objective entirely.

**What is unpriced, concretely.** `kanso build` writes the IR and shells out
to clang (src/main.rs:606): `dev_clang` runs `-O0`, `release_clang` runs
`-O3 -flto` with `-mllvm -inline-threshold=2000`, a threshold whose own
comment records that it was found by measuring a non-monotone ladder —
1250 read −0.9473%, 1500 read −0.8295%, 2000 read −2.0948%. That tuning is
real optimizer work whose cost the index cannot see. `bench/emitted_golden.txt`
counts what the emitter WROTE, "counted from jsonbench.ll before the linker
touches it", which is output rather than cost.

**The incentive this leaves.** Welfare pays for the optimizer's output,
through `run_instructions` on the run program, and charges nothing for the
optimizer's time. One-sided, with no budget, which is the shape that ends in a
compiler nobody wants to run. It is also the exact failure the 2026-08-25
gavel was called to end, one layer down: the front end can never land a silent
26% again, and the back end still can.

**The two tiers already exist.** `dev_clang` and `release_clang` are the fast
and rigorous paths of the tiering Clay raised in the same conversation. The
design question is not whether to build them; it is which one welfare prices
and which one the run terms are measured on. That is the ledger's new Blocking
entry, "What the compile term counts once codegen is in it".

**What this costs to fix.** Adding codegen to the term rebases
`compile_instructions` and `compile_allocs`, so the floor re-ratchets as a
model correction — the same mechanism the 2026-08-25 gavel itself specified:
"The floor re-ratchets from the rescored model in the same change, recorded as
a model correction." Weights and satiation stay as they are unless the new
numbers argue otherwise, which is a separate argument made about the weights.

**A correction to this session's own advice.** The chat told Clay an hour
earlier that the trade he wanted — longer compiles for a faster binary — was
"already free," and presented that as the model working. It is the defect,
described approvingly. The gradient it creates is real and so is the hole.

---

## 2026-09-16 — a log heading wrapped onto a second line, and the drift gate counted it twice

`scripts/page_drift` decides how far the log has run ahead of
docs/compiler.html by counting the lines a commit adds that begin `## `, and
fails past a budget of three. Its `heading?` is one line: `text/slice l 1 4 ==
"+## "`. Nothing else in the gate asks what a heading is.

kanso#1448 and kanso#1451 each landed a title hard-wrapped at the column limit,
and the wrap put `## ` at the start of the second line too:

    ## 2026-09-16 — A folder handed to `fold` is never called by name, and a
    ## walk that found no call site answered yes (DONE)

Markdown renders that as two h2s, the second a sentence fragment. The gate
reads it as two entries, so each of those pull requests spent two of the three
where it owed one, and any later branch inherits a budget already half gone.
Both are joined onto one line here; the words are unchanged.

**The property, and why it is not "a heading is short".** Non-dated `## `
sections are a real convention in this file — an entry carries `## the
measurement`, `## CI's sitting`, `## CORRECTION: ...` as its own sub-sections,
and fourteen of them stand in the live log. What separates one of those from a
wrap is what precedes it: a section heading follows a blank line, a wrap
follows the line it broke from. So the rule
`tests/a_log_heading_is_one_line.rs` reads is that no `## ` line is
immediately preceded by another `## ` line. Measured when it was written: two
violations in the live log, zero across the archive's 1,272 entries, so the
convention was already universal and only these two entries broke it. It reads
both files, so an archived wrap is caught too.

Watched red on the unfixed log, naming both pairs in the failure message, then
green on the join.

---

## 2026-09-16 — glibc's malloc was a seventh of the compile, and nothing had ever tried another allocator

Found by profiling `kanso check compile_corpus` on the explicit-box fold's tip.
Every row callgrind attributes to glibc's `malloc.c` and `arena.c` comes to
6,325,284 of the 41,698,193 instructions the process executes — 15.17%. That is
larger than any kanso function on the profile and larger than the three next
ones together: `HashMap::insert` 3.94%, `infer::eval_expr'2` 3.61%,
`check_after_infer` 2.81%.

The first draft of this entry said 12.69%, which was the four rows callgrind
prints above its default threshold — `_int_malloc`, `_int_free`, `malloc` and
`free`. Thirteen more sit under it, `malloc_consolidate` and `unlink_chunk`
among them, and they are the same allocator doing the same work.

The archive and the live log were searched for an allocator swap before this
was built. Neither carries one; the words mimalloc, jemalloc and
`#[global_allocator]` appear nowhere in either. The compiler has had a
`GlobalAlloc` of its own since the compile counters were minted, but it is a
tally over `std::alloc::System` and the allocator under it had never been the
subject.

**The change is where mimalloc goes, not that it is used.** It goes UNDER the
counting wrapper in src/main.rs rather than beside it, so `Counting::alloc`
still adds the layout's size to `ALLOC_BYTES`, still bumps `ALLOC_CALLS`, still
takes the running maximum into `PEAK_BYTES`, and only the call it forwards to
changes. Every counter the objective reads is therefore the compiler's own
demand and not the allocator's bookkeeping, which is what makes the measurement
below a clean one.

**The three rows, gate-shaped.** Measured with `env -i` and the pinned
`GLIBC_TUNABLES` the gates use, `kanso::main` inclusive, one build each on this
container:

    row                    glibc         mimalloc        delta
    compile_instructions    41,228,435    37,317,886    -3,910,549  -9.485%
    entry_instructions     146,066,668   133,330,370   -12,736,298  -8.720%
    library_instructions   146,749,935   133,478,166   -13,271,769  -9.044%
    summed                 334,045,038   304,126,422   -29,918,616  -8.956%

For scale, the last ten compile-side merges moved these rows between 0.13% and
1.14% each.

**Reproducibility is the disqualifying test, and it passes.** The 2026-09-05
ruling is one row, one value: a run that disagrees halts the vein rather than
being recorded beside it. An allocator with its own heuristics is exactly the
kind of thing that could make the row a distribution instead of a number, so it
was measured three times on one mimalloc binary before anything else was
decided: 37,317,886, 37,317,886, 37,317,886. Identical to the instruction.

**Nothing else moves.** `compile_allocs` 27,395, `compile_alloc_bytes`
4,612,036, `compile_peak_bytes` 787,956, `compile_passes` 7, `compile_rounds`
47, `compile_visits` 15,474 — byte-identical on both. The emitted LLVM IR is
byte-identical too: the same program built by both compilers gives
`45e0851958630978e6f23e6de2f57cf6` either way, so the `emitted_code` and
`machine_code` veins have nothing to see. `ldd` reads the same five entries, so
`compile_libraries` is unmoved: libmimalloc-sys compiles C into the binary and
links no shared object.

**What it costs.** Five packages join the lock file — mimalloc,
libmimalloc-sys, and cc with its own find-msvc-tools and shlex — and `cc`
compiles C at build time, so a C compiler becomes a build requirement rather
than only a run one. The dependency is gated to non-wasm targets the way
rustyline is; mimalloc does not build for wasm32 and the playground's cdylib
carries no global allocator of its own. The wasm blob builds RC=0 with the gate
in place.

Because nothing but the three instruction veins can see the swap, a revert
would leave every counter agreeing and only the goldens objecting. Row
`compiler_allocator` watches that, host-bound like its neighbours and proved
where its gate is green.

**Six megabytes of resident memory, and one test in the tree could see them.**
mimalloc reserves its first arena and commits it up front, so a process that
has allocated almost nothing is already holding about six megabytes the
compiler never asked for. Nothing in the objective can see that: the
`run_peak_bytes` term is the runtime's arena counters and `compile_peak_bytes`
is the tally in src/main.rs, and both are the program's own demand rather than
the operating system's number.

`tests/bind_chain_depth.rs` reads the operating system's number, through
`ru_maxrss` at `wait4`, and it is the only thing here that does.
`the_measurement_sees_a_shape_that_nests` went red on the swap. It is the
falsifier for its neighbour — a chain that runs as a loop stays flat, and that
claim is worth nothing unless the same instrument can see a shape that does not
— and it asserts that a nesting shape ten times deeper costs more than twice
the memory. A constant six megabytes sits under both of its readings and
squeezes the ratio toward one: 3.16 on glibc, 1.97 on mimalloc with its
defaults. The growth itself had barely moved. The shape at four hundred cost
1.84 MB above an empty program either way, and the shape at four thousand cost
17.1 MB on glibc against 14.9 MB on mimalloc.

So the option comes off. `mi_option_arena_eager_commit` set to zero returns the
floor to 5.69 MB against glibc's 5.23 MB, and costs 79,036 instructions of the
3,989,585 the swap saves — under two per cent of the win, and the table above
is the measurement with the option already off. The call has to reach mimalloc
before the first allocation does. From the top of `main` it is too late and
measures 26.4 MB at four thousand exactly as though it had never been made,
because Rust's runtime allocates on the way in; it runs from the process's own
constructor table instead.

The number it passes is copied out of the C library's header, because the Rust
bindings stop naming options well before this one. A copied number goes stale
in silence, so `tests/the_allocator_option_is_the_one_the_header_names.rs`
reads the header libmimalloc-sys vendors, counts the enum, and asserts the two
agree. Both specs were watched red under `an_allocator_option_off_by_one`,
which moves the constant to five: the header spec names the disagreement, and
bind_chain_depth reports 26,435,584 bytes against 13,451,264 — the arena back,
and the falsifier no longer falsifying.

**And the same swap is worth nothing at runtime, which is the arena's doing.**
The first question this raises is whether the run program wants it too, where
the objective weighs a fall far more heavily. It does not. On the same fold tip,
runbench's whole allocator bill is 6,023,245 of 1,840,366,983 instructions —
`_int_malloc` 2,455,724, `_int_free_merge_chunk` 883,298, `malloc` 864,125,
`_int_free` 760,003, `free` 465,319, `_int_free_maybe_consolidate` 349,529,
`unlink_chunk` 245,247. That is 0.3273%, against 15.17% on the compile side:
nearly forty times less, on a workload nine hundred times longer.

The reason is the thing the runtime was built around. A kanso program serves
its values out of the arena and reclaims them at the beat, so glibc sees the
arena's block requests and almost nothing else; the compiler has no arena and
asks libc for every String, every Vec and every table it grows. So this lever
is the compile side's alone, and it is large there precisely because that side
never got the design the other one did. Declined for the runtime by arithmetic
before building — mimalloc's own text would cost the `.text` vein more than
0.33% can return.

**And with the allocator out of the way, the profile's top is hashing.** On the
mimalloc build, `HashMap::insert` is 4.31%, `rustc_entry` 3.78% and
`reserve_rehash` 2.75% — 10.84% in hashbrown — with `__memcmp_avx2_movbe` a
further 3.03% underneath them, which is what comparing String keys costs when
two hashes collide. Not one function, so not one fix, and kanso#1033 already
declined the interned symbol for the AST's own field at 365 conversion sites.
The map keys are a different question from the AST field and nobody has
measured them. Open, and the largest thing left on the compile side.

## CI's sitting, and the floor

CI measured the three rows on the runner and they agree with this container
to within one per cent, in the same direction and slightly further:

    row                    golden         CI          delta       CI      container
    compile_instructions   40,794,557   36,886,838   -3,907,719  -9.579%   -9.485%
    entry_instructions    144,656,649  131,957,599  -12,699,050  -8.779%   -8.720%
    library_instructions  145,339,594  132,092,011  -13,247,583  -9.115%   -9.044%
    summed                330,790,800  300,936,448  -29,854,352  -9.025%

The cost-goldens job failed on exactly three steps and its own vein summary
names the same three. Everything else agreed: compile_allocs 27,395,
compile_memory byte-identical, the five compiler libraries unchanged, and
all fourteen runtime work rows, the emitted vein and the machine-code vein
untouched. The container's projection and CI's reading were the same
measurement on different silicon, which is the only claim this change needed
them to support.

Welfare 69.59 -> 69.75, banked with `--set` after the goldens carried CI's
rows rather than before. Eight page spans quoting the three goldens were
rewritten, and three sentences around them were rewritten by hand: a
narrative delta and a live span cannot sit in one clause, because the delta
is historical and the span is whatever the golden says today. The one that
had already gone wrong read "it reads X today, 1,056 lower" about a change
that predated two more.

## Round three: a reproduction failure, and the wall clock inside the allocator

CI measured the three compile rows twice on one commit and got two answers,
13 instructions apart on all three. The 2026-09-05 ruling is one row, one
value — a disagreement halts the vein and is neither keyed nor averaged — so
the round went to hunting it instead of to landing.

Ten runs in this container gave 37,317,886 every time, so whatever it is does
not vary run to run on one host. The CPU model, the glibc build and the rustc
build were identical on the two runners, so it is not silicon and not a
toolchain. And the same 13 on three compiles that differ in size by more than
three to one puts it in what the process does once, rather than in the work.

The profile then found something the hunt was not looking for. mimalloc
returns free pages to the operating system on a clock: each arena carries a
deadline, and a pass over one asks `_mi_clock_now` — glibc's `clock_gettime`,
through the vDSO — whether that deadline has gone by. On
`kanso check compile_corpus` it asks 163 times. **How many times it asks
depends on how long the process has been running, and that is wall time, not
work.** The whole purge machinery costs 8,288 instructions on the module row
against 44,608 on entry and 48,487 on library — five times as much for three
and a half times the work, which is the shape of a term keyed to elapsed time
rather than to anything the compiler did.

A deterministic vein cannot hold that. `mi_option_purge_delay = -1` disables
purging, takes `_mi_prim_clock_now` from 163 calls to 3, and removes those
same 8,288 / 44,608 / 48,487 instructions. The three that remain are the
stamp mimalloc takes when it initialises, which runs once whatever the timing.
The compiler is a short-lived process that exits and gives everything back at
once, so never purging removes work rather than adding it, and
`bind_chain_depth` still passes — not purging does not raise the resident
floor when nothing frees at scale. Reproduced three times at 37,309,598 /
133,285,762 / 133,429,679, identical each time.

**The timer does not explain the 13, and that thread stays open.** A
difference in how many deadlines expired would land on the long compiles and
not the short one, where the observed gap was the same on all three. So this
change removes a wall-clock dependence that was real and would have surfaced
eventually, and the original disagreement is still unexplained. If it returns,
the three remaining reads are where to look next. They are
`_mi_clock_start`'s calibration: it reads the clock twice to measure what a
read costs, then a third time for the process's start stamp, behind a
`mi_clock_diff == 0` guard that lets it happen once. So their count cannot
vary, and their cost is the host's vDSO — 33 instructions here, 11 a call.
A clocksource priced differently would shift all three rows by the same
small amount, which is the shape that was observed; it does not divide 13
by three, so that is a suspect rather than an answer.

The option index is pinned the same way the first one is.
`tests/the_allocator_option_is_the_one_the_header_names.rs` now carries a
table of (constant, option name) pairs and reads the enum position of each out
of the header libmimalloc-sys vendors. A second test splits the constructor,
collects every first argument to `mi_option_set`, and fails if the table does
not name it — because a list of pinned numbers that is allowed to be
incomplete pins nothing. Both watched red: the first under a new mutation,
`a_purge_delay_that_is_not_the_purge_delay`, which moves the constant to 16;
the second under a hand-added `show_errors` call, which it named as a number
copied out of the header and pinned by nothing.

The three compile goldens go back deliberately red this round. CI has to
re-measure them on the runner, because the container reads about one per cent
high and the whole point of the change is that these rows now hold still.

## Round four: CI's rows, and the container was wrong in the way that proves it

CI measured the three rows with purging off:

    row                    before          after         saved    container said
    compile_instructions   36,886,838   36,878,537       8,301          8,288
    entry_instructions    131,957,599  131,884,271      73,328         44,608
    library_instructions  132,092,011  132,025,154      66,857         48,487
    summed                300,936,448  300,787,962     148,486

The goldens carried a projection for one round: CI's earlier row minus the
purge machinery's cost as this container measured it. It was 13 out on the
module row, 28,720 out on entry and 18,370 out on library.

**That pattern is the argument.** A fixed per-process cost would have carried
across all three rows unchanged. What actually happened is that the short
compile agreed to within 13 and the two long ones saved sixty and thirty-eight
per cent more than projected, because the runner is slower under callgrind and
crosses more deadlines in the time it takes. The purge count is counted out by
elapsed time, and two hosts running the same binary reach different numbers.
That is the whole reason the option is off, and CI demonstrated it while
disagreeing with the projection rather than while agreeing with it.

Welfare 69.74755197041536 -> 69.74831917675371, banked after the goldens
carried CI's rows. The rise is small because 148,486 instructions out of
300,787,962 is 0.05% of one of five terms.

Everything else in the cost-goldens job agreed: compile_allocs 27,395,
compile_memory byte-identical, the five compiler libraries unchanged, all
fourteen runtime work rows, the emitted vein and the machine-code vein. The
job's own vein summary named exactly the three that were deliberately red and
nothing else.

**The 13-instruction disagreement is still open.** Nothing here explains it,
and the arithmetic that would have — three fixed init reads at 11 instructions
a call — does not divide 13. What this round establishes is narrower and worth
having on its own: the compile veins no longer contain a term that counts wall
time.

## Round five: the same source, two binaries, thirteen instructions

Round four's head went "behind" when kanso#1457 landed, so origin/main was
merged in to clear it. That merge carried `design/compiler-log.md` and
`design/log/compiler-log-archive.md` and nothing else. Neither is
`include_str!`'d into the compiler, so the bytes the build compiles are
identical, and all three compile rows came back exactly 13 higher.

`compile_instructions.sh` prints the binary's sha and the row together for
exactly this, and its two cases are settled differently: one sha counting two
rows halts the vein as a reproduction failure, two shas is an ordinary move
until the pair is built and both are read. The pair:

    round four   sha bd05a61f3a68   library row 132,025,154
    round five   sha 45b8072b4ea7   library row 132,025,167

Two shas. What the header's own account of this variance describes — a binary
whose data and bss differ starting the heap at a different break — is not what
happened here: `.text` 2,805,762, `.data` 12,672 and `.bss` 29,976 are
identical to the byte on both. Every frame callgrind prints above the ninety
per cent threshold is identical to the instruction, `mi_free` at 2,016,114 and
`mi_theap_malloc_aligned` at 1,971,116 with it, so the 13 is not inside any
function large enough for the profile to name. The block count is what moved:
27,583,359 against 27,583,361. Thirteen instructions in two basic blocks,
inside `main`, below every frame the profile prints.

The three rows are written to CI's round-five reading and the eight page spans
that quote them move with it. Welfare holds the floor at 69.75 — 39
instructions summed across a 168,762,834-instruction term is 2.3e-7, which the
score cannot see.

Recorded rather than explained, and this is the third reading rather than the
first: rounds three and four agreed on one value across two runs and round five
read another, so the row is stable for a given binary and moves between them.
Under glibc the same shape cost 508 instructions and the header carries seven
readings and four distinct values for it. At 13 it is 39 times smaller, which
is the one part of this that got better.

## 2026-09-16 — a binder pays for a lookup it never reads

`infer::arm_can_run` decides whether an arm's patterns could match a call's
arguments, from the literals alone. CI's library-corpus profile names its
closure at 4,023,145 instructions, 3.03% of that compile, and it is the only
frame in the top ten that is one expression rather than a pass.

The shape is the one this queue has shipped against before. For every
(pattern, argument) pair the closure computed the argument's string shape —
which asks `consts` for an `Ident` and walks every template part of a `Str` —
then its integer literal, then whether it was a literal at all, and only then
looked at the pattern. Four of the match's arms read one of those three each;
the fifth reads none of them. That fifth arm is `_ => true`, the binder, and a
binder matches whatever it is handed. Most patterns are binders, so most pairs
paid for a hash lookup and a walk and threw both answers away.

Each arm asks for what it reads now, and `str_shape` and `is_literal` are named
functions rather than expressions in the prelude. Both are pure — a `HashMap`
read and a match over the expression — so computing them later, or not at all,
cannot change the answer. The literal-dispatch rules the doc comment states are
untouched: a module constant bound to a string literal still counts as that
literal, and an interpolated string with fixed text in it still cannot match a
shorter one.

Gate-shaped on this container, `env -i` with the pinned tunables, `kanso::main`
inclusive, one build each:

    compile_instructions    37,309,598 ->  37,127,534    -182,064   -0.4880%
    entry_instructions     133,285,762 -> 132,067,512  -1,218,250   -0.9140%
    library_instructions   133,429,679 -> 132,213,543  -1,216,136   -0.9114%
    summed                 304,025,039 -> 301,408,589  -2,616,450   -0.8606%

The module row was read twice on two stagings and came back identical both
times. It also falls about half as hard as the other two, and the corpora are
why rather than the change: the entry corpus names ten imports and the library
corpus is the whole of `lib/`, where the module corpus names four, so the two
long rows walk far more declarations and far more calls into groups than the
short one does.

CONTAINER FIGURES, not CI's. This box reads about 1.2 per cent high on these
rows and the goldens are CI's; round one was deliberately red on all three and
CI's own reading is what the goldens carry:

    compile_instructions    36,878,550 ->  36,695,922    -182,628   -0.4952%
    entry_instructions     131,884,284 -> 130,618,857  -1,265,427   -0.9595%
    library_instructions   132,025,167 -> 130,762,703  -1,262,464   -0.9562%
    summed                 300,788,001 -> 298,077,482  -2,710,519   -0.9011%

The projection held on all three: the container said -0.4880%, -0.9140% and
-0.9114% where CI reads -0.4952%, -0.9595% and -0.9562%, so a box 1.2 per cent
high on the absolutes was within seven hundredths of a point on every
percentage. Welfare 69.75 -> 69.76, banked with the rows in.

Nothing else the compiler counts moves, and that was measured rather than
argued. `KANSO_COUNTERS=1` on both binaries off the same staging reads
`compile_alloc_bytes` 4,612,036, `compile_allocs` 27,395, `compile_peak_bytes`
787,956, `compile_passes` 7, `compile_rounds` 47 and `compile_visits` 15,474 on
each. The last two carry the correctness argument: an identical round count and
an identical visit count mean the fixpoint did the same work in the same order,
so inference reached the same answers. A reorder that had changed one would
have moved them.

No ratchet row. The compile goldens are already the objection to a revert —
put the three computations back in front of the match and the rows disagree by
the amounts above — which is how kanso#1382 through kanso#1387 shipped the same
kind of reordering, none of which minted a row either.
## 2026-09-16 — the compile side re-read once the allocator stops being the answer

kanso#1456 took glibc's malloc out of the compiler, and a profile that has been
read the same way for a month changes shape enough to be worth reading again.
This is that reading: the mimalloc binary with `arena_eager_commit` off,
`kanso check compile_corpus` under callgrind at gate settings, 37,911,833
instructions for the whole process. THE CONTAINER'S, not CI's — its rustc is
1.94.1 against the runner's 1.98.1 and its anchored row reads 37,317,886 where
CI reads 36,878,550, about 1.2 per cent high. Every figure below is a share of
one profile taken in one place, so the shares carry and the absolutes do not.

**The tables are now twice the allocator.** Every mimalloc symbol's self cost,
summed over the 150 of them the profile names, is 2,771,809 — 7.31%. hashbrown
comes to 5,415,908 across six rows: `insert` 1,634,418, `rustc_entry`
1,435,071, `reserve_rehash` 1,043,739, `get_mut` 463,841, `contains_key`
460,418, `get` 378,421. That is 14.29%, and `__memcmp_avx2_movbe` sits under it
at 1,147,976 (3.03%) comparing keys that missed. For the month the archive's
"the runtime is a minority of what a decode costs now" (2026-08-31) has been
right that "the front end's remaining 13.2% is malloc and free"; it is 7.31%
now, and the largest dimension on the compile side is the hash tables.

**And the tables are diffuse, on both corpora.** Attributed to the compiler
frame that owns each table, `insert` on the module corpus is 2,727,637
instructions over 15,097 calls from 40 callers, the largest `qualify` at 15.7%;
`rustc_entry` is 1,899,327 over 13,843 calls from 14, the largest
`check_merged_after_aliases` at 19.1%. The library corpus says the same thing
at three times the scale: `insert` 8,341,795 (6.28%) over 47,168 calls from 47
callers with `qualify` again the largest at 13.0%, `rustc_entry` 6,538,220
(4.92%) over 46,900 calls from 14 with `check_merged_after_aliases` at 18.9%.
The 2026-09-07 reading of the rehash family — twenty-odd owning sites, the
largest 0.68% — holds for the whole table family and for both corpora, so the
14.29% is a dimension rather than a change.

**The second-largest row is flat.** `infer::eval_expr'2` is 5,612,486
instructions on the library corpus, 4.23%, and the archive has only ever
carried it as a witness that two binaries agree — never as an attributed lead.
It is 117 self-instructions over 47,749 calls, 28,495 of them its own
recursion, with no callee above thirty per cent of what it hands out:
`try_fold` 1,662,080, `HashMap::get` 862,238, `widen_param` 677,454, `memcmp`
532,875, `Name as PartialEq<str>::eq` 382,974. A tree walk spending 117
instructions a node across every expression form is the shape `encode_onto`
turned out to have on the run side, and the answer is the same — there is no
block to remove. What the callee list does say is that 27,254 name comparisons
and 32,152 memcmps sit under one pass, which is the string-key theme again
rather than a lead of its own.

**What the compiler asks the allocator for.** 24,936 allocations, 23,241
deallocations, 1,052 reallocations — 49,229 calls. Inclusive they read
2,150,252, 576,072 and 524,026 instructions, which is 86, 25 and 498 apiece.
The per-call numbers are close to what an allocator costs; what is left on this
dimension is the number of calls.

**The pre-sizing seam is closed, and there is now a mechanism beside the
measurement.** "a set nobody read, and one that grew from empty" (2026-08-30)
measured the six filtered collects at 4,514 instructions and declined them.
"the runtime is a minority of what a decode costs now" (2026-08-31) called the
seam kanso#1158 opened exhausted, with `reserve_rehash` at 0.10% for its
largest named caller. "the front end is flat too, and one of its leads is an
artefact of the profiler's environment" (2026-09-07, fifth) priced the rehash
family at 4.22% over twenty-odd sites and declined it as twenty guesses at a
final size. All three readings stand. What none of them had was the reason so
little was there, and the split gives it: of the 8,543 `finish_grow` calls,
7,491 reach `__rust_alloc` and 1,052 reach `__rust_realloc`. Seven vector grows
in eight are that vector's first allocation rather than a doubling.
`with_capacity` replaces the grow path and keeps the allocation, so the most it
can reach is the bookkeeping — 611,974 instructions of self cost across
`finish_grow`, `grow_one` and `do_reserve_and_handle`, 1.61% — and only at a
site where the count is already in hand.

At the largest single growth site the arithmetic runs the other way.
`parser::P::parse_app` and its recursion twin own 314,363 instructions of grow,
0.83%, the biggest of the 41 callers. The argument vector is a `Vec::new()`
filled by pushing, and the match under it hands back the head unchanged when
the vector came out empty, so a call with no arguments allocates nothing at
all. A `with_capacity` there buys a grow in the minority case and pays an
allocation in the majority.

**The rehash reading, re-taken.** 1,912 rehashes at 1,540,385 instructions
inclusive is 806 apiece, and 87.9% of them are called from `insert` and
`rustc_entry` themselves rather than from a compiler frame — tables growing
during ordinary insertion. The 2026-09-07 count of twenty-odd owning sites is
unchanged by the allocator swap.

**What no entry in the log or the archive has proposed.** A bump arena for the
compiler. The runtime has had one since the beats landed and every kanso value
is served from it; the compiler asks libc for every String, every Vec and every
table it grows, one call at a time, and gives each back the same way. Searched
both files for `bump`, `bumpalo`, `allocator_api` and `arena`: 607 lines, and
every one of them is the runtime's arena, the beat, or a fixture that happens
to name a function `bump`. Neither `bumpalo` nor `allocator_api` appears. The
shape is the only one on this dimension that reaches the call count rather than
the per-call cost, and it is unsized: `alloc::vec::Vec` and `String` take a
custom allocator only behind the unstable `allocator_api`, so the question it
turns on is how many of the 49,229 calls belong to collections a phase-scoped
arena could own. That is not answered here. Recorded as an open lead with
nothing above it that sizes it.
## 2026-09-16 — the interpreter hashed against an attacker it does not have

Clay's gavel that morning ordered three counters for the interpreted engine and
named their order in his own words: "start-time is vastly more important than
speed which is more important than memory usage." Start-up was measured first
and is kanso#1461. This is the other two, and opening them found something.

**The corpus builds its own input.** `bench/interp_corpus` interpolates a
document of 220 objects and decodes it six times, so the workload is a property
of the corpus alone. Every other benchmark in this tree reads
`bench/large.json`, and a row that reads a file is a row that moves when the
file does.

**The anchor is the interpreter's own thread, not `kanso::main`.** `kanso run
--interp` pins a one-gigabyte stack and hands the program to a thread of its
own, so the main thread holds the front end and 1.8% of the run: 48,026,664
against 2,700,128,254 for the whole process on the first sitting.
`run_interpreted_on_stack` is that thread's entry, it is not recursive, and it
excludes the loader for the same reason the compile rows exclude it.

**The vein opened onto a reproduction failure.**

Two runs of one binary over one corpus read 2,651,460,189 and 2,648,375,305 —
3,084,884 apart, 0.116% — while the front end's own anchor read 48,026,664
twice in the same pair of runs. One row, one value is the 2026-09-05 ruling, so
that halts the vein and is hunted rather than keyed.

It took one grep. `src/eval.rs` declared the interpreter's tables with
`std::collections`: `fns` and `types`, the `knots` cell map, the typeset cache,
and the two cycle-guard sets under `values_equal` and `render`. `RandomState`
draws a fresh key from the OS on every process, so each run probes those tables
in a different order and does a different amount of work reaching the same
answer.

That is exactly the defect kanso#1449 cost three CI rounds, two published
corrections and a withdrawn escalation to find on the compile path.
`tests/the_compile_path_hashes_with_a_fixed_seed.rs` exists to stop it
recurring, and it EXCUSED this file, with this reason:

    the interpreter. No compile golden runs a program, and the interpreter's
    own cost is not counted by any exact vein.

Both halves were true when they were written and the second half is what this
change falsifies. The excuse is gone, `src/eval.rs` is spelled `crate::hash`,
and the spec covers the file that had the defect.

**It is a fall as well as a fix.**

Three consecutive runs read 2,375,580,224. Against the higher of the two
disagreeing readings that is 275,879,965 fewer instructions, a fall of 10.40%:
SipHash-1-3 was hashing every name the interpreter looked up, on a path where
the keys are the program's own identifiers and there is no adversary. The
argument `src/hash.rs` makes for the front end held for the interpreter the
whole time and nobody had made it.

The two memory rows read identically before the change and after it —
`interp_allocs` 5,313,431 and `interp_peak_bytes` 933,202 — which is the check
on what it touched. A probe sequence moves how much work a table does and not
how many bytes it asks for.

**What the veins are and are not.**

`bench/interp_instructions_golden.txt` and `bench/interp_memory_golden.txt` are
exact veins of their own and NOT objective terms, the way `.text` is under the
2026-09-05 ruling. The objective takes them when the model splits, which is
that gavel's own build.

The memory vein covers the front end and the interpreter together, on purpose:
the interpreted engine is a deployment rather than a stage of one, and what an
interpreted run costs includes deciding what to run. The instruction vein
excludes the front end, because it is the SPEED row and the front end has a
speed row of its own.

The rows recorded are this container's. It runs rustc 1.94.1 against the
runner's 1.98.1, so both gates refuse to compare here; round one was
deliberately red on both and CI's own reading is what stands.

**What CI read, and the one row that did not move between the hosts.**

    interp_instructions  container 2,375,580,224   CI 2,324,888,431   -2.18%
    interp_allocs        container     5,313,431   CI     5,313,431    0
    interp_peak_bytes    container       933,202   CI       933,202    0

The two memory rows came back EXACTLY as this container measured them, across
rustc 1.94.1 here and 1.98.1 there, while the instruction row beside them
diverged 2.18% between the same two hosts. That is worth writing down rather
than assuming: what a run ASKS the allocator for is the program's own shape,
and what it COSTS to ask is the toolchain's. The container reads about 1.2%
HIGH on the three compile rows, so the interpreted row's divergence is the same
sign and about twice the size, which is what an interpreted run being mostly
the interpreter's own loop would predict.

Three layout moves came with the change: compile_instructions -483,
entry_instructions -2,874, library_instructions -1,722. `kanso check` never
constructs an interpreter, so none of them is this change doing work
differently -- src/eval.rs IS the compiler, and editing it moves the compiler's
bytes and what sits around them. Three different magnitudes for one edit is the
signature of layout rather than of work. Welfare holds at 69.75.

The gate gains the in-job second reading kanso#1463 adds to the three compile
gates, for the reason that entry gives: the compile rows read 13 apart on two
runs of identical source that evening and the start-up row read 33 apart, and
a reader had to reconstruct which case that was by comparing job logs by hand.



## 2026-09-16 — the compile rows moved by thirteen and the job log could not say why

kanso#1459's two rounds carry identical compiler source. Round two changed the
three goldens, design/compiler-log.md, bench/welfare_floor.json and one page,
and nothing the compiler compiles. All three compile rows came back exactly 13
higher:

    compile_instructions    36,695,922 ->  36,695,935
    entry_instructions     130,618,857 -> 130,618,870
    library_instructions   130,762,703 -> 130,762,716

PROGRAM TOTALS moves by the same 13 and so does the `main` frame, so it is
inside the run rather than in the loader. The same 13 hit kanso#1460, whose
whole diff was a log entry, and a re-run of that commit came back on the
golden.

**What the gate printed, and why it was not enough.**

The gate has printed a binary sha and a silicon line on every run since the
last time this happened, precisely so a reader could settle case (1) against
case (2). Here is what the two rounds carry:

    round one   cpu="cpu family 0x19 model 0x1"    sha=c234bfc0577c   row=130762703
    round two   cpu="cpu family 0x19 model 0x11"   sha=770141d59043   row=130762716

Two variables and one observation. The runner's CPU MODEL moved, from AMD Zen 3
to Zen 4, and the BINARY'S SHA moved with it. Either could own the 13 and
nothing in either job separates them.

Two things are ruled out. Cargo is reproducible: three release builds of one
source on this container land on one sha, and three more with
`codegen-units = 1` land on one sha, so "the build is not deterministic" is a
hypothesis with no evidence under it. And a different glibc ifunc variant is
not it by size -- masking AVX-512 through `glibc.cpu.hwcaps` on this container
moves the library row by 393,285 instructions where the CI gap is 13. Thirteen
is a branch taken once per process on a CPU-feature test, not a different
memcpy.

**The fix is one more reading, and it costs nothing on a green run.**

The question "did this binary count two numbers, or did two binaries count one
each" is answerable inside the job that asks it. So each of the three compile
gates now counts a second time, on the same binary in the same box, and only
when the first reading disagreed with the golden. It prints both and then says
which case it is in its own words:

    library_again row=133327398 (the first reading was 133327398)
    ::error::THIS BINARY IS STABLE. A second count in this same job, on
    ::error::this same binary, read 133327398 -- the same number.

A run that is going to pass pays nothing. A run that is going to fail pays one
callgrind pass, about thirty seconds, and hands back the thing a reader has
twice had to reconstruct by comparing two job logs by hand.

This does not settle the 13. It makes the NEXT occurrence settle itself:
readings that agree inside one job put the difference outside the run, where
the sha and the silicon lines are, and readings that disagree are case (2) on
the spot.

**And the second reading goes into the artifact, not only into the log.** The
`*_got.txt` files are catted in one step at the end of the job, about eighty
lines from its tail; the callgrind output the error block sits under is several
hundred. Reading the first occurrence of this cost four fetches of whole job
logs to recover two sha lines and two cpu lines, and the `*_again` row would
have cost a fifth. A reader who has to fetch the whole job to learn whether the
binary was stable is a reader who will not bother, so `compile_again`,
`entry_again` and `library_again` are appended to the three `*_got.txt` files
and arrive with the rows they belong to.

**A third observation arrived while this was being written.** kanso#1463's own
first run read 36,878,537, 131,884,271 and 132,025,154 — the same exact −13 on
all three rows that kanso#1460 read, on a branch whose whole diff is gate
scripts, a log entry and a mutation. Three pull requests now, none of which
compiles differently from main, and the same thirteen.

## The second reading landed, and it is case (2)

The run after that one carried the new row into its artifact dump:

    library_instructions=132025619
    library_again=132025167

One binary, one corpus, one box, ONE JOB. Two callgrind runs minutes apart, 452
instructions apart, and the second landed exactly on the golden. On the same
run `compile instructions` and `entry instructions` both passed, so it is not
one fixed term per process either.

**So the compile vein does not reproduce on the runner**, and the 2026-09-05
ruling's case (2) applies: it halts the vein and is hunted rather than pinned.

**And the reading published for it a few hours earlier was wrong.** The CPU
model and the binary sha were put forward as the two candidates, on the
evidence of kanso#1459's two rounds, where both had moved together. Neither is
it. The same binary on one machine does not reproduce, which no comparison
across two job logs could ever have shown — and which is the whole argument for
reading it inside the job. Cargo's build reproducibility, three builds landing
on one sha, was never the question.

The container is why four rounds of cross-run comparison could not reach it.
Eight runs of the library gate's own command here, one binary, one box, one
sitting: `kanso::main` 133,335,824 and the whole process 133,939,674, EIGHT
TIMES, to the instruction. The object is now the difference between this
container and the runner, rather than the difference between two runners.

That also rules out per-process randomness as the cause, which was the first
guess worth having: a hasher seeded from the OS, or anything else drawn fresh
per process, would vary here too and does not. Two more are ruled out by the
corpus. `library_corpus` is a single FILE, so the loader's directory walk never
runs for it — and that walk sorts anyway. And the row is anchored at
`kanso::main`, which is inside `lang_start_internal`, so the `/proc/self/maps`
parse the 2026-09-15 ruling called external state is already outside it.

What the magnitudes say, across four veins: +6 on the interpreted run's 2.3
billion, ±13 on the compile rows' 131 million, +33 on start-up's 6 million, and
+452 on the library row in the sighting above. Small, not proportional to the
row, and not equal across rows — so neither a term that scales with the work
nor one fixed cost per process. It is a small number of instructions in
something whose iteration count moves slightly, and every red compile row from
here carries its own second reading to narrow it with.

## 2026-09-16 — a function named for an imported type, and the backend that could not find it

Seven lines, and `kanso check` says ok while the two engines disagree:

    import "std/json"

    pub play = print "{entry 1} {length (json/decode "[1]")}"

    fn entry i
      i + 1

The interpreter prints `2 1`, which is right. The native backend answers
`error: native backend: unknown type `<module>/entry``. Rename the function to
`row` and everything passes; drop the json import and everything passes. So the
trigger is a module declaring a function whose name one of its imports exports
as a TYPE — and that is a thing the language allows, because the two are
different namespaces and the checker has always said so.

**The chain, end to end.**

1. `enroll_bare` gives json's exported type `entry` a bare twin named `entry`.
2. `check::declared_names` returns ONE flat set holding both `program.types`
   names and `program.fns` names.
3. `qualify` builds its spelling map from that set, so this module's `fn entry`
   puts `entry -> <module>/entry` in it.
4. `rewrite_pattern` rewrites a `Pattern::Ctor`'s TYPE name through that same
   map, so the bare `entry` type becomes `<module>/entry`.
5. `codegen.rs`'s `emit_pattern` looks that up in `type_ids`, which holds
   `json/entry` and `entry` and not it, and returns the internal error.

Three other lookups share the map and the bug: `Pattern::Annotated`'s type,
`Expr::Upcast`'s target, and a typeset member inside `qualify` itself. Each is
a type position reading a map that also holds function names.

**The fix, and why it is one map rather than two.**

A constructor is CALLED by its type's name, so a VALUE position has to be able
to find a type in this map. What must not happen is the reverse. So the map's
value gains a flag — the spelling, and whether the name it replaces is a type —
and the four type positions require it while the one value position does not.
Two maps would have meant threading a second parameter through
`rewrite_pattern`, `rewrite_stmt`, `rewrite_scope` and `rewrite_expr` and their
thirty call sites; one flag changes the five lookups and nothing else.

**The spec.**

`tests/golden/micro/a_function_named_for_an_imported_type.kso`. The micro
corpus runs every fixture as a LIBRARY through the harness's generated entry,
which is the import path this bug lives on — `golden.rs`'s own comment says
"the library path is also where four separate qualification bugs lived, none of
which could fail a corpus that only ran files", and this is the fifth.

Watched red before it went green. With the type flag taken off the constructor
arm alone:

    a_function_named_for_an_imported_type answers differently as a library
      left: ""
     right: "2 1\n"

— the program produces nothing, because the backend refuses it, which is the
failure as a user meets it rather than a claim about a map.

**What it is not.**

It is not a design decision about whether a function may share a name with an
imported type. The checker already permits it and the interpreter already runs
it; the loader disagreed with both, and the native backend's way of saying so
was an internal error rather than a diagnostic. The differential law allows an
engine to REFUSE a feature with a clear diagnostic and forbids it to diverge
silently, and `unknown type <module>/entry` is neither clear nor a diagnostic.

**What it costs, measured on CI.** `compile_instructions` 36,878,550 →
36,900,512, `entry_instructions` 131,884,284 → 131,966,724,
`library_instructions` 132,025,167 → 132,070,594, and `compile_allocs`
27,395 → 27,397. That is one set of type names per dependency, built once
where the qualifier already walks the dependency's declarations. Welfare falls
0.00106 and the floor moves by exactly that: a name the language says means a
constructor has to mean one, which is the case CLAUDE.md rules needs no gavel.

**Round two's allocation row failed on a number that agreed.** CI measured
`compile_allocs=27397`, the golden said `compile_allocs=27397`, and the job
said `compile allocations disagrees with its golden`. `compile_allocs.sh`
strips its golden with `grep -v '^#'` and hands the result to `diff`, and the
note added after the value left a blank line between them — a line that
survives the strip and that the gate's output has no counterpart for. Nine
gates read a golden that way. `tests/a_golden_diffed_line_by_line_holds_no_blank_line.rs`
finds them off the scripts and refuses a golden that carries one.
## 2026-09-16 — the linearity analysis asked the whole program once per question

Clay's gavel that morning made development-loop cost its own welfare, and the
first thing measured under that heading was not the front end. Callgrind on
`kanso build` of a FORTY-LINE corpus, kanso's own process:

    kanso::build                           1,150,998,944   97.91%
    codegen::emit_ir                       1,144,148,711   97.32%
    linear::Analysis::new                    760,255,840   64.67%
    linear::Analysis::callers_hand_over      747,681,475   63.60%
    linear::Analysis::callsites_unique_in    623,488,195   53.04%

Two thirds of a build, in the pass that decides which `push` call sites own
their list. `kanso check` never runs it — linearity is a codegen-time analysis
— so not one of the three compile veins has ever seen a byte of it, and the
model that would is the one ruled this morning.

**The shape is the declares quadratic again.** `fixpoint()` loops; each round,
for every (name, arity, index) still believed linear it asks
`callers_hand_over`, which walks every function, every statement and every
expression in the program looking for calls to that one name. `escapes_as_value`
does a second full-program walk per (name, arity) per round.

`escapes_as_value` is exact in one pass, and that is what this change is.
`mentioned_as_value(e, name, arity)` was true exactly when some occurrence of
`Ident(name)` was not the head of an application of `arity` arguments. So the
question needs two facts per name: whether it ever occurs outside an
application head, and which argument counts it heads an application with. Both
are properties of the program alone — nothing the fixpoint does can change
either — so one walk before the fixpoint starts answers every ask.

    kanso::main             1,238,723,077 -> 1,065,298,291   -173,424,786   -14.00%
    linear::Analysis::new     760,290,312 ->   586,876,488   -173,413,824   -22.81%

The two falls agree to eleven thousand instructions, which is the check that
the win is where the reading said it was and not somewhere else.

**The emitted IR is byte-identical on all fourteen benchmarks**, runbench
included at 36,085 lines. The analysis feeds codegen, so that is the claim
worth making about a rewrite of it: what the compiler decides has not moved,
only what it spends deciding.

**And the walk it replaced is kept as the oracle.** `mentioned_as_value` is
`#[cfg(test)]` now rather than deleted, and a spec runs both it and the index
over lib/json and bench/compile_corpus for every (name, arity) pair
`escapes_as_value` can be handed — several thousand questions, both ways, on
programs this repository actually compiles. Watched red before it was trusted:
with `escapes` answering `bare.contains` alone, so that an application head
with the wrong argument count stopped escaping, it names the first
disagreement — "lib/json: the index and the walk disagree about
`Get_position` at arity 0".

**And then the larger half turned out to need no state at all.**
`callers_hand_over` walks every function looking for calls to one name, and
`callsites_unique` has exactly ONE `return false` of its own: the bad call site
for that name. Every other path recurses or falls through to
`child_exprs(..).all(..)`. So a declaration whose body never mentions the name
can only answer true, and walking it is the whole of the cost. The same walk
that built the escape index records, per name, which declarations mention it —
which is a property of the program and needs no round of the fixpoint — and
`callers_hand_over` iterates those and no others.

    kanso::main             1,238,723,077 ->   495,523,498   -743,199,579   -60.00%
    linear::Analysis::new     760,290,312 ->    17,129,979   -743,160,333   -97.75%

The pass that was two thirds of a build is 3.5% of what is left of one.
`Analysis::new` falls 44.4x and the build falls 2.5x, and the two absolute
falls agree to thirty-nine thousand instructions, which says again that the
whole of it is inside that pass.

The emitted IR is byte-identical on all fourteen benchmarks after both changes,
checked separately for each.

What is genuinely left is `callsites_unique_in` itself, which is still the
work that remains inside those few declarations, and `codegen::Backend::emit`,
which was 12.54% of the old build and is a much larger share of the new one.

**What CI read, and it is not what this change did.** The three compile rows
fell -13,771 (-0.0373%), -46,634 (-0.0354%) and -46,344 (-0.0351%). None of
them is this change doing less work on that path: `kanso check` does not run
the linearity analysis at all. `in_place_pushes` is called from `emit_ir`, and
from main.rs only behind `KANSO_BEAT_REPORT`, which no gate sets.

So it is the layout family, and a larger member of it than the seven before it.
The falls are proportional to the row rather than a fixed amount per process,
which rules out the maps parse; what they most likely are is
`mentioned_as_value` leaving the release build — it is `#[cfg(test)]` now — and
the generic instantiations linear.rs shares with the check path being inlined
differently without it. Recorded as unattributed rather than explained, which
is the honest state of it. Welfare holds at 69.75.
## 2026-09-16 — gavel: two welfares and a meta-welfare over them, and the floor re-ratchets

Clay ruled the ledger's "What the compile term counts once codegen is in it"
the same day it was filed, and ruled it by supplying the framework rather than
picking among the entry's options. The entry had offered his own proposal back
to him as an option 3 to choose; he corrected that: "what do you mean my call
in the shape of the fix. I just discussed with you a general framework for
updating the welfare metrics."

**The ruling.** The objective becomes three numbers.

    development welfare   the edit-test loop: front-end cost (`kanso check`,
                          which `kanso test` runs on every invocation),
                          dev-tier codegen (`-O0`), interpreter start-up,
                          interpreter speed, interpreter memory
    production welfare    the binary: native run instructions, native run
                          memory, release-tier codegen (`-O3 -flto`)
    meta-welfare          a function of the two, and the number CI gates on

His framing, verbatim: "if we're optimizing for production performance (CPU
and memory) and not compile performance, then compile performance
(speed/instructions and to some extent memory) becomes more like a very
dialed-down input to the overall welfare. then we have a separate welfare for
the interpreted version, where start-time is vastly more important than speed
which is more important than memory usage. of course sometimes these welfare
metrics themselves will conflict, so then you need something like a
'meta-welfare' which is a function of both, because sometimes it will make
sense to do a change which makes development speed much better in exchange for
a very small production performance cost, or vice versa."

**Why the split, and not a re-weighting.** Interpreter start-up is paid on
every test run and never once in production. The same microsecond is enormous
in one context and free in the other, and no single scalar can hold both
readings of it. That dimension is unexpressible in today's model and is the
reason the split earns its cost.

**One correction to the proposal as stated, and it was made in the chat before
the ruling.** Compile cost does not dial DOWN, it MOVES. `kanso test` runs the
front end on every invocation, so front-end cost sits beside interpreter
start-up as a first-class DEVELOPMENT term. Production welfare carries codegen
rather than checking.

**What it closes.** The entry asked which of the two clang tiers welfare
should price. Under two welfares there is nothing to pick: dev-tier codegen is
a development term, release-tier codegen is a production term, both counted
where their cost is paid.

**Three things the entry raised that were never Clay's to settle, and are
recorded here as consequences rather than decisions.**

- **One floor, on the meta.** This follows from the standing rule that the sum
  is the objective and the terms are diagnostics. Ratcheting the sub-scores
  separately would re-enable the part-against-whole optimisation that rule
  exists to stop. Filing it as an open question was the chat's error.
- **Whether the meta layer saturates is the implementer's.** The 2026-08-25
  gavel already says so in its own words: "Weights and satiation for the
  measured terms are the holder's to price from evidence... that is
  implementation under the ledger's own charter, and it does not come back
  here." Noted for the pricing: `a·W_prod + b·W_dev` with a linear meta is
  algebraically one flat term list, so a saturating meta is what makes the
  composition more than arithmetic — a sub-welfare near its ceiling then earns
  little from further wins, which is how the model says "the interpreter is
  fast enough now."
- **Start-up reuses count-from-`main`.** Interpreter start-up is the stretch of
  execution normalised out of the compile row on 2026-09-15; measuring it is
  not a contradiction, since noise inside one measurement is the object of
  another. The counter counts kanso's own start-up work and normalises the
  loader's, which is the machinery kanso#1439 shipped.

**THE FLOOR RE-RATCHETS.** Clay: "yeah you've got to re-ratchet." The
changeover is a model correction, not a rebase recorded and left: the meta
floor is set from the rescored model in the same change, exactly as the
2026-08-25 gavel specified for the last model correction. No change rides
across the changeover holding a score it earned under the old model.

**What cloud builds.** The counters for the development side, which do not
exist yet — interpreter start-up, interpreter speed, interpreter memory, and
dev-tier codegen cost — and release-tier codegen cost on the production side.
`bench/objective_sources.txt` gains every one of them in the same commit that
adds them, with `tests/the_objective_reads_what_the_gate_watches.rs` replaying
the file, because this model's PROSE has gone stale twice while the file never
did. Weights and satiations priced from evidence. The entry leaves the ledger
with this commit and STATUS.md carries the build.

## 2026-09-17 — where two readings part, function by function

The three compile rows each read one figure out of a callgrind profile:
`kanso::main` inclusive. When two readings of one binary on one machine
disagree, that figure says how much and nothing about where. Every hunt
through 2026-09-16 had to guess from the size of the move, and the guesses
have been wrong twice: the runner's CPU model and the binary's sha were both
published as the cause of the 13 and neither was.

`scripts/gates/profile_diff.sh` totals each function's SELF cost in two
profiles, joins on the name, and prints every function that moved. The three
gates call it in the branch that has already established case (2) — the same
binary counting two numbers in one job — where the two profiles are still on
disk and nothing else in the job can say which frame carries the difference.

**The profile is parsed here rather than through `callgrind_annotate`, and
that is not a preference.** `--threshold` is a percentage of the total and 100
is its maximum, so the tool stops as soon as the running percentage rounds to
100. On a profile whose hot function is 99.999% of it, the entire tail is
dropped — and the tail is this instrument's whole subject, because thirteen
instructions in a hundred and thirty-two million live nowhere else. The first
draft read the annotated table and reported two profiles differing by exactly
that as identical. `tests/two_readings_that_part_name_the_frame.rs` is built
from a frame one ten-thousandth of its profile for that reason, and it was
watched red against the first draft before the parser replaced it.

**Two real library profiles on this container agree function by function.**
Eight runs had already read 133,335,824 identically; this is the same fact at
a far finer resolution, and it says the container is not where the flutter
lives. The parser's self-cost sum matches `callgrind_annotate`'s PROGRAM
TOTALS exactly on a real profile, which is the check that it reads the format
rather than something near it.

**The temp files are named for the process.** Three gates diff their own pair
and the spec runs two comparisons at once; a fixed path had one of them
reading the other's answer, which is how the second spec first went red.

**The two CI runs that differ by 13 differ in one thing.** kanso#1464's rounds
carried identical compiler source — round two changed goldens, the log, the
floor and one page. Same glibc 2.39-0ubuntu8.9, same rustc 1.98.1, identical
`.text=2799906 .data=12672 .bss=29976`, and all three compile rows exactly 13
lower in round two. Identical section sizes rule out layout. The binary sha
differs because mimalloc's `options.c` prints a banner built from `__DATE__`
and `__TIME__` — `libmimalloc-sys` passes `-Wno-error=date-time` for it — and
those are fixed-length strings, so every offset in the binary is unmoved. What
is left is the runner: AMD family 0x19 model 0x11 in round one, model 0x1 in
round two.

**A single CPU feature bit moves the row by single digits.** On this container,
`GLIBC_TUNABLES` `hwcaps=-AVX2_Usable` moves the library row by exactly +2,
from 133,429,679 to 133,429,681. Larger masks move it by a great deal —
`-ERMS` by −1.86M, `-AVX_Fast_Unaligned_Load` by −395k — and that is routine
selection. The +2 is a per-process constant of the shape the 13 has.

**Eager binding is not the lever, and the control says why.** `LD_BIND_NOW=1`
raises the row by 2,511, and so does `XX_BIND_NOW=1`, which means nothing to
the loader: the whole move is one more entry in the environment, not the
binding mode. The row costs 2,511 instructions per environment variable
regardless of that variable's length — 1, 2, 3, 4 and 8 characters all read
identically. The gates run under `env -i` with two variables, so this is
normalised on CI already; it is recorded because it is the same class of thing
and it was very nearly published as a finding about the loader.

**The profiles now leave the job.** The comparison left is between two runs on
two runners, which no single job can make. The cost-goldens job uploads the
three profiles it counted, and the host's CPU family, model, stepping, glibc,
rustc and binary sha beside them, so `profile_diff.sh` can be run across a
model 0x1 sitting and a model 0x11 one and name the frame that carries the 13.



## 2026-09-16 — the first development counter measured, and it found a quadratic

Clay's gavel that morning made interpreter start-up a first-class term: it is
paid on every `kanso test` invocation and never once in production, and no
single scalar can hold both readings of the same microsecond. The entry says
the development counters do not exist. This is the first of them measured, and
measuring it was enough.

`kanso play` on a file holding one `print "x"` cost **69,207,585 instructions**
at the `kanso::main` anchor, 70,258,769 for the whole process. That is nearly
twice what the whole module corpus costs to check, to print one line.

**Three quarters of it was substring search.** `memchr_aligned` 30.88%,
`<&str as Pattern>::is_contained_in` 24.76%, `CharSearcher::next_match`
18.06%. Followed through an inlined closure to a `Vec::from_iter` making 1,204
closure calls, and from there to one owner: `codegen::Backend::emit`, three
calls, 65,049,261 instructions, 21,683,087 apiece — 92.6% of start-up in one
expression.

The expression is the `declares` filter. `DECLARES` holds 1,187 lines: 163
`declare` lines and 1,024 lines of inline helper body. The filter walks all of
them and asks `referenced(sym)` for the 163, and `referenced` built a probe
with `format!` and then searched the emitted body, the call twins, and — the
quadratic — `DECLARES.lines().filter(..).any(|l| l.contains(&probe))`, which
re-split `DECLARES` and re-scanned its 1,024 helper lines.

**The `||` is why the shape of the program decides the cost.** A symbol the
body actually calls answers on the first clause and never reaches the third.
Counted on the two ends of the range: the one-line program references 33 of the
163, so 130 fall through and re-scan 1,024 lines apiece — about 133,000 line
scans; runbench references 82, so 81 fall through. The quadratic bites hardest
on the program that uses the least, which is the program `kanso test` compiles
over and over.

Two changes, measured one at a time.

**The helper text is built once.** Joining the non-declare lines into one
string ahead of the filter is exactly equivalent: the probe is `@sym(`, which
holds no newline, so no probe can match across a join made with one.
69,207,585 -> 8,460,712, a fall of 87.8%.

**And then the haystacks are read once rather than per candidate.** The
question asked of each text is whether `@sym(` appears in it, and the set of
symbols satisfying that can be read off in a single pass: every `@` begins a
name and the next `(` ends it. That is the same answer for the same reason the
`DECLARES` parser below it already reads a declared name as the span between
those two characters — a symbol holds no `(`, so the first one after an `@` is
exactly where the name stops. `referenced` becomes a set lookup.
8,460,712 -> 4,934,340.

    kanso::main     69,207,585 -> 4,934,340   -64,273,245   -92.87%
    whole process   70,258,769 -> 5,991,364

Fourteen times.

**What it does not do is make a big build faster, and the reason is worth
having.** `kanso build bench/runbench` reads 10.1 seconds before and after. Its
wall clock belongs to clang — `clang -O3 -c` on the emitted 1.25 MB of IR is
3.2 seconds on its own — and against that the front end is 0.057 seconds and
the emitter's saving disappears into the noise. The row this change moves is
kanso's own work on a program small enough for that work to be the whole of it.
Wall time on the one-liner moves with it but by much less than the instruction
count does, 0.0496s to about 0.040s, for the same reason: `kanso play` still spawns
clang and links.

The emitted IR is byte-identical across the pair on
the largest program in the tree — runbench, 36,085 lines, md5
`ebd24064f1a55e8effeb3ed5f08d4cf4` before and after — so nothing about what the
compiler produces has changed, only what it spends deciding it.

**Nothing in the tree could see this.** `compile_instructions` and its two
neighbours stop before codegen, and `emitted_code` and `machine_code` read
output that did not move. A win of this size with no golden against it is a win
the next change is free to give back, which is the ironclad rule's whole
subject — so the counter comes with it rather than after it, and the gavel
orders that counter anyway.

**The vein.** `bench/startup_instructions_golden.txt`, read by
`scripts/gates/startup_instructions.sh`, counting `kanso play` on
`bench/startup_corpus/main.kso` — one `print` — at the `kanso::main` anchor
with the same emptied environment and pinned tunables the three compile rows
use. It is the fourth callgrind row and the only one that reaches codegen. Its
own sitting on this container, one box and one corpus, the two binaries
differing in nothing else:

    base    69,183,407
    fixed    4,919,980
    delta  -64,263,427   -92.89%   14.06x

It is an exact vein of its own and NOT an objective term, the way `.text` is
pinned under the 2026-09-05 ruling. The objective takes it when the model
splits, which is the gavel's own build and not this one.

**CI's own first sitting is 6,018,394, and the container is 22.3% BELOW it.**
That is the opposite sign and twenty times the size of the 1.2% this box reads
HIGH on the three compile rows, and the row's own composition is why: start-up
is the loader, the prelude and the backend deciding what to emit for one
`print`, and the loader's share belongs to the runner's glibc and rustc rather
than to anything the compiler does. The compile rows are dominated by the
compiler's own passes and this one is not, so the projection that holds for
them does not hold here. CI's is the number the golden carries.

**The row reproduces.** Four runs of one binary on this container all read
4,919,980, which is the question the gate's case (2) asks and the reason to ask
it before pinning anything. The reproduction matters more here than on the
compile rows, because `kanso play` SPAWNS -- a clang feature probe, clang
itself and the linker -- and a process that forks is a process whose own work
could vary with what it forks into. Measured against that worry: with
`--trace-children`, the three children of a `kanso build` count byte-identically
across two runs (clang -cc1 532,991,920, the clang driver 31,648,129, ld
87,120,997) while kanso's own process varies by 480. The anchor sits inside
`kanso::main`, and four sittings say the variance does not reach it.

**AND THEN CI COUNTED IT TWICE.** Round two wrote 6,018,394 in and read
6,018,427 on identical compiler source -- round two's whole diff was goldens,
the log, the floor and one page -- a move of 33. The three compile rows beside
it agreed exactly across the same pair, so it is this row alone. kanso#1459's
two rounds did the same thing with 13 on all three compile rows, and the same
13 hit kanso#1460, whose whole diff was a log entry.

Thirty-three has an obvious suspect and it is wrong. `clock_gettime` costs
exactly 33 instructions in this profile -- three calls at 11 apiece, which is
`_mi_clock_start`'s calibration, the three reads the purge-delay setting left
behind and which src/main.rs's own comment names as a term whose cost is the
host's vDSO. Walking the profile's call graph settles it: the chain is `(below
main)` -> `_mi_auto_process_init` -> `mi_process_attach` -> `mi_process_init`
-> `_mi_stats_init` -> `_mi_clock_start`, and `kanso::main` is not an ancestor
of any of it. mimalloc calibrates from `.init_array`, before main, so those
three reads are already outside the anchor and cannot be what moved.

So the 33 is unexplained, like the 13, and both are outside the diff. What this
round adds is the machinery to settle the next one: the gate now counts a
SECOND time on the same binary in the same job when the first reading
disagrees, and says in its own error text whether the binary is stable or
counted two numbers. kanso#1463 does the same for the three compile gates.

**One more measurement, on a real build rather than a one-liner.** Gate-shaped
with `--trace-children`, five processes counted, on a forty-line corpus that
imports std/list and std/text:

    kanso build, whole process tree, main         1,834,946,532
    kanso build, whole process tree, this branch  1,765,595,173
                                                    -69,351,359   -3.78%

So the declares fix is worth 69.35 million instructions on a build that spawns
clang and links, not only on the one-line program the vein counts. kanso's own
share of that tree is 1,238,699,420 on main and 1,169,347,956 here.

**Three layout moves came with the change**, and they are small and do not
move together: `compile_instructions` -194 (-0.0005%), `entry_instructions`
-346 (-0.0003%), `library_instructions` -18 (-0.00001%). `kanso check` stops
before codegen, so none of them is this change doing work differently; they are
the move CLAUDE.md's note on that row describes, because src/codegen.rs IS the
compiler and editing it moves the compiler's bytes and what sits around them.
Three different magnitudes on three rows is the signature of layout rather than
of work. Welfare holds at 69.75.

Four things had to move with it, and three were found by specs rather than by
hand, which is the point of them. `scripts/gates/library_box.sh` stages the new
corpus. `scripts/trend_gate/trend_gate.kso` names the golden, because
`tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads `bench/` off
disk and keys on the word `golden` — it went red the moment the file existed.
`scripts/ratchet/ratchet.kso`'s `host_bound` list gains the gate, because
`tests/a_host_bound_gate_is_reported_not_credited.rs` asserts that list and the
gates running callgrind are the same set, and it named the omission exactly:
"a callgrind gate left off the list makes a runner mismatch fail the whole
job". And `.github/workflows/ci.yml` gains the step and its entry in the vein
summary, which is the block that actually fails the job.
## 2026-09-17 — the allocator was guessing at addresses, and the row was paying for it

The three compile rows have disagreed with their goldens by thirteen
instructions across runs of identical source since the compiler moved to
mimalloc on 2026-09-15, and the 2026-09-05 ruling halts a vein that counts two
numbers for one row. Two published diagnoses were wrong: the runner's CPU
model and the binary's sha, which were confounded with each other on the only
evidence available at the time.

**The instrument that settled it prints where two profiles part.**
kanso#1463 made each compile gate take a second reading inside the job when
its row fails, and `scripts/gates/profile_diff.sh` totals every function's
self cost in both profiles and lists the ones that moved. On the first run
that carried it, the entry row read 131,884,793 and then 131,884,271 — one
binary, one corpus, one job, 522 apart — and all sixteen functions that moved
were mimalloc's OS-allocation path: `mi_page_map_set_range_prim`,
`mi_os_prim_alloc_at`, `_mi_prim_alloc`, `_mi_os_alloc`, `_mi_os_zalloc`,
`mmap`, `prctl`, `_mi_os_get_aligned_hint`, the stat counters and the mutex
around them.

**mimalloc's own source says why.** `v3/src/os.c`:

```c
#if (MI_SECURE>=1 || defined(NDEBUG))  // security: randomize start of aligned allocations
    const uintptr_t r = _mi_theap_random_next(theap);
    init = init + ((MI_HINT_ALIGN * ((r>>17) & 0xFFFFF)) % MI_HINT_AREA);
```

A release build defines `NDEBUG`, so every process draws a 4 MiB-aligned base
out of a 4 TiB window from per-process entropy and hands it to `mmap` as a
hint. The page map commits its entries by address, so the same allocation
costs a different number of instructions depending on where it lands. Three
runs on one container, one binary, one corpus and one environment, watched
with `--trace-syscalls`, asked the kernel for `0x48e11400000`,
`0x52844800000` and `0x38240c00000`.

**So it is normalised rather than explained.** `MI_NO_ALIGNED_HINT` is
mimalloc's switch for exactly this: the function then always returns NULL and
the OS chooses, which under valgrind's address-space manager is the same
address every run. `.cargo/config.toml` defines it through `CFLAGS`, because
libmimalloc-sys exposes no feature for it and the `cc` crate appends `CFLAGS`
to its own flags. `mi_option_max_vabits` looked like a runtime lever and is
not one: it sizes the page map and never reaches the
`mi_os_mem_config.virtual_address_bits` the hint reads.

**One mutation had to be rewritten, and it said so in advance.** `a_library_the_row_cannot_see.sh` writes a `.cargo/config.toml` carrying
`-C prefer-dynamic`, so the compiler grows a shared object the instruction row
cannot see. Its own comment anticipated this: "a repo that grows its own cargo
config has somewhere for this flag to be lost, so the mutation stops rather
than appending into it." It appends now, and refuses only if a `[build]`
section is already there to collide with.

**It costs nothing measurable.** The library row read 133,429,679 with the
hint and 133,429,679 without it, and three runs without it agree function by
function. `tests/the_allocator_does_not_guess_at_addresses.rs` reads
mimalloc's source and goes red if a crate bump renames the switch or a config
edit drops it.

**Round two: the three rows read thirteen lower, and always did.** CI on the
no-hint binary reads `compile_instructions` 36,878,537, `entry_instructions`
131,884,271 and `library_instructions` 132,025,154 — each thirteen below its
golden. The change did not move them: the run before this one, with the hint
still on, read 36,878,537 and 132,025,154 for two of the three. The goldens
were written from a sitting that drew an unlucky address and have been
thirteen high since. The entry row is where it shows plainly: with the hint on
it read 131,884,793 and then 131,884,271 inside one job, and with the hint off
it reads 131,884,271 and nothing else.

**Round three, after kanso#1466.** CI on the merged head reads
`compile_instructions` 36,695,922, `entry_instructions` 130,618,857 and
`library_instructions` 130,762,703 — the figures this branch had measured
before, to the instruction. The merge brought main's values in and this writes
the branch's back. Two runs agreeing is what the allocator fix bought.

**Round three, after kanso#1466: the interp row is on its golden.** The
interpreted run had read 2,324,888,437 against a golden of 2,324,888,431 on
every other run; on the merged head it reads 2,324,888,431 and CI says
`interp_instructions: 2324888431, on the row`. Both memory rows are
byte-exact. The six instructions were the allocator picking a random base
address, which is the same fault the three compile rows had at thirteen.

The three compile rows come back at `compile_instructions` 36,878,080,
`entry_instructions` 131,881,423 and `library_instructions` 132,023,458,
against main's 36,878,537, 131,884,271 and 132,025,154. That fall is the
fixed-seed hashing this branch puts on the compile path.

`compile_instructions` 36,900,512, `entry_instructions` 131,966,724 and
`library_instructions` 132,070,594 — to the instruction, the figures round two
measured and the next run then disagreed with by thirteen. The merge brought
main's values in and this writes the branch's back. `compile_allocs` reads
27,397 and agrees, now that the note sits above the value rather than after it.

**Round three: the start-up row was counting a cold cache.** One binary, one
job, read 6,018,427 and then 4,869,632 — 1,148,795 apart, a fifth of the row.
`kanso play` takes the native path, so the first process writes
`kanso_runtime_<profile>_<key>.o` and `kanso_run_<key>` into the temp
directory and every process after it reuses them. The gate now runs the
program once before it counts, which puts the cache in the state every reading
after the first would have seen. That is the 2026-09-15 rule applied to a
cache rather than to a clock: put the external state into a known state, do
not explain it afterwards.

The three compile rows come back on the merged head at `compile_instructions`
36,878,356, `entry_instructions` 131,883,938 and `library_instructions`
132,025,149, against main's 36,878,537, 131,884,271 and 132,025,154.
**Round three, after kanso#1466: the same three rows, read again and agreeing.**
CI on the merged head reads `compile_instructions` 36,864,779,
`entry_instructions` 131,837,650 and `library_instructions` 131,978,823 — to
the instruction, the figures round two took from CI and that the next run
disagreed with by thirteen. The merge brought main's values in and this writes
the branch's back. It is the first time this vein has reproduced across two
runs since the compiler moved to mimalloc, which is what kanso#1466 was for.

**Round four, after kanso#1464.** CI reads `compile_instructions` 36,682,232,
`entry_instructions` 130,573,787 and `library_instructions` 130,716,747 against
main's 36,864,779, 131,837,650 and 131,978,823. Asking the pattern before the
binder is what this branch's share of that is; the rest of the move against
round three is kanso#1464 arriving underneath it.

## 2026-09-17 — the thirteen is not compiler work

**All three compile rows move by the same thirteen.** DONE. The module, entry
and library rows have disagreed with their goldens by exactly thirteen
instructions across several sittings, and until now each was read on its own.
Put side by side on kanso#1463 they read 36,864,766 against 36,864,779,
131,837,637 against 131,837,650, and 131,978,810 against 131,978,823. Three
routes through the compiler, one of them 3.6 times the size of another, each
off by thirteen. A term that costs the same thirteen on a 36.9-million-
instruction compile and a 132.0-million-instruction compile does not scale with
the input, so it is not the compiling. Every account that put the thirteen in
the front end is dead: the lexer, inference, the emitter and a layout effect on
hot code all grow with the source, and this does not.

**The two sittings of one commit agreed.** DONE. kanso#1463's job was re-run on
its own head to try for two profiles differing in nothing but the run. Both
attempts read the same three numbers. So the row reproduces within a commit
since kanso#1466 took mimalloc's randomised base address out, and the thirteen
separates a sitting from the sitting the golden was taken on.

**The same tree, twice, thirteen apart.** DONE. kanso#1462 was green on
`fc3305f8`. Its branch was updated — protection wants the checks on an
up-to-date head — and the identical work came back thirteen out on all three
rows: 36,862,804 to 36,862,817, 131,830,523 to 131,830,536, 131,972,417 to
131,972,430. What the update brought in was `hooks/post-merge`,
`scripts/install_hooks.sh` and one test file, 132 lines, none of them compiled
into the binary, `include_str!`'d, or read by a compile gate, with no golden
moving. Same compiler, same corpus, same goldens, measured twice. This is the
experiment the re-run above was trying to manufacture, and it arrived on its
own.

**Two re-runs of one head agreed, which the coin-flip reading does not
predict.** OPEN. kanso#1462's failed jobs were re-run on `b8327112` and read
the same three numbers again, +13 from its own earlier green sitting of
`fc3305f8`. Nothing compiled into the binary differs between those two trees:
there is no build.rs, and every `include_str!` in src/lib.rs names a file
under lib/. So the value is a function of something that holds across two
separately-allocated runners of one head and changes between two heads whose
compiled input is identical. Two runs agreeing is a one-in-two event and
proves nothing on its own, but it is enough to stop calling this a per-run
flip until a sitting says otherwise. The host facts now printed beside the
floor -- the kernel release and version, which no job has ever printed --
are there because a per-host term is what this shape looks like and the CPU
model has already been refuted.

**Where it can be.** DONE. The three profiles the job already writes name the
candidates by themselves: 588 frames carry the same self cost across all three
workloads, 556,052 instructions in all. Restricted to frames the row can see —
reachable from `kanso::main`, which is the figure the gates read — 38 remain,
51,269 instructions, and not one of them is compiler work. The largest block is
mimalloc's scan of the environment for its own options, 49,449 instructions.
`getauxval`, called twice from std's stack-overflow handler at 146 each, the
`sbrk`/`brk`/`__glibc_morecore` trio, `sigaltstack`, the argv walk and the
stdout flush make up the rest. A near-empty compile carries 36 of the 38 at
byte-identical cost, which is what a per-process term looks like.

**What the environment actually is.** DONE. The gates run under
`env -i PATH=... GLIBC_TUNABLES=...` and believe they have pinned it. The child
sees seven variables: valgrind adds `LD_PRELOAD`, `LD_LIBRARY_PATH`,
`GLIBCPP_FORCE_NEW`, `GLIBCXX_FORCE_NEW` and `PWD` on top of the two. On one
runner image those are fixed, so this is not shown to be the thirteen — it is a
normalisation the gate claims and does not have, and mimalloc's scan of it is
the single largest per-process term inside the row.

**Ruled out by measurement.** DONE. Visible CPU count does not move the row:
four runs of `kanso check compile_corpus` on this box, bare, under `taskset -c
0` and under `taskset -c 0,1`, all read 37,285,436. The runner's CPU model was
refuted earlier by two sittings on different models reading the same number and
two on the same model reading different ones.

**The instrument.** DONE. `scripts/gates/per_process_floor.sh` prints that
floor — the frames whose self cost held across every workload given, and their
sum — derived from the profiles rather than from a list, so a frame that
appears or disappears is reported. It runs in the cost-goldens job, gates
nothing and pins nothing. Two jobs whose `per_process_floor=` lines differ by
thirteen name the frame between them, which turns a hunt nobody can reproduce
on demand into a comparison of two job logs. `scripts/gates/callgrind_self.sh`
is the reader under it: `callgrind_annotate`'s `--threshold` is a percentage
and stops once the running total rounds to the figure asked for, so the tail it
drops is where a thirteen-instruction frame lives.

**Still open.** OPEN. Which of the 38 carries the thirteen. The next sitting
that reads the other value answers it, and the answer arrives in a job log
rather than in an argument. kanso#1463 stays blocked until then: it is the
change that would pin a disagreeing row as a second value, which is what the
rule it implements forbids.

## 2026-09-17 — the wall is bind with a discarded value, and the one thing that made it more than that is gone

Clay, reading a book sample: "wasn't this convention always a mistake? we
invented >> to deal with no return value. but then we realized that you always
have a return value, which is the effect. so this was really just .> i
believe. one of the fused combinators."

**Measured, on the binary at `cc180f2f`.** `a >> b` and `a .> (_ -> b)` are
indistinguishable on every shape the chat could build:

    print "one" >> print "two"                  one / two
    print "one" .> (_ -> print "two")           one / two
    os/read_file! "nope" >> print "after"       short-circuits, nothing after
    print "left ok" >> print "right {boom}"     left ok, then the endpoint
    print "left ok" .> (_ -> print "right {boom}")   identical
    print "left {boom 1}" >> print "right {boom 2}"  boom 1 alone
    print "left {boom 1}" .> (_ -> print "right {boom 2}")  identical

**The archive says the last two used to differ, and that is the finding.** The
2026-08-24 entry measuring the wall recorded:

    print "left {boom a}"  >> print "right ok"        -> a
    print "left ok"        >> print "right {boom b}"  -> b
    print "left {boom a}"  >> print "right {boom b}"  -> [a b]

with "Nothing prints in any of the three. `>>` orders effects, and both
descriptions are built before either runs, so a failure raised while building
is not ordered by the wall — two of them are simultaneous and merge, the same
reasoning the parallel group uses. Haskell's `>>` answers `a` in the third
case because it is lazy in its right side; kanso builds both and learns more."

Eager construction of both operands was the one thing a lambda could not
imitate, and it does not hold on today's build: the third case answers `boom
1` rather than `[boom 1, boom 2]`, and `left ok` prints where the entry says
nothing printed. The chat could not find the entry that moved it. So the
semantic that earned `>>` its own operator went away unrecorded, and what is
left is sugar for a bind whose callback ignores its argument.

**The tree is split between the two spellings.** 570 sites write `>>` — 1 in
lib, 67 in scripts, 83 in book samples, 419 in tests — and 184 write
`.> (_ -> ...)`, four of them in lib/net/http alone. One operation, two
spellings, and nothing in the language says which. CLAUDE.md's own reason for
having no formatter and no linter is that "the grammar decides every question
a linter would ask", and here it has stopped.

**Three rules bear on it and none was applied to the wall.** The 2026-08-26
gavel minted `done`, which removed the premise that an effect answers nothing.
The 2026-08-29 gavel made effects a type whose only doors are the three words.
The 2026-08-31 gavel said that in chain position the fused form is the ONLY
spelling — and `>>` is a fourth chain-position operator over effects that
predates the effect type and was never held against that rule.

**The parked objection does not apply.** design/pending-gavels.md's Parked
list carries "dot-absorbs-`>>`: argued no — erases the visible then/bind
split." That was about the PLAIN dot absorbing the wall. The plain dot stopped
binding on 2026-08-29 and is an ordinary application now, so the entry argues
against a proposal nobody is making.

Both questions go to the ledger: whether the wall survives the fused
operators, and whether the simultaneous-failure merge was meant to go.

## 2026-09-17 — the sweep, and two rulings built inside a day

Fired 03:47:17Z, run at 03:47. Eight pull requests open in kanso, none in kq,
the oldest 5.9 hours, so nothing aged. Four are red on `cost goldens` — the
welfare and counter work moving veins under a model change — and three are
blocked or behind. Fifteen merged since the 2026-09-16 sweep; twelve name the
rulings they weighed and the three that do not are a log trim, a ratchet row
and a build-artifact hook, none of them a self-generated lead.

**The build hole is built.** kanso#1447 landed the 2026-08-24 ruling the day
after the chat found it twenty-three days off the unbuilt list.
`person "ada" _` runs, and `docs/book/samples/ch03/knot.kso` carries `_` where
it carried `none` for three weeks. Its row comes off. One site still reads
`none`: `tests/golden/micro/bare_field.kso`, which is likely correct rather
than missed — the 2026-08-24 entry says "a field may legitimately hold `none`
forever" — and is noted here so the next reader does not re-derive it.

**The explicit box is most of the way built and one probe says not all.**
`effect 5` answers a box, so the constructor landed on the ledger's
recommended spelling. `menu["dango"]!` answers a box, so the 2026-09-16
reversal is built. What did not reproduce is part 3, the check-time refusal:

    fn boom _
      err "b"

    n = boom 1

    print "{n + 1}"

reaches the endpoint at run time rather than being refused at check, on a
one-arm group whose answer is provably an err. Whether that is a limit of what
infer proves or a gap in the pass is cloud's to determine; the program is
recorded here so the question starts from a fixture rather than a memory. The
row stays until it is answered.

**Cloud is already building the two-welfare ruling**, seventeen hours after it
landed: kanso#1461 takes interpreter start-up down 14x and kanso#1470 opens
the codegen row, which is the half of a build nothing counted.

## 2026-09-17 — the allocator asked for an alignment it never needed

**mimalloc's Rust shim sends every allocation through the aligned path.** DONE.
`MiMalloc::alloc` calls `mi_malloc_aligned(size, align)` whatever the alignment
is. That wrapper checks the alignment is a power of two, builds a mask from it,
takes a candidate block off the small-page free list and tests whether the
block is aligned, before it can hand back the block `mi_malloc` would have
handed back on its own. `mi_theap_malloc_aligned` was 1,608,924 instructions of
`kanso check compile_corpus`, which is more than any frame in check.rs.

**Almost none of that work had anything to do.** DONE. A `Vec<u8>`, a `String`
and any record whose widest field is a pointer or a u64 ask for eight, and every
block mimalloc gives out is at least eight-aligned. Those go straight to
`mi_malloc` now; anything wanting more still takes the aligned path. Measured on
this container, three rows, before and after:

    compile_instructions    36,956,079 -> 36,241,230   -1.93%
    entry_instructions     131,550,570 -> 129,114,693  -1.85%
    library_instructions   131,694,754 -> 129,252,791  -1.85%

CI read it as:

    compile_instructions    36,682,232 -> 35,969,565   -1.943%
    entry_instructions     130,573,787 -> 128,144,579  -1.860%
    library_instructions   130,716,747 -> 128,281,268  -1.863%

against this container's projection of 1.93%, 1.85% and 1.85%, which is the
projection working. The floor is ratcheted to 69.79 in the same commit.

**Eight, and not sixteen, and mimalloc says why itself.** DONE. From
v3/src/alloc.c: `mi_assert_internal(page->block_size < MI_MAX_ALIGN_SIZE ||
_mi_is_aligned(block, MI_MAX_ALIGN_SIZE))`. A block is sixteen-aligned unless it
is smaller than sixteen bytes, and `Layout` carries size and alignment
independently, so `align 16, size 8` is spellable. Eight is the bound that holds
for every size.

**src/main.rs is in the wasm build, and the first round forgot it.** DONE.
`UNDER` is mimalloc on every target that can build it and `std::alloc::System`
on wasm32, and the bypass was written without that guard. On wasm
`libmimalloc_sys` is not linked at all, so the branch did not compile there,
`docs/kanso.wasm` never got built, and six jobs went red behind one missing
blob: the browser differential, the site, the asset digests, the specs and the
other host. The bypass carries the same `#[cfg(not(target_arch = "wasm32"))]`
the allocator above it does; the System allocator honours its `Layout` and has
nothing to skip.

**A run-time spec cannot reach this code, and two were written before that was
noticed.** DONE. The `#[global_allocator]` is in the binary crate; an
integration test links the library, so every allocation a test makes goes
through the harness's allocator instead. One of the two asked for alignment 4096
in blocks of eight bytes and still passed with the bound raised to 8192, which
is what a spec that reaches nothing looks like from the outside. What ships
reads source: mimalloc's assertion where it is written, and the bound in
src/main.rs against it. Both watched red — the first by altering the assertion
it quotes, the second at 16.

**mi_realloc is worse, measured and declined.** REFUTED. `GlobalAlloc`'s
default `realloc` allocates, copies and frees, which is what a growing `Vec`
pays at every doubling, and mimalloc can sometimes extend a block where it
stands. Overriding `realloc` to call `mi_realloc` on the same alignment bound
read 36,315,572 against the 36,241,230 above: a RISE of 74,342, 0.21%. Whatever
the in-place extensions save on this workload, `mi_realloc`'s own path costs
more. It also moved `compile_peak_bytes` by six bytes, because the live-bytes
accounting cannot be made exactly equivalent through one call where the default
route makes two. Not taken.

**What is left here.** OPEN. The profile under this is flat: with the three
quadratics gone (kanso#1464, kanso#1468 and the beat index), the top frame of
`kanso check library_corpus` is `__memcmp_avx2_movbe` at 3.5%, and its callers
are ten map lookups of which the largest is 0.94%. The structural lever is
interning names to integers so the maps stop comparing strings at all, which
would reach that 3.5% and part of the 2.6% in rehashing beside it. That is a
refactor across check.rs, infer.rs and codegen.rs, and it is not costed yet.

**Round four, after kanso#1464.** The interpreted run falls 3,920,499 to
2,320,967,932: the linearity analysis runs on the interpreter's compile path
too, and indexing it is felt here. CI read that figure and then read it again
in the same job. The three compile rows come back at 36,862,804, 131,830,523
and 131,972,417 against main's 36,864,779, 131,837,650 and 131,978,823, and
both interpreter memory rows are byte-identical to the round before.

## 2026-09-17 — CI's sitting of the merged tree, and the floor moves with the silicon

kanso#1462 merged with main after kanso#1459 landed. CI read the merged tree
below the values the merge carried forward:

    compile_instructions    36,682,232 -> 36,681,044   -1,188   -0.0032%
    entry_instructions     130,573,787 -> 130,567,058  -6,729   -0.0052%
    library_instructions   130,716,747 -> 130,711,295  -5,452   -0.0042%

`kanso check` does not run the interpreter, so the hashing this branch changes
is not on this corpus; all three are the layout kind the row's header
describes.

**The per-process floor tracks the silicon, and it still does not explain the
thirteen.** DONE. Three sittings, three CPUs, three floors:

    cpu 25/1    558,610   605 frames
    cpu 26/2    556,282   605 frames
    cpu 6/173   558,222   604 frames

So the floor is a host reading rather than a constant, which is what it should
be. What it cannot do is carry the thirteen: kanso#1469 and kanso#1470 both ran
on cpu 25/1 and both printed 558,610 over 605 frames, to the instruction, with
their three compile rows thirteen apart. A term the floor holds cannot move
while the floor does not. kanso#1474 prints the module compile's whole listing
beside it for that reason.

## 2026-09-17 — kanso#1462 on the merged tree: CI's sitting

The branch merged with kanso#1472 and CI measured the merged tree. Four rows
moved against the values the merge carried forward:

    interp_instructions  2,320,967,932 -> 2,178,656,557  -142,311,375  -6.13%
    compile_instructions    35,969,565 -> 35,968,356           -1,209  -0.0034%
    entry_instructions     128,144,579 -> 128,138,080          -6,499  -0.0051%
    library_instructions   128,281,268 -> 128,275,780          -5,488  -0.0043%

The interpreter row is the branch's own. The three compile rows are layout:
`kanso check` does not run the interpreter, so none of the hashing this branch
changes is on that corpus. Welfare rose and is banked at 69.7922205456612.

`per_process_floor=556292 frames=605 kernel=6.17.0-1022-azure cpu=25/1`.

- **DONE** the rows are CI's.

**Round four, after kanso#1464.** CI reads `compile_instructions` 36,885,953,
`entry_instructions` 131,919,543 and `library_instructions` 132,022,229 against
main's 36,864,779, 131,837,650 and 131,978,823. The set of a dependency's type
names is the rise, and it is the same rise round three measured; the figures
differ because kanso#1464 arrived underneath them.

## 2026-09-17 — CI's sitting of the merged tree: a correctness fix that costs a little

kanso#1465 merged with main after kanso#1459 landed. The merge carried
kanso#1459's values forward so the gate had one value to fail against; CI read
the merged tree above them:

    compile_instructions    36,682,232 -> 36,703,489   +21,257   +0.0579%
    entry_instructions     130,573,787 -> 130,655,644  +81,857   +0.0627%
    library_instructions   130,716,747 -> 130,760,194  +43,447   +0.0332%

**The rise is the fix.** DONE. `declared_names` returned types and functions in
one set, so qualifying a function's name rewrote an imported type's constructor
pattern with it, and a program that named both compiled into one that named the
wrong thing. Keeping them apart costs the checker a little more work to be
right, and a sixteen-hundredth of a per cent of a compile is what being right
costs here.

**The floor does not move.** DONE. welfare reads 69.76 against a floor of
69.76: the merge's own resolution took the higher of the two floors and this
tree clears it. Nothing to lower and nothing to bank.

## 2026-09-17 — kanso#1465 on the merged tree: CI's sitting, and the floor drops

The branch merged with kanso#1472 and CI measured the merged tree:

    entry_instructions   128,144,579 -> 128,214,733   +70,154   +0.055%
    library_instructions 128,281,268 -> 128,348,838   +67,570   +0.053%
    compile_instructions  35,969,565 -> 35,967,913     -1,652   -0.0046%

The two rises are the branch's own cost. The qualifier now keeps a set of the
type names it must not rewrite, and the entry and library routes pay to build
and read it. The module row falls, which is layout.

Welfare falls to 69.79141882095341 and **the floor is lowered to meet it**,
under the 2026-09-13 ironclad rule: the change makes the language work to its
specification. A seven-line program that `kanso check` passed had the two
engines printing different things — the interpreter `2 1`, the native backend
`error: native backend: unknown type <module>/entry` — which is the differential
law broken, not a preference. So the floor drops by exactly what the fix costs,
the reason is in the ratchet history, and this does not go to the ledger.

`per_process_floor=558232 frames=604 kernel=6.17.0-1022-azure cpu=25/1`. Note
604 frames rather than 605: a frame this binary does not have. The floors are
comparable only between two sittings of ONE binary, which is why a reading from
another branch says nothing about this one.

- **DONE** the rows are CI's and the floor is where the measurement put it.

## 2026-09-17 — the bound discharge had no golden, and it is built

`STATUS.md`'s "Ruled, unbuilt" section carries the explicit box, and among what
that ruling owes are "the two cost levers, an inlined bind for a pure index
read and bound discharge for a literal index into a known-length list, each
measured". Both are on main. Probed against `15e182b1`:

    xs = [10 20 30]
    show xs[1]     runs, no `none` arm needed
    show xs[3]     runs, no `none` arm needed
    show [7 8][2]  runs, no `none` arm needed
    show xs[0]     error[exhaustive]: this can be a none
    show xs[4]     error[exhaustive]: this can be a none
    show xs[n]     error[exhaustive]: this can be a none

Indices run from 1, so 1 through 3 are the whole of a three-element list. The
discharge is exact at both edges and does not fire for an index the compiler
cannot read off the source. The inlined bind has a golden already —
`an_index_without_the_bang_reaches_its_twin`, whose first line says the index
written without the `!` inlines.

The discharge had none. That corpus pins the MISS path and nothing pinned the
hit: a compiler that discharged every index read, which is the wrong rule and a
quiet one, passed every golden in the tree. This entry ships the fixture that
fails on it, and it is the four cases above in one program so the two halves
cannot drift apart.

It was watched red before it was watched green: with `at zero` reading `10`
instead of `<none>`, `micro_corpus_survives_a_release_build` names the file and
prints both lines.

Two notes on writing a micro golden, both learned the slow way here. A file
without `\npub play` is SKIPPED by both micro tests, silently — the first draft
of this fixture used bare statements, was read by nothing, and passed with its
expected output deliberately wrong. And the file may not carry a blank line
between its header comment and the first definition.

- **DONE** the discharge is pinned.
- **OPEN** the explicit box's row in STATUS.md: every part of it this session
  could probe is built. The row is the chat's to remove, so this is a report.

## 2026-09-17 — a gate reached its verdict and buried it past the cap

kanso#1463 gave the three compile gates a second reading: when a row parts
from its golden, the same binary is counted again in the same job, and the
gate says whether it read the same number (the change moved the row) or a
different one (a reproduction failure, which halts the vein). That is the
right machinery and it worked. Nobody could read what it said.

GitHub keeps **fifty annotations per check run**. Each failing row emits about
a dozen `::error::` lines, and the cross-run thirteen parts all three compile
rows at once — so the job carries forty-odd lines of explanation and the two
lines naming the verdict fall off the end. kanso#1477 is the cleanest case:
its whole diff is one golden fixture and a log entry, it cannot reach the
compiler at all, its three rows each read −13, and the job could be read as
far as `a move of -13` and no further.

The verdict now comes after the three lines naming the row and the size of the
move, and before the explanation of the two cases. `<row>_again` also goes out
as a `::notice::`; it was a `printf` to stdout and an append to the artifact,
and both of those have to be fetched, where an annotation comes back over the
ordinary API. `codegen_instructions.sh` took the same pair of fixes on
claude/codegen-rows earlier today, for the same reason found the same way.

`tests/a_gate_says_its_verdict_before_it_explains.rs` pins both, reading the
four gates off disk. Watched red on each half: moving the verdict block back
to the end of `entry_instructions.sh` names it at byte 9781 against an
explanation at 7209, and deleting the notice from `library_instructions.sh`
names that file.

**The thirteen itself, four pairs deep.** Every pair is two sittings of one
binary with the floor identical to the digit and the rows exactly thirteen
apart:

    kanso#1469   558610/605 twice        rows 13 apart
    kanso#1465   558232/604 twice        rows 13 apart
    kanso#1468   558726/605 twice        rows 13 apart
    kanso#1474   558610 -> 558620, +10   rows -13

and kanso#1477 adds the cleanest demonstration that it is not any diff: a
golden fixture and a log entry moved all three rows by thirteen.

- **DONE** the verdict is inside the cap.
- **OPEN** what the verdict says. The gates are armed on every branch carrying
  main; the next parting row answers from inside its own job.

## 2026-09-17 — eight runs of one binary, identical to the instruction

The thirteen had never been attempted locally. It has now, with the compile
gate's own box, its own command and its pinned tunables:

    run 1: 36384683      run 5: 36384683
    run 2: 36384683      run 6: 36384683
    run 3: 36384683      run 7: 36384683
    run 4: 36384683      run 8: 36384683

Eight sittings, one binary, one container, one corpus. Identical to the
instruction. **So the cross-run thirteen is not two runs of one binary**, and
`kanso#1463`'s second reading — which counts the same binary again inside the
failing job — should print VERDICT (1) every time.

What is left is two BINARIES. `compile_instructions.sh` has carried the reason
in its own header since it was written: cargo builds are not bit-reproducible,
a binary whose data and bss differ starts the heap at a different break, and
the 508 that row once read was exactly that — sha 55fb850296d1 counted
41,831,767 and sha de5bfab22fbd counted 41,832,275. Every CI job builds its own
compiler. Two sittings of one commit are two builds.

The sha is already printed on every run, green or red, put there so a hash
could be paired with a value. It goes to stdout, and stdout reaches only the
job log, which is the one artifact unreachable from a session reading over the
API. Three notices fix that, and the spec pins them.

That makes the next pair decisive either way. Two sittings of one head thirteen
apart with DIFFERENT shas is the answer: the row moves with the binary and the
fix is a reproducible build, not a compiler change. The same shas would refute
it and leave something genuinely unexplained inside `kanso::main`.

- **DONE** eight runs say it is not the run.
- **OPEN** whether it is the build. The sha notice answers it on the next pair.

**Round four: the warm-up holds.** CI read the start-up row 4,876,986 and then
4,876,986 again in the same job, where the round before it read 6,018,427 and
then 4,869,632. The row was counting a cold runtime-object cache and now it
counts the compile. It landed on a runner of yet another generation — AMD
family 0x1a model 0x2, where the last two rounds were 0x19 model 0x1 and model
0x11 — and read the same number twice there, which is the fleet answering the
CPU-model account that kanso#1466 retired.

On the head merged with kanso#1464 the three compile rows read 36,862,211,
131,827,785 and 131,970,557 against main's 36,864,779, 131,837,650 and
131,978,823. That fall is the declares scan this branch removes.

## 2026-09-17 — CI's sitting of the merged tree: the start-up change, priced

kanso#1461 merged with main after kanso#1459 landed. The merge carried
kanso#1459's values forward; CI read the merged tree below them:

    compile_instructions    36,682,232 -> 36,679,432   -2,800   -0.0076%
    entry_instructions     130,573,787 -> 130,564,072  -9,715   -0.0074%
    library_instructions   130,716,747 -> 130,707,970  -8,777   -0.0067%

**Not the change doing less work here.** DONE. `kanso check` does not run the
interpreter, so the start-up scan this branch removes is not on this corpus at
all. All three are the layout kind the row's header describes: src/eval.rs is
the compiler's own bytes, and editing it moves them and what sits around them.
The tenth such move recorded on this row.

**The floor does not move.** DONE. welfare reads 69.76 against a floor of
69.76. The interpreter start-up row is this branch's own vein and it is the one
the change is for; these three are collateral.

## 2026-09-17 — kanso#1461 on the merged tree: CI's sitting

The branch merged with kanso#1472 (the allocator's alignment) and CI measured
the merged tree. Four rows moved against the values the merge carried forward:

    startup_instructions   4,876,986 -> 4,838,300   -38,686   -0.793%
    compile_instructions  35,969,565 -> 35,966,784    -2,781   -0.0077%
    entry_instructions   128,144,579 -> 128,134,739   -9,840   -0.0077%
    library_instructions 128,281,268 -> 128,272,434   -8,834   -0.0069%

The start-up row is the branch's own: the interpreter stops asking the declares
scan once per name. The three compile rows are the layout kind — `kanso check`
does not run the interpreter, so none of the work this branch removes is on
that corpus, and what moves them is src/eval.rs being bytes the compiler
carries. Welfare rose and is banked at 69.79226700979217.

`per_process_floor=558659 frames=605 kernel=6.17.0-1022-azure cpu=26/2`. A
fifth distinct floor reading, and the floors are only comparable between two
sittings of ONE binary — every branch links a different compiler, so a floor
that differs across branches says nothing. The pair that matters is still two
sittings of one head.

- **DONE** the rows are CI's.

## 2026-09-17 — kanso#1461 on the merged tree, and the rows read the right way

CI's sitting on the tree merged with kanso#1465:

    compile_instructions  35,967,913 -> 35,965,137    -2,776
    entry_instructions   128,214,733 -> 128,204,898    -9,835
    library_instructions 128,348,838 -> 128,340,017    -8,821
    startup_instructions   4,838,300 -> 4,838,323        +23

**The three compile rows are LAYOUT.** This branch changes `src/eval.rs`, the
interpreter, and `kanso check` never runs it. The entry above on this row said
so and was right; two entries written later the same day said "work removed
from the corpus" about changes that could not run either, and both have been
corrected. The grep that settles it takes a second: find the callers of the
changed function, and if they all sit under `emit_ir` or under the run path,
the compile corpora never reached them.

`startup_instructions` is the row this branch is for, and it is a
re-measurement rather than a regression: the previous 4,838,300 was read on a
different tree. The 13.8x fall is in the value either way.

Welfare rose and is banked at 69.79153807658396. A rise is banked whatever
moved it; what moved this one is the compiler's bytes, and whether the ratchet
should be banking that at all is the open question in the ledger.

- **DONE** the rows are CI's, and their cause is named correctly.

## 2026-09-17 — the thirteen is in bucket zero, and bucket zero has a candidate

Two sittings of this head, same compiler source, printed frame digests that
differ in exactly one bucket:

    a8995bb9  b0=1765679/40    rows thirteen under the golden
    e7a6b980  b0=1765692/40    rows on the golden, job green

1,765,692 − 1,765,679 = **13**. The other thirty-one buckets are byte-identical
and the frame count is forty in both, so no frame appeared or disappeared: one
frame among forty changed cost by thirteen. Both sittings read
`per_process_floor=558620 frames=605`, the fifth pair with the floor identical
to the digit while the rows part.

Bucket zero is the frames whose NAME LENGTH is a multiple of thirty-two.
Listing them out of a local profile of the same workload gives forty, and one
of them costs **exactly thirteen instructions**:

    13   ./nptl/./nptl/pthread_attr_init.c:pthread_attr_init@@GLIBC_2.2.5

That is the same family as the term Clay ruled out on 2026-09-15, when
`pthread_getattr_np`'s parse of `/proc/self/maps` was found moving this row
with the binary's layout: "this has nothing to do with compiler performance and
obviously shouldn't be part of what we measure." The anchor at `kanso::main`
was the answer to that one, and it drops the stack guard placed above it. This
is thread-attribute setup reached from inside `kanso::main` instead.

It is a candidate, not the answer. Two things are unchecked: whether CI's
bucket-zero frames are the forty this box lists, and whether the thirteen in
CI's digest is this frame's cost appearing and disappearing or another frame in
the bucket moving by the same amount. The step below settles both — the forty
frames of bucket zero, by name and cost, in one notice.

- **DONE** the bucket is named, and the digest earned its place doing it.
- **OPEN** the frame. One line in the next pair of sittings.

## 2026-09-17 — the stack-slot check reads the first space, and the lever was a tenth the size advertised

`FnEmit::write` diverts every `alloca` to the head of the entry block so LLVM
does not keep a frame pointer for the function, and it recognised one by
searching the whole line for ` = alloca `. A slot reads `%name = alloca <type>`
and `%name` holds no space, so the needle begins at the line's first space or it
is nowhere. Measured on runbench's 36,086 emitted lines: all 235 slots put it
between offsets five and seven, and each of those is that line's first space.

```
kanso build pkg/runbench, kanso's own process
  before  31,280,056,421
  after   31,279,771,548   -284,873
```

IR byte-identical, 1,250,754 bytes.

**And the figure this was chosen on was wrong.** The 2026-09-16 build profile
put 18.2 million instructions over 144,261 lines against this search, about a
hundred and ten a line. The measurement above is eight a line. The 18.2 million
was an inclusive cost read as a self cost — the same mistake the hand-written
caller-tree parser made on 2026-09-15, arrived at by a different route. A lever
priced from a profile is a hypothesis; this one was worth a sixty-fourth of its
price and is recorded at what it is.

It is still worth having: free, exact, and pinned. `kanso check` stops before
codegen, so no welfare term can see it at all until the codegen rows land on
kanso#1470.

`tests/a_stack_slot_is_found_where_the_first_space_is.rs` compiles the module
fixture, runbench and a list-literal sample and asserts the narrow reading and
the whole-line one agree for every emitted line — 235 slots among them, so the
agreement is not between two functions that both said no. Watched red with the
reading pinned to offset zero; it named `%t77 = alloca [2 x %KValue]`.

- **DONE** the check is narrow and the two readings agree over real IR.
- **NOTE** a profile's inclusive cost has now mispriced a lever twice in three
  days. Price a frame from its SELF cost, or build it and measure the whole.

## 2026-09-17 — the compile profile by SELF cost, and the map keys measured at last

The entry above says to price a frame from its self cost. This is that profile,
taken the same afternoon on main: `kanso check compile_corpus`, 36,830,740
instructions, staged in the box the gate uses.

```
  1,636,363  4.44%  hashbrown HashMap::insert
  1,476,482  4.01%  infer::eval_expr'2
  1,435,071  3.90%  hashbrown rustc_entry
  1,165,325  3.16%  check::check_after_infer
  1,158,417  3.15%  infer::infer
  1,149,825  3.12%  __memcmp_avx2_movbe
  1,124,080  3.05%  check::check_merged_after_aliases
  1,043,583  2.83%  RawTable::reserve_rehash
    985,190  2.67%  lexer::lex_line
    755,027  2.05%  infer::eval_expr
    692,234  1.88%  parser::parse
    680,372  1.85%  check::per_node_walk'2
    656,191  1.78%  __memcpy_avx_unaligned_erms
    616,257  1.67%  lexer::lex
    603,907  1.64%  mi_free
```

Hash tables come to 17.9% with the lookups added — insert, rustc_entry,
reserve_rehash, contains_key, get_mut, get — and `__memcmp_avx2_movbe` at 3.12%
sits underneath them, which is what comparing string keys costs on a collision.
The allocator adds 5.1%.

**The map keys are the question the 2026-09-14 entry left open, and here they
are.** That entry recorded kanso#1033 declining an interned symbol for the AST's
own field at 365 conversion sites, and said in the same paragraph that the MAP
KEYS are a different question nobody had measured. Measured now, by caller:

```
  428,500  1.16%  < RawIterRange::fold_impl        (2,180 calls)
  291,898  0.79%  < qualify                        (1,531)
  218,600  0.59%  < Resolver::flush_unused           (831)
  181,262  0.49%  < Map::fold                        (868)
  169,058  0.46%  < bound_in_pattern               (1,454)
  151,997  0.41%  < HashSet IntoIter::fold           (750)
  148,308  0.40%  < compile_module_loaded'2          (797)
  135,045  0.37%  < Vec SpecFromIterNested::from_iter (519)
  126,238  0.34%  < collect_pattern_names          (1,302)
  115,956  0.31%  < fuse_enumerable                  (588)
```

Twenty-four more callers below these, none above 0.10%. So the keys are the
same shape as the rehash lever: one habit repeated in thirty places, largest
1.16%. Interning reaches all of it from underneath, which is the case for
doing it, and it lands on thirty sites across check.rs, infer.rs, name.rs and
codegen.rs, which is the case for not doing it while fourteen pull requests
are open against those files.

`reserve_rehash`'s own callers are `HashMap::insert` over 1,171 rehashes and
`rustc_entry` over 295. (The `phase::watched` rows the caller tree prints at
20.93% and 19.93% are inclusive chains — the whole compile passes through them
— and are not attributions. Reading one as a self cost is the mistake the entry
above corrects.)

- **DONE** the map keys are measured; the 2026-09-14 entry's open line closes.
- **OPEN** the refactor itself, and it wants a quiet tree.

## 2026-09-17 — kanso#1482's four rows, merged with main and measured by CI

The stack-slot branch merged main and the conflict resolution carried main's
four instruction rows forward, so the tree was reading numbers no sitting on it
had produced. CI's sitting on the merged head:

```
  compile_instructions   35,965,137 -> 35,965,230     +93
  entry_instructions    128,204,898 -> 128,205,462    +564
  library_instructions  128,340,017 -> 128,340,528    +511
  startup_instructions    4,838,323 ->   4,837,367    -956
```

All four are LAYOUT. `kanso check` stops before codegen and this branch changes
src/codegen.rs alone, so nothing any of these rows counts as work went near the
change; what moved them is the compiler binary carrying different bytes. The
signs say the same thing — three up, one down, no direction.

Welfare weighs the first two and not the other two, so the change costs +657
summed compile instructions. The dead band is ±0.001 points, about 105,000
summed, so the objective does not move and there is nothing to bank. The gate
agrees: it exits 0 with the value and the floor both reading 69.79.

What the branch buys is 284,873 instructions off `kanso build bench/runbench`,
and no vein on main counts that yet — the codegen rows are kanso#1470's. So
this is a change whose cost is measured and whose gain is not, until that lands.

- **DONE** the four goldens carry CI's rows; eight page spans follow them.
- **OPEN** kanso#1470's codegen rows, which would put a number on the gain.

**CI's sitting on the head merged with main after kanso#1479.** Five rows move
and only one of them is this change.

```
interp_instructions   2,178,656,557 -> 2,178,559,085    -97,472   -0.0045%  WORK
compile_instructions     35,965,137 ->     35,969,617     +4,480   +0.012%  LAYOUT
entry_instructions      128,204,898 ->    128,218,761    +13,863   +0.011%  LAYOUT
library_instructions    128,340,017 ->    128,352,943    +12,926   +0.010%  LAYOUT
startup_instructions      4,838,323 ->      4,838,372        +49   +0.001%  LAYOUT
```

The interpreted row is the change — this branch's whole compiler diff is
`src/eval.rs` and one new `interpreter_counters` in `src/main.rs`, called only
from the interpreter's `Drop` under `KANSO_COUNTERS`. `kanso check` runs
neither, so the other four are the binary's bytes moving.

**And the objective passes.** The four layout rows put 18,343 on the welfare
term — 0.011% of 164,188,378, which at 9.49e-9 a point is 0.00017 — and
`verdict` has a dead band of 0.001 either side of the floor, so a move this
size neither fails nor asks to be banked. Worth writing down because the four
rows look alarming and the score does not move.

## 2026-09-17 — kanso#1477's three rows take the thirteen, and the fourth does not

CI's sitting on the merged head:

```
  compile_instructions   35,965,137 -> 35,965,150     +13
  entry_instructions    128,204,898 -> 128,204,911     +13
  library_instructions  128,340,017 -> 128,340,030     +13
  startup_instructions    4,838,323 ->   4,838,323       0
```

Three rows move by the same thirteen and the fourth does not move at all.
That pattern names itself: the thirteen lives in `core::slice::memchr::memrchr`
under `LineWriter`, seeking the last newline in the result line each of the
three gates' own runs prints. The start-up gate prints nothing, so it has no
thirteen to draw.

Which side of the thirteen a given binary lands on is a property of its layout.
kanso#1483 stops the measured runs printing that line, and when it lands the
three rows lose the thirteen and the whole family of moves with it.

Welfare weighs the module and entry rows, so this costs +26 summed compile
instructions against a dead band of about 105,000. The objective does not move.

- **DONE** the three goldens carry CI's rows; two page spans follow them.
- **OPEN** kanso#1483, after which this row family stops drawing lots.

## 2026-09-17 — the rows are a coin, and the thirteen is not a branch's to bank

The entry above wrote CI's sitting into this branch's three rows. Its next
build measured the numbers it had just replaced, so the rows are reverted and
this correction stands in their place.

Five builds today, on trees whose compiler source is byte-identical apart from
kanso#1482's:

## 2026-09-17 — the floor is bimodal, and the gap is exactly ten

Three measurements today, and the third is the one to keep.

**The build reproduces here.** Four clean rebuilds of one tree in this
container, each preceded by `touch src/main.rs` so nothing was cached:

```
n   sha256           .text     .data   .bss    kanso::main
1   b8a64fe29f820c63 2841218   12664   29912   36377641
2   b8a64fe29f820c63 2841218   12664   29912   36377641
3   b8a64fe29f820c63 2841218   12664   29912   36377641
4   b8a64fe29f820c63 2841218   12664   29912   36377641
```

So `cargo build --release` is bit-reproducible where the toolchain, the path
and the environment hold still, and the row follows the binary exactly.

**Two builds on CI disagree in their sha and agree on every row.** Run
35197408041 was re-run on the same commit, 274c89ca, whose whole diff is three
gate scripts, one test, the log and one page — nothing `include_str!`'d, nothing
the compiler carries. Attempt one built sha `fde1fb87…` on cpu family 25 model
17; attempt two built sha `13e6cf22…` on family 25 model 1. The three compile
rows read 35,967,926 / 128,214,746 / 128,348,851 on both, to the instruction,
and the floor read 558232/604 on both. That refutes the CPU model for the third
time, now within one commit, and it says the sha difference CI shows between two
builds is not a difference the count can see. The sections would say which part
of the binary moved; they are printed and were not readable, which this change
fixes.

**The floor takes two values, ten apart.** Sixty sittings across eighteen
branches, grouped by branch and by the floor's own frame count:

```
claude/name-spaces       frames=[604]  floors=[558222, 558232]          gap 10
claude/welfare-split     frames=[605]  floors=[556282, 556292]          gap 10
claude/linear-groups     frames=[605]  floors=[556432, 556442, 558665]  gap 10
claude/group-indices     frames=[605]  floors=[558678, 558688]          gap 10
claude/beat-indexed      frames=[605]  floors=[556457, 558690, 558700]  gap 10
claude/prune-indexed     frames=[605]  floors=[558716, 558726]          gap 10
claude/declares-scan     frames=[604,605] floors=[558259, 558649, 558659] gap 10
claude/codegen-rows      frames=[604,605] floors=[558232, 558610, 558620] gap 10
claude/self-dump         frames=[604,605] floors=[558232, 558610, 558620] gap 10
main                     frames=[604,605] floors=[558232, 558610, 558620] gap 10
```

Every branch that sat more than once and did not change its frame count shows
exactly two floors, ten apart. The other gaps in that table — 378, 390, 2223,
2233 — are commits that changed the compiler. Ten is not one of those: it
recurs on ten branches with unrelated diffs, at four different absolute values.

It is not the thirteen. The floor is the set of frames whose self cost held
across all three workloads, and a row moving by thirteen while the floor moves
by ten in the other direction is two facts, not one. What it is, is the first
property of these sittings that is BIMODAL rather than noisy, and a two-valued
flag is a thing that can be chased. The frame that carries the ten is named the
same way kanso#1474 names the thirteen: bucket the floor listing and diff two
digests.

- **DONE** the sections join the sha as notices, on all three compile gates,
  pinned by `tests/a_gates_binary_is_described_where_it_can_be_read.rs`.
- **DONE** the build reproduces in this container, four for four.
- **OPEN** what costs exactly ten. The next pair of sittings that straddle the
  two floors has the digest to name it.

## 2026-09-17 — the thirteen is `memrchr`, called from the line the gate prints

Two sittings of this branch, one commit apart. The commit between them is
`cargo fmt` over one test file: no compiler source, nothing `include_str!`'d,
and both sittings printed
`compile_binary sections .text=2803570 .data=12672 .bss=29912`, byte for byte,
on the same runner family and model.

```
compile_instructions   35,967,913   ->   35,967,926      +13
entry_instructions    128,214,733   ->  128,214,746      +13
library_instructions  128,348,838   ->  128,348,851      +13
per_process_floor         558,232   ->      558,232        0
```

kanso#1474's digest, diffed:

```
frame_digest   1 bucket differs:  b0 = 1,767,181/40  ->  1,767,194/40   +13
floor_digest   0 buckets differ
frame_bucket0  1 frame of 40 differs:
               core::slice::memchr::memrchr   185  ->  198   +13
```

One frame. `core::slice::memchr::memrchr`, and its caller on this box is
`<std::io::stdio::StdoutLock as std::io::Write>::write_all`, twice, for 185
instructions. That is `LineWriter` looking backwards for the last newline in
what the process printed — and what `kanso check` prints is one line, nineteen
bytes: `compile_corpus: ok`.

So the thirteen has never been the compiler. It is the cost of writing the
gate's own result line, and it moves with the alignment of a heap buffer rather
than with anything the front end decided. The floor could not name it because
the floor is the set of frames whose self cost holds across all three
workloads, and this one holds across all three — at two values.

**Why it took this long.** Every earlier round asked what differed between the
two readings and found nothing: same sha, same sections, same CPU, same
floor, same kernel, same toolchain, and eight within-binary runs agreeing to
the instruction. All of that was true and none of it was the question. The
question was WHICH FRAME, and nothing printed a per-frame listing until
kanso#1474. The instrument named it on its second pair.

**What it costs.** `kanso::main` inclusive is the anchor, chosen on 2026-09-04
so the row counts the compiler's own work and not the loader's; printing the
result is inside that frame and is not compiling. The 2026-09-15 rule is the
one that applies: external state is normalized before it is measured, and a
term that cannot be normalized is excluded with the exclusion named in the
golden's header. Buffer alignment cannot be normalized from here. So the print
comes out of the measured region, which re-baselines all three rows at once and
is its own change.

**Demonstrated, not inferred.** The frame's cost scales with what the process
prints. Same binary, same box, one `kanso check` against two corpora whose only
difference is the length of the name that goes into the printed line:

```
compile_corpus                           memrchr = 63
compile_corpus_with_a_much_longer_name   memrchr = 81
```

Twenty-four more characters on the line, eighteen more instructions in the
frame. (`kanso::main` moved 14,177 the other way on that pair, which is the
path-length term this file's header already carries at about 160 instructions a
character; a different effect, an order of magnitude larger, and not this one.)

**And the control holds.** Run 35204603108 was re-run on one commit,
`ef0a028e`: two attempts, two CPU models (25/1 and 25/17), and every printed
quantity byte-identical — the three rows, the floor at 558338/604, all
thirty-two digest buckets, and all forty frames of bucket zero including
`memrchr`. So within a commit the measurement reproduces exactly; the thirteen
appears only BETWEEN commits, which is where the binary can move under a frame
whose section sizes do not.

- **DONE** the frame is named, with the digest diff that names it, and its
  cost is shown to follow the printed text.
**What is still open underneath it.** A test-file-only edit does not rebuild
`target/release/kanso`: the sha was `b8a64fe29f820c63` before and after adding
a comment to a test crate, in this container. And four clean rebuilds of one
source here give one sha. So the two sittings above should have been built from
the same bytes, and the frame that carries the thirteen should not have moved.
Whether CI's two independent builds of one source actually agree is the
question, and kanso#1479's `compile_binary sha256=` notice answers it the next
time a pair parts — which is the reason to land that one first.

- **OPEN** whether CI's two builds of one source are the same binary. One line
  in the next pair, once kanso#1479 is on main.
- **OPEN** taking the print out of the row. Three welfare-weighted rows
  re-baseline together, so it lands on its own with its own sitting.

## 2026-09-17 — the compile rows stop counting the line the run prints

Five CI builds today, across trees whose compiler source is byte-identical:

```
  main            ae5183a8   35,965,150   128,204,911   128,340,030
  kanso#1477      716fcfc4   35,965,150   128,204,911   128,340,030
  kanso#1481      e005650f   35,965,150   128,204,911   128,340,030
  kanso#1477      8e4e5665   35,965,137   128,204,898   128,340,017
  kanso#1485      4c5b282b   35,965,137   128,204,898   128,340,017
```

Two faces, thirteen apart on every row, and `startup_instructions` reads
4,838,323 on all five. Within a build the reading is exact: every job's
`<row>_again` has matched its first.

Neither way of not printing helps, because both change the process the gate
measures. On one box, `kanso check compile_corpus`:

4,838,323 on all five. Within a build the reading is exact — every job's
`<row>_again` has matched its first. A re-run of main's own failed job, a
second build of the same source, came back green on the other face, which is
the control.

kanso#1485 tried writing one face into the goldens and its own build measured
the face it had just replaced. So a value cannot settle this.

`kanso check` prints one line when it finishes and `kanso::main` inclusive
counted it. Under it `LineWriter` runs `core::slice::memchr::memrchr` over the
formatted bytes to find the last newline, and what that frame costs moves with
the binary's layout. The start-up gate prints nothing, which is why it never
drew.

Not printing costs more than it saves, because both ways of asking change the
process the gate measures:

```
  env -i, two variables, printing      36,817,649
  env -i, three variables, printing    36,829,255   +11,606
  env -i, three variables, KANSO_QUIET 36,828,139    -1,116
  two variables, printing              36,817,388
  two variables, --quiet               36,818,319      +931
```

The environment variable costs ten times what the quiet saves — the compiler
asks getenv about seven thousand times and each ask walks the block — and an
argv entry costs about twice it. So kanso#1483 as built is a regression.

What is left is the 2026-09-15 rule: a term that cannot be normalized is
excluded and the exclusion is named in the golden's header.
`std::io::stdio::_print` is reached once per run, from `kanso::driven`, and
`core::slice::memchr::memrchr` has no other caller in a `kanso check`.
`claude/row-excludes-the-print` subtracts that subtree from `kanso::main`
inclusive, which should map both faces onto one row.

- **DONE** the rows here are back to main's, and this branch waits on that one.
- **OPEN** whether the exclusion holds across builds, which its own CI answers.

The compiler asks getenv about seven thousand times and each ask walks the
environment block, so a third variable costs ten times what the quiet saves;
an argv entry costs about twice it. Both move the initial process layout,
which is the same class of thing the thirteen is. kanso#1483 as built is a
regression on its own row and does not land.

What is left is the 2026-09-15 rule: a term that cannot be normalized is
excluded and the exclusion is named in the golden's header. The three gates
subtract `std::io::stdio::_print` inclusive from the anchored reading, on both
of their measured runs. `_print` is reached once per run, from
`kanso::driven`, its whole subtree is the line — 828 instructions on the
profile this was read from, with `memrchr`'s 133 inside it — and nothing else
in a `kanso check` prints to stdout, because diagnostics go to stderr.

A spec reads the three gates off disk, finds the six readings that become a
row, and fails on one that does not take the print off. Watched red with the
entry gate's second reading put back the old way, which it named by line.

- **DONE** built; the exclusion parses out of a real profile here (828).
- **OPEN** whether both faces land on one row, which this branch's CI answers,
  and the three rows it re-baselines once they do.

## 2026-09-17 — the excluded row, measured

CI's sitting with the printed line taken out of the anchored reading:

```
  compile_instructions   35,964,325
  entry_instructions    128,204,133
  library_instructions  128,339,261
  startup_instructions    4,838,323   (unchanged, and green throughout)
```

Against the two faces the rows had been drawing, the excluded subtree is:

```
            low face      high face     subtracted
  compile   35,965,137    35,965,150    812 / 825
  entry    128,204,898   128,204,911    765 / 778
  library  128,340,017   128,340,030    756 / 769
```

Thirteen apart in each pair, which is the thirteen — it was inside the
subtree, as `startup_instructions` and the frame dumps had said. Which face
this build drew is not knowable from one sitting, and the thing that settles
it is a second build reading 35,964,325 again.

The gate now emits the excluded amount as a notice (`compile_printed=`,
`entry_printed=`, `library_printed=`). A number that only ever appears
subtracted cannot answer the first question a future drift raises, which is
whether the printed line's own cost moved.

Welfare weighs the module and entry rows and they fall 812 and 765 together,
1,577 against a dead band of about 105,000. The objective does not move, and
this is a change in what is counted rather than a gain to bank.

- **DONE** the three rows carry the excluded sitting; one page span follows.
- **OPEN** the second build, which is the whole claim.

## 2026-09-17 — kanso#1482's three rows, priced: layout, upward

The first sitting this branch has taken on the anchor kanso#1487 left, so
each number is one value rather than a face of the thirteen.

```
  compile_instructions   35,964,325 ->  35,964,418   +93 (+0.00026%)
  entry_instructions    128,204,133 -> 128,204,697  +564 (+0.00044%)
  library_instructions  128,339,261 -> 128,339,772  +511 (+0.00040%)
  startup_instructions    4,838,323 ->   4,837,367  -956 (-0.0198%)
```

`compile_allocs` came back 27,397 and compile memory byte-identical, so no
decision the compiler makes changed.

Three of the four worsened and they are named here because the trend gate
asks for that and it is right to: a row that moves without a sentence is the
thing the rule exists to catch. What moved them is the binary's layout. The
change is the emitter's stack-slot check reading the line's first space
instead of searching it for a substring, and the three rows above count
`kanso check`, which stops before codegen. The pass cannot run on any of
them.

The row the change is for is start-up, and it falls 956.

welfare weighs 657 of this against a dead band of about 105,000 and does not
move.

## 2026-09-17 — kanso#1462's rows, priced: the compiler compiles with the hasher it changes

CI's sitting on the anchor kanso#1487 left, so each number is one value rather
than a face of the thirteen.

```
  compile_instructions   35,964,325 ->  35,968,792   +4,467 (+0.0124%)
  entry_instructions    128,204,133 -> 128,217,983  +13,850 (+0.0108%)
  library_instructions  128,339,261 -> 128,352,174  +12,913 (+0.0101%)
  interp_allocs            5,313,431 ->   5,313,434       +3
```

`interp_instructions` came back 2,178,559,085 and `interp_peak_bytes` 933,202,
both on the row; `startup_instructions` was green at 4,838,372, `compile_allocs`
at 27,397, and compile memory byte-identical.

The three `kanso check` rows rise together and by about the same fraction,
which is what this change's shape predicts: the interpreter's hasher is part
of the compiler, so the compiler's own maps are built with it. A hash that
is cheaper to compute and worse at spreading costs a little more in a map
that is read many times per entry, and the compile corpus is exactly that.

The three allocations are not explained here. Peak bytes and the instruction
row both came back unchanged, so nothing about the shape of the interpreted
run moved.

welfare weighs 18,317 of this against a dead band of about 105,000 and does
not move.

- **DONE** the four rows, priced.
- **OPEN** nothing; the branch is CI's to confirm.

## 2026-09-17 — the bound discharge, merged onto the excluded row

kanso#1487 landed, so the three `kanso check` rows now exclude the line the
run prints and read 35,964,325 / 128,204,133 / 128,339,261 on main. This
branch changes no compiler source — a micro golden and the entries above —
so it takes those rows as they stand and owes no regeneration of its own.

The two earlier entries here are superseded by that: the thirteen this branch
drew, and the revert of the row bump that chased it, were both the frame
kanso#1487 removed.

- **DONE** merged onto main; the rows are main's.

## 2026-09-17 — kanso#1482 on the tree merged with kanso#1462: five rows, all down

    compile_instructions    35,968,792 -> 35,968,171        -621   -0.0017%
    entry_instructions     128,217,983 -> 128,213,972      -4,011   -0.0031%
    library_instructions   128,352,174 -> 128,348,205      -3,969   -0.0031%
    interp_instructions  2,178,559,085 -> 2,178,502,266   -56,819   -0.0026%
    startup_instructions     4,838,372 -> 4,837,381          -991   -0.020%

Nothing worsened. Four of the five are layout: `is_a_stack_slot` is asked from
`FnEmit::write`, which sits under `emit_ir`, and neither `kanso check` nor
`kanso run --interp` reaches codegen at all. src/codegen.rs is the compiler,
so editing it moves the compiler's bytes and what sits around them.

The interpreted row's 56,819 is worth writing down as a scale for that vein.
kanso#1468 moves the same row 237,834 in the other direction in this same
round, from an edit in the same file that likewise never executes on the
corpus. A layout term of tens to hundreds of thousands is what this row has,
and a move of that size on it means nothing on its own.

Start-up is the one corpus where the change does run, because `kanso play`
takes the native path. A one-line program emits few enough lines that the 991
saved and the layout term the other four rows show are the same size, so this
reading does not separate them; both point down and the row takes the number.

- **DONE** the rows are CI's.

## 2026-09-17 — kanso#1480's rows challenged, bisected, and the calibration's blind spot found

kanso#1480 shares one linearity `Analysis` where three were built, takes
51,082,187 instructions off a `kanso build` (−7.77%) with the emitted IR
byte-identical, and raises the two rows welfare weighs by 627,157. It was
escalated as Clay's call with three ways out — lower the floor, reweigh the
objective, or park the branch — all resting on the reading that the rise is
the layout term.

**The challenge.** This tree has measured that term on the anchored frame
twice. `scripts/gates/compile_instructions.sh`'s header carries a 2026-09-04
ladder of seven binaries differing only in code nothing reaches: span 1,028,
with 7,632 bytes of unreachable code moving the frame 402, not monotone in
`.text`. design/pending-gavels.md's `.rodata` entry carries a hundred-function
pair at 5,849 unpinned and identical pinned. kanso#1480 moves
`compile_instructions` 140,122 and `entry_instructions` 487,035, the latter
larger than the 330,496 separating its parent kanso#1478 from main. Two orders
of magnitude above both calibrations is not a thing to wave through, so the
escalation went back with a request to attribute the number rather than a
recommendation to rule on it.

**The bisection, cloud's, within the hour.** On the runner's compiler: 105
added lines in the linearity analysis cost **357** on the module row, and 74
lines rewriting two private functions in the emitter cost **145,472** —
functions reachable only from `emit_ir`, which a check never calls. So the
row moved a hundred and forty-five thousand instructions because code that
does not run on the measured path was rewritten.

**The blind spot is the shape of the perturbation, and the calibration never
covered it.** The seven-binary ladder ADDED functions nothing reaches, which
leaves every existing decision where it was; kanso#1480 REWRITES existing
unreachable functions, which moves what sits around them. Those are different
perturbations, the ladder bounded only the first, and nothing said so. One
`#[inline(never)]` in the same module, measured the same afternoon, cost 1,503
— the same family as the ladder, and three hundred times smaller than a
rewrite.

**So the challenge was right and its conclusion was wrong.** The number was
not established as layout, and saying so is what produced the bisection. What
it was taken to imply — that the rise must therefore be real work on the
check path — does not follow and is false. The correction is recorded here
rather than folded away, beside the one from earlier today about probing for
absence, because both are the same error: an argument from a measurement
whose scope was never checked.

- **DONE** the number attributed, by cloud's bisection, to a rewrite of code
  the measured path does not run.
- **OPEN** whether that makes the welfare fall a cost the project should pay.
  It is not external state under the 2026-09-15 rule — the binary's layout is
  produced by the change — and it is not work on the measured path either.
  design/pending-gavels.md's `.rodata` entry is where that sits, and it now
  has a case its earlier measurements could not make.
- **OPEN** whether the ladder should be re-run with rewrites rather than
  additions, so the calibration bounds the perturbation changes actually make.
  Cloud's.

## 2026-09-17 — STATUS.md counted the build hole twice and the two welfares not at all

The "Ruled, unbuilt" intro is the paragraph cloud reads before choosing what
to build. On 2026-09-17 it read: *Two rows stand on 2026-09-17, the build hole
having come off built (kanso#1447 ...): the explicit box ... and the build
hole, ruled 2026-08-24 and found off this list on 2026-09-16.*

So it named the build hole as having come off and as one of the two standing
rows, in one sentence, and did not name the two welfares at all — which is the
larger of the two rows that actually stand, and the one with four counters
outstanding. The headings below it were right the whole time; only the prose
that counts them was wrong, left behind by the edit that removed the build
hole's row on 2026-09-16.

This is the same failure mode CLAUDE.md's counter bullets are written against:
a count kept in a sentence and maintained by hand goes stale, and the thing it
counts is read off the sentence rather than off the list. Here the cost is that
cloud reads a standing row as retired and a retired row as standing on the one
page whose job is to hold them.

Corrected in place, with the correction recorded in the paragraph so a reader
who saw the old one knows which is which. `tests/the_status_index_counts_the_ledger.rs`
pins the ledger index's three sentences and does not reach this section.

- **DONE** the paragraph names the two rows that stand.
- **OPEN** whether a spec should read this section's headings and check the
  intro's count against them, the way the ledger index is already pinned.
  That spec has caught the ledger's count twice.

## 2026-09-17 — the box row probed, and the probe was wrong twice first

kanso#1477's body reports the explicit-box ruling stale, and kanso#1478 and
kanso#1480 cite that in their "Rulings weighed" paragraphs. Probed against a
release build of the branch tip, the report holds on everything the probe
reached: the `effect` constructor answers a box `bind` and `rescue` take, an
`(err _)` arm matches a bare err anywhere, and the check-time refusal fires in
all three shapes part 3 names --

    print "{boom 0 + 1}"     error[exhaustive]: this can be an err and `+` wants a value
    print "{(boom 0)[0]}"    error[exhaustive]: this can be an err and an index wants a value
    print (add1 (boom 0))    error[exhaustive]: this can be an err and `add1` has no arm for it

-- each naming the position and pointing at an `(err _)` arm.

**The entry is here for the two wrong answers that came first, because they
were the same mistake twice in ten minutes.**

The first pass reported the refusal unbuilt. Its three fixtures each bound the
err to a name, `x = boom 0` then `x + 1`, and the rule reads calls rather than
names. That is deliberate, it is the blind spot the `none` rule has always had,
and `docs/compiler.html` section 71 states it in the paragraph describing the
rule. The probe had found the documented exception and called it a hole.

The second pass corrected the first and added a narrower claim: that section 71
says *chapter 4 says so rather than leaving a reader to find it*, and chapter 4
does not. Chapter 4 does, in the paragraph immediately after the railway
sample: *`share` above is a name, and the checker reads calls, not the names
they are bound to, so the failure rides past `with_tip` at run time and the
endpoint reports it.* The search behind that claim was `grep` for the words
"blind spot", which is a search for a phrasing rather than for a fact.

So nothing is owed on the page or in the book, and `railway.kso` runs because
the chapter says it runs. Both wrong answers reached three surfaces before
being caught -- a STATUS.md row, a pull request body and a comment on
kanso#1480 -- and each correction went to the same three.

**The rule this leaves.** A report that a feature is ABSENT is worth what the
search for it being PRESENT was worth. Running the fixture is not that search;
the fixture only shows what happened, and what was supposed to happen is
written in the section that describes the rule. Read that first, then probe,
and where the probe contradicts the documentation suspect the probe. Both
passes here had the answer one paragraph away.

- **DONE** the constructor, the `(err _)` arm and the check refusal probed
  built, and both wrong answers corrected on all three surfaces.
- **OPEN** what the probe did not reach and the row cannot retire without: the
  710 `xs[i]!` sites, `!` names in lib answering a box, and the two cost levers
  kanso#1477 reports built. Their own pass.

## 2026-09-17 — the `.rodata` entry had two voices, and the pin was never the instrument

The ledger's `Pinning .rodata to a fixed page` entry gained cloud's bisection
this afternoon and came out contradicting itself: a body saying the term it
prices is three hundred times larger than anybody thought, and a
recommendation, written weeks earlier, declining the pin on the strength of the
small reading. An entry with two voices cannot be ruled, and the ledger's own
rule is that every entry carries a recommendation so a sitting can be a yes or
a no.

Rewritten, and the decline stands on different ground. **The pin's own evidence
never reached the case that matters.** Its demonstration was two sources a
hundred functions apart — an ADDITION, the same perturbation family as the
seven-binary ladder, and the family the pin was shown to fix. Nobody has run it
against a REWRITE, which is what costs 145,472. Adopting the pin on that
evidence would be adopting it on a measurement of something else, which is the
error this entry is now a record of twice over.

**And the gate's own header points at a different mechanism.** It says what it
found chasing this row's variance: a binary whose data and bss differ starts
the heap at a different break, which moves how much work malloc does to service
an identical request sequence, with every kanso symbol identical to the
instruction and only glibc's allocator moving. The heap break is set by where
`.bss` ends. A fixed `.rodata` start does not fix it.

So the instrument worth ruling on is a heap that starts at the same address
every run. It removes the term for additions and rewrites alike, and it changes
the shipped binary by nothing — which answers the kanso#1234 objection the old
recommendation leaned on, since nothing gets special-cased away from what ships.
It is also the 2026-09-15 rule read literally. The gate already pins ten
`GLIBC_TUNABLES` for this exact reason and where the heap begins is the one it
does not pin.

The entry now names the measurement that settles it, and says it should not be
ruled without one: cloud's kanso#1480 pair read twice, once with `.rodata`
pinned and once with the heap start fixed. A build and two callgrind runs.

- **DONE** the entry has one voice and a recommendation a sitting can answer.
- **OPEN** those two readings. Cloud's, and until they exist this is an
  argument rather than a decision.

## 2026-09-17 — the rewrite explanation withdrawn, and what the ledger said on it

kanso#1492 built the ladder this morning's entry asked for and the answer
refutes the entry. Eight rewrites of `without_stats_gate`, unreachable from a
check, eight distinct binaries: the row is identical to the instruction across
all of them, `.text` spanning 256 bytes. Rewriting unreachable code costs
nothing.

So the sentence in "kanso#1480's rows challenged, bisected, and the
calibration's blind spot found" — that a rewrite moves what sits around it, and
costs three hundred times what an addition does — is wrong. It was an
attribution read as a mechanism: the bisection established that 145,472 arrived
with 74 rewritten lines, and this chat wrote that down as rewriting being the
cause. A difference-in-differences is not a mechanism until something isolates
it, and the isolating experiment says zero.

**Where that claim had already travelled, and what each cost.**

    design/compiler-log.md   the entry above, on main       cloud corrects in kanso#1492
    docs/compiler.html §73   published, stating the rule     corrected here
    design/pending-gavels.md the `.rodata` recommendation    withdrawn here, unmerged

The ledger one is the one that mattered. kanso#1489 rewrote that entry's
recommendation around "the term is small for additions and large for rewrites,
and most changes rewrite", and asked for the entry to be re-weighed on it. That
is the sentence a sitting would have been ruling against. It is withdrawn
before the pull request lands rather than corrected after.

**What survives, and the entry is stronger for it.** The pin's decline no
longer rests on a contested reading of the term's size, because all three
calibrated shapes now agree it is small — about 402 for an unreachable
addition, zero for an unreachable rewrite, 2,733 for a reached one. A pin that
removes part of a few-thousand-instruction term, at the price of a one per cent
larger shipped binary, is a bad trade on any of those numbers. And the second
argument is untouched: the gate header's own mechanism is the heap break, set
by where `.bss` ends, which a fixed `.rodata` start does not reach.

**The 146,628 is open again**, fifty times the largest calibrated shape and
belonging to no shape anybody has measured. kanso#1492 names what would settle
it: the frame-level diff of the two compile profiles, which CI uploads as
artifacts on every run.

- **DONE** the claim withdrawn from the ledger before it was ruled on, and from
  the published page.
- **OPEN** what carries the 146,628. Cloud's, and it needs the profile pair
  rather than another table.
