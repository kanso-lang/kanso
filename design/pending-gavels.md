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

## 2. RECLASSIFIED 2026-08-03: read-write map uniqueness was never a gavel

Clay, declining it: "This isn't a question for me... it's a deep technical
compiler logistics issue." Correct — whether the linearity fixpoint tolerates
a read before a consume changes nothing a kanso developer observes, and the
standing rule is that compiler internals are settled by measurement.

Measurement then closed it. The recorded consequence — "the quadratic 10k
tally at 2.0 GB" — does not reproduce: a seeded 50-key read-write tally is
dead linear at ~1.8 allocations per iteration with a flat peak (4,653 /
9,203 / 18,303 / 36,503 allocs across 2,500 to 20,000 iterations). The
quadratic died when the read-side compaction landed, and this entry was never
updated. What remains is roughly one avoidable allocation per iteration,
which is an optimization, not a defect, and is recorded in the log.

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

## 12. What `==` answers on — PROPOSED RULING (tasks #54 and #76 together)

Two open tasks are one question, and measuring the current behaviour changed
what the question is. Both engines, 2026-08-01:

    1 < 1.0      false        1 <= 1.0     true
    1.0 < 1      false        1.0 >= 1     true
    1 == 1.0     false        1.5 < 2      true
    w == w       false        w < w        refuses
    point 1 == point 1  true  point 1 < point 2   refuses
    [1] == [1]   true         [1] < [2]    refuses

**#76 is not the coin flip the task recorded.** It offered two settlements —
numbers are one domain (`1 == 1.0` true), or `<` refuses across int and float
— and called them equally consistent. They are not. `1 <= 1.0` answers **true**
and `1.0 >= 1` answers **true**, which is the assertion that the two are equal,
made by two operators that already shipped. `<` and `>` answer false for that
pair for the same reason: neither is strictly less. Three of the four ordering
operators have already ruled that int and float are one numeric domain in
which 1 and 1.0 are the same number. Settlement (b) would mean withdrawing
that and breaking `1.5 < 2`. So the proposal is settlement (a), and it is a
repair rather than a choice: `==` is the one operator dissenting from a rule
the others implement.

**The tempting unification is wrong, and the table above is why.** "Equality
answers exactly where ordering answers" reads well and would settle both tasks
at once, and it would also delete record and list equality: `<` refuses on both
while `==` compares them structurally, which is plainly wanted. Ordering asks
for a total order and answers only where one exists. Equality asks whether two
values are the same and answers wherever there is structure to walk. They are
different questions over different domains, and the disagreement between them
is not itself evidence of a bug.

**#54 stays a real choice, narrowed.** A description and a closure have no
structure to walk, so `==` has nothing to answer with, and `false` for `w == w`
is an invented fact rather than a computed one. Two settlements survive:

- **(a) refuse**, the way `<` already does on the same values. It matches what
  the language does with every other meaningless question — `1 + "a"`, `p < 2`,
  an `if` on a non-bool — and it costs a program nothing that works today,
  since nothing can be relying on a self-comparison answering false.
- **(b) identity**, so `w == w` is true. Useful for "have I already got this
  effect", but it exposes a fact purity should hide: two descriptions built by
  the same expression at two places describe the same effect and would still
  compare false, so the answer tracks allocation rather than meaning.

Recommended: (a). Refusing says what is true — that the question has no
answer — where identity answers a different question than the one asked.

**Sizing, measured rather than guessed.** Both engines hold the defect in one
place. `src/eval.rs:2800` ends `values_equal_seen` with `_ => false`, so an
int-against-float pair and a description-against-description pair fall into
the same catch-all; `src/runtime.c:2659` has `if (a.tag != b.tag) return 0;`
and lets K_DESC and K_CLOSURE reach the default. Settling both means a mixed
numeric arm and an opaque-value refusal in each, plus the wasm path, which
reaches the interpreter's own comparison. `1 == "a"` stays false either way:
different structural types is a well-formed question with a real answer.

## RULED 2026-07-28: accessors are functions

