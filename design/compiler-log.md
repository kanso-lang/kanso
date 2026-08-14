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

## what a constant that names itself is worth

Three engines held a self-naming constant three ways, and one of them made up
an answer. `ring = cell ring` printed `probe/cell 0` on native where the oracle
refused: the mention loads the constant's global while `k_caf_init` is still
building it, and a zeroed `KValue` is the integer zero. Nothing said so. A
program could read that field and compute with it.

Two changes, both small.

`k_render` had no case for a cell, so an unforced one fell through the switch
to `<value>` where the oracle says `<thunk>`. It now unwraps a forced cell and
names a pending one, which is what the oracle does — rendering demands nothing
on either engine.

And `k_caf_init` seeds every cell with a blackhole before it runs any builder.
The mention then loads a cell nobody has demanded rather than a zero, so the
fabricated integer is gone. What replaces it is `probe/cell <thunk>`, which is
still not the oracle's `error[runtime]: a lazy binding demands its own value` —
but it is honest about having no value, and demanding it refuses rather than
answering.

    jsonbench calls     2346 -> 2350
    jsonbench lines    13329 -> 13338
    defines, branches  unchanged
    welfare            75.69, at the floor

Four calls and four stores, once, before main — the decoder has four constant
cells. Nothing in a decode loop moved.

Two gaps stay open and are written down rather than left to be rediscovered.

The demand path is the larger one. The oracle forces where a value's identity
is needed; native forces only where the callee pattern-matches the argument,
so `x + 1` on a pending cell answers "`+` is not defined for these values"
instead of demanding it. Forcing both operands in `emit_binop` was built and
measured: it did not fix the case — a parameter's static set carries no thunk
bit, so the force is a no-op exactly where it is needed — and it changed the
decoder's emitted code, where it is not. That is a demand-analysis question,
not an emitter one, and it was declined here on the measurement.

The browser holds a deferral as the closure that would answer it, so it says
`<fn>` where the other two say `<thunk>`. Both are honest about having no value
yet and they do not use the same word, which is a gap recorded in
`tests/golden/wasm_gaps.txt` rather than a divergence hidden. Closing it means
teaching the renderer to ask a deferral what it is, and the renderer is the
interpreter's own.

## a builder can count itself now

`regexp/matches` over a freshly stitched chapter cost 45.4 seconds where the
same bytes read from a file cost 2.3, every match paid again, and crossing a
beat made it fast. Several ticks had gone at that with the wrong theories —
laziness, memoisation, evacuation, the matcher's shape — and every one of them
was wrong for the same reason: they were allocation theories, and every
allocation counter is byte-identical between the fast and slow positions.
9,205,650 allocs, 327,097 evacuations, 281,268 thunk evaluations, 1,056 beat
iterations, both ways. Twenty times the wall clock for identical allocation
work is a walk, not an allocation.

Two memos, and a builder could use neither.

`k_str_chars` caches a character count in `cap` as a negative number, and only
`if (cap == 0)`. A builder's `cap` is the room it may grow into, so the memo
was never written and every read walked the whole string. The seek cursor in
`k_b_slice` — one remembered position, which is what makes a forward sweep over
prose linear — is gated the same way, `s == k_seek_str && s->cap < 0`, and its
comment said why: a builder may grow under the cursor.

    position                    scans        bytes walked   wall
    before the io/write     2,433,691   136,831,850,690    45.4 s
    after the io/write              0                 0     2.3 s

136 GB of character counting for a book of a few hundred KB.

The count now lives in the eight bytes before a builder's data, where
`k_buf_of` keeps a list's header for the same reason, and it is kept as bytes
arrive rather than recomputed: each part counts its own characters once and
caches that itself, so the sum is free and nothing ever walks the accumulator.
The cursor's objection turns out not to hold — a builder only ever appends, and
the cursor is an offset into what is already there, which appending does not
move, not even across a grow, because the header is the same object and the
offset is a number rather than a pointer.

    45.4 s  ->  12.7 s   the count kept
    12.7 s  ->   2.30 s  the cursor opened to builders

which is the baseline and the file-read control, both 2.3. The report the
script prints is byte-identical.

A DESIGN BUILT AND MEASURED INSUFFICIENT, first: a sticky "every character is
one byte" bit in the top of `cap`, which answers what slice asks without
needing anywhere to put a count. It was hit zero times. The stitched chapter
has 158 multibyte characters in 70,773 bytes, so the condition never held —
the em-dashes the slice comment already warns about, in a case built to serve
the ascii path. A cheaper proxy for the count could not stand in for it.

`str_scans` and `str_scan_bytes` are new, and they are the point. This cost is
invisible to every allocation counter because walking allocates nothing, which
is exactly how it survived: the timing looks like layout noise and nothing else
moves. `tests/golden/mem/builder_counts_once.kso` pins them flat — 2 scans and
15 bytes for four hundred reads of a 7,200 byte string. With the memo read
deleted the same program answers 402 and 2,400,015, so the ratchet has been
seen red for its own reason.

Every vein moved by exactly two lines and nothing else: 41 `.mem` goldens and
all four cost goldens gained the two counters with every other number
byte-identical, and `bench/emitted_golden.txt` did not move at all — the
counter is a runtime one and the decoder emits the same code. Welfare 75.69,
at the floor. kq keeps veins of its own and will want the same two lines when
its pin moves.

## a subtype matches where its parent does

Clay ruled it: subtypes work as in Go, with one difference — kanso dispatches
on the types of all call arguments rather than a receiver, so Go's "the
declared type inherits no methods" does not transfer, because kanso has no
method sets. A function matches on any subtype of the type it takes, and the
specificity rule already settled breaks the tie. That is the whole rule.

Three rows disagreed with it. A constructor pattern refused a subtype; a field
read refused one, which is the same mechanism because a getter is synthesised
as a constructor pattern; and an arm written for `type money int` was dead
where the same program written with a record ran it — `money 350 * 2` answered
700, silently, on every engine. Nothing here broke a law: all three engines
agreed on the wrong answer.

The rule was already implemented, once, for annotations. `type_match_depth`
walks a subtype chain and adds a step per level, so `_:money` matching a
`sale_price` ranks below an exact `_:sale_price`. Constructor patterns never
learned it, and neither did the two backends' arm ordering, which scored every
constructor pattern a flat 2000.

So the change is the same sentence in six places: match at the level the
pattern NAMES rather than peeling to the base first, count the wrappers removed
getting there, and rank by that count. `Pattern::Ctor` in the interpreter,
`k_check_rec` and `k_field` in the runtime, `rt_check_rec` and the two field
readers in the browser, and `2000 + depth_of(ty)` in both backends' scorers.

A first attempt peeled every wrapper before comparing, which made a pattern
naming the child unmatchable — it compared `sale_price` against the base
`money` and found nothing. The precedence probe caught it. The lesson is that
peeling and naming are different questions: a field read peels, a pattern
names.

    fn scale (money c)  on a sale_price     700, was "no overload matches"
    sale.cents through a subtype            350, was "not a record"
    an arm on `type money int`              ran, was silently skipped
    child arm against parent arm            child, was parent
    two levels, nearest ancestor            middle, was parent

Four micro goldens, each watched red on main for its own reason. Welfare holds
at 75.69 and the whole suite is green, which is the interesting part: a rule
this central changed and no existing golden moved. The corpus had no program
that asked a constructor pattern about a subtype.

This unblocks the division-by-zero design. Both halves of "handle as
generically as they like" hold now: an arm on `_:math_failure` catches a
`divide_by_zero`, and reading the reason off it works, so the root type may be
`type math_failure string` with live operator arms.

## the chart has to draw

Nothing asserted that the compiler page's chart renders. Not the site smoke
harness, not the browser differential, not any test in tests/. The series could
have drawn empty, or not at all, and every check stayed green.

The gap is older than it looked. I first recorded it as coverage that #791 and
#793 removed when they deleted the drawing scripts, and that is wrong — the
comment #793 deleted says so directly: "nothing has ever run scripts/long_view,
so the published chart sat frozen at whenever somebody last ran it by hand."
The script was manual, CI never ran it, and its failing never made anything
red. Those two changes improved on what preceded them; the hole predates all
of it.

`site_smoke.py` already drives headless Chrome over three pages, so it is the
place. It now renders `compiler.html` too, and asserts the chart drew five
series of exactly six points.

The number comes from a stub. The chart reads its rows from the history branch
over the network, and a test that let it do so would depend on a branch's
contents and on the network being there, and could pin nothing. So `render`
gained a prelude — a script in the head, before the page's own — which stands
in for `fetch` when the url names history.jsonl and hands back six rows. The
prelude passes every other request through, which the probe needs, because it
reports its findings by posting one.

Asserting the count rather than the presence is the whole point. An element
exists on a page whose chart drew nothing, and a presence check passes there,
which is the failure this is for.

Seen red before it was pinned green: with `drawTrend` returning early, the
harness says `the chart drew []; five series of 6 points were fed to it`.

The first stub was invented rather than shaped, and CI caught it where this
machine did not: `rejected: RangeError: Invalid time value`. The rows carried
the counters the chart reads and no `date`, which the page formats, so it threw
before drawing — a fixture the pipeline would never produce, asserting a
fiction, which is the trap the corpus rules already name. The rows are now
shaped from a real row of the history branch.

Two things about that are worth keeping. The probe reported a blank reason at
first, because an unhandled rejection does not fire the `error` listener; it
listens for both now. And the failure was invisible here and loud there, which
is the ordinary shape of a browser difference — the local Chrome tolerated the
invalid date the CI one refused.

Then it failed again, differently, and the second cause is the one worth
remembering. The chart takes its host out of the DOM at draw time, and the
element sits after the script that draws it. A stub answering in a microtask
beats the parser to that element, `drawTrend` finds no host and returns
silently — no error, nothing drawn. A real fetch is never that fast, so the
page has always worked in a browser and only ever failed against a stand-in
quicker than the network. The stub now resolves on `load`, which puts it back
behind parsing where the thing it stands in for lives.

The rule underneath: a stub is a stand-in for timing as much as for content.
One that answers sooner than the real thing tests a program nobody runs.

## a deferred constant says it is one

Native reported `+` is not defined for these values where the oracle reported
a lazy binding demands its own value. Both refuse, so nothing answered where
another refused — but only one of those sentences is true, and the false one
blames an operator for not knowing a value it was never given.

The obvious fix was built and declined on 2026-08-08: forcing both operands in
`emit_binop` neither fixed the case nor came free. `maybe_force` bails when the
value's static set has no thunk bit, so the force was a no-op exactly where it
was needed, and it fired elsewhere and moved the decoder's emitted code.

The bit is lost at the source. `Expr::Field` answers TOP, which carries the
thunk bit — but that arm never runs, because a field read is an application of
a getter since accessors became functions, and the getter's return comes from a
constructor pattern over the record's fields. Those fields carry whatever the
constructor's arguments carried, and a constant naming itself carried a set
that said the value could not be a thunk. Every consumer downstream inherited
that: the getter, the parameter, the operator.

So a mention of a constant inside its own body now carries the thunk bit. There
is nothing else it could hand over at that moment — the value does not exist
yet — and saying so lets every read downstream decide for itself.

    the decoder's emitted IR     byte-identical
    welfare                      75.69, at the floor
    suite                        55 of 55

The IR being byte-identical is the measurement that says the widening is
confined: the decoder has no self-naming constant, so nothing about it moved.
`defers_into_containers` already made the same trade for a list, map or string
read, and this is the same phenomenon in a record.

The corpus had never asked the browser this question, and it answers a third
way: it recurses until the stack ends. Its guard marks the deferral's handle
before calling it, but re-evaluating the constant makes a fresh handle, so the
mark is never the one arrived at. The interpreter's knot is keyed by name and
installs a blackhole before evaluating, which is why it answers correctly. The
gap is written into the wasm ledger rather than left to be rediscovered.

## a record field waits too

A constant naming itself ran the browser out of stack where native and the
oracle refuse. Three readings of it were wrong before the code said what was
true, and the wrong ones are worth the space because each looked convincing.

The first read `forced()` and found its guard keyed to the deferral's handle,
while `rt_defer` mints a fresh handle every call — so the mark could never be
the one arrived at. That reasoning holds and is not the cause: keying the mark
to the constant instead, the way the interpreter keys its knot by name, left
the program running out of stack exactly as before. Reading a function is not
evidence about what runs.

What was true is one line up. `emit_element` defers a self-mention so the cell
it would read can be filled later, and its own comment said "a list element" —
because that is the only place it was called. A record field went through
`emit_expr` and was emitted where it stood, so the mention called the constant
again and the recursion had no floor. `ring = [ring]` worked and
`ring = cell ring` did not, which is the whole difference.

A field now waits the way an element does, and the stack is no longer the
answer. What the browser says instead is that `+` is not defined for these
values, and that is not a bug in the operator: `ring.v` IS a deferred `ring`,
so reading it through yields the record, and the browser is answering about the
record it found. Coherent cyclic semantics — and not the oracle's, which
installs a blackhole and treats re-entry as the error.

So the gap stays, restated. It moved from a stack overflow to a semantic
difference, which is smaller and honest, and closing it means giving the
browser a blackhole keyed by name rather than another patch at the read.

## 2026-08-09 — the python went, and four defects came out with it

