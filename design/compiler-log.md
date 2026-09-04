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

## 2026-09-03 — THE SIXTH CHECK RESTING ON A MENTION, AND IT WAS THE ONE THAT CITED THE OTHER FIVE

**DONE.** Writing `tests/the_ratchet_carries_what_its_gates_need.rs` earlier today
produced a first draft that could not fail: it asked whether
`scripts/ratchet/toolchain.sh` *contained the string* `valgrind`, and the
script's own header, which explains why valgrind is there, names it four times.
That is the kanso#1137 family. Having walked into it, the obvious next question
was whether anything already in the tree had the same shape.

One did, and its header cites kanso#1137 by name:

```rust
// tests/the_objective_does_not_weigh_machine_code_size.rs
assert!(gate.contains("bench/text_golden.txt"), "…no longer diffs …, so nothing
        counts what the compiler emits…");
```

`scripts/gates/machine_code.sh` names the golden three times: the host check at
line 12, the diff at line 32, and one line of its `::error::` message at 37.
Replace the first two with nothing and the gate reads no golden at all — and the
spec passes, certifying the exact state its own failure text describes.
Measured, not reasoned:

```
--- remaining mentions:
37:  echo "::error::bench/text_golden.txt. Allocation counters cannot"
test the_machine_code_vein_is_still_gated ... ok
```

Both halves read operative lines now — comments dropped, and lines whose whole
job is to print a diagnostic dropped with them — and a third test holds the
counterexample so the file cannot drift back. Watched red on the gutted gate and
green on the restored one.

**Reading lines is still not running the gate**, and the entry says so where a
reader will find it: the behavioural proof is the ratchet's `machine_code` row,
which applies a defect nightly and refuses a gate that stayed green. The spec is
the cheap per-pull-request half of the same split the ratchet itself documents.

**The sweep found no seventh.** Every test that reads a repository file was
checked for a positive `contains` against prose-bearing text. Three others take
that shape and all three read data rather than source: `bench/welfare_floor.json`
(no comments), emitted LLVM IR, and a hako lock file. The rest assert on a
tool's output, which is where a spec should be entering anyway.

**The pattern worth carrying.** Both instances today were written by somebody
who knew the rule — this file's own header argues it, and the toolchain spec was
drafted an hour after reading it. Knowing the rule does not catch the case;
trying to make the check fail does. The cost of that attempt is one `sed` and
one `cargo test`.

---

## 2026-09-03 — THE COMPILE ROW MOVED 992 AND THE FRONT END DID NOT

**DONE.** CI refused the branch on the vein that is keyed by silicon:

```
the work the FRONT END does changed on family0x19-model0x1:
bench/compile_instructions_by_cpu.txt says 41631006 and this run counted 41631998.
The row is keyed by silicon, so the runner is not the answer here:
the same family and model counted both numbers.
```

`compile_instructions` **41,631,006 -> 41,631,998, +992 (+0.0024%)**, and the
cause is written in that file's own header from earlier the same day:
`src/main.rs` embeds the runtime with `include_str!("runtime.c")`, so a change
to the runtime changes the compiler binary even when it changes no compiler
code. The wrong-arity reorder added a few lines of C; the embedded source grew,
the rodata after it shifted, and the front end does the same work at different
addresses. `compile_allocs`, `compile_peak_bytes` and every fixpoint counter are
byte-identical, which is what says the work did not change.

The Zen 3 row (0x19/0x1) this run landed on is re-sat. The Intel (0x6/0xcf) and
Zen 4 (0x19/0x11) rows are removed rather than carried forward, on the rule
already recorded there: a value measured against the old binary is worse than no
value. They are re-sittings when they next refuse, and each costs one red run.

**This is the second time in one day this file has been emptied for the same
mechanism**, and the price is now visible rather than argued: a runtime change
of any size costs one red CI run per chip in the pool before the vein reads
clean again. Whether the vein should measure the front end against a binary
that embeds the runtime is a real question and it is not this change's to
answer; what is clear is that `include_str!` makes "the front end moved" and
"the runtime moved" indistinguishable to this row, and only the other three
compile counters can tell them apart.

Welfare is 76.01 before and after — 992 parts in 41.6 million is below the
hundredth the score is rounded to.

---

## 2026-09-03 — THE REPORT NAMED THE GATE AND NOT ITS REASON

**DONE.** Naming the gate in `ALREADY RED` answered the question it was built
for on its first CI run — `sh scripts/gates/instructions.sh`, red because
valgrind was not on the ratchet's box — and could not answer the second. With
valgrind installed by `scripts/ratchet/toolchain.sh` the same gate is **still
red on the baseline**, and a line that says only which gate leaves a reader
holding hypotheses with no way to choose between them. The gate wrote down why
it failed; the ratchet was throwing that away.

`ALREADY RED` and `UNBUILT` now carry the last eight lines the gate printed,
indented under the finding. Watched red on the fixture that commits a tracked
python file: without the gate's words the refusal does not contain
`crept_in.py`, and the spec says so.

**One hypothesis is already dead.** The obvious candidate was the worktree: the
baseline builds in `/tmp/kanso-ratchet-base` against a shared target symlink,
and paths baked into a binary would move an instruction count the way a run id
that gained a digit does. Built both ways and compared:

```
8a406faa2e27030c41bc8dd12ab9750fe5ddb408afe54cedd52ce66f5cd475d3  ./jsonbench
8a406faa2e27030c41bc8dd12ab9750fe5ddb408afe54cedd52ce66f5cd475d3  /tmp/kanso-pathtest/jsonbench
```

Byte-identical. The benchmark does not depend on the directory it was built in,
so whatever reddens that gate on the ratchet's runner is something else.

**OPEN, and the next run answers it.** The live candidate is the one recorded
two entries ago: `bench/instructions_golden.txt` is not keyed per silicon the
way `compile_instructions` is, and the ratchet job lands on whatever CPU the
pool gives it — a second, independent sitting of those eleven rows on a runner
unrelated to the cost-goldens job's. If the gate's own words turn out to be a
row diff, that settles it and the vein needs the treatment kanso#1226 gave the
compile row. If they are something else entirely, they will say so, which is
the whole point of printing them.

---

## 2026-09-03 — TWO SITTINGS OF THE SAME ELEVEN ROWS IN ONE COMMIT, AND THEY DISAGREE

**MEASURED.** Printing the gate's own words paid on its first run. The
baseline's refusal now reads:

```
ALREADY RED cost goldens (deterministic ratchet, no clocks)
  gate: sh scripts/gates/instructions.sh
  red before any mutation, so no row sharing it is proof
    digestbench 81252330
    the work the benchmarks do changed. A rise is a regression to explain…
```

`digestbench 81252330`. The cost-goldens job, **on the same commit**, measured
`81252316`. Fourteen instructions apart, two jobs, two runners
(1000054847 and 1000054838), one image and one glibc.

**This is the first time the eleven rows have been sat twice in one commit.**
Every earlier reading came from one job on one runner per run, so the vein has
only ever been compared against itself across commits — where a chip change and
a code change are the same event. The log has carried the question since
2026-09-01: *"These rows claim to be exact and the pool is not… nothing here
measures that fraction."* This measures it, at n=1: two runners, one row, +14.

**Two hypotheses are dead.** The benchmark binary is byte-identical between the
repo root and a worktree built against a shared target symlink
(`7779456c957c6cff…` both), and running the same binary from those two
directories gives the same count to the instruction (81,251,917 twice, in this
container). So neither the build directory nor the run directory moves it.