Clay: "so we should make that change." Field access becomes ordinary
function application. `name` is a function, so `list/map people name`
works, and an accessor is a value that can be passed, composed and
mapped like any other.

Two arguments were put and both were wrong, recorded because the
reasoning behind a ruling is worth more than the ruling:

- That `foo.bar` on a typeset "cannot be a static field offset". It can.
  It would be a second form of invocation, resolved by name and
  invisible to normal dispatch — which is what `rt_field_by_name`
  already does. The case against it is that it is a second mechanism,
  not that it is impossible.
- That a reader "cannot tell which mechanism without information that
  is not on the page". This language omits inferred types from source on
  purpose and expects the editor to show them through the language
  server. An argument from what the page alone carries does not apply
  here, and will not apply to the next question either.

What carried it: an accessor that is a function is a value. Field
syntax hands you nothing you can give to anything else.

What Haskell did, checked rather than recalled, because it is the
field's main data point and it argues against the ruling:

- Haskell 98 record fields **are** top-level selector functions, which
  is exactly "accessors are functions". Two records sharing a field
  name in one module was a duplicate definition, which is why Haskell
  code carries `personName`, `dogName` prefixes.
- `DuplicateRecordFields` (GHC 8.0) allowed the duplicates but still
  "does not permit a field and a normal value binding to have the same
  name".
- GHC had type-based disambiguation for ambiguous fields and **removed
  it**: from 9.4.1 selector names must be entirely unambiguous, with
  `-Wambiguous-fields` warning on code that relied on the old rules.
- `NoFieldSelectors` exists to stop generating the selectors at all,
  and `OverloadedRecordDot` brings back `person.name` through the
  `HasField` class.

So Haskell started where this gavel points and spent twenty-five years
walking back to dot notation with the selectors switched off.

Where they ended up, and where they are going, because the destination
matters more than the retreat:

- The surface is `person.name`, desugared to `getField` of a `HasField`
  class; record update desugars to `setField`. The mechanism is a
  typeclass, not a name.
- `setField` **has not shipped in a released compiler**. Overloaded
  record update still needs `RebindableSyntax` and hand-written
  `getField`/`setField` in scope. Four years after the dot syntax,
  update is still provisional — a caution about how long the tail on
  this kind of change is.
- Proposal 583 splits `HasField` into independent `GetField` and
  `SetField` with no superclass relationship, `Field` as a constraint
  synonym, laws, unlifted types. The stated motivation is **read-only
  virtual fields** — a field that is computed rather than stored — and
  they avoid making `GetField` a superclass of `SetField` precisely so
  write-only fields stay expressible.

Two things follow that bear on the ruling.

**Their mistake was the namespace, not the functions.** A Haskell
selector is a monomorphic name — `name :: Person -> String` — so a
second `name` is a duplicate definition. Every escape they built was
about getting field names out of the value namespace, and `HasField` is
them bolting on dispatch-by-argument-type because the language lacked
it. kanso has that natively: `length` already carries arms for lists,
strings and maps, so a record arm is the ordinary case. The namespace
question is only frightening if one pictures a flat monomorphic
namespace rather than a dispatch group.

**Virtual fields come free here.** The thing proposal 583 needs a class
redesign to reach — a field that is computed rather than stored — is
just another arm once accessors are functions. `fn area (rect w h)` is
indistinguishable from a stored field at the call site. That is an
argument for the ruling, and it is where their own vision points.

**The risk, stated precisely.** GHC removed type-directed
disambiguation because inference could not always know the argument
type at the use site. kanso should answer the same question: what
happens where the argument's type is not statically known? Every
function here already faces it, so it is presumably answered — but
fields would make it far more common, and that is the one place their
failure mode would reappear.

The distinction that may or may not rescue it: Haskell's selectors are
monomorphic functions and disambiguation was a bolt-on to inference,
which could not always know the argument type at the use site. kanso
dispatches on argument type as its universal mechanism — `length`
already has arms for lists, strings and maps — so a record arm is the
ordinary case rather than an extension. That is a real difference and
it is also exactly the kind of reasoning that talks somebody past a
warning, so it is recorded as an argument rather than a conclusion.