The two harnesses that drove headless chrome are kanso now, and nothing in the
repo is python. `scripts/site_smoke` makes four visits — the landing sample,
the playground, a book chapter, the chart — and
`scripts/browser_differential_run` compiles all 287 corpus programs in the tab
and holds each against the native engine: 279 agree, 8 known gaps, 0 disagree,
which is what the python said.

Porting them found four defects nothing in the suite could reach.

Two were the library's. `serve_until` folded many requests and carried state
between them, then answered whatever closing the listener answered — a harness
had nothing to decide pass or fail from, and the handler cannot print instead,
because a print and a turn as adjacent statements are a parallel group. And a
connection that says nothing killed the server: `parsed` demanded a request
line from one read and refused, which is exactly what a browser's speculative
preconnection is. curl never opens one, which is why two socket tests had never
seen it.

Two were the runtime's, and both are the reason to port real workloads rather
than write more synthetic tests.

`k_repaired_settle` raises the beat mark over bytes a repair copied into the
arena. It raised three of the mark's four fields and left `left` holding the
count from when the mark was taken, so the mark described a position that did
not exist; a later rewind restored that pair and the arena handed out memory
past the end of a block. glibc caught it on linux as a sysmalloc assertion.
macOS never noticed, including under Guard Malloc in strict mode, because the
damage lands inside a block kanso itself owns. The invariant that found it —
`k_arena + k_arena_left` meets the current block's end — is now checked at every
rewind, which costs one comparison and says so at the moment the mark goes
wrong.

And a program could only ever open sixty-three sockets and processes. The guard
compared how many handles had EVER been taken against the size of the table
rather than asking whether a slot was free, and one counter served both doors.
A server spends two a turn: the connection it accepts, and the `run` in the
statement beside it, which forks rather than blocking because it has to yield.
Twenty-eight requests answered and thirty-four did not.

Three hypotheses died by measurement on the way to the first of those, and one
false verification nearly shipped: two container runs compared as fixed against
unfixed both passed, because the real difference between them was whether
chromium had started at all. The verification that holds compares fixed against
unfixed under the invariant assertion, same run, browser working.

### The decode slowdown has a third of a name

The 7.6% between 07-27 and 08-07 reproduces at one build a side: a worktree at
5896d03c with a 2500-rep jsonbench runs 1.87 s against today's 2.02 s, eight
per cent. Profiling both — four interleaved pairs, self time, totals within one
per cent — puts `k_b_put_mut` and `k_b_push_mut` together at about 9.3% of
samples today against 6.0% then. Three points of the eight.

The old side drifts 4.3 to 8.2 across its four runs, which is most of the
spread and says a one-second sample is noisy. The first pair alone read as
+6.7 points and would have been a fourth retracted explanation. What is claimed
is the direction and roughly a third of the gap, not the cause.

## 2026-08-09 — two rules that were narrower than they read

### A tie only matters at the top

The dispatch check compared arms two at a time. Two arms that each win
somewhere were refused, and the third arm that settles them — the one at least
as specific as both, the very arm the diagnostic tells the author to write —
was invisible to it, because no pairwise comparison ever looks at a third.

    fn + a:math_failure _:math_failure
    fn + a:math_failure _
    fn + _ b:math_failure

Clay's rule, on being shown that the division-by-zero design could not be
written: it does not matter if some arms tie as long as there is a more
specific one that does not; the problem is only a tied maximum. So the check
gains a third-arm search, and a genuine tie — the same program with the
covering arm removed — is still refused. The change only ever accepts programs
the old rule rejected.

What it unlocks is arithmetic that carries a failure in from either operand,
and with it the whole of the contended division-by-zero design runs today on a
`string` root: propagation from either side, through a chain, untouched
arithmetic unaffected, and generic-or-specific handling by subtype dispatch.
Two blockers recorded against that design turn out to have been closed weeks
earlier by the subtype-matching fix, and were only still standing because
nobody re-measured them.

### A construction is a value nothing reads

Re-measuring the last open question of that design — whether a math failure
built and dropped is silent — turned up a hole rather than an answer. The
unused-value check keys its return sets on the program's functions, so a
constructor was never in the table and the lookup missed. `box 1` on a
non-final line died at run time with "`&` joins two descriptions", an operator
that appears nowhere in the author's program, where the neighbouring call is
refused at compile time with a message that says what is wrong.

Narrowed twice before the shape held. `over 10 n` where `over` divides is
deliberately allowed, because division can fail and a line that can fail is
still doing something — multi-arm had nothing to do with it. And a final
statement that is a plain value is silent for every shape, which is a program
that computes an answer and does nothing with it; it is loud only when effects
precede it. So the general rule was right where Clay said it was, and the one
exception was a bug.

## 2026-08-09 — the decode slowdown is cost per call, and two fixes for it failed

Counters on `k_b_put_mut` and `k_b_push_mut` entries, both trees, the same
benchmark: put 1,254,150 and push 1,459,800 on each side, with allocations
already known identical. So the profile's "the in-place writes carry a third
of it" is not more calls. It is the same calls costing more.

Five randomised layouts a side, one sitting, medians:

    old 5896d03c              1.979 1.988 2.009 1.993 1.994   mean 1.9926
    new, as it stands         2.156 2.173 2.188 2.184 2.181   mean 2.1764
    new, three guards removed 2.128 2.111 2.105 2.109 2.100   mean 2.1106

The gap is 0.1838 s and removing three guards recovers 0.0658 of it — 36%,
lower than control in every layout. Each alone is also lower in every layout:
`k_check_map_key` on every put (0.0404), `k_map_replace` on every put (0.0354),
`k_born_this_beat` on the push fast path (0.0316). They overlap, so the
combined figure is the one that means anything. This is the first causal
number on this question; everything before it was attribution from samples.

### Two fixes, both declined

Hoisting each guard's fast path to the call site so the common case is a
compare with no call: 2.1748 against 2.1764. Nothing. Both `k_map_replace` and
`k_map_view_insert` already return immediately when the map has no sorted view,
which decode never builds, so what they execute is a load and a predicted
branch — and removing them still helps while making them cheaper to reach does
not.

Splitting `k_b_put_mut` so the append onto a view-less unique map is a small
inlinable body and everything else sits behind one `noinline` call: 2.1408
against 2.1536 in the same sitting. Half a per cent, and higher than control in
one of the five layouts. Not enough to justify restructuring a hot path.

Discharging the key check statically, the way provable overflow checks were
discharged — a second entry point the emitter picks when the key's inferred set
is within int, string and failure — never fires. The sets at the seven put
sites in the decoder read 0b11111111111111 at four of them and only slightly
narrower at the rest. The emitter does not know a decoded key is a string, so
there is nothing to discharge. Narrowing operand sets at those sites is the
prerequisite, and it is its own piece of work.

What is left standing is that the cost is the work being present rather than
the cost of reaching it, and that __TEXT has grown 114,688 against 98,304 while
runtime.c grew a third. Neither of those is a fix.

## 2026-08-09 — a fifth of the emitted code, and what it did not explain

A program that decodes json and prints a number was carrying `text/trim`'s
whole chain, most of `io`, and the wrappers that only those named. The emitter
now drops any define no remaining line mentions, iterated to a fixpoint, which
is the rule it already applied to declares. jsonbench falls from 250 defines
and 13,338 lines to 154 and 10,529; the module compile sample from 6,349 to
4,430. Rounds and visits do not move — deciding costs the same and only
writing costs less.

Three constraints, each of which failed loudly first. The fixpoint is
load-bearing, because a dead caller still writes a call to its dead callee and
one sweep leaves a definition named by something that is itself about to go. A
library keeps everything, because its callers live outside the module the
emitter can see and there is nowhere for the walk to start. And the fnref
statics sit between definitions rather than inside them, so a splitter that
runs a block to the next `define` swallows them and three examples stop
linking.

### It says nothing about the decode slowdown, and that is the point

The obvious hope was that this tests the last standing hypothesis for the 7.6%
— that the runtime's 33% growth costs cache. It does not, because the binary
never contained this code: `-flto` internalises and strips it already, and
__TEXT is 114,688 bytes before and after. A fifth less IR through the compiler,
byte-identical machine code out.

Five randomised layouts read 2.129 against the day's earlier 2.154, about one
per cent, in the wrong direction to be trusted: the two sittings are hours
apart, and one of the five layouts is slower rather than faster. With the
machine code identical there is nothing for a real difference to come from, so
that is drift.

### So the code-size hypothesis was tested properly, and it is REAL

Changing what LLVM keeps is what tests it, and the first thing that test needed
was an honest measurement of the growth. `__TEXT` is page-granular — 98,304 and
114,688 are exactly six and seven sixteen-kilobyte pages — so the "16.7%" this
log quoted on the same day says almost nothing. The machine code itself is the
`__text` section, and it went 54,708 to 79,192 bytes. Forty-five per cent, not
seventeen.

Padding the old tree with 383 never-called `used, noinline` functions brings
its `__text` to 79,184, within eight bytes of the new side, with every other
thing about it unchanged. Five randomised layouts each, one sitting:

    old, as it was   1.930 1.959 1.959 1.961 1.971   mean 1.9560
    old, padded      1.988 1.993 1.994 1.983 1.978   mean 1.9872

Slower in EVERY layout, by 1.6%. Against a 0.1838 s gap that is 17% of it, from
code the decoder never executes — it only sits between the code it does.

Two causes now account for about half the slowdown: the three per-call guards
at 36%, and code size at 17%. They are not cleanly separable, since removing
the guards also removes code, so treat the sum as an upper bound rather than
53% exactly. The filler is uniform where the real growth is not, so 1.6% is the
magnitude and not a precise attribution.

### The welfare index cannot see this

75.69 before and after, and the floor stays there. The index was ratcheted to
76.29 mid-change and put back: that reading came from the prune still applying
to libraries, which the entry guard then correctly stopped. Its emitted-lines
term samples five single-file libraries, and a library is exactly what this
leaves alone. The gain is real on jsonbench and on the module sample and the
model is blind to it — which is an argument for the sample including an
entry-bearing program, made here and not acted on, because changing what
welfare measures is a change to the objective.

## 2026-08-09 — what the runtime grew, and one hint that made it worse

The forty-five per cent is mostly new functions rather than old ones swelling.
Compiling both runtimes to objects and differencing per-symbol sizes: 293
symbols exist only in the new one, 22,123 bytes of the 24,484. The largest
individual moves are `k_repair_interior` (2,608, new), `k_exec` (+2,436),
`k_step` (+2,328), `k_interior_survives` (1,760, new), `k_render` (+1,460),
`k_copy_size` (+1,452) and `k_b_push_into_proven` (1,344, new).

Almost none of that runs during a decode. Sampling jsonbench for two seconds
finds twenty-three runtime functions on CPU, out of 749 symbols in the object:
the builders, the buffer and string allocators, the comparison and equality
helpers, the beat push and cohort pop, and the map view insert. Everything else
is in the same text region without being reached.

### Marking the hot set `hot` is a loss

The obvious hint, and it goes the wrong way. Twenty-three `__attribute__((hot))`
markers, five randomised layouts a side, one sitting:

    unmarked   2.069 2.090 2.120 2.140 2.141   mean 2.1120
    marked     2.151 2.162 2.171 2.182 2.171   mean 2.1674

Slower in every layout, by 2.6%, with `__text` identical at 79,192 bytes either
way. The attribute did not move anything into its own section on Mach-O; what
it changed was how willing the inliner was, and the same bytes arranged
differently cost more. Declined and reverted.

That the size is unchanged while the time is not is the useful part: it says
the cost being chased here is arrangement, not volume, and a change that only
reorders is enough to move it in either direction.

## 2026-08-09 — the whole slowdown fits inside what a profile can arrange

Profile-guided optimisation, as a ceiling measurement rather than a proposal:
build with `-fprofile-generate`, decode with it, merge, rebuild with
`-fprofile-use`. Five randomised layouts a side, one sitting:

    today, plain     2.071 2.099 2.123 2.142 2.156   mean 2.1182
    today, with PGO  1.917 1.924 1.934 1.954 1.957   mean 1.9372
    07-27, plain     1.979 1.988 2.009 1.993 1.994   mean 1.9926

Faster in every layout, by 8.5% — and 2.8% faster than the build the 7.6%
regression is measured against. The machine code also shrinks, 79,192 to
75,852 bytes.

THE CAVEAT IS THE WHOLE CAVEAT: the profile was taken from the benchmark it
was then measured on. That is overfitting by construction, and 8.5% is a
ceiling rather than anything shippable. A real profile would come from the
corpus and buy less.

What it establishes is the shape rather than the size. Eleven days of diffuse
slowdown, none of it visible in any deterministic counter, all of it inside
what arrangement and profile-driven inlining can undo — and undo past the
starting point. So this is not accumulated algorithmic cost that has to be
found and removed one commit at a time. It is a compiler making worse guesses
as the code it is guessing about grows, which is consistent with everything
else measured today: the guards that cost when present but not when reached,
the never-executed padding that costs 1.6%, and `hot` markers that cost 2.6%
without changing a byte of size.

A decision follows and it is not the compiler's to make. Building kanso's
releases with PGO would need a profile in the repository, a job to regenerate
it, and an answer for what happens on a host that has no profile — and it
changes what the published numbers mean, because the number would then depend
on a workload chosen in advance. That belongs to Clay.

## 2026-08-09 — what a counter cannot see, and two gates for it

