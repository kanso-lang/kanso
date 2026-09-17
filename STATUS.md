# Status

What I am doing, and what is waiting on Clay. Rewritten at every stopping
point; if it disagrees with the task list, the task list is right and this file
is stale — say so.

## Waiting on Clay

The decisions live in design/pending-gavels.md — the single ledger; this file
only indexes it. **Blocking right now: two** — whether the wall `>>` survives the fused
operators, and whether its simultaneous-failure merge was meant to go, both
filed 2026-09-17. The compile-term question before them was ruled the day it
was filed: two welfares and a meta-welfare over them, with the floor
re-ratcheted. The `!` question before it was ruled 2026-09-16 and
reversed the same day: the bang is the channel that bubbles, so `xs[i]!` and
every `!` name answer a box. The box-wrapping question before it was
ruled 2026-09-15. The reconstruction coverage
question was ruled on 2026-09-10, taking cloud's recommendation: rows 15..390
stay unscored on the run terms, the eight-phase half stays built, and
re-measuring the old commits is filed as a lead with a feasibility probe in
front of it. Clay ruled
the two before it on 2026-09-08,
taking the recommendation each carried: the compile term reads a fixed
corpus rather than whatever lib/json imports, and an infinite or nan float
renders as `inf`, `-inf` and `nan`. Both are cloud's to build. The welfare
history's baseline left the same day, with the seven gavels beside it. Before
them, the compile row's drift was the
last blocking entry, and Clay finished it on 2026-09-05: one row, one value.
The pinned pair and the per-chip key are retired, every move is attributed to
the change under test and handled by the ordinary ratchet, and consistency is
verified by reproduction — same build, any runner, same number. A reproduction
failure halts the vein and is hunted to its source, the way the
`/proc/self/maps` term was, rather than pinned or recorded as a mode. The
residual entry that had moved to "Open, not blocking" is retired with it.

That entry was filed carrying a second question, about a 0.008 welfare fall,
and Clay ruled that half on 2026-09-04: the objective should incorporate the
win the corpus could not see, so the corpus is what gets fixed. bench/readbench
does it and the repair scores +2.69, so the fall is gone rather than allowed.
The digest question — what a digest costs, and whether it stays written in
kanso — sat here and was bounced on 2026-08-29: a performance question with no
surface area is the implementer's, per the ledger's own charter, and the log
carries the research mandate it left with.

**Six questions are waiting** — two blocking, both filed 2026-09-17 out of
one reading of a book sample: whether the wall `>>` survives the fused
operators, and whether its simultaneous-failure merge was meant to go. The
compile-term question before them was filed and ruled on 2026-09-16: two
welfares and a meta-welfare over them, with the floor re-ratcheted. The
welfare-floor entry left
Blocking on 2026-09-14 with all three of its asks already answered: its two
builds merged on 2026-09-13, the `git add bench/welfare_floor.json` permission
it wanted is moot under Clay's ironclad rule of the same day, and its options
4 and 5 ask for a branch, which CLAUDE.md now says was never his to grant. Two
more left the ledger on
2026-09-05 in the same sitting as the compile row: machine-code size gets no
term in welfare, and `.text` stays in its own exact vein. Counted from the ledger, which is the only place that count is true;
this file said fourteen in one paragraph and sixteen in another until the
2026-08-29 sitting ruled the rest, said four with three in the ledger until
2026-09-14, and had "one blocking" in one paragraph and "two blocking" in
another on the same day, and said "one blocking" here while the paragraph above
it said zero until 2026-09-16. The July letters are closed: Clay ruled the
last five in one sitting on 2026-08-26, and that ruling reached main only on
2026-08-28, on a branch that had been sitting unmerged.

That sitting is what the 2026-08-25 sweep was for. Clay's ask was *"the goal
here would be to not have anything left to gavel. I feel like I keep gathering
things and then you ask me the same question 10 to 20 more times."* So each
entry cites the search behind it and proposes an answer, and a sitting can be a
yes or a no rather than a fresh design conversation. On 2026-08-29 every
remaining question was ruled in one pass.

