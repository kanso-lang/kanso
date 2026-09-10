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

## 2026-09-08 (fifth) — the entry path compiles and nothing counted it

Searched the log, the archive and design/ before filing: `compile_parsed_entry`
appears in the 2026-09-08 entries for kanso#1324, #1325 and #1326 and in
kanso#1329's revert, and every one of them measures it on a corpus that lived in
a session's temporary directory. None of them asks why there is no vein.

**The two compiles.** `kanso check <directory>` is a module and goes through
`compile_module_inner`. `kanso check <file>` with a top-level expression is an
entry and goes through `compile_parsed_entry`, which merges the imports itself
and runs its own whole-program check at src/lib.rs:160. Proved by probe, not by
reading: an `eprintln!` at the entry site fires once for
`bench/entry_corpus/main.kso` and not at all for `kanso check
bench/compile_corpus`.

Every compile gate in the tree checks a directory. So the call at line 160 was
watched by nothing, and `KANSO_PHASES` cannot separate the two — `load_dependencies`
compiles each import through the module path, so a phase report over an entry is
the union of both.

**What that cost.** kanso#1326 projected a RISE of 629 from this container and CI
read a FALL of 367. kanso#1324 and #1325 landed on numbers no CI job could
reproduce. Two further findings — the synthetic-twin skip below and kanso#1329's
reverted reorder — were measured and could not be landed against anything.

**The vein.** `bench/entry_corpus/main.kso` names ten imports and uses each,
following bench/compile_corpus's rule that a workload is named rather than
inherited; it imports ten where the compile corpus imports four, because the
entry path's own work is the merge and the check over everything the imports
bring and a corpus with one small import measures mostly the module path
underneath it. `scripts/gates/entry_instructions.sh` counts it the way
`compile_instructions.sh` counts its own — same box, same emptied environment,
same pinned tunables, same `kanso::main` anchor — and every reason for those is
left in the original rather than restated.

`bench/entry_instructions_golden.txt` opened holding zero, because the row is
CI's and this container reads high against CI's rustc. Round one was red on
purpose and CI answered **163,886,731**, on binary sha 3c53d0acdbcb — the same
sha that counted `compile_instructions=48,757,859` in the same job, so both
rows answer for one build. The container had projected 165,183,406 from the
same recipe minus the host check: 1,296,675 high, or +0.79%, which is the
offset already recorded between rustc 1.94.1 here and CI's 1.98.1. The
projection was right about the size and could not have been recorded as a row.

**And the trend gate did not walk the new golden.** Found by asking which
files in bench/ `scripts/trend_gate/trend_gate.kso` names, which its own
comments say is the only method that has ever found one of these. The list has
been short five times: three cost goldens nobody entered, then readbench —
whose golden the gate could not see while two of its rows were welfare terms —
then livebench, then the consolidated run program. This would have been the
sixth, in the very PR that exists because a compile the gates could not see
went unpriced.