Clay, on being shown the decode slowdown: the whole point of welfare and the
counters feeding it is to prevent exactly this. His guess was that the
benchmark lacked the realism of a real json workload. It did not. The workload
that regressed IS the benchmark — decode_allocs, decode_peak_bytes and
decode_arena_blocks are welfare terms — and every one of them was byte
identical across the eleven days: 7,577,414 allocations before and after, the
same arena blocks, the same peak. So were the two kernels' call counts,
1,254,150 puts and 1,459,800 pushes. A broader benchmark would have regressed
just as silently.

The reason is written in this repository's own rules and was known when the
model was built: wall time is absent because it cannot be made deterministic,
and what a model leaves out it implicitly weights at zero. The regression lived
in per-call guard work and in code size. Nothing counted either.

### Two gates, and what each is worth

The machine code now has a golden. `__TEXT` is page-granular — 98,304 and
114,688 are six and seven sixteen-kilobyte pages — so the "16.7% growth" this
log quoted on the same day was noise dressed as a figure. Section `__text` went
54,708 to 79,192 bytes, forty-five per cent, and padding an old build to that
size with functions it never calls costs 1.6% of decode by itself.

The two in-place writers now have presence counters, by path rather than by
call. A doubling of either was invisible: the fast path allocates nothing, so
`allocs` could not see it. The counter earned itself immediately — the ch10
sample reads push_mut_slow=1000, meaning a beat-carried accumulator takes the
guarded fast path zero times in a thousand, because a list that crossed a beat
boundary is not born this beat. That guard is one of the three costing 36% of
the regression and nothing had ever shown it never fires.

Neither gate is the answer Clay wants. Both catch causes somebody enumerated.

### A time term was proposed, tested, and is weaker than it sounds

kq races itself against jq interleaved because a ratio holds where an absolute
wobbles, so the same shape was tried here against serde_json. Three sittings:

    kanso  0.8452 0.8480 0.8512 ms/decode   spread 0.71%
    serde  0.8494 0.8594 0.8616             spread 1.4%
    ratio  0.9951 0.9867 0.9880             spread 0.85%

The ratio is WORSE than kanso's own number, because serde's harness varies more
than kanso's and dividing by it adds that variance rather than cancelling it.
The trick works for kq, where contention against jq is the dominant and common
noise; it does not transfer.

What remains is that kanso's own per-decode time is stable to about 0.7% on a
quiet box, against a regression that arrived in one to three per cent steps. A
gate at that resolution catches the larger steps and passes the rest. Worth
having, not worth calling a guarantee.

The same day, kq's published table was found to be regenerated on a shared
runner: two consecutive runs of identical code moved every row by nine to
twenty-two per cent and reversed the largest, publishing an 18% regression
where a head-to-head on one box measured the new compiler 4.2% faster.
Interleaving cancels noise within a run; the machine itself is the variable and
one run is one sample of it. Ruled: the race goes, the counters stay, the
timings go back to being a dated hand sitting.

## 2026-08-10 — the work a program does, and the instrument that was already named

Clay, on being told the metrics could not see a 7.6% decode regression: he had
said repeatedly that he meant CPU operations as deterministic code analysis,
not wall clock, and believed that was what compile speed and run speed
measured. He was right on every count, and finding out took reading the page
rather than arguing from memory.

This page already names the instrument: "retired instructions as the third
instrument — the work a process did, reproducible to a few tenths of a percent
and immune to whatever else the box is doing." Counting them settles it.
Callgrind, both builds printing the same checksum:

    2026-07-27   2,545,249,871
    2026-08-07   2,762,364,162     +8.5%

The wall-clock regression was 7.6 to 9.2 per cent. The decoder is doing eight
and a half per cent more work, and every allocation counter was byte-identical
the whole way. Three consecutive runs give identical digits, so this pins like
any other golden and needs no band.

Where the work went, by function, from the same run:

    k_str_alloc        0 -> 40,464,393   (k_str_n fell 20,884,830)
    k_map_view_insert  0 -> 22,574,700
    k_b_put_mut                +21,683,400
    k_eq_rec                   +21,562,500
    k_b_push_mut               +21,398,550
    k_viewreg_migrate  0 -> 16,446,300

The runtime accounts for +127.4M of the +217.1M; the rest is in emitted user
code, which cannot be matched by name across the two trees because the older
one used flat module names.

### What this retracts

Two claims made this week go, and one survives.

Gone: that the slowdown is arrangement rather than volume. Volume rose 8.5%.
The padding experiment is real — 24 KB of never-executed code costs 1.6% — but
it is a small term beside the work itself.

Gone: that the model excludes time because time cannot be made deterministic.
That conflates wall clock with work done, which this project already separates
and I did not check. Work done is deterministic to the digit.

Survives: the three per-call guards. put_mut, push_mut and map_view_insert
together are about 66M instructions, roughly 30% of the rise, close to the 36%
the wall-clock removal measured.

### What was wrong on the chart

Two of its five lines were called "run speed" and "compile speed". They are
`allocs + encode_allocs`, and `compile_rounds + compile_visits + emitted_lines`.
Neither counts an instruction, so this regression was invisible to them by
construction. They are named for what they count now, and a legend gives every
line's derivation.

### PGO, measured and declined

Instrumenting the decode gauntlet and rebuilding against its own profile moves
it 8.5% — a number produced by profiling the program being timed, which is
overfitting by construction. A profile taken from encodebench, oneshot and
basket, never from decode, moves it 1.7%, lower in every one of five randomised
layouts. That reproduces the roughly one per cent this page already recorded
years of measurement ago, and it does not pay for a profile in the repository,
a job to regenerate it, and a published figure that depends on a workload
chosen in advance.

## 2026-08-10 — the view test at the call site buys nothing

`k_map_view_insert` costs 22,574,700 instructions across 1,254,150 calls in the
decode benchmark, eighteen apiece, and its whole body sits behind an early
return on `m->sorted`. The function is too large to inline, so the reading was
that every put pays a call to learn the map has no sorted view.

Hoisting that test to both call sites gives 2,762,362,598 instructions against
a baseline of 2,762,362,598, measured in one sitting on one toolchain with the
environment pinned. Identical to the digit, and basket likewise at 51,928,666.
clang already elides the call — partial inlining is exactly this shape — so the
eighteen instructions are the view maintenance itself, on a view that is live.

That relocates the target. The decode benchmark does build a sorted view and
keeps 1.25 million insertions into it, and the question worth asking is why a
decode asks for sorted order at all, not how cheaply the answer is maintained.
Declined; the branch is gone.

## 2026-08-10 — the instruction gate was reading the run id

The gate added yesterday fired on its second CI run for fourteen instructions
in three billion, one benchmark up and another down by the same amount, with
nothing in the compiler changed.

The kernel copies a process's environment onto its stack and libc walks it
before main, so a `GITHUB_RUN_ID` that gained a digit costs instructions. A
local pair of runs differing only in environment length read 51,930,665 and
51,930,740; with `env -i` both read 51,928,666. CI now measures with the
environment emptied, and every benchmark came down by about seventy-five
thousand — the startup walk leaving the measurement.

It happened twice. The pull request carrying this very log entry — a markdown
file and nothing else — failed the same gate, on a tree whose instruction count
could not have moved. Two occurrences in two runs is the rate, and a
performance gate that a documentation change can fail is a gate nobody will
believe when it fires for a real reason.

The pin stays exact. A gate needing a tolerance to survive its own second run
is measuring the runner.

## 2026-08-10 — one byte view per scalar token, and the fusion that would skip it

`k_bytes_view` costs 83,462,400 instructions in the decode benchmark, three per
cent of the run, across 3,091,200 calls at twenty-seven apiece. Every one comes
from `k_b_slice` and nowhere else.

That is not waste. `cs[p]` on bytes answers an int through `k_b_at`, so reading
a character allocates nothing; the slices are one per scalar token — the string
in `string_at`, the digits in `parse_number`, the keyword in `word`. Three
million tokens in four megabytes is the right order.

What it does show is a fusion. `string_at` reads

    text/utf8 (text/slice cs start (p - 1))

and `text/utf8` on a byte view validates and copies into a fresh string, so the
twenty-four byte header the slice just built is read once and dropped. 3,054,450
of the 3,091,200 slices are that shape. A rule that lowers utf8-of-slice to one
call taking a pointer and a length would skip the header entirely, which is
worth about two per cent of decode if the rest of the path is unchanged.

Not built. Recorded because the number is measured and the shape is specific.
## 2026-08-10 — utf8 of a slice, and two ways to be green and wrong

The decoder built one byte view per scalar token: 3,091,200 of them, 83.5
million instructions, every call from k_b_slice. The view could never pay for
itself, because k_utf8_finish takes a buffer without copying only when the view
owns capacity and a view carved out of another's bytes owns none.

Lowering the pair as one call: decode 2,762,362,598 retired instructions to
2,699,946,398 (-2.26%) and 7,577,414 allocations to 6,272,114 (-17.2%), 41.8 MB
less asked for. oneshot -0.69%. Encode and basket barely move, which is right —
neither reads text this way. Machine code grows 112 bytes per binary for the
kernel, and one emitted call replaces two.

Two defects on the way, both green through every check.

The first fired zero times. It matched the bare name `utf8`, and a std wrapper
keeps its qualified spelling until the forwarder map resolves it at the emit
site. Byte-identical counters caught it; correctness tests could not, because
the code was correct and dead.

The second broke the differential law. An err records the frame it was born in,
and `text/utf8` is a wrapper the oracle really calls, so fusing past it moved
the birthplace to the caller's file. The suite, the book samples and the
diagnostics differential all passed: nothing in the corpus reads an invalid
byte through a slice. Found by hand-building one. The fused call now interns
the origin from the wrapper's declaration; tests/golden/runtime pins it.

The intermediate repair is the one to remember. Fusing only the already-inlined
spelling removed the divergence and fired zero times — the first bug wearing a
correctness argument. A divergence going away is not evidence the code runs.

Checked and clear: to_int, to_float and from_code are the other builtins that
birth an err through a wrapper, and all three name the same origin on both
engines. The bypass this fusion introduced was the only one of its kind.

## 2026-08-10 — the stopwatch cannot see the fusion, and that is the point

Randomised-layout timing of the utf8-of-slice fusion, five neutral paddings per
tree, nine runs each, floor per padding, median of the five floors:

    base    137562 132535 136214 132672 133191   median 133191 us
    fused   129935 130662 133797 133226 133496   median 133226 us

Delta +0.03%. The spread inside one tree is 3.79% and 2.97%, comfortably larger
than the 2.26% the instruction count measures exactly. The decode board does
not move, and no millisecond figure on any page is touched by this change.

The first harness said the fused build was 0.75% SLOWER, which is how the flaw
was found: a 17% allocation cut and a 2.26% instruction cut do not produce a
slowdown, so the instrument was wrong before the code was. Each timestamp was
its own `python3 -c`, putting an interpreter startup inside every measured
interval — about thirty milliseconds on a benchmark that runs in a hundred and
twenty. Timing all nine runs inside one process removed it.

This is the case the counter veins were built for. Two exact numbers moved, the
clock could not say anything, and a per-commit claim in milliseconds would have
been an artefact of where the linker happened to put things.

## 2026-08-11 — GAVELED: as-patterns, `r@(rect w h)`

Clay asked whether kanso could use Ruby's `@`. Its Ruby meaning — mutable
per-object state — names a hazard kanso does not have, so importing it would be
decoration. The use that fits is the as-pattern: bind the whole value while
destructuring its parts, the spelling Haskell, Rust and Scala share. Ruled in.

It is not sugar. An arm that dispatches on shape loses the value it matched, so
returning that value means rebuilding it:

    pub fn rebuilt (rect w h) n
      stepped (rect w h) n

Passing the record through instead costs two allocations over a hundred
thousand iterations. Rebuilding it segfaults at a hundred thousand and survives
at fifty — the tail call is lost when an argument is a constructor application,
so recursion depth becomes stack depth. `@` removes the reason to write that
shape.

The lost tail call is a separate defect and is being fixed on its own terms; a
language feature that lets careful authors avoid a compiler bug is not a fix for
the bug.

Scope for the first cut: function arms, hugging spelling, no nesting until a
second real case asks for it.

## 2026-08-12 — DONE: tailcc kept wherever the arguments fit the registers

The rebuild that segfaults at a hundred thousand hops was not the beat carry and
not a lost tail call in the emitter. The IR holds a `musttail` for it. The
release build deletes it: `--release` stripped `musttail` and `tailcc` from every
line before handing the IR to clang, so the optimized build relied on -O3 to make
the jump on its own, and here it declined. A hand-built `-O3 -flto` binary of the
same IR with the two words left in runs the hundred thousand hops and prints 3.

The strip was there because `tailcc` is miscompiled on arm64 at -O1 and above.
That claim still holds — retaining it everywhere leaves ten of the ninety-five
micro samples segfaulting, nine through std/regexp. What was never measured is
where the miscompilation starts. Sweeping the corpus against a cap on how many
argument registers a parameter list may want:

    cap 8   0 of 95 fail
    cap 9   1 of 95
    cap 10  9
    cap 11  9
    cap 12  10

The boundary is exactly the arm64 argument register file, x0 through x7. A
%KValue and a %parsed are two registers each, so an arm of five or more values
spills, and regexp's hot arms take seven. Below the line the optimized build and
the unoptimized one agree.

