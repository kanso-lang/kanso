# Pending gavels

The single source of truth for decisions awaiting Clay — ruled so on
2026-08-23, unifying what had forked across four files. An entry is here
because it is about the language a user meets: surface, semantics,
observable behavior. Implementation details do not come here; whoever
holds the file decides them and answers for the decision in the log.

The lifecycle, restored from this file's own precedent of 2026-08-15:
an entry lives here while open; the ruling is recorded in
design/compiler-log.md, which is the history and nothing else; and the
entry leaves this file in the same commit. Everything that ever left is
one `git log -p -- design/pending-gavels.md` away.

Rules of the ledger:

- **An entry cites its search, or it is invalid.** Before filing, search
  design/compiler-log.md, design/log/compiler-log-archive.md and every
  design/*.md for the question. The entry then says what the search found
  — the ruling that partly covers it, the experiment that answers its
  premise — or states plainly that it found nothing. An entry with no
  citation line is not a pending decision; it is an unsearched one, and
  it does not go to Clay.
- **An entry carries a recommendation.** Every question below says what
  the holder of the file would do and why, so a sitting can be a yes or a
  no rather than a fresh design conversation. Where the recommendation is
  to close the question, one word does it.
- **A gaveled item carries its citation forever.** Where an entry
  survives because only part of it was ruled, the ruling's marker stays
  in the entry. This rule exists because gavel 1b's marker has now
  fallen out of this file twice — once caught and restored, once in the
  `e3052383` rewrite that made this single ledger — and each loss made a
  settled question look open. The log is append-only and cannot lose a
  fact; this file is maintained by hand and has. When the two disagree,
  the log wins.
- STATUS.md may index this file. It does not carry decision text.
- Sessions cite entries by the headings below, never by a session task
  id — a task list is private to its session and its numbers resolve
  nowhere else.
- Edits to this file ride small, promptly-merged PRs, never a feature
  branch, so the ledger cannot fork.

The residual sweep of 2026-08-25 walked the log, the archive and every
design doc for questions that were asked and never answered. What it
found is below: every remaining question asked once, with a recommendation,
so the list can be ruled in batched sittings and end. The intent is that
this is the whole of it. Six candidates the sweep turned up
were already answered by the shipped code or by a later gavel, and those
went to the log rather than here.

## Blocking — a fixture, gate, or merge is waiting

### Should the fixed-temporary pin cover both codegen tiers, or the release tier alone?

**Cited:** the live log's 2026-09-17 entries adding `codegen_instructions` and
excluding kanso's own process from it; the 2026-09-16 gavel splitting welfare
into production and development, which puts the two tiers on opposite sides of
the objective; and the archive's kanso#1512 entry, "THE 11 IS THE TEMP OBJECT'S
NAME", where nine of ten temporary names read 5,163,341,031 and `4b8c1a` read
5,163,341,042, reproduced three times.

**The question.** kanso#1513 pins the temporary object's name so the link step
stops reading a different number depending on which name the driver drew.
`scripts/gates/codegen_instructions.sh` takes the tier as `$1` and adds
`KANSO_FIXED_TEMPS=1` to all three of its `env -i` lines, so the variable is in
the DEV run's environment as well — and `src/main.rs:744-747` dispatches on the
tier (`match release { true => release_clang(...), false => dev_clang(...) }`),
so `dev_clang` never reads it. The dev row moves anyway. The pull request is
green but for the two codegen rows, which are red by design until this is
settled.

**THAT SENTENCE UNDERSTATES IT, AND THE REAL FIGURE BEARS ON THE CHOICE.** A
cost-goldens job on 2026-09-18, on this branch merged with main, reports EIGHT
veins disagreeing rather than two:

    compile instructions        entry instructions
    library instructions        start-up instructions
    interpreted run instructions   what emitting costs
    dev-tier codegen            release-tier codegen

The goldens on the branch are byte-identical to main's, so none of this is
staleness. The cause is that the pin is 24 lines of `src/main.rs` — the
COMPILER'S OWN SOURCE — so the binary changes and every layout-sensitive row
moves with it, the way the interpreted row's seven does on any relink.

**This is an argument about option 1's cost, in option 1's own terms.** The
objection already written against it is that re-basing a row on an unexplained
move is what the goldens exist to catch. Six of these eight are exactly that:
layout moves with no mechanism, on rows nobody was asking about. Option 2 does
not avoid them either — narrowing the pin still leaves `src/main.rs` changed —
so the honest statement is that the pin costs eight re-based rows whichever
tier it covers, and the choice is only about which codegen row stops drawing.
That was not visible when the options were written.

1. **Pin both tiers**, as kanso#1513 is written. The dev row is re-based to
   whatever the pinned name produces, and both rows stop drawing. The cost is
   that the dev row's new value has no mechanism behind it: the flag does not
   reach `dev_clang`, so what moved it is unexplained, and re-basing a row on
   an unexplained move is the thing the goldens exist to catch.
2. **Narrow the pin to the release tier.** `codegen_instructions.sh` sets
   `KANSO_FIXED_TEMPS` only when `$1` is the release tier. The release row stops
   drawing, the dev row keeps whatever variance it has, and nothing is re-based
   without a mechanism.
3. **Leave both unpinned** and accept an 11-instruction draw on a 5.16-billion
   row.

**THE MECHANISM, found 2026-09-18, and it was not in this entry before.**
kanso#1512 measured the effect and could not say why. A cost-goldens job on
kanso#1504 — an unrelated branch, no codegen in its diff — hit the draw on its
own, and the gate did what it is built to do: it counted the binary a second
time, declared `VERDICT (2): REPRODUCTION FAILURE`, and printed its own
per-process and per-frame diff.

    ./kanso build pkg/codegen_corpus --release
        80648421 then 80648151, -270   kanso::build    <- already excluded
    clang:probe    32265497   both readings
    clang          31732189   both readings
    clang        1617286141   both readings
    /usr/bin/ld ... -plugin LLVMgold.so ... -plugin-opt=O3
      5160407609 then 5160407598, -11
        -11  1816463 -> 1816452  llvm::StringMapImpl::LookupBucketFor(llvm::StringRef)

All eleven are in `ld`, and inside `ld` they are one function: LLVM's
`StringMap` bucket probe during the LTO link. The temporary object's name is a
string that map hashes, and a different name walks a different number of
buckets. Every clang process reproduced byte for byte, so the IR kanso emits is
identical and read identically.

**This bears on the choice in one direction.** The mechanism is an LTO
mechanism — `LLVMgold.so`, `-plugin-opt=O3`, the release link. The dev tier is
`-O0` with no `-flto` and runs no LTO plugin, so it cannot be what moves the
dev row. The dev row's move stays real and unexplained with one fewer candidate
behind it, which is an argument for 2 rather than 1: option 1 re-bases a row
whose move is now slightly less explained than it looked, not more.

**And the release golden's header is wrong on one point.** It says the children
reproduced byte for byte, measured 2026-09-16 with `--trace-children`. That
held for clang and has now been shown false of `ld`. Correcting it is separate
from this question and does not wait on it.

**THE FRAME, 2026-09-18 — and it is the thing the golden's header asked for.**
`bench/codegen_instructions_release_golden.txt` ends its analysis with what was
left after threads were pinned to one and the output path was cleared: "CI's
two readings differ by 11, all of it inside `ld` ... the container cannot
reproduce it ... The gate now diffs the two readings frame by frame when they
disagree, so the next job that sees it names the frame instead of the
magnitude."

A cost-goldens job on kanso#1504 saw it, and the gate named the frame:

    /usr/bin/ld ... -plugin LLVMgold.so ... -plugin-opt=O3
      5160407609 then 5160407598, -11
        -11  1816463 -> 1816452  llvm::StringMapImpl::LookupBucketFor(llvm::StringRef)

All three clang processes byte-identical, as they have been since jobs=1.

**WHAT THIS DOES NOT SAY.** It names where the eleven landed, not what moved
it. The header has already ruled out the two candidates this frame suggests:
the temporary names ("they agree even though the temp-file names differ between
the runs ... which rules the paths out as the term") and a fixed string in
general ("a fixed string would cost a fixed number"). A `StringMap` probe count
is a symptom that a lookup walked a different number of buckets; it does not
say why the map was in a different state.

**ONE LIMIT ON THE EVIDENCE THAT RULES NAMES OUT, offered as scope rather than
as a mechanism.** The header rules the temp-file names out with a container
pair: "two complete pipeline runs, staged and warmed exactly as the gate does
it, agree to the instruction on all four counted processes, and they agree even
though the temp-file names differ between the runs". That is the same container
pair whose `ld` reads 5,146,602,703 twice — the host the header immediately
goes on to say "cannot reproduce it". So the experiment establishes that
differing names do not destabilise `ld` on a host where `ld` is already stable.
It cannot bound what they contribute on the runner, where it is not. This does
not make names the cause and nothing here suggests they are; it says the one
experiment ruling them out was run where the effect does not occur, which is
worth knowing before the sitting treats them as excluded.

**AND IT UNSETTLES THIS ENTRY'S OWN CITATION.** The entry cites the archive's
kanso#1512, "THE 11 IS THE TEMP OBJECT'S NAME", with nine of ten names reading
5,163,341,031. The golden's header has since re-explained that figure twice:
first as LTO thread partitioning, now pinned with `-plugin-opt=jobs=1`, and
then as the state of the output path, which the gate now clears before every
build — 5,163,341,031 is the header's own "absent" reading. So the premise the
options below were written against may no longer hold, and the sitting should
settle whether it does before choosing between them.

**IT REPLICATED, ON A SECOND BRANCH, AND THE RESIDUE IS EXACTLY ELEVEN BOTH
TIMES.** A cost-goldens job on kanso#1502 — a float-rendering branch with no
codegen in its diff — hit the draw on 2026-09-18 and halted the same vein. Put
beside kanso#1504's job:

                        kanso#1504              kanso#1502
    ld, first        5,160,407,609           5,139,582,528
    ld, again        5,160,407,598           5,139,582,517
    residue                    -11                     -11
    probes, first        1,816,463               1,822,415
    probes, again        1,816,452               1,822,404
    frame          LookupBucketFor         LookupBucketFor
    clang x3            identical               identical

**A THIRD JOB DREW IT, and it was this pull request's own** — the docs-only
branch carrying this entry, which cannot touch codegen by construction:

    kanso#1537   6,824,133,291 then 6,824,133,280, -11
                 probes 1,819,373 -> 1,819,362
                 frame  llvm::StringMapImpl::LookupBucketFor

Three jobs, three branches, three different probe counts — 1,816,463,
1,822,415, 1,819,373 — and the same eleven every time. Three branches, `ld`
totals 20,825,081 apart across the first two, and the drop is ELEVEN PROBES
each time, all of it in one frame.
A residue that holds at a fixed eleven across that much movement in the
quantity it is a residue of is not noise in the ordinary sense, and it is not
proportional to anything the two jobs differ in. Whatever costs the eleven
costs the same eleven on both.

This does not name the cause, and it narrows the search in one way worth
writing down: a candidate has to explain a CONSTANT, not a variance.

**AND THE RATE IS NOT ONE IN TEN.** Option 3 below offers to accept "a known,
reproducible, one-in-ten draw", a figure that came from kanso#1512's ten
temporary names. The cost-goldens jobs run on 2026-09-18, after the output
path was cleared and threads pinned, with the reading each produced:

    kanso#1504, first job    DREW        -11
    kanso#1502               DREW        -11
    kanso#1537               DREW        -11
    kanso#1538               reproduced
    kanso#1504, second job   reproduced

**THE SAME BRANCH IS ON BOTH SIDES OF THAT LIST**, which is the useful part:
kanso#1504 drew on one job and reproduced on the next with the same head, so
this is a property of the JOB and not of any branch's diff. Three in five is
the figure as of the fifth job, and the denominator grows with every job run
today — no count written into this entry will stay true, which is why the list
is here instead of a rate. What survives the next job is the direction: the
draw is far commoner than one in ten, and every one of them halts the vein for
a change that did not cause it.

**AND THE SAME JOB SHOWS WHERE kanso's OWN PROCESS MOVES, which is the
excluded one.** The gate prints it anyway, and on kanso#1502 it moved 1,610
between the two readings, broken down:

    +1610  PROGRAM TOTALS
     +924  __memcmp_avx2_movbe                        libc
     +713  kanso::build
      -27  HashMap<String, ()>, std::hash::random::RandomState  ::insert

`RandomState` is seeded per process from the OS, so a `HashMap` keyed by
`String` probes a different sequence on every run, and comparing keys is what
`memcmp` is doing there. That is a candidate mechanism for the excluded
process's own variance and it is testable — a fixed hasher, two readings — but
it is untested, so it is written here as a lead. **It bears on the 2026-09-15
normalization ruling rather than on this entry's question**: `codegen_instructions`
already excludes this process, while `compile_instructions`, `entry_instructions`
and `library_instructions` all count it and have been reproducing to the
instruction, which is the first thing any test of this has to explain.

**Recommendation: 2.** The measurement is the release tier's — that is where the
name was shown to move the count, three times. The dev row's move is real but
unexplained, and option 1 would bank it as though it were understood. 3 keeps a
known, reproducible, one-in-ten draw in a row the objective weighs.

**What is NOT being asked.** Which tier the pin covers is the question. What it
does to the language is not; nothing here changes a program's meaning.

**CORRECTION, 2026-09-18.** This entry said the pin costs no welfare, and CI has
now measured that it does. On the tree merged with kanso#1527 the runner read
`codegen_instructions_release` at 6,833,786,335 against a golden of
6,824,133,280 — the pin costs the release row 9,653,055 instructions, and that
row is a production-side welfare term. The dev row read 596,153,756 against
596,157,624, a fall of 3,868.

REPRODUCED, 2026-09-18, on a second runner and a different tree — this branch
re-merged onto main after kanso#1531. Both rows read exactly what they read
before, 6,833,786,335 and 596,153,756, and every other vein in the job's
summary block agrees. So the cost is the pin's, not the sitting's, and the
question below is a question about what the project will pay rather than about
whether the figure is real.

The container had projected the opposite. `-save-temps=obj` read 6,811,830,244
twice there against 6,813,182,505 three times without it, which is a SAVING of
about 1.35 million; the runner reads a COST seven times that size, in the other
direction. So the pin's effect on the row's level does not travel between
hosts, and only its effect on the row's STEADINESS — the eleven — was ever
reproduced on both. Option 1 and option 2 both buy steadiness with about
9.65 million instructions of welfare on the runner. Option 3 buys the eleven
back and spends nothing.

That changes what option 3 is worth, and it is why the floor now has to be part
of the answer rather than a footnote to it. Whichever of 1 and 2 is chosen, the
pull request lowers the floor by what the pin costs, and the reason recorded is
the measurement above.

### Does the wall survive the fused operators?

**Cited:** the live log's "the wall is bind with a discarded value" (2026-09-17),
which measures the equivalence; the archive's "gavel: the fused chain
operators" (2026-08-31), which says the fused form is the only spelling in
chain position; "gavel: the July letters close" (2026-08-26), letter D, which
minted `done` and removed the wall's premise; and this file's Parked line
"dot-absorbs-`>>`: argued no", which is about the plain dot and does not reach
this question.

**The question.** `a >> b` and `a .> (_ -> b)` are the same program. 570 sites
spell it one way and 184 the other. Which spelling does the language keep?

1. **The wall goes.** `.> (_ -> ...)` is the spelling. About 570 sites are
   respelt, the book's prose moves in ch02 through ch09, and the wall's
   surface leaves src/ast.rs, check.rs, codegen.rs, eval.rs, lexer.rs and
   lib.rs. Every effect sequence a learner reads gets four characters longer
   at each step.
2. **The wall stays as the only discard spelling.** An inline
   `.> (_ -> ...)` is refused where it is written, with "write the wall". 184
   sites shorten, the book is untouched, the compiler keeps the wall and gains
   a refusal. The gap: `a .> shown` where `shown` ignores its argument stays
   legal, because a group arm cannot be refused for what it does with a
   parameter — so the rule catches every discard visible at the site and
   leaves those that are not.
3. **The wall is renamed into the family**, a fourth fused form. Costs what 1
   costs and buys only that four operators look like four operators.

**Recommendation: 2.** It keeps the short spelling for the most common effect
operation and still makes the grammar decide, which is the principle at stake;
it costs 184 sites against 570 and eight chapters. 3 pays 1's price for
appearance.

### Was the wall's simultaneous-failure merge meant to go?

**Cited:** the archive's 2026-08-24 entry measuring the wall, which records
that `>>` built both operands before running either, so two failures raised
during construction merged — `print "left {boom a}" >> print "right {boom b}"`
answering `[a b]` — "the same reasoning the parallel group uses", and which
names the collision as open: "Whether that is the rule to keep is #141, and it
collides with #105 wanting the right side lazy for an unrelated reason:
laziness would buy back Haskell's answer and lose the merge. That is the
trade, and it is Clay's."

**The finding.** It never reached this ledger, and the behaviour changed while
it sat unasked. On the binary at `cc180f2f` the third case answers `boom 1`
alone and `left ok` prints, where the entry says nothing printed in any of the
three. The chat could not find the entry that moved it.

**The question.** Two halves, and the first may answer the second.

1. Was the merge removed deliberately? If some change ruled it out and said so,
   this entry closes on a citation and the archive's #141 is settled.
2. If it went unrecorded, the trade is live and unchanged: keep eager
   construction and the merge, which tells a reader about both failures; or
   make the right side lazy, which buys Haskell's answer and loses the merge.

**Recommendation:** find the commit before ruling. This is cloud's to bisect
and it does not block the wall question, which is answered the same way
whichever way this goes: if the merge is gone, the wall is pure sugar, and if
it comes back, it comes back as a property of bind rather than of a fourth
operator, since `.>` can be made eager in its right side and a lambda's body
is the only thing deferring it.

## Open, not blocking

### Where does a golden live that pins ONE engine's answer where another refuses?

**Cited:** the differential law as this file and CLAUDE.md state it -- a feature
may land on fewer engines only if the others REJECT it with a clear diagnostic,
never silently diverge; `docs/book/ch02.html` and
`docs/book/samples/ch02/overflow.out`, which pin native's refusal at the int64
boundary; and `tests/golden.rs`'s `micro_corpus_agrees_across_engines`, which
runs every micro fixture on both engines and requires agreement.

**The question.** kanso's integers are arbitrary-precision by specification. The
interpreter implements that with `BigInt`; the native build is int64 and raises
`integer overflow (int64 native build; spec int is arbitrary precision)` past
the boundary. The refusal is pinned. The interpreter's own ANSWERS there are
not, and they cannot be: the micro corpus is the only behavioural corpus, it
runs both engines, and it requires them to agree, so no fixture in it can hold
a program native refuses.

That matters now because the interpreter's `Value::Int` holds a `BigInt` whose
clone allocates, and 610,763 of the run's 1,555,866 value clones are integers --
every one of which fits an `i64`. An inline machine integer with `BigInt` on
overflow removes those allocations, and the way it goes wrong is promoting one
step late, which prints a WRAPPED number rather than raising. Nothing in the
tree would catch that.

1. **A fixture kind that pins one engine where another refuses.** A micro
   fixture gains an optional companion recording the refusal, so the corpus
   asserts "interp says X, native refuses with Y" rather than requiring
   agreement. The cost is a second shape of golden for every reader to learn,
   and a door to divergences being pinned rather than fixed.
2. **Native gains arbitrary precision**, the two engines agree, and an ordinary
   micro golden works. The cost is a bignum in the compiled runtime, on the
   production side of the objective, for a case programs rarely reach.
3. **Leave it unpinned** and let the small-integer change rest on the
   interpreter's existing arithmetic tests. The cost is that the one defect the
   change can introduce is the one nothing watches.

**Recommendation: 1.** The divergence is already sanctioned and already
documented in the book; what is missing is a place to assert it mechanically,
and 2 spends production cost to remove a divergence the project chose. But
this is a question about what the corpus is FOR, which is Clay's rather than
the implementer's.

**What is NOT being asked.** Whether to build the small-integer change; that
is an ordinary performance question and it is unsized. Only where its fixture
lives.


### What spelling does "cyclic structures sized by data" need?

**Cited:** the archive's "block-born is the whole cohort" (2026-08-29), whose
words are *cyclic structures sized by data (a graph parsed from input, N
linked nodes from a map) gain a spelling*; the live log's build-hole gavel of
2026-09-16, which took back the two shapes that reached that purpose, on
reasoning this entry does not ask to undo; the 2026-09-09 entry building the
cohort as kanso#1359, which named birth through a call as the next widening
and left it to the implementer; and the live log's 2026-09-18 entry "birth
through a call, measured", which is the measurement STATUS.md's cohort row
has owed since 2026-09-16 and which is what raises this question rather than
answering it.

**The question.** The gavel's purpose needs a program to make N nodes, where
N comes from data, and tie them to each other. Five probes against a release
build of `30fb1abe` say that today it cannot, and they fail for five different
reasons:

1. a hole outside a `build` block is refused — so the maker cannot be a
   function;
2. a value a call returns is not block-born, so its field cannot be filled —
   this is the one that widening `born_of` would fix;
3. a hole cannot escape the block it was written in, because it must be
   filled before that block freezes — so the maker cannot be a `build` of its
   own either;
4. a lambda lexically inside the block is outside it for the hole rule, which
   closes the `map` shape the purpose actually takes;
5. `ns[0].field = ...` is a syntax error before any analysis runs, because a
   fill's target parses as a bare name — so N nodes need N names.

Two nodes named by hand still work, and `tests/golden/mem/build_cycle.kso`
pins that. What is missing is only the sizing.

**Recommendation, and it is a question about the spelling rather than a
build.** Widening `born_of` to see through a call is real, separable and worth
doing on its own terms, and it fixes exactly the second of those five. It does
not reach the purpose, so building it and calling the row closed would be
wrong. What the purpose needs is a decision about which of these to open:

- **a build block that iterates** — a form binding one name per element of a
  list, so N nodes get N births without N names in the source. This is the
  smallest change that reaches the words of the gavel, and it leaves the hole
  rule exactly as the build-hole gavel left it.
- **a hole that survives a call**, which means a function whose answer carries
  an unfilled field and a caller obliged to fill it. That is a second effect
  in the type, and a much larger language change.
- **the purpose retired**, with the gavel's sentence about data-sized cycles
  struck and the alias and the field of a born node left as what the cohort
  gained.

The holder of this file would open the first. It is the one that keeps every
rule the 2026-09-16 gavel established and adds a binder rather than an escape
hatch. But which of the three is Clay's, because the gavel's own words are
what is at stake.

### The box constructor's spelling

**Cited:** the live log's "gavel: the box is explicit, an err is a value, and
a bare err halts where it lands" (2026-09-15), which rules that a value or an
err can be boxed by hand and leaves the word unnamed; the archive's "gavel:
effects are types, and the words are the only doors" (2026-08-29), which
names the type `<t>effect` and the three eliminators `bind`, `annotate`,
`rescue`; and "gavel: the fused chain operators" (2026-08-31), which gave the
three words their chain spellings. Nothing names the introducer.

**The question.** What is the prefix word that boxes a value or an err by
hand, so that `<int>effect` can be built in pure code? It is an ordinary
one-argument function, effect-shaped in its answer and value-shaped in its
argument, and it needs a chain spelling only if a chain ever ends by boxing,
which nothing in lib does today.

**Recommendation:** `effect`, the type's own name in prefix position:
`effect 5` answers `<int>effect` holding 5, `effect (err "bad")` answers a
box holding the failure. A type spelled `<t>effect` and a constructor
spelled `effect` read as one thing, the way `err reason` builds an err. No
chain spelling until a chain wants one. Cloud builds against this unless
Clay names a different word; it does not block the build.

### How far does a binding position carry a box?

**Cited:** the live log's "a box handed to a binding parameter reaches the
dispatch, and the dispatch answers wrong" (2026-09-16), which names three
possible answers and rules none of them; the entry below it of the same date,
which measures the cheapest one; the archive's "gavel: effects are types, and
the words are the only doors" (2026-08-29), which says a box is opened by
`bind`, `annotate` and `rescue` and by nothing else, and does not say what a
box handed to an ordinary parameter does. Ten micro and runtime fixtures pin
today's answer on three engines, `a_description_reaches_a_dispatch` and
`a_plain_dot_hands_the_box_over` first among them.

**The question.** A box carried through a plain parameter is invisible to the
checker from then on. `encode_onto` handed a box directly is refused;
`elem_onto x` then `encode_onto x` inside that body is not, because nothing
says `x` holds a box. kq lost three unit tests to that shape and kanso-json
two sites, each a box arriving at a group with no arm that could match it, so
the program died at run time with a diagnostic about arguments rather than
about the box.

Refusing a box at every bare-binder position closes it and was built and
measured: one refusal across the whole tree's modules, and TEN in the corpora,
two of which are the ruled behaviour itself — a description reaching a
dispatch lands on the bare arm, and `held e` receives a box and hands it back.
So the blunt rule is not available without reversing those.

**Recommendation:** track the box through the parameter — a position bound to
a box carries a box into the body, and the existing refusal then fires at the
call inside it. That is a typing change rather than a check, it leaves every
one of the ten fixtures alone, and it is the answer the log entry that opened
this thread already called the honest one. Cloud does not build it until Clay
rules, because it changes what the checker proves about every program, and it
does not block anything in flight: every site in kanso, kq and kanso-json is
spelled today so that no box reaches a group with no arm for it.

### A byte-position scan on a string, for the escape path

**Cited:** the live log's "the per-call floors, mapped after the inlines"
(2026-09-15), whose closing paragraph measures this change and says in its own
words that it "goes to Clay with this number and is not built here" — and then
no entry was ever filed here, so it went to nobody. Searched this ledger, the
live log and `design/log/compiler-log-archive.md` for `byte-position`,
`find2_below_str` and the escape path: those two log paragraphs are the only
mentions, and the question has never been asked. Also read: kanso#1291's escape
scan, which skipped and then iterated, and kanso#1276's proven length, both of
which worked inside the view rather than removing it.

**The question.** `escape_onto` looks through a string for the three bytes JSON
escapes. It cannot look at the string: `text/find2_below` takes bytes, so
`escape_onto` builds a thirty-two-byte bytes view of the string first, every
time, and drops it unused when the string is clean, which is nearly always.
That is seventeen instructions and thirty-two arena bytes per string,
16,026,750 instructions a run.

A scratch builtin `text/find2_below_str` looks through the string's own bytes
and answers 0 for a miss, and `escape_onto` builds the view only when it hits.
Container A/B on the kanso#1437 leaves, output byte-identical on both programs:
runbench 1,823,814,374 -> 1,801,576,724, −22,237,650 (−1.2193%), the decoder
unmoved. That is more than the view's own seventeen instructions because the
element loop's beat and the view's arena bytes go with it. The patch sits in
the session scratchpad as `escape_str.patch`.

What the measurement cannot settle is the surface. A string's positions are
codepoints everywhere else in `text`, and this primitive takes a byte floor and
answers a byte offset.

**Recommendation:** add it, spelled so the byte offset is never a position. What
this call site asks is where the clean prefix ends, and the answer is consumed
as a bound for building the view rather than as an index into the string; every
`text` operation a program can reach still counts in codepoints. If that reading
is too fine a distinction, the other answer is to keep the view and close the
question — 1.22% of the run term is the price, written down, and the queue
stops re-finding it.


### Pinning `.rodata` to a fixed page, so code growth stops moving the compile rows

**Cited:** the live log's 2026-09-15 entry "the maps parse is outside all three
compile rows", whose closing paragraph measures this and says the choice "is
Clay's, and goes to him with these numbers rather than to the ledger" — where
this ledger is the only channel a waiting decision has, so it went nowhere.
Searched this ledger, the live log and `design/log/compiler-log-archive.md` for
`rodata`, `section-start` and the page-pin family: that one log paragraph is the
only mention, and the question has never been asked here. Also read: kanso#1234,
which chased the same "by layout" noise to glibc's `/proc/self/maps` parse and
was ruled with `setarch` and sorts rather than a link change; and kanso#1404,
which took the checkout path out of the rows. Neither touches section placement.

**The question.** The three compile instruction rows move when code that sits
ahead of `.rodata` in the binary grows, because every literal after it shifts.
Most of this month's pull requests carry a line in their body naming some part
of their compile-row delta as "by layout", and that term is what those words
mean.

Pinning the section to a fixed address removes the term for anything growing
ahead of it. Built with `-C link-arg=-Wl,--section-start=.rodata=0x100000` on
two sources differing by a hundred functions: `program` reads 43,471,592 on
both, identical to the instruction, where the unpinned pair differed by 5,849.

The price is the gap the linker writes. The binary grows from 4,677,120 to
5,724,880 bytes at that address — about 1 per cent — or roughly 52 KiB at
0x40000, one page above today's `.rodata`, which fails the link loudly the day
the sections ahead of it outgrow it.

What the pin cannot reach is growth *inside* `.rodata`. `src/runtime.c` and
`lib/*.kso` are `include_str!`'d into it, so a runtime or library edit shifts
every literal after them whatever the section's start — and those are the edits
behind most of the "by layout" lines. So the pin buys the code-only case and
leaves the common one alone.

**Three more measurements of the term, gathered 2026-09-17, all pointing the
same way.** They matter because the pin is priced against how large the term
is, and every reading so far puts it small.

- `scripts/gates/compile_instructions.sh`'s own header carries a seven-binary
  ladder from 2026-09-04, sources differing only in code nothing reaches. The
  anchored frame spans 1,028 across all seven, 7,632 bytes of unreachable code
  moves it 402, and the movement is not monotone in `.text`. Data-only changes
  leave the frame identical to the instruction. The header's conclusion is that
  a difference near a thousand on this row is not evidence on its own.
- kanso#1473 and kanso#1478 measured the same three rows to the instruction on
  two different binaries, which says the term is quantized rather than noisy
  and that most changes do not move it at all.
- kanso#1480 moves `compile_instructions` 140,122 and `entry_instructions`
  487,035 — two orders of magnitude above both readings above. Bisected by
  cloud on 2026-09-17: 105 added lines in the linearity analysis cost 357,
  and 74 lines REWRITING two private emitter functions cost 145,472. Both
  are unreachable from a check.

**Withdrawn: the rewrite explanation, measured and false.** This entry briefly
said the term is small for ADDITIONS and large for REWRITES, on the strength of
kanso#1480's bisection attributing 145,472 to 74 rewritten lines. kanso#1492
built the ladder that tests it — eight rewrites of `without_stats_gate`, which
`kanso check` never reaches, each a distinct binary — and the row is IDENTICAL
TO THE INSTRUCTION across all eight, with `.text` spanning 256 bytes. So
rewriting unreachable code costs nothing, and the sentence that was going to
re-weigh this entry is void.

**What the three calibrated shapes now say, and they all say small.**

    unreachable additions   ~402, span 1,028 over seven binaries   (2026-09-04)
    unreachable rewrites    0, over eight binaries                 (kanso#1492)
    a reached addition      2,733                                  (kanso#1492)

kanso#1480 reads +146,628 on CI, fifty times the largest of those. So the term
this entry prices is SMALL in every shape anybody has measured, and the 146,628
is not it — it is unexplained, and it belongs to whichever frame the profile
diff names rather than to the layout term at all.

**Recommendation, rewritten 2026-09-17 after the ladders.** Still decline the
section pin, and do not close the question with it — the pin was never the
right instrument and there is a better one to rule on.

*Why the pin is not worth its price.* Every calibrated shape of the term is
small: ~402 for an unreachable addition, 0 for an unreachable rewrite, 2,733
for a reached one. The pin removes part of a term that costs at most a few
thousand instructions on a 36-million-instruction row, and it costs a 1 per
cent larger shipped binary or a measurement build linked unlike the shipped
one. That trade was the first recommendation's reasoning and the ladders have
only strengthened it.

*And the mechanism points elsewhere.* `scripts/gates/compile_instructions.sh`'s
header names what it found when it chased this: *a binary whose data and bss
differ starts the heap at a different break. That moves how much work malloc
does to service an identical request sequence without moving a single
instruction the compiler executes* — seven readings, four distinct values,
every kanso symbol identical to the instruction and only glibc's allocator
moving. If that is also what carries the 146,628, then pinning `.rodata` does
nothing for it: the heap break is set by where `.bss` ENDS, and a fixed
`.rodata` start does not fix that.

*The instrument worth ruling on instead.* Give the measured run a heap that
starts at the same address every time, and the term goes away for additions and
rewrites alike without the shipped binary changing by a byte — which answers
the kanso#1234 objection the first recommendation leaned on, since nothing is
special-cased away from what ships. This is the 2026-09-15 rule applied
literally, in Clay's words: *you clear it out so it's identical every single
run or you do something that puts it into a persistent known initial state.*
The gate already pins ten `GLIBC_TUNABLES` for exactly this reason; where the
heap begins is the one it does not pin.

*And a second thread arrived at the same instrument, from the other end.*
kanso#1492's entry "seven silicons, one recorded block, and a reader that was
never called" chases six instructions on `interp_instructions` — 2,178,502,266
against 2,178,502,272 across two CI jobs at one source. It built a reader for
the whole 123-row CPU feature block and ruled the silicon out: identical on
both jobs. Its live candidate is stated as an argument rather than a
measurement, and it is this entry's mechanism in different words — the
interpreted run is the allocation-heavy workload, 5,313,434 allocations
against a compile's 27,397, and *where the allocator's heap starts moves with
the size of the file the loader mapped*. So a fixed heap start is the
instrument two independent chases now want, on two different veins. Whichever
one measures it first answers the other, and that raises what the reading
below is worth without changing what it is.

*What this entry needs before it is ruled, and it is cheap.* One reading of
kanso#1480's own commit pair with the heap start fixed. If the 146,628 dies
under a fixed heap start, the instrument is chosen and the ruling is a
formality; if it survives, the move is not a layout term of any kind and this
entry is not where it belongs. Either way the frame-level diff of the two
compile profiles names it, and kanso#1492 says CI uploads both as artifacts on
every run. A build and two callgrind runs, and cloud's. Nothing here should be
ruled without it.

### Raising escapebench's size, so it pins the bracket's benefit and not only its cost

**Cited:** the archive entry of 2026-09-05, "the clean run in front of the
first escape, declined four ways", whose closing paragraph measures this and
says *whether to raise its size is Clay's*. Found on 2026-09-17 by cloud's
reach fix for `tests/a_question_sent_to_clay_has_a_ledger_entry.rs`, which
reads the archive as well as the live log and went red on three sends; the
other two were answered in the log the same day and this is the one with
nowhere to land. Searched this ledger, the live log and the archive for
`escapebench`, `27.6%` and the bracket by name: that paragraph is the only
place the question is asked, and it has never been asked here.

**The question.** escapebench is small enough that the escape bracket's cost
falls inside it on every run while its benefit falls outside. The entry's
measurement: at the third block the growing accumulator's superseded buffers
exceed a block and the rewind is the only thing holding the peak down, so a
change DELETING the bracket would read as a **27.6% win with every memory
counter flat**. A benchmark that prices a mechanism's cost and none of its
benefit reports a deletion as an improvement, which is the failure the corpus
README exists to prevent, one level in.

**The price of fixing it, which is why this is Clay's and not the
implementer's.** `escape_instructions` is a welfare term. A bigger escapebench
is a slower job on every run forever, and it moves a weighted counter, so the
baseline moves with it and the history's rows before the change are not
comparable across the boundary. That is a cost paid by the whole project
against a failure mode nobody has actually triggered.

**Recommendation: raise it, and take the baseline move.** The 2026-09-05
corpus-first ruling already settled the principle for this exact shape — *"the
corpus is blind" is never a reason to lower the floor; it is a corpus defect,
and the remedy is to add the benchmark the objective could not see, baseline
it forward, and let the fix score.* A benchmark that would score a deletion as
a win is the same defect seen from the other side, and the same remedy applies:
size it so the bracket's benefit is inside, re-baseline that term in the same
change, and say in the log which way it went.

The alternative is to leave it and rely on a reader noticing, which is what
this entry is evidence does not happen — the measurement sat in the archive
for twelve days and surfaced only because a spec learned to read that file.

## Stale — the July campaign's unclosed letters (GAVELS.md, retired here)

EMPTY. Clay ruled the last five in one sitting on 2026-08-26 — C struck,
`done` minted for D, G struck on the July provenance measurement, Z
confirmed declined, AA explicit-cast only. Every letter A1–X, BB, C, D, G,
Z and AA now has a ruling in the log or the archive; the section stays as a
header so a reader looking for the campaign finds where it went.

## Parked — on the record, no action

- `<<` labels: walls cover staircases; revive on real DAG demand.
- Labeled nameless patterns: parked 2026-08-19 — needs a fresh look
  against the post-24 language, not pending. Group headers stay behind
  it.
- dot-absorbs-`>>`: argued no — erases the visible then/bind split.
- Postfix index on `)`: `(sort xs)[1]` stays illegal; bind-then-index.
- `;` inline separator: the borrow if inline groups are ever demanded.
- `&` as bitwise: orthogonal, someday.
- `serve` / processes: the executor-loop primitive; next design
  campaign — three investigations already terminate there. The July
  reification form (an err becoming an inert Failure record at the
  supervisory boundary) died with gavel 1; the campaign starts from
  the three combinators.
- Hako tag-signing and checksum policy: parked in design/hako.md until
  something is worth attacking. The lock already carries a sha.
- Monorepo hakos (several modules per repo): the path shape allows it;
  the lock-granularity decision waits for a real case.
- Survivor cap 4× block threshold: the multiplier is a judgment call;
  the principle (the dance's transient stays at threshold scale) is in
  the log.
