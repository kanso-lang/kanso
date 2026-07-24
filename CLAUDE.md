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

### Performance-PR definition of done
1. Benchmarks re-run; **same-sitting interleaved numbers published
   immediately** — dated, conditions named — in the site docs and every
   dependent repo (kq, kanso-json, vse). The table IS the latest sitting;
   idle-machine floors are a footnote refreshed when the box idles.
   **The number-bearing surfaces are a checklist, not a memory** — walk
   ALL of them every time: compiler.html decode board, compiler.html
   lazy scoreboard (§07), compiler.html recipe block (§08),
   index.html landing panel, about.html prose numbers, kq README table,
   kq TRY.md timings, kanso-json README if it grows numbers. Three of
   these sat stale for a day because the sweep ran on recall.
2. Profile evidence in the PR (which line died, what the floor is now).
3. Append-only log entry (design/compiler-log.md): decisions, measurements,
   open threads. Negative results (built-measured-declined) are recorded on
   the compiler page so ideas stay declined.
4. Techniques ledger and mined-queue statuses move in the same PR.

### Design flow
- Dialog before changes while Clay is designing; a gavel is recorded in the
  append-only log AND a memory file before implementation starts.
- Docs present the settled design; chronology lives only in the log.
