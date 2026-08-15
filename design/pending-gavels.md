# Pending gavels

Every decision waiting on Clay, in one place. Each entry says what the
question is, where it came from, what the interim state is, and what it
unblocks. Nothing here is urgent in the sense of broken; everything here
is a fork the project has deliberately not taken without a ruling.

## 1. The err-arm rule — PROPOSED RULING (dialog converged 2026-07-27; awaiting Clay's final read)

The two-universes rule, mechanized without provenance tracking:

**PROVENANCE, and why the shipped proxy is not it (Clay, 2026-07-28).**
Clay: the criterion is where the err *stack originated*, not which
package the receiving function sits in — "package A can't rescue an
err from package B by passing it into a function defined in B", and
he adds that he cannot see the simplest way to hand that to a
compiler. He is right, and the advisory built here uses a proxy for
it: the reason type's qualifier. Two measurements, both against the
current tree:

1. **The proxy has a reachable hole.** A package can raise an err
   whose reason is a foreign type and then rescue it, because the
   qualifiers differ and the advisory sees a foreign reason:
   `err (json/parse_failure 1 "mine")` raised in user code, then
   matched by `(err _:json/parse_failure)` and turned into a string.
   Runs today, advisory silent. Provenance says illegal; the proxy
   says fine.
2. **Foreign construction is not enforced at all.** The doctrine says
   "construction is module-private; importers build through pub
   factories" (modules-plan). It is not: user code builds
   `json/parse_failure 99 "I made this"` and prints it. That is an
   independent gap, and it is also what makes (1) reachable.

**BUILT (2026-07-28, Clay: "build it correctly"). Provenance is
computed, not proxied.** `src/provenance.rs` gives each dispatch group
the set of packages whose errs it may hand back and takes a fixpoint
over the call graph, the way inference already does for value sets —
Clay's observation that one hop suffices is what makes it cheap: an
err can only reach a function through a pattern that matches err, so
every step of a failure's travel is a call whose callee names it. The
rule reads off it directly: a group that may receive an err raised in
its own package must return an err. A pub group is seeded with its
own package, because its callers are not all in view and anyone may
hand a package its own failure back. Cost, measured A/B on one
binary: 0.6 ms on `kanso check` (KANSO_NO_PROV switches it off, as
KANSO_NO_FUSE does for fusion).

The laundering case is caught: raise `err (json/parse_failure …)` in
your own code, rescue it with a foreign-reason arm, and provenance
names the raiser — the borrowed name buys nothing. Rescuing a
genuinely foreign err stays silent. Both pinned.

Construction enforcement is therefore no longer needed for soundness,
though the doctrine still says it and it is still unenforced — a
separate gap, recorded above.

If construction were enforced, the proxy would have been exact rather than
approximate — only json can build a json reason, so a json-reasoned
err can only have been raised by json. That is the cheapest route to
a compiler-checkable provenance: **enforce the construction rule the
doctrine already states, and the reason type becomes a sound witness
for the raiser.** The alternatives are a whole-program dataflow that
tracks which functions could have raised the err reaching a site, or
a runtime check against the origin the err already carries — exact,
but a runtime failure mode for a rule the language wants static.

Recommendation: enforce construction first, then the proxy needs no
argument. Clay's call, since it is a language rule with its own
migration (every `json/parse_failure` built outside json becomes a
call to a json factory).

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
  Two ways out (Clay, 2026-07-28, on why tests need this at all — an
  assertion is a value, and a package may not produce one from its own
  err): a **file-scope exemption for `*_test.kso`**, which is one line
  and crude, a hole in a language rule at file granularity; or
  **assertions get a toolchain surface** — the harness is not the
  package, so a builtin that reads a failure is a foreign party
  rescuing, legal under the rule as written, needing no exemption and
  not leaking into shipped code. The exemption is what ships behind
  the advisory today; the surface is the cleaner design and wants a
  small amount of shaping. See design/testing.md.
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

**PREREQUISITE SATISFIED: 1b is GAVELED (Clay, 2026-08-14).** Named
structure reads cross packages — dot access and keyed patterns on pub
types, one level — with pub granted per field; positional
destructuring stays module-local; construction stays factory-only.
This supersedes "types are opaque outside their module, always" and
retires projection boilerplate like std/json's failure_position. The
measured trap-but-cannot-read gap (a foreign type NAME crosses an
import, so `(err e:json/parse_failure)` dispatches, while the opacity
rule banned reading its fields) is thereby closed, and the license
below is shippable.

## 3. Dependency to_string arms

Should a dependency module's `to_string` arms join the importing
program's render group? Today only the root module's arms merge; a
dep's stay qualified and never join (recorded in
`design/render-plan.md`). A library exporting a money type today cannot
also ship its rendering. Surface question: whether rendering is part of
a module's exportable surface or a root privilege.

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