**The four open, not blocking** — the box constructor's spelling, recommending
`effect`, which no build waits on; how far a binding position carries a box, filed 2026-09-16 with the ten
fixtures that refuse the blunt answer; and a byte-position scan on a string for
the JSON escape path, filed 2026-09-16 carrying a −1.2193% runbench measurement
that had been taken on 2026-09-15 and filed to nobody; and pinning `.rodata`
to a fixed page so code growth stops moving the compile rows, filed 2026-09-16
with a recommendation to decline it, the second measured decision in two days
found sitting in the log with no entry to go to. The maps parse's share
of the compile row's drift left this list ruled on 2026-09-15 and built the
same night, kanso#1439. The assert hako left it by being built — `lib/expect`,
kanso#1233, 2026-09-03.

**The July letters are closed.** C struck, `done` minted for D, G struck on the
July provenance measurement, Z confirmed declined, AA explicit-cast only. Every
letter A1–X, BB, C, D, G, Z and AA has a ruling in the log or the archive.

Six candidates the sweep turned up were **already answered by shipped code**
and went to the log rather than the ledger, so they cannot reach him again: the
lambda-parens rule, `next`'s signature, cyclic rendering and equality, `run`
versus `play`, the three small July spellings, and the write-once marker.

Three rules now guard the file. An entry **cites its search** of the log, the
archive, every design doc **and the tests**, or it is invalid — an unsearched
question does not go to Clay. The third source was added on 2026-08-27, after
two questions turned up that the 2026-08-25 sweep had missed because each was
recorded only in a spec: `module_differential`'s known-defect ledger carried
one, and an `#[ignore]` reason in `tests/entry_file.rs` carried the other,
whose log entry ends "That is a gavel." A sweep that reads only prose cannot
see a question a test is holding. And an entry **carries a recommendation**,
because a question with no proposed answer turns one sitting into ten.

## Ruled, unbuilt

Rulings Clay has made that no pull request has yet built. Cloud reads this
whole section before choosing what to build next, and chooses with
discretion; a pull request on something else says which of these it weighed.
The chat adds a row the day a ruling lands and removes it the day the build
lands on main. Verified against the tree on 2026-09-09 by probing the
compiler. The list has called itself a floor since that day because the
2026-08-29 sitting had never been audited; the sweep recorded below audits
that one sitting, and the phrase stays, because it is still true for the
rest. Swept again on 2026-09-14 against merged main: the effect-type
row came off, built and merged as kanso#1372, and the exhaustiveness row came
off with kanso#1369. The book row came off with kanso#1412, which its own
log entry calls the live remainder of that ruling. The two welfares came off
when kanso#1491 landed the second half of the 2026-09-16 gavel; every item on
its Owes list was probed against a build of main rather than read off a
report, and for about an hour on 2026-09-17 the section was empty for the
first time since it was created. THREE rows stand as of 2026-09-17 21:35Z. The
first came from the 2026-08-29 sweep: every one of the twenty rulings in that
sitting was probed against a release build of `5e256ce0`, nineteen came back
built or declined, and the twentieth is the first row below. The second is
the 2026-09-15 normalization ruling, which is ironclad and which a counter
minted a day later does not satisfy.

