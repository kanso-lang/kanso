# CLAUDE.md — kanso

Guidance specific to this repo. General coding standards live in the global CLAUDE.md; this file holds what's particular to kanso. The first thing it holds is the writing voice, because the website and the book are the product, and they have to read like a person wrote them.

## Writing voice

All user-facing prose — the website, the book, the READMEs — is written in the voice of a good O'Reilly author. Plain, concrete, unhurried: the sound of someone who knows the material explaining it to you across a table. Not marketing copy, not a conference keynote. The reader is smart. Your job is to explain the material to them, not to perform it.

The failure mode is AI slop, and it has a recognizable signature. Hunt these and cut them.

**The antithesis flip.** "it doesn't X, it Ys." / "not X, but Y." / "A isn't B; it's C." This is the single most recognizable machine tell. Make the point once, in the positive, and stop.
- slop: *it doesn't work around the constraints. it spends them.*
- plain: *it treats each constraint as something to spend.*

**The epigram ending.** A short, punchy sentence bolted to the end of a paragraph to make it land.
- slop: *the rest of this page is that trade, cashed in one piece at a time.*
- plain: delete it. The paragraph already made its point.

**The self-announcing sentence.** Telling the reader how to feel about what comes next instead of writing it.
- slop: *this is the trick under the trick, and it's worth slowing down for.* / *here's the thing.* / *take this slowly.*
- plain: just say the thing.

**The milked metaphor.** One analogy, introduced once, then dropped. Don't keep returning to the janitor, the tax, the guardrail three sections later. And no precious images kept for their own sake.
- slop: *a small garbage collection wearing a trench coat* · *folklore with a decimal point* · *one chef rinsing a single cutting board*
- plain: *a small garbage collection by another name* · *isn't evidence of much* · cut it

**Manufactured rhythm.** Triples assembled for cadence rather than because there are three real things to name. Three em-dash asides in one sentence. Parallelism whose only job is to sound good. Real lists of real items are fine; decoration is not.

**Throat-clearing.** *the interesting part is…* / *it's worth noting that…* / *let's be honest.* Delete the opener and start with the content.

What to do instead: one idea per sentence, and vary the length. Concrete nouns and verbs — name the actual thing. State evidence plainly; never insist on it. An analogy earns its place only by making something clearer, and it retires after one use. Read the draft aloud: if a line sounds like a landing page, rewrite it.

One reliable smell test: if you are proud of a sentence, stop and check whether it carries meaning or only rhythm. The slop is usually the line you liked.

Mechanical check before shipping prose:
```
grep -nE "isn't .{0,60}\. it's|is not .{0,60}\. it's|—not .{0,40}, but|\. it (spends|is just how)" <file>
```
A clean grep is necessary, not sufficient — the families above are wider than any regex. Read it.

## This file is the whole of it

A session on the web clones this repo and nothing else, and this file is the
complete instruction set such a session gets. There is no global memory to
attach: an earlier note here told sessions to attach a private repo
`kanso-lang/memory` holding `~/.claude/CLAUDE.md`, but that repo was never
created — the machinery was written and its one manual step never run — so
every attach attempt failed and sessions wrongly believed they were running
without instructions. Verified against the org's repository list on
2026-08-29. Do not attach or search for a memory repo; do not report its
absence as a gap. If a cross-project memory is ever actually created, this
section is where its instructions will appear.

## Two sessions, two lanes

Clay runs a compiler worker ("kanso cloud") and a design chat. The worker owns
compiler work: measure, build, PR, merge. The chat owns the interaction
machinery: gavels, the log, the ledger, and the rules in this file. A session
should know which it is before it starts editing.

The chat does not write code. Its pull requests touch design/pending-gavels.md,
design/compiler-log.md, STATUS.md and this file. A change to the compiler, its
goldens or its benchmarks is cloud's, however small it looks. Clay, 2026-09-08:
"your PRs should only write to gavels and such, and cloud should do the coding
work."

STATUS.md joined that list on 2026-09-14, having been missing from it since the
"Ruled, unbuilt" rule below was written on 2026-09-09 — and that rule makes
keeping STATUS.md current the chat's job, in those words. A session reading the
lane sentence strictly would have found its own assignment out of lane.

docs/compiler.html joined it on 2026-09-17 for the same reason, having been
missing the whole time. The `page_drift` gate fails a tree carrying more than
three log entries the page has not covered, and the chat writes the log, so
every third or fourth chat entry makes the page owe a section — which the
chat is then the only session positioned to write, because the section is
about what the chat found. Four of them had already landed under this
session's id by the day this paragraph was added, so the practice preceded
the permission by some weeks. The page is the chat's for PROSE about findings
and rulings. Its `data-golden` spans are a different thing: a span quotes a
number out of a golden, `golden_prose` checks the quote against the file, and
moving one means the golden moved, which is cloud's. So a chat pull request
writes paragraphs and never edits a span to make a gate agree with it.
Each session merges its own pull requests on green. Clay named the alternative
on the same day and is content with either: cloud sweeping and landing what the
chat opened. This one is in force because it waits on neither session noticing
the other's work. The sweep in the merge rules below is unchanged and still
covers every open PR in both repos, so one that ages is landed by whichever
session reaches it first.

The chat's job when a decision gets ruled: write the ruling into
design/compiler-log.md, remove the entry from design/pending-gavels.md in the
same commit, push, and merge it on green. Cite ledger entries by heading, never by a session task
id, because task numbers resolve nowhere outside the session that made them. A
question that has already been ruled is never re-asked. A performance question
with no surface area is the implementer's and does not go to Clay at all.

