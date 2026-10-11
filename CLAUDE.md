# CLAUDE.md — kanso

Guidance for this repo. There is no global CLAUDE.md behind it (see "This file is the whole of it" below). The first thing it holds is the writing voice, because the website and the book are the product, and they have to read like a person wrote them.

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

A session on the web clones this repo and nothing else, and this file is its
complete instruction set. There is no global memory and no `kanso-lang/memory`
repo; do not attach or search for one, and do not report its absence as a gap.
The history behind these rules is in the log and in this file's own git
history; the file states the rules.

## Two sessions, two lanes

Clay runs a compiler worker ("kanso cloud") and a design chat. A session knows
which it is before it edits anything.

- **Cloud owns the compiler**: `src/`, `lib/`, the goldens, `bench/`, `tests/`,
  and `scripts/`. It measures, builds, opens pull requests and merges
  them.
- **The chat owns the interaction machinery**: design/compiler-log.md, the
  design notes, this file and the prose of docs/compiler.html. It files,
  labels and closes issues. It does not write code; a change to the compiler,
  its goldens or its benchmarks is cloud's however small it looks. Clay: "your
  PRs should only write to gavels and such, and cloud should do the coding
  work."
- **The page is the chat's for prose, never for numbers.** A `data-golden` span
  quotes a golden, `golden_prose` checks the quote, and moving one means the
  golden moved, which is cloud's. The chat writes the page's sections because
  `page_drift` fails a tree more than three log entries ahead of the page, and
  the chat writes most of the log.
- **Each session merges its own pull requests on green.** The sweep under
  "Merge and conflict discipline" still covers every open pull request, so one
  that ages is landed by whichever session reaches it first.

### The tracker

**GitHub issues are the tracker.** Each open item is an issue in
kanso-lang/kanso, labelled by kind (`needs-clay`, `ruled`, `cleanup`, `perf`),
by lane (`lane:chat`, `lane:cloud`) and by priority (`p1`, `p2`, `p3`). A pull
request that finishes one says `Fixes #N`. An issue says what is open; the log
says what happened. Read the queue with
`gh api 'repos/kanso-lang/kanso/issues?state=open&labels=<label>'`.

- **A decision for Clay is a `needs-clay` issue, and nowhere else.** It carries
  the search behind it (the log, the archive, the docs, the tests), the options
  and a recommendation, so a sitting can be a yes or a no. A question left in a
  pull request body or a log paragraph is one nobody sees. A performance
  question with no surface area is the implementer's and never goes to Clay. A
  question already ruled is never re-asked: cite the ruling and close the issue.
- **When Clay rules, the chat records it**: a `— gavel:` entry in the log, and
  in the same change the issue is closed (nothing to build) or relabelled
  `ruled` and `lane:cloud` with what is owed as a checklist. Cite a ruling by
  its log heading or issue number, never by a session task id.
- **Cloud chooses the build order, from the whole list.** It reads every open
  `ruled` issue at each check-in and before opening a pull request, and a pull
  request on a self-generated lead says which rulings it weighed and why the
  lead came first. Clay: "it needs to have discretion over the order. but it
  just needs to be able to see and consider the entire list to do that, not
  just take the next thing."
- **Counting rulings takes two greps.** Rulings since mid-August are headed
  `— gavel:`; older ones use `GAVEL:`, `GAVELED:`, `GAVEL (…):` or
  `GAVEL, IMPLEMENTED:`. No heading matches both, so count each and add:

  ```
  grep -hE "^## " design/compiler-log.md design/log/compiler-log-archive.md | grep -cE "— gavel[:,]"
  grep -hE "^## " design/compiler-log.md design/log/compiler-log-archive.md | grep -cE "GAVEL(ED)?[:,(]"
  ```

  Never write a ruling count in prose, and a sweep that calls a period complete
  names the pattern it matched on. `page_drift`'s `ruling?` reads only
  `— gavel:` and `— directive:`, so an old-spelling ruling counts against the
  page's budget.

### Access and permissions

