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
---

## 2026-09-17 — the beat rewind's fast path: 23 instructions to 15

`k_beat_iter` is what a compiler-proven beat loop calls between iterations to
give the arena back. runbench calls it 2,692,766 times and it was 61,672,983
instructions, 3.35% of the whole program. A task note from a fortnight ago
put it at 0.6%; the note was an estimate and the profile is not.

Per call that is 22.9 instructions, against a fast path of six stores and
three tests. Disassembled, the path was 23 instructions and nine of them
existed to turn `k_beat_depth` into `&k_beat_stack[depth - 1]`:

    mov k_beat_depth,%eax / dec / cmp $0x3f / ja
    mov %eax,%edx / mov %rdx,%rax / shl $5
    lea k_beat_stack,%rcx / lea (%rcx,%rax,1),%rdi

That address cannot change for the life of the loop, and the compiler cannot
know it: the loop body calls other functions, any of which might push a beat.

Two changes, measured separately.

**The registry summary moves into the mark it describes.** `k_reg_any` was a
parallel `int[K_BEAT_MAX]` indexed by depth, so the rewind — which has the
mark pointer in hand — had to turn it back into a depth to read the flag. It
is a field of `KMark` now, read at a displacement. In the same step the two
flag tests become one: `k_buf_dirty` and `reg_any` are both zero on
essentially every rewind, and `!(k_buf_dirty | m->reg_any)` is one branch
where two predicted-taken jumps stood. 23 instructions to 20, and runbench
1,840,367,648 → 1,832,202,462, −0.4437%.

**The innermost mark is cached beside the depth.** `k_beat_top` holds
`&k_beat_stack[k_beat_depth - 1]`, or NULL at depth zero, and the eight
remaining address instructions become a load and a test. 20 to 15.

The cache is not free, and where it is paid is worth writing down. Seven
sites move the depth and each now maintains the pointer. At `k_beat_push`
the new top is the mark just written and the range test is dead code, so
that site is one store: +500,595 over 507,685 pushes. At `k_beat_pop` the
new depth may be zero or past the top, so the cmov stays: eight instructions,
+4,004,752. Against those, `k_beat_iter` gives back 21,468,255.

    k_beat_iter   61,672,983 -> 40,204,728   -21,468,255   -34.81%
    k_beat_pop    14,517,216 -> 18,521,968    +4,004,752
    k_beat_push   15,017,844 -> 15,518,439      +500,595
    runbench   1,840,367,648 -> 1,823,406,517  -16,961,131   -0.9216%

The three account for the total within 1,777 instructions. The ratio is what
makes it pay: runbench iterates 2,692,766 times against 507,685 pops, five to
one, so five instructions moved off the iteration buy eight onto the pop.

These are this container's callgrind readings. `bench/instructions_golden.txt`
refuses comparison here — the rows were measured on glibc 2.39-0ubuntu8.9 and
clang 19.1.1 against this box's 8.7 and 18.1.3 — so CI takes the row and the
floor is banked after it lands.

**What the cache costs in safety, and what pays for it.** A stale `k_beat_top`
is not a crash. It rewinds the arena to an OUTER loop's mark, freeing memory
the inner loop is still reading, and what surfaces is a wrong answer somewhere
else entirely. So the counting build asks at every iteration whether the
cached pointer is the one the depth names, and dies by name when it is not.
`tests/the_cached_beat_top_tracks_the_depth.rs` runs beats nested three deep
under `--counters`; dropping the maintenance from `k_beat_pop` turns it red
with `the cached beat top and the beat depth disagree`.

Getting that spec to fail took two tries, and both failures are the reason it
is worth having. The first program built strings into its accumulator, which
compiles to a CARRY beat: `k_beat_iter_carry` computes its own mark and never
reads the cache, so the emitted code called it four times, called `k_beat_iter`
not at all, and the spec passed with the maintenance removed. The second
carried a scalar and allocated nothing — and a loop with nothing to reclaim
emits no beat at all. What the spec needs is both: laps that allocate, and a
carried value that is a scalar. Each lap builds a padded string and keeps only
its length.


## 2026-09-17 — kanso#1504 on CI: the beat rewind's row, and the one thing it costs

The container projected runbench 1,840,367,648 → 1,823,406,517, −16,961,131,
−0.9216%. CI, on its own machine and its own baseline, reads 1,821,933,936 →
1,804,998,570: a fall of **16,935,366, 0.9295%**. The two deltas are 25,765
apart, 0.0014% of the number, which is as close as this vein gets between
machines — and is why the per-frame attribution taken on the container can be
trusted even though its absolute figures cannot be compared with CI's.

    k_beat_iter   61,672,983 -> 40,204,728   -21,468,255   -34.81%
    k_beat_pop    14,517,216 -> 18,521,968    +4,004,752
    k_beat_push   15,017,844 -> 15,518,439      +500,595

**Fourteen run-side rows moved, not one, and the spread is the finding.** The
first push wrote runbench's number alone and CI refused it, which was right:
thirteen rows were left describing a runtime this branch had widened. A vein is
the whole file.

    escapebench      84,780,592 ->     75,228,606    -9,551,986  -11.2667%
    basket           33,678,746 ->     32,776,834      -901,912   -2.6780%
    runbench      1,821,933,936 ->  1,804,998,570   -16,935,366   -0.9295%
    livebench     2,825,430,323 ->  2,805,024,580   -20,405,743   -0.7222%
    encodebench   3,497,149,260 ->  3,476,743,520   -20,405,740   -0.5835%
    oneshot          17,888,155 ->     17,837,178       -50,977   -0.2850%
    readbench         4,628,429 ->      4,627,056        -1,373   -0.0297%
    digestbench       9,967,039 ->      9,966,673          -366   -0.0037%
    scanbench       462,269,296 ->    462,269,305            +9   +0.0000%
    jsonbench     1,133,644,520 ->  1,133,645,592        +1,072   +0.0001%
    pendbench       208,138,815 ->    208,139,955        +1,140   +0.0005%
    indexbench        2,895,708 ->      2,895,743           +35   +0.0012%
    deepbench       347,289,236 ->    347,635,275      +346,039   +0.0996%
    widebench        33,516,094 ->     33,644,020      +127,926   +0.3817%

A row falls in proportion to how much its program beat-loops. escapebench is
the extreme at 11.27% because escaping a string is a tight beat loop with
almost nothing else in it, so the fifteen instructions are most of what a lap
costs. The rises are the layout term: every binary grew 368 to 560 bytes,
because the mark carries a field more and there is a new global beside it, and
deepbench and widebench are the two paying that without beat loops to spend it
on. The objective weighs `work_runbench` alone, so welfare reads 76.71 either
way; the other thirteen rows are watched rather than scored, which is exactly
why the vein is diffed whole.

Eight compile-side rows moved, and seven of them are layout. `src/runtime.c`
is `include_str!`'d into the compiler, so changing it changes the compiler's
own bytes and what the linker does with them:

    compile_instructions      35,869,355 ->     35,870,761      +1,406   +0.0039%
    entry_instructions       127,872,255 ->    127,877,328      +5,073   +0.0040%
    library_instructions     128,010,052 ->    128,015,155      +5,103   +0.0040%
    interp_instructions    2,182,307,043 ->  2,182,420,936    +113,893   +0.0052%
    startup_instructions       3,955,899 ->      3,957,812      +1,913   +0.0484%
    codegen_instructions_dev 596,161,166 ->    596,182,348     +21,182   +0.0036%
    emit_instructions         60,197,743 ->     60,201,844      +4,101   +0.0068%
    runbench text                319,954 ->        320,514        +560

The eighth is not layout. `codegen_instructions_release` rises **11,227,515,
0.1645%** — that row counts the C toolchain and excludes kanso's own process,
so it is the only one that COMPILES runtime.c rather than carrying its bytes,
and clang at `-O3 -flto` now has a mark with a field more and a global beside
it. Both readings in the job were identical.

So the trade is: 11.2 million instructions once per release build, against
16.9 million on every run of the program. The objective weighs run speed at
0.45 on the production side and the release build at 0.15, and takes it —
welfare 76.65 → 76.71, banked.

**No page can quote the run-side vein.** Publishing CI's row, the page got
`data-golden="run.runbench"` and `golden_prose` answered `UNKNOWN KEY
run.runbench`. `golden_for` knows three families — decode, encode and compile —
and anything else resolves against an empty golden;
`bench/instructions_golden.txt` also writes `name value` rows where the
parser wants `name=value`, so listing it would take widening the parser too.
The attribute came off and the row is plain text on the page until both are
done. kanso#1337 cost a run to the same gap on the library vein, and the
gate's own comment records it.

The gate itself is sound, and this entry nearly said otherwise. `--write`
prints the unknown key and carries on, because there is nothing for it to
rewrite, and reading that output alone it looks like a warning. Run plain,
`golden_prose` exits 1 and `all_pages.sh` reports `pages objected:
golden_prose` — checked by injecting the bogus key and reading the exit code
rather than the text. A claim about what a guardrail does is worth the thirty
seconds it takes to watch it fail.


## 2026-09-18 — kanso#1504 re-merged onto main, and every run-side row named with the value it landed on

Three landed underneath this branch while it sat dirty and invisible:
kanso#1486, kanso#1496 and kanso#1507. The eight compile-side goldens carry
MAIN'S values forward and the merged sitting is CI's; kanso#1507 in particular
changed what the release-codegen row COUNTS, by pinning ld's LLVM plugin to one
thread, so nothing this branch measured on that row is comparable with anything
measured after it.

The run-side veins are this branch's own and survived the merge untouched.
Every one of the fifteen, named with the value it landed on:

    work_basket        33,678,746 ->    32,776,834    -901,912   -2.678%
    work_escapebench   84,780,592 ->    75,228,606  -9,551,986  -11.267%
    work_runbench   1,821,933,936 -> 1,804,998,570 -16,935,366   -0.930%
    work_livebench  2,825,430,323 -> 2,805,024,580 -20,405,743   -0.722%
    work_encodebench 3,497,149,260 -> 3,476,743,520 -20,405,740  -0.583%
    work_oneshot       17,888,155 ->    17,837,178     -50,977   -0.285%
    work_readbench      4,628,429 ->     4,627,056      -1,373   -0.030%
    work_digestbench    9,967,039 ->     9,966,673        -366   -0.004%
    work_indexbench     2,895,708 ->     2,895,743         +35   +0.001%
    work_scanbench    462,269,296 ->   462,269,305          +9   +0.000%
    work_jsonbench  1,133,644,520 -> 1,133,645,592      +1,072   +0.000%
    work_pendbench    208,138,815 ->   208,139,955      +1,140   +0.001%
    work_deepbench    347,289,236 ->   347,635,275    +346,039   +0.100%
    work_widebench     33,516,094 ->    33,644,020    +127,926   +0.382%
    text                1,755,372 ->     1,762,300      +6,928   +0.395%

**Nine fall and six rise, and the six are the layout term.** `k_beat_top` is a
pointer the runtime now carries, so every binary grew: `text` is up 6,928
bytes, 0.395%. The two largest rises sit near that figure without matching it
— `work_widebench` 0.382%, `work_deepbench` 0.100% — which is what a shifted
working set looks like, since how much a binary's growth costs a given run
depends on what that run touches. The other four rises are 35, 9, 1,072 and
1,140 instructions, a handful on runs of millions to billions.

The falls are the change: a beat that finds its mark instead of computing it
retires 23 instructions where it retired 15, and the benchmarks that rewind
most often gain most. escapebench rewinds on every escape and gains 11.27%;
the scanners and the index, which barely beat at all, do not move.
## 2026-09-18 — kanso#1504's compile-side rows on the merged tree, and the release row reproducing

The run-side veins were this branch's own and are recorded above. These eight
are CI's sitting on the tree merged with kanso#1486, kanso#1496 and kanso#1507:

    codegen_instructions_release 6,822,651,561 -> 6,841,893,129 +19,241,568 +0.2820%
    emit_instructions               60,196,725 ->    60,221,314     +24,589 +0.0408%
    codegen_instructions_dev       596,159,774 ->   596,180,956     +21,182 +0.0036%
    interp_instructions          2,182,576,109 -> 2,182,585,809      +9,700 +0.0004%
    startup_instructions             3,951,796 ->     3,953,725      +1,929 +0.0488%
    compile_instructions            35,441,027 ->    35,441,565        +538 +0.0015%
    library_instructions           126,804,425 ->   126,804,746        +321 +0.0003%
    entry_instructions             126,349,040 ->   126,348,616        -424 -0.0003%

**The release row is the one real cost and this branch expected to pay it.**
It is the only row that COMPILES src/runtime.c rather than carrying its bytes,
and the beat cache adds a pointer and the code that keeps it: 19.2 million
instructions of clang and ld, 0.282%. The dev tier pays a twentieth of that for
the same change, because `-O0` does far less with the extra code. The other six
are under a twentieth of a per cent apiece and are layout.

**AND THE RELEASE ROW REPRODUCED.** `codegen_release_again` read
6,841,893,129 — the same number, in the same job. That matters more than the
value: before kanso#1507 pinned ld's LLVM plugin to one thread, this row could
not be read twice and get one answer, and it halted its own vein on exactly
that failure two rounds ago. This is the first sitting where a tree that
CHANGES runtime.c reads it twice and agrees, which is a stronger test of the
pin than the trees that left runtime.c alone.

The trade is the objective's to judge and it judges in favour: nine run-side
veins fall, the largest 11.27%, against 19.2 million on a row weighted for
production build cost. Welfare rose and is banked.
**And the floor was banked twice on this branch, because the first bank broke
the rule that exists for exactly this.** "Bank AFTER the goldens carry CI's
rows, never before." The first `--set` here ran while the eight compile-side
goldens still held MAIN'S values carried forward, so it recorded a score this
container projected from rows nobody had measured: 76.71669769306608. CI then
measured them, the release row came in 19.2 million higher than main's, and
welfare read 0.01 BELOW the floor its own branch had just set. A branch cannot
fail its own bank without something being wrong with the bank.

The second `--set` is CI's figure, 76.7108285575541, and it is still a rise of
0.053 over main's 76.6576 — the change is a gain, and the projection was
simply too generous about a row it had not seen. Recorded rather than quietly
re-run, because the failure looks exactly like a regression in the logs and is
not one: nothing about the change moved between the two banks, only what was
known about it.
## 2026-09-18 — kanso#1504's rows re-measured on the tree merged with kanso#1509

kanso#1509 landed under this branch and moved the compile-side rows on its own,
so every figure this branch had measured before it was taken against a base
that no longer exists. The five affected goldens were carried forward at main's
values and the round re-measured them. CI's sitting, with the second reading in
the same job matching the first to the instruction on all four rows that take
one:

    compile_instructions    35,441,774 ->    35,442,739    +965   (+0.0027%)
    entry_instructions     126,350,802 ->   126,352,290  +1,488   (+0.0012%)
    library_instructions   126,806,203 ->   126,807,903  +1,700   (+0.0013%)
    startup_instructions     3,933,223 ->     3,935,119  +1,896   (+0.0482%)
    emit_instructions       52,115,454 ->    52,140,118 +24,664   (+0.0473%)

**All five are LAYOUT.** `src/runtime.c` is `include_str!`'d into the compiler,
so a change to it changes the compiler's own bytes and the layout under them.
None of these five routes runs the beat code this branch touches: three of them
are `kanso check` and carry runtime.c's bytes without compiling it, `emit_ir`
stops before the backend, and the interpreted start-up links the runtime but
does not execute the rewind. The two largest rises in absolute terms are the
two smallest baselines, which is what a fixed layout term looks like spread
over rows of different sizes.

`interp_instructions` held at 2,182,585,809, the value this branch measured
before the re-merge, and both codegen rows agreed with their goldens:
`codegen_instructions_dev` 596,180,956 and `codegen_instructions_release`
6,841,893,129. **The release row reproducing is the thing worth noticing.**
That row is the one kanso#1507 pinned by holding `ld`'s LLVM plugin to one
thread, and this branch changes `src/runtime.c`, which is the only input the
release row compiles rather than carries. It has now read the same number on
two different jobs on two different trees that both change runtime.c.

The floor is re-banked on these rows rather than on the projection the
re-merge carried.

## 2026-09-18 — kanso#1504's rows on the tree merged after kanso#1511, and a floor that had been banked on main's row

CI's sitting on `38fa8750`, every row with the value it landed on:

    compile_instructions      35,442,006 ->    35,442,391       +385  (+0.0011%)
    entry_instructions       126,350,641 ->   126,351,986     +1,345  (+0.0011%)
    library_instructions     126,806,286 ->   126,807,492     +1,206  (+0.0010%)
    startup_instructions       3,363,379 ->     3,363,835       +456  (+0.0136%)
    interp_instructions    2,182,527,453 -> 2,182,576,175    +48,722  (+0.0022%)
    emit_instructions         51,543,408 ->    51,547,188     +3,780  (+0.0073%)
    codegen_instructions_dev     596,157,624 ->   596,197,703    +40,079  (+0.0067%)
    codegen_instructions_release 6,824,133,280 -> 6,841,691,425 +17,558,145 (+0.2573%)

The first six are layout. The branch's own source has not moved since the
previous sitting, and what changed under it is main.

**The release row is not layout, and the floor had been banked as though it
were.** The branch measured 6,841,893,129 for itself at `3db62375`. The
2026-09-18 merge of kanso#1512 resolved
`bench/codegen_instructions_release_golden.txt` toward main, so the tree
carried main's 6,824,133,280 — and the floor was then re-banked on that tree,
at 76.88347521753009, crediting the beat rewind with a codegen row 17.5
million instructions cheaper than the one it produces. This job reads
6,841,691,425, which is 201,704 below the branch's earlier figure and
17,558,145 above main's. Two readings of the branch's own cost that agree to
0.003% is what a real cost looks like; the value that sat between them for a
day was main's.

So the floor is re-banked at 76.87843049336072 on the tree's own eight rows.
The floor before the bad bank was 76.87853949372271, so this is a restoration
within 0.00014 rather than a regression admitted.

**The rule it breaks is one this file already carries, with a different
victim.** "Carry ALL rows forward or none" was written about the trend gate:
leaving one row at the branch's value while the others take main's makes a
fall that paid for a rise read as main's. The same resolution going the other
way — a row taken from main while the rest stay the branch's — costs the
FLOOR instead, and it is worse, because the trend gate says so out loud and a
bank says nothing at all. A merge that touches a golden the branch has
measured for itself is a merge that needs the branch's number put back before
anything is banked on the tree.

## 2026-09-18 — kanso#1504 re-merged onto main after kanso#1515

kanso#1515 landed underneath this branch and took 14,155,510 instructions off
the interpreted row, 2,182,584,048 -> 2,168,428,538. That row and the five
compile-side rows beside it carry MAIN'S values now: this branch's readings
were taken against a tree that no longer exists, and carrying main's forward
gives each gate one number to fail against rather than none while making CI's
diff read exactly what this branch does to today's main.

`bench/codegen_instructions_release_golden.txt` did not conflict, so the
branch's own 6,841,691,425 stands — the gain this PR is for is still in the
tree while the six rows around it are main's. The floor is main's,
76.82771395446468, because a floor banked on the branch's layout rows prices a
tree that no longer exists; `interp_instructions` is a weighed development term
and main's row is 14.2 million lower, so the merged tree scores above this
floor and the ratchet follows CI's sitting.

The three compile spans on the page follow the goldens and carry main's values
with them.

## 2026-09-18 — kanso#1504's rows on the tree merged after kanso#1515

Six layout rows, one job, against the values carried forward from main:

      compile           35,443,611 ->    35,443,639       +28   +0.0001%
      entry            126,354,834 ->   126,356,158    +1,324   +0.0010%
      library          126,810,299 ->   126,811,904    +1,605   +0.0013%
      interpreted    2,168,428,538 -> 2,168,245,153  -183,385   -0.0085%
      start-up           3,363,774 ->     3,364,590      +816   +0.0243%
      emitting          51,546,788 ->    51,544,132    -2,656   -0.0052%

Four rose, two fell, every one under three hundredths of a per cent and with
mixed signs, which is what a shifted binary looks like. The compile, entry,
library and emit rows each read the same value twice in the job.

`codegen_instructions_release` reads 6,841,691,425, the branch's own row from
its pre-merge sitting: main did not touch that golden, so the merge left it
alone and it is the term this branch actually pays. `codegen_instructions_dev`
reads 596,197,703, also the branch's own. The floor is banked at
76.88172594705054.

The same comparison beside kanso#1502's is worth keeping: both branches sat on
identical carried-forward rows, and the interpreted row fell 183,385 here and
162,511 there. Two different changes, two falls of the same order on a row
neither of them executes, which is the layout term's size on this tree rather
than anything either branch did.

## 2026-09-18 — kanso#1504 re-merged onto main after kanso#1516

kanso#1516 landed the interpreter's name memory underneath this branch, so all
six layout rows and the floor carry MAIN'S values again. The branch's own
readings were taken against a tree that no longer exists, and
`bench/codegen_instructions_release_golden.txt` did not conflict, so the row
this branch is actually for -- 6,841,691,425 -- is untouched by the merge.

Third re-merge for this branch. Each costs it a round, and the cost is
`required_status_checks.strict` with several changes in flight rather than
anything wrong with any of them.

## 2026-09-18 — kanso#1504 re-merged onto main after kanso#1517

kanso#1517 landed the callee memory under this branch, so the five layout rows
and the floor carry MAIN's values again and this round is deliberately red on
them.

The goldens auto-merged this time and that was checked rather than trusted:
every one of the six instruction rows now reads main's value exactly —
compile 35,486,173, entry 126,498,498, library 126,953,661, start-up 3,362,788,
emitting 51,451,897, interpreted 1,963,826,350. An auto-merge on a golden is
worth a diff, because git will take a clean apply on a file where the right
answer is a judgement.

Two rows are the branch's own and CI measured them: codegen release at
6,841,691,425 and dev at 596,197,703. The release one costs 0.007 points and
buys the run side 1,804,998,570, which is 14,293,146 instructions below the
tree kanso#1502 sits on.

Welfare reads 76.93 against main's 76.88. The 0.05 is not banked here for the
same reason it was not banked on kanso#1502: welfare weighs the five carried
rows, so a --set now would freeze a score this container projected rather than
the one CI measures. The rows come first.

## 2026-09-18 — kanso#1504, CI's rows on the tree merged after kanso#1517

    compile_instructions      35,486,173 ->    35,487,349    +1,176   +0.0033%
    entry_instructions       126,498,498 ->   126,500,546    +2,048   +0.0016%
    library_instructions     126,953,661 ->   126,956,802    +3,141   +0.0025%
    startup_instructions       3,362,788 ->     3,363,385      +597   +0.0178%
    emit_instructions         51,451,897 ->    51,456,279    +4,382   +0.0085%
    interp_instructions    1,963,826,350 -> 1,963,826,376       +26   +0.0000013%

TWO BRANCHES MEASURED IN THE SAME HOUR GIVE THE INTERPRETED ROW A CROSS-CHECK
IT HAS NOT HAD BEFORE. kanso#1502 and this one are different edits to
`src/runtime.c` — two divisions in float rendering there, the cached beat top
here — both merged onto the same main, both read by CI within two minutes of
each other. The interpreted row moved 15 on one and 26 on the other.

That is worth more than either number alone. The corpus decodes a document it
built itself and never enters the C runtime, so neither edit can give it work;
if one of them had, the two would not both land in the tens on a row of
1,963,826,350. Both branches also read `interp_allocs` 4,810,437 and
`interp_peak_bytes` 951,438 exactly, and an allocation counter counts operations
rather than a host, so a row with real work in it would have moved that one too.

The five layout rows rose on both branches, by different amounts in the same
direction, which is the ordinary signature of a moved binary rather than of
work.

`compile_allocs` read 27,313. Both codegen rows are this branch's own and read
exactly: 596,197,703 dev and 6,841,691,425 release.

## 2026-09-18 — kanso#1504 re-merged onto main after kanso#1518

Three conflicts, the same three as kanso#1502 took in the same hour and for the
same reason: kanso#1518 moved the interpreted rows and left every layout golden
alone, so those auto-merged.

The interpreted rows carry main's — 1,555,890,579, 3,879,653 and 961,165. This
branch's cached beat top moved that row by 26 instructions on 1.96 billion last
sitting, so tens is what it should read again; anything larger is kanso#1518's
arithmetic, not the beat's.

## 2026-09-18 — kanso#1504, CI's row: nineteen instructions, and a third reading in the tens

One vein disagreed and it disagreed by nineteen:

    interp_instructions   1,555,890,579 -> 1,555,890,598   +19   +0.0000012%

Every other row read its golden exactly — all five layout rows, both codegen
rows, `compile_allocs`, and both interpreted memory rows.

THREE RUNTIME EDITS IN ONE DAY HAVE NOW MOVED THIS ROW BY 15, 26 AND 19. They
are three different functions in `src/runtime.c` — two divisions in Ryu's float
rendering, the cached beat top, and the beat top again against a newer main —
and every reading lands under thirty on a row of one and a half billion. The
corpus decodes a document it built itself and never enters the C runtime, so
none of them can be work.

`interp_allocs` agreeing at 3,879,653 across all three is what makes that a
check rather than an assertion. An allocation counter counts operations rather
than a host, and real work in the interpreted run would have moved it.

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
## 2026-09-22 — name equality stops calling out, and the interpreted run falls 1.71%

`__memcmp_avx2_movbe` was 4.71% of the interpreted run: 47,999,433 instructions
over 2,617,709 calls, 18.3 each, on names of twenty-two bytes or fewer.
`callgrind_annotate --tree=caller` does not say who calls it -- at threshold 100
the callers it lists sum to about fifty thousand -- but the raw profile does,
and summing each `cfn=` by its caller accounts for the frame exactly.

## 2026-09-22 — the whole function table, for the six gates that still could not be diffed

kanso#1558 gave `compile_instructions.sh` an uncapped print of its profile's
function table, after three instructions could not be located from two job
logs. The gate carried `--threshold=90 | head -40`: forty rows of the hundred
and twenty-five that threshold has, fifteen functions once the headers come
off, and every one of the fifteen equal across the two jobs.

That fix landed for one gate. Six others carried the identical line.

    entry_instructions     /tmp/cg.entry      threshold=90 | head -40
    library_instructions   /tmp/cg.library    threshold=90 | head -40
    startup_instructions   /tmp/cg.startup    threshold=90 | head -40
    interp_instructions    /tmp/cg.interp     threshold=90 | head -40
    instructions           /tmp/cg.$b         threshold=90 | head -40, two of fourteen
    emit_instructions      /tmp/cg.emit       inclusive only

The fourth is the gate STATUS.md's standing row is about. `interp_instructions`
read 2,178,502,266 and 2,178,502,272 on two CI jobs of one commit, each stable
across the gate's own second reading, and the row has been open since
2026-09-15 for want of a carrier. The instrument that would say which frames
moved did not exist on that gate. The fifth printed a breakdown for two of the
fourteen benchmarks it profiles, so a move in any of the other twelve had
nothing behind it at all.

WHAT WENT IN. Each gate now prints its profile's whole exclusive table, on
every run, before the comparison -- the same three properties kanso#1558
established, for the same reason. On every run, because a diff needs the side
that AGREED and the agreeing side never takes a failure path.

Checked against a real profile rather than by reading: 659 rows off a
`kanso --version` run, reaching functions that retire a single instruction.
The top exclusive frame is `_mi_strnicmp` at 243,800, which is mimalloc
resolving its options by name -- the frame the standing row's own measurement
found moving 117 instructions per environment variable. The instrument sees
what the row is about.

`tests/every_callgrind_gate_prints_its_whole_table.rs` derives the governed
list off disk: every gate writing a callgrind profile, less two exemptions
named with reasons. Each of its three assertions was watched red on its own
mutation -- the print removed, the print capped, the print moved into the
failure path.

TWO EXEMPTIONS, and they are reasons rather than spellings. `codegen_
instructions.sh` names its profiles `/tmp/cg.codegen.$tier.%p`, one per
process across the clang driver, the convention probe, `clang -cc1` and ld, at
6.8 billion instructions on the release tier; it also already diffs its own two
readings per frame. `path_independence.sh` writes `/tmp/cg.pi` and never reads
it -- the count comes off valgrind's stderr and every measurement in its loop
overwrites the same path, so nothing survives to print. Both want their own
change and their own measurement of what it costs the job log.

The derivation was wrong on its first run and the spec said so: filtering for
a profile path with no shell expansion dropped `emit_instructions` and
`instructions`, which write `"$out"` and `/tmp/cg.$b` and are one profile per
run all the same. What separates codegen is many processes under one reading,
which is why that is in the exemption list and not in the filter.

This does not close the standing row. It builds the instrument the row has
been guessing without, and the row comes off when a carrier for the six is
found.
## 2026-09-22 — the interpreted row moves with .text, and the frame that carries it is memcmp

The instrument above was built for STATUS.md's standing row, which has wanted a
carrier since 2026-09-15 for a six-instruction difference between two CI jobs on
one commit. Its first use, on this container, three arms and a control:

    arm                                  .text      .rodata           row
    main, unpadded                   2,862,578      825,896   973,143,830
    main, read again                 2,862,578      825,896   973,143,830
    +64 KiB of rodata no code reads  2,862,578      891,432   973,143,830
    +40 never-called functions       2,862,690      891,432   973,510,133
    +38 of the same functions        2,862,690      891,432   973,510,133

Each arm is a distinct binary by sha256. The control is the second row: one
binary read twice is byte-identical, which is what CI also reports of its own
second readings.

GROWING RODATA DOES NOT MOVE IT. Sixty-four kilobytes of a `#[used]` array no
code reads leaves the row unchanged to the instruction.

GROWING TEXT DOES, and by a lot for the size: 112 bytes of `.text` moves the
row **+366,303**, 0.038%. The forty probe functions are identical bodies and
the linker folds them, which is why forty and thirty-eight give the same
sections -- and the same row, on different binaries.

WHERE IT GOES, off the whole-table diff:

        +366,005  __memcmp_avx2_movbe [libc.so.6]
            +320  _dl_relocate_object   (the probe's own relocations, outside
            +200  _dl_relocate_object    the row's anchor)
             +52  __memcpy_avx_unaligned_erms
    single digits  eleven others, -4 to -1

One frame is the move. The interpreter is deterministic and its input is fixed,
so the same comparisons happen in every arm; what changed is where the bytes
being compared sit. `__memcmp_avx2_movbe` takes a different number of
instructions for the same comparison depending on its arguments' addresses.
That the delta arrived with the `.text` growth is a difference-in-differences,
and the mechanism inside memcmp is left open.

WHAT THIS DOES NOT SHOW is that this carries the six. kanso#1558 reported CI's
non-reproducing binaries as having identical section sizes, and the fifth arm
here says two binaries with identical sections read an identical row. So section
size is not the carrier of a six between two CI jobs. What the standing row
gains is a frame to look at and one candidate struck off: its own note named
`_mi_os_commit_ex`, `mi_bitmap_setN` and `_mi_prim_commit` as where to look
next, and across every arm here the allocator's commit frames do not appear in
the moved list at all.

A NOTE ON READING THE TABLE, because the first diff of these two profiles was
wrong and the mistake is easy. `callgrind_annotate` prints `count name`
separated by spaces and the names contain spaces -- `<alloc::vec::into_iter::
IntoIter<T,A> as core::iter::traits::iterator::Iterator>::try_fold`. Splitting
on whitespace truncates every generic at its first space, six distinct
instantiations collapse onto one key, and the diff then reported ten million
instructions moving in `IntoIter`. Stripping thousands separators from the whole
line rather than from the count does the same thing to `<T,A>`. Join on a tab
and neither happens: 1,397 rows, 1,397 distinct keys, four movers. The printed
table itself is faithful -- `callgrind_annotate` does not truncate when its
output is not a terminal, checked by counting distinct keys with and without
`COLUMNS` set, and they are equal.

AND A LEAD THAT IS NOT ABOUT MEASUREMENT AT ALL. `__memcmp_avx2_movbe` is
**4.71%** of the interpreted run, 47,999,433 instructions of 1,019,911,013.

`callgrind_annotate --tree=caller` does not answer who calls it: at threshold
100 the callers it lists sum to about fifty thousand, three parts in a thousand
of the frame. The raw profile does. Callgrind records each call site as a
`cfn=` beside its caller's `fn=` with the call's inclusive cost on the next
line, so summing those by caller is a twenty-line read of the file, and it
accounts for **47,999,433** -- the frame exactly, which is the check that the
parse is right.

    13,336,477   27.8%   kanso::eval::lookup
    12,535,464   26.1%   kanso::eval::Interp::call_builtin
     6,390,396   13.3%   kanso::eval::Interp::eval_global
     3,277,188    6.8%   kanso::eval::Interp::call_named

`eval::lookup` walks the environment comparing each bound name to the one being
looked up. The comparing was never the cost: the bytes fit in two registers, and
`str == str` checks the lengths and then calls out, so the fourteen instructions
a comparison bought the AVX2 entry sequence and the call around it.

THE FIRST ATTEMPT WAS WRONG AND THE MEASUREMENT SAID SO. It gave `Name` an
`eq_str` taking a `&str`, padding it into a twenty-two byte buffer to get the
same fixed-width loads. Every one of the 909,375 calls went, the frame fell
13,336,477 -- and the row went UP 288,354. The zero-fill and the copy came to as
much as the call they replaced. Both sides have to be inline already.

WHAT LANDED. `PartialEq for Name` compares two inline names as `buf[0..16]` as a
`u128` and `buf[14..22]` as a `u64`. The ranges cover all twenty-two bytes and
overlap by two, which is what makes the second load fixed-width instead of a
tail loop. `lookup` and `eval_ident` take a `&Name` rather than a `&str`, which
they can because `Expr::Ident` has held a `Name` since the 2026-08-29 ruling;
the one cold caller, a `set` statement's target, is a `String` and builds an
inline `Name` at the call.

    interpreted row   973,143,830 -> 956,538,727   -16,605,103   -1.71%
    memcmp calls        2,617,709 ->   1,707,612      -910,097
    memcmp cost        47,999,433 ->  35,413,346   -12,586,087

The row falls by four million more than the frame does, which is `lookup` itself
getting cheaper once the call is gone. Every other caller of memcmp is
byte-identical across the two profiles, which is the check that nothing else
moved.

CI'S SITTING, and every row it moved fell.

    interp_instructions     923,151,727 -> 908,952,299  -14,199,428  -1.5381%
    emit_instructions        51,618,058 ->  51,481,045     -137,013  -0.2654%
    library_instructions    127,184,941 -> 127,146,502      -38,439  -0.0302%
    entry_instructions      126,728,843 -> 126,691,703      -37,140  -0.0293%
    compile_instructions     35,549,673 ->  35,540,015       -9,658  -0.0272%
    startup_instructions      3,363,186 ->   3,362,329         -857  -0.0255%

Both codegen rows are byte-identical, which is right: they count the C
toolchain and this change is kanso's own Rust.

The five compile-side falls are not layout. The FRONT END compares names too --
resolving, checking, inferring -- and `compile_instructions` is `kanso check`
over a library, so it pays the same comparison the interpreter does. That was
not predicted here before CI measured it.

The container read the interpreted saving at 16,605,103 and CI reads
14,199,428, 14% apart. Both are falls of the same shape and the golden is CI's;
the container's figure never goes in it.

Welfare 77.27959877643865 -> 77.28677792407876, banked in this commit and after
the goldens carried CI's rows rather than before. Development moves 78.45 to
78.49 and production does not move at all, which is what a change to the
compiler's own Rust should look like. The saving is a floor rather than an estimate for a second reason: `.text`
grew 4,832 bytes, and the entry two above this one measured `.text` growth
pushing this row UP -- 112 bytes moved it +366,303 -- so whatever the layout term
is doing here, it is working against the number above.

THE SPEC. `tests/a_name_compares_by_the_word_and_still_by_the_text.rs` is about
the two loads and where they meet. Byte 16 is the first byte only the second
load sees, bytes 14 and 15 are read twice, byte 21 is the last byte anything
sees, and a comparison that quietly stopped at byte 16 would pass a spec built
from short names alone -- 89.8% of real identifiers are seven bytes or fewer, so
the corpus would never have said. It flips one byte at each of the twenty-two
positions in turn.

Watched red on three mutations. Dropping the second load and shortening its
range both fail `one_byte_apart_is_not_equal`. Dropping the LENGTH CHECK does
not fail anything, and that is correct rather than a gap: `Name::new` zero-fills
past the length, so two names of different lengths differ in the buffer as well.
The check stays because a reader should not have to know about the fill to
believe the comparison, and because it is what keeps this right if the fill ever
changes.

`call_builtin` is the other half and is not touched here: 12,535,464
instructions over 564,790 calls, comparing a `&str` against string literals.

WHAT SHAPE THOSE LITERALS ARE IN was written down wrong twice before it was
read, and both wrong versions reached a pull request body. It is not an
if-chain wanting a `match`. It IS a `match name { ... }` already, over 49
string-literal arms -- three bare `name == "..."` tests sit ahead of it and the
rest is the match. Rust lowers a string match to a switch on the length and
then a comparison against each candidate of that length, and those comparisons
are the calls. So no rearrangement of the arms helps.

What would is comparing something other than bytes: a builtin resolved to an id
once, where the checker already knows the name is a builtin, instead of
resolved by its text at every call. That is a bigger change than this one and
it is not specified here beyond the shape.

     3,105,230    6.5%   kanso::eval::Interp::eval_tail::{{closure}}
     2,988,189    6.2%   kanso::eval::Interp::dispatch_loop
     2,834,403    5.9%   <num_bigint::biguint::BigUint as PartialEq>::eq
       895,227    1.9%   hashbrown::map::HashMap::contains_key
       418,640    0.9%   kanso::eval::eval_binop
       331,938    0.7%   kanso::eval::match_one

Six of the first eight are name resolution: looking a name up in the
environment, choosing a builtin, resolving a global, dispatching a call. They
are 41.6 million instructions between them, **4.1% of the interpreted run**,
and what they are doing is comparing names byte by byte. §117 of the page
records why: a name is the front end's identifier type, which keeps twenty-two
bytes or fewer inline, so equality on it is a memcmp of those bytes. That was
the right trade for the allocator -- it took 1.3 million allocations out -- and
it left every comparison a byte compare.

HOW BIG EACH CALL IS decides what the fix is, and the first draft of this
paragraph guessed wrong. It said an interned name would make each comparison a
word compare, which is true and is more work than the numbers ask for. The
profile carries the call counts beside the costs:

    memcmp total          2,617,709 calls    47,999,433    18.3 each
    eval::lookup            909,375 calls    13,336,477    14.7 each
    Interp::call_builtin    564,790 calls    12,535,464    22.2 each
    Interp::eval_global     332,026 calls     6,390,396    19.2 each
    Interp::call_named      165,949 calls     3,277,188    19.7 each

Eighteen instructions a call, on names of twenty-two bytes or fewer. That is
the AVX2 entry sequence and the call itself rather than the comparing: the
bytes being compared fit in two registers.

THEN READ THE CALL SITES, which is the step this entry skipped twice. It first
said the fix was interning; corrected, it said an inline word compare on
`Name`'s own `PartialEq`. Neither top caller compares a `Name` to a `Name`.

`eval::lookup` walks the environment with `bound.as_str() == name`, a `Name`'s
text against a `&str` the caller holds. `Interp::call_builtin` compares a
`&str` against string literals. Those are two different shapes and only the
first is the identifier type's to fix.

This paragraph first said the second was an if-chain wanting a `match`. It is a
`match name { ... }` already, over 49 string-literal arms, with three bare
`name == "..."` tests ahead of it. Rust lowers a string match to a switch on
the length and then a comparison against each candidate of that length, and
those comparisons are the calls, so rearranging the arms buys nothing. What
would is comparing something other than bytes -- a builtin resolved to an id
once, where the checker already knows the name is a builtin.

So the lead is two leads. The `lookup` half is 13,336,477 instructions, 1.3%
of the interpreted run, and is one method and one call site. The
`call_builtin` half is 12,535,464, 1.2%, and is a restructuring.
`interp_instructions` weighs both on the development side.

The lesson is the one this repository keeps paying for: a number bounds what it
measured. The call counts said the cost is call overhead and that much holds.
What the fix is was a guess until the call sites were read, and it was wrong
twice before they were.

## 2026-09-22 — call_builtin identifies itself inline, and the interpreted run falls another 2.42%

kanso#1563 took `eval::lookup` off `__memcmp_avx2_movbe`. This is the second
caller on that list: `Interp::call_builtin`, 12,535,464 instructions over
564,790 calls.

MEASURE FIRST, and the measurement chose a much smaller change than the one
already written down. `call_builtin` is invoked 107,625 times and makes 5.25
comparisons each -- 116 instructions per invocation spent working out which
builtin it is. A `match` on a `&str` switches on the LENGTH and then walks the
candidates of that length, and the buckets are wide:

    length  3  ->  6 candidates      length  7  ->  7
    length  4  ->  7                 length  8  ->  8
    length  5  -> 12                 length  9  ->  6
    length  6  -> 12                 length 11  ->  1

Tallying the names at run time says which buckets matter. **Twelve distinct
builtins account for every call in the corpus**, and every one of the hot ones
sits in a wide bucket: `append` 29,631 calls at length 6, `length` 11,874 at 6,
`slice` 7,700 at 5, `utf8` 5,504 at 4, `find2` 5,500 at 5, `bytes` 5,301 at 5.
`append` alone is 37% of all dispatches. 5.25 is what walking a twelve-wide
bucket costs.

THE CHANGE IS ONE LINE AND FIFTY-ONE PREFIXES. `match name` becomes
`match name.as_bytes()` and each pattern gains a `b`. Rust lowers a byte-string
pattern to a length test and inline word compares; no call leaves the function.
Nothing else moves -- the arm bodies are untouched, `name` stays in scope for
the arity errors and the `_` arm's diagnostic, and the diff is 52 lines changed
in one direction.

    interpreted row   956,538,727 -> 933,389,998   -23,148,729   -2.42%
    memcmp calls        1,707,612 ->   1,142,822      -564,790
    memcmp cost        35,413,346 ->  22,116,197   -13,297,149

The 564,790 is `call_builtin`'s own count exactly, and it is gone from the
caller list altogether. The row falls by ten million more than the frame does,
which is the bucket walk's own branches going with the calls.

THE SPEC TOOK THREE TRIES AND THE FIRST TWO PROVED NOTHING. Both are recorded
because each looked right.

The first read the patterns off `src/eval.rs` and called every one. That is
self-referential: rename `b"append"` to `b"appned"` and the spec calls
`builtin_appned`, finds it dispatches, and passes. Watched doing exactly that.

The second took its names from `lib/` instead -- a real oracle -- and passed
the same mutation for a better reason. **The programs never reached the
interpreter.** The checker gates `builtin_` names to std-origin files, so
`builtin_append` in a scratch file is refused with `is internal to the standard
library` before anything dispatches. The control says it plainest:
`builtin_nosuchthing` draws that same refusal rather than `unknown builtin`. Every
program in that spec, valid name or nonsense, produced one message that had
nothing to do with the question.

So the question moved to where it can be answered: two files that must agree.
`lib/*.kso` names 46 builtins through the `builtin_` door and `src/eval.rs`
dispatches 51, and a name in the first that is missing from the second would
answer `unknown builtin` at run time. Mangling `append` turns that red and names
the mangled entry; a `b` that lands inside the quotes turns a second assertion
red; reverting the match to `&str` does not compile at all.

CI'S SITTING, and it is a THIRD of what this box projected.

    interp_instructions     908,952,299 -> 900,471,358   -8,480,941  -0.9330%
    entry_instructions      126,691,703 -> 126,696,892      +5,189  +0.0041%
    library_instructions    127,146,502 -> 127,149,930      +3,428  +0.0027%
    compile_instructions     35,540,015 ->  35,541,148      +1,133  +0.0032%
    emit_instructions        51,481,045 ->  51,481,382        +337  +0.0007%

The container read the interpreted saving at 23,148,729 and CI reads 8,480,941:
2.73 times apart, where the same projection for kanso#1563 an hour earlier was
14% out. Both are falls and the direction is not in doubt; the size is, and the
golden is CI's.

That gap is the measurement rule this log keeps restating, at a magnitude worth
recording. What the change removes is 564,790 CALLS -- a count the program
decides, identical on any machine. What each call COSTS is the glibc the host
carries: this container's 2.39-0ubuntu8.7 against the runner's 8.9, and a
different AVX2 entry sequence behind the same name. So the count travels and the
price does not, and a projection built from the price is worth what the price
is. kanso#1563's entry predicted that shape without putting a number to it;
this is the number.

The four compile-side rows rise because byte patterns are more code.
`startup_instructions` and both codegen rows did not move at all.

Welfare 77.28677792407876 -> 77.2907927488371, banked after the goldens carried
CI's rows. Development 78.49 -> 78.51 and production is unmoved, which is right:
the interpreter is a development-side term and the benchmarks run compiled code
that never reaches this dispatch.

WHAT IS LEFT of the memcmp frame after both changes, on the container's profile:
22,116,197 instructions over 1,142,822 calls, led by `eval_global` at 332,026
and `call_named` at 165,949. Those resolve a name against the program's
declarations rather than against a fixed list, so neither takes this trick.
## 2026-09-22 — the two name-keyed maps: built, measured, declined

kanso#1563 and kanso#1564 took `eval::lookup` and `call_builtin` off
`__memcmp_avx2_movbe`. The frame's next two callers are `eval_global` at
332,026 calls and `call_named` at 165,949, and both are the same shape: a
`Map<String, _>` probed with a `&str`, which hashes the bytes and then compares
the key with `str == str` -- a memcmp call.

The obvious continuation is to key those two maps by `Name` and probe them with
a `&Name`, so the comparison is the inline word compare kanso#1563 built. The
callers already hold one. `to_string()` on the insert would go too, since
cloning an inline `Name` allocates nothing.

IT WORKS AND IT COSTS MORE THAN IT SAVES.

    memcmp calls           1,142,822 ->   644,915     -497,907
    memcmp instructions   22,116,197 -> 12,339,438   -9,776,759
    interpreted row      933,389,998 -> 937,705,542   +4,315,544

`eval_global` and `call_named` leave the caller list entirely and the 497,907
is their two lookup counts to within sixty-eight. The row still rises, and the
whole-table diff says where:

        +10,456,358  <Q as hashbrown::Equivalent<K>>::equivalent   2,984 -> 10,459,342
         -9,776,759  __memcmp_avx2_movbe
         +3,485,161  Interp::call
         +2,149,368  __memcpy_avx_unaligned_erms
         -2,103,532  Interp::eval
         -1,313,332  Interp::eval_global

A `HashMap<String, _>` probed by `&str` compares through a path that ends in a
`memcmp` call. A `HashMap<Name, _>` probed by `&Name` compares through
hashbrown's `Equivalent`, and that shim did not inline: it went from 2,984
instructions to 10,459,342, which is 21 per probe against memcmp's 19.6 plus
its call. The inline word compare is in there somewhere and never got the
chance to pay. The memcpy rise is the two cold sites that now build a `Name` --
`eval_binop`'s operator and `Value::FnRef`'s `Rc<str>` -- and `Interp::call`'s
rise is the second of those.

So the trick that worked twice does not extend to a hash-map probe, and the
reason is in hashbrown rather than in the comparison. Reverted. Keyed maps stay
`String`-keyed until somebody has a way to make `equivalent` inline, and that is
a different question from the one kanso#1563 answered.

AND ONE THING THIS PULL REQUEST MEASURED WITHOUT MEANING TO. It changes
`design/compiler-log.md` and `docs/compiler.html` and nothing else -- no
`src/`, no `lib/`, no `Cargo` -- so the compiler CI built for it is the same
source main's was, on a freshly built binary. `interp_instructions` came back
equal to main's golden to the instruction, and every other vein with it.

That is worth writing down beside STATUS.md's standing row, which has been open
since 2026-09-15 on the premise that two CI jobs of ONE COMMIT read six apart.
Tonight gave three sightings of single-digit drift and this is the fourth
reading, the only one where the compiler source did not change at all, and it
is the only one that did not drift:

    tree                                       interp_instructions   vs main
    main                                            900,471,358         --
    this branch, compiler source identical          900,471,358          0
    kanso#1504, runtime.c changed                   900,471,351         -7
    kanso#1561 first sitting, a counter added       908,952,292         -7
    kanso#1561 third sitting, same branch           900,471,344        -14

The first two rows are the ones that matter together: different binaries from
identical source, same reading. The rest changed `src/runtime.c`, which
`include_str!` puts inside the compiler, so their bytes and their layout moved.
kanso#1562 measured 112 bytes of `.text` moving this row 366,303 through
`__memcmp_avx2_movbe`, and single digits are the small end of the same thing.

It does not close the row -- one job is not the two the row describes, and the
row's own pair was on a commit nobody has re-run since. What it does is put a
control under it: when the compiler source is untouched, this row did not move.

AND THE COMMIT THAT ADDED THE PARAGRAPH ABOVE REPRODUCED kanso#1558. This
branch has now been through CI twice with byte-identical compiler source, and
`compile_instructions` read differently:

    0586890f   35,541,148   agreed with main
    74f99dba   35,541,151   +3

Three instructions, on a tree whose whole diff is `design/compiler-log.md` and
`docs/compiler.html`. kanso#1558's own header describes that shape to the word
-- 35,551,167 against 35,551,170 on a branch with exactly those two files --
and this is the first time it has been caught twice on ONE branch, which takes
the base out of the question along with the source. Every other row in the
second job agreed with main to the instruction: `entry`, `library`, `startup`,
`emit`, `interp`, both codegen rows, `compile_allocs`, `compile_peak_bytes`.
One row of twelve moves and it is always the same one.

THE INSTRUMENT IS THERE AND THIS SESSION CANNOT READ IT. kanso#1558 put an
uncapped function table on this gate for exactly this moment and kanso#1562 put
one on six more, so both jobs printed theirs. The compile table sits at step 19
of 41, and the six gates after it now print tables of their own -- five or six
thousand lines between it and the end of the log. The API this session reads
job logs through returns a tail, and a tail that deep is not on offer; the
artifact holds the raw profiles and its blob host answers
`gateway answered 403 to CONNECT` here. So the diff that would name the three
instructions is written down in two places and reachable from neither.

That is a defect in the instrument rather than in the finding, and the remedy
is small: print the table where a tail can reach it -- a final step of the job,
after the summary -- or write it to the step summary. Not done here, because
this pull request is a measurement and a CI change is a different one.

AND THE OSCILLATION MAKES THE GATE UNPASSABLE BY EITHER ANSWER, which is worth
stating plainly because it is not a thing a golden is built to survive. Leave
`bench/compile_instructions_golden.txt` at main's 35,541,148 and the
cost-goldens job fails, because CI measured 35,541,151. Set it to 35,541,151
and the TREND gate refuses: a row worsened, nothing improved, and a pure
regression is the one move it declines outright. Both are the gates working.
The golden stays at main's figure, because 35,541,148 is what this tree read
the first time and what main reads, and a coin that has come up three ways in
two tosses is not a number to pin.

What is still on the list, from the container's profile after kanso#1564:
12,339,438 instructions over 644,915 calls, led by `eval_tail`'s closure at
159,127 and `dispatch_loop` at 147,267, with `BigUint`'s own `PartialEq` at
132,846 -- that last one is num_bigint comparing digits and is not a name at
all.

## 2026-09-23 — the third reading, and two corrections to the entry above

A third CI job ran on this branch, on a head whose compiler source is again
byte-identical to the two before it, and `compile_instructions` read
**35,541,151** — the same as the second.

    0586890f   35,541,148   agreed with main
    74f99dba   35,541,151   +3
    36cf8333   35,541,151   +3, the same as the job before it

All twenty-six other veins read `success` in the summary block of all three
jobs, `compile instructions` alone failing.

THE ENTRY ABOVE CALLS THIS ROW A COIN AND THAT IS WITHDRAWN. Two readings of
three agree, and they are the two most recent; counting main's own golden the
four available readings run 148, 148, 151, 151, which is the shape of a step
rather than of a toss. What sits between the first job and the second is not in
the diff, because the source is identical across all three — it is whatever the
runner pool handed out. Four points is what that claim rests on and it is not
stretched further here. It does remove the argument that pinning 35,541,151
would pin a figure that came up once.

The comment on kanso#1565 carrying the same wording was corrected the same
hour; this entry records it because the entry above is where the claim was
written down first.

AND THE PARAGRAPH SAYING THE INSTRUMENT CANNOT BE READ FROM HERE IS TOO BROAD.
The log API saves an oversized result to a file on disk rather than refusing
it, so the whole of what it returns is reachable by shell. What is true is
narrower: it returns the last 5,000 lines and no more, whatever tail length is
asked for — a 20,020-line job hands back 5,000 — so the `interp_instructions`
table IS reachable and was read and diffed the same night, 1,726 rows from each
of two jobs, and the `compile_instructions` table at step 19 of 41 is not.
kanso#1566 packs every table into that tail, which is what makes this row's
three jobs diffable frame by frame. None of them could be read at the time.



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
all sixty-seven `.mem` files, regenerated with `all_counters.sh --write`.
`ten_handups` joins the trend gate's `higher` table in the change that mints
it, for the reason `lower_a` gives about presence counters: dropping the kernel
should read as a worsening and want its sentence.

TWO MORE PANELS THAN THIS ENTRY FIRST CLAIMED. It said the book samples do not
carry the tenure counters, on a grep of `book/`. The counter panels live in
`docs/book/samples/`, and `ch10/counters_counters.out` and
`ch12/fused_counters.out` both print the whole counter block, so both gained
`ten_handups=0` and both HTML panels were rewritten from them. CI's book-samples
job is what said so. `bench/emitted_golden.txt` really does not carry them.

THE COMPILE-SIDE ROWS, from CI, each with the value it landed on. `src/runtime.c`
is `include_str!`'d into the compiler, so its bytes are the compiler's own and
its layout moves with them; the benchmarks compile the counter out, which is why
`machine code` and every run-side row agreed. Eight of these refuse comparison
on this box, so they are CI's reading and not a projection; each was read twice
in the job and both readings were identical.

    compile_instructions             35,549,673 ->     35,552,219    +2,546
    entry_instructions              126,728,843 ->    126,735,656    +6,813
    library_instructions            127,184,941 ->    127,192,276    +7,335
    startup_instructions              3,363,186 ->      3,365,595    +2,409
    emit_instructions                51,618,058 ->     51,630,500   +12,442
    interp_instructions             923,151,727 ->    923,151,719        -8
    codegen_instructions_dev        596,158,173 ->    596,158,155       -18
    codegen_instructions_release  6,825,827,822 ->  6,825,821,967    -5,855

The five rises are the layout, and `emit_instructions` is the largest of them
because it is the only row anchored inside the compiler's own emitting rather
than around a child process. The three falls are not savings and nothing here
could have made them ones: eight on 923 million is nine parts per billion, and
the interp row is the subject of STATUS.md's standing row precisely because it
moves by single digits between jobs on one commit.

Welfare reads 77.28 against a floor of 77.27959877643865 and the floor sentinel
passes, so there is no rise to bank and no drop to explain. The five rises cost
less than the score's own resolution at these magnitudes.
## 2026-09-22 — kanso#1561 re-merged onto main after kanso#1563

kanso#1563 landed the inline name compare and took six compile-side goldens
with it. Neither side of this merge described the merged tree, so those six
carry MAIN'S values forward and CI measures the difference; this branch's own
readings on them were taken before that change existed and are not comparable
with anything after it.

Worth setting beside kanso#1502, which took the same merge on the same day and
read different numbers for five of the six. Only the interp row's +7 is shared.
So these are not a property of kanso#1520 that a branch inherits — they are
where each branch's own code lands once the compiler around it is rebuilt, and
a reader who saw one of the two sets would have been wrong to expect the other.

The floor is banked at 77.17 after these rows, not before them.

## 2026-09-18 — kanso#1504 on the merged tree, and a claim this branch withdraws

`interp_instructions` re-bases from 1,138,001,430 to **1,138,001,437**, a rise
of seven on a branch that caches the innermost beat mark and never enters the
interpreted corpus. Every other row read its golden exactly, both codegen rows
and `interp_allocs` at 2,539,998 included.

kanso#1502 read the same 1,138,001,437 on the same day, on a different runner —
Intel, family 0x6 model 0x6a, which `bench/dispatch.txt` does not record —
changing two divisions in a float renderer. Two branches with nothing in common
but their base, agreeing to the instruction, is what re-bases the row.

**This branch withdraws a claim it made earlier today.** A log entry here
described a pattern: the same seven, three times, on three different absolute
values — 1,260,262,910 to ...917 on the kanso#1520 merge, kanso#1502's reading,
and this one — and concluded it was layout jitter with three readings behind
it. That entry, and the golden edit it justified, are off this branch.

What went wrong is worth more than the retraction. The gate reported *this
binary counted two numbers in one job: 1138001437 and then 1138004452*, and the
entry was written treating that as noise to be explained. It was not noise. The
difference is 3,015, both jobs printed `interp_printed=3015` two screens above
the error, and the gate's second count was reading the raw anchored frame where
its first reading had the printed line taken off. kanso#1524 fixes the gate and
records the rest.

So the pattern was two readings and an artifact wearing the same number. Two
readings still re-base a row. They do not name a mechanism, and none is named
here beyond the standing guess that it is layout.

The lesson is the one already in CLAUDE.md, arrived at the expensive way: read
the thing the number describes before running anything against it. The gate
prints what it subtracted, as a notice, precisely so the next drift can be
answered — and the entry that went wrong was written without reading it.

## 2026-09-18 — three trees, three silicons, and what the seven is not

The row was re-based on two agreeing readings with no mechanism named. Pulling
the third job log puts real bounds on what the mechanism can be, so the guess
gets narrowed rather than left standing.

    tree            silicon            .text      .rodata   interp row
    kanso#1522 PR   AMD  0x19 / 0x1   2,840,050   803,856   1,138,001,430
    kanso#1502      Intel 0x6 / 0x6a  2,840,050   805,776   1,138,001,437
    kanso#1504      AMD  0x1a / 0x2   2,840,050   806,288   1,138,001,437

**The silicon is not it.** The two trees that agree to the instruction ran on
an Intel part and an AMD Zen 5 part, whose feature blocks differ in fifty-odd
rows — cache sizes, `rep_movsb_stop_threshold`, `isa_1`, the `xsave` sizes.
kanso#1492 built `bench/dispatch.txt` and the `differs` reader to answer
exactly this, and this is the first time it has had three jobs to answer with.
Both runs also resolved to `__memcmp_avx2_movbe`, so the resolver picked the
same implementation on both.

That matters beyond this row. The standing ruling "a welfare counter reads
three parts per billion" (2026-09-15) has sat with the resolver as its leading
suspect since it was filed. On this row the suspect has an alibi.

**`.text` is not it either, and that is the surprising one.** All three trees
emit byte-identical `.text` — 2,840,050 — and the rows still differ. Two of the
three branches change the compiler's own Rust in different places and the
compiled size lands on the same number, which is itself worth knowing; what
follows is that a row moving while `.text` holds cannot be explained by code
layout in the ordinary sense.

**`.rodata` is the only section that moves**: 803,856, 805,776, 806,288. The
smallest reads 430 and the two larger read 437. Three points, and the two
larger ones differ by 512 while reading the same row, so this is a
correspondence and not yet a function.

What this does not do is name a mechanism. It rules two out. The next reading
that would say something is a tree whose `.rodata` matches one of these three
exactly and whose row disagrees — that would rule `.rodata` out too — or a
deliberate `.rodata` change of known size on an otherwise identical tree, which
would make it a function or kill it.

## 2026-09-18 — the .rodata correspondence, tested and killed

The entry above left `.rodata` standing as the one candidate the three-tree
table had not ruled out, and named the experiment that would settle it: a
deliberate `.rodata` change of known size on an otherwise identical tree. That
experiment is cheap, it runs on this container, and it was run.

Main at `36433243`, release, callgrind, twice — once as it stands and once with
4,096 bytes of non-zero immutable data added to `src/lib.rs` under `#[used]`,
reached by nothing the program runs:

    .rodata=823,832  .text=2,860,898   row=1,173,233,661
    .rodata=827,928  .text=2,860,898   row=1,173,233,661

`.rodata` grew by exactly 4,096, `.text` held byte-for-byte, and the row did
not move by one instruction.

**So `.rodata` size does not move this row, and the correspondence was three
points lining up by chance.** 803,856 reading 430 and the two larger values
reading 437 is what two coin flips look like when you only have three of them.
The page section that recorded it as a correspondence rather than a function
was right to, and is now corrected to say it is neither.

That leaves all three candidates dead: not the silicon, not `.text` size, not
`.rodata` size. What remains is the one thing the table could not separate —
`.text` CONTENT. Equal size is not equal code, and the three trees are three
different branches; which functions landed at which addresses, and how they
aligned, differs between them while the section total happens to match. That is
layout in the narrow sense of addresses rather than sizes, and nothing here
isolates it.

The next experiment, if the seven is ever worth more than it has cost: perturb
`.text` at constant size on one tree — reorder two functions, or pad one — and
read the row. This probe took four minutes and killed a published claim, which
is the argument for running the cheap one before writing the careful sentence.

The figures above are this container's and are not comparable with CI's, which
is exactly why the experiment is sound: both readings come from the same box,
and what is compared is the difference between them.

## 2026-09-18 — kanso#1504 on the tree merged with kanso#1525: five layout rows, all down

The linearity read landed on main and this branch took it. CI's sitting on the
merged tree, against the rows main carried:

    compile_instructions     35,552,188 ->    35,549,348    -2,840   -0.008%
    entry_instructions      126,735,634 ->   126,728,937    -6,697   -0.005%
    library_instructions    127,192,177 ->   127,184,826    -7,351   -0.006%
    startup_instructions      3,365,595 ->     3,363,168    -2,427   -0.072%
    emit_instructions        51,630,538 ->    51,617,748   -12,790   -0.025%

**Every one of the five fell, and none of them is work this branch does.**
`kanso check` stops before the beat rewind runs and `kanso play` on a one-line
program never enters a beat loop, so nothing these rows count can be paying for
a cached mark pointer. What moved is the layout: this branch adds a field to
`KMark` and removes eight instructions from a runtime function, `src/runtime.c`
is compiled into the binary that the front end also lives in, and the code
landed differently around it.

That they all moved the SAME WAY is what makes the reading easy this time. When
kanso#1525 took this merge the four compile-side rows all ROSE by the same
0.19%, and the entry there was that a uniform shift is what a layout move looks
like. This is the same shape with the sign reversed and a tenth the size.

The interpreted rows did not move at all — 1,075,174,600, 2,486,376 and 833,130,
main's values to the unit. That is the check on the reading, and it is the
strongest one available: the interpreted corpus is the workload most sensitive
to `src/eval.rs`, this merge brought a large `src/eval.rs` change, and the rows
that measure it agree exactly because kanso#1525 already priced them. A layout
story that moved those too would not be a layout story.

Both codegen rows and `compile_allocs` also read their goldens exactly.

## 2026-09-18 — what the beat-top branch does to the interpreted row after kanso#1531

The interpreted row on this branch is `interp_instructions=1,029,696,282`,
where main reads 1,029,696,275. A rise of 7 instructions, 0.0000007%.

Layout, and the mechanism is the same one this branch's earlier readings
record. `kanso run --interp` never reaches the native runtime, but the binary
it runs holds `src/runtime.c` as bytes, because the compiler `include_str!`s
that file. This branch changes that file, so the code the interpreted run
walks past is arranged differently and the row moves with the arrangement
rather than with anything it measures.

Seven instructions against a row of a billion. The trend gate asked for the
sentence because the goldens carried the new value with nothing naming it.

## 2026-09-18 — kanso#1504's rows on the tree merged after kanso#1533

CI measured the beat-rewind branch on the tree carrying the dispatcher
change. Six rows moved, all of them small, and three went each way:

    compile_instructions   35,550,010 ->  35,549,668      -342  -0.0010%
    entry_instructions    126,729,588 -> 126,729,774      +186  +0.0001%
    library_instructions  127,186,008 -> 127,185,869      -139  -0.0001%
    emit_instructions      51,617,476 ->  51,620,167    +2,691  +0.0052%
    startup_instructions    3,363,916 ->   3,363,431      -485  -0.0144%
    interp_instructions   995,837,536 -> 995,837,543        +7  +0.0000007%

Four were counted twice in the one job and every repeat agreed to the
instruction: compile_again 35,549,668, entry_again 126,729,774,
library_again 127,185,869, emit_again 51,620,167.

A change that only moves layout gives no sign about direction, and this
sitting is the cleanest demonstration of that on record: one diff, six rows,
three down and three up, all under 0.015%. `interp_allocs` and
`interp_peak_bytes` are byte-identical, so nothing the interpreter counts
changed, and the seven instructions on the interpreted row are the relink.
kanso#1502 read +14 on its own tree in the same sitting and byte-identical on
another. Re-based, not explained, and the header on each golden says so.

The branch's own rows are the run-time ones and they fell: livebench
2,825,430,323 -> 2,805,024,580 and runbench 1,821,933,936 -> 1,804,998,570.

## 2026-09-18 — kanso#1504 reads the same seven instructions against a different baseline

CI measured the beat-rewind branch again, this time on the tree merged after
kanso#1534. Every row it moved before moved the same way, and the interpreted
row did something worth writing down:

    interp_instructions   975,944,763 -> 975,944,770   +7   +0.0000007%

The sitting before this one read +7 as well, against a baseline of
995,837,536. Two readings, two baselines 19,892,773 apart, the same seven
instructions. The earlier note called it "what a relink moves this row by"
and said it was re-based rather than explained; a second reading at a
different absolute value is what turns that from a guess into a small
measured fact about this branch's binary.

`interp_allocs` and `interp_peak_bytes` are byte-identical in both sittings,
so nothing the interpreter counts changed either time. The other five rows
are as recorded above: -342, +186, -139, +2,691, -485.

## 2026-09-18 — the same seven, a third time, against a third baseline

    995,837,536 -> 995,837,543
    975,944,763 -> 975,944,770
    957,583,234 -> 957,583,241

Three CI sittings of kanso#1504, each against a baseline the one before it did
not have, spanning 38,254,302 instructions between the first and the last, and
the delta is seven every time. `interp_allocs` and `interp_peak_bytes` are
byte-identical in all three.

The first note called it re-based rather than explained, which was right with
one reading. Three make it a small measured fact instead: this branch's relink
costs the interpreted row seven instructions, and the figure does not drift
with the size of the row it sits on. A layout delta that reproduces across
baselines is worth more than the same delta observed once, because once is
consistent with noise that happened to land near seven.

**AND THE RELEASE CODEGEN ROW PASSED THIS SITTING**, on the same branch whose
previous sitting halted that vein with 6,841,691,436 then 6,841,691,425. That
is what the temporary object's name being DRAWN predicts: most names give one
number and a minority give another, so an unpinned branch fails the row
intermittently. kanso#1512 measured nine of ten names at one value and
`4b8c1a` at another; this is the tenth case arriving on its own, in a job
nobody set up to look for it.

## 2026-09-18 — the same seven, a fourth time, against a fourth baseline

    995,837,536  ->  995,837,543   +7
    975,944,763  ->  975,944,770   +7
    957,583,234  ->  957,583,241   +7
    939,042,794  ->  939,042,801   +7

**The baselines span 56,794,742 instructions and the residue has not moved by
one.** `interp_allocs` reads 1,410,530 and `interp_peak_bytes` 833,466, both
byte-identical to the golden, so nothing the interpreter COUNTS changed.

Four readings on four baselines is the strongest form the relink claim has
taken. It is also the shape the release codegen draw has: a constant residue
that survives large movement in the quantity it is a residue of. The two are
different veins, different hosts within the job, and different magnitudes —
seven here, eleven there — and nothing measured connects them. Written down
beside each other because a constant residue is a narrow thing to look for and
this tree now has two.

The job also drew the release codegen vein again, which is this branch's
second and the sixth today. That one is design/pending-gavels.md's to rule on
and no work here moves it.

## 2026-09-18 — three conflict markers shipped in the published page, and what did not catch them

**A MISTAKE, caught by `no_published_page_carries_a_conflict_marker` on CI.**
kanso#1504's `docs/compiler.html` went to CI carrying `<<<<<<< HEAD`,
`>>>>>>> origin/main` and `=======`. Main never had them and no other branch
did; it was this one resolution.

**HOW.** The merge left the page unmerged with main's new section on one side
and this branch's two on the other. A script then moved main's section ahead of
this branch's and renumbered — and the block it cut, from `<h2 id="reserved">`
to `<h2 id="coda">`, spanned the closing marker. So the move carried
`>>>>>>> origin/main` with it and orphaned `=======` above the coda. Every line
of both sides survived; three lines of git punctuation came along.

**WHAT DID NOT CATCH IT, which is the part worth keeping.** The resolution was
checked, and by four things:

    git diff --diff-filter=U        no unmerged paths, because the script
                                    had rewritten the file
    comm -23 on the h2 anchors      nothing lost
    sec-num duplicates              none
    sh scripts/gates/all_pages.sh   all three gates green

The three page gates read `data-golden` spans, the log's drift budget, and
three families of sentence. A marker is none of those. The anchor and
duplicate checks read `<h2 id=` and `sec-num`, and a marker is neither. Four
checks, all passing, none of them looking at the thing that was wrong — which
is the shape this tree already has a name for: a verification that names one
file keeps passing while the defect moves next door.

The spec existed and it worked, on the push. The container check that would
have caught it before the push did not exist, and now does:
`grep -cE '^<<<<<<< |^>>>>>>> |^=======$'` over the pages is part of the
resolve-and-verify pass, beside the anchor diff.

**AND THE RULE UNDER IT.** A script that moves a region of a file must not be
run on a file that still has conflict markers in it, because the region it cuts
is defined by content and the markers are content. Resolve first, then move.

**AND THE SPEC WAS THIS SESSION'S OWN, WRITTEN THIS MORNING FOR THIS FAILURE.**
`e9d99540`, 14:37 today, merged as kanso#1526, carrying this session's id. Its
message says: "Today a merge resolution left `<<<<<<< HEAD`, `=======` and
`>>>>>>> origin/main` in docs/compiler.html and the commit went in. All three
page gates then ran on that tree and ALL THREE PASSED." Eight hours later the
same file took the same three markers in the same place, past the same three
gates plus two checks added since.

So the honest version is not that a spec caught a mistake. **The lesson was
found, written down, pinned in CI, and then repeated**, because what went into
the tree was a spec and what was needed on the container was a grep. A spec
guards the push. It does not guard the twenty minutes before the push, and that
is where the same hands make the same move again.

That is the whole argument for `verify_resolution.sh` being a script rather
than a paragraph: the paragraph existed, in a commit message, in this file, and
in the spec's own doc-comment, and it did not survive contact with a resolution
at speed. The check now runs beside the anchor diff, and it was watched red on
a planted marker before being trusted.

kanso#1526's message also names the second half, which held again today: "the
merge that leaves a marker is the same one that leaves two sections numbered
88." Two sections numbered 97 is what the duplicate check caught on this same
branch an hour earlier.

---

## 2026-09-19 — kanso#1504's release row, and what banking on a carried-forward value cost

CI's sitting on the merged tree: `codegen_instructions_release` 6,824,133,280
-> 6,841,691,425, a RISE of 17,558,145 (+0.2573%), with
`codegen_release_again` reading 6,841,691,425 in the same job. The row
reproduced.

**THE VALUE WAS ALREADY RIGHT AND THE FILE HAD TWO OF IT.** 6,841,691,425 is
exactly what the duplicated second row carried before it was deleted. What was
wrong was that it sat BESIDE main's 6,824,133,280 rather than replacing it, and
the reader adds rows with `+=`. So the branch had the correct measurement all
along, inside a file that could not be read correctly.

**AND CARRYING MAIN'S VALUE FORWARD COST A RED ROUND.** Deleting the duplicate
left a choice of which single value to keep, and main's was chosen on the
ground that neither was a reading of this tree. That reasoning holds and the
outcome still cost a round, which is worth writing down rather than defending:
when a duplicate is deleted and one of the two values came from CI on a tree
this one descends from, that value is the better carry-forward, and the header
says which sitting it was.

**THE FLOOR ENTRY ABOVE IT IS A CORRECTION, and the mistake is the interesting
part.** The floor was banked at 77.33 while the golden still held main's
codegen row, because the sentinel fails an unbanked rise and the choice at that
moment was between a red pull request and a number computed off a carried value.
It was marked PROVISIONAL in its own `why`. CI then measured the row 17,558,145
higher, the term costs 0.007 points, and the provisional floor was a hair too
high — so the tree read as a FALL against a floor derived from itself.

The rule that says bank after the goldens carry CI's rows is what this
violates, and the violation was deliberate and flagged. What it shows is that
the flag is not enough: a provisional floor makes the next reading look like a
regression, and a reader who has not got this entry would spend the round
looking for what got worse. The tree is a rise over main's 77.2665986848222 and
always was.

Both numbers now come from CI's sitting.

---

## 2026-09-19 — the relink seven, a fifth time, against a baseline it was projected onto

`interp_instructions` 932,183,914 -> **932,183,921** on kanso#1504's merged
tree. A RISE of SEVEN, and this one was written down before CI measured it.

When kanso#1540 moved the baseline under this branch, the golden was rebased to
main's 932,183,914 plus seven and the header said what that was:

> So 932,183,921 is a PROJECTION off the fifth baseline, not a reading. Four
> confirmations make it a good one and that is still not a measurement: CI
> measures this tree, and a fifth agreement is worth having on the record where
> a fifth assumption is worth nothing.

CI read 932,183,921. The five now stand as:

     995,837,536  ->    995,837,543   +7
     975,944,763  ->    975,944,770   +7
     957,583,234  ->    957,583,241   +7
     939,042,794  ->    939,042,801   +7
     932,183,914  ->    932,183,921   +7

The baselines span 63,653,622 instructions and the residue has not moved by
one. `interp_allocs` reads 1,309,483 and `interp_peak_bytes` 833,463, both
byte-identical to main, so nothing the interpreter COUNTS changed. The seven is
the relink, and a prediction that named its value in advance and was then
measured is a stronger form of that claim than four agreements found after the
fact.

**AND THE ROW IS PRICED HERE BECAUSE THE TREND GATE ASKED.** It reported
`interp_instructions` UNPRICED — worsened or re-based with no sentence in this
branch's log delta naming it and the value it landed on — which is the gate
doing exactly its job: the movement is fine and the silence was not.

`emit_instructions` 51,617,476 -> 51,620,167 and the two codegen rows moved with
the same relink; `work_runbench` 1,821,933,936 -> 1,804,998,570 and
`work_escapebench` 84,780,592 -> 75,228,606 are what this branch is for.

---

## 2026-09-19 — kanso#1504 on the tree merged after kanso#1543, and the sixth baseline

`interp_instructions` 923,151,727 -> **923,151,734**, a rise of SEVEN, and this
one is a projection again rather than a reading.

The baseline moved under this branch for the sixth time: main is now
923,151,727 after the frame-node pool. The branch's own effect on this row is
the relink seven, and the record it now has is worth stating because it decided
how this file was written:

     995,837,536  ->    995,837,543   +7
     975,944,763  ->    975,944,770   +7
     957,583,234  ->    957,583,241   +7
     939,042,794  ->    939,042,801   +7
     932,183,914  ->    932,183,921   +7    <- written down BEFORE CI read it

Five readings across baselines spanning 63,653,622 instructions, the residue
unmoved, and the fifth was a prediction this file carried in advance and the
runner then confirmed exactly. That is the strongest form the claim has taken,
and it is still a projection: CI measures this tree, and a disagreement is the
finding rather than a number to quietly correct.

The other five compile-side goldens took MAIN's rows unchanged. Those are
kanso#1543's CI measurements on a tree this one descends from, which is the
carry-forward rule this session learned the expensive way on the duplicated
codegen row: when one of two values came from CI on an ancestor, that is the
one to keep.

`interp_allocs` and `interp_peak_bytes` are main's, untouched, so nothing the
interpreter counts changed on this branch.

---

## 2026-09-19 — the relink seven broke on its sixth baseline, and it was a prediction

    interp_instructions   923,151,727 ->  923,151,726      -1
    compile_instructions   35,551,167 ->   35,549,341  -1,826
    emit_instructions      51,619,793 ->   51,617,717  -2,076
    entry_instructions    126,732,646 ->  126,729,042  -3,604
    library_instructions  127,188,882 ->  127,184,937  -3,945
    startup_instructions    3,363,672 ->    3,363,742     +70

Every row read twice in the same job and agreed with itself.

**THE SEVEN BROKE.** This branch's interpreted row had risen by exactly seven
on five consecutive baselines spanning 63,653,622 instructions. The fifth was
written into the golden as a PROJECTION before CI measured it — 932,183,921 —
and the runner read 932,183,921. That agreement was recorded here as "the
strongest form the claim has taken".

On the sixth baseline it is MINUS ONE.

**AND NOTHING ABOUT THE BRANCH CHANGED.** What changed underneath it was
kanso#1543 landing the dispatch-pooling family, which moved the interpreter's
own code. So the seven was a property of the five trees it was measured on
rather than a constant of the relink, and the residue that looked like a law
was a coincidence of layout that survived five baselines and died on the sixth.

**THE PREDICTION BEING RIGHT ONCE IS WHAT MAKES THIS WORTH WRITING DOWN.** A
projection confirmed in advance is the strongest evidence a claim of this kind
can get short of a mechanism, and this one had it. It still broke, because
nothing ever isolated WHY the relink cost seven — the entries that recorded it
were careful to say the delta arrived with the change and left the mechanism
open, and that caution is the only reason this is a correction rather than a
theory collapsing.

So: no projection replaces the figure above, and the next merge that moves this
baseline gets no prediction from this branch. Five agreements bought one wrong
answer in both size and sign.

**The other five rows are layout re-basings** of a few thousand each. The beat
cache changes src/runtime.c, which those rows carry without compiling, so what
moved is where the bytes sit.

## 2026-09-19 — kanso#1504's seventh baseline, and a bank that ran ahead of CI

The branch went red on the two codegen rows again, and for the ordinary reason:
it had banked a floor of 77.32716 while both codegen goldens still carried
main's values. Those two rows are the only ones on this branch that measure
what `src/runtime.c` costs to compile, so a bank taken before they carry CI's
reading is a bank on a projection.

CI's sitting, run 35419504606:

    codegen_instructions_dev      596,153,756 -> 596,193,835     +40,079   +0.0067%
    codegen_instructions_release  6,833,786,335 -> 6,843,462,951  +9,676,616  +0.1416%

Both reproduced in the same job — `codegen_dev_again` and
`codegen_release_again` read the same figures — which is kanso#1507's
`-Wl,-plugin-opt=jobs=1` holding on a tree that changes runtime.c.

Scored on those rows the tree reads 77.3244 against main's floor of
77.26807421236497, so the branch is 0.056 ahead of main and 0.0028 behind its
own premature bank. The floor is re-set to the measured number. The release
row costs 0.008 points at weight 0.15, and the runtime saving the beat cache
buys covers it.

The two readings this branch has taken of the release row, +9,676,616 today
and +19,241,568 on 2026-09-18, do not measure the same thing: kanso#1513's
fixed-temp pin re-based the row by about 1.35 million underneath them. The
golden's header now says so beside both numbers.


## 2026-09-19 — the same five paragraphs, kept twice on a second branch

kanso#1502 was found carrying five long paragraphs of docs/compiler.html in
duplicate, both sides of a conflict kept with a blank line between the copies.
The resolution check grew a test for it the same hour, and the next branch it
ran against — this one — had all five as well. So the defect is not one bad
resolution; it is what this repo's page conflicts do when both sides are kept,
and it had been sitting in two branches at once.

The page rendered correctly in both. What sees it is `golden_prose`, reporting
one drifted number four times instead of twice, and now the resolution check
before that.

## 2026-09-18 — the layout rows the kanso#1520 merge left on the beat-top branch

Renamed on 2026-09-22: kanso#1502 wrote an entry under the same title about
its own branch, that one reached main first, and keeping both is the point —
the two branches read DIFFERENT rows off the same merge. kanso#1520 landed under this branch — the clone sized for the growth that
follows it — and every instruction row moved with the binary it rebuilt. This
branch touches the beat reporter and nothing the front end runs, so none of the
six is work anybody did on this branch; all six are where the code landed after
another change resized the compiler around it. Written down because a number
that changes without a sentence is the thing to catch.

CI's readings on the merged tree, against the values carried forward from main:

    compile_instructions      35,486,333 ->     35,489,169     +2,836
    entry_instructions       126,498,292 ->    126,507,679     +9,387
    library_instructions     126,954,304 ->    126,963,794     +9,490
    interp_instructions    1,260,262,910 ->  1,260,262,917         +7
    startup_instructions       3,363,378 ->      3,363,577       +199
    emit_instructions         51,456,464 ->     51,456,185       -279

The interp row's +7 is the same order as the ±13 the module row has drawn
across trees whose compiler source was identical; kanso#1487 measured that one
and it is a face of the layout rather than a cost. The compile-side three are
larger and one-directional, which is what an inlining decision re-made against
a different `src/eval.rs` looks like. Both compile memory rows and both codegen
rows agreed without an edit, which is the check on that reading: allocation and
peak counts are decisions the code makes, and they did not move.

Worth setting beside kanso#1502, which took the same merge on the same day and
read different numbers for five of the six. Only the interp row's +7 is shared.
So these are not a property of kanso#1520 that a branch inherits — they are
where each branch's own code lands once the compiler around it is rebuilt, and
a reader who saw one of the two sets would have been wrong to expect the other.

The floor is banked at 77.17 after these rows, not before them.

## 2026-09-22 — kanso#1504's rows on the tree merged with kanso#1502, and the floor banked on them

kanso#1502 landed at 19:00 and this branch merged it. Neither side's goldens
described the merged tree, so the merge carried main's and CI measured the
difference. Its sitting, golden before against CI's reading:

    runbench              1,819,291,716 -> 1,802,356,350   -16,935,366   -0.93%
    encodebench           3,485,406,060 -> 3,465,000,320   -20,405,740   -0.59%
    jsonbench             1,133,644,520 -> 1,133,645,592        +1,072
    runbench .text              319,986 ->       320,546          +560
    encodebench .text           135,746 ->       136,306          +560
    jsonbench .text             122,354 ->       122,882          +528
    compile_instructions     35,549,673 ->    35,551,455        +1,782
    entry_instructions      126,728,843 ->   126,733,982        +5,139
    library_instructions    127,184,941 ->   127,190,464        +5,523
    startup_instructions      3,363,186 ->     3,363,875          +689
    emit_instructions        51,618,058 ->    51,623,314        +5,256
    codegen dev             596,158,173 ->   596,192,991       +34,818
    codegen release       6,825,827,822 -> 6,837,945,401   +12,117,579
    interp_instructions     923,151,727 ->   923,151,726            -1

CORRECTED, an hour later. The table above named three benchmarks because the
first reading of CI's diff was filtered to three names, and ALL FOURTEEN moved —
every benchmark links the runtime, so a runtime change reaches every row. The
round that followed went red on `work` and `machine code` alone, with all eight
instruction rows already agreeing, and that is what said so. Both veins carry
fourteen rows and fourteen were set this time, which is the check that would
have caught it: count the rows in the file against the rows you wrote.

    deepbench     347,644,995 -> 347,635,275        livebench  2,793,281,380
    escapebench    75,228,606                        scanbench    462,269,305
    pendbench     208,139,955                        digestbench    9,966,673
    oneshot        17,807,820                        readbench      4,627,056
    basket         32,776,834                        widebench     33,676,020
    indexbench      2,895,743

The .text rows moved with them, +480 to +560 on each. Summed, `text` reads
1,755,820 -> 1,762,748, +6,928 bytes, +0.395% — the whole of what the cached
mark costs in code, spread across every binary that links the runtime. The
earlier table in this branch named that row at 1,762,300, a figure two
landings old; the trend gate wants the value a row LANDED on and it was the
one row of the fifteen the fourteen-row correction above did not re-read.
None of the eleven is a welfare term, so the floor banked on the first pass
still stands at 77.34.

WHICH WAY AND WHY. The two falls are the branch's subject: `k_beat_iter` stops
turning a depth back into `&k_beat_stack[depth - 1]` and reads a cached mark
instead, and the run program iterates five times for every pop it makes. The
rises are the code that does it — 560 bytes of `.text` on every benchmark that
links the runtime, and the compile-side rows moving with the bytes the compiler
carries, since `src/runtime.c` is `include_str!`'d into it. The release codegen
row is the largest of them at +12,117,579, and it is clang optimising 560 more
bytes.

The interp row came back one LOWER, which is the same size as the ±13 that row
has drawn across trees whose compiler source was identical. It is not a saving
and nothing on this branch could have made it one.

Scored, the floor moves 77.27959877643865 -> 77.33517935341673, banked in this
commit and after the goldens carried CI's rows rather than before. The page's
eight drifted spans follow the goldens; four of them quote
`compile_instructions` from four distinct paragraphs, checked against the
duplicate-paragraph shape kanso#1557 recorded — no long line appears twice.
## 2026-09-22 — kanso#1504 re-merged onto main after kanso#1563

kanso#1563 landed the inline name compare at 22:08 and took six compile-side
goldens and the welfare floor with it. Neither side of this merge described the
merged tree, so the six goldens and `bench/welfare_floor.json` carry MAIN'S
values forward and CI measures the difference. Nothing this branch had measured
on those rows is comparable with anything measured after it: `interp_
instructions` alone moved 923,151,727 to 908,952,299 on that change, and the
five compile rows moved with it.

    compile_instructions    35,540,015    entry_instructions   126,691,703
    library_instructions   127,146,502    startup_instructions   3,362,329
    emit_instructions       51,481,045    interp_instructions  908,952,299

`bench/welfare_floor.json` did not conflict, so the floor this branch is scored
against is kanso#1563's 77.28677792407876.

Five page paragraphs conflicted, every one a `data-golden` span quoting those
rows, every one resolved to main's figure: a span follows its golden. Resolved
hunk by hunk rather than by taking the file whole, because §118 lives in it --
it is still there, and the log kept every entry from both sides.

The counter veins are untouched by the merge: the twelve cost goldens and all
sixty-seven `.mem` files still agree, and `ten_handups` still reads 3 on the run
program and 1, 1 and 4 on the three fixtures that see it.

CI'S SITTING ON THE MERGED TREE, each row with the value it landed on. The
branch was re-merged twice as kanso#1563 and then kanso#1564 landed, so these
are the third sitting and the figures below are the ones on disk:

    library_instructions    127,149,930 -> 127,156,898    +6,968   +0.0055%
    entry_instructions      126,696,892 -> 126,702,373    +5,481   +0.0043%
    compile_instructions     35,541,148 ->  35,543,171    +2,023   +0.0057%
    startup_instructions      3,363,118 ->   3,363,110        -8   -0.0002%
    emit_instructions        51,481,382 ->  51,481,601      +219   +0.0004%
    interp_instructions     900,471,358 -> 900,471,344       -14   -0.0000%

The five rises are the counter's own bytes. `ten_handups` adds a global, an
increment behind a predicted-not-taken test and a line of stats output, and
`src/runtime.c` is `include_str!`'d into the compiler, so the compiler carries
those bytes whether or not anything counts. `compile_allocs`, `compile_peak_
bytes`, both codegen rows and every run-side row are byte-identical.

**THE INTERP ROW DRIFTS BY SINGLE DIGITS EVERY SITTING, AND THAT IS THE
STANDING ROW AGAIN.** Against main it read seven low on the first sitting and
fourteen low on this one, and `startup_instructions` came back eight low here
too. Nothing on this branch can reach name comparison -- the only Rust that
changed is a counter declaration, an increment and a `fprintf` -- and a drift
that changes size between sittings of the same branch is the same thing
STATUS.md's
"a welfare counter reads three parts per billion" has been open on since
2026-09-15. Seven in 909 million is eight parts per billion. It is recorded here
as another sighting rather than explained: kanso#1562 measured `.text` growth
moving this row through `__memcmp_avx2_movbe`, and this branch grows `.text`,
so the direction is at least consistent with that. The floor holds at
77.28677792407876 either way and the sentinel passes, so there is no rise to
bank and nothing to defend.

    floor          77.28677792407876

Five page paragraphs conflicted, every one of them a `data-golden` span
quoting those rows, and every one resolved to main's figure for the same
reason the goldens were: a span follows its golden. Resolved hunk by hunk
rather than by taking either file whole, because this branch's own two
sections live in that file -- both are still there, and the duplicate-line
count is main's, so the shape kanso#1557 recorded did not happen here.

The run-side veins are this branch's own and survived the merge untouched;
`text` and the fourteen work rows still read what the entry above records.

CI'S SITTING ON THE MERGED TREE. Five compile-side rows moved and each rose,
which is this branch's `.text` growth arriving on top of kanso#1563's falls:

    library_instructions    127,149,930 -> 127,158,876    +8,946   +0.0070%
    entry_instructions      126,696,892 -> 126,702,408    +5,516   +0.0044%
    compile_instructions     35,540,661 ->  35,543,672    +3,011   +0.0085%
    emit_instructions        51,481,382 ->  51,484,057    +2,675   +0.0052%
    startup_instructions      3,362,329 ->   3,363,729    +1,400   +0.0416%

Taken three times as kanso#1563 and kanso#1564 landed underneath; the figures
above are the last sitting and the ones on disk. `startup_instructions` is the
one row CI reported as AGREEING this round, and it is in the table anyway: it
agreed with the golden this branch already carried, which was 1,400 above
main's. A row that agrees with its own branch has still moved against the base,
and the trend gate compares against the base -- it asked for this row by name
when the first draft of this table left it out.

`interp_instructions` read 900,471,351 against main's 900,471,358, seven low. Seven on 900
million is the drift STATUS.md's standing row is about rather than anything
this branch did: it adds a field to the mark and a global beside it,
`src/runtime.c` is `include_str!`'d into the compiler so its bytes are the
compiler's, and nothing here touches what the interpreter does. kanso#1561's
three sittings read seven low, then fourteen, on a branch that adds only a
counter, so the size of the drift moves between sittings of one tree.
Both codegen rows and `compile_allocs` are identical too.

THE FLOOR, AND THE THREE JOBS AN UNBANKED ONE REDDENS. This tree scored 77.34
against the 77.28677792407876 it inherited from kanso#1563, and a rise nobody
banks fails `the_undoctored_goldens_hold_the_floor` -- which runs in `specs`
and in `the other host` as well as behind the welfare job, so the round came
back red on three jobs with only one cause. Banked at 77.34, and the run-side
falls compose with kanso#1563's development-side falls exactly as the two sides
of the objective are meant to: production 57.13 -> 57.23, development 78.49
unmoved.
## 2026-09-23 — the allocator's page commits are inside the anchor, and are worth 107,802

STATUS.md's standing row — "a welfare counter reads three parts per billion" —
ends by naming where to look next: the frames that moved between its corpus
arms were `_mi_os_commit_ex`, `mi_bitmap_setN` and `_mi_prim_commit`, the
allocator committing pages. That lead is measured here. It does not close the
row, and what it rules out is as useful as what it finds.

SIX ARMS of the interp gate's exact run, same box, same binary, same
`env -i PATH=... GLIBC_TUNABLES=...` line, differing by at most one variable.
The row is the gate's own anchor, `run_interpreted_on_stack` inclusive. Every
arm was read twice and the two commit arms three times; every reading in this
entry repeated to the instruction.

    arm                                   the row      PROGRAM TOTALS
    nothing extra                      933,390,836        980,366,329
    MIMALLOC_EAGER_COMMIT=1            933,390,854        980,383,626
    MIMALLOC_EAGER_COMMIT=0            933,390,854        980,383,626
    MIMALLOC_EAGER_COMMIX=1            933,390,854        980,383,626
    MIMALLOC_ARENA_EAGER_COMMIT=0      933,390,854        980,384,520
    MIMALLOC_RESERVE_OS_MEMORY=256MiB  933,498,656        980,489,293

THE COMMIT PATH IS INSIDE THE ANCHOR AND IT IS LARGE. Reserving the arena up
front takes `_mi_os_commit_ex` from 5,401 to zero and `_mi_prim_commit` from
730 to zero, and drops `mi_bitmap_setN` from 30,665 to 24,179 — and the row
moves 107,802. That is 0.0115% of the row and four orders of magnitude more
than the six this row is about, so page commitment is not a small term hiding
at the bottom of the profile. It is a real part of what the gate counts.

WHAT THIS RULES OUT. Neither spelling of eager commit moves anything at all:
`MIMALLOC_EAGER_COMMIT` at 1 and at 0 give the same row, the same PROGRAM
TOTALS, and the same three commit frames, and so does `MIMALLOC_EAGER_COMMIX`,
a name of the same length that mimalloc has never heard of. B against C is zero
frames different out of 1,396. The commit frames move for one knob in six, and
that knob is the one that stops the commits happening.

AND THE ALLOCATOR'S OPTION READING IS OUTSIDE THE ANCHOR, which corrects the
shape of this row's own candidate. `MIMALLOC_VERBOSE=1` moves 107 frames and
217,814 instructions of PROGRAM TOTALS — `_mi_vsnprintf` +87,496,
`mi_buffered_out` +19,234 — and leaves the row byte-identical at 933,390,854.
So mimalloc IS reading its environment, and every instruction it spends doing
so falls outside `run_interpreted_on_stack`. The 117-per-variable term this row
recorded on 2026-09-19 is a fact about the process, not about the number the
gate pins. One extra variable costs this row 18.

NOTHING VARIED RUN TO RUN. Every arm is byte-identical across its repeats, so
this box still cannot reproduce the six, and the control the row already has
stands.

WHAT IS LEFT, and it is a question rather than a measurement. Page commitment
has the shape the row has been looking for: it is page-granular, so it moves in
lumps rather than smoothly, and how many pages a run commits depends on where
the allocator's heap starts. `MIMALLOC_RESERVE_OS_MEMORY` would put it into a
persistent known initial state, which is the 2026-09-15 ruling's own phrasing.
Against that: committing pages is work the program really does, so reserving
them up front normalizes by changing the subject, and the row would stop
counting 107,802 instructions the production allocator spends. Which of those
the ruling means is not cloud's to decide alone, and it belongs in the ledger
rather than in this entry.

Measured against a release build of the tree at `354e24d4`, using
`scripts/gates/function_table.sh` for the per-frame readings.
## 2026-09-23 — the tables were printed and unreadable, so they are packed into the tail

kanso#1558 and kanso#1562 put an uncapped whole function table on seven gates,
for the standing row that moves by three instructions on byte-identical source.
The reasoning was that naming which frames carry a delta takes both jobs' whole
tables, which it does. What neither change checked is whether a table that is
printed can be READ.

It mostly cannot. A log API returns the TAIL of a job and caps it: asking for
60,000 lines of the 20,020-line `cost goldens` job returned exactly the last
5,000, and those held two function tables of the twenty the job prints. The
other eighteen are written down in a place nothing reaches. The artifact beside
the profiles is not an answer either — its blob host answers `gateway answered
403 to CONNECT` under some egress policies, which is why the tables went into
the log in the first place.

So each table is now also emitted PACKED, in a step that runs last:
gzip+base64 at 200 columns turns 1,726 rows into 189 lines, and twenty tables
fit in the tail together with room for the steps after them. The step carries
`if: always()`, because the run that most wants the tables is the one where a
row went red. `scripts/gates/function_table.sh` annotates and stashes one
profile — the seven gates now call it instead of each carrying a copy of the
pipeline — and `scripts/gates/function_tables_tail.sh` packs whatever was
stashed, prints the recipe for reading a block back, and reports its own line
count against a budget rather than leaving an overflow to be discovered by a
reader whose tail starts halfway through a block.

Each `#table` line carries the sha256 of the DECODED table, so a short tail is
distinguishable from a whole one, and the index of table names is printed last
and unpacked, so a reader who got only the final handful of lines still learns
which tables the job carried.

TWO THINGS THIS COST, both from the same habit of trusting a check's shape
rather than running it.

`fold` ends its last chunk WITHOUT a newline, so `cat "$body"; echo "#end
$name"` put the end marker on the tail of the final base64 line. A reader's
`sed` range then runs to the end of the log instead of to the end of the block.
The spec written for the round trip did not catch it: its fixture folded on a
200-byte boundary, where the defect does not appear. It was found by running
the pair against a real callgrind profile, which is the entry a reader actually
uses.

And the round trip passed anyway, on the broken format, for a second reason.
The recipe is a pipeline ending in `gunzip`, `gunzip` is content with rubbish
after a complete stream, and a pipeline's status is its last stage's. `base64`
printed `invalid input` while the recovered table came out byte-identical. The
spec now reads stderr as well as the status, and a separate spec asserts each
marker is on a line of its own at five table sizes, because the defect hides at
whichever size happens to fold evenly.

That is the same rule this log keeps paying for: never take a verdict from the
last stage of a pipe, and break what a new check watches before trusting it.

## 2026-09-23 — the reserve arm moves frame_for, not the commits, and the entry above says otherwise

The entry above is headed "the allocator's page commits are inside the anchor,
and are worth 107,802". The second half is wrong, and this entry carries the
correction and what the number actually is.

WHAT WAS DONE WRONG. The reserve arm's commit frames were read — two to zero,
the third down 6,486 — and the row was read, 107,802 higher. The two were put
together without diffing the arm frame by frame. That is a
difference-in-differences presented as a mechanism, which is the failure this
log has a rule about, committed inside the entry that cites the rule.

WHAT THE FRAMES SAY. The matched pair is `MIMALLOC_EAGER_COMMIT=1` against
`MIMALLOC_RESERVE_OS_MEMORY=256MiB` — one variable each, so the cost of reading
one more environment variable cancels. 84 frames move.

    sum of falls   -15,794          sum of rises  +121,461

    what falls                            what rises
      -6,486  mi_bitmap_setN               +111,696  kanso::eval::Interp::frame_for
      -5,401  _mi_os_commit_ex               +2,808  _mi_os_reuse
      -1,144  _mi_subproc                    +2,237  __vfscanf_internal      [libc]
        -730  _mi_prim_commit                +1,105  ____strtoul_l_internal  [libc]
        -438  mprotect                [libc]   +367  mi_bchunk_xsetNC
        -432  mi_arena_try_alloc_at            +216  _mi_prim_reuse

The commit path inside the anchor is worth about **15,794**. The row rises
because `kanso::eval::Interp::frame_for` costs 111,696 more when the arena is
reserved up front, and that frame is the interpreter's own. Two further passes
of both arms read 933,390,854 and 933,498,656 on the row and 12,341,583 and
12,453,279 on `frame_for`, to the instruction each time.

THE DIRECTION WAS MISLEADING TOO. "Worth 107,802" reads as a saving. Reserving
makes the row LARGER by 107,802 and the whole process larger by 105,667: it
removes the commits and costs more than they were.

AND THE CORRECTED FINDING IS THE STRONGER ONE. A knob that changes nothing but
where the heap starts moves kanso's own hot interpreter function by 111,696
instructions, reproducibly. The standing row has wanted a mechanism in the
compiler's own code since 2026-09-15; the frame diff of the interp tables
earlier the same night could offer only an address relocation, flagged there as
an attribution rather than a mechanism. This is in a named function.

What it does NOT show is that CI's three or six comes from heap placement. A
256MiB reservation is a wholesale intervention, not the difference between two
runners, and nothing on this box varies run to run. What it shows is that this
row is layout-sensitive in kanso's own code, which is the thing the row has been
trying to establish.

WHAT STANDS from the entry above, unchanged: the commit frames are inside the
anchor; neither spelling of eager commit moves anything, zero frames different
out of 1,396; the allocator's option reading falls outside the anchor while
`MIMALLOC_VERBOSE=1` moves 217,814 of PROGRAM TOTALS and leaves the row
byte-identical; and nothing varied run to run in any arm.

Surfaces the wrong attribution reached: this log (corrected here, the log being
append-only), kanso#1567's title and body, and its merge commit message, which
cannot be edited. A comment on kanso#1567 carries the same correction.

## 2026-09-23 — what a runtime.c change does to the six rows, decomposed exactly

kanso#1566 put every function table inside the log tail, and kanso#1561 is the
first branch with a compiler change to run under it. Its whole Rust-visible
diff is a counter declaration, an increment and an fprintf in `src/runtime.c` —
which `include_str!` puts inside the compiler, so the compiler's own bytes move.
Diffing its six tables against main's, frame by frame, joining on the first
space:

    table      TOTALS Δ  =  constant     memcmp    memchr    rest
    compile        -246  =    -2,235    +2,023       -34      +0
    entry        +3,212  =    -2,235    +5,481       -34      +0
    library      +4,699  =    -2,235    +6,968       -34      +0
    startup      -1,488  =    -2,235      +781       -34      +0
    emit           -628  =    -2,235    +1,637       -30      +0
    interp         +361  =    -2,235    +2,626       -34      +4

Three terms account for every row exactly. NOT ONE NAMED KANSO FRAME MOVED in
any of the six.

**The constant is 29 frames that move by the SAME amount in all six tables**,
summing to -2,235 whatever the workload — 36.9 million instructions for the
compile row against 946.9 million for interp. Workload-proportional work cannot
do that; a fixed cost paid once per process can. The named frames say what it
is:

    -1,180  __vfscanf_internal            -38  __isoc23_sscanf
      -582  ____strtoul_l_internal        -29  _IO_setb
       -88  _IO_sputbackc                 -24  pthread_getattr_np
       -73  getdelim                      -24  _IO_no_init
       -40  _IO_str_init_static_internal   -4  getline

`getdelim`, `sscanf`, `strtoul` and the `_IO_*` family are what reading a text
file line by line is made of, and `pthread_getattr_np` moves with them. That
frame is the one CLAUDE.md's 2026-09-15 normalization ruling names by hand, and
what it parses is `/proc/self/maps`. So the constant is glibc reading this
process's own memory map once at thread set-up, and what it reads differs
because the binary's layout differs.

That inference is as far as a FLAT table goes, and it should be said plainly: a
flat profile has no caller edges, so this is the signature of the maps parse
rather than a demonstration that `pthread_getattr_np` called those frames. The
raw profile's `cfn=`/`calls=` pairs would settle it.

**The varying term is `__memcmp_avx2_movbe`**, +781 on the startup row to +6,968
on library. It scales with the workload, which is what a `.text`-layout effect
does, and it is the frame kanso#1562 identified as the carrier.

WHAT THIS GIVES THE STANDING ROW. Its question since 2026-09-15 has been what
carries a single-digit drift on trees that cannot reach the compiler's
decisions. The answer here is two terms, both in libc, and one of them is
already ruled to need normalizing. The maps parse is CONSTANT per binary pair
and rides on every row at once, so it moves rows a change cannot otherwise
touch; and -2,235 is three orders of magnitude bigger than the three
instructions kanso#1565 is standing down on, so a smaller layout difference
producing a smaller constant is the shape to look for next.

WHAT IT IS NOT. One pair of jobs on two different binaries. The -2,235 is a
property of that pair, not a constant of the project, and nothing here shows
CI's three comes from this term. The next reading that would bear on it is the
same decomposition across two jobs whose binaries are identical, which is now
one command per job rather than unavailable.

## 2026-09-23 — the compile row leaves the walk out, and the two faces were 487 and 490

`compile_instructions` is measured with `<std::fs::ReadDir as Iterator>::next`
inclusive subtracted, the way it already subtracts `std::io::stdio::_print` and
for the same reason. CI's first sitting with the exclusion in:

    compile_instructions   35,541,148 ->  35,540,661     -487   -0.0014%

`compile_again` reads 35,540,661 too, so the gate's two readings agree on the
excluded row as they did on the unexcluded one.

THE ARITHMETIC CLOSES THE STORY. This row drew two faces, 35,541,148 and
35,541,151. The excluded reading is 487 below the first. So the walk cost 487 on
the runner that read the low face and 490 on the one that read the high face,
and 490 is exactly what the walk measures on the container this session runs in.
The two faces were never two compilers; they were one compiler and a directory
walk that costs three more on some filesystems than others.

WHAT IT COST AND WHAT IT BOUGHT. 487 instructions of 35.5 million, 0.0014%,
which is the size of the term being excluded rather than a change in the
compiler. What it buys is a row that can be pinned exactly again, which is what
kanso#1504, kanso#1565 and kanso#1568 have each been unable to do.

THE FALSIFIER IS IN THE GOLDEN'S HEADER, and it is the next thing to check: the
row should now read ONE value where it drew two, because the faces differed only
inside the excluded subtree. A second sitting that alternates means the
exclusion is aimed at the wrong frame.

## 2026-09-23 — kanso#1561's compile row on the excluded gate, and the frame that IS the two faces

kanso#1570 is on main, so `compile_instructions` no longer counts the directory
walk. This branch's golden follows, derived rather than waited for:

    compile_instructions   35,540,661 ->  35,542,684    +2,023   +0.0057%

THE DERIVATION, and it rests on a relation checked at both ends. The walk's
inclusive cost is 363 plus the self cost of
`<std::sys::fs::unix::ReadDir as Iterator>::next`, the frame that carries the
drift. Two independent points fix that: a job whose frame read 124 had a walk of
487, because main went 35,541,148 -> 35,540,661 under the exclusion; and this
container, whose frame reads 127, measures the walk at 490. This branch's job
read 124, so its walk was 487 and 35,543,171 - 487 = 35,542,684.

THE FRAME'S SELF COST IS THE TWO FACES, across five jobs and three trees:

    job                        frame self   row
    kanso#1566 job 2                  124   35,541,148   (low)
    kanso#1568                        127   35,541,151   (high)
    kanso#1561                        124   35,543,171
    kanso#1504 job 3                  124   35,544,159
    kanso#1504 job 4                  127   35,544,162

124 or 127, three apart, tracking the face every time. Nothing else in the
compile table moved between any of those pairs.

TWO CHECKS THAT DID NOT HAVE TO AGREE AND DO.

The delta against main is +2,023 on the excluded gate and was +2,023 on the
unexcluded one, because both ends dropped by their own walk. An exclusion that
removed the right term has to leave every difference between two trees exactly
as it was.

And +2,023 is the `__memcmp_avx2_movbe` delta measured between this branch and
main in the six-row decomposition earlier today. The gated row moved by exactly
the memcmp term and by nothing else, which places the constant that entry
attributed to the `/proc/self/maps` parse OUTSIDE `kanso::main`'s inclusive
anchor — where a thread set-up cost belongs. That was not predicted; it falls
out of two measurements taken for different reasons.

IF CI DISAGREES the derivation is wrong and its number is the one to take.


## 2026-09-23 — kanso#1504's compile row moved three on a merge that changed no code

Merging main in twice (kanso#1567 and kanso#1566, a log entry and a CI change,
neither reaching anything `include_str!` puts in the compiler) moved
`compile_instructions` from 35,544,159 to **35,544,162**.

The golden was set to 35,544,162 on the strength of that, and the NEXT job read
35,544,159 again. The entry after this one carries what the two jobs' tables say
and why the golden is back at 35,544,159; what stands here is the pair of facts
that sighting established.

This is the kanso#1558 phenomenon for the sixth time and the third branch, and
two things about this sighting are worth keeping.

**The gate's own second reading agrees with its first.** `compile_again` reads
35,544,162 in the same job. So a job is internally consistent and the three
appear BETWEEN jobs, which is what kanso#1565's three readings said and this
confirms on a different tree.

**The other five instruction rows match their goldens to the instruction** —
`entry` 126,702,408, `library` 127,158,876, `startup` 3,363,729, `emit`
51,484,057, `interp` 900,471,351, every one exact.

That second fact rules something out, and the entry above about the six-row
decomposition is why. A runtime.c change moves all six rows through two terms:
a CONSTANT, the same in every table, which is glibc parsing `/proc/self/maps`
at thread set-up; and `__memcmp_avx2_movbe`, which scales with the workload.
The constant moves every row by the same amount. Five rows here did not move at
all, so **the three cannot be the constant term**. Whatever carries it touches
the compile workload and nothing else.

What it is remains open. The reading that would name it is this job's packed
compile table against one from a job of this same tree that read 35,544,159 —
and that earlier job predates kanso#1566, so its table is at step 19 of 41 and
out of reach. The next occurrence has both sides.

## 2026-09-23 — the three instructions are ReadDir::next, and the row is a directory walk

The row that has drifted by single digits since 2026-09-15 has a named frame.

kanso#1504's tree went through CI three times with identical compiler source and
`compile_instructions` read **35,544,159**, then **35,544,162**, then
**35,544,159** again. Not a step; it alternates. Both of the last two jobs ran
under kanso#1566, so both packed their whole function tables into the log tail,
and the two compile tables can be diffed against each other for the first time.

**1,335 of 1,336 frames are byte-identical. One moved.**

    -3     127 -> 124   <std::sys::fs::unix::ReadDir as Iterator>::next

PROGRAM TOTALS moved -3. The gated row moved -3. The single frame moved -3. The
three agree exactly, and the other five tables — entry, library, startup, emit,
interp — are byte-identical between the two jobs, zero frames moved in any of
them.

**WHY THIS ROW AND NO OTHER.** `kanso check` routes a single argument by what it
finds: a DIRECTORY is a module, a file of bare statements is an entry, a file of
definitions alone is a library. The compile row checks `lib/json`, a directory,
so its route opens one and walks it — `opendir` 51, `__getdents` 18, `readdir`
171, `DirEntry::path` 8 are all in its table. The entry and library tables have
no `ReadDir` frame AT ALL, because a single file is never walked. The emit and
interp tables do have one, and it reads 124 in BOTH jobs: they walk a directory
that did not move.

So the carrier is directory iteration, and what a directory iteration costs is
the filesystem's answer rather than the program's. The entries are the same
entries; what readdir hands back them in — the packing of the dirent buffer,
the order, the name lengths it walks — is state the code under test did not
produce. That is what the 2026-09-15 ruling is about, in the words Clay used:
clear it out so it is identical every run, or put it into a persistent known
initial state.

WHAT IS NOT ESTABLISHED. Why the walk costs three more in one job than another
is open; the entry set is fixed and staged by `library_box.sh`, so the
difference is in how the filesystem lays those entries out, and this is one pair
of jobs. Nothing here says the `interp_instructions` half of the standing row
has the same carrier — interp's own `ReadDir` frame did not move, and its drift
has been measured at seven and fourteen rather than three.

WHAT IT COSTS TODAY. This is the gate's exactness meeting a counter with
external state under it. The golden goes back to 35,544,159, which two of the
three jobs read including the most recent; the next job may read 35,544,162 and
turn this pull request red again through nothing it did. A normalization that
staged the corpus so the walk is identical every run would end it, and that is
its own change.

## 2026-09-23 — kanso#1504's compile row on the excluded gate, predicted before it was measured

kanso#1570 is on main, so `compile_instructions` no longer counts the directory
walk. This branch's golden follows, and the value was DERIVED rather than waited
for.

This row drew two faces on this tree, 35,544,159 and 35,544,162, and the walk
cost 487 on the runner that read the first and 490 on the one that read the
second. Subtracting each face's own walk gives the same number both ways:

    35,544,159 - 487 = 35,543,672
    35,544,162 - 490 = 35,543,672

So the golden is 35,543,672, and the delta against main's excluded base of
35,540,661 is +3,011 — identical to the +3,011 this branch priced against main's
unexcluded 35,541,148, because both ends dropped by their own walk. The priced
line above is restated on the new base and its delta is unchanged.

That agreement is the check on the arithmetic rather than a coincidence: an
exclusion that removed the right term has to leave every difference between two
trees exactly as it was, and it does.

IF CI DISAGREES, the prediction is wrong and the number it reports is the one to
take. Writing it down first is what makes that worth knowing.

## 2026-09-23 — kanso#1561 on main after kanso#1504: two runtime.c changes, measured by CI

kanso#1504 landed underneath this branch, and both change `src/runtime.c`, which
`include_str!` puts inside the compiler. The runtime merged without a conflict,
both changes are present — `k_beat_top` from kanso#1504 and `k_stat_ten_handups`
from this branch — and the tree builds.

THE DERIVED GOLDEN ABOVE DOES NOT SURVIVE THIS. 35,542,684 was this branch's
excluded compile row on main as it stood before kanso#1504. The tree now carries
both changes, a combination no job has measured, and layout effects do not add:
this branch's +2,023 against old main was entirely `__memcmp_avx2_movbe`, which
is a `.text`-layout term, and two layout moves do not compose into the sum of
their deltas. So it is not derived this time.

Eight golden files conflicted — compile, entry, library, startup, emit, interp
and both codegen tiers — each as this branch's reading on old main against
kanso#1504's. Neither describes the combined tree and this host cannot measure
it, different rustc. Each takes main's value as a PLACEHOLDER, and this branch
is expected to go red once on `cost goldens` so that CI can report the combined
rows, which then replace the placeholders and are priced here.

The `ten_handups` counter rows in the cost goldens are this branch's own and
came through the merge intact.


## 2026-09-23 — kanso#1561's combined rows, as CI read them

The placeholders above came back from CI red as expected, and every row was
read twice on one runner and agreed with itself. Against main at 18fae808:

    row                            main           this tree      delta
    compile_instructions           35,543,672     35,540,661     -3,011
    entry_instructions            126,702,408    126,696,892     -5,516
    library_instructions          127,158,876    127,149,930     -8,946
    startup_instructions            3,363,729      3,362,329     -1,400
    emit_instructions              51,484,057     51,481,382     -2,675
    codegen_instructions_release 6,837,945,401  6,837,938,796     -6,605
    codegen_instructions_dev       596,192,991    596,192,991          0
    interp_instructions           900,471,351    900,471,358         +7

`compile_instructions` lands on 35,540,661, which is main's excluded value from
before kanso#1504 to the instruction. kanso#1504 moved the row by +3,011 and
this tree, carrying both changes, moves it back by the same amount. That is
where the combined layout put it; nothing here says why the two moves cancel.

The six falls are `.text`-layout moves from a runtime.c edit that `include_str!`
carries into the compiler. This branch changes no compiler Rust, and a counter
increment behind a `K_COUNTING` test is not work any of these rows performs.

`interp_instructions` rises by 7 to 900,471,358, and the rise belongs to this
tree. Twenty-eight saved cost-goldens jobs from the last two days read the row
at 900,471,344, 900,471,351 or 900,471,358, and the value follows the tree and
nothing else: every job whose `startup_instructions` read 3,362,329 read 358,
on four CPU models (family 0x6 model 0xcf, 0x19/0x1, 0x19/0x11, 0x1a/0x2), and
every job at 3,363,729 read 351. The compile row draws two faces per tree from
the directory walk; this row draws one. So the steps of 7 are moves between
trees, and there is no runner drift here to hide one in.

The packed self-cost tables of a 351 job (kanso#1504) and this tree's 358 job
differ in two libc functions: `__memcmp_avx2_movbe` by -2,836 and
`__memcpy_avx_unaligned_erms` by +42. Every other differing row is a
function whose address moved and whose cost did not. Most of the memcmp term
sits outside the anchor, since the row moved by 7 and not by thousands, and
self cost cannot say which few instructions fall inside `run_interpreted_on_stack`.

This host does not reproduce it. Run by hand with the gate's own command and
tunables, main and this tree both read 933,390,005, with the memcmp term moving
+3,857 and the row not at all. This container's rustc is 1.94.1 against CI's
1.98.1, so the binaries lay out differently and this reading says nothing
about CI's. The mechanism stays open. It is a layout term
of the kind the 2026-09-15 ruling covers, priced here at the value it landed
on.

A CORRECTION to the entry above, "kanso#1561's compile row on the excluded gate,
and the frame that IS the two faces", which landed on main with kanso#1504. It
says interp's drift "has been measured at seven and fourteen rather than three".
The only disagreement within one commit that the standing row records is six,
2,178,502,266 against 2,178,502,272, on 2026-09-15. Seven and fourteen are
moves between trees. The same twenty-eight jobs bear on that row: no tree among
them read two interp values, on a row now near 900 million where the six was
read near 2,178 million, so the six has not recurred in them. That is
twenty-eight jobs, and it does not close the row.

## 2026-09-23 — memcmp chose its path by page offset, and the counted runs now preload one that does not

Two findings, both on STATUS.md's row "A welfare counter reads three parts per
billion", and a normalization under the 2026-09-15 ruling that the second one
calls for.

### The interp half: one value per tree in 132 jobs

A survey of 132 cost-goldens jobs from 2026-09-17 to 2026-09-23, keyed on the
commit CI built (the merge commit for a pull request) and on the tree hashes of
`src`, `lib` and `bench/interp_corpus`, found 36 trees read by two or more jobs.
Grouped by tree and by the `scripts/gates` tree together, every one of 37 groups
read exactly one `interp_instructions` value, on CPU families 0x6, 0x19 and 0x1a
alike. `startup_instructions` and `entry_instructions` are single-valued the
same way. The only splits inside a tree follow a gate change: two trees read two
values each, and in both the lower value came only from gates tree 275581a8 and
the higher only from 59d917a7, 3,199 apart, which is the printed-line term the
second one subtracts. Eight jobs whose `interp_again` differed from their first
reading differ by exactly that job's `interp_printed`, in the gate versions
whose second pass did not yet subtract it.

The six the row records, 2,178,502,266 against 2,178,502,272, was read on
kanso#1492's job on 2026-09-17, before kanso#1505 took the printed line off the
interp row that evening. The printed term itself varies between jobs on one
tree: 3,186 on one job and 3,199 on another, both on tree ffb98cc870. So a gate
that still counted printing could draw two values on one commit, and the six is
the size of move that term makes. That is consistent with the six and does not
isolate it, because those two jobs predate the notice that prints the term.
What stands is that since the term came off, no tree has read two values.

### The compile rows: libc's memcmp reads the address

kanso#1561 adds forty lines to `src/runtime.c`, which the compiler embeds with
`include_str!`, and moved every compile-side row by a few thousand
instructions. Profiled on this container for main and for kanso#1561, the
interp process's `__memcmp_avx2_movbe` differed by +3,857 over identical call
counts: `<Name as PartialEq<str>>::eq` made 1,129 calls in both and cost 2,698
more, `check::builtin_arity` made 5,887 in both and cost 1,081 more.
Line-level counts, against glibc 2.39's `memcmp-avx2-movbe.S`, place it: 478
calls left the no-page-cross path at lines 414 to 423 and took
`L(page_cross_less_vec)` at 429 onward. The test at 407 to 411 is

    movl  %edi, %eax
    orl   %esi, %eax
    andl  $(PAGE_SIZE - 1), %eax
    cmpl  $(PAGE_SIZE - VEC_SIZE), %eax
    jg    L(page_cross_less_vec)

which asks where the operands sit and nothing about what they hold. Forty lines
of runtime moved strings in `.rodata` across page offsets, and the same
comparisons took the other branch.

THE ISOLATION. A replacement `memcmp` and `bcmp` whose cost depends only on
the length and on where the first difference falls, preloaded into
`kanso check compile_corpus` with the gate's own tunables, on both trees:

    tree        libc's memcmp        preloaded
    main          35,990,100        36,656,738
    kanso#1561    35,992,788        36,656,738

The 2,688 between the trees is exactly the memcmp delta, and with the preload
the two processes' PROGRAM TOTALS read 37,292,002 each. The only rows that
still differ are functions whose address moved and whose cost did not. A byte
loop showed the same thing at 37,803,013 on both, 5% above libc's figure,
which is why the version that shipped compares eight bytes at a time.

THE NORMALIZATION. The 2026-09-15 ruling names "a layout the linker chose" as
state a counter must not read. So compile, entry, library, startup, interp and
emit now run `scripts/gates/address_blind.sh`, which builds
`scripts/gates/address_blind/compare.c`, compares the same sixteen bytes at page
offset 64 and at 4080 under callgrind counting only the comparing frame, refuses
unless the two counts agree, and prints the library's path. Every `env -i` line
in those six gates preloads it. Without the preload the self-test reads 26 and
32 and refuses; with it, 35 and 35.
`tests/every_counted_kanso_run_compares_blind.rs` derives the gates from disk,
every script that runs `./kanso` under callgrind, and asserts each resolves the
library before its first run and preloads it on every `env -i` line. It was
watched red twice: with the preload dropped from one line of the library gate,
and with the interp gate's call to the helper replaced.

WHAT IT COSTS AND HOW IT IS PRICED. The replacement reads about 1.9% more than
libc's fast path on the compile row, because avx2 compares 32 bytes in an
instruction. The six goldens hold main's values as placeholders, this branch is
expected red once on cost goldens, and CI's rows replace them. Every golden's
header says its value is not comparable with a reading taken before today. The
welfare terms these rows feed are re-based by the same ratio, as the compile
term's baseline was re-based by 465,864 when the row began counting the
compiler's own frame. This is a measurement change, and it should neither
score as the compiler getting slower nor let a later change bank the difference.

WHAT IS NOT COVERED. `codegen_instructions` counts clang and ld, whose work
genuinely changes with the runtime they compile; whether they also read
addresses is a separate measurement. The benchmark rows in `instructions.sh`
count programs the compiler emits, which call libc's memcmp from `runtime.c`
and presumably have the same exposure; nothing here measures that. The
compiler page's section "two terms under every row", carried by kanso#1568,
tabulates kanso#1561's first CI reading, and one of its columns is `memchr`
moving by 30 to 34 in every row. glibc's `memchr-avx2.S` carries the same
page-offset test at its lines 77 and 78. On this container, once memcmp is
preloaded, no libc function differs between the two trees, so that term has
not been reproduced here.

CI'S ROWS, on main's tree at 18fae808 with the preload, read by one job on
family 0x6 model 0x6a. Each gate read twice and the two readings agreed:

    row                     libc's memcmp    preloaded        delta
    compile_instructions     35,543,672     36,200,554      +656,882
    entry_instructions      126,702,408    129,110,493    +2,408,085
    library_instructions    127,158,876    129,559,007    +2,400,131
    startup_instructions      3,363,729      3,367,191        +3,462
    interp_instructions     900,471,351    920,710,206   +20,238,855
    emit_instructions        51,484,057     52,190,331      +706,274

Only the six preloaded gates disagreed with their goldens. The replacement's
own frames appear in every table, so the preload took. The rises run from
0.10% on startup to 2.25% on interp; why interp pays the most is not measured
here.

The welfare terms are re-based after kanso#1561 lands under this branch, not
before. On this container the preload made kanso#1561 and main read the same
row. If CI agrees, kanso#1561's deltas vanish under the preload, and a re-base
priced against main's rows now would leave the merged tree under the floor
kanso#1561 banks. Re-basing after the merge prices the switch against the rows
that floor was banked on. That is also the test of the prediction: the rows
above should not move when kanso#1561 comes in underneath.

THE RE-BASE, done on the merge with kanso#1561. Four baselines in
`bench/welfare_floor.json` are scaled by the preloaded row over kanso#1561's
row, each rounded up so that no ratio falls: compile_instructions (the compile
and entry rows together) 671,773,822 to 684,500,178, startup_instructions
4,838,372 to 4,845,369, interp_instructions 2,178,559,085 to 2,227,524,026,
and emit_instructions 382,212,543 to 387,475,984. The score reads the floor
kanso#1561 banked, 77.3466, and the history entry says this is a re-basing.

THE PREDICTION, TESTED, AND HALF OF IT FAILED. With kanso#1561 merged
underneath, CI read interp, startup and emit exactly as before: 920,710,206,
3,367,191 and 52,190,331, on family 0x19 model 0x1 where the first reading was
on family 0x6 model 0x6a. Compile, entry and library moved by +44, +122 and
+122, to 36,200,598, 129,110,615 and 129,559,129. In each of those three tables
the whole move is one function, `__memcpy_avx_unaligned_erms`, and the other
rows that differ are functions whose address moved and whose cost did not.
On this container memcpy did not move between the two trees at all, so the
extra is not bytes copied, which would show on any host.

A probe says why. glibc's memmove, which serves memcpy, chooses part of its
large-copy path by the distance between destination and source. The same 1,500
bytes cost 177 instructions at a distance of 4096 and 179 at 5000, and 2,000
bytes cost 225 and 227 at two other distances.

So `scripts/gates/address_blind/copy.c` replaces memcpy and memmove as well,
choosing every branch by the length alone. Up to 128 bytes it loads a head and
a tail that may overlap and stores both, the way libc handles short copies.
Above that it loads the far end first, moves 128 bytes a step with unaligned
AVX2 loads and stores, and stores the far end last. Everything a step writes
was loaded before the step writes it, and memmove walks backward when the
destination starts inside the source. The 1,500-byte probe reads 151 at both
distances, against libc's 177 and 179.

It took three tries. Moving 32 bytes a step through words read 477 on that
probe. A version finishing with single bytes went to CI on 77297b88 and read
startup 3,486,298 and emit 53,551,685, because it cost 1.9 times libc's memcpy on start-up's short copies,
which would have weighed copying above the rest of each row. With the size
classes, this container's compile row reads 36,335,519 with both preloads, 1%
above libc's 35,990,100, and the preloaded memcpy costs 477,178 instructions
there against libc's 799,618.

`scripts/gates/address_blind/check.c` now runs before either probe. It compares
all four replacements with libc on 80,000 random cases and on every length to
300 at every distance from -140 to 140, 164,882 in all, because a wrong memmove would corrupt the compiler under
measurement rather than show up as a wrong count. It was watched red with
memmove made to copy forward always: `memmove disagrees with libc: n=685
src=1130 dst=1496`. It was watched red again with
the forward move re-reading its tail after the loop instead of before:
`memmove disagrees with libc: n=947 src=633 dst=615`. A second probe,
`copy_probe.c`, refuses unless the two distances cost the same.

The six goldens keep the memcmp-only values as placeholders, and this branch is
expected red once more on cost goldens. When CI reads the rows, the four
welfare baselines above are scaled again by the new row over the golden they
were re-based against, so the score stays at kanso#1561's floor.

THE ROWS THIS BRANCH LANDS, read by CI on f4b0ce05 (family 0x19 model 0x11)
with memcmp, bcmp, memcpy and memmove all preloaded, each gate read twice with
the two readings agreeing. Against kanso#1561's rows under libc:

    row                     libc (kanso#1561)   preloaded      change
    compile_instructions       35,540,661      35,876,811      +0.95%
    entry_instructions        126,696,892     127,696,380      +0.79%
    library_instructions      127,149,930     128,229,577      +0.85%
    startup_instructions        3,362,329       3,320,132      -1.25%
    interp_instructions       900,471,358     921,740,873      +2.36%
    emit_instructions          51,481,382      51,184,463      -0.58%

The re-base above is carried forward once more, from the memcmp-only rows to
these, with the same rule: each baseline scaled by the new row over the old
and rounded up. The final baselines are compile_instructions 677,304,273,
startup_instructions 4,777,652, interp_instructions 2,230,017,575 and
emit_instructions 380,008,132, against 671,773,822, 4,838,372, 2,178,559,085
and 382,212,543 before this branch. The score reads 77.3466 before and after,
and the history carries both steps as re-basings.

## 2026-09-23 — the benchmark rows do not read where the runtime's data sits

kanso#1571 left the benchmark rows in `instructions.sh` on its list of things
not covered: the programs they count call libc's memcmp and memcpy from
`runtime.c`, and the compile rows had moved when strings in the compiler's
`.rodata` crossed page offsets. This measures whether the benchmark rows can
move the same way.

An unused array, kept with `__attribute__((used))`, was added to `runtime.c` at
three sizes, and `bench/runbench` was built each time with `.text`
byte-identical and `.rodata` grown by 1,008, 2,912 and 4,128 bytes. Counted the
way `instructions.sh` counts it, from one fixed directory under `env -i`:

    .rodata      row
    43,528   1,820,479,435
    44,536   1,820,479,435
    46,440   1,820,479,435
    47,656   1,820,479,435

Four layouts, one number. The compiler's exposure came from comparing names
held in `.rodata` against names on the heap. The runtime compares and copies
values in its own arena, whose addresses do not follow the binary's data
layout, so the same libc functions read the same paths here. This is one
container and one benchmark, and a change to `.text` was not tried. It is
enough to take the item off kanso#1571's list without adding a preload to
programs that do not need one.

The first attempt at this reported a move of 14, and it was the run directory.
The two binaries ran from `rb-base` and `rb-pad1000`, and the whole 14 was
`strcspn` in the dynamic loader, which scans the executable's path. That is the
reason `tests/every_counted_run_sits_at_one_fixed_path.rs` exists, met again
from the other side. Run from one directory, the difference was zero.

## 2026-09-23 — the preloaded compare costs what libc's does

kanso#1571's memcmp and bcmp compared eight bytes a step and finished byte by
byte. Their cost depended on nothing but the length and the first difference,
which is what they were for, but they cost too much. On the interpreted run,
`eval_global`, `call_named`, `eval_tail` and `dispatch_loop` together make
about 780,000 equality comparisons, and each cost 44 to 48 instructions against
about 15 on libc's AVX2 path. `compare` came to 42,011,663 instructions, 4.19%
of the run. A row that weighs a name comparison at three times its cost
overstates what removing one is worth, and removing them from `eval_global`
is the next lead that profile offers.

So `compare.c` takes the shape `copy.c` took. A comparison under 32 bytes loads
a head and a tail that may overlap, 16, 8, 4 or 2 bytes each, and orders them
as big-endian integers, which is correct because the overlapping bytes agree
whenever the heads do. From 32 bytes up it steps 32 bytes at a time with an
AVX2 compare and a movemask, then checks the last 32 bytes, overlapping what
the loop already found equal. Every branch depends on the length or on the
bytes, never on where they sit, and the helper's page-offset probe still reads
one count at both offsets.

On this container, with memcpy and memmove preloaded as before:

    row                 libc          previous compare    this compare
    interp       933,390,005 (a)        955,713,307        939,398,706
    compile       35,990,100 (a)         36,335,519         36,125,644

(a) read before kanso#1561 landed, so it is the scale of the gap rather than an
exact baseline. `compare` on the interpreted run falls from 42,011,663 to
25,396,950.

`check.c` adds every length to 100, equal and then with one byte raised and one
lowered at every position: 175,083 cases in all agree with libc. It was
watched red with the 16-to-31-byte class skipping its tail:
`compare disagrees with libc at n=24`.

The six goldens hold kanso#1571's rows as placeholders and this branch is
expected red once on cost goldens. When CI reads the rows, the four welfare
baselines are scaled by the new row over the old, rounded up, as before.

CI'S ROWS on fb997da5 (family 0x19 model 0x1), each gate read twice and
agreeing, against kanso#1561's rows under libc:

    row                     libc (kanso#1561)   preloaded      change
    compile_instructions       35,540,661      35,671,647      +0.37%
    entry_instructions        126,696,892     126,996,739      +0.24%
    library_instructions      127,149,930     127,520,399      +0.29%
    startup_instructions        3,362,329       3,372,848      +0.31%
    interp_instructions       900,471,358     905,979,540      +0.61%
    emit_instructions          51,481,382      51,172,461      -0.60%

kanso#1571 landed at +0.79% to +2.36% on the same rows. The re-base is carried
once more, from kanso#1571's rows to these, each baseline scaled by the new row
over the old and rounded up: compile_instructions 673,557,765,
startup_instructions 4,853,511, interp_instructions 2,191,885,330,
emit_instructions 379,919,026. The score reads 77.3466 before and after.

A NEGATIVE RESULT, from the profile this was meant to clear the way for. On the
interpreted run `eval_global` is 57.0M instructions inclusive, 5.78%, and
`eval::lookup` is 72.8M self. Much of both is a global name walking the whole
environment chain, missing, and then going to a string-keyed map. Caching
`eval_global` by the address of the `Name` it was asked about is unsound. Making
`eval` take `&'a Expr`, which the cache needs to be safe, gives ten lifetime
errors, and two of them (`c` and `expr` "does not live long enough") show the
evaluator running expressions built at run time, whose addresses can be reused.
A map keyed by `Name` was built in kanso#1565 and declined. What would work is
a static scope pass that marks each identifier local or global, and it would
have to follow the lexical binding forms exactly, because the checker lets a
local shadow a bare-enrolled import. That is a large change to the reference
interpreter, and it is left as a lead.

## 2026-09-23 — what a scope pass would save the interpreted run, counted

The negative result in the entry above leaves one route to the global-name
cost: a static pass that marks each identifier local or global, so that a
global never walks the environment chain and never reaches the string-keyed
map. Before building it, the ceiling. A counter in `eval::lookup` for one run
of the interpreted corpus, in a scratch tree that was not kept:

    lookups that found a local      724,304   frames walked  1,590,733   2.2 each
    lookups that missed             332,025   frames walked  1,071,803   3.2 each

Every miss is a global. The profile counts 332,026 calls into `eval_global`
where the counter saw 332,025 misses; the one apart is not explained here. Misses are 31% of lookups and
40% of the frames walked, because a miss walks the whole chain where a hit
stops early. `eval::lookup` is 72,789,828 instructions self on this run, so
the misses are roughly 29 million of it if steps cost alike, and
`eval_global` is 57,004,207 inclusive, most of which is the map. Together
that is of the order of 80 million of the 939,398,706 this container counts
for the interpreted row, about 8%, and it is the largest single lead the
profile offers.

What the pass has to get right is the reason it is not built in this entry.
The checker lets a local shadow a bare-enrolled import, so a name's being a
global somewhere does not make every use of it global; the pass has to follow
every binding form the evaluator has, lexically, and the interpreter is the
oracle every other engine is held to.

## 2026-09-23 — every identifier resolved the same way each time it ran, in two corpora

The scope pass in the entry above needs one property to be true before it can
be as cheap as it wants to be: that whether an identifier finds a local does not
depend on which execution of it is running. If that holds, the answer can be
learned on a node's first execution and kept in the node, with no pass over
every binding form and no address-keyed table. The checker cannot simply be
asked instead: it resolves the program it checks, and the evaluator also runs
expressions it cloned, into a lazy bind's thunk among other places (`eval.rs`
at the lazy bind site takes `expr.clone()`), which is where kanso#1573's ten
lifetime errors came from. A verdict stored in the node travels with the clone.

Tested, not argued. `eval_ident` was made to record, per node keyed by span and
name, how many executions found a local and how many missed, in a scratch tree
that was not kept:

    interpreted corpus         388 nodes   253 always local   135 always global   0 both
    400 golden programs        826 nodes                                          0 both

No node did both. Keying by span can only merge nodes, never split one, so a
collision would have shown as "both" rather than hidden one. The reading is
consistent with the code read so far: the lazy bind captures its environment
before it binds the name, which is the order the checker resolves in. The
evaluator extends an environment at about ten places (`bind` and `bind_all`
callers, `match_one`'s pattern binds, a closure's parameter, a synthesized
`entry.bind_name`), and not all of them have been read against this property;
two corpora are evidence, not a proof.

What building it costs, so the next session prices it before starting.
`Expr::Ident` appears 278 times across 16 files. A third field changes the size
of `Expr`, which moves every compile-side row, and the flag must be an
`AtomicU8` or similar if the AST is shared across the interpreter's stack
thread. The ceiling it buys is the entry above's, about 8% of the interpreted
row.

## 2026-09-23 — an identifier remembers whether it found a local

Built on the property the entry above tested. `Expr::Ident` gains a third field,
a `Resolution` holding 0 until the node first runs, then 1 if it found a local
and 2 if it did not. `eval_ident_at` sends a node kept as global straight to
`eval_global`, so it no longer walks the environment. On the interpreted
corpus that walk had missed 332,025 times across 1,071,803 frames. A node kept
as local still walks, because it has to find which frame holds it.

The field is an `AtomicU8` read and written with `Relaxed`, because an `Expr`
can be reached from the interpreter's own stack thread. It fits in the enum's
existing size: `size_of::<Expr>()` is 56 before and after, measured.
`Expr::Ident` appeared 278 times across 16 files. The patterns took a `, _`
and the constructors a `Resolution::default()`, applied from the compiler's
own error spans, and rustfmt settled the result. A clone of a node copies what
it knows. The evaluator's own clones go into thunks and closures, and each
carries the environment it was defined in, so it runs where the node sits.
The inliner does move nodes to new places, but it runs at load
(`inline_builtin_wrappers` in `src/lib.rs`), before anything is evaluated and
while every flag is still 0. These were read, not proven exhaustive; the check
below is what would catch a path that was missed.

THE CHECK THAT KEEPS IT HONEST. In a debug build every execution of a kept node
checks the kept answer: a node kept as global that finds a local, or kept as
local that misses, stops the run and names the identifier and its place. Every
spec runs in a debug build, so the golden, differential and micro suites check
the property on every program they carry. The whole suite passes with it on,
591 tests, the eleven `wasm_engine` failures being the usual local absence of
`docs/kanso.wasm`. It was watched red: with a first hit recorded as global
instead of local, the golden suite stopped on `v` at line 11, column 12, "was
kept as global and found a local".

WHAT IT BUYS, on this container with the address-blind preload:

    row                     main           this branch
    interp_instructions   939,398,706     919,550,926    -19,847,780  (-2.11%)
    compile_instructions   36,125,644      36,018,620       -107,024

The interpreted row falls by about a quarter of the ceiling the scope-pass
entry put at 80 million, because only the walk is gone: a global still pays
`eval_global`'s string-keyed map on every use. The compile row's fall is not
isolated here; the compiler's own code changed at 278 places, and CI's rows
will say whether it holds. Keeping `eval_global`'s answer on the node as well
would take a slot index that means something only to one interpreter, and an
AST can outlive the interpreter that ran it, so that is left alone.

CI'S ROWS on 6a44d698 (family 0x19 model 0x11), each gate read twice and
agreeing, against main at 4381c2a9:

    row                     main           this branch      change
    compile_instructions    35,671,647     35,374,375      -297,272  (-0.83%)
    entry_instructions     126,996,739    126,091,396      -905,343  (-0.71%)
    library_instructions   127,520,399    126,613,848      -906,551  (-0.71%)
    startup_instructions     3,372,848      3,372,380          -468
    interp_instructions    905,979,540    887,079,102   -18,900,438  (-2.09%)
    emit_instructions       51,172,461     51,381,691      +209,230  (+0.41%)

The interpreted row falls by what this container projected, within a tenth of a
point. The three compile rows fall by 0.7 to 0.8% and emit rises by 0.4%, and
nothing here isolates either. `kanso check` does evaluate: constants are
knotted through the interpreter, so some of the compile-side fall may be the
same skipped walk, and some may be the layout of a compiler that changed at
278 places. Emit runs the code generator, which never evaluates, so its rise
is the second kind until something shows otherwise. The allocation, memory
and codegen rows did not move.

## 2026-09-23 — a global keeps its resolution in a slot its node is stamped with

kanso#1576 took the environment walk off a global. What it left is the
string-keyed map: every use of a global still hashed its name and compared it
in `eval_global`. Keeping the answer on the node was set aside in that entry,
because a slot means something to one interpreter and an AST can outlive the
interpreter that ran it.

A generation stamp answers that. `Resolution` widens to an `AtomicU32`. Values
from 65,536 up hold a sixteen-bit generation and a sixteen-bit slot. Each
`Interp` takes its generation from a process-wide counter and keeps a table of
the globals it has resolved. A node stamped by this interpreter reads its slot.
A node stamped by any other is read as plain global, resolved by name, and
stamped again, so a stale slot is never read. A node that cannot be stamped,
because the generations ran out or the table is full, resolves by name each
time and takes no slot, so the table cannot grow per execution. `Expr` stays
at 56 bytes, measured again.

The debug check now covers the slot as well: a node that reads a slot holding
something other than what its name resolves to stops the run. The whole suite
passes with it on, 591 tests, the eleven `wasm_engine` failures being the
local absence of `docs/kanso.wasm`. It was watched red with each slot read
from its neighbour: "`push` at line 11, column 4 read slot 5 and it is not what
the name resolves to".

On this container with the address-blind preload, the interpreted row reads
879,893,558 against kanso#1576's 919,550,926, a further 39,657,368 (4.3%). The
compile row reads 36,018,837 against 36,018,620. Together with kanso#1576 that
is 59.5 million off the 939.4 million this container counted before either,
about three quarters of the 80 million the scope-pass entry put as the
ceiling.

**CI's rows**, from the cost-goldens job of the first push, taken into the
goldens:

    interp_instructions     887,079,102 -> 853,048,810   -34,030,292   -3.84%
    interp_peak_bytes           834,079 ->     846,367       +12,288   +1.47%
    interp_allocs             1,063,795 ->   1,063,803            +8
    compile_instructions     35,374,375 ->  35,400,616       +26,241   +0.07%
    entry_instructions      126,091,396 -> 126,100,824        +9,428
    library_instructions    126,613,848 -> 126,623,258        +9,410
    startup_instructions      3,372,380 ->   3,372,366           -14
    emit_instructions        51,381,691 ->  51,381,691     unchanged

The memory is the slot table: 12,288 bytes is a doubled `Vec` of resolved
globals, and the eight allocations are its growth. Interpreter speed carries
0.11 of the development side and interpreter memory 0.04, and the objective
scores the trade at 77.37 against a floor of 77.36, so the rise is banked.

---

## 2026-09-23 — the tailcc pass reads code lines only, and the preamble is indexed when the compiler compiles

Two text passes at the end of `Backend::emit` ran over every line of the
module, and the module of a one-line program is 1,155 lines, 1,186 of them
DECLARES before pruning. Between them they were most of what `print "x"` cost
to build.

**The bug.** `narrow_tailcc` drops `tailcc` from every function a `musttail`
does not reach. It read string constants as well as code, and a string
constant is one line opening with `@` whose bytes are the program's. So
`print "call tailcc here"` lost the word from its constant and kept the
declared length, and clang refused the module:

    error: constant expression type mismatch: got type '[9 x i8]' but expected '[16 x i8]'

The interpreter printed the line. That is a divergence between engines, and
`tests/golden/micro/a_string_may_spell_what_the_ir_spells.kso` pins it with
three strings spelling `call tailcc`, `musttail call tailcc @f(` and
`define tailcc`. Watched red on main, where `micro_corpus_agrees_across_engines`
reported that the sample answers differently as a library. It is green here,
and green in the release-built corpus.

**The first fix.** Lines opening with `@` are passed through, and a line
without `tailcc ` is copied before `symbol_of` allocates its callee's name.
Every rewrite the pass makes needs that word, so the second half changes only
the cost. Nothing the emitter writes on a global line carries `tailcc`.

**The second.** DECLARES is a constant, and every build split it into lines,
found each declare's symbol, asked every line whether it opened a stats gate,
and allocated a `String` per kept line to join afterwards. `index_declares` now
does that scan in const evaluation, and `declares_for` walks the finished table
into one buffer. The refusals the run-time fold made about a malformed gate are
const assertions, so a ninth gate or a misshapen one fails the compiler's own
build; changing `STATS_GATE_SITES` to 9 was watched fail with `evaluation
panicked: the stats gate moved`. The old scan is kept verbatim as the oracle in
`the_declares_table_is_the_scan_it_replaced`, which compares the two over five
cuts of which declares survive, counted and shipped. Breaking the fold's label
by one space turned it red.

DECLARES holds no `tailcc`, and a third const assertion says so, so `emit` now
hands `narrow_tailcc` only what follows the preamble.

**What moved.** `runbench.ll` is byte-identical, and so is the start-up
program's module in both a counted and a shipped build. On this container, with
the gates' own commands, the address-blind preload, and `GITHUB_ACTIONS=1` so
the host gate measures instead of refusing:

    startup_instructions   3,404,805 ->   979,543   -2,425,262   -71.2%
    emit_instructions     51,896,570 -> 46,378,063   -5,518,507   -10.6%

The first fix alone read 2,698,935 and 48,118,120. The container runs rustc
1.94.1 and the goldens were measured on 1.98.1, so CI's rows go into the
goldens. No other row reads either pass: `kanso check` stops before codegen,
and the codegen rows count clang, which is handed the same bytes.

The whole process is now 1,603,771 instructions, of which the row's
`kanso::main` is 979,543 and the dynamic loader 396,850. Inside the row,
`emit_ir` is 645,986, so emitting is still two thirds of what the row counts.

**CI's rows**, taken into the goldens; every other row read what main has:

    startup_instructions    3,372,366 ->    968,441   -2,403,925   -71.28%
    emit_instructions      51,381,691 -> 45,953,348   -5,428,343   -10.56%

The first reading was 968,455, taken before main was merged in; the merged
tree reads 14 fewer, which is kanso#1577's own start-up move.

Welfare scores 78.15 against a floor of 77.37, and the rise is banked.

---

## 2026-09-23 — the convention probe is asked once per clang, not once per build

Both codegen rows count a `kanso build`'s whole child tree, and one of those
children was not codegen. `closure_convention` asks whether this host's clang
takes `preserve_none` by compiling a two-define module, and it asked on every
build: a whole clang run, 32,201,483 instructions, 6.4% of the dev row, to
learn what the build before it had learned.

The answer belongs to the clang binary. `remembered_probe` keeps it in the temp
directory beside the runtime objects, keyed by the clang the PATH resolves to,
followed through its links, with the file's size and modification time, so
installing another clang asks again. A key that cannot be formed, or a file
holding anything but the one byte written there, means asking. The answer is
staged under a pid-tagged name and renamed into place, so a build running
beside the first reads the whole byte or nothing. The gates warm both tiers
before counting, as they already do for the runtime object, so the counted
build reads the cache.

`tests/the_convention_probe_runs_once_per_clang.rs` puts a clang in front of the
real one that logs every command, builds one program twice against one temp
directory, and counts the probe's compiles: one. Watched red with the build
asking `preserve_none_probe` directly again: two.

On this container, with the gates' commands and `GITHUB_ACTIONS=1`:

    codegen_instructions_dev        596,013,703 ->   563,812,220   -5.40%
    codegen_instructions_release  6,843,200,439 -> 6,810,998,956   -0.47%
    processes                     five -> four, each tier

The short-tree guard in `codegen_instructions.sh` fails a build that ran fewer
than four processes, which still holds; its message said a real build runs five
and now says four, with the probe a fifth only before its answer is cached.

**CI's rows**, taken into the goldens:

    codegen_instructions_dev        596,192,991 ->   563,926,696   -5.41%
    codegen_instructions_release  6,837,938,796 -> 6,805,672,501   -0.47%
    startup_instructions                968,441 ->       967,869      -572

Both codegen rows fell by 32,266,295, the same figure on each tier, which is one
probe's compile. Start-up falls 572 instructions. The emitted, `.text`, emit and
compile rows read what main has. The rise is banked.

---

## 2026-09-23 — sha256 carries its eight words as arguments, and its loops rewind

`docs/compiler.html` §114–115 put 3,145,728 bytes of the run program's peak in
the digest phase and traced it to `sha256/blocked` and `sha256/digested` losing
their carry to the library path prefix. Admitting a one-slot carry was built
there and declined, because the sizing walk read a garbage length under it.
This change makes the carry unnecessary instead of admitting it.

Both loops held the eight working words as a list. `compress` built a new
eight-element list every round and `digested` a new one every block, so a
list crossed every rewind, and a list built inside the loop is a value the
cluster analysis will only let cross by carrying it. The words are eight
arguments now. A round passes six along unchanged and computes two with `&
whole`, which infers as INT, so every slot is a threaded parameter or a
scalar, and both clusters bracket with nothing carried:

    sha256/compress/10, sha256/rounding/11   bracketed with its cluster
    sha256/blocked/10, sha256/digested/11    bracketed with its cluster

The answer becomes a list once per block, when the sixty-four rounds are done,
and `digested` adds it into the running words. `turned`, `added`, `summed`,
`summing`, `shifted_state` and `start` are gone. The five `kanso test lib/sha256` tests
pass, and the run program prints `runbench 46013475` either way.

On this container:

    digestbench   allocs          23,582 ->     7,199
                  alloc_bytes  1,998,801 ->   671,841
                  arena peak   2,097,152 -> 1,048,576   2 blocks -> 1
    run program   arena peak  38,604,496 -> 35,458,768
                  instructions 1,820,479,421 -> 1,812,623,924   -0.43%

The instruction fall is the per-round list going: no allocation, no length
checks on `s[5]`, no copying six words into a new list. Welfare scores 77.78
against a floor of 77.37 on the peak alone, with the instruction rows as main
has them; CI's rows go in before the rise is banked.

Three counters read worse, and each is the change working. Rewinds are what
the loops do now, so `run_beat_iters` rises to 2,709,016, `digest_beat_iters`
to 8,441 and `a_digest_holds_every_block_it_walked_beat_iters` to 76. The
per-block sum built its list with eight pushes, and that list is gone, so
`run_push_mut_fast` falls to 1,098,392, `digest_push_mut_fast` to 10,184 and
`a_digest_holds_every_block_it_walked_push_mut_fast` to 120. With fewer lists
built there are fewer buffers to hand on, so `run_buf_reuse` reads 144,961,
`digest_buf_reuse` 1 and `a_digest_holds_every_block_it_walked_buf_reuse` 1.
Every allocation and byte counter beside them fell.

`tests/golden/mem/a_digest_holds_every_block_it_walked.kso` carried a header
describing the 2026-08-31 state, with numbers its own golden had not held for
weeks (1,980 allocations against a golden of 397). It says what the digest
does now, and its golden reads 270.

**CI's rows**, taken into the goldens:

    runbench              1,802,356,350 -> 1,794,573,732   -7,782,618   -0.43%
    digestbench               9,966,673 ->     5,773,783   -4,192,890   -42.07%
    entry_instructions      126,100,824 ->   125,949,337     -151,487
    library_instructions    126,623,258 ->   126,452,016     -171,242
    runbench .text              320,546 ->       319,218
    digestbench .text           108,274 ->       105,634

The emitted vein reads four fewer defines for each of the two programs and,
for runbench, 16 more calls with 22 fewer branches; the compile, start-up,
emit, interpreted and codegen rows read what main has. Welfare scores 77.81
against a floor of 77.37, and the rise is banked.

Section 115's garbage length in the carry-sizing walk is untouched by this
and still open: it is reachable only when a library loop is admitted to the
carry tier, which nothing now needs for this peak.

Two specs pinned the old digest, and both went red on CI as their own notes
said they would.

- `tests/sha256_peak.rs` pinned a peak that grew with the message and said a
  streaming hash would read the same number at its two sizes. It now does:
  7,340,064 at 65,536 bytes and at 131,072. A third size, 262,144, reads
  24,117,296, and the rest of the growth is the padding. A copy of
  `padded_bytes` made public and called alone, with no compression, reads the
  same three numbers byte for byte, because it copies the message to append the
  terminator and the length. The spec pins all three sizes and says so.
- `tests/a_program_is_not_its_directory.rs` used a copy of the digest to show
  the library path prefix changing a program's memory. The digest carries
  nothing across a rewind now, so both directories read 1,048,576 and the copy
  stopped showing the defect, which is still there. The package is now a
  five-line loop with the shape the digest had: it builds a list it drops and
  hands the next turn a list it keeps. Under `lib/` it reads 5,242,880 at
  20,000 turns. In any other directory it reads one block.

---

## 2026-09-23 — ThinLTO for the release build, declined at a third of what it costs

The release codegen row is 75% the linker. On this container, with the gate's
own commands and `GITHUB_ACTIONS=1` so it measures, the codegen corpus's
release build counted:

    kanso 81,494,353   clang:probe 32,201,483   clang 31,669,971
    clang -cc1 1,617,890,071   ld 5,161,438,914   total 6,843,200,439

`ld` is the LTO link: it optimizes and emits the runtime together with the
program, every build. `-flto=thin` for both the runtime object and the link,
the one change:

    codegen release    6,843,200,439 -> 6,605,682,161   -3.47%
    run program        1,812,623,924 -> 1,867,477,968   +3.03%

At today's ratios a per cent of the release codegen row is worth about 0.023
of welfare and a per cent of the run row about 0.078, so the trade is +0.08
against -0.24. ThinLTO imports less across modules than full LTO, and the run
program is built on the runtime's small helpers being inlined into it. The
container's clang is 18.1.3 where CI's is 19.1.1, so the sizes here are this
host's; the ratio between them is the finding.

---

## 2026-09-23 — lambdas nothing reaches are pruned, and a trampoline is written only for a call that uses it

At `-O0` clang compiles every function a module defines. The codegen corpus's
module defined 112, and 48 of them were named nowhere else in it. Most of
those were `internal alwaysinline` helpers from DECLARES, which the always-
inliner drops on its own. What clang was compiling for nothing was the rest:
seventeen lifted lambda bodies, `klam0` to `klam16`, and three trampolines,
`"d_list/advance_6.c"`, `"d_list/bounded_flat_5.c"` and
`"d_list/bounded_more_5.c"`.

`prune_unnamed` walks the module from the entry and strikes what nothing names,
but it only ever considered `d_` and `w_` symbols. A lambda body is named by its
wrapper or by the closure built over it; one inside a library function the
program never reaches is named by neither, and it survived every prune.
`klam` is a candidate now, in the prune and in the oracle beside it. The
trampolines come from `narrow_tailcc`, which wrote one for every function
that keeps `tailcc` and spills past eight registers, whether or not any call
was rerouted through it. It writes one now only for a function a rerouted
call names.

Two specs, each watched red: `a_lambda_body_goes_with_the_wrapper_that_named_it`
fails with "the lambda nothing reaches was kept" when `klam` is taken out of
the prune's candidates, and `none_when_nothing_is_rerouted` fails when every
trampoline is written again. Its first draft checked for a quoted name the
emitter never writes and passed against the old code too; the name is `f.c`.

On this container, with the gates' commands and `GITHUB_ACTIONS=1`:

    codegen_instructions_dev   596,013,703 -> 505,923,202   -15.12%
      clang -cc1               446,229,133 -> 357,391,531
    emit_instructions           51,896,570 ->  50,057,227    -3.54%
    codegen corpus module         6,107 lines -> 5,299, 112 defines -> 83
    runbench.ll                  35,860 lines -> 34,853, 595 defines -> 528

The run program reads 1,820,479,435 against 1,820,479,421 and its `.text` is
the same size: LTO was already dropping these at link time, so the release
binary does not change, and the release codegen row should move only by what
`clang -cc1` saves parsing them. CI's rows go into the goldens.

**CI's rows**, taken into the goldens:

    codegen_instructions_dev       596,192,991 ->   506,101,048   -15.11%
    codegen_instructions_release 6,837,938,796 -> 6,617,211,630    -3.23%
    emit_instructions               45,953,348 ->    44,610,460    -2.92%
    startup_instructions               968,441 ->       973,054    +4,613

The release row fell further than parsing alone would suggest: 220,727,166
instructions, where the dev row's `clang -cc1` saving on this container was
88,837,602. Which half of the release tree took it, the compile or the LTO link,
was not measured.

`startup_instructions` is the one row that rose. It reads 973,054, up 0.48%. It
counts kanso's own start, not anything the compiler emits, so it arrived with
the new code in the compiler binary. What in that code costs 4,613 instructions
before `main` does any work was not isolated. The run program's instruction and
`.text` rows read what main has.

The emitted vein falls on twelve of its fourteen programs; escapebench and
indexbench read what main has. runbench reads 528 defines
against 599, and deepbench 86 against 115. Welfare rises 0.16, and the rise is
banked.

`specs` then failed on `compile_cost`, whose modules vein counts what the
compiler emitted for a module: 5,377 lines and 99 defines before, 4,569 and 70
after. Regenerated. With kanso#1583 merged from main, the codegen and start-up
goldens hold a projection, each tier's row less the probe compile that change
removed, and are replaced by CI's rows before the floor is banked again. The
start-up projection is 972,482, this change's 4,613 on top of main's 967,869.
CI read all three projected rows exactly, and the rise is banked over them.

kanso#1580 then landed on main, and the digest and run programs now carry
both changes. Their emitted rows, counted here from the `.ll` files, read
`digestbench defines=155 calls=1210 branches=819 lines=9012` and `runbench
defines=523 calls=5749 branches=3445 lines=34623`. The `.text`, codegen and
emit rows come from CI, and the floor is banked again after them.
CI's cost goldens agreed with every merged row, and the floor is banked over
them.

---

## 2026-09-23 — the regexp scan rewinds at every start position, and a rewind keeps the seek cursor below the mark

`docs/compiler.html` §112–115 put 29,360,128 of the run program's 38,604,496
arena bytes in the split phase and priced reclaiming them at +4.75 welfare,
then went after it through the carry tier and found that tier closed to library
loops by a path prefix. The loop holding the memory needs no carry at all.

**Where the memory was.** Each phase count taken to its floor on its own, peak
arena bytes of the counting run program:

    baseline       38,604,496   36 blocks
    encode = 1     38,604,496   decode = 1 the same
    index = 1      35,651,584
    digest = 1     35,458,768
    split = 1       9,244,368    8 blocks

The split phase is `regexp/find_all` over a subject no match can be found in.
Its per-position loop is `scanned` -> `scanning` -> `skipping` -> `landed` ->
`scanned`, a tail cycle. Instrumenting `eligible_clusters` showed the cycle
passing the entry, value-use and allocation tests and then refused inside
`cluster_edges_ok`:

    refuse regexp/scanning/4 slot 2 set 0x7fdf

Slot 2 is the position. Its set is everything but `DONE`, because the callers
that resume a scan read their position out of a hit's field, `m.to` or
`m.from + 1`, and `Expr::Field` infers as TOP. TOP includes `BYTES`, and the analysis refuses
a slot that might be a byte builder, since evacuating one copies its buffer at
every rewind. With the position fixed, the walk's answer was the next refusal:
`landed` took it as its fourth argument, so a record crossed each position.

**The change is in `lib/regexp` only.** The walk's answer is a local in
`skipping`, which returns the hit or tail-calls onward, and `landed` is gone.
`scanned` enters a new loop head, `probing`, once with `at | 0`; `|` infers
as INT, and every edge inside the cycle passes `at + 1`, which infers as INT or
FLOAT. The cluster is now `probing`/`scanning`/`skipping`, it carries nothing,
and it brackets:

    scanbench peak     161,480,704 -> 1,048,576    154 blocks -> 1
    run arena peak      38,604,496 -> 9,244,368     36 blocks -> 8
    run output         runbench 46013475 both ways

`tests/golden/mem/a_scan_that_finds_nothing_keeps_nothing.kso` pins the shape
at 156 characters: one block and 157 rewinds here, four blocks and none on
main. `a_class_asks_by_the_byte`, whose pattern does match, moves one
allocation and 80 bytes and rewinds 1,601 times; its output is unchanged.

A second route was tried first and is not in the tree: typing a dot read as
the union of that field's construction sets. It narrows nothing here, because
the positions reach the records through the matcher's continuation lambdas,
whose parameters infer as TOP. It also turned up that a type with fields used
as a function value widens none of its field sets, which constructor patterns
rely on. No compiled program reaches that: the native backend refuses the
shape, "`point` as a bare value is not yet supported", and the interpreter,
which runs it, does not read inference. It becomes live the day the backend
accepts a type as a value.

**The regression the first build carried.** `prose_check` ran past five
minutes where main takes 28 seconds. Sampling the process put it in
`k_b_slice_walk` under `worth_trying?`: a character read by position in text
that is not all ascii resumes from one remembered place, `k_seek_str`, and
`k_beat_rewind` forgot it on every rewind. With a rewind at every position,
every position walked the page from the front.

Forgetting it is only needed when the arena can hand the string's address back,
and in the fast path that is exactly `[m->ptr, k_arena)`: no block has been
taken since the mark. The first cut tested the lower end alone, which is sound
and was still slow, because a page can sit in an older block at a higher
address than the mark. The two-ended test, in `uintptr_t` so no unrelated
pointers are subtracted, takes `prose_check` to 15.1 seconds, and its system
time from 14.4 seconds to 0.5, since its memory no longer grows.

`seek_resumes` is the presence counter for the cursor. The mutation
`a_rewind_that_forgets_every_seek_cursor`, the old unconditional forget, takes
`tests/golden/mem/a_scan_keeps_its_place_in_the_text.kso` from 408 to 276, and
the ratchet carries it. The trend gate reads it as higher-is-better. All twelve
cost goldens, the mem vein and the two book counter panels carry the line.

**The counters that read worse.** The scans rewind now, so `scan_beat_iters`
rises to 1,016, `run_beat_iters` to 2,693,195 and
`a_class_asks_by_the_byte_beat_iters` to 1,601. That fixture's pattern matches,
so its scan leaves the cluster with a hit each time, and the run pays one more
allocation of 80 bytes and reuses one buffer fewer:
`a_class_asks_by_the_byte_allocs` reads 10,469,
`a_class_asks_by_the_byte_alloc_bytes` 468,255,
`a_class_asks_by_the_byte_sh_buf` 118,432 and
`a_class_asks_by_the_byte_buf_reuse` 0. Which allocation it is has not been
isolated; the output is unchanged.
`seek_resumes` is minted, and reads 689,999 on the run program.

**What the cursor test costs.** On this container the run row reads
1,820,479,435 on main and 1,833,933,410 with both changes, +13,453,975: the
rewind's fast path now loads the cursor and the arena pointer and compares,
five instructions on each of 2,693,195 iterations. `k_beat_iter` was a real
call from the loops that rewind, because the large ones spend LTO's inlining
budget before reaching it. `always_inline` on it reads 1,826,634,704, giving
back 7,298,706, and every counter vein agrees. The net is +6,155,269, 0.34%.

**CI's rows**, taken into the goldens. The rewind's cursor test is paid by
every program that rewinds, and `always_inline` gives part of it back:

    work_runbench        1,802,356,350 -> 1,809,683,884   +0.41%
    work_encodebench     3,465,000,320 -> 3,479,505,321   +0.42%
    work_livebench       2,793,281,380 -> 2,807,786,381   +0.52%
    work_basket             32,776,834 ->    33,024,826   +0.76%
    work_oneshot            17,807,820 ->    17,844,087   +0.20%
    work_deepbench         347,635,275 ->   347,896,726   +0.08%
    work_scanbench         462,269,305 ->   462,289,601
    work_digestbench         9,966,673 ->     9,966,845
    work_readbench           4,627,056 ->     4,627,255
    work_jsonbench       1,133,645,592 -> 1,133,645,757
    work_indexbench          2,895,743 ->     2,895,771
    work_pendbench         208,139,955 ->   208,139,965
    work_escapebench        75,228,606 ->    72,849,606   -3.16%
    work_widebench          33,676,020 ->    33,660,074

`text`, the sum of the benchmarks' `.text`, reads 1,767,676, the inlined
rewind in every loop that makes one. `library_instructions` reads 126,699,213
and `startup_instructions` 3,372,417, where `lib/regexp` and `src/runtime.c`
are compiled into the compiler; `entry_instructions` reads 126,074,458 and the
two codegen rows 596,206,478 and 6,841,764,937. The scan benchmark's peak fell
by 160 MB while its instruction row moved by 20,296.

With CI's rows the objective scores 82.09 against a floor of 77.37, and the
rise is banked. On the tree merged with kanso#1578, `startup_instructions`
reads 968,492: that change's 968,441 and this one's +51.

**What moves.** The run program's peak is a deterministic counter and scores
here: welfare 77.36 -> 82.10, production 57.23 -> 66.07, with the instruction
rows as main has them. The fast rewind now compares before it stores, and the
run program takes 2,693,195 beat iterations. The run row, the compile rows and the codegen rows come from CI,
and the rise is banked after they land.

Open: the carry tier's path prefix, which this change routed around rather
than replaced; and a type used as a value, which must widen its field sets in
`infer.rs` before the native backend accepts one.

With kanso#1583 merged from main, the codegen and start-up goldens hold a
projection until CI measures them: main's rows plus this change's own moves,
which puts `codegen_instructions_dev` at 563,940,183,
`codegen_instructions_release` at 6,809,498,642 and `startup_instructions` at
967,920.

kanso#1580 then landed on main, and the run program carries both changes. Its
arena peak reads 6,098,640 in 6 blocks, against 35,458,768 with the digest
change alone and 9,244,368 with the scan change alone. The two sets of rewinds
add: `run_beat_iters` reads 2,709,445. The emitted rows are counted here from
the `.ll` files, `runbench defines=595 calls=5982 branches=3540 lines=35805`.
The instruction, `.text`, entry, library, codegen and emit rows come from CI,
and the floor is banked again after them.

**CI's rows over the merged tree**, taken into the goldens. The codegen and
start-up projections above were exact.

    work_runbench             1,794,573,732 -> 1,801,929,451   +7,355,719   +0.41%
    work_digestbench              5,773,783 ->     5,799,501      +25,718
    entry_instructions          125,949,337 ->   125,944,853       -4,484
    library_instructions        126,452,016 ->   126,522,328      +70,312
    text runbench                   319,218 ->       319,810
    text digestbench                105,634 ->       105,938

Every row here is this change's own cost measured on the new base, which is
what the scan's rewinds, the seek-cursor test on every rewind and the new
counter cost in instructions. The run program pays 0.41% for a peak that falls
from 35,458,768 to 6,098,640. `library_instructions` rises with the library
text the compiler carries, since lib/regexp is compiled into it. The
`data-golden` spans quoting the entry and library rows were rewritten by
`golden_prose --write`. Summed over the fourteen binaries, `text` reads
1,764,140.

kanso#1582 then landed on main. Merged over it, the emitted rows are counted
here from the `.ll` files, `scanbench defines=271 calls=3108 branches=2112
lines=19621` and `runbench defines=523 calls=5733 branches=3430 lines=34568`,
and the codegen and start-up goldens hold a projection, main's rows plus this
change's own moves: `codegen_instructions_dev` 473,848,240,
`codegen_instructions_release` 6,588,771,476 and `startup_instructions`
972,533. CI's rows replace them before the floor is banked again.
CI read start-up exactly and both codegen rows differently from the
projection: `codegen_instructions_dev` 473,849,441, 1,201 above it, and
`codegen_instructions_release` 6,585,606,376, 3,165,100 below. The two moves
of the release row do not add, which is the LTO link reading the combined
program rather than either change alone. Welfare rises over the projection and
is banked again.

---

## 2026-09-23 — the seek cursor records which marks its string lies under, so a rewind asks one question

kanso#1579 made the rewind's fast path forget the seek cursor only when the
string it names sits in `[m->ptr, k_arena)`, the range the rewind hands back.
That is exact and it cost five instructions an iteration: load the cursor and
the arena pointer, two subtractions, a compare. CI read it as +0.41% on the
run program after `always_inline` had given half of it back.

The question can be answered when the cursor is set instead, which a scan
does once per string it starts on, where a loop rewinds at every iteration.
`k_seek_note` records the innermost mark the string lies under: the top mark
if the string is not in the range that mark would hand back, one deeper if it
is, and conservatively one deeper whenever a new block has been taken since
the mark, because then the range is not one test. At depth zero the answer is
the bottom of the stack, since every mark comes after the string, and past the
stack's last slot it is one past the end, so every rewind forgets. The rewind
then compares its own mark against that pointer:

    cmp    %rdi, k_seek_under
    jbe    keep
    movq   $0, k_seek_str

A mark pushed later than the one recorded cannot hand the string back, and
one popped and pushed again in the same slot comes after the string too, so
comparing slot addresses is enough. The slow path still forgets unconditionally.

On this container:

    runbench              1,826,634,704 -> 1,815,911,759   -10,722,945
    against main          1,820,479,421 -> 1,815,911,759    -4,567,662   -0.25%
    prose_check           15.1 s -> 13.1 s

Every counter vein agrees, `seek_resumes` included; the beat differential
reads 0 of its 96 layout pairs disagreeing, and
`a_seek_cursor_does_not_outlive_its_string`, the fixture for the cursor kept
past its string, passes. The mutation `a_rewind_that_forgets_every_seek_cursor`
now replaces the new comparison and still takes
`a_scan_keeps_its_place_in_the_text` from 408 to 276.

**Declined on the way, recorded so it stays declined.** `encode_onto` is
entered 2,380,860 times on the run program at 26 instructions of frame and
dispatch a call, and most calls are leaves. Giving `elem_onto` and a new
`value_onto` their own string and int arms, so a list element or a map value
of those kinds never entered `encode_onto`, read 1,874,943,854 against main's
1,820,479,421: +54,464,433, +2.99%, with the same output and all 23
`lib/json` tests passing. The two small dispatchers cost more than the calls
they saved. `docs/compiler.html` §81 and §110 found the dispatcher's frame is its own;
this is the same wall from the caller's side.

**CI's rows**, taken into the goldens, against kanso#1579's:

    work_runbench             1,801,929,451 -> 1,791,146,495   -10,782,956   -0.60%
    work_encodebench          3,479,505,321 -> 3,459,377,334   -20,127,987
    work_livebench            2,807,786,381 -> 2,787,658,394   -20,127,987
    work_basket                  33,024,826 ->    32,568,835      -455,991
    work_deepbench              347,896,726 ->   347,687,558      -209,168
    work_digestbench              5,799,501 ->     5,766,137       -33,364
    work_jsonbench            1,133,645,757 -> 1,133,645,908          +151
    work_pendbench              208,139,965 ->   208,139,970            +5
    codegen_instructions_dev    473,849,441 ->   473,884,358       +34,917
    codegen_instructions_release 6,585,606,376 -> 6,598,389,807 +12,783,431   +0.19%
    startup_instructions            972,533 ->       972,482           -51

The run program gets back more than kanso#1579's cursor test cost it, and the
two encoders fall by the same 20,127,987, which is the rewind their loops take
most often. jsonbench and pendbench move by 151 and 5 instructions, both
programs that rewind rarely, where the note taken on every scan outweighs the
test it saves. `.text` rises on all but the run program, 1,765,228 summed over
the fourteen binaries, for `k_seek_note`'s body. The release codegen row rises
0.19% for the same runtime code compiled and linked into every program.
Against main, which does not yet carry kanso#1579, two more rows read worse.
`work_indexbench` reads 2,895,756, 13 above main, and `work_scanbench`
462,285,578, 16,273 above: both programs scan strings and seldom rewind, so
they pay the note on every scan and collect little of what it saves.

---

## 2026-09-23 — a call between a package's own modules is a cohort again

A construction cohort brackets a call whose arguments are immutable: the arena
is marked before the call and rewound after it when the answer is a scalar, or
the answer is copied out when it is not. The license admitted a call only when
the callee's module name extended the caller's by a segment. That was the
spelling of nesting before module identity became the canonical path, and the
archive entry "the qualified door" (2026-08-18) found the test had stopped
matching and said it had to ask whether the caller's module imports the
callee's. It was never rebuilt. Since then the only caller it admitted was the
root module.

The run program shows the cost. `runbench/tally` calls `index/total`,
`escape/total` and `split/total`, each a phase that builds its own strings and
answers a number, and none was bracketed. On a tree with kanso#1579,
kanso#1580 and kanso#1581 merged, the index phase's doubled string and its
slice, 1,572,880 and 1,380,032 bytes, stayed live after it returned, and the
next allocation opened a fresh one-megabyte block on top of them. That was the
run program's peak.

The license now asks whether the callee is declared in another file than the
caller. A direct call by name can only go down an import, so that is the
relation the archive entry asked for.

Admitting every such call cost the run program 13.1% of its instructions,
because json's own modules call one another from its recursive descent,
millions of times, and each call paid a push and a pop. So a caller that a
cycle can reach gets no bracket. `cycle_reached` finds every cycle in what the
bodies mention and every name those cycles reach. A caller outside that set
runs a number of times its straight-line callers fix, which is where a phase
starts and ends. Excluding only the members of cycles was not enough:
`json/number_done` sits in no cycle, `scan` reaches it once per number, and
jsonbench rose 5.3%.

`tests/a_phase_gives_its_garbage_back.rs` builds a package whose `app` module
calls `phase/churn` twice. It reads `cohort_frees=2` and an arena peak of
3,145,744. Watched red with the old test: one free, and 5,242,912.

On the merged tree, with the gates' commands:

    runbench   arena peak        6,098,640 ->     5,050,064   -17.2%
               instructions  1,808,039,943 -> 1,810,051,241   +0.11%
    oneshot    instructions     17,945,090 ->    19,972,550   +11.3%
    deepbench  instructions    356,864,590 ->   358,111,263   +0.35%

jsonbench, encodebench, widebench, livebench, escapebench, indexbench and
readbench read the same instructions to the unit. oneshot's rise is one pop
around `json/decode` in `hold/report`. The survivor guard sizes a decoded
document that is nearly all of what the call grew, keeps the region, and the
sizing is the 2,027,460 instructions. The archive shows oneshot had this pop
when the license was first generalized; its peak and allocations do not move.

On main, with kanso#1579 and kanso#1580 merged, the run program's peak is the
index phase, and it falls as measured on the merged tree above:

    run program   arena peak      6,098,640 ->   5,050,064   -17.2%
                  cohort_frees            1 ->           4

The counters that read worse all arrived with the new pops, which is where a
heap answer is copied out before its call's garbage is rewound, and none of
them changes an output. In the run program `run_allocs` reads 5,698,908,
`run_alloc_bytes` 458,172,125, `run_evac_allocs` 68,318, `run_evac_bytes`
10,791,200 and `run_sh_buf` 109,369,344. One append moved from the in-place
path to the copying one, `run_push_mut_fast` 1,098,391 and `run_push_mut_slow`
1,638,122, and one more string was scanned from its start, `run_str_scans` 164
and `run_str_scan_bytes` 5,473,158. pendbench and
scanbench each gained one pop: `pend_allocs` reads 806,180,
`pend_alloc_bytes` 45,529,344, `pend_evac_allocs` 2,431 and `pend_evac_bytes`
384,944, and scanbench's `ten_handups` reads 1. digestbench and the mem vein
read what main has.

**CI's rows**, taken into the goldens:

    work_runbench        1,794,573,732 -> 1,796,601,576   +2,027,844   +0.11%
    work_oneshot            17,807,820 ->    19,783,275   +1,975,455  +11.09%
    work_deepbench         347,635,275 ->   349,439,154   +1,803,879   +0.52%
    work_scanbench         462,269,305 ->   462,283,315      +14,010
    work_digestbench         5,773,783 ->     5,773,971         +188
    work_basket             32,776,834 ->    32,777,016         +182
    work_pendbench         208,139,955 ->   208,132,753       -7,202
    startup_instructions       967,869 ->       971,368       +3,499
    emit_instructions       45,953,348 ->    46,222,808     +269,460   +0.59%

Each program that gained a pop gained two calls and a few lines per
bracketed site, which the emitted vein counts: summed, `emitted_other_calls`
reads 20,237 and `emitted_other_lines` 135,899. `text`, summed over the
fourteen binaries, reads 1,766,412. `emit_instructions` is `cycle_reached`,
a walk of every body and a Tarjan pass the emitter did not make before.
`startup_instructions` counts kanso's own start and rose with the compiler's
code; what in it costs 3,499 instructions was not isolated. Both codegen rows
read what main has. The run program pays 2,027,844 instructions for a peak
3,145,728 bytes lower, and welfare rises; the rise is banked.

kanso#1582 then landed on main, which pruned lambdas nothing reaches. Merged
over it, the emitted rows are counted here from the `.ll` files, and each
program that gained a pop still gained its two calls a site: summed,
`emitted_other_calls` reads 18,849 and `emitted_other_lines` 127,780. The
start-up and emit goldens hold a projection, main's rows plus this change's own
moves: `startup_instructions` 975,981 and `emit_instructions` 44,879,920. CI's
rows replace them.

kanso#1579 then landed on main, and the rows above were measured before it.
Merged over it, the counters and emitted rows are regenerated here, summed
`emitted_other_calls` 18,837 and `emitted_other_lines` 127,690, and the
instruction, `.text` and start-up rows hold main's values until CI measures
the merged tree. The floor is banked again after that.

**CI's rows over the merged tree**, taken into the goldens:

    work_runbench        1,801,929,451 -> 1,803,940,793   +2,011,342   +0.11%
    work_oneshot            17,844,087 ->    19,819,542   +1,975,455  +11.07%
    work_deepbench         347,896,726 ->   349,700,605   +1,803,879   +0.52%
    work_basket             33,024,826 ->    33,027,009       +2,183
    work_scanbench         462,289,601 ->   462,289,850         +249
    work_digestbench         5,799,501 ->     5,799,688         +187
    work_pendbench         208,139,965 ->   208,132,763       -7,202
    startup_instructions       972,533 ->       976,032       +3,499

The emit projection was exact at 44,879,920, and both codegen rows read what
main has. `text`, summed over the fourteen binaries, reads 1,771,932. The run
program pays 2,011,342 instructions for a peak 1,048,576 bytes lower, and
welfare rises; the rise is banked.

kanso#1581 then landed on main. Merged over it, every counter vein and the
emitted rows read what they read above, and CI's rows are in the goldens:
`work_runbench` 1,793,157,817, `work_oneshot` 19,769,235, `work_deepbench`
349,491,458, `work_basket` 32,569,017, `work_scanbench` 462,285,827,
`work_digestbench` 5,766,324, `work_pendbench` 208,132,768 and
`startup_instructions` 975,981, with `text` summed at 1,772,924. The projection
from main's rows plus this change's moves over kanso#1579 was exact for five
of the eight and within 2,001 for the rest. The run program pays 2,011,322
instructions for a peak 1,048,576 bytes lower, and the rise is banked.

---

## 2026-09-23 — the runtime links as machine code, and the release build costs less than half

The release codegen row is three quarters the LTO link, and the ThinLTO entry
above found where: `ld` optimizes and generates code for the runtime together
with the program on every release build. Split by phase on the codegen corpus,
the link is 1,727,215,014 instructions of optimization and 3,168,917,243 of
code generation. What it generates code for is mostly the runtime:

    runtime .text    59,082 bytes   k_exec 9,980, k_render_at 4,548, ...
    program .text    19,135 bytes

`cached_runtime_object("release", ...)` compiled the runtime `-O3 -flto`,
which is bitcode. It compiles `-O3` now, which is machine code, and the link
optimizes and generates the program alone. On this container, with the gates'
commands and `GITHUB_ACTIONS=1`:

    codegen_instructions_release   6,598,715,476 -> 2,848,583,206   -56.8%
    runbench                       1,810,051,241 -> 1,871,199,161    +3.38%

The run program's rise is three helpers the link used to inline into their
callers: `k_b_find2_below_raw` +62,131,590, `k_b_find2_raw` +56,425,515 and
`k_beat_iter` +43,146,280, against callers that read that much less. At
today's ratios the release row's fall is worth about +1.6 and the run row's
rise about -0.26. ThinLTO shrank the same link and was declined at -3.47% for
+3.03%; this is the trade that version was reaching for, with the runtime
kept whole.

The runtime object's cache key did not include the flags it was compiled
with. A machine that had built the bitcode object would have kept linking it
under the new flags until the temp directory was cleared, so the flags are in
the key now. `the_runtime_key_names_the_flags` fails with "two flag sets share
a cached runtime object" when they are taken out.

**The three helpers stay in the link.** The runtime is compiled
`-DK_HOT_ELSEWHERE`, which leaves out `k_b_find2_raw`, `k_b_find2_below_raw`
and `k_beat_iter`, and a second object defines them as bitcode, so the LTO
link inlines them into the program as it did. `hot_source` in src/main.rs
builds that object's source from `runtime.c`'s own text: the three
definitions, `k_tail_window` and `k_beat_rewind` beside them, and the typedefs
they need. A dozen specs read these functions out of `runtime.c`, and a second
copy would drift from what they check. The globals the helpers reach lose
`static` so both objects can name them: `k_blocks`, `k_seek_str`,
`k_beat_stack`, `k_beat_depth`, `k_beat_top`, `k_seek_under`, `k_buf_dirty`
and two counters, and `k_beat_rewind_slow`.

    codegen_instructions_release   6,598,715,476 -> 2,898,793,716   -56.07%
    runbench                       1,810,051,241 -> 1,811,839,413    +0.10%

The helpers cost the release row 50,210,510 instructions and give back
59,359,748 of the run program's 61,147,920. Every counter vein and the lazy
tier read what main has, the counting build included, which puts the two
counters the helpers increment in the second object.
`every_hot_definition_is_found` fails when a signature the extraction looks
for is edited out of `runtime.c`.

**CI's rows**, taken into the goldens:

    codegen_instructions_release 6,598,389,807 -> 2,898,336,765   -56.08%
    codegen_instructions_dev        473,884,358 ->   473,933,874   +49,516
    work_runbench        1,793,157,817 ->   1,798,630,190   +0.31%
    work_jsonbench       1,133,645,908 ->   1,138,862,859   +0.46%
    work_encodebench     3,459,377,334 ->   3,481,870,112   +0.65%
    work_livebench       2,787,658,394 ->   2,805,256,521   +0.63%
    work_deepbench         349,491,458 ->     351,324,436   +0.52%
    work_escapebench        72,849,606 ->      74,602,433   +2.41%
    work_scanbench         462,285,827 ->     463,280,487   +0.22%
    work_oneshot            19,769,235 ->      19,850,560   +0.41%
    work_basket             32,569,017 ->      32,585,639   +0.05%
    work_widebench          33,660,015 ->      33,660,736   +0.00%
    work_pendbench         208,132,768 ->     208,152,834   +0.01%
    work_indexbench          2,895,756 ->       2,906,581   +0.37%
    work_digestbench         5,766,324 ->       5,767,585   +0.02%
    work_readbench           4,627,255 ->       4,630,249   +0.06%

CI reads the run program at +0.31% where this container read +0.10%, and
every benchmark pays something: the helpers that stay in the link are the
three the run program lost most to, and the other programs lean on runtime
functions this change leaves outside it. escapebench pays most,
2.41%. `text` rises on every program, 2,249,452 summed over the
fourteen against 1,772,924 before -- a native runtime is linked whole, where the
LTO link dropped what the program never reached -- and machine-code size
carries no welfare term. `compile_instructions` reads 35,400,618,
`entry_instructions` 125,944,855, `library_instructions` 126,522,330 and
`startup_instructions` 975,983, each 2 above main: `runtime.c` is embedded in
the compiler and grew by the guard comments, which moves the compiler's own
layout. The `data-golden` spans quoting the three compile rows were rewritten
by `golden_prose --write`. The objective reads 85.09 against 84.00, and the
rise is banked.

The trend gate first read this change as a pure regression. Both codegen rows
were walked by it and named in no direction table, so the release row's fall
counted toward neither side while fourteen work rows rose.
`tests/every_compile_vein_row_has_a_direction.rs` exists to catch exactly that
and missed it, because it matched vein files ending `instructions_golden.txt`
and the codegen veins end `instructions_dev_golden.txt` and
`instructions_release_golden.txt`. The spec now matches any instructions vein,
went red naming the two rows, and the rows are in the table as lower-is-better.

---

## 2026-09-23 — the runtime inlines at the program's threshold

kanso#1585 compiled the release runtime as machine code, on its own, with
`-O3`. While it was bitcode the LTO link had applied `-inline-threshold=2000`
to it along with the program, so compiled alone it fell back to clang's
default of 225, and helpers the runtime calls on itself stopped being inlined:
comparing profiles, `k_b_push_grow` became a call of its own and read
19,222,350 instructions on jsonbench, and `k_survives` 774,400 on runbench.
The runtime is now compiled with the program's threshold.

On this container, runbench at four thresholds for the runtime alone:

    225     1,811,839,413
    1000    1,809,106,361
    2000    1,806,069,074
    4000    1,805,972,054

2000 is the program's value and 4000 buys 97,020 more. Against main before
kanso#1585, the fourteen work rows read:

    jsonbench -0.30%   encodebench -0.10%   oneshot -0.10%   basket -1.02%
    widebench +0.67%   deepbench +2.33%     escapebench +1.63%
    pendbench -0.01%   indexbench +0.39%    scanbench +1.66%
    digestbench -0.09% readbench +0.07%     livebench -0.21%  runbench -0.22%

deepbench and scanbench read worse at 2000 than at 225, where the rest read
better; the run program is the one the objective weighs. The release codegen
row reads 2,903,108,801 against 2,898,793,716 at 225, +0.15%, which is the
linker placing a larger runtime object; the runtime's own compile is cached
and the gate warms it before counting. CI's rows go into the goldens.

**CI's rows**, taken into the goldens:

    codegen_instructions_release 2,898,336,765 -> 2,900,713,494   +0.08%
    work_jsonbench       1,138,862,859 ->   1,129,050,376   -0.86%
    work_encodebench     3,481,870,112 ->   3,479,633,527   -0.06%
    work_oneshot            19,850,560 ->      19,872,932   +0.11%
    work_basket             32,585,639 ->      32,376,516   -0.64%
    work_widebench          33,660,736 ->      33,530,471   -0.39%
    work_deepbench         351,324,436 ->     359,347,473   +2.28%
    work_escapebench        74,602,433 ->      74,053,454   -0.74%
    work_pendbench         208,152,834 ->     208,259,451   +0.05%
    work_indexbench          2,906,581 ->       2,907,171   +0.02%
    work_scanbench         463,280,487 ->     468,791,320   +1.19%
    work_digestbench         5,767,585 ->       5,787,538   +0.35%
    work_readbench           4,630,249 ->       4,630,466   +0.00%
    work_livebench       2,805,256,521 ->   2,803,274,864   -0.07%
    work_runbench        1,798,630,190 ->   1,793,355,755   -0.29%

CI reads the run program at -0.29% and the release row at +0.08%. The work
rows that rise at 2000 are the ones this container showed rising, deepbench
and scanbench most. `text`, summed over the fourteen binaries, reads
3,372,588: a runtime that inlines more is a larger object, linked
whole. The objective rises, and the rise is banked.

---

## 2026-09-23 — the program is optimized once, at the link

A release build ran the optimizer over the program twice. `clang -O3 -flto`
put the program's IR through the full pipeline in `clang -cc1` on the way to
bitcode, 1,394,202,866 instructions on the codegen corpus, and the LTO link
ran its own `-O3` pipeline over the program and the hot helpers together.
The pre-link level is now `-O1`, with `-Wl,-plugin-opt=O3` after the driver's
own so the link stays at `-O3`. On this container, over kanso#1586:

                          release codegen     runbench
    -O3, as it was        2,903,108,801       1,806,069,074
    -O2                   2,536,822,676       1,815,479,975   +0.52%
    -O1                   1,751,444,900       1,895,750,256   +4.97%
    no pre-link passes    1,380,698,213       2,053,358,047  +13.69%

The link's pipeline expects its input already simplified, which is why
removing the pre-link passes outright costs the run program 13.69%. Scored by
the welfare script against main's goldens scaled by these ratios, -O1 reads
+0.19 and -O2 +0.11; the objective weighs a release build at 0.15 of
production against run speed's 0.45, and at these ratios the build's saving
is the larger. CI's rows go into the goldens.

**CI's rows**, taken into the goldens:

    codegen_instructions_release 2,900,713,494 -> 1,751,097,561   -39.63%
    work_jsonbench       1,129,050,376 ->   1,196,422,427   +5.97%
    work_encodebench     3,479,633,527 ->   3,356,324,328   -3.54%
    work_oneshot            19,872,932 ->      20,373,089   +2.52%
    work_basket             32,376,516 ->      32,678,753   +0.93%
    work_widebench          33,530,471 ->      34,746,491   +3.63%
    work_deepbench         359,347,473 ->     366,363,489   +1.95%
    work_escapebench        74,053,454 ->      80,047,456   +8.09%
    work_pendbench         208,259,451 ->     209,065,155   +0.39%
    work_indexbench          2,907,171 ->       2,927,166   +0.69%
    work_scanbench         468,791,320 ->     451,724,087   -3.64%
    work_digestbench         5,787,538 ->       5,866,515   +1.36%
    work_readbench           4,630,466 ->       4,630,465   -0.00%
    work_livebench       2,803,274,864 ->   2,824,128,034   +0.74%
    work_runbench        1,793,355,755 ->   1,853,571,514   +3.36%

CI reads the run program at +3.36% where this container read +4.97%. `text`,
summed over the fourteen binaries, reads 3,215,292 against
3,372,588. The objective rises, and the rise is banked.

`the other host (macos, arm)` failed eight native targets on the first round
with `ld: unknown options: -plugin-opt=O3`. `-plugin-opt` is the gold plugin's
spelling and Apple's ld64 has no such option, so the split is Linux-only and
other hosts keep `-O3` for both steps, as they were. The rows above are
Linux's and do not move.

## 2026-09-24 — the program cache keys its IR at a third of SipHash's cost

`kanso play` and `kanso run` reuse a linked binary when its key matches, and
the key was `DefaultHasher` over the emitted IR. The start-up corpus is one
`print`, and its IR is 40,413 bytes, almost all of it runtime declarations.
Hashing that cost 101,260 instructions, about a tenth of the start-up row:
SipHash at two and a half instructions a byte.

`hash::key_of` replaces it. Two lanes take alternate words in xxHash64's
round (multiply the word, add, rotate, multiply) and murmur3's finaliser
mixes each lane. The key is 128 bits, where SipHash gave this one 64, and a
collision here runs a different program than the one asked for.

    startup_instructions   995,155 -> 908,786   -8.68%   (this container)

**The first draft was wrong, and the spec found it.** It used xor, multiply
and rotate, with no multiply after the rotate. An odd multiply flips only the
top bit when the top bit flips, the rotate carries that bit to bit 30 whole,
and a flip of bit 30 in the lane's next word erases it: 1,010 of 32,768
single-bit changes to an IR-shaped input met another one's key.
`tests/the_program_key_sees_every_bit.rs` holds three properties: every
single-bit change gives a key no other one gives; no pair of top-bit changes
in words the same lane reads cancels (65,280 pairs); and a short input is not
one padded with zeros. Watched red twice. Without the rotate, 3,866 single
changes collide and 4,920 pairs cancel. Without the multiply after it, 3,633
and 61.

**The same profile named a second cost.** `narrow_tailcc` strips the tail
call convention from functions no `musttail` reaches, and it read the
program's body three times, splitting it into lines on each pass. Splitting a
string is a search for each newline, and the three splits of the one-line
program's body were 75,721 instructions. The body is now split once and its
lines walked three times. runbench's IR, 1,199,233 bytes, is byte-identical
before and after.

    startup_instructions   908,786 -> 876,314   -3.57%   (this container)
                           995,155 -> 876,314  -11.94%   both changes

A shortcut was tried first and dropped: returning the body untouched when it
never spells `tailcc`. That never fires, because every user function is
emitted with the convention and this pass is what strips it, and the row
read 913,993 against 908,786.

**CI's rows**, taken into the goldens:

    startup_instructions     975,983 ->     870,779   -10.78%
    emit_instructions     44,879,920 ->  43,339,387    -3.43%

`emit_instructions` counts `codegen::emit_ir` on the compile corpus, and the
single split is what moved it. No other row moved. The objective rises, and the
rise is banked.

## 2026-09-24 — a short float is rendered by scaling and dividing back

`render_ryu` was 4.33% of the run program: 82,180,980 instructions over
191,070 calls, 430 a float. Every float in the encode corpus has seven or fewer
significant digits, and for a decimal that short ryu's 125-bit multiplies and
its digit removal do more work than the answer needs.

`ryu_d2d` now tries the decimal places in order first. At place p it rounds
f·10^p to an integer m and accepts m·10^-p when `(double)m / 10^p == f`. The
division is one correctly rounded operation on two exact doubles, so a match
proves the decimal reads back as f under strtod's round-half-even. It cannot
miss one: while ulp(f)·10^p ≤ 1/4, at most one decimal with p places reads back
as f, it lies within 1/8 of the exact product, and the rounded product is
within 1/4 of that. So the first place that passes gives the shortest decimal,
and the one ryu would choose. A decimal with fewer significant digits at a
later place would have to sit across a power of ten from it, and that power of
ten is a one-digit candidate at a place already tried. The path covers f in
[2^-20, 2^50); outside that range, or when the bound runs out, ryu decides as
before.

The conversions are signed. Every product is below 2^51, and on x86-64 an
unsigned double conversion is about a dozen instructions each way where a
signed one is one. The unsigned first build saved 14,344,110; the signed one
saves twice that.

    render_ryu        82,180,980 -> 53,842,770    430 -> 282 a float
    runbench       1,895,843,068 -> 1,867,504,858   -1.49%   (this container)

**The harness came first.** A differential fuzzer takes `ryu_d2d` and
`render_ryu` out of the runtime at HEAD and out of the working tree, compiles
both, and compares digits, exponent and rendered text. It covers random doubles
across the path's exponent range and past both ends, short decimals of one to
seventeen digits at every scale to 10^22 with their neighbours on both sides,
decimals ending in a 5, and both sides of every binary exponent and power of
ten in range. 1,495,188,243 compared, 0 differ. Three mutations each went red:
dropping the rounding step (7,088 differ in 1,286,385), loosening the bound
sixty-four times (69,680) and not stripping an integer's trailing zeros
(117,369).

**The shipped spec gained the property this rests on.**
`every_rendered_float_reads_back_as_itself` checked round trip and shortest,
and both pass a renderer that picks the wrong neighbour of the right length.
The loosened-bound mutation did exactly that. The spec now also checks
closest: when the nearest k-digit decimal (glibc's `%.*e`, which is exact)
reads back, the renderer must have chosen it. The condition matters. The
first draft required the nearest decimal whether or not it read back, and it
reported 93 failures, every one a power of two. There the doubles below are
twice as dense, so the interval that reads back is half as wide on that side:
2^-1017 prints as 7.120236347223045e-307 although ...044 is nearer, and Python's
`repr` agrees. The spec also gained a million short decimals of every length,
with their neighbours. Watched red: the loosened bound gives 208,683 not
closest and 28,657 not shortest, and the dropped rounding gives 25,474 not
shortest.

**A presence counter, `ryu_short`,** counts the floats the short path settled.
It sits after `ryu_renders` in every counter dump, so all twelve cost goldens,
the .mem vein and the two book samples that print counters gained the line.
It equals `ryu_renders` in every golden: 191,070 on the run program, 849,200 on
encode and live, 8,000 on wide and 2,123 on oneshot. The trend gate reads it as
higher-is-better beside `seek_resumes`. No allocation counter moves; rendering
allocates nothing.

**The general loop is now outside every benchmark.** Because no benchmark float
reaches ryu's digit-removal loop, the ratchet row that guarded its two-a-trip
shape through the work vein could not go red any more, and it is retired. Its
replacement closes the short path, and `run_counters` goes red on
`ryu_short=191070` -> `0`. The objective does not see floats of more than
about fifteen significant digits, or outside [2^-20, 2^50), and those still
cost what they did.

**CI's rows**, taken into the goldens:

    work_runbench         1,853,571,514 ->   1,825,042,054   -1.54%
    work_encodebench      3,356,324,328 ->   3,229,526,728   -3.78%
    work_livebench        2,824,128,034 ->   2,697,330,434   -4.49%
    work_widebench           34,746,491 ->      30,712,160  -11.61%
    work_oneshot             20,373,089 ->      20,056,095   -1.56%
    startup_instructions        975,983 ->         976,034   +51
    codegen_instructions_dev    473,933,874 ->   473,952,844   +18,970
    codegen_instructions_release 1,751,097,561 -> 1,751,553,021  +455,460

Merged over kanso#1591, which took start-up to 870,779, CI reads 870,779
again. The +51 above did not carry over: it was projected at 870,830, main's
value plus this change's own delta, and the projection was wrong, so the row
is CI's.

`text`, summed over the fourteen binaries, reads 3,244,860 against 3,215,292:
2,112 bytes more in each, which is the short path and the digit writer it
shares with ryu. The three codegen and start-up rows rise because the runtime
the corpus builds is larger by the same code. The objective rises by 0.08, and
the rise is banked.

## 2026-09-24 — declined: keeping a dispatcher's big arms out of line

`encode_onto` saves and restores six callee-saved registers on every one of
its 2,380,860 calls a run, about 62 million instructions. The emitted
dispatcher is small, but the link inlines `encode_list`, `encode_map` and
`escape_onto` into it. After that, the true, false, null and number arms,
each a runtime call in tail position, pay the frame the string and container
arms need.

Marking the three callees `noinline` in runbench.ll and relinking with the
release command takes the run program from 1,867,504,858 to 1,841,390,170,
-1.40%. The pairs taken alone do not help: the two container callees read
1,868,242,224 and `escape_onto` alone reads 1,890,472,678. The emitter has no
way to name those three without the profile, so three general rules were
measured the same way:

    every function in a recursive cycle            2,283,659,459   +22.3%
    every user call inside a dispatcher             1,880,320,396   +0.69%
    user calls inside a recursive dispatcher        1,869,244,114   +0.09%

The last rule is the one that includes `encode_onto`, and the other 62
dispatchers it covers spend what that one saves. The gain belongs to one
function's arm frequencies, which is profile data, so this is declined until
the emitter has a profile to read.

## 2026-09-24 — a short float's text is written from its scaled integer

The short path finds a float's decimal as an integer m and a place count p,
and handed ryu's digit buffer back to `render_ryu`. The plain branches then
placed the decimal point by walking that buffer a byte at a time, copied the
digits after it, and ran `ryu_declen`'s ladder a second time. For a value from
10^-4 up to 10^15 the text is m / 10^p, then a point and m's last p digits with
their leading zeros. `render_ryu` now writes that straight into the output,
the integer part through `ryu_write` and the fraction two digits at a time.
Values outside that range, and floats the short path leaves, go through the
digit buffer and the branches as before.

The search moved into its own `ryu_short`, so `render_ryu` runs it once and
hands a miss to ryu's core, now `ryu_long`, without searching a second time.
`ryu_d2d` is the pair of them and still answers the spec harness. `ryu_long` is
`noinline, cold`: inlined, its registers set the frame every short render paid.

    render_ryu     53,842,770 -> 44,289,000   the direct writer
                   44,289,000 -> 42,463,170   ryu_long kept out of line
                   282 -> 222 instructions a float
    runbench    1,867,504,858 -> 1,856,125,258   -0.61%   (this container)

The differential fuzzer compared this core against main's, before either
short path, on the same classes as the first sitting: 1,495,188,243 compared, 0
differ. Three mutations of the writer each went red: dropping the odd leading
digit of the fraction (140,311 differ in 1,884,414), admitting values down to
10^-5 (17,762) and admitting sixteen-digit integers (209). The float spec
passes unchanged.

**Two ratchet rows follow the path.** The row that walked the plain branches'
digit copies a byte at a time guarded code no benchmark float reaches now: under
that mutation runbench and encodebench read 1,856,125,258 and 3,516,909,606,
identical to the unmutated tree. Its replacement closes the direct writer, and
runbench reads 1,861,738,918, which the work vein sees. The short-path row's
mutation now patches `ryu_short`'s range test, the line the restructure
rewrote. No counter moves.

**CI's rows**, taken into the goldens:

    work_runbench         1,825,042,054 ->   1,813,491,634   -0.63%
    work_encodebench      3,229,526,728 ->   3,178,191,528   -1.59%
    work_livebench        2,697,330,434 ->   2,645,995,234   -1.90%
    work_widebench           30,712,160 ->      30,153,189   -1.82%
    work_oneshot             20,056,095 ->      19,927,757   -0.64%
    startup_instructions        976,034 ->         975,983   -51
    codegen_instructions_release 1,751,553,021 -> 1,750,938,263  -614,758
    codegen_instructions_dev    473,952,844 ->   473,969,350   +16,506

`text`, summed over the fourteen binaries, reads 3,260,988 against 3,244,860,
1,152 bytes more in each: the direct writer beside the branches it skips.
`codegen_instructions_dev` rises by the same code compiled at -O0. The
objective rises, and the rise is banked.

## 2026-09-24 — the dev tier's instruction selector falls back on the calling convention

`llc -O0` on the codegen corpus's IR spends 137,115,835 of 326,668,822
instructions, 41.97%, in SelectionDAG's per-block selection, against
6,953,622 in FastISel. FastISel works through a block from its terminator, and
a terminator it cannot select sends the whole block to SelectionDAG. Its
remarks name 307 `br label` terminators into blocks with `%KValue` phis, 158
`ret %KValue` and 66 calls.

The aggregate looked like the cause, which would have made this an emitter
ABI change. Before sizing that, the corpus IR was rewritten so that user
functions return `void` and their callers read a stand-in runtime call. The
rewrite is wrong as a program and fine as a codegen measurement. `llc` then
read 323,782,838, -0.88%, and the misses moved rather than shrank: 265 calls
and 78 returns, most of them `ret void` in `tailcc` functions. FastISel on
x86-64 selects neither calls nor returns in the `tailcc` convention.

`tailcc` is what guarantees the tail calls that let mutual recursion run in
constant stack, and a dev build without it would overflow on the deep
recursion a release build runs. So the fallback is the convention's, and
taking the aggregate out of the returns would not remove it. The lead is
closed until FastISel selects `tailcc`. GlobalISel was measured on the same
IR at 1,070,967,706, three times the default. Running `instcombine` after the
always-inliner cost 41,445,873 in `opt` and saved 19,550,143 in `llc`, a net
loss of 21,895,730.

## 2026-09-24 — the scanners' ratchet row builds again

The ratchet on kanso#1585 failed one row, marked UNBUILT: "the two byte
scanners called out of line with their constants". Its mutation removes
`always_inline` from `k_b_find2_raw` and `k_b_find2_below_raw`. Since #1585
the release build lifts both into a small bitcode unit, and `hot_source` found
them by a start string that included the attribute. With the attribute gone
it found nothing, the compiler panicked, and the mutated tree never reached
the gate. Every branch touching runtime.c selects that row, so the same
failure waited for each of them.

`hot_source` now finds each definition by its signature and takes it from the
start of the line, attribute and all. The text it lifts from the unmutated
runtime is byte-identical, 2,242 and 2,893 bytes. The mutated tree builds, and
runbench reads 1,917,945,517 against 1,867,504,844, which the work vein sees.
Marking the scanners `noinline` instead reads the same 1,917,945,517: the link
does not inline a plain definition back. `the_scanners_are_found_without_their
_attribute` lifts the unit from runtime text with the attribute removed, and
went red with the start strings keyed on the attribute again.

## 2026-09-24 — a wildcard arm keeps a record slot boxed

Found while writing a dev-tier spec. This program printed 7 under the oracle
and ran out of stack natively, under `kanso run`, `kanso play` and a
`kanso build` binary alike:

    type point
      x
      y

    fn total (point x y)
      x + y

    fn total _
      0

    print "{total (point 3 4) + total 5}"

The escape analysis gave `total`'s parameter the by-value record convention,
because every arm that names a record there names `point`. The `_` arm did not
count against it. A caller converts its argument for that convention with
`k_parsed_words`, which passes a failure through and otherwise reads two
fields off a record. The int 5 has no fields, and that read is where the stack
ran out. The analysis now also asks the inference which shapes reach the
position, and keeps the convention only when every one is a record, a failure
or a thunk that is forced before the call. The json decoder's carried slots
receive records and failures only, so they keep it: the runtime counter sweep
and the compile sweep agree with every golden this host can compare.

`tests/golden/micro/a_wildcard_arm_keeps_a_record_slot_boxed.kso` is the
program above, and the micro corpus runs it on every engine and as a release
build. Without the fix it went red on the native engine, which printed
nothing.

A record of another type needed a second rule. The inference has one bit for
every record, so it cannot tell a `pair` from a `point`, and `total (pair 1 2)`
printed 10 natively where the oracle printed 7: the pair's two fields were
read as a point's. An arm that takes any value at the position, meaning a
name, `_` or an annotated name, now keeps the slot boxed as well.
`a_wildcard_arm_takes_a_record_of_another_type.kso` pins it. With only the
first rule it printed 10. The two rules together still leave every vein this
host can compare where it was. The first rule is still needed for a literal
arm, since `fn total 5` beside a record arm lets an int reach the slot with no
wildcard in sight.

CI's rows. The two new checks run at every emit, for every carried position,
and cost the one-line start-up program 47 instructions:

    startup_instructions   870,779 ->    870,826   +47
    emit_instructions   43,339,387 -> 43,339,434   +47

Every other row agreed. The fix builds the differential law, which the
language rests on, so the floor comes down by what it costs.

## 2026-09-24 — a release build links with lld where it can

The release build's LTO link ran in GNU ld with LLVM's plugin. On the codegen
corpus that process was 1,163,896,203 instructions, of which LLVM itself was
926,724,119. The rest was the linker's own work: 128,765,719 in libc,
52,810,020 in libbfd and 14,726,035 in the dynamic loader. lld does the same
LTO in-process with less around it:

    codegen, release tier   1,750,778,100 -> 1,676,140,277   -4.26%   (this container)
    runbench                1,895,843,054 -> 1,895,843,321   +267

The program is the same code: lld honours the link's `O3` and the inlining
threshold, and an `O2` link measured the same day moved runbench +0.31%.

lld is asked for, not assumed. An lld from a different LLVM release than
clang's cannot read clang's bitcode, and a runner can carry one, so
`lld_links_lto` links a one-line LTO program with `-fuse-ld=lld` and uses lld
only if that succeeds. The answer is remembered under the identities of both
tools, the way the calling-convention probe's is. Only Linux asks; Apple's
ld64 is untouched. CI's cost-goldens job installs `lld-19` beside `clang-19`,
puts it on PATH and asserts the version, as it does for clang. It also
installs the image's default `lld`: the codegen gate runs under `env -i
PATH=/usr/bin:/bin`, where clang is the image's 18, and the first round, with
only 19 installed, left both codegen rows exactly where they were.
`tests/a_release_build_links_with_lld_when_it_can.rs` builds a release binary
and requires lld's `Linker:` stamp in it exactly when lld can take the link.
It went red with `-fuse-ld=bfd` in the flag's place.

**The dev tier links with it too.** Its link has no LTO in it, and GNU ld
still spent 85,738,887 instructions on the codegen corpus against lld's
45,154,514:

    codegen, dev tier         475,880,595 ->   435,319,658   -8.52%   (this container)

The same probe decides, since an lld that can take an LTO link can take a
plain one. The spec builds both tiers and went red on each with
`-fuse-ld=bfd` in that tier's place.

**The specs job's runner has lld beside clang but not on PATH.** The spec's
own probe said lld could take the link and kanso, finding no `ld.lld` on PATH,
never asked; the binary carried no stamp and the spec went red. What is on
PATH is now part of the probe's key and not a condition of asking, so the
probe decides wherever clang can find lld. The ratchet's toolchain installs
the same packages as the cost-goldens job, which
`the_ratchet_carries_what_its_gates_need` requires.

**lld links on every thread unless told not to, and a count cannot have
that.** Two CI runs of one tree read the dev row 434,345,526 and 434,337,763
and the release row 1,677,317,287 and 1,677,792,392: the thread pool's
scheduling lands in callgrind's count. Three dev links on this container read
434,957,073, 434,928,291 and 434,926,291 on the default and 434,600,869 three
times with `--threads=1`. `KANSO_LTO_JOBS`, which the gates already set to ask
for one LTO job, now sets lld's thread count too. A user's build keeps every
thread.

**CI's rows**, from the first run, taken into the goldens and replaced by the
next round's:

    codegen_instructions_dev      473,933,874 ->   434,345,526   -8.35%
    codegen_instructions_release 1,751,097,561 -> 1,677,317,287   -4.21%
    startup_instructions              870,779 ->       870,804   +25
    work_basket                    32,678,753 ->    32,678,777   +24
    work_runbench               1,853,571,514 -> 1,853,571,431   -83
    compile_instructions, entry_instructions and library_instructions -2 each

The benchmarks are now linked by lld as well. Their `.text` sums to 3,215,152
against 3,215,292, and every work row but basket moved by 20 to 116
instructions, down. Basket rose 24 and start-up 25, which is layout: the
binaries lld links place the same code differently, and the compiler's own
bytes grew by the probe. The objective rises, and the rise is banked.

## 2026-09-24 — a build runs clang's jobs without the driver

A build spawned three processes: the clang driver, `clang -cc1`, and the
linker. The driver of a dev build on the codegen corpus retired 31,702,963
instructions, 26,450,962 of them in the dynamic loader's `_dl_start`
relocating libLLVM and libclang-cpp. Its only work was deciding the two
commands it then ran, and those depend on the toolchain, the flags and the
file names, never on the program.

So the driver is asked once, with `-###`, for a build whose input and output
carry placeholder names in a stage directory of their own. The two commands
are kept under a key naming both tools, every flag and object, and the two
environment variables the driver reads for paths. Every later build runs them
directly with its own names put back, and the link writes straight to the
build's output. The driver still runs when it prints anything but two jobs,
when a job's program is missing, when `-save-temps` is asked for, off Linux,
and when `KANSO_CLANG_DRIVER` is set.

    codegen, dev tier        434,010,676 ->   401,968,983   -7.38%   (this container)
    codegen, release tier  1,676,647,397 -> 1,644,593,553   -1.91%

Both counts are the child tree with the job cache warm, the way the gate
reads it. The binaries are byte-identical to the driver's on both tiers.
`tests/a_build_runs_clang_s_jobs_without_its_driver.rs` builds each tier
through the driver, then by asking and by replaying, and compares the bytes.
It went red with the output's placeholder left in the link: no `main` was
written where the driver's had been.

**No path of the process reaches a job.** The gate sets `KANSO_FIXED_TEMPS`,
which adds `-save-temps=obj` to a release build so the LTO object has the
same name every run. ld's plugin hashes that path, and a random one moved the
row by eleven instructions one run in eleven. The replay drops that option and
fixes the names itself. The object is `<name>.o` relative to the stage, the
stage is named by a digest of the output path rather than the pid, the two
compilation directories are `.`, and the replay refuses a job that still names
the stage. Under the gate's own environment, with lld on one thread, three
release replays read 1,641,527,539 each and two driver builds 1,674,062,024
each, -1.94%. Two dev replays read 401,763,944 against the driver's
433,335,421, -7.29%.

CLAUDE.md lists the codegen row's child tree as "the clang driver, the
convention probe's clang, `clang -cc1` and ld". A warm build no longer has
the driver in it.

CI's rows go into the goldens.

**The gate's floor on processes, and the pipe.** The first CI round for this
change failed two ways, and neither was about the rows. First,
`codegen_instructions.sh` refused a tree of fewer than four processes, and a
warm build with no driver is three: kanso, `clang -cc1` and ld. It exited
before its second reading, so the job printed one number per tier and no
verdict. The floor is three now, and the message names the driver as one of
the two answers that are cached. Second, `asked` read the driver's `-###`
listing with `.output()`, and
`tests/the_compiler_never_drains_a_childs_pipes.rs` forbids that anywhere in
src/main.rs, because draining a pipe is scheduling rather than work. The
listing now goes to a file in the stage, and the call is `.status()`.

CI's rows, taken into the goldens:

    codegen_instructions_dev       434,345,526 ->   402,338,337   -7.37%
    codegen_instructions_release 1,677,317,287 -> 1,643,423,398   -2.02%

Two readings of each tier on this container, clang 18, agreed to the
instruction: 401,763,927 dev and 1,642,105,200 release. kanso#1592's dev row
read 434,007,475 and then 434,007,432 in one CI job, and here the lld link was
the process that moved, by 43 instructions: it links the object the driver
names with a random suffix. The replay names that object itself, so this
change carries kanso#1592 and supersedes it.

CI's rows over main with kanso#1589 and kanso#1590, where both codegen tiers
read the same number twice in one job:

    codegen_instructions_dev       402,338,337 ->   402,356,485
    codegen_instructions_release 1,643,423,398 -> 1,643,086,293

lld lays the linked benchmarks out differently from GNU ld. Each benchmark's
`.text` grows six bytes, and each one's work falls by between 20 and 116
instructions: runbench's work goes from 1,813,491,634 to 1,813,491,551, and
its text from 390,914 to 390,920 bytes. The text rows for every benchmark in
bench/text_golden.txt are this runner's sitting: jsonbench text=215816,
encodebench text=234920, oneshot text=224136, basket text=219992, widebench
text=241384, deepbench text=203064, escapebench text=188504, pendbench
text=211928, indexbench text=188312, scanbench text=307064, digestbench
text=221192, readbench text=188872, livebench text=224968 and runbench
text=390920. Summed, the `text` counter goes from 3,260,988 to 3,261,072. The
objective does not weigh machine-code size.

Over main with kanso#1597, whose two checks cost start-up 47 instructions on
the same 870,779 this change added 25 to, the start-up row is written as
870,851: the two moves summed. The next CI round says whether they add.

## 2026-09-24 — a dev build calls the runtime's helpers instead of inlining them

Every module carries the runtime's small helpers as definitions: tag tests,
the fast arms of append, index and length, and the closure-call twins, all
`alwaysinline`. At `-O0` the always-inliner still honours the attribute and
copies each helper into every call site, and the instruction selector then
walks every copy. On the codegen corpus's module, which defines thirty-six of
them:

    clang -cc1 -O0, as emitted            359,109,516
    clang -cc1 -O0, attribute removed     316,180,072   -11.95%   (this container)

The dev tier is the one that compiles fast, and its binaries' speed is not a
term in the objective. The release tier, where inlining the helpers is the
point, keeps the attribute. `emit_ir_dev` gives the dev tier's module, and
`kanso build` without `--release`, `kanso run` and `kanso play` use it. The
dev module's text is made when the compiler is built: see below.

No helper needs inlining to be correct. None allocates on the stack, reads a
frame or return address, or makes a `musttail` call, and the program calls
them only with plain calls. The whole suite passes with dev modules built
this way.

`tests/a_dev_build_calls_the_runtime_helpers.rs` builds a program with a
closure call and an append on both tiers. It requires that the dev module
define no helper `alwaysinline`, that the release module define some, and
that both binaries print the same line. It went red with `emit_ir_dev` asking
for inlined helpers.

**What still sends the dev tier to SelectionDAG is the aggregate.** With the
helpers called rather than inlined, `clang -cc1` still spends 164,838,950 of
338,572,876 instructions in `SelectionDAGISel`, and FastISel's remarks name 265
calls, 157 returns and 59 branches into blocks with `%KValue` phis: calls that
pass a `%KValue`, returns of one, and phis of one. Routing each function's
returns through a stack slot and one small return block was measured and
declined. FastISel cannot store an aggregate either, and `cc1` read 328,312,489
against 316,182,558. What would reach it is a dev module that carries a
`%KValue` as two `i64`s through calls, returns and phis, which is an emitter
change of its own.

**The emit gate's anchor moves with the work.** `emit_instructions` read
`codegen::emit_ir` inclusive on a dev build, and a dev build now enters
through `emit_ir_dev`. Both entry points call `emit_ir_for`, which is kept out
of line, and the gate now reads that frame. It found 43,868,047 on this
container.

**CI's rows**, taken into the goldens:

    codegen_instructions_dev     473,933,874 ->   431,017,582   -9.06%
    startup_instructions             870,779 ->       880,309   +9,530
    emit_instructions             43,339,387 ->    43,359,136   +19,749

The dev row is the saving. The start-up rise was the first draft's: every
line of DECLARES asked whether it carried the attribute, 6,192 instructions of
`Backend::emit` on the one-line program and 10,508 in all. The dev text and
its line index are now made when the compiler is built, as `DECLARES_DEV` and
`DECLARE_LINES_DEV`, and `declares_for` picks a pair once. Start-up on this
container read 902,425 with the check and 875,795 without, against main's
876,314, and the next CI round's rows replace the two above. A unit test holds
the dev text to the release text with the attribute stripped by a scan at run
time, and went red when the dev branch was handed the release pair.

The next CI round, with the dev text made when the compiler is built:

    startup_instructions             880,309 ->       866,508   -13,801
    emit_instructions             43,359,136 ->    43,320,461   -38,675

Against main's 870,779 and 43,339,387 both rows now fall, and the floor is
banked on them.

Over main with kanso#1589 and kanso#1590, the dev row read 431,093,381 twice
in one job. That is 75,799 above the 431,017,582 taken before the merge, and
it is the link: `codegen_instructions_dev` now includes ld linking the
runtime those two changes grew. The floor was banked on the earlier row
before CI had measured the merge, and it is set again on this one, which is
still above main's.

Over kanso#1593, which runs clang's two jobs itself and links with lld, and
main with kanso#1597, the three rows this change moves are written as the
two changes' measured deltas summed over main's: the dev row 359,480,516,
start-up 866,580 and emit 43,320,508. The next CI round replaces them.

CI's rows over kanso#1593 and main: start-up 866,580 and emit 43,320,508, as
summed, and the dev row 359,558,108, read twice alike, 77,592 above the sum.

## 2026-09-24 — a dev module leaves out the helpers nothing reaches

DECLARES defines thirty-three runtime helpers, and every module carries all of
them. A release build inlines them and the optimiser drops what is unused. At
`-O0` nothing drops an unused internal function, so `clang -cc1` selected
instructions for every helper in every dev module. On the codegen corpus
twenty-four of the thirty-three were called by nothing: the dev module defined
thirty-five internal functions, thirty-three helpers and two closure twins,
and eleven of them were reached.

When the compiler is built, `index_helpers` reads each helper's extent in the
dev text, from its `define` to its lone closing brace, and the helpers each one
calls by `@name(`. It then closes those calls to a fixed point, one bit per
helper in a `u64`. At emit, `declares_for_program` marks the helpers the
program's bodies and twins name, takes everything they reach, and skips the
line ranges of the rest. The release text is unchanged and still carries every
helper, because its inliner drops them anyway.

The first draft scanned the whole text once per helper inside the const
evaluator, which rustc stopped as taking too long; one pass that tracks the
current helper by line does the same work.

Measured on this container, clang 18, with the codegen gate's environment:

    codegen_instructions_dev   430,900,667 -> 396,836,531   -7.90%
      clang -cc1               314,696,278 -> 280,905,590
      ld                        84,671,938 ->  84,398,490
    startup_instructions           875,795 ->     801,634   -8.47%

Start-up falls because `kanso play` emits a dev module for its one-line
program, and that module no longer writes the thirty-two helpers `print`
does not call. CI's rows replace these.

`tests/a_dev_module_defines_only_the_helpers_it_reaches.rs` builds a bare
`print` and a program with a closure call and an append on both tiers. It
requires both binaries to print the same thing and every internal function
the dev module defines to be named on some other line of it. With every helper
counted as reached it went red on both programs, listing the helpers nothing
called.

CI's rows for the pruning, taken into the goldens. The dev row read the same
number twice in one job:

    codegen_instructions_dev     431,017,582 ->   396,949,978   -7.90%
    startup_instructions             866,508 ->       775,903   -10.46%
    emit_instructions             43,320,461 ->    43,295,953   -24,508

Over kanso#1594's rows on top of kanso#1593, this change's rows are written as
its own measured moves applied to its parent's: the dev row 325,412,912,
start-up 775,975 and emit 43,296,000. The next CI round replaces them.

CI's rows over kanso#1594's: the dev row 326,056,595, read twice alike, and
start-up 775,975 and emit 43,296,000, as summed.

## 2026-09-24 — a release module leaves out the helpers nothing reaches too

The dev tier's helper pruning applies to release modules as well. A release
build inlines its helpers, and the optimiser removes the ones nothing calls.
But clang parses all thirty-three first and runs the early passes over them.
`declares_for_program` now takes the release text's helper index
(`HELPERS_RELEASE`, built from DECLARES the way `HELPERS_DEV` is built from
the dev text), so the two tiers keep the same helpers.

Measured on this container, clang 18, gate environment, release tier:

    clang (the -flto compile)   555,145,625 -> 545,749,531   -9,396,094
    ld (the LTO link)         1,164,544,832 -> 1,164,544,832   unchanged
    codegen_instructions_release 1,751,444,900 -> 1,742,048,806   -0.54%

The link's count is identical, which is what dropping text the optimiser
would have deleted anyway should look like. The runtime counter sweep agrees
with every golden, `bench/text_golden.txt` included. The emitted-code goldens
fall because each module defines fewer functions: the decoder goes from 134
defines and 9,052 lines to 119 and 8,814. The compile-cost goldens fall for
the same reason. CI's codegen rows replace the local ones above.

Two specs assumed every module carried every helper, and both now state their
property over the helpers a module defines. `perf_ratchet`'s
`hot_predicates_are_inline_definitions_not_declares` checks that each hot
predicate the program calls is an `alwaysinline` definition, and it fails if
the program calls none. `the_counting_build_and_the_shipped_one_agree` now
expects the gates that DECLARES holds in the helpers the module defines,
counted by `codegen::stats_gates_carried`, rather than all eight. With every
gate folded in the counting build it went red, 0 against 4. The pruning spec,
renamed to `tests/a_module_defines_only_the_helpers_it_reaches.rs`, now holds
the release module to the same rule. It went red when the release tier was
handed no helper index.

This clears the way for helpers that only one tier calls: a release module
that does not call one no longer carries it.

**Declined: a block of its own for each return.** FastISel cannot lower a
`%KValue` return, and a terminator it cannot lower sends its whole block to
SelectionDAG. Moving each such `ret` into a one-instruction block, and each
`br` into a `%KValue` phi block into a trampoline, was tried on the pruned
corpus module with a text rewrite. `clang -cc1 -O0` read 287,802,692 as
emitted, 297,839,204 with 163 returns split, and 301,669,877 with 222 returns
and branches split. Each block SelectionDAG takes costs a fixed setup, so
splitting added blocks faster than it removed work. An all-SelectionDAG
compile of the same module read 335,125,013, so FastISel saves 47 million
instructions today.

CI's rows for the release pruning, over kanso#1595's:

    codegen_instructions_release 1,643,086,293 -> 1,634,830,087   -8,256,206
    startup_instructions               775,975 ->       776,882   +907
    emit_instructions               43,296,000 ->    43,332,550   +36,550

Both tiers' rows read the same number twice in one job. Emit and start-up pay
for the release tier now walking the helper index to find what a module
reaches. The objective weighs release codegen far above the emit row, which
has saturated, so the trade comes out ahead.

## 2026-09-24 — the dev tier asks its hot predicates in two words

At -O0 clang's fast instruction selector lowers a call only when every
argument is a scalar. A call passing a `%KValue` goes to SelectionDAG on its
own. On the pruned codegen corpus 216 of those calls went to `k_not_failure`,
`k_check_rec_fast` and `k_truthy`, and 168 of them to `k_not_failure`, which
reads only the tag. DECLARES now carries `k_not_failure_w(i64 tag)`,
`k_truthy_w(i64 tag, i64 pay)` and `k_check_rec_fast_w(i64 tag, i64 pay, ...)`.
A dev module extracts the value's words and calls those, and each slow path
builds the `%KValue` back only where it calls the runtime. `FnEmit::predicate`
does the rewrite at every site that writes one of the three calls. A release
module never calls the two-word forms, so kanso#1596's pruning leaves them
out, and the emitted-code and compile-cost goldens do not move.

The sites were chosen with a text rewrite of the corpus module before any
emitter change. `clang -cc1 -O0` read 287,802,692 as emitted, 270,908,624 with
`k_not_failure` called on the tag, and 264,829,993 with all three rewritten.
Writing the tag test inline at each site read 272,759,087, which is worse than
the call on the tag.

Measured on this container, clang 18, gate environment:

    clang -cc1 (dev)            280,905,590 -> 256,343,115
    codegen_instructions_dev    396,836,531 -> 372,274,647   -6.19%
    startup_instructions            787,528 ->     798,956   +11,428

Start-up pays for three more names in the sorted list that each `declare` line
is looked up in. The binary search goes one level deeper, which is 159 more
`memcmp` calls on the one-line program. A dev codegen fall of 6.19% is worth
more to the objective than a start-up rise of 1.45%. CI's rows replace these.

`tests/a_dev_build_asks_its_predicates_in_words.rs` builds a program with a
record test between two record arms, a truth test and a failure test on both
tiers. It requires the same output from each, that the dev module passes no
temporary `%KValue` to any of the three, that it calls each two-word form, and
that the release module calls none of them. It went red with the dev tier
asking the `%KValue` forms. The first draft of its program had a record arm
beside a wildcard arm, which is how kanso#1597 was found.

CI's rows, over kanso#1596's:

    codegen_instructions_dev     326,056,595 ->   301,084,979   -7.66%
    startup_instructions             776,882 ->       788,333   +11,451
    emit_instructions             43,332,550 ->    45,838,870   +2,506,320

Both dev readings agreed. The emit row rises by more than the rewrite's own
frames: each predicate call now writes two or three lines where it wrote one,
and every pass over the body text (the call scans, the reachability pass,
the formatting and allocation behind each line) pays for the extra lines. The
objective weighs the emit row lightly and dev codegen heavily, and welfare
rises 0.03 on the three together.

## 2026-09-24 — a literal's words are written, not extracted

The arithmetic fast path reads each operand's payload with `inline_payload`,
and for a literal operand that was `extractvalue %KValue { i64 0, i64 1 }, 1`:
the number 1 with an instruction around it. clang's fast selector at -O0 does
not lower an `extractvalue` of a constant, so the rest of the block went to
SelectionDAG. The pruned corpus module had 48 of them, and a text rewrite that
wrote the word instead took `clang -cc1` from 263,368,026 to 260,365,961.
`inline_tag` and `inline_payload` now answer a literal's word directly, and
the argument to an unboxed parameter goes through `inline_payload` rather than
writing its own `extractvalue`.

Measured on this container, clang 18, gate environment:

    clang -cc1 (dev)            256,343,115 -> 251,483,787
    codegen_instructions_dev    372,274,647 -> 367,410,698   -1.31%
    startup_instructions            798,956 ->     798,960   +4

The release module loses the same lines, and the optimiser had already folded
them, so the runtime counter sweep, `bench/text_golden.txt` included, agrees
with every golden. The emitted-code goldens fall, the decoder from 8,814 lines
to 8,650, and the compile-cost goldens with them. CI's rows replace the local
ones above.

`tests/a_literal_s_words_are_written_not_extracted.rs` builds a countdown on
both tiers and requires that neither module reads a word off a literal and
that both print 55. It went red with `literal_word` answering nothing.

CI's rows, over kanso#1598's, each read twice alike where the gate reads twice:

    codegen_instructions_dev       301,084,979 ->   296,677,065   -1.46%
    codegen_instructions_release 1,634,830,087 -> 1,633,515,589   -0.08%
    startup_instructions               788,333 ->       788,337   +4
    emit_instructions               45,838,870 ->    45,620,520   -218,350

The four instructions of start-up are the literal check on the one-line
program's few operands.

## 2026-09-24 — the type tables are arrays, not switches

Every module defines four functions the runtime calls to print a record and
read a field by name: `k_type_name`, `k_type_shown`, `k_type_field_count` and
`k_type_field_name`. Each was a switch over the type id, with an arm per type,
and `k_type_field_name` held a nested switch per type's fields. clang's fast
selector at -O0 does not lower a switch, and on the codegen corpus a text
rewrite of those eighteen switches into compare chains alone took `clang
-cc1` from 258,576,424 to 255,915,152. The ids are dense, 0 for the entry and
a type's position plus one, and an alias's slot falls back as it did. So
each lookup is now a load from a constant array behind a bounds check. The
arrays and the four functions are written into `globals`, beside the interned
strings, and not into the body that the call scans and the reachability pass
read.

Measured on this container, clang 18, gate environment:

    codegen_instructions_dev        367,410,698 ->   360,741,989   -1.82%
    codegen_instructions_release  1,742,456,776 -> 1,723,410,294   -1.09%
      clang (-flto compile)         545,440,966 ->   538,359,753
      ld (the LTO link)           1,165,261,367 -> 1,153,296,098
    startup_instructions                798,960 ->       750,547   -6.06%

The first draft wrote the four functions and their arrays into the body. The
dev row fell the same, but start-up rose to 824,547, because every pass over
the body walked the longer text. Moving the arrays alone out of it left
start-up at 814,293, and moving the functions with them took it to 750,547.
The runtime counter sweep agrees with every golden. The emitted-code goldens
fall, the decoder from 8,650 lines and 771 branches to 8,570 and 764 and one
call more, where `k_type_field_name` asks `k_type_field_count` for its bound.
The compile-cost goldens fall with them. CI's rows replace the local ones
above.

`tests/type_tables_are_read_not_switched.rs` prints two records and reads
fields by name, both keyed and with `.b`, on both tiers. It requires the
output the interpreter gives and that no module switches on a type id. It
went red against the switch tables.

The dispatchers' own switches are left. A compare chain in their place was
worth 1,965,238 instructions to the dev compile, and the release tier wants
the switch for its jump tables.

CI's rows, over kanso#1599's, with both codegen tiers read twice alike:

    codegen_instructions_dev       296,677,065 ->   289,762,106   -2.33%
    codegen_instructions_release 1,633,515,589 -> 1,614,704,366   -1.15%
    startup_instructions               788,337 ->       740,091   -6.12%
    emit_instructions               45,620,520 ->    45,195,370   -0.93%

Every benchmark's work rises by between 26 and 938 instructions, and its
`.text` shrinks. The switch arms were straight-line returns and the table is a
bounds check, a load and a return, so each lookup the runtime makes costs a
few instructions more. The run program's work goes from 1,813,491,551 to
1,813,492,695 (work_runbench 1,813,492,695). The rest land at work_jsonbench
1,196,422,558, work_encodebench 3,178,192,129, work_oneshot 19,927,890,
work_basket 32,679,271, work_widebench 30,153,767, work_deepbench
366,364,093, work_escapebench 80,047,462, work_pendbench 209,065,629,
work_indexbench 2,927,172, work_scanbench 451,725,005, work_digestbench
5,866,958, work_readbench 4,630,497 and work_livebench 2,645,995,367. The run
row's rise is six parts in ten million, which the objective weighs far below
the codegen and start-up falls.

## 2026-09-24 — a dev dispatcher compares instead of switching

A dispatcher whose arms discriminate on int literals, or on a value's tag,
compiles to a branch on that value, written as a `switch`. clang's fast
selector at -O0 does not lower a switch, and the block went to SelectionDAG.
`FnEmit::switch_on` writes the switch in a release module, where the
optimiser makes a jump table of it, and a compare and a branch per case in a
dev module. The three dispatcher sites use it. `d_thunk_eval` keeps its
switch: its arms load and pass `%KValue`s and return one, so they go to
SelectionDAG whatever branches to them.

Measured on this container, clang 18, gate environment:

    clang -cc1 (dev)            244,780,924 -> 243,383,587
    codegen_instructions_dev    360,741,989 -> 359,341,981   -0.39%
    startup_instructions            750,547 ->     750,547

The release module is unchanged, and so are the emitted-code, compile-cost and
runtime goldens. `tests/a_dev_dispatcher_compares_instead_of_switching.rs`
builds a four-arm int dispatcher on both tiers. It requires the same output
from each, no switch outside the thunk dispatcher in the dev module, and a
switch in the release module. It went red with the dev tier writing the
switch.

The compiler page's §132 covers this change and the three before it on the
dev tier.

CI's rows, over kanso#1600's, the dev row read twice alike:

    codegen_instructions_dev    289,762,106 -> 287,891,869   -0.65%
    emit_instructions            45,195,370 ->  45,312,275   +116,905

The emit row pays for the compare chain's extra lines, as the two-word
predicates' did.

## 2026-09-24 — each declare line knows whether DECLARES calls it

A module keeps a `declare` line from DECLARES when the program calls its
symbol, or when one of DECLARES's own definitions does. The second question
was a binary search over the 65 context-call names, asked for every `declare`
line of every module. Each comparison went through `memcmp`, and each probe
cost about thirty instructions on the start-up program. The answer depends
on DECLARES alone, so `index_declares` now computes it when the compiler is
built, as `DeclareLine::context`, and emit asks only the program's own calls.

The modules are byte-identical: the codegen corpus's dev and release modules
compare equal to the ones the previous compiler wrote. Measured on this
container, with the gate's environment:

    startup_instructions   750,547 -> 693,898   -7.55%

The unit test `every_declare_line_knows_whether_declares_calls_it` holds every
line's flag, in both the release and the dev text, to the binary search it
replaced. It went red, on `k_truthy_bad`, with the flag left false. CI's
start-up and emit rows replace the local one above.

CI's rows, over kanso#1601's:

    startup_instructions         740,091 ->     683,920   -7.59%
    emit_instructions         45,312,275 ->  45,259,445   -52,830

The codegen rows are unchanged, as byte-identical modules should leave them.

## 2026-09-24 — the lexer stops allocating per word and per number

Four allocations or conversions the lexer made for every token of a kind,
each for a value that lived one statement:

- `lex_word` built a `String` for every identifier and keyword, matched it
  against the keywords and copied it into a `Name`. On an ascii line the word
  is now borrowed from the source.
- `Scanner::new` decoded every line into characters as UTF-8. An ascii line's
  bytes are now widened instead.
- An operator was found by collecting it and the next character into a
  two-character `String` and comparing that to each spelling. It is now
  compared a character at a time.
- An integer literal was collected into a `String` and parsed by `BigInt`'s
  general radix conversion. One of eighteen digits or fewer, which always
  fits a `u64`, is now summed where it lies; a longer one takes the old path.

Measured on this container against main, in the gates' own environment:

    compile_instructions     36,020,748 ->  35,022,168   -2.77%
    entry_instructions      128,029,876 -> 124,752,107   -2.56%
    library_instructions    128,618,323 -> 125,307,971   -2.57%
    startup_instructions        876,369 ->     871,732   -0.53%
    compile_allocs               27,313 ->      22,567   -17.38%

The allocation count is the same on every host, so its golden moves here.
The instruction rows are CI's to measure. The interpreted run's row does not
move, since it counts only the run.

`tests/an_integer_literal_reads_the_same_at_every_width.rs` reads literals
either side of the eighteen-digit edge, at `i64`'s limit, and past 64 bits,
on both engines. Past 64 bits it requires the interpreter's answer and the
native engine's refusal by name. It went red with the fast path widened to
twenty digits. The identifier and line changes are exercised by every
program the suite compiles.

The same branch then took the allocation profile past the lexer. The
allocator's callers were attributed back through the generic frames (vector
growth, hash-table growth, `String` clones) to the first compiler function
above them. Nine places were allocating a container that is filled once and
dropped, and most of them held a single entry:

- the spacing check built a zeroed `bool` row per line to mark effect-type
  runs, which almost no line holds. It is now empty until one is found.
- the unused-line check built a vector of join leaves per statement. One is
  now cleared and refilled.
- the alias canonicaliser kept a `Vec` per declaration site and a `HashSet`
  per bare name, to learn whether a name had exactly one target. The first
  entry now lives in the map, and the set is a three-state count.
- `qualify` kept its bound names as `String`s and cloned the whole list for
  every scope and lambda. They are `Name`s now, which hold a short name
  inline. Its owned-name keys became `Name`s too, and each qualified
  spelling, which it composed twice (once as a key, once for the
  declaration), is composed once and moved.
- trmc kept three vectors per dispatch group before two tests that turn
  nearly every group away. The tests now read the arms in place.
- the module's set of every declared name held `String`s. It holds `Name`s.
- the demand pass built a zeroed discard row per group. A group with no
  wildcard now has no entry, which reads the same at the one place it is
  consulted.
- fusion's two name tables held `String` pairs. They hold `Name` pairs.
- the shadow resolver started each declaration with an empty vector of
  locals, and built a hash set per closed scope to find shadowed bindings.
  One vector now serves the file, and the few later bindings in a scope are
  scanned.

Measured on this container, the lexer's rows above as the base:

    compile_instructions     35,022,168 ->  33,798,765   -3.49%
    entry_instructions      124,752,107 -> 120,510,858   -3.40%
    library_instructions    125,307,971 -> 121,070,327   -3.38%
    startup_instructions        871,732 ->     864,859   -0.79%
    compile_allocs               22,567 ->      15,341   -32.02%

Over main, the whole branch takes compile_allocs from 27,313 to 15,341,
-43.83%, and compile_instructions from 36,020,748 to 33,798,765, -6.17%.
The allocation golden moves here and the instruction rows are CI's. Every
spec in the suite passes except the wasm engine's, which needs a
`docs/kanso.wasm` this container has no target to build; CI builds it.

**CI's rows**, over main with kanso#1602, taken into the goldens:

    compile_instructions     35,400,616 ->  33,315,822   -5.89%
    entry_instructions      125,944,853 -> 118,942,141   -5.56%
    library_instructions    126,522,328 -> 119,486,941   -5.56%
    startup_instructions        683,920 ->     672,962   -1.60%
    emit_instructions        45,259,445 ->  45,206,776   -0.12%
    compile_peak_bytes          787,956 ->     777,072   -1.38%
    interp_instructions     853,048,810 -> 852,977,487   -0.01%
    interp_allocs             1,063,803 ->   1,049,281   -1.37%
    interp_peak_bytes           846,367 ->     846,191   -0.02%

The interpreted run moves because the interpreter lexes and parses its
program before running it. Every row falls, and the rise is banked.

## 2026-09-24 — two strings compare without opening an equality generation

`==` and `!=` reach the runtime as `k_cmp`, which sent every pair through
`k_eq`. That opens a cycle-tracking generation, since a record can hold
itself, and then walks a dozen tag tests: an opaque check on each side, two
subtype tests, three thunk tests and the bytes-against-list pair. Only then
does it reach the length and byte compare a string needs. std/regexp asks
this at every step of a literal: `text/slice s at at == c`, one character
against the pattern's. On runbench's scan that is 91,378 questions, at about
ninety instructions each.

`k_cmp` now answers two strings asked `==` or `!=` itself: the same pointer,
or the same length and bytes. A one-character slice comes from `k_str_n`'s
cache and a literal is permanent, so the equal case is usually the pointer.
Anything that is not two strings takes the old road, subtypes and thunks
included, so the two cannot disagree on a pair they both see.

Measured on this container, in the gate's environment:

    the scan phase alone (split/total 428)   88,511,374 ->    81,593,763
    work_runbench                         1,856,032,715 -> 1,849,115,104   -0.37%

The allocation veins and the lazy tier do not move. The instruction rows are
CI's.

`tests/golden/micro/two_strings_compare_by_their_bytes.kso` builds its pairs
at run time: a prefix on either side, the empty string against itself and
against a character, equal bytes behind two pointers, multi-byte text and a
last-byte difference. With the length check dropped from the new path, the
native engine answered `"a" == "ab"` true and the interpreter false, so the
corpus went red on the engine that changed.

The same branch then took one more runtime path. A beat's rewind at the end of
each loop iteration checked the shelf and the registries, compared the seek
cursor's mark and restored the arena pointer and its remaining count. When
nothing was allocated since the mark, the pointer already equals the mark's,
the block is the same, and so the restore writes back what is there. No
string can lie above a mark nothing was allocated past, so the cursor has
nothing to forget either. `k_beat_rewind` now returns there. A loop that only
pushes into a list it owns takes that exit on every iteration. A loop that
allocates pays one extra comparison.

Measured on this container, both sides built and counted here:

    runbench        1,856,032,715 -> 1,852,458,662    -3,574,053   -0.19%
    escapebench        83,605,696 ->    80,014,690    -3,591,006   -4.30%
    basket             32,800,063 ->    32,460,035      -340,028   -1.04%
    encodebench     3,516,817,001 -> 3,521,676,598    +4,859,597   +0.14%
    livebench       2,795,889,600 -> 2,800,749,197    +4,859,597   +0.17%
    deepbench         374,547,103 ->   374,651,687      +104,584   +0.03%

The encoder's loops allocate on every iteration, so they pay the comparison
and never take the exit. The objective weighs runbench alone, and it falls.
No allocation counter moves, and the lazy tier agrees, since the exit changes
no state the rewind would not have written back.

With both changes, runbench reads 1,856,032,715 -> 1,845,541,051 on this
container, -10,491,664 (-0.57%). CI's rows go into the goldens.

**CI's rows**, over main with kanso#1602, taken into the goldens:

    work_runbench      1,813,492,695 -> 1,804,051,708    -9,440,987   -0.52%
    work_scanbench       451,725,005 ->   417,133,254   -34,591,751   -7.66%
    work_escapebench      80,047,462 ->    76,456,459    -3,591,003   -4.49%
    work_basket           32,679,271 ->    32,347,253      -332,018   -1.02%
    work_digestbench       5,866,958 ->     5,842,672       -24,286   -0.41%
    work_encodebench   3,178,192,129 -> 3,179,984,475    +1,792,346   +0.06%
    work_livebench     2,645,995,367 -> 2,653,048,163    +7,052,796   +0.27%
    work_deepbench       366,364,093 ->   366,468,663      +104,570   +0.03%
    work_oneshot          19,927,890 ->    19,945,518       +17,628   +0.09%
    work_readbench         4,630,497 ->     4,630,893          +396   +0.01%
    work_widebench        30,153,767 ->    30,153,803           +36   +0.00%
    codegen_instructions_dev      287,891,869 ->   287,894,238   +2,369
    codegen_instructions_release 1,614,704,366 -> 1,614,817,091  +112,725

scanbench runs the regexp scan at its full size, which is why it falls the
furthest. The rows that rise are loops that allocate on every iteration and
so pay the rewind's comparison without taking its exit. The encoder is the
largest of them. `text`, summed over the fourteen binaries, reads 3,258,368
against 3,256,224: 128 to 224 bytes a binary, since every binary carries both
paths. The two codegen rows
rise because the runtime the link carries is larger: the dev row counts the
link, and the release row compiles the hot unit where the rewind lives.
The objective weighs runbench, which falls, and the rise is banked.

## 2026-09-24 — a dispatcher's heavy arms stay out of its frame

A group of clauses that switches on its argument's type compiles to one
function, and LLVM gives that function one frame. The encoder's
`encode_onto` is eight clauses: true, false, null, an int and a float each
append a few bytes, and a string, a list and a map each run a loop. LLVM
inlined all three loops, the escaper's among them, and their registers are
callee-saved ones. So the function pushed six registers and popped them on
every one of runbench's 2,380,860 calls, 1,438,110 of them for a scalar
whose arm is one call into the runtime. An earlier experiment kept only the
escaper out of line and read +1.24%: the list and map arms still needed the
frame, so every call kept it and the string arm paid for a call on top.

`kept_out` names the callees to keep as calls. A name is loop-bearing when
it sits in a cycle of the call graph or can reach one. In a group that
switches on type, with at least one clause that calls nothing loop-bearing
and at least one that does, every loop-bearing function the heavy clauses
name is defined `noinline`. On runbench that is `escape_onto`,
`encode_list` and `encode_map` and nothing else. Two wider rules were tried
first and are recorded so they are not tried again. The same rule without
the type condition marked eighty-six functions, the decoder's scanning
loops among them, and runbench rose 5.7%. Marking the decoder's four value
parsers as well as the encoder's three added only 921,789 instructions of
saving over the encoder alone.

Measured on this container, both sides built from one tree:

    encodebench   3,516,841,151 -> 3,438,943,935   -77,897,216   -2.22%
    livebench     2,795,835,875 -> 2,683,373,059  -112,462,816   -4.02%
    runbench      1,846,944,217 -> 1,820,829,169   -26,115,048   -1.41%
    oneshot          20,252,928 ->    19,971,752      -281,176   -1.39%
    widebench        30,090,895 ->    29,722,895      -368,000   -1.22%

The other nine benchmarks read the same to the instruction. The instruction
rows will be CI's. No allocation counter moves. The ratchet gains a row,
"a dispatcher's heavy arms inlined into the frame its cheap arms pay", whose
mutation takes the mark off both headers that write it; the work vein is its
witness, and the table above is that mutation measured.

## 2026-09-24 — a number is read out of the bytes it sits in

Every JSON number the decoder meets ends in `text/to_int (text/slice cs
start (p - 1))` or the same with `to_float`. The slice builds a view of the
range for the one purpose of handing it to the converter, which parses it
and drops it. The emitter now sees that pair and calls
`k_b_to_int_slice` or `k_b_to_float_slice` with the source and the two ends.
When the source is bytes and the range lies inside it, the door parses the
bytes where they sit. Any other shape, including an inverted range, a start
below one, an end past the length or a string source, falls back to the
slice and the converter it replaces, so the answer is the one the pair gave.
The two converters' bodies moved into `k_to_int_text` and `k_to_float_text`,
which take a pointer and a length, and the old doors call them too. Only the
builtin spelling is fused, as with the append fusion above it, and the
interpreter is unchanged.

Measured on this container, both sides built here from main ef56eac8:

    runbench      1,856,033,857 -> 1,846,944,217   -9,089,640   -0.49%

The instruction rows will be CI's. The allocation counters fall wherever a
number is decoded, one view fewer per number:

    decode   allocs     4,390,215 -> 3,757,665   alloc_bytes 246,359,648 -> 226,118,048
    decode   sh_bytes  21,567,600 -> 6,386,400
    run      allocs     5,698,908 -> 5,280,394   alloc_bytes 458,172,125 -> 443,885,453
    run      sh_bytes  41,290,272 -> 31,270,680  sh_buf 109,369,344 -> 108,745,936
    run      evac_allocs 68,318 -> 62,993        survive_slots 110,799 -> 108,671
    oneshot  allocs        53,579 -> 49,362
    live     allocs     7,544,011 -> 7,539,794

Two counters on the run program read worse by their direction tables.
`run_ten_frees` falls 7 -> 1 and `run_ten_handups` 4 -> 2, while
`run_ten_blocks` falls 7 -> 6 and `run_cohort_frees` moves 4 -> 2. With
fewer views to evacuate, the tenure allocator claims one block fewer, and
the beat cycles that used to fill and then empty whole blocks no longer
fill them, so there is less to hand up and less to give back. No peak row
on the run program moves, so nothing is held that was not held before. `push_mut_slow` falls 1,638,122 -> 1,638,121 and
`push_mut_fast` rises by the same one. The decoder's emitted code falls:
defines 119 -> 117, calls 1,182 -> 1,172, branches 764 -> 762, lines 8,570
-> 8,542, as the two library wrappers are no longer reached.

`tests/golden/micro/a_number_read_from_a_slice_reads_the_range_in_place.kso`
reads integers and floats out of ranges in the middle, at one digit, with a
sign and an exponent, stopping inside the digits, inverted, starting below
one, ending past the length, holding no digits, and out of a string. Parsing
one byte short in the integer door went red on six lines of it (`123` for
`1234`, `err ""` for `2`).

Writing the spec turned up a native bug that predates this change. A float
with more than nineteen significant digits leaves the Eisel-Lemire path for
strtod, and strtod reads until a byte stops it. A range read out of the middle
of bytes is followed by more digits, so strtod read past it and the parse was
refused: `text/to_float (text/slice long 1 22)` over twenty-nine digits said
`"1234567890123456789012" is not a number` on main, where the interpreter
answers `1.2345678901234568e+21`. The integer slow path had the same shape
through strtoll. Both slow paths now parse a terminated copy of the range, on
the stack up to sixty-three bytes. The golden carries the case, and on the
unfixed runtime its last line goes red with the refusal above. runbench reads
1,846,944,217 either way, since no call there leaves the fast path.

The same probe found four more places where native and the interpreter
disagree about what a number is, in every form (a string, bytes and a slice
all agree within each engine, so none of this is new). Native accepts a
leading space and a hex float, `" 12"` and `"0x1f"`, where the interpreter
refuses both. The interpreter's `to_int` accepts `"1_000"`, because
num-bigint takes underscores as separators, where its own `to_float` and
native refuse it. And `"123456789012345678901234567890.5"` is reported as
overflowing natively and as not an integer by the interpreter. Those are the
next change, with an adversarial golden of their own.

CI's sitting, over main ef56eac8:

    runbench      1,813,492,695 -> 1,805,310,423   -8,182,272   -0.45%
    jsonbench     1,196,422,558 -> 1,187,932,917   -8,489,641   -0.71%
    oneshot          19,927,890 ->    19,872,995      -54,895
    livebench     2,645,995,367 -> 2,645,949,841      -45,526

Several rows are worse, each by a small amount:

- `text` rises on all fourteen binaries, 3,744 bytes each and 3,309,600
  summed. Every binary carries both new doors and the two text parsers,
  whether or not it reads a number.
- `emit_instructions` goes to 45,239,459 (+32,683). The emitter asks each
  one-argument call whether it is one of the two conversions over a slice.
- `codegen_instructions_dev` goes to 287,916,082 and
  `codegen_instructions_release` to 1,614,729,561. clang compiles the
  larger runtime.
- `startup_instructions` goes to 673,771 (+809).
- `work_widebench` goes to 30,361,821 (+208,054, 0.69%). widebench binds
  its slice to a name before converting it, so it never reaches the fused
  door and calls `k_b_to_int` and `k_b_to_float`, which now call the text
  parsers. On this container's clang 18 the same two binaries read
  30,090,895 and 30,090,909. The rise is clang 19's answer to the split,
  and the mechanism is not isolated further.
- `work_encodebench` goes to 3,178,219,787 (+27,658), `work_digestbench`
  to 5,867,017 (+59) and `work_readbench` to 4,630,551 (+54). None of the
  three reads a number.

The objective weighs runbench and not the others. Welfare reads 86.04
against a floor of 86.01, and the rise is banked.

## 2026-09-24 — a number is the same number on every engine

The probe written for the previous entry found four shapes of text where
native and the interpreter disagreed about what `to_int` or `to_float`
returns. The interpreter is the oracle, and in three of the four it was
right. Native's digit loops handle every ordinary number and pass everything
else to strtoll or strtod, which read more than the interpreter's parse:

- a leading space or tab, which libc skips, so `" 12"` was 12 natively and
  refused by the interpreter;
- a hex float, `"0x1f"` and `"0x1p3"`, which strtod reads as 31 and 8;
- a nan with a payload, `"nan(1)"`;
- `"123456789012345678901234567890.5"` in `to_int`, which natively reported
  the overflow strtoll raised on the digits before the point. The
  interpreter reported that it is not an integer, which is the better
  answer: no number of digits would make it one.

The slow paths now refuse the first three and ask whether the whole range was
read before asking whether it overflowed. Bytes that are not utf-8 are
refused as bytes, `bytes are not an integer`, which is what the interpreter
says because it reads bytes as text before it parses them; natively they had
been quoted, high byte and all. Only a range holding a byte above 127 is
checked, so the error path of an ascii number costs no utf-8 pass and moves
no utf-8 counter.

The fourth disagreement was the interpreter's own. num-bigint reads `_` as a
digit separator, so `to_int "1_000"` was 1000 there and refused natively,
while the interpreter's own `to_float "1_000"` refused it too. `to_int` now
refuses text holding an underscore before it asks num-bigint.

`tests/golden/micro/a_number_is_the_same_number_on_every_engine.kso` asks
each case of a string, of its bytes and of a range cut out of longer bytes.
On the unfixed tree native went red on ten lines and the interpreter on
one, the separator.

## 2026-09-24 — four ideas measured and declined

Recorded so the same profile does not send anyone back to them.

`find2_below` over a string, so the encoder skips `text/bytes s` for a clean
string. Built as an experiment on the number span's branch: run allocations
5,698,908 -> 4,915,728 and shared bytes 41,290,272 -> 22,493,952, with
`arena_peak_bytes` unmoved at 5,050,064. The per-string view does not set the
peak. What remains is about one per cent of runbench, and the change widens
a public std/text function to strings, which is surface. Declined; the
compiler page carries it as item 16.

Caching each shipped module's compile by path, so an entry importing
std/text through five libraries compiles it once. `compile_peak_bytes` went
768,704 -> 1,030,180, because the cache holds every tree for the whole
compile. Presizing the front end's hash maps failed the same way at +1.09%
on the peak. Both declined; item 17.

`noinline` on the escape body, so the encoder's leaf arms skip its frame:
runbench 1,856,032,701 -> 1,879,000,521, +1.24%. The inlined scan is worth
more than the frame.

Fusing `length s[i]` over text into a range test. It is 690,000 calls on
runbench, but the fusion would skip the very index walk the index shape is
there to keep linear. Not built.

## 2026-09-24 — an import of a shipped module skips a check fixed at build time

Every module is compiled on top of its dependencies and then checked merged
with them: its own functions and theirs, through one inference and every
check that reads it. For a shipped module that merged program is the
shipped library and nothing else, embedded in the binary by `include_str!`,
so the check gives the same empty answer in every program that imports it.
It was asked once per import in every compile anyway. On the entry corpus
that was seventeen merged checks, one per module the ten imports reach,
before the entry's own; `check_merged_after_aliases_with` was 54.5 million of
the row's 119.6 million instructions.

An import of a `std/` module now skips that check. The loader marks the
compile it starts for a `std/` path, and only that path: a module handed in
by the browser and the embedded `./hako` are checked as before. The entry
path's own merged check still reads every function the program holds,
shipped ones included. Planting a division by a literal zero in std/text
showed it. A program importing std/text is refused at that check whether or
not the module's own check runs.

`kanso::check_shipped` asks the skipped check for one module, and
`tests/every_shipped_module_checks_clean.rs` asks it for every module in
`SHIPPED_MODULES`. It also holds that list to the loader's table, read out
of the source, so a module added to the table cannot be skipped by every
import and checked by nothing. With the planted division the spec went red
on `std/text` with `error[value]: division by zero (module std/text)`.

Measured on this container, main and the branch built and counted here:

    compile_instructions     33,752,913 ->  26,242,711   -22.25%
    entry_instructions      120,345,927 ->  86,840,719   -27.84%
    library_instructions    120,905,216 ->  87,395,127   -27.72%
    startup_instructions        682,061 ->     659,077    -3.37%
    compile_allocs               15,341 ->      14,354    -6.43%
    front_end_rounds                 47 ->          15
    front_end_visits             15,474 ->       7,576
    interp_allocs             1,049,281 ->   1,048,350
    interp_instructions     879,786,540 -> 879,803,604   +17,064

The allocation, round and visit counts are the same on every host and move
here; the instruction rows are CI's. `compile_peak_bytes` does not move: the
front end's peak is reached later than any of the skipped checks.
`interp_instructions` rises by 17,064, two millionths, with its allocations
down 931. The interpreted run's anchor counts the run and not the compile,
so this reads as layout, the move this family of rows makes on an edit to
the compiler's own Rust.

`tests/inference_passes.rs` counted four whole-program inference passes for
its sample module and counts two now. The sample imports two shipped
modules, and each of those skipped checks carried one inference pass.

## 2026-09-24 — the beat pass classifies each group once

`beat::beat_loops` asks three passes of the same program: which groups get
a plain beat, which cycles get a cluster, and which entries are demoted.
All three read the set of groups that allocate and each group's verdict.
Neither changes between the passes, but `alloc_groups` ran five times a
build and `classify_all` three. `beat_loops` now computes both once and
hands them down; `report` does the same.

On `kanso play` of a one-line program the pass fell from 47,824
instructions to 25,105. Measured on this container, both sides built and
counted here:

    startup_instructions      693,898 ->    671,493      -22,405   -3.23%
    emit_instructions      45,678,945 -> 43,543,044   -2,135,901   -4.68%

The emitted IR of runbench, scanbench, deepbench, pendbench and escapebench
is byte-identical between the two compilers, so no runtime or code vein
moves.

The emitter's two symbol scans, `called_symbols` and `queries_named`, found
each `@` with a byte-at-a-time `position`. They use `str::find` now, which
is memchr for an ascii character. Over the change above:

    startup_instructions      671,493 ->    669,976       -1,517
    emit_instructions      43,543,044 -> 42,414,714   -1,128,330

CI's rows go into the goldens.

Two measurements from the same day are declined here so they stay
declined. Marking `d_json/escape_onto_2` `noinline`, so that
`encode_onto`'s leaf arms would skip the six-register frame the inlined
escape scan needs, took runbench from 1,856,032,701 to 1,879,000,521, 1.24%
more work. And lld's `--lto-CGO2` and `--lto-CGO1` on the release link moved
the codegen corpus's release children from 1,613,540,338 to 1,613,169,031
and 1,612,366,289. The code-generation level barely moves the link; its
cost is in the pipeline level, which was declined before.

## 2026-09-24 — a JSON number's end is found sixteen bytes at a time

lib/json found where a number ends by walking it: `scan_at` dispatched on
each byte, with arms for `.`, `e`, `E`, `+`, `-`, a digit and anything
else. The emitted loop was tight, 17 instructions a byte on the digit arm,
but runbench decodes 417,483 numbers and the loop cost 80,293,356
instructions, 23 a character.

`text/number_span cs p` answers the same question in one call: the first
position at or after `p` whose byte cannot be part of a number, negated when
a `.`, `e` or `E` went past. That sign is the float mark the walk carried as
an argument. A position outside the bytes is its own answer, as the walk
stopped there. The native runtime classifies sixteen bytes a step with SSE2
and walks what is left one byte at a time. The interpreter walks every byte,
and the wasm engine reaches the builtin through the interpreter. It is
`builtin_number_span` underneath, public in std/text the way `find2_below`
is, because the frozen decoders jsonbench and its siblings build are
generated from lib/json and a `builtin_` name is refused outside std.

Measured on this container, both sides built and counted here:

    runbench      1,856,032,715 -> 1,815,628,135   -40,404,580   -2.18%

The instruction rows are CI's. `number_spans` is the scan's presence
counter, so every cost golden gains a line, 417,483 on the run program and
zero where nothing decodes. No other counter moves. kq's veins gain the
same line, and that is kq's pin bump to absorb. The decoder's emitted code
falls: calls 1,182 -> 1,165, branches 764 -> 747, lines 8,570 -> 8,422, and
the front end's visits on the corpus 15,474 -> 15,131. Every program that
imports std/text now emits the five-line forwarder, so each row of the
emitted golden for the other benchmarks reads five lines more.

`tests/golden/micro/a_number_span_ends_where_the_byte_walk_ends.kso` puts
runs either side of sixteen and thirty-two bytes, a mark on each side of a
block edge, ends at a delimiter and at the end of the bytes, positions
outside the bytes, and a `.` past the end inside the same block. Counting
every mark in the block went red on that last case (`-4` for `4`), and
stepping one past the found byte went red on the runs (`17` for `16`).

CI's sitting for the branch that carries this change, the beat pass
classifying once and the shipped-module skip together, over main ef56eac8:

    runbench               1,813,492,695 -> 1,775,946,549   -2.07%
    jsonbench              1,196,422,558 -> 1,142,229,058   -4.53%
    compile_instructions      33,315,822 ->    25,473,385  -23.54%
    entry_instructions       118,942,141 ->    85,271,986  -28.31%
    library_instructions     119,486,941 ->    85,824,404  -28.17%
    interp_instructions      852,977,487 ->   785,998,385   -7.85%
    emit_instructions         45,206,776 ->    42,350,970   -6.32%
    startup_instructions         672,962 ->       628,645   -6.59%
    codegen_instructions_dev     287,891,869 ->   287,815,887
    codegen_instructions_release 1,614,704,366 -> 1,614,369,990
    compile_peak_bytes           777,072 ->       768,704
    interp_allocs              1,048,350 ->     1,036,127
    interp_peak_bytes            846,191 ->       837,389

One row is worse. `text`, the size of each benchmark's machine code, rises
on all fourteen, 3,256,224 -> 3,267,232 summed, 11,008 bytes. Every binary
carries the SSE2 span routine whether or not it decodes, which is 496 bytes
on the ten that never read a JSON number. jsonbench, oneshot and livebench
rise 1,536 and runbench 1,440, the routine plus its call sites. The objective does not weigh
`text`, and the runtime saving on the decoders pays for it many times over.

The first CI round was red in three places besides the goldens. The ch08
panel quoting lib/json/number.kso still showed `scan_at`, so it now quotes
`scan`, `spanned` and the two `number_done` arms. The new diagnostic,
`number_span takes bytes and a position`, had no golden; it has one in the
runtime corpus, `a_list_is_not_bytes_for_number_span`. And the ratchet's
mutation for a module rewritten twice anchored on the `let diags =` line the
skip replaced with a `match`, so it now anchors on the `false =>` arm and
inserts after the `};` that closes the match. The second round found the
two book samples that print every counter, ch10's `counters` and ch12's
`fused`, one line short: each now carries `number_spans=0`, and their panels
were rewritten from the samples.

## 2026-09-24 — the number work and the dispatch frames land together

kanso#1611 carries kanso#1605 (the number span, with the beat pass
classifying once and shipped std modules skipping their fixed check),
kanso#1608 (numbers read in place), kanso#1609 (numbers agree across
engines) and kanso#1610 (a type dispatcher's heavy arms kept out of its
frame), over main with kanso#1604. Each carried entry above stands as
written. CI's sitting on the combined tree, against main:

    runbench      1,804,051,708 -> 1,750,593,608   -53,458,100   -2.96%
    jsonbench     1,196,422,554 -> 1,127,050,463   -69,372,091   -5.80%
    livebench     2,653,048,163 -> 2,626,918,335   -26,129,828   -0.98%
    encodebench   3,179,984,475 -> 3,164,604,377   -15,380,098   -0.48%
    oneshot          19,945,518 ->    19,420,522      -524,996   -2.63%
    widebench        30,153,803 ->    29,785,857      -367,946   -1.22%

The dispatch rule reads smaller on CI than on this container, where it took
encodebench down 2.22% and livebench 4.02% by itself; CI's clang 19 had
already spent less on those frames.

Three rows are worse. `text` sums to 3,421,248, the SSE2 span routine, the
two slice doors and the text parsers in every binary, and the out-of-line
bodies the encoder now calls. `work_digestbench` reads 5,842,731 (+59) and
`work_readbench` 4,630,947 (+54); neither program touches the code that
changed, and both moved by the same few dozen instructions in kanso#1608's
own sitting.

## 2026-09-24 — what sets runbench's memory peak, and three ideas measured and declined

`arena_peak_bytes` on the run program is 5,050,064: four 1 MiB blocks and an
855,760-byte block for the index shape's subject string. Zeroing one phase at
a time names what holds them. The top-level `doc = json/decode raw` keeps two
blocks for the whole run; with `doc` decoded from `"[1]"` the floor is one.
The decode loop adds two more, because each decode starts partway into a
block; with a tiny `doc` it adds one. The pending-cell shape adds one, the
index shape adds the oversize block, and encode, deep, escape, split and
digest add nothing. Smaller blocks barely move it: 512 KiB reads
5,050,064, 256 KiB 4,787,920 and 128 KiB 4,845,568, so block rounding accounts
for at most a quarter of a megabyte of the peak.

The executed `run` is the entry's bare clone of `runbench/run`, and the cohort
license leaves a synthetic clone unbracketed, so the top-level decode has no
cohort. Lifting that exclusion as an experiment gave it one, and the pop kept
the region (`cohort_kept=1`) with the peak unchanged: the region is mostly the
tree the run goes on to use.

Declined, with the numbers:

- A shared cache for four- to seven-byte tokens in `k_b_utf8_slice_raw`, a
  thousand permanent slots that never evict. Run allocations 5,280,394 ->
  4,685,773 and `sh_str` 35,646,128 -> 16,558,928, with 596,055 tokens
  shared, but runbench rose 1,768,671,540 -> 1,777,292,093 (+0.49%) and the
  peak did not move. A token's arena path is one bump, and the lookup costs
  more than the bump it replaces.
- Keeping `escape_rest` out of line as well, so `escape_onto`'s clean path
  drops its frame: runbench 1,820,829,169 -> 1,822,724,299.
- Moving `to_float`'s strtod fallback out of line to drop its frame from the
  fast path: 106,513 instructions, 0.006%.

## 2026-09-24 — a byte view read in its frame keeps its header there

`text/bytes s` of a string borrows the string's bytes and writes a
three-word header (length, data, capacity) into the arena. JSON's
`escape_onto` makes one for every string it writes, 942,750 on the run
program, and only reads it: a length, a scan, some bytes by index, slices to
append. The header cost the bump, and every read went through memory.

`framed_views` in src/codegen.rs finds the bindings `x = bytes e` whose
header can live in the function's own frame. The function must sit on no
cycle of the call graph, so the frame is not claimed again on each pass of a
loop. Every later mention of `x` must be a read that keeps nothing: the first
argument of `length`, `find2`, `find2_below` or `slice` (a slice writes its
own header over the same bytes), the base of an index, or an argument to a
parameter that is read the same way in every clause, found as the largest
such set. A return, a list, a closure, `>>`, a binding the demand pass may
make lazy, or a call through a name the body binds all fail it. The function
then makes its tail calls as plain calls, since a tail call gives the frame
back before the callee reads the header.

The emitter frames a view only when inference has proven its argument a
string. The first build also framed views of values that might not be
strings, with a slow arm to `k_b_bytes`. The view was then a phi over a stack
pointer and an arena pointer, LLVM kept the header in memory, and the run
program fell 11,110,137 instructions where a hand edit of the IR without the
slow arm had shown 28,013,580. Proving the string removes the arm. On the
run program `escape_onto` qualifies and `regexp/in?` does not.

CI's sitting, against main:

    runbench    1,750,593,608 -> 1,717,879,328   -32,714,280   -1.87%
    livebench   2,626,918,335 -> 2,481,521,540  -145,396,795   -5.54%
    oneshot        19,420,522 ->    19,057,035      -363,487   -1.87%

and on this container runbench read 1,768,671,540 -> 1,739,715,210. Every
other row is byte-identical, and `text` falls for the three programs that
moved.

The run program's allocations fall 5,280,394 -> 4,337,644, one per escaped
string, and `sh_bytes` 31,270,680 -> 8,644,680. The live program's fall
7,539,794 -> 3,349,794.

`a_view_read_where_it_was_made_allocates_no_header` in the mem vein reads
`allocs=2` and `sh_bytes=0` for a thousand views; the ratchet row
`framed_view` empties the set and the fixture reads `allocs=1002`.
`a_view_that_outlives_its_frame_keeps_its_header` in the micro corpus makes
views that are returned, listed, captured, handed through a parameter that
returns them and rebound, two at a time at the same stack depth, and reads
them afterwards. With the analysis made to accept every view, the native
build printed invalid UTF-8 and ran out of stack.

The analysis asks only about bindings of `bytes` and the parameters a view
is handed to, and a program with no such binding pays one scan of its
top-level statements. Its first build asked about every parameter of every
group and put CI's `emit_instructions` at 45,378,600 against 42,771,526. As
built, `emit_instructions` reads 42,892,990, +121,464 (+0.28%), the analysis
on a corpus that does bind a view. `startup_instructions` reads 636,107
(+550) and `codegen_instructions_release` 1,613,900,206 (+4,754), both the
twin's text in every program. The compile, entry, library, interpreter and
dev codegen rows fall by layout.

The emitted code grows by the twin: one comment line in every program, and
its body in the four programs that frame a view. `emitted_lines` reads 8,412
and `emitted_defines` 118 for the decoder, and `emitted_other_lines` 115,794
and `emitted_other_defines` 1,731 over the other thirteen. The compile
golden's corpus rows sum to `lines` 1,505, one more each, and `module_lines`
reads 3,581.

## 2026-09-24 — a counting run builds its own binary

`kanso run` keeps each program's binary in the temp directory under a key
made of the program's IR and the runtime's digest. A run under
`KANSO_COUNTERS` emits the same IR and links a counting runtime, and the key
did not say which, so a counting run after an ordinary one ran the ordinary
binary and printed no counters. The mem vein runs its fixtures that way, and a
fixture run once by hand before its golden existed regenerated as the
sentence saying the binary had no counters. The key now carries a mark for
a counting build.

`tests/a_counting_run_builds_its_own_binary` runs a program nobody else runs,
once plainly and once counting, and reads the counters on the second. It
failed on main with an empty stderr and passes with the mark. The compiler's
own layout moves, so the compile-side rows are CI's to report.

`counters_wanted` is now read once and kept. The emitter, the runtime
object's key and the program binary's key each asked, and each ask walks the
environment. The one place that sets the flag, `--counters`, does it while
parsing the arguments, before anything asks. On this box `startup_instructions`
reads 643,462 on main and on the keyed build, and 643,146 read once.

CI's rows with both commits: `compile_instructions` 25,432,497 -> 25,395,488,
`entry_instructions` 85,149,814 -> 85,032,735, `library_instructions`
85,700,954 -> 85,584,660 and `emit_instructions` 42,892,990 -> 42,866,674,
each a read of the environment the emitter no longer repeats.
`startup_instructions` reads 636,119 against 636,107, a rise of 12
(+0.0019%), which is the layout of a binary that moved; this box read that
row 316 lower. The codegen and interpreter rows read as main's.


## 2026-09-24 — a long slice shares its string's bytes, and an oversize block leaves its neighbour open

`text/slice` of a string now returns a header whose `data` points into its
parent when the slice is sixty-four bytes or more, where it used to copy the
bytes. A parent with room (`cap > 0`) is a builder, whose storage moves and
is freed as it grows, so its slices are still copies. Every other string's
bytes live exactly as long as the string does, and evacuation copies a view's
own bytes and nothing around them, so a view never holds more of its parent
than the parent would have held alone.

A view does not end in a terminator, because the byte after it is its
parent's. The runtime reads a terminator only where it hands a string to the
C library: the three `fopen`s, both `stat`s, `getenv`, `opendir`, the argv of
both process spawns, the directory sort's `strcmp` and the runtime's own
error sentences, which print with `%s`. Each now goes through `k_cstr`, which
returns the data when it is terminated and a terminated copy otherwise. The
two copies evacuation makes wrote `len + 1` bytes, taking the terminator
along; they now write `len` and a zero.

On the run program the slice alone takes the arena peak 5,050,064 ->
4,718,608 and `arena_blocks` 6 -> 5; with the split below it reads 4,194,304. `alloc_bytes` falls 413,717,453 -> 412,321,053,
`sh_str` 35,646,128 -> 34,249,728 and `str_scans` 163 -> 161, because a view
of a slice whose character count is known carries the count. basket and scan
fall by a slice each. The view's length test first sat ahead of the ascii
path's one-character case, which the ascii cache answers, and CI read
scanbench 417,133,247 -> 421,136,287 (+4,003,040): a matcher slicing one
character at a time, 1,001,004 slices, four instructions each. The
one-character case is now asked first. On this box scanbench then reads
441,704,194 -> 440,702,231 (-1,001,963) and runbench 1,739,715,210 ->
1,738,092,378 (-1,622,832), with the split below included.

The peak's makeup was read by printing the live blocks at each new peak.
Before, the top was 1,380,032 + 1,572,880 + 1,048,576 + 1,048,576: the index
shape's slice, the subject it was cut from, and two ordinary blocks. After, it
is 1,572,880 and three ordinary blocks. The page's section 133 said the old
peak was four 1 MiB blocks and one block of 855,760 bytes. That is the right
total and an impossible shape, because a block is 1 MiB or it is exactly one
allocation larger than 1 MiB. The section is corrected.

With the slice shared, the index shape's top was the subject's block and a
fresh 1 MiB block pushed straight after it, while the block before the
subject still had room. An allocation larger than a block gets a block of
exactly its size, and that block became the bump region with nothing left in
it, so the next small allocation opened a new one. Now, when the current
block has 4 KiB or more left, `k_alloc_oversize` splits that tail off as a
block of its own and pushes it back above the oversize block to serve what
follows. The host shrinks to the part in use, so no two blocks cover the same
bytes and every walk over the chain reads it as before. A rewind that pops a
tail gives its bytes back to the host, and never to the spare list, since
malloc never handed them out. `KBlock` gains the `host` pointer and a pad, 32
bytes where it was 16, which moves the arena that follows it by sixteen bytes
and costs nothing a block holds.

The split alone moves nothing on main: there the two oversize allocations
are the top, and both land above the same tail. Over the shared slice it takes
the run program's arena peak from 4,718,608 to 4,194,304. That is what the run
reads with the index shape cut to one character, so the index shape no longer
sets the peak.

Specs: `tests/golden/mem/a_long_slice_shares_its_text` reads alloc_bytes
16,464 for a thousand slices of ninety-eight characters, one ascii and one
walked, and 128,464 with views off. `tests/golden/runtime/
a_long_slice_ends_where_it_was_cut` prints a sixty-four-character slice
through an error sentence, and with `k_cstr` handing over the parent's bytes
the sentence runs on into the rest of the line.
`tests/golden/mem/an_oversize_string_leaves_its_neighbour_open` doubles a
string to 2,097,152 bytes and prints a sentence after it: its arena peak reads
3,145,744, one block and the string, and 4,194,336 with the split off. The
ratchet carries all three as mutations.


## 2026-09-24 — a greedy run of one character is counted, then backed off

std/regexp walks a pattern by continuations. A repetition takes one more of
its body by walking the body with a continuation that asks for the next one,
so every character a greedy `[a-z]+` takes costs a closure capturing eight
values, a call into it and a call back into the repetition. The run
program's scan phase runs `[a-z]+zzq` over a subject that never contains
`zzq`, and at every start position the repetition takes the rest of the
subject and then gives it back a character at a time. That was 91,806
steps at about a thousand instructions each, 5.4% of runbench.

When the body is one character, a literal, a dot or a class, none of that is
needed: each character it takes moves the position by one and captures
nothing. `walked` now counts such a run in one pass, stopping at the
repetition's ceiling, and offers the rest of the pattern the longest run,
then one shorter, down to the floor. That is the order the general walk
tries lengths in, so the matches are the same. Lazy repetitions, and any
repetition of a group, a sequence or an alternation, still take the general
walk. The class test also binds the set's bytes once instead of building
them inline.

On this container, against main:

    runbench    1,739,715,210 -> 1,710,209,853   -29,505,357   -1.70%

`a_greedy_run_of_one_character_backs_off_in_order` in the micro corpus runs
sixteen patterns that give characters back, hit floors and ceilings, match
nothing, stop a dot at a newline, negate a class, capture on either side of a
run, meet an end anchor, and repeat lazily or over a group. Every engine
prints what the library printed before the change. The ratchet row
`counted_run` drops the count's ceiling, and `x{2,3}x` on "xxxxx" takes five
characters where it takes four.

The library is compiled into every program that imports it, and the new
functions are code in the two that scan: `emitted_other_defines` reads 1,747,
`emitted_other_calls` 18,794, `emitted_other_branches` 11,468 and
`emitted_other_lines` 117,099, all of it scanbench and runbench.

## 2026-09-24 — a map whose pairs are in order is its own view

A map keeps its pairs in the order they were put and builds a sorted,
deduplicated view the first time something reads it. The view is a copy of
the pairs in a malloc'd buffer, held for as long as the map lives, and it is
what `held_peak_bytes` measures on the run program: 728,040 bytes, the views
of the top-level document's 2,761 objects, which the encoder reads ninety
times.

Every one of those objects had its keys put in ascending order with none
repeated, so every view was a copy of pairs that were already sorted. The
view build now checks that first, n - 1 key comparisons, and when it holds it
points the view at the pairs. The paths that write through a view were
already few:

- a replace of a key already present patches the value in place, which is
  the same slot in both;
- the in-place put appends a pair and then inserts it into the view, and for
  an alias it extends the view when the new key sorts last and otherwise
  turns the alias into a copy before inserting;
- the in-place put's growth path moves the pairs to a bigger buffer, and
  moves an alias with them;
- the registry flush and the carry path free a view, and both now ask whether
  it is one.

Pairs are frontier-shared between maps, but a map that shares a buffer sees
only its own prefix, and no append changes a prefix.

On this container, against main:

    runbench         1,768,671,540 -> 1,763,871,501   -4,800,039   -0.27%
    held_peak_bytes        728,040 ->       416,312     -311,728
    view_allocs              2,761 ->             0

`arena_peak_bytes` does not move. What `held_peak_bytes` still holds is the
encoder's output builders.

The copy an out-of-order insert makes is sized to 1, 3, 7, 15, the series a
view built at the first read would have reached by then. The first build
dropped the alias there and let the next read build a view at its exact size,
and doubling from an exact size overshoots the series: `growing_map`, 800 keys
in descending order, held 73,696 bytes of view where main holds 49,120, and
`fused_tally` 10,720 against 7,744. Sized to the series, no counter in any
golden rises.

`a_transient_maps_view_is_freed` exists to show a transient map's view being
freed, and its keys were put in order, so it now built no view to free. Its
seed takes `b` and the put takes `a`, and it reads what it read on main.

`a_map_whose_keys_arrived_in_order_is_its_own_view` in the mem vein reads
`view_allocs=0` and `held_peak_bytes=0` for a thousand ascending keys; the
ratchet row `ordered_view` copies the pairs into a view again and the fixture
reads one view of 32,016 bytes. `a_map_read_in_order_shares_its_pairs` in the
micro corpus reads maps between writes that keep the order, break it, repeat a
key, outgrow the first buffer and share another map's pairs, and every engine
prints the same maps. With the insert made to extend an alias whatever the
new key, the native build printed a descending map in the order it was put.

## 2026-09-24 — a builder only one function holds grows by realloc

A builder grows by doubling, and a grow at a site the linearity analysis
proved unique took a new buffer, copied the old one into it and freed the
old one. That is what `realloc` does, and glibc does it better: it extends
the block in place when the space after it is free, and past its mmap
threshold it remaps the pages instead of copying them. The unique grow now
calls `realloc`, counted as the malloc and the free it replaces so that
`bytes_malloc`, `bytes_freed` and `allocs` keep their meaning. The old and
new buffers are also never both held, and the run program's
`held_peak_bytes` was taken at exactly that moment, when the encoder's
output builder went from about 128 KB to about 256 KB.

Carried with the ordered view in the same pull request. On this container,
each against main before either:

    ordered view alone  runbench -4,800,039   held_peak_bytes 728,040 -> 416,312
    realloc alone       runbench -20,873,318  held_peak_bytes 728,040 -> 589,266

and the two together, against main with kanso#1613:

    runbench         1,739,715,210 -> 1,710,701,594   -29,013,616   -1.67%
    held_peak_bytes        728,040 ->       277,538     -450,502

Most of the instruction saving is the memcpy that the remap skips.

`a_builder_that_outgrows_its_buffer_is_never_held_twice` in the mem vein
appends 100,000 bytes to a unique builder and reads `held_peak_bytes=135182`,
the last buffer alone. The ratchet row `regrow` sends the grow back through
malloc, copy and free, and the fixture reads 202,780, the last two buffers.

The same grow decides where the new buffer lives by asking whether the
builder's header dies at the innermost rewind. It asked that as two walks of
the block chain, whether the header is live and whether it is at or below the
mark, and the second walked every block under the mark. `k_above_mark` asks
the one question with a walk that stops at the mark's block, and gives the
same answer for every pointer. On the 176,697 grows of the run program that
is 2,527,440 instructions, 1,710,701,594 -> 1,708,174,154, and every counter
vein agrees with the goldens.

CI's sitting of the three together, against main with kanso#1613:

    runbench      1,717,879,328 -> 1,686,535,157   -31,344,171   -1.82%
    livebench     2,481,521,540 -> 2,370,527,779  -110,993,761   -4.47%
    encodebench   3,164,604,377 -> 3,055,075,047  -109,529,330   -3.46%
    oneshot          19,057,035 ->    16,662,299    -2,394,736  -12.57%

Three rows rise. `work_basket` reads 32,567,631 (+220,378) and
`work_jsonbench` 1,127,375,513 (+325,050), and on this container both rises
are `k_b_put_mut`: basket's 9,077,621 -> 9,400,908, jsonbench's +171,000. The
in-place put now asks whether a map's view is an alias before it inserts
into it or grows it, which is a compare on every put into a map that has a
view. `work_widebench` reads 29,833,857 (+48,000). `text` sums to 3,443,664,
1,648 bytes more a binary for the alias paths and the regrow, and the two
codegen rows rise with it: `codegen_instructions_dev` reads 287,845,497 and
`codegen_instructions_release` 1,614,602,673.

## 2026-09-24 — four more ideas measured and declined

Each of these was built and measured on the run program, and each gave back
less than it cost.

- Parsing a number's digits eight at a time. The float and integer parsers
  took fast_float's eight-digit conversion, with the digits shifted up so the
  unused lanes read as leading zeros. runbench rose 1,768,671,540 ->
  1,772,691,930 (+4,020,390): `k_b_to_float_slice` 41,479,317 -> 45,289,035
  and `k_b_to_int_slice` 21,977,802 -> 22,188,474. A chunk costs about
  thirty-five instructions, and the run's numbers have three or four digits
  either side of the point, which the byte loop reads for about nine a digit.
  It would pay on runs of five digits or more.
- Giving a list four slots on its first push instead of eight. runbench rose
  13,825,862 (+0.78%) and `sh_buf` fell 108,745,936 -> 101,402,528, but
  `arena_peak_bytes` stayed at 5,050,064. The peak is counted in 1 MiB blocks
  and the saving drops none of them.
- Arena blocks of 256 KiB instead of 1 MiB, on the tree of kanso#1614.
  runbench rose 1,710,701,594 -> 1,722,748,393 (+0.70%) and
  `arena_peak_bytes` fell 5,050,064 -> 4,787,920. Scored by the objective's
  marginals that is about +0.06 for memory against -0.04 for instructions,
  and the memory side is where this program's live set happens to fall
  against a block boundary: 512 KiB reads the same peak as 1 MiB.
- Marking bytes known to be valid UTF-8 so that `utf8` of them skips the
  check, on the tree of kanso#1614, where runbench reads 1,708,174,154. A
  fourth header word held a magic value when the bytes were a string's view,
  a slice of known bytes cut at two character boundaries, or a builder that
  had been appended only known pieces and ascii bytes. Skipping every check
  outright reads 1,673,709,248, so -34,464,906 was the most it could give.
  Tracked through every append, `k_b_utf8` fell 57,584,088 -> 11,241,558 but
  runbench rose to 1,738,916,203 (+30,742,049). The byte arms paid a test per
  byte (`str_char` +10,361,142, `escape_onto` +9,750,330), the counts the
  check had seeded were taken later by `k_str_chars_scan` (+13,280,220), and
  the larger arms pushed `encode_onto` out of its callers (+35,175,886 over
  the encoder's four functions). Tracked only on views and their slices, with
  every owned buffer unknown so that no append writes more, runbench rose to
  1,721,492,961 (+13,318,807). Of what `utf8` spends on this program, the
  builders' checks are the part that costs, and the decoder's tokens, which
  a slice could have vouched for, are ascii of four to seven bytes: checking
  one costs less than proving its two ends.

## 2026-09-24 — four pull requests carried together

kanso#1614, kanso#1615, kanso#1617 and kanso#1618 each touched the log, and
three of them the runtime and the ratchet's row list, so each would have
merged main again behind the one before it and waited on a ratchet that takes
over an hour for a runtime branch. They are carried here over main at
kanso#1616 and land together.

The conflicts were the log, where every entry is kept; the ratchet's row
list, where every row is kept and the lists chain `rows_b1q`, `rows_b1p`,
`rows_b1v`; the page, where kanso#1614 and kanso#1617 had each written a
section 135, and the second is now 136, with section 133's pointer to it
moved; and the cost goldens and the floor, which were regenerated rather than
merged.

The counter veins, regenerated over the combined tree, move three files. On
the run program `arena_peak_bytes` falls 5,050,064 -> 4,194,304 and
`held_peak_bytes`, which kanso#1614 took from 728,040 to 277,538, holds
there, so `run_peak_bytes` reads 4,492,354 against main's 5,798,616.
`allocs` falls 4,337,644 -> 4,147,652 and `alloc_bytes` 413,717,453 ->
397,171,773.

CI's reading of the combined tree. runbench falls 1,686,535,157 ->
1,655,310,739 (-31,224,418, -1.85%) and scanbench 417,133,247 -> 296,418,691,
the regexp's counted run. jsonbench, oneshot, basket, deepbench, indexbench,
digestbench and livebench fall too. Five rows rise by what a runtime this
size moves in layout: `escapebench` lands at 76,459,452 (+2,998),
`pendbench` 209,069,804 (+4,181), `readbench` 4,631,756 (+809),
`encodebench` 3,055,075,201 (+154) and `widebench` 29,833,954 (+97). Every
program's `text` grows 2,432 bytes, the runtime's own growth, and runbench's
`text` lands at 415,784 and scanbench's at 323,672, which also carry the
regexp's larger helpers. `entry_instructions` lands at 86,461,385
(+1,428,650, +1.68%) and `library_instructions` at 87,004,317 (+1,419,657,
+1.66%), which is kanso#1615's larger regexp library compiled on each of
those routes; its own rows were never taken before the carry.
`codegen_instructions_dev` lands at 287,912,357 (+66,860, +0.02%), and
`codegen_instructions_release` falls to 1,614,559,051. The compile, emit,
start-up and interpreter rows read as kanso#1618 left them.

By the trend gate's keys: `work_escapebench` lands at 76,459,452,
`work_pendbench` at 209,069,804 and `work_readbench` at 4,631,756, the layout
rises above. `work_basket` reads 32,561,464 and `work_jsonbench`
1,127,061,575, each below main's golden and above the history's reading from
before kanso#1613, whose rise is priced in its own entry. `text` sums to
3,486,864 against main's 3,420,592 (+66,272): 2,432 bytes of runtime in each
of the fourteen programs and the regexp's helpers in the two that match.

## 2026-09-24 — an arm no value reaches is not emitted

std/list's `next` has an arm for every lazy adapter the library declares:
bounded, capped, counting, cursor, cycled, grown, mapped, paired, repeated,
sifted, skipped. A program that maps once reaches `next`, so it emitted every
arm and everything each arm calls. The codegen corpus is sixty-two lines that
map and fold, and it emitted 4,628 lines of IR, 1,560 of which were
adapters it never builds and what they call.

`without_unbuilt_arms` drops an arm before anything is emitted when a record
type in its pattern, at any depth, has no value in the program. A value of a
declared type exists only if an expression names the type -- a construction,
a partial, a constructor passed on, an upcast to it -- or if the runtime
builds it, and the runtime builds one record type, `entry`, id 0, in
`entries`. A subtype's value matches its parent's patterns, so building one
builds its ancestors. Counting names over every declaration found every
adapter built, because std/list declares a builder for each, so reachability
and construction are one fixpoint: a group is reached when a live arm names
it, an arm is live when its group is reached and nothing in its pattern is
unbuilt, and a live arm's body builds what it names. The roots are the entry,
every constant, and the groups the emitter calls without the source naming
them, which are the renderer and the user operators. A group whose every arm
would go keeps them all, so a call that reaches it fails as it did. The
interpreter is untouched, and the golden, micro and differential corpora all
pass.

On the codegen corpus the IR falls 4,628 -> 3,068 lines (172,246 -> 117,491
bytes). The child tree of a dev build on this box reads 287,264,761 ->
200,819,456 instructions (-30.1%) and of a release build 1,613,493,611 ->
856,255,424 (-46.9%). The compile golden's module row falls 3,581 -> 2,181
lines and 45 -> 35 defines, and seven benchmarks emit less:
`emitted_other_lines` for encodebench 9,640 -> 8,242, widebench 10,623 ->
9,225, deepbench 4,284 -> 2,886, pendbench 5,330 -> 4,260 and digestbench
8,087 -> 6,688. Over the carried tree of kanso#1619, whose regexp helpers
scanbench and runbench also emit, scanbench falls 18,930 -> 17,524 and
runbench 33,871 -> 33,009. The decoder's
own golden and every runtime counter are unchanged.

CI's rows, over the carried tree of kanso#1619. `codegen_instructions_dev`
falls 287,912,357 -> 201,466,586 (-30.03%) and
`codegen_instructions_release` 1,614,559,051 -> 857,150,087 (-46.91%).
`emit_instructions` falls 42,866,674 -> 35,545,509 (-17.08%), since there is
less to write. The pass itself costs little: `compile_instructions` lands at
25,396,458 (+970), `entry_instructions` at 86,465,684 (+4,299),
`library_instructions` at 87,008,618 (+4,301), and `startup_instructions` at
639,966 (+3,847, +0.60%), each a clone of the program and a walk of its
bodies in a build that emits. On the run side encodebench falls 13,099,200
and pendbench 6,198. Three rows rise: `work_deepbench` lands at 367,712,368
(+1,296,000, +0.35%), `work_basket` at 32,576,428 (+14,964) and
`work_digestbench` at 5,842,662 (+14), which is the dispatch of groups that
lost arms laid out and inlined differently. Every program's `text` falls, by
5,648 to 9,632 bytes, and `text` sums to 3,420,864 against the carried tree's
3,486,864 (-66,000).

Spec: `tests/an_arm_no_value_reaches_is_not_emitted` builds a program that
maps and one that drops. The first must not define `d_list/next_skipped_2`,
the second must, and both must print what the interpreter prints. Watched red
with the pass returning nothing: the mapping program defined it. The ratchet
carries the mutation.


## 2026-09-24 — a cycle nothing reaches is not emitted, nor a string nothing names

`prune_unnamed` struck a definition from the emitted module once no other
surviving definition named it. That is reference counting, and a cycle names
itself. std/list sorts by merging, and the merge is four functions calling
round: `merge`, `merge_on`, `pick` and `advance`. A program that imported
std/list and never sorted struck `sort`, `msort` and `span`, then kept the
four, because each was still named by the one before it, along with `drain`,
which only they call. The same shape kept std/list's window helpers
`bounded_flat` and `bounded_more` once the arm prune had dropped the arms that
call them, and std/text's two trimming walks, `from_front` through
`past_the_end` and `step_in` back to itself, and `from_back` with
`step_back`.

The prune now marks. The roots are the entry, `d_thunk_eval`, and every block
that is not a candidate. A block is kept when the mark reaches it by a name
some live block writes, or by a closure cell a live block loads. The oracle in
the unit tests is the same mark written the slow way, one `names_symbol`
search per question, and a new unit test builds a two-block cycle nothing
names and asserts both go.

On the codegen corpus the IR falls 3,068 -> 2,073 lines. The child tree of a
dev build on this box reads 200,891,196 -> 155,302,680 instructions (-22.7%)
and of a release build 856,351,463 -> 718,270,625 (-16.1%). In the dev tier,
`clang -cc1` takes 111,055,320 of that, and 35,541,211 of those are clang
compiling an empty module, so the fixed floor is about a third of the
compile. The compile golden's module row falls 2,181 -> 1,488 lines and 35
-> 28 defines. The decoder's emitted lines fall 8,412 -> 6,849 and 118 -> 84
defines. Every other benchmark emits less: encodebench 8,242 -> 7,330, oneshot
8,347 -> 8,104, widebench 9,225 -> 7,781, deepbench 2,886 -> 2,159, pendbench
4,260 -> 3,544, scanbench 17,524 -> 15,765, indexbench 986 -> 766,
digestbench 6,688 -> 5,776, readbench 1,089 -> 847, livebench 8,486 ->
8,243 and runbench 33,009 -> 31,460. The twelve cost veins and the lazy tier
are unchanged.

Spec: `tests/a_cycle_nothing_reaches_is_not_emitted` builds a program that
sums a list and one that sorts it. The first must not define
`d_list/merge_5`, the second must, and both must print what the interpreter
prints. Watched red against the counting prune: the summing program defined
the merge. The ratchet carries the mutation, which starts every block marked.

The strings that pruned functions interned are left out too. A string is
interned when the emitter reaches a literal or an err site, and the function
that asked for it may be pruned afterwards. On the codegen corpus, after the
mark, 198 of 270 strings and 264 of their literal cells were named by nothing:
31,191 of the module's 85,984 bytes. Each is now emitted only when the body or
a type table names it, read off by `unquoted_globals` in one pass. The corpus
IR falls again, 2,073 -> 1,611 lines, the module row 1,488 -> 1,149, the
decoder 6,849 -> 6,410, and runbench 31,460 -> 29,993. Every other benchmark
falls with them. `tests/a_string_nothing_names_is_not_emitted` builds a
program that sums a list and asserts every string and literal cell left in its
module is named somewhere else. Watched red with every string emitted: 328
were named by nothing. The ratchet carries the mutation.

A dispatcher stops at the arm that leaves no way into the next. The emitter
opens arm k's `fail` block before it knows whether any parameter check will
branch to it, and when every check is proved away nothing does, so the arms
after it and the whole failure path were blocks nothing reached. They were 7%
of the corpus's IR, 9% of runbench's and 13% of the decoder's, and clang
parsed each one only to delete it in its first pass. Dropping unreached blocks
after the fact, a pass over every function body, took the corpus to 1,480
lines but cost the emitter more than clang saved: `emit_ir_for` on this box
rose 33,539,504 -> 37,473,772. That pass is declined. Almost every such block
has the one shape, so the dispatcher now asks once per arm whether its text
branches to `fail{k}` and stops writing when it does not. The corpus IR is the
same 1,480 lines, and `emit_ir_for` falls to 30,538,357 (-8.9%). The decoder's
emitted lines fall 6,410 -> 5,406 and runbench's 29,993 -> 26,033;
encodebench, oneshot, widebench, livebench, scanbench, digestbench and the
rest fall with them. The compile golden's module row falls 1,149 -> 1,079, and
its five samples fall between 14 and 34 lines each, the string filter's share
included; `guards` loses a define, a function whose one caller sat in a dead
failure path. `tests/a_block_nothing_branches_to_is_not_emitted` builds a
program with a dispatcher whose checks are proved away and asserts every block
in its module is reached from its function's entry. Watched red with the early
close taken out: 22 blocks nothing reached, the first of them
`d_list/fold_3`'s `fail2`. The ratchet carries the mutation.

An unboxed parameter is read as its word. An integer parameter the escape
analysis unboxes crosses as `i64 %xNr` and is boxed on entry, and every read
of its tag or payload took the box back apart with an `extractvalue`, though
the tag is 0 and the payload is the argument. `FnEmit` now records the two
words a boxed parameter was built from, `inline_tag` and `inline_payload`
answer from the record, and a box that nothing else reads is left out of the
body. The corpus IR falls 1,480 -> 1,453 lines, the decoder's 5,406 -> 5,211
and runbench's 26,033 -> 25,224. On this box the dev child tree reads
144,052,165 against 144,536,585 and the release tree 713,344,643 against
713,946,965, and `emit_ir_for` rises 197,598 (+0.65%) for the scan that finds
an unread box. The two development terms share a satiation, so at their
current ratios a per cent of dev codegen is worth about eight of emitting, and
the trade comes out ahead. `tests/an_unboxed_parameter_is_read_as_its_word`
asserts that no function in a module extracts a word from a boxed unboxed
parameter. Watched red with the words unrecorded: nine reads took a parameter
back apart. The ratchet carries the mutation.

A switch dispatcher writes its `nomatch` only when a case falls to it. A
switch whose cases cover every value its discriminator can hold never does,
and its failure path was the 13-line residue left in std/json's byte switches
after the early close. The decoder's emitted lines fall 5,211 -> 5,104 and
runbench's 25,224 -> 25,117. The block spec now decodes a string with an
escape in it, which reaches those switches, and watched red with `nomatch`
always written: 18 blocks nothing reached, the first of them
`d_json/str_char_4`'s. The ratchet carries the mutation.

A tag already known to be the int tag is not compared with it. Arithmetic on
two values asks whether both tags are 0 before it takes the fast path, and a
literal's tag is known, so `n + 1` wrote `icmp eq i64 0, 0` and an `and` on
every addition; a byte index with a literal key did the same beside its bytes
test. `both_ints` skips a known tag and answers `true` for two. The decoder's
emitted lines fall 5,104 -> 5,063 and runbench's 25,117 -> 24,854.
`tests/a_tag_known_to_be_int_is_not_compared` asserts that no line in a module
compares two constants, and watched red with every tag compared: five lines
compared 0 with 0. The ratchet carries the mutation.

The string filter reads names by number. It collected every unquoted `@name`
in the body and the type tables into a hash set and asked the set about `sN`
and `sN_lit`, which are the only names it ever asks about, and `intern` names
string N `sN`. `named_strings` reads each `@s` name as a number into a pair of
flags. On `kanso play`'s start-up for a one-line program the set was 23,334
instructions, and `Backend::emit` falls 339,769 -> 325,775 on this box with
the module unchanged. The mutation for the string spec now marks every string
named, and watched red again: 239 strings and cells that nothing named.

Reading every box built from words as its words, rather than only the unboxed
parameters, was measured and declined. `FnEmit::write` recorded each line of
the boxing shape, and runbench's extracts from such boxes fell 1,130 -> 18,
but the record is a test and an allocation on every line the emitter writes:
on this box `emit_ir_for` rose 30,613,012 -> 31,136,898 (+1.7%) while the dev
child tree fell 45,406 and the release tree rose 337,442. clang at `-O0` folds
an extract of an `insertvalue` for nearly nothing, so the lines were cheap to
keep.

The interpreted row rose 4,189,569 between two sittings of this branch that
changed only the emitter, and the whole rise was one collect in `call_named`:
the arguments to a builtin were forced through a `Result` adapter, and rustc
stopped inlining the adapter's fold into it, laying out as a call what had
been inline. The argument forcing is now a loop that forces a thunk where it
lies and leaves every other value alone, which is what `force_thunk` did with
them anyway, so there is nothing left for the layout to decide. The same shape
in `call_builtin`, a map through `sub_base` into a fresh collect, now rewrites
only a subtype's slot. And `low_byte` read a number's first byte by writing
out all of its bytes with `to_bytes_le`; it now reads the magnitude's lowest
word, which allocates nothing and answers the same byte, the magnitude's, for
a negative number as it did before. On this box the interpreted corpus reads
846,937,172 -> 818,016,245 -> 802,836,171 -> 790,883,683 across the three.

Moving a dispatch's arguments into the winner's bindings rather than cloning
them was measured and declined. `match_one` clones 620,601 values on the
interpreted corpus and the dispatcher drops its own copies once the winner is
known, so binding a parameter matched whole to a placeholder and moving the
argument in afterwards looked like most of `Value::clone`. It read 790,883,683
-> 790,646,809 on this box: nearly every one of those clones is a field bound
out of a record pattern, which the record still holds, and a parameter matched
whole is a small share of them.

CI's rows, over kanso#1620's. `codegen_instructions_dev` falls 201,466,586 ->
144,314,284 (-28.37%) and `codegen_instructions_release` 857,150,087 ->
713,520,095 (-16.76%). `emit_instructions` falls 35,545,509 -> 30,190,261
(-15.07%) and `startup_instructions` 639,966 -> 601,897 (-5.95%), since `kanso
play` emits the program to key its binary cache. `interp_instructions` lands
at 733,050,032 (-6.69%): the two loops in `call_named` and `call_builtin` more
than take back the 4,189,569 the collect's layout had cost.
`compile_instructions` lands at 25,460,259 (+0.25%), `entry_instructions` at
86,678,016 (+0.25%) and `library_instructions` at 87,221,497 (+0.24%), which
is the layout of a compiler whose emitter and interpreter both changed; `kanso
check` reaches neither. Every runtime row and every `text` row is unchanged,
and so is every allocation counter but one: `interp_allocs` falls 1,036,127 ->
983,321, by 52,806, which is `low_byte`'s call count, one vector each.

Two dev-tier leads were measured and declined. At `-O0` FastISel selects
none of the corpus: it refuses `insertvalue` on `%KValue`, the aggregate
argument and the aggregate return, so every function falls back to
SelectionDAG, which is 34,446,800 of `clang -cc1`'s 111,055,320. Passing a
value as two words is a change to every signature the emitter writes and every
runtime entry point, which is too big a change to take on here. GlobalISel
(`-mllvm -global-isel`) segfaults on the module under LLVM 18.

Most of each tool's fixed cost is the dynamic loader relocating LLVM's shared
libraries. `clang -cc1` on an empty module reads 35,541,211 instructions,
26,069,125 of them in `_dl_relocate_object` for libclang-cpp and libLLVM, and
`ld.lld` linking an empty object reads 40,530,158, 17,699,276 of them the same
way. A dev build pays both, about 44 million of its 145.7 million. Running
`llc -O0 -disable-verify` in place of `clang -cc1` loads libLLVM alone and
reads 95,768,077 on the corpus against 102,053,958, but it writes different
unwind tables and relaxation, and Xcode's clang ships no `llc`, so the dev
tier would carry two back ends for about 4% of one row. Declined.

## 2026-09-24 — a function's overflow checks share one trap

Every `+`, `-` and `*` the emitter proves to be between two integers is an
`llvm.*.with.overflow` call and a branch on its flag, and the block that
branch takes on overflow is three lines: a call to `k_die` with the overflow
message, and `unreachable`. It was written once an op. `d_json/str_low_5`
carried six copies of it, and runbench's module 131 in all. Each function now
keeps one, made the first time an op asks for it and written by `body` after
everything else, so every op in the function branches to the same label.

The mixed path, where one side might not be an int, is unchanged. Its
overflow falls to the slow call, which reports the overflow itself.

The emitted veins fall: the decoder's calls 629 -> 582 and lines 5,063 ->
4,922, runbench's calls 4,296 -> 4,171 and lines 24,854 -> 24,479, and every
other row by the number of checked ops it holds. `compile_golden`'s recursion
sample loses a call and three lines. Defines and branches are unchanged
everywhere, since the op still branches and no function was added.

On this container, against kanso#1621's tree: `codegen_instructions_dev`
143,741,446 -> 143,725,457, `codegen_instructions_release` 713,139,845 ->
712,729,450 and `emit_instructions` 30,468,658 -> 30,458,722. The codegen
corpus holds few proven-int ops, so the rows move little. The run program
reads 1,676,858,251 -> 1,676,080,238 (-0.0464%) and jsonbench 1,127,903,599
-> 1,126,719,499 (-0.1050%), with the cold blocks at the end of each function
rather than between its hot ones. scanbench reads +994. CI's rows follow.

`tests/an_overflow_trap_is_written_once_a_function.rs` compiles a function
with six checked ops and asserts it calls `k_die` with the overflow message
once, and that no function in the module calls it twice. A second test runs
the same function on numbers that do not fit: native dies with the overflow
message and prints nothing, and the interpreter answers
9214148664817920226007. Watched red with every op taking its own trap:
`spread` held 6. The ratchet row `one_trap` breaks it the same way.

Returning through one shared block was measured and declined. With every
`ret %KValue` in the codegen corpus rewritten to branch to a block that joins
the two words in `i64` phis and returns once, `clang -cc1 -O0` read
104,633,266 -> 107,801,878. FastISel misses 110 instructions on the original
module and 119 on the rewrite. A block whose terminator FastISel misses goes
to SelectionDAG whole, and once the returns stop doing that, each call that
returns a `%KValue` misses on its own and is selected alone, which costs
more.

## 2026-09-25 — blank lines are counted by binary search

`check_blank_policy` asks, for every pair of adjacent lines, how many blank
lines lie between them, and it answered by filtering the file's whole list of
blank lines each time. That is quadratic in the length of a file, and it was
`parser::parse`'s own largest loop: 473,034 blank-line reads on the entry
corpus, whose ten imports are parsed as modules first. The lexer records blank
lines in the order it meets them, so the list is ascending, and two
`partition_point` calls now bound the run. The continuation check's
`contains` on the same list becomes a `binary_search`.

On this container, with kanso#1621 and the overflow-trap change beneath it:
`compile_instructions` 25,943,273 -> 25,526,442 (-1.61%),
`entry_instructions` 88,243,804 -> 84,889,198 (-3.80%) and
`library_instructions` 88,801,286 -> 85,446,623 (-3.78%).
`startup_instructions` reads 611,469 -> 611,436. CI's rows follow.

Nothing a program does changes, and the error corpus pins the diagnostics
the count feeds: `blank_line_in_body`, `missing_blank_between_decls`,
`partial_chain` and `a_dot_continuation_under_its_statement`. Watched red with
the count off by one, which fails the error corpus and every corpus whose
files have blank lines. The two boundaries are always non-blank lines, so
moving either comparison between `<` and `<=` changes nothing, and a
mutation that does only that stays green.

## 2026-09-25 — `entries` takes one block for its records, buffer and list

`k_b_entries` already put its n records in one arena block. The item buffer
and the list header were two more bumps, each through `k_alloc`, and the
buffer first asked the free list for a size class that a three-pair map never
has. They now sit in the same block, after the records, unless the free list
holds a buffer of exactly the size wanted, in which case that buffer is used
as before. A buffer outgrown later goes to the free list at the capacity its
header records, which stops short of the list header behind it.

On this container: runbench 1,676,080,238 -> 1,674,340,808 (-0.1038%),
encodebench and livebench -7,730,800 each, oneshot -19,327, every other
benchmark unchanged. `allocs` falls by two a call and nothing else moves: the
run program 4,147,652 -> 3,650,672, encode 3,344,272 -> 1,135,472, and the
`.mem` rows for a map walk 3,004 -> 1,004. The bytes are the same bytes, so
every peak row holds.

## 2026-09-25 — a beat's mark is taken inline

`k_beat_push` joins `k_beat_iter` in the release build's hot unit, the
bitcode the LTO link inlines into the program, so a beat loop's entry takes
its mark without a call. The over-deep case, a push past the stack's
sixty-fourth mark, keeps a call of its own in `k_beat_push_deep`. To reach
the hot unit, `k_carries` and `k_live_block_bytes` lose their `static`, and
the push clears the carry's three fields itself rather than through
`k_carry_clear`.

On this container, over the `entries` change: runbench 1,674,340,808 ->
1,673,252,318 (-0.0650%), encodebench and livebench about -4.92 million
each, deepbench -1,059,144, widebench -79,955, and no benchmark rises. Every
counter vein agrees. The hot unit's own test now names `k_beat_push`.

Taking `k_beat_pop` inline the same way was measured and declined. With it,
runbench read +101,881 and deepbench +747,447 against the push alone, and
encodebench did not move.

## 2026-09-25 — an empty literal is one call and one bump

`[]` and `{}` went through `k_list_lit` and `k_map_lit` with a count of zero
and an empty stack array. Each asked for its buffer's size class at run time,
probed the free list, took the buffer and the header in two bumps, and ran a
copy loop over nothing. The decoder opens 272,349 arrays and 273,339 objects
a run on runbench, and the two functions were 27,465,349 instructions there,
about 50 a literal. The emitter now calls `k_list_empty` or `k_map_empty` for
an empty literal and writes no stack array for it. Each probes the free list
for a constant class and otherwise takes header and buffer in one bump,
header first.

On this container, over the `k_beat_push` change: runbench 1,673,252,318 ->
1,656,770,406 (-0.9850%), jsonbench 1,126,719,496 -> 1,103,823,346
(-2.0321%), deepbench -1,612,003, encodebench -208,936, oneshot -152,245,
livebench -152,641, escapebench -105,002. digestbench reads 5,891,153 ->
5,939,233 (+0.8161%). The rise is in `d_sha256/digested_11`: with no stack
array at its empty literal, LLVM stopped writing the `'2` clone it had
inlined before, and the loop that remained runs 88,400 instructions more. The
runtime helper is not in that difference. runbench's module falls 24,479 ->
24,458 lines and the decoder's 4,922 -> 4,919.

`allocs` falls by one a literal in every vein that makes one. The new `.mem`
fixture `an_empty_literal_takes_one_bump` makes a thousand of each and reads
2001, where the tree before the change reads 4001. Watched red with the two
emitter arms switched off. Its neighbour `a_map_walk_builds_no_scratch_pair`
quoted allocation figures from before the `entries` change and now quotes the
current ones.

## 2026-09-25 — an interpolated int is written where it stays

`"{i}"` with an int went through `k_render`: the switch every value takes, the
digits written into a stack buffer, and `k_str_n` copying them into a fresh
string. That was about 109 instructions an int, and the run program's pend
shape makes 200,000 of them in `churn`. `k_b_render_value` now answers an
int itself. A single digit comes from the ascii cache through `k_str_n`, as it
did before. Any other int's length is known before its first digit, so the
string is allocated at that length and `k_itoa` writes into it.

On this container, over the empty-literal change: runbench 1,656,770,406 ->
1,650,960,035 (-0.3507%) and pendbench 209,746,680 -> 186,507,227
(-11.0797%). basket reads -579, deepbench -14 and scanbench +5, which is
layout. Every counter vein agrees. A first version allocated for single
digits too and read two veins' `allocs` 10 and 1,836 higher, which is how
the cache path was found. The render differential agrees on its 86 values,
the numeric differential on its 2,163 programs, and the int extremes print
the same on both engines.

## 2026-09-25 — a failure check is one compare

Every "is this value a failure" the emitter asked was a call to the
alwaysinline `k_not_failure` and a compare of its answer with zero. The
release pipeline paid its inliner to open each call, and the dev tier, which
does no other optimising, kept the widening and the second compare as well
as the two-word call it had been rewritten to. The emitter now extracts the
tag, or takes it from `known_words`, and compares it with `K_ERR_TAG`, which
is 5. A tag known at compile time answers `true` or `false` outright.
runbench's module falls 4,171 -> 3,564 calls and 24,458 -> 24,330 lines, and
loses a define, since nothing calls `k_not_failure` now and the prune drops
it. The decoder's calls fall 582 -> 485.

On this container, against the int-render head: `codegen_instructions_dev`
143,592,238 -> 142,070,258 (-1.06%), `codegen_instructions_release`
720,659,681 -> 715,347,931 (-0.74%) and `emit_instructions` 30,390,367 ->
29,621,519 (-2.53%). runbench 1,650,960,035 -> 1,648,120,677 (-0.1720%) and
jsonbench -5,695,350. scanbench reads +22. Every counter vein agrees.

`tests/the_err_tag_is_the_runtime_s.rs` holds the number to the position of
`K_ERR` in the runtime's tag enum, and to the compare in the declared
`k_not_failure`, so the emitter cannot drift from either. Watched red with
the constant set to 6.

## 2026-09-25 — a nine-word arm keeps its tail calls on x86-64

A release build narrows `tailcc` to the arms whose arguments fit arm64's
eight argument registers, because past them arm64 miscompiles the convention.
Every call into a wider arm became an ordinary call, and that was done on
every architecture. The JSON decoder's `obj_key_end`, which skips whitespace
before a key's colon, takes nine words. So each key of an object entered it
through a call, the chain back to `obj_key_start` never returned until the
object closed, and the stack held a frame per key. A release build decoding
an object of 300,000 keys died on SIGSEGV. The musttail edges in the emitted
text were all correct; `narrow_tailcc` in src/main.rs took them away after.

The limit is now `TAILCC_WIDEST`: eight on arm64 as before, nine on x86-64,
which lowers a spilling `tailcc` call correctly. Nine was measured, against
the head before this change, on this container:

    narrowed above   runbench           jsonbench          scanbench
    8                1,648,120,677      1,098,127,996      310,445,976
    9                1,643,981,958      1,094,365,546      310,445,975
    10               1,645,003,836      1,094,365,546      316,479,002
    none             1,645,903,453      1,094,365,546      316,941,464

Ten and above let std/regexp's wider arms keep their tail calls too, and a
tail call copies its stack arguments into the caller's area; past three of
them that costs more than the frame it saves. runbench reads -0.2511% and
jsonbench -0.3426% at nine, and nothing else moves.

`tests/a_big_object_decodes_in_a_release_build.rs` builds a program that
decodes a 300,000-key object in release and asserts it prints the count. It
runs on x86-64 only, because arm64 keeps the narrow limit and the frame per
key with it. Watched red with the limit at eight: SIGSEGV. The ratchet row
`nine_words` breaks it the same way.

## 2026-09-25 — the failure twins go, and the specs that read them move

The compare-of-the-tag change left the IR twins `k_not_failure` and
`k_not_failure_w` with no caller, and three specs still read them. The full
suite on the branch's head found two, which CI would have found a round
later: `a_dev_build_asks_its_predicates_in_words` wanted the dev module to
call `k_not_failure_w`, and `perf_ratchet` read the twin's body out of a
module that no longer carried it. Its recursive program also no longer
called any hot predicate, since its failure checks now fold to constants.
The twins, their `predicate` form and their entries in the declares list are
gone. The dev spec now asserts that neither tier calls a failure predicate in
any form. `perf_ratchet` gives its hot-predicate test a program whose `if`
asks `k_truthy`. It replaces the twin test with one that finds the emitted
compares against `K_ERR_TAG` and none against the none tag, watched red with
the compare written against 4. The err-tag spec keeps its enum check. No
emitted vein moves, since the release prune had already dropped the unused
twins from every module.

Letting a group with a single int-literal arm pass its byte raw, as groups
with two already do, was measured and declined. It would have taken
`obj_key_end` to eight words and so kept its tail calls on arm64 as well.
On this container runbench read +0.4448% and jsonbench +0.8978%: a ladder
dispatcher reboxes the raw byte at entry and compares the box, which costs
more than passing the box did.

## 2026-09-25 — CI's rows for kanso#1622

Measured by CI on the branch's head, against main's goldens after kanso#1621.
The run program reads 1,655,310,689 -> 1,623,308,009 (-1.9333%). jsonbench
reads -2.7490%, pendbench -13.4194%, oneshot -1.4097%, digestbench -1.3805%,
livebench -0.4958%, encodebench -0.3915% and widebench -0.3755%. readbench
reads +4 and indexbench is unchanged. `codegen_instructions_dev` falls
144,314,284 -> 142,644,443 (-1.16%), `emit_instructions` 30,190,261 ->
29,340,940 (-2.81%), `compile_instructions` 25,460,259 -> 25,041,901
(-1.64%), `entry_instructions` 86,678,016 -> 83,306,187 (-3.89%),
`library_instructions` 87,221,497 -> 83,849,828 (-3.87%),
`startup_instructions` 601,897 -> 601,491 and
`interp_instructions` 733,050,032 -> 732,994,592.

Three kinds of counter rose. `codegen_instructions_release` reads
713,520,095 -> 715,952,319 (+0.34%). The hot unit that the LTO link
optimises now carries `k_beat_push`, and the runtime gained `k_list_empty`,
`k_map_empty` and the int path in `k_b_render_value`. The machine-code `text`
row rose on thirteen of the fourteen benchmarks, by 1,152 to 5,072 bytes, for
the same reason: runbench text 407,976 -> 413,048, jsonbench text 230,168 ->
231,384, encodebench text 246,088 -> 249,048, oneshot text 244,104 ->
246,008, basket text 227,736 -> 230,264, widebench text 248,664 -> 251,976,
deepbench text 207,400 -> 208,808, escapebench text 202,296 -> 203,688,
pendbench text 216,968 -> 218,120, indexbench text 202,072 -> 203,272,
digestbench text 225,800 -> 228,328, readbench text 202,568 -> 203,768 and
livebench text 244,984 -> 246,872. scanbench text fell by 400. Summed over
the fourteen, `text` reads 3,420,864 -> 3,448,224. `work_readbench` reads
4,631,756 -> 4,631,760, and those four instructions are layout. The runtime saving on run, which is the
objective's heaviest term, is what these pay for.

A lead measured and declined while the rows were taken: passing the JSON
decoder's key to `obj_key_end` as its `parsed` record, which would have
brought the function to eight words and so kept its tail calls on arm64 too,
boxed the record at every key. runbench read 1,876,423,117 against
1,643,981,958.

---

## 2026-09-25 — a tail cycle takes one flat signature under preserve_none

The JSON decoder is a cycle of tail calls, and under `tailcc` every arm
saved the callee-saved registers it used on entry and restored them before
each jump out. The archive's entry on the entry block named `preserve_none`
as the remedy and said it could not be swapped in, because a `musttail` may
cross an arity or a type only under `tailcc`, and this cycle crosses both.
LLVM holds every other convention to matching prototypes. So the cycle is
given one: `preserve_none_tails` in src/main.rs joins every `tailcc` function
to the ones it `musttail`s into, flattens each parameter to `i64` words (a
`%KValue` or `%parsed` is two, an `i64`, `ptr` or `double` one), and pads
every member to the widest with `poison`. The decoder's widest arm,
`obj_key_end`, is nine words and the convention passes twelve in registers
on x86-64. Entry rebuilds the two-word parameters with `insertvalue`, and
each call takes its arguments apart with `extractvalue`.

It runs on release builds on x86-64 when the clang on PATH takes
`preserve_nonecc`, which is the probe the closure convention already asks.
The arm64 limit in `narrow_tailcc` is a miscompile, and nobody has measured
this convention there, so arm64 is left as it was. A set of functions is left
alone when a member's address is taken, when a parameter has a type this does
not flatten, or when the set is wider than twelve words.

With the rewrite in place, `narrow_tailcc` narrows only arms wider than the
register file, twelve words rather than nine, since an arm that fits passes
nothing on the stack. The nine-word limit stood because a tail call copies
stack arguments into its caller's frame, and that cost does not arise here.

This container now measures with clang 19 and lld 19 selected the way the
cost-goldens job selects them, and runbench on the branch's base read
1,623,307,999 against CI's 1,623,308,009. The earlier rows in this log were
taken under clang 18, which is why they drifted from CI's. Each benchmark was
relinked by hand from its own `.ll` and the objects its build used, and every
relink was byte-identical to the built binary before the rewrite was applied:

    benchmark       before           flat, nine words   twelve words
    runbench        1,623,307,985    1,556,827,897      1,556,366,722   -4.1237%
    jsonbench       1,096,078,130    1,018,225,730      1,018,225,716   -7.1028%
    livebench       2,358,727,241    2,282,241,016      2,282,241,002   -3.2427%
    oneshot            16,425,179       15,716,251         15,716,251   -4.3161%
    encodebench     3,030,067,322    2,977,257,216      2,977,257,216   -1.7429%
    deepbench         366,627,999      360,447,991        360,447,977   -1.6856%
    widebench          29,721,589       29,433,525         29,433,511   -0.9692%
    scanbench         296,385,134      296,885,322        292,370,800   -1.3544%
    basket             32,539,840       32,493,041         32,493,055   -0.1438%
    pendbench         181,007,965      181,799,647        181,799,633   +0.4374%
    digestbench         5,761,671        5,787,266          5,787,266   +0.4442%

escapebench, indexbench and readbench do not move. Every binary printed what
its base printed. pendbench and digestbench rise because a caller that keeps
a value live across a call into the cycle now saves it itself, where the
callee's prologue used to, and in those two programs the calls into the
cycle outnumber the hops inside it.

**And the rest of the program's functions take the convention too.** A
function the program calls in the ordinary way saved the callee-saved
registers it used whether or not its caller had anything in them.
`preserve_none_calls` gives every function the module defines and calls only
directly `preserve_nonecc`, so the caller saves what it keeps live across the
call and nothing else is saved. What the runtime calls by name keeps the C
convention: the pass reads every `extern` in src/runtime.c, which today is
`d_thunk_eval`, `k_type_field_count`, `k_type_field_name` and `k_user_main`.
`main`, the module's `k_` helpers and anything whose address is taken are
kept as well. The first hand trial converted `d_thunk_eval` along with the
rest, and runbench segfaulted in it when `k_force_slow` called it with C
registers. Measured by building through the compiler, against the tail
rewrite alone:

    runbench       1,556,366,722 -> 1,547,521,959   -0.5683%
    livebench      2,282,241,002 -> 2,209,970,830   -3.1666%
    oneshot           15,716,251 ->    15,539,394   -1.1254%
    scanbench        292,370,800 ->   291,352,654   -0.3482%
    jsonbench      1,018,225,716 -> 1,018,801,116   +0.0565%
    digestbench        5,787,266 ->     5,787,505   +0.0041%

and the others within fourteen instructions, which is the path-length noise
between two build directories. Against the base, runbench reads
1,623,307,985 -> 1,547,521,959, -4.6687%, before the doors below. Unit tests pin the externs read
from the runtime, the linkage order (`define internal preserve_nonecc`), and
the kept address, watched red without the extern check and without the
address check. Ratchet row `plain_calls`.

**Four runtime doors take it as well.** With the program's functions on
`preserve_none`, a C-convention runtime function the program calls still
saves what it uses. Each hot door was marked on its own in a copy of the
runtime and the run program relinked, against 1,547,521,959:

    k_b_append_rendered   -4,998,510     k_b_at                +9,209,935
    k_b_entries           -4,571,478     k_b_utf8_slice_raw    +4,306,698
    k_b_utf8              -2,157,799     k_b_to_int_slice         +42,273
    k_b_to_float_slice    -1,113,156     k_b_append_slice      segfault

`at` and `utf8_slice_raw` get dearer because their callers keep more live
across the call than the callee used to save. The `append_slice` trial
segfaulted and was set aside with its cause unfound: the rewrite covered the
module's declare and its one call, in the prelude's `k_b_append_slice_fast`,
so the caller that still spoke C is somewhere this trial did not look. The
four that pay are defined with
`K_DOORCC` in src/runtime.c, which is `preserve_none` under the same probe as
the closures, and `PRESERVE_NONE_DOORS` in src/codegen.rs writes the keyword
on their declares and calls in every tier where the probe says yes. The run
program reads 1,547,521,959 -> 1,535,239,270 (-0.7937%), widebench -1.6852%,
jsonbench -0.4874%, encodebench -0.2694%, oneshot -0.2061%, basket -0.0369%
and livebench +0.0180%. The spec `the_doors_the_program_calls_agree` holds
the runtime's list and the emitter's equal, watched red with `k_b_utf8` off
the emitter's.

The emitted-code rows count the `.ll` a release build writes, which is the
IR after this rewrite, and every flattened parameter and argument is one or
two more lines of `insertvalue` or `extractvalue`. Calls, branches and
defines do not move. `emitted_lines` reads 5,379, against 4,891 before the
rewrite and 5,063 on main, `emitted_other_lines` reads 76,037 -> 82,781, and
runbench's module 24,330 -> 27,176. LLVM folds these lines away before it selects
instructions, and the codegen rows do not see them: the codegen gate runs
under `/usr/bin`'s clang 18, which does not take the convention.

The specs job now selects clang 19 as the cost-goldens job does, with
llvm-19 for `opt-19`. It had run the image's clang 18, so no spec ran a
release binary built the way the measured ones are, and the closures'
`preserve_nonecc` had never met the suite either. Under clang 19 the whole
suite passed on this container except `the_ir_kanso_writes_passes_that_verifier`,
whose first choice of verifier was the bare `opt`, LLVM 18's, which refused
the keyword at the parser in 76 of 214 programs. It now asks the tools of
clang's own release first.

The two specs: `a_tail_cycle_crosses_arities_in_a_release_build` runs a cycle
of a five-word and a six-word arm, carrying an int, a float64 and a string,
four million hops deep, and compares it with the interpreter. Watched red
with a two-word parameter rebuilt in reverse order: the binary read a payload
as a tag and stopped with `no overload of \`cycle/ping\` matches these
arguments`. The unit tests in src/main.rs pin the signature, the padding, and
the three cases that leave a set alone, and were watched red with the
padding and the address check each removed. Ratchet rows `flat_tails`
(the rewrite skipped, seen by the work vein) and `flat_order` (the
parameter reversed, seen by the spec).

---

## 2026-09-25 — CI's rows for kanso#1623

Measured by CI on `a80647e2`. The run program reads 1,623,308,009 ->
1,535,239,320 (-5.4253%) against main's golden after kanso#1622's rows,
which is within fifty instructions of what this container measured under
clang 19. jsonbench reads 1,096,078,477 -> 1,013,835,727 (-7.5034%),
livebench 2,358,727,588 -> 2,210,369,697 (-6.2897%), oneshot -5.5847%,
widebench -2.6395%, encodebench -2.0078%, deepbench -1.6857% and scanbench
-1.6979%. pendbench reads 181,008,423 -> 181,800,105 (+0.4374%) and
digestbench 5,762,004 -> 5,787,838 (+0.4484%), the rise the first entry
explains: their calls into a cycle outnumber the hops inside it.

`codegen_instructions_release` reads 715,952,202, 117 below the branch's
base, and `startup_instructions` 601,506 and `emit_instructions` 29,340,955,
fifteen above each. None of the three gates takes the convention, since each
runs under `/usr/bin`'s clang 18, so these are the compiler's own layout. The
machine-code `text` row, summed over the fourteen, reads 3,448,224 ->
3,439,344. encodebench's fell 6,368 bytes, runbench's 1,488 and widebench's
1,296; scanbench's grew 640 and digestbench's 608, and six others moved by
less than 550 either way.

Welfare reads 87.98 against a floor of 87.69, production 74.49, and the rise
is banked.

---

## 2026-09-25 — an append that must grow asks first

With the program's frames gone, `k_b_append_range` was the runtime function
spending the largest share of itself on its frame: 71.3% of 2,000,934
instructions on the run program. Every one of its 142,731 calls arrived with a
bytes value that had no buffer yet, the empty accumulator an encoder starts
from, so every call pushed five registers, tested the capacity, popped them
and jumped to `k_b_append_grow`. The test now sits in an always-inline
wrapper at each caller, and the fitting path is `k_b_append_fit`, out of line
as before. On this container under clang 19, against kanso#1623:

    runbench       1,535,239,270 -> 1,534,028,068   -0.0789%
    widebench         28,937,509 ->    28,777,507   -0.5529%
    jsonbench      1,013,835,366 -> 1,011,966,666   -0.1843%
    oneshot           15,507,370 ->    15,495,158   -0.0787%
    encodebench    2,969,232,524 -> 2,969,366,924   +0.0045%
    livebench      2,210,369,336 -> 2,210,455,278   +0.0039%

and the other eight unchanged. The output is the same either way, so the work
vein is what sees it: ratchet row `grow_first` drops the test.

`k_b_append_slice`, its other caller, still pushes five registers on each of
its 175,797 calls, 1,933,767 instructions of frame. It was one of the doors
tried under `preserve_none` in the entry above and the trial segfaulted, so
it is left as it is.

---

## 2026-09-25 — CI's rows for kanso#1624

Measured by CI on `0afacb5c`, against kanso#1623's goldens. The run program
reads 1,535,239,320 -> 1,534,028,118 (-0.0789%), widebench 28,937,870 ->
28,777,868 (-0.5529%), jsonbench -0.1843% and oneshot -0.0787%; encodebench
+0.0045% and livebench +0.0039%. Every binary's machine code is 368 bytes
smaller, and `text` sums to 3,434,192. `codegen_instructions_release` reads 715,946,899 and
`codegen_instructions_dev` 142,639,014, 5,303 and 5,429 below the branch's
base. Welfare reads 87.99 and the rise is banked.

---

## 2026-09-25 — the interpreter moves a winning argument into its binding

Arm selection tries every candidate in a group, and `match_one` bound a
parameter that is a name to a clone of its argument, so each candidate paid
for the clone whether it won or not. On the interpreted corpus that was
620,601 clones from `match_one`, 26,424,710 instructions of
`Value::clone`, most of them a `BigInt` or a `String` that allocates, and as
many drops after. A name parameter, bare or annotated, now holds `none` while
the arms are tried, and once an arm has won `bind_moved` moves each argument
into its place. The argument vector is cleared before the body runs and
nothing reads it after, which the dispatcher already said.

    interp_instructions (this container)   750,610,887 -> 728,721,835   -2.9161%

The printed output is the same. The row is the only thing that can see this,
so ratchet row `moved_binds` clones at every candidate again and gates on the
interpreted run's instructions.

---

## 2026-09-25 — CI's rows for kanso#1625

Measured by CI on `01d02dc6`. `interp_instructions` reads 732,994,592 ->
709,556,834 (-3.1975%) and `interp_allocs` 983,321 -> 929,201 (-5.5038%).
Every native row is kanso#1624's. Development welfare reads
88.84 and the meta 88.00, and the rise is banked.

---

## 2026-09-25 — a list that outgrows four slots takes eight, and the run program's peak loses a block

One decode of bench/large.json allocates 1,695,392 bytes for a 188,698-byte
document, and a histogram of `k_alloc`'s sizes named the largest share:
1,463 allocations of 272 bytes, 397,936 in all. Those are list buffers of
sixteen slots. A list starts at four, and `k_b_push_grow` took the smallest
power of two that held the new element and doubled it again, so the fifth
push went to sixteen. The document's 2,752 lists average 3.5 elements and
none of the ones that grow get far past five.

The grow now doubles again only past sixteen: the steps are 4, 8, 16, 64, 256,
where they were 4, 16, 64, 256. On this container under clang 19 the run
program's `arena_peak_bytes` reads 4,194,304 -> 3,670,032, one block fewer,
and `alloc_bytes` 397,171,773 -> 386,879,005, for +0.1925% of its
instructions on the tree of kanso#1623. A block of the run's peak is worth
more to the objective than a fifth of a per cent of its instructions, and
CI's rows will say by how much.

Three other shapes were measured and set aside. Holding the doubling back
only to eight gave the same peak for +0.1520% of the run program, but it put
every longer list on 8, 32, 128, 512, which overshoots a thousand elements by
twice as much: the book's counters sample doubled its permanent peak, and
pendbench rose 6.27%. Plain doubling from four read the same peak for
+0.8232%. Starting an empty map at two pairs rather than four read the same
peak for +2.0088%.

A list of nine to sixteen elements now takes one more grow than it did, and
that is what most of the counters below record: a push that finds its buffer
full goes to the slow path once more. The mem fixture
`a_fifth_push_takes_eight_slots` builds 300 lists of five and pins
`sh_buf=67200`, which read 105,600 on the old grow; ratchet row `fifth_push`
restores the old doubling. The counters that rose, and where they landed:

    run_beat_iters                                                  2,708,989 -> 2,708,992
    run_bytes_malloc                                                   16,679 -> 20,551
    run_evac_allocs                                                    62,993 -> 63,041
    run_evac_bytes                                                 10,017,088 -> 10,018,720
    run_push_mut_fast                                               1,098,392 -> 1,097,990
    run_push_mut_slow                                               1,638,121 -> 1,638,523
    push_mut_fast                                                   1,325,400 -> 1,325,250
    push_mut_slow                                                     134,400 -> 134,550
    encode_alloc_bytes                                            657,702,640 -> 657,770,480
    encode_push_mut_fast                                               27,535 -> 25,634
    encode_push_mut_slow                                                3,654 -> 5,555
    encode_sh_buf                                                  73,267,200 -> 73,335,040
    oneshot_push_mut_fast                                               8,836 -> 8,835
    oneshot_push_mut_slow                                                 896 -> 897
    basket_bytes_malloc                                                    30 -> 32
    basket_push_mut_fast                                               12,310 -> 12,241
    basket_push_mut_slow                                              104,190 -> 104,259
    pend_alloc_bytes                                               45,529,344 -> 45,542,480
    pend_push_mut_fast                                                799,997 -> 799,796
    pend_push_mut_slow                                                  1,203 -> 1,404
    pend_sh_buf                                                    15,826,144 -> 15,839,280
    escape_alloc_bytes                                             65,760,112 -> 66,192,112
    escape_bytes_malloc                                                12,000 -> 15,000
    wide_alloc_bytes                                                5,590,848 -> 5,590,992
    wide_push_mut_fast                                                 15,994 -> 15,993
    wide_push_mut_slow                                                      6 -> 7
    wide_sh_buf                                                       349,616 -> 349,760
    digest_alloc_bytes                                                671,841 -> 690,561
    digest_push_mut_fast                                               10,184 -> 10,053
    digest_push_mut_slow                                                  200 -> 331
    digest_sh_buf                                                     554,352 -> 573,072
    live_push_mut_fast                                                  8,836 -> 8,835
    live_push_mut_slow                                                    896 -> 897
    a_cap_around_a_count_is_a_range_alloc_bytes                        87,792 -> 87,936
    a_cap_around_a_count_is_a_range_push_mut_fast                       2,995 -> 2,994
    a_cap_around_a_count_is_a_range_push_mut_slow                           5 -> 6
    a_cap_around_a_count_is_a_range_sh_buf                             87,456 -> 87,600
    a_carried_value_written_into_an_older_node_push_mut_fast           15,596 -> 15,195
    a_carried_value_written_into_an_older_node_push_mut_slow            1,205 -> 1,606
    a_class_asks_by_the_byte_alloc_bytes                              468,255 -> 468,399
    a_class_asks_by_the_byte_push_mut_fast                                599 -> 598
    a_class_asks_by_the_byte_push_mut_slow                                  4 -> 5
    a_class_asks_by_the_byte_sh_buf                                   118,432 -> 118,576
    a_digest_holds_every_block_it_walked_alloc_bytes                   15,953 -> 16,241
    a_digest_holds_every_block_it_walked_push_mut_fast                    120 -> 117
    a_digest_holds_every_block_it_walked_push_mut_slow                     24 -> 27
    a_digest_holds_every_block_it_walked_sh_buf                         9,520 -> 9,808
    a_loop_invariant_capture_is_copied_every_rewind_alloc_bytes        102,064 -> 102,208
    a_loop_invariant_capture_is_copied_every_rewind_push_mut_fast            496 -> 495
    a_loop_invariant_capture_is_copied_every_rewind_push_mut_slow              4 -> 5
    a_loop_invariant_capture_is_copied_every_rewind_sh_buf             21,904 -> 22,048
    a_pushed_call_keeps_the_sweep_alloc_bytes                      13,152,080 -> 13,238,480
    a_pushed_call_keeps_the_sweep_bytes_malloc                          2,400 -> 3,000
    a_repaired_node_below_the_mark_holds_tenure_alloc_bytes         2,450,128 -> 2,450,272
    a_repaired_node_below_the_mark_holds_tenure_push_mut_fast          15,596 -> 15,195
    a_repaired_node_below_the_mark_holds_tenure_push_mut_slow           1,204 -> 1,605
    a_repaired_node_below_the_mark_holds_tenure_sh_buf                522,848 -> 523,088
    an_escaped_list_gives_its_buffer_back_alloc_bytes                  73,680 -> 102,480
    an_escaped_list_gives_its_buffer_back_bytes_malloc                    200 -> 400
    an_escaped_list_gives_its_buffer_back_perm_peak_bytes                 272 -> 416
    an_inner_beat_opens_its_tenure_in_the_block_outside_alloc_bytes     14,699,600 -> 14,707,040
    an_inner_beat_opens_its_tenure_in_the_block_outside_push_mut_fast         77,980 -> 75,975
    an_inner_beat_opens_its_tenure_in_the_block_outside_push_mut_slow          4,020 -> 6,025
    an_inner_beat_opens_its_tenure_in_the_block_outside_sh_buf      4,843,184 -> 4,850,624
    early_exit_alloc_bytes                                             88,064 -> 88,208
    early_exit_bytes_malloc                                                 5 -> 6
    fold_push_shape_alloc_bytes                                       175,104 -> 175,392
    fold_push_shape_bytes_malloc                                            5 -> 6
    fold_push_shape_push_mut_fast                                       3,995 -> 3,994
    fold_push_shape_push_mut_slow                                       4,005 -> 4,006
    fold_push_shape_sh_buf                                             87,536 -> 87,680
    fused_map_shape_alloc_bytes                                       175,104 -> 175,392
    fused_map_shape_bytes_malloc                                            5 -> 6
    fused_map_shape_push_mut_fast                                       3,995 -> 3,994
    fused_map_shape_push_mut_slow                                       4,005 -> 4,006
    fused_map_shape_sh_buf                                             87,536 -> 87,680
    fused_reducer_alloc_bytes                                          22,032 -> 22,176
    fused_reducer_bytes_malloc                                              4 -> 5
    fused_select_shape_alloc_bytes                                    175,136 -> 175,424
    fused_select_shape_bytes_malloc                                         5 -> 6
    fused_select_shape_push_mut_fast                                    2,995 -> 2,994
    fused_select_shape_push_mut_slow                                    4,005 -> 4,006
    fused_select_shape_sh_buf                                          87,536 -> 87,680
    fused_tally_alloc_bytes                                            42,720 -> 42,864
    fused_tally_bytes_malloc                                                4 -> 5
    piped_reducer_alloc_bytes                                          22,032 -> 22,176
    piped_reducer_bytes_malloc                                              4 -> 5
    record_fields_alloc_bytes                                           4,688 -> 4,832
    record_fields_push_mut_fast                                            48 -> 47
    record_fields_push_mut_slow                                             2 -> 3
    record_fields_sh_buf                                                1,392 -> 1,536
    skip_shape_alloc_bytes                                             89,568 -> 89,856
    skip_shape_bytes_malloc                                                 5 -> 6
    skip_shape_push_mut_fast                                                9 -> 8
    skip_shape_push_mut_slow                                            4,001 -> 4,002
    skip_shape_sh_buf                                                     432 -> 576
    sort_shape_bytes_malloc                                                 4 -> 5
    sort_shape_push_mut_fast                                            4,818 -> 4,754
    sort_shape_push_mut_slow                                              670 -> 734
    string_headers_alloc_bytes                                          3,088 -> 3,232
    string_headers_push_mut_fast                                           48 -> 47
    string_headers_push_mut_slow                                            2 -> 3
    string_headers_sh_buf                                               1,392 -> 1,536
    take_shape_alloc_bytes                                            175,264 -> 175,552
    take_shape_bytes_malloc                                                 5 -> 6
    take_shape_push_mut_fast                                            2,995 -> 2,994
    take_shape_push_mut_slow                                            4,005 -> 4,006
    take_shape_sh_buf                                                  87,536 -> 87,680
    tally_shape_alloc_bytes                                            93,536 -> 93,680
    tally_shape_bytes_malloc                                                5 -> 6
    the_same_capture_built_below_the_mark_is_shared_alloc_bytes        101,968 -> 102,112
    the_same_capture_built_below_the_mark_is_shared_push_mut_fast            496 -> 495
    the_same_capture_built_below_the_mark_is_shared_push_mut_slow              4 -> 5
    the_same_capture_built_below_the_mark_is_shared_sh_buf             21,904 -> 22,048
The book's counters sample in chapter 10, and the same sample quoted in
chapter 12, read one allocation more (9) and 22,176 bytes where they read
22,032, the extra grow step on a list of a dozen.

## 2026-09-25 — CI's rows for kanso#1626

The cost goldens job measured the list-growth change at its head. The run
program reads `work_runbench` 1,534,028,118 -> 1,536,983,528, a rise of
2,955,410 (+0.19%), the same share this box measured, and
`arena_peak_bytes` holds at 3,670,032. The other work rows that rose are
priced here: `work_deepbench` 360,448,338 -> 364,679,442 (+1.17%),
`work_escapebench` 76,348,450 -> 77,668,593 (+1.73%), `work_jsonbench`
1,011,967,027 -> 1,012,694,527, `work_encodebench` 2,969,367,257 ->
2,969,691,811, `work_oneshot` 15,495,457 -> 15,497,912, `work_basket`
32,481,362 -> 32,495,961, `work_widebench` 28,777,868 -> 28,778,052,
`work_pendbench` 181,800,105 -> 181,845,174, `work_digestbench` 5,787,838 ->
5,813,302 and `work_livebench` 2,210,455,639 -> 2,210,555,256. A list of
five to sixteen items now grows twice where it grew once, and deep and
escape build many lists of that size; the rises arrived with the change and
nothing has isolated that as their cause. The objective weighs only the run
program's row among these, and the meta score rose by 0.10 with the peak
included.
`codegen_instructions_dev` reads 142,639,084 and
`codegen_instructions_release` 715,946,959, seventy and sixty
instructions above the last sitting.


What sets the run program's peak now, found by shrinking one phase at a time
on this branch: the top-level `doc = json/decode raw` holds a full block for
the whole run (a `doc` of `[1]` reads 2,621,456 and no held bytes), and the
index shape adds 524,304 (with `index_chars = 1` the peak is 3,145,728). The
decode loop, encode, deep, pend, escape, split and digest each move nothing
when shrunk to one. The index shape's subject is a view into the last string
its doubling built, so the slice is not a copy; the 524,304 is the join that
builds a 1,572,864-byte string while the 786,432-byte one it doubles is still
held.

Measured and declined while the rows were taken: a direct-mapped cache of a
thousand permanent four- to seven-byte strings in `k_b_utf8_slice_raw`, so
the decoder's keys are shared rather than allocated. One decode of
bench/large.json fell 1,581,088 -> 1,335,840 bytes, and the run program's
`alloc_bytes` 386,879,005 -> 362,637,757, but `arena_peak_bytes` stayed at
3,670,032: a decoded `doc` still needs more than one block. The same cache
was declined on 2026-09-24 at +0.49% of the run program's instructions.

Two block sizes were measured against this peak and declined. At 512 KiB
the run program's `arena_peak_bytes` stays at 3,670,032 and its work row
reads 1,537,988,243 against 1,537,075,881 on this box. At 256 KiB the peak
falls to 3,407,888 and the work row rises to 1,561,986,738, +1.62%; by the
objective's own curves the peak term gains about 0.0012 of production and
the run term loses about 0.0018, so the smaller block is a net loss. A
lower oversize threshold (256 KiB) moved nothing either: the 786,432-byte
string the index shape doubles lands in a spare 1 MiB block, which the
live count already includes.

## 2026-09-25 — two ratchet rows pointed at k_list_empty and k_map_empty

kanso#1622's ratchet found two rows blind. Both mutations patched the path
an empty literal took before kanso#1622 routed `[]` and `{}` through
`k_list_empty` and `k_map_empty`: one gave `k_mklist` a one-slot buffer for
a count of zero, and the other sent `k_map_lit`'s copy of two pairs or fewer
through memcpy. Empty literals reach neither now, so the work vein stayed
green under both. The list row now gives the buffer `k_list_empty` carves a
capacity of one, and jsonbench read 1,042,970,592 against 1,012,694,527
with it applied. The map row is renamed "an empty map literal built as any
other" and turns off the emitter's `{}` arm, so the literal goes back
through `k_map_lit` with a count of zero; jsonbench read 1,025,447,142.
`k_map_lit`'s short copy for one or two pairs stays in the source with no
row, because no benchmark writes such a literal.

## 2026-09-25 — an accumulator's buffer grows where it is

A list that a beat loop builds from outside the loop keeps its buffer out of
the arena, allocated by `k_buf_perm`. When it outgrew that buffer,
`k_b_push_grow` allocated a larger one with malloc, copied the elements,
freed the old buffer and called `k_permreg_add` again for the same field.
The run program's escape shape grows each of its 3,872 lists five times
this way, and those 19,360 grows cost 12,746,994 instructions, about 658
each. A grow whose buffer is already out of the arena now calls
`k_buf_perm_regrow`, a realloc, and leaves the field's first registration
standing: `l->items` is the same field after the realloc, and
`k_permreg_flush_held` already passes over a field it has freed.

Measured on this box against the branch it sits on, every work row fell or
held. `work_runbench` 1,536,983,069 -> 1,530,322,616 (-0.433%),
`work_escapebench` -6.231%, `work_basket` -1.281%, and the rest by under
0.02%. The golden takes CI's last reading plus this box's difference, since
the two agreed to 459 instructions on the run program. The permanent peak
counts one buffer where the malloc path held two for a moment:
`perm_peak_bytes` 20,512 -> 16,400 on the run program, the escape
benchmark and five mem fixtures, 81,952 -> 65,552 on seven more, 416 -> 272
on one, and 5,308,464 -> 4,259,872 on basket. The book's counters sample
in chapters 10 and 12 reads the same fall. The counters count the realloc
as the malloc and the free it replaces, as `k_bytes_buf_regrow` does, so
`bytes_malloc` and `bytes_freed` are unchanged everywhere. The new helper
and its branch add 192 bytes of machine code to each benchmark, so `text`
reads 3,434,192 -> 3,436,880 over the fourteen.

The fixture `an_accumulator_regrows_where_it_is` pins it at 16,400 and read
20,512 with the regrow turned off; the ratchet row `perm_regrow` makes that
mutation. A map's pairs take the same grow in `k_b_put_mut`, and they were
left alone: with that arm made to abort, the mem corpus and all fourteen
benchmarks ran clean, and no fixture I could write put a map accumulator
under a beat, so the arm would have had no golden. That path also frees a
malloc'd predecessor without subtracting it from `k_perm_live`, so the
permanent peak it reports runs high; nothing reaches it today.

## 2026-09-25 — CI's rows for kanso#1627

CI's work rows agreed with the projection in the golden, row for row, and
its machine code agreed with this box. The two codegen rows moved:
`codegen_instructions_dev` 142,639,084 -> 142,641,709, a rise of 2,625, and
`codegen_instructions_release` 715,946,959 -> 715,866,504, a fall of 80,455.

## 2026-09-25 — an interpreted call finds its callee by the reference's address

Every call the interpreter makes through a function reference went through
`call_named`, which looked the callee up in `callees`, a map keyed by the
name's text. On the interpreted corpus that was 168,588 lookups at about a
hundred instructions each: the name hashed, the bucket probed and the key
compared with memcmp. The tail-call path asked the same map again through
`calls_a_group`, 147,199 times, to learn whether the callee was a dispatch
group.

A `Value::FnRef` is made in one place, `resolve`, and `names` keeps what it
made for the interpreter's life, so one name reaches a call through one
`Rc<str>`. `call_ref` and `group_of` now look the callee up in
`callees_by_ref`, keyed by that pointer. Each entry holds its own count on
the name, so no other name can come to live at an address the table knows.
`call_named` keeps the map by text for the operators and builtins that call
by name, and `calls_a_group` had no caller left and is gone.

On this box the interpreted row reads 728,723,173 -> 710,784,378, -2.46%,
with the corpus's output byte-identical. That box does not compare with
CI's golden, so `interp_instructions` carries CI's last reading less the
same 17,938,795, 691,618,039, until CI measures it. The ratchet row
`callee_by_ref` turns `call_ref` back to the lookup by text, and the row
read 725,003,497 here with it applied.

CI then read `interp_instructions` 709,556,834 -> 690,933,840, a fall of
18,622,994 (-2.62%). The second table costs the interpreted run six
allocations and 5,264 bytes of peak: `interp_allocs` 929,201 -> 929,207 and
`interp_peak_bytes` 837,389 -> 842,653, +0.63%. The objective weighs the
peak at 0.04 of the development side against 0.11 for the instructions.

## 2026-09-25 — a run of byte compares is read as one window

The json decoder matches `true`, `false` and `null` with chains such as
`cs[p + 1] == 114 and cs[p + 2] == 117 and cs[p + 3] == 101`. Each read in
the chain was compiled on its own: `p + k` with an overflow check, a test of
each end of the bytes, a merge of the byte with the none an out-of-range
read answers, and the compare. That is about fourteen instructions a byte,
and the run program matches 612,500 literals.

`emit_byte_run` recognises an `and` chain, at any grouping, in which every
conjunct compares a non-strict read `x[p + k]` or `x[p]` with a byte
literal, over the same two names, where the inference has proven `x` bytes
and `p` an int. It emits one test that `p + kmin` is at least one and
`p + kmax` at most the length, then plain loads and compares. Outside that
window the chain is false, because some read is none. The general path is
still emitted for that case rather than `false`, since a `p` near the top
of the integer range makes one of the sums overflow, and that traps.

On this box against kanso#1628's head: `work_jsonbench` -3.457%,
`work_oneshot` -1.506%, `work_runbench` 1,530,322,616 -> 1,506,572,318
(-1.552%), `work_livebench` -0.011%, and every other row byte-identical.
The goldens carry CI's last reading plus that difference. The chain's
general path is kept beside the fused one, so the code grows: `text` rises
496 bytes on jsonbench, oneshot and livebench and 384 on runbench, and the
emitted-code rows add fifteen branches and 102 lines to each program that
decodes JSON: the decoder's `emitted_branches` 495 -> 510 and
`emitted_lines` 5,379 -> 5,481, runbench's 2,752 -> 2,767 and 27,176 -> 27,278, oneshot's
578 -> 593 and 6,421 -> 6,523, livebench's 598 -> 613 and 6,551 -> 6,653.
Summed over the programs the gates read, `emitted_other_branches`
7,908 -> 7,953, `emitted_other_lines` 82,781 -> 83,087, and `text`
3,436,880 -> 3,438,752.

The micro fixture `a_byte_run_reads_its_window_once` probes a two-read and
a three-read run at every position of a six-byte string, both edges
included, and agrees with the interpreter. With each fused read moved one
byte late it disagreed; the ratchet row `byte_run_late` makes that
mutation, and `byte_run_apart` turns the fusion off for the work vein.

## 2026-09-25 — CI's rows for kanso#1629

CI's work, machine-code and emitted-code rows agreed with the goldens row
for row. Two compile-side rows rose with the change:
`startup_instructions` 601,506 -> 601,513 and
`emit_instructions` 29,340,955 -> 29,343,426, +0.0084%.

## 2026-09-25 — a rewind with nothing to take back asks one question

Every iteration of a beat loop ends in `k_beat_rewind`, and its fast path
asked three things before returning: whether the shelf or a registry held
anything, whether the chain's head was still the mark's block, and whether
the arena pointer still stood at the mark. A loop that allocated nothing
answered all three the same way every time, and the run program takes
2,708,992 beat iterations. The pointer is now asked right after the
shelf-and-registries test, and an unmoved pointer returns at once. Blocks
never overlap, so a pointer equal to the mark's lies in the mark's block,
and the head of the chain is no longer loaded to be told so. A tail split
off an oversize block begins at its host's bump pointer, and its own bump
region starts after its header, past any mark taken in that host, so its
pointer cannot stand at a mark either. The path for
a loop that did allocate is the one it was, after the same two tests.

On this box against kanso#1629's head: `work_runbench` 1,506,572,318 ->
1,499,116,075 (-0.495%), `work_escapebench` -4.931%, `work_basket`
-1.041%, `work_livebench` -0.559%, `work_digestbench` -0.418%,
`work_oneshot` -0.203% and `work_encodebench` -0.161%. Five rows rose:
`work_deepbench` 364,679,442 -> 364,731,746 (+52,304, +0.014%),
`work_scanbench` 291,353,015 -> 291,354,017 (+1,002), `work_pendbench`
181,843,867 -> 181,843,966 (+99), `work_widebench` 28,778,045 -> 28,778,067
(+22) and `work_indexbench` 2,855,794 -> 2,855,795 (+1). The goldens carry
CI's last reading plus this box's difference. Every program's `.text`
shrinks by 80 to 112 bytes; the run program's reads 411,768 -> 411,656.

The ratchet row `unmoved_arena` skips the new exit, and escapebench read
76,509,908 with it applied.

## 2026-09-25 — CI's rows for kanso#1630

CI agreed with every projected work row and with the text, emitted and
compile veins. The two codegen rows moved, since `kanso build` compiles the
runtime the rewind lives in: `codegen_instructions_release` 715,866,504 ->
715,981,278 (+114,774, +0.0160%) and `codegen_instructions_dev` 142,641,709
-> 142,641,575 (-134). The release rise is clang's work on the runtime and
is the price of the change; the run program it buys is 7.46 million
instructions cheaper.

## 2026-09-25 — two byte appends in a row ask for room once

The json encoder writes every escape as two appends: the backslash, then the
letter. Each append of a proven int to a proven byte builder the function
owns is already a call to `k_b_append_mut_int`, which checks that the
builder is owned, that its bytes end at the buffer's front, and that one
more byte fits. The second call asks all three again about the builder the
first one just returned.

After the body is emitted and pruned, `paired_appends` looks for two such
calls where the second appends to the first's result and that result is
read nowhere else in the function, and replaces them with one call to
`k_b_append_mut_int2`. The new helper checks room for two bytes and stores
both in order. When the fast path refuses, it runs the two single appends
as written, so a builder that has to grow still grows the way it did.

On this box against kanso#1630's head: `work_livebench` 2,197,955,843 ->
2,182,311,843 (-15,644,000, -0.712%), `work_oneshot` 15,232,332 ->
15,193,222 (-39,110, -0.257%) and `work_runbench` 1,499,116,075 ->
1,495,596,175 (-3,519,900, -0.235%). Those three programs each carry one
merged pair in their emitted IR and the other eleven carry none, so their
rows are byte-identical. The goldens carry CI's last reading plus this
box's difference.

The helper's five comment lines stay in every program after the pruner
drops an unused definition, as the comments of its neighbours do, so
`emitted_lines` rises by five everywhere: jsonbench 5,481 -> 5,486, and in
bench/compile_golden.txt recursion 263 -> 268, dispatch 281 -> 286, guards
268 -> 273, records 332 -> 337, build_block 226 -> 231 and the module 1,051
-> 1,056. The three programs that use the helper carry its definition too:
runbench 27,278 -> 27,320 lines, 3,564 -> 3,565 calls, 2,767 -> 2,770
branches, 493 -> 494 defines, and the same +42 lines, +1 call, +3
branches and +1 define on livebench and oneshot. Their `.text` grows 224 bytes each (runbench 411,656 -> 411,880).
That is the inlined two-byte fast path, and the work it saves is the three
rows above.

In the trend gate's totals: `lines` 1,370 -> 1,395 and `module_lines` 1,051
-> 1,056 in the compile golden, `emitted_lines` 5,481 -> 5,486,
`emitted_other_lines` 83,087 -> 83,263, `emitted_other_calls` 9,731 ->
9,734, `emitted_other_branches` 7,953 -> 7,962, `emitted_other_defines`
1,529 -> 1,532 (the helper's define in each of the three) and `text`
3,437,632 -> 3,438,304.

tests/golden/micro/two_bytes_appended_at_once encodes three strings that
each need an escape. It went red with the helper's two stores swapped. The
ratchet row `byte_pair_swapped` makes that swap and `byte_pair_apart` skips
the merge, which the work vein sees.

CI's first reading of this branch found the peephole itself costly:
`emit_instructions` 29,343,426 -> 31,425,628 (+7.1%) and
`startup_instructions` 601,513 -> 681,955. The first version split the
whole emitted body into lines and copied every one of them back, on every
build, whether or not the program held a pair. It now searches from one
call of the single append to the next, counts uses only inside the function
around a candidate, and returns the body untouched when nothing matches.
On this box, the emit row against kanso#1630's head went from +2,117,616
with the first version to +80,807.

Three other leads were measured today and declined:

- A width cache in `k_b_at`. The run program rose 1,499,116,075 ->
  1,503,003,891 (+3,887,816) and indexbench 7.1%.
- A summary word over the cohort stacks for `pop_any`. It fell 0.16% only
  while the push cleared the word without looking, and `k_cohort_pop` can
  leave a depth tenured. Made safe, the run program fell 0.026% and
  deepbench rose 0.08%.
- Alias tags on the byte-append helpers, separating the bytes header, the
  buffer header and the data. The run program was byte-identical. The first
  append's slow path joins the fast path before the second append starts,
  so the second's loads cannot be forwarded from the first's stores.

## 2026-09-25 — CI's rows for kanso#1631

CI agreed with the three projected work rows and with the text and emitted
veins. The compile-side rows moved with the peephole and with the layout of
the compiler around it. Four rose: `startup_instructions` 601,513 -> 605,252
(+3,739), `emit_instructions` 29,343,426 -> 29,390,013 (+46,587), the cost of
searching each program's body for the pair, `codegen_instructions_dev`
142,641,575 -> 142,645,204 (+3,629) and `codegen_instructions_release`
715,981,278 -> 715,986,547 (+5,269). Three fell: `compile_instructions`
25,041,901 -> 25,004,892, `entry_instructions` 83,306,187 -> 83,186,043 and
`library_instructions` 83,849,828 -> 83,730,469, none of which reaches the
emitter, so the fall is layout. The run program's 3,519,900 fewer
instructions carry the score over the four rises.

## 2026-09-25 — a proven list is measured and indexed in place

The inference already knows, for many parameters, that every value reaching
them is a list: `encode_pairs acc es i` in lib/json is handed `entries m` and
then itself. The emitter used that fact for bytes and nowhere else. `length`
of a proven list still called the length twin, which asks whether the tag is
a list or bytes before loading the length, and `es[i]` still called the index
twin, which asks the index's tag, the container's tag, and then whether it
was a list or bytes a second time before it loads the slot.

`length` of a value proven to be a list or bytes now reads the header's
first word, which is the length for both. A plain index of a proven list
with a proven int checks the bounds and loads the slot, and answers none
outside them, the way the twin's list arm does. What comes out of a list is
whatever the list holds, so that result carries no set and is forced where
it is used, as before. The strict form, `xs[i]!`, answers a box around the
element, which the runtime builds, so it keeps the general path; the first
draft took it too and a_builtin_demands_its_string printed nothing.

On this box against kanso#1631's head: `work_runbench` 1,495,596,175 ->
1,479,090,614 (-16,505,561, -1.104%), `work_livebench` 2,182,311,843 ->
2,120,705,451 (-2.823%), `work_encodebench` 2,964,897,120 -> 2,861,479,405
(-3.488%), `work_digestbench` 5,788,416 -> 5,540,248 (-4.287%),
`work_oneshot` -1.014%, `work_basket` -0.479%, `work_scanbench` -1,419 and
`work_widebench` -1. The other six rows are byte-identical. The goldens carry
CI's last reading plus this box's difference. Every program whose work moved
is also smaller: runbench's `.text` 411,880 -> 406,776 and its module
27,320 -> 26,838 lines and 3,565 -> 3,493 calls. The compile corpus's module
reads 1,056 -> 1,047 lines, 102 -> 100 calls and 82 -> 81 branches.

tests/golden/micro/a_proven_list_is_read_in_place walks a proven list, asks
at and past both ends, and takes one strict index. It went red with each slot
read one late. The ratchet row `list_read_late` makes that change;
`list_index_twin` and `list_length_twin` send the index and the length back
through the twins, and the work vein sees each.

Several leads were measured against the branches below this one and
declined. kanso#1632 marked every lambda's wrapper `alwaysinline` in a
release module, so that a fold over a known lambda carries the lambda's body
in its loop. encodebench fell 6.106% and no other work row moved, but CI read
`codegen_instructions_release` 715,986,547 -> 774,984,002 (+8.24%), and
welfare does not score encodebench, so the change came out behind by the
objective and was closed. `inlinehint` in place of `alwaysinline` left every
work row byte-identical. Non-trivial loop unswitching, meant to hoist
`fold_flat`'s per-element list-or-bytes test, read byte-identical on all
fourteen rows whether the flag went to the LTO link or to the pre-link `-O1`
compile. A helper merging a byte append followed by a proven one raised
encodebench 0.134% at the five escape sites in its frozen `esc_byte`. Writing a
json key's quotes beside the key, so that `,"` and `":` would be adjacent
pairs for that helper, raised livebench 5.507%, runbench 1.874% and oneshot
1.978%, and the helper fired in none of the three.

## 2026-09-25 — the nine-word row watches the limit the release path reads

The ratchet run on kanso#1626 reported one row blind: "a nine-word arm
narrowed on x86". Its mutation set `TAILCC_WIDEST`'s x86-64 value to eight,
and a_big_object_decodes_in_a_release_build stayed green. On a clang with
`preserve_none`, which the probe finds on CI and on this box, the release path
narrows at `PRESERVE_NONE_REGISTERS`, twelve, and reads `TAILCC_WIDEST` only
where the probe finds no `preserve_none`. So the mutation changed a value the
spec's build never reads. It now narrows the `preserve_none` call site at
eight, and the spec goes red the way its header says: the 300,000-key object
kills the release binary with SIGSEGV. The x86-64 `TAILCC_WIDEST` value is
left without a row; no build on a host with `preserve_none` reads it.

This pull request carries the stack beneath it to main in one run, kanso#1622
through kanso#1631, because the same blind row sat in every one of them and a
fix pushed to each would have cost a ratchet run apiece.

Across the whole carry, measured against main, two rows end higher and each
was priced by the pull request that moved it. `work_readbench` lands at
4,631,757, one instruction over main's 4,631,756. `emitted_other_lines` lands
at 82,139 against 76,037, 6,102 more lines across the thirteen programs,
the net of the moves each carried entry prices. The objective weighs no
emitted-lines term.

## 2026-09-25 — CI's rows for kanso#1633

CI agreed with every projected work row and with the text and emitted veins.
Shorter bodies are less for clang to compile: `codegen_instructions_release`
715,986,547 -> 698,557,830 (-17,428,717, -2.43%), `codegen_instructions_dev`
142,645,204 -> 142,050,065 and `emit_instructions` 29,390,013 -> 29,261,817.
`startup_instructions` rose 605,252 -> 605,388 (+136).

Measured on this branch and declined: building every zero-parameter
definition, which runs once, for size. The release codegen child tree on
this box read 697,137,164 -> 692,496,161 with `minsize` (-0.67%), 696,902,517
with `optsize` and 697,644,423 with `cold`. At the release term's current
ratio the best of the three is worth about 0.00005 of production welfare.

## 2026-09-25 — the interpreter's per-call questions stop hashing, and a plain value skips a call

Three questions the interpreter asks on every call went through a hash map.
Profiled on the interpreted corpus against kanso#1633's head, the three
together cost about 39 million of its 710,784,361 instructions on this box.

Whether a push writes in place was asked of a set keyed by (file path, line,
column), so each of 44,006 container calls hashed the declaration's path, and
the question averaged 141 instructions. Each frame now gathers its own file's
sites once, as (line, column), and keeps them on the frame; a frame is built
once per declaration, so the gathering happens once per declaration.

A call through a function reference found its callee in a map keyed by the
name's address, at about 67 instructions a call over 316,000 calls. Entering
a declaration found its frame in a map keyed by the declaration's address,
at about 71 over 163,000. Each map now has a direct-mapped table of 256 slots
in front of it, holding the last key and answer per slot, and a hit is a
compare and a clone. The callee table keeps a key only while the map pins
the name, and a declaration borrows from the program for the interpreter's
life, so neither table can hold an address that has been handed to another
value. A slot is chosen by multiplying the address by the golden ratio and keeping
the top eight bits, because allocations sit at regular strides and the low
bits of an address repeat; the plain low bits read 689,433,103 against
687,554,523. Sixty-four slots collided too often: 702,682,526 against
697,580,223 at 256, on the tree before the inline test below.

`force_thunk` was a call made on nearly every value the interpreter
produces, to learn in most cases that the value is not a lazy cell: 10.6
million instructions in its own frame. The test is now inline at each
caller, and only a cell reaches the out-of-line loop that forces it.

A hit in either table then still paid for the frame its miss path needed:
fourteen instructions of saves and restores around a lookup of about thirty.
Each miss is now a function of its own, out of line. The frame lookup is
inlined at its callers; the callee lookup stays a call of its own, which
measured 679,842,551 against 680,197,280 with it inlined.

On this box, `interp_instructions` 710,784,361 -> 679,842,551 (-30,941,810,
-4.35%) in six steps: 705,510,238 for the frame's own sites, 701,900,029
with the callee table, 697,580,223 with the frame table, 689,433,103 with
the inline test, 687,554,523 with the golden-ratio slots, and 679,842,551
with the misses out of line.
`interp_peak_bytes` rises 842,648 -> 860,472 (+17,824) and `interp_allocs`
929,207 -> 929,249 (+42), for the two tables and the per-frame site sets.

The ratchet rows `in_place_site`, `recent_callee`, `recent_frame`,
`inline_force` and `frame_hit_inline` each undo one step without changing an
answer, and `interp_instructions` sees each. Measured on the tree each was
written against, the five read 712,672,223, 717,053,460, 708,325,182,
697,580,223 and 680,327,242.

## 2026-09-25 — CI's rows for kanso#1634

CI read `interp_instructions` 690,933,840 -> 661,830,756 (-29,103,084,
-4.21%), a little more than this box's -4.35% of its own baseline.
`interp_peak_bytes` rose 842,653 -> 860,477 (+17,824, +2.12%) and
`interp_allocs` 929,207 -> 929,249 (+42): the recent-callee and recent-frame
tables are two vectors of 256 slots allocated on the first call, and each
frame's in-place sites are a set of its own, built on the first question.
Welfare weighs the peak at 0.04 of the development side against 0.11 for the
instructions, so the trade is the one the objective asks for.

The front-end rows moved with the compiler's layout, since eval.rs is linked
into every `kanso check`: `compile_instructions` 25,004,892 -> 25,041,977
(+37,085, +0.15%), `entry_instructions` 83,186,043 -> 83,306,754 (+120,711,
+0.15%), `library_instructions` 83,730,469 -> 83,850,394 (+119,925, +0.14%),
`startup_instructions` 605,388 -> 605,441 (+53) and `emit_instructions`
29,261,817 -> 29,274,393 (+12,576, +0.04%). None of these paths runs the
interpreter's call machinery, and `compile_allocs` stayed at 14,276.

## 2026-09-25 — a regexp scan asks first for the literal every match holds

The run program's split phase and scanbench search a subject built from
"abcdefghijklm" for `[a-z]+zzq`. There is no match, and finding that out was
quadratic: at every start the letter run was taken to the end of the subject
and given back one letter at a time, each step asking for the `zzq` after it.

A regexp program now carries the literal every match must contain: the
longest run of plain characters side by side in the pattern's top-level
sequence, "zzq" here, or "" when there is none. A class, repetition,
alternation or anchor ends a run, and a group is looked through. The first
scan of a subject, the one at position 1, asks whether the subject holds that
literal and answers no match without scanning when it does not. Later scans
start after a match, and a match holds the literal, so they do not ask. The
search is `text/find2` for the literal's first byte and a comparison of the
rest at each place it stands. A case-insensitive pattern is rewritten into
classes before the literal is taken, so `(?i)zzq` asks for nothing and still
finds "ZZQ".

runbench 1,479,091,073 -> 1,421,154,308 (-57,936,765, -3.92%), with the same
printed tally. The arena peak stays at 3,670,032.

Two mem fixtures, `a_scan_that_finds_nothing_keeps_nothing` and
`a_scan_keeps_its_place_in_the_text`, used `[a-z]+zzq` to watch the scan
itself: that a scan which keeps nothing holds a flat peak, and that it keeps
its place in the text. With the literal asked for first neither walked a
position, the first's `beat_iters` read 0 and the second's `seek_resumes`
fell to 0, which would have left the `seek_kept` ratchet row blind. They now
search for `[a-z]+[x-z]`, which ends in a class the alphabet never supplies,
so each start is still walked and backed off; their arena peaks did not move
and `seek_resumes` holds at 408. scanbench cannot follow them:
`the_run_program_carries_the_shapes_unchanged` holds it to the run program's
split phase character for character, so it keeps `[a-z]+zzq`, answers at
once, and `work_scanbench` falls 291,352,598 -> 285,012 (-99.90%). The
flat-peak property it was written to show is now pinned by the mem fixture.

The run program's split phase now measures a subject scanned once for a
literal rather than a backtracking walk. The program is ruled content, so its
pattern is left as it was. Whether the phase should be given a pattern that
still backtracks is a question about the objective, which is Clay's to
answer, and it does not hold this change up.

tests/golden/micro/a_match_holds_the_literal_its_pattern_spells finds
matches whose literal sits after a class, inside a group, at the end of the
subject and twice in it, one that is absent, and one under `(?i)`. With the
literal run carried across a class, `ab.cd` asked for "abcd" and found
nothing in "xx abXcd". That is the ratchet row `literal_run`; `literal_asked`
removes the question and the work vein sees the scan come back. The
lookbehind's runtime goldens quote the module's line numbers, which moved
from 411 to 448.

Found on the way and left for its own change: `regexp/split` treats a scan
that found nothing like an empty match and scans again from the next
position, so splitting on a separator that does not occur in the rest of the
subject is quadratic in that rest.

Built, measured and declined on the way, as kanso#1635. Interning the
decoder's short tokens in a static slab took one decoded large.json from
1.58 MB to 1.31 MB, and with half-megabyte arena blocks the base the run
program stands on took three blocks where it had taken two of a megabyte, so
the arena peak fell to 3,145,744. That fall was a layout: with this change
the split phase stops allocating, the index phase meets the arena elsewhere,
and the peak is 3,670,032 at either block size. At a megabyte, interning
buys the objective nothing and costs 14,496 permanent bytes. Half-megabyte
blocks also made the carry's copying follow the block boundaries: kq's scale
gate read `evac_bytes` 6,768 -> 183,600 on ten times its input where a
megabyte reads 23,088, and 24,016, 11,104 and 61,136 at two, four and twenty
times, which is a layout rather than a growth law. Two things the work found
stand on their own and are recorded for whoever next shrinks the block: a
malloc'd 256 KiB tenure block lands in the heap once glibc's mmap threshold
has risen past it, and at half-megabyte blocks that stopped the encoder's
byte builder growing in place, 990 reallocs and 12.6 million instructions of
copying; and the spare list hands back the first block at least as large as a
request, so a regular request can take a freed oversize block and the live
chain counts it whole.

What else moved, against main. The literal question is code the regexp
module carries, and the two programs that import it emit it:
`emitted_other_defines` 1,548, `emitted_other_calls` 9,697,
`emitted_other_branches` 7,995, `emitted_other_lines` 83,384, and the text
vein's summed `text` row 3,424,848. The program record's fourth field and the
literal it holds move `run_sh_rec` to 48,174,640, `run_sh_str` to
34,249,776 and `run_bytes_malloc` to 20,556, and scanbench's `scan_sh_rec`
to 1,632, `scan_sh_str` to 672 and `scan_bytes_malloc` to 35.
a_class_asks_by_the_byte moves by the same record and literal:
`a_class_asks_by_the_byte_allocs` 9,268,
`a_class_asks_by_the_byte_alloc_bytes` 468,513,
`a_class_asks_by_the_byte_sh_rec` 87,440, `a_class_asks_by_the_byte_sh_str`
352 and `a_class_asks_by_the_byte_bytes_malloc` 15.

The two scan fixtures re-base with their pattern, since a class is tried at
every backed-off position where a literal was.
a_scan_that_finds_nothing_keeps_nothing:
`a_scan_that_finds_nothing_keeps_nothing_allocs` 74,066,
`a_scan_that_finds_nothing_keeps_nothing_alloc_bytes` 2,778,784,
`a_scan_that_finds_nothing_keeps_nothing_find2_calls` 24,495,
`a_scan_that_finds_nothing_keeps_nothing_sh_bytes` 1,175,784,
`a_scan_that_finds_nothing_keeps_nothing_sh_buf` 13,152,
`a_scan_that_finds_nothing_keeps_nothing_sh_str` 768,
`a_scan_that_finds_nothing_keeps_nothing_bytes_malloc` 41,
`a_scan_that_finds_nothing_keeps_nothing_push_mut_fast` 5 and
`a_scan_that_finds_nothing_keeps_nothing_str_scan_bytes` 37.
a_scan_keeps_its_place_in_the_text:
`a_scan_keeps_its_place_in_the_text_allocs` 1,802,
`a_scan_keeps_its_place_in_the_text_alloc_bytes` 106,592,
`a_scan_keeps_its_place_in_the_text_find2_calls` 255,
`a_scan_keeps_its_place_in_the_text_sh_bytes` 19,752,
`a_scan_keeps_its_place_in_the_text_sh_buf` 50,784,
`a_scan_keeps_its_place_in_the_text_sh_str` 5,760,
`a_scan_keeps_its_place_in_the_text_bytes_malloc` 41,
`a_scan_keeps_its_place_in_the_text_push_mut_fast` 5 and
`a_scan_keeps_its_place_in_the_text_str_scan_bytes` 37.

CI's reading added two compile rows this box could not compare. The entry
and library routes each compile lib/regexp, which now carries the literal
pass and the scan's question: `entry_instructions` 83,186,043 -> 84,496,376
(+1.58%) and `library_instructions` 83,730,469 -> 85,059,188 (+1.59%). Every
work, text and emitted row CI measured matched the projection from this box,
runbench 1,421,154,308 among them. The objective weighs neither compile row,
so the welfare banked with this change is the runbench fall's.

CI's reading of the tree merged with main after kanso#1637. The two compile
rows that compile lib/regexp take both changes: `entry_instructions`
84,619,753 and `library_instructions` 85,181,699, each within 2,700 of the
two deltas summed. Every other row CI measured matched the goldens.

## 2026-09-25 — a split whose scan finds nothing ends there

`regexp/split` scanned for its separator from where the last piece ended, and
when the scan found nothing it scanned again from the next position, as it
does after an empty match. A scan that finds nothing has already looked at
every start after it, so each of those scans found nothing too, and a
separator that stopped occurring early made the split quadratic in what came
after it. It now takes the rest of the subject as the last piece. An empty
match is stepped past from the position it stood at rather than from the
scan's start, which is the same continuation: the scan found the leftmost
match, so no start between the two holds another.

tests/golden/mem/a_split_stops_where_the_separator_does splits "a, b, "
followed by 300 characters on ", ". It reads `beat_iters=303` and 682
allocations; scanning again after finding nothing read 45,453 and 91,582.
That is the ratchet row `split_stops`. tests/golden/micro/
a_split_ends_where_its_separator_does pins the pieces for a separator that
never matches, one that stops, one at the end, and three patterns that match
the empty string, and read the same before and after the change on both
engines. No benchmark splits, so no work row moves.

CI's reading moved the two compile rows that compile lib/regexp:
`entry_instructions` 84,496,376 -> 84,512,441 (+16,065) and
`library_instructions` 85,059,188 -> 85,101,407 (+42,219). The split's new
arm and its guard are code both routes compile. The objective weighs neither
row, and every work row CI measured matched main.

Merged again after kanso#1638, whose split adds its own small cost to the two
rows that compile lib/regexp. Projected by adding that delta to this branch's
CI reading: `entry_instructions` 84,635,818 and `library_instructions`
85,223,918. CI's readings will replace both.

CI read the merged tree at 755fb36d: `entry_instructions` 84,635,818 ->
84,635,833 (+15) and `library_instructions` 85,223,918 -> 85,224,038 (+120).
The projection added two deltas measured on different trees, and the
remainder is layout. Neither row is weighed.

---

## 2026-09-25 — a small map is a sorted vector

The interpreter held every map as a `BTreeMap`. A B-tree leaf has room for
eleven entries whatever it is given, and the maps a program builds are mostly
JSON records with a few keys. The interpreted corpus decodes 220 maps of four
keys each, and massif put their leaves at 158,400 bytes of the interpreter's
peak.

`Value::Map` now holds `Entries`: a vector kept sorted while a map has eight
entries or fewer, and a `BTreeMap` once a ninth key is put. A map never goes
back, so a large map built one key at a time still inserts in log time. Both
forms walk in key order, which is all equality, rendering and `entries` ask of
them, and the few uses of the map (the literal, `put`, `entries`, `length`,
indexing, equality, rendering and the wasm runtime's literal) go through
`new`, `insert`, `get`, `len` and `iter`.

On this container `interp_peak_bytes` falls 860,475 -> 779,732 (-80,743,
-9.4%) and the interpreter's own instructions fall 342,899, with
`interp_allocs` unchanged. Projected against CI's goldens:
`interp_peak_bytes` 860,477 -> 779,734 and `interp_instructions` 661,830,756
-> 661,487,857.

tests/golden/micro/a_map_crosses_eight_entries holds maps on both sides of
eight: eight keys, a ninth put onto them, a key overwritten on each side, a
nine-key literal, the same nine entries put in another order and compared,
and int keys beside string keys. The ratchet row `map_grow` grows into the
tree without the ninth key, and the interpreter prints nine as eight.

---

## 2026-09-25 — an interpreted int in a machine word

The interpreter's `Value::Int` was a `BigInt`. Kanso's ints are unbounded, so
the interpreter needs one, but a `BigInt` keeps its digits in a heap vector
even when it holds 1, and copying a value copies the vector. The interpreted
corpus makes about a quarter of a million ints, nearly all of them small.

`Value::Int` now holds `Int` (src/int.rs): `Small(i64)` while a number fits a
word and `Big(Rc<BigInt>)` once it does not. Addition, subtraction,
multiplication, division and remainder try the word first with checked
arithmetic and fall back to the `BigInt` on overflow. Division and remainder
truncate, as `BigInt`'s do, and the one word division that overflows, the least
integer over -1, falls back like the others and answers 9223372036854775808. A
result that fits a word always goes back into one, so each number has a single
form; equality, ordering and map keys rely on that.

Measured on this container against main, with the sorted-vector maps of the
entry above in both trees:

| row | main | this tree | change |
| --- | ---: | ---: | ---: |
| interpreter instructions (container) | 679,841,199 | 613,222,005 | -9.80% |
| `interp_allocs` | 929,249 | 925,948 | -3,301 |
| `interp_peak_bytes` | 860,477 | 779,736 | -80,741 |

Small ints alone read 614,059,488 instructions and a peak of 841,117. With the
maps the peak is set elsewhere in the run, and against the map change alone it
rises 2 bytes, 779,734 -> 779,736. That `interp_allocs` falls only 3,301
while a quarter of a million ints stop allocating digits was not looked into
further; the row reads what the gate reads.

Projected against CI's goldens: `interp_instructions` 661,487,857 ->
595,211,562, the container's fall of 66,619,194 subtracted; `interp_allocs`
929,249 -> 925,948; `interp_peak_bytes` 779,734 -> 779,736. CI's reading
replaces the instruction row.

An earlier try put the `BigInt` behind an `Rc` and nothing else. It cut the
instructions 4.5% and raised `interp_allocs` 27%, since every new int then
allocated twice, and it scored about +0.011. It was not kept.

tests/an_interpreted_int_crosses_the_word.rs crosses the word with each
operator and with `list/sum`, then brings numbers back into range and compares
them with `==`, `<` and `>`, as a map key, and against a float. The ratchet
row `int_word` keeps every overflowed result in the `BigInt` form, and the
interpreter then says `max + 1 - 1 == max` is false.

Both changes were then merged over the carried tree of #1644 and measured
again against it. On this container `interp_allocs` goes 929,213 -> 925,912,
`interp_peak_bytes` 799,829 -> 718,808 and the interpreter's instructions
679,404,788 -> 612,658,897 (-66,745,891, -9.82%). The goldens carry those
memory rows and `interp_instructions` 659,398,270 -> 592,652,379, the
container's fall subtracted. The figures above were taken against main before
the carrier; these replace them as the pull request's claim.

CI measured the branch at 6d35abb2. `interp_instructions` 659,398,270 ->
614,239,448 (-45,158,822, -6.85%), less than this container's -9.82%;
`interp_allocs` 925,912 and `interp_peak_bytes` 718,808, as projected. Four
compile-side rows moved with the binary's layout, since `src/int.rs` and the
map code are new text in the compiler: `compile_instructions` 25,195,466 ->
25,204,253 (+8,787), `entry_instructions` 85,204,646 -> 85,215,074 (+10,428),
`library_instructions` 85,763,002 -> 85,773,867 (+10,865), and
`emit_instructions` 29,343,492 -> 29,285,427 (-58,065). The three rises are
priced at under 0.0001 of welfare together; the interpreter's fall is worth
about +0.024.

## 2026-09-25 — a walk by index steps from its cursor, and a join knows its count

The run program's index phase reads a 690,000-character string of one- to
four-byte characters one index at a time. The runtime keeps a cursor, the
last character an index found and its byte, so each index resumed from it,
but `k_b_at` reached the cursor through `k_str_seek`, which first asks
whether the string is all ascii and then whether the cursor's own character
was wanted. An index of the character after the cursor's now steps from it
directly: the cursor's byte plus its character's width. It cost 71
instructions an index and costs 59.

The same phase builds its subject by joining it to itself until it is long
enough, and asks `length` after every doubling. A string memoises its
character count the first time something asks for it, but a join produced a
string with no count, so every doubled subject was scanned from the front. A
join whose pieces and separator each carry a count now writes the sum. The
pieces are asked in a pass of their own that stops at the first piece without
a count. Asking inside the copying loop cost pendbench 4,803,400 instructions,
because pend joins four thousand rendered numbers that nothing has counted.

runbench 1,421,154,308 -> 1,410,414,243 (-10,740,065, -0.76%), with the same
printed tally: 8,280,000 from the step and 2,458,767 from the count, less what
the join spends asking. indexbench fell 317,493 and scanbench 3,160. The rest
is priced here. `work_basket` rose 12 to 31,593,366 and `work_pendbench` 1,200
to 181,845,166: each join now asks its first piece, and its separator when
there is one, whether they carry a count. The runtime is larger by the step,
the count and their comments, and the text vein's summed `text` row rose
4,928 to 3,429,776, 352 bytes a benchmark.

`seek_steps` is new, a presence counter for the step, and joins the trend
gate's list of counters where a fall is the worse direction. The step still
counts as a resume, so `seek_resumes` holds whether the step is taken or not;
only `seek_steps` says which. It reads 689,999 on the run program and 3,999 on
`the_length_of_an_indexed_character_needs_no_scan`, and 0 everywhere else. The
count takes `str_scan_bytes` from 4,092,666 to 945,324 on the run program.

tests/golden/micro/a_walk_by_index_steps_from_its_cursor joins counted,
uncounted and multibyte pieces and walks the result by index past its end. A
seed without the separator's count read `spaced: 9` for eleven characters, a
seed that took an uncounted piece read `mixed: 3` for five, and a step taken
from another string's cursor read the walk one character late. Three ratchet
rows: `seek_step` and `join_seed` on the run counters, `join_sep` on the micro
corpus.

Found on the way and left for its own change. With clang 19 the emitter
chooses the preserve_none convention and `through_doors` rewrites four runtime
calls by running `str::replace` eight times over the whole module. On `kanso
play` of a one-line program that is 402,596 of the 1,067,649 instructions
under `kanso::main`. No gate sees it: the gates run with `/usr/bin` first on
PATH, where the image's clang is 18 and takes the other convention, so the
rewrite never runs under measurement.

CI read every work, text and emitted row as projected. The two codegen rows
moved with the runtime's size: `codegen_instructions_dev` 142,050,065 ->
142,053,001 (+2,936) and `codegen_instructions_release` 698,557,830 ->
698,563,613 (+5,783), the cost of compiling the new counter and the cursor
arm into every program.

Merged again after kanso#1634. The page section this change adds is now 150,
after kanso#1634's 149, and the floor is banked on the merged tree's rows,
which CI has not read yet.

## 2026-09-25 — the doors take their convention in one pass

With clang 19 the emitter chooses the preserve_none convention for closures,
and four runtime functions, the doors, take it too. `through_doors` wrote the
keyword into every declare of and call to them with eight `str::replace`
calls, two per door, each of which built the whole module again. On `kanso
play` of a one-line program that was 402,596 of the 1,067,649 instructions
under `kanso::main`. It is now one pass that stops only where `%KValue @k_b_`
appears and writes the keyword when a door's name and its `(` follow and a
`declare ` or `call ` comes before.

The module is byte-identical: `kanso build bench/runbench --release` writes
the same `runbench.ll` from main and from this branch. The one-line `kanso
play` falls from 2,132,907 instructions to 1,787,508 for the whole process on
this container, -345,399.

No gate sees either number. The gates run with `/usr/bin` first on PATH, where
the image's clang is 18, the probe for preserve_none fails, and the function
returns at its first line. `startup_instructions` and `emit_instructions`
therefore stay where they were, and the objective cannot see a change that
takes a sixth off the start-up of every `kanso play` and `kanso test` under
the clang the benchmarks are built with.

The specs job builds with clang 19, so the doors' convention is checked by
every native program it runs. Matching a door by the prefix of its name hands
`k_b_utf8`'s convention to the `k_b_utf8_...` functions: the run program then
prints `runbench 31740465` where it should print `runbench 46013475` and exits
0, and the golden suite goes red on the micro corpus, its release build and
the runtime corpus. Dropping the `call ` case fails the run program with
`utf8 takes a list of byte values`. The ratchet row `door_prefix` holds the
first; the ratchet's box selects clang 19, so the mutation reaches the code.

CI read two rows one instruction higher than main: `emit_instructions`
29,261,817 -> 29,261,818 and `startup_instructions` 605,388 -> 605,389. Under
clang 18 the new pass never runs, so this is the layout of a function that
returns at its first line.

Merged again after kanso#1634, which moved both rows further. CI read the
merged tree one instruction above main on each again: `emit_instructions`
29,274,393 -> 29,274,394 and `startup_instructions` 605,441 -> 605,442.

## 2026-09-25 — a short literal is appended as one word

The json encoder writes every `true`, `false` and `null` with `text/append acc
"true"` into a builder it owns. The emitter already took those appends past the
runtime: the string arm of `k_b_append_mut_byte` loads the literal's cell,
reads its header and copies its length in overlapping loads. A literal of one
to eight bytes appended in place now goes through `k_b_append_mut_word`, which
takes the literal as a word the compiler wrote into the call and its length as
a constant. When the builder has eight bytes of room at its frontier the word
is stored whole and the length moves by the literal's own length; the bytes
past the literal land in room the builder owns and past its length, where
nothing reads them. Otherwise a cold runtime function builds the literal and
appends it in place.

It has to be in place. The first slow arm went through `k_b_append`, the one
the string arm falls back to, and encodebench's counting build crashed. That
arm refuses only a builder with no room for the literal, and `k_b_append` then
grows it. This one also refuses a builder with room for the literal but not
the word, where `k_b_append` claims the room and hands back a new header in
the arena. The encoder carries its builder through beat loops by identity, so
the loop kept the old header, its rewind reclaimed the new one, and the next
append read a header the arena had given to something else.

The first cut also inlined the slow arm, and the larger encoder functions
cost more than the word saved: encodebench fell 1.46% while runbench rose
0.89% and livebench 1.74%. Out of line, on this container: encodebench
2,861,479,405 -> 2,736,636,903 (-124,842,502, -4.36%), livebench -41,831,652
(-1.97%), runbench 1,421,153,849 -> 1,414,525,181 (-6,628,668, -0.47%) with
the same tally, and oneshot -105,172. `work_widebench` rose 303,982 to
29,082,048 (+1.06%); widebench appends few literals and moved with where its
code landed. No counter moved in any vein: the counting build takes the slow
arm every time, and appending in place counts what the string arm's fallback
counted. The emitted veins price the helper and its call sites:
`emitted_lines` 5,496, `emitted_other_lines` 83,655,
`emitted_other_defines` 1,553 and `emitted_other_branches` 8,015, with
`emitted_other_calls` down 34 to 9,663.

tests/golden/micro/a_short_literal_is_appended_whole builds text from
literals of one, three, eight and nine bytes, multibyte ones among them, so
the room runs out part-way through a word at different appends. A length moved
by eight instead of the literal's length printed the bytes past it.
tests/golden/mem/a_literal_appended_across_a_rewind encodes a small document
forty times; with the slow arm through `k_b_append` its counting build
segfaults, and `a_cycle_that_allocates_nothing_needs_no_bracket` drifts from
30 allocations to 20,030. Ratchet rows `word_length`, `word_slow` and
`word_used`.

The helper is written into every module the way the other in-place appends
are, and release builds prune it where nothing calls it. So each program in
`bench/compile_golden.txt` gained its ten lines, recursion 268 -> 278 among
them, for `lines` 1,395 -> 1,445 over the five, and
`bench/compile_golden_modules.txt` read `module_lines` 1,047 -> 1,057. Calls,
branches, rounds and visits did not move.

Built, measured and declined on the way:

- A key's closing quote and its colon as one literal, tried a second time on
  top of the word. `entry_onto` escaped the key itself and appended `"\":"`.
  runbench rose 22,294,121 (+1.58%) and livebench 89,482,230 (+4.30%) against
  the word alone: the key no longer went through `encode_onto`'s dispatch,
  and the arm that replaced it was larger where it was inlined.
- One word for a beat loop's rewind test. A global held the innermost mark's
  pointer while the buffer shelf and that mark's registries were empty,
  refreshed at every write to the top, the registry bits and the shelf flag,
  so an iteration asked one comparison. escapebench fell 3.10%, but runbench
  rose 3,597,581 (+0.25%), encodebench 42,471,101 (+1.48%) and livebench
  29,109,596 (+1.37%): refreshing on every pop and every shelf write cost more
  than the iteration saved.
- `FnEmit::write` without `writeln!`. The emit row fell 677,411 (-2.29%) on
  this container and start-up 2,274, worth a few hundredths of a point; left
  for a change that takes the formatting out of the emitter as a whole.

Merged after kanso#1634. This change's page section is now 150. The floor is
banked on the projected rows so the sentinel can read the branch; CI's rows
replace them.

CI's rows on the merged tree: every work row as projected but
`work_livebench`, which reads 2,078,874,160, twenty above. The development
rows price the helper, which every dev module carries and compiles:
`emit_instructions` 29,274,393 -> 29,288,701 (+14,308),
`startup_instructions` 605,441 -> 606,991 (+1,550) and
`codegen_instructions_dev` 142,050,065 -> 142,160,577 (+110,512). The release
tier prunes the helper where nothing calls it, and `codegen_instructions_release`
falls 698,557,830 -> 698,399,695 (-158,135) with the smaller encoder.

---

## 2026-09-25 — a second play of the same file does not compile

`kanso play` keeps the native binary it builds, keyed by a hash of the
program's IR, so an unchanged file runs again with no clang. Finding the
binary meant compiling the file and emitting its IR on every run. On the
start-up corpus, `print "x"`, this container counted 615,803 instructions under
`kanso::main`: 172,583 in `compile_play_file` lexing, parsing and checking the
file with the modules it loads, and 404,031 in `emit_ir_dev` writing IR that
was hashed and dropped.

A play file imports the standard library and nothing else, and the compiler
embeds every std module except `std/expect`. The loader now notes when a
program reads a module from disk or from handed-in sources. When a play file
read nothing but embedded modules, its IR is decided by the file's name and
text, the compiler, the runtime digest, the closure convention the installed
clang takes, the counting flag and the `KANSO_` environment. `played_key`
hashes those with `key_of`, naming the compiler by its path, length and
modification time, which a rebuild always changes. The first play compiles,
emits and builds through the IR's key as before, and hard-links the binary as
`kanso_play_<key>`. A later play of the same text reads the file, forms the
key and runs that binary without lexing, parsing, checking or emitting: the
name exists only if a play compiled the same inputs cleanly. A file that reads
`std/expect` goes through the IR's key every time, and so does any play with a
`KANSO_` variable set, since several of them ask the compiler to report on its
own work. A warm play whose program dies by a signal compiles the file then,
to word the message the way the compiled program would.

On this container the start-up row falls from 615,803 to 51,616, -564,187
(-91.6%), and `emit_instructions` reads 29,578,328 on both trees, since that
row builds the codegen corpus with `kanso build`. `startup_instructions` is
605,441 -> 52,375 on CI (-553,066, -91.3%). The projection was 41,254: what
is left of a warm play is reading the file, forming the key and starting the
binary, which costs about the same on both hosts, so it does not scale with
the part that went. The start-up row no longer
reaches the front end or the emitter on its measured run; the compile rows and
`emit_instructions` are the ones that watch them.

tests/a_play_file_is_keyed_by_its_text holds three cases. A file rewritten
from `print "first"` to `print "second"` between plays prints `second`; with
the text left out of the key it prints `first`. A play file importing a
`std/expect` read through `KANSO_STD` prints `a?` after the module changes
from `!` to `?`; with the loader's note removed it prints `a!`. A file that
runs out of stack says so on its second play as on its first; with the warm
path's explanation dropped the second says nothing. The ratchet rows are
`play_text`, `play_disk` and `play_signal`.

CI's other rows moved with the compiler's layout, each down:
`compile_instructions` 25,041,977 -> 25,027,159, `entry_instructions`
84,635,833 -> 84,572,805, `library_instructions` 85,224,038 -> 85,160,887,
`emit_instructions` 29,274,393 -> 29,249,085 and `interp_instructions`
661,830,756 -> 659,854,491.

---

## 2026-09-25 — an application takes exactly its arguments

`parse_app` built each application's arguments in a vector grown from
nothing, so the first push gave it room for four. An `Expr` is 56 bytes, most
applications take one or two arguments, and those vectors live as long as the
syntax tree. On the compile corpus they held 122,752 bytes at the front end's
peak, most of it empty slots.

The arguments now go on one stack shared by every application being parsed,
held in a thread-local, and each application takes its own off the top with
`split_off`, which allocates exactly their number. A parse error truncates the
stack back to where that application began. The parser never recovers from an
error part-way through an expression, so this only keeps the stack from
holding arguments nobody will take.

The interpreter parses the same way, so both memory rows move. On this
container `compile_peak_bytes` falls 768,700 -> 708,668 (-60,032, -7.8%) and
`interp_peak_bytes` 860,475 -> 799,827 (-60,648, -7.0%); `compile_allocs`
falls 36 and `interp_allocs` 36, the regrowths of applications with more than
four arguments. Projected against CI's goldens: `compile_peak_bytes` 768,704
-> 708,672, `interp_peak_bytes` 860,477 -> 799,829, `compile_allocs` 14,276 ->
14,240 and `interp_allocs` 929,249 -> 929,213. The thread-local costs 149,522
instructions on `kanso check` of the corpus, 0.58%, which CI's compile rows
will price.

Built and measured on the way:

- `shrink_to_fit` on each argument vector reaches the same peak but adds 1,090
  allocations, one reallocation per application.
- A stack held in the parser instead of a thread-local, taken at `P::new` and
  handed back at drop, adds 356 allocations: parsers nest, and an inner one
  starts with an empty stack.
- `shrink_to_fit` on a module's function vector at the end of `parse` raised
  the peak to 723,640: later passes push generated functions onto it, and an
  exact vector doubles on the first push.
- Sizing `canonicalize_bare_aliases`'s two tables to the declarations each
  holds, and reserving the merged module's functions exactly, left the peak
  where it was; neither is live at the counted peak.

The ratchet row `args_exact` gives each argument vector four slots again and
the compile-memory gate reads 782,140.

---

## 2026-09-25 — five changes carried together

kanso#1639, kanso#1640, kanso#1641, kanso#1642 and kanso#1643 were each
green but for the ratchet, which ran forty minutes to an hour and a half a
round, and each merge would have sent the other four back through it. They
land together from one branch. Each has its own entry above; this one records
what the combination did to the rows.

The veins were regenerated on the combined tree. The literal word's mem
fixture gains the `seek_steps` line the index walk added. Summed over the
fourteen binaries, `text` reads 3,427,984, between the index walk alone at
3,429,776 and the literal word alone at 3,423,056: the walk's cursor arm adds
code and the word's smaller encoder takes some away. The work rows the walk moved take its CI delta on top of the
literal word's CI rows: `work_runbench` 1,403,785,575, `work_indexbench`
2,538,302, `work_pendbench` 181,845,166 and `work_scanbench` 281,852. This
container read each of the four the usual 347 to 439 instructions below that.

The development rows move with the compiler's layout and with the runtime
every build compiles, and the five changes each moved them. They take CI's
readings of this tree.

CI read the carried tree at 913723b0. Every work row matched the projection
but `work_basket`, 31,593,354 -> 31,593,366, twelve above. Both memory rows
landed on their projections: `compile_peak_bytes` 708,672 and
`interp_peak_bytes` 799,829. Against main, the development rows read:

    compile_instructions         25,041,977 ->  25,195,466   +0.61%
    entry_instructions           84,635,833 ->  85,204,646   +0.67%
    library_instructions         85,224,038 ->  85,763,002   +0.63%
    emit_instructions            29,274,393 ->  29,343,492   +0.24%
    interp_instructions         661,830,756 -> 659,398,270   -0.37%
    codegen_instructions_dev    142,050,065 -> 142,163,302   +0.08%
    codegen_instructions_release 698,557,830 -> 698,405,052  -0.02%
    startup_instructions            605,441 ->      52,375  -91.35%

The three front-end rows carry the argument stack's thread-local, which the
argument-vector entry measured at 0.58% of a check on this container, and the
emit row carries the literal word's helper, which every module is emitted with.

## 2026-09-25 — a program that runs out of stack in the page lets go of its cells

The small-int change turned `the_wasm_engine_agrees_with_the_golden_corpus`
red on the specs job and on the other host, with `kanso_compile_wasm: wasm
unreachable instruction executed` on the program after `deep_recursion.kso`.
Skipping the recursion let the rest of the corpus pass, and the map-only
commit before the ints passed whole.

A panic hook that wrote into the output buffer named the panic:
`src/wasm_rt.rs:82`, the `borrow_mut` of REG in `load`. The recursion runs
out of stack in wasmi, and the trap lands wherever the deepest call of the
leaf work happens to be. With the ints it lands inside `push`, in the
growth of REG's vector, while the RefCell is borrowed. A trap unwinds
nothing, so the flag stayed set for the life of the instance and the next
program's `load` panicked on a cell no live code held. Where the trap lands
is a property of the call depths of whatever the leaf work calls, so any
change to the interpreter's arithmetic could move it into a borrow. The ints
did. The bug was already there.

The cells wasm_rt keeps are now `Held`: a RefCell inside an `UnsafeCell`,
reached through `Deref` so every existing borrow site reads as before. At an
entry point, `renew` takes the borrow when it can, and when a dead program
still holds it, it writes a fresh RefCell over the cell. The old value is
leaked, since a trap may have left it half-written. Writing a RefCell
through a shared reference is refused by the `invalid_reference_casting`
lint, which is why the outer `UnsafeCell` is there. `load`, `exec_main`'s
transcript clear and `take_error` all renew.

`a_program_that_runs_out_of_stack_leaves_the_engine_usable` runs the
recursion and then `print "{1 + 2}"` on one instance. It went red with
`renew` asking for the borrow, and green with the fix. The ratchet row
`dead_borrow` makes that mutation.

---

## 2026-09-25 — a dev link in gold, without a build ID

The dev codegen row counts three processes on the codegen corpus. On this
container they are `clang -cc1` at 97,852,592 instructions, `ld.lld` at
43,615,136 and kanso itself, which the row excludes. Nearly 18 million of
lld's count is the dynamic loader resolving libLLVM's symbols before lld reads
an input. `clang -cc1` pays the same kind of toll, about 25 million, and
cannot be moved.

The link can. A dev link has no LTO in it, so any linker takes it. The same
link with the driver's arguments costs:

| linker | instructions |
| --- | ---: |
| GNU ld | 82,492,510 |
| lld | 43,557,070 |
| gold | 29,108,334 |
| gold, `--build-id=none` | 24,762,073 |

Gold computes the build ID as a SHA-1 of the output, 4,346,261 instructions
here. Nothing reads a dev binary's build ID; the binary is rebuilt whenever
its source changes.

So `dev_clang` asks `dev_link_args`, which returns `-fuse-ld=gold` and
`-Wl,--build-id=none` where a probe links a one-line program with gold, and
`lld_args` everywhere else. The answer is cached under clang's and gold's
identities, the way lld's is, and the replayed jobs' cache key now names
`ld.gold` beside `ld.lld`. The release link keeps lld, which does the LTO.

On this container the dev row's children fall 141,467,728 -> 122,697,643
(-18,770,085, -13.3%): the link is 24,845,051 and `clang -cc1` is unchanged.
Projected against CI's golden: `codegen_instructions_dev` 142,163,302 ->
123,393,217. CI's reading replaces it, and says whether its runner has gold;
a runner without it keeps lld and reads the old row.

The ratchet row `dev_gold` hands the dev link back to `lld_args`, and the row
reads 141,467,728 here.

CI measured the branch at e3ccffb1: `codegen_instructions_dev` 142,163,302 ->
125,549,128 (-16,614,174, -11.7%). The runner has gold, and its link is
dearer than this container's by about two million instructions. No other vein
moved.

At 9e45c486 `startup_instructions` read 52,375 -> 52,427 (+52). The warm play
it counts runs none of the new code; the rise arrived with the change and its
mechanism was not isolated. It costs well under 0.0001 of welfare.

## 2026-09-25 — a key costs the same to name whatever its value

The gold link's start-up row read 52,427 on one CI run and 52,375 on the
next, with no source change between the two commits. The carrier had read
52,375 too. Unpacking both runs' function tables and diffing them left four
kanso lines: `pad_integral` +29, `String::write_char` +35, `usize` LowerHex
−8 and `write_str` −6. The difference came to 52, all of it in formatting.

A warm `kanso play` names the preserve_none answer's file
`kanso_pn_answer_{key:016x}`, where the key is an FNV hash of clang's
canonical path, size and modification time. `{:016x}` writes the digits the
value has and then pads one `write_char` per missing digit, so a key whose top
nibble is zero costs more to name. Clang's modification time differs between
runner images, so the key does too, and one image in sixteen lands on a
leading zero. The same is true of the lld and gold answers' keys, the replay
identity's jobs file, the runtime object and the program key. Only the pn key
is reached on a warm start-up, which is why only that row showed it.

Measured here with a copy of clang whose modification time was set by hand,
five mtimes on the padded binary read 47,199, 47,199, 47,248, 47,248 and
47,302: one leading zero nibble costs 49 and two cost 103. The same five on
the fixed binary all read 47,208. An ld.gold copy moved nothing on either
binary, since a warm run links nothing.

`hex16` writes sixteen digits from a fixed loop, and every `{:016x}` in
src/main.rs goes through it. `pid_tag_of`'s `{pid:07}` had the same padding
and now writes its digits the same way. A unit spec checks that `hex16`
spells what `{:016x}` spelled, and
`no_name_pads_a_number_through_format` in
tests/every_temp_path_pads_its_pid.rs fails on any padded format left in
main.rs. It went red with the gold answer's format put back.

CI read the fixed binary's start-up at 52,416, and `startup_instructions`
takes that row: 41 above the 52,375 main carries, which is the fixed writers'
cost against a key whose top nibble was not zero. It was measured at 9 on this
container, where clang's key differs.

---

## 2026-09-25 — the short float search starts where the last one ended

`render_ryu` finds most floats' text without ryu. `ryu_short` tries each place
count p from zero: it scales the float by 10^p, rounds, and keeps the first p
whose quotient divides back to the float. Every float runbench renders takes
that path, 191,070 of them, and 170,820 take four places, so each of those
paid five tries.

Passing is monotone in p while the search's bound holds. A decimal with p
places is also a decimal with p + 1 places, it lies in the same interval of
decimals that read back as the float, and rounding the scaled float finds it
there too. So a try at any p says which side of it the answer lies on. The
search now starts at the place count the previous float took, walks down while
the place below also passes, or walks up from a failure, and stops where the
answer changes. The digits are the same; a float with the same precision as
the one before it costs two tries.

On this container, against the carried tree of #1644:

| row | before | after | change |
| --- | ---: | ---: | ---: |
| runbench | 1,403,785,136 | 1,390,193,031 | -13,592,105 (-0.97%) |
| encodebench | 2,736,636,903 | 2,676,227,371 | -60,409,532 (-2.21%) |
| livebench | 2,078,873,799 | 2,018,464,267 | -60,409,532 (-2.91%) |
| widebench | 29,081,687 | 28,835,810 | -245,877 |
| oneshot | 14,934,042 | 14,783,086 | -150,956 |

The other nine benchmarks moved five instructions or fewer. Every counter vein
and the lazy tier agree with their goldens: the change alters how many tries a
float takes and nothing a counter counts. The instruction golden carries the
falls above subtracted from CI's rows, and CI's reading replaces them.

The guess is one static int. A wrong guess costs tries and never changes the
answer, and the bound check sends a guess the float's magnitude cannot use
back to zero.

tests/every_rendered_float_reads_back_as_itself.rs lifts the search out of
`runtime.c` and sweeps 5,809,326 doubles in an order that moves the guess
about. With the walk down from a passing guess removed it reports 975,871 not
shortest. The ratchet row `ryu_guess` starts the search at zero again, and the
work vein reads runbench back at its old count.

CI measured the branch at a71892d9. The work rows are the projection's to
within twenty instructions: runbench 1,403,785,575 -> 1,390,193,490
(-13,592,085), encodebench 2,736,637,236 -> 2,676,227,704 and livebench
2,078,874,160 -> 2,018,464,628 (-60,409,532 each). Three rows rose.
Every benchmark's `.text` grew 4,256 bytes, runbench's to 413,496; machine code
has no welfare term. `codegen_instructions_dev` went 142,163,302 -> 142,166,083
(+2,781) and `codegen_instructions_release` 698,405,052 -> 698,560,720
(+155,668, +0.02%). The mechanism of the two codegen rises was not isolated.
Together they cost under 0.0001 of welfare, against about +0.05 for runbench.
Six benchmarks that render no floats rose five instructions each with the
layout, none of them weighed: `work_deepbench` 364,731,746 -> 364,731,751,
`work_escapebench` 69,238,422 -> 69,238,427, `work_indexbench` 2,538,302 ->
2,538,307, `work_pendbench` 181,845,166 -> 181,845,171, `work_readbench`
4,631,757 -> 4,631,762 and `work_scanbench` 281,852 -> 281,857. The `text`
vein's sum goes 3,427,984 -> 3,487,568, the 4,256 bytes in each of fourteen
binaries.

Two rows stand above main only because of what the carrier brought, and both
fell from the carrier's readings on this branch: `work_basket` lands on
31,593,364, two below the carrier's 31,593,366 and ten above main's
31,593,354, and `work_widebench` on 28,836,171, 245,877 below the carrier's
29,082,048 and 58,105 above main's 28,778,066. Both rises over main arrived
with the carried changes and are priced in their entries.

## 2026-09-25 — a list that leaves the arena starts at 256 slots

An accumulator that outlives its beat keeps its buffer outside the arena, and
each grow there is a `realloc`. The growth steps are 4, 8, 16, 64, 256 and
1024, and a list first left the arena at eight slots, so runbench's escape
phase took each of its 3,872 lists through a malloc and four reallocs on the
way to a thousand elements: 15,488 reallocs at about 445 instructions each.
A permanent list now starts at 256 slots, which leaves one realloc to 1024.

Measured on this container against the float search's tree, one binary each
way: runbench 1,390,193,031 -> 1,383,035,977 (-7,157,054, -0.515%) and
escapebench -758,747. The run program's `bytes_malloc` falls 20,556 -> 8,940
and `perm_peak_bytes` stays at 16,400, since the one list live at the peak
already held 1,024 slots. Starting at 1,024 read 1,381,944,072, 1,091,891
lower, and at 64 read 1,385,580,220. The 256 floor was chosen over 1,024
because it bounds what a short permanent list costs at 4,112 bytes instead of
16,400.

That cost shows in one fixture. `an_escaped_list_gives_its_buffer_back` grows
two hundred nine-element lists that each leave the arena:
`an_escaped_list_gives_its_buffer_back_perm_peak_bytes` rises 272 -> 4,112
and `an_escaped_list_gives_its_buffer_back_alloc_bytes` 102,480 -> 841,680,
while its `bytes_malloc` halves, 400 -> 200. Fourteen other mem fixtures and
the basket, escape and run veins allocate less and nothing else in them moved.

Six benchmarks that grow few permanent lists paid for the larger first
allocation. None is weighed. `work_jsonbench` rises 251,550 to 977,804,371,
`work_encodebench` 4,641 to 2,676,232,345, `work_pendbench` 3,612 to
181,848,783, `work_oneshot` 1,677 to 14,785,155, `work_livebench` 1,677 to
2,018,466,305, `work_digestbench` 804 to 5,541,383 and `work_widebench` 21 to
28,836,192. The rows are projected from this container's delta onto CI's
readings and are replaced by CI's own.

The mem vein pins the change: `an_accumulator_regrows_where_it_is` reads
`bytes_malloc` 40 where it read 100. The ratchet row `perm_wide` removes the
floor and the mem corpus spec goes red on `a_pushed_call_keeps_the_sweep`,
`bytes_malloc` 3000 against 1200.

Each `bytes_freed` falls with the grows it counted, since a realloc counts as
the free it replaces: `run_bytes_freed` 8,824, `escape_bytes_freed` 6,000,
`basket_bytes_freed` 7, `a_pushed_call_keeps_the_sweep_bytes_freed` 1,200,
`an_accumulator_regrows_where_it_is_bytes_freed` 40,
`an_escaped_list_gives_its_buffer_back_bytes_freed` 200,
`early_exit_bytes_freed` 2, `fold_push_shape_bytes_freed` 2,
`fused_map_shape_bytes_freed` 2, `fused_select_shape_bytes_freed` 2,
`skip_shape_bytes_freed` 2, `take_shape_bytes_freed` 2,
`tally_shape_bytes_freed` 2, `fused_reducer_bytes_freed` 1,
`fused_tally_bytes_freed` 1, `piped_reducer_bytes_freed` 1 and
`sort_shape_bytes_freed` 1. No buffer is freed later than it was.

## 2026-09-25 — an empty map opens with room for five pairs

`{}` seeded a map with eight slots, four pairs, and the decoder opens every
object with it. bench/large.json's 2,761 objects hold one to five keys, 532
to 570 of each size, so the 570 five-key objects each grew their map on the
fifth key: 56,430 grows a run, every call `k_b_put_mut` received, at about
190 instructions with the copy and the donation of the old buffer. The seed is
ten slots now. Ten is not a class the shelf keeps, so `k_map_empty` takes its
header and pairs from one allocation every time and no longer asks the shelf,
and a map that outgrows the seed moves to sixteen slots, which is a class.

Measured on this container on top of the list change, one binary each way:
runbench 1,383,035,977 -> 1,371,385,604 (-11,650,373, -0.842%), jsonbench
-17,589,150, encodebench -119,690, oneshot -112,902, livebench -98,727 and
basket -23,616. No other benchmark moved. `arena_peak_bytes` stays
3,670,032, `held_peak_bytes` 277,538 and `perm_peak_bytes` 16,400. The run
program's `put_mut_grow` falls 56,430 -> 0.

Every empty map is 32 bytes larger, which the byte counters show as rises
with no peak behind them: `run_alloc_bytes` 367,965,182, `run_sh_buf`
99,383,504, `decode_alloc_bytes` 211,278,848, `decode_sh_buf` 115,288,800,
`encode_alloc_bytes` 657,785,584, `encode_sh_buf` 73,350,144,
`live_alloc_bytes` 526,566,224, `live_sh_buf` 71,949,392,
`oneshot_alloc_bytes` 2,908,952, `oneshot_sh_buf` 946,544,
`basket_alloc_bytes` 7,495,057 and `basket_sh_buf` 662,736. The shelf serves
fewer buffers, since an empty map no longer takes one from it:
`run_buf_reuse` 88,758, `decode_buf_reuse` 134,250, `encode_buf_reuse`
4,008, `live_buf_reuse` 895, `oneshot_buf_reuse` 895 and `basket_buf_reuse`
250. `put_mut_fast` rises in five veins as the grows become fast puts:
`run_put_mut_fast` 827,739, `decode_put_mut_fast` 1,254,150 and 8,361 in
encode, live and oneshot. The mem fixtures that open a map rise 32 bytes a
map: `an_empty_literal_takes_one_bump_alloc_bytes` 304,048 and
`an_empty_literal_takes_one_bump_sh_buf` 256,000 for its thousand maps,
`a_map_whose_keys_arrived_in_order_is_its_own_view_alloc_bytes` 65,696 and
`a_map_whose_keys_arrived_in_order_is_its_own_view_sh_buf` 65,584,
`growing_map_alloc_bytes` 181,952 and `growing_map_sh_buf` 65,584,
`map_put_alloc_bytes` 9,120 and `map_put_sh_buf` 4,080,
`readwrite_map_alloc_bytes` 11,056 and `readwrite_map_sh_buf` 448,
`repeated_key_shape_alloc_bytes` 248,160 and `repeated_key_shape_sh_buf`
4,080, `fused_tally_sh_buf` 9,728 and `tally_shape_sh_buf` 2,096.

The mem vein pins the seed: with it put back to eight,
`a_map_whose_keys_arrived_in_order_is_its_own_view` reads its old bytes and
`mem_corpus_pins_native_allocator_counters` goes red. The ratchet row
`map_seed` makes that mutation.

## 2026-09-25 — four changes carried together

kanso#1645, #1646, #1647 and #1648 are carried to main in one pull request
over 058042db, so the ratchet runs once rather than four times: the dev link
in gold with fixed-cost key names, the interpreter's small values with the
wasm cell fix, the float search's starting guess, and the two container
seeds. The ratchet chain runs through each change's rows in that order, and
the log keeps every entry.

The counter veins agree with the combined tree as each branch left them,
since the four touch different parts of the runtime. The dev codegen row is
projected as the gold link's CI reading plus the float search's rise of
2,781, 125,551,909, and CI's reading of the combined tree replaces it. The
same goes for every layout row.

## 2026-09-25 — an empty list opens with room for six

`[]` seeded a list with four slots, and the decoder opens every array with
it. bench/large.json's 2,752 arrays hold one to six elements, 417 to 478 of
each size, and one of 160, so the 893 arrays of five or six each grew on the
fifth push. The seed is six now. Like the map's, six is not a class the
shelf keeps, so `k_list_empty` takes the header and items from one
allocation every time. An array that outgrows six moves to eight.

Measured on this container on top of the four carried changes, one binary
each way: runbench 1,371,385,604 -> 1,355,766,990 (-15,618,614, -1.139%),
jsonbench -23,115,450, escapebench -207,000, deepbench -196,604,
encodebench -192,602, oneshot -165,764, livebench -157,370, basket -1,134
and scanbench -20. `arena_peak_bytes` stays 3,670,032, `held_peak_bytes`
277,538 and `perm_peak_bytes` 16,400, and no peak moved in any vein or mem
fixture. Three rows rose and none is weighed: `work_digestbench` 402 to
5,541,785, `work_pendbench` 172 to 181,848,955 and `work_widebench` 3 to
28,836,195. The rows are projected onto CI's and CI's own replace them.

Every empty list is 32 bytes larger. The byte counters that rise with it, and
the shelf reuses and allocations that move with them, land on these values:
`run_buf_reuse` 127, `run_sh_buf` 102,830,160, `alloc_bytes` 215,949,248,
`sh_buf` 119,959,200, `encode_alloc_bytes` 657,873,536, `encode_buf_reuse`
3,112, `encode_sh_buf` 73,438,096, `oneshot_alloc_bytes` 2,940,088,
`oneshot_sh_buf` 977,680, `basket_alloc_bytes` 7,522,273, `basket_sh_buf`
689,952, `pend_alloc_bytes` 45,563,344, `pend_allocs` 805,979,
`pend_buf_reuse` 502, `pend_sh_buf` 15,860,144, `escape_sh_buf` 336,000,
`scan_alloc_bytes` 11,069, `scan_sh_buf` 1,056, `wide_alloc_bytes` 5,591,024,
`wide_sh_buf` 349,792, `digest_alloc_bytes` 694,833, `digest_allocs` 7,199,
`digest_buf_reuse` 1, `digest_sh_buf` 577,344, `live_alloc_bytes` 526,597,360,
`live_sh_buf` 71,980,528, `a_cap_around_a_count_is_a_range_alloc_bytes`
87,968, `a_cap_around_a_count_is_a_range_sh_buf` 87,632,
`a_carried_value_written_into_an_older_node_alloc_bytes` 2,595,040,
`a_carried_value_written_into_an_older_node_allocs` 33,667,
`a_carried_value_written_into_an_older_node_buf_reuse` 781,
`a_carried_value_written_into_an_older_node_sh_buf` 682,096,
`a_class_asks_by_the_byte_alloc_bytes` 507,105,
`a_class_asks_by_the_byte_sh_buf` 157,168,
`a_demanded_knot_allocates_one_cell_alloc_bytes` 256,
`a_demanded_knot_allocates_one_cell_sh_buf` 144,
`a_digest_holds_every_block_it_walked_alloc_bytes` 16,417,
`a_digest_holds_every_block_it_walked_allocs` 270,
`a_digest_holds_every_block_it_walked_buf_reuse` 1,
`a_digest_holds_every_block_it_walked_sh_buf` 9,984,
`a_loop_invariant_capture_is_copied_every_rewind_alloc_bytes` 102,240,
`a_loop_invariant_capture_is_copied_every_rewind_sh_buf` 22,080,
`a_pushed_call_keeps_the_sweep_sh_buf` 67,200,
`a_repaired_node_below_the_mark_holds_tenure_alloc_bytes` 2,493,952,
`a_repaired_node_below_the_mark_holds_tenure_allocs` 34,047,
`a_repaired_node_below_the_mark_holds_tenure_buf_reuse` 783,
`a_repaired_node_below_the_mark_holds_tenure_carry_dedup` 46,
`a_repaired_node_below_the_mark_holds_tenure_sh_buf` 567,232,
`a_scan_keeps_its_place_in_the_text_alloc_bytes` 108,704,
`a_scan_keeps_its_place_in_the_text_sh_buf` 52,896,
`a_scan_that_finds_nothing_keeps_nothing_alloc_bytes` 2,783,968,
`a_scan_that_finds_nothing_keeps_nothing_sh_buf` 18,336,
`a_split_stops_where_the_separator_does_alloc_bytes` 24,671,
`a_split_stops_where_the_separator_does_sh_buf` 1,168,
`an_accumulator_regrows_where_it_is_sh_buf` 2,240,
`an_empty_literal_takes_one_bump_alloc_bytes` 336,048,
`an_empty_literal_takes_one_bump_sh_buf` 288,000,
`an_escaped_list_gives_its_buffer_back_alloc_bytes` 848,080,
`an_escaped_list_gives_its_buffer_back_sh_buf` 22,400,
`an_inner_beat_opens_its_tenure_in_the_block_outside_alloc_bytes` 14,926,832,
`an_inner_beat_opens_its_tenure_in_the_block_outside_buf_reuse` 3,895,
`an_inner_beat_opens_its_tenure_in_the_block_outside_carry_dedup` 209,
`an_inner_beat_opens_its_tenure_in_the_block_outside_sh_buf` 5,070,416,
`early_exit_sh_buf` 112, `effect_push_shape_alloc_bytes` 3,328,
`effect_push_shape_sh_buf` 768, `fold_push_shape_sh_buf` 87,744,
`fused_map_shape_sh_buf` 87,744, `fused_reducer_sh_buf` 112,
`fused_select_shape_sh_buf` 87,744, `fused_tally_sh_buf` 9,952,
`piped_reducer_sh_buf` 112, `record_fields_alloc_bytes` 4,864,
`record_fields_sh_buf` 1,568, `skip_shape_sh_buf` 640,
`sort_shape_alloc_bytes` 214,672, `sort_shape_buf_reuse` 102,
`sort_shape_sh_buf` 178,032, `string_headers_alloc_bytes` 3,264,
`string_headers_sh_buf` 1,568, `take_shape_sh_buf` 87,744,
`tally_shape_sh_buf` 2,128,
`the_same_capture_built_below_the_mark_is_shared_alloc_bytes` 102,144,
`the_same_capture_built_below_the_mark_is_shared_sh_buf` 22,080,
`unsafe_wrap_alloc_bytes` 208 and `unsafe_wrap_sh_buf` 112. No peak rose with
any of them.

The mem vein pins the seed: with it put back to four, the empty literal's
fixtures read their old bytes and `mem_corpus_pins_native_allocator_counters`
goes red. The ratchet row `list_seed` makes that mutation.

CI read the four carried changes at cf18309e before the list seed joined
them. Its rows were taken as read: runbench 1,371,384,302, 1,761 below the
projection, oneshot 14,671,933, `codegen_instructions_dev` 125,551,388 and
`codegen_instructions_release` 698,557,632. Every binary's `text` read 48
bytes below the float search's reading, and this container's reading of the
same tree agreed with CI to the byte. The list seed's rows are CI's carried
readings plus this container's deltas, and its `text` rows, 80 bytes below
the carried ones in every binary, are this container's reading, which the
carried tree showed to match CI's.
Summed over the fourteen binaries, `text` lands on 3,485,776: the float
search's 4,256 bytes a binary, less 48 from the carried seeds and 80 from the
list seed.

## 2026-09-25 — an empty builder has room for its first append

`bytes ""` built a view of the empty string, which owns no storage, and the
next thing that happens to it is always an append. The decoder unescapes a
string by appending its runs to `text/bytes ""`, so each of runbench's
175,527 escaped strings grew its builder from nothing in `k_b_append_grow`:
15.4M instructions a run, about 88 a grow. The emitter now writes the literal
`bytes ""` as a call to `k_b_bytes_seed`, which returns an empty builder with
64 bytes of room, header and buffer in one bump. Sixty-four is the capacity
that first grow chose, so every later grow is the one it was.

The first cut tested for an empty string at run time, in the inlined view,
and was measured and dropped: it saved 19,167,257 instructions on runbench
but cost encodebench 12,531,202, two instructions on every string it escapes.
A 48-byte seed was measured before that and dropped too. It lowered the run
program's `held_peak_bytes` from 277,538 to 206,458, but it shifted every
later grow's size, and one mem fixture's held peak rose 47%.

Measured on this container against the carried list seed, one binary each
way: runbench 1,355,766,990 -> 1,332,912,068 (-22,854,922, -1.686%),
jsonbench -34,925,054, livebench -280,686, oneshot -233,137 and encodebench
-47,854. No other benchmark moved. `arena_peak_bytes` stays 3,670,032,
`held_peak_bytes` 277,538 and `perm_peak_bytes` 16,400. Two mem fixtures'
held peaks fall, `builder_transient` 80 -> 0 and `stream_write` 23,920 ->
16,000, and none rises. Every module's IR carries one more line, the seed's
declaration, so the compile golden's `lines` rise by one in each shape. The
rows are CI's reading of daac530d plus this container's deltas, and CI's own
replace them: at daac530d CI read runbench 1,355,770,463, deepbench
364,523,751, pendbench 181,849,069, oneshot 14,506,645,
`codegen_instructions_dev` 125,548,064 and `codegen_instructions_release`
698,554,515.

The mem vein pins the seed: with the emitter's arm disabled,
`mem_corpus_pins_native_allocator_counters` goes red. The ratchet row
`bytes_seed` makes that mutation.

A builder whose first buffer is the seed never frees it, since arena storage
goes with its rewind, so each `bytes_freed` that counted that buffer falls by
one: `a_builder_that_outgrows_its_buffer_is_never_held_twice_bytes_freed` 10,
`a_class_asks_by_the_byte_bytes_freed` 4,
`a_cycle_of_four_rewinds_once_a_trip_bytes_freed` 11,
`a_cycle_that_allocates_nothing_needs_no_bracket_bytes_freed` 21,
`a_local_bound_under_a_guard_keeps_the_beat_bytes_freed` 8,
`a_scan_keeps_its_place_in_the_text_bytes_freed` 1,
`a_scan_that_finds_nothing_keeps_nothing_bytes_freed` 1,
`a_split_stops_where_the_separator_does_bytes_freed` 2,
`append_in_place_bytes_freed` 0, `append_of_a_slice_boxes_nothing_bytes_freed`
0, `beat_builder_bytes_freed` 4, `beat_cycle_bytes_freed` 4,
`builder_reclaim_bytes_freed` 200, `builder_transient_bytes_freed` 0 and
`stream_write_bytes_freed` 100. Summed over the fourteen binaries `text`
lands on 3,486,032.

CI read the builder seed at 42d0b138 and its rows were taken: runbench
1,332,911,604, 3,951 below the projection, `codegen_instructions_dev`
125,541,184, `codegen_instructions_release` 698,599,960, 45,445 above the
list seed's, and `emit_instructions` 29,311,721, 26,294 above. Both rises
arrived with the seed's emitter arm and its runtime function; which part of
the change moved each was not isolated. Both are small against runbench.
A spec that holds DECLARES_CONTEXT_CALLS to the calls DECLARES makes caught
the seed named there, where it does not belong: the seed is called from
emitted code only.

Taking the seed off that list also took its `declare` out of the nine
programs that never write `bytes ""`, so their emitted rows went back to
main's: basket 5,436 lines, widebench 6,168, deepbench 1,743, escapebench
628, pendbench 2,965, scanbench 14,419, indexbench 557, digestbench 4,498
and readbench 601. The compile corpus's five programs and its module fall by
the same line and match main's again, so `compile_cost` holds main's golden.
CI read the tree at 947d34ad and its rows were taken:
`codegen_instructions_dev` 125,554,679, 13,495 above the 42d0b138 reading,
`codegen_instructions_release` 698,561,887, 38,073 below it, and
`emit_instructions` 29,311,866, 145 above. Both rises arrived with the
declare leaving the preamble; which line moved each was not isolated.

## 2026-09-26 — gavel: the wall goes

Ruled by Clay, answering the ledger's "Does the wall survive the fused
operators?", which recommended keeping the wall and refusing an inline
`.> (_ -> ...)`. His words, first on seeing `>>` still in the tree: "ack, i
still see the use of >> in the codebase", and then, asked the ledger's
question again: "i have voluminously weighed in on this. we said the wall
goes. we've been over this." The question had been answered in conversation
and never written here, which is why the ledger still carried it and why it
was asked a second time. It is written here now so it is not asked a third.

The ruling is option 1 of the entry. `>>` leaves the language, and
`a .> (_ -> b)` is the one spelling of a step that ignores what came before.
The surface leaves the lexer, the parser, the checker, the three engines and
the book, and a program that still writes `>>` is refused at the token with
the spelling to use instead.

The entry leaves the ledger in this commit. Its sibling, "Was the wall's
simultaneous-failure merge meant to go?", stays: its own text says the wall
question is answered the same way whichever way it goes, and that if the
merge comes back it comes back as a property of bind. That is still open.

## 2026-09-25 — a large builder grows by half

The run program's `held_peak_bytes` was one buffer. Each of the 90 encodes of
large.json grows a malloc'd builder by doubling, 130 bytes up to 277,522, and
the last doubling set the peak. How full that buffer ends up depends on where
the output length falls between two powers of two, and a doubled buffer is on
average roughly a quarter empty when its builder finishes.

A malloc'd builder past 16 kb now grows to one and a half times what it holds.
Below 16 kb it still doubles: growing by half from the first byte cost 4.26
million instructions on runbench for a peak of 193,834. The arena regime keeps
doubling too, since the arena reclaims none of the sizes a builder passes
through. Five settings were measured on runbench against the tree before it:

    past 16 kb, x1.5    held 197,616   instructions -58,990
    past 32 kb, x1.25   held 206,802   instructions +21,979
    past 64 kb, x1.25   held 211,742   instructions -36,145
    past 64 kb, x1.5    held 234,190   instructions -99,886
    everywhere, x1.5    held 193,834   instructions +4,260,748

The run program's held peak falls from 277,538 to 197,616 bytes, which takes
`run_peak_bytes` from 3,963,970 to 3,884,048, 2.0% down. Runbench is projected
at 1,332,852,600, 59,004 below. The other programs that build large text pay
for the extra grows, since a buffer under glibc's mmap threshold is copied
when it moves: encodebench is projected at 2,677,033,890 (+1,161,691),
livebench at 2,018,350,615 (+421,093) and oneshot at 14,274,642 (+1,134).
Welfare weighs none of those three.

A buffer's final size now depends on where its length falls between two steps
of one and a half, so one fixture comes out behind:
`a_cycle_of_four_rewinds_once_a_trip` holds 298,940 bytes at its peak, up from
279,906. The others that cross 16 kb fall: the builder that outgrows its
buffer 135,182 to 128,318, the cycle that allocates nothing 426,000 to 338,274
and the guarded local 33,806 to 25,358. The mem vein pins those peaks, and the
ratchet row `half_grow` restores the doubling and turns
`mem_corpus_pins_native_allocator_counters` red, as it did by hand before this
was committed.

CI read the tree at 78bf6606 and matched the four projections to the
instruction. It also found widebench at 28,756,197, 79,998 below, which the
projection had not measured, `codegen_instructions_dev` at 125,554,618, 61
below, and `codegen_instructions_release` at 698,561,899, 12 above. Those rows
are CI's.

Every binary's text is 16 bytes smaller. The keys the trend gate reads as
worse, with the values they land on: `run_alloc_bytes` 373,249,458,
`run_bytes_freed` 9,004, `encode_alloc_bytes` 665,835,936,
`encode_append_fast` 42,312,400, `encode_append_grow` 5,600,
`encode_bytes_malloc` 5,600, `oneshot_alloc_bytes` 2,960,506,
`oneshot_bytes_malloc` 14, `text` 3,485,808, `live_alloc_bytes` 534,764,560,
`live_bytes_malloc` 5,600,
`a_builder_that_outgrows_its_buffer_is_never_held_twice_alloc_bytes` 368,258,
`a_builder_that_outgrows_its_buffer_is_never_held_twice_allocs` 16,
`a_builder_that_outgrows_its_buffer_is_never_held_twice_append_fast` 99,987,
`a_builder_that_outgrows_its_buffer_is_never_held_twice_append_grow` 13,
`a_builder_that_outgrows_its_buffer_is_never_held_twice_bytes_malloc` 13,
`a_cycle_of_four_rewinds_once_a_trip_alloc_bytes` 5,397,104,
`a_cycle_of_four_rewinds_once_a_trip_allocs` 141,193,
`a_cycle_of_four_rewinds_once_a_trip_append_fast` 99,985,
`a_cycle_of_four_rewinds_once_a_trip_append_grow` 15,
`a_cycle_of_four_rewinds_once_a_trip_bytes_malloc` 15,
`a_cycle_of_four_rewinds_once_a_trip_held_peak_bytes` 298,940,
`a_cycle_that_allocates_nothing_needs_no_bracket_alloc_bytes` 979,198,
`a_cycle_that_allocates_nothing_needs_no_bracket_allocs` 32,
`a_cycle_that_allocates_nothing_needs_no_bracket_append_fast` 139,974,
`a_cycle_that_allocates_nothing_needs_no_bracket_append_grow` 27 and
`a_cycle_that_allocates_nothing_needs_no_bracket_bytes_malloc` 27.

## 2026-09-25 — a short token is shared

The decoder turns every string token into a fresh string: an allocation, a
utf-8 validation and a copy. Runbench reads 861,498 tokens a run, 840,807 of
them four to seven bytes long, and large.json holds 898 distinct ones. The
same few hundred keys and short values come back on every document.

A token of four to seven bytes is now looked up first. Its bytes and length
make a 64-bit key, and a two-way, direct-mapped cache of 4,096 slots hands
back a permanent string for a key it has seen. A miss validates the token and
fills the slot if either way is free; a full pair sends the token down the
ordinary path, so the cache never evicts and its storage is bounded by its
width. A hit needs no validation, since the bytes it matched were validated
when the slot was filled. The strings live in a static store that the survival
test recognises by address, so a rewind neither frees nor copies them. On
runbench the cache takes 840,175 hits and 632 misses, and every miss fills a
slot. A one-way cache of the same width missed 20,302 times.

The first version kept the strings in `malloc` storage, which the survival
test treats as dying at a rewind, and the carry copied them out: runbench rose
3.1 million instructions. The second tested the store at the top of every
survival check and cost deepbench 6.9 million. The test now runs only for
pointers outside the arena.

Runbench is projected at 1,326,066,150, 6,786,450 below (-0.51%), and
jsonbench at 891,229,689, 10,945,028 below. A program that decodes once pays
for the fills and gets few hits: oneshot is projected at 14,842,561, 567,919
above. Deepbench rises 1,617,979 and livebench 241,358; both arrived with the
change and neither was isolated. `perm_peak_bytes` on the run program rises
from 16,400 to 31,568, the 632 filled slots at 24 bytes each.

The mem vein's new fixture, `a_short_token_is_shared`, decodes a six-token
document a hundred times and pins 402 allocations and 120 permanent bytes.
With the cache disabled it reads 1,002 allocations and 19,232 bytes of string
headers; the ratchet row `token_cache` makes that mutation.

CI read the tree at abc4015e: runbench 1,326,065,791 and oneshot
14,842,337, 359 and 224 below the projections, `codegen_instructions_dev`
125,569,980, 15,362 above #1650's reading, and
`codegen_instructions_release` 698,322,865, 239,034 below it. Those rows are
CI's.

Every binary's text is 4,672 bytes larger. The keys the trend gate reads as
worse, with the values they land on: `run_perm_live_bytes` 15,168,
`run_perm_peak_bytes` 31,568, `perm_live_bytes` 15,168, `perm_peak_bytes`
15,168, `encode_alloc_bytes` 665,564,160, `encode_perm_live_bytes` 15,168,
`encode_perm_peak_bytes` 15,168, `oneshot_perm_live_bytes` 15,168,
`oneshot_perm_peak_bytes` 15,168, `work_deepbench` 366,141,730,
`work_digestbench` 5,542,082, `work_indexbench` 2,538,557, `work_pendbench`
181,896,213, `work_readbench` 4,631,851, `work_scanbench` 282,021, `text`
3,551,216, `live_alloc_bytes` 534,492,784, `live_perm_live_bytes` 15,168 and
`live_perm_peak_bytes` 15,168.

## 2026-09-25 — a map key is written as a key

The json encoder wrote each map key through `encode_onto`, the same group that
writes every value, so each key paid for a type test over eight arms before it
reached `escape_onto`, and the colon after it was one more single-byte append.
A key is always a string. `entry_onto` now hands it to `key_onto`, whose
parameter is typed `k:string`, and `key_onto` appends the closing quote and
the colon as the two-byte literal `":`. The opening quote is appended by the
caller, straight after the brace or the comma.

Five arrangements of the same bytes were measured on runbench against the tree
before this one:

    escape_onto on the untyped k, `":` as one literal       +22,332,650
    key_onto typed, opening quote inside it                 -16,643,789
    key_onto typed, `{"` and `,"` as two-byte literals      -25,555,650
    as above with `{` and `,` apart from the quote          -26,042,668
    as above with the closing quote apart from the colon    -15,858,366

The first line is the reason for the typed parameter: the same call made on
the bare `k` of the entry pattern costs more than the old dispatch did. Why
the two-byte literal pays after a key and costs before one was not isolated.

Runbench is projected at 1,300,023,123, 26,042,668 below (-1.96%), livebench
at 1,905,214,048, 113,377,925 below (-5.6%), and oneshot at 14,557,927,
284,410 below. No other compiled program moves. The interpreter runs the same
library, and the interpreted corpus falls 18,767,151 on the container, which
projects `interp_instructions` at 595,472,297. The ratchet row `key_typed`
sends the key back through the untyped escape, which leaves the output the
same and put runbench at 1,337,789,395 when it was tried by hand.

`append_fast` falls because there are fewer appends, and the checker visits
one definition more. The keys the trend gate reads as worse, with the values
they land on: `run_append_fast` 8,256,960, `run_perm_allocs` 93,
`oneshot_append_fast` 90,477, `front_end_visits` 7,416,
`emitted_other_branches` 8,024, `emitted_other_calls` 9,671,
`emitted_other_defines` 1,556, `emitted_other_lines` 83,727, `text` 3,548,064,
`live_alloc_bytes` 534,601,584, `live_append_fast` 31,135,470 and
`a_literal_appended_across_a_rewind_append_fast` 2,080.

CI read the tree at 1ab7904e and matched the work rows to the instruction. The
library carries one definition more, and the compile side paid for it:
`compile_instructions` 25,269,300 (+65,047), `entry_instructions` 85,326,396
(+111,322), `library_instructions` 85,855,270 (+81,403), `compile_allocs`
14,272 (+32) and `compile_peak_bytes` 710,281 (+1,609). The interpreted run
reads 595,493,632, 21,335 above the projection, with `interp_allocs` at
899,769 (-26,143) and `interp_peak_bytes` at 720,417 (+1,609). Those rows are
CI's.

## 2026-09-25 — a tail call carries its group

The interpreter runs a tail call through the dispatcher's loop: `eval_tail`
hands back the callee's name and arguments, and the loop looks the name up in
`fns` to find the overloads it dispatches over. `eval_tail` had already asked
`callee_of_ref` whether the callee is a group, and that answer holds the
group. The interpreted corpus makes 107,607 tail calls, and each paid a hash
of the name and a compare of its bytes to find the same group again.

`Flow::Tail` now carries the group beside the name, and the loop takes it. The
interpreted run falls 13,651,102 instructions on the container, 2.3%. CI read
`interp_instructions` at 584,588,724 at e8870bbd, 10,904,908 below its reading
of the typed key.

The ratchet row `tail_group` puts the lookup back and leaves the carried group
unused. The interpreted run read 595,873,686 with it, back where it started.
No compiled program moves. The compile rows are a layout vein and can move
with any edit to the compiler's Rust, so they are left for CI to read.

## 2026-09-25 — a length, an index and a literal become words directly

The interpreter keeps an int in a machine word when it fits, but three kinds
of site still reached it through a `BigInt`. `length`, `char_code`, `find`,
`number_span`, `now` and a byte index built a `BigInt` from a `usize` or an
`i64`, which allocates its digits, and handed it to `Int::from` to be shrunk
back into a word. The interpreted corpus does that 68,891 times, 33,239 from
builtins and 35,652 from byte indexing. Those sites now build the word
directly, which takes 2,095,679 instructions off the interpreted run.

The other two read a literal the parser stores as a `BigInt`: evaluating an int
literal, 125,002 times, and matching an int pattern, 85,873 times. Both went
through `BigInt::to_i64`, which walks the digits in general. A number that fits
a word has at most one digit, so `word_of` reads it and its sign directly; that
took 1,184,161 off. The conversion also saved six registers on every call
because its rare path clones the `BigInt` into an `Rc`. With the clone moved
into a cold function and the rest inlined into the literal's evaluation, a
further 3,105,935 came off.

The interpreted run falls 6,385,775 instructions on the container, 1.1%. CI
read `interp_instructions` at 578,807,674 at f774749e, 5,781,050 below its
reading of the tail. The
spec `the_ends_of_the_word_read_back_as_words` pins the two ends of the word:
the largest positive literal, the first past it, and a sum that lands on the
most negative word from outside it. Letting that sum stay a `BigInt` printed
`false` where `under + 1 == least` should be true, and reading the first
literal past the word as a wrapped `i64` sent it to the wrong clause. The
ratchet row `word_read` sends a literal through `to_i64` first; the
interpreted run read 577,707,831 with it. No compiled program moves, and the
compile rows are left for CI to read.

## 2026-09-25 — a literal divisor cannot fail

Inference gave every `/` and `%` a possible err, because division by zero is a
failure. The runtime fails only on a zero divisor, and the checker already
refuses a divisor written as the integer zero where it is reached unguarded.
So a divisor written as any other literal cannot fail, and the err was noise.
It was not harmless: it flowed through `push` into escape's accumulator, whose
parameter set then carried an err. The dispatcher tests every parameter that
may hold a failure on entry, so escape's builder, 1.55 million iterations, paid
that test on each one.

`/` and `%` now add an err only when the divisor is not a nonzero literal. On
the container, over the carried group's tree: runbench -9,744,067 (-0.75%),
livebench -27,388,087 (-1.44%), escapebench -2,451,022 (-3.59%), deepbench
-1,344,000, jsonbench -531,900, basket -133,385, oneshot -71,532 and
pendbench -1,000. encodebench rises 492,380 (+0.018%), which is layout.
indexbench and digestbench move 14 each way. The rows are projected from those
readings and CI's reading replaces them.

Every binary that carries such a remainder gets shorter: the decoder's calls
484 -> 482, branches 510 -> 508, lines 5,495 -> 5,479, and runbench's .text
416,760 -> 415,736. The front end's visits on the compile corpus fall 7,416
-> 7,411. The allocation counters do not move. The compile rows this host
refuses are left for CI.

The ratchet row `literal_divisor` puts the err back on a literal divisor. The
emitted-code gate reads the decoder's old counts with it. A zero written as a
literal divisor was also tried as nonzero, to find a program that would show
it; with the checker refusing the unguarded integer zero, none of the shapes
tried printed differently on the native engine and the interpreter.

The trend gate reads six keys as worse against its baseline, which predates
the stack under this change; this change lowers or holds each of them. They
land on `work_deepbench` 364,797,730, `work_digestbench` 5,542,068,
`work_indexbench` 2,538,571, `work_pendbench` 181,895,213,
`emitted_other_defines` 1,555 and `text` 3,544,496.

CI read the carrier at e63ff428. The work rows landed within 14 instructions
of the projection, and the compile side moved where the container could not
see it, every row down: `codegen_instructions_dev` 125,569,980 -> 123,915,790,
`codegen_instructions_release` 698,322,865 -> 695,954,249, `emit_instructions`
29,311,866 -> 28,882,533, `compile_instructions` 25,269,300 -> 25,267,312,
`entry_instructions` 85,326,396 -> 85,321,306 and `library_instructions`
85,855,270 -> 85,850,050. Fewer entry tests is less IR for clang to compile
and less for the emitter to write.

Against main, the trend gate reads nine keys as worse across the six carried
changes, and they land here: `run_append_grow` 1,260, `run_bytes_malloc`
9,120, `oneshot_append_grow` 14, `oneshot_perm_allocs` 9, `live_append_grow`
5,600, `live_perm_allocs` 8 and `a_literal_appended_across_a_rewind_perm_allocs`
16, which are the half-step grow's and the token cache's allocation shapes;
and `work_encodebench` 2,677,016,790 (+0.04%) and `work_oneshot` 14,486,409
(+1.49%). The two work rows arrived with the carried changes and no one of
them has been isolated as their cause; runbench, the row the objective
weighs, fell 7.4% across the same set.

The ratchet found what the token cache left behind. Its row `slice_words`
sends a four-to-seven-byte slice through `k_str_n` instead of the two-word
copy, and the gate stayed green: the cache returns every slice of those
lengths before the copy is reached, hits from the cache and misses from
`k_token_miss`, so the copy was dead code and the mutation reached nothing.
The branch, the row and its mutation are gone. Every benchmark reads within
14 instructions of the tree that kept them, which is what dead code costs.
The two codegen rows moved on CI because clang compiles a runtime with one
branch fewer: `codegen_instructions_dev` 123,915,881 (+91) and
`codegen_instructions_release` 695,954,277 (+28). Welfare holds at its floor.

## 2026-09-25 — a proven tag is assumed

Inference proves many parameters are exactly one kind of heap value: a list
the builder threads through, a string a scanner walks, a map the encoder
fills. The emitter already records that set, and its own folds read it. The
helpers it inlines do not: `k_b_push_mut_fast` asks whether its list is a
list, and the string and map helpers ask the same of theirs, by testing the
tag word they were handed. LLVM cannot answer those tests, because the tag
word arrives as a function parameter.

The dispatcher's entry now tells it. A boxed parameter whose set is exactly
a string, a list, a map or bytes gets an `llvm.assume` that its tag is that
kind's, and LLVM folds every dominated test of it. A program that declares a
subtype gets none, because a subtype's value carries tag 15 whatever it
wraps; `tag_switch_shape` refuses the same programs for the same reason. No
program here shows that the gate is needed: inference does not give a
subtype-wrapped string the plain string set in any shape tried.

On the container, over the literal divisor's tree: runbench -66,651,001
(-5.17%), jsonbench -76,795,501 (-8.62%), encodebench -146,237,823 (-5.46%),
livebench -42,089,654 (-2.24%), oneshot -615,489 (-4.25%), escapebench
-2,399,986 (-3.65%), digestbench -91,203, indexbench -40,097, basket
-260,382, widebench -224,004, deepbench -64,000, scanbench -172 and
pendbench +998. Every benchmark prints the same bytes. The instruction rows
are projected and CI's reading replaces them. The allocation counters do not
move.

Machine code shrinks: runbench's .text 415,736 -> 408,440, encodebench's
253,976 -> 248,184. The emitted IR grows by a call line per assume: the
decoder's calls 482 -> 539 and lines 5,479 -> 5,651, runbench's calls 3,531
-> 3,781 and lines 27,291 -> 28,042, and the compile-cost module's lines
1,057 -> 1,067. The codegen and compile rows are refused on this host and
left for CI.

The ratchet row `tag_assumed` inverts the subtype test, so no program without
a subtype gets an assume; runbench read 1,290,279,893 with it, the tree
before this change.

While building a fixture for the gate, a divergence turned up that predates
this change and does not depend on it. A value of `type name string` is a
string to the interpreter, which prints `length (name "kanso")` as 5 and
`"{s}!"` as `hi!`. Natively, on both tiers, `length` refuses it and the
string builder refuses to start from it. That needs its own fix.

Against main, the trend gate reads seventeen keys as worse across the changes
this branch carries, and they are named here with where they land. The
half-step grow and the short-token cache moved the allocation shapes:
`run_append_grow` 1,260, `run_bytes_malloc` 9,120, `oneshot_append_grow` 14,
`oneshot_perm_allocs` 9, `live_append_grow` 5,600, `live_perm_allocs` 8 and
`a_literal_appended_across_a_rewind_perm_allocs` 16. The assume lines are the
emitted rows: `emitted_calls` 539, `emitted_lines` 5,651,
`emitted_other_calls` 10,323, `emitted_other_lines` 85,135, `module_calls` 103
and `module_lines` 1,067. `text` lands on 3,520,752 summed over the fourteen
binaries, which the token cache's store raised and the assume lowered. The
work rows `work_deepbench` 364,733,730, `work_pendbench` 181,896,211 and
`work_scanbench` 281,849 sit within 0.06% of main's.

CI read the change at 2673fa68, and the work rows landed within 350 of the
projection. The release build is what moved furthest:
`codegen_instructions_release` 695,954,249 -> 406,427,590, 41.6% less. With
the tag tests folded as soon as the assume is seen, the release link's
optimiser has far fewer branches, blocks and inlining candidates to work
through, and the objective prices that as production cost. The dev tier
compiles the assume lines without folding anything with them,
`codegen_instructions_dev` 123,915,790 -> 124,017,062 (+0.08%), and the
emitter writes them, `emit_instructions` 28,882,533 -> 29,156,813 (+0.95%).

## 2026-09-25 — a subtype is its parent to a builtin

A value of `type name string` is a string to every builtin on the
interpreter, whose `call_builtin` unwraps each argument before it reads any.
The native engines handed the builtin the wrapper itself, tag 15, and the
runtime refused it: `length`, `text/slice`, `text/trim`, `text/split`,
`text/chars`, `text/to_int` and `text/char_code` all died on a `name` where
the interpreter answered, and a template that opened with one could not seed
its builder. Operators, comparison and rendering already looked through the
wrapper, which is why `==` and `"{n}"` agreed. The divergence predates the
tag assume and turned up while writing that change's fixture.

In a program that declares a subtype, the emitter now routes every real
builtin's arguments through `k_unsub`, and a builder's seed as well. A type's
constructor comes through the same emitter and is left alone, since it must
see the value it wraps: routing it too lost a line of
`a_child_arm_beats_its_parent`. A program with no subtype emits nothing new.
The micro fixture `a_string_subtype_is_a_string_to_a_builtin` runs the seven
builtins and a template on both engines and a release build; without the
routing the native engines printed nothing. The ratchet row
`subtype_unwrapped` turns the routing off.

Seeding the builder was first done in `k_b_str_builder`, and that cost
runbench 100,538 instructions: the release link optimises the runtime with
the program and inlines the builder into `escape_onto`, where one more test
changed what it made. Done at the call site instead it costs nothing there.
What remains is the function's own presence: `k_unsub` makes every binary 48
bytes longer and moves the layout the link sees, projected at runbench
+5,742, encodebench +149,706 and livebench +54,519 on the container. The
compile rows are left for CI.

The trend gate reads `text` as worse; it lands on 3,521,424 summed over the
fourteen binaries, 672 bytes above the tag assume's, which is `k_unsub`
forty-eight bytes at a time.

CI read the change at 061eaef6. The work rows landed within 28 of the
projection: runbench 1,223,633,797, 5,728 above the tag assume's. The
compile rows moved by the function's presence and nothing else:
`emit_instructions` 29,156,813 -> 29,158,246, `codegen_instructions_dev`
124,017,062 -> 124,021,731 and `codegen_instructions_release` 406,427,590 ->
406,403,837.