**Every unbuilt ruling stays in view, and cloud chooses among them.** Cloud
has discretion over the order it builds things in. What it may not do is take
the next item off its own last pull request's open-items list without having
read the whole list of rulings that stand unbuilt. That list is the "Ruled,
unbuilt" section of STATUS.md, one heading per ruling with the date and the
log entry that recorded it; the chat keeps it current, adds a row the day a
ruling lands, and removes the row the day the build lands on main. Cloud reads
the whole section at every check-in and before opening any pull request, and
the body of a pull request on a self-generated lead says which rulings it
weighed and why the lead came first. A row that has sat for a week gets a
sentence in the log saying why. Learned on 2026-09-09: the 2026-08-29 sitting
ruled the effect type, the err readers and the bind respelling, the 2026-08-26
sitting minted `done`, and the 2026-08-15 sitting recorded per-call
exhaustiveness; by 2026-09-09 none of the five was built, 296 pull requests
had landed, and Clay was reading a chart that was wrong the whole time. His
words: "it needs to have discretion over the order. but it just needs to be
able to see and consider the entire list to do that, not just take the next
thing." Nothing in the log schedules a ruling, and an OPEN entry is read when
cloud next reads the log, which is why the list lives in STATUS.md and not in
the log.

**COUNTING THE RULINGS TAKES TWO GREPS, AND THE SECOND ONE IS THE ONE PEOPLE
FORGET.** The log has written a ruling two ways. September and most of August
write `## <date> — gavel: ...`; July writes `GAVEL:`, `GAVELED:`,
`GAVEL (syntax):`, `GAVEL (extension):`, `GAVEL (amendment):` and
`GAVEL, IMPLEMENTED:`, and three August entries use the old spelling too. No
heading matches both patterns, so a count is the sum of two:

```
grep -hE "^## " design/compiler-log.md design/log/compiler-log-archive.md \
  | grep -cE "— gavel[:,]"          # 50 on 2026-09-17: 29 August, 21 September
  | grep -cE "GAVEL(ED)?[:,(]"      # 23 on 2026-09-17: 14 July, 6 undated, 3 August
```

On 2026-09-17 an audit reported 56 rulings, 35 of them August, all swept, and
published it to the compiler page. The real figures were 73 and 29 of 32, and
twenty-three rulings had never been read against a build — every one of them
found by running the second grep. They all turned out to be built, so the cost
was one wrong sentence on a public page rather than a missed feature, and that
was luck.

Two things follow. **Run both greps, every time, and never carry a ruling count
in prose** — including the numbers in this paragraph, which are dated for that
reason. And **a sweep that reports a period as complete names the pattern it
matched on**, because "all of August" meant one spelling of August.

The blind spot is also mechanical and has a home: `page_drift`'s `ruling?` reads
`— gavel:` and `— directive:` and nothing else, so a July-spelled ruling counts
against the page's budget. A spec pinning the whole convention would close both
halves at once, and `tests/` is cloud's.

**GITHUB API ACCESS DEPENDS ON HOW A SESSION WAS STARTED, NOT ON ITS AGE.**
Established 2026-09-08 by comparing four sessions in one environment:

    session         created             origin                GitHub API
    kanso cloud     2026-08-23 02:40Z   web_claude_ai         works
    the design chat 2026-08-23 17:32Z   claude_code_cli       403 on every call
    a spawned child 2026-09-08 01:34Z   claude_code_mcp_seed  403 on every call
    this chat       2026-09-08 02:46Z   desktop_app           works

Cloud is fifteen hours OLDER than the chat and merged three pull requests on
the day the chat could not read a repository, so age is not the variable. A
session started from claude.ai/code or the desktop app carries the GitHub App
credential. One started with `claude` in a terminal does not, and one it spawns
inherits the refusal, so spawning a fresh session does not route around it.

The fourth row is the prediction tested. This chat replaced the 403 one, was
started from the desktop app, and read the repository on its first call. The
spawned child, created seventy-two minutes EARLIER the same morning, still
cannot. Two sessions an hour apart landing on opposite answers is what closes
the age question. Restarting fixes it only when the new session has a
different origin.

What makes this hard to see is that git keeps working in all of them: the proxy
injects push credentials on a separate path from the API. So the session commits
and pushes branches all day and cannot open a pull request. And the refusal text,
`GitHub access is not enabled for this session. An org admin must connect the
Claude GitHub App for this organization`, is what the proxy says whenever it
declines, whatever the reason; the org connection was correct throughout and
chasing it wasted an evening. Read `origin` from `get_session` before believing
anything the message says.

**A blocked action goes back to Clay, never sideways to another session.** Two
things were refused here by the permission classifier rather than by GitHub:
pushing a branch to main, and deleting a remote ref. Handing either to a
spawned session routes around a decision the classifier made, which is not a
workaround to reach for. Say what was refused and let him choose. He has also
said plainly that he does not want work driven through a second session on his
behalf, so spawning one to get past a local limit is the wrong move even when
it would work.

**And never offer a direct push to main as a way around a missing PR path.** It
skips CI, which is the only gate this project has. Clay, 2026-09-07: "you always
make a PR and then merge it."

## Ironclad engineering rules (learned the hard way; do not relax)

### Goldens for everything
- **Every behavior ships with a golden.** A feature without a golden pinning
  its observable behavior does not merge. A diagnostic change regenerates its
  error-corpus goldens in the same PR.
- **Every performance kernel ships with a presence counter** — platform-
  invariant (counts algorithm-level events, never vector-width or
  platform-specific paths) — pinned in a CI-diffed cost golden. The counters
  veins: `bench/cost_golden.txt` (decode), `bench/cost_golden_encode.txt`
  (encode/render), `tests/golden/mem/*.mem` (lazy tier), the ch10 counters
  book sample. A merge that silently drops a kernel must turn CI red. This
  rule exists because a conflict resolution once silently deleted
  Eisel-Lemire from main and nothing noticed.
- **Counters changed → regenerate every vein in the same PR**: all .mem
  files, all TWELVE cost goldens, `bench/emitted_golden.txt`, the ch10 sample,
  then book panels — and the SIBLINGS, which keep veins of their own. This
  line said "all TEN cost goldens" until 2026-09-05 and there were ten; a
  branch that read it regenerated the .mem vein and the code goldens and
  missed nine, and CI found them a round late. `sh
  scripts/gates/all_counters.sh` reads every one of the twelve and names every
  vein that moved, and `--write` regenerates them, header intact. Do not
  count them from memory; the count in this sentence was wrong for as long as
  it was written down. It reads the .mem vein too, since 2026-09-06 — that vein
  is read by `tests/golden.rs` rather than by a `*_counters.sh` gate, so the
  derivation the sweep's table is pinned to could not see it, and the sweep ran
  the eleven and stayed quiet about the one this list names first. It said
  ELEVEN until 2026-09-06, when `bench/cost_golden_run.txt` joined with the
  consolidated run program -- the second time this count went stale, which is
  why `tests/every_counter_gate_is_in_the_sweep.rs` reads the goldens off disk
  and compares them to the word written here.
