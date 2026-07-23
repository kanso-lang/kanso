# Build blocks — the ratified design

Gaveled 2026-07-19 (the sixteen-rulings session). This document is the
durable record of that conversation; the design below is settled, not
proposed. Implementation is next in the ratified build order
(memory frontier → modules → lazy enumerable → **build blocks** → hako).

## The surface

```
graph = build
  a = node "a" []
  b = node "b" [a]
  set a peers [b]        # closes the cycle; a keeps its identity
  [a b]
```

- `build` opens a block expression; its last expression is the result,
  frozen to an ordinary immutable value at the boundary.
- `set target field value` is the single write form. It is an
  identity-preserving field write: `a` stays the same node while its
  field changes, so `b`'s existing reference to `a` sees the update.
  This is what cycle construction actually requires — rebinding cannot
  close a cycle (`a = node_with_peer a b` makes a *new* `a`).
- `set` is grammatically legal only inside `build`. Outside, mutation
  does not parse. The mutable universe is delimited by one greppable
  word: auditing "where can state change?" is `grep build`.
- No new declaration forms. No `var`, no `let mut`, no mutability
  annotation on any name, no rebinding anywhere. Bindings mean the
  same thing in every scope. Mutability is a property of the *place*,
  not the name.

## The legality rule

`set`'s target must be **block-born**: the checker traces the target
to a construction inside the same `build` block. Values passed in
from outside are immutable as always — a function cannot modify a
data structure that was passed to it, only point new structures at
it. This static rule is also exactly the premise the theorem needs.

## The birthday theorem

**Immutable values cannot point at younger values.** A value that
existed before the block ran was already complete; making it point
into the block would be mutation of pre-existing data, which the
block forbids. Every pointer in the heap aims pastward — except
inside a build block, and the block boundary contains the exception.

Therefore **a cycle can only exist among values born in the same
block.** Cycles cannot cross birthdays. The strongly-connected
cluster is always one block's birth cohort.

## Cohort memory

Count the cohort, not the nodes:

- The block allocates into its own arena.
- Freeze returns the cluster with **one** count on the whole cohort.
  Interior pointers — including the cycles — are invisible to
  counting; units cannot be cyclic with each other, by the theorem.
- References from the cohort out to older values are counted once at
  freeze; references into the cohort from later values bump the
  single cohort count.
- Last outside reference drops → the whole arena frees in one shot.
  No cycle collector, no weak annotations, no leaks. Counting stays
  complete; the counted unit is "value or frozen cohort."

Documented tradeoff, accepted with open eyes: keeping one node of a
frozen graph alive keeps the whole cohort alive (the way a Go slice
pins its array). The pure-tree world outside build blocks is
untouched — per-value counting and reuse exactly as before.

## What build blocks are not for

State over time. The executor loop's "variable" is a fold
accumulator — each frame's argument. Build blocks exist for
constructing gnarly values efficiently (including genuinely cyclic
ones); the one true mutable cell stays outside the pure boundary, in
the shell. (The Elm-shaped app answer from the same session.)

## Implementation notes (2026-07-23, pre-build survey)

The shipped memory model has moved since the gavel: the strict tier
is rewinding arenas (not per-value RC), the lazy tier is refcounted
thunks. The design survives the move cleanly — better than cleanly:

- **Native**: a build block is already region-shaped. Allocating the
  cohort contiguously in the current arena and freezing by simply
  ceasing to write is the natural v1; the rewind machinery frees
  regions wholesale and never traverses interiors, so interior cycles
  cost it nothing. The one machine that must learn about cycles is
  the beat carry deep-copy (a naive deep copy of a cyclic cohort
  recurses forever): v1 makes cohort-holding loops beat-ineligible,
  the same conservative posture the byte builder took.
- **Interp**: values are Rc-shared; `set` needs interior mutability
  during construction and a freeze step. A cyclic cohort under plain
  Rc leaks — acceptable for the correctness oracle short-term,
  fixed by giving the cohort one owner cell mirroring the native
  story when it matters.
- **Rendering/equality/encode over cyclic values** need a visited
  set or a depth rule — to be settled at implementation time (the
  book chapter's examples will force the answer).
- The book chapter ships **with** the feature — panels must execute.
