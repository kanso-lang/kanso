# Pending gavels

Every decision waiting on Clay, in one place. Each entry says what the
question is, where it came from, what the interim state is, and what it
unblocks. Nothing here is urgent in the sense of broken; everything here
is a fork the project has deliberately not taken without a ruling.

## 1. The err-arm rule — PROPOSED RULING (dialog converged 2026-07-27; awaiting Clay's final read)

The two-universes rule, mechanized without provenance tracking:

- **The rule, in one line (Clay's phrasing, 2026-07-28): a function
  that receives an err raised in its own package must return an err.**
  Everything else follows. Inspection inside that function is
  unrestricted — read the reason's fields, compute with them, build
  what you like — because the constraint is on what comes back out,
  not on what may be looked at. The transitive case falls out too:
  hand your own err to a helper that returns an int and the helper is
  the violation, since it is the one that received it. What
  *converting* costs is therefore a foreign reason: an arm may turn
  an err into a value only by naming a reason type owned by a
  different published package.
- **Trapping is naming.** Since patterns can only name pub types, a
  package's catchable surface is exactly its pub reason types —
  catchability is pub-ness.
- **Unstoppable = private.** An err whose reason type is not exported
  cannot be named downstream, so it bubbles with no way to stop it.
  Both original motives (no err control flow within a party; forcible
  bubbling) come from type visibility alone.
- **Bare `err reason` arms may only return err-or-subtype** —
  annotation and wrapping on the way through, never absorption.
- **Local subdirectory modules are one universe** (no license).
  Workspace siblings — separately published packages in one repo — are
  licensed (treat them as potentially two teams) with a `kanso check`
  advisory naming the smell. Vendored code is owned code: its errs
  stop being trappable.
- **Construction stays module-private**, which also prevents forging a
  foreign failure to launder control flow.
- The cheat (splitting your own code to trap your own errs) is
  self-defeating: making a failure trappable requires publishing the
  reason type, which makes it catchable by every client forever —
  there is no "handleable by me only" state, which is exactly the
  state the rule exists to ban.

SETTLED (Clay, 2026-07-27): (a) std is foreign to user code — std
exceptions are absolutely rescuable. And **subtype matching** (Clay's
ruling, the Ruby rescue model made order-independent): an arm naming a
reason type catches every descendant, and the dispatch ladder gains
one rung — a subtype ascription is more specific than its ancestor's.
A value's ancestor chain is a line, so matching ascriptions are
totally ordered and subtype matching can never tie (shrinks the
tie-rejection gavel's surface). Consequence, accepted into the
proposal: **unstoppable refines to "no pub ancestor"** — a private
leaf under a pub root is trappable through the root, so a package can
publish one coarse root ("everything of mine you may handle") while
keeping leaves private and refinable; a truly unstoppable failure
gets a reason chain that is private top to bottom. This structurally
reproduces Ruby's StandardError/Exception split — handleable things
under a published root, defects rooted outside it — with pub doing
the work Ruby's inheritance convention does.

SETTLED (Clay, 2026-07-27, second round): (b) wrapping carries the
original — mechanized as the only two legal spellings. err stays
opaque magic (reason + origin + hop trace, runtime-maintained, never
record fields); infectiousness makes `err some_err` inert, so the
annotate-nothing case is the identity re-raise (`fn handle e:err`,
return `e` — trace intact; canon prefers this over reconstruction,
which would mint a new birth site), and wrapping is the builtin
factory `wrap_err new_reason original` — the one deliberate hole in
infectiousness, attaching the original as cause in the magic layer,
rendered nested at the endpoint. Discard-while-wrapping has no
spelling. (c) re-export follows the door, trapping follows the leaf:
a re-exported type is the re-exporter's as a *name* (the ultimate
caller neither knows nor cares), but the conversion license compares
the arm's package against the type's *origin* package — where the err
stack leafs out. Everyone but the origin may trap it, including the
re-exporter.

**CORRECTED (Clay, 2026-07-28).** An earlier entry here said a package
cannot inspect its own errs. It can, all day. The rule constrains the
*return*, not the look: an arm may match its own err, read every field
of the reason, and build whatever it likes from them — it must hand
back an err. "Any function it passes its own err to must also return
err." The advisory built for this already draws exactly that line
(an arm reading both fields and re-raising is silent; only conversion
to a value is flagged), so the code was right and the description was
not.

What survives the correction is narrower and still real: **a package
cannot get a non-err value out of its own err**, and an assertion
needs a value. Measured — a test constant that touches an err
propagates it, and the harness reports `FAILED (returned err …)`, so
equality, interpolation and every other route are closed alike. That
is why the test-file exemption below is still wanted: not because
inspection is blocked, but because an assertion is a value and a
package may not produce one about its own failure.

Consequences measured against the current tree:

- std/json's `failure_position` and `failure_reason` become illegal
  (std trapping std's own `parse_failure`). They are also unnecessary
  under the structure-access amendment — foreign clients trap and read
  fields directly — so they migrate away, taking
  examples/json_failure_door.kso and the ch08 suite with them.
- std's other own-err arms are all legal: `number_ok`, `string_ok`,
  and `must` match err and *return* err. The discipline was already
  being followed where it matters.
- **Now measured, not predicted (2026-07-28).** The rule is built as
  `advisory[license]`, sound-by-under-approximation: it flags only an
  arm whose reason type carries the arm's own qualifier, where same
  module means same universe with no plumbing needed. The fleet's
  entire violation set is two functions — std/json's
  `failure_position` and `failure_reason`, plus kq's vendored copies
  of them. Everything else is silent, including kq's
  `render_result (err reason)`, which re-raises. `*_test.kso` files
  are exempt per the recommendation below.
- **A package can no longer assert about its own failure paths.**
  json_test's position assertions and its `defect?` predicate both
  convert an err to a value, which is the one thing the rule forbids.
  RECOMMENDED (interim, being built behind): a **file-scope exemption
  for `*_test.kso`** — a test is the author inspecting internals, not
  shipped behavior, and the alternative is that no package can ever
  test the errs it raises. Crisp, file-level, trivially checkable.
- **The pedagogy consequence needs Clay's eye.** ch08's teaching
  program (`positions.kso`) has a decoder and a `show` arm that
  dispatches on its own `parse_failure` — legal today, illegal under
  the rule, and *unfixable within one program*, since local modules
  share a universe. The chapter teaches "an err a caller might
  reasonably handle is a value to dispatch on" (ch04's line), which
  the rule narrows to "across a package boundary." Either the chapter
  restructures around std/json as the foreign library, or the book
  teaches that within one program handleable outcomes are `none` and
  values, and err-dispatch belongs at library boundaries. This is a
  real narrowing of the failure story the book currently tells.

Still open, smaller: the dot-prefix canon for local imports (nested
local paths spelled `./a/b`, bare multi-segment = hako name — makes
every import's universe readable in its spelling); the subtype
declaration spelling (`type foo string` vs `type post_body:string`);
the into-subtype spelling (ctor-form `foo ""`, previously ruled, vs
the sketch's postfix `"":foo`); positional destructuring of foreign
types (recommended: named access only crosses).
`design/err-migration.md` holds the migration plan.

**PREREQUISITE, found by building (2026-07-28): the license cannot
land until 1b does.** Measured against the current implementation: a
foreign client may trap by membership — `(err e:json/parse_failure)`
compiles and dispatches, since a foreign type NAME crosses an import
— but may not read the failure's fields, because the opacity rule
still bans foreign destructuring. So the only way to extract a
position or a reason is the owner's projection function, which is
precisely what the license forbids the owner to write. Together the
two rules make failure data unreachable. 1b is not a nicety attached
to this ruling; it is the thing that makes the ruling shippable.

## 1b. Foreign structure access — PROPOSED AMENDMENT to modules-plan
(Clay, 2026-07-27 dialog)

Named structure reads cross packages: dot access (`e.position`) and
keyed patterns (`(parse_failure position: p)`) on pub types, one level,
the Demeter-legal read of a value in hand. Positional destructuring
stays module-local (it couples to full field count and order — the
author's layout freedom). Construction stays factory-only. This
supersedes "types are opaque outside their module, always" and retires
projection boilerplate like std/json's failure_position. Open detail:
per-field pub, or all fields readable with the pub type — argued on
the merits, not demand. For per-field: the doctrine's own sentence is
"pub is name-level surface," a field is a name, so field-level pub is
the same rule applied uniformly, not a new feature; and with
construction factory-only, authors will have computed internal fields
(caches, normalized forms) that all-with-type forces them to either
expose or split into a pub shell around a private core — ceremony the
per-field spelling deletes. For all-with-type: records are data, a
hidden field is a sign the value is carrying non-data, and one fewer
visibility site keeps patterns and dot-reads uniform. Recommendation:
per-field pub, because it is the existing rule made uniform rather
than an addition.

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