**And the floor is still a floor, because August is not the log.** The live
log and the archive carry **56** entries whose heading begins `gavel:`.
**Thirty-five carry an August date and all thirty-five are now swept**, on
2026-09-17: seventeen on 08-29, nine across the five oldest dates, nine across
08-24, 08-25, 08-26 and 08-31. **Two came back unbuilt**, and both are rows
below. The **twenty-one** that remain are all September; six of them were
probed on the way past and are built — the whole-float rendering of 09-06 and
the inf/nan words of 09-08, each with a golden carrying its citation, and the
09-03 suffix contracts, whose own entry recorded them as unimplemented and
which now refuse at the declaration with `error[naming]` on both halves, and
the 09-08 `page_drift` fix, built as the first of the two shapes that gavel
named and pinned by three specs, and the 09-06 consolidated run program,
whose header writes down the mix the gavel required, and the 09-03
exceptional-failure doctrine, whose anticipated outcome rides inside the box
the later rulings apply and dispatches as data once opened — and **fifteen**
are unread. Two in thirty-five is not zero, so what September holds is a guess until
somebody reads it. That is what "floor" means and why the word stays. The build hole came off built as kanso#1447, the day after
it was found twenty-three days off this
list; the compile row's normalization, ruled 2026-09-15, was built the same
night (kanso#1439); and the explicit box came off the same afternoon it was
probed part by part against a release build of main. It does not wait on
anything.

**The box row's removal, so it can be put back if anybody disagrees.** Every
item on its Owes list was checked against a build of `298636b7`: the `effect`
constructor answers a box; an `(err _)` arm matches a bare err anywhere; the
check refusal fires on an operator, an index and an arm-less call; ch04
documents the rule and the name blind spot it keeps; no `[...]!` anywhere in
lib, scripts, bench or the book samples is handed straight to an operator, and
`xs[1]! + 1` is refused with a diagnostic naming `.>`, so the 710 sites are
respelt; `read_file!` and `read_bytes!`, the only two `!` names in lib, both
end in `.>` and answer a box; and the two cost levers are on main with the log
entry "the bound discharge had no golden, and it is built" carrying the probe
and the fixture. The one residue is a SPELLING convention rather than a build:
"no bang where the bound is provable" is not enforced -- `xs[1]!` on a
three-element literal compiles -- so the tree compiling cannot prove no
needless bang survives. That is a tidy-up, not a ruling awaiting a build.

The paragraph before this one was wrong on 2026-09-17: it named the build hole
as a standing row and as one that had come off, in the same breath, and did not
name the two welfares at all. This paragraph is the first thing cloud reads
before choosing work, so a row miscounted here is a row chosen or skipped
wrongly.

### The cohort gavel's data-sized cycle (2026-08-29, narrowed 2026-09-16)

The archive entry "block-born is the whole cohort", Clay: "okay whole cohort
it is." Built on 2026-09-09 as kanso#1359 with all four shapes the gavel
names. The build-hole gavel of 2026-09-16 took two of them back: a record an
`if` chose and an element of a born list can no longer be written through,
because a hole is filled exactly once and a name whose birth is `Either`
cannot be shown to fill one. That reasoning is sound and this row does not
ask for it to be undone.

What the row asks for is the purpose the two shapes carried. The cohort
gavel's words are "cyclic structures sized by data (a graph parsed from
input, N linked nodes from a map) gain a spelling", and against a build of
main there is no such spelling left: an indexed element cannot fill a hole, a
field built with a value cannot be written at all, birth does not flow through
a call, and N nodes cannot carry N names. The alias and the field of a born
node stay built and are not part of this row.

Route, and the reason this is a build rather than a question: the 2026-09-09
entry names birth through a call as the next widening of this analysis and
claims it as the implementer's. A call that returns one record may resolve to
one birth, which would give the fill its uniqueness back. That is a thing to
measure before it is a thing to build on, and if the measurement says no, the
finding goes to the ledger as a question about what the cohort gavel's
purpose is owed instead.

Owes: a measurement of whether birth through a call resolves to one birth;
if it does, the widening and a micro golden building a cycle over a
data-sized list; if it does not, a ledger entry stating what the gavel's
purpose needs. Either way the golden's header stops claiming four shapes
while the checker admits two.

### A welfare counter reads three parts per billion (2026-09-15)

Clay's words, on the compile row's `/proc/self/maps` parse: "you want to set
up the run so that any external State like this is normalized. you clear it
out so it's identical every single run or you do something that puts it into
a persistent known initial state." Ironclad, and recorded in CLAUDE.md as
superseding the kanso#1234 argument rather than reopening it.

`interp_instructions` landed a day later in kanso#1491 and does not satisfy
it. Two CI jobs read 2,178,502,266 and 2,178,502,272, each stable across the
gate's own second reading.

**Read kanso#1492 before this row.** Its log entry "seven silicons, one
recorded block, and a reader that was never called" and `docs/compiler.html`
§77 built the instrument this row was guessing at: the gates printed a CPU
family and model and stopped, and the reader for the whole 123-row feature
block had nothing recorded to compare against. Across ninety-odd job logs
there are seven distinct blocks differing in 57 rows. On these two jobs the
block is identical, all 123 rows, so the silicon is out.

Cloud's candidate, left as cloud left it — an argument, not a measurement:
six in 2,178,502,266 is three parts per billion, and the interpreted run is
the allocation-heavy workload at 5,313,434 allocations against a compile's
27,397, so a term proportional to work fits where a constant does not, and
where the allocator's heap starts moves with the size of the file the loader
mapped.