So `--release` now strips the convention only from an arm whose arguments
overflow x7, and from the calls into it. Everything narrower keeps the guarantee
the optimized build used to give up — including every arm the beat machinery
brackets. `tests/golden/micro/a_record_rebuilt_at_depth.kso` runs two hundred
thousand hops through a pair of arms passing a rebuilt record; it segfaults on
the old rule and prints `w: 3` on the new one, on all three engines and under a
release build.

What a wide arm still gives up is the jump. It spends a frame per hop, which is
what it did before this change, and a deep recursion through one overflows the
stack — loud, where the convention's own defect is a binary that jumps to an
address that was a value.

Machine code moves slightly and in both directions on the measuring host:
jsonbench 79,488 to 79,364 bytes, encodebench 97,236 to 98,040, oneshot 96,668
to 97,380, basket 90,340 to 90,604. `bench/text_golden.txt` and
`bench/instructions_golden.txt` are regenerated from the runner in this PR.

OPEN: the miscompilation itself is not diagnosed, only bounded. A reduced case
against upstream LLVM would be worth having, and until then the cap is a
measured workaround rather than an explanation.

## 2026-08-12 — the guaranteed tail call is worth 2.96% of decode

The runner's callgrind numbers for the change above, which was written to fix a
stack overflow and turned out to buy speed:

    jsonbench    3,187,436,860 -> 3,092,945,560   -2.96%
    encodebench  9,283,433,083 -> 9,284,549,883   +0.01%
    oneshot         64,031,111 ->    63,411,063   -0.97%
    basket          56,717,877 ->    56,638,584   -0.14%

The decoder's arms are narrow — two or three values — so nearly all of them
cross back under the register cap and get the jump instead of a frame. Encode is
flat, and the code grew 0.37% on the decoder, which is the frame setup the
convention writes where the C one wrote a call.

Welfare 66.65 -> 66.72, banked. This is 2.96 of the 8.5% decode regression
recovered, on top of the 2.43% the utf8-of-slice fusion returned.

## 2026-08-12 — every gate now ships a defect it must catch

Five checks went green in one day while checking nothing, which is what the
ratchet is for: each of the twelve gating jobs in ci.yml carries a mutation —
a defect to introduce and the command that must then go red — or a written
reason it cannot have one. `kanso run scripts/ratchet` checks that
correspondence on every change and costs nothing; `-- prove` applies each
mutation in a scratch worktree nightly and refuses a gate that stayed green.

The first prove run found three of its own rows unfit, which is the point:

    BROKE  decode allocation      the golden's key was misspelled
    BROKE  grammar stops painting the file it named is not there
    BLIND  one engine reworded    the differential never reaches that message

The decode row had been editing the golden's number, which exercises the diff
and not the counter behind it — a counter reading a constant passes an edited
golden, and that is exactly the failure the ratchet exists to catch. It now
turns off the uniqueness fixpoint, which doubles the decode benchmark's
allocations, 6,272,114 to 12,539,564.

The engine-divergence row reworded a blackhole message the diagnostics
differential never asks about; that harness probes std functions and language
paths for the wrong type. It now rewords the map-key refusal, which an existing
probe hits and which is written once in each engine.

The grammar row, with its path fixed, still left the gate green. The check
asserts one scope, entity.name.type, where the job was titled "types, strings
and comments still painted". The title now says what it checks and the
coverage gap is a task.

Twelve of twelve red on the second run. The machine-code row is red on macOS
for the wrong reason — Apple's size takes no --format, and the gate now says so
rather than reporting section sizes it could not read — so prove is
authoritative on linux, where the nightly runs.

## 2026-08-12 — a 127 exit is a status, and native called it a failure

Found by the ratchet's own prove pass, where a gate whose binary the mutation
had removed reported "cannot start sh" and stopped the run.

127 is what a shell reports for a command it cannot find, and it is also a
status a program may choose. The exit code alone cannot tell them apart, so
native took every 127 for a failure to start. The oracle spawns through a
library that keeps a close-on-exec pipe for exactly this and reported the
status, so one program had two answers.

The child keeps that pipe here now. Exec closes it and the parent reads
end-of-file; when exec comes back instead, the child writes its errno through,
and those bytes are the only evidence that nothing ever ran. Both paths carry
it — the blocking run and the fork the scheduler waits on. Counters unmoved,
welfare 66.72 held.

The machine-code golden moved for it: +80 bytes on all four benchmarks, the
same number on each because the exec pipe lives in the runtime every program
links. Nothing on a hot path — the benchmarks start no processes — and the
allocation counters and welfare are unmoved.

    jsonbench    82,610 -> 82,690
    encodebench 101,538 -> 101,618
    oneshot     100,722 -> 100,802
    basket       92,082 -> 92,162

## 2026-08-12 — the sorted view a decode never asks for

A temporary counter in the map runtime, one run of the decode benchmark:

    view insertions attempted   1,254,150
    with a view to insert into          0
    sorted views built                  0
    beat pops migrating a view    632,550
    maps carried on any of them         0

So a decode builds maps and never asks one for a sorted view, and both
functions were being called only to answer a load and a branch. That divides
out to 18.0 instructions per insertion and 26.0 per pop, which is the frame
rather than the work, and it accounts for the whole 39M the 2026-08-07 profile
attributed to two functions that had not existed a fortnight earlier.

Each guard moves to its caller. The instruction vein, measured on the runner:

    jsonbench    3,092,945,560 -> 3,080,294,561   -0.41%
    encodebench  9,284,549,883 -> 9,229,327,520   -0.59%
    oneshot         63,411,063 ->    63,188,854   -0.35%
    basket          56,638,584 ->    56,597,311   -0.07%

The cost is 1,024 bytes of machine code on each of the four, the guard copied
to every call site. Allocation counters are byte-identical. Welfare 66.72 ->
66.75, banked.

Encode falls furthest, which is the part the profile did not predict — the
encode path writes maps too, and it was paying the same toll.

## 2026-08-12 — the whole and its parts from one match

`r@(rect w h)` binds the value that matched while its fields are destructured,
spelled the way Haskell, Rust and Scala spell it. Gaveled 2026-08-11.

An arm that dispatches on shape otherwise loses the value it matched, so
answering with it means building it again from the fields — and a rebuilt
argument is a constructor application rather than a value, which is a shape the
compiler will not tail-call.

The pattern grew a field rather than a variant: `Ctor { ty, fields, whole }`,
so every pass that only cares about the shape reads `..` and is untouched. As-
patterns are constructor-only, which the type now states. `@` hugs both sides,
and `_@` is refused — an as-pattern names what it matched and `_` names
nothing.

**An as-bound parameter gives up the by-value register convention.** That
convention passes a two-field record's words rather than the record, so there
is nothing to hand the name that would not have to be built, which is the cost
the pattern exists to remove. The opt-out is a property of the position rather
than of one arm: if any arm there names the whole, every arm at that position
is boxed. Getting that wrong does not produce a wrong answer — it emits
`call void @k_carry_stage(%KValue )`, an operand that is not there, and the
verifier says so.

The name is boxed, and that is not a detail. Held inline, an
`Option<(String, Span)>` costs every constructor pattern in every program forty
bytes for a field that is `None` almost everywhere: the front end's peak on
lib/json went 819,217 bytes to 868,507, a 6% rise the compile-memory gate
refused. Behind a box it is one word, and the peak reads 815,619 — slightly
under where it started, because the option now fits in the padding the variant
already had.

Air around the sigil belongs to the lexer, which refuses it before the parser
runs. The parser had grown its own copy of that check; it could never fire, and
the diagnostic corpus said so.

Three engines agree on `tests/golden/micro/an_as_pattern_binds_the_whole.kso`.
Open, and waiting for a second real case rather than a guess: whether `@` binds
in `=` bindings, and whether it nests.

## 2026-08-12 — the tie check ran on almost nothing

`check_arm_ties` opened with a guard:

    if parents.is_empty() {
        return;
    }

so a program declaring no subtype was never checked at all — and almost no
program declares one. The rest of the check does not need parents: `compare`
falls back to pattern rank, which is the whole of the answer for literal and
binder arms. The guard was reading "we only care about subtype chains" into a
function that had grown past that.

It surfaced sideways. Division answering a `math_failure` puts a string
subtype in every program, which switched the check on everywhere at once, and
lib/regexp stopped compiling. The first reading was that a subtype had changed
an unrelated verdict. It had not: the verdict had simply never been asked for.

With the guard gone, the sweep over lib, examples and scripts names eight arm
sets. Every one is a genuine tie by the rule, and two are order-dependent with
different answers on the ambiguous call:

    fn other code true _        code - 32
    fn other code _ true        code + 32

`other 65 true true` matches both. It is unreachable — the caller asks
`lower?` and `upper?` of one code — but nothing said so, and the arms were
settled by the order they were written in. `stopped 0 false` in golden_prose
is the same shape and reachable: one arm writes nothing, the other exits 1.

Where the two flags encode one three-way answer, the fix is one discriminator,
and then the state that made the arms tie cannot be written. `swapped` asks
`case_of` for "lower", "upper" or neither; `open_start?` had three alternatives
written as three arms and reads as one `or`. Where the shape is genuinely two
independent conditions, the arm the diagnostic names is the fix, and it says in
one line what the arm order used to say silently.

## 2026-08-12 — division by zero is a value

Gaveled 2026-08-10, and it took the tie-check fix above to land.

`math_failure` under `string` and `divide_by_zero` under that are installed in
every program, because `/` is the compiler's and an arm of an operator may only
match a type its own module defines — no library could name what a primitive
division produces. A handler asks for whichever it wants: the specific failure,
any math failure, or the reason as text.

    fn told f:divide_by_zero    specifically: division by zero
    fn told f:math_failure      some math failure: ...
    fn told n                   a number: 2

A computed zero divisor answers the value on all three engines. A literal one
is still refused where it is written: that check is named `decidable_walk` and
fires only when no input was involved, which makes it a provable mistake rather
than a state a program can reach.

Two things the build turned up.

The name is per-compiler, not per-module. Qualification renames the types a
module owns, so every module was getting its own `math_failure` and the one
division builds matched none of them. The prelude is excluded from that rename.

The declarations cost compile work: the module benchmark's emitted lines go
4,430 to 4,528 and its calls 748 to 749, the second being the call that tells
the runtime which ids the compiler assigned. Two type declarations in every
program is what that buys, and there is no cheaper way to let a primitive
operator answer a declared type.

Two interpreter unit tests used `1 / 0` as a convenient err and now raise one,
which is what they were always about.

## 2026-08-12 — the prelude only where a program can reach it, and the book relearns failure

The first draft of #159 installed `math_failure` and `divide_by_zero` into every
compilation unit and let qualification rename them, so jsonbench shipped six
copies — `io/math_failure`, `render/math_failure`, `jsonbench/list/…` — and each
took a case in every type-metadata table the emitter writes. Emitted lines rose
10,529 to 10,676 and the module benchmark rose 4,430 to 4,528. Welfare fell 0.19
under the floor and CI refused the merge, which is the gate doing its job.

Qualification now drops the pair rather than renaming it, and the merged program
gets one bare copy back. The first rule for installing it was "the program can
reach a math failure" — a `/` or `%` anywhere. That was wrong, and the gate said
so: encodebench came out 4.2% slower, 9.23 to 9.62 billion instructions, and
oneshot 1.5%, while jsonbench and basket moved by about thirty instructions each.
An asymmetry that large is not layout noise, which moves every row.

encodebench renders numbers, digit extraction divides by ten in its innermost
loop, and it never once asks which failure it has. It was paying for a
distinction it cannot observe. So the rule is observability: the pair is declared
only when some arm names `math_failure` or `divide_by_zero`, or a type is
declared under one. Where nothing names them, `10 / 0` answers the bare string —
same text, same output, and nothing in such a program can tell the difference.

One golden does move, and not because of the compiler: the machine-code row
falls 864 bytes on encodebench, oneshot and basket, and holds on jsonbench. The
emitted IR is byte-identical for all four, so the fall is entirely in the
runtime — division and modulo used to raise, building a string and carrying an
origin pointer at each of five call sites, and now they answer a value from one
argument. jsonbench links neither `k_div` nor `k_mod`, so the linker never
brings the smaller code in and its row does not move.

The rest is that a program which does not ask is compiled exactly as before.
All four benchmarks, the five compile samples and the module sample are
byte-identical to main; encodebench measures 8.746 billion again. Every golden
here reverts rather than moves, and welfare reads 66.75, the floor. The feature
costs what it costs only where it is used, which is the honest place to charge
it.

The book had built chapter 04 on division by zero as its running err, and that is
no longer what division answers. Seven samples moved. Chapter 02 keeps the
division and teaches the new thing: the value reads as its reason. Chapter 04
needs a failure a caller genuinely cannot handle, so `share_of` says `err "no one
to share the bill with"` and the railway runs unchanged — born in the reader's own
code, trace through `receipt ← with_tip`, past a catch-all arm that never sees it.
The merge-and-short-circuit pair take a strict index miss beside the parse
failure, which are two distinct errs where there was one. Appendix A's endpoint
entry is now `endpoint.kso`, a strict index rather than a division.

