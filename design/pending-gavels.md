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

- STATUS.md may index this file. It does not carry decision text.
- Sessions cite entries by the headings below, never by a session task
  id — a task list is private to its session and its numbers resolve
  nowhere else.
- Edits to this file ride small, promptly-merged PRs, never a feature
  branch, so the ledger cannot fork.

## Blocking — a fixture, gate, or merge is waiting

### Does a DEMANDED knot count as a thunk allocation?

The undemanded half is ruled and built; this is what survived it. `x = [x]`
that something reads answers `1` on both engines, and the counters do not
agree about what building it cost: native reports `thunk_allocs=1` and the
oracle `0`, because the oracle's `knotted` makes its cell without touching
the counter. It predates the deferral and is untouched by it. The question
is what `thunk_allocs` counts — every cell, or only the ones a program's own
laziness made — and the answer decides which engine moves. No fixture can pin
a demanded knot's allocation shape until it is settled; the value and the
forces already agree, so nothing else waits. Filed 2026-08-24.

### The compile-memory band has been hiding main's own drift

`bench/compile_memory_golden.txt` holds peak bytes at 871,649 and CI
asserts only that reality is within two per cent — 17,432 bytes of slack
to absorb a documented host divergence of 56. Main measures 872,025 on
a container one glibc revision off the runner, three runs identical, and
the runner itself says 872,061 twice; 376 of those bytes are main's own,
accrued green. It matters because welfare reads that row as the current
value of the compile-memory term rather than measuring it, so the floor
was ratcheted against a
figure the compiler left behind. Put the measured number in the file and
`scripts/welfare` exits 1. Three options priced in the log for
2026-08-23: `--set` the floor on a fall, which `--set` has never been
used for; pay the bytes back out of the front end; or tighten the
band to something near the divergence it documents. Filed 2026-08-23.

Updated 2026-08-24, because the gap has grown and the entry should be
decided against what is true now. Main measures 876,930 here (three runs
identical) and the runner said 876,956 on the same tree, so the drift is
5,307 bytes against 17,432 of slack — up from 376 when this was filed.
#993 is 4,895 of that: the reader index the inference fixpoint now holds,
which bought 5,438 expression visits and is a trade welfare took happily.
None of it is visible to welfare, which reads the golden rather than the
compiler, and that is the whole entry.

Updated again 2026-08-24 with what the 26 bytes are: the checkout path.
Peak reads 876,898 plus twice the length of the directory the compiler
ran in, at every length measured — 16, 17, 21, 29 and 49 characters, one
tree, one binary. `/home/runner/work/kanso/kanso` is 29 characters, so
the formula gives 876,956, which is what the runner said. The container
and the runner agree on the compiler's own number to the byte.

That is the premise the band was widened against, and it does not hold.
The header of `bench/compile_memory_golden.txt` cites a linux/macos
divergence of 56 bytes; the gate runs in the cost-goldens job, which is
ubuntu-only, so that divergence has never gated anything. What 17,432
bytes of slack absorb on the machines CI actually uses is two bytes per
character of checkout path, and 5,307 bytes of main's own drift.

So there is a fourth option, and it is the one that fits the rule
against bands. Subtract the path term in the gate and pin peak exactly.
The coefficient need not be hardcoded: `kanso check lib/json` takes well
under a second, so the gate can run it from two directories of known
differing length and derive the term, which keeps working if the front
end ever stops holding the path twice. Peak becomes a number rather than
a band, drift shows the day it happens, and the three options already
priced become a question about 5,307 bytes rather than about whether
anyone can see them.

The same measurement retired a row from `bench/compile_allocs_golden.txt`
in #998: `compile_alloc_bytes` is 7,942,033 plus the identical term, so
it was pinning the clone. `compile_allocs` is flat at 148,073 across all
five lengths and across rustc 1.94.1 and 1.98.0, and it is what that
golden holds.

### Does `>>` accumulate run-time effect failures, or stop at the first?

Both channels are otherwise settled: the value channel answers one
value, and failures merge associatively so the feared fractal never gets
built — the applicative/monad split, with `.` as the monad. The open
fork is measured: a *build* failure on either side merges, but a
run-time *effect* failure on the left stops the right dead — one
operator behaving two ways depending on when the failure lands. Say the
wall orders effects and unordered failures cannot preempt, or run the
right anyway and merge. It collides with the ruling that `>>` defers its
right side (2026-08-15) wanting the right lazy. Filed 2026-08-22.

## Open, not blocking

### Riders under the err gavel (the three-combinator model, 2026-08-15)

- **Spelling**: names and syntax for annotate and rescue — combinator
  call vs marked arm on a chain — and whether the existing chain
  err-arm syntax is annotate's surface (the chain's value arm and err
  arm are bind's and annotate's callbacks already, spelled as dispatch
  arms).
- **Construction enforcement**: reason building module-private is
  stated by the doctrine, unnecessary for soundness now that provenance
  is computed, and unenforced. Enforce it or strike it.

### `--explain-copies`

The *where* half of the observability item — a diagnostic naming the
source site of each evacuation copy. Needs span plumbing through the
carry machinery; the CLI surface deserves a shape ruling before
building.

### An assert hako

A real assertion library in the rspec direction Clay sketched —
`(expect 1) . to (equal x)` — as its own small surface design, never
improvised inside a test fix. Its arms are foreign to every tested hako,
so the err license needs nothing special. Queued 2026-08-17.

### Dot chains route around accessor privacy (Demeter)

A chain can reach a field the owning module would not expose directly —
the unbuilt half of per-field `pub`, and a real hole in the privacy
story. Probing it on 2026-08-23 found what the build needs first: the
checker has no record type at a field read (`pub x` inside a type is a
syntax error today; `has no field` is raised in eval.rs at run time), so
the fence needs record-type inference the value sets do not do. A
run-time refusal was considered and declined: this language refuses
before anything runs. Low priority.

### The interpreter's 10,000-frame guard

Constant chosen to hold under debug builds on the 1 GB thread. Standing
offer: say the word for a higher constant or an env override.

## Stale — the July campaign's unclosed letters (GAVELS.md, retired here)

The July design doc ruled its A1–X and BB (those rulings are in the log
and archive; the doc's full text is in git history). Letters that never
closed, each needing revalidation against the post-boundary-language
world of 2026-08-17 before it is worth Clay's time:

- **C — pure/yield**: does the fold-yield idiom need a named primitive
  (`out >> yield store`) or does a plain value on `>>`'s right
  auto-lift? Asked before `>>` deferred its right side; the question's
  shape may not have survived.
- **D — what a succeeded effect yields**: `none` today, whose silent
  railway-skip is a footgun; a `done` marker was the alternative. Same
  caveat as C.
- **G — eta-reduction as canon**: ban the forwarding lambda
  (`map (c -> fetch c)` → `map fetch`) plus the composition rules a
  dispatch group held as a value still owes
  (design/function-values.md).
- **Z — errors without exceptions**: presumed declined — the 2026-08-15
  err gavel kept err with the foreign-only rescue license, which is the
  world Z1 abolished. One word confirms and this line moves to the log.
- **AA — newtype dispatch acceptance**: ancestor-walking was rejected
  by every prior finding; the live half is typeset acceptance as the
  idiom vs explicit cast only. The 2026-08-19 ruling covered the
  declaration and ctor form, not acceptance.

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
  campaign — three investigations already terminate there.
- Survivor cap 4× block threshold: the multiplier is a judgment call;
  the principle (the dance's transient stays at threshold scale) is in
  the log.
