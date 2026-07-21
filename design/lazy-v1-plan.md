# lazy v1: the fragment, both engines, one PR

The staged ruling (compiler-log 2026-07-21) ships the experimentally
verified fragment first. This plan pins the v1 decisions so the build is
mechanical.

## Surface: what gets a thunk

A binding is **conditionally demanded** when its value flows into a
dispatch argument position where at least one arm of the callee group
discards that parameter (`_`). Kanso has no `if`; dispatch is the only
branch, so "some arm ignores it" is exactly "demand depends on which arm
wins," which is unknowable before the scrutinized arguments are seen.
Those bindings compile to thunks. Every other binding compiles strict,
exactly as today — provably-demanded work gains nothing from a cell.

Streams/generators and the work-pool scheduler are later slices; v1 is
conditional-demand bindings only.

## Semantics

- A thunk forces at **value scrutiny**: dispatch selection on the value,
  arithmetic, comparison, interpolation/render, destructure, effect
  execution. Binding and argument-passing do NOT force — deferral across
  the call boundary is the whole point.
- Force-once: the cell overwrites itself with the result and drops its
  captures (the retention bound).
- An `err` produced during a force propagates from the force point. A
  thunk never forced never errs — this IS the ratified lazy semantics,
  and it is v1's one observable change: a skipped binding whose
  computation would have erred no longer errs. A golden program pins
  this exact divergence-from-strict explicitly.
- Both engines land in the same PR. A lazy interpreter against a strict
  native would disagree on skipped-err programs; the lattice forbids it.

## Representation

- **Interp** (design-first engine): `Value::Thunk(Rc<RefCell<ThunkState>>)`,
  `Pending { expr, env } | Forced(Value)`; re-entrant force is a defect
  (blackhole, matches `<<loop>>`).
- **Native**: defunctionalized cells — no closures. Each thunked bind
  site gets a site id; the cell is `{ rc, site_id, args[] }`, and force
  is one switch over site ids calling the site's generated evaluator.
  Cells are RC'd and recycled through a free list, allocated OUTSIDE the
  beat arenas (a pending thunk must not pin a rewindable region — the
  jhc/Compact Regions lesson). v1 copies captured values into the cell
  at creation; cheap (captures are locals) and it severs every
  arena-lifetime entanglement. Revisit only with evidence.

## Counters and goldens

Semantic counters — `thunk_allocs`, `thunk_forces`, `thunk_evals`,
`thunk_live_exit` — are printed by BOTH engines under mem-stats and must
match byte-for-byte: evaluation counts are semantics, not implementation.
Allocator counters (`allocs`, `arena_blocks`, ...) stay native-only.
The `.mem` goldens grow the semantic lines; the oracle asserts the
interp's semantic lines against the same golden. `skip_unused.mem` must
show `thunk_evals=0` for the skipped site; `shared_twice.mem` shows
`thunk_evals=1` under two reads; a new `skipped_err` program pins that
the strict-era error no longer occurs.

## Order of work

1. Demand pass (`src/demand.rs`, sibling of `escape.rs`): mark
   conditionally-demanded bind sites; consumed by both engines.
2. Interp thunks + force points + semantic counters; oracle assertions.
3. Native cells (`runtime.c`: RC tier + free list), codegen (site-id
   evaluators, thunk creation at marked binds, force at scrutiny).
4. Goldens regenerated as an explicit, reviewed diff; `skipped_err`
   added to the differential corpus.
