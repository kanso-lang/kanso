# Pending gavels

Every decision waiting on Clay, in one place. Each entry says what the
question is, where it came from, what the interim state is, and what it
unblocks. Nothing here is urgent in the sense of broken; everything here
is a fork the project has deliberately not taken without a ruling.

## 1. The err-arm rule

May a dispatch arm match on `err`? The three-way split under discussion:
arms may match err freely / arms may never match err (err is only ever
propagated) / err arms allowed only in designated handler positions.
`design/err-migration.md` holds the migration plan for whichever way it
goes. Interim: the current engines propagate err through calls and no
user arm matches it. Unblocks: the failure-model chapter's final form
and several std error-path signatures.

## 2. Read-write map uniqueness

`linear.rs` kills uniqueness on any second mention, so read-then-write
shapes (`put m k (bump m[k])` in every spelling) never select `put_mut`,
and write-only builds never cache the view compaction adopts. Two
designs on the table: read-before-consume tolerance in linear.rs (the
FBIP core; needs a demand interlock so a thunked read forced after the
mutation cannot see the wrong map) or capped view inheritance on plain
`put` (a policy cliff). This single ruling unblocks three threads: the
quadratic-maps fix (the 2.0 GB 10k tally), cohort-license widening to
heap arguments, and the kq import-boundary gap.

## 3. Dependency to_string arms

Should a dependency module's `to_string` arms join the importing
program's render group? Today only the root module's arms merge; a
dep's stay qualified and never join (recorded in
`design/render-plan.md`). A library exporting a money type today cannot
also ship its rendering. Surface question: whether rendering is part of
a module's exportable surface or a root privilege.

## 4. Streaming stdout — GAVELED AND SHIPPED (2026-07-27)

Clay ruled "implement." io/write landed as the eighth description
(kanso#397) with the bind loop's floor guard; kq streams its printer
(kq#36) and now holds less memory than jq on every scoreboard row
(full print 30.0 MB vs 30.8). Kept here for the record; nothing
remains to decide.

## 5. Open dispatch groups

The coherence/orphan rule for user-extensible groups — who may add arms
to a group they did not define, and what keeps two libraries' arms from
colliding. Unlocks user-defined sequences joining std's enumerable
machinery. The annotated-arm split was judged the five-minute half; the
orphan rule is the real question.

## 6. Tail-call promise wording

How the language documents its tail-call guarantee (what is promised
semantics vs what is an optimization), now sharper-edged because TRMC
makes some non-tail shapes loop as well, and the interpreter refuses
unlicensed recursion at 10,000 frames while native's ceiling is the OS
stack. The book currently describes behavior without promising it.

## 7. Tie-rejection ratification

The dispatch rule when two arms tie on specificity (interim since #370:
a bare call that ties head-on between imports is refused, one qualifier
fixes it). Needs ratifying or replacing as the permanent rule.

## 8. Exhaustive ratification

Whether dispatch groups may be declared exhaustive (a call with no
matching arm becomes a compile error instead of a runtime one), and
what the annotation looks like.

## 9. Map-collision and range-as-spread interims

Two standing interim rulings that need a yes or a replacement: map
builders resolve key collisions last-write-wins (the `put` rule,
applied by tally/group_by/index_by/to_h), and range spelled as a spread.

## 10. The ?-suffix spec amendment

Predicate names end in `?` (`all?`, `any?`). The spec's identifier
grammar needs the amendment ratified (where `?` may appear, whether
user identifiers may use it freely).

## 11. put/at bool-key asymmetry

`put` and `at` disagree on boolean keys (one accepts, one misses).
Small, but it is a semantics hole in the map surface and should be one
sentence in the spec once ruled.

## Also open, not blocking any current work

- **TRMC v2**: license operands by inferred set (any provably-int
  expression, not just literals — covers `n * fact (n - 1)`). Needs
  `Inference` threaded out of `check::check` rather than recomputed, or
  the compile-cost golden pays a phantom infer run.
- **`--explain-copies`**: the *where* half of the observability item —
  a diagnostic naming the source site of each evacuation copy. Needs
  span plumbing through the carry machinery; the CLI surface deserves a
  shape ruling before building.
- **The interpreter's 10,000-frame guard**: constant chosen to hold
  under debug builds on the 1 GB thread. If you want it higher, or an
  env override, say the word.
- **Survivor cap 4× block threshold (2 MB)** in the cohort's
  survivor-ratio guard: the multiplier is a judgment call; the principle
  (the dance's transient must stay at threshold scale) is recorded in
  the log.