Two files in bench/ are unwalked and one of them belongs that way:
`bench/compile_libraries_golden.txt` holds five sonames rather than counters
and its own gate diffs it byte for byte. So the excuse list is one line long,
and `tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads bench/
off disk, asks the gate which files it names, and fails on anything neither
walked nor excused. Its three assertions were each watched red: dropping the
entry golden from the gate names it in the failure; an excuse for a file that
is not there fires the second; an excuse for a golden the gate already walks
fires the third. Finding this by hand a sixth time was not a plan.

**The ratchet row separates the two veins, measured.** The mutation asks the
entry's whole-program check twice. In the box:

    entry_instructions   165,183,406 -> 190,382,616   +25,199,210  (+15.25%)
    compile_instructions  49,162,592 ->  49,162,592   byte-identical

A vein whose defects another vein already catches would not be worth its
callgrind run. This one is worth it: the compile row cannot move for a defect on
this path however much work it does.

**MEASURED, NOT SHIPPED — the synthetic-twin skip.** `enroll_bare` gives every
exported declaration of an imported module a twin under its short name, cloning
the whole declaration, body and all. On the entry corpus that is 145 of 882
declarations and 155 of 1,035 statements, and the whole-program check walks both
copies. It is visible as a defect only under kanso#1329's reshape, where six
error fixtures reported one diagnostic twice at the same line AND the same
column — `field_missing/play` and its twin `play`, both at span 4 of the same
file, each answering `check_field_exists` once. The module path has not had this
since kanso#1328: `canonicalize_bare_aliases` takes the twins out before the
check there, which is why `compile_instructions` reads synthetic=0 and this whole
thread is invisible to it.

Skipping synthetic declarations in the three checks that are pure body walks —
`check_build_blocks`, `check_none_in_collections`, `check_field_exists` — was
built and measured in the box:

    entry_instructions   165,183,406 -> 164,922,557   -260,849  (-0.1579%)
    compile_instructions  49,162,592 ->  49,170,337     +7,745  (+0.0158%)

The entry row falls 34 times what the compile row rises, and the compile row's
rise is the branch itself plus layout — the module path has no twins to skip.
**Welfare reads the compile row and not the entry row**, so by the objective as
it stands today this change is a small loss. That is a question about the
objective's inputs rather than about the change, and it is not settled here: the
skip is left out of this PR, and what lands is the vein it would be measured
against. Recorded as OPEN.

The remaining 0.85% of kanso#1329's reverted reorder is inference, which is 22.6%
of the entry compile against `check_merged`'s 38.3%. `infer` indexes declarations
positionally — `vec![0; program.fns.len()]`, groups by index — so it does not take
a `continue`, and skipping the twins there is a different change from this one.

- **DONE** — the entry vein: corpus, gate, golden with CI's row, CI step and
  summary row, ratchet row, sweep membership, the trend gate's own list and
  the spec that keeps it honest. The derivations in
  `tests/the_compile_sweep_names_every_compile_gate.rs` walked past
  `bench/entry_*` and now do not, and `the_compile_row_holds_one_value` covers
  both instruction goldens rather than one, because the one-row-one-value ruling
  is about the shape of a row and not about a filename.
- **OPEN** — the synthetic-twin skip, measured above, held on the objective
  question: should welfare's compile term read the entry path as well as the
  module path? Two compiles, one term.
- **OPEN** — the twins inside `infer`, worth most of the remaining 0.85%.

## 2026-09-08 — the corpus change banked a six-point fall, and the precedent says it should not have

Clay, reading the published chart on kanso-lang.dev/numbers: why didn't the new
corpus ruling go retroactive, so there is no drop? The answer is that kanso#1321
left the three compile baselines where they were, and the score fell 6.29 points
for a change that touched no compiler code.

    2026-09-08 01:53   welfare 66.0241 -> 59.7360      #1321
    compile_instructions   19,316,962 -> 52,603,220
    compile_allocs             11,613 ->     31,596
    compile_peak_bytes        375,222 ->    789,740

**The reason #1321 gives.** "The baselines those three are divided by were taken
on lib/json and are left where they are: the index has an arbitrary origin and
only its direction and the size of its moves mean anything, so re-deriving a
historical compiler's cost on a corpus that did not exist then would buy
nothing. This is a change of origin, not a regression."

**The origin is arbitrary; the move is not.** That sentence is the argument
against leaving it. A reader of the chart sees a six-point fall, and a fall is
what the index says a change made worse. CLAUDE.md is explicit: "Moving the
floor to accommodate a change while leaving the weights alone is declaring the
objective wrong without saying so."

**Re-deriving history was never what the precedent asks for.** It asks for one
measured factor per row, taken on the same head under both definitions -- which
#1321 already measured and recorded. Scaling each baseline by its own factor
restores each ratio exactly:

    row                     factor   ratio left as-is   ratio re-based   before
    compile_instructions    2.7232         1.0753           2.9282       2.9282
    compile_allocs          2.7207         1.9658           5.3483       5.3483
    compile_peak_bytes      2.1047         1.0373           2.1833       2.1833

    baselines: 56,563,967 -> 154,032,855 · 62,110 -> 168,985 · 819,217 -> 1,724,228

Every ratio returns to the digit it held before the corpus moved, so the score
does not move and the chart is flat across the change.

**Three precedents, all in this repository.** The archive: "RE-BASELINED THE
SAME WAY #729 WAS: `basket_allocs` scaled by exactly the factor", and
"RE-BASELINED SO IT BANKS NOTHING, the same method as #729 and #741." The floor
file's own history at 73.53: "the compile row counts the compiler's own frame;
the baseline is re-based by the same 465,864." And at 84.51, scanbench entering
the corpus: "The score does not move on entry, by design."

**What is not in dispute.** #1321 is right that a term measured on a library
moves whenever that library changes its imports, and the fixed corpus is the
ruling of 2026-09-08. Nothing here argues against the corpus. The question is
only whether the change of measurement banks a fall, and the answer this
project has given three times is that it does not.

**OPEN, and cloud's**, since the baselines and the floor are code. Two readings
are available and both were taken on the changeover head, so no re-measurement
is needed. If the fall is kept deliberately, that is a claim about the weights,
which CLAUDE.md says is settled before the floor moves rather than after.

## 2026-09-08 (second) — the compile term read one compile out of two

Searched the log, the archive and design/ before filing: the re-basing precedent
is the 2026-09-05 entry for kanso#1242 and the archive's #729 and #741; the entry
vein opened in kanso#1330, whose own entry above closes with this as an OPEN
question — "should welfare's compile term read the entry path as well as the
module path? Two compiles, one term." This answers it.

**The term summed one path.** `kanso check <directory>` takes
`compile_module_inner`; `kanso check <file>` with a top-level expression takes
`compile_parsed_entry`, which merges the imports itself and runs its own
whole-program check. Every `kanso run` takes the second, and nothing counted it
until kanso#1330. The compile term now adds the two rows:

    compile_instructions   48,757,859 + 163,886,731 = 212,644,590

**The baseline moves with it, so the score does not.** The entry vein has no
reading at this objective's epoch, because its corpus did not exist then, so its
baseline is imputed at the ratio the module row holds:

    r = 154,032,855 / 48,757,859 = 3.1591390221
    entry baseline    163,886,731 * r = 517,740,967
    summed baseline   154,032,855 + 517,740,967 = 671,773,822
    summed ratio      671,773,822 / 212,644,590 = 3.1591390216

The preservation is algebraic — `(cb + ec*r) / (cc + ec) = r` for any `ec` — and
it was measured rather than trusted: welfare reads 66.29 against a floor of 66.29
before and after.

**What the shape of the change turned out to be.** The plan recorded for this
work said it was one line in `bench/objective_sources.txt`, and that was wrong.
welfare does not build its counters from that file; it reads the goldens itself,
in `fn measured` and a reader chain, and objective_sources is the LINK that the
trend gate's `shifted?` and `tests/the_objective_reads_what_the_gate_watches.rs`
replay. Both halves are needed and they are different files. The spec was watched
red before it passed: with the second key removed it reports `compile_instructions
reads 212644590 from welfare and 48757859 from compile_instructions`.

Two smaller things the edit forced. The compile veins now reach `measured` as one
list rather than as four positional arguments, because a fourth `compile[4]!` at
the call site overflows the 80-column rule by three characters; the next golden
to join is now one list entry and one binding. And the binding is `ent`, because
`entry` is bare-enrolled from an import and the resolver refuses to shadow it.

**golden_prose needed the same golden and would not have said so.** Its
`golden_for` answers for `decode`, `encode` and `compile` and returns `[]` for
any other family, so a page span written `data-golden="entry.entry_instructions"`
resolves against an empty golden — a span nothing watches, which is the exact
failure that gate was widened to fix in kanso#1047. The entry row joins the
`compile` family instead, whose own comment already licenses it: the key names do
not collide.

**Still to ship: the twin skip.** kanso#1330 measured it and deliberately left it
out, because the objective could see the module row's +7,745 and not the entry
row's −260,849. Under the sum it is a fall of 253,104, −0.1190%. It is a separate
change because it moves two goldens whose values are CI's, and this one moves no
counter at all.

## 2026-09-08 (third) — the twin skip ships, now that the objective can see it

Searched the log, the archive and design/ before filing: this thread is the OPEN
item at the end of the kanso#1330 entry above ("the synthetic-twin skip, measured
above, held on the objective question"), and the entry above that, for kanso#1331,
answered the question it was held on. The archive's prior art is `enroll_bare`
and `canonicalize_bare_aliases` in the kanso#1328 entry. Nothing else is new.

`enroll_bare` clones every exported declaration of an imported module under its
short name — body and all, `synthetic = true`. On `bench/entry_corpus` that is
145 of 882 declarations and 155 of 1,035 statements. Three checks in
`check_merged` are pure body walks and were the only three of sixteen that read
those clones: `check_build_blocks`, `check_none_in_collections` and
`check_field_exists`. Thirteen others already skip them. These three now do too.

CI's rows, which are the ones this vein may hold:

    entry_instructions   163,886,731 -> 163,612,976   -273,755  (-0.1671%)
    compile_instructions  48,757,859 ->  48,761,165     +3,306  (+0.0068%)

Under kanso#1331's summed compile term that is

    212,644,590 -> 212,374,141   -270,449  (-0.1272%)

and welfare moves 66.2874470488728 -> 66.28984813328917, banked in this PR.

**compile_instructions RISES to 48,761,165 and that is the change's own doing.**
The module path has had no twins to skip since kanso#1328 put
`canonicalize_bare_aliases` in front of the check there, so the walk does the
same work and now pays for a test that can never say yes; src/check.rs is the
compiler, so its bytes and the layout under them move with the edit as well.
The row is traded against the entry row's fall, which is 83 times it.

**The container projected the deltas and got the digits wrong in both
directions**, which is the kanso#1326 lesson again. It read 165,183,406 ->
164,922,557 for the entry row (-260,849) against CI's -273,755, and +7,745 for
the module row against CI's +3,306. Sign and order of magnitude carried across
rustc 1.94.1 here and 1.98.1 there; nothing finer did. Both rows are copied out
of the job log.

The compile row rises because the module path has no twins left to skip — since
kanso#1328 `canonicalize_bare_aliases` deletes them before the check there — so
what that row records is the branch itself plus layout. Under kanso#1331's summed
term the entry row's fall is 34 times it, and the trade lands the right way up.

A twin's body IS the original's body under a second name, and both copies are in
the same merged program, so a twin can answer nothing the original answers
differently. That is why no diagnostic moves. The doubling was visible once, under
kanso#1329's reverted reorder: six error fixtures reported one diagnostic twice at
the same line and the same column, `field_missing/play` and its twin `play`. On
current main the dependency's own compile refuses first, so the second copy never
reaches a reader — which is why this change has counters and no fixture.

`infer` is deliberately not given the skip. It indexes declarations positionally
(`vec![0; program.fns.len()]`, groups by index), so a `continue` misaligns it.
That is the rest of kanso#1329's reverted reorder and stays open.

## 2026-09-08 (fourth) — a compile left its IR behind, and the guard for it read the litter

Searched the log, the archive and design/ before filing: the archive names
`cached_program_binary` once, in the entry that introduced the per-pid IR path
after concurrent builds segfaulted inside clang. Nothing records the leak, and
nothing else in the tree measures what a `kanso run` leaves in the temp
directory.

`kanso run` caches its binary under a hash of the IR and `runtime.c`. On a miss
it writes the IR to `kanso_run_<key>_<pid>.ll`, hands that to clang, and renames
the staging binary into place. The `.ll` was never removed. One cold run in an
isolated TMPDIR leaves four files and exactly one of them is a leak:

    kanso_run_<key>                     185 KB   the binary cache, intentional
    kanso_run_<key>_<pid>.ll             42 KB   THE LEAK, one per cache MISS
    kanso_runtime_dev_<hash>.c          415 KB   content-keyed, shared, bounded
    kanso_runtime_dev_<hash>.o          258 KB   likewise

At 42 KB a miss this reached about 112,000 files in one long-lived container,
which is the whole of a session's disk allowance, and it was the true cause of
four "spec failures" chased as real during kanso#1330. The staging path needs no
removal because `rename` consumes it.

**The obvious fix blinds a real guard, which is why this took two attempts.**
`tests/concurrent_build.rs::two_builds_of_one_program_do_not_share_a_file`
proved that two concurrent builds are handed different paths BY FINDING THE
LEFTOVER `.ll` AND READING A PID OUT OF ITS NAME. Delete the file and the guard
has nothing to look at; the first draft of this change shipped the removal, the
guard went green on an empty set, and the whole thing was backed out. A guard
resting on a bug fails the moment the bug is fixed.

So the guard now watches during the build rather than counting what is left. The
IR is written before clang starts and removed after it returns, and a cold run of
that fixture is ~133 ms with ~100 ms of that window, against a one-millisecond
poll. It cannot pass vacuously: seeing no IR file at all is a failure with its
own sentence, because "the race never happened" and "the race happened and was
safe" must not look alike.

**Why the sibling is not enough on its own**, measured rather than assumed. With
the pid stripped back out of the path, this guard caught the defect in 10
sittings of 10 and again in 5 of 5 after the rebase, where
`many_builds_of_one_program_all_answer` caught it in 9 of 10 — it passed once
with the bug in place, because whether two processes actually overlap on the
file is the race, and the race is not owed to anyone. One of the two is a
corruption that may or may not happen; this one is the decision that allows it.

No counter moves. `kanso run`'s temp handling is not on any measured path: the
compile veins run `kanso check`, which never reaches `cached_program_binary`.

## 2026-09-08 (fifth) — the entry reorder re-derived, and the alias pass refuses a valid program

Searched the log, the archive and design/ before filing: the 2026-09-08 entry
"the entry path's reorder costs a diagnostic, and is reverted" is this same
change, and the entry after it records the vein that now measures it. The
archive's `canonicalize_bare_aliases` entries are about what the pass costs, not
about when it runs. What neither has is the failure set as it stands today.

**DECLINED, a second time, on wider grounds than the first.** Hoisting
`canonicalize_types` and `canonicalize_bare_aliases` out of
`compile_parsed_entry`'s success arm is worth, on 9ec9f7e9 in this box:

    entry_instructions   164,922,557 -> 163,362,291   -1,560,266  (-0.9461%)
    compile_instructions  49,170,337 ->  49,170,337            0

against kanso#1329's -1,674,396 (-1.0136%) for the same edit a few merges
earlier. The module row is byte-identical here, which is what a
`compile_parsed_entry`-only edit should read; CI has not been asked, so that is
a projection.

**The failure set has moved since the revert.** `scripts/module_differential`
reads 0 wrong on 9ec9f7e9 and 2 wrong with the reorder, and only one of the two
is the one kanso#1329 recorded:

    a call from the entry at the wrong arity
      error[arity]: no 2-argument arm of `m/one` (arms take 1)
      where the program says `one`

    a type and a function sharing a name
      expected it to compile; got
      error[opacity]: `m/thing` is foreign -- only `m` builds a `thing`

The sibling-arity case kanso#1329 also lost now passes. In its place is a
program that compiled before the reorder and does not compile after it, which is
a worse thing than a diagnostic quoting the wrong spelling. `m/b.kso` declares
`pub fn thing _`, the entry writes `print "{thing 0}"`, and the answer should be
`m/thing 0`.

**Both objections are the alias pass, not the type pass.** Moving
`canonicalize_bare_aliases` alone and leaving `canonicalize_types` in the success
arm leaves the differential at the same 2 wrong, and reads slightly BETTER than
moving both:

    entry_instructions   164,922,557 -> 163,353,361   -1,569,196  (-0.9515%)

8,930 better than the pair, because the alias pass deletes the twins before
`canonicalize_types` walks them rather than after. `thing` in `print "{thing 0}"` is a CALL, so the alias pass rewrites it to
`m/thing` like any other bare name, and the opacity check then reads a qualified
name as a foreign type construction. One pass, one mechanism, two checks that
read a name after something else has rewritten it.

**What would make it shippable**, stated more narrowly than kanso#1329 could:
`check_merged`'s opacity and arity checks have to see the spelling the program
used. The obvious inverse map from `aliases` is unsound -- it is keyed by name,
so it would also rewrite the diagnostic for a call the user really did write
qualified. The per-site record was then built as a probe and measured, and it is
affordable:

    alias-only reorder, no record   163,353,361
    with the per-site record        163,380,706   +27,345

1.7% of the prize, leaving -1,541,851 (-0.9349%) against 164,922,557. And the
27,345 is not the recording. THE PASS REWRITES NOTHING ON ANY MEASURED CORPUS: a
counter at the rewrite site reads 0 sites on bench/entry_corpus, 0 on
bench/compile_corpus and 0 on lib/json, against 1 on the `m/thing` fixture that
draws the opacity refusal. The vector never allocates, so what the 27,345 buys is
an extra parameter carried through a recursive walk over every expression in the
program, and a shape that hangs the recorder off a walker rather than threading
it should cost less. Two things a real implementation must handle that the probe
did not: the reader half in check.rs, and the second caller of the same walker at
src/lib.rs:2616, which walks with the door map.

**And the two readers want different things, which reading `foreign_constructions`
settles.** Its own comment states the invariant the reorder breaks, at
check.rs:1847: "A qualified name can never be a local binding, so unlike the
arity walk beside it this needs no shadowing set: the slash IS the foreignness."
That holds only while every slash in the merged program was written by a person.
After the alias pass has run a slash also means the pass put one there, and the
check fires on `m/thing 0` -- a call of an imported function -- as though it were
a construction of the imported type of the same name. So opacity does not want a
spelling to quote. It wants to SKIP a head the pass rewrote, because that head
was never a construction. Arity is the one that wants the spelling. One record,
two uses, and a fix that handed both readers the old name would leave the opacity
refusal exactly where it is.

This is not a gavel: the
substance was ruled in kanso#1120, a diagnostic names what the import writes, and
which mechanism satisfies it is the implementer's.

**How this came to be built twice, since the answer is a process one.** The task
list carried it as BUILT AND PROVEN with a full `cargo test` behind it. The
kanso#1329 entry names that exact evidence as worthless here --
`scripts/module_differential` is a kanso program run by the diagnostics-
differential CI job and `cargo test` never invokes it -- so the suite was green
both times and said nothing both times. The filing search caught it before the
branch was pushed, which is what the search is for.

- **DECLINED** — the entry path's alias-pass reorder, in any shape that leaves a
  check reading a rewritten name. Re-measured, re-refused, and this time the
  opacity refusal is on the record beside the arity one.
- **OPEN** — the pre-canonical spelling for `check_merged`'s two name-reading
  checks. Worth -1,569,196 on the entry row in the alias-only shape, and
  -1,541,851 with the per-site record that makes it sound. What is unbuilt is
  the reader half: the two checks in check.rs that have to consult the record
  instead of the node, and a fixture for each.
- **OPEN, unchanged** — the twins inside `infer`, which is the other half of the
  reorder's value and is blocked on a different thing: `infer` indexes
  declarations positionally, and a group keyed by (name, arity) is a dispatch
  group, so the twin is what lets a bare name resolve.

## 2026-09-08 — the welfare column spans four measurement epochs and the rewrite scores them on one ruler

Clay, reading kanso-lang.dev/numbers after kanso#1331 landed: "I still have
no clear accounting of why the welfare went down and the website still does
not look great." The account, read off the rewritten column on
origin/perf-history:

    2026-09-06 11:26   91.57 -> 58.96   -32.62   #1284  one consolidated run program
    2026-09-08 01:53   70.03 -> 67.75    -2.27   #1321  the compile corpus is named
    2026-09-08 10:17   67.91 -> 66.29    -1.63   #1331  the compile term sums both compiles

None of the three is the compiler getting worse. Each is a change in what is
measured, and the column still steps at each one because
`scripts/welfare_rescore` scores every row against the single baseline the
floor file holds today.

**The mechanism.** The rewrite exists so the column is "rewritten under one
formula whenever the formula moves, which is what makes two points on it
comparable" (docs/numbers.html). One formula does make rows comparable when
the WEIGHTS move. It does not when the MEASUREMENT moves, because the counters
change magnitude while the baseline does not. Today's compile baseline is
671,773,822, and the rows it divides come from four epochs:

    epoch                           compile_instructions   ratio    term
    lib/json, one compile                 19,316,962       34.78   0.9858
    compile_corpus, one compile           52,603,220       12.77   0.9623
    compile_corpus, module row            48,757,859       13.78   0.9650
    compile_corpus, both summed          212,644,590        3.16   0.8634

Adjacent epochs differ by a factor that is the workload and never the
compiler, and the term falls across each boundary by that factor. The -2.27
and the -1.63 are those two falls, weighted. #1331 re-based the floor so the
row it wrote is right; it could not re-base the rows before it, because the
rewrite has no notion of an epoch to re-base them to.

**The run side is the same disease and the larger cliff.** #1284 re-based the
run counters to parity at the changeover, so every row before it scores its
run terms against a baseline it never carried. Clay ruled the repair on
2026-09-07: share-weighted phases, renormalised over the phases a row carries,
based at row 70 (2026-08-10), and the ruling closes with "the rewrite is
cloud's." As of this entry, `scripts/` holds no share-weighted reconstruction.
The -32.62 on the chart is that ruling unbuilt.

**What makes the column flat across a change of measurement.** The rewrite
needs an epoch table: for each change of measurement, the head it happened at
and the per-row factor measured there. The corpus move's factors are already
recorded in the floor file (2.7232, 2.7207, 2.1047) and the summing's is
computed in kanso#1331. A row is then scored against the baseline scaled to its
own epoch, which is one re-basing per epoch, applied in the rewrite rather than
only in the floor file. It is what the three floor-file precedents did by hand
for the current row, done for every row.

**OPEN, cloud's, two items.** Build the 2026-09-07 ruling for the run side.
Apply the same per-epoch scaling to the compile side's two changes of
measurement. Until both land, the chart shows the history of what was measured
rather than the history of the compiler, and no reader can tell which.

## 2026-09-08 (sixth) — the pre-canonical spelling, and the entry reorder ships

Searched the log, the archive and design/ before filing: the entry above
("the entry reorder re-derived") is this thread's own, and leaves exactly this
as OPEN with the recorder measured and the reader half unbuilt. kanso#1329
records the first revert, kanso#1328 the module path's reorder, kanso#1120 the
ruling both readers have to satisfy. This builds what that OPEN item names.

**The reorder ships, with the two readers that make it honest.**
`canonicalize_bare_aliases` runs in front of `check_merged` on the entry path
now, so the whole-program check no longer walks the synthetic twins the pass is
about to delete. `canonicalize_types` stays in the success arm, which is the
cheaper of the two shapes by 8,930 instructions.

The pass returns a `Rewrites` — line and column to the bare name it replaced —
and `check_merged_after_aliases` hands it to the two checks that read a call's
name. Every other caller runs the pass after the check and passes an empty
record, where both readers behave as they always did.

**The two readers want different things, and that is the whole finding.**

    check.rs arity (two sites)   quotes the recorded bare name
    foreign_constructions        SKIPS a head the pass rewrote

Arity is a wording question and kanso#1120 settles it: the diagnostic names what
the import writes. Opacity is not. Its own comment states the invariant, at
check.rs:1847 — "A qualified name can never be a local binding, so unlike the
arity walk beside it this needs no shadowing set: the slash IS the foreignness."
That holds only while every slash was written by a person. After the pass, a
slash also means the pass put one there, and the check fires on a call of an
imported function as though it were a construction of the imported type of the
same name. No wording of that message is right; the site is not a construction
at all.

**On scripts/module_differential: 0 wrong, from the 2 wrong the reorder cost
before.** Both objections are gone, and both readers were watched red on their
own:

    opacity skip disabled   1 wrong -- `m/thing` is foreign, on a program that compiles
    arity spelling disabled 1 wrong -- quotes `m/one` where the source says `one`

Each mutation loses exactly its own fixture and no other, so neither reader is
dead code and neither is doing the other's work.

**What it costs, in this box.**

    entry_instructions   164,922,557 -> 163,499,802   -1,422,755  (-0.8627%)
    compile_instructions  49,170,337 ->  49,207,870      +37,533  (+0.0763%)
    summed                214,092,894 -> 212,707,672  -1,385,222  (-0.6470%)

The module row rises for the same reason kanso#1332's did: the path pays for
something it cannot use. Its record is always empty, and what it pays is a
parameter carried through `arity_walk_expr` and `foreign_constructions`'s walk,
both recursive over every expression. Neither lookup runs on a clean module
compile -- the arity one sits inside the refusal branch and the opacity one
behind a name being in the foreign set -- so the cost is the threading, not the
reading. Under kanso#1331's summed compile term the trade is 38 to 1 in favour,
and the sum is what the objective reads.

Against the reorder measured WITHOUT the readers (163,353,361), the readers cost
the entry row 146,320. The probe in the entry above put the recorder alone at
27,345; the rest is the two further walkers now carrying the same parameter.

These are container numbers and none of them is a row. CI counts both compile
veins, and this branch expects a deliberate red first round for exactly that.
**CI's rows, and what the container got wrong about them.**

    entry_instructions   163,612,976 -> 162,170,772   -1,442,204  (-0.8814%)
    compile_instructions  48,761,165 ->  48,791,172      +30,007  (+0.0615%)
    summed               212,374,141 -> 210,961,944   -1,412,197  (-0.6650%)

Welfare 66.2898 -> 66.30, ratcheted in the same change.

The container projected -1,422,755 and +37,533. Sign and order right on both,
digits wrong on both, and the two errors ran the same way: it UNDERSTATED the
entry fall by 19,449 and OVERSTATED the module rise by 7,526. Its standing
offset is +0.8% on the LEVEL of each row, so the naive expectation was that it
would overstate a fall; a level offset between toolchains does not carry to a
delta, and this pair is the demonstration. Under the summed term the trade is
48 to 1 in favour, against the 38 to 1 the container projected.


**The module path had both defects live, and nothing in the tree asked it.**
kanso#1328 moved the same pass in front of the same check on the module path
three days before this, and handed the check nothing. So on main today:

    kanso check <a module>   opacity REFUSES a program that compiles
    kanso check <a module>   arity quotes `m/one` where the source says `one`

Reduced, that is a module whose sibling declares `pub type thing` beside
`pub fn thing _`, importing it and calling `thing 0`. The entry-path form of
exactly that program is c7 in scripts/module_differential, and it was watched
through both reverts of the entry reorder; the module form had no case at all,
so the sweep read 0 wrong on a defect it could not see. The fix is the entry
path's, and both programs go into the sweep as c26 and c27 -- watched red on the
pre-fix compiler for the two messages above, verbatim, before they went green.

Threading the module path costs almost nothing because it was already paying:
`check_merged` built an empty `Rewrites` on every call, and the change replaces
that construction with the real one. Entry +121, compile +63 against the
readings in the table above, both already folded in.

**And the third path is now watched before it moves.** `kanso check` on a single
library file takes `compile_library`, which still checks before it canonicalizes
-- so both readers are right there today. c28 and c29 say so, and they were
watched red by making exactly the reorder the OPEN item below proposes for that
path: both go wrong together, with the same two messages. `compile_one` carries
a byte-identical block, so the mutation is one edit applied twice and the two
paths answer as one.

**What this says about where a defect gets found.** The reorder was reverted
twice on the entry path for objections the sweep caught within a round, because
the entry path had cases. The same reorder shipped on the module path and its
two objections sat for three days. The corpus decides what a sweep can see, and
a path with no case in it reads clean whatever it does.

- **DONE** — the OPEN item the entry above filed. The reorder, the record, both
  readers, both mutations, and the differential back to 0 wrong.
- **OPEN, and now priced** — src/lib.rs:348 and :425 still check before
  canonicalizing. compile_one is reached only from `compile_repl`
  (src/repl.rs:290) and compile_library only from `kanso check <a library
  file>`. Both merge `dep_program`, so both see the twins, and both would break
  the way the entry path did -- they were never blocked on a measurement, they
  were blocked on this.

  Measured on this box, on `kanso check bench/compile_corpus/compile_corpus.kso`
  with the reorder and the record applied to both sites:

      library_instructions   50,244,948 -> 48,681,802   -1,563,146  (-3.111%)

  Larger in proportion than the entry path's -0.8627%, on a path no vein
  watches. The differential stays 29 cases 0 wrong through it, which is what
  says the record makes the reorder correct there and not merely cheaper; c28
  and c29 go red on the same edit with the record left out. The baseline
  reproduced to the instruction on a second run. What is owed before it ships
  is the vein, since a fall nothing counts is a fall nothing keeps.
- **OPEN, unchanged** — the twins inside `infer`, the other half of the
  reorder's value. `infer` indexes declarations positionally and a group keyed
  by (name, arity) is a dispatch group, so the twin is what lets a bare name
  resolve.

---

## 2026-09-08 (seventh) — the third compile path gets a row

`kanso check` routes a single file by its content and the three routes are
three different compiles. A directory is a module and takes
`compile_module_inner`, which is what `bench/compile_corpus` and
`compile_instructions` watch. A file with bare statements is an entry and takes
`compile_parsed_entry`, which `bench/entry_corpus` and `entry_instructions`
have watched since kanso#1330. A file of definitions alone is a library and
takes `compile_library` — and nothing in the tree counted it.

That path is not a corner. `kanso test` takes it on every run, and so does
`kanso check` on any single file that is not an entry, which is most files
here.

`bench/library_corpus/library_corpus.kso` names ten imports and uses each, the
shape `bench/entry_corpus` has and for the same reason: `compile_library`
merges the dependency program and runs its own whole-program check over
everything the imports bring, so a corpus with one small import would measure
mostly the work underneath it. The directory is named to the same length as
`compile_corpus`, because the count tracks the length of the path the compiler
is handed at about 160 instructions a character.

The container projected 165,589,540 and CI wrote the row: **164,253,088**, on
binary sha 9bc8f829af68 in the job that also counted
compile_instructions=48,791,172 and entry_instructions=162,170,772, so all
three answer for one build. The projection is 1,336,452 high, +0.8136%, which
lands on the offset the other two rows already carry between this box's rustc
1.94.1 and CI's 1.98.1. Only CI may write the row, and the reason is that
offset.

CI's summary named exactly one failing vein and eighteen green, which is what
round one was for.

**A CORRECTION, made the round after the claim.** This entry and round two's
commit message both said the ratchet job proved `library_ir` on the runner.
Read the job log: it did not, and could not have. The ratchet's second pass is
`ratchet -- touched origin/main`, which selects only rows patching a file the
branch changed, and it reported

    ratchet: 1 rows patch a file this branch changed
      the ratchet (every gate has a mutation that turns it red)

-- one row, the ratchet's own. This branch touches ci.yml, CLAUDE.md, three
gate scripts, the ratchet, the trend gate, a spec, the log and two new bench
files, and no `src/`; `the_library_program_is_checked_twice.sh` patches
`src/lib.rs`, so the touched guard correctly passed it over. What CI did run is
the first pass, `every mutation still matches the source it patches`, which
does read the new mutation's anchor against `src/lib.rs` and found it. So the
anchor holds on the runner and the row's provability there is untested; it was
proved in the container, 165,589,540 -> 190,698,277. The next change to
`src/lib.rs` -- the reorder -- is the branch that will select this row and
prove it on CI.

**THE SPEC PREDICTED ITS OWN FAILURE MODE AND THIS IS THE INSTANCE.**
`tests/the_compile_sweep_names_every_compile_gate.rs` derives the sweep's list
from goldens matching `bench/compile_*` and `bench/entry_*`, and its own doc
comment says: *a prefix list is exactly the shape that goes stale when a vein
is added under a new name.* `bench/library_instructions_golden.txt` matches
neither prefix, so both derivations in that file walked straight past it and
the sweep would have looked like coverage while missing the newest vein. Both
are widened here, in the commit that adds the vein.

The trend gate's own coverage spec did NOT have that hole:
`tests/every_counter_golden_is_walked_by_the_trend_gate.rs` reads `bench/` off
disk and keys on `contains("golden")`, so it went red the moment the file
existed and named what was missing. Two coverage specs over the same tree, one
keyed on a prefix and one on a substring, and only the substring one survived a
new name.

A third coverage spec found the other half of the same gap. The ratchet keeps a
`host_bound` list of the gates that count under callgrind, so that a runner the
golden does not name is reported as unproven rather than credited as a
regression, and `tests/a_host_bound_gate_is_reported_not_credited.rs` derives
that list from the gates that actually run the tool. It went red naming `sh
scripts/gates/library_instructions.sh` as soon as the gate existed. Three specs
over one tree: the substring-keyed pair spoke, the prefix-keyed one did not.

The mutation is `the_library_program_is_checked_twice.sh`, the library twin of
the entry one, and its anchor takes two steps rather than one:
`let merged_diags = check::check_merged(&program, false);` appears twice in
`src/lib.rs` because `compile_one` carries a byte-identical block, so the
function is found by its signature and the duplicate goes in at the first such
call after it. What it proves is the argument for the row: the same edit leaves
`compile_instructions` and `entry_instructions` green. It rides in the ratchet
as `library_ir`, beside `compile_ir` and `entry_ir`.

Proved rather than assumed, in the order the rule asks for: it applies (two
calls become three, and the third is inside `compile_library` at 432 with
`compile_one`'s at 355 untouched), it compiles, and the row it moves goes
165,589,540 -> 190,698,277, a rise of 25,108,737 or 15.16%, against a gate that
asserts equality. Counted here with the host check bypassed on purpose, because
this container may not compare the row and the question was whether the
mutation moves it rather than what the value is. Restored, rebuilt, clean.

Still open, unchanged by this: `src/lib.rs`'s two remaining callers check
before they canonicalize. That reorder is measured — 50,244,948 -> 48,681,802,
**−1,563,146 / −3.111%** — and was blocked on this vein. It is not in this
commit, so the row this one opens is the pre-reorder baseline and the next PR
is what spends it.

---

## 2026-09-08 (eighth) — the last two callers check before they canonicalize

kanso#1328 put `canonicalize_bare_aliases` in front of the whole-program check
on the module path and kanso#1335 did it on the entry path. `src/lib.rs` kept
two callers running the old order: `compile_one`, reached only from
`compile_repl`, and `compile_library`, reached from `kanso check` on a file of
definitions. Both merge `dep_program`, so both see the twins, and both would
break the way the entry path did. They were never blocked on a measurement.
They were blocked on a vein, and the vein opened yesterday.

Measured on the library corpus, both readings on this container, the recipe
`scripts/gates/library_instructions.sh` uses minus the host check:

    library_instructions   165,589,540 -> 164,300,594   -1,288,946   -0.7784%

Read twice, on two builds whose shas differ, and identical to the instruction.

**The recorded −3.111% is a different workload and does not belong to this
row.** The 2026-09-08 (sixth) entry measured `kanso check
bench/compile_corpus/compile_corpus.kso` treated as a library file, 50,244,948
-> 48,681,802, and that corpus is a third the size of `bench/library_corpus`,
so the same absolute saving reads as four times the proportion there. The row
this change is watched by falls 0.7784%.

**The record was watched red before it was watched green.** The reorder without
it takes `scripts/module_differential` from 29 modules 0 wrong to 2:

    a bare call to a shared name, from a library file
      expected it to compile:
      error[opacity]: `m/thing` is foreign -- only `m` builds a `thing`;
      ask it for one through a pub function

    a call from a library file at the wrong arity
      refused, but not with
      'error[arity]: no 2-argument arm of `one` (arms take 1)':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

The first REFUSES A PROGRAM THAT COMPILES; the second is the spelling
kanso#1120 settled. Both are the library-path twins of what the entry path had,
which is why the record ships with the reorder rather than after it. With
`check_merged_after_aliases(&program, false, &rewritten)` at both sites the same
run is 29 modules, 0 wrong.

**THE ROW COULD NOT BE SELECTED AT ALL, and this branch is what found out.**
kanso#1337 said its mutation was verified only as an anchor that still matches,
never run, because the touched pass selects rows patching a file the branch
changed and that branch touched no `src/`. This one rewrites `src/lib.rs`, so
the row should have been selected. It was not. CI's touched pass named three
rows and neither `library_ir` nor its entry twin was among them:

    ratchet: 3 rows patch a file this branch changed
      cost goldens — a front-end pass that owns the program's names instead of
        borrowing them
      welfare — a number the page states about the present drifting from its
        golden
      diagnostics differential — a name a module keeps private crossing an
        import anyway

THE SELECTION KEYS ON GUARD LINES, NOT ON THE PATCHED FILE. `read_it` collects
the lines of a mutation that spell `grep -q` and asks whether any of them names
a file the branch changed. That is deliberate and kanso#1254 argued it well: a
guard IS the dependency, and reading the whole script instead selected 33 rows
where 12 were at risk. But both compile-path mutations reach `src/lib.rs`
through `"$file"` and assert with `grep -c`, so the path appears on no guard
line and neither row could ever be selected, whatever a branch touched.

`entry_instructions` has been in that position since kanso#1330 and
`library_instructions` since kanso#1337 — the same pair the trend gate had left
unclassified, found the same afternoon by two different means.

Both mutations now carry a guard line that spells the path, and the selection
goes from three rows to five with the entry and library rows both named. Watched
blind first: the three-row listing above is this branch before the repair.

**EIGHT MORE ARE IN THE SAME POSITION**, and the count is off disk rather than
from memory: of 112 mutations, 77 name a path under `src/` and 69 of those name
one on a `grep -q` line. The two repaired here were among the ten that did not.
The eight left are

    a_module_rewritten_twice_before_it_is_checked.sh
    a_path_copied_once_per_declaration.sh
    a_validator_that_skips_its_tail.sh
    a_validator_the_reference_disagrees_with.sh
    an_entry_program_rewritten_twice_before_it_is_checked.sh
    clippy_bait.sh
    misformatted_source.sh
    the_wasm_engine_answers_something_else.sh

and each is unselectable for the same reason: it reaches its file through a
variable and asserts with `grep -c`, `grep -cF` or `grep -A1`. Repairing them is
its own change, and it owes a spec that reads the mutations off disk — which is
why one is not shipped here. Written and watched red, it named all ten; with a
green list of eight it would only be a list, and a list is the shape that goes
stale.

`canonicalize_types` stays where it is. On the entry path moving it too read
8,930 instructions worse, because the alias pass deletes the twins before that
one would have walked them, and there is no reason to expect the other
direction here.

**CI's row.** The vein summary named exactly one failing counter against
eighteen green, which is what round one was for:

    library_instructions   164,253,088 -> 162,970,167   -1,282,921   -0.7811%

The container projected a fall of 1,288,946, or 0.7784%. It overstated the
saving by 6,025 instructions — half a per cent of the fall — while reading
0.8136% high on the LEVEL of the row. A level offset between toolchains does
not carry to a delta, and this is the second sitting to say so: kanso#1335
recorded the same thing on the entry and module rows, where the projections
also came out conservative.

**THE OTHER TWO COMPILE ROWS DID NOT MOVE AT ALL**, and that is worth writing
down because this change edits `src/lib.rs`, the compiler's own Rust.
`compile_instructions=48,791,172` and `entry_instructions=162,170,772` are
byte-identical to their goldens in the job that counted the new row. CLAUDE.md's
prior is that `compile_instructions` usually moves on such an edit through
layout alone; it holds, and this is the second recorded change small enough to
leave it alone, after kanso#1285.

**The mutation's anchor went stale in this same change, and pass one of the
ratchet is what said so.** `the_library_program_is_checked_twice.sh` anchors on
`let merged_diags = check::check_merged(&program, false);`, which is the exact
line the reorder rewrites, at both sites. So round one's ratchet job failed
`every mutation still matches the source it patches` and skipped the touched
pass underneath it — the pass this branch exists to reach. The anchor moves to
`check::check_merged_after_aliases(&program, false, &rewritten)` and the
duplicate lands at 465, inside `compile_library` (390), with `compile_one`'s
call at 372 untouched.

A mutation anchored on a line a change is about to rewrite goes stale in that
change. That is the ordinary case rather than a surprise, and the first pass
exists to report it on the runner before the row it guards is ever read.

**A FIFTH READER, found by running the trend gate on this change's own diff.**
The fall printed as UNCLASSIFIED:

    UNCLASSIFIED — no direction table names these, so they count
    toward neither side of the pure-regression rule:
      library_instructions

The gate walks the golden and has no direction for the counter in it, so it can
report the move and cannot say which way is better. A rise of the same size
would have printed identically and counted toward neither side of the
pure-regression rule. `entry_instructions` has been in the same position since
kanso#1330 — two compile veins unclassified for as long as they have existed.

This is the digestbench failure one vein down, which
`tests/every_benchmark_in_the_work_vein_has_a_direction.rs` exists to prevent
and could not see here: it reads `bench/instructions_golden.txt` and stops. Its
own argument carries straight across — each compile golden holds retired
instructions for one compile and fewer is better in all of them, so every row
has a direction and none of them is a presence counter whose direction means
nothing alone.

`tests/every_compile_vein_row_has_a_direction.rs` asserts it over every
`bench/*instructions_golden.txt` other than the work vein's, read off disk, so a
fourth compile path opening a vein under a fourth name is covered on the day the
file lands. Watched red first, naming both: `["entry_instructions",
"library_instructions"]`. Both join `lower_gg` and the gate now prints
`improved: library_instructions 164,253,088 -> 162,970,167`.

That makes five readers a compile vein owes: its gate, `all_compile.sh`'s
`gates=` line, the trend gate's walk, `golden_prose`, and the trend gate's
direction table. kanso#1337 wired three of them.

---

## 2026-09-08 (ninth) — the twins inside infer, sized at last

The log has carried this OPEN item since 2026-09-08 (fifth) and twice since:
*the twins inside `infer`, the other half of the reorder's value.* It has never
had a number. It has one now, and the number is small.

`enroll_bare` (src/lib.rs) clones every exported declaration of every imported
module under its short name, and those clones are real declarations that infer
and check both walk. On `bench/library_corpus` the top-level enrollment makes
**292 function twins and 45 type twins against a merged program of 1,437
functions** — one declaration in five. infer is 22.5% of that compile.

**THE OBVIOUS INSTRUMENT DOES NOT WORK, and the way it fails is the blocker
arriving as a diagnostic.** An env-gated early return in `enroll_bare`, so the
twins are never made, stops the compile at

    error[name]: `first` is already a declaration; rename the binding
      --> std/regexp/regexp.kso:654:3

Inside the standard library, not at the corpus's call sites -- `library_corpus`
writes every call qualified and needs no bare twin of its own. The bare space is
load-bearing inside std. That is the positional-index blocker already recorded
for this thread: `group_members`, `ctx.current_index` and the reader bitmap are
all indexed by position in `program.fns`, and beat.rs, check.rs and codegen.rs
all read `inference.returns[i]` by that same position.

**The instrument that does work: skip only the BODY WALK.** In infer's fixpoint
sweep, beside the dirty test, skip a declaration whose `synthetic` is set. Every
twin stays in `program.fns`, so no index moves. All readings on one probe
binary, so the added branch cancels:

    corpus                walked        skipped        delta
    library_corpus   167,284,685   166,685,167     -599,518   -0.3584%
    entry_corpus     164,832,926   164,512,869     -320,057   -0.1942%
    compile_corpus    49,403,663    49,412,576       +8,913   +0.0180%

infer itself: 38,972,582 -> 38,342,754, -1.6161%.

**One declaration in five is one instruction in sixty-two.** A count-based
estimate assuming infer is linear in the declaration count says 20.3% of infer,
4.57% of the row; the measurement refutes that by 12.6x. The reason is the
fixpoint: only DIRTY declarations are revisited, and a twin nothing calls goes
clean after its first visit and is never re-dirtied again.

**The module row goes the wrong way, and that is a finding about the probe.**
Skipping the walk leaves `returns[twin]` at its default instead of the answer
the walk would have written, so other declarations infer different values and
the dirty sets and round count move with them. The delta is a mix of walks not
taken and a fixpoint doing different work, and on `compile_corpus` the second
term wins. So these three numbers size the thread; they are not a ceiling in the
strict sense.

**Where that leaves it.** The shipping shape has to copy the original's answer
into `returns[twin]` rather than leave it at the default, and that copy costs
something none of this pays. So the honest reading is a couple of tenths of a
percent before the copy, against the alias-pass reorder's 0.7784% on the same
row in the same session. The thread goes to the back of the queue: not refuted,
not worth building next, and no longer unpriced. What is owed first if it is
ever picked up is recording the twin/original pairing at the `enroll_bare` clone
site, which is the cheap half of the copy.

## 2026-09-08 (tenth) — a host-bound row was dropped on every host, including the one that could prove it

`type bound` in scripts/ratchet/ratchet.kso has said since kanso#1228 that a row
sharing a host-bound gate is skipped where the runner cannot answer it and
**"on a run that lands on the golden's silicon the row is proved normally"**. The
code has never done the second half. `kept_provable` dropped every row whose
gate is on the `host_bound` list, and that list is a property of the GATE — its
golden pins exact counts and the runner pool is not one machine — which says
nothing about whether THIS run landed on the golden's silicon.

So `library_ir`, `entry_ir`, `compile_ir` and `work` could be selected and never
proved, anywhere. Not on a container, where the gate refuses and the drop is
right. Not on CI, where the gate compares and the drop is wrong.

kanso#1338's ratchet job is the instance, and it is the first branch that could
ever have produced one — the guard-line repair it carried is what made the two
compile rows selectable at all:

    ratchet: 5 gates green before any mutation
    ratchet: 5 rows patch a file this branch changed
      ... a front-end pass that owns the program's names instead of borrowing them
      ... work on the entry path, where the compile row measures a module
      ... work on the library path, which the entry and module rows both walk past
      ... a number the page states about the present drifting from its golden
      ... a name a module keeps private crossing an import anyway
    ratchet: 3 rows

Five gates green. The baseline had just run `entry_instructions.sh` and
`library_instructions.sh` on that runner and both compared. Then three rows ran,
and the two compile rows left the report with no line at all — not a finding, not
a skip notice, nothing. The same job's cost-goldens run read `entry instructions`
and `library instructions` green on their own steps, which is the same fact from
the other side.

THE BASELINE ALREADY ASKS THE RIGHT QUESTION and its answer was being thrown
away. It runs every distinct gate on an unmutated worktree before any mutation,
and pushes a finding for each one it could not answer. A host-bound gate with a
finding is `UNPROVEN THIS RUN` and its row must be dropped; a host-bound gate
with no finding was green on this machine, minutes ago, and a red under mutation
in a sibling worktree on the same machine is the mutation's doing.

`finding` gains the gate it is about, and `kept_provable` reads it back. Four
lines of decision where there was one, and the static list keeps its job: it
still decides whether a red baseline FAILS the run or is excused.

Watched red, with the whole program rather than a piece of it. The spec copies
scripts/ratchet to a temp directory with one constant changed — `work_gate`
points at `scripts/gates/python_free.sh` instead of `instructions.sh` — which
makes a two-git-grep gate host-bound, the property four callgrind gates otherwise
have and only a runner can exercise. Then it runs the real `prove python-free`
against a worktree of HEAD. On the old rule:

    ratchet: 1 gates green before any mutation
    ratchet: no row on this runner could be proved; none was claimed
    ratchet: 0 rows

on the new one:

    ratchet: 1 gates green before any mutation
    ratchet: 1 rows
      red   python-free (the harnesses stay kanso) — a python call creeping back into a harness
    ratchet: every row turned its gate red

Ten seconds for the file's four fixtures. It lives beside
`a_gate_red_before_the_mutation_is_refused_rather_than_credited`, whose baseline
pass this reads, and shares that file's mutex because `prove` names its scratch
worktrees by fixed paths. The other three fixtures stayed green throughout,
which is what says the baseline pass itself did not move.

What this does not do is prove a compile row on a runner. That is CI's to say,
and the reading is in the next ratchet job on a branch touching src/: five rows
selected should now be five rows proved.

## 2026-09-08 (eleventh) — the per-module check, built and refuted as specified

§59 prices the per-module whole-program check at 20.38% gross and reasons down
to "about 8.5 million instructions, roughly 16.5% of the row" for a version that
moves the check up and leaves six slash-guarded passes where they are. This
built that version. The ceiling is larger than the page says and the change is
not reachable, and both halves are worth writing down.

Repriced first, with an env-gated skip of `check_merged` for every module that
is not the root, both readings on one probe binary:

    corpus                        checked        skipped         delta
    compile_corpus (module)     49,209,611     40,189,876    -18.3292%
    library_corpus (library)   164,306,354    126,030,042    -23.2957%

The library row gains more because 67.8% of that compile is
`load_dependencies` and its corpus names ten imports where the module corpus
names four. The shape §59 describes is most of what the library path does.

**§59 NAMES ONE MECHANISM AND THERE ARE THREE.** The page's account is the
slash: six passes skip any declaration whose name carries one, so run at the
root they apply to the root and pass over every dependency. That is right and it
is not the whole list.

The second is a message that QUOTES A DECLARATION. With only the three
slash-guarded checks kept per module, `scripts/module_differential` reads 29
modules and 2 wrong:

    a call to a sibling at the wrong arity
      refused, but not with 'error[arity]: no 2-argument arm of `one`':
      error[arity]: no 2-argument arm of `m/one` (arms take 1)

    an arm no call can reach
      refused, but not with 'error[dispatch]: overlapping overloads of `twice`':
      error[dispatch]: overlapping overloads of `m/twice` are illegal

Neither `check_call_arities` nor `check_overlapping_arms` is slash-guarded. What
makes them per-module is that the message names a declaration and the merge has
already qualified it — the spelling kanso#1120 settled. Keeping those two per
module returns the sweep to 29 and 0.

The third is ATTRIBUTION, and it is the one that closes the route. With five
checks per module the suite is 122 binaries and four fail, five tests:

    a_library_at_fault_is_reported_through_the_program_that_imports_it
    error_corpus_reports_each_golden_diagnostic
    the_wasm_engine_agrees_with_the_golden_corpus
    the_wasm_engine_complains_the_way_the_others_do
    the_front_end_infers_the_whole_program_four_times

The first four are one defect. A diagnostic raised on a dependency's
declarations at the root loses the file, the span and the module suffix that the
dependency's own compile supplied:

    error[naming]: `silly` answers only true or false: name it `silly?`
      --> deep_library_error/main.kso:4:8

    error[naming]: `silly` answers only true or false: name it `silly?`
      (module deep_library_error/deep)

It points at `main.kso`, which does not contain the fault. Detection is
unaffected — the dependency's declarations are all in the merged program — so
what is lost is attribution. The fifth failure is the saving showing up: a spec
pins how many times the front end infers the whole program, and running the
check once at the root is what moves it.

So any check that can fire on a dependency's declarations stays per module
unless a root-raised diagnostic can name where the declaration came from, and
that is nearly all of them. `infer::infer` is 11.8M of `check_merged`'s 19.1M
and runs for any check that reads inference, so the saving leaves with them.
18.33% and 23.30% are a ceiling reachable only with provenance on merged
declarations, which is a larger piece of work than a two-way split and is what
this thread owes next.

kanso#1003 walked this route once and withdrew it as "the per-dependency
check_merged is not redundant" without naming what made it so. §59 named one
thing. There are three, and they are written down now. Nothing shipped: the
probe is reverted and the page is corrected to say what the tree says.

## 2026-09-09 — the touched pass could not see eight mutations, in three different ways

kanso#1338 repaired two mutations the ratchet's `touched` pass could never
select and counted eight more in the same position. Reading all eight says they
are not one kind, which is why a repair that treats them alike would have been
wrong.

The pass selects the rows a branch could have blinded by intersecting the files
the branch changed with the paths each mutation names on a guard line, and
`guarding` recognised one spelling of grep.

**THREE guarded correctly and were still invisible.**
`a_module_rewritten_twice_before_it_is_checked`, `a_path_copied_once_per_declaration`
and `an_entry_program_rewritten_twice_before_it_is_checked` all do this:

    n=$(grep -cF "$target" src/lib.rs)
    [ "$n" -eq 1 ] || { echo "moved or multiplied ($n)" >&2; exit 1; }

src/lib.rs is spelled right there, on a grep line. That guard is STRICTER than
the `grep -q` its neighbours use — it catches the anchor multiplying as well as
vanishing — and `guarding` keyed on the literal `grep -q`, so the three
mutations that guard best were the three it could not read.

Widening the predicate to `grep -q` or `grep -c` is measured rather than
assumed: over the 112 mutations on disk it takes the satisfying count from 69
to 72, and the three are exactly the three that were blind. Nothing else moves,
so it cannot over-select. That is the whole repair for this shape, and it is one
line rather than three edited scripts.

**THREE reach their file through a variable** — `grep -qF '<anchor>' "$f"` with
`f=src/runtime.c` or `f=src/wasm.rs`. A grep line with no path on it. Exactly the
shape kanso#1338 repaired, repaired the same way: one guard line spelling the
path.

**TWO carry no grep at all**, because they append rather than anchor:
`clippy_bait` and `misformatted_source` both `printf ... >> src/lib.rs`. They
cannot go stale the way an anchored mutation does, which is why they had no
guard — and they are NOT exempt. An `allow(clippy::ptr_arg)` anywhere in
src/lib.rs makes the bait inert with the gate green, and a crate-level rustfmt
escape does the same to the other. Both now assert the escape hatch is absent,
which is a guard worth having on its own and puts the path where the pass can
read it.

77 of 77 mutations naming a src/ file now name it on a guard line.

**The spec asserts the property, not the list.** kanso#1338 wrote the
list-shaped version, watched it red naming ten, and declined to ship it: with
the ten repaired it would have been a green list, and a list goes stale the next
time somebody adds a mutation.
`tests/every_mutation_names_its_file_on_a_guard.rs` reads the directory off
disk. Watched red on one of each repairable shape, naming both by file and by
path:

    a_validator_that_skips_its_tail.sh names ["src/runtime.c"] and guards none of them
    clippy_bait.sh names ["src/lib.rs"] and guards none of them

## 2026-09-09 (second) — the chart drew the counters the objective had stopped reading

docs/numbers.html is the long view: one row per merged commit, and a chart over
it whose own subject line reads "what a run costs, what compiling costs, and the
welfare score they roll up into". Five of its six lines drew counters the
objective had retired.

    line              read                                        status
    run instructions  instructions + encode_instructions          retired 09-06
    run memory        oneshot_arena_peak_bytes                    retired 09-06
    binary size       text_bytes                                  never a term
    compile work      compile_rounds+compile_visits+emitted_lines retired 09-03
    compile memory    compile_peak_bytes                          a term
    welfare           welfare                                     the score

The 2026-09-06 gavel made the objective's runtime one consolidated program, and
`run_instructions` and `run_peak_bytes` replaced the thirteen work rows and
twelve memory rows the run side used to weigh. The 2026-09-03 rebuild had
already retired fixpoint rounds, expression visits and emitted lines.
`scripts/perf_record` writes a row's objective counters straight out of
`welfare --counters`, unfiltered, so both counters the gavel minted have been in
every row written since that day. Neither had a line.

Read off the five hundred rows on the history branch:

    counter                     rows   from         distinct   span
    run_instructions              57   2026-09-06         18   3.04e9 -> 2.39e9
    run_peak_bytes                57   2026-09-06          5   156.8 -> 46.7 MB
    compile_instructions         105   2026-09-03         52
    compile_allocs               349   2026-08-09         45
    compile_peak_bytes           349   2026-08-09         22
    oneshot_arena_peak_bytes     349   2026-08-09          2   the drawn one

The last row is what a reader was looking at. `run memory` took two values, 2
MiB and 3 MiB, across every row that holds it, so that line was flat with one
step in it over the span where the counter the score reads fell 3.4x.

**THE CHART DRAWS THE OBJECTIVE'S FIVE TERMS NOW**, in the order
`bench/objective_sources.txt` lists them, plus welfare and plus binary size.
`.text` stays because the page has nowhere else to show it and the 2026-09-05
ruling is a fact worth publishing: no machine-code-size term in welfare, and
`.text` keeps its own exact vein. It is drawn grey and the legend says why.

`tests/the_chart_draws_the_objective.rs` replays `bench/objective_sources.txt`
against the TREND array in both directions, and was watched red on the old page
naming exactly the two failures:

    the objective reads ["compile_allocs", "compile_instructions",
      "run_instructions", "run_peak_bytes"] and the chart draws no line for them

    the chart draws ["compile_rounds", "compile_visits", "emitted_lines",
      "encode_instructions", "instructions", "oneshot_arena_peak_bytes"],
      which the objective does not read

Two keys are allowed past the second test by name, each with its reason written
beside it: `welfare` is the score rather than a counter it reads, and
`text_bytes` carries the 2026-09-05 ruling. A third arriving without a reason
turns the spec red.

THE SITE SMOKE FIXTURE HAD GONE STALE THE SAME WAY. Its stub history was six
hand-picked rows, and the keys picked were the ones the chart happened to read
in august: on 2026-09-09 it held neither `run_instructions` nor `run_peak_bytes`
and could not have caught a chart that drew neither. It is generated from the
newest real row now, every key of it, each value stepped so no series can draw
flat and pass for a drawn one. The count it asserts went from six series to
seven, and the protective property was watched: dropping `run_instructions` from
one stub row alone reads `[5,6,6,6,6,6,6]` and fails.

THE PAGE'S ROW ACCOUNTING WAS WRONG, and it is the same kind of error one layer
up. It said 349 of the five hundred rows are scored on the compile counters
alone and the other 151 have no score at all, which accounts for every row and
leaves out the 57 that are scored on all five. The real shape is 244 rows on two
counters at coverage 0.28, 48 on three at 0.44, 57 on all five at 1.00, and 151
with none. The paragraph is restated in dates rather than counts. Those counts
perish on the next merge — the file grows a row per commit, and "the five
hundred rows" was already a number waiting to go stale.

## 2026-09-09 (third) — the entry row and the library row are the same measurement

`bench/objective_sources.txt` said the instruction term "sums BOTH compile
paths: `kanso check <dir>` is a module and `kanso check <file>` is an entry".
`kanso check <file>` is TWO paths — it routes by content, and a file of
definitions alone is a library taking compile_library — so the sentence names
two of three and the term takes two of three, which is right for a different
reason than the one written down.

Whether the library row should join is answered by measurement rather than by
a gavel. Callgrind on this container, `kanso check` on each corpus:

    frame                     module        entry       library
    kanso::main           49,226,188  163,918,585  164,230,408
    load_dependencies     29,179,760  112,141,854  112,163,002
    compile_module_inner  41,345,039  100,795,947  100,804,487
    check_merged_after..  17,585,954   62,110,471   62,242,504
    infer::infer          10,931,355   36,679,783   36,496,937

bench/entry_corpus and bench/library_corpus name the IDENTICAL ten imports —
bits, io, json, list, math, path, regexp, render, sha256, text — and the two
totals sit 311,823 apart out of 164M, which is 0.19%. Their dependency loads
agree to 21,148, which is 0.019%. Every shared frame agrees to under half a
per cent; the paths diverge only in the roughly 21M above load_dependencies,
where compile_parsed_entry reads 142.58M against compile_library's 143.18M.

The term is already 66% dependency loading: 29.2M for the module corpus's four
imports and 112.1M for the entry corpus's ten, out of 213.1M. Admitting the
library row would count that same 112.1M a third time and weight loading three
to one against the compiler's own passes, for a dimension the entry row already
carries. The module row is redundant with neither, at four imports against ten
and 49.2M against 164M.

`bench/library_instructions_golden.txt` is unaffected and still fails CI when
it moves. A vein the objective does not weigh is still a vein — that is what
kanso#1337 opened it for.

**THE ATTRIBUTION PLUMBING §59 OWES WAS ALREADY PROTOTYPED, and the prototype
lives nowhere the tree can see.** kanso#1340 refuted moving check_merged to the
root because a root-raised diagnostic loses the file, the span and the
`(module …)` suffix, and named provenance on merged declarations as what the
thread owes next. That session built the plumbing and left it in a scratch
directory: a `diag::HasFile` trait implemented for `ast::FnDecl` that returns
`&self.file`, and a `diag::attributing(&program.fns)` wrapper standing in for
`&program.fns` at each per-declaration check, so a diagnostic raised inside the
loop carries the declaration's own file. Both patches are stale against main —
they fail at src/check.rs:724 and src/lib.rs:3578, where kanso#1338's reorder
moved under them — so what survives is the shape, which is written down here
because a container is not a record.

Four facts make that shape the right one, and they were checked rather than
assumed. `FnDecl` already carries `file: Arc<str>` and `span`, so the
provenance is in the tree. The merge does not re-stamp it: `stamp_file` runs
per parse, per file, and the merges are plain `extend`, so a merged
dependency's declaration keeps its own file. The file cannot ride on `Span`,
because kanso#1135 made a span two u32 for a 7.1% peak win and an `Arc<str>`
there hands it back; `Diagnostic` is the place, since diagnostics are built
only on errors. And the `(module …)` suffix needs no new state at all —
`split_qual` on the merged declaration's qualified name recovers it.

The scope is 23 checks in `check_merged_after_aliases` and 65 `Diagnostic::new`
sites in check.rs. The prize is the ceiling kanso#1340 repriced: −18.33% on the
module row and −23.30% on the library row.

**A CORRECTION, WRITTEN THE SAME NIGHT.** The paragraph above says the patches
fail "at src/check.rs:724 and src/lib.rs:3578". That conflates two of them:
`419_attribution.patch` fails only at check.rs:724, in four hunks that are all
the same substitution, and the lib.rs failure belongs to the wider
`419_all_six.patch`. Rebasing the first is mechanical — four lines — and it
builds.

Building it says the shape is further from shipping than the paragraph above
implies, in two specific ways, and both were found by running the suite rather
than by reading.

THE PROTOTYPE IS NOT INERT. The obvious shipping order — land the attribution
first, doing nothing until `check_merged` moves to the root, then move it —
does not work: `cargo test --release --test golden` goes red on
`error_corpus_reports_each_golden_diagnostic`.

    fixture: tests/golden/errors/a_reexport_of_a_name_nothing_offers.kso
    got:  error[name]: no import offers a pub `nonexistent` to re-export
            --> std/text/text.kso:3:5
    want: error[name]: no import offers a pub `nonexistent` to re-export
            --> a_reexport_of_a_name_nothing_offers.kso:3:5
             3 | pub nonexistent
                       ^

THE ATTRIBUTION IS DYNAMICALLY SCOPED, so it names whoever is iterating rather
than what the diagnostic is about. That refusal is raised at src/lib.rs:3348,
about a re-export in the user's own file, and it came out attributed to
std/text. The `Attributed` iterator holds its last item's guard until the
iterator itself drops, so a walk whose iterator outlives the raise site leaks
its attribution forward. A thread-local read by a constructor cannot tell the
declaration in hand from the declaration some other loop last held.

AND `render_across` IS NEVER CALLED. The lib.rs half of the patch is a single
`Diagnostic::new` conversion; no call site passes it the sources map. Every
cross-file attribution therefore renders against an empty map, which is the
second half of that fixture's diff — the quoted source line is gone.

So the next step is not a rebase and a measurement. It is: attribute at the
raise site rather than through a thread-local, and wire `render_across` at the
`compile_*` call sites. Then the reorder is measurable. kanso#1340 called this
"a larger piece of work than a two-way split" and was right; this is what it
consists of.

## 2026-09-09 — the chart's palette was picked for one of the two pages it is drawn on

**DONE.** kanso#1343 fixed *which* counters the trend chart draws and left
*how* it draws them alone. Clay, on the result: "both blue lines have gone up."
The rise was real and already answered — two corpus re-basings, kanso#1321 and
kanso#1331, with the baseline moving under them so the floor went 59.74 →
66.3024 — but the reading was harder than it needed to be, because three of the
seven lines were in the blue band. This is the colour half.

Colour on a categorical chart is computable, so it was computed. Run against
the two surfaces the page actually paints (`--bg`, light `#fcfbf7` and dark
`#0c0c0f`), the shipped palette failed four checks:

    lightness band      all seven outside it
    chroma floor        #94a3b8 at 0.035 — reads as grey, so as gridline
    CVD separation      #38bdf8 <-> #a78bfa, 5.2 deutan against a floor of 8
    normal-vision floor #fb923c <-> #facc15, 14.6 against a floor of 15

And the finding nobody had looked for: **one palette was shipped for two
surfaces, and it had been chosen for the dark one.** On the light page every
one of the seven lines sat under 3:1 — yellow at 1.48:1. The blue-band
collision was the visible half of that; the light page was worse and unremarked.
The grey was mine, from #1343, and it was the chroma failure.

Each mode now takes its own steps, validated against its own surface. The
ordering was searched rather than chosen: of the 5,040 orderings of seven hues,
536 clear the adjacent-pair gates in both modes, and 216 of those also clear
them among the three compile lines taken all-pairs — which is the comparison a
reader of this chart makes, and which the adjacent pairlist does not cover. The
one kept scores 9.2 worst-case either way (OKLab ΔE ×100, floor 8) with a
within-group normal-vision margin of 24.6 against a floor of 15.

Two orderings show why the second gate was worth adding. `aqua blue orange
violet yellow magenta green` passes the adjacent gate identically — 16.3 light,
13.0 dark — and puts orange beside yellow *inside* the compile group, where
they separate by 4.8 under deuteranopia and 10.6 under normal vision. The
adjacent check cannot see it, because those two lines are not adjacent. Every
ordering that moves yellow onto binary size, which is where the weakest-contrast
hue belongs, drops the adjacent margin to 6.9 — the warn band. That refinement
was rejected: it costs more than it buys.

Binary size draws dashed instead of grey. The dash says what the grey was
trying to say — this is the one line the score does not read — without failing
the chroma floor. The legend key is drawn as the mark, dash included.

Three light-mode steps still sit under 3:1 (#1baf7a 2.72, #eda100 2.09,
#e87ba4 2.60). That is a documented relief rather than a pass, and the relief
has to be real: a faint line is readable only if its number is written down
somewhere. It was not. **The panel under the chart carried no row for any of
the seven series** — it listed `compile_rounds`, `compile_visits` and
`emitted_lines`, three counters the 2026-09-03 rebuild retired from the
objective, and nothing the chart drew. So the panel now leads with the chart's
lines, built from `TREND` rather than from a second list written by hand, in the
chart's own order.

Two specs, both watched red first.
`tests/the_chart_palette_is_the_one_that_was_measured.rs` pins the steps and
the sequence in both modes and refuses a raw hex in `TREND` — four assertions,
each shown failing on its own, including the hex one, which the token assertion
short-circuits past under the obvious mutation. `missing_panel_rows` in
scripts/site_smoke reads both the chart's keys and the panel's rows off the
rendered page in a browser and asserts they are equal: dropping the spread
turns it red, and so does reversing it, since containment would let the two
orders drift while a reader matches a faint line to its number by position.

Also here: welfare's chart reader stops rounding to hundredths. The rounding
existed to survive a `Math.round` the vertical axis never needed — the axis is
per-series and floating — and it cost the panel the last two digits of every
move.

OPEN, unchanged by this: whether a chart row should mark the commits where the
baseline was re-based. `bench/welfare_floor.json` holds 224 history entries and
exactly two begin "re-basing, not a gain"; both join to their rows through the
`kanso#NNNN` in their prose, which works and breaks silently. The version worth
building stamps the baseline into the row itself, beside `scored_by` and
`scored_weight`, and back-fills the two historical steps once.

## 2026-09-09 (second) — page_drift reads committed history, so it under-reports on a dirty tree

**DONE.** kanso#1345 ran `sh scripts/gates/all_pages.sh` twice before pushing and
both times read `page drift 3/3` — at the budget, green. CI on the same content
read `the log is 4 entries ahead, and the budget is 3` and failed the
cost-goldens job, taking `welfare` with it as a skip.

Neither reading is wrong. The gate asks git for the last commit that touched
`docs/compiler.html` and diffs that commit against HEAD, so it sees only what
is COMMITTED. Locally the new log entry was still in the working tree, so it
was not in the diff and the count was the three that came before it. The commit
turned the same tree into 4/3.

It cuts both ways, and the second direction is the one that costs a round: an
uncommitted page edit does not discharge the debt either. Adding §63 to
docs/compiler.html and re-running the sweep still reported the failure, because
the edit was not yet a commit. Run this gate after committing, or read its
number as a lower bound.

Two other things this found. The gate keys on `docs/compiler.html` alone, so
editing docs/numbers.html — which is what kanso#1345 did, twice over — moves no
count at all. That is the gate working as written rather than a defect: the two
pages are different things, and the settled-design page is the one the log is
supposed to stay level with. And the budget is genuinely cumulative across
sessions: the four entries named were kanso#1341, #1343, #1344 and #1345, of
which two are the chart campaign, so §63 was written for that campaign as the
gate's own message invites.

## 2026-09-09 (third) — a merged-check diagnostic on the module path had no location at all

**DONE.** kanso#1340 refused moving `check_merged` to the root because a
root-raised diagnostic loses the file, the span and the `(module …)` suffix,
and named provenance on merged declarations as what the thread owed next. That
plumbing was prototyped on 2026-09-08 and left in a scratch directory with two
defects, both recorded on the entry above. This is the repair, and the second
defect turned out to be worse and more useful than the record had it.

**DEFECT ONE: the attribution was dynamically scoped.** The prototype read the
file from a thread-local set by the walk, and its `Attributed` iterator held
its last item's guard until the iterator itself dropped, so a walk outliving
the raise site leaked one file's attribution onto a diagnostic raised somewhere
else. `Diagnostic` now carries `file: Option<Arc<str>>` set AT THE RAISE SITE,
from the declaration in hand, by `Diagnostic::about(&decl.file)`. A value
passed in has no guard to outlive it. The whole golden suite passes, including
`error_corpus_reports_each_golden_diagnostic`, the test the prototype turned
red — so the plumbing is inert where the prototype's was not, and a check opts
in one call at a time.

**DEFECT TWO IS NOT "render_across is never called".** On the module path a
merged-check diagnostic was formatted as kind, message and the `(module …)`
suffix, with the span and the source line DROPPED ENTIRELY — the loop built
that string by hand and never touched the renderer. kanso#1340's blocker was
not a thing to build; it was sitting in `compile_module_loaded` being done.

The corpus already held the proof, in a pair nobody had read side by side.
`tests/golden/errors/let_binding` carries both variants of one program:

    .stderr           error[name]: `let` is not a type …
                        --> let_binding.kso:2:7
                         2 |   let x = 1
                                   ^
    .imported.stderr  error[name]: `let` is not a type … (module let_binding)

Same program, same error, two routes through the front end, and the module
route reported no location. `compile_module_loaded` now renders through
`render_across`, which picks each diagnostic's own source and falls back to
naming a file with no quoted line when its text is not in hand. Where that
source comes from is the next section: the first two answers both cost the
objective, and the third costs nothing.

The suffix stays at the end of the header line, so an existing message is
byte-identical and simply gains the two lines under it. That is what keeps the
blast radius small: one check wired, and exactly two goldens move, both by
addition. Measured on a real two-file module, a dependency's error caught only
by the merged check now reads `--> …/inner/core.kso:5:9` with the line quoted,
where it used to read the message and nothing else.

`check_binding_patterns` is the one check wired, as the caller that keeps this
from being plumbing with nothing behind it.

**Two things found on the way.** The per-file checks were never broken: the
`unused` refusal already names a dependency's file and line correctly, so the
loss is specific to the whole-program check over the merged program. And the
two goldens that moved are single files compiled as modules, so they prove the
module ROUTE gained a location and NOT that a dependency's file is named — the
`errors_module` fixtures still pass untouched, because their errors come from
checks not yet wired. A cross-file golden is still owed and is listed below
rather than claimed here.

OPEN, in order: `.about(&decl.file)` at the remaining raise sites in
`check_merged_after_aliases`; the entry and library paths' merged renders,
which have the same shape; a cross-file fixture under tests/golden/errors_module;
the forty-four module goldens regenerated; and then the reorder this was always
for, at kanso#1340's repricing.

**AND THE SIZE OF THAT FIRST ITEM IS NOT TWENTY-TWO.** This entry said so
until the survey behind it was redone. The driver calls 23 checks; 14 raise in
their own body, 17 sites between them, 1 wired here, so 13 checks and 16 sites
remain. Three have a declaration with `.file` already in hand at the raise
site; the rest raise inside a closure.

The redo also found what a count of the callees cannot see. NINE of the 23
raise nothing themselves -- `check_boolean_equality`, `check_build_blocks`,
`check_call_arities`, `check_call_shaped_list`, `check_decidable_failures`,
`check_err_as_value`, `check_field_exists`, `check_if_arity` and
`check_literal_arguments` -- and delegate to helpers that do. check.rs holds 65
`Diagnostic::new` against the 17 inside the driver's direct callees, so wiring
the checks is not the whole job and the helper sites need the file reaching
them too.

Two bad surveys preceded the good one, and both failed silently. The first was
a boundary scan by line that mis-sliced any body holding a nested `fn`, and
reported zero raise sites for four checks against 65 in the file. The second
matched `\nfn NAME` and found NOTHING AT ALL, because the driver is `pub fn` --
a scan that returns an empty set reads like an answer. A survey whose result
is a count wants a total it can be checked against; 65 is that total here.

**THE FIRST TWO SHAPES BOTH COST THE OBJECTIVE, AND THE THIRD IS FREE.** The
renderer needs the text of the file a diagnostic is about, and the merge loop
had been eating `parsed` — so the obvious repair is to keep the text across the
merge. Round one did that, with a `Vec<(String, String)>` built as the loop
consumed the triples, and CI turned five compile gates red:

    compile_allocs          29,606 ->      29,613     +7
    compile_peak_bytes     773,818 ->     774,847     +1,029
    compile_instructions 48,791,172 ->  48,744,634    -46,538
    entry_instructions  162,170,772 -> 162,528,521    +357,749
    library_instructions 162,970,167 -> 162,823,672   -146,495

The +7 is exact and derived: that vector is one allocation per module loaded,
and `KANSO_PHASES=1 kanso check compile_corpus` prints seven `load` lines
(compile_corpus, std/json, std/text, std/list, std/testing, std/text again,
std/render — std/text twice because a module is compiled once per path to it,
which "Remembering a compiled module costs more than compiling it again"
measured and declined). This container read 29,613 and 774,847 too, agreeing
with CI to the unit on both, as those two rows always have.

The three instruction rows are layout: three routes on ONE binary sha moving
+357,749, −46,538 and −146,495 in the same job cannot be seven allocations.
`welfare` priced the whole thing at −0.01. A fall means the change goes or the
weights are argued, and the right answer here was a third one: the shape was
wrong.

Shape two: keep `parsed` itself alive and take each program out of it in
place, so nothing new is allocated at all. allocations went back to 29,606 and
peak went the OTHER way, 773,818 -> 775,730 — worse than shape one by 883
bytes, because a `(String, String, Program)` triple is about three times the
width of a pair and holding that vector holds the wider one. Priced on the
objective, the 7 allocations saved are worth about a twentieth of what the 883
bytes cost. Declined.

Both shapes are answering the wrong question, and the two measurements
together say so: the rise in each is the size of the VECTOR and not of the text
it points at — the same files held two ways, 1,029 bytes and 1,912 bytes, where
holding the corpus's actual source would be tens of kilobytes.

Shape three ships, and it holds nothing at all. The loader is now
`module_sources`, and
the diagnostics branch CALLS IT AGAIN. A clean compile runs the code it always
ran, byte for byte; a compile that is about to print an error opens its own
files a second time, which nothing anywhere measures. On this container both
rows read exactly main's numbers — `compile_allocs=29606`,
`compile_peak_bytes=773818` — and `welfare` reads 66.30 against the floor of
66.30. If the second read fails the diagnostics still print, without their
source lines.

The general form is worth keeping. A repair that hangs state on the success
path to serve a failure that usually does not happen has bought the wrong
thing, and doing the work again on the failing path costs nothing anyone
measures. The three compile veins said so within a round.

**CI SAID, AND THE ANSWER IS A BETTER PROOF THAN ROUND ONE'S.** All three
instruction rows moved on the shipping shape too, and this time all three
fell together:

    row                    golden        CI            move
    compile_instructions   48,791,172    48,746,831    -44,341   (-0.0909%)
    entry_instructions    162,170,772   162,044,531   -126,241   (-0.0778%)
    library_instructions  162,970,167   162,840,377   -129,790   (-0.0796%)
    compile_allocs             29,606        29,606          0
    compile_peak_bytes        773,818       773,818          0

The last two rows are what make this worth writing down. The shipping shape
adds NO work to any successful compile, and the two counters that measure the
front end's work say so in the same job that counted the three that moved. So
the layout reading is not an inference from the size of the change here; it is
a measurement with the alternative already excluded.

Round one is the control. Seven allocations and a kilobyte moved those same
three rows -46,538, +357,749 and -146,495 -- three directions on one binary
sha. Zero allocations moved them -44,341, -126,241 and -129,790. A row that
answers differently to two shapes of one change while the work counters hold
still is the compiler's own bytes, and nothing about the corpus.

Summed on the objective's compile term the fall is 170,582 (-0.0809%), so
`welfare` rose and the floor is held at 66.30 in this PR with the reason
recorded. It is banked as layout and claimed as nothing else: the front end
did not get faster at anything, and the next change is not free to spend this.

## 2026-09-09 (fourth) — the chart drew two differently-scored populations as one line

**DONE.** The design chat's entry of the same day, "the cliff is the run terms
joining the score, and the chart draws a coverage change as a fall", diagnosed
what Clay has been looking at and named three pieces. This is the third of
them, the one that makes the page honest today rather than right.

`scripts/welfare_rescore` scores a row on the counters it carries and
renormalises the weights that remain, and it writes `scored_weight` into every
row so that a reader is not fooled. The chart never read the field. Across the
500 rows there are five runs of it and four boundaries:

    rows       scored_weight   welfare
    0..30      0.28            74.64 -> 73.77
    31..181    0.00            no score, the line has a gap here
    182..390   0.28            82.82 -> 89.72
    391..438   0.44            91.67 -> 91.57
    439..499   1.00            58.96 -> 66.30

Two of those boundaries are steps in the line, and neither is the compiler.
Compile instructions joining on 2026-09-03 takes the score 89.72 -> 91.67. The
run counters joining on 2026-09-06 take it 91.57 -> 58.96, because the compile
terms carry the advantage they have accumulated since august while the run
terms start at parity against a baseline measured on the day they joined. That
second one is the whole of the "dramatically worse" the page has been showing.

The chart marks every boundary with a dashed muted rule labelled with the
coverage to its right, and splits the welfare polyline per run, drawing it
faded wherever the coverage is below 1.00. Each segment reaches one point into
the next run so the step itself is drawn rather than left as a gap the eye
closes by guessing. Coverage gets no colour of its own: it is not an entity,
and a categorical hue would have made it an eighth series.

**`scored_weight` IS TEXT, AND THE FIRST CUT OF THIS DID NOTHING.** Every row
in the history carries the string "0.00", "0.28", "0.44" or "1.00", the same
way `welfare` is text and has always been read through `parseFloat`. A
`typeof r.scored_weight === 'number'` guard read all 500 rows as unscored,
found one run, drew no rule, and split nothing -- a change that ships, passes
its own eye test, and leaves the picture exactly as wrong as before. The runs
are keyed on the text now, which is canonical to two places and so compares
exactly, and the number is parsed only to decide whether the coverage is full.

The spec is in the site smoke, which renders the page in a browser and reads
the marks off the DOM. Its stub carried six rows at one coverage and could not
have seen any of this, so it now carries the shape the real history has: four
older rows without the run counters at 0.44, two with them at 1.00. That makes
`missing_series` a real assertion -- [2 2 2 5 6 6 6 6], the two run lines
short, the four old counters full, and welfare in TWO strokes -- and adds
`missing_bounds` for the one rule and its label.

Both were watched red first, and they fail differently, which is what says
they are testing two things. Under the `typeof` guard: `marks: []` and series
[2,2,6,6,6,6,6], both checks red, the real bug reproduced. With the split
removed but the guard correct: series red, `marks: ["coverage 1.00"]` still
right. Restored, green.

The prose said the score falls "from about 75 to about 52" and the column has
read 91.57 -> 58.96 since the compile epochs moved under it. Corrected, with
both steps named. `sh scripts/gates/all_pages.sh` green on all three. The
labels were checked for collision rather than eyeballed: four rules, tightest
gap 16px, right edge 1097 of 1200.

OPEN, both the chat's and both still cloud's: reconstructing the run terms for
the 439 earlier rows so they score on all five, and the compile-side epoch
table. When those land the boundaries stop being steps and these rules stop
having anything to mark.

## 2026-09-09 — the cliff is the run terms joining the score, and the chart draws a coverage change as a fall

Clay, 2026-09-09: "the latest welfare metric still looks like it has gotten
dramatically worse. I do not understand this. the only things you should have
done recently to the corpus had to do with applying the metric consistently.
that shouldn't have had anything to do with the compiler being worse in any
kind of way."

He is right, and the column agrees with him: it reads 66.30 across the last
fourteen commits with every counter byte-identical. What looks dramatically
worse is the shape, a peak of 91.67 on 2026-09-03 and 66.30 today, and the
whole of that gap is the step at 2026-09-06 11:26. This entry corrects the
mechanism the 2026-09-08 entry gave for that step, because the correction
changes what a reader should conclude from the picture.

**The 91 was never a score of the compiler.** `scripts/welfare_rescore` scores
a row on the counters it carries, drops a term whose counters are absent, and
renormalises the weights that remain; its own header says a row scoring well
on what it has "reads the same as a full row scoring well." `run_instructions`
and `run_peak_bytes` exist in none of the 439 rows before 2026-09-06 11:26 and
in all 61 after. So every earlier row is scored on the two compile terms alone,
at `scored_weight` 0.44, and reads high because the compile ratios are large.
The first row after carries all five at `scored_weight` 1.00, and the run terms
enter at parity, r = 1 against satiation 2.0, so each contributes a third of
its weight and pulls the total down:

    2026-09-06 09:43   scored_weight 0.44   welfare 91.57   compile terms only
    2026-09-06 11:26   scored_weight 1.00   welfare 58.96   all five, run at parity

The 2026-09-08 entry said the earlier rows "score against a baseline they never
carried." They do not score the run terms at all. The remedy is unchanged; the
reading is different. The chart draws two differently-scored populations as one
line.

**The page already says this, and the chart does not.** docs/numbers.html:
"read `scored_weight` before reading a step in the line ... the step where the
run counters arrive is a change in what was recorded rather than in what the
compiler costs." That sentence is correct, and finding it is the reader's job.
The line draws 288 rows at coverage 0.28 or 0.44 and 61 at 1.00 in one stroke,
one colour, with no mark at the boundary. The rescore writes `scored_weight`
into every row so that, in its own words, "a reader needs to not be fooled,"
and the chart does not read the field. That is why the site does not look
right: the safeguard is in the data and in the prose and nowhere in the
picture. The prose is also stale, "from about 75 to about 52," against a column
that now reads 91.57 to 58.96, because the compile epochs moved under it.

**What Clay's principle requires, in three pieces.** A change in what is
measured is neutral to the score, so the line is flat across every one of
them.

1. The run side: the 2026-09-07 ruling, still unbuilt. Reconstruct the run
   terms for the 439 earlier rows from the per-benchmark counters they do
   carry (`instructions` and `encode_instructions` in 424 of them),
   share-weighted, based at 2026-08-10, so those rows score on all five terms
   and the boundary disappears.
2. The compile side: the epoch table from the 2026-09-08 entry, so each of
   the four compile epochs is scored against a baseline scaled to its own
   measurement.
3. The chart: draw `scored_weight`. A lighter or dashed stroke below 1.00, or
   a marker at each coverage boundary, so that until 1 and 2 land a step that
   is a coverage change looks like one. This is an afternoon, and it makes the
   page honest today; 1 and 2 make it right.

**OPEN, all three cloud's.** The rescore and the chart are code and the page
is cloud's surface.

## 2026-09-09 — the merged check has four routes, and the fourth is the repl

§60 and CLAUDE.md both say `kanso check` routes a single file by content and
that the three routes are three compiles: a DIRECTORY is a module, a file of
bare STATEMENTS is an entry, a file of DEFINITIONS alone is a library. That is
true of `kanso check` and it is not the whole census.
`check::check_merged_after_aliases` has FOUR call sites in src/lib.rs:

    line   caller                    route
     168   compile_parsed_entry      entry      (a file with bare statements)
     372   compile_one               THE REPL   (not reachable from kanso check)
     465   compile_library           library    (a file of definitions alone)
    3666   compile_module_loaded     module     (a directory)

`compile_one` has one caller, `compile_repl`, which has one caller,
src/repl.rs:290. Its own doc comment says it serves both `kanso play` — the
playground's convention — and the repl prompt, assembling imports and units
into one source. So the fourth route is a user-facing surface that the website
runs, and no census keyed on `kanso check` could see it, which is why three
separate readings of this code have said three.

**The module row's name is one level off too.** CLAUDE.md says the module route
takes `compile_module_inner`. It does — but the raise site is
`compile_module_loaded`, which `compile_module_inner` calls at src/lib.rs:3375.
The entry point and the raise site are different functions, and a survey
grepping for the raise site finds the second name while the doc names the first.

**What this owes.** kanso#1346 repaired the attribution on the module path and
wired one check (`check_binding_patterns`); the `.about()` wide pass is 13
checks and 16 sites. This adds a fourth render to that pass rather than three,
and the repl's is the one with a user watching: a diagnostic in the playground
that loses its file and span loses it in a browser. Not measured yet — whether
the repl route renders locations today is the next question, and it is asked
here rather than assumed either way.

**How the count went wrong before.** Recorded on 2026-09-08 in this log: a
survey whose product is a count wants a total to check against, because an
empty or short result set reads like an answer. The route census had a total
available and did not use it — `kanso check`'s three branches — and the
function has four callers. Grep for the callee, count the call sites, and
reconcile against the routes; do not derive the call sites from the routes.

---

## 2026-09-09 — a fold's in-place write was licensed without asking about its seed

**DONE.** Native disagreed with the interpreter on a fifteen-line program, and
the disagreement was silent: a stale value, printed as an answer.

**How it surfaced.** Building the compile epoch table (kanso#1347 item 2),
`scripts/welfare_rescore` walks the five hundred history rows carrying a
per-counter divisor and stores each row's divisor in a map keyed by commit. It
read 1.0 for every row. The rescored file came back byte-identical to the one
the old tool wrote, which is what sent me looking: a change that does arithmetic
on every row and moves no digit is either a no-op or a lie.

**The defect.** `src/linear.rs` decides which `put` and `push` sites may write
in place. Inside a fold it grants the folder's own accumulator parameter that
licence, on the strength of `folder_is_unique` — which inspects the LAMBDA and
nothing else. A fold writes into its SEED on the first step, so the licence also
needs the seed to be uniquely owned, and that half was never asked. Both grant
sites had the hole: the licence walk (`callsites_unique_in`) and the marking
walk (`walk_for_push_in`).

`unique_in_with`'s own fold arm has always asked both, so "is this fold's RESULT
unique" was answered correctly the whole time while "may this folder write" was
not. The two questions sit forty lines apart in the same file.

**The shape that reaches it** needs two folds. An outer fold's accumulator
carries a value and a record of that value at each step; the inner fold hands
the value straight back on a step with nothing to do, so the stored copy IS the
accumulator; the next step's write lands in both. Neither fold alone does it — a
write outside a loop traces back to a literal, and a folder whose seed is built
where it stands owns it.

    native: first: 2   second: 2   fresh: 2
    interp: first: 1   second: 2   fresh: 2

**The fix** asks both conditions, in one place, because the two walks must agree
about it and this is exactly where they had drifted:

    fn fold_owns_accumulator(&self, args, ctx, scoped) -> bool {
        self.folder_is_unique(&args[2], ctx, scoped)
            && self.unique_in(&args[1], ctx, scoped)
    }

**What it costs: nothing on the runtime side, and three layout-sized falls on
the compile side.** `sh scripts/gates/all_counters.sh` reads the twelve cost
veins and the lazy tier and every one is byte-identical, so no in-place site in
the benchmarked code was standing on a non-unique seed. The compile sweep saw
nothing move on this host, with six of the nine gates host-bound. CI then read
all three instruction rows:

    counter                golden          CI            delta
    compile_instructions   48,746,831      48,746,192      -639   (-0.0013%)
    entry_instructions    162,044,531     162,042,653    -1,878   (-0.0012%)
    library_instructions  162,840,377     162,839,321    -1,056   (-0.0006%)

with `compile_allocs` 29,606, `compile_peak_bytes` 773,818 and the machine-code
row byte-identical beside them. Welfare's floor rises 66.3039170230475 ->
66.30393941879086 and is ratcheted in this PR.

**Ratcheting a golden is a page edit.** The three rows are quoted by five
`data-golden` spans in compiler.html, and moving the goldens without walking the
pages left all five stale — `golden_prose` caught it as the LAST step of the
welfare job, so the job read red with the number itself green at 66.30 and its
floor met. CLAUDE.md's rule says a page edit ends with
`sh scripts/gates/all_pages.sh`; the trigger is wider than the rule's wording,
because a golden that moves silently re-points every span that quotes it. One of
the five needed prose rather than a swap: the sentence said the library vein
"fell 1,282,921 instructions, or 0.78%, to" that row, and a row that moves again
makes the arithmetic false. The landing is now a fixed figure and the span
carries today's reading beside it.

**The direction is not evidence the fix is cheaper, and the entry says so.**
Two mechanisms could each produce a move this size, and 639 instructions cannot
separate them: the analysis does MORE work at every fold site (one extra
`unique_in` on the seed) and LESS at a site the new condition rejects, because
the marking walk then never descends into the folder's body. A condition that
is strictly added cannot make the analysis cheaper on its own. The work
counters staying put is the usual layout signature, and that is what the rows
are recorded as.

**The fixture** is `tests/golden/micro/a_folds_seed_is_held_by_something_else`,
which `micro_corpus_agrees_across_engines` runs on native and on the oracle
against one golden, and the wasm and browser sweeps run on the third engine.
Watched red through that harness before the fix landed: it named the fixture,
the native run, and `left: "first: 2"` against `right: "first: 1"`. Its last
line is the legitimate case — a seed born at the call, which the fold may still
write through — so a licence simply switched off would not satisfy it.

**All three accumulator kinds were reachable, and the fixture carries all
three.** The grant is per fold rather than per write, so one hole covered
`put`, `push` and `append` — but that is a claim about the code, and the
cheap way to settle it is to write the other two and look. Built, and on the
unfixed compiler native answered every one of them wrong:

    kind    native            the oracle
    map     first: 2          first: 1
    list    first: [1 9]      first: [1]
    bytes   first: [120 121]  first: [120]

A fixture exercising one of the three would have left the other two resting on
the claim.

**What generalises.** A two-part condition split across two call sites is one
edit away from disagreeing, and nothing here would have caught the disagreement:
the analysis has no differential of its own, and the corpus had no program whose
answer depended on it. The counters could not see it either — an unsound licence
makes a program FASTER and wrong. What found it was arithmetic that had to move
and did not.

## 2026-09-09 — the compile side has three changes of measurement, not two

The 2026-09-08 entry on the compile term said the workload had been
re-measured twice, and named a four-row table whose third row was a
compiler change rather than a change of measurement. Scanning all 500
rows of the perf history for an adjacent move over 20% in
`compile_instructions` finds three:

    row  commit    what happened                     instr    allocs    peak
    446  5f5561c   #1291: lib/json drops std/list   0.4586   0.4490   0.5177
    476  dfd118a   #1321: lib/json -> corpus        2.7232   2.7207   2.1047
    485  8535884   #1331: sums both compile paths   4.3612   1.0000   1.0000

The middle row reproduces the three factors already recorded in the
floor file, to the digit, which is what says the factors can be read
off the history at all. The last row moves `compile_instructions`
alone, as `bench/objective_sources.txt` implies — one gate key each
for allocs and peak, two for instructions.

The first row is the one nobody wrote down, and the 2026-09-08 entry
supplies its own reason for counting it: "a term measured on a library
moves whenever that library changes what it imports: #1291 dropped
std/list from lib/json and halved the compile veins with the compiler
untouched." That is the disease #1321 was opened to cure, so it owes a
factor like the other two.

The record already disagreed with the entry. kanso#1347 item 2 asks
for "the epoch table from the 2026-09-08 entry, so each of the FOUR
compile epochs is scored against a baseline scaled to its own
measurement" — and four epochs need three boundaries. The count was
wrong against the record before it was wrong against the measurement,
which is the cheaper of the two to check.

**DONE — `bench/compile_epochs.txt`**, one boundary a line in
`bench/runbench_phases.txt`'s style, read by `scripts/welfare_rescore`
and applied to the baseline before the compile term is scored. Four
epochs: A rows 0..445 (lib/json with std/list), B 446..475 (without),
C 476..484 (compile_corpus, module row alone), D 485..499 (module and
entry summed, today's). A row's divisor is the product of the factors
of every boundary at or after it.

Either side of each boundary, welfare before the table and after it:

    boundary                     before             after
    a23b005 -> 5f5561c    59.0275 -> 61.0313   56.7964 -> 57.0280
       step                     +2.0038             +0.2316
    e12a68e -> dfd118a    70.0258 -> 67.7549   66.0242 -> 66.0242
       step                     -2.2709             +0.0000
    fd3144d -> 8535884    67.9134 -> 66.2874   66.2875 -> 66.2874
       step                     -1.6260             -0.0001

The two pure re-basings flatten. dfd118a and 8535884 touch no src/ and
no lib/, and their steps go to zero; the -0.0001 is the floor file's
four-place 2.7207, which is the precision the factor was recorded at.

**The first boundary keeps +0.2316, and that is the answer rather
than a residual.** 5f5561c is #1291, which halved the compile workload
and made the escape scan skip-then-iterate for runbench -3.3884%.
Taking the ruler out leaves the runtime win standing. A boundary that
flattened to zero here would have been the table erasing a real
improvement.

**One part of this is independent evidence and the rest is not.** The
446 and 485 factors were derived from the counter jumps, so applying
them and recovering 1.0000 is the same arithmetic inverted. The 476
factors were measured by another session at the time of the change and
written into the floor file; they land at 1.0000 against numbers they
were never fitted to. That is the check worth having.

**The table could not be measured at all until the fold-seed
miscompilation was fixed** (kanso#1349, same day). The divisor map read
1.0 for all 500 rows because every stored divisor aliased the last one:
`list/fold`'s folder was granted an in-place write licence without
anybody asking whether the fold's seed was uniquely owned. With the fix,
`oldest_divisor` reads compile_allocs 1.2215943, compile_instructions
5.446526138624, compile_peak_bytes 1.08960319 — the products above, to
the digit. The table's first working run was the miscompilation's first
reproduction.

**The boundaries a history carries must be a SUFFIX of the table**, and that
one rule covers three situations that look alike. It took three cuts to find.

The first refused any history missing a boundary. That turned three spec files
red for a reason none of them was about — `the_reconstruction_refuses_a_splice_that_moved`,
`a_row_predating_a_counter_is_scored_on_what_it_has` and
`the_score_says_what_it_was_made_of` all stage synthetic histories — and
seventeen fixtures would have had to carry three commits they say nothing
about, with every future one after them.

The second was all-or-nothing: refuse a partial match, give a history carrying
none of them no epochs. That is right about fixtures and wrong about the file
CI actually rewrites. `history.jsonl` is bounded to the newest 500 rows, so the
oldest boundary leaves the window one day while the table still names it — and
under all-or-nothing that is a partial match, so main goes red for a file doing
exactly what it is meant to do. Read off origin/perf-history on 2026-09-09
the three sit at rows 444, 474 and 483 of 500, so the first departure is 444
merges out — long enough that it would have arrived with nobody expecting it.
The position DROPS BY ONE WITH EVERY MERGE, because the window keeps the newest
five hundred, which is why this reading is dated: it was 446/476/485 two merges
earlier, and a fixed count written down here is a count that goes stale between
the measuring and the writing. It already did once, in this entry's own first
draft.

So: all present, the table applies. A hole in the middle, or the newest gone
while an older one stays, is a stale table and refuses. None at all is the
whole table fallen off the front — a fixture — and gets no epochs. The
boundaries that left the window are dropped from the product, because a
boundary older than every row scales no row.

**DONE — the spec, and the first cut proved nothing.**
`tests/the_compile_epochs_flatten_their_own_boundaries.rs` holds four.
The first stages, per boundary, a fixture carrying every boundary's
commit — so the epoch table applies at all — where only the
pair under test holds counters: 10,000 before and 10,000 x factor
after, so the four-place factors make every number an exact integer and
nothing rounds inside the fixture. The second replays the counter names
against `bench/objective_sources.txt`, so a typo cannot sit in the
table scaling nothing. The third leaves one boundary out of a fixture at a
time — never the oldest, so an older one is always still present — and requires
the refusal by name.

**The fourth is the one whose first cut proved nothing, for the second time in
this entry.** It scores the same rows twice, once against a fixture carrying
every boundary and once against a slid window, and requires the two to agree.
The obvious assertion was that the boundary still FLATTENS, and that survives
the mutation the test exists to catch: if the departed boundaries keep their
factors in the product, every divisor shifts by the same amount, so every step
is still right and a pair either side of a boundary agrees exactly as before.
What moves is the absolute column, against a floor that is ratcheted. Watching
the mutation pass is what found it; the rewritten test reads 97.1221 against
the whole-window figure and goes red.

The first cut passed under a mutation that moved both numbers. The
welfare column is a STRING — `fixed` renders it and `put` stores what
it rendered — and the helper walked digits from the colon, hit the
opening quote, and answered the empty string for every row: the
comparison was `"" == ""`. The helper now reads between the quotes and
asserts what it read is a number. The mutation that works is
`epoch_value v d[k] true` in `at_epoch`, which takes the identity arm
for every counter and still compiles; `1.0 * v / 1.0` does not, because
`f` goes unused and the tool dies before reading stdin, which the spec
then reported as a broken pipe rather than as the tool's own
diagnostic. The harness lets that write fail and prints what the tool
said.

**OPEN — the re-basing marks (task #449).** Stamping the baseline into
each row and marking where it moved cannot be derived from the
counters, and the measurement says why: the compile side has three
adjacent moves over 20% and the run side has ZERO across
`run_instructions`, `instructions` and `encode_instructions` over all
500 rows. The run baseline did move — kanso#1284 re-based the run
counters to parity, which is the -32.62 step — and no counter moved
with it, because only the ruler changed. A workload change shows up in
the rows and a baseline reset does not, so deriving the marks from the
counters would find three of the four and silently miss the largest.
Queued behind this entry because it edits the same file.

**Owed from kanso#1346, recorded here (closes the kanso#1339 thread).**
The ratchet's touched pass selected ELEVEN rows on kanso#1346's branch,
against kanso#1338's five selected and three proved. kanso#1346 merged
without recording it.

**ANSWERED — what else the fold-seed hole reached.** `scripts/welfare_rescore`
was miscompiled and nothing pinned its output, so the question is whether any
other tool in `scripts/` was in the same position. Fourteen were run on native
and on the oracle and their output compared: `welfare`, `page_drift`,
`golden_prose`, `diagnostic_coverage`, `fingerprint`, `book_quotes`,
`grammar_check`, `stale_a_panel`, `perf_record`, `trend_gate` and `ratchet`
agree byte for byte. `prose_check` and `book_panels` were not compared — the
oracle does not finish either inside five minutes. `site_smoke` disagrees, and
it is the divergence `tests/a_file_that_is_not_text.rs` already pins: native
reads `docs/kanso.wasm` and hands the bytes back, the oracle refuses with "the
bytes are not text". Reduced to seven lines, that is the whole of it. Whether
`read_file` should be byte-transparent on every engine is filed as a design
question and is not re-asked here.

## 2026-09-09 — the rescore read its own arithmetic back as a measurement

Main went red at 06:48 on the `perf history` job, one of nineteen, and
stayed red: `the splice rows disagree on a phase they share`. The cause
is kanso#1346, merged the run before.

`scripts/welfare_rescore` rebuilds `run_instructions` for the rows
measured before runbench existed, and it WRITES the rebuilt count into
the row it hands back. ci.yml feeds it
`origin/perf-history:history.jsonl` — its own previous output — appends
one row and rescores the lot. So a rebuilt count comes back as an input,
and nothing in the file says which counts were measured.

The numbers, read off three successive history files:

    file       all-eight rows   run-bearing   carry BOTH
    dc658d65   48 (last 438)    62 (first 439)     0
    81ffc9e5   48 (last 437)    63 (first 438)     0
    22ca0fb3   48 (last 436)   112 (first 389)    48

The first two are the design: the phases were the measurement until
runbench arrived, the consolidated count took over, and the two sets
meet without overlapping. The third is one run later. Forty-eight rows
gained a rebuilt count at once, the oldest of them became the splice
anchor at line 389, and it was compared against the newest all-phase row
at 436 — forty-seven commits apart, so of course every phase disagreed.
The refusal was right. What it was refusing was this tool's own output.

**DONE — a measured run count is not the same as a row carrying one.**
A row carrying every phase is from before runbench, when the phases WERE
the measurement and the consolidated count did not exist to be taken, so
a count sitting on such a row is this tool's arithmetic. The splice
anchor now selects rows that carry a count and NOT the full phase set.
That is checkable against the record rather than asserted, which is what
the table above is for.

**And the rebuild recomputes rather than trusting what it finds.** A
count written on an earlier run was scored against whatever anchor that
run picked, and keeping it lets one run's arithmetic outlive the
reasoning behind it. Recomputing also repairs the file already carrying
the rebuilt counts, with no hand edit: the corrupted history rescores
clean, and a second pass over that output is byte-identical.

**The property is idempotence, and it is what CI relies on.** "Rewrite
the whole column every push" is only safe if feeding the tool its own
output gives the same answer, and nothing asserted that. The spec is in
`tests/the_reconstruction_refuses_a_splice_that_moved.rs` beside the
refusal it belongs with: rescore a three-row history, feed the output
back, require success and an identical column. It reproduces main's
failure message in a postcard, and it would have caught this on the day
kanso#1346 landed rather than one merge later.

**What generalises.** A tool whose output is its own next input needs
that property pinned, and this one had a guard against exactly the error
it went on to commit — the splice check exists because a rebuilt row
spliced onto a row that moved slides the whole history smoothly and
plausibly. The guard fired correctly and pointed at the wrong culprit,
because the file it reads cannot say which numbers were measured. A row
that does not record where its number came from will eventually be asked
to answer for it.

**And the page was describing a coverage level that no longer exists.**
`docs/numbers.html` said a row holding three of the five counters scores
0.74 — it said 0.44, and named the two rises as 0.28 to 0.44 and 0.44 to
1.00. Read off the file today the levels are 0.00 (151 rows), 0.28 (237),
0.74 (48) and 1.00 (64), and what each holds is:

    0.28   compile_allocs, compile_peak_bytes
    0.74   those two, compile_instructions, run_instructions
    1.00   those four and run_peak_bytes

The forty-eight at 0.74 are the reconstructed rows. kanso#1346 gave them a
rebuilt run count, which took them from three of five to four, and the prose
was not followed — the same omission that let the rebuilt counts be read back
as measurements. The figures moved twice over: once when the reconstruction
landed and again under the epoch table.

**None of the four is a `data-golden` span**, so `golden_prose` cannot see
them and CI stays green with the page wrong. That is the gap kanso#1338 closed
for the library row by wiring it into the gate, and it is still open for
everything the chart's prose quotes. The figures are dated in the sentence
now, which is a weaker guard than a span and the honest one to have while they
are unwatched.

The two rises read 0.28 -> 0.74 with the score 88.28 -> 62.67, and 0.74 ->
1.00 with 64.95 -> 56.73, measured against origin/perf-history rescored by
this tree — which is what CI writes on the next push to main.

## 2026-09-09 — the blob stops being committed, and the guard covered two entries of eleven

`docs/kanso.wasm` is what the playground runs and what every spec in
`tests/wasm_engine.rs` runs, and it was a build artifact that was also
committed. It is not committed any more, and the guard that was supposed to
make that safe was covering two of the eleven places the artifact is opened.

**The criterion was already written down and the answer was already in.**
ci.yml carried a step, deliberately not a gate, whose own comment said: "Once
a few runs have said the same thing, the answer decides whether this becomes a
gate or the committed blob stops being committed." It has said DIFFERS every
time. The measurement is not needed to reach that answer either — the
committed blob's last commit is `6f8c876e`, from 2026-09-06, and main has
merged past it many times since, so it is built from older source and cannot
reproduce whatever the toolchain does. Rebuilt on this container it is
1,736,492 bytes against the committed 1,719,102, a difference of 17,390. Both
steps are gone; the rebuild that every job already ran stays.

**Four jobs rebuilt it and a fifth read it, and the first sweep here said
five and called that every consumer.** ci.yml's specs, other-host and site
jobs, pages.yml and `scripts/browser_differential.sh` do rebuild. The asset
digests job does NOT: it runs jekyll over `./docs` and fingerprints the `_site`
that copy produces, with no rebuild anywhere in it. So the sentence "fingerprint
reads `_site`, built after the rebuild" was true of pages.yml and false of the
job in ci.yml with the same shape.

Found by running it rather than by reading it. A `_site` copied from a `docs/`
with no blob gives `missing asset: kanso.wasm` and exit 1 out of
`scripts/fingerprint`, and `undigested_references.sh` then fails behind it on
five surviving references. That would have been a red round. The job rebuilds
first now — before jekyll rather than before the fingerprint, because jekyll is
what copies the file — and the repaired sequence reads fingerprint exit 0 and
gate exit 0 on a staged `_site`.

The lesson is the one the tree keeps relearning: a list of consumers assembled
by grep names the files that mention the artifact, and the job that breaks is
the one that reads it through something else.

**The guard was at two call sites and there are eleven.** `freshness()` was
called from `the_wasm_engine_agrees_with_the_golden_corpus` and
`the_wasm_engine_complains_the_way_the_others_do`. The other nine —
four playground-prompt specs, two page-failure specs, the builtin-count
refusal, the error corpus and the partial-over-a-value limit — called
`Toolchain::load` with no check at all, so a stale blob let nine specs run an
old engine and pass. The guard's own doc comment said a stale artifact "would
let this whole file pass while proving nothing about the source"; that was
true of nine twelfths of the file it was written in.

Measured rather than reasoned: with the blob moved aside, the suite reported
eleven failures, two of them naming `scripts/build_wasm.sh` and nine saying
only "the wasm artifact reads". The check now lives in `Toolchain::load`,
where all eleven pass, and re-running with the blob absent gives the same
sentence eleven times.

**The checkout arm is deleted with the thing that caused it.** kanso#1180 added
an arm for the case where the blob is a few milliseconds older than every
source file, because a fresh clone writes `docs/` before `src/` and so tripped
the guard on every checkout. A checkout no longer writes the blob, so that arm
cannot fire; keeping it would leave a paragraph of explanation for a state the
tree can no longer reach.

**And it closes the mtime guard's most common false pass.** kanso#107 kept the
guard an mtime comparison and wrote down that content it cannot see may be
stale. The way that bit in practice was `git checkout -- docs/kanso.wasm`,
which stamps a NEW mtime on OLD bytes: the guard then passes and the engine
runs a blob built from an older compiler. kanso#1350's own validation
paragraph is a case of it — two wasm specs failing on this container for
exactly that reason. With no committed copy there is nothing to restore, so
that trigger is gone. The guard is still mtime-based and still cannot see
content; this removes the one routine way of fooling it, not the weakness.

**What this does not claim.** The pack keeps every historical version of the
blob — 382 of them, 26,886,564 bytes — and removing the file from the tip does
not shrink an existing clone. The saving is prospective: the blob stops gaining
a version per rebuild that anyone remembers to commit.

## 2026-09-09 — a row says where its run count came from, and the mark is sticky

The 2026-09-09 red main was a provenance failure wearing an arithmetic
costume. `welfare_rescore` wrote rebuilt `run_instructions` into the rows it
handed back, ci.yml fed it `origin/perf-history:history.jsonl` — its own
previous output — and with nothing on the row saying which counts were
computed, the rebuilt ones read as measured and one became the splice anchor.
kanso#1350 fixed the tool: the anchor now takes rows carrying a count and NOT
the full phase set, and the rebuild recomputes rather than trusting what it
finds. That closes the loop for this tool. It does not help the next reader,
and the file still could not answer the question.

It can now. A row that this tool rebuilt carries `run_source: "rebuilt"`; a row
whose count it did not write carries `"measured"`; a row with no count carries
nothing. Read off the live history the split is 48 rebuilt, 65 measured, 387
unmarked, and the mark agrees with `scored_weight` on every row — all 48
rebuilt sit at coverage 0.74, all 65 measured at 1.00, and no row has a count
without a mark. The welfare column and every `run_instructions` are
byte-identical to what CI wrote, and the file stays a fixed point under its own
tool over three passes.

**The mark carries no factor, and that is the design rather than an omission.**
The compile side is RE-BASED by an exact divisor a row could name;
this side is REBUILT from phase shares, and there is no divisor because no
ruler moved. A `1.0000` written here for symmetry would read as "measured
against an unmoved baseline", which is the precise misreading the mark exists
to prevent.

**THE FIRST CUT STRIPPED THE MARK AND RE-DERIVED IT, and that was wrong.** It
looked like the welfare column, which is recomputed every pass, so it was
treated the same way. It is not the same kind of fact. Whether a count was this
tool's arithmetic is a fact about where the number CAME FROM, and nothing
re-measures a commit from three weeks ago, so it cannot expire. The failure is
visible in the degraded pass: with no row available as an anchor nothing is
rebuilt, and a mark re-derived from scratch then sees only a row carrying a
count and writes `"measured"` on it. Measured on a history with the anchors
removed:

    fixture            rebuilt   measured
    the strip          0         48
    the sticky mark    48        0

Forty-eight rows relabelled from true to false, by the field whose whole job is
to stop that confusion. The mark is sticky now: a row already claiming
"rebuilt" keeps it, and "measured" is only ever written onto a row that has a
count and no claim. With an anchor present the rebuild recomputes anyway, so
the sticky path is the degraded one, and there it degrades toward the truth.

`a_rebuilt_count_stays_marked_rebuilt_when_the_anchor_is_gone` pins it, entered
through the real tool on a three-row fixture and then on the same rows with the
anchor dropped. Watched red under the mutation that restores the strip, which
reddens that spec and leaves the other three green.

**Still owed, and it is the other half of the task.** The compile side has an
exact per-counter divisor in `bench/compile_epochs.txt`, and a row does not yet
name the epoch it was scored in. That wants the boundary commits tracked
through `crossed` rather than only their product, so it is a real change and
not a line. The two marks answer different questions and must not be made to
look alike.

---

## 2026-09-09 — a row names the epoch its baseline was measured in

**DONE.** `bench/compile_epochs.txt` names three commits that changed how the
compile counters are measured. `scripts/welfare_rescore` divides the baseline
by every factor from a row's epoch forward, so two rows either side of a
boundary are scored against two different baselines. Which one a given row sat
on was not written anywhere. A reader comparing a row from March against one
from September was comparing two numbers over two rulers and had nothing on the
page to tell them apart.

Every rescored row now carries `scored_base`. The fold that computes the
divisors already walked the boundaries; it now carries the list of boundaries
still ahead of the row alongside the divisor map, and stamps each row with the
first one left. A row past all three reads `as measured`.

**The boundary row belongs to the NEWER epoch.** Its own factor is already
divided out — it is the first row measured the new way — so it is named by the
next boundary, not by itself. The first cut named from the list before the
row's own commit was removed, and stamped row 443 `before 5f5561c` when 5f5561c
is the commit that row IS. That reads as though the row predates a change it is
the first to carry, which is the one misreading the column exists to prevent.
The spec pins the whole sequence rather than a shape, so the off-by-one turns
it red wherever it shifts.

Over the live 500-row history: 442 rows `before 5f5561c`, 30 `before dfd118a`,
9 `before 8535884`, 19 `as measured`. The boundaries land at rows 443, 473 and
482, which cross-checks against the positions recorded independently on
2026-09-07. Every other column is byte-identical to what the previous tool
wrote over the same input, and the output is a fixed point under itself.

**Two specs, both watched red first.** `every_row_names_the_epoch_its_baseline
_was_measured_in` fails under the off-by-one with the whole sequence shifted by
one row. `a_history_under_no_epochs_at_all_is_scored_as_measured` holds the
second copy of the naming — `divisors` has an arm for a history carrying no
boundary at all, which builds the map in one pass instead of folding across
boundaries, and the naming is written there too. Without that spec the second
copy could say anything.

**A new column reddens seven specs in a file that pins whole rows, and that
is the corpus working.** `tests/a_row_predating_a_counter_is_scored_on_what_it
_has.rs` compares each rescored row byte for byte rather than reading one
field out of it, so adding `scored_base` broke five scoring cases and both
no-column cases at once. Nothing was wrong with those specs. The rows this
tool writes are read by the chart and by the tool itself on its next pass, so
a field added to them is a change to what those readers see, and a spec that
could not tell would be the defect. Their expectations carry the new column
now, and the file says why every fixture row in it reads `as measured`.

**What this does NOT give the run side.** A rebuilt `run_instructions` has no
factor: it is a ratio to an anchor, not a division by a recorded constant. The
run half of this thread (kanso#1351) marks such a row `run_source: rebuilt`
instead. Writing a fabricated `1.0000` there to make the two sides symmetric
would read as "measured against an unmoved baseline", which is exactly the
misreading the mark prevents.

---

## 2026-09-09 — an explanation of the error model, checked against the interpreter, found three gaps

OPEN, all three cloud's; the first two are the language as designed, not shipping. Clay asked for a written explanation of how failure
works in kanso for a friend. Every claim in it was run through
`target/release/kanso play` before it went out, and three did not hold as the
design says they should. None of them is in the book, which uses only the
spellings that work; each is a place where a reader who trusts a ruling
rather than a page gets a wrong answer.

**1. The possible-none check is gated behind an environment variable, and
the gate is the defect.** `check_none_exhaustive` in src/check.rs is the
diagnostic the book's own story implies — "this can be a none and `describe`
has no arm for it — resolve it here, or give `describe` a `none` arm" — and it
runs only under `KANSO_EXHAUSTIVE`, where the 2026-07-24 none campaign left
it while it waited on per-arm return sets. Clay, on reading that here:
"KANSO_EXHAUSTIVE is of course total nonsense. you're just describing how the
language works. the exhaustiveness when you're looking for a match on an arm
has always been the way the language works since like the first couple of
days of designing it." So there is nothing to rule and nothing to wait for:
a call whose argument can be a none, made to a group with no `none` arm, is
refused at check, and the flag comes out. The program the explanation used is
the book's menu sample with the `none` arm deleted and the call moved into a
function body:

    fn describe price
      "{price} yen"

    fn quote menu
      describe menu["pocky"]

    print (quote { "dango":350 "taiyaki":500 })

    kanso play                       <none> yen, exit 0
    KANSO_EXHAUSTIVE=1 kanso play    error[exhaustive] at the argument, exit 2

The second line is the language; the first is what ships. With the arm typed
`price:int` the bare run instead dies at execution time: `error[runtime]: no
overload of `describe` matches these arguments`, and a literal `none` handed
to that typed arm fails the same way at runtime, where a literal string
handed to it is refused at check (`literal_arg_type`). The 2026-08-15 sitting
recorded the same rule as "8: exhaustiveness dissolves into per-call
coverage": each call's inferred value set is checked for an unambiguously
matching arm, provable gaps are compile diagnostics, unprovable calls keep
the runtime err. The 2026-07-24 entry "what the last exhaustiveness report
is, and is not" says the one false report left was a group-level return set
where a per-arm one would be exact, and that is an implementation detail for
whoever ungates it. The explanation as sent states the check as the language,
without the flag.

**2. The 2026-08-29 gavel "effects are types, and the words are the only
doors" is unbuilt on the point measured: a `.` over an io still binds
automatically, so a `.` step headed by `rescue` or `annotate` is swallowed.**
The ruling: there is NO automatic bind; `<t>effect` is a box that can be
passed as data; `bind`, `annotate` and `rescue` are the sole eliminators,
ordinary functions taking the effect first. Under it, `effect . rescue orders`
is the ordinary pipe supplying the first argument — `rescue effect orders` —
because the dot no longer opens the box. On the shipped interpreter the dot
still opens it: the step synthesises a bind around `rescue orders`, bind skips
on failure, and the callback is never called.

    os/read_file! "no-such-file.txt" . rescue orders . print
      error[endpoint]: unhandled err reached the executor: "cannot read ..."

    rescue (os/read_file! "no-such-file.txt") orders . print
      no orders yet

    os/read_file! "/etc/hostname" . shout . print
      vm!!                       (the retired automatic bind, still shipping)

Both of the first two parse. The first is what the ruling describes and its
handler never runs; the second is what every fixture and both book chapters
use, and it is also the ruled prefix form, so nothing published is wrong. An
earlier draft of this entry filed the piped form as a fresh ledger question,
"build the rider or retire it"; Clay's correction the same hour — "we got rid
of that rule when we agreed that you have to use explicit combinators" — is
the gavel above, and a ruled question is never re-asked. What is owed is the
build: the dot stops binding over an effect, a box where a value is expected
is refused, and the two book chapters that teach the automatic railway (ch04's
call-site short-circuit, ch05's "piping into an io is bind") move with it —
the ledger's "The book teaches the boundary language" entry already holds
that half. The ruled chain, in the shape the 2026-08-29 gavel "the chain line
keeps its dot" gave it — "the combinators look and act like regular
functions", so a continuation spells them `. rescue orders` like any other
function —

    os/read_file! "the-orders-file-that-is-not-there.txt"
      . rescue orders
      . bind shout
      . bind print

parses today and dies at the executor with the handler never called, for the
same reason as the one-line form. Under "A ruling outranks a lead" the gavel
is at the front of the queue with item 1.

**3. A stale entry in the lexer's borrowed-keyword table names `rescue`.**
`kanso_form_for` in src/lexer.rs lists `try | catch | except | rescue` with the
message "a failure rides the same rails as a value; an arm names the err",
written when the language had no such word. The table is consulted by the
needless-continuation check, so a statement headed by `rescue` and continued
on `.` lines that would fit in eighty characters reports `error[syntax]: kanso
has no `rescue``. Lengthen the first line past the fold and the same program
runs:

    rescue (os/read_file! "the-orders-file-that-is-not-there.txt") orders
      . shout
      . banner
      . print

    *** no orders yet!! ***

Remove `rescue` from that arm of the table; `try`, `catch` and `except` stay.
The error corpus has no fixture for a worded step at a statement head, which is
why the message survived the word's arrival.

**One thing the check confirmed rather than broke**, recorded because it is
the sharpest demonstration of the own-err rule found so far and belongs in the
book's boundary chapter when that is written. Annotating a foreign failure
makes it yours:

    rescue (annotate (os/read_file! "no-such-file.txt") said) orders . print
      error[endpoint]: unhandled err reached the executor: "orders file: ..."
        born in the entry at s2.kso:13
        passed through orders

The raw read error was foreign and `orders`' `(err _)` arm caught it a line
earlier. `annotate` raised a new err in the entry's own module, so the same arm
is now walked past. That is the rule working exactly as ruled, and it is the
example to teach it with.
The two rises read 0.28 -> 0.74 with the score 88.28 -> 62.67, and 0.74 ->
1.00 with 64.95 -> 56.73, measured against origin/perf-history rescored by
this tree — which is what CI writes on the next push to main.

## 2026-09-09 — a number is written into the accumulator, and an arm is told what the switch decided

**DONE.** `append acc "{n}"` is how the JSON encoder writes every number, and
the template rendered `n` into a string for the one purpose of copying its
bytes into `acc` and dropping it: 379,530 strings a runbench, built to be
copied once. The render alone was 5.50% of the run program's instructions
(`k_b_render_value` inclusive, 130.1M), and the ryū and itoa digit cores
under it are about 111M of that and stay — what goes is the string around
them, the dispatch that reached it and the copy out of it.

Two changes, and the first does nothing without the second.

**The emitter fuses the pair.** `append acc "{x}"` with `x` proved a number
emits one call to `k_b_append_rendered`, which writes the digits into a
64-byte stack buffer through `k_render_number` — the int and float arms of
`k_render_at`, extracted so the render and the door share one writer — and
copies them into the accumulator from there. Where the accumulator has 24
bytes to spare the copy is three words whatever the length (a number is at
most 24 bytes, `-1.7976931348623157e308`, and the buffer holds 64), so the
call into memcpy for a handful of digits is not made. A value the ambient
`render/to_string` group could claim keeps the dispatch the template would
have made, so a user arm is never skipped, and every other tag the door
hands to `k_render` itself, so the bytes cannot differ from the unfused
spelling; the differential goldens hold that across all three engines.

**An arm is told what the switch decided.** The first build of the fusion
emitted zero fused sites on runbench. `encode_onto`'s `n:int` arm is reached
through the tag switch (§ the tag switch, 2026-09-05), which has already
proved the tag is 0 — and inside the arm the discriminator still carried the
whole group's set, REC and DESC and THUNK included. So the body forced `n`
again, asked whether a user `to_string` arm could claim it, and took the
generic door on every builtin it handed `n` to. `arm_tags_set` now records
the case's tags as the discriminator's set for the arm's body — INT for the
int arm, FLOAT for the float arm, REC for a record arm, the nullary's tag for
its arm — and restores the whole set after. Both number arms fuse, and the
`k_force_fast` in front of each is gone.

On the container: runbench **2,368,295,010 -> 2,347,625,055** (−20,669,955, −0.8728%),
the same bytes out, `append_rendered` 379,530, and `append_fast`,
`append_grow`, `ryu_renders` and every allocation and peak counter identical.
The two mutations measured alone say which half is which: the fusion off
with the arms narrowed reads 2,366,863,311 (−1,431,699, the narrowing's
own gain on the rest of the program: the forces and dispatches it removes
from every switch arm), and the arms un-narrowed with the fusion on reads
2,368,105,221 (−189,789) — with the whole set inside the arm the fusion is
never eligible and every site takes the fallback, which is the byte twin the
generic path always took. Neither half is the win; the pair is, because the
first cannot fire until the second tells it the tag.

**Two wrong cuts on the way, both in the profile.** The first build read
+0.15%: `k_render_number` and `k_b_append_range` were both out of line, so
the fused door paid two calls where the old path paid one, and
`k_b_append_range` alone rose 3.2M -> 22.2M. Both inline now. The second was
the fallback arm — the door the fusion takes when the value is not a number
or a user arm could claim it — which called `k_b_append_mut` directly and
paid 22,789,710 in `k_b_append_wide` on runbench, where the generic path
would have reached the byte twin, whose string arm copies inline. The
fallback takes the twin. Neither mutation could be read until that was
fixed: both mutants measured +0.9% over main, the same 21M, for a reason
that had nothing to do with what they mutate.

**Rows and pricing.** `append_rendered` joins the trend gate's higher-is-
better list beside `append_fast`. Ratchet rows `append_rendered` and
`arm_narrowed`, mutations `a_scalar_rendered_into_a_string_and_copied_again`
(the fusion predicate reads false) and `an_arm_not_told_what_the_switch_
decided` (the narrowing filters to nothing), both on the instructions gate.

**What #456 found, recorded here because this is its first consequence.**
Read against the objective's own marginals, the queue since 2026-09-08 had
been working the dimension holding a tenth of the headroom. A ten per cent
improvement is worth 0.76 points on run speed, 0.64 on run memory, 0.19 on
compile speed and 0.19 on compile memory; the remaining points are 18.34,
9.70, 3.46 and 2.20. The last 26 merges moved welfare 0.1115 in total, and
25 of them were compile-side. The run program's profile puts 23.2% of its
instructions in two loops, `encode_onto` (13.38%) and `value_for` (9.86%),
and this entry is the first of what that map says to do.

**The round CI returned, and the sweep defect under it.** Three jobs went
red on the first run. The cost goldens read `46a47 > sh_bytes=...` on all
twelve: main had gained `sh_bytes` between the branch and the merge, the
merge kept this branch's goldens, and `all_counters.sh --write` then reported
`rewrote` twelve times and changed no file. Its rewrite replaced data rows
line for line and stopped at the golden's last row, so a counter the runtime
had just gained — one more row than the golden held — was dropped, silently.
The rewrite is now `scripts/gates/keep_header.sh`, which appends the measured
file's remaining rows, and `tests/the_sweep_write_keeps_a_new_row.rs` runs it
on a postcard-sized golden and was watched red on exactly that case. The
book's two counters panels drifted by the `append_rendered` row and are
rewritten. The diagnostic scan found `k_render_number: not a number` with no
golden; it is the render's `default:` arm and neither caller can reach it,
so it is listed in `tests/golden/unpinned_diagnostics.txt` with the control-
flow argument. CI's rows: runbench 2,392,210,251 -> 2,369,642,706 (−0.9434%),
encodebench −1.9672%, livebench −2.7667%, oneshot −1.1334%, widebench
−2.0173%, jsonbench −0.0192%. The three compile rows rose, layout moves on
a compiler whose emitter grew: `compile_instructions` 48,746,192 ->
48,749,059, `entry_instructions` 162,042,653 -> 162,051,772,
`library_instructions` 162,839,321 -> 162,846,242. The machine code grew
with it, `text` 1,487,564 -> 1,494,508 summed over the fourteen binaries,
6,944 bytes for the fused door and its fallback twin. Welfare 66.30 ->
66.37, banked.

## 2026-09-09 — read_file is text and read_bytes is bytes, on every engine

**Built:** the 2026-08-29 ruling (archive, "gavel: read_file is text,
read_bytes is bytes, per precedent"), after eleven days on the unbuilt list.
`read_file` refuses a file whose bytes are not utf-8, with one sentence on
all three engines: `cannot read {path}: the bytes are not text`. Until now
native handed the bytes through as a string and the interpreter refused them
with its own words, so the same program answered differently by engine.
`read_bytes` is the other reader: it hands the bytes back as they are, and
takes the same two doors as `read_file` — `read_bytes` answers
`file_not_found` as data and `read_bytes!` insists. `write_file` and
`net_write` accept bytes, so a program can read a binary and write it back
or serve it.

**Where the bytes go.** The http library's `rendered` interpolated the body
into one string, which renders a bytes body as a list. The status line and
headers are now a preamble, `delivered` writes a string body in one write
and a bytes body in two, and the content-length is measured on `as_bytes`,
which is a text body's utf-8 or a bytes body itself. The browser differential
and the fingerprint script read `docs/kanso.wasm` through `read_bytes!`
rather than through a text read that only worked because nothing checked.

**Fixtures.** `tests/golden/micro/a_file_that_is_not_text.kso` reads a
five-byte file — `ff fe 00 41 80`, the first byte one no utf-8 sequence
begins with — both ways and prints the refusal and the five bytes; the
harness runs it on both engines and through a release build.
`tests/golden/runtime/read_bytes_takes_a_path_string.kso` pins the builtin's
own refusal of a non-string path, reached through a binding the check cannot
see through. The 63-entry arity table, the 59-name builtin list and the
56-call codegen table each grew by one, and the descriptor tag is 30.

**Two things found on the way.** A definition added to `std/os` above
`insisted` moved the line four runtime goldens quote (`os.kso:113`), so the
new readers sit below it. And a helper named `head` in `lib/net/http`
collided with four parameters of that name; the check refuses the shadowing,
which is the right answer, and the helper is `preamble`.

**The emitted code moved, and the sweep saw it.** `all_compile.sh` read
`emitted_code` MOVED on eight programs, every one that imports `std/os`: the
decoder itself in `bench/emitted_golden.txt` (defines 140 -> 142, calls
1,232 -> 1,236, lines 9,219 -> 9,245) and seven of the thirteen in
`bench/emitted_golden_others.txt` — encodebench, oneshot, widebench,
digestbench, readbench, livebench and runbench — each up two defines, four
to six calls and 26 to 28 lines, with every branch count identical. The two
defines are `read_bytes` and `read_bytes!`, which a program importing the
module carries whether or not it calls them; none of the eight does. A rise
on this vein is a regression to explain, and this is the explanation: the
library grew two definitions and the emitter carries a module's every
definition, as it did when lib/json dropped std/list and the veins halved.
Both goldens are regenerated with their headers kept. The first reading of
the sweep's output saw only the second file's diff and wrote that the
decoder had not moved; the re-run after regenerating that file said
otherwise, which is what the re-run is for.

**Rulings weighed.** STATUS.md's "Ruled, unbuilt" list (kanso#1353) holds
twelve rows. This is the smallest that touches every engine, and the one a
reader hits first: the book's boundary chapter cannot describe two readers
until both exist, and `read_file` handing bytes through on native while the
interpreter refused them was a divergence the differential law forbids. The
other eleven — effects as types, err readers, the fused chain operators,
pure fallibility boxed, `done`, exhaustiveness without the flag, a
qualified name as its module's declaration, records printing qualified,
the backends' partial over a value, block-born as the whole cohort, and the
boundary chapter — stay in the order the list gives them; the next build is
taken from it.

**Round three: the counter the local sweep never ran.** CI's cost-goldens
job, once the runner's apt mirror stopped returning a mismatched index, read
`utf8_bytes` up on eight runtime veins by the size of the input each reads:
188,698 on the decoder, encode, oneshot, wide, read, live and run programs,
8,192 on the digest. That is the ruling's own cost made visible. `read_file`
validates every byte it hands back now, where native used to hand bytes
through unread, and the counter that counts validated bytes counts the input
file. The allocation counters beside it are byte-identical. The twelve cost
goldens were regenerated with `all_counters.sh --write` on this branch, which
the first two rounds skipped — the emitted sweep ran and the counter sweep did
not, and CI found the difference. The retired-instruction rows CI measured are
copied in: runbench 2,369,642,706 -> 2,369,917,628 (+274,922, +0.0116%), the
decoder +274,897, encode +274,968, oneshot +274,897, read +274,804, live
+274,897, wide +90,524, digest +6,153; basket, deepbench, escapebench,
pendbench, indexbench and scanbench read no file and hold to the instruction.
That is 1.46 instructions a byte for the validator, the word-at-a-time arm the
2026-09-07 entry measured. Every `.text` row rose too, 1,040 bytes on the
eight readers and about 3,056 on the six that do not read, since the reader
pair and the refusal are runtime code every program links. The compile rows:
compile_instructions 48,749,059 -> 48,751,741, entry 162,051,772 ->
162,061,812, library 162,846,242 -> 162,857,425, each the two definitions
`std/os` gained. Welfare reads 66.3705 against a floor of 66.3715, a fall
of 0.0009 that the gate's band holds; the floor is re-set to it with
`--set` all the same, because the trend gate's pure-regression rule reads
this branch as worse on twenty-six counters and better on none, and lets
that through only when welfare_floor.json's history names the change that
spent it. It is the differential-law exception welfare.kso states: a
`read_file` that answers the same on three engines is not a trade.

**Priced, row by row, for the trend gate.** The counters the branch moved and
the values they landed on: `utf8_bytes` 11,164,198, `run_utf8_bytes`
24,415,348, `encode_utf8_bytes` 75,741,068, `live_utf8_bytes` 75,741,068,
`oneshot_utf8_bytes` 450,566, `wide_utf8_bytes` 289,183, `read_utf8_bytes`
188,698, `digest_utf8_bytes` 8,192; `work_jsonbench` 1,485,334,799,
`work_encodebench` 3,963,988,526, `work_oneshot` 21,737,118, `work_widebench`
35,332,240, `work_digestbench` 10,426,549, `work_readbench` 4,561,941,
`work_livebench` 3,481,899,703, `work_runbench` 2,369,917,628;
`emitted_defines` 142, `emitted_calls` 1,236, `emitted_lines` 9,245,
`emitted_other_defines` 2,353, `emitted_other_calls` 20,516,
`emitted_other_lines` 133,241; `text` 1,523,196; `compile_instructions`
48,751,741, `entry_instructions` 162,061,812, `library_instructions`
162,857,425. Each is the validator, the two readers or the refusal, and the
paragraph above says which.

## 2026-09-09 — an err answers `.reason`, `.cause` and `.origin`, on every engine

The 2026-08-29 gavel "an err has readers" (archive), built. STATUS.md had
carried it as unbuilt since the sitting: `annotate e (err -> "config:
{err.reason}")` — the gavels' own sample — was refused at check time with
`no record type has a field reason`, and a callback holding an err could
look at nothing inside it.

**Built.** A field read desugars to a getter call, `Get_reason e`, as every
field read does, so the reader lives where a getter is entered. Each engine's
dispatcher answers an err at its ENTRY, before any arm is tried: the
interpreter in `dispatch_loop_inner`, native in the prologue `emit_reader_hole`
writes for both dispatcher shapes (`k_is_err` then `k_err_read`), the page
with `rt_err_read` in `emit_dispatcher`. `reason` is the value the err was
raised with; `cause` the err it wrapped, or none; `origin` the "{fn} at
{file}:{line}" it was born at, or none for an executor-born one. The
interpreter's `err_read` is the oracle and the wasm host calls it; native's
`k_err_read` mirrors it arm for arm. The entry is the only place that works:
a placeholder arm `(err Read)` matched a foreign err before the failure
pass-through and would have answered the reason to `.cause`, and an
own-hako err is one no arm may see, where a reader has to see both.

**The group has to exist.** A read of a field no record declares resolves to
no getter at all, so `desugar_field_reads` now synthesises one arm per reader
field nobody declares. Synthesised per module, as the record getters are, it
produced one identical arm in every module and the merge refused the overlap;
it runs once over the merged program instead. The checker's declared-field
set gains the three names so the read passes the `no record type has a field`
fence, and a record reaching a reader group still gets the field error every
getter gives.

**Fixture.** `tests/golden/micro/an_err_has_readers.kso`, settled failures
only, so the page runs it too: a plain err's reason, an annotated err's reason
and its cause's reason, `.cause` of an unwrapped err (`<none>`), a cause read
through a second `rescue`, the origin's function name on both, and json's
decode failure read as `e.reason.reason` — the case where a reader and a
record field share a name. Watched red on the checker first. The origin names
the raising function qualified when the program is imported, so the fixture
carries an `.imported.out` twin like the record-printing ones. Two things the
fixture taught while it was being written: `print none` writes `<none>`, and a
named group handed the err passes it through as ever, so the readers are
applied inside the lambda and the group gets the piece.

**Veins, CI's rows.** The readers sit at every dispatcher's entry and the
checker's field set grew by three names, so the front end carries them:
`compile_allocs` 29,606 -> 29,714 (+108), `compile_instructions` 48,751,741 ->
48,820,126 (+68,385, +0.14%), `entry_instructions` 162,061,812 -> 162,734,847
(+673,035, +0.42%), `library_instructions` 162,857,425 -> 163,022,357
(+164,932, +0.10%). The compile peak (773,818), every runtime vein, the
emitted, text and machine-code rows are byte-identical. Priced, row by row,
for the trend gate: `compile_allocs` 29,714, `compile_instructions`
48,820,126, `entry_instructions` 162,734,847, `library_instructions`
163,022,357. Welfare reads 66.36 against the 66.37 floor, a fall of 0.01 the
readers pay in compile cost with nothing offsetting it; the 2026-08-25 ruling's
language clause applies, so the floor moves down to the reading, 66.3596,
by hand in `bench/welfare_floor.json` with its history entry (`--set` refuses
a fall of this size by design, and says the file is the door), and no
optimisation rides along to hide the price.

---

## 2026-09-10 — gavel: rows 15..390 stay unscored, and re-measurement is a lead with a probe in front of it

On "The reconstruction the 2026-09-07 ruling ordered has two usable phases,
not four, for 376 of the 439 rows", Clay: "I'm okay being pragmatic and
taking the first one." Ruled: option (1). Rows 15..390 carry no run terms and
get none reconstructed; they stay scored on the compile terms they carry, at
the coverage the rescore already stamps on them, and the chart keeps drawing
the coverage boundary at 2026-09-03 the way kanso#1346 draws every boundary.
The eight-phase half (rows 391..438) is built and stays. Nothing further is
owed on this entry; it leaves the ledger with this ruling.

Clay's second sentence is a question, and it is answered here so it is not
re-derived: "isn't it pretty trivial to just rerun the current metrics on the
old versions?" Mechanical, not trivial, and one unknown decides whether it is
possible at all. The mechanics: for each of 376 commits, `cargo build
--release` at that commit, `kanso build bench/runbench` with the compiler it
produced, one callgrind run for `run_instructions` and one counters run for
`run_peak_bytes`, on ONE host in ONE sitting that also re-measures HEAD, so
every row is on the same ruler by construction. About four minutes a commit
with a warm cargo cache, so a day of serial machine time for all 376, or an
afternoon sampling every fourth. The unknown: whether a compiler from
2026-08-10 accepts today's bench/runbench source. The surface moved between
then and now — the bang choosing the channel, `done`, the consolidated run
program itself — and nobody has tried. So the lead is filed with its probe
first: build the compiler at row 15's commit, compile today's runbench with
it, and read the answer. Yes means the sweep is a script and the rows come
back as `run_source: remeasured`. No means a runbench pinned to the old
surface, which measures a different program and would need its own ruling.
This is a lead, not a ruling: it goes on cloud's list as a lead and stays
off the "Ruled, unbuilt" section unless Clay says the word.

## 2026-09-09 — block-born is a proof, not a spelling

Built: the 2026-08-29 ruling "block-born is the whole cohort" (STATUS.md,
now removed; the archive entry of that name, Clay: "okay whole cohort it
is"). A field write's target had to be a name bound directly to a
construction in the same `build`; `twin = ada` followed by `twin.partner =
bob` was refused with the syntactic fence, and the 2026-07-28 measurement
found the same refusal on a node chosen by an `if`, a node taken out of a
list the block built, and a node reached through a field. The theorem never
asked for the fence. It asks that the cohort be closed, and every one of
those four values is inside it.

**What the checker proves now.** `check_build_blocks` used to carry a set
of names; it carries a cohort. Each value the walk proves born is an entry,
and a name, a field or an element holds an entry. A construction is born,
and its entry records which of its fields hold born values, read off the
constructor's arguments by position. A name bound to a born name shares the
entry, which is what makes an alias an alias: a write through `twin` is a
write to `ada`'s entry, and `ada.partner` read afterwards is the value that
was written. A list or map literal is born, and its elements share what
every element has; an empty literal shares nothing. An index of a born
literal is that shared entry, a field of a born value is the field's entry,
and a constructor pattern hands each position the matching field. An `if`
whose arms are both born is the meet of the two: a field or an element read
through it is read through to both arms and is born only when both answer,
so a later write to either arm is seen. A write made through the choice
reached whichever arm was taken, so both arms forget the field and the
choice remembers the write. That last rule is the one a reader is likely
to question, and `build_write_a_field_an_if_may_have_overwritten` pins it:
`other = cell "other" young`, then `chosen = if … other young` and
`chosen.link = old`, and `other.link` is no longer proved, because at run
time it may hold `old`.

A write inside an `if` arm's statement list may not have happened, so it
takes the field's proof away rather than supplying one. Anything a call
answers is not proved, a parameter is not, a name from an enclosing block
or an earlier iteration is not, and the six escape fixtures from July stand
unchanged beside four new ones: an `if` with an older arm, an element
beside an older one, a field a constructor filled with an older value, and
the overwritten field above.

**Two spellings of a field read.** The module route rewrites `b.up` to its
getter, `Get_up b`, before the check runs, and the play route after it, so
the walk reads both: `kanso check` on the fixture said ok while the same
program imported was refused at `over.id = 10`, and the getter arm is why
it is not.

**The write form is unchanged.** `target.field = value` with a name on the
left is still the one form, per the 2026-07-19 design; what widened is how
the name's birthday is proven. `ring[2]!.id = 20` does not parse and does
not need to: `middle = ring[2]!` then `middle.id = 20` is the same write.

**What it does not reach, and what that leaves.** Every algorithm the
2026-07-28 entry named — union-find's path compression, an e-graph's
rewire, unification binding the variable it found — reaches its node
through a call: `find` is a recursive function and its node is a parameter.
The four flows admit the shapes the entry measured and not the algorithms,
and design/memory-frontier-research.md's 4.4 row says so. Birth flowing
through a call is the next widening of this analysis, mine, and it is not a
new question.

**Spec.** `tests/golden/micro/a_build_writes_what_it_can_prove_was_born`
runs on all three engines: the alias, the field a constructor filled, the
field a write set, the indexed element and the chosen node, each written
through, then printed. Refused by the old compiler at the first of them.
The four escape fixtures are in the error corpus with their imported twins.
The diagnostic scan reads 313, none newly unpinned; the message is the one
July wrote, since a parameter is still not a construction made in the
block.

**Veins.** CI's sitting on the host-keyed compile rows, copied in:
`compile_instructions` 48,820,126 -> 48,820,567 (+441, +0.0009%),
`entry_instructions` 162,734,847 -> 162,736,962 (+2,115, +0.0013%),
`library_instructions` 163,022,357 -> 163,024,919 (+2,562, +0.0016%). The
cohort walk is a few hundred instructions more than the set walk on every
build block the corpus compiles, and the three routes carry the same
checker. `compile_allocs` and `compile_peak_bytes` did not move; nothing on
the runtime side did. Welfare reads 66.3596 either side, a fall of 0.00002
that the trend gate reads as a pure regression, so the fall is recorded in
bench/welfare_floor.json's history under the 2026-08-25 language clause,
the floor 66.35962 -> 66.35960.

## 2026-09-09 — a qualified name is its module's declaration

The 2026-08-29 ruling of that name (archive), built. STATUS.md carried it as
unbuilt; the log's 2026-09-08 entry "the reorder refuses a valid program"
declined it twice for want of the mechanism, and module_differential carried
the shape as its one `known_wrong` entry.

**What was wrong.** A module that declares `pub fn join` while importing
`std/text` holds two kinds of declaration under the bare name: its own arms and
the bare-enrollment twin of text's `join`. The loader qualified both to
`dep/join`, first writer won, and the first writer was the twin whenever the
import loaded first — so the module's own `pub` read as private from outside,
and the compiler's answer to that was an opacity refusal, "an import of `dep`
exports `join` too and took the name". With the refusal lifted the hazard was
worse than the refusal: a consumer's `dep/join ["x" "y"] "-"` reached std's arm
under dep's name and answered `x-y`, while dep's own arm could never be reached
at all.

**What the ruling says.** `dep/join` is dep's own `join`, and only that. Inside
dep, a bare `join` still dispatches over dep's own arms and the import's
together, because that is what an import is for.

**How.** The bare overload space of a mixed name moves to a spelling no
consumer can write: `dep/~join`, `ast::bare_space`, the mark being `~`, a
character the lexer never makes part of a name. In `qualify`: the module's own
bare declarations and its imports' twins are collected, `mixed` is their
intersection, and `owned` becomes a map from a bare name to the spelling its
call sites are rewritten into — `dep/~join` for a mixed name, `dep/join` for
every other. A twin in a mixed group takes the bare-space spelling, loses
`pub`, and holds no claim on the qualified one; the module's own arm keeps
`dep/join` and gets a synthetic non-pub clone in the bare space, so an
inside call finds both. The two-claims bookkeeping (`shadowed`, the
"took the name" message, the `Some(false), false` arm) goes, and `Loaded`
loses its third element. `Diagnostic::new` runs every message through
`ast::spoken`, which strips the mark, so a sentence about the bare-space
group says `dep/join`, which is what the author wrote.

**Spec, watched red on the pre-ruling compiler.** module_differential's
`known_wrong` is empty for the first time; its one entry is now case c30, a
module's own pub sharing a name with a dependency's, printing `OWN(x,y)` on
both engines. Two cases beside it: c31, a list handed to `dep/join` where dep's
arm takes an int, refused by the check as `no arm of `dep/join` takes a list
here (arms take int)` rather than dispatched into the import; and c32, a bare
call inside the module reaching the import's arm (`x-y`) beside the qualified
call reaching dep's own (`OWN(1,2)`). On the pre-ruling compiler all three fail
with the opacity refusal, 32 modules 3 wrong. The same hazard is pinned byte
for byte in `tests/golden/errors_module/a_list_handed_to_a_modules_own_int_arm`,
two diagnostics at columns 17 and 27, where the old compiler wrote the opacity
refusal at column 8. The page's §12 paragraph, which described the "took the
name" diagnostic as the settled answer, now describes the ruling.

**Verified on the container.** clippy, rustfmt, the errors_module corpus, the
golden suite, reexports, the wasm engine walk on a fresh blob, the unit tests,
the diagnostic scan (313 literal diagnostics either side, 0 newly unpinned), `all_counters.sh`
(the twelve cost veins and the lazy tier agree), `all_pages.sh` (three gates
agree; the §12 paragraph is the page edit), module_differential 36 modules 0
wrong.

**Veins.** `emitted_code` moved on two of the thirteen others, scanbench
19,752 -> 19,764 and runbench 34,806 -> 34,828 lines (the summed key
`emitted_other_lines` 133,241 -> 133,275). Against round one's tree, built
on this box for the comparison, the difference is dead text: each program
interns the raw name of a bare-space group (`scanbench/~split`, `split/~split`,
`runbench/~total`) beside the spoken form, and nothing in the IR names the
constant; runbench also carries a second copy of the capture-free thunk for
pend's `cards -> spent cards`, the lambda in `pend/total`, whose twin sits in
runbench's `~total` group, and neither copy is named either. `emitted_other_defines`
2,353 -> 2,354 and `emitted_other_calls` 20,516 -> 20,517 are that thunk.
Defines, calls and branches hold on scanbench. For the trend gate, the landed
host rows, CI's sitting on round two's commit: `compile_instructions`
48,820,567 -> 48,849,164 (+28,597, +0.0586%), `entry_instructions`
162,736,962 -> 162,730,512 (−6,450, −0.0040%), `library_instructions`
163,024,919 -> 163,431,666 (+406,747, +0.2495%). Round one's sitting read
48,784,668, 162,434,379 and 163,223,647; the reorder and the file test cost
the difference. The lines are the string table: `std/regexp`
declares private `first`, `spread` and `repeat` while importing `std/list`,
which exports all three, so those are mixed groups now, and the own-only
`regexp/spread` group's name and dispatch sentence are interned beside the
bare-space group's. Before the scrub the same table carried `regexp/~first`
and `no overload of `regexp/~first` matches these arguments`, which is how
the mark was found to reach a message a program can print. The decoder's
golden did not move. The host-keyed compile rows are CI's: `qualify` walks
one more set per module and clones an arm per mixed name, and the same-box
pair below is the container's projection, not the row.

Same-box pair, callgrind on the gate's boxes, #461's tree against this one:
module 50,441,411 -> 50,397,314 (−44,097, −0.0874%), entry 167,441,542 ->
167,533,649 (+92,107, +0.0550%), library 168,198,150 -> 168,348,197
(+150,047, +0.0892%); `compile_allocs` 29,714 -> 29,262 (−452: −465 for the
two-claims bookkeeping and the `shadowed` set gone, +13 for the vectors the
reorder builds), `compile_alloc_bytes`
4,806,203 -> 4,826,630 (+20,427, the clones), `compile_peak_bytes` 777,308
identical, rounds 66 and visits 22,133 identical. The allocation fall is
worth about 0.03 on the index by the per-term arithmetic in the 2026-09-09
read_bytes entry and the instruction move is inside the band, so the
projection is a small rise, to be banked with `--set` once CI's rows are in.

**Round one's kq failure, and two fixes on the bare space.** CI's kq job died
in the scale gate with `` `-` is not defined for these values ``: a bare call
inside a module that declares an arm over a name one of its imports exports
reached the import's arm. Dispatch tries a group's arms in declaration order,
and before the ruling the module's own arms stood ahead of the twins
`enroll_bare` appends, so a bare call both could take reached the module's
own. The clones were appended after the twins, which flipped that. c34 is the
shape: a module declaring `sum` over `list/sum` and calling `sum` bare from
`twice` printed `12` where the ruling's compiler prints `-12`, watched red.
The clones go in ahead of the twins now, each group of them in front of the
first twin of its name. Putting the two side by side found the second defect:
`check_constants` walks consecutive runs of one name, and a module's own
constant `bytes` beside the twin of `text/bytes` (the fold fixture #1349
added, run as a library by the golden suite) is a run holding an arity-0 arm,
refused as `a constant admits no overloads`. It had passed by accident, the
clone and the twin never adjacent. The check reads the arms the module wrote,
since the bare space is a union nobody declared and the module's constant
already stands alone under `dep/bytes`; c35 pins the shape and is red under
the reorder alone. The reorder found a third, in the golden suite's library
twins of `an_operand_that_varies_still_loops` and `trmc_count`: trmc writes
its loop wrapper as a synthetic arm under the module's own name, and qualify
read every synthetic bare arm as an import's twin, so `weigh` was a mixed
group in a module that imports nothing and the wrapper went to the bare space
as if it were std's. With the twin last, the bare call still reached it; with
the module's own arms ahead, the plain recursion ran first and the stack ran
out on every engine. A twin is now a synthetic bare arm from a file the module
does not own, `own_files` being the files of its non-synthetic bare
declarations; the wrapper keeps its qualified spelling and gets its clone
like any own arm. c36 pins it, a counted recursion inside a module called bare
a million deep, red under the reorder alone. The three compile rows below are
CI's reading of round two's commit, copied in.

**What stays in STATUS.md's "Ruled, unbuilt", and why.** Read whole before
this PR was chosen: block-born landed in #1359; this row, records printing
qualified and the partial over a value are a stack of three, built and
verified in worktrees, landing in that order; `done` and the fused chain
operators follow, with the plain dot untouched. Three rows are built and
cannot open yet. The effect type's two halves (the plain dot as an
application, the `<t>effect` spelling) and the exhaustiveness check on arm
match are one design with the fused operators: each breaks binds or arms in
kq the way #1357 did, kq's respellings live in kq, and kq is outside this
session's reach, so they wait on that respell and their rows stay. The
pure-fallibility rider waits on the ledger's "where the box wraps" ruling.

## 2026-09-09 — records print qualified, everywhere

The 2026-08-29 gavel of that name (archive), built. STATUS.md carried it as
unbuilt: `tests/entry_file.rs` still ignored the test the gavel said would
flip, and it still expected the unqualified form.

**What was wrong.** A record printed its type's spelling as the compiler
held it, and the compiler held two: a module reached through an import has
its declarations qualified (`lane/slow_lane`), and the same file run directly
does not (`slow_lane`). So `slow_lane 7` printed one way from `kanso play
lane.kso` and another from an entry importing `lane`, and the micro and mem
corpora carried `.imported.*` twins for exactly that divergence.

**The ruling.** Go's `main.T`: a record prints its module-qualified name
whatever the entry path, and the root module takes the name an importer
would write for it — a file's stem, a directory's name.

**How.** `ast::Program` gains `root`, set where each compile path returns:
the entry and play paths, the library path and the module path, from the
file's stem or the directory's name. Rendering is the only reader.
`eval::set_root`, called when an interpreter is built over a program,
records the root and the set of bare types the program declares;
`eval::shown_type` qualifies a type in that set and leaves every other
spelling alone, which is how the compiler's own `entry` record — the map
entry, declared by no module — stays `entry` on every engine. The
interpreter's `render` and the page's host (which renders through it) read
that; native gets a second table beside `k_type_name`, `k_type_shown`,
emitted by codegen and read only by `k_render_at`, so the identity the
runtime matches on and the field-error messages are untouched. Diagnostics
are untouched everywhere: a checker sentence names the type the author
wrote.

**Watched red.** Before the root reached the entry path, `kanso play
alone/lane.kso` printed `slow_lane 7` while `kanso run main.kso` printed
`lane/slow_lane 7`, on both engines; with the root threaded, both print
`trouble: [err lane/slow_lane 7] lane/slow_lane 7`. The first cut prefixed
every bare type, and the interpreter and the page printed `maps/entry` where
native printed `entry` — the wasm walk caught it on `examples/maps.kso`; the
set of declared types is the fix.

**Spec.** `tests/entry_file.rs`: `a_record_prints_qualified_whatever_the_entry_path`
replaces the ignored test, runs the fixture's direct twin (`alone/lane.kso`,
bare statements) and its imported form on both engines, and asserts all four
print the same qualified line. The fixture is rewritten to the modern shape:
the old one caught its own err with an arm, which the 2026-08 err rulings
retired, and `pub play` files refuse `kanso play`. Five example goldens
regenerate (`build_blocks`, `field_typesets`, `markers`, `none_is_a_value`,
`record_render`), each a prefix and nothing else; one book sample
(`appb/wrap_err`, an unhandled `config_bad`) and its panel. The micro, mem,
runtime and error corpora did not move: their `pub play` fixtures were
already reached through a staged import, which is where the twins came
from.

**Verified on the container.** rustfmt, clippy, the golden suite (eleven
tests with the twin spec), entry_file, errors_module, reexports, the unit
tests, the wasm engine walk on a fresh blob (twelve), `book_check.sh` (every
sample verified), the diagnostic scan (311 literal diagnostics, 0 newly
unpinned), `all_counters.sh` (the twelve cost veins and the lazy tier agree),
`all_pages.sh` (three gates agree).

**Veins.** `emitted_code` moved on every program by two lines and nothing
else — the decoder's `emitted_lines` 9,245 -> 9,247, the thirteen others'
summed `emitted_other_lines` 133,275 -> 133,301 (+2 each), defines, calls
and branches identical — and `compile_cost`'s five samples by one line
each, `lines` 6,094 -> 6,099, with `module_lines` 5,268 -> 5,269. The lines are `k_type_shown`: a program whose
root declares no bare type prints every record under the identity's
spelling, so the table is an alias of `k_type_name` rather than a second
switch, and the benchmarks are all that shape. The first cut emitted the
switch unconditionally and cost each program a define, a branch and thirty
to sixty lines. Every runtime vein is byte-identical: the render calls one
table where it called the other. The host-keyed compile rows are CI's;
`qualify` is untouched, and the same-box pair against #1360's tree is below.

Same-box pair, callgrind on the gate's boxes, #1360's tree against this one:
module 50,397,314 -> 50,386,179 (−11,135, −0.0221%), entry 167,533,649 ->
167,577,852 (+44,203, +0.0264%), library 168,348,197 -> 168,341,016 (−7,181,
−0.0043%): the root's name and the shown-name table are a few thousand
instructions either way, inside the noise of a rustc relayout.

**The build_cycle fixture is measured through the import now.** Folding
its `.imported.mem` twin leaves `tests/golden/mem/build_cycle.mem` holding
the numbers the staged import always produced, so the trend gate reads a
re-basing rather than a move: `build_cycle_alloc_bytes` 2,576 -> 3,168,
`build_cycle_perm_allocs` 7 -> 9, `build_cycle_sh_str` 1,936 -> 2,528,
`build_cycle_sh_buf` 176 -> 208, `build_cycle_thunk_allocs`,
`build_cycle_thunk_evals` and `build_cycle_thunk_live_exit` 0 -> 1 each,
`build_cycle_carry_dedup` 0 -> 1. The direct route's numbers were the
twin's other half, and the direct route is gone.

**A seventh twin, met on the rebase.** #1356 landed
`tests/golden/micro/an_err_has_readers.imported.out` while this sat in its
worktree: the fixture printed an err's origin with its file stripped, and an
import qualifies the raising function's name, so `boom` read
`an_err_has_readers/boom` through the harness's staged entry. The twin
went the way of the other six. The probe strips the module's slash too,
since what it asks is which function raised the err, and the qualified
spelling by route is the runtime corpus's accepted divergence, kept there
in its `.imported.stderr` twins.

One spec had pinned the old spelling for the root's own records:
`tests/a_type_name_as_a_bare_value.rs` expected a nullary record declared
in the entry file to print bare, `unit`. Under the ruling the root is a
module like any other and its name is the file's, so the line reads
`nullary_native/unit` on native and `nullary_oracle/unit` on the oracle,
one per staged file name; the expectation moved, found when the queue was
stacked on one worktree on 2026-09-10.

**Round one's compile rows, and the got file CI could not show.**
`compile_instructions` 48,849,164 -> 48,866,385 (+17,221, +0.0353%),
`entry_instructions` 162,730,512 -> 162,751,831 (+21,319, +0.0131%),
`library_instructions` 163,431,666 -> 163,473,663 (+41,997, +0.0257%), CI's
sitting on round one, copied in and noted in each golden. Three rows in one
band, on a change whose only per-program work in the front end is the root's
own String: the layout reading the compile goldens' older notes carry.
`compile_allocs` also disagreed, and its value is not in this entry because
nothing could reach it. The gate refuses to measure on a host the golden does
not name — this container is rustc 1.94.1 against the golden's 1.98.1, and it
says in as many words to let CI measure and copy the rows out of the job log.
The job log's own step for that, "the rows as measured here, to copy into the
goldens", named three got files by hand and there are five;
`allocs_got.txt` was one of the two it missed, and the gate's own message sits
fourteen gates and two callgrind dumps from the end, past what the log API's
tail returns. So the step globs `*_got.txt` now. The hand-written list
arrived with the first compile vein in #1214 and has been extended by hand
twice since, at #1330 for the entry row and #1337 for the library row, and
#1337 paid a round for the omission; the glob is what stops a third.
`compile_allocs` stays deliberately red this round, and round two prints the
number this one could only have guessed.

**Round two read the number, and found four refusals beside it.**
`compile_allocs` 29,262 -> 29,276 (+14, +0.0478%): the root's own String and
the set of declared bare types, paid once per compile. That row compared. The
four beside it did not. Rounds two and three landed on a runner whose
toolchain the goldens do not name, and `library_instructions` says so in its
own words -- "the sitting above was counted on a toolchain
bench/library_instructions_golden.txt does not name, so it is not a
reproduction of the recorded build and says nothing about the value". The
tell is arithmetic rather than prose: `compile_instructions` measured
48,866,385 against a golden holding 48,866,385 and still failed, and
`entry_instructions` and `library_instructions` did the same. A gate whose
measured value equals its golden and still fails is refusing, not
disagreeing.

Which gates refuse follows from what each golden names.
`bench/instructions_golden.txt` names glibc; the three compile-instruction
goldens name glibc and rustc; `compile_allocs` and `compile_memory` name
rustc alone. Every glibc-keyed gate refused on this runner and both
rustc-keyed gates compared, which is the split the measured-on machinery is
for. So the work vein did not move: `deepbench` 389,214,251 and `runbench`
2,369,917,611 are that runner's sitting, not a regression, and
`bench/instructions_golden.txt` is left alone.

Round one landed on a runner the goldens do name -- its three compile rows
came back as real comparisons, "counted X against Y", and those are the rows
copied above. The refusal is not this change's and no edit here can clear it:
the remedy the gate names is to re-measure every row on the new image in one
go and update the measured-on lines, which is its own change and not a ruling
this PR should fold in.

`tests/the_job_log_prints_every_got_file.rs` pins the property rather than
the glob: every `*_got.txt` any script under scripts/gates writes must be one
the printing step will cat. Watched red with the hand list restored, where it
names both files that list missed, `allocs_got.txt` and
`compile_libraries_got.txt`. The glob satisfies the property today; what the
spec is for is the next session writing the list out by hand again.

**Round five re-measures the four refusing veins, and it rides here.** The
paragraph above says the re-measure is its own change. That was wrong. The
runner pool has rolled forward, so a branch opened to carry the fix would land
on the new image and refuse in the same four places, and there is no tree that
goes green on both. It rides here.

The image is glibc 2.39-0ubuntu8.9 against the goldens' 2.39-0ubuntu8.8, and
nothing else moved with it: `compile_memory` printed "measured-on rustc=1.98.1;
here rustc=1.98.1" and compared, and the `machine_code` gate compares against
clang=19.1.1 and passed. Every run through 14:21Z compared and every run from
14:32Z has refused, four rounds of this PR straddling the change. main's last
run is 14:15Z and green, so main is green on the record and would be red if it
ran again.

The revision costs almost nothing, and round one is what makes that a
measurement rather than two sittings read side by side. Round one counted this
branch's content on 8.8 and round four counted it on 8.9 -- same commit
content, same clang, same rustc, same benchmark sources -- and the work vein
compared GREEN in round one, so every row sat exactly on its golden with this
change already in the tree. Of the seventeen glibc-keyed values, fifteen are
byte-identical across the pair: `compile_instructions` read 48,866,385 both
times, `entry_instructions` 162,751,831, `library_instructions` 163,473,663,
and twelve of the fourteen work rows did not move at all. The two that moved
are `work_deepbench` 389,214,232 -> 389,214,251, a RISE of 19 (+0.000005%), and
`work_runbench` 2,369,917,628 -> 2,369,917,611, a fall of 17 (-0.000001%). Both
are
the revision and neither is this change.

Both rows are copied and the four measured-on lines take 8.9:
`bench/instructions_golden.txt`, `bench/compile_instructions_golden.txt`,
`bench/entry_instructions_golden.txt` and `bench/library_instructions_golden.txt`.
This is the shape the runbench note in the first of those already recorded for
8.7 -> 8.8, where one row moved 1,014 instructions and the rest did not, and it
is smaller. Two ubuntu revisions have now been measured across and each moved
one or two of the allocator-heavy rows by tens of instructions; what a revision
costs is a property of the revision, and the gate refuses either way, which is
why both of these were read instead of assumed.

Welfare reads 66.38 against the 66.37750307886598 floor and is not re-set. The
17 instructions `work_runbench` gives back are worth about a ten-millionth of a
point, and they are the revision's rather than this change's. Ratcheting them
would pin a glibc package revision into the floor, so a pool that rolled back
would read fractionally under it and redden CI for a reason no pull request
caused. The rule that a rise is banked is for a gain a change earned; this one
is left where it is, said out loud here rather than passed over.


## 2026-09-09 — the backends build the partial over a value

Built: the 2026-08-29 ruling "the backends build the partial over a value"
(STATUS.md, now removed; the archive entry of that name, Clay: "BUILD IT").
`&f 2` where `f` is a parameter, a local or a lambda settles its arity when
the arguments arrive, on every engine. Until now native and the page refused
it as a limit of their own — "`f` is a value here, and a partial over a
value settles its arity when its arguments arrive — this backend fixes it
where the closure is written" — because both lowered a partial to the lambda
it is equivalent to, and a lambda fixes its parameter count where it is
written. tests/partial.rs and tests/wasm_engine.rs each pinned that refusal
as a limit rather than a judgement about the program; both specs are flipped
to agreement with the oracle.

**Native.** A partial over a value is a closure with no body: `arity` is -1,
which marks it, and the environment holds the callee first and the held
arguments after it, `ncaps` counting both, so the copy and tenure walks that
size an environment by `ncaps` carry it unchanged. `k_partial0..4` build one
— `&f a b` evaluates the callee and then each argument and the first failure
among them is the answer, callee first, the interpreter's order in its
App-with-Partial arm; a bare `&f` wraps whatever `f` holds. `k_call0..4`
gain one branch in front of their arity test: a body-less closure gathers
held and fresh, asks the callee's arity (a closure's field, a fnref's field,
anything else answers nothing, which is what the interpreter's `arities_of`
answers for a partial over a partial or a value that is not callable), and
dispatches through `k_callN` when the count is an arm's, grows when it is
short, and dies past every arm with the oracle's sentence, `no 3-argument
arm of `<fn>` (arms take 2)`. `()` on one runs it or says how many it is
short by, `this function takes 1 argument(s), got 0`, zero when it holds
more than the callee takes, as the oracle says it. `k_call_decided` routes
a partial to `k_call1` for the same reason the oracle's decided path reaches
`call`. The emitter's inlined fast arm tests `arity == n` before it calls,
so a partial never reaches a body through it and the hot path is
byte-identical: `all_compile.sh` on the branch reads nothing moved that this
host can see, and `all_counters.sh` agrees to the counter.

**The emitter.** `partial_lambda` no longer refuses a value; the two call
sites that reach it — `Expr::Partial` and an application whose head is one
— ask `declared` first and route a value to `emit_partial_value`, which
emits the callee as a value, the supplied arguments, and one call to
`k_partial{n}`. The flattening of `(&f 2) 5` into one call, which is right
for a declared group because dispatch happens on the total count, is kept
for groups only: over a value the total is the callee's to settle at run
time, so the held arguments build the partial and the rest reach it through
`k_call`, which is where the count is settled.

**The page.** `rt_partial` builds a slot the same shape — `Slot::C` with a
`PARTIAL` arity of -4, the environment holding the callee and the held
arguments — and `call_closure` and `call_decided` route it to
`partial_apply`, the interpreter's `apply_partial` and `run_partial` in one
function. A group or builtin handed out as a value used to carry arity -1,
which told a partial nothing about when it was finished, so the wrapper
now carries the counts its arms take as a mask below `MASKED_ARITY`
(-1000): `arities_of` reads the bits back, a lambda answers its one count,
and a partial or a cell answers nothing. `call_closure`'s own arity test
reads `arity >= 0`, so a masked wrapper still dispatches on the count it is
handed, as it did.

**Spec.** `tests/golden/micro/a_partial_over_a_value.kso` runs on all three
engines: a parameter holding a three-arm group finished in one call and in
two steps, a two-arm group, `()` on a complete partial, a partial rendered
as `<fn>`, and a lambda held in a local as the callee. Two runtime fixtures
pin the two sentences on native and the oracle:
`a_partial_over_a_value_past_every_arm` and `a_partial_over_a_value_run_short`.
tests/partial.rs gains the four run-time shapes on both engines, and the
page's flipped spec runs the program the two of them agree on and reads
`14`. The two refusal sentences leave the diagnostic scan.

**The page's call head, met on the rebase.** The page had declined any call
whose head is not a name, a keyword or a lambda with `unsupported call head`,
which is exactly the shape a partial over a value arrives in: `(foo add) 5
7`, a call whose answer is the callee. The backend now computes any such head
as a value and calls it, the runtime naming what it cannot call, so the two
page specs and the runtime fixture run on the third engine.

**CI's rows, round one.** Five veins moved and the job named all five.

The three compile rows rise: `compile_instructions` 48,866,385 -> 48,879,362
(+12,977, +0.0266%), `entry_instructions` 162,751,831 -> 162,822,303 (+70,472,
+0.0433%), `library_instructions` 163,473,663 -> 163,543,712 (+70,049,
+0.0429%). `src/runtime.c` is `include_str!`'d into the compiler, so
`k_partial0..4` and the branch each of `k_call0..4` gains are bytes the
compiler carries and compiles, and `src/codegen.rs` and `src/wasm_backend.rs`
change beside it. `compile_allocs` 29,276 and `compile_peak_bytes` 773,818 are
byte-identical: the front end does no more work, it only has more text.

The whole `.text` vein rises, and it was not in the prediction this branch was
pushed with. Every one of the fourteen rows gains 2,096 bytes or 2,432 --
`text_jsonbench` 97,986 -> 100,418, `text_encodebench` 118,834 -> 120,930,
`text_oneshot` 109,858 -> 112,290, `text_basket` 112,018 -> 114,226,
`text_widebench` 124,098 -> 126,194, `text_deepbench` 74,306 -> 76,402,
`text_escapebench` 55,442 -> 57,874, `text_pendbench` 90,738 -> 92,834,
`text_indexbench` 59,458 -> 61,890, `text_scanbench` 158,642 -> 160,738,
`text_digestbench` 109,554 -> 111,650, `text_readbench` 55,970 -> 58,402,
`text_livebench` 110,434 -> 112,866, `text_runbench` 245,858 -> 247,954. The
runtime is linked into every program, so a runtime that grows grows all of
them; the 336-byte spread is the arms a program's own dispatch makes
reachable. The PR body predicted the host-keyed compile rows and stopped
there, which was the wrong list: a change to src/runtime.c moves the compile
rows AND the text vein, and nothing on this container could see either.

**And the work vein FALLS, on ten rows of fourteen, with none rising.**
`work_runbench` 2,369,917,611 -> 2,369,679,773 (-237,838, -0.0100%),
`work_deepbench` 389,214,251 -> 387,470,247 (-1,744,004, -0.4481%),
`work_widebench` 35,332,240 -> 35,268,228 (-64,012, -0.1812%), `work_pendbench`
590,971,748 -> 590,970,940 (-808), and a uniform -8 on `work_jsonbench`,
`work_encodebench`, `work_oneshot`, `work_digestbench`, `work_readbench` and
`work_livebench`. `work_basket`, `work_escapebench`, `work_indexbench` and
`work_scanbench` are byte-identical.

That is the opposite of what the change looks like it should do. It adds a
branch in front of every `k_call0..4` arity test, and the naive reading is that
every native call pays one more test. What moved is the emitter's inlined fast
arm: it tests `arity == n` before it calls now rather than after, which is both
what keeps a partial from ever reaching a body through it and a cheaper test
than the one it replaced. deepbench and widebench build the deepest and widest
structures in the corpus and call hardest, and they carry most of the fall. The
uniform -8 is one instruction on a path six programs take eight times and the
four flat rows never take.

The fall is small and it is measured rather than argued: both sittings are
CI's, on the same glibc 2.39-0ubuntu8.9 image, and the only difference between
them is this commit.

Summed, the vein the gate weighs is `text` 1,523,196 -> 1,554,668, a rise of
31,472 (+2.0662%). That is the whole of the runtime's growth counted once per
program across the fourteen. Machine-code size carries no welfare term, ruled
2026-09-05, so it is watched exactly and priced here rather than traded.

**Welfare falls, and the floor moves under the language clause.**
66.37750307886598 -> 66.37657452327063, a spend of 0.00093. The run term
improves and the compile term pays more: `work_runbench` falls 0.010% where
the three compile rows rise 0.039% together, and the smaller relative move on
the heavier, later-satiating term does not cover the larger one on the lighter.
`compile_allocs` and `compile_peak_bytes` are byte-identical, so the front end
does no more work; it has more text to compile.

The floor moves to the reading because a RULED feature spent it, which is the
2026-08-25 clause and the same ground #1359 and #1356 moved on. It is not a
claim that the weights are wrong: the objective is reporting the trade
correctly, and a partial over a value on every engine is worth 0.00093 of it
by Clay's own "BUILD IT". The first cut of this entry recorded the move as a
`--set` with a reason that read like a banked gain, which was wrong about the
direction; the history entry says the fall and its size now.
## 2026-09-09 — a succeeded effect yields `done`

Built: the 2026-08-26 ruling "the July letters close", letter D (STATUS.md's
row "`done` is minted", now removed): a succeeded effect yields `done`, and
`none` means absence and nothing else. Until now a print, a write, a sleep, a
`make_dir`, a `write_file`, a kill or a socket close yielded `none` on every
engine, so a chain that bound the yield could not tell a finished effect
from a missing value, which is the railway-skip the ruling closed.

**The value.** `done` is the fourth nullary beside `true`, `false` and
`none`: a literal, an arm's pattern, an annotation's type word and a typeset
member. It renders `<done>`, the way `none` renders `<none>` and a
description `<io>` — a value that is not data prints in brackets, so a
yield that reaches an interpolation is visible for what it is rather than
passing as a word. It equals itself and nothing else; `done == none` is
false on every engine. Infer carries it as its own bit, `DONE`, above the
thunk bit, and the twelve effects that used to yield the empty set yield
`DONE` now: the empty set was how the decided dispatch chose the catch-all
arm for a yield, and with the bit in place a `done` arm is reached on
native the way the oracle reaches it. Native's tag is `K_DONE`, appended
after `K_SUB` so every existing tag keeps its number; the page spells the
literal `Lit::Done` and answers type code 9 for it; its effects run through
the interpreter's executor, so the page's yields moved with the oracle's.

**Twelve effects on two executors.** The interpreter's `execute` and
native's `k_exec` each answer `done` for print, write, write_err, sleep,
the nil description a sleep settles into, make_dir, write_file,
write_bytes, kill, send, send_bytes and the socket close. A read that finds
no file, an environment variable that is not set and `read_bytes` on a
missing path still answer `none`, since those are absences.

**`done` was already a name, twice.** `std/list` declared `pub type done`
with one field, `drained`, and every construction in the module was `done
true`: a record carrying no information, matched by `(done _)` in eleven
arms and re-answered as `done d`. design/enumerable.md §8 had spelled the
iterator's end bare, `next src . (done -> acc)`, all along. So the record
is gone and `next` answers the nullary when a source is drained; the
arms read `done` and the constructions are the word. `std/net/http`'s
`done value` carries the value a serve loop stops with, which is a
different thing, so it is `stopped value` and `http/stop` builds it, with
both callers in scripts/ unchanged since they call `http/stop`.
`std/regexp` used `done` as the name of a repetition count in one arm
family, twenty-five sites; it is `reps`. examples/next_protocol.kso reads
`fn drain done acc`. Anything outside this repository that matched
`(done _)` on a list iterator reads `done` now, and a binding named `done`
is refused with the sentence every other taken name gets,
`done_is_a_value_not_a_name` in the error corpus.

**Spec.** `tests/golden/micro/a_succeeded_effect_yields_done` on all three
engines: a print's yield and a write's yield bound and printed, `<done>`,
equal to `done`, not `none`, and dispatched to an arm that names `done`
beside the arm that does not. The old compiler refuses the file at the
arm, `unused binding done`, since `done` was a parameter name to it.

**What moved.** The list record was one allocation per drained source, so
three runtime cost goldens fall by exactly the drains their programs make:
basket 28,169 -> 28,166 allocs, pend 4,007,549 -> 4,007,349, run
6,936,567 -> 6,936,517, and four `.mem` fixtures with them
(`a_carried_value_written_into_an_older_node` 100,550 -> 100,149, the
other three the same shape). Eight programs that import std/list emit less
now that eleven arms match a tag instead of destructuring a record:
encodebench 1,638 -> 1,607 calls and 11,197 -> 11,105 lines, basket,
widebench, deepbench, pendbench, scanbench, digestbench and runbench each
31 to 40 calls and 92 to 103 lines fewer, branches down 8 or 9 apiece;
the decoder does not import it and is byte-identical. The module compile
golden reads lines 5,269 -> 5,177, calls 758 -> 727, branches 437 -> 430
and `module_visits` 2,409 -> 2,471, and `compile_memory`'s
`front_end_visits` 22,426 -> 22,562, the page's span rewritten: a nullary
in an arm is a cheaper emit and a
slightly longer inference, since `DONE` is a bit the fixpoint carries
where a record was a type it did not. **Re-measured on the base this opens against.** The rows above were first
counted on main 3c7a163b and this change opens against dc980ddf, three merges
later, so `all_compile.sh` was re-run and the two veins it can see were
regenerated here: `emitted_code` on the eight programs and `compile_cost`'s
module row. The deltas are dn's own and did not move -- 31 to 40 calls and 92
to 103 lines fewer, visits 2,409 -> 2,471 -- but two LEVELS did, by the two
lines a program kanso#1361's alias added: encodebench's lines read 11,197 ->
11,105 where the first sitting read 11,195 -> 11,103, and the module row
5,269 -> 5,177 where it read 5,268 -> 5,176. `front_end_visits` 22,426 ->
22,562 is measured rather than projected: `compile_memory` checks rounds and
visits before it asks the host anything, because those count the compiler's
own algorithm and are the same everywhere, and only its `compile_peak_bytes`
row refused.

**CI's sitting, and a prediction that was six-sevenths right.** Round one named
SEVEN veins and this entry had named six of them: the four host-keyed compile
rows, `compile_memory`'s peak, and `machine_code`. The seventh was `work`, the
runtime instruction vein, and the miss is instructive because the entry's own
record implies it: a change that removes one allocation per drained source
removes the instructions that allocation cost. The seventh name written above
was the `text` vein, which is not a seventh gate at all -- `machine_code.sh`
diffs bench/text_golden.txt, so the two are one. Six gates predicted, seven
named, and the arithmetic was wrong in both directions at once.

Every one of the thirty-nine moved keys, by name:

    work           encodebench 3,963,988,518 -> 3,963,988,451        -67
                   basket         34,694,178 ->     34,690,245     -3,933  -0.0113%
                   widebench      35,268,228 ->     35,268,161        -67
                   deepbench     387,470,247 ->    387,470,234        -13
                   pendbench     590,970,940 ->    583,758,224 -7,212,716  -1.2205%
                   scanbench     726,019,157 ->    726,019,079        -78
                   digestbench    10,426,541 ->     10,426,515        -26
                   runbench    2,369,679,773 ->  2,367,877,484 -1,802,289  -0.0761%

    text           encodebench       120,930 ->        120,098       -832
                   basket            114,226 ->        113,170     -1,056
                   widebench         126,194 ->        125,362       -832
                   deepbench          76,402 ->         75,538       -864
                   pendbench          92,834 ->         91,346     -1,488
                   scanbench         160,738 ->        159,842       -896
                   digestbench       111,650 ->        110,530     -1,120
                   runbench          247,954 ->        246,402     -1,552
                   jsonbench         100,418 ->        100,434        +16
                   oneshot           112,290 ->        112,306        +16
                   escapebench        57,874 ->         57,890        +16
                   indexbench         61,890 ->         61,906        +16
                   readbench          58,402 ->         58,418        +16
                   livebench         112,866 ->        112,882        +16

    compile_allocs                     29,276 ->        29,000       -276  -0.9427%
    compile_instructions           48,879,362 ->    48,572,851   -306,511  -0.6271%
    entry_instructions            162,822,303 ->   161,836,689   -985,614  -0.6053%
    library_instructions          163,543,712 ->   162,541,723 -1,001,989  -0.6127%
    compile_peak_bytes                773,818 ->       769,071     -4,747  -0.6135%
    front_end_visits                   22,426 ->        22,562       +136  +0.6064%

jsonbench, oneshot, escapebench, indexbench, readbench and livebench do not
move on the work vein at all, and they are the six that rise exactly 16 bytes
on the text vein. That is the whole shape of the change in two lines: the +16
is `K_DONE`, the tag appended after `K_SUB` in src/runtime.c, which links into
every program whether or not the program can produce the value; the eight that
fall are the eight that import std/list, where eleven arms match a tag instead
of destructuring a one-field record. pendbench takes 1.22% of the work vein
because it drains one source per pending cell, where the rest drain a handful.

The five compile rows fall in one band -- 0.9427%, 0.6271%, 0.6053%, 0.6127%,
0.6135% -- which reads as work removed rather than as the layout move the
compile-row notes usually record: a declaration the front end used to lex,
parse, infer and check is gone. `front_end_visits` goes the other way by 136,
and that is deliberate: `DONE` is a bit the inference fixpoint carries where a
record was a type it did not, so a nullary in an arm is a cheaper emit and a
slightly longer inference. Only the peak is a welfare term.

Welfare 66.38 -> 66.42, banked with `--set` in this same PR. All five terms
improved: `run_instructions` and `compile_peak_bytes` by the deletions above,
`compile_instructions` (the objective's sum of the module, entry and library
rows) by 2,294,114, and `compile_allocs` by 276.

**A blank line cost a round.** Round two took all seven veins' rows and CI came
back with six green and `compile allocations` still red -- on a golden whose
value matched CI's to the digit. The diff was `1d0` against a blank: the note
above the row had been separated from the note before it by an empty line, and
`compile_allocs.sh` builds its expected file with `grep -v '^#'` and nothing
else, so the blank survived into the comparison. The other four compile goldens
carry blanks today and their gates do not mind. Round three deletes the blank
and writes the trap into that golden's own header, because the habit it broke --
separate notes with an empty line -- is right in every other file here.
## 2026-09-09 — the fused chain operators

Built: the 2026-08-31 gavel "the fused chain operators" (archive; STATUS.md's
row, now removed). In chain position each of the three words has one
spelling, the chain dot plus one character of channel: `.>` is `bind`, `.!`
is `annotate`, `.?` is `rescue`.

    settled = json/decode text
      .> count_of
      .! (e -> "settled: {e.reason.reason}")
      .? (e -> "fell back on {e.reason}")

**What the parser learned.** The lexer reads `.` pressed against `>`, `!` or
`?` as one token carrying the word it stands for, and lets that token lead a
continuation line the way the bare dot does. `.!` and `.?` desugar to the
ordinary applications `annotate x f` and `rescue x f`, which every engine
already speaks prefix-style (kanso#1116), so a desugared step reaches the
same `k_b_annotate`, `Desc::Rescue` and `rt_rescue` the prefix form reaches
and the emitters, the runtime and the page are untouched. The right-hand
side is one function, a lambda, a name or a group: `.> f x` is refused with
the partial named, since the gavel's common case is the bare function and a
held argument already has a spelling, `&f x`. A `. bind f`, `. annotate f`
or `. rescue f` step is refused with the fused form named. In chain position
that spelling retired, as ruled, superseding the 2026-08-29 keep-the-dot
ruling for the three words. The words remain prefix functions everywhere
else, and every fixture that calls them that way is unchanged.

**`.>` desugars to the piped step, and the measurement that decided it.**
The first cut desugared `x .> f` to the application `bind x f` like the
other two. On the interpreter that is the same program: `bind` on a
description builds `Desc::Bind`, on a settled failure skips the callback,
on a value calls it, which is what the automatic bind does at a piped step
in each case. Native did not agree. The beat keys on a piped lambda — `App
{ piped: true, head: Lambda }` is the shape beat.rs and infer.rs read as a
chain step whose body is the loop — and a lambda handed to `bind` as an
argument is an escaping closure to both, so the arena stopped rewinding:
with the tree respelled (below) the decoder's `beat_iters` read 151 -> 1
and its `arena_peak_bytes` 2,097,152 -> 251,658,240, runbench's 45,944,528
-> 209,522,384, and welfare fell 9.23 points. So `x .> f` desugars to the
piped application, `App { head: f, args: [x], piped: true }`, the node the
automatic bind has always been, and every pass reads it as it did. The
gavel's sentence holds — `x .> f` IS `bind x f` on every input — and the
node that carries it is the one the compiler already optimises. When the
effect type ends the automatic bind, the plain-dot step over a box will
mean something else and this node will need its own flag; that build owns
the flag, and the emitter's `k_b_bind` stays the prefix word's.

**Every automatic bind is spelled `.>`.** With the spelling in, infer's
piped-over-DESC branch was instrumented to print the file and the head
position of every step it bound automatically, and the tree was checked
program by program (every scripts/ directory, hako, bench, the examples,
the book samples and the golden corpora; the play files through `kanso
play`): 519 sites, 372 with a lambda on the right and 147 with a bare name,
none with a held argument. A script rewrote the dot at each head position
to `.>` — 498 dots, since a chain of several steps reports one site per
step — and a second census under the same instrument read zero. That zero
was over the programs the sweeps compile, and it was not the tree. A dot
followed by a lambda or a name is a grep, and compiling every file that
grep names under the same instrument found 44 more dots in 22 files that
no sweep reaches: the benchmark sources `build_benchmarks.sh` builds and
nothing checks (`total 4000 . (t -> io/write "{t}\n")`, the deep, pend
and wide bodies, and the main text `make_jsonbench` writes out), the
book's ch09 `vse` sample, the trace-demo example, the two workahead
programs, and 56 more in the programs thirteen Rust specs under tests/
carry as string literals, which no census of `.kso` files could reach,
and one in the playground's dice example in docs/play.js. Those are
respelled too — 599 dots in 132 files — and the grep census reads zero. The lesson is the one §30 already records: a dynamic census
counts the programs that ran, so the check is the static list of
candidates, each one compiled. Four statements crossed 80 columns and
wrap onto continuation lines; nothing else changed, and no golden moved:
the piped node is the one those goldens were measured on. The plain-dot
steps whose subject infer reads as a value keep their dot — `"kanso" .
greet . print`, `expect 42 . to (equal 42)` — because under the effect
type a plain-dot step over a value stays an application and only the
steps over a box change meaning. The book's panels were regenerated and
the sentences beside them that named the dot as the way into an effect
now name `.>`; ch04 and ch05's teaching of the railway itself waits on the
effect type, as the ledger says. The book's highlighter tokenised `.>` as a
dot and a `>` and lost the name after it, so `scripts/book_panels` reads
the three fused spellings as one operator token and a name in front of one
as a head; the 29 panels that carry one are re-rendered. The panel check
compares a panel's text with its sample, markup stripped, so a highlighter
change moves no panel on its own; four panels in ch03 and appa carry
markup the highlighter cannot produce (an ascription's type, a name after
an operator, a bare name on its own line), which is why the check stays
on the text.

**Spec.** `tests/golden/micro/a_chain_step_names_its_channel.kso` runs on all
three engines, settled failures only: a group as the bare right-hand side of
`.?` and `.>`, two lambdas, a value bound twice, and the chain above on a
success and on a failure. `tests/golden/chainwords/a_fused_chain_over_an_effect.kso`
is the log's `. rescue orders` program in the ruled spelling: a missing file,
`.? orders .> shout .> print`, prints `no orders yet!!` on native and the
oracle. Two error fixtures pin the refusals, `a_chain_word_is_spelled_fused`
(`. rescue told`) and `a_fused_step_takes_one_function` (`.> told 3`). The
compiler page's combinators section says what landed.

**Three libraries changed, so the compile rows are CI's.** `lib/net/net.kso`,
`lib/net/http/http.kso` and `lib/os/os.kso` respell their chain steps, and
`lib/*.kso` is `include_str!`'d into the compiler, so those are bytes the
compiler carries and compiles. The two compile-side veins this host can read
are unmoved -- `emitted_code` on all fourteen programs and `compile_cost`'s
module and modules rows -- which is what a respelling that changes no emitted
code does. The six host-keyed rows (`machine_code`, `compile_memory`,
`compile_allocs`, `compile_instructions`, `entry_instructions`,
`library_instructions`) refuse here and are expected to move on CI, by layout
at minimum; whatever it reports is copied in from its sitting rather than
projected from this container, which takes clang 18.1.3 against CI's 19.1.1.

**Cherry-picked onto 0d7b3e57, not rebased.** The branch sat on `local/done`,
whose content main now carries in squashed form. The pick asked for two
resolutions, both the log's and the archive's both-append hunks;
`lib/net/http/http.kso` auto-merged, because the collision integration had
predicted there was `done`'s `stopped` against this change's `.>` on one line
and `done` is on main now. Re-verified on the picked tree rather than on the
branch's own worktree: that worktree carries `done`'s code against `done`'s
pre-change module golden, since dn's own commit moves the row and the
regeneration happened later, so `compile_cost` is red there and green here.

**CI named THREE of the six, and the three that held say what the change is.**
The prediction above reasoned from the library edit alone -- three `lib/*.kso`
files change, `lib/*.kso` is `include_str!`'d into the compiler, so all six
host-keyed rows are in play -- and that is right about which rows CAN move and
silent about which will. What moved:

    compile_instructions   48,572,851 ->  48,393,437   -179,414  -0.3693%
    entry_instructions    161,836,689 -> 161,314,264   -522,425  -0.3228%
    library_instructions  162,541,723 -> 162,023,537   -518,186  -0.3188%

and `compile_allocs` (29,000), `compile_peak_bytes` (769,071) and the whole
`text` vein are BYTE-IDENTICAL in the job that counted those three. The front
end allocated exactly the same and held exactly the same doing this compile, and
the linker emitted the same machine code, so nothing about the work changed;
what changed is src/lexer.rs and src/parser.rs, and the three instruction rows
are the layout veins CLAUDE.md's prior describes. Three rows falling together in
one narrow band with the allocation row still is the layout signature; real work
removed would have moved allocs with them, the way `done` did an hour earlier.

Welfare 66.42 held and re-set: the objective's compile term sums the three rows,
so it takes the whole 1,220,025, and the rise is under a hundredth of a point.
Five compiler.html spans quoting the three rows were rewritten by
`golden_prose --write`.

## 2026-09-10 — a record built into its own first field

Found while writing a mem fixture for the carry tier: a loop that chains
records, `grow (node n i) (i + 1)`, came out cyclic on native and correct on
the oracle, on main as on the branch. The emitter builds a constructor into
a record the arm has finished with — `shift (n - 1) (point (p.x + 1) p.y)`
reads every field it needs before the constructor runs, so by then `p` is
done with and its storage is free — and `sole_finished_record` judged
finished by counting mentions: the parameter read in this constructor's
arguments and nowhere else in the arm, and handed over by every caller. A
bare mention counted as a read. `node n i` mentions `n` once, in the
constructor, and every caller hands it over; the emitter passed it to
`k_rec_reuse` as the victim, and the runtime overwrote the victim's fields
with arguments the first of which was the victim. The new node's `next`
pointed at its own storage, and `show` walked it until the stack ran out.

**The fix** is in the analysis. Finished means read from, and a field read
is the only read that leaves nothing behind: `field_reads_in` counts the
mentions that are the base of a `var.x`, and a parameter whose mentions in
the arguments are not all field reads is not a victim. A bare mention stores
the record itself, whether directly or inside another constructor, and a
record something is about to hold is not free. The runtime is unchanged;
`k_rec_reuse` still trusts its caller, which is the arrangement the
2026-09-04 attribution (k_rec 30.39% of pendbench) chose.

The first cut of that rule declined every reuse in the tree — basket's
`sh_rec` 130,192 -> 386,192, `record_reuse_shape`'s 4,006 allocations back
where 3 had been — and the reason is worth a line for the next pass written
against this AST. `desugar_field_reads` runs before the linear analysis,
and after it there is no `Expr::Field` anywhere: `p.x` is `Get_x p`, the
getter applied to the record. A count of `Expr::Field` reads zero on every
program. `field_reads_in` reads both spellings now, the getter application
by `getter_field` on the head's name, and the veins agree.

One site in the tree was the defect's shape and never showed it.
lib/json/scan.kso's `fail p reason` builds `err (parse_failure p reason)`,
and `reason` — mentioned once, bare, in the constructor's arguments, handed
over by every caller — was the victim at that site. It is a string at every
call, and `k_rec_reuse` allocates fresh when the victim is not a record of
the constructor's width, so the program was right by the runtime's guard
rather than by the analysis; a caller passing a two-field record as the
reason would have built the failure into its own `reason` field. The site
emits `k_rec` now, which is the one line the emitted veins lose.

**Spec.** `tests/golden/micro/a_loop_that_chains_records_keeps_each_node`
on all three engines, `3>2>1>0>end`; watched red on native (the stack ran
out) with the oracle green before the fix, both green after.

**Cost.** None at run time: `all_counters.sh` reads the twelve cost veins
and the lazy tier agreeing with their goldens, and welfare holds at 66.31.
The emitted veins each lose the `k_rec_reuse` line at lib/json's `fail`,
landed at: emitted_lines 9,135 (from 9,136), and in the others vein
encodebench 11,142, oneshot 9,064, widebench 12,124, pendbench 7,033,
scanbench 19,728, livebench 9,181 and runbench 34,673 lines, one fewer
each; defines, calls and branches hold, because the `k_rec_reuse` declare
that goes is not a define and its call is replaced by a `k_rec` call. The
six host-keyed compile rows are refused on this container and copied from
CI's sitting.

**CI's five host-keyed rows, and what each one says.** The container refuses
five of the veins this change moves, so round one was red on all five and CI's
sitting is what lands. Two go down and three go up.

The runtime rows fall in four of fourteen programs and hold in ten: basket
34,690,245 -> 34,690,216 (-29), pendbench 583,758,224 -> 583,755,724 (-2,500),
scanbench 726,019,079 -> 726,018,879 (-200), runbench 2,367,877,484 ->
2,367,876,664 (-820). Machine code falls in nine and holds in five, jsonbench
100,434 -> 100,130 the largest at -304. NINE against the emitted vein's EIGHT:
basket loses sixteen bytes of text without losing an emitted line, because the
`k_rec_reuse` declare it drops was already text some other program shared.

The three compile rows RISE, together and by nearly the same fraction:
compile_instructions 48,393,437 -> 48,412,144 (+18,707, +0.0387%),
entry_instructions 161,314,264 -> 161,360,451 (+46,187, +0.0286%),
library_instructions 162,023,537 -> 162,069,092 (+45,555, +0.0281%). That is
the price of the answer. `sole_finished_record` used to count mentions, which
costs nothing; `field_reads_in` asks of each mention whether it is the base of
a field read, which walks. Every compile route runs the linear pass, so all
three rows move, and a per-declaration check that moves them by the same
fraction is what a uniform cost looks like.

**The trade, and the objective's verdict.** A rise anywhere is a thing to
state rather than defend, and this one is real: the compiler does 110,449 more
instructions summed across the three routes to buy 3,549 fewer at run time on
the four programs that reach the shape. Stated that way it sounds like a bad
bargain, and by the raw counts it is. The objective disagrees, because the
counts are not what it weighs: welfare reads **66.42 against a floor of 66.42,
held exactly**, since a 0.03% rise on a compile term measured in hundreds of
millions moves the saturating curve by less than the floor's own precision.
The change is a MISCOMPILATION fix besides — a program that came out cyclic
now does not — and that is not a term the objective has at all.

## 2026-09-10 — three library shapes off the run program's profile

Three arms, each one a shape the profile pointed at, each one measured on
today's main rather than on the base they were first cut against.

**A cap around a count is a range.** pendbench's churn fuses to a fold over
`take naturals n`, and the general capped arm pulled every element through
`next`, building a count, a step, a cap and another step apiece — four records
read once and dropped. `fold (capped left (counting at))` counts instead, the
sibling of the window arm already beside it.

**A class asks by the byte.** std/regexp's character classes walked a string
per candidate; the arm reads the byte.

**The colon is the arm before the error.** lib/json's `obj_key` ran `skip_ws`
to the colon, took back only its position, and then had `expect_char` load the
same byte a second time to compare it. `obj_key_end` is the shape `array_delim`
and `obj_delim` already take: the colon is an arm, the whitespace is the arm
before it, and the error is the arm after.

**A deletion at the head of a library file moves every diagnostic below it.**
Dropping `import "std/text"`, `expect_char` and both `expect_check` arms takes
eleven lines off the top of lib/json/scan.kso, so `fail` moves from line 13 to
line 2 and the endpoint diagnostic that names its birthplace moves with it:
`tests/golden/runtime/a_lone_surrogate_is_half_a_character.stderr` now reads
`born in json/fail at std/json/scan.kso:2`. Two tests read that one golden —
`runtime_corpus_reports_endpoint_violations` in tests/golden.rs and
`interpreter_reports_each_runtime_endpoint_violation` in tests/oracle.rs — and
both went red until it was regenerated. Nothing else in the tree quotes a
scan.kso line. Worth writing down because no counter, gate or sweep can see it:
the change is a deletion of dead helpers, and it moved a user-visible sentence.

**What they cost, on this tree over origin/main ef2f4ea4.** The arms were cut
against 32080acf and re-measured after #1366 merged in. Not one runtime counter
moved between the two bases: #1366 changed which constructor `fail` emits, not
what anything allocates.

    pendbench   allocs      4,007,349 ->   807,149   (-79.86%)
                alloc_bytes   257 MB  ->     65 MB   (-74.66%)
                arena peak      2 MB  ->      1 MB
                sh_rec    192,067,248 ->    54,448   (-99.97%)
    scanbench   allocs      3,975,884 -> 3,011,152   (-24.26%)
                arena peak    198 MB  ->    161 MB   (-18.52%)
                sh_str     37,039,216 ->     1,632
    runbench    allocs      6,936,517 -> 5,958,961   (-14.09%)
                arena peak   45.9 MB  ->   39.7 MB   (-13.70%)

`sh_rec` falling by a factor of three and a half thousand on pendbench is the
counted fold: those four records an element were most of what the benchmark
allocated. `sh_str` on scanbench is the byte class, which is why that row goes
to almost nothing rather than down a fraction.

**And the instructions, which this container could not see at all.** The work
vein is host-keyed and refused here, so welfare read main's row and reported
the floor merely held; CI's sitting is where the change actually shows.

    pendbench    583,755,724 ->   226,874,035   -356,881,689  -61.1355%
    scanbench    726,018,879 ->   587,488,478   -138,530,401  -19.0808%
    runbench   2,367,876,664 -> 2,262,265,575   -105,611,089   -4.4602%
    jsonbench  1,485,334,791 -> 1,470,973,791    -14,361,000   -0.9669%
    oneshot       21,737,110 ->    21,641,370        -95,740   -0.4404%
    deepbench    387,470,234 ->   387,090,234       -380,000   -0.0981%
    basket        34,690,216 ->    34,672,719        -17,497   -0.0504%
    livebench  3,481,899,695 -> 3,481,803,955        -95,740   -0.0027%

Five rows are byte-identical: encodebench, widebench, escapebench, indexbench,
digestbench and readbench reach none of the three arms. Nine of the fourteen
fall. pendbench losing three fifths of its instructions is the counted fold
alone -- the four records an element were not only allocated but walked.

**Welfare 66.42 -> 67.56**, the largest single move since the 2026-09-03
rebuild, banked with `--set` in this same PR. Both the allocation terms and
run_instructions pay for it; the container's blind reading had put the gain at
0.86 and the whole of that was the memory side.

**The compile side pays, and the emitted code grows.** Three new arms is three
more declarations for the front end to visit: front_end_visits 22,562 -> 22,727
(+165, +0.7313%) with rounds holding at 62, the arms being wider rather than
deeper so nothing re-converges. Every program that compiles std/list carries
the counted arm, so the emitted rows rise across the board — encodebench 11,104
-> 11,232 lines, scanbench 19,673 -> 19,922, runbench 34,726 -> 34,979. The
decoder is the one row that gains lines while LOSING calls: 9,246 -> 9,272 with
calls 1,236 -> 1,226, because it reaches the colon arm and neither of the other
two. Four rows are byte-identical — escapebench, indexbench, digestbench and
readbench reach none of the three. That is the trade stated plainly: more code
written, much less allocated.

**Every counter that went the wrong way, with the value it landed on.**

The byte class trades string work for byte work, so two counters that were
zero on scanbench are no longer zero and the same pair rises on runbench, which
runs split: `scan_find2_calls` 0 -> **501,505**, `scan_sh_bytes` 0 ->
**24,072,240**, `run_find2_calls` 3,017,520 -> **3,109,759**, `run_sh_bytes`
36,862,800 -> **41,290,272**. Against them `scan_sh_str` falls 37,039,216 ->
1,632 and `scan_sh_buf` 24,105,200 -> 33,024. The bytes a class reads are the
bytes it used to read as a string, and it stops copying them.

The compile-module vein carries the three new declarations: `module_defines`
100 -> **101**, `module_calls` 727 -> **754**, `module_branches` 430 -> **446**,
`module_lines` 5,177 -> **5,304**, `module_visits` 2,471 -> **2,534**, beside
`front_end_visits` 22,562 -> **22,727**.

Three more declarations cost the compiler itself, and these are CI's rows:
`compile_instructions` 48,412,144 -> **49,091,884** (+679,740, +1.4040%),
`entry_instructions` 161,360,451 -> **164,041,672** (+2,681,221, +1.6616%),
`library_instructions` 162,069,092 -> **164,324,373** (+2,255,281, +1.3910%),
`compile_allocs` 29,000 -> **29,350**, `compile_peak_bytes` 769,071 ->
**774,660** (+5,589, +0.7267%). The objective's compile term is module plus
entry, 209,772,595 -> 213,133,556, a rise of 1.60% against a run-instruction
fall of 4.46% on a term that satiates late; the trade is not close.

The `text` vein sums 1,543,836 -> **1,547,852** across the fourteen, and it
moves both ways by families: **+800 or so** where the
counted fold lands (deepbench 75,538 -> 76,338, digestbench 110,530 ->
111,330, encodebench 119,826 -> 120,642, widebench 125,090 -> 125,906,
runbench 246,082 -> 246,914, pendbench 91,090 -> 91,778, basket 113,154 ->
113,650), **-400** where the colon arm does (jsonbench 100,130 -> 99,730,
oneshot 112,002 -> 111,602, livebench 112,578 -> 112,178), and scanbench
159,602 -> **159,570**, the one row that carries the byte class and comes out
32 bytes smaller. escapebench, indexbench and readbench are byte-identical.
The machine code grows where an arm is added and shrinks where three helpers
go, which is what the emitted vein says in lines.

The emitted veins are the same three arms written out: `emitted_branches` 816
-> **819** and `emitted_lines` 9,246 -> **9,272** on the decoder;
`emitted_other_defines` 2,354 -> **2,366**, `emitted_other_calls` 20,245 ->
**20,445**, `emitted_other_branches` 12,639 -> **12,800**, `emitted_other_lines`
132,530 -> **133,802** across the thirteen beside it. `emitted_calls` on the
decoder FALLS, 1,236 -> 1,226.

Two rows of the inner-beat tenure fixture move with the counted fold, which is
what that fixture folds:

    an_inner_beat_opens_its_tenure_in_the_block_outside_survive_slots 2,004 -> 32,404
    an_inner_beat_opens_its_tenure_in_the_block_outside_ten_frees 3 -> 2

with `ten_blocks` 3 -> 2 beside them. The fold stops building four records an
element, so far less
is allocated inside the beat (allocs 512,494 -> 177,420, alloc_bytes 41.2 MB ->
16.9 MB) and the beat runs a third as many iterations (158 -> 68); what survives
a longer-lived block is counted in more slots and freed in one fewer.

**Five veins are CI's.** machine_code, compile_allocs, compile_instructions,
entry_instructions and library_instructions are refused on this container, and
compile_memory_golden.txt is refused whole (measured-on rustc=1.98.1, here
1.94.1) even though the visits row it carries is host-invariant. Round one is
red on those and CI's sitting is what lands.