What this row adds is a correction to itself. The other ten counters in the
same two jobs agree to the instruction, and that was written here as the nine
sharing the binary not sharing the exposure. It is not evidence of that: the
other instruction rows run from 4.8 million to 128 million, where three parts
per billion is a fraction of one instruction, so none of them could have
shown this either way.

Owes: measure cloud's candidate, or replace it. And one small thing that is
not blocked on it — `interp_instructions.sh` prints `.text`, `.data` and
`.bss`, where `compile_instructions.sh`, which the interp gate's own header
sends the reader to, prints `.rodata` too, with a seven-binary calibration in
its header for why. One awk alternation, and the next occurrence starts with
the section the compile gate already watches. Checked against kanso#1492
rather than assumed: it added twenty-two lines to that gate wiring in the
silicon comparison and left the section line reading `text|data|bss`.

If the reading cannot be made to repeat between machines, the question that
follows is whether an exact pin is the right instrument for a counter whose
artifact and host both move under it. That one is Clay's, and this row does
not decide it in advance.

### A demanded knot counts on one engine only (2026-08-24)

The archive entry "a demanded knot counts, and the oracle moves", Clay: "it
seems so obvious." The day before, 2026-08-23 had found it and sent it to the
ledger in these words: *the DEMANDED knot still disagrees. Native reports
`thunk_allocs=1` where the oracle reports `0`, because the oracle's `knotted`
builds its cell without touching the counter.* The gavel names which side
moves — the oracle — and calls it bookkeeping with no semantic change
anywhere.

Measured on a release build of main, 2026-09-17, by flipping the arm of the
mem vein's undemanded fixture so the knot is read and running it through an
importing entry on both engines:

    thunk_allocs   native 1   oracle 0
    thunk_forces   native 1   oracle 1
    thunk_evals    native 1   oracle 1
    stdout         native 1   oracle 1

Both demand it, both agree it was forced and evaluated, and one counter
disagrees, in the direction the ruling ruled against.

Nothing in the tree compares the two. `tests/golden.rs:194` runs the mem vein
with no `--interp`, so every `.mem` file is one engine's reading, and no
`*_differential` script mentions `KANSO_COUNTERS` or `thunk_allocs`. The
comment four lines above that loop says what was meant to close it, still in
the future tense: *the lazy fragment will extend these with engine-shared
semantic counters (forces, evaluations, cells live at exit) asserted on both
engines.*

Owes: the oracle's `knotted` touching the counter, per the ruling; the fixture
the 2026-08-24 entry itself named as unblocked and nobody wrote, pinning a
demanded knot's allocation shape; and a decision about the wider hole, since
one engine's `.mem` reading cannot catch a divergence by construction. The
differential law says engines agree or one refuses out loud, and a counter
nothing compares is outside it.

## In flight

This section does not list open pull requests. It did until 2026-09-15, and
the list it carried was stale by the next morning both times it was written:
cloud lands five or more a day, so a snapshot here is wrong within hours and
nobody reads STATUS.md for it. Open pull requests live on GitHub, and the
daily sweep reads them there — age, mergeable state and the check-run tally
straight from CI, never from a comment. What belongs here is a branch that is
NOT a pull request and would otherwise be invisible: a worktree holding
unpushed work, a measurement mid-run. As of 2026-09-15 there is none.

## What landed on 2026-08-29

    kanso b42699d4  #1115  the err migration count, corrected
    kanso ab158a37  #1116  the three chain words on all three engines
    kanso 2b2f7e36  #1117  §23 told the opposite of what shipped
    kanso 5d49b125  #1118  a rescue inside a group, on all three engines
    kanso afc7947b  #1119  a builtin's count, checked where a user
                           function's is
    kanso 4c40088e  #1120  a module is named the way an import writes it,
                           and the page reads the error corpus
    kanso 6bc5b168  #1121  the backend still indexed the argument the
                           front door had started counting
    kq    f0f413a   #81    the pin absorbs seventy-three kanso merges

