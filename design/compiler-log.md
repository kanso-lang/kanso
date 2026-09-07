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
welfare job red until the page said the new numbers and why they moved. The
specs job was red beside it: `json_decode_loops_stay_conservative` pins which
lib/json groups the beat analysis licenses, and the escaper's four-group cycle
now qualifies — it threads the encoder's byte builder by identity, the
licence the encoders hold. The pin lists it. Its rewinds free nothing, and
the classifier that could see that is the change after the next.

The ratchet's first round on the iterated shape went UNBUILT on
`an_encoder_that_walks_a_clean_string`: its patch still named `escape_able`,
which left the library with the fold. It now hands a clean string to
`escape_rest` at position 1 — the first byte through `esc_byte`, a second scan,
the rest as a slice — so the program answers the same bytes and `find2_calls`
rises by one per clean string. The row's name in ratchet.kso says so.

## 2026-09-07 — A CLUSTER REWINDS ONCE A TRIP

A beat cluster of several groups — a mutual tail cycle the analysis has
licensed to rewind the arena between iterations — rewound on every internal
tail edge. `escape_at -> escape_next -> escape_found -> escape_more ->
escape_at`, the cycle "The scan, iterated" (above) put into lib/json, is four
groups, so every found byte paid four rewinds; one frees everything the trip
allocated, and `k_beat_iter` is 24 instructions a call on the empty fast path.
The entry above measured the beat's share at 52.7M of that change's 104.5M.

### What it does

`beat_loops` hands codegen a set of rewinding edges. A self-loop rewinds on
its one edge, as before. A cluster of several members rewinds on the back
edges of a depth-first walk over its internal tail graph, roots in name order:
every cycle in a directed graph crosses at least one back edge of any
depth-first walk, so every trip round any cycle still rewinds, and a simple
cycle rewinds once. Codegen's plain-rewind arm consults the set; the carry arm
is untouched, and a cluster with a carried member keeps every edge, because
the carry protocol was not measured under fewer rewinds and this entry does
not claim it. `back_edges` has a unit test: a four-cycle closes on one edge,
two cycles sharing a member on two.