## 8. Exhaustive ratification

Whether dispatch groups may be declared exhaustive (a call with no
matching arm becomes a compile error instead of a runtime one), and
what the annotation looks like.

## 15. Should `>>` defer its right side

`a . f` hands the continuation over as a closure, so nothing past the
current link exists until the link runs. `a >> b` takes `b` as an
already evaluated description, so building the first link requires
evaluating the second, which requires the third: the whole chain is
constructed before any of it runs, and the construction is what
exhausts the stack. A loop written with `>>` dies where the same loop
written with `.` does not.

INTERIM SHIPPED: all three engines now name the operator in the
diagnostic rather than blaming the recursion, since a function calling
itself in the right operand of `>>` is visible in the source and the
runtime cannot work the cause out at the moment it reports. That makes
the failure legible; it does not make the loop run.

## 16. Should block-born widen to a dataflow property

Today `block-born` is a syntactic property, which is why in-place graph
algorithms inside build blocks stay out of reach (ledger 4.4). Widening
it to a dataflow property is the unblock. It is a language-surface
question rather than an allocator one, which is why it sits here rather
than being settled by measurement.

## 17. Does printing a lazy sequence force it

`print "{list/map [1 2 3] (x -> x * 2)}"` shows the adapter chain rather
than the elements. Rendering the elements means forcing, and `cycled`
is infinite — so the question is whether print forces, forces a bounded
prefix, or keeps showing the chain and makes the user ask for
`to_list`. Route A (an arm in `lib/render`) was unblocked by #723, so
the mechanism exists whichever way this goes; what is missing is the
semantics.

## 18. `pure` as a record type (Clay's proposal)

Clay proposed `type answer / value` plus one executor arm per engine.
Recorded when it was raised and never taken further. It touches how a
pure value and a description relate, which is the same seam as 19
below, so the two probably want deciding together.

## 19. Should io infect — auto-lifting operators over descriptions

Whether an operator applied to a description should lift automatically,
so a description behaves like the value it will produce. The pull is
ergonomic; the cost is that the type of an expression stops being
readable from the expression. Raised, not argued.

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
- **An `os` package** (Clay): what moves out of `io` — `MkdirAll` was
  the example. A surface question, cheap once the shape is agreed.
- **Sequencing more than two binds prettily** (Clay's
  `multiplyTwoRandoms`): today the third bind nests. Wants a form, not
  a mechanism.
- **Dot chains route around accessor privacy** (Demeter): a chain can
  reach a field the owning module would not expose directly. Low
  priority, and a real hole in the privacy story.

## 20b. What a pending cell shows when it reaches output

Same surface as 20, found 2026-08-14 while closing task #210. Independent
enough to be ruled separately, close enough that ruling 20 first may decide it.

    fn noted acc _ true
      acc

    fn noted acc why false
      text/concat acc [why]

    fn gathered acc at
      one = wants[at]!
      why = "did not paint {one.why}"
      noted acc why false

    pub first = gathered [] 1

    print "{first}"

All three engines answer `[<thunk>]`. The differential law is satisfied. The
author computed a string and the program shows them a word about the
implementation instead.

The `_` arm is what defers: a parameter crosses already-evaluated only when
every arm demands it. Every demanding site that has learned to force closed one
form of this — the field read (#889), `text/join` (#890), the strict index
(#892) — and rendering a list is the one left.

**Why rendering has not learned.** Forcing at the top of `k_render` was built on
2026-08-13 and recurses forever on `ring = [ring]`: forcing the cell answers a
list holding the same cell. `<thunk>` is what terminates it today.

**Why that is fixable.** Rendering already has a cycle guard. It prints
`<cycle>` and it is keyed on records — `eval.rs:3491`, `runtime.c:3185`. The
comment at `runtime.c:3363` says every cycle passes through a record, and a
self-naming constant is the counterexample: its cycle passes through a thunk
cell. Extending the guard's path to cells terminates by the same argument the
record path already relies on.

**The cost of doing it.** `a_constant_that_holds_itself` currently records
`[<thunk>]` and would record `[<cycle>]`. That is the whole user-visible
consequence, and it is a question about what a self-naming constant displays —
which is why it belongs beside 20 rather than in a runtime PR.

**One obstacle if it is ruled yes.** `render_seen` in `eval.rs` is a free
function with no interpreter handle, so the oracle cannot force from inside it
without restructuring. Native and the browser can. Whoever builds it starts
there.

**The third answer.** Neither engine renders, and the demanding sites keep
being taught one at a time as programs find them. That is where the last three
fixes landed and it has no end condition — nothing says which site is next
except a user hitting it.