**Where a rule lives is the through-line, and §29 now carries it.** A
builtin's argument count had four answers — the interpreter checked at
runtime, the native backend checked in its emit list, the page not at all,
and `kanso check` did not know — because the counts were a second field on a
table answering a different question. `print (wrap_err 1)` aborted the
compiler outright. This is the third instance of one shape in eight days,
after the `val` accessor and `rt_maybe_bind`'s inline copy of a predicate,
and the observation worth keeping is how the duplicates arrive: not one was
written as a copy. Each was a local detail at a site that needed it, and
became a duplicate later when somebody wrote the abstraction.

**And the opposite lesson on the same rule.** Once the front end checks the
count, the backend's own check looks like exactly that kind of second copy.
It is not: three builtins are emitted inline and read their arguments by
index before any guard runs, so with the front-end check off the abort is
still there. Knowing a fact twice is the problem; refusing to proceed
without it twice is the defence.

**A second host separates work from layout, and it costs a minute.** Same
diff, `compile_instructions`: +12 here against CI's +83,829 on #1120, then
byte-identical here against +2,138 on #1121. Every earlier layout
attribution in the log argued from the call graph. Two callgrind runs in the
container make it a number, and the same trick bisected kq's work vein to a
single kanso commit — the container cannot compare absolute numbers with the
runner, and its deltas matched to the instruction.


**The corpus reads programs that RUN, and that is a gap.** The browser
differential takes `examples`, `tests/golden/runtime` and
`tests/golden/micro`, so a program refused at compile time is read on two
engines and never on the third. Measured on 2026-08-29: all 173
`tests/golden/errors` fixtures put through the page's compile door give 141
byte-identical answers, 32 differing, 0 that run and 0 declined — and every
one of the 32 is the same cause, native's `(module foo.kso)` against the
page's `(module foo)`. `src/lib.rs:3484` prints the resolved path on native
and the import path on the page. So the gate is one decision away: which
spelling is a module's name. Task #116.

## What landed on 2026-08-27 and 2026-08-28

    kanso fa6b10d0  #1100  the scan reads eval.rs: 175 diagnostics became 242
    kanso d887d913  #1099  `if` and the guard say one sentence on all three
    kanso 0c958475  #1098  `cannot destructure` diverged on three engines
    kanso 7c173bd4  #1101  the page said "this value" about two things
    kanso d2177f3a  #1102  six val sites, not four: a constructor and an
                           interpolation too
    kanso aec580f8  #1104  the walk could not say how much it had walked
    kanso a705979b  #1105  the page refused before three sites could explain
    kanso 3a7e76c5  #1103  two refusals named each other, and a reader had
                           nowhere to go
    kanso 61af582a  #1106  the page answered `1 + d` with `d`
    kanso 9441e3f3  #1107  three more of the same family
    kanso d45c4f17  #1108  the scan reads the browser engine: 242 became 262
    kanso ad851b12  #1109  the page catches up: §27, the answer not the sentence
    kanso 5f53290d  #1110  the log holds forty entries again

**The through-line is one accessor.** `val` in `src/wasm_rt.rs` answers a value
for a value and a handle for a closure and refuses everything else in its own
words. Any site that opens with `match val(h)` and owns a sentence about what
it was handed therefore never reaches that sentence, and any site that means to
CARRY a description refuses it instead. Both halves were live, and the second
half had a case in it that was not a sentence at all: `1 + d` handed the
operand's own handle back, so the expression evaluated to `d` and the page
printed `<io>` where the other two engines refuse.

That one is the finding worth carrying. Every diagnostic gate this project has
compares refusals, and the program does not refuse; the error corpus pins what
a program writes to stderr, and it writes to stdout. The three-engine walk,
which compares what each engine PRINTS, is the only instrument that could have
seen it — and it could only see it because #1104 had just taught the walk to
account for every program in the corpus.

**The method that found the rest.** Following the failing program found four
sites. Sweeping the accessor's own call sites — reading each one and asking
what a program could hand it — found six, then a refusing family, then a
carrying one. Where the sweep was wrong it was wrong in a specific way: twice a
site was written off as unreachable on the strength of a probe that ran a
DIFFERENT program. `(opaque d).nope` is a name error and never reaches the
runtime; `(opaque d).n`, where some record declares `n`, reaches `rt_no_field`.
A site is unreachable only when a program written to reach IT fails to.

