# Pending gavels

Every decision waiting on Clay, in one place. Each entry says what the
question is, where it came from, what the interim state is, and what it
unblocks. Nothing here is urgent in the sense of broken; everything here
is a fork the project has deliberately not taken without a ruling.

## 1. The err rule — GAVELED 2026-08-15: the three-combinator model

The enforcement question dissolved into structure (Clay, 2026-08-15
dialog). Failure handling is three combinators over the effect's two
channels — one callback, one concern each:

- **`bind effect ok`** — maps the success channel; a failure passes
  through untouched, Haskell's `>>=`. Infectiousness is bind with the
  function as its callback. Nothing to check.
- **`annotate effect fail`** (name open) — maps failure to failure: the
  callback reads the reason and returns context; the result re-wraps as
  err with the original as cause (`wrap_err` in flow position,
  `withExceptT` / Rust's `map_err`). Universal — annotating your own
  failure never changes channel. Nothing to check.
- **`rescue effect fail`** — maps failure to success (`catchError`). The
  one licensed door: legal only where the failure is foreign, checked at
  rescue call sites — every reason type the callback's arms name must
  originate in a hako foreign to the caller's; the default arm re-raises
  for free, since an err returned into the success wrap is still an err.
  `src/provenance.rs` already computes the reachable-raiser set; the
  check relocates from every err arm to rescue sites alone.

The July settlements survive unchanged: trapping is naming (pub reason
types are the catchable surface), subtype matching with the
specificity rung, unstoppable = no pub ancestor, `wrap_err` carries the
original, re-export follows the door while trapping follows the leaf,
workspace siblings foreign-with-advisory, vendored code owned. Sharpened
2026-08-15: **the universe boundary is the hako** — local subdirectory
modules are one universe however the author lays them out; splitting
your code buys nothing. What dissolves: the one-bind clause "an err arm
must answer an err" stops being a rule anyone obeys — under
bind/annotate the machinery re-wraps, so it holds by construction.

Precedent (recorded so the novelty stays visible): the trio is the
bifunctor algebra of Either — Haskell `>>=`/`withExceptT`/`catchError`,
Rust `map`/`map_err`/`or_else` with anyhow's `.context` as the annotate
idiom, ZIO `flatMap`/`mapError`/`catchSome`, Wlaschin's railway-oriented
programming. The foreign-only license on rescue has no precedent; it is
kanso's thesis, and everything else is the field's consensus arranged
around it.

### Riders still open under this gavel

- **Spelling**: names and syntax for annotate and rescue — combinator
  call vs marked arm on a chain — and whether the existing chain err-arm
  syntax is annotate's surface (the chain's value arm and err arm are
  bind's and annotate's callbacks already, spelled as dispatch arms).
- **Construction enforcement** (reason building module-private): no
  longer needed for soundness since provenance is computed, still stated
  by the doctrine, still unenforced.
- **The test surface**: a package cannot produce a value about its own
  failure, and an assertion is a value. Either the `*_test.kso`
  file-scope exemption (shipped, crude) or a toolchain assertion surface
  (cleaner, wants shaping — design/testing.md).
- **ch08 pedagogy**: positions.kso dispatches on its own parse_failure —
  unfixable within one program since local modules share a universe. The
  chapter restructures around std/json as the foreign library, or the
  book narrows its failure story to the package boundary.
- **Migration**: the arm-based advisory and the fleet's two violations
  (std/json's failure_position/failure_reason, retired by 1b, plus kq's
  vendored copies) move onto the combinator surface;
  design/err-migration.md updates to this shape.
- **Smaller spellings** carried from the July entry: the dot-prefix
  canon for local imports, the subtype declaration spelling, the
  into-subtype spelling.

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