Regenerating those panels turned up something older. `book_panels --write` is
killed on native after 3.9 trillion instructions and a 579 GB peak, where the
oracle finishes in a moment; origin/main does the same, so it predates this work.
The read path is fine because `keeping path text (not write or text == raw)`
short-circuits and never forces the chapter it built — which is also why no gate
has ever exercised the write path. Recorded as its own thread with the CI gap
beside it.

## 2026-08-12 (later) — a 580 GB write, and a builder re-seeded every iteration

`book_panels --write` is killed on native after 3.9 trillion instructions and a
580 GB peak, where the oracle finishes in a moment. It predates the math-failure
work; origin/main does the same. Regenerating panels with `--interp` is the
workaround until this closes.

The trigger looked absurd. Holding the chapter prefix fixed and growing the
trailing text, the same six panel rewrites finish at +562 bytes and run away at
+563. A `sample` profile ended the guessing: every stack sits in
stitching → fixed → no_recorded → settled_out → outing → escaped →
regexp/rebuilding. `escaped` is the three `replace_all` calls that turn `&`, `<`
and `>` into entities, reached only down `no_recorded` — the branch for a panel
whose recorded output cannot be found. One byte decides whether any panel reaches
that branch at all.

`replace_all` is quadratic in subject length wherever it matches: 1,020
characters cost about 5M instructions and 32,640 cost 1,554M, with a 448 MB peak.
The shape is the cause. Two loops doing identical appends:

    fn direct acc n
      direct "{acc}…" (n - 1)

    fn laundered acc n
      kept = "{acc}…"
      handed kept n

    direct     2000 →  15,457,481   4000 →  17,081,656   8000 →  19,027,210
    laundered  2000 →  67,489,554   4000 → 207,241,899   8000 → 752,908,406

Direct is linear; handing the accumulator through one intermediate function is
quadratic. `replace_all` is written the second way — `swapped` computes
`kept = "{acc}{before}{repl}"` and hands it to `stepped_over`, which recurses.

The mechanism is in `call_arg`. A string a group builds by joining onto itself
needs its seed converted where it enters from outside, because a builder writes
into the header it was given and an interned literal cannot be written through.
The self-recursive call is exempt, since it already carries the builder made
here. Nothing else is: `entering` is true for any call that is not the group's
own, so `handed` calling `laundered` re-seeds. `k_b_str_builder` mallocs
`2 * len + 32` and memcpys the whole string, so the accumulator is copied once
per iteration — the copy the analysis exists to remove, reintroduced by one hop.

The guard: `k_b_str_builder` returns its argument when the header it arrived in
still has room. Seeding again would copy the whole string, and a group whose
parameter is flagged has every caller handing it over uniquely, so that header is
ours to write. When the buffer is full the seed still happens, which is the
growth the append would have paid anyway.

    laundered, n=8000      752,908,406 -> 16,519,843
    the fatal fixture   3.96e12 instr, 582 GB -> 120M instr, 8.9 MB
    book_panels --write            killed -> completes

`tests/golden/mem/a_builder_handed_on_is_still_a_builder` pins it, and it was
watched red first: without the guard its 40 appends cost 40 mallocs, one per
iteration. It costs 25 now. The same appends written without the hop cost one,
because that shape never reaches the builder path at all — closing that gap is a
separate thread, and the golden says so rather than implying this is the floor.

The first version of that golden proved nothing. It used `text/append`, which
does not go through the seed at all, and its counters were identical with the
guard and without it. A spec that cannot fail is worse than none, and the only
reason this one was caught is that reverting the fix is a step rather than a
formality.

Worth recording that the first reading of this was wrong. `string_builders`
looked like an analysis nothing called, because `grep` returns nothing on
codegen.rs — a quirk already written down in this repo and not applied. It is
called, at codegen.rs:442, and its two sets are consulted at 2637 and 921. The
analysis is fine and flags both shapes; what fails is the exemption being keyed
on self-recursion rather than on whether the value handed over is already a
builder.

Second gap, separate: no gate exercises the write path at all. `book_check.sh`
runs book_panels read-only, and the read path never forces the chapter it built,
because `keeping path text (not write or text == raw)` short-circuits.

## 2026-08-12 (later still) — a build block binds into the scope around it

design/build-blocks.md has carried an amendment since 2026-08-01: "build blocks
don't return anything. their result is just present in the outer scope." Half of
it shipped. The compiler went on requiring a result expression as the block's
last line, so every build block in the tree was written to the surface the
amendment overrode — that was the only form that compiled. Clay found it by
reading examples/build_cyclic_eq.kso and asking why the block ends in `a`.

The first attempt read the problem as scoping and was wrong in an instructive
way. The parser requirement is one `matches!`. The checker keeps a block's names
by not truncating its locals. The interpreter threads the environment through its
three statement loops rather than cloning it. All three were written, and `a`
was still unknown, because `build` is an expression and the entry's statement
grouping folds consecutive statements into `Join` and `Seq` — so the block
arrived at `eval()` as a value and no statement loop ever saw it. Instrumenting
the arms printed nothing; instrumenting `eval` printed it.

Making `build` a `Stmt` variant produces non-exhaustive matches at 104 sites
across twelve files. The smaller lever is where the burial happens: `has_surface`
decides which bodies get the grouping machinery, and it counted a build as a line
of the effect surface. A construction site is not surface. Excluding it there
leaves the build a plain statement in the body, which the three threading loops
already handle, and no AST change is needed at all.

Two rules followed from that. The unused-expression check exempts a build,
because it has no value to go unused. And `x = build` is refused on its own
terms rather than by the removed rule — "`build` answers nothing to bind `x` to".

The cohort question the amendment left open is answered by counting. A block can
only reclaim what it allocated and did not bind, and across every build block in
the tree nothing is allocated that is not either bound or reachable from
something bound: `a.peers = [b]` allocates a list that lives on through `a`, and
the records are all named. So the boundary reclaims zero, which is what a
construction site is — you name the parts to wire them together. It still cannot
dissolve: build_write_enclosing_block refuses a write from an inner block to a
name bound in the enclosing one, and that diagnostic needs the block to have an
identity. So the block keeps its identity for legality and does no reclamation.

The wasm backend needed the matching change: a build leaves no word on the stack
when its last statement is a field write, which the validator caught as "expected
a type but nothing on stack". It now emits the statements and answers a none the
caller drops.

Sixteen fixtures moved to the amended surface, plus both examples, the ch03 knot
sample and its prose, the two playground examples, the design doc's own surface
section, and the compile-cost sample — whose visits fell 26 to 24, two fewer
expression visits because the block no longer has a result to type. Every other
counter is unmoved and welfare holds at 66.75.

## 2026-08-12 — the write path gate reached the wrong branch, and there is still a runaway

The gate #862 added watches `book_panels --write` on a copy of the book. It stales
ch03 and deletes a recorded output. Neither reaches the code it was written for.

A missing `.out` takes `no_recorded`'s first arm, which leaves the panel alone —
the reading being that a chapter may quote something the book does not own. The
comparison that escapes a recorded body into html only runs when the `.out` is
present and disagrees with the chapter. And ch03 carries no output panel at all:
every one of its panel titles is a bare `.kso` name, so the output shape has
nothing to match there. The gate exercised one path, the source panel, and its
comment claimed the other.

Staling both shapes in ch04, which carries both, ends in SIGKILL at 83 GB
resident. On current main, with #860's guard in place. Isolated:

    one source panel                     1 rewritten,    249 MB
    one output panel                     1 rewritten,    248 MB
    two source panels                    2 rewritten,    249 MB
    one output in ch04 + one in ch03     2 rewritten,    248 MB
    two output panels, one chapter       SIGKILL,   88,200,871,936 bytes
    source + output, same sample         SIGKILL,   83,283,951,616 bytes

Two rewrites in one chapter, at least one of them an output panel. Two in
different chapters are fine, which puts it in the per-chapter accumulator rather
than anything a chapter does on its way in. It is not a quadratic: the smallest
case so far is a chapter of 3,896 bytes reaching 52.8 GB, and the same two panels
inside a twelve-panel chapter finish in 246 MB. What separates them is what
follows the second rewrite, not how much precedes it.

This is almost certainly the original 580 GB runaway, which means #860 fixed a
contributor and not the cause. Recorded as task #204.

Three claims from that investigation are corrected here.

The escaping is not where the bug bites. With the #860 guard removed, the same
write path peaks at 16 to 19 MB over the staled book, over a chapter truncated
mid-panel, and over recorded bodies grown to 10, 20, 43 and 82 KB — the escaping
reads its subject from a file, where cap equals len and the guard has nothing to
answer. What #860 fixes is an accumulator handed through an intermediate
function, which is the shape its mem golden already pins.

That guard is a constant factor on the allocation counters, not an order of
growth. Both sides are quadratic in `alloc_bytes` for a hop-carried accumulator,
at a steady ratio across sizes:

    n=200    4,229,649 fixed    6,300,049 buggy      809 / 1,409 mallocs
    n=800   58,435,913 fixed   91,197,513 buggy    3,209 / 5,609 mallocs
    n=3200 898,049,513 fixed 1,420,775,913 buggy  12,809 / 22,409 mallocs

The 45x figure in `bench/text_golden.txt` is an instruction count, and these are
allocation counters; `alloc_bytes` reserves where instructions copy, so the two
do not contradict. Nothing here re-measures instructions, and that claim stands
as it was recorded.

And the reading that a truncated chapter reproduced the runaway was wrong. It
came from a run that printed nothing, which was taken for a kill. Rebuilt from
the same 7,717-byte prefix against a compiler with the bug restored, it finishes
in 18.7 MB.

The corrected gate stales both shapes in ch04 and asserts both come back. It
turns red on #204, so it lands with the fix rather than before it.

## 2026-08-12 — the builder guard is withdrawn: spare capacity is not sole ownership

#860 added one line to `k_b_str_builder`:

    if (src->cap > src->len) return sv;

with the reasoning that a group whose parameter is flagged has every caller
handing the accumulator over uniquely, so a header arriving with room to write
is ours. The second half does not follow from the first. Spare capacity is a
property of how a string was allocated, not of who holds it. `text/join`
over-allocates, so its result arrives with room while the parts it was built
from are still live, and a builder that trusts the header writes into a buffer
somebody else is holding.

What that costs, measured on ch04 of the book with two panels staled and
`book_panels --write` run over a copy:

    guard in    SIGKILL, 83,283,951,616 bytes resident
    guard out   2 panels rewritten, 265,682,944 bytes, chapter byte-identical

Bisected to it by ablation, one substitution at a time, each against the same
fixture: keeping the old body for an output panel is fine; splicing the recorded
body in unrendered is fine; `render_output` with the escaping removed is fine;
the `tagged` true-arm replaced by `escaped line` is fine, while the same
substitution in `pointed` or `carets` still runs away. Inside that arm, `rest =
parts[1]!` is fine and `rest = text/join parts ":"` is not. A join, feeding a
builder, through a header the join did not own.

So #860 fixed a real cost and bought it with an unsound test. The cost is real:
40 mallocs for 40 appends through one intermediate function against one without
the hop, and the mem golden goes back to pinning 40. What a builder needs before
claiming a header is a uniqueness it can prove. That is a piece of work, not a
comparison of two integers, and it is task #203's to do.

Three things this corrects in the #860 record. The claim that the guard took
`book_panels --write` from killed to finished is backwards — it is what killed
it. The 45x instruction figure in `bench/text_golden.txt` was measured on a
shape nothing re-tested here, and is withdrawn with the guard rather than
disproven. And the 580 GB runaway that started all of this is still unexplained:
it predates the guard, and nothing measured since reproduces it.

The corrected write-path gate goes red with the guard and green without it,
which is the first time anything in CI has had an opinion about this code.

## 2026-08-13 — a builder carried through a hop needs no seed

An accumulator handed round a cycle was re-seeded on every hop, and seeding copies
the whole string. The exemption at codegen recognised only a group's own
self-call as staying inside the cycle, so `build_onto -> handed -> build_onto`,
`walked -> walking -> walked` and `stitched -> stitching -> stitched` each paid a
copy per iteration. #860 tried to buy that back with a runtime `cap > len` test,
which was unsound and cost 88 GB on the write path before it was withdrawn.

Nothing about it needs a runtime test. `callers_hand_over` already proves, whole
program, that every call site passes a uniquely-owned value at a position. What
was missing is the same question asked one hop further out: a parameter a group
forwards straight into a carrying position is carrying one too, when every arm of
the group forwards it, every caller hands it over, the arm mentions it nowhere
else, and some call site actually names the group. Carriers join the set whose
callers convert a seed, so the conversion moves out to where a value enters the
cycle rather than happening on every lap.

That alone dies in a beat loop, and the reason is worth recording. The carry copy
strips builder-ness on purpose — `ns->cap = s->cap < 0 ? s->cap : 0`, "a builder's
positive cap does not travel: the copy owns no room" — so a seed made once and
carried across a rewind arrived at the next join as a string with no room, and
`k_concat_arr_mut` said so. Two changes make seed-once true. The builder's header
is malloc'd like the storage it already pointed at, so a rewind reaches neither.
And a string with a positive cap is carried by identity rather than copied, which
is what `k_b_str_builder`'s own comment has claimed since it was written: its own
header, outside the loop that will grow it, storage the arena's rewinds cannot
reach.