Both follow-on questions are now ruled.

**Spelling: `x.name` reads, `_.name` is the accessor.** The dot stays
tight, as it was, and the section supplies the thing field syntax could
not: a value you can hand to something else.

    clay.name                      the read
    list/map people _.name         the accessor, dispatching on each element

The pipe spelling `foo . bar . baz` was built first, on the reasoning
that the pipe and the dispatch already in the language reach overloaded
field access with no new mechanism. It works, and it costs more than it
saves. Making the accessor reachable by name puts every field name in
the value namespace, and three things follow: a field may no longer
share a name with a type, so `type post` with a `state` field and a
`state` typeset stops compiling; destructuring shadows the accessor, so
ch03's own example cannot show `track artist minutes title = song` and
`song.title` in one scope; and the pipe binds looser than arithmetic and
application, so `point (p.x + 1) p.y` becomes
`point ((p . x) + 1) (p . y)`.

Haskell reached the same place from the other side and its answer is the
one taken here. `OverloadedRecordDot` classifies the dot through the
whitespace-sensitive operator mechanism — tight is field selection,
loose stays composition — and the users' guide gives the section
directly: "You may also write `(.b)` to mean a function that 'projects
the `b` field from its argument'". `NoFieldSelectors` then stops
generating the top-level selectors, so field names leave the value
namespace entirely while construction, update and pattern matching keep
working. Clay: One Right Way — and `_` is already the pipe-position
hole, so the section is spelled with the hole this language has rather
than the parenthesis convention that one does.

**Namespace: getters are arms under a name no program can spell.** A
getter is a real dispatch arm, so every type declaring `name` joins one
group and `_.name` is polymorphic across all of them — the one-up over
Haskell survives, since `HasField` exists only because their selectors
are monomorphic. What does not survive is the getter occupying a
writable name. It is declared as `Get_name`, which the lexer cannot
produce, and it is never module-qualified: reading a field is
structural, so it needs nothing brought into scope and can collide with
nothing.

**And the Haskell risk does not transfer, measured rather than
argued.** GHC removed type-directed disambiguation because inference
could not always pin the argument type at the use site, and their
resolution is a compile-time class lookup with no runtime fallback:
unpinned means ambiguity error. kanso dispatches at runtime through
`k_check_rec` and specialises only when the inferred set is narrow
enough to allow it. A bare unannotated parameter handed to a two-arm
group resolves both ways correctly:

    fn describe v
      speak v

    describe (dog "rex")   -> woof from rex
    describe (cat "tom")   -> meow from tom

Not statically known is the ordinary case here, not a failure.

What remains is implementation, and one small thing worth noticing on
the way: `a.peers = [b]` inside a build block is a *write*, a statement
form, and this ruling is about reads. The two look alike and are not
the same construct.

## DECLINED 2026-07-29: named arguments and the label set

Explored in a day's dialog and declined by the author who proposed it.
Recorded because the reasoning is worth more than the outcome, and
because it will occur to somebody again.

**What was on the table.** One colon in four places — `fn speak
animal:(dog n)`, `speak animal:pet`, `user age:47 name:"clay"`, `{ age:a
name:n } = clay` — with the call's label set acting as a dispatch
selector, so a shorter set is a less specific call and the piped value
fills the slot marked `_`. Order would then carry nothing, which is what
made alphabetizing fields and parameters look mandatory rather than
merely tidy.

**What killed it, in the author's words: minimalism.** "Order is as
concise as it gets. names are actually more verbose. and you'd have to
sometimes re-map them with a colon." Two arguments behind that, both
his.

The first retires an objection rather than making one. Complecting is
symmetric here: matching arguments to parameters needs *some*
correspondence, position is one and name is another, and neither is
free. "You've got to complect something to match the args, and name
isn't inherently better than position." The Hickey framing recorded
during the dialog leaned on an asymmetry that is not there.