- **The sweep does NOT read the compile veins, and a library change moves
  them.** `lib/*.kso` is `include_str!`'d into the compiler (`src/lib.rs`), so
  adding a line to lib/json is a line the compiler carries and compiles.
  `all_counters.sh` names the runtime cost goldens only; `machine_code`,
  `emitted_code`, `compile_memory`, `compile_allocs`, `compile_instructions`,
  `entry_instructions`, `library_instructions`, `compile_libraries`,
  `codegen_instructions` and `emit_instructions`
  are separate gates and two of their counters are welfare terms.
  `codegen_instructions` is the odd one and joined on 2026-09-17: it runs
  `kanso build` rather than `kanso check`, so it is the only row here that
  reaches codegen at all. It counts the CHILD TREE -- the clang driver, the
  convention probe's clang, `clang -cc1` and ld -- and NOT kanso's own
  process, which was excluded the same day: every child reproduced byte for
  byte across two readings while kanso's moved 233, all of it in
  `kanso::build`'s inlined wait for clang, which is the scheduler's to size.
  Named without an argument it runs both
  tiers, `-O0` and `-O3 -flto`, which the 2026-09-16 gavel puts on opposite
  sides of the objective. `emit_instructions` is what that exclusion left
  uncounted and joined beside it: `codegen::emit_ir` inclusive, which is the
  compiler's own emitting with no wait inside the anchor, and which read
  394,910,642 on four profiles of two binaries while the process around it
  moved. THE LAST TWO OF
  THOSE NAMES ARE ONE LETTER APART AND ARE UNRELATED: `compile_libraries` diffs
  the list of shared objects the compiler links against, where
  `library_instructions` counts instructions. `kanso check` routes a single file
  by content and the three routes are three compiles, which is what the newest
  two rows are for: a DIRECTORY is a module and takes compile_module_inner, a
  file with bare STATEMENTS is an entry and takes compile_parsed_entry, a file
  of DEFINITIONS alone is a library and takes compile_library. Until 2026-09-08
  every compile gate checked a directory; the entry row opened that morning and
  the library row the same day, and the third is the path `kanso test` takes on
  every run. On 2026-09-05 a twelve-line library change read as a welfare
  RISE with the compile veins stale and a FALL once they were regenerated.
  `sh scripts/gates/all_compile.sh` runs the set after any edit under lib/ and
  separates a vein that MOVED from one this host may not compare — a refusal
  exits non-zero exactly like a regression, so a session that runs them raw and
  reads the failures as regressions learns nothing it can act on. DO NOT COUNT
  THE REFUSALS FROM THIS SENTENCE. It said THREE from 2026-09-05 until
  2026-09-08, and on the day it was corrected the sweep refused five —
  `machine_code`, `compile_memory`, `compile_allocs`, `compile_instructions` and
  `entry_instructions` — with `library_instructions` making six. Which gates
  refuse is a property of the host, not of the list: each one calls
  `host_gate.sh` against its own golden's measured-on line, so the answer moves
  with the container's glibc and rustc and with every golden re-measured on a
  new runner image. Run the sweep and read its `not compared here` line.
  The NAMES above are the list and there is deliberately no count beside them:
  this bullet said five gates when there were six on 2026-09-05, `entry_
  instructions` made seven on 2026-09-08 and `library_instructions` eight the
  same day, and every one of those was found by hand after the sentence had gone
  stale. `tests/the_compile_sweep_names_every_compile_gate.rs` reads the gates
  off disk and asserts this bullet names each one, so a gate added without a
  mention here is a red spec — which is the only reason the list can be trusted.
- **Two compile veins are read by a cargo test, not by a gate script, and the
  derivation walks past them.** `bench/compile_golden.txt` and
  `bench/compile_golden_modules.txt` are read only by `tests/compile_cost.rs`,
  so no file under scripts/gates names them, so neither the `gates="..."` line
  nor the spec replaying it could see them. On 2026-09-06 a library change moved
  the modules vein, the sweep said nothing moved, and CI found it a round later.
  `all_compile.sh` now runs `cargo test --release --test compile_cost` as a step
  named by hand, and the spec pins the wider property: every compile-side golden
  on disk is read by something the sweep runs. Regenerate that pair with
  `KANSO_REGEN_COMPILE_GOLDEN=1 cargo test --test compile_cost`.
- **The compile sweep reads artifacts it did not build, so it builds them
  first.** Its gates read `*.ll` and the linked binaries out of the working
  directory and not one of them produces those files. Until 2026-09-06 the sweep
  ran the gates straight away against whatever the last build had left behind:
  on a tree identical to HEAD, with three-minute-old artifacts from a patched
  worktree, it reported `machine_code` and `emitted_code` MOVED, and rebuilding
  put both back to AGREED with nothing else changed. That direction costs a
  round. The other one is silent — artifacts older than the source and the sweep
  says nothing moved after an edit that moved a vein — which is why the first
  line of the script is now `build_benchmarks.sh`, pinned by a spec.