**And the scan finally reads the engine a reader meets first.** `src/wasm_rt.rs`
writes 36 `die(` sites spelling 24 sentences, on the engine the website's
playground runs, and no gate had read a word of it. Two openers took the count
from 242 to 262, and both carry a ratchet mutation watched turning the gate
red. #1105 had claimed in a comment that the scan already watched two
hand-copied sentences for drift; it did not, and could not have — the copies
are gone now anyway, dissolved by #1106's placeholder.

## Next

The directive of 2026-08-26 — merged to main only on 2026-08-28, in #1112,
having sat unmerged on the branch that carried it — says the running cloud
session makes no further claude-code-remote tool calls, because it predates
the checked-in allowlist and every one of them prompts Clay's phone. That
holds: self-scheduled check-ins are bash waits, and creating a successor
session is itself such a call, so Clay starts one when he wants one. The queue
drains and refills in the meantime; it is not a boundary the session can act
on by itself.

**What to build next is chosen from the "Ruled, unbuilt" section above,
never from this one.** What follows is the 2026-08-29 snapshot of the err
spelling's landing and is kept as history of that build; the rulings that
superseded it the same afternoon and on 2026-08-31 are rows above. The
original opener read "the biggest buildable thing is now the err spelling,
and it is ruled", gaveled 2026-08-26 and on main since #1112:

    io/read_file path
    bind     (text -> json/parse text)
    annotate (e -> "config: {e.reason}")
    rescue   when_failed

Three words, all ordinary two-argument functions — `bind effect callback` —
threaded by the chain rule already in the language. The callback receives the
err itself, so a dispatch group is a legal callback. Annotate cannot resurrect
by construction. Rescue is the sole door, its foreign-only license checked at
the word. `.` retires from chain-step position; field access is untouched.

