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
