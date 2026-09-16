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

## 2026-09-13 (eleventh) — the in-place byte append tested for a buffer, then tested that the byte fits, and the second test already says the first

`k_b_append_mut_byte`'s byte arm loaded the capacity word, masked off the
storage-regime bit kanso#1397 put in bit 0, and asked `cap != 0` on the RAW
word before it would read the buffer header. Then, past that branch, it asked
`len + 1 <= capa` — and that second question already answers the first. `len`
is never negative, so `len + 1 >= 1`, so the fit can only hold when
`capa >= 1`, so `cap != 0`.

The zero test was not dead code, which is why it could not simply be deleted.
It fenced the header load: the block behind it reads `data[-8]` to check that
the buffer's used-length still matches, and on a buffer with no capacity there
is no header eight bytes back to read. The fix is a reorder — compute the fit
from `len` and the masked capacity FIRST, branch on it, and load the data
pointer and its header only on the arm where the fit held. Every path that
reaches the header load satisfied the old test too, so the guard is strictly
narrower.

**The string arm keeps its zero test, and the reason is worth writing down.**
It looks identical — same mask, same `scap != 0`, same header read behind it —
but its fit is `slen + n <= scapa` where `n` is the appended string's length,
and `n` can be zero. An empty accumulator appending an empty string satisfies
`0 <= 0` with the capacity word at zero, reaches the header load, and reads
eight bytes that are not there. The byte arm is safe only because its `n` is
literally 1. Two arms that look the same and are not.

A/B on this container, callgrind both sides, re-measured after kanso#1400
landed and the branch was re-cut onto it: runbench 2,010,049,371 ->
2,003,781,037, a fall of **6,268,334 (−0.3119%)**. Against the pre-#1400 base
the same change measured −6,268,320, fourteen instructions apart — the same
change at the same size on either side of a merge. The twelve cost veins and
the lazy tier all AGREE.

It is the count, not the site, that makes this worth 0.3%. Inside
`d_json/encode_onto_2` alone there are eighteen places that store ONE literal
byte — `"`, `[`, `{`, `,`, `]`, `}`, `:`, `\`, `n`, `t` — and between them they
run 5,583,330 times per runbench, each paying the same eleven-instruction
guard ladder first.

Every emitted vein FALLS by exactly two lines, on every program with no
exception: the decoder 9,145 -> 9,143 and each of the thirteen others two
lighter, `emitted_lines` and `emitted_other_lines` with it, and `module_lines`
5,319 -> 5,317. `calls`, `branches` and `defines` are byte-identical
everywhere, which is what a two-instruction reorder inside one `alwaysinline`
shim should look like. On the pre-#1400 base escapebench and readbench did not
move; on this base they do, because #1400's own emitted change had shifted
them and the two-line fall is uniform once both are in.

**On kanso#1400's first base the two changes were exactly additive.** Measured
on one tree carrying both, against the pre-#1400 base: runbench 2,034,746,727
-> 2,003,781,051, a fall of 30,965,676, against −24,697,356 for #1400 alone
and −6,268,320 for this one. The sum is 30,965,676 — the same number to the
instruction, with no interaction term. They touch different code (`emit_at`'s
general path against the prelude's append shim), and the re-measurement above
confirms it from the other direction: with #1400 landed, this change is still
worth the same 6.27 million.

MAPPED on the way, and the reason the ladder was found at all: `encode_onto_2`
is 340,877,621 instructions, 16.72% of runbench, over 2,380,950 calls.
TWENTY-SEVEN instructions run on every one of those calls — six pushes, `sub
$0x58`, two argument moves, the failure guard's `cmp/jne`, the
six-instruction jump table, then two return moves, `add $0x58`, six pops and
`ret $0x8` — which is 64,285,650, 3.16% of runbench, and NINETEEN of the
twenty-seven are pure calling convention (2.22%). kanso#1338 already declined
the one shape that shrinks that, outlining the arm that sizes the frame, at
+2.5582%. No instruction in the function runs more than once per call: there
is no loop, the other 276M is straight-line arm work across 843 instructions
in 48 executed runs, and the ladder above is the largest repeated shape in it.

Also measured and NOT taken: fusing adjacent appends so the second inherits
the first's guards. Eighteen of the twenty-four store sites have a call
between them and a call can move the buffer, so only the escape pairs (`\`
then `n`, `"`, `t`, `r`, `\`) are genuinely back-to-back, and those run about
100,000 times each — roughly 0.27% of runbench for a much larger change. Left
on the table with its number.

**CI's sitting, on the post-kanso#1400 base.** Four work rows fall and ten are
byte-identical, jsonbench among them: encodebench 3,692,200,106 ->
3,641,023,306 (−51,176,800 / −1.3861%), livebench 3,143,851,726 ->
3,115,992,526 (−27,859,200 / −0.8861%), runbench 2,009,290,191 ->
2,003,021,871 (−6,268,320 / −0.3120%), oneshot −69,648. The two pure-encode
benchmarks fall four and three times harder in proportion than runbench, which
is the ladder's own distribution: the sites paying it are the encoder's, the
decoder does not append single literal bytes, and jsonbench — the decode alone
— does not move at all.

CI's runbench delta is −6,268,320, which is the container's PRE-#1400 reading
to the instruction and fourteen off its post-#1400 one. Three measurements of
a two-instruction reorder on two hosts across a merge, spanning fourteen
instructions in total.

Six .text rows fall and none rises: widebench −288, encodebench −144, oneshot
and livebench −128 each, runbench −112, jsonbench −32; eight identical.

**The three compile rows split, and the split is the point.** On the pre-#1400
base all three ROSE (+1,656, +3,970, +3,563). On this base
`compile_instructions` FALLS 420 (46,105,350 -> 46,104,930) and
`library_instructions` FALLS 312 (154,371,207 -> 154,370,895), while
`entry_instructions` rises 1,225 (153,612,462 -> 153,613,687). Same source
change, opposite signs on two rows of three, and nothing about the compiler's
decisions differs: `compile_allocs` is 30,273 on both bases and
`compile_memory` is byte-identical on both. That is the layout vein behaving
exactly as CLAUDE.md describes it — the row moves with the bytes of the
compiler and the layout under them, so what else is in the binary changes its
sign. A branch that reads one of these rows as evidence about its own change
is reading the linker.

Welfare 68.50 -> 68.52, banked here.

## 2026-09-13 (twelfth) — the nightly ratchet's baseline is red on two gates, and neither reason is a mutation

Ratchet run 33 (scheduled, main at `1b51e688`) ended `ratchet: the baseline is
not green`. The baseline pass reads every gate on an unmutated worktree before
it applies anything, and a gate already red there proves nothing about any row
that shares it. Two gates were red. They are unrelated and only one is fixed
here.

### site_smoke: the blob is built, not committed, and the ratchet never built it

    ALREADY RED site (landing sample and playground run in a browser)
      gate: ./target/release/kanso run scripts/site_smoke
      error[endpoint]: unhandled err reached the executor:
        "cannot read docs/kanso.wasm: no such file"

kanso#1350 stopped committing `docs/kanso.wasm`. ci.yml's site job rebuilds it
with `sh scripts/build_wasm.sh` in the step before the gate. The ratchet's
`landing` row set its worktree up with `release` — `cargo build --release` —
and nothing else, so from the day the blob left the tree that gate answered a
missing file rather than a defect. The row has proved nothing since.

The setup is now `with_blob`, which runs `build_wasm.sh` first. The sibling row
`in_the_page` was never affected: `scripts/browser_differential.sh` runs
`build_wasm.sh` itself at line 9.

`scripts/ratchet/toolchain.sh` DOES install the wasm32 target, and says it is
there because "the browser rows' gates rebuild docs/kanso.wasm". That sentence
is true of `browser_differential.sh` and was never true of `site_smoke`.
Installing a target is not building an artifact, and the gap between those two
is exactly what nothing could see:
`tests/the_ratchet_carries_what_its_gates_need.rs` reads INSTALL LINES —
`apt-get install`, `rustup target add` — by construction, so a build step in
ci.yml is outside its reach. It gains the artifact check, watched red on the
unfixed tree and paired with the same commented-out-line trap the valgrind
check carries.

### instructions.sh: the row depends on where the tree is checked out

The second gate is a diagnosis and not a fix. Eleven of fourteen work rows
disagreed with the golden by exactly ±14 and three agreed exactly, and which
is which is a function of the benchmark's NAME LENGTH with no exceptions:

    name length   delta   rows
        6           0     basket
        7         +14     oneshot
        8           0     runbench
        9         -14     jsonbench deepbench livebench pendbench
                          readbench scanbench widebench
       10           0     indexbench
       11         +14     digestbench encodebench escapebench

The gate's own header has the mechanism: the kernel puts the exec path on the
new process's stack and libc walks it before main. Measured directly here on
one byte-identical binary, copied into directories whose length rises by one
character at a time, the count is periodic with period four and amplitude
fourteen — `/tmp/plen/aaaa`/indexbench reads 3,226,048 at a full path length
of 25, 29, 33 and 37 and 3,226,062 at every other length between.

That law plus two directory lengths reproduces all fourteen deltas exactly,
zeros included. CI's cost-goldens job runs from the repo root,
`/home/runner/work/kanso/kanso`, 29 characters. `scripts/ratchet/ratchet.kso`
sets `base_dir = "/tmp/kanso-ratchet-base"`, 23. The six-character difference
is a two-step phase shift, which sends rows whose length lands on the low
phase up by fourteen, rows on the high phase down by fourteen, and leaves the
rest alone. The header's closing sentence — "CI always runs from the repo
root, so the goldens are consistent and this costs the gate nothing" — is
false for the ratchet job, which is CI and does not.

So the gate cannot be green in the ratchet's scratch worktree, and the 28 rows
that share `sh scripts/gates/instructions.sh` have never been proved. The
ratchet's own excuse for the refusal blames silicon — "its golden is exact
counts and the pool is not one machine" — and a ±14 pattern grouped perfectly
by filename length is not silicon.

The obvious remedy is REFUTED here rather than shipped. Exec'ing every
benchmark through one constant absolute path does not make the row
path-independent: `digestbench` run from `/tmp/kanso-ir/digestbench` with only
the working directory changed still reads 10,060,595 against 10,060,609, three
times each, stable. Eight of the fourteen also read their input relative to the
working directory and answer in about 225,000 instructions from anywhere else,
so the cwd cannot simply be pinned either. A row that is a property of the
binary rather than of the checkout needs both the exec path and the input
staged at fixed paths, and that is a larger piece of work than this entry
settles.

## 2026-09-13 (thirteenth) — the instruction goldens were a property of the checkout path, and now they are a property of the binary

The twelfth entry diagnosed this and stopped there, because the obvious remedy
was refuted. This is the remedy that works.

### What was wrong

`bench/instructions_golden.txt` is exact by design — "a rise is a regression to
explain and a fall is a win to bank". It was also, silently, keyed to where the
tree sat. The count of a byte-identical binary is periodic with period four and
amplitude fourteen in the length of the path it is exec'd from: indexbench
reads 3,226,048 at full-path lengths 25, 29, 33 and 37 and 3,226,062 at every
other length between.

CI's cost-goldens job runs from `/home/runner/work/kanso/kanso`, so the goldens
mean "measured from twenty-nine characters" and nothing said so. The nightly
ratchet reads the same gate from `/tmp/kanso-ratchet-base`, six characters
shorter, and eleven of its fourteen rows disagreed by exactly +/-14 while three
agreed — grouped perfectly by the benchmark's NAME length, which is what gave
the mechanism away. Twenty-eight ratchet rows share that gate and none of them
had ever been proved.

### Why the obvious fix is not the fix

Exec'ing every benchmark through one fixed absolute path is not enough.
`digestbench` run from `/tmp/kanso-ir/digestbench` with only the working
directory changed still read 10,060,595 against 10,060,609, three reads each,
stable. The working directory matters too, and it cannot simply be pointed
somewhere neutral: eight of the fourteen open an input relative to it and
answer in about 225,000 instructions from anywhere else. Run from an empty
directory they say so out loud —

    cannot read bench/large.json: no such file

— and the list is short. `bench/large.json` for jsonbench, encodebench,
oneshot, readbench, livebench and runbench; `bench/wide.json` for widebench;
`bench/digest_input.txt` for digestbench. The other six open nothing.

