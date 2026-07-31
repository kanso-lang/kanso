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
  files, both cost goldens, the ch10 sample, then book panels.

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

### Merge and conflict discipline
- **Never blanket-resolve conflicts** (`checkout --ours`/`--theirs`) on
  runtime.c or any load-bearing file — resolve hunk by hunk.
- **No fire-and-forget merges.** Do not arm auto-merge and move on: wait
  for CI green, merge, and verify the content landed on origin/main —
  `state == MERGED` plus a grep of the changed lines. If CI fails, fix
  and repeat. A PR is not "shipped" until this loop closes; saying
  otherwise is false reporting. (Auto-merge silently failed to fire on
  green PRs more than once, and stale docs sat live for hours.)
- `git add -A` sweeps stray working-tree files into commits — scope adds
  to the paths the change owns. (A stray repl experiment once rode into a
  PR and silently broke its CI for a day.)

### The welfare number only goes up

- **One scalar covers runtime and compile cost together**, because the
  per-counter goldens cannot see a trade. `scripts/welfare.kso` weighs decode
  allocations and arena blocks, encode allocations and arena blocks, fixpoint
  rounds, expression visits and emitted lines into a single score. **It is an
  index, not a percentage** — the ceiling is a hundred, where every term costs
  nothing, and the origin is arbitrary. Only its direction and the size of its
  moves mean anything. It currently reads about 46. Every
  term is deterministic, so the number moves only when somebody changes the
  compiler. CI fails when it drops.
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
  declaring the objective wrong without saying so.
- **A rise is held, not banked.** When the number goes up, run `--set` in the
  same PR. A gain nobody ratchets is a gain the next change is free to spend.
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
2. Profile evidence in the PR (which line died, what the floor is now).
3. **Every change carries a perf check**, not just perf PRs: re-run the
   decode floor and the compile timings, and move the published numbers
   when either shifts substantially. Compile speed is a published claim
   now, so a change that slows the front end owes the note an edit.
4. Append-only log entry (design/compiler-log.md): decisions, measurements,
   open threads. Negative results (built-measured-declined) are recorded on
   the compiler page so ideas stay declined.
5. Techniques ledger and mined-queue statuses move in the same PR.

### Design flow
- Dialog before changes while Clay is designing; a gavel is recorded in the
  append-only log AND a memory file before implementation starts.
- Docs present the settled design; chronology lives only in the log.