**All three words work on all three engines** as of 2026-08-29 (kanso#1116).
`tests/golden/chainwords/` holds four cases, pinned on both engines against
one golden, and three mutations were watched turning the right case red.

`annotate` needed no new description field: the runtime builds its wrapper
closure itself, over the callback and the site, and hands it to rescue's node.
`k_closure` was already there for that shape.

**The page was diverging in silence and nothing in the tree could see it.**
It compiled `rescue` and then propagated the failure the other two engines
catch, because the wasm backend has no node for the words and the names fell
through its generic call path rather than its `unsupported call` arm. Every
chainwords case needs a filesystem to make an effect fail, and a page has
none, so no program the page could run reached the words.
`tests/golden/micro/a_settled_failure_meets_the_three_words.kso` closes that:
it reaches them through a subject that has already failed, with no filesystem,
so all three engines run it. It went red on the wasm harness on its first run,
and the page was built rather than left refusing. The general lesson is worth
carrying: the engine with no world is the one that most needs a case, and a
fixture that fails an effect cannot be that case.

One narrow hole is written down rather than assumed absent: the page's
`as_desc` answers None for a rescue or annotate slot, so a worded step other
than `bind` inside a `join` is refused there rather than scheduled. Nothing in
the corpus reaches it.

`bind`, `rescue` and `annotate` are reserved names now. Two fixtures in the
corpus had defined functions by two of those names and were renamed, which is
a fair measure of how ordinary the words are.

**`rescue` was new capability, not a respelling** — the finding that reshapes
the rest. An execution-time failure could not be handled inside a chain at
all: a chain step over an effect wraps its expression in a one-parameter
closure, and handing a failure to a closure returns the failure rather than
entering the body, so the callback was never called whatever its shape.
`docs/book/samples/ch05/fallback.kso` is that program and ch05 teaches it as
the design. So the migration's err-arm half is empty for a stronger reason
than the greps below gave: the chain err-arm was never a working surface, and
the one site that looks like one is the book's counter-example.

**What is left is one pass over the lambda steps.** 485 loose-dot steps in
fleet code with comments and strings stripped: 346 are `. (lambda)`, the
monadic steps that respell as `bind`, and 139 are `. named_fn`, the threading
form the effect-first rider preserves. (An earlier count of 309 counted
differently and is superseded.) Most of the 346 are in `scripts`, which is the
gates, so a botched pass there takes CI down rather than a user program.

**It cannot start until Clay rules on the line grammar.** The gavel's sample
is dotless — `bind (text -> ...)` — while the same paragraph says the first
argument comes from the rule that makes `(expect 1) . to (equal x)` work, and
that rule is the dot. Dropping the dot collides with the rule that an indented
line under an argument-taking statement is one more argument (`src/lexer.rs`),
which the leading `.` is currently the only thing distinguishing — so the
dotless form needs the parser to know the three words, contradicting "nothing
about them is syntax". Filed.

**Two things the gavel's own sample needs and does not have.** Its lambda form
reads `e.reason`, and an err has no such reader — nor could a lambda use one,
since every operation on an err propagates it. A group callback destructures
instead, which is the gavel's primary story. Whether an err gains readers that
get past its own infectiousness, the way `wrap_err`'s second argument does, is
open.

**`done` is the second, smaller one.** Letter D was ruled in the same sitting:
a succeeded effect yields `done`, not `none`. `none` means absence and nothing
else. Chains that tested for `none` after an effect migrate.

**What needs Clay, and cannot start without him:**

  - the one blocking question, what a digest costs, filed with its measurement
  - `delete_branch_on_merge=true` and the 324-branch purge (task #109), both
    checked from here on two days and both refused by the tooling
  - whether an err gains readers a lambda callback can use

**Six questions wait in `design/pending-gavels.md`** — two blocking, four
open — each with a recommendation. Recounted four times on 2026-09-16: the
binding-position question joined Open with its measurement, the escape
path's byte-position scan joined it, measured on 2026-09-15 and filed to
nobody until the sweep that found this paragraph's own overview stale, the
`.rodata` page pin joined it the same day — a second send found unfiled, which
is what built the check that now reads the log for them — and what the compile
term counts once codegen is in it joined Blocking, filed by the chat against
the 2026-08-25 gavel the term had never carried.
Recounted on 2026-09-15 before that, when the
box-wrapping entry left Blocking ruled, the `!` question it left behind
joined Blocking, the constructor's spelling joined Open with a
recommendation the build does not wait on, and the maps-parse entry left
Open ruled the other way from its recommendation: the parse is external
state, and the compile row is normalized so it is not counted. Counted
from the ledger by reading its `###` headings, which is what this line has
always claimed to do and had stopped doing TWICE. On 2026-09-05 it said nineteen while the ledger
held three; fifteen had been gaveled and had left the file as the lifecycle
requires, and the count here was carried forward instead of recounted. It was
recounted that day and went stale again within three days: it said six, none
blocking, while the ledger held three under a heading named Blocking, and
four of the six it named had been ruled or shipped since. A count maintained
by hand goes stale by default, so `tests/the_status_index_counts_the_ledger.rs`
now reads both files and fails when this sentence and the ledger disagree.

The two blocking are the wall's two questions, filed 2026-09-17. The four
open are the box constructor's spelling,
recommended `effect`, which cloud builds against unless Clay names another;
how far a binding
position carries a box; a byte-position scan on a string for the escape path;
and pinning `.rodata` to a fixed page so code growth stops moving the compile
rows. The last three were filed on 2026-09-16 and this sentence named two of
five until it was recounted the same day.

This sentence named three blocking entries that Clay ruled on 2026-09-08 and
went on naming them for six days. The count above it is pinned by a spec and
the names beside it are not, so the names are what rots; a reader who trusted
them would have brought him three settled questions and missed the live one.

Rulings since the last recount: no machine-code-size term in welfare and no
two-value chip row (both 2026-09-05); the granted-baseline question CLOSED AS
MOOT by the gavel of 2026-09-06, which put the run side on one consolidated
program; and the assert hako, which was never a decision in the end — it was
built on 2026-09-03 as `lib/expect` (kanso#1233) while the ledger went on
recommending that somebody build it.

**The rules that carry forward**, each earned twice:

  - a scan over a corpus enumerates the corpus's file types from the harness
    that reads it, never from the ones that came to mind
  - a claim that a site cannot be reached is only as good as the program
    written to reach THAT site — `(opaque d).nope` is a name error and proves
    nothing about `(opaque d).n`
  - a branch with a ruling on it is not a record until it merges. Four gavels
    sat unread for up to thirty-two hours, and this session reported the tree
    settled while they did.