### What ships

`scripts/gates/instructions.sh` copies the fourteen binaries and those three
files into `/tmp/kanso-ir` and measures from there, so the exec path and the
working directory are both constant and the row is a property of the binary.

Measured on three benchmarks that sit in different phases, from two trees
thirteen and twenty-eight characters long:

    bench          TODAY from A   TODAY from B  differ | STAGED A  STAGED B  differ
    oneshot          19,287,398     19,287,412      14 | 19,287,398  19,287,398   0
    escapebench      82,939,084     82,939,098      14 | 82,939,084  82,939,084   0
    digestbench      10,060,595     10,060,609      14 | 10,060,595  10,060,595   0

escapebench opens no file, digestbench does, and both stop moving.

NO GOLDEN CHANGES. `/tmp/kanso-ir` is thirteen characters and the repo root is
twenty-nine; both are 1 mod 4, so the staged reading is the repo-root reading.
Checked rather than argued: indexbench and digestbench read 3,226,062 and
10,060,595 from a twenty-nine-character tree and the same two numbers staged.

### The check, and the draft of it that could not fail

`scripts/gates/path_independence.sh` runs indexbench and digestbench from four
source trees whose names differ by one character each and refuses if any row
moves. It is a step in the cost-goldens job, ahead of every comparison.

The first draft used TWO trees, fifteen and thirty-one characters. It passed
with the staging taken out — the swing has period four and sixteen is 0 mod 4,
so both sat in the same phase and the check could not fail. Four consecutive
lengths cover every phase, so the refusal does not depend on having guessed the
period right. Watched red on that version, with the staging removed:

    ::error::indexbench read 3226062 3226048 3226062 3226062 from four trees
    ::error::digestbench read 10060595 10060609 10060609 10060609 from four trees

and green with it restored.

### What CI said about the first push, and two repairs

The step went red having printed nothing at all. The whole of the job log for
it was `Process completed with exit code 127`.

127 is a shell saying a command was not there. valgrind is installed inside the
step named "how much work", and this new step runs ahead of it, so valgrind was
not on PATH — and because a callgrind total is read off stderr, the gate sends
stderr to a file, which is where `valgrind: not found` went. Reproduced here by
renaming the binary in a copy of the script: exit 127, no output, the message
sitting in `/tmp/ir.pi`.

The install is now its own step ahead of both, and the gate reads its own
failures out loud: an empty count prints the captured file and says which
benchmark and which tree it was measuring.

    ::error::no instruction count came back for indexbench run from /tmp/kanso-pi.
    ::error::What the run said, in full:
    ::error::    env: 'valgrind_absent': No such file or directory

The second repair is a diagnostic bug the first push carried. `rc` was both the
per-benchmark verdict and the job's, so indexbench going red printed the error
block for digestbench too, over four readings that agreed. The comment in the
script promises that either one going red names which half broke, and one
variable could not keep that. Watched with a stub that moves indexbench and
holds digestbench still: indexbench named, digestbench reported clean, the job
still red.

The rest of the run is what the change predicted. `work:success` — the fourteen
staged rows matched the golden exactly, so `/tmp/kanso-ir` and the repo root do
sit in the same phase and no golden needed regenerating.

### A third repair: the new gate is the first callgrind gate that reads no golden

`specs` and `the other host` were both red on one spec,
`a_host_bound_gate_is_reported_not_credited`. Its property is that the ratchet's
`host_bound` list — gates whose red the baseline reports rather than fails on —
holds exactly the gates that count instructions under callgrind. The list is
pinned to a property of the gates instead of to anyone's judgement, because an
entry excusing a gate that is not silicon-bound turns a real failure into a
note. `path_independence.sh` runs callgrind, so the list and the property came
apart the moment it landed.

Declaring it host-bound would have been the wrong repair. The four gates on the
list diff their counts against numbers a different machine wrote down, which is
why a foreign runner reddens them whatever the mutation did. This one counts one
binary from four tree depths and asks the four readings to agree with EACH
OTHER; every runner in the pool answers it the same way, and a red here is
always a real red.

So the property gained its second half: a gate is host-bound when it counts
instructions under callgrind AND compares them against a recorded golden, which
it says by calling `host_gate.sh`. Watched three ways before it passed — giving
`path_independence.sh` a `host_gate.sh` call puts it on the right-hand side and
the list goes red; taking the call out of `instructions.sh` drops it off and the
list goes red; removing `bound_a` from the list goes red without either script
moving.

## 2026-09-13 (fourteenth) — the digit test built a boolean and took it apart, three instructions on every byte of every number

`d_json/scan_4` is 4.65% of runbench — 93,099,402 instructions over only 113
addresses — and unlike the functions above it in the profile it is not per-call
work. Four equal-trip groups at about three million trips each sit in one
contiguous run, 0x24a40 to 0x24a8c. That is the loop over the bytes of a JSON
number, and twenty instructions in it cost 63M, 3.14% of the program.

Six of the twenty were this:

    0x24a64  3,062,862  add    $0xd0,%al      ; b - '0'
    0x24a66  3,062,862  cmp    $0xa,%al       ; carry set iff it is a digit
    0x24a68  3,062,862  mov    $0x3,%eax      ; <-- the flag becomes a value
    0x24a6d  3,062,862  sbb    $0x0,%rax      ;     2 if digit, 3 if not
    0x24a71  3,062,862  cmp    $0x2,%rax      ; <-- and a flag again
    0x24a75  3,062,862  jne    24ab1

The middle three carry the answer of `cmp $0xa,%al` across to the `jne`, which
`jb` would have taken straight off the flags. The source said

    digit = 47 < c and c < 58
    if digit (scan cs start (p + 1) marked) (number_done cs start p marked)

and `emit_cond` never saw the comparison. It walks an `and` and branches off
the flags — its own comment says so, and kanso#1271 shipped that — but only
when the condition IS the expression. Here the condition is a bound NAME, and
`f.lookup` gives back an SSA operand rather than the expression behind it, so
the emitter fell through to taking a value apart. In the IR the tell is one
line:

    %t80 = icmp slt i64 %t78, %t79
    %t81 = select i1 %t80, %KValue { i64 2, i64 0 }, %KValue { i64 3, i64 0 }

The first comparison of the `and` branches on its i1; the last one, whose value
the binding holds, is materialised.

Asked in place the select is gone and the loop branches off the flags. Measured
by copying both runbench binaries into one directory and counting each there,
because the exec path shifts a count by fourteen (the thirteenth entry):

    baseline  2,003,781,671
    in place  1,994,172,731   -9,608,940, -0.4795%

Output byte-identical, and the whole suite green on all three engines, 454
passed. The disassembly predicted 9,188,586 from the three instructions alone;
the extra 420,354 is the register pressure the materialisation cost around
them.

THE NAME IS THE PRICE. `marked` had to become `m` for the line to fit eighty
columns, and that formatting rule is why the binding was written in the first
place. The file already calls its other parameters `cs`, `p`, `bs` and `n`, so
the short name is in keeping — but the durable fix is the emitter seeing
through a once-used binding, and that would give the longer name back. It needs
either the binding's expression kept where `emit_cond` can reach it or a
front-end rewrite that inlines a single-use condition; `lookup` returning an
operand is the whole obstacle.

AND THE CENSUS SAYS THAT FIX BUYS NOTHING HERE — but the first census was
wrong, and the way it was wrong is the finding.

Sweeping every `.kso` under lib, std, scripts, bench and hako for a binding
whose value is a comparison, `and`, `or` or `not` and whose name is an `if`
condition within eight lines returns seven sites: this line, its twin in
`bench/jsonbench/jsonbench/number.kso` (the frozen decoder jsonbench compiles,
a control that stays as it is — its counters are byte-identical here, as they
should be), three that bind `list/find` where the name holds an option a reader
needs, and two in `hako/hako/update.kso` where the name is read twice, so a
once-used rule would not fire, and which no benchmark compiles.

That sweep filtered on the operator and so could not see `blank = ws? c`, which
stands EIGHT times in `lib/json/value.kso` on the decoder's busiest paths —
`array_delim` alone is 4.13% of runbench. Dropping the filter finds them, and
the disassembly looks like the same defect: at 0x29200 sit `mov $0x2,%edi` /
`cmp $0x2,%rdi` / `jne`, and at 0x292b9 `mov $0x3,%edi` / `cmp $0x2,%rdi` /
`je`, two flags turned into a value and back.

NEITHER BLOCK EVER RUNS. A callgrind profile taken with `--dump-instr=yes`,
parsed so its self costs reconcile to the program total exactly
(1,994,172,731, every function agreeing with `callgrind_annotate`), gives all
six addresses a count of zero. The `jae` at 0x291fe is taken on all 672,606
trips: the input has no whitespace between array elements, so the arm the
binding sits in is never entered. Reading a cost off a disassembly is what
this was — the instructions are in the binary and they cost nothing, and the
same profile says `array_delim` is not a loop at all but 122 instructions of
per-call work spread over 492 addresses, the hottest reached 943,569 times.

MEASURED, AND IT IS NOT. All eight rewritten to ask in place, as
`if (ws? c) ...`, build a runbench byte-identical to main's — md5
034613928ffe1a3ef39424e0c8b92353 on both sides — and `emitted_code` AGREED. `ws?` is a group over the byte
returning literal `true` and `false`, so the condition is a CALL, and
`emit_cond` has nothing to walk: a callee hands back a tagged value whether or
not the caller names it. The binding is free. What `emit_cond` can walk is an
`and` of comparisons, which is why the line above it paid and these eight do
not. The eight fewer source lines move one vein, `front_end_visits` 22,449 ->
22,359, which is not a welfare term, and the change is declined — the name
`blank` says what the test means.

So the emitter learning to see through a once-used binding is worth the name it
gives back and nothing measurable on this tree, and the shape worth teaching it
next is the other one: a group whose arms are all boolean literals, whose
switch could jump straight to the `if`'s targets instead of building a tag for
a compare to take apart. That is where the six instructions at 0x29200 and
0x292b9 live, and no source spelling reaches them.

Two compile veins moved and both are regenerated here. The emitted goldens lose
two branches and one line in every program that carries the number scanner —
the decoder 795 -> 793 branches and 9,143 -> 9,142 lines, oneshot 784 -> 782
and 9,072 -> 9,071, livebench 798 -> 796 and 9,189 -> 9,188, runbench 3,447 ->
3,445 and 34,789 -> 34,788 — which is the materialisation leaving. front_end
visits fall 22,449 -> 22,437, twelve fewer expressions for the front end to
walk on each round it is dirty; rounds hold at 62. Every runtime counter and
the lazy tier are byte-identical: this removes instructions, not events.

## CORRECTION: the 9.6M does not reproduce on CI, and the change is a compile win