The second is decisive and comes from a ruling this project already
made. Named arguments are a readability feature, and kanso has already
chosen to buy readability from the editor rather than the page — which
is exactly why inferred types are omitted from source and the language
server shows them. Spelling labels into the text while omitting types is
inconsistent. The consistent position keeps position.

A third argument, which surfaced from the author's remark about
re-mapping with a colon: naming gives a call site two forms, bare when
the local's name happens to match and `x:item` when it does not.
Position has one spelling, always. By the one-way-to-do-it measure
naming is the more complected option, not the less.

**What was genuinely on the other side**, so a future revisit argues
against the real thing. The asymmetry that survives is blast radius
rather than complecting: position derives correspondence from a global
property of the parameter list, so adding, removing or reordering
changes every call site at once, where a rename breaks only the calls
that mention it and breaks them loudly. And the silent-reorder hazard is
narrower than it first appeared — type dispatch already catches a swap
of differently-typed arguments, so only adjacent same-typed parameters
are exposed. That window did not pay for a permanent verbosity tax on
every call in a language whose name means plain.

**What survives the reversal, unchanged:**

- Accessors are functions, ruled above. Reads are application; that has
  nothing to do with labels.
- Keyed destructuring stays exactly as it is: `{ author:writer title } =
  hello`. The author's reason is that it solves a different problem —
  taking one field out of a record without walking past the others with
  `_` placeholders — and it never depended on labelled calls.
- `_` in pipe position, which multi-argument pipes need under position
  just as much.
- Ordering rules stay where they are: typeset members and keyed reads
  alphabetical, record fields and function groups the author's. The
  instruction to alphabetize everything was conditional on order losing
  its meaning, and order keeps it.

## 13. What an imported record prints as

The same program prints one thing run directly and another through an
import:

    run directly:      record point 3 4, absent <none>
    through an import: record sample/point 3 4, absent <none>

The defect is not which is right — it is that both happen, so moving a
type declaration into a library changes what a program prints. Three
micro samples show it: `render_record_none`, `subtype_chain` (`animal`
against `sample/animal`) and `err_trap_named` (`slow_lane` against
`sample/slow_lane`).

RECOMMENDATION: the bare form. Printing is for a human reading output
and a module path is a fact about source organisation, not about the
value; `<none>`, `true` and `1.5` carry no provenance either. Two
modules owning a `point` and needing to be told apart is what a
`to_string` arm is for, and that already works.

ONE ASYMMETRY, and it is the part wanting a view rather than a rubber
stamp: an err's REASON is not an ordinary value. For a crash, which
package raised it is worth more than reading cleanly —
`sample/quota_torn` says where to look and `quota_torn` does not. So a
defensible answer is that values render bare and err reasons render
qualified. That is two rules where one would do, which is normally the
argument against, except that failures and values are already different
things here.

UNBLOCKS: the harness rework (running the micro corpus through a
generated entry, which is step one of migrating `play` out of the
compiler) and the three samples above. Everything else in the
import-path bug family is closed.

## 14. Are kanso's bytes a type or a convention

The interpreter has no distinct bytes value — bytes ARE a list of ints
there — and native has a real `K_BYTES` tag that refuses anything else.
So `[97]` is bytes to one engine and a type error to the other, and
`text/find2 [97] 0 97 98` answers `1` under the interpreter while
native refuses to run it. Four builtins diverge; genuine bytes agree on
all four, which is what makes it narrow.

Two ways, and they are not equivalent: native widens to accept a list
of ints wherever it accepts bytes, which says a list and bytes are
interchangeable — the thing native's representation exists to deny; or
the interpreter gains a distinct bytes value so it can refuse a list the
way native does, which removes the ambiguity at the root and touches
every place the interpreter builds or reads bytes.

INTERIM: `tests/a_bare_list_is_or_is_not_bytes.rs` pins the half that
already agrees and carries the divergent half `#[ignore]`d — not
because the work is unfinished but because the assertion cannot be
written down until somebody rules.

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