Three mem goldens fall. allocs 82 to 1, 10 to 2, 10 to 3; alloc_bytes 5,304 to 48,
22,557 to 80, 8,246 to 112; bytes_malloc 40, 7 and 6 all to zero. One malloc was
what the same appends cost written without the hop, so the hop is now free. basket
follows: allocs 28,192 to 28,184, alloc_bytes 4,890,672 to 4,882,522, bytes_malloc
16 to 10. Welfare holds at 66.75 — this vein reads instructions, and the
allocations it removes were not the ones it weighs.

Where the durable header comes from went four ways before one of them was free.
Mallocing it at every seed buys the whole win and costs basket 2.9% of its
instructions, and welfare refused that. Choosing the seed by whether the callee
is a beat loop, or by whether the position is one it carries, puts basket's
evacuation copies back — 0 to 8,012 — because the seed and the staging are
different call sites and a predicate at one can disagree with the other. What
works is to stop predicating at the seed at all: every seed stays an arena bump,
and `k_carry_stage_kept` moves the header off the arena when `k_survives` says
the rewind would reach it. The staging site is the only place that knows which
header is carried by identity, and it is the place that already knows it. At most
once per builder, because a promoted header survives the mark from then on.

The first cut of that carried ANY string with room by identity, written straight
into `k_deep_copy`, and kq caught what this repo's corpus did not:
`unicode_identity` came back as 267 NUL bytes, the right length of freed storage.
Capacity is not ownership — two references could alias one builder, and the next
growth reallocs and frees what the other is still reading. The compiler knows
which slot it proved, so the carry is told: `k_carry_stage_kept` marks the
accumulator's slot and only that slot crosses by identity. kq is green on it,
specs, its own cost goldens and the scale gate.

Two dead ends are recorded so they are not walked again. Mallocing the header
alone does not work: the copy still strips the cap, so the builder arrives roomless
whatever its header is made of. And guarding the exemption on `beat.ids` — leaving
beat loops to re-seed — is correct and costs 80 mallocs against main's 40, because
the boundary seed then buys nothing and adds one.

The smallest failing program was tests/golden/micro/walking_multibyte_forward run
through an import, which is the plain `walked/walking` shape and reached the
corpus check before anything larger did.

## 2026-08-13 — matching a keyword in kanso costs more than the slice it saves

Target 2 of the decode regression. `word` in lib/json/value.kso sliced bytes out
of the input and compared the slice to the keyword, and the keyword was a list of
codes — `bytes_null = [110 117 108 108]`. Comparing bytes to a list walks the two
element by element with a tag check on each. Probed on jsonbench: about 900,000
comparisons, every one an equality, every one K_BYTES against K_LIST, not one
failing on length, 3,900,672 elements walked. That is the 96,257,700 instructions
the caller attribution put under k_eq_rec'k_cmp.

Matching the keyword where it sits removes the walk and the slice together, and on
every counter this project keeps it read as a win: decode allocs 6,272,114 to
5,334,614, alloc_bytes 292,667,712 to 262,667,712, sh_bytes 50,450,400 to
27,950,400, arena_blocks 3 to 2, arena_peak_bytes 3,145,728 to 2,097,152, oneshot
allocs 119,826 to 81,598.

Retired instructions say the opposite, and they are the measure that matters here:
decode 3,080,294,566 to 3,454,549,280, a rise of 374,254,714 and 12.2%. Against
940,000 allocations removed that is about four hundred instructions added per
keyword. Recognising one character now walks three dispatch groups — `word_at?`
into `word_step?` into `word_more?` and back — where the slice form made one call
and one comparison, so the per-character machinery costs more than the allocation
and the list walk it replaced. Written in kanso, at this granularity, dispatch is
the expensive part. The win is still there to take; it needs a runtime compare of
a byte range against a literal, one call and no allocation, rather than a
character loop spelled in the language. Reverted, target 2 reopened.

oneshot falls the other way over the same change, 63,188,864 to 51,301,180. Four
new functions moved the emitted decoder — defines 154 to 159, lines 10,529 to
10,807 — and this page already holds a measurement that a no-op change moves
decode about three per cent by layout alone. An 18.8% move on the smallest
benchmark from four added functions is not a result to bank on one sitting.

### The gate that should have caught this, and did not

Welfare scored the change 66.77 to 66.97 and the floor was ratcheted on that
reading. The reading was of a stale vein: welfare's four run-speed terms are read
out of bench/instructions_golden.txt, and that file still held the previous
change's numbers because the instruction gate runs behind the machine-code gate,
which was red for an unrelated reason for two runs. So the model was shown a
decoder that had not moved and an oneshot that had not moved, and reported a gain
on the memory terms alone. With the measured numbers written in, welfare reads
66.97 for real — the model genuinely prefers this trade, because oneshot's 25.8%
outweighs decode's 5.4% at their weights. That is a second thing to settle, and it
is not settled here: the model would have banked a 12% decode regression on the
strength of a benchmark move that layout can explain.

Welfare must not be able to score a golden it has not seen regenerated. A number
computed from a stale input is worse than no number, because it carries the
authority of the gate.

The obvious spelling was tried first and refused by measurement. Writing the
keywords `text/bytes "null"` makes every comparison a memcmp, and costs one
allocation per comparison: decode allocs rise to 7,209,608. `Backend::is_constant_body`
(codegen.rs:1872) gives a constant a CAF cell when its body is a literal, and a
call is not a literal, so the bytes constant is rebuilt at every use where the list
literal is built once at k_caf_init and frozen. That gap is worth closing on its
own — any module-scope constant computed by a call is rebuilt per use, language
wide — but it is a language question, because a frozen constant is built before
main and a body that can fail would fail earlier. Left for a gavel.

The cohort fixture was rebuilt along the way and is kept, because what it found
outlives the change that exposed it. tests/cohort.rs pins that a bound,
branch-chosen, piped decode frees its construction garbage, and under the
keyword rewrite it kept instead. Probed at the guard with the survivor budget
lifted: the survivor is 995,504 either way — it is the
decoded document, which is what the cohort's region returns — while `grown` halves
from 2,097,152 to 1,048,576, because the garbage that filled the second arena
block is gone. Nearly all of a one-block region surviving is exactly the case the
guard's comment names, "a decode that is nearly all live tree", and keeping it is
right. So a document with a big tree can no longer pin the freeing side at all.
The fixture now decodes a long JSON string of `\u` escapes, where six bytes become
one and the survivor stays a fifth of what grew; the test writes that input rather
than committing a megabyte. Watched red at 50,000 escapes, where the region never
trips the threshold and the cohort does not fire.

