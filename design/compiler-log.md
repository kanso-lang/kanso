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

## 2026-09-06 — K_REC IS 30% OF PENDBENCH, AND THE REUSE ANALYSIS REACHES 100 OF ITS 3,200,900

**ATTRIBUTED (#340), no change.** kanso#1258 took `k_rec` out of encodebench's
profile entirely — the entry above §54 records that it "does not appear in the
profile at all" there. It is pendbench's largest function by a wide margin and
had never been read on that benchmark.

    k_rec   184,084,984 over 3,200,900 calls   57.5 each

28.60% of the 643,666,736 pendbench ran at when this was measured, and 30.39% of
the 605,691,007 the branch now lands on, because the `k_itoa` change above took
74,845,459 out from under it.

**Two call sites are all of it**, and they are the same one at two recursion
depths:

    d_list/next_1     96,026,195 over 1,600,400 calls   60.0 each
    d_list/next_1'2   88,003,483 over 1,600,000 calls   55.0 each
    d_pendbench/made_1    28,000 over       500 calls
    d_list/iter_1         16,000 over       300 calls
    d_list/fold_3         12,000 over       200 calls
    k_rec_reuse            6,000 over       100 calls

A lazy list's `next` builds a fresh cursor record every step, 3.2 million of
them a run.

**The runtime already has the shape that would avoid it and the analysis does
not reach here.** `k_rec_reuse` writes the new fields over a victim record when
the victim is a record of the same arity, and `src/codegen.rs` emits it wherever
`reusable_records` — the linearity analysis, keyed by file and line and column —
says this construction is the last reader of some record in scope. In pendbench
it fires **100 times against 3,200,900**, and none of the hundred is in
`d_list/next_1`.

Whether it *should* reach there is the open question and not a defect on the
evidence here: a lazy list's previous cursor may still be held by the caller,
which is what laziness is for, and a reuse that wrote over a live cursor would
be a miscompilation rather than a slow path. What the numbers establish is the
size of the prize — 3.2 million constructions at 57.5 instructions, 30% of the
benchmark — and that the analysis currently answers no to all of them.

**OPEN**, deliberately: the next step is to read `reusable_records` against
`lib/list`'s `next` arms and find out whether the no is a proof or a gap.

## 2026-09-06 — THE REUSE ANALYSIS NEVER ASKS ABOUT A DESTRUCTURED PARAMETER

**ANSWERED (#341).** The entry above left open whether `reusable_records`
refusing all 3,200,900 of pendbench's record constructions is a proof about
liveness or a gap. It is a gap, and a one-line one.

`sole_finished_record` in `src/linear.rs:811` opens its loop over the arm's
parameters with

    let Pattern::Var(name, _) = pattern else { continue };

so a parameter that is destructured rather than named is skipped before any
question about it is asked. Every `next` arm in `lib/list/list.kso` destructures:
`(cursor at source)`, `(bounded at stop source)`, `(capped left source)`,
`(counting at)`, `(cycled at source)`, `(grown seed stretch)`,
`(mapped shape source)`, `(paired left right)`, `(repeated value)`,
`(sifted keep source test)`, `(skipped burn source)`. Eleven arms, no bare
parameter among them, so the analysis produces no candidate for any of them —
whatever the liveness would have said.

The construction it would have to reason about is the one the arm writes as its
own step: `onward = cursor (at + 1) source` reads `at` and `source` out of the
cursor that arrived and builds another of the same width. That is the shape
`k_rec_reuse` exists for, and the shape the doc comment on `reusable_records`
describes — `shift (n - 1) (point (p.x + 1) p.y)`, with `p` finished by the time
the constructor runs — with the parameter destructured at the door instead of
read through a name.

**This is a finding, not a licence.** Being skipped is not the same as being
safe: whether the arriving cursor is finished still turns on
`callers_hand_over`, and a lazy list exists so that a caller can hold a cursor
and ask it for more later. Writing over one a caller still holds would be a
miscompilation, which is the failure mode this analysis is built to avoid. What
is established is that the question has never been put — the 30.39% is
unexamined rather than examined and declined.

**OPEN**: extend `sole_finished_record` to destructured parameters and find out
what `callers_hand_over` answers for `list/next`. The measurement to take first
is whether the eleven arms' incoming cursors are handed over, because a no there
closes the thread at no cost.

## 2026-09-06 — THE CURSOR IS NOT HANDED OVER, SO THE 30% CLOSES AT NO COST

**CLOSED (#342).** The thread above ends where it was designed to end cheaply.
Before extending `sole_finished_record` to destructured parameters, ask the
other half of its condition about `list/next` and see whether the answer is
already no.

A temporary `KANSO_REUSE_PROBE` in `reusable_records` — never committed —
printing `Analysis`'s two questions for every group whose name ends in `next`:

    PROBE list/next/1  escapes_as_value=false  hand_over_p0=false  params=["Ctor/other"]

`next` is never mentioned as a value, so the analysis has every one of its call
sites to look at, and having looked at them it says some caller does not hand
its cursor over uniquely. That is the answer a lazy list should give: a caller
holds a cursor and asks it for more later, which is what the structure is for,
and a reuse that wrote over a held cursor would be a miscompilation.

**So the 3,200,900 constructions are refused twice over**, and only the first
refusal was the one-line skip. Extending the analysis to destructured
parameters would change nothing here: `sole_finished_record` requires both
`here == everywhere` and `callers_hand_over`, and the second is already false.
The 30.39% is not reachable this way.

The gap the entry above found is still a gap — a destructured parameter is
skipped before the question is asked, so some other arm somewhere may be losing
a reuse it would qualify for. What is settled is that `list/next` is not one of
them, and that pendbench's largest function is doing work the objective has no
cheaper way to buy.

## 2026-09-06 — CI'S NUMBERS, AND THE PROJECTION MISSED THE TWO ROWS THAT SHARE A PROGRAM

**The goldens now hold CI's readings rather than the container's projection.**
Both changes above were measured here and written in as CI's previous values
plus the container's deltas, which is the method that had landed to the digit
for seven changes running. On this branch it landed for eleven rows of thirteen
and missed twice, by the same amount both times.

    row            projected          CI              miss
    encodebench    4,425,110,405   4,425,477,605   +367,200
    livebench      4,437,721,317   4,438,088,517   +367,200
    the other 11   exact

encodebench and livebench are the same program — livebench runs encodebench's
against the shipped library instead of the frozen copy — so a single cause
shows up twice at identical size. The same pair missed by the same 367,200 on
the previous head, so it is a property of that program on these two hosts and
not of either change.

`scripts/gates/dispatch.sh` exists to answer whether silicon accounts for a
moved row, and it cannot answer here: `differs` returns 2 for want of a recorded
block, and there is no `bench/dispatch.txt` because that was resolved
deliberately — a recorded block would have blinded the ratchet. This run printed
`cpu family 0x19 model 0x1`. What the numbers support is that the container's
deltas are reliable for eleven of these thirteen programs and unreliable for the
encode program on this pair of hosts; what would settle the mechanism is a
sitting of that one program on both, which is not this change's to take.

**`compile_instructions` moves from 41,461,538 to 41,461,798**, a rise of 260,
and `docs/compiler.html`'s `data-golden` follows it. This is the gate's own case
(1), which its failure text spells out: `src/runtime.c` is `include_str!`'d into
the compiler, so a runtime edit moves this row with the front end untouched.
Worth recording that the intermediate head measured +6,169 and the `k_itoa`
commit brought it back to +260 — the row tracks the size and shape of the
embedded text, not the compiler's work.

Welfare reads 74.62 against a floor of 74.62 on CI's numbers, so the floor set
on the projection stands without a re-set.

## 2026-09-06 — PARSE_VALUE'S 26.98%, AND THE POINTER IT RELOADS 17 MILLION TIMES

**ATTRIBUTED (#343).** The decode's largest function had a function-level figure
and no instruction-level reading. `d_jsonbench/parse_value_2'2` is 468,780,150
instructions, **26.98% of jsonbench** and more than double the next function.

Searched first: `parse_value` appears three times in the log and not at all in
the archive. kanso#1262 explains its growth — it absorbed `scan_at_5`'s
133,577,400 by inlining and gained 136,356,000 doing it — and the entry-block
thread (kanso#1245) with `preserve_none` behind it (#290, toolchain-blocked)
covers its prologue. None of them reads what the 468 million is made of.

Same method as `encode_onto`: callgrind at instruction granularity joined to
`objdump` over the function's 1,021 instructions. 594 execute and the join
accounts for all 468,780,150.

**Grouped by execution count, the per-call band is the largest.** 49
instructions run once per call at 2,713,950 calls — 132,983,550, 28.37% of the
function and **7.65% of jsonbench**. That band holds the thirteen-instruction
prologue (six pushes and a 120-byte frame), the entry's first two byte reads,
and the dispatch on what the byte is.

**The shape that repeats is the non-strict byte index.** `bs[i]` inlines to a
bounds test, a reload of the buffer's data pointer, the fetch, and a `cmove`
substituting the past-the-end sentinel:

    test %rbx,%rbx              ; index above zero
    jle  ...
    cmp  %rbx,%rsi              ; index within the length
    jl   ...
    mov  0x8(%r14),%rcx         ; the data pointer, again
    movzbl -0x1(%rcx,%rbx,1),%ecx
    cmove %rdx,%rcx             ; or 0x100, past the end

Twenty-two fetch sites execute **17,041,950** times between them, 0.98% of
jsonbench. Twenty-three pointer reloads execute **17,359,050** times, 1.00% —
one per fetch, plus one site that reloads without fetching.

**The length is hoisted and the pointer is not.** `parse_value` loads the byte
count once at entry and keeps it in `%rsi` for the whole call; it loads
`0x8(%r14)` afresh at every read. `%r14` never changes, and the buffer it points
into is the input document, which nothing in the decode appends to.

LLVM is right not to hoist it and `!invariant.load` would be wrong: the `data`
field of a bytes header is genuinely mutable, because `k_b_append_mut` rewrites
it when a builder grows, and a callee between two fetches could do exactly that.
What makes the decode's reads redundant is a fact about this buffer rather than
about the type — the same distinction that made the closure's `!invariant.load`
correct where this one would not be.

**So the 1.00% is real and its fix is not one line.** Recorded as the size of
the prize and the reason the obvious route is closed. What is left unexamined in
this function is the residual of the per-call band once the prologue is set
aside, which is #290's ground and blocked on the same toolchain.

---

## 2026-09-06 — SPLIT ASKED MEMCMP PER BYTE POSITION: readbench −97.7490%, scanbench −44.0754%, welfare 75.16

**DONE.** Searched first, as the filing gate requires: `readbench` and
`scanbench` appear in this file and in `log/compiler-log-archive.md` only as
rows that moved — thirty-odd mentions between them, every one a benchmark
total. Neither has ever had a function-level reading, and `d_readbench` and
`d_scanbench` return nothing in either file. `k_b_split` appears nowhere in
this file and nowhere in the archive.

**Two functions are 98.13% of readbench.** Callgrind on the merged main
compiler, 2,038,390,385 instructions:

    k_b_split              1,245,429,204   61.10%
    __memcmp_avx2_movbe      754,792,000   37.03%

The benchmark reads `bench/large.json` — 188,698 bytes with no newline in
them — and splits it on `"\n"` two hundred times. `k_b_split` walked the bytes
twice, once to count the pieces and once to cut them, and called `memcmp` at
every position of both walks: **75,479,200 calls**, which is 200 rounds by two
walks by 188,698 positions, at exactly 10.0 instructions a call. The function's
own 1,245,429,204 is 16.5 instructions a position on top. Fifty-three
instructions of input byte to learn that a one-byte separator is not there.

**A match can only start where the separator's first byte is.** `memchr`
covers the ground in one vectorised pass and `memcmp` runs only at the
candidates it hands back; a one-byte separator skips the `memcmp` entirely.
Both walks call one helper, so the counting pass and the cutting pass cannot
drift apart again.

    row            before             after            delta        pct
    readbench   2,038,390,798     45,883,331   -1,992,507,467  -97.7490%
    scanbench   1,384,644,173    774,357,155     -610,287,018  -44.0754%
    basket         38,029,049     35,508,390       -2,520,659   -6.6282%
    pendbench     605,691,420    605,536,009         -155,411   -0.0257%

Nothing rises. The other nine rows are byte-identical on the container's own
A/B, which is what says the four falls are the change: those nine programs
call split nowhere, so a change confined to split cannot reach them.

**scanbench was not the target and is the second largest win here.** It was one
of the two benchmarks named in the previous entry's list of programs with no
attribution at all, and it turns out to share readbench's defect without
sharing its shape.

**The `text` vein worsens, deliberately, from 1,262,522 to 1,264,058.** The
four binaries that call split each gain the 384 bytes of `k_split_find`; the
other nine never link it and are unchanged. 1,536 bytes for 2.6 billion
instructions.

**`compile_instructions` falls 41,461,798 -> 41,460,229, by 1,569.** The gate's
own case (1): `src/runtime.c` is `include_str!`'d into the compiler, so editing
the runtime moves what the compiler carries and compiles. Measured twice on the
container, 41,881,485 -> 41,879,916 both times, the second reading identical to
the first.

**No allocation counter moves.** All eleven veins agree with their goldens
untouched, which is the expected shape: the change removes instructions, not
allocations.

**Watched red twice, for two different reasons, and the second found a hole in
the corpus.**

The memchr span off by one — searching `len - seplen - from` bytes rather than
one more — misses a separator sitting at the last position it can occupy, and
the fixture says so in four cases: `"ab\n"` splits into one piece instead of
two, `"ab"` on `"ab"` into one instead of two.

Taking a matching first byte for a match without verifying the rest gives
`"a:b::c"` on `"::"` three pieces instead of two, and `"→x→→y"` on `"→→"` a
piece cut mid-codepoint. **The shipped corpus passed that mutation.**
`tests/golden/micro/text_split.kso` had three multi-byte separator cases and in
every one the separator's first byte occurred only where the whole separator
did, so nothing in it could tell a first-byte match from a real one. Three
cases were added that can, and the mutation reddens them on both engines
against the interpreter's answers.

**OPEN — the two walks are still two walks.** The counting pass exists to size
the buffer, and it now costs a memchr sweep rather than a memcmp per byte, so
it is cheap enough that removing it was not measured. A growable buffer or a
recorded position list would halve the remaining scan. Not attempted here.

**OPEN — `k_b_chars` and `k_b_at` are the neighbours with the same shape.**
`k_b_chars` walks a string twice to count codepoints and then to cut them.
Nobody has priced either at the instruction level.

---

## 2026-09-06 (later) — CI'S COMPILE ROW IS 373 ABOVE THE CONTAINER'S PROJECTION

**DONE.** The entry above projected `compile_instructions` at 41,460,229, from a
container A/B that read 41,881,485 -> 41,879,916 twice over. CI counted
**41,460,602**, so its own delta from 41,461,798 is 1,196 rather than 1,569.
CI's sitting is the record and the golden holds 41,460,602;
`docs/compiler.html`'s `data-golden` follows it. **The correction is to that
one line of the entry above: `compile_instructions` falls 41,461,798 ->
41,460,602, by 1,196.** The direction and the reason are unchanged — this row
moves because `src/runtime.c` is `include_str!`'d into the compiler.

**Everything else in that entry landed to the digit.** All thirteen work rows
and all thirteen `.text` rows came back from CI byte-identical to the
projection, including readbench 45,883,331 and scanbench 774,357,155. Nine of
the thirteen instruction deltas were zero, which is why: only the four
benchmarks that call split could move, and their deltas were measured on the
container against a binary built the same way.

**Why this row is the one that misses.** Its own header says so: cargo builds
are not bit-reproducible, and a binary whose data and bss differ starts the
heap at a different break, which moves how much work malloc does to service an
identical request sequence. 373 instructions is that, and it is a fifth of the
5,124 the header records for a change of chip. The runtime rows do not have
this exposure because they are counted on programs the compiler emitted rather
than on the compiler itself.

---

## 2026-09-06 (third) — SPLIT HANDS BACK THE INPUT WHEN IT FINDS NO SEPARATOR: readbench −90.6543%, welfare 75.17

**DONE.** Continues the first entry of today, whose OPEN thread asked what was
left in readbench once the memcmp-per-position went. Searched again as the
filing gate requires: this file's only mentions of `k_b_split` are today's two
entries, `log/compiler-log-archive.md` has none, and neither file anywhere
discusses handing a split's input back as its own piece.

**With the scan fixed, readbench is 82.66% one memcpy.** 45,882,918
instructions, of which `__memcpy_avx_unaligned_erms` is 37,928,498 and
`__memchr_avx2` 7,684,400 — 400 memchr calls, two per round, one per walk, at
19,211 instructions for 188,698 bytes. The copy is the 200 rounds copying the
whole document to return the single piece, because the separator is not in it.

**A KStr's `data` and `len` are written once at construction.** The only
in-place write to one anywhere in the runtime is `k_str_chars` memoising the
codepoint count into `cap`, and two holders of one string would compute that
alike. So when the count loop finds nothing, the one piece can be the input
value.

    row            before             after            delta        pct
    readbench      45,883,331      4,288,131      -41,595,200  -90.6543%
    scanbench     774,357,155    776,362,839       +2,005,684   +0.2590%
    basket         35,508,390     35,510,217           +1,827   +0.0051%
    pendbench     605,536,009    605,537,209           +1,200   +0.0002%

`read_allocs` 615 -> 415, exactly the two hundred copies; `read_alloc_bytes`
37,942,816 -> 198,816 and `read_sh_str` 37,932,784 -> 188,784. `scan_allocs`
falls by 4 and `scan_sh_str` by 128, which is scanbench's four splits that find
nothing.

**THE THREE ROWS THAT RISE ARE THE SHAPE OF THE TEST, and the first shape cost
three times as much.** Written as a test on the last piece — `from == 0 ? sv :
k_str_n(...)` — `sv` stayed live to the tail and spilled: scanbench makes
501,500 split calls and paid **twelve instructions on every one**, 6,017,840 in
total, for a path four of them take. Taken as an early return before the buffer
is sized, the loop below reads exactly what it read before and the cost is four
instructions a call. The call counts are identical across all three builds, so
this is the test and the branch, not work.

The early return also skips the second walk for that case, which is why
readbench lands at 4,288,131 rather than the 8,138,118 the first shape read.
That closes the first entry's OPEN thread about the two walks for the only
input where the second one was free to remove.

**The `text` vein worsens, deliberately, from 1,264,058 to 1,265,018** — 240
bytes on each of the four binaries that call split. **`compile_instructions`
rises 41,460,602 -> 41,461,827**, which is `src/runtime.c` growing by the
comment and the block, `include_str!`'d into the compiler. That row is
PROJECTED from a container A/B of 41,879,916 -> 41,881,141; the entry above
this one records CI reading 373 off the container's last projection of it, so
CI's sitting corrects this if it differs.

**Welfare 75.16 -> 75.17, `--set` in this PR.** The objective takes the trade:
readbench's dimension is nearly saturated, so most of a 90% fall scores
nothing, and scanbench's 0.259% is a real loss against it. The sum still rises.

**Watched red.** Returning `sv` whether or not a separator was found makes the
last piece the whole input, and the corpus names it in nine cases at once:
`"a,b,c"` on `","` answers `["a" "b" "a,b,c"]`, `"a/b/c"` joined back reads
`a/b/a/b/c`. Both engines agree with the interpreter on all thirteen cases with
the fix in place.

---

## 2026-09-06 (fourth) — k_b_chars IS 504 INSTRUCTIONS; k_b_at IS 44.51% OF indexbench

**CLOSED and ATTRIBUTED.** The entry above left `k_b_chars` and `k_b_at` open
as neighbours of `k_b_split` with the same double-walk shape, and said neither
had ever been priced. Searched first: `k_b_chars` and `k_b_at` appear in
neither `design/compiler-log.md` nor `log/compiler-log-archive.md` at the
function level; `k_b_at` is the function kanso#1172 and kanso#1173 gave the
seek cursor, and those entries name the cursor rather than the function's
share.

**`k_b_chars` is 504 instructions in the whole corpus.** It is reached by one
benchmark, scanbench, on one call. Its double walk — once to count the
codepoints and once to cut them — is the shape `k_b_split` had, and removing it
would be worth 0.00% of anything the objective weighs. CLOSED by measurement
without building.

**`k_b_at` is the one worth a number.**

    benchmark     calls      Ir        a call   share
    indexbench   20,000   2,088,089    104.4   44.51%
    basket       12,000   2,553,876    212.8    7.19%

They are two different paths through one function. indexbench's is the string
index: 10,000 of its 20,000 calls reach `__memcpy_avx_unaligned_erms`, which is
the fresh one-codepoint string each index returns. basket's is the map index:
12,000 calls to `k_map_sorted` and 20,467 to `__memcmp_avx2_movbe`, 1.7 key
comparisons a lookup over a small sorted array.

**Recorded as size, not as a plan.** indexbench is 4,690,952 instructions in
total, the smallest row in the corpus, so all of `k_b_at` there is 2.09 million
against the 41.6 million the entry above banked on readbench. `index_instructions`
is also one of the granted baselines — it entered the objective at its
dimension's standing — which is the standing question in #319. Whoever takes
this should read that entry in `design/pending-gavels.md` first.

---

## 2026-09-06 (fifth) — obj_key_start IS 197 INSTRUCTIONS A CALL, AND 170 OF THEM RUN EVERY TIME

**DONE.** Attribution only — no code changes. `d_jsonbench/obj_key_start_4'2`
is 234,197,700 instructions, **13.48% of jsonbench** and the second largest
function there after `parse_value`.

Searched first: it has a function-level figure in
`log/compiler-log-archive.md` (281,591,550, 9.71%, alongside a note that
`value_for` is called 1,188,150 times from it) and three mentions in this file,
the largest a fall of 77,361,900 from the dispatch relaxation. None of the four
says what the remaining instructions are.

Callgrind at instruction granularity joined to objdump over the function's 647
instructions; 221 execute and the join accounts for all 234,197,700.

**1,188,150 calls, 197.1 instructions each.** The striking thing is how little
of it is a loop: **170 instructions execute at exactly the call count**,
201,985,500, which is 86.25% of the function and 11.62% of jsonbench. Only four
bands run at any other frequency, the largest 17 instructions at 788,400.

By opcode, over the whole function:

    mov      69,204,750  29.55%
    cmp      38,020,800  16.23%
    jne      16,634,100   7.10%
    xor      13,069,650   5.58%
    je       11,881,500   5.07%
    test      8,317,050   3.55%
    movzbl    8,317,050   3.55%
    push      7,128,900   3.04%
    pop       7,128,900   3.04%

`cmp`, `jne`, `je` and `test` together are 31.95%: this is a straight-line body
that tests and branches rather than one that computes. `movzbl` at 3.55% is the
byte reads — 7 a call, against `parse_value`'s 22 sites at a much lower
frequency.

**Recorded as the shape, not as a repair.** A 170-instruction straight-line
prologue-to-return body on a function entered 1,188,150 times is where a
specialisation would pay, and the same measurement says what to compare against:
`parse_value` is 49 instructions in its own per-call band. Whoever takes this
should establish first whether the 170 is one arm or the sum of a dispatch over
several, because those want different repairs.

---

## 2026-09-06 (sixth) — THE DECODE'S CALL CHAIN, PRICED PER CALL: str_char IS 621 INSTRUCTIONS

**DONE.** Attribution only. The three functions under `obj_key_start` in
jsonbench's profile, each of which had a share and no per-call number. Searched
first: `str_char_4` appears once in this file and once in the archive,
`array_step` twice and once, and none of the five gives a call count or a
per-call cost.

    function                        Ir      share      calls   a call
    parse_value_2'2         468,780,150    26.98%  2,713,950     49*
    obj_key_start_4'2       234,197,700    13.48%  1,188,150    197.1
    str_char_4              165,240,450     9.51%    265,950    621.3
    array_step_3'2          135,270,450     7.79%    410,550    329.5
    string_at_4             101,214,154     5.83%  1,571,250     64.4

    * parse_value's 49 is its per-call BAND from the 2026-09-06 entry, not its
      whole per-call cost; the other four are the function total over its calls.

**The chain is `parse_value` -> `obj_key_start` -> `string_at_4` ->
`str_char_4`, and it narrows sharply.** `string_at_4` is entered 1,188,150
times from `obj_key_start` — once per call, exactly — plus 317,100 from
`parse_value` and 66,000 from the non-recursive `obj_key_start`. Of its
1,571,250 entries only **265,950 reach `str_char_4`**, one in six.

**`str_char_4` is the most expensive per call in the decode, by a factor of
three over `obj_key_start`.** 724 instructions in the function, 213 execute,
and the join accounts for all 165,240,450. Unlike `obj_key_start` it is a loop:
its bands run at 3,484,500, 3,218,550, 2,800,200, 2,534,250, 684,300 and
265,950, so 13.1 inner iterations for every call. `cmp`, `je`, `jne` and `test`
together are 41.24% of it — a higher branch share than any other function
measured today — and it calls `k_b_utf8` for 55,519,500 of its inclusive cost.

**Recorded as where to look next, with the reason it is not obvious.** 621
instructions a call over 265,950 calls is 9.51% of jsonbench, and one in six
`string_at_4` entries reaching it says the ASCII path already avoids it most of
the time. So the prize is what the non-ASCII sixth costs, and whether 13.1
iterations a call is the string's length or a scan that restarts.

---

## 2026-09-06 (seventh) — THE COMPILE ROW CANNOT BE PROJECTED FROM A CONTAINER A/B, AND TODAY MISSED TWICE

**DONE.** CI counted `compile_instructions=41,462,716` for the change above; the
entry projected 41,461,827 from a container A/B of 41,879,916 -> 41,881,141.
The golden holds CI's figure and `docs/compiler.html`'s `data-golden` follows
it. **The correction is to that one line: `compile_instructions` rises
41,461,798 -> 41,462,716, by 918.** The direction is unchanged — `src/runtime.c`
grew and it is `include_str!`'d into the compiler.

**Twice in a row today, and by different amounts.** The memchr change projected
41,460,229 and CI read 41,460,602, a miss of 373. This one projected 41,461,827
and CI read 41,462,716, a miss of 889. Both projections came from a container
A/B measured on a pair of builds, both reproduced on the container to the
instruction on a second reading, and both were wrong about CI by a few hundred.

**So stop projecting this row.** Every other vein takes a container delta
faithfully: today all twenty-six runtime rows and all twenty-six `.text` rows
across two changes came back from CI byte-identical to what the container
predicted, and nine of thirteen instruction deltas were zero by construction.
This row does not, and its own header says why — cargo builds are not
bit-reproducible, a binary whose data and bss differ starts the heap at a
different break, and that moves how much work malloc does to service an
identical request sequence. The container's delta measures ITS pair of
binaries; CI builds a different pair.

A session touching `src/runtime.c` or `lib/` should therefore push once with
the row unchanged, let the gate fail, and copy CI's value out of the job log —
one red round that is expected rather than two that are not. The container
reading is still worth taking, as the check that the row moved in the direction
the change implies; 373 and 889 are both far below the 5,124 the header records
for a change of chip, so a projection that misses by thousands is a different
problem and should be hunted.

---

## 2026-09-06 (eighth) — obj_key_start's 170 ARE EIGHTEEN STRETCHES, SO THERE IS NO ARM TO LIFT

**CLOSED.** The 2026-09-06 (fifth) entry recorded that 170 of
`obj_key_start_4'2`'s instructions execute on every one of its 1,188,150 calls
— 86.25% of the function, 11.62% of jsonbench — and left the question of
whether that is one arm or the sum of a dispatch. Measured on the same
instruction-level join: **eighteen disjoint stretches**, none of them adjacent.

    0x3f80  30    0x407f  15    0x40dd   3    0x41b2  21
    0x4240   4    0x4263   2    0x4276  12    0x42d4   3
    0x4340   7    0x466a   8    0x46a2   3    0x46d2   5
    0x46ee   4    0x4726  18    0x4792   4    0x47ae   2
    0x47c1  16    0x4a16  13

Every call threads all eighteen, and between them sit the instructions that
execute at other frequencies. So the 170 is not a straight-line body that a
specialisation could lift out whole; it is the always-taken skeleton of a
branchy one, and the 31.95% of the function that is `cmp`, `jne`, `je` and
`test` is that skeleton's shape rather than a prologue.

**What that rules out.** Outlining "the arm" has no arm to outline; the fifth
entry's suggestion to compare against `parse_value`'s 49-instruction per-call
band does not carry, because that band IS contiguous and this one is not. A
repair here has to remove branches or the work they guard, one stretch at a
time, and each stretch is between 2 and 30 instructions — so the largest single
prize in the function is 30 instructions a call, 35,644,500, 2.05% of
jsonbench.

The next entry's `str_char_4` is the better target on this evidence: 621.3
instructions a call against this function's 197.1, and a loop rather than a
skeleton.

---

## 2026-09-06 (ninth) — str_char's 42 instructions a byte, and why nothing hoists them

`d_jsonbench/str_char_4` is 165,240,450 instructions, 9.51% of jsonbench. It is
entered 265,950 times — once per escaped string per iteration, 1,773 × 150 —
and everything inside is the walk of that string's tail.

**What the input is.** `bench/large.json` holds 10,475 string literals of
77,732 bytes; 1,773 of them contain a backslash, and 26,019 bytes lie at or
after each one's first backslash. That tail is what `string_at` hands to
`str_chars`, and it is dense: 4,562 escapes and 6,335 clean runs between them
averaging 2.67 bytes, 2,802 of those runs empty because two escapes are
adjacent. The 2026-09-01 entry declined the escaped-tail run-scan at +1.16%
without knowing this; the census is why it lost, and it retires the idea rather
than leaving it to be tried again. There is nothing to scan.

**Where the 42 go.** Joined instruction by instruction against the callgrind
profile, the literal-byte path is 12 in the indexed load (two bounds tests, the
input string's `len` and `data`, and the tag round trip through the 0x100
sentinel), 6 in the three arm compares, and 24 in the inlined
`k_b_append_mut_byte`: the accumulator's tag, the `k_stats_on` read, `cap` and
its absolute value in three instructions, `len`, `data`, the frontier word at
`data - 8`, then the store and two length writes. The escape path is 49 —
the same, plus a jump table and one of four 23-instruction arms.

**The ceiling.** Every guard in the byte arm was deleted and the program
measured, which is not shippable and answers the only question worth asking:

    jsonbench   1,737,413,813 → 1,698,791,213   −38,622,600   −2.223%

Twelve instructions an append across 3,218,550 appends. That is the whole of
what the guard set can ever be worth.

**Three attempts on the reloads, none of which moved a byte.** The header
loads repeat every iteration because the byte store may alias them.

1. `!alias.scope` on the store and the frontier word, `!noalias` on
   `k_stats_on` and the three header loads — a true claim: a bytes buffer's
   data is a separate allocation from the KBytes that names it. Byte-identical
   machine code.
2. The reason turned out to be that there is **no loop**. The emitter writes
   tail recursion as `musttail call tailcc`, and TailCallElim is forbidden to
   touch a `musttail` call — except for a SELF call, which it does convert:
   `d_list/fold_go_3`, `d_list/next_1` and `d_jsonbench/skip_ws_2` all carry a
   `tailrecurse` block with a back edge after `opt -O3`. A MUTUAL cycle gets
   nothing, and `str_char_4` ↔ `str_chars_3` ↔ `str_escape_4` is mutual. So are
   jsonbench's four other most expensive functions. The loop in the machine
   code is the backend's jump past the prologue, and no IR pass ever saw it.
3. `alwaysinline` on the two forwarders collapses the cycle: `str_char_4` then
   carries `tailrecurse` and a real back edge. It is worth −950,250, −0.055%.
   With the metadata of (1) added on top, the count is identical to the digit —
   1,736,463,563 either way. A loop was necessary and is not sufficient: the
   loop body still contains the calls the fast arms fall back to
   (`k_b_append_mut`, `k_b_at`, `k_b_utf8`), and LICM hoists to a preheader,
   which needs the load invariant over every path rather than the hot one.

**And the musttail is right.** Rewriting all 125 of them as plain
`tail call tailcc` — which would let TailCallElim at the self-recursive ones
freely — costs 1,737,413,813 → 1,854,484,765, +6.74%, output byte-identical.
LLVM's answer without the guarantee is a real frame.

**What this corrects.** `call_twin`'s comment in src/codegen.rs says its ten
callable tests "are loop-invariant and LICM can hoist them out of the loop
TailCallElim makes of the recursion". That holds for a self-recursive fold and
not for a mutual cycle, and the 2026-09-05 (fourth) entry attributed the
byte-identical `!invariant.load` result to dereferenceability when the simpler
reading is available for any mutual caller: LICM had no loop to work in.

**Closed.** The remaining safe money in `str_char` is the three instructions
that take `|cap|`, 9,655,650 or 0.56% of jsonbench, and it costs a change to
how KBytes records its allocation regime. Not taken.

---

## 2026-09-06 (tenth) — the array walked the same whitespace twice

`array_items` in lib/json/value.kso opened with `p2 = skip_ws cs p` and then
called `parse_value cs p2`. `parse_value` opens with `skip_ws` of its own, so
the second walk always began on the byte the first had stopped on and found it
not to be whitespace. One line, deleted:

    jsonbench   1,737,413,813 -> 1,698,318,413   -39,095,400   -2.2503%
    oneshot        25,494,372 ->    25,233,736      -260,636   -1.0223%
    livebench   4,438,088,070 -> 4,437,827,434      -260,636   -0.0059%

The other ten rows are byte-identical, which is the check that the fall is this
change and not the weather. livebench moves by the same 260,636 as oneshot
because both decode once; jsonbench decodes 150 times.

**Why it is worth 2.25% for one line.** `array_items` is called once per array
element, and both `skip_ws` copies are inlined into their callers, so each
element paid a bounds-checked byte read, a four-way whitespace test and the
tagged round trip through the 0x100 sentinel to learn what the caller before it
had already learned. bench/large.json's arrays hold 410,550 elements over 150
iterations.

**The rest of the family was checked and is not redundant.** Seven other sites
write `p2 = skip_ws cs p`, and every one of them reads `cs[p2]` afterwards or
compares p2 against the length: `array_step`, `obj_items`, `obj_key`,
`obj_value`, `parse_array`, `parse_object` and `finish`. `parse_value`'s own
skip is the one that does the work. This was the only duplicate.

**Behaviour is identical, including on failures.** Under the old code an error
inside the element was reported at parse_value's p2; under the new code
parse_value computes the same p2 from p. The 23 json tests pass, and so does
the full suite (36 test binaries, 0 failures) once docs/kanso.wasm is rebuilt —
lib/*.kso is `include_str!`'d into the compiler, so the wasm blob carries this
change and had to be regenerated with it.

**The veins.** No allocation counter moves: all eleven runtime cost goldens
agree, because this removes instructions rather than allocations. Three that do
move, all falls, all banked in this change: `.text` -272 bytes on each of the
three decoding binaries, the emitted code one call and two lines lighter, and
the front end's visits on lib/json 17,169 -> 17,115. Welfare 75.17 -> 75.21.

**Where it came from.** Looking for the second read of an already-loaded byte
in `array_step`'s instruction-level profile — 13 instructions at 1,429,650
executions, 1.07% of jsonbench — and finding a whole redundant scan one frame
up instead. The `cs[p2]` re-read the search started from is still there and is
still 1.07%; it needs `skip_ws` to hand back the byte it stopped on, which is a
larger change and is not in this one.

---

## 2026-09-06 (eleventh) — whitespace becomes the arm before the error

The entry above removed one redundant `skip_ws`. Seven remained, and every one
was followed by a dispatch on `cs[p2]` — the byte the scan had just loaded and
thrown away. In `array_step`'s instruction-level profile that second read is 13
instructions at 1,429,650 executions, 1.07% of jsonbench on its own.

The dispatch tables already exist. Whitespace becomes the arm before the error
in each of them, and the caller hands its byte straight in:

    fn array_step cs (parsed p v) acc
      array_delim cs cs[p] p (push acc v)

    fn array_delim cs c p acc
      blank = ws? c
      if blank (array_delim cs cs[p + 1] (p + 1) acc) (array_bad c p)

Six sites convert: `array_step`→`array_delim`, `obj_items`→`obj_key_start`,
`obj_value`→`obj_delim`, `parse_array`→`array_open`, `parse_object`→`obj_open`,
`parse_value`→`value_for`. Two do not: `obj_key` feeds `expect_char`, which has
no `cs` to advance with, and `finish` compares the position against the length.

    jsonbench   1,698,318,413 -> 1,573,203,261   -125,115,152   -7.3670%
    oneshot        25,233,736 ->    24,399,645       -834,091   -3.3055%
    livebench   4,437,827,434 -> 4,436,993,353       -834,091   -0.0188%

The other ten rows are byte-identical.

**Almost none of that is whitespace.** bench/large.json is 188,698 bytes and
holds 2,238 blanks, 1.19%, all of them spaces inside string values that the
decoder never dispatches on. The fall is the layer: a `skip_ws` call per token
that loaded a byte, tested it against four literals, wrapped the answer in a
tag and handed back only the position, and a caller that then loaded the same
byte again.

**The four-arm form was built first and the objective declined it.** Writing
whitespace as literal arms — `fn array_delim cs 9 p acc` and three more per
site, 24 in all — reads better and measures further: jsonbench −8.5219%,
oneshot −3.8237%. It costs 1,109 more front-end visits, 1,409 more emitted
lines and 6.42% more compile instructions (container A/B, 41,831,743 ->
44,515,270), and welfare comes out at **75.19 against a floor of 75.21**. A
fall is a fall: the shape went. The guarded form is one arm and one binding per
site plus three small helpers for the error messages, keeps 86% of the runtime
win, and costs 1.70% of compile instructions (41,831,743 -> 42,543,278) —
welfare 75.32.

That comparison is the useful part. Two spellings of one idea, identical in
behaviour, and the objective separates them: the difference is entirely how
much source the front end has to read.

**The veins.** No allocation counter moves. `.text` RISES on the three decoding
binaries — jsonbench 95,074 -> 95,346, oneshot 118,434 -> 118,962, livebench
119,010 -> 119,570 — because the helpers and the `ws?` call sites are code the
`skip_ws` inline copies were not; welfare weighs no machine-code size term
(ruled 2026-09-05), so that is a movement to state. Emitted code rises: the
decoder's calls 1,764 -> 1,835, branches 1,150 -> 1,207, lines 12,047 ->
12,588. Front-end visits FALL, 17,115 -> 17,092, because the eight `p2 = ...`
bindings that leave are about what the guards cost.

**CI's rows for the entry above, and three projections in this one.** CI read
the array_items change's compile veins and all three moved: compile_instructions
41,462,716 -> **41,411,787**, a fall of 50,929, and compile_peak_bytes 715,275
-> **714,995**, a fall of 280. compile_allocs moved too and its value is further
back in the job log than the API hands back. So this change carries
compile_instructions at 42,123,322 — CI's 41,411,787 plus the container's own
A/B delta of 711,535 — compile_peak_bytes at CI's 714,995 with no delta for
this change, and compile_allocs unchanged. Both of the latter two gates refuse
on this container (rustc 1.94.1 against the runner's 1.98.1) and cannot be
measured here at all. One red round is expected and CI's sitting is the record.

The nine counters that worsen, by the gate's own keys and the values they land
on: emitted_defines 184, emitted_calls 1,835, emitted_branches 1,207,
emitted_lines 12,588; emitted_other_defines 1,799, emitted_other_calls 15,820,
emitted_other_branches 9,884, emitted_other_lines 103,019; and text 1,265,562.
All nine are the same thing said nine ways — six guarded arms, three helper
functions and eleven `ws?` call sites are code the inlined `skip_ws` was not.
They buy 125,115,152 instructions off the decode, and welfare weighs none of
them.

---

## 2026-09-06 (twelfth) — the objective's own count was one behind, in three files

CLAUDE.md said `scripts/welfare.kso` weighs "decode allocations and arena
blocks, encode allocations and arena blocks, fixpoint rounds, expression visits
and emitted lines". The path has a directory in it, and none of those last three
has been a term since the 2026-09-03 rebuild. What the objective actually weighs
is twenty-eight counters: an instruction row for each of the thirteen
benchmarks, twelve memory rows, and `compile_instructions`, `compile_allocs`
and `compile_peak_bytes`.

The cost of the stale sentence is in the entry above. Building the four-arm
whitespace fold, I read the emitted-lines rise of 4.5% as a welfare term and
spent a round working out why the number went the other way; the objective
cannot see that vein at all, and the fall came entirely from
`compile_instructions`.

`bench/objective_sources.txt` and the spec that replays it both said 27, and
the file has held 28 since livebench joined on 2026-09-05. Nothing was
unchecked — `tests/the_objective_reads_what_the_gate_watches.rs` reads the file
rather than a number, and it passes — but three pieces of prose disagreed with
the data beside them, which is the shape CLAUDE.md's own "all TEN cost goldens"
correction was about. All three now say 28, and CLAUDE.md's sentence names the
counters and points at the file rather than listing them from memory.

---

## 2026-09-06 (thirteenth) — the digit test travelled as a tag

`scan_at` ended in

    digit_step cs start p marked (47 < c and c < 58)

and `digit_step` had `true` and `false` arms. So a comparison the emitter fuses
into a branch when an `if` consumes it was instead materialised as a tagged
boolean, passed as an argument, and taken apart by the callee's dispatch. In
the merged `value_for_3'2` that reads, per digit:

    2b9e  add    $0xffffffffffffffd0,%r10
    2ba2  cmp    $0xa,%r10
    2ba6  mov    $0x3,%edi
    2bab  sbb    $0x0,%rdi
    2baf  cmp    $0x2,%rdi
    2bb3  jne    2c30

Three of those six build the tag and test it, at 4,640,700 executions.

Writing the test as an `if` inside `scan_at` and deleting `digit_step`:

    jsonbench   1,573,203,261 -> 1,570,703,811   -2,499,450   -0.1589%
    oneshot        24,399,645 ->    24,382,982      -16,663   -0.0683%
    livebench   4,436,993,353 -> 4,436,976,690      -16,663   -0.0004%

**A sixth of the arithmetic prediction, and the reason is worth having.** Three
instructions at 4,640,700 executions is 13,922,100, and the row moves 2,499,450.
The `and` of two comparisons still travels as a value — only the last step, the
`if`'s own test, fuses. So the emitter's `Cond` machinery reaches a comparison
under an `if` and not a comparison under an `and` under an `if`, and the 2026-09-04
entry's 1.60% figure for this family is the ceiling rather than the take.

Every other vein falls with it, which is the unusual part: compile_instructions
−101,081 on the container, front-end visits 17,092 -> 17,068, the decoder's
emitted defines/calls/branches/lines all down, and `.text` −48 bytes on each of
the three decoding binaries. Two arms leave the library and nothing replaces
them. Welfare 75.32 -> 75.33.

The branch's ten worsened counters against main, by the gate's keys and the
values they land on: compile_instructions 42,022,241 (a projection; CI's
sitting corrects it), emitted_defines 183, emitted_calls 1,834,
emitted_branches 1,205, emitted_lines 12,562; emitted_other_defines 1,797,
emitted_other_calls 15,818, emitted_other_branches 9,880, emitted_other_lines
102,967; and text 1,265,418. The three entries above have the reasons: six
guarded whitespace arms and three helper functions are code the inlined
`skip_ws` was not, and the digit test's `if` gives a little of it back. Against
that the decode retires 166,710,002 fewer instructions, oneshot 1,111,390 fewer.

---

## 2026-09-06 (fourteenth) — the ratchet caught its own mutation going stale

`a_decoder_that_answers_a_wrong_checksum` patched `acc2 = push acc v` in
`array_step`, and that binding went in the eleventh entry above when the
function stopped calling `skip_ws` and started handing its byte straight to
`array_delim`. The ratchet's `applies_all` reported it on the same branch that
caused it:

    ratchet: 1 mutations no longer apply
      STALE json decoder end-to-end (native, 150 decodes)

The push is an argument now and the mutation doubles it there. Watched red on
the new source before it was taken as fixed: the mutated decoder answers
checksum 48000 against the 24000 it owes, which is the doubling the mutation's
own comment predicts, and `scripts/gates/native_checksum.sh` exits 1 on it and 0
restored.

`applies_all` reads a worktree of HEAD rather than the working tree, so the fix
has to be committed before the ratchet can see it. A session that edits the
mutation and re-runs from the working tree gets the same STALE line and has no
way to tell whether the edit was wrong.

---

## 2026-09-06 (fifteenth) — CI's compile rows, and the floor I set on a guess

CI measured the branch and every projection landed exactly except the compile
veins. All thirteen work rows, all four emitted rows, all twelve emitted-other
rows and all thirteen `.text` rows came back byte-identical to the container's
own A/B deltas applied to CI's previous sitting — twenty-nine rows, no misses.
The three that could not be projected:

    compile_instructions   42,022,241 -> 42,018,130   -4,111    projected, out by 4,111
    compile_peak_bytes        714,995 ->   722,429   +7,434    NOT PROJECTED AT ALL
    compile_allocs                  ?                          not yet read

`compile_peak_bytes` is the one that matters. Both it and `compile_allocs`
refuse on this container — rustc 1.94.1 against the runner's 1.98.1 — and a
refusal exits before measuring, so the eleventh entry above carried CI's value
for the PREVIOUS commit with no delta for its own change. The whitespace fold's
six guarded arms and three helper functions are declarations the front end
holds while it checks, and they cost 7,434 bytes, 1.04%.

**So the floors of 75.32 and 75.33 were set on numbers nobody had measured.**
With CI's two, the branch head reads **75.30**. That is still a rise against
main's 75.17 — the decode is 9.6% cheaper and the objective takes the trade —
but it is 0.03 below a floor I wrote from a projection, and the floor has to
come down to what was measured rather than the change being excused past it.
That is a re-basing of a number that was never a reading, not an accommodation:
the weights are untouched and the runtime rows are exactly what was claimed.

The floor is not moved in this commit, because `compile_allocs` is still
unread and 75.30 is provisional in the same way 75.33 was. CI's next sitting
gives it, and the floor is set once on three measured rows.

**What to do differently.** A vein that refuses on the container is a vein with
no projection, and carrying the previous commit's value into a golden reads as
a measurement when it is a placeholder. The eleventh entry said so and set the
floor anyway. Push with the row unchanged, take the red, and set the floor from
CI — the same rule the 2026-09-06 (seventh) entry wrote for
`compile_instructions`, which applies with more force here because this row
cannot even be A/B'd.

---

## 2026-09-06 (sixteenth) — the last compile row, and the floor set on three readings

`compile_allocs` came back from CI on f5f4914d: 25,490 -> 25,817, a rise of
327, or 1.28%. That is the third and last of the compile veins, and it closes
the entry above. All three, as measured:

    compile_instructions   41,462,716 -> 42,018,130   +555,414   +1.34%
    compile_peak_bytes          714,995 ->    722,429     +7,434   +1.04%
    compile_allocs               25,490 ->     25,817       +327   +1.28%

The three move together and by about the same fraction, which is what a change
that adds declarations to lib/json should look like: six guarded whitespace
arms and three helper functions enter the library where an inlined `skip_ws`
used to be, and the digit test's two arms leave it. Against that the decoder
retires 166,709,589 fewer instructions a run, 9.5952%.

The branch reads **75.27**, and the floor is now that number. It was 75.33,
and 75.33 was not a reading — the eleventh and thirteenth entries set it from
projections for two rows that cannot be projected at all. `--set` refuses to
lower the objective, and refuses correctly: it cannot tell a re-basing from an
excuse. So the floor was lowered by hand, which is what the flag's own refusal
tells you to do, and this entry is the sentence a reviewer reads beside the
diff. The weights are untouched. Against main's 75.1655 the branch is a rise of
0.108.

Worth keeping separate: 75.33 -> 75.27 is not a regression this branch
introduced between one commit and the next. Nothing in the tree changed between
the two numbers. The first was arithmetic on a placeholder and the second is a
measurement, and the difference between them is the size of the error in the
placeholder.

---

## 2026-09-06 (seventeenth) — the `and` under an `if` is asked in pieces

The digit test in the (thirteenth) entry measured a sixth of what the
arithmetic predicted, and that entry said why: the `and` of two comparisons
still travels as a value, so only the `if`'s own test fuses. That names a gap
in the emitter rather than a fact about the language, and this closes it.

`a and b` parses to `if a b false`, `a or b` to `if a true b` and `not a` to
`if a false true`, so a condition is very often another `if`. `emit_cond` had
no arm for one. It fell through to `emit_expr`, which built the inner `if` as a
value — a phi over tagged booleans — and then `test_cond_value` called
`k_truthy` on the phi and branched on the answer. Two comparisons that each
already knew their answer as an i1 were rebuilt into a tag and taken apart
again, which is the family the (2026-09-05) comparison change measured at 1.60%
of encodebench on one site.

`emit_cond` recurses into it now. Each arm of the inner `if` is asked the same
question the outer one asked, and an arm that is the literal the desugaring
wrote is an unconditional branch. Nothing is duplicated: both arms branch to
the labels the outer `if` already made, so the change adds blocks and removes
instructions.

    work_jsonbench   1,570,704,224 -> 1,564,492,424   -6,211,800   -0.3955%
    work_oneshot        24,383,381 ->    24,341,969      -41,412   -0.1698%
    work_livebench   4,436,977,137 -> 4,436,935,725      -41,412   -0.0009%
    work_scanbench     776,362,839 ->   776,364,842       +2,003   +0.0003%

The other nine work rows hold and no allocation counter moves — all eleven
veins agree. Every other vein falls with it: the decoder's emitted calls
1,834 -> 1,832, branches 1,205 -> 1,185 and lines 12,562 -> 12,509; six of the
twelve `_other` rows fall and none rises; machine code falls on six binaries,
400 bytes on jsonbench and on livebench.

**This entry said the three compile rows do not move at all, and that was wrong
about one of them.** `compile_allocs` and `compile_peak_bytes` are byte-identical,
as CI confirmed. `compile_instructions` FELL 42,018,130 -> 41,886,863, a fall of
131,267 or 0.31%, and it is layout: `kanso check lib/json` stops before the
backend runs, so no decision this row counts can change, but src/codegen.rs is
the compiler and the compiler's own bytes move under it. The vein has recorded
seven layout-only moves before, all from runtime or prelude edits; this is the
first from the emitter, and the largest. "The backend never runs" keeps the
decisions identical and says nothing about where they land.

work_scanbench 776,364,842 is the one row that pays, and it is 0.0003%. A
condition whose arms are not constants gains two blocks and a branch where the
phi used to be, and LLVM does not always fold them back.

The four work rows are PROJECTIONS — the golden is CI's and this container
reads a different glibc — so each is the golden plus the container's own A/B
delta, measured on one host from the repo root with both binaries in place.
Every other row here is exact.

Watched red before it passed, on the old emitter and for the right reason:
`an_and_under_an_if_is_asked_in_pieces` reported that `pick` still called
`k_truthy`. `a_condition_made_of_and_is_asked_in_pieces` covers the shapes the
new arm reaches — two int comparisons, the runtime path a text comparison
takes, a `<` declared over a record, `not`, `or`, both nestings of the two, an
`and` in tail position, and a hand-written `if` standing where a condition
goes — and both engines answer it identically.

---

## 2026-09-06 (eighteenth) — the index's two bounds compares buy something

`k_index_fast` and `k_b_at_fast` both test a 1-based position with two signed
compares and an `and`:

    %lo = icmp sgt i64 %i, 0
    %hi = icmp sle i64 %i, %len
    %inr = and i1 %lo, %hi

One unsigned compare on the offset answers both. Below 1 the subtraction wraps
to something enormous and fails the same `ult` that a position past the end
fails, so `icmp ult (i - 1), len` is exactly equivalent and three instructions
become two. Built, measured, DECLINED: it is worse on nine of the thirteen
benchmarks.

    livebench    4,436,935,278 -> 4,526,588,390   +89,653,112   +2.02%
    encodebench  4,425,477,206 -> 4,502,836,968   +77,359,762   +1.75%
    digestbench     77,352,921 ->    79,973,742    +2,620,821   +3.39%
    oneshot         24,341,570 ->    24,594,343      +252,773   +1.04%
    jsonbench    1,564,492,011 -> 1,568,798,811    +4,306,800   +0.28%
    scanbench      776,364,429 ->   775,361,412    -1,003,017   -0.13%

The checksum stays 24000, so this is not a correctness difference. What the two
signed compares buy is a FACT: on the fast path LLVM knows `i >= 1` and
`i <= len`, and it spends that on the addressing mode — every index in the
decoder's disassembly reads `movzbl -0x1(%rax,%rbp,1)`, with the `-1` folded
into the address. The unsigned form proves only `j < len`, so the offset is
materialised at every use. Two instructions saved at the compare, more than two
paid everywhere the result is read.

Reverted, and the baseline returns to 1,564,492,011 to the instruction.

The attribution that prompted it is worth keeping. On the decoder after the
whitespace fold and the `and` change, `obj_key_start_4'2` is 205,282,500
instructions, 13.12% of jsonbench and second only to `value_for_3'2` at 22.60%,
and its whole body is six repeats of the indexed-load block at 1,060,050
executions each. Two structural walls stand behind it, both already recorded:
the length and the data pointer are RELOADED at every site because calls sit
between them and may clobber memory, and there is no LLVM loop to hoist out of
because the recursion is a mutual cycle. A third thing the join shows is
smaller and real: the guard reads the index's tag with three compares before it
compares the byte, and one of the three — the failure test — is provably dead,
because the tag is a phi over exactly `{int, none}`.

---

## 2026-09-06 (twentieth) — an append of a slice reads the range in place

`d_jsonbench/str_char_4` was 164,974,500 instructions, 10.58% of the decode.
It is the walk after an escape: once a string has a `\n` in it, `str_chars`
went to the closing quote a character at a time, appending each. 1,773 of
bench/large.json's 10,475 strings have an escape, and the walk covers 26,019
bytes a parse — 42.27 instructions a byte, against roughly two on the find2
path the escape-free strings take.

find2 already knows how to skip to the next quote or backslash. Writing that:

    fn str_chars cs p acc
      str_run cs p (text/find2 cs p 34 92) acc

    fn str_run cs p n acc
      str_char cs cs[n] n (text/append acc (text/slice cs p (n - 1)))

is WORSE on its own, by 3.4635%. The runs between two escapes are a median of
three bytes, and a `slice` of three bytes is a view header the arena hands out
to be read once and dropped: `k_b_slice_raw` appears at 59,127,900 where it
was absent, and `k_b_append_wide` goes 7,446,600 to 54,299,400.

So the emitter fuses the pair. `append acc (slice cs a b)` is recognised
before either argument is emitted — the same place and the same wrapper-
spelling rule as the `utf8` of a `slice` above it — and reaches
`k_b_append_slice_fast`, an alwaysinline door that tests the four tags, does
the slice's bounds arithmetic itself, and copies the range into the
accumulator's spare capacity with the small-copy ladder the string arm of
`append_mut_byte` already uses. Nothing is boxed.

Three things had to be in it before it paid:

- **The door has to inline.** Out of line, through the C, the pair cost
  1,585,031,315 against the baseline's 1,559,465,765 — a call into
  `append_slice` and a second into `append_range` are more than the byte walk
  they replace.
- **The empty range has to be answered inline.** 1,363 of the corpus's 6,335
  runs are empty, because two escapes sitting next to each other leave no
  bytes between them, and sending those to the C left the row at +1.6394%.
- **The empty range must not build a view either.** `k_b_append_slice`'s C
  path first appended `k_bytes_view(data, 0)`, which is a 32-byte header for
  nothing: 204,450 allocations a decode, and the row read -0.9153% rather
  than -1.0607%.

Three work rows fall and ten are byte-identical:

    work_jsonbench   1,559,466,178 -> 1,542,925,378  -16,540,800  -1.0607%
    work_oneshot        24,300,109 ->    24,190,898     -109,211  -0.4494%
    work_livebench   4,432,486,910 -> 4,432,419,027      -67,883  -0.0015%

The decode's allocations fall with them: allocs 4,999,965 -> 4,734,015, a fall
of 265,950, which is exactly one per escaped string per run — `string_at`'s
own `append (slice ...)` at the head of the escape path is a fused site too,
and its view is the one that goes. alloc_bytes 259,660,448 -> 251,150,048 and
sh_bytes 27,950,400 -> 21,567,600 with it. find2_calls rises 1,571,250 ->
2,521,500, one per run, and append_fast falls 3,218,550 -> 1,634,550: that is
the trade, one scan for four appends.

Machine code rises on exactly the three programs that have the pair in them —
jsonbench 91,922 -> 93,362, oneshot 115,442 -> 116,882, livebench 116,050 ->
117,490, 1,440 bytes each — and is byte-identical on the other ten, which is
the check that the door is linked only where it is used. The emitted-line
count rises everywhere, by 134 lines, because the emitter writes the door into
every module and the linker drops it again.

lib/json gains one declaration and the front end pays for it: rounds 40 -> 42,
visits 17,068 -> 17,264, compile_peak_bytes 722,429 -> 724,493,
compile_allocs 25,817 -> 25,899, compile_instructions 41,888,129 ->
42,089,618. Banked.
Welfare holds at 75.30 and the floor is re-set on the new terms.

The other gate keys this branch moves, by name: oneshot_append_fast, oneshot_find2_calls, front_end_rounds,
front_end_visits, emitted_branches, emitted_calls, emitted_defines,
emitted_lines, emitted_other_branches, emitted_other_calls,
emitted_other_defines, emitted_other_lines, live_append_fast,
live_find2_calls.
oneshot_find2_calls 20,950 -> 27,285 and live_find2_calls 4,200,475 ->
4,206,810 are the same scan-for-appends trade the decode makes;
oneshot_append_fast 127,239 -> 116,679 and live_append_fast 42,334,257 ->
42,323,697 are its other half. The four emitted_ keys and the four
emitted_other_ keys are the door's 134 lines in every module.

Every counter this branch moved, with the value it landed on:

    emitted_branches 1,185 -> 1,203
    emitted_calls 1,832 -> 1,847
    emitted_defines 183 -> 185
    emitted_lines 12,509 -> 12,716
    emitted_other_branches 9,815 -> 9,981
    emitted_other_calls 15,808 -> 15,858
    emitted_other_defines 1,797 -> 1,811
    emitted_other_lines 102,778 -> 104,532
    a_builder_handed_on_is_still_a_builder_alloc_bytes 165 -> 198
    a_cluster_entered_by_a_tail_call_sweeps_sh_str 9,035,232 -> 9,035,264
    a_pushed_call_keeps_the_sweep_sh_buf 19,168 -> 19,200
    a_repaired_node_below_the_mark_holds_tenure_sh_buf 10,976 -> 11,008
    an_escaped_list_gives_its_buffer_back_sh_buf 6,368 -> 6,400
    build_cycle.imported_sh_buf 176 -> 208
    builder_guard_sh_str 176 -> 208
    builder_reclaim_sh_bytes 936 -> 960
    builder_transient_sh_bytes 1,896 -> 1,920
    effect_push_shape_sh_buf 672 -> 704
    fold_push_shape_sh_buf 174,848 -> 174,880
    fused_map_shape_sh_buf 174,848 -> 174,880
    fused_select_shape_sh_buf 174,848 -> 174,880
    fused_tally_sh_buf 9,872 -> 9,904
    record_fields_sh_buf 2,736 -> 2,768
    sort_shape_sh_buf 180,464 -> 180,496
    stream_write_sh_bytes 4,776 -> 4,800
    string_builder_shape_alloc_bytes 8,246 -> 8,279
    string_headers_sh_buf 2,736 -> 2,768
    take_shape_sh_buf 174,848 -> 174,880
    tally_shape_sh_buf 1,984 -> 2,016
    the_same_capture_built_below_the_mark_is_shared_sh_buf 10,944 -> 10,976

bench/compile_golden.txt's five samples each gain the same 134 lines, and its
four totals with them: lines 5,018 -> 5,688, calls 205 -> 215, branches
312 -> 357, defines 189 -> 194. Per sample that is 1,006 -> 1,140 on
recursion, 998 -> 1,132 on dispatch, 995 -> 1,129 on guards, 1,047 -> 1,181 on
records and 972 -> 1,106 on build_block, with one more define and two more
calls apiece. That is the door written into every module again. Rounds and
visits do not move at all, which is the check that the front end decided
nothing differently for these five.

The module sample in bench/compile_golden_modules.txt carries the door too:
module_lines 5,051 -> 5,185, module_calls 752 -> 754, module_branches
421 -> 430, module_defines 97 -> 98, and module_rounds and module_visits
hold.

`tests/golden/mem/append_of_a_slice_boxes_nothing.mem` is the spec, and it was
watched red: with the fusion switched off it reads allocs=85 and
sh_bytes=1944 against the 45 and 984 it pins, one view per round over forty
rounds. `tests/golden/micro/an_append_of_a_slice_reads_the_range_in_place`
covers what the door has to answer the same way the unfused pair did — an
inverted range, a start below one, an end past the length, a range inside
multibyte text, and two appends threaded through one accumulator.


## 2026-09-06 (nineteenth) — the counter switch is set once, and every row falls

`k_stats_on` initialised itself on first use, inside `k_alloc`:

    if (k_stats_on != 0) {
        if (k_stats_on < 0) k_stats_on = getenv("KANSO_COUNTERS") != NULL;
        ...

`k_alloc` is `always_inline` and it is inlined into every hot caller there is,
so every loop that allocates carried a WRITE to that global — and a loop that
writes a global cannot have any read of that global hoisted out of it. Every
counter check in every hot loop reloaded it and re-tested it once an iteration,
on release runs where the counters are off and the answer never changes for the
life of the program. A constructor sets it before main, the lazy line goes, and
LICM hoists the rest without being told anything.

All thirteen work rows fall. This vein has not recorded a clean sweep before.

    work_scanbench     776,364,842 ->   768,876,199   -7,488,643   -0.9646%
    work_digestbench    77,353,320 ->    76,854,629     -498,691   -0.6447%
    work_deepbench     708,507,318 ->   705,892,821   -2,614,497   -0.3690%
    work_jsonbench   1,564,492,424 -> 1,559,466,178   -5,026,246   -0.3213%
    work_oneshot        24,341,969 ->    24,300,109      -41,860   -0.1720%
    work_widebench      54,690,359 ->    54,610,292      -80,067   -0.1464%
    work_readbench       4,288,131 ->     4,283,685       -4,446   -0.1037%
    work_encodebench 4,425,477,605 -> 4,421,003,600   -4,474,005   -0.1011%
    work_livebench   4,436,935,725 -> 4,432,486,910   -4,448,815   -0.1003%
    work_basket         35,510,217 ->    35,477,286      -32,931   -0.0927%
    work_escapebench   114,596,730 ->   114,584,648      -12,082   -0.0105%
    work_indexbench      4,691,365 ->     4,691,237         -128   -0.0027%
    work_pendbench     605,537,209 ->   605,526,497      -10,712   -0.0018%

Machine code shrinks with it, 1,263,818 bytes to 1,233,770, a fall of 30,048 or
2.38%, on every one of the thirteen: each inlined copy of `k_alloc` carried a
getenv call and a second test, and every copy of that goes. The emitted-code
vein does not move at all, which is the check that this is the runtime rather
than the compiler: `src/runtime.c` changes what a program links, not what the
emitter writes.

**It also fixes the counted run.** Every counting site tests `k_stats_on > 0`,
and -1 fails that test, so anything that ran before the switch turned positive
was never counted at all — and under the lazy form it stayed at -1 until the
first inlined `k_alloc` body reached the getenv. Seven counters move, every one
of them upward, which is the evidence for what they are: basket_allocs 28,169,
basket_alloc_bytes 4,900,753, basket_bytes_malloc 31, basket_sh_str 622,320,
pend_sh_buf 32,134,736, escape_sh_buf 96,000 and scan_sh_buf 24,105,200. One
allocation and one byte malloc at startup on basket, sixteen bytes of shared
string with them, and thirty-two to forty-eight bytes of shared buffer on the
other three. Small, and silently missing for as long as the switch has been
lazy.

The `.mem` vein carries it too: 45 of its files move, on `allocs`,
`alloc_bytes`, `bytes_malloc`, `sh_str`, `sh_rec`, `sh_buf` and `sh_bytes`, and
every one of them upward. `builder_counts_once` is the shape of all of them —
allocs 10 -> 11, alloc_bytes 22,557 -> 22,590, bytes_malloc 7 -> 8, sh_str
32 -> 48 — one startup allocation and sixteen bytes of shared string that the
lazy switch never saw.

The gate keys that moved with them, so the sweep has them by name:
a_builder_handed_on_is_still_a_builder_alloc_bytes,
a_builder_handed_on_is_still_a_builder_allocs,
a_builder_handed_on_is_still_a_builder_bytes_malloc,
a_builder_handed_on_is_still_a_builder_sh_str,
a_cluster_entered_by_a_tail_call_sweeps_sh_str,
a_digest_holds_every_block_it_walked_sh_bytes,
a_pushed_call_keeps_the_sweep_sh_buf,
a_repaired_node_below_the_mark_holds_tenure_sh_buf,
an_escaped_list_gives_its_buffer_back_sh_buf,
an_unasked_equality_stays_a_cell_sh_str, append_in_place_sh_bytes,
beat_builder_sh_bytes, beat_cycle_sh_bytes, build_cycle.imported_sh_buf,
builder_counts_once_alloc_bytes, builder_counts_once_allocs,
builder_counts_once_bytes_malloc, builder_counts_once_sh_str,
builder_guard_sh_str, builder_reclaim_sh_bytes, builder_transient_sh_bytes,
early_exit_sh_buf, effect_push_shape_sh_buf, fold_push_shape_sh_buf,
force_path_sh_str, fresh_builder_sh_bytes, fresh_cycle_sh_bytes,
fused_map_shape_sh_buf, fused_reducer_sh_buf, fused_select_shape_sh_buf,
fused_tally_sh_buf, lazy_verdict_is_per_arm_sh_rec, many_cells_sh_str,
piped_reducer_sh_buf, record_fields_sh_buf, record_reuse_shape_sh_rec,
returned_thunk_sh_str, reuse_guard_sh_rec, shared_twice_sh_str,
skip_shape_sh_buf, skip_unused_sh_str, skipped_err_sh_str,
sort_shape_sh_buf, stream_fold_sh_str, stream_write_sh_bytes,
string_builder_shape_alloc_bytes, string_builder_shape_allocs,
string_builder_shape_bytes_malloc, string_builder_shape_sh_str,
string_headers_sh_buf, take_shape_sh_buf, tally_shape_sh_buf,
the_same_capture_built_below_the_mark_is_shared_sh_buf, unsafe_wrap_sh_buf.

The book carries the same correction in two places. `ch10/counters_counters.out`
and `ch12/fused_counters.out` are counted runs, and both read `sh_buf=0` where
they now read `sh_buf=32`: `k_buf` adds to `k_stat_sh_buf` BEFORE the `k_alloc`
that used to flip the switch, so the first buffer a program allocated was the
one that never got counted. Both samples and both chapter panels are
regenerated here.

`!invariant.load` on the six prelude reads of the switch was tried on top of
this and is WORSE: jsonbench 1,559,465,765 -> 1,561,061,464 and livebench
+8.4M on the container, with everything else identical. It is dropped. The
write was the whole blocker; once it is gone LICM hoists without being told,
and telling it costs something at the sites where the hoist was not the
cheapest shape.

`compile_instructions` moves 41,886,863 -> 41,888,129, a rise of 1,266 or
0.003%, and it is layout from a file the compiler only carries: `src/runtime.c`
is `include_str!`'d into `src/main.rs`, so twenty-six lines of C the front end
never executes still shift the compiler's own bytes. `compile_allocs` and
`compile_peak_bytes` are byte-identical, as they were the last two times this
row moved on its own.

The thirteen work rows are PROJECTIONS — the golden is CI's and this container
reads a different glibc — so each is the golden plus the container's own A/B
delta, measured on one host from the repo root with both binaries in place.
Every other row here is exact. `src/runtime.c` is `include_str!`'d into the
compiler, so the compile veins move too and CI is the record for them.
## 2026-09-06 (twenty-first) — the blank byte walks the whole ladder, and splitting the function does not split the loop

`d_jsonbench/value_for_3'2` is 353,650,950 instructions on the merged decoder,
22.92% of jsonbench, over 2,713,950 calls. Twenty-seven of its 489
instructions — the loop at 0x2bb0-0x2c25 — run MORE than once a call:
125,707,350 instructions, **8.1473% of the benchmark** and 35.55% of the
function. The loop head runs 5,276,700 times, 1.94 a call.

It is the whitespace skip. bench/large.json is pretty-printed, so nearly every
value is preceded by a blank byte, and a blank byte is what `value_for` decides
LAST:

    2bb0  cmp  $0x4,%rdi          ; is the byte `none`
    2bb8  lea  -0x2b(%r11),%rdi   ; the jump table's range starts at 43
    2bbc  cmp  $0x1a,%rdi
    2bc0  ja   2bcb               ; 9, 10, 13 and 32 all miss it
    2bcb  cmp  $0x65,%r11
    2bd1  cmp  $0x100,%r11
    2bde  add  $-0x30,%r11        ; number_start?
    2be2  cmp  $0xa,%r11
    2bef  cmp  $0x2,%rdi          ; ws?
    2bf5  inc  %rcx               ; and only now, advance one byte

The four blank bytes sit below the table's range, so each one falls through the
table, both fallback compares and the digit test before `ws?` answers. `array_delim`
above keeps whitespace as its last arm for a reason the entry beside it gives —
one dispatch on one loaded byte does two jobs — and that reasoning is right for
a three-arm ladder. This one is nine.

### Two shapes, both declined

**Split the run out into its own function.** `value_blank` hands a blank byte
to a `value_run` that tests `ws?` and nothing else, and only the byte that ends
the run enters the ladder:

    fn value_run cs c p
      blank = ws? c
      if blank (value_run cs cs[p + 1] (p + 1)) (value_for c cs p)

The loop is **byte-identical** afterwards: the same 27 instructions, the same
125,707,350, the head still at 5,276,700. `value_run` and `value_for` are
mutually tail-recursive, so the emitter puts them in one cluster and the two
kanso functions share one emitted loop — the split cannot reach the machine
code. jsonbench reads 1,542,924,905 -> 1,540,668,605, and that −0.1462% is
`obj_key_start` and `array_step` moving under a different inlining, not the
loop. It costs 110 emitted lines in every module, front_end_visits 17,264 ->
17,318, and the compile veins with them.

**Make the four blank bytes arms of the dispatch.** Written as `fn value_for 9`,
`10`, `13` and `32`, they are entries in the jump table rather than a test
after it, and the table's range opens from 43..69 to 9..123. That is **worse by
0.5853%**: 1,542,924,905 -> 1,551,955,655, with `value_for` itself 353,650,950
-> 362,681,550. A table of 115 entries costs every byte that reaches it more
than the ladder cost the blanks, and the blanks are 1.94 a call against the one
real value.

So the 8.15% is not reachable by rearranging the library. What is left is a
builtin that answers "the first byte here that is not one of these four" in one
call, the way `find2` answers the quote-or-backslash question for the string
scan. That is a new primitive for one caller, and it is a separate question.

## 2026-09-06 (twenty-second) — the scalar validator's ascii bytes are too few to skip

`k_utf8_bad_scalar` is 38,820,450 instructions on the merged decoder, 2.52% of
jsonbench, over 203,700 calls — 190.58 apiece. Its byte loop runs 15.45 times a
call and 13.72 of those go down the ascii arm:

    if (b0 < 0x80) { i += 1; continue; }

`K_UTF8_SCALAR_MAX` is 32, so the arm serves every string of thirty-two bytes
or fewer that carries at least one high byte — the door answers a wholly ascii
run without coming here at all.

The obvious repair is the one the split scan took in #1269: read eight bytes,
test them against `0x8080808080808080`, advance eight when the mask is clear.
It is **worse by 0.2314%**: jsonbench 1,542,924,905 -> 1,546,494,665, and
`k_utf8_bad_scalar` itself 38,820,450 -> 42,390,150. The 3,569,700 the kernel
gains is the whole 3,569,760 the program gains.

The reason is the length. A string of thirty-two bytes or fewer, with a high
byte somewhere in it, leaves ascii runs shorter than eight between the high
bytes and near the ends, so `i + 8 <= len` rarely holds. Every ascii byte pays
the guard and almost none of them get the skip. The same word test that saved
97% of readbench in #1269 loses here, because there the runs were kilobytes
and here they are single digits.

### The harness was watched red first

`scripts/utf8_differential` extracts this function's text from `src/runtime.c`
at run time and checks it against an independently written reference. With the
skip in it reports 45,189,025 checks and 0 mismatches. Breaking the mask to
`0x8080808080808000` — which stops the test seeing a high bit in the word's
first byte — gives 1,816,277 mismatches and exit 1, the first at
`59 52 f4 5f 35 71 10 1e 32 66 6e 3f`. So the gate does cover this arm, which
is what made the measurement worth trusting.

Reverted.

### And the two scans over a string are not two walks

The natural next thought is that the decode reads every string twice — `find2`
looking for the closing quote or a backslash, then the validator — and that one
pass could do both. The profile says otherwise. Neither is a per-byte loop:

    k_b_find2_raw       65,892,000 over 2,521,500 calls   26.13 each, 26 instrs
    k_b_utf8_slice_raw  86,519,850 over 1,305,300 calls   66.28 each, 94 instrs

No instruction in either runs more than twice a call. `find2_raw` is SSE2 on
x86-64 and NEON on aarch64, sixteen bytes a step, so a short string is answered
in one step; `k_b_utf8_slice_raw` is the ascii door, and it reaches the scalar
validator on 25,650 of its 1,305,300 calls. What both rows measure is per-call
setup over a scan that is already vectorised or already short-circuited, so
there is no shared walk for a fused pass to save. The saving would be one
call's frame, not one pass over the bytes.

### The decode has no deep loop left

Counting, for every block over four per cent, how many of its instructions run
more than once a call:

    value_for_3'2     353,650,950   2,713,950 calls   130.3 each    27 of 489 loop
    obj_key_start_4'2 205,282,500   1,060,050         193.7          0 of 231
    str_run_4         122,966,100     265,950         462.4        107 of 271
    array_step_3'2    119,421,450     410,550         290.9         82 of 124
    string_at_4       104,138,254   1,571,250          66.3          0 of 156

`obj_key_start` and `string_at` are flat: every instruction exactly once a
call. Of the three that do loop, only `value_for`'s runs deep, and the entry
above closes it.

`array_step`'s head runs 3.48 times a call and `str_run`'s 3.57. The first is
`array_delim` skipping the comma, newline and indent between two elements —
the same whitespace shape as `value_for`, through a three-arm ladder rather
than a nine-arm one, at about thirty-five instructions a blank byte. The
second is one iteration per escape in an escaped string, about a hundred and
thirty instructions each: 2.57 `find2` calls, a utf-8 validation and an
append, which is the fused path doing the work it exists to do.

So what remains is per-call overhead and short loops earning their keep. A
later reading should not go looking for another `value_for`.

---

## 2026-09-06 (twenty-third) — a lambda that captures nothing is a link-time constant

`w_klam17` is 712,277,200 instructions of encodebench, 16.11%, over 11,658,800
calls at 61.09 apiece. It is the escape fold's lambda — `(a b -> esc_byte a b)`
in `lib/json/text.kso`, one call per byte of every string that carries an
escape. Every one of those calls comes from `encode_onto`, because `list/fold`
and `escape_able` are both inlined into it; the closure call is the only
indirection left.

Joining the callgrind profile to the disassembly instruction by instruction:

    69a0  push %rbp / %r15 / %r14 / %r13 / %r12 / %rbx    1.00 a call
    69aa  sub  $0x18,%rsp                                 1.00
    ...
    6b0d  the b >= 32 arm, 24 instructions                0.843
    6c05  mov  %r15b,(%rsi,%rax,1)   ; the byte           0.843
    7082  three movs, add rsp, six pops, ret              1.00

Fifteen of the sixty-one are the frame: six callee-saved pushes, the stack
adjustment, and their mirror on the way out. 84.3% of the calls take the arm
that stores one byte.

### Why the call could not be resolved

`k_call2_fast` reaches the closure through `%fnp = load ptr, ptr %c`, and `%c`
came from `k_closure_lit`, which fills a mutable global on first visit. LLVM
cannot know what is in it, so the tag test, the arity test and the call itself
all stayed. The emitted code says the value never changes: a lambda with no
captures is the same closure every evaluation, which is why the cell existed.

So the emitter writes the closure as a module constant instead:

    @klam17_cell_env = internal constant %KValue zeroinitializer
    @klam17_cell_clo = internal constant { ptr, ptr, i64, i64 }
                       { ptr @w_klam17, ptr @klam17_cell_env, i64 0, i64 2 }
    @klam17_cell     = internal constant %KValue
                       { i64 11, i64 ptrtoint (ptr @klam17_cell_clo to i64) }

and the site loads it.

**What actually folds, checked against the shipped binary rather than assumed.**
The K_CLOSURE tag test folds and goes. The address folds: where the baseline
loaded the closure pointer from a stack slot, `mov 0x38(%rsp),%r10`, this one
writes `lea @klam17_cell_clo,%r10`. Two of the program's three call sites
become `call 6810 <w_klam17>`.

The hot one does not. At 0x61fb, the site the escape fold reaches 11,658,800
times, the emitted code still reads

    61e4  lea   0x23a65(%rip),%r10   # klam17_cell_clo
    61eb  mov   0x8(%r10),%rdi       ; the env, loaded
    61fb  call  *(%r10)              ; the fn, loaded

and the arity is still re-read at 0x6214, `cmpq $0x2,0x18(%rcx)`, from that
same constant. LLVM resolved the ADDRESS of a constant global and then declined
to constant-fold three loads out of it. The payload crosses as an i64 by the
KValue ABI, so the pointer reaches the load as `inttoptr(ptrtoint(@g))`; the
two cold sites fold through that and this one does not. Why they differ is not
established here, and this entry does not guess.

`k_deep_copy`'s in-place arm gains `if (cl->ncaps == 0) break;`. It used to
memcpy a one-slot env and write `cl->env` back into the header when the env did
not survive, which is a store into `.rodata` now. A closure over nothing holds
no arena pointer, so there was nothing to evacuate either way.

### What it bought

    encodebench   4,421,026,939 -> 4,390,891,562   -0.6816%
    jsonbench     1,542,924,905 -> 1,542,924,537   -368

`livebench` is the other program that runs this fold, over `lib/json` rather
than the frozen snapshot, and it reads 4,400,130,843 here against CI's golden
of 4,432,419,027 — a fall of 32,288,184, 0.728%. That comparison crosses hosts,
which is worth 23,339 instructions on encodebench, 0.0005%. `basket` falls
4,090 and `pendbench` 7,757, both at the noise of that offset: eight
capture-free lambdas in basket and five in pendbench, none of them in a loop.

`w_klam17` itself does not move: LLVM declines to inline 240 instructions of
jump table into a 606-instruction loop, so the frame is still paid. The whole
of the encode fall is `encode_onto`, 1,730,978,829 -> 1,703,308,420, and joined
instruction by instruction it is three things rather than a devirtualization:
the `k_closure_lit` call and its first-visit branch leave the loop, the
accumulator's tag test folds, and the loop stops reloading the list pointer
from `0x50(%rsp)` every iteration because the frame has a register to spare —
`sub $0x188,%rsp` becomes `sub $0x178`. The loop body is 33 instructions a byte
where it was 34. The decoder has one capture-free lambda and it is not in a
loop.

`perm_allocs` falls in all ten cost goldens — two allocations per capture-free
lambda, the KClosure and its one-slot env, now in `.rodata`. Emitted calls fall
in every program: the decoder 1,847 -> 1,845, encodebench 1,648 -> 1,646,
basket 1,278 -> 1,270. Three constants replace one call, so `lines` falls where
the program has few such lambdas and rises slightly where it has many.

The fifteen frame instructions are still there, and #290 is what would take
them: `preserve_none` on the wrapper, blocked on LLVM 19.

### CI's rows, and the one that rose

    encodebench   4,421,003,600 -> 4,390,892,021   -0.6811%
    livebench     4,432,419,027 -> 4,400,131,256   -0.7285%
    oneshot          24,190,898 ->     24,109,317  -0.3373%
    deepbench       705,892,821 ->    704,511,486  -0.1957%
    digestbench      76,854,629 ->     77,175,692  +0.4177%

Eleven of the thirteen fall. escapebench and indexbench rise by 28 each, which
is one closure built once instead of a first-visit branch taken once.
digestbench rises 321,063, and its `.text` rises 192 bytes over the same
change while its emitted lines fall by seven — the direct call changes what
LLVM inlines there, downstream of anything the emitter wrote. Welfare weighs
all thirteen and reads 75.30 -> 75.31, so the trade is taken. `compile_instructions`
rises 2,728, 0.0065%, which is what any edit to src/codegen.rs costs.

### And the wrapper still cannot be inlined

The direct call raises the obvious next question: `w_klam17` has one hot caller
now, so mark it `alwaysinline` and let the loop swallow it. Built and measured:
encodebench reads **4,390,891,562, byte-identical**. The IR changes — the
wrapper is inlined into `encode_onto` and the tailcc body is called from there,
rather than the body being inlined into the wrapper — and the machine
instruction count does not move at all, because the frame is paid either way.
Reverted. The fifteen instructions are #290's.

### The four that rose, by the keys the gate reads

    work_digestbench       76,854,629 ->  77,175,692   +0.4177%
    work_escapebench      114,584,648 -> 114,584,676   +28
    work_indexbench         4,691,237 ->   4,691,265   +28
    compile_instructions   42,089,618 ->  42,092,346   +0.0065%

`work_escapebench` and `work_indexbench` gain 28 apiece, which is one closure
built once at link time instead of a first-visit branch taken once at run time.
`compile_instructions` is what any edit to src/codegen.rs costs: the emitter is
the compiler, so its own bytes and the layout under them move whether or not
the decision this row counts changed. `work_digestbench` is the one with no
account: its emitted lines FALL by seven over the same change while its `.text`
rises 192 bytes, so the 321,063 is a choice LLVM made downstream of the direct
call at digestbench's two cold sites, and this entry does not guess further.
Welfare weighs all thirteen work rows and the three compile rows together and
reads 75.30 -> 75.31, so the corpus is ahead and the trade is taken.

### What is left, priced

`encode_onto`'s sixty spine instructions — the ones that run on essentially
every one of its 10,581,600 calls — are 672,571,200 instructions, **15.32% of
encodebench**. Fifteen of them are the frame: six callee-saved pushes, a
376-byte stack adjustment, and their mirror on the way out, 158,724,000
instructions or 3.61%. #338 declined outlining the arm that sizes that frame at
+2.5582%, so the shape is known and priced.

The rest is the fold loop, which runs 1.102 times per `encode_onto` call and
carries the three unfolded loads above. Two of them are the closure's fn and
env, which nothing about this program can change at run time; a third is the
arity. That is three loads and a compare, 46.6M instructions, 1.06% of
encodebench, sitting behind an `inttoptr(ptrtoint(@g))` the optimizer resolves
for the address and not for the contents.

---

## 2026-09-06 (twenty-fourth) — the fold asks the length twice, and the tag test is why

`encode_onto`'s escape fold reads the same header field twice per byte. The
loop, at 0x62d2 in the shipped binary:

    62d2  cmpq  $0xd,0x40(%rsp)   ; is the collection bytes?
    62da  mov   (%r15),%rcx       ; the length, load one -- `length coll < i`
    62dd  cmp   %r14,%rcx
    62e7  ...   call k_b_length   ; the other arm
    6324  cmp   %r14,(%r15)       ; the length, load two -- `coll[i]!`

Both are `cmp %r14,(%r15)`, the identical comparison against the identical
address, and nothing between them writes memory on the fast path. GVN does not
forward the first to the second because the `k_b_length` call arm merges
between them, and the merge is the tag test: `k_b_length_fast` inlines a header
load for a list or a bytes and calls the C entry for anything else.

**What the second compare is worth: 1.4650%.** Both emission sites in the
demanded-index guard were neutralised in turn, which is unshippable and prices the guard:

    the whole guard removed     4,390,891,562 -> 4,324,668,394   -1.5081%
    only `idx <= len` removed   4,390,891,562 -> 4,326,560,692   -1.4650%
    only `idx >= 1` removed                                      -0.0431%

The `>= 1` half is nearly free because LLVM proves it from the induction
variable; the upper bound and the load under it are the whole cost. #349
already declined folding the two signed compares into one unsigned one -- the
signed pair is what buys `movzbl -0x1(%rax,%rbp,1)`, with the `-1` in the
address -- so this thread is about asking the same compare once rather than
about asking fewer of them.

**The static proof does not reach it.** The emitter has a `proven` path for the
demanded index when `set_of(container) == BYTES`, and `length` had none, so a
`length` of a value already proved bytes went through the twin's tag test for
nothing. The emitter writes the header load directly now, and every row falls
or holds:

    encodebench  4,390,891,562 -> 4,389,081,554   -1,810,008   -0.0412%
    livebench    4,400,130,843 -> 4,399,421,576     -709,267   -0.0161%
    oneshot         24,108,858 ->     24,107,081       -1,777   -0.0074%
    jsonbench    1,542,924,537 -> 1,542,924,177         -360
    widebench       54,609,406 ->     54,609,398           -8
    digestbench     77,175,233 ->     77,175,230           -3

Six fall and the other seven are byte-identical -- basket, deepbench,
pendbench, escapebench, indexbench, scanbench and readbench have no `length`
site the sets prove. Nothing rises.

**And it does not touch the fold.** `list/fold_flat` is one function for lists
and bytes both, so its `coll` carries the union and neither its `length` nor
its index gets the proof -- the tag test above is the run-time answer to a
question the call site already knew. The fold is inlined into `encode_onto` and
the back edge at 0x63f3 makes a machine loop of it, but the recursion is
`musttail`, so LLVM sees no loop and LICM never runs: the tag test, the length
load and the bound are paid on every byte.

That names the next thing rather than doing it. `coll` is passed unchanged to
every self-call, which makes it invariant across the cycle by construction, and
an emitter that specialised on an invariant parameter's tag once at entry would
collect the 1.4650% and the tag ladder with it.

The two emitted-code veins move in opposite directions and the trend gate wants
both named. `emitted_calls` falls 1,845 -> 1,843 and `emitted_other_calls`
15,823 -> 15,814, because a call to the twin leaves each site. `emitted_lines`
rises 12,708 -> 12,716 and `emitted_other_lines` 104,496 -> 104,532, because
four IR lines take its place: an `inttoptr`, a `getelementptr`, a `load` and an
`insertvalue`. Four lines a call is the trade, and `text` falls 1,233,802 ->
1,233,434 over the same change, so what the linker kept is smaller than what
the emitter wrote. `defines` and `branches` hold in every program.

CI's own sitting, which is the one the goldens hold, agrees with the container
on every sign and on four of the six magnitudes to the instruction:

    encodebench  4,390,892,021 -> 4,389,082,013   -1,810,008   -0.0412%
    livebench    4,400,131,256 -> 4,399,422,049     -709,207   -0.0161%
    oneshot         24,109,317 ->     24,107,540       -1,777   -0.0074%
    jsonbench    1,542,924,950 -> 1,542,924,650         -300
    widebench       54,609,879 ->     54,609,871           -8
    digestbench     77,175,692 ->     77,175,689           -3

basket, deepbench, escapebench, pendbench, indexbench, scanbench and readbench
hold to the instruction. Welfare 75.31082442642723 -> 75.31169684664573,
banked with `--set`.

`compile_instructions` FALLS, 42,092,346 -> 42,091,852. CLAUDE.md's rule is that
this row moves on any edit to the compiler's own Rust and usually upward,
because src/codegen.rs is the compiler and the layout under its bytes moves with
them; #1275 paid 2,728 for a smaller diff than this one. It falls here for a
reason the diff shows: the proven-bytes arm returns before the generic builtin
path builds `args_ir`, a Vec of formatted Strings one per argument, and before
it collects the argument sets `infer::builtin_set` reads. The emitter writes
four IR lines where it wrote one call and does less work deciding to.

---

## 2026-09-06 (twenty-fifth) — the fold's container cannot be proved from the library, and the beat is why

The twenty-fourth entry left the 1.4650% behind one wall: `list/fold_flat` is
one function for lists and bytes both, so its `coll` carries the union and the
emitter's proven-bytes paths cannot fire. The cheapest way to test whether that
union is the whole story is to give the escape fold a container the sets CAN
prove — a fold of identical shape, in `lib/json/text.kso`, called only with
bytes:

    fn escape_able acc bs
      esc_flat bs acc (a b -> esc_byte a b) 1

    fn esc_flat bs acc f i
      if (length bs < i) acc (esc_flat bs (f acc bs[i]!) f (i + 1))

The lambda is kept deliberately. #313 declined removing the escape fold's
closure at livebench +3.01% and named the beat as the reason, so a version
calling `esc_byte` directly would have re-run a declined experiment and
confounded this one. Same closure, same arity, same body shape; the only thing
that changes is which function the fold is.

BUILT, MEASURED, DECLINED. Both programs stay correct (`wrote 74072800`,
`checksum 24000`) and the cost is not close:

    livebench    4,399,421,576 -> 4,740,164,338   +340,742,762   +7.74%
    oneshot         24,107,081 ->     25,619,336     +1,512,255   +6.27%
    jsonbench    1,542,924,177 -> 1,542,923,800           -377

The live encode counters say what happened, so this is attributed rather than
guessed:

    allocs          9,233,103 ->  51,551,103    5.6x
    alloc_bytes   710,726,112 -> 2,064,935,712  2.9x
    sh_bytes      100,713,384 -> 1,116,345,384   11x
    arena_peak_bytes  2,097,152 ->   7,340,032
    beat_iters      5,032,401 ->         401

The beat stopped. `beat_iters` falls by four orders of magnitude and the
allocations it was reclaiming become real ones.

**This isolates a variable #313 could not.** That entry removed the closure and
respelled the fold together, and attributed its +3.01% to the closure. Here the
closure is untouched and the beat collapses anyway, so a lambda's presence is
not what the beat turns on.

**And the beat that dies is not the fold's.** `KANSO_BEAT_REPORT=1` gives the
same verdict for the fold in both shapes:

    beat: list/fold_flat/4: grow-only: another group tail-calls it
                            (unbracketed entry) (argument 2 also carries heap)
    beat: json/esc_flat/4:  grow-only: another group tail-calls it
                            (unbracketed entry) (argument 2 also carries heap)

Grow-only both times: the fold never had a beat to lose. What the report shows
moving is two functions the change does not touch:

    json/encode_items/3   beat: rewinds every iteration
                       -> grow-only: argument 1 may carry heap across the iteration
    json/encode_pairs/3   beat: rewinds every iteration
                       -> grow-only: argument 1 may carry heap across the iteration

The encoder's item and pair loops are where livebench's five million beat
iterations were, and respelling the fold two levels below them changes what the
analysis concludes about their accumulator. **An earlier revision of this entry
said the beat depends on which function the fold is and pointed at the
`imported`/`carried` retains in `src/beat.rs`. That was wrong on both counts,
and reading the report rather than the source is what corrected it.**

The entry hop was tested too, since `escape_able -> list/fold -> fold_flat` has
one more call than `escape_able -> esc_flat`. Mirroring it exactly --
`escape_able -> esc_fold -> esc_flat` -- changes nothing: `beat_iters` 401 and
`allocs` 51,551,103 again, to the instruction. So the depth of the entry is not
it either. What remains is the accumulator's provenance, and the report names
the conclusion (`argument 1 may carry heap`) without saying which step reached
it; that is not established here.

What it settles for the invariant-parameter thread: the library route is
closed. A fold respelled where the sets can see it costs two loops above it
their beats, and more than the proof was ever worth, so the 1.4650% has to be
collected in the EMITTER with one fold, or not at all. It also sharpens the
caution for that work: a specialisation that clones a fold has to be watched at
its CALLERS, because this cost landed two levels up from the edit and the
instruction row alone would have said only "+7.74%, unexplained".

---

## 2026-09-06 (twenty-sixth) — the fold reads its length once, and the guard is three instructions shorter

The twenty-fourth entry ended by naming what it had not done: `coll` is passed
unchanged to every self-call of `list/fold_flat`, so its length cannot change
across the tail cycle, and an emitter that knew that would stop re-deriving it.
This does that by hand, in the library, to price it:

    pub fn fold coll init f
      fold_flat coll init f 1 (length coll)

    fn fold_flat coll acc f i len
      if (len < i) acc (fold_flat coll (f acc coll[i]!) f (i + 1) len)

Seven of the thirteen work rows fall, six hold, none rises. Container numbers;
CI's sitting is the one the goldens hold.

    encodebench  4,389,081,554 -> 4,328,659,954   -60,421,600   -1.3766%
    livebench    4,399,421,576 -> 4,338,999,976   -60,421,600   -1.3734%
    digestbench     77,175,230 ->     75,582,016    -1,593,214   -2.0644%
    oneshot         24,107,081 ->     23,956,027      -151,054   -0.6266%
    deepbench      704,512,368 ->    702,628,368    -1,884,000   -0.2674%
    basket          35,473,136 ->     35,403,114       -70,022   -0.1974%
    pendbench      605,518,680 ->    605,515,280        -3,400   -0.0006%

jsonbench, escapebench, indexbench, scanbench, readbench and widebench are
identical to the instruction. The decoder compiles lib/list — lib/json/text.kso
imports it — and never calls the fold.

**The beat was checked before anything else.** #1277 wrote a mechanism out of
`src/beat.rs` and `KANSO_BEAT_REPORT=1` refuted both halves of it, so the
report runs first now. Its diff against the base is one line:

    - list/fold_flat/4: grow-only: another group tail-calls it ...
    + list/fold_flat/5: grow-only: another group tail-calls it ...

Same verdict, one more parameter in the name. `json/encode_items/3` and
`json/encode_pairs/3` — the two that lost their beats in #1277 and cost 7.74%
— keep them here, and every allocation counter in all eleven cost goldens is
byte-identical.

### Where the 60,421,600 are

Two binaries built from the same tree, run under callgrind. Every function in
the binary is identical in retired instructions except `encode_onto`, which
falls 1,711,836,829 -> 1,651,415,229: exactly the whole delta. Every call
count is identical too — `w_klam17` 11,658,800, `k_b_length` 400,
`k_beat_iter` 5,032,000.

`--dump-instr=yes` puts the rest on two addresses. The loop guard, at the back
edge, executed 12,368,000 times:

    base                              hoisted
    cmpq  $0xd,0x40(%rsp)             cmp  0x20(%rsp),%r14
    jne   62e7                        jg   6272
    mov   (%r15),%rcx
    cmp   %r14,%rcx
    jge   6310

Five instructions become two. Those five are `k_b_length_fast` inlined: a tag
test, a header load, and the comparison the fold asked for. 3 x 12,368,000 =
-37,104,000.

The loop body, executed 11,658,800 times, goes from 28 instructions to 26. The
two that leave are both

    lea  0x238ba(%rip),%r10   # klam17_cell_clo

— the base re-materialises the closure's address on each arm of every
iteration, and the hoisted loop holds it in `%rbx` for the whole fold.
2 x 11,658,800 = -23,317,600. The two together are -60,421,600, the measured
number, with nothing left over.

**The hypothesis this replaced was wrong, and worth writing down.** I expected
the win to be GVN forwarding the second load once the twin's call arm stopped
merging blocks between them — the mechanism the twenty-fourth entry described.
It is not. `k_b_length` is called 400 times in both binaries, so the call arm
never ran and there was no merge to defeat; the loop simply stopped asking for
the length. Reading the addresses took ten minutes, and without them the guess
would have been written here as the mechanism.

### What it costs

`fold_flat` carries a fifth parameter, so its declaration and both edges of its
tail cycle each take one more argument, and every program that imports std/list
pays the same ten IR lines. The trend gate wants each named with where it
landed: `emitted_calls` 1,843 -> 1,845, `emitted_branches` 1,203 -> 1,204,
`emitted_lines` 12,716 -> 12,726, `emitted_other_calls` 15,814 -> 15,832,
`emitted_other_branches` 9,981 -> 9,990, `emitted_other_lines` 104,532 ->
104,622. escapebench, indexbench and readbench do not import std/list and hold
to the line; `defines` holds everywhere.

A ninth vein carries the same fifth parameter and I missed it on the first
round, so CI found it: `bench/compile_golden_modules.txt`, which
`tests/compile_cost.rs` reads and which `scripts/gates/all_compile.sh` does not.
`module_calls` 753 -> 755, `module_branches` 430 -> 431, `module_lines` 5,176 ->
5,186 and `module_visits` 2,403 -> 2,409; `rounds` and `defines` hold. It is the
same +2, +1, +10 as the emitted goldens plus six more expression visits, and the
`module` sample is the only row in that pair of files that imports anything, so
the five samples in `bench/compile_golden.txt` hold. **The sweep script is not
the list.** `all_compile.sh` names six gates and there is a seventh vein behind
a cargo test; regenerate it with `KANSO_REGEN_COMPILE_GOLDEN=1 cargo test --test
compile_cost` whenever lib/ moves.

`text` FALLS, 1,233,434 -> 1,233,354, and the rows inside it disagree: five
fall, two rise, six hold, with pendbench +864 and deepbench +144 against
digestbench -368 and scanbench -192. Ten more IR lines in every one of them and
the bytes go both ways, so which way is downstream of the emitter — the same
finding digestbench's row made under the capture-free lambda.

The front end pays too, and all four of its rows are CI's own sitting, copied
out of the round-one job log: `front_end_visits` 17,264 -> 17,290, twenty-six
more expression visits for one more name; `compile_peak_bytes` 724,493 ->
725,365, a rise of 872 or 0.12%; `compile_allocs` 25,899 -> 25,913;
`compile_instructions` 42,091,852 -> 42,117,183, a rise of 25,331 or 0.0602%.
`front_end_rounds` holds at 42. `lib/*.kso` is `include_str!`'d into the
compiler, so twelve lines of library edit are twelve lines the compiler carries
and compiles.

The container read `compile_peak_bytes` as 725,365 too, to the byte, on
rustc=1.94.1 against the golden's 1.98.1 — one more sitting for #1271's finding
that the two hosts agree on that row, from a pair of toolchains that have not
agreed on it before.

**Welfare 75.31169684664573 -> 75.3427479384712**, a rise of 0.031, banked with
`--set`. The runtime falls buy the compile rises comfortably: runtime satiates
late and compile early, which is the trade the weights exist to price.

CI's thirteen work rows agree with the container on every sign and on all seven
magnitudes to the instruction:

    encodebench  4,389,082,013 -> 4,328,660,413   -60,421,600   -1.3766%
    livebench    4,399,422,049 -> 4,339,000,449   -60,421,600   -1.3734%
    digestbench     77,175,689 ->     75,582,475    -1,593,214   -2.0644%
    oneshot         24,107,540 ->     23,956,486      -151,054   -0.6266%
    deepbench      704,511,486 ->    702,627,486    -1,884,000   -0.2674%
    basket          35,473,609 ->     35,403,587       -70,022   -0.1974%
    pendbench      605,519,153 ->    605,515,753        -3,400   -0.0006%

While regenerating that golden: **`scripts/gates/compile_memory.sh` told a
reader that rounds and visits are welfare terms, and they have not been since
the 2026-09-03 rebuild.** `bench/objective_sources.txt` weighs
`compile_instructions`, `compile_allocs` and `compile_peak_bytes` and nothing
else about the front end. The message is corrected to say what those two rows
actually are: counters of the compiler's own algorithm, the same on every host,
which is why they are checked everywhere and the peak row is not. This is the
same stale-prose family as the CLAUDE.md sentence corrected earlier today,
which named fixpoint rounds and expression visits as objective terms for three
days after they stopped being any.

### The same edit on the encoder's pair, BUILT, MEASURED, DECLINED

A crude census of lib/ — a column-0 `fn` reader, so it sees single-line headers
and misses pattern-parameter arms — finds 19 self-recursive declarations of 617,
and exactly three (declaration, parameter) pairs that are invariant across the
self-calls AND take that parameter's length in the body: `json/encode_items`
`xs`, `json/encode_pairs` `es`, `list/drain` `xs`. `fold_flat` has left the list
because it no longer asks.

Both remaining shapes were built. The beat report ran first each time and the
beats SURVIVE in both — `encode_items` and `encode_pairs` still read `beat:
rewinds every iteration`, which is what #1277 destroyed and what made that
change cost 7.74%. This is a different thing, and it is worse anyway:

    with an entry hop, as fold has     livebench +9,900,860   +0.2282%
                                       oneshot      +24,812   +0.1036%
    hoisted to the two callers, which
    already hold the container         livebench +4,382,460   +0.1010%
                                       oneshot      +11,016   +0.0460%

Removing the hop halves the damage and does not turn it positive. Declined in
both shapes.

### What decides it: iterations per entry

The saving is per ITERATION — three instructions off a loop guard — and the
price is per ENTRY: one more argument at the call, and a `length` the caller now
takes through the twin's tag test because nothing proves what it holds. The
densities are measurable and they are far apart:

    list/fold over the escape bytes   1,773 entries   29,147 iterations  16.44
    json/encode_items over large.json's arrays   2,752   9,732 elements   3.54
    json/encode_pairs over its objects           2,761   8,361 entries    3.03

At sixteen iterations an entry the hoist is worth 1.3766%. At three it costs
0.1010%. **So an emitter pass that hoisted every invariant length would make
most sites worse.** The invariance is the cheap half of the question; the trip
count is the half that decides, and the compiler cannot see it.

### What is still on the table

The emitter-level pass, priced now rather than assumed. A hidden extra parameter
is genuinely required, because there is no IR loop in a musttail cycle to hoist
out of — what the entry on #346 found — so the pass changes a dispatch group's
signature and every external caller of it. Two constraints found while looking:
the predicate has to key on the GROUP rather than one arm, and self-calls miss
mutual cycles, of which `bounded_flat -> bounded_more -> bounded_step` in
lib/list/list.kso is one. `src/linear.rs:811` is the precedent to copy. Whether
it is worth building at all now turns on the break-even above: the shapes it can
help are the ones already entered through a wrapper, where the per-entry cost is
paid before the pass arrives.

## 2026-09-06 (twenty-seventh) — three veins a sweep could not see, and one it could not have graded

Both sweeps derive their lists from the gate SCRIPTS on disk, so a golden read
by a cargo TEST is outside the derivation and outside the spec that replays it.
Three goldens are in that position. `all_compile.sh` gave a wrong answer twice
today, once for that reason and once for a different one, and the runtime sweep
has had the same blind spot the whole time in the vein CLAUDE.md names first.

### The golden with no gate script

The sweep derives its list from the scripts under `scripts/gates` that name a
compile-side golden, and `tests/the_compile_sweep_names_every_compile_gate.rs`
replays that derivation, so a gate added later is a red spec rather than a vein
nobody sweeps. Two goldens sit outside the derivation entirely:
`bench/compile_golden.txt` and `bench/compile_golden_modules.txt` are read by
`tests/compile_cost.rs`, and no file under scripts/gates names either one. The
twenty-sixth entry's library change moved the modules vein — lines 5,176 →
5,186, calls 753 → 755, branches 430 → 431, visits 2,403 → 2,409 — the sweep
reported nothing had moved, and the specs job went red a round later.

Enumeration settles which goldens are affected: of the nine compile-side
goldens on disk, exactly two have no reader among the gate scripts, and they
are that pair.

`all_compile.sh` now runs `cargo test --release --test compile_cost` as a step
named by hand, with a comment saying why it cannot be derived like the others.
The spec widens from "every compile-side gate script is swept" to "every
compile-side golden on disk is read by something the sweep runs" — a property
of the tree rather than of the script's own list. Watched red first, naming
`["compile_golden.txt", "compile_golden_modules.txt"]`.

The new step's red is not hypothetical. `--test compile_cost` is the one target
that failed on rounds two and three of #1278, on linux and on macos arm both,
which is how the missing vein was found at all — the step runs exactly that
target, so a moved pair turns the sweep MOVED for the same reason CI turned red.

### The artifacts nobody built

The widened sweep then reported `machine_code` and `emitted_code` MOVED on a
tree byte-identical to HEAD: +1 define, +1 call, +3 branches and +46 lines on
all twelve programs at once. A uniform move across programs with nothing in
common points at the runtime preamble every one of them carries.

The gates read `*.ll` and the linked binaries out of the working directory, and
not one of them produces those files. The `.ll` files were timestamped 12:01,
written while `src/codegen.rs` still held an unshipped `k_b_append_mut_int`
twin; the revert landed at 12:03. The +46 lines were that twin. Running
`build_benchmarks.sh` and re-running both gates gave AGREED with nothing else
changed.

Reading artifacts newer than the source costs a round. Reading artifacts older
than the source is silent — the sweep says nothing moved after an edit that
moved a vein, which is the failure the script was written to prevent.
`all_counters.sh` opens with `cargo build --release` for exactly this reason,
and CLAUDE.md already carried the rule for a bare `kanso build`; the compile
sweep had never had it. `build_benchmarks.sh` is now its first line, pinned by a
spec watched red in both halves: absent, and present but after the gates.

### What the sweep reads now

    machine_code           AGREED
    emitted_code           AGREED
    compile_memory         REFUSED
    compile_allocs         REFUSED
    compile_instructions   REFUSED
    compile_libraries      AGREED
    compile_cost           AGREED

Seven readers over the nine compile-side goldens. The three refusals are the
host gate: this container is glibc 2.39-0ubuntu8.7 and rustc 1.94.1 against the
goldens' 8.8 and 1.98.1, and CI measures those three.

### The vein the runtime sweep never read

`all_counters.sh` has the same derivation and the same blind spot.
`tests/golden/mem/*.mem` — fifty-six goldens, the lazy tier — is read by
`tests/golden.rs`, so there is no `*_counters.sh` gate naming it and the veins
table pinned to that derivation could not carry a row for it. CLAUDE.md names
the .mem vein FIRST in the list a counter change must regenerate, and the sweep
that exists to run that list has never read it.

It runs it now, as a step named by hand, with `KANSO_REGEN_MEM_GOLDEN=1` under
`--write`. The spec asserts the property rather than the line: the sweep's own
text, plus the source of every `--test` target it runs, has to mention the vein.
Watched red first.

The step costs 158 seconds on this container. That is real beside the eleven
counter runs, and it buys the one dimension the file is named for.

### The one that looked like a fourth hole and is not

`bench/instructions_golden.txt` is read by `scripts/gates/instructions.sh` and
by neither sweep: `all_counters.sh` derives its table from `*_counters.sh`
scripts and that gate is not one, and it holds runtime rows so it has no place
in the compile sweep. The difference from the pair above is that a container
cannot grade it either way — `host_gate.sh` refuses it here, because the rows
are retired instructions and this glibc and rustc are not the goldens'. CI runs
the gate directly and its rows are copied out of the job log. A sweep row for it
could print REFUSED and nothing else, so it stays where it is.

No counter moved in this change. It touches `scripts/gates/all_compile.sh`,
`scripts/gates/all_counters.sh`,
`tests/the_compile_sweep_names_every_compile_gate.rs`,
`tests/every_counter_gate_is_in_the_sweep.rs`, CLAUDE.md and this file.

## 2026-09-06 (twenty-eighth) — the tags the sets already proved, and seven copies of one body

`esc_byte` dispatches on the byte before it appends it. By the time
`k_b_append_mut_byte`'s twin opens with `atag == 13` and `xtag == 0`, both
answers are already decided on the path that reached it, and LLVM will not
thread them away: the block is a merge that other predecessors reach with other
tags. Four instructions on 9,833,200 of encodebench's 11,658,800 escape-fold
iterations.

The emitter already knows both facts. `f.set_of(acc) == BYTES` and
`f.set_of(x) == INT` are exactly what the twin re-asks, so where the sets prove
them there is a second door that starts after the tag tests — `append_mut_int`,
the same body from the stats flag onward. The ownership test and the frontier
test still decide, and anything they refuse still falls through to the C path.

The second change is in the library. `esc_byte`'s seven int-literal arms each
spelled the same two-append body, so the lambda folded over a string carried
seven expansions of one thing. They share `esc_pair` now.

    change 1 alone   livebench −0.3843%   encodebench −0.2631%   oneshot −0.1738%
    + change 2       livebench −0.0948%   oneshot −0.0429%
    together         livebench −0.4787%

**encodebench does not move on change 2, and that is correct.**
`bench/encodebench/encodebench/` is a frozen copy of lib/json taken at 20ab931d
on 2026-08-07, so a lib/json edit cannot reach it. `bench/livebench` is the
benchmark that watches the shipped library, and it is the row to read for any
library-side change.

### The beat was checked first

Both benchmarks were reported on the patched tree and again on the base at
71c80472, sorted and diffed. `livebench` and `encodebench` are IDENTICAL, every
group and every verdict. Neither change moves a beat.

### No runtime counter moves at all

All eleven cost veins agree, and the lazy tier regenerates byte-identical.
That is the right shape: this removes instructions per iteration, not
allocations. The runtime evidence is `bench/instructions_golden.txt`, which
this container may not measure.

### What it costs, by which change reaches which program

The twin is written into the preamble every program carries, so it reaches all
thirteen without anything importing anything. `esc_pair` is in lib/json, so it
reaches only the three programs that import it.

    emitted_defines           185 ->       187      the decoder takes both
    emitted_calls           1,845 ->     1,846
    emitted_branches        1,204 ->     1,209
    emitted_lines          12,726 ->    12,801
    emitted_other_defines   1,811 ->     1,825
    emitted_other_calls    15,832 ->    15,844
    emitted_other_branches  9,990 ->    10,030
    emitted_other_lines   104,622 ->   105,232
    module_lines            5,186 ->     5,232
    module_calls              755 ->       756
    module_branches           431 ->       434
    module_defines             98 ->        99
    defines                   194 ->       199      compile_golden.txt, summed
    calls                     215 ->       220
    branches                  357 ->       372
    lines                   5,688 ->     5,918

Ten of the twelve in `emitted_golden_others.txt` take the twin alone: +1
define, +1 call, +3 branches, +46 lines. oneshot and livebench take `esc_pair`
as well: +2 defines, +1 call, +5 branches, +75 lines. So `esc_pair` on its own
is +1 define, +2 branches, +29 lines, and it is a fair trade — it buys the
front end 26 fewer expressions to visit.

`compile_golden.txt`'s five samples each move +46 lines, +1 call, +3 branches,
+1 define; rounds and visits hold. None of the five imports anything, which is
what makes them the clean reading of the twin's cost on its own.

Two rows FALL. `front_end_visits` 17,290 -> 17,264, because seven copies of one
body became one. And `text` is a wash: four of the thirteen binaries move by 32
bytes each and they split both ways — encodebench 112,546 -> 112,578 and
widebench 117,170 -> 117,202 rise, oneshot 116,034 -> 116,002 and livebench
116,642 -> 116,610 fall, nine hold. Net zero across the thirteen, which is the
linker placing the same work differently rather than more or less of it.

`compile_instructions`, `compile_allocs` and `compile_peak_bytes` are CI's —
this container is rustc 1.94.1 and glibc 2.39-0ubuntu8.7 against the goldens'
1.98.1 and 8.8 — and the welfare floor moves with them.

### CI's rows, from round one on 31537922

The four veins this container may not compare were measured by the runner and
copied in. Every one of them FALLS.

    encodebench           4,328,660,413 -> 4,317,272,013   -0.2631%
    livebench             4,339,000,449 -> 4,318,225,649   -0.4788%
    oneshot                  23,956,486 ->    23,904,549   -0.2168%
    compile_instructions     42,117,183 ->    42,061,345   -0.1326%
    compile_allocs               25,913 ->        25,862   -0.1968%
    compile_peak_bytes          725,365 ->       724,798   -0.0782%

The ten other work rows hold to the instruction. They are the decode and the
digest, and neither appends a byte the sets prove is an int nor imports
lib/json/text's escape arms.

The three compile terms falling together is the twin paying for itself twice.
The library half deletes six copies of a two-append body, so there is less of
lib/json to lex, parse, infer and check — that is `compile_allocs` and
`compile_peak_bytes`. `compile_instructions` takes that plus the layout move
any edit to src/codegen.rs makes, and this time the layout went the same way.

Welfare 75.3427 -> 75.36, floor ratcheted in the same commit.

### The sweep #1279 built caught its first move, on the next change

`compile_cost` went MOVED locally, before the push. That vein was structurally
invisible three hours ago: no file under scripts/gates named it, so neither the
sweep's derived list nor the spec replaying that derivation could see it, and
#1278 learned it existed only when `specs` and `the other host` went red on two
rounds. The step added to close that hole earned itself on the very next
change that moved it.

### Regenerating it deleted the note the last change left

`KANSO_REGEN_COMPILE_GOLDEN=1` writes the golden's original header plus the
measured rows and drops everything in between. The dated note b3024fb9 added to
`bench/compile_golden_modules.txt` this morning lasted three hours and vanished
on this regeneration. It was visible only because `git diff --stat` showed the
file SHRINKING by ten lines when it should have grown.

`scripts/gates/all_counters.sh --write` is written specifically to avoid this,
rewriting data rows line for line and leaving comments alone; the compile_cost
regen does the opposite and nothing said so. Both notes are restored by hand and
the asymmetry is written into the file. The fix belongs in the regen — either
rewrite in place, or refuse when there are comments it would drop — with a spec
that regenerates a golden carrying a note and asserts the note survives. That
property is testable and is currently false.

### The variance is unexplained

Sitting to sitting on this container the encode rows move by about sixty
instructions with no change to the tree. Two candidate causes were tested and
refuted: the change itself (reverting src/codegen.rs reproduced the same
numbers) and callgrind's argument length (`--callgrind-out-file=<path>` and
`=/dev/null` both gave escapebench 114,584,277). A third does not apply —
container versus runner, when both sittings were this container. The cause is
UNKNOWN, and it is written that way rather than guessed a third time, because
the first two guesses were asserted before they were tested and neither
survived.

The twenty-sixth entry carries the same ~60 on the encode-pair decline's two
figures (livebench +9,900,860, oneshot +24,812). Percentages are unaffected to
four decimals, and both readings were declines either way.

### What is still open, now with a number on it

28 of the escape lambda's 61 instructions a byte are frame and dispatch, about
7.5% of encodebench, and change 2 collects 0.0948% of it. The rest is reachable
and was measured today, at the `.ll` level against controls built the same way:

    noinline on u_bytes        Ir                   delta
    encodebench     4,317,271,654 -> 4,230,851,654   -2.0017%
    livebench       4,318,225,276 -> 4,271,590,876   -1.0799%
    oneshot            23,904,190 ->    23,787,604   -0.4877%

`u_bytes` writes `\u00XX` for a control byte and does not run once on any of
these inputs. It costs anyway: LLVM inlines a single-caller internal function
unconditionally, because doing so is free in code size, and the 27 lines it
folds into the fold's lambda push that lambda from four callee-saved registers
to six. The whole livebench delta is `w_klam17` alone, 708,178,000 ->
661,543,600 over 11,658,800 calls: four instructions a byte, two pushes and two
pops, paid by every plain byte in every string.

**The obvious rule is refuted.** Marking all seventy-two single-call-site user
functions in livebench `noinline` costs **+5.67%** — 4,318,225,276 ->
4,563,103,685. So "one caller" is not the discriminator, and neither is size:
`u_bytes` is 27 IR lines, smaller than fifty of the seventy-two.

The discriminator is that the arm is cold, which the compiler has no way to
know. The record already settled whose question that is: the archive's
2026-09-02 entry, restated in the 2026-09-05 sitting, keeps an inferred cold
arm on this side of the charter — how a dispatch group is emitted is not
something a user meets — so this is the emitter's to answer, not a gavel. The
narrow inference to build and measure next: a function whose only call site is
an `if` arm inside the body of a lambda passed to a fold.

---

## 2026-09-06 — THE REGENERATION KEEPS ITS NOTES, AND A SECOND COLD-ARM RULE IS REFUTED

**SHIPPED, the regen half.** The entry above left the property named and
false: `KANSO_REGEN_COMPILE_GOLDEN=1` wrote a header literal from
`tests/compile_cost.rs` plus the measured rows, dropping whatever a previous
change had written between them. `rewrite_rows` keeps every comment line where
it is and replaces only the data rows, which is what
`scripts/gates/all_counters.sh --write` has always done; the literal header is
the fallback for a golden that does not exist yet.

The spec was watched red on the old write first, and failed naming the dropped
note rather than something incidental. End to end, with the real environment
variable against the real goldens, `bench/compile_golden.txt` and
`bench/compile_golden_modules.txt` both come out byte-identical when nothing
moved — sixteen and thirty comment lines respectively, all still there.

### The narrow cold-arm rule the entry above proposed: REFUTED

That entry ended by naming the inference to build and measure — a function
whose only call site is an `if` arm inside the body of a lambda passed to a
fold. Built as a census over `livebench.ll` and measured the same way as the
first rule, marking what it selects and relinking against a control:

    livebench                                  Ir             delta
    u_bytes alone                    4,271,590,876   -46,634,400  -1.0799%
    control                          4,318,225,276
    every single-caller (72 marked)  4,563,103,685  +244,878,409  +5.6708%
    the narrow rule (48 marked)      4,415,008,067   +96,782,791  +2.2413%

The narrow rule selects forty-eight, and among them are `encode_list_2`,
`encode_map_2` and `escape_onto_2` — the last of which the 2026-09-06 entry on
the merged group already priced alone at +2.5582%. Being reachable from a
lambda and sitting behind a branch does not make an arm cold: most of the
decoder is reachable from a lambda, and every `if` has two arms.

**Two rules measured, both worse than doing nothing, and the win is still
there.** What separates `u_bytes` from the forty-seven others is how often its
arm is taken, and that is the one thing neither the emitter nor LLVM at `-O3`
without a profile can see. The next candidate is not another syntactic
predicate over the emitted module — those are now two for two — but a source of
frequency: a counted run, or a construct in the language that says it.

## 2026-09-06 — A SWITCH ARM WAS NEVER TOLD WHAT IT HAD BEEN HANDED

**DONE.** A dispatcher's parameters carry what inference proved about them, so a
body indexing one of those slots can skip the tag test whose answer the front
end already holds. The ladder dispatcher has always done this: a bare `Var`
parameter records its group's set as `emit_pattern_known` binds it. The switch
dispatcher — a group whose arms name byte values or tags, which is how the json
decoder is written — binds the arm's names straight to `%x{i}` and recorded
nothing, because the switch has already decided what the value is and there is
no pattern left to emit. Every group the switch took over went back to paying
the test.

`record_param_sets` says it at the entry, from both dispatchers. On the same
host, control against change:

    jsonbench     1,542,924,237 -> 1,526,906,787   -16,017,450   -1.0381%
    encodebench   4,317,271,614 -> 4,310,952,517    -6,319,097   -0.1464%
    oneshot          23,904,150 ->     23,797,367      -106,783   -0.4467%
    basket           35,403,174 ->     35,365,158       -38,016   -0.1074%
    digestbench      75,582,076 ->     75,564,654       -17,422   -0.0231%
    livebench     4,318,225,236 -> 4,318,118,450      -106,786   -0.0025%
    pendbench       605,515,340 ->    605,514,940          -400   -0.0001%
    scanbench       768,849,487 ->    768,849,367          -120   -0.0000%

widebench, deepbench, escapebench, indexbench and readbench are unchanged to the
instruction. Nothing rises.

The whole jsonbench fall is four functions, and they sum to the total exactly:

    obj_key_start'2   205,282,500 -> 198,630,750    -6,651,750
    array_step'2      119,421,450 -> 113,421,600    -5,999,850
    str_run             122,966,100 -> 120,913,200    -2,052,900
    obj_key_start        37,622,100 ->  36,408,600    -1,213,500
    array_step            2,483,550 ->   2,384,100       -99,450

Each of those groups indexes `cs` — `cs[n]`, `cs[p]`, `cs[p + 1]` — inside an
arm the switch selected, and `cs` is bytes at every call the group has.

THE GAP DATES FROM THE SWITCH ITSELF (kanso#1266, 2026-09-04). It cost decode
1.04% for two days with every allocation counter and every `.mem` row
byte-identical, which is the shape of regression the emitted and `.text` veins
exist to catch — and neither caught it either, because the switch was a fall on
both when it landed and the lost narrowing was inside that fall.

The fixture is
`tests/golden/micro/a_switch_dispatched_arm_knows_what_it_was_handed.kso`. It
took three attempts to write one that reaches the change, and the two failures
are the finding restated: a program whose group is a ladder emits
byte-identical IR before and after, and so does a group with one int arm and a
`none` arm, because `switch_shape` wants two int arms or one against a bare
generic tail. The fixture that works has arms on 46 and 59, a `none` arm, and a
generic arm that reads `cs[p + 1]` — and its `d_probe/scan_4` loses the tag test
in the diff. It carries `first` beside it, whose one slot holds bytes on one
call, a list on the next and a string on the third: the group's set is all three
and the test stays, which is what makes the narrowing sound.

The first mutation of this change did not fail, and that was information. Forcing
the recorded set to `BYTES` for every parameter changed no emitted line in the
probe, because `emit_pattern_known`'s `Var` arm overwrote it a moment later with
the correct set. That is the fact that sent the search to the switch.

VEINS. Every allocation counter and every `.mem` row agrees — `all_counters.sh`
reads clean. `.text` falls on ten of thirteen programs (jsonbench 93,138 ->
92,370, livebench 116,610 -> 115,778, digestbench 105,170 -> 104,546) and holds
on the other three. The emitted goldens fall on nine. `compile_cost`,
`compile_libraries` and the module vein agree. `bench/instructions_golden.txt`
is NOT regenerated here: this container is glibc 2.39-0ubuntu8.7 against the
golden's 2.39-0ubuntu8.8, so `host_gate.sh` refuses the comparison and the rows
above are a same-host A/B rather than a sitting the golden can take. CI measures
them and they get copied in from the job log.

**OPEN.** The narrowing is still per-group, so a slot whose group sees three
shapes pays the test even where a caller proved one. That is the specialisation
thread — an `.ll`-level probe on `d_list/fold_flat_5` measured it at livebench
−1.1854% and encodebench −1.2112% with output byte-identical on both — and it
needs a clone of the group under a caller-proved set, which this change does not
build.

## 2026-09-06 — CI'S ROWS FOR THE SWITCH ARM, AND THE 390 THAT ARE LAYOUT

**DONE.** The entry above measured the change on a container whose glibc is
2.39-0ubuntu8.7 against a golden measured on 2.39-0ubuntu8.8, so it could
report deltas and not rows. CI measured the rows. **Every delta agrees to the
instruction, on all thirteen.**

    jsonbench    1,542,924,650 -> 1,526,907,200   -16,017,450   -1.0381%
    encodebench  4,317,272,013 -> 4,310,952,916    -6,319,097   -0.1464%
    oneshot         23,904,549 ->     23,797,766      -106,783   -0.4467%
    basket          35,403,587 ->     35,365,571       -38,016   -0.1074%
    digestbench     75,582,475 ->     75,565,053       -17,422   -0.0231%
    livebench    4,318,225,649 -> 4,318,118,863      -106,786   -0.0025%
    pendbench      605,515,753 ->    605,515,353          -400   -0.0001%
    scanbench      768,849,900 ->    768,849,780          -120   -0.0000%

widebench, deepbench, escapebench, indexbench and readbench held. That
agreement is worth having: an A/B on this container measures the same events CI
does, and only the absolute rows are the runner's.

`compile_instructions` rose 390, 42,061,345 -> 42,061,735, and it is the layout
vein rather than a decision. `kanso check lib/json` stops before codegen, so an
emitter change cannot alter anything this row counts — and src/codegen.rs IS the
compiler, so its bytes and the layout under them move anyway. `compile_allocs`
and `compile_peak_bytes` are byte-identical, which is what says the front end
really did not move. This is the ninth recorded layout-only move of that row.

Welfare **75.36021114125158 -> 75.38438760189786**, banked in the same push.

## 2026-09-06 — THE OBJECTIVE MEASURES ONE RUN PROGRAM

**GAVEL (Clay, 2026-09-06).** "The objective measures ONE consolidated run
program — retired instructions as run speed, peak as run memory. Build it:
decode and encode as the bulk, the stress shapes (wide, deep, pending, escape,
index, digest, scan) at realistic proportion, the mix stated in the program's
header. Per-phase counters stay as diagnostic goldens; the shelves stop being
objective inputs. Baseline it, re-set the floor in the same PR as a definition
change, and rewrite history.jsonl's welfare column once. Retire the
advertised/guards split and the granted-baseline machinery for run counters;
close #317 and #319 as moot and remove #319 from the ledger."

**SEARCHED** before filing: the live log's entries of 2026-09-03 (the objective
rebuild), 2026-09-04 (readbench joins) and 2026-09-05 (livebench joins, and the
granted-baseline entry that held the clean-prefix change); the archive's
satiation design; `design/pending-gavels.md`, which carried the granted-baseline
question as its own entry. Nothing there anticipates one consolidated program.

**THE PROGRAM.** `bench/runbench` runs eight phases in one process and prints
one line. The mix is measured rather than asserted: each phase was built alone
with the other seven zeroed, and the count that landed it on its share is the
one in the source.

    decode  98 rounds of lib/json over large.json  1,045,772,072  34.54%
    encode  90 rounds over the same document       1,042,585,953  34.43%
    deep    545 trials of the nested grid            188,771,447   6.23%
    digest  sha256 over a 15,675-byte slice          152,850,246   5.05%
    index   690,000 characters walked by index       150,623,773   4.97%
    escape  3,872 lines through the escaper          149,982,432   4.95%
    pend    50 records with a field left pending     149,607,747   4.94%
    split   428 characters through the splitter      147,499,570   4.87%
                                                   -------------
    the whole program                              3,043,742,734

Decode and encode carry 68.97% between them. The counts are not round because
`split` is quadratic in its size — 120 characters cost 11,800,000 and 1,526 cost
1,870,000,000 — so its share was solved for rather than chosen. The program
imports `std/json` rather than carrying a frozen copy: the three frozen
benchmarks keep doing that isolation job as diagnostics, and what the project
costs to run includes the library it ships.

That table is the container's sitting. `bench/instructions_golden.txt` carries
CI's, **3,043,743,748** — 1,014 instructions apart, 0.00003%, where glibc
differs — and in the same sitting every other row was byte-identical to its
golden, which is the shape host divergence takes here. The difference moves no
percentage in the second column, so the shares stand as measured, and welfare's
baseline moved with the golden rather than with the table: the ratio stays one
and the floor does not move.

**THE FLOOR IS NOT COMPARABLE WITH THE ONE BEFORE IT.** 75.38 becomes 51.89.
The run counters are re-baselined to today's measurement, so the run terms sit
at parity, while the three compile rows keep the advantage they have
accumulated since august — most of what is left in the number is compile cost.
The floor was LOWERED BY HAND to 51.0 and then `--set` wrote the measured
51.89, which is the only override welfare allows and the one its own comment
names. A fourth loosening of the ratchet guard was considered and declined:
three earlier ones were each an escape.

**THE THIRTEEN ARE DIAGNOSTICS NOW, AND STILL GATED.** Every cost golden, every
`.mem` row and every instruction row stays, and `instructions.sh` walks
fourteen. What changes is that a move in one of them says WHERE a cost went
rather than whether the project came out ahead. `tests/every_benchmark_is_in_the
_objective.rs` asserted the opposite property — every rowed benchmark is
weighed — so it inverts: the objective weighs exactly one program, and that
program is built, measured and rowed. Its counter reader used `find` once per
line, which a second counter written on the same line defeated; the mutation
proved it green with two benchmarks in the model, and it scans every occurrence
now.

**151 ROWS OF HISTORY CANNOT BE SCORED.** No row predating today carries
`run_instructions` or `run_peak_bytes`, because the program they name did not
exist. 349 of the five hundred are scored on the three compile counters alone,
at coverage 0.28 or 0.44; the other 151, between 2026-08-13 and 2026-08-24,
carry none of the five, because the interpolation miscompilation of that
fortnight took the compile group out of the row. The rescorer died on the first
of them saying "`*` is not defined for these values" — an empty denominator
surfacing three functions later, in the rounding. Those rows now get no welfare
column and `scored_weight` 0.00, which the chart already draws as a gap, and a
score the previous formula produced is removed rather than left sitting under a
`scored_by` that names the new one. Nothing is invented for them: the counters
are still in the file, so a future formula that can read them will.

**THE GRANTED-BASELINE RULE IS RETIRED**, and the question in the ledger goes
with it. `entering` gave a counter new to the model a baseline of `now *
standing` so that landing day cost the floor nothing; nine of twenty-one stood
on it. It was never neutral afterwards — saturation is concave, so a counter
granted a high standing has almost no headroom and one at parity has a great
deal — and that difference decided the carry-tier verdict of 2026-09-01: 74.31
-> 73.75 with the digest baselines granted, 70.14 -> 72.99 with them at parity.
With the run side one program, nothing joins a benchmark at a time and the rule
has nobody left to grant to. A counter in the model with no baseline is refused
now, by name.

The refusal is taken in the arm that decides whether to score at all. The first
shape returned it from `baseline_of`, and an effect is a value in kanso: the
complaint came back as the baseline, `score` indexed it, and the program died
saying "indexing takes a list or string with a 1-based position, or a map with
a key" — a true sentence about the wrong thing, three functions from the cause.

**OPEN.** Whether the compile rows should re-baseline too, restarting the whole
index at parity together. It is a one-line change to the floor file and it is
Clay's, not mine: it decides whether the number's origin is the day the
objective was last redefined or the accumulated history of the compile side.
The gavel did not say, and the index reads what it reads either way.

## 2026-09-06 — A WHOLE FLOAT KEEPS ITS POINT IN THE EXPONENT FORM TOO

**GAVEL (Clay, 2026-09-06), one of five side rulings that day: "whole floats
keep their point."**

**SEARCHED** before filing: the live log carries no entry on float rendering
since the negative-render fix of 2026-09-05 (`c072c8b5`); the archive's float
entries are the ryū landing, the digit-pair table and the shortest-round-trip
work, none of which touches the exponent branch's fractional part;
`design/pending-gavels.md` carried nothing on it.

**WHERE IT WAS.** `render_ryu` in src/runtime.c and `render_float` in
src/eval.rs are the two renderers, and there is no third — the wasm engine is
eval.rs compiled to wasm, and a grep of src/ for the guard, for `render_float`
and for `render_ryu` finds these two. Both wrote the leading digit and then a
fractional part only `if k > 1`, so a mantissa of one significant digit reached
the exponent form with nothing after it:

    before   1e+20   1.5e+20   1e-07   1.5e-07   3.0   -1e+20
    after    1.0e+20 1.5e+20   1.0e-07 1.5e-07   3.0   -1.0e+20

The fixed branch already kept the point. Two lines, one per engine, and the
interpreter and a `--release` native build agree byte for byte on every value
above.

**THE FIXTURE WAS NAMED FOR THE RULE AND DID NOT HOLD IT.**
`tests/golden/micro/a_whole_float_keeps_its_point.kso` has existed since the
`%.1f` removal, and line 3 of its golden read `1e+15 -1e+15` — a whole float
with no point, in the file that claims whole floats keep theirs. Its header
explained the exception in passing rather than treating it as one. So the
ruling had a home in the corpus before it had a fix, and what the change owed
was a wider fixture rather than a new one: the four shapes are a mantissa of
one significant digit against several, times the fixed form against the
exponent form, because a different line of the renderer writes the point in
each. The multi-digit cases are what say the point is not written twice.

**THE BLAST RADIUS IS TWO GOLDENS, AND THE FIRST READING SAID ONE.**
`micro_corpus_agrees_across_engines` asserts inside a loop, so it panics on the
first fixture that disagrees and never reaches the rest; taking that one name
as the count is reading a stop as a total. Grepping every stored golden for an
exponent-form float with a single significant digit — `(^|[^0-9.])[0-9]e[+-]`,
which catches any mantissa rather than just 1 — finds
`a_negative_double_renders_behind_its_sign` and
`a_whole_float_keeps_its_point`, and nothing else in tests/, docs/, lib/,
bench/ or scripts/. No book panel, no page, no library test.

**NO COUNTER MOVES.** `all_counters.sh` prints no divergence for any of the
twelve cost veins, and `kanso test` on lib/expect, lib/json, lib/list and
lib/path passes unchanged — so nothing in the shipped library renders a
single-digit exponent float and the json encoder's output for the benchmark
corpus is byte-identical. That is the surprising half: a change to float
rendering that the decode and encode goldens cannot see.

**THE COMPILE VEIN DID NOT MOVE, AND I SAID IT WOULD.** This paragraph first
read that `compile_instructions` moves because src/eval.rs is the compiler's
own Rust — CLAUDE.md's rule, with seven layout-only moves recorded behind it —
and that CI would have to hand over the new number. CI measured 42,061,735,
which is the golden to the instruction. `compile_allocs` and
`compile_peak_bytes` held too, and all nineteen veins read `success` in the
cost-goldens job's own summary block.

So the rule as written is too strong. A two-line edit inside one function,
each line replacing a conditional tail with an unconditional one, left the
front end's retired-instruction count byte-identical. The seven earlier moves
are real and the prior is a good one; what this shows is that it is a prior
rather than a law, and that a change small enough to leave the layout alone
leaves this row alone with it. Project it from CI either way — being wrong in
this direction costs a paragraph, and in the other direction a red round.

`all_compile.sh` here reports `machine_code`, `emitted_code`,
`compile_libraries` and `compile_cost` AGREED and REFUSES the three counters
whose golden names a glibc this container does not carry. The twelve runtime
cost veins and the lazy tier all agree, measured after a
`cargo build --release`.

## 2026-09-06 — TWO GATES THAT COULD NOT SEE WHAT THEY WATCH

**SEARCHED** before filing: the live log's entries on the counter sweep are
the 2026-09-06 one that added the .mem vein to it and kanso#1282's correction
of that vein's timing; neither notices what the step's exit status is actually
reading. The archive's `all_counters.sh` entries are its creation and the
count corrections. On the wasm blob, kanso#1180 settled that the freshness
guard stays an mtime comparison and said nothing about reproduction.
`design/pending-gavels.md` carries neither.

### The sweep blamed the .mem vein for any failure in the golden binary

`cargo test --release --test golden` builds TEN tests and the lazy-tier step
read the exit status of all ten. One of them reads `tests/golden/mem/*.mem`.
So a micro-corpus mismatch, a diagnostics mismatch, a strict-mode divergence —
any of the other nine — printed `counters moved: mem`, which names the one
vein that had not moved and sends the reader to regenerate a file that is
already correct.

Reproduced rather than argued: breaking `a_pinned_clock_reads_the_same_in_both_engines`
and running both invocations on that same tree,

    cargo test --release --test golden                     FAILED
    cargo test --release --test golden mem_corpus_pins_...  PASSED

The fix names the test. Which test to name is derived rather than written
down — `KANSO_REGEN_MEM_GOLDEN` is what regenerates the vein, so the test that
reads it is the test that owns it, and
`tests/the_sweep_reads_the_mem_vein_alone.rs` asserts exactly one test reads
that variable and that the sweep names it. Both of its assertions were watched
red on the unfixed script first, and the first run of the second one went red
for the wrong reason: the step's own comment says `--test golden` while
explaining why the name is there, and got counted as a third branch. The spec
skips comment lines now.

**It is also 47x faster, which was not the point and is the larger effect.**
The nine other tests include the two micro-corpus runs, and those are where
the time goes:

    the whole golden binary        158 s
    the one test that owns the vein  3.35 s
    the whole twelve-vein sweep    ~200 s -> 51 s

The header comment recording the 158 and the 204-second `--write` figure is
corrected in place: both numbers were the whole binary rather than this vein,
and they are kept because kanso#1282's entry recorded them.

### And nothing on CI could say whether the committed wasm blob reproduces

`docs/kanso.wasm` is a build artifact that is also committed, and no job had
ever hashed the committed blob and a rebuild on one machine. The specs job
already rebuilds it, and the rebuild OVERWRITES the file, so the committed
hash has to be taken in a step before. It now is, and a step after prints
both with the runner's rustc version into the run summary.

The first reading, on this branch:

    committed  60b9fc8a...   written at 6f8c876e (kanso#1272)
    rebuilt    5d1566dc...   rustc 1.98.1
    verdict    DIFFERS

On this container, at rustc 1.94.1, the same source builds to `512d43f4...` --
a third hash. Touching `src/lib.rs` to force a real recompile and building
again gives `512d43f4...` a second time, so the build is reproducible within
one toolchain.

Two figures in the first draft of this entry were wrong, and both are
corrected above. It gave the committed hash as `be72313e...`, which was never
this file's hash -- it has read `60b9fc8a...` since kanso#1272, here and on
the runner. And it offered two consecutive rebuilds as the evidence for
determinism, which showed nothing at all: with no source change cargo relinks
nothing and copies the same file back. The forced recompile is the test that
carries the claim.

Fourteen commits separate the committed blob from HEAD and five of them touch
`src/`, so the blob is a build of older source than either rebuild. That has
to be ruled out before the toolchain is suspected of anything.

**It is a measurement and it cannot fail, deliberately.** A gate wants an
answer to gate on and there is not one yet. A few runs saying the same thing
decide whether this becomes a gate or the blob stops being committed; until
then a step that turns CI red on a difference nobody has explained would be
a gate written before its rule.

## 2026-09-06 — THE CLOSURE CALLING CONVENTION, AND A STEP THAT ONLY PRINTED

`preserve_none` is an LLVM 19 calling convention: the callee may clobber every
register, so it needs no callee-saved prologue. kanso's closure calls are the
shape it exists for — a call through a function pointer into a body that
returns straight back. kanso#1287.

### The emitter cannot write the keyword without asking

The two spellings fail in opposite directions. clang 18 hard-errors on
`preserve_nonecc` in a `.ll` and only warns on `__attribute__((preserve_none))`
in C, where the warning is a silent no-op. A probe using the C attribute would
answer yes on clang 18 and then emit nothing, so the probe writes a two-define
`.ll` and compiles it. The C side is gated behind `K_CLOSCC` with
`-DKANSO_PRESERVE_NONE` and `-Werror=unknown-attributes`, so the attribute
cannot be quietly dropped on the runtime's nine closure-pointer sites.

Both halves or neither: an emitter-only version MISCOMPILES. encodebench
printed `error[runtime]: bytes takes a string` because `k_call2` still used the
C convention while the emitted define had changed. The spec pins the define and
the call site moving together for that reason, and dropping the call-site
keyword fails it by name.

### What it costs and what it buys

CI's sitting, under clang 19. All fourteen work rows move and they split both
ways:

    digestbench  -6.3265%      readbench    +0.0040%
    encodebench  -2.7123%      escapebench  +0.0368%
    livebench    -2.4236%      indexbench   +1.7043%
    widebench    -2.0954%      basket       +1.7993%
    runbench     -1.0301%      jsonbench    +2.0807%
    scanbench    -0.3328%      pendbench    +2.5082%
    deepbench    -0.3146%
    oneshot      -0.2474%

The decode pays for the encode. A caller that may clobber every register spills
what it wanted to keep across the call, and the decode's hot loops hold more
live state across a closure call than the encode's do. `.text` grows on every
benchmark, 528 bytes on readbench to 1,088 on basket: the convention removes
each callee's prologue and pays at the call sites, which outnumber the callees.

compile_instructions rises 42,061,735 -> 42,163,520, 0.2420%. The front end is
untouched — `kanso check lib/json` stops before codegen — and compile_allocs
and compile_peak_bytes are byte-identical, which is what says so. It is the
layout vein and the largest move it has recorded, because the edit is larger
than the ones before it: src/codegen.rs and src/main.rs are the compiler, and
src/runtime.c is `include_str!`'d into it.

Welfare 51.89 -> 51.95, and the floor is set.

### THE SPLIT WAS WRONG, AND THE OBJECTIVE SAID SO

The machinery shipped first, alone, so that each half would have one
attributable effect. That reasoning ignored what judges a merge. With CI on
clang 18 the probe answers false and the emitter writes the fallback, so the
machinery alone is a 0.2420% compile cost with every runtime counter
unchanged. Welfare scored it 51.88 against a floor of 51.89 — a fall, with no
weights argument available for it. `tests/the_digest_is_priced_on_both_sides`
caught the committed goldens failing to hold the floor, independently of my
own arithmetic. Two halves that each fail and together pass are one change.

### AND THE INSTALL DID NOT SELECT ANYTHING

The step that installed clang 19 used `update-alternatives --install
/usr/bin/clang`. On this image /usr/bin/clang is a package-owned symlink to
`../lib/llvm-18/bin/clang` rather than an alternatives path, so the symlink
stayed where it was. The step PRINTED `Ubuntu clang version 18.1.3` and nothing
read it: the probe answered false, the goldens were regenerated against a
toolchain nobody had selected, and the round measured nothing.

The lesson is not the mechanism, which is an image detail. It is that a step
which prints a fact instead of checking it cannot fail, and a check that cannot
fail is the same defect this log has recorded under other names. The step
greps for `clang version 19` and exits non-zero otherwise; /usr/local/bin
precedes /usr/bin, so a symlink there wins without touching the dpkg file.

That round also left a reading worth keeping: every work row rose by EXACTLY
159 instructions, indexbench's 4.69M and encodebench's 4.31B alike, with
emitted_code and machine_code byte-identical. A constant offset independent of
workload is per-process startup rather than anything in a loop, and the run
before it — same source, same clang 18 — had those rows byte-identical to
their goldens, so the 159 arrived with the package install. It is NOT
attributed. The rows it applied to have since been remeasured under clang 19
and the number is not carried into them.

### The veins now say which compiler measured them

bench/text_golden.txt already carried `measured-on clang=`, moved to 19.1.1.
bench/instructions_golden.txt gains one beside its glibc line: the emitter
writes a different module when the probe fails, so those rows became a property
of the host's clang the day it started probing. A clang-18 host writes the
fallback and cannot regenerate either file.

### And the second line broke the gate that reads it

The instructions vein became the first golden in the tree to name two facts,
and the gate refused a runner that matched both. `measured_on.sh` collects the
lines with `sed -p`, so `want` came back newline-joined, while `have` is built
by a loop that joins with spaces; the two are compared as one string. Its own
header has documented the two-line form since it was written -- nothing had
ever used it. The error printed the two strings looking identical, because the
only difference was the whitespace between them.

The same expression cost the machine-code vein a round for a second reason.
The pattern was a plain prefix strip, so any comment line opening with the
phrase became fact data, and these goldens carry dated notes in prose. A note
reading "measured-on moves to clang 19.1.1 with these rows" -- mine, written in
the commit above -- asked the host about a fact called `moves` and exited 2
before measuring anything.

Both reds landed on a run whose work rows were byte-identical to the goldens,
which is worth saying plainly: the cost-goldens job named two veins and neither
number had moved. A measured-on line is now one or more `key=value` fields and
nothing else, joined with a space however many lines carry them. A line SHAPED
like a fact list still reaches the case that refuses one it cannot read, so a
`clnag=19.1.1` typo is caught rather than dropped;
tests/a_golden_may_name_more_than_one_fact.rs pins that alongside the two
repairs, and asks the gate what this host is rather than deriving it a second
time. Four of its five were red first, each with the message CI printed.

### The published numbers, and the one the sweep missed

The checklist in CLAUDE.md, every surface: compiler.html's decode board, the
lazy scoreboard, the recipe block, the compile-speed note, index.html's landing
panel, about.html's prose. The board and the panel are ms/decode and peak
memory, and the release rule holds those to a sitting on an idle box; every
per-benchmark instruction figure on the page names the sitting that measured it
and is a record rather than a tracker.

That sweep concluded nothing moves, and it was wrong. §31 quotes
`compile.compile_instructions` in a `data-golden` span -- a figure that TRACKS
the golden rather than recording a sitting -- and this change moved that row, so
the page read 42,061,735 against a golden of 42,163,520. `golden_prose` caught
it on the first run of the welfare job, which is the first run this PR got,
because welfare waits on cost-goldens and cost-goldens had been red until the
trend gate was priced.

The lesson is the one CLAUDE.md already states and the sweep still failed:
walking a list of PAGES is not walking a list of NUMBERS. A `data-golden` span
is not a dated record and cannot be reasoned about as one; the gate reads them
all and is the only thing that should be trusted to. What the veins carry --
fourteen work rows, fourteen .text rows, one compile row -- is unchanged by
this, and the counters the page quotes as records (`el_parses` at 318450, the
four arena blocks) sit in cost goldens that came back green.

### The seven rows that worsened, each with the value it landed on

The trend gate asks for the number, not the argument, and it is right to: a
regression named in prose but not priced is one nobody can check later. Against
origin/main at f534f487:

| counter | before | after |
|---|---|---|
| `work_jsonbench` | 1,526,907,200 | 1,558,677,818 |
| `work_pendbench` | 605,515,353 | 620,703,023 |
| `work_basket` | 35,365,571 | 36,001,898 |
| `work_indexbench` | 4,691,265 | 4,771,217 |
| `work_escapebench` | 114,584,676 | 114,626,851 |
| `work_readbench` | 4,283,257 | 4,283,427 |
| `text` | 1,468,908 | 1,479,004 |
| `compile_instructions` | 42,061,735 | 42,163,520 |

Six of the eight are the decode paying for the encode, which the section above
attributes: a caller that may clobber every register spills what it wanted to
keep across the call, and the decode's hot loops hold more live state across a
closure call than the encode's do. `work_readbench` at 170 instructions and
`work_escapebench` at 42,175 are too small to be that or anything else; they are
the layout moving under a binary whose prologues changed.

`text` is the sum of the fourteen rows and rises 10,096 bytes because the
convention removes a prologue from each closure callee and pays at the call
sites, which outnumber them. `compile_instructions` is the layout vein and is
attributed above.

Against these, eight rows fall: `work_digestbench` 75,565,053 to 70,784,439,
`work_encodebench` 4,310,952,916 to 4,194,027,086, `work_livebench`
4,318,118,863 to 4,213,463,624, `work_widebench` 54,609,871 to 53,465,568,
`work_runbench` 3,043,743,748 to 3,012,388,655, `work_scanbench` 768,849,780 to
766,291,267, `work_deepbench` 702,627,486 to 700,416,944 and `work_oneshot`
23,797,766 to 23,738,890. The objective weighs run speed through runbench, which
is the fifth of those, and welfare rose. The trade is taken on the sum, not
defended row by row.

### The ratchet row that went blind, and whose it was

The ratchet job read `1 rows proved nothing` on every head of this branch:

    BLIND specs — a release build that gives up the guaranteed tail call: the gate stayed green

Seven hypotheses were measured in the container and every one came back red,
meaning the mutation reddened the gate here under clang 18, under clang 19,
through the ratchet's own worktree-and-shared-target path, in CI's row order,
and on heads whose goldens matched. The eighth thing checked was the record:
the nightly `prove all` on main, which is the only run that exercises this row
on a runner when a branch does not touch `src/main.rs`. It proved the row red
on 2026-09-04 (159f6b2b) and read it BLIND on 2026-09-05 (794113fc) and
2026-09-06 (def24ed3). The row was main's for two nights before this branch
touched the file the mutation patches, and `touched` selected it. My earlier
claim that "main at f534f487 was green" read the pull-request job, which on
main itself selects no rows; it was not evidence about this row.

The route the mutation takes is a stack overflow. `a_record_rebuilt_at_depth`
hops two hundred thousand times through a pair of arms; with `musttail`
stripped, each hop spends a frame, and the release binary dies before its
`print`. What decides whether it dies is the host's stack limit, and the
binary's need has been falling:

    compiler at 159f6b2b (2026-09-04), mutated:  segfaults at 16384 KB, prints at 24576 KB
    compiler at 974cf44f (this branch), mutated:  segfaults at 15360 KB, prints at 16384 KB

both under clang 19.1.1, and the second the same under clang 18. GitHub's
Linux runners raised their default stack from 8 MB to 16 MB
(actions/runner-images#3257); every shell here has 8 MB. So the Sep 4 binary
overflowed both, the Sep 5 binary overflowed only the container, and eighteen
commits landed between the two nightlies -- the frames shrank under the
runner's limit somewhere in #1245 through #1257, and nothing in the tree named
the limit the proof depended on. The container could not reproduce CI because
it was comparing against a number CI never printed.

The fix pins the number: `micro_corpus_survives_a_release_build` runs each
release-built sample under `ulimit -s 8192`, so the gate's reading stops
depending on where it runs. Raising the container's limit to 32 MB reproduced
the blind row locally; the fix reddens it there; and the unmutated corpus
still passes under the pinned 8 MB, which is what the row proved on every
shell before this. CI's ratchet is the spec that was red first, on eight heads.

## 2026-09-07 — THE RUN THE SCAN ALREADY PROVED, UNDER THE OBJECTIVE THAT CAN PAY FOR IT

`escape_onto` scans a string once for a quote, a backslash or a control byte,
and when the scan finds nothing the whole string leaves in one copy. When it
finds something, the fold ran over every byte of the string — including the
bytes in front of the first escape, which the scan had just proved clean. On
`bench/large.json` that run is 7,690 of the 29,147 bytes the fold walks, a
quarter of its work (archive, 2026-09-05, "The run the scan already proved").

That entry built the skip four ways, measured livebench −3.06% and oneshot
−1.45%, and the objective declined all four: livebench had entered the corpus
that morning at its dimension's standing, satiated, and the compile row it
charged sat near its baseline where the curve is steep. Two days later the
2026-09-06 gavel made one program the whole run-speed term, based at its own
current reading — parity, the steepest point of its curve — and the same
change prices the other way round. This is the library-only shape from that
entry, rebuilt against the one program. kanso#1291.

### What it is

`escape_split` reads the scan's answer `n`. Not found: the whole string
appends, as before. Found at 1: the fold runs from the first byte, as before.
Found later: the bytes before `n` append in one copy — `append acc (slice bs 1
(n - 1))`, which the emitter fuses into a bounded copy on a unique builder —
and the fold runs over `slice bs n len`. One suffix slice allocates per
escaping string; the prefix does not.

The fold-from-an-index shape without that slice needs a `bounded` view, and
`list/bounded` is opaque outside std/list: only `take` over a `cursor` builds
one and nothing public builds a cursor at a position. That is new std surface,
and it is not this change.

### What it measures, container, clang 19 selected as CI's toolchain.sh selects it

    runbench   3,012,388,061 -> 2,962,077,059   -50,311,002   -1.6702%

The program prints the same `runbench 46013475` and lib/json's twenty-three
specs pass. Inside the profile, the fold's lambda `w_klam38` falls 156,716,820
to 116,468,280 and `d_list/fold_3` 143,977,844 to 119,754,344; `encode_onto`
rises 299,938,349 to 309,197,639 and memcpy 105,555,225 to 106,097,655, which
is the two slices. The container reads CI's runbench row 594 instructions low
on the same binary, so CI's fall should land within a few hundred of that.

The compile row, `kanso check lib/json` under callgrind on this host:
45,987,098 -> 46,118,838, +131,740, +0.2865%. It is a library change and the
library is compiled by the compiler; the front end visits 17,264 -> 17,560
expressions on lib/json.

### Priced

Welfare, with the container's two deltas laid over CI's rows (runbench
2,962,077,653 and compile_instructions 42,295,260 projected): **51.95 ->
52.05, +0.10**, and `--set` is owed once CI's rows land. The run term sits at
ratio 1.0 where its satiation-2.0 curve is steepest and carries 0.30; the
compile term is at 1.34 on a 0.5 curve and the row is one of two counters in a
0.32 term. The same trade the old objective declined at −0.02.

### Counters that moved, each with the value it landed on

Three cost goldens, regenerated by `all_counters.sh --write`; the .mem vein
did not move. The slices are the allocations, the fold doing less is
`append_fast`, and the suffix copies are `sh_bytes`:

| counter | before | after |
|---|---|---|
| `run_allocs` | 8,027,805 | 8,157,315 |
| `run_alloc_bytes` | 601,755,829 | 606,924,889 |
| `run_append_fast` | 10,599,183 | 10,036,593 |
| `run_held_peak_bytes` | 719,516 | 728,040 |
| `run_sh_bytes` | 36,862,800 | 39,971,040 |
| `live_allocs` | 9,233,103 | 9,808,703 |
| `live_alloc_bytes` | 710,726,112 | 733,699,712 |
| `live_append_fast` | 42,323,697 | 39,823,297 |
| `live_held_peak_bytes` | 719,516 | 728,040 |
| `live_sh_bytes` | 100,713,384 | 114,527,784 |
| `oneshot_allocs` | 60,088 | 61,527 |
| `oneshot_alloc_bytes` | 4,025,196 | 4,082,630 |
| `oneshot_append_fast` | 116,679 | 110,428 |
| `oneshot_held_peak_bytes` | 632,284 | 645,048 |
| `oneshot_sh_bytes` | 395,208 | 429,744 |

`run_peak_bytes` is the objective's memory term and `held_peak_bytes` is a
third of its sum: +8,524 on 156,826,904 is 0.0054%, worth 0.000 points, and
the trade is taken on the sum.

The compile veins, by `all_compile.sh`: the decoder's emitted code
`emitted_defines` 187 -> 189, `emitted_calls` 1,843 -> 1,864,
`emitted_branches` 1,206 -> 1,217, `emitted_lines` 12,777 -> 12,886 — two
arms and the split are two more defines and the calls they make — and the
same on the three programs importing std/json: oneshot 187/1,836/1,195/12,706
-> 189/1,857/1,206/12,815, livebench 188/1,860/1,209/12,823 ->
190/1,881/1,220/12,932, runbench 577/6,041/3,406/34,343 ->
579/6,062/3,417/34,452. `front_end_visits` 17,264 -> 17,560. CI's rows —
`work_runbench`, `work_livebench`, `work_oneshot`, `compile_instructions`,
`compile_allocs`, `compile_peak_bytes`, `text` — are the next round's, and
this entry gains their values then.

### The mutation, and the one it changed the shape of

`an_encoder_that_walks_a_clean_string` greps for the fast-path line by its
text, and the line moved from `escape_clean` to `escape_split`; it is
repointed, and applies. A second row, `clean_run`, patches the same line the
other way — the found arm back to `escape_able acc bs`, walking the run again
— and the live vein's `append_fast` rises by the bytes walked while `allocs`
falls by the slices, so `live_counters` reads it. The gutted `escape_rest`
arm was the first shape and does not compile: an arm with an unused parameter
is refused, and a build that never runs is UNBUILT rather than red.

### The scan, iterated: no byte of an escaping string is walked one at a time

The skip above stops at the first escape and hands the suffix to the fold.
The 2026-09-06 (ninth) entry's census said what that suffix looks like on
`bench/large.json`: 4,562 escapes with 6,335 clean runs between them averaging
2.67 bytes, 2,802 of them empty. It declined the run-scan on the DECODE side
with that census. The encode side was built anyway, because the arithmetic is
different there: a fold step costs 66 instructions a byte (2026-09-06, fourth),
and one `find2_below` call plus one fused slice-append is a fixed price per run
however long the run is.

`escape_rest` now appends the prefix, escapes the byte at `n`, and calls
`find2_below` again from `n + 1`; each clean run it finds leaves as `append acc
(slice bs p (m - 1))`, which the emitter fuses on a unique builder, and the next
found byte escapes. The fold, its lambda, and the suffix slice are gone, and so
is `import "std/list"` — the fold was lib/json's only use of it.

    runbench   2,962,077,059 -> 2,910,317,247   -51,759,812   -1.7474%

on the same container and toolchain; from before the skip, 3,012,388,061 ->
2,910,317,247, -3.3884%. The program prints `runbench 46013475` and the
twenty-three specs pass. In the profile the lambda `w_klam38` (116,468,280) is
gone and `d_list/fold_3` falls 119,754,344 -> 43,707,584 (the folds that
remain are other phases'); `escape_run` is 30,822,840 new and
`k_b_find2_below_raw` rises 60,993,810 -> 84,989,250, which is the 410,580
extra scans. **The beat took 52.7M of the 104.5M the fold gave back**:
`k_beat_iter` 64,497,420 -> 100,083,420, `k_beat_pop` 39,072,410 ->
51,359,300, `k_beat_push` 15,017,844 -> 19,804,944, and `beat_iters` 2,690,246
-> 4,172,996. The escape cycle is a beat cluster and every found byte is an
iteration of it where the fold spun once per string. That is the next thing to
look at on this path and it is not this change.

Counters, against the skip's goldens (`all_counters.sh --write`; the .mem vein
did not move). `allocs` and `sh_bytes` return to their pre-skip values because
the suffix slice was the only allocation the skip added; `find2_calls` is the
scans; `append_fast` falls by the bytes no fold walks:

| counter | skip | iterated |
|---|---|---|
| `run_allocs` | 8,157,315 | 8,027,805 |
| `run_alloc_bytes` | 606,924,889 | 602,780,749 |
| `run_beat_iters` | 2,690,246 | 4,172,996 |
| `run_find2_calls` | 2,606,940 | 3,017,520 |
| `run_append_fast` | 10,036,593 | 8,834,013 |
| `run_sh_bytes` | 39,971,040 | 36,862,800 |
| `live_allocs` | 9,808,703 | 9,233,103 |
| `live_alloc_bytes` | 733,699,712 | 715,281,312 |
| `live_beat_iters` | 5,032,401 | 11,622,401 |
| `live_find2_calls` | 4,206,810 | 6,031,610 |
| `live_append_fast` | 39,823,297 | 34,478,497 |
| `live_sh_bytes` | 114,527,784 | 100,713,384 |
| `oneshot_allocs` | 61,527 | 60,088 |
| `oneshot_alloc_bytes` | 4,082,630 | 4,036,584 |
| `oneshot_beat_iters` | 12,581 | 29,056 |
| `oneshot_find2_calls` | 27,285 | 31,847 |
| `oneshot_append_fast` | 110,428 | 97,066 |
| `oneshot_sh_bytes` | 429,744 | 395,208 |

### The compile veins halve, and the compiler had nothing to do with it

`kanso check lib/json` compiles lib/json and everything it imports. With the
fold gone, lib/json imports std/text and nothing else, and std/list — 492 lines,
the largest module in std — leaves the closure. Every compile vein reads it:

- `compile_instructions`, in the gate's own box (lib without its `*_test.kso`
  files, the gate's tunables): 42,632,518 before the skip on this host against
  CI's 42,594,953 for the skip, and **19,580,079** iterated. The container and
  the runner agree on this row to 0.1%, so CI's should land near 19.56M.
- `compile_allocs` 25,862 -> 11,613 and `compile_peak_bytes` 724,798 -> 375,222
  on this host; CI read 26,018 and 749,443 for the skip.
- `front_end_rounds` 42 -> 35, `front_end_visits` 17,560 -> 9,884.
- The emitted code of every program importing lib/json: the decoder
  189/1,864/1,217/12,886 -> 139/1,254/814/9,215 (defines/calls/branches/lines),
  oneshot 189/1,857/1,206/12,815 -> 139/1,247/803/9,144, livebench
  190/1,881/1,220/12,932 -> 140/1,271/817/9,261. runbench imports std/list for
  its own phases and rises 579/6,062/3,417/34,452 -> 581/6,111/3,438/34,683,
  which is the escape cycle's arms.

Priced with the container's rows laid over CI's: **welfare 51.95 -> 55.55**,
with `compile_peak_bytes` still at the skip's row; the peak's fall adds about
half a point more. The compile term carries 0.32 on a 0.5 curve and its ratio
goes 1.34 -> 2.9, which is where nearly all of it comes from; the run term's
further fall is worth about 0.5.

The number is real by the objective's definition and it measures the workload
shrinking, not the compiler getting faster. A later change that gives lib/json
a reason to import std/list again would read as a four-point fall and have to
argue its way past the floor. Whether the compile term's workload should be
lib/json's import closure, which a library edit can halve, or a fixed corpus
that only the compiler moves, is Clay's, and it is filed in
design/pending-gavels.md ("The compile term's workload"). Until he rules the
standing rule holds — a rise is held — so `--set` goes in with CI's rows and
the entry says which part is the workload.

### The mutation, again

`a_clean_run_walked_byte_by_byte` grepped for the `escape_split` line and
patched the found arm back to `escape_able`, which no longer exists. It now
greps `escape_run`'s one line and rewrites it as a self-call appending one
byte at a time — no fold, no import, every parameter in use, so the mutated
package compiles (checked on a copy). The live vein reads it through
`append_fast`. `an_encoder_that_walks_a_clean_string` greps the `escape_split`
line, which did not move.

### CI's rows, and the floor

The runner counted the iterated shape on 2026-09-07: `work_runbench`
2,910,317,901 (the container read 2,910,317,247, 654 low, as it was on the
skip), `work_livebench` 3,984,010,329, `work_oneshot` 23,182,078, and
`work_jsonbench` 1,558,677,818 -> 1,561,185,741 — the decoder imports lib/json
and its layout moved with the library. `compile_instructions` 42,594,953 ->
19,335,435 (the container's box read 19,580,079), `compile_allocs` 26,018 ->
11,613, `compile_peak_bytes` 749,443 -> 375,222, both to the byte what the
container read. `.text` for the three programs importing lib/json: oneshot
115,986 -> 103,906, livebench 116,530 -> 104,450, runbench 240,386 -> 240,882.

**Welfare 51.95 -> 57.03, +5.07, and the floor is set there** with the
reason naming the workload. The run term's share is about 0.5; the rest is
the three compile rows, which is the question in the ledger. The compiler page
quotes those three rows through `data-golden`, and the prose gate held the
welfare job red until the page said the new numbers and why they moved.

The ratchet's first round on the iterated shape went UNBUILT on
`an_encoder_that_walks_a_clean_string`: its patch still named `escape_able`,
which left the library with the fold. It now hands a clean string to
`escape_rest` at position 1 — the first byte through `esc_byte`, a second scan,
the rest as a slice — so the program answers the same bytes and `find2_calls`
rises by one per clean string. The row's name in ratchet.kso says so.
