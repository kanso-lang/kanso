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

Three plain reads remain in the migrated code, each deliberately: the fixture
testing the argument check, hako's lock, and the book's showcase. Four more
arrived later on this branch and are the point rather than a remainder — the
`tests/golden/read_beat` fixtures, which exist to exercise the plain read's
type. Counted here because the sentence above was written before them and a
reader grepping for `os/read_file` finds seven.

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

CI then landed on Zen 3 and refused, which is the deletion working: no row,
no comparison. It counted 41,830,604 on sha d89bda86538a — `family0x19-
model0x11`'s value to the instruction — and I wrote the row and said the two
AMD models agree for the second consecutive binary.

**That was one reading, and the next one corrected it.** Nine minutes later
the same chip on the same binary counted 41,831,112:

    00:37:38  family0x19-model0x1  d89bda86538a  41,830,604
    00:46:23  family0x19-model0x1  d89bda86538a  41,831,112

508 apart, which is the residual this file's header records on the INTEL from
an entirely different binary — 41,831,767 and 41,832,275. Two vendors, two
binaries, the same gap. That is the strongest evidence the vein has that the
split is one mechanism rather than a coincidence of layouts, and it arrived
because a single pin refused a second reading instead of averaging it.

Where it lives, off the two profiles: every kanso frame is identical to the
instruction — `eval_expr'2` 1,633,593, `check_merged` 1,586,580, `infer`
1,238,613, `lex_line` 866,486, `parse` 589,004 — and the whole difference is
glibc, `_int_malloc` −580, `_int_free` −19, `__memcmp_avx2_movbe` +66. The
front end does the same work; the allocator walks a different heap. That is
what the header attributes this term to and what pinning the tunables did not
remove.

So Zen 3's row takes the pair and Zen 4's stays a single, because Zen 4 has
shown one mode on this binary and a pair there would be a prediction. The
Intel row is still absent and still wants its own sitting, one per CI run, as
the header priced it.

**And the prediction resolved itself within the hour.** Main's first run after
the merge landed on Zen 4 and counted 41,831,112 against its single — the
second mode, the same 508. So the pair was not a prediction there either; it
was one reading away, and the single refusing is what produced the second.

    family0x19-model0x11   41,830,604   and   41,831,112
    family0x19-model0x1    41,830,604   and   41,831,112

Four readings, two chips, one binary, and every one lands on one of two
values. Neither model has produced a third. The mode belongs to the run and
not to the silicon: the same silicon produces both, and different silicon
produces the same pair. What would settle it is a third value on any chip, or
one chip producing the same value twenty times running. The cap of two is what
refuses to quietly absorb the first if it comes.

Welfare 73.06, unmoved: 1,372 instructions on a term whose baseline is
57,029,831 is below the gate's own resolution.

## 2026-09-04 — the yield is carried per declaration, and the corpus was measuring its own workaround