**What is NOT decided, and deliberately.** One row on one pair of runners is a
data point, not a design. The options are a golden per chip (the treatment
kanso#1226 gave the compile row, at eleven rows times the pool instead of one),
declaring the two callgrind gates unprovable by the ratchet (two rows blind by
declaration, which is what the ratchet exists to prevent), or something that
compares the mutated measurement against the baseline's own rather than against
a golden. Choosing between them on one number is the shape of reasoning this log
keeps catching, and the pending-gavels ledger is explicitly not for
implementation questions — this one is the file-holder's, answered here.

So the evidence gathers instead: the captured tail goes from eight lines to
twenty-four, which is enough for the cpu line `dispatch.sh name` prints and the
whole eleven-row diff. Every runtime-touching pull request from here is another
paired sitting, attributable to its silicon, at no extra cost. The decision gets
made on a table rather than on a pair.

---

## 2026-09-03 — A FOURTH SHAPE: REPORTED AND NOT CREDITED

**DONE, and it corrects the entry above.** That entry listed three ways out of
the paired-sitting problem — a golden per chip, declaring the two callgrind
gates unprovable, or comparing the mutation against the baseline's own
measurement — and said choosing between them on one data point was premature.
It was, and it still is. What the entry missed is that none of them has to be
chosen, because the question it was answering was the wrong one.

The ratchet's baseline asks *is this gate red for a reason other than the
mutation*. For `sh scripts/gates/instructions.sh` on a foreign runner the answer
is yes, and that is a fact about the ROW rather than about the branch. So the
row is **reported and not credited**: the baseline prints `UNPROVEN THIS RUN`
with the gate's own words and does not fail, and the proving pass drops every
row sharing that gate instead of applying a mutation to a gate already red.

**That is the opposite of the blindness kanso#1228 was built to catch, and the
difference is one word.** Before, a red gate was silently counted as PROOF. Now
it is silently counted as nothing, and says so out loud. On a run that lands on
the golden's silicon the row is proved normally, so this costs coverage only on
the runs where coverage was never available.

**The danger is the list, not the mechanism** — an entry excusing a gate that is
not silicon-bound turns a real failure into a note. So the list is pinned to a
property of the gates rather than to anyone's judgement:
`tests/a_host_bound_gate_is_reported_not_credited.rs` requires `host_bound` to
be exactly the set of gate scripts that invoke callgrind on an operative line. A
new callgrind gate left undeclared turns it red; a declared gate that runs none
turns it red too.

**That spec's own first draft could not fail, which makes three today.** It read
the `bound` BINDINGS rather than the `host_bound` LIST, so removing an entry
from the list left it green — the binding was still there and the list it was
absent from was never opened. Watched red the second time, and the failure names
both sides:

```
  left: ["sh scripts/gates/instructions.sh"]
 right: ["sh scripts/gates/compile_instructions.sh", "sh scripts/gates/instructions.sh"]
```

Three checks in one day whose first draft was satisfiable without the property
holding. The common thread is that each read something ADJACENT to the thing it
meant to assert — a comment beside an install, an error message beside a diff, a
binding beside a list. Trying to make the check fail is what found all three, and
it cost one `sed` each.

---

## 2026-09-03 — THE SKIP I ADDED AN HOUR AGO HAD A FALSE GREEN IN IT

**DONE.** `kept_provable` drops every row sharing a host-bound gate, and the
proving pass then runs the rest. When the rest is empty — a branch whose whole
selection is host-bound — the loop had nothing to do and the closing line said

```
ratchet: 0 rows
ratchet: every row turned its gate red
```

which congratulates a run that proved nothing. That is the exact shape the pass
exists to refuse, reintroduced by the fix for it, and it survived because the
`told` arms dispatch on whether any row FAILED and no arm asked whether any row
RAN.

The empty case says so now and does not fail, because no diff could have proved
those rows on this runner:

```
ratchet: no row on this runner could be proved; none was claimed
```

**Found by reading the summary path rather than by a spec**, and the reason
there is no fixture is worth stating: constructing one needs a scope whose every
selected row is host-bound, and `asked?` selects by job name, so the cheapest
such scope drags in seventeen rows and their builds. The behavioural proof is
the nightly. What is pinned per-pull-request is the list
(`tests/a_host_bound_gate_is_reported_not_credited.rs`), which is where a
mistake is actually likely.

**Four in one day now**, and the pattern has stopped being a coincidence: the
toolchain spec read a comment beside an install, the machine-code spec read an
error message beside a diff, the host-bound spec read a binding beside a list,
and this one read a failure count beside a run count. Every one of them was
adjacent to the property and satisfiable without it.

---

## 2026-09-03 — A NUMBER'S BYTES WERE WALKED TWICE

**SHIPPED.** `lib/json` read every number twice. `number_end` walked the bytes
asking where the number stopped, and `mark_from?` walked the same bytes again
asking whether a `.`, `e` or `E` had gone past, because the answer decides
`to_int` against `to_float`. The scan carries the mark as a boolean argument
now, so one walk answers both questions.

```
jsonbench   2,533,092,019 -> 2,428,220,306    -104,871,713, or 4.14%
```

Measured in the container, three runs, same digits. The runner's own rows are
what land in `bench/instructions_golden.txt`; this is the size of the move, not
the number to paste.

**The profile says exactly where it went.** `value_for` was the largest symbol
in the decode at 648,032,400, and it was a merged one — clang had inlined the
whole value-parsing path into it, the number scanner included. It reads
239,779,050 now, and the surviving scanner stands as its own symbol at
272,697,300. 648.0 - 239.8 - 272.7 leaves 135.5M, and the total fell 104.9M:
the difference is the arms the merged function no longer carries.

**What it cost.** The front end visits 17,169 expressions on `lib/json` where it
visited 16,806, a rise of 2.2%, and the emitted decoder gains 89 lines and 20
calls while losing one define. Two small walkers became one larger one with an
extra argument. `.text` FALLS 48 bytes on jsonbench and on oneshot, which is the
second scanner leaving. No allocation counter moves at all — the same slice, the
same `to_int` or `to_float`, per number — which is why this is a change the cost
goldens could not have seen and the instructions vein could.

**The counters, by name, and where they landed.** `front_end_visits` 16,806 ->
17,169. `emitted_calls` 1,808 -> 1,828 and `emitted_lines` 12,044 -> 12,133 for
the decoder; `emitted_other_calls` 14,532 -> 14,552 and `emitted_other_lines`
87,826 -> 87,915 for the eight beside it, all of that move being oneshot, which
imports the same library. `emitted_defines` and `emitted_other_defines` each
fall by one, and `text` falls 96 bytes over the eleven programs. Every one of
those five rises is the same fact: one scanner with an extra argument and more
arms, where there used to be two with none.

**The mark rides as an argument rather than in a record beside the end
position.** A record would have been an allocation per number: 4,217 of them per
decode, 632,550 over the benchmark, and that is a term welfare weighs where
instructions on this path are one it weighs more lightly. The boolean costs
nothing and the arms read the same.

**Two new arms had no coverage at all.** `e` and `E` never appeared in
`lib/json/json_test.kso` before today — the float tests were all `3.25` and
`2.5`, so the exponent forms went through a scanner nobody had exercised.
`test_decode_exponent`, `test_decode_exponent_upper`,
`test_decode_exponent_signed` and `test_decode_negative` are new, and each was
watched red first: dropping the `101` arm's `true` reds `test_decode_exponent`
with `invalid number` at position 1, and deleting the `69` arm reds
`test_decode_exponent_upper` with `unexpected trailing characters` at 2. The
third falsifier — a `digit_step` that drops the mark — the LANGUAGE refuses:
`marked` becomes an unused binding and the compiler will not build it.

**OPEN.** `bench/widebench/widebench/` and `bench/encodebench/encodebench/`
carry their own copies of the json library, and they differ from `lib/json`
already — widebench's has a `pretty.kso` the shipped library does not. They are
frozen fixtures rather than stale copies, and nothing in the tree says so. They
are left alone here; whether a benchmark that vendors a library should track it
is a question worth asking once rather than per change.

---

## 2026-09-03 — DECLINED: THE ESCAPED-STRING TAIL DOES NOT WANT A RUN SCAN

**DECLINED by measurement, +1.16%.** A clean json string is found with one
`find2` and copied with one slice. A string with an escape in it is not: past
the first backslash, `str_chars` walks the rest one byte at a time, through a
dispatch and a one-byte append each. Making the tail scan runs the way the head
does — `find2` to the next quote or backslash, then append the run in one
copy — reads like the obvious fix.

```
jsonbench   2,533,092,019 -> 2,562,563,906    +29,472,300, or 1.16%
```

**The distribution is why.** In `bench/large.json`, 1,773 of 11,057 strings
carry an escape, and they hold 4,562 runs between them totalling 16,895 bytes:
a mean run of **3.7 bytes**, with 1,029 of the runs empty because escapes come
back to back. The per-run fixed cost measured about 300 instructions — 113 in
the scanner's own glue, 75 in `find2`, 46 in `k_b_slice`, 49 in the wide append
— against the 50 instructions a byte the walk costs. The run has to be six bytes
before it breaks even and it is under four.

**A fused `append(acc, slice(cs, a, b))` does not rescue it.** `k_b_utf8_slice`
already exists for the same shape one line above, so the pattern is available;
it would take back the 46 instructions the slice allocation costs, which leaves
the change roughly 60M worse than doing nothing.

**What the probe found instead is worth keeping.** Two programs, a clean string
and one with a single leading escape, at 2,000 / 4,000 / 8,000 bytes:

```
clean    213,329   264,566   366,562     25.6 instructions a byte
escaped  316,640   469,253   773,152     76.0 instructions a byte
```

Linear in both, so there is no quadratic hiding here. But a byte in an escaped
string's tail costs about **50 instructions more** than the same byte in a clean
one, and that is the language's per-byte dispatch-and-append, not anything
`lib/json` chose. The disassembly of `str_char` is 41 instructions round the
loop: six to index, ten to box the byte and unbox it again for the dispatch,
eight to dispatch, thirteen for the in-place append including two loads of
`k_stats_on`, four for bookkeeping. Whoever wants this path faster should go
after those ten, not after the number of walks.

---

## 2026-09-03 — A BYTE SWITCH THAT REBUILT THE BYTE BEFORE IT LOOKED AT IT

**SHIPPED, and it is the largest single runtime move this log holds.** A group
whose arms discriminate on a byte read out of a byte string crosses the call
boundary as a raw `i64` — the byte, or 256 for the `none` a read past the end
answers. `rebox_params` then rebuilt a `KValue` from that raw value, and the
dispatch tree pulled the rebuilt struct apart again: tag out, compare to 0,
payload out, switch. The comment above the reconstruction said the round trip
"folds back into a raw switch". **It does not.** `str_char`'s loop:

```
5132:  cmp    $0x4,%rax
5136:  cmove  %rbp,%rcx        ; 256 for none, on the caller's side
513c:  cmp    $0x100,%rcx
5143:  sete   %dl
5146:  cmove  %r13,%rcx
514a:  shl    $0x2,%edx
514d:  test   %edx,%edx        ; the tag test the tree wrote
```

Seven instructions a byte to take apart a value that was never assembled, and
every byte-dispatching function in the decoder paid it. The tree switches on
`%xNr` directly now, with 256 as the `none` case.

```
jsonbench    2,533,092,019 -> 2,098,859,754   -17.1424%
oneshot         34,322,446 ->     31,427,168    -8.4355%
widebench       61,890,181 ->     59,506,049    -3.8522%
encodebench  5,848,702,451 ->  5,846,994,368    -0.0292%
indexbench       5,242,363 ->      5,241,950    -0.0079%
basket          40,300,172 ->     40,299,759    -0.0010%
deepbench      676,465,730 ->    676,462,050    -0.0005%
digestbench     81,252,316 ->     81,251,917    -0.0005%
escapebench    130,170,751 ->    130,170,352    -0.0003%
pendbench      715,732,938 ->    715,732,552    -0.0001%
scanbench    1,423,437,576 ->  1,423,437,163    -0.0000%
```

Those are the CONTAINER's, which is where the ratios in this entry come from
because a ratio needs both ends measured on one box. The runner's rows are the
ones the golden takes, and it reads the seven small ones as not moving at all;
the paragraph below has them.

**Nothing rises.** The jsonbench figure carries the single-pass number scan
above it as well; this change is 13.56% of it on its own, 2,428,220,306 to
2,098,859,754. **widebench is the clean attribution**: it vendors its own copy
of the json library, frozen, so the 3.85% there is the dispatch and nothing
else. The seven programs that do not dispatch on bytes move by four hundred
instructions or fewer, which is the compiler emitting a slightly different
module and the linker laying it out differently.

**The counters, against the branch point rather than against the entry above
it, because that is what the trend gate reads:** `emitted_calls` 1,808 ->
1,820, `emitted_branches` 1,210 -> 1,186, `emitted_lines` 12,044 -> 12,053,
`emitted_other_calls` 14,532 -> 14,526, `emitted_other_branches` 8,749 ->
8,673, `emitted_other_lines` 87,826 -> 87,659, `emitted_defines` 169 -> 168 and
`emitted_other_defines` 1,469 -> 1,468. Calls and lines rise because the number
scan above bought them; the two changes pull those two counters in opposite
directions and the scan pulls harder. **Branches only fall**, in the decoder
and in the eight beside it, and that is this change alone: nothing about the
number scan removes a branch. `text` falls 8,080 bytes over the eleven
programs — 1,936 on jsonbench, 2,080 on encodebench, 1,888 on oneshot and 2,080
on widebench, and not a byte on the other seven. Rounds and
visits on `lib/json` do not move at all, which is the check that this changed
what the backend writes rather than what the front end decides. No allocation
counter moves.

**One literal had to be guarded, and the fixture found it before CI did.** 256
is the sentinel, so a program that writes `fn kind 256` — an arm no byte can
ever reach — would send every read past the end of a byte string to that arm.
The boxed tree is immune because it tests the tag before it looks at the
payload. The divergence was real and I watched it:

```
--- native      --- interpreter
bracket         bracket
quote           quote
a byte that cannot be    some other byte
```

A group with any int literal outside 0..255 stays on the boxed path, where the
arm stays as dead as the oracle says it is.
`tests/a_byte_arm_no_byte_can_reach.rs` pins both halves — the impossible
literal and an ordinary byte group beside it, so a change that disabled the
fast path everywhere would not read as a pass. Watched red against the
unguarded draft, with the two engines' answers printed side by side.

**Why the emitted-line count is the presence counter here.** There is no
observable output that distinguishes a raw switch from a boxed one; what
distinguishes them is `emitted_branches`, which falls by 24 in the decoder and
76 across the eight beside it because the tag test and the none test are gone
from every byte-dispatching call. Revert the change and that counter goes straight back
up, which is what the vein is for.

**What CI's own sitting says, and what it cost the compiler.** The runner reads
the four moved rows 413 or 399 above the container and reads the other seven
EXACTLY where they were: jsonbench 2,098,860,167, oneshot 31,427,567, widebench
59,506,462, encodebench 5,846,994,767, and no movement at all in basket,
deepbench, escapebench, pendbench, indexbench, scanbench or digestbench. The
container's few-hundred-instruction falls on those seven were the container.

**The compiler pays, and the first attribution I wrote for it was wrong.**
Three compile counters rise — `compile_instructions` 41,631,998 -> 41,831,767
(+0.48%), `compile_peak_bytes` 713,606 -> 715,275 (+0.23%), `compile_allocs`
25,394 -> 25,485 (+0.36%) — and I priced all three against the raw byte switch,
because it was the larger change and the rises arrived with it. **`kanso check`
never runs the backend.** It lexes, parses, infers, runs provenance and the
advisories, and stops; a codegen change cannot reach those rows except through
the binary's layout. Every one of them belongs to the json library's extra arms.

Held rather than reasoned, because a reason that sounds right is what produced
the wrong version. With the codegen change reverted and lib/json untouched, the
container reads `compile_allocs=25485` and `compile_peak_bytes=715275` — the
same two numbers as the branch head, to the byte. And the instruction row,
three builds under the same tunables:

```
main                                 42,032,508
the library change alone             42,238,115   +205,607
the library change and the switch    42,235,790     -2,325
```

The switch gives 2,325 BACK, inside the layout band the by-cpu file documents in
thousands. Rounds hold at 40. The Zen 4 row is removed rather than carried
forward, per that file's rule about values measured against an old binary.
**Welfare 76.0100 -> 76.1700**: the objective takes the trade, and compile cost
satiates at 0.5 against runtime's 2.0, which is exactly the asymmetry it was
weighted for.

**The published scoreboard is dated now, and says which way it is stale.** The
per-decode floors in §08 — kanso 0.87 ms against serde_json's 0.90, naive rust's
1.04 and go's 2.05 — come from seven interleaved rounds on 2026-08-07, by the
slope method, and nothing on the page said so. They are re-sat at a release and
not on demand, because randomised-layout timing puts the spread within one tree
at about three per cent and this box is not idle. So the caption carries the
date, and a paragraph under the block says the decoder has moved a long way
since and names these two changes as 17.14% of it. Publishing a number I cannot
honestly re-measure would be worse than publishing a stale one that says it is
stale.

The lesson is the one this log keeps relearning from a different direction. Two
changes shipped together, one large and one small, and every unexplained number
attached itself to the large one. What separated them was not an argument about
mechanisms but two rebuilds and four counters.

**And the landing page was quoting a sitting nobody had re-read.** The
number-bearing surfaces are a checklist rather than a memory, so walking it
found `docs/index.html`'s receipts panel carrying `reasonably-written rust 1.02`
and `go encoding/json 1.95` where §08 reads 1.04 and 2.05. They are the
2026-07-27 figures. #756 replaced that panel's kanso and serde rows with a
pointer to the live board and left the two rows underneath alone, so the site has
disagreed with itself about two of its four lanes for five weeks with every gate
green. Both rows now read the 2026-08-07 sitting and the caption names its date,
the way §08's does. Nothing checks this: the two pages hold the numbers as prose
and `golden_prose` only reads what carries a `data-golden` attribute, which these
cannot, because a hand-sat wall clock has no golden to read.

**A chip the pool had never shown.** CI's compile-instructions gate went red
on `family0x6-model0xcf` — Emerald Rapids, absent from the four the by-cpu
file's header lists — with the refusal that an unrecorded chip is an unsat row.
It read 41,832,275 where Zen 3 read 41,831,767 on the same binary, sha
55fb850296d1 printed on both. 508 apart, against the 5,124 the header
decomposes between two other chips on one binary. The ifunc effect is a
property of the pair, not a constant, which is a second argument for the key
over a band: a band wide enough for 5,124 hides every front-end move this vein
has caught, and one narrow enough for 508 would still have refused this run.

**A comment claimed a property the machine code contradicted, and nothing in
the tree could see it.** That is the same family as #1137's four pins that
rested on prose — except that one was a spec reading a comment, and this was a
comment asserting an optimiser outcome. The optimiser is entitled to change its
mind between releases; a claim about what it will do belongs in a counter.

## 2026-09-03 — EIGHT BOOK PANELS QUOTED A LIBRARY NOBODY WAS CHECKING THEM AGAINST

The landing page says every code panel in the book is executed against the real
toolchain before it may appear. Ten were not, and four of those had drifted so
far they printed code the language cannot compile.

**How the gap is shaped.** `scripts/book_panels` regenerates a panel by
resolving its title to a sample it owns: `<samples>/<title>`, or a bare
filename one directory deeper. The deeper branch compares a path's LAST SEGMENT
to the whole title, so a title carrying a slash — `lib/json/number.kso` — can
never match, and the harness leaves it alone by design: "a name that is nowhere
is left alone: the panel may be quoting something this does not own." A
directory-module title falls in the same hole from the other side, because
`literal.kso` names `samples/ch08/literal/` rather than a file. Eight ch08
panels quote `lib/json`, two quote directory modules, and nothing read any of
them.

**What was in them.** The `lib/json/text.kso` panels wrote `at cs n` for the
index; there is no `at` under any spelling, so that panel has been printing a
name error for as long as `cs[n]` has been the syntax. They also wrote
`concat [] (slice ...)` where the library writes `append (bytes "")`. The
`scan.kso` panel named `_is_ws` for a function renamed `ws?`. The `value.kso`
panel named three byte-array constants and a `_word` that sliced the tail out
and compared arrays — the library replaced all of it with `rue?`, `alse?` and
`ull?` comparing bytes where they sit. The two directory-module panels still
showed a `pub play` and a `told`/`chosen` reporting pair from before those
samples were split into `main`/`lit`/`report`.

**Why the check is structural.** These panels are excerpts with editorial
changes: a private declaration gains a leading `_`, a module qualifier is
dropped, a field gains a `:type` the file leaves bare. So the text cannot be
compared and the panel cannot be regenerated. Two properties survive every one
of those changes:

  every declaration the panel shows is declared in the file it names, and
  every name the panel uses is a name that file can reach.

`scripts/book_quotes` checks both. The reachable set is read off the package
rather than listed: its own declarations and bindings, the tail of every
qualified name it calls, and every bare name its own code uses. So a panel may
name what the file could name, and there is no table of builtins here to go
stale in its turn — which is the failure mode a hand-written list would have
reproduced one level up.

It found eleven things across four panels and nothing else, and it is precise
in the direction that matters: run against the fixed book it is silent, and
renaming `ws?` in the library reddens it in two lines. That rename is the
ratchet mutation.

**And then the gap closed, because the claim about it was wrong twice.** The
first version of this entry said the three remaining panels each want a
different resolution rule, and that ch09's `vse/methods.kso` names a file in
the vse repository. Both are wrong. Two rules reach all three: ch10's titles
are written relative to `docs/book`, and ch07's and ch09's relative to a
sample directory one level in — `samples/ch07/teahouse/` holds `main.kso`
beside a package dir `teahouse/`, and `samples/ch09/vse/` the same. ch09's
`vse/` is a sample package in this repository.

Both rules are in, and they found four more panels in two more chapters:

- ch07's `teahouse/menu.kso` showed `fn describe (err reason)` — an arm naming
  its own package's err. The language refuses that and the sample dropped it.
  **The chapter contradicted itself on one page**: the panel three inches below
  demonstrates the replacement, `testing/when_failed pocky (r -> ...)`, and
  only that one was machine-checked, because book_panels owns it.
- ch07's `teahouse/menu_test.kso` showed a hand-written `err?` predicate where
  the file imports `std/testing` and calls `testing/failed?`.
- ch10's `pingpong.kso` wrote `even`/`odd` for the file's `even?`/`odd?`, and
  wrapped the statements in a `main =` the file does not have.
- ch10's `classify.kso` had the same `main` wrapper.

Fourteen panels across four chapters, on a page that says every one of them is
executed against the real toolchain before it may appear.

**ch09 is clean, and that was verified rather than assumed.** Six panels there
quote `vse/`, and silence from a checker is worth nothing until you have seen
it speak: inserting a declaration no file has into the ch09 panel reddens the
gate in two lines. So the silence is coverage.

**One false-positive class had to go first.** `true`, `false` and `none` are
literals the grammar provides, and the reachable set is read off the package's
own text, so a small file that happens never to write `false` was forbidding a
panel from showing an arm that answers it. They are always reachable now. That
is the only exception, and it is a list of spellings rather than of names —
which is what keeps it from becoming the builtin table this deliberately does
not have.

A title that resolves to a FILE beside its chapter stays skipped for a
different reason: book_panels owns it, and reading ch07's `shop.kso` panel
against its `shop/` directory is how this first went wrong.

**And the prose went with them.** Four paragraphs described the old
implementations — the byte-array constants, the four hex digits read inline
before `_str_hex4` existed, the sample's helpers as "verbatim from
number.kso" when they are the shape the library left behind. Nothing checks
prose against code and nothing here proposes to; what the gate buys is that
the CODE beside the prose can no longer drift silently, which is what made the
prose wrong.

## 2026-09-03 — a recorded chip disagreeing with itself, and a row nobody reads

Two findings on `bench/compile_instructions_by_cpu.txt`, both from one
branch whose diff was `design/compiler-log.md` and its archive.

**The third cause. OPEN.** `scripts/gates/compile_instructions.sh` prints a
`compile_sample cpu= sha= row=` line on every run for one purpose, and its
comment names it: *"same cpu and same sha with different rows would mean
something is moving that neither the key nor the diff can see."*

| when | key | binary sha | counted |
| --- | --- | --- | ---: |
| 12:33 | family0x6-model0xcf | 55fb850296d1 | 41,832,275 |
| 13:05 | family0x6-model0xcf | 55fb850296d1 | 41,831,767 |

508 apart, same key, same binary, and no front-end change in the diff. So the
key does not determine the row, and the premise the per-chip table rests on is
weaker than it claimed when #1226 built it.

The row moved to 41,831,767, which three readings say — Zen 3, Zen 4, and this
chip at 13:05 — against one that says 41,832,275. **That is a choice under
uncertainty and not a measurement.** The honest state is two readings of one
key on one binary, differing, and the golden's own note says so in those words
rather than dressing the move up as a re-sitting.

What it wants is a sitting: several runs on one Emerald Rapids runner, to see
whether the value is bimodal there the way it was across the two AMD models
before the tunables were pinned. If it is, the missing term is something the
pinned tunables do not cover, and the first candidates are the ones the file's
header already names as excluded from the key — cache sizes and derived
thresholds — reaching the row by a path other than the malloc and string
tunables the gate sets.

**A second row for one chip. DONE.** Correcting that row, I edited it in place
AND appended it, and every gate in the tree stayed green over a file holding
two `family0x6-model0xcf` lines. The lookup is
`awk '$1 == k { print $2; exit }'`: the first match answers and every later row
is dead. So the file had one authority and one decoration, and the decoration
sat at the bottom where the newest sitting goes, which is where a reader is
most likely to trust it.

`compile_ir_row.sh` refuses a doubled key now, before the lookup rather than
after, so a reader is told about the duplicate instead of being sent to re-sit
a front end that did not move — which would have written a third row into a
file that already had one too many. Five refusals where there were four.

Three specs, each watched red on the unfixed script: a duplicate that AGREES
with itself is still refused, because the defect is the second line rather than
its value; a duplicate whose second row is what the run counted is not reported
as a moved front end; and a duplicate on a chip this run did not land on is
refused too, since otherwise the file stays broken for every run but the
unlucky one. New ratchet row `compile_ir_doubled` with a mutation that deletes
the block and reddens exactly those three.

The gap was cheap to find only because a mistake walked into it. Nothing else
in the tree reads this file, so nothing else could have.

## 2026-09-03 (later) — the chip was not the answer, and a refusal nobody could act on

Two corrections to the entry above it, one of them to a claim it made.

**The 508 is glibc's allocator, and the cpu is ruled out. CORRECTS the entry
above.** That entry left the thread OPEN and proposed cache sizes and derived
thresholds as the candidates. That guess is wrong and the measurement was
available the whole time.

Both runs printed their full 123-line cpu feature block — no `bench/dispatch.txt`
is recorded, and `dispatch.sh name` dumps the block in CI while none is. The two
blocks are byte-identical: every feature word, the stepping, and
`data_cache_size` among them. Both ran glibc 2.39-0ubuntu8.8, callgrind 3.22.0
and rustc 1.98.0, on binary sha 55fb850296d1.

The same two job logs carry the profiles. Every kanso symbol agrees to the
instruction across the pair — `eval_expr'2` 1,633,593 both times, `check_merged`
1,589,240, `lex_line` 866,486, `parse` 589,004. Three rows differ:

| | 12:33 | 13:05 | |
| --- | ---: | ---: | ---: |
| `_int_malloc` | 1,551,384 | 1,551,964 | +580 |
| `_int_free` | 1,522,333 | 1,522,352 | +19 |
| `memcmp-avx2-movbe` | 1,353,408 | 1,353,342 | −66 |

All three are glibc. The front end executed the identical instruction sequence
and the allocator did not, which is the same shape the by-cpu file's header
records for the 5,064 cluster the tunables were pinned to remove: `_int_malloc`
and `memcmp` moving together, an alignment difference downstream of a heap
layout difference. Pinning the tunables shrank it from 5,064 to 508 and did not
remove it.

Five consecutive runs in a container on one binary read 42,235,790 every time,
so the count is deterministic per host and the 508 is not run-to-run jitter.

**Still OPEN**, with a narrower question: what moves the heap layout when the
binary, the cpu features, glibc, valgrind and the environment all agree? "Is it
the cpu" is answered, and the answer is no.

**A refusal nobody could act on. DONE.** The runner's rustc moved 1.98.0 ->
1.98.1 and all three compile veins refused at once. The refusal is right — the
rows belong to the toolchain that measured them. What follows it was not.

Every compile gate called `measured_on.sh` under `set -e`, so the mismatch
stopped the gate before it measured anything, and each then printed that
refusal's standing advice: *let CI measure it and copy the rows out of the job
log.* There were no rows in the job log. The only host allowed to produce them
is the one that just stopped, so a branch in this state could not be brought to
green by anybody — not by CI, which stopped, and not from a container, which
may never record its own numbers.

`scripts/gates/host_gate.sh` answers two questions where `measured_on` answered
one. A container may neither compare nor measure: its numbers going into a
golden over the runner's is the accident `measured_on` exists after, so it stops
and prints nothing. CI may not compare and MUST measure, because CI's sitting is
the only one that may ever be recorded — it measures, prints the rows under
`::error::`, and still fails. `dispatch.sh` already draws this line the same way
and for the same reason.

`compile_instructions` additionally does not read its row against any recorded
chip on such a host, since a row from an unnamed host has no chip to be read
against.

Four specs, both directions watched red: with the CI arm removed the run that
must measure stops, and with it always taken the container that must stop
measures. Ratchet row `host_measures`.

The three goldens are NOT re-sat here. This makes CI able to report the sitting
they need, which was the missing step; the numbers follow from the next run.

## 2026-09-03 (later still) — the rustc bump moved nothing, and it cost three gates

CI re-sat all three compile veins under rustc 1.98.1 on family0x6-model0xcf,
binary sha de5bfab22fbd, using the measure-and-refuse path built in the entry
above. Every number came back identical to the one recorded under 1.98.0:

| | 1.98.0 | 1.98.1 |
| --- | ---: | ---: |
| compile_allocs | 25,485 | 25,485 |
| compile_peak_bytes | 715,275 | 715,275 |
| compile_instructions | 41,831,767 | 41,831,767 |

So the four measured-on lines move and nothing else does. Welfare is unmoved
and is not re-set.

**One datum against the granularity these files pin.** `measured_on.sh`'s own
header says of rustc: *"the upstream version only, for the same reason clang
carries no package revision: nothing here shows a point release moving a count,
and pinning tighter than the evidence reds the gate on changes that are not
changes."* A patch bump that reddened three gates and moved no number is
exactly that shape, and glibc is the contrast the same header draws — its
Ubuntu revision is pinned because 2.39-0ubuntu8.7 against 8.8 was SHOWN to move
about four hundred instructions.

The pin is not loosened on one observation. Recorded so the next point release
is a second datum rather than a fresh surprise, and if it also moves nothing
the argument for reading rustc as major.minor is made on two.

**The two rows nobody re-sat are carried, and the file says so.** Zen 3 and Zen
4 hold 41,831,767 from 1.98.0. kanso#1230 removed a row rather than carry one
across a toolchain move; the difference here is that a measurement exists. The
toolchain changes the binary identically for every chip and the one chip re-sat
moved by nothing, so deleting two correct values to force sittings that would
reproduce them buys nothing. The assumption is written into the file and is
self-correcting: a carried row that is wrong goes red the first time CI lands
on that chip, which is the signal a missing row would have given one run later.

## 2026-09-03 (fourth) — the carry was wrong one run later, and the row is bimodal

**CORRECTS the entry above on two counts.** That entry said the rustc point
release "moved nothing" and defended carrying two chip rows across it. CI
answered both within ten minutes.

**The carry. DONE, the other way.** `bench/compile_instructions_by_cpu.txt`
already carried the rule — *a value measured against the old binary is worse
than no value* — and the entry above overrode it with an argument: a
measurement existed, the toolchain changes the binary identically for every
chip, and deleting correct values to force sittings that would reproduce them
buys nothing. CI landed on Zen 3 one run later and read 41,832,275 against the
carried 41,831,767. The rule was right and the argument was wrong. Zen 3 is
recorded from CI's own reading; **Zen 4 is removed**, because it has still
never been measured on this binary and carrying it a second time would be the
same mistake with the same argument.

**The row is bimodal, and it is CLAY'S CALL.** Four readings, tunables pinned:

| when | key | binary sha | rustc | counted |
| --- | --- | --- | --- | ---: |
| 12:33 | family0x6-model0xcf | 55fb850296d1 | 1.98.0 | 41,832,275 |
| 13:05 | family0x6-model0xcf | 55fb850296d1 | 1.98.0 | 41,831,767 |
| 16:25 | family0x6-model0xcf | de5bfab22fbd | 1.98.1 | 41,831,767 |
| 16:35 | family0x19-model0x1 | de5bfab22fbd | 1.98.1 | 41,832,275 |

Two values 508 apart. One chip produced both on one binary; two chips produced
different values on one binary. Neither the key nor the binary picks a mode,
so the 2026-09-02 ruling this file was built on — *the row moves with the CPU,
so record it* — rests on a premise that does not hold. The file's own header
had set out what these runs would test, and this is the branch it named: the
chips still disagree, so pinning was not the whole explanation.

An exact row is red about half the time on a chip that produces both modes.
That is a design question rather than a number to re-sit, and it is filed in
design/pending-gavels.md with four options and a recommendation. **It blocks
kanso#1232 and every branch after it**, because no value in that file makes CI
reliably green until it is ruled.

So the sitting's honest reading is narrower than the entry above claimed: the
allocation and peak counters reproduced exactly across the toolchain bump, on
every host that has run them, and the instruction count cannot be said to have
moved or held, because it has two values and the bump is not separable from
the bimodality. compiler.html §40 is corrected to match.

## 2026-09-03 (fifth) — the bimodality reaches the trend gate

Setting the bare `compile_instructions=` to the other mode made the trend gate
refuse the branch, in its own words:

    worsened: compile_instructions 41,831,767 -> 41,832,275
    FAIL  a pure regression: something got worse and nothing got better.

Right by its rules, and the claim is false — nothing in that branch touches the
front end. The number moved because the reference row moved from one mode to
the other.

**So the bimodality is not only a flaky row.** The first row of
`bench/compile_instructions_by_cpu.txt` is also the bare number welfare,
golden_prose and the trend gate read, so a mode flip there presents as a
counter that worsened with nothing traded. The objective's own regression
detector fires on noise, and what stands between it and a false regression is
which chip happens to be listed first.

The reference is family0x6-model0xcf, which has three readings against Zen 3's
one, and whose 41,831,767 is what main already carries. Zen 3 keeps its own
measured row below it. **The uncomfortable half is stated in the file**: this
ordering is also the one that makes CI green, and choosing a reference to quiet
a gate is exactly what that file exists to catch. It is not that move, because
the alternative asserts something untrue — but the reason has to be written
down rather than assumed, since the two are indistinguishable from the diff.

Added to the ledger entry as the strongest argument for ruling this rather than
living with it.

## 2026-09-03 (sixth) — the fifth reading, and three chips is enough

CI landed on Zen 4 — the row removed an hour earlier for never having been
measured on this binary — and read 41,831,767, the low mode, on sha
de5bfab22fbd. Recorded, because an unrecorded chip is exactly what the gate
asks for and because it is a measurement rather than a mode being chased.

|  | family0x6-model0xcf | family0x19-model0x1 | family0x19-model0x11 |
| --- | --- | --- | --- |
| on sha de5bfab22fbd | 41,831,767 | 41,832,275 | 41,831,767 |
| earlier, sha 55fb850296d1 | both | — | — |

Three chips, two values, one binary across all three. Intel reads low, Zen 3
reads high, Zen 4 reads low — and Intel read high as well, twenty minutes
before it read low, on one binary with byte-identical CPU feature blocks.

**So the chip does not select the mode**, and that is the whole premise
`bench/compile_instructions_by_cpu.txt` is named for. Two chips agreeing is
what the file already warns is not evidence they agree; three chips split two
against one, with the odd one out having previously read the other value, is
evidence of something else entirely.

Every row is CI's own most recent reading now and nothing is predicted from
another chip. Whether that is a stable arrangement or a coin flip per run is
the question in design/pending-gavels.md, unchanged by this reading except
that it is now five measurements rather than four.

## 2026-09-03 (seventh) — two of three chips have produced both values

A run against the fully recorded table — all three chips present, every row
from CI's own sitting — counted 41,831,767 and refused. The only row not
already holding that value is Zen 3, which read 41,832,275 eighteen minutes
earlier on the same binary.

| chip | on sha de5bfab22fbd |
| --- | --- |
| family0x6-model0xcf | both values, twelve minutes apart |
| family0x19-model0x1 | both values, eighteen minutes apart |
| family0x19-model0x11 | the low value once |

**So the mode is not a property of the silicon.** Two of three chips have
produced both, on one binary, with byte-identical CPU feature blocks where
those were compared. The modes are GLOBAL and the key
`bench/compile_instructions_by_cpu.txt` is built on separates nothing.

That simplifies what is filed rather than complicating it: what wants
recording is two acceptable values for the vein, not two per chip. The
ledger entry says so now.

**The rows are not flipped to the value just seen**, and this is the entry
that has to say why, because six readings in one afternoon is exactly the
pressure under which a session starts chasing. Setting each row to whatever
CI last read would make the gate green and would delete the finding, and the
finding is the only thing here worth having. They stay as measured.

Six readings is enough to decide on, so further ones go into the ledger's
table rather than earning entries here.

## 2026-09-03 (eighth) — what a failing test is allowed to tell you

`design/pending-gavels.md` has carried the assert hako since 2026-08-17 with
the gate lifted and a recommendation to build it. Built, and the measurement
that motivates it was taken first rather than assumed.

**What a failing `==` says.** A test is a constant and `==` is the assertion,
which decides pass or fail and nothing else:

    test_a_failure_on_purpose = decode "42" == 43
    test_a_failure_on_purpose ... FAILED (returned false)

The name, and no sign of what it got. The operands are gone by the time the
runner holds the boolean, so the runner cannot recover them — whatever carries
them has to be the assertion.

**The concept was already there.** The runner prints the value a test
returned, so a matcher answering a RECORD reports its own diff with no change
to the runner at all. The same wrong assertion, both ways:

    test_the_old_way ... FAILED (returned false)
    test_the_new_way ... FAILED (returned expect/mismatch 43 42)

That is the mushroom test passing rather than a feature being added. `lib/expect`
is `expect`, `to`, `equal` and `be_true` — about twenty lines, no builtin, no
runner change, and `expect` is the identity because the chain is the surface.

**Kept out of `lib/testing` deliberately.** That hako's header says "nothing
here adds a second way to write a test that can already be written", and a
matcher IS a second spelling of `==`. The distinction that earns it a package
of its own is that it is the only way to write a test that reports what it got;
the stance in `lib/testing` stays intact and the addition is opt-in.

**Watched red, and two of the four mutations the LANGUAGE refused.** Swapping
the mismatch record's fields reddens the two specs that assert its shape;
making the match arm answer a record instead of `true` reddens the two that
assert a match. The other two — a mismatch answering `false`, and a generic
first arm — do not compile: `error[unused]` on the now-dead bindings, and the
most-specific-first ordering rule. A gate the compiler enforces needs no spec,
and that is worth recording rather than counting as coverage.

**The surface shape is still Clay's**, and the ledger entry stays until he
rules on it. What has changed is that it is now a thing to read rather than a
thing to imagine, and the evidence for wanting it is a measurement.

## 2026-08-31 — rider: pure fallibility is boxed too

Clay raised `foo["bar"]!` against the effects-are-types gavel: an err
with no io underneath — value or effect? A crash-semantics reading
(insistence violated = defect, halt) was proposed and DECLINED by
Clay on the shipped mechanism: "you just get an error back and you
can handle it however you want and it bubbles up. so maybe it should
be an effect after all... since that's the only way to enforce the
bubbling." That entailment is the ruling:

- **Any operation whose answer includes an err yields `<t>effect`,
  io or not.** The box is defined by fallibility — the unresolved
  outcome — and io was never the criterion. One failure system.
- `foo["bar"]` stays the data form: the value or `none`, absence as
  ordinary data, no box (and post-`done`, `none` is unambiguous).
- `foo["bar"]!` yields `<v>effect`; the suffix-grammar contract ("a
  `!` function's answer typeset includes an err") re-reads as: a `!`
  operation answers a box.
- The bang family stays rescuable under the standing license — the
  map-collision reasoning holds (the err is raised in std, std is
  foreign to every caller). Nothing supersedes.
- Enforcement is why: under explicit elimination the box is the only
  carrier that makes bubbling mandatory. A bare err would need the
  retired railway; a box cannot be dropped or mistaken for a value.

Separately proposed and AWAITING GAVEL: the fused chain operators
(`.>` bind, `.!` annotate, `.?` rescue — reviving the archived `.>`
of 2026-08-16), bare-function right-hand sides, sole spelling in
chain position with the words remaining prefix functions. Not ruled;
recorded so the proposal is not re-derived.

## 2026-08-31 — gavel: the fused chain operators

Clay: "the fused operators are a Go." The three combinators gain
fused chain spellings — the chain dot plus one character of channel:

    config = io/read_file path
      .> json/parse                      # chain through bind
      .! (e -> "config: {e.reason}")     # chain through annotate
      .? when_failed                     # chain through rescue

- Each is pure sugar with a fixed desugaring: `x .> f` IS `bind x f`,
  `.!` annotate, `.?` rescue. Semantics, licenses, and the auto-
  rewrap live at the words; the parser learns three operators, not
  three meanings. `.>` revives the archived spelling of 2026-08-16.
- The right-hand side is a bare function — a lambda, a named
  function, or a dispatch group — no wrapper lambda for the common
  case: `.> json/parse`, `.? when_failed`.
- **In chain position the fused operators are the only spelling.**
  `. bind (f)` retires as a chain form, superseding the 2026-08-29
  keep-the-dot ruling for the three combinators specifically; plain
  `.` application chaining is untouched. The words remain ordinary
  prefix functions everywhere else (`rescue (foo["bar"]!) handler`),
  so each position has exactly one spelling.
- All three land together, per the no-yagni-in-language-design rule:
  a chain that can `.>` but must fall back to a word for annotate
  would be the inconsistency this family exists to remove.
- Pure fallibility benefits identically:
  `foo["bar"]! .? (e -> "anonymous")` is one-line handling with no
  ceremony, per the boxed-fallibility rider above.

## 2026-08-31 — directive: the welfare chart replays the current formula

Clay, seeing the #184 re-scoring as a cliff in the trend chart: the
graph should be continuous. The history rows store raw counters per
merge, so the chart's welfare line is to be RECOMPUTED — today's
formula, today's baseline, replayed over every stored row — rather
than plotting the number as it was computed at the time. The cliff
disappears because it was never a change in the compiler.

Rules of the replay:
- The recomputed series starts where its counters start. A segment
  computed over a subset of counters (before the instruction or
  text veins existed) is either omitted or visibly labeled; it is
  never passed off as the full formula.
- The floor history in bench/welfare_floor.json stays exactly as it
  is — it is the audit trail of when the objective moved, and it is
  not smoothed. The chart reads the replay; the audit reads the
  floor file.
- Every future formula or baseline change re-runs the replay in the
  same PR, so the chart is always one definition applied everywhere,
  never a splice.

## 2026-09-02 — gavel: the weights, tuned to the developer's order of noticing

Clay, asked whether the welfare weights were justifiably optimal,
restated the objective: "the core thing we're attempting to optimize
for here is ultimately developer happiness. performance should be
insane, but if a language doesn't compile wickedly fast, like go,
devs may choose not to use it in the first place." He took the
recommendation that followed. The weights argument, recorded as the
doctrine requires before the floor moves:

    term             was    now   satiation
    run speed        0.30   0.30  2.0   (unchanged)
    run memory       0.30   0.26  2.0
    compile speed    0.28   0.32  0.5
    compile memory   0.12   0.12  0.5   (unchanged)

- **Compile speed rises, funded from run memory.** A developer feels
  compile latency on every edit and feels peak memory only when
  something falls over. Compile latency is an adoption gate; the
  memory model is the identity, but the objective's job is to punish
  regressions in the order developers notice them. Compile memory
  stays at 0.12 — the compiler's own footprint is a CI-container
  guard, not something a developer perceives.
- **The run-speed term splits in two halves.** Half is the equal-
  weighted mean of the advertised workloads — decode (jsonbench) and
  encode (encodebench), the rows the front page makes claims about
  and the workload the language exists for. Half is the equal-
  weighted mean of every other run benchmark in the objective — the
  shape guards, today oneshot, basket, wide, deep, pending, scan and
  digest (the last two joined in #1215) — which stress the memory
  model and protect against pathologies. The rule is "advertised
  versus everything else", so a benchmark added later lands in the
  guard half without this entry needing an edit. A shape win no longer scores as if a real workload
  got faster; the guards stay fully armed against regression. Per-
  counter saturation (the #184 ruling) applies inside each half
  before its mean.
- **Satiation constants stand.** Compile at 0.5 already says the
  target is instant and past instant nothing more is bought, with
  the curve's asymmetry making a compile regression cost more than
  the equivalent gain earns — the adoption-gate shape. Runtime at
  2.0 says eight times faster is eight times faster.
- **The exclusions stand**: wall time (nondeterministic), binary
  size (measured pointing the wrong way, #1217/#1219, pinned by
  spec), and everything unmeasurable, which the floor-permeable-to-
  language rule keeps welfare from vetoing.

Lands as a weights change: recorded here, `--set` in the same PR with
this entry's reason, the floor re-set, and the chart replay re-run so
the history reads under one definition.

## 2026-09-03 (ninth) — the compile row was counting the host's memory map

**DONE (the finding). Clay ruled on the entry the same day, and the ruling is
option 1 with the term named: make the row deterministic with glibc still
counted — no pinned pair, no band, no exclusion.** His words: "if changes to the
compiler can interact with glibc in a way that means generally more/less work,
which the 'compiler's own instructions' measure would be blind to, then we need
a way to include glibc's instructions but make them consistent."

That is right, and it kills the fix this session had built. Collecting from
`kanso::main` and leaving startup out would have made the vein blind to exactly
the case he names: a compiler change that causes glibc to do more work. The
toggle is dropped.

What survives is the measurement, because the ruling asks for the term and this
is the term.

`kanso check` reads `/proc/self/maps` before it reads a line of kanso. glibc's
`pthread_getattr_np` does it on Rust's behalf, to find the main thread's stack:
open the file, read it in 1024-byte chunks, `sscanf` each line until the one
holding the stack pointer turns up. What that costs is a property of the host's
memory map — how many mappings it has, how long their pathnames are — and this
vein has been counting it since it was minted.

The container reads the row deterministically, which is what made this
findable: forty runs of the gate's exact command on one binary returned one
value forty times. So the knobs below move a number that does not otherwise
move.

    one more shared library in the process     +32,090

That is an `LD_PRELOAD` of an empty `.so`: four more lines for the parse to
walk, and thirty-two thousand instructions the compiler never executed.
Smaller edits to the same text give smaller steps. Lengthening the executable's
own file name from nine characters to ten moves the row +2,193, and the profile
diff across that pair names the maps parse and nothing else — `__vfscanf_internal`
+1,180, `____strtoul_l_internal` +576, `_IO_sputbackc` +88, then `getdelim`,
`getline`, `_IO_setb` and `pthread_getattr_np` itself. Zero kanso symbols move.

That is the signature the six CI readings recorded: "every kanso symbol agrees
to the instruction ... three rows differ and all three are glibc". A startup
term is global rather than per-chip, which is why two of three chips produced
both values, and why pinning the glibc tunables took the spread from 5,064 to
508 without closing it.

The 508 itself is the downstream half. What the parse allocates and frees
before `main` sits under everything the compiler allocates afterwards, so
`_int_malloc` walks different bins and `memcmp` compares at different
alignments for an identical request sequence. A second knob isolates that half:
`argv[0]` at 39 characters rather than 38 moves the row +480, all of it in
`__memcmp_avx2_movbe`.

**It can be measured out**, which is why this is an instrument repair and not a
question for Clay. Collecting from the compiler's own entry point rather than
from `exec`:

    valgrind --tool=callgrind --collect-atstart=no --toggle-collect=kanso::main

                                  raw        toggled
    one more shared library    +32,090             -6
    name 9 -> 10 characters     +2,193             +5
    36 name lengths, spread      2,900             90

Ten toggled runs on one binary returned one value ten times. The residual 90 is
`argv[0]`, which the box already holds fixed at `./kanso`.

**OPEN — the rewiring itself.** Changing what the gate collects invalidates
every row in `bench/compile_instructions_by_cpu.txt`, the bare golden the trend
gate and `golden_prose` read, welfare's baseline for the counter, and the pinned
figure in the compiler page. Only CI may write those values, one chip per run,
so it is its own change and lands red until it is re-sat. Two things it owes:
a guard, because a `--toggle-collect` whose function is missing yields
`summary: 0` and exit 0 — a silent zero that would be reported as the front end
moving; and a welfare re-baseline in the same commit, because the number falls
by about 1% without a single instruction of work being removed, and banking that
as a win would be recording an instrument change as an improvement.

## 2026-09-03 — gavel: the bimodal row is made deterministic, not excused

On "The compile-instructions row is bimodal" (#265), Clay declined
both the pinned pair and an exclusion of glibc from the count: "if
changes to the compiler can interact with glibc in a way that means
generally more/less work, which the 'compiler's own instructions'
measure would be blind to, then we need a way to include glibc's
instructions but make them consistent, like how rspec can run with a
seed... you run some instruction at the top to clear out the glibc
state in test mode. this is crucial to be specific about."

So the ruling is option 1 with the term named. The measured signature
— five consecutive runs in one container agree exactly, two runners
with the same binary and chip disagree — says the randomness is fixed
at container birth, not per exec. Two suspects fit that shape and
both are cheap to test:

1. **Directory read order.** `kanso check lib/json` reads a source
   directory; ext4's readdir order is a hash order seeded per
   filesystem instance, so a fresh runner disk orders the same files
   differently, consistently within one VM. If the loader does not
   sort entries before compiling them, the allocation sequence — and
   so the heap layout, and so the alignment paths in `_int_malloc`
   and `memcmp` — differs per VM. This is a build-determinism defect
   independent of the vein: the compiler's work must never depend on
   inode hashes. Fix: sort in the loader.
2. **ASLR.** Heap base and mmap addresses randomize per exec unless
   disabled; `setarch -R` around the measurement removes the term.

Environment (env -i), glibc tunables and glibc revision are already
pinned. Apply both fixes, measure on several fresh runners; if the two
modes collapse to one, glibc stays in the count under a single exact
pin — no pair, no band, no exclusion. Only a residual that survives
both reopens the question, and then the fallback is the pinned pair,
never blindness. The entry leaves Blocking with this commit; the
rustc-patch rider is the implementer's (the pin did its job;
regenerate with a sentence).

## 2026-09-03 (tenth) — the bimodal row is made deterministic, not excused

**DONE for the two suspects, OPEN for the answer.** Supersedes the OPEN
paragraph closing the entry above, which planned to exclude process startup from
the count. Clay ruled against that the same afternoon: "if changes to the
compiler can interact with glibc in a way that means generally more/less work,
which the 'compiler's own instructions' measure would be blind to, then we need
a way to include glibc's instructions but make them consistent." He is right,
and the toggle is dropped — it would have made the vein blind to exactly the
case he names. No pair, no band, no exclusion.

The ruling named two suspects, both fitting the measured signature: five
consecutive runs in one container agree exactly while two runners on one binary
and chip disagree, so the randomness is fixed at container birth rather than per
exec. Both are now tested and both answers differ from the guess.

**Readdir order was already sorted, and cannot be the term.** `src/lib.rs`
sorts the files a compile reads before reading any of them; `src/eval.rs` sorts
what `list_dir` answers. The four remaining `read_dir` calls are in
`src/main.rs` on the `test` path and order-independent by construction — three
are `.any()` or `.count()`, and the fourth is a vector whose only order-sensitive
use is a single-element match.

One was sorted anyway on the rule rather than the symptom, and then WITHDRAWN,
which is the more useful finding. It changed no answer and it changed the
binary: `.text` 2,514,718 -> 2,517,278, moving this row 41,831,767 -> 41,834,008
and invalidating every recorded value. That is the wrong price for a defensive
sort, and worse than its cost is what it did to the experiment — the ruling asks
for the modes to be measured away on several fresh runners, and a changed binary
makes every earlier reading incomparable, so `setarch -R` and a new layout would
have been confounded from the first run. With the sort out, the binary is
main's, the recorded rows stand, and setarch is the only variable. The
determinism fix is worth landing on its own once this vein is settled and the
row it moves is not the row under test.

**ASLR is disabled now, and on the container it changes nothing.** `setarch -R`
reads 42,235,790 against 42,235,790 without it, and forty unwrapped runs on one
binary returned one value forty times. Applied regardless: the modes have only
ever appeared on the runners, and a container cannot rule out what it has never
reproduced. The gate prints `compile_aslr disabled=` so the job log says whether
the wrapper took; CI reads `yes`.

**Nothing is priced, because nothing moved.** With the sort withdrawn this
branch changes `scripts/gates/compile_instructions.sh` and no compiler source,
so the binary is byte-identical to main's and every counter reads what main
records. The trend gate has nothing to say and welfare holds at 76.1742943805134
against its own floor. An earlier revision of this branch did move
`compile_instructions` +2,241 and spent 0.00015 of the score declaring it; that
is reverted, and the declaration with it.

**The evidence so far, and it is not yet an answer.** Two CI readings on binary
sha b0e5a906c73d, twelve minutes apart, both on family0x19-model0x11, both
41,834,008 — the first agreement on one binary since the pair appeared. That
binary is the withdrawn one, so the agreement is kept as history rather than as
a result: it says the value repeats, on a tree that no longer exists. Against
that, a control taken at the same time on kanso#1235, which runs the UNFIXED
gate: 41,831,767 on a design-only diff, then 41,832,275 with one HTML file
added, same binary, 508 apart. So the modes were live on the old gate minutes
before the new gate read the same value twice.

**OPEN.** Two agreeing readings on one chip is not enough. What settles it is
the other chips landing on their own values and holding them. If they do, glibc
stays counted under a single exact pin and the per-chip key separates nothing —
the file collapses back to one golden. If a second value appears on a recorded
chip, neither suspect was the term, and the entry above already names what is
left: glibc parses `/proc/self/maps` before `main` to find the stack bounds, one
more shared library in the process moves the row 32,090, and that cost belongs
to the host's memory map rather than to the compiler. That is what "make them
consistent" would then have to reach.

## 2026-09-03 (eleventh) — one binary, one chip, two values: the pair is pinned

**DONE.** Closes the OPEN half of the entry above, which had the two suspects
tested and the answer outstanding. Searched the live log and
`design/log/compiler-log-archive.md` for prior treatments of the compile row's
spread before filing: the archive carries the 5,064-apart clusters that the
glibc tunables closed, and the live log carries the ninth and tenth entries.
Nothing there proposes a pinned pair, so this is new.

**Both suspects are falsified, and `setarch -R` comes back out.** The ruling
applied it on the argument that the modes only ever appear on the runners and a
container cannot rule out what it has never reproduced. That was the right
reason to try it and CI has now answered: `e47e412d` printed
`compile_aslr disabled=yes` and counted 41,832,275 on a chip whose row held
41,831,767. It moved nothing on the container either — 42,235,790 against
42,235,790, and forty unwrapped runs returning one value forty times. A knob
measured twice to move nothing is not carried, so the gate loses it and keeps
the finding.

**What settles it is the sha the gate prints.** `compile_binary sha256=` was
added to pair a reading with the binary that produced it and had never yet
answered a question. It answers this one:

| commit | binary sha | cpu | counted |
|---|---|---|---|
| `fc993f83` | `de5bfab22fbd` | family 0x6 model 0xcf | 41,831,767 |
| `e47e412d` | `de5bfab22fbd` | family 0x6 model 0xcf | 41,832,275 |

One binary, one chip, two values, eight minutes apart. The second ran with
`setarch -R` and the first without, so the same pair is the falsifier for the
last suspect. A third run — main at `5b0f2eb1` — counted 41,832,275 on
`family0x19-model0x11` against a recorded 41,831,767, so the second chip has
shown both values on this binary too.

I got this wrong once in the middle of the investigation and it is worth
recording how. Two runs agreeing at 41,832,275 on one sha, against rows recorded
on an older sha, read as "the binary moved and the rows are stale" — a tidier
answer than bimodality, and I had reverted the pair machinery on it before
fetching the third log. `fc993f83` killed it: same sha, same chip, the low
value. Two points that agree are consistent with almost anything.

**So the row pins a pair, which the ruling pre-authorised**: "only a residual
that survives both reopens the question, and then the fallback is the pinned
pair, never blindness." `scripts/gates/compile_ir_row.sh` grows three refusals
— a row that pins neither one value nor two, a row that pins three, and a pair
whose halves are the same number — and the lookup takes every value on the row
and asks whether the count is among them. Two is a cap and not a convention: a
band wide enough to hold 508 also holds kanso#1226's -5,621, which was a real
change to the compiler. The golden's bare line stays the reference row's FIRST
value, so a mode flip cannot reach welfare or the trend gate as a regression;
`bench/compile_instructions_golden.txt` is byte-identical to main's on this
branch, which is the check that it cannot.

Six specs, all watched red first, and three ratchet mutations verified to redden
this suite: the new `a_pinned_pair_grows_into_a_band`, plus the two existing
mutations whose refusal-count guards had to move from five to seven.

**A fourth chip, recorded the same afternoon.** CI landed on
`family0x6-model0xad` — Granite Rapids, new to the pool — and the gate refused
rather than passing on another chip's number, which is the unrecorded-chip
design doing its job. It counted 41,832,275 on the same binary
`de5bfab22fbd`, so three of the four recorded chips have now read the high
value there and two of them have read both. It is recorded as a SINGLE,
because one reading is one reading and a row may not claim a mode nobody has
seen it take.

That standing is an argument the per-chip key is separating nothing on this
binary: the value looks like a property of the run rather than of the silicon.
Not acted on. The key costs nothing while it is still right about the
cross-binary case the file's header decomposes, and one binary is not enough
to retire it.

**OPEN, and stated as a measurement rather than a plan.** The ninth entry's
term — glibc parsing `/proc/self/maps` before `main`, at a cost that is a
property of the host's memory map — is still the only mechanism that fits, and
it is not established that two runners differ in their maps by the 508 this
needs. The row would have to print the map's line count beside the count to
say. Nothing rests on the answer now that the pair holds the gate green.

## 2026-09-03 — the weights move to the developer's order of noticing

**DONE for the weights and the floor, HELD for the replay.** Implements the
gavel of 2026-09-02, which recorded the argument and left the build. Searched
the live log and `design/log/compiler-log-archive.md` before filing: the
archive carries the 2026-08-29 saturation ruling and the entries that priced
0.30/0.30/0.28/0.12, and nothing there implements this split.

    term                    was    now   satiation
    run speed (advertised)         0.15  2.0    new half
    run speed (guards)             0.15  2.0    new half
    run speed               0.30    —    2.0    splits
    run memory              0.30   0.26  2.0
    compile speed           0.28   0.32  0.5
    compile memory          0.12   0.12  0.5    (unchanged)

**Two terms of 0.15, not one term averaging two halves.** They are the same
arithmetic and the pair reports better: the breakdown says which half moved.
The guard half is written as the REMAINDER — `guard_work` is `held_work` plus
`paced_work` — so a benchmark added later lands there without this entry or
that line needing an edit, which is what "advertised versus everything else"
asks for.

**What the split buys, in the numbers.** Nine guards against two advertised
rows meant a guard carried nine elevenths of the run-speed term and the front
page's own claims carried two elevenths. A shape win scored as if a real
workload had got faster. On the parity fixture a thousandfold win now scores
**52.99** on an advertised row against **49.11** on a guard; before the split
both read **48.48**, because a counter was a counter.

**The score falls 76.1743 to 73.0623 and nothing about the compiler changed.**
Both halves of that are worth stating. The fall is real — under the developer's
stated order of noticing the project is further from ideal than the old weights
said, because compile cost is the weakest dimension (+36.3% instructions,
+143.7% allocations against baseline) and it just gained weight, while run
memory, which is strong, lost some. And it is not a regression: no counter
moved, and scores either side of this commit were taken with different rulers.
That is the second such step in this line; the first is the 2026-08-29
saturation ruling, 87.85 to 73.83.

**The floor is edited by hand, and the tool asked for that.** `--set` refuses
to lower the objective — "A fall means the change is worse by the project's
stated preferences ... this is Clay's call to make, in conversation — not a
flag's. (The floor file itself can be edited by hand, where a reviewer will see
it.)" The gavel is that conversation, so the reason goes in the history entry
and the number goes in the file, where the diff shows it. The refusal is right
and stays; a flag that could lower the floor is a flag that could launder a
regression.

Three specs, and the two that existed were watched red at the numbers above
before the pins moved. The new one —
`a_win_on_an_advertised_row_outscores_the_same_win_on_a_guard` — was watched
red against the unsplit formula, where it read 48.48 for both fixtures, which
is exactly the property it exists to deny.

**HELD, and sent to Clay: the chart replay.** The gavel ends "the chart replay
re-run so the history reads under one definition." `docs/numbers.html` states
the opposite rule and stated it before the gavel: "the welfare line is
recorded, not recomputed. the score a commit shipped with is a fact about that
commit, and replaying history against today's baseline would rewrite it." It
already carries the 2026-08-29 step documented as a discontinuity rather than
replayed.

A replay is also under-determined, which is the part the gavel could not have
known. The objective's counter set has grown — the digest in #1198, scan,
escape and index in #1215 — so a commit whose goldens predate a counter has no
value for it, and scoring it under today's formula means inventing one through
the granted-baseline machinery. That machinery exists to admit a counter going
forward at its dimension's standing, not to backfill a history it was never in.
Not resolved here, because what the recorded line MEANS is his to say and not a
matter of how to compute it.

## 2026-09-03 — the replay could not be computed, and the reason is the rows

**DONE for the prerequisite, the chart itself still to build.** Searched the
live log and the archive before filing: the 2026-08-31 directive rules the
replay and the rider of today restates it against the page's older sentence.
Neither says what the stored rows contain, which turned out to be the thing
that decides whether a replay is possible at all.

**The rows carry 12 of the 24 counters the formula reads.** Measured on the
newest row in the perf-history branch — commit `a100f4f`, this afternoon's
merge:

    missing: wide_instructions, deep_instructions, pending_instructions,
             digest_instructions, scan_instructions, escape_instructions,
             index_instructions, scan_arena_blocks, scan_peak_bytes,
             digest_peak_bytes, digest_arena_blocks, compile_instructions

`compile_instructions` among them, which is the vein this whole day was about.
`perf_record` writes a hand-picked list that has not kept step with the model:
the digest counters joined the objective in #1198 and scan, escape and index in
#1215, and none of them joined the row.

So the rider's rule — "the replayed series begins at the first commit for which
every counter in the current formula exists" — names no commit. Applied to the
data as it stands the replayed line is EMPTY, and would have been empty however
carefully the chart was written. That is worth stating plainly because the
failure would have looked like a charting bug.

**The fix is that the objective names its own counter set.** `welfare
--counters` prints the `name=value` pairs `score` was given, and `perf_record`
records those. Assembling the list a second time in `perf_record` is what
produced this: two lists drift the first time a counter joins the model, and
nothing was watching the second one. Printed from where the score is computed,
the row cannot fall behind the formula — a counter that enters the model enters
the row in the same commit.

Three specs, two of them watched red against main's welfare, where the flag
prints the banner instead of a counter set. The third pins that asking what was
scored does not move the floor, because a second door to the ratchet is the one
thing this must never become.

**STILL TO BUILD, and it needs rows that do not exist yet.** `perf_record` has
to carry the printed set into the history row, and the chart has to replay the
current formula over the rows that carry it. The replayed line then starts at
the first commit merged after that lands, which satisfies "no backfill" by
construction rather than by a rule anyone has to remember. The recorded
`welfare` field stays for the earlier rows, drawn distinctly and labeled as
scored under earlier definitions, and `bench/welfare_floor.json` remains the
audit trail either way.

## 2026-09-03 — the objective emits its own model, so the chart can replay it

**DONE for the parameters, the chart still to draw.** Searched the live log and
the archive before filing: the entry above records that the rows carried 12 of
24 counters and fixes that; nothing there covers where the WEIGHTS reach the
chart from, which is the second half of the same problem.

The replay has to happen on the page, because the page is static and the rule
is "the current formula and baseline over the stored rows" — a value computed
once and stored cannot re-score old rows when the formula next moves. That puts
two things at risk of being copied onto the page: the numbers and the
arithmetic.

**The numbers are the dangerous half and now come from the tool.** `welfare
--model` prints the terms and the baseline in the shape every other vein here
uses:

    term run speed (advertised)|0.15|2.0|decode_instructions,encode_instructions
    base decode_instructions=3266896510

A weight retyped into a chart is a weight that survives the next gavel, and the
line would then show a formula nobody ruled while looking exactly as
authoritative. Emitted, it cannot: the 0.32 that landed this afternoon is in
that output because it is in the model.

**json was the first attempt and was the wrong reach.** It made both readers —
this repo's spec and the chart's javascript — grow a parser apiece. Lines cost
neither.

**The arithmetic is restated on the page, and a spec makes that safe.**
`the_model_and_the_rule_reproduce_the_score` reads only `--model` and
`--counters`, applies the rule as a reader of the 2026-08-29 gavel would state
it — saturate `r / (r + s)` per counter, mean within the term, weight, sum —
and asserts the answer against welfare's own banner. It agrees to 73.06. That
agreement is evidence rather than tautology because the test shares no code
with the tool; it is written from the ruling, not from the implementation.

A second spec pins that the weights sum to one. A term added without taking
weight from another reweighs every other term silently, which is a change to
what the project wants made by arithmetic instead of by a gavel.

Both were watched red. Emitting `t.satiation` where the weight belongs made the
reproduction spec answer 479.0922 against welfare's 73.06 — the restatement and
the tool disagreeing is exactly the failure it exists to catch. Raising the
compile-speed weight to 0.33 made the sum spec say 1.0100000000000002.

**STILL TO DRAW.** numbers.html reads the emitted model, replays over the rows
carrying the full counter set, starts the line at the first such row, keeps the
earlier rows' recorded scores in a visibly distinct style labelled as scored
under earlier definitions, and its "recorded, not recomputed" sentence is
rewritten to match. That needs a real row, which CI writes on the commit after
the counter-set change lands. The sentence and the chart move together: a page
claiming a replay it does not perform is the one outcome worse than the stale
sentence.

## 2026-09-03 (second) — the chart replays, and the sentence that denied it is gone

**DONE.** Searched the live log and the archive before filing: the entry above
built `welfare --model` and left the drawing undone, and the 2026-08-31
directive "the welfare chart replays the current formula" is the ruling this
answers. Nothing in either file draws it.

**Two lines where there was one.** The solid one is the replay: every row
scored by the model and baseline in force today, so its points can be read
against each other. The dashed one is what each commit shipped with, kept
because it is the record and it is where the 2026-08-29 definition step lives.
They share a scale. Drawn to their own ranges each would fill the plot and the
reader would compare two shapes with no axis between them, when the gap where
they overlap is the thing worth seeing.

**No backfill, and it is a spec rather than an intention.** `replayScore`
answers null for a row missing any counter the model reads, so the solid line
starts at the first row carrying the whole set — today that is the commit after
the counter-set change, and before it the rows genuinely do not hold the
numbers. `a_row_missing_a_counter_is_not_scored` pins it at three rows: whole,
partial, empty. Watched red by making a missing counter skip rather than
refuse, which scored a half-row 16.67 and drew it beside real points.

**The page's own functions are what the spec runs.** `parseModel` and
`replayScore` are lifted out of the html by brace matching and run under node
against `welfare --model` and `welfare --counters`; the answer has to be
welfare's own. A copy pasted into a test would agree with itself forever.
Watched red by replacing the saturation term with a constant: 72.2758 against
73.06, which is exactly the size of drift that looks like a real move.

**`model.txt` sits beside `history.jsonl`,** written by the same job, replaced
rather than appended — it describes the model as it stands and its history is
welfare_floor.json. If it cannot be fetched the recorded line still draws and
the replayed one is absent, which is the right failure: a page with no numbers
beats a page with stale ones.

**The sentence is gone.** "the welfare line is recorded, not recomputed" was
true when written and stopped being true on 2026-08-31. The page now says the
score is computed in the browser from each commit's counters against today's
baseline, and says where the baseline comes from.

The ratchet gained `chart_replay`, anchored on the saturation term rather than
a number, because the number moves whenever the compiler does.

## 2026-09-03 — gavel: failures are for the exceptional; the bang chooses the channel

Clay, correcting a "map ten files and collect the results" example:
"if you're talking about files you expect to be there, then them not
being there is exceptional. if you know it's possible for them to not
be there, you wouldn't use an exception, you'd just return a
file_not_found type." Gaveled as doctrine:

- **A failure is for the exceptional. An anticipated outcome is
  data.** If an alternative is part of the operation's normal
  vocabulary, the answer is a typeset — `text | file_not_found` — and
  dispatch handles both arms like any values: no box, no bubbling, no
  rescue license. The failure channel, with its provenance and cause
  chain, is for what the program did not plan for.
- **The bang chooses the channel, everywhere.** The map rule already
  ruled — `foo[k]` answers `none` as data, `foo[k]!` answers a
  failure — generalizes to every operation with an anticipated
  alternative, io included: `io/read_file path` answers
  `text | file_not_found`; `io/read_file! path` is the caller
  insisting, and a violation bubbles as a failure. The suffix
  grammar's contract (a `!` name answers a box) is the same rule seen
  from the declaration side. The canonical chain reads
  `io/read_file! path .> json/parse .! ... .? when_failed`.
- **Downstream, not decisions:** containers may still hold results
  (test matchers, supervisors, #1057), since holding is neither
  proceeding-as-success nor an unmarked conversion; and the io
  boundary deep-demands the program's final value, so a failure
  buried in an output structure fails the run unless it was rescued
  into data first.

The renaming of the box from `effect` to `result` is recommended
beside this and awaits its own word.

## 2026-09-03 — rider: the replay is ruled; the page's rule yields to it

The cloud session held the chart replay because docs/numbers.html
says "the welfare line is recorded, not recomputed... replaying
history against today's baseline would rewrite it," and because the
counter set has grown (digest in #1198; scan, escape, index in #1215),
so a commit predating a counter has no value for it. Both points are
answered by the record as it stands:

- **The page's rule is older than the gavel and yields to it** —
  later replaces older. Clay ruled the replay on 2026-08-31 ("yes"),
  seeing the #184 re-scoring as a cliff that "was never a change in
  the compiler." The page's sentence is rewritten to say what the
  chart now shows: the current formula and baseline, replayed over
  the stored rows, so the line is one definition applied everywhere.
- **No backfilling.** The replayed series begins at the first commit
  for which every counter in the current formula exists. Earlier
  history is not invented through the granted-baseline machinery
  (which admits a counter going forward, never backward); it is
  either omitted or drawn from the recorded scores in a visibly
  distinct style and labeled as scored under earlier definitions.
  The directive of 2026-08-31 already said this; it is restated here
  so it cannot be read as under-determined.
- **The audit trail is untouched.** bench/welfare_floor.json keeps
  every step with its reason — the 87.85 -> 73.83 re-scoring
  included — and that file, not the chart, is where "what did a
  commit ship with" is answered.

## 2026-09-03 (twelfth) — the third chip shows both values, on one binary

**DONE, and it is the gate reporting rather than a change.** Searched the live
log and the archive before filing: the eleventh entry pins the pair and names
two chips that had shown both, and nothing there records a third.

CI on kanso#1235 counted **41,831,767** on `family0x19-model0x1`, whose row
pinned only 41,832,275, on binary sha `de5bfab22fbd` — the same binary every
reading in this sequence was taken on. The row becomes a pair. The refusal was
correct and the number was real, which is the whole point of a pin that admits
exactly what has been measured and nothing else.

The standing on this binary:

| chip | values seen |
|---|---|
| family0x6-model0xcf | both |
| family0x19-model0x1 | both |
| family0x19-model0x11 | both |
| family0x6-model0xad | 41,832,275 only |

**Three chips of four have produced both values and no chip has produced a
third.** Two claims follow, and only the first is made here. The mode is a
property of the run rather than of the silicon — three independent chips each
producing the same two numbers is hard to read any other way. The second, that
the per-chip key can therefore be retired, is NOT claimed: the fourth chip has
one reading, and a key is retired on evidence rather than on a pattern that
holds for three quarters of the rows.

What would settle it is stated so the next session does not have to design an
experiment: `family0x6-model0xad` reading 41,831,767. Every recorded row would
then hold the same pair, the key would demonstrably distinguish nothing on this
binary, and the file could collapse to one golden holding two values — the
outcome its own header predicted for the case where pinning worked and the
chips stopped disagreeing. Until then the key is kept, because it is still
right about the cross-binary case the header decomposes, where two chips read
5,124 apart.

The mechanism is still the open question and is unchanged by this: glibc's
`/proc/self/maps` parse fits the signature, and what is not established is that
two runs differ in their maps by the 508 this needs.

## 2026-09-03 — gavel: the suffix contracts are refusals, as ruled in July

On "What a `!` name promises on the declaration side" (#272), Clay:
"rule it." Option 1 — and it is not new. The July suffix-grammar
ruling (archive, "The suffix grammar, with teeth") already said: "Both
suffixes are checked contracts, not conventions: a `?` function must
answer bool, and a `!` function's answer typeset must include an err
type. The checker refuses violations of either." The entry did not
cite it; the only news is that the contract was never implemented —
`pub fn shout! x` answering a plain string compiles today.

Under effects-are-types the contract reads directly off the answer
type: **a `!` name must answer a result; a `?` name must answer bool;
the checker refuses either violation at the declaration.** A bang
that cannot fail is a lie in the name — `foo[k]!` differs from
`foo[k]` precisely by being able to fail. The implementation question
the entry raised (can infer see whether an answer reaches the failure
channel) dissolved with failures becoming a type: the checker reads
the declared or inferred answer type, the same way it reads any
other. Options 2 (unchecked convention) and 3 (std-only) would
respectively reverse a standing ruling and mint a two-tier rule the
language has nowhere else; both declined. Enforce both suffixes in
the same change, since `?` sits in the identical unimplemented state.
The ledger entry leaves with this commit wherever it lives.

## 2026-09-03 — the bang half of the suffix contract, and the half already built

**DONE.** Searched the live log and the archive before filing, and this time
for the right phrase: the archive's "The suffix grammar, with teeth" is the
July ruling, and Clay's gavel above cites it. That search is what my #272
ledger entry skipped, which is how a settled question got asked again.

**The correction the fixtures forced.** Both the gavel and my own survey said
`?` sat in the identical unimplemented state as `!`. It does not. A fixture
answering a string from `ready?` is already refused, by a check in check.rs
that reads the same inferred answer set this change reads:

    error[naming]: `ready?` asks a question: a `?` function answers true or
    false (err may ride along)

So only the bang half was missing, and the change is one condition beside the
one that was already there.

**The measurement that explains why nobody noticed.** There are ZERO
`!`-suffixed declarations anywhere in the tree, against 353 `?`-suffixed ones.
The contract was unenforced because nothing exercised it. That also fixes the
ordering with the io half: `io/read_file!` will be the first `!` name the
language has ever had, so the guard lands before the name it guards rather
than after.

**The rule, mirroring the query direction exactly.** A `!` name's answer set
must contain ERR. It fires on a provable lie only — an empty set means
inference learned nothing, and TOP, which a generic driver widens to, still
holds ERR, so neither is accused. That conservatism is not new caution; it is
copied from the `?` direction, which has run over 353 declarations without a
false positive.

**Watched red, then watched the fixture prove itself.** `shout!` answering a
plain string compiled clean and printed `hi!` before the change. After it, the
refusal fires. Then the check was disabled and the corpus went red on that
fixture, which is the half that matters: a golden that cannot fail is a golden
that proves nothing.

Diagnostics 309 to 310, none newly unpinned. All eight gate scripts and every
std module still check. The compile counters could not be measured here — the
container runs rustc 1.94.1 against the goldens' 1.98.1, which is the host pin
doing its job — so CI reads them.

## 2026-09-03 — CI read the compile row, and the fall is not the compiler

Searched the live log and the archive before filing. The maps-parse term is
recorded here twice already — the 2026-09-03 entries that found it and the
by-cpu table's own note — and neither says what happens when an ordinary
change moves it. Nothing in either file records a compile row falling while
the front end rose, so that part is new.

The entry above ended by saying CI would read the compile counters this
container cannot. It did, and the row refused: the table pinned 41,831,767 /
41,832,275 on family0x6-model0xcf and the run counted 41,829,232, a fall of
2,535.

**The front end got more expensive while the row fell.** A differential
settles it without needing the host pin, because a difference between two
builds on one machine does not care what rustc CI runs. Same source, with the
check and without, everything else held:

    without the check   42,235,790
    with the check      42,240,325   +4,535

The profile diff attributes all of it, and exactly one kanso symbol moves:

    kanso::check::check_merged                          +848

The rest is glibc's startup parse of /proc/self/maps:

    __vfscanf_internal                                +2,360
    ____strtoul_l_internal                            +1,158
    __memcmp_avx2_movbe                                 -841
    getdelim, _IO_sputbackc, _IO_setb, sscanf, the rest  ~860

So CI's 2,535 fall is +848 of front-end work and roughly 7,070 of maps-parse
and allocator movement going the other way on a differently laid-out binary.
The container moves that term the other way on the same source change. That is
what a term keyed to the host's memory map rather than to the code does, and
it is the mechanism this vein measured a few entries ago: one more shared
library in the process moves the row +32,090 with the compiler executing
identical instructions.

The gate asks a change to say whether its move is a regression to explain or a
win to bank. This one is neither. The compiler costs 848 more instructions to
run, for a refusal it did not have, and the row it is measured by went down.

**Every row cleared; one survives because CI re-sat it.** The binary is new —
sha 0804abe57190 against the de5bfab22fbd every recorded reading was taken on
— so the file's own rule decides the rest: a value measured against the old
binary is worse than no value. Three rows go. family0x6-model0xcf holds
41,829,232 as a SINGLE, and the golden's bare line follows it.

The single is expected to refuse. Three chips have produced both modes on one
binary, so an exact pin on one reading goes red about half the time until a
second makes it a pair. Writing 41,829,740 to pre-empt that would be recording
a number nobody counted, which this file refuses on every page.

**Welfare does not move.** 73.06 against a floor of 73.06. 2,535 out of 41.8
million is 0.006%, and compile cost satiates at 0.5, so the term does not
reach two decimal places. There is nothing to ratchet.

`docs/compiler.html`'s tagged figure moves with the golden; golden_prose was
what caught it.

## 2026-09-03 — the second chip, and the two agree to the instruction

The row cleared in the entry above left one chip recorded, and said the single
was expected to refuse. It did, one run later and for the other reason the
gate has: CI landed on Zen 3, whose stale row had just been removed, and
refused because no row named it.

It counted 41,829,232 — family0x6-model0xcf's value exactly, on the same
binary sha 0804abe57190.

    21:33  family0x6-model0xcf   0804abe57190   41,829,232
    21:42  family0x19-model0x1   0804abe57190   41,829,232

On the previous binary those two chips had each produced BOTH modes, 508
apart, and this file's standing question was what still moves the heap layout
when the binary, the cpu features, glibc, valgrind and the environment all
agree. On this binary they do not differ at all.

Two readings settle nothing. Each chip has shown one mode once, which is the
same state that preceded the last disagreement, and reading agreement into it
now would be the mistake this vein keeps catching. What it is: the first
binary on which cross-chip agreement has been seen rather than the pair. The
next reading on either chip is what says whether the modes came back.

The row is recorded from CI's own sitting, which is what every row in this
file is.

## 2026-09-03 — the io half: absence is data, and the bang chooses the channel

Searched the live log and the archive before filing. The archive's nineteen
`read_file` mentions are the message wording, the not-text case and the
os/io split; the live log's five are this thread. Neither has the typeset
shape, so this is the gavel's io half arriving rather than a question asked
twice.

**What a read answers now.** `os/read_file path` answers `text |
file_not_found`, and both arms dispatch like any values — no box, no
bubbling, no rescue license. `os/read_file! path` is the caller insisting,
and a missing file bubbles from it as a failure. That is Clay's ruling of
this morning applied where he applied it: "if you know it's possible for
them to not be there, you wouldn't use an exception, you'd just return a
file_not_found type."

The gavel writes `io/read_file`. It is built as `os/read_file`, because the
Go split of 2026-08-17 put the filesystem in `std/os` and `io` keeps the
read and write surface. The name in the ruling is shorthand for the
operation, not a placement.

**The shape, copied from `run`.** A builtin cannot name a type declared in
kanso, so `builtin_read_file` answers a two-element list and the std wrapper
names it — exactly what `run` already does with its three. `file_not_found`
has one field, which makes it a transparent nominal subtype: the value IS
the path, carrying the tag dispatch reads.

**Where the split is made, on each engine.** The interpreter splits on
`ErrorKind::NotFound`, native on `errno` being `ENOENT` or `ENOTDIR`. Every
other reason to fail a read stays a failure, because none of them is part of
reading's ordinary vocabulary. The playground has no filesystem at all, so
every read there is unplanned and still errs; it refuses with its own clear
diagnostic, which is what the differential law allows.

    absent    interpreter and native    absent: nowhere.txt
    present   interpreter and native    read: hello
    insisted  interpreter and native    cannot read nowhere.txt: no such file

**A divergence the fixtures caught before CI could.** `a_rescue_inside_a_
joined_stage` reads one golden across three engines, and its comment says
why: the arm discards the reason, so a page saying it has no filesystem and
native saying the file is not there both come out `rescued`. With a plain
read that stops being true — two engines answer data where the third errs.
The bang restores it, and the fixture now carries that reason.

**What the migration cost, counted.** Twenty-eight calls in thirteen scripts
insist; sixteen more across the browser sweep, the book harnesses, the
coverage scan and the benchmarks; five fixtures and three embedded programs
that read a missing file to exercise the failure channel; and two goldens
that cite an `os.kso` line number, because the new type sits above `exit`
and moved it 39 to 51. That last is a cost the call-site survey could not
see and the suite found in one run.

Three plain reads remain, each deliberately: the fixture testing the
argument check, hako's lock, and the book's showcase.

**hako lost a race.** `locked_at` called `os/exists` and then read the file
it had just asked about. One read now, answering both cases, and the window
between the two calls is gone with the second call.

**The book teaches it from the other end.** `ch05/fallback.kso` was a sample
that FAILED — a plain arm does not catch a failure, only `rescue` does, and
its recorded output was the endpoint error. The same program now prints "no
orders yet", because the arm it always had is the right one for data. The
two `missing` samples and `rescued` insist, so they still show a failure
reaching the endpoint and a rescue catching one.

**Every read costs one more plan step**, visible in the ch05 plan goldens:
the wrapper's lambda over the builtin, where the bare builtin had none.
`count_plan` goes two continuations to three and `save_plan` one to two.
Recorded rather than hidden — it is the price of naming the alternative in
the language instead of in a message string.

(This paragraph said TWO steps when it was first written, and it was right
about the shape it described: `read_file!` was `read_file . insisted`, two
kanso functions deep. The entry below rewrites the bang to read the builtin
directly, which is one function, and the goldens moved with it. Corrected
here rather than left to contradict the numbers a reader can run.)

Welfare 73.06, on the floor and unmoved.

## 2026-09-03 — one welfare line, and thirty-six dips read

Searched the live log and the archive before filing. The live log's
2026-08-31 entry rules the replay and the 2026-09-03 entries carry its
implementation; the archive's `replay` mentions are the clock, the compile-
memory backfill of 2026-08-01 and the 474-commit rebuild that was declined.
Neither has the rescoring rule or the dip investigation, so this is the
directive's answer rather than a question asked twice.

**The directive.** Clay, on the chart: "i don't need anything 'replayed' vs
'recorded', i'm just talking about re-doing it for those historical commits
based on the numbers we already have, one time." Retire the two-line design.
Rewrite `history.jsonl`'s welfare column in place, under the current formula,
on the counters each row carries — terms average over present counters,
absent terms dropped and weights renormalized per row, the row tagged with
the formula that scored it. Repeat on every future formula change. And: "the
line must be monotone non-decreasing except at a gaveled language feature;
investigate any other dip as an un-ratcheted rise or a defect."

**A correction to the reason he gave.** He said the chart drew no replayed
line because `model.txt` had never reached the perf-history branch. That was
true when he first looked and had already been fixed — the file landed with
row 499, published by #1236's own merge. The real reason is worse. The
replay refused any row missing a counter, and eighteen of the objective's
twenty-four counters exist only in the last two rows of five hundred, so the
line it could draw was two points long. Publishing `model.txt` would not have
helped; the two-line design was unrescuable for a reason nobody had checked.

**The counter presence, measured off the file.** `encode_arena_blocks`,
`compile_allocs` and `compile_peak_bytes` in rows 1–138 and 290–500;
`encode_instructions`, `oneshot_instructions` and `basket_instructions` from
row 123; the other eighteen in rows 499 and 500. Rows 139–289 carry three
counters of twenty-four.

**Thirty-six dips, and thirty-three of them are real.** The falls at rows 34,
56, 62, 65, 72, 128 and 134 are one counter: `compile_allocs` climbing
118,364 to 137,324 across August, which is the front end getting more
expensive a commit at a time with nothing watching that dimension yet. Row
165 is `encode_instructions` 9.208G to 9.713G with `oneshot` and `basket`
moving with it. The remaining twenty-five are the same shape, smaller. These
are un-ratcheted rises exactly as the directive names them, visible now
because one formula reads the whole file where each row was scored by
whatever definition was in force that week.

**Three are the rule meeting sparse rows, and they are the ones to know
about.** Row 123 (−11.25): three instruction counters arrive with baselines
set at their own commit's value, so each enters at a ratio of one and
saturates at 0.333 while the mature counters sit far above. Row 139 (−25.59):
the compile counters leave the rows entirely, the compile-speed and
compile-memory terms are dropped, and 0.44 of weight renormalizes onto run
terms scoring a third where compile scored two thirds. Row 499 (−0.88): the
other eighteen arrive, mixed — the guard term rises 0.069 to 0.111, run
memory falls 0.259 to 0.221. None is a compiler regression and none is a
defect in the rewrite.

**So every row carries `scored_weight`.** Each term's weight times the share
of its counters the row holds; one means the row was scored on the whole
objective. It reads 0.31, 0.41, 0.11, 0.41 and 1.00 across the four spans
above, and it steps at exactly the three rows the score steps at for no
compiler reason. Renormalization alone cannot say this: it asks whether a
term can be scored at all, so a term keeps its full weight on one counter of
ten and a row scored on six of twenty-four still reads 1.00.

**The formula date moved into the model.** `welfare -- --model` now emits
`formula <date>` ahead of the terms, and the rescorer stamps every row's
`scored_by` from it and refuses a model without one. A copy of the date in
the rescorer would keep its old value through the gavel that moved the
formula, and five hundred rows would claim a definition that no longer
exists.

**The rewrite runs on every push, not only on a formula change.** It costs
1.3 seconds over five hundred rows and removes the step somebody has to
remember. It runs before perf-history is checked out, because that branch
holds `history.jsonl` and nothing else — after the checkout there is no
`scripts/welfare_rescore` in the tree to run. `model.txt` is deleted from the
branch in the same push: nothing reads it now, and a copy of the weights
nobody reads is a copy that goes stale unnoticed.

**Two implementations of one formula, held together by a spec.** `welfare`
scores the goldens and `welfare_rescore` scores a row, so the rule is stated
twice and the second statement can drift. The spec builds the row the
objective would record today, runs the rescorer on it, and requires welfare's
own banner — rounded to welfare's two places rather than compared inside a
tolerance. A 1% change to the rescorer's satiation turns it red at 72.90
against 73.06. The ratchet row that watched the page's javascript now watches
this.

**Eight specs, three mutations watched red.** Removing the renormalization
answers 25.00 where the fixture wants 33.3333; saturating the mean instead of
each counter answers 90.20 where it wants 66.2954, which is the shape the
2026-08-29 gavel closed; measuring coverage by term instead of by counter
reads 1.00 where the fixture wants 0.63. Every number in the fixture is
derived by hand in the comment beside it and none of them moves when a
benchmark does.

Welfare 73.06, unmoved. The rewritten column reads 73.0625 for the newest
row, which is the same number to welfare's precision.

## 2026-09-03 — the read's answer had no type, and the decode lost its bracket

Searched the live log and the archive before filing. The live log's io-half
entry of this morning is the migration; the archive's `beat` entries are the
tier work of August and the carry-tier decline of 2026-09-01. Neither has the
read's shape or the yield hole, so this is a defect the io half introduced
and the golden caught.

**What broke.** `os/read_file` answers `text | file_not_found`, and a builtin
cannot name a type declared in kanso, so the builtin handed back a
two-element list `[there, text]` and the wrapper read it. The list costs the
caller the string's type: inference gives a list index the top set, `beat.rs`
reads that set to decide whether a slot may be carried across a rewind, and
an untyped slot keeps the grow-only arena. jsonbench decoded the same bytes
with 248 arena blocks instead of 2, a 260 MB peak instead of 2 MB, and
`beat_iters` 1 instead of 151. Same decoder, same document, 124 times the
peak.

**The builtin answers `none`.** The shape `os/env` already uses for a
variable that is not set, and inference already types it: `read_file` yields
`STR | NONE`, the wrapper's `found` dispatches on the none arm, and the text
arm keeps its type. All three engines make the same split — the interpreter
on `ErrorKind::NotFound`, native on `ENOENT` or `ENOTDIR`, the playground
refusing because it has no filesystem.

**`none` is threadable.** It has no payload, so nothing in it can dangle
across a rewind — the criterion `THREADED` already states for strings and
records. Its absence cost jsonbench the plain beat: with `NONE` out of the
set the loop is a *carry* beat, evacuating its document argument every
iteration, and with it in the set the loop is a plain beat that rewinds.
Measured both ways on the branch.

**And a hole the fix walked into.** `os/read_file` is typed because
`desc_yield` reads a chain's yield by the head's BARE NAME, and the wrapper's
name collides with the builtin's. `os/read_file!` — same body, one character
more — matches nothing and falls to the top set. Written out:

    os/read_file  "large.json" . go   loop/3: beat: rewinds every iteration
    os/read_file! "large.json" . go   loop/3: grow-only: argument 1 ...

Same package, same loop, one character apart.

**Counted, because the bang is the small end of it.** Cross the table's
entries against every std wrapper that sits over an effect builtin and EIGHT
wrapper names are absent: `net/listen`, `net/port`, `net/accept`, `net/read`,
`net/close_conn`, `net/close_listener`, `os/read_file!` and `os/kill`. The
names that do hit are hitting by coincidence — `os/write_file` because the
wrapper and the builtin happen to share a name, `net/write` because it lands
on the `write` entry meant for `io/write` — and the table's own `net_write`
and `net_close` rows are reachable only from inside lib/net, since the
wrappers are called `write`, `close_conn` and `close_listener`.

`net/read` is the one that matters. A socket server looping over the bytes it
read is the ordinary shape, and it pays what jsonbench paid. On a twelve-line
probe whose loop allocates once per iteration:

    os/read_file  "x" . go    beat: rewinds every iteration
    os/read_file! "x" . go    grow-only: argument 1 may carry heap
    net/listen ":0" . (l -> net/accept l . (c -> net/read c . go))
                              grow-only: argument 1 may carry heap

The cause is proven rather than inferred: adding one arm reading
`base(n) == "read" | "net_read" => STR` and rebuilding flips the net/read
probe to `beat: rewinds every iteration`. That arm was reverted and is not in
this branch, because more table rows are the bug. The fix is to infer a kanso
function's yield in the fixpoint beside `ctx.returns` and have `desc_yield_of`
consult it, leaving the table to cover true builtins only — a change to the
fixpoint rather than a patch to a match arm, which is why it is named here and
not made here.

So jsonbench's generated main writes the arm out — `fed`, which is
`os/read_file!` at the call site — with the reason in
`bench/make_jsonbench`. Both spellings mean the same thing, and when a kanso
function's yield is inferred it goes back to the bang.

**The goldens moved, uniformly and once.** Five programs read a file at
runtime and all five pay the same: allocs +9, alloc_bytes +288,
`cohort_frees` 0 to 1, `evac_allocs` +12, `carry_dedup` +2. That is the read
wrapper's dispatch, once per program rather than per iteration, and it is the
price of naming absence in the type. The decode's own shape is unchanged —
`arena_blocks` 2, `arena_peak_bytes` 2,097,152, `beat_iters` 151,
`el_parses` 318,450, `find2_calls` 1,571,250 all byte-identical to main.

Welfare 73.06, unmoved: the objective weighs instructions, peaks and blocks,
and nine allocations move none of them.

**The spec.** `tests/golden/read_beat` reads its own source and loops over it
200 times and 800 times; `beat_iters` reads 201 and 801. Watched red with the
list put back in `read_value` and `found` back to `if r[1]! r[2]!`: both
report 0, the loop never bracketing. It asserts `beat_iters` rather than
`arena_blocks` because these programs fit one block under either shape —
measured — and a check that cannot fail is worse than none. The block count
is pinned where it is sensitive, in `bench/cost_golden.txt`.

**Every vein the read's shape moved, named.** The trend gate reads this
paragraph, and each counter below moved for the one reason above — the read
wrapper's arms are code the compiler now writes and each program now runs.

The decoder's own emitted code: `emitted_defines` 168 → 175,
`emitted_calls` 1,820 → 1,863, `emitted_branches` 1,186 → 1,199,
`emitted_lines` 12,053 → 12,263. The eight programs beside it:
`emitted_other_defines` 1,468 → 1,492, `emitted_other_calls` 14,526 →
14,653, `emitted_other_branches` 8,673 → 8,713, `emitted_other_lines`
87,659 → 88,322. The machine code follows: `text` 1,010,214 → 1,021,094
across the eleven, jsonbench alone 83,938 → 86,418.

The runtime counters, the same +9 allocations and +288 bytes on each program
that reads a file, with the evacuation and dedup that come with the extra
dispatch: `allocs`, `alloc_bytes`, `evac_allocs`, `evac_bytes`,
`cohort_frees`, `carry_dedup` on the decode vein; `encode_allocs`,
`encode_alloc_bytes`, `encode_evac_allocs`, `encode_evac_bytes`,
`encode_cohort_frees`, `encode_carry_dedup` on encode; `oneshot_allocs`,
`oneshot_alloc_bytes`, `oneshot_evac_allocs`, `oneshot_evac_bytes`,
`oneshot_cohort_frees`, `oneshot_carry_dedup`; `wide_allocs`,
`wide_alloc_bytes`, `wide_evac_allocs`, `wide_evac_bytes`,
`wide_cohort_frees`, `wide_carry_dedup`; `digest_allocs`,
`digest_alloc_bytes`, `digest_evac_allocs`, `digest_evac_bytes`,
`digest_cohort_frees`, `digest_carry_dedup`.

The published figure moved with them: the landing panel and §04's golden
paragraph both quote the decode's allocation count, 4,999,958 → 4,999,967.

Three veins refuse from the container and say so in their own words —
`bench/instructions_golden.txt` (measured on glibc 2.39-0ubuntu8.8, here
8.7), `bench/compile_allocs_golden.txt` and
`bench/compile_memory_golden.txt` (rustc 1.98.1, here 1.94.1). Their rows
are copied out of the CI job log, which is what those refusals instruct.

Each of those, with the value it landed on, because the trend gate reads the
number and not only the name: `alloc_bytes` 259,660,208 to 259,660,496,
`evac_bytes` 112 to 496, `encode_alloc_bytes` 853,081,504 to 853,081,792,
`encode_allocs` 16,249,018 to 16,249,027, `encode_evac_bytes` 576 to 960,
`oneshot_alloc_bytes` 4,434,348 to 4,434,636, `oneshot_allocs` 79,361 to
79,370, `oneshot_evac_bytes` 96 to 480, `wide_alloc_bytes` 6,452,160 to
6,452,432, `wide_allocs` 144,020 to 144,029, `wide_evac_allocs` 244 to 256,
`wide_evac_bytes` 519,728 to 520,080, `digest_alloc_bytes` 54,149,841 to
54,150,129, `digest_allocs` 230,214 to 230,223, `digest_evac_bytes` 1,520 to
1,904.

**The eleven work rows, and why two of them fell.** CI counted the
instruction vein on this branch. Four rows rose: `work_jsonbench`
2,098,860,167 to 2,098,864,932, `work_encodebench` 5,846,994,767 to
5,847,000,948, `work_oneshot` 31,427,567 to 31,431,613, `work_digestbench`
81,252,316 to 81,256,613. Four to six thousand instructions each, on the
programs that read a file, once at startup — the same dispatch that shows as
+9 allocations in their cost goldens.

Two fell, and by more: `widebench` 59,506,462 to 59,384,053 and `deepbench`
676,465,730 to 675,925,724. deepbench imports no `std/os` and reads nothing,
so nothing in the io half can reach it. What reaches it is `src/runtime.c`,
which grew seventeen lines and is compiled into every program. Replacing
runtime.c with main's and rebuilding deepbench in the container reproduces
the fall to the instruction: 676,462,050 against 675,922,044, −540,006, the
same figure CI measured on a host whose absolute counts differ by 3,680.

widebench's −122,409 lands in the same code and will not decompose the same
way. Its beat tiers are identical on both trees, and callgrind names the
movers:
`k_copy_size'2` −47,468, `k_exec` −36,594, `k_copy_size` −31,665, `k_exec'2`
−15,942. All four are runtime.c functions this branch does not edit; the hot
kanso code — `d_widebench/value_for_3'2`, `render_ryu`, `k_ten_holds` — is
byte-identical. But swapping runtime.c alone accounts for only −16,007 of it,
because the runtime is compiled together with the program, and the emitted
wrappers the io half adds change what clang inlines from the runtime into
widebench. The two halves interact and are not separable by subtraction.

Four of the remaining five moved by a single instruction — `basket`,
`escapebench`, `indexbench`, `scanbench` — and `pendbench` fell 209.

**`compile_instructions` 41,829,232 to 41,830,604**, and the row it lands in
is a fresh one. The front end genuinely changed — `os/read_file` yields
`STR | NONE`, lib/os gained two arms, `THREADED` gained a set — so every
chip's row in `bench/compile_instructions_by_cpu.txt` went stale at once and
the two that were there are deleted rather than carried. CI landed on Zen 4,
which has no reading on the binary those two were counted against, so the
1,372 is a chip and a binary moved together and cannot be read as front-end
work. The other two chips are re-sittings when they next refuse, one per CI
run, which is the price this file's header already names for a keyed row.

Welfare 73.06, unmoved: 1,372 instructions on a term whose baseline is
57,029,831 is below the gate's own resolution.