CI read `work:success` on this branch — runbench 2,003,021,871, identical to
its golden, every one of the fourteen work rows unmoved. The −9,608,940 above
is real on this container and is a property of its LLVM, not of the compiler:
rustc here is 1.94.1 against CI's 1.98.1, and CI's backend evidently already
folds the materialised boolean that this container's leaves standing. The
emitted IR still loses the branches on both — CI's own `runbench defines=593
calls=5943 branches=3445 lines=34788` matches the regenerated golden exactly —
so the select is gone from the IR and the machine code was already without it.

The lesson is the one this repo keeps relearning about host-keyed veins, in a
direction it had not hit before: a RUNTIME row can be host-keyed too, not by
the fourteen-instruction exec-path offset kanso#1404 fixes, but by which
optimiser saw the IR. A container measurement of a codegen-shaped change sizes
what THIS toolchain does with it, and CI is the only authority on what ships.

What lands, then, is compile-side and small. All four compile veins move
because `lib/*.kso` is `include_str!`'d into the compiler:

    compile_allocs        30,273 ->      30,258   (-15)
    compile_instructions  46,104,930 ->  46,072,247  (-32,683, -0.0709%)
    entry_instructions    153,613,687 -> 153,586,146 (-27,541, -0.0179%)
    library_instructions  154,370,895 -> 154,378,731 (+7,836,  +0.0051%)

Three fall and the library row rises, which is the layout vein behaving as
CLAUDE.md describes it. The change stands on that and on eight fewer source
lines, not on the runtime figure it was built for.

**The floor is banked, and that was the round CI asked for.** With the four
goldens carrying CI's rows, the welfare job read 68.52 against a floor of
68.5198 and failed the PR: the objective went up and nobody held it, which is
a red pull request rather than a gift to the next change. `--set` ran on the
committed goldens, in the order CLAUDE.md gives — CI's rows first, then the
ratchet, then the page spans — and the floor is 68.51979544326865 ->
68.52093722983558, ratchet 261. The three page gates agree afterwards:
golden_prose 0 drifted, page_drift 1/3, prose_check 0 tells. The whole rise is
the compile term; the run term did not move at all, which is the correction
above stated as a number.

## 2026-09-13 (fifteenth) — the instruction vein counts a byte of memcpy as an instruction

Chasing a lead off the runbench profile turned up something about the profile
itself. `__memcpy_avx_unaligned_erms` is 38,690,280 of runbench's 1,994,172,731
— 1.940% — and 27,194,862 of that sits on ONE address, 0x188d87. An address
that hot is either a loop body or something counted oddly. It is the second:

    188d84:  sub    %rdi,%rcx
    188d87:  rep movsb %ds:(%rsi),%es:(%rdi)
    188d89:  vmovdqu %ymm0,(%r8)

Callgrind simulates a `rep`-prefixed string instruction one iteration at a
time, so `rep movsb` costs one Ir per BYTE moved. The arithmetic closes it:
instrumenting `k_b_append_grow`'s copy says runbench moves 24,964,380 bytes
through it in 1,080 copies, largest 138,756, and the profile's call edge from
that function into memcpy reads 24,342,225 Ir. That is 1.026 bytes per
instruction — the count is the byte count.

WHAT THIS MEANS FOR READING THE VEIN. ERMS moves tens of bytes a cycle, so
memcpy's real share of runbench is a small fraction of the 1.940% the profile
shows, and a lead ranked by Ir that lands on memcpy is a mirage. That is how
this one died: `k_b_append_grow` looked like 24.3M instructions, 63% of all
memcpy and 1.22% of the program, and it is 25 MB of copying that the hardware
does in about a millisecond. Growth is already geometric — `cap = 2 * (a->len +
n)` — so 25 MB against 1,080 copies is amortised doubling behaving exactly as
it should, and there is nothing to fix.

The objective is not wrong, and nothing here asks to change it. Its run term is
deterministic, comparable between two trees, and that is what a ratchet needs.
But a change that trades work for bytes moved, or bytes moved for work, is
scored on a scale where one byte weighs one instruction, and the two are not
worth the same. Read a memcpy row as bytes, and price a change against it
knowing that.
## 2026-09-13 (sixteenth) — ch05 never said what an effect hands back

`done` was ruled at the 2026-08-26 sitting and built in kanso#1363: a
succeeded effect yields it, `none` means absence and nothing else, and the
two used to be the same answer. Nothing in the book said so. A census of
every chapter and every appendix for the value found it nowhere — ch03,
ch04, ch08 and ch10 carry the English word "done" in ordinary prose, and no
page carries the kanso one. ch05 is the effects chapter and taught the
whole i/o vocabulary without saying what a `print` or a `write_file`
answers.

This is the last piece of STATUS.md's "The book teaches the boundary
language" that a census can find still owed. The rest of that row has
landed in pieces: kanso#1394 put ch04's boundary panel in, kanso#1392 gave
ch05 all three chain words and all three fused spellings, kanso#1395 the
`<t>effect` spelling. ch04 has carried the one sentence the 2026-08-17
gavel named — "your own failures only bubble" — since kanso#1076.

**Where it goes.** After the `save.kso` round trip, which is the first
place a reader watches a yield get discarded: `os/write_file "order.txt"
order >> os/read_file! "order.txt" .> print`. The paragraph that follows
already explains that `>>` orders two effects that share no data, so the
question "then what did the write answer?" is one line away, and the
chapter used to leave it there.

**The panel.** `yielded.kso` binds the write's yield with `.>` and prints
three lines: the yield interpolated (`<done>`), which of two arms caught it
(`done`, not `none`), and `yield == none` (`false`). Three facts, one
program, and each of them is a thing #1363 changed.

**Watched red twice, each for its own reason.** The book has two gates over
a panel and they fail on different things, so both were made to fire.
Changing one word of the panel's source away from the .kso turned
`book_panels` red (`drifted: ch05.html :: yielded.kso`) — that is the proof
the gate reads this panel at all rather than skipping it. Recording the
.out as the language answered BEFORE `done` existed — `<none>`, the `none`
arm, `true` — turned it red again. The second red is the one that matters:
it is this panel going red on a compiler where the ruling had not shipped.

**Checked rather than repeated.** The #1363 log entry says a read that
finds no file answers `none`. On current main it does not: `os/read_file
"nope.txt"` answers `os/file_not_found "nope.txt"`, which is the
fallback.kso panel four sections later. The prose says an unset environment
variable instead, which was run and does answer `<none>`. Also deliberately
absent: a count of the effects that yield `done`. The entry for #1363 says
twelve; a number in the book goes stale the first time an effect is added,
and nothing in the tree would catch it.

All three page gates agree afterwards — golden_prose 0 drifted, page_drift
2/3, prose_check 0 tells — and `book_check.sh` verifies every sample.

## 2026-09-13 (seventeenth) — the log back to forty, and three leads closed by reading a measurement correctly

Seven entries out, byte-identically: `design/compiler-log.md` lines 23..933 of
origin/main appended to `design/log/compiler-log-archive.md`. Live 46 -> 40
headings, archive 1,233 -> 1,240; 1,279 across the pair before, 1,280 after,
the one addition being this entry. No heading is duplicated. The archive's
newest entry is now "the fused chain operators" and the live log's oldest is "a
record built into its own first field", which directly followed it, so the
chronology across the pair is unbroken.

Verified four ways before committing: the archive's original bytes are an exact
prefix of the new file; the moved headings are the oldest prefix of the old live
log's, in order; the kept headings are the rest of it, in order, with this entry
appended; and every heading across the two files appears exactly once.

**Three leads closed, and none of them needed a build.** All three were closed
by reading a measurement correctly rather than by changing anything, which is
worth recording together because the same mistake was available in each.

**`d_runbench/tally_4` is not harness overhead.** It is the fifth-largest self
row in runbench at 92,053,131 (4.59%), and the run program's own driver showing
up that high would mean the objective's run term partly measures the harness.
It does not. `bench/runbench/runbench/runbench.kso`'s `tally` calls
`escape/total`, `index/total`, `split/total`, `decoded`, `encoded` and
`digested`, and no `d_escape/`, `d_index/`, `d_split/`, `d_deep/` or `d_pend/`
row appears anywhere in the profile: they are all inlined. The 92M is those
phase bodies sitting in tally's frame.

**And the factor of three against the phase map is a unit error, not a stale
table.** `bench/runbench_phases.txt` gives escape 4.95% + index 4.97% + split
4.87% = 14.79% for exactly the three phases tally inlines, against tally's
4.59%. The phase table's columns are whole-build INCLUSIVE totals — its header
says each phase was built alone with the other seven zeroed — and 4.59% is
callgrind SELF cost. The callees are the runtime rows below it. 448M inclusive
against 92M self is consistent, and the phase map is not stale.

**The compile side's rehash lead is refuted the way kanso#1163's was.** The
front end is flat: `kanso check compile_corpus` in the gate's box reads
47,358,302 and no self row exceeds 4.83%. The aggregates are not flat — hashing
sums to 13.13% and allocation to 14.36% — and `reserve_rehash` at 1,150,492
(2.43%) looked like maps still growing after the pre-sizing work. Attributing
it with `--separate-callers=2` finds no owner: the largest kanso-named site is
`qualify` at 83,596 (0.18%), then `inline::aliases` 82,014, `demand::analyze`
77,341, `advisory::name_types` 51,398 and 59,688, `inline_builtin_wrappers`
44,989, `trmc::rewrite` 43,877, `compile_module_loaded` 41,644. A dozen sites,
none above 0.18%, against an earlier measurement that priced pre-sizing six
filtered collects at 4,514 instructions. Do not re-open without a new
mechanism.

**What that mapping leaves standing, unstarted.**
`kanso::check::check_merged_after_aliases` is the single largest self row at
2,289,636 (4.83%) and has been worked twice without being displaced. Nothing
here says what those instructions are spent on; that measurement was not taken.

THE SHAPE TO CARRY. Twice in one day a self cost was about to be written down
as though it were inclusive — once in a hand-written callgrind parser that
reported 99,188,064,506 against a 1,994,172,731 program because it read the
cost line after `calls=` as self, and once here against the phase table. Read
the units before writing the number down.

## 2026-09-13 (eighteenth) — the name questions join the one descent

Built: the sixth fold. kanso#1382 through kanso#1386 moved five checks into
`check_per_node`'s single descent and each returned between 0.36% and 1.14% of
the summed compile term. `check_named_per_node` was the largest check still
walking the program on its own, and it is the sixth to go in.

`named_walk` was a `for_each_child` recursion over the same declarations and
the same statements `per_node_walk` already descends, asking three questions at
each node: a call's arity against the group that could answer it, a foreign
type built outside its owner, a typeset named as a value. What is left once the
recursion line goes is `named_at`, called from `per_node_walk` beside the other
eight. `Named` rides in `PerNode` as one reference, the way `FieldScan` and the
literal groups do; `own` and the shadowable list ride in `DeclState`, which is
already per-declaration.

**The answers go in their own vector, and that is deliberate.** This route hands
diagnostics back in PUSH order — only the arity-gated early return sorts — and
the fused walk's answers are rotated to the back so they land where the checks
they replaced used to push. A name answer riding along with them would come out
after every check that used to follow it. So `DeclState` carries
`named_diags`, `check_per_node` returns it, and `check_merged_after_aliases`
splices it in at the position `check_named_per_node` pushed from. The arity gate
also keeps reading only the walk's own vector, so a name-side arity refusal does
not trip a return that was written about a different refusal.

MEASURED on the container, `kanso check` in the gate's box, `kanso::main`
inclusive, one build each:

    vein                 main           fold          delta
    module         46,906,319     46,545,400   −360,919 (−0.7695%)
    entry         156,302,043    155,075,270 −1,226,773 (−0.7848%)
    library       157,445,822    156,226,595 −1,219,227 (−0.7744%)
    summed        361,654,184    357,847,265 −2,806,919 (−0.7761%)

The container reads about 0.8% above CI on all three rows, an offset every
compile vein has carried since kanso#1337, so the projection onto the landed
goldens is roughly −354,600 / −1,205,500 / −1,195,500, summed −2,755,600
(−0.7783%). The goldens in this commit still hold main's values and round one is
expected red on all three; CI's rows are written in the round after.

**The corpus could not see the ordering, and now it can.** Thirty-two fixtures
in `tests/golden/errors` raise more than one diagnostic and not one of them
paired a name answer with a later check's, so splicing the name answers at the
END instead of at their old position left all 465 fixtures green. That is a gap
rather than a licence. `a_typeset_beside_a_later_refusal` raises both — `shape`
named as a value, which the name half refuses, and `let x = 1`, which
`check_binding_patterns` refuses after it — and it was watched red under exactly
that mutation, the two lines coming back in the other order.

**OPEN: the shadow suppression may be unreachable.** `arity_at`'s one use of the
declaration's bound names is to drop a diagnostic when a local shadows a
declared group, and the machinery around it — the stretch list, the second pass
that builds the bound-name set only when that list is non-empty — moved across
verbatim. Deleting the drop leaves all 465 error fixtures green, leaves
`compile_corpus` and `lib/json` compiling, and the two shapes that would reach
it are refused earlier by "`x` is already a declaration; rename the binding":
a top-level bind of a declared name, and a parameter named after one. Whether
any program can reach it is not answered here, and the machinery stays until
something answers it.

CI's rows, round two. `compile_instructions` 46,072,247 -> 45,708,985 (−363,262
/ −0.7885%), `entry_instructions` 153,586,146 -> 152,355,905 (−1,230,241 /
−0.8010%), `library_instructions` 154,378,731 -> 153,175,369 (−1,203,362 /
−0.7795%); summed 354,037,124 -> 351,240,259 (−2,796,865 / −0.7900%). The
container projected −2,755,600 summed, so the projection came in at 0.9853 of
the landing — the second-closest of the six folds, behind kanso#1386's 0.9960.

Only those three moved. The job's own vein summary — the authority, since the
nineteen counter steps are `continue-on-error` and their API conclusions lie —
reads `compile memory:success`, `compile allocations:success` and `machine
code:success` alongside the three failures. That agrees with the box: allocs
30,258, alloc bytes 4,841,171, peak 777,126, passes 8, rounds 62 and visits
22,437 are all byte-identical between main and the fold. A descent goes; an
allocation does not, and `.text` does not move either, which is worth saying
because `src/check.rs` changed substantially and the machine-code vein has
caught layout-only moves seven times before.

And the three rows REPRODUCED. CI read them twice, on two commits that differ
only in markdown so carry the same compiler binary — `fd7ee486` before the base
merge and `7f7a65a4` after — and both sittings gave 45,708,985 / 152,355,905 /
153,175,369, with `compile_allocs` 30,258 and every `text=` and emitted row
identical between them. That is what `compile_instructions.sh` asks for in
place of a per-host key: the same build on any runner counts the same number,
and a run that disagrees halts the vein. These did not disagree.

Welfare rose 0.01 and is banked in the same round, the goldens carrying CI's
rows first: floor 68.52093722983558 -> 68.5353360462189, ratchet 262. Five
`data-golden` spans on compiler.html quoting the compile rows were rewritten by
`all_pages.sh --write`, and all three page gates agree afterwards.

## 2026-09-13 — a second fused descent, and a linker coin-flip the compile veins cannot see past

**DONE — the two post-inference questions join one walk (kanso#1409).**
`check_effect_discarded` and `check_none_exhaustive` each walked the whole
program for itself: the same declarations, the same statements, the same nodes,
in the same order, each with its own `Vec<&Expr>` stack and its own
`for_each_child`. They are asked on one descent now.

**This is not the fold kanso#1382–#1408 built six times, and the difference is
structural rather than stylistic.** Those six moved a check into
`check_per_node`'s descent. These two cannot go there. Both read `inference`,
and `check_merged_after_aliases` builds that AFTER `check_per_node` returns —
and only once the arity gate has passed, because inference indexes an `if`'s
branches and must never run over a shape the walk refused. Four checks still
pay a descent of their own, and all four are on the far side of that gate:
these two, `check_wall_operands` and `check_box_where_value`. So the remaining
work is a SECOND fused walk rather than more of the first. This entry is that
walk with the first two in it.

Sized off the `#[inline(never)]` profile that answered kanso#1408's question:
each of the four pays 180,936 instructions of `for_each_child` descent, the
same figure to the instruction, which is exactly the part a shared walk
removes.

**The answers go into two vectors, and that is the whole care of the change.**
This route hands diagnostics back in push order — only the arity-gated early
return sorts — and the two checks pushed from either side of the fused walk's
`diags.rotate_left(walked)`. One shared vector would move one of them. So
`check_after_infer` returns both separately and the caller splices each where
its own check used to push.

**Measured** on the container, in the gate's box, one build each, both built
from the same worktree so the toolchain state is identical. PROGRAM TOTALS
rather than `kanso::main`, for the reason below:

    vein      main          fold          delta
    module     47,015,134    46,735,014   −280,120   (−0.5958%)
    entry     155,535,350   154,624,501   −910,849   (−0.5856%)
    library   156,349,297   155,461,319   −887,978   (−0.5679%)
    summed    358,899,781   356,820,834  −2,078,947  (−0.5793%)

**OPEN, and it is about the veins rather than about this change — valgrind
cannot read the symbols of the binary this branch builds.** The compile gates
read `kanso::main` inclusive. On this binary valgrind never prints
`Reading syms from` for the executable at all, so every kanso frame comes back
as a hex address and that row is unavailable. It is not transient: it survives
a clean rebuild, it follows the bytes when the binary is copied under another
name, and the base binary built minutes earlier in the same worktree and run
from the same directory resolves 244 kanso frames.

The cause is a linker layout difference and nothing else. `readelf -l` on the
two binaries differs in exactly one place: `.relro_padding` sits inside the
RELRO/data segment in main's binary and in the following segment in this one.
The section table is otherwise identical, `.symtab` holds 6,687 entries, and
`nm -C` finds `kanso::main` at 0x13ea80.

It is rare rather than universal, and that was checked rather than assumed: a
probe binary built from merged main plus one dead function resolves 265 kanso
frames and keeps `.relro_padding` in segment 04. So an arbitrary edit does not
flip it; something about the size or ordering this change gives the data
segment does.

Comparing PROGRAM TOTALS costs nothing here, because on the base binary the gap
between totals and `kanso::main` is 469,781 / 470,606 / 470,527 across the three
veins — flat to within 825 instructions — so the deltas are identical either
way. What is worth watching is CI. `compile_instructions.sh` treats a missing
`kanso::main` frame as a toolchain failure and halts the vein rather than
pinning whatever the pipe returned, which is the right call and the reason the
gate has that arm at all. But it means all three compile veins are hostage to
where the linker puts one padding section, on any change, independent of what
the change does. If CI's valgrind reads this binary the rows land normally. If
it does not, that is the finding, and the gate's arm is what surfaced it.

**The corpus could not see the ordering, and now it can.** Splicing both answer
vectors at the END instead of at their own positions left all 465 error
fixtures green. Not one of them paired an `effect` or `exhaustive` answer with
a diagnostic from a check that pushes after it — the same gap kanso#1408 found
on its own half, in a different place. `an_effect_discarded_beside_a_later_refusal`
raises both: `ignore` throws away the effect it is handed, which
`check_effect_discarded` refuses, and two non-final lines compute values nobody
reads, which `check_discarded_value` refuses after it. Under exactly that
mutation it goes red with the effect line rotated to the back — `unused /
unused / effect` against `effect / unused / unused`.

**CI's sitting, and the symbol question ANSWERED: the flip is this container's,
not the veins'.** Round one measured normally on CI. No gate errored, nothing
said `the profile carries no kanso::main frame`, and all four rows came back:

    row                    main          kanso#1409    delta
    compile_allocs           30,258          30,241    −17       (−0.0562%)
    compile_instructions 45,708,985      45,490,501    −218,484  (−0.4780%)
    entry_instructions  152,355,905     151,628,169    −727,736  (−0.4777%)
    library_instructions153,175,369     152,469,420    −705,949  (−0.4609%)
    summed              351,240,259     349,588,090  −1,652,169  (−0.4704%)

So CI's linker puts `.relro_padding` where its valgrind can still read the
binary, and the paragraph above is about this container rather than about the
compile veins. The gate's halt-the-vein arm is still the right arm to have —
it is what would have surfaced this had CI hit it — but nothing here shows the
veins are hostage on CI, and the earlier draft of this entry said they might
be. Corrected here rather than left standing.

`compile_allocs` moved and the container could not see it. The fold drops one
of the two `Vec<&Expr>` worklists and adds two `Vec<Diagnostic>` to keep the
checks' answers apart; seventeen allocations is the net. It is not in the
container's reading at all, because that reading was whole-process instruction
totals.

**The container over-projected, and by more than any fold before it.** It
projected roughly −2,061,000 on the summed term and CI read −1,652,169, a ratio
of 0.80. The five folds of kanso#1382–#1386 read 0.98 to 0.996 against the same
kind of projection, and kanso#1408 read 0.985. The difference is the
measurement rather than the change: those were `kanso::main` against
`kanso::main`, and this one was PROGRAM TOTALS across two binaries the linker
laid out differently, so the loader and stack-guard work above `main` is not
the same quantity on both sides. A projection off whole-process totals is worth
about what this one was worth — the sign and the order of magnitude — and the
number to quote is CI's.

Runtime did not move: `work:success` on the same run, runbench 2,003,021,871,
identical to its golden. Welfare 68.5353360462189 -> 68.54457814481255, banked
in the same round; seven `data-golden` spans on docs/compiler.html rewritten.

## 2026-09-13 — the wall question joins the second descent, and two mutations the corpus could not see

DONE. `check_wall_operands` walked the whole program for itself, over the same
declarations, statements and nodes that `check_after_infer` had just walked in
the same order. It asks its question on that one descent now. Three questions
share the walk where two did: the effect one, this one, and exhaustiveness.

The wall's answers go into their own vector and are spliced in where the check
used to push — after the effect answers, before `check_discarded_value`, ahead
of the rotation. That is the whole of what a reader sees change, which is
nothing: the diagnostics come back in the order they came back in before.

Two properties this fold could break, and what the corpus said about each.

The `bound` shielding is pinned. Hand `never_describes` an empty set instead of
the declaration's bound names and `a_wall_whose_name_is_a_local` goes red,
because `naturals` is bare-enrolled in std/list and the fixpoint answers it at
arity zero. Watched.

The SPLICE POSITION was not pinned, and nothing in the corpus held a wall
refusal beside an exhaustiveness one. Move the append to after
`diags.rotate_left(walked)` and all 465 error fixtures stayed green.
`a_wall_refused_ahead_of_an_earlier_exhaustiveness` closes it: one program with
one of each, and the wall is reported first though it is nine lines later, so
the order is explicable by the splice and by nothing else. Watched red under
that exact move — the two lines swap — then green.

A THIRD mutation left the corpus green and is NOT a gap to close with a
fixture. Drop the `decl.synthetic` skip and nothing notices, because trmc is
the only pass that writes synthetic declarations and every body it writes is a
`BinOp` or an `App` over names and integers — no `Seq`, so no wall. The skip is
live rather than dead: 24 synthetic declarations reach this check on
`kanso check lib/regexp`, arriving through dependencies that finished their own
compiles before the merge. `trmc::rewrite` runs after
`check_merged_after_aliases` on every route, so a declaration this pass writes
is never checked in the compile that wrote it — only in a dependent's. The skip
is kept because it is the behaviour that was there; it is not reachable from
anything trmc can produce today.

Container projection, and the caveat is the same one kanso#1409 recorded. The
baseline binary's `.relro_padding` lands in the segment after RELRO, valgrind
declines to read its symbols, and `kanso::main` is unavailable — so this is
PROGRAM TOTALS against PROGRAM TOTALS across two differently-laid-out binaries,
which #1409 measured at 0.80 of CI where main-against-main read 0.98 to 0.996.
Two separate builds of merged main's source landed in the same bad layout and
neither could be read. Module 46,577,748 -> 46,344,651 (−233,097 / −0.5004%),
entry 154,193,331 -> 153,442,665 (−750,666 / −0.4868%), library 155,036,546 ->
154,286,263 (−750,283 / −0.4839%), summed −1,734,046 (−0.4873%). The gap
between totals and `kanso::main` on the readable binary is 460,605 / 461,402 /
461,329 — flat to within 800 across the three routes, and about 9,200 from the
gap #1409 read on its own binary, which is the size of the error this
substitution carries. CI's rows are the ones to quote.

The runtime sweep agrees with its goldens and the full suite is green.

CI'S SITTING, and the caveat above did not bite. compile_allocs 30,258 ->
30,224 (−34), compile_instructions 45,490,501 -> 45,251,941 (−238,560 /
−0.5244%), entry 151,628,169 -> 150,873,629 (−754,540 / −0.4977%), library
152,469,420 -> 151,715,592 (−753,828 / −0.4944%), summed −1,746,928
(−0.4997%). The container's totals-against-totals projection was −1,734,046
and CI read 1.0074 of it — closer than the six main-against-main folds before
it, and well inside the ~9,200 of gap drift that substitution carries. So the
0.80 kanso#1409 read is what that shape CAN cost, not what it must: a fold
whose whole effect is one descent removed moves the startup work not at all,
and the two binaries' pre-main gaps happened to sit near each other.
compile_memory, machine_code, emitted_code and compile_libraries all AGREED.
Runtime is untouched: `work:success`, runbench 2,003,021,871, identical to its
golden. Welfare 68.5446 -> 68.5543, banked, ratchet 263. Seven `data-golden`
spans on docs/compiler.html rewritten.
## 2026-09-13 — ch04 names the type its plumbing hands around, and a branch STATUS.md said was gone

`<t>effect` shipped in kanso#1395 and ch05 teaches it properly: an effect has a
type, the compiler infers it, a parameter can be declared `e:<string>effect`,
and the refusal a boxed value earns is a panel. ch04 reaches the same machinery
nine sections earlier — "the two failures" opens by pointing `os/read_file!` at
`version.txt` and piping the answer with `.>` — and it named nothing. It called
the value "a description of the read" and asked the reader to take the plumbing
on credit for two pages, which is the right instruction and leaves the reader
without a word for the thing on the page in front of them. One clause names it.

This is the live remainder of the ruling STATUS.md carries as "The book teaches
the boundary language" (queued P1, 2026-08-26). kanso#1392 gave ch05 the three
chain words and the three fused spellings, kanso#1394 gave ch04 its rescue
collision and its boundary panel, kanso#1406 gave ch05 `done`. Measured on
merged main before this change: ch05 names `<t>effect` twice and carries
twenty-two chain-operator spellings; ch04 carried ten of the operators and zero
of the type.

**And the branch that row says does not exist is in the container.**
STATUS.md's note reads "That branch is not in the compiler worker's container —
no branch or worktree matching book/ch04/ch05/boundary exists there — so treat
the prose as unwritten until someone points at a commit." `claude/book-effect-type`
is checked out at `/tmp/wt-book`, tip `d540fe3a`, with three real commits under
two merges of main. The conclusion the note draws is right and the reason is
wrong: every file that branch adds is already on main. `boxed.kso`,
`boxed_check.out`, `fused.kso` and `fused.out` are all in
`docs/book/samples/ch05/` on `origin/main`, and its ch05 prose landed through
kanso#1392 and kanso#1395. A `git diff origin/main...HEAD` on it reads as 71
insertions across twelve files, which is what made it look live; that is a
three-dot diff against a stale merge base. Two-dot against current main it is
ninety-six files and six thousand deletions BEHIND. The branch is superseded,
not pending, and nothing is owed to it.

## 2026-09-13 — the box question joins the second descent, and the splice gap a third time

DONE. `check_box_where_value` was the last of the four checks that walked the
whole program after inference for itself. It asks on the same descent now.
Four questions share one walk over the declarations, the statements and the
nodes: the effect one, the wall one, this one, and exhaustiveness.

Its tables are the heaviest of the four and they move whole: the group table
carrying a return set, a member chain and a binds-anything mask; the tail
walk that decides whether a group answers a box by following its arms'
last statements; the lazily-filled binder set behind the shadowing test. None
of that changes. What goes is the second `for decl in &program.fns` loop and
the second `Vec<&Expr>` worklist under it.

The binder set is the one piece that has to be rebuilt per declaration rather
than once: `shadows(i, name)` is asked of THIS declaration and fills on the
first ask, so the closure the walk hands to `site` is made inside the
declaration loop. Everything else is built once above it.

THE SPLICE GAP IS NOW THREE FOR THREE. Move the box answers from after the
exhaustiveness answers to beside the wall's, and all 465 error fixtures stayed
green — the same thing kanso#1409 found for the effect answers and kanso#1411
for the wall's. Nothing in the corpus held a box refusal beside another
post-inference one. `a_box_refused_after_an_earlier_exhaustiveness` closes it:
one program with one of each, watched red under that exact move (the two lines
swap), then green. Three folds, three fixtures, one shape of gap — a corpus
organised by what a check refuses has nothing that pins the ORDER two checks
refuse in, and the fused descent is what makes that order a property of one
line rather than of the call sequence.

Container, main-against-main this time — both binaries let valgrind read
`kanso::main`, which kanso#1411's baseline did not. Module 45,884,046 ->
45,622,817 (−261,229 / −0.5693%), entry 152,981,263 -> 152,124,687 (−856,576 /
−0.5599%), library 153,824,934 -> 152,964,635 (−860,299 / −0.5593%), summed
−1,978,104 (−0.5609%). The six folds that measured this way read 0.98 to 0.996
of CI.

The runtime sweep agrees with its goldens and the full suite is green.

**And the fold made the ratchet's own mutation stale, in both halves.** CI's
ratchet job went red on `1 mutations no longer apply` — `the_box_check_asks_
when_nothing_answers_a_box`, the mutation that proves the cost-goldens job can
see the box question's `any_boxed` skip. It greps for
`let any_boxed = returns.values()...` and the fold renamed that map to `groups`,
because `check_after_infer` already had a `returns` parameter. The anchor is
repointed.

The comment above it was stale in a way the grep could not catch. It said the
skip is worth 637,295 instructions on the module corpus and 1,308,849 on the
entry corpus, "43% of what the whole pass costs". Re-measured on this branch:
+103,180 and +114,497. A factor of six. Both readings were right when taken —
while `check_box_where_value` was a pass of its own the skip bought its entire
traversal, and now that the question rides a descent somebody else is making
the skip buys only the per-node work. The mutation still turns the gate red by
a hundred thousand instructions, so it still does its job; what it proves is
just smaller. Rewritten with the new numbers and the reason they fell, because
a mutation's prose is the only place that reasoning is recorded.

**CI's rows, and the floor.** The compile gates refuse on this container, so
round one went deliberately red on all four and CI measured them:
compile_instructions 45,251,941 -> 44,994,843 (−257,098 / −0.5681%),
entry_instructions 150,873,629 -> 150,030,446 (−843,183 / −0.5589%),
library_instructions 151,715,592 -> 150,868,905 (−846,687 / −0.5581%), summed
−1,946,968 (−0.5597%). compile_allocs 30,224 -> 30,207. The box's own box
projected −1,978,104 summed and CI read 0.9843 of it, the closest agreement of
the six folds so far. Runtime did not move: `work:success` on the same run,
runbench 2,003,021,871, identical to its golden. Floor 68.55 -> 68.56.
## 2026-09-13 — a qualified name is joined, not formatted

The loader mints a canonical name by putting a module's qualifier, a slash and
a declaration's name together, and fifteen sites in `src/lib.rs` did it with
`format!`. A `{}` on a `&str` is not free: it goes out through `Display::fmt`,
`Formatter::pad` and `write_str` into a string that starts empty and grows.
Callgrind on the module corpus puts 1,175 `format_inner` calls in the whole
compile and 712 of them inside `qualify`, at 611,799 instructions — 1.32% of
the compile term for a concatenation whose three lengths are known before a
byte is written.

`ast::qualified` does one exactly-sized allocation and three copies.
`bare_space`, which was the same shape with the bare-space mark in the middle,
joins it. Measured on the gate's own box with the environment emptied,
baseline binary against changed binary, all three corpora:

    module   46,330,414 ->  45,629,252   (-701,162,   -1.5134%)
    entry   153,908,707 -> 151,529,300   (-2,379,407, -1.5460%)
    library 155,032,113 -> 152,702,149   (-2,329,964, -1.5029%)
    summed  355,271,234 -> 349,860,701   (-5,410,533, -1.5229%)

Read twice on two builds, identical to the instruction. That is larger than
any of the six descent folds, and larger than the 1.32% `qualify` alone
accounts for: `open_qualified_doors` has 150 of the calls, and two lines in
the type loop were building the SAME string twice — once as the exports
table's key, once as the declaration's new name. The second is a move of the
first now.

**What this does not touch.** The strings both shapes produce are
byte-identical, so the whole error corpus, every differential sweep and every
golden but the instruction veins are unmoved. That is the property the
ratchet's new row watches: `the_qualified_name_goes_back_through_format`
writes the helper's body back to `format!` and the module row rises 690,734
(+1.5138%), the entry row 2,306,556 and the library row 2,273,585, with
nothing else moving at all.

**Where the rest of the compile's allocator time is.** The same profile puts
malloc and free at 6,855,502 instructions over 30,232 allocations, 14.8% of
the compile term at about 227 instructions an allocation. A third of those
allocations are vectors growing (`grow_one` 5,817, `do_reserve_and_handle`
4,730), and 3,642 of the second kind are `String as Write::write_str` — the
same formatting machinery this entry is about, reached from `format!` calls
elsewhere. The lexer's 4,407 are the `String`s the AST needs, which
kanso#1033 declined interning at 365 conversion sites. No other single owner
holds more than a per cent.

**The third sitting, on the merged base, and all three rows fall.** CI
re-measured on the kanso#1414 base:

    compile_instructions   43,910,543 ->  43,910,243    -300  -0.0007%
    entry_instructions    146,573,721 -> 146,570,800  -2,921  -0.0020%
    library_instructions  147,378,070 -> 147,374,531  -3,539  -0.0024%
    summed                337,862,334 -> 337,855,574  -6,760  -0.0020%

`compile_allocs` held at 28,361 again and `compile_peak_bytes` at 776,055,
the same pair of byte-identical rows as before. The reorder is three lines of
`src/runtime.c` and the front end does not run a line of it; these six
thousand instructions are the compiler's own bytes sitting differently.

Set the two sittings of the same change side by side and the paragraph above
is checked rather than asserted:

                            on kanso#1416     on kanso#1414
    compile_instructions           +883              -300
    entry_instructions           -2,236            -2,921
    library_instructions         -2,878            -3,539
    summed                       -4,231            -6,760

Same diff, same three lines, and the module row changes sign between one base
and the next while the other two keep theirs and grow by about a quarter.
CLAUDE.md's rule reads that a delta survives a change of base when the work
removed is a fixed count of operations, and does not when it is a share of a
pile something else just made smaller. Nothing here is a count of operations
at all: no decision was removed, so there is nothing to count, and every one
of these six numbers is an arrangement. An arrangement belongs to one binary.
kanso#1418 measured the other half of the same rule on the same day, where
`compile_allocs` fell by 424 against both of its bases because it counts
`HashSet` constructions that stopped happening.

So a layout row is projected from CI or not at all, and never from a
container whose glibc and clang the gate has already refused. The work vein
is where this change was aimed and where its win is; the compile veins are
regenerated, said out loud, and not banked against.

## the fourth sitting, and four bases give four answers

kanso#1418 landed and the branch merged it, so CI measured the same three
lines a fourth time:

    compile_instructions   42,877,925 ->  42,869,709   -8,216  -0.0192%
    entry_instructions    144,056,402 -> 144,034,290  -22,112  -0.0153%
    library_instructions  144,858,538 -> 144,834,698  -23,840  -0.0165%
    summed                331,792,865 -> 331,738,697  -54,168  -0.0163%

`compile_allocs` held at 27,937 and `compile_peak_bytes` at 776,055 for the
fourth time running.

Four sittings of one diff now sit side by side:

    base          module      entry     library      summed
    kanso#1415       +35     -1,854        +230      -1,589
    kanso#1416      +883     -2,236      -2,878      -4,231
    kanso#1414      -300     -2,921      -3,539      -6,760
    kanso#1418    -8,216    -22,112     -23,840     -54,168

The module row changes sign twice across the four. The fourth is eight times
the third and thirty-four times the first, and it is the only one where all
three fall together by a comparable share.

Nothing about the diff changed between them. It is three lines swapping two
arms of a tag switch in `src/runtime.c`, and `kanso check` never executes one
of them. What changed is the binary the rows were measured on: kanso#1418
rewrote the advisory fixpoint to revisit on demand, which moved the
compiler's own code and every address after it, so the reorder's embedded
bytes land somewhere else again.

The earlier sections of this entry read the sign disagreement as the tell
that a layout move is not a decision. Four sittings say something stronger
and worth writing down plainly: the SIZE carries no information either. A
layout delta is a property of one arrangement, and four bases are four
arrangements, so a reading from any of them predicts nothing about the next.

Which settles how to handle these rows, at the cost of six rounds on this
branch. A compile row on a runtime-only change is regenerated from whatever
base CI last ran, said out loud, and not reasoned from or banked against.
The objective cannot see it in any case: fifty-four thousand instructions on
three hundred and thirty-two million is a sixtieth of a per cent, and the
work vein -- runbench 2,000,261,871, indexbench 3,185,298, both CI's own --
is where this change was aimed and where its win is.

## the fifth sitting, on the base kanso#1419 left

kanso#1419 (ryu's pair loop) is chained ahead of this branch, so CI measured
the same three lines a fifth time, on the arrangement that change leaves:

    compile_instructions   42,872,197 ->  42,870,872   -1,325  -0.0031%
    entry_instructions    144,042,077 -> 144,037,300   -4,777  -0.0033%
    library_instructions  144,842,133 -> 144,837,840   -4,293  -0.0030%
    summed                331,756,407 -> 331,746,012  -10,395  -0.0031%

Against main's goldens, which kanso#1419 has not yet moved, the three rows
land at compile_instructions 42,870,872, entry_instructions 144,037,300 and
library_instructions 144,837,840: rises of 506, 1,351 and 1,615, all three
kanso#1419's layout and not this diff's. `compile_allocs` 27,937 and
`compile_peak_bytes` 776,055 for the fifth time.

The work vein reads the same two deltas it read on the kanso#1415 base, to
the instruction: runbench 1,997,551,401 -> 1,994,791,401 (-2,760,000 /
-0.1382%) and indexbench 3,265,296 -> 3,185,298 (-79,998 / -2.4500%). The
saving is a fixed cost per call and the call count did not move between
bases. Machine code likewise: the same eight rows +32 and four +48, text
1,735,036 -> 1,735,484 (+448), which against main's 1,734,364 lands at
1,735,484 with kanso#1419's 672 in front of it.

Welfare 68.75 -> 68.76, banked in the same commit.



**CI's rows.** The container's box reads about 0.8% high on these veins and the
projection was 5,410,533 summed; CI's sitting on the base kanso#1413 left reads:

    module    44,994,843 -> 44,301,309    -693,534 (-1.5414%)
    entry    150,030,446 -> 147,706,790  -2,323,656 (-1.5488%)
    library  150,868,905 -> 148,544,439  -2,324,466 (-1.5407%)
    summed   345,894,194 -> 340,552,538  -5,341,656 (-1.5443%)

compile_allocs 30,207 -> 29,169 (−1,038, −3.4363%) and compile_peak_bytes
777,126 -> 776,055 (−1,071, −0.1378%): an exactly-sized `String` holds no slack
where a grown one rounds up, and these names are live at the peak. The
container projected 776,055 for the peak and CI read the same number.

Runtime did not move: `work:success` on the same run, runbench 2,003,021,871,
identical to its golden. Floor 68.56 -> 68.64.

## 2026-09-13 — two tables clone names the program is holding open

A module reached by two import paths contributes its declarations twice, and
`collapse_diamonds` drops the second copy. It decides by building a key per
declaration — the canonical file id, the name, the arity, the line, the column,
whether the declaration is synthetic — and a key per type, and putting each into
a set. The name went in as a cloned `String`.

The key is read once and dropped. The clone was a heap allocation per
declaration and per type for a name the program holds open beside it, and the
only reason it was there is that `retain` needs the vector mutably: a set of
`&str` borrowed from the elements cannot live across the closure that removes
them.

`prune_unused_getters` answered that in kanso#1141's family and left the shape
written down — compute a keep mask under an immutable borrow, drop the borrow,
then `retain` over the mask. Both halves of the collapse do that now.

On the gate's own box (`library_box.sh`, environment emptied), the same tree
with and without the change:

    module    46,069,191 -> 45,848,443    -220,748 (-0.4792%)
    entry    153,041,019 -> 152,256,142   -784,877 (-0.5128%)
    library  153,410,944 -> 152,591,027   -819,917 (-0.5345%)
    summed   352,521,154 -> 350,695,612 -1,825,542 (-0.5178%)

Two builds of the changed source agree to 66, 52 and 52 instructions, against a
delta three thousand times larger. compile_allocs 30,207 -> 29,695 (−512,
−1.6949%), compile_alloc_bytes −15,280, compile_peak_bytes byte-identical at
777,126: the mask is a `Vec<bool>` the size of the declaration list, and it
costs one allocation where the clones cost 512.

**Where this came from.** A fresh callgrind profile of the compile on merged
main, read by self cost. The compile term is flat now — the allocator is 15.5%
spread over six symbols, hashbrown 13.7% over six more, infer 12% over four —
and the way in was to ask who the allocator's 26,487 callers are rather than
which function is hottest. `RawVecInner::finish_grow` holds 8,417 of them and
`lex_line` 8,223; the collapse's two `retain`s were fourth and seventh, at 379,531
and 154,704 instructions in two calls apiece. A count of calls found what a
count of instructions did not.

**The same question, a second table.** `fuse_enumerable` builds a set of
std/list's short names, asks it `contains` once per declaration, and drops it.
Nothing in it needed owning either. The set borrows now; `shorts` beside it
still owns, because that one outlives the borrow the rewrite mutates through --
the same constraint the keep mask answers above.

Both changes together, against the same baseline:

    module    46,069,191 -> 45,741,657    -327,534 (-0.7109%)
    entry    153,041,019 -> 151,895,480  -1,145,539 (-0.7485%)
    library  153,410,944 -> 152,285,923  -1,125,021 (-0.7333%)
    summed   352,521,154 -> 349,923,060  -2,598,094 (-0.7370%)

compile_allocs 30,207 -> 29,399 (−808, −2.6749%), alloc_bytes −21,315, peak
byte-identical. The two are very nearly additive: −220,748 and −104,581 apart
sum to −325,329 against −327,534 measured together, and the 2,205 between them
is the layout moving, not the changes interacting.

**The ratchet rows.** `the_dedup_keys_own_their_names` writes both key types back
to owned `String`s and leaves the keep mask alone, so the row watches the borrow
and not the shape around it. Under the mutation module rises 234,146 (+0.5107%),
entry 860,245 (+0.5650%), library 848,228 (+0.5559%) and compile_allocs 515. The
mutated tree reads slightly ABOVE the pre-change baseline — 46,082,589 against
46,069,191 — because the mask itself is not free; the change wins by removing
the clones, not by removing work the mask replaced.

`the_fused_name_set_owns_its_names` is its own row rather than a second case of
the first, because a second table can regress on its own: module +104,581
(+0.2281%), entry +339,005 (+0.2227%), library +369,885 (+0.2424%),
compile_allocs +296.

**CI's rows.** The container's box measured against the tree kanso#1413 left;
kanso#1415 landed first and took its own bite out of the same family, so CI
measured this change against a base that had already stopped cloning fifteen
qualified names. These are CI's numbers and they are what the goldens hold:

    module    44,301,309 -> 44,031,424    -269,885 (-0.6092%)
    entry    147,706,790 -> 146,767,592    -939,198 (-0.6358%)
    library  148,544,439 -> 147,572,025    -972,414 (-0.6546%)
    summed   340,552,538 -> 338,371,041  -2,181,497 (-0.6406%)

`compile_allocs` 29,169 -> 28,361, a fall of 808 — the container projected
exactly 808 and CI read exactly 808, because an allocation removed is an
allocation removed whatever the base. `compile_peak_bytes` byte-identical at
776,055, and `work:success` with runbench 2,003,021,871 unchanged.

The instruction rows are the ones that moved with the base: the container
projected −2,598,094 summed where CI read −2,181,497, so the projection ran
1.191x high. That is not this box reading the vein wrong. It is the same shape
as the allocation row's exactness read the other way round: an allocation is a
count and does not care what else was removed, where an instruction total is a
layout and does. Both PRs remove `String` clones from the loader, so the second
one lands on a smaller pile.

Floor banked 68.64 -> 68.68, and `all_pages.sh --write` rewrote the seven
`compiler.html` spans that quote these goldens.

## 2026-09-13 — the advisory union that had nothing to union, and the pre-size that cost more than it saved

`advisory::name_types` answers "which types can this name be" by unioning the
answer sets of every arm in the name's group. It started the union at nothing
and grew it: 829 of the compile's 1,283 hashbrown table growths came from that
one loop, 182,282 instructions on `kanso check lib/json`. Most groups hold ONE
declaration, and a union of one set is that set, so the arm's answer is cloned
straight back and the incremental insert path is skipped entirely.

Measured with the compile gate's own box, environment emptied, all three
corpora, baseline against changed:

    module   46,561,759 -> 46,447,127   −114,632 (−0.2462%)
    entry   154,656,197 -> 154,469,445  −186,752 (−0.1207%)
    library 155,779,162 -> 155,592,215  −186,947 (−0.1200%)
    summed                              −488,331 (−0.1368%)

**The obvious companion is a regression, and that is the third time.** Pre-size
the union — sum the arms' answer lengths, build the set with that capacity —
and the module row reads 46,628,372, which is +66,613 ABOVE the baseline. Both
changes together reach only −148,040, so the sizing walk costs about 340,000 of
the clone's 488,000 and then some. kanso#1157 declined pre-sizing six filtered
collects at 4,514 instructions and kanso#1159 declined the other direction; the
rule those three share is that a walk to measure a table costs more than the
rehash it saves, whenever the table is small and the walk is over cache-cold
slices. Only the clone ships.

**The wider lead is closed as diffuse.** `reserve_rehash` is 1,150,492
instructions, 2.50% of the compile term, and no owner holds a tenth of it: the
generic `insert` and `rustc_entry` nodes carry most of it, and under those it
splits across `qualify` (343,761), `Resolver::flush_unused` (192,797),
`bound_in_pattern` (171,421), `check_merged_after_aliases` (521,449 through
entry), `inline::aliases` (271,684), `infer::infer` (229,569) and a dozen more,
none above 0.75%. Fourteen `::default()` hash constructions survive in the
whole compile path — kanso#1158 already pre-sized the thirteen whose capacity
was knowable, and what is left grows across a recursive dependency walk whose
final size nothing knows before the loop that fills it. `name_types` was the
one piece with a shape that does not need a capacity at all.

**CI's rows, on the base kanso#1416 left.** The branch was re-cut onto merged
main after kanso#1416 landed, because its round-one rows were read against the
tree kanso#1415 left and that base is gone:

    compile_instructions   44,031,424 ->  43,910,543  -120,881  -0.2745%
    entry_instructions    146,767,592 -> 146,573,721  -193,871  -0.1321%
    library_instructions  147,572,025 -> 147,378,070  -193,955  -0.1314%
    summed                338,371,041 -> 337,862,334  -508,707  -0.1503%

All three fall together, and the entry and library rows track each other to 84
instructions — they run the same passes over corpora built to the same shape,
so a decision the front end stops making shows up in both at the same size.
`compile_allocs` held at 28,361 and `compile_peak_bytes` at 776,055: the clone
that went away was of a borrowed set, so no allocation site moved.

The summed figure barely shifted between bases. Round one read -510,720
(-0.1500%) against kanso#1415's tree and CI now reads -508,707 (-0.1503%)
against kanso#1416's — 2,013 instructions apart on half a million, and the
same percentage to three places. That is worth setting beside kanso#1416's
own reading, where the container's projection ran 1.191x high against CI on
the instruction rows. A delta survives a change of base when the work it
removes is a fixed count of operations; it does not when the work is a share
of a pile that something else has just made smaller.

Floor banked, welfare 68.68 held. `all_pages.sh --write` rewrote four
compiler.html lines quoting the three goldens.

## 2026-09-14 — the advisory fixpoint asked every declaration six times to learn 142 things

`advisory::return_type_names` decides, for every function, which record type
names its return value can carry. It is a monotone fixpoint, and it was
round-robin: ask every declaration in order, over and over, until a whole pass
changes nothing.

A probe on `bench/compile_corpus` says what that costs:

    decls=315  rounds=6  visits=1890  grew=142  nonempty=130

1,748 of the 1,890 visits — 92.5% — walked a body and learned nothing. A
declaration's answer can only change when an answer it reads has grown, and
the round-robin has no way to ask that question, so it asks all of them.

It is a worklist now. A body asks `name_types` about a fixed set of names —
the body, the groups and the type names never move — so the set of declaration
indices it reads is the same on every visit. The first pass records those
indices as it takes them, builds the reverse map, and after that a declaration
is re-asked only when one of the answers it read has grown.

## the measurement

Base is merged main at kanso#1416. Both binaries measured by callgrind in the
staged box, the compiler built from the same tree apart from this change:

    module     45,286,209 -> 44,049,155  -1,237,054  -2.7316%
    entry     149,978,957 -> 146,969,629  -3,009,328  -2.0065%
    library   151,165,544 -> 148,160,637  -3,004,907  -1.9878%
    summed    346,430,710 -> 339,179,421  -7,251,289  -2.0931%

These are container numbers and the compile gates refuse on this host, so CI
measures the rows that land. The ratio this box has projected at has run
between 0.84 and 1.19 of CI's over the last dozen changes, which is why the
projection is written down as a projection.

The ceiling was measured before the shape was chosen, by capping the loop at
one round and letting the answers be wrong: 45,612,584 -> 43,999,030 on the
pre-#1416 base, a fall of 1,613,554. The worklist takes about three quarters
of that. The rest is the first pass, which still visits everything and has to,
and the 142 revisits that are real.

## what a worklist can get wrong, and the fixture for it

Stopping early. So the fixture is four hops long and declared caller-first,
which is the worst order for the round-robin and the order most likely to
expose a re-queue that only walks forward: `relay` calls `hop_one` calls
`hop_two` calls `hop_three`, and only `hop_three` names `json/parse_failure`.
One round-robin pass carried the type one hop, so the advisory on `relay` was
the fifth thing to become true.

Watched red two ways before it was green. Dropping the re-queue entirely
turned it red AND took `leaky` with it, which says the break was too coarse to
prove the new fixture earns its place. Re-queueing only readers with a HIGHER
index — the plausible off-by-one — left all seven older advisory specs green
and turned exactly this one red. That is the spec doing work nothing else in
the tree was doing.

The assertion is the advisory a reader sees, not the round count and not the
visit count. Those are the decomposition, and the decomposition is the thing
that just moved.

## the answers are identical, and that was checked rather than argued

`kanso check` on both binaries over every `.kso` in the tree: 1,074 files, 0
diverging. Over every module directory under lib, the three corpora and
scripts: 45 modules, 0 diverging. A fixpoint's answer does not depend on the
order its queue is drained in — the union is monotone and the loop runs until
nothing grows — but the differential is cheap and the argument is not the
evidence.

`all_compile.sh` reports `emitted_code`, `compile_libraries` and
`compile_cost` AGREED; the other six gates refuse on this host. Nothing the
emitter writes changed, which is the expected shape: this pass produces
advisories and feeds no code.

## CI's four rows, and a container projection that ran 13% high

    compile_instructions   44,031,424 ->  42,908,199  -1,123,225  -2.5510%
    entry_instructions    146,767,592 -> 144,112,872  -2,654,720  -1.8087%
    library_instructions  147,572,025 -> 144,915,019  -2,657,006  -1.8004%
    summed                338,371,041 -> 331,936,090  -6,434,951  -1.9017%
    compile_allocs            28,361  ->     27,937         -424  -1.4950%

The container measured -7,251,289 summed on two builds from one tree. CI read
-6,434,951, which is 0.8874 of the projection. The compile gates refuse on this
host, and this is the second reading this week where the refusal moved a delta
rather than a level — kanso#1417 projected exactly half of CI's on both work
rows. A worklist's saving is rounds of walking that no longer happen, and how
much each walk costs is an inlining decision the two toolchains make
differently. Project the sign from a refused host; take the size from CI.

`compile_allocs` fell 424. `body_types` allocates a `HashSet` per visit, so
1,890 visits became 315 plus re-asks; against that the worklist keeps a reverse
read map and a queue, and the net is the 424.

Runtime did NOT move. `work:success` on the same run, runbench 2,003,021,871
and all thirteen other rows identical to their goldens, which is the shape a
front-end change should have: nothing this pass decides reaches the emitter.

## the base moved under all four rows, and the rule called both halves right

kanso#1414 landed after the first sitting. It removes the union-build from
`name_types` when a group has one arm, which is work every visit of this
fixpoint was doing, so the four rows measured against the old base were
deltas against a base that had since got cheaper on its own.

CLAUDE.md says which way that cuts, and the prediction went into the branch
before CI answered: the saving here is visits that no longer happen times
what a visit costs, kanso#1414 made a visit cost less, so the fall should
come back SMALLER. CI, on the merged base:

    compile_instructions   43,910,543 ->  42,877,925  -1,032,618  -2.3516%
    entry_instructions    146,573,721 -> 144,056,402  -2,517,319  -1.7174%
    library_instructions  147,378,070 -> 144,858,538  -2,519,532  -1.7096%
    summed                337,862,334 -> 331,792,865  -6,069,469  -1.7963%
    compile_allocs            28,361  ->     27,937         -424  -1.4950%

The summed fall came back 365,482 instructions smaller than the -6,434,951
read against the old base. That is the rule's first half.

The second half is the allocation row, and it is the more interesting one.
`compile_allocs` fell 424 against the old base and 424 against the new one --
the same number, to the allocation, across a change of base that moved every
instruction row beside it. An allocation is a count of operations. It does
not care what else got cheaper, because nothing kanso#1414 did removes a
`HashSet` this fixpoint builds; it only made the instructions around one
cheaper. The three instruction rows are a share of a pile, and a share
shrinks when the pile does.

So one run of one branch shows both halves of the rule, in the same table:
the counter that survives a change of base and the counters that do not,
told apart by what they count rather than by how they behaved.

The trend gate reads the four as improved and nothing as worsened.


## 2026-09-14 — five merged leads in a row, and not one of them named a ruling

CLAUDE.md's "Every unbuilt ruling stays in view" rule asks two things of cloud.
It may order the work however it likes. What it owes in exchange is a sentence:
the body of a pull request on a self-generated lead says which rulings it
weighed and why the lead came first.

Read off merged main on 2026-09-14, the five most recent leads carry no such
sentence. kanso#1413 (the box question joins the second fused descent),
kanso#1415 (a qualified name is joined), kanso#1416 (two tables clone names),
kanso#1414 (a union of one set) and kanso#1418 (the advisory fixpoint) run
from 22 to 102 lines of body apiece and none of them mentions the unbuilt
list, the ledger, or a ruling of any kind.

**The choices those five made were defensible, which is the point.** By the
time kanso#1413 opened, the effect type had merged (kanso#1372, 04:12Z), the
exhaustiveness rule had merged (kanso#1369, 05:16Z), and the book's live
remainder had merged (kanso#1412, 15:44Z). What was left on the list was the
pure-fallibility rider, blocked on a ruling nobody has made. There was nothing
buildable to weigh them against, and a lead was the right call five times
running.

So the missing sentence costs nothing here, and that is exactly when a habit
goes. The rule was written on 2026-09-09 because five rulings had stood
unbuilt through 296 merged pull requests, and what made that possible was
that no body ever had to say what it had read. A body that says "the list
holds one row and it is blocked" takes ten seconds and is checkable by anyone
reading the pull request afterwards. Nothing else in the machinery can tell
the difference between a lead chosen over a considered list and a lead taken
off the top of the last pull request's open items.

Recorded rather than filed as a question. The rule stands as written and
needs no ruling; this entry is the record that it went unobserved five times
in one day, so the next reader has a date to count from.

## 2026-09-14 — the floor entry leaves the ledger, every one of its three asks already answered

`design/pending-gavels.md` carried a second Blocking entry beside the
box-wrapping question: "The welfare floor cannot be staged from this session,
and two ruled builds wait on it". It asked Clay for one of five things. All of
them are settled, and it had been asking anyway.

**Its two builds merged.** kanso#1372 landed 2026-09-13T04:12Z and kanso#1369
at 05:16Z. The entry describes both as parked behind a permission.

**Its central ask is moot.** The entry wants a Bash permission rule for `git
add bench/welfare_floor.json`, refused seven times by the auto-mode
classifier. Clay ruled on 2026-09-13, verbatim: "you don't need to ask my
permission to lower the welfare floor if it is in service of making the
language actually work for the specification. this is an ironclad rule."
CLAUDE.md's welfare section carries it. The floor moved 67.77800065192253 ->
67.754 by hand on kanso#1369 and both branches went green.

**Its options 4 and 5 ask for a branch.** Both request "a third branch" —
option 4 for ledger-only edits, option 5 for kanso#487's fused descent. Clay's
answer to that shape of request is in CLAUDE.md under WHAT THE BRANCH RULE
ACTUALLY SAYS: "i can't believe you paused to ask me permission to make a
branch. Jesus Christ." Branches are free and were free when the entry was
written.

The entry's measurements are worth keeping and are kept: the ablation ceiling
for kanso#487's fusion (sixteen whole-program checks costing 36,348,088
instructions together, a bare no-work walk costing 1,147,185, so fifteen
fused descents are worth about 17.2M against a combined ask of 6,829,872) is
what the second fused descent has been landing against all week — kanso#1409,
kanso#1411 and kanso#1413 are that work. Nothing in the entry was wasted. It
simply stopped being a question and nobody removed it.

**How it survived.** Its own option 4 explains it: the entry lived on
`claude/go-to-town-m0dicm`, kanso#1369's build branch, because the ledger's
header says ledger edits ride small promptly-merged pull requests and the
entry was written from a branch that could not merge until the thing it asked
for was granted. So it reached main only when that branch merged — on
2026-09-13, the day its asks were answered. It was obsolete before it arrived.

**And it was miscounted.** This session told Clay twice that Blocking held
exactly one entry. That count came from grepping the headings of a working
tree checked out to an older branch, and was never re-run after the branch was
rebuilt from main. The ledger is one file with one canonical copy on
origin/main; a count taken anywhere else is a count of something else.


## 2026-09-14 — three ratchet rows proved nothing, and each one for its own reason

The ratchet reported `ten_walk`, `char_word` and `alloc_gate` BLIND on
kanso#1417. They are blind on `origin/main` at `15e1c3b7` with that branch
nowhere in the tree, so the reading is main's. Each was chased to an
artefact rather than argued from the equal counts.

Baseline runbench on the container, `env -i` under callgrind from one fixed
directory: **1,994,172,731**.

    mutation                                 runbench md5   runbench
    none                                     3b9af36f       1,994,172,731
    a_wide_character_copied_through_a_call   d1064916       1,994,172,731
    a_tenure_walk_asked_about_arena_pointers 3b9af36f       1,994,172,731

**`alloc_gate` is blind by construction, and has been since kanso#1393 and
kanso#1396.** `scripts/gates/instructions.sh` copies `./runbench` — the plain
binary. `build_benchmarks.sh` builds the counting set first under `--counters`,
moves it aside, and builds the plain set second, so the binary the gate
measures has `K_COUNTING` at 0. Both the guarded form and the mutant's
two-branch form sit inside `if (__builtin_expect(K_COUNTING && ...), 0)` and
compile to nothing there. The counting binary keeps both, and both count
identically, so no allocation counter can see it either. What kanso#1298 won
is no longer in the artefact it was won on; there is nothing left to protect.

**`ten_walk`'s function is not in the linked binary.** `nm runbench` lists
`k_ten_holds_outside` and no `k_ten_holds`. The mutation edits the one-line
body of `k_ten_holds`, which the release link does not emit, so the mutated
compiler — a different binary, md5 `7d005e1c` against `28f31106` — produces a
byte-identical `runbench`. A mutation that cannot change the bytes cannot
redden a gate over them.

**`char_word`'s arm is never executed.** The equal instruction count on a
binary that genuinely differs is suggestive and not proof, so the copy was
poisoned instead of slowed: `memcpy(os->data, "ZZZZ", 4)` in place of the
character's own bytes. runbench's output is byte-identical either way
(`e8e74ccb` both ways) on a binary whose md5 is `33402193`. The wide arm of
`k_b_at` is not reached by the run program at all. CI's own gate agrees from
the other side — it diffs all fourteen rows and stayed green under the
mutation, so no benchmark reaches it.

The three rows and their mutations are removed. `instructions.sh` keeps its
other rows, so the job stays covered and the coverage check still passes.

What this leaves open: the run corpus indexes no wide character anywhere. That
is a gap in the corpus rather than in the ratchet, and closing it means adding
to the run program, which moves every golden and the welfare floor. Recorded
here rather than done alongside a row removal.

A CORRECTION to my own first reading of the same nightly, before it reached a
commit: I took its two baseline objections as one and wrote that an UNPROVEN
gate ends the pass. It does not. `told p r true` builds its finding with
`ok = true`, `unmutated` exits only on findings where `not f.ok`, and
`kept_provable` already drops the rows sharing an unanswerable gate and carries
the rest into proving. The comment above `answerable?` says exactly this and I
had read past it. So the 2026-09-13 run exited on the ALREADY RED site gate
alone, and once kanso#1403's blob setup does its job the nightly reaches the
mutation phase on any runner — marking the instructions-gate rows unproven
where the silicon does not match, and proving everything else.
## 2026-09-14 — to_float called a truncated significand certain, and it is off by one ULP

`k_b_to_float` takes the Eisel-Lemire path when the scan sees a clean
number. The scan keeps nineteen significant digits and drops the rest,
and it handed the dropped ones to Lemire without saying they were gone.
Lemire's method answers exactly or declines, and what it decides is a
rounding boundary: a significand that has lost its twentieth digit can
sit on the far side of that boundary from the number the program wrote.
When it does, native returns the neighbouring double and the interpreter
returns the right one.

Lemire's own implementation carries a `truncated` flag for this, and so
does fast_float. This one did not.

The scan now sets `cut` when it drops a nonzero digit, and the fast path
is taken only while `cut` is clear. A dropped ZERO is not a truncation in
value — `10000000000000000000` is exactly the nineteen-digit `w` times
ten — so a trailing run of zeros still takes the fast path. That
distinction is what keeps the run corpus where it was: all 210,177 float
parses on runbench go through Eisel-Lemire before and after, because none
of them drops a nonzero digit.

The differential harness in the parse direction, against `strtod`,
reported 221 mismatches on the old scanner and 0 on the new one over
23,264,660 cases. The smallest is twenty significant digits:
`44090656.994409065` parsed 44090656.99440906 and should parse
44090656.99440907.

`tests/golden/micro/a_long_significand_rounds_like_the_oracle.kso` is the
fixture, six cases from twenty digits to thirty-six plus the trailing-zero
case that must stay on the fast path. Every line in it was a divergence
before the fix; `micro_corpus_agrees_across_engines` runs it on both
engines. Watched red on a binary built from the unfixed source first.

Row `truncated_significand`, mutation
`a_truncated_significand_taken_as_certain`, which removes the `!cut`
guard.

CORRECTION. This entry first said "all twelve cost goldens and the lazy
tier agree, so the fix is free on the counters this repo watches". The
first clause is true and the second does not follow, and CI refuted it:
five veins moved. `all_counters.sh` reads the twelve RUNTIME COST
goldens and the lazy tier, and those did all agree. The work vein, the
text vein and the three compile veins are read by other gates, the sweep
never touched them, and "the counters this repo watches" is the wider
set. Do not read a green sweep as a silent tree.

CI's sitting. Six of the fourteen work rows rise and the other eight are
byte-identical, and the six are exactly the programs that link
`k_b_to_float`. Each landed at:

    work_runbench      2,003,021,871 -> 2,003,046,621   +24,750  +0.0012%
    work_jsonbench     1,250,438,261 -> 1,250,475,761   +37,500  +0.0030%
    work_widebench        33,078,691 ->    33,142,691   +64,000  +0.1935%
    work_encodebench   3,641,023,306 -> 3,641,023,556      +250  +0.0000%
    work_livebench     3,115,992,526 -> 3,115,992,776      +250  +0.0000%
    work_oneshot          19,292,672 ->    19,292,922      +250  +0.0013%

and the summed text vein with them, 1,734,268 -> 1,734,364 (+96): the
same six .text rows rise 16 bytes apiece and the other eight hold. The
96 is 6 x 16, which is the check that the two veins agree about which
programs the change reached.

widebench is the largest share because it is the smallest of the six and
parses the most floats per instruction; runbench carries the largest
absolute rise and the smallest fraction. That is what a guard flag costs
on a path a benchmark reaches a few hundred thousand times, and it buys
a correctly-rounded double where the two engines used to disagree.

The three compile rows all FELL: compile_instructions 42,877,925 ->
42,870,366 (-7,559), entry 144,056,402 -> 144,035,949 (-20,453),
library 144,858,538 -> 144,836,225 (-22,313), with compile_allocs and
compile_peak_bytes byte-identical. `src/runtime.c` is `include_str!`'d
into the compiler at src/main.rs:826, so its bytes are bytes the
compiler carries and a change to it moves the layout underneath. The
front end does no less work than it did; the fall is layout and is
recorded as such, not claimed as a compile-side win.

Welfare 68.73238080266131 -> 68.73254693776617, banked in this PR. The
compile fall outweighs the runtime rise, so the objective came out ahead
and the floor is raised rather than lowered. The differential-law
exception was not needed here.


## 2026-09-14 — the append is already in place everywhere, and the counter that said otherwise counts two constructors

The standing lead off `encode_onto` was "a 32-byte arena conversion at 0.61%".
Re-attributing it on merged main (15e1c3b7) refuted the lead and produced two
wrong answers on the way, both recorded here because the second was caught
only by going back for the call sites.

**It is not readable from a profile.** `callgrind_annotate` on runbench —
1,994,172,731 instructions, `env -i`, run from the repository root — has no row
for `k_bytes_owned` and none for `k_alloc`. Both inline into every caller,
which is what kanso#1221 and kanso#1298 were for. Only the outlined append
family shows: `k_b_append_rendered` 23,466,064 (1.18%), `k_b_append_grow`
19,358,910 (0.97%), `k_b_append_slice` 9,141,444 (0.46%), `k_b_append_range`
2,000,394 (0.10%), two more under a thousandth, summing to 54,010,642 (2.71%).
So the 0.61% was not read off a self row, because there is none.

**THE FINDING: every emitted append in the run program already mutates in
place.** Counting call sites in `runbench.ll` rather than reasoning about the
analysis:

    k_b_append_mut_byte     18   mutate = 1
    k_b_append_mut           2   mutate = 1
    k_b_append_rendered      2   both with the literal i64 1
    k_b_append_byte          0
    k_b_append               1   inside the k_b_append_byte SHIM, not program code
    k_b_append_slice         1   inside the k_b_append_slice_fast SHIM

Twenty-two emitted sites, and the only two that pass a non-mutating flag are
the slow-path tails inside the runtime's own inline shims — and
`k_b_append_byte`, the shim holding one of them, has no call sites at all. The
uniqueness analysis is not failing on this workload. **`src/linear.rs` is
refuted as a lead for the run program**, and the round that would have widened
its Perceus fixpoint, with the differential sweep an aliasing argument owes,
would have bought nothing.

**The wrong answer that got there: a counter with two increment sites.**
`k_stat_sh_bytes` reads 41,290,272, and `sizeof(KBytes)` is three words, so
41,290,272 / 24 = 1,720,428 exactly — a clean division, which is precisely what
made it convincing. It was written down as `k_bytes_owned`'s call count, and
against `append_fast` 8,834,013 that gave a fast path splitting 80.5% mutating
against 19.5% allocating, with a ceiling of 1.04–1.38% of runbench.

Every one of those numbers is withdrawn. `k_stat_sh_bytes` is incremented at
TWO sites: `k_bytes_owned` (runtime.c:7965) and `k_bytes_view`
(runtime.c:6828), the borrowed-view constructor that builds the same 24-byte
header with `cap = 0`. 1,720,428 is the two summed. Nothing separates them —
`k_stat_view_allocs` is bumped at runtime.c:6489, in neither of them, so it is
not the split either. The header-allocation count is UNKNOWN, and with it the
ceiling.

An exact division is not a check. It follows from the two sites sharing one
`sizeof`, so it would have held however the calls divided between them.

**What to carry forward.** The append path is done: it mutates in place at
every site the run program emits, and no lead survives there. Anyone returning
to `k_bytes_owned` needs a counter of its own first — the existing one cannot
answer the question, and a second constructor is exactly what a shared counter
hides.

THE SHAPE TO CARRY. `callgrind_annotate` ships with valgrind and was not being
used; the hand-written parser that once read 99,188,064,506 against a
1,994,172,731 program was solving a problem the tool already solves. And a
function with no row has inlined, which is information rather than a reason to
hunt for the cost elsewhere. Both of those held. What did not hold was reading
one counter as one call site: `grep -n` for the counter name before dividing by
anything, and count the emitted call sites before reasoning about the pass that
decides them.

## 2026-09-14 — ryū's pair loop is already the right shape, and the two reformulations that measured faster had each introduced a divide

Three shapes, all measured against kanso#1419's head (runbench 1,988,868,701).

The lead came off the pair loop in `render_ryu`. It searches with `vp` and `vm`
and brings `vr` down on the same trip, so `vr`'s division does work the search
will throw away on every trip but the last. Hoisting it out should cost one
division instead of `pairs` of them.

Two ways to hoist it, and a third written afterwards:

    table    vr /= RYU_POW100[pairs]           1,987,944,191   -924,510   -0.0465%
    switch   nine arms, literal divisors       1,987,371,341  -1,497,360   -0.0753%
    loop     divide by the literal 100         1,993,136,111  +4,267,410   +0.2146%

The first two look like wins and are not. The base divides by the literal
`100`, and LLVM turns a literal divisor into a multiply-high — three of them a
trip, which is what the comment above that loop has said since kanso#1260.
`RYU_POW100[pairs]` is a runtime value, so it compiles to `div %r8`. Counted in
the linked runbench binary, `render_ryu` holds 0 divides on the base and 2
under each of the table and switch shapes. A standalone translation unit at
-O2 reads 0, 2 and 4 for the same three: it agrees with the binary on the base
and on the switch and counts two extra under the table, which is the inliner
seeing a different call graph. Either way the direction is the same and the
base is the shape with none.

**The switch was written to dodge exactly this and did not.** Nine arms, each
with its own literal divisor, is nine multiply-highs — until LLVM tail-merges
the arms back into one divide with a phi'd divisor, which is the same runtime
value by another route. Writing the constants out does not survive the
optimiser. Only a construction with no runtime divisor anywhere denies it the
merge, which is what the third shape is: a loop dividing by the literal `100`.
That one has no divide at all and costs 22.3 instructions a call, because it
walks `vr` down in its own loop instead of riding the search's trips.

DECLINED, all three. The base is the shape that already has no divide in it.

**What this says about the vein.** Callgrind scores `div r64` as one
instruction. On the silicon this project publishes numbers for it is 20 to 40
cycles against a multiply's three. welfare's run term is an instruction count,
so the objective would have scored the table and the switch as wins, the floor
would have ratcheted up on them, and the published decode board would have
moved the wrong way — on a change that is slower everywhere it runs.

This is the second time the queue has been misled by the distance between what
the vein counts and what the hardware does, and it is the opposite direction
from the first. `2026-09-13 (fifteenth) — the instruction vein counts a byte of
memcpy as an instruction` found the vein OVERcounting something cheap, at one
Ir per byte moved by `rep movsb`, which makes a memcpy-shaped lead look bigger
than it is. This one is the vein UNDERcounting something expensive, which makes
a divide-shaped change look like a win. Both are the same gap read from
different ends, and the rule that falls out of the pair is narrow enough to be
useful: before believing an instruction delta, check whether the diff moved any
instruction whose cost and whose count disagree. `div`, `rep`-prefixed string
moves, and the divisions LLVM has already turned into multiplies are the three
this repo has hit.

OPEN, and not a gavel: whether welfare's run term should weigh a `div` at more
than one is a question about the weights, and nothing here moves the floor in
either direction, so there is nothing to rule yet. Recording it so the next
change that trades a multiply-high for a divide meets this entry before it
meets CI.

**The differential.** Each shape was checked against the base byte-for-byte
before being priced: 23,264,660 cases for the loop shape, structured (2048
exponents x 64 mantissas x both signs, the powers of ten from -320 to 308 with
their `nextafter` neighbours, a million integers and their /7 and x1e-9) plus
uniform random over the bit space, 0 mismatches and 0 round-trip failures
through `strtod`. The round-trip check runs on non-negatives only:
`render_ryu` never writes the sign, its caller does (runtime.c:4304), so
feeding it a negative renders the magnitude and `strtod` reads back a positive.
That guard was missing at first and reported 25,115,977 failures, none of them
real.

**And the wider census, which is the reason the rule is forward-looking.**
Counting `div`/`idiv` sites across all fourteen benchmark binaries on the same
build: eleven hold exactly one, in `k_exec`, which is process plumbing and runs
once. The other three — encodebench, widebench and basket — hold five, the same
one in `k_exec` plus two each in `k_div` and `k_mod`. Those two are the outlined
helpers kanso#1292 minted when it sent integer quotient and remainder through a
call with zero and -1 handled there, and a language whose `/` and `%` divide has
to divide somewhere.

So no hot path in the shipped runtime executes a divide today, and none of the
297 changes that have landed traded a multiply-high for one. The rule this
entry leaves behind has no current violations to repair; it exists to catch the
next change that would have been the first.


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

## 2026-09-16 — A folder handed to `fold` is never called by name, and a
## walk that found no call site answered yes (DONE)

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

## 2026-09-16 — The lazy-verdict ratchet row proved nothing, and scan_counters
## had no row at all (DONE)

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