**Searched first**, as the filing gate requires: design/compiler-log.md (the
2026-09-03 entry names this fix, counts the eight absent wrappers and defers
it), design/log/compiler-log-archive.md (2026-07-28 on `desc_yield`'s missing
arms, 2026-07-29 on closing that gap with an error-corpus program) and every
design/*.md. The fix below is the one the 2026-09-03 entry named and declined
to make in place.

**`desc_yield` answered from a table keyed on a chain head's bare name.**
`os/read_file` hit it because the wrapper's name collides with the builtin's;
`os/read_file!` — same body, one character more — missed and fell to the top
set, and so did seven other std effect wrappers. A loop past any of them kept
the grow-only arena. The yield is now a second per-declaration answer beside
`ctx.returns`: the fixpoint asks the body's tail what running the description
hands over, grows it monotonically, and wakes the same readers `returns` does.
A call in yield position consults it. The table remains, reached only by names
the program did not declare — true builtins.

**Watched red first.** `tests/golden/read_beat/reading_insisted.kso` is the
`os/read_file!` twin of `reading.kso`, same document and same loop:

    reading.kso           beat_iters=201
    reading_long.kso      beat_iters=801
    reading_insisted.kso  beat_iters=1      <- before
    reading_insisted.kso  beat_iters=201    <- after

**`bench/make_jsonbench` gets its main back.** Its comment said the `fed` pair
was written out because of this hole and would come out when the yield was
inferred, so it does: `os/read_file! "bench/large.json" . go`. jsonbench reads
`arena_blocks=2`, `arena_peak_bytes=2097152`, `beat_iters=151` — the same
numbers the workaround produced. **Every counter in every cost golden is
byte-identical.** That is the finding, not an aside: the corpus was measuring
the workaround, so it could not have scored the fix.

**The cohort fixture's comment was wrong about which cohort it pinned.**
`a_bound_branch_chosen_pipe_still_fires_the_cohort` asserted `cohort_frees=1`
and said the 1 was the decode proving its argument a string. It was the
*read*: `os/read_file!` is a qualified call crossing down into `os` with a
string argument, which is the license, and the decode was not licensed at all
because the bound `source` answered top. The two are told apart by what they
evacuate — the read's cohort copied 432 bytes, and a program whose source is
`io/stdin` on both arms reads `cohort_frees=1` with `evac_bytes=400144`, which
is the decode's alone. The count is 2 now and `evac_bytes` is pinned beside it
so the next reader cannot make the same mistake.

**And the second cohort costs that fixture something, with a thread left
open.** Its whole counter set: allocs 34 to 36, alloc_bytes 1,741,504 to
1,941,536, evac_allocs 15 to 19, evac_bytes 432 to 400,496 — while
`arena_blocks` holds at 3 and `arena_peak_bytes` at 3,297,184, byte for byte.
So the decode's cohort copies its 200 KB result twice to reclaim about a
megabyte, and the program's peak does not move, because a cohort reclaims at
the pop and the peak was reached before it. On a program that exits there the
copy buys nothing measurable.

That is a question about the survivor-ratio guard's threshold rather than
about this change: `2 * survivor > grown` keeps the region, and here 400 KB
against roughly 1.2 MB says copy. The license was always meant to reach a
decode whose argument is a proven string; what moved is that inference can now
prove it. Whether the guard should also weigh WHEN the reclaim lands is not
something this branch measured and is not asserted either way.

**Emitted code falls on three programs, and so does the machine code.** The
decoder: defines 175 to 174, calls 1,863 to 1,851, branches 1,199 to 1,196,
lines 12,263 to 12,218. pendbench and digestbench each lose one call and one
line, and those two are the inference change alone — they never touched
jsonbench's `fed`. `.text` falls on the same three and in the same
proportions: jsonbench 86,418 to 86,002, pendbench 83,538 to 83,474,
digestbench 102,802 to 102,738.

**Three things were found by CI rather than here, all one mistake.** The local
sweep ran nine runtime counter gates and three compile ones and stopped, on
the belief that was the set. The cost-goldens job also runs `machine_code.sh`
and `instructions.sh`, and named four red veins where two were expected; then
`book_check.sh` failed on two panels, `ch04/missing.out` and
`ch05/missing.out`, which print the same err whose birth line the comment
above shifted. Three goldens under `tests/` had already needed the same edit,
so the number of places that pin `os.kso:103` was four more than the three I
found by grepping `tests/`.

`.github/workflows/ci.yml` names FORTY-FIVE entry points. Twelve were being
run. All of them that can run in this container have now been run and are
green — every differential, the coverage and drift scans, the trend gate,
`utf8_differential`, `row_carries_the_objective`, `native_checksum` — and the
ones that cannot are the three host-gated compile veins and the ratchet.

The lesson is the one CLAUDE.md already states about kq's five veins, and this
is the kanso instance: the vein list is a file to read, never a set to recall.
Reading it is one grep and it would have saved three rounds.

**A second hole of the same shape, one level down.** `desc_yield_of` looks
through a binding to what the bound description yields, and it did that only
at the top of the expression: the `if` arm recursed into `desc_yield`, which
sees an identifier and gives up. So a chain head that was a bound local
answered and the same local inside a branch did not.
`tests/golden/read_beat/reading_branch.kso` binds two reads, branches over
them and pipes the result; it read `beat_iters=1` and reads 201 with the
recursion routed through the lookthrough. That routing measured 381
instructions CHEAPER than not doing it, which is below this row's own
resolution — so it is free, and the four recursive positions all take it.

**What it costs, measured rather than assumed.** compile_instructions
42,239,175 to 42,348,055 on this container, +0.2578%, same command as the
gate. Three shapes were tried:

    naive                              42,544,586   +0.723%
    one hash lookup, not two           42,483,163   +0.578%
    stop at the top of the lattice     42,389,340   +0.356%
    one wake per visit, inlined        42,348,055   +0.258%

By callgrind the residual is `desc_yield` +49,409 and the group lookup about
+18,000 (measured on the 42,348,436 reading, before the free lookthrough); `demand::analyze` +27,661 and the parser +15,773 are binary layout,
and this change touches neither of those files.

**A fourth shape was tried and is worse, which is the interesting one.**
Computing the yield only for declarations some yield position has actually
asked about — marking and waking on first ask — reads 43,390,986, a full
percent above the baseline and worse than doing it for everything. The wake
storm costs more than the walks it saves.

**`lib/os/os.kso` carried a claim this change falsifies, and correcting it
turned up a second thing.** `read_file!` is written as a chain off the builtin
rather than a call to `read_file`, and its comment gave the reason: a chain
whose head is a kanso function answered the top set. That is no longer true,
and measured — `read_file path . (r -> insisted path r)` reads
`arena_blocks=2` and `beat_iters=151` on jsonbench, the same as the shipped
spelling. So the shorter form is available now.

It is still not a one-line swap, and the benchmark could not have shown why.
`read_file` answers `file_not_found` where the builtin answers `none`, so
`insisted`'s first arm has to move with it; leaving that arm alone, a missing
file reaches the caller as a record rather than a failure, and the program
answers `length takes a list, string, or map, not os/file_not_found "…"` with
the err gone. jsonbench reads a file that is there, so it passed both ways.
The comment now says the constraint is retired and what moving the spelling
would take; the code is unchanged.

**CI's four sittings, and the three host-gated veins are pinned to them.**
The work vein falls on the same three programs the emitted count did, and by
amounts the size of the calls that went away:

    jsonbench   2,098,864,932 -> 2,098,864,471    -461
    pendbench     715,732,729 ->   715,732,721      -8
    digestbench    81,256,613 ->    81,256,592     -21

The other eight rows are byte-identical, which is the check that the fall is
these three programs and not the runner.

**compile_allocs rises by exactly five blocks, and the five are named.** 25,485
to 25,490, with 1,712 more bytes. `compile_rounds` holds at 40 and
`compile_passes` at 5, so the fixpoint does not iterate more — the five blocks
are the `decl_yields` vector, one per whole-program inference, and the front
end runs five. Measured in a worktree at `origin/main` against this branch on
the same host, and the baseline read 25,485 there, matching CI's golden to the
block: this counter is reproducible in the container even though its gate is
keyed to the runner's rustc.

**compile_instructions: 41,930,035 on family0x19-model0x1, sha 42283602b2c8.**
Against the pair that row held, +99,431, or +0.2377%. The container measured
this change at +108,880 on its own binary — two hosts, two absolute values,
one direction and the same order of magnitude, which is as much agreement as
this row admits.

**The second mode arrived on the next run, and the gap is not 508.** Same
chip, same binary sha 42283602b2c8: 41,930,035 then 41,931,559. That is
**1,524, which is 3 x 508 exactly**. Every reading this file had recorded put
the residual at one 508 — twice on Zen 3, and the header's Intel pair from a
different binary — so this is the first that puts it at three, and it says the
508 is a QUANTUM rather than a two-valued toggle. The row holds a pair of
measured points on a lattice, not two modes.

The profiles support that and nothing else. Across the two runs every kanso
frame is identical to the instruction — `eval_expr'2` 1,652,497, `check_merged`
1,586,580, `infer` 1,251,874, `lex_line` 866,486, `parse` 589,004 — and the
only visible move is `__memcmp_avx2_movbe` 1,356,959 to 1,357,025, +66. The
remaining 1,458 sits below callgrind's 90% cut. The front end does identical
work; glibc walks a different heap.

**What the cap of two now means here, and it is a question rather than a
settled thing.** If the residual is a lattice, a third point is likely rather
than surprising, and the cap will refuse it. That refusal is the finding to
record and not a licence to widen: a row admitting every multiple of 508 pins
nothing. What a third reading would actually raise is whether this term can be
pinned per-chip at all, or whether the quantum has to be subtracted before
comparison — and that is for the log and for Clay, not for the file's shape.
Proven on the branch: the gate accepts both measured values and refuses
41,932,067, which is one more 508.

**Both per-cpu rows went stale and only one was re-sat, so the other is gone
rather than carried.** Zen 4 has not been counted on this binary, and keeping
its old pair would pin two numbers nobody measured for a compiler that no
longer exists. Removing the row makes the gate refuse on that chip and print
the sitting, which is what it already does for the Intel key. Proven on the
branch: the gate accepts 41,930,035 on Zen 3, refuses a fabricated second
value, and refuses the now-unkeyed Zen 4.

**Welfare falls about 0.008 and the reason is the corpus, not the weights.**
Filed as a pending gavel — "The welfare model cannot see the yield hole,
because the corpus was written around it" — with the recommendation to ship
and move the floor, and with the honest open question stated: whether "the
corpus is blind here" may move a floor at all, or whether the corpus change
has to come first.

## 2026-09-04 — the per-declaration yield reaches as far as the declarations, and five builtins were past the end of it

**Searched first**, as the filing gate requires: design/compiler-log.md (the
entry above, which is the one being corrected, plus the 2026-09-03 entry it
answers), design/log/compiler-log-archive.md (2026-07-28 on `desc_yield`'s
missing arms and 2026-07-29 on the error-corpus program written to close that
gap — neither reaches the builtin table's completeness) and every design/*.md.
Nothing rules on what follows.

**CORRECTS the entry above.** It says eight std effect wrappers fell to the top
set and that carrying the yield per declaration fixes them. Five of the eight
it does not fix, and the reason is structural rather than an oversight in the
list: the per-declaration answer reaches a wrapper written *over a declaration*.
`net/listen at` is `builtin_listen at . held`, and `held` is a group the
fixpoint has walked, so the wrapper answers from its own body. `net/read c` is
`builtin_net_read c.handle` with nothing after it. There is no declaration to
ask, so the builtin table is still what answers — and the table did not name
`net_read`, `net_port`, `accept`, `listen` or `kill`.

The entry above also says "the table remains, reached only by names the program
did not declare — true builtins", which is exactly right and is the sentence
that should have prompted the check below. It did not.

**Watched red first**, over a real socket rather than a hand-built handle: a
server that reads a request and threads it through a loop that allocates.

    beat_iters=0      <- before, on branch head 48ac2f93
    beat_iters=200    <- after

200 and not 201: the loop runs inside a fiber the scheduler resumed, so the
outermost bracket a whole-program loop gets is not there to count. The fixture
is `A_REQUEST_THROUGH_A_LOOP` in tests/sockets_serve.rs, which already owns the
port-file handshake and the serialising lock every socket test needs.

**The yields, read off what the executor actually answers** (`src/eval.rs`,
the `Desc` arms): `Receive` answers `Value::Str`, so `net_read` is STR;
`SocketPort`, `Accept` and `Listen` answer `Value::Int`, so those three are
INT; `Kill` answers `Value::NoneV`, which is the same nothing `Send`,
`CloseSocket`, `Write` and `WriteFile` answer, so `kill` joins the group the
table already scores 0.

**And the table is checked now, so this cannot recur quietly.**
tests/every_effect_builtin_says_what_it_yields.rs reads BOTH lists out of
src/infer.rs — the arm of `builtin_returns` that answers `DESC | fails`, plus
`print`, which is typed on its own beside `err` — and asserts every one of the
twenty-two is named somewhere in `desc_yield`. Watched red by deleting
`net_read`'s arm: *these builtins answer a description and desc_yield does not
say what they yield: ["net_read"]*. A second spec pins the twenty-two by name,
so a reading that silently finds nothing is a failure rather than a pass.

**What it costs.** `compile_allocs` is 25,490 before and after — identical, to
the block. Retired instructions on the container, same box and same tunables as
the gate: 42,344,047 to 42,344,081, **+34**, and attributable rather than
layout: `desc_yield_of` falls 42 as the `start` arm collapses into a `matches!`,
and `desc_yield::base` rises 69 as two new arms call it. +34 is one fifteenth
of the 508 quantum the entry above recorded, so CI's own pair will move by
whatever it moves by and has to be re-sat from the job log. Every runtime
counter is untouched — no benchmark in the corpus opens a socket or kills a
process.

**What is measured and what is not.** The fixture measures `net/read`, and
that is the one of the five with a heapish yield: a string is what `beat.rs`
has to prove before it will carry a slot across a rewind. `net_port`, `accept`
and `listen` yield an int and `kill` yields nothing, so none of the four has a
carry decision to change and no fixture here claims one. They are in the table
because the completeness spec requires every builtin that answers a description
to have an answer, and because a name absent from the table answers the top set
rather than nothing — which is a wrong answer whether or not anything currently
reads it. That is the whole of the case for those four; it is not a measured
win and is not written as one.

**What the bracket is worth, measured but NOT pinned.** A server reading a
400 KB POST and threading it through sixty iterations, on `48ac2f93` against
`40aec705`:

    arena_blocks         182  ->    4
    arena_peak_bytes     190,840,832  ->  4,194,304
    beat_iters             0  ->   60

45x the peak, which is the socket twin of the file read's 260 MB against 2 MB.
Three runs of each shape agreed to the byte in the scratch directory, and a
golden pinning those numbers was written and then **taken out again**: the same
program inside the test harness read `tallied 120` rather than `tallied
3923160`. One `net/read` is one `recv`, so how much of a 400 KB body has
arrived when the server reads decides what the loop sees, and it differs
between a shell and a spawned child on a loaded machine. Three agreeing runs
were not evidence of determinism, they were evidence of one machine state.

So the corpus keeps the small-request `beat_iters` assertion, which is
timing-independent because the whole request fits one `recv`, and the 45x above
stays a measurement in this entry rather than a number CI diffs.

**This is not a gap in the library, and the fixture's shape is why it looked
like one.** `lib/net/http` already reads until the request is whole:
`heard`/`joined`/`gathering` call `net/read` again on a short segment and stop
when the head has landed and as many body bytes as content-length promised,
measured by subtraction so a body containing a blank line survives. Its own
comment says it — "a read is a segment, not a request". The probe called
`net/read` directly, one layer under that, which is the right layer for asking
what a bound read yields and the wrong one for reading a request. A program
that wants a whole request has a verb for it.

**The compile row lands at 41,845,704, and the number is not comparable to
the one it replaces.** CI refused, as expected, and the run that refused sat
on Zen 4 (family 0x19 model 0x11) where the previous sitting was Zen 3
(0x19/0x1). So `compile_instructions` goes 41,930,035 to 41,845,704 in
`bench/compile_instructions_by_cpu.txt`, and the −84,331 is a chip change with
this branch's +34 somewhere inside it, not a fall the compiler earned. Zen 4
had no row since it was cleared on 2026-09-03 for having no reading on the
previous binary; this is its first on this one, which makes it the reference
row and moves the golden's bare line and compiler.html's figure with it.

Zen 3's pair is removed rather than carried. Both its values were measured and
both were measured on a binary 34 instructions away from this one, which is the
same reason Zen 4's pair went yesterday. It is a re-sitting when it next
refuses.

**NEGATIVE RESULT — the fix does NOT reach `http/serving`, and the guess that
it would was worth checking.** The gather loop binds what `net/read` yields, so
it looked like the shipped server had been on the grow-only arena for as long
as the table was missing `net_read`. Measured, the same POST through
`lib/net/http` on `48ac2f93` and on this head:

    arena_blocks           6  ->  6
    arena_peak_bytes  6,291,456  ->  6,291,456
    beat_iters             0  ->  0
    allocs             2,595  ->  2,586

`beat_iters=0` on BOTH sides is the answer: that loop never bracketed, so there
was no bracket for the missing yield to cost. It is not a self-recursive
accumulator of the shape `beat.rs` brackets — `gathering` reaches `heard`
reaches `joined` reaches `gathering`, through a continuation each time. The
nine allocations are real and are all the fix is worth there.

So the 45x above is what a program written directly against `net/read` pays,
and the library's own server was never paying it. Written down because the
opposite is the natural assumption and nothing in the tree would have
contradicted it.

**AND THE RE-SIT MOVED WELFARE, WHICH IT SHOULD NOT BE ABLE TO DO.** Measured
by running `scripts/welfare` against each value the row has carried today, with
nothing else changed:

    compile_instructions=41930035   welfare 73.05
    compile_instructions=41931559   welfare 73.05
    compile_instructions=41845704   welfare 73.06

**WHAT MOVED IT, CORRECTED.** This was first written up as a chip change,
because the run that refused sat on Zen 4 where the previous sitting was Zen 3,
and the correction is the next run: it sat on **Zen 3** and counted 41,845,704
as well. Same chip as the 41,930,035 sitting, different binary — sha
42283602b2c8 then, sha 0e081d4c2c96 now — and **-84,331 for a source change
the container measures at +34**. The two AMD models agreeing to the instruction
on this binary is what hid it for one run.

So the term that moved is binary layout, not silicon, and it moved the row by
2,480 times the front-end work the change actually did. This file's header
already names layout — a docs-only pull request once moved this row 5,081 —
and this is sixteen times that.

**That makes the second gavel question sharper rather than weaker, and it is
still not one I answer.** The original framing (which chip CI drew) was wrong.
The real one: `compile_instructions` moves by layout far more than by
front-end work, and the welfare floor ratchets against the number that
contains both. 0.01 of welfare — the same size as the fall this branch is
blocked on — was bought here by a relink. Filed in design/pending-gavels.md
with the corrected evidence; nothing changed on it.

**THE 508 LATTICE REAPPEARS ON THIS BINARY, and Zen 4's row is a pair.** The
run after the Zen 3 sitting was Zen 4 again and counted 41,844,180 where the
row pinned 41,845,704 — same chip 0x19/0x11, same binary sha 0e081d4c2c96. The
gap is 1,524, three 508s, and the profiles say what they said the last time
this happened: every kanso frame identical to the instruction across the two
runs, and only `__memcmp_avx2_movbe` moving, 1,356,842 against 1,356,776, which
is 66.

Gaps of 508 (twice, in this file's history) and of 1,524 (twice now) have been
read, and every one is a multiple of 508. That is the lattice. Two 1,524s in a
row is not evidence of a fixed separation and nothing here claims one.

The cap of two binds and Zen 4 holds both measured values. Verified locally
that the gate accepts either on that key, accepts Zen 3's single, refuses a
third value one more 508 down, and refuses Zen 4's second value on Zen 3's key.

**AND CI HAS BEEN RUNNING A NINE-BINARY PREFIX OF THE SUITE, on both hosts,
for every run of this branch.** `cargo test` stops at the first failing test
BINARY rather than the first failing test, and the binaries run in alphabetical
order. `a_granted_baseline_says_it_is_one` is red for the welfare fall, and it
sorts ninth. So `specs` and `the other host (macos, arm)` were both running
`a_bare_list…` through `a_gate_red_before…` and stopping — and reporting that
as the suite.

Found by reading the arm job's log to check whether the new socket spec passed
there. It had never run. `tests/sockets_serve.rs` sorts long after the letter
a, so the fixture this entry is built on has not executed on arm once, and
nothing anywhere said so: the job was red for the reason everybody expected and
silent about the coverage it had stopped providing.

Both jobs take `--no-fail-fast` now. A red suite that hides the rest of itself
is the same fault as a green one that proves nothing, and this one hid about
ninety binaries behind one expected failure. The exit code is unchanged — a
failure anywhere still fails the job — so the only difference is what a reader
of the log can see.

**THE RATCHET CANNOT GO GREEN WHILE WELFARE IS RED, and that is by design
rather than a second fault.** Its job log says so exactly: `ratchet: the
baseline is not green` … `ALREADY RED welfare … red before any mutation, so no
row sharing it is proof`. That is
`a_gate_red_before_the_mutation_is_refused_rather_than_credited` doing what it
was built for — a mutation cannot be credited against a gate that was failing
before it was applied. Seventeen rows are selected on this branch because it
touches files many mutations patch, and every one of them shares the welfare
gate.

So the red count on this branch is not a list of problems. `welfare`, `specs`,
`the other host (macos, arm)` and `the ratchet` are one cause with four faces,
and the fourth is downstream of the first three rather than beside them.
Written down because the ratchet's red looks like a new failure every time and
is not one.

**THE SOCKET SPEC PASSES ON ARM, and that is the first time it has run
there.** The arm job on `d11470a3` — the first head carrying `--no-fail-fast`
— ran the whole suite and ended with

    error: 2 targets failed:
        `--test a_granted_baseline_says_it_is_one`
        `--test the_digest_is_priced_on_both_sides`

which is the two welfare tests and nothing else. `beat_iters=200` holds on
aarch64 as it does on x86_64, and so does the rest of `tests/sockets_serve.rs`
— the port handshake, the serialising lock, the curl client. The counter is a
property of the bracket rather than of the architecture, which is what it was
supposed to be and was not evidence for until now.

It also says the flag was worth its cost. The arm suite took about eight
minutes where the nine-binary prefix took under one, and what the extra seven
bought is the knowledge that exactly two targets fail — the same two the local
`--no-fail-fast` run found. Before, that agreement was untested on one of the
two hosts.

**OPEN — the corpus still cannot see this class of fix.** Same shape as the
gavel this branch is waiting on. The five builtins are absent from every
benchmark, so a change that takes a socket read from a 260 MB peak to 2 MB
scores exactly zero and pays 34 instructions. The socket golden pins the
behaviour, which is what the goldens rule asks for; it does not put the
dimension in front of welfare, and nothing here proposes that it should.

**A THIRD CHIP LANDED ON THIS BINARY AND COUNTED THE SAME NUMBER.** The `cost
goldens` job on `132a0c3a` refused, and the refusal names a key the table did
not hold:

    silicon: cpu family 0x6 model 0xcf
    compile_sample cpu="cpu family 0x6 model 0xcf" sha=0e081d4c2c96 row=41845704
    nothing in bench/compile_instructions_by_cpu.txt was counted on family0x6-model0xcf

41,845,704 is what Zen 4 counted on that binary and what Zen 3 counted on it.
Three silicon keys, one binary sha, one value to the instruction. The row is
added and the note beside it says so.

Emerald Rapids is not new to the pool — it held `41,831,767 41,832,275` on sha
de5bfab22fbd, the pair `scripts/gates/compile_ir_row.sh` still uses as its
worked example, and lost the row when the binary moved.

**IT SHARPENS THE CORRECTION THIS ENTRY ALREADY CARRIES.** The file is keyed by
silicon on the strength of two readings about 5,124 apart. Those are from
`f6e24e91`, and what the header claims identical across them is the SOURCES:
`compile_sample`'s sha landed in that same commit, so no reading from before it
pairs a chip with a binary at all. Meanwhile the layout term measured here is
-84,365, sixteen times the gap the key was built on. Walking every recorded
state of the rows, each one carries a single value or a single pair across all
its chips:

    f6e24e91  0x6/0xcf 41500974  |  0x19/0x1 41495850  0x19/0x11 41495850
    bbbcdc90  three chips at 41831767 41832275, a fourth at 41832275
    7110a2e6  0x6/0xcf 41829232  |  0x19/0x1 41829232
    3b9df304  0x19/0x11 41830604 |  0x19/0x1 41830604 41831112
    now       0x19/0x11 41845704 41844180 | 0x19/0x1 41845704 | 0x6/0xcf 41845704

The first row is the only state where two chips disagree, and it is the state
whose binaries nobody recorded.

**WHAT IS NOT ESTABLISHED, AND WHY THE KEY STAYS.** The row is no more a
function of the binary than of the chip: Zen 4 read 41,844,180 on this same
sha, so something moves the count inside one chip and one binary. Three
agreeing chips say the chip term is small on this binary; they cannot say it is
zero on another, because the readings that would settle it were taken before
anything wrote the sha down. Removing the key on this evidence would be trading
a measured guard for an inference. It goes to the gavel already open on this
row instead, as a second question under the same heading.

**THE LAYOUT TERM, MEASURED ON PURPOSE INSTEAD OF INFERRED — AND IT CORRECTS
THE NOTE ABOVE.** The claim that the row's founding readings are confounded by
binary layout rested on the header's sentence that cargo does not build the
same bytes twice. On this host it does. Two from-scratch builds of an unchanged
tree into different target directories:

    9fcc6686dc47  .text=2550854 .data=2640 .bss=312
    9fcc6686dc47  .text=2550854 .data=2640 .bss=312

Byte-identical. So a pull request touching only `docs/` and `design/` — neither
of which any `include_str!` reaches; only `lib/**`, `hako/**` and
`src/runtime.c` are baked in — produces the same binary, and the 5,081 the
header attributes to "an edit the compiler cannot see" is NOT layout. It is the
chip, or the mode below. That reading supports the per-silicon key rather than
questioning it, and the note above is wrong to have leaned the other way.

**WHAT DOES MOVE IT.** Same procedure as the gate minus the host stop, which
hard-refuses on a container, so these are within-container comparisons of seven
binaries on one chip:

    sha           .text     .data  .bss    instructions
    9fcc6686dc47  2550854   2640   312     42,344,081   baseline
    82ec0846958a  2550854   2640   312     42,344,081   dead pub fn, linker dropped it
    5d50f9d9721d  2550854   2640   312     42,344,081   no_mangle fn, dropped too
    8663815286be  2550854   2640   1336    42,344,081   +1 KiB .bss
    09a6c2fab6b8  2550854   2640   312     42,344,093   +64 KiB .rodata
    7fc53be7987e  2550854   2640   4408    42,346,211   +4 KiB .bss
    3c1e1cff9e3b  2550854   2640   65848   42,346,211   +64 KiB .bss

Three different shas with unmoved sections read one value to the instruction, so
a relink alone is not the term — the sections have to move. 64 KiB of read-only
data costs 12. Growing `.bss`, which is the mechanism the gate's own comment
names, costs 2,130 — and costs the SAME 2,130 at 4 KiB as at 64 KiB, while 1 KiB
costs nothing.

**SO THE ROW IS BIMODAL BY CONSTRUCTION.** Seven binaries produced two values,
42,344,081 and 42,346,211, and where a binary lands is decided by whether its
`.bss` crosses a boundary between one page and four. That is the shape this vein
has been reporting for a fortnight from the other end — the two clusters 5,064
apart on one unchanged binary, the 508 lattice, Zen 4's pair on one sha. This is
the first time the flip has been produced deliberately, with the compiler's work
held fixed.

**WHAT IT SETTLES FOR THE GAVEL, AND WHAT IT DOES NOT.** Welfare reads this row
as a ratcheted magnitude, and 2,130 of it can be bought or lost by moving a
static nobody executes. That half of the open entry is measured now rather than
argued. It does not explain the -84,331 recorded earlier in this entry: that is
forty times this probe's step, between two binaries whose `.text` differed by a
real source change, and nothing here shows a shift that large. One chip, one
glibc, one container — CI's hosts are not this host, and the numbers above are a
demonstration of sensitivity rather than a calibration of it.

**TEN BINARIES, AND THE TWO CLAIMS ABOVE ARE BOTH WRONG.** Growing `.text`
alone was never tested — every probe above left it at 2,550,854, because the
linker dropped each dead function. Reaching one through an environment variable
the gate never sets (it runs `env -i`) keeps it, and the picture changes:

    sha           .text     .data  .bss    instructions
    9fcc6686dc47  2550854   2640   312     42,344,081   baseline, three runs
    82ec0846958a  2550854   2640   312     42,344,081   dead pub fn, dropped
    5d50f9d9721d  2550854   2640   312     42,344,081   no_mangle fn, dropped
    8663815286be  2550854   2640   1336    42,344,081   +1 KiB .bss
    09a6c2fab6b8  2550854   2640   312     42,344,093   +64 KiB .rodata
    5e73453bcc7b  2550950   2640   312     42,343,660   200 unreached fns
    2152c689dc78  2566982   2640   312     42,345,628   400 unreached fns
    2a4e10fb2116  2550950   2640   312     42,345,904   100 unreached fns
    7fc53be7987e  2550854   2640   4408    42,346,211   +4 KiB .bss
    3c1e1cff9e3b  2550854   2640   65848   42,346,211   +64 KiB .bss

**The measurement is deterministic per binary** — the baseline read 42,344,081
three times over — so every difference here is a property of the binary and
nothing else.

Retracted with it: "a relink alone is not the term, three shas with unmoved
sections read one value to the instruction." Four binaries agreeing was luck.
`2a4e10fb2116` and `5e73453bcc7b` have the same `.text`, `.data` and `.bss` to
the byte and read **2,244 apart**, so the triple the gate prints does not
determine the value. Retracted too: "bimodal by construction." Ten binaries gave
six values across a span of 2,551.

**WHAT STANDS, AND IT IS THE STRONGER STATEMENT.** The row is a deterministic
function of the binary and of nothing the gate can see about the binary, and a
source change that does no new work moves it by up to 2,551 — the size of
kanso#1226's -5,621, which is a real change this vein exists to catch. It moves
in both directions: `5e73453bcc7b` reads 421 BELOW the baseline for nothing but
two hundred functions no execution reaches. A ratchet would bank that as a win.

The mechanism is not chased here. Callgrind counts instructions rather than
cycles, so layout cannot move the count directly; something upstream — the heap
break, glibc's allocator paths, what the loader maps — has to be doing it, and
naming which would want a separate sitting.

**THE MECHANISM, NAMED AND ACCOUNTED TO THE INSTRUCTION.** callgrind's call
graph answers it: `std::rt::lang_start_internal` calls `pthread_getattr_np`,
which parses `/proc/self/maps` with `getline` and `sscanf` to find the stack
bounds for Rust's guard page. That parse is inside the row.

Splitting each profile into the parse and the program — `pthread_getattr_np`
inclusive, and `std::rt::lang_start::{{closure}}` inclusive, which is everything
the compiler actually does:

    binary                      row          maps parse   the program
    9fcc6686dc47 baseline       42,344,081      112,580    41,878,959
    45c6dbed10bb +64 KiB .bss   42,346,211      114,710    41,878,959
    2a4e10fb2116 100 fns        42,345,904      112,586    41,880,776
    5e73453bcc7b 200 fns        42,343,660      110,317    41,880,801

**The `.bss` probe adds no code, and the compiler's work is identical to the
instruction — 41,878,959 both times. All 2,130 of the row's move is the parse.**
The two function probes differ from each other by 2,269 in the parse and 25 in
the program, which is the 2,244 above. They sit 1,817 over the baseline's
program because the probe branch calls `std::env::var_os` on every start, so
those two were never work-free and the earlier entry should not have called
them that; the `.bss` and `.rodata` probes are the clean ones.

**WHAT THAT MAKES OF THE PREVIOUS ENTRY.** "A source change that does no work
moves the row by up to 2,551" stands, and now has its cause: a different binary
gets a different `/proc/self/maps`, and glibc's parse of it costs a different
number of instructions. Nothing about the front end changed in any of it.

**THIS IS A TERM THE RECORD ALREADY KNOWS.** kanso#1234 found the compile row
counting glibc's startup parse of `/proc/self/maps`, and the ruling of
2026-09-03 was NO EXCLUSION — the toggle was dropped and sorts plus `setarch`
shipped instead. So nothing here proposes excluding it, and nothing is changed.
What is new is the size: the parse is 0.27% of the row and 100% of its
binary-to-binary drift, and `lang_start::{{closure}}` is a counter that sat
still through a change that moved the published row 2,130. That is a fact the
ruling was made without, so it goes to the gavel rather than into a gate.

**AN OPEN THREAD THAT IS ALREADY CLOSED.** The 2026-09-01 entry leaves open that
`bench/widebench/widebench/` and `bench/encodebench/encodebench/` vendor the
json library, differ from `lib/json`, and that "nothing in the tree says so."
The second half stopped being true in kanso#1231: both directories have a
README saying the copy is frozen deliberately, naming the sha it was taken at
(919d2ef3 and 20ab931d), what the freeze buys, and what it therefore cannot
see. Walking the log's open threads found this one and re-derived it from
scratch before reaching the READMEs, which is the cost of a thread that closed
without being marked.

Their figure holds too. Both say the copies "differ from lib/json by 216
lines", and diffing the five shared files today gives 113 lines only in
lib/json and 103 only in the bench copies. `lib/json` has not moved since
c8442597, so the number the READMEs quote is still the number.

The question the entry raised — whether a benchmark that vendors a library
should track it — is answered there too, and against tracking: kanso#1230
shipped a library change and a codegen change together, and the frozen
benchmark is what separated them.

**THREE MORE OPEN THREADS, TWO OF THEM CLOSED, AND ONE CREDIT TO CORRECT.**
Walking the rest of the live log's `OPEN` markers:

**The ninth entry named `/proc/self/maps` before I did.** It reads: "glibc
parses `/proc/self/maps` before `main` to find the stack bounds, one more
shared library in the process moves the row 32,090, and that cost belongs to
the host's memory map rather than to the compiler." Today's work established
that; it did not find it. The entries above are written as though the mechanism
were new, and it was the leading candidate on the record, un-established. What
is added is the establishment and the size — the profile split, `program`
holding at 41,878,959 to the instruction across a 2,130 move — and the caller,
which is Rust's `lang_start_internal` placing its stack guard rather than glibc
before `main`.

**The eleventh entry's test is answered, in its second branch.** It asked
whether the other chips land on their own values and hold them, or whether a
second value appears on a recorded chip. Both were seen: three silicon keys
counted 41,845,704 on sha 0e081d4c2c96, and Zen 4 read 41,845,704 and
41,844,180 on that same sha. By the entry's own reading, a second value on a
recorded chip means neither earlier suspect was the term and the maps parse is
what remains, which is what the split measures. Its first branch does NOT
follow — chips agreeing on one binary is a different observation from chips
holding their own stable values, and the file's history has a state where two
disagreed on binaries nobody recorded.

**The twelfth entry's question is still open as asked.** It wants two RUNNERS
shown to differ in their maps by the 508 the row needed, and proposes printing
the map's line count. Today's evidence is binary-to-binary on ONE host, so it
answers the same question one level down and leaves that one standing.

**The rewiring thread is closed by a ruling, not by work.** The `--toggle-collect`
entry owed a guard and a welfare re-baseline; the ruling of 2026-09-03 was no
exclusion and the toggle was dropped. `grep` finds no trace of it in `scripts/`
or `.github/`, so nothing is owed and the marker should not send another reader
after it.

**Still genuinely open:** `bench/instructions_golden.txt` is not keyed per
silicon the way the compile row is, and there is no `by_cpu` file beside it.

**THE WORK VEIN CARRIES NONE OF THIS TERM, so the last open thread is a
different question.** `bench/instructions_golden.txt` is not keyed per silicon
and its eleven rows have disagreed between sittings, which invites reading the
compile row's treatment across. The mechanism does not carry across. Profiling
`jsonbench` the way the gate does and grepping both profiles for the parse:

    symbol          compile row   jsonbench
    getattr_np           2             0
    lang_start           2             0
    vfscanf              2             0
    getdelim             2             0
    sscanf               2             0

The benchmarks are C the compiler emitted, linked natively. Nothing in them
installs a Rust stack guard, so nothing parses `/proc/self/maps`, and the term
that is 100% of the compile row's binary-to-binary drift is absent from the
eleven work rows entirely. Whatever moves them between sittings is something
else, and the compile row's per-silicon key is not a fix to copy over on the
strength of today's finding.

Recorded and left there. Saying what DOES move them wants two runners, which is
CI's to give.

**ZEN 3 IS A PAIR TOO, AND IT IS THE SAME PAIR — WHICH CORRECTS WHAT I TOLD
THE PULL REQUEST.** I reported `cost goldens` green on `c1422f27` and
`0119f95a`. The second is wrong: the success I read was run 33839339418, whose
head is `4d34057e`. The job actually went green on `c1422f27` and `4d34057e`
and RED on `37138246` and `0119f95a`, and the two reds are one finding.

Both refused on `family0x19-model0x1`, binary sha `0e081d4c2c96`, counting
41,844,180 against the 41,845,704 the row pinned. Different runner machines,
one chip key, one binary, gap 1,524 — the same 1,524 Zen 4 shows on this
binary and the same multiple of 508 the lattice has produced every time. Zen
3's row gains its second value, which the cap of two allows, and both AMD keys
now hold the identical pair on the identical binary.

The profiles agree. Every kanso frame is the same to the instruction across the
two modes, and `__memcmp_avx2_movbe` reads 1,356,776 on the runs that counted
low against 1,356,842 on the Intel run that counted high, which is 66 — the
same 66 recorded when Zen 4's pair appeared.

**AND IT REACHES THE TWELFTH ENTRY'S QUESTION FROM THE OTHER SIDE.** That entry
wants two RUNNERS shown to differ, and said today's binary-to-binary evidence
left it standing. Two runners of one chip key, on one binary, have now produced
the two modes. Combined with the split that puts the whole mode difference in
`pthread_getattr_np`, the reading is that their maps differ. It is an inference
and stays one: nothing has printed a map's line count on two runners and
compared them, which is still what would settle it, and
`scripts/compile_row_probe.sh` now prints the term a runner would have to show.

## 2026-09-04 — THE CORPUS WAS WRITTEN AROUND THE HOLE, SO THE OBJECTIVE PRICED THE REPAIR AT ZERO

**DONE.** `bench/readbench` joins the objective, and the yield repair is worth
**+2.69 welfare** where it was worth 0.000 an hour ago. Clay's ruling, verbatim:
"that's just saying that your welfare metric should be incorporating that
metric and it's not so you need to fix that right?"

**WHAT THE CORPUS COULD NOT SEE.** `os/read_file!` fell to the top set because
`desc_yield` read a chain's yield off the head's BARE name, so a loop past it
lost its beat and ran on a grow-only arena. `bench/make_jsonbench` was written
around exactly that: the bang was spelled out into a `fed` pair, with a comment
saying it would come back when the yield was inferred. It did, in 62879e23 —
and every counter in every cost golden was byte-identical, because the corpus
had been measuring the workaround. Eleven benchmarks and not one of them read
a file with the spelling the repair fixes.

**THE BENCHMARK IS THE FIXTURE AT SIZE.** `tests/golden/read_beat/reading.kso`
already did this at 200 rounds; readbench is the same loop over
`bench/large.json`. Two compilers, same program, same bytes:

    159f6b2b  arena_blocks 41  arena_peak_bytes 42,991,616  beat_iters 1
    this head arena_blocks  1  arena_peak_bytes  1,048,576  beat_iters 201

The round count is a free parameter and that is a hazard worth stating: the
defective side is linear in rounds and the fixed side is flat, so the size of
the ratio is a number somebody picks. It was picked by matching the fixture,
before the score was computed, and not adjusted afterwards.

**THE TWO MEMORY BASELINES ARE MEASURED, NOT GRANTED.** welfare.kso's entering
rule puts a newcomer at its dimension's standing so landing day is never a
score move, and its own comment leaves open "whether a granted baseline should
be replaced by real history once the counter has some." This counter has some:
41 blocks and 42,991,616 bytes, read off 159f6b2b today. Granting them would
have scored the repair at zero a second time, which is the whole thing being
fixed. They go into `bench/welfare_floor.json` by hand, so a reviewer sees them
in the diff, and the report's granted list no longer names them.

`read_instructions` IS granted, and the reason is that it has no history worth
recording: 2,000,668,271 against 2,000,657,408, a fall of 10,863 or 0.0005%.
The row is in the objective as a tripwire — the benchmark is here for its
arena, and this is what stops a later change buying that arena back with
instructions.

**WHAT THE NUMBER DOES.** Both trees scored with the same model:

    159f6b2b (the defect)   70.83
    this head (the repair)  73.52

So the floor's 73.06 was set on a model that could not see this dimension, and
73.52 - 73.06 is not the size of the repair. The repair is 2.69; the model
gaining a term it was blind to is worth -2.23 on the old tree. Both belong in
the floor's `why` and neither is the other.

**THE FOLD IS A NEGATIVE RESULT.** Before the ruling arrived the plan was to
dissolve the fall by removing the five allocations `decl_yields` costs —
`returns` and `decl_yields` folded into one `answers: Vec<Set>` of length 2n
with an `nfns`, since both are asked in the same round, grow monotonically,
travel the same edges and wake the same readers. It works: `compile_allocs`
returns to 25,485 exactly, `compile_alloc_bytes` and `compile_peak_bytes` hold
to the byte, rounds and visits are identical, and the yield fixtures pass. It
also costs **9,562 instructions**, measured with `scripts/compile_row_probe.sh`
on two binaries: `program` (the `lang_start::{{closure}}` inclusive count, the
compiler's actual work) 41,878,959 -> 41,888,521, with `maps` identical at
112,580 both sides, so the whole move is the fold and none of it is layout.
Priced: 73.060846 unfolded against 73.060572 folded. The objective declines it,
and the index computed to five places is what says so — the allocation counter
alone would have accepted it.

**AND A HOLE FOUND ON THE WAY.** `scripts/gates/scan_counters.sh` exists and no
CI step runs it: grep for it across `.github/` returns nothing, while
oneshot, basket, wide, pend, escape, digest and decode all have steps.
scanbench's `arena_blocks` and `arena_peak_bytes` are both in the model, so
`bench/cost_golden_scan.txt` is a golden nothing compares against. Not widened
into this change; recorded here and queued.

**ADDED LATER THE SAME DAY — the obvious mutation proves nothing.** The gate
shipped with a hand-run refusal behind it and no ratchet row, which is the
scan_counters hole in mirror image: that one had a row from the first and no CI
step ran the gate. Both are closed now, and writing the mutation turned up
something the entry above got wrong by implication.

Dropping `| ctx.decl_yields[i]` from `call_yield` — the per-declaration yield
join, the thing 62879e23 was about — leaves readbench reading arena_blocks 1,
arena_peak_bytes 1,048,576 and beat_iters 201. Unchanged. Built and measured
rather than assumed, which is the only reason it was caught: the edit looked
like the mutation and would have shipped as one.

What readbench pins is the GROUP CONSULT — a chain head naming a declaration
the program made, asked what that declaration yields. Short-circuit it and the
benchmark reads 41, 42,991,616 and 1, the defect's numbers to the byte on a
compiler that has the repair. So the benchmark and the yield table cover
different halves: `os/read_file!` returns something that already carries its
type and is answered by its own declaration, where `net/read c` is a bare
builtin call with nothing to ask and needs the table. The socket fixture is
still the only thing pinning the second half, and this does not change that.

## 2026-09-04 — THE ROW NAMED THE INDEX IT WAS READING, NOT THE TOOL THAT FAILED

**DONE.** `perf_record` asks welfare for the score and takes the second field of
its first line. When welfare itself died it printed no such line, and the row
builder reported `missing index 2` born in `score_in` — a message naming the
reader instead of the tool it read, pointing at a file nothing was wrong with.
Three CI heads failed that way during kanso#1240 while readbench's instruction
row was being harvested, and each one sent a reader to `perf_record.kso:258`
when the fault was two processes away.

Reproduced before it was touched, in a staged tree with the readbench row cut
out of the instructions golden:

    error[endpoint]: unhandled err reached the executor: "missing index 2"
      born in perf_record/score_in at scripts/perf_record/perf_record.kso:258

and after:

    welfare printed no score, so this row has none to carry. it exited 1
    saying: ... "missing index "readbench"" born in welfare/worked at
    scripts/welfare/welfare.kso:375

The reader now points at welfare, and welfare names the counter.

**THE OBVIOUS FIX IS WRONG AND WOULD HAVE BEEN WORSE THAN NONE.** `os/run`
hands back a status, and refusing on a non-zero one is the shorter edit. But
welfare exits 1 on a fall AND on a rise nobody ratcheted — `os/exit 1` at three
sites — and it prints a score in every one of them. Those are the commits the
perf history most needs: a refusal keyed to the status would go silent on
exactly the rows a reader would later want. Checked in welfare.kso before
writing anything, which is the only reason the shape is what gets asked and the
status only rides along in the message.

**The spec enters where a user enters**: a staged tree with the goldens copied,
the compiler and scripts and library borrowed from the checkout, a git
repository because the row carries its commit, and one counter's instruction
row removed — the state a branch is in between minting a benchmark and
harvesting its row from CI. Watched red on the old body, for `missing index 2`.

**Only half of it discriminates, and the entry says so.** The second test —
that a healthy run still reads its score — passes on the OLD code too. It is
there to catch a refusal that fires on a good run, which would cost the history
every row it has, and it is not evidence for the fix.

## 2026-09-04 — THE ROW COUNTS THE PROGRAM NOW, AND THREE QUARTERS OF THE LINKER'S LUCK GOES WITH IT

**DONE, and it closes the last blocking entry in the ledger.** The entry asked
whether a term whose movement is dominated by binary layout should set a
ratcheted floor. Clay answered by redirecting the question — "can you not force
that to be consistent with some initial setup that's specific to specs?" — and
he was right that I had conflated two things. The 2026-09-03 ruling was NO
EXCLUSION of glibc. Excluding the process's own startup is a different act and
nobody had ruled on it.

`scripts/gates/compile_instructions.sh` now reads `std::rt::lang_start::
{{closure}}` inclusive out of the callgrind profile instead of the summary
line. That frame is everything `main` does, so every libc call the compiler
makes is still counted; what it drops is the 465,122 instructions above it —
the loader mapping five shared objects, and Rust placing its stack guard, which
parses `/proc/self/maps` with `getline` and `sscanf` and therefore moves with
where the linker happened to put things.

**How much it buys, on seven binaries whose sources differ only in code or data
nothing reaches:**

    variant           .text     row         maps     program
    baseline          2550854   42,344,081  112,580  41,878,959
    +50 dead fns      2552534   42,348,024  114,845  41,879,987
    +200 dead fns     2558486   42,347,128  112,586  41,879,361
    +400 dead fns     2565174   42,348,044  110,341  41,879,922
    +3 KiB .bss       2550854   42,346,221  114,720  41,878,959
    +64 KiB .bss      2550854   42,346,221  114,720  41,878,959
    +64 KiB .rodata   2550854   42,344,099  112,598  41,878,959

Whole process spans 3,963; the frame spans 1,028.

**THE FIRST READING WAS WRONG AND THE CORRECTION IS THE INTERESTING PART.** Four
binaries measured earlier — baseline, +3 KiB .bss, +64 KiB .bss, +64 KiB
.rodata — gave 41,878,959 four times, and I wrote invariance into three files
on the strength of it. Those are all DATA changes. The `.text` case had never
been probed with the frame read out, and it does not hold: 7,632 bytes of code
no execution reaches moves the frame 402, and the movement is not monotone in
`.text` — fifty dead functions move it more than four hundred do. The first
`.text` probe I ran was contaminated too, because it reached the functions
through an environment variable, which is executed work; the honest version
keeps them with a `#[used]` array of function pointers and calls nothing.
`scripts/compile_row_probe.sh` said "ALL of that movement is in `maps`" and now
says what the seven binaries show.

**WHAT THE SPLIT GOES BLIND TO, and the guard for it.** Startup work scales
with what gets loaded, so the one compiler change that moves the dropped half is
growing a dependency — one more shared object measured at about 32,090.
`bench/compile_libraries_golden.txt` pins the five sonames `ldd` reports and
`scripts/gates/compile_libraries.sh` diffs them. A new dependency now turns red
saying `libfoo.so.1 appeared` rather than showing 32,090 mixed into a term that
moves 3,963 for nothing. Watched red both ways before it shipped: dropping
`libm.so.6` from the golden, and adding a name that is not there.

**The ratchet row's mutation is `a_library_the_row_cannot_see.sh`**, and it
links Rust's standard library dynamically with `-C prefer-dynamic` rather than
editing the golden — an edit to the golden would prove nothing about the thing
being guarded. Watched red on the real gate: `> libstd-46d936097e8c5b85.so`.
Its anchor is the absence of `.cargo/config.toml`, so a repo that grows one
stops the mutation instead of silently appending into it.

**OPEN.** The goldens still hold whole-process values and CI has to re-sit them
under the new definition, which is a deliberate red round. When it does, the
welfare baseline `compile_instructions: 57029831` owes a correction of the same
size as the drop on that host — the startup half is an additive constant the
old baseline also paid, so leaving it alone would read a measurement change as
a free win. That correction and the re-sitting land together.


## 2026-09-04 — THE ANCHOR WAS THE STANDARD LIBRARY'S, AND CI REFUSED IT

**DONE, and it corrects the entry above.** That entry said the row reads
`std::rt::lang_start::{{closure}}` inclusive. CI refused on the first run:

    ::error::the profile carries no std::rt::lang_start::{{closure}} frame

The frame is real on this container under rustc 1.94.1 and absent from the
runners' profiles under 1.98.1. So the anchor was a name the standard library
owns, which a version bump can move with nobody here touching a line — and the
gate's own refusal path is the only reason that showed up as a stop rather than
as a number.

**The anchor is `kanso::main` now**, this crate's own symbol, with an
`inline(never)` in src/main.rs whose doc comment says the measurement is why it
is there. It sat exactly 10 instructions below the closure on all four profiles
retained from the seven-binary sitting, and the baseline rebuilt with the
annotation reads `row 42,344,081` — identical to the instruction — and
`program 41,878,949`. So the annotation costs nothing and the published spans,
3,963 whole-process against 1,028 in the frame, hold under either anchor.

**The refusal now prints the profile before it refuses.** The round that made
this necessary said "cannot be read out of it" and printed nothing that could
be read instead, so finding out what the profile did contain took a second
round. `callgrind_annotate --inclusive=yes --threshold=99 | head -30` goes to
the job log first, and the refusal points at it. This is the same defect
`scripts/perf_record` had for a day — a reader that reports its own failure
without reporting what it saw.

**`compiler libraries` was green on that same CI run**, which is the half of
this change that was not being re-sat.


## 2026-09-04 — THE 1,028 IS ONE MEMCMP, AND IT IS ALIGNMENT

**DONE.** The entry above left a residual: the compiler's own frame still moves
about a thousand instructions between binaries that do no different work, and
said the movement comes from `.text`. Diffing the two profiles by self cost
says exactly where, and the whole 3,047 of the two-hundred-function binary
accounts for itself:

    +1,624  _dl_relocate_object  (dl-machine.h)
    +1,015  _dl_relocate_object  (do-rel.h)
      +402  __memcmp_avx2_movbe
        +6  ____strtoul_l_internal

The first two are the loader, above `main`, and the split already drops them.
**The 402 is the entire in-frame residual and it is one function** — glibc's
AVX2 memcmp, counting differently on identical comparisons.

That is an alignment difference, and this vein has reported it from the other
end before: the comment on the pinned tunables in
`scripts/gates/compile_instructions.sh` names two profiles that differed in
`_int_malloc` and in `__memcmp_avx2_movbe` on one unchanged binary. Growing
`.text` moves the end of `.bss`, the kernel starts the heap after it, and every
allocation the front end makes lands at a different alignment. The comparisons
are the same comparisons; the vector loop takes a different number of steps to
reach them.

**No fix is available from here.** The tunables pin malloc's thresholds and
cache sizes, which is what made the row comparable in the first place, but none
of them pins where the break starts — that is the kernel's, computed from the
binary's own size. So the residual stands at about a thousand, attributed, with
the mechanism named and the remedy out of reach. It is a twenty-eighth of the
smallest front-end change on record, which is what makes it liveable.

**Why this is worth having written down:** the residual was the last live half
of the ledger entry, and "about a thousand instructions of link luck" is the
kind of sentence that stays vague for months. It is one glibc function and a
heap base. Anyone who later finds a way to pin that base can close it.


## 2026-09-04 — A SPEC THAT REPORTED ON THE STATE OF SOMEBODY'S TARGET DIRECTORY

**DONE.** `tests/a_row_names_the_tool_that_failed` passed here and failed on CI,
both cases, with the same line:

    cannot start ./target/release/kanso

`perf_record` runs `./target/release/kanso`, and the spec staged its scenario by
symlinking the checkout's whole `target` into it. `cargo test` builds debug, so
on a clean machine that path does not exist. The spec passed for anyone who had
run `cargo build --release` first and failed for everyone else, which means what
it was reporting on was the state of a directory rather than the state of the
program.

The healthy case is the one that shows it clearly. It asserts a healthy run says
nothing on stderr; on CI it said `cannot start`, so the assertion fired for a
reason that has nothing to do with what the spec is about. A spec whose green
depends on an artifact it does not create is a spec that can go quiet at any
time.

**The stage brings its own compiler now**, symlinked from `CARGO_BIN_EXE_kanso`
to `target/release/kanso` inside the stage. Reproduced first by moving the
release binary aside — same message, same two cases — then green with the fix in
place and the binary still absent. It costs 30 seconds against 7, because the
inner calls run the debug build, and that is the price of a spec that carries
its own subject.


## 2026-09-04 — THE ROW IS RE-SAT, AND THE BASELINE MOVES WITH IT SO NOBODY BANKS A DEFINITION

**DONE.** CI counted the compiler's own frame on Zen 4 (family0x19-model0x11),
sha 4dc725bdb40d:

    whole process   41,845,704   what the row used to read, to the instruction
    kanso::main     41,379,840   what it reads now
    dropped            465,864   loader and stack guard

That is 742 from the container's 465,122 on a different toolchain and binary,
which is the agreement worth having: the quantity being dropped is process
startup, and it is the same size in both places.

**The other two chips' rows are removed rather than carried.** The definition
changed for all of them and CI can only re-sit one chip per run, so Zen 3 and
Intel are re-sittings when they next refuse. This file has done exactly that
twice before, for the same reason: a value measured under the old definition is
worse than no value. The pair the AMD rows carried is not re-sat either —
whether it survives is for CI to say, and the layout term it came from is most
of what this change removes.

**The welfare baseline moves by the same 465,864**, 57,029,831 -> 56,563,967.
The startup half is an additive constant the old compiler paid too, so
subtracting it is what the baseline would have read under the new definition.

**And that raises the score by 0.01, which is not a win and is recorded as
not one.** 73.5162 -> 73.5256. Removing a constant from both sides of a ratio
raises it: 1.362866 becomes 1.366938, because the constant was diluting a
measured improvement. The compiler did not get faster. The measurement got more
sensitive by about a third of a per cent of its own term, and the `--set`
reason says so in those words. A reader who finds this rise in the history and
looks for the commit that earned it will find this entry instead.

**What the trend gate will report on the next run is a fall of 465,864 in
`compile_instructions`, listed as better.** It is not better. It is the same
compiler measured from a different frame. The gate has no way to tell a
definitional re-basing from a win — it can tell a MINTED counter, which has no
baseline at all, and this one has a baseline that moved underneath it. Whether
that deserves machinery is a real question and not one this change answers.


## 2026-09-04 — A COMMENT LINE SPLIT IN HALF BECAME A ROW, AND THE DUPLICATE CHECK CAUGHT IT

**DONE, and the defect was mine.** The re-sitting entry above put its note into
`bench/compile_instructions_by_cpu.txt` by inserting at the first occurrence of
`family0x19-model0x11` in the file. That occurrence is inside a comment
PARAGRAPH written months ago, not at the row, and the insertion cut the line
after its `# ` — leaving

    family0x19-model0x11 rather than carry a value measured against the previous

as a bare line with no `#`. The file's parser reads a key and a value off any
non-comment line, so the chip acquired a second row whose value is the word
`rather`.

CI refused, precisely and by name:

    bench/compile_instructions_by_cpu.txt has more than one row for a chip:
        family0x19-model0x11
    Only the first is ever read, so the rest say nothing while looking like a
    sitting.

That check went in on 2026-09-03 for a different reason — a re-sitting that
appended instead of editing — and it caught a case nobody had in mind: a
comment turning into a row by accident. **The first row is the one welfare
reads, so had the check not existed, `compile_instructions` would have been the
string `rather` and the failure would have surfaced somewhere much further from
its cause.**

The note is above the row now, the comment line is whole, and `compile_ir_row.sh`
reads 41,379,840 on the key.

**The refused run had already taken its sitting, and it was a different chip.**
`compile_sample cpu="cpu family 0x19 model 0x1" sha=4dc725bdb40d row=41379840`
— Zen 3, same binary as the Zen 4 sitting, same value to the instruction. So
the file gains a second row from a run that failed: the measurement is printed
before the comparison, which is what makes it usable. The two AMD models agree
on this binary, the fourth on which they have, and the keys stay separate for
the reason the header already gives.


## 2026-09-04 — THE GATE CALLED A DEFINITION CHANGE A WIN, AND THE SENTENCE IS IN THE MERGE

**DONE.** The entry above this one moved `compile_instructions` from 41,845,704
to 41,379,840 by counting the compiler's own frame instead of the whole process,
and subtracted the same 465,864 from welfare's baseline so the ratio stayed
comparable. Nothing in the compiler got faster. What the trend gate printed on
that merge, verbatim:

    improved: compile_instructions 41,845,704 -> 41,379,840  (bench/compile_instructions_golden.txt)
    every changed counter is priced (or improved).

The entry noted the sensitivity and left the question of machinery open. It is
answered by the run: the gate misreports today, and the misreport is permanent
record.

**The signal is welfare's baseline, and it is exact.** `welfare --set` moves the
FLOOR and never the baseline, so a baseline value moves only when somebody
decides the old reading and the new one are not of the same quantity. A counter
whose golden moved and whose baseline moved in the same diff is therefore
neither better nor worse — it takes the third state kanso#1200 built for minted
counters, prints as `re-based`, and counts toward neither side of the
pure-regression rule. It still owes a sentence: the same naming check a
worsening gets, because the value it landed on is what a write-up of a re-basing
states.

**WHAT THE MATCH REACHES, and what it does not.** Welfare renames the runtime
counters on the way into the objective — `jsonbench` is `decode_instructions` in
the baseline — so matching a golden row to a baseline key by name covers the
compile vein and misses the runtime one. Rather than copy welfare's rename table
into a second place to go stale, a baseline that moved and matched no golden row
is listed by its welfare name under RE-BASED, unclaimed, and refused unless the
log delta names it. A runtime re-basing is then visible and priced even though
the gate cannot say which row it belongs to.

**Reaching the rest is not "ask welfare for the mapping", and the measurement
says why. OPEN.** Written that way in this entry's first draft, then checked:
`welfare --counters` prints 27 counters, and matching each one's value against
every row of every golden the trend gate walks leaves 8 unmatched. Two are
ambiguous — `decode_peak_bytes` reads 2,097,152, which is also
`arena_peak_bytes` in the oneshot, basket and wide goldens — and six have no row
carrying that value anywhere, because `peak_of` SUMS three pools:
`arena_peak_bytes + held_peak_bytes + perm_peak_bytes`. A `*_peak_bytes` term is
a derived quantity, not a row.

So the link is not a rename table and cannot be a bijection: it is many-to-one
for every memory term, and welfare does not know the trend gate's prefixes
(`work_jsonbench` is the gate's spelling of the row welfare calls
`decode_instructions`). What would close it is an explicit table naming, for
each objective counter, the gate keys it is derived from and how — identity or
sum — in one place, with a spec that replays the derivation and asserts it
reproduces welfare's own reading. That is a real piece of design rather than a
plumbing change, and this entry records the measurement that rules out the
cheap version.

**Watched, both directions.** The gate at f3047edd against 1a0cb51e reproduces
the `improved:` line above; the same pair under the new gate reads `re-based:`
and lists the counter. The mutation
`a_re_basing_that_pays_for_a_regression` moves the compile golden and the
baseline together beside a plain `work_jsonbench` worsening, with both moves
written up: green under the old gate at exit 0 — a real regression paid for by a
definition change — and refused under the new one by the pure-regression rule.
Deleting the mutation's log paragraph turns that refusal into UNPRICED, which is
red for the wrong reason, so `tests/a_re_basing_row_stays_a_pure_regression.rs`
holds the paragraph to the values the `sed` lines write. Both of its assertions
were watched red. The orphan path was watched too: moving
`decode_instructions` alone is refused as UNSTATED and passes once a sentence
names it.

## 2026-09-04 — A BENCHMARK JOINED THE OBJECTIVE AND THE GATE THAT WATCHES THE OBJECTIVE COULD NOT SEE IT

**DONE.** Found while mapping welfare's counters onto the trend gate's keys for
the entry above: `bench/cost_golden_read.txt` is not in the gate's golden list.
readbench joined the objective the day before (kanso#1240) with
`read_arena_blocks` and `read_peak_bytes` as welfare terms, the golden was
written, and the one program whose job is to watch the objective's inputs was
never told about the file. Either row could have moved by any amount in
silence.

This is kanso#1046's finding one benchmark later — *"half the score's inputs
were invisible to the gate that exists to watch the score's inputs"* — and it
was found the same way both times, by asking which files in `bench/` the gate
names. That is now the only method that has ever found one of these, and the
list is short by one every time a benchmark lands.

Setting `arena_peak_bytes` in the read golden to 9,999,999,999 produces NO
OUTPUT WHATEVER from the gate at f3047edd, exit 0. With the file listed:

    worsened: read_arena_peak_bytes 1,048,576 -> 9,999,999,999  (bench/cost_golden_read.txt)

and a refusal. `a_read_counter_worsens_for_nothing` is that mutation, rowed.

`bench/compile_libraries_golden.txt` is the only other file in `bench/` the
gate does not walk, and it belongs there: it holds five sonames rather than
counters, and its own gate diffs it byte for byte.

## 2026-09-04 — THE LINK IS WRITTEN DOWN, AND THE RUNTIME HALF OF THE RE-BASING CHECK WORKS

**DONE, and it closes the OPEN in the entry two above.** That entry said the
many-to-one link between welfare's counters and the trend gate's keys "is a real
piece of design rather than a plumbing change". Built and it is a table:
`bench/objective_sources.txt`, 41 rows for 27 counters, `<objective counter>
<gate key>` a line, several lines for a counter that is a sum.

Twelve counters are one row each (`decode_instructions work_jsonbench`), eight
more are identities on the compile goldens and the arena blocks, and seven
`*_peak_bytes` are three rows apiece because `peak_of` adds the arena, held and
perm pools. Written and then replayed against `welfare --counters`: 27 counters,
0 unreproduced.

**A prefix, not a name.** The first cut classified nothing in the runtime vein
and I nearly recorded that as a limit again. `unchanged` was handing `shifted?`
the counter with its golden's prefix STRIPPED — `jsonbench`, where the link file
names `work_jsonbench` — so the check worked only for the four goldens whose
prefix is empty, which is exactly the compile vein it was first written for. The
probe that caught it was moving `jsonbench` and `decode_instructions` together
and watching the gate still say `improved`. It says `re-based` now.

**The spec is the whole of it, because a written link is one a rename breaks
silently.** `tests/the_objective_reads_what_the_gate_watches.rs` reads the
gate's own golden-and-prefix bindings, sums each counter's rows out of those
files, and asserts the total is what welfare prints — for every counter welfare
scores and no counter it does not. Watched red four ways: a counter dropping out
of the file, a row renamed on the golden side, a name the file invents, and a
nonzero pool removed from a sum.

**What it cannot see, said rather than left to be found.** The check is on
totals, so a pool reading nought contributes nothing and dropping its row costs
the sum nothing. Every `held_peak_bytes` and `perm_peak_bytes` in the goldens is
nought today, which makes twelve of the 41 rows unfalsifiable right now. The
direction that matters is covered: a pool added to `peak_of` and not to the file
is nonzero by the time anyone cares, and a dropped zero row starts failing the
day its pool carries a byte. The spec's own header says so.

The `RE-BASED, unclaimed` listing survives with a narrower meaning: a baseline
that moved while not one of the golden rows it is made of did — a ratio moving
with no measurement behind it — and it still owes a sentence.

## 2026-09-04 — A THIRD CHIP, AND IT READS WHAT THE OTHER TWO READ

**DONE.** CI refused kanso#1242 on `family0x1a-model0x2`, a key
`bench/compile_instructions_by_cpu.txt` did not carry:

    nothing in bench/compile_instructions_by_cpu.txt was counted on
    family0x1a-model0x2, so this run's 41379840 cannot be compared to anything.

The branch's diff is scripts, tests, bench data and this log — `git diff
origin/main -- src/ lib/ Cargo.toml Cargo.lock` is empty — so the front end is
untouched and this is a chip new to the pool rather than a row gone stale. Its
sitting is **41,379,840**, the same figure to the instruction as Zen 4 and
Zen 3 on the same binary. Three keys, one number.

The row goes LAST. The first row is what welfare and
`compile_instructions_golden.txt` read, and moving that authority to a chip
because it happened to be today's runner is how a value ends up published for a
reason nobody chose. Checked after adding it: all three keys resolve to
41,379,840 and an invented key still refuses.

That the keys agree is not an argument for merging them — the file's header
gives the reason, and this is the fifth binary on which two or more have agreed
while the noise the key exists for stays real.

**A clippy warning in the same round.** `manual_is_multiple_of` on the
comma-grouping helper in `a_re_basing_row_stays_a_pure_regression.rs`. Fixed;
`cargo clippy --all-targets --all-features` is at zero warnings.

## 2026-09-04 — THE LOG IS BACK TO FORTY, AND ONE OPEN THREAD WENT WITH THE FIVE

**DONE.** Five entries move to `design/log/compiler-log-archive.md`, unedited
and in order: the whole 2026-09-03 chip series from *a recorded chip
disagreeing with itself* through *the bimodality reaches the trend gate*. The
live file was at 44 against a budget of forty.

**One of them is still OPEN and this entry is where it stays findable.** The
first of the five holds *the third cause*: `family0x6-model0xcf` counted
41,832,275 at 12:33 and 41,831,767 at 13:05 — 508 apart, same key, same binary,
no front-end change in the diff — so the key did not determine the row, and the
per-chip table rested on a weaker premise than kanso#1226 claimed for it.

**THE MECHANISM IS ALREADY MEASURED, AND THE REDEFINITION DOES NOT REMOVE IT.**
I first wrote that the 508 was glibc parsing `/proc/self/maps`, which #1241 now
drops, and that the thread had therefore got cheaper. The archive says
otherwise, in an entry that corrects the very guess I was repeating. The two
profiles differ in three rows and all three are glibc's allocator:

    _int_malloc        1,551,384   1,551,964   +580
    _int_free          1,522,333   1,522,352    +19
    memcmp-avx2-movbe  1,353,408   1,353,342    −66

Every kanso symbol agrees to the instruction across the pair. `_int_malloc` and
`_int_free` are calls the compiler makes, so they sit INSIDE `kanso::main` and
the new definition counts them exactly as the old one did. The thread is as open
as it was.

**What #1241 did give it is a name for the mechanism.** Its seven-binary table
found the same signature — `__memcmp_avx2_movbe` moving 402 while every other
in-frame symbol held — and traced it: growing `.text` moves the end of `.bss`,
the kernel starts the break after it, and every allocation lands at a different
alignment. The 508 is that phenomenon between two runs rather than between two
binaries, which sharpens the open question to a single one: what moves the heap
base when the binary sha, the 123-line cpu feature block, glibc, valgrind and
the environment all agree? Pinning the malloc tunables took the cluster from
5,064 to 508 and did not remove it, and no tunable pins where the break starts.

A third chip was recorded on kanso#1242 — `family0x1a-model0x2`, reading
41,379,840, the same to the instruction as both AMD models. That says the keys
agree on this binary and says nothing about whether one of them disagrees with
itself.

## 2026-09-04 — THREE OPEN MARKERS THE RECORD HAS ALREADY ANSWERED, AND ONE THAT MISLEADS

**DONE.** Searched the live log and `design/log/compiler-log-archive.md` before
filing: the archive carries the 2026-09-03 allocator measurement and the chip
series moved there this afternoon, and nothing in either closes these by name.
The live file carried five `OPEN` markers. Three are answered, and one of the
three is worse than stale.

**A LIVE ENTRY ASSERTS A HYPOTHESIS ITS OWN CORRECTION SITS IN THE ARCHIVE.**
The ninth-entry thread reads *"glibc parsing `/proc/self/maps` before `main` is
still the only mechanism that fits"*. It is not. The archived entry that
corrects it diffs the two profiles and finds three rows moving, all of them the
allocator:

    _int_malloc        1,551,384   1,551,964   +580
    _int_free          1,522,333   1,522,352    +19
    memcmp-avx2-movbe  1,353,408   1,353,342    −66

Every kanso symbol agrees to the instruction across the pair, and the entry
rules the cpu out on 123 byte-identical feature lines.

**I repeated the error today, which is the evidence that the shape is the
problem rather than the wording.** Writing kanso#1243 I restated the maps
hypothesis from the live entry, concluded that kanso#1241 had removed the
mechanism, and pushed that. The archive corrected me on the read. A reader of
the live log alone would make the same move, because the hypothesis is here and
its refutation is forty thousand lines away with no pointer from this side. The
trim rule says the archive keeps the older end and this file keeps the last
forty; it does not say a live entry may go on asserting what an archived one
disproved. When a trim splits a correction from what it corrects, the live half
owes a line saying so — and this is that line.

**The rewiring is done by another mechanism.** That thread describes
`--toggle-collect` and what it would owe. Clay dropped the toggle, and
kanso#1241 reached the same end by reading `kanso::main` inclusive out of the
profile instead. Both debts were paid in that change: the guard exists (the gate
refuses a profile carrying no such frame, and prints the frames it does carry),
and the welfare baseline moved 57,029,831 -> 56,563,967 in the same commit, so
the instrument change could not be banked.

**The two-readings thread came out BOTH ways, which its dichotomy did not
allow.** It said other chips holding their own values would mean the key
separates nothing, and a second value on a recorded chip would mean neither
suspect was the term. Since then: Zen 4 and Zen 3 read 41,379,840 on sha
4dc725bdb40d; Intel `family0x1a-model0x2` read 41,379,840 this afternoon;
`family0x6-model0x6a` read 41,379,840 on this branch, whose diff is `design/`
alone; and `family0x6-model0xcf` produced two values 508 apart on one binary.
Four keys agree wherever they have shared a binary AND one key disagrees with
itself. Both branches fired, so the disjunction was wrong rather than either
answer being.
What survives is the narrower question the entry above already carries: what
moves the heap base when everything else agrees.

**What stays open is the corpus.** The five effect builtins are in no benchmark,
so a socket read going from a 260 MB peak to 2 MB scores zero. readbench does
not close it — that benchmark splits a string 200 times, which is the read
beat's repair and not the builtins. It is open on purpose, with its reason
written where it was filed.

## 2026-09-04 — where a program claims its stack

Searched first: the archive's 2026-08-31 entry on `k_b_append_grow` is the
nearest thing, and it is about a runtime helper's callee-saved prologue rather
than about the emitted module. Nothing in the log, the archive or the compiler
page mentions where an `alloca` stands. This is new.

The emitter wrote each call's argument array in the block that filled it. A
record built inside an `if` arm put its array in that arm; a dispatch arm the
same. Sixty-eight of the decoder's seventy-three stack slots stood outside
their function's first block, and LLVM reads a slot placed there as a dynamic
stack object whatever its size: the function keeps a frame pointer it would
otherwise omit, restores `rsp` through it on every return path, and claims the
slot again on each pass instead of once.

Callgrind, on jsonbench before the change: `push`/`pop`/`leave` are 252,410,292
of 2,098,864,058 instructions, 12.03% of the decode. The frame pointer's own
share — `push %rbp`, `mov %rsp,%rbp`, `pop %rbp` and the `lea -0xN(%rbp),%rsp`
that restores the stack — is 51,317,285 of those. The rest is callee-saved
registers, which a fixed frame does not remove.

The fix holds every `alloca` back at a single choke point in `FnEmit` and
writes them at the head of the entry block, so a site nobody has written yet
gets the placement too. Hoisting them by hand in the `.ll` measured the same
number as the compiler change does, which is how the size of the move was known
before any Rust was written.

```
    jsonbench     2,098,864,058 -> 1,914,624,003   -8.778%   (container)
    encodebench   5,847,000,948 -> 5,808,061,875   -0.666%
    oneshot          31,431,613 ->    30,109,010   -4.208%
    basket           40,300,171 ->    39,933,683   -0.909%
    widebench        59,384,053 ->    58,446,988   -1.578%
    deepbench       675,925,724 ->   678,046,033   +0.314%
    escapebench     130,170,750 ->   130,170,344   -0.000%
    pendbench       715,732,721 ->   702,115,194   -1.903%
    indexbench        5,242,362 ->     5,241,703   -0.013%
    scanbench     1,423,437,575 -> 1,425,333,592   +0.133%
    digestbench      81,256,592 ->    80,996,019   -0.321%
    readbench     2,000,657,821 -> 2,000,657,400   -0.000%
```

These are the container's numbers and its glibc is not the runner's, so
`bench/instructions_golden.txt` is regenerated from CI's own sitting rather
than from here. CI counted, on the same commit:

```
    jsonbench     2,098,864,471 -> 1,914,624,416   -8.778%   (runner)
    encodebench   5,847,000,948 -> 5,808,062,274   -0.666%
    oneshot          31,431,613 ->    30,109,409   -4.207%
    basket           40,300,171 ->    39,934,096   -0.908%
    widebench        59,384,053 ->    58,447,401   -1.578%
    deepbench       675,925,724 ->   678,049,713   +0.314%
    escapebench     130,170,750 ->   130,170,743   -0.000%
    pendbench       715,732,721 ->   702,115,580   -1.903%
    indexbench        5,242,362 ->     5,242,116   -0.005%
    scanbench     1,423,437,575 -> 1,425,334,005   +0.133%
    digestbench      81,256,592 ->    80,996,418   -0.320%
    readbench     2,000,657,821 -> 2,000,657,813   -0.000%
```

**work_deepbench rises to 678,049,713 and work_scanbench to 1,425,334,005.**
Those are the two the container also had rising, to the same third decimal
place, and they are the two whose `.text` grows most — scanbench by 3,424
bytes and deepbench by 128. Neither is a change to what those programs do:
every allocation counter in both is byte-identical, the emitted line count is
byte-identical, and the source they run is untouched. What moved is the layout
LLVM had to schedule against. They are bought by the ten that fall, which the
welfare number prices at 73.53 -> 73.73.

**compile_instructions 41,379,840 -> 41,377,711**, a fall of 2,129: the emitter
holds a `Vec` of slot lines per function and splices it, and that is cheaper
than what it replaced. The per-chip table is re-sat down to the one row CI
measured, Zen 3. The other three keys were all reading 41,379,840, and they
measure a compiler that no longer exists — removed rather than carried with the
delta applied, the same call the Zen 4 row got on 2026-09-03. Ten rows fall and two rise. The two that rise are the two whose
`.text` grows most, which is the shape to expect: a fixed frame gives LLVM a
different layout to schedule against and it inlines differently in both
directions.

Not one allocation counter moves. All ten counter gates are byte-identical, and
so is `bench/emitted_golden.txt` — the same lines are written either way and
only their order changes. That is the reason this needed a spec of its own:
`tests/a_stack_slot_stands_in_the_entry_block.rs` builds a record in each arm
of an `if` and reads the emitted `.ll`. Watched red before it was watched green,
reporting two of three slots adrift in `d_slot/pick_1`.

**text 1,070,072 -> 1,074,552.** Six rows fall, six rise, none by more than
2.4%, and the largest single move is scanbench 145,106 -> 148,530. jsonbench
spends 672 bytes here for 184 million instructions.

**welfare 73.53 -> 73.73**, floor set in the same commit.

**A ratchet row went blind on this branch, and the row was right.**
`a_re_basing_that_pays_for_a_regression` writes a re-basing beside a plain
worsening and requires the trend gate to refuse under the pure-regression rule.
Here it stayed green: the branch's own eleven falling work rows sat on the other
side of the mutation's rise, so the gate read a trade. The rule is about the
whole branch and the mutation was only about its own three files, which is a
hole any improving branch would have opened. It now resets `bench` and
`tests/golden/mem` to the base before it edits, so the three values it writes
are the only moves the gate can see. Watched red afterwards, and for the row's
own reason: `RE-BASED ... compile_instructions` followed by the pure-regression
refusal.

**OPEN — what the callee-saved registers cost.** Removing the frame pointer
leaves 201 million instructions of `push`/`pop` on jsonbench, 9.6% of the
decode. The decoder is a chain of mutually tail-calling functions, and a tail
call pops the whole frame before its `jmp` and the target pushes it again. That
is a different question from this one and nothing here answers it.

## 2026-09-05 — the ascii test comes out from behind the wide pass

Searched first: yesterday's entry on the entry-block stack slots is the nearest
neighbour and is about placement rather than about this. The archive's
2026-08-31 entry on `k_b_append_grow` is the same shape one layer down — a
cheap path trapped behind an expensive function's prologue — and its conclusion
was taken back by the welfare number. This one is not.

`k_utf8_bad` counted the bytes, ran the ascii word-test, and then ran the wide
SIMD pass, all in one function. LLVM will not inline that function anywhere:
the wide pass is seven constant loads and two zeroed accumulators before its
first block, and a table of three sixteen-byte lookups after it. So every token
a json document holds paid a call to reach a two-load answer — 83,092,800
instructions for 1,571,250 answers on jsonbench, 53 apiece for a mean run of
seven ascii bytes.

The front door is `always_inline` now and holds the counter and the test; the
pass is `k_utf8_bad_wide` and is reached only by a run carrying a byte with the
high bit set. Caller and callee are both in `runtime.c`, which is why a C
attribute reaches this one at all — see the rider below for where it does not.

```
    jsonbench     1,914,624,003 -> 1,898,797,203   -0.827%   (container)
    oneshot          30,109,010 ->    30,003,499   -0.350%
    widebench        58,446,988 ->    58,318,988   -0.219%
    basket           39,933,683 ->    39,915,674   -0.045%
    encodebench   5,808,061,875 -> 5,807,958,536   -0.002%
```

Seven rows are byte-identical and none rises. Every allocation counter is
unchanged and so is the emitted line count: this is entirely inside the runtime.

**text 1,074,552 -> 1,081,288.** Five rows rise 1,168 to 1,392 bytes and seven
hold exactly still: the ascii test is written at each of the validator's four
call sites now instead of once inside the pass, and the grammar walk is a real
function on every target rather than dead code behind an `#else`. The seven
that hold are the programs that never reach utf-8 validation. jsonbench spends
1,392 bytes for 36.9 million instructions.

**The harness caught the rename, which is what it is for.**
`scripts/utf8_differential` extracts the validator's text out of `runtime.c`
rather than carrying a copy, and the split moved the signature it looks for: it
answered `missing index 2` from `body_of` and stopped. It reads three pieces
now — `k_all_ascii` verbatim, the wide pass as `harness_utf8_wide`, and the
front door as `harness_utf8_ok` with its call renamed — so what is checked is
the composition. 45,189,025 validator checks and 8,346,016 counter checks
against the independently written reference, zero mismatches. Watched red
first: dropping the door's call to the pass gives 30,746,942 mismatches.

**RIDER on yesterday's OPEN thread — why the callee-saved 9.6% sits at entry.**
The entry-block entry left 201 million instructions of `push`/`pop` unexplained.
LLVM's shrink-wrapping cannot move them: it needs one save point dominating
every use and one restore point post-dominating them, and these functions have
three to eight returns spread across blocks. In jsonbench's emitted module
`parse_value_2` has 3 returns and 1 `musttail`, `obj_key_start_4` 7 and 1,
`string_at_4` 6 and 2, `array_step_3` 4 and 1, `scan_at_5` 8 and 7; the module
holds 125 `musttail` sites across 15 functions with more than one. A `musttail`
exit needs the epilogue before its `jmp`, so each of those blocks is a restore
point and the prologue has to dominate all of them. The general remedy is
`preserve_none`, the calling convention LLVM added for this shape, and it
landed in LLVM 19. The pinned toolchain is Ubuntu clang 18.1.3, which rejects
`preserve_nonecc` at the parser while accepting `tailcc` on the same file. The
fix is named and gated on a toolchain bump. What stays inside reach is reducing
what is live across the non-tail calls in the hot dispatchers, which is a
per-function matter.

**And then the third arm, in the same change.** With the ascii runs turned away
the wide pass was 60,769,050 instructions over 203,700 calls, 3.29% of the
decode: 13% of the document's strings are not ascii, and each cost 298 to
validate about seven bytes. The setup is the whole of it, and its own comment
already said so. The portable arm of that pass walked the grammar a byte at a
time and nothing but a host without simd ever reached it, so it became a
function and the door chooses on the length.

```
    jsonbench     1,898,797,203 -> 1,877,751,303   -1.108%   (container)
    oneshot          30,003,499 ->    29,863,200   -0.468%
    widebench        58,318,988 ->    58,270,995   -0.082%
```

Every arm returns the same sentence, so which one answered is not observable,
and the harness reads a third piece now — with the threshold taken out of the
line that defines it rather than written down twice. Watched red: dropping the
surrogate bound on `0xED` gives 2,048 mismatches, exactly `ED A0-BF` by
`80-BF`, so the fuzzer reaches the arm that moved.

**DECLINED on the way: the lead byte's table.** Replacing the eight-comparison
chain that decides a lead byte's width with a 256-entry table costs 1,428,150
instructions instead of saving any — 1,877,751,303 against 1,879,179,453 on
jsonbench. The chain is predictable, and the ascii bytes that dominate a
mostly-ascii run are tested before it and never reach either. Recorded so it is
not rediscovered.

**CI's sitting, which is what the goldens carry.** The rows above are the
container's; the runner counts a few hundred higher on every one of them and
its numbers are the ones pinned. Five work counters move and none rises:
`work_jsonbench` lands on 1,877,751,716 (−1.926%), `work_oneshot` on 29,863,599
(−0.816%), `work_widebench` on 58,271,408 (−0.301%), `work_basket` on
39,914,087 (−0.050%) and `work_encodebench` on 5,807,819,641 (−0.004%). The
other seven are byte-identical.

**And `compile_instructions` rose 682, to 41,378,393, which is layout.** The
front end did not change on this branch — the diff is `src/runtime.c`, the
differential harness, the goldens and this file. `src/runtime.c` is
`include_str!`'d into the compiler, so editing the runtime moves the compiler's
own bytes: 1,435 more of them here. `kanso check lib/json` emits nothing, so
the validator that changed cannot run during the measurement. This is the
seventh time this vein has moved for layout and the second time a runtime edit
has done it — kanso#1226 moved it −5,621 the same way, held on one chip with
both binary shas printed. `compile_allocs`, `compile_peak_bytes`, rounds and
visits are byte-identical.

The per-chip table was re-sat down to one row for the same reason: both AMD
rows measured a binary that no longer exists, and the first CI run handed out
Emerald Rapids (`family0x6-model0xcf`), new to the table under any binary. One
reading on a new chip and a new binary cannot split the 682 between them, and
the row said so.

A second run settled it. Zen 3 (`family0x19-model0x1`) refused on the same
branch and printed 41,378,393 — the same to the instruction as Emerald Rapids
on this binary, and Zen 3 had read 41,377,711 on the one before it. One chip,
two binaries: the whole 682 is the binary and none of it is the chip. Zen 3's
row goes back last, because the first row is what welfare and the golden read
and that authority stays where it was.
## 2026-09-05 (second) — two doors take scalars instead of boxes

Searched first: this morning's entry on the utf-8 door is the nearest
neighbour and is about a function's prologue rather than about its arguments.
The archive's 2026-08-31 entry on `k_b_append_grow` is the other half of the
same subject — what an `always_inline` in `runtime.c` can and cannot reach —
and it is cited below rather than repeated.

**An `always_inline` in `runtime.c` cannot reach a generated caller.** The
emitted module declares the source-level type, `%KValue @k_b_find2(%KValue,
%KValue, %KValue, %KValue)`; clang compiles the same C function to the
ABI-lowered `{i64,i64} @k_b_find2(i64 x6, ptr byval(%struct.KValue))`. LTO
sees two function types and keeps the call. Marking `k_b_find2` moved
jsonbench by zero instructions and left the symbol in the binary. The machine
ABI does agree — this is an inlining barrier and not a miscompile — and the
lever is the prelude shim, the mechanism already carrying `k_force_fast`,
`k_b_append_byte`, `k_b_length_fast` and ten others.

**find2.** Four KValues do not fit the six integer registers the SysV ABI
has: three fill them and the fourth arrives as a pointer to the caller's
stack, so the caller stores it and the callee loads and unpacks it before it
can splat the byte. That was fourteen of the function's fifty-four
instructions on jsonbench, where the scan itself was ten, and the entry
guards were seventeen more. `k_b_find2_raw` takes a pointer, a length and
three integers; `k_b_find2_fast` tests the four tags, reads the header, and
hands it those five.

```
    jsonbench     1,877,751,303 -> 1,827,146,403   -2.695%   (container)
    oneshot          29,863,200 ->    29,525,834   -1.130%
    encodebench   5,807,819,242 -> 5,807,486,906   -0.006%
```

Nine rows are byte-identical and none rises.

**slice.** The bytes arm builds a view — a header, a pointer and a length —
and reaching it cost three failure guards, two tag tests and the unboxing of
three KValues. `k_b_slice_raw` takes a pointer, a length and two integers and
holds the four compares and the add; `k_b_slice_fast` tests the three tags.

```
    jsonbench     1,827,146,403 -> 1,806,581,553   -1.126%   (container)
    widebench        58,270,995 ->    57,886,995   -0.659%
    oneshot          29,525,834 ->    29,388,735   -0.464%
    encodebench   5,807,486,906 -> 5,807,186,902   -0.005%
```

**And four rows rise, which is the shim's own cost showing.** They are the
programs that slice something other than bytes: every such call now pays
three tag tests before falling through to the same C function it always
reached. scanbench 1,425,333,592 -> 1,427,829,097, a rise of 2,495,505 and
0.175%; digestbench +256; basket +13; indexbench +2. Three rows hold exactly
still.

The objective takes the trade. Read on the container's own sitting for both
sides, so the comparison is against itself: 73.82 with find2 alone, 73.85
with slice beside it. The scan's 0.175% is real and it is smaller than what
the decode gains.

**What both cost in code.** The prelude grows one define, two calls, one
branch and twenty-five to thirty-one lines in every program whether it calls
the builtin or not, which is what the emitted goldens move by; the linker
drops the shim wherever nothing calls it, which is why only the programs that
use it grow `.text`.

**Both ship with a fixture watched red.** `find2_at_its_edges` covers a hit
at the first byte and the last, `from` below one and past the end, the
earlier of two bytes winning, no match, and on a twenty-four byte string a
hit inside the sixteen-byte vector step and one only the scalar tail reaches.
Changing the raw door's clamp from `from < 1 ? 0 : from - 1` to
`from - 1 < 0 ? 0 : from` moves three of its lines on native with the
interpreter untouched. `slice_at_its_edges` covers each end of the range on
bytes and a list and a string falling through; swapping `from` and `to` in
the shim's call empties the forward slices and fills the backwards one.

**And the one that did not need a shim.** `k_str_n` copies n bytes into a
fresh `KStr` and terminates it. It is `static` in `runtime.c` and so is every
caller, so an `always_inline` here does reach them — the barrier the two shims
went around is a barrier between the emitted module and the runtime, not
inside the runtime.

```
    indexbench        5,241,705 ->    4,791,711   -8.585%   (container)
    scanbench     1,427,829,097 -> 1,406,297,911   -1.508%
    jsonbench     1,806,581,553 -> 1,791,154,032   -0.854%
    oneshot          29,388,735 ->   29,275,330   -0.386%
    pendbench       702,115,194 ->  700,501,409   -0.230%
```

basket, widebench, encodebench and digestbench fall too. Two rows rise by
twelve and fourteen instructions. One rises for real: readbench
2,000,657,400 -> 2,038,393,555, a rise of 1.886%. That benchmark reads a file
and slices it, so it holds few short strings and many long ones, and an
inlined copy loop loses to a called one when the copy dominates.

**Its price is machine code, and it is the largest of the three by far.** All
twelve `.text` rows rise, 400 to 5,024 bytes and 34,320 in total, where the
two shims grew four rows and eight. A prelude twin is one function the linker
drops where nothing calls it; this is a copy loop written out at every site
that makes a string. jsonbench spends 4,320 bytes for 15,427,521 instructions
and scanbench 5,024 for 21,531,186. indexbench spends 1,952 for 449,994,
the worst rate of the twelve, and still falls 8.585%.

**The objective cannot see that price.** welfare weighs allocations, arena
blocks, instructions, fixpoint rounds, expression visits and emitted lines,
and has no term for the size of the machine code — so 73.85 -> 73.90 is the
sum over everything it does weigh, and the 34,320 bytes are priced only in
`bench/text_golden.txt`. Whether the index should carry a `.text` term is a
question about the weights and it is left as one here, with the measurement
that raises it. Nothing about these three changes turns on the answer: each
of them rises on the current model and each of them rises on any model that
weighs `.text` at less than what the instruction falls are worth.

**Every code counter the two twins moved, with the value it landed on.** Two
prelude twins are two `define`s, four `call`s, two `branch`es and about fifty
lines written into every module the emitter produces, whether the program
calls the builtin or not; the linker drops what nothing calls, so the machine
code grows only where it is used. `bench/compile_golden.txt` reads `defines`
169, `calls` 217, `branches` 262, `lines` 4,147, and its `rounds` and `visits`
do not move at all — the emitter writes more and deciding what to write costs
the same. `bench/compile_golden_modules.txt` reads `module_defines` 93,
`module_calls` 777, `module_branches` 413, `module_lines` 4,997.
`bench/emitted_golden.txt` reads `emitted_defines` 176, `emitted_calls` 1,855,
`emitted_branches` 1,198, `emitted_lines` 12,273, and
`bench/emitted_golden_others.txt` reads `emitted_other_defines` 1,565,
`emitted_other_calls` 14,869, `emitted_other_branches` 8,856 and
`emitted_other_lines` 90,688. `text` lands on 1,117,272, of which 34,320 is
`k_str_n` and the rest the two twins.

**CI's sitting, which is what the goldens carry.** The rows above are the
container's. Nine of the runner's twelve fall: `work_jsonbench` lands on
1,791,154,445 (−4.612% against main), `work_indexbench` on 4,792,124
(−8.584%), `work_scanbench` on 1,406,298,324 (−1.335%), `work_oneshot` on
29,275,729 (−1.968%), `work_widebench` on 57,839,394 (−0.741%),
`work_pendbench` on 700,501,795 (−0.230%), `work_basket` on 39,875,796,
`work_encodebench` on 5,802,859,663 and `work_digestbench` on 80,995,637.
scanbench's is the number to read for the slice twin's cost: that twin took
2,495,505 off it on its own and `k_str_n` gave back more, so the vein records
a fall where the per-change readings record a rise and then a larger fall.

**Three rise, and all three are named here because the trend gate refuses a
move with no sentence.** `work_deepbench` lands on 678,049,725 and
`work_escapebench` on 130,170,757 — twelve and fourteen instructions, the
twins' tag tests on the few calls those two programs make. `work_readbench`
lands on 2,038,393,968, a rise of 37,736,155 and 1.886%, and it is `k_str_n`:
that benchmark reads a file and holds few short strings and many long ones,
where an inlined copy loop loses to a called one. `compile_instructions` lands
on 41,378,619, a rise of 226 — held on ONE chip across both binaries, Emerald
Rapids reading 41,378,393 before the twins and 41,378,619 after, so none of it
is the silicon. The prelude gained two twins and `runtime.c` two raw doors;
`kanso check lib/json` emits nothing, so the front end does the same work.

**welfare 73.77 -> 73.90**, banked with `--set` in the same commit.

## 2026-09-05 (third) — the argument that would not fit

`utf8` applied to `slice` has been lowered as a single call since the fusion
landed, and that call carries three `KValue`s and a pointer to the wrapper's
origin: seven register-sized arguments where the SysV ABI has six. The seventh
spills to the caller's stack and the callee reloads it before it can put the
origin in an err. `k_b_utf8_slice_raw` takes a pointer, a length, two integers
and the origin — five — and the `k_b_utf8_slice_fast` twin in the prelude
tests the three tags and calls it. This is the third door to take the shape
kanso#1247's §48 describes, and the first where the spill rather than the
unpack was the thing to remove.

**Held on one host, in one directory, against a clean build of the commit it
sits on.** jsonbench 1,791,154,032 -> 1,783,182,882, a fall of 7,971,150 and
0.4450%. oneshot 29,275,330 -> 29,222,188, 53,142 and 0.1815%. encodebench
5,802,859,264 -> 5,802,805,722, 53,542 and 0.0009%. widebench's binary changed
and its count did not move at all. The other eight benchmark binaries came out
BYTE-IDENTICAL to the ones before the change, so those eight rows could not
have moved and were not measured twice. Nothing rises.

**`.text` grows 32 bytes, 16 each on jsonbench and oneshot**, and the other
ten rows are unchanged because their binaries are. The twin is `internal` and
the linker drops it where nothing calls it, so the whole cost lands on the two
callers. Beside `k_str_n`'s 34,320 bytes across all twelve rows in kanso#1247,
this is the same mechanism reading the other way, and it is one more case for
the `.text`-in-welfare question sitting in design/pending-gavels.md rather than
an answer to it.

**The fixture is tests/golden/micro/utf8_of_a_byte_slice_at_its_edges.kso, and
the door had none.** tests/golden/runtime/utf8_of_a_slice_names_the_wrapper.kso
reaches the same lowering with a LIST, which falls through to the general slice
and the general `utf8`; the bytes arm — the arm the fusion exists for, and the
one the raw door now holds — was pinned by nothing. The fixture walks every
range the clamp refuses and both sides of the validity test, including a
two-byte sequence taken whole, the same sequence cut in half, and its
continuation byte alone. Watched red twice: with `k_utf8_bad` dropped from the
raw door the two invalid cases print raw bytes instead of refusing, and with
`to > blen` dropped from the clamp the output goes to binary.

**A measurement mistake, recorded because the gate now carries the fix.** The
first A/B built the baseline in a scratch worktree and the twin in the repo
and measured each where it was built. escapebench and digestbench each read
+14, and both binaries were byte-identical to the baseline's. The same
escapebench counts 130,170,344 under one path and 130,170,358 under the other:
the kernel puts the exec path on the new process's stack beside the
environment, and libc walks that before main. `scripts/gates/instructions.sh`
already empties the environment for this reason and said nothing about the
path; it does now. CI always runs from the repo root, so no golden was ever
wrong — only a local comparison across two directories, which is how this vein
gets measured while a change is being decided.

**Every code counter the twin moved, with the value it landed on.** One prelude
twin is one `define`, two `call`s, one `branch` and twenty-six lines written
into every module the emitter produces, whether the program reaches `utf8` over
a `slice` or not. `bench/emitted_golden.txt` reads `emitted_defines` 177,
`emitted_calls` 1,857, `emitted_branches` 1,199 and `emitted_lines` 12,298;
`bench/emitted_golden_others.txt` reads `emitted_other_defines` 1,576,
`emitted_other_calls` 14,891, `emitted_other_branches` 8,867 and
`emitted_other_lines` 90,971. `bench/compile_golden.txt` reads `defines` 174,
`calls` 227, `branches` 267 and `lines` 4,277, with `rounds` and `visits`
unmoved at 12 and 115 — the emitter writes more and deciding what to write
costs the same. `bench/compile_golden_modules.txt` reads `module_defines` 94,
`module_calls` 779, `module_branches` 414 and `module_lines` 5,023, its
`module_rounds` and `module_visits` also unmoved. `text` lands on 1,117,304.

**CI's sitting, which is what the goldens carry, and it agrees with the
container to the instruction.** `work_jsonbench` lands on 1,783,183,295,
`work_oneshot` on 29,222,587 and `work_encodebench` on 5,802,806,121 — falls of
7,971,150, 53,142 and 53,542, each of them the SAME NUMBER the container
measured on its own pair of builds. Nine rows hold exactly still. That
agreement is worth naming: the two hosts count different totals for every row
and have never before matched on a delta, let alone on three.

**`compile_instructions` lands on 41,377,663, a FALL of 956**, and this one
needs no second chip either: family0x19-model0x11 counted 41,378,619 on the
binary before the twin and 41,377,663 on the one with it, so none of the 956 is
the silicon. `kanso check lib/json` emits nothing, so the door that changed
cannot run during the measurement; the compiler's own bytes moved, which is
layout for the eighth time on this vein and the first time it has gone down for
a runtime edit. The Emerald Rapids and Zen 3 rows measured the binary before
this change and are removed rather than carried with the delta applied.

**welfare 73.90 -> 73.91**, banked with `--set` in the same commit.

**And the callee-saved question got its missing number, measured on this
binary.** The log has carried "9.6% at entry" since the LLVM 19 investigation,
with the per-function split recorded as unmeasured. It is 5.569% —
99,299,946 instructions of 1,783,182,882 — counted by taking each symbol's
pushes of rbx, rbp and r12-r15 from `objdump`, multiplying by callgrind's call
count for that symbol, and doubling, because every call runs all the pushes and
the taken exit runs the matching pops (a `musttail` jump restores them first,
so it counts too). Where it sits: `d_jsonbench/parse_value_2` 32,569,200
(1.826%, 2,714,100 calls over six registers), `obj_key_start_4` 15,049,800,
`k_b_utf8_slice_raw` 10,442,400, `k_b_slice_raw` 5,391,000, `number_done_4`
5,060,400, `k_buf` 5,026,200, `array_step_3` 4,953,600, `k_b_to_int` 3,769,200,
`obj_items_3` 3,360,000, `k_b_append_grow` and `str_char_4` 3,191,400 each,
`k_rec` 3,076,500, `k_str_lit` 2,127,616, `k_b_bytes` 1,064,400,
`k_map_view_insert` 1,026,000. The generated decoder carries 3.6 points of the
5.569 and the runtime's C doors carry 1.969, so a `preserve_none` that reached
only the emitted functions would still be the larger half. The toolchain is
still a version short of it and shrink-wrapping is still ruled out by the exit
counts, so nothing here is actionable yet; what changes is that the size of the
prize is a measurement rather than an estimate, and it is smaller than the
estimate was.

## 2026-09-05 (fourth) — the largest spill, and the census that found it

Three doors had been twinned by reading the profile and picking what looked
expensive. The census asks the question directly instead: which of the doors
the emitter declares take more than the six integer argument registers the
SysV ABI has? A `KValue` is two of them, so the answer is countable from
`src/codegen.rs` alone, and it is five — `k_call4` and `k_b_find2_below` at
ten, `k_call3` and `k_b_find2` at eight, `k_b_utf8_slice` at seven.

`k_b_find2` and `k_b_utf8_slice` were already done. `k_call3` and `k_call4`
appear in no benchmark profile at all. That leaves `k_b_find2_below`, which
carries 481,347,200 instructions of self cost in encodebench — 8.3% of it —
and spills FOUR arguments, more than any other door in the tree.

**The measurement, held on one host in one directory against a clean build of
the commit it sits on.** encodebench 5,802,805,722 -> 5,563,212,122, a fall of
239,593,600 and 4.1289%. oneshot 29,222,188 -> 28,623,204, 598,984 and 2.0498%.
widebench's binary changed and its count did not move at all; jsonbench's
binary is byte-identical, and so are the other eight, so those nine rows could
not have moved. Nothing rises. `.text` grows 224 bytes on each of the three
programs whose binaries changed and nothing on the other nine.

The twin takes about half of what the door spent. Four spilled arguments are
four stores in the caller and four loads and unpacks in the callee, on every
call, in front of a scan that is a compare and a branch per byte.

**And the census is worth keeping as a method.** `k_b_slice` fits in six
registers and its twin still won, on the guards rather than the spill, so the
overflow test is necessary for THIS mechanism and not sufficient for finding
every candidate. What the count does give is a closed list: after this one,
the only doors left that overflow are two the corpus never calls, so this
family is finished unless a new door is declared or the two call doors start
appearing in a benchmark.

**tests/golden/micro/find2_below_at_its_edges.kso**, twelve cases. The one
line in `bytes_are_their_own_value` took a hit on the first byte of a
four-byte string and nothing else was pinned: not the clamp, not the limit
guard that chooses between the sixteen-byte vector step and the scalar loop,
not the tail the vector step cannot reach. The fixture covers each of the
three ways a byte can match, both ends of the range, a hit inside the vector
step and one only the tail reaches, and both ways the limit guard turns the
vector path off — a zero limit, where the below-test can never fire, and a
limit past any byte, where it fires on the first. Native and the interpreter
agree byte for byte. Watched red: dropping the below-limit test from the raw
door's scalar tail moves exactly the two cases that turn on it.

**A counter nearly doubled in silence.** The body extracted into the raw door
already opened with `k_stat_find2_calls++`, and the prologue written in front
of it added a second. `find2_calls` is pinned in the cost goldens, so the
first CI run would have refused a counter that moved for no reason anybody
could name. It was caught by reading the extracted function before building
it, which is the only reason it is a footnote rather than an entry of its own.

**Every code counter the twin moved, with the value it landed on.** A fourth
prelude twin is one `define`, two `call`s, one `branch` and thirty-five lines
written into every module the emitter produces, whether the program reaches
`find2_below` or not. `bench/emitted_golden.txt` reads `emitted_defines` 178,
`emitted_calls` 1,859, `emitted_branches` 1,200 and `emitted_lines` 12,332;
`bench/emitted_golden_others.txt` reads `emitted_other_defines` 1,587,
`emitted_other_calls` 14,913, `emitted_other_branches` 8,878 and
`emitted_other_lines` 91,353. `bench/compile_golden.txt` reads `defines` 179,
`calls` 237, `branches` 272 and `lines` 4,452, with `rounds` and `visits`
unmoved at 12 and 115. `bench/compile_golden_modules.txt` reads
`module_defines` 95, `module_calls` 781, `module_branches` 415 and
`module_lines` 5,058, its `module_rounds` and `module_visits` also unmoved.
`text` lands on 1,117,976 — 672 bytes over three programs, and the other nine
rows do not move because their binaries are byte-identical to the ones before
this change.

**CI's rows, and the two hosts agreed to the instruction again.**
`bench/instructions_golden.txt` reads `work_encodebench` 5,563,212,521 and
`work_oneshot` 28,623,603 — falls of 239,593,600 and 598,984, which are the
container's two deltas exactly, on a different host with different absolute
values. That is the second consecutive change on which this vein has recorded
exact agreement between the runner and the container; the utf8-over-slice twin
was the first. The other ten rows hold to the digit.

**The compile row moved and the table is one row again.** `src/runtime.c` is
`include_str!`'d into the compiler, so a door added to it moves the compiler's
own bytes and the layout under them, and every chip's row goes stale at once.
CI drew `family0x19-model0x1`, found no row for it — the previous entry's block
had removed the stale one — and refused rather than comparing against somebody
else's number. `compile_instructions` lands on 41,377,855 where the golden
carried 41,377,663, and the Zen 4 row it replaces measured the binary before
this one and is removed rather than carried forward with the delta applied.
`kanso check lib/json` emits nothing, so the scan door that changed cannot run
during the measurement; what moved is layout, for the ninth time on this vein.
`compile_allocs` 25,490, `compile_peak_bytes` 715,275, `rounds` and `visits`
are byte-identical.

**welfare 73.99**, from 73.91, and `--set` in this same commit.

## 2026-09-05 (fifth) — a lambda that captures nothing is one value

**Searched first**, as the filing gate requires: `grep -n "closure" design/compiler-log.md
design/log/compiler-log-archive.md` returns the closure-call dispatch work
(kanso#1119, the builtin's count at the front door) and the arity divergence
(kanso#1229), neither of which touches how a closure is BUILT. `k_closure` has
allocated a header and an environment per evaluation since the emitter first
wrote one, and nothing in the log or the archive proposes otherwise.

**The find came out of the profile the find2_below twin left.** After that
change encodebench sits at 5,563,212,122 and `k_closure` is 709,200 calls and
48,225,600 instructions of it, 0.87%, with 39,006,110 of that its own body.
Every call is TWO arena allocations — a `KClosure` and an environment, sized
`ncaps ? ncaps : 1` so the empty one is allocated too — and 1,418,400 of
encodebench's 16,249,027 allocations are that pair. Two of the three
`k_closure` sites the emitter writes for encodebench pass `ncaps` 0.
`escape_able_2` is the hot one: it allocas a one-slot array, stores nothing
into it, and hands `list/fold` a closure that never differs.

**A lambda that captures nothing is the same value every evaluation, which is
what `str_const` already says about a string literal** — "a literal is the
same value every evaluation, so it builds once into a permanent slot instead
of allocating per visit". `k_closure_lit` is `k_str_lit` with a closure in it:
a per-site `KValue` cell, `k_alloc_perm` on the first visit, the cell returned
on every visit after. Codegen emits it whenever the capture list comes out
empty, and the alloca goes with the allocations.

**The measurement, one host, one directory, the base binaries swapped in place
so the exec path cannot move the count.**

    encodebench   5,563,212,122 -> 5,527,056,277   -36,155,845   -0.6499%
    digestbench      80,995,238 ->    80,558,930      -436,308   -0.5387%
    oneshot          28,623,204 ->    28,533,106       -90,098   -0.3148%
    scanbench     1,406,297,911 -> 1,406,247,294       -50,617   -0.0036%
    deepbench       678,046,045 ->   716,507,476   +38,461,431   +5.6724%
    basket, widebench, pendbench, readbench and jsonbench rise by 3,200 or
    less each; escapebench and indexbench do not move at all.

**deepbench's rise is code layout and callgrind says so with call counts.**
The carry walk visits the SAME nodes: `k_copy_size'2` is entered 859,622 times
before and 857,697 after, `k_copy_size` 1,534,544 and 1,535,314, `k_ptrmap_at`
1,571,990 and 1,571,239 — every one within 0.2%. What moved is the cost of a
visit: 273,639,699 instructions over 2,394,166 entries is 114.3 apiece against
301,674,351 over 2,393,011, which is 126.1. Twelve instructions more per call,
in a function this change does not touch, on a program that enters it two and
a half million times. Removing the allocas from the emitted module moved
clang's inlining budget inside `k_copy_size` under LTO.

**Two attempts to steer it, both declined by measurement.** The first marked
every literal closure with a shared sentinel environment and short-circuited
it to "survives" in `k_slots_survive`, `k_interior_survives`, `k_copy_size`
and `k_deep_copy`, on the theory that permanent storage answering no to
`k_survives` was forcing copies. deepbench went to +6.0799% and took basket,
widebench and pendbench up with it — more code in the same hot walk, and the
theory was wrong anyway: deepbench's own `k_closure` only falls 26,840,055 to
23,980,088, so most of its closures capture something and were never
literals. The second marked `k_closure_lit` `noinline, cold`: deepbench stayed
at +5.63% and encodebench's gain shrank from 0.6499% to 0.2805%. A probe that
added the same function under a name codegen never emits left every benchmark
binary BYTE-IDENTICAL, which is the useful negative — the layout move needs
the emitted module to change, not just runtime.c to grow.

**The objective takes the trade.** welfare reads 73.99170779271341 against
74.00198125390742 on the container's own sitting for both sides, a rise of
0.01027. It is a thin one, and the reason it is thin is that RUNTIME
ALLOCATION COUNTS ARE NOT IN THE OBJECTIVE — only peak bytes and arena blocks
are — so the 1,418,400 allocations encodebench stops making score exactly
zero. The instruction rows are the whole of what welfare sees here.

**tests/golden/micro/a_lambda_that_captures_nothing_is_one_value.kso**, ten
cases, because sharing closure identity is the new risk and nothing pinned it.
Two capture-free lambdas in one body each need their own slot; a lambda that
DOES capture must still get a fresh closure per call. Watched red twice.
Routing a one-capture lambda through the literal path moves exactly the three
capture lines — "a capture of five" 6 to 1, "a capture of nine" 10 to 1, "five
again, after nine" 6 to 1, because the capture is never stored — and leaves
the seven capture-free lines alone. Giving every lambda site ONE cell moves
"two lambdas in one body" 115 to 20, since `b` resolves to `a`, and turns the
fold's two-argument lambda into `error[runtime]: this function takes 1
argument(s), got 2`.

**Every counter that worsened, with the value it landed on.** The emitter
writes one global and one call where it wrote an alloca and a store, and the
globals are lines nothing removes: `emitted_lines` 12,344, `module_lines`
5,068, `emitted_other_lines` 91,445, `text` 1,119,560. `emitted_defines`,
`emitted_calls` and `emitted_branches` hold at 178, 1,859 and 1,200, and the
five programs in `bench/compile_golden.txt` hold entirely — none of them
writes a lambda. `.text` FALLS on the two programs with no capture-free
lambda to convert, escapebench 52,658 to 52,466 and indexbench 56,002 to
55,810, and rises on the other ten.

**The mem vein moves in both directions and the trade is the point.** Fifteen
files trade arena allocations for permanent ones, one pair per lambda site,
paid once for a process rather than once per visit, and each lands on a value
worth writing down: a_digest_holds_every_block_it_walked_perm_allocs 34,
a_repaired_node_below_the_mark_holds_tenure_perm_allocs 20,
fused_tally_perm_allocs 9, effect_push_shape_perm_allocs 6,
fold_push_shape_perm_allocs 6, fused_map_shape_perm_allocs 6,
early_exit_perm_allocs 5, fused_select_shape_perm_allocs 5,
skip_shape_perm_allocs 5, sort_shape_perm_allocs 5, take_shape_perm_allocs 5,
a_loop_invariant_capture_is_copied_every_rewind_perm_allocs 4,
tally_shape_perm_allocs 4, fused_reducer_perm_allocs 3 and
piped_reducer_perm_allocs 3. Against that, `allocs` falls in every one of
the fifteen — 1,980 to 1,850 in the digest, 5,137 to 5,133 in the repaired
node, 23 to 19 in early_exit — and `alloc_bytes` with it.

The work rows and welfare wait on CI's sitting, and the compile row goes stale
with them: both `src/codegen.rs` and `src/runtime.c` changed, so the compiler's
own bytes moved for the tenth time on that vein.

**CI's sitting, and the two hosts agreed on ten of twelve rows.** Four work
rows fall: work_encodebench 5,527,056,676 (−36,155,845, −0.6499%),
work_digestbench 80,559,329 (−436,308, −0.5387%), work_oneshot 28,533,505
(−90,098, −0.3148%) and work_scanbench 1,406,247,707 (−50,617). Every one of
those four deltas is the container's TO THE INSTRUCTION, as are
work_jsonbench's, work_basket's, work_readbench's, work_widebench's and both
unmoved rows — ten of twelve, the third consecutive change on which this vein
has recorded exact agreement between runner and container.

**The two the hosts disagree on are exactly the two whose rise is layout.**
work_deepbench 716,506,831 (+38,457,106, +5.6724%) against the container's
+38,461,431, and work_pendbench 700,529,602 (+27,807) against +27,780. A
layout effect is a property of the binary a host built, so a per-host
difference there is the shape to expect; a work difference would not have
been. The remaining rises are the same thing smaller: work_basket 39,878,996
(+3,200), work_readbench 2,038,397,546 (+3,578), work_jsonbench 1,783,183,673
(+378), work_widebench 57,839,754 (+360). work_escapebench 130,170,757 and
work_indexbench 4,792,124 do not move at all, because neither program has a
capture-free lambda to convert.

**Every counter vein moved the same way, and the trade is one shape.** Nine
cost goldens swap arena allocations for a permanent pair per lambda site.
`allocs` falls in all nine — decode 4,999,965, encode 14,830,625 (−1,418,402),
oneshot 75,822, basket 28,170, pend 4,007,400, scan 3,975,888, wide 144,027,
digest 213,707, read 615 — and `alloc_bytes` with each. Against that,
`perm_allocs` lands on decode_perm_allocs 6, encode_perm_allocs 14,
oneshot_perm_allocs 12, basket_perm_allocs 41, pend_perm_allocs 25,
scan_perm_allocs 51, wide_perm_allocs 8, digest_perm_allocs 36 and
read_perm_allocs 5. Three more pending-cell counters move with the shape of
what the evacuation now walks: pend_evac_bytes 501,056, pend_survive_slots
118,477 and pend_sh_buf 32,134,704.

**compile_instructions 41,379,503**, re-sat on family0x19-model0x11 after CI
refused an unrecorded chip. This move is not only layout: the emitter writes
one global and one call where it wrote an alloca and two stores, so
`kanso check lib/json` really does emit different bytes. compile_allocs holds
at 25,490 and compile_peak_bytes at 715,275.

**Two book panels went with the counters** and nothing but the full check
caught them: docs/book/samples/ch10/counters_counters.out and
docs/book/samples/ch12/fused_counters.out both print `allocs` and
`perm_allocs`, and both move 12 to 10 and 1 to 3. The ch10 counters sample is
named in CLAUDE.md's list of veins to regenerate together, and this session
regenerated the .mem files and the four code goldens and missed it.

**welfare 74.00**, from 73.99, and `--set` in this same commit.

**Three chips counted the same compile row and all three agreed exactly.** The push
that landed CI's sitting moved only goldens, the log and the pages, so the
compiler binary was byte-identical to the one Zen 4 had counted. CI then drew
family0x19-model0x1, found no row and refused, as an unrecorded chip must.
It read 41,379,503 — Zen 4's value to the instruction. The next round drew
family0x6-model0xcf, the Intel, and read 41,379,503 as well.

**That meets the condition this vein set for itself.** The block written when
the utf8-over-slice twin landed said that three keys agreeing once is one
binary, and that a later change showing the same three agree again is the
evidence for asking whether the table should collapse — an argument made about
the table, with the readings behind it, rather than a tidy-up. This is that
second binary. Emerald Rapids, Zen 3 and Zen 4 now agree to the instruction on
two of them, and the reason is legible: kanso#1241 cut this row down to the
compiler's own frame, and the glibc ifunc dispatch the key was invented to
separate lives in the loader and libc, outside that frame.

The argument is not made in this change, which is a codegen change carrying a
fixture and nine goldens; the second reading is recorded so it can be made
from two rather than from one. What the current design cost here is worth
recording beside it: this branch spent THREE CI rounds adding three rows that
all say the same number, because the pool hands out a chip at random and every
unrecorded one is a refusal. The rows are appended last, because the first is
the one the golden carries.

## 2026-09-05 (sixth) — ten cost goldens, and the file that said four

**Searched first**, as the filing gate requires. `grep -n "cost golden"
design/compiler-log.md design/log/compiler-log-archive.md CLAUDE.md` returns the
counter-regeneration rule in several places and no entry proposing a single
local sweep; `ls scripts/gates/*counters*.sh` returns ten files and nothing in
the tree runs them together.

### What the gap cost

kanso#1249's first push regenerated the mem vein, the four code goldens and the
wasm blob, and missed nine `bench/cost_golden*.txt` files and two book panels.
CI found them a round late. The cause was not carelessness about the list, it
was the list: CLAUDE.md's "Counters changed → regenerate every vein in the same
PR" said **all four cost goldens**, and there are ten. A session that read the
instruction and followed it exactly would still have missed six.

### The sweep

`scripts/gates/all_counters.sh` builds the benchmarks once, then walks a table
of ten `vein:program:golden` rows and diffs each measured file against its
golden with the comment headers stripped. It does not stop at the first
disagreement — the point of having one command is to learn about all ten in one
run — and it prints `counters moved: …` naming every vein that differs.
`--write` rewrites the data rows in place while preserving each golden's
comment header, which a `cp` of the measured file destroys;
`bench/cost_golden_wide.txt` and `bench/cost_golden_pend.txt` both carry one.

### The spec, watched red

`tests/every_counter_gate_is_in_the_sweep.rs`, three tests.

The first compares the sweep's table against `scripts/gates/*_counters.sh` on
disk, so a gate added without a row turns it red. Its first version demanded a
row for a vein called "all", because the glob matched `all_counters.sh` itself;
that is recorded in the test's doc comment as how it proved it could fail. The
second checks that every row names a golden and a program that exist.

The third extracts the word immediately before "cost goldens" in CLAUDE.md and
compares it to the number of `bench/cost_golden*.txt` files. With "four"
restored it reads *there are 10 cost goldens and CLAUDE.md counts them "four"*.
It was written wrong the first time: it asserted that the claim contained "10",
which is true with "four" written, because the same line names the ch10
counters sample. A spec that cannot fail is what this log has caught more than
once, and this one was caught by trying to break it before believing it.

### The sweep run against two real trees

Green on a clean tree, and green again on a tree carrying an unrelated
runtime.c change that moves instructions but no counted event — which is the
answer that change needed and could not otherwise have had cheaply.

Watched red by perturbing `allocs` in two goldens at once. It reports BOTH:

    === encode (bench/cost_golden_encode.txt)
    1c1
    < allocs=999999
    ---
    > allocs=14830625
    === pend (bench/cost_golden_pend.txt)
    1c1
    < allocs=888888
    ---
    > allocs=4007400
    counters moved: encode pend
    run with --write to regenerate them, then say why in design/compiler-log.md

Exit 1 when anything moved, 0 when nothing did. Not stopping at the first
disagreement is the property worth having: a session that learns about one
moved vein per build learns slowly.

### Two CI rounds this branch could not have avoided

The first was mine: the row check asserted `bench/<program>` exists, and
`bench/jsonbench/` is gitignored and written by `bench/make_jsonbench`, so it
is in any tree that has run the benchmarks and in no fresh clone. It read
state the repository does not carry, which is the shape of the bug this whole
file is about. The fix accepts either the directory or its generator.

The second was the compile row. CI drew `family0x6-model0x6a`, an Ice Lake-SP
with no row in `bench/compile_instructions_by_cpu.txt`, and refused — on a
value of 41,379,503, which is what the three recorded chips read. That is
four keys reading one number on one binary, and nothing here touches the
compiler: CLAUDE.md, a shell script and a Rust test file are none of them
`include_str!`'d. The row is recorded with the reading beside it. The
argument for collapsing the table still has the 5,064-instruction
falsification to answer and is not made here.

## 2026-09-05 (seventh) — a condition is asked, not read

Every work row in the corpus falls and none rises, both sides counted by CI:

    digestbench     80,559,329 ->     77,290,591    -3,268,738   -4.0576%
    encodebench  5,527,056,676 ->  5,322,691,304  -204,365,372   -3.6975%
    oneshot         28,533,505 ->     27,733,120      -800,385   -2.8051%
    pendbench      700,529,602 ->    681,319,796   -19,209,806   -2.7422%
    jsonbench    1,783,183,673 ->  1,745,058,173   -38,125,500   -2.1381%
    indexbench       4,792,124 ->      4,692,123      -100,001   -2.0868%
    widebench       57,839,754 ->     57,201,224      -638,530   -1.1040%
    scanbench    1,406,247,707 ->  1,395,728,077   -10,519,630   -0.7481%
    basket          39,878,996 ->     39,737,538      -141,458   -0.3547%
    deepbench      716,506,831 ->    714,674,831    -1,832,000   -0.2557%
    escapebench    130,170,757 ->    130,170,757             0    0.0000%
    readbench    2,038,397,546 ->  2,038,397,546             0    0.0000%

escapebench and readbench do not move by a single instruction, and that is
the control rather than a disappointment: neither has a comparison in a hot
loop — one tagged comparison in escapebench's whole emitted IR, two in
readbench's — so a change that only touches comparisons must leave them
exactly alone, and it does.

Nothing in the runtime or the libraries changed. This is `src/codegen.rs`.

### What the `if` was doing with a comparison

`b < 32` in the json escape path emitted an `icmp slt` and then a `select` on
it, building a KValue tagged 2 or 3. A phi merged that with `k_cmp`'s answer
from the guarded slow arm, and the `if` then called `k_not_failure` and
`k_truthy` on the phi and branched on the result. LLVM folds the pair away
where the select reaches the branch directly; through the phi it cannot,
because `k_cmp` may answer with a failure. What survived to machine code was

    xor %eax,%eax ; cmp $0x20,%r15 ; setl %al ; xor $0x3,%rax
    xor %edx,%edx ; cmp $0x2,%rax  ; jne

`setl` becomes a tag and two instructions later the tag is compared back.
`cmp $0x20,%r15 ; jge` is the whole of the work.

### How it was found, and how much that one site cost

Task #304 asked where the 66 instructions a byte in the escape fold go.
callgrind with `--dump-instr=yes`, merged with objdump, attributes the
ordinary-byte path — 9,834,000 of 11,658,800 steps — to the instruction: 20
for the frame, 13 for value plumbing, 25 for the inlined byte append of which
13 are guards, and 11 for the `b < 32` test. Those eleven are 108,174,000
instructions where 19,668,000 would do, **1.6013% of encodebench from one
comparison**.

The same 66 also answered #304's own premise. Nothing is heap-boxed: a KValue
is two words in registers, so "boxes both operands" named a tag
materialisation rather than an allocation. The step's real costs are the
indirect dispatch through the closure's stored `w_klam17` pointer, the frame
(six callee-saved pushes for a function that writes one byte, which is #290's
dimension), and this tag round trip.

### The change

`emit_cond` replaces `emit_expr` on the `if`'s condition. Where the condition
is a comparison and neither operand routes to a user arm, the guarded compare
branches straight to the then and else labels from its own i1 and builds
nothing. The slow arm is unchanged — `k_cmp`, a failure test, a truthiness
test — and its failure becomes one more arm of the `if`'s phi. Both `if` sites
go through it: the value form, where a failing condition joins the merge, and
the tail form, where it returns.

### What it costs

No allocation counter moves; all ten cost goldens are byte-identical. Emitted
code and machine code both fall: module_lines 5,068 -> 4,970, module_calls
781 -> 769, emitted_lines 12,344 -> 12,134, emitted_calls 1,859 -> 1,835,
emitted_branches 1,200 -> 1,163, emitted_other_lines 91,445 -> 90,198,
emitted_other_calls 14,913 -> 14,728, emitted_other_branches 8,878 -> 8,669,
and text 1,119,560 -> 1,117,736.

ONE counter worsens, and it is the one the change is made of:
**module_branches 415 -> 420**. Five branches is what a select and a phi and
two calls become when the comparison decides the branch itself, and it buys
twelve fewer calls and ninety-eight fewer lines in the same file, along with
every fall listed above. The trend gate asked for it by name and by the value
it landed on, which is 420.

compile_allocs and compile_memory hold: 25,490 allocations and 715,275 peak
bytes, both unchanged. compile_instructions RISES by 519, from 41,379,503 to
**41,380,022** on family0x19-model0x1 — 0.00125% of the front end.

`kanso check lib/json` emits no LLVM IR, so the front end runs no line this
change touched. What moved is the compiler's own layout: codegen.rs gained a
routine and everything after it sits at a different address. This repo has
measured that artifact before, when adding and removing allocas moved clang's
inlining budget inside untouched functions; 519 instructions is its size here.
The next run drew family0x6-model0xcf, an Emerald Rapids, and read the same
41,380,022 — a different vendor and a different microarchitecture landing on
the instruction. Two rows are re-sat and two are stale, and they stay stale
until CI draws them, one per run; the golden's bare line tracks the first of
those, so it does not move yet either.

That second reading matters beyond this change. The per-chip key exists
because two AMD models once read 41,503,893 and 41,498,829 on ONE binary, and
that divergence predates kanso#1241, which cut this row down to the compiler's
own frame and left the ifunc dispatch the key was invented to separate outside
the measurement. Every reading since has agreed: four chips on the old binary,
and now two on the new one before a third has been asked.

### The fixtures, and the pair that passed for the wrong reason

`tests/golden/micro/a_condition_is_asked_rather_than_read` runs the fast arm
in both `if` forms, a text comparison through the slow arm, and a `<` declared
over a record that must still reach its arm. Two runtime fixtures pin a
failing comparison, one per `if` form.

Both runtime fixtures were written first with the failure handed in as an
ARGUMENT, and passed. A failing argument short-circuits the arm before its
body runs, so the comparison was never reached: breaking the slow arm's
failure test left them green. Binding the failure inside the body reaches it,
and the endpoint trace shows the difference by losing its "passed through"
line. Found by trying to break them, which is the only reason it was found.

Three breaks, three reddenings. Inverting the fused branch moves the two
fused rows and leaves the text and record rows alone. Making the slow arm
skip its failure test reddens both runtime fixtures. Fusing where a record
routes to a user arm kills the micro program.

## 2026-09-05 (eighth) — the library's encoder gets a watcher

**DONE.** `bench/livebench` runs encodebench's program — decode
`bench/large.json` once, encode it four hundred times — against the library
that ships. It prints encodebench's exact checksum, 74072800, so the two do the
same work, and the difference between them is the only thing that separates
them: which copy of lib/json they hold.

**work_livebench 5,312,541,181**, MINTED. The baseline carries no reading for
it, so the first one is a measurement rather than a move. Beside encodebench's
5,322,690,905 that is a gap of 10,149,724, or **0.19%** — the frozen control's
drift from the shipped library since the snapshot was taken at 20ab931d on
2026-08-07. `live_instructions` joins the objective as a granted baseline,
entering at its dimension's standing, so welfare holds at 74.1474896668362 on
landing day and the term pays and charges from here.

### Why a benchmark that duplicates one already in the corpus

`bench/encodebench` and `bench/widebench` vendor frozen snapshots of lib/json,
and their READMEs say why: with the library held still, a move there is the
compiler's. kanso#1230 is the recorded case where that control did the
separating, and it stays.

What the freeze costs is that nothing then watches the library's own encoder at
load. `bench/jsonbench` is generated from lib/json but only decodes.
`bench/oneshot` imports the real library and encodes exactly once, inside a
program whose bulk is the decode beside it. So the whole encode side of the
shipped library was watched by a single encode of a 185 KB document.

That is not a hypothetical. A twelve-line change to lib/json's escape path that
skips the proved-clean prefix — the scan has already shown those bytes need no
escaping, so they leave in one copy instead of one at a time through a call —
falls 471,213 instructions on oneshot, 1.70%, and the objective priced it at
+0.0039 before the compile veins were regenerated and −0.02 after. It is being
held for this benchmark rather than shipped against the old corpus.

### The mutation, and what each gate saw

The ratchet row for the new vein carries a LIBRARY defect on purpose:
`escape_clean` re-encodes the string and folds it byte by byte instead of
taking the fast path when nothing needs escaping. Applied at 643399da:

    live_counters      RED    allocs 14,819,276 -> 18,300,076, +23.5%
                              append_fast 42,334,257 -> 56,462,657
    oneshot_counters   RED    the same defect, at one encode's scale
    encode_counters    GREEN  the frozen copy cannot see it at all

The third line is the argument for this benchmark and the argument for keeping
the frozen copy, in one reading.

The first draft of that mutation deleted the fast path outright and left `s`
and `n` unused, which the language refuses — so the tree did not compile, the
gate never ran, and the row would have been credited for a build failure. It
re-encodes `s` instead. `benchmarks` setup rebuilds the compiler first, which
matters here for a reason worth writing down: `lib/*.kso` is `include_str!`'d
into the compiler, so a library edit without a `cargo build` changes nothing.
In a worktree built before the edit, `kanso build` succeeded with
`lib/json/text.kso` holding outright syntax garbage.

### The sweep, and the veins it does not read

The cost goldens are eleven now, not ten. `scripts/gates/all_counters.sh` gains
its row and CLAUDE.md its count, both pinned by
`tests/every_counter_gate_is_in_the_sweep.rs` and both watched red: dropping
the row reports `no row for ["live"]`, and leaving the count at "ten" reports
`there are 11 cost goldens and CLAUDE.md counts them "ten"`.

CLAUDE.md also gains what this thread found the hard way: **the sweep reads the
runtime cost goldens and nothing else.** `machine_code`, `emitted_code`,
`compile_memory`, `compile_allocs` and `compile_instructions` are separate
gates, two of their counters are welfare terms, and a library change moves all
of them because the library is compiled into the compiler. The escape change
above read as a welfare RISE with those veins stale and a FALL once they were
regenerated, and nothing in the sweep would have said so.

### Round one: CI's own sitting, and a third chip goes bimodal

Two rows moved on the first CI round and neither is a regression.

**work_livebench 5,312,541,181 -> 5,312,541,628** is CI's sitting replacing the
container's; every other work row matched to the digit. The 447 is the same
per-host offset the gate's header documents.

**family0x19-model0x11 is pinned as a pair now, 41,379,503 41,380,022.** CI
counted the second value on a run whose diff against main touches nothing
compiled into the compiler — this branch adds a benchmark, a gate and a log
entry, and `git diff origin/main -- src/ Cargo.toml Cargo.lock lib/` is empty —
so the binary is the one kanso#1251 landed green at 41,379,503 on that same
family and model. Same silicon, same source, the other value.

That makes THREE of the four recorded chips seen reading both numbers, and the
two values are the same two every time. A key that separates chips which all
read the same pair is a key that is not separating anything, and the collapse
argument that has been waiting on evidence now has its third reading.

### Round two: two specs that were doing exactly their job

The suite caught what the counter gates could not, and both failures are the
same shape as the ones this benchmark exists to prevent.

**`every_benchmark_in_the_work_vein_has_a_direction`** reported livebench "in
`bench/instructions_golden.txt` and in no direction table, so a rise in any of
them reads as UNCLASSIFIED drift and the trend gate exits green". Its own
header records digestbench arriving in the vein a day before it arrived in the
table, and a 6.5x regression passing green in between. livebench is now in
`lower_j` beside digestbench, scanbench and readbench. A benchmark added to
make a fall visible, whose rises read as drift, would be worse than not adding
it.

**`the_score_says_what_it_was_made_of`** pins the objective's counter set by
hand rather than deriving it, so that a row can carry exactly what the formula
reads. `live_instructions` joined the model and not that list.

Neither was found locally, because the checks run before pushing were the
counter sweep, the ratchet, the trend gate, welfare, format, clippy and the two
specs the change obviously touched — not `cargo test`. The gates all agreed;
the suite did not. On a change that adds a row to a vein and a counter to the
model, the suite is the check that matters, and it costs sixteen minutes
against the two CI rounds it would have saved.