On runbench's emitted code the plain rewind sites go from sixteen to ten: the
escape cycle's four become one (`escape_more -> escape_at`), `escape/many ->
more` and `regexp/ending_flag`, `regexp/digits` drop theirs, each cycle keeping
the other direction.

### What it measures

    runbench   2,910,317,247 -> 2,879,430,792   -30,886,455   -1.0613%

on the container, clang 19; the same bytes out. Every counter but `beat_iters`
is byte-identical — allocations, arena and held peaks, evacuations — which is
the claim: the rewinds that left were freeing nothing the next one would not.
`beat_iters` per program, from the veins `all_counters.sh --write` rewrote:

| program | before | after |
|---|---|---|
| runbench | 4,172,996 | 2,937,383 |
| livebench | 11,622,401 | 6,148,001 |
| oneshot | 29,056 | 15,370 |
| escapebench | 1,206,001 | 1,203,000 |
| basket | 114,020 | 114,007 |
| `a_cluster_entered_by_a_tail_call_sweeps` | 400,001 | 200,001 |
| `a_pushed_call_keeps_the_sweep` | 241,201 | 240,600 |
| `an_escaped_list_gives_its_buffer_back` | 2,001 | 1,800 |
| `beat_cycle` | 400 | 200 |
| `builder_transient` | 2,680 | 1,360 |

Nine of the fourteen benchmarks and most of the corpus do not move: their
loops are self-loops, which rewind exactly as they did.

The emitted code loses a call line per dropped site: the decoder 1,254 ->
1,251 calls, runbench 6,111 -> 6,105, and the `.text` of six programs falls
by 32 to 80 bytes on this host. `compile_instructions` moves with the compiler's
own bytes and CI says by how much.

Priced with the container's row laid over CI's (2,879,431,446 projected):
**welfare 57.03 -> 57.10, +0.07**, all of it the run term; `--set` with CI's
rows.

### The spec, and the mutation

`tests/golden/mem/a_cycle_of_four_rewinds_once_a_trip.kso` is a four-group
cycle appending to a builder 100,000 times. The compiler before this change
counts `beat_iters=400001` on it — watched — and this one `100000`, and the
.mem golden pins the latter. The ratchet row `every_edge` applies
`a_cluster_rewinds_on_every_edge_again`, which chains every internal edge back
onto the back-edge set; the mem vein reads it through that fixture and four
others.

### The measurement that was not one

The first cut of this change mis-spliced `beat_loops` and dropped the tail of
the function — the demotable entries, the carry-beat self-loops, and the rule
that keeps imported groups out of the carry tier. That build licensed
lib/sha256's clusters as carried beats, which the rule forbids, and runbench
ran 15.9 billion instructions, 77% of them in `k_slots_survive` under the
digest's evacuations. For twenty minutes that read as "back edges break the
carry protocol", and a comment saying so was written into the source before
the site list of the original emitter — no rewinds in sha256 at all — showed
the cause. The comment is gone; the carry-cluster rule that survives is the
unmeasured, conservative one. A profile that changes by 5x on a change that
should move one counter is the change being wrong, and the first check is the
emitted site list, which takes a minute.

## 2026-09-07 — A CYCLE THAT ALLOCATES NOTHING IS NOT A BEAT

The entry above took the escape cycle's four rewinds a trip down to one. The
one that stayed frees nothing: every trip appends into the encoder's builder,
whose storage is malloc'd, slices the input under those appends, which the
emitter fuses into the same copy, and asks `find2_below` for an integer.
Nothing lands in the arena, and the bracket around the cycle — a push and a
pop per escaping string, 655,650 of them in runbench — brackets nothing.
Skipping the cycle by name measured the ceiling first: runbench 2,879,430,792
-> 2,855,405,382, -0.8341%, every memory counter unchanged.

### Why the classifier thought it allocated

`alloc_groups` decides which groups allocate, and a cluster with no allocating
member is left unbracketed (`Verdict::PureLoop` has existed for exactly this).
A builtin reached through its std wrapper arrives in the tree spelled
`builtin_append`, `builtin_bytes`, `builtin_find2_below`, and those spellings
are on neither of the classifier's lists and are not program functions, so
`expr_allocates` took each for a closure value whose body it could not see.
`json/esc_pair` seeded as allocating on `builtin_append`, `escape_next` on
`builtin_find2_below`, `escape_onto` on `builtin_bytes`. `text/slice`, when it
was spelled that way, was on the allocating list by name.

### What it does now

Three things, each the size of a sentence. A head is looked up by its bare
builtin name — the `builtin_` prefix and any module path stripped — so the
lists say what they were written to say. `find2_below` joins the pure list;
it returns an integer. And an `append` at a site the linearity analysis
proved unique (`MutSites`, keyed by source position, the same set the chain
test reads) is asked only about what it appends: its target is the builder,
whose growth is outside the arena, and a `slice` nested as its argument is the
fused copy, so only the slice's own arguments are asked about. Pushes and
puts are not admitted: a list or map grown in place still takes its next
buffer from the arena.

With that the escaper's cycle has no allocating member, and `beat_loops` does
not license it. The entry from `escape_rest` stops being a demoted plain call
and is a tail call again.

### What it measures

    runbench   2,879,430,792 -> 2,855,405,382   -24,025,410   -0.8341%

the ceiling exactly, on the container with clang 19; the same bytes out.
`beat_iters` 2,937,383 -> 2,686,373 and nothing else moves.

### The spec, and the mutation

`tests/golden/mem/a_cycle_that_allocates_nothing_needs_no_bracket.kso` is a
four-group cycle that scans bytes with `find2_below`, appends the run before
each hit as a fused slice and the hit as two bytes, over 120,000 bytes built
by a loop of the same kind. The compiler before this change counts
`beat_iters=60001` on it — 20,000 for the loop that builds the input, 40,000
for the cycle, one for the entry — watched; this one counts `0`, with `allocs`
and the arena's one block identical, and the .mem golden pins the zero. The
ratchet row `pure_cycle` applies `a_pure_cycle_bracketed_again`, which makes
the in-place test never hold; the mem vein reads it through that fixture and
every other beat golden.

### What else moved

Nothing but `beat_iters`, in three cost goldens and two .mem fixtures, and no
peak anywhere: runbench 2,937,383 -> 2,686,373, livebench 6,148,001 ->
5,032,401, oneshot 15,370 -> 12,581; `append_in_place` and
`append_of_a_slice_boxes_nothing` 40 -> 0, each a loop that only appends in
place and had been rewinding forty times for nothing. Every other beat in the
corpus allocates and keeps its bracket.

Priced with the container's row laid over CI's (2,855,406,036 projected):
**welfare 57.10 -> 57.16, +0.06** on top of the entry above, all of it the run
term; `--set` with CI's rows.

The emitted code loses the bracket's calls: the decoder 1,251 -> 1,248, runbench
6,105 -> 6,102, widebench 1,843 -> 1,832 — a loop of widebench's that only
appends in place lost a bracket it never counted through — and `.text` falls
by 32 to 96 bytes on six programs here. `compile_instructions` is CI's.

One spec pinned a bracket this takes off: `tests/cohort.rs` read
`beat_iters=150000` on `cohort_kept.kso`, whose growing loop appends a
sixteen-byte literal in place into a builder. Its growth is malloc'd, so the
rewind freed nothing, and with the bracket gone every other counter the
fixture prints — allocs, alloc_bytes, arena_peak_bytes, held_peak_bytes,
append_grow, bytes_malloc, bytes_freed — is byte-identical. The pin reads
zero now, with the reason beside it, so a classifier that brackets the loop
again is red. Four CI rounds went by with the specs job red on that one
line before it was read: the cost-goldens job is the one this kind of
change usually moves, and it was the only one being read.

## 2026-09-07 — THE REMAINDER AND THE QUOTIENT OF TWO INTEGERS ARE ONE INSTRUCTION EACH

`%` always went through `k_mod`, whatever the inference knew about its
operands, and runbench asked it 2,565,677 times: 1,548,800 from the escape
phase's indexing, 451,638 from the decoder's `str_run`, the rest from the hex
arms and the phase drivers. Every one was an integer pair. The call is 43
million instructions of self cost, about seventeen a call, before the argument
boxing around it. `/` went through `k_div` the same way.

`emit_binop_builtin` has had an integer fast path for `+ - *` and the six
comparisons since the tag switch; `/` and `%` were routed to the call above
it. On a pair the inference has proved integer they are `srem` and `sdiv` now,
with two divisors sent to the call as before: zero, whose failure is the value
`a_division_by_zero_is_a_value` shows a handler asking for by name, and minus
one, whose quotient overflows at the least integer — `k_div` dies there,
`k_mod` answers zero. Two compares and a branch decide it.

    runbench   2,855,405,382 -> 2,816,922,233   -38,483,149   -1.3477%

on the container with clang 19, the same bytes out, and no counter moves at
all — the change is instructions and nothing else. The remainder is
38,068,669 of it and the quotient 414,480: runbench divides little. Six sites
in its emitted code, one per source spelling that survives inlining.

`tests/golden/micro/the_remainder_of_two_integers.kso` pins the remainder's
sign on all four quadrants, the least integer against minus one and against
seven, the largest against two, the quotient's truncation on all four
quadrants, and the zero divisor for both operators, as a handler's match and
as text, on every engine; the interpreter and native agreed before the golden
was written. The ratchet row `remainder` sends every remainder and quotient
back to the runtime and the emitted goldens read the calls that return.

Priced with the container's row laid over CI's (2,816,922,887 projected):
about +0.09 on the run term.

The emitted code trades a call line for the instruction at each site: the
decoder 1,248 -> 1,247 calls, runbench 6,102 -> 6,096, and `.text` moves by a
few bytes either way on six programs here. `compile_instructions` is CI's.

### CI's rows, and the floor

The runner counted runbench at 2,816,922,887 for the three changes together,
654 above the container's 2,816,922,233, the offset every reading since the
consolidated run program has shown. The other work rows that moved: oneshot
23,182,078 -> 22,579,660, basket 36,001,898 -> 35,737,604, deepbench
700,416,944 -> 690,817,043, escapebench 114,626,851 -> 85,754,925 (the bracket
that came off its non-allocating loop), pendbench 620,703,023 -> 620,687,423,
livebench 3,984,010,329 -> 3,743,207,118. `.text` falls on eight programs and
rises 32 bytes on scanbench. `compile_instructions` 19,335,435 -> 19,315,995,
a fall of 19,440 with `compile_allocs` and `compile_peak_bytes` byte-identical:
the layout row, moved by the emitter's two new arms and the beat's walk. The
first round was red on rustfmt alone; the second on the three veins above.

The emitted counters the trend gate reads move with the srem/sdiv arms: each
of the six sites trades one call line for two compares, two branches and a
phi, so `emitted_branches` 814 -> 820, `emitted_lines` 9,215 -> 9,237,
`emitted_other_branches` 12,623 -> 12,671, `emitted_other_lines`
132,436 -> 132,622, while `emitted_calls` 1,254 -> 1,248 and
`emitted_other_calls` 20,699 -> 20,661. Lines the compiler writes that the
processor runs as one instruction each; the work rows above are what they
cost.

**welfare 57.03 -> 57.25**, all of it the run term, held with `--set`.

## 2026-09-07 — THE TENURE WALK WAS ASKED ABOUT POINTERS THE ARENA STILL HELD

`k_ten_holds` was 2.99% of runbench: 84,272,512 instructions over 375,922
asks, 224 an ask, all of them from the sizing walk `k_copy_size` makes before
an evacuation. The function walks every beat depth below the current one and
every tenure block at each, and answers whether a pointer was promoted by an
earlier lap. Two things made it expensive, and the first one I built for was
the smaller.

### The depths

Most of the sixty-four depths hold nothing, and the walk visited each. A
bitmask of the depths that hold a block, kept at the three places the table
changes — the alloc, the hand-up, the release — lets the walk visit the set
bits only, and the release computes `k_ten_any` from the mask instead of
re-walking the table. runbench 2,816,922,233 -> 2,801,388,617, −0.5515%. Too
little: the function was still 182 an ask.

### The blocks

A print at the hand-up and the release said why. runbench's inner loop
tenures 1,600 bytes a run and finishes with a heap value, so its block is
handed up to the loop outside rather than freed — forty-nine times — and the
outer depth ends the phase holding forty-nine blocks of 256 KiB with 1,600
bytes in each. Every ask walked all forty-nine, and the asks miss: the
pointer the sizing walk asks about is a node the loop built this lap, sitting
in the arena above the mark, which no tenure block can hold because tenure
blocks are malloc'd. So `k_survives_x` now asks that first — is the pointer in
the arena chain between the head block and the mark's block — and answers no
without touching the tenure blocks when it is. The chain above the mark is one
or two blocks.

    runbench   2,816,922,233 -> 2,762,899,581   −54,022,652   −1.9178%

on the container with clang 19, both changes together, the same bytes out.
`k_ten_holds` is 22,125,975 now, 110,420 asks, the ones for pointers that are
not in the arena at all: the carry buffer's, the tenured ones, the hits.

No counter moves. `ten_blocks` and `ten_frees` read 55 either way, the twelve
veins and the lazy tier agree with their goldens, and the emitted code is the
compiler's. The work vein is the only witness, as it was for the inline pins,
and the ratchet row `ten_walk` puts the tenure walk back in front of the
arena check so that vein goes red.

### A thread left open

Forty-nine 256 KiB blocks for 78 KB of tenured bytes is address space rather
than pages, and no counter reads it, so it is not a regression by the
project's own measures. It is still forty-nine mallocs and frees a phase for
a block that could hold three hundred laps of tenure. A hand-up that appended
the child's bytes into the parent's block cannot be written — the carried
values hold pointers into the child's block — but a hand-up that kept the
child's block as the parent's head, and an inner loop that opened its tenure
in a block the parent already has room in, might. Not measured.

## 2026-09-07 — THE LENGTH OF AN INDEXED CHARACTER, ANSWERED WITHOUT A SCAN

runbench's index phase reads a string one character at a time and asks each
one's length: `acc + length s[i]`, 690,000 times. `k_b_length` was 51,794,417
instructions, 1.87%, and the per-address profile said where: 345,220 of its
875,119 calls scanned. `s[i]` hands back a fresh string of one character, and
the count memo a string keeps in its header was empty on every one of them,
so `length` walked the character's bytes — one to four — through
`k_utf8_chars`, with its wide-loop prologue, its tail loop and its two
statistics increments around it. The other 529,898 calls read a memo, then
paid the call, the jump table and the six pushed and popped registers to get
to it.

### The index knows the count

A one-character string has one character. `k_b_at`'s string arm writes the
memo before it hands the string back — one store, on the path that already
built the header. The ascii cache's strings get it once; a multi-byte
character's fresh string gets it every time.

    runbench   2,762,899,581 -> 2,749,444,509   −13,455,072   −0.4870%

`str_scans` on runbench 345,257 -> 254 and `str_scan_bytes` 23,679,615 ->
22,644,612 in `bench/cost_golden_run.txt`; no other vein moves, since no
other benchmark reads text a character at a time. The fixture
`the_length_of_an_indexed_character_needs_no_scan` walks 4,000 characters of
a six-character alphabet, three of them multi-byte, and pins `str_scans=12`:
the subject's own count, the doublings that built it, and the ascii cache's
first sight of its three ascii bytes. The old runtime counts 2,015 on it —
one more per multi-byte position — watched red. The ratchet row `one_char_memo` writes the memo as "not
counted" and the mem corpus goes red on that row.

### The twin reads the memo

`k_b_length_fast`, the twin the emitter inlines at every `length`, answered
lists and bytes from their headers and sent everything else to C. A string
whose header holds a count is the same shape: a tag compare, a load of the
`cap` field, a sign test and a complement. The C entry keeps the scan and
its counters, so nothing a golden reads is skipped.

    runbench   2,749,444,509 -> 2,739,572,213    −9,872,296   −0.3591%

The emitted goldens move by the twin's definition: `emitted_lines` 9,237 ->
9,252 and `emitted_branches` 820 -> 822 on the decoder, `emitted_other_lines`
132,622 -> 132,817 and `emitted_other_branches` 12,671 -> 12,697 across the
thirteen, fifteen lines and two branches each, with `emitted_calls` and
`emitted_other_calls` unchanged. No counter moves. The work vein is the
witness, and the ratchet row `length_memo` sends every string back through
the call.

Together with the tenure walk above, the container reads runbench
2,816,922,233 -> 2,739,572,213, −2.746%, on clang 19, the same bytes out.

### On CI, and two rises that were code shape

CI's first sitting of the pair, main -> the branch, on the runner:

    work_runbench     2,816,922,887 -> 2,739,482,932 -  77,439,955  −2.7491%
    work_indexbench       4,771,217 ->     3,919,315 -     851,902 −17.8550%
    work_scanbench      766,291,267 ->   739,196,984 -  27,094,283  −3.5358%
    work_widebench       53,465,568 ->    52,982,106 -     483,462  −0.9042%
    work_deepbench      690,817,043 ->   703,185,109 +  12,368,066  +1.7904%
    work_livebench    3,743,207,118 -> 3,771,674,281 +  28,467,163  +0.7605%
    work_encodebench  4,194,027,086 -> 4,208,546,793 +  14,519,707  +0.3462%
    work_digestbench     70,784,439 ->    71,305,273 +     520,834  +0.7358%
    work_oneshot         22,579,660 ->    22,671,583 +      91,923  +0.4071%
    work_basket          35,737,604 ->    35,791,600 +      53,996  +0.1511%
    work_pendbench      620,687,423 ->   620,868,615 +     181,192  +0.0292%
    work_escapebench     85,754,925 ->    85,763,927 +       9,002  +0.0105%
    work_jsonbench    1,561,185,741 -> 1,561,186,402 +         661  +0.0000%
    work_readbench        4,283,427 ->     4,284,088 +         661  +0.0154%

The four falls are the change. Ten rows rose, three of them by ten million
or more, and neither change has a path those programs take: deepbench,
encodebench and livebench hold no tenure block at all (`ten_blocks=0` on all
three), so the arena check never runs, and every `length` they ask is a
list's, so the twin's string arm is never entered. The container reproduces
both large rises to the instruction — deepbench 690,908,520 -> 703,276,586,
encodebench 4,194,027,170 -> 4,208,546,905 — and the per-address profile says
what they are.

**deepbench is register pressure.** `k_copy_size` and its recursion went
388,937,919 -> 400,869,541 over the same 384,835 calls, and the disassembly
shows why: with `k_above_mark` inlined through `k_survives_x` into the sizing
walk, the function grew 3,706 -> 4,431 bytes, its prologue went from twenty
instructions to twenty-four — four more spills, on all 2,777,846 entries —
and the hot path reloads a block pointer it used to keep in a register. The
walk itself never ran. The repair is to put the arena check inside the
outlined `k_ten_holds(p, m)`, behind the same `k_ten_any && m` test that
outlined the tenure walk before, so the sizing walk's own code is what it was.
deepbench 691,251,495 on the container, 342,975 above main; runbench
2,739,492,779 against 2,739,482,932 with the check inline, a wash.

**encodebench is a lost fold.** `encode_onto` gained 15,586,000, and the
histogram of its instructions by execution count puts all of it on two paths:
four instructions more on one taken 3,344,400 times and two more on one taken
1,104,400 times. The first is `length es < i` in the pair loop. On main the
twin's "list or bytes" test is two compares that instcombine folds to one,
`(tag | 4) == 13`, and the loop keeps the folded value in a stack slot: two
instructions. With the string arm's third compare on the same `%tag`,
SimplifyCFG gathers the three into a switch first and lowers the switch as a
chain — six instructions — and the fold never happens. The twin now writes
the fold out, `%t4 = or i64 %tag, 4` and one compare, and a compare on a
different value is not a case of that switch. The pair loop is back to two
instructions, and runbench falls a further 3,342,208 for the same reason, to
2,736,140,571 on the container: −2.8681% against main's 2,816,922,233 for
the pair.

What is left of encodebench after that is 4,206,674,837, +12,647,667 over
main, and it is not a path: the histogram shows one instruction more on three
paths (taken 11,658,800, 4,190,000 and 3,480,800 times) and two fewer on a
fourth (2,792,000), a spilled value reloaded in the escape loop. The twin's
two new blocks inline at every `length` in the function and the register
allocator makes different choices around them. Held as the price of the memo:
runbench is the row the objective weighs and it falls; encodebench is a
diagnostic, and this is where its 0.30% went. livebench, the same program on
the shipped library, reads 3,743,461,255 with both repairs, 254,137 over
main's 3,743,207,118: the allocator lays the shipped encoder out differently,
and its 28,467,163 was the two repaired shapes and nothing else.

**The other veins.** `text` 1,451,772 -> 1,474,748 across the fourteen
binaries, 976 to 2,944 bytes each: the twin's string and count arms inline at
every `length`, and `k_ten_holds` carries the arena walk. `lines` 5,918 ->
5,993 and `branches` 372 -> 382 over the five compile samples, `module_lines`
5,232 -> 5,247 and `module_branches` 434 -> 436: the twin's fifteen lines and
two branches arrive through the preamble every program carries, and none of
the six samples asks a string's length. `compile_instructions` 19,315,995 ->
19,317,810, 1,815 of layout, the compiler's own bytes having moved.

The ratchet gains `length_fold`, which writes the two compares back and asks
the work vein. Welfare 57.25 -> 57.44, held.

CI's second sitting, with both repairs in, main -> the branch:

    work_runbench     2,816,922,887 -> 2,736,141,165 -  80,781,722  −2.8677%
    work_scanbench      766,291,267 ->   737,694,496 -  28,596,771  −3.7318%
    work_indexbench       4,771,217 ->     3,999,487 -     771,730 −16.1747%
    work_widebench       53,465,568 ->    53,095,410 -     370,158  −0.6923%
    work_encodebench  4,194,027,086 -> 4,206,674,765 +  12,647,679  +0.3016%
    work_digestbench     70,784,439 ->    71,338,042 +     553,603  +0.7821%
    work_deepbench      690,817,043 ->   691,249,951 +     432,908  +0.0627%
    work_livebench    3,743,207,118 -> 3,743,461,197 +     254,079  +0.0068%
    work_pendbench      620,687,423 ->   620,751,169 +      63,746  +0.0103%
    work_basket          35,737,604 ->    35,752,074 +      14,470  +0.0405%
    work_oneshot         22,579,660 ->    22,580,295 +         635  +0.0028%
    work_jsonbench    1,561,185,741 -> 1,561,186,047 +         306  +0.0000%
    work_readbench        4,283,427 ->     4,283,433 +           6  +0.0001%
    work_escapebench     85,754,925 ->    85,754,928 +           3  +0.0000%

The runner agrees with the container to within a thousand on every row that
moved: deepbench's 12,368,066 is 432,908, livebench's 28,467,163 is 254,079,
and encodebench keeps its 12,647,679 of register allocation. digestbench's
553,603 and basket's 14,470 are the same allocator noise at a smaller scale,
and the six rows under a thousand are layout. `text` reads 1,451,772 ->
1,451,346 after the two repairs, every one of the fourteen binaries a few
hundred bytes smaller than main; `compile_instructions` 19,315,995 ->
19,319,033, layout again. Welfare 57.44 -> 57.45, held.

**Declined on the way: a settled top-of-stack mark for `k_beat_iter`.** The
iteration entry recomputes the depth to find its mark; a pointer kept settled
at push, pop and the five other sites that move the depth would save that.
Built and measured: runbench 2,739,572,213 -> 2,748,365,853, +0.3210%. The
rewind recomputes the depth for `k_reg_any` regardless, and seven settle
sites cost push and pop more than the iteration saved. Reverted.

### The ratchet's modulo row went blind at kanso#1292

This PR's third round turned the ratchet job red on a row it did not write:
`numeric`, whose mutation `native_floors_a_negative_modulo` patched the
runtime's `k_mod` to floor, reported BLIND -- the numeric differential stayed
green with the floor in place. kanso#1292 made the emitter write two proved
integers' remainder as one `srem` and send only the zero and minus-one
divisors to the call, so the runtime arm the mutation patched stopped
answering any case the sweep writes, and nothing noticed until this branch
touched src/runtime.c and the touched-rows check selected the row. The
mutation now patches the emitter, `srem` to `urem`, and the sweep reads
`print (-1 % 2147483648)` as 2147483647 on native against the oracle's -1:
watched red on the container before this round was pushed.

---

## 2026-09-07 — AN INNER BEAT OPENS ITS TENURE IN THE BLOCK OUTSIDE

The thread the tenure-walk entry left open. runbench's inner loop tenures
1,600 bytes a run and ends with a heap value, so its 256 KiB block is handed
up to the depth outside at every pop rather than freed, and the outer depth
ended the phase holding forty-nine blocks for 78 KB. The above-mark check
stopped the sizing walk from asking those blocks about arena pointers; the
asks that are not arena pointers — the carry buffer's, the tenured ones, the
hits — still walked all forty-nine, 22,125,975 instructions over 110,420 asks.

### One block, not one a lap

`k_ten_alloc`, asked by a beat that holds no block yet, now looks at the
depth outside first: when that depth's head block has room for the request
and its bytes are under the licence, the bytes are carved from it. They are
accounted to the outer depth and freed with it, which is where a handed-up
block's bytes went anyway; the inner beat never owns a block, so its pop has
nothing to hand up and nothing to free. The first lap still opens a block —
the outer depth has none until the first hand-up — and every lap after that
lands in it.

    runbench   2,736,140,571 -> 2,717,267,333   −18,873,238   −0.6898%

on the container with clang 19, the same bytes out. `k_ten_holds` is
10,920,518 now. `ten_blocks` and `ten_frees` in `bench/cost_golden_run.txt`
read 55 -> 6; no other counter in the twelve veins or the lazy tier moves,
and widebench and scanbench, which tenure at one depth with nothing outside
it, still read one block each. The trend gate reads the pair as
`run_ten_blocks` 55 -> 6, improved, and `run_ten_frees` 55 -> 6, worsened,
because a free is a thing to want more of when the blocks are held fixed;
here the blocks fell with them, and `run_ten_frees` lands on 6 because six
blocks were opened and all six were freed, where fifty-five had been.

### What a program that kept its own blocks would lose

A beat whose result is not heap frees its block at the pop. With the sharing,
its bytes sit in the outer block until the outer depth pops — a few kilobytes
of garbage a lap in a block that would otherwise have been mapped and
unmapped a lap. The licence check on the outer depth's bytes bounds it the
way `K_TEN_CAP` bounds any depth, and the outer depth's own asks answer yes
for those bytes exactly as they did when the block was handed up whole.

The fixture `an_inner_beat_opens_its_tenure_in_the_block_outside` runs the
repaired-node fixture's inner loop under five laps of an outer one and pins
`ten_blocks=3`: the laps tenure 363,552 bytes over 7,336 requests, a 256 KiB
block fills every two laps and change, and the third opens for the last
sixteen bytes of the run. The runtime before it read 5, one a lap and each
five-sixths empty, watched red with every other counter identical. The
ratchet row `ten_open` makes every inner beat open its own block again and
asks the run program's counters — the first row that gate has had since
runbench joined on 2026-09-06.

---

## 2026-09-07 — A BEAT POP WITH NOTHING TO DO

`k_beat_pop` on runbench: 507,685 pops, 36,026,319 instructions, 71 a pop.
Its rewind twin got a fast path on 2026-09-05 — `k_beat_rewind` tests the
dirty flag, the depth's registry summary and the mark's block and takes the
arena back in four stores — and the heap-result pop kept its full frame.
That pop deep-copies a carried result, migrates the three registries and
hands the depth's tenure blocks up, and on runbench 507,678 of the 507,685
carry nothing, have nothing registered and hold no tenure block: the copy
was skipped by its flag, the three migrates each found an empty registry
and wrote three zeros over zeros, and `k_ten_hand_up` returned at its first
line. What every one of them paid was the frame those calls need — six
callee-saved pushes and pops around four tests that all said no.

### The four tests, then the frame

The pop's work moves out of line into `k_beat_pop_slow`, the split the
rewind made; `k_beat_pop` itself tests the result's tag, the carry flag,
`k_reg_any[d]` and `k_ten_blocks[d]` and returns the result when all four
are clear. `k_reg_any` is the summary the rewind already trusts: every add
to the chunk, view and permanent registries sets its bit, every migrate or
flush clears it, and the chunk spill count travels with the chunk bit, so a
clear summary is a proof that the three migrates would have done nothing.
A pop with a tenure block still hands it up, and a carried result is still
copied, on the slow side.

    runbench   2,717,267,333 -> 2,692,922,207   −24,345,126   −0.8959%

on the container with clang 19, the same bytes out (md5 e8e74ccb…). The
pop is 14,517,216 now, 28.6 an execution, and the seven that take the slow
side are the ones with a block to hand up. No counter in the twelve veins or
the lazy tier moves: `arena_blocks`, `beat_iters`, `evac_allocs`,
`survive_slots`, `ten_blocks` and `ten_frees` print the same run to run, and
`all_counters.sh` agrees with every golden. The ratchet row `pop_fast` puts
the frame back under every pop and asks the work vein, which is the only
witness this change has: a pop that does nothing leaves no counter behind.

**OPEN.** The seven slow pops are runbench's outer laps, and each pays the
hand-up plus the frame; nothing to take there. The four tests are 28
instructions because two of them index by depth from a global base, and a
per-depth record holding the carry flag, the registry summary and the block
head would make them one load each. `k_beat_rewind` reads two of the same
fields, so the record would pay twice.

---

## 2026-09-07 — A PUSH OFF THE FRONTIER WALKED THE CHAIN TWICE TO PLACE ITS HEADER

`k_b_push_into_proven` on runbench: 362,602 calls, 43,992,462 instructions
self, 121 a call, and 300,063 of the calls grew. The decoder opens an array
with the empty literal, whose buffer holds one slot, and the second push
outgrows it: 214,000 growths a run from a one-slot buffer, 291,742 of the
growths arriving from the in-place push, which had already asked the
frontier question and found the answer no. Each growth then asked
`k_outlives_beat` whether the header predates the beat, and that walked the
block chain twice -- 550,142 loop iterations for 276,218 asks -- to learn
that a header born a few bytes below the bump pointer is in the head block.
After it, glibc's memcpy was called to move the one element, twenty-five
instructions to move sixteen bytes, and the frame around all of it was sized
by the refusal's 128-byte message buffer.

### Four small things, one push

- `k_outlives_beat` answers from the head block without a walk, the test
  `k_born_this_beat` already makes: a header there predates the beat exactly
  when the innermost mark sits in the same block above it. The walks remain
  for a header anywhere else.
- The growth moves into `k_b_push_grow`, which the in-place push calls
  directly once its own frontier test has failed; the general push reaches
  it the same way after its own. Neither re-asks what the other answered.
- The refusal, "push takes a list and a value", is out of line and cold, so
  its buffer no longer sizes the frontier arm's frame.
- A list of four elements or fewer is copied by a loop, the split `k_rec`
  made on 2026-09-05; so is the list literal's payload in `k_mklist`, 316,478
  calls a run of which the empty literal is most, and a closure's captures
  in `k_closure`, 243,978. Thirteen to sixteen instructions each for memcpy
  to learn it had nothing, or one thing, to move.

    runbench   2,692,921,601 -> 2,674,319,744   −18,601,857   −0.6908%

on the container with clang 19, the same bytes out. Against main, with the
tenure-block and pop entries above it: 2,736,140,571 -> 2,674,319,744,
−2.2594%. No counter in the twelve veins or the lazy tier moves;
`all_counters.sh` agrees with every golden. The ratchet row `push_walk` sends
every header back through the two walks and asks the work vein, the only
witness a walk that reaches the same answer leaves.

### The one that lost

`k_b_at` builds a one-character string 345,000 times a run for the
multi-byte half of its calls, two to four bytes through `k_str_n`'s memcpy
at seventeen instructions a call. A byte loop in its place, with the count
memo written directly: `k_b_at` 76,518,176 -> 83,188,192, +6,670,016,
against the 5,865,000 the memcpy calls had cost. The loop's bound is a
variable where the copies above have a constant, and a variable-count byte
loop is dearer than the call it replaced. Reverted before the sweep; the
k_b_at attribution in the length entry stands as written.

---

## 2026-09-07 — A SHORT UTF-8 RUN VALIDATED A BYTE AT A TIME AROUND ITS WIDE CHARACTERS

`k_utf8_bad_scalar` on runbench: 134,442 calls, 25,621,497 instructions, 190
a run. It is the arm for a run of thirty-two bytes or fewer that carries a
byte with the high bit set -- the door's ascii predicate has already turned
away every run that carries none -- and it walked the grammar a byte at a
time, ascii included: 938,619 loop iterations for 232,551 wide characters,
117,018 of them two bytes and 115,533 three. The ascii around the characters
was 706,068 of the iterations, and each was a load, a test and a branch back.

### A word at a time, and the tail as the last word

The ascii arm reads eight bytes as one word, as `k_all_ascii` does. A clean
word is eight bytes in three instructions; a word with a high bit in it says
where, by counting trailing zeros, and the walk lands on that byte rather
than testing its way there. Runbench's strings are short, so after the words
came a tail of up to seven bytes and 444,609 single steps: the tail reads the
run's last word, overlapping bytes already walked, with those bytes shifted
out. A run under eight bytes still steps.

    words   runbench   2,674,319,744 -> 2,667,323,216   −6,996,528   −0.2616%
    tail    runbench   2,667,323,216 -> 2,665,704,368   −1,618,848   −0.0607%

on the container with clang 19, the same bytes out; `k_utf8_bad_scalar` is
17,006,121. The differential harness under `scripts/utf8_differential`, which
extracts the arm's text from the source and runs it against an independent
scalar decoder, passed 45,189,025 cases and 8,346,016 counts with 0
mismatches after each step. No counter in the twelve veins or the lazy tier
moves -- `utf8_bytes` counts bytes handed in, not steps -- and
`all_counters.sh` agrees with every golden. The ratchet row `scalar_words`
closes both word arms and asks the work vein.

**OPEN.** What is left in the arm is the grammar itself, twenty or so
instructions a wide character through an if-else ladder on the lead byte,
4.6 million over the run. A 256-entry table keyed on the lead byte would
give the width and the continuation range in one load. Not measured.

---

## 2026-09-07 — A TOKEN SLICE OF FOUR TO SEVEN BYTES COPIED THROUGH A CALL

`k_b_utf8_slice_raw` on runbench: 861,498 calls, 57,102,905 instructions
self and a memcpy call each, 12,893,364 inside glibc. It is `utf8` over a
slice of bytes, the decoder's way of making a token into a string, and the
arm is already short -- a clamp, the ascii test, the arena bump, the copy --
so what was left was the copy: 840,807 of the slices are four to seven
bytes, a key or a short value or a number's digits, and glibc's memcpy
spends fifteen instructions choosing how to move five bytes.

### Two overlapping words

A slice of four to seven bytes is copied as two four-byte words, the second
ending at the last byte, the shape `k_all_ascii` reads its short runs by.
Under four bytes and eight or over go through `k_str_n` as before; the
single-byte ascii slice still comes from the cache.

    runbench   2,665,704,368 -> 2,649,866,050   −15,838,318   −0.5942%

on the container with clang 19, the same bytes out; memcpy's self fell
12,612,105 and the arm's own 3,226,213, the call's argument moves and the
return. No counter in the twelve veins or the lazy tier moves;
`all_counters.sh` agrees with every golden. The ratchet row `slice_words`
sends every slice back through the call and asks the work vein.

The same profile names the calls glibc still takes for a handful of bytes:
`k_render_at` 579,291 for a number's digits into its string, `k_b_at`
345,000 for one character (declined in the push entry above, and the
reason holds), `k_map_lit` 273,339 for an object's pairs, `render_ryu`
191,070 inside the float rendering. The render's is next.

---

## 2026-09-07 — A RENDERED NUMBER'S DIGITS COPIED THROUGH A CALL

`k_render_at` on runbench: 579,981 calls, 45,299,443 instructions self and
a memcpy call each, 8,499,478 inside glibc. A number renders its digits into
a sixty-four-byte stack buffer and then builds the string from them through
`k_str_n`, whose copy is glibc's memcpy: fifteen instructions to choose how
to move a handful of digits, then the move.

### Sixteen, eight, and the call for what is left

The digits go over as one sixteen-byte copy, eight more past fifteen, and
the call for anything past twenty-three -- which a rendering does not
reach, a double's shortest round-trip form being at most twenty-four bytes
and an integer's at most twenty. Both sides have the room: the buffer is
sixty-four bytes, so the reads past the length are inside it, and `k_alloc`
rounds a string's storage up to sixteen, so sixteen bytes fit a string of
fifteen or fewer and twenty-four fit one of sixteen or more. A word loop
bounded by the length was tried first and cost twelve a call against the
call's fifteen; the fixed copies cost five. A one-digit number still comes
from the ascii cache through `k_str_n`, as it did: the first cut of this
allocated it, and the sweep said so -- `allocs`, `sh_str` and
`perm_allocs` moved on basket, pend, scan, run and the lazy tier, the
last one downward because the cache's own permanent cells were never made.

    loop    runbench   2,649,866,050 -> 2,648,129,405    −1,736,645   −0.0655%
    fixed   runbench   2,649,866,050 -> 2,638,957,305   −10,908,745   −0.4117%

on the container with clang 19, the same bytes out; `k_render_at` is
42,890,176. Seventeen renderings at the edges -- the largest and smallest
doubles, a denormal, both ends of the integer range, negative zero, a
twenty-digit whole, a repeating fraction -- print the same bytes on native
before and after, the longest twenty-three. No counter in the twelve veins
or the lazy tier moves; `all_counters.sh` agrees with every golden. The
ratchet row `render_words` makes the first copy exact, which sends every
rendering back through the call, and asks the work vein.

### What the edge probe found instead

The probe's first value overflowed on the oracle, and the oracle panicked:
`render_float` in src/eval.rs asks Rust's `{:e}` for the digits and expects
an `e` in the answer, and `inf` has none. Native renders the same value as
`1.797693134862316e+308` -- the largest double's digits for a value that is
not a double's -- and `inf - inf` as `2.696539702293474e+308`. Neither
engine has an answer for an infinite or nan float, no golden prints one,
and this log, the archive and the ledger have never mentioned the case.
What `"{x}"` says for such an x is a language surface, so it is filed in
design/pending-gavels.md rather than decided here; the copy above changes
nothing about it.

The profile's remaining calls for a handful of bytes: `k_b_at` 345,000 for
one character, declined above; `k_map_lit` 273,339 for an object's pairs;
`render_ryu` 191,070 inside the float rendering.

### The map literal, and a record's marker arm

The same profile named `k_map_lit`: 273,339 calls, every one the empty
literal the decoder opens an object with, and every one copying its no
pairs through glibc's memcpy at thirteen instructions. The split `k_rec`
and `k_mklist` make, at two pairs: runbench 2,638,957,305 -> 2,634,857,220,
−4,100,085, −0.1554%, the same bytes out, and the ratchet row
`map_lit_copy` sends the literal back through the call.

`k_rec` itself is 1,003,448 calls at 52, five callee-saved pushes around a
failure scan, an arena bump and a copy of two or three fields. Moving the
nullary-record arm -- the marker cache, 203,049 of the calls -- out of line
to shrink the frame measured +1,827,441: the call cost the marker arm more
than the frame cost the rest. Reverted.

### The buffer's size class, answered by the trailing zeros

`k_buf` is 1,431,562 calls a run on runbench, and `k_buf_class` inside it
answered which free list a capacity belongs to by doubling from four until
it reached the capacity: 1,255,865 rounds of a six-instruction loop, 5.6 a
call, eleven for a capacity of 8,192 and five for the decoder's arrays at
64. The classes are `4 << c`, so the trailing zeros of a power of two say
the class in one instruction, and a test for a power of two says whether
there is one. runbench 2,634,857,220 -> 2,621,939,707, −12,917,513,
−0.4903%, the same bytes out; `k_buf` 51,214,420 -> 40,157,120, and
`buf_reuse`, `allocs` and `alloc_bytes` print the same, so the classes
answered are the classes that were. The ratchet row `buf_class_loop` puts
the doubling loop back and asks the work vein.

### The two-byte scan's tail, a word at a time: declined

`k_b_find2_below_raw` is the escape scan's search for a quote, a backslash
or a control byte: 1,353,330 calls on runbench, 84,989,250 instructions,
and 1,207,350 of the calls never fill a sixteen-byte vector, so the byte
loop after it walked 5,504,670 bytes at ten instructions each. A word at a
time for that tail, the trick the utf-8 arm uses, with the two equalities
and the floor folded into one mask: runbench 2,621,939,707 ->
2,681,926,147, +59,986,440, +2.2879%, and the scan itself 84,989,250 ->
140,449,770. A run between escapes is a few bytes and often one, and the
mask's setup -- three splats, two xors, three subtractions and the shift
for the tail -- costs more than the bytes it would have walked. Reverted;
the vector loop keeps the long runs and the byte loop the short ones.


### An indexed wide character, copied as one word

`k_b_at` on a string is 690,000 calls a run on runbench, 76,518,176
instructions self, and for the 345,000 that land on a wide character --
two, three or four bytes -- it built the string through `k_str_n`'s memcpy,
seventeen instructions for glibc to choose how to move three bytes. The
push entry above declined a byte loop bounded by the width, +805,016. The
character goes over as one four-byte word now, and both sides have the
room: `at + w <= len`, so a four-byte read from `at` reaches the string's
own terminator at worst, and the write lands inside storage `k_alloc`
rounded up to sixteen. runbench 2,621,939,707 -> 2,612,624,701,
−9,315,066, −0.3553%, the same bytes out; `k_b_at` 73,068,170, glibc's
memcpy 5,977,361 lighter. No counter in the twelve veins or the lazy tier
moves; the ratchet row `char_word` sends every wide character back
through the call.

### A survivor asked two walks of the chain

The sizing walk that decides whether a trip's result is worth copying
asks of every node whether the arena keeps it and, when it does not,
whether a tenure block holds it. `k_survives` walked the block chain to
answer the first, and `k_ten_holds` walked it again through
`k_above_mark` to rule a pointer above the mark out before it read the
tenure blocks: 661,528 chain steps for 333,873 asks, and a second call
with its own frame for 323,356 of them. `k_where` walks the chain once and
says below, above or outside, and `k_survives_x` reads the tenure blocks
only for outside; `k_ten_holds` keeps the two-step shape for the callers
that still want it. runbench 2,612,624,701 -> 2,606,982,659, −5,642,042,
−0.2160%, the same bytes out; `k_ten_holds_outside` is asked 122,915
times where `k_ten_holds` was asked 270,781.

A string built by `k_str_alloc` keeps its bytes right after its header, so
a header the walk has just found not to survive has bytes that do not
survive either, and the K_STR arm asked the walk again of them anyway. It
asks only of storage that lives elsewhere now -- a slice's, a builder's
-- and the K_BYTES arm the same. runbench 2,606,982,659 -> 2,602,519,000,
−4,463,659, −0.1712%; together −10,105,701, −0.3868%, and against main
−4.8836%. `k_copy_size` 54,253,637 self -> 41,602,877. No counter in the
twelve veins or the lazy tier moves. Ratchet rows `walk_once` (runbench
2,609,054,586 with the second walk back, +6,535,586) and `data_beside`
(2,606,982,659 with the string's bytes walked again, +4,463,659), each
watched.

Declined on the way: deciding an immediate's zero before the walk's frame,
by an inline wrapper over `k_worth_sizing` at every call. 176,112 of the
walk's 509,985 entries were immediates returning zero through six
callee-saved pushes, and the wrapper measured runbench +1,777,234,
+0.0680%: the guard at seventeen call sites, most of them already guarded
by the loops that make them, cost more than the frames it spared.

### CI's sitting for the ten, and the two rows that rose

CI's rows for kanso#1294 at 3ebac68e: runbench 2,736,141,165 ->
2,602,519,654, −4.8836%, and every other work row fell with it,
encodebench −3.18%, livebench −3.57%, jsonbench −4.09%. The text vein fell
11,600 bytes summed over the fourteen binaries. Welfare 57.45 -> 57.80,
held with `--set`. Two rows rose and the trend gate names them:
`compile_instructions` landed on 19,319,117, +84 over 19,319,033, the
runtime's bytes moving under the compiler that carries them (§48 says why
that row moves on an edit to the compiler's own text); and `work_readbench`
landed on 4,283,478, +45 over 4,283,433, the read program's one trip
through the sizing walk and the beat pop paying for flags it never sets.
Forty-five instructions on a four-million row is the price of the ten
elsewhere, and the sum is what the objective weighs. The page gained §55
for the campaign, which is the entry page_drift was owed at six entries
against a budget of three.
## 2026-09-07 — EVERY CONSTANT IS BUILT ONCE ON NATIVE, AS THE INTERPRETER ALREADY DOES

**DONE.** A zero-argument definition is a constant, and the interpreter has
computed every one of them once, behind a knot cell, since the knot work of
2026-08 (`eval_ident` sends every constant through `knotted`). Native froze
only a constant whose body was a literal -- an int, a string of literal
parts, a list or map of literals -- and a knotted one; every other constant
was an ordinary nullary function, recomputed at every mention. The archive's
2026-08-30 entry saw this ("any module-scope constant computed by a call is
rebuilt per use, language wide") and left it for a gavel on one ground: a
frozen constant was then built before main by `k_caf_init`, so a body that
could fail would have failed earlier than the interpreter fails it. The
2026-08-23 ruling took that ground away -- a cell is filled on first demand
now -- and the gavel was never filed in the ledger, so the question stood
answered by the oracle and nobody had written it down.

Found by profile. runbench's digest phase is sha256, whose sixty-four round
constants are eleven literal lists (six to a line, for the width limit)
joined by `text/concat` into `rounds`; `compress` reads `rounds[at]` once a
round, and each read rebuilt the table: ten concats, sixty-four pushes
through `list/to_list`, 16,000 times a run. `k_b_concat` alone was 26.6M
instructions of runbench, and every one of them built a table whose value
never changes.

`is_constant_body` answers yes for every zero-argument definition now. The
one case the widening broke was the corpus's own: `x = [x]` reached through
`play = err x` rendered `[[<cycle>]]` on native against the interpreter's
`[<cycle>]`, because freezing `play` copied `x`'s already-frozen list into a
second buffer and the cycle came back to the first. A frozen buffer is
immortal, so `k_caf_freeze` registers each one and `k_survives_x` answers
yes for a pointer inside any of them; the copier keeps the identity, the
fixture prints `[<cycle>]` on both engines again, and a beat that carries a
reference to a constant no longer copies the constant into its carry buffer
either (it did, for every literal constant, since the day they froze).

On the container with clang 19, the same bytes out of every benchmark:

    runbench   2,602,519,000 -> 2,487,359,798   −115,159,202   −4.4249%

and the allocation veins, which the work vein cannot see, move further:
runbench `allocs` 8,027,805 -> 7,659,778, `alloc_bytes` 602,780,749 ->
503,456,077, `arena_peak_bytes` 156,045,008 -> 57,478,864 (−63.2%),
`arena_blocks` 148 -> 54, `push_mut_fast` 1,937,322 -> 961,383 and
`sh_buf` 219,712,208 -> 123,205,712, the sixty-four pushes a round gone;
digestbench `allocs` 213,707 -> 23,841 and `arena_peak_bytes` 54,525,952 ->
3,145,728, its `push_mut_fast` 514,511 -> 10,956. Every benchmark's
`perm_allocs` rises by one to eight -- a `KCarryBuf` header per constant
frozen -- and `evac_allocs`/`evac_bytes` by the constants' own copies, the
mechanism's stated price; `carry_dedup` 2 -> 3 and 70 -> 71 where a constant
is reached twice in one freeze. pendbench's `bytes_malloc` 133 -> 0 and
`perm_live_bytes` 629,328 -> 0: its bytes constant was malloc'd at every
mention and lives in one frozen buffer now, which the freeze counts as
`evac_bytes` rather than as permanent live bytes. The lazy tier moves the
same way on the fixtures whose `play` is a constant with a value, which is
most of them: a copy of the result at the end, once.

The compile veins move with the emitted code, since every constant now
carries a `_build` symbol and a cache in front of it: `emitted_code` rises,
the decoder 139 -> 141 defines and 9,252 -> 9,297 lines, runbench 581 -> 591
defines and 34,773 -> 34,979 lines, the same shape on every program; that is
the cache's price and the work vein is what it buys. `compile_cost` moves the
same way, regenerated.

Ratchet row `freeze_all` puts the knot-only rule back and asks the work vein.
The golden corpora agree on both engines after the fix above; the wasm
backend keeps its own rule (`const_cell` answers only a knotted name) and
still rebuilds every other constant at each mention. That is a cost and not
a divergence: an effect in a constant's body is a value until it is
presented to IO, so `stamp = print "built"` mentioned twice renders
`<io><io>` on both engines and prints nothing, and a wasm program says the
same bytes either way. Aligning wasm is a thread for the wasm engine's own
sake, not this entry's. CI's work rows and welfare `--set` follow in the
next round.

### Every counter this moved, with the value it landed on

The trend gate wants each key by name. Outside the lazy tier:

  run_buf_reuse 56,651
  run_evac_allocs 15,994
  run_evac_bytes 8,587,152
  run_perm_allocs 91
  run_push_mut_fast 961,383
  evac_allocs 24
  evac_bytes 784
  perm_allocs 5
  encode_evac_allocs 38
  encode_evac_bytes 1,248
  encode_perm_allocs 11
  oneshot_evac_allocs 24
  oneshot_evac_bytes 768
  oneshot_perm_allocs 9
  basket_evac_allocs 3
  basket_evac_bytes 192
  basket_perm_allocs 26
  branches 387
  calls 235
  defines 204
  lines 6,098
  module_branches 437
  module_calls 759
  module_defines 100
  module_lines 5,270
  pend_evac_allocs 2,673
  pend_perm_allocs 17
  pend_sh_buf 32,542,640
  emitted_branches 824
  emitted_calls 1,254
  emitted_defines 141
  emitted_lines 9,297
  emitted_other_branches 12,731
  emitted_other_calls 20,763
  emitted_other_defines 2,343
  emitted_other_lines 133,570
  escape_evac_allocs 3
  escape_evac_bytes 80
  escape_perm_allocs 2
  scan_evac_allocs 41
  scan_evac_bytes 8,976
  scan_perm_allocs 51
  wide_evac_allocs 265
  wide_evac_bytes 520,352
  wide_perm_allocs 7
  digest_buf_reuse 131
  digest_evac_allocs 58
  digest_evac_bytes 4,528
  digest_perm_allocs 38
  digest_push_mut_fast 10,956
  read_evac_allocs 24
  read_evac_bytes 752
  read_perm_allocs 4
  live_evac_allocs 32
  live_evac_bytes 944
  live_perm_allocs 8

and in the lazy tier, one fixture a line -- a frozen constant costs a
permanent header and the copy of its value, and a fixture whose `play` is a
constant with a value pays that copy once at the end:

  a_builder_handed_on_is_still_a_builder_evac_allocs 3, a_builder_handed_on_is_still_a_builder_evac_bytes 160, a_builder_handed_on_is_still_a_builder_perm_allocs 4, a_builder_handed_on_is_still_a_builder_survive_slots 4
  a_cluster_entered_by_a_tail_call_sweeps_evac_allocs 3, a_cluster_entered_by_a_tail_call_sweeps_evac_bytes 80, a_cluster_entered_by_a_tail_call_sweeps_perm_allocs 13, a_cluster_entered_by_a_tail_call_sweeps_survive_slots 4
  a_cycle_of_four_rewinds_once_a_trip_evac_allocs 3, a_cycle_of_four_rewinds_once_a_trip_evac_bytes 80, a_cycle_of_four_rewinds_once_a_trip_perm_allocs 14, a_cycle_of_four_rewinds_once_a_trip_survive_slots 4
  a_cycle_that_allocates_nothing_needs_no_bracket_evac_allocs 3, a_cycle_that_allocates_nothing_needs_no_bracket_evac_bytes 80, a_cycle_that_allocates_nothing_needs_no_bracket_perm_allocs 4, a_cycle_that_allocates_nothing_needs_no_bracket_survive_slots 4
  a_digest_holds_every_block_it_walked_buf_reuse 3, a_digest_holds_every_block_it_walked_evac_allocs 39, a_digest_holds_every_block_it_walked_evac_bytes 3,920, a_digest_holds_every_block_it_walked_perm_allocs 39, a_digest_holds_every_block_it_walked_survive_slots 4
  a_digest_holds_every_block_it_walked_push_mut_fast 124
  a_loop_invariant_capture_is_copied_every_rewind_evac_allocs 22, a_loop_invariant_capture_is_copied_every_rewind_evac_bytes 16,624, a_loop_invariant_capture_is_copied_every_rewind_perm_allocs 4, a_loop_invariant_capture_is_copied_every_rewind_survive_slots 504
  a_map_walk_builds_no_scratch_pair_perm_allocs 11, a_map_walk_builds_no_scratch_pair_survive_slots 32,004
  a_map_walk_builds_no_scratch_pair_ten_frees 0
  a_pushed_call_keeps_the_sweep_evac_allocs 3, a_pushed_call_keeps_the_sweep_evac_bytes 80, a_pushed_call_keeps_the_sweep_perm_allocs 2, a_pushed_call_keeps_the_sweep_survive_slots 4
  a_repaired_node_below_the_mark_holds_tenure_evac_allocs 3,685, a_repaired_node_below_the_mark_holds_tenure_perm_allocs 19, a_repaired_node_below_the_mark_holds_tenure_sh_buf 2,490,256, a_repaired_node_below_the_mark_holds_tenure_survive_slots 408
  a_repaired_node_below_the_mark_holds_tenure_bytes_freed 0
  a_transient_maps_view_is_freed_evac_allocs 7, a_transient_maps_view_is_freed_evac_bytes 192, a_transient_maps_view_is_freed_perm_allocs 5, a_transient_maps_view_is_freed_survive_slots 4
  an_accumulator_loop_reclaims_its_garbage_evac_allocs 3, an_accumulator_loop_reclaims_its_garbage_evac_bytes 80, an_accumulator_loop_reclaims_its_garbage_perm_allocs 14, an_accumulator_loop_reclaims_its_garbage_survive_slots 4
  an_escaped_list_gives_its_buffer_back_evac_allocs 3, an_escaped_list_gives_its_buffer_back_evac_bytes 80, an_escaped_list_gives_its_buffer_back_perm_allocs 2, an_escaped_list_gives_its_buffer_back_survive_slots 4
  an_inner_beat_opens_its_tenure_in_the_block_outside_evac_allocs 62,144, an_inner_beat_opens_its_tenure_in_the_block_outside_evac_bytes 33,957,408, an_inner_beat_opens_its_tenure_in_the_block_outside_perm_allocs 19, an_inner_beat_opens_its_tenure_in_the_block_outside_survive_slots 4,009
  an_unasked_equality_stays_a_cell_evac_allocs 3, an_unasked_equality_stays_a_cell_evac_bytes 80, an_unasked_equality_stays_a_cell_perm_allocs 4, an_unasked_equality_stays_a_cell_survive_slots 4
  an_undemanded_knot_allocates_nothing_evac_allocs 3, an_undemanded_knot_allocates_nothing_evac_bytes 80, an_undemanded_knot_allocates_nothing_perm_allocs 3, an_undemanded_knot_allocates_nothing_survive_slots 4
  append_in_place_evac_allocs 3, append_in_place_evac_bytes 160, append_in_place_perm_allocs 4, append_in_place_survive_slots 4
  append_of_a_slice_boxes_nothing_evac_allocs 5, append_of_a_slice_boxes_nothing_evac_bytes 240, append_of_a_slice_boxes_nothing_perm_allocs 5, append_of_a_slice_boxes_nothing_survive_slots 4
  beat_builder_evac_allocs 3, beat_builder_evac_bytes 80, beat_builder_perm_allocs 14, beat_builder_survive_slots 4
  beat_cycle_evac_allocs 3, beat_cycle_evac_bytes 80, beat_cycle_perm_allocs 14, beat_cycle_survive_slots 4
  build_cycle.imported_evac_allocs 10, build_cycle.imported_evac_bytes 368, build_cycle.imported_perm_allocs 9, build_cycle.imported_survive_slots 12
  builder_counts_once_evac_allocs 3, builder_counts_once_evac_bytes 80, builder_counts_once_perm_allocs 4, builder_counts_once_survive_slots 4
  builder_guard_evac_allocs 3, builder_guard_evac_bytes 112, builder_guard_perm_allocs 11, builder_guard_survive_slots 4
  builder_reclaim_evac_allocs 3, builder_reclaim_evac_bytes 80, builder_reclaim_perm_allocs 15, builder_reclaim_survive_slots 4
  builder_transient_evac_allocs 3, builder_transient_evac_bytes 80, builder_transient_perm_allocs 5, builder_transient_survive_slots 4
  early_exit_evac_allocs 3, early_exit_evac_bytes 80, early_exit_perm_allocs 3, early_exit_survive_slots 4
  effect_push_shape_evac_allocs 20, effect_push_shape_evac_bytes 672, effect_push_shape_perm_allocs 6, effect_push_shape_survive_slots 6
  fold_push_shape_evac_allocs 3, fold_push_shape_evac_bytes 96, fold_push_shape_perm_allocs 6, fold_push_shape_survive_slots 4
  force_path_evac_allocs 3, force_path_evac_bytes 80, force_path_perm_allocs 3, force_path_survive_slots 4
  fresh_builder_evac_allocs 3, fresh_builder_evac_bytes 80, fresh_builder_perm_allocs 13, fresh_builder_survive_slots 4
  fresh_cycle_evac_allocs 3, fresh_cycle_evac_bytes 80, fresh_cycle_perm_allocs 13, fresh_cycle_survive_slots 4
  fused_map_shape_evac_allocs 3, fused_map_shape_evac_bytes 96, fused_map_shape_perm_allocs 6, fused_map_shape_survive_slots 4
  fused_reducer_evac_allocs 3, fused_reducer_evac_bytes 80, fused_reducer_perm_allocs 3, fused_reducer_survive_slots 4
  fused_select_shape_evac_allocs 3, fused_select_shape_evac_bytes 96, fused_select_shape_perm_allocs 5, fused_select_shape_survive_slots 4
  fused_tally_evac_allocs 3, fused_tally_evac_bytes 80, fused_tally_perm_allocs 7, fused_tally_survive_slots 4
  growing_map_evac_allocs 3, growing_map_evac_bytes 80, growing_map_perm_allocs 13, growing_map_survive_slots 4
  lazy_verdict_is_per_arm_evac_allocs 7, lazy_verdict_is_per_arm_evac_bytes 208, lazy_verdict_is_per_arm_perm_allocs 5, lazy_verdict_is_per_arm_survive_slots 4
  many_cells_evac_allocs 3, many_cells_evac_bytes 80, many_cells_perm_allocs 3, many_cells_survive_slots 4
  map_put_evac_allocs 3, map_put_evac_bytes 80, map_put_perm_allocs 12, map_put_survive_slots 4
  piped_reducer_evac_allocs 3, piped_reducer_evac_bytes 80, piped_reducer_perm_allocs 3, piped_reducer_survive_slots 4
  readwrite_map_evac_allocs 3, readwrite_map_evac_bytes 80, readwrite_map_perm_allocs 13, readwrite_map_survive_slots 4
  record_fields_evac_allocs 3, record_fields_evac_bytes 80, record_fields_perm_allocs 2, record_fields_survive_slots 4
  record_reuse_shape_evac_allocs 3, record_reuse_shape_evac_bytes 80, record_reuse_shape_perm_allocs 5, record_reuse_shape_survive_slots 16,006
  repeated_key_shape_evac_allocs 3, repeated_key_shape_evac_bytes 96, repeated_key_shape_perm_allocs 16, repeated_key_shape_survive_slots 4
  returned_thunk_evac_allocs 3, returned_thunk_evac_bytes 96, returned_thunk_perm_allocs 3, returned_thunk_survive_slots 4
  reuse_guard_evac_allocs 3, reuse_guard_evac_bytes 112, reuse_guard_perm_allocs 12, reuse_guard_survive_slots 4
  shared_twice_evac_allocs 3, shared_twice_evac_bytes 80, shared_twice_perm_allocs 3, shared_twice_survive_slots 4
  skip_shape_evac_allocs 3, skip_shape_evac_bytes 96, skip_shape_perm_allocs 5, skip_shape_survive_slots 4
  skip_unused_evac_allocs 3, skip_unused_evac_bytes 80, skip_unused_perm_allocs 4, skip_unused_survive_slots 4
  skipped_err_evac_allocs 3, skipped_err_evac_bytes 80, skipped_err_perm_allocs 4, skipped_err_survive_slots 4
  sort_shape_evac_allocs 3, sort_shape_evac_bytes 80, sort_shape_perm_allocs 3, sort_shape_survive_slots 4
  stream_fold_evac_allocs 3, stream_fold_evac_bytes 80, stream_fold_perm_allocs 3, stream_fold_survive_slots 4
  stream_write_evac_allocs 12, stream_write_evac_bytes 480, stream_write_perm_allocs 15, stream_write_survive_slots 4
  string_builder_shape_evac_allocs 3, string_builder_shape_evac_bytes 80, string_builder_shape_perm_allocs 5, string_builder_shape_survive_slots 4
  string_headers_evac_allocs 3, string_headers_evac_bytes 80, string_headers_perm_allocs 4, string_headers_survive_slots 4
  take_shape_evac_allocs 3, take_shape_evac_bytes 96, take_shape_perm_allocs 5, take_shape_survive_slots 4
  tally_shape_evac_allocs 3, tally_shape_evac_bytes 96, tally_shape_perm_allocs 4, tally_shape_survive_slots 4
  the_length_of_an_indexed_character_needs_no_scan_evac_allocs 47, the_length_of_an_indexed_character_needs_no_scan_evac_bytes 62,160, the_length_of_an_indexed_character_needs_no_scan_perm_allocs 7, the_length_of_an_indexed_character_needs_no_scan_survive_slots 4
  the_same_capture_built_below_the_mark_is_shared_evac_allocs 14, the_same_capture_built_below_the_mark_is_shared_evac_bytes 8,416, the_same_capture_built_below_the_mark_is_shared_perm_allocs 4, the_same_capture_built_below_the_mark_is_shared_survive_slots 504
  unsafe_wrap_evac_allocs 3, unsafe_wrap_evac_bytes 80, unsafe_wrap_perm_allocs 4, unsafe_wrap_survive_slots 4

### Three specs pinned the rebuilt table without knowing it

`tests/sha256_peak.rs` pinned 7,340,032 arena bytes for a 1,024-byte hash
and exactly twice that for 2,048 -- seven kilobytes of arena a message byte,
which was the round table's garbage and not the hash's retention. Both
sizes read 1,048,576 now, one arena block, the floor nothing smaller can
show; the spec measures 65,536 and 131,072 bytes instead, pinned at
17,825,808 and 40,894,496, and asserts growth rather than an exact doubling,
because the arena grows by whole blocks. `tests/a_program_is_not_its_directory.rs`
pinned the peak under `lib/` at 27,262,976 for the same reason and reads
2,097,152 now, still not the 1,048,576 it reads elsewhere, so the defect it
watches is still there. `tests/cohort.rs` pinned `evac_bytes=400496` for two
cohorts' copies and carries 400,768 now: the freeze's copies are evacuations
too, 272 bytes of frozen constants over the same two cohorts. Welfare 57.80
-> 64.05 on the container's sitting, held with `--set`; CI's rows re-set it.

### CI's sitting for the freeze, and the rows that rose with it

CI's rows for kanso#1295 at aa46fa86: runbench 2,602,519,654 ->
2,490,112,330 (−4.3191%) and digestbench 67,957,547 -> 10,775,912
(−84.14%), the round table's rebuild gone from both. Welfare 64.05 ->
64.36, held with `--set`. Eleven work rows rose by small amounts and the
trend gate names each: `work_pendbench` landed on 608,937,924 (+3,365,047,
+0.56%), `work_encodebench` on 4,075,264,719 (+2,284,936), `work_livebench`
on 3,610,815,239 (+924,424), `work_widebench` on 51,944,211 (+294,528),
`work_deepbench` on 686,441,869 (+149,998), `work_escapebench` on
85,495,224 (+49,293), `work_basket` on 34,236,107 (+34,632), `work_oneshot`
on 21,864,579 (+21,030), `work_scanbench` on 736,173,290 (+5,060),
`work_readbench` on 4,288,146 (+4,668), `work_jsonbench` on 1,497,268,731
(+4,538) and `work_indexbench` on 3,732,426 (+3,720). Two costs are in
those numbers and the next entry prices them apart: `k_survives_x` walks
the chain from its head for every ask now, where a program with tenure off
walked from the mark's block, and it scans the frozen ranges for a pointer
in no block; and a frozen value reached from a carried value is a survivor
the sizing walk still repairs through, element by element, when nothing
inside a frozen buffer can ever need repair. pendbench's 629 KB constant is
the case that shows it, and it is the next change. The text vein rose
10,400 bytes summed, `text` landing on 1,449,180, one `_build` symbol and
its cache per constant; `compile_instructions` fell to 19,316,501.

## 2026-09-07 — A SURVIVOR'S BLOCK ASKED AFTER EVERY NEWER ONE

**DONE.** kanso#1295's CI sitting left eleven small work rows up, and the
previous entry named two costs for them. One of the two was wrong, and the
profile that would have said so was already on disk: `k_frozen_holds` is
asked 1,804 times in the whole of pendbench, 52,182 instructions. Nothing
reached from a carried value there is frozen, and the walk that entry
blamed on "pendbench's 629 KB constant" is the ordinary carry of the
accumulator list, which has cost the same since the list existed; the
`perm_live_bytes` the number came from were `text/join`'s malloc'd
builders. The other cost was the whole rise, and the first repair of it was
the wrong one too. Putting the tenure-off shortcut back in `k_survives_x`
-- `k_survives` from the mark's block, then the frozen ranges -- took
pendbench to 605,900,745 and encodebench to 4,073,121,388 and moved
runbench 2,487,359,798 -> 2,490,296,277, +2,936,479: `k_survives` walks the
older blocks for a node above the mark and finds nothing, and runbench's
chain is fifty-four blocks long at its peak.

Neither walk was the right one. `k_where` started at the head of the chain
and reached the mark's own block only after every block newer than it, and
`k_survives` started at the mark's block and walked every block older than
it for a node that was never there. The node the sizing walk asks about is
almost always in the mark's block: above the mark if the loop built it this
lap, below if the lap before left it. `k_where` asks that block first now,
then the newer blocks from the head down to it, then the older ones, and
answers outside only after all three. A mark with no block, which is how
the freeze copies everything, finds every arena node above as before.

    pendbench     608,937,982 ->   604,694,569    -4,243,413   -0.6968%
    encodebench 4,075,264,731 -> 4,072,255,544    -3,009,187   -0.0738%
    runbench    2,487,359,798 -> 2,483,621,153    -3,738,645   -0.1503%
    deepbench     686,441,869 ->   647,639,361   -38,802,508   -5.6527%
    widebench      51,944,211 ->    50,448,586    -1,495,625   -2.8793%
    livebench   3,610,815,239 -> 3,609,374,810    -1,440,429   -0.0399%
    basket         34,236,107 ->    34,124,075      -112,032   -0.3272%
    oneshot        21,864,579 ->    21,841,750       -22,829   -0.1044%

on the container with clang 19, the same bytes out; the six rows the
freeze had not moved shift by under 500 each, the container's sitting
against CI's: `work_jsonbench` lands on 1,497,268,438, `work_escapebench`
on 85,495,206, `work_indexbench` on 3,732,364, `work_scanbench` on
736,173,216, `work_readbench` on 4,287,853 and `work_digestbench` on
10,775,474. deepbench is the largest
by far and was never in the freeze's list: its fold over lists of ints
sizes a long list of survivors every pop, and the beat's block sits
under several newer ones, so every ask walked those first. pendbench and
encodebench are under their pre-freeze rows now, 605,572,877 and
4,072,979,783. `all_counters.sh` agrees with every golden: no counter
moves, so the work vein is the witness. Welfare 64.36 -> 64.38 on the
container's rows, held with `--set`.

CI's sitting at acf1d8b4 agrees with the container's to within twelve
instructions on every row but two: `work_runbench` lands on 2,483,620,655,
which against CI's own freeze row of 2,490,112,330 is -6,491,675, -0.2607%,
and `work_deepbench` on 647,637,793. The reordered walk is thirty-two more
bytes of machine code a program, forty-eight where a third block comes
into it: `text` lands on 1,449,676, +496 summed over the fourteen, and
`compile_instructions` on 19,316,936, +435, the layout vein moving with the
runtime the compiler carries. The floor is re-set on CI's rows. Ratchet row `mark_block_first`
puts the walk from the head back -- the mutant is byte-for-byte main's
`k_where` -- and asks the work vein; dry-run red before it was committed.
CI's rows and `--set` follow in the next round.

## 2026-09-07 — THREE MORE SHORTCUTS ON THE RUN PROGRAM'S TEXT PHASES

**DONE.** After kanso#1296 the run program's profile has no runtime function
above three per cent that is not the decoder's or the encoder's own body,
so this entry takes three of the text phases' costs together, each with
its row and its mutation.

**A wide character built in the arena at every index.** `s[i]` on text
hands back one character, and an ascii one has come from k_str_n's cache
of interned singles for a long time; a character of two, three or four
bytes was built in the arena every time, 690,000 times a run on the index
phase, whose subject is the six characters of `aé😀b€z` doubled to 1.4 MB.
k_b_at keeps a direct-mapped cache of 256 permanent strings now, keyed on
the character's own bytes, never evicting -- a slot another character
already holds sends this one down the arena path, so the permanent storage
is bounded by the width whatever the text does. The index refusal's message
buffer came out of line at the same time, since it was the largest thing in
the function's frame, and the cursor answers the next character directly:
a walk by index asks for the one after the one it just read at every step,
and the general walk's bookkeeping was most of what the step paid.
`allocs` on the run program 7,659,778 -> 7,314,778, one fewer per wide
character read, `alloc_bytes` 503,456,077 -> 492,416,077, and with the
index phase no longer filling blocks with one-character strings
`arena_blocks` 54 -> 43 and `arena_peak_bytes` 57,478,864 -> 45,944,528;
`run_perm_allocs` lands on 94 from 91, the three characters cached, and
`sh_str` on 54,603,840. The lazy tier's fixture for the indexed character's
length reads the same way: `the_length_of_an_indexed_character_needs_no_scan_allocs`
2,045 -> 45, `the_length_of_an_indexed_character_needs_no_scan_alloc_bytes`
109,936 -> 45,936, `the_length_of_an_indexed_character_needs_no_scan_perm_allocs`
landing on 10 from 7 and `the_length_of_an_indexed_character_needs_no_scan_sh_str`
on 8,064. k_b_at 106 -> 92 instructions a call.

**Four ascii blocks validated one at a time.** The wide utf-8 pass tests
each sixteen-byte block for ascii before it classifies it, twelve
instructions to learn that sixteen bytes of an encoder's output need
nothing. Four blocks now go through one test with one mask when the
previous block was ascii too. The gain is smaller than the shape suggests:
large.json's strings carry enough wide characters that most sixty-four-byte
windows hold one, and the pass falls back to the block loop for them.

**A slice stepped a character at a time between its positions.** text/slice
of multibyte text walked from the front to each of the two character
positions it needs, one k_cp_len step per character: the index phase's
subject slice walked 690,000 characters that way, 14 instructions each.
The bytes between the positions are counted a word at a time now, by the
continuation bytes in each eight, and the walk stops one word short of
either position so the character loop still lands on it. A character cut
by a word's end is given back to the loop.

    runbench     2,483,620,655 ->  2,466,456,226   -17,164,429   -0.6911%
    indexbench       3,732,366 ->      3,255,310      -477,056   -12.7816%
    encodebench  4,072,255,532 ->  4,070,190,344    -2,065,188   -0.0507%
    livebench    3,609,374,812 ->  3,607,309,610    -2,065,202   -0.0572%
    oneshot         21,841,738 ->     21,836,587        -5,151   -0.0236%

against CI's rows for kanso#1296; the other nine rows move by under a
hundred, the layout. `work_deepbench` lands on 647,639,361, +1,568 -- it
slices nothing and indexes nothing, so that is the runtime's bytes shifting
under it -- and `work_digestbench` on 10,775,541, +79, `work_basket` on
34,124,089, +12, `work_escapebench` on 85,495,206, +12.

on the container with clang 19, the same bytes out. `all_counters.sh`
regenerated the run vein and the one .mem fixture for the allocations; no
other counter moves. Welfare 64.38 -> 65.80, the run peak's fall paying most
of it, held with `--set`.
Ratchet rows `wide_char_cache`, `ascii_four_blocks` and `slice_count_words`
each put one of the three back and ask the work vein. The utf-8 harness
passes 45,189,025 cases with 0 mismatches against the reference.

CI's sitting at ded9a059 agrees with the container's to within twelve
instructions on every row but two: `work_runbench` lands on 2,466,455,728,
-0.6911% against CI's own row for kanso#1296, and `work_deepbench` on
647,637,793, the layout again. The seven rows that land two instructions
over the container's reading are `work_jsonbench` on 1,497,268,440,
`work_widebench` on 50,448,588, `work_pendbench` on 604,694,571,
`work_indexbench` on 3,255,312, `work_scanbench` on 736,173,218,
`work_readbench` on 4,287,855 and `work_livebench` on 3,607,309,612, where
glibc differs. The cache's lookup, the outlined refusal and the cursor's
new arm are a kilobyte of machine code a program that indexes text and
272 bytes for one that does not: `text` lands on 1,459,900, +10,224 summed
over the fourteen; `compile_instructions` falls to 19,316,149. The floor is
re-set on CI's rows.

## 2026-09-07 — THE ALLOCATION COUNTER'S GATE IS ONE BRANCH

**DONE.** k_alloc inlines into every hot caller, and the test in front of
its two counters was written as `!= 0` around a second `if (k_stats_on)`,
a shape left from the days the switch initialised itself lazily and held
-1 until the first allocation. Clang compiled the pair to a compare and
two branches -- one for the positive switch, one for the negative it has
not held since the constructor began setting it before main -- at every
inlined allocation: 7.3 million a run on the run program, 4.7 million on
jsonbench. The gate makes the same `> 0` test every other counting site
makes now, one compare and one branch, and the counters it guards count
exactly as before, since both shapes fire on 1 and not on 0.

    runbench     2,466,455,728 ->  2,453,160,233   -13,295,495   -0.5391%
    jsonbench    1,497,268,440 ->  1,484,987,475   -12,280,965   -0.8202%
    encodebench  4,070,190,332 ->  4,058,910,167   -11,280,165   -0.2771%
    livebench    3,607,309,612 ->  3,596,079,568   -11,230,044   -0.3113%
    pendbench      604,694,571 ->    598,281,250    -6,413,321   -1.0606%
    scanbench      736,173,218 ->    729,801,588    -6,371,630   -0.8655%
    basket          34,124,077 ->     33,931,564      -192,513   -0.5642%
    deepbench      647,637,793 ->    647,494,319      -143,474   -0.0222%
    oneshot         21,836,575 ->     21,743,437       -93,138   -0.4265%
    widebench       50,448,588 ->     50,368,545       -80,043   -0.1587%
    digestbench     10,775,462 ->     10,721,075       -54,387   -0.5047%
    readbench        4,287,855 ->      4,288,040          +185   +0.0043%
    indexbench       3,255,312 ->      3,260,259        +4,947   +0.1520%
    escapebench     85,495,194 ->     85,558,220       +63,026   +0.0737%

against CI's rows for kanso#1297. Three rows rise: `work_escapebench`
lands on 85,558,220, +63,026, `work_indexbench` on 3,260,259, +4,947,
and `work_readbench` on 4,288,040, +185. Those three allocate little
where they spend, and what moved for them is the inlining below rather
than the gate: k_buf's body landing inside k_b_push_grow and its
neighbours costs escapebench's push path a few instructions a call.

on the container with clang 19, the same bytes out, and every counter
byte-identical: the counted run takes the same branch it always did.
`all_counters.sh` agrees with every golden. Welfare 65.80 -> 65.84,
held with `--set`. The run program's fall is
1.8 instructions an allocation rather than one, and the profile says why:
with k_alloc a branch smaller, clang inlined k_buf and k_list_lit into
their callers -- 34,157,401 and 9,379,099 instructions of self cost gone
from the two names, their callers absorbing less than that -- and
k_render_at fell 2,317,164, k_b_utf8_slice_raw 923,571, k_rec 800,350. Ratchet row `alloc_gate` puts the two-branch
gate back and asks the work vein; dry-run against the tree before it was
committed.

**CI's sitting.** The runner's rows differ from the container's by the
usual few instructions, and the goldens carry the runner's: `work_runbench`
2,453,159,735, `work_jsonbench` 1,484,987,477, `work_encodebench`
4,058,910,155, `work_pendbench` 598,281,252, `work_deepbench` 647,492,751.
The three rising rows land on `work_escapebench` 85,558,208 (+63,014),
`work_indexbench` 3,260,261 (+4,949) and `work_readbench` 4,288,042
(+187), priced above. `compile_instructions` lands on 19,316,808, +659
on 19,316,149: a two-line change to a function the runtime inlines
everywhere, and the compiler carries the runtime as a string, so the
row moves with the bytes of that string and nothing else. Every `.text`
row falls with k_alloc's second branch gone from every inlined site:
`text` 1,459,900 -> 1,449,644, runbench 241,810 -> 241,682, indexbench
55,378 -> 53,954 the largest single fall at -1,424. Welfare on CI's rows
65.84, held with `--set`.

## 2026-09-07 — THE SIZING WALK CALLED ON EVERY IMMEDIATE

**Search.** `k_copy_size`, `k_worth_sizing`, `sizing walk`, `k_ptrmap_at`
in the log, the archive and design/*.md. The deepbench entry that added
`k_worth_sizing` to the list arm is the nearest: it named the list arm and
stopped there. Nothing on the closure, description or subtype arms, and
nothing on the seen-map's call.

**Where it was.** The bind chain sizes its continuation on every step, so
the step can choose between leaving it, staging it through the carry pair
and flooring the region under it. On the run program that is 52,368 walks
a run, 60,216,283 instructions inclusive, 2.45% of the whole, for a
continuation of six or seven nodes each. The walk's self cost read 93
instructions a node, and the instruction-level profile put the first
fourteen of them in front of a test that sent the caller straight back:
the closure, description and subtype arms recursed on every slot they
held, and a capture or a description's slot is an int or a string as
often as a pointer. 510,001 slots visited, 176,113 of them immediates,
each paying the frame's seven pushes and seven pops to learn it was not
heap. The list arm has asked `k_worth_sizing` before the call since
deepbench; the record arm too; the other three had never been given the
question.

The second cost was the seen-map. `k_copy_seen_check` asks `k_ptrmap_at`
once a node, 346,000 times a run, and the probe is a multiply, a mask
and a compare behind a call: 11,449,834 instructions in the callee's own
name, 33 an ask.

**What changed.** The closure, description and subtype arms ask
`k_worth_sizing` before recursing, the test the list and record arms
already make. `k_ptrmap_probe` and `k_ptrmap_at` are `always_inline`, at
all six of their sites: the seen-map, the interior-survives memo and the
copy map's four.

    runbench   2,453,160,233 -> 2,446,395,268   -6,764,965   -0.2758%

on the container with clang 19, the same bytes out. `k_copy_size`'s
recursion 338,862 calls -> 216,337; `k_ptrmap_at` 11,449,834 -> 0, its
body landing in `k_copy_size` (+4,411,000 self across the two names) and
`k_deep_copy` (+99,380). The chain step's walk 60,216,283 -> 54,045,408
inclusive. No counter moves: the walk answers the same sizes, since an
immediate always sized at zero, and `all_counters.sh` agrees with every
golden.

The other rows move with the seen-map's call, and two of them move
more than the run program does, because the walk is a larger share of
what they do:

    deepbench    647,494,319 ->   599,236,632  -48,257,687   -7.4530%
    widebench     50,368,545 ->    49,141,296   -1,227,249   -2.4365%
    pendbench    598,281,250 ->   598,210,956      -70,294   -0.0117%
    basket        33,931,564 ->    33,935,458       +3,894   +0.0115%

deepbench folds over lists of ints under a beat, and its sizing walk
asks the seen-map 2.35 million times a run: `k_ptrmap_at` 77,636,945 ->
0 there, `k_copy_size` +27,665,576 absorbing the probe. The walk is
still 230,739,478 instructions of deepbench's 599 million, and that is
the next thing to read. `work_basket` lands on 33,935,458, +3,894, the
inlined probe's bytes at the copy map's four sites on a program whose
walks are few. Every other row moves under 0.05%.

**What is left in the walk.** 216,337 nodes at about 140 instructions
inclusive each: the frame, the budget and carry tests, the seen-map's
probe, `k_survives_x` at 36 a call, and the list arm's sixteen-instruction
slot loop over 498,400 items of which 39,731 were worth the call. The
survives ask is the largest of those and is already one walk of the
chain since kanso#1296. The walk is a cost the chain step pays for the
choice it makes; making the choice without the walk is a different
design and is not this entry's.

Ratchet row `size_walk` puts the unconditional calls and the out-of-line
probe back and asks the work vein, the only witness a walk that answers
the same sizes leaves. Dry-run against the tree before it was committed.

**CI's sitting.** The runner's rows differ from the container's by a few
instructions and the goldens carry the runner's: `work_runbench`
2,446,394,810, `work_deepbench` 599,235,221, `work_widebench`
49,141,298, and the one rising row `work_basket` lands on 33,935,460,
+3,894 on main's 33,931,566, priced above. `compile_instructions` falls
851 to 19,315,957, the runtime string's bytes moving under the compiler.
The `.text` vein rises for the first time in this run of entries: `text`
1,449,644 -> 1,463,756, +14,112, and it is 1,008 bytes on every one of the
fourteen binaries, jsonbench 93,010 -> 94,018 to runbench 241,682 ->
242,690. That is the probe's body at its six sites, five of them copies
the walk's own inlining did not need; the 2026-09-05 ruling keeps machine
code out of welfare and in its own exact vein, so the kilobyte is
recorded here and weighed nowhere. Welfare on CI's rows 65.86, held.

## 2026-09-07 — A CHAIN STEP THAT SIZES ONLY WHEN THE REGION HAS DRIFTED, DECLINED FOR NOW

**Search.** `chain step`, `k_arena_at_carry`, `drift`, `leave`, `retired two
carries later` in the log, the archive and design/*.md. The entry that
introduced the leave branch added the drift test beside a size test and
called both load-bearing, the size test "excluding the shape that carries
a large value forward, which must keep being evacuated or the size walk
re-reads it every step". Nothing since has asked what the leave would cost
without the size test in front of it.

**The measurement.** The sizing walk that the entry above trimmed still
runs on every chain step, and on deepbench it is 381,213,741 instructions
inclusive, 63.6% of the program, to learn what the previous step's walk
learnt. Asking the drift test first, and leaving without a walk while the
region has not drifted a quarter megabyte past the last staged top:

    deepbench    599,236,632 ->   211,925,873  -387,310,759  -64.6342%
    widebench     49,141,296 ->    36,468,436   -12,672,860  -25.7887%
    runbench   2,446,395,268 -> 2,393,553,867   -52,841,401   -2.1600%
    pendbench    598,210,956 ->   596,853,570    -1,357,386   -0.2269%

on the container with clang 19, the same bytes out on all fourteen
benchmarks, every allocation, peak and evacuation counter in the twelve
veins byte-identical, and only the walk's own `survive_slots` moving. It
is the largest single move the run program has had, and it does not ship
today, for two reasons that are the same reason.

**Why not.** The carry pair retires a buffer two stages after it was
filled, and the tenure tier promotes a value the walk finds inside the
previous stage's buffer, "lived a lap". Both were written when a stage was
a step, and both assume it. Under the drift policy the fixture
`an_inner_beat_opens_its_tenure_in_the_block_outside`, which exists to pin
the inner-beat block carve of kanso#1294 by reading `ten_blocks=3`, reads
`ten_blocks=0`: its inner chain of 400 steps drifts eighty kilobytes a lap
and stages once, so nothing it holds lives a lap and nothing is promoted.
Lengthening its laps to 3,000 and 6,000 elements gave 28 stages and still
no promotion. And a second variant, which measured the drift from the
rewound top instead of the pre-stage top so that a stage is never followed
by a second one a step later, segfaulted in `k_deep_copy` under
`k_repair_interior` eight frames down `k_beat_iter_carry` while running
`scripts/trend_gate`: a repaired node's interior read out of a buffer the
stage had begun to reuse. The variant that measured from the pre-stage
top ran the same program clean, and the difference between them is that
the first accidentally stages twice at the start of every chain, which
promotes what the chain holds into tenure before the buffers turn over.
That is luck, not a design, and the crash is the shape the tenure
fixture's comment warned about.

**What would make it ship.** The prize is real and the policy is right in
outline; the retirement rule under it is not. Either a stage that follows
a drift copies out of both carry buffers before it reuses either, or
whatever a repaired node holds in a carry buffer is promoted at the stage
that repairs it rather than the one after. Both are changes to the carry,
priced by the .mem vein and the run program's peak, and the inner-beat
fixture must reach tenure again under whichever ships, with its numbers
rewritten. Recorded here so the measurement is not made twice; the
variant runtimes and the mutation for the row are in the session's
scratch, and the entry that ships it will carry them.

## 2026-09-07 — A CHAIN STEP SIZED ITS CONTINUATION EVERY STEP, AND THE POP THAT RETIRED THE CARRY PRUNED

**Search.** `chain step`, `k_arena_at_carry`, `drift`, `leave`, `nsz`,
`prune`, `k_interior_survives` in the log, the archive and design/*.md. The
nearest entry is yesterday's, which measured this policy and declined it:
"the carry pair's two-stage retirement and the tenure tier's lived-a-lap
promotion both assume a stage is a step", with a segfault it could not
explain. This entry explains it and ships the policy.

**Where it was.** The bind chain sizes its continuation on every step to
choose between flooring the region under a large value, leaving a small one
where it is while the region has not drifted, and staging everything else
through the carry pair. The walk ran before the choice, every step.
deepbench's continuation is a closure over a list of eight ints, and its
384,833 steps each walked it at about a thousand instructions to learn what
the previous step's walk had learnt.

**What changed, and what it exposed.** The drift test is asked first, so a
step whose region has not drifted a quarter megabyte past the last staged
top leaves without walking. That alone segfaulted the trend gate, and the
cause is a rule that was true only because a stage happened every step: the
copy-out at a carried pop PRUNES at any survivor whose immediate interior
survives. One level down is all `k_interior_survives` can see, so an arena
node two levels down holding a pointer into the depth's carry buffer was
never repaired at the pop — it was repaired by the caller's next stage,
which used to be the next step. Let the chain leave for a few hundred steps
and the buffer is grown, freed and reused underneath that pointer first.

Two narrower repairs were built and both are wrong. Forcing a stage after
any inner pop that copied out of a carry fixes the crash and costs
deepbench 893,347,801 against 599,236,632, because it forces a stage
everywhere the baseline could leave. Holding the grown buffer to the pop
rather than freeing it still segfaults, because the from/to swap overwrites
a buffer as well as freeing it. What ships is the walk: at that one call
site the copy-out does not prune, so it descends and repairs a carry
pointer at any depth.

    deepbench    599,236,632 ->   410,388,147  -188,848,485  -31.5152%
    widebench     49,141,296 ->    36,460,868   -12,680,428  -25.8039%
    runbench   2,446,395,268 -> 2,418,520,807   -27,874,461   -1.1394%

on the container with clang 19, the same bytes out on all three. The
evacuation counters RISE, which is the deep walk paying for itself: the run
program's `evac_allocs` 15,994 -> 74,551 and `evac_bytes` 8,587,152 ->
10,578,112, pendbench's 2,673 -> 3,201 and 498,320 -> 723,248, and
basket's 3 -> 5. Against them widebench's arena peak falls 3,145,728 ->
2,097,152 and its `arena_blocks` 3 -> 2, and the run program's
`perm_live_bytes` 43,584 -> 0 with `perm_peak_bytes` 53,856 -> 10,272.
Welfare weighs the trade and reads 65.86 -> 65.95, held with `--set`.

**Both tenure fixtures had to be rewritten to keep seeing anything.** A
chain of small steps with no garbage never drifts, so it never stages,
never carries and never promotes — and promotion is what
`a_repaired_node_below_the_mark_holds_tenure` and
`an_inner_beat_opens_its_tenure_in_the_block_outside` exist to pin. Under
the new policy both read `ten_blocks=0`, which is a fixture proving
nothing. Each now allocates a list per step that it discards, demanded by
arithmetic that leaves the index unchanged, and they are back to
`ten_blocks=1` and `ten_blocks=3` with the same bytes out. A fixture that
goes blind is repaired in the change that blinded it.

Ratchet rows `chain_drift` (every step sizes again) and `pop_deep` (the
pop prunes again); both dry-run against the tree before they were
committed, one line each. Welfare 65.86 -> 65.95, held.

**CI's sitting, and every key it moved.** The runner's rows are in the
goldens now and the floor is held on them. The three veins that disagreed
with the container are work, machine code and compile instructions; every
counter vein agreed, which is what says the container and the runner walk
the same shapes.

    deepbench    599,235,221 ->   410,388,149  -188,847,072  -31.5147%
    widebench     49,141,298 ->    36,460,870   -12,680,428  -25.8039%
    runbench   2,446,394,810 -> 2,418,520,678   -27,874,132  -1.1394%

The deep copy-out is a real cost and these are its keys. The run program: run_alloc_bytes 493,143,117, run_allocs 7,316,097, run_beat_iters 2,693,239, run_buf_reuse 56,614, run_evac_allocs 74,551, run_evac_bytes 10,578,112, run_sh_buf 123,904,112, run_ten_blocks 7.

pendbench, whose chain now leaves where it used to stage: pend_alloc_bytes 257,195,216, pend_allocs 4,007,549, pend_beat_iters 200, pend_buf_reuse 118, pend_evac_allocs 3,201, pend_evac_bytes 723,248, pend_sh_buf 35,309,968, pend_survive_slots 157,611. Its arena peak falls to 2,097,152 with arena_blocks 2 against the extra copying.

basket: basket_alloc_bytes 4,955,665, basket_allocs 28,171, basket_evac_allocs 5, basket_evac_bytes 55,104, basket_str_scan_bytes 113,467, basket_str_scans 2,011.

The work vein's twelve small risers, each a few instructions of the
reordered chain step: work_basket 34,010,143, work_digestbench 10,719,185, work_encodebench 4,058,888,975, work_escapebench 85,558,105, work_indexbench 3,258,722, work_jsonbench 1,484,986,567, work_livebench 3,596,058,560, work_oneshot 21,742,515, work_pendbench 602,145,183, work_readbench 4,287,134, work_scanbench 729,800,419.

The compile row and the machine-code vein carry the new branch and the
wider KCopy: compile_instructions 19,317,662, text 1,465,084.

The lazy tier moves in two ways. Four fixtures were not edited and their
counters moved from the policy alone; two were rewritten to keep reaching
tenure and their counters are a different program's. Both are re-based
here: a_builder_handed_on_is_still_a_builder_alloc_bytes 294, a_builder_handed_on_is_still_a_builder_allocs 5, a_builder_handed_on_is_still_a_builder_evac_allocs 4, a_builder_handed_on_is_still_a_builder_evac_bytes 256, a_loop_invariant_capture_is_copied_every_rewind_ten_frees 0, a_repaired_node_below_the_mark_holds_tenure_alloc_bytes 7,078,880, a_repaired_node_below_the_mark_holds_tenure_allocs 100,914, a_repaired_node_below_the_mark_holds_tenure_push_mut_slow 1,604, a_repaired_node_below_the_mark_holds_tenure_sh_rec 4,025,872, a_repaired_node_below_the_mark_holds_tenure_sh_str 1,721,504, an_inner_beat_opens_its_tenure_in_the_block_outside_allocs 514,502, an_inner_beat_opens_its_tenure_in_the_block_outside_push_mut_slow 6,020, an_inner_beat_opens_its_tenure_in_the_block_outside_sh_rec 20,129,168, an_inner_beat_opens_its_tenure_in_the_block_outside_sh_str 8,606,752, builder_counts_once_alloc_bytes 28,606, builder_counts_once_allocs 12, builder_counts_once_evac_allocs 4, builder_counts_once_evac_bytes 6,096, builder_counts_once_str_scan_bytes 6,015, builder_counts_once_str_scans 3, effect_push_shape_alloc_bytes 3,264, effect_push_shape_allocs 83, effect_push_shape_beat_iters 5, effect_push_shape_evac_allocs 50, effect_push_shape_evac_bytes 1,696, string_builder_shape_alloc_bytes 12,295, string_builder_shape_allocs 12, string_builder_shape_evac_allocs 4, string_builder_shape_evac_bytes 4,096, string_builder_shape_str_scan_bytes 4,001, string_builder_shape_str_scans 3.

---

## 2026-09-07 — the pop's copy-out prunes again, and the write sites say when it may not

**DONE.** kanso#1300 made the copy-out at a carried pop walk deep at every
pop, because a carry pointer could sit two levels down where
`k_interior_survives` cannot see it. That cost 4.97% of deepbench and bought
correctness on one program. This entry buys the 4.97% back without giving up
the correctness.

The route such a pointer takes is single. The copy machinery decides what to
share and what to copy; nothing it decided to share can acquire a new pointer
except through a write that goes around it, and there are exactly eight of
those — `k_set_field`, `k_map_replace`, `k_b_put_mut`, `k_b_put`,
`k_b_entries`, `k_b_push_into_proven`, `k_b_push_mut`, `k_b_join`. Seven are
instrumented; `k_b_entries` is not, because the record it writes into is
freshly allocated and therefore above every mark.

So `k_carry_written` latches the first time one of the seven stores a pointer
that lands in a live carry buffer, and the copy-out reads that latch as its
`deep` flag. The latch never clears, so the walk is only ever turned ON later
than #1300 turned it on — the change cannot make a program that was correct
incorrect.

Asking has to be nearly free, and getting there took three shapes:

- A bounding box over every carry buffer ever malloc'd, plus the exact walk
  for what it admits. **pendbench +2.43%.** The box grows to span whatever the
  allocator handed out between two distant buffers, so nearly every write asks
  the exact question.
- The same box over the LIVE buffers only, recomputed at the two places the
  set changes (the sizing malloc, the cohort free). **pendbench unchanged.**
  The box was not the cost.
- The profile said where it was: `k_b_join` +3,617,214 and `k_b_push`
  +579,391 on runbench. `join` forces every element and asked in front of the
  thunk test, so it asked once per element on a path that mostly writes back
  the bits already there. Behind the test it asks only when a force actually
  replaced a thunk. The whole ask is now one unsigned compare
  (`(uintptr_t)payload - base < span`), and a span of zero means both "no
  buffer is live" and "already latched".

**Container sitting, interleaved, both binaries run from the repo root:**

| row | main | this | delta |
|---|---|---|---|
| deepbench | 413,732,754 | 395,091,117 | **−4.5054%** |
| runbench | 2,400,141,224 | 2,398,016,752 | **−0.0885%** |
| pendbench | 588,668,860 | 589,537,053 | +0.1475% |
| jsonbench | 1,452,183,939 | 1,452,700,824 | +0.0356% |
| widebench | 37,430,544 | 37,432,922 | +0.0064% |

Separating the two halves needed a third binary with the machinery present and
`deep` pinned to 1: asking cost deepbench 0.62%, runbench 0.18%, pendbench
2.44%; pruning bought deepbench 4.97% and runbench 2,734,052. Those are the
numbers the shapes above were chosen against.

**CI's sitting, which is the one that counts.** The container A/B above was
right about the direction and shy about the size. On CI, against main:
`work_deepbench` 410,388,149 -> **389,214,232** (−5.1595%),
`work_runbench` 2,418,520,678 -> **2,414,841,737** (−0.1521%) and
`work_pendbench` 602,145,183 -> **598,215,444** (−0.6526%). runbench is the
objective's whole run-speed term and it falls further on CI than in the
container; pendbench, which the container read as a 0.15% riser, falls here.
Welfare 65.95 -> **65.96**, held with `--set` in this PR.

Nine work rows rise and one of them is not small. `work_basket` 34,010,143 ->
**34,698,417** (+2.02%) is the prune's own cost showing up where the prune
helps most: basket's `evac_bytes` fell 55,104 -> 192, so the walk that used to
copy those bytes now asks `k_slots_survive` about them instead, and asking is
what the row counts. The other eight are the ask on paths that never latch --
`work_encodebench` 4,058,895,905 (+0.0002%), `work_livebench` 3,596,062,732
(+0.0001%), `work_scanbench` 729,804,590 (+0.0006%), `work_widebench`
36,463,282 (+0.0066%), `work_jsonbench` 1,485,161,449 (+0.0118%),
`work_oneshot` 21,745,451 (+0.0135%), `work_indexbench` 3,265,868 (+0.2193%)
and `work_digestbench` 10,745,219 (+0.2429%). `text` 1,465,084 -> **1,474,076**
is the seven inlined asks and the two new runtime functions; every one of the
fourteen binaries grew, none by more than a kilobyte.

**Counters.** The prune shares where it used to copy, so the evacuation
counters fall: basket `evac_bytes` 55,104 → 192 and `evac_allocs` 5 → 3,
`string_builder_shape` 4,096 → 80, `builder_counts_once` 6,096 → 80, allocs
falling with them. `survive_slots` RISES in all seven places that count it, because
`k_slots_survive` is what the prune asks and a walk that does not prune never
asks it: `run_survive_slots` 61,752 -> 159,393, `pend_survive_slots` 157,611
-> 157,811, `encode_survive_slots` 129,871 -> 129,873, `live_survive_slots`
129,871 -> 129,873, `basket_survive_slots` 16,000 -> 16,002,
`record_reuse_shape_survive_slots` 16,004 -> 16,006 and
`effect_push_shape_survive_slots` 4 -> 15. Both directions are the same change and the runbench row falls
anyway.

**The spec.** `tests/golden/mem/a_carried_value_written_into_an_older_node`
is the shape reduced to a page: `xs` is copied into the carry every lap, and
each step pushes one of its elements into storage made before the bind, so a
node the walk would share holds a pointer into the buffer the pop retires. It
latches at `k_b_push`'s frontier write, which is where `scripts/trend_gate`
latches too. Watched red first: under the mutation the fixture reads
`survive_slots=807` and `carry_dedup=17` against `407` and `416`.

**A CORRECTION, made before this landed.** The commit that opened kanso#1301
said the mutation segfaults `scripts/trend_gate`, copying kanso#1300's record.
It does not, on this tree. The gate's workload is the golden diff against a
base, and with the mutation applied and five bases tried -- HEAD~1, e751c948,
82310c5d, a8d5296b, 38865021 -- none faulted. #1300 saw the fault while the
shape was being built and its record stands for that tree; what is here is the
mechanism and the counters, not a program that faults on demand. The guard
stays: that a hazard is unreachable today is not that it is unreachable, and
#1300 is the reason to believe it is not.

**OPEN — nothing in the tree faults without the walk**, and the reason the
four reduced shapes could not is worth writing down, because it narrows what a
faulting program has to look like. `k_interior_survives` does not stop at the
node: for a list it asks `k_survives_x(l->items, m)` AND
`k_slots_survive(l->items, l->len, m)`. So a node that holds a carry pointer
in one of its own slots is never pruned at -- that slot does not survive, and
the walk descends and repairs it. The hazard needs the pointer TWO levels
down, behind a survivor whose every immediate slot survives, which is the
sentence #1300's fix was written from and which I had been reading as "one
level" when writing fixtures.

Getting there from kanso source is the hard part. An in-place push returns a
new header and leaves the old one's length behind, so a write two levels down
is invisible to a reader holding the outer node unless the emitter proved
uniqueness and took the `push_mut` path that moves `len` in place. The four
shapes each failed one of those two conditions: `t1` and the trend-gate shape
put the pointer in the outer node's own slot, and `t2` and `t3` put a freshly
allocated node in between, which sits above the mark and so does not survive
either. A fifth shape was written and run: a pre-existing inner list reached through a
pre-existing outer one, `push outer[1]! x` each lap with `outer` threaded
unchanged. It reads back `held 0` -- the outer node never sees the growth, so
the emitter is not taking the `push_mut` path there and the written slot is
not reachable from the outer node at all. Both conditions have to hold at
once and no shape yet holds both, which is a fair reason to suspect the
hazard needs the emitter to prove a uniqueness it does not prove here. That
is settled by `src/linear.rs` without another sitting: a push is marked in
place only when its list argument traces back to a fresh `[]` through a chain
in which every step is moved and never aliased, and an index expression is not
one of those. So the write that would put a carry pointer two levels down
cannot be an in-place write on a node reached by an index.

That is not a proof the hazard is unreachable. `k_set_field`, `k_map_replace`
and `k_b_put_mut` write into nodes reached other ways, and the argument covers
`push` only. It is a reason the corpus is quiet and a reason the other six
sites are where to look. The latch watches all seven either way, which is the
point of latching rather than reasoning.

## 2026-09-07 — a comparison the emitter could not prove is not a thunk

**The defect.** `emit_binop_builtin` ends by merging two arms into a phi: the
inlined fast path, and the call the slow path makes. For `+`, `-` and `*` that
call is `k_add`, `k_sub` or `k_mul`; for the six comparisons it is `k_cmp`.
The phi was returned with no `f.record` on it, and `set_of` answers
`unwrap_or(TOP)` for a name it has never seen. TOP carries THUNK, so
`maybe_force` emitted a `k_force_fast` in front of every `if` whose condition
was one of those phis — a force on a value that is a boolean by construction.

Both arms are known. `k_cmp` answers `k_bool` or the failure it was handed.
`k_add` and its two siblings answer an int, a float or that same failure, and
`k_die` on anything else. The fast arms are the `select` between the two
constant tags and the `insertvalue` of an int. So the phi's set is
`BOOL | ERR` for a comparison and `INT | FLOAT | ERR` for the three
arithmetic ops, with the operands' failure bits carried through the way the
`/` and `%` line above it already carries them. Recording that is the whole
change.

**What it cost.** runbench 2,414,841,906 -> 2,400,271,241, −0.6034%, same
output checksum. `k_force_fast` in runbench's emitted code 557 calls -> 451;
runbench's lines 34,979 -> 34,873, pendbench −15 calls, digestbench −25,
jsonbench byte-identical because its comparisons were already proved pure-int.
`kanso check lib/json` moves 6 instructions on this container, and
`compile_allocs`, `compile_peak_bytes`, `compile_rounds` and `compile_visits`
are byte-identical. Projected welfare 65.96 -> 66.00, held with `--set`.

**Where it was found.** `value_for` is 258,970,635 instructions, 10.72% of
runbench, over 1,791,306 calls — 144.6 a call, spread across 522 addresses
with the top fifty accounting for half. It is not the arm ladder. Forty-one
per cent of it is the number scan: `scan` and `scan_at` inline into it, and
the per-byte loop runs 3.48 million times for 417,483 numbers at about 32
instructions a byte. Two of those instructions are `cmp $0xe` and its branch,
which is the inlined thunk test — tag 14 is `K_THUNK`, not a failure tag, and
the operand is a `select` of 2 and 3.

**Three shapes measured and not shipped.**

The overflow checks, priced as a ceiling and never a candidate: replacing
every `llvm.sadd/ssub/smul.with.overflow.i64` and its `k_die` trap block with
a plain `nsw` op reads runbench 2,414,841,906 -> 2,382,274,700, −1.3486%.
That is the entire checked-arithmetic budget of the run program, and the
overflow death is defined, message-pinned behaviour, so the number is a
measurement rather than a proposal.

Ten digit arms on `scan_at`, one for each of 48 through 57. Five sparse cases
over a 59-slot span lower to a compare ladder, where fifteen lower to a jump
table: runbench 2,414,841,906 -> 2,380,504,534, −1.4219%, `value_for`
258,970,635 -> 224,633,376, same checksum — more than the whole ceiling above.
**The objective declines it: welfare 65.96 -> 65.86.** Priced one term at a
time against the floor, runtime pays +0.10 and the three compile terms take
0.21 back: `compile_instructions` −0.09 (20,135,321 -> 21,018,953 here),
`compile_allocs` −0.05 (11,613 -> 12,029), `compile_peak_bytes` −0.07
(375,222 -> 388,852). Front-end visits 9,884 -> 10,308 and emitted lines +96.
This is the pending question in design/pending-gavels.md with a number on it:
the compile term's workload is lib/json itself, so thirty lines added to the
library are measured as a 4.4% compile regression on the very lines they add,
while a program that imports lib/json compiles them once and decodes numbers
forever. Recorded here rather than argued; the weights are Clay's.

The same reordering written to cost less library text — the digit range asked
in `scan` before the dispatch, with `scan_at` keeping the five rare bytes —
reads −0.2914%, a fifth of the arms. The extra hop costs what the table saves.

**Two earlier guesses at `value_for`, both wrong.** Whitespace written as arms
measured +0.1216%. And the emitter does write a real `switch i64` for
integer-literal arms; the "compare chain" the first reading assumed is LLVM's
lowering of a sparse switch, not the emitter's shape.

**CI's rows.** runbench 2,414,841,737 -> 2,400,271,058 (-0.6034%, agreeing
with the container's reading to four figures), pendbench 598,215,444 ->
596,612,948 (-0.2679%), digestbench 10,745,219 -> 10,420,391 (-3.0230%). The
`.text` vein falls with them: runbench 243,858 -> 241,714, pendbench 86,914 ->
86,610, digestbench 106,562 -> 105,634. `compile_instructions` is 19,317,662,
unchanged. Every other row in both veins holds. Floor 65.96 -> 66.00.

**A second instance of the same defect, found and not shipped.** A float
literal is emitted as a call to `k_float` and its result carried no set
either, so it read as TOP the same way -- and `k_float` answers a float and
nothing else. Recording it changes runbench by zero, and changes no
emitted-code row of any of the fourteen programs. Nothing in the corpus can
see it, so nothing pins it, and a change no golden can fail is not one to
carry. It is written down here so the next float-heavy program in the corpus
finds it already diagnosed. The other unrecorded `%KValue` in the sweep is
`k_env_get`, and TOP is right there: a captured name really can be a thunk.

**And the instruction file was wrong about the objective, which is how the
digit arms were mispriced by hand before `welfare` was run.** CLAUDE.md said
the score weighs twenty-eight counters — an instruction row per benchmark and
twelve memory rows — which was the shape before the 2026-09-06 gavel cut the
runtime side to one consolidated program. It weighs five:
`run_instructions`, `run_peak_bytes`, and the three compile rows.
`bench/objective_sources.txt` had the same count in its header while its own
later note recorded the change. Arithmetic over the old model gave the digit
arms a net rise of about +0.04; the real run gave −0.10, because the three
compile terms carry a full point of weight between them against run speed's
two, and all three moved the same way. Both files corrected, and the paragraph
in CLAUDE.md now says to run `welfare --counters` rather than trust the
sentence — it has been wrong twice in two days.

---

## 2026-09-07 — a boolean is not a failure, and the wider version of that is slower

**DONE.** `inline_not_failure` folds to `true` when the emitter has proved the
value is a boolean. That is what an `if` over a comparison hands it, and it is
27 of runbench's 1,223 tag tests. The comparison phi's recorded set drops the
ERR it was carrying unconditionally since kanso#1302, which makes 2 of those
27 available: `k_cmp` returns a failure only when handed one, and every other
arm below its failure test answers `k_bool`. `k_add`, `k_sub` and `k_mul`
are the same shape, so the arithmetic arm drops it too; `k_div` and `k_mod`
really do mint one, for division by zero, and their arm keeps it.

**An empty set is not a proof, and the first draft read it as one.**
`group_param_set` answers 0 for a parameter the inference reached no shapes
for, and 0 satisfies every `& mask == 0` test written about it — so
`set & !BOOL == 0` held for a value nothing was known about. The dispatcher's
propagate block, which hops the failing argument out of a call, then read
`true` for every argument and fell through to `no overload of X matches these
arguments`. `a_construction_merges_its_failures` lost its third line on native
while the interpreter kept it, and the micro corpus caught it. The fold now
requires a non-empty set, and the propagate block asks with a twin that never
folds — that block is reached BECAUSE a value is outside the set recorded
for it, so a set cannot speak there at all. Both are needed: the first is the
general rule, the second the one site where even a true set is the wrong
question.

On clang 19.1.1 in a container, which reproduces CI's runbench row to 189
instructions (2,400,271,247 against the golden's 2,400,271,058):

    runbench   2,400,271,247 -> 2,398,991,700   -1,279,547   -0.0533%

The number is the same to the instruction with the 24 unsound folds in and
with them out, so none of them sat on a path a benchmark runs — the whole
fall is the 27 that survive the guard.

CI's fourteen work rows, against main. Three fall and four rise, and the four
are named here because the trend gate is right to ask: `work_jsonbench`
lands on **1,487,045,449** (+0.1269%), `work_scanbench` on **730,307,092**
(+0.0689%), `work_oneshot` on **21,758,011** (+0.0578%) and `work_livebench` on
**3,596,075,294** (+0.0003%). Against `work_runbench` 2,398,991,511
(−0.0533%), `work_encodebench` 4,058,633,349 (−0.0065%) and `work_widebench`
36,127,282 (−0.9215%).

The four rises are not attributed per row and this entry does not pretend they
are. What is known: every one of the thirteen emitted rows FALLS, these four
included, so the compiler is writing less code for them and the rise is
downstream of what it wrote — the same reshuffle class the wide version showed
at ten times the size, where re-parenting and inlining moved millions between
functions for a net of thousands. jsonbench rising 0.1269% while runbench falls
0.0533% on a change whose 27 sites include nine in the json decoder is the
sharpest form of that: the same code, the two benchmarks disagreeing on sign.
The objective weighs runbench, and runbench falls.

All twelve cost veins and the lazy tier are byte-identical — removing a tag
test allocates nothing — and every one of the thirteen emitted rows falls.
Welfare floor 66.00009186328253 -> 66.00389069855446.

**REVERTED — the same fold on every ERR-free set.** ERR is outside every
ERR-free set, not only outside BOOL, so `set & ERR == 0` is sound too and
removes 690 of the 1,223. It is slower:

    runbench   2,400,271,247 -> 2,403,954,263   +3,683,016   +0.1534%

and welfare 66.00 -> 65.99, so the objective declines it. That build carried
the empty-set defect too, so its 690 sites are an upper bound on what a sound
version of the same idea removes — and it lost anyway, which is what makes
the decline safe to record. The decode side does
what #384 predicted — `obj_key_start` 138,744,540 -> 133,023,429, −4.12% — and
the total rises anyway. Measured on clang 18.1.3 first (+0.1169%) and the sign
held on 19.

**WHY it rises is not established, and the first answer written here was
wrong.** It said `d_json/word_4` stopped being inlined into `value_for` and
appeared as 9,900,000 on its own. `word_4` is `musttail`-called from three
sites, so it cannot be inlined at any threshold: marking it `noinline` by hand
in the IR and relinking gives a byte-identical binary. Its 0 -> 9,900,000 is
the linker folding it with an identical twin in one build and not the other,
which moves where callgrind files the cost and not what the cost is. The rows
that remain are `encode_onto` and `entry_onto`, and `--separate-callers=2` says
those are real. See the entry below.

**This bounds #384.** That thread put the failure-tag compares at 40,778,277
executions across runbench, 1.50%, and read a cannot-fail analysis as having a
ceiling near 3% once their branches went with them. The ceiling is real and it
is not reachable by folding every one of them: the 690-site version is the
whole of that idea and it loses. What is left of #384 is the 27 sites here and
whatever a per-site rule could find, which is a different and much smaller
piece of work than the thread described.

**Two ratchet rows, not one.** `a_comparison_that_might_be_a_failure` takes
the fold's mask from BOOL to nothing; `a_comparison_that_might_be_a_thunk`
already sends the phi's whole set to TOP, which takes the fold out and puts
`k_force_fast` back on top of that. runbench's `calls` over the two halves:
6026 with neither, 6026 with the tighter phi alone, 6001 with the fold alone,
5999 with both. The fold is 25 of the 27 and the phi is the other 2; nothing
else in the emitter reads that bit, so the tighter set buys nothing by itself.
An earlier draft of this paragraph read 20 for the second cell — it was
measuring a mutation that left the empty-set folds standing.

**CLOSED — the encoder pays, and it is the recursive pair.**
`--separate-callers=2` on both builds, each measured as `./runbench` from the
repo root so the exec path is the same: shipped 2,398,991,700, wide
2,403,950,554, +4,958,854.

Almost every large row in the flat profile is re-parenting. `d_escape/more_4`
is inlined into `d_runbench/tally_4` in the wide build, so everything under it
moves up one level: `more_4'tally_4'w_klam39` 65,309,063 -> 0 against
`tally_4'w_klam39'k_worded_step` 20,716,576 -> 86,045,501, a net 19,862.
`k_beat_iter` under the two parents nets 2,388, sha256's `compress_4` −2,051,
`k_b_push_grow` exactly 0. `word_4` appears at 9,900,000 across its two caller
chains and the decode functions it re-parents from fall 10,380,150 — a net
480,150 in the change's favour, not the 9.9M rise the flat profile showed.

What survives is four rows and they are all the encoder:

    encode_onto'2 under entry_onto        +3,360,690
    entry_onto'2 under encode_onto        +3,816,180
    entry_onto under encode_onto/tally     +698,760
    d_thunk_eval under k_force_slow        +764,400

8,640,030 of rise, in a mutually recursive pair and in the slow force behind
it. That is where the wide fold's cost is. Removing a tag test from a function
that calls itself through another changes what LLVM can prove across the cycle,
and the last row says some of what it stops proving is that a value is already
forced. The narrow rule does not go near it, and that is checked rather
than assumed: an `eprintln` on the fold, keyed on an environment variable and
run over `kanso build bench/runbench --release`, names all 27 sites. Thirteen
are in regexp, nine in the json decoder (`value_for`, `obj_key_start`,
`skip_ws`, `scan`, `obj_open`, `obj_delim`, `array_open`, `array_delim`,
`value_blank`), two in list, and none in `encode_onto` or `entry_onto`. So the
narrow rule avoids the recursive pair by not reaching it, and whether it would
survive reaching it is still unknown.

The first attempt at this probe hand-linked the two `.ll` files against the
newest cached runtime object and both binaries died with `bytes takes a string`
after 414,247 instructions: `cached_runtime_object` keys on the closure
convention and the newest object on this box was built under the other one, so
a hand-link picks the wrong half and the two halves disagree about registers.
Let `kanso build` do the linking.
