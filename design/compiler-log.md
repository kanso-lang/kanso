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
## 2026-09-17 — the log goes back under its cap, and the cost of the move was a guess

design/compiler-log.md stood at 95 entries and 6,060 lines against a stated cap
of forty. CLAUDE.md's rule is that the live file holds the last forty and the
rest goes to design/log/compiler-log-archive.md unedited; the archive had not
been fed since it was last trimmed, so the file everyone reads at the tail had
grown to two and a half times the size the rule allows.

Fifty-six entries move, untouched, and the live file keeps thirty-nine plus
this one. Nothing is deleted: 40 live and 1,358 archived against 95 and 1,302
before, so all 1,397 are still on disk and the archive gained exactly what the
live file lost.

**The move was deferred twice today on a cost nobody measured.** Both times the
reason given was that it would conflict with every open branch's log appends,
with twelve in flight. That was a guess, and the guess was wrong. Trial-merged
against all eleven open branches, with the same merge run against main as the
control:

    branch                    vs main   vs the archive move   delta
    claude/prune-indexed         1              1               0
    claude/beat-indexed          7              7               0
    claude/linear-groups         7              7               0
    claude/group-indices         7              7               0
    claude/value-use-index       1              1               0
    claude/analysis-once         1              1               0
    claude/declares-const        1              1               0
    claude/aliases-once          1              1               0
    claude/codegen-rows          1              1               0
    claude/slot-prefix           0              0               0
    claude/welfare-split         1              1               0

Zero on every row. Every conflict those branches have they already have against
main, from the log TAIL they re-resolve on each base merge; the archive move
takes from the HEAD of the file, which no open branch touches. The two regions
do not meet.

This is the fifth time in one afternoon that a claim rested on a number nobody
checked, and the first caught by the rule the other four bought — CLAUDE.md's
"A measurement bounds what it measured", landed an hour before this. The
control run is the whole of it: a conflict count means nothing without the
count the change is supposed to have caused it to rise from.

**And the move disarmed a spec, which the ratchet caught and this entry did
not predict.** `tests/a_question_sent_to_clay_has_a_ledger_entry.rs` reads
`design/compiler-log.md` for paragraphs that send a measured decision to Clay
and checks each has a ledger entry to go to. Both sends it was written against
are 2026-09-15 entries, and both moved. The live log now holds ZERO such
paragraphs and the archive holds eight, so the spec passes over an empty
population and `scripts/ratchet/mutations/a_send_to_clay_with_no_ledger_entry.sh`
can no longer break it:

    ratchet: 1 mutations no longer apply
      STALE specs (unit, golden, differential) — a measured decision sent to
      Clay with no ledger entry to go to

**The hazard generalises and is worth the sentence.** Archiving moves content
out from under every check that reads the live log for HISTORY rather than for
the tail. Surveyed: twenty-nine files read `design/compiler-log.md` and exactly
one, `tests/a_log_heading_is_one_line.rs`, also reads the archive. Most of the
rest read the tail — the trend gate's worsened-counter sentence, the gates'
measured-on lines, page_drift's budget — and are unaffected. The Clay-send spec
is the one that reads history, and it is the one that went quiet.

So the archive move waits on the spec reading both files, which is a change to
`tests/` and therefore cloud's. The property it pins is about the log's
history, and the log is two files now.

- **DONE** the move prepared, the counts reconciled, and the conflict cost
  measured at zero against all eleven open branches.
- **OPEN** `a_question_sent_to_clay_has_a_ledger_entry` reading the archive as
  well as the live log. Cloud's, and this move should not land before it: a
  trim that silently empties a spec's population is a coverage regression
  whatever the line count says.

**RE-RESOLVED, 2026-09-18, and the counts above were already stale when the
merge reached them.** This entry claims 56 moved, 40 live and 1,358 archived.
Read off the two branches instead of off the paragraph: main carries 147 live
and 1,302 archived, 1,449 entries in all, and this branch before the merge
carries 65 live and 1,358 archived, 1,423 in all. So main's live file had grown
to 147 against a cap of forty while the branch that fixes that sat open, and
the branch's own live file had drifted from 40 to 65 under its later merges.

The merged tree holds 40 live and 1,419 archived, 1,459 in all. That is main's
1,456 plus this branch's own three entries, which is the check worth doing:
nothing was deleted at any step, and the sum accounts for every entry on both
sides. The figures moved twice while this branch sat open, once for kanso#1501
and once for kanso#1486, which is the merge treadmill rather than anything
about the move.

**The second re-merge needed a judgement the first did not.** Main's side of
the log conflict held 112 entries this branch had already archived, and
keeping both sides would have pulled every one of them back into the live
file. Two of the 112 were not in the archive — kanso#1486's, landed after this
branch took its snapshot. So the resolution keeps exactly the main-side
entries the archive does not already hold, and drops the rest from the live
side because they are on disk in the other file. Checked rather than asserted:
no live heading appears twice, and no live heading appears in the archive.

The prediction above survived a test it could not run. It measured zero added
conflicts against eleven branches and gave the reason: the move takes from the
head of the file and open branches append to the tail, so the two regions do
not meet. Main appended to that tail, and the merge conflicted on exactly the
region every branch already conflicts on against main. Resolving it needed no
judgement about the archive — both sides kept, main's first — and the trim ran
on the result.

The four log-reading specs pass on the merged tree:
`a_question_sent_to_clay_has_a_ledger_entry`, `a_log_heading_is_one_line`,
`a_ruling_is_not_a_page_the_log_owes` and
`a_re_basing_row_stays_a_pure_regression`. The first is the one recorded below
as disarmed by the move and re-armed against both files, so it is the one that
had to be re-run here rather than assumed.

This correction is folded into the entry it corrects rather than appended
beside it. The entry has not landed, so its numbers are still its own to get
right, and a second heading would have spent the page-drift budget on an
arithmetic fix. The third entry below, which lands cloud's reach fix, earns
its own heading: it changes a spec and a ratchet mutation rather than a
number.

## 2026-09-17 — three sends the archive would have hidden, and one of them was never answered

Cloud built the reach fix for `tests/a_question_sent_to_clay_has_a_ledger_entry
.rs` — read the archive as well as the live log — and ran it against main
before landing it. It goes red on **three** sends out of 66,449 archived lines.
That is the spec doing its job the moment it could see the whole log, and the
three are this file's to answer rather than the spec's to be taught around.

**One of cloud's three is characterised wrongly, and the correction matters
because it decides whose work it is.** The `lex_word` send closes *whether it
exists is Clay's* and opens *Gavel #159 would delete this*, which cloud read as
a citation the spec cannot recognise — a gavel number where the spec looks for
a file name. It is not a citation problem. **Gavel #159 bounced.** The
2026-08-29 entry "the inline-name entry bounces the same way the digest did"
sent it out of the ledger unruled: *zero surface area — no program can tell how
the compiler stores a name. Per the same-day ruling that performance questions
with no surface are the implementer's, it leaves the ledger unruled.* So the
send's premise is stale rather than unfiled, and teaching the spec to accept
gavel numbers would have made it green over a question that no longer exists.
Recorded here so the paragraph is answered: whether the `String` exists is the
implementer's, and has been since 2026-08-29, and the 407,394 instructions the
send offers are the implementer's to spend or keep. Nothing goes to
`design/pending-gavels.md` for it, because a bounce is the state where the
ledger is the wrong place by construction. The 2026-08-30 entry "eight
changes, and what they did to gavel #159" postdates the bounce and still reads
it as live; it is wrong on that point for the same reason.

**The second is answered, and answered the ordinary way.** The compile row that
counted the binary rather than the process — 41,904,811 on this container,
split 33,586,490 in the compiler against 7,982,541 in libc — says in its own
words that it *is filed as one rather than done here*. It was filed in
`design/pending-gavels.md`, and it was
ruled: the 2026-09-15 normalization gavel, built the same night as kanso#1439,
which CLAUDE.md records and which this afternoon's sweep verified against
`scripts/gates/compile_instructions.sh` — the gate anchors below Rust's stack
guard and drops the 465,122 instructions above that frame. Filed, ruled, built.

**The third is a real send with nowhere to land, and it goes to the ledger with
this commit.** escapebench pins the escape bracket's COST on every run and its
BENEFIT on none, so a change deleting the bracket would read as a 27.6% win
with every memory counter flat. The entry says *whether to raise its size is
Clay's, and not free: `escape_instructions` is a welfare term and a bigger
benchmark is a slower job.* That is a question for Clay, filed nowhere, sitting
in the archive since before the spec existed. It is an entry in
`design/pending-gavels.md` as of this commit, under the heading "Raising
escapebench's size, so it pins the bracket's benefit and not only its cost",
with its search cited and a recommendation to raise it and take the baseline
move.

**What this says about archiving, which is the point.** The move does not
create the defect; it reveals three that were already there, and the spec could
not see any of them while it read one file. Cloud's reach fix is right and is
cloud's to land. What it needed was not an exemption list but the two
paragraphs above and one ledger entry — the answers the sends were owed.

- **DONE** all three sends answered: one bounced and recorded, one filed and
  ruled, one now in the ledger.
- **OPEN** cloud's reach fix, and with it the question of whether the spec
  should recognise a bounce at all. A send answered by a bounce has no ledger
  entry by construction, which is a third state the spec does not model.
## 2026-09-18 — cloud's reach fix, and two holes the archive move exposed under it

The entry above closes with the reach fix left open: the spec reads
design/compiler-log.md alone, so it stops checking a send the moment the trim
walks past it. It lands here, and it did not land cleanly — the move exposed
two more holes, both of which had been silently there.

**The spec reads both files, archive first.** The archive is by construction
older than everything live and the "filed by a later entry" rule reads
forward, so joining them in that order keeps the rule meaning what it meant.
They join with a blank line so no paragraph straddles the seam. The entry
above says this was done and ran against main; it was not, and the file read
one path until this commit. Recorded rather than quietly fixed, because that
entry's own subject is a claim nobody checked.

**Hole one: a bounce is a third state and the spec modelled two.** A send is
filed and open, or filed and ruled, or BOUNCED — sent out of the ledger
unruled because the question turned out to have no surface area a program
could see, which the 2026-08-29 ruling makes the implementer's. A bounce has
no ledger entry by construction, so a rule demanding the ledger's name can
never be satisfied by one. Gavel #159 is the worked example and the entry
above names it. A later paragraph now answers a send when it quotes one of the
send's own measurements AND either names the ledger or records the bounce. The
measurement is what ties the answer to the send; without it the word "bounced"
anywhere in the log would excuse everything.

**Hole two: a send measured only in per cent could never be filed by a later
entry.** `carries_a_measurement` counts a bare percentage, so `27.6%` makes a
paragraph a send. The tie-back read comma-grouped integers alone, so `mine`
came back empty and the later-entry escape could not fire — leaving such a
send satisfiable only by naming the ledger in its own paragraph, which is the
one thing a send written before the rule existed cannot go back and do. The
escapebench send is exactly that: measured at 27.6%, filed in the ledger under
its own heading by the commit above, answered by a paragraph naming the ledger
and quoting 27.6%, and still reported unanswered. The two functions accept the
same thing now.

**And the ratchet's mutation followed the paragraph out of the live file.**
It anchors on the paragraph that files the `.rodata` page pin, which the trim
moved to the archive; it went STALE rather than red, which is the failure mode
a ratchet exists to prevent. It looks in whichever of the two files holds the
anchor now, and exits non-zero naming the problem if neither does. Watched:
applied, the spec goes red; restored, green.

With all four in, the spec passes on the merged tree and the three archived
sends are answered where they stand — one bounced, one filed and ruled, one in
the ledger.
## 2026-09-18 — the release-codegen row was counting the scheduler

kanso#1487 hunted this row once, found the variance in kanso's own process,
excluded that process, and recorded that what was left — three `clang`
processes and `ld` — came back byte for byte across two readings. It drew two
faces again within the day:

    codegen_instructions_release=6838057046
    codegen_release_again=6838057035

One job, one binary, eleven apart. The gate prices every process it runs, and
that is what settled it: all three clang children byte-identical, kanso's own
process moving +325 and already excluded, and **`ld` moving −11, the whole row
delta**.

Diffing the two `ld` profiles function by function — 18,604 entries — exactly
one differed: `llvm::StringMapImpl::LookupBucketFor`, a hash probe count.

**It is parallel LTO.** `ld` splits LTO codegen across threads, callgrind
counts every thread, and how the work lands is the scheduler's to decide rather
than the input's. Four pairs of links on a container, byte-identical bitcode,
both clang children byte for byte every time:

    different output path, plugin picks:  20,565,047,254  20,565,047,243   -11
    same output path, plugin picks:       20,565,047,241  20,565,049,584 +2,343
    same output path, jobs=1:             20,574,502,681  20,574,502,681      0

The magnitude changing between pairs is what ruled out the two candidates that
looked obvious. The output path was one — my own first pair used two different
`-o` names, which could have been the whole story and was not. The pid was the
other, and the golden's header had offered it as kanso#1487's untaken lead; it
is only on the STAGING name of the runtime object, renamed to a pid-free path
before clang or ld sees it. A fixed string costs a fixed number. Eleven one
pair and 2,343 the next is a scheduler.

So the gate asks for one thread, per the 2026-09-15 rule. It is not the
default: a user's release build has no row to keep and every reason to use its
cores, and `release_clang` adds the option only when the variable is set.

