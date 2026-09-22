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

## 2026-09-18 — what a small integer costs to copy, and a benchmark that measured nothing first

Making `Value::Int` an inline machine word is the ordered-first lead, and
before 90 call sites are touched it is worth knowing what the copy it removes
costs. `BigInt::clone` is inlined into `Value::clone` and lives in another
crate, so `#[inline(never)]` cannot split it and the annotated source cannot
price it the way it priced `String::clone`. A microbenchmark can.

THE FIRST ONE MEASURED NOTHING, and the check that caught it took one extra
run: doubling the count moved the total by FIFTEEN instructions. Cloning a
constant and reading its bit count is loop-invariant, so LLVM hoisted the whole
body; `#[inline(never)]` on the enclosing function does not stop it optimising
inside. `black_box` on the input, the clone and the drop fixed it. A benchmark
whose figure does not move with its count is measuring its own overhead, and
varying the count is how that is found out rather than assumed.

With the loop defeated, over mimalloc as the interpreter uses:

    n =   100,000     4,436,432
    n =   610,763    22,313,150
    n = 1,221,526    43,689,870

The slope is 35.00 instructions per iteration from the two largest points and
35.00 from the two smallest, with 936,430 of fixed overhead -- which matches
what the hoisted version read, so the constant is the process and the slope is
the work.

    over the walk's 297,244 integer hits     ~10.4 million
    over all 610,763 integer clones          ~21.4 million

WHAT THAT IS AND IS NOT. The 35 covers a clone, a `bits()` read, a drop and
three `black_box`es, so the clone alone is less: it BOUNDS clone-and-drop from
above. And a microbenchmark allocating and freeing in a tight loop is the
allocator's best case; a real run interleaves and frees later, which could push
the true figure up. So this bounds one component of `Value::clone`'s 46,791,170
of self cost, and it is not a prediction of what the change saves.

Beside the strings, which the annotated source priced directly at 12,872,332
over 137,732 clones, the allocating half of every value copy in this run is
somewhere around 34 million against an interpreted row of 1.075 billion. Three
per cent, bounded from above, for a change to one type.

That is the number the corpus question in the ledger is worth answering for --
or not. Recorded so the answer can be weighed rather than guessed.

## 2026-09-18 — one line copies a whole expression tree, fifty thousand times

Reading the annotated source for the integer work turned up a bigger and much
narrower lead beside it. `Expr::clone` in the interpreted run:

    body: arg.clone()          11,471,717     50,235 calls     228 each
    a second site                  77,936        221 calls
    expr: expr.clone()                285          1 call

