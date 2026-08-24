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