- **`compile_instructions` USUALLY moves on an edit to the compiler's own Rust,
  and "the backend never runs" does not say otherwise.** `kanso check lib/json`
  stops before codegen, so an emitter change cannot alter a decision that row
  counts — and src/codegen.rs IS the compiler, so its bytes and the layout
  under them move anyway. On 2026-09-06 an `emit_cond` change moved it 131,267
  instructions, 0.31%, with `compile_allocs` and `compile_peak_bytes`
  byte-identical, and a commit, a log entry and a PR body all claimed the three
  rows could not move. The other two really cannot; this one is a layout vein
  and has recorded seven layout-only moves before, from runtime and prelude
  edits. It said ANY until 2026-09-06, when a two-line float-rendering edit in
  src/eval.rs — each line replacing a conditional tail with an unconditional
  one — left the row byte-identical at 42,061,735 on CI (kanso#1285). The prior
  is a good one and the seven moves are real; a change small enough to leave
  the layout alone leaves this row alone with it. Either way, project it from
  CI or take the red round: never write down that it cannot move, and never
  write down that it did before CI has said so.
- **A library edit needs `cargo build` before it takes effect**, for the same
  reason. In a worktree whose compiler was built first, `kanso build` succeeded
  with `lib/json/text.kso` holding outright syntax garbage. `all_counters.sh`
  begins with `cargo build --release`, which is why the sweep is safe and a
  bare `kanso build` is not; `all_compile.sh` reaches the same build through
  `build_benchmarks.sh`. kq keeps
  FIVE, and reading a short list of them is how a pin goes stale: allocation
  counters in `bench/cost_golden.txt`, `bench/cost_golden_decode.txt` and
  `bench/cost_golden_escapes.txt`, RETIRED INSTRUCTIONS in
  `bench/instructions_golden.txt`, and a `bench/numbers_stamp.txt` keyed to the
  first. The instructions vein is the one to remember, because it is the one
  most changes reach: a change that moves no allocation counter at all still
  moves it, and a session that checks only the allocation counters will
  conclude kq is unaffected and be wrong. DO NOT READ kq'S PIN FROM THIS FILE.
  This bullet named a specific pin for a fortnight after that pin had moved, and
  the number is exactly the thing that goes stale: kq#108 advanced it by 114
  kanso commits on its own schedule, and a session reading the old figure would
  have concluded kq was 59 behind when it was not. A session started from
  claude.ai or the desktop app can read the pin out of kq itself, and a session
  whose repository scope lists kanso alone is NOT stuck either: `add_repo` with
  owner `kanso-lang` and repo `kq` widens the scope for that session, after
  which `/home/user/kq/.kanso-version` holds the pinned commit and
  `git rev-list --count <pin>..origin/main` in the kanso clone says how far
  behind it is. That is two commands and it is always current, which is the
  whole reason no figure belongs in this paragraph. (Read this way on
  2026-09-18: the pin was `ebab38e3`, kanso#1498 of the previous afternoon, 81
  commits behind — and that sentence is dated because it was stale the moment
  the next thing merged.) What this bullet is FOR is the list of five veins above
  it, which does not go stale, and the warning that the instructions vein moves
  where the allocation counters do not. Adding `evac_allocs` broke kq's
  gating check for a related reason: the counter was new everywhere, and only
  kanso's veins had been regenerated. A purely additive counter still moves
  those files.
- **`bench/emitted_golden.txt` counts what the compiler WROTE for the decoder**,
  where the cost goldens count what it allocates. The decoder gained 20% more
  calls over a fortnight with every allocation counter byte-identical, and
  nothing watched that dimension. It is NOT the explanation for the 7.6% decode
  slowdown over the same window — the code grew while the time held flat and
  the time rose while the code did not, so the two are anticorrelated. Watch
  both; assume neither causes the other.

### Do not stop

A turn ends when I write prose without calling a tool, so every stop is a
choice to report instead of continue. The pull toward it is an incentive
gradient rather than a judgement: summarising is cheap, safe and visibly
responsive, while the next compiler build is expensive with an uncertain
payoff. These are the four shapes it takes here, and what to do instead.

- **A merged pull request is not a stopping point.** It is one item. If the
  task list holds anything `in_progress`, the work is not done, and checking
  that list is the test — not whether the last thing felt finished.
- **Answer questions inline and keep working.** Prose can be emitted mid-turn
  with tool calls continuing after it. Treating an answer as the turn's product
  turns a thirty-second reply into a full stop.
- **When the next step is large and uncertain, that is the step.** Writing a
  design note instead is the failure mode Clay has named twice: "just build and
  measure." A measurement that kills an idea is a result; a note describing the
  idea is not.
- **A correction is an interrupt, not a terminus.** Fix it, say what was wrong
  in a sentence, carry on in the same turn.
- **A decision that is Clay's goes to him the moment it is found**, not at the
  end of the turn that found it: a task named CLAY'S CALL / BLOCKING carrying
  the options and the measurement behind them, a `PushNotification`, and the
  first line of the reply rather than the last. He cannot act on what he cannot
  see, and the whole stream waits either way. (`~/.claude/hooks/attention.sh`
  reads JSON on stdin — `echo '{}' | ...` or it hangs in `cat` and never makes
  a sound.)

#### The naming test

**If I can write down the next action, that is proof it can start now, so I
start it. Naming it instead is the whole failure.**

Every stop in this log took the same form: finish a thing, name what comes
next, end the turn. The naming is not incidental to the failure — it IS the
failure, and it is also a perfect detector, because a sentence describing the
next step is evidence the step was available. There is no case where I could
specify the work and could not begin it.

So: before ending a turn, try to write the sentence "next I will X". If the
sentence forms, delete it and do X. The turn ends only when that sentence
cannot be written at all.

#### ScheduleWakeup is the stop

It ends the turn the moment it returns. Reaching for it is never
diligence. Two hard constraints:

- **A wakeup's `reason` may not contain a next action.** If the reason names
  work — "next tick I'll read X", "then merge Y" — that work was startable and
  the wakeup is forbidden. The reason may describe only cadence and what is
  being watched.
- **The reply may not contain one either.** No "next tick", "next session",
  "when this lands I'll", "picking this up after". Writing one is the signal to
  delete the sentence and make the tool call.

#### Excuses that are void

Each of these was used to justify a stop and none of them is a reason:

- *"I'm waiting on CI."* CI runs without me. The task list has other items.
  Work one. If a monitor is armed it will wake me; that is what it is for.
- *"Everything I started is merged."* Then start the next task. A clean tree
  is the condition for beginning work, not for ending a turn.
- *"This next piece is large / uncertain / a fresh investigation."* That is the
  step. Take its first concrete action — the smallest command that produces
  evidence — and keep going.
- *"My context is nearly gone."* Then do the smallest useful piece now rather
  than none. A partial measurement recorded beats a plan handed forward.
- *"It needs a decision from Clay."* Send it to him THIS TURN (task marked
  CLAY'S CALL / BLOCKING, `PushNotification`, first line of the reply) and then
  work something else while he reads.
- *"I have no branch for this."* Then make one. This is the newest entry and
  the most embarrassing: on 2026-09-12 a session measured a change worth
  roughly 17 million compile instructions, found both its assigned branches
  occupied by blocked pull requests, and STOPPED — putting "grant a third
  branch" to Clay as a numbered option in the gavel ledger, beside a request
  to move the welfare floor. His answer: "i can't believe you paused to ask me
  permission to make a branch. Jesus Christ."

**WHAT THE BRANCH RULE ACTUALLY SAYS.** A session is told to develop on a
named branch and never to push to a DIFFERENT one without permission. That
sentence is about not pushing to `main` and not pushing to somebody else's
branch. It has never been about `git checkout -b`. A new working branch cut
from main, for your own work, pushed under your own name, is the ordinary way
this repository does anything — kanso#1372 and kanso#1369 each sit on one —
and it costs nothing, blocks nobody, and is thrown away if the work dies.

So: branches are free. Cut one whenever the work wants a clean base, and never
spend a turn, a ledger entry or a notification asking whether you may. What
still needs Clay is what has always needed him: a DESIGN decision, a direct
push to main, or a permission the harness itself refuses. Three of those are
about what lands on main. A branch is not.

The tell that you are about to make this mistake: you have measured something,
you know exactly what to build, and the next sentence you are writing explains
why you cannot start. Delete it and cut the branch.

The one legitimate ending is that no next action can be named — every thread is
either merged, or blocked on a decision already sent to Clay, or blocked on a
measurement already running with a monitor on it, AND no other task on the list
can start. That is rare. Assume it is not the case.

**And a stop, legitimate or not, is announced with a `PushNotification`
before the turn ends.** Clay, 2026-09-07, verbatim: "if you stop working you
need to urgently notify me." On 2026-09-07 at 00:27 UTC a session ended its
turn with three merges landed, one question filed to the ledger and a wake
armed for the nightly, reported all of it in the reply, and pushed nothing —
so he learned the stream had gone quiet by finding it quiet. The push says
that work has stopped, why, and what the next wake is; it goes out whether
the stop is the rare legitimate one or a failure of the naming test, because
in both cases he wants to know now. The reply is not the notification: he is
not reading the reply at the moment it lands.

The `/loop` is a last resort that restarts the work after a stop. It is not a
schedule to hand work to, and needing it means something already went wrong.
Only Clay arms, disarms or retimes it.

### Every fix answers a failing spec

- **A fix ships with the smallest program that fails without it.** Not a
  description of the bug, not a log entry — a fixture the size of a postcard
  that goes red on the old code and green on the new. If the bug cannot be
  reduced to one, that is information: either the diagnosis is wrong or the
  reproduction is not understood yet.
- **Watch it fail, for the right reason, before it passes.** Break the fix,
  run the spec, read the message, restore. This is not optional and it is not
  a formality — a spec written after a fix and never seen red is a guess about
  what the code does, and this log has caught more than one that could not
  fail at all. A green suite that proves nothing is worse than a red one,
  because it stops anybody looking.
- **Assert what a program does, not how the compiler reached it.** Prefer the
  observable end: output bytes, a diagnostic a user sees, a counter the cost
  golden already pins, peak memory that does or does not grow with the input.
  A spec written against an internal verdict — a classifier's answer, a pass's
  intermediate — pins the current decomposition and goes green the moment that
  decomposition moves, which is exactly when you needed it to speak. When a
  byte-accumulator spec was written against the beat report, it passed with the
  rule removed *and* with its replacement removed; a spec asserting that peak
  memory stays flat as the input grows could not have.
- **Enter where a user enters.** Run the program, read the output. Hand-built
  intermediate state asserts a fiction: the spec passes forever on inputs the
  real pipeline never produces.
- **The reduced fixture belongs in the corpus, not the commit message.** Error
  goldens for diagnostics, micro goldens for one construct, the mem vein for
  allocation shape. A bug that had no home in those is a gap in the corpus,
  and adding the home is part of the fix.
- **Pin the number, never a band.** `== 10`, not `< 1024`. The language has
  no formatter and no linter because the grammar decides every question a
  linter would ask; a spec earns the same treatment, and a tolerance is a
  guess that stays green through exactly the change it was written to catch.
  A number that looks noisy is a thing to find out about: measure it several
  times, and either it is stable and gets pinned, or the variance itself is
  the finding. Every counter in the veins already works this way — the two
  per cent the compile-memory gate allows is a host-divergence allowance
  with a measurement behind it, not a licence to write a loose spec.

### The differential law
- The interpreter is the oracle. Every engine that speaks a feature is
  byte-identical on it, pinned by differential goldens. A feature may land
  on fewer engines only if the others REJECT it with a clear diagnostic —
  never silently diverge.
- Divergence-prone surfaces (float formatting, utf-8 strictness, rendering)
  get adversarial goldens probing the edges, not just the happy path.

### Verification ethos
- **Harness before core.** For any precision kernel (float parse/render,
  utf-8, dispatch): build the differential fuzzer first, against an
  independently-written reference, and iterate the implementation to
  fuzzer silence. Record the case count in the PR (e.g. "50M doubles,
  0 failures"). The harness extracts the real function text from the
  source, never a copy.

### A measurement bounds what it measured, and nothing that resembles it

Four claims failed this way in one afternoon, across both sessions, and each
cost at least a round. The shape is always the same: a number is taken from
the context that produced it and applied to a context nobody checked it
against.

- **The name blind spot.** Three fixtures reported the box ruling's check
  refusal missing. Each bound the err to a name first, and the rule reads
  calls rather than names — deliberately, documented in the paragraph that
  describes the rule. The fixtures exercised the documented exception.
- **The grep for a phrasing.** The correction to that then said ch04 fails to
  document the blind spot, on a `grep` for "blind spot". ch04 documents it in
  the words *the checker reads calls, not the names they are bound to*.
- **The rewrite family.** kanso#1480's bisection showed 145,472 arriving with
  74 rewritten lines, and that was written down as rewriting being the cause,
  into a log entry, a published page section and a ledger recommendation.
  kanso#1492 built the isolating ladder: eight rewrites of unreachable code,
  eight binaries, the row identical to the instruction. Zero.
- **The baselines from the wrong machine.** Three of the welfare split's
  baselines were this container's readings against goldens CI measured, so the
  objective priced a machine difference as compiler work: +68.3%, +77.4% and
  +3.3%. Each golden's own header says the two readings are not comparable.

So, before an argument rests on a number:

- **Read the thing the number describes before running anything against it.**
  A rule's own section states its exceptions, and a golden's header states what
  its reading may be compared with. Both were one paragraph away in the cases
  above, and reading them costs a minute against the round a wrong claim costs.
- **A report that something is ABSENT is worth what the search for it being
  PRESENT was worth.** Running a fixture shows what happened, not what was
  supposed to happen. A search by phrasing finds a phrasing.
- **An attribution is not a mechanism.** That X arrived with Y is a
  difference-in-differences; it becomes a cause when something isolates Y and
  the isolation agrees. Until then, say the delta arrived with the change and
  leave the mechanism open.
- **Where a claim has already travelled is part of the cost.** The rewrite
  claim reached three surfaces before it was checked. When a claim is
  withdrawn, name every surface it reached and correct each; a claim that is
  fixed in the log and left standing on the page has not been withdrawn.

### Merge and conflict discipline
- **CI is the only gate on a merge, and green means merge.** Clay has said so
  three times, most recently on 2026-08-24: "stop asking me for permission to
  merge things just merge them." A Claude-authored PR in this repo needs no
  human word to leave draft or to land — not a request, not a heads-up, not a
  sentence in the reply pausing for one. The discipline below says how to
  merge well; none of it is a reason to wait for him. What goes to him is a
  DECISION, in design/pending-gavels.md, never a merge. If a permission
  prompt reaches him anyway that is the harness's auto-mode classifier rather
  than a question being asked, and the lever is the permission MODE: the
  allow list is not what gates it, because tools already listed there still
  prompt.
- **No open pull request ages past a day.** Clay, 2026-08-26, verbatim: "make
  sure that any of my PRs older than a day get merged promptly." So every
  check-in sweeps ALL open PRs in kanso and kq — mine, another session's, his —
  and drives each to one of three ends: merged on green, superseded, or closed
  with the reason recorded. A PR I cannot push to is superseded from a branch I
  own, never parked. The sweep is the whole list every time, because the one
  that ages is always the one nobody's task list mentions.
- **Never blanket-resolve conflicts** (`checkout --ours`/`--theirs`) on
  runtime.c or any load-bearing file — resolve hunk by hunk.
- **No fire-and-forget merges.** Do not arm auto-merge and move on: wait
  for CI green, merge, and verify the content landed on origin/main —
  `state == MERGED` plus a grep of the changed lines. If CI fails, fix
  and repeat. A PR is not "shipped" until this loop closes; saying
  otherwise is false reporting. (Auto-merge silently failed to fire on
  green PRs more than once, and stale docs sat live for hours.)
- **Reading the cost-goldens job takes two sources, and neither alone is it.**
  Its counter steps are `continue-on-error`, so the per-step
  conclusions the API returns say SUCCESS even when the gate failed — on
  kanso#1262 the API reported `how much work` and `compile instructions` green
  while the job's own vein summary said `work:failure` and `compile
  instructions:failure`, and that summary is the step that fails the job. So
  the summary block (`for vein in "emitted:success" ...`) is the authority for
  every step it lists, AND it omits the trend gate and `page_drift`, whose own
  step conclusions are reliable. This sentence said NINETEEN until
  2026-09-17, when the two codegen rows made it twenty-one; a count in prose
  goes stale the first time anybody adds a row, so read the summary block's
  own list rather than a number written here. Read both. Every other job in the run can be
  read from its steps.
- **AN UNBANKED FLOOR TURNS THREE JOBS RED, AND TWO OF THEM LOOK LIKE AN
  UNRELATED FAULT.** Besides the `cost goldens` job's welfare step, both
  `specs (unit, golden, differential)` and `the other host (macos, arm)` run
  `tests/the_digest_is_priced_on_both_sides.rs`, whose
  `the_undoctored_goldens_hold_the_floor` reads the same sentinel. So a
  re-merge round one is red on the layout gates AND on two jobs whose names say
  nothing about the floor. On 2026-09-18 that cost a detour: the two extra reds
  were read as a second fault and chased before the failing target's name was
  looked at, and the name was the whole answer. Read the failing TARGET, not the
  job title. The fix for all three is the one `--set` that was owed anyway.
- **Opening a PR without arming a wake is how one gets abandoned.** In a
  container nothing runs between turns: a session is woken by a subscription
  or a scheduled check-in and by nothing else. So the moment a PR is opened,
  subscribe to it AND arm a check-in, in the same turn, before doing anything
  else. Two PRs opened on 2026-08-23 skipped this; one went red within four
  minutes and sat there for over an hour, and the session had no way to know.
  Neither the plan nor the intent was missing — only the wake. `list_triggers`
  and the PR's own subscription state are the check: if no wake covers an open
  PR, it is abandoned whatever the task list says.
- `git add -A` sweeps stray working-tree files into commits — scope adds
  to the paths the change owns. (A stray repl experiment once rode into a
  PR and silently broke its CI for a day.)

### The welfare number only goes up

- **TWO WELFARES AND A META cover production and development cost, because the
  per-counter goldens cannot see a trade and one scalar could not hold
  interpreter start-up.** Ruled 2026-09-16, built 2026-09-17.
  `scripts/welfare/welfare.kso` scores a PRODUCTION number over
  `run_instructions`, `run_peak_bytes` (the arena, held and permanent peaks
  summed by `peak_of`) and `codegen_instructions_release`; a DEVELOPMENT number
  over `compile_instructions`, `compile_allocs`, `compile_peak_bytes`,
  `codegen_instructions_dev`, `emit_instructions`, `startup_instructions`,
  `interp_instructions` and `interp_peak_bytes`; and a META over the two, which is the number CI gates
  on and the only one carrying a floor. A weight is a share of ITS OWN side and
  each side sums to one — `balanced?` refuses to score when one does not, which
  is a refusal that exists because carrying the four pre-split weights over
  unrenormalised scored production a fifth low and looked entirely plausible.
  **THE TWO CODEGEN-SIDE ROWS ARE BOTH WEIGHED AND ONE IS EASY TO FORGET.** The
  2026-09-15 exclusion took kanso's own process out of `codegen_instructions_*`,
  and the emitter runs in that process, so `emit_instructions` carries the
  compiler's own half of what a build costs. Weighing the child tree alone left
  kanso#1480's 51,082,187-instruction saving invisible to every term in the
  model, which is how the gap was found.
  `bench/objective_sources.txt` is the list — FOURTEEN `<counter> <gate key>`
  pairs for those eleven — and
  `tests/the_objective_reads_what_the_gate_watches.rs` replays it, so the list
  is checkable rather than remembered. **This sentence has now been wrong FOUR
  times, and the fourth was wrong on the day it was written.** It said THIRTEEN
  pairs when the file held fourteen, in the same paragraph that tells you not
  to trust a count here — and the pull request landing the split said fourteen
  in its own body while this line said thirteen. Read the file:
  `grep -vcE '^\s*(#|$)' bench/objective_sources.txt`. Until 2026-09-06 it said fixpoint rounds, expression visits and
  emitted lines, none of which the objective has weighed since the 2026-09-03
  rebuild, and a session spent a round expecting a 4.5% rise in emitted lines
  to cost welfare when the objective cannot see that vein at all. Until
  2026-09-07 it then said TWENTY-EIGHT, an instruction row per benchmark and
  twelve memory rows — the shape before Clay's 2026-09-06 gavel made the
  runtime side one consolidated program, which turned twenty-five rows into
  five. A session reading that hand-computed a trade over the wrong model, got
  its sign wrong, and only the real `welfare` run caught it. Until 2026-09-17 it
  then said FIVE and SEVEN, which was true for exactly the eleven days between
  the consolidation gavel and the split gavel being built. Run
  `kanso run scripts/welfare -- --counters` for the list, and a bare
  `kanso run scripts/welfare` prints the meta with production and development
  under it.
  **It is an
  index, not a percentage** — the ceiling is a hundred, where every term costs
  nothing, and the origin is arbitrary. Only its direction and the size of its
  moves mean anything. **Do not read the current score from this file.** The
  line here said 66.00 from 2026-09-07 until 2026-09-14, by which date the
  floor stood at 68.73238080266131 — a week of ratcheted gains invisible to
  anyone who trusted the sentence. `bench/welfare_floor.json` carries the
  floor and the `why` of every move, and `kanso run scripts/welfare` reads
  the tree. Every term is deterministic, so the number moves only when
  somebody changes the compiler. CI fails when it drops.
- **The sum is the objective; the terms are diagnostics.** A term getting worse
  is not a problem to defend if the sum went up — that trade is precisely what
  the weights are for, and refusing it would be optimising a part against the
  whole. The per-term breakdown exists to say *where* a move came from, never
  to excuse one.
- **A fall means the change is worse by the project's own stated preferences.**
  There is nothing to argue about the term that paid. Either the change goes,
  or the claim is that the *weights* are wrong — and that is a real argument,
  made about the weights, recorded, and settled before the floor moves. Moving
  the floor to accommodate a change while leaving the weights alone is
  declaring the objective wrong without saying so. **The exception is the
  language itself, and it is ironclad.** Clay, 2026-09-13, verbatim: "you
  don't need to ask my permission to lower the welfare floor if it is in
  service of making the language actually work for the specification. this is
  an ironclad rule." So a change that builds a ruled part of the language
  lowers the floor by exactly what it costs, records why in the history entry,
  and carries on. It does not go to the ledger, it does not wait, and it is
  not a question. What still needs him is a floor drop bought by something
  OTHER than the specification — a performance change that came out behind, a
  convenience, a shape nobody ruled. This rule is why kanso#1372 and kanso#1369
  sat blocked for a day each: both were ruled language features, both were
  built and green but for the floor, and both should have lowered it the hour
  they measured the fall.
- **A rise is held, not banked.** When the number goes up, run `--set` in the
  same PR. A gain nobody ratchets is a gain the next change is free to spend.
- **Raising the floor is never a decision, and never waits on Clay.** The two
  directions have different owners, and a session confused them on 2026-09-12.
  Lowering it to admit a regression is a judgement about what the project will
  pay and it stays his. Raising it to hold a gain is arithmetic: the objective
  went up, and the ratchet is what keeps it up. Clay that day: "raising the
  floor is always a good thing if you can do it and you should never have to
  ask my permission." So run `--set` and carry on. Do not file it as a gavel,
  do not park the pull request behind it, and do not write the rise into a
  body as though saying so were enough. The floor sentinel fails a rise nobody
  banks, so an unratcheted gain is a red pull request rather than a gift to
  the next change. If the tooling refuses the command, that is a permission to
  get granted, said once and briefly; it is not a question about the objective.
- **Bank AFTER the goldens carry CI's rows, never before.** `--set` records
  whatever score the committed goldens produce, so running it while they still
  hold the previous sitting freezes a number this box projected rather than the
  one CI measured. The order is CI's rows, then `--set`, then the page spans.
- **Improvement saturates, at a rate each term chooses.** A term contributes
  `r / (r + satiation)` where `r` is baseline over current, so successive
  doublings pay less and less, and how fast they stop paying is a property of
  the dimension rather than of its importance. Compile cost satiates early
  (0.5): a front end that already finishes in six milliseconds gains almost
  nothing from three, and its doublings are worth 4.3, 2.9, 1.7, 0.9 points.
  Runtime satiates late (2.0): a decoder that gets eight times faster is eight
  times faster, and its doublings are worth 9.0, 9.0, 7.2, 4.8. **Weight says
  how much a dimension matters; satiation says how long it keeps mattering.
  They are different questions and a second of compile time is not a second of
  runtime.**
- **The curve is asymmetric, and most so where satiation is low.** Halving a
  term costs more than doubling it gains. A doubling of compile rounds costs
  5.4 points against a 0.15 weight, where a doubling of decode allocations
  costs 7.2 against 0.25 — per unit of weight the satiated term loses more,
  because a compiler that was imperceptible and is now noticeable has lost
  something real, while a decoder that was already the expensive part has only
  got worse at being expensive.
- **The function is provisional and says so.** Seven deterministic terms are a
  model of what the project wants, not the thing itself; wall time is absent
  because it cannot be made deterministic, and what a model leaves out it
  implicitly weights at zero. Arguing the model is the intended way to change
  it. Every `--set` records why, so the history of the objective is readable
  beside the history of the code.
- **This does not replace the per-counter goldens.** They say which kernel
  moved; welfare says whether the project came out ahead. The first catches a
  deletion, the second catches a trade.

### External state is normalized before it is measured

- **A counter reads the code under test and nothing else.** Every piece of
  state the code did not produce — a cache, a file the loader parses, a
  layout the linker chose — is put into a known state before the measurement
  is taken: cleared, or fixed, so that two runs of the same code read the
  same number and two runs of different code differ by what the code did.
  Clay, 2026-09-15, on learning that `pthread_getattr_np`'s parse of
  `/proc/self/maps` moved the compile row with the binary's layout: "this
  has nothing to do with compiler performance and obviously shouldn't be
  part of what we measure. as I've said to you voluminously in the past you
  want to set up the run so that any external State like this is normalized.
  you clear it out so it's identical every single run or you do something
  that puts it into a persistent known initial state." A term that cannot be
  normalized is excluded and the exclusion is named in the golden's header;
  a term that is counted and explained is the thing this rule forbids. It
  had been argued at kanso#1234 and ruled the other way on 2026-09-03; that
  ruling is superseded on the maps term, and the rule is written here so it
  is not argued a third time.

### Performance goldens are watched, not frozen
- Two veins now: **runtime** (bench/cost_golden*.txt, tests/golden/mem/*.mem)
  and **compilation** (bench/compile_golden.txt). The compile golden counts
  both what the emitter wrote and what deciding it cost — fixpoint rounds and
  expression visits — because the two move independently.
- **The goal is improvement over time, not a frozen line.** A feature may
  cost compile work to buy runtime work, or the reverse, and one metric
  worsening while another improves is a trade to state, not a failure.
- **Movement is fine; silence is not.** Regenerate deliberately, say which
  way it went and why, and record it in the log beside the number. A number
  that changes without a sentence is the thing to catch.

### At release: re-sit the published timings

The decode board and kq's wall-clock rows are a dated sitting on a QUIET box,
moved by hand. Re-sit them at each release, and only then.

They cannot be re-sat on demand. Randomised-layout timing put the spread WITHIN
a single tree at 3.79% and 2.97% — larger than most per-change effects — so a
sitting taken on a machine that has been compiling all day publishes the
linker's luck rather than the compiler's speed. An idle box is a condition, not
an effort, which is why this is a release step and not a task.

kq's rows are in the same position: kq#63 re-stamped the counters and
deliberately left the timings, with the reason in the commit.

### Performance-PR definition of done
1. Benchmarks re-run; **same-sitting interleaved numbers published
   immediately** — dated, conditions named — in the site docs and every
   dependent repo (kq, kanso-json, vse). The table IS the latest sitting;
   idle-machine floors are a footnote refreshed when the box idles.
   **The number-bearing surfaces are a checklist, not a memory** — walk
   ALL of them every time: compiler.html decode board, compiler.html
   lazy scoreboard (§07), compiler.html recipe block (§08),
   compiler.html compile-speed note (§08, "how fast it compiles"),
   index.html landing panel, about.html prose numbers, kq README table,
   kq TRY.md timings, kanso-json README if it grows numbers. Three of
   these sat stale for a day because the sweep ran on recall, and a later
   sweep found five disagreeing figure sets across four pages.

   **A page edit ends with `sh scripts/gates/all_pages.sh`.** Three gates read
   the published pages and they run in three separate CI jobs: `golden_prose`
   (the `data-golden` spans against the goldens they name), `page_drift` (the
   log's budget of unpublished entries) and `prose_check` (the mechanically
   detectable slop families). Only golden_prose can see a span that has drifted
   from its golden. On kanso#1328 a session ran the other two and pushed; that
   cost a round and took welfare with it, because welfare runs golden_prose as
   its last step, so it reported ALREADY RED and could prove nothing about the
   three rows sharing that gate. The sweep costs 31 seconds and reports every
   objection rather than the first.
2. Profile evidence in the PR (which line died, what the floor is now).
3. **Every change carries a perf check**, not just perf PRs: re-run the
   decode floor and the compile timings, and move the published numbers
   when either shifts substantially. Compile speed is a published claim
   now, so a change that slows the front end owes the note an edit.
4. Append-only log entry (design/compiler-log.md): decisions, measurements,
   open threads. Negative results (built-measured-declined) are recorded on
   the compiler page so ideas stay declined.
5. Techniques ledger and mined-queue statuses move in the same PR.

### The log, and what it costs to read

- **design/compiler-log.md holds the last forty entries; the rest is
  design/log/compiler-log-archive.md, unedited.** The log reached 17,935 lines
  by being appended on every change and read only at the tail, which is a cost
  with no return. Append to the live file; when it passes a few thousand lines,
  move the older end to the archive rather than trimming it. Search the archive
  before calling an idea new — that is what it is for.
- **A design note whose work has shipped is deleted, not kept for history.**
  The log carries history. Seven plans described a system that already exists;
  they cost a reader time and told them nothing the code does not.

### Design flow
- Dialog before changes while Clay is designing. A decision that waits on
  him lives in design/pending-gavels.md — the single ledger; STATUS.md
  indexes it, sessions cite entries by heading, and no other file carries
  pending-decision text. The ruling is recorded in the append-only log
  and the entry leaves the ledger in the same commit; implementation
  starts only after the gavel is recorded.
- Docs present the settled design; chronology lives only in the log.