- **GitHub API access depends on how a session was started.** Sessions started
  from claude.ai/code or the desktop app carry the GitHub App credential; one
  started with `claude` in a terminal does not, and a session it spawns
  inherits the refusal. Git push keeps working either way, because the proxy
  injects push credentials separately. The refusal text ("GitHub access is not
  enabled for this session…") is the same whatever the cause, so read `origin`
  from `get_session` before believing it.
- **A blocked action goes back to Clay, never sideways to another session.**
  When the permission classifier refuses something (a push to main, deleting a
  remote ref), say what was refused and let him choose. He does not want work
  driven through a second session on his behalf.
- **Never offer a direct push to main.** It skips CI, the only gate this
  project has. Clay: "you always make a PR and then merge it."
- **Branches are free.** The instruction to develop on a named branch is about
  not pushing to `main` or to someone else's branch. Cutting a new branch from
  main for your own work needs nobody's permission; never spend a turn, an issue
  or a notification asking. What needs Clay is a design decision, a push to
  main, or a permission the harness refuses.

## Ironclad engineering rules (do not relax)

### Goldens for everything

- **Every behavior ships with a golden.** A feature without a golden pinning
  its observable behavior does not merge. A diagnostic change regenerates its
  error-corpus goldens in the same pull request.
- **Every performance kernel ships with a presence counter** pinned in a
  CI-diffed cost golden. Counters are platform-invariant: they count
  algorithm-level events, never vector widths or platform paths. A merge that
  silently drops a kernel must turn CI red; one once deleted Eisel-Lemire from
  main and nothing noticed.
- **Counters changed means regenerate every vein in the same PR**: the .mem
  files, all TWELVE cost goldens, `bench/emitted_golden.txt`, the ch10 sample,
  the book panels, and the siblings' own veins. Do not count the goldens from
  memory; `sh scripts/gates/all_counters.sh` reads every one and names each that
  moved, and `--write` regenerates them with headers intact. (The count word in
  this bullet is pinned by `tests/every_counter_gate_is_in_the_sweep.rs`, which
  is why it is the one count left in this file; #1846.)
- **The runtime sweep does not read the compile veins, and a library change
  moves them**, because `lib/*.kso` is `include_str!`'d into the compiler.
  `machine_code`, `emitted_code`, `compile_memory`, `compile_allocs`,
  `compile_instructions`, `entry_instructions`, `library_instructions`,
  `compile_libraries`, `codegen_instructions` and `emit_instructions`
  are separate gates, and `sh scripts/gates/all_compile.sh` runs them all. Facts a
  reader needs about them:
  - `kanso check` routes by content: a directory is a module, a file with bare
    statements is an entry, a file of definitions alone is a library. The
    three routes are three compiles, with a row each.
  - `codegen_instructions` runs `kanso build`, so it is the only row that
    reaches codegen. It counts the child tree (clang, `clang -cc1`, ld) and
    excludes kanso's own process. Without an argument it runs both tiers,
    `-O0` and `-O3 -flto`. `emit_instructions` counts `codegen::emit_ir`, the
    compiler's own emitting.
  - `compile_libraries` lists the shared objects the compiler links;
    `library_instructions` counts instructions. The names are one letter apart
    and unrelated.
  - Which gates refuse to compare depends on the host, because each checks its
    golden's measured-on line. A refusal exits non-zero like a regression, so
    read the sweep's `not compared here` line rather than counting failures.
  - `tests/the_compile_sweep_names_every_compile_gate.rs` fails if a gate on
    disk is missing from this bullet, so the list above can be trusted.
- **Two compile veins are read by a cargo test rather than a gate**:
  `bench/compile_golden.txt` and `bench/compile_golden_modules.txt`, read by
  `tests/compile_cost.rs`. `all_compile.sh` runs that test by name. Regenerate
  with `KANSO_REGEN_COMPILE_GOLDEN=1 cargo test --test compile_cost`.
- **The compile sweep builds its artifacts first** (`build_benchmarks.sh` is
  its first line, pinned by a spec), because its gates read `*.ll` files and
  binaries they do not produce, and stale artifacts read as movement or hide it.
- **`compile_instructions` usually moves on an edit to the compiler's Rust**,
  even one that cannot change a decision it counts, because the row moves with
  the binary's layout. A small enough edit leaves it alone. Take the number
  from CI: never write down that it cannot move, or that it did before CI says
  so.
- **A library edit needs `cargo build` before it takes effect.** A bare
  `kanso build` runs the old library; both sweeps build first.
- **kq keeps five veins** and the instructions vein is the one most changes
  reach: allocation counters in `bench/cost_golden.txt`,
  `bench/cost_golden_decode.txt` and `bench/cost_golden_escapes.txt`, retired
  instructions in `bench/instructions_golden.txt`, and
  `bench/numbers_stamp.txt` keyed to the first. A change that moves no
  allocation counter still moves instructions. A new counter, even a purely
  additive one, moves the siblings' files too.
- **Read kq's kanso pin from kq, never from prose.** `add_repo` with owner
  `kanso-lang` and repo `kq` widens a session's scope; then
  `/home/user/kq/.kanso-version` holds the pin and
  `git rev-list --count <pin>..origin/main` says how far behind it is.
- **`bench/emitted_golden.txt` counts what the compiler wrote for the
  decoder**, where the cost goldens count what it allocates. The two move
  independently; watch both and assume neither causes the other.

### Do not stop

A turn ends when I write prose without calling a tool, so every stop is a
choice to report instead of continuing.

- **A merged pull request is one item, not a stopping point.** If anything is
  in progress, the work is not done.
- **Answer questions inline and keep working.** Prose can be emitted mid-turn.
- **When the next step is large and uncertain, that is the step.** Take its
  first concrete action. Clay: "just build and measure." A measurement that
  kills an idea is a result; a note describing it is not.
- **A correction is an interrupt.** Fix it, say what was wrong in a sentence,
  carry on.
- **A decision that is Clay's goes to him the moment it is found**: a
  `needs-clay` issue, a `PushNotification`, and the first line of the reply.
  Then work something else while he reads.
- **The naming test.** Before ending a turn, try to write "next I will X". If
  the sentence forms, X can start now, so delete the sentence and do X. The
  turn ends only when no next action can be named: every thread is merged,
  waiting on a decision already sent to Clay, or waiting on a measurement with
  a monitor on it, and nothing else on the list can start.
- **`ScheduleWakeup` is a stop.** Its `reason` may describe only cadence and
  what is watched, never a next action, and the reply may not contain one
  either ("next tick", "when this lands I'll").
- **These are not reasons to stop**: waiting on CI, everything being merged, the
  next piece being large, context running low (do the smallest useful piece),
  a decision needing Clay (send it and work on), and having no branch (cut one).
- **Every stop is announced with a `PushNotification`** saying that work has
  stopped, why, and what the next wake is. Clay: "if you stop working you need
  to urgently notify me." He is not reading the reply when it lands.
- **The `/loop` is a last resort** that restarts work after a stop, and only
  Clay arms, disarms or retimes it.

### Every fix answers a failing spec

- **A fix ships with the smallest program that fails without it**, a fixture
  the size of a postcard. If the bug cannot be reduced to one, the diagnosis or
  the reproduction is not understood yet.
- **Watch it fail, for the right reason, before it passes.** Break the fix,
  run the spec, read the message, restore. A spec never seen red is a guess.
- **Assert what a program does**: output bytes, a diagnostic a user sees, a
  counter a golden pins, peak memory that does or does not grow. A spec on an
  internal verdict pins the current decomposition and goes green when it moves.
- **Enter where a user enters.** Run the program and read the output;
  hand-built intermediate state asserts a fiction.
- **Run a guard bare.** Never pipe a check whose exit status is the answer
  (`| tail` takes `tail`'s status). Put a read of a produced file in the same
  `&&` chain that produced it, and delete the file first so a stale one cannot
  answer. Before trusting a new check, break what it watches and watch it go
  red; this applies to the plumbing as well as the assertion.
- **The reduced fixture belongs in the corpus**: error goldens for
  diagnostics, micro goldens for one construct, the mem vein for allocation
  shape. A bug with no home there is a gap, and adding the home is part of the
  fix.
- **Pin the number, never a band.** `== 10`, not `< 1024`. A number that looks
  noisy is measured several times, and either it is stable and pinned or the
  variance is the finding. The compile-memory gate's 2% is a measured
  host-divergence allowance, not a precedent.

### The differential law

- The interpreter is the oracle. Every engine that speaks a feature is
  byte-identical on it, pinned by differential goldens. A feature may land on
  fewer engines only if the others reject it with a clear diagnostic, never by
  silently diverging.
- Divergence-prone surfaces (float formatting, utf-8 strictness, rendering) get
  adversarial goldens that probe the edges.

### Verification ethos

- **Harness before core.** For a precision kernel (float parse and render,
  utf-8, dispatch), build the differential fuzzer first, against an
  independently written reference, and iterate to fuzzer silence. Record the
  case count in the pull request ("50M doubles, 0 failures"). The harness
  extracts the real function text from the source, never a copy.

### A measurement bounds what it measured

A number applies to the context that produced it and to nothing that
resembles it. Before an argument rests on one:

- **Read what the number describes first.** A rule's section states its
  exceptions, and a golden's header states what its reading may be compared
  with, including whether a reading from this container may stand beside
  one CI took.
- **A report that something is absent is worth what the search for it was
  worth.** A fixture shows what happened, and a grep for a phrasing finds that
  phrasing.
- **An attribution is not a mechanism.** That a delta arrived with a change is
  a difference-in-differences; it becomes a cause when something isolates the
  change and agrees.
- **A withdrawn claim is corrected everywhere it travelled**: the log, the
  page, the issue, the pull request body.

### Merge and conflict discipline

- **CI is the only gate, and green means merge.** A Claude-authored pull
  request needs no human word to leave draft or to land. Clay: "stop asking me
  for permission to merge things just merge them." What goes to him is a
  decision, as a `needs-clay` issue. A permission prompt that reaches him is
  the harness's classifier; the lever is the permission mode, not the allow
  list.
- **No open pull request ages past a day.** Clay: "make sure that any of my PRs
  older than a day get merged promptly." Every check-in sweeps all open pull
  requests in kanso and kq, whoever opened them, and drives each to merged,
  superseded, or closed with the reason recorded. One I cannot push to is
  superseded from a branch I own.
- **Opening a pull request arms its wake in the same turn**: subscribe to it
  and schedule a check-in. Nothing runs in a container between turns, so a pull
  request with no wake is abandoned.
- **No fire-and-forget merges.** Wait for green, merge, and verify the content
  on origin/main (`state == MERGED` and a grep of the changed lines).
  Auto-merge has silently failed to fire before.
- **A finished coordinated sibling branch answers for every later branch with
  its name.** `.github/clone-sibling.sh` prefers a kq, vse or kanso-json branch
  named after the branch under test, and names get reused. Read the job's
  `kq: <branch>` line before its diagnostics, and retire a coordinated branch
  when its counterpart merges by merging main's tree onto it.
- **Never blanket-resolve conflicts** (`checkout --ours`/`--theirs`) on
  runtime.c or any load-bearing file. Resolve hunk by hunk.
- **A resolution that keeps both sides can leave a paragraph twice**, and every
  structural check passes over it. After resolving docs/compiler.html, look for
  a long line that appears more than once; `golden_prose` reporting one drift
  more times than expected is the duplicate showing.
- **The cost-goldens job takes two sources to read.** Its counter steps are
  `continue-on-error`, so their API conclusions say success even when a gate
  failed; the job's own vein summary block is the authority for every step it
  lists. The trend gate and `page_drift` are outside that block and their own
  conclusions are reliable.
- **An unbanked floor turns three jobs red**: the welfare step, and
  `specs` and `the other host` through
  `tests/the_digest_is_priced_on_both_sides.rs`. Read the failing target, not
  the job title. The fix is the `--set` that was owed.
- **`git add -A` sweeps in stray files.** Add the paths the change owns.

### The welfare number only goes up

- **Three sides and a meta.** `scripts/welfare/welfare.kso` scores a
  production side (the native run and release-tier codegen), a development
  side (the front end, dev-tier codegen, the emitter, start-up, and the
  interpreter's speed and memory) and a browser side (compiling and running in
  the tab), and a meta over them. The meta is what CI gates on and the only
  number with a floor. A weight is a share of its own side, each side sums to
  one, and `balanced?` refuses to score when one does not.
- **Read the terms and the score from the tree.** `kanso run scripts/welfare`
  prints the meta and its sides, `-- --counters` prints the terms,
  `bench/objective_sources.txt` pairs each term with its gate key, and
  `tests/the_objective_reads_what_the_gate_watches.rs` replays that file.
  `bench/welfare_floor.json` holds the floor and the reason for every move.
  `emit_instructions` is the term most easily forgotten: it carries the
  compiler's own half of a build, which `codegen_instructions_*` excludes.
- **It is an index.** The ceiling is a hundred, where every term costs
  nothing; the origin is arbitrary. Only direction and the size of moves mean
  anything. Every term is deterministic, so it moves only when the compiler
  changes, and CI fails when it drops.
- **The sum is the objective; the terms are diagnostics.** A term getting worse
  is not a problem if the sum rose, which is the trade the weights exist for.
- **A fall means the change is worse by the project's own preferences.** Either
  the change goes, or the claim is that the weights are wrong, argued and
  settled as a weight change before the floor moves.
- **The exception is the language itself.** Clay: "you don't need to ask my
  permission to lower the welfare floor if it is in service of making the
  language actually work for the specification. this is an ironclad rule." A
  change that builds a ruled part of the language lowers the floor by exactly
  what it costs, records why in the floor's history, and carries on. A floor
  drop bought by anything else needs Clay.
- **Under the 2026-09-05 corpus-first gavel, a blind corpus is repaired
  first**: add the benchmark, baseline it before the fix, and the fix scores as
  a rise.
- **A rise is banked in the same pull request with `--set`**, and raising the
  floor never waits on Clay. Clay: "raising the floor is always a good thing if
  you can do it and you should never have to ask my permission." The sentinel
  fails a rise nobody banks.
- **Bank after the goldens carry CI's rows.** `--set` records whatever the
  committed goldens score, so the order is CI's rows, then `--set`, then the
  page spans.
- **Improvement saturates.** A term contributes `r / (r + satiation)`, where
  `r` is baseline over current, so each doubling pays less than the one before.
  Satiation is set per term in welfare.kso: low where a dimension stops
  mattering early (compile cost), high where it keeps mattering (runtime).
  Weight says how much a dimension matters; satiation says for how long. The
  curve is asymmetric, so halving a term costs more than doubling it gains,
  most of all where satiation is low.
- **Every environment kanso runs in has terms, and none sits at zero.** Each
  gets a CPU term and a memory term at a weight chosen for how much it matters
  next to production. Clay: "you can choose a pragmatic weight to apply to that
  versus actual production run time and stuff but it doesn't have zero
  weight." An engine or tier joins the objective in the pull request that adds
  it, with a golden, a gate and a re-banked floor. The browser side counts
  `bench/interp_corpus` compiled and run in the tab under wasmi's fuel meter
  (`tests/browser_cost.rs`, `scripts/gates/browser_cost.sh`); its rows belong to
  the rustc that built `docs/kanso.wasm`, so take them from CI.
- **The function is provisional.** Wall time is absent because it cannot be
  made deterministic, and anything the model leaves out it weights at zero.
  Arguing the model is how it changes, and every `--set` records why.
- **It does not replace the per-counter goldens.** They catch a deletion;
  welfare catches a trade.

### External state is normalized before it is measured

A counter reads the code under test and nothing else. Every piece of state the
code did not produce (a cache, a file the loader parses, a layout the linker
chose) is cleared or fixed before measuring, so two runs of the same code read
the same number. Clay: "you want to set up the run so that any external State
like this is normalized." A term that cannot be normalized is excluded and the
exclusion is named in the golden's header. Counting it and explaining it is
what this rule forbids.

### Performance goldens are watched, not frozen

- Three families: **runtime** (`bench/cost_golden*.txt`,
  `tests/golden/mem/*.mem`), **compilation** (the compile gates above) and
  **browser** (`bench/browser_golden.txt`, `scripts/gates/browser_cost.sh`).
- **The goal is improvement over time.** A feature may cost compile work to buy
  runtime work, or the reverse; one metric worsening while another improves is
  a trade to state.
- **Movement is fine; silence is not.** Regenerate deliberately and say in the
  log which way each number went and why.

### At release: re-sit the published timings

The decode board and kq's wall-clock rows are a dated sitting on a quiet box,
re-sat at each release and only then. Spread within one tree under randomised
layout is 3 to 4%, larger than most per-change effects, so a sitting on a busy
machine publishes the linker's luck.

### Performance-PR definition of done

1. **Benchmarks re-run, and same-sitting interleaved numbers published**, dated
   with conditions named, in the site docs and every dependent repo (kq,
   kanso-json, vse). Walk every number-bearing surface from the list, not from
   memory: compiler.html's decode board, lazy scoreboard (§07), recipe block
   (§08) and compile-speed note (§08); index.html's landing panel; about.html's
   prose numbers; kq's README table and TRY.md timings; kanso-json's README if
   it grows numbers.
2. **A page edit ends with `sh scripts/gates/all_pages.sh`**, which runs
   `golden_prose`, `page_drift` and `prose_check` together. Only `golden_prose`
   sees a drifted span, and welfare runs it last, so skipping it costs welfare
   too.
3. Profile evidence in the pull request: which line died, where the floor is
   now.
4. **Every change carries a perf check**: re-run the decode floor and the
   compile timings, and move the published numbers when either shifts
   substantially.
5. A log entry with the decisions, measurements and open threads. Negative
   results go on the compiler page so ideas stay declined.
6. Technique-ledger and mined-queue statuses move in the same pull request.

### The log

- **design/compiler-log.md holds the last forty entries**; older ones move,
  unedited, to design/log/compiler-log-archive.md. Append to the live file and
  move the oldest entry out when it passes forty. Search the archive before
  calling an idea new.
- **A design note whose work has shipped is deleted.** The log carries the
  history.

### Hygiene is continuous

Clay: "the instructions should explicitly maintain ongoing refactoring so that
things stay hygienic over time and I don't have to explicitly remind you."
Cruft here has one recurring shape: a fact written in two places and corrected
in one.

- **Leave each file you touch cleaner than you found it.** A small cleanup of
  code or prose the pull request already changes goes in its own commit.
  Anything larger becomes a `cleanup` issue.
- **One fact, one home.** A number, list or rule lives in one place and
  everything else points at it. A second copy is deleted or made a pointer in
  the same pull request. A test whose only job is keeping two copies in step
  means one copy should not exist.
- **The cleanup queue is worked as it is filled.** Cloud lands at least one
  `cleanup` pull request for every five other pull requests it merges, from the
  open `cleanup` issues in its lane by priority. The chat works its
  `lane:chat` cleanup issues whenever it has a pull request open anyway.
- **Cloud sweeps the repository once a week** and files a `cleanup` issue in
  the right lane for each finding, with the evidence and what done looks like:
  a rule or fact stated twice; a count in prose a script could print; a design
  note whose work shipped; a source file past a few thousand lines with seams
  in it; tracked files nothing reads; a test that only keeps two copies in
  step; docs describing something that no longer exists. A finding in this
  file, the log or the docs is `lane:chat`.
- **A refactor is a pure move.** No golden changes. A compile or machine-code
  row that moves by layout is named as layout in the log, and welfare moves are
  reported and banked like any other.
- **Instructions are rules, not stories.** This file states each rule once,
  with its reason in a sentence or two. How a rule was learned belongs in the
  log.

### Design flow

- Dialog before changes while Clay is designing. A decision that waits on him
  is an open `needs-clay` issue and appears nowhere else, a pull request body
  included. The ruling goes in the log and the issue is relabelled or closed in
  the same change; implementation starts after the gavel is recorded.
- Docs present the settled design; chronology lives only in the log.