`tests/the_measured_link_pins_its_thread_count.rs` watches both halves,
because the property needs the gate to ask AND the compiler to pass the ask on,
and a property split across two files is one nothing checks. Watched red both
ways: dropping the variable from one `env -i` line, and spelling the option
`--thinlto-jobs` (which is lld's; this ld rejects it).

The row's absolute value moves — single-threaded LTO partitions the work
differently, about 0.046% higher on the container — so nothing measured before
this is comparable with anything after, and CI takes the sitting.
## 2026-09-18 — the release-codegen row, round two: three clangs pinned, and an eleven left inside ld

Round one measured what `-Wl,-plugin-opt=jobs=1` bought and what it left. CI's
per-process notice is the whole reading, first count against second:

    process        first          again        delta
    clang:probe    32,265,497     32,265,497       0
    clang          31,732,189     31,732,189       0
    clang -cc1  1,617,286,141  1,617,286,141       0
    ld          5,141,367,734  5,141,367,745     +11
    kanso          89,463,216     89,463,528    +312   (excluded from the row)

The three clang processes were the drifting half of this row and they are
byte-identical now. That is the change working. The row's absolute value falls
4,177,959 (0.0612%) with it, and none of that is a saving: single-threaded LTO
partitions the same work differently and callgrind counts every thread, so
nothing measured before this change is comparable with anything measured after.

**The container reproduces the fix and cannot reproduce the residue.** Two
complete pipeline runs here — staged and warmed exactly as the gate does it,
not the direct clang invocation round one used — agree to the instruction on
all four counted processes, `ld` included at 5,146,602,703 twice. They agree
even though the temp-file names differ between them: the probe compile reads
`/tmp/kanso_pn_probe_0020122.ll` in one run and `..._0021879.ll` in the other,
and clang's object is `codegen_corpus-89a40b.o` against `...-0c759c.o`. Both
are fixed-width, both feed `ld` on its command line, and neither moves a
count. So the paths are ruled out as the term, which was the standing
hypothesis and is now a dead one.

Seven other rows moved and all seven are layout, each named here with the
value it landed on:

    compile_instructions        35,869,355 ->    35,869,250      -105
    entry_instructions         127,872,255 ->   127,872,509      +254
    library_instructions       128,010,052 ->   128,010,220      +168
    interp_instructions      2,182,303,844 -> 2,182,293,088   -10,756
    startup_instructions         3,955,899 ->     3,955,888       -11
    codegen_instructions_dev   596,161,166 ->   596,159,774    -1,392
    emit_instructions           60,197,743 ->    60,197,827       +84

Every one is under five ten-thousandths of a per cent and the signs are mixed.
src/main.rs gained a four-line `match` on an environment variable, and
src/main.rs is the compiler, so its bytes move and every row that runs the
compiler moves with them.

**Those seven are recorded and not carried, and CI has now re-read them.**
kanso#1486 landed underneath this branch between the sitting above and the
merge, taking the three check routes down 1.17%, so the values in that table
were measured against a base that no longer exists. All seven goldens carried
MAIN'S values forward and CI measured the merged tree:

    compile_instructions        35,441,049 ->     35,441,027        -22
    entry_instructions         126,348,616 ->    126,349,040       +424
    library_instructions       126,804,150 ->    126,804,425       +275
    interp_instructions      2,182,523,679 ->  2,182,576,109    +52,430
    startup_instructions         3,951,284 ->      3,951,796       +512
    emit_instructions           60,200,209 ->     60,196,725     -3,484

Six moved, all layout, mixed signs, the largest 24 parts per million.

**The two codegen rows did not move, and that is the result.**
`codegen_instructions_dev` read 596,159,774 and `codegen_instructions_release`
read 6,822,651,561 — the same two numbers this branch measured on a different
tree in a different job, agreeing to the instruction. A row that halted its own
vein with a reproduction failure two rounds ago now reproduces across jobs.
That is what `-Wl,-plugin-opt=jobs=1` bought, and it is better evidence than
the single green round, because the two readings come from trees that differ
by kanso#1486.

Welfare sits on main's floor with the codegen pair re-based rather than
scored. `entry_instructions`, `library_instructions` and
`emit_instructions` are the three that rose; nothing in this branch runs on the
entry or library corpus or writes a different line of IR, so what moved is
where the code sits rather than what it does.

**What the eleven gets instead of a guess.** The gate reported a magnitude and
nothing else, which is two more CI rounds to find a process and then a frame.
It now pairs the two readings and diffs their per-function self costs, printing
the frames that moved. Two defects in that came out of running it against two
real readings rather than reading it:

  The key cannot be the program name. A build runs clang three times, so
  `/usr/bin/clang` paired the probe compile against the driver and announced a
  532,767 disagreement between two processes that were never the same process.
  It cannot be the whole argv either, because the argv carries exactly the temp
  paths that differ by construction. The key is the argv with runs of digits
  and hex flattened: stable across runs, and still telling the three clangs
  apart.

  The percentage column is not one field. `callgrind_annotate` right-aligns it,
  so `(100.0%)` is one whitespace-separated field and `( 4.02%)` is two, and an
  awk counting fields reads the frame name off a different column depending on
  the size of the number. Every frame name came out blank. It is a regex on the
  whole line now.

Run against the container's two passes, which differ only in the excluded
kanso process, it pairs all five correctly, stays silent on the four that
agree, and names the two frames that moved in the fifth:
`__memcmp_avx2_movbe` −176 and `kanso::build` +419 on a process total of +243.
## 2026-09-18 — the section line prints .rodata, and the row that asked for it had the premise backwards

STATUS.md's normalization row carries a small item marked as not blocked on
the measurement beside it: `interp_instructions.sh` prints `.text`, `.data`
and `.bss`, where `compile_instructions.sh` "prints `.rodata` too, with a
seven-binary calibration in its header for why". One awk alternation, it said.

The calibration is in that header. The printing is not. Grep the gates for
`rodata` and the only two hits in the tree are both inside a comment — lines
156 and 163 of `compile_instructions.sh`, the calibration table itself. No
gate printed `.rodata`, and the interp gate was not behind the others: all six
gates that print a section line printed the same three sections.

So the change is nine lines rather than one, across six files, and every one
of them now reads `text|rodata|data|bss`. On a release build of the compiler
that adds a column worth 866,912 bytes beside the 2,844,578 of `.text`.

**Why the pair matters, from the table that was misread.** Seven binaries
differing only in code or data nothing reaches:

    variant           .text     row         maps     program
    baseline          2550854   42,344,081  112,580  41,878,959
    +64 KiB .bss      2550854   42,346,221  114,720  41,878,959
    +64 KiB .rodata   2550854   42,344,099  112,598  41,878,959
    +400 dead fns     2565174   42,348,044  110,341  41,879,922

`.bss` and `.rodata` are the two cases where the anchored frame comes back
identical to the instruction and `.text` is not one of them. A gate printing
`.bss` and withholding `.rodata` prints half of that pair, so a reader looking
at a moved row could see that the zero-initialised data grew and not that the
constant data did.

**The spec reads the gates off disk and pins the count at nine.** A list
written down by hand goes stale the way CLAUDE.md's counter count did, twice,
and the way this STATUS row did. It skips comment lines, which is the whole
reason the row was wrong, and it pins rather than bounds: a floor of "at least
nine" would stay green through a gate that stopped printing its sections,
which is the change it exists to catch. Watched red first, naming all nine
lines with their files.

No counter moves. The section line is a notice; the three gates that also
write a `.sections` file only `cat` it into another notice, and nothing
compares either.

The measurement half of that STATUS row — cloud's three-parts-per-billion
candidate for the interpreted row's six — is untouched here and still owed.
## 2026-09-18 — the emitter's 9.32% was two frames read as one, and the real figure is 3.82%

kanso#1478's entry closes its open thread with "substring search over IR
lines, 7,929,096 instructions inclusive, 9.32%, all of it reached from
`Backend::emit`". That number landed on main and it is wrong. It sums two
frames that do different things.

`next_match` is a `CharSearcher`. Most of its 5,806,878 from `Backend::emit`
is `.contains(char)` and `.find(char)` — single-character scans, which are
already the cheap idiom and have nothing to give. The substring cost proper is
`<&str as Pattern>::is_contained_in`: **3,260,397 inclusive, 3.82%**, over
25,374 calls.

The site did not converge either. Two candidates were instrumented on that
branch's head and counted on `kanso build bench/codegen_corpus`:

    without_stats_gate  line.contains("load i32, ptr @k_stats_on")  1,057 calls
    prune_unnamed       names[at].contains(name)                    1,463 calls

2,520 calls of 25,374. The rest is inlined into `Backend::emit` from
somewhere a source grep does not reach, and `body_calls`, `body_lines`,
`twin_calls` and `declares_context_calls` are all `crate::hash::Set` lookups
rather than searches. A release build with `RUSTFLAGS=-g` still annotated as
`???:`, because the benchmarks' hot code is clang's — from runtime.c and the
emitted IR — so a rustc flag was never going to give line information there.

**Declined at 3.82%**, with the sites unfound, against a front end whose three
compile routes have come down about 1.2% apiece this week from changes whose
sites were obvious. The run side's leads are worth more.

The correction goes here rather than into kanso#1478, because the log is
append-only and this project corrects by later entry. What the original entry
got right is that there is a flat 85 million in the emitter with one cluster
in it; what it got wrong is how big the cluster is, and 9.32% would have sent
somebody looking for two and a half times the prize that is there.
## 2026-09-18 — the 649 unreachable blocks are terminators, and 621 of them follow a call that already says noreturn

The run-program profile left one lead marked still open: "649 `unreachable`
blocks in 599 defines, roughly 3.6% of emitted lines, paid by clang and ld on
every build." Measured on `kanso build bench/runbench`, 36,085 emitted lines:

    unreachable blocks                  649    1.798% of emitted lines
    ... preceded by a k_die-family call  621
    ... standalone block tails            28
    defines                              599

**The share is 1.798%, and 3.6% is the pair.** 649 lines is 1.798%; 1,298 —
each `unreachable` with the `call void @k_die(...)` above it — is 3.597%. The
figure was right about the two lines together and the sentence reads as though
the terminators alone cost that.

**And the lead is closed rather than open.** An LLVM basic block must end in a
terminator. `k_die`, `k_die_arity`, `k_die_overload` and `k_die_destructure`
are all declared `noreturn` in the emitted preamble and carry
`__attribute__((noreturn, noinline))` in runtime.c, so the block after one of
those calls has no fall-through and `unreachable` is the terminator it is
required to have. There is nothing to delete: 621 of the 649 are mandatory,
and the other 28 are ordinary block tails after a label or a `ret`.

Emitting fewer of them means emitting fewer `k_die` sites, and every one of
those is a runtime check a program can reach — an arity mismatch, an overload
with no match, a destructure of the wrong shape, integer overflow. That is a
change to what the language checks, not a codegen saving, and it is not on the
table.

Recorded so the count stops reading like slack. What clang and ld pay for
these lines is real and it is the price of the checks, which is a different
sentence from the one the lead was written in.
## 2026-09-17 — the same analysis, built three times from the same program

Eighth in the run, and the simplest one to state: `in_place_pushes`,
`reusable_records` and `string_builders` each opened with
`let analysis = Analysis::new(program)`, and `codegen::emit_ir` calls all three
on consecutive lines. So the whole linearity analysis was built three times
over one program in one compile.

    in_place_pushes    25,468,138 instructions   1 call
    reusable_records   25,468,351                1 call
    string_builders    25,482,737                1 call
                       ----------
                       76,419,226  of a 657,731,658 build   11.6%

Each keeps its public wrapper, which builds an `Analysis` and delegates to a
`*_with` body that reads one it was handed. `for_the_emitter` builds one and
asks all three. The wrappers stay because the corpus and a dozen specs call
them one at a time, and building an analysis for one question is the right cost
when only one is asked.

`kanso build bench/runbench` falls **51,082,187 instructions, 7.77%**,
657,731,658 to 606,649,471 — the two rebuilds, and nothing else, since the
emitted IR is byte-identical at 36,085 lines.

The spec keeps the three separate calls as the oracle and compares all five
returned collections against the shared answer, over `lib/json` and over a
small program written so two of the three answer non-empty. That second part
is asserted rather than assumed: three empty answers agree with each other, and
a fixture that only agreed emptily would pass with the sharing removed, with it
wrong, or with the functions gutted. Watched red by returning
`Default::default()` for one of the three, which fails the non-emptiness
assertion and the comparison, in both tests.

`reusable_records` is left to the lib/json case. A hand-written record fixture
was tried and cost three rounds to the grammar — `rec box` is not snake_case,
`rec box\n  n int` is a needless continuation, `rec box n int` reads as a
library with statements — and bought nothing a library full of real records did
not already cover.

**REBUILT ON MAIN, 2026-09-18, and the numbers above are the original
sitting.** The branch that carried this change was the top of a stack whose
other members have all landed, so its diff against main had grown to include
their work and every row on it was stale. It is rebuilt as one cherry-pick of
ee19ee80 onto main: src/codegen.rs and src/linear.rs applied without a
conflict, and only this log entry needed resolving.

The 657,731,658 baseline and the 7.77% are against a tree that no longer
exists — kanso#1475, kanso#1476, kanso#1478 and kanso#1493 have all landed
under it since, and the largest of those took the same build down 69.64% on
its own. The saving is real and the mechanism is unchanged; the SIZE of it on
today's main is CI's to measure, and this branch carries main's rows forward
so the gate has one number to fail against rather than none.

**The safety claim reproduces on the rebuild.** `emitted_code` AGREED on the
compile sweep, so the emitted IR is byte-identical on today's main as it was
on the old base — which is the whole argument that building the analysis once
instead of three times changes nothing a program can see. 160 test binaries
pass; the one failure is `wasm_engine`, which wants a `docs/kanso.wasm` this
container has not built and CI does.

**CI'S SITTING ON THE MERGED TREE, and the emitter row is the one that
matters.** Five rows moved:

    emit_instructions       60,196,725 ->  52,115,454  -8,081,271  -13.4248%
    startup_instructions     3,951,796 ->   3,933,223     -18,573   -0.4700%
    compile_instructions    35,441,027 ->  35,441,774        +747   +0.0021%
    entry_instructions     126,349,040 -> 126,350,802      +1,762   +0.0014%
    library_instructions   126,804,425 -> 126,806,203      +1,778   +0.0014%

`emit_instructions` anchors at `codegen::emit_ir` inclusive, which is exactly
where the three constructions sat, and it falls 13.42%. The start-up row falls
0.47% because the interpreter carries the compiler's bytes and two whole
analysis constructions have left the emitter's path.

The three `kanso check` routes rise by a few hundred each, and that is layout
by construction: `kanso check` stops before codegen, so the saving in `emit_ir`
is outside those rows entirely. Falls of 8.08 million and 18,573 against rises
of 747, 1,762 and 1,778 is the shape a real saving plus a moved binary makes.

`codegen_instructions_dev`, `codegen_instructions_release`,
`interp_instructions`, `compile_allocs` and both interp memory rows AGREED
with main to the instruction.

- **DONE** one analysis.
- **OPEN** the run of eight is over. What is left in a build is flat: `memcmp`
  at 35.2M of 606.6M and a hash-and-compare cluster around it worth about 17%,
  which is name hashing and wants interning. Attributed to callers it is
  diffuse — the largest single caller of `memcmp` is 6.2M, 1.0% — so there is
  no first map to intern that pays on its own. That is the same answer the
  check side gave, now with the build side agreeing.
## 2026-09-17 — DECLARES calls sixty-two symbols, and the compiler was finding that out every time

kanso#1468's index made `kanso build bench/runbench` fall 69.64% and made the
start-up row RISE 239,427, and that entry said why: the index costs a fixed
amount to build and saves in proportion to what the program emits, so a single
`print` is where the trade is worst. Attributing it named the fixed part.

```
called_symbols                          278,812 self
  < Once::call_once_force::{{closure}}  292,701 inclusive, 1,023 calls
  < Backend::emit                        18,159 inclusive,     2 calls
```

The 1,023 calls are `declares_context_calls()` walking DECLARES' non-declare
lines. DECLARES is a `const`. The answer is the same in every process kanso has
ever run, and it is sixty-two names.

So they are written down, sorted, and asked with a binary search: six
comparisons an ask against a hash table that has to be built first.

```
kanso play startup_corpus, kanso::main inclusive
  kanso#1468       5,148,482
  written down     4,532,728      -615,754    -11.96%
```

More than the scan itself, because the table went with it — no build, no hash
per query. Against main, which does not have kanso#1468's index at all, the row
reads 4,882,857, so this lands **350,129 below the branch point** while keeping
the 69.64%. The emitted IR for runbench is byte-identical.

`the_declares_symbols_are_the_ones_declares_calls` recomputes the set from
DECLARES with the scan it replaces and asserts both directions, plus sorted,
deduped and non-empty. Watched red three ways: a symbol the list names and
DECLARES does not call (it named `k_zz_not_called`), a symbol DECLARES calls
and the list drops (it named `k_b_at`), and the sort broken — which
`binary_search` would otherwise answer wrongly and quietly.

- **DONE** the constant is a constant.
- **OPEN** the start-up golden, which falls by the 615,754 above. CI's sitting
  takes it; this host refuses the recorded toolchain.


**REBUILT ON MAIN, 2026-09-18.** The branch carrying this had been open 15.7
hours and its diff against main had grown to 689 lines of src/codegen.rs plus
four goldens, because it sat on a stack whose other members have since landed.
It is rebuilt as one cherry-pick of 785c21b8 onto main: src/codegen.rs applied
without a conflict, and only this entry needed resolving — and the resolution
had to drop 27 entries the cherry-pick re-added that main has since moved into
the archive.

The rows are main's, carried forward, and the merged sitting is CI's to take.
`emitted_code` AGREED on the compile sweep, so the emitted IR is byte-identical
on today's main: precomputing the symbol set changes what the compiler asks,
not what it writes.

**CI'S SITTING ON THE MERGED TREE.** Six rows moved:

    startup_instructions     3,951,796 ->   3,384,249    -567,547  -14.3617%
    emit_instructions       60,196,725 ->  59,636,068    -560,657   -0.9314%
    interp_instructions  2,182,576,109 -> 2,182,620,735   +44,626   +0.0020%
    entry_instructions     126,349,040 -> 126,355,774      +6,734   +0.0053%
    library_instructions   126,804,425 -> 126,810,678      +6,253   +0.0049%
    compile_instructions    35,441,027 ->  35,443,639      +2,612   +0.0074%

Start-up falls 14.36%, which is far more than the emitter's 0.93% share of the
same saving. What the saving IS has been measured on both rows; why it lands so
much harder on start-up than on `emit_ir` is not claimed here beyond the plain
reading, that start-up runs the derivation over the whole declare block before
any program does anything.

The four rises are layout: the derivation runs before `kanso check` reaches
those routes, so the saving is outside them, and rises of a few thousand
against falls of 567,547 and 560,657 is the shape a real saving plus a moved
binary makes.

`codegen_instructions_dev`, `codegen_instructions_release`, `compile_allocs`
and both interp memory rows AGREED with main to the instruction, which is the
measured half of the emitted-IR-is-identical claim.

The spec was watched red on the rebuilt tree rather than taken on trust from
the old branch. Dropping `"k_b_append_byte"` from `DECLARES_CONTEXT_CALLS`
fails `the_written_list_is_what_the_scan_finds`, which is the one of the three
that compares the written list against a scan of DECLARES itself; the other
two, which check sortedness and non-emptiness, stay green on that edit, which
is what makes the first one the load-bearing assertion. Restored: all three
green.
## 2026-09-18 — kanso#1510's rows re-measured after kanso#1509, and the release row read the golden exactly

kanso#1509 landed under this branch, so all six compile-side goldens were
carried forward at main's values and the round measured the merged tree. CI's
sitting, second reading matching the first to the instruction on all four rows
that take one:

    startup_instructions     3,933,223 ->     3,364,974  -568,249  (-14.4474%)
    emit_instructions       52,115,454 ->    51,554,407  -561,047   (-1.0765%)
    compile_instructions    35,441,774 ->    35,445,148    +3,374   (+0.0095%)
    entry_instructions     126,350,802 ->   126,358,241    +7,439   (+0.0059%)
    library_instructions   126,806,203 ->   126,813,486    +7,283   (+0.0057%)
    interp_instructions  2,182,576,109 -> 2,182,620,735   +44,626   (+0.0020%)

The two falls are what this branch is for: precomputing the DECLARES symbol
set takes about 565,000 instructions out of both routes that run the
derivation, and the two figures land within 7,202 of each other. The four
rises are layout — the derivation runs before `kanso check` reaches its work
and before the interpreted run reaches its own, so the saving is outside
those routes and what moved in them is where the code sits. Every one of the
four is under a hundredth of a per cent.

**And the release-codegen row read 6,822,651,561 — the golden, exactly.** The
previous head of this branch was red on that row alone, at +11, with the same
job's second reading landing on the golden. This round agrees with the golden
on both readings. So the +11 is intermittent and is not this branch's: a PR
that changes only `src/codegen.rs` does not move a row twice and then stop.
kanso#1512 isolates a real dependence of that row on un-normalized state — the
prior contents of the output path, worth 2,354 — and says plainly that it does
not explain this 11, which stays open.
## 2026-09-18 — ld reads what is already at -o, and the codegen row moved with it

kanso#1510's round went red on `codegen_instructions_release` alone, at +11,
with the gate's own second reading landing exactly on the golden:
6,822,651,572 and then 6,822,651,561, one binary, one corpus, one machine.
The instrument kanso#1507 built for this named the frame rather than the
magnitude: `llvm::StringMapImpl::LookupBucketFor` inside `/usr/bin/ld`, −11,
and the whole-program delta was −11 too.

A string-keyed hash probe moving with the strings put the random temp-file
name under suspicion — clang writes `/tmp/codegen_corpus-89a40b.o` with fresh
hex every run. Three names then read identically and this entry ruled the name
out. **THAT WAS WRONG AND THE ENTRY BELOW OVERTURNS IT** — the effect is
sparse, about one name in ten, and three samples could not see it. What
follows is still correct about the output path; it is the sentence about the
name that does not survive.

    object at -89a40b.o    5,163,341,031
    object at -0c759c.o    5,163,341,031
    object at -aaaaaa.o    5,163,341,031

Three different names, three identical readings. What the first pass of that
experiment showed was a defect in the experiment: the repeat run wrote its
binary to a path six characters longer than the others, so the one thing held
constant across the three "different name" runs was silently varied in the
fourth. **The output path, not the input name.**

Isolated properly — same binary, same corpus, same `ld` command every time,
varying only what was sitting at `-o`:

    output path absent          5,163,341,031   twice, to the instruction
    output path an empty file   5,163,341,036   twice, +5
    output path 100 bytes       5,163,343,385
    output path 5 MB            5,163,343,385
    output path the real binary 5,163,343,385   +2,354 over absent

Three groups, each internally identical to the instruction across repeats, and
size stops mattering once the file is non-empty. `ld` looks at what is already
there, and how much it finds costs 2,354 instructions.

**The gate was reading the third group by accident.** `stage_and_warm` wipes
the box and then warms both tiers, so the counted build always found the
warm-up's binary at `-o`. Right answer, no reason: dropping a warm-up or
reordering the two would have moved the row by 2,354 with nothing in the diff
to explain it. It now clears the output path before every build it performs,
warm and counted alike, which is the 2026-09-15 rule applied literally —
Clay's words were "you clear it out so it's identical every single run".
`tests/the_codegen_gate_clears_its_output_before_every_build.rs` pins it
structurally, and was watched red twice: once with the clear before the
counted build removed, where it names the line and how far the build sits from
the nearest clear, and once with `codegen_corpus.ll` dropped from the clear.

**Two things this does NOT do, and both matter more than what it does.**

It is a MEASUREMENT CHANGE, not a compiler saving. Nothing about the compiler
moved. The fall it produces is the gate no longer counting `ld` inspecting a
file the previous build left behind, and the floor entry says so in those
words rather than banking it as a gain.

And it does not explain kanso#1510's 11. The sizes do not match, the frame
does not match — 2,354 spread across `ld`'s file handling against 11 inside a
StringMap probe — and this gate has always been in the "existing binary" state
on both readings of a job, so the term this fixes was constant across the pair
that disagreed. What is fixed here is a real dependence on un-normalized state
that nobody had noticed; the within-job 11 is still open, and calling it
explained because a neighbouring mechanism was found is the attribution error
this log has recorded four times.

**The whole term is in `ld`, and the SIGN differs between this box and the
runner.** Running the gate's own pipeline under callgrind, per process, twice
in each mode (each mode reproduced to the instruction):

                   no clear          clear            delta
    kanso        82,061,217      82,061,004            -213   (excluded)
    clang-probe  32,178,589      32,178,589               0
    clang        31,644,251      31,644,251               0
    clang -cc1 1,617,283,971   1,617,283,971               0
    ld        5,146,605,294   5,146,192,384        -412,910

All three clang processes are byte-identical. Every instruction of the
difference is `ld`'s, which is what the isolated experiment said and this
confirms on the real inputs rather than on a hand-built object. The magnitude
is not the isolated 2,354 — the real pipeline links a different object against
a different library set, and the term is worth more there.

CI's first round on this change read `codegen_instructions_dev` DOWN 2,150 and
`codegen_instructions_release` UP 1,481,719. This box reads the release row
DOWN 412,910. **Opposite signs on the same row**, and nothing here predicts the
runner's: the two hosts differ in gcc, and `ld`'s work on an absent output
against an existing one is evidently not the same trade on both.

That does not change what the normalization is for. The point is a FIXED state,
not a smaller number, and "absent" is the only one of the three that can be
reached without depending on what ran before: an empty `touch` lands in the
middle group at +5, so "existing with content" cannot be established except by
building, which is the accident being removed. The row re-bases once, in
whichever direction the host takes it, and then stays put.

What this does mean is that the size of this term cannot be quoted from either
host as though it were a property of the change. It is quoted here as two
measurements on two machines, which is what it is.

Both codegen goldens carry the old value with the change named in the header.
CI moves them.
## 2026-09-18 — the eleven is the temp object's NAME, and five samples said it was not

The entry above rules the random temp-object name out of the release-codegen
row's 11, on three names reading identically and then five. That is wrong, and
it is wrong in the way this log keeps recording: a search for a thing being
ABSENT is worth what the search was worth, and five samples of a sparse effect
is not worth much.

Ten names, one binary, one corpus, the output path held absent every time:

    89a40b  5,163,341,031      000000  5,163,341,031
    0c759c  5,163,341,031      4b8c1a  5,163,341,042
    aaaaaa  5,163,341,031      d7e60f  5,163,341,031
    1f2e3d  5,163,341,031      2a9b53  5,163,341,031
    ffffff  5,163,341,031      6c1d84  5,163,341,031

**Nine read one number and one reads eleven more.** `4b8c1a` was then run three
more times and read 5,163,341,042 every time, with `89a40b` back at
5,163,341,031 beside it. The name decides the count, the count is decided
deterministically, and the difference is **exactly the 11** the release-codegen
row has been disagreeing with itself by.

That closes the mechanism. `clang` writes its LTO object to
`/tmp/codegen_corpus-XXXXXX.o` with fresh hex every run, `ld`'s LLVM plugin
puts that path into a `StringMap`, and about one name in ten probes one bucket
further. The frame kanso#1507's instrument named on kanso#1510 was
`llvm::StringMapImpl::LookupBucketFor`, which is that probe, and the whole-
program delta was the same 11. Two CI jobs — kanso#1510's first head and
kanso#1511, the latter touching neither codegen nor runtime.c — saw it, which
is the rate a one-in-ten effect gives over the number of rounds this row has
had.

**What the earlier entry got right stands.** The output path's prior contents
is a second, separate dependence, worth 2,354 isolated and 412,910 in the
pipeline, and clearing it is still right. What it got wrong is the sentence
saying the name is out, and that sentence reached this log, a commit message
and a pull request body before ten samples overturned it. All three are
corrected: the name is IN, it is the eleven, and the fix is a deterministic
object name rather than a cleared output.

The next step is that fix, and it is not in this change: an env var the gate
sets, in the shape of `KANSO_LTO_JOBS`, making `release_clang` write its
object to a fixed path. It is separated so that one round measures one thing —
the cleared output re-bases both codegen rows here, and a second re-base on
top of it could not be told apart.
## 2026-09-18 — gavel built: a demanded knot counts on both engines, and the oracle moved

Ruled 2026-08-24, on the archive entry "a demanded knot counts, and the oracle
moves", Clay: "it seems so obvious." The day before had found it and written
it down exactly: *the DEMANDED knot still disagrees. Native reports
`thunk_allocs=1` where the oracle reports `0`, because the oracle's `knotted`
builds its cell without touching the counter.* The gavel named which side
moves. It stood unbuilt for twenty-five days.

Reproduced first, on a release build of `bc282f04`, by flipping the arm of
`an_undemanded_knot_allocates_nothing` so the knot is read and running it
through an importing entry on both engines:

    thunk_allocs   native 1   oracle 0
    thunk_forces   native 1   oracle 1
    thunk_evals    native 1   oracle 1
    stdout         native 1   oracle 1

**The bump does not go where it first looks like it goes.** `eval_ident`
routes EVERY zero-arity constant through `knotted` — its own comment says so,
and the reason is that asking whether a constant mentions its own name reads
`a = f b` and `b = f a` as two ordinary constants and then recurses until the
process dies. So counting a cell wherever `knotted` builds one read 2 on this
fixture rather than 1: one for `demanded/x`, which is the knot, and one for
`demanded/play`, which is not. A probe printing the name at each cell is what
said so; the first patch was wrong and green-looking on the narrow assertion.

What native counts is a `k_thunk_new`, and the emitter only emits one for a
constant in `codegen::knotted_constants` — the set that reaches itself through
a chain of mentions. The oracle now filters by that same predicate, computed
once per run through a `OnceCell` on the first constant cell it builds rather
than at construction, because `kanso check` makes an `Interp` and evaluates no
constant, and that route is a weighed development term.

Two fixtures, and they are a pair:

- `tests/golden/mem/a_demanded_knot_allocates_one_cell.kso` pins the shape the
  2026-08-24 entry named as unblocked and nobody wrote — 1 alloc, 1 force, 1
  eval, 1 live at exit. Its twin still reads 0 on both engines, so the
  2026-08-23 ruling that an undemanded knot allocates nothing is untouched.
- `tests/a_demanded_knot_counts_the_same_on_both_engines.rs` runs the same
  program through the real binary both ways and asserts the whole thunk
  triple, PINNED rather than merely compared: two engines agreeing on a wrong
  number is the failure a differential assertion cannot see.

Watched red twice before it was watched green — once on the unfixed tree
(oracle 0 against native 1) and once with the bump replaced by a no-op after
the fix was in. The native arm passes in both, which is the arm that should.

**And the hole was the FIXTURE, not the comparison — which is the reverse of
what this entry said in draft.** The draft read `tests/golden.rs`, saw the mem
vein run with no `--interp`, and concluded that nothing in the tree compared
the two engines. `tests/oracle.rs:211` is what it missed:
`mem_corpus_interp_matches_the_semantic_counters` walks the same corpus,
evaluates each case on the interpreter, and asserts thunk_allocs, thunk_forces
and thunk_evals against the native goldens, leaving frees, escaped and
live_exit alone as allocator behaviour. That loop has been there the whole
time.

It stayed green because the corpus held exactly one knot and that one was
undemanded, where both engines read zero and agreed by saying nothing. Checked
rather than assumed: with the new fixture in the vein and the bump replaced by
a no-op, that loop goes red naming the file and the row, `thunk_allocs=0`
against `thunk_allocs=1`. So the ruling could have been caught by machinery
that already existed, on the day somebody wrote a three-line program.

A differential loop is worth exactly the corpus under it, and the comment in
`tests/golden.rs` now says which loop reads the other engine rather than
promising one in the future tense. STATUS.md's row for this ruling carries the
draft's claim, citing `tests/golden.rs:194` and that future-tense comment; the
row comes off with this build, and this paragraph is here so the reason it was
wrong comes off with it.

Costs, as this host can read them: `emitted_code` and `compile_cost` AGREED,
every runtime cost vein and the whole lazy tier AGREED. The eight compile rows
this container refuses are CI's, and `interp_instructions` refuses here too —
its row is the one to read off the job log, since the change adds a predicate
walk and a set lookup on the interpreted path.
## 2026-09-18 — what the demanded-knot ruling costs, on CI's own rows

kanso#1511's first round measured the price of building the 2026-08-24 gavel.
CI's sitting on the tree merged with main:

    interp_instructions  2,182,576,109 -> 2,182,638,759  +62,650  (+0.0029%)
    interp_allocs            5,313,332 ->     5,313,348      +16  (+0.0003%)
    emit_instructions       52,115,454 ->    52,119,322   +3,868  (+0.0074%)
    library_instructions   126,806,203 ->   126,807,028     +825  (+0.0007%)
    entry_instructions     126,350,802 ->   126,351,031     +229  (+0.0002%)
    compile_instructions    35,441,774 ->    35,441,736      -38  (-0.0001%)
    startup_instructions     3,933,223 ->     3,932,978     -245  (-0.0062%)

**Two of these are the change and five are layout.** The interpreted run is
the only route that evaluates a constant, so it is the only one that fires the
`OnceCell` and asks `codegen::knotted_constants`. 62,650 instructions is that
one whole-program walk plus a set lookup at every constant cell after it, and
16 allocations is the set of owned names the walk answers with.

The other five move because the binary moved. `kanso check` makes an `Interp`
and evaluates no constant, which is exactly why the predicate is computed
lazily rather than in `Interp::new` — the three check routes and `emit_ir` pay
nothing for it, and two of the five FELL. Every one of the five is under a
hundredth of a per cent.

**The price is the ruling's, and it is cheap for what it buys.** 0.0029% of an
interpreted run is what it costs for the two engines to agree about a demanded
knot's allocation, which the differential law requires and which the gavel
ruled the oracle's side of twenty-five days ago.

**And the release-codegen row read +11 again, on a branch that touches
neither codegen nor runtime.c.** The gate's own per-process breakdown settles
what moves:

    first:  kanso=81075461 clang:probe=32265497 clang=31732189
            clang=1617286141 ld=5141367745
    again:  kanso=81075205 clang:probe=32265497 clang=31732189
            clang=1617286141 ld=5141367734

All three clang processes are byte-identical between the two readings, and
`ld` alone differs, by 11. kanso's own process differs by 256 and is excluded
from the row. So the 11 lives in `ld` and in nothing else, it has now been
seen on kanso#1510 and here, and it appears on a change to the interpreter's
counting — which is as far from the linker as a change in this repository
gets. It is the measurement rather than the branch.

kanso#1512 isolates one real dependence of that row on un-normalized state and
says plainly it is not this. The breakdown above narrows what remains: whatever
the 11 is, it is inside `ld`, it is not the three clang invocations, and it is
not the output path's prior contents, because the gate re-stages between the
two readings and both counted builds therefore find the warm-up's binary at
`-o`.
## 2026-09-18 — correcting what kanso#1511 costs: the interpreted row does not resolve it, and the sixteen allocations do

The entry above reads the first round's `interp_instructions` rise of 62,650 as
"that one whole-program walk plus a set lookup at every constant cell after
it". The second round, on the tree merged after kanso#1510 landed, reads the
row the other way:

    round 1, base 2,182,576,109   ->  2,182,638,759   +62,650
    round 2, base 2,182,620,735   ->  2,182,597,360   -23,375

One change, two bases, two signs. So the walk's cost is below what this row
resolves, and the first entry's sentence attributing 62,650 to it was reading
a layout term as work.

**What reproduces is `interp_allocs`, at +16 on both rounds.** The predicate
answers with a set of owned names, built once per run, and sixteen allocations
is what that set costs on this program. That is the price of the ruling, it is
the same number against two different bases, and it is the number to quote.

The other five rows moved by between 219 and 10,522 with mixed signs, all
under a fiftieth of a per cent, on routes that evaluate no constant and
therefore never fire the `OnceCell` at all:

    compile_instructions    35,445,148 ->    35,444,548     -600
    entry_instructions     126,358,241 ->   126,359,513   +1,272
    library_instructions   126,813,486 ->   126,814,937   +1,451
    startup_instructions     3,364,974 ->     3,364,755     -219
    emit_instructions       51,554,407 ->    51,543,885  -10,522 Both codegen rows AGREED with
their goldens, and the release row read 6,822,651,561 — the golden exactly —
on a tree that changes the interpreter and nothing else.

This is the same correction shape as the rewrite family and the three
container baselines: a delta that arrived with a change was written down as
the change's cost, and a second measurement against a different base says the
row cannot see it. What a row cannot resolve, it cannot attribute.
## 2026-09-18 — the data-sized cycle is blocked by the HOLE's placement, before birth through a call is reached

STATUS.md's "Ruled, unbuilt" carries the 2026-08-29 cohort gavel — "cyclic
structures sized by data (a graph parsed from input, N linked nodes from a map)
gain a spelling" — and owes a measurement first: *whether birth through a call
resolves to one birth*. Its route says a call returning one record may resolve
to one birth, which would give the fill its uniqueness back.

Measured on a release build of main `07b96058`, nine fixtures, each one a
`build` block of a dozen lines. What compiles today:

    two constructions bound to names, mutually linked, then collected
    into a list literal                                            ok
    a node whose hole is filled with itself                        ok
    the two-node cycle with both holes                             ok

What is refused, and the rule that refuses it:

    a = fresh "a"; a.link = b        `a` is not a construction made in
    (fresh's body constructs)        this `build` block
    [(cell "a" _) (cell "b" _)]      `_` stands only where a construction's
    (n -> cell n _)                    argument goes, inside one
    if flag (cell "a" _) (cell "b" _)          -- the same refusal
    pair (cell "a" _) (cell "b" _)   admitted; refused later for never
                                     being filled

**There are two blockers and the row names only the second.** Birth not
flowing through a call is real and the diagnostic is exact. But the hole's
PLACEMENT rule bites first: `_` is admitted where the construction carrying it
is a binding's whole right-hand side, or an argument of another construction,
and nowhere else. A list literal, an `if` arm and a lambda all refuse it.

That is what stops the gavel's own example. "N linked nodes from a map" wants
a hole inside the lambda handed to `list/map`, or inside a list literal — both
refused — and N nodes cannot be N named bindings, which is the one shape that
works. So even if birth through a call were built exactly as the row's route
describes, the data-sized spelling would still not exist, because the holes
could not be written down.

The nested-construction case is the one that shows the boundary is placement
rather than dataflow: `pair (cell "a" _) (cell "b" _)` gets past the placement
rule and is refused by the fill-once rule for never being filled. The hole is
admitted there; it is the list, the arm and the lambda that are not.

**What this does NOT settle.** Whether birth through a call resolves to one
birth is still open, and this measurement does not answer it — it says the
question is not the first one. Reading `Cohort::made` shows it pushes a fresh
entry on every call rather than memoising by source position, which is
evidence that per-call-site identities would come out distinct, and evidence
is not a measurement: nothing here ran a build with birth flowing through a
call. Recorded as an argument, the way the earlier one about the allocator's
heap was.

The ledger entry the row's Owes asks for is the chat's to file, and what it
should say is that the gavel's purpose needs the placement rule widened before
the dataflow one is, not instead of it.

**One thing the fixture turned up on the way past.** A hole in a list literal
is reported twice: once by the placement rule and once by the fill-once rule
for never being filled. Both are true and the second is a consequence of the
first, so a reader gets four diagnostics for two holes. The `if` arm and the
lambda report once each. Recorded rather than fixed, because which of the two
should stay silent is a question about the diagnostics rather than the rule,
and the reason is worth writing down for whoever widens the rule.

`BuildScan::born_of` treats a list literal as birth-transparent — its doc says
so, "an element of a list or map literal whose every element is born" — so it
descends into the elements, finds `cell "a" _`, and registers a fill-once
obligation for the hole. `per_node_walk`, which decides placement, does not
carry `hole_ok` through a list literal, so it refuses the same hole. **The two
walks disagree about whether a list literal is a place a construction can
stand**, and the double-report is that disagreement showing. Neither diagnostic
is false, which is why this is left alone: suppressing the second hides a true
statement, and admitting the hole is a change to the language and Clay's. The
corpus now pins whatever the answer turns out to be:
`tests/golden/errors/a_hole_away_from_a_construction_argument` carries all
three shapes and both goldens, and was watched red twice — once with a word
changed in the message and once with the lambda's hole removed from the
program.
## 2026-09-18 — the interpreted row does not vary, and the reading that said it did came out of a stale box

STATUS.md's second standing "Ruled, unbuilt" row is the 2026-09-15
normalization ruling against `interp_instructions`, which two CI jobs read
six apart. Its Owes: *measure cloud's candidate, or replace it.* The candidate
was that six in 2.18 billion is three parts per billion, that the interpreted
run is the allocation-heavy workload, and that where the allocator's heap
starts moves with the size of the file the loader mapped.

**The first answer this branch recorded was wrong.** Four runs staged out of
`/tmp/kanso-compile-ir` read the gate's anchor at 2,648,173,504,
2,646,456,996, 2,649,443,833 and 2,646,343,385 — a spread of 3,100,448, or
0.117% — with `core::hash::sip::Hasher::write` live at 187,455,582, 6.96% of
the run. A frame diff of the extremes put the whole difference in
`eval_ident`, `type_decl`, `call_named`, `__memcmp_avx2_movbe` and
`hashbrown`'s `contains_key`, against a `dispatch` that went the other way.
That went into this branch as a property of the current tree.

It is not. Four runs of a release build of `07b96058` — the same commit the
first reading names — read the anchor at **2,231,670,466, four times, with no
`sip::Hasher::write` frame in the profile at all.** Four runs of a release
build of `14530ee9` read 2,231,485,945 four times, likewise. The row is exact
on this container and always was.

**What the box held.** `scripts/gates/library_box.sh` copies
`./target/release/kanso`; it does not build it. A worktree's target directory
holds whatever was last built in it, and the binary in the box was one from
before kanso#1449's sibling fix of 2026-09-16 — `3ee41dcf`, "the interpreter
hashed against an attacker it does not have", which moved the interpreter's
`fns`, `types`, `knots`, typeset cache and cycle-guard sets off
`std::collections`.

Built at `3ee41dcf^` and handed today's corpus and today's `lib`, that
compiler reads:

    2,649,396,935
    2,654,191,562
    2,650,473,347
    2,653,900,730

with `sip::Hasher::write` at **187,455,582, 6.95%** — the same figure to the
instruction — and a frame diff of two runs naming `eval_ident` +1,933,122,
`type_decl` +1,164,960, `call_named` +844,947, `__memcmp_avx2_movbe` +434,498,
`contains_key` +421,421 and `dispatch` −4,199. Same binary, same signature,
same spread. The provenance is not inferred from a resemblance; the SipHash
figure and all six frames reproduce.

**So the mechanism the first reading named was right about that binary and
wrong about this tree.** Probe sequences moving while the hashing itself does
not is exactly what `src/hash.rs` describes, and
`tests/the_compile_path_hashes_with_a_fixed_seed.rs` already covers
`src/eval.rs`, so no map on this tree could have produced it.

**The 2026-09-15 row is therefore not what it was filed as.** Two CI jobs read
six apart, and nothing on this container reproduces even that: eight runs
across two release builds gave two values, one per build. Six in 2.18 billion
remains unexplained, and it is a CI-side question about two runners rather
than a randomly-seeded map. What this branch can say is that the map
hypothesis is dead and the container shows no variance to chase.

**And the staging script now builds what it stages.** One line,
`cargo build --release`, ahead of the copy, which is the rule
`all_counters.sh` and `all_compile.sh` already carry and which this script was
missing. `tests/the_box_stages_a_binary_it_built.rs` pins it: the build line
exists, it precedes the copy when comments are stripped, and the path it
copies is the one the build writes. Watched red three ways — the line deleted,
the line moved after the copy, and the copy pointed at a different path.

**What it cost to not have it.** A wrong spread, a wrong mechanism and a
search for a map that does not exist, all in an entry that reached an open
pull request. Nothing on main, because the reproduction happened before the
merge — but the only reason the reproduction happened was that the number was
re-measured rather than re-read. A gate that measures a binary nobody built answers about
some other tree, and prints a plausible number doing it.
## 2026-09-17 — the eta-reduction argument re-measured on the bind's ground

The 2026-07-25 entry *"BUILT, MEASURED, DECLINED: eta-reduction is not
semantics-preserving here"* declined `(a b -> f a b)` -> `f` on the differential
law. An `err` recorded a hop for every function it passed through, the
eta-expanded lambda was a function, and removing it made native print

    born in first at welcome.kso:4
    passed through greet

where the interpreter printed only the first line. Native and the oracle
disagreeing is the one thing not permitted, and the entry says so.

The 2026-09-15 explicit-bind ruling moved that ground in its own words: the
provenance hop "now accrues at binds rather than at skipped calls". So the
argument was re-run rather than re-asserted. Three spellings of one call, each
on both engines:

    label "flan" seasonal                       passed through label
    lab "flan" seasonal   (fn lab d c = label d c)   passed through lab
    (d c -> label d c) "flan" seasonal          no hop line at all

**Both engines agree in every one.** The divergence that killed the
optimization is gone.

A claim that did not survive checking, recorded because the checking is the
point: the first draft of this entry said ch05's golden "has moved with it".
It has not. `docs/book/samples/ch05/welcome.out` has read `born in first` and
nothing else since the book landed — `git log -S"passed through"` over that
file is empty. It is the oracle's answer, and what July's change did was make
NATIVE print a line the golden never carried. Nothing about the golden moved.
What moved is the rule below, which the three fixtures measure directly rather
than inferring from a file that was never going to say.

The rule the three readings describe is simple: **a hop names the function the
err was about to enter, so a named function records one and an anonymous one
records nothing.** A lambda has no name to print.

### What that does to the optimization, and what it does not

Eta-reducing the third spelling to the first no longer makes the engines
disagree. It makes the trace GAIN a line — `passed through label` — where the
lambda spelling printed none. That is still a change to what a program prints,
so the optimization is still not trace-preserving and stays declined here.

The direction is worth noticing. July's entry already said the added line was
arguably the truer one: "the value really does pass through `greet`, so the
native line is arguably the honest one and the lambda was hiding a real hop."
That reading now applies to source the author wrote rather than to a rewrite
the emitter performed. Wrapping a call in a lambda silently drops its
provenance line, on both engines, and a reader of ch04's story would not expect
`(d c -> label d c)` and `label` to trace differently.

### The question, and whose it is

**Should a value passing into an anonymous function record a hop, and what
names it?** July said that question "belongs to a gavel rather than to an
optimization's side effects", and it still does. What has changed is that it is
now answerable on its own terms rather than through a declined optimization:
nothing is waiting on it, no engine disagrees, and the measurement is three
fixtures long.

`tests/a_hop_is_recorded_for_a_name.rs` pins all three readings and the
agreement between engines, with the fixtures in the corpus rather than in this
entry. All three were watched red first — the engine comparison pointed at a
mismatched pair, the named expectation changed, and the anonymous one pointed
at the direct spelling.

- **DONE** the July decline's ground is re-measured and the differential
  objection is gone.
- **DECLINED STILL** eta-reduction changes the trace, now by adding a line.
- **FOR THE LEDGER** what a hop means for an anonymous function. Not blocking.
## 2026-09-17 — the interpreter copies 180 MB building byte strings, and uniqueness is why it cannot stop

`interp_instructions` became a weighted term on the 2026-09-16 gavel, so the
interpreted run was profiled for the first time with a counter on it. The
largest single thing in it is not interpretation:

    402,818,345  18.49%  __memcpy_avx_unaligned_erms
    147,104,143   6.75%  <Interp>::dispatch
    144,592,299   6.64%  <Interp>::eval_ident
    117,641,168   5.40%  mi_free
    104,394,489   4.79%  _mi_theap_malloc_zero

memcpy's callers are `__rust_realloc` at 8.27% and `<Vec<u8> as Clone>::clone`
at **7.76% over 43,572 calls** — about 4,000 instructions each. Every one of
those clones comes from `<Interp>::call_builtin`.

### Two sites, and the small one was built first

`call_builtin` deep-copies a `Vec<u8>` in two places. Both were instrumented
with `Rc::strong_count` and run over `interp_corpus`.

**`utf8`, 6,606 calls.** `Rc::try_unwrap` instead of a copy takes
`interp_instructions` from 2,228,593,160 to 2,228,294,740 on this box — a fall
of **298,420, or 0.0134%**, against the 7.76% the profile suggested. The probe
says why: every one of the 6,606 calls found `strong=3`. **The branch never
fires.** What the change removed was the call to `clone`, not the copy inside
it, and `memcpy` came back 402,825,525 against 402,818,345 — unmoved. Declined.

**`append`, 36,966 calls, and this is the one that matters.** The byte builder
copies its whole accumulator before extending it:

    let mut out = (**items).clone();

Over the corpus that is **180,081,360 bytes copied** — accumulators up to 9,906
bytes, rebuilt one append at a time. The refcounts:

    strong=1    1,326    3.6%
    strong=2   11,880   32.1%
    strong=3   13,206   35.7%
    strong=5    9,228   25.0%
    strong=6    1,326    3.6%

So the same fix fails here for the same reason, only less completely: 3.6% of
the appends could extend in place, and 96.4% could not. Six and a half of the
180 megabytes.

### What the numbers actually say

**The copy is not the defect; the holders are.** A buffer partway through a
fold is held by the argument vector, by the wrapper function's environment, by
the caller's, and by the fold's own state, and the interpreter has no way to
know that all but one of those are about to go away. The compiled engine does
know — the linearity analysis proves the accumulator unique and appends extend
the builder in place, which is what the 2026-08-26 entry "byte-builder growth
is malloc-backed and a mut-grow frees its predecessor" records. **The
interpreter has no such proof and its refcounts say uniqueness is rare.**

Every stdlib entry point is a one-line wrapper — `pub fn append acc x` calling
`builtin_append acc x` — and each wrapper's environment is one of the holders.
That is a cost the wrapper's author cannot see and the profile only shows once
somebody counts.

- **DECLINED, measured** `Rc::try_unwrap` at `utf8`: 0.0134%, and the branch
  never fires on this corpus.
- **DECLINED, measured** the same at `append`: 3.6% of the copies, 6.5 MB of
  180.
- **OPEN** what would actually pay is a value that can be appended to without
  being unique — a builder or a rope — or an argument protocol that does not
  leave a copy in the wrapper's frame. That is the interpreter's value model
  rather than a patch, and it is now worth pricing: 18.49% of the weighted
  vein is memcpy, and 180 MB of it is this one builtin.


## 2026-09-17 — the eleven is `ld`, and the gate had already called it a reproduction failure

**This corrects the entry that stood here, which was mine.** It read
`codegen_instructions_release counted 6,826,827,780 against 6,826,827,769`,
concluded the row does not reproduce across jobs, and re-based the golden.
The conclusion was right about the row and wrong about where to look, and the
re-base was the one thing the gate's header forbids.

The gate prints every process in the tree and takes a second reading. Both
readings of that job, side by side:

    first   kanso=412,662,004  clang:probe=32,265,587  clang=31,705,914  clang=1,617,294,647  ld=5,145,561,632
    again   kanso=412,661,592  clang:probe=32,265,587  clang=31,705,914  clang=1,617,294,647  ld=5,145,561,621

`ld` counted **5,145,561,632 and then 5,145,561,621** on one binary in one job,
eleven apart. The three clang processes are identical to the instruction in
every reading taken today, on every branch. The row's variance is the linker's
and nothing else's.

The gate said so in the same breath and the entry walked past it:

    codegen_release_again=6826827769  first_reading=6826827780

which is case (2) in `codegen_instructions.sh`'s own header — "THE SAME BUILD
COUNTED TWO NUMBERS. That is a REPRODUCTION FAILURE. It halts this vein and is
hunted to its source — never pinned as a second value, and never recorded as a
mode." Reading the `counted X against Y` line and writing Y is exactly the move
that header exists to stop.

The cross-branch table the old entry built proves nothing either. This branch
read 6,826,827,780 on one round and 6,826,827,769 on the next, from a diff of
72 lines of markdown. A branch cannot move a row in two directions; both values
were draws from the same coin.

The golden goes back to **6,826,827,769**, here and on kanso#1495.

**What is open is eleven instructions inside `ld`**, and by the 2026-09-15 rule
it is not a curiosity to explain: a counter reads the code under test and
nothing else, and the linker is external to every change this row is asked
about. It is also 75% of the row — 5.15 billion of 6.83. Either what moves it
is found and normalized, or `ld` comes out of the sum and the header says why.

- **DONE** the wrong value withdrawn from two branches, and the variance
  localised from "some job differs" to one process and eleven instructions.
- **OPEN** those eleven. The gate already has the instrument: it takes the
  second reading. What it needs is to keep `ld`'s two profiles when they
  disagree and diff the frames.
## 2026-09-18 — a unique container is extended where it stands, and kanso#1497's decline was a prediction

kanso#1497 profiled the interpreted run and found `__memcpy_avx_unaligned_erms`
its largest single frame, with 180,081,360 bytes of it inside `append`
rebuilding an accumulator one byte at a time. It tried `Rc::try_unwrap` on
`utf8`, measured 298,420 instructions, counted `append`'s refcounts — unique on
1,326 of 36,966 calls, 3.6% — and declined the same fix for `append` on that
share. **The share was measured; the fix was not.**

Run, on `push`, `put` and `append` together, and beside a second change that
clears the argument vector before the body runs. Four release builds of main
`14530ee9`, one box, one corpus, the gate's own anchor and the profile's
largest frame:

    tree                     anchor          memcpy        delta
    main                  2,231,485,945   402,825,998         —
    args.clear() alone    2,227,528,092   402,825,747    -3,957,853
    taken() alone         2,217,291,320   390,296,335   -14,194,625
    both                  2,213,414,350   390,296,335   -18,071,595

**The two are additive and independent.** 3,957,853 + 14,194,625 is 18,152,478
against 18,071,595 measured, 80,883 apart on 2.2 billion. `taken()` takes every
byte of the memcpy saving on its own — 12,529,663 instructions, 3.11% of that
frame — and `args.clear()` moves memcpy by 251, which is refcount traffic
rather than copying. 0.81% of the interpreted row, and the corpus prints the
same bytes as main.

**The prediction was not wrong about the shape it measured.** An accumulator
threaded through a name the caller still holds is pointed at by that frame too:

    fn stack xs n
      stack (push xs n) (n - 1)

reads `interp_allocs=19,053` on both trees, byte for byte, and a 400-append
`text/append` loop of the same shape reads 13,805 on both. `Rc::try_unwrap`
cannot fire and does not. An accumulator that arrives as another call's
answer is pointed at by nothing else:

    fn stack xs n
      stack (push (push xs n) n) (n - 1)

reads 22,362 allocations copying and 21,162 extending in place, one ask per
copy avoided. `lib/list`'s own `put acc k (push (bucket acc[k]) x)` is that
second shape, which is why the corpus moved and the first two fixtures did
not. A refcount histogram taken at one instant answers for the calls it
sampled; what a fix is worth is a different question and only a run answers
it.

**The spec pins the count, not the bytes, and the exclusion is measured.**
`tests/a_unique_container_is_extended_in_place.rs` runs the real binary on the
second shape and pins `interp_allocs` at 21,162; it was watched red on main at
22,362 and green on a tree carrying `taken()` without `args.clear()`, which is
what says the spec pins the half it names. `interp_alloc_bytes` and
`interp_peak_bytes` are left out because they track the length of the path the
run was handed: the same fixture at `/tmp/chain` reads 10,524,425 and 148,058,
and at a name 34 characters longer reads 10,578,250 and 148,485, with
`interp_allocs` at 21,173 both times. A spec staging under
`std::env::temp_dir()` would pin macOS's `/var/folders/...` against Linux's
`/tmp`. That is the 2026-09-15 rule: what cannot be normalized is left out and
the exclusion is named.

**What `args.clear()` is pinned by is the vein.** It changes no allocation and
no output — only instructions — so there is no fixture that fails without it
short of `interp_instructions` itself, which is an exact golden and moves.
Shipping it beside a change that does have a fixture is the honest shape:
the table above says which half each number belongs to.

**`sort` and `concat` were built with it and taken back out.** They copy the
same way and the change is the same two lines, so they went in. Two runs of
the five-builtin build read the anchor at 2,214,199,828, which is 785,478
ABOVE the three-builtin build's 2,213,414,350, with `memcpy` at 390,296,396
against 390,296,335 — 61 apart, so neither new site fired once over the whole
corpus. The corpus sorts and concatenates lists that something else still
points at, and what the two extra call sites bought was a bigger binary. Both
reverted; the three that fire are what ships.

**Still open.** Uniqueness stays rare in the threaded-accumulator shape, and
that is the shape a fold writes. kanso#1497's remaining question is untouched:
a value that can be appended to without being unique, or an argument protocol
that does not leave a copy in the caller's frame. What has changed is the price of the
cheap half: 14,194,625 instructions, declined the day before on a share
rather than a reading.

## 2026-09-18 — the spec pinned a count the host owns, and macOS said so

`tests/a_unique_container_is_extended_in_place.rs` pinned `interp_allocs` at
21,162 on a fixture staged under `std::env::temp_dir()`. The first CI round on
the other host went red on exactly that file: macOS stages under
`/var/folders/...` where Linux stages under `/tmp`, and a run's allocations
track the length of the path it was handed.

**The exclusion was half-right and the half it got wrong was the important
one.** The file's own header excluded `interp_alloc_bytes` and
`interp_peak_bytes` for that reason, measured: the same fixture at `/tmp/chain`
reads 10,524,425 and 148,058, and at a name 34 characters longer reads
10,578,250 and 148,485. `interp_allocs` held at 21,173 across that pair, and
holding across ONE pair of Linux paths is not the same property as holding
across two operating systems. The absolute count then moved from 21,162 to
21,180 between two revisions of the spec itself, because the entry file's name
got shorter.

**A difference is what survives.** The fixture now runs the same program at 300
rounds and at 600, from entry files of the same name length in the same
directory, and pins what the second costs over the first. Every fixed
allocation — the loader, the path, the library, the entry — is identical in
both runs and cancels. Copying reads 19,201 for those rounds and extending in
place 18,001: the 1,200 copies the two builders would have made, one allocation
each. Green on the branch, red on main at 19,201, and the number is one the
host cannot move.

That is the 2026-09-15 rule applied to a spec rather than a gate: what cannot
be normalized is not measured, and the way to normalize an absolute count with
a host-sized constant inside it is to subtract the constant.

## 2026-09-18 — kanso#1515's rows on CI, and the interpreted row falls 0.649%

CI's sitting on `259daa83`:

    interp_instructions   2,182,597,360 -> 2,168,428,538  -14,168,822  (-0.649%)
    interp_allocs             5,313,348 ->     5,310,696       -2,652
    interp_peak_bytes           933,202 ->       933,182          -20
    compile_instructions     35,444,548 ->    35,443,611         -937
    entry_instructions      126,359,513 ->   126,354,834       -4,679
    library_instructions    126,814,937 ->   126,810,299       -4,638
    startup_instructions      3,364,755 ->     3,363,774         -981
    emit_instructions        51,543,885 ->    51,546,788       +2,903

Both codegen rows read their goldens exactly, which is what an interpreter-only
change should do: `kanso build`'s child tree never sees `src/eval.rs`. The five
compile-side rows are layout.

**This container read the fall at 18,071,595 and CI reads 14,168,822.** Both
are real and CI's is the one the objective takes. The two halves were measured
apart here — `args.clear()` alone 3,957,853, `taken()` alone 14,194,625, both
18,071,595, additive to within 80,883 — and that split is a property of this
box's binary rather than of the change; what travels is the sign and the
mechanism.

Welfare rises to 76.8277 from a floor of 76.82429875406118, past the
sentinel's 0.001 band, so the floor is banked at 76.82771395446468. Raising it
is arithmetic rather than a decision.

## 2026-09-18 — the interpreter worked out what a name meant on every evaluation

`eval_ident` answered "what does this name stand for" by walking a ladder, and
walked it again every time the same node was evaluated. The rungs, in order:
the environment; `fns`; three descriptor names behind a `strip_prefix`;
`types`; then `fns` and `types` a SECOND time, a sixty-name array of builtins,
and `Rc::from(name)` to build the reference it returns.

A counter on each rung, run over `bench/interp_corpus`:

      calls into eval_ident        1,056,329
      answered by the environment    724,304   68.6%
      found in fns                   175,255
      of those, constants                  1
      reaching the type probe        332,024   31.4%
      the type probe hits             11,886
      returning a reference          325,412

So roughly a third of all resolutions walked the whole ladder, probing two maps
twice each and scanning sixty names, to reach an answer that cannot change
during a run: `fns`, `types` and the builtin list are all fixed once the
program is parsed. The answer is remembered on first sight now, in a
`RefCell<Map<String, Named>>` filled lazily rather than at construction --
`kanso check` builds an `Interp` and evaluates nothing, and that route is a
weighed welfare term, which is the same reason `cycles` beside it is a
`OnceCell`.

**Measured on this container, two binaries built in one worktree and staged
into boxes of equal path length**, because the entry route's count moves with
the path it is given:

      instructions   2,007,688,216 -> 1,836,055,421   -171,632,795   -8.55%
      allocations        5,310,694 ->     4,985,431       -325,263   -6.12%
      peak bytes           933,280 ->       942,308         +9,028   +0.97%

The allocation fall is the `Rc::from` that is no longer built per reference:
325,412 of those, against a measured fall of 325,263. The 149 the two differ by
is what the table costs -- a `String` per distinct name, plus whatever the map
allocated growing to hold them. How that 149 splits between the two is not
measured here and nothing rests on it. The peak rise is the same table.
Thunk counters are byte-identical, so nothing semantic moved.

**The first shape of this cost half the win, and the reason is worth keeping.**
`Named` began with a `Desc(Desc)` arm and a `Lit(Value)` arm, which sized every
row of the table by the largest variant of two other enums; the run read
1,851,622,574 and the peak rose 16,196. Spelling the three descriptors and the
four literals as arms of their own — every remaining arm a pointer or nothing —
took the row down to a tag and a word, and the run to 1,836,055,421 with the
peak rising 9,028. Fifteen and a half million instructions for a smaller table
on a corpus whose table holds a few hundred rows: what moved is cache lines rather than work.

**A spec for this could not be written the obvious way, and finding that out
cost a build.** The intended fixture was a name meaning a declaration at one
mention and a binding at another, so that a memory consulted ahead of the
environment would answer wrong. Three attempts were refused by the checker
before the fixture ran: `push_local` rejects a binding spelled like a
declaration and a binding spelled like a builtin. The fourth attempt used a
bare-enrolled import, which IS shadowable, built the broken interpreter to
watch it fail — and it passed.

Dumping the memory's keys says why. On that program they are `sample/play`,
`print`, `sample/shade`, `sample/apply`, `sample/round` and `builtin_round`. A
bare-enrolled import is qualified before a body is evaluated, so the spelling
the memory holds is never the spelling a binding can take. The soundness rests
on that plus the two refusals, and both are now pinned:
`tests/golden/errors/a_binding_may_not_take_a_name_that_resolves_without_it`
holds the refusals, watched red by disabling the check in `push_local`, and
`tests/golden/micro/a_bare_import_is_qualified_before_it_is_evaluated` holds
the separation, with its own comment saying plainly that it does not guard the
memory's ordering, because that is what the broken build proved.

The presence counter for the change is `interp_allocs`: remove the memory and
that row moves 6.12%.

## 2026-09-18 — kanso#1516's rows on CI, and the two engines of the measurement agreed

CI's sitting on the tree merged with main:

      interpreted    2,168,428,538 -> 1,997,105,566  -171,322,972   -7.9008%
      allocations        5,310,696 ->     4,985,433      -325,263   -6.1246%
      peak bytes           933,182 ->       942,210        +9,028   +0.9675%
      compile           35,443,611 ->    35,447,843        +4,232   +0.0119%
      entry            126,354,834 ->   126,368,664       +13,830   +0.0109%
      library          126,810,299 ->   126,824,214       +13,915   +0.0110%
      start-up           3,363,774 ->     3,364,523          +749   +0.0223%
      emitting          51,546,788 ->    51,554,663        +7,875   +0.0153%

Both codegen rows read their goldens exactly and `compile_allocs` is unmoved.
The five compile-side rises are layout: src/eval.rs is the compiler, so its
bytes move and every row that runs the compiler moves with them, and none of
those five routes evaluates a name. Each by its key, with the value it landed
on: `compile_instructions` 35,447,843, `entry_instructions` 126,368,664,
`library_instructions` 126,824,214, `startup_instructions` 3,364,523 and
`emit_instructions` 51,554,663.

**THE TWO MEASUREMENTS AGREED, AND HOW CLOSELY IS THE POINT.** This container
projected a fall of 171,632,795 from two binaries built in one worktree; CI, on
a different rustc and a different glibc, reads 171,322,972. The two deltas are
309,823 apart — 0.18% of the delta. The absolute rows cannot be compared across
those hosts at all and the goldens' headers say so; what travels is the
difference, and this is the sharpest reading of that yet taken here.

The two counter rows travel further still: the container read 5,310,694 ->
4,985,431 and 933,280 -> 942,308, different absolute values on both rows and
the SAME -325,263 and +9,028. They count operations rather than a host, which
is why the instruction row's two readings could be expected to agree as closely
as they did.

`interp_peak_bytes` is the term that pays, 0.010 points. It is the table the
remembered answers live in, and it is what the other two rows were bought with.
Welfare rises to 76.87 and the floor is banked at that.



## 2026-09-18 — the same question at the call sites, and a vector cloned per tail hop

kanso#1516 gave `eval_ident` a memory of what a non-local name stands for. The
same profile showed the question asked again at two sites that change does not
reach:

      16,171,431  0.71%  type_decl'call_named'call
      15,346,817  0.67%  contains_key'eval_tail'dispatch
      15,088,508  0.66%  type_decl'eval_tail'dispatch
       2,942,810  0.13%  contains_key'dispatch'call_named
     -----------
      49,549,566  2.17%

`call_named` had a ladder of its own — `err`, the types map, the function map —
walked on every call. `group_of` inside `eval_tail` asked
`type_decl(n).is_none() && fns.contains_key(n)` about every `FnRef` callee: two
probes for one bit. Both read a second memory now, keyed the same way.

**Two memories rather than one, and the reason is the row size.** A value needs
the name as an `Rc<str>`; a call needs the overload group. An arm carrying both
is 24 bytes of payload where either alone is 16, and that would size every row
of the table by the pair — the mistake kanso#1516 made once already and paid
fifteen and a half million instructions for.

Measured against kanso#1516's head, same worktree, boxes of equal path length:

      instructions   1,836,055,421 -> 1,802,816,521   -33,238,900   -1.81%
      allocations        4,985,431 ->     4,810,435      -174,996   -3.51%
      peak bytes           942,308 ->       951,537        +9,229   +0.98%

Against main, the two changes together: **2,007,688,216 -> 1,802,816,521,
-204,871,695, -10.20%**; allocations -9.42%; peak +1.96%.

**THE ALLOCATION FALL IS NOT THE MEMORY**, and the attribution matters because
the memory is what the change is about. `dispatch_loop_inner` cloned the whole
overload VECTOR on every tail hop. The groups are `Rc<Vec<..>>` now, because
the memory has to hold one without borrowing from `self` — a group lives in a
map owned by the `Interp`, so a `&[&FnDecl]` taken out of it borrows `self` and
cannot be stored in a field of `self`. Making them shared was the lifetime's
price and the tail hop's saving: a refcount bump where there was a vector copy.
The 174,996 is that.

**The profile said 49.5 million and the change bought 33.2.** The 49.5 is what
the removed frames cost; the change also ADDS a probe at each of the three
sites, so the prediction was a ceiling rather than an estimate. What the
remaining 16.3 million is made of — the new probes, the layout the edit moved,
or both — is not separated here, and nothing below rests on it.

Start-up does not pay for the `Rc` per group. `kanso play` on the start-up
corpus reads 3,399,666 before and 3,384,980 after, a fall of 14,686 — and that
route takes the native path rather than building an `Interp`, so the fall is
layout and the per-group allocation is not in the number at all. What the
reading says is only that nothing on the start-up path got worse; the
allocation itself is priced by the interpreted row, which fell.

`group_of`'s rewrite drops two literal exclusions and keeps one. `err` was
excluded by name and is now excluded because the memory answers `Callee::Err`
for it; a type name was excluded by a `type_decl` probe and is now excluded
because the memory answers `Constructor`. `if` stays a literal: it is a
declaration AND the conditional form, and the form wins at this site.

## 2026-09-18 — kanso#1517's rows on CI, and the three check routes name their own cost

      interpreted    1,997,105,566 -> 1,963,826,350   -33,279,216   -1.6664%
      allocations        4,985,433 ->     4,810,437      -174,996   -3.5100%
      peak bytes           942,210 ->       951,438        +9,228   +0.9794%
      compile           35,447,843 ->    35,486,173       +38,330   +0.1081%
      entry            126,368,664 ->   126,498,498      +129,834   +0.1027%
      library          126,824,214 ->   126,953,661      +129,447   +0.1021%
      start-up           3,364,523 ->     3,362,788        -1,735   -0.0516%
      emitting          51,554,663 ->    51,451,897      -102,766   -0.1993%

Each key with the value it landed on: `compile_instructions` 35,486,173,
`entry_instructions` 126,498,498, `library_instructions` 126,953,661 and
`interp_peak_bytes` 951,438.

**The three check rows are WORK and not layout, and their agreement is what
says so.** They rose 0.1081%, 0.1027% and 0.1021% — three routes, three
different programs, one figure to three decimal places. A shifted binary does
not do that; it moves rows by different amounts in mixed directions, which is
what start-up and emitting did here. `kanso check` builds an `Interp`, and
`Interp::new` now wraps every function group in an `Rc` so the callee memory
can hold one without borrowing from `self`. That is an allocation per group, on
a route that constructs the interpreter and then evaluates nothing with it.

It is the cost this change pays and it is priced in the sum: 130,000
instructions on each check route against 33,279,216 off the interpreted one.

**The two hosts agreed again, and on the counters exactly.** The container
projected the interpreted fall at 33,238,900 and CI reads 33,279,216 — 40,316
apart, 0.12% of the delta. `interp_allocs` fell by 174,996 on both, the same
integer. `interp_peak_bytes` rose 9,229 here and 9,228 on the runner, one byte
apart on a row whose absolute values the two hosts do not share.

## 2026-09-18 — the frame a raise would print, built once per declaration

Two changes, measured together against main at kanso#1517: the interpreted row
falls **363,081,711 instructions, 20.14%**, from 1,802,816,521 to 1,439,734,810
on this container. Both readings are this box's and are a comparison with each
other only; the base matches what the earlier ladder predicted for this tree to
the instruction, which is the check that the two are the same measurement.

`frame_of` is the larger half. It formats a trace line and asks which package a
file belongs to — two allocations — and the interpreter called it on EVERY entry
into EVERY body. The corpus enters about half a million bodies. The answer
depends on the declaration alone: `frame_of` reads `decl.name` and `decl.file`
and nothing else, so it is a pure function of its argument and can be
remembered.

THE FRAME IS NOT ONLY A DIAGNOSTIC, AND THIS WAS WRITTEN DOWN WRONG FIRST. The
draft of this entry said the frame is "a string only read when an err is
raised", which is true of its `prefix` and false of its `hako`. `own_failure`
(`src/eval.rs:3655`) compares the failure's package against the frame's, and
`Word::Rescue if own_failure(..) => Ok(yielded)` is the foreign-only licence:
a rescue declines a failure raised in its own package and passes it through. So
the frame decides whether a rescue FIRES. Memoizing it per declaration is still
exactly right — both halves are pure in the declaration — but the change touches
error-handling semantics rather than reporting, and it deserves to be described
that way.

THE KEY IS AN ADDRESS, AND THE COMPILER IS WHAT MAKES THAT SAFE. The memory is
`Map<usize, Frame>` keyed on `decl as *const FnDecl as usize`, which is sound
only if no two declarations can ever share a key — that is, only if every
address in it outlives the interpreter. `frame_for` takes `&'a FnDecl`, where
`'a` is the lifetime of the `Program` the interpreter borrows, so a temporary
cannot be passed at all. Making that signature explicit was not cosmetic: it
failed to compile until `eval_body_of`, `eval_body_flow`, `knotted` and the
dispatch loop's `overloads` were widened to `'a` too, and each of those errors
is a place where a future edit could otherwise have handed in a borrow that
dies. The argument used to be "all four call sites happen to pass a
program-owned declaration", which is a review; it is now a type.

The smaller half is `match_one`'s two binding sites, where `name.to_string()`
became `name.as_str().to_owned()`. `Name`'s `to_string` goes through `Display`
and `core::fmt`; `str`'s is specialized in std. Same allocation, less machinery
around it.

What pins the cost: `tests/a_unique_container_is_extended_in_place.rs` reads
12,001 per extra round where it read 15,601 before, and the twelve is this
change — six body entries a round at two allocations each.

WHAT CATCHES A WRONG FRAME WAS FOUND BY BREAKING THE MEMO, AND IT IS NOT THE
FIXTURE THIS ENTRY FIRST NAMED. The draft claimed
`tests/golden/micro/an_err_has_readers.kso` covers it, on the reasoning that it
raises from two declarations and prints the name each `origin` carries. Run
against a build whose key is hard-coded to 0 — every declaration sharing one
entry — that fixture is BYTE-IDENTICAL to its golden. It exercises the readers,
not the memory.

The corpus does catch it, at `a_chain_step_names_its_channel`, and the way it
fails says why the frame matters: the broken build prints NOTHING. A collapsed
key gives every declaration the first one's package, `own_failure` then answers
wrong, a rescue declines a failure it should have handled, and the failure
leaves the program instead of its output. That is a much louder failure than a
misnamed trace line, and it is the one the semantics deserve.

The lesson is the one this log keeps relearning: a fixture that looks like it
tests the thing has to be watched failing before it can be said to. Reasoning
about which fixture covers a change is how the last four wrong claims were
made.

## 2026-09-18 — two vectors built at the size the parameter list already states

`match_params` opened `score` and `binds` with `Vec::new()` and pushed into
them, and `dispatch` calls it once per overload on every call. `score` takes
exactly one entry per parameter — the capacity is not an estimate — and a
`Vec::new()` that reaches three entries has reallocated twice getting there.

    main                              1,802,816,521
    + the frame memory                1,439,734,810   -363,081,711
    + both vectors reserved           1,410,101,998    -29,632,812

**29,632,812 instructions, 2.06%** of the tree it lands on, for two words
changed. Together with the frame memory the interpreted row falls
**392,714,523, or 21.78%** against main at kanso#1517.

Where it was found: the A/B's own callgrind output, annotated rather than
re-run. `RawVecInner::finish_grow` carried 51,556,413 instructions (3.47%) and
`RawVec::grow_one` 27,969,576 (1.88%) on the post-memory binary — 5.35% between
them, which is a vector growing one element at a time somewhere hot. This is
one of the somewheres; the pair is still worth reading for the others.

The whole golden corpus passes unchanged, which is the assertion that matters:
a capacity is not observable, so any output difference would have meant the
change was not what it looked like.

### the reserve's mechanism, isolated

The paragraph above named two rows as where the saving would come from. Both
were re-read on the binary that has it, and they are the two that moved:

    RawVecInner::finish_grow   51,556,413 -> 22,646,445   -28,909,968
    RawVec::grow_one           27,969,576 ->  9,171,576   -18,798,000
                                                          -47,707,968

The net is 29,632,812 rather than 47.7 million, and the difference is visible in
the same profile: `dispatch` rose from 144,489,885 to 154,770,762, because
`Vec::with_capacity` inlines into its caller where `grow_one` was a call. So the
reserve does not remove that work, it moves two thirds of it and pays for the
rest inline, which is the trade a reserve IS.

This is the difference between a delta and a mechanism. The saving was predicted
from two named rows before the change was written, and those two rows are the
ones that fell — that is an isolation, not a difference-in-differences. Thirty
million arriving with the change while some third row moved would have been the
weaker claim this log has been caught making before.

What is left of the shape: 31,818,021 instructions, 2.18%, still in those two
rows. The parameter vectors were one site, not the site.

### the same shape twice more, in the two argument vectors

`eval_tail` and `eval`'s call arm each opened a `Vec::new()` and pushed one
value per argument, over an `args` slice whose length is right there.

    + match_params' two vectors    1,410,101,998
    + the two argument vectors     1,392,296,124    -17,805,874

Against main at kanso#1517 the interpreted row is now down **410,520,397
instructions, 22.77%**.

Four `with_capacity` calls have paid 47,438,686 between them, which is more than
the frame memory's smaller half, and none of them changed a line of logic. That
is worth saying plainly rather than dressing up: the shape is a vector opened
empty next to a length the code already holds, and this interpreter had it in
four hot places. The remaining `finish_grow` and `grow_one` say there are more.

The golden corpus passes on both steps.

### what the reserves did to the allocation counters, including the worry that was wrong

A reserve allocates where `Vec::new()` does not, so an overload whose parameters
are all literals — binding nothing — would now take a vector it never fills, on
every dispatch attempt. That was worth checking rather than assuming, because
`interp_allocs` is a welfare term and the interpreter tries many candidates per
call.

    main                              interp_allocs 4,810,437   peak   951,504
    + frame memory, match_params      interp_allocs 3,906,737   peak   961,391
    + the two argument vectors        interp_allocs 3,879,653   peak   961,231

**930,784 fewer allocations, 19.35%.** The worry had the sign backwards: a
reserve replaces several growth allocations with one, so even where it takes a
vector that stays empty it is buying more than it spends. The argument-vector
step isolates that on its own — 27,084 fewer allocations for two
`with_capacity` calls and nothing else — so the shape reduces the counter rather
than merely being swamped by the memory beside it. What is NOT isolated here is
`match_params`' reserve alone, which shares a binary with the frame memory; the
argument step is the evidence for the shape.

THE PEAK ROSE 9,727 BYTES, 1.02%, and that is the frame memory rather than the
reserves: one `Frame` per declaration the run enters, held for the run. It is a
term the objective weighs and it is being paid for knowingly — 392 million
instructions and 930,784 allocations against ten kilobytes held.

## 2026-09-18 — kanso#1518, CI's rows: the interpreted run falls 20.77%

    interp_instructions   1,963,826,350 -> 1,555,890,579   -407,935,771  -20.77%
    interp_allocs             4,810,437 ->     3,879,653       -930,784  -19.35%
    interp_peak_bytes           951,438 ->       961,165         +9,727   +1.02%

TWO THINGS IN THIS SITTING ARE WORTH MORE THAN THE FALL.

**The allocation row is EXACTLY what this container read.** Not close — the same
integer, 3,879,653, on two machines with different glibc and different rustc
whose absolute instruction rows cannot be compared at all. That is the third
time this has held: kanso#1516 and kanso#1517 each had their allocation deltas
agree to the unit across the same two hosts. A counter of operations travels
between machines where a counter of instructions only nearly does, and "nearly"
is measurable here — the container projected the instruction fall at 410,520,397
and the runner read 407,935,771, 0.63% of the delta apart.

**Every layout row read its golden EXACTLY, and that was not the expectation.**
`compile_instructions`, `entry_instructions`, `library_instructions`,
`startup_instructions` and `emit_instructions` all agreed on a branch that
rewrites a large part of `src/eval.rs` — a new table on `Interp`, four
signatures widened to `'a`, a memoized function and four `with_capacity` calls.
The standing prior in CLAUDE.md is that `compile_instructions` USUALLY moves on
an edit to the compiler's own Rust, because src/eval.rs is the compiler and its
bytes move. It did not move here, and neither did the other four.

That is a data point for the prior rather than against it — the paragraph
already allows it, on the strength of a two-line float-rendering edit that left
the row byte-identical at kanso#1285. This is a much larger edit doing the same
thing, so the size of a diff is not what predicts the layout rows. What predicts
them is not known, and this entry does not guess.

The floor is banked after these rows, never before.

## 2026-09-18 — the floor sentinel is read by three jobs, and a pin was named in prose

Two corrections to CLAUDE.md, both of the same family: a fact written down in a
place that could not be checked against the thing it described.

THE FLOOR. A re-merge round one leaves the layout goldens carrying main's values
and the floor unbanked, and the practice written down for it said the board goes
red on the layout gates "plus the floor sentinel" — one job. It is three.
`tests/the_digest_is_priced_on_both_sides.rs` runs in `specs` and on the macOS
host as well, and its `the_undoctored_goldens_hold_the_floor` reads the same
number the `cost goldens` job's welfare step reads. So two jobs whose titles say
nothing about welfare go red for it, and this morning they were read as a second
unrelated fault and chased before the failing target's name was looked at. The
name was the whole answer, and it was one line down in the log. The rule added
is to read the failing TARGET rather than the job title.

Watched rather than argued: with the goldens stashed back to the values CI had
tested, the spec panics `welfare 76.88   floor 76.87 ... a rise nobody
ratchets`, and it passes with the floor banked.

THE PIN. The kq bullet carried "kq's pin sits at kanso#1120 with 59 commits
behind it", read off the repo on 2026-08-31. kq#108 moved it 114 commits on
2026-09-14 and the sentence stayed. Worse, a session whose repository scope is
kanso alone cannot check that sentence at all, so it reads as current. The
specific figure is gone and the bullet now says to read the pin from kq or to
say it could not be read. What the bullet is for — the five veins, and the fact
that the instructions vein moves where the allocation counters do not — does not
go stale and is what remains.

## 2026-09-18 — a blocking question sat in a task list for four hours

kanso#1513 was opened at 06:19Z with two codegen rows red by design, waiting on
a decision. The decision was written down — as a session task marked CLAY'S CALL
/ BLOCKING — and the session then worked other threads, which is what the rules
say to do. The part that was skipped is the part that matters: it never reached
`design/pending-gavels.md`, and that file is the only place Clay reads pending
decisions. A task list is this session's, not his.

So for four and a half hours the pull request was blocked on a question nobody
could answer, and the board read it as an ordinary red. It is filed now, under
Blocking, with the three options and the measurement behind each: nine of ten
temporary names read 5,163,341,031 and `4b8c1a` read 5,163,341,042, reproduced
three times, against a `dev_clang` that never reads the flag the gate sets for
it (`src/main.rs:744-747`). STATUS.md's two counts moved from two blocking to
three.

The rule this breaks was already written: a decision that is Clay's goes to him
the moment it is found. What was missing is that "goes to him" has a file name,
and marking a task is not it. Worth adding to the check a session runs when it
opens a red pull request: if the redness is waiting on a decision, the ledger
gets an entry in the same turn, and the pull request's body cites the heading.

## 2026-09-18 — a finished ruling sat in the list cloud reads to choose work

STATUS.md's "Ruled, unbuilt" section said three rows stood. One of them,
"A demanded knot counts on one engine only (2026-08-24)", was built and merged
as kanso#1511 earlier the same day. The section's own second sentence says the
chat removes a row the day its build lands on main; the removal belonged in that
commit and was not made.

So for several hours the one list cloud is told to read before choosing what to
build advertised a finished job, complete with an Owes list of three things that
are done. That is the precise failure the section exists to prevent, pointing
the other way: it was written because five ruled features sat unbuilt while 296
pull requests landed, and an entry that is finished and still listed costs the
same reader the same wrong turn.

Probed against main before removing it rather than taken from a merge notice:
`src/eval.rs:3505` consults `codegen::knotted_constants`, and
`tests/golden.rs:194` names `mem_corpus_interp_matches_the_semantic_counters`,
whose loop kanso#1511's entry records being watched red against the unfixed
interpreter. Both halves the ruling asked for are there.

Two rows stand now: the cohort gavel's data-sized cycle, and the 2026-09-15
normalization ruling.

## 2026-09-18 — a clone sized for the growth that follows it

`Vec::clone` allocates capacity exactly equal to length. So `taken`'s clone arm
handed back a full vector, and the `push` or `extend_from_slice` immediately
after it had no room and reallocated — copying the whole buffer a SECOND time.
Every shared `push` and every shared `append` was paying for its contents twice.

    main (kanso#1518)          1,392,296,124
    + the clone sized to grow  1,297,297,477    -94,998,647   -6.82%

**And the peak came down with it**, which was not the point of the change:

    interp_allocs      3,879,653 -> 3,843,587     -36,066
    interp_peak_bytes    961,231 ->   885,118     -76,113   -7.92%

The peak falls because the reallocation was a doubling: a vector at 1,000 that
needs 1,001 asks for 2,000, and the sized clone asks for 1,001. That repays the
frames table's +9,727 from kanso#1518 eight times over, so the interpreter now
holds LESS than it did before any of this line of work started.

WHERE IT CAME FROM. `memcpy` is the largest single frame in the interpreted run
and its callers were read off the profile rather than guessed:
`__rust_realloc` 67,362,609 (4.68%) over 56,338 calls, and `kanso::eval::taken`
65,960,854 (4.58%) over 35,640. The realloc figure did NOT move when kanso#1518's
four `with_capacity` calls took `finish_grow`'s self cost from 51,556,413 to
8,135,509 — it stayed at exactly 67,362,609 across three profiles. That was the
clue: the reserves fixed small vectors growing by one, and this is large buffers
being copied whole, a different population reached by a different path.

The unique arm is deliberately untouched. A vector nobody else points at may
already carry spare capacity, and reserving on it would be this same mistake
pointing the other way.

Against main before any of the interpreter work, the row has now fallen
2,007,688,216 -> 1,297,297,477, **35.38%**.

The whole golden corpus passes, which is the assertion that matters: a capacity
is not observable, so an output difference would have meant the change was not
what it looked like.

### where the copying went, and one refinement measured and declined

The sized clone did more than take 94,998,647 off the row. It took `memcpy` off
the top of the profile:

    __memcpy_avx_unaligned_erms   162,103,398 (11.27%) -> 32,012,270 (2.38%)
    __rustc::__rust_realloc        67,362,609 of memcpy ->    770,336 self

Both halves of the memcpy story went at once, and the reason is that they were
one story: `taken`'s clone allocated exactly, `push` reallocated, and the pair
copied the same bytes twice. Removing the second copy removes the realloc that
performed it.

What is left of the copying is inside `taken_to_grow` itself, 44,816,580 (3.33%)
of self cost, where `out.extend(shared.iter().cloned())` walks the elements.

**AND THE OBVIOUS REFINEMENT IS WRONG, which is why it was measured.** A bulk
`extend_from_slice` looks strictly better than an element-wise clone, and for
`Vec<u8>` it should reduce to a `memcpy`. Built and run:

    extend(shared.iter().cloned())   1,297,297,477
    extend_from_slice(&shared)       1,323,762,604    +26,465,127

**26,465,127 WORSE.** Declined. The reasoning that recommended it — a bulk copy
beats a loop — is sound about bytes and says nothing about `Vec<Value>`, which
is the vector this helper is mostly handed and whose elements are not `Copy`.
Whatever the two spellings compile to for that case, the iterator one is better
here by two per cent of the whole run, and the guess was worth exactly what a
guess is worth.

THE NEXT LEAD IS SMALLER THAN IT WAS, and the note that sized it needs saying
again with this in it. `taken`'s copy-when-shared was 65,960,854 of memcpy over
35,640 calls when the linearity lead was priced at about 59 million. That
population is what this change just made cheap. Anything built on
`linear::in_place_pushes` now competes with a copy that already costs far less,
so the lead must be re-measured against this binary before it is built, not
taken from the earlier figure.

## 2026-09-18 — kanso#1520, CI's rows, and an instruction delta three times the projection

    interp_instructions   1,555,890,579 -> 1,260,262,910   -295,627,669  -19.00%
    interp_allocs             3,879,653 ->     3,843,587        -36,066   -0.93%
    interp_peak_bytes           961,165 ->       885,052        -76,113   -7.92%
    compile_instructions     35,486,173 ->    35,486,333           +160
    entry_instructions      126,498,498 ->   126,498,292           -206
    library_instructions    126,953,661 ->   126,954,304           +643
    startup_instructions      3,362,788 ->     3,363,378           +590
    emit_instructions        51,451,897 ->    51,456,464         +4,567

THE CONTAINER PROJECTED 94,998,647 AND THE RUNNER READ 295,627,669. Three times
as much, on the same source, and it is not a bad measurement on either side.

On the same pair of runs the two machines agreed TO THE BYTE on what the change
does: 36,066 fewer allocations and 76,113 fewer peak bytes, the same integers on
both hosts, on rows whose absolute values they do not share. So the change is
understood and behaves identically. Only its instruction price differs, and by
a factor of three.

WHAT THIS CORRECTS. Four sittings in a row — kanso#1516, kanso#1517 and
kanso#1518 — had their instruction deltas agree across the two hosts to 0.18%,
0.12% and 0.63%, and the log and the compiler page both wrote that down as
though it were a property of the counter. It is not. What a copy of a thousand
elements costs in instructions is a property of the rustc that built the
interpreter; how many copies happen is a property of the program. Only the
second crosses a machine boundary. Section 87 carried the older wording onto
main this morning and is corrected in this same commit rather than left to
stand.

The four close agreements were four changes whose work happened to compile
similarly on both hosts. This one does not, and the honest reading of the
earlier four is that they were not evidence of a law.

The floor is banked after these rows. The run side is unchanged — this branch
touches the interpreter only.

## 2026-09-18 — a bound name was copied twice, and the environment now keeps it inline

The interpreted row has fallen four times this week by taking work out of the
inner loop: a name resolution remembered, a call site's ladder remembered, a
frame built once per declaration, a clone sized for the growth that followed
it. This one is smaller in idea and about the same size in effect. Binding a
name to a value allocated twice, and after this it usually allocates not at
all.

`match_one` walks a pattern against an argument and pushes what it matched
into a `Bindings`, which was a `Vec<(String, Value)>`. Every name it pushed
was `name.as_str().to_owned()` — a fresh heap string. The caller then drained
that vector into the environment, and `bind` took `&str` and did
`name.to_string()`: a second malloc, a second memcpy of the same bytes, and a
free of the first one line later. Two allocations to store one name, once per
binding, on about half a million body entries in the corpus.

The first half of the fix is the obvious one. `bind` takes the name by value,
and the two loops that drain a `Bindings` move their string in rather than
lending it. Callers that hold an AST name clone at the call site, which is
what `bind` was doing for them anyway.

The second half is what makes the remaining copy free. `Name` is the type the
front end already uses for identifiers — twenty-four bytes, exactly what a
`String` costs, holding twenty-two bytes or fewer in the value itself and
boxing anything longer. 99.77% of identifier occurrences across `lib/` fit
inline and 89.8% are seven bytes or fewer, so for almost every binding the
remaining copy is twenty-four bytes of stack. `Env::name`, `Bindings` and
`ClosureData::params` change together, because a name that arrives as a `Name`
and is stored as a `String` pays for the crossing at the boundary. `Env` is
the same size it was.

Measured on this container, release, callgrind, `bench/interp_corpus`,
anchored at `run_interpreted_on_stack`:

    on 9b39e360, before kanso#1520
      main                       1,392,767,370
      the second copy gone       1,329,687,387      -63,079,983
      the environment holds Name 1,265,684,136      -64,003,251
                                                   -127,083,234    -9.12%

    on 30fb1abe, with kanso#1520 under it
      main                       1,297,739,654
      both                       1,169,848,989      -127,890,665    -9.85%

The two halves pay almost the same, which is what a doubled cost looks like
taken off one copy at a time. The total agrees across the two bases to within
807,431, 0.6% of itself, so kanso#1520 and this are independent.

These are container projections and not the row. kanso#1520 projected
-94,998,647 here and CI read -295,627,669, three times over, while its
allocation and peak deltas matched to the byte. The direction is the claim.

### The allocation count says which half did what

`a_unique_container_is_extended_in_place` runs the same program at 300 and at
600 rounds and pins the difference, so every fixed allocation cancels and what
is left is what the extra rounds cost. It went red on this branch, which is
the spec doing its job: the rounds got cheaper. Three trees were built rather
than one number subtracted from another:

    main                     12,001 per 300 rounds
    the second copy gone     10,801         four a round less
    the environment holds Name 9,001          six a round less again

Six bindings a round, and four of them arrive through a pattern match. The
first commit reaches only those four, because only a binding that came through
a `Bindings` had a second copy to drop; the second reaches all six, because
every binding allocated its name once whichever route it came by. The six is
the same six the frame-memory row in that spec's ladder counted, which is
what a body entry costs to enter.

The pinned number moves from 12,001 to 9,001 and the assertion is not widened.

### What was uncovered, and now is not

Storing a `Name` makes the boxed path load-bearing in the interpreter, and
nothing in the corpus bound a name past twenty-two bytes.
`a_name_longer_than_the_inline_bound_still_binds` binds a forty-five-byte
parameter, a thirty-nine-byte local, and two parameters whose first
twenty-two bytes are identical. The last pair is the one that matters: a store
keeping only the inline prefix would give both the same key, and a single
truncated name would still match a lookup truncated the same way, so one long
name alone proves nothing.

Watched red first, with `bind` storing `Name::new(&name.as_str()[..22])`:

    native   40 7 12                                               exit 0
    interp   error[runtime]: unknown name
             `a_parameter_name_longer_than_the_inline_bound`       exit 1

Native is untouched by the break, because compiled code binds nothing through
an environment. What catches this is the micro corpus running both engines
rather than the micro corpus running at all, which is the differential law
doing the work a single-engine golden could not.

One thing the fixture found on the way in, and it is worth writing down
because it nearly passed for the wrong reason. The first draft put a blank
line between its comment header and the first `fn`, and the run failed with
`error[formatting]: the file may not begin with a blank line` — empty stdout,
which reads exactly like the change being broken. A fixture that fails before
it runs proves nothing about what it was written to test, and the only way to
tell the two apart is to read the message rather than the verdict.

## 2026-09-18 — kanso#1522's rows on CI, and a container projection that held

The two commits under this branch are in the entry above. These are CI's
readings of them, against main at kanso#1521:

    interp_instructions  1,260,262,910 -> 1,138,001,430   -122,261,480  -9.70%
    interp_allocs            3,843,587 ->     2,539,998     -1,303,589 -33.92%
    interp_peak_bytes          885,052 ->       884,985            -67

Every other row in the job is byte-identical to main: `compile_instructions`
35,486,333, `entry_instructions` 126,498,292, `library_instructions`
126,954,304, `startup_instructions` 3,363,378, `emit_instructions` 51,456,464,
`compile_allocs` 27,313, and both codegen rows. Two veins failed and they are
the two this change is about.

The allocation fall is the largest single move that vein has recorded, and it
is the mechanism rather than a side effect: a binding allocated its name twice
and now usually allocates it not at all.

### The projection held, where kanso#1520's was three times out

This container read the instruction fall at 127,890,665 and the runner reads
122,261,480 — 4.40% apart. kanso#1520, six hours earlier, projected 94,998,647
here and CI read 295,627,669, and its own golden's header wrote that down as
the rule: *what a copy of N elements costs in instructions is the rustc that
built the binary; how many copies happen is the program.*

That rule predicts this. kanso#1520 changed how many BYTES a copy moves, so
its instruction delta is a per-byte cost the two hosts do not share. This
change removes allocations, and an allocation is a count the program decides —
so the two hosts differ only in what one malloc costs, and they are closer on
that than on a memcpy. The allocation deltas were the ones that matched to the
byte on kanso#1520, and here they are what moved.

So a container projection travels when the thing it projects is a count, and
does not when it is a per-byte price. That is a prediction with two readings
behind it rather than a law, and the next change that moves bytes should be
expected to break the box's projection again.

## 2026-09-18 — birth through a call, measured, and it is not what the cohort gavel's purpose is waiting on

STATUS.md's cohort row has owed one measurement since 2026-09-16: whether a
call that returns one record resolves to one birth. The 2026-09-09 entry named
that widening as the next one and left it to the implementer. This is the
measurement, taken by running programs rather than by reading the analysis.

The analysis first, because it says what to expect. `born_of` in `src/check.rs`
handles `Expr::App` with an identifier head in three ways: `if` takes both arms
and joins them, a field getter takes the base's field, and anything else does
`let decl = *types.get(name.as_str())?`. That `?` is the whole answer for a
function call — `types` holds type declarations, a function name is not in it,
and the call resolves to nothing. The comment above it says so: *a call that
merely returns a record may hand back something older.*

Five programs, each run through `kanso play` on a release build of
`30fb1abe`. What each one is refused for is the finding, and the five refusals
are not the same refusal.

**One. A hole cannot be written outside a build block.**

    fn fresh id
      node id _

    error[build]: `_` is a hole for a field a `build` block fills; it stands
    only where a construction's argument goes, inside one

**Two. A call that returns a record is not block-born.** This is the case the
row's route is about, and it is the only one of the five that widening
`born_of` would fix.

    fn fresh id
      node id []
    build
      a = fresh "a"
      a.peers = [b]

    error[build]: `a.peers = ...` writes only block-born values: `a` is not a
    construction made in this `build` block, so it stays immutable

**Three. A hole cannot escape the block it was written in.**

    fn fresh id
      build
        n = node id _
      n

    error[build]: `_` in `node`'s `peers` is never filled: a hole is filled
    exactly once before the block freezes

**Four. A lambda lexically inside a build block is outside it for this
purpose**, which is the shape "N nodes from a list" actually takes.

    build
      ns = list/to_list (list/map [1 2 3] (i -> node "{i}" _))

    error[build]: `_` is a hole for a field a `build` block fills; it stands
    only where a construction's argument goes, inside one

**Five. A fill's target parses as a bare name**, so N nodes need N names before
any analysis gets a say.

    build
      ns[0].peers = [b]

    error[syntax]: expected a parameter pattern

And the control, so the five refusals are refusals of something rather than of
everything: two nodes named by hand, filled, and collected into a list runs and
prints the cycle.

So the answer to the row's question is that birth through a call does not
resolve today, and that widening it is real and separable work — it fixes the
second of these and nothing else. What it does not do is reach the gavel's
purpose. "Cyclic structures sized by data" needs a hole to survive either a
call or a lambda, and one and three close those two directions with different
rules, and five closes the indexed route in the grammar before the checker is
consulted.

That is the branch the row named: the measurement says no, so what the purpose
needs goes to the ledger as a question about the spelling rather than a build
anybody can start.

### And `build_cycle.kso` now says what it pins

That fixture had no header. The row's Owes said the golden's header should
stop claiming four shapes while the checker admits two, and a repo-wide search
finds that claim in `design/compiler-log.md` and
`design/memory-frontier-research.md`, both about the memory frontier's shapes,
and in no golden header anywhere. There was no claim to correct; there was a
file saying nothing. It now carries what it pins, which two shapes the
build-hole gavel took back, and the sentence that two names is the largest
cycle the language admits rather than a choice the fixture made.

Writing it cost a round, for the second time today and for a second reason.
The header's eleventh line ran to 82 characters and the run stopped with
`error[formatting]: a line holds at most 80 characters`, stdout empty, which is
what the mem corpus reported: a stdout mismatch against the golden with an
empty left side. This morning's fixture failed the same way on a blank line
between its comment block and the first declaration. Both are the grammar
refusing the file before a line of it runs, and both look from the test's
verdict exactly like the change under test being broken. The message says
which; the verdict cannot.

### The knot spec staged two tests into one directory

The macOS job named a second failing target beside the index count:
`a_demanded_knot_counts_the_same_on_both_engines`, which landed this morning
as kanso#1511 and which this branch does not touch. Its message was not in the
part of that log I read, and the log carries exactly one panic — the index
one. It passes on Linux, on this container three times out of three, and on
main's own macOS job. That is what a race looks like from outside, so the code
was read rather than the verdict.

`ran(interp)` stages its library and entry in
`temp_dir()/kanso-demanded-knot-{interp}` and calls `remove_dir_all` on that
path at both ends. THREE tests call it FOUR times, and `cargo test` runs them
on parallel threads. Printing the path and the thread id says the rest:

    /tmp/kanso-demanded-knot-0   ThreadId(2)   ThreadId(4)
    /tmp/kanso-demanded-knot-1   ThreadId(3)   ThreadId(4)

Two threads, one directory, destructive on entry. One thread's leading
`remove_dir_all` deletes the library another has just written and is about to
run.

Watched red by widening the window rather than by waiting for luck: holding
the first two calls for 800ms between the write and the run makes the other
two fail with `the run failed (native)` and `the run failed (oracle)` — a
target that fails with no counter assertion at all, which is the shape the
macOS job showed.

The fix is one staging directory per CALL, from an atomic counter, zero-padded
so the path length stays fixed as well as unique. Length does not matter to
these three assertions, which pin thunk counters; it matters to the sibling
spec `a_unique_container_is_extended_in_place`, whose header records a run's
allocations tracking the length of the path it was handed and macOS staging
under `/var/folders/...`. One habit for both is cheaper than remembering which
is which.

What this does not establish is that the race is what macOS hit. The failure's
own message was never visible to me, and a spec that is green here and red
there could have had another reason. What is established is that the shared
directory is real, that it fails under a widened window, and that it is a
defect either way.
## 2026-09-18 — the gate that accused a stable binary, and the arithmetic behind it

Two pull requests sat blocked on the same red check and the same words:

    THIS BINARY COUNTED TWO NUMBERS IN ONE JOB: 1138001437 and then
    1138004452, on one binary, one corpus and one machine.

`interp_instructions.sh` takes a second callgrind pass whenever the row
disagrees with its golden, and the second pass exists to separate two cases
that are settled differently. A binary that counts two numbers is a
reproduction failure: the vein halts and the cause is hunted, and the value is
never pinned. A binary that counts one number the golden does not hold is an
ordinary ratchet. The gate's own text says so.

The binary was stable. 1,138,004,452 − 1,138,001,437 is 3,015, and both jobs
printed `interp_printed=3015` as a notice. The first reading is the anchored
frame with the printed line taken off; the second was the raw frame. Two
readings of the same quantity, less one subtraction.

That is why the pair reproduced across two branches and two runners. kanso#1502
carries two divisions in Ryu's float rendering, kanso#1504 caches the innermost
beat mark, neither goes near the interpreted corpus, and both drew
1,138,001,437 and 1,138,004,452 — because the gap is not a measurement at all,
it is the subtraction the second reading skipped.

**Where it came from.** kanso#1487 gave the three compile-side gates the
exclusion on 2026-09-17, writing the reader as `printed_cost()` and calling it
on both profiles. kanso#1505 brought the same exclusion to the interpreted row
that day and open-coded the pipeline instead of naming it, which reached the
first reading and not the second. The fix is to make this gate look like its
three siblings.

**What the defect cost, and why nothing caught it.** The second count only runs
on the failure path, so the bug fires exactly when a reader most needs the
answer and is invisible the rest of the time. It cost two pull requests a round
each and very nearly cost more than that: the branch-side diagnosis had already
been written down as layout jitter — a +7 that had supposedly landed three
times on three different absolute values — and a log entry saying so was
committed before the job log was read. A spread of 3,015 on a row whose
allocation counter never moved is not layout, and the gate's own notice line
said as much two screens above the error.

So the golden edit that entry justified is withdrawn, and the +7 goes back to
being an unexplained disagreement between this tree and its golden rather than
a shape with three readings behind it. Whether the row should read
1,138,001,430 or 1,138,001,437 is a question the fixed gate gets to answer.

`tests/a_gates_second_count_is_read_like_its_first.rs` pins the property: a
gate that subtracts a printed line from its first reading subtracts one from
its second, and it reads for the CALL rather than the text, because the defect
was one call site out of two rather than a missing subtraction anybody could
see. It reads by the same marker as
`every_anchored_gate_answers_for_the_printed_line.rs`, so the two agree on
what counts as subtracting by construction; a looser match had
`startup_instructions.sh` failing on prose explaining why its program prints
nothing.
## 2026-09-18 — the interpreter reads the linearity analysis at last

`linear::in_place_pushes` has told the emitter since kanso#1359 which push,
put and append sites extend a container nothing else will read. The emitter
writes `push_mut_fast` at those sites and mutates. The interpreter consulted
the analysis not at all — `grep 'linear::' src/eval.rs` came back empty — and
cloned at every one of them.

The corpus says how much that costs. Instrumenting `taken` and
`taken_to_grow` to print `Rc::strong_count` gives 44,006 container-extension
calls, of which only **1,326 (3.01%)** are uniquely owned and 42,680 (96.99%)
clone. That 3.01% is worth pausing on: kanso#1520 won 295,627,669 instructions
without moving it, by sizing the clone for the growth that follows rather than
by making more containers unique.

Those 44,006 calls come from **sixteen** distinct sites, and all sixteen are
in the **fifty-four** a build of the same corpus marks in-place. So almost
every clone the interpreted run makes is one the compiled engine already does
not make.

### What it took, and what it did not

Not the plumbing, which was the expected obstacle and was already built.
`call_builtin` has taken a `&Frame` since the frame work, and `Frame` is an
`Option<Rc<Site>>` built once per declaration by `frame_of` and memoized by
pointer since kanso#1518. `Site` gains the declaration's file — an `Arc` clone
on a struct already paid for — and the key becomes the same (file, line,
column) the emitter uses at its own push arm.

The gate is `Rc`. It refuses `&mut` above one holder, and the other holder is
the environment node binding the accumulator's name, in a persistent chain
where removing one node costs more than the clone. So this is the project's
SECOND `unsafe`, after `name.rs`: the contents are taken through
`Rc::as_ptr`, leaving an empty vector behind for a holder the analysis proved
never looks.

The justification is the analysis rather than the refcount, and the
differential law makes it checkable. Building the version with NO gate — an
unconditional take — turns exactly the right things red:

    a_list_held_twice_is_not_pushed_into
    a_builder_passed_twice_through_a_wrapper_is_not_written_through
    an_imported_builder_held_twice_is_not_written_through
    micro_corpus_agrees_across_engines
    a_unique_container_is_extended_in_place

Three of those are named for this exact defect. All five are green with the
gate in. That is the argument: not that the write is safe because somebody
reasoned it was, but that the corpus already knows what an unsound in-place
write looks like and says so.

### The numbers

On this container, release, callgrind, `bench/interp_corpus`, against main at
`c5974233`:

    run_interpreted_on_stack  1,169,887,916 -> 1,116,560,609  -53,327,307  -4.56%
    interp_allocs                 2,539,996 ->     2,486,374      -53,622  -2.11%
    interp_peak_bytes               885,189 ->       833,330      -51,859  -5.86%
    printed answer                interp 59442 on both

The ungated build measured 69,425,920 on the older base, and the difference
between that and 53,327,307 is the gate's own cost plus what kanso#1522
removed from under it. Both are container readings. This change removes
memcpys, so it moves BYTES — the kanso#1520 shape, whose container projection
came in three times low, where kanso#1522 moved counts and held to 4.40%. The
runner should read MORE than 53,327,307, and the golden will say.

### And the one pin that this repository was missing

`a_unique_container_is_extended_in_place` moves 9,001 to 7,803 per 300 rounds,
four a round less, which is the two builders' four extending calls no longer
copying their accumulator.

That row matters more than the arithmetic. The five specs above catch the
optimisation firing where it should NOT. Every one of them stays green if the
gate silently stops matching — if a key changes, if a span moves, if the
analysis is handed a different program. This number does not. It is the only
thing in the tree that fails when the optimisation quietly stops happening,
and the spec's own header now says so.

## 2026-09-18 — kanso#1525's rows on CI, and a projection that was right about its direction

Eight rows moved. CI's sitting, against main:

    interp_instructions   1,138,001,430 -> 1,075,174,600   -62,826,830   -5.52%
    interp_allocs             2,539,998 ->     2,486,376       -53,622   -2.11%
    interp_peak_bytes           884,985 ->       833,130       -51,855   -5.86%
    compile_instructions     35,486,333 ->    35,552,188       +65,855   +0.19%
    entry_instructions      126,498,292 ->   126,735,634      +237,342   +0.19%
    library_instructions    126,954,304 ->   127,192,177      +237,873   +0.19%
    emit_instructions        51,456,464 ->    51,630,538      +174,074   +0.34%
    startup_instructions      3,363,378 ->     3,365,595        +2,217   +0.07%

**The container projected 53,327,307 and said to expect more.** It read
62,826,830, 17.8% above the projection. The reason was written down before the
job ran rather than after: this change removes memcpys, so it moves BYTES, and
a per-byte price is a property of the compiler that built the interpreter. The
kanso#1520 shape came in three times low; this one came in a sixth low, and in
the same direction.

**The allocation counter travelled exactly.** −53,622 on this container and
−53,622 on the runner, to the unit. That is the second half of the same rule
and it keeps holding: a counter of operations crosses a machine boundary
intact, a counter of instructions does not.

`interp_peak_bytes` moved −51,859 here against −51,855 there, four apart on
fifty-two thousand, and the first draft of this entry called that four
unexplained. It is not a puzzle, and checking took one command.

`interp_memory.sh` refuses to compare on this box at all: the golden was
measured on glibc 2.39-0ubuntu8.9 with rustc 1.98.1 and this container is 8.7
and 1.94.1, so the gate declines rather than reads. The absolute figures differ
by about two hundred at both ends for the same reason — 885,189 here against
884,985 there before the change, 833,330 against 833,130 after. Two hundred is
the host, and four is what survives of it in a difference.

So the notable thing is the opposite of what the draft said: across two hosts
the gate will not even compare, the DELTA agreed to within four bytes. The
golden's own header says what its reading may be set beside, and reading it
first would have saved writing the sentence twice.

**The four compile-side rows are the price of the change existing.** `kanso
check` stops before the interpreter runs and the analysis is behind a
`OnceCell` that only the interpreter touches, so not one of those rows can be
paying for work this change does. They move because `src/eval.rs` IS the
compiler: it grew, and the binary's layout moved under it. Three of them move
by the same 0.19% — +65,855, +237,342 and +237,873 on three different routes
through the same front end — which is what a uniform layout shift looks like.
CLAUDE.md's standing note on this row says an edit to the compiler's own Rust
usually moves it, and this is that case rather than the rarer one where a small
enough change leaves the layout alone.

So the trade is 62.8 million interpreted instructions against 715,346 across
the four compile-side rows, and it is not close.

## 2026-09-18 — the page gates read a conflict marker and called it prose

Five branches each added a section to `docs/compiler.html` today, and every
merge from main conflicted in the same place: the end of the file, just above
the coda, where every new section goes. Four of those resolutions went fine.
One did not, and what it cost is the point.

The HTML was left unmerged. A script then rewrote the file for an unrelated
reason — renumbering a section — which wrote the working tree's contents back
out, markers included, and the commit went in with `<<<<<<< HEAD`, `=======`
and `>>>>>>> origin/main` sitting in the published page.

**All three page gates then ran on that tree and all three passed.**
`golden_prose` reads the `data-golden` spans and there were none in the hunk.
`page_drift` counts log entries against the page's git history and the page had
moved. `prose_check` reads twenty-nine pages for three families of sentence,
and a conflict marker is not a sentence. The sweep printed *the three page
gates agree with what the tree says* over a page with three markers in it.

Nothing downstream would have caught it either. The markers are text in HTML,
so a browser renders them as a line of prose rather than failing; the site
builds; the book checks pass. It was found by a `grep` run for something else.

`tests/no_published_page_carries_a_conflict_marker.rs` closes it. It reads the
same two directories `prose_check` reads, off disk rather than from a list, so
a page added later is covered without anybody remembering the file exists. It
was watched red against a reproduction of the exact failure — the same three
markers in the same place — and names the file and line of each.

Two details in it are deliberate. The middle marker is matched as a WHOLE LINE
equal to seven equals signs, where the other two are matched as prefixes: a row
of equals signs is ordinary punctuation under a heading or inside a fenced
block, and matching it loosely would fail on prose somebody wrote on purpose.
And the message says to check the section numbers afterwards, because the merge
that leaves a marker is the same merge that leaves two sections numbered 88 —
both happened in the same resolution, and finding one is a reason to look for
the other.

The general shape is one this file already knows: three gates that read the
same file for three different properties leave the union of what none of them
reads. The published number, the entry budget and the sentence families were
each checked, and whether the file was a finished merge was checked by nothing.
## 2026-09-18 — the log goes back under its cap, and the reason it had not was checkable

`design/compiler-log.md` reached **77 entries and 4,624 lines** against a cap
this file states as forty. **The first thirty-seven move** to
`design/log/compiler-log-archive.md`.

That count is the invariant; what is left behind is not. Forty remained when
the move was made, and every merge from main since has appended more — the file
was at forty-three by the time this landed. The cap is a level the rotation
returns the file to, not a bound each commit holds, and the next rotation is
due when it drifts this far again.

It is a pure move and that was verified rather than assumed: every one of the
1,496 headings across the two files is present before and after, and the two
files together are **byte-identical in total** — 4,353,591 before, 4,353,591
after. Nothing was trimmed, reflowed or reworded.

The boundary is by FILE ORDER and not by date, because the log is not sorted by
date. Entries are appended when the work happens, so an entry written on the
18th about the 17th sits after one written on the 18th about the 18th; the
thirty-seventh entry is dated 09-18 and the thirty-eighth 09-17. Taking the
first thirty-seven in file order keeps the archive's tail and the live file's
head adjacent, which is what "move the older end" means here.

**Why this sat undone, and what that cost.** The task carried a note saying the
rotation conflicts with every open branch, so it should wait for an empty board
— and the board has not been empty for weeks. The note was wrong, and one
command says so. A rotation DELETES from the head of the file; a branch APPENDS
at the tail; the two hunks do not overlap. Against the four branches open when
this was written:

    claude/ryu-two-divisions      CLEAN
    claude/beat-top               CLEAN
    claude/interp-reads-linear    CLEAN
    claude/no-conflict-markers    CLEAN

`git merge-tree --write-tree` answers that in about a second per branch, and
the four log conflicts resolved by hand earlier today were all tail-against-tail
— two branches appending in the same place — which is a different thing
entirely and is what the note generalised from.

The ruling counts are unchanged across the move, both greps: fifty under
`— gavel[:,]` and twenty-three under `GAVEL(ED)?[:,(]`. They are printed here
because a move of this size is exactly when a count would go wrong unnoticed,
and because the second grep is the one people forget.
## 2026-09-18 — a byte beside the name, built and declined: one byte costs eight

The interpreted run spends **48,165,663 instructions (4.14%)** inside
`__memcmp_avx2_movbe`, and the largest caller is `eval_ident`'s walk of the
environment chain — **724,304 calls**, a figure that matches the one section 87
already publishes for locals that stop at that walk.

The walk compares `frame.name.as_str() == name`. `str` equality checks the
length and then calls `memcmp`, so length is already a free rejection and what
reaches libc is every binding in the chain that happens to be the same length
as the one being looked up. A byte of the name, compared first, should reject
most of those without a call.

It was built. `Env` gained a `head: u8`, `bind` read it once, `lookup` compared
it before the string. Both engines print the same answer.

    base   row 1,115,996,775   memcmp 48,165,663
    head   row 1,126,405,836   memcmp 45,699,943

**The memcmp fell by 2,465,720 and the run rose by 10,409,061.** A net loss of
about eight million instructions, 0.93% of the row.

**The mechanism is the node, and it was measured rather than guessed.**
`std::mem::size_of::<Env>()` reads 64 on main and **72** with the byte. There is
no padding to put it in: `Name` is 24 bytes, `Value` is 32, `Option<Rc<Env>>` is
8, and 24 + 32 + 8 is exactly 64. Rust already orders the fields to pack them,
so one byte of payload costs eight of node, and the interpreted run allocates
about 2.5 million of them. That is twenty megabytes of extra traffic to save
two and a half million instructions of comparison.

**So this is declined on arithmetic, not on taste, and the arithmetic says what
would change it.** A discriminator that lives in space the node already has
would keep the saving and drop the cost. `Name`'s own 24 bytes are full — the
inline variant is twenty-two bytes of text plus a length and a tag — so there is
no room there either. What is left is a representation change to `Name` or to
`Value`, which is a larger question than this lead, and the 2.5 million is the
ceiling on what answering it would be worth.

Recorded so the next reader does not rebuild it. The idea is sound; the node is
the wrong size for it.

## 2026-09-18 — the second way to avoid the compare, and what losing twice says

The entry above declined a byte beside the name because the node grew from 64
to 72. It named the alternative: a discriminator in space the node already
owns. There is one, and it is better than a discriminator — it is the whole
comparison.

`Name::new` zero-fills the inline buffer before copying, so two inline names
hold byte-identical twenty-two-byte buffers exactly when they hold the same
text. The padding makes the length implicit and an identifier cannot contain a
NUL to blur it. So the walk can build a padded key ONCE per lookup and compare
a fixed-width block per frame, which rustc inlines, where a `&str` comparison
of a run-time length reaches `memcmp`. Nothing grows: the key lives on the
stack for the duration of one lookup.

    base   row 1,115,996,775   memcmp 48,165,663
    key    row 1,150,537,278   memcmp 35,577,082

**It removes 12,588,581 instructions of `memcmp`, twenty-six per cent of the
whole figure, and the run rises 34,540,503.** A net loss of about thirty-four
million, three times worse than the byte.

**What was checked about correctness, stated exactly.** The interpreted corpus
prints byte-identical output before and after, which is what makes the two
instruction counts comparable. That is a comparison of two BUILDS on one
engine, and the first draft of this entry called it "both engines print the
same answer" — which would be the differential check and was not run. The
golden suite was started against this build and the worktree was removed out
from under it, so it reported a failure that is an artifact of the removal and
says nothing either way. Since the change is declined, no further verification
was done; if it is ever revived, the differential goldens are where it starts.

**Two schemes, both sound, both losing, and the second loss is the informative
one.** The byte lost to the node's size; this one has no node cost at all and
loses by more. What is left to blame is the setup: a key is built per LOOKUP
and the saving is per FRAME, so the trade only pays when the chain is long, and
on this corpus it is not.

That reframes the lead. The 48 million is real, and it belongs to the
walk rather than to how the walk compares. Every scheme that keeps the walk and makes the compare
cheaper is paying a per-lookup cost to save a per-frame one, and the ratio
decides it. What would actually remove the cost is not walking: resolving a
local to a slot at parse time, so the interpreter indexes instead of searching.
That is a larger change than either of these, and it is the one the 48 million
argues for.

Recorded with the arithmetic so the next reader inherits the conclusion rather
than the two experiments. The comparison is not the problem.

## 2026-09-18 — the chain is 2.52 frames deep, which is why both schemes lost

The entry above said the trade "only pays when the chain is long, and here it is
not." That was inferred from two losses rather than measured, so it was
measured. A counter in `lookup` over the interpreted corpus:

    lookups          1,056,329
    frames visited   2,662,536      2.52 per lookup
    misses             332,025      31.4%, and a miss walks the whole chain

**The instrument agrees with two figures this project already published, to the
unit.** 1,056,329 is the `eval_ident` call count in section 87. Subtracting the
misses leaves **724,304**, which is section 87's count of locals that stop at
the environment walk AND the memcmp caller count read off the debuginfo build.
Three independent paths to the same two numbers.

**2.52 is the whole explanation.** A scheme that pays a setup cost per LOOKUP
and saves per FRAME has two and a half frames to amortise it over. The padded
key cost about forty-seven instructions a lookup and could save at most the
eighteen-odd a small `memcmp` costs, times 2.52 — so it lost, and it would have
lost at any setup cost above about forty-five. The head byte had no setup at all
and lost to the node instead. Neither failure was about the comparison.

**And it bounds what slot resolution could be worth, which is the point of
measuring rather than guessing.** 48,165,663 instructions of `memcmp` over
2,662,536 frame visits is about eighteen a visit. Resolving a local to an index
at parse time removes the visit, not just the compare, and an index costs two or
three instructions instead of eighteen — so the ceiling is roughly forty
million, near 3.6% of the interpreted row. That is worth doing and it is now a
number rather than a hope.

It is also a larger change than anything tried here: slots have to survive
closures, which capture an environment rather than a frame. Recorded as sized
and unbuilt.

## 2026-09-18 — the eleven, on a pull request that changes no code at all

This branch edits `design/compiler-log.md` and `docs/compiler.html`. That is the
whole diff: no Rust, no C, no kanso, no golden. Its cost-goldens job failed.

    codegen_instructions_release   first  6,824,133,291
                                   again  6,824,133,280
                                   golden 6,824,133,280

Eleven apart, in one job, both readings seeing five processes
(`first_procs=5 again_procs=5`, so the gate's guard against a second reading
that measured something else is satisfied).

**This is the strongest isolation the eleven has had.** kanso#1512 hunted it to
the temp object's NAME — nine of ten names reading one value and `4b8c1a`
reading eleven more — and everything since has been a reading on a branch that
changed *something*, which always leaves room for the change. A diff of two
documentation files leaves none. The row moved with nothing to attribute it to.

It is also the second twice-in-one-job reading today. kanso#1502 drew
6,820,866,355 then 6,820,866,344 a couple of hours ago; the absolute values
differ because main moved between them, and the eleven does not.

**What this settles, and what it does not.** It settles that the release row's
instability is a property of the build rather than of any change under test, so
a reader who sees this row disagree should not look at the diff. It does not
say what the pin should be — that is the question in
`design/pending-gavels.md` under Blocking, waiting on Clay since 06:19Z, and
kanso#1513 carries the proposal to narrow the pin to the release tier. This
entry is evidence for that decision and not a substitute for it.

`codegen_release_kanso_excluded` moved 74 between the two readings
(80,660,230 and 80,660,156). That is kanso's own process, excluded from the row
by the 2026-09-15 normalization ruling, printed rather than counted — and it
moving while the counted row holds is the exclusion doing its job.

## 2026-09-18 — the depth distribution, and a cheaper lead than the one just recorded

The entry above sized slot resolution off an average of 2.52 frames. An average
hides the shape, so the shape was measured. Hits by depth, interpreted corpus:

    1   267,899   37.0%
    2   199,016   27.5%
    3   130,524   18.0%
    4   101,095   14.0%
    5    25,770    3.6%
    6+        0

**The chain never exceeds five.** That is worth knowing on its own: there is no
tail, so nothing here is waiting on a pathological case.

Splitting the frame visits by outcome is what changes the recommendation:

    hit frames    1,590,733  over 724,304 hits    2.20 deep
    miss frames   1,071,803  over 332,025 misses  3.23 deep
    total         2,662,536                       matching the earlier count

**Misses are 31.4% of lookups and 40.3% of every frame visited.** A miss walks
the whole chain and finds nothing, because the name is not a local at all — it
is a function, a descriptor, a type or a builtin, and `eval_ident` falls
through to the rest of the ladder afterwards. At roughly eighteen instructions
a visit that is **about 19.3 million instructions spent walking chains that
cannot succeed**.

**So the cheaper lead is not to make the walk faster but to skip it.** A name
that is never bound as a local at a given site is decidable where the site is
compiled, and skipping the walk for those needs no slot machinery, no upvalue
analysis and no change to how a closure captures — the three things that make
slot resolution large. It is worth roughly half of what slot resolution is
worth and a small fraction of the work.

That does not retire slot resolution: the 1,590,733 hit-frames are still there
and only an index removes them. It reorders the two. Do the skip first, measure
what is left, and let the remainder argue for the redesign or not.

Recorded rather than started, with four pull requests in flight. The
measurement is the deliverable here; kanso#1516 already memoises what a name
resolves to, so the first question for whoever picks this up is why that memory
does not already prevent the walk.

## 2026-09-18 — the spec the slot work needs, watched red before it was green

`tests/golden/micro/a_local_resolves_to_its_own_depth_not_a_neighbour` pins that
a local resolves to its own binding and not to the one beside it. Resolving a
local to an INDEX has to land on the same frame the walk lands on, and the way
that goes wrong is an index one out, which reaches a neighbouring binding rather
than nothing. So every wrong answer here has to be a wrong VALUE.

Three shapes, chosen for that:

`ladder` binds five consecutive integers and prints all five. Off by one in
either direction prints a neighbour, and every candidate answer is a perfectly
good number.

`after_guard` puts three binds after a `return ... if` line. This is the shape
the entry above got wrong: `Stmt` has three variants and none of them branches,
which reads like a flat sequence, and `Expr::Guard` carries its own statement
list and runs it only when the condition is false. The binds after a guard are
nested rather than next.

`shadowed` takes the name of something already in scope, and THE CHECKER
NARROWS WHICH SHADOWING IS EVEN POSSIBLE. A local may not take the name of a
declaration in its own module -- "`base` is already a declaration; rename the
binding" -- which the first draft of this fixture was refused for. The only
legal shadowing is of a bare-enrolled import, so `round` shadows `math/round`,
and reaching past the local finds a function value rather than an error. That
is a constraint worth having: at any site a name is a local or it is not, and
never both.

WATCHED RED, FOR THE RIGHT REASON. `lookup` was broken to return the matched
frame's PARENT -- one frame too far, exactly what an index off by one does --
and the fixture printed:

    10 11 12 13 14      against   11 12 13 14 15
    1 11 21             against   11 21 31
    1 8                 against   8 9

Every line a valid number, nothing raised, all three shapes caught. The native
engine was untouched by the break, so the two engines disagreed and
`micro_corpus_agrees_across_engines` failed on the divergence rather than on
the golden alone. Restored, and green again.
## 2026-09-18 — the 19.3 million was an average applied to the wrong half

Section 90 closed by sizing a lead: misses are 31.4% of lookups and 40.3% of
every frame visited, "about 19.3 million instructions spent on walks that
cannot succeed". That figure was 1,071,803 miss visits times the eighteen
instructions a visit the section derived earlier. The visits were counted. The
eighteen was an average over every visit, and it does not describe the half it
was applied to.

BUILT. A filter that skips a walk which cannot succeed. `bind` is the only
place an `Env` frame is made, so it records the shape of each name it binds --
the first byte and the length -- into a 256-entry table on the interpreter, and
`lookup` answers `None` without walking when the shape is absent. The table only
gains bits and a set bit admits the walk, so a collision costs a walk that would
have happened anyway and can never change an answer.

Sized first on the corpus: 35 distinct names are ever bound, the 332,025
lookups that reach no frame are spread over 73 others, and one of the 73 shares
a shape with a local. So the table skips 326,733 of the 332,025 -- 98.4%.

MEASURED, and it costs 24,320,374 (+2.18%). Four binaries, one worktree, each
staged into the gate's own box in turn:

    base                                          1,115,996,775
    the reference threaded, never read            1,132,586,649   +16,589,874
    the shape recorded, never tested              1,140,109,198    +7,522,549
    recorded and tested, walks skipped            1,140,317,149      +207,951

THE PRIZE WAS SIXTEEN TIMES SMALLER THAN THE ARITHMETIC. The last row is the
whole of what skipping the walks is worth, and it is a rise. `memcmp` reads it
directly: 48,165,663 in the first three binaries and 47,209,178 in the fourth.
Removing every one of the 1,071,803 miss visits takes 956,485 off `memcmp` --
1.99% of the figure, from 40.3% of the visits.

So a miss visit costs 0.89 instructions of `memcmp` and a hit visit costs 29.68.
Thirty-three times apart, and section 90's eighteen is the average of the two.
The mechanism is visible in the names: Rust's string equality checks the length
before it calls libc, every local this corpus binds is seven bytes or shorter,
and the names that miss are `builtin_append`, `json/parsed`, `text/utf8`. A
miss walk is rejected on length at every frame and never reaches libc at all.

AND THREADING THE REFERENCE COSTS MORE THAN THE IDEA. 16,589,874 of the rise is
a fourth argument to `bind` that nothing reads -- 534,994 binds, all from
`dispatch`, 31 instructions each. Recording into it adds 7,522,549 more, 14 a
bind. A thread-local would remove the first half and the second half alone still
exceeds the 956,485 ceiling, so there is no arrangement of this idea that wins.
Declined on arithmetic, with both halves isolated rather than attributed.

WHAT IT DOES TO THE SLOT LEAD IS THE OPPOSITE. Section 90 put slots at "around
forty million", from the same eighteen. The `memcmp` is not spread over the
visits evenly: 47,209,178 of it sits in the 1,590,733 HIT visits, which are what
a slot removes. The lead is larger than the page says and its cheap-looking
neighbour was the part worth nothing. Still a ceiling on one term, still
unbuilt, and now sized against the population it describes.

## 2026-09-18 — what a slot would actually need, read off the tree before building

The re-sizing above puts 47,209,178 of the `memcmp` in the hit visits, which is
what resolving a local to an index removes. Section 90 said the hard part is
that closures capture an environment rather than a frame. Reading the tree says
which part of that is hard and which part is already settled.

SETTLED: a local's depth from its own frame is static at every site.

`Stmt` has exactly three variants -- `Bind`, `Expr` and `Set` -- and not one of
them branches. THAT DOES NOT MAKE A BODY A FLAT SEQUENCE, which is what this
entry said first and corrected an hour later. The branching is in the
expressions, and three of them carry a statement list of their own: `Block`,
`Build`, and `Guard`'s `rest`. A `return x if c` line evaluates `rest` only when
the condition is false, so the statements after a guard are a nested list rather
than the next items in a sequence.

`tests/golden/micro/a_wall_whose_name_is_a_local.kso` says exactly that, in its
own comment: "a binding that follows a `return` line lives in `Expr::Guard`'s
own statement list rather than in a block, and a walk that knew only blocks
refused this program". An earlier pass learned it from a program the compiler
refused. This one read the enum and walked past the fixture, which is the same
mistake as sizing a lead from an average -- the thing was written down one file
away.

What survives is what slots need. Each of the three runs its list on the
environment as it stands where it appears, so a site inside one has a static
depth. Only a `Build` in STATEMENT position takes `&mut env` and lets its binds
escape into the parent's sequence; a `Block` or `Build` in expression position
takes `env` by reference and its binds go no further. The body is a TREE of
statement lists with static depths, rather than one flat list.

A bind pattern is irrefutable by rule -- `destructure`'s fallback says so in as
many words, "binding patterns are irrefutable: names and constructor patterns
only" -- so a `Stmt::Bind` binds the pattern's whole static set of names or the
evaluation stops with an error. There is no path that binds half of them and
carries on.

Parameters are static too, for a reason worth writing down because it looks
dynamic. `dispatch_loop_inner` picks the overload at runtime by score, so WHICH
declaration's body runs is a runtime answer -- but `match_params` builds a FRESH
`binds` vector per candidate, and only the winner's survives. `match_one`'s `?`
does return from the middle of a constructor's field loop with earlier
sub-patterns already pushed, and that partial vector is discarded whole with the
candidate. So on the match that wins, `binds` holds exactly that declaration's
binders in left-to-right order, plus `bind_whole`'s one when the pattern names a
whole. Fixed per declaration.

STILL HARD, and it is the only part: `ClosureData` carries
`env: Option<Rc<Env>>` and `call_closure` starts from `closure.env.clone()`, so
a closure captures the chain. A site inside a closure body has a static depth to
its own parameters and a depth to a captured name that depends on where the
closure was made. That is the ordinary upvalue problem and it has ordinary
answers -- a two-level index, or a flat upvalue vector built once at closure
creation. Which one this wants is the question the build starts from.

The spec it needs is small and should be written first: a local shadowing a
global, read at a site that also reaches the global from an enclosing scope, so
a wrong index prints the wrong value instead of crashing.

## 2026-09-18 — the closure capture is not in the way, measured

The entry above called the closure capture "the only part" still hard about
resolving a local to an index, and named the two usual answers. Neither is
needed to collect the prize, and the reason is a count rather than an argument.

`call_closure` pushes the captured head onto a stack around the body it
evaluates, and `lookup` marks whether its walk passed that frame before it
matched. Over bench/interp_corpus:

    closure calls                                440

    hits   outside any closure body          722,764    99.79%
           inside one, above the capture        1,540     0.21%
           inside one, at or below it               0     0

    hit    outside any closure body        1,588,753    99.88%
    visits inside one, above the capture        1,980     0.124%
           inside one, at or below it               0     0

NOT ONE NAME LOOKUP IN THE RUN REACHES A CAPTURED FRAME. The 1,540 hits inside
a closure body all resolve to the closure's own parameters, which sit above the
capture and have static depths like any other parameter.

So the 47,209,178 of `memcmp` the hit visits carry is 99.88% inside plain
function bodies, and a slot scheme that covers those and leaves closures walking
by name collects essentially all of it. The upvalue question is real and it is
not on the path.

WHAT THIS BOUNDS. One corpus, and a program written around closures would read
differently -- the corpus makes 440 closure calls against 1,056,329 lookups, so
it is barely exercising the case. What the figure does bound is the thing that
matters here: the row this change is for is measured on THIS corpus, so the
change that moves the row needs no upvalue resolution. A program that leans on
closures would keep the walk it has today and lose nothing it has now.

## 2026-09-18 — the static depth is right on 656,939 hits and wrong on none

The pass that resolving a local to an index needs was written as a PROBE rather
than a pass: it works out what depth each `Ident` site would resolve to, and
`lookup` then compares that answer with the depth its own walk reaches. So the
run says whether the static answer is right, instead of the code looking right.

The probe walks each declaration with a scope that mirrors the chain --
parameters deepest in `match_params` order, then each `Stmt::Bind`'s binders on
top, a lambda's parameters above its capture, a `Block` or a `Build` in
expression position scoped to itself, a `Build` in statement position escaping
into its parent, and a guard's `rest` continuing on the same scope. A later
binding of the same name shadows an earlier one, so the search runs from the
top.

Over bench/interp_corpus:

    ident sites in the program                 1,757
    sites resolving to a local                 1,098    62.5%

    hits where the static depth AGREED       656,939    90.7% of all hits
    hits where it DISAGREED                        0
    misses at a site it called a local             0

NOT ONE WRONG DEPTH, AND NOT ONE NAME CALLED A LOCAL THAT WAS NOT ONE. The
remaining 67,365 hits are sites the probe has no opinion on, and the reason is
the probe's key rather than the idea: a `Span` is a line and a column with no
file, so two modules collide, and 577 keys are poisoned to keep the check sound.
A pass keyed by site identity has no such loss.

THE LAST NUMBER IS THE ONE THAT NEEDED MEASURING. A first draft reported 18,695
misses at sites it had called locals, which would have been a wrong answer in a
real pass. Every one was a collision: a resolving site in one module sharing a
line and column with a global read in another. Poisoning a key the moment ANY
site that does not resolve touches it took the figure to zero, and it is zero
because the ambiguity is gone rather than because the count was quietly dropped
-- the poisoned keys rose from 15 to 577 in the same run and the agreed hits
fell from 689,940 to 656,939 to pay for it.

What remains is representation. The depth is provably available; where the index
lives on the node -- a new `Expr` variant written by a rewrite at load, against a
cell on `Expr::Ident` -- is the next question, and it is the one the build
starts from.

## 2026-09-18 — the slot lead is dead, and it died of the error this branch corrected

The entries above corrected the 19.3 million by showing that the miss walks carry
almost none of the `memcmp`, and then re-sized the slot lead UPWARD on the
grounds that the 47.2 million left over must be in the hit walks. The same
mistake, one level up. The hit walks carry 1,335,388 of it.

BUILT, to find that out. A per-site inline cache: the address of the `Name` in
each `Ident` node keys a direct-mapped table of 4,096 entries holding the depth
that site's local was found at last time. The walk jumps straight to the
remembered depth, checks the name it lands on and falls back on a mismatch, so a
stale entry costs one comparison and can never answer wrongly. It needs no
static pass and no AST change; the depth being static was measured first and
then deliberately not relied on.

It is correct -- the corpus and the shadowing fixture print byte-identical
output -- and it costs 31,209,169, a rise of 2.80%. What matters is not the sign
but which counters moved:

    eval_ident      115,147,991 -> 152,742,778     +37,594,787
    dispatch        130,726,726 -> 130,726,726      byte-identical
    match_one        68,846,453 ->  68,846,453      byte-identical
    memcmp           48,165,663 ->  46,830,275      -1,335,388

THE CACHE REMOVES ESSENTIALLY THE WHOLE HIT WALK AND TAKES 1,335,388 OFF
`memcmp`. With the 956,485 the shape filter took off the miss walks, the entire
environment walk accounts for 2,291,873 of the 48,165,663 -- 4.8% of the figure,
and 0.205% of the interpreted row.

`match_one` is byte-identical at 68,846,453 in every build here and calls
`memcmp` 733,821 times against `eval_ident`'s 724,304. The bulk of the 48 million
is PATTERN MATCHING, and it always was. The walk arrived beside it in the
profile and three successive claims priced the walk at the whole of it: section
90's forty million, this branch's 47.2 million, and the sentence in section 90
saying the ceiling rises.

WHERE THE CLAIM TRAVELLED, and what is corrected: section 90 on main (the forty
million and the correction added to it today), section 91 on this branch, this
branch's body, and kanso#1530's body, which cites the 47.2 million as the reason
its fixture exists. The fixture is still worth having -- it pins that a local
resolves to its own depth, which nothing else did -- but not for that reason.

WHAT IS ACTUALLY LEFT. `match_one` is 5.8% of the run and carries roughly half
the `memcmp` calls; that is the lead the walk was standing in front of, and it
has not been looked at. Nothing here sizes it, deliberately: sizing a lead by
dividing a counter among its callers is what produced three wrong numbers in one
day, and the next figure written down about `match_one` should come from a
differential rather than a division.

## 2026-09-18 — withdrawn: the cache kept the expensive comparison, so it never priced the walk

The entry above says the per-site cache "removes essentially the whole hit walk"
and concludes from 1,335,388 that the environment walk is 0.205% of the row and
the slot lead is dead. The first clause is wrong and the rest follows from it.

The cache jumps to the remembered depth and then CHECKS THE NAME IT LANDS ON.
That check is a comparison of two equal-length names, which is the case that
reaches libc and runs the full twenty-two bytes. What the cache removed is the
redundant comparisons on the way down; what it kept is the confirming one, and
that is the expensive one.

A debuginfo build says so directly. `eval_ident'2` calls `memcmp` 724,304 times
-- exactly the number of hits, so ONE call per hit, and the 866,429 non-matching
visits never reach libc at all, which is the same length-rejection the misses
showed. `match_one` calls it 768,153 times. So the walk's `memcmp` is one full
comparison per hit and the cache preserved every one of them.

WITHDRAWN: that the whole environment walk is 2,291,873, that it is 0.205% of
the row, and that the slot lead is dead. What stands is the shape filter's
956,485 for the misses, the cache's 1,335,388 for the redundant comparisons, and
the cache's own cost of 31,209,169. What a real slot is worth is UNMEASURED.

AND THE ATTEMPT TO MEASURE IT FOUND SOMETHING ELSE. A ceiling variant that
trusts the remembered depth without re-checking the name raised `is not callable`
on the corpus. The depth is not the problem -- 656,939 hits agreed with a static
depth and none disagreed. The KEY is: `Stmt::Bind`'s lazy path clones the whole
`Expr` into the thunk, at three sites, so expression nodes are allocated and
dropped throughout the run and A NODE'S ADDRESS IS NOT A SITE IDENTITY. A
recycled address compares equal to a live entry. The checking cache survives
that because the name comparison rejects the stale answer; the trusting one
answered from a recycled entry.

So the check that made the cache expensive is also the check that made it
correct, and a slot scheme wanting to drop it needs a key the AST owns rather
than one the allocator hands out. That is a real constraint on the design and it
was not visible before something was built on the wrong key.

Four claims about this one counter have now been withdrawn in a day -- 19.3
million, forty million, 47.2 million, and 0.205%. Every one divided a measured
total by a measured count. The two figures that have survived, 956,485 and
1,335,388, were each read off the same counter in two binaries that differed by
one change.

## 2026-09-18 — match_one's comparisons are not in the arms that compare names

Before sizing `match_one` by removing something, the obvious target was the arms
that compare type names, so they were counted first. Over bench/interp_corpus:

    match_one calls           1,135,058
      Ctor against Record        29,052     2.6%
      Ctor against Sub                0
      Nullary                    76,408     6.7%
      StrLit                          0
      Annotated                  59,430     5.2%

Those are 164,890 of 1,135,058 — 14.5%. The other 85% are `Var` and `Wildcard`,
which compare nothing at all. And the debuginfo profile puts `match_one`'s
`memcmp` calls at 768,153, which those arms cannot account for on any per-call
cost.

So the target was wrong, and interning the constructor type names -- which was
the plan -- would reach 29,052 dispatches. Recorded as a negative result rather
than acted on.

WHERE THE CALLS ARE IS OPEN, and the next step is separation rather than another
guess: `match_one` is a profile frame, not a function, and `bind_whole`,
`type_match_depth` and whatever else is inlined into it are counted as part of
it. Nothing here says which of them calls libc. A `#[inline(never)]` on each
candidate, one build, would split the frame and name it, and that is a
measurement rather than a division.

## 2026-09-18 — the withdrawal is withdrawn, and the walk's cost is the clone

The entry above withdrew the 2,291,873 on the strength of a debuginfo profile
that appeared to show `eval_ident` calling `memcmp` 724,304 times. It does not.
That reading was wrong in a way worth naming exactly, because the tool invites
it: in `callgrind_annotate --tree=caller`, the `<` caller lines come BEFORE the
`*` entry they belong to. I read a block as belonging to the entry above it, so
`Value::clone`'s callers were read as `memcmp`'s.

The annotated source settles it and needs no interpretation. Inside `lookup`:

      909,375   if frame.name.as_str() == name {
    2,897,216       return Some(frame.value.clone());
   31,514,002   => <kanso::eval::Value as Clone>::clone (724,304x)

EVERY COMPARISON THE ENVIRONMENT WALK MAKES, over all 2,662,536 frame visits,
COSTS 909,375. The two differentials agree with it and always did: removing
every miss visit took 956,485 off `memcmp`, removing the redundant hit compares
took 1,335,388. Three readings of the same quantity, by three routes, all in the
same million.

So the 2,291,873 stands, the slot lead is dead for the reason first given, and
the entry above it is withdrawn in full. Also withdrawn: that `match_one` calls
`memcmp` 768,153 times. That was `Value::clone` again, from
`binds.push((name.clone(), arg.clone()))`.

AND THE MISREADING PRODUCED THE FIRST WELL-FOUNDED NUMBER OF THE WHOLE THREAD.
The walk's expense is not what it compares; it is what it returns. The clone on
the hit costs **31,514,002** across 724,304 hits, forty-three instructions each,
against 909,375 for every comparison in the run. Thirty-five to one.

A slot index does not touch that. It removes the visiting and the comparing,
which together are worth about two and a quarter million, and leaves the clone
exactly where it is -- which is why every scheme tried today lost: they were all
aimed at the cheap half. What would touch it is `lookup` answering a REFERENCE
into the frame rather than a clone, so a caller that only reads pays nothing;
`eval_ident` returns `EvalResult` by value, so that is a real change to its
signature and its callers rather than a local trick.

Unsized on purpose. 31,514,002 is what the clone costs, not what removing it
would save, and the difference between those two is the thing this day has been
about.

THE METHOD, since four figures went wrong before it was found: the annotated
source (`callgrind_annotate --auto=yes`, run where the recorded relative paths
resolve) gives a cost per LINE and per CALL SITE. It is the only reading here
that has not been wrong. The caller tree invites an off-by-one, and a total
divided by a count is not a measurement.

## 2026-09-18 — what a Value clone actually copies

The walk's expense is `Value::clone` at the hit, 31,514,002 over 724,304 calls.
Reading what that clone does narrows it further, and the enum answers most of it
by inspection: of `Value`'s variants, `Map`, `ErrV`, `List`, `Bytes`, `Record`,
`Sub`, `FnRef` and `Closure` are all behind an `Rc`, so cloning them is a
refcount bump; `Float`, `True`, `False`, `NoneV` and `Done` are copies of
nothing. TWO VARIANTS ALLOCATE: `Str(String)` and `Int(BigInt)`.

The annotated source prices the first of those. Inside `Value::clone`:

    12,872,332   => <String as Clone>::clone   (137,732x)

Ninety-three instructions a clone, and that is every `Value::Str` copied
anywhere in the run rather than only the ones the walk makes -- the reading is
per call site inside `Value::clone`, which has many callers. So it is a bound on
one component of the 31.5 million, not a share of it, and it is written down
that way on purpose.

WHAT IT SUGGESTS is that the interpreter copies string BODIES when it copies
values, where every other compound variant it holds is shared. `Rc<str>` would
turn a ninety-three-instruction allocating clone into a bump.

WHAT IT COSTS IS THE REASON NOT TO ASSUME. A `String` is what gets APPENDED to,
and `Rc<str>` cannot be appended in place -- so the trade is a cheaper copy
against a more expensive build, and which way it comes out is a property of how
much this corpus concatenates versus how much it copies. That is a build and a
differential, not an argument. kanso#1515 and kanso#1520 are both about building
strings and lists in place, so the appending side is not hypothetical.

Recorded as the lead the whole day was looking for, unsized deliberately.

## 2026-09-18 — half of every Value clone allocates, and the integers are four times the strings

The entry above reasoned about `Value::Str` and put a trade beside it: `Rc<str>`
buys a cheaper copy at the price of a more expensive build, because a `String`
is what gets appended to. THAT TRADE IS WRONG FOR THIS CODEBASE and the fix was
one grep. `append` takes `Value::Bytes` -- "append takes bytes and a string,
bytes, or byte" -- and grows an `Rc<Vec<u8>>`. Nothing appends to a
`Value::Str`; a string is built as a local `String`, wrapped once, and copied
thereafter. kanso#1515 and kanso#1520 are about `Bytes` and `List`, not this.

So the counts were taken instead of reasoned about. A `Clone` impl written by
hand in place of the derive, counting per variant, over bench/interp_corpus:

    Value clones        1,555,866
      Str                 137,732     8.9%   566,128 bytes, 4.1 per clone
      Int                 610,763    39.3%
      everything else     807,371    51.9%   an Rc bump or a copy of nothing

HALF OF EVERY CLONE ALLOCATES, AND THE INTEGERS ARE FOUR AND A HALF TIMES THE
STRINGS. `Value::Int` holds a `BigInt`, whose clone allocates a digit vector; on
this corpus that is 610,763 heap allocations for numbers that are almost all
small. The strings average FOUR BYTES, so `String::clone`'s ninety-three
instructions there are the allocator rather than the copying.

The interp vein's allocation counter reads 2,486,376. These two variants are
748,495 of whatever that counts -- adjacent figures from different instruments,
so no arithmetic is done between them here.

WHAT THIS MAKES THE LEAD. A small-integer representation -- an inline `i64` with
`BigInt` only when it overflows -- removes 610,763 allocations, and is a change
to `Value` rather than to any one path, so every clone in the run pays less, not
only the environment walk's. `Rc<str>` is the same shape and a quarter the size.

UNSIZED, and this is the fourth time today that mattered: 610,763 is how often
the allocation happens, not what removing it saves. The build is the measurement.

## 2026-09-18 — every integer the run clones fits in an i64

The counting `Clone` impl was extended to ask, of each `Value::Int` it copied,
whether the `BigInt` fits an `i64`. Over bench/interp_corpus:

    Int clones            610,763
    of those, fitting i64 610,763      all of them

NOT ONE. Every integer this corpus copies is small, and each of those copies
allocates a digit vector because `BigInt` keeps its magnitude on the heap.

That settles the shape of the change without settling its size: an inline `i64`
with `BigInt` reached only on overflow removes 610,763 allocations here and
falls back never. It is a change to `Value` rather than to a path, so it pays
wherever a value is copied -- the environment walk's 724,304 hits, `match_one`'s
binds, every argument handed to a call -- rather than at one site.

WHAT IT COSTS IS THE PART TO BUILD RATHER THAN ARGUE. `Value` gains a variant or
`Int` gains a discriminant, and every arithmetic site has to promote at exactly
the right point. The language's integers are arbitrary-precision by design, and
a fast path that overflows one step late is a wrong answer rather than a slow
one, so the fixture comes first: a golden holding a value either side of the
i64 boundary, watched red on a deliberately-wrong promotion.

Still unsized, deliberately. 610,763 is how often the allocation happens. What
removing it saves is a differential nobody has run.

## 2026-09-18 — the interpreter's integer boundary has no home in the corpus

The small-integer change needs a fixture before it needs code, so one was
written: `9223372036854775807 + 1`, `-9223372036854775808 - 1`, and three
products that cross the boundary from operands that do not. Running it found
the constraint the build would otherwise have met late.

NATIVE REFUSES THERE. `error[runtime]: integer overflow (int64 native build;
spec int is arbitrary precision)`. The interpreter answers exactly:

    9223372036854775808 -9223372036854775809 18446744073709551614 -18446744073709551616
    9223372037000250000 18446744073709551614 -18446744073709551616

That divergence is sanctioned and already pinned -- the differential law allows
an engine to REJECT what another accepts provided the diagnostic is clear, and
`docs/book/ch02.html` with `docs/book/samples/ch02/overflow.out` carries it.

WHAT IS NOT PINNED is the second line: the interpreter's own answers at the
boundary. The micro corpus runs every fixture on both engines and requires them
to agree, so it cannot hold a program native refuses, and there is no other home
for an interpreter-only behavioural golden. So the exact place an inline `i64`
fast path would go wrong -- promoting one step late, and printing a WRAPPED
number rather than raising -- is a place nothing in the tree currently watches.

That is the finding, and it is a gap in the corpus rather than a fact about
integers. The change needs a home for it first: either a fixture kind that pins
one engine's answer where another refuses, or the native side gaining
arbitrary precision so the two agree and an ordinary micro golden works. Which
of those the project wants is a question rather than an implementation detail,
so it goes to the ledger rather than being decided here.

Recorded before any code, which is the whole point of writing the fixture first.

## 2026-09-18 — what the walk's own clones copy, which orders the two leads

Two leads came out of the walk's 31,514,002 of `Value::clone`: answering a
REFERENCE into the frame rather than a copy, and making small integers inline.
Which to build first is a question the counters answer directly, by asking what
the walk's 724,304 hits actually clone:

    Int          297,244    41.0%     allocates a digit vector
    Str           68,866     9.5%     allocates
    Rc-backed    331,740    45.8%     a refcount bump
    flat          26,454     3.7%     copies nothing

    allocating   366,110    50.5%

HALF THE WALK'S CLONES ALLOCATE, AND THE INTEGERS ARE FOUR TIMES THE STRINGS
here as they are across the run. So the two leads are complementary rather than
competing, and the order is settled by which covers more:

Small integers reach 41.0% of the walk's clones and, being a change to `Value`
itself, the same 39.3% of every OTHER clone in the run -- `match_one`'s binds,
every argument handed to a call. It is one representation change with one
correctness question (promote at exactly the right step).

A reference return reaches the 45.8% that are already only a refcount bump,
where the saving per clone is smallest, and it costs a signature change through
`eval_ident` and its callers, several of which genuinely need an owned value.

So integers first, and the reference return is worth re-sizing only after,
against whatever the row then reads. Neither is sized: these are counts of how
often, not measurements of what removing them saves, and that distinction is
the one this day was about.
