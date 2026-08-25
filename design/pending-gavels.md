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

Nothing. The section stays so the next entry has somewhere to land.

## Open, not blocking

### Welfare cannot see what the compiler costs to run

Four merges on 2026-08-24 took `compile_allocs` from 148,073 to 87,290 and
the front end's retired instructions down about a quarter, and the welfare
score did not move: 84.87 before, 84.87 after, floor untouched.

That is not a bug in the script. Its model of compile cost is four counters,
and every one of them is flat across those changes:

    compile_speed_counters = ["front_end_rounds" "front_end_visits" "emitted_lines"]
    compile_memory_counters = ["compile_peak_bytes"]

    front_end_rounds        40 -> 40
    front_end_visits    17,786 -> 17,786
    emitted_lines        1,534 -> 1,534
    compile_peak_bytes 871,649 -> 871,649   (the golden welfare reads)

Rounds and visits count the work the compiler decided to do. Emitted lines
count what it wrote. Peak counts what it held. None of them counts what it
does — the allocator traffic, or the instructions retired getting there. What
a model leaves out it weights at zero, and the tree now has two veins,
`bench/compile_allocs_golden.txt` and `bench/compile_instructions_golden.txt`,
watching a dimension the objective scores at nothing.

The question is whether welfare should carry a compile-traffic term, and at
what weight and satiation. The case for: a compiler that retires a quarter
fewer instructions is faster in the way a user feels, and the score is
supposed to say whether the project came out ahead. The case against: the
score already has four compile terms against three runtime dimensions, the
weights were argued once with reasons recorded, and adding a fifth to chase
today's work is how an objective gets fitted to its history.

There is a smaller variant that avoids the weights argument entirely: leave
the terms alone and note in the script's own prose that compile traffic is
deliberately outside the model, so the next person to find a silent 26% knows
it was a choice.

One tangle worth naming, because it shares a root with the compile-memory
entry above: `compile_peak_bytes` DID move, 876,930 to 864,274, an
improvement of 12,656 bytes. Welfare did not see it because welfare reads the
golden rather than the compiler, and the band lets the golden sit unmoved.
Whatever is ruled there decides whether this term reports live movement at
all. Filed 2026-08-24.


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