ONE LINE CARRIES 99.3% OF IT. It is the `lazy_if` path, building a deferred
argument as a closure with no parameters:

    values.push(Value::Closure(Rc::new(ClosureData {
        params: Vec::new(),
        body: arg.clone(),
        ...

`ClosureData.body` is an owned `Expr`, so every deferral copies the argument's
whole subtree. 228 instructions a copy, 11,471,717 in all, which is 1.03% of
this box's interpreted row.

WHY IT IS NOT A FIVE-LINE FIX, checked rather than assumed. `ClosureData` has
five constructions and three readers, all of the form
`self.eval(&closure.body, ...)`, which would take an `Rc<Expr>` unchanged. But
the clone is of an `Expr` the AST owns, so wrapping the FIELD in an `Rc` still
copies once to build the `Rc`. The copy goes away only if the thing being
cloned is already shared -- `App { args: Vec<Rc<Expr>> }` in the AST, built once
by the parser.

That is a narrow AST change: one field of one variant, and the parser is the
only thing that constructs it. No semantics move, nothing is promoted, and
unlike the integer lead there is no corpus question in front of it -- a deferred
argument evaluates to the same value however its expression is stored, and the
existing differential goldens already say so on every engine.

Sized from the annotated source, which is the instrument that has been right;
unsized as a saving, which is the distinction this day was about. 11,471,717 is
what the copying costs, not what removing it returns.


## 2026-09-18 — `if` in value position built three closures to use one

`eval_tail` has always handled `if` directly: ask the condition, then evaluate
in tail position the branch that answer chooses. `eval` never did. In value
position every argument to `if` was wrapped in a nullary closure -- the whole
`Expr` subtree copied, the environment and the frame cloned, an `Rc` allocated
-- and all three handed to `builtin_if`, which forces the condition on the way
in and then the one branch it picks. TWO OF THE THREE WERE BUILT TO BE THROWN
AWAY AND THE THIRD TO BE OPENED IMMEDIATELY.

The value path now mirrors the tail path, with `force` and `force_thunk` doing
what `builtin_if` did:

    base       1,115,996,775
    mirrored   1,070,343,718     -45,653,057     -4.09%

The annotated source had put the subtree copying alone at 11,471,717 over
50,235 clones, and one line of it carried 99.3%. The rest of the fall is what
was built and opened around those copies.

MEASURED IN TWO STEPS, because the first one was too timid. Making only the
CONDITION strict -- leaving the branches wrapped -- read 1,106,899,444, a fall
of 9,097,331. Reading `eval_tail` afterwards showed the shape the value path
should have had all along, and mirroring it is five times the change.

NOTHING OBSERVABLE MOVES, checked rather than assumed. The corpus prints the
same answer. A failure VALUE is still the answer rather than a fault. A
condition that cannot be evaluated prints byte-identically on both engines
before and after: `error[value]: division by zero`, from the same module.

AND NO BEHAVIOURAL FIXTURE IS POSSIBLE, which is the more useful half of this
entry and was found by breaking the code rather than by reasoning. A golden was
written where the untaken branch PRINTS, on the view that a strict `if` would
announce itself. The code was then broken to evaluate all three arguments, and
the fixture PASSED.

Effects in this language are VALUES. Evaluating `print "ran skipped"` builds an
effect; it performs nothing until something sequences it, and the untaken branch
is never sequenced. A failing branch is the same -- its failure is a value, and
`builtin_if` discards the branch it did not choose. So `if`'s laziness is not
observable by any terminating program: what it buys is COST and TERMINATION.

That is why this ships with the cost golden and no behavioural fixture, and why
the one that was written was discarded rather than committed. A golden that
passes with the rule removed is worse than none, because it stops anybody
looking.

## 2026-09-18 — two tests shared one staging directory, and cargo runs them at once

`a_unique_container_is_extended_in_place` went red on kanso#1502's Linux job
and kanso#1529's macOS job within an hour of each other. Neither diff touched
the interpreter; kanso#1502's touches `src/runtime.c`, which is the NATIVE
runtime, and this fixture measures the interpreted one. Both were read as the
fixture being flaky and left.

The panic says what it is, and reading it was the whole diagnosis:

    the interpreted run failed:
    error[name]: unknown name `builders/run`
      --> /tmp/kanso-unique-container/run_300.kso:3:1

The library is THERE and has nothing in it. `std::fs::write` is a
`File::create` followed by a `write_all`, and `File::create` truncates. Both
tests in the file staged into one hardcoded path, cargo runs the tests in a
binary on parallel threads, so one test's staging truncated the library while
the other test's `kanso` was reading it. A zero-byte library is a valid
library with no definitions in it, which is why the run got as far as
resolving a name.

THE FAILING TEST IS NOT THE ONE THE FILE IS NAMED FOR. Both rounds were
recorded as `a_unique_container_is_extended_in_place`, because that is the
cargo TARGET, and the target is the file. The test that actually died is
`the_builders_answer_what_they_answered_before` — the other one passed in the
same run. Read the failure block, not the target list.

Each test stages its own tree now, named for a tag it passes. The tags are all
the same length so a reading under one is comparable with a reading under
another, and the directory's own name growing by seven characters moved the
absolute counts and left `PER_EXTRA_ROUND` at 7,803 — the subtraction doing
exactly what this fixture's header says it is for.

TWENTY-FIVE RUNS OF THE PAIR ON AN IDLE BOX DID NOT REPRODUCE IT, which is
why the spec forces the interleaving rather than waiting for it. `ran` no
longer stages; it takes a tree that is already staged. That split is not
tidiness — the first version of the spec truncated one tree's library and then
called `ran`, which re-wrote the library on the way in and healed the very
window being tested, so it PASSED against the shared directory. Moving the
staging out is what makes the window stay open across the read.

Watched red by making `staged` ignore its tag, which is the shared directory
this file used to have, and the message is CI's byte for byte.

This is the 2026-09-15 rule read from the other side. External state gets
normalized before it is measured; a directory two threads write is not
normalized and is nobody's.

## 2026-09-18 — the dispatcher copied a name it was already holding a share of

With the value-position `if` measured and on its way to main, a fresh profile
of the interpreted run put `dispatch` at the top: 130,726,726 instructions of
self cost, 11.70% of the program. Two of its callee counts stood out, and one
of them explained itself immediately.

    <str as Display>::fmt      119,535 calls     23,936,233 instructions
    the tail-hop line          119,542 executions

The same count twice. `dispatch_loop` held its function name as a `String`,
and the trampoline's hop wrote `name = next.to_string()` where `next` is the
`Rc<str>` that `Flow::Tail` already carries. `to_string` goes through
`ToString`, which goes through `Display`, which goes through the formatting
machinery: two hundred instructions and an allocation to copy a name that was
one pointer away.

`dispatch_loop` takes an `Rc<str>` now. The hop is a move. Every use of the
name inside the loop was already `&name` or `&*name` — `err_reader`,
`self.fns.get(&*next)`, `hop`, `getter_field`, `spoken` — so nothing else
changed.

    base       1,115,996,775
    shared     1,089,450,665     -26,546,110     -2.38%

The corpus answers `interp 59442` on both binaries, byte for byte.

READING THE PROFILE WITHOUT DEBUG INFO WOULD NOT HAVE FOUND IT. The release
build's frames annotate as `???`, so there is a cost per function and nothing
per line. Rebuilding with `CARGO_PROFILE_RELEASE_DEBUG=1` is what put the hop
and the formatter side by side on the same count, and the agreement of two
independently derived numbers is what made it a finding rather than a guess.

THIS IS THE THIRD OF ITS KIND. kanso#1516 gave `eval_ident` a memory so a name
resolved once rather than per mention; kanso#1522 stopped a binding copying its
name twice. Each time the name was already owned somewhere and was rebuilt
anyway. Worth looking for the fourth.

## 2026-09-18 — the candidate loop allocated for arms it was about to reject

The same profile put `__rust_alloc` at 1,471,992 calls from `dispatch` and
`__rust_dealloc` at 1,089,150. `match_params` built `score` and `binds` with
`Vec::with_capacity` on every overload candidate and returned `None` the moment
`match_one` refused, so a candidate that failed on its first parameter had
already paid for two allocations. Arm selection tries every overload in the
group and keeps one. 175,246 dispatches at about four candidates each, two
vectors apiece, is the call count almost exactly.

One pair of buffers now serves the whole candidate list, cleared between
candidates, and `match_params_into` answers whether the candidate matched
rather than handing back vectors. When a candidate wins, the outgoing best's
vectors become the working pair, so the winner hands its capacity on instead of
leaving the next candidate to grow from nothing.

    base       1,115,996,775
    buffered   1,109,925,182      -6,071,593     -0.54%
    __rust_alloc  36,857,970 -> 29,713,800      -7,144,170    -19.4%

THE ALLOCATOR LOST A FIFTH OF ITS WORK AND THE ROW MOVED LESS THAN THAT. The
two numbers differ by about a million, which is what the buffer bookkeeping
costs back: a `clear()` per candidate, and two `mem::replace` on every
candidate that wins. Both readings are worth keeping, because the one that
matters for the objective is the row and the one that says the change did what
it was meant to is the allocator.

AND THE ATTRIBUTION OVERSHOT. 65,283,877 was `__rust_alloc` INCLUSIVE from
`dispatch` -- everything under it, `mi_malloc` and the rest -- where 36,857,970
is that function's own self cost across the whole program. Reading the first as
a budget for the second is the kind of arithmetic this log has withdrawn five
figures for. What the change was worth is the differential, and the
differential is 6,071,593.

## 2026-09-18 — a merge loop committed conflict markers, and the check that missed them

Four branches took a merge of main in one loop. The loop resolved
design/compiler-log.md, ran `git add -u`, committed and pushed. Three of the
four went out carrying `<<<<<<<` in files the loop had never looked at:
`bench/welfare_floor.json` on two of them and
`bench/interp_instructions_golden.txt` on two.

CI found it in the shape the file's own reader would:

    error[endpoint]: unhandled err reached the executor:
    json/parse_failure 1 "unexpected character `<`"
      born in json/fail at std/json/scan.kso:2

THE CHECK WAS REAL AND LOOKED AT ONE FILE. After resolving, the loop counted
markers in design/compiler-log.md and printed the count. It read zero, which
was true, and said nothing about the two bench files git had also left
conflicted. A verification that names the file it verifies will keep passing
while the defect moves one directory over. Count markers across the whole
tree, or let the thing that reads the file read it -- here, running
`kanso run scripts/welfare` would have failed instantly on all three.

AND THE FIRST FIX WAS WORSE THAN THE SECOND. `git checkout origin/main --
bench/interp_instructions_golden.txt` clears the markers and takes the row,
and it silently dropped kanso#1502's own header note recording its sitting.
The same move on kanso#1513 would have dropped its note explaining why a
branch that adds no interpreter code moves the interpreter's row. That file's
history IS its comments, which is what the standing rule against blanket
resolution is protecting. Both were resolved hunk by hunk instead: main's
value on the conflicted line, every comment kept.

`git add -u` after a merge stages whatever git left behind, including what it
could not merge. The rule already written down is to scope adds to the paths
a change owns; the addition here is that a loop doing it across several
branches turns one slip into three.

## 2026-09-18 — the peak rose 205 bytes while the count fell a quarter

CI's rows for the two dispatcher changes, and one of them goes the other way:

    interp_allocs       2,328,213 -> 1,740,991    -587,222    -25.2%
    interp_peak_bytes     833,128 ->   833,333        +205     +0.02%

`interp_peak_bytes` is priced here because the trend gate asked and because
the direction deserves a sentence rather than a shrug. A quarter of the run's
allocations stop happening and its high-water mark goes UP by two hundred
bytes.

THE LIKELY READING, and it is a reading rather than a finding: one pair of
candidate buffers now serves a whole overload group, so `binds` keeps whatever
capacity the widest arm in that group reached and holds it for the length of
the dispatch. Per-candidate vectors were freed at whatever size each arm
needed. Fewer, larger, longer-lived beats more, smaller, shorter-lived on the
count and can lose on the peak.

NOTHING ISOLATES THAT, AND THE FIRST DRAFT OF THIS PARAGRAPH GAVE THE WRONG
REASON. It said 205 bytes was below what a differential on this box could
separate from the arena's rounding. That was a guess about the instrument, and
the instrument was one command away: three runs of the same binary print
`interp_peak_bytes=834117` three times, byte for byte. The counter is the
program's own arena accounting and it is deterministic, so 205 bytes is
perfectly separable. What is missing is not resolution but an ISOLATION -- a
build per hypothesis -- and that is a different kind of cost.

So the mechanism stays open because nobody has spent that, not because the
number is too small to see. The golden's header says the same. What is not in
doubt is the trade: 587,222 allocations against 205 bytes, and the objective
weighs both.

## 2026-09-18 — a call's parameters bound in one frame instead of one each

`bind` was `Some(Rc::new(Env { name, value, parent: env }))`: one heap node per
BINDING. A call with four parameters pushed four nodes, and every name the body
mentioned walked past all four to reach the caller's scope. Three frames in the
interpreted profile are that decomposition:

    eval_ident, self cost          115,147,991   10.61%
    Rc::drop_slow                   45,932,054    4.23%   229,625 calls
    memcmp                          47,672,229    4.39%

2,662,536 frame visits served 724,304 hits, 3.7 nodes a lookup.

`Env` is an enum now. `One(Name, Value, parent)` is a single name bound on its
own — a lazy thunk, a pattern variable, a `build` step. `Many(Bindings, parent)`
is a whole call's parameters. `Many` scans its slots in REVERSE, which is what
keeps shadowing the same: binding a, b, c as three `One` frames leaves c
outermost, so one frame holding them has to answer c first.

    base       1,038,405,822
    grouped    1,022,961,605     -15,444,217     -1.49%
    __rust_alloc  25,548,225 -> 20,621,100        -19.3%

The corpus prints `interp 59442` on both binaries.

IT COMPOSES WITH THE CANDIDATE BUFFERS UNDERNEATH IT, and that is most of why
it is cheap. Arm selection fills one `binds` vector and the winner hands it on;
`bind_all` moves that same vector into the frame the body runs in. The
allocation selection already paid for becomes the scope, rather than being
freed and replaced by one node per parameter. A call that binds nothing pushes
no frame at all, so a chain never carries an empty node for the walk to step
over.

ENV WAS FULLY ENCAPSULATED, which is the only reason this is a small diff: one
constructor and one reader, `bind` and `lookup`, and nothing outside eval.rs
names the type. Three functions and the dispatch site.

WHY THIS ONE AND NOT THE FOUR kanso#1529 DECLINED. The head byte, the padded
key, the shape filter and the per-site cache all aimed at the COMPARING and
left the structure alone; all four cost more than they saved. This aims at the
node count, which is the allocation count and the walk depth at once. That is a
reason to try it rather than a prediction, and the four that failed looked
sound too — what settles it is the differential above.

The allocation fixture re-reads 7,803 -> 6,003 -> 5,403 across the two changes.
The last two are `grow acc n` and `stack xs n`, each taking two parameters and
each called once a round.

## 2026-09-18 — what the grouped frame did not touch, which is the useful half

A profile of the interpreted run with the frame change in, taken on a release
build with debug info so the frames carry names rather than `???`:

    eval                    68,919,538   6.44%
    memcmp                  47,631,349   4.45%
    Value::clone            47,204,366   4.41%
    match_one               44,851,891   4.19%
    drop_in_place<Value>    41,234,808   3.85%
    eval_ident              39,702,258   3.71%
    dispatch_loop           30,804,862   2.88%
    lookup                  30,732,822   2.87%

THE PROFILE IS FLAT NOW. The top frame is 6.44% where this morning's was
11.70%, and the two that led it — `dispatch_loop` at 121,568,733 and
`eval_ident` at 115,147,991 — are 30,804,862 and 39,702,258. `lookup` appears
as its own symbol because the enum match made it too big to inline; its
inclusive cost from `eval_ident` is 116,087,760 over 1,056,328 calls.

MEMCMP DID NOT MOVE: 47,672,229 before, 47,631,349 after, a difference of
41,000 on a counter of 47 million. That is the useful half of this reading.
Grouping a call's parameters into one frame removes NODES and ALLOCATIONS and
leaves the comparing exactly where it was, because the same names are compared
against the same query — they are laid out in a vector rather than strung
through a chain. Anyone reading the row fall and concluding the walk now
compares less would be wrong, and the number says so.

So what the walk costs after the structural change is two nearly equal halves:
comparing names, 47,631,349, and copying the value out, 47,204,366.
kanso#1529 built four schemes at the first half — a head byte, a padded key, a
shape filter, a per-site cache — and all four cost more than they saved. The
second half is what section 91 sized and nothing has touched.

Percentages here are of this binary's total, and a debug-info build is not the
one the goldens are measured on. The ordering is what this reading is for.

## 2026-09-18 — an inline hint on `lookup` bought exactly nothing

Making `Env` an enum made `lookup` too big for LLVM to inline on its own, and
the annotated source priced what that seemed to cost: 9,506,961 instructions
attributed to the `fn lookup(...)` line over 1,056,328 calls, and 2,656,200 to
its closing brace. Twelve million on what looked like call machinery, on a
function with two call sites and one of them cold.

`#[inline]`, and the same A/B that measured the frame change:

    without   1,022,961,605     allocations 20,621,100
    with      1,022,961,605     allocations 20,621,100

IDENTICAL TO THE INSTRUCTION. The binaries are not identical — the hinted one
is 56 bytes smaller and has a different sha — so the hint reached the compiler
and changed its output. It changed nothing this corpus pays for.

WHAT THAT CORRECTS. The twelve million is the function's own entry work, the
part that sets up the walk, attributed by the profiler to the first and last
lines because that is where the instructions live. Reading it as a call frame
waiting to be removed was a guess about what an attribution MEANS, and the
same guess this log has been wrong with before: a number on a line tells you
where instructions are, not what removing something would return.

The hint is reverted rather than kept. A no-op carrying a comment that claims
a reason is worse than no change at all, because the next reader believes it.

## 2026-09-18 — kanso#1534, CI's rows: the grouped frame on the runner

CI measured the environment change on the tree merged after kanso#1533, and
the three interpreted rows moved:

    interp_instructions   995,837,536 -> 975,944,763   -19,892,773  -1.9976%
    interp_allocs           1,740,991 ->   1,412,516      -328,475  -18.8670%
    interp_peak_bytes         833,333 ->     834,117          +784  +0.0941%

Every other row in the job is byte-identical to main: `compile_instructions`
35,550,010, `entry_instructions` 126,729,588, `library_instructions`
127,186,008, `emit_instructions` 51,617,476, `startup_instructions`
3,363,916, `compile_allocs` 27,313, both codegen rows. A change confined to
`src/eval.rs`'s runtime path moved no layout row at all this time, which is
worth recording beside the seven layout-only moves the compile row has shown
before: the prior that an edit to the compiler's own Rust usually moves it is
a prior, not a rule.

**THE TWO HOSTS DISAGREE ON THE SIZE AND AGREE ON THE DIRECTION.** This
container's isolation read the instruction fall at 15,444,217 against its own
base of 1,038,405,822; the runner reads 19,892,773 against 995,837,536. The
split is the one kanso#1520 and kanso#1522 already mapped: how many
allocations a change removes is a count the program decides and travels
between hosts, and what the removed work cost in instructions is the rustc
that built the binary and does not.

**`interp_peak_bytes` ROSE 784 AND LANDED ON 834,117, AND NOTHING HERE
EXPLAINS IT.** The count falls 18.87% on the same change. A peak is a
high-water mark where a count is a total, so the two are free to move apart:
what a grouped frame changes is the shape of what is live at the run's widest
moment. Which shape that is would take a build per hypothesis and none has
been run. The counter is deterministic — three runs of one binary printed
834,117 byte for byte on this container, and the runner printed the same
figure — so 784 is separable, and what is missing is the isolation rather
than the resolution. Recorded as measured, mechanism open.

This is the second peak rise in two changes recorded this way (kanso#1533's
was 205). Two unexplained rises against two large count falls is the point at
which the pair is worth a build of its own rather than another note; that is
a lead, not a conclusion.

## 2026-09-18 — the name lookup was carrying the frame of the path it does not take

Section 94 left the environment walk shallower and the profile re-ranked.
`eval_ident` came second at 115,147,991 instructions of its own, 10.61% of
the interpreted run, behind the dispatcher. The obvious guesses were both
wrong and both cost nothing to rule out, because the answer was in the same
file: the `names` map has used FxHash since src/hash.rs was written, so
hashing is not where the time goes, and `Named` is an enum of `Rc`s and unit
variants, so cloning one out of the map is a refcount.

The annotated source says where it goes, and it is not in the body at all:

    16,901,264 (1.58%)  fn eval_ident(&self, name: &str, span: Span, ...
     7,122,532 (0.67%)      if let Some(value) = lookup(env, name) {
     1,448,608 (0.14%)          return Ok(value);
       996,075 (0.09%)      match named {
     8,450,632 (0.79%)  }

25,351,896 instructions, 2.37% of the whole run, on the opening line and the
closing brace. A release build annotates as `???` without debug info, which
is why this took a second build: `CARGO_PROFILE_RELEASE_DEBUG=1`.

A name that a scope binds is answered by `lookup` and returns. A name that no
scope binds walks a `RefCell` borrow, a map probe, a `Named` match and, the
first time, the whole resolve ladder. Both paths were one function, so the
frame the second one needs was being built and torn down for the first one
too. Moving the second into `#[inline(never)] fn eval_global` leaves
`eval_ident` as a match over `lookup`'s answer.

    isolation, two binaries on one container
    base    1,022,880,858
    split   1,007,027,010   -15,853,848   -1.5499%

The base was read at two distinct paths of the SAME LENGTH and printed
1,022,880,858 both times, so the harness is stable and the count is not
reading a path. That check exists because the first pass ran the two arms in
`/tmp/ab-isbase` and `/tmp/ab-issplit`, which differ by one character, and
this row is known to move with path length. The effect is four orders of
magnitude larger than that confound could be; the point of re-running was to
know rather than to argue.

**THIS IS THE SAME SHAPE AS THE INLINE HINT THAT BOUGHT NOTHING, AND IT WENT
THE OTHER WAY.** Earlier today `#[inline]` on `lookup` left the row identical
to the instruction across two genuinely different binaries. The entry cost a
profile attributes to a function is not automatically call overhead waiting
to be removed -- there it was the function's own work, and the hint could not
touch it. What is different here is that the cost is a frame sized for a path
that is usually not taken, which splitting does remove. Neither outcome was
predictable from the attribution, which is the argument for building both.

## 2026-09-18 — the same split, one function up, costs 0.33%

The frame argument above says `eval_ident` was building a frame its common
path does not need. `eval` is the function above it, 92,306,061 instructions
of its own, with 16,623,552 on its signature line — the same shape, one
level up, and a bigger match: fifteen arms, several of which build maps,
lists and records while `Ident` and `Int` return in a few instructions.

Four arms moved out of line behind `#[inline(never)]`: `Upcast`, `MapLit`,
`Field`, `Index`. Same corpus, same harness, one binary each.

    base   1,022,880,858
    split  1,026,292,184   +3,411,326   +0.3335%

DECLINED. The base here is the same binary and the same reading as the row
above, 1,022,880,858, so the two experiments are directly comparable: the
split that helped bought 15,853,848 and this one costs 3,411,326.

What separates them is which arms are actually rare. `eval_ident`'s second
path is taken only by a name no scope binds, and in a corpus that decodes a
document most names are bound. `Field` and `Index` are how a decoded document
is read, so they are not the rare arms they look like in a source listing —
moving them out adds a call to a hot path and saves a frame that the `App`
arm, still inline, goes on requiring. Reading the match for which arm LOOKS
expensive picked the wrong four.

Three builds now on one hypothesis: an inline hint on `lookup` (no change at
all), the `eval_ident` split (-15,853,848), and this (+3,411,326). The
hypothesis "a profile's entry cost is a frame that can be removed" has been
right once in three. It is a reason to build, not a reason to expect.

## 2026-09-18 — kanso#1535, CI's row for the frame split

    interp_instructions   975,944,763 -> 957,583,234   -18,361,529  -1.8814%

And nothing else in the job moved at all. `compile_instructions` 35,550,010,
`entry_instructions` 126,729,588, `library_instructions` 127,186,008,
`emit_instructions` 51,617,476, `startup_instructions` 3,363,916,
`compile_allocs` 27,313, `interp_allocs` 1,412,516, `interp_peak_bytes`
834,117 — every one byte-identical to main. That is the second change today
confined to the runtime path that left every layout row alone, after
kanso#1534 did the same. The prior that an edit to the compiler's own Rust
usually moves `compile_instructions` has now missed twice in a row, which is
worth remembering next time it is offered as a reason.

This container projected 15,853,848 and the runner reads 18,361,529 — same
direction, 15.8% larger. Both hosts agreeing on direction and differing on
size is the split kanso#1520 and kanso#1522 mapped: a change that removes a
COUNT travels, and what the removed work cost in instructions is the rustc
that built the binary.

**The interpreted row today, every figure CI's own:** 1,075,174,600 at the
start of the day, then 1,029,696,275 (kanso#1531), 995,837,536 (kanso#1533),
975,944,763 (kanso#1534) and 957,583,234 here. A fall of 117,591,366, or
10.94%, over five merged changes, none of which changed what the interpreter
computes.

## 2026-09-18 — two more routes at the parameter copy, both declined, and what five builds say

Section 95 shipped the one win in a family of five. The other four are here
with their numbers, because an idea that is not written down as declined gets
built again.

**THE LINE THEY ALL AIM AT.** The annotated profile puts one push of
`match_one`'s `Var` arm at 723,255 `Value` clones for 29,241,307
instructions. Call counts read out of the callgrind file rather than inferred
from instruction ratios: `Value::clone` 1,555,866 calls, `match_one`
1,100,726, `lookup` 1,056,329, `eval_global` 332,024, `dispatch_loop'2`
55,711, the recursive `match_one'2` 34,332. So that one line makes 46.5% of
every clone in the run, and `lookup`'s hits — 1,056,329 less 332,024 — come
to 724,305, which lands on section 91's 724,304.

**ROUTE ONE: SCORE WITHOUT BINDING.** `match_one`, `bind_whole` and
`match_params_into` took a `bind: bool`; selection walked every candidate
with it false and the winner was walked a second time with it true.

    base    1,007,027,010
    split   1,059,328,533   +52,301,523   +5.19%

Declined. The winner's second walk costs more than every discarded clone
saved, and not only in clones: `match_one` carries 44,851,891 of its own
beside the 29,241,307 of copying, so doubling the winning arm's match swamps
it.

**ROUTE TWO: MOVE THE WINNER'S ARGUMENTS OUT.** No second walk. Matching
pushed a `Pick` — `Arg(usize)` for a parameter at the top of the list,
`Val(Value)` for a name bound inside a record — and the winner's picks became
bindings by `std::mem::replace(&mut args[i], Value::NoneV)`. The dispatcher
owns that vector and clears it a line later; its own comment already said
nothing below reads it.

    base    1,007,027,010
    moved   1,017,171,110   +10,144,100   +1.01%

Declined. A `Pick` is a tag plus a `Value`, so the hot push got WIDER, and
the winner's picks are walked into a freshly allocated `Bindings`, which is
an allocation and a walk per dispatch that did not exist before. Which of the
two dominates is not measured.

**SO THE LINE RESISTS BOTH.** The copy is 32 bytes and, for every variant but
`Int` and `Str`, a refcount. Deferring it costs more than that. Indexing it
costs more than that. 29,241,307 instructions is real and measured and, on
this evidence, is what copying a parameter is worth paying.

**FIVE BUILDS, ONE WIN.**

    inline hint on lookup                   0
    eval_ident frame split         -15,853,848   shipped as kanso#1535
    eval arm split                  +3,411,326   declined
    score without binding          +52,301,523   declined
    winner's arguments moved out   +10,144,100   declined

Each is two binaries on one container, one corpus, paths of the same length,
identical output. The win was the smallest change of the five.

**AND THE PROFILE IS FLAT, WHICH IS OLDER NEWS THAN IT LOOKED.** The first
draft of this entry put the post-split top frame at 7.70% against 11.70%
"this morning", which reads as the split having flattened it. Measuring the
same statistic on both sides instead:

    pre-split (1,070,153,314)     post-split (1,054,297,214)
      top 1   6.44%                 top 1   7.70%
      top 5  23.34%                 top 5  24.86%
      top 10 36.83%                 top 10 36.55%
      top 20 49.76%                 top 20 48.95%

Twenty functions to reach half the run, on both sides. The profile was
ALREADY flat before the split, and the split RAISED the top frame's share by
taking work out of `eval_ident` while `eval` stayed. The 11.70% was
`dispatch_loop'2` on a tree three changes older, so the flattening belongs to
the dispatcher change and the environment frame. Attributing it here would
have been the rewrite-family error again, a fortnight after that one was
withdrawn.

A cross-check worth keeping: the two debug-info profiles differ by
15,856,100 and the release A/B read the split at 15,853,848 — two toolchain
configurations, 2,252 apart.

**WHERE THIS LEAVES THE NEXT ATTEMPT.** `__memcmp_avx2_movbe` read
47,672,229, then 47,631,349, then 47,602,341 across the dispatcher change,
the environment frame and the frame split — unmoved by three changes to how
scopes are built and walked, and already the target of four declined schemes
in kanso#1529. With the five here, the two largest named things in the
interpreted profile have nine declined builds between them. Picking the top
frame off a profile has a record of one in five. The next real gain is
structural, and the way to find it is not another reading of the same list.

---

## 2026-09-18 — the counted row takes two values, and which one depends on where you stand

**MEASURED.** The A/B discipline this tree uses says the two arms must sit in
directories of the same path length, and the reason written down for it was
that the row "moves with path length". That is not the shape. One binary
(`/tmp/kanso-resbase`), one corpus, `kanso run interp_corpus --interp` counted
from ten directories differing only in name and length:

    /tmp/p                             len  6   1,007,027,010
    /tmp/pathaaaa                      len 13   1,007,027,010
    /tmp/pathbbbb                      len 13   1,007,027,010
    /tmp/pathaaaaaaaa                  len 17   1,007,027,010
    /tmp/pathaaaaaaaaa                 len 18   1,007,027,010
    /tmp/pathaaaaaaaaaa                len 19   1,007,027,010
    /tmp/pathaaaaaaaaaaa               len 20   1,007,027,010
    /tmp/pathaaaaaaaaaaaa              len 21   1,007,004,925
    /tmp/pathaaaaaaaaaaaaaaaa          len 25   1,007,004,925
    /tmp/pathaaaaaaaaaaaaaaaaaaaaaaaa  len 33   1,007,004,925

**TWO VALUES, 22,085 APART, ONE STEP BETWEEN 20 AND 21.** Flat across six
lengths below it and three above. The two equal-length directories with
different names agree, which rules the name out. And the LONGER path reads
FEWER instructions, so whatever this is, it is not a cost that grows with the
string being carried.

`interp_instructions.sh` documents the variable it knows about — "the count
tracks the length of the path the compiler is handed — about 160 instructions
a character" — and that is a different variable, the ARGUMENT handed to
`kanso`, measured in `library_box.sh`. This one is the working directory, with
the argument held at the relative `interp_corpus` throughout. A per-character
model does not describe it and neither does the sign.

**THE ENVIRONMENT IS A SECOND TERM AND A SMALLER ONE.** Same binary, same
directory, one variable added:

    plain                          1,007,027,010
    KANSOPAD=<12 characters>       1,007,027,097   +87
    KANSOPAD=<120 characters>      1,007,027,097   +87

+87 for one more entry, and the SIZE of the entry does not matter. So the term
is the count of environment entries rather than the bytes in them.

**NEITHER OF THESE IS LOOSE IN CI, and that is the point of writing them
down.** The gates already run under `env -i` with a fixed `GLIBC_TUNABLES` and
already `cd` into a fixed box, so both terms are pinned. What the measurement
adds is how narrow the margin is. There are two boxes:

    /tmp/kanso-compile-ir   21   compile, entry, library, interp, start-up
    /tmp/kanso-codegen      18   codegen, emit

21 is ONE CHARACTER past the step and 18 is three short of it, so the two sit
on opposite sides. Renaming the first box one character shorter would move
five rows by 22,085 with no compiler change behind it, and the only thing
standing between that and a re-based golden was that nobody had reason to
rename it.

`tests/every_counted_run_sits_at_one_fixed_path.rs` pins it: each gate uses
its own box, no gate declares a third, and each box is the length its goldens
were measured at. The first draft asserted there was ONE box and went red
naming the second, which is how the pair above came to be measured rather than
assumed.

**WHAT THIS DOES NOT EXPLAIN.** CI holds the path fixed, so this is not the
account of the ±7 and ±13 the interpreted row has drawn between jobs. The
entry that calls those the relink stands; nothing here touches it. Two
container readings support that separation from the other side: the same
source built twice into binaries with different content hashes — once by a
comment length change, once by a different worktree path — read 985,444,659
twice and 1,007,027,010 five times, at equal path lengths. A binary whose
bytes move while its path does not leaves this row alone.

**OPEN.** Why the step sits between 20 and 21 is not established, and the two
obvious guesses — an allocation size class the absolute path crosses, and a
small-string threshold — are guesses. The direction is the awkward part for
both: the longer path costs less.

## 2026-09-18 — the same two lines cost 12 million in one function and save 21 million in the one above it

**BUILT AND SHIPPING.** The previous entry closed by saying the next attempt
should not be another reading of the profile's self-cost list, which had gone
one for five. So this one came off a different axis: tally which callers
reach the allocator, rather than which functions carry instructions.

Out of the post-split profile, by caller:

    456,133  __rust_alloc  <- RawVecInner::finish_grow
    471,517  finish_grow   <- RawVec<T,A>::grow_one
    237,980  grow_one      <- kanso::eval::match_one
    225,431  grow_one      <- kanso::eval::Interp::dispatch_loop'2

463,411 of the run's 471,517 reallocations come from those two call sites,
and both are the same pair of buffers. `dispatch_loop_inner` declares `score`
and `binds` at the top of each dispatch and the candidate loop recycles them
— a winner hands its vectors back as the working pair rather than leaving the
next candidate to allocate from nothing, which kanso#1497 already built. What
they do not survive is the dispatch: the winner's bindings become the
environment frame and its score is kept beside `best`, so the next dispatch
starts at capacity zero and climbs 1, 2, 4 from nothing. 225,431 growths
against 55,711 dispatches is four reallocations a call.

**THE FIRST PLACEMENT LOST BY TWELVE MILLION.** `score.reserve(params.len())`
and `binds.reserve(params.len())` went after the two `clear()` calls at the
top of `match_params_into`:

    base      1,007,027,010
    reserve   1,019,044,912   +12,017,902   +1.19%

Which is where the day's fifth decline would have been written down, with the
mechanism left open. The draft entry saying so was written and pushed as
kanso#1538 before the code was read carefully enough — the honest reason it
is not in this file is that the reading came next and changed the answer.

**`match_params_into` RUNS ONCE PER CANDIDATE, AND THE ALLOCATION IS ONCE PER
DISPATCH.** Arm selection tries every arity match, so the reserve was paid
several times per dispatch to fix an allocation that happens once.

*(Corrected 2026-09-19: this said "roughly twenty times per dispatch —
1,100,726 calls to `match_one` against 55,711 dispatches". That ratio is
`match_one` CALLS a dispatch. `match_one` runs once per PARAMETER of each
candidate tried, so it bounds the candidates from above and equals them only
if every arm took one argument. See the entry of that date.)* The two
`clear()` calls it sat behind are per-candidate housekeeping on buffers the
loop already owns; they are not where the buffers come from.

**MOVED ONE FUNCTION UP, THE SAME REQUEST WINS.** `Vec::new()` becomes
`Vec::with_capacity(args_len)` at the two declarations in
`dispatch_loop_inner`, which is where the pair is actually created:

    base      1,007,027,010
    hoisted     985,444,659   -21,582,351   -2.14%

A swing of 33,600,253 instructions between two placements of the same
request, and the winning one is the larger gain of the day — bigger than the
frame split that shipped this morning as kanso#1535, which took 15,853,848.

`args_len` is exact for `score`, which takes one entry per parameter, and a
floor for `binds`, since a `Ctor` pattern can bind its fields and a whole.

**WHAT THIS SAYS ABOUT THE FIVE DECLINES.** It does not retract any of them;
each was a different change and each was measured. What it does retract is
the inference that was forming around them — that the dispatch path had been
read out, and that a correctly-measured quantity not turning into a saving
was the shape of this code rather than the shape of five particular attempts.
One of those five was placed a function away from the one that works.

**The base row is now read four times at 1,007,027,010**, from two binaries
with different content hashes (`555c7ab8` in the argmove pair, `e8cac1ad`
here) built from trees carrying the same interpreter at different paths. On a
row known to move with path length, that is the clearest statement so far
that this differential reads the code.

Seven builds on the dispatch path today, two wins.

**OPEN.** The tally that found this has three more entries nobody has been
after: `drop_slow` from `dispatch_loop'2` 229,625 times, `__rust_alloc` from
`String::clone` 149,093, and the 8,106 reallocations that are neither of the
two call sites above. The allocator-caller axis is not spent.

**AND THE SAVING IS NOT FEWER ALLOCATIONS, which the first version of this
entry implied and the commit message said outright.** Tallying the allocator
edges on both arms of the same A/B:

                      base        hoisted      delta
    __rust_alloc    1,361,559    1,362,891     +1,332
    __rust_dealloc  1,349,431    1,350,763     +1,332
    __rust_realloc     35,956       32,638     -3,318
    grow_one          464,507      112,013   -352,494
    finish_grow       489,542      137,048   -352,494

The allocation count goes UP. What the reserve removes is the GROWTH path:
352,494 fewer `grow_one`/`finish_grow` pairs, and with them the capacity
arithmetic, the amortised-doubling branch and the element copy each regrow
makes. 21,582,351 over 352,494 is 61.2 instructions a growth, which is a
plausible price for that work and is not a plausible price for an allocation.

Written down because the difference decides what to look for next. "Reserving
saves allocations" would send the next reader after allocation counts, and the
counts here are flat to a tenth of a per cent. The commit message on the
source change carries the looser phrasing; this is the correction.

## 2026-09-18 — kanso#1538, CI's rows for the per-dispatch reserve

    interp_instructions   957,583,234 -> 939,042,794   -18,540,440  -1.9362%
    interp_allocs           1,412,516 ->   1,410,530        -1,986  -0.1406%
    interp_peak_bytes         834,117 ->     833,466          -651  -0.0781%

Everything else in the job is byte-identical: compile 35,550,010, entry
126,729,588, library 127,186,008, emit 51,617,476, start-up 3,363,916,
compile_allocs 27,313, compile_peak_bytes 787,956. **The third runtime-only
change today that moved no layout.** The prior that editing the compiler's own
Rust moves `compile_instructions` has now missed three times running on
changes confined to the interpreter's hot path, which is the shape the
2026-09-06 correction described: a change small enough to leave the layout
alone leaves that row alone with it.

This container projected 21,582,351 and the runner reads 18,540,440 — same
direction, smaller, on different silicon (family 0x6 model 0xcf against 0x19).
Neither number is the other's check; the golden's header says the two hosts
are not comparable, and what is comparable is the sign.

**TWO ALLOCATION NUMBERS POINT OPPOSITE WAYS AND DO NOT DISAGREE.** CI's
`interp_allocs` falls 1,986 while the callgrind tally of `__rust_alloc` CALLS
rises 1,332. They are different instruments over different scopes — kanso's own
traffic counter over the whole run, against callgrind's call count over the
toggled interpreted thread — and both are written into the memory golden's
header so that a later reader finds the explanation next to the numbers rather
than a contradiction.

`interp_peak_bytes` gives back 651 of the 784 kanso#1534 recorded as open and
unexplained. A buffer asked for at its final size is never live beside the
smaller one it replaces, and a peak is where that overlap would show — a
candidate, not a mechanism, and no build isolates it.

Welfare 77.25 -> 77.26, banked in the same pull request.

---

## 2026-09-18 — two green sweeps did not cover four gates, and the spec that says so passed vacuously first

**BUILT.** On 2026-09-18 `all_counters.sh` printed "the twelve cost veins and
the lazy tier agree with their goldens" on a tree whose interpreted row had
moved 18,540,440 instructions. Nothing was lost — the move was deliberate and
CI measured it — but two sweeps reading green is what a session takes for
coverage, and four gates were in neither list:

    instructions           bench/instructions_golden.txt
    interp_instructions    bench/interp_instructions_golden.txt
    interp_memory          bench/interp_memory_golden.txt
    startup_instructions   bench/startup_instructions_golden.txt

The first is retired instructions per benchmark, which is the dimension every
allocation counter is blind to. CI runs all four, each as its own step with its
own row in the cost-goldens summary, so a pull request can go red on a gate no
container sweep names.

`scripts/gates/all_interp.sh` is the third sweep, in `all_compile.sh`'s shape
and for its stated reasons. **All four REFUSE on this container**, and that is
an argument for including them rather than against: a gate printed as REFUSED
says the row is unchecked and CI will measure it, where saying nothing says it
agreed. That distinction is the whole reason `all_compile.sh` separates the two
verdicts.

**THE SPEC PASSED WITHOUT CHECKING ANYTHING, AND ONLY BREAKING IT SHOWED THAT.**
`tests/every_golden_is_swept_by_something.rs` pins that every gate CI runs
against a golden is named by one of the three sweeps. Written, it went green.
Removing `interp_instructions` from the new sweep to watch it fail, it stayed
green — its `reads_a_golden` scan began AT `bench/` rather than after it, and
`/` is not in the character set it walks, so every gate read as naming no
golden at all and the assertion ranged over an empty set.

One character of offset, and a spec that would have shipped proving nothing
about the hole it was written for. CLAUDE.md's rule is exact about this — watch
it fail, for the right reason, before it passes — and the rule earned its place
again here. Fixed, it names `interp_instructions` and nothing else.

The property is anchored to the workflow rather than to a list in the spec,
because a list in the spec is the thing that goes stale: CI running a gate is
the obligation, so a gate added to CI and to no sweep is a red spec. Two
siblings already cover one sweep each — `every_counter_gate_is_in_the_sweep.rs`
and `the_compile_sweep_names_every_compile_gate.rs` — and by construction
neither could see a gate belonging to neither of them.

## 2026-09-18 — the release golden's header said the children reproduce, and one of them does not

**CORRECTION.** `bench/codegen_instructions_release_golden.txt`'s 2026-09-17
entry describes the row as "three `clang` processes and `ld`, every one of
which came back byte for byte across two readings". That is what its two
readings showed and it is not true in general. Three cost-goldens jobs on
2026-09-18 halted the vein for a reproduction failure, every clang process
byte-identical and the whole difference inside `ld`:

    kanso#1504   5,160,407,609 then 5,160,407,598   -11
                 probes 1,816,463 -> 1,816,452
    kanso#1502   5,139,582,528 then 5,139,582,517   -11
                 probes 1,822,415 -> 1,822,404
    kanso#1537   6,824,133,291 then 6,824,133,280   -11
                 probes 1,819,373 -> 1,819,362

All of it in `llvm::StringMapImpl::LookupBucketFor`, which is the frame the
header's own analysis instrumented the gate to name. Three different probe
counts and one residue.

The correction is appended to the header as a dated entry rather than written
over the 2026-09-17 one, for the reason the log works that way: what that
sentence recorded was true of its two readings, and what is wrong is the
general claim a later reader takes from it.

**kanso#1538 and kanso#1504's SECOND job both reproduced.** One branch has now
been on both sides, which settles the draw as a property of the job rather
than of any diff. The rate is not written down here; it changes with every job
run today, and the list lives in the ledger entry.

This corrects the record and settles nothing about the pin, which is
design/pending-gavels.md's to rule on.

## 2026-09-18 — the score buffer had no reason to be freed, and freeing it cost five million

**BUILT AND SHIPPING**, following the previous change rather than a fresh
reading of the profile. kanso#1538 gave the two dispatch buffers their arity up
front. The question this one asks is why either is allocated per dispatch at
all.

`binds` has an answer: it is moved into `bind_all` and becomes the environment
frame, so its allocation is still doing work after the dispatch ends. `score`
has none. It exists to compare candidates, it is kept beside `best` while one
candidate is winning, and then it is dropped. So it was declared above the
tail-call loop instead, and the winner's buffer is handed back to the working
variable rather than falling out of scope.

    base    985,444,659
    kept    980,371,488   -5,073,171   -0.51%

The allocator edges say it is the change and nothing else:

                     base        kept        delta
    __rust_alloc    1,362,891   1,243,349   -119,542
    __rust_dealloc  1,350,763   1,231,221   -119,542
    grow_one          112,013     112,013          0
    finish_grow       137,048     155,543    +18,495

**119,542 allocate-and-free pairs, at 42.4 instructions each.** The growth path
is untouched, which is the point: kanso#1538 took the growths and this takes
the allocations, and the two costs are separable and were separated. The
+18,495 in `finish_grow` is the retained buffer growing when a later dispatch
arrives with more parameters than the one that sized it — the price of keeping
it, and it is in the measured total.

**119,542 IS NOT THE DISPATCH COUNT, AND IT IS NOT THE ITERATION COUNT
EITHER.** There are 55,711 dispatches, so this is 2.15 pairs each, and the
reason it exceeds one is the loop the buffer now lives above: a tail call goes
round again without leaving `dispatch_loop_inner`, and every one of those was
allocating and freeing a score buffer too. `frame_for` is called once an
iteration and reads 175,246, so the iterations are about three per dispatch.

That leaves 119,542 removed against roughly 175,246 that could have been, and
the first draft of this entry said "one per tail hop" without checking the
second number. About two-thirds of iterations were allocating. What accounts
for the other third is NOT established here. The obvious candidate is arity
zero — `Vec::with_capacity(0)` allocates nothing, so a nullary dispatch never
had a buffer to free — and that is a candidate, not a measurement: nothing in
this profile counts dispatches by arity. The saving is 119,542 pairs whatever
explains the gap.

**Three wins in eight builds today**, and the two since the allocator-caller
tally are both wins, against one in five before it.

**A FOURTH CHANGE LEAVES THE COMPARING ALONE.** `__memcmp_avx2_movbe` reads
46,087,562 then 46,118,166 across the reserve pair, +30,604, +0.066%. A draft
of this entry had it FALLING 1,484,175, off 47,602,341 — which is a debug-info
profile read against a release one, two build configurations rather than two
trees. Compared inside its own sitting it has not moved, and §96's count of
three changes becomes four.

**THE ITERATION COUNT IS CONFIRMED FROM A SECOND FRAME, and the remaining
allocations are named.** `drop_in_place<Option<(Vec<u8>, &ka...)>>` is called
175,246 times, and that type is `best` — `Score` is `Vec<u8>` and `Bindings` is
`Vec<(Name, Value)>`, so `Option<(Score, &FnDecl, Bindings)>` is exactly what
the annotator truncated. Two unrelated frames agreeing at 175,246 makes the
iteration count a measurement rather than an inference, and leaves the gap to
119,542 standing as the open part.

**A SCORE IS ONE BYTE PER PARAMETER.** `Score = Vec<u8>`, so what this change
stopped allocating was a handful of bytes, and 42.4 instructions is the
measured price of an allocate-and-free pair that small. A guess before the
build put it near 120, which is the price of a larger one.

**OPEN, and this is where the dispatcher's allocations now are.**
`__rust_alloc` is still reached 404,871 times from `dispatch_loop`, against
175,246 iterations. Two per iteration are accounted for by construction: the
bindings vector, which `bind_all` turns into the environment frame, and the
`Rc<Env>` node that holds it. That is 350,492, and the remaining 54,379 are
not attributed. Neither of the two is removable the way the score was — both
outlive the dispatch — so the next thing to ask about them is whether a frame
whose refcount reaches one at the end of a call can be handed back rather than
freed. That is a larger change than anything built today and nothing here
measures it.

**A SPEC CAUGHT IT, WHICH IS THE SPEC WORKING.**
`tests/a_unique_container_is_extended_in_place.rs` pins the allocation
DIFFERENCE between a 300-round run and a 600-round one, exactly rather than as
a band, and it went red: 5,403 expected, 4,803 read. Six hundred fewer over
three hundred extra rounds is two a round. Its own doc names the protocol —
"the number was re-read rather than the assertion widened. A change in what
the ROUNDS cost is exactly what the subtraction exists to see" — so the number
is re-read to 4,803 with the reason beside it.

Two things make that safe rather than convenient. The sibling test still reads
`1200 600` and `2400 1200`, so the in-place path is doing what it did; and the
number moved DOWN, where a container that stopped being extended in place
would move it sharply up.

**WHICH two of a round's dispatches stopped allocating is left open in that
file on purpose.** The paragraphs above it decompose their own deltas by
counting calls, and the same arithmetic does not obviously give two here. Two
a round is the measurement. A decomposition guessed into a spec's doc is what
the next reader would check their own change against.

## 2026-09-18 — kanso#1540, CI's rows for the kept score buffer

    interp_instructions   939,042,794 -> 932,183,914   -6,858,880  -0.7304%
    interp_allocs           1,410,530 ->   1,309,483    -101,047   -7.1638%
    interp_peak_bytes         833,466 ->     833,463          -3  -0.0004%

Every compile vein byte-identical: compile 35,550,010, entry 126,729,588,
library 127,186,008, emit 51,617,476, start-up 3,363,916, compile_allocs
27,313, compile_peak_bytes 787,956. **The fourth runtime-only change in a row
that moved no layout.**

This container projected 5,073,171 and the runner reads 6,858,880 — the same
direction and larger, which is the shape every host split has taken today.

**`interp_allocs` REPRODUCES ACROSS HOSTS AND `interp_instructions` DOES NOT.**
The instrumented container run printed `interp_allocs=1309483` for this tree
and the runner reads 1,309,483 — the same figure to the unit, on machines whose
instruction counts differ by millions. It counts what the program asked the
allocator for rather than what the machine did, which is why its gate carries
no host-divergence allowance and why the instruction gates refuse on this box
while this one would not have.

The two allocation instruments agree in sign here — CI's counter falls 101,047
and the callgrind tally of `__rust_alloc` CALLS over the toggled thread falls
119,542 — where on kanso#1538 they pointed opposite ways. Both entries say the
same thing about why: different scopes, different definitions, and neither is
the other's check.

Welfare 77.26 -> 77.27, banked in the same pull request.

**THE HOST-INDEPENDENCE CLAIM, CHECKED AND HALF REFUTED.** The entry above says
`interp_allocs` reproduces across hosts, on the strength of one agreement, which
is a coincidence until it is two. Run against a second tree: on kanso#1538's the
container prints `interp_allocs=1410530` and the runner reads 1,410,530. Two
trees, two hosts, exact both times.

The same run refutes the wider reading. `interp_peak_bytes` on that tree is
833,458 on the container against 833,466 on the runner — **eight bytes apart**.
So it is the TRAFFIC COUNT that is host-independent and not the memory rows as
a family. A peak is a high-water mark of what the allocator held at one instant
and what it held depends on the machine, where a call count does not; that is a
candidate for the mechanism, and the eight bytes are the measurement.

Worth the two minutes it took. The claim had already been written into a golden
header, where the next reader would have taken it for both rows.

---

## 2026-09-18 — a fourth round asserting the name was not the cause, and it was

**A CORRECTION, and the thing corrected is this session's own work from this
evening.** kanso#1513's entry established the mechanism: twenty-two names, one
binary, one corpus, every other input held and the output path absent each
time, twenty reading 5,163,341,031 and two reading 5,163,341,042. Eleven apart,
deterministic per name, about one name in eleven. The eleven is the temporary
object's name.

That entry also wrote down the hazard, in these words: "Three rounds were spent
on it, and one was spent asserting the name was not the cause. That assertion
rested on three samples, then five, of a one-in-eleven effect."

**This evening spent a fourth.** Working from the golden's header rather than
from that entry, the ledger gained a section saying the cause was open, a
section offering a "limit on the evidence that rules names out", and a section
saying kanso#1512's premise "may no longer hold". All of it landed on main in
kanso#1537.

**THE SPECIFIC ERROR.** The header re-explains 5,163,341,031 as its own reading
with the output path ABSENT, and that was read as the figure belonging to the
path instead of to names. But the twenty-two-name experiment was run with the
output path absent too, and 5,163,341,031 is what twenty of its twenty-two
names read. Same configuration, same number. The header's reading is the low
bucket. Two measurements agreeing was mistaken for two explanations competing.

**AND THE THREE FINDINGS THAT FELT LIKE PROGRESS ARE PREDICTIONS OF THE
MECHANISM.** `LookupBucketFor` is where a `StringMap` probe walks, and the name
is the string it walks for. A two-bucket per-name effect gives a residue that
is always eleven, which is what "a candidate has to explain a constant" was
groping toward. And a job draws when its two readings straddle the buckets:
2 × (1/11) × (10/11), about 17%, the same order as the day's handful.

The per-frame diff and the replication table are worth keeping. They are
evidence FOR the name and were read as evidence that the cause was unknown.

**WHY IT HAPPENED, since the rule it breaks is already in CLAUDE.md twice.**
The header of a golden and the log entry of the branch that fixes the thing are
two sources, and only one of them was read. The entry was one `awk` away in a
worktree already checked out. "Read the thing the number describes before
running anything against it" is the rule, and a branch's own log entry is part
of the thing.

---

## 2026-09-18 — a golden carried one row twice, and four sweeps read it as a sum

**BUILT AND SHIPPING**, a spec and two one-row deletions.

`bench/codegen_instructions_release_golden.txt` held
`codegen_instructions_release=` twice on kanso#1504 — 6,824,133,280 and
6,841,691,425 — and twice on kanso#1502, with 6,820,866,344 as the second.
Both came from the same move: a merge brought main's row forward, wrote a
header block explaining which reading it was, and appended it UNDER the row
already there rather than replacing it.

The reader adds:

    *out.entry(format!("{prefix}{}", name.trim())).or_default() += n;

That `+=` is right and has to stay. A cost golden holds one row per sample and
the gate's `totals` sums them, and `bench/objective_sources.txt` maps one
objective counter onto several gate keys. What the `+=` cannot tell apart is a
second sample from a second copy. The gate read 13,665,824,705 — exactly the
two added.

**WHAT DID NOT SPEAK.** The counter sweep, the compile sweep and all three page
gates ran green on both branches. Not one of them reads a golden for a repeated
key: the sweeps compare a measured row against the file, and if the file
answers with a sum they compare against the sum.

**WHAT DID, AND WHAT IT SAID.**
`tests/the_objective_reads_what_the_gate_watches.rs` went red with:

    `codegen_instructions_release` reads 6841691425 from welfare and
    13665824705 from codegen_instructions_release. The link is wrong or a
    pool joined the sum.

The link was fine. No pool had joined anything. That spec compares TOTALS, so a
doubled row reaches it as a wrong total and it names the last thing that could
produce one. Two branches were then red on a message pointing at the wrong
file, and the defect is a duplicated line three directories away.

This is the standing note about a verification that names one file while the
defect sits next door, in its cleanest form yet: the spec was correct, its
assertion was correct, and its DIAGNOSIS sent a reader to
`bench/objective_sources.txt`.

**THE SPEC.** `tests/a_golden_holds_one_row_per_counter.rs` reads the trend
gate's own list of goldens — the same `[["bench/…" "prefix_"]]` parse the
objective spec uses, so a golden added to the gate is covered without a second
list to maintain — and asserts no counter name appears twice within one file.
It reports the file, the name and the count.

Watched red by planting the exact fault, a second
`codegen_instructions_release=6841691425` appended to main's copy:

    bench/codegen_instructions_release_golden.txt:
      `codegen_instructions_release` appears 2 times

It is deliberately not a value check. Whether a row holds the right number is
what the gates are for and what CI measures; this asserts only that there is
one of it to read, which is the property a merge breaks and no measurement can
restore.

The invariant was checked against the tree before it was written: all 29
goldens the gate names hold at most one row per counter on main, so this pins
something already true rather than declaring a convention.

**THE TWO FIXES.** One row each, main's carried forward, with the duplicate's
history in the header. Neither branch's own reading survives, because both were
taken against bases that have since moved under kanso#1540, kanso#1541 and
kanso#1544 — CI measures the merged trees and the rows are copied out of the
job logs.

---

## 2026-09-18 — what the dispatcher still allocates, and why the profile cannot finish the sentence

**OPEN, measured as far as this instrument goes.** kanso#1540's entry left one
thread: after the score buffer stopped being allocated per dispatch, the
dispatcher's remaining allocations are the bindings vector, which `bind_all`
turns into the environment frame, and the `Rc<Env>` node holding it. Both
outlive the dispatch, so neither is removable the way the score was. The
question that follows is whether a frame whose strong count reaches one when
the body returns can be handed back rather than freed.

A debug-info profile of the interpreted corpus, `--auto=yes`, says this much:

    175,246   iterations (frame_for, and the drop of `best`, agreeing)
    229,625   Rc<T,A>::drop_slow            <- dispatch_loop'2
    457,929   drop_in_place<Value>          <- Rc<T,A>::drop_slow
    796,091   drop_in_place<Value>          <- dispatch_loop'2
    521,766   instructions on the line `Some(Rc::new(Env::Many(binds, env)))`

`drop_slow` runs only when a strong count reaches ZERO, so the dispatcher is
taking an `Rc` to its last handle 229,625 times against 175,246 iterations —
1.31 a time.

**AND THAT IS NOT THE FRAME'S SHARE.** The dispatcher holds three kinds of
`Rc`: the frame, the name it dispatches on, and the overload list. Every one of
them is `Rc<T,A>::drop_slow` in the profile, because the type parameter is gone
by then and `drop_in_place<Env>` is inlined into it — there is no edge in the
whole file naming `Env`, on a debug-info build, with auto-annotation on. So
1.31 per iteration bounds the frame's share from ABOVE and says nothing about
where inside that bound it sits.

Written down because the bound is the useful part and the temptation is to read
it as the answer. If every one of those were the frame, reclaiming it would be
worth 175,246 allocate-and-free pairs, which today's two measurements price
between 42.4 and 61.2 instructions each: seven to ten million, about one per
cent. That is the ceiling, and the floor is zero.

**WHAT WOULD SETTLE IT IS A BUILD, not another profile.** The dispatcher moves
the environment into `eval_body_flow`, so measuring how often it comes back
unshared means keeping a handle and counting — which is most of the change
itself. The profile has been run; the next step is the build.

**AND THE BUILD SETTLED IT: 173,921 OF 173,922.** The bound above is the
answer, at its ceiling. An instrumented binary took a `Weak` to the frame
before the body ran and tried to upgrade it after:

    interp_frames_made=173922
    interp_frames_dead=173921

One frame in the whole run outlives the body that was given it. The `Weak` was
deliberate rather than a second strong handle, because a strong clone would
make `Rc::try_unwrap` fail everywhere inside the body and change the behaviour
being measured.

So the frame is reclaimable 99.9994% of the time, and the prize is the full
173,922 allocate-and-free pairs rather than some fraction of them: seven to ten
and a half million instructions, 0.75% to 1.1% of the interpreted row, at the
42.4 and 61.2 per pair that kanso#1540 and kanso#1538 measured.

173,922 against the 175,246 iterations the profile counts is the dispatches
whose parameter list binds nothing, where `bind_all` hands the parent
environment back and builds no frame at all.

**WHAT THE BUILD WOULD BE.** The dispatcher moves the environment into
`eval_body_flow`, so the frame dies in there. Keeping the owner in the
dispatcher and passing a reference would let `Rc::try_unwrap` take the
`Env::Many(binds, parent)` back afterwards, and with it the bindings vector's
allocation for the next iteration. `eval` already takes its environment by
reference, so the shape exists. The risk to check is that holding the frame one
frame longer does not change what the body's own uniqueness checks see, and
`a_unique_container_is_extended_in_place` is the spec that would say so: it
pins an exact per-round allocation count and caught the last change to this
function.

The instrumentation is reverted. It is described here rather than kept, since a
counter that exists to answer one question is a vein to regenerate forever
after.

---

## 2026-09-18 — the dispatcher takes its frame back, and gets two thirds of what the count promised

**BUILT AND SHIPPING.** The entries above measured the opportunity and then
settled it: the environment frame is dead 173,921 times out of 173,922 by the
time the body has finished with it. So the dispatcher keeps a handle beside the
one the body gets, and afterwards asks for it back.

    let held = env.clone();
    let flowed = self.eval_body_flow(decl, env);
    if let Some(rc) = held {
        if let Ok(Env::Many(slots, _)) = Rc::try_unwrap(rc) {
            pool = slots;
        }
    }

`Rc::try_unwrap` declines in exactly the case the instrumented build found: a
lazy thunk that captured its defining environment. The bindings vector the
frame was built around goes into a pool above the tail-call loop and the next
dispatch takes it instead of allocating.

    base    980,371,488
    frame   977,583,095   -2,788,393   -0.28%

                     base        frame       delta
    __rust_alloc    1,243,349   1,123,808   -119,541
    __rust_dealloc  1,231,221   1,111,680   -119,541
    grow_one          112,013      94,847    -17,166
    drop_slow         262,323     160,154   -102,169

**THE PROJECTION WAS SEVEN TO TEN AND A HALF MILLION AND THE BUILD PAID 2.79.**
That is the useful part of this entry. The projection priced 173,922 pairs at
the 42.4 and 61.2 instructions the two changes before it measured. What arrived
is 119,541 pairs at 23.3 each, and both halves of the gap are real: fewer pairs
than frames, because a pooled vector that already has capacity does not
allocate when it is reserved again, and a cheaper pair than either earlier
measurement, because this one BUYS the saving — a reference count up, a
reference count down and a `try_unwrap` check on every dispatch, against an
allocate-and-free it does not make.

`drop_slow` falling 102,169 is the frame being unwrapped instead of dropped
through it, and is the clearest single sign the change does what it says.

**WHAT IS STILL ALLOCATED.** The `Rc<Env>` node itself. `bind_all` calls
`Rc::new` on every dispatch that binds anything, and reclaiming the vector
inside does nothing about the node around it. That is the other half of the
projection and it is untouched.

**A SPEC CAUGHT THIS ONE TOO**, the same one, the same way:
`a_unique_container_is_extended_in_place` read 4,203 against a pin of 4,803 —
two fewer allocations a round for the third change running. Re-read per that
file's protocol. This change is the one most likely to disturb what that spec
measures, because it holds a second handle to a frame whose values are the
containers `push`, `put` and `append` call `Rc::try_unwrap` on. It does not,
and the reason is that the frame was already alive for the whole body: the
extra handle moves when the frame dies, from inside the body to just after it,
and a container's uniqueness is decided while the body runs. The sibling test
still answers `1200 600` and `2400 1200`, and the number moved down.

Nine builds on the dispatch path today, four wins.

## 2026-09-18 — kanso#1543, CI's rows, and an objective that barely moves

    interp_instructions   932,183,914 -> 929,300,332   -2,883,582  -0.3094%
    interp_allocs           1,309,483 ->   1,183,336    -126,147   -9.6333%
    interp_peak_bytes         833,463 ->     834,079        +616  +0.0739%
    compile_instructions   35,550,010 ->  35,551,167      +1,157
    entry_instructions    126,729,588 -> 126,732,646      +3,058
    library_instructions  127,186,008 -> 127,188,882      +2,874
    emit_instructions      51,617,476 ->  51,619,793      +2,317
    startup_instructions    3,363,916 ->   3,363,672        -244

The second count in the same job read 929,300,332 as well, so the binary is
stable and the disagreement is with the golden rather than within the run.

**FIVE COMPILE-SIDE ROWS MOVED, AND THIS IS THE FIRST CHANGE IN THE FAMILY BIG
ENOUGH TO DO IT.** kanso#1538 and kanso#1540 left every one byte-identical, and
their entries said so. This one adds a pool, a clone and a `try_unwrap` to
`dispatch_loop_inner`, and the layout moved: four up, one down, each reproducing
twice inside the job. `kanso check` stops before the interpreter runs, so none
of it is the change's subject. The 2026-09-06 correction stands with a third
data point — the prior that editing the compiler's own Rust moves these rows is
a good one, and the exception is a change small enough to leave the layout
alone.

**THE PEAK ROSE, AND IT IS THE PRICE RATHER THAN A SURPRISE.** Pooling a buffer
means the buffer is resident when the run is at its widest. kanso#1540's entry
predicted this direction in those words and then happened to fall 3; this one
pays 616. Traffic falls 126,147 against it, which is the two rows doing what
the gate says they do: a total and a high-water mark, free to move apart.

**AND THE OBJECTIVE BARELY MOVES.** Welfare reads 0.00 above the floor — a rise
the sentinel still wants banked, and banked it is, but the honest summary is
that a 0.31% instruction fall is very nearly cancelled by what the pooling
costs in residency and in layout. The instruction row is not the objective and
this is the clearest case today of the difference: three changes that each took
millions off the interpreted row moved welfare 77.25 to 77.26 to 77.27 to 77.27.

That is the model working rather than failing. `interp_instructions` sits on
the development side under a satiating curve, and a row already improved 132%
against its baseline pays very little for the next percent. A change wanting to
move the number has to find production work or an unsatiated term.

---

## 2026-09-18 — the frame's node comes back too, and this one buys its saving with nothing

**BUILT AND SHIPPING.** kanso#1543 took the vector out of a dead frame and let
the `Rc` go. Its own entry named what it left: "the node around the vector is
still allocated. every dispatch that binds anything still calls for one, and
reclaiming what is inside does nothing about it." This is that node.

The reason the node had to be destroyed was the tool. `Rc::try_unwrap` reaches
the value by consuming the handle, so the only way to get the vector out was to
free the `RcBox` around it, and the next dispatch called `Rc::new` for a fresh
one. `Rc::get_mut` reaches the same vector through a handle that stays alive.
So the pool stops being a `Bindings` and becomes an `Option<Rc<Env>>`: the node
lends its vector to the candidate list at the top of the loop and takes it back
when the body is done.

    base    977,646,574
    node    972,776,892   -4,869,682   -0.50%

Two readings of each arm, in one box, interleaved; both arms repeated their own
figure exactly. The two arms sit at `/tmp/wt-rcbase` and `/tmp/wt-rcnode`,
named to the same length on purpose.

**THE TRAFFIC IS THE SAME 119,541 kanso#1543 RECLAIMED**, which is the useful
part of the measurement:

                          base         node       delta
    interp_allocs       1,183,336    1,063,795   -119,541
    interp_alloc_bytes 84,810,613   75,247,333  -9,563,280
    interp_peak_bytes     834,079      834,079           0

Not a number near it — the same one. kanso#1543 stopped allocating the VECTOR
for 119,541 frames and this stops allocating the NODE for 119,541 frames, so
the two changes are reclaiming the same set of frames from two sides, and the
set is now fully accounted. The bytes divide exactly: 9,563,280 over 119,541 is
80.0, which is what an `RcBox<Env>` occupies.

**AND THE PRICE PER PAIR IS THE UNDISCOUNTED ONE.** 4,869,682 over 119,541 is
40.7 instructions, against the 42.4 kanso#1540 measured for a small allocate-
and-free pair. kanso#1543 got 23.3 for the same count because it bought its
saving: a reference count up, a reference count down and an unwrap check on
every dispatch. This one swaps `try_unwrap` for `get_mut` at the same point in
the same code and adds no traffic of its own, so what arrives is close to the
full price of the pair. The two changes together take 119,541 frames from two
allocations each to none, for 7,658,075 instructions.

**THE PEAK DOES NOT MOVE**, where kanso#1543's rose 616 for the pooled vector.
A retained 80-byte node is not resident at the high-water mark, because the
node it replaces was resident there in the base arm too.

**WHAT THE PER-SYMBOL TABLE SHOWS, AND WHAT IT DOES NOT.** Self costs, same
sitting:

                      base         node        delta
    mi_free        26,050,308   23,420,394   -2,629,914
    __rust_alloc   17,054,835   15,261,720   -1,793,115
    mi_malloc       8,269,835    7,433,048     -836,787
    __rust_dealloc  2,270,982    2,031,900     -239,082
    drop_slow       3,230,145    5,758,049   +2,527,904
    drop_slow'2       538,984    1,074,180     +535,196
    grow_one        2,496,923    2,496,923            0
    finish_grow     8,509,860    8,509,860            0

The allocator falls and the reference-count teardown rises. That is attribution
moving rather than a second effect: the bindings a frame holds used to be
dropped through `binds.clear()` on a bare vector, where the compiler inlined
that work into the dispatch loop, and they are now dropped out of a vector that
lives inside an `Rc<Env>`, where it lands in `drop_slow`'s symbol. The same
`Value` drops happen either way and the row fell by more than the two rises
together. Written down without a mechanism attached, because this is a
difference in where the profiler filed the work and nothing here isolates it.
The claims that rest on isolation are the three counters above, each read
twice.

**UNIQUENESS: `get_mut` IS THE STRICTER TEST AND THAT IS FINE HERE.**
`try_unwrap` reads the strong count alone; `get_mut` reads the weak count too,
so a frame with a live `Weak` would be reclaimed by the first and refused by
the second. Nothing in `src/` takes a `Weak<Env>` — the instrumented build that
did was reverted after kanso#1543 measured with it — so the two agree today.
The difference is written into the source rather than argued away, because the
day something takes a `Weak<Env>` this becomes a silent loss of reuse rather
than a bug.

There is no `unreachable!` on the reclaim path. A refused `get_mut` carries the
winner's bindings back out through a spare and falls through to `bind_all`, so
the worst case is an allocation rather than a panic on a live interpreter.

**THE SPEC IS THE SAME SPEC, THE FOURTH CHANGE RUNNING.**
`tests/a_unique_container_is_extended_in_place.rs` pins what 300 extra rounds
of the two builders cost in allocations, and it went red at 3,603 against a
pinned 4,203 — another 600 over 300 rounds, another two a round. Its sibling
still answers `1200 600` and `2400 1200`, so the in-place path is undisturbed,
and the number moved DOWN, which is the wrong direction for a container that
stopped being extended in place. Re-read rather than widened, as that file's
own protocol says.

The two a round has now held across four changes to this loop. The file still
declines to decompose it, and that is deliberate: a wrong decomposition written
there is what the next reader would check their change against.

**OPEN, and smaller than the last one.** The loser's bindings buffer is still
freed per dispatch. The candidate loop swaps `binds` with the outgoing best's
vector, so when the winner's goes into the frame, the other one falls out of
scope at the end of the iteration. Pooling it is the same trick a third time,
and the ceiling on it is one allocate-and-free pair per dispatch that had more
than one arity-matching candidate. Unmeasured; the count is not in hand.

---

## 2026-09-19 — kanso#1545, CI's rows for the pooled frame node

    interp_instructions   929,300,332 -> 923,151,727   -6,148,605  -0.6617%
    interp_allocs           1,183,336 ->   1,063,795     -119,541
    interp_peak_bytes         834,079 ->     834,079            0

Read twice in the same job, the same number both times, so the disagreement was
with the golden rather than within the run. Every other vein reported success:
the twelve cost veins, all eight compile-side rows, both codegen tiers, emitting
and start-up. A runtime-only change that moved no layout, the fifth running.

**THE CONTAINER PROJECTED 4,869,682 AND THE RUNNER READ 6,148,605.** Same
direction, larger, different silicon — the third time this family has landed
that way, after kanso#1540 (5,073,171 projected against 6,858,880 read) and
kanso#1543. Three is enough to say the container under-reads this row's
improvements rather than that any one reading was unlucky; it is not enough to
say by how much, and the ratio is 1.26, 1.35 and 1.35 on the three.

**THE ALLOCATION COUNT REPRODUCED ON BOTH ARMS.** The container read 1,183,336
for the base and 1,063,795 for this tree; the runner read the same two numbers.
That is the property this vein has and the instruction vein does not, and it
now has a two-arm confirmation rather than a one-sided one.

**119,541 IS THE SAME COUNT kanso#1543 RECLAIMED VECTORS FOR.** Not a number
near it. That change stopped allocating the vector for a set of frames and this
one stopped allocating the node around the same set, so the set is fully
accounted rather than merely reduced. What the two take together is 119,541
frames from two allocations each to none.

The peak did not move on either host, where kanso#1543's rose 616 for the
vector it kept. A retained 80-byte node is not resident at the high-water mark,
because the node it replaces was resident there before it.

**THE INTERPRETED ROW SINCE THIS FAMILY STARTED.** 1,075,174,600 to 923,151,727
on CI's readings, a fall of 14.1% over ten builds and five wins.

**WHAT THE OBJECTIVE DOES WITH IT: almost nothing, again.** `interp_instructions`
sits on the development side under a satiating curve and is now more than 136%
improved against its baseline, so six million more buys a hundredth of a point.
That is the model working as designed rather than failing, and it is the third
entry in a row to say so. A change wanting to move the number has to find
production work or an unsatiated term.

---

## 2026-09-19 — pooling the loser's buffer: built, measured, declined

**BUILT AND DECLINED.** kanso#1545's entry left the third of these: the
candidate loop swaps the working vector with the outgoing best's, so when the
winner's goes into the frame the other falls out of scope at the end of the
iteration. Pooling it is the same trick again. It costs.

    base     972,776,892
    spare    976,125,124   +3,348,232   +0.34%

Three readings of each arm, interleaved, each arm repeating its own figure
exactly. The arms sit at `/tmp/wt-basers` and `/tmp/wt-losers`, named to the
same length.

**AND IT DOES SAVE THE ALLOCATIONS IT SET OUT TO SAVE**, which is what makes
the result worth keeping:

                          base         spare       delta
    interp_allocs       1,063,795    1,041,355    -22,440
    interp_alloc_bytes 75,247,333   69,333,733  -5,913,600
    interp_peak_bytes     834,079      834,303       +224

22,440 allocate-and-free pairs gone, and the row still went UP by 3.35
million. At the 40.7 instructions a pair kanso#1545 measured, those pairs are
worth about 914,000, so the bookkeeping cost something over four million.

**THE ASYMMETRY IS THE WHOLE ANSWER.** The spare has to be taken out and put
back on EVERY dispatch — 175,254 of them — to serve a reuse that fires on
22,440. About 24 instructions a dispatch of moves and drops, against a saving
on one dispatch in eight. The two changes that worked did not have this shape:
kanso#1543 and kanso#1545 pay their bookkeeping on the same frames they save,
so the ratio is one to one.

**THE CEILING WAS MEASURED FIRST AND WAS STILL TOO KIND.** An instrumented
build counted 38,294 of 175,254 dispatches leaving the working buffer with
capacity, which projected about 1.56 million. The real saving was 22,440 pairs,
not 38,294: some of those buffers had capacity they had recycled within the
dispatch rather than allocated. So the projection was 70% too high on the
count before the overhead was counted at all.

A ceiling computed from a count is an upper bound on the SAVING and says
nothing about the COST of collecting it. Both of the previous two changes
happened to have negligible collection cost and that is not a property of the
technique.

**WHERE THE DISPATCH LOOP STANDS.** Three poolings attempted, two kept. The
score buffer (kanso#1540), the frame's vector (kanso#1543) and the frame's node
(kanso#1545) together took the interpreted row from 1,075,174,600 to
923,151,727 on CI's readings. The loser's buffer is the one that does not pay,
and this entry is here so it is not tried a fourth time.

---

## 2026-09-19 — what a name lookup walks, and the two fifths of it that find nothing

**OPEN, measured.** The dispatch-loop pooling family is finished — three
attempted, two kept — and the interpreted profile's next frames are
`match_one` at 68,147,655, `lookup` at 67,319,929 and `Value::clone` at
54,332,739. This measures the second of those before anything is built on it.

An instrumented build counted what `lookup` does on the interpreted corpus:

    look_calls        1,056,329
    look_probes       2,662,536     2.52 name comparisons a call
    look_depth        1,155,900     1.09 frames walked a call
    look_bytes        5,291,628     1.99 bytes a comparison
    look_miss           332,025     31.4% of calls answer nothing
    look_miss_probes  1,071,803     40.3% of ALL probes
    look_sameptr              0

Four things follow, and the third is the one to build on.

**THE CHAIN IS SHALLOW AND THE SCAN IS NOT.** 1.09 frames a call: a lookup
almost always answers in the frame it starts in, or fails in it. So the cost is
the linear scan of one frame's slots, not a walk up a long parent chain, and
anything aimed at chain depth is aimed at 9% of the calls.

**THE NAMES ARE TINY.** 1.99 bytes a comparison. Two-character names, compared
with a length check and a memcmp call whose overhead dwarfs the two bytes it
reads. `__memcmp_avx2_movbe` is 34,234,622 in the same profile, and
34,234,622 over 2,662,536 is 12.86 — which is the right SIZE for these
comparisons and is NOT evidence that they are these comparisons. Nothing here
isolates memcmp's callers, so that share stays open; the probe count is
measured and the attribution is not.

**TWO FIFTHS OF THE COMPARISONS ARE MADE BY LOOKUPS THAT FIND NOTHING.**
1,071,803 of 2,662,536. A miss averages 3.23 probes against a hit's 2.20,
because a hit can stop early and a miss cannot stop at all — it scans every
slot of every frame before falling through to the global table. A name the
compiler could tell was global would skip the walk entirely, and that is 40% of
this frame's scanning plus 332,025 calls' worth of entry and exit.

**AND `look_sameptr` MEASURES THE REPRESENTATION, NOT AN OPPORTUNITY.** The
zero is a tautology and it was nearly published as a finding. `Name` is not a
pointer: src/name.rs holds a name INLINE, one length byte and twenty-two of
payload, in the AST node itself. So each name is its own buffer and two names
reading the same text can never share an address — the counter could only ever
have been zero, whatever the program did.

**AND INTERNING IS ALREADY RULED ON.** That module's own header says so in its
third sentence: interning was measured and declined in kanso#1033 at 365
conversion sites for one field of twenty-nine, and the 2026-08-29 ruling took
the other road, which needs no table, no lifetime and no id. The inline
representation IS that road. A first draft of this entry proposed interning as
the larger of two leads, against a ruling recorded a fortnight ago, because the
module that says so was one file away and was not read.

The 1.99 bytes a comparison is the same header's measurement seen from the
other side: 89.8% of identifier occurrences across lib/ are seven bytes or
fewer. A comparison already that short is not where the instructions are.

Nothing built. The lead is the misses, and it is the only one these numbers
support: their saving is bounded below by work that is provably wasted.

---

## 2026-09-19 — a filter on the frame, and a premise the measurement had already refuted

**BUILT AND DECLINED.** kanso#1547 measured that 332,025 of 1,056,329 lookups
answer nothing and spend 1,071,803 of the run's 2,662,536 name comparisons
doing it, and named the misses as the lead. The cheapest way to skip a miss is
to let the frame say no: a one-word membership filter over the binding names,
tested before the scan.

    base      972,776,892
    filter    983,593,036   +10,816,144   +1.11%

Two readings of each arm, identical both times, allocations byte-identical.
Worse than the loser's-buffer pooling and for a related reason.

**THE PREMISE WAS REFUTED BY THE SAME ENTRY THAT SUGGESTED THE LEAD.** A
membership filter is an optimisation for a LONG scan: it earns its keep when
rejecting costs much less than walking. kanso#1547's own numbers say the scan
is 2.52 comparisons over 1.09 frames. That is not a long scan, and there was
never much for a filter to skip.

The 40.3% is a SHARE and the lead was read as though it were a magnitude.
1,071,803 wasted comparisons is two fifths of the run's name-comparison work
and still a small number of instructions in absolute terms, against a filter
that must be computed once per lookup — `name_bit` on all 1,056,329 calls —
and rebuilt on every frame that binds. Both of those are paid whether or not
anything is skipped.

**AND THAT IS THE SECOND TIME TONIGHT**, after the loser's buffer. Both come
from one habit: pricing a saving from a COUNT while leaving the unit cost
unmeasured and the overhead out of the model entirely.
kanso#1545's entry stated the rule and this change was built the next hour
without applying it.

So the rule earns a sharper form. **Before building on a count, divide it.**
1,071,803 probes over a run is a share; 2.52 probes over a call is a scan
length, and the second is what decides whether skipping the scan can pay. The
same two numbers were in hand both times.

**WHAT REMAINS TRUE.** The misses are still 31.4% of lookups and still walk
frames they will never match in. What is now also known is that each of those
walks is short, so anything that helps has to remove the CALL rather than
shorten the scan — which is the global-name skip, and which needs a per-ident
answer with no hashing in it. src/demand.rs is the precedent for a
whole-program side table, and it does not fit: `is_lazy_bind` is asked once per
statement where this would be asked 1,056,329 times. `Expr::Ident(Name, Span)`
has nowhere to keep a bit, and giving it one is 278 match sites across sixteen
files. That is the size of the thing, measured rather than guessed, and it is
the honest reason nothing is built here tonight.

The spec written for the filter went with it. It pinned that a pooled frame
rebuilds its filter with its slots — a real hazard while the filter existed,
since kanso#1545 keeps one node across dispatches — and it was watched red on
exactly that omission, `error[runtime]: unknown name `b``. With no filter there
is nothing for it to pin.

---

## 2026-09-19 — twenty candidates a dispatch was two point six, and the ratio said so

**A CORRECTION, on three surfaces including a published page.** The dispatch
loop's own comment, the kanso#1538 log entry and compiler.html all said arm
selection tries "roughly twenty" candidates a dispatch. Measured on the
interpreted corpus:

    dispatches      175,254
    candidates      456,967    2.61 a dispatch
    match_one     1,135,058    6.48 a dispatch, 2.48 a candidate
    of which bind   817,017    72% of match_one calls push a binding

**WHERE THE TWENTY CAME FROM, which is the useful part.** The log entry shows
its working: "1,100,726 calls to `match_one` against 55,711 dispatches". That
is 19.75, and it is `match_one` CALLS a dispatch. `match_one` runs once per
PARAMETER of each candidate tried, so calls a dispatch bounds the candidates
from ABOVE and equals them only if every arm took exactly one argument. The
denominator was right and the numerator was counting something else.

**THE OLD FIGURE'S OWN ARITHMETIC AGREED WITH THE SMALLER NUMBER.** kanso#1538
measured the misplaced reserve at 12,017,902 instructions. Over 175,254
dispatches that is 68.6 a dispatch, which is 26.3 per reserve at 2.61
candidates — a plausible `Vec::reserve` — and 3.4 at twenty, which no reserve
costs. The contradiction was sitting in the same paragraph for a fortnight.

**AND IT IS THE THIRD TIME TODAY**, after the loser's buffer and the frame
filter, that a count was used with the wrong denominator. Those two divided a
run total by nothing at all and read a share as a magnitude; this one divided
by dispatches and labelled the answer candidates. The habit is the same: a
count is worth what its denominator says, and the denominator has to be the
thing being counted.

**WHAT CHANGES AND WHAT DOES NOT.** kanso#1538's conclusion stands — the
reserve belongs once per dispatch rather than once per candidate, and moving it
was worth 21,582,351. What changes is the SIZE of the explanation: the reserve
was misplaced by a factor of 2.61, not twenty, and its 12 million came from a
reserve call that is not cheap rather than from being paid twenty times.

**AND IT RETIRES A LEAD.** A candidate loop trying twenty arms to find one
looks like an obvious place for a pre-filter on the first argument's type. At
2.61 tried and 202,987 of 456,967 matching — 44.4% — there is little to filter:
the loop already tries barely more arms than it accepts. That idea is closed by
the measurement rather than by an attempt.

---

## 2026-09-19 — where the production run spends, measured because the objective says to look there

**OPEN, measured, nothing built.** Three changes tonight took 15.8 million off
the interpreted row and moved welfare 77.25 to 77.27. That is the model working:
`interp_instructions` sits on the DEVELOPMENT side and is 136% improved against
its baseline, so it is deep into its satiating curve. `scripts/welfare/welfare.kso`
says where the money is in one line:

    d_run_speed = dimension run_speed_counters "run speed" 2.0 0.45

Satiation 2.0 — late, so doublings keep paying — and 0.45 of the production
side. Three entries tonight ended with "a change wanting to move the number has
to find production work or an unsatiated term", and then the next change went
back to the interpreter. This is the first look at the other side.

**THE RUN PROGRAM, PROFILED.** bench/runbench is the whole production speed term
since the 2026-09-06 gavel. Its top frames, self cost, on this container:

    1,840,368,355   PROGRAM TOTALS
      392,547,176   21.33%   json/encode_onto
      154,484,748    8.39%   json/obj_key_start
       97,131,375    5.28%   json/parse_value
       91,704,604    4.98%   runbench/tally
       85,720,338    4.66%   json/array_delim
       84,209,220    4.58%   render_ryu
       77,645,700    4.22%   json/scan
       65,074,779    3.54%   json/str_escape
       61,672,983    3.35%   k_beat_iter
       49,334,986    2.68%   k_b_at

**AND DIVIDED, which is the rule the two declines tonight bought.** A share is
not a magnitude and a total says nothing about what one call costs:

    encode_onto     2,380,860 calls    164.9 instructions a call
    obj_key_start     784,179 calls    197.0
    parse_value     1,100,187 calls     88.3
    k_b_at            690,000 calls     71.5
    k_beat_iter     2,685,021 calls     23.0

`encode_onto` is the largest frame in the production workload by a factor of
two and a half, and 164.9 instructions a call is a lot for a function whose job
is mostly appending a few bytes.

**THE OBVIOUS SUSPICION IS WRONG, AND READING THE OUTPUT SAID SO BEFORE
ANYTHING WAS BUILT.** `encode_onto` is an overloaded group of EIGHT arms, all
of arity two — `true`, `false`, `json_null`, then `n:int`, `x:float64`,
`s:string`, `xs:[]some` and `m:map[string some]`. A document of mostly strings
and numbers looks as though it must reach its arm past the three nullary tests
every call, which would make arm selection a share of the 164.9.

The emitted IR settles it:

    %t9 = extractvalue %KValue %x1, 0
    switch i64 %t9, label %L7 [
      i64 2, label %arm0    i64 3, label %arm1
      i64 0, label %arm3    i64 1, label %arm4
      i64 6, label %arm5    i64 9, label %arm6
      i64 10, label %arm7   i64 7, label %L8
    ]

A jump table on the value's tag. Eight arms cost one switch, arm ORDER decides
nothing, and the only linear step is the record case at `L8`, one
`k_check_rec_fast` for tag seven. The dispatch is not where the instructions
are.

**SO THE 164.9 IS THE APPENDING.** Per call the common path is one
`k_not_failure` on the accumulator, the switch, and an arm body: a string is
`k_b_append_mut_byte`, `escape_onto`, `k_b_append_mut_byte` — three calls and a
whole escape pass — where an int or a float is one `k_b_append_rendered`. That
is real work rather than overhead, and it is why the frame is large.

**TWO CHANGES DIED TONIGHT FOR WANT OF THIS STEP.** The loser's buffer and the
frame filter were both built on a suspicion that the code would have refuted,
and this one was refuted for the price of reading eighty lines of IR. The lead
stays open and its shape has changed: what to attack in the encoder is the
appending path, and nothing here says that path is wasteful.

---

## 2026-09-18 — the object gets a name the run chooses, and the eleven has nowhere left to live

kanso#1512 closed the mechanism and said the fix belonged in a round of its
own. This is it.

`clang` writes its LTO object to `/tmp/<stem>-XXXXXX.o` with fresh hex every
run. `ld`'s LLVM plugin puts that path into a `StringMap`, and how far the
probe walks depends on the string. Twenty-two names, one binary, one corpus,
every other input held fixed and the output path absent each time:

    twenty names          5,163,341,031
    `4b8c1a`, `fedcba`    5,163,341,042

Eleven apart, deterministic per name — `4b8c1a` was run four times in all and
read the high value every time — and about one name in eleven. That is the
eleven `codegen_instructions_release` has been disagreeing with ITSELF by ever
since kanso#1507 pinned the plugin's thread count and took the drift from
millions down to this. kanso#1502's round drew both buckets inside one job,
6,820,866,344 and then 6,820,866,355, on a branch whose diff is two divisions
in float rendering.

**Three rounds were spent on it, and one was spent asserting the name was not
the cause.** That assertion rested on three samples, then five, of a
one-in-eleven effect. The lesson is the log's own and it now has a fourth
entry: a report that something is ABSENT is worth what the search for it being
PRESENT was worth.

`-save-temps=obj` derives the object's name from the input, so the string is
the same every run. It rides `KANSO_FIXED_TEMPS`, which only the gate sets,
for the reason `KANSO_LTO_JOBS` is not the default either: it leaves a file in
the user's directory and a user's release build has no row to keep.
`tests/the_measured_link_names_its_object.rs` pins three things — the flag is
conditional on the variable, the two pins travel together on every `env -i`
line, and `clear_output` removes the object the flag leaves behind.

**That last one is kanso#1512's lesson applied to this change's own leavings.**
A saved object is state the next build would find at a known path, which is
exactly what clearing the binary and the IR was for. All three go.

**And the spec caught its own first shape.** Counting the sites and asserting
three passed when one site lost the variable: a comment naming the pair made
the count four, so dropping one left three. The assertion is per line now —
every executable `env -i` that pins the threads pins the name too, and the
reverse — and it was watched red in both directions.

Cost, as the container reads it: the whole pipeline goes 6,813,182,505 (three
identical runs without the flag) to 6,811,830,244 (two identical runs with
it), about 1.35 million lower. **A MEASUREMENT CHANGE and not a compiler
saving** — nothing about the compiler moved, and what the flag buys is a link
whose object has the same name twice. Both codegen goldens carry the old value
with the change named in the header; CI moves them.

## 2026-09-18 — kanso#1513's four layout rows, written in; the two codegen rows are the decision

CI's sitting on the tree merged with main after kanso#1515:

      compile           35,443,611 ->    35,443,609        -2
      entry            126,354,834 ->   126,354,832        -2
      library          126,810,299 ->   126,810,297        -2
      start-up           3,363,774 ->     3,363,770        -4

Minus two, two, two and four. `-save-temps=obj` rides `KANSO_FIXED_TEMPS`,
which only the codegen gate sets, and none of these four routes links anything;
what moved is the `match` in `release_clang` sitting in the binary they carry.

**Two rows are left alone on purpose.** `release-tier codegen` reads
6,833,786,335 against the golden's 6,824,133,280, and writing that row in IS the
floor drop this branch is waiting on — a drop bought by measurement
infrastructure rather than by the specification, which the 2026-09-13 rule
leaves with Clay. `dev-tier codegen` also disagrees, and the comment on the pull
request claimed it could not: the claim was that `-save-temps=obj` never reaches
the dev tier. One of those two is wrong and this entry does not say which,
because nothing here isolated it. That is the next measurement on this branch,
not a sentence.

## 2026-09-18 — kanso#1513 re-merged onto main after kanso#1516

The four layout rows carry MAIN'S values again; they had been written in at
CI's readings one main ago and kanso#1516 has moved the binary under them since.
The two codegen goldens did not conflict, so the branch still carries main's
`codegen_instructions_release` against CI's 6,833,786,335 — which is the whole
point and the whole blocker.

Re-merged rather than left dirty because a dirty pull request gets no CI at all,
and a board with nothing on it reads exactly like a green one. Nothing about
the decision has changed.

---

## 2026-09-19 — kanso#1513, CI's rows for the fixed-temp pin, and what it costs

    entry_instructions       126,729,588 ->   126,732,646     +3,058
    library_instructions     127,186,008 ->   127,188,882     +2,874
    interp_instructions      932,183,914 ->   932,183,929        +15
    startup_instructions       3,363,916 ->     3,363,672       -244
    codegen_instructions_dev 596,157,624 ->   596,153,756     -3,868
    codegen_instructions_rel 6,824,133,280 -> 6,833,786,335 +9,653,055
    emit_instructions         51,617,476 ->    51,619,793     +2,317

Seven rows, all re-based rather than regressed: the pin changes src/main.rs, so
the binary's layout moves and every row that tracks layout moves with it.

**EVERY ROW IN THIS JOB REPRODUCED ON A SECOND READING**, including
`codegen_instructions_release` at 6,833,786,335 twice. That is the change
working rather than a detail of it. In the same sitting, on a branch WITHOUT
the pin, kanso#1502's release row read 6,820,866,355 and then 6,820,866,344 —
eleven apart, same binary. The two branches are the controlled comparison the
question needed: one draws two values in a job, the other draws one.

**THE PRICE IS 0.003 POINTS**, and it is the release-tier row: 6,826,827,769 ->
6,833,786,335 against the baseline, 9.65 million instructions of clang and ld.
Welfare falls a hundredth below the floor.

**AND THAT IS NOT A FLOOR THIS SESSION WILL LOWER.** The ironclad exception
covers a change that builds a ruled part of the LANGUAGE, and this is an
instrument. There is a real argument that the 2026-09-15 normalization
ruling — "you do something that puts it into a persistent known initial
state" — already covers it, which is exactly what the pin does to the
temporary's name. What makes that argument premature is that the SCOPE of the
pin is itself the open question in design/pending-gavels.md: lowering the
floor to make this green would settle by action a question already sent to
Clay, and the tier the pin covers is what decides how much it costs.

So the rows go in, the cost is now a measured number rather than an unknown,
and the entry waits. The gavel is better informed than it was: 0.003 points,
against a row that stops drawing two values in one job.

---

## 2026-09-19 — the eighth row, and how seven got copied and one did not

`compile_instructions` 35,550,010 -> **35,551,167** on kanso#1513's merged
tree, +1,157, with `compile_again` reading 35,551,167 in the same job. The
eighth row the fixed-temp pin re-bases, and the one that was missed when the
other seven were copied in.

**WHY IT WAS MISSED, which is the part worth keeping.** The seven were taken
from the job's `::error::` lines, and this row did not appear among them
because it failed in a later round, after the seven had been rebased. The
summary block's own vein list named it — `"compile instructions:failure"` — and
that block is the authority CLAUDE.md points at for exactly this reason. Read
the vein list, not the error lines.

And the movement itself was predicted in writing: `compile_instructions`
USUALLY moves on an edit to the compiler's own Rust, because src/main.rs IS
the compiler whatever the front end stops before. A pin that changes main.rs
moves the layout and every row that tracks it.

**WHAT THIS CLEARS AND WHAT IT DOES NOT.** With the eighth row in, every vein
agrees and the trend gate is satisfied. Welfare still falls below the floor by
the 0.003 points the release tier costs — so that blocker is now confirmed
alone rather than merely asserted while another failure sat underneath it. The
entry waits on the scope ruling in design/pending-gavels.md, as the previous
entry says.

---

## 2026-09-19 — kanso#1513's eight re-based rows are superseded, and why they still go

kanso#1543 landed the dispatch-pooling family, so main's compile-side rows
moved again and this branch's merge conflicted on six of them. All six take
MAIN's side, which throws away readings CI made on this branch's own merged
tree a few hours ago.

**THAT IS THE RIGHT CALL AND IT IS WORTH SAYING WHY**, because the rule this
session wrote could be read the other way. The carry-forward rule says keep the
value CI measured on a tree this one DESCENDS FROM. This branch's own eight
rows were measured on a tree that descended from main-before-kanso#1543, and
main has moved since; they are measurements of a tree that no longer exists on
either side of the merge. Main's rows are the newest CI reading on a genuine
ancestor, so they are the carry-forward, and the pin's effect on them is
re-measured by CI on the merged tree.

So the eight figures the previous entry recorded stand as history and not as
this tree's rows. What they established does not move: every one of them
reproduced on a second reading in the same job, including
`codegen_instructions_release` at 6,833,786,335 twice, against kanso#1502 in
the same sitting drawing 6,820,866,355 and then 6,820,866,344. The pin works.
What is not yet known a second time is the SIZE of its re-basing on top of the
pooling, and only CI can say.

The floor is unchanged and still below its mark by the 0.003 points the release
tier costs. That remains the one blocker and it remains a scope question in
design/pending-gavels.md.

## 2026-09-19 — the dev row's 3,868 is the environment block, and the question it was sent to Clay over is answered here

The ledger asked whether the fixed-temp pin should cover both codegen tiers or
the release tier alone, and rested the whole question on one objection: pinning
both re-bases the dev row on a move with no mechanism behind it, because
`KANSO_FIXED_TEMPS` is read in `release_clang` and never reaches `dev_clang`.
That reading of src/main.rs is right. The conclusion drawn from it was not.

`scripts/gates/codegen_instructions.sh` sets the variable on all three of its
`env -i` lines, so the dev run's environment block grew by nineteen bytes, and
a child process inherits that block on its initial stack. The row IS the child
tree — kanso's own process is excluded from it, which the gate says at line 169
— so the block is the only thing this change alters for the processes the row
counts. Three arms at the dev tier, one tree, two passes reproducing byte for
byte:

    KANSO_FIXED_TEMPS=1      595,943,218   the gate as written
    nothing                  595,947,057   main's block
    KANSO_FIXED_TEMPX=1      595,943,490   nineteen bytes, read by nothing

A variable `dev_clang` never reads moves the tree 3,839, and a variable NOTHING
reads moves it 3,567, against the 3,868 CI reads on this row. The two arms
differ by 272 on content alone at the same length, which is the block again.
The container's figures are its own and an instruction delta does not travel;
what travels is that the unread arm moves nearly as far as the pinned one.

So the dev row's move is explained, the objection is answered, and option 1 —
pin both tiers, as kanso#1513 is written — stands. The golden's own header had
explained the move as layout from the changed src/main.rs, which cannot be
right for this row for the same reason: kanso's process is excluded. That
paragraph is corrected in place.

**This question should not have gone to Clay at all.** design/pending-gavels.md
opens by saying an entry is there because it is about the language a user
meets, and that implementation details do not come there — whoever holds the
file decides them and answers for the decision in the log. A gate's environment
variable is an implementation detail by any reading. The entry is withdrawn on
that ground rather than ruled, and the decision is recorded here. It sat
blocking a green pull request for a day, which is what filing it cost.

**And the floor takes the pin's 0.003.** CI read the pinned release row at
6,833,786,335 against 6,824,133,280, a re-basing of 9,653,055: `-save-temps=obj`
writes the object beside the output instead of into a temporary, and the link
does that work either way but now writes it where the name is fixed. The
compiler is not slower and the counter measures a different build, which the
objective cannot see, so it prices the re-basing as a regression. 0.003 points,
banked with the reason, under the 2026-09-15 rule that external state is put
into a known initial state rather than explained afterwards.

`tests/the_digest_is_priced_on_both_sides.rs` went red on the other host over
the same 0.003 — `the_undoctored_goldens_hold_the_floor` runs welfare against
the tree's real goldens — so the macos failure and the welfare job were one
failure with one fix.

**The eleven, caught again while this was being decided.** kanso#1551 is a page
and log branch with no code in its diff, and its cost-goldens job read the
release row at 6,824,133,291 and then 6,824,133,280 — eleven apart, in one job,
with `ld` carrying all of it: 5,142,849,464 then 5,142,849,453. That is the
third branch to halt on this and the second to show the residue is exactly
eleven. It is also the argument for landing the pin rather than leaving the row
to draw: the cost is 0.003 points once, and the alternative is about one job in
eleven going red on nothing a diff can explain.

---

## 2026-09-19 — a variable nothing reads moves the interpreted row by 117

STATUS.md's normalization row has carried the same Owes since 2026-09-15:
measure cloud's candidate for the six instructions two CI jobs disagreed by on
`interp_instructions`, or replace it. The candidate was an argument rather
than a measurement — that where the allocator's heap starts moves with the
size of the file the loader mapped, so a term proportional to work fits where
a constant does not. This measures the half of it that can be measured here.

Two arms, one tree at `0d164b55`, the interp gate's own anchor and exclusions
(`run_interpreted_on_stack`, the printed line subtracted), each read twice and
byte-identical both times:

    env -i PATH=... GLIBC_TUNABLES=...                      972,776,892
    env -i PATH=... GLIBC_TUNABLES=... KANSO_FIXED_TEMPX=1  972,777,009   +117

Nothing in the tree reads `KANSO_FIXED_TEMPX`. The variable is nineteen bytes
of environment block, which every child carries on its initial stack, and the
row moves 117 instructions for it. The allocation counters do not move at all:
`interp_allocs` 1,063,795, `interp_alloc_bytes` 75,247,333 and
`interp_peak_bytes` 834,079 are byte-identical across the two arms. So the
interpreter asked the allocator for exactly the same things in the same order
and the row still moved.

**And the carrier is named, because the term is exactly linear in the count.**
Four arms, each read twice and byte-identical:

    0 extra   972,776,892
    1 extra   972,777,009   +117
    2 extra   972,777,126   +117
    3 extra   972,777,243   +117

117 a variable, to the instruction. Layout does not do that — adding bytes to a
block moves an address once and by whatever the alignment says. A scan does.
Diffing the two profiles frame by frame names it: the frames that move are
`getenv`, `__strncmp_avx2`, and the allocator's own `_mi_prim_getenv`,
`_mi_strnicmp` and `_mi_toupper`. mimalloc resolves its options by name, each
lookup walks the environment block comparing as it goes, and one more variable
is one more comparison in every walk. Most of that is start-up, which the
anchor excludes; the 117 is the part inside it, so the interpreter's own thread
resolves an allocator option while it runs.

**117 against a six.** The residue two CI jobs disagreed by is three parts per
billion of this row. It is not this term — between two jobs on one commit the
environment is identical, and six is not a multiple of 117. What this settles
is that the row carries a term of that shape, twenty times the size of the
disagreement, with a mechanism behind it rather than a suspicion. And it is the
2026-09-15 rule's own case: the state can be put into a known one, so the
reading stops depending on it.

**The corpus-size arms are contaminated and are reported as such.** Growing
the corpus file with comment lines the interpreter never runs also moved the
row — 65 instructions for 62 KB and 4,484 for 124 KB — and that looked like
the candidate's own claim about the mapped file. It is not clean. The bigger
file really does cost the loader more: `interp_allocs` 1,063,795 -> 1,063,800,
`interp_alloc_bytes` +187,488, `interp_peak_bytes` +78,649. The anchor excludes
the loader's frames and does not exclude the allocator state the loader leaves
behind, so those arms mix a layout move with a real one. The environment arm is
the clean one and is what the finding rests on.

Also worth the sentence: the two contaminated arms are not linear in file size,
65 then 4,484, so whatever they move is a boundary rather than a cost per byte.
The gate pins `glibc.malloc.mmap_threshold` at 131,072 and the larger corpus is
124,621 bytes, which is close enough to be the first thing to look at and is not
evidence of anything yet.

**What the row still owes.** The six. The environment term has a mechanism and
a normalization; what carried the disagreement between two jobs whose
environment was identical is still open, and the frames that moved between the
contaminated corpus arms — `_mi_os_commit_ex`, `mi_bitmap_setN`,
`_mi_prim_commit` — are where to look next. Those are the allocator committing
pages, which is a different question from the allocator reading its options.

---

## 2026-09-19 — frames are 6.57% of the production run, and the threshold that would cut them costs more than it saves

Three entries this week ended by saying a change wanting to move the objective
has to find production work. This is what the production run spends getting
into and out of functions, and what happens when the one lever that moves it is
pushed.

Measured on runbench at `0d164b55`, callgrind with `--dump-instr`, whole
program 1,840,276,313 — two profiles agreeing to the instruction. Summing, over
every function whose entry block ran, the leading callee-saved pushes and the
stack reserve, plus the pops' own measured cost:

    prologue   64,847,962   3.52%
    pops       56,140,300   3.05%
    together  120,988,262   6.57%

A floor rather than a total, because the epilogue's `add $N,%rsp` is not
counted. Larger than `render_ryu` at 4.58% and larger than the whole beat
machinery at 5.09%, and until now it had no name.

    30,952,350  d_json/encode_onto_2     2,380,950 calls, 7 prologue instructions
    14,303,721  d_json/parse_value_2     1,100,286
    10,760,607  d_json/obj_key_start_4     827,739

`encode_onto` is 21.33% of the run and 164.9 self instructions a call, twelve
in and eight out. Reading its jump table and counting entries at each target
gives the arm mix: the number arms take 379,530 calls and are four movs and a
call; `true`, `false` and `null` take 562,500 between them and are five
instructions each into a shared append; strings take 942,750; maps and lists
the rest. So 39.6% of the calls into that dispatcher pay a frame sized by the
arms they do not take.

**Shrink-wrapping is not the lever, and that was checked rather than assumed.**
Compiling `runbench.ll` with `-mllvm -enable-shrink-wrap=false` gives a
byte-identical prologue, so LLVM is not sinking it either way. With a jump
table to eight arms and callee-saved uses in several of them, the entry block
is the only place that dominates them all.

**And splitting the dispatcher would not help, which is the other obvious fix
and is now measured.** The idea is to emit the cheap arms so they need no
frame and tail-call the heavy ones, on the reading that the dispatcher's frame
is sized by the arm bodies inlined into it. Linking `runbench.ll` by hand says
otherwise. Marking `d_json/escape_onto_2` `noinline` — the string arm, the
heaviest thing in there — takes the stack reserve from `0x58` to `0x38` and
leaves all six pushes. Marking EVERY call site inside the dispatcher `noinline`,
so no arm body is inlined at all, leaves all six pushes and `0x38` again.

So the registers are the dispatcher's own. It holds both arguments live across
the calls it makes — `k_not_failure`, `k_err_hop`, `k_check_rec_fast` — and a
`KValue` is two words, so the two arguments alone are four registers that must
survive a call. There is nothing there for an emitter-level arm split to remove,
and the idea is declined without being built.

**Inlining is the lever, and it is already where it should be.** The ladder
beside the flag in src/main.rs was measured against 1000 and stops at 3000;
these two rungs are new, on a `runbench.ll` byte-identical across every arm, so
the only difference is what the linker's LTO was told:

    225    1,938,999,983   343,128 bytes   22,317,482 calls
    2000   1,840,276,313   424,088         18,812,341
    4000   1,823,291,354   510,152
    8000   1,821,134,592   567,496

225 to 2000 removes 3,505,141 calls and 98,723,670 instructions: 28.2 a call,
which is a frame plus the call and the return. That is the mechanism, priced.

**And 4000 is declined, on the objective rather than on `.text`.** The comment
that chose 2000 gave binary size as the reason, and machine-code size has no
welfare term — Clay ruled that on 2026-09-05 — so that reason could not have
decided it. The scored reason is `codegen_instructions_release`, weight 0.15 on
the production side. Same box, two passes, each arm byte-identical:

    release codegen   6,827,333,184 -> 7,075,918,942   +248,585,758   +3.64%
    dev codegen         595,943,218 ->   595,943,218   byte-identical

The dev tier cannot move: `dev_clang` passes `-O0` and no `-mllvm`.

Scored by applying each ratio to the goldens, the run gain alone takes welfare
77.27 to 77.33, and the pair together to **77.26** — one hundredth below the
floor. The break-even was worked out before the codegen arm finished, and
written down then so it could be scored rather than fitted: the raise pays only
if the release build costs under 3.2% more. It came in at 3.64%.

It is worse than that once travel is allowed for. The same comment records that
CI's work row moved 59% of what this container's ladder projected for 1000 to
2000. At that travel the run gain is nearer 0.54% and the trade is not close.

**The comment beside the flag held half of this and it was not read first.**
The ladder, the non-monotonicity and the 59% travel factor were all one file
away, and two arms were built before anyone looked. What the arms added is real
— the codegen price and the welfare verdict, neither of which was there — but
the rule stands and this is another instance of it: read the thing the number
describes before running anything against it.

---

## 2026-09-19 — what a point of welfare costs, measured on every counter the objective reads

Three sessions in a row have chosen what to work on by reading the weights and
the satiations and reasoning about them. The objective can be asked directly
instead. Stage `bench/` and `scripts/`, scale ONE golden row by 0.9, and run
`kanso run scripts/welfare -- --score`, which prints four places. The difference
is what a tenth off that row is worth in meta welfare. Fourteen rows, one at a
time, base 77.2707:

    per 10%   counter                        golden row
    +0.6644   run_instructions               instructions:runbench
    +0.5091   run_peak_bytes                 cost_golden_run:arena_peak_bytes
    +0.1952   codegen_instructions_release   codegen_instructions_release
    +0.0923   startup_instructions           startup_instructions
    +0.0692   codegen_instructions_dev       codegen_instructions_dev
    +0.0446   interp_instructions            interp_instructions
    +0.0233   compile_peak_bytes             compile_memory:compile_peak_bytes
    +0.0214   compile_instructions           entry_instructions
    +0.0197   interp_peak_bytes              interp_memory:interp_peak_bytes
    +0.0197   compile_allocs                 compile_allocs
    +0.0094   run_peak_bytes                 cost_golden_run:held_peak_bytes
    +0.0068   emit_instructions              emit_instructions
    +0.0060   compile_instructions           compile_instructions
    +0.0003   run_peak_bytes                 cost_golden_run:perm_peak_bytes

The same sweep at 1% gives the same order with every value about a tenth of
these, so the curve is near enough straight over that range and the table can be
trusted to rank even though each figure is an average over its own step rather
than a derivative.

**Two rows are two thirds of the board.** `runbench` and the run program's arena
peak come to 1.1735 of a 1.6808 total: 69.8%. Everything else together is worth
less than half of `runbench` alone.

**And the arena peak has had no work at all.** It sits at 38,604,496 bytes and
is worth 77% of what the run's instruction count is worth, which nothing in the
last fortnight's log would suggest. Every entry in that window is instructions:
the dispatch-pooling family, the beat rewind, the digit loop, the frames. The
second most valuable row in the model has not been named once.

**What this says about the fortnight.** `interp_instructions` is worth 0.0446 a
tenth, fifteen times less than `runbench`. The three dispatch-pooling changes of
2026-09-18 took 15.8 million off a 939 million row, 1.7%, and the objective
moved 77.25 to 77.27 — which is exactly what this table predicts and is why it
felt like so little for three merged changes. `compile_instructions` is worth
0.0060 a tenth, a hundred and eleven times less than `runbench`, and it is the
row this project has spent the most rounds arguing about.

The figures are marginal at today's ratios and move as the terms improve, so the
table is dated and belongs in the log rather than in a doc that reads as
standing. What would keep it current is a `--marginal` flag on the welfare
script itself, printing this table from the model it already holds. That is the
follow-up; the table above is the reason to want it.

## 2026-09-19 — the arena peak is three quarters one phase, and that phase is 4.9% of the work

The table above puts the run program's arena peak second on the board at 0.5091
a tenth. This is where it lives. One count at a time, taken to its floor, every
other count left alone, `arena_peak_bytes` read off the counters build:

    baseline              38,604,496   36 blocks
    decode = 1            38,604,496   36     unchanged
    encode = 1            38,604,496   36     unchanged
    decode = 1, encode = 1 38,604,496  36     unchanged
    deep = 1              38,604,496   36     unchanged
    escape = 1            38,604,496   36     unchanged
    pend = 1              38,604,496   36     unchanged
    index = 1,000         35,651,584   34
    digest = 1            35,458,768   33
    split = 1              9,244,368    8

**Split holds 29,360,128 of it, 76%, and split is 4.87% of the program's
instructions.** Decode and encode are 69% of the work between them and hold
none of the peak at all: taking both to a single round leaves the number
unmoved to the byte.

Staging `bench/` with `arena_peak_bytes` at 9,244,368 and scoring: welfare
**77.2707 to 82.0199, +4.7492**. Every compiler change merged in the two days
before this entry moved the objective by 0.02 together.

**A cause was written down eleven days ago, in the benchmark's own header.**
`bench/runbench/runbench/split/scanbench.kso` records a 2026-09-08 measurement:
codegen reads `beat_loops`, `beat_loops` drops every group whose file begins
`std/` or `lib/` from the carry tier — `src/beat.rs:196` — and clearing that
filter gives a peak of 1,048,576 bytes over one block with `alloc_bytes`
unchanged: the same allocation, now reclaimed.

The measurement is the header's and stands. The mechanism it names does not, and
the entry below reports the instrumentation: exactly eleven imported groups lose
a carry at that filter, and `regexp/walked/5` is not among them. It is classified
grow-only because another group tail-calls it, so it has no carry to strip and
the filter never reaches it. What clearing the filter changes is some other
loop's reclamation, and which one is open.

Clearing it wholesale is not the fix and was measured not to be: runbench was
still running after ten minutes against a 0.4-second baseline, because every
library loop begins evacuating, and the 2026-09-01 sitting priced that removal
at -0.56 welfare under the objective of the day. The header names the shape of
the real fix — "the carry tier being decided by a path prefix rather than by the
property the prefix stands in for" — and `src/beat.rs:185` says what the
property is: a shared library driver threads its caller's invariant source
through the loop, and carrying that copies an unbounded value every iteration.
The machinery for saying so already exists for clusters, as the threaded-slot
fixpoint in `cluster_edges_ok`.

So the lead is not new. What is new is its size: the largest single move on the
board by two orders of magnitude, against a fix whose shape is already written
down and whose crude form is already priced.

## 2026-09-19 — the first cut at the prefix filter, declined at 7.4x

The entry above says the carry tier is decided by a path prefix rather than by
the property the prefix stands in for, and that the machinery for the property
already exists as the threaded-slot fixpoint in `cluster_edges_ok`. The obvious
first cut follows from that: `cluster_edges_ok` already refuses to carry a slot
it finds threaded, so a carry that came out of the cluster analysis has already
been checked for the thing the filter guards against, and the filter could
exempt it.

Built, on a branch, in four lines: record which groups took their carry from
`eligible_clusters` and let those through both `carried.retain` and the
`ids.retain` beside it.

    arena_peak_bytes   38,604,496 -> 35,458,768    -3,145,728, worth +0.4127
    allocs              5,730,653 ->  6,550,655     +820,002
    alloc_bytes       459,964,461 -> 494,316,813    +34,352,352
    run instructions  1,840,276,313 -> 13,618,672,806   SEVEN POINT FOUR TIMES

Declined. The peak gain is real and the instruction cost is not survivable, and
the shape is the 2026-09-01 catastrophe in miniature — that removal left
runbench running after ten minutes against a 0.4-second baseline, and this one,
over the cluster subset alone, costs 7.4x.

**What it rules out is worth having.** `cluster_edges_ok` excludes threaded and
chain-threaded slots from the carry before it returns, so a cluster carry has
already passed the test the filter's comment describes — and exempting exactly
those carries still blows up. So the threaded fixpoint as it stands is not what
the path prefix is standing in for. Whatever the real property is, "the cluster
analysis approved this carry" does not imply it, and the next attempt has to
find the difference rather than assume the two agree.

The split phase's 29,360,128 bytes are untouched by this cut, which is its own
evidence: `regexp/walked/5` is reported grow-only for an outside tail call, so
its carry comes through `demotable_entries` and `crossing_positions`, not
through a cluster. That path has no threaded fixpoint at all — `arg_ok` accepts
a bare parameter only when its inferred set is within THREADED, so a bare
parameter carrying ordinary heap becomes a crossing position and would be
evacuated every iteration. Giving that path the fixpoint is where the next
attempt goes.

## 2026-09-19 — eleven groups, and the walker is not one of them

The cut above was made twice, by two different criteria, and both came back with
byte-identical counters and byte-identical instructions: `allocs` 6,550,655,
`alloc_bytes` 494,316,813, `arena_peak_bytes` 35,458,768 over 33 blocks, and
13,618,672,806 instructions. Two changes agreeing to the byte is a thing to
explain rather than to report twice, so the filter was instrumented instead.

Under `KANSO_THREAD_REPORT`, building runbench, exactly ELEVEN imported groups
reach the point where the prefix strips a carry:

    json/array_open/3    carry [1]     regexp/more_flags/4     carry [2]
    list/holds_all?/2    carry [0]     sha256/compress/4       carry [0, 1]
    list/found_in/2      carry [0]     regexp/leading_flags/3  carry [2]
    list/holds_any?/2    carry [0]     sha256/turned/3         carry [0, 1]
    json/obj_open/3      carry [1]     sha256/blocked/3        carry [1]
    sha256/digested/4    carry [1]

Both cuts therefore did the same thing — let those eleven carry — which is why
they agreed. 7.4x is what those eleven cost, against 3,145,728 bytes of peak.

**`regexp/walked/5` is not in the list, and cannot be.** Its report line reads
`grow-only: another group tail-calls it (unbracketed entry)`, so `classify` gives
it `GrowOnly`; `demotable_entries` only considers `OutsideTailCall`, and the
carry tiers only ever see a group that got one. A group with no carry has none
to strip.

So the scanbench header's measurement stands and its mechanism does not. Clearing
the filter really does take that benchmark's peak to one block — the header
measured it — but not by restoring the walker's carry, because the walker has
none. What it restores is some other loop's, and naming that loop is the next
step rather than a detail: the 29,360,128 bytes are still where they were, and
the instrumentation says they are not behind this filter in the way the header
says they are.

Recorded rather than left, because the header's sentence has been read three
times now as a ready-made diagnosis, including once in the section above this
one before the instrumentation ran.

## 2026-09-19 — the carry's width is the property, and one test says not yet

The eleven were priced one at a time, with a probe that lets a named imported
group keep its carry. Alone, every one of them reads the baseline on both
columns: `arena_peak_bytes` 38,604,496 and 0.26 seconds. So the cost is a
pairing rather than a group, and the pairs separate cleanly:

    sha256/compress/4 + sha256/turned/3     35,458,768   1.01s
    sha256/blocked/3  + sha256/digested/4   35,458,768   0.26s
    the other seven                         38,604,496   0.27s
    all eleven                              35,458,768   0.95s

**The whole peak saving sits with the cheap pair.** `blocked` and `digested`
give the entire 3,145,728 bytes at baseline wall time; `compress` and `turned`
give the same bytes and all of the cost. Under callgrind the cheap pair reads
1,841,054,483 instructions against main's 1,840,276,313 — +778,170, +0.0423% —
with `allocs` up 251 and `alloc_bytes` up 20,080. Scored: **77.2707 to 77.6807,
+0.4100.**

The first pair carries TWO positions each and the second ONE. So the width of a
carry reads the property the path prefix was standing in for: evacuating one
slot a lap is what the tier is for, and evacuating several is where a library
loop starts copying its caller's work. It is a proxy for bytes copied per
iteration, which nothing in the pass can measure, and it is a proxy the loop's
own shape supplies rather than its file name. Built as "an imported group keeps
a carry of at most one position": 1,841,081,961 instructions and the same
35,458,768, scored 77.2707 to 77.6806. The five extra groups it admits beyond
the named pair cost 27,478 instructions between them.

**And it does not ship tonight, because a test goes red.**
`beat::tests::json_decode_loops_stay_conservative`: the rule admits
`json/array_open/3` and `json/obj_open/3`, and that test asserts only the
byte-builder encoders may rewind, because "scanners threading records or lists
stay on the grow-only arena". The other 61 tests pass and the differential
corpus is green.

**What that test pins is worth reading carefully, and the first reading here was
wrong.** It looks like a safety judgement about freeing memory under a live
reference. The prefix it protects is a COST guard, and says so in its own words
at `src/beat.rs:185`: carrying a shared library driver's threaded source "copies
an unbounded value per iteration". Safety is established elsewhere and still is
— the `THREADED` set with its list argument, the map exclusion, the bytes chain
licence — and those run whatever the prefix does.
`bounded_accumulator_carries` pins a carried list directly: a fixed-shape
rebuild carries, and the evacuation handles it.

So what the next session owes is not a proof of memory safety. It is a decision
about an expectation that was written when the rule was a file path, with a
measurement now saying those two groups cost nothing either way. Changing a test
because the rule beneath it changed is ordinary; changing one to get green is
not, and this entry exists so the difference is on the record before anybody
edits it.

The measurement says those two json groups contribute nothing either way: the
seven non-sha256 groups read the baseline on both columns. So the whole +0.41 is
available without touching them, and what the next session owes is a rule that
admits the sha256 pair on a property rather than by name, leaves json's scanners
where that test wants them, and says why the difference is real.

## 2026-09-19 — carry width is not the property, and the ratchet says so in three voices

`beat_loops` decides which imported loops may evacuate their slots and rewind by
asking whether the declaration's `file` begins `std/` or `lib/`. That field is
the one error origins are built from, and it is deciding a program's memory: the
same package under a directory called `lib` compiles to one that never reclaims
a block, and one directory over to one that does.
`tests/a_program_is_not_its_directory.rs` has pinned that since it was written,
as a defect.

The prefix stands in for something real. A shared library driver threads its
caller's invariant source through the loop, and evacuating that copies an
unbounded value every lap; removing it outright on 2026-08-31 turned the digest
quadratic, 1.3s to 68s at 128 KB.

MEASURED, one group at a time, on runbench. Eleven imported groups lose a carry
at that point. Alone, every one reads the baseline on both columns — peak
38,604,496 and 0.26 seconds — so the cost is a pairing:

    sha256/compress + sha256/turned      peak 35,458,768   1.01s
    sha256/blocked  + sha256/digested    peak 35,458,768   0.26s
    the other seven                      peak 38,604,496   0.27s

The expensive pair carries two positions each and the cheap pair one, and the
whole 3,145,728 bytes sit with the cheap pair. A rule admitting a carry of at
most one position was built, measured at 77.6807 against a 77.2707 floor, and
opened as kanso#1556.

**IT IS DECLINED, and the reason took three translations to read.** Under the
rule `kanso run scripts/ratchet` reports `the program ran out of stack:
recursion went deeper than the stack holds`. The binary it runs prints
`out of memory`. Printing the request at the point of failure gives the third
and true version:

    CARRYOOM need=18446744072171062384 depth=3 carry_n=1

2^64 less 1,538,489,232. `k_copy_size` read a length of about minus one and a
half billion out of a node it was sizing for the staged copy, the sum wrapped,
and `malloc` refused it. What the prefix keeps out is a carried value whose
interior the sizing walk cannot read, and the WIDTH of a carry does not see
that at all. The rule was a proxy chosen from a pairing, and the pairing was
about wall time.

The sweep over which groups have to be rescued for the failure to appear:

    {holds_all?, holds_any?}                     dies
    {holds_all?, holds_any?, next}               dies
    {holds_any?, next, next_skipped}             dies
    {holds_all?, holds_any?, next, next_skipped} dies
    all five                                     dies
    every other subset tried                     runs

`list/holds_any?/2` is in every coalition that dies and in none that survives.
It drives `any?`, and the backtrace at the failure has one `any?` running inside
another's predicate — `ratchet/read_it` to `any?` to `holds_any?` to the
predicate closure to `any?` again. Excluding the three list drivers that take a
predicate runs the ratchet clean AND still reads arena_peak_bytes 35,458,768
against main's 38,604,496, so a rule shaped that way would ship the whole
saving.

**That rule is not shipped, because there is no small program that fails without
it.** A nested `any?` over two 800-element lists reads 1,048,576 on both arms.
The coalitions are not monotone: {holds_all?, holds_any?} dies where either
alone runs, and {holds_all?, holds_any?, next_skipped, found_in} runs again —
which is a failure sitting near a memory limit rather than a rule being broken,
and it means a subset sweep cannot say which group is unsafe, only which one is
over the line in this program. A guard swept out of one program is a guard
nobody can check.

What this branch leaves behind: the eleven groups and their carries, the pairing
measurement, the number the sizing walk actually read, and a next step that is
smaller than the one it started with — find the program that makes `k_copy_size`
read a garbage length. The rule follows from that.
---

## 2026-09-17 — the digit loop carried a value it only needed at the end, and then the tail gave it back

`render_ryu` is 84,209,220 instructions of runbench, 4.58%, 440.7 a float over
191,070 calls. A quarter of that is one loop taking digits off two at a time,
and it divides three values a trip — `vp` and `vm` to decide whether another
pair comes off, and `vr` because `vr` is the answer.

Only the first two decide anything. `vr` is carried through and read once at
the end, so it comes out: count the pairs with two divisions a trip, then take
`vr` down in one step. Measured against main `3120df0e`:

    runbench    1,840,368,292 -> 1,837,534,164   -2,834,128   -0.1540%
    render_ryu     84,209,220 ->     81,375,750   -2,833,470   -3.365%
    a float             440.7 ->          425.9        -14.8
    .text             319,346 ->        317,954       -1,392

The whole of the fall is inside that one function, to 658 instructions, and
`.text` comes down by the same 1,392 on every benchmark because the loop is
one piece of code they all share.

### the first shape gave most of it back, and the disassembly says where

Taking `vr` out of the loop was worth only **924,584** on its own — 4.8 a
float. The loop really did get cheaper; the tail ate it. Disassembled, main
against that first shape:

    main   0x3e200..0x3e242   21 instructions, 3 mul, 7 mov
    first  0x3dc50..0x3dc7d   15 instructions, 2 mul, 5 mov

**Six instructions a trip**, and at 5.35 trips a float that is 32.1 — against
4.8 measured. The tail was costing back twenty-seven.

It was dividing twice by a table entry:

    round_up = (vr / RYU_POW100[pairs - 1]) % 100 >= 50;
    vr /= RYU_POW100[pairs];

Two divisions by a value the compiler cannot see, so two real `div`
sequences. But `RYU_POW100[pairs]` is `RYU_POW100[pairs - 1] * 100`, so
dividing by the smaller one first leaves both remaining steps with a CONSTANT
divisor, and a constant divisor is a multiply-high:

    uint64_t q = vr / RYU_POW100[pairs - 1];
    round_up = (uint32_t)(q % 100) >= 50;
    vr = q / 100;

One variable division instead of two. That recovers **1,909,544 more
instructions** and takes the change from 4.8 a float to 14.8 — three times the
first shape's worth.

### the lead this answers, and the claim of mine it corrects

The 2026-09-14 entry named seven of the loop's twenty-one instructions as moves
shuttling `vp`, `vm` and `vr` around the back edge, sized them at 37
instructions a float, and left them.

I first explained the result by saying three divisions cost six moves whatever
the C says, so removing a division removes two. **That was asserted, not
counted, and the disassembly above does not support it**: the loop carries
seven moves with three divisions and five with two. Removing one division
removed two moves and one multiply-and-shift pair — six instructions a trip,
which is close to the entry's seven and confirms its reading of the loop. What
the entry could not have known is that the saving is only collectable if the
step replacing the loop is cheaper than what it replaces.

### the harness found two bugs before either number was taken

`tests/every_rendered_float_reads_back_as_itself` sweeps 2,809,326 values
against `strtod`, lifting `ryu_d2d` and `render_ryu` out of `src/runtime.c`
rather than copying them.

The power table was written with nine entries, which is one short: a u64 sits
below 1.9e19, so nine pairs can come off and index nine is the last the step
reads. The harness crashed on that before any measurement.

Watched red a second time, deliberately, on the subtlest thing the step could
get wrong — `round_up` reading `RYU_POW100[pairs]` rather than `[pairs - 1]`,
one pair over: **222,173 of 2,809,326 did not read back**.

- **DONE** built, swept, measured twice, and the second shape is what ships.
  Rows are CI's to take.
- **OPEN** the rest of `render_ryu`. At 425.9 a float it is still the largest
  leaf in the run program after the four kanso-level frames.
## 2026-09-18 — kanso#1502 on the merged tree, and the `.text` claim was the wrong sign

CI's sitting, run side first. Five rows moved and nine are byte-identical:

    encodebench  3,497,149,260 -> 3,485,406,060  -11,743,200  -0.3358%
    livebench    2,825,430,323 -> 2,813,687,123  -11,743,200  -0.4156%
    runbench     1,821,933,936 -> 1,819,291,716   -2,642,220  -0.1450%
    oneshot         17,888,155 ->    17,858,797      -29,358  -0.1641%
    widebench       33,516,094 ->    33,548,094      +32,000  +0.0955%

encodebench and livebench fall by the SAME 11,743,200, to the instruction.
That is one kernel doing one job: both corpora render the same floats the same
number of times, and the digit loop's third division is gone from each of them
equally. The benchmarks that render no floats — jsonbench, basket, deepbench,
escapebench, pendbench, indexbench, scanbench, digestbench, readbench — do not
move a single instruction.

**The `.text` claim was the wrong sign.** This branch's entry records
`.text -1,360 bytes`. CI reads **+32 bytes on every one of the fourteen
binaries**, without exception. The earlier figure was taken before kanso#1478,
kanso#1493 and four others landed, on a tree where the surrounding code was
different; what the change does to the emitted size on today's main is add
thirty-two bytes. widebench's +32,000 instructions is that growth being paid
for by the benchmark whose working set it disturbs most, and it is the only
run row that rises.

The compile side moved eight rows, seven of them under two hundredths of a per
cent and all layout. The exception is `codegen_instructions_release`, up
1,758,635 (0.026%), which is the only row that COMPILES runtime.c rather than
carrying its bytes. `codegen_release_again` read 6,824,410,196 — the same
number in the same job, which is kanso#1507's thread pin holding on a third
tree that changes runtime.c.
Every row that rose, named by the counter the gate spells and the value it
landed on:

    codegen_instructions_release  6,824,410,196   +1,758,635  +0.0258%
    work_widebench                   33,548,094      +32,000  +0.0955%
    emit_instructions                60,206,729      +10,004  +0.0166%
    interp_instructions           2,182,584,048       +7,939  +0.0004%
    library_instructions            126,807,848       +3,423  +0.0027%
    entry_instructions              126,352,058       +3,018  +0.0024%
    compile_instructions             35,442,511       +1,484  +0.0042%
    startup_instructions              3,951,975         +179  +0.0045%
    text                            1,755,820 total       +448

`text` is the sum over the fourteen binaries and each one grew by exactly 32
bytes, so the total moves 448. Seven of the eight instruction rows are under
two hundredths of a per cent and carry runtime.c's bytes rather than compiling
it; the release row is the exception and the reason is given above.
## 2026-09-18 — kanso#1502's compile-side rows re-measured after kanso#1509

kanso#1509 landed under this branch, so the five compile-side goldens were
carried forward at main's values and the round re-measured them on the merged
tree. CI's sitting, with the second reading in the same job matching the first
to the instruction on all four rows that take one:

    compile_instructions    35,441,774 ->    35,443,452  +1,678   (+0.0047%)
    entry_instructions     126,350,802 ->   126,354,605  +3,803   (+0.0030%)
    library_instructions   126,806,203 ->   126,809,996  +3,793   (+0.0030%)
    startup_instructions     3,933,223 ->     3,933,390    +167   (+0.0042%)
    emit_instructions       52,115,454 ->    52,125,468 +10,014   (+0.0192%)

**All five are LAYOUT.** The two divisions this branch merges are in float
rendering, and none of these five routes renders a float: three of them are
`kanso check`, `emit_ir` stops before the backend, and the interpreted
start-up links the runtime without reaching it. Four of the five land within
five thousandths of a per cent of where they were, which is the size a shifted
binary moves a row that does not run the changed code.

The run-side rows this branch exists for are unchanged from its own sitting and
stand where its earlier entry recorded them; `interp_instructions` and both
codegen rows agreed with the goldens this branch already carries.
## 2026-09-18 — the eleven caught in the act, on kanso#1502's own job

kanso#1512's entry closed the mechanism on the container: clang names its LTO
object `/tmp/codegen_corpus-XXXXXX.o` with fresh hex every run, `ld`'s plugin
probes a `StringMap` with that path, and about one name in eleven lands a
bucket further. Twenty-two names were sampled there and exactly two, `4b8c1a`
and `fedcba`, read eleven more than the other twenty.

This branch's round drew both buckets inside one job:

    codegen_instructions_release  = 6,820,866,344
    codegen_release_again    row  = 6,820,866,355

Eleven apart, one binary, one corpus, one machine, with the gate re-staging
the box between the two. That is the same term, on the runner rather than on
the container, and it is the plainest evidence yet: the readings differ by the
exact figure the name experiment produces, on a branch whose diff is two
divisions in float rendering.

`codegen_instructions_dev` agreed with itself at 596,162,050 on both readings,
which fits — the dev tier does not run the LTO plugin.

The row written here is the FIRST reading, because that is the one the gate
compares against the golden. Until the object name is pinned, this row has
about a one-in-eleven chance per job of drawing the other bucket and going red
for no reason a diff can explain. Pinning it is its own change: `-save-temps=obj`
gives clang a deterministic object name, and on the container it also moves the
whole pipeline by 1,352,261, so it re-bases the row as well as steadying it and
belongs in a round of its own.

## 2026-09-18 — kanso#1502's rows on the tree merged after kanso#1511

CI's sitting on `8c5b50f0`, every row with the value it landed on:

    compile_instructions      35,444,548 ->    35,442,350    -2,198
    entry_instructions       126,359,513 ->   126,351,950    -7,563
    library_instructions     126,814,937 ->   126,807,146    -7,791
    startup_instructions       3,364,755 ->     3,363,766      -989
    interp_instructions    2,182,597,360 -> 2,182,579,844   -17,516
    emit_instructions         51,543,885 ->    51,546,941    +3,056

All six are layout. The branch's own source has not moved since the sitting
that banked the floor; what moved under it is main, which gained kanso#1511
and kanso#1512. Both codegen rows read their goldens exactly on this job —
6,820,866,344 release and 596,162,050 dev — which is the branch's own change
holding still while the compiler around it moved.

`emit_instructions` is the one that rose, by 3,056 instructions on 51.5
million, 0.0059%. `codegen::emit_ir` inclusive is a layout vein like the
other five: the emitter's decisions cannot change when the emitted code does
not, and `emitted` and `machine code` both agreed on this job.

Welfare scores 76.8343 against a floor of 76.83355375328496. The rise is
0.0008, inside the sentinel's 0.001 band, so there is nothing to bank.

**And the three compile spans on the page follow the goldens.** They drifted
on the first push of these rows because the sweep was not run, which cost a
round: `golden_prose` is the only one of the three page gates that reads a
`data-golden` span, and the two that were run cannot see one. A golden push
runs `sh scripts/gates/all_pages.sh` and the trend gate, whether or not it
feels like it touches a page.

## 2026-09-18 — kanso#1502 re-merged onto main after kanso#1515

kanso#1515 landed underneath this branch and moved the interpreted row
2,182,584,048 -> 2,168,428,538, a fall of 14,155,510. That row and the five
compile-side rows beside it now carry MAIN'S values, carried forward: this
branch's own readings were taken against a tree that no longer exists, and
carrying main's forward gives each gate one number to fail against rather than
none while making CI's diff read exactly what this branch does to today's main.

The floor is main's too, 76.82771395446468, banked by kanso#1515. The previous
round read a rise of 0.0008 against a floor of 76.83355375328496 and banked
nothing; that comparison was against the tree kanso#1511 left, and both sides
of it have moved. What this branch does to welfare is CI's to measure on the
merged tree, and the ratchet follows that reading rather than this one.

The three compile spans on the page follow the goldens, so they carry main's
values as well.

## 2026-09-18 — kanso#1502's rows on the tree merged after kanso#1515

Six layout rows, one job, against the values carried forward from main:

      compile           35,443,611 ->    35,442,256    -1,355   -0.0038%
      entry            126,354,834 ->   126,352,941    -1,893   -0.0015%
      library          126,810,299 ->   126,808,451    -1,848   -0.0015%
      interpreted    2,168,428,538 -> 2,168,266,027  -162,511   -0.0075%
      start-up           3,363,774 ->     3,363,824       +50   +0.0015%
      emitting          51,546,788 ->    51,541,777    -5,011   -0.0097%

Five fell and one rose, all under a hundredth of a per cent. LAYOUT: the two
divisions this branch merges are in float rendering and none of these routes
renders one -- three are `kanso check`, `emit_ir` stops before the backend, and
the interpreted corpus decodes a document it built itself. The compile, entry,
library and emit rows each read the same value twice in the job, so the
binaries are stable and the disagreement was with the golden.

Both codegen rows read their goldens exactly: 596,162,050 dev and 6,820,866,344
release, which is `-Wl,-plugin-opt=jobs=1` holding across another tree.

Welfare rises past the sentinel's band and the floor is banked at
76.83783682357429. The round before this one carried main's rows forward and
was red on exactly these six gates plus the floor sentinel, which is what a
re-merge round is for: the branch cannot know what the merged tree reads until
CI reads it, and a floor banked on the pre-merge rows would price a tree that
no longer exists.

## 2026-09-18 — kanso#1502 re-merged onto main after kanso#1516

kanso#1516 landed the interpreter's name memory underneath this branch and took
171,322,972 instructions off the interpreted row. All six layout rows and the
floor carry MAIN'S values again, for the reason they did after kanso#1515: this
branch's readings were taken against a tree that no longer exists.

This is the third re-merge this branch has taken, and each one costs it a round.
That is the price of `required_status_checks.strict` with several changes in
flight, not a defect in any of them: a merge makes every other open pull request
dirty, a dirty one gets no CI at all, and the rows have to be taken again
against the main that now exists.

The branch's own change has not moved through any of it. The ratchet passed on
the previous head before it went dirty.

## 2026-09-18 — kanso#1502, CI's rows on the tree merged after kanso#1516

Round two. The round before this carried main's values forward and was red on
five gates plus the floor; these are the merged tree measured on the runner.

THE FLOOR FAILS THREE JOBS, NOT ONE, and this round is where that got counted
properly. An unbanked rise turns `cost goldens` red at its welfare step, and it
also turns `specs (unit, golden, differential)` and `the other host (macos, arm)`
red, because both run `tests/the_digest_is_priced_on_both_sides.rs` and its
`the_undoctored_goldens_hold_the_floor` reads the same sentinel. Reading the
board as "the ratchet job plus the floor" undercounts it by two jobs, and the
two extra reds look like a second, unrelated fault until the target name is
read. Watched here: with the goldens stashed back to the values CI tested, the
spec panics `welfare 76.88   floor 76.87 ... a rise nobody ratchets`, and it
passes with the floor banked.

    compile_instructions      35,447,843 ->    35,443,478    -4,365   -0.0123%
    entry_instructions       126,368,664 ->   126,353,950   -14,714   -0.0116%
    library_instructions     126,824,214 ->   126,809,760   -14,454   -0.0114%
    startup_instructions       3,364,523 ->     3,363,366    -1,157   -0.0344%
    emit_instructions         51,554,663 ->    51,543,783   -10,880   -0.0211%

All five fell, all under four hundredths of a per cent, and the mechanism is
nameable rather than assumed. `src/runtime.c` is `include_str!`'d into the
compiler at `src/main.rs:1010` and digested into a constant at `src/hash.rs:176`,
so a runtime edit moves 450,100 bytes of the compiler's own `.rodata` and every
route that runs the compiler moves with it. None of these five routes executes
the runtime: three are `kanso check` and stop before codegen, `emit_ir` writes IR
and stops before the backend, and the fifth starts the interpreter.

Six rows read their goldens exactly, and the interpreted one is the one that
says something. `interp_instructions` holds 1,997,105,566, which is main's value
after kanso#1516, and CI read that integer back on the merged tree. The
interpreted corpus decodes a document it built itself, and it does not touch the
C runtime, so the branch's two divisions cost it nothing. `interp_allocs`
4,985,433, `interp_peak_bytes` 942,210 and `compile_allocs` 27,313 likewise.
Both codegen rows read exactly too — 596,162,050 dev and 6,820,866,344 release —
which is `-Wl,-plugin-opt=jobs=1` holding across a fourth tree.

The floor is banked at 76.88. The five falls are layout and none of them is
work, so what the ratchet holds here is the arithmetic rather than a gain the
branch earned.

This is the fourth re-merge this branch has taken. The cost is
`required_status_checks.strict` with several changes in flight: a merge turns
every other open pull request dirty, a dirty one gets no CI, and the rows have
to be read again against the main that now exists. The branch's own change has
not moved through any of it.

## 2026-09-18 — kanso#1502 re-merged onto main after kanso#1517

The fifth re-merge, and the round above it is already superseded. kanso#1517
landed the callee memory under this branch minutes after CI read the rows for
kanso#1516's tree, so the five layout rows carry MAIN's values again and this
round is deliberately red on them and on the floor.

kanso#1517 moved those five the other way and calls it WORK rather than layout:
the three `kanso check` routes each rose almost exactly 0.10%, because
`Interp::new` now wraps every function group in an `Rc`, which is an allocation
per group on a route that constructs an interpreter and evaluates nothing.
Nothing here disputes that; it is written down so the next reading of this
golden knows the row it is being compared against went up for a reason.

The floor is NOT banked in this round, on purpose. Welfare reads 76.89 against
main's 76.88 and the 0.01 is real — the branch's two divisions take 2,642,220
instructions off runbench, and the work vein that carries it read `success` on
the last job. But welfare also weighs the five carried rows, so a `--set` now
would freeze a score this container projected rather than the one CI measures.
The rows come first and the bank follows them.

## 2026-09-18 — kanso#1502, CI's rows on the tree merged after kanso#1517

    compile_instructions      35,486,173 ->    35,487,966    +1,793   +0.0051%
    entry_instructions       126,498,498 ->   126,503,235    +4,737   +0.0037%
    library_instructions     126,953,661 ->   126,959,068    +5,407   +0.0043%
    startup_instructions       3,362,788 ->     3,363,586      +798   +0.0237%
    emit_instructions         51,451,897 ->    51,457,106    +5,209   +0.0101%
    interp_instructions    1,963,826,350 -> 1,963,826,365       +15   +0.0000008%

The five layout rows rose this time where they fell against the kanso#1516
tree, which is the same term with the opposite sign: `src/runtime.c` is
`include_str!`'d into the compiler and digested into a constant, so 450,100
bytes of the binary's own `.rodata` move and every route that runs the compiler
moves with them. None of the five executes the runtime.

THE INTERPRETED ROW IS THE ONE WORTH READING, and it is worth reading because
of how small it is. Fifteen instructions on 1.96 billion. Against the kanso#1516
tree this row read its golden EXACTLY, and what changed underneath it since is
kanso#1517 rather than anything on this branch — the corpus decodes a document
it built itself and never enters the C runtime. So the fifteen is not the two
divisions. A frame-level move of this size has been recorded in this log
before, at plus or minus 13, and that one was chased to `memrchr`; whether this
is the same frame was NOT checked, and the row is written in saying only that
fifteen is far too small to be work and that nothing else about the row moved.
`interp_allocs` read 4,810,437 and `interp_peak_bytes` 951,438, both agreeing
with their golden, which is what a row with no work in it looks like.

`compile_allocs` read 27,313 and both codegen rows read exactly — 596,162,050
dev and 6,820,866,344 release.

## 2026-09-18 — kanso#1502 re-merged onto main after kanso#1518

The sixth re-merge, and the cheapest of them. kanso#1518 landed the frame memory
and four reserves, which moved the interpreted rows and nothing else: the five
layout goldens auto-merged because that branch did not move one of them, and
only `interp_instructions`, the floor and the log needed resolving.

That is worth noting beside the earlier rounds. A branch whose diff is large in
`src/eval.rs` cost this one three conflicts; the two branches before it, whose
diffs were small and in `src/runtime.c`, cost it eight. What a merge costs is
about which FILES moved, not how much.

The interpreted rows carry main's — 1,555,890,579, 3,879,653 and 961,165 — and
they are the ones to watch here for the same reason as last round: this branch's
two divisions moved that row by 15 instructions on 1.96 billion, so anything
larger than tens in the next sitting is kanso#1518's arithmetic showing through,
not Ryu's.

## 2026-09-18 — kanso#1502, CI's row: eight instructions, and the fourth reading under thirty

    interp_instructions   1,555,890,579 -> 1,555,890,587   +8   +0.0000005%

Every other row read its golden exactly, including both interpreted memory rows.

FOUR RUNTIME EDITS IN ONE DAY HAVE NOW MOVED THIS ROW BY 15, 26, 19 AND 8. Four
different functions in `src/runtime.c`, two sittings against the kanso#1517 tree
and two against the kanso#1518 one, and every reading under thirty on a row of
one and a half billion.

`interp_allocs` read **3,879,653 on all four**. That is what turns four small
numbers into a finding rather than four shrugs: an allocation counter counts
operations rather than a host, so work in the interpreted run would have moved
it, and it did not move once. The size of any single reading proves nothing —
the agreement of the counter beside it across four independent edits is the
evidence.

## 2026-09-18 — the layout rows the kanso#1520 merge left on this branch

kanso#1520 landed under this branch — the clone sized for the growth that
follows it — and every instruction row moved with the binary it rebuilt. The
branch touches `render_ryu` and nothing the front end runs, so none of the six
is work anybody did on this branch; all six are where the code landed after
another change resized the compiler around it. Written down because a number
that changes without a sentence is the thing to catch.

CI's readings on the merged tree, against the values carried forward from main:

    compile_instructions      35,486,333 ->     35,488,929     +2,596
    entry_instructions       126,498,292 ->    126,507,730     +9,438
    library_instructions     126,954,304 ->    126,961,994     +7,690
    interp_instructions    1,260,262,910 ->  1,260,262,917         +7
    startup_instructions       3,363,378 ->      3,363,252       -126
    emit_instructions         51,456,464 ->     51,455,635       -829

The interp row's +7 is the same order as the ±13 the module row has drawn
across trees whose compiler source was identical; kanso#1487 measured that one
and it is a face of the layout rather than a cost. The compile-side three are
larger and one-directional, which is what an inlining decision re-made against
a different `src/eval.rs` looks like. Both compile memory rows and both codegen
rows agreed without an edit, which is the check on that reading: allocation and
peak counts are decisions the code makes, and they did not move.

The floor is banked at 77.13 after these rows, not before them.

## 2026-09-18 — kanso#1502 on the merged tree, and the seven that two branches agree on

`interp_instructions` re-bases from 1,138,001,430 to **1,138,001,437**, a rise
of seven on a branch that changes two divisions in Ryu's float rendering and
goes nowhere near the interpreted corpus.

Every other row in the cost-goldens job read its golden exactly: both codegen
rows, all five layout rows, `compile_allocs`, and both interpreted memory rows.

What makes the number worth writing down is that it was read twice, on the same
day, by two branches with nothing in common but their base. kanso#1504 caches
the innermost beat mark; this one divides differently in a float renderer;
neither touches what the interpreted corpus runs. They landed on different
runners — this branch's was Intel, family 0x6 model 0x6a, which
`bench/dispatch.txt` does not record — and both read 1,138,001,437 to the
instruction. `interp_allocs` read 2,539,998 on both and on main, which is the
check that keeps this out of the "real work" column: allocations are decisions
the program makes, and they did not move.

**What is reproduced, and what is guessed.** The golden's 1,138,001,430 came
from kanso#1522's own PR tree. 1,138,001,437 is what a tree carrying that
change *merged* reads. That the two differ by seven is established by two
independent readings. Why they differ is not.

An earlier note on kanso#1504 claimed a pattern — the same seven across three
trees at three different absolute values — and that note is withdrawn. One of
its three readings was the gate's second count, which does not subtract the
printed line and therefore reported a stable binary as unstable by exactly
3,015; kanso#1524 fixes it. Two readings of seven are still two readings, and
they are enough to re-base a row. They are not enough to name a mechanism, so
none is named here beyond the standing guess that it is layout.

## 2026-09-18 — the eleven, drawn twice in one job, with the golden on the second

kanso#1502's re-run cleared every vein it had been failing. The interpreted row
read 1,138,001,437 and agreed with the re-base; the three compile rows,
start-up, emit, both memory rows and the dev codegen tier all read their
goldens exactly. One row disagreed, and it is the one already under a blocked
question:

    codegen_instructions_release  first  6,820,866,355
                                  again  6,820,866,344
                                  golden 6,820,866,344

Eleven apart, in one job, on one binary. Both readings saw five processes —
`first_procs=5 again_procs=5` — so the gate's own guard against a second
reading that measured something different is satisfied.

This is the intermittent kanso#1512 hunted to its source: the temp object's
NAME. Nine of ten names read one value and `4b8c1a` reads eleven more,
reproduced three times that day. What is new here is the ORDER. Every earlier
sighting had the expensive name and the golden on opposite sides of a whole
job; this job drew the expensive name on its first pass and the cheap one on
its second, so the same binary showed both faces within one run and the second
landed on the golden exactly.

That is worth recording because it is the cleanest demonstration yet that the
eleven is a property of the build rather than of the machine or the day: one
process tree, one sitting, two answers, and the difference is which name the
temp object got.

`codegen_release_kanso_excluded` moved too — 80,447,724 against 80,449,362,
1,638 apart — which is kanso's own process and is excluded from this row by the
2026-09-15 normalization ruling. It is printed rather than counted, and it
moving while the counted rows hold is the exclusion doing its job.

The fix is kanso#1513's and is a decision, not a measurement: it sits in
design/pending-gavels.md awaiting Clay.

## 2026-09-18 — kanso#1502 on the tree merged with kanso#1525: six rows, and they do not agree on a direction

CI's sitting on the merged tree, against the rows main carried:

    compile_instructions     35,552,188 ->    35,553,022      +834
    entry_instructions      126,735,634 ->   126,739,095    +3,461
    library_instructions    127,192,177 ->   127,194,352    +2,175
    interp_instructions  1,075,174,600 -> 1,075,174,614       +14
    startup_instructions      3,365,595 ->     3,364,440    -1,155
    emit_instructions        51,630,538 ->    51,629,594      -944

**Three up and two down, and that is worth saying rather than smoothing over.**
kanso#1504 took the same merge an hour earlier and its five layout rows all
fell together; the entry there called a uniform shift what a layout move looks
like. This one is not uniform. Both branches are layout stories — neither
changes what the front end decides — but "they all moved the same way" was a
property of that reading and not of this one, and a reader who took it as the
signature of layout would mis-read this table.

What layout actually predicts is that the rows move a little and
inconsistently, because each one is a different route through a binary whose
code landed in different places. Uniformity is one thing that can happen, not
the test.

**The interpreted row rose by fourteen, and the allocation counter did not
move.** 2,486,376 and 833,130 are main's values to the unit. That matters more
here than on kanso#1504, because this branch changes Ryu's float rendering and
the interpreted corpus does render floats — so a real-work reading was
available and the allocation row is what rules it out. Fourteen on 1.075
billion is the same order as the sevens this row has been drawing all day, and
those have now been shown not to be the silicon, not `.text` size and not
`.rodata` size.

Both codegen rows and `compile_allocs` read their goldens exactly.

## 2026-09-18 — what the ryu branch does to the interpreted row after kanso#1531

The interpreted row on this branch is `interp_instructions=1,029,696,289`,
where main reads 1,029,696,275. A rise of 14 instructions, 0.0000014%.

It is layout and nothing else. The row runs `kanso run --interp`, which never
reaches the native runtime; what it carries of `src/runtime.c` is the bytes,
because the compiler `include_str!`s that file and the interpreted run is a
process whose binary holds it. This branch takes four hundred bytes out of
`ryu_d2d` and adds none, and every benchmark's `.text` comes down 1,360 bytes
with it, so the code the interpreted run walks past is arranged differently.

Fourteen instructions is what that arrangement is worth on this corpus. The
branch's own earlier sittings recorded the same effect against the pre-`if`
baseline, 2,182,576,109 -> 2,182,584,048, a rise of 7,939 on a row four times
the size. Same shape, same direction.

The trend gate asked for this sentence and was right to: the row was re-based
in the goldens with nothing in the log naming it or the value it landed on.

## 2026-09-18 — kanso#1502's rows on the tree merged after kanso#1533

CI measured the ryu branch on the tree carrying the dispatcher change. Five
compile-side rows rose and the interpreted row did not move at all:

    compile_instructions   35,550,010 ->  35,554,080    +4,070  +0.0114%
    entry_instructions    126,729,588 -> 126,742,198   +12,610  +0.0099%
    library_instructions  127,186,008 -> 127,198,066   +12,058  +0.0095%
    emit_instructions      51,617,476 ->  51,645,272   +27,796  +0.0538%
    startup_instructions    3,363,916 ->   3,366,495    +2,579  +0.0767%
    interp_instructions   995,837,536 -> 995,837,536         0   exactly

Four of the five were counted TWICE in the one job and the repeat agreed to
the instruction every time: compile_again 35,554,080, entry_again
126,742,198, library_again 127,198,066, emit_again 51,645,272. So the rises
are the binary, not the reading.

They are layout. `kanso check` stops before codegen and cannot see a change
to how a double is rendered, and this diff grows every binary's `.text` by
32 bytes. The branch's own rows -- what the digit loop costs at run time --
are the ones in `bench/instructions_golden.txt`, and they fell: encodebench
3,497,149,260 -> 3,485,406,060, livebench 2,825,430,323 -> 2,813,687,123,
runbench 1,821,933,936 -> 1,819,291,716, oneshot 17,888,155 -> 17,858,797.

**THE INTERPRETED ROW READ ZERO THIS TIME, AND IT READ +14 BEFORE.** The
branch's earlier note against main at kanso#1531 recorded a rise of 14
instructions and called it the same layout move the compile rows show. On
this tree the row is byte-identical. Both readings are CI's and neither is
wrong; what they show is that 14 instructions on a 995 million row is inside
what a relink moves it by, so the earlier sentence attributed to a mechanism
something that is better described as the row not moving. The note stays in
the header with its number, because it is what was measured; this paragraph
is what it means.

---

## 2026-09-19 — the eleven drawn twice in one job again, and the wrong row restored

CI's sitting on kanso#1502's merged tree read `codegen_instructions_release`
6,820,866,355 first and 6,820,866,344 second, in the same job, on the same
binary. Eleven apart.

The cause is settled and was settled before tonight: kanso#1513's
twenty-two-name experiment held every input and varied only the temporary
object's name, and twenty of twenty-two names read the low bucket while
`4b8c1a` and `fedcba` read the high one. About one name in eleven. This job
drew the rare one on its first reading.

**WHY THE LOW VALUE IS THE ROW.** `codegen_instructions.sh` compares the FIRST
reading and exits green before it takes a second — the reproduction check runs
only on a row that already disagrees. So the pinned value wants to be the
common bucket, and the row is green about ten jobs in eleven.

**AND IT WAS THE ROW, until the duplicate beside it was deleted.** This file
carried `codegen_instructions_release=` twice, 6,824,133,280 from main and
6,820,866,344 from this branch. Deleting the duplicate left a choice of which
to keep and main's was carried forward, on the ground that neither was a
reading of THIS tree. kanso#1504's entry records what that reasoning missed:
when one of the two came from CI on a tree this one descends from, that is the
better carry-forward. Both branches made the same wrong choice for the same
reason, and both cost a round. Restored here.

**WHAT IS STILL OPEN IS A SCOPE, NOT A MECHANISM.** Pinning the temporary's
name closes this for good, and which tier the pin covers is a decision in
design/pending-gavels.md. Until it is ruled this row is the one flaky gate in
the tree — and its flakiness has a named mechanism, a measured rate and a known
fix, which is a different thing from a gate nobody understands.

---

## 2026-09-19 — kanso#1502's rows after the pooling landed, and the bucket it drew

    interp_instructions   923,151,727 ->  923,151,719      -8
    compile_instructions   35,551,167 ->   35,549,049  -2,118
    emit_instructions      51,619,793 ->   51,616,373  -3,420
    entry_instructions    126,732,646 ->  126,727,524  -5,122
    library_instructions  127,188,882 ->  127,183,938  -4,944
    startup_instructions    3,363,672 ->    3,363,249    -423

Each of the four that take a second reading agreed with itself.

**AND `codegen_instructions_release` PASSED.** It read 6,820,866,344 — the low
bucket, which is the value this branch's golden was restored to a few hours ago
on the argument that the gate compares the FIRST reading and the low bucket is
the common one, twenty of twenty-two names. The job before this one drew the
high bucket on its first reading and went red. Two jobs, two buckets, one
binary, and the pinned value was right about which to expect.

That is the eleven behaving exactly as kanso#1513's experiment says it will,
and it is the second time today the same branch has shown both faces. Nothing
new is claimed from it: the mechanism was settled and the rate was measured,
and this is the rate happening.

**THE INTERPRETED ROW MOVES EIGHT**, where kanso#1504's moved one the other
way. Both branches touch rendering and both now sit on the pooled dispatch
loop, so the small residues each carries are against a floor that moved under
them. Neither is a constant: kanso#1504's entry of today records a residue that
held five times and broke on the sixth, and this row gets no projection from
here for the same reason.

## 2026-09-19 — kanso#1502's two layout rows, moved by the merge rather than by the branch

The cost-goldens job on the merged tree halted two rows and both are layout:

    library_instructions   127,183,938 -> 127,184,941   +1,003
    startup_instructions     3,363,249 ->   3,363,186      -63

Neither is this branch's. kanso#1513 added the fixed-temp pin to `src/main.rs`,
so the compiler's own bytes moved, and these two rows carry the binary's layout
rather than counting the digit loop. The job read each number twice and got the
same answer — "this binary is stable" — so the disagreement was with the golden
and not within the run.

Regenerated from CI's readings, which is the only place they can come from: this
container's toolchain is not the runner's. Welfare is unmoved at 77.2774,
`library_instructions` being no term of the objective and the start-up move
being nineteen parts per million of a row weighted 0.25 on the development side.

## 2026-09-19 — kanso#1502 on CI's rows, and five paragraphs a resolution kept twice

The branch was red on six veins and on the ratchet. The six are the ordinary
thing: every compile-side golden still carried main's value while the branch
changes `src/runtime.c`. CI's sitting, run 35419502516:

    compile_instructions          35,549,049 ->     35,549,673       +624   +0.0018%
    entry_instructions           126,727,524 ->    126,728,843     +1,319   +0.0010%
    interp_instructions          923,151,719 ->    923,151,727         +8
    emit_instructions             51,616,373 ->     51,618,058     +1,685   +0.0033%
    codegen_instructions_dev     596,153,756 ->    596,158,173     +4,417   +0.0007%
    codegen_instructions_release  6,833,786,335 -> 6,825,827,822 -7,958,513  -0.1165%

The first five are layout, in both directions, as a shifted binary reads. The
sixth is work, and it goes down: the release tier is the only row that compiles
src/runtime.c rather than carrying its bytes, and this branch removes two
divisions from the float renderer. Every codegen and compile reading reproduced
in the same job, which is kanso#1507's plugin pin and kanso#1513's fixed-temp
name still holding on a tree that changes runtime.c.

Scored on those rows the tree reads 77.2796 and the floor is banked there.

THE RATCHET FAILURE WAS SOMETHING ELSE, and it is the part worth keeping. The
job reported `golden_prose` ALREADY RED before any mutation, on one drifted
number quoted four times:

    drifted: compiler.html :: compile.library_instructions shows 127,183,938,
             golden says 127,184,941

Four, because an earlier conflict resolution on this branch had kept both sides
of five long paragraphs. `docs/compiler.html` carried each of them twice, with
a blank line between the copies, and the page still rendered: main has two
`compile.library_instructions` spans and the branch had four. The resolution
check this repo uses looks for conflict markers, unmerged paths, lost sections,
duplicate section numbers and ascending numbers, and every one of those passed
over a page with ten duplicated lines in it. The duplicates are removed here and
the check now also fails a paragraph that appears twice.

That is the standing shape from 2026-09-18 again: a verification that names one
file keeps passing while the defect moves next door. It took a gate that reads
the page's numbers to find it, and it found it by counting a drift four times
instead of twice.
## 2026-09-22 — compile_instructions read three apart on a tree that cannot reach the compiler

kanso#1556 changes `design/compiler-log.md` and `docs/compiler.html` and nothing
else. Only `lib/*.kso` is `include_str!`'d into the compiler, so its compiler
sources are main's to the byte. CI disagreed anyway:

    main        af78401   35,551,167   green
    kanso#1556  f0167bd   35,551,170   red, +3

The gate takes a second count in the same job whenever the row parts, and it
read 35,551,170 again — VERDICT (1), the binary is stable, so the disagreement
is with the golden rather than within the run.

The two jobs differ in two things at once, which is the confound the gate's own
header has been complaining about since 2026-09-16:

    binary sha256   50656a4ebe6f   against   6d7c7e355188
    silicon         family 0x19 model 0x11   family 0x1a model 0x2

WHAT IT RULES OUT. The two builds have BYTE-IDENTICAL section sizes — .text
2,836,114, .rodata 803,768, .data 12,672, .bss 29,912 on both — so the
seven-binary ladder in the gate's header, which perturbs those sizes, does not
describe this pair. And every one of the fifteen functions in the threshold-90
listing agrees to the instruction across the two jobs, `__memcmp_avx2_movbe`
at 1,129,005 and `__memcpy_avx_unaligned_erms` at 792,855 on each. So the
feature block that `dispatch.sh differs` reported did NOT make glibc resolve a
different string routine, which was the leading candidate. The three
instructions are below that threshold, in the tail the listing does not print.

The inclusive listing puts them inside `main`: PROGRAM TOTALS, the ld.so frame,
both `(below main)` frames, `__libc_start_main` and `main` are each exactly 3
apart, and `compile_printed` is 812 on both.

WHAT WOULD FINISH IT is the frame-level diff of the two profiles, which the gate
prints itself on VERDICT (2) and not on VERDICT (1). Both jobs uploaded theirs
as artifacts; neither can be fetched from a container, because the blob host
answers `CONNECT tunnel failed, response 403` — already recorded in the gate's
header, and the reason the sha and the sections are emitted as notices.

ORDINARY CARGO NON-DETERMINISM IS OUT, measured rather than assumed. There is
no `build.rs` in this crate and no `env!` or `option_env!` in `src/`, so nothing
embeds a commit or a timestamp. On one container, `touch src/lib.rs src/main.rs`
followed by `cargo build --release` recompiled the crate and produced a
BYTE-IDENTICAL binary, sha 5bdfd6b4029b both times. So two builds of identical
sources under one toolchain agree, and CI's two shas are not that.

What the two shas can still be: the four sections the gate prints are the
LOADED ones, and a difference in `.comment`, the build id, or the unwinding
tables moves a sha without moving an executed instruction. That half is not
settled here.

WHAT THE LOG WOULD HAVE TO CARRY for the next occurrence to be localizable. The
gate prints the exclusive listing at `--threshold=90 | head -40`, and on this
corpus that threshold has 125 function rows, so 85 of the rows it already
computed are thrown away before the log sees them. The whole table is 1,115
rows; 99 is 282 and 99.9 is 494. A three-instruction move can sit in a function
too small to make any threshold, so only the whole table guarantees catching it.

And the artifact is not an alternative from here, for a reason worth naming
precisely: `productionresultssa7.blob.core.windows.net:443` is refused by this
session's EGRESS POLICY — the proxy's own status endpoint records
`connect_rejected, gateway answered 403 to CONNECT`. That is an organization
policy denial rather than anything GitHub did, so no credential and no retry
reaches it, and the job log is the only channel a session of this kind has.

The silicon stays the live candidate, narrowed. glibc resolves its string
routines by ifunc at start-up, the two the listing prints resolved the same, and
the ones it does not print — strlen, memset, memchr and the rest — are where an
AVX512 machine and one without it would part. Three instructions is the size of
one such difference, not of a compiler change.

This box cannot arbitrate: `host_gate.sh` refuses it at glibc 2.39-0ubuntu8.7
and rustc 1.94.1 against the golden's 8.9 and 1.98.1, so a reading taken here
reproduces nothing.

THE RE-RUN SETTLES IT, AND THE PREDICTION GOES DOWN FIRST. The failed job was
re-run once. Its `compile instructions` step came back GREEN, so a third job on
this same tree read 35,551,167 — the branch did not move the row, which was
never in much doubt and is now measured. What the re-run's `compile_sample` line
says next is decisive, and there are only two answers:

    sha 6d7c7e355188 again, row 35,551,167
        one binary counted two numbers on two machines. That is a REPRODUCTION
        FAILURE by this vein's own definition, the silicon is the variable, and
        the vein halts.

    sha 50656a4ebe6f, row 35,551,167
        the row tracks the BINARY and not the machine, the two shas are the
        whole story, and what needs explaining is why one tree built twice
        produced two binaries when this container builds it twice and gets one.

Written before the reading, because the confound has stood for six days and a
reading interpreted afterwards can be made to fit either.

THE READING, AND NEITHER BRANCH OF THE PREDICTION HAPPENED. The re-run built a
THIRD binary and drew a THIRD machine:

    job                 sha            silicon                  row
    main af78401        50656a4ebe6f   family 0x19 model 0x11   35,551,167
    kanso#1556 att. 1   6d7c7e355188   family 0x1a model 0x2    35,551,170
    kanso#1556 att. 2   543f040c05d4   family 0x19 model 0x1    35,551,167

The prediction assumed the re-run would reproduce one of the two shas. It
reproduced neither, and that is the first result: THREE CI JOBS ON ONE TREE
BUILT THREE DIFFERENT BINARIES, where this container rebuilding the same tree
twice produced one. Whatever makes cargo's output vary is in the runner and not
in the sources.

The second result is the one the confound was in the way of. Attempts on
family 0x19 model 0x11 and family 0x19 model 0x1 carry DIFFERENT binaries and
agree on the row TO THE INSTRUCTION. So on that pair the sha moves the row by
nothing, and the 35,551,170 belongs to the job that drew family 0x1a. The
variables are separated in the direction the gate's header could not separate
them from outside: the binary is inert across the pair that shares a family,
and the outlier is the one that does not.

A FOURTH READING KILLS THE SILICON. The entry above concluded, from three
jobs, that the binary moves this row by nothing and that family 0x1a was the
outlier. kanso#1559 — another tree that cannot reach the compiler, a log entry
and a page paragraph — read 35,551,170 on **family 0x19 model 0x11**, which is
the model main read 35,551,167 on the same morning.

    job                sha            silicon                  row
    main af78401       50656a4ebe6f   family 0x19 model 0x11   35,551,167
    kanso#1556 att.1   6d7c7e355188   family 0x1a model 0x2    35,551,170
    kanso#1556 att.2   543f040c05d4   family 0x19 model 0x1    35,551,167
    kanso#1559 att.1   c4649028a8d2   family 0x19 model 0x11   35,551,170
    kanso#1559 att.2   93fab21748ce   family 0x19 model 0x1    35,551,167

One model, both values. So the machine does not decide it, and the paragraph
below that reasoned from the 0x19 pair agreeing is withdrawn: two jobs agreeing
on a value was a coincidence of which binary they built, not a property of the
silicon they ran on.

What is left standing is the BINARY, and FIVE JOBS HAVE BUILT FIVE DISTINCT
ONES from sources that cannot differ — 50656a4e, 6d7c7e35, 543f040c, c4649028,
93fab217. Every one of the five carries the same four section sizes. So the
thing that moves is inside a section whose length did not change, or in bytes no
section size counts — and the whole-table print this commit adds is what would
name the function, which is the reason it exists.

THAT CI NEVER REPRODUCES A BINARY IS NOW THE QUESTION, because this container
does: a forced recompile here gave the same sha twice. Whatever varies lives in
the runner rather than in the sources, and until it is named this row cannot be
pinned to a build.

AND ALL THREE BINARIES PLACE THE SAME BYTES. The sections the gate prints are
identical across the three, to the byte: `.rodata` 803,768, `.text` 2,836,114,
`.data` 12,672, `.bss` 29,912. Three shas, one layout. So the sha differences
live where a section size cannot see them — the build id, `.comment`, the
unwinding and debug tables, or bytes rearranged inside a section that did not
change length. Nothing of that kind is executed by `kanso check`.

One triple is not a law. What it licenses is the next experiment rather than a
conclusion — the row read against family 0x1a again, on a binary that has
already read 35,551,167 somewhere else — and the whole-table print below is what
would say which function carries the three when it happens.

BUILT, in this commit. The gate now annotates the whole profile on EVERY run —
`callgrind_annotate --threshold=100`, uncapped, inside a collapsed group — so
two jobs can be diffed to the instruction from their logs alone. Verified
against a real profile: 1,115 rows, 86 KB, reaching functions that retire one
instruction. `tests/the_compile_gate_prints_the_whole_table.rs` pins the three
properties that matter, and each was watched red for its own reason: the table
is asked for at all, its pipeline does not truncate, and it is printed BEFORE
the row is compared, since a job whose row agreed is the side a comparison is
always missing and the failure path never emits one.

The row is worth +0.0060 a tenth in the 2026-09-19 marginal table, the least
valuable of the fourteen the objective reads, and on this pair it is the only
one of twenty-seven veins that parted.


## 2026-09-22 — a finished coordinated branch is a landmine for the next branch that shares its name

kanso#1558 changes a log entry and one gate's output, and it went red on
`kq specs` with two dozen `error[exhaustive]` and `error[effect]` diagnostics
out of kq's source. The cause is in the job log's second line:

    kq: claude/go-to-town-m0dicm (performance goldens from main)

`.github/clone-sibling.sh` prefers a sibling branch NAMED AFTER the branch under
test, which is how a language change and the sweep it forces in kq, vse and
kanso-json get checked together. kq had a branch of that name. It was cut to
check kq against kanso#1369, which merged on 2026-09-14; it was twelve commits
ahead of kq main and twenty-five behind, with nothing open on it, and its source
predates the err-reader and effect rulings the compiler now enforces.

So a coordinated branch nobody closed answers for every later kanso branch that
draws the same name, and branch names here are assigned rather than chosen.

FIXED by taking kq main's tree onto that branch in a merge commit — history
intact, every commit still reachable, no rewrite — after checking that kq's own
suite passes against a build of kanso main with that tree: eleven fixture
goldens against `jq -S`, the three cost goldens, the scale gate and the
published-numbers stamp. Merging kq main into it properly was the other option
and it is disproportionate: eleven conflicted files, among them the pin, the
README's numbers and four goldens, all to revive a branch whose work is done.

The script's own header has the general form of this already, about the
`sibling-goldens-move` licence: "A file left behind now names a branch nobody is
on, so it grants nothing and nobody has to remember to delete it." The branches
themselves have no such property, and this is the second mechanism in that file
to be bitten by a leftover.

## 2026-09-22 — the carry-width rule's decline does not reproduce

kanso#1556 declined the carry-width rule three days ago on one observation:
under it `kanso run scripts/ratchet` died, and printing the request at the
point of failure gave `CARRYOOM need=18446744072171062384 depth=3 carry_n=1`.
That entry and `docs/compiler.html` §115 both present it as what killed the
rule. Rebuilt and re-run today, it does not happen.

THE RECONSTRUCTION IS EXACT, which is the first thing to establish, because a
negative result is worth nothing if the change was not really there. `src/beat.rs`
is the ONLY compiler source differing between `dc368fb7` — the commit that built
the rule — and its merge-base with main, and main's `src/` and `lib/` are
byte-identical to that merge-base, since the two commits that have landed since
touch CLAUDE.md, the log and the page. So main plus that one file IS the branch's
compiler.

THE POSITIVE CONTROL SAYS THE RULE IS LIVE. On `bench/runbench`, counters on:

    main's compiler    arena_blocks 36   arena_peak_bytes 38,604,496
    with the rule      arena_blocks 33   arena_peak_bytes 35,458,768

Those are kanso#1556's own headline figures, to the byte.

WHAT THE RULE DOES TO THE RATCHET, with that control passing:

    kanso run scripts/ratchet          exit 0, three times, nothing on stderr
    dev-tier binary                    exit 0, three times
    release-tier binary (--release)    exit 0, three times
    an AddressSanitizer build          clean, no report
    peak RSS against main's            28,220 KB against 28,176 KB

The WIP state of the rule, `6cc0e349`, was tried as well, on the chance the
failure belonged to the earlier shape: same arena peak, same clean exit.

NOT MEMORY PRESSURE EITHER. kanso#1556 read the non-monotone coalitions as "a
failure sitting near a memory limit", and the natural reading of a
non-reproduction is that this container has more room today. The peak RSS says
otherwise: the rule's ratchet peaks slightly BELOW main's, and the whole program
is 28 MB.

ONE THING ABOUT THE BOX, said because this project treats external state as
part of a measurement. Installing the sanitizer runtime earlier in the same
session moved this container's glibc from 2.39-0ubuntu8.7 to 8.9, and every
reading above was taken after that. Both arms share it, so the comparison is
unaffected, and the arena peak is the evidence rather than the argument:
35,458,768 here is kanso#1556's figure to the byte, measured on another box on
another day.

WHAT THIS LICENSES, AND WHAT IT DOES NOT. It does not re-land the rule.
kanso#1556 carried a second and independent objection — the rule admits
`json/array_open/3` and `json/obj_open/3`, so `beat::tests::
json_decode_loops_stay_conservative` had to be updated, and that entry says
itself that changing a test because the rule beneath it changed is ordinary
while changing one to get green is not. That question is untouched by anything
here. What this does is remove the reason the decline was actually written on,
and put a measured +0.41 back in play as something to decide rather than
something already settled.

The surfaces the CARRYOOM claim reached are this log and §115 of the page, and
both are corrected in this commit rather than only here.


## 2026-09-22 — every program in the corpus under a sanitizer, and the one report it expects

`src/runtime.c` reclaims memory in bulk, and the walks that decide what survives
follow pointers into storage the reclaim can take back. The file's own comments
carry two accounts of that going wrong — a `KStr` read in a munmap'd page, and a
node below a mark holding a tenured pointer that `k_repaired_settle` still does
not move. Both were found by a program crashing. Nothing in this tree had ever
run a sanitizer over the corpus.

It has now, and the result is a clean one:

    81  CLEAN   67 mem-corpus programs, 13 benchmarks, scripts/ratchet
     2  KNOWN   runbench and scanbench, the same guarded tail read
     1  SKIP    bench/numeric, which does not compile — see below
     0  FOUND

THE INSTRUMENT WAS WATCHED FAILING FIRST, which is the only reason the zero is
worth anything. `scripts/asan/sweep.sh --prove` patches a read of one byte from
a block `k_ten_release` has just freed into a COPY of the runtime, builds the
one fixture whose `.mem` golden pins `ten_frees=1`, and asserts the report
names that function. It does:

    #0 ... in k_ten_release .../runtime_broken.c:1547:33 heap-use-after-free

THE ONE KNOWN REPORT is `k_b_find2_raw`, and reading it wrongly is easy.
`k_tail_window` permits the final sixteen-byte load only when the pointer sits
at least sixteen bytes below a page boundary, so the load cannot reach an
unmapped page and the surplus is masked off afterwards. AddressSanitizer
reports it anyway, and the address it prints is the LAST byte of the load
rather than the first — so the report reads as though the access began ten
bytes past the end when it began inside the buffer. It is classified KNOWN and
printed rather than filtered, because a filter on that frame would hide every
later finding in it.

WHAT THE SWEEP TURNED UP BY ACCIDENT: `bench/numeric` has not compiled since
kanso#505 migrated the tree. `main.kso` calls `sq_sum` with no import, and
`lib.kso` beside it exports one; `kanso check bench/numeric` exits 2. Nothing
in scripts/, .github/, tests/ or bench/ names the directory, which is why a
benchmark being broken for that long went unnoticed. The import is restored in
this commit and it runs: `sum 2666668666667000000`, which is
n(n+1)(2n+1)/6 at n = 2,000,000.

AND IT IS THE ONLY ONE. Every directory under `scripts/`, `lib/` and `bench/`
holding `.kso` files was put through `kanso check`. Ten refuse, and nine of the
ten are the checker being asked the wrong question rather than anything broken:
eight are standard-library modules, which reach `builtin_*` names that only
resolve inside std, and the two `bench/workahead` programs carry a statement
beside their declarations, which is `kanso play`'s shape and not `check`'s —
run properly they both exit 0 and agree on `report: 449999997`. `bench/numeric`
is the only one that fails on its own merits.

It is not wired into CI. Every runner would need `libclang-rt-*-dev`, and the
known report would have to become a suppression maintained in two places. The
sweep is for the question the runtime's comments keep raising, asked by hand.

## 2026-09-22 — the tenure fixture does not catch the change it says it catches

`tests/golden/mem/a_repaired_node_below_the_mark_holds_tenure.kso` ends its
header with a promise:

> The fixture pins `ten_blocks=1` beside `ten_frees=1` so that a change which
> stops handing them up is a red test rather than a segfault in `k_copy_size`.

It does not. Stopping the hand-up leaves every counter in its `.mem` vein
byte-identical, so the test stays green.

THE EXPERIMENT. `k_ten_hand_up` was replaced, in a COPY of the runtime, by a
call to `k_ten_release` — the change the promise is about, which frees a
depth's tenure blocks at the inner pop instead of passing them to the depth
outside. Then:

    the path is exercised    traced: HANDUP d=1 with a real block, once
    every counter            identical, all forty-odd rows of the vein
    under a sanitizer        clean, no report

The counters cannot see it because `ten_frees` counts the block being freed
either way. Handing up moves WHEN and at WHICH DEPTH the free happens, and the
vein records neither.

WHAT THE FIXTURE DOES ESTABLISH is narrower than its header claims. It builds
the configuration the comment is about — a node below the mark holding a
pointer into tenure — and it does not read through that pointer after the pop.
So it reaches the construction and never the danger, which is why removing the
hand-up changes nothing observable in it.

WHAT WOULD CLOSE IT, neither done here. A counter that separates a hand-up from
a release would make the promise true, and that is a counters change: every
`.mem` file, twelve cost goldens, the emitted vein, the ch10 sample and the
siblings, all in one pull request. Or the header's last paragraph goes, and the
fixture keeps the claim it can support.

A caveat on the sanitizer half: the binary was built `-O0` against a
plain-malloc runtime, where the shipped one is `-O3 -flto`. That bounds the
sanitizer result to this build. The counter result is not so bounded — those
rows are deterministic, which is the whole reason they are pinned.

## 2026-09-22 — ten_handups: where a tenure block dies, which no counter could say

kanso#1560 found that `a_repaired_node_below_the_mark_holds_tenure` does not
catch the change its header promises to catch, and named the fix without doing
it: a counter separating a hand-up from a release. This is that counter, and
the two claims the fixture was carrying are corrected with it.

WHY THE PAIR IS BLIND. `k_ten_hand_up` moves a depth's tenure blocks to the
depth outside; `k_ten_release` frees them where they are. Either way the block
is claimed once and given back once, so `ten_blocks` and `ten_frees` are
identical across the two. The hand-up moves only WHICH DEPTH does the freeing,
and until now nothing recorded that. Replacing the call left all sixty-seven
`.mem` goldens byte-identical.

`ten_handups` counts a hand-up that moves a real block, after the early return.
With it, three fixtures go red on the swap:

    a_carried_value_written_into_an_older_node        1 -> 0
    a_repaired_node_below_the_mark_holds_tenure       1 -> 0
    an_inner_beat_opens_its_tenure_in_the_block_outside  4 -> 0

The third also moves `ten_blocks` 5 -> 3, which is the whole of what any
counter could see before. `bench/cost_golden_run.txt` reads 3, so the row is
live on the benchmark corpus rather than only in the lazy tier.

`scripts/ratchet/mutations/a_tenure_block_freed_where_it_was_handed_up.sh` is
the swap, and its row gates the mem vein. Watched red before it was written
down.

WHAT MAKES THE REPAIRED-NODE CASE SAFE IS NOT THE HAND-UP. runtime.c said it
was, in the paragraph above `k_ten_hand_up`, and the fixture's header said it
too. Poisoning the depth's live tenure bytes -- 39,200 of them -- at three
points one step apart in `k_beat_pop_slow` says which step matters:

    before the carry's deep copy     dies, reading a length out of 0xAB
    after the copy, before migrates  correct output
    at the hand-up                   correct output

The copy reads the tenured bytes out. After it nothing below the mark points
into tenure, which is why the swap leaves that program clean under
AddressSanitizer and why its output is byte-identical with the handed-up block
poisoned. `c->used_flag` is 1 on that pop, so the copy is the step that runs.

That is an isolation and not an attribution: the three poisons differ only in
position, and the answer flips across one of the two gaps.

The hand-up is load-bearing, on the other fixture. An inner beat that opens its
tenure in the outer depth's block is the shape whose blocks must travel, and
that is where the runtime's comment now points.

VEINS. A minted counter is additive and moves the lot: twelve cost goldens and
all sixty-seven `.mem` files, regenerated with `all_counters.sh --write`. The
compile veins move too -- `src/runtime.c` is `include_str!`'d into the compiler,
so its bytes are the compiler's -- and eight of them refuse comparison on this
box, so CI takes those rows and the floor is banked after they land. Neither
`bench/emitted_golden.txt` nor the book samples carry the tenure counters, so
neither moves. `ten_handups` joins the trend gate's `higher` table in the change
that mints it, for the reason `lower_a` gives about presence counters: dropping
the kernel should read as a worsening and want its sentence.