Two harness lessons paid for twice each. The stdlib is include_str!-embedded and
disk does not win, so swapping a lib/*.kso between refs without rebuilding reads
the old stdlib. And `survivor` at the guard is bounded by
`k_copy_size_budget = min(cap, grown/2)` and abandoned as soon as it exceeds it —
a reading near grown/2 is the budget, not a size.

## 2026-08-13 (later) — the string header is made inline

Target 3 of the decode regression, and the cheapest of the four. When a string's
header and its bytes became one allocation the work moved into `k_str_alloc`, and
the caller attribution taken during the regression hunt shows what that cost:
40,464,393 instructions in the new function against 20,884,830 that `k_str_n` gave
up. The body is a few stores and a branch on the stats flag, small enough that the
call around it is a fair share of the work.

Forced inline, the symbol leaves the binary entirely — `nm ./jsonbench` no longer
finds it — and every benchmark falls: decode 3,080,294,566 to 3,067,580,067
(0.41%), encode 9,229,331,934 to 9,207,299,438 (0.24%), oneshot 0.22%, basket
0.30%. Allocation counters are byte-identical, which is what inlining owes them.

The machine code falls too, which was the outcome least expected: jsonbench 83,794
bytes to 81,458, and 2,112 to 2,416 off each of the other three. Forty-eight call
sequences are larger than forty-eight copies of a body the optimiser can fold once
it can see the length it is called with. Both veins moved the same way, so there
was no trade to argue.

One caution for the next reader of a profile: the attribution predicted about
twenty million on the decoder and twelve and a half arrived. It named the right
function and oversold the size, because attribution counts what a function was
charged, not what removing it gives back.

## 2026-08-13 (later still) — the keyword, spelled straight

Target 2, second attempt, and the measurement that the first one earned. The
scanner sliced bytes out of the input and compared the slice against a keyword
held as a list of codes: an allocation per literal, then two collections walked
element by element with a tag check on each.

The first attempt matched character by character in kanso and cost decode 12.2%.
This one is straight-line. The first byte already chose the arm, so what is left
is the tail of one known keyword — three or four comparisons and a single
dispatch on the answer. Reading past the end answers none, which matches
nothing, so a truncated literal needs no length test.

    decode      3,067,580,067 -> 2,900,220,537   -5.46%
    oneshot        63,049,215 ->    47,548,171  -24.59%

Same idea, two spellings, eighteen points apart. What separates them is not the
algorithm but how much dispatch stands between the comparison and the bytes:
three groups per character against none. Written in kanso, at this granularity,
dispatch is the whole cost.

Allocations follow: decode 6,272,114 -> 5,334,608, arena blocks 3 -> 2, peak a
third lower, alloc_bytes down 30 MB, sh_bytes 50.4M -> 28.0M, and three fewer
permanent allocations now the keyword lists are gone. oneshot's evacuation
collapses, 63,967 copies to 5, and its carry_dedup goes to zero. The emitted
decoder grows 522 lines for three predicates, and 560 bytes of machine code per
binary; the front end visits 600 more expressions. Welfare 66.79 -> 67.37.

oneshot's cohort flips from freeing to keeping, as it did under the first
attempt: the construction garbage that filled a second arena block is gone, so
nearly all of one block survives and the guard's keep arm is right.

This is also the first change scored under the repaired gate. The counters were
red, and the instruction vein reported anyway — six veins each said their piece
in one run, which is what the first attempt's 12.2% needed and did not get.

## 2026-08-13 (later still) — where the decoder's work actually is

Target 4 needed a profile and the profile already existed: the instruction gate
runs callgrind over all four benchmarks, reads one number out of stderr and
discards the rest. Every previous time this page asked where the work went,
somebody stood up a bespoke run to recover what that file already held. The
gate prints it now (#869), for the decoder and the one-shot.

The decoder, 2,900,220,537 instructions:

    642,319,500 (22.15%)  value_for
    281,591,550 ( 9.71%)  obj_key_start
    201,726,450 ( 6.96%)  k_b_append_mut
    150,691,950 ( 5.20%)  array_delim
    133,687,650 ( 4.61%)  k_b_put_mut
    119,519,100 ( 4.12%)  array_step
    118,048,650 ( 4.07%)  k_utf8_bad
    115,818,750 ( 3.99%)  str_char
    102,122,570 ( 3.52%)  memcpy
    102,067,050 ( 3.52%)  k_b_push_mut
     94,880,250 ( 3.27%)  string_at
     89,752,050 ( 3.09%)  k_b_find2
     73,770,450 ( 2.54%)  obj_delim

Emitted user code is about half the decoder and value_for alone is a fifth of
it. Two things it is NOT: the dispatch, which already lowers to one switch and
eight comparisons, and k_utf8_bad, which despite the name is the validator
rather than an error path — Keiser & Lemire with an ascii fast path, and four
per cent to validate a document is honest. Why value_for costs what it does,
beyond running once per json value, is not yet established. Per-line
annotation, not opcode counting, is what will answer it.

### The reason five readings in a row were wrong

src/codegen.rs held two NUL bytes, in the diagnostic literals interned as C
strings. A NUL makes grep call the file binary and print nothing, and nothing
is indistinguishable from no-match. Searching 4,433 lines of the emitter for
`Index`, `tailcc`, `extractvalue` and `insertvalue` returned empty every time.
All four are present — 2, 32, 41 and 29 occurrences.

From that silence came the conclusions that codegen.rs was not the emitter,
that the aggregate opcodes must come from somewhere else, and a whole theory
that value_for's cost was tagged values marshalled across the call boundary.
The theory was written down as a finding before python counted the same file
and disagreed with grep. Escaped as `\0` the string is the same string, and
jsonbench.ll is byte-identical either way, which is the check that matters
(#870).

The general form is worth keeping beside the harness lessons already on this
page: a tool that cannot answer looks exactly like a tool answering no.

## 2026-08-13 (last) — value_for's cost is per call, and the number to beat

Target 4 had a profile and no account of it. `value_for` is 22% of the decoder
at 642,319,500 instructions and the only thing anyone could say was that it runs
once per json value. Call counts settle it — they come from the profile already
collected and need neither source mapping nor an instruction dump.

    value_for called 1,188,150x from obj_key_start alone
    self cost 642,319,500
    => 400 to 540 retired instructions of SELF cost per call

For a function that switches on one byte and hands off, that is enormous, and
the switch is not where it goes: the dispatch is one `switch` and eight
comparisons. The body is the target, and it now has a number. A hundred
instructions per call is about 120M, four per cent of the decoder.

The same probe measured the workload: 1,188,150 object keys — `string_at`,
`k_b_put_mut` and `k_b_find2` are each called exactly that many times from
`obj_key_start` — with 412,650 array opens and 348,150 object opens.

### Four ways not to ask this question

Written down because each cost a run or a retraction.

Opcode counts in the emitted IR are not machine cost. A theory of value_for
built on counting its `extractvalue`s was retracted; SSA aggregate ops are
plumbing the backend folds into registers.

`callgrind_annotate` cannot render `--dump-instr` data at all — that needs
KCachegrind. Two probes spent proving it, the second by adding `--auto=yes`,
which does something else entirely.

`--auto=yes` annotates SOURCE, and every kanso frame in a profile reads `???`
because the emitted IR carries no debug metadata. No clang flag invents that;
the `!dbg` records would have to come from codegen. Worth knowing anyway: `-g`
does not perturb the pinned instruction counts, so emitting debug info later is
safe from the goldens' side.

What worked was the cheapest thing available, asked last.

## 2026-08-13 — value_for is not overhead, and the decode campaign closes

The 22% attributed to `value_for` is parsing, inlined. Two facts settle it,
and both were a shell command away the whole time.

`nm` finds exactly ONE `value_for` symbol in the binary. So the profile's
`value_for_3'2` is callgrind's recursion-depth notation, not an LLVM clone,
and the 642M is genuinely that function's own instructions.

That symbol spans **3,076 bytes of machine code**, for a kanso function that
is a switch and eight arms. LLVM has folded the parse routines into it. So
"value_for is 22% of the decoder" reads "parsing a value is 22%", which is
unremarkable for a json decoder, and there is nothing there to remove.

Everything structural around it was already optimal, read off the emitted IR:

    define tailcc %parsed @"d_jsonbench/value_for_3"(i64 %x0r, %KValue %x1, i64 %x2r)
    entry:
      %t1 = icmp eq i64 %x0r, 256
      ...
    L2:
      switch i64 %t4, label %arm7 [ i64 34 ... i64 123 ]

The dispatch byte and the position both cross as raw `i64`; only `cs` is a
tagged pair, and it cannot be otherwise. The `none` case rides as sentinel 256
in that raw i64 — which is the "widen unboxing with a sentinel" idea written up
here as future work, already built. The entry rebox is the documented one SROA
folds against the switch.

Target 4 asked where ~90M of instructions in emitted user code go. The answer
is that they are not overhead, so the campaign that opened with an 8.5%
regression closes here:

    decode instructions   3,080,294,566 -> 2,900,219,722   (-5.8%)
    decode allocations        6,272,114 ->     5,334,608   (-15%)
    arena blocks                      3 ->             2
    welfare                       66.75 ->         67.37

If decode is attacked again, the honest starting points from the same profile
are the runtime kernels, which are inlined into nothing and are plainly
themselves: `k_b_append_mut` 6.96%, `k_b_put_mut` 4.61%, `k_b_push_mut` 3.52%,
`memcpy` 3.52%, `k_b_find2` 3.09%. About 21% together.

### Seven wrong predictions, and what they have in common

The keyword rewrite, the dispatch lowering, `k_utf8_bad` as an error path,
which file holds the emitter, opcode counts as machine cost, INT|NONE forcing a
box, and an LLVM clone. Every one died to reading or measuring rather than to
argument.

Two lessons that generalise past this decoder. The obvious optimisation is
usually already implemented here — the sentinel was in the code before it was
in the plan. And the profile rarely points where the code shape suggests, which
is the entire reason `bench/instructions_golden.txt` exists.

## 2026-08-13 — the renderer is right to demand nothing, and one sentence of it is wrong

Native prints `<thunk>` where the oracle prints the value:

    fn noted acc _ true
      acc

    fn noted acc why false
      text/concat acc [why]

Neither engine has a missing force. They defer at different points. The oracle
barely defers at all — `stored()` builds a cell only for an expression that
awaits a knot, so an interpolated string binding is evaluated eagerly and no
cell ever reaches the list. Native defers on strictness: a parameter crosses
already-evaluated only when every arm demands it, and the `_` arm means one
does not. The emitted IR shows the dispatcher storing its parameter with no
force and no release, which is correct for that policy.

So native is lazier than the oracle, and the difference escapes as a word no
program meant to print.

### Making the renderer force, declined

The obvious repair is one line in `k_render`:

    if (v.tag == K_THUNK) return k_render(k_force(v), quote);

It makes the reduced program agree on both engines. It also turns

    ring = [ring]

from `[<thunk>]` into `error[runtime]: the program ran out of stack`. Once the
constant finishes, its cell is forced to a list holding that same cell, so
forcing inside rendering recurses forever. The micro corpus catches it twice,
across-engines and release-built. Reverted.

Which settles what `<thunk>` is for. It is not a marker that somebody forgot a
force — it is the terminating display for a cell that cannot be demanded,
and rendering must not demand or a cyclic value has no printable form.

The comment above that branch is therefore right about the design and wrong
about the fact it cites: it says the oracle answers the same way, and the
oracle prints the value, never having built a cell. A load-bearing comment
asserting agreement where the engines diverge is how the next reader gets this
backwards.

A real fix belongs where the cell is made, not where it is shown — most likely
by widening the strictness rule so a parameter an arm ignores is still passed
evaluated when it is cheap and non-recursive. That decision meets the
self-naming constant question from the other side, and that one is Clay's.

## 2026-08-14 — a survival check that walks the whole document, once per element

kq prints a flat JSON array quadratically. 646 KB of numbers takes 5.52 s
against jq's 0.02, and the gap doubles every time the array does.

    n        kq instructions      jq
    10,000    14,878,229,442      73,377,031
    20,000    12,140,556,591     121,039,691
    40,000    48,294,076,532     213,710,753
    80,000   192,602,610,421     402,225,238

Decode is clean; it is all in the print. And every counter kq owns stays
linear across those rows — allocations, appends, evacuated bytes, pushes,
shares. A gate watching any of them sees nothing.

### What it is

`k_interior_survives` answers whether a value's interior outlives a mark, and
for a container it answers by walking:

    case K_LIST:  return k_survives(l->items, m)
                      && k_slots_survive(l->items, l->len, m);

The value it is asked about is the beat's carried slot, which is a description
whose payload reaches the decoded document. So each check walks the whole
array. It is called three times per element — 60,000 at n=20,000 and 120,000
at n=40,000, exactly linear — and each call is O(n). The work per call is
counted nowhere, which is why the counters all looked healthy.

### The regime break

Below about twenty thousand elements a second quadratic dominates instead: the
accumulator is arena-allocated, the rewind evacuates it whole, and
`k_copy_size` runs two hundred million times — 20,016 per element, against
300,045 at twice the size. Crossing the threshold moves the buffer to malloc
below the mark and that cost disappears, which is why n=20,000 retires fewer
instructions than n=10,000. Two quadratics, one per regime, and the crossover
hides both from anyone reading a single pair of sizes.

### Reading the profiler

The first eight readings of the sample output ranked `k_copy_size` first. That
came from the section headed "Total number in stack (recursive counted
multiple)", where a node's count includes its children — it ranks callers above
the leaf they call, and summing across occurrences double-counts every path.
The section to read is "Sort by top of stack", which names one function.

Arithmetic would have caught it sooner: `k_copy_size` runs 600,045 times at
n=40,000, which cannot fill a multi-second profile.

### What did not find it

Eight reductions in kanso, none reproducing. Five structural hypotheses, each
killed by building and measuring: that the beat bracket itself was at fault,
that the nesting of the element step mattered, that the element type mattered,
that the container's age against the accumulator mattered, that appending and
beating in one frame mattered. The builder analysis really does recognise only
`"{acc}suffix"` interpolation and really does miss `text/append` — a genuine
narrowness, and not this bug.

None of the reductions carried a description that reached a large list, which
is the one ingredient that matters.

## 2026-08-14 — the memo, the benchmark the corpus lacked, and a threshold that measured worse

The survival check diagnosed the day before is fixed, and the fix ships behind
a benchmark built to see it.

### The fix

A beat's rewind frees only what lies above its mark. A list whose slots all
lie below the OUTERMOST mark therefore outlives every rewind that can happen,
and its verdict cannot change — which makes it the one verdict worth keeping.
`k_interior_survives` now memoises that, keyed by the list pointer, with the
generation reset whenever the outermost mark's arena pointer changes.

Two earlier keyings were both unsound and both looked like triumphs: keyed by
the mark's stack address, because slots are reused, and keyed by its arena
position, because a rewound arena returns to where it started. Each collapsed
the timings to nothing and each printed wrong output, caught only by diffing
against jq. A perf change that looks too good is a correctness question first.

Lists only. In-place list growth demands `k_born_this_beat`, which puts the
list above the mark where the scan never runs; `k_b_put_mut` grows a map in
place with no such guard, so maps are excluded.

### The numbers, on the runner

    widebench   15,730,176,374 -> 134,061,218 instructions   117x
    encodebench  9,207,299,685 -> 9,207,704,515              +0.0044%
    basket          55,817,123 -> 55,921,156                 +0.19%
    jsonbench, oneshot                                       unmoved
    .text                                                    +704 bytes, all five

Quadratic to linear, for a hash probe on two programs.

### The corpus could not see either half

All four benchmarks carry deeply nested documents whose widest array holds ten
elements, and all four render one string. A cost that grows with a container's
length is invisible to every vein they feed — which is how this reached kq's
headline row with every counter green.

`widebench` decodes a flat array of twenty thousand floats and streams it back
one element at a time through a chained io bind, the shape kq prints a
top-level array with. On the same fixture its counters and kq's agree:
`beat_iters` 20,001 against 20,002, `evac_allocs` 120,011 against 120,021,
`append_grow` 20,000 both. It joins all three linux veins, and
`bench/cost_golden_wide.txt` gets a gate of its own, having had none.

The allocator counters will not move when this cost does. A check that walks a
list allocates nothing, which is the whole reason the instruction vein exists.

### Declined by measurement: a length threshold on the memo

A hash probe costs more than scanning a short list, so the memo was gated to
lists of sixteen or more. Measured against the memo without it: encodebench
-16,232 instructions, basket +8,001, widebench +100,012. Ninety-two thousand
worse in total and spent in the worst place. The lists these benchmarks check
are simply not short, so the skip almost never fires and every check pays a
length compare to learn that. Withdrawn.

### Still open

A second quadratic below the arena-to-malloc crossover, untouched by this. At
n=10,000 the instruction count is byte-for-byte the pre-fix figure, and the
hot function differs by regime: `k_copy_size` and `k_deep_copy` below,
`k_interior_survives` above. It makes a cliff — 109 KB takes 0.98 s where
149 KB is instant.

## 2026-08-14 (later) — the second quadratic, and ten measurements of what it costs

The wide-array print had two quadratics, not one. The memo fixed the survival
check that walked the carried list once per element; this is the evacuation
that deep-copied the same list once per element.

### The arithmetic that named it

`evac_bytes` is `16 * n * n` to the digit — 4,099,840,640 at n=16,000 against
16 · 16,000² = 4,096,000,000. Sixteen bytes is a KValue, and the array is the
decoded document's items buffer.

The copy never stabilised because `k_survives` walks the arena's blocks and
answers 1 only for a pointer below the frontier inside one of them. The carry
buffer is malloc'd and in no block, so last iteration's copy answered 0 —
"must be evacuated" — and was copied again, forever.

### Tenure on survival, not on size

A value found inside the previous iteration's buffer has lived a whole lap, so
it is promoted into storage the pair never overwrites, and every later
iteration shares it. A value the loop built fresh this lap is not there and
goes to the pair as before. That is what keeps a loop building a megabyte an
iteration from accumulating megabytes — the shape behind the 83 GB runaway —
and it is why a size threshold is the wrong signal.

1.42 s to 0.00 s at n=16,000, output byte-identical, `evac_bytes` linear.

### Declined by measurement: the same idea as an arena block

Appending a KBlock at the tail of the chain would be recognised by the walk
already there, with no second question asked anywhere. Built three times.
Every time the output stayed byte-identical, four of five counter gates stayed
green and the wide benchmark stayed at 0.00 s; every time it failed
`mem_corpus_pins_native_allocator_counters` on `effect_push_shape.kso` and
killed `book_panels --write`. The third attempt measured that: **91,241,398,272
bytes of maximum resident set.**

Positional membership recognises every pointer *inside* a block, where a hash
set recognises only allocation bases. A string's data pointer then reads as
surviving and is shared rather than copied, and a write path built from a
string accumulator handed through an intermediate function is exactly the shape
that turns aliasing into unbounded growth. Recognising a region is not
recognising the objects in it, and the difference is not conservative.

### What the cost is, after ten variants

                                  jsonbench  encodebench  oneshot  basket
    tenured test in k_survives      +1.24%      +5.49%     +3.46%   +2.10%
    test after the arena walk       +1.24%      +5.49%     +3.68%   +2.06%
    wrappers always_inline          +1.24%      +5.49%     +3.46%   +2.10%
    the question split              +0.12%      +5.49%     +3.02%   +2.61%
    k_survives_x always_inline      +0.72%      +5.47%     +3.06%   +6.10%
    the window hoisted              +0.12%      +5.49%     +3.04%   +2.67%
    no tenured answer at all        +0.72%      +5.46%     +2.62%   +5.75%
    the walk collapsed              +0.10%      +5.49%     +3.44%   +2.59%
    k_born_this_beat written out    +0.69%      +5.49%     +3.70%   +6.17%

Encodebench read 5.49% in every one, including the isolation that removed the
tenured answer from the copy walk entirely and the one that removed the shape
it was asked through. Neither is the cause. Three of the nine made other rows
worse and were withdrawn.

Two worked and are in. Splitting the question — `k_survives` the pure arena
walk, `k_survives_x` the wider one, asked only by the copy machinery and
`k_born_this_beat` — took jsonbench from 1.24% to 0.10%. Collapsing each walk
to one function, with the tenure flag set where a lived-a-lap node is found and
never unset within an evacuation, left every binary 4,976 bytes smaller than
baseline.

What remains is a branch in `k_copy_alloc`, the funnel every evacuated byte
passes through, and a field in the struct that carries it. Neither can go while
tenuring exists. Whether quadratic-to-linear on wide arrays is worth five and a
half per cent on the encoder is welfare's question, not a counter's.

### The fixture moved

Twenty thousand elements to sixteen thousand. Above 16,384 the items buffer's
capacity doubles and the survivor-ratio guard begins refusing the evacuation
and keeping the region, so a fixture on that side of the line exercises none of
this — the benchmark could not fail without the fix. On the new fixture the
unfixed compiler runs 11,627,314,301 instructions against 130,337,917.

## 2026-08-14 — kq's headline row re-sat, and the gate that could not see it

kq's pin was fourteen commits behind, and the commits it was missing were the
two written because of kq. Bumping it (kq#66) and re-racing against jq 1.7.1 on
an M4 Max:

| workload | kq | jq | | work |
|---|---:|---:|---|---|
| path query, 188 KB | 2.6 ms | 4.5 ms | kq 1.7x | 1.98x less |
| path query, 1.9 MB | 11.6 ms | 24.4 ms | kq 2.1x | 1.92x less |
| full print, 188 KB | 5.0 ms | 12.2 ms | kq 2.4x | 2.94x less |
| full print, 1.9 MB | 39.8 ms | 103.4 ms | kq 2.6x | 3.07x less |

The last row read 147 ms on 2026-08-09 and was the one workload jq won. It is
now the widest of the four. Two sittings an hour apart agreed on every wall
figure to within 1.3% and on every instruction ratio exactly, which is why the
instruction column is now published: load average was 2.35 and the wall clock
alone would not have been worth much. Peak footprint 28.0 MB against 30.7.

### Every gate kq owns stayed green

Both cost goldens, the scale gate and the published-numbers stamp passed
without regeneration, across a 3.7x move in the number the README publishes.

That is correct and it is a hole. `bench/numbers_gate` exists to notice that
the compiler the numbers were measured against has left the tree — it was
written after #639 let kq claim a three-times-faster row that had become ten
times slower — and it fingerprints `bench/cost_golden.txt`. Neither quadratic
touches an allocator counter: one is a survival check that allocates nothing,
the other a copy into a buffer that already exists. So the gate answers whether
the allocations are the same and is read as whether the published numbers still
hold. Here those came apart in the favourable direction. Task #216 is the vein
that would have caught it, following `bench/instructions_golden.txt` rather
than inventing a second harness.

### #210 closed, and one thing in it is a decision

The laziness divergence is gone: #892 taught the strict index to force what it
hands back, and the program recorded in the task now answers identically on
both engines. Four shapes checked before calling it closed rather than moved.
The task's title was wrong about why the render-forces fix was declined — a
golden killed it, not a measurement, which matters because a cost-based decline
is what Clay's correctness ruling of 2026-08-12 voids.

PR #900 adds the golden #892 shipped without, watched red by putting
`return found;` back at runtime.c:5581.

What is left is that all three engines print `[<thunk>]` for a list whose
element was never demanded. The law is satisfied and the author still sees a
word about the implementation where they computed a string. Forcing inside
render terminates if the cycle guard's path is extended from records to thunk
cells — `runtime.c:3363` claims every cycle passes through a record, and a
self-naming constant is the counterexample — at the cost of
`a_constant_that_holds_itself` recording `[<cycle>]`. That is a display
decision about self-naming constants, so it is gavel 20b rather than a runtime
PR.

### The escaped buffer had one owner and it was not enough (#912)

A list that outlives the beat it is being appended in gets its storage from
`k_buf_perm`, outside the arena, so the loop's rewind cannot reach it. Exactly
one thing ever freed that storage: the growth path, when the list outgrew the
buffer. The last buffer of every escaped list was therefore held until the
process exited.

For a program with one accumulator that is one buffer, bounded, invisible. For
a loop that builds a list per iteration it is one buffer per iteration. vse is
that loop, and its peak was a straight line in the trial count — both sides
built to a binary and timed as a binary, same box, same sitting, same seed,
byte-identical table:

    trials    main               #912
    10,000      972,324,864       3,932,160
    20,000    1,939,095,552       3,883,008
    40,000    3,875,864,576       3,883,008

The fix registers the owning FIELD rather than the buffer, which is what
`k_viewreg` already does for sorted views. That choice is what makes the
growth path need no change at all: a buffer that is outgrown is followed
without a fixup, so its free stays exactly where it was, and freeing through
the field nulls it, so a field registered once per growth is freed once. The
depth is the innermost mark the owning header does not survive — the beat
whose rewind reclaims the header itself, which is when nothing can reach the
storage any more.

An earlier attempt did the opposite and removed the growth-path free on the
assumption that registration had taken ownership. Instrumenting it said
`placed=0, homeless=6` on a mem fixture: every escaped buffer registered
nowhere, live equalled peak, and nothing was freed at all. The lesson is
narrow and worth keeping — a change that moves who frees a thing must not
remove one free before the replacement is proven to fire on the same inputs.

What remains is a header older than every live mark, which finds no depth and
is left alone. That class is one buffer per accumulator, bounded by the
program text rather than by its input: basket allocates 25, the growth path
frees 11, and the 14 that remain are unchanged. vse places 24,013,471 and
leaves none. kq allocates none at all.

Two veins moved, both because the runtime gained code: the instruction counts
and the machine-code size. Neither is measurable on a darwin host, so both
came from the runner and are regenerated in the PR.

### Welfare cannot see any of that

The score is 84.51 before and after — the floor exactly. Every memory term
welfare has reads the arena: `decode_peak_bytes`, `encode_peak_bytes`,
`oneshot_peak_bytes`, `basket_peak_bytes`, `decode_arena_blocks`,
`encode_arena_blocks`. Escaped storage is malloc'd, so a real program going
from 3.88 GB to 3.88 MB registers as nothing.

`perm_live_bytes` and `perm_peak_bytes` exist in the goldens since #911 and
are not in the objective. Two questions decide what to do, and both are
arguments about the weights rather than about this change: whether a memory
term should read the process peak instead of the arena peak — the arena peak
is the number the beat design is about, and it is also the number that held
flat through a 3.88 GB leak, and a term that cannot go wrong is not a term —
and, if escaped storage becomes its own term, what satiation it takes, given
that unbounded growth is exactly the failure it would exist to catch. Filed
as task #223 rather than settled here.

### A measurement trap, recorded because it cost one

`/usr/bin/time -l kanso run .` reports the maximum resident size of the
compiler and the program together, and the compiler peaks near 40 MB. Read
that way, a program whose own peak is 3.9 MB reads as 40.8 MB — and `run`
caches the build when the source has not changed, so the figure moves
depending on whether a compile happened at all. The first numbers reported for
this fix were that mistake, and the 95x they showed was really 998x. Build to
a binary, then time the binary.

### The corpus could not hold the shape it was being asked about (#913)

Every benchmark allocates a handful of buffers outside the arena and holds
them to exit, bounded by the program text. None was bounded by its INPUT. So
the fix that stopped an accumulator leaking priced as pure cost — each
benchmark paid for the check and none could collect — and, worse, the reverse
change would have priced as a win.

escapebench builds three thousand accumulators and drops each. Before the
registry it held 24,624,000 bytes at exit and freed none; after, it holds
nothing and frees three thousand. Its gate is the only one here that can see
storage the arena never held, and its mutation removes the registration line:
watched red at exactly the pre-fix numbers, then green.

The first draft of the benchmark measured ZERO, which is how the next entry
was found. It pushed `mixed k n` rather than arithmetic.

### A model that sees less scores higher (#914)

`peak_of` summed the arena and the string chunks. Escaped accumulator storage
is a third malloc pool, in the goldens since #911 and in no term, so all four
memory terms understated what a program holds — and a program going from 3.88
GB to 3.88 MB scored exactly zero.

Only basket has any: its term goes 2,168,288 to 4,920,848, and its baseline is
rebased by the same factor so the ratio holds at 82.50025 either side. That is
the rule for a new counter applied to a widened one, and it means the
definition change costs nothing while everything after it is measured against
the fuller sum.

The number worth keeping is what the reverse showed. With the rebased baseline
in place, dropping the pool back out reports 84.58 against a floor of 84.51.
**Welfare catches falls, so it cannot catch a change that makes the objective
blinder** — a blinder objective scores higher and the gate banks it. #122
closed this hole for the string pool and it reopened for the next one, so the
comment on peak_of now says any pool added later joins the sum on the day it
gets a counter rather than on the day something large enough to notice moves
it.

### A call answers the same question a builtin does (#915)

The rule that decides whether a pushed element can hold a pointer consulted
the builtin table and nothing else. `push xs n` kept its beat bracket and
`push xs (mixed k n)` lost it, and a loop without its bracket never sweeps:
the arena holds at 1,048,576 bytes for a self-loop and reaches 33,554,432 at
four thousand rounds for the same loop written with a call.

Arithmetic is what a toy loop pushes and a call is what a real one pushes, so
the shape getting the memory model's full benefit was the shape nobody writes.

The suspicion that had to clear first was laziness — a thunked call stores an
arena pointer that a rewind frees, which would have made the refusal correct.
Measured on both variants: `thunk_allocs` and `thunk_forces` are 0 either way.
The strictness analyser had already made the call strict, so the refusal was
costing brackets for nothing.

All six counter veins are byte-identical, which is also the evidence that no
benchmark in the corpus pushes a call.

### The cheapest form of a fact is not to store it

The first shape put a set of scalar-returning names on the inference. It cost
24,212 bytes of front-end peak on lib/json — 2.96%, outside the gate's band —
and took welfare below its floor for a benefit the corpus cannot show. A
sorted `Vec` instead of a `HashSet` measured IDENTICALLY, which is the finding:
the container was never the cost, the owned names were.

`arg_ok` already holds the program and is the single entry point into these
rules, so the four analysis functions take it too and the callee is resolved by
walking the declarations. The question is asked only of a call standing where
an accumulator's element goes, so a rare walk replaced a permanent map:
819,217 to 823,379 bytes, 0.51%, inside the band with the golden untouched.

`welfare --set` refuses a fall by design — "Clay's call to make, in
conversation — not a flag's" — and that guard is what made the cheaper shape
worth finding rather than routing around.

### A cluster says what it is (#917)

The beat report said `beat: rewinds every iteration (also an unbracketed
entry)` for a loop measured rewinding once, contradicting itself in one line.

`report` drops clustered and demoted groups from the classifier's answer and
re-adds them with a hardcoded `Verdict::Beat`, whose words are "rewinds every
iteration". That is right for a demoted entry — demoting the entry is exactly
what lets the loop bracket — and wrong for a cluster, which brackets as a
unit. Instrumenting the report confirmed which path the pair took: clustered,
with the demoted set empty.

It also explains an inconsistency that had looked unaccountable. `classify`
checks `outside_tails` second and would have returned `OutsideTailCall`, so it
never said `Beat` here; the hardcoded push did, and `blockers` then truthfully
appended the contradicting parenthetical.

The report had no gate of any kind — nothing in tests, scripts or CI ran it —
so the wording could drift with nothing to say so. The new text says what a
cluster IS rather than asserting what it does, which stays accurate whether or
not a cluster entered from inside its own tail cycle behaves differently. That
remains unmeasured, as does `CarryBeat`, whose text contains the same phrase.
## 2026-08-14 (last) — a fifth shelf, and the same absence twice

`bench/deepbench` was built to answer the welfare question on #903: the corpus
had no program with the shape that made a survival memo cost 39% of vse, and
three attempts to write one moved zero instructions. The fourth works, and what
the three missed was the io bind. vse's trials fold binds an effect per element
and the lambda captures the structure it just built, which is what puts many
list nodes in front of the copy walk. Binding a plain value instead compiles,
runs, and produces `evac_allocs=0` — the effect is load-bearing rather than
incidental. It is deterministic without an environment, which the instruction
gate requires: `io/write ""` is a real description with nothing to write, so
the bind stands while the values come from arithmetic.

    without the guard   2,591,215,700 instructions
    with the guard      2,553,005,144      1.50%

Then its allocation profile turned out to answer a different question.

    shelf        evac_allocs          allocs      evac_bytes
    decode                11       7,577,414             464
    encode                19               —               —
    basket                 0               —               0
    one-shot          63,967         128,528       ~1.99 MB
    deepbench      3,619,987       2,840,006     155,647,536

It evacuates more objects than it allocates, and copies 83.5% of every byte it
allocates, while holding one arena block.

Task #149 concluded from the first four rows that "on three of the four shelves
the beat model is very close to optimal in what it retains, and a refcounting
runtime would pay count traffic on 7.5M, 16.2M and 28k allocations to improve
on almost nothing." That reading holds for those four programs and does not
generalise. Where a program builds a structure per trial and consumes it before
the next, evacuation is the dominant cost and the Perceus trade is open rather
than closed.

Two wrong conclusions this week rest on the same absence: the corpus could not
see the memo regression, and it flattered the beat model against RC. One
missing shape.

The number that would settle the second one is the live-at-boundary probe run
on this shelf, which has not been done.
